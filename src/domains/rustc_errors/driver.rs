//! The `--fix-compile` oracle driver (Plan 019 Phase 1, T1.2/T1.4) — the
//! bounded repair loop:
//!
//! ```text
//! check → parse+dedupe → propose per error (playbooks) → order
//!       → apply best → recheck → keep | revert → repeat ≤ max_iters
//! ```
//!
//! ## Keep/revert semantics (T1.4)
//!
//! A candidate is **kept** iff the package's coded-error count strictly
//! DECREASED after applying it. Everything else — no change, more errors,
//! the error moved elsewhere — reverts the exact original bytes at the
//! span and marks the candidate tried. The next iteration re-proposes
//! against the fresh diagnostics + fresh source, so byte spans never go
//! stale and the next-best untried candidate is picked. This is
//! deliberately stricter than "compiles": a fix that trades one error for
//! another (the `Box::leak` shape — compiles, wrong) must not survive
//! arbitration.
//!
//! ## Loop bounds
//!
//! `max_iters` (default 5) bounds the loop. The `tried` set bounds it
//! further: when every candidate for the current error set has been tried
//! and reverted, the loop stops (`NoMoreCandidates`) — the misses are the
//! authoring queue for the next playbook templates (Phase 4 T4.4).
//!
//! ## Write semantics
//!
//! Review mode (no `--write`) NEVER writes: one check, one proposal pass,
//! the plan printed from [`FixCompileReport::planned`]. Write mode runs
//! the loop and leaves on disk exactly the kept fixes — a reverted file
//! is byte-identical to its pre-run content (the `--fix --verify`
//! unconditional-write trap does not carry over; Plan 019's founding
//! requirement).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::compile_oracle::{
    CheckRunner, CheckVerdict, check_verdict, owning_package_root, resolve_reported_path,
    workspace_root,
};
use crate::span_embed::{SPAN_EMBED_DIMS, span_embed as embed_shape};

use super::diag::{RustcErrorDiag, dedupe_by_site, parse_error_diags};
use super::playbooks::{FixCandidate, Playbooks, SpanEdit};

/// One attempt in the fix-compile loop's ORDERED sequence (Issue 91 T1.5 —
/// the per-attempt ring). The report's `applied`/`reverted` Vecs lose the
/// interleaved order, and the trajectory store holds per-FAMILY counters
/// (the T1 mining verdict) — T2's oscillation detector (error A → fix →
/// error B → fix → error A) and Issue 093's per-run strategy posterior both
/// consume THIS sequence: one instrumentation, two kernel consumers
/// (katgpt-rs 593 + 592).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// 0-based position in the run's attempt order.
    pub seq: usize,
    /// The rustc error code the candidate answered (e.g. `E0507`).
    pub code: String,
    /// Stable site identity: [`span_hash`] of (file, line) — 16 hex. The
    /// oscillation key is `(code, span_hash)`: the same error RETURNING to
    /// the same site after an intervening fix is the saddle-circling
    /// signature.
    pub span_hash: String,
    /// The playbook strategy that was tried.
    pub strategy_id: String,
    /// The loop outcome at this attempt.
    pub outcome: AttemptOutcome,
    /// Coded-error count before → after the attempt (equal on every revert
    /// path except a masking-increasing one; strictly decreasing on `Kept`
    /// unless masking decreased — see `masking`).
    pub errors: (usize, usize),
    /// Masking-stage count before → after (Issue 100): diagnostics whose
    /// code is in [`super::diag::MASKING_STAGE_CODES`]. The keep rule is the
    /// LEXICOGRAPHIC decrease of `(masking, count)` — a keep at equal count
    /// is legal exactly when masking dropped (the unmasking trade: a typeck
    /// error resolved, revealing the borrowck errors it was blocking).
    /// `#[serde(default)]` so pre-Issue-100 rings deserialize as (0, 0) —
    /// consistent, because under the old rule every `Kept` had a strict
    /// count decrease at masking 0.
    #[serde(default)]
    pub masking: (usize, usize),
    /// Issue 105 (run-level, stamped at dump time — `0` on in-memory
    /// records and pre-105 rings): Σ over ALL attempts of
    /// `masking_after + errors_after` — the integral of the journey's
    /// residual states, PC-ALM's `λ_T` raw material (α applied by the
    /// consumer, never baked into the stamp).
    #[serde(default)]
    pub journey_residual_sum: u32,
    /// Issue 105 (run-level, stamped at dump time): the run's terminal
    /// residual — `masking_after + errors_after` of the LAST attempt
    /// (0 when the run made no attempts, the neutral point).
    #[serde(default)]
    pub terminal_residual: u32,
    /// Issue 107 T2 — the span-shape embedding at ATTEMPT TIME: hashed
    /// unigram+bigram bag over the ORIGINAL trigger text (`shape`),
    /// L2-normalized, ×127 i8-quantized (integer-stamped so the record
    /// keeps `Eq` — the journey-credit precedent). Captured at write time
    /// because the site's bytes DRIFT as fixes land and `span_hash` is a
    /// site ID, not a content hash — offline re-embedding would read
    /// post-run bytes. `None` on pre-T2 rings.
    #[serde(default)]
    pub span_embed: Option<[i8; SPAN_EMBED_DIMS]>,
    /// Issue 140 T3 — the proposing diagnostic's normalized shape class
    /// (the retry-family key's shape half, [`crate::insight::shape_class`]).
    /// `None` on pre-140 rings and digest-less attempts.
    #[serde(default)]
    pub shape_class: Option<String>,
    /// Issue 140 T3 — the distilled insight text at attempt time
    /// ([`crate::insight::distill`]) — the admission predicate's subject.
    /// NOT filtered: the ring records the honest text; admission is the
    /// consuming policy's judgment, never baked into the record. `None`
    /// on pre-140 rings.
    #[serde(default)]
    pub insight: Option<String>,
}

/// The loop outcome of one [`AttemptRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AttemptOutcome {
    /// Applied and kept — `(masking, count)` strictly decreased
    /// lexicographically (Issue 100; before it: the count alone).
    Kept = 0,
    /// Reverted: no progress (the lex pair did not decrease, or the span
    /// drifted).
    RevertedNoProgress = 1,
    /// Reverted: i/o (the file could not be read or written).
    RevertedIo = 2,
}

impl AttemptOutcome {
    /// The stable render label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Kept => "kept",
            Self::RevertedNoProgress => "reverted:no-progress",
            Self::RevertedIo => "reverted:io",
        }
    }
}

/// Issue 105 — the run-level journey credit (PC-ALM's dual accumulator,
/// katgpt-rs Research 554): credit is the **integral of the residual
/// history plus the terminal residual**, never the terminal verdict alone.
/// A keep that ends its run at a typeck-masked E0614 (the "strand-keep",
/// the Bench 089/090 poison) has a legal per-attempt lexicographic delta
/// but a large `journey_residual_sum` AND `terminal_residual` — the
/// journey's error is the credit.
///
/// Integer-stamped (u32) so [`AttemptRecord`] keeps `Eq`; α/ρ are applied
/// by the consumer ([`JourneyCredit::poverty`]), never baked into the
/// stamp — the sweep discipline stays open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourneyCredit {
    /// `λ_T`'s raw material: `Σ_t r_t` over every attempt, where
    /// `r_t` = `masking_after + errors_after` (the state each attempt left
    /// behind — kept or reverted; a revert restores the state, so its
    /// after-values already read as the restored state).
    pub journey_residual_sum: u32,
    /// `r_T`: the last attempt's `masking_after + errors_after` (0 when the
    /// run made no attempts — the neutral point).
    pub terminal_residual: u32,
    /// The last attempt's `masking_after` — the TAINT condition's input:
    /// a run that ends with a masking-stage error present is the poison
    /// class, however legal each individual keep was. A run that ends at
    /// masking 0 with ordinary errors remaining is HONEST partial progress
    /// (the floor's normal shape) and is never tainted.
    pub terminal_masking: u32,
}

