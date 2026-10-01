//! Issue 140 T3 — insight-conditioned retry at the [`CompileDriver`]'s
//! ordering + observer seams (the RLTL;DR modelless lane, `.research/228`
//! / arXiv:2609.37633).
//!
//! The paper's loop, one line: sequential retries conditioned on
//! accumulated one-sentence insights distilled from verifier outputs beat
//! i.i.d. retries 57% vs 6% on Pass@128=0 tasks — with a FROZEN model.
//! Our mapping: the driver's iteration IS the retry loop; the attempt
//! ring IS the accumulation surface; [`crate::insight::distill`] IS the
//! one-sentence distiller (T1) and [`crate::insight::admit`] IS the
//! abstraction-level judgment (T2). What this module adds is the
//! CONDITIONING: which strategies a retry considers, reordered by the
//! family's admitted insight history, only while the family is
//! struggling.
//!
//! ## What "conditioning" means modellessly — and the control arm
//!
//! The paper's insight is READ by a generator. The modelless driver has
//! no reader; what it has is candidate selection. The insight text
//! therefore earns its keep as **the admission predicate's subject** —
//! the quality filter deciding which experiences enter the conditioning
//! statistics — while the statistics themselves are per-strategy
//! Beta-LCB scores over the family's recent rows (the same
//! [`katgpt_core::best_belief_score`] primitive the flat and
//! shape-conditional orderings use). The honest consequence, stated in
//! the issue and measured by the eval: a deterministic template over
//! (error-code, shape-class) features consumed as strategy statistics is
//! ORDERING under a new name unless the admission filter changes
//! outcomes. [`Mode::Control`] is exactly that arm — same features, same
//! scoring, no admission judgment — and the paired comparison
//! (insight vs control, LB95 > 0) is what separates conditioning from
//! ordering. The arm where a reader exists (a frozen generator reading
//! the sentences) is riir-train Plan 432's lane, not this one.
//!
//! ## The goldilocks gate (the paper's consult rule)
//!
//! "Consult memory only when running success ≤ 50%": a family whose
//! running success exceeds [`ARM_THRESHOLD`] observes BYTE-IDENTICAL
//! behavior to the static arm — the policy's ordering falls back to
//! [`order_candidates`] (the unarmed path must not smuggle a second
//! difference into the control comparison). Once armed, a family stays
//! armed until running success reaches [`RELEASE_THRESHOLD`] (hysteresis
//! — a single keep must not disarm a still-struggling family at the
//! boundary).
//!
//! ## The contextual-drag guard
//!
//! The store's recording surface accepts ONLY attempt-ring records
//! carrying a [`DiagDigest`](super::playbooks::DiagDigest) — distilled,
//! message-derived rows. There is no API path that stuffs raw ring text
//! (rendered diagnostics, attempt summaries) into the conditioning
//! context: the paper measured raw-summary conditioning FAILING, and the
//! structure makes the violation unexpressible rather than forbidden.
//! [`Mode::Raw`] — the eval's deliberate negative control — recovers the
//! episode-bound shape of that failure lawfully, by keying families on
//! `span_hash` (site identity): rows never transfer across sites, so the
//! arm must not beat insight conditioning.
//!
//! ## Determinism
//!
//! Every input is deterministic for a given (candidates, attempt)
//! history: family keys are exact strings, scores are `f32` compared by
//! `total_cmp`, exact ties fall through to (total replacement length,
//! strategy id), and the unarmed path is the static order. Two runs over
//! the same repo state order identically.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use katgpt_core::best_belief_score;

use super::driver::{order_candidates, AttemptOutcome, AttemptRecord, ObserverFn, OrderingFn};
use super::playbooks::FixCandidate;
use crate::insight::admit;

