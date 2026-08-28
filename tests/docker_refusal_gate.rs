//! Plan 030 T0.5(b) + T0.5(c) — the refusal gate and the syntax floor.
//!
//! Proposal 006 caveat 5 names advice that is ANTI-MODERN: a healer that
//! "helpfully" merged consecutive `RUN`s would be actively harmful, because
//! separate RUNs are separate BuildKit cache layers. Prose cannot enforce
//! that. These tests can: they fail the moment a proposer starts emitting
//! a refused shape.
//!
//! The one sanctioned merge — the adjacent `apt-get update` / install pair —
//! is asserted here too, from both sides: it MUST fire on the coupling bug
//! and MUST NOT generalize to any other adjacent RUN pair.

#![cfg(feature = "docker")]

use riir_clippy::domains::docker::proposers::fix_all;
use riir_clippy::domains::docker::rules::{scan_source, ScanContext};
use riir_clippy::domains::docker::{refuses, SyntaxVersion};

fn run_count(src: &str) -> usize {
    riir_clippy::domains::docker::scanner::scan(src)
        .stanzas
        .iter()
        .filter(|s| s.instruction_of(src) == "RUN")
        .count()
}

#[test]
fn dl3059_refused_consecutive_runs_are_never_merged() {
    // Three independent RUNs with nothing coupling them. DL3059 would merge
    // them; we must not touch the count.
    let src = "FROM alpine:3.20\n\
               RUN echo one\n\
               RUN echo two\n\
               RUN echo three\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert_eq!(run_count(&fixed), 3, "the healer consolidated RUNs (DL3059) — REFUSED shape");
    assert_eq!(fixed, src, "a clean multi-RUN file must come back byte-identical");
}

#[test]
fn cache_mount_runs_are_never_merged_either() {
    // The tempting case: two cargo RUNs that "look mergeable". Merging them
    // would destroy exactly the layer boundary that makes the first one
    // cacheable across source edits.
    let src = "FROM rust:1.97.0-slim\n\
               WORKDIR /app\n\
               RUN --mount=type=cache,target=/usr/local/cargo/registry cargo fetch\n\
               RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert_eq!(run_count(&fixed), 2, "cache-bearing RUNs were merged — REFUSED shape");
}

#[test]
fn apt_clean_is_never_emitted() {
    let src = "FROM debian:bookworm-slim\n\
               RUN apt-get update && apt-get install -y --no-install-recommends curl\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert!(
        !fixed.contains("apt-get clean"),
        "emitted the redundant `apt-get clean` — REFUSED (the useful half is DL3009's rm -rf, which we DO emit)"
    );
    assert!(
        fixed.contains("rm -rf /var/lib/apt/lists/*"),
        "the DL3009 half must still be emitted"
    );
}

#[test]
fn labels_are_never_merged() {
    let src = "FROM alpine:3.20\n\
               LABEL a=\"1\"\n\
               LABEL b=\"2\"\n\
               LABEL c=\"3\"\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert_eq!(fixed, src, "LABEL instructions were merged — REFUSED (pre-BuildKit lore)");
}

#[test]
fn qemu_native_build_is_never_rewritten_to_cross() {
    // The in-tree deploy/Dockerfile shape: a QEMU-native alpine builder,
    // deliberately. Both strategies are valid (caveat 6) — annotation only.
    let src = "FROM rust:1.97.0-alpine AS builder\n\
               ARG TARGETPLATFORM\n\
               RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release\n\
               \n\
               FROM scratch\n\
               COPY --from=builder /app/x /x\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert!(
        !fixed.contains("$BUILDPLATFORM"),
        "rewrote a QEMU-native build to a cross build — REFUSED (a decision table, not a defect)"
    );
    assert_eq!(fixed, src);
}

#[test]
fn the_sanctioned_apt_merge_fires_on_the_coupling_bug() {
    // The exception, from the positive side: a separate `apt-get update`
    // layer IS a correctness bug, and this specific adjacent pair merges.
    let src = "FROM debian:bookworm-slim\n\
               RUN apt-get update\n\
               RUN apt-get install --no-install-recommends -y curl \\\n && rm -rf /var/lib/apt/lists/*\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert_eq!(run_count(&fixed), 1, "the apt update/install coupling was not merged");
    assert!(fixed.contains("apt-get update \\\n && apt-get install"));
}

#[test]
fn the_sanctioned_merge_does_not_generalize() {
    // An `apt-get update` followed by a RUN that installs NOTHING must not
    // merge — the exception is scoped to the coupling, not to apt in general.
    let src = "FROM debian:bookworm-slim\n\
               RUN apt-get update\n\
               RUN echo unrelated\n";
    let fixed = fix_all(src, &ScanContext::default());
    assert_eq!(run_count(&fixed), 2, "the apt exception generalized to an unrelated RUN");
}

