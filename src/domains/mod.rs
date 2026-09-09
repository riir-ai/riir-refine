//! Domain modules — pluggable healing domains behind the `HealDomain` trait.
//!
//! Each domain is gated behind its own Cargo feature:
//! - `clippy_lints` (default) — `clippy::*` lint healing (the original domain)
//! - `rust_perf` (default since 2026-08-13, Phase 2) — `rust-optimize` perf
//!   healing (promoted after Bench 013 GOAT G1+G5 PASS)
//! - `kernel_heal` (opt-in, Phase 3) — GPU kernel optimization healing
//! - `docker` (opt-in, Plan 030) — Dockerfile bounded-fix healing +
//!   retrieval; the only domain with no `syn` (a hand-written stanza
//!   scanner) and the only one whose oracle is an external binary
//!   (hadolint, when on PATH)
//! - `rustc_errors` (opt-in, Plan 019) — rustc compile-error repair keyed
//!   by error code: the E0597 + borrowck (E0502/E0499/E0505) playbook
//!   families behind the `--fix-compile` driver, plus the premise probe
//!   and the `cargo fix` floor that bound the territory.
//! - `dist` (opt-in, Issue 085) — shipped-artifact build-config advice:
//!   release-profile size knobs + the workspace-member profile trap, over
//!   name-matched `Cargo.toml` manifests. Suggest-only in Phase 1; the
//!   second no-`syn` scanner after docker.

#[cfg(feature = "clippy_lints")]
pub mod clippy_lints;

#[cfg(feature = "rust_perf")]
pub mod rust_perf;

#[cfg(feature = "kernel_heal")]
pub mod kernel_opt;

#[cfg(feature = "rustc_errors")]
pub mod rustc_errors;

#[cfg(feature = "sec")]
pub mod sec;

#[cfg(feature = "docker")]
pub mod docker;

#[cfg(feature = "dist")]
pub mod dist;
