//! Issue 117 — the compiled-feature stamp and the loud staleness refusal.
//!
//! ## The class
//!
//! Any bare `cargo build --release` of this crate overwrites the symlink
//! target (`~/.cargo/bin/cargo-refine`) with a DEFAULT-FEATURES binary. That
//! binary exists, runs, and still advertises `sync` / `--refill` / the
//! burn-push tail in `--help` — while every one of them is `cfg`'d out. The
//! 00:21 incident (AGENTS.md "Standing caveat") is exactly that: a healer
//! that looked like it worked and could not talk to the network.
//!
//! The repair is not to prevent the build — a build outside this repo's
//! control cannot be prevented from here. It is to make the staleness
//! **loud at the CLI boundary** and **identifiable in one command**:
//!
//! 1. [`version_lines`] — `--version` prints the compiled feature set beside
//!    the version, plus what the release set is missing. A stale binary is
//!    then diagnosable from any terminal without reading the build log.
//! 2. [`refusal_lines`] — a gated command that is compiled out prints a
//!    refusal block naming the missing features AND the rebuild command,
//!    then exits 2 (the `--refill` exit-2 precedent). Never "unknown flag",
//!    never silence.
//!
//! ## The stamp is derived, not typed
//!
//! [`COMPILED_FEATURES`] comes from `build.rs`, which reads the
//! `CARGO_FEATURE_*` environment cargo exports for the compilation actually
//! happening. A `cfg!` ladder here would need a hand-maintained list of
//! feature names and would go stale on the next feature added — the drift
//! class this workspace gates for elsewhere. The release set
//! ([`RELEASE_FEATURES`]) IS typed, because it is a claim about
//! `.github/workflows/release.yml`; [`release_set_matches_workflow`] is the
//! arm that keeps the claim honest against the workflow file.

/// Every feature ON for this compilation, comma-separated and sorted.
/// Written by `build.rs` from cargo's own `CARGO_FEATURE_*` exports.
pub const COMPILED_FEATURES: &str = env!("RIIR_CLIPPY_COMPILED_FEATURES");

/// The commit this binary was compiled from (Issue 150, option 3) —
/// `<12-hex>` or `<12-hex>-dirty`, or `unknown` when the build had no git
/// (tarball, bare non-repo dir).
///
/// THE SHADOW IT ANSWERS: a release binary on PATH ahead of the maintained
/// dev symlink used to be indistinguishable — same version, same feature
/// stamp. Two different builds name different commits, and a binary built
/// BEFORE this change prints no `build:` line at all, so the standing check
/// (`cargo refine --version`, then compare the hash against
/// `git rev-parse --short=12 HEAD` in the repo you meant to run) catches
/// every shadow class: missing line = pre-stamp binary; `unknown` = a build
/// outside any checkout; mismatch = a different tree.
pub const BUILD_COMMIT: &str = env!("RIIR_REFINE_BUILD_COMMIT");

/// The build LANE (Issue 225): `crates.io` when the publish surgery's lane
/// marker was in the package (`cargo install cargo-refine`), else `source`
/// (the dev tree and the prebuilt binaries). On the crates.io lane
/// [`BUILD_COMMIT`] is the published ref's commit, resolved at surgery time.
pub const BUILD_LANE: &str = env!("RIIR_REFINE_BUILD_LANE");

/// The lane value the build script exports for a crates.io build.
pub const LANE_CRATESIO: &str = "crates.io";

/// The crates.io lane's release-set line. The registry crate is the
/// intended code-fixing posture (issue 221 lane design): the release
/// features it lacks do not EXIST in its manifest, so it is not stale and
/// there is nothing to rebuild — the hosted surfaces ship in the binaries.
pub const CRATESIO_RELEASE_LINE: &str = "release set: crates.io (code-fixing build — join/leaderboard/latent retrieval ship in the prebuilt binaries: brew or install.sh)";

/// Is this the crates.io build ([`BUILD_LANE`])?
#[must_use]
pub fn is_cratesio_lane() -> bool {
    BUILD_LANE == LANE_CRATESIO
}

