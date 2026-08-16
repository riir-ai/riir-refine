//! Real-`cargo clippy` labeling oracle — Plan 005 T3.5.5.
//!
//! ## Why this exists
//!
//! T3.5.4 built an instantiation re-ranker that appeared to lift clippy top-1
//! from 70.8% to 95.8%. The number was an artifact:
//! `clippy_fixture_labels_verify_via_proposers` asserts that every clippy
//! fixture fires its OWN proposer and **zero** of the other 40 cross-fire —
//! exactly the property the re-ranker exploits. `rerank_circularity_diagnostic`
//! proved the contamination to the digit (`RerankMode::All` top-1 == pool
//! recall @8, 46/48). The fixture set simply **cannot measure a re-ranker**:
//! the oracle that assigned the labels is the oracle doing the ranking.
//!
//! This module supplies an oracle with no such relationship. It asks real
//! `cargo clippy` what fires on a span and reports the answer verbatim.
//! `cargo clippy` knows nothing about our corpus, our proposers, or our
//! retrieval scores, so a G1 measured against it is a real measurement.
//!
//! ## What it fixes about the old oracle
//!
//! | | proposer oracle | this oracle |
//! |---|---|---|
//! | label source | our own `propose_fix` | rustc + clippy |
//! | cross-fire | asserted to be ZERO | whatever is actually true |
//! | compiles? | never checked (`syn`-only) | checked — errors are reported |
//!
//! The zero-cross-fire property is the artificial part. Real code trips
//! several lints at once, and a span where multiple rules genuinely co-apply
//! is precisely where a re-ranker either earns its place or does not.
//!
//! ## Cost + determinism
//!
//! One `cargo clippy` per distinct span, on a BLAKE3-keyed temp crate so
//! repeat runs reuse cargo's build cache. Deterministic for a fixed toolchain
//! — but NOT across toolchains, since clippy's lint set moves between
//! releases. That is why this is a feature-gated measurement harness and not
//! a committed regression floor: pinning a floor to it would break the suite
//! on every rustup.
//!
//! Modelless: clippy is a deterministic rules engine. No weights, no training.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// What real clippy reports about one span.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OracleVerdict {
    /// Whether cargo actually ran (false → cargo missing or I/O failure).
    pub ran: bool,
    /// Whether the span compiled. A span that does not compile yields NO
    /// usable labels — clippy emits errors instead of lints — and must be
    /// excluded from any measurement rather than counted as "no lints".
    pub compiles: bool,
    /// Every `clippy::*` lint clippy reported, deduplicated and sorted.
    /// Sorted so the verdict is comparable and printable without re-sorting.
    pub lints: BTreeSet<String>,
    /// Diagnostic when `ran` is false, or the first hard error when the span
    /// does not compile.
    pub error: Option<String>,
}

impl OracleVerdict {
    fn did_not_run(err: impl Into<String>) -> Self {
        Self { ran: false, compiles: false, lints: BTreeSet::new(), error: Some(err.into()) }
    }

    /// The subset of reported lints that our corpus actually knows about.
    ///
    /// Clippy with `pedantic`/`nursery` on reports plenty we have no rule for
    /// (`must_use_candidate`, `missing_panics_doc`, …). Those are not wrong,
    /// they are simply outside the retrieval vocabulary, so scoring against
    /// them would measure corpus coverage rather than ranking quality.
    #[must_use]
    pub fn within_vocabulary<'a>(&'a self, vocabulary: &'a BTreeSet<String>) -> BTreeSet<String> {
        self.lints.intersection(vocabulary).cloned().collect()
    }
}

/// Asks real `cargo clippy` which lints fire on a span.
pub struct ClippyOracle {
    cargo_bin: String,
    work_root: PathBuf,
}

