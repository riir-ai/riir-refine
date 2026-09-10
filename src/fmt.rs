//! Native rustfmt integration for `cargo heal --fix --write` — default-on,
//! `--no-fmt` opt-out. Encodes the heal-slice formatting discipline as tool
//! behavior so every consumer gets it without a SKILL:
//!
//! 1. **fmt-clean-at-HEAD gate** — rustfmt runs on a healed file ONLY if the
//!    original (pre-heal) content was already rustfmt-clean. Files with
//!    pre-existing drift stay unfmt'd so a mechanical heal never drags
//!    unrelated formatting churn into the diff (the slice-2 rule).
//! 2. **CRLF skip** — rustfmt on a CRLF file converts every line ending,
//!    exploding the diff to the whole file (the slice-13 trap). Reported,
//!    never converted.
//! 3. **Edition from the owning Cargo.toml** — bare-file rustfmt defaults to
//!    a pre-2024 edition and hard-fails on edition-2024 let-chains. Defaults
//!    to "2024" when no manifest is found.
//! 4. rustfmt missing / parse failure → warning + skip, never fails the heal
//!    (the healed file is still valid Rust, just not re-formatted).
//!
//! Dry runs (`--fix` without `--write`) report the per-file fmt plan
//! (`would-format` / `skip (pre-existing drift)` / `skip (CRLF)`) plus a
//! summary so the fmt surface is visible before anything is written.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Per-file outcome of the fmt-clean-at-HEAD gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmtDecision {
    /// Original was rustfmt-clean → the healed file gets rustfmt'd after write.
    Format,
    /// Original had pre-existing rustfmt drift → healed file stays unfmt'd
    /// (only the healed lines change; no formatting churn enters the diff).
    SkipDrift,
    /// CRLF line endings → rustfmt would convert every EOL (whole-file diff).
    SkipCrlf,
}

/// Running fmt counters for the end-of-run summary.
#[derive(Default)]
pub struct FmtSummary {
    pub formatted: usize,
    pub skip_drift: usize,
    pub skip_crlf: usize,
    /// rustfmt ran but failed (e.g. parse error on the healed output).
    pub failed: usize,
    /// Dry-run mode: decisions were counted but nothing written — the
    /// summary says "would-format" instead of "formatted".
    pub dry: bool,
}

impl FmtSummary {
    #[must_use]
    pub fn total(&self) -> usize {
        self.formatted + self.skip_drift + self.skip_crlf + self.failed
    }

    /// One-line summary, e.g. `fmt: 22 formatted, 70 skip pre-existing
    /// drift, 3 skip CRLF` (dry runs say `would-format`). Empty when
    /// nothing was decided.
    #[must_use]
    pub fn line(&self) -> String {
        if self.total() == 0 {
            return String::new();
        }
        let verb = if self.dry {
            "would-format"
        } else {
            "formatted"
        };
        let mut parts = vec![format!("{} {}", self.formatted, verb)];
        if self.skip_drift > 0 {
            parts.push(format!("{} skip pre-existing drift", self.skip_drift));
        }
        if self.skip_crlf > 0 {
            parts.push(format!("{} skip CRLF", self.skip_crlf));
        }
        if self.failed > 0 {
            parts.push(format!("{} FAILED", self.failed));
        }
        format!("fmt: {}", parts.join(", "))
    }
}

/// Is a rustfmt binary reachable? Checked once per process.
#[must_use]
pub fn rustfmt_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("rustfmt")
            .arg("--version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// Decide the fmt plan for one healed file. `src` is the ORIGINAL
/// (pre-heal) text; `file` must be the on-disk pre-heal file (rustfmt
/// `--check` reads it — this also picks up any rustfmt.toml by path).
#[must_use]
pub fn decide(src: &str, file: &Path, edition: &str) -> FmtDecision {
    if src.contains("\r\n") {
        return FmtDecision::SkipCrlf;
    }
    let clean = Command::new("rustfmt")
        .args(["--check", "--edition", edition])
        .arg(file)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if clean {
        FmtDecision::Format
    } else {
        FmtDecision::SkipDrift
    }
}

/// rustfmt a healed file in place. Err carries the stderr head for the
/// failure report (the healed file is left as-written — still valid Rust).
///
/// # Errors
///
/// Err when rustfmt is missing (`spawn failed`) or exits non-zero (e.g. a
/// parse error on the healed output — the file is left as-written).
pub fn apply(file: &Path, edition: &str) -> Result<(), String> {
    let out = Command::new("rustfmt")
        .args(["--edition", edition])
        .arg(file)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("spawn failed: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let head = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    Err(if head.is_empty() {
        format!("exit {}", out.status)
    } else {
        head.chars().take(120).collect()
    })
}

