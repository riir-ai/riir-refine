//! The kernel pregate (Issue 208, Bench 131 pre-registration): detect
//! kernel-bearing files BEFORE the kernel matcher fans out, then scope the
//! fan-out to the indexed/mapped files only.
//!
//! The G5-cost fence (added p50 ≤ 1.0 / p95 ≤ 5.0 ms per file over the
//! registered mixed walk, Benches 125/129) fails at 33–39× because the
//! kernel domain's per-span query mass runs on EVERY file of the scan. The
//! pregate removes whole FILES (`Gate`) or non-signal SPANS (`Scope`) from
//! the kernel matcher's input set at ~lexical cost — Bench 129's named
//! structural lever (b), "a cheaper first-pass scan key", owner-directed
//! 2026-11-05.
//!
//! **Recall-biased BY DESIGN:** a false positive costs only the old matcher
//! time on one file; a false negative HIDES a matching span — the bench's
//! kill criterion. The needle set is therefore GPU-vocabulary-shaped, not
//! import-exact: any hit signals the file.
//!
//! **Inert without the feature:** the module is always compiled (the unit
//! tests run at default features), but [`KernelPlan::for_file`] returns
//! [`KernelPlan::Off`] without scanning when `kernel_pregate` is off —
//! byte-identical default posture, zero added cost. The wiring lives in
//! `draft::latent_matcher::suggest_file_inner`; the scan path's per-span
//! verdict rides one bool, every other domain untouched.

/// The span-scope window (pinned in Bench 131 BEFORE any timing run): a span
/// is in scope when its line range intersects any signal line ± this many
/// lines. Widening it is a NEW pre-registration, never an in-place amendment.
pub const SPAN_SCOPE_WINDOW_LINES: usize = 24;

/// The scope-arm switch (Bench 131's GATE+SCOPE arm): promote-only env —
/// arms on the exact literals `1`/`true`/`on` (the env-switch law), read
/// once per process. Without it the feature arms the file-level GATE arm.
const SCOPE_ENV: &str = "RIIR_KERNEL_PREGATE_SCOPE";

/// The line-cap switch (Bench 134's CAP arms): promote-only env, the exact
/// pinned literals `4`/`2` — see [`line_cap`].
const LINE_CAP_ENV: &str = "RIIR_KERNEL_PREGATE_LINE_CAP";

/// The pinned needle set (Bench 131, pinned pre-run; widening is a NEW
/// pre-registration). Case-insensitive ASCII substring over the raw source;
/// `.wgsl` paths always signal. `cube` deliberately subsumes `cubecl` and
/// `#[cube]` (and matches `cubemap`/`cubic` — accepted false positives under
/// the recall bias above).
const SIGNAL_NEEDLES: [&str; 10] = [
    "kernel",
    "wgpu",
    "cube",
    "wgsl",
    "workgroup",
    "threadgroup",
    "compute_pass",
    "computepipeline",
    "dispatch_compute",
    "@compute",
];

/// First-byte prefilter: the distinct lowercased first bytes of
/// [`SIGNAL_NEEDLES`] — the single-pass scan tests a position against the
/// needle list only when its byte lands in this set.
const NEEDLE_FIRST_BYTES: [u8; 6] = *b"kwctd@";

fn needle_first_byte_class(b: u8) -> bool {
    NEEDLE_FIRST_BYTES.contains(&b)
}

/// `.wgsl` paths are kernel surfaces by definition (the kernel domain's
/// non-syn scan lane scans exactly these, Issue 025 T4).
#[must_use]
fn is_wgsl_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wgsl"))
}

/// Case-insensitive ASCII match of `needle` at `hay[pos..]` (both sides
/// lowercased via `| 0x20` on ASCII letters; the needles are ASCII by
/// construction).
fn ci_matches(hay: &[u8], pos: usize, needle: &[u8]) -> bool {
    if pos + needle.len() > hay.len() {
        return false;
    }
    hay[pos..pos + needle.len()]
        .iter()
        .zip(needle.iter())
        .all(|(a, b)| (a | 0x20) == (b | 0x20))
}