/// α — the dual step on the journey integral (PC-ALM's α). 0.25 keeps a
/// typical 2–4-attempt journey inside sigmoid's responsive band.
pub const JOURNEY_ALPHA: f32 = 0.25;
/// ρ — the terminal residual's weight in the composite (PC-ALM's ρ). The
/// terminal masking state is the poison's defining signal, so it outweighs
/// any single journey term.
pub const JOURNEY_RHO: f32 = 2.0;

impl JourneyCredit {
    /// Fold a run's ordered attempt ring into the credit triple.
    #[must_use]
    pub fn from_attempts(attempts: &[AttemptRecord]) -> Self {
        let mut credit = Self::default();
        for a in attempts {
            // Coded-error totals are single-run diagnostics (~10⁰–10¹);
            // saturating conversion is exact for every real value.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let r = (a.masking.1 + a.errors.1) as u32;
            credit.journey_residual_sum = credit.journey_residual_sum.saturating_add(r);
            credit.terminal_residual = r;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let tm = a.masking.1 as u32;
            credit.terminal_masking = tm;
        }
        credit
    }

    /// The composite poverty `J = sigmoid(α·λ_T + ρ·r_T)` ∈ (0,1).
    /// 0.5 = neutral (no journey data); →1 = the journey never resolved
    /// (the strand-keep class). HIGH means the successes this run recorded
    /// rode an error-heavy journey — down-weight their CREDIT, never the
    /// keep rule (Issue 100's lexicographic rule stands untouched).
    #[must_use]
    pub fn poverty(&self, alpha: f32, rho: f32) -> f32 {
        // Counts are small (< 2^24); f32 is exact.
        #[allow(clippy::cast_precision_loss)]
        let (lam, res) = (
            self.journey_residual_sum as f32,
            self.terminal_residual as f32,
        );
        katgpt_core::sigmoid(alpha * lam + rho * res)
    }

    /// The taint condition: the run ended with a masking-stage error
    /// present. Only tainted runs contribute a composite to the per-key
    /// accumulator — clean runs bump the journey count alone.
    #[must_use]
    pub fn tainted(&self) -> bool {
        self.terminal_masking > 0
    }

    /// The composite as a ×256 fixed-point u32 (the [`TrajRowDoc`] stamp —
    /// integer-exact, saturating, decode-stable). Callers invoke this ONLY
    /// on tainted runs; clean runs stamp nothing.
    #[must_use]
    pub fn taint_composite_256(&self) -> u32 {
        // Counts are small (< 2^24); f32 is exact.
        #[allow(clippy::cast_precision_loss)]
        let (lam, res) = (
            self.journey_residual_sum as f32,
            self.terminal_residual as f32,
        );
        let c = JOURNEY_ALPHA * lam + JOURNEY_RHO * res;
        // Journey magnitudes are small (counts ~10⁰–10¹); ×256 cannot
        // approach u32::MAX before saturating honestly.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            (c * 256.0) as u32
        }
    }
}

/// Stable site identity for an attempt: blake3(`"file:line"`), first 16
/// hex — deterministic across runs and machines (the oscillation detector
/// joins attempts across the run by it).
#[must_use]
pub fn span_hash(file: &Path, line: usize) -> String {
    let full = blake3::hash(format!("{}:{}", file.display(), line).as_bytes());
    let hex = full.to_hex();
    hex[..16].to_string()
}

/// One applied-and-kept fix, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedRecord {
    pub file: PathBuf,
    pub line: usize,
    pub code: String,
    pub strategy_id: String,
    /// Coded-error count before → after this fix.
    pub errors: (usize, usize),
    /// The trigger text the candidate replaced (the ORIGINAL bytes at the
    /// primary span) — the Phase 4 trajectory's `buggy` half.
    pub shape: String,
    /// The replacement text landed for the primary edit — the trajectory's
    /// `fixed` half.
    pub fix: String,
    /// The T6.10 semantic-note annotation (see [`RevertedRecord`]).
    pub semantic_note: Option<&'static str>,
    /// Issue 148 T-b follow-up: the strategy-menu label — the OTHER
    /// strategy ids the driver considered for this error site (same
    /// file/line/code, the post-filter candidate set the choice was made
    /// from) when it chose this one, sorted + deduped, capped at the
    /// store's `REJECTED_CAP` where that constant is compiled. Empty = the
    /// chosen strategy was the only candidate (or no menu was resolved) —
    /// the store reads empty as `None`, never erasing a label.
    pub rejected: Vec<String>,
}

/// One tried-and-reverted candidate, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevertedRecord {
    pub file: PathBuf,
    pub line: usize,
    pub code: String,
    pub strategy_id: String,
    /// Why it reverted.
    pub reason: RevertReason,
    /// The trigger text (trajectory `buggy`) — the oracle rejected THIS
    /// move at THIS shape, which is exactly the fact the memory wants.
    pub shape: String,
    /// The attempted replacement (trajectory `fixed`).
    pub fix: String,
    /// The T6.10 semantic-note annotation, when the strategy's fix is in
    /// the semantics-changing class (clone-instead-of-borrow, drop-order
    /// change, ownership transfer) — surfaced in the report + CLI diff so
    /// the change is NAMED, never silent.
    pub semantic_note: Option<&'static str>,
    /// Issue 148 T-b follow-up: the same strategy-menu label as
    /// [`AppliedRecord::rejected`] — the alternatives considered at this
    /// site when the oracle rejected THIS move (the contrastive negatives
    /// a refine-rule suite mint trains against).
    pub rejected: Vec<String>,
}

/// Why a candidate was reverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertReason {
    /// Error count did not decrease (no-op fix, new error, or the error
    /// moved).
    NoProgress,
    /// The file could not be read back or written (I/O).
    Io,
}

impl RevertReason {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::NoProgress => "no progress",
            Self::Io => "i/o",
        }
    }
}

/// A coded error no playbook proposed for — the template-authoring queue
/// (Phase 4 T4.4 dumps these to `<repo>/.refine/misses/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissRecord {
    pub code: String,
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
    /// The full pre-rendered diagnostic, when cargo included it — the
    /// miss dump writes this verbatim (the plan's T4.4).
    pub rendered: Option<String>,
    /// Component-attributed failure class (Issue 047). A `rustc_errors` miss
    /// is a no-candidate miss by construction (no template proposed, or the
    /// diagnostic was unlocalizable/unreadable before candidates could be
    /// collected) — always E (corpus gap: the playbook lacks the code).
    pub component: crate::component::ComponentClass,
}

/// A review-mode plan line (no writes happened).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRecord {
    pub file: PathBuf,
    pub line: usize,
    pub code: String,
    pub strategy_id: String,
    /// First line of the replacement, for the printed plan.
    pub replacement_head: String,
    /// The T6.10 semantic note, when the strategy carries one.
    pub semantic_note: Option<&'static str>,
}

