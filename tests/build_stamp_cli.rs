//! Issue 117 T3 — the staleness stamp and the refusal, AT THE PROCESS
//! BOUNDARY (the `kat_soft_gate_cli.rs` / `kat_refill_cli.rs` pattern).
//!
//! The class this guards is a binary that EXISTS, runs, and advertises its
//! gated commands in `--help` while they are `cfg`'d out — the 00:21
//! incident. Two halves, and a unit test can only reach the first:
//!
//! | half | asserted here |
//! |---|---|
//! | the stamp | `--version` exits 0 and names the version, the compiled feature set, and the release-set verdict |
//! | the refusal | in a build WITHOUT `kat_transport`, `sync` exits **2** with the refusal block — never `unknown flag`, never silence |
//!
//! The second half is `#[cfg(not(feature = "kat_transport"))]` on the TEST
//! FUNCTION rather than on the file: a whole-file `#![cfg]` would make this
//! target compile to an empty binary under the release feature set (and
//! under `--all-features`), printing a green `0 passed` — the Issue-713
//! green-zero trap. With the split, every feature set runs the stamp arms
//! and exactly one of them additionally runs the refusal arm.
//!
//! The file-level gate is the cargo-refine bin's own `required-features`: with
//! less than that there is no binary to invoke at all, and
//! `CARGO_BIN_EXE_cargo-refine` does not resolve.

#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))]

use std::process::Command;

