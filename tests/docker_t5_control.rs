//! Plan 030 T0.2/T0.5 + T1.5 G1 — the docker domain's control gates.
//!
//! Three things are pinned here, and all three are about the scanner and
//! the rules being WRONG in the safe direction:
//!
//! 1. **The T5 control** (the Issue 025 WGSL-scan precedent): the scanner's
//!    structural reading of the four real in-tree Dockerfiles, pinned
//!    BEFORE any rule interpretation. If stanza spans drift, this fails
//!    first and the rule results become uninterpretable.
//! 2. **The negative controls**: the two known-good production Dockerfiles
//!    (`riir-chain/deploy`, mmorpg `authority-container`) must yield ZERO
//!    **fixable** findings. Nonzero falsifies the proposers.
//! 3. **The G5-real target**: `edge-wallet-container` must yield the
//!    findings the plan predicted — a rule set that finds nothing on the
//!    file it was built to heal is not conservative, it is broken.
//!
//! ## Sibling-repo files, honestly handled
//!
//! The real Dockerfiles live in sibling repos. When a sibling is absent
//! (a fresh clone, a worktree, CI without the neighbours) the file-backed
//! assertions SKIP with a printed line rather than pass silently — the
//! oracle-degradation discipline. The embedded structural fixtures below
//! always run, so the scanner is never fully unpinned.

#![cfg(feature = "docker")]

use riir_clippy::domains::docker::rules::{scan_source, scan_source_with, ScanContext, FIXABLE_DOCKER};
use riir_clippy::domains::docker::scanner::{self, Scope};

/// The four in-tree Dockerfiles, as sibling-relative paths.
const IN_TREE: &[(&str, &str)] = &[
    ("deploy", "../riir-chain/deploy/Dockerfile"),
    (
        "edge-wallet",
        "../riir-chain/cloudflare/edge-wallet-container/Dockerfile",
    ),
    (
        "authority",
        "../riir-mmorpg-examples/cloudflare/authority-container/Dockerfile",
    ),
    (
        "tick-test",
        "../riir-mmorpg-examples/cloudflare/container-tick-test/Dockerfile",
    ),
];

fn read(name: &str) -> Option<String> {
    let path = IN_TREE.iter().find(|(n, _)| *n == name)?.1;
    if let Ok(s) = std::fs::read_to_string(path) { Some(s) } else {
            println!("SKIP {name}: sibling repo absent at {path} (degrading, not passing silently)");
            None
        }
}

/// The origin-resolved view of a sibling file: the pins below pin
/// origin/develop, not whatever state the LOCAL sibling clone happens to
/// ride (a local rewind must not flip these gates — the T5 control's
/// contract is against the tree the workspace ships, and origin IS that
/// tree; the 2026-08-31 edge-wallet local rewind is the live instance).
/// Falls back to the on-disk file when git cannot resolve the path (a
/// fresh clone with no origin yet degrades to the old local behavior).
fn read_pinned(name: &str) -> Option<String> {
    let path = IN_TREE.iter().find(|(n, _)| *n == name)?.1;
    // Split "../<repo>/<dir>/<file>" into the file's own directory + the
    // bare file name. `git show <rev>:<path>` resolves <path> relative to
    // the process CWD, so from the file's own directory the `./<file>`
    // form is the one that resolves (the repo-root-relative form 404s from
    // a subdir — measured, error 128).
    let (repo_dir, file_name) = path.rsplit_once('/').map(|(d, f)| (d, f.rsplit_once('/').map_or(f, |(_, f)| f)))?;
    let Ok(out) = std::process::Command::new("git")
        .args(["show", &format!("origin/develop:./{file_name}")])
        .current_dir(repo_dir)
        .output()
    else {
        return read(name);
    };
    if out.status.success() && !out.stdout.is_empty() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        read(name)
    }
}

// ─────────────────────── 1. the T5 scanner control ───────────────────────

