//! The seed-corpus PRINTER (the `cloudflare/kat-service/tests/e2e_fixtures.rs`
//! marker pattern): emits the compiled-in clippy seed corpus as the
//! lease-plane CorpusPack JSON the kat-service worker publishes as its
//! bootstrap epoch segments (`src/seed_corpus.json`, `include_str!`-ed).
//!
//! Refresh flow (run from the riir-dapps repo, the consumer):
//!
//! ```sh
//! CARGO_TARGET_DIR=/tmp/kat_seed_printer cargo test -p riir-clippy \
//!   --test kat_seed_corpus_printer -- --nocapture \
//!   | sed -n 's/^<<<SEED_CORPUS \(.*\) >>>$/\1/p' > \
//!   ../riir-dapps/cloudflare/kat-service/src/seed_corpus.json
//! ```
//!
//! The JSON is DETERMINISTIC (no timestamp, entries in corpus order, the
//! builder's own sort by rule_id) — the committed file only moves when the
//! corpus itself moves, which is exactly what the lease commitment should
//! track. Gated `kat_service` because the corpus module is domain-gated
//! the same way the healer's is.

#![cfg(feature = "kat_service")]

use riir_clippy::{build_seed_corpus, HealEntry};

#[test]
fn print_seed_corpus() {
    let corpus = build_seed_corpus();
    let entries: Vec<serde_json::Value> = corpus
        .entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "rule_id": e.rule_id(),
                "buggy_code": e.buggy_code(),
                "fixed_code": e.fixed_code(),
            })
        })
        .collect();
    let pack = serde_json::json!({
        "pack_version": 1u32,
        "domains": [ { "domain": "clippy", "entries": entries } ],
    });
    println!(
        "<<<SEED_CORPUS {} >>>",
        serde_json::to_string(&pack).expect("seed corpus serializes")
    );
}