/// The outcome of a [`CompileDriver::run`].
#[derive(Debug, Clone, Default)]
pub struct FixCompileReport {
    /// Whether cargo ran at all (false: no packages / cargo missing).
    pub ran: bool,
    /// cargo check invocations.
    pub n_checks: usize,
    /// Loop iterations executed (write mode only).
    pub iterations: usize,
    /// Fixes applied and kept.
    pub applied: Vec<AppliedRecord>,
    /// Candidates tried and reverted.
    pub reverted: Vec<RevertedRecord>,
    /// Errors with no candidate at the final proposal pass.
    pub misses: Vec<MissRecord>,
    /// Review-mode plan (empty in write mode).
    pub planned: Vec<PlannedRecord>,
    /// Coded errors at the final check.
    pub remaining: usize,
    /// Zero coded errors at the final check.
    pub clean: bool,
    /// Wall time of each write-mode loop iteration, milliseconds (the G2
    /// latency axis — incremental `cargo check` dominates each entry;
    /// measured release for the published p50/p99).
    pub iter_ms: Vec<f64>,
    /// Why the run could not complete (cargo could not run, unreadable
    /// package...).
    pub error: Option<String>,
    /// The ORDERED per-attempt ring (Issue 91 T1.5) — every candidate the
    /// loop tried, in try order, regardless of outcome. Empty in review
    /// mode (no attempts are made) and on a clean first check. Dumped per
    /// run to `<root>/.refine/fixseq/run-<ts>.jsonl` by the CLI (the
    /// `dump_misses` law: an aid, never a gate).
    pub attempts: Vec<AttemptRecord>,
    /// Issue 118 T1: bench-target error sites DECLINED by the bench guard
    /// (distinct `(file, line, code)` — strategies for one site count
    /// once). The routine write path leaves these standing, loudly.
    pub bench_declined: usize,
    /// Issue 118 T1: distinct bench files whose candidates were declined.
    pub bench_declined_files: Vec<PathBuf>,
    /// Issue 118 T3: bench files that proceeded under the explicit-path
    /// override (flagged, not declined — the operator named the file).
    pub bench_forced_files: Vec<PathBuf>,
}

impl FixCompileReport {
    /// Issue 105: the run's journey credit, folded from the attempt ring
    /// on demand (computed, never stored — the ring is the source of
    /// truth, and the early-return arms of `run` stay untouched).
    #[must_use]
    pub fn journey(&self) -> JourneyCredit {
        JourneyCredit::from_attempts(&self.attempts)
    }
    /// Append one attempt to the ring (seq = current length — the push
    /// order IS the try order). Takes the CANDIDATE (not its scattered
    /// fields) so the Issue-140-T3 digest rides along at the one chokepoint
    /// every attempt path funnels through — the same single-point law as
    /// `span_embed`; `best.shape` is the span's ORIGINAL bytes (embedded
    /// here, at push time).
    fn push_attempt(
        &mut self,
        best: &FixCandidate,
        outcome: AttemptOutcome,
        errors: (usize, usize),
        masking: (usize, usize),
    ) {
        let seq = self.attempts.len();
        self.attempts.push(AttemptRecord {
            seq,
            code: best.code.clone(),
            span_hash: span_hash(&best.file, best.line),
            strategy_id: best.strategy_id.clone(),
            outcome,
            errors,
            masking,
            span_embed: Some(embed_shape(&best.shape)),
            shape_class: best.diag_digest.as_ref().map(|d| d.shape_class.clone()),
            insight: best.diag_digest.as_ref().map(|d| d.insight.clone()),
            // Run-level Issue-105 values are unknown at push time; the
            // dump stamps them over these zeros.
            journey_residual_sum: 0,
            terminal_residual: 0,
        });
    }
}

/// Candidate-ordering key: stable (file, line, code) with the shortest
/// TOTAL replacement first — the Phase 1 "min-delta" heuristic (primary +
/// extra edits; a two-byte pair beats a `.clone()` insertion). Phase 4 T4.2
/// swaps in `BetaPosterior` ordering over `(error_code, strategy_id)`
/// memory if it measures a gain; this function is the swappable seam.
///
/// `pub` since Issue 140 T3: the insight-conditioned retry policy's
/// UNARMED fallback is this exact order (a family whose running success
/// exceeds the consult gate must observe byte-identical behavior to the
/// static arm — a policy that reordered even while unarmed would smuggle
/// a second difference into the control comparison).
pub fn order_candidates(cands: &mut [FixCandidate]) {
    cands.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.code.cmp(&b.code))
            .then(a.total_len().cmp(&b.total_len()))
            .then(a.strategy_id.cmp(&b.strategy_id))
    });
}

/// Identity of a tried candidate — retries across iterations must not
/// re-apply the same failed move at the same site. Includes every edit
/// (primary + extras) so two strategies differing only in their extra
/// edits stay distinct. `pub(crate)` so the Issue-093 revive policy can
/// key its suppression records identically.
pub(crate) type TriedKey = (
    PathBuf,
    String,
    usize,
    usize,
    String,
    Vec<((usize, usize), String)>,
);

pub(crate) fn tried_key(c: &FixCandidate) -> TriedKey {
    (
        c.file.clone(),
        c.strategy_id.clone(),
        c.byte_span.0,
        c.byte_span.1,
        c.replacement.clone(),
        c.extra_edits
            .iter()
            .map(|e| (e.byte_span, e.replacement.clone()))
            .collect(),
    )
}

/// The T4.2 ordering-hook signature (see [`CompileDriver::with_ordering`]).
pub type OrderingFn = std::sync::Arc<dyn Fn(&mut [FixCandidate]) + Send + Sync>;

/// The per-attempt observer signature (Issue 093 T2): called once per
/// completed attempt with the ring record (code, strategy, outcome, errors
/// before→after) — the in-process feed the per-run strategy posterior
/// consumes (the offline twin is the `.refine/fixseq/` ring dump).
pub type ObserverFn = std::sync::Arc<dyn Fn(&AttemptRecord) + Send + Sync>;

/// The tried-candidate revive signature (Issue 093 T2): consulted for a
/// candidate already in the `tried` set; `true` re-admits it for one more
/// attempt. The default policy (no hook) never revives — suppression is
/// permanent, the shipped behavior.
pub type ReviveFn = std::sync::Arc<dyn Fn(&FixCandidate) -> bool + Send + Sync>;

/// The bounded fix-compile loop driver (Plan 019 Phase 1).
pub struct CompileDriver {
    runner: CheckRunner,
    max_iters: usize,
    /// Optional candidate-ordering override (the T4.2 seam): when set,
    /// replaces the static min-delta [`order_candidates`] — the
    /// shape-conditional `BetaPosterior` ordering installs here (built by
    /// the CLI over a [`crate::traj_store::TrajStore`], `latent_retrieval`
    /// builds). The closure composes its own tiebreak.
    ordering: Option<OrderingFn>,
    /// Optional per-attempt observer (Issue 093 T2) — see [`ObserverFn`].
    observer: Option<ObserverFn>,
    /// Optional tried-candidate revive policy (Issue 093 T2) — see
    /// [`ReviveFn`]. When unset, the retain below drops every tried
    /// candidate unconditionally (the shipped never-revive behavior).
    revive: Option<ReviveFn>,
}

