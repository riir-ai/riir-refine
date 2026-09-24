//! Issue 134 T3 re-arm — `docker` domain eval fixtures (hand-authored
//! variants of the 31 corpus anti-patterns; the `eval_fixtures_perf`
//! convention — variant image names/packages, canonical instruction shapes,
//! one span per rule).
//!
//! Pure data; consumed by `super::retrieval_eval` under the
//! `latent_retrieval` feature.
//!
//! Honest confusability note: `cache-mount-missing-on-cargo-build` and
//! `manifest-first-layer-split` carry BYTE-IDENTICAL corpus buggy texts, and
//! the apt family overlaps — the domain's demonstrability constant measures
//! exactly that selection difficulty (0.867 at the Bench-101 landing). The
//! cache-mount fixture below deliberately drops `COPY . .` (manifest-first's
//! trigger) per the single-defect convention; the miss that remains is the
//! corpus's own near-duplicate, not the fixture's.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data (some
// spans contain `"` or `"#`-adjacent sequences; mixed styles churn every
// fixture edit).
#![allow(clippy::needless_raw_string_hashes)]

/// (`rule_id`, `span_code`) — one span per rule; each genuinely instantiates
/// the rule's anti-pattern (semantic label, authored against the corpus entry).
pub const DOCKER_SPANS: &[(&str, &str)] = &[
    (
        "apt-lists-not-removed",
        r#"FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends jq
"#,
    ),
    (
        "apt-no-install-recommends",
        r#"FROM ubuntu:24.04
RUN apt-get update && apt-get install -y ca-certificates
"#,
    ),
    (
        "apk-add-no-cache",
        r#"FROM alpine:3.19
RUN apk add openssl
"#,
    ),
    (
        "add-local-path-to-copy",
        r#"FROM alpine:3.19
ADD ./gateway /usr/local/bin/gateway
"#,
    ),
    (
        "cmd-entrypoint-exec-form",
        r#"FROM scratch
CMD /usr/local/bin/relayd
"#,
    ),
    (
        "apt-not-apt-get",
        r#"FROM debian:bookworm-slim
RUN apt update && apt install -y git
"#,
    ),
    (
        "maintainer-to-label",
        r#"FROM debian:bookworm-slim
MAINTAINER ops@example.org
"#,
    ),
    (
        "pipefail-before-piped-run",
        r#"FROM debian:bookworm-slim
RUN curl -sSL https://example.org/tool.tar.gz | tar -xz -C /opt
"#,
    ),
    (
        "workdir-absolute-path",
        r#"FROM rust:1.98.0-slim
WORKDIR srv
"#,
    ),
    (
        "instruction-casing",
        r#"FROM scratch
run echo ready
"#,
    ),
    (
        "legacy-key-value-format",
        r#"FROM scratch
ENV TRACE_LEVEL debug
"#,
    ),
    (
        "no-empty-continuation",
        r#"FROM alpine:3.19
RUN echo one \

  && echo two
"#,
    ),
    (
        "apt-update-install-same-run",
        r#"FROM debian:bookworm-slim
RUN apt-get update
RUN apt-get install --no-install-recommends -y gnupg \
 && rm -rf /var/lib/apt/lists/*
"#,
    ),
    (
        "cache-mount-missing-on-cargo-build",
        // No `COPY . .` — that line is manifest-first-layer-split's trigger,
        // and the two corpus buggy texts are byte-identical; the single-defect
        // convention (clippy's zero-cross-fire rule) keeps this fixture to
        // cache-mount's OWN trigger: a cargo build with no cache mount.
        r#"FROM rust:1.98.0-slim
WORKDIR /workspace
RUN cargo build --release
"#,
    ),
    (
        "missing-syntax-directive",
        r#"FROM alpine:3.20
COPY --link ./relay /relay
"#,
    ),
    (
        "cargo-locked-flag",
        r#"FROM rust:1.98.0-slim
WORKDIR /workspace
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release
"#,
    ),
    (
        "workspace-single-member-build",
        r#"FROM rust:1.98.0-slim
WORKDIR /workspace
RUN cargo build --release --features telemetry
"#,
    ),
    (
        "dockerignore-missing",
        r#"FROM rust:1.98.0-slim
WORKDIR /workspace
COPY . .
"#,
    ),
    (
        "arg-secret-to-secret-mount",
        r#"FROM alpine:3.19
ARG DEPLOY_KEY
RUN echo "key=$DEPLOY_KEY" >> /root/.deploy_env
"#,
    ),
    (
        "manifest-first-layer-split",
        r#"FROM rust:1.98.0-slim
WORKDIR /workspace
COPY . .
RUN cargo build --release
"#,
    ),
    (
        "cache-mount-id-scope",
        r#"FROM rust:1.98.0-slim AS tools
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

FROM rust:1.98.0-alpine AS tools-musl
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release
"#,
    ),
    (
        "entrypoint-shell-form-signal-block",
        r#"FROM debian:bookworm-slim
ENTRYPOINT /usr/local/bin/relay --port $LISTEN_PORT
"#,
    ),
    (
        "build-stage-missing-buildplatform",
        r#"FROM rust:1.98.0-slim AS compile
WORKDIR /src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=compile /src/target/release/relay /usr/local/bin/relay
"#,
    ),
    (
        "distroless-distro-suffix-pin",
        r#"FROM rust:1.98.0-slim AS build
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=build /srv/target/release/agent /agent
"#,
    ),
    (
        "musl-scratch-ca-certs",
        r#"FROM rust:1.98.0-alpine AS build
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=build /srv/target/x86_64-unknown-linux-musl/release/agent /agent
"#,
    ),
    (
        "runtime-stage-missing-user",
        r#"FROM rust:1.98.0-slim AS build
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=build /srv/target/release/agent /usr/local/bin/agent
CMD ["/usr/local/bin/agent"]
"#,
    ),
    (
        "chef-stage-toolchain-skew",
        r#"FROM rust:1.98.0-slim AS planner
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.98.0-alpine AS cook
RUN cargo chef cook --release --recipe-path recipe.json
"#,
    ),
    (
        "base-tag-not-digest-pinned",
        r#"FROM rust:1.98.0-slim AS builder
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /srv/bin/worker /usr/local/bin/worker
"#,
    ),
    (
        "copy-link-artifact-rebase",
        r#"# syntax=docker/dockerfile:1.4
FROM rust:1.98.0-slim AS builder
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /srv/bin/worker /usr/local/bin/worker
"#,
    ),
    (
        "musl-allocator-perf-annotation",
        r#"FROM rust:1.98.0-alpine AS builder
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /srv/bin/worker /usr/local/bin/worker
EXPOSE 9443
"#,
    ),
    (
        "healthcheck-tool-absent-in-runtime",
        r#"FROM rust:1.98.0-slim AS builder
RUN cargo build --release

FROM scratch
COPY --from=builder /srv/bin/worker /usr/local/bin/worker
HEALTHCHECK CMD ["curl", "-f", "http://localhost:9443/healthz"]
"#,
    ),
];
