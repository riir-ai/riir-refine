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
//! ## Four measured facts this parser is built on
//!
//! 1. **There is no structured output.** `--progress=rawjson` emits only
//!    `vertexes`/`statuses` objects; the check results are printed as plain
//!    text *outside* the JSON stream. Text parsing is not a shortcut here,
//!    it is the only interface.
//! 2. **Findings go to stdout, build errors go to stderr.** They are
//!    cleanly separable, which is what makes fact 3 handleable.
//!    Re-verified 2026-09-06 on Docker 29.7.2: the `WARNING:` block rides
//!    stdout, the `ERROR:` resolve-failure block rides stderr, and the
//!    progress (`#N ...`) lines ride stderr.
//! 3. **A resolve failure SILENTLY TRUNCATES the finding set.** Feeding a
//!    two-defect Dockerfile whose base image cannot be resolved reported
//!    *one* warning, not two, and still exited 1. So a partial result is
//!    indistinguishable from a complete one by shape alone — the only
//!    signal is `ERROR:` on stderr, and [`CheckRun::Errored`] discards the
//!    findings rather than reporting a subset as if it were the whole.
//! 4. **A completed run always prints `Check complete` on stdout**;
//!    a failed resolve never does (measured 2026-09-06, Issue 071). This
//!    is the classification backstop for fact 3: if a future buildx
//!    reroutes or rewords the error summary, the `ERROR:`-prefix gate
//!    stops firing — and a run without the marker still lands
//!    [`CheckRun::Errored`], never a fake `Ran(empty)`. That fake was the
//!    mechanism behind Issue 071's misread: a registry-blocked box produced
//!    a verdict that looked like "the check stopped firing" when the check
//!    phase had never completed.
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

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

/// Hard wall-clock cap on ONE `docker build --check` invocation.
///
/// Measured healthy cost is ~0.2 s (`FROM scratch`) to ~2.2 s (a registry
/// base, which does not warm). This cap is ~50x the slowest healthy call,
/// so it can never fire on a working daemon — it exists for the state
/// where the docker CLI **hangs instead of failing**. Measured live
/// (2026-09-05, Docker Desktop 4.87, a ~75-min window where the
/// `docker-credential-desktop` helper blocked in `open()`): every
/// `build --check` on a non-`scratch` base inherited that hang and the
/// oracle (plus any gate behind it) stalls until an OUTER timeout kills
/// the whole run. The wedge self-recovered (helper re-probed instant
/// the same evening), so it is a TRANSIENT daemon state — which is
/// exactly why the bound belongs in code and not in a one-off kill:
/// the next occurrence is unannounced. A hang is worse than
/// [`CheckRun::Unavailable`]: it converts "not checked" into "never
/// finished checking".
const CALL_TIMEOUT: Duration = Duration::from_secs(120);

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
/// The invocation is bounded by [`CALL_TIMEOUT`]; on timeout the child is
/// killed and the run degrades to [`CheckRun::Unavailable`] — the
/// "check-by-hang" failure mode (a wedged credential helper blocks the
/// CLI for as long as the daemon stays wedged) must not be
/// distinguishable from a pass by anyone waiting on us.
#[must_use]
pub fn build_check(source: &str) -> CheckRun {
    if !available() {
        return CheckRun::Unavailable;
    }
    let Some((code, stdout, stderr)) = run_raw(source) else {
        return CheckRun::Unavailable;
    };
    classify(code, &stdout, &stderr)
}

