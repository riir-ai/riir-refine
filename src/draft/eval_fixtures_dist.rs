//! Issue 134 T3 re-arm — `dist` domain eval fixtures (hand-authored variants
//! of the 6 corpus anti-patterns; the `eval_fixtures_perf` convention —
//! variant crate names, canonical manifest shapes, one span per rule).
//!
//! Honest confusability note: three ship-size rules share the bare-`[[bin]]`
//! trigger shape (`shipped-binary-unstripped` / `-missing-size-profile` /
//! `cli-panic-unwind-tables` all fire on a `[[bin]]` manifest with no
//! profile keys), so their corpus texts are near-duplicates and their
//! fixtures are legitimately close in the retrieval space — the domain's
//! demonstrability constant measures exactly that selection difficulty.
//!
//! Pure data; consumed by `super::retrieval_eval` under the
//! `latent_retrieval` feature.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data.
#![allow(clippy::needless_raw_string_hashes)]

/// (`rule_id`, `span_code`) — one span per rule; each genuinely instantiates
/// the rule's anti-pattern (semantic label, authored against the corpus entry).
pub const DIST_SPANS: &[(&str, &str)] = &[
    (
        "shipped-binary-unstripped",
        r#"[[bin]]
name = "asset-gen"

[dependencies]
anyhow = "1"
"#,
    ),
    (
        "shipped-binary-missing-size-profile",
        r#"[[bin]]
name = "bundle-tool"

[dependencies]
clap = "4"
"#,
    ),
    (
        "cli-panic-unwind-tables",
        r#"[[bin]]
name = "sweep-cli"
"#,
    ),
    (
        "member-profile-ignored",
        r#"[package]
name = "worker-crate"

[[bin]]
name = "worker-crate"

[profile.release]
strip = true
"#,
    ),
    (
        "large-api-dependency-mostly-unused-hint",
        r#"[package]
name = "desktop-shell"

[dependencies]
serde = "1"
rustix = { version = "0.38", features = ["fs", "termios"] }
"#,
    ),
    (
        "pgo-training-set-covers-real-workload-hint",
        r#"[[bin]]
name = "index-tool"

[profile.dist]
inherits = "release"
strip = true
lto = "fat"
codegen-units = 1
panic = "abort"
"#,
    ),
];
