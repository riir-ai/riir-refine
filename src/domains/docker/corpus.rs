//! The docker seed corpus — Phase 1 (Plan 030, Proposal 006).
//!
//! Every rule was checked against the LIVE incumbents at authoring time
//! (hadolint 2.15.1 + `docker build --check`, 2026-08-28) rather than
//! against the `Research` 086 snapshot — the standing mine-time rule. The
//! dedup verdict rides in each rule's `incumbent` field so it cannot drift
//! away from the rule it describes.
//!
//! | Rule | Family | Class | Fix | Incumbent |
//! |---|---|---|---|---|
//! | `apt-lists-not-removed` | `PkgHygiene` | A | bounded | DL3009 |
//! | `apt-no-install-recommends` | `PkgHygiene` | A | bounded | DL3015 |
//! | `apk-add-no-cache` | `PkgHygiene` | A | bounded | DL3019 |
//! | `add-local-path-to-copy` | `RunCorrectness` | A | bounded | DL3020 |
//! | `cmd-entrypoint-exec-form` | `SignalPid1` | A | bounded | DL3025 / `JSONArgsRecommended` |
//! | `apt-not-apt-get` | `PkgHygiene` | A | bounded | DL3027 |
//! | `maintainer-to-label` | `RunCorrectness` | A | bounded | DL4000 |
//! | `pipefail-before-piped-run` | `RunCorrectness` | A | bounded (debian only) | DL4006 |
//! | `workdir-absolute-path` | `RunCorrectness` | A | bounded | DL3000 |
//! | `instruction-casing` | `RunCorrectness` | A | bounded | `ConsistentInstructionCasing` |
//! | `legacy-key-value-format` | `RunCorrectness` | A | bounded | `LegacyKeyValueFormat` |
//! | `no-empty-continuation` | `RunCorrectness` | A | suggest | `NoEmptyContinuation` |
//! | `apt-update-install-same-run` | `CachePerf` | **B** | bounded | none |
//! | `cache-mount-missing-on-cargo-build` | `CachePerf` | **B** | bounded | none |
//! | `missing-syntax-directive` | `CachePerf` | **B** | bounded | none |
//! | `cargo-locked-flag` | `RustIdiom` | **B** | suggest (gated) | none |
//! | `workspace-single-member-build` | `RustIdiom` | **B** | suggest (gated) | none |
//! | `dockerignore-missing` | `CachePerf` | **B** | suggest (gated) | none |
//! | `arg-secret-to-secret-mount` | `SecretsHygiene` | A | suggest | DL3064 |
//! | `manifest-first-layer-split` | `CachePerf` | **B** | suggest | none |
//! | `cache-mount-id-scope` | `CachePerf` | **B** | suggest | none |
//! | `entrypoint-shell-form-signal-block` | `SignalPid1` | A | suggest | DL3025 |
//! | `build-stage-missing-buildplatform` | `MultiPlatform` | **B** | annotate | none |
//! | `distroless-distro-suffix-pin` | `BaseChoice` | **B** | suggest | none |
//! | `musl-scratch-ca-certs` | `BaseChoice` | **B** | suggest | none |
//! | `runtime-stage-missing-user` | `BaseChoice` | **B** | suggest | none |
//! | `chef-stage-toolchain-skew` | `RustIdiom` | **B** | suggest | none |
//!
//! ## Killed at mine time (recorded, not silently dropped)
//!
//! - **`consolidate-consecutive-runs` (DL3059)** — killed as ANTI-MODERN
//!   and promoted to an explicit refusal ([`super::refuses`]).
//! - **`apt-get-clean`** — redundant under the default `docker-clean`
//!   config; the useful half is DL3009, which we fix. Refusal.
//! - **`merge-label-instructions`** — pre-`BuildKit` lore. Refusal.
//! - **`pin-package-versions` (DL3008/DL3018)** — hadolint detects and the
//!   in-tree files trip it, but there is NO bounded fix: the correct
//!   version is a registry fact we refuse to look up (no network calls).
//!   Suggest-only would duplicate hadolint's own message verbatim, so it is
//!   dropped rather than shipped as an echo.
//! - **`tag-the-base-image` (DL3006)** — same reason, plus renovate/
//!   dependabot own version auditing (Proposal 006 §NOT shipped).
//! - **`copy-from-references-defined-stage` (DL3022)** — hadolint FIRES A
//!   FALSE POSITIVE here: `riir-chain/deploy/Dockerfile` uses `BuildKit`
//!   **named contexts** (`--build-context riir-ai=../riir-ai`), which
//!   hadolint cannot see, so it reports two phantom findings. We must not
//!   reimplement a detector that is wrong on our own tree.
//! - **`qemu-native-to-buildplatform-cross`** — both strategies measured
//!   valid; annotation only (refusal against rewriting).
//!
//! ## Reclassified at mine time (the T1.1 live re-verification)
//!
//! Two rules authored as Class B turned out to have live incumbents that
//! the `Research` 086 snapshot did not carry — found by running the dedup
//! check against hadolint 2.15.1 rather than trusting the note:
//! `arg-secret-to-secret-mount` (DL3064) and
//! `entrypoint-shell-form-signal-block` (DL3025, which covers ENTRYPOINT
//! as well as CMD). Both stay in the corpus — the fix side is still
//! unoccupied — but their novelty claim was false and the ledger says so.
//!
//! A third apparent overlap was investigated and REFUTED:
//! `apt-update-install-same-run` looked like it collided with DL3009, but
//! that was a fixture artifact (a bare `apt-get update` RUN necessarily
//! also trips the apt-lists rule). Probed directly — a split where the
//! update RUN already deletes its lists — hadolint is entirely SILENT, so
//! the cache-staleness axis is genuinely unoccupied, as `Research` 086 said.

