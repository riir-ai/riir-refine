//! The ANE pregate (Plan 377 — the owner's two-pass ask: "code scan for ane
//! first pass before apply second pass like kernel"): detect ANE-bearing
//! files BEFORE the ane matcher fans out, then scope the fan-out to the
//! signaling files (and within them, to the signal lines ± window).
//!
//! The shape is [`crate::kernel_pregate`]'s (Issue 208 / Bench 131) applied
//! to the Apple Neural Engine surface:
//!
//! - **First pass** (this module): ONE recall-biased byte-scan per file —
//!   a pinned needle set over the raw source (case-insensitive ASCII
//!   substring, first-byte prefiltered) + a path-component word signal.
//! - **Second pass** (the matcher): the `ane` domain joins the fan-out only
//!   where the pregate allows the span — non-signaling files never pay the
//!   ane index query, signaling files pay it only near the signal.
//!
//! **Recall-biased BY DESIGN** (the kernel doctrine): a false positive costs
//! one unneeded index query on one file; a false negative HIDES a matching
//! span — the kill criterion. The needles are therefore CoreML/ANE-vocabulary
//! shaped, not import-exact.
//!
//! **PROMOTION POSTURE (opt-in until the G2 cost leg passes):** `ane_pregate`
//! implies `ane`; both are opt-in features, NOT default, NOT the release
//! set. The demote env [`DEMOTE_ENV`] (`RIIR_ANE_PREGATE`, exact literal
//! `0`) is DEMOTE-ONLY from birth (env-switch law): when the feature is
//! promoted to default, unset = armed and the exact `0` restores the
//! ungated incumbent byte-identically. While opt-in, the SAME env is the
//! honest A/B arm (armed vs `=0` inside one build). The explicit
//! `--domain ane` bypass (armed once per process by the scan entry) wins
//! over everything — the gate never refuses an explicit ask.
//!
//! **Pinned needles (Plan 377, pinned BEFORE any timing run; widening is a
//! NEW pre-registration, never an in-place amendment):**
//!
//! | needle | catches | accepted false positives |
//! |---|---|---|
//! | `coreml` | `CoreML`, `coremltools` | `coreml`-prefixed names |
//! | `core_ml` | `objc2_core_ml` | underscore-prefixed cousins |
//! | `core-ml` | `objc2-core-ml` | hyphenated cousins |
//! | `mlmodel` | `MLModel`, `.mlmodel`, `mlmodelc` | hypothetical mlmodels |
//! | `mlpackage` | `.mlpackage` | — |
//! | `mlmultiarray` | `MLMultiArray` | — |
//! | `mlstate` | `MLState` (streaming state) | — |
//! | `mlcomputeunits` | `MLComputeUnits` | — |
//! | `mlcomputeplan` | `MLComputePlan` | — |
//! | `neuralengine` | `NeuralEngine` identifiers | — |
//! | `neural engine` (prose) | the phrase in comments/docs | GPU-vendor prose |
//! | `ane` (BOUNDARY) | the bare word ANE (either case) | standalone prose `ane` |
//!
//! The boundary needle matches only when the bytes on BOTH sides of the
//! 3-byte hit are not ASCII alphabetic — `lane`/`plane`/`mane`/`canes`
//! never signal, `the ANE`, `(ANE)`, `ANE:` do.

/// The span-scope window (Plan 377, pinned BEFORE any timing run): a span is
/// in scope when its line range intersects any signal line ± this many
/// lines. Value borrowed from the kernel pregate's measured
/// [`crate::kernel_pregate::SPAN_SCOPE_WINDOW_LINES`] precedent (24);
/// widening it is a NEW pre-registration.
pub const ANE_SPAN_SCOPE_WINDOW_LINES: usize = 24;

/// The demote env (DEMOTE-ONLY): the exact literal `0` disarms the gate for
/// the whole process (every file `Off` — the ungated incumbent,
/// byte-identically); unset AND junk both ride the armed default (junk env
/// is unset-equivalent, never a demote — the kernel scope-env law).
const DEMOTE_ENV: &str = "RIIR_ANE_PREGATE";

