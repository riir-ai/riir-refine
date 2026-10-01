//! Plan 165 — the repo-posture fix lane, AT THE PROCESS BOUNDARY (the
//! `build_stamp_cli.rs` pattern).
//!
//! The class this guards is the 2026-09-21 riir-ai intake's 26:1 churn:
//! `--fix` on a foreign repo applying clippy-lane edits the target's OWN
//! clippy never flagged (our `doc_markdown` corpus is broader than
//! clippy's `[a-z][A-Z]` regex). `--repo-posture` intersects the
//! clippy-lane edits with the feed's same-lint diagnostic spans; the
//! two-occurrence fixture below is the minimal faithful reproduction —
//! ONE occurrence confirmed by the feed, one not.
//!
//! | arm | asserted |
//! |---|---|
//! | the gate | only the feed-confirmed occurrence is rewritten; the unconfirmed one stays byte-identical; exit 0; the decline is reported |
//! | the refusal | `--repo-posture` with no feed (auto-oracle off, no `--clippy-json`) exits **2** — never a silently-unverified apply wearing the posture name |
//! | P39 the NotBenchable gate | under posture the BENCHABLE perf edit applies and the NotBenchable one (`shrink-to-fit-after-build`, the dapps specimen class) is declined loudly; exit 0 |
//! | P39 the override | `--apply-notbenchable` re-enables the declined edit; the benchable edit still applies |
//! | P39 the misuse | `--apply-notbenchable` without `--repo-posture` exits **2** — the flag overrides nothing there |
//!
//! Hermetic: `RIIR_HEAL_KAT_LEASE=0` (the lease phase's own CI knob —
//! never touches the network), `RIIR_HEAL_KAT_METER=0` + `RIIR_HEAL_KAT_GATE=0`
//! (the billing plane + soft gate), `RIIR_HEAL_SELF_EVOLVE=0` (no store
//! writes), `RIIR_HEAL_AUTO_ORACLE=0` (no cargo spawn), and an unreachable
//! loopback `RIIR_HEAL_KAT_SERVICE_URL` so any best-effort tail transport
//! fails in milliseconds without an exit-code change (the tails' contract).
//!
//! File-level cfg = the cargo-refine bin's own `required-features` (the
//! `build_stamp_cli.rs` law): with less there is no binary to invoke and
//! `CARGO_BIN_EXE_cargo-refine` does not resolve.

#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))]

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Two `manual_range_contains` occurrences with DISTINGUISHABLE bounds so
/// the healed file tells us WHICH one was rewritten: the first (`3..=5`)
/// is covered by the feed's single diagnostic; the second (`7..=9`) is
/// not. The proposer fires on both (param-annotated scalar + literal
/// bounds — the Batch-158 shape).
const FIXTURE: &str = "pub fn in_band(x: u8) -> bool {
    x >= 3 && x <= 5
}

pub fn in_band_late(x: u8) -> bool {
    x >= 7 && x <= 9
}
";

fn heal(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let mut envs: HashMap<&str, String> = HashMap::new();
    envs.insert("RIIR_HEAL_KAT_LEASE", "0".to_string());
    envs.insert("RIIR_HEAL_KAT_METER", "0".to_string());
    envs.insert("RIIR_HEAL_KAT_GATE", "0".to_string());
    envs.insert("RIIR_HEAL_SELF_EVOLVE", "0".to_string());
    envs.insert("RIIR_HEAL_AUTO_ORACLE", "0".to_string());
    // Any best-effort tail transport (fix-stats, burn) fails in
    // milliseconds here and — by the tails' own contract — can never
    // change the exit code.
    envs.insert("RIIR_HEAL_KAT_SERVICE_URL", "http://127.0.0.1:9/".to_string());
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .args(args)
        .envs(envs)
        .current_dir(dir)
        .output()
        .expect("spawn cargo-refine");
    (
        out.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// A one-diagnostic feed covering ONLY the first occurrence's condition.
/// No `children` → no consumable suggestion → the P21 oracle lane stays
/// inert and the run exercises the PROPOSER gate alone. (A trailing
/// newline is unnecessary — the parser iterates lines either way.)
fn one_diagnostic_feed(fixture: &str) -> String {
    let off = fixture.find("x >= 3 && x <= 5").expect("fixture site 1");
    let end = off + "x >= 3 && x <= 5".len();
    let line = fixture[..off].matches('\n').count() + 1;
    format!(
        "{{\"reason\": \"compiler-message\", \"message\": {{\"level\": \"warning\", \
         \"code\": {{\"code\": \"clippy::manual_range_contains\"}}, \"spans\": \
         [{{\"file_name\": \"src/lib.rs\", \"line_start\": {line}, \"byte_start\": {off}, \
         \"byte_end\": {end}, \"is_primary\": true}}]}}}}"
    )
}

fn fixture_dir(feed: Option<&str>) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "repo_posture_cli_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(dir.join("src")).expect("mkdir src");
    std::fs::write(dir.join("src/lib.rs"), FIXTURE).expect("write fixture");
    if let Some(text) = feed {
        std::fs::write(dir.join("feed.json"), text).expect("write feed");
    }
    dir
}

#[test]
fn applies_only_the_feed_confirmed_edit() {
    let dir = fixture_dir(Some(&one_diagnostic_feed(FIXTURE)));
    let (code, text) = heal(
        &dir,
        &[
            "--fix",
            "--no-fmt",
            "--repo-posture",
            "--clippy-json",
            "feed.json",
            "src/lib.rs",
        ],
    );
    let healed = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "posture run must exit 0. output:\n{text}");
    assert!(
        healed.contains("(3..=5).contains(&x)"),
        "the CONFIRMED occurrence must be healed:\n{healed}"
    );
    assert!(
        healed.contains("x >= 7 && x <= 9"),
        "the UNCONFIRMED occurrence must stay byte-identical:\n{healed}"
    );
    assert!(
        !healed.contains("(7..=9).contains(&x)"),
        "the unconfirmed occurrence must NOT be rewritten:\n{healed}"
    );
    assert!(
        text.contains("repo-posture: declined 1 clippy-lane edit"),
        "the decline must be reported loudly:\n{text}"
    );
}

