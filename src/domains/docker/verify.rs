//! The `docker_verify` oracle tier — Plan 030 T0.3.
//!
//! Tier 0 (always available) is the stanza check in
//! [`DockerDomain::validate`](super::DockerDomain). Tier 1 re-runs
//! **hadolint** — the incumbent detector — to confirm that a Class-A rule's
//! finding is actually GONE after our fix. That is the `clippy_verify`
//! symmetry: the domain that overlays another tool's detection proves its
//! fixes against that tool, not against its own opinion.
//!
//! ## Degradation is a fact, not a pass (the Issue 018 oracle-split)
//!
//! When the binary is absent, every entry point returns `None` and logs
//! ONE line. `None` is never coerced to `false` and never to `true`:
//! "we could not check" and "the check failed" are different facts, and a
//! gate that conflates them starts lying the first time a CI box lacks the
//! tool.
//!
//! ## Scope
//!
//! hadolint runs on the CANDIDATE TEXT over stdin. Nothing is written to
//! disk, no daemon is contacted, and no network call is made — Tier 2
//! (`docker build --check`) and Tier 3 (a real build) stay out of the
//! library and live in the bench harness.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::corpus::build_docker_seed_corpus;

/// One hadolint finding, reduced to what the agreement gate needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HadolintFinding {
    /// The rule code (`DL3009`, `SC2086`).
    pub code: String,
    /// 1-based line.
    pub line: usize,
}

/// Whether the oracle binary is usable, probed once per process.
#[must_use]
pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let ok = Command::new("hadolint")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            // The ONE degradation line (Issue 018). Not a warning storm:
            // once per process, and it says what it means.
            eprintln!(
                "docker_verify: hadolint not on PATH — Class-A agreement DEGRADES to the Tier-0 \
stanza check. This is 'not checked', NOT 'checked and passed'."
            );
        }
        ok
    })
}

/// Run hadolint over `source`. `None` means the oracle was unavailable —
/// never an empty finding list.
#[must_use]
pub fn hadolint(source: &str) -> Option<Vec<HadolintFinding>> {
    if !available() {
        return None;
    }
    let mut child = Command::new("hadolint")
        .args(["--no-color", "--format", "json", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(source.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some(parse_json(&String::from_utf8_lossy(&out.stdout)))
}

/// Minimal extractor for hadolint's JSON array.
///
/// A hand-rolled reader rather than a serde model: the domain ships ZERO
/// new dependencies, the two fields we need are flat, and a malformed line
/// degrades to "no findings parsed" instead of a panic.
fn parse_json(stdout: &str) -> Vec<HadolintFinding> {
    let mut out = Vec::new();
    for chunk in stdout.split('{').skip(1) {
        let code = field(chunk, "\"code\":\"").map(str::to_string);
        let line = field(chunk, "\"line\":").and_then(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        });
        if let (Some(code), Some(line)) = (code, line) {
            out.push(HadolintFinding { code, line });
        }
    }
    out.shrink_to_fit();
    out
}

fn field<'a>(chunk: &'a str, key: &str) -> Option<&'a str> {
    let rest = &chunk[chunk.find(key)? + key.len()..];
    if key.ends_with('"') {
        rest.split('"').next()
    } else {
        Some(rest)
    }
}

/// The verdict of one Class-A agreement check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    /// hadolint reported the incumbent code on the buggy fixture and NOT on
    /// the fixed one — our detector and our fix both agree with the oracle.
    Agrees,
    /// hadolint did not report the code on the buggy fixture: our detector
    /// claims a finding the incumbent does not make. Either the fixture
    /// drifted or the rule is not the overlay it says it is.
    OracleSilentOnBuggy,
    /// hadolint still reports the code after our fix — the fix does not
    /// resolve what it claimed to.
    StillFiresAfterFix,
    /// The oracle was unavailable. NOT a pass.
    Unavailable,
}