/// One invocation of the real binary. Nothing here reaches the network or
/// the user's real config — `--version` and a refusal both answer before any
/// account/service read.
fn heal(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .args(args)
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

/// `--version` stays exit 0 and keeps its first line, and now carries the
/// compiled feature set. The first line is asserted separately because
/// anything that parsed the pre-117 single-line output must keep working.
#[test]
fn version_prints_the_compiled_feature_stamp() {
    let (code, text) = heal(&["--version"]);
    assert_eq!(code, Some(0), "--version must exit 0. output:\n{text}");
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    assert!(
        first.starts_with("cargo-refine "),
        "line 0 must stay `cargo-refine <version>`, got {first:?}"
    );
    assert!(
        text.contains("features: "),
        "the stamp must name the compiled set. output:\n{text}"
    );
    assert!(
        text.contains(riir_refine::build_stamp::COMPILED_FEATURES),
        "the printed set must BE this build's set. output:\n{text}"
    );
    assert!(
        text.contains("release set:"),
        "the stamp must state the release-set verdict. output:\n{text}"
    );
    // riir-kat Issue 003 T9: the NET PIN line — sourced from riir-kat's own
    // const (the member that ENFORCES the pin names it). This test build is
    // unpinned, so the multi-net label is the pinned expectation — UNLESS
    // this IS an all-features build, where both pins ride and the conflict
    // sentence is the correct const (issue 159: the conflict posture is a
    // legal build, not a test failure). The expectation mirrors the same
    // cfg! match riir-kat's own test derives (issue 163: the conflict label
    // names the hosted refusals — loopback follows allow-local there).
    assert!(
        text.contains(&format!("net: {}", riir_refine::kat_net_pin::NET_PIN)),
        "the version block must carry the net pin. output:\n{text}"
    );
    match (cfg!(feature = "net-mainnet"), cfg!(feature = "net-devnet")) {
        (false, false) => {
            assert_eq!(
                riir_refine::kat_net_pin::NET_PIN,
                "multi (default mainnet)",
                "this test binary enables no pin features"
            );
        }
        (true, true) => {
            assert_eq!(
                riir_refine::kat_net_pin::NET_PIN,
                "conflict: net-mainnet+net-devnet (every hosted host refuses; \
                 loopback follows allow-local)",
                "both pins enabled: the conflict label is the pinned expectation"
            );
        }
        (true, false) => {
            assert_eq!(
                riir_refine::kat_net_pin::NET_PIN,
                "mainnet",
                "net-mainnet only: the label names the pin"
            );
        }
        (false, true) => {
            assert_eq!(
                riir_refine::kat_net_pin::NET_PIN,
                "devnet",
                "net-devnet only: the label names the pin"
            );
        }
    }
}

/// `-V` is the documented short form and must print the same block — a
/// stamp reachable only through the long form is a stamp half the users
/// never see.
#[test]
fn short_v_prints_the_same_block() {
    let (long_code, long_text) = heal(&["--version"]);
    let (short_code, short_text) = heal(&["-V"]);
    assert_eq!(long_code, short_code);
    assert_eq!(long_text, short_text);
}

/// Issue 150 (option 3): the version block names the commit, and — the
/// STANDING shadow check — a binary built from THIS checkout names THIS
/// checkout's HEAD. The `build:` line's ABSENCE only catches pre-150
/// binaries (the shipped 0.2.1); the hash comparison is what keeps catching
/// the shadow from the next release on: mismatch = some other tree's binary
/// answering on PATH. `CARGO_BIN_EXE` builds this very tree right before the
/// test runs, so the two must agree (optionally with a `-dirty` suffix —
/// this worktree is usually mid-edit). The one benign race: a sibling
/// session committing between the binary's build-script run and this read —
/// a re-run resolves it.
#[test]
fn version_names_the_build_commit_and_it_matches_this_checkout() {
    let (code, text) = heal(&["--version"]);
    assert_eq!(code, Some(0), "--version must exit 0. output:\n{text}");
    let line = text
        .lines()
        .find(|l| l.starts_with("build: "))
        .expect("a post-150 binary must print the build: line — its absence IS the Issue-150 shadow. output:\n{text}");
    let stamp = line.trim_start_matches("build: ");
    assert!(
        riir_refine::build_stamp::is_valid_stamp(stamp),
        "malformed build stamp {stamp:?}"
    );
    let head = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git in this checkout");
    assert!(
        head.status.success(),
        "git rev-parse failed in this checkout"
    );
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    assert!(
        stamp.starts_with(&head),
        "build stamp {stamp:?} does not start with this checkout's HEAD {head} — the binary answering was NOT built from this tree (the Issue-150 shadow class). Rebuild."
    );
}

/// The verdict half of the stamp: whichever side this build is on, the
/// output must say so, and must name the rebuild command EXACTLY when the
/// release set is incomplete. A complete build that nags is noise; an
/// incomplete one that does not is the 00:21 incident.
#[test]
fn the_release_verdict_matches_this_build() {
    let (_, text) = heal(&["--version"]);
    let missing = riir_refine::build_stamp::missing_release_features();
    if missing.is_empty() {
        assert!(text.contains("release set: complete"), "{text}");
        assert!(!text.contains("rebuild:"), "{text}");
    } else {
        assert!(text.contains("release set: STALE"), "{text}");
        assert!(
            text.contains(&riir_refine::build_stamp::rebuild_command()),
            "a stale binary must print the rebuild command. output:\n{text}"
        );
        for f in &missing {
            assert!(text.contains(f), "missing feature {f} unnamed:\n{text}");
        }
    }
}

/// The refusal half — only measurable in a build that actually lacks the
/// transport. Gated on the FUNCTION so the target never compiles to an
/// empty binary (see the module doc).
#[cfg(not(feature = "kat_transport"))]
#[test]
fn sync_refuses_loudly_instead_of_disappearing() {
    let (code, text) = heal(&["sync"]);
    assert_eq!(
        code,
        Some(2),
        "a compiled-out `sync` must exit 2 (the --refill precedent). output:\n{text}"
    );
    assert!(text.contains("sync: refused"), "{text}");
    assert!(text.contains("kat_transport"), "{text}");
    assert!(
        text.contains(&riir_refine::build_stamp::rebuild_command_for(&[
            "kat_transport"
        ])),
        "the refusal must name a rebuild that SUPPLIES what it asked for. output:\n{text}"
    );
    // The two shapes the class is mistaken for.
    assert!(!text.contains("unknown flag"), "{text}");
    assert!(!text.trim().is_empty(), "silence is the worst outcome");
}

/// Same for `--refill`: it is a FLAG rather than a subcommand, so it takes a
/// different path through the parser and needs its own row.
#[cfg(not(feature = "kat_transport"))]
#[test]
fn refill_refuses_loudly_too() {
    let (code, text) = heal(&["--refill"]);
    assert_eq!(code, Some(2), "output:\n{text}");
    assert!(text.contains("kat_transport"), "{text}");
    assert!(!text.contains("unknown flag"), "{text}");
}

/// In a build that HAS the transport, `sync` must not be refused by the
/// staleness path at all — the guard must not fire on a healthy binary.
/// (It may still fail for its own reasons; what it may not do is claim the
/// feature is missing.)
#[cfg(feature = "kat_transport")]
#[test]
fn a_complete_build_never_prints_the_staleness_refusal() {
    assert!(
        riir_refine::build_stamp::missing_release_features()
            .iter()
            .all(|f| *f != "kat_transport"),
        "the stamp disagrees with cfg! about kat_transport"
    );
    let (_, text) = heal(&["--version"]);
    assert!(
        !text.contains("sync: refused"),
        "a healthy build must not refuse. output:\n{text}"
    );
}

// ── riir-dapps Proposal 009 §9 / issue 129: the airdrop verbs ──────────────
// Same two halves as the sync/refill rows above, one feature over: in a
// build WITHOUT `admin`, `get`/`send` exit 2 with the refusal block naming
// the feature AND the rebuild; in a build WITH it, the pre-network key
// discipline holds (a missing/malformed TUNA_CAN_OPENER_KEY refuses exit 1
// before any socket, naming the env var, never echoing a value).

#[cfg(not(feature = "admin"))]
#[test]
fn tuna_airdrop_verbs_refuse_when_admin_is_compiled_out() {
    let acct = "aa".repeat(16);
    for (verb, argv) in [
        ("get", vec!["get", "100", "tuna"]),
        ("send", vec!["send", "100", "tuna", acct.as_str()]),
    ] {
        let (code, text) = heal(&argv);
        assert_eq!(code, Some(2), "{verb}: output:\n{text}");
        assert!(text.contains("admin"), "{verb}: {text}");
        assert!(text.contains("rebuild with"), "{verb}: {text}");
        assert!(!text.contains("unknown"), "{verb}: {text}");
    }
}

/// Issue 159 posture for the tuna verbs: an all-features build carries BOTH
/// net pins, the allowed hosted set is empty, and the binary refuses the
/// service URL (exit 2) before any verb-local check runs — that refusal is
/// the correct outcome there, not a failure. Returns `true` when this build
/// IS the conflict posture and the refusal was asserted, so the caller
/// skips its single-net expectations.
#[cfg(feature = "admin")]
fn asserted_net_conflict_refusal(code: Option<i32>, text: &str) -> bool {
    if !cfg!(all(feature = "net-mainnet", feature = "net-devnet")) {
        return false;
    }
    assert_eq!(code, Some(2), "conflict posture: output:\n{text}");
    assert!(
        text.contains("service url refused") && text.contains("net-mainnet+net-devnet"),
        "conflict posture must name the overlap refusal. output:\n{text}"
    );
    true
}

#[cfg(feature = "admin")]
#[test]
fn tuna_airdrop_missing_key_refuses_pre_network() {
    let acct = "bb".repeat(16);
    for argv in [
        vec!["get", "100", "tuna"],
        vec!["send", "100", "tuna", acct.as_str()],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
            .args(&argv)
            .env_remove("TUNA_CAN_OPENER_KEY")
            .output()
            .expect("spawn cargo-refine");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if asserted_net_conflict_refusal(out.status.code(), &text) {
            continue;
        }
        assert_eq!(out.status.code(), Some(1), "output:\n{text}");
        assert!(
            text.contains("TUNA_CAN_OPENER_KEY"),
            "the refusal must name the env var. output:\n{text}"
        );
    }
}

#[cfg(feature = "admin")]
#[test]
fn tuna_airdrop_below_floor_refuses_locally_at_exit_2() {
    // 99 TUNA < the 100-TUNA floor — refused BEFORE any secret is read
    // (the cap mirror fires ahead of the env discipline by design).
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .args(["get", "99", "tuna"])
        .env_remove("TUNA_CAN_OPENER_KEY")
        .output()
        .expect("spawn cargo-refine");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if asserted_net_conflict_refusal(out.status.code(), &text) {
        return;
    }
    assert_eq!(out.status.code(), Some(2), "output:\n{text}");
    assert!(text.contains("free-tier floor"), "output:\n{text}");
}

#[cfg(feature = "admin")]
#[test]
fn tuna_airdrop_dry_run_is_fully_local_and_shows_the_hash() {
    // `get` needs the LOCAL account (not a network) for the target — on a
    // test box without one, the refusal is the honest answer; either way
    // nothing dials (the dry-run path never constructs a transport).
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .args(["get", "100", "tuna", "--dry-run"])
        .env_remove("TUNA_CAN_OPENER_KEY")
        .env_remove("KAT_CRANK_TOKEN")
        .output()
        .expect("spawn cargo-refine");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if asserted_net_conflict_refusal(out.status.code(), &text) {
        return;
    }
    assert!(
        text.contains("DRY RUN") || text.contains("no local account"),
        "either the local preview or the honest account refusal — output:\n{text}"
    );
}