#[test]
fn refuses_without_a_feed() {
    let dir = fixture_dir(None);
    let (code, text) = heal(&dir, &["--fix", "--no-fmt", "--repo-posture", "src/lib.rs"]);
    let content = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        code,
        Some(2),
        "no-feed posture must REFUSE exit 2, not apply unverified. output:\n{text}"
    );
    assert!(
        text.contains("repo-posture: no clippy feed available"),
        "the refusal must name the mode and the remedies:\n{text}"
    );
    assert_eq!(
        content, FIXTURE,
        "a refused run must leave the file untouched"
    );
}

#[test]
fn rejects_posture_without_fix() {
    let dir = fixture_dir(None);
    let (code, text) = heal(&dir, &["--repo-posture", "src/lib.rs"]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(2), "misuse must exit 2. output:\n{text}");
    assert!(
        text.contains("--repo-posture requires --fix"),
        "the misuse message must name the requirement:\n{text}"
    );
}

/// P39 (post-mining intake, 2026-09-25): the posture lane's NotBenchable
/// decline. The fixture carries ONE NotBenchable perf site (the
/// `shrink-to-fit-after-build` guessed-hint shape — the exact dapps
/// specimen class) beside ONE benchable perf site (`vec-with-capacity`, a
/// measured rule). Under `--repo-posture` the benchable edit applies and
/// the NotBenchable one is declined loudly; `--apply-notbenchable`
/// re-enables it; the flag without the mode is misuse. The feed is an
/// EMPTY file — a clean-tree posture run proceeds (the documented
/// behavior) and the perf lane is what moves.
const NOTBENCH_FIXTURE: &str = "pub fn live_avatar_ids(roster: &[u64]) -> Vec<u64> {
    let mut ids = Vec::with_capacity(4096);
    for &id in roster {
        if id != 0 {
            ids.push(id);
        }
    }
    ids
}

pub fn build(n: usize) -> Vec<u32> {
    let mut v = Vec::new();
    for i in 0..n {
        v.push(i as u32);
    }
    v
}
";

fn notbench_fixture_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "repo_posture_notbench_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(dir.join("src")).expect("mkdir src");
    std::fs::write(dir.join("src/lib.rs"), NOTBENCH_FIXTURE).expect("write fixture");
    // An EMPTY feed: a clean tree is a legitimate posture input (warn +
    // proceed) — the perf lane is the only thing this arm exercises.
    std::fs::write(dir.join("feed.json"), "").expect("write feed");
    dir
}

#[test]
fn posture_declines_notbenchable_perf_edits() {
    let dir = notbench_fixture_dir();
    let (code, text) = heal(
        &dir,
        &[
            "--fix",
            "--no-fmt",
            "--repo-posture",
            "--clippy-json",
            "feed.json",
            "src/lib.rs",
        ],
    );
    let healed = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "posture run must exit 0. output:\n{text}");
    assert!(
        healed.contains("Vec::with_capacity(n)"),
        "the BENCHABLE perf edit must apply under posture:\n{healed}"
    );
    assert!(
        !healed.contains("shrink_to_fit"),
        "the NotBenchable edit must be declined under posture:\n{healed}"
    );
    assert!(
        healed.contains("Vec::with_capacity(4096)"),
        "the declined site's original text must stay byte-identical:\n{healed}"
    );
    assert!(
        text.contains("declined 1 NotBenchable rust_perf edit"),
        "the decline must be reported loudly with the override named:\n{text}"
    );
}

#[test]
fn apply_notbenchable_overrides_the_decline() {
    let dir = notbench_fixture_dir();
    let (code, text) = heal(
        &dir,
        &[
            "--fix",
            "--no-fmt",
            "--repo-posture",
            "--apply-notbenchable",
            "--clippy-json",
            "feed.json",
            "src/lib.rs",
        ],
    );
    let healed = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "override run must exit 0. output:\n{text}");
    assert!(
        healed.contains("ids.shrink_to_fit();"),
        "the explicit override must re-enable the NotBenchable edit:\n{healed}"
    );
    assert!(
        healed.contains("Vec::with_capacity(n)"),
        "the benchable edit still applies:\n{healed}"
    );
}

#[test]
fn rejects_apply_notbenchable_without_posture() {
    let dir = fixture_dir(None);
    let (code, text) = heal(
        &dir,
        &["--fix", "--no-fmt", "--apply-notbenchable", "src/lib.rs"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(code, Some(2), "misuse must exit 2. output:\n{text}");
    assert!(
        text.contains("--apply-notbenchable requires --repo-posture"),
        "the misuse message must name the requirement:\n{text}"
    );
}
