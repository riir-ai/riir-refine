//! The `docker_buildcheck` oracle tier — Plan 030 Phase 2, Tier 2.
//!
//! Tier 0 is the stanza check, Tier 1 is [`hadolint`](super::verify), and
//! this is **Tier 2: `docker build --check`** — `BuildKit`'s own linter, the
//! incumbent for the four rules whose `incumbent` is a check NAME rather
//! than a `DL` code. Until this module existed those four claims were
//! verified once by hand ([Bench 056](../../../.benchmarks/056_docker_domain_phase0_1_goat.md)
//! postscript) and then trusted; now they are a standing gate.
//!
//! ## Why a separate feature from `docker_verify`
//!
//! hadolint is a static binary that reads stdin. `docker build --check`
//! needs a **running daemon** and, for any fixture whose base image is not
//! `scratch`, a **registry round-trip** to resolve source metadata. That is
//! a strictly heavier dependency class, so it gets its own opt-in gate
//! rather than riding along on `docker_verify`. Measured on Docker 29.7.2 /
//! buildx v0.36.1: `FROM scratch` ~0.2 s, a registry base ~1.3-2.2 s per
//! invocation, and the registry hit does NOT warm — a repeat run paid it
//! again. This oracle therefore never runs in the heal path; it runs in
//! gates and benches.
//!
//! ## Three measured facts this parser is built on
//!
//! 1. **There is no structured output.** `--progress=rawjson` emits only
//!    `vertexes`/`statuses` objects; the check results are printed as plain
//!    text *outside* the JSON stream. Text parsing is not a shortcut here,
//!    it is the only interface.
//! 2. **Findings go to stdout, build errors go to stderr.** They are
//!    cleanly separable, which is what makes fact 3 handleable.
//! 3. **A resolve failure SILENTLY TRUNCATES the finding set.** Feeding a
//!    two-defect Dockerfile whose base image cannot be resolved reported
//!    *one* warning, not two, and still exited 1. So a partial result is
//!    indistinguishable from a complete one by shape alone — the only
//!    signal is `ERROR:` on stderr, and [`CheckRun::Errored`] discards the
//!    findings rather than reporting a subset as if it were the whole.
//!
//! The exit code is deliberately **not** used as the verdict: 1 means both
//! "found warnings" and "the build errored", and 0 means both "clean" and
//! (for a plain `docker build`) "built fine". It cannot carry the
//! distinction this module exists to preserve.
//!
//! ## Degradation is a fact, not a pass (the Issue 018 oracle-split)
//!
//! No daemon, no `--check` support, or a build error => the caller gets
//! [`CheckRun::Unavailable`] / [`CheckRun::Errored`], never an empty
//! finding list. "We could not check" and "we checked and it was clean"
//! stay different facts.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// One `BuildKit` check finding, reduced to what the agreement gate needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCheckFinding {
    /// The check name (`ConsistentInstructionCasing`, `JSONArgsRecommended`).
    pub check: String,
    /// 1-based line within the checked Dockerfile.
    pub line: usize,
}

/// The outcome of one oracle invocation.
///
/// Three variants rather than `Option<Vec<_>>` because the middle case is
/// real and load-bearing: the oracle CAN run, produce output, exit non-zero
/// and still be reporting only part of the truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckRun {
    /// The oracle ran to completion. The list is exhaustive (possibly empty
    /// — that is a genuine "clean").
    Ran(Vec<BuildCheckFinding>),
    /// No daemon / no `--check` support / the binary is absent. NOT a pass.
    Unavailable,
    /// `BuildKit` errored (typically base-image resolve). Any findings it did
    /// print are a SUBSET, so they are dropped: reporting them would let a
    /// truncated run masquerade as a clean one.
    Errored(String),
}

/// Whether the whole oracle path works, probed once per process.
///
/// The probe is the oracle itself on `FROM scratch` — an end-to-end
/// capability test rather than a version-string guess, and cheap
/// (~0.2 s, no registry round-trip) precisely because `scratch` resolves
/// locally.
#[must_use]
pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let ok = matches!(run_raw("FROM scratch\n"), Some((0, ref out, _)) if out.contains("Check complete"));
        if !ok {
            // The ONE degradation line (Issue 018), once per process.
            eprintln!(
                "docker_buildcheck: `docker build --check` unusable (no daemon, no --check \
support, or docker absent) — Tier-2 agreement DEGRADES. This is 'not checked', NOT 'checked \
and passed'."
            );
        }
        ok
    })
}

