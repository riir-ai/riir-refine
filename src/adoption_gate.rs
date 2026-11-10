//! katgpt-rs Plan 633 T1 / riir-refine Issue 220 — the adoption/retention
//! regression gate's CONSUMER seam (the AGCL form, Research 657 §A5).
//!
//! **Consume, never fork:** the scoring primitive is
//! `katgpt_core::adoption_gate` (the producer, landed `cf65c58a7` —
//! `OutcomeDigest`, `DedupRing`, `Decontaminator`, `score_adoption`,
//! `AdoptionGate::decide`). This module owns only the three consumer
//! surfaces the issue names:
//!
//! 1. **The outcome window** ([`AdoptionWindow`]) — the (rule, fix
//!    verdict) outcome stream as an append-only JSONL sidecar
//!    ([`OUTCOMES_FILE`]), replayed in file order into the producer's
//!    [`AdoptionGate`] (ingest dedups, so replay is idempotent). One
//!    outcome = one applied edit's final verdict: `accepted` = the fix
//!    survived the compile gate (and the oracle did not report a persist),
//!    `rejected` = it was reverted. The digest is
//!    [`outcome_digest`]`("rule", fix_text)` — the same rule producing the
//!    same output again is the SAME outcome, which is exactly the AGCL
//!    `repeated` signal.
//! 2. **The active-set gate** ([`AdoptionWindow::demote_verdict`] /
//!    [`AdoptionWindow::readmit_verdict`]) — on `--active-set-demote` the
//!    challenger is the window MINUS the rule's rows (what the demoted
//!    active set would still produce); on `--active-set-readmit` the
//!    challenger is the rule's OWN historical rows and the binding axis is
//!    `repeated` (a readmit that re-produces known-rejected outputs is the
//!    regression class the gate exists for).
//! 3. **The seat head-swap watch** ([`AdoptionWindow::seat_swap_verdict`])
//!    — seat-lane rows carry the seat receipt's build fingerprint; a build
//!    change is the head swap, and the challenger (the rows under the new
//!    build) is scored against the incumbent window (the rows under the
//!    prior build).
//!
//! # Decision policy (the consumer's operator-command divergence)
//!
//! The producer's decline-first law (`NoEvidence` → keep the incumbent)
//! binds AUTOMATED adoption. The consumers here are OPERATOR commands and
//! an armed serve boundary, so the policy is **gate on evidence, abstain
//! otherwise**: an empty window or a rule with no rows ABSTAINS and the
//! command proceeds exactly as before (named in the output); a `Keep`
//! demote verdict or a `repeated > 0` readmit/swap verdict REFUSES loud.
//! `decline` remains the default answer for any future AUTOMATED demotion
//! path (the fade-predictive gate); nothing in this module demotes or
//! readmits by itself.
//!
//! # Determinism
//!
//! The window replays in file order; the producer's scorer is the pinned
//! f64 expression tree (Issue-940 law, producer-side). The G1 gate test
//! replays the producer fixture's ingest sequences through THIS seam and
//! asserts bit-identical verdict numbers.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use katgpt_core::adoption_gate::{AdoptionDecision, AdoptionGate, AdoptionVerdict, OutcomeDigest};

/// The window sidecar, repo-root-relative (the `resolve_heal_root` posture
/// — the same root the active-set marker and the escalation manifest ride).
pub const OUTCOMES_FILE: &str = ".refine/adoption_outcomes.jsonl";

/// The seat lane's window tag — the fix lane records every seat pick's
/// final verdict under this key (the same spelling the serve boundary uses
/// for `EditAtLine::rule_id`, so the generic kept/revert recorder covers
/// seat picks without a second write site).
pub const SEAT_TAG: &str = "escalation_seat_pick";

/// The on-disk row format version. Written on every row; the loader
/// accepts any `v` it can decode row-wise (a foreign `v` fails that row
/// LOUD — corrupt lines are never silently skipped).
pub const OUTCOME_ROW_VERSION: u32 = 1;