/// The consult line — arm when the family's running success is at or
/// below this (the paper's goldilocks rule: memory helps the struggling,
/// hurts the succeeding).
pub const ARM_THRESHOLD: f32 = 0.50;
/// The release line — hysteresis: once armed, a family stays armed until
/// running success reaches this (a 15pt band; single keeps must not flap
/// the gate at the boundary).
pub const RELEASE_THRESHOLD: f32 = 0.65;
/// The insertion bound — at most this many of the family's MOST RECENT
/// rows form the retry context (the paper's small-context law; older
/// history falls out of the statistics, never accumulates forever).
pub const MAX_INSIGHTS: usize = 5;
/// The unseen-strategy prior — `best_belief_score(0, 0, 0.32)`, the same
/// Beta(1,1)@ε floor the flat and shape-conditional arms use (one scoring
/// vocabulary across every ordering policy).
const NEUTRAL: f32 = 0.32;
/// ε for [`best_belief_score`] (the fleet constant — 0.32 everywhere a
/// Beta-LCB strategy score appears).
const BELIEF_EPSILON: f32 = 0.32;

/// The arm selector — which experiences enter the conditioning context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Insight conditioning: only attempts whose distilled insight
    /// ADMITS ([`admit`]) record rows — the abstraction filter is the
    /// claimed mechanism (generic lessons transfer; episode-bound traces
    /// are label noise).
    Insight,
    /// The REQUIRED control arm: the deterministic template over the
    /// same (error-code, shape-class) features WITHOUT the admission
    /// judgment — every distilled row records. If insight does not beat
    /// this arm paired, the deterministic lane collapses into ordering
    /// and the claim stays unearned (the issue's honesty note).
    Control,
    /// The negative control: families keyed by `(code, span_hash)` —
    /// site identity, maximally episode-bound, structurally unable to
    /// transfer across sites. Must NOT beat insight conditioning (the
    /// paper's raw-summary/contextual-drag failure, reproduced lawfully
    /// at the key level).
    Raw,
}

/// One accumulated conditioning row (the insight-store unit).
struct Row {
    strategy_id: String,
    kept: bool,
}

/// Per-family state: the recent-row context + the running totals the
/// goldilocks gate reads.
#[derive(Default)]
struct Family {
    rows: Vec<Row>,
    kept: u32,
    reverted: u32,
    armed: bool,
}

/// Counters the eval prints beside every arm (a tie must be interpretable:
/// `refused == 0` means the corpus never exercised the admission filter,
/// not that the filter has no effect).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PolicyStats {
    /// Rows that entered the conditioning context.
    pub recorded: u32,
    /// Rows the admission predicate refused (Insight mode only).
    pub refused_admission: u32,
    /// Distinct families seen.
    pub families: u32,
    /// Families currently armed.
    pub armed: u32,
    /// Gate arm transitions (unarmed → armed).
    pub gate_arms: u32,
    /// Gate release transitions (armed → unarmed).
    pub gate_releases: u32,
}

#[derive(Default)]
struct Store {
    families: HashMap<(String, String), Family>,
    stats: PolicyStats,
}

/// The insight-conditioned retry policy: install via
/// [`Self::ordering`] + [`Self::observer`] on the driver (the
/// Issue-093 composition shape — the two closures share one store).
///
/// ```no_run
/// use riir_clippy::domains::rustc_errors::insight_retry::{
///     InsightConditioning, Mode,
/// };
/// use riir_clippy::domains::rustc_errors::floor::DriverFixer;
///
/// let pol = InsightConditioning::new(Mode::Insight);
/// let fixer = DriverFixer::new()
///     .with_ordering(pol.ordering())
///     .with_observer(pol.observer());
/// ```
pub struct InsightConditioning {
    inner: Arc<Mutex<Store>>,
    mode: Mode,
}