use super::{DockerFamily, DockerRule, DockerDomain, SyntaxVersion};
use crate::corpus::Corpus;

/// Build the docker seed corpus (BLAKE3-committed via `Corpus<D>`).
#[must_use]
pub fn build_docker_seed_corpus() -> Corpus<DockerDomain> {
    Corpus::new(entries())
}

/// Shorthand constructor — the corpus is data, and 27 struct literals of
/// nine fields each is noise that hides the rules. The argument count is
/// the point (one per rule field), so the clippy arity lint is silenced
/// here rather than obeyed by inventing a builder for a private fn.
#[allow(clippy::too_many_arguments)]
fn rule(
    rule_id: &str,
    docker_concept: &str,
    trigger: &str,
    buggy_code: &str,
    fixed_code: &str,
    family: DockerFamily,
    min_syntax: SyntaxVersion,
    incumbent: Option<&str>,
    notes: Option<&str>,
) -> DockerRule {
    DockerRule {
        rule_id: rule_id.to_string(),
        docker_concept: docker_concept.to_string(),
        trigger: trigger.to_string(),
        buggy_code: buggy_code.to_string(),
        fixed_code: fixed_code.to_string(),
        family,
        min_syntax,
        incumbent: incumbent.map(str::to_string),
        notes: notes.map(str::to_string),
    }
}

