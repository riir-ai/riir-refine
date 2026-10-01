//! The compile-oracle substrate (Plan 019 T0.3): the cargo-check
//! subprocess and path resolution shared by [`crate::fix_verify`] (the
//! `--fix --verify` batch gate) and
//! [`crate::domains::rustc_errors::driver`] (the `--fix-compile` bounded
//! loop).
//!
//! ## Why a separate feature
//!
//! `fix_verify` implies `latent_retrieval` because it extends the cargo-refine
//! bin surface — but its CODE imports only `std` + `serde`. The
//! `rustc_errors` domain is exact-keyed by rustc error code and must not
//! drag riir-rag + riir-neuron-db into a compile fixer. Extracting the
//! genuinely shared core behind retrieval-free `compile_oracle` lets both
//! consumers share one oracle loop without coupling their feature trees:
//!
//! - `fix_verify = ["compile_oracle", "latent_retrieval"]`
//! - `rustc_errors = ["compile_oracle"]`
//!
//! ## What lives here
//!
//! - [`CheckRunner`] — one `cargo check --all-targets --message-format=json`
//!   invocation in a package root, with optional extra args (feature sets).
//!   Two env modes: [`CheckRunner::new`] inherits the environment (the
//!   caller's RUSTFLAGS are their reality — a real-package repair should be
//!   measured against the flags the user compiles with), while
//!   [`CheckRunner::isolated`] applies the probe hygiene (per-dir
//!   `CARGO_TARGET_DIR`, `RUSTFLAGS`/`RUSTC_WRAPPER` scrubbed) for scaffold
//!   crates whose diagnostics ARE the measurement (the
//!   [`crate::domains::rustc_errors::probe`] lesson).
//! - [`owning_package_root`] / [`workspace_root`] — the manifest walk.
//! - [`resolve_reported_path`] — rustc reports workspace-relative file
//!   names; this resolves them back to on-disk paths.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One cargo check result: the JSON-lines stdout + the exit status.
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub json: String,
    pub exit_ok: bool,
}

/// The three-way read of a check's completion (katgpt-rs Issue 841 — the
/// `fix_verify` refusal-vs-truncation audit).
///
/// The `Err` arm of [`CheckRunner::run_check`] fires only when cargo fails
/// WITHOUT emitting JSON; a cargo process killed mid-run (OOM, user kill,
/// disk-full rustc abort) that already flushed an artifact line comes back
/// as `Ok` with `exit_ok == false` and a PARTIAL stream. From the JSON
/// alone that state is indistinguishable from a clean pass — and both
/// decision consumers (the `--fix --verify` loop, the `--fix-compile`
/// driver) used to read it as one. The exit status is the completion
/// sentinel: a non-zero exit with no attributable diagnostics means the
/// gate did not complete, and no keep may ride it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckVerdict {
    /// Exited 0 with no error diagnostics — clean.
    Clean,
    /// Error diagnostics present (any exit status) — attribute + revert.
    Errors,
    /// Exited unsuccessfully with NO error diagnostics — the check did not
    /// complete. Every decision consumer must treat this as a refusal
    /// (restore + loud error), never as clean.
    Incomplete,
}

/// Classify a check result. `has_error_diags` comes from the consumer's OWN
/// diagnostic parser (`fix_verify`'s `parse_error_locations`, the
/// `rustc_errors` driver's `parse_error_diags`) — the same predicate the
/// pending keep/revert decision reads.
#[must_use]
pub fn check_verdict(exit_ok: bool, has_error_diags: bool) -> CheckVerdict {
    match (exit_ok, has_error_diags) {
        (true, false) => CheckVerdict::Clean,
        (_, true) => CheckVerdict::Errors,
        (false, false) => CheckVerdict::Incomplete,
    }
}

