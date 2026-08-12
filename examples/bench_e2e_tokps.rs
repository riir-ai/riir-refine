//! E2E tokens/sec harness for the riir-clippy healing pipeline.
//!
//! Measures the **same deliverable** an LLM produces for the same task: given
//! `(lint_name, buggy_code)`, emit fixed Rust source. The metric is
//!
//! ```text
//! tok/s = tokens(emitted fix) / wall_clock(request → emitted fix)
//! ```
//!
//! Token counts are NOT computed here — the emitted fixes are written to a
//! JSONL file and tokenized by the companion script against the **same**
//! Bonsai tokenizer used by the llama.cpp arm (via the resident server's
//! `/tokenize` endpoint). That keeps the numerator apples-to-apples; this
//! harness owns only the denominator.
//!
//! The case list is NOT hardcoded here — it is read from a shared fixture so
//! both arms are driven by the same `(lint, buggy, want)` triples. Two fixtures
//! ship: `e2e_cases_seed.json` (in-distribution — the corpus holds the answer)
//! and `e2e_cases_heldout.json` (same lints, unseen code — the generalization
//! axis).
//!
//! Run with:
//! ```sh
//! cargo run --release --example bench_e2e_tokps --features "self_evolve" -- \
//!     benches/fixtures/e2e_cases_seed.json /tmp/clippy_e2e_seed.jsonl
//! ```

#![allow(clippy::pedantic)]

use riir_clippy::{
    build_seed_corpus, ClippyFixEntry, ClippyTernaryDrafter, HealPipeline, RustAnalyzerPruner,
};
use std::io::Write;
use std::time::Instant;

/// One benchmark case, shared with the llama.cpp arm via the fixture file.
#[derive(serde::Deserialize)]
struct Case {
    lint: String,
    buggy: String,
    want: String,
}

#[derive(serde::Deserialize)]
struct CaseFile {
    name: String,
    cases: Vec<Case>,
}

/// Realistic frozen direction-vector weights (64 rows x 256 cols), matching
/// the Bench 001 GOAT-gate scale. Deterministic ternary pattern — production
/// would thaw these from a `.bits` file.
fn realistic_weights() -> katgpt_core::TernaryWeights {
    let rows = 64;
    let cols = 256;
    let mut weights = vec![0.0f32; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            weights[i * cols + j] = match (i + j) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            };
        }
    }
    katgpt_core::TernaryWeights::quantize_from_f32(&weights, rows, cols)
}

fn percentile(sorted_ns: &[u128], p: f64) -> u128 {
    if sorted_ns.is_empty() {
        return 0;
    }
    let idx = ((sorted_ns.len() - 1) as f64 * p).round() as usize;
    sorted_ns[idx]
}