/// Issue 118 T3: the explicit-path override set. A bench-file candidate
/// whose file one of the caller's paths NAMES — the exact file, or a
/// `benches` directory above it — proceeds under a recorded flag instead
/// of the routine decline; naming the target IS the operator's consent.
/// Matching is best-effort canonicalized then lexical (see
/// [`crate::bench_guard::user_path_forces`]).
struct ForcedPaths {
    roots: Vec<PathBuf>,
}

impl ForcedPaths {
    fn new(user: &[PathBuf]) -> Self {
        Self {
            roots: user.to_vec(),
        }
    }

    fn matches(&self, file: &Path) -> bool {
        self.roots
            .iter()
            .any(|u| crate::bench_guard::user_path_forces(u, file))
    }
}

impl Default for CompileDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl CompileDriver {
    /// Environment-inheriting runner, `max_iters = 5` (the plan default).
    #[must_use]
    pub fn new() -> Self {
        Self {
            runner: CheckRunner::new(),
            max_iters: 5,
            ordering: None,
            observer: None,
            revive: None,
        }
    }

    /// Environment-isolated runner (scaffold crates — see
    /// [`CheckRunner::isolated`]).
    #[must_use]
    pub fn isolated() -> Self {
        Self {
            runner: CheckRunner::isolated(),
            max_iters: 5,
            ordering: None,
            observer: None,
            revive: None,
        }
    }

    /// Override the iteration bound (`--max-iters`).
    #[must_use]
    pub fn with_max_iters(mut self, n: usize) -> Self {
        self.max_iters = n;
        self
    }

    /// Install a candidate-ordering override (the T4.2 seam — see the
    /// field docs). The closure receives every candidate of an
    /// iteration, each carrying its trigger `shape` (filled at proposal
    /// time), and must leave them in apply order.
    #[must_use]
    pub fn with_ordering(mut self, f: OrderingFn) -> Self {
        self.ordering = Some(f);
        self
    }

    /// Install the per-attempt observer (Issue 093 T2 — see [`ObserverFn`]).
    /// Called after EVERY attempt outcome site (kept / no-progress /
    /// io×2 / span-drift), in try order, with the ring record.
    #[must_use]
    pub fn with_observer(mut self, f: ObserverFn) -> Self {
        self.observer = Some(f);
        self
    }

    /// Install the tried-candidate revive policy (Issue 093 T2 — see
    /// [`ReviveFn`]). A `true` verdict re-admits a tried candidate into
    /// an iteration's candidate list; the driver re-inserts it into the
    /// `tried` set on failure exactly as before, so the policy owns
    /// one-shot semantics.
    #[must_use]
    pub fn with_revive(mut self, f: ReviveFn) -> Self {
        self.revive = Some(f);
        self
    }

    /// Fire the observer on the ring's newest record, if armed.
    fn observe(&self, report: &FixCompileReport) {
        if let (Some(o), Some(rec)) = (&self.observer, report.attempts.last()) {
            o(rec);
        }
    }