/// The 1-based lines (sorted, deduped) carrying any needle hit. `.wgsl`
/// paths signal at line 1 with an empty source (the whole file IS the
/// signal). One pass over the bytes with the first-byte prefilter; a hit
/// records the current line and the scan continues (overlapping needle
/// hits collapse into the one line).
#[must_use]
pub fn signal_lines(src: &str, path: &str) -> Vec<u32> {
    if is_wgsl_path(path) {
        return vec![1];
    }
    let hay = src.as_bytes();
    let mut lines = Vec::new();
    let mut last = 0u32;
    let mut line: u32 = 1;
    let mut i = 0usize;
    while i < hay.len() {
        let b = hay[i] | 0x20;
        if needle_first_byte_class(b) {
            for needle in SIGNAL_NEEDLES {
                if needle.as_bytes()[0] | 0x20 == b && ci_matches(hay, i, needle.as_bytes()) {
                    if line > last {
                        lines.push(line);
                        last = line;
                    }
                    break;
                }
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
    let lo = start_line.saturating_sub(SPAN_SCOPE_WINDOW_LINES);
    let hi = end_line.saturating_add(SPAN_SCOPE_WINDOW_LINES);
    let idx = signal_lines.partition_point(|&l| (l as usize) < lo);
    signal_lines.get(idx).is_some_and(|&l| (l as usize) <= hi)
}

fn scope_armed() -> bool {
    static ARMED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ARMED.get_or_init(|| matches!(std::env::var(SCOPE_ENV).as_deref(), Ok("1" | "true" | "on")))
}

/// The per-span line-query cap (Bench 134, issue 208 T6): promote-only env
/// `RIIR_KERNEL_PREGATE_LINE_CAP` — arms ONLY on the exact pinned literals
/// `"4"` and `"2"` (any other value is unset-equivalent, never armed; the
/// env-switch law). Read once per process. `None` = uncapped (the exact
/// pre-cap behavior).
#[must_use]
pub fn line_cap() -> Option<usize> {
    static CAP: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| match std::env::var(LINE_CAP_ENV).as_deref() {
        Ok("4") => Some(4),
        Ok("2") => Some(2),
        _ => None,
    })
}

/// The explicit-`--domain kernel` bypass (the operator asked for the domain,
/// the gate never refuses an explicit ask). Armed once per process by the
/// scan entry (`heal.rs` main) BEFORE any file walk; consulted by
/// [`KernelPlan::for_file`]. ONE process-level static — the arm and the
/// reader must see the same cell.
static DOMAIN_BYPASS: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// Arms the bypass for the rest of the process (idempotent; a later
/// [`KernelPlan::for_file`] resolves `Off`).
pub fn arm_domain_bypass() {
    let _ = DOMAIN_BYPASS.set(());
}

#[must_use]
fn domain_bypass_armed() -> bool {
    DOMAIN_BYPASS.get().is_some()
}

/// The per-file kernel-matcher participation plan, computed ONCE per file
/// per run (the run-level index the scan walk builds as it reads).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelPlan {
    /// The gate is inert (feature off, or the `--domain kernel` bypass):
    /// every span kernel-allowed — the exact pre-gate behavior.
    Off,
    /// File-level gate (Bench 131's GATE arm): kernel allowed iff the file
    /// signaled.
    Gate { signaled: bool },
    /// Span scope (the GATE+SCOPE arm): kernel allowed iff the span's lines
    /// intersect a signal line ± [`SPAN_SCOPE_WINDOW_LINES`]. A file that
    /// never signal denies every span (Gate's semantics, subsumed).
    Scope { signal_lines: Vec<u32> },
}

impl KernelPlan {
    /// The per-file decision. Zero-cost `Off` when the `kernel_pregate`
    /// feature is off (a folded branch — the default posture never scans).
    #[must_use]
    pub fn for_file(src: &str, path: &str) -> Self {
        if !cfg!(feature = "kernel_pregate") || domain_bypass_armed() {
            return Self::Off;
        }
        if scope_armed() {
            Self::Scope {
                signal_lines: signal_lines(src, path),
            }
        } else {
            Self::Gate {
                signaled: !signal_lines(src, path).is_empty(),
            }
        }
    }

    /// The per-span verdict. `true` keeps the exact pre-gate behavior; the
    /// denied shape is the kernel-disabled shape (no `by_domain` entry).
    #[must_use]
    pub fn span_allowed(&self, start_line: usize, end_line: usize) -> bool {
        match self {
            Self::Off => true,
            Self::Gate { signaled } => *signaled,
            Self::Scope { signal_lines } => span_in_scope(start_line, end_line, signal_lines),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_vocabulary_signals_case_insensitively() {
        assert_eq!(
            signal_lines("let x = 1;\nlet y = 2;\n", "a.rs"),
            [] as [u32; 0]
        );
        assert_ne!(
            signal_lines("use cubecl::prelude::*;\nlet x = 1;\n", "a.rs"),
            [] as [u32; 0]
        );
        assert_ne!(
            signal_lines("let idx = KernelIndex::new();\n", "a.rs"),
            [] as [u32; 0]
        );
        assert_ne!(
            signal_lines("let b = WGPU_BACKEND;\n", "a.rs"),
            [] as [u32; 0]
        );
        assert_ne!(
            signal_lines("threadgroup_size(64);\n", "a.rs"),
            [] as [u32; 0]
        );
    }

    #[test]
    fn signal_lines_carry_correct_one_based_numbers() {
        let src = "let a = 1;\nlet b = workgroup(2);\nlet c = 3;\nlet d = wgsl_path;\n";
        let lines = signal_lines(src, "a.rs");
        assert_eq!(lines, vec![2, 4]);
    }

    #[test]
    fn wgsl_paths_always_signal_even_empty() {
        assert_eq!(signal_lines("", "grid.wgsl"), vec![1]);
    }

    #[test]
    fn span_scope_window_boundaries() {
        let signals = vec![100u32];
        // window ± 24: [76, 124] allowed on both ends.
        assert!(span_in_scope(76, 100, &signals));
        assert!(span_in_scope(100, 124, &signals));
        assert!(span_in_scope(90, 110, &signals));
        // Just outside: 50..75 -> hi = 99 < 100; 125..130 -> lo = 101 > 100.
        assert!(!span_in_scope(50, 75, &signals));
        assert!(!span_in_scope(125, 130, &signals));
        // Empty signal set denies everything.
        assert!(!span_in_scope(1, 1000, &[]));
    }

    #[test]
    fn plain_rust_file_does_not_signal() {
        let src = "fn main() { println!(\"hello\"); }\nstruct S { a: u32 }\n";
        assert_eq!(signal_lines(src, "main.rs"), [] as [u32; 0]);
    }

    #[test]
    fn off_plan_allows_every_span() {
        let plan = KernelPlan::Off;
        assert!(plan.span_allowed(1, usize::MAX));
    }
}