/// Runs `cargo check --all-targets --message-format=json` in a package root.
///
/// `--all-targets` is load-bearing, not cargo-default cargo-culting: edit
/// sites and repair targets live in `#[cfg(test)]` code too (the
/// `settlement_goat.rs` incident Plan 019 was filed on was a TEST file;
/// Issue 015's G5 run caught a `collapse_detector` const only reachable from
/// tests). Plain `cargo check` would "verify" those files without compiling
/// them.
pub struct CheckRunner {
    cargo_bin: String,
    /// Extra args appended to every check (feature sets etc. — the
    /// `--verify-args` passthrough).
    extra_args: Vec<String>,
    /// Probe-style env hygiene (see the module docs). `false` (the default)
    /// inherits the caller's environment.
    isolate_env: bool,
}

impl Default for CheckRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckRunner {
    /// Environment-inheriting runner (real-package use).
    #[must_use]
    pub fn new() -> Self {
        Self {
            cargo_bin: std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
            extra_args: Vec::new(),
            isolate_env: false,
        }
    }

    /// Environment-isolated runner (scaffold-crate use): per-dir
    /// `CARGO_TARGET_DIR` + `RUSTFLAGS`/`RUSTC_WRAPPER`/
    /// `CARGO_BUILD_RUSTFLAGS` scrubbed. Inherited flags can change WHICH
    /// diagnostics fire — which would corrupt a measurement or a
    /// keep/revert decision made on scaffold crates.
    #[must_use]
    pub fn isolated() -> Self {
        Self {
            isolate_env: true,
            ..Self::new()
        }
    }

    /// Append extra cargo args to every check (feature sets etc.).
    #[must_use]
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Explicit cargo binary override (defaults to `$CARGO`, then `cargo`).
    /// The kill-path e2e points this at a fake cargo that simulates a
    /// mid-check death; production callers never need it.
    #[must_use]
    pub fn with_cargo_bin(mut self, bin: impl Into<String>) -> Self {
        self.cargo_bin = bin.into();
        self
    }

    /// One check in `pkg`. `Err` only when cargo could not run at all
    /// (missing binary, broken manifest, bad args) — compile errors come
    /// back as `Ok(outcome)` with `exit_ok = false` + error diagnostics in
    /// the JSON.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when the cargo process cannot be
    /// spawned or fails without emitting any JSON (bad args / broken
    /// manifest).
    pub fn run_check(&self, pkg: &Path) -> Result<CheckOutcome, String> {
        let mut cmd = Command::new(&self.cargo_bin);
        cmd.current_dir(pkg)
            .arg("check")
            .arg("--all-targets")
            .arg("--message-format=json");
        for a in &self.extra_args {
            cmd.arg(a);
        }
        if self.isolate_env {
            cmd.env("CARGO_TARGET_DIR", pkg.join("target"))
                .env_remove("RUSTFLAGS")
                .env_remove("RUSTC_WRAPPER")
                .env_remove("CARGO_BUILD_RUSTFLAGS");
        }
        let output = cmd
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.status.success() && !stdout.lines().any(is_json_line) {
            // cargo itself failed without emitting JSON (bad args, broken
            // manifest, workspace-belief refusal) — surface stderr, with
            // the workspace-exclusion diagnosis appended when the
            // signature matches.
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(enrich_cargo_run_error(
                "cargo check",
                pkg,
                &first_meaningful_line(&stderr),
            ));
        }
        Ok(CheckOutcome {
            json: stdout,
            exit_ok: output.status.success(),
        })
    }