    /// Extra cargo args for every check (`--verify-args` passthrough).
    #[must_use]
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.runner = self.runner.with_extra_args(args);
        self
    }

    /// Explicit cargo binary (test/diagnostic seam — mirrors
    /// [`crate::fix_verify::FixVerifier::with_cargo_bin`]).
    #[must_use]
    pub fn with_cargo_bin(bin: impl Into<String>) -> Self {
        Self {
            runner: CheckRunner::new().with_cargo_bin(bin),
            ..Self::new()
        }
    }

    /// Run over the packages owning `paths`.
    ///
    /// `write == false` → review mode: one check + one proposal pass,
    /// nothing written, the plan in [`FixCompileReport::planned`].
    /// `write == true` → the bounded loop; disk holds exactly the kept
    /// fixes on return.
    ///
    /// Issue 118: in write mode, bench-target candidates are DECLINED
    /// (or, when `paths` names the file / its `benches/` dir explicitly,
    /// flagged and proceeded) — see the `bench_*` report fields. Review
    /// mode is untouched: it writes nothing, so there is nothing to guard.
    ///
    /// The loop body is intentionally one unit: propose → pick → apply →
    /// recheck → keep/revert share the `tried` set + the diagnostic state,
    /// and splitting it would thread four locals through helper
    /// signatures without clarifying any step (the `fix_verify` precedent).
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn run(&self, paths: &[PathBuf], playbooks: &Playbooks, write: bool) -> FixCompileReport {
        let mut report = FixCompileReport::default();

        // Scope: the owning packages of the given paths (deduped, sorted
        // for deterministic check order).
        let mut pkgs: Vec<PathBuf> = Vec::new();
        for p in paths {
            let Some(pkg) = owning_package_root(p) else {
                report.error = Some(format!("no Cargo.toml above {}", p.display()));
                return report;
            };
            if !pkgs.contains(&pkg) {
                pkgs.push(pkg);
            }
        }
        if pkgs.is_empty() {
            return report;
        }
        pkgs.sort();

        let mut diags = match self.check_all(&pkgs, &mut report) {
            Err(e) => {
                report.error = Some(e);
                return report;
            }
            Ok(d) => d,
        };
        report.ran = true;
        if diags.is_empty() {
            // The T2.3 gate: code that already compiles → nothing to fix,
            // zero edits. This arm is what the founding incident's
            // unnecessary fix would have tripped over.
            report.clean = true;
            return report;
        }

        if !write {
            // Review mode: propose once, plan, done. No writes.
            for d in &diags {
                collect_candidates(playbooks, d, &mut report, true);
            }
            report.remaining = diags.len();
            return report;
        }

        // ── Write mode: the bounded loop ─────────────────────────────────
        let mut tried: HashSet<TriedKey> = HashSet::new();
        // Issue 118: the explicit-path override set + the bench-guard
        // bookkeeping (distinct sites/files, so per-iteration re-proposals
        // of the same declined error never double-count).
        let forced = ForcedPaths::new(paths);
        let mut bench_flagged: HashSet<PathBuf> = HashSet::new();
        let mut bench_files_declined: HashSet<PathBuf> = HashSet::new();
        let mut bench_sites: HashSet<(PathBuf, usize, String)> = HashSet::new();
        for _iter in 0..self.max_iters {
            let iter_t0 = std::time::Instant::now();
            // Propose for every current diagnostic.
            let mut candidates: Vec<FixCandidate> = Vec::new();
            for d in &diags {
                candidates.extend(collect_candidates(playbooks, d, &mut report, false));
            }
            // Fill each candidate's trigger shape (the T4.2 ordering key)
            // from the CURRENT source, then order.
            for c in &mut candidates {
                if let Ok(src) = std::fs::read_to_string(&c.file) {
                    c.shape = src
                        .get(c.byte_span.0..c.byte_span.1)
                        .unwrap_or("")
                        .to_string();
                }
            }
            match &self.ordering {
                Some(f) => f(&mut candidates),
                None => order_candidates(&mut candidates),
            }
            // Issue 093 T2: a revive policy may re-admit a TRIED candidate
            // (context-scoped, one-shot by the policy's own bookkeeping);
            // the default (no hook) keeps the shipped never-revive retain.
            let revive = self.revive.clone();
            candidates.retain(|c| {
                !tried.contains(&tried_key(c)) || revive.as_ref().is_some_and(|r| r(c))
            });
            // Issue 118 T1 (the P30 lane shape, one domain over): a bench
            // target's numbers ARE its artifact, and a compile repair can
            // alter WHAT the bench measures — hoisting a loop, changing a
            // type's size/copy semantics, deleting the "dead" result that
            // was the timing subject (the exact specimen class that
            // motivated P30 was itself compile-shaped). The routine write
            // path DECLINES bench-file candidates; an explicit user path
            // naming the file (or its `benches/` dir) overrides with a
            // recorded flag, never silently (the T3 posture). The decision
            // lands in the report; the CLI prints it loudly.
            let mut allowed: Vec<FixCandidate> = Vec::with_capacity(candidates.len());
            for c in candidates.drain(..) {
                let is_bench = std::fs::read_to_string(&c.file).is_ok_and(|src| {
                    crate::bench_guard::is_bench_target(&c.file.to_string_lossy(), &src)
                });
                if !is_bench || forced.matches(&c.file) {
                    if is_bench && bench_flagged.insert(c.file.clone()) {
                        report.bench_forced_files.push(c.file.clone());
                    }
                    allowed.push(c);
                    continue;
                }
                if bench_files_declined.insert(c.file.clone()) {
                    report.bench_declined_files.push(c.file.clone());
                }
                if bench_sites.insert((c.file.clone(), c.line, c.code.clone())) {
                    report.bench_declined += 1;
                }
            }
            candidates = allowed;
            let Some(best) = candidates.first().cloned() else {
                break; // no untried candidates left — stop (misses recorded)
            };
            // Issue 148 T-b follow-up: the strategy menu for THIS decision —
            // the considered alternatives at best's site minus best itself,
            // captured once and carried on every record the attempt can
            // produce (kept or reverted; the menu describes what was
            // considered, not how the attempt ended).
            let rejected_menu = strategy_menu(&candidates, &best);

            report.iterations += 1;
            let before_errors = diags.len();
            let before_masking = masking_count(&diags);

            // Apply: splice + write.
            let Ok(original) = std::fs::read_to_string(&best.file) else {
                report.push_attempt(
                    &best,
                    AttemptOutcome::RevertedIo,
                    (before_errors, before_errors),
                    (before_masking, before_masking),
                );
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::Io,
                    shape: String::new(),
                    fix: String::new(),
                    semantic_note: best.semantic_note,
                    rejected: rejected_menu.clone(),
                });
                tried.insert(tried_key(&best));
                self.observe(&report);
                continue;
            };
            let shape = original
                .get(best.byte_span.0..best.byte_span.1)
                .unwrap_or("")
                .to_string();
            let Some(applied_text) = apply_edits(
                &original,
                best.byte_span,
                &best.replacement,
                &best.extra_edits,
            ) else {
                // Span drifted (should not happen — proposed against the
                // same read) — treat as no-progress revert.
                report.push_attempt(
                    &best,
                    AttemptOutcome::RevertedNoProgress,
                    (before_errors, before_errors),
                    (before_masking, before_masking),
                );
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::NoProgress,
                    shape,
                    fix: best.replacement.clone(),
                    semantic_note: best.semantic_note,
                    rejected: rejected_menu.clone(),
                });
                tried.insert(tried_key(&best));
                self.observe(&report);
                continue;
            };
            if std::fs::write(&best.file, &applied_text).is_err() {
                report.push_attempt(
                    &best,
                    AttemptOutcome::RevertedIo,
                    (before_errors, before_errors),
                    (before_masking, before_masking),
                );
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::Io,
                    shape,
                    fix: best.replacement.clone(),
                    semantic_note: best.semantic_note,
                    rejected: rejected_menu.clone(),
                });
                tried.insert(tried_key(&best));
                self.observe(&report);
                continue;
            }

            // Recheck.
            let new_diags = match self.check_all(&pkgs, &mut report) {
                Err(e) => {
                    // cargo broke mid-loop: restore the file, report, stop.
                    let _ = std::fs::write(&best.file, &original);
                    report.error = Some(e);
                    break;
                }
                Ok(d) => d,
            };

            // Issue 100 — the keep rule is the LEXICOGRAPHIC strict decrease
            // of (masking-stage count, total count). A count decrease that
            // INCREASES masking is a revert (the measured clone-out strand:
            // trading two borrowck errors for one typeck error that blocks
            // borrowck is regress, not progress); an equal-count trade that
            // DROPS masking is a keep (the unmasking trade: a typeck error
            // resolved, revealing the fixable borrowck errors it masked).
            // Well-founded on ℕ× ℕ — the termination argument survives.
            let after_errors = new_diags.len();
            let after_masking = masking_count(&new_diags);
            let kept = (after_masking, after_errors) < (before_masking, before_errors);
            if kept {
                report.push_attempt(
                    &best,
                    AttemptOutcome::Kept,
                    (before_errors, after_errors),
                    (before_masking, after_masking),
                );
                report.applied.push(AppliedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    errors: (before_errors, after_errors),
                    shape,
                    fix: best.replacement.clone(),
                    semantic_note: best.semantic_note,
                    rejected: rejected_menu.clone(),
                });
                diags = new_diags;
                self.observe(&report);
                if diags.is_empty() {
                    report.clean = true;
                    report
                        .iter_ms
                        .push(iter_t0.elapsed().as_secs_f64() * 1000.0);
                    break;
                }
            } else {
                // No progress → exact revert (T1.4).
                let _ = std::fs::write(&best.file, &original);
                report.push_attempt(
                    &best,
                    AttemptOutcome::RevertedNoProgress,
                    (before_errors, after_errors),
                    (before_masking, after_masking),
                );
                report.reverted.push(RevertedRecord {
                    file: best.file.clone(),
                    line: best.line,
                    code: best.code.clone(),
                    strategy_id: best.strategy_id.clone(),
                    reason: RevertReason::NoProgress,
                    shape,
                    fix: best.replacement.clone(),
                    semantic_note: best.semantic_note,
                    rejected: rejected_menu.clone(),
                });
                tried.insert(tried_key(&best));
                self.observe(&report);
            }
            report
                .iter_ms
                .push(iter_t0.elapsed().as_secs_f64() * 1000.0);
        }

        report.remaining = diags.len();
        report.clean = diags.is_empty();
        report
    }

    /// Check every package, parse + dedupe + resolve diagnostics to
    /// absolute paths. `Err` when cargo could not run, AND when a check
    /// did not complete (non-zero exit, zero parseable diagnostics — the
    /// Issue 841 kill shape: a killed recheck with no attributable errors
    /// reads as (0, 0) on the keep rule's lexicographic measure, the
    /// maximum decrease, and would keep a fix nothing verified).
    fn check_all(
        &self,
        pkgs: &[PathBuf],
        report: &mut FixCompileReport,
    ) -> Result<Vec<ResolvedDiag>, String> {
        let mut out: Vec<ResolvedDiag> = Vec::new();
        for pkg in pkgs {
            let outcome = self.runner.run_check(pkg)?;
            report.n_checks += 1;
            let diags = dedupe_by_site(&parse_error_diags(&outcome.json));
            if check_verdict(outcome.exit_ok, !diags.is_empty()) == CheckVerdict::Incomplete {
                return Err(format!(
                    "cargo check in {} exited unsuccessfully with no error diagnostics — the \
                     check did not complete (killed, output lost, or a cargo-level refusal); \
                     no fix is kept on an incomplete recheck",
                    pkg.display()
                ));
            }
            for d in diags {
                out.push(ResolvedDiag {
                    diag: d,
                    pkg: pkg.clone(),
                });
            }
        }
        Ok(out)
    }
}