/// Rows-per-file compaction ceiling: when an append pushes the file past
/// this many rows it is rewritten keeping the newest
/// [`OUTCOME_ROW_KEEP`] (deterministic — file order, tail kept). The rings
/// only remember 32 per class, so this bounds the FILE, not the verdicts.
pub const OUTCOME_ROW_COMPACT_AT: usize = 4096;

/// Rows kept by the compaction pass.
pub const OUTCOME_ROW_KEEP: usize = 2048;

/// One outcome identity: `blake3(tag || 0x00 || payload)`. The NUL byte
/// keeps `(rule_a, "bc")` and `(rule_ab, "c")` distinct.
#[must_use]
pub fn outcome_digest(tag: &str, payload: &str) -> OutcomeDigest {
    let mut bytes = Vec::with_capacity(tag.len() + payload.len() + 1);
    bytes.extend_from_slice(tag.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(payload.as_bytes());
    OutcomeDigest::of(&bytes)
}

/// One JSONL row. `digest_hex` keeps the file diffable/auditable; the hex
/// round-trips through [`OutcomeDigest::as_bytes`]. `build` is the seat
/// receipt's build fingerprint on seat-lane rows ([`None`] elsewhere).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeRow {
    /// Format version — always [`OUTCOME_ROW_VERSION`] on write.
    pub v: u32,
    /// The lane/rule tag (the bare `rule_id`, or [`SEAT_TAG`]).
    pub tag: String,
    /// `true` = the fix survived the gate (kept); `false` = reverted.
    pub accepted: bool,
    /// [`outcome_digest`]`(tag, payload)` as lowercase hex.
    pub digest_hex: String,
    /// The seat receipt's build fingerprint — seat rows only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

/// Why a demote/readmit/swap consult answered the way it did. `Abstain`
/// carries no verdict numbers (nothing was measured); the two gate
/// verdicts carry the full producer verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum GateVerdict {
    /// Evidence exists and it does not regress — proceed (the verdict line
    /// names the numbers).
    Proceed(AdoptionVerdict),
    /// Evidence exists and it regresses — refuse. The verdict carries the
    /// numbers for the refusal line.
    Refuse(AdoptionVerdict),
    /// No evidence either way — the consumer proceeds, abstention named.
    Abstain,
}

impl GateVerdict {
    /// The refusal predicate — `Refuse` iff the producer said `Keep` (the
    /// challenger scores strictly below the incumbent, OR ties it — since
    /// katgpt-rs issue 994 the pinned tie rule gives ties to the incumbent,
    /// decline-first; a tie is a no-evidence window).
    #[must_use]
    pub fn refused(&self) -> bool {
        matches!(self, Self::Refuse(_))
    }
}

/// The seat head-swap watch's answer over the window's seat rows.
#[derive(Debug, Clone, PartialEq)]
pub enum SeatSwapVerdict {
    /// The current build has no seat rows yet — no evidence; serve.
    NoEvidence,
    /// The current build's historical rows regress nothing — serve.
    Proceed {
        /// Rejected outputs repeated by the current build's rows.
        repeated_hits: usize,
        /// The current build's row count (the evidence size).
        rows: usize,
    },
    /// The current build's rows repeat known-rejected outputs — stand the
    /// seat down for the run (in-memory, session-scoped; the manifest is
    /// never touched — nothing auto-demotes).
    StandDown {
        /// Rejected outputs repeated by the current build's rows.
        repeated_hits: usize,
        /// The current build's row count.
        rows: usize,
    },
}

/// The outcome window: the sidecar's rows (the audit truth) + the
/// producer gate replayed over them (the scoring state) + per-tag digest
/// sets (the challenger slices).
#[derive(Debug, Default)]
pub struct AdoptionWindow {
    path: PathBuf,
    rows: Vec<OutcomeRow>,
    gate: AdoptionGate,
    /// tag → the digests that tag produced (membership, not order).
    by_tag: BTreeMap<String, BTreeSet<[u8; 32]>>,
    warned_append: bool,
}

