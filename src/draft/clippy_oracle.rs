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
use std::fmt::Write as _;
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
        Self {
            ran: false,
            compiles: false,
            lints: BTreeSet::new(),
            error: Some(err.into()),
        }
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

/// The synthetic probe crate manifest, shared by the single-snippet and
/// batch paths — one source so the two instruments never drift on the
/// crate shape they measure against.
const ORACLE_CARGO_TOML: &str = "[package]\n\
    name = \"clippy_oracle\"\n\
    version = \"0.0.0\"\n\
    edition = \"2021\"\n\
    \n\
    [lib]\n\
    path = \"src/lib.rs\"\n";

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
        // [documented-fixed-root] — BLAKE3-keyed crate dirs, each with its own
        // `target/` (the child DROPS CARGO_TARGET_DIR); a pid suffix would kill
        // the cross-run cargo cache (see clippy_lints::verify::ClippyVerifier;
        // katgpt-rs Issue 832 T5).
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
        Command::new(&self.cargo_bin)
            .arg("--version")
            .output()
            .is_ok()
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

    /// Label every snippet in ONE `cargo clippy` invocation — the Plan 049
    /// T0.0b batch path. Each snippet becomes its own module (`s0.rs`,
    /// `s1.rs`, …) of a single BLAKE3-keyed temp crate; clippy runs once and
    /// every diagnostic is attributed back to its snippet by the primary
    /// span's file. Returns one verdict per snippet, input order.
    ///
    /// Attribution + honesty contract:
    /// - the batch is trusted ONLY when the whole probe crate compiled
    ///   (`success &&` every module error-free). The measured reason: a
    ///   hard error ANYWHERE suppresses rustc's late lint passes
    ///   crate-wide, so the surviving modules' warnings are never emitted
    ///   — attributing them would silently label clean snippets as
    ///   "no lints" (the exact mislabeling this oracle exists to
    ///   prevent; caught live by the agreement gate).
    /// - any batch that fails that test — an unattributable error, a link
    ///   failure, or one broken snippet — falls back to per-snippet
    ///   [`Self::lints_for`] runs WHOLESALE, so every verdict always keeps
    ///   the single-snippet contract. Broken spans are the minority in a
    ///   labeling workload, so the one-invocation win still applies to the
    ///   common all-clean batch.
    /// - a snippet with no attributed diagnostics reads `compiles == true`
    ///   and no lints — a genuinely clean span (or, under cargo's
    ///   fresh-unit diagnostic replay, the replayed verdict of a
    ///   previously-run identical batch — the same mechanism
    ///   [`Self::lints_for`] relies on).
    ///
    /// Cross-module caveat: lints are scope-sensitive (dead-code analysis
    /// sees sibling modules), so a batch verdict can differ from the
    /// single-snippet verdict at the margins. The committed test pins
    /// agreement on clean snippets; measurement code should label a dataset
    /// with ONE of the two instruments, never mix.
    pub fn lints_for_batch(&self, snippets: &[&str]) -> Vec<OracleVerdict> {
        if snippets.is_empty() {
            return Vec::new();
        }
        let module_files: Vec<String> = (0..snippets.len())
            .map(|i| format!("src/s{i}.rs"))
            .collect();
        let module_refs: Vec<&str> = module_files.iter().map(String::as_str).collect();

        let crate_dir = match self.prepare_batch_crate(snippets) {
            Ok(dir) => dir,
            Err(e) => {
                return vec![
                    OracleVerdict::did_not_run(format!("temp crate setup failed: {e}"));
                    snippets.len()
                ];
            }
        };
        let (stdout, success) = match self.run_clippy_status(&crate_dir) {
            Ok(x) => x,
            Err(e) => return vec![OracleVerdict::did_not_run(e); snippets.len()],
        };

        match parse_batch_verdicts(&stdout, &module_refs) {
            // Trust the attribution only for an ALL-CLEAN build: any hard
            // error suppresses rustc's late lints crate-wide, so a mixed
            // batch's clean modules were never labeled and must not read
            // as silently-clean.
            BatchParse::Attributed(v) if success && v.iter().all(|x| x.compiles) => v,
            // Unattributable error, link failure, or one broken snippet:
            // fall back wholesale so every verdict keeps the single-snippet
            // contract.
            _ => snippets.iter().map(|c| self.lints_for(c)).collect(),
        }
    }

    /// Compile-probe a batch: ONE `cargo check` over the shared batch
    /// crate, hard errors attributed to their module by primary-span file.
    ///
    /// Plan 049 T0.1: real-repo spans carry a substantial non-compiling
    /// fraction (crate-internal references, edition drift), and
    /// [`Self::lints_for_batch`] answers any batch containing ONE broken
    /// snippet with the per-snippet fallback — ~1 cargo spawn per span.
    /// Probing first lets a caller clip the broken spans and label an
    /// all-compiling batch, which takes the attributed fast path (one
    /// spawn per whole batch).
    ///
    /// Returns one flag per snippet, input order. A module with no
    /// attributed error reads as compiling — the same contract as a
    /// single-snippet [`Self::lints_for`].`compiles`. An unattributable
    /// error (primary span outside the module set — must not exist in a
    /// crate this module fully controls) falls back to per-snippet
    /// single-run probes, matching [`Self::lints_for_batch`]'s honesty
    /// rule: unmeasurable is not compiling.
    pub fn compiles_batch(&self, snippets: &[&str]) -> Vec<bool> {
        if snippets.is_empty() {
            return Vec::new();
        }
        let module_files: Vec<String> = (0..snippets.len())
            .map(|i| format!("src/s{i}.rs"))
            .collect();
        let module_refs: Vec<&str> = module_files.iter().map(String::as_str).collect();

        let Ok(crate_dir) = self.prepare_batch_crate_in("probe_", snippets) else {
            return vec![false; snippets.len()];
        };
        // Same rule as run_clippy_status: the inherited CARGO_TARGET_DIR
        // routes same-name temp packages into one target dir and crosses
        // their diagnostic replay.
        let Ok(output) = Command::new(&self.cargo_bin)
            .arg("check")
            .arg("--manifest-path")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--message-format=json")
            .env_remove("CARGO_TARGET_DIR")
            .output()
        else {
            return vec![false; snippets.len()];
        };
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

        match parse_batch_verdicts(&stdout, &module_refs) {
            BatchParse::Attributed(v) => v.into_iter().map(|x| x.compiles).collect(),
            // The crate shape is fully controlled, so this path should be
            // dead; when it fires anyway, per-snippet single-run probes are
            // the honest fallback (same rule as lints_for_batch).
            BatchParse::Unattributable(_) => snippets
                .iter()
                .map(|c| self.lints_for(c).compiles)
                .collect(),
        }
    }

    /// BLAKE3-keyed temp crate so repeat calls reuse cargo's build cache.
    /// Scaffold writes are read-before-write (`write_probe_source`): an
    /// unchanged repeat must not bump mtimes, or that cache hit stops being
    /// one — and concurrent same-key processes then write nothing at all.
    fn prepare_crate(&self, code: &str) -> Result<PathBuf, String> {
        let hex = blake3::hash(code.as_bytes()).to_hex();
        let dir = self.work_root.join(&hex[..16]);
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;
        crate::domains::probe_witness::write_probe_source(&dir.join("Cargo.toml"), ORACLE_CARGO_TOML)?;
        crate::domains::probe_witness::write_probe_source(&src_dir.join("lib.rs"), code)?;
        Ok(dir)
    }

    /// BLAKE3-keyed BATCH temp crate: one module per snippet so repeat runs
    /// of the SAME snippet sequence reuse cargo's build cache. The key
    /// hashes each snippet LENGTH-PREFIXED (the incremental multi-part hash
    /// — no concatenation copy, and no `a‖b` vs `a‖b‖c` ambiguity).
    /// `prefix` namespaces the crate dir: the compile PROBE and the clippy
    /// LABEL run must never share a dir, because cargo's diagnostic-cache
    /// replay on a target dir that a previous `cargo check` populated
    /// SUPPRESSES the clippy warnings (measured, Plan 049 T0.1: a
    /// check-then-clippy sequence emitted ~zero lints on 200 spans that
    /// fire readily on a fresh dir).
    fn prepare_batch_crate(&self, snippets: &[&str]) -> Result<PathBuf, String> {
        self.prepare_batch_crate_in("batch_", snippets)
    }

    fn prepare_batch_crate_in(&self, prefix: &str, snippets: &[&str]) -> Result<PathBuf, String> {
        let mut hasher = blake3::Hasher::new();
        for s in snippets {
            hasher.update(&(s.len() as u64).to_le_bytes());
            hasher.update(s.as_bytes());
        }
        let hex = hasher.finalize().to_hex();
        let dir = self.work_root.join(format!("{prefix}{}", &hex[..16]));
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;
        // Read-before-write, same contract as `prepare_crate` above.
        crate::domains::probe_witness::write_probe_source(&dir.join("Cargo.toml"), ORACLE_CARGO_TOML)?;
        let mut lib = String::new();
        for (i, s) in snippets.iter().enumerate() {
            let _ = writeln!(lib, "pub mod s{i};");
            crate::domains::probe_witness::write_probe_source(&src_dir.join(format!("s{i}.rs")), s)?;
        }
        crate::domains::probe_witness::write_probe_source(&src_dir.join("lib.rs"), &lib)?;
        Ok(dir)
    }

    /// `cargo clippy --message-format=json -- -W clippy::all -W clippy::pedantic -W clippy::nursery`
    ///
    /// All three groups, because the corpus spans style/complexity/perf AND
    /// nursery/pedantic rules; enabling only `all` would silently label some
    /// fixtures "no lint" for the wrong reason. Over-reporting is handled by
    /// [`OracleVerdict::within_vocabulary`], under-reporting could not be.
    fn run_clippy(&self, crate_dir: &Path) -> Result<String, String> {
        self.run_clippy_status(crate_dir).map(|(out, _)| out)
    }

    /// [`run_clippy`] plus the exit status — the batch path needs it to
    /// catch failures that produce no attributable diagnostic (link
    /// errors, a wedged rustc).
    ///
    /// The child env DROPS `CARGO_TARGET_DIR`: every keyed temp crate is a
    /// package named `clippy_oracle`, and routing them all into one
    /// inherited target dir cross-contaminates cargo's fingerprint/diagnostic
    /// replay across DIFFERENT contents (measured: batch vs single label
    /// divergence, T0.1 session). Each `crate_dir` therefore gets its own
    /// `target/` beside it.
    fn run_clippy_status(&self, crate_dir: &Path) -> Result<(String, bool), String> {
        let output = Command::new(&self.cargo_bin)
            .arg("clippy")
            .arg("--manifest-path")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--message-format=json")
            .env_remove("CARGO_TARGET_DIR")
            .arg("--")
            .arg("-W")
            .arg("clippy::all")
            .arg("-W")
            .arg("clippy::pedantic")
            .arg("-W")
            .arg("clippy::nursery")
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;
        Ok((
            String::from_utf8_lossy(&output.stdout).into_owned(),
            output.status.success(),
        ))
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
        let Some(message) = v.get("message") else {
            continue;
        };

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

/// The outcome of attributing one batch clippy stream to its modules.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BatchParse {
    /// One verdict per module, input order.
    Attributed(Vec<OracleVerdict>),
    /// A hard error pointed outside the module set — the batch cannot be
    /// honestly attributed and the caller must fall back to per-snippet
    /// runs.
    Unattributable(String),
}

