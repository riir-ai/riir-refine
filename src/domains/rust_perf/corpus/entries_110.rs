// Batch 429 — the hunch eg2cpu distill (Plan 311 / Research 350,
// undrgrnd-labs/hunch @ `22cf73010b0d399894068b1b9d21c5e4101514b6`,
// MIT OR Apache-2.0, the verdict pin UNMOVED at T0 re-read 2026-11-28 —
// upstream tip 30f2b696 moved past it with sched.rs UNTOUCHED (the diff
// names docs/performance.md + simd.rs only; every sched.rs quote below is
// identical at BOTH pin and tip), the perf.md tables re-verified at the
// pin). THREE rules: the caller-fairness scheduler (B+ decision lead, the
// batch's strongest row) + the BFMMLA measured anti-regime (B) + the
// per-length pool table (B− content). ALL suggestion-only
// NotBenchable-by-domain — the dial9/B136 annotation class, the verdict's
// own gate (criterion 1 fails for DOMAIN reasons: fairness/contention
// tables, hardware-gated matrix units, encoder workloads live outside the
// PerfBenchValidator's host shape; the source ships source-measured tables,
// stronger provenance than a zero-number source).
//
// Dedup re-verified at landing HEAD `08dba70e` (both vocabularies): the
// scheduler's nearest kin entries_28 `empty-slot-first-victim-order`
// (victim ordering INSIDE one pool vs caller-FAIRNESS ACROSS pools of
// requests — placement vs scheduling, the Research-350 kinship note
// carried in-rule) + entries_02 `async-yield-point` (executor-cooperative
// fairness inside one task vs cross-caller dispatch fairness — in-rule);
// round-robin/linger/collection-window zero-owner; `bfmmla` zero-owner
// tree-wide; `per-length` pool zero-owner (entries_95's lane-split is a
// bar-STRUCTURE axis, unrelated); `efficiency cores` hits are kernel_opt
// GPU E-core rows (entries_gpu_50/60 — device work-stealing, not a CPU
// pool-size table).

use crate::domains::rust_perf::PerfRule;