/// Run `docker build --check` over `source` on stdin with an empty context.
///
/// Nothing is written to the repo and no image is produced — `--check` is a
/// parse-and-lint pass. A registry round-trip still happens for any base
/// image that is not `scratch`; that is `BuildKit`'s behavior, not ours.
#[must_use]
pub fn build_check(source: &str) -> CheckRun {
    if !available() {
        return CheckRun::Unavailable;
    }
    let Some((_code, stdout, stderr)) = run_raw(source) else {
        return CheckRun::Unavailable;
    };
    // Fact 3: stderr `ERROR:` means the finding set on stdout is a subset.
    if let Some(err) = stderr.lines().map(str::trim).find(|l| l.starts_with("ERROR:")) {
        return CheckRun::Errored(err.to_string());
    }
    CheckRun::Ran(parse_checks(&stdout))
}

/// [`build_check`] reduced to `Option`, for callers that treat "could not
/// check" and "checked partially" the same way. Both still map to `None` —
/// never to an empty `Vec`.
#[must_use]
pub fn findings(source: &str) -> Option<Vec<BuildCheckFinding>> {
    match build_check(source) {
        CheckRun::Ran(f) => Some(f),
        CheckRun::Unavailable | CheckRun::Errored(_) => None,
    }
}

/// Spawn the oracle. `None` only when the process could not be run at all.
fn run_raw(source: &str) -> Option<(i32, String, String)> {
    let mut child = Command::new("docker")
        .args(["build", "--check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(source.as_bytes()).ok()?;
    // Drop the handle so the child sees EOF; otherwise it waits on stdin.
    drop(child.stdin.take());
    let out = child.wait_with_output().ok()?;
    Some((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// Extract `(check name, line)` pairs from the human-readable report.
///
/// The shape, verbatim from the tool:
///
/// ```text
/// WARNING: ConsistentInstructionCasing - https://docs.docker.com/go/...
/// Command 'from' should match the case of the command majority (uppercase)
/// /tmp/empty-dir123/Dockerfile:1
/// ```
///
/// A finding whose location line never arrives is dropped rather than
/// guessed at — the same discipline as the hadolint parser's "no line field
/// => not a finding".
fn parse_checks(stdout: &str) -> Vec<BuildCheckFinding> {
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    for raw in stdout.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("WARNING: ") {
            // A new WARNING before the previous one's location => the
            // previous was malformed. Drop it, do not carry it forward.
            let name = rest.split(" - ").next().unwrap_or(rest).trim();
            pending = (!name.is_empty()).then(|| name.to_string());
            continue;
        }
        let Some(name) = pending.as_ref() else { continue };
        // The location line: `<path>:<n>`. Anchor on the Dockerfile name so
        // a message that happens to end in `:12` cannot be mistaken for one.
        let Some((path, num)) = line.rsplit_once(':') else { continue };
        if !path.ends_with("Dockerfile") {
            continue;
        }
        if let Ok(n) = num.trim().parse::<usize>() {
            out.push(BuildCheckFinding { check: name.clone(), line: n });
            pending = None;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = "Check complete, 2 warnings have been found!\n\n\
WARNING: ConsistentInstructionCasing - https://docs.docker.com/go/dockerfile/rule/consistent-instruction-casing/\n\
Command 'from' should match the case of the command majority (uppercase)\n\
/var/folders/tmp/empty-dir99/Dockerfile:1\n\
--------------------\n   1 | >>> from scratch\n--------------------\n\n\
WARNING: LegacyKeyValueFormat - https://docs.docker.com/go/dockerfile/rule/legacy-key-value-format/\n\
\"ENV key=value\" should be used instead of legacy \"ENV key value\" format\n\
/var/folders/tmp/empty-dir99/Dockerfile:2\n";

    #[test]
    fn parses_the_real_report_shape() {
        let got = parse_checks(REPORT);
        assert_eq!(
            got,
            vec![
                BuildCheckFinding { check: "ConsistentInstructionCasing".into(), line: 1 },
                BuildCheckFinding { check: "LegacyKeyValueFormat".into(), line: 2 },
            ]
        );
    }

    #[test]
    fn clean_report_has_no_findings() {
        assert!(parse_checks("Check complete, no warnings found.\n").is_empty());
        assert!(parse_checks("").is_empty());
    }

    #[test]
    fn a_message_ending_in_a_colon_number_is_not_a_location() {
        // The anchor is the `Dockerfile` suffix, not "has a colon".
        let report = "WARNING: SomeCheck - https://x\n\
see also RFC 1234:56\n\
/tmp/d/Dockerfile:7\n";
        assert_eq!(parse_checks(report), vec![BuildCheckFinding {
            check: "SomeCheck".into(),
            line: 7
        }]);
    }

    #[test]
    fn a_warning_with_no_location_is_dropped_not_guessed() {
        let report = "WARNING: Orphan - https://x\nmessage only\n\
WARNING: Real - https://y\n/tmp/d/Dockerfile:3\n";
        assert_eq!(parse_checks(report), vec![BuildCheckFinding {
            check: "Real".into(),
            line: 3
        }]);
    }
}