/// Reduce one completed oracle invocation to a [`CheckRun`]. Pure, so the
/// classification rules are unit-testable without a daemon.
///
/// Two independent gates, in order:
///
/// 1. **An `ERROR:` line on stderr** — the measured resolve-failure shape
///    (2026-09-06, Docker 29.7.2: the whole `ERROR: alpine:3.20: failed to
///    resolve...` block rides stderr while stdout stays empty). The
///    findings that may have printed are a subset, so they are dropped.
/// 2. **The completion marker** — a run whose stdout never prints
///    `Check complete` did not finish the lint phase, whatever its stderr
///    says. Measured 2026-09-06: every completed run prints it ("Check
///    complete, N warnings" / "no warnings"), every failed resolve prints
///    none. This is the belt to gate 1's braces: if a future buildx
///    reroutes or rewords the error summary, gate 1 stops firing — and
///    THIS gate catches the run, because the failure of a completed check
///    to print the marker is a stream-routing drift, and misreading it as
///    `Ran(empty)` is exactly the fake-clean that once produced an
///    `OracleSilentOnBuggy` misdiagnosis (Issue 071). The marker's own
///    wording changing lands `Errored` — loud, never a fake `Ran`.
#[must_use]
pub fn classify(_exit: i32, stdout: &str, stderr: &str) -> CheckRun {
    if let Some(err) = stderr.lines().map(str::trim).find(|l| l.starts_with("ERROR:")) {
        return CheckRun::Errored(err.to_string());
    }
    if !stdout.contains("Check complete") {
        return CheckRun::Errored(
            "no `ERROR:` line and no `Check complete` marker — the check phase never finished"
                .to_string(),
        );
    }
    CheckRun::Ran(parse_checks(stdout))
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

/// Spawn the oracle under [`CALL_TIMEOUT`]. `None` when the process could
/// not be run at all OR outlived the cap (killed) — the caller degrades
/// that to [`CheckRun::Unavailable`], because a hang is "not checked",
/// never a pass. `wait_with_output` has no timeout form, and a piped
/// `kill` without draining can deadlock the child on a full pipe buffer,
/// so the pipes are drained on worker threads while the deadline is
/// polled with `try_wait`.
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

    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    // Drain both pipes on worker threads so a full pipe buffer can never
    // block the child while we enforce the deadline.
    let (out_tx, out_rx) = std::sync::mpsc::channel();
    let (err_tx, err_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = out_tx.send(buf);
    });
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        let _ = err_tx.send(buf);
    });

    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().ok().flatten() {
            break status;
        }
        if start.elapsed() >= CALL_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!(
                "docker_buildcheck: invocation exceeded the {CALL_TIMEOUT:?} cap and was killed — \
degrading to Unavailable (a hang is 'not checked', never a pass)"
            );
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    };

    let out = out_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();
    let err = err_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();
    Some((
        status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
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
    out.shrink_to_fit();
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

    // ── classify: the two failure gates, unit-tested without a daemon ──

    const COMPLETED_DIRTY: &str = "Check complete, 1 warning has been found!\n\
WARNING: LegacyKeyValueFormat - https://docs.docker.com/go/dockerfile/rule/legacy-key-value-format/\n\
\"ENV key=value\" should be used instead of legacy \"ENV key value\" format\n\
/tmp/d/Dockerfile:2\n";

    #[test]
    fn classify_completed_runs_parse_findings() {
        assert_eq!(
            classify(1, COMPLETED_DIRTY, ""),
            CheckRun::Ran(vec![BuildCheckFinding {
                check: "LegacyKeyValueFormat".into(),
                line: 2
            }])
        );
        assert_eq!(classify(0, "Check complete, no warnings found.\n", ""), CheckRun::Ran(vec![]));
    }

    #[test]
    fn classify_stderr_error_wins_even_over_a_marker() {
        // A resolve failure measured 2026-09-06: stdout empty, stderr
        // carries the `ERROR:` summary. No findings may survive.
        let err = "ERROR: alpine:3.20: failed to resolve source metadata \
for docker.io/library/alpine:3.20: DeadlineExceeded: context deadline exceeded\n";
        assert_eq!(
            classify(1, "", err),
            CheckRun::Errored(err.trim_end().to_string())
        );
        // Even a (never-observed) both-shapes run degrades to Errored.
        assert!(matches!(
            classify(1, COMPLETED_DIRTY, err),
            CheckRun::Errored(_)
        ));
    }

    #[test]
    fn classify_run_without_the_completion_marker_is_never_ran() {
        // The Issue 071 hardening: a stream-rerouted failure (no `ERROR:`
        // line reachable on stderr) must land Errored, never a fake
        // `Ran(empty)`.
        assert!(matches!(
            classify(1, "", "#2 ERROR: DeadlineExceeded\n"),
            CheckRun::Errored(_)
        ));
        assert!(matches!(classify(0, "", ""), CheckRun::Errored(_)));
        assert!(matches!(
            classify(1, "WARNING: X - https://x\n/tmp/d/Dockerfile:1\n", ""),
            CheckRun::Errored(_)
        ));
    }

    #[test]
    fn classify_error_line_inside_the_progress_block_does_not_erro() {
        // The progress prefix `=> ERROR` does not start with `ERROR:` —
        // measured stderr shape. Without a completed marker it still errs
        // (gate 2); with one it is a completed run.
        let progress = "#1 [internal] load build definition\n#1 DONE 0.0s\n";
        assert!(matches!(
            classify(1, COMPLETED_DIRTY, progress),
            CheckRun::Ran(_)
        ));
        assert!(matches!(
            classify(1, "", progress),
            CheckRun::Errored(_)
        ));
    }
}
