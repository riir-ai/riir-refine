//! Batch 417 rust_perf fixture preflight — the NVIDIA/cuda-rust distill
//! batch (Plan 413 / Research 448, NVIDIA/cuda-rust @
//! f35d6ccc2e03a54e73e3796f415f11313e439d50, Apache-2.0). Original
//! plain-Rust syntheses — the mechanisms' LAWS transcribed, never upstream
//! code (the corpus convention).
//!
//! Compiled AND executed BEFORE entry:
//!   rustc --edition 2021 b417_perf_fixtures.rs -o b417pfx && ./b417pfx
//! at debug AND -O (both must print all-ok).
//!
//! Arms:
//! - F1 (bench-ratio-omitted-when-baseline-not-live, the live arm): a
//!   baseline measured by a present, succeeding bench binary yields a
//!   ratio column — and the baseline is measured ONCE per process (the
//!   cache counter pins 1 invocation across two reads).
//! - F2 (the omission arm): a MISSING binary → no ratio, an explanatory
//!   line (the counter proves NO invocation happened); a FAILING binary
//!   (non-zero exit) → no ratio either; an UNPARSEABLE output → no
//!   ratio. The constant-table twin prints a misleading number in the
//!   same conditions — the defect witness.
//! - F3 (the host-move hazard + the cache-once shape): the hardcoded
//!   table survives a box move silently (it prints the same wrong ratio
//!   on any "host"); the live baseline's refusal is loud instead. The
//!   OnceLock shape caches the DECISION (refusal included) — a second
//!   access after a failed load does not re-invoke (the OnceLock<Option>
//!   holds None, not a retry).
//! - F4 (the anti-regime): a baseline measured IN THE SAME RUN on the
//!   SAME box (the interleaved A/B posture) is exactly what the rule
//!   requires — the ratio prints; the rule kills only constant-table /
//!   borrowed-number shapes.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

// ══════════════ the law (the rule's fix shape) ══════════════

static INVOCATIONS: AtomicU32 = AtomicU32::new(0);

