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
//! (The lowercasing unmangles `-` back from cargo's `_` since the
//! `net-mainnet` release pin joined the manifest — the stamp carries the
//! manifest spelling, and `build_stamp::has` normalizes both sides.)
//!
//! Issue 150 (option 3) adds the COMMIT stamp: the rerun inputs and the
//! capture logic live in the shared [`build_commit`] module (build scripts
//! cannot host unit tests; that module's tests run through
//! `tests/build_commit_logic.rs`). ⛔ The rerun list here is a SUPERSET of
//! everything that can move either stamp — the first `rerun-if-changed`
//! line disables cargo's any-package-file default, so dropping a line freezes
//! one of the stamps silently.
#[path = "build_commit.rs"]
mod build_commit;

fn main() {
    let mut on: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_")
                .map(|f| f.to_ascii_lowercase().replace('_', "-"))
        })
        .collect();
    on.sort();
    println!(
        "cargo:rustc-env=RIIR_CLIPPY_COMPILED_FEATURES={}",
        on.join(",")
    );

    let cwd = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
    );
    for line in build_commit::rerun_lines(&cwd) {
        println!("{line}");
    }
    println!(
        "cargo:rustc-env=RIIR_REFINE_BUILD_COMMIT={}",
        build_commit::capture_commit(&cwd)
    );
}