impl Default for ClippyOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl ClippyOracle {
    /// System cargo, temp work root.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cargo_bin: "cargo".to_string(),
            // Distinct from ClippyVerifier's root so a concurrent
            // self_evolve run and this harness never share a crate dir.
            work_root: std::env::temp_dir().join("riir_clippy_oracle"),
        }
    }

    /// Override the temp crate root (sandboxing, or isolating parallel runs).
    #[must_use]
    pub fn with_work_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.work_root = root.into();
        self
    }

    /// Is cargo reachable? Callers should SKIP rather than fail when false.
    #[must_use]
    pub fn is_available(&self) -> bool {
        Command::new(&self.cargo_bin).arg("--version").output().is_ok()
    }

    /// Ask clippy what fires on `code`.
    #[must_use]
    pub fn lints_for(&self, code: &str) -> OracleVerdict {
        let crate_dir = match self.prepare_crate(code) {
            Ok(dir) => dir,
            Err(e) => return OracleVerdict::did_not_run(format!("temp crate setup failed: {e}")),
        };
        let stdout = match self.run_clippy(&crate_dir) {
            Ok(o) => o,
            Err(e) => return OracleVerdict::did_not_run(e),
        };
        parse_verdict(&stdout)
    }

    /// BLAKE3-keyed temp crate so repeat calls reuse cargo's build cache.
    fn prepare_crate(&self, code: &str) -> Result<PathBuf, String> {
        let hex = blake3::hash(code.as_bytes()).to_hex();
        let dir = self.work_root.join(&hex[..16]);
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;
        let cargo_toml = "[package]\n\
            name = \"clippy_oracle\"\n\
            version = \"0.0.0\"\n\
            edition = \"2021\"\n\
            \n\
            [lib]\n\
            path = \"src/lib.rs\"\n";
        std::fs::write(dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("write Cargo.toml failed: {e}"))?;
        std::fs::write(src_dir.join("lib.rs"), code)
            .map_err(|e| format!("write lib.rs failed: {e}"))?;
        Ok(dir)
    }

    /// `cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery`
    ///
    /// All three groups, because the corpus spans style/complexity/perf AND
    /// nursery/pedantic rules; enabling only `all` would silently label some
    /// fixtures "no lint" for the wrong reason. Over-reporting is handled by
    /// [`OracleVerdict::within_vocabulary`], under-reporting could not be.
    fn run_clippy(&self, crate_dir: &Path) -> Result<String, String> {
        let output = Command::new(&self.cargo_bin)
            .arg("clippy")
            .arg("--manifest-path")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--message-format=json")
            .arg("--")
            .arg("-W")
            .arg("clippy::all")
            .arg("-W")
            .arg("clippy::pedantic")
            .arg("-W")
            .arg("clippy::nursery")
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Parse cargo's JSON stream into a verdict.
///
/// Split out from the runner so it is unit-testable without spawning cargo.
#[must_use]
pub fn parse_verdict(json_output: &str) -> OracleVerdict {
    let mut lints = BTreeSet::new();
    let mut first_error: Option<String> = None;

    for line in json_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = v.get("message") else { continue };

        // A hard error means the span never type-checked, so any lints on it
        // are unreliable and the span must be excluded, not scored.
        if message.get("level").and_then(serde_json::Value::as_str) == Some("error") {
            if first_error.is_none() {
                first_error = message
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
                    .or_else(|| Some("unknown compile error".to_string()));
            }
            continue;
        }

        if let Some(code) = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(serde_json::Value::as_str)
        {
            if let Some(bare) = code.strip_prefix("clippy::") {
                lints.insert(bare.to_string());
            }
        }
    }

    OracleVerdict {
        ran: true,
        compiles: first_error.is_none(),
        lints,
        error: first_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(level: &str, code: Option<&str>, text: &str) -> String {
        let code_json = code.map_or("null".to_string(), |c| format!(r#"{{"code":"{c}"}}"#));
        format!(
            r#"{{"reason":"compiler-message","message":{{"level":"{level}","code":{code_json},"message":"{text}"}}}}"#
        )
    }

    #[test]
    fn extracts_clippy_lints_and_strips_the_prefix() {
        let out = [
            msg("warning", Some("clippy::needless_return"), "x"),
            msg("warning", Some("clippy::redundant_closure"), "y"),
        ]
        .join("\n");
        let v = parse_verdict(&out);
        assert!(v.ran && v.compiles);
        assert_eq!(
            v.lints,
            ["needless_return".to_string(), "redundant_closure".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn non_clippy_diagnostics_are_ignored() {
        // rustc's own lints are not corpus rules and must not become labels.
        let out = msg("warning", Some("unused_variables"), "x");
        assert!(parse_verdict(&out).lints.is_empty());
    }

    #[test]
    fn a_hard_error_marks_the_span_uncompilable() {
        let out = [
            msg("error", None, "cannot find type `User` in this scope"),
            msg("warning", Some("clippy::needless_return"), "x"),
        ]
        .join("\n");
        let v = parse_verdict(&out);
        assert!(v.ran);
        assert!(!v.compiles, "an error-level message must clear `compiles`");
        assert!(v.error.as_deref().is_some_and(|e| e.contains("User")));
    }

    #[test]
    fn non_json_and_non_message_lines_are_skipped() {
        let out = format!(
            "not json at all\n{}\n{}",
            r#"{"reason":"compiler-artifact","target":{}}"#,
            msg("warning", Some("clippy::single_match"), "z")
        );
        let v = parse_verdict(&out);
        assert_eq!(v.lints, ["single_match".to_string()].into_iter().collect());
    }

    #[test]
    fn vocabulary_filter_keeps_only_known_rules() {
        let v = parse_verdict(
            &[
                msg("warning", Some("clippy::needless_return"), "x"),
                msg("warning", Some("clippy::must_use_candidate"), "y"),
            ]
            .join("\n"),
        );
        let vocab: BTreeSet<String> = ["needless_return".to_string()].into_iter().collect();
        assert_eq!(
            v.within_vocabulary(&vocab),
            ["needless_return".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn empty_output_is_a_clean_compiling_span() {
        let v = parse_verdict("");
        assert!(v.ran && v.compiles && v.lints.is_empty());
    }
}
