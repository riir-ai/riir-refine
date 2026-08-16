//! Issue 024 — cross-domain (clippy × `rust_perf`) co-present fixture pairs.
//!
//! ## Why these exist
//!
//! Research 484 (Skill Entropy, He et al. arXiv:2608.05139) measured that
//! LLMs solve a skill −4..−13% worse when a *different* domain's skill is
//! exercised in the same chain. Whether the multi-domain healer has the
//! retrieval-level analog of that switch cost — an anti-pattern retrieving
//! its rule worse when another domain's anti-pattern is co-present in the
//! span — was unmeasured. [`super::switch_cost`] measures it on these
//! pairs.
//!
//! ## Composition, not authoring
//!
//! Each pair reuses the **exact** solo fixture codes from
//! [`super::eval_fixtures_clippy::CLIPPY_SPANS`] and
//! [`super::eval_fixtures_perf::PERF_SPANS`] (looked up at runtime, never
//! duplicated). The solo and co-present measurements therefore share
//! byte-identical constituent code — the paired delta is attributable to
//! co-presence alone, not to fixture drift.
//!
//! ## Pair selection rules (the confound guards)
//!
//! - The perf constituent must be **self-contained** (std-only, compiles
//!   standalone) so the merged span can be labeled by the real-clippy
//!   oracle (a no-dep temp crate). This excludes e.g. `fxhashmap` (its
//!   fixture imports `rustc_hash`, and the proposer's Gate-0 self-check
//!   requires that import — an oracle-compile/proposer-fire conflict
//!   recorded here so it is not re-tried naively) and `regex-hoist-lazylock`.
//! - The perf constituent's shape must not itself instantiate a **clippy
//!   corpus lint** (`simd-chunk`/`copy-from-slice`/`branchless-conditional`
//!   index-and-loop shapes ARE `needless_range_loop` — excluded) nor a
//!   second **perf proposer** pattern (`dedupe-hashset-vs-vec` and
//!   `smallvec-stack` carry `Vec::new()`+push = `vec-with-capacity` —
//!   excluded).
//! - The clippy constituent's shape must not instantiate any perf
//!   proposer pattern (none of the twelve use `HashMap`, `Vec::new`+push,
//!   or a string-literal `.to_string()`).
//! - No identifier collisions within a merged span.
//!
//! The committed label test below enforces the cross-fire half of these
//! rules against the proposers; the oracle truth test
//! (`tests/cross_domain_oracle_truth.rs`) enforces the exactly-one-corpus-
//! lint half against real clippy.

/// `(clippy_lint, perf_rule)` — 12 pairs. Constituent codes are resolved from
/// the solo fixture sets at runtime (first match by name; deterministic).
///
/// Order within a merged span is a MEASUREMENT AXIS (the directed pair):
/// every pair is evaluated in both textual orders by
/// [`super::switch_cost::cross_domain_spans`].
pub const CROSS_PAIRS: &[(&str, &str)] = &[
    ("needless_return", "vec-with-capacity"),
    ("identity_op", "static-str"),
    ("collapsible_if", "entry-vs-contains-insert"),
    ("needless_bool", "bool-to-bitpack"),
    ("manual_map", "avoid-clone"),
    ("redundant_field_names", "format-concat-loop"),
    ("double_parens", "collect-avoidance"),
    ("iter_nth_zero", "mem-take"),
    ("deref_addrof", "box-avoid"),
    ("question_mark", "btreemap-sorted-iteration"),
    ("needless_match", "retain-inplace"),
    ("redundant_pattern_matching", "precompute-table"),
];
