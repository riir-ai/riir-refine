//! Issue 232 T1 — the consumed-window ledger (Research 495, the T³
//! holdout-epistemology extraction's strongest item; source: dang2026t3 via
//! `riir-train/.research/495`).
//!
//! **The law:** a selection may be re-judged only on outcome windows its
//! selection did not consume. Self-scoring on consumed data doesn't stall —
//! it INVERTS below baseline (T³ measured own-half testing at −0.158 by
//! epoch 1, +0.046 WORSE than the record script by epoch 5). Today this
//! repo's selection surfaces re-read the same recorded-outcome store that
//! shaped them: [`crate::traj_store::TrajStore::recompute_elo`] replays
//! every trajectory row it previously rated, and the Elo/`BetaPosterior`
//! ratings feeding [`select_best_candidate`](crate::self_evolve) come from
//! that same store — the adaptive-reuse pattern (Dwork et al., Science
//! 2015, is the class; Bai et al. arXiv:2010.05843 the split theory; the
//! delta here is surface-specific, never "novel mechanism").
//!
//! **What the ledger records:** WHICH windows each selection consumed (the
//! window's BLAKE3 digest over its canonical row encoding) + a per-window
//! REUSE counter. The reuse count itself is the poison variable — T³'s
//! valid→poisoned curve is NON-MONOTONE in it — so a boolean "burnt" flag
//! would miss the shape; the counter is the ledger's first-class output.
//!
//! **The verdict law (report-only):** a consumed window's re-validation
//! verdict is [`WindowRead::Consumed`] → **UNDECIDABLE** — the ledger
//! reports the reuse count and NEVER folds it into a refusal. Nothing here
//! blocks a replay, demotes a rule, or gates a flag promotion; the
//! correlation/utility cell (the issue's next gate) decides whether the
//! signal earns an actuator, and that decision is NOT this module's.
//!
//! **Tier discipline:** every constant in the T³ paper (the 4-pass knee,
//! the epoch-5 inversion) is a TRAINING-surface number — none is imported.
//! The knee for THIS surface, if the cell ever arms one, is measured here
//! first.
//!
//! ## Persistence
//!
//! JSON, versioned (`format_version` 1), caller-named path (the CLI uses
//! `<root>/.refine/window_ledger.json`). The ledger grows one entry per
//! DISTINCT window state (each store mutation = a new digest) — a report
//! artifact, not a sync surface; a future compaction lane is deliberate
//! non-scope.
//!
//! ## Feature posture
//!
//! `consumed_window_ledger` (opt-in, default-OFF, no implies — std +
//! `blake3` only). Feature-off is byte-identical by construction: the gate
//! adds new code paths only, no shared-path edit. The traj-store hook is
//! `all(consumed_window_ledger, latent_retrieval)`.

use std::collections::BTreeMap;

/// The ledger file's schema version. A reader refusing an unknown version
/// is fail-closed (the file is advisory state — the ledger rebuilds from
/// fresh at version refusal, never guesses).
pub const LEDGER_FORMAT_VERSION: u64 = 1;

/// One consumption event's durable half: how many consumptions the window
/// had before this one (the reuse count at read time), who read it, when.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowConsumption {
    /// Caller-named selection id (`"recompute-elo-cli"`, a fixture id, …).
    pub selection: String,
    /// Unix seconds at the read (provenance only — never parsed back).
    pub epoch: u64,
    /// Consumptions of this window BEFORE this read (0 = first).
    pub reuse: u32,
}

/// The verdict of consulting the ledger before a selection reads a window.
/// Report-only: `Consumed` is information, never a refusal — the caller
/// proceeds and prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowRead {
    /// First consumption — nothing prior consumed this exact window state.
    Fresh,
    /// Re-consumption — the window was read `reuse` times before (by any
    /// selection, this one included). The re-validation's validity is
    /// **UNDECIDABLE**: the report says so and moves on.
    Consumed { reuse: u32 },
}

/// Per-window durable state.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowState {
    /// Total consumptions (the reuse counter's source of truth).
    pub count: u32,
    /// First consumption's epoch.
    pub first_epoch: u64,
    /// Last consumption's epoch.
    pub last_epoch: u64,
}

/// The consumed-window ledger: window digest → consumption state, plus the
/// per-selection index (WHICH windows each selection consumed — deduped,
/// insertion-ordered).
#[derive(Debug, Clone, Default)]
pub struct WindowLedger {
    windows: BTreeMap<String, WindowState>,
    selections: BTreeMap<String, Vec<String>>,
    consumptions: Vec<WindowConsumption>,
}