/// Whitespace-insensitive comparison — formatting is not a correctness axis.
/// Must match the Python arm's `normalize()` exactly.
fn normalize(code: &str) -> String {
    code.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn main() {
    let cases_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "benches/fixtures/e2e_cases_seed.json".to_string());
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/tmp/clippy_e2e.jsonl".to_string());

    let case_file: CaseFile = serde_json::from_str(
        &std::fs::read_to_string(&cases_path).expect("read cases fixture"),
    )
    .expect("parse cases fixture");

    // Repetitions per lint. The pipeline is deterministic, so repetitions
    // measure latency dispersion, not answer variance.
    let reps: usize = std::env::var("REPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    // Which feature path to time. `self_evolve` is the repo's best-validated
    // flag (Issue 008: +50pp healing rate), but "best" must be measured, not
    // assumed — `MODE=l1l2` times the default-features path (L1 drafter + L2
    // pruner, no memory/evolution overlay) for the same fixture.
    let mode = std::env::var("MODE").unwrap_or_else(|_| "self_evolve".to_string());
    assert!(
        mode == "self_evolve" || mode == "l1l2",
        "MODE must be `self_evolve` or `l1l2`, got `{mode}`"
    );

    let corpus = build_seed_corpus();
    assert!(corpus.verify(), "corpus BLAKE3 commitment must verify");

    // ── Cold-start cost: process-visible setup (corpus + weights + memory) ──
    // The llama.cpp arm pays model load; this arm pays corpus thaw + seeding.
    // Measured separately, reported separately — no hiding it in the hot path.
    let t_setup = Instant::now();
    let weights = realistic_weights();
    let mut drafter = ClippyTernaryDrafter::with_weights(weights.clone());
    let pruner = RustAnalyzerPruner::new();
    let mut pipeline = HealPipeline::new(ClippyTernaryDrafter::with_weights(weights), RustAnalyzerPruner::new());
    if mode == "self_evolve" {
        pipeline.seed_from_corpus(&corpus.entries);
    }
    let setup_ns = t_setup.elapsed().as_nanos();

    let mut rows: Vec<String> = Vec::new();
    let mut all_ns: Vec<u128> = Vec::with_capacity(case_file.cases.len() * reps);
    let mut parse_ok = 0usize;
    let mut exact_ok = 0usize;
    let mut attempts = 0usize;

    println!("=== cases: {} ({}) ===", case_file.name, cases_path);

    for case in &case_file.cases {
        let lint = &case.lint;
        // The drafter's candidate pool is the corpus slice for this lint. On the
        // held-out fixture the pool holds a same-lint template, NOT the answer.
        let candidates: Vec<ClippyFixEntry> = corpus.lookup(lint).cloned().collect();
        if candidates.is_empty() {
            continue;
        }
        let buggy = case.buggy.clone();

        // One timed heal attempt in the mode under test. Returns
        // `(emitted_fix, parses)`; the closure body is the *only* thing between
        // the two clock reads.
        let one_call = |drafter: &mut ClippyTernaryDrafter,
                        pipeline: &mut HealPipeline|
         -> (u128, String, bool) {
            match mode.as_str() {
                "l1l2" => {
                    let t0 = Instant::now();
                    let draft = drafter
                        .draft_fix(lint, &buggy, &candidates)
                        .expect("L1 drafter returned None — corpus/lint mismatch");
                    let v = pruner.validate(&draft.code, lint);
                    (t0.elapsed().as_nanos(), draft.code, v.parses)
                }
                _ => {
                    let t0 = Instant::now();
                    let r = pipeline
                        .heal(lint, &buggy, &candidates)
                        .expect("heal returned None — corpus/pipeline mismatch");
                    (t0.elapsed().as_nanos(), r.fixed_code, r.parses)
                }
            }
        };

        // Warm-up: one untimed call so the scratch buffers reach steady state
        // (G4 alloc-free invariant — we measure the steady state, not warm-up).
        let _ = one_call(&mut drafter, &mut pipeline);

        let mut per_lint_ns: Vec<u128> = Vec::with_capacity(reps);
        let mut emitted = String::new();
        let mut parses = false;

        for _ in 0..reps {
            let (ns, code, ok) = one_call(&mut drafter, &mut pipeline);
            per_lint_ns.push(ns);
            emitted = code;
            parses = ok;
        }

        per_lint_ns.sort_unstable();
        all_ns.extend_from_slice(&per_lint_ns);
        attempts += 1;
        if parses {
            parse_ok += 1;
        }
        let exact = normalize(&emitted) == normalize(&case.want);
        if exact {
            exact_ok += 1;
        }

        let p50 = percentile(&per_lint_ns, 0.50);
        let p99 = percentile(&per_lint_ns, 0.99);

        // JSONL row: the emitted fix + its latency. External tokenizer supplies
        // the token count.
        rows.push(format!(
            "{{\"lint\":{},\"parses\":{},\"exact\":{},\"p50_ns\":{},\"p99_ns\":{},\"emitted\":{}}}",
            json_str(lint),
            parses,
            exact,
            p50,
            p99,
            json_str(&emitted),
        ));

        println!(
            "  {lint:<26} p50={:>8.2}µs  p99={:>8.2}µs  parses={parses:<5} exact={exact:<5} bytes={}",
            p50 as f64 / 1000.0,
            p99 as f64 / 1000.0,
            emitted.len(),
        );
    }

    all_ns.sort_unstable();
    let p50 = percentile(&all_ns, 0.50);
    let p99 = percentile(&all_ns, 0.99);
    let mean = all_ns.iter().sum::<u128>() as f64 / all_ns.len() as f64;

    println!();
    println!("=== riir-clippy e2e heal latency (MODE={mode}, release) ===");
    println!("lints         : {attempts}");
    println!("reps/lint     : {reps}");
    println!("setup (cold)  : {:.3} ms", setup_ns as f64 / 1e6);
    println!("p50           : {:.3} µs", p50 as f64 / 1000.0);
    println!("p99           : {:.3} µs", p99 as f64 / 1000.0);
    println!("mean          : {:.3} µs", mean / 1000.0);
    println!("parse-valid   : {parse_ok}/{attempts}");
    println!("exact-match   : {exact_ok}/{attempts}");

    let mut f = std::fs::File::create(&out_path).expect("create output jsonl");
    for row in &rows {
        writeln!(f, "{row}").expect("write row");
    }
    // Trailing summary object so the external scorer has the aggregate too.
    writeln!(
        f,
        "{{\"_summary\":true,\"mode\":{},\"fixture\":{},\"lints\":{attempts},\"reps\":{reps},\"setup_ns\":{setup_ns},\"p50_ns\":{p50},\"p99_ns\":{p99},\"mean_ns\":{:.0},\"parse_ok\":{parse_ok},\"exact_ok\":{exact_ok}}}",
        json_str(&mode),
        json_str(&case_file.name),
        mean
    )
    .expect("write summary");
    println!("\nwrote {out_path}");
}

/// Minimal JSON string escaper (no serde dep needed in the harness path).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
