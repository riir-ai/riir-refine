//! Stage 2 tuning demo: populate the adaptive pool via `cargo clippy`.
//!
//! This is the **slow, off-hot-path tuning loop** described in Issue 010's
//! fix direction. For each `(lint_name, buggy_code)` pair in the held-out
//! fixture, it:
//!
//! 1. Tries Stage 1 (the L1 drafter + identity guard).
//! 2. If Stage 1 declines, runs the per-lint AST proposer.
//! 3. Verifies each proposed fix against `cargo clippy`.
//! 4. Verified fixes join the Stage 2 adaptive pool.
//!
//! Run with:
//! ```sh
//! cargo run --release --example stage2_tuning_demo \
//!   --features self_evolve,clippy_verify
//! ```
//!
//! ## Expected output
//!
//! A `TuneReport` showing the breakdown of stage1_verified vs stage2_verified
//! fixes. The Stage 2 pool size after the run should be ≥1 (the GOAT gate
//! target from Issue 010: "Stage 1 + identity guard + Stage 2 must beat 0/10").

#![allow(clippy::pedantic)]

use riir_clippy::{
    build_seed_corpus, ClippyTernaryDrafter, ClippyVerifier, HealPipeline, RustAnalyzerPruner,
    TuneReport,
};

/** One held-out case (kept parallel to the bench fixture). */
#[derive(Debug, Clone)]
#[allow(dead_code)] // `want` is the canonical reference, not consumed by the tuning loop.
struct Case {
    lint: &'static str,
    buggy: &'static str,
    want: &'static str,
}

/// The 10 held-out cases (same as `tests/stage2_goat.rs`).
const HELDOUT: &[Case] = &[
    Case {
        lint: "needless_return",
        buggy: "fn bar(x: i32) -> i32 { return x + 1; }",
        want: "fn bar(x: i32) -> i32 { x + 1 }",
    },
    Case {
        lint: "redundant_closure",
        buggy: "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| sq(n)).collect() }",
        want: "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(sq).collect() }",
    },
    Case {
        lint: "needless_lifetimes",
        buggy: "fn head<'a>(v: &'a Vec<u8>) -> &'a u8 { &v[0] }",
        want: "fn head(v: &Vec<u8>) -> &u8 { &v[0] }",
    },
    Case {
        lint: "redundant_field_names",
        buggy: "struct Cfg { name: String, port: u16 }\nfn mk(name: String, port: u16) -> Cfg {\n    Cfg { name: name, port: port }\n}",
        want: "struct Cfg { name: String, port: u16 }\nfn mk(name: String, port: u16) -> Cfg {\n    Cfg { name, port }\n}",
    },
    Case {
        lint: "manual_map",
        buggy: "fn to_text(x: Option<u8>) -> Option<String> {\n    match x {\n        Some(v) => Some(v.to_string()),\n        None => None,\n    }\n}",
        want: "fn to_text(x: Option<u8>) -> Option<String> {\n    x.map(|v| v.to_string())\n}",
    },
    Case {
        lint: "unnecessary_unwrap",
        buggy: "fn get(r: Option<u32>) -> u32 {\n    if r.is_some() { r.unwrap() } else { 7 }\n}",
        want: "fn get(r: Option<u32>) -> u32 {\n    r.unwrap_or(7)\n}",
    },
    Case {
        lint: "needless_range_loop",
        buggy: "fn double_all(v: &Vec<i32>) -> Vec<i32> {\n    let mut out = Vec::new();\n    for i in 0..v.len() { out.push(v[i] * 2); }\n    out\n}",
        want: "fn double_all(v: &Vec<i32>) -> Vec<i32> {\n    v.iter().map(|x| x * 2).collect()\n}",
    },
    Case {
        lint: "useless_conversion",
        buggy: "fn twice(n: u64) -> u64 {\n    let m: u64 = n.into();\n    m * 2\n}",
        want: "fn twice(n: u64) -> u64 {\n    n * 2\n}",
    },
    Case {
        lint: "unnecessary_mut_passed",
        buggy: "fn count(v: &Vec<u8>) -> usize { v.len() }\nfn go(mut v: Vec<u8>) -> usize { count(&mut v) }",
        want: "fn count(v: &Vec<u8>) -> usize { v.len() }\nfn go(v: Vec<u8>) -> usize { count(&v) }",
    },
    Case {
        lint: "needless_borrow",
        buggy: "fn show(s: &str) -> usize { s.len() }\nfn go(s: &str) -> usize { show(&s) }",
        want: "fn show(s: &str) -> usize { s.len() }\nfn go(s: &str) -> usize { show(s) }",
    },
];

fn main() {
    println!("=== Stage 2 Tuning Demo (Issue 010 GOAT gate) ===\n");

    let corpus = build_seed_corpus();
    let mut pipeline =
        HealPipeline::new(ClippyTernaryDrafter::fallback(), RustAnalyzerPruner::new());
    pipeline.seed_from_corpus(&corpus.entries);

    let verifier = ClippyVerifier::new();
    if !verifier.is_cargo_available() {
        eprintln!("FATAL: cargo / clippy not on PATH — cannot verify fixes.");
        std::process::exit(1);
    }

    let cases_iter = HELDOUT.iter().map(|c| (c.lint, c.buggy));
    let report: TuneReport = pipeline.tune_against_fixture(cases_iter, &verifier);

    println!("\n--- TuneReport ---");
    println!("total            : {}", report.total);
    println!(
        "stage1_hits      : {} (already in pool)",
        report.stage1_hits
    );
    println!(
        "stage1_verified  : {} (L1 drafter + clippy pass)",
        report.stage1_verified
    );
    println!(
        "stage2_proposed  : {} (proposer fired)",
        report.stage2_proposed
    );
    println!(
        "stage2_verified  : {} (proposer + clippy pass)",
        report.stage2_verified
    );
    println!("verified total   : {}", report.verified_count());
    println!("failed           : {}", report.failed_count());
    println!("\nStage 2 pool size: {}", pipeline.adaptive().len());

    let gate_pass = report.verified_count() > 0;
    println!(
        "\n=== GOAT G1 (beat 0/10): {} ===",
        if gate_pass { "PASS" } else { "FAIL" }
    );
    if !gate_pass {
        std::process::exit(2);
    }
}
