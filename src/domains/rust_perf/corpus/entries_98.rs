// Batch 408 — the TKCC road-blog transcription (Plan 407 / riir-ai Research
// 440 + riir-infer note 030): together.ai/blog/road-to-together-compiler,
// published 2026-11-16, POST-ONLY — the blog URL is the pin (the B353/B363
// live-fetch precedent), every quote below byte-verified against the LIVE
// text at this batch's T0 (2026-11-24).
//
// rust_perf 393 → 396: three measurement-honesty rules, all suggestion-only
// NotBenchable (the metric-mismatch class — each rule's win is a
// measurement-honesty property no validator hosts; the dial9/136 annotation):
// - `benchmark-input-distribution-realism` — a kernel timed on synthetic
//   inputs lies whenever its memory behavior is input-distribution-dependent:
//   all-zeros AND uniformly-random are BOTH lies, in opposite directions
//   (zeros maximize locality, random maximizes dispersion); the reading that
//   decides must come from the workload's real distribution, and where no
//   real sample exists the DISTRIBUTION axis is swept and the band reported,
//   never a single synthetic point.
// - `isolated-bench-cache-state-vs-live` — a cache-flushed isolated kernel
//   bench prices predecessor-kernel cache reuse as cold (39–58% slower than
//   live); selection decisions built on the flushed posture alone carry a
//   built-in cold-start tax; bench the candidate both isolated AND in-situ,
//   select on the live posture, report both.
// - `harness-overhead-contaminates-us-kernels` — a microbenchmark's own
//   per-iteration overhead (launch machinery + timer reads) sets a FLOOR
//   under every reading; a no-work operation still reads 2.5 µs, and a
//   selection between candidates whose true delta sits inside that floor is
//   a coin flip priced as a decision (the Triton-over-cuBLAS −15%).
//
// The kernel_opt half of the batch is count-neutral (5 faces on their
// owners, same commit). Fixtures: fixtures/b408_fixtures.rs (standalone,
// debug AND -O, executed pre-entry). NOTHING INVENTED — every quote read at
// the live blog this session. Two staging corrections carried (the honest
// T0 record): the staged "random routing activates 13–38% more MoE experts"
// datum is NOT re-verifiable in the live text (likely a figure annotation)
// and is NOT carried in any rule; the staged "3 of 5 edges FULL, only the
// attention edge fine-grained" paraphrase is sharpened to the live text's
// shape (3 full + 1 effectively-full + 1 genuinely fine-grained).
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 408 — the TKCC road-blog `rust_perf` rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b408_rules() -> Vec<PerfRule> {
    vec![
        r1_benchmark_input_distribution_realism(),
        r2_isolated_bench_cache_state_vs_live(),
        r3_harness_overhead_contaminates_us_kernels(),
    ]
}