fn entries() -> Vec<DockerRule> {
    use DockerFamily as F;
    use SyntaxVersion as S;
    vec![
        // ────────────────── Class A — fix overlay ──────────────────
        rule(
            "apt-lists-not-removed",
            "an apt install that keeps /var/lib/apt/lists bakes the package index into the layer forever — it is dead weight in every pull",
            "a RUN containing `apt-get install` with no `rm -rf /var/lib/apt/lists/*` and no apt cache mount in the same RUN",
            "FROM debian:bookworm-slim\nRUN apt-get update && apt-get install -y --no-install-recommends curl\n",
            "FROM debian:bookworm-slim\nRUN apt-get update && apt-get install -y --no-install-recommends curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            F::PkgHygiene, S::Builtin, Some("DL3009"), None,
        ),
        rule(
            "apt-no-install-recommends",
            "apt pulls recommended packages by default — in a container that is tens of megabytes nobody asked for",
            "a RUN containing `apt-get install` without `--no-install-recommends`",
            "FROM debian:bookworm-slim\nRUN apt-get install -y curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            "FROM debian:bookworm-slim\nRUN apt-get install --no-install-recommends -y curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            F::PkgHygiene, S::Builtin, Some("DL3015"), None,
        ),
        rule(
            "apk-add-no-cache",
            "apk keeps its index unless told not to — `--no-cache` is the one-flag equivalent of apt's rm -rf dance",
            "a RUN containing `apk add` without `--no-cache`",
            "FROM alpine:3.20\nRUN apk add build-base\n",
            "FROM alpine:3.20\nRUN apk add --no-cache build-base\n",
            F::PkgHygiene, S::Builtin, Some("DL3019"), None,
        ),
        rule(
            "add-local-path-to-copy",
            "ADD silently unpacks archives and fetches URLs; for a plain local path COPY is the instruction that does exactly what it says",
            "an ADD whose operands are neither a URL nor an archive, with no --checksum",
            "FROM scratch\nADD ./app /usr/local/bin/app\n",
            "FROM scratch\nCOPY ./app /usr/local/bin/app\n",
            F::RunCorrectness, S::Builtin, Some("DL3020"), None,
        ),
        rule(
            "cmd-entrypoint-exec-form",
            "shell-form CMD runs the process as a CHILD of `sh -c`, so it never receives SIGTERM — the container is killed instead of stopped",
            "a CMD/ENTRYPOINT whose args are not a JSON array and carry no shell metacharacter",
            "FROM scratch\nCMD /usr/local/bin/app\n",
            "FROM scratch\nCMD [\"/usr/local/bin/app\"]\n",
            F::SignalPid1, S::Builtin, Some("DL3025"),
            Some("Behavior-adjacent: exec form changes WHO receives the signal. The fix refuses when the command carries a pipe, &&, redirect, glob or variable — there the shell is doing real work."),
        ),
        rule(
            "apt-not-apt-get",
            "`apt` is the interactive CLI and warns that it has no stable interface for scripts; `apt-get` is the scripting one",
            "a RUN containing `apt install` or `apt upgrade`",
            "FROM debian:bookworm-slim\nRUN apt install --no-install-recommends -y curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            "FROM debian:bookworm-slim\nRUN apt-get install --no-install-recommends -y curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            F::PkgHygiene, S::Builtin, Some("DL3027"), None,
        ),
        rule(
            "maintainer-to-label",
            "MAINTAINER is deprecated; LABEL maintainer= is the metadata channel that survives",
            "a MAINTAINER instruction",
            "FROM scratch\nMAINTAINER ops@example.com\n",
            "FROM scratch\nLABEL maintainer=\"ops@example.com\"\n",
            F::RunCorrectness, S::Builtin, Some("DL4000"), None,
        ),
        rule(
            "pipefail-before-piped-run",
            "in a pipeline only the LAST command's exit status survives, so a failed download followed by a successful tar builds a broken image and reports success",
            "a RUN containing a pipe, in a stage with no SHELL pipefail set",
            "FROM debian:bookworm-slim\nRUN curl -sSL https://example.com/x.tgz | tar -xz -C /opt\n",
            "FROM debian:bookworm-slim\nSHELL [\"/bin/bash\", \"-o\", \"pipefail\", \"-c\"]\nRUN curl -sSL https://example.com/x.tgz | tar -xz -C /opt\n",
            F::RunCorrectness, S::Builtin, Some("DL4006"),
            Some("The SHELL insert is emitted ONLY for a debian-family base. On alpine/busybox /bin/sh has no pipefail and the insert would break the build — hadolint says the same; there the finding ships without a fix."),
        ),
        rule(
            "workdir-absolute-path",
            "a relative WORKDIR resolves against whatever directory the base image happened to leave behind — the build is then base-image-order dependent",
            "a WORKDIR whose path starts with neither `/` nor a variable",
            "FROM rust:1.97.0-slim\nWORKDIR app\n",
            "FROM rust:1.97.0-slim\nWORKDIR /app\n",
            F::RunCorrectness, S::Builtin, Some("DL3000"), None,
        ),
        rule(
            "instruction-casing",
            "mixed instruction casing is the loudest tell of a machine-written Dockerfile and BuildKit warns on it by default",
            "an instruction keyword carrying a lowercase letter",
            "FROM alpine:3.20\nrun echo hi\n",
            "FROM alpine:3.20\nRUN echo hi\n",
            F::RunCorrectness, S::Builtin, Some("ConsistentInstructionCasing"), None,
        ),
        rule(
            "legacy-key-value-format",
            "`ENV key value` is the pre-1.0 form; the space-separated variant silently swallows the rest of the line as one value",
            "an ENV/LABEL with no `=` and two whitespace-separated operands",
            "FROM alpine:3.20\nENV RUST_LOG info\n",
            "FROM alpine:3.20\nENV RUST_LOG=info\n",
            F::RunCorrectness, S::Builtin, Some("LegacyKeyValueFormat"), None,
        ),
        rule(
            "no-empty-continuation",
            "a line continuation followed by a blank line is DROPPED by the parser — what the author wrote and what the build runs diverge silently",
            "a stanza line ending in the escape character whose successor line is empty",
            "FROM alpine:3.20\nRUN echo a \\\n\n  && echo b\n",
            "FROM alpine:3.20\nRUN echo a \\\n  && echo b\n",
            F::RunCorrectness, S::Builtin, Some("NoEmptyContinuation"),
            Some("Suggest-only: which side of the blank line the author meant to keep is not derivable from the text."),
        ),
        // ────────────────── Class B — novel detection ──────────────────
        rule(
            "apt-update-install-same-run",
            "a cached `apt-get update` layer pairs STALE package lists with a fresh install — the canonical cache-busting bug, and it fails weeks after the Dockerfile was written",
            "a RUN containing only `apt-get update`, immediately followed by a RUN that installs, in the same stage",
            "FROM debian:bookworm-slim\nRUN apt-get update\nRUN apt-get install --no-install-recommends -y curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            "FROM debian:bookworm-slim\nRUN apt-get update \\\n && apt-get install --no-install-recommends -y curl \\\n && rm -rf /var/lib/apt/lists/*\n",
            F::CachePerf, S::Builtin, None,
            Some("This is the ONE sanctioned RUN merge and it is scoped to the adjacent apt update/install pair. It is not a DL3059 consolidation: the reason is cache CORRECTNESS, not layer count. See the refusal gate."),
        ),
        rule(
            "cache-mount-missing-on-cargo-build",
            "without a cache mount every source edit re-downloads and recompiles the whole dependency graph — the single largest avoidable cost in a Rust image build",
            "a RUN containing `cargo build`/`cargo install` with no `--mount=type=cache` anywhere in the stanza",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\n",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nCOPY . .\nRUN --mount=type=cache,target=/usr/local/cargo/registry \\\n    --mount=type=cache,target=/usr/local/cargo/git \\\n    cargo build --release\n",
            F::CachePerf, S::Builtin, None,
            Some("Only the two CARGO_HOME mounts are emitted — they are path-independent in the official rust images. The target-dir mount is deliberately NOT emitted: its path depends on WORKDIR, and a cache mount over the wrong path silently caches nothing. Note also that a target cache mount is NOT part of the image, so the binary must be copied out within the same RUN."),
        ),
        rule(
            "missing-syntax-directive",
            "a file using an above-builtin frontend feature without pinning `# syntax=` builds differently depending on the daemon it lands on",
            "a heredoc, `COPY --link` or `COPY --exclude` in a file whose `# syntax=` is absent or below the feature's floor",
            "FROM alpine:3.20\nCOPY --link ./app /app\n",
            "# syntax=docker/dockerfile:1.4\nFROM alpine:3.20\nCOPY --link ./app /app\n",
            F::CachePerf, S::V1_4, None,
            Some("Scoped HARD to files that actually USE an above-builtin feature. An unconditional version of this rule fires on every correct in-tree Dockerfile — all four omit the directive and all four are right to."),
        ),
        rule(
            "cargo-locked-flag",
            "without `--locked` the build silently re-resolves the dependency graph, so the image does not match the lockfile the repo tested",
            "a `cargo build` without `--locked`, ONLY when the caller supplies evidence that a lockfile is committed",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nRUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release\n",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nRUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --locked --release\n",
            F::RustIdiom, S::Builtin, None,
            Some("CALLER-GATED. `--locked` fails the build outright when no lockfile is committed, so a text-only scan never proposes it. With the evidence supplied it is bounded and exact."),
        ),
        rule(
            "workspace-single-member-build",
            "`cargo build` at a workspace root builds every member and resolves features against the ROOT — the deploy artefact then carries crates it never needed, or fails on a feature the root does not declare",
            "a `cargo build` with no `-p`/`--package`/`--bin`, ONLY when the caller reports a multi-member workspace",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nRUN cargo build --release --features authority\n",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nRUN cargo build --release -p my-daemon --bin my-daemon --features authority\n",
            F::RustIdiom, S::Builtin, None,
            Some("CALLER-GATED + suggest-only: which member to build is not in the Dockerfile. This is riir-chain deploy/Dockerfile's own documented lesson — `-p riir-chaind` is load-bearing there because the wallet features belong to the DAEMON package, not the workspace root."),
        ),
        rule(
            "dockerignore-missing",
            "with no .dockerignore the whole tree ships to the daemon — .git and target/ included; the measured in-tree case was a 184 MB context",
            "a build context directory with no .dockerignore (caller-supplied evidence)",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nCOPY . .\n",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nCOPY . .\n",
            F::CachePerf, S::Builtin, None,
            Some("CALLER-GATED, and the fix is a NEW FILE, not a Dockerfile edit — so it is suggest-only by construction. Fixture pair is intentionally identical: the defect is outside the file."),
        ),
        rule(
            "arg-secret-to-secret-mount",
            "a build ARG is recorded in the image history — anyone with the image can read it back; `RUN --mount=type=secret` never enters a layer",
            "an ARG whose NAME contains SECRET/TOKEN/PASSWORD/API_KEY/PRIVATE_KEY",
            "FROM alpine:3.20\nARG NPM_TOKEN\nRUN echo \"//r.example/:_authToken=$NPM_TOKEN\" > ~/.npmrc\n",
            "FROM alpine:3.20\nRUN --mount=type=secret,id=npm_token \\\n    echo \"//r.example/:_authToken=$(cat /run/secrets/npm_token)\" > ~/.npmrc\n",
            F::SecretsHygiene, S::Builtin, Some("DL3064"),
            Some("RECLASSIFIED Class B -> Class A at mine time (2026-08-28): the live hadolint DOES detect this as DL3064 (\"Potentially sensitive data should not be used in the ARG or ENV commands\"), which the Research 086 snapshot did not carry. The delta is still real — DL3064 detects and does not fix — but the novelty claim was wrong and is corrected here rather than in prose. Suggest-only: the fix requires the CALLER to pass `--secret id=npm_token,...` at `docker build` time. Rewriting the Dockerfile alone would break the build — the caller-gate class."),
        ),
        rule(
            "manifest-first-layer-split",
            "`COPY . .` before the build makes every source edit invalidate the dependency layer; copying manifests first lets the dependency compile survive source churn",
            "a `cargo build` RUN in a stage that copied the whole context before it",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\n",
            "FROM rust:1.97.0-slim\nWORKDIR /app\nCOPY Cargo.toml Cargo.lock ./\nRUN mkdir -p src && echo 'fn main() {}' > src/main.rs && cargo build --release\nCOPY . .\nRUN cargo build --release\n",
            F::CachePerf, S::Builtin, None,
            Some("Suggest-only: the correct split depends on the crate layout (workspace members, build scripts, non-Rust assets). cargo-chef does this properly and is the recommended target; the stub-main variant shown is the dependency-free approximation."),
        ),
        rule(
            "cache-mount-id-scope",
            "two stages mounting the same cache target without an `id=` SHARE one cache — different toolchains or targets then overwrite each other's artifacts",
            "the same `--mount=type=cache,target=...` value in two different stages with no `id=`",
            "FROM rust:1.97.0-slim AS a\nRUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release\n\nFROM rust:1.97.0-alpine AS b\nRUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release\n",
            "FROM rust:1.97.0-slim AS a\nRUN --mount=type=cache,id=registry-gnu,target=/usr/local/cargo/registry cargo build --release\n\nFROM rust:1.97.0-alpine AS b\nRUN --mount=type=cache,id=registry-musl,target=/usr/local/cargo/registry cargo build --release\n",
            F::CachePerf, S::Builtin, None,
            Some("Suggest-only: the id names are the author's call, and sharing is sometimes CORRECT (two stages on the same toolchain SHOULD share). The finding cites the risk; it does not claim a defect."),
        ),
        rule(
            "entrypoint-shell-form-signal-block",
            "shell-form ENTRYPOINT makes PID1 the shell, and `sh` does not forward SIGTERM — the container ignores `docker stop` and is SIGKILLed after the grace period",
            "an ENTRYPOINT in shell form whose command carries a shell metacharacter (so exec form is not a safe rewrite)",
            "FROM debian:bookworm-slim\nENTRYPOINT /usr/local/bin/app --config $CONFIG\n",
            "FROM debian:bookworm-slim\nCOPY entrypoint.sh /entrypoint.sh\nENTRYPOINT [\"/entrypoint.sh\"]\n",
            F::SignalPid1, S::Builtin, Some("DL3025"),
            Some("RECLASSIFIED Class B -> Class A at mine time (2026-08-28): DL3025 covers ENTRYPOINT as well as CMD, so this is an overlay of the same incumbent as `cmd-entrypoint-exec-form`, not novel detection. It remains a SEPARATE rule because its fix differs (a wrapper script, not a JSON array). Suggest-only precisely BECAUSE of the metacharacter: the shell is doing real work (variable expansion), so exec form would change behavior. The correct fix is a wrapper script ending in `exec \"$@\"`, which the healer cannot author blind."),
        ),
        rule(
            "build-stage-missing-buildplatform",
            "a multi-platform build with no platform intent runs the compiler under QEMU emulation on every non-native arch — measured 5.2x-7x slower on compile-heavy trees",
            "a multi-stage file whose compiling stage pins no --platform and whose file mentions no TARGETPLATFORM/TARGETARCH/BUILDPLATFORM",
            "FROM rust:1.97.0-slim AS builder\nWORKDIR /app\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=builder /app/target/release/app /usr/local/bin/app\n",
            "FROM --platform=$BUILDPLATFORM rust:1.97.0-slim AS builder\nARG TARGETARCH\nWORKDIR /app\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=builder /app/target/release/app /usr/local/bin/app\n",
            F::MultiPlatform, S::Builtin, None,
            Some("ANNOTATION ONLY — never rewritten. Both strategies are valid: QEMU-native keeps the toolchain simple (riir-chain deploy/Dockerfile does exactly this, deliberately) while $BUILDPLATFORM cross is far faster but needs a cross toolchain. The choice is dep-complexity x matrix-width; the healer cites the decision table."),
        ),
        rule(
            "distroless-distro-suffix-pin",
            "an unsuffixed distroless tag floats across debian releases, and the glibc it ships must be >= the glibc the builder linked against — the in-tree pairing broke on exactly this",
            "a runtime-stage base under gcr.io/distroless/ with no -debianNN suffix and no digest",
            "FROM rust:1.97.0-slim AS builder\nRUN cargo build --release\n\nFROM gcr.io/distroless/cc\nCOPY --from=builder /app/target/release/app /app\n",
            "FROM rust:1.97.0-slim AS builder\nRUN cargo build --release\n\nFROM gcr.io/distroless/cc-debian13\nCOPY --from=builder /app/target/release/app /app\n",
            F::BaseChoice, S::Builtin, None,
            Some("Suggest-only: WHICH suffix is correct depends on the builder's glibc (debian13 = trixie = glibc 2.41). Picking one blind can produce an image that fails at exec time with a GLIBC version error — the loudest possible failure, and the reason this is not auto-fixed."),
        ),
        rule(
            "musl-scratch-ca-certs",
            "a scratch runtime has no CA bundle, so every outbound TLS call fails at runtime with an opaque certificate error",
            "a runtime stage FROM scratch with no ca-certificates copied into it",
            "FROM rust:1.97.0-alpine AS builder\nRUN cargo build --release --target x86_64-unknown-linux-musl\n\nFROM scratch\nCOPY --from=builder /app/target/x86_64-unknown-linux-musl/release/app /app\n",
            "FROM rust:1.97.0-alpine AS builder\nRUN cargo build --release --target x86_64-unknown-linux-musl\n\nFROM scratch\nCOPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt\nCOPY --from=builder /app/target/x86_64-unknown-linux-musl/release/app /app\n",
            F::BaseChoice, S::Builtin, None,
            Some("Suggest-only: a binary that makes no outbound TLS call needs no bundle, and the healer cannot prove which it is from the Dockerfile. The in-tree deploy/Dockerfile copies the bundle and documents exactly why (the CLIENT side needs roots even though the daemon speaks plaintext behind a proxy)."),
        ),
        rule(
            "runtime-stage-missing-user",
            "a runtime stage with no USER runs as root — a container escape then starts with uid 0, and the daemon almost never needs to write to its own filesystem",
            "a multi-stage file whose runtime stage is not scratch and declares no USER",
            "FROM rust:1.97.0-slim AS builder\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=builder /app/target/release/app /usr/local/bin/app\nCMD [\"/usr/local/bin/app\"]\n",
            "FROM rust:1.97.0-slim AS builder\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=builder /app/target/release/app /usr/local/bin/app\nUSER 10001:10001\nCMD [\"/usr/local/bin/app\"]\n",
            F::BaseChoice, S::Builtin, None,
            Some("Suggest-only: WHICH uid, and whether the process needs a writable path owned by it, are deployment facts. A blind `USER 10001` on an image that writes to /var breaks it at runtime. The in-tree deploy/Dockerfile shows the good form (numeric uid, because scratch has no /etc/passwd) and states the reason."),
        ),
        rule(
            "chef-stage-toolchain-skew",
            "a cargo-chef cook stage on a different base tag than the build stage recooks the entire dependency layer every build — the exact cost chef exists to remove",
            "two or more stages running `cargo chef` on different base images",
            "FROM rust:1.97.0-slim AS planner\nRUN cargo chef prepare --recipe-path recipe.json\n\nFROM rust:1.97.0-alpine AS cook\nRUN cargo chef cook --release --recipe-path recipe.json\n",
            "FROM rust:1.97.0-slim AS planner\nRUN cargo chef prepare --recipe-path recipe.json\n\nFROM rust:1.97.0-slim AS cook\nRUN cargo chef cook --release --recipe-path recipe.json\n",
            F::RustIdiom, S::Builtin, None,
            Some("Suggest-only: which base is the right one is the author's call (the skew may be deliberate for a musl target — in which case chef's recipe should be per-target too)."),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::docker::rules::{DOCKER_RULES, FIXABLE_DOCKER};
    use crate::domains::docker::scanner;

    #[test]
    fn every_fixture_scans_as_a_dockerfile() {
        for r in entries() {
            for (which, src) in [("buggy", &r.buggy_code), ("fixed", &r.fixed_code)] {
                let scan = scanner::scan(src);
                assert!(
                    !scan.stanzas.is_empty() && !scan.stages.is_empty(),
                    "{} {which} fixture does not scan as a Dockerfile",
                    r.rule_id
                );
            }
        }
    }

    #[test]
    fn rule_ids_unique_and_kebab() {
        let mut seen = std::collections::BTreeSet::new();
        for r in entries() {
            assert!(seen.insert(r.rule_id.clone()), "duplicate rule id {}", r.rule_id);
            assert!(
                r.rule_id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not kebab-case",
                r.rule_id
            );
        }
    }

    #[test]
    fn corpus_and_detector_rule_sets_agree() {
        let ids: std::collections::BTreeSet<String> =
            entries().into_iter().map(|r| r.rule_id).collect();
        let detectors: std::collections::BTreeSet<String> =
            DOCKER_RULES.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            ids, detectors,
            "the corpus and the detector table have drifted apart"
        );
        for id in FIXABLE_DOCKER {
            assert!(DOCKER_RULES.contains(id), "{id} is fixable but not a detector rule");
        }
    }

    /// The Plan 030 close condition, mechanically: fewer than 10 BOUNDED
    /// rules closes the domain at Phase 1. This pins the measured number so
    /// a silent regression cannot slip under it.
    #[test]
    fn phase1_lands_at_least_ten_bounded_rules() {
        assert!(
            FIXABLE_DOCKER.len() >= 10,
            "bounded-rule count {} is below the Plan 030 close condition of 10",
            FIXABLE_DOCKER.len()
        );
        // The Proposal 006 Phase-2 gate: >= ~20 rules post-falsification.
        assert!(
            entries().len() >= 20,
            "total rule count {} is below the Phase-2 gate of 20",
            entries().len()
        );
    }

    #[test]
    fn class_a_rules_name_their_incumbent() {
        for r in entries() {
            if let Some(inc) = &r.incumbent {
                assert!(
                    inc.starts_with("DL") || inc.chars().next().is_some_and(char::is_uppercase),
                    "{}: incumbent id {inc} is not a DL rule or a BuildKit check",
                    r.rule_id
                );
            }
        }
    }
}
