//! Domain modules — pluggable healing domains behind the `HealDomain` trait.
//!
//! Each domain is gated behind its own Cargo feature:
//! - `clippy_lints` (default) — `clippy::*` lint healing (the original domain)
//! - `rust_perf` (opt-in, Phase 2) — `rust-optimize` perf healing
//! - `kernel_heal` (opt-in, Phase 3) — GPU kernel optimization healing

#[cfg(feature = "clippy_lints")]
pub mod clippy_lints;

#[cfg(feature = "rust_perf")]
pub mod rust_perf;

#[cfg(feature = "kernel_heal")]
pub mod kernel_opt;