/// Which module (index into `module_files`) does a rustc JSON `file_name`
/// belong to? Matched COMPONENT-wise on the path tail (`src/s3.rs` matches
/// the absolute `…/batch_x/src/s3.rs`), never substring-wise — a
/// 14-snippet batch has `s1` AND `s13`, and `"src/s13.rs"` must not match
/// `"src/s3.rs"` the way a suffix test would.
fn module_of_file(file_name: &str, module_files: &[&str]) -> Option<usize> {
    let p = Path::new(file_name);
    module_files.iter().position(|m| p.ends_with(Path::new(m)))
}

/// The primary-span file of a rustc JSON diagnostic, if any: the first
/// `is_primary: true` span, else the first span.
fn primary_span_file(message: &serde_json::Value) -> Option<&str> {
    let spans = message.get("spans")?.as_array()?;
    let primary = spans
        .iter()
        .find(|s| s.get("is_primary").and_then(serde_json::Value::as_bool) == Some(true));
    primary
        .or_else(|| spans.first())?
        .get("file_name")
        .and_then(serde_json::Value::as_str)
}

/// Attribute a batch clippy JSON stream to per-module verdicts.
///
/// Exit-status-blind by design — the caller (which spawned cargo) owns the
/// success flag; this fn only attributes what the stream says. An
/// unattributable WARNING is dropped (in a crate we fully control — empty
/// `lib.rs` plus one module per snippet — it should not exist, and one
/// stray warning must not poison N verdicts); an unattributable ERROR is
/// the fallback signal.
fn parse_batch_verdicts(json_output: &str, module_files: &[&str]) -> BatchParse {
    let mut lints: Vec<BTreeSet<String>> = vec![BTreeSet::new(); module_files.len()];
    let mut first_error: Vec<Option<String>> = vec![None; module_files.len()];

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
        let Some(message) = v.get("message") else {
            continue;
        };
        let is_error = message.get("level").and_then(serde_json::Value::as_str) == Some("error");
        let module = primary_span_file(message).and_then(|f| module_of_file(f, module_files));

        if is_error {
            let text = message
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown compile error")
                .to_string();
            match module {
                Some(i) => {
                    if first_error[i].is_none() {
                        first_error[i] = Some(text);
                    }
                }
                None => return BatchParse::Unattributable(text),
            }
            continue;
        }

        let Some(code) = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(bare) = code.strip_prefix("clippy::") else {
            continue;
        };
        if let Some(i) = module {
            lints[i].insert(bare.to_string());
        }
    }

    BatchParse::Attributed(
        lints
            .into_iter()
            .zip(first_error)
            .map(|(lints, error)| OracleVerdict {
                ran: true,
                compiles: error.is_none(),
                lints,
                error,
            })
            .collect(),
    )
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
            [
                "needless_return".to_string(),
                "redundant_closure".to_string()
            ]
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

    // ── batch attribution (parse-level, no cargo spawned) ──────────────

    /// A compiler-message JSON line carrying a primary span in `file`.
    fn msg_in(file: &str, level: &str, code: Option<&str>, text: &str) -> String {
        let code_json = code.map_or("null".to_string(), |c| format!(r#"{{"code":"{c}"}}"#));
        format!(
            r#"{{"reason":"compiler-message","message":{{"level":"{level}","code":{code_json},"message":"{text}","spans":[{{"file_name":"{file}","is_primary":true}}]}}}}"#
        )
    }

    #[test]
    fn batch_lints_attribute_to_their_own_module() {
        let out = [
            msg_in("src/s0.rs", "warning", Some("clippy::needless_return"), "x"),
            msg_in(
                "src/s1.rs",
                "warning",
                Some("clippy::redundant_closure"),
                "y",
            ),
        ]
        .join("\n");
        let mods = ["src/s0.rs", "src/s1.rs"];
        match parse_batch_verdicts(&out, &mods) {
            BatchParse::Attributed(v) => {
                assert!(v.iter().all(|x| x.ran && x.compiles));
                assert_eq!(
                    v[0].lints,
                    ["needless_return".to_string()].into_iter().collect()
                );
                assert_eq!(
                    v[1].lints,
                    ["redundant_closure".to_string()].into_iter().collect()
                );
            }
            BatchParse::Unattributable(t) => {
                panic!("expected attribution, got Unattributable({t:?})");
            }
        }
    }

    #[test]
    fn batch_error_marks_only_its_own_module_uncompilable() {
        let out = [
            msg_in("src/s0.rs", "warning", Some("clippy::needless_return"), "x"),
            msg_in(
                "src/s1.rs",
                "error",
                None,
                "cannot find value `boom` in this scope",
            ),
        ]
        .join("\n");
        let mods = ["src/s0.rs", "src/s1.rs"];
        match parse_batch_verdicts(&out, &mods) {
            BatchParse::Attributed(v) => {
                assert!(
                    v[0].compiles,
                    "a sibling's error must not poison a clean module"
                );
                assert!(!v[1].compiles);
                assert!(v[1].error.as_deref().is_some_and(|e| e.contains("boom")));
            }
            BatchParse::Unattributable(t) => {
                panic!("expected attribution, got Unattributable({t:?})");
            }
        }
    }

    #[test]
    fn batch_error_outside_the_module_set_signals_fallback() {
        // Crate-level errors land in lib.rs, which is OURS, not a
        // snippet's — those must trigger the per-snippet fallback, never
        // read as "the snippet failed".
        let out = msg_in("src/lib.rs", "error", None, "crate-level boom");
        let mods = ["src/s0.rs"];
        match parse_batch_verdicts(&out, &mods) {
            BatchParse::Unattributable(t) => assert!(t.contains("boom")),
            BatchParse::Attributed(_) => panic!("expected the fallback signal, got Attributed"),
        }
    }

    #[test]
    fn module_attribution_is_component_wise_not_substring_wise() {
        // A 14-snippet batch has s1 AND s13; a suffix match would land
        // s13's diagnostics in s1.
        let names: Vec<String> = (0..14).map(|i| format!("src/s{i}.rs")).collect();
        let mods: Vec<&str> = names.iter().map(String::as_str).collect();
        let out = msg_in(
            "src/s13.rs",
            "warning",
            Some("clippy::needless_return"),
            "x",
        );
        match parse_batch_verdicts(&out, &mods) {
            BatchParse::Attributed(v) => {
                assert_eq!(v[13].lints.len(), 1, "s13 keeps its own lint");
                assert!(
                    v[..13].iter().all(|x| x.lints.is_empty()),
                    "no suffix bleed into s0..s12"
                );
            }
            BatchParse::Unattributable(t) => {
                panic!("expected attribution, got Unattributable({t:?})");
            }
        }
    }

    #[test]
    fn absolute_span_paths_match_their_module() {
        // cargo runs rustc from the crate dir, so real spans carry relative
        // paths — but a path-sensitive match must survive absolute ones too.
        let out = msg_in(
            "/tmp/riir_clippy_oracle/batch_ab12/src/s2.rs",
            "warning",
            Some("clippy::unused_unit"),
            "x",
        );
        let mods = ["src/s0.rs", "src/s1.rs", "src/s2.rs"];
        match parse_batch_verdicts(&out, &mods) {
            BatchParse::Attributed(v) => {
                assert_eq!(v[2].lints.len(), 1);
                assert!(v[..2].iter().all(|x| x.lints.is_empty()));
            }
            BatchParse::Unattributable(t) => {
                panic!("expected attribution, got Unattributable({t:?})");
            }
        }
    }

    #[test]
    fn an_empty_batch_runs_nothing_and_returns_nothing() {
        let oracle = ClippyOracle::default();
        assert!(oracle.lints_for_batch(&[]).is_empty());
    }

    // ── batch vs single (real cargo; a clippy_verify build has cargo) ──

    #[test]
    fn batch_labels_agree_with_per_snippet_labels_on_an_all_clean_batch() {
        let oracle = ClippyOracle::new().with_work_root(
            std::env::temp_dir().join(format!("riir_clippy_oracle_batch_{}", std::process::id())),
        );
        if !oracle.is_available() {
            // The parse-level gates above carry the contract offline.
            return;
        }
        let clean_a = "fn a(x: i32) -> i32 { return x; }";
        let clean_c = "pub fn c(v: &[u8]) -> Option<u8> { v.iter().nth(0).copied() }";
        let snippets = [clean_a, clean_c];

        let batch = oracle.lints_for_batch(&snippets);
        assert_eq!(batch.len(), 2);
        let single: Vec<OracleVerdict> = snippets.iter().map(|c| oracle.lints_for(c)).collect();

        for (b, s) in batch.iter().zip(&single) {
            assert!(b.ran && s.ran && b.compiles && s.compiles);
        }
        // The attributed path must label clean snippets identically to the
        // single-snippet instrument — the contract that lets one dataset
        // use either path.
        assert!(
            !single[0].lints.is_empty(),
            "expected the fixture lint to fire"
        );
        assert_eq!(
            batch[0].lints, single[0].lints,
            "batch vs single label drift on a clean snippet"
        );
        assert_eq!(batch[1].lints, single[1].lints);
    }

    #[test]
    fn a_batch_with_a_broken_snippet_falls_back_wholesale() {
        // MEASURED (the first run of the agreement gate): a hard error
        // anywhere in the probe crate suppresses rustc's late lint passes
        // CRATE-WIDE — the clean sibling's needless_return was never
        // emitted and an attributed mixed batch would have mislabeled it
        // as clean. The policy is therefore: one broken snippet ⇒ the
        // whole batch falls back to per-snippet runs, and every verdict
        // must match its single-run twin exactly.
        let oracle = ClippyOracle::new().with_work_root(
            std::env::temp_dir().join(format!("riir_clippy_oracle_batch_{}", std::process::id())),
        );
        if !oracle.is_available() {
            return;
        }
        let clean_a = "fn a(x: i32) -> i32 { return x; }";
        let broken = "fn b() { let _: DoesNotExist = (); }";
        let snippets = [clean_a, broken];

        let batch = oracle.lints_for_batch(&snippets);
        let single: Vec<OracleVerdict> = snippets.iter().map(|c| oracle.lints_for(c)).collect();
        for (i, (b, s)) in batch.iter().zip(&single).enumerate() {
            assert_eq!(
                b, s,
                "snippet {i}: fallback verdict must equal the single run"
            );
        }
        assert!(
            !batch[1].compiles,
            "the broken snippet must read uncompilable"
        );
        assert!(
            batch[0].compiles,
            "the clean snippet must keep its real labels"
        );
    }

    #[test]
    fn compiles_batch_splits_a_mixed_batch_without_the_fallback_storm() {
        // Plan 049 T0.1's instrument: ONE check run must separate the
        // compiling spans from the broken ones — the property that lets the
        // dataset harness clip broken spans and label the clean remainder
        // in attributed batches instead of paying ~1 spawn per span.
        let oracle = ClippyOracle::new().with_work_root(
            std::env::temp_dir().join(format!("riir_clippy_oracle_batch_{}", std::process::id())),
        );
        if !oracle.is_available() {
            return;
        }
        let good_a = "pub fn a(x: i32) -> i32 { x + 1 }";
        let broken_b = "pub fn b() { let _: DoesNotExist = (); }";
        let good_c = "pub fn c(v: &[u8]) -> Option<u8> { v.iter().nth(0).copied() }";
        let broken_d = "pub fn d() -> String { format!(\"{}\", NonexistentType::field) }";

        let flags = oracle.compiles_batch(&[good_a, broken_b, good_c, broken_d]);
        assert_eq!(flags, vec![true, false, true, false]);

        // The T0.1 sequence contract: probe FIRST, label SECOND — the label
        // run must still see every lint the probe's check did not suppress.
        // (The probe and the label use separate keyed crate dirs; sharing a
        // dir made cargo's diagnostic replay swallow the clippy warnings.)
        let surviving = [good_a, good_c];
        let verdicts = oracle.lints_for_batch(&surviving);
        assert!(verdicts.iter().all(|v| v.ran && v.compiles));
        assert!(
            !verdicts[0].lints.is_empty(),
            "post-probe label run lost the warnings — probe/label crates share a dir?"
        );

        // Empty input stays a no-op (the offline contract).
        assert!(oracle.compiles_batch(&[]).is_empty());
    }

    #[test]
    fn prepare_repeats_do_not_touch_scaffold_mtimes() {
        // katgpt-rs Issue 832 T5: the scaffold is a pure function of the dir
        // key, so an unchanged repeat must write NOTHING — the old
        // unconditional rewrite bumped mtimes and defeated the very cargo
        // cache the BLAKE3 key exists to provide (the verify.rs
        // write_probe_source lesson, ported here). mtime equality is exact
        // under the guard: no write means no metadata change at all.
        let root =
            std::env::temp_dir().join(format!("riir_clippy_oracle_mtime_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let oracle = ClippyOracle::new().with_work_root(&root);
        let mtime = |p: &std::path::Path| {
            std::fs::metadata(p)
                .expect("scaffold file must exist")
                .modified()
                .unwrap()
        };

        let code = "pub fn oracle_mtime_probe(x: i32) -> i32 { x }";
        let dir = oracle.prepare_crate(code).expect("prepare");
        let (lib_t, toml_t) = (mtime(&dir.join("src/lib.rs")), mtime(&dir.join("Cargo.toml")));
        let dir2 = oracle.prepare_crate(code).expect("prepare repeat");
        assert_eq!(dir, dir2, "same code must key the same crate dir");
        assert_eq!(
            mtime(&dir2.join("src/lib.rs")),
            lib_t,
            "unchanged repeat must not bump lib.rs mtime — the cargo cache hit dies otherwise"
        );
        assert_eq!(mtime(&dir2.join("Cargo.toml")), toml_t);

        // The batch lane: same snippet sequence → same key → zero writes.
        let snippets = [
            "pub fn a(x: i32) -> i32 { x }",
            "pub fn b(x: u8) -> u8 { x }",
        ];
        let bdir = oracle.prepare_batch_crate(&snippets).expect("batch prepare");
        let (blib_t, bs0_t) = (mtime(&bdir.join("src/lib.rs")), mtime(&bdir.join("src/s0.rs")));
        let bdir2 = oracle.prepare_batch_crate(&snippets).expect("batch repeat");
        assert_eq!(bdir, bdir2, "same snippet sequence must key the same crate dir");
        assert_eq!(mtime(&bdir2.join("src/lib.rs")), blib_t);
        assert_eq!(mtime(&bdir2.join("src/s0.rs")), bs0_t);

        let _ = std::fs::remove_dir_all(&root);
    }
}