impl AdoptionWindow {
    /// Load the window at `root` (the repo root, not the sidecar path).
    /// An ABSENT file is the empty window, not an error (the active-set
    /// law: the default posture owns nothing). A present-but-corrupt file
    /// is a LOUD refusal — a silently-empty window would let every gate
    /// abstain forever.
    ///
    /// # Errors
    ///
    /// A read failure other than not-found, a row that is not the expected
    /// JSON shape, or a digest hex that does not round-trip.
    pub fn load(root: &Path) -> Result<Self, String> {
        Self::load_file(root.join(OUTCOMES_FILE))
    }

    /// Load the window from an explicit sidecar path (the G3 cell
    /// instrument's posture — the real windows live in sibling repos,
    /// read-only to that run; [`load`](Self::load) roots at a repo root,
    /// this takes the file itself). Read-only: a loaded window never
    /// writes — only [`AdoptionWindow::record`] does.
    ///
    /// # Errors
    ///
    /// A read failure other than not-found (an absent file is the empty
    /// window), a row that is not the expected JSON shape, or a digest hex
    /// that does not round-trip — the corrupt-window LOUD refusal, never a
    /// silently-empty gate.
    pub fn load_file(path: PathBuf) -> Result<Self, String> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    path,
                    ..Self::default()
                });
            }
            Err(source) => return Err(format!("{}: {source}", path.display())),
        };
        let mut window = Self {
            path,
            ..Self::default()
        };
        for (line_no, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let row: OutcomeRow = serde_json::from_str(line)
                .map_err(|e| format!("{}:{}: {e}", window.path.display(), line_no + 1))?;
            window.push_row(row)?;
        }
        Ok(window)
    }

    /// Decode + file one loaded row (the load path AND the append path's
    /// tail). The digest hex must round-trip; the replay dedups (a repeat
    /// row updates nothing — the producer's ingest law).
    fn push_row(&mut self, row: OutcomeRow) -> Result<(), String> {
        let mut bytes = [0u8; 32];
        for (i, b) in row
            .digest_hex
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
        {
            let hex = std::str::from_utf8(b)
                .map_err(|_| format!("{}: digest hex is not UTF-8", self.path.display()))?;
            let nibble = u8::from_str_radix(hex, 16)
                .map_err(|e| format!("{}: bad digest hex: {e}", self.path.display()))?;
            bytes[i] = nibble;
        }
        let digest = OutcomeDigest::from_bytes(bytes);
        self.gate.ingest(row.accepted, digest);
        self.by_tag
            .entry(row.tag.clone())
            .or_default()
            .insert(bytes);
        self.rows.push(row);
        Ok(())
    }

    /// The window's rows in file order (the perturbation harness's input —
    /// flip verdict bits, rebuild with [`AdoptionWindow::from_rows`],
    /// re-consult).
    #[must_use]
    pub fn rows(&self) -> &[OutcomeRow] {
        &self.rows
    }

    /// The window's distinct tags, sorted (the consult sweep's population).
    #[must_use]
    pub fn tags(&self) -> Vec<&str> {
        self.by_tag.keys().map(String::as_str).collect()
    }

    /// The window's row count (file rows — the audit truth, pre-dedup).
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Build an in-memory window from rows in file order (the judge-noise
    /// perturbation harness — replaying flipped verdicts never touches the
    /// disk). Same decode + ingest-dedup as the load path. The digest hex
    /// must round-trip: rows come from a loaded window or a fixture, and a
    /// bad hex panics LOUD rather than silently skipping (the corrupt-line
    /// law, constructor form).
    ///
    /// # Panics
    ///
    /// On a row whose `digest_hex` does not round-trip to 32 bytes — the
    /// same corruption a [`AdoptionWindow::load_file`] would return as an
    /// `Err`, panicked here because the constructor's callers feed it
    /// already-validated rows (a perturbed copy of a loaded window's own
    /// [`AdoptionWindow::rows`]).
    #[must_use]
    pub fn from_rows(rows: Vec<OutcomeRow>) -> Self {
        let mut window = Self {
            path: PathBuf::new(),
            ..Self::default()
        };
        for row in rows {
            if let Err(e) = window.push_row(row) {
                panic!("adoption window from_rows: {e}");
            }
        }
        window
    }

    /// Record one outcome: digest, dedup against the T rings (a repeat
    /// outcome — the same tag producing the same output with a verdict —
    /// updates nothing and writes no row), append best-effort. A write
    /// failure warns ONCE per window and never fails the caller (the
    /// `record_traj` posture): the in-memory gate still scored the event.
    pub fn record(&mut self, tag: &str, accepted: bool, payload: &str, build: Option<&str>) {
        let digest = outcome_digest(tag, payload);
        if !self.gate.ingest(accepted, digest) {
            return; // already filed — never double-file
        }
        let bytes = digest.as_bytes();
        self.by_tag
            .entry(tag.to_string())
            .or_default()
            .insert(*bytes);
        let row = OutcomeRow {
            v: OUTCOME_ROW_VERSION,
            tag: tag.to_string(),
            accepted,
            digest_hex: hex_of(bytes),
            build: build.map(str::to_string),
        };
        if let Err(e) = self.append_row(&row) {
            if !self.warned_append {
                self.warned_append = true;
                eprintln!("adoption-gate: window append failed (further failures silent): {e}");
            }
        }
        self.rows.push(row);
        self.maybe_compact();
    }

    fn append_row(&self, row: &OutcomeRow) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(
            file,
            "{}",
            serde_json::to_string(row).map_err(std::io::Error::other)?
        )
    }

    /// Compaction: past [`OUTCOME_ROW_COMPACT_AT`] rows, rewrite the file
    /// keeping the newest [`OUTCOME_ROW_KEEP`] (file order, tail). The
    /// in-memory state is untouched (the rings' 32-window makes the drop
    /// verdict-neutral for anything still scoreable).
    fn maybe_compact(&mut self) {
        if self.rows.len() <= OUTCOME_ROW_COMPACT_AT {
            return;
        }
        let keep = &self.rows[self.rows.len() - OUTCOME_ROW_KEEP..];
        let mut body = String::new();
        for row in keep {
            body.push_str(&serde_json::to_string(row).expect("row re-encodes"));
            body.push('\n');
        }
        // Best-effort — the same warn-once posture as append.
        if let Err(e) = std::fs::write(&self.path, body) {
            if !self.warned_append {
                self.warned_append = true;
                eprintln!("adoption-gate: window compaction failed: {e}");
            }
        }
    }

    /// The `--active-set-demote` gate for `rule`: the challenger is the
    /// window MINUS the rule's rows (the outcomes the demoted active set
    /// would still produce), scored by the producer's
    /// [`AdoptionGate::decide`] against the incumbent window.
    ///
    /// Demoting a rule that was carrying accepted fixes drops `retained`;
    /// demoting a rule whose rows were mostly rejected RAISES the
    /// challenger's score — both readings are the gate's point.
    #[must_use]
    pub fn demote_verdict(&self, rule: &str, alpha: f64) -> GateVerdict {
        let Some(rule_digests) = self.by_tag.get(rule) else {
            return GateVerdict::Abstain;
        };
        let candidate_accepted: Vec<OutcomeDigest> = self
            .gate
            .accepted()
            .iter()
            .filter(|d| !rule_digests.contains(d.as_bytes()))
            .collect();
        let candidate_outputs: Vec<OutcomeDigest> = self
            .gate
            .accepted()
            .iter()
            .chain(self.gate.rejected().iter())
            .filter(|d| !rule_digests.contains(d.as_bytes()))
            .collect();
        let verdict = self
            .gate
            .decide(&candidate_accepted, &candidate_outputs, alpha);
        match verdict.decision {
            AdoptionDecision::Adopt => GateVerdict::Proceed(verdict),
            AdoptionDecision::Keep => GateVerdict::Refuse(verdict),
            AdoptionDecision::NoEvidence => GateVerdict::Abstain,
        }
    }

    /// The `--active-set-readmit` gate for `rule`: the challenger is the
    /// rule's OWN historical rows (the outcomes re-admitting it would
    /// re-produce). The composite decision is uninformative here (the rule
    /// produced nothing while demoted, so `retained` reads ~0 against the
    /// demoted-period window) — the binding axis is `repeated`: ANY
    /// historical output that sits in the rejected ring is a known
    /// regression the readmit would repeat → [`GateVerdict::Refuse`].
    /// A rule with no rows abstains; a clean-history rule proceeds.
    #[must_use]
    pub fn readmit_verdict(&self, rule: &str, alpha: f64) -> GateVerdict {
        let Some(rule_digests) = self.by_tag.get(rule) else {
            return GateVerdict::Abstain;
        };
        let candidate_accepted: Vec<OutcomeDigest> = self
            .gate
            .accepted()
            .iter()
            .filter(|d| rule_digests.contains(d.as_bytes()))
            .collect();
        let candidate_outputs: Vec<OutcomeDigest> = self
            .gate
            .accepted()
            .iter()
            .chain(self.gate.rejected().iter())
            .filter(|d| rule_digests.contains(d.as_bytes()))
            .collect();
        if candidate_outputs.is_empty() {
            return GateVerdict::Abstain;
        }
        let verdict = self
            .gate
            .decide(&candidate_accepted, &candidate_outputs, alpha);
        if verdict.repeated > 0.0 {
            GateVerdict::Refuse(verdict)
        } else {
            GateVerdict::Proceed(verdict)
        }
    }

    /// The seat head-swap gate for the build fingerprint `current_build`:
    /// the challenger is the seat rows recorded under that build (what
    /// THIS head produced before), the incumbent window is the rest. The
    /// binding axis is `repeated` (same readmit rationale — a head whose
    /// past picks repeat known-rejected outputs regresses by evidence).
    /// A build with no rows (first sighting) is [`SeatSwapVerdict::NoEvidence`].
    #[must_use]
    pub fn seat_swap_verdict(&self, current_build: &str) -> SeatSwapVerdict {
        let mut build_digests: BTreeSet<[u8; 32]> = BTreeSet::new();
        for row in &self.rows {
            if row.tag == SEAT_TAG && row.build.as_deref() == Some(current_build) {
                // Re-derive from the row's own hex (order-free — rows are
                // the audit truth).
                let mut bytes = [0u8; 32];
                for (i, b) in row
                    .digest_hex
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .enumerate()
                {
                    bytes[i] =
                        u8::from_str_radix(std::str::from_utf8(b).unwrap_or("zz"), 16).unwrap_or(0);
                }
                build_digests.insert(bytes);
            }
        }
        if build_digests.is_empty() {
            return SeatSwapVerdict::NoEvidence;
        }
        if build_digests.is_empty() {
            return SeatSwapVerdict::NoEvidence;
        }
        let rows = self
            .gate
            .accepted()
            .iter()
            .chain(self.gate.rejected().iter())
            .filter(|d| build_digests.contains(d.as_bytes()))
            .count();
        let repeated_hits = self
            .gate
            .rejected()
            .iter()
            .filter(|d| build_digests.contains(d.as_bytes()))
            .count();
        if repeated_hits > 0 {
            SeatSwapVerdict::StandDown {
                repeated_hits,
                rows,
            }
        } else {
            SeatSwapVerdict::Proceed {
                repeated_hits,
                rows,
            }
        }
    }
}

fn hex_of(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}