    /// One `cargo clippy --all-targets --message-format=json -- -W clippy::<lint>…`
    /// run in a package root — the lint-resolution oracle (037-T2). Same
    /// env/extra-args handling as [`Self::run_check`]; the named lints are
    /// forced on regardless of lint group (pedantic/nursery lints otherwise
    /// never fire under a default-group run), so resolution can be judged
    /// per lint. `Err` only when cargo could not run at all — lint findings
    /// come back as `Ok(outcome)` in the JSON whatever the exit status.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when the cargo process cannot be
    /// spawned or fails without emitting any JSON.
    pub fn run_clippy(&self, pkg: &Path, lint_names: &[String]) -> Result<CheckOutcome, String> {
        let mut cmd = Command::new(&self.cargo_bin);
        cmd.current_dir(pkg)
            .arg("clippy")
            .arg("--all-targets")
            .arg("--message-format=json");
        for a in &self.extra_args {
            cmd.arg(a);
        }
        cmd.arg("--");
        for lint in lint_names {
            cmd.arg("-W").arg(format!("clippy::{lint}"));
        }
        if self.isolate_env {
            cmd.env("CARGO_TARGET_DIR", pkg.join("target"))
                .env_remove("RUSTFLAGS")
                .env_remove("RUSTC_WRAPPER")
                .env_remove("CARGO_BUILD_RUSTFLAGS");
        }
        let output = cmd
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if !stdout.lines().any(is_json_line) {
            // cargo itself failed without emitting JSON (bad args, broken
            // manifest, workspace-belief refusal) — surface stderr, with
            // the workspace-exclusion diagnosis appended when the
            // signature matches.
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(enrich_cargo_run_error(
                "cargo clippy",
                pkg,
                &first_meaningful_line(&stderr),
            ));
        }
        Ok(CheckOutcome {
            json: stdout,
            exit_ok: output.status.success(),
        })
    }
}

pub(crate) fn is_json_line(line: &str) -> bool {
    line.trim_start().starts_with('{')
}

pub(crate) fn first_meaningful_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("(no stderr)")
        .to_string()
}