#[test]
fn t5_control_scanner_structure_on_in_tree_files() {
    // (name, stages, stanzas, first-stage base)
    let expected: &[(&str, usize, usize, &str)] = &[
        // Counts independently verified against
        // `grep -cE '^[A-Za-z]+ ' <file>` at pin time (2026-08-28) — the
        // scanner is not allowed to define its own ground truth here.
        // base 1.97.0 -> 1.98.1 on 2026-09-09 (Batch 140 rider): re-measured
        // against origin/develop — `438b4d44` (09-07, the engram-producer
        // lane) bumped the builder tag as a rider; stages 2 + stanzas 18
        // re-verified unchanged, only the tag moved. The pin had been red
        // for any fetcher since 09-07 with the owning lane gone.
        ("deploy", 2, 18, "rust:1.98.1-alpine"),
        // 9 -> 11 on 2026-08-28: riir-chain ADOPTED the healer's fixes
        // (Plan 030 T1.6), adding the `SHELL` pipefail stanza and a
        // runtime-stage `USER`. Re-verified with `grep -cE '^[A-Za-z]+ '`.
        // 11 -> 9 on 2026-08-31 — the LOCAL sibling clone was REWOUND to the
        // pre-adoption shape (9 stanzas, all four fixable findings back);
        // origin/develop still carries the healed 11-stanza file. The pin
        // here pins origin (`git show origin/develop:<file>`); the local
        // rewind is recorded in the snapshot (its riir-chain is behind 33
        // commits and was rolled back from `d0bc21b` era by the sibling).
        // Until the sibling's clone re-syncs, the file-backed assertions
        // SKIP via `read()` — the documented degradation path.
        ("edge-wallet", 2, 11, "rust:1.97.0-slim"),
        ("authority", 2, 11, "rust:1-slim"),
        ("tick-test", 2, 9, "rust:1-slim"),
    ];
    for (name, stages, stanzas, base) in expected {
        let Some(src) = read_pinned(name) else { continue };
        let scan = scanner::scan(&src);
        assert_eq!(scan.stages.len(), *stages, "{name}: stage count drifted");
        assert_eq!(scan.stanzas.len(), *stanzas, "{name}: stanza count drifted");
        assert_eq!(scan.stages[0].base, *base, "{name}: first base drifted");
        // Every stanza's span must slice cleanly and start with its keyword.
        for st in &scan.stanzas {
            let text = st.text(&src);
            assert!(
                text.starts_with(st.keyword(&src)),
                "{name}: stanza span does not start at its keyword ({})",
                st.line
            );
        }
    }
}

#[test]
fn t5_control_multiline_run_spans_the_whole_continuation() {
    // deploy/Dockerfile's build RUN is a 9-line continuation carrying a
    // shell `case` — the hardest span in the tree.
    let Some(src) = read("deploy") else { return };
    let scan = scanner::scan(&src);
    let run = scan
        .stanzas
        .iter()
        .find(|s| s.instruction_of(&src) == "RUN" && s.text(&src).contains("cargo build"))
        .expect("the build RUN");
    assert!(
        run.end_line > run.line + 5,
        "the build RUN collapsed to {} lines — continuations are not being followed",
        run.end_line - run.line + 1
    );
    let text = run.text(&src);
    assert!(text.contains("--mount=type=cache"), "flags lost from the span");
    assert!(text.contains("cp \"target/"), "the span stops before its last line");
}

#[test]
fn t5_control_global_scope_is_pre_from_only() {
    for (name, _) in IN_TREE {
        let Some(src) = read(name) else { continue };
        let scan = scanner::scan(&src);
        for st in &scan.stanzas {
            if st.scope == Scope::Global {
                assert_eq!(
                    st.instruction_of(&src),
                    "ARG",
                    "{name}: non-ARG instruction in global scope at line {}",
                    st.line
                );
            }
        }
    }
}

#[test]
fn t5_control_comment_blocks_attach_to_their_stanza() {
    // deploy/Dockerfile is comment-rich and the comments are load-bearing
    // retrieval context (Proposal 006).
    let Some(src) = read("deploy") else { return };
    let scan = scanner::scan(&src);
    let from = scan
        .stanzas
        .iter()
        .find(|s| s.instruction_of(&src) == "FROM")
        .expect("the builder FROM");
    let start = from.comment_start.expect("the builder FROM carries a comment block");
    assert!(start < from.start);
    assert!(
        src[start..from.start].contains("Builder"),
        "the attached block is not the one directly above"
    );
}

// ───────────────────── 2. the negative controls ──────────────────────