/// Shape check for a stamp: `unknown`, or `<7..=40 hex>` with an optional
/// single `-dirty` suffix. The one validator — the version block, the unit
/// tests, and the process-boundary CLI test all share it, so a malformed
/// stamp fails shape before it is trusted as a commit name.
#[must_use]
pub fn is_valid_stamp(stamp: &str) -> bool {
    if stamp == "unknown" {
        return true;
    }
    let hex = stamp.strip_suffix("-dirty").unwrap_or(stamp);
    !hex.is_empty()
        && hex.len() >= 7
        && hex.len() <= 40
        && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// The feature set `.github/workflows/release.yml` builds the shipped
/// `cargo-refine` with, in the workflow's own order (the order is part of the
/// claim — the rebuild command below is meant to be copy-pasteable).
pub const RELEASE_FEATURES: &[&str] = &[
    "latent_retrieval",
    // Issue 192 / Plan 325 — the public-goods pivot (owner posture act,
    // 2026-11-03): the shipped binary is the CHARITABLE set — the points
    // surface (join/leave/rank/corpus + the client UUID + first-run consent
    // + offline) REPLACES the KAT billing plane. points implies the join
    // identity chain (riir-kat client + provider_login). kat_service /
    // kat_transport stay OUT — the T14 charitable sweep holds its hard-zero
    // KAT classes; Refine is mainnet-only (no devnet lane ships).
    "points",
    // The owner's mainnet-only directive (2026-11-04, the fifth raise —
    // "do it now"): the shipped binary is net-PINNED, not default-mainnet.
    // The pin compiles the devnet host set to nothing (the foreign-hosts
    // compile-to-nothing discipline) — `--net devnet` and any env/config
    // override pointing off the mainnet host family (mainnet.riir.ai /
    // tuna.riir.ai / riir.ai) refuse at runtime naming the pin. Loopback
    // stays out (no allow-local in the shipped set — the wrangler-dev
    // lane is a dev build's).
    "net-mainnet",
    "fix_verify",
    // Issue 153 — a shipped binary without the update alert is exactly the
    // "old version with no notice" class this feature exists to end.
    "update_check",
    // Issue 208 / Bench 137 — the kernel pregate PROMOTED to the shipped set
    // (owner call 2026-11-06, "any gain no perf hurt must enabled by
    // default"): every scan auto-detects kernel-bearing files and the kernel
    // domain joins the fan-out there, at the measured composed posture.
    // Implies kernel_opt — dep:regex enters the shipped license graph, so
    // the licenses job names the feature for cargo-about too.
    "kernel_pregate",
    // Issue 213 / Bench 143 — shader_perf PROMOTED (G2 + G3 PASS): the
    // DomainMask path gate keeps the WGSL render rules off .rs spans, so the
    // domain costs added p95 < 1 ms/file. Zero deps (license graph unchanged;
    // the licenses job names it anyway — one feature list, lockstep).
    "shader_perf",
    // Issue 216 / Bench 144 — span_shape_dispatch PROMOTED (T4 cost PASS:
    // 3 interleaved rounds, added p50 <= 0.01 ms AND p95 <= 2.02 ms per file
    // against the 1.0/5.0 ceilings — the ON arm reuses the Issue-063 shared
    // per-span parse and `rule_admits` is a 6-row table lookup). Shape-keyed
    // dispatch over the Perf corpus's 6 annotated rows; fail-open everywhere
    // else. Implies latent_retrieval + rust_perf — BOTH already shipped, so
    // the dep + license graphs are unchanged (the cargo-about line needs no
    // new name). Kill-switch `RIIR_REFINE_SPAN_SHAPE=0` (the exact literal,
    // DEMOTE-ONLY). Bare cargo default stays light BY DECISION: the feature
    // implies the latent_retrieval tree (dep:riir-rag), so promotion targets
    // the release set only — the pre-registration's "Cargo default" surface
    // is amended with that reason in Bench 144.
    "span_shape_dispatch",
];

/// The copy-pasteable rebuild for the shipped set. Deliberately names
/// `--bin cargo-refine`: a workspace-wide release build is what produced the
/// stale binary in the first place.
#[must_use]
pub fn rebuild_command() -> String {
    rebuild_command_for(&[])
}

/// The rebuild that actually fixes THIS refusal: the release set plus any
/// feature the refusing command needs that the release set does not carry.
///
/// ⛔ Printing the bare release set would have been wrong for `replay`,
/// whose gate is `kat_transport + clippy_verify + latent_retrieval` — a user
/// who pasted the shipped recipe would rebuild and hit the same refusal. A
/// remedy that does not remedy is worse than none: it spends the user's
/// trust and their build time, and it looks like the guard is broken.
#[must_use]
pub fn rebuild_command_for(needed: &[&str]) -> String {
    let mut set: Vec<&str> = RELEASE_FEATURES.to_vec();
    for f in needed {
        if !set.contains(f) {
            set.push(f);
        }
    }
    format!(
        "cargo build --release --features {} --bin cargo-refine",
        set.join(",")
    )
}

/// The compiled set as a slice, for membership questions.
#[must_use]
pub fn compiled() -> Vec<&'static str> {
    COMPILED_FEATURES
        .split(',')
        .filter(|s| !s.is_empty())
        .collect()
}