/// Resolve the rustfmt edition for a file from its owning package's
/// Cargo.toml (`edition = "..."`). Walks up like `owning_package_root`
/// (absolute-path normalized — the Issue 015 relative-path lesson), default
/// `"2024"` (the workspace standard + cargo's default for new crates).
#[must_use]
pub fn edition_for(file: &Path) -> String {
    package_root(file)
        .and_then(|root| std::fs::read_to_string(root.join("Cargo.toml")).ok())
        .and_then(|manifest| {
            manifest.lines().find_map(|l| {
                let l = l.trim();
                l.strip_prefix("edition").and_then(|rest| {
                    let v = rest.trim_start().strip_prefix('=').unwrap_or(rest).trim();
                    let v = v.trim_matches('"');
                    (!v.is_empty() && v.chars().all(|c| c.is_ascii_digit())).then(|| v.to_string())
                })
            })
        })
        .unwrap_or_else(|| "2024".to_string())
}

fn package_root(file: &Path) -> Option<PathBuf> {
    let file = std::path::absolute(file).ok()?;
    let mut dir = file.parent()?.to_path_buf();
    loop {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each test gets its own unique subdirectory under `temp_dir` (created),
    /// so a parallel sibling test's cleanup can never remove a shared parent
    /// and this terminal's per-session TMPDIR teardown can't strand files.
    fn temp_file(name: &str, content: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("riir_clippy_fmt_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    fn cleanup(p: &Path) {
        // Remove the file's whole unique per-test directory — never a
        // shared parent (the parallel-cleanup hazard this shape prevents).
        if let Some(dir) = p.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn crlf_short_circuits_before_rustfmt() {
        // CRLF is detected on the text alone — no rustfmt needed.
        let p = temp_file("crlf.rs", "fn main() {\r\n    println!(\"x\");\r\n}\r\n");
        assert_eq!(
            decide("fn main() {\r\n}\r\n", &p, "2024"),
            FmtDecision::SkipCrlf
        );
        cleanup(&p);
    }

    #[test]
    fn clean_original_decides_format() {
        if !rustfmt_available() {
            return; // no rustfmt on PATH — decision layer untestable here
        }
        let p = temp_file("clean.rs", "fn main() {\n    println!(\"hi\");\n}\n");
        let src = std::fs::read_to_string(&p).unwrap();
        assert_eq!(decide(&src, &p, "2024"), FmtDecision::Format);
        cleanup(&p);
    }

    #[test]
    fn drifted_original_decides_skip_drift() {
        if !rustfmt_available() {
            return;
        }
        let p = temp_file("drift.rs", "fn main() {\nprintln!(\"bad indent\");\n}\n");
        let src = std::fs::read_to_string(&p).unwrap();
        assert_eq!(decide(&src, &p, "2024"), FmtDecision::SkipDrift);
        cleanup(&p);
    }

    #[test]
    fn apply_formats_in_place() {
        if !rustfmt_available() {
            return;
        }
        let p = temp_file("apply.rs", "fn main() {\nprintln!(\"x\");\n}\n");
        assert!(apply(&p, "2024").is_ok());
        let after = std::fs::read_to_string(&p).unwrap();
        assert!(
            after.contains("    println!(\"x\");"),
            "rustfmt should have re-indented: {after}"
        );
        cleanup(&p);
    }

    #[test]
    fn edition_from_owning_manifest() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("riir_clippy_fmt_ed_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let f = dir.join("src.rs");
        std::fs::write(&f, "fn a() {}\n").unwrap();
        assert_eq!(edition_for(&f), "2021");
        // No manifest above the unique temp root → default 2024.
        let lone_dir =
            std::env::temp_dir().join(format!("riir_clippy_fmt_lone_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&lone_dir).unwrap();
        let lone = lone_dir.join("lone.rs");
        std::fs::write(&lone, "fn a() {}\n").unwrap();
        assert_eq!(edition_for(&lone), "2024");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&lone_dir);
    }

    #[test]
    fn summary_line_counts() {
        let mut s = FmtSummary::default();
        assert_eq!(s.line(), "");
        s.formatted = 2;
        s.skip_drift = 3;
        s.skip_crlf = 1;
        let line = s.line();
        assert!(line.contains("2 formatted"), "{line}");
        assert!(!line.contains("would-format"), "{line}");
        s.dry = true;
        let line = s.line();
        assert!(line.contains("2 would-format"), "{line}");
        assert!(line.contains("3 skip pre-existing drift"), "{line}");
        assert!(line.contains("1 skip CRLF"), "{line}");
        assert_eq!(s.total(), 6);
    }
}