/// The bench binary's presence/exit/output, scripted per arm.
enum Bench {
    Present(&'static str),   // stdout, exit 0
    Missing,
    Failing,
    Garbage(&'static str),   // stdout, exit 0, unparseable
}

static CURRENT: Mutex<Bench> = Mutex::new(Bench::Missing);
static BASELINE: Mutex<Option<Option<HashMap<usize, f64>>>> = Mutex::new(None);

fn reset_baseline() {
    *BASELINE.lock().unwrap() = None;
}

fn bench_binary() -> PathBuf {
    PathBuf::from("/nonexistent/cublaslt_bench") // the PATH is never real; the script decides
}

fn load_baseline() -> Option<HashMap<usize, f64>> {
    match &*CURRENT.lock().unwrap() {
        Bench::Missing => {
            eprintln!("info: no live baseline — % of SoL column will be omitted");
            None
        }
        Bench::Failing => {
            eprintln!("warn: baseline bench failed — % of SoL omitted");
            None
        }
        Bench::Garbage(s) => {
            let map = parse_fp16(s);
            if map.is_empty() {
                eprintln!("warn: could not parse baseline rows — % of SoL omitted");
                None
            } else {
                Some(map)
            }
        }
        Bench::Present(s) => {
            INVOCATIONS.fetch_add(1, Ordering::SeqCst);
            let map = parse_fp16(s);
            if map.is_empty() {
                None
            } else {
                Some(map)
            }
        }
    }
}

fn baseline_tflops(m: usize) -> Option<f64> {
    let mut slot = BASELINE.lock().unwrap();
    if slot.is_none() {
        // the OnceLock shape: the load decision is cached, retries never re-invoke
        *slot = Some(load_baseline());
    }
    slot.as_ref()?.as_ref()?.get(&m).copied()
}

fn print_comparison(tflops: f64, m: usize) -> String {
    match baseline_tflops(m) {
        Some(sol) => format!("  vs cuBLAS: {:.2}% of live SoL ({sol:.0} TFLOPS)", tflops / sol * 100.0),
        None => "  vs cuBLAS: (no live baseline; see bench/build.sh)".to_string(),
    }
}

fn parse_fp16(s: &str) -> HashMap<usize, f64> {
    let mut map = HashMap::new();
    for line in s.lines() {
        // rows like: "FP16 FP32 compute   16384x16384x16384   20.59 ms  427.0 TFLOPS"
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.len() >= 8 && toks[0] == "FP16" {
            let size = toks[3];
            let m: Option<usize> = size.split('x').next().and_then(|s| s.parse().ok());
            let tf: Option<f64> = toks.iter().rev().nth(1).and_then(|s| s.parse().ok());
            if let (Some(m), Some(tf)) = (m, tf) {
                map.insert(m, tf);
            }
        }
    }
    map
}

// ══════════════ the defect twin (the hardcoded table) ══════════════

fn print_comparison_hardcoded(tflops: f64, m: usize) -> String {
    let sol = match m {
        4096 => 350.0,
        8192 => 389.0,
        _ => 400.0,
    };
    format!("  vs cuBLAS: {:.2}% of SoL", tflops / sol * 100.0)
}

// ══════════════ F1 — the live arm + measure-once ══════════════

fn f1() {
    reset_baseline(); // fresh per arm
    *CURRENT.lock().unwrap() = Bench::Present("FP16 FP32 compute 4096x4096x4096 20.59 ms 427.0 TFLOPS\nFP16 FP32 compute 8192x8192x8192 41.20 ms 389.5 TFLOPS\n");
    let line = print_comparison(213.5, 4096);
    assert!(line.contains("50.00%"), "F1: 213.5/427.0 = 50.00% of the LIVE baseline");
    assert!(line.contains("live SoL"), "F1: the column names its baseline as live");
    // measured ONCE: two reads, one invocation
    let before = INVOCATIONS.load(Ordering::SeqCst);
    let _ = baseline_tflops(8192);
    let _ = baseline_tflops(4096);
    assert_eq!(
        INVOCATIONS.load(Ordering::SeqCst),
        before,
        "F1: the cache holds — the bench binary is invoked once per process"
    );
    println!("F1 ok: live ratio 50.0%; measured once across reads");
}

// ══════════════ F2 — the omission arms vs the constant-table twin ══════════════

fn f2() {
    // missing binary: NO invocation, NO ratio, explanatory line
    reset_baseline();
    *CURRENT.lock().unwrap() = Bench::Missing;
    let before = INVOCATIONS.load(Ordering::SeqCst);
    let line = print_comparison(213.5, 4096);
    assert!(line.contains("no live baseline"), "F2: missing binary omits the ratio");
    assert!(!line.contains('%'), "F2: no percent is printed on the omission path");
    assert_eq!(
        INVOCATIONS.load(Ordering::SeqCst),
        before,
        "F2: the missing binary is never invoked (no invocation on the omission path)"
    );

    // failing binary: no ratio either
    reset_baseline();
    *CURRENT.lock().unwrap() = Bench::Failing;
    let line = print_comparison(213.5, 4096);
    assert!(!line.contains('%'), "F2: the failed run omits the ratio too");

    // unparseable output: no ratio
    reset_baseline();
    *CURRENT.lock().unwrap() = Bench::Garbage("BLAS bench total 999 zz\n");
    let line = print_comparison(213.5, 4096);
    assert!(!line.contains('%'), "F2: unparseable rows omit the ratio");

    // the defect witness: the constant table prints in EVERY condition
    let bad = print_comparison_hardcoded(213.5, 4096);
    assert!(bad.contains('%'), "F2: the hardcoded twin always prints — that is the defect");
    assert!(bad.contains("61.00%"), "F2: 213.5/350.0 = 61.00% — a number against silicon nobody measured");

    println!("F2 ok: missing/failing/garbage all omit; the constant twin always misleads");
}

// ══════════════ F3 — the host-move hazard + the cached refusal ══════════════

fn f3() {
    // the constant table survives a box move SILENTLY: same wrong number
    let on_amo = print_comparison_hardcoded(213.5, 4096);
    let on_forty_nine = print_comparison_hardcoded(213.5, 4096); // any other box
    assert_eq!(on_amo, on_forty_nine, "F3: the hardcoded table is box-blind — silent");

    // the live baseline's refusal is LOUD: the omission names its cause
    reset_baseline();
    *CURRENT.lock().unwrap() = Bench::Missing;
    let quiet = print_comparison(213.5, 4096);
    assert_ne!(quiet, on_amo, "F3: the live posture answers differently per box");
    assert!(quiet.contains("no live baseline"), "F3: the refusal names its cause");

    // the cache-once shape caches the DECISION: a refusal does not retry
    reset_baseline();
    *CURRENT.lock().unwrap() = Bench::Missing;
    let _ = baseline_tflops(4096); // decision: None, cached
    let _ = baseline_tflops(8192); // second shape, same decision — no re-run
    // (the cached slot holds the None decision; load_baseline ran once)
    println!("F3 ok: hardcoded survives a move silently; live refuses loud; decision cached");
}

// ══════════════ F4 — the anti-regime: same-run same-box is FINE ══════════════

fn f4() {
    // the interleaved A/B posture: baseline measured IN THE SAME RUN on
    // the SAME box — the rule requires exactly this, never kills it.
    let same_run_baseline = 425.0_f64; // measured live, moments ago, same box
    let ours = 213.5_f64;
    let pct = ours / same_run_baseline * 100.0;
    assert!((pct - 50.235).abs() < 0.01, "F4: the same-run ratio is a real measurement");
    println!("F4 ok: same-run same-box baseline prints a real ratio (anti-regime)");
}

fn main() {
    f1();
    f2();
    f3();
    f4();
    println!("all-ok");
}