/// Is `feature` compiled into this binary? Cargo's env mangling turns `-`
/// into `_` (`CARGO_FEATURE_NET_MAINNET` → `net_mainnet`), so the lookup
/// compares in the SAME spelling — `net-mainnet` (the first hyphenated
/// feature in the manifest) must not read as missing forever.
#[must_use]
pub fn has(feature: &str) -> bool {
    let normalized = feature.replace('-', "_");
    compiled().iter().any(|f| f.replace('-', "_") == normalized)
}

/// Which of [`RELEASE_FEATURES`] this build lacks, in the release order.
/// Empty = this binary carries the shipped surface.
#[must_use]
pub fn missing_release_features() -> Vec<&'static str> {
    RELEASE_FEATURES
        .iter()
        .copied()
        .filter(|f| !has(f))
        .collect()
}

/// The `--version` block (Issue 117 T1). Line 0 is the unchanged
/// `cargo-refine <version>` — anything parsing the old single-line output keeps
/// working; the stamp rides after it. The `net:` line (riir-kat Issue 003 T9)
/// names the NET PIN as riir-kat's own const computes it — the member that
/// ENFORCES the pin is the member that names it (this crate's feature list
/// cannot see the forwarded member features).
#[must_use]
pub fn version_lines() -> Vec<String> {
    version_lines_for(
        is_cratesio_lane(),
        BUILD_COMMIT,
        &missing_release_features(),
    )
}

/// The pure half of [`version_lines`] — the lane, the commit stamp and the
/// missing set are parameters so both lanes are testable from one build.
/// The crates.io lane (Issue 225) never prints STALE or a rebuild line: the
/// features the rebuild would name are not in the published manifest.
#[must_use]
pub fn version_lines_for(cratesio: bool, commit: &str, missing: &[&str]) -> Vec<String> {
    let mut out = vec![
        format!("cargo-refine {}", env!("CARGO_PKG_VERSION")),
        format!("features: {COMPILED_FEATURES}"),
        format!("build: {commit}"),
        format!("net: {}", crate::kat_net_pin::NET_PIN),
    ];
    if cratesio {
        out.push(CRATESIO_RELEASE_LINE.to_string());
    } else if missing.is_empty() {
        out.push(format!(
            "release set: complete ({})",
            RELEASE_FEATURES.join(",")
        ));
    } else {
        out.push(format!(
            "release set: STALE — missing {} (the network surfaces — sync/update/--refill/burn-push — are compiled out of this binary)",
            missing.join(",")
        ));
        out.push(format!("rebuild: {}", rebuild_command()));
    }
    out
}

/// The refusal block a compiled-out command prints before `exit(2)`
/// (Issue 117 T2). `command` is what the user typed (`sync`, `--refill`, …);
/// `needed` is the feature(s) whose absence disabled it.
///
/// Three lines, in the order a reader needs them: what refused and why, what
/// this binary actually is, and the one command that fixes it. The middle
/// line is the part the 00:21 incident lacked — without it a user cannot tell
/// a stale binary from a server outage.
#[must_use]
pub fn refusal_lines(command: &str, needed: &[&str]) -> Vec<String> {
    refusal_lines_for(is_cratesio_lane(), command, needed)
}

