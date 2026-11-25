// Batch 417 — the NVIDIA/cuda-rust (cuda-oxide) distill batch's `rust_perf`
// member (Plan 413 / Research 448). Source: github.com/NVIDIA/cuda-rust @ pin
// f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0; the verdict pin
// 5e5c77b19a4b… was one commit behind — the tip moved (#1866) and every quote
// was re-verified byte-exact at the NEW pin per the B414 re-pin discipline;
// #1866 touched none of this batch's quote sites). ONE rule:
// bench-ratio-omitted-when-baseline-not-live (measurement-honesty family —
// that family's home per the Batch-413 domain-routing precedent).
//
// Dedup re-verified at the landing HEAD (3+ vocabularies, every hit read):
// `hardcod.*baseline|live baseline|omit.*ratio|different silicon|SoL` over
// BOTH corpora — the `different silicon` hits are the foreign-silicon
// NotBenchable DISCLOSURES (the annotation class), not this rule; the
// measurement-honesty family (entries_70/79/84/94/98/100 + the kernel-side
// witnesses) owns the workload, the timer, the band, the denominator, and
// the cache posture — ZERO owner of the BASELINE'S PROVENANCE axis.

use crate::domains::rust_perf::PerfRule;

pub fn b417_rules() -> Vec<PerfRule> {
    vec![
        // ── R1: the ratio column exists only against a LIVE baseline ──
        PerfRule {
            rule_id: "bench-ratio-omitted-when-baseline-not-live".to_string(),
            name: "a benchmark report's ratio column (% of peak / % of SoL / speedup×) exists only against a baseline measured LIVE on the reporting box — a hardcoded, borrowed, or cross-silicon constant baseline is a misleading number: measure the baseline once per process (cached), and when it cannot be measured, OMIT the ratio column with an explanatory line instead of printing it".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// A bench report whose '% of SoL' compares against a
// HARDCODED table from another GPU — every host prints a
// misleading ratio against silicon it does not have.
fn print_cublas_comparison_naive(tflops: f64, m: usize) {
    let sol = match m {
        4096 => 350.0,   // B200 constants, hardcoded 2025
        8192 => 389.0,   // borrowed from a vendor deck
        16384 => 412.0,
        _ => 400.0,      // a guess for every other shape
    };
    let pct = tflops / sol * 100.0;
    println!("  vs cuBLAS: {pct:.1}% of SoL"); // misleading on ANY other box
}

fn run_bench(m: usize) {
    let tflops = measure_ours(m);
    print_cublas_comparison_naive(tflops, m);
}
"#.to_string(),
            fixed_code: r#"// LIVE baseline: measured once per process by invoking
// the reference bench, cached; if it cannot be measured, the
// ratio column is OMITTED — never printed against foreign silicon.
use std::collections::HashMap;
use std::sync::OnceLock;

static BASELINE: OnceLock<Option<HashMap<usize, f64>>> = OnceLock::new();

fn baseline_tflops(m: usize) -> Option<f64> {
    BASELINE
        .get_or_init(load_baseline)
        .as_ref()?
        .get(&m)
        .copied()
}

fn load_baseline() -> Option<HashMap<usize, f64>> {
    let bin = bench_binary();
    if !bin.exists() {
        eprintln!("no live baseline at {} — % of SoL omitted", bin.display());
        return None;
    }
    let out = std::process::Command::new(&bin).output().ok()?;
    if !out.status.success() {
        eprintln!("baseline bench failed — % of SoL omitted");
        return None;
    }
    parse_baseline(&String::from_utf8_lossy(&out.stdout))
}

fn print_cublas_comparison_live(tflops: f64, m: usize) {
    match baseline_tflops(m) {
        Some(sol) => {
            let pct = tflops / sol * 100.0;
            println!("  vs cuBLAS: {pct:.1}% of live SoL ({sol:.0} TFLOPS)");
        }
        // the source's own law: omit, never fabricate
        None => println!("  vs cuBLAS: (no live baseline; see bench/build.sh)"),
    }
}
"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (bench_spec: None — measurement-honesty axis, Batch 109 precedent: the rule's content is that a ratio against a non-live baseline is not a measurement, so there is no latency quantity to Pareto). SOURCE (NVIDIA/cuda-rust @ f35d6ccc2e03a54e73e3796f415f11313e439d50, tip one commit past the verdict pin — re-verified at the NEW pin, Apache-2.0; original fixtures, never source code): the gemm_sol harness's OWN REPAIR is the law, verbatim at the pin (cuda-oxide/crates/rustc-codegen-cuda/examples/gemm_sol/src/main.rs:72-84): 'LIVE cuBLASLt BASELINE (replaces the previous hardcoded B200 constants)' + 'The baseline is measured by `bench/cublaslt_bench` ... On first access we invoke that binary, parse the FP16 section of its output, and cache the result. If the binary is missing or fails, the per-phase reports omit the \"% of SoL\" column rather than printing a misleading number against a baseline measured on different silicon.' + the call-site comment (:195-198): 'Replaces the previous hardcoded `match m { ... }` blocks that compared every host GPU against B200's cublasLt SoL.' THE MECHANISM: a `OnceLock<Option<HashMap<shape, tflops>>>` — the bench binary invoked once, output parsed per shape, the result cached for the process; pre-warm at startup so the ~25 s measurement never lands mid-print; the None arm prints an explanatory placeholder, never a fabricated percent. FIRES AT HOME: this workspace's own box-state law (the bench_preflight PROVENANCE line, the league scoreboard's bench-box disclosure) is the same discipline from the measurement side — the baseline here, the box state there; neither travels. FAMILY ADJUDICATION (the measurement-honesty owners, read at the landing HEAD): benchmark-input-distribution-realism (entries_98) owns the INPUT distribution (zeros and random both lie); isolated-bench-cache-state-vs-live (entries_98) owns the CACHE POSTURE (flushed-microbench vs in-situ); acceptance-band-derived-from-own-axis-measured-cv (entries_100) owns the BAND derivation; specdec-gate-metric-normalized-by-drafts-per-round (gpu_161 family) owns the DENOMINATOR law; work-removed-build-is-not-a-valid-lower-bound (entries_84) owns the ABSENT-WORK census; wall-clock / alloc-proxy / icount / harness-overhead / instruction-count-cache-blindness own the TIMER and the instrument; degenerate-output owns the OUTPUT quality. THIS owns the COMPARISON BASELINE's provenance: is the number you divide by alive on THIS box — the one axis the family had no owner for (grep: `hardcod.*baseline` zero; `different silicon` hits are the foreign-silicon NotBenchable disclosures, the annotation class). THE ANTI-REGIME is the league's interleaved A/B posture: ratios against a baseline measured in the SAME run on the SAME box are exactly what this rule requires — the rule kills only the constant-table and borrowed-number shapes. The B200-absolute-numbers caveat rides: the blog's 2,118 TFLOP/s is provenance, never a bar. Fixtures pin the three arms (live-measured ratio / missing-binary omission / failed-run omission) + the cache-once property + the host-move hazard (the constant table survives a box move silently; the live baseline refuses instead). UPSTREAM: present at the pin (their landed decision — decision content only, never a PR; the outbound-PR lane is EMPTY by construction). Batch 417 / Plan 413 / Research 448.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