impl WindowLedger {
    /// An empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Consult the ledger BEFORE a selection reads `digest`, then record
    /// the consumption. `Fresh` on first sight; `Consumed { reuse }` when
    /// the window was consumed `reuse` times before — the caller's cue to
    /// report UNDECIDABLE and proceed (this module never refuses).
    pub fn consult(&mut self, digest: &str, selection: &str, epoch: u64) -> WindowRead {
        let state = self.windows.entry(digest.to_string()).or_default();
        let read = if state.count == 0 {
            state.first_epoch = epoch;
            WindowRead::Fresh
        } else {
            WindowRead::Consumed { reuse: state.count }
        };
        state.count = state.count.saturating_add(1);
        state.last_epoch = epoch;
        let seen = self.selections.entry(selection.to_string()).or_default();
        if seen.last().map(String::as_str) != Some(digest) {
            seen.push(digest.to_string());
        }
        self.consumptions.push(WindowConsumption {
            selection: selection.to_string(),
            epoch,
            reuse: read_reuse(&read),
        });
        read
    }

    /// The reuse count of `digest` (0 = never consumed).
    #[must_use]
    pub fn reuse_of(&self, digest: &str) -> u32 {
        self.windows.get(digest).map_or(0, |s| s.count)
    }

    /// The distinct windows a selection has consumed, insertion-ordered.
    #[must_use]
    pub fn windows_of(&self, selection: &str) -> &[String] {
        self.selections.get(selection).map_or(&[], |v| v.as_slice())
    }

    /// The full consumption log (the per-event provenance view).
    #[must_use]
    pub fn consumptions(&self) -> &[WindowConsumption] {
        &self.consumptions
    }

    /// The report line for a read — the ONLY surface a caller needs to
    /// print. The verdict word is pinned (`UNDECIDABLE`); the ledger
    /// refuses to phrase a refusal.
    #[must_use]
    pub fn report_line(&self, digest: &str, read: &WindowRead) -> String {
        match read {
            WindowRead::Fresh => format!(
                "[consumed-window] digest={} reuse=0 verdict=FRESH — first consumption, no prior read",
                short(digest)
            ),
            WindowRead::Consumed { reuse } => format!(
                "[consumed-window] digest={} reuse={reuse} verdict=UNDECIDABLE — re-judging on a consumed window; a reused window's verdict does not certify (I232 T1, report-only)",
                short(digest)
            ),
        }
    }

    /// Serialize (versioned). The digest keys are hex — a `BTreeMap` keeps
    /// the encoding canonical (byte-identical save → save).
    #[must_use]
    pub fn to_json(&self) -> String {
        #[derive(serde::Serialize)]
        struct Doc<'a> {
            format_version: u64,
            windows: &'a BTreeMap<String, WindowState>,
            selections: &'a BTreeMap<String, Vec<String>>,
            consumptions: &'a [WindowConsumption],
        }
        serde_json::to_string(&Doc {
            format_version: LEDGER_FORMAT_VERSION,
            windows: &self.windows,
            selections: &self.selections,
            consumptions: &self.consumptions,
        })
        .unwrap_or_else(|_| "{}".to_string())
    }

    /// Parse a ledger file's bytes. Unknown version → loud error (the
    /// caller starts fresh, never guesses); corrupt JSON → loud error.
    ///
    /// # Errors
    ///
    /// Unknown `format_version` or malformed JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct Doc {
            format_version: u64,
            windows: BTreeMap<String, WindowState>,
            selections: BTreeMap<String, Vec<String>>,
            consumptions: Vec<WindowConsumption>,
        }
        let doc: Doc = serde_json::from_slice(bytes)
            .map_err(|e| format!("consumed-window ledger: corrupt JSON: {e}"))?;
        if doc.format_version != LEDGER_FORMAT_VERSION {
            return Err(format!(
                "consumed-window ledger: format_version {} unsupported (this build reads {LEDGER_FORMAT_VERSION})",
                doc.format_version
            ));
        }
        Ok(Self {
            windows: doc.windows,
            selections: doc.selections,
            consumptions: doc.consumptions,
        })
    }

    /// Load from `path` (missing file = fresh ledger, the cold start);
    /// unreadable/corrupt → fresh + the reason on stderr — an advisory
    /// artifact must not brick the lane it reports on.
    #[must_use]
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read(path) {
            Ok(bytes) => match Self::from_json(&bytes) {
                Ok(ledger) => ledger,
                Err(e) => {
                    eprintln!("consumed-window: {e} — starting fresh");
                    Self::new()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::new(),
            Err(e) => {
                eprintln!(
                    "consumed-window: read {} failed: {e} — starting fresh",
                    path.display()
                );
                Self::new()
            }
        }
    }

    /// Persist atomically (tmp + rename). Best-effort: a failure is
    /// reported on stderr, never fatal — the ledger is advisory state.
    pub fn save(&self, path: &std::path::Path) {
        let json = self.to_json();
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, json.as_bytes()) {
            eprintln!("consumed-window: write {} failed: {e}", tmp.display());
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            eprintln!("consumed-window: rename {} failed: {e}", path.display());
        }
    }
}