/// Walk up from a `.rs` file to the nearest directory holding a
/// `Cargo.toml` — the owning package root (cargo forbids nested packages,
/// so the first manifest is THE manifest of the crate the file compiles
/// into).
#[must_use]
pub fn owning_package_root(file: &Path) -> Option<PathBuf> {
    // Normalize to an absolute path first: with a relative input like
    // `src/foo.rs` (files collected from CLI args), `pop()` walks down to
    // the EMPTY path whose `Cargo.toml` join resolves against the process
    // cwd — returning `Some("")` whose `Command::current_dir` then fails
    // with os error 123 (the Issue 015 GOAT run's spawn failure).
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

/// Walk up from a package root to the workspace root (the nearest
/// ancestor — including the package itself — whose `Cargo.toml` declares
/// a `[workspace]` table). Falls back to the package root for standalone
/// crates. Used to resolve rustc's workspace-relative `file_name`s.
#[must_use]
pub fn workspace_root(pkg_root: &Path) -> Option<PathBuf> {
    let mut dir = Some(pkg_root.to_path_buf());
    while let Some(d) = dir {
        let manifest = d.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest) {
            if manifest_declares_workspace(&text) {
                return Some(d);
            }
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    // No [workspace] found above: the package stands alone.
    Some(pkg_root.to_path_buf())
}

fn manifest_declares_workspace(text: &str) -> bool {
    text.lines()
        .any(|l| l.trim_start().starts_with("[workspace"))
}

/// Diagnose the cargo "current package believes it's in a workspace when
/// it's not" refusal for `pkg`: the owning manifest has NO `[workspace]`
/// table but some ancestor manifest DOES — so cargo adopted the ancestor's
/// workspace, and that workspace does not list `pkg` as a member (or lists
/// it under a glob `exclude`). Returns the root cause + remedy, or `None`
/// when the shape does not apply (`pkg` self-roots, or no enclosing
/// workspace exists — the ordinary standalone case).
///
/// Membership itself is NOT parsed (ancestor `members` lists accept globs,
/// and a false "you are excluded" would be worse than none) — this is the
/// precondition set of the refusal, surfaced with the fix that resolves
/// every instance of it. The container-tick-test incident (2026-09-04,
/// mmorpg `bd4a4f2` follow-up): the whole `--verify` batch auto-reverted
/// on this error with only cargo's one-line stderr to go on.
#[must_use]
pub fn workspace_belief_cause(pkg: &Path) -> Option<String> {
    let manifest = pkg.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).ok()?;
    if manifest_declares_workspace(&text) {
        return None; // self-rooting — cargo never walks above it
    }
    let mut dir = pkg.parent()?.to_path_buf();
    loop {
        let ancestor = dir.join("Cargo.toml");
        if ancestor.is_file() {
            if let Ok(ancestor_text) = std::fs::read_to_string(&ancestor) {
                if manifest_declares_workspace(&ancestor_text) {
                    return Some(format!(
                        "cargo refused because {} has no [workspace] table while the \
                         enclosing workspace at {} does not list it as a member — add an \
                         empty `[workspace]` table to {} (the self-rooting precedent), or \
                         run the pass scoped to that crate root",
                        manifest.display(),
                        ancestor.display(),
                        manifest.display(),
                    ));
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Enrich a cargo spawn/run failure with the workspace-belief diagnosis
/// when the stderr matches cargo's signature line; otherwise the message
/// passes through unchanged. Shared by [`CheckRunner::run_check`] and
/// [`CheckRunner::run_clippy`] — both fail this way on the same shape.
fn enrich_cargo_run_error(label: &str, pkg: &Path, stderr_first: &str) -> String {
    let base = format!("{label} failed to run in {}: {stderr_first}", pkg.display());
    if stderr_first.contains("believes it's in a workspace") {
        match workspace_belief_cause(pkg) {
            Some(cause) => format!("{base}; {cause}"),
            None => base,
        }
    } else {
        base
    }
}

/// Resolve a rustc-reported file name to an on-disk path. rustc reports
/// workspace-RELATIVE paths (`crates/foo/src/lib.rs`); try the workspace
/// root first, then the package root, and canonicalize whichever exists.
#[must_use]
pub fn resolve_reported_path(p: &Path, ws: &Path, pkg: &Path) -> PathBuf {
    if p.is_absolute() {
        return std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    }
    let joined = ws.join(p);
    if let Ok(c) = std::fs::canonicalize(&joined) {
        return c;
    }
    let joined_pkg = pkg.join(p);
    if let Ok(c) = std::fs::canonicalize(&joined_pkg) {
        return c;
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owning_package_root_walks_up() {
        let tmp = std::env::temp_dir().join(format!("co_pkg_root_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let pkg = tmp.join("ws").join("crates").join("a");
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(pkg.join("Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        std::fs::write(
            tmp.join("ws").join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\"]\n",
        )
        .unwrap();
        let f = pkg.join("src").join("lib.rs");
        assert_eq!(owning_package_root(&f).as_deref(), Some(pkg.as_path()));
        assert_eq!(
            workspace_root(&pkg).as_deref(),
            Some(tmp.join("ws").as_path())
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The Issue 841 verdict matrix: all four (exit, diagnostics) cells.
    /// `Incomplete` is the kill/truncation shape — non-zero exit, zero
    /// parseable diagnostics, indistinguishable from clean on the JSON
    /// alone — and is its own verdict precisely so no consumer can fold it
    /// into `Clean` by accident.
    #[test]
    fn check_verdict_matrix() {
        assert_eq!(check_verdict(true, false), CheckVerdict::Clean);
        assert_eq!(check_verdict(true, true), CheckVerdict::Errors);
        assert_eq!(check_verdict(false, true), CheckVerdict::Errors);
        assert_eq!(check_verdict(false, false), CheckVerdict::Incomplete);
    }

    /// The container-tick-test shape: a memberless nested crate under a
    /// parent workspace. The precondition set of cargo's "believes it's in
    /// a workspace" refusal — the cause must name both manifests + the
    /// remedy. Negative controls: self-rooting manifest + plain standalone
    /// (no ancestor workspace) both return `None`.
    #[test]
    fn workspace_belief_cause_names_the_excluded_nested_crate() {
        let tmp = std::env::temp_dir().join(format!("co_ws_belief_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // Positive: nested crate WITHOUT [workspace] under a parent WITH one.
        let nested = tmp.join("parent").join("cloudflare").join("tick");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            tmp.join("parent").join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        std::fs::write(nested.join("Cargo.toml"), "[package]\nname = \"tick\"\n").unwrap();
        let cause = workspace_belief_cause(&nested).expect("cause for the nested shape");
        assert!(cause.contains("tick"), "names the nested manifest: {cause}");
        assert!(
            cause.contains("[workspace] table"),
            "states the remedy: {cause}"
        );

        // Negative: self-rooting manifest — cargo never walks above it.
        std::fs::write(
            nested.join("Cargo.toml"),
            "[package]\nname = \"tick\"\n\n[workspace]\n",
        )
        .unwrap();
        assert!(workspace_belief_cause(&nested).is_none(), "self-rooted");

        // Negative: no ancestor workspace at all (plain standalone).
        let plain = tmp.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(plain.join("Cargo.toml"), "[package]\nname = \"p\"\n").unwrap();
        assert!(workspace_belief_cause(&plain).is_none(), "no ancestor");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The enrichment only fires on cargo's signature line; unrelated
    /// spawn failures pass through byte-identical.
    #[test]
    fn enrich_cargo_run_error_matches_the_signature_only() {
        let tmp = std::env::temp_dir().join(format!("co_ws_enrich_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let nested = tmp.join("parent").join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            tmp.join("parent").join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        std::fs::write(nested.join("Cargo.toml"), "[package]\nname = \"n\"\n").unwrap();

        let sig = "current package believes it's in a workspace when it's not";
        let enriched = enrich_cargo_run_error("cargo check", &nested, sig);
        assert!(
            enriched.contains("cargo check failed to run in"),
            "{enriched}"
        );
        assert!(
            enriched.contains(sig),
            "keeps the original line: {enriched}"
        );
        assert!(enriched.contains("[workspace] table"), "adds the remedy");

        let other = "no such subcommand: `chek`";
        assert_eq!(
            enrich_cargo_run_error("cargo check", &nested, other),
            format!("cargo check failed to run in {}: {other}", nested.display()),
            "unrelated errors unchanged"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The Issue 015 regression, re-pinned at the new home: a RELATIVE
    /// input must resolve to an absolute package root, not the empty path.
    #[test]
    fn owning_package_root_relative_input_is_absolute() {
        let tmp = std::env::temp_dir().join(format!("co_pkg_rel_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        std::fs::write(tmp.join("src").join("lib.rs"), "").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();
        let rel = Path::new("src/lib.rs");
        let got = owning_package_root(rel).expect("package root found");
        std::env::set_current_dir(prev).unwrap();
        assert!(
            got.is_absolute(),
            "relative input must return an absolute package root, got {got:?}"
        );
        assert_eq!(got.file_name(), tmp.file_name());
        assert!(got.join("Cargo.toml").is_file());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_reported_path_prefers_workspace_join() {
        let tmp = std::env::temp_dir().join(format!("co_resolve_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let ws = tmp.join("ws");
        let pkg = ws.join("crates").join("a");
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(ws.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(pkg.join("Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        let lib = pkg.join("src").join("lib.rs");
        std::fs::write(&lib, "").unwrap();
        let canon = std::fs::canonicalize(&lib).unwrap();
        let got = resolve_reported_path(Path::new("crates/a/src/lib.rs"), &ws, &pkg);
        assert_eq!(got, canon);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn workspace_detection_reads_manifest_tables() {
        assert!(manifest_declares_workspace(
            "[package]\nname = \"x\"\n\n[workspace]\nmembers = [\"crates/a\"]\n"
        ));
        assert!(!manifest_declares_workspace("[package]\nname = \"x\"\n"));
    }
}
