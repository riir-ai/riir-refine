//! Plan 030 T1.5 G4 — the scan path's steady-state allocation behavior.
//!
//! The `SuggestScratch` contract: buffers are cleared and refilled, so
//! capacity stabilizes and the steady state performs no growth. This test
//! asserts the observable, and it asserts the two honest limits too — a
//! gate that claimed "alloc-free" while findings still allocated would be
//! measuring the wrong thing.
//!
//! What is proven here:
//! 1. capacities stop growing after warm-up, across DIFFERENT inputs;
//! 2. a CLEAN file's scan produces zero findings, so it allocates no
//!    finding strings at all;
//! 3. the scratch path and the allocating path agree exactly — the shared
//!    detector body means there is one behavior, not two.

#![cfg(feature = "docker")]

use riir_refine::domains::docker::rules::{
    DockerScratch, ScanContext, scan_into, scan_source_with,
};

const FILES: &[&str] = &[
    "FROM debian:bookworm-slim\nRUN apt-get install -y curl\n",
    "FROM alpine:3.20\nrun apk add build-base\nENV RUST_LOG info\n",
    "FROM rust:1.97.0-slim AS b\nWORKDIR app\nCOPY . .\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=b /app/x /x\nCMD /x\n",
    "FROM scratch\nADD ./app /app\nMAINTAINER ops@example.com\n",
];

#[test]
fn g4_scratch_capacity_reaches_a_steady_state() {
    let ctx = ScanContext::default();
    let mut scratch = DockerScratch::default();
    // Warm-up: every input once, so capacity reaches the max any of them needs.
    for _ in 0..3 {
        for src in FILES {
            let _ = scan_into(src, &ctx, &mut scratch);
        }
    }
    let warm = scratch.capacities();
    // Steady state: many more passes must not grow either buffer.
    for _ in 0..200 {
        for src in FILES {
            let _ = scan_into(src, &ctx, &mut scratch);
        }
    }
    assert_eq!(
        scratch.capacities(),
        warm,
        "scan buffers grew after warm-up — the scan path is not reaching a steady state"
    );
    assert!(
        warm.0 > 0,
        "the stanza buffer never filled — the test is not exercising the path"
    );
}

#[test]
fn g4_clean_file_produces_no_findings_to_allocate() {
    // The in-tree negative control's shape: a correct file. Zero findings
    // means zero per-finding allocation, which is the strongest form of the
    // claim available for an owned-output API.
    let clean = "FROM rust:1.97.0-alpine AS builder\n\
                 ARG TARGETPLATFORM\n\
                 WORKDIR /build\n\
                 RUN apk add --no-cache build-base\n\
                 RUN --mount=type=cache,target=/usr/local/cargo/registry \\\n    cargo build --release\n\
                 \n\
                 FROM scratch\n\
                 COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt\n\
                 COPY --from=builder /build/x /x\n\
                 USER 10001:10001\n\
                 ENTRYPOINT [\"/x\"]\n";
    let mut scratch = DockerScratch::default();
    let findings = scan_into(clean, &ScanContext::default(), &mut scratch);
    assert!(
        findings.is_empty(),
        "clean file produced findings: {findings:?}"
    );
}

#[test]
fn g4_scratch_and_allocating_paths_agree() {
    let ctx = ScanContext::default();
    let mut scratch = DockerScratch::default();
    for src in FILES {
        let owned = scan_source_with(src, &ctx);
        let borrowed = scan_into(src, &ctx, &mut scratch);
        assert_eq!(
            owned.as_slice(),
            borrowed,
            "the scratch path and the allocating path disagree on {src:?}"
        );
    }
}

/// G2 — the latency shape. Not a hard threshold (a shared box makes
/// wall-clock a poor gate; see the house rule on load-flaky timing tests),
/// but the per-scan cost is REPORTED so a pathological regression is
/// visible in the run log.
#[test]
fn g2_scan_latency_is_reported() {
    let ctx = ScanContext::default();
    let mut scratch = DockerScratch::default();
    let big = FILES.join("\n");
    let n = 2_000;
    let t0 = std::time::Instant::now();
    for _ in 0..n {
        let _ = scan_into(&big, &ctx, &mut scratch);
    }
    let per = t0.elapsed() / n;
    println!("G2: {} bytes/scan, {per:?}/scan (steady state)", big.len());
    // Measured 2026-08-28 on the M3 Max: 17.05 µs/scan RELEASE, 1.24 ms
    // DEBUG. The threshold below is sized for the debug build this test
    // normally runs in; the release number is the one to quote, and the
    // ~70x gap is ordinary unoptimized-String overhead, not a regression.
    assert!(
        per < std::time::Duration::from_millis(5),
        "a {}-byte scan took {per:?} — 3 orders of magnitude off the expected µs scale",
        big.len()
    );
}