/// The pinned CoreML/ANE needles — case-insensitive ASCII substring over the
/// raw source (see the module header table). ASCII by construction; the
/// scanner folds ASCII-letters-only ([`crate::kernel_pregate`] Issue 214).
const SIGNAL_NEEDLES: [&str; 10] = [
    "coreml",
    "core_ml",
    "core-ml",
    "mlmodel",
    "mlpackage",
    "mlmultiarray",
    "mlstate",
    "mlcomputeunits",
    "mlcomputeplan",
    "neuralengine",
];

/// The prose needle — a two-word phrase the substring scan matches across
/// the space byte (comments/doc text carry it; identifiers cannot).
const PROSE_NEEDLES: [&str; 1] = ["neural engine"];

/// The BOUNDARY needle — the bare word `ANE`. Matched only when the bytes
/// immediately before and after the 3-byte hit are not ASCII alphabetic.
const BOUNDARY_NEEDLES: [&str; 1] = ["ane"];

/// First-byte prefilter: the distinct lowercased first bytes of every
/// needle above — the single-pass scan tests a position against the needle
/// list only when its byte lands in this set.
const NEEDLE_FIRST_BYTES: [u8; 4] = *b"cmna";

fn needle_first_byte_class(b: u8) -> bool {
    NEEDLE_FIRST_BYTES.contains(&b)
}

/// Case-insensitive ASCII match of `needle` at `hay[pos..]` (the
/// [`crate::kernel_pregate::ci_matches`] fold — ASCII-letters-only).
fn ci_matches(hay: &[u8], pos: usize, needle: &[u8]) -> bool {
    if pos + needle.len() > hay.len() {
        return false;
    }
    hay[pos..pos + needle.len()]
        .iter()
        .zip(needle.iter())
        .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// The boundary check for [`BOUNDARY_NEEDLES`]: the bytes at `pos - 1` and
/// `pos + needle.len()` must not be ASCII alphabetic (string edges count as
/// non-alphabetic). The needles are ASCII by construction, so byte offsets
/// cannot split a UTF-8 codepoint at the BOUNDARIES — the hit itself is
/// ASCII, and a multi-byte char cannot be alphabetic-ASCII.
fn boundary_ok(hay: &[u8], pos: usize, needle_len: usize) -> bool {
    let before_ok = pos == 0 || !hay[pos - 1].is_ascii_alphabetic();
    let after = pos + needle_len;
    let after_ok = after >= hay.len() || !hay[after].is_ascii_alphabetic();
    before_ok && after_ok
}

/// Any path component (split on `/`, `\`, `-`, `_`, `.`) whose whole word is
/// `ane` (case-insensitive)? `crates/riir-infer-ane/src/lib.rs`,
/// `laya_ane.rs`, `ane/mod.rs` signal; `domain_router.rs`, `plane.rs`,
/// `panel.rs` never do (the word must be EXACTLY `ane`).
#[must_use]
pub fn path_signals(path: &str) -> bool {
    path.split(['/', '\\', '-', '_', '.'])
        .any(|word| word.eq_ignore_ascii_case("ane"))
}

/// The 1-based lines (sorted, deduped) carrying any needle hit. One pass
/// over the bytes with the first-byte prefilter; overlapping hits collapse
/// into the one line. A path-signaled file never reaches this scan (the
/// whole file is the signal — see [`AnePlan::PathSignal`]).
#[must_use]
pub fn signal_lines(src: &str) -> Vec<u32> {
    let hay = src.as_bytes();
    let mut lines = Vec::new();
    let mut last = 0u32;
    let mut line: u32 = 1;
    let mut i = 0usize;
    while i < hay.len() {
        let b = hay[i].to_ascii_lowercase();
        if needle_first_byte_class(b) {
            let mut hit = false;
            for needle in SIGNAL_NEEDLES.iter().chain(PROSE_NEEDLES.iter()) {
                if needle.as_bytes()[0].to_ascii_lowercase() == b
                    && ci_matches(hay, i, needle.as_bytes())
                {
                    hit = true;
                    break;
                }
            }
            if !hit && b == b'a' {
                for needle in BOUNDARY_NEEDLES {
                    if ci_matches(hay, i, needle.as_bytes()) && boundary_ok(hay, i, needle.len()) {
                        hit = true;
                        break;
                    }
                }
            }
            if hit && line > last {
                lines.push(line);
                last = line;
            }
        }
        if hay[i] == b'\n' {
            line += 1;
        }
        i += 1;
    }
    lines
}

/// Any signal line within `[start - WINDOW, end + WINDOW]`? (Binary search;
/// `signal_lines` is sorted ascending by construction.)
#[must_use]
pub fn span_in_scope(start_line: usize, end_line: usize, signal_lines: &[u32]) -> bool {
    let lo = start_line.saturating_sub(ANE_SPAN_SCOPE_WINDOW_LINES);
    let hi = end_line.saturating_add(ANE_SPAN_SCOPE_WINDOW_LINES);
    let idx = signal_lines.partition_point(|&l| (l as usize) < lo);
    signal_lines.get(idx).is_some_and(|&l| (l as usize) <= hi)
}

fn demoted() -> bool {
    static DEMOTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *DEMOTED.get_or_init(|| std::env::var(DEMOTE_ENV).as_deref() == Ok("0"))
}

/// The explicit-`--domain ane` bypass (the operator asked for the domain,
/// the gate never refuses an explicit ask). Armed once per process by the
/// scan entry (`heal.rs` main) BEFORE any file walk; consulted by
/// [`AnePlan::for_file`]. ONE process-level static — the arm and the reader
/// must see the same cell.
static DOMAIN_BYPASS: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// Arms the bypass for the rest of the process (idempotent; a later
/// [`AnePlan::for_file`] resolves `Off`).
pub fn arm_domain_bypass() {
    let _ = DOMAIN_BYPASS.set(());
}

#[must_use]
fn domain_bypass_armed() -> bool {
    DOMAIN_BYPASS.get().is_some()
}

/// The per-file ane-matcher participation plan, computed ONCE per file per
/// run (the run-level index the scan walk builds as it reads).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnePlan {
    /// The gate is inert (feature off, the demote env, or the `--domain ane`
    /// bypass): every span ane-allowed — the exact pre-gate behavior.
    Off,
    /// Path signal: the file's own path names ane (`riir-infer-ane`,
    /// `laya_ane.rs`, an `ane/` directory) — the WHOLE file is an ANE
    /// surface; every span allowed without a line scan.
    PathSignal,
    /// Content signal: the file carries needle hits; a span is ane-allowed
    /// iff its lines intersect a signal line ±
    /// [`ANE_SPAN_SCOPE_WINDOW_LINES`]. An empty line set denies every span
    /// (the file-level deny, subsumed).
    Scope { signal_lines: Vec<u32> },
}