impl InsightConditioning {
    /// One policy for one arm. The mode is fixed at birth (an arm never
    /// mutates mid-run — the eval's arms differ by mode alone).
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Store::default())),
            mode,
        }
    }

    /// The ordering half: snapshot the armed families' bounded contexts,
    /// then rank candidates by per-strategy Beta-LCB over the family's
    /// ≤[`MAX_INSIGHTS`] most recent rows. No armed family → the STATIC
    /// order, byte-identical (the goldilocks rule's unarmed guarantee).
    #[must_use]
    pub fn ordering(&self) -> OrderingFn {
        let inner = Arc::clone(&self.inner);
        Arc::new(move |cands: &mut [FixCandidate]| {
            // Snapshot under the lock, sort without it (the observer may
            // fire from the same thread between ordering calls; it never
            // fires DURING one). The flat `code\0shape` key keeps the
            // lookup allocation to one String per CANDIDATE (scores are
            // computed exactly once each — the decorate pass).
            let snapshot = {
                let s = inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let mut out: HashMap<String, HashMap<String, (u32, u32)>> = HashMap::new();
                for (k, f) in s.families.iter().filter(|(_, f)| f.armed) {
                    let mut per: HashMap<String, (u32, u32)> = HashMap::new();
                    for row in f.rows.iter().rev().take(MAX_INSIGHTS) {
                        let e = per.entry(row.strategy_id.clone()).or_default();
                        if row.kept {
                            e.0 += 1;
                        } else {
                            e.1 += 1;
                        }
                    }
                    out.insert(format!("{}\u{0}{}", k.0, k.1), per);
                }
                out
            };
            if snapshot.is_empty() {
                order_candidates(cands); // unarmed = the static arm, exactly
                return;
            }
            let score = |c: &FixCandidate| {
                let Some(d) = c.diag_digest.as_ref() else {
                    return NEUTRAL;
                };
                match snapshot.get(&format!("{}\u{0}{}", c.code, d.shape_class)) {
                    None => NEUTRAL,
                    Some(per) => match per.get(&c.strategy_id) {
                        None => NEUTRAL,
                        Some(&(s, f)) => best_belief_score(s, f, BELIEF_EPSILON),
                    },
                }
            };
            // Decorate-sort-undecorate: scores computed once per candidate
            // (the family key's shape half is a String — per-comparison
            // clones would allocate O(n log n) times for an O(n) need).
            let scores: Vec<f32> = cands.iter().map(score).collect();
            let mut idx: Vec<usize> = (0..cands.len()).collect();
            idx.sort_by(|&a, &b| {
                scores[b]
                    .total_cmp(&scores[a])
                    .then(cands[a].total_len().cmp(&cands[b].total_len()))
                    .then(cands[a].strategy_id.cmp(&cands[b].strategy_id))
            });
            let ordered: Vec<FixCandidate> = idx.into_iter().map(|i| cands[i].clone()).collect();
            for (dst, src) in cands.iter_mut().zip(ordered) {
                *dst = src;
            }
        })
    }

    /// The observer half: distill nothing (the digest rode the record),
    /// judge admission by mode, append the row, update the gate.
    #[must_use]
    pub fn observer(&self) -> ObserverFn {
        let inner = Arc::clone(&self.inner);
        let mode = self.mode;
        Arc::new(move |rec: &AttemptRecord| {
            // The contextual-drag guard, structurally: no digest, no row.
            // Pre-140 rings and digest-less attempts simply do not
            // condition.
            let (Some(shape_class), Some(insight)) = (&rec.shape_class, &rec.insight) else {
                return;
            };
            let key_half = match mode {
                Mode::Raw => rec.span_hash.clone(),
                Mode::Insight | Mode::Control => shape_class.clone(),
            };
            let mut s = inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if matches!(mode, Mode::Insight) && !admit(insight) {
                s.stats.refused_admission += 1;
                return;
            }
            let kept = matches!(rec.outcome, AttemptOutcome::Kept);
            let key = (rec.code.clone(), key_half);
            let is_new_family = !s.families.contains_key(&key);
            let fam = s.families.entry(key).or_default();
            fam.rows.push(Row {
                strategy_id: rec.strategy_id.clone(),
                kept,
            });
            if kept {
                fam.kept += 1;
            } else {
                fam.reverted += 1;
            }
            // The gate reads the family's POST-ROW totals; compute the
            // transition while `fam` is still borrowed, apply the stats
            // counters after it drops. (Counts are attempt totals ~10⁰–10¹ —
            // exact in f32.)
            let total = fam.kept + fam.reverted;
            #[allow(clippy::cast_precision_loss)]
            let success = fam.kept as f32 / total as f32;
            let arm_now = !fam.armed && success <= ARM_THRESHOLD;
            let release_now = fam.armed && success >= RELEASE_THRESHOLD;
            if arm_now {
                fam.armed = true;
            } else if release_now {
                fam.armed = false;
            }
            // (`fam`'s borrow ends at its last use — the stats counters
            // below mutate `s` after NLL releases it.)
            if is_new_family {
                s.stats.families += 1;
            }
            s.stats.recorded += 1;
            if arm_now {
                s.stats.gate_arms += 1;
            } else if release_now {
                s.stats.gate_releases += 1;
            }
        })
    }

    /// A consistent snapshot of the printable counters (the eval's
    /// interpretability requirement — a tie with `refused == 0` means the
    /// corpus never exercised the filter).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // family counts are corpus-scale (≤ 10³)
    pub fn stats(&self) -> PolicyStats {
        let s = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut st = s.stats;
        st.armed = s.families.values().filter(|f| f.armed).count() as u32;
        st
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::rustc_errors::driver::span_hash;
    use crate::domains::rustc_errors::playbooks::DiagDigest;
    use std::path::{Path, PathBuf};

    fn cand(file: &str, line: usize, code: &str, strat: &str, digest: DiagDigest) -> FixCandidate {
        FixCandidate {
            strategy_id: strat.to_string(),
            code: code.to_string(),
            file: PathBuf::from(file),
            line,
            byte_span: (0, 0),
            replacement: "x".to_string(),
            shape: String::new(),
            semantic_note: None,
            diag_digest: Some(digest),
            extra_edits: Vec::new(),
        }
    }

    fn digest(shape_class: &str, insight: &str) -> DiagDigest {
        DiagDigest {
            shape_class: shape_class.to_string(),
            insight: insight.to_string(),
        }
    }

    fn attempt(
        code: &str,
        shape_class: Option<&str>,
        insight: Option<&str>,
        span: &str,
        strat: &str,
        kept: bool,
    ) -> AttemptRecord {
        AttemptRecord {
            seq: 0,
            code: code.to_string(),
            span_hash: span.to_string(),
            strategy_id: strat.to_string(),
            outcome: if kept { AttemptOutcome::Kept } else { AttemptOutcome::RevertedNoProgress },
            errors: (2, if kept { 1 } else { 2 }),
            masking: (0, 0),
            journey_residual_sum: 0,
            terminal_residual: 0,
            span_embed: None,
            shape_class: shape_class.map(str::to_string),
            insight: insight.map(str::to_string),
        }
    }

    /// One failure arms the family (running success 0 ≤ 0.50 — the
    /// paper's consult line); a kept majority releases it, and the band
    /// in between holds (hysteresis).
    #[test]
    fn gate_arms_on_failure_and_holds_through_the_band() {
        let pol = InsightConditioning::new(Mode::Insight);
        let obs = pol.observer();
        // 3 keeps, then a failure: success 3/4 = 0.75 — never armed...
        for _ in 0..3 {
            obs(&attempt(
                "E0502",
                Some("cannot borrow _ as mutable"),
                Some("E0502 cannot borrow `v` as mutable"),
                "aa",
                "e0502-clone-out",
                true,
            ));
        }
        obs(&attempt(
            "E0502",
            Some("cannot borrow _ as mutable"),
            Some("E0502 cannot borrow `v` as mutable"),
            "aa",
            "e0502-clone-out",
            false,
        ));
        assert_eq!(pol.stats().gate_arms, 0, "0.75 never arms");
        // ...a second failure: 3/5 = 0.6, still above 0.50 — not armed
        // (the gate reads the RAW rate, the paper's own line)...
        obs(&attempt(
            "E0502",
            Some("cannot borrow _ as mutable"),
            Some("E0502 cannot borrow `v` as mutable"),
            "aa",
            "e0502-clone-out",
            false,
        ));
        assert_eq!(pol.stats().gate_arms, 0, "0.6 is above the consult line");
        // ...a third: 3/6 = 0.50 — AT the line arms (≤)...
        obs(&attempt(
            "E0502",
            Some("cannot borrow _ as mutable"),
            Some("E0502 cannot borrow `v` as mutable"),
            "aa",
            "e0502-clone-out",
            false,
        ));
        assert_eq!(pol.stats().gate_arms, 1);
        assert_eq!(pol.stats().armed, 1);
        // ...a keep inside the band: 4/7 ≈ 0.57 — armed STAYS armed...
        obs(&attempt(
            "E0502",
            Some("cannot borrow _ as mutable"),
            Some("E0502 cannot borrow `v` as mutable"),
            "aa",
            "e0502-clone-out",
            true,
        ));
        assert_eq!(pol.stats().armed, 1, "hysteresis holds through the band");
        // ...climb to ≥0.65: 5/8 = 0.625 no, 6/9 ≈ 0.67 yes — released.
        obs(&attempt(
            "E0502",
            Some("cannot borrow _ as mutable"),
            Some("E0502 cannot borrow `v` as mutable"),
            "aa",
            "e0502-clone-out",
            true,
        ));
        assert_eq!(pol.stats().armed, 1);
        obs(&attempt(
            "E0502",
            Some("cannot borrow _ as mutable"),
            Some("E0502 cannot borrow `v` as mutable"),
            "aa",
            "e0502-clone-out",
            true,
        ));
        assert_eq!(pol.stats().gate_releases, 1);
        assert_eq!(pol.stats().armed, 0);
    }

    /// THE separation: a literal-laden insight (a path/line-laden trace)
    /// records in Control mode and is REFUSED in Insight mode — the
    /// admission filter is the only difference between the arms.
    #[test]
    fn insight_mode_refuses_what_control_records() {
        let trace = "E0502 fix at /Users/x/src/lib.rs:483 hash a1b2c3d4e5f6 line 1204";
        assert!(!admit(trace));
        let mut arms = [Mode::Insight, Mode::Control].map(InsightConditioning::new);
        for pol in &mut arms {
            pol.observer()(&attempt(
                "E0502",
                Some("cannot borrow _ as mutable"),
                Some(trace),
                "aa",
                "e0502-clone-out",
                false,
            ));
        }
        assert_eq!(arms[0].stats().recorded, 0);
        assert_eq!(arms[0].stats().refused_admission, 1);
        assert_eq!(arms[1].stats().recorded, 1);
        assert_eq!(arms[1].stats().refused_admission, 0);
    }

    /// The unarmed path is the STATIC order, byte-identical — a family
    /// above the consult line must observe no behavioral difference from
    /// the static arm (the control comparison's precondition).
    #[test]
    fn unarmed_ordering_is_the_static_order() {
        let pol = InsightConditioning::new(Mode::Insight);
        let mut cands = vec![
            cand("b.rs", 1, "E0502", "z", digest("k", "i")),
            cand("a.rs", 9, "E0502", "y", digest("k", "i")),
            cand("a.rs", 2, "E0502", "x", digest("k", "i")),
        ];
        let mut expect = cands.clone();
        order_candidates(&mut expect);
        pol.ordering()(&mut cands);
        assert_eq!(cands, expect);
    }

    /// An armed family reorders: the historically-kept strategy outranks
    /// the reverted one, and the unarmed family's candidates hold the
    /// prior (falling to the min-delta tiebreak among themselves).
    #[test]
    fn armed_family_promotes_the_kept_strategy() {
        let pol = InsightConditioning::new(Mode::Insight);
        let obs = pol.observer();
        let sc = "cannot borrow _ as mutable";
        let ins = "E0502 cannot borrow `v` as mutable — consider cloning the value";
        // deref-removal failed twice at this shape → family armed.
        for span in ["aa", "bb"] {
            obs(&attempt("E0502", Some(sc), Some(ins), span, "e0502-deref-removal", false));
        }
        // read-relocate kept once (a DIFFERENT site — cross-site transfer
        // is the whole point of the family key).
        obs(&attempt("E0502", Some(sc), Some(ins), "cc", "e0502-read-relocate", true));
        assert_eq!(pol.stats().armed, 1);

        let mut cands = vec![
            // deref-removal is SHORTER (min-delta would pick it first)...
            cand("a.rs", 1, "E0502", "e0502-deref-removal", digest(sc, ins)),
            // ...but its row history is (0,2) while read-relocate is (1,0).
            cand("a.rs", 1, "E0502", "e0502-read-relocate", digest(sc, ins)),
        ];
        pol.ordering()(&mut cands);
        assert_eq!(cands[0].strategy_id, "e0502-read-relocate");
        assert_eq!(cands[1].strategy_id, "e0502-deref-removal");
    }

    /// The insertion bound: only the [`MAX_INSIGHTS`] most recent rows
    /// count. The oldest row here is the family's ONLY keep for
    /// `deref-removal` — bounded out, its score drops below the unseen
    /// prior (a kept-but-forgotten strategy ranks under an untried one).
    #[test]
    fn only_the_tail_max_insights_rows_count() {
        let pol = InsightConditioning::new(Mode::Insight);
        let obs = pol.observer();
        let sc = "cannot borrow _ as mutable";
        let ins = "E0502 cannot borrow `v` as mutable";
        // Oldest: a KEEP for deref-removal (beyond the eventual tail)...
        obs(&attempt("E0502", Some(sc), Some(ins), "s0", "e0502-deref-removal", true));
        // ...then 5 failures (the tail), arming the family.
        for i in 1..=5 {
            obs(&attempt(
                "E0502",
                Some(sc),
                Some(ins),
                &format!("s{i}"),
                "e0502-deref-removal",
                false,
            ));
        }
        assert_eq!(pol.stats().armed, 1);
        let mut cands = vec![
            // deref-removal: unbounded (1,5); bounded tail (0,5) — the
            // keep fell out of the context window.
            cand("a.rs", 1, "E0502", "e0502-deref-removal", digest(sc, ins)),
            // an unseen strategy: NEUTRAL 0.32.
            cand("a.rs", 1, "E0502", "e0502-clone-out", digest(sc, ins)),
        ];
        pol.ordering()(&mut cands);
        assert_eq!(
            cands[0].strategy_id, "e0502-clone-out",
            "best_belief_score(0,5) < 0.32 < best_belief_score(1,5): the bounded-out keep must not rank"
        );
    }

    /// The negative control's mechanism: Raw mode keys families by SITE,
    /// so two attempts at the same shape on different sites form TWO
    /// families — episode-bound rows structurally never transfer.
    #[test]
    fn raw_mode_keys_families_by_site_never_transferring() {
        let pol = InsightConditioning::new(Mode::Raw);
        let obs = pol.observer();
        let sc = "cannot borrow _ as mutable";
        let ins = "E0502 cannot borrow `v` as mutable";
        obs(&attempt("E0502", Some(sc), Some(ins), "site-a", "e0502-deref-removal", false));
        obs(&attempt("E0502", Some(sc), Some(ins), "site-b", "e0502-read-relocate", true));
        assert_eq!(pol.stats().families, 2, "site keys never merge");
        assert_eq!(pol.stats().recorded, 2);
        // And an ordering over a THIRD site consults nothing: both armed
        // families' keys (span hashes) miss the candidate's family key
        // (its shape class) — the static fallback.
        let mut cands = vec![cand("a.rs", 1, "E0502", "e0502-clone-out", digest(sc, ins))];
        let mut expect = cands.clone();
        order_candidates(&mut expect);
        pol.ordering()(&mut cands);
        assert_eq!(cands, expect);
    }

    /// Determinism pin: identical history + candidates → identical order.
    #[test]
    fn conditioning_is_deterministic() {
        let build = || {
            let pol = InsightConditioning::new(Mode::Insight);
            let obs = pol.observer();
            let sc = "cannot borrow _ as mutable";
            let ins = "E0502 cannot borrow `v` as mutable";
            obs(&attempt("E0502", Some(sc), Some(ins), "aa", "e0502-deref-removal", false));
            obs(&attempt("E0502", Some(sc), Some(ins), "bb", "e0502-read-relocate", true));
            pol
        };
        let mut a = vec![
            cand("a.rs", 1, "E0502", "e0502-deref-removal", digest("cannot borrow _ as mutable", "i")),
            cand("a.rs", 1, "E0502", "e0502-read-relocate", digest("cannot borrow _ as mutable", "i")),
        ];
        let mut b = a.clone();
        build().ordering()(&mut a);
        build().ordering()(&mut b);
        assert_eq!(a, b);
    }

    /// `span_hash` remains the site identity the Raw mode keys on —
    /// stable + discriminating (the driver's own pin, re-asserted here
    /// because the Raw arm's semantics hang on it).
    #[test]
    fn span_hash_keys_raw_families_stably() {
        let a = span_hash(Path::new("/x/src/lib.rs"), 7);
        assert_eq!(a, span_hash(Path::new("/x/src/lib.rs"), 7));
        assert_ne!(a, span_hash(Path::new("/x/src/lib.rs"), 8));
    }
}
