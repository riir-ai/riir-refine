//! `rustc_errors` — compile-error repair keyed by rustc error code
//! (Plan 019; Issue 035).
//!
//! - [`diag`] — suggestion-aware rustc JSON parsing (error code +
//!   `suggested_replacement` + applicability).
//! - [`probe`] — the premise probe: does rustc already auto-fix this code?
//! - [`floor`] — the G-floor harness: any [`floor::CompileFixer`] scored
//!   against `cargo fix --broken-code` on a known-inverse mutation corpus
//!   (arms: the [`floor::CargoFixBaseline`] floor, the [`floor::DriverFixer`]
//!   product arm, the [`floor::NullFixer`] falsification control).
//! - [`driver`] — the Phase 1 `--fix-compile` bounded repair loop
//!   (check → propose → apply → recheck → keep/revert), over the
//!   [`playbooks`] registry.
//! - [`playbooks`] — the template families: [`playbooks::e0597`] (Phase 2,
//!   six proposers) + [`playbooks::borrowck`] (Phase 3 — E0502/E0499/
//!   E0505; E0596/E0382/E0507/E0373 ship NO template by the measured
//!   floor, Bench 041).
//!
//! **Exact-keyed, so no retrieval.** The diagnostic carries the error code —
//! nothing here needs latent lookup, which is why the `rustc_errors` feature
//! does NOT pull `latent_retrieval` (Plan 019 T0.3). `src/fix_verify.rs`
//! imports only `std` + `serde`; its `latent_retrieval` implication is a
//! CLI-surface artifact, not a code dependency.

pub mod diag;
pub mod driver;
pub mod evolve;
pub mod floor;
pub mod playbooks;
pub mod probe;