// ---------------------------------------------------------------------------
// Rule 1 — benchmark-input-distribution-realism
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r1_benchmark_input_distribution_realism() -> PerfRule {
    PerfRule {
        rule_id: "benchmark-input-distribution-realism".to_string(),
        name: "A kernel or hot function timed on SYNTHETIC inputs lies whenever its \
               memory behavior is input-distribution-dependent — all-zeros AND \
               uniformly-random are BOTH lies, in opposite directions (zeros maximize \
               locality, random maximizes dispersion), so the reading that decides \
               comes from the workload's real input distribution (the serving \
               histogram, the real token ids, the real routing skew); where no real \
               sample is available the DISTRIBUTION axis is swept and the band \
               reported, never one synthetic point presented as the number"
            .to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: a benchmark suite that times a gather/lookup kernel on
// one synthetic input distribution and reports the number as THE number.

#[bench]
fn bench_embedding_lookup(b: &mut Bencher) {
    // ids initialized to zero — every lookup hits the same row
    let ids = vec![0u32; SEQ_LEN];
    b.iter(|| embedding_gather(&table, &ids));
    // reads ~2x fast on a 64K sequence: artificially high cache reuse
}

#[bench]
fn bench_moe_grouped_gemm(b: &mut Bencher) {
    // uniformly-random routing — the opposite lie: max expert spread
    let routes = random_routing(NUM_EXPERTS, TOKENS);
    b.iter(|| grouped_gemm(&weights, &routes));
    // reads 4-21% slower than the real routing distribution
}
"#
        .to_string(),
        fixed_code: r#"// FIXED: the deciding measurement runs on the workload's real input
// distribution; the synthetic extremes are swept as a BOUND and reported
// as a band, never as the number.

#[bench]
fn bench_embedding_lookup_real(b: &mut Bencher) {
    // ids sampled from the serving histogram (the real corpus's token
    // frequency), the synthetic extremes recorded as the bracket
    let ids = real_token_ids(SEQ_LEN);
    b.iter(|| embedding_gather(&table, &ids)); // THE number
    // disclosure: all-same-id reads ~2.1x fast (locality inflation),
    // uniform-random reads the dispersion pessimism — the band is in
    // the report, the decision keys on the real-distribution row.
}

#[bench]
fn bench_moe_grouped_gemm_real(b: &mut Bencher) {
    let routes = real_routing_skew(NUM_EXPERTS, TOKENS);
    b.iter(|| grouped_gemm(&weights, &routes)); // 4-21% tighter than random
}

#[test]
fn distribution_band_brackets_the_truth() {
    // the law's own check: the synthetic extremes BRACKET the real read
    // when memory behavior is distribution-dependent; a real-distribution
    // reading outside the band means the band is mis-measured.
}
"#
        .to_string(),
        context_notes: Some(
            "Grade B, NotBenchable-by-domain (the metric-mismatch class — the win is a \
             measurement-honesty property: no validator hosts a distribution-realism \
             verdict). SOURCE: together.ai/blog/road-to-together-compiler (2026-11-16, \
             POST-ONLY — the blog URL is the pin; quotes byte-verified against the live \
             text 2026-11-24, Plan 407 T0). THE TWO-SIDED LAW, in-source: zeros — 'when \
             benchmarking `torch.embedding`, Inductor initializes the tensor of token \
             IDs to zero, causing every token to access the same embedding vector. For \
             a 64K-token sequence, this made the kernel **2.1× faster than with random \
             token IDs**, due to artificially high cache reuse.' Random — 'Random \
             inputs overestimate MoE grouped-GEMM latency by 4–21% on GLM 5.3' + body: \
             'PyTorch's default benchmarking can be anywhere from 4-21% slower than if \
             we had modified the benchmark to use realistic inputs for the MoE \
             grouped-GEMM kernel.' THE LAW'S OWN TAKEAWAY (verbatim): 'accurate \
             benchmarking requires taking measurements from inside a real forward pass \
             on real input data.' THE CATASTROPHIC TAIL (the selection-consequence \
             instance): an agent-written top-k indexer that 'grouped attention scores \
             into 2,048 histogram bins... On random inputs, this ran in 20 µs. But \
             real inputs contained many exact zeros from ReLU, concentrating candidates \
             into one bin, triggering quadratic work, and increasing runtime to 62 \
             ms' — three orders of magnitude, invisible to every synthetic-distribution \
             bench. THE DIRECTIONALITY IS THE DECISION CONTENT: zeros and randoms are \
             not two flavors of one error — they lie in OPPOSITE directions (zeros \
             inflate locality → read fast; random inflates dispersion → read slow), so \
             'average the two synthetics' is not a remedy; only the real distribution \
             decides, and the extremes serve as the reported band. ADJUDICATION \
             (mandatory, both directions): vs `calibration-probe-swept-band-single-\
             point-outlier` (entries_94) — THAT owns the sweep-vs-point axis for a \
             calibration INPUT (one outlier probe point vs a swept band over the \
             workload's real range); THIS owns the distribution-SHAPE axis (the band's \
             endpoints are themselves lies in opposite directions — compose: sweep the \
             band AND key the deciding row on the real distribution); vs entries_08's \
             input-pinning rider (the sybil fixed-panel) — THAT owns CROSS-ARM \
             consistency (the same panel both sides so the A/B reads see one \
             distribution); THIS owns the panel's CONTENT (a fixed panel of synthetic \
             rows is consistently wrong — compose: fixed AND real-shaped); vs \
             `degenerate-output-invalidates-throughput-claim` (entries_84) — THAT owns \
             the OUTPUT-quality axis (degenerate OUTPUT invalidates a rate); THIS owns \
             the INPUT-distribution axis (degenerate INPUT invalidates a timing) — \
             mirror poles of the same honesty family, never merged; vs the io family's \
             page-cache honesty caveats (B118/B139/B145, 'page-cache state dominates \
             any microbench') — those are the HOST-storage instance of the same moral \
             (state-dependent readings), cited as the family's precedent, never \
             re-owned. WORKLOAD FACE (count-neutral, live-found at the T0 re-read — \
             unstaged, disclosed): the distribution axis generalizes to the workload \
             regime — 'enabling freezing improved throughput by 10–26% at larger batch \
             sizes, where the GPU performs matrix-matrix multiplications. However, at \
             batch size 1... enabling the same flag caused a **49% slowdown**' (the \
             FP32 conversion was not reversed for matrix-vector ops) — a config chosen \
             on one workload regime inverts on another; the bench that arms a config \
             must run the serving regime (batch size, sequence shape), never the \
             bench-only regime. REGIME: lookup/gather/embedding/top-k/routing/hash — \
             any kernel whose cache or control-flow behavior is a function of the \
             input VALUES (histograms, binning, sparse dispatch). ANTI-REGIMES: \
             compute-dense kernels with static access patterns (dense GEMM over \
             contiguous tiles — the input distribution moves nothing, the caveat must \
             not fire); inputs that are genuinely the workload's own uniform draw. \
             FIXTURES: b408 F1 (the gather-over-table pair: all-same-id vs \
             uniform-random vs skew-real reads — the band brackets the real read; the \
             histogram/binning blowup pair: concentrated-vs-uniform keys; the tamper \
             arm = the confident single-point synthetic number). NotBenchable \
             suggestion-only (a parse-only proposer cannot see a bench's input \
             distribution). Source: Together AI, 'Last year we built megakernels. This \
             year we deleted them: The road to Together Compiler', Part 1 Finding #2 \
             + Part 2's agent-kernel anecdote. Batch 408 / Plan 407."
                .to_string(),
        ),
        // Not benchable: the win axis is a distribution-realism verdict;
        // no validator hosts a synthetic-vs-real input pair (the dial9/136
        // class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 2 — isolated-bench-cache-state-vs-live
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r2_isolated_bench_cache_state_vs_live() -> PerfRule {
    PerfRule {
        rule_id: "isolated-bench-cache-state-vs-live".to_string(),
        name: "A cache-flushed isolated kernel benchmark prices predecessor-kernel \
               cache reuse as COLD — in a real forward pass the same kernel often \
               reads data its predecessors left hot, so the isolated reading \
               overestimates live time by a measured band (39–58% in the source's \
               L2-flush harness); a selection between candidates built ONLY on the \
               flushed posture carries that cold-start tax, so each candidate is \
               measured BOTH isolated (the stable comparison) AND in-situ inside the \
               real sequence (the posture that decides), and the two readings are \
               reported together"
            .to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: candidates ranked only under the flushed isolated posture;
// the selection ships on numbers the live sequence will never see.

fn select_kernel(candidates: &[KernelBench]) -> &KernelBench {
    // every candidate was benched after a full L2 flush
    candidates.iter().min_by(|a, b| a.flushed_us.total_cmp(&b.flushed_us)).unwrap()
}

// the flushed table:
//   candidate A (flushed) 1.15 us   <- selected
//   candidate B (flushed) 1.22 us
// live in the forward pass (predecessor leaves the KV slice hot):
//   candidate A 1.40 us   candidate B 1.31 us  <- the ranking FLIPS
"#
        .to_string(),
        fixed_code: r#"// FIXED: both postures measured; the in-situ reading decides, the
// isolated reading stays beside it as the controlled comparison.

struct KernelBench {
    flushed_us: f64, // stable, controlled — the comparison anchor
    in_situ_us: f64, // the live posture — the one that decides
}

fn select_kernel(candidates: &[KernelBench]) -> &KernelBench {
    // select on the posture the kernel actually runs in
    candidates.iter().min_by(|a, b| a.in_situ_us.total_cmp(&b.in_situ_us)).unwrap()
}

#[test]
fn flushed_ranking_flip_is_disclosed() {
    // the law's own check: when the flushed and in-situ rankings
    // disagree, the report names the flip — never ships the flushed
    // ranking silently.
}
"#
        .to_string(),
        context_notes: Some(
            "Grade B, NotBenchable-by-domain (the metric-mismatch class). SOURCE: \
             together.ai/blog/road-to-together-compiler (2026-11-16, POST-ONLY — the \
             blog URL is the pin; quotes byte-verified against the live text \
             2026-11-24, Plan 407 T0). THE LAW, in-source: 'Torch Inductor clears the \
             GPU's L2 cache before benchmarking individual kernels, but during \
             inference, kernels often reuse data that is cached from preceding \
             operations. We observed **39–58% slower timings in Inductor's isolated \
             benchmarks** than in the live forward pass, where cache reuse can reduce \
             memory traffic.' The witness table (all four rows verified): Block-mask \
             slice (KV indices) live 1.22 vs benched 1.92 µs (+57.9%); Block-mask \
             slice (block counts) 1.34 vs 2.05 (+52.4%); Fused QKV split / V-cache \
             write 1.15 vs 1.66 (+44.5%); Embedding lookup / normalization 1.40 vs \
             1.95 (+38.8%). THE BLOG'S OWN SEPARATION IS THE AXIS MAP: this is \
             Finding #2's FIRST mechanism (caching behavior), distinct from its \
             SECOND (benchmarking overhead — the timer-floor rule of this batch); \
             one misleads by the cache STATE, the other by the TIMER'S floor — a \
             harness can carry both defects at once and neither subsumes the other. \
             DIRECTION DISCLOSED: the isolated reading OVERESTIMATES live time \
             (39–58% slower) — the mirror-image class exists too (the B398 \
             sparse-video note: µbench 15× vs live 1.33×, an isolated reading that \
             exaggerates a WIN) and rides `work-removed-build-is-not-a-valid-lower-\
             bound` (entries_84, count-neutral); both instances teach one law — the \
             isolated posture is a COMPARISON anchor, never the deciding number. \
             ADJUDICATION (mandatory, both directions): vs `adoption-trial-rotate-\
             operands-per-launch-dram-streaming` (gpu_60) — THAT owns operand \
             RESIDENCY as a design choice (rotate operands to stream vs let them \
             cache); THIS owns the measurement POSTURE (flushed vs in-situ) — \
             compose: a residency redesign must be measured in the posture it will \
             serve in, and this rule forbids deciding it from the flushed table \
             alone; vs the io family's page-cache honesty caveats (B118 prefault \
             admission, B139 PLE, B145 mincore probe — 'page-cache state dominates \
             any microbench' recorded three times) — the HOST-storage precedent of \
             the same moral; THIS generalizes to the kernel/L2 layer where the \
             predecessor is another KERNEL, cited never merged; vs \
             `wall-clock-ungated-shared-ci-noise-floor` (entries_79) — THAT owns \
             which metric judges in a shared environment; THIS owns which cache \
             STATE the reading must see; vs `calibration-probe-swept-band-single-\
             point-outlier` (entries_94) — the sweep axis composes (sweep sizes AND \
             postures). REGIME: microsecond-class kernels benched for selection \
             inside a sequence (decode pipelines, fusion candidates, kernel-choice \
             tables); any A/B where both arms share a warmable working set. \
             ANTI-REGIMES: cold-start claims (first-touch latency is the CLAIM — \
             measure deliberately cold, the io family's pole); candidates with no \
             shared working set (the flushed and in-situ readings coincide; the \
             dual measurement is then a formality, not a defect); macrobenches \
             where the kernel's own time dwarfs cache state. FIXTURES: b408 F2 (the \
             selection-flip pair: candidate A wins flushed, candidate B wins warm — \
             the naive flushed-only selector ships the wrong winner and the law's \
             check names the flip; the agree arm where postures coincide; the \
             tamper arm = the silent flushed-only report). NotBenchable \
             suggestion-only. Source: Together AI road blog, Part 1 Finding #2 \
             first mechanism. Batch 408 / Plan 407."
                .to_string(),
        ),
        // Not benchable: the win axis is a posture-honesty verdict; no
        // validator hosts a flushed-vs-in-situ pair (the dial9/136 class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 3 — harness-overhead-contaminates-us-kernels
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r3_harness_overhead_contaminates_us_kernels() -> PerfRule {
    PerfRule {
        rule_id: "harness-overhead-contaminates-us-kernels".to_string(),
        name: "A microbenchmark's own per-iteration overhead (dispatch machinery, \
               launch, timer reads) sets a FLOOR under every reading — a no-work \
               operation still measures 2.5 µs in the source's harness and readings \
               can exceed the profiler's GPU times by ~3.6 µs — so the harness floor \
               is MEASURED (an empty or no-GPU-work operation) before any \
               microsecond-class number is trusted, and a selection between \
               candidates whose true delta sits inside that floor is refused, never \
               priced"
            .to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: a selection table built on raw harness readings, no floor
// measured, deltas inside the floor priced as decisions.

fn select_decode_kernel(benches: &[(KnName, f64)]) -> KnName {
    // readings: triton 7.4, cublas 6.4 — a 1.0 us "win" priced as real
    benches.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0
}

// what the harness never disclosed: its own no-work operation reads
// 2.5 us, and readings exceed the profiler's GPU times by ~3.6 us —
// the 1.0 us "win" is inside the floor; the live pass showed the
// selection was a 15% REGRESSION.
"#
        .to_string(),
        fixed_code: r#"// FIXED: the floor is measured first; selections inside the floor
// refuse by name instead of shipping a coin flip.

fn harness_floor_us(harness: &mut BenchHarness) -> f64 {
    // a no-work operation: dispatch + launch + timer machinery only
    harness.measure_no_work_op() // the floor, measured not assumed
}

fn select_decode_kernel(benches: &[(KnName, f64)], floor_us: f64) -> Result<KnName, String> {
    let best = benches.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    let second = /* next-best */;
    if best.1 - second.1 <= floor_us {
        // the delta the harness can resolve is smaller than the floor
        // it adds: refuse, never price (the source's 7.4-vs-6.4 pick
        // shipped a 15% regression on exactly this shape)
        return Err(format!(
            "selection delta within harness floor ({floor_us} us); profile in-situ"
        ));
    }
    Ok(best.0)
}
"#
        .to_string(),
        context_notes: Some(
            "Grade C+, NotBenchable-by-domain (the metric-mismatch class). SOURCE: \
             together.ai/blog/road-to-together-compiler (2026-11-16, POST-ONLY — the \
             blog URL is the pin; quotes byte-verified against the live text \
             2026-11-24, Plan 407 T0). THE LAW, in-source: 'Many decode kernels \
             execute in just a few microseconds, making precise timing essential. We \
             found that Inductor's benchmarking harness reported **2.5 µs for an \
             operation that performed no GPU work**, while other measurements \
             exceeded the GPU execution times reported by Nsight Systems by \
             approximately 3.6 µs. This is because the harness includes CPU launch \
             overhead after an L2 cache flush, which results in Inductor recording \
             idle time.' THE SELECTION CONSEQUENCE (verbatim): 'On Llama 1B's \
             output-projection GEMM, Inductor selected a Triton kernel that ran \
             **7.4 µs versus 6.4 µs for cuBLAS** in the live forward pass—a 15% \
             slowdown.' THE BLOG'S OWN TAKEAWAY: 'accurate benchmarking requires \
             taking measurements from inside a real forward pass on real input \
             data' — the same sentence anchors this batch's rule 1; the two rules \
             split it by axis (rule 1 the input distribution, this the timer \
             floor). ADJUDICATION (mandatory, both directions): vs \
             `wall-clock-ungated-shared-ci-noise-floor` (entries_79) — THAT owns \
             the CI-granularity floor DERIVED from an empty-change baseline \
             (ms-class gate policy, which metric judges); THIS owns the \
             microbench-granularity floor (the harness's own per-iteration cost \
             inside ONE reading) — the same moral at the next scale down, compose \
             (an empty-change baseline IS a no-work operation read); vs \
             `instruction-count-cache-blindness` (entries_79) — THAT owns a METRIC \
             that cannot see locality (equal icount, divergent wall); THIS owns a \
             metric that sees TOO MUCH (overhead the op never paid) — opposite \
             failure directions of measurement-blindness, never merged; vs \
             `work-removed-build-is-not-a-valid-lower-bound` (entries_84) — THAT \
             owns an arm doing LESS work (the census refuses unequal arms); THIS \
             owns overhead ADDED to equal work (the floor contaminates both arms \
             equally and can still flip their ranking) — a floor defect survives \
             the work census, which is why both gates compose; vs \
             `token-count-from-authoritative-counter-not-stream-chunks` \
             (entries_84) — THAT owns the measurement UNIT (count what the \
             authority counts); THIS owns the measurement OFFSET (subtract what \
             the op never paid) — unit vs zero-point, both halves of an honest \
             reading; vs `degenerate-output-invalidates-throughput-claim` \
             (entries_84) — the OUTPUT-quality axis, distinct. THIN-BUT-KEPT \
             verdict recorded: one mechanism, but the axis (timer floor vs every \
             named neighbor) is unowned and the selection consequence is measured \
             (a 15% regression shipped); the keep is the honest call, the drop \
             would leave the floor axis nameless. IN-STACK CORROBORATION (not an \
             owner): katgpt-rs `timed_region_guard_gate` — timed regions whose \
             result is dead satisfy latency bars by ABSENT WORK (the floor's \
             mirror image: work deleted instead of overhead added); the two \
             defects read as plausible numbers from opposite directions. REGIME: \
             µs-class decode-kernel timing, dispatch-heavy harnesses, any A/B \
             whose delta is within the harness's own per-iteration cost. \
             ANTI-REGIMES: ms-class benches (the floor is noise, not signal — \
             entries_79's territory); profiling-instrument reads (the profiler's \
             own GPU-time row is the floor-free authority; the rule says measure \
             the floor, not abandon the harness). FIXTURES: b408 F3 (the floor \
             pair: a no-work op's nonzero reading + a selection whose true delta \
             sits inside the measured floor flipping run-to-run while a delta \
             well above the floor stays stable; the tamper arm = the floor-free \
             selector pricing the in-floor delta). NotBenchable suggestion-only. \
             Source: Together AI road blog, Part 1 Finding #2 second mechanism. \
             Batch 408 / Plan 407."
                .to_string(),
        ),
        // Not benchable: the win axis is a floor-honesty verdict; no
        // validator hosts a floor-contaminated selection pair (the
        // dial9/136 class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}