/// A diagnostic resolved against its reporting package.
struct ResolvedDiag {
    diag: RustcErrorDiag,
    pkg: PathBuf,
}

/// Count diagnostics whose code sits at a masking stage
/// ([`crate::domains::rustc_errors::diag::MASKING_STAGE_CODES`]) — the
/// primary term of the keep rule's lexicographic measure (Issue 100).
fn masking_count(diags: &[ResolvedDiag]) -> usize {
    diags.iter().filter(|d| d.diag.is_masking()).count()
}

/// Issue 148 T-b follow-up (the driver lane's suite-mint label): the
/// strategy menu behind one apply decision — the considered candidates'
/// strategy ids at the CHOSEN candidate's error site (same file, line,
/// code — the diagnostic identity after `dedupe_by_site`), MINUS the
/// chosen strategy. The input is the post-filter candidate set the choice
/// was made from (revive-suppressed and bench-declined candidates were
/// not considered, so they are not in the menu). Sorted + deduped like
/// every label; capped at [`crate::traj_store::REJECTED_CAP`] where
/// that constant is compiled (`latent_retrieval` owns it — the only build
/// with a store consumer; without it the playbook-bounded menu travels
/// whole and any store that ever reads it clamps). Empty (solo candidate)
/// reads as `None` downstream — a missing menu never erases one.
#[must_use]
fn strategy_menu(candidates: &[FixCandidate], chosen: &FixCandidate) -> Vec<String> {
    let mut ids: Vec<String> = candidates
        .iter()
        .filter(|c| c.file == chosen.file && c.line == chosen.line && c.code == chosen.code)
        .map(|c| c.strategy_id.clone())
        .filter(|id| id != &chosen.strategy_id)
        .collect();
    ids.sort();
    ids.dedup();
    #[cfg(feature = "latent_retrieval")]
    ids.truncate(crate::traj_store::REJECTED_CAP);
    ids
}

/// Resolve a diagnostic's primary span to an absolute path, read the
/// source, and propose. In review mode the candidates land in
/// `report.planned`; in write mode they're returned for ordering. Records
/// a miss when the primary span is absent, the file is unreadable, or no
/// playbook proposes.
fn collect_candidates(
    playbooks: &Playbooks,
    rd: &ResolvedDiag,
    report: &mut FixCompileReport,
    review: bool,
) -> Vec<FixCandidate> {
    let d = &rd.diag;
    let Some((file_name, line)) = d.primary.clone() else {
        report.misses.push(MissRecord {
            code: d.code.clone(),
            file: PathBuf::from(&file_name_placeholder(&d.message)),
            line: 0,
            message: d.message.clone(),
            rendered: d.rendered.clone(),
            component: crate::component::classify(
                &crate::component::FailureSignal::miss_no_candidates(),
            ),
        });
        return Vec::new();
    };
    let ws = workspace_root(&rd.pkg).unwrap_or_else(|| rd.pkg.clone());
    let file = resolve_reported_path(Path::new(&file_name), &ws, &rd.pkg);
    let Ok(source) = std::fs::read_to_string(&file) else {
        report.misses.push(MissRecord {
            code: d.code.clone(),
            file,
            line,
            message: format!("(unreadable) {}", d.message),
            rendered: d.rendered.clone(),
            component: crate::component::classify(
                &crate::component::FailureSignal::miss_no_candidates(),
            ),
        });
        return Vec::new();
    };
    let cands = playbooks.propose(d, &file, &source);
    if cands.is_empty() {
        report.misses.push(MissRecord {
            code: d.code.clone(),
            file,
            line,
            message: d.message.clone(),
            rendered: d.rendered.clone(),
            component: crate::component::classify(
                &crate::component::FailureSignal::miss_no_candidates(),
            ),
        });
        return Vec::new();
    }
    // Issue 140 T3 — the retry-conditioning digest, computed ONCE per
    // diagnostic (the span_embed law: the message is current only here)
    // and carried on every candidate the diagnostic proposed.
    let digest = super::playbooks::DiagDigest::new(d);
    let mut cands = cands;
    for c in &mut cands {
        c.diag_digest = Some(digest.clone());
    }
    if review {
        for c in &cands {
            report.planned.push(PlannedRecord {
                file: c.file.clone(),
                line: c.line,
                code: c.code.clone(),
                strategy_id: c.strategy_id.clone(),
                replacement_head: c.replacement.lines().next().unwrap_or("").to_string(),
                semantic_note: c.semantic_note,
            });
        }
    }
    cands
}

fn file_name_placeholder(msg: &str) -> String {
    format!("(no primary span) {msg}")
}

