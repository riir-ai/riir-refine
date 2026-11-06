//! The corpus pins manifest (plan 329 P1 — issue 197, the uniffi
//! per-API-checksum handshake × cargo-vet aggregate ported).
//!
//! ONE committed file — `.benchmarks/pins_manifest.json` — is the single
//! source of truth for every corpus count the repo pins:
//!
//! ```json
//! {"clippy_lints": {"rules": N},
//!  "rust_perf": {"rules": N, "categories": N, "benchable": N, "notbenchable": N},
//!  "kernel_opt": {"rules": N, "witness": {"kernel_categories": 15, "experts": 15, "activity_dim": 8}},
//!  "sec": {"rules": N}}
//! ```
//!
//! The numbers are DERIVED, never hand-typed: `scripts/pins_manifest.py`
//! runs [`dump::dump_pins_manifest`] (this module, under `cargo test`) with
//! `kernel_opt,sec,rust_perf_bench` enabled and the compiled corpus
//! builders report their own live counts. The count asserts across
//! `corpus/tests.rs` / `clippy_lints/corpus.rs` / `sec/corpus.rs` /
//! `kernel_opt/corpus/tests.rs` / `kernel_opt/router.rs` read this manifest
//! through [`count`] instead of re-typing numbers — the asserts stay
//! deliberate speed bumps (the rubber-stamp defense: `--write` cannot
//! "fix" a failing assert, only a real corpus change can, and that change
//! must land WITH the regenerated manifest diff). The B308 class — two
//! hand-maintained pin families drifting apart — is structurally closed:
//! there is exactly one number, and everything reads it.
//!
//! The script's PROSE half gates the same values into AGENTS.md /
//! README.md / `.docs/11_domains/*` live rows. A red from this gate is
//! repaired by `scripts/pins_manifest.py --write` ONLY when the corpus
//! genuinely moved; the manifest diff is then part of the corpus landing.

#![cfg(test)]

use serde_json::Value;

// The readers are dead code at `--no-default-features` (every consumer is a
// domain test, compiled only with its domain feature).
#[cfg(any(
    feature = "clippy_lints",
    feature = "rust_perf",
    feature = "kernel_opt",
    feature = "sec"
))]
/// The committed manifest, parsed once per call site need (test code —
/// parse cost is irrelevant; failing LOUD beats caching a stale read).
#[must_use]
pub(crate) fn manifest() -> Value {
    serde_json::from_str(include_str!("../.benchmarks/pins_manifest.json"))
        .expect("committed .benchmarks/pins_manifest.json must parse")
}

/// Read one manifest count with a loud, named failure — never a silent 0.
///
/// `key` may carry a pointer suffix (`"witness/experts"`).
#[cfg(any(
    feature = "clippy_lints",
    feature = "rust_perf",
    feature = "kernel_opt",
    feature = "sec"
))]
#[must_use]
pub(crate) fn count(domain: &str, key: &str) -> usize {
    manifest()
        .pointer(&format!("/{domain}/{key}"))
        .and_then(Value::as_u64)
        .map_or_else(
            || {
                panic!(
                    "pins_manifest.json: missing /{domain}/{key} — run \
                     `scripts/pins_manifest.py --write` and land the manifest \
                     diff with the corpus change"
                )
            },
            |v| usize::try_from(v).expect("manifest count fits usize"),
        )
}

/// The derivation half. A no-op unless `RIIR_REFINE_PINS_DUMP` names an
/// output path (the script sets it); REFUSES to emit a partial manifest —
/// a feature left off the build would silently shrink the counts, so the
/// dump asserts all four domains are present and the caller builds with
/// `--features kernel_opt,sec,rust_perf_bench` (`clippy_lints` +
/// `rust_perf` are default-on).
#[test]
fn dump_pins_manifest() {
    let Some(path) = std::env::var_os("RIIR_REFINE_PINS_DUMP") else {
        return; // ordinary test run — the derivation is the script's job
    };
    // Mutated only when a domain feature is on; `--no-default-features`
    // builds keep the shape (and the refusal below) compiled.
    #[allow(unused_mut)]
    let mut root = serde_json::Map::new();

    #[cfg(feature = "clippy_lints")]
    root.insert(
        "clippy_lints".to_string(),
        serde_json::json!({
            "rules": crate::domains::clippy_lints::corpus::build_seed_corpus().entries.len(),
        }),
    );

    #[cfg(feature = "rust_perf")]
    {
        let corpus = crate::domains::rust_perf::corpus::build_perf_seed_corpus();
        let categories: std::collections::BTreeSet<&str> =
            corpus.entries.iter().map(|e| e.category.as_str()).collect();
        // benchable/notbenchable need the BenchSpec field, which only
        // exists under `rust_perf_bench` — the script's build posture
        // enables it (checked by the required-keys assert below).
        #[cfg(feature = "rust_perf_bench")]
        root.insert(
            "rust_perf".to_string(),
            serde_json::json!({
                "rules": corpus.entries.len(),
                "categories": categories.len(),
                "benchable": corpus.entries.iter().filter(|e| e.bench_spec.is_some()).count(),
                "notbenchable": corpus.entries.iter().filter(|e| e.bench_spec.is_none()).count(),
            }),
        );
        #[cfg(not(feature = "rust_perf_bench"))]
        root.insert(
            "rust_perf".to_string(),
            serde_json::json!({
                "rules": corpus.entries.len(),
                "categories": categories.len(),
            }),
        );
    }

    #[cfg(feature = "kernel_opt")]
    root.insert(
        "kernel_opt".to_string(),
        serde_json::json!({
            "rules": crate::domains::kernel_opt::corpus::build_kernel_seed_corpus().entries.len(),
            "witness": {
                "kernel_categories":
                    crate::domains::kernel_opt::corpus::KERNEL_CATEGORIES.len(),
                "experts": crate::domains::kernel_opt::router::NUM_KERNEL_EXPERTS,
                "activity_dim": crate::domains::kernel_opt::router::ACTIVITY_DIM,
            },
        }),
    );

    #[cfg(feature = "sec")]
    root.insert(
        "sec".to_string(),
        serde_json::json!({
            "rules": crate::domains::sec::corpus::build_sec_seed_corpus().entries.len(),
        }),
    );

    for required in [
        "clippy_lints",
        "rust_perf",
        "kernel_opt",
        "sec",
        "rust_perf_bench",
    ] {
        let present = match required {
            // The benchable/notbenchable pair proves the rust_perf_bench posture.
            "rust_perf_bench" => root
                .get("rust_perf")
                .is_some_and(|r| r.get("benchable").is_some()),
            other => root.contains_key(other),
        };
        assert!(
            present,
            "pins dump: domain/posture `{required}` missing — the dump must run \
             at `--features kernel_opt,sec,rust_perf_bench` (clippy_lints + \
             rust_perf are default); a partial manifest is refused, never written"
        );
    }

    let json = serde_json::to_string_pretty(&Value::Object(root)).expect("manifest serializes");
    std::fs::write(&path, json + "\n").expect("pins dump write");
}