/// Check one Class-A corpus rule against the oracle.
///
/// Class-B rules (no incumbent) return [`Agreement::Unavailable`] — there
/// is no oracle to agree with, by definition.
#[must_use]
pub fn class_a_agreement(rule_id: &str) -> Agreement {
    let corpus = build_docker_seed_corpus();
    let Some(rule) = corpus.entries.iter().find(|r| r.rule_id == rule_id) else {
        return Agreement::Unavailable;
    };
    let Some(code) = rule.incumbent.as_deref() else {
        return Agreement::Unavailable;
    };
    // hadolint owns the DL codes. A BuildKit check NAME belongs to Tier 2
    // (`docker build --check`), which is the `docker_buildcheck` feature —
    // absent it, the honest answer is still `Unavailable`, never a pass.
    if !code.starts_with("DL") {
        return tier2_agreement(rule);
    }
    let (Some(before), Some(after)) = (hadolint(&rule.buggy_code), hadolint(&rule.fixed_code))
    else {
        return Agreement::Unavailable;
    };
    verdict(
        before.iter().any(|f| f.code == code),
        after.iter().any(|f| f.code == code),
    )
}

/// The shared before/after reduction — one place, so Tier 1 and Tier 2
/// cannot drift into disagreeing about what "agrees" means.
fn verdict(fires_on_buggy: bool, fires_on_fixed: bool) -> Agreement {
    match (fires_on_buggy, fires_on_fixed) {
        (false, _) => Agreement::OracleSilentOnBuggy,
        (true, true) => Agreement::StillFiresAfterFix,
        (true, false) => Agreement::Agrees,
    }
}

/// Tier-2 dispatch. Two `cfg` arms rather than a `cfg`-gated body: both
/// compile in every feature state, which is the Bench 056 lesson about the
/// dead-code failure class that turned CI red on the sec landing.
#[cfg(feature = "docker_buildcheck")]
fn tier2_agreement(rule: &super::DockerRule) -> Agreement {
    use super::buildcheck;

    let Some(check) = rule.tier2_check() else {
        return Agreement::Unavailable;
    };
    let (Some(before), Some(after)) = (
        buildcheck::findings(&rule.buggy_code),
        buildcheck::findings(&rule.fixed_code),
    ) else {
        // Unavailable OR a truncated (`Errored`) run — both are "not
        // checked". `buildcheck` already collapsed that distinction, and
        // collapsing it toward `Unavailable` is the safe direction.
        return Agreement::Unavailable;
    };
    verdict(
        before.iter().any(|f| f.check == check),
        after.iter().any(|f| f.check == check),
    )
}

#[cfg(not(feature = "docker_buildcheck"))]
fn tier2_agreement(_rule: &super::DockerRule) -> Agreement {
    Agreement::Unavailable
}

/// Check every rule that names a Tier-2 (`BuildKit`) check.
///
/// Returns `(rule_id, check, verdict)` per rule. This is the standing gate
/// that Bench 056's postscript could only assert by hand.
#[must_use]
pub fn all_tier2_agreements() -> Vec<(String, String, Agreement)> {
    build_docker_seed_corpus()
        .entries
        .iter()
        .filter_map(|r| {
            let check = r.tier2_check()?.to_string();
            Some((r.rule_id.clone(), check, tier2_agreement(r)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_hadolint_array() {
        let json = r#"[{"code":"DL3006","column":1,"file":"-","level":"warning","line":1,"message":"x"},{"code":"DL3019","column":1,"file":"-","level":"info","line":2,"message":"y"}]"#;
        let got = parse_json(json);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[0],
            HadolintFinding {
                code: "DL3006".into(),
                line: 1
            }
        );
        assert_eq!(got[1].code, "DL3019");
        assert_eq!(got[1].line, 2);
    }

    #[test]
    fn malformed_output_degrades_to_empty_not_panic() {
        assert!(parse_json("").is_empty());
        assert!(parse_json("not json at all {oops").is_empty());
        assert!(parse_json("[{\"code\":\"DL1\"}]").is_empty()); // no line field
    }

    #[test]
    fn class_b_rules_have_no_oracle_to_agree_with() {
        assert_eq!(
            class_a_agreement("cache-mount-missing-on-cargo-build"),
            Agreement::Unavailable
        );
        assert_eq!(class_a_agreement("not-a-rule"), Agreement::Unavailable);
    }
}
