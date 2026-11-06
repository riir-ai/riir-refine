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
//! **PROMOTED to default (owner call 2026-11-06, "any gain no perf hurt
//! must enabled by default"):** `kernel_pregate` is a default feature
//! (implies `kernel_opt`) — every scan auto-detects kernel-bearing files
//! and the kernel domain joins the fan-out on signaling files only, at the
//! Bench-137 posture (SCOPE + `LINE_CAP=2` + `QUERY_TEXT_CAP=2048` baked as
//! the in-code defaults). The three envs are now DEMOTE-ONLY (the exact
//! literal `0` restores that axis's pre-promotion behavior);
//! `--no-default-features` still compiles the gate away (the tested
//! flag-OFF posture), and the explicit `--domain kernel` bypass still
//! wins. The wiring lives in
//! `draft::latent_matcher::suggest_file_inner`; the scan path's per-span
//! verdict rides one bool, every other domain untouched.

/// The span-scope window (pinned in Bench 131 BEFORE any timing run): a span
/// is in scope when its line range intersects any signal line ± this many
/// lines. Widening it is a NEW pre-registration, never an in-place amendment.
pub const SPAN_SCOPE_WINDOW_LINES: usize = 24;

/// The scope-arm switch (Bench 131's GATE+SCOPE arm). PROMOTED default-on
/// 2026-11-06 (owner call — the Bench-137 posture ships as the default):
/// unset = armed; the exact demote literal `0` restores the GATE-only
/// incumbent byte-identically (env-switch law). Read once per process.
const SCOPE_ENV: &str = "RIIR_KERNEL_PREGATE_SCOPE";

/// The line-cap switch (Bench 134's CAP arms): promote-only env, the exact
/// pinned literals `4`/`2` — see [`line_cap`].
const LINE_CAP_ENV: &str = "RIIR_KERNEL_PREGATE_LINE_CAP";

/// The per-query TEXT budget (Bench 136, issue 208 T7): promote-only env,
/// pinned literals `4096`/`2048`/`1024` — see [`query_text_cap`].
const QUERY_TEXT_CAP_ENV: &str = "RIIR_KERNEL_PREGATE_QUERY_TEXT_CAP";

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
    *ARMED.get_or_init(|| match std::env::var(SCOPE_ENV).as_deref() {
        // Demote literal only; unset AND junk both ride the promoted default
        // (the arm is the default now — junk env is unset-equivalent, never a
        // demote).
        Ok("0") => false,
        _ => true,
    })
}

/// The per-span line-query cap (Bench 134, issue 208 T6). PROMOTED
/// default-on 2026-11-06 (owner call) at the Bench-137 measured value:
/// unset/junk = `Some(2)`; the exact demote literal `0` restores the
/// uncapped pre-cap behavior byte-identically; `4` remains an accepted
/// tuning literal (the Bench-134 pin). Read once per process.
#[must_use]
pub fn line_cap() -> Option<usize> {
    static CAP: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| match std::env::var(LINE_CAP_ENV).as_deref() {
        Ok("0") => None,
        Ok("4") => Some(4),
        _ => Some(2),
    })
}

/// The per-query TEXT budget (Bench 136, issue 208 T7). PROMOTED
/// default-on 2026-11-06 (owner call) at the Bench-137 measured value:
/// unset/junk = `Some(2048)`; the exact demote literal `0` restores the
/// no-truncation pre-cap behavior byte-identically; `4096`/`1024` remain
/// accepted tuning literals (the Bench-136 pins). When armed, EVERY query
/// text in the kernel index's capped suggest path (the whole-span query
/// AND every per-line query) is head-truncated to at most B bytes — the
/// Bench-135 finding is that the residual G5 cost is query-COUNT-invariant,
/// i.e. carried by the text mass of the whole-span query over giant spans.
#[must_use]
pub fn query_text_cap() -> Option<usize> {
    static CAP: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| match std::env::var(QUERY_TEXT_CAP_ENV).as_deref() {
        Ok("0") => None,
        Ok("4096") => Some(4096),
        Ok("1024") => Some(1024),
        _ => Some(2048),
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