#[test]
fn negative_control_no_fixable_findings_on_known_good_files() {
    for name in ["deploy", "authority"] {
        let Some(src) = read(name) else { continue };
        let fixable: Vec<&str> = scan_source(&src)
            .into_iter()
            .filter(|f| FIXABLE_DOCKER.contains(&f.rule_id))
            .map(|f| f.rule_id)
            .collect();
        assert!(
            fixable.is_empty(),
            "{name} is a known-good production Dockerfile but the proposers want to edit it: {fixable:?}"
        );
    }
}

/// The measured suggest-only findings on the negative controls, pinned.
///
/// The two files are clean of FIXES, not of every citation — and that
/// distinction is the honest form of the control. `authority-container`
/// carries one real suggest-only finding (its distroless runtime declares
/// no `USER`, so it runs as root; distroless ships `:nonroot` variants),
/// which is a genuine observation about a good file, not a defect claim
/// against it. Pinned so it cannot silently grow.
#[test]
fn negative_control_suggest_only_findings_are_pinned() {
    let expected: &[(&str, &[&str])] = &[
        // Plan 030 T2.1(b) added a suggest-only family, and the two controls
        // discriminate as designed: `deploy` is musl-on-scratch, so it draws
        // the allocator annotation and NOT the digest or --link citations
        // (scratch has no image to pin and no parent filesystem to rebase
        // onto). `authority` draws both of those and not the allocator one —
        // it is `gcr.io/distroless/cc-debian13`, a real registry image with a
        // real parent filesystem, and it is not a musl build. That corrected
        // a prediction: `distroless-distro-suffix-pin` was expected to absorb
        // the digest citation, but it only fires on a base MISSING the
        // `-debianNN` suffix, and this one carries it. Measured, then pinned.
        ("deploy", &["musl-allocator-perf-annotation"]),
        (
            "authority",
            &[
                "base-tag-not-digest-pinned",
                "copy-link-artifact-rebase",
                "runtime-stage-missing-user",
            ],
        ),
    ];
    for (name, want) in expected {
        let Some(src) = read(name) else { continue };
        let mut got: Vec<&str> = scan_source(&src).into_iter().map(|f| f.rule_id).collect();
        got.sort_unstable();
        got.dedup();
        assert_eq!(&got, want, "{name}: suggest-only finding set drifted");
    }
}

// ───────────────────── 3. the G5-real target ──────────────────────

/// The pre-adoption shape of `edge-wallet-container/Dockerfile`, reduced to
/// the three spans the Plan 030 T1.5 prediction named: a piped `RUN`, an
/// uncached `cargo build`, and a runtime stage with no `USER`. It is a
/// REDUCTION, not a copy — the real file's stub-manifest `printf` payload is
/// ~2 KB of irrelevant TOML — and it exists because the live file no longer
/// carries these defects: riir-chain adopted the fixes (Plan 030 T1.6,
/// 2026-08-28). Without it, adoption would silently erase the evidence that
/// the rules ever fired on the file they were built to heal.
const EDGE_WALLET_PRE_ADOPTION: &str = "\
FROM rust:1.97.0-slim AS builder
WORKDIR /app
COPY .container-src/ ./

RUN mkdir -p /riir-neuron-db/src \\
 && echo '// stub' \\
    | tee /riir-neuron-db/src/lib.rs > /dev/null