#[test]
fn refusals_are_declared_as_data() {
    // The refusal list is what the docs cite; it must not silently shrink.
    let names: Vec<&str> = refuses().into_iter().map(|(n, _)| n).collect();
    for want in [
        "consolidate-consecutive-runs",
        "apt-get-clean",
        "merge-label-instructions",
        "rewrite-qemu-native-to-buildplatform-cross",
    ] {
        assert!(names.contains(&want), "the refusal {want} disappeared from the declared list");
    }
}

// ───────────────────── T0.5(c): the syntax floor ─────────────────────

#[test]
fn a_rule_refuses_below_its_declared_syntax_floor() {
    let corpus = riir_clippy::domains::docker::corpus::build_docker_seed_corpus();
    let rule = corpus
        .entries
        .iter()
        .find(|r| r.rule_id == "missing-syntax-directive")
        .expect("the rule");
    assert_eq!(rule.min_syntax, SyntaxVersion::V1_4);
    assert!(!rule.fix_allowed_under(SyntaxVersion::Builtin));
    assert!(rule.fix_allowed_under(SyntaxVersion::V1_4));
}

#[test]
fn syntax_directive_rule_is_scoped_to_files_that_need_it() {
    // Every in-tree Dockerfile omits `# syntax=` and every one is RIGHT to:
    // none uses an above-builtin feature. An unconditional rule would fire
    // on all four and teach the corpus that correct files are buggy.
    let builtin_only = "FROM alpine:3.20\nRUN --mount=type=cache,target=/x echo hi\nCOPY a /a\n";
    let ids: Vec<&str> = scan_source(builtin_only).into_iter().map(|f| f.rule_id).collect();
    assert!(
        !ids.contains(&"missing-syntax-directive"),
        "fired on a file that uses only builtin-frontend features"
    );
    // ...and it DOES fire once the file uses `COPY --link` (1.4).
    let needs_14 = "FROM alpine:3.20\nCOPY --link a /a\n";
    let ids: Vec<&str> = scan_source(needs_14).into_iter().map(|f| f.rule_id).collect();
    assert!(ids.contains(&"missing-syntax-directive"), "did not fire on a --link file");
    // ...and stays silent when the file already pins a sufficient frontend.
    let pinned = "# syntax=docker/dockerfile:1.7\nFROM alpine:3.20\nCOPY --link a /a\n";
    let ids: Vec<&str> = scan_source(pinned).into_iter().map(|f| f.rule_id).collect();
    assert!(!ids.contains(&"missing-syntax-directive"), "fired on an already-pinned file");
}

#[test]
fn pipefail_refuses_on_a_non_bash_base() {
    // alpine's /bin/sh has no pipefail — the "fix" would break the build.
    let alpine = "FROM alpine:3.20\nRUN wget -qO- https://x/y.tgz | tar -xz -C /opt\n";
    let findings = scan_source(alpine);
    let f = findings
        .iter()
        .find(|f| f.rule_id == "pipefail-before-piped-run")
        .expect("the finding still reports");
    assert!(f.fix.is_none(), "emitted a bash SHELL on an alpine base — would break the build");
    assert_eq!(fix_all(alpine, &ScanContext::default()), alpine);
    // On a debian base it IS emitted.
    let debian = "FROM debian:bookworm-slim\nRUN curl -sSL https://x/y.tgz | tar -xz -C /opt\n";
    assert!(fix_all(debian, &ScanContext::default()).contains("pipefail"));
}

#[test]
fn exec_form_refuses_when_the_shell_is_doing_work() {
    for src in [
        "FROM alpine:3.20\nCMD /app --config $CONFIG\n",
        "FROM alpine:3.20\nCMD /app && /other\n",
        "FROM alpine:3.20\nCMD /app | tee /log\n",
        "FROM alpine:3.20\nCMD /app > /log\n",
    ] {
        assert_eq!(
            fix_all(src, &ScanContext::default()),
            src,
            "rewrote a shell-form CMD whose shell is doing real work: {src}"
        );
    }
    // The plain case IS rewritten.
    let plain = "FROM alpine:3.20\nCMD /usr/local/bin/app\n";
    assert!(fix_all(plain, &ScanContext::default()).contains("CMD [\"/usr/local/bin/app\"]"));
}

#[test]
fn add_refuses_urls_and_archives() {
    for src in [
        "FROM alpine:3.20\nADD https://example.com/x.txt /x.txt\n",
        "FROM alpine:3.20\nADD app.tar.gz /opt/\n",
        "FROM alpine:3.20\nADD --checksum=sha256:abc https://e/x /x\n",
    ] {
        assert_eq!(
            fix_all(src, &ScanContext::default()),
            src,
            "rewrote an ADD that does something COPY cannot: {src}"
        );
    }
    let local = "FROM alpine:3.20\nADD ./app /usr/local/bin/app\n";
    assert!(fix_all(local, &ScanContext::default()).contains("COPY ./app"));
}
