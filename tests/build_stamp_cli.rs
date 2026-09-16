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
//! The file-level gate is the cargo-heal bin's own `required-features`: with
//! less than that there is no binary to invoke at all, and
//! `CARGO_BIN_EXE_cargo-heal` does not resolve.

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
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-heal"))
        .args(args)
        .output()
        .expect("spawn cargo-heal");
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
        first.starts_with("cargo-heal "),
        "line 0 must stay `cargo-heal <version>`, got {first:?}"
    );
    assert!(
        text.contains("features: "),
        "the stamp must name the compiled set. output:\n{text}"
    );
    assert!(
        text.contains(riir_clippy::build_stamp::COMPILED_FEATURES),
        "the printed set must BE this build's set. output:\n{text}"
    );
    assert!(
        text.contains("release set:"),
        "the stamp must state the release-set verdict. output:\n{text}"
    );
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

/// The verdict half of the stamp: whichever side this build is on, the
/// output must say so, and must name the rebuild command EXACTLY when the
/// release set is incomplete. A complete build that nags is noise; an
/// incomplete one that does not is the 00:21 incident.
#[test]
fn the_release_verdict_matches_this_build() {
    let (_, text) = heal(&["--version"]);
    let missing = riir_clippy::build_stamp::missing_release_features();
    if missing.is_empty() {
        assert!(text.contains("release set: complete"), "{text}");
        assert!(!text.contains("rebuild:"), "{text}");
    } else {
        assert!(text.contains("release set: STALE"), "{text}");
        assert!(
            text.contains(&riir_clippy::build_stamp::rebuild_command()),
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
        text.contains(&riir_clippy::build_stamp::rebuild_command_for(&["kat_transport"])),
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
        riir_clippy::build_stamp::missing_release_features()
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
