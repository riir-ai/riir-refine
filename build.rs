//! Issue 117 T1 — the compiled-feature stamp, DERIVED rather than typed.
//!
//! `--version` has to be able to say which features this binary was compiled
//! with, because the whole failure class it answers (the 00:21 incident) is a
//! binary that exists, advertises its gated flags in `--help`, and has them
//! `cfg`'d out. A hand-maintained list of feature names in the source would
//! drift from `[features]` the first time a feature is added — the drift this
//! workspace has a gate for one axis over.
//!
//! Cargo already computes the answer: it exports `CARGO_FEATURE_<NAME>` to the
//! build script for every feature that is ON for THIS compilation, including
//! the ones turned on transitively. Lower-casing the suffix recovers the
//! feature name exactly, because every feature in this manifest is `snake_case`
//! (cargo's mangling only differs for `-`-bearing names, and there are none).
//! So the stamp is a projection of the real build, not a claim about it.
fn main() {
    let mut on: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_")
                .map(|f| f.to_ascii_lowercase())
        })
        .collect();
    on.sort();
    println!("cargo:rustc-env=RIIR_CLIPPY_COMPILED_FEATURES={}", on.join(","));
    // The set is a function of the manifest + the invocation; re-run when the
    // manifest moves. (Cargo already re-runs on a feature-selection change.)
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
}