impl AnePlan {
    /// The per-file decision. Zero-cost `Off` when the `ane_pregate` feature
    /// is off (a folded branch — the default posture never scans).
    #[must_use]
    pub fn for_file(src: &str, path: &str) -> Self {
        if !cfg!(feature = "ane_pregate") || demoted() || domain_bypass_armed() {
            return Self::Off;
        }
        if path_signals(path) {
            return Self::PathSignal;
        }
        Self::Scope {
            signal_lines: signal_lines(src),
        }
    }

    /// The per-span verdict. `true` keeps the exact pre-gate behavior; the
    /// denied shape is the ane-disabled shape (no `by_domain` entry).
    #[must_use]
    pub fn span_allowed(&self, start_line: usize, end_line: usize) -> bool {
        match self {
            Self::Off | Self::PathSignal => true,
            Self::Scope { signal_lines } => span_in_scope(start_line, end_line, signal_lines),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coreml_vocabulary_signals_case_insensitively() {
        assert_eq!(signal_lines("let x = 1;\nlet y = 2;\n"), [] as [u32; 0]);
        assert_eq!(
            signal_lines("use objc2_core_ml::MLModel;\nlet x = 1;\n"),
            vec![1]
        );
        assert_eq!(
            signal_lines("// coremltools conversion target\nfn f() {}\n"),
            vec![1]
        );
        assert_eq!(signal_lines("let units = MLComputeUnits::all;\n"), vec![1]);
        assert_eq!(signal_lines("let plan = MLComputePlan::new();\n"), vec![1]);
        assert_eq!(signal_lines("struct MlState;\n"), vec![1]);
    }

    #[test]
    fn neural_engine_phrase_and_identifier_signal() {
        assert_eq!(
            signal_lines("// runs the graph on the Neural Engine\nfn f() {}\n"),
            vec![1]
        );
        assert_eq!(
            signal_lines("let engine = neuralengine_default();\n"),
            vec![1]
        );
    }

    #[test]
    fn boundary_ane_signals_but_substrings_never_do() {
        assert_eq!(signal_lines("// the ANE residency is 100%\n"), vec![1]);
        assert_eq!(signal_lines("(ANE) placement\n"), vec![1]);
        // lane / plane / mane / canes: the boundary needle must NOT fire —
        // and no other needle substring-matches them.
        assert_eq!(signal_lines("let lane = Lane::new();\n"), [] as [u32; 0]);
        assert_eq!(signal_lines("fn plane_diagnostics() {}\n"), [] as [u32; 0]);
        assert_eq!(signal_lines("// the mane event\n"), [] as [u32; 0]);
        // Word-adjacent punctuation signals; plural does not.
        assert_eq!(signal_lines("ANE:\n"), vec![1]);
        assert_eq!(signal_lines("the ANEs\n"), [] as [u32; 0]);
        // Underscore/digit adjacency counts as a boundary (ane_2 is ane-ish).
        assert_eq!(signal_lines("ane_2 = 1;\n"), vec![1]);
    }

    #[test]
    fn signal_lines_carry_correct_one_based_numbers() {
        let src = "let a = 1;\nlet b = MLModel::load()?;\nlet c = 3;\n// ANE\n";
        assert_eq!(signal_lines(src), vec![2, 4]);
    }

    #[test]
    fn path_word_ane_signals_exact_word_only() {
        assert!(path_signals("crates/riir-infer-ane/src/lib.rs"));
        assert!(path_signals("laya_ane.rs"));
        assert!(path_signals("ane/mod.rs"));
        assert!(path_signals("src/ANE/convert.rs"));
        assert!(!path_signals("src/domain_router.rs"));
        assert!(!path_signals("src/plane.rs"));
        assert!(!path_signals("src/panel/mod.rs"));
        assert!(!path_signals("src/lane/mod.rs"));
    }

    #[test]
    fn scope_window_boundaries_match_the_pinned_constant() {
        let signals = vec![100u32];
        let w = ANE_SPAN_SCOPE_WINDOW_LINES;
        assert!(span_in_scope(100 - w, 100, &signals));
        assert!(span_in_scope(100, 100 + w, &signals));
        assert!(!span_in_scope(1, 100 - w - 1, &signals));
        assert!(!span_in_scope(100 + w + 1, 200, &signals));
        assert!(!span_in_scope(1, 1000, &[]));
    }

    #[test]
    fn off_and_path_signal_allow_every_span() {
        assert!(AnePlan::Off.span_allowed(1, usize::MAX));
        assert!(AnePlan::PathSignal.span_allowed(1, usize::MAX));
        let denied = AnePlan::Scope {
            signal_lines: Vec::new(),
        };
        assert!(!denied.span_allowed(1, 2));
    }

    #[test]
    fn plan_for_file_honors_the_demote_env_and_path_signal() {
        if !cfg!(feature = "ane_pregate") {
            // The feature-off posture: Off regardless of the env and the
            // path (the folded branch — the default posture never scans).
            assert_eq!(
                AnePlan::for_file("fn main() {}\n", "plain.rs"),
                AnePlan::Off
            );
            assert_eq!(
                AnePlan::for_file("fn main() {}\n", "crates/riir-infer-ane/src/lib.rs"),
                AnePlan::Off
            );
            return;
        }
        // The demote env is read once per process — inside a test binary it
        // is whatever the harness set. Assert the CONTRACT, not one posture:
        // `Off` when demoted, otherwise a path-signaled file is `PathSignal`
        // and a clean file's `Scope` denies every span on a signal-free
        // file.
        let plan = AnePlan::for_file("fn main() {}\n", "plain.rs");
        if demoted() {
            assert_eq!(plan, AnePlan::Off);
        } else {
            assert!(
                matches!(plan, AnePlan::Scope { ref signal_lines } if signal_lines.is_empty()),
                "a plain file under the armed gate must deny (Scope, no lines)"
            );
        }
        let signaled = AnePlan::for_file("fn main() {}\n", "crates/riir-infer-ane/src/lib.rs");
        if demoted() {
            assert_eq!(signaled, AnePlan::Off);
        } else {
            assert_eq!(signaled, AnePlan::PathSignal);
        }
    }
}