pub fn b429_rules() -> Vec<PerfRule> {
    vec![
        // ── #1: caller-fair round-robin dispatch (B+ decision lead) ──
        PerfRule {
            rule_id: "caller-fair-round-robin-dispatch".to_string(),
            name: "a batched job multiplexer shared by many concurrent callers schedules for CALLER FAIRNESS, not job order: a lone request runs immediately (no window); under contention the first idle caller becomes the dispatcher and waits ONE collection window for stragglers (paid only while under the in-flight cap); workers then take jobs ROUND-ROBIN ACROSS callers (a BTreeMap range-scan from the last-served caller, wrapping) and LONGEST-FIRST WITHIN one caller's batch (so its batch doesn't end waiting on one long text); workers LINGER a short idle window instead of tearing down (a resubmitting caller finds them alive — otherwise every round pays full spawn); each job's result publishes at once; a mid-dispatch arrival takes the next free slot so one caller's large batch delays others by about one pass, never a whole batch; and a panicking job is isolated per-job (the caller's other results survive)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// A shared batched engine: jobs run in ARRIVAL order on
// a freshly-spawned worker per request. Two hazards: one
// caller's 512-text batch head-of-line blocks every other
// caller's single request behind the whole batch, and each
// request pays worker spawn/teardown again.
fn submit(engine: &Engine, texts: Vec<String>) -> Vec<Output> {
    let mut outs = Vec::with_capacity(texts.len());
    for t in &texts {
        outs.push(engine.run_one(t)); // serialized, arrival order
    }
    outs
}
"#.to_string(),
            fixed_code: r#"// Caller-fair dispatch: queue per caller, workers pick
// round-robin ACROSS callers (wrapping from the last one
// served), longest-first WITHIN the picked caller's batch,
// a short collection window under the in-flight cap, worker
// linger across rounds, per-job panic isolation.
fn submit_fair(engine: &Engine, caller: CallerId, texts: Vec<String>) -> Vec<Output> {
    let mut order: Vec<usize> = (0..texts.len()).collect();
    // longest first: the caller's batch doesn't end waiting
    // on one long text while its short ones already finished
    order.sort_by_key(|&i| std::cmp::Reverse(texts[i].len()));
    engine.enqueue(caller, order, texts)
}

// the dispatcher-side pick (wrapping scan = fair rotation):
fn pick(queue: &BTreeMap<CallerId, VecDeque<Job>>, last_caller: &mut CallerId) -> Option<Job> {
    let next = queue.range(*last_caller + 1..).next()
        .or_else(|| queue.iter().next())?; // wrap to the front
    *last_caller = *next.0;
    next.1.front().cloned()
}
"#.to_string(),
            context_notes: Some("NotBenchable — contention-fairness domain reason (the deciding metric is p99 latency under multi-caller contention; the validator has no multi-caller harness; the source numbers are THEIR workload, quoted as motivation: 86 req/s @ 32 callers, p99 466 ms). THE NAMED LAW, verbatim at the pin (undrgrnd-labs/hunch @ 22cf73010b0d, eg2cpu/src/sched.rs, MIT OR Apache-2.0; module doc lines 1-9): 'A lone request runs at once' + 'waits a short collection window so stragglers can join (only when fewer than max_in_flight() are queued)' — the window knob EG2_BATCH_WINDOW_US default 1000 µs, 'Only paid under contention; a lone request never waits' (:64-65); workers 'take the next job round-robin across callers' via a BTreeMap range-scan 'queue.range(self.last_caller + 1..).next().or_else(|| self.queue.iter().next())' — the WRAP (:46) — and 'last_caller' is stamped per pick (:52); within-caller order: 'A caller's own jobs go longest first, so its batch doesn't end waiting on one long text' (:97, the sort_by_key Reverse(len)); worker linger EG2_LINGER_US default 2000 µs, 'how long an idle dispatch worker waits for another request before leaving' (:58) — the teardown-else hazard named in the module doc: callers 'find the workers gone and the whole round restarts'; per-job panic isolation: catch_unwind per job, the result publishes as None and ONLY the owning caller's expect fires (:156-157, :108) — the other callers' results survive; mid-dispatch arrivals: 'a large batch from one caller delays others' single requests by about one pass, not by the whole batch' (module doc :9-10). ⛔ ADJUDICATIONS: distinct-from entries_28 `empty-slot-first-victim-order-save-before-overwrite` (victim ORDERING inside one cache pool vs caller-FAIRNESS ACROSS pools of requests — placement vs scheduling, compose never merge); distinct-from entries_02 `async-yield-point` (cooperative fairness INSIDE one task's executor vs cross-caller dispatch fairness ACROSS independent callers); the linger half composes with entries_13's wake-economics family (steal-batch/stealer side, send-batching/sender side — this is the DISPATCH-side linger). REGIME: in-process batched multiplexers over a bounded worker pool with heterogeneous job costs and multiple concurrent callers (embed/encode servers, batch-verify lanes, e2e harness drivers). ANTI-REGIME: single-caller pipelines (the solo fast-path is the whole story); uniform-cost jobs where arrival order IS fair. Batch 429 / Plan 311 / Research 350.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #2: the BFMMLA measured anti-regime (B) ──
        PerfRule {
            rule_id: "bfmmla-below-fp32-fmla-apple-m5".to_string(),
            name: "on Apple M5-class P-cores the BF16 matrix instruction is a measured ANTI-REGIME for compute-bound matmuls: a P-core issues 2.1 G BFMMLA/s (68 GFLOP/s), HALF its fp32 FMLA rate, so the bf16 path is SLOWER than plain NEON fp32 despite the narrower type — check the per-core instruction rate, not the type width, before routing a matmul family to a half-width hardware instruction; and the SME coupling: when SME is present it TAKES the matmuls, so the bf16 instruction path only ever matters SME-off".to_string(),
            category: "simd".to_string(),
            buggy_code: r#"// Router picks the bf16 matmul because "bf16 = less
// bandwidth, must be faster" — no per-core instruction-rate
// check, no SME-presence coupling.
fn pick_matmul(isa: Isa, sme_available: bool) -> MatMulImpl {
    if isa == Isa::Neon && cpu_has_bf16() {
        MatMulImpl::Bf16Mla // the "narrower type" default
    } else {
        MatMulImpl::F32Neon
    }
}
"#.to_string(),
            fixed_code: r#"// FIXED: the measured-rate router — bf16 BFMMLA only
// where it actually wins (or where SME is present, which
// takes the matmuls anyway), fp32 NEON as the default on
// M5-class where BFMMLA measures HALF the FMLA rate.
fn pick_matmul(isa: Isa, sme_available: bool) -> MatMulImpl {
    if sme_available {
        return MatMulImpl::SmeFmopa; // SME takes the matmuls
    }
    if isa == Isa::Neon && cpu_has_bf16() && bfmmla_wins_locally() {
        MatMulImpl::Bf16Mla // measured, never assumed
    } else {
        MatMulImpl::F32Neon // the M5-class winner
    }
}

// the micro-arm that decides (paired, same shapes, the
// sequential-AB interleaved posture on a quiet box):
fn bfmmla_wins_locally() -> bool {
    let f32_rate = measure_gflops(MatMulImpl::F32Neon);
    let bf_rate = measure_gflops(MatMulImpl::Bf16Mla);
    bf_rate > f32_rate // M5-class: false (68 vs 90-93 GFLOP/s)
}
"#.to_string(),
            context_notes: Some("NotBenchable-by-platform at the validator (hardware-gated: aarch64 + FEAT_BF16 + SME absent-or-off; the row's fixture gates on std::arch::is_aarch64_feature_detected!(\"bf16\") with a scalar/fp32-NEON fallback, so non-ARM builds compile the fallback arm only). THE MEASURED TABLE, verbatim at the pin (undrgrnd-labs/hunch @ 22cf73010b0d, docs/performance.md:55-67, the source's own M5 Pro measured A/B — provenance, never a bar): 'a P-core issues 2.1 G BFMMLA/s (68 GFLOP/s), half its fp32 FMLA rate, so the bf16 path is slower than NEON fp32'; matmul one P-core 90–93 vs 65–68 GFLOP/s; 6 threads 410–460 vs 300–357; 18 threads 550–1,030 vs 300–683; 1×16tok 16.8–17.0 vs 22.5–23.0 ms; 1×128tok 68.8–69.0 vs 96.5–98.0 ms; 256×32tok 3,810–3,920 vs 2,520–2,790 tokens/s; e2e ×1.4 worse. THE COUPLING, verbatim (:55): 'applies only with EG2_SME=off here, since SME takes the matmuls when present' — and the dispatch site (:445-447): 'SME matmuls take precedence: with sme::available, bfmmla is ignored'. The corpus's measured-anti-regime class ('so the shape is not re-tried blind'): the row exists so a future bf16-kernel port does not re-derive the loss. This row is the SMALLEST sibling of the SME lane intel (Research 350 candidate 4: SME fp32 FMOPA is the WINNER arm at 1.5 TFLOP/s single-core / 2–2.7 TFLOP/s matmul shapes — routed to the riir-infer laya CPU posture, never a corpus rule). REGIME: compute-bound encoder-shaped GEMMs on aarch64 with FEAT_BF16, SME absent-or-off. ANTI-REGIME: bandwidth-bound matmuls (bf16's narrower loads can still win there — the rate is an ALU-rate statement, not a bandwidth one); x86 (bf16 VNNI-family rates differ); GPUs. Batch 429 / Plan 311 / Research 350.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── #3: per-length thread-pool selection (B− content) ──
        PerfRule {
            rule_id: "per-length-thread-pool-selection".to_string(),
            name: "a variable-length small-forward serving lane sizes its thread pool PER INPUT-LENGTH BAND, not one global pool: short texts want the SMALL pool (8 threads), longer texts the larger one (12) — both measured directions (under-threading costs 2× at mid lengths: with 4 threads a 32-token text takes twice as long as a 16-token text; over-threading straggles on heterogeneous cores: past the P-core count, matmul tasks land on the efficiency cores' slower matrix units and stall the wave) — with a floors table (e.g. 40:8,160:12) tuned against ALTERNATING A/B runs per band".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// One global pool for every input length: short texts
// oversubscribe (straggle on E-core matrix units), mid
// texts under-thread (the 2x cliff), long texts never
// reach the bigger pool.
static POOL: LazyLock<ThreadPool> = LazyLock::new(|| {
    ThreadPoolBuilder::new().num_threads(12).build().unwrap()
});

fn forward(texts: &[&str]) -> Vec<Embedding> {
    POOL.install(|| texts.par_iter().map(embed).collect())
}
"#.to_string(),
            fixed_code: r#"// FIXED: a floors table keyed by input length — the band
// picks the pool; the table is tuned per machine with
// ALTERNATING A/B runs per band (never one-shot).
//   env: EG2_POOLS="40:8,160:12"  (le40->8 threads, le160->12)
static POOLS: LazyLock<LengthPools> = LazyLock::new(|| {
    LengthPools::from_floors(std::env::var("EG2_POOLS")
        .as_deref().unwrap_or("40:8,160:12"))
});

fn forward(texts: &[&str]) -> Vec<Embedding> {
    let band = texts.iter().map(|t| t.len()).max().unwrap_or(0);
    POOLS.for_len(band).install(|| {
        texts.par_iter().map(embed).collect()
    })
}
"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the deciding metric is encoder-workload tokens/s per length band; the validator has no encoder host). THE MEASURED TABLE, verbatim at the pin (undrgrnd-labs/hunch @ 22cf73010b0d, docs/performance.md:38-53, the source's own M5 Pro alternating A/B — provenance, never a bar): the default floors '40:8,160:12' against '40:4,80:8,160:12' — 1×32tok 25.6–26.8 vs 32.1–35.5 ms (the under-thread 2×-class at mid length), 16/128/512-token bands PARITY (the band boundary is real, not universal); 'Texts of 48 and 64 tokens run 2–6% faster on 12 threads than on 8 threads. 8 threads suit texts of up to 40 tokens, even with only 6 P-cores. With 4 threads, a 32-token text takes twice as long as a 16-token text' (:53). THE OVER-THREADING COROLLARY, verbatim (:34): 'The vision tower runs on 8 threads: on more, matmul tasks land on the efficiency cores' slower matrix unit and straggle' — the matrix unit is per-cluster (P-cluster vs E-cluster), so threads past the P-core count EXPORT work to slower units; on SME machines fewer threads per text pay off (:22, 'shared by the cores of a cluster, so with SME fewer threads per text pay off'). The tuning METHOD is part of the law: the source's table is 'Against ... alternating runs' — the sequential-AB law's interleaved posture, never one-shot cells. ⛔ ADJUDICATIONS: distinct-from entries_gpu_50/60's E-core rows (GPU host work-stealing/thread-shape vs a CPU pool-size TABLE); distinct-from rayon's own auto-sizing (the pool count is per-BAND, rayon sizes globally); the under-threading half is the familiar rayon lesson MEASURED at a band boundary. REGIME: batched variable-length small-forwards on heterogeneous-core CPUs (P+E clusters). ANTI-REGIME: uniform-cost jobs (one pool IS the answer); single-matrix-unit boxes (no straggle axis). Batch 429 / Plan 311 / Research 350.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