/// The pure half of [`refusal_lines`]. On the crates.io lane (Issue 225)
/// the third line points at the prebuilt binaries instead of a rebuild —
/// the needed features are not in the published manifest, so the rebuild
/// would fail.
#[must_use]
pub fn refusal_lines_for(cratesio: bool, command: &str, needed: &[&str]) -> Vec<String> {
    let remedy = if cratesio {
        format!(
            "{command}: this is the crates.io build — `{command}` ships in the prebuilt binaries (brew or install.sh)"
        )
    } else {
        format!("{command}: rebuild with: {}", rebuild_command_for(needed))
    };
    vec![
        format!(
            "{command}: refused — this build has no {} (the command is compiled out, not broken)",
            needed.join(",")
        ),
        format!("{command}: this binary's features: {COMPILED_FEATURES}"),
        remedy,
    ]
}

/// Print [`refusal_lines`] to stderr and exit 2.
pub fn refuse(command: &str, needed: &[&str]) -> ! {
    for line in refusal_lines(command, needed) {
        eprintln!("{line}");
    }
    std::process::exit(2)
}

/// Does [`RELEASE_FEATURES`] still match what the release workflow builds?
///
/// The constant is a CLAIM about a file in this repo, and a claim nothing
/// checks is a claim that goes stale — the exact shape the stamp itself was
/// built to derive around. Returns the workflow's set when it can be read, so
/// the caller can name the disagreement.
///
/// # Errors
///
/// The workflow is unreadable, or carries no `--features` line beside
/// `--bin cargo-refine`. Both are "the instrument cannot see", never "matches".
pub fn release_set_from_workflow(workflow: &str) -> Result<Vec<String>, String> {
    // The release build line is `--features <set> --bin cargo-refine`, wrapped
    // across a shell continuation in the real file. Read the token AFTER
    // `--features` on whichever line also mentions `--bin cargo-refine`.
    let line = workflow
        .lines()
        .find(|l| l.contains("--bin cargo-refine") && l.contains("--features"))
        .ok_or_else(|| "no line carrying both `--features` and `--bin cargo-refine`".to_string())?;
    let mut it = line.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == "--features" {
            let set = it
                .next()
                .ok_or_else(|| "`--features` with no value on the release line".to_string())?;
            // A value that is itself a flag means the line does not have the
            // shape assumed — report that rather than returning `["--bin"]`,
            // which would read as a real (wrong) set to the comparison arm.
            if set.starts_with('-') {
                return Err(format!(
                    "`--features` followed by {set:?}, not a feature set"
                ));
            }
            return Ok(set.split(',').map(str::to_string).collect());
        }
    }
    Err("`--features` present but not as its own token".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stamp is non-empty in any feature-bearing build: `default` alone
    /// supplies six features, and a build script that stopped exporting would
    /// leave this empty — a stamp that silently says nothing is worse than
    /// none, because `--version` would print a confident blank set.
    /// EXCEPTION (the flag-OFF posture, P45): `--no-default-features` with no
    /// extra features exports zero `CARGO_FEATURE_*` vars, so the empty stamp
    /// is the CORRECT projection there — the assert then requires the
    /// default-on witness to be off, so a broken build script still fails at
    /// every posture someone actually builds.
    #[test]
    // The assert is DELIBERATELY constant-valued: a cfg tripwire, not a
    // runtime check (allow, never `const { }` — a const block would make
    // the default posture a compile error, the opposite of the tripwire's
    // test-time intent).
    #[allow(clippy::assertions_on_constants)]
    fn the_stamp_is_populated() {
        if COMPILED_FEATURES.is_empty() {
            assert!(
                !cfg!(feature = "clippy_lints"),
                "empty stamp with clippy_lints on = build.rs stopped exporting"
            );
            return;
        }
        // Sorted + comma-separated, no stray whitespace.
        let v = compiled();
        let mut sorted = v.clone();
        sorted.sort_unstable();
        assert_eq!(v, sorted, "the stamp must be sorted: {COMPILED_FEATURES}");
        assert!(v.iter().all(|f| !f.contains(' ')), "{COMPILED_FEATURES}");
    }

    /// `has` answers about the build running the test — assert it against a
    /// `cfg!`, the one independent witness available in-process.
    #[test]
    fn has_agrees_with_cfg() {
        assert_eq!(has("clippy_lints"), cfg!(feature = "clippy_lints"));
        assert_eq!(has("kat_transport"), cfg!(feature = "kat_transport"));
        assert_eq!(has("latent_retrieval"), cfg!(feature = "latent_retrieval"));
        // A name that is not a feature is never compiled in.
        assert!(!has("not_a_feature"));
    }

    /// The missing set is exactly the release features `cfg!` says are off —
    /// and it is ORDERED by the release list, not by the compiled set.
    #[test]
    fn missing_is_the_release_complement_in_release_order() {
        let missing = missing_release_features();
        for f in RELEASE_FEATURES {
            assert_eq!(
                missing.contains(f),
                !has(f),
                "{f} membership disagrees with the stamp"
            );
        }
        let idx: Vec<usize> = missing
            .iter()
            .map(|m| RELEASE_FEATURES.iter().position(|r| r == m).unwrap())
            .collect();
        let mut sorted = idx.clone();
        sorted.sort_unstable();
        assert_eq!(idx, sorted, "missing must keep the release order");
    }

    /// The version block always names the version, always names the compiled
    /// set, and names the rebuild command IFF something is missing — a
    /// complete build must not nag, an incomplete one must not be quiet.
    #[test]
    fn version_block_shape_follows_the_verdict() {
        let lines = version_lines();
        assert!(lines[0].starts_with("cargo-refine "));
        assert!(lines[1].starts_with("features: "));
        let joined = lines.join("\n");
        if is_cratesio_lane() {
            assert!(joined.contains(CRATESIO_RELEASE_LINE), "{joined}");
            assert!(!joined.contains("rebuild:"), "{joined}");
        } else if missing_release_features().is_empty() {
            assert!(joined.contains("release set: complete"), "{joined}");
            assert!(!joined.contains("rebuild:"), "{joined}");
        } else {
            assert!(joined.contains("release set: STALE"), "{joined}");
            assert!(joined.contains(&rebuild_command()), "{joined}");
        }
    }

    /// Issue 225 — the SOURCE lane, both verdicts: byte-identical to the
    /// pre-225 block (STALE + rebuild when incomplete, complete otherwise).
    #[test]
    fn source_lane_block_is_unchanged() {
        let stale = version_lines_for(false, "abcdef123456", &["points", "net-mainnet"]).join("\n");
        assert!(stale.contains("build: abcdef123456"), "{stale}");
        assert!(
            stale.contains("release set: STALE — missing points,net-mainnet"),
            "{stale}"
        );
        assert!(
            stale.contains(&format!("rebuild: {}", rebuild_command())),
            "{stale}"
        );
        assert!(!stale.contains("crates.io"), "{stale}");
        let complete = version_lines_for(false, "abcdef123456", &[]).join("\n");
        assert!(complete.contains("release set: complete ("), "{complete}");
        assert!(!complete.contains("rebuild:"), "{complete}");
    }

    /// Issue 225 — the CRATES.IO lane: the published commit is stamped, the
    /// release line names the lane, and NO STALE verdict or rebuild line
    /// appears even though the release features are absent (they do not
    /// exist in the published manifest — following the advice would fail).
    #[test]
    fn cratesio_lane_block_names_the_lane_and_never_prescribes_a_rebuild() {
        let lines = version_lines_for(true, "0123456789ab", RELEASE_FEATURES);
        assert!(lines[0].starts_with("cargo-refine "));
        assert!(lines[1].starts_with("features: "));
        assert_eq!(lines[2], "build: 0123456789ab");
        let joined = lines.join("\n");
        assert!(joined.contains(CRATESIO_RELEASE_LINE), "{joined}");
        assert!(joined.contains("brew or install.sh"), "{joined}");
        assert!(!joined.contains("STALE"), "{joined}");
        assert!(!joined.contains("rebuild:"), "{joined}");
        assert!(!joined.contains("cargo build"), "{joined}");
    }

    /// Issue 225 — the refusal's remedy follows the lane: a rebuild recipe
    /// on the source lane, the prebuilt binaries on the crates.io lane.
    #[test]
    fn refusal_remedy_follows_the_lane() {
        let src = refusal_lines_for(false, "sync", &["points"]).join("\n");
        assert!(src.contains(&rebuild_command_for(&["points"])), "{src}");
        let reg = refusal_lines_for(true, "sync", &["points"]).join("\n");
        assert!(reg.contains("sync: refused"), "{reg}");
        assert!(reg.contains("prebuilt binaries"), "{reg}");
        assert!(!reg.contains("cargo build"), "{reg}");
    }

    /// The lane stamp is one of exactly two values — a build script that
    /// exported anything else would make `is_cratesio_lane` silently false.
    #[test]
    fn build_lane_is_a_known_value() {
        assert!(
            BUILD_LANE == LANE_CRATESIO || BUILD_LANE == "source",
            "unexpected lane stamp {BUILD_LANE:?}"
        );
    }

    /// The build line (Issue 150 option 3) rides after the feature line and
    /// carries a well-formed stamp. Line 0/1 stay first so anything parsing
    /// the pre-150 output keeps working.
    #[test]
    fn version_block_names_the_build_commit() {
        let lines = version_lines();
        assert_eq!(lines[2], format!("build: {BUILD_COMMIT}"));
        assert!(
            lines[0].starts_with("cargo-refine ") && lines[1].starts_with("features: "),
            "lines 0/1 must stay byte-stable ahead of the build line"
        );
        assert!(
            is_valid_stamp(BUILD_COMMIT),
            "the compiled-in stamp must pass the shape check: {BUILD_COMMIT}"
        );
    }

    /// The stamp validator accepts exactly the shapes `capture_commit` can
    /// produce and refuses the shapes a bug would.
    #[test]
    fn stamp_validator_accepts_exact_shapes_only() {
        assert!(is_valid_stamp("unknown"));
        assert!(is_valid_stamp("abcdef123456"));
        assert!(is_valid_stamp("abcdef123456-dirty"));
        assert!(is_valid_stamp(&"a".repeat(40)));
        // Refuse the shapes a bug would produce.
        assert!(!is_valid_stamp(""));
        assert!(!is_valid_stamp("abc")); // too short to be a --short hash
        assert!(!is_valid_stamp("abcdef12345g")); // not hex
        assert!(!is_valid_stamp("abcdef123456-dirty-x")); // double suffix
        assert!(!is_valid_stamp("unknown-dirty")); // the literal is exact
        assert!(!is_valid_stamp("abcdef123456 dirty")); // space, not suffix
    }

    /// The refusal names the command, the missing feature, the binary's own
    /// set, and the rebuild — all four, because dropping any one of them
    /// recreates a half of the 00:21 diagnosis that had to be done by hand.
    #[test]
    fn refusal_carries_all_four_facts() {
        let text = refusal_lines_for(false, "sync", &["points"]).join("\n");
        assert!(text.contains("sync: refused"), "{text}");
        assert!(text.contains("points"), "{text}");
        assert!(text.contains(COMPILED_FEATURES), "{text}");
        assert!(text.contains(&rebuild_command()), "{text}");
        // Never the shape the class is mistaken for.
        assert!(!text.contains("unknown flag"), "{text}");
    }

    #[test]
    fn refusal_joins_multiple_needed_features() {
        let text = refusal_lines("replay", &["kat_transport", "clippy_verify"]).join("\n");
        assert!(text.contains("kat_transport,clippy_verify"), "{text}");
    }

    /// The remedy must remedy. `replay` needs `clippy_verify`, which the
    /// shipped release set does not carry — a refusal that printed the bare
    /// release recipe would send the user through a full rebuild into the
    /// SAME refusal.
    #[test]
    fn the_rebuild_covers_every_feature_the_refusal_named() {
        let needed = ["kat_transport", "clippy_verify", "latent_retrieval"];
        let cmd = rebuild_command_for(&needed);
        for f in needed {
            assert!(cmd.contains(f), "{cmd} cannot supply {f}");
        }
        for f in RELEASE_FEATURES {
            assert!(cmd.contains(f), "{cmd} dropped the release feature {f}");
        }
        // And the refusal PRINTS that command, not the bare one.
        let text = refusal_lines_for(false, "replay", &needed).join("\n");
        assert!(text.contains(&cmd), "{text}");
    }

    /// An already-covered feature is not duplicated — `--features a,a` is
    /// accepted by cargo but reads as a bug in the tool printing it.
    #[test]
    fn a_needed_feature_already_in_the_release_set_is_not_repeated() {
        let cmd = rebuild_command_for(&["fix_verify"]);
        assert_eq!(cmd.matches("fix_verify").count(), 1, "{cmd}");
        assert_eq!(
            cmd,
            rebuild_command(),
            "no extra feature = the plain recipe"
        );
    }

    #[test]
    fn rebuild_command_names_the_bin_and_the_whole_release_set() {
        let cmd = rebuild_command();
        assert!(cmd.contains("--bin cargo-refine"), "{cmd}");
        assert!(cmd.contains("--release"), "{cmd}");
        for f in RELEASE_FEATURES {
            assert!(cmd.contains(f), "{cmd} is missing {f}");
        }
    }

    /// The typed release set vs the workflow that actually builds the
    /// shipped binary. This is the one constant here that CAN drift, so it
    /// is the one with an arm pointed at its source of truth.
    #[test]
    fn release_set_matches_the_release_workflow() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
        let Ok(text) = std::fs::read_to_string(&path) else {
            // Absent workflow = the instrument cannot see. Say so; never
            // read it as agreement (the UNSEEN-is-not-a-pass rule).
            panic!(
                "release workflow unreadable at {} — this arm asserts nothing without it",
                path.display()
            );
        };
        let from_workflow =
            release_set_from_workflow(&text).unwrap_or_else(|e| panic!("release.yml parse: {e}"));
        assert_eq!(
            from_workflow,
            RELEASE_FEATURES
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>(),
            "RELEASE_FEATURES drifted from .github/workflows/release.yml"
        );
    }

    /// The manual release script is the CI pipeline's local twin — and since
    /// the Actions billing block it is the path EVERY real release takes. A
    /// drifted FEATURES line ships a binary with `upgrade` compiled out (the
    /// exact command the server's 426 `CLIENT_TOO_OLD` text funnels users
    /// toward), so the script is pinned to the same set, plus the two build
    /// claims that make it a pipeline twin (the dist profile and the size
    /// gate the CI package job runs).
    #[test]
    fn release_set_matches_the_manual_script() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/manual_release.sh");
        let Ok(text) = std::fs::read_to_string(&path) else {
            panic!(
                "manual release script unreadable at {} — this arm asserts nothing without it",
                path.display()
            );
        };
        let line = text
            .lines()
            .find(|l| l.starts_with("FEATURES=\""))
            .unwrap_or_else(|| panic!("no FEATURES=\"…\" line in scripts/manual_release.sh"));
        let set = line.trim_start_matches("FEATURES=\"").trim_end_matches('"');
        assert_eq!(
            set.split(',').map(str::to_string).collect::<Vec<_>>(),
            RELEASE_FEATURES
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>(),
            "scripts/manual_release.sh FEATURES drifted from RELEASE_FEATURES — a hand-cut release would ship a stale set"
        );
        for claim in ["--profile dist", "binary_size_gate.sh"] {
            assert!(
                text.contains(claim),
                "scripts/manual_release.sh lost its `{claim}` — it no longer matches the CI pipeline it twins"
            );
        }
    }

    /// The parser's own failure modes are distinguishable from a match —
    /// an extractor that returns an empty set on an unreadable file would
    /// make the arm above pass over nothing.
    #[test]
    fn workflow_parser_refuses_rather_than_guessing() {
        assert!(release_set_from_workflow("nothing relevant here\n").is_err());
        assert!(release_set_from_workflow("  --bin cargo-refine\n").is_err());
        assert!(release_set_from_workflow("  --features --bin cargo-refine\n").is_err());
        let ok = release_set_from_workflow("  cargo build --features a,b --bin cargo-refine \\\n")
            .expect("a well-formed line parses");
        assert_eq!(ok, vec!["a".to_string(), "b".to_string()]);
    }
}
