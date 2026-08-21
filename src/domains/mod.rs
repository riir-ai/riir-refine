//! Domain modules — pluggable healing domains behind the `HealDomain` trait.
//!
//! Each domain is gated behind its own Cargo feature:
//! - `clippy_lints` (default) — `clippy::*` lint healing (the original domain)
//! - `rust_perf` (default since 2026-08-13, Phase 2) — `rust-optimize` perf
//!   healing (promoted after Bench 013 GOAT G1+G5 PASS)
//! - `kernel_heal` (opt-in, Phase 3) — GPU kernel optimization healing
//! - `rustc_errors` (opt-in, Plan 019) — rustc compile-error repair. Ships
//!   the measurement layer only so far (premise probe + `cargo fix` floor).

#[cfg(feature = "clippy_lints")]
pub mod clippy_lints;

#[cfg(feature = "rust_perf")]
pub mod rust_perf;

#[cfg(feature = "kernel_heal")]
pub mod kernel_opt;

#[cfg(feature = "rustc_errors")]
pub mod rustc_errors;
