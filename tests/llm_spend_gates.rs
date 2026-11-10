//! Issue 046 gate fixtures — the USD LLM daily-cap ledger.
//!
//! G1 (the issue's own gate): a planted $0.99-spent state declines the next
//! worst-case-over-reserve call; a fresh UTC day passes it. Plus the wiring
//! pins: the client's default model is priced (from_env's fail-fast never
//! refuses the designated model), and the reserve→reconcile→reopen cycle is
//! lossless through the file.
//!
//! ```bash
//! cargo test --features cf_workers_ai --test llm_spend_gates
//! ```
//!
//! The G2 alloc fixture lives in its OWN binary (`tests/llm_spend_g2_alloc.rs`)
//! — the `kat_meter_g2_alloc` convention: a global CountingAllocator counts
//! every thread's allocations, so it needs a single-test binary.

#![cfg(feature = "cf_workers_ai")]

use riir_refine::cf_workers_ai::CfClient;
use riir_refine::llm_spend::{
    CapReached, PRICING_VERSION, SPEND_VERSION, SpendError, SpendLedger, SpendState,
    today_epoch_days,
};

const QWEN: &str = "@cf/qwen/qwen3.8-27b";

fn tmp_file(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("llm_g1_{name}_{}", std::process::id()))
}

fn planted(spent: f64, day: u64) -> SpendState {
    SpendState {
        version: SPEND_VERSION,
        pricing_version: PRICING_VERSION,
        day,
        cap_usd: 1.0,
        spent_usd: spent,
        reserved_usd: 0.0,
        calls: 5,
        declines: 0,
        all_time_spent_usd: spent,
    }
}

/// **G1**: $0.99 spent under a $1.00 cap declines the next call whose
/// worst-case reserve crosses the remaining $0.01; a fresh UTC day resets
/// the accumulator and the SAME call passes.
#[test]
fn g1_planted_99_cents_declines_then_a_fresh_day_passes() {
    let file = tmp_file("g1");

    // Today, $0.99 spent: the worst case of 8192 max out-tokens at qwen's
    // $3.20/M is ≈ $0.0262 — over the $0.01 that remains.
    std::fs::write(
        &file,
        serde_json::to_string(&planted(0.99, today_epoch_days())).unwrap(),
    )
    .unwrap();
    let mut l = SpendLedger::open_at_with_cap(file.clone(), 1.0);
    let err = l.reserve(QWEN, "fn f() {}", 8192).unwrap_err();
    let SpendError::CapReached(CapReached {
        spent, wanted, cap, ..
    }) = &err
    else {
        panic!("expected CapReached, got {err:?}");
    };
    assert!((*spent - 0.99).abs() < 1e-9, "planted spend carried");
    assert!(
        *wanted > 0.01,
        "worst case crosses the remainder (got {wanted:.5})"
    );
    assert!((*cap - 1.0).abs() < 1e-9);
    assert_eq!(l.state().declines, 1, "the decline is counted");

    // A stale (pre-today) day rolls: spent resets, and the SAME call passes.
    std::fs::write(&file, serde_json::to_string(&planted(0.99, 0)).unwrap()).unwrap();
    let mut fresh = SpendLedger::open_at_with_cap(file.clone(), 1.0);
    assert!(
        fresh.state().spent_usd.abs() < 1e-12,
        "rollover reset the accumulator"
    );
    let reserved = fresh
        .reserve(QWEN, "fn f() {}", 8192)
        .expect("fresh UTC day passes the same call");
    assert!(reserved > 0.0);
    let _ = std::fs::remove_file(file);
}

/// The wiring pin: `CfClient::from_env`'s fail-fast pricing check can never
/// refuse the designated default model (or the Bench 049 measured arm).
#[test]
fn the_default_and_measured_models_are_priced() {
    let ids = [
        CfClient::DEFAULT_MODEL,
        "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
    ];
    for id in ids {
        assert!(
            riir_refine::llm_spend::price_for(id).is_some(),
            "from_env would refuse {id} — keep the pricing table in step with the client"
        );
    }
}

/// The caller-facing cycle the harness runs: reserve → reconcile(actual) →
/// save → reopen, losslessly through the persisted file.
#[test]
fn reserve_reconcile_save_reopen_is_lossless() {
    let file = tmp_file("cycle");
    let mut l = SpendLedger::open_at_with_cap(file.clone(), 1.0);
    let reserved = l.reserve(QWEN, "fn main() { let x = 1; }", 512).unwrap();
    l.reconcile(reserved, QWEN, 210, 96, true);
    l.save().unwrap();
    let reopened = SpendLedger::open_at_with_cap(file.clone(), 1.0);
    assert_eq!(
        reopened.state(),
        l.state(),
        "state survives the file round-trip"
    );
    assert!((reopened.state().spent_usd - (210.0 * 0.45 + 96.0 * 3.2) / 1e6).abs() < 1e-12);
    let _ = std::fs::remove_file(file);
}
