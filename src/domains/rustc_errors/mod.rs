//! `rustc_errors` — compile-error repair keyed by rustc error code
//! (Plan 019; Issue 035).
//!
//! What ships here TODAY is the measurement layer, not the fixer:
//!
//! - [`diag`] — suggestion-aware rustc JSON parsing (error code +
//!   `suggested_replacement` + applicability).
//! - [`probe`] — the premise probe: does rustc already auto-fix this code?
//! - [`floor`] — the G-floor harness: any [`floor::CompileFixer`] scored
//!   against `cargo fix --broken-code` on a known-inverse mutation corpus.
//!
//! The driver (`--fix-compile`) and the playbook templates are Plan 019
//! Phase 1–3 and are NOT here yet. That ordering is deliberate: the plan's
//! founding premise ("rustc is structurally blind to the borrowck family")
//! turned out to be false on 5 of 8 measured codes, so the floor is measured
//! before templates are authored against it.
//!
//! **Exact-keyed, so no retrieval.** The diagnostic carries the error code —
//! nothing here needs latent lookup, which is why the `rustc_errors` feature
//! does NOT pull `latent_retrieval` (Plan 019 T0.3). `src/fix_verify.rs`
//! imports only `std` + `serde`; its `latent_retrieval` implication is a
//! CLI-surface artifact, not a code dependency.

pub mod diag;
pub mod floor;
pub mod probe;
