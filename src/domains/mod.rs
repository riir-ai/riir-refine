//! Domain modules — pluggable healing domains behind the `HealDomain` trait.
//!
//! Each domain is gated behind its own Cargo feature:
//! - `clippy_lints` (default) — `clippy::*` lint healing (the original domain)
//! - `rust_perf` (default since 2026-08-13, Phase 2) — `rust-optimize` perf
//!   healing (promoted after Bench 013 GOAT G1+G5 PASS)
//! - `kernel_opt` (opt-in, Phase 3) — GPU kernel optimization healing
//! - `docker` (default since 2026-08-28, Plan 030) — Dockerfile bounded-fix
//!   healing + retrieval; the only domain with no `syn` (a hand-written
//!   stanza scanner) and the only one whose oracle is an external binary
//!   (hadolint, when on PATH)
//! - `rustc_errors` (default since 2026-08-29, Plan 019) — rustc
//!   compile-error repair keyed by error code: the E0597 + borrowck
//!   (E0502/E0499/E0505) playbook families behind the `--fix-compile`
//!   driver, plus the premise probe and the `cargo fix` floor that bound
//!   the territory. Driver-shaped: inert until invoked.
//! - `dist` (default since 2026-09-13, Issue 085) — shipped-artifact
//!   build-config advice: release-profile size knobs + the workspace-member
//!   profile trap, over name-matched `Cargo.toml` manifests. Suggest-only in
//!   Phase 1; the second no-`syn` scanner after docker.
//! - `shader_perf` (opt-in, Plan 134 / Proposal 014) — WGSL render-pipeline
//!   (`@vertex`/`@fragment`) bounded-fix healing: loop boundedness (the TDR
//!   class), sample budgets, branch-vs-sample divergence, derivative
//!   safety. `@compute` stays `kernel_opt` (entry-point routing split). The
//!   third no-`syn` scanner; consumers: the upcoming shader-toy editor's
//!   submit/save gate + riir-dapps Proposal 002's Stage-2 mint gate.

// Issue 112: the scratch-probe identity witness, shared by every domain
// whose oracle spawns cargo on a generated crate. Gated on the union of
// those three features rather than living inside one of them — a copy per
// site is a second thing to get wrong, which is the shape Issue 110 was.
#[cfg(any(
    feature = "clippy_verify",
    feature = "rust_perf_verify",
    feature = "rust_perf_bench"
))]
pub mod probe_witness;

#[cfg(feature = "clippy_lints")]
pub mod clippy_lints;

#[cfg(feature = "rust_perf")]
pub mod rust_perf;

#[cfg(feature = "kernel_opt")]
pub mod kernel_opt;

#[cfg(feature = "rustc_errors")]
pub mod rustc_errors;

#[cfg(feature = "sec")]
pub mod sec;

#[cfg(feature = "docker")]
pub mod docker;

#[cfg(feature = "dist")]
pub mod dist;

#[cfg(feature = "shader_perf")]
pub mod shader_perf;