/// The reuse half of a [`WindowRead`] (for the consumption log's stamp).
fn read_reuse(read: &WindowRead) -> u32 {
    match read {
        WindowRead::Fresh => 0,
        WindowRead::Consumed { reuse } => *reuse,
    }
}

/// The report line's short digest form (first 12 hex chars).
fn short(digest: &str) -> &str {
    &digest[..digest.len().min(12)]
}

#[cfg(all(test, feature = "consumed_window_ledger"))]
mod tests {
    use super::*;

    #[test]
    fn fresh_then_consumed_flags_own_and_foreign_rereads() {
        // I232 G1: a rule re-scored on its own selection window FLAGS (the
        // report-only refusal-free verdict); a foreign selection reading
        // the same consumed window flags identically — the window is stale
        // regardless of who reads it.
        let mut ledger = WindowLedger::new();
        assert_eq!(ledger.consult("d1", "selA", 100), WindowRead::Fresh);
        assert_eq!(
            ledger.consult("d1", "selA", 200),
            WindowRead::Consumed { reuse: 1 }
        );
        assert_eq!(
            ledger.consult("d1", "selB", 300),
            WindowRead::Consumed { reuse: 2 }
        );
        assert_eq!(ledger.reuse_of("d1"), 3);
        // The per-selection index records WHICH windows each consumed.
        assert_eq!(ledger.windows_of("selA"), &["d1".to_string()]);
        assert_eq!(ledger.windows_of("selB"), &["d1".to_string()]);
        assert_eq!(ledger.consumptions().len(), 3);
        assert_eq!(ledger.consumptions()[2].reuse, 2);
    }

    #[test]
    fn disjoint_window_proceeds_fresh() {
        // I232 G1: a disjoint (evolved) window is a Fresh read even when a
        // sibling window is heavily consumed.
        let mut ledger = WindowLedger::new();
        for epoch in 1..=4 {
            let _ = ledger.consult("d1", "sel", epoch);
        }
        assert_eq!(ledger.consult("d2", "sel", 9), WindowRead::Fresh);
        assert_eq!(ledger.reuse_of("d2"), 1);
    }

    #[test]
    fn report_line_pins_the_verdict_words() {
        // The verdict law in vocabulary: a consumed read reports
        // UNDECIDABLE and never phrasings a refusal; a fresh read says
        // FRESH.
        let mut ledger = WindowLedger::new();
        let fresh = ledger.consult("abc", "s", 1);
        let line = ledger.report_line("abc", &fresh);
        assert!(line.contains("verdict=FRESH"), "{line}");
        assert!(!line.contains("UNDECIDABLE"), "{line}");
        let consumed = ledger.consult("abc", "s", 2);
        let line = ledger.report_line("abc", &consumed);
        assert!(line.contains("verdict=UNDECIDABLE"), "{line}");
        assert!(line.contains("reuse=1"), "{line}");
        assert!(line.contains("report-only"), "{line}");
        for banned in ["refus", "blocked", "demote", "skip"] {
            assert!(!line.to_ascii_lowercase().contains(banned), "{line}");
        }
    }

    #[test]
    fn persistence_round_trips_and_refuses_unknown_versions() {
        let mut ledger = WindowLedger::new();
        let _ = ledger.consult("d1", "selA", 100);
        let _ = ledger.consult("d1", "selA", 200);
        let _ = ledger.consult("d2", "selB", 300);
        let json = ledger.to_json();
        let back = WindowLedger::from_json(json.as_bytes()).expect("round trip");
        assert_eq!(back.reuse_of("d1"), 2);
        assert_eq!(back.reuse_of("d2"), 1);
        assert_eq!(back.windows_of("selB"), &["d2".to_string()]);
        assert_eq!(back.consumptions().len(), 3);
        // Canonical encoding: save → save is byte-identical.
        assert_eq!(json, back.to_json());
        // Unknown version → loud refusal, never a guess.
        let mutated = json.replace(
            &format!("\"format_version\":{LEDGER_FORMAT_VERSION}"),
            "\"format_version\":999",
        );
        assert!(WindowLedger::from_json(mutated.as_bytes()).is_err());
        // Corrupt JSON → loud refusal.
        assert!(WindowLedger::from_json(b"{not json").is_err());
    }

    #[test]
    fn load_missing_file_is_a_fresh_cold_start() {
        let dir = std::env::temp_dir().join(format!("cwl_load_{}", std::process::id()));
        let path = dir.join("ledger.json");
        let ledger = WindowLedger::load(&path);
        assert_eq!(ledger.reuse_of("anything"), 0);
    }
}