RUN cargo build --release -p riir-wallet --example edge_wallet_agent --features native

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/examples/edge_wallet_agent /usr/local/bin/edge_wallet_agent
EXPOSE 8080
CMD [\"/usr/local/bin/edge_wallet_agent\"]
";

#[test]
fn g5_prediction_holds_on_the_pre_adoption_content() {
    let mut got: Vec<&str> = scan_source(EDGE_WALLET_PRE_ADOPTION)
        .into_iter()
        .map(|f| f.rule_id)
        .collect();
    got.sort_unstable();
    got.dedup();
    // Predicted by Plan 030 T1.5 before the run: the pipe in the stub-
    // manifest RUN (hadolint agrees — DL4006 at that line), the uncached
    // cargo build, and the root runtime.
    for want in [
        "pipefail-before-piped-run",
        "cache-mount-missing-on-cargo-build",
        "runtime-stage-missing-user",
    ] {
        assert!(got.contains(&want), "the pre-adoption content lost the {want} finding (got {got:?})");
    }
}

/// The T1.6 adoption gate, from riir-clippy's side: the consumer file the
/// healer was built for must STAY healed. A regression here means somebody
/// re-introduced a defect the healer already fixed in `riir-chain`
/// (`cloudflare/edge-wallet-container/Dockerfile`), and the consumer repo's
/// own `scripts/dockerfile_heal_gate.sh` is the other half of this pin.
#[test]
fn edge_wallet_stays_healed_after_adoption() {
    let Some(src) = read_pinned("edge-wallet") else { return };
    let fixable: Vec<&str> = scan_source(&src)
        .into_iter()
        .filter(|f| FIXABLE_DOCKER.contains(&f.rule_id))
        .map(|f| f.rule_id)
        .collect();
    assert!(
        fixable.is_empty(),
        "edge-wallet regressed — bounded findings are back: {fixable:?}"
    );
    // The suggest-only set is pinned too, so a NEW citation is visible
    // rather than absorbed. `runtime-stage-missing-user` is gone because the
    // adoption took that citation (USER 65534:65534, measured serving).
    let mut suggest: Vec<&str> = scan_source(&src).into_iter().map(|f| f.rule_id).collect();
    suggest.sort_unstable();
    suggest.dedup();
    // Plan 030 T2.1(b), 2026-08-29: of the two new citations this file drew,
    // `copy-link-artifact-rebase` was TAKEN (the file now carries
    // `COPY --link` + the `# syntax=` line it requires; rebuilt and verified
    // serving /health as uid 65534), so it is gone. `base-tag-not-digest-pinned`
    // stands — see the reason in riir-chain's dockerfile_heal_gate.sh: a digest
    // is reproducible but frozen, and with no renovate/dependabot here, an
    // un-bumped base accumulating CVEs is the worse risk for an image holding
    // a signing key.
    assert_eq!(
        &suggest,
        &["base-tag-not-digest-pinned"],
        "edge-wallet: suggest-only finding set drifted"
    );
}

#[test]
fn g5_target_fixes_keep_the_file_scannable() {
    let Some(src) = read("edge-wallet") else { return };
    let fixed = riir_clippy::domains::docker::proposers::fix_all(&src, &ScanContext::default());
    let scan = scanner::scan(&fixed);
    assert_eq!(scan.stages.len(), 2, "the fix changed the stage graph");
    assert!(
        fixed.contains("--mount=type=cache,target=/usr/local/cargo/registry"),
        "the cache mount was not applied"
    );
    assert!(
        fixed.contains("SHELL [\"/bin/bash\", \"-o\", \"pipefail\", \"-c\"]"),
        "the pipefail SHELL was not applied"
    );
    // Idempotence: re-running finds nothing left to fix.
    let twice = riir_clippy::domains::docker::proposers::fix_all(&fixed, &ScanContext::default());
    assert_eq!(twice, fixed, "the fix is not idempotent");
}

// ───────────────────── caller-gate discipline ──────────────────────

#[test]
fn gated_rules_stay_silent_without_evidence() {
    // The gated rules are exactly the ones that would otherwise fire on the
    // known-good files. Silent without evidence; firing with it.
    let src = "FROM rust:1.97.0-slim AS b\nWORKDIR /app\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=b /app/x /x\nUSER 10001\nCMD [\"/x\"]\n";
    let bare: Vec<&str> = scan_source(src).into_iter().map(|f| f.rule_id).collect();
    for gated in [
        "cargo-locked-flag",
        "workspace-single-member-build",
        "dockerignore-missing",
        "build-stage-missing-buildplatform",
    ] {
        assert!(
            !bare.contains(&gated),
            "{gated} fired on a text-only scan — it must require caller evidence"
        );
    }
    let ctx = ScanContext {
        lockfile_committed: Some(true),
        workspace_multi_member: Some(true),
        dockerignore_present: Some(false),
        multi_platform_build: Some(true),
    };
    let armed: Vec<&str> = scan_source_with(src, &ctx)
        .into_iter()
        .map(|f| f.rule_id)
        .collect();
    for gated in [
        "cargo-locked-flag",
        "workspace-single-member-build",
        "dockerignore-missing",
        "build-stage-missing-buildplatform",
    ] {
        assert!(armed.contains(&gated), "{gated} did not fire even WITH its evidence");
    }
}