/// Replace `text[span]` with `replacement`, then apply `extra_edits` — all
/// in one atomic pass (spans applied lowest-first with the consumed prefix
/// tracked, so every offset is used exactly once against the ORIGINAL
/// text). `None` when any span is out of bounds, off a char boundary, or
/// OVERLAPS another (the caller treats it as a failed apply; a candidate
/// whose edits collide is malformed and must never half-land).
#[must_use]
fn apply_edits(
    text: &str,
    span: (usize, usize),
    replacement: &str,
    extras: &[SpanEdit],
) -> Option<String> {
    let mut all: Vec<((usize, usize), &str)> = Vec::with_capacity(extras.len() + 1);
    all.push((span, replacement));
    all.extend(extras.iter().map(|e| (e.byte_span, e.replacement.as_str())));
    all.sort_by_key(|((s, _), _)| *s);

    let mut out = String::with_capacity(text.len() + replacement.len());
    let mut cursor = 0usize;
    for ((s, e), repl) in all {
        if s < cursor {
            return None; // overlapping edits
        }
        if !text.is_char_boundary(s) || !text.is_char_boundary(e) || e < s || e > text.len() {
            return None;
        }
        out.push_str(&text[cursor..s]);
        out.push_str(repl);
        cursor = e;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 91 T1.5 — the span identity is STABLE (same site → same hash,
    /// across calls) and DISCRIMINATING (different site → different hash);
    /// 16 hex chars.
    #[test]
    fn span_hash_is_stable_and_discriminating() {
        let a = span_hash(Path::new("/x/src/lib.rs"), 7);
        let a2 = span_hash(Path::new("/x/src/lib.rs"), 7);
        let b = span_hash(Path::new("/x/src/lib.rs"), 8);
        let c = span_hash(Path::new("/x/src/other.rs"), 7);
        assert_eq!(a, a2, "same site is stable");
        assert_ne!(a, b, "line discriminates");
        assert_ne!(a, c, "file discriminates");
        assert_eq!(a.len(), 16, "16 hex chars: {a}");
    }

    /// Issue 148 T-b follow-up — the driver lane's strategy-menu label: the
    /// considered candidates at the CHOSEN site minus the chosen strategy
    /// (other sites and other codes excluded, duplicates deduped, sorted),
    /// capped at the store's label cap where it is compiled, and EMPTY for a
    /// solo candidate (which the store reads as `None` — never erasing).
    #[test]
    fn strategy_menu_is_menu_minus_chosen_at_the_chosen_site() {
        let cand = |file: &str, line: usize, code: &str, strategy: &str| FixCandidate {
            strategy_id: strategy.to_string(),
            code: code.to_string(),
            file: PathBuf::from(file),
            line,
            byte_span: (0, 1),
            replacement: String::new(),
            shape: String::new(),
            semantic_note: None,
            diag_digest: None,
            extra_edits: Vec::new(),
        };
        // The chosen site carries three strategies (one repeated — dedup);
        // the other line and the other code are DIFFERENT decisions.
        let candidates = vec![
            cand("a.rs", 3, "E0502", "s-b"),
            cand("a.rs", 3, "E0502", "s-a"),
            cand("a.rs", 3, "E0502", "s-b"),
            cand("a.rs", 7, "E0502", "s-far"),
            cand("a.rs", 3, "E0499", "s-othercode"),
        ];
        let chosen = cand("a.rs", 3, "E0502", "s-a");
        assert_eq!(strategy_menu(&candidates, &chosen), vec!["s-b".to_string()]);
        // Solo candidate: the empty menu — `None` downstream, never erasing.
        let solo = vec![cand("a.rs", 3, "E0502", "s-only")];
        assert_eq!(
            strategy_menu(&solo, &cand("a.rs", 3, "E0502", "s-only")),
            Vec::<String>::new()
        );
        // The cap: a wide same-site menu truncates to the store's label cap
        // (`latent_retrieval` owns the constant; without it the label has no
        // consumer and the menu travels whole — playbook-bounded).
        #[cfg(feature = "latent_retrieval")]
        {
            let wide: Vec<FixCandidate> = (0..24)
                .map(|i| cand("a.rs", 3, "E0502", &format!("s-{i:02}")))
                .collect();
            let menu = strategy_menu(&wide, &cand("a.rs", 3, "E0502", "s-07"));
            assert_eq!(menu.len(), crate::traj_store::REJECTED_CAP);
            assert!(!menu.contains(&"s-07".to_string()), "chosen stays out");
            assert_eq!(menu[0], "s-00", "sorted");
        }
    }

    /// Issue 91 T1.5 — the ring record round-trips through the jsonl
    /// encoding the fixseq dump writes.
    #[test]
    fn attempt_record_round_trips() {
        let r = AttemptRecord {
            seq: 3,
            code: "E0507".to_string(),
            span_hash: span_hash(Path::new("/x/src/lib.rs"), 7),
            strategy_id: "e0507-move-out".to_string(),
            outcome: AttemptOutcome::RevertedNoProgress,
            errors: (4, 4),
            masking: (1, 1),
            journey_residual_sum: 0,
            terminal_residual: 0,
            span_embed: Some(crate::span_embed::span_embed(
                "pub fn f(v: &String) { let s = v; }",
            )),
            shape_class: Some("use of moved value".to_string()),
            insight: Some("E0507 use of moved value".to_string()),
        };
        let line = serde_json::to_string(&r).unwrap();
        let back: AttemptRecord = serde_json::from_str(&line).unwrap();
        assert_eq!(back, r);
        assert_eq!(r.outcome.label(), "reverted:no-progress");
        // Issue 100 — a PRE-issue-100 ring line (no `masking` field)
        // deserializes as (0, 0): the old rule's keeps all had a strict
        // count decrease at masking 0, so the default read is consistent,
        // never a misclassification.
        let old = r#"{"seq":0,"code":"E0502","span_hash":"x","strategy_id":"s","outcome":"Kept","errors":[2,1]}"#;
        let legacy: AttemptRecord = serde_json::from_str(old).unwrap();
        assert_eq!(legacy.masking, (0, 0));
        assert_eq!(legacy.errors, (2, 1));
        // Issue 105 — a PRE-105 ring line (no journey fields) decodes as 0
        // = absent, and a stamped line round-trips the run-level values.
        let legacy105: AttemptRecord = serde_json::from_str(
            r#"{"seq":0,"code":"E0502","span_hash":"x","strategy_id":"s","outcome":"Kept","errors":[2,1],"masking":[0,0],"journey_residual_sum":5,"terminal_residual":1}"#,
        )
        .unwrap();
        assert_eq!(legacy105.journey_residual_sum, 5);
        assert_eq!(legacy105.terminal_residual, 1);
        assert_eq!(legacy.journey_residual_sum, 0);
        assert_eq!(legacy.terminal_residual, 0);
        // Issue 107 T2 — every legacy line above (pre-100, pre-105) ALSO
        // decodes the embedding column as None; a T2 line round-trips the
        // quantized vector byte-exact.
        assert_eq!(legacy.span_embed, None);
        assert_eq!(legacy105.span_embed, None);
        let t2 = serde_json::to_string(&AttemptRecord {
            span_embed: Some(r.span_embed.unwrap()),
            ..r.clone()
        })
        .unwrap();
        let back2: AttemptRecord = serde_json::from_str(&t2).unwrap();
        assert!(back2.span_embed.is_some());
        assert!(t2.contains("\"span_embed\":"), "column serialized: {t2}");
    }

    /// Issue 105 (PC-ALM credit, Research 554) — the journey accumulator
    /// on the three canonical ring populations the benches recorded:
    /// the F4 clean unmasking ladder, the pre-100 F3 strand-keep, and the
    /// post-100 honest all-revert F3. The poison signature: the strand
    /// ring's poverty EXCEEDS the clean ring's, and only the strand ring
    /// is tainted.
    #[test]
    fn journey_credit_ranks_strand_above_clean() {
        // F4 (Bench 090, the designed clean shape):
        //   E0614 KEEP (1→1, m1→0) — the unmasking trade, then
        //   E0499 KEEP (1→0, m0→0) — the swap finishes. Run ends clean.
        let f4 = vec![
            AttemptRecord {
                seq: 0,
                code: "E0614".to_string(),
                span_hash: "f4a".to_string(),
                strategy_id: "e0614-deref-at-use".to_string(),
                outcome: AttemptOutcome::Kept,
                errors: (1, 1),
                masking: (1, 0),
                journey_residual_sum: 0,
                terminal_residual: 0,
                span_embed: None,
                shape_class: None,
                insight: None,
            },
            AttemptRecord {
                seq: 1,
                code: "E0499".to_string(),
                span_hash: "f4b".to_string(),
                strategy_id: "e0499-swap-second-decl".to_string(),
                outcome: AttemptOutcome::Kept,
                errors: (1, 0),
                masking: (0, 0),
                journey_residual_sum: 0,
                terminal_residual: 0,
                span_embed: None,
                shape_class: None,
                insight: None,
            },
        ];
        // The PRE-100 F3 strand (Bench 089's recorded poison): the
        // count-only rule KEPT the clone-out (2→1) while masking went
        // 0→1 — the run then ends at the masked E0614 with nothing left
        // to propose. Every keep in this run rode a poisoned journey.
        let strand = vec![AttemptRecord {
            seq: 0,
            code: "E0502".to_string(),
            span_hash: "f3".to_string(),
            strategy_id: "e0502-clone-out".to_string(),
            outcome: AttemptOutcome::Kept,
            errors: (2, 1),
            masking: (0, 1),
            journey_residual_sum: 0,
            terminal_residual: 0,
            span_embed: None,
            shape_class: None,
            insight: None,
        }];
        // Post-100 F3 (Bench 090's honest shape): everything reverts, the
        // file is byte-identical — no keeps, but the journey still records.
        let f3_honest = vec![AttemptRecord {
            seq: 0,
            code: "E0502".to_string(),
            span_hash: "f3".to_string(),
            strategy_id: "e0502-clone-out".to_string(),
            outcome: AttemptOutcome::RevertedNoProgress,
            errors: (2, 2),
            masking: (0, 1),
            journey_residual_sum: 0,
            terminal_residual: 0,
            span_embed: None,
            shape_class: None,
            insight: None,
        }];

        let c_f4 = JourneyCredit::from_attempts(&f4);
        let c_strand = JourneyCredit::from_attempts(&strand);
        let c_rev = JourneyCredit::from_attempts(&f3_honest);
        let c_empty = JourneyCredit::default();

        // The clean ladder: integral 1 (states 1 then 0), terminal 0,
        // UNTAINTED — honest partial progress is never poisoned.
        assert_eq!(c_f4.journey_residual_sum, 1);
        assert_eq!(c_f4.terminal_residual, 0);
        assert!(!c_f4.tainted());
        // The strand: integral 2, terminal 2, TAINTED — the exact poison.
        assert_eq!(c_strand.journey_residual_sum, 2);
        assert_eq!(c_strand.terminal_residual, 2);
        assert!(c_strand.tainted());
        // The honest all-revert run records the journey but — crucially —
        // taint is about the KEEPS' environment; a reverted run still ends
        // masked, so it reads tainted (its POVERTY is high) — but it
        // recorded NO successes, so there is nothing to poison downstream.
        assert!(c_rev.tainted());

        // Poverty ordering: the masked-end runs (strand, honest-revert)
        // sit far above the clean ladder, and the no-data point is exactly
        // neutral 0.5. (The honest-revert run can read POORER than the
        // strand — it left more error behind — but it recorded no
        // successes, so its poverty never discounts anything downstream.)
        let p_f4 = c_f4.poverty(JOURNEY_ALPHA, JOURNEY_RHO);
        let p_strand = c_strand.poverty(JOURNEY_ALPHA, JOURNEY_RHO);
        let p_rev = c_rev.poverty(JOURNEY_ALPHA, JOURNEY_RHO);
        let p_empty = c_empty.poverty(JOURNEY_ALPHA, JOURNEY_RHO);
        assert!(
            p_strand > p_f4,
            "strand {p_strand} must exceed clean {p_f4}"
        );
        assert!(p_rev > p_f4);
        assert!((p_empty - 0.5).abs() < 1e-6, "no-data neutral is 0.5");
        assert!(
            p_f4 > p_empty,
            "a busy-but-clean journey is honest evidence, above neutral"
        );

        // The ×256 fixed-point composite is stable and only consumed for
        // tainted runs. (0.25·2 + 2·2)·256 = 1152 exactly.
        assert_eq!(c_strand.taint_composite_256(), 1152);
        assert_eq!(c_empty.taint_composite_256(), 0);
    }

    #[test]
    fn apply_edits_replaces_exact_span() {
        let text = "let r = v;";
        let at = text.find("r = v").unwrap();
        let out = apply_edits(text, (at, at + 5), "mut r = v", &[]).unwrap();
        assert_eq!(out, "let mut r = v;");
    }

    #[test]
    fn apply_edits_rejects_bad_spans() {
        assert_eq!(
            apply_edits("héllo", (1, 2), "x", &[]),
            None,
            "non-char boundary"
        );
        assert_eq!(apply_edits("abc", (2, 1), "x", &[]), None, "end < start");
        assert_eq!(apply_edits("abc", (0, 9), "x", &[]), None, "out of bounds");
        assert_eq!(apply_edits("abc", (0, 3), "x", &[]).as_deref(), Some("x"));
    }

    #[test]
    fn apply_edits_applies_multi_edits_atomically() {
        // The E0502 deref-removal shape: delete the `&` and the `*` in one
        // candidate — half-landing must be impossible.
        let text = "let first = &v[0];\n    v.push(1);\n    *first\n";
        let amp = text.find('&').unwrap();
        let star = text.find("*first").unwrap();
        let out = apply_edits(
            text,
            (amp, amp + 1),
            "",
            &[SpanEdit {
                byte_span: (star, star + 1),
                replacement: String::new(),
            }],
        )
        .unwrap();
        assert_eq!(out, "let first = v[0];\n    v.push(1);\n    first\n");
    }

    #[test]
    fn apply_edits_rejects_overlapping_edits() {
        assert_eq!(
            apply_edits(
                "abcdef",
                (1, 4),
                "XY",
                &[SpanEdit {
                    byte_span: (3, 5),
                    replacement: "Z".to_string(),
                }],
            ),
            None,
            "an edit starting inside the primary span must fail the whole candidate"
        );
    }

    #[test]
    fn ordering_is_file_line_then_shortest_total_replacement() {
        let mk = |file: &str, line: usize, repl: &str, strat: &str| FixCandidate {
            strategy_id: strat.to_string(),
            code: "E0597".to_string(),
            file: PathBuf::from(file),
            line,
            byte_span: (0, 0),
            replacement: repl.to_string(),
            shape: String::new(),
            semantic_note: None,
            diag_digest: None,
            extra_edits: Vec::new(),
        };
        let mut cands = vec![
            mk("b.rs", 1, "short", "z"),
            mk("a.rs", 9, "longer replacement", "y"),
            mk("a.rs", 2, "xxxxxxxx", "x"),
            mk("a.rs", 2, "yy", "w"),
        ];
        order_candidates(&mut cands);
        assert_eq!(cands[0].file, PathBuf::from("a.rs"));
        assert_eq!(cands[0].line, 2);
        assert_eq!(cands[0].replacement, "yy", "shortest delta first");
        assert_eq!(cands[1].replacement, "xxxxxxxx");
        assert_eq!(cands[2].line, 9);
        assert_eq!(cands[3].file, PathBuf::from("b.rs"));
    }

    #[test]
    fn ordering_counts_extra_edits_in_total_len() {
        let base = |strat: &str, extras: Vec<SpanEdit>| FixCandidate {
            strategy_id: strat.to_string(),
            code: "E0597".to_string(),
            file: PathBuf::from("a.rs"),
            line: 1,
            byte_span: (0, 0),
            replacement: String::new(),
            shape: String::new(),
            semantic_note: None,
            diag_digest: None,
            extra_edits: extras,
        };
        // A two-byte deletion pair must sort BEFORE a `.clone()` insertion
        // even though the primary replacement is empty in both.
        let pair = base(
            "deref-removal",
            vec![
                SpanEdit {
                    byte_span: (1, 2),
                    replacement: String::new(),
                },
                SpanEdit {
                    byte_span: (5, 6),
                    replacement: String::new(),
                },
            ],
        );
        assert_eq!(pair.total_len(), 0, "deletions add no bytes");
        let mut cands = vec![
            base(
                "clone-insert",
                vec![SpanEdit {
                    byte_span: (9, 9),
                    replacement: ".clone()".to_string(),
                }],
            ),
            pair,
        ];
        order_candidates(&mut cands);
        assert_eq!(cands[0].strategy_id, "deref-removal");
    }

    #[test]
    fn tried_key_distinguishes_strategies_at_same_span() {
        let a = FixCandidate {
            strategy_id: "s1".to_string(),
            code: "E0597".to_string(),
            file: PathBuf::from("a.rs"),
            line: 1,
            byte_span: (3, 8),
            replacement: "x".to_string(),
            shape: String::new(),
            semantic_note: None,
            diag_digest: None,
            extra_edits: Vec::new(),
        };
        let b = FixCandidate {
            strategy_id: "s2".to_string(),
            ..a.clone()
        };
        assert_ne!(tried_key(&a), tried_key(&b));
        assert_eq!(tried_key(&a), tried_key(&a.clone()));
    }
}
