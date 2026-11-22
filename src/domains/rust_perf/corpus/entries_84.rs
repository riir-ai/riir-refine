// Batch 369 — the sybil inference-optimisation-skill distill
// (sybil-solutions/inference-optimisation-skill @
// fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT).
//
// rust_perf 358 → 364: R1 `token-count-from-authoritative-counter-not-stream-chunks`
// (measurement-honesty, the A− LEAD) + R2 `degenerate-output-invalidates-throughput-claim`
// + R3 `gpu-residency-summed-beyond-process-and-cgroup-accounting`
// + R4 `specdec-composite-decomposed-into-per-step-time-and-accepted-length`
// + R5 `unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately`
// + R6 `work-removed-build-is-not-a-valid-lower-bound` — ALL suggestion-only
// NotBenchable (the metric-mismatch class: the win metric is a measurement-
// verdict property no validator hosts — the dial9/B136 annotation, not a
// demotion). + 3 count-neutral FACES on existing hosts (F1 the local-drift
// half + F3 the warmup-discard half both onto `wall-clock-ungated-shared-ci-
// noise-floor` entries_79 — one host, two riders; F2 the input-pinning half
// onto `aa-control-prices-the-fidelity-instrument` entries_08).
//
// Quotes byte-verified at pin fda968e7fc1996aa23c67139c755fe541c9d4be0
// (re-cloned 2026-11-18 at the batch window; HEAD == origin/main == the pin —
// STILL the tip). NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: count from the authoritative counter, never delivery increments ──
pub fn r1_token_count_from_authoritative_counter_not_stream_chunks() -> PerfRule {
    PerfRule {
        rule_id: "token-count-from-authoritative-counter-not-stream-chunks".to_string(),
        name: "A rate/latency measurement that derives its UNIT by counting delivery increments (SSE chunks, batched rows, coalesced writes) or sizes its workload by a word→token folklore multiplier → take the count from the AUTHORITY that owns the unit (the server's usage-stats frame, the real tokenizer); increments under-count exactly when payloads PACK multiple units (specdec emits several tokens per chunk), and multipliers mis-size at the vocabulary's tails".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: tokens "measured" by counting SSE chunk events; the workload
// sized by a word->token folklore multiplier.
#[derive(Default)]
struct StreamBench {
    chunks_seen: usize,
    words: usize,
}

fn on_sse_chunk(b: &mut StreamBench, _payload: &str) {
    b.chunks_seen += 1; // one chunk == one token, assumed
}

fn tokens_generated(b: &StreamBench) -> usize {
    b.chunks_seen // under-counts whenever a chunk packs several tokens
}

fn size_workload(b: &StreamBench) -> usize {
    // "about 1.5 tokens per word" folklore — real prose runs ~3.5
    (b.words as f64 * 1.5) as usize
}"#.to_string(),
        fixed_code: r#"// FIXED: the unit comes from the authority — the stream's final usage
// frame (include_usage) or a re-tokenization; sizing re-tokenizes a
// sample instead of multiplying by folklore.
#[derive(Default)]
struct StreamBench {
    usage_tokens: Option<usize>,
}

fn on_usage_frame(b: &mut StreamBench, prompt_tokens: usize, completion_tokens: usize) {
    // the authoritative counter — specdec's packed tokens INCLUDED
    b.usage_tokens = Some(prompt_tokens + completion_tokens);
}

fn tokens_generated(b: &StreamBench) -> usize {
    // refuse to guess: a run without its usage frame has NO token count
    b.usage_tokens
        .expect("no usage frame — re-run with include_usage; chunk counts are not tokens")
}

fn size_workload(sample: &str, tokenize: &dyn Fn(&str) -> usize, target_words: usize) -> usize {
    let n_words = sample.split_whitespace().count().max(1) as f64;
    let tok_per_word = tokenize(sample) as f64 / n_words; // sampled, not folklore
    (target_words as f64 * tok_per_word) as usize
}"#.to_string(),
        context_notes: Some("Grade A− — the batch's LEAD, NotBenchable-by-domain (the metric-mismatch class — the win metric is a measurement-verdict property no validator hosts). SOURCE (sybil-solutions/inference-optimisation-skill @ fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT — pin re-verified at the batch window 2026-11-18: re-cloned, HEAD == origin/main == the pin; quotes byte-verified at the pin). THE LAW, references/methodology.md:7 verbatim: 'Count tokens with the tokenizer or the server's usage stats, not streamed chunks. With speculative decoding, engines emit several tokens per SSE chunk; counting chunks under-reported MTP decode. Use `stream_options: {include_usage: true}` or `continuous_usage_stats`, or re-tokenize the output.' The harness requested exactly that (scripts/bench.py:67: '\"max_tokens\": max_tokens, \"stream\": True, \"stream_options\": {\"include_usage\": True},'). THE SIZING HALF, methodology.md:8 verbatim: 'Size prompts with the real tokenizer. \"Random word\" documents were ~3.5 tokens per word, not ~1.3–1.9, so nominal \"32K/128K\" cells were really 58K/232K.' Both faces are ONE law — the measurement UNIT comes from the authority that owns it, never a derived approximation; the approximation is wrong exactly in the regime the benchmark exists to probe (payload packing; vocabulary tails). ADJUDICATION (dedup re-run at the landing HEAD, vocabularies chunk|stream|count + usage stats|tokenizer: entries_05/07/08 hits are chunks_exact/chunked-loop ITERATION-SHAPE rules — a different axis; 'usage stats' zero at rule level; the audit-counter family owns bookkeeping, never measurement units) — zero owners, novel. REGIME: any rate/latency benchmark whose unit is delivered by increments the producer may PACK (SSE streams, batched gRPC, inflight batching, coalesced page writes); workload sizing by per-unit rate extrapolation. ANTI-REGIMES: streams whose chunks are provably one-unit (an unprefixed line protocol — the increment count IS the count); billing audits (there the authority IS the increment ledger). NOT the timed-region absent-work family (a deleted loop satisfying a latency bar — a COMPILE-TIME DCE axis): THIS is a mis-derived COUNT at runtime, the work all ran. FIXTURES: b369 F1 (the packed-chunk arm under-counts vs the usage frame; the folklore-multiplier arm mis-sizes vs the sampled tokenizer rate; the one-token-per-chunk anti-regime where both agree; the tampered zero usage-frame refuses rather than falling back to chunks). Plan 384 / Research 417.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R2: output sanity gates every throughput claim ──────────────────────
pub fn r2_degenerate_output_invalidates_throughput_claim() -> PerfRule {
    PerfRule {
        rule_id: "degenerate-output-invalidates-throughput-claim".to_string(),
        name: "A tok/s (or any rate) verdict computed from tokens ÷ time alone → assert OUTPUT SANITY beside it before publishing: repeated word 8-gram share > 0.5 over a > 50-gram floor (looping), failed-request count, and device-busy fraction in the window; a suspiciously fast record is usually broken output, not a win — a run failing any arm publishes NO rate".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: publish tok/s from the counters alone — the output text is
// measured and then ignored, so a model looping on a numbered list
// sets a "record".
struct Run {
    tokens: usize,
    secs: f64,
    failed_requests: u32,
    _text: String, // captured, never inspected
}

fn throughput(r: &Run) -> f64 {
    r.tokens as f64 / r.secs // the 117 tok/s "record" lives here
}"#.to_string(),
        fixed_code: r#"// FIXED: the rate is asserted BESIDE an output-sanity gate — repeated
// 8-gram shingle share over a 50-gram floor, failed requests, device
// busy — and an invalid-sanity run publishes NO rate.
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

struct Run {
    tokens: usize,
    secs: f64,
    text: String,
    failed_requests: u32,
    device_busy_frac: f32,
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Rate(f64),
    InvalidOutput(&'static str),
}

fn shingle_hash(words: &[&str]) -> u64 {
    let mut h = DefaultHasher::new();
    words.hash(&mut h);
    h.finish()
}

fn looping(text: &str, n: usize, thresh: f64) -> bool {
    let w: Vec<&str> = text.split_whitespace().collect();
    if w.len() <= n {
        return false;
    }
    let grams: Vec<u64> = (0..w.len() - n)
        .map(|i| shingle_hash(&w[i..i + n]))
        .collect();
    let total = grams.len();
    let unique: HashSet<u64> = grams.iter().copied().collect();
    total > 50 && 1.0 - (unique.len() as f64 / total as f64) > thresh
}

fn verdict(r: &Run) -> Verdict {
    if r.failed_requests > 0 {
        return Verdict::InvalidOutput("failed requests in window");
    }
    if r.device_busy_frac < 0.60 {
        return Verdict::InvalidOutput("device idle in window");
    }
    if looping(&r.text, 8, 0.5) {
        return Verdict::InvalidOutput("looping output (repeated 8-gram shingles)");
    }
    Verdict::Rate(r.tokens as f64 / r.secs)
}"#.to_string(),
        context_notes: Some("Grade B, NotBenchable-by-domain (the metric-mismatch class — the win metric is a measurement-verdict property no validator hosts). SOURCE (sybil-solutions/inference-optimisation-skill @ fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT, pin re-verified at the batch window; quotes byte-verified at the pin). THE LAW, references/methodology.md:42 verbatim: 'Flag looping output (repeated shingles), failed requests, an idle GPU (<60% busy in the window), prefill inside the decode window, and KV-bound admission. A 117 tok/s \"record\" was a model emitting a numbered list in a loop.' The bounded detector is the harness's own (scripts/bench.py:100-105, verbatim shape): share of repeated word 8-grams, flagged past 0.5 ONLY over a >50-gram floor — the floor is load-bearing (short outputs would flag on chance repetition). The idle-device arm (<60% busy) and the failed-request arm complete the assertion set; a run failing ANY publishes no rate. DISTINCT FROM the timed-region loud-zero family (katgpt-rs gate: rustc+fat-LTO deletes a timed loop so the bar is met by ABSENT work — a compile-time axis): THIS is the runtime output-quality axis — the work ran, the number is real, and the OUTPUT is broken. ADJUDICATION (dedup at landing HEAD, vocabularies degenerate|looping|shingle|sanity over all corpora — zero rules; entries_72's 'looping' hit is a worker-pool NAME, unrelated). REGIME: any throughput/latency claim over generated content (LLM serving, compression corpora, procedural generation); perf-regression suites that gate wins on generated output. ANTI-REGIMES: deterministic-content benchmarks where the text IS pinned (repetition is the input, not a defect); rates over non-text work. FIXTURES: b369 F2 (looping arm flagged, healthy arm clean, the <50-gram floor holds on short near-repetitive text — no overfire, the inverted-predicate tamper arm reds). Plan 384 / Research 417.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R3: unified-memory footprint = cgroup anon + deduped driver residency ──
pub fn r3_gpu_residency_summed_beyond_process_and_cgroup_accounting() -> PerfRule {
    PerfRule {
        rule_id: "gpu-residency-summed-beyond-process-and-cgroup-accounting".to_string(),
        name: "A memory budget on a unified-memory host read from process/cgroup counters (MemAvailable delta, cgroup.current) alone → the true footprint is cgroup anon PLUS driver residency: per-fd `drm-resident-*` rows DEDUPLICATED by `drm-client-id` (one shared buffer behind several fds counts ONCE — the dedup is load-bearing); GPU allocations bypass both counters and the naive read can be ~2× under (14 GiB read vs 29.4 GiB real)".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the budget reads process/cgroup counters only — on a
// unified-memory host the GPU's residency is invisible to both, so
// the budget over-commits.
fn mem_budget_bytes(mem_available: u64, cgroup_anon: u64, headroom: u64) -> u64 {
    // device buffers never enter either counter
    let in_use = cgroup_anon;
    mem_available
        .saturating_sub(headroom)
        .saturating_sub(in_use)
}"#.to_string(),
        fixed_code: r#"// FIXED: driver residency summed per fd, DEDUPLICATED by client id
// (one shared buffer behind several handles counts once), then added
// to cgroup anon.
use std::collections::HashSet;

struct FdResidency {
    client_id: Option<u64>, // None = not device memory
    resident_bytes: u64,
}

fn real_footprint_bytes(cgroup_anon: u64, fd_rows: &[FdResidency]) -> u64 {
    let mut seen = HashSet::new();
    let mut gpu = 0u64;
    for row in fd_rows {
        let Some(key) = row.client_id else { continue };
        if !seen.insert(key) {
            continue; // the dedup IS the law: shared buffers count once
        }
        gpu += row.resident_bytes;
    }
    cgroup_anon + gpu
}"#.to_string(),
        context_notes: Some("Grade B−, NotBenchable-by-domain (the metric-mismatch class — the win metric is a measurement-verdict property no validator hosts). SOURCE (sybil-solutions/inference-optimisation-skill @ fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT; quotes byte-verified at the pin). THE LAW, references/methodology.md:46-47 verbatim: 'On unified memory (Apple, Intel/AMD iGPU, GB10/Spark-class), GPU allocations bypass cgroup accounting. A MemAvailable delta showed 14 GiB where the real footprint was 29.4 GiB.' + 'Sum driver residency (Linux DRM: `drm-resident-*` in `/proc/<pid>/fdinfo`, deduplicated by `drm-client-id`) plus cgroup `anon`.' The harness operationalizes it (scripts/mem.sh:5-6 verbatim: 'Unified memory (Intel/AMD iGPU, Linux DRM drivers): sums drm-resident-* (deduplicated by drm-client-id)' / 'over the container's processes + cgroup anon. Reading other users' fdinfo needs root: run the watchdog'). THE DEDUP IS LOAD-BEARING: one buffer shared by several fds (render + import + export handles) must count ONCE — raw row-summing inflates by the aliasing factor. ADJUDICATION (dedup at landing HEAD, vocabularies footprint|drm.resident|cgroup|MemAvailable|residency — zero rules on the residency-summing axis; the MemAvailable name-hits are cache-policy rules (entries_37's host cache budget, entries_21's display carve), never footprint-measurement laws; entries_70 is the named kin, adjudicated in-rule below). ADJACENT-DISTINCT from `macos-pressure-reads-footprint-not-rss` (entries_70, mandatory in-rule both directions): that rule picks the right PROCESS counter (phys_footprint over RSS) on macOS; THIS says the true footprint is NOT ALL IN the process — device residency must be summed from DRIVER interfaces on unified memory (compose: on an M-series host the honest memory row is footprint + the wired-limit note). REGIME: unified-memory serving hosts (Metal, iGPU, GB10-class), containerized GPU workloads, any memory-budget/OOM guard priced from MemAvailable or cgroup.current. ANTI-REGIMES: discrete GPUs with per-process VRAM accounting (nvidia-smi per-pid is the authority there); CPU-only workloads. LANE NOTE (the recorded OFFER, never auto-applied): the league boxes ARE unified-memory Metal hosts whose box-state disclosures quote free RAM only — a driver-residency probe rides the perf-rematch PROVENANCE line. FIXTURES: b369 F3 (a model device whose fds share one client id — counted once; two clients — counted twice; the no-device anti-regime where counters suffice; the tampered duplicate-client row the dedup catches). Plan 384 / Research 417. FACE (Batch 398, Plan 398 / Research 432, count-neutral — the ADMISSION half): ddalcu/mlx-serve's memory ceiling @ 96311b26 (MIT, src/host/registry/memory_ceiling.zig:60-95) carries this rule's honest accounting to the ADMISSION decision — the ceiling is min(working_set_limit, mlx_footprint + free_system), adopted after the #64 incident: a 90K-token MoE prefill admitted against a static 115 GB max, and Metal threw 'Insufficient Memory' FROM THE COMMAND-BUFFER COMPLETION HANDLER — an UNCATCHABLE async throw (the error arrives outside any unwinding context; admission-time honesty is the only defense); TWO refinements land here: WIRED_LIMIT_MARGIN_BYTES = 8 GiB ('past the limit Metal RETURNS ZEROS before an uncatchable abort' — the failure mode is silent zeros, worse than the throw) and wiredLimitFloor (an operator's iogpu.wired_limit_mb declaration lifts the free-RAM term, never past total-minus-margin) — the footprint accounting this rule owns, extended with the admission gate + the operator-declared floor; compose (this rule sums the footprint honestly, the face's gate refuses the admission honestly). FACE (Batch 398, Plan 398 / Research 432, count-neutral — the ADMISSION half): ddalcu/mlx-serve's memory ceiling @ 96311b26 (MIT, src/host/registry/memory_ceiling.zig:60-95) carries this rule's honest accounting to the ADMISSION decision — the ceiling is min(working_set_limit, mlx_footprint + free_system), adopted after the #64 incident: a 90K-token MoE prefill admitted against a static 115 GB max, and Metal threw 'Insufficient Memory' FROM THE COMMAND-BUFFER COMPLETION HANDLER — an UNCATCHABLE async throw (the error arrives outside any unwinding context; admission-time honesty is the only defense); TWO refinements land here: WIRED_LIMIT_MARGIN_BYTES = 8 GiB ('past the limit Metal RETURNS ZEROS before an uncatchable abort' — the failure mode is silent zeros, worse than the throw) and wiredLimitFloor (an operator's iogpu.wired_limit_mb declaration lifts the free-RAM term, never past total-minus-margin) — the footprint accounting this rule owns, extended with the admission gate + the operator-declared floor; compose (this rule sums the footprint honestly, the face's gate refuses the admission honestly).".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R4: decompose the specdec composite into its factors ────────────────
pub fn r4_specdec_composite_decomposed_into_per_step_time_and_accepted_length() -> PerfRule {
    PerfRule {
        rule_id: "specdec-composite-decomposed-into-per-step-time-and-accepted-length".to_string(),
        name: "A speculative-decoding verdict read off the composite tok/s alone (or off acceptance length alone) → decompose and report BOTH factors — per-step time (ms per verify step) and accepted length — with the composite DERIVED, never the judge: tok/s = accepted ÷ step_time, and higher acceptance can run 11–14% slower (3.26 vs 2.85 accepted)".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the headline is the composite — or worse, acceptance alone.
#[derive(Default)]
struct SpecReport {
    accepted_mean: f64,
    step_ms: f64,
}

fn better(a: &SpecReport, b: &SpecReport) -> bool {
    // "more accepted tokens per draft == faster" — the inversion hides here
    a.accepted_mean > b.accepted_mean
}"#.to_string(),
        fixed_code: r#"// FIXED: the factors are reported separately; the composite is
// DERIVED (tok/s = accepted / step_time), never the judge alone.
#[derive(Debug, Default)]
struct SpecReport {
    accepted_mean: f64,
    step_ms: f64,
}

impl SpecReport {
    fn tokens_per_sec(&self) -> f64 {
        self.accepted_mean / (self.step_ms / 1000.0)
    }
}

fn better(a: &SpecReport, b: &SpecReport) -> bool {
    a.tokens_per_sec() > b.tokens_per_sec()
}

fn describe(r: &SpecReport) -> String {
    format!(
        "accepted {:.2} @ {:.2} ms/step = {:.1} tok/s — factors reported, composite derived",
        r.accepted_mean, r.step_ms, r.tokens_per_sec()
    )
}"#.to_string(),
        context_notes: Some("Grade B−, NotBenchable-by-domain (the metric-mismatch class — the win metric is a measurement-verdict property no validator hosts). SOURCE (sybil-solutions/inference-optimisation-skill @ fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT; quotes byte-verified at the pin). THE LAW, references/methodology.md:29-30 verbatim: 'Judge per-step time (ms per verify step) and accepted length separately. tok/s = a ÷ step_time.' + 'Higher acceptance can be slower: a 7-token draft accepted 3.26 vs MTP's 2.85, yet ran 11–14% slower.' The composite-product decomposition law: optimize the product, diagnose the FACTORS, report both — the composite alone cannot rank two engines whose factors trade off, and acceptance ALONE inverts. The counter face carried: acceptance counters update on request completion in some engines — a short window sees a biased sample; probe whole requests. ADJUDICATION (mandatory, both directions): `alloc-proxy-inversion-work-ratio-caveat` (entries_79) owns the PROXY-inversion axis (a proxy that anti-correlates in another regime — an alloc delta read alone); adjacent in spirit, DISTINCT fix-space — THIS decomposes a rate PRODUCT into its factors and pins the derivation direction (compose: the proxy rule prices a delta read alone, this splits the delta's parents). The B177 `aa-control-prices-the-fidelity-instrument` (entries_08) owns fidelity gating of the INSTRUMENT; THIS owns the factor split of the REPORTED quantity (compose: the A/A gate proves the instrument, the factor split proves the verdict). REGIME: speculative decoding / multi-token prediction / any draft-verify serving lane; any composite rate = a ÷ b where a and b move independently across systems. ANTI-REGIMES: single-factor sweeps (same engine, draft length swept — the composite IS the target). LANE NOTE: the specdec beat lane (riir-infer 1179/1183 + riir-train 492) reports acceptance and per-step numbers — this rule is its report-shape law. FIXTURES: b369 F4 (high-acceptance/slow-step ranks BELOW low-acceptance/fast-step on the composite — the inversion the acceptance-only form hides; the equal-step anti-regime where the proxy holds; the impossible-state tamper — accepted > draft or step ≤ 0 — refuses). Plan 384 / Research 417.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R5: unique prompts defeat the prefix cache; cold/warm never pool ────
pub fn r5_unique_prompts_defeat_prefix_cache_and_cold_warm_reported_separately() -> PerfRule {
    PerfRule {
        rule_id: "unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately".to_string(),
        name: "A prefill number measured over reusable prompts and published as ONE undivided prefill → build prompts UNIQUE per run (a seeded random head) so the prefix cache cannot hit unless hits are the DELIBERATE measurement, and report cold and warm (cache-hit) separately — never publish a cache-hit prefill as prefill".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: prompts reuse a fixed head across runs; the prefill number
// is published without a cold/warm split.
const HEAD: &str = "[doc shared-prefix] ";

fn build_prompt(doc: &str) -> String {
    format!("{HEAD}{doc}") // the prefix cache hits on run 2+
}

fn prefill_tok_s(prompt_tokens: usize, ttft_s: f64) -> f64 {
    prompt_tokens as f64 / ttft_s // could be a cache-hit number; nobody knows
}"#.to_string(),
        fixed_code: r#"// FIXED: the head is unique per run (seed + monotonic nonce), and the
// phase is DECLARED — cold misses the cache, warm deliberately hits it
// and says so.
use std::sync::atomic::{AtomicU64, Ordering};

static RUN: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, PartialEq, Clone, Copy)]
enum Phase {
    Cold, // unique head: the cache cannot hit
    Warm, // DELIBERATE cache-hit measurement, reported as warm
}

fn build_prompt(doc: &str, seed: u64, phase: Phase) -> String {
    match phase {
        Phase::Cold => {
            let run = RUN.fetch_add(1, Ordering::Relaxed);
            format!("[doc {seed}-{run:x}] {doc}") // unique per run: no cache hit
        }
        Phase::Warm => format!("[doc warm-head] {doc}"), // the SAME head again: hits
    }
}

fn prefill_report(prompt_tokens: usize, ttft_s: f64, phase: Phase) -> String {
    let rate = prompt_tokens as f64 / ttft_s;
    match phase {
        Phase::Cold => format!("prefill (cold) {rate:.1} tok/s"),
        Phase::Warm => format!("prefill (warm, cache-hit) {rate:.1} tok/s — NOT prefill"),
    }
}"#.to_string(),
        context_notes: Some("Grade C+/B−, NotBenchable-by-domain (the metric-mismatch class — the win metric is a measurement-verdict property no validator hosts). SOURCE (sybil-solutions/inference-optimisation-skill @ fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT; quotes byte-verified at the pin). THE LAW, references/methodology.md:9 + :11 verbatim: 'Build prompts before the clock starts. Make them unique per run, with a random prefix, so prefix caching cannot hit, unless you are measuring cache hits deliberately.' + 'Prefill tok/s: prompt tokens ÷ TTFT on a cold, unique prompt. Report cold and warm (cache-hit) separately; never publish a cache-hit prefill as prefill.' The harness operationalizes BOTH halves (scripts/bench.py:117 verbatim: 'head = f\"[doc {seed}-{time.time_ns()}] \"  # defeats prefix caching'). The DECODE-side carve-out is real and carries: the fixed 6-prompt panel at bench.py:34, comment verbatim 'fixed panel: acceptance and speed depend on content, so never randomise these' — decode acceptance may ride a fixed panel because content conditions acceptance (the input-pinning face lands on `aa-control-prices-the-fidelity-instrument` this batch); the PREFILL rate must not reuse the panel's prefix. ADJUDICATION (dedup at landing HEAD, vocabularies prefix.cache|cache.hit|cold.warm|prompt.head over all corpora: the hits are prefix-SUM arithmetic, cache POLICY (keep-warm/reserve-stream-caches), and seam-warming rules — zero owners on the measurement-contamination axis; the entries_08 A/A-rule face this batch is the decode-side complement, compose never merge). ADJACENT-DISTINCT from R1 (this batch): same measurement genus, different fix-SITE — R1 owns the counting UNIT; THIS owns prompt CONSTRUCTION + phase reporting (compose in the same harness). REGIME: prefill/TTFT benchmarks; any cache-keyed serving path (prefix caches, KV reuse, memoized IPC handshakes); cold/warm splits. ANTI-REGIMES: the deliberate cache-hit measurement (then the phase IS the headline — 'warm (cache-hit)' — the rule's own carve-out); single-shot loads with no reuse. FIXTURES: b369 F5 (the constant-head arm's second run hits the model cache — its 'prefill' is a hit and the fixed form SAYS so; the unique-head arm misses cold; the deliberate-warm regime reports its phase; the tampered head-collision is detected). Plan 384 / Research 417.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R6: an arm that removed work is no lower bound ──────────────────────
pub fn r6_work_removed_build_is_not_a_valid_lower_bound() -> PerfRule {
    PerfRule {
        rule_id: "work-removed-build-is-not-a-valid-lower-bound".to_string(),
        name: "A perf bound claimed from an arm that does LESS work (a debug build skipping dequantization, a graph-off profile timed as the step) — an arm that removes work changes CODE GENERATION and measures a DIFFERENT program, so it bounds nothing → bounds hold only between arms whose EXECUTED-WORK CENSUS is equal, and a census mismatch refuses the claim instead of reporting a speedup".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the lower bound comes from an arm that skips work.
fn dequantize(w: &[f32], debug_skip: bool) -> Vec<f32> {
    if debug_skip {
        return Vec::new(); // "just for the profile run" — the arm does LESS
    }
    w.iter().map(|x| x * 2.0).collect()
}

fn lower_bound_ms(_full_ms: f64, reduced_arm_ms: f64) -> f64 {
    reduced_arm_ms // claimed as a floor on the full path — the codegen differs
}"#.to_string(),
        fixed_code: r#"// FIXED: the bound is admitted only between arms with an EQUAL
// executed-work census — the census counts logical ops the run
// PERFORMED, so a build that skips work fails the gate loudly.
#[derive(Default, PartialEq, Debug)]
struct WorkCensus {
    dequant_rows: u64,
    gemm_ticks: u64,
}

fn dequantize(w: &[f32], census: &mut WorkCensus) -> Vec<f32> {
    census.dequant_rows += w.len() as u64;
    w.iter().map(|x| x * 2.0).collect()
}

fn valid_lower_bound(a: &WorkCensus, b: &WorkCensus) -> Result<(), &'static str> {
    if a != b {
        // different work == different program: no bound is claimed
        return Err("work census differs — the arm removed work; the bound is INVALID");
    }
    Ok(())
}"#.to_string(),
        context_notes: Some("Grade C+, NotBenchable-by-domain (the metric-mismatch class — the win metric is a measurement-verdict property no validator hosts). SOURCE (sybil-solutions/inference-optimisation-skill @ fda968e7fc1996aa23c67139c755fe541c9d4be0, MIT; quotes byte-verified at the pin). THE LAW, references/methodology.md:54 verbatim: 'Debug builds that remove work (e.g. skipping dequantization) change code generation, so they are not valid lower bounds.' The paired clause (:52): 'Profile with graphs off to get a per-kernel breakdown, but remember that graphs change launch overhead. Time the graph replay itself for the real step time.' One category error, two instances: an A/B arm that does LESS work measures a DIFFERENT program — its time bounds nothing about the arm that does the work. The remedy is a WORK CENSUS beside the timing: the executed logical ops (rows dequantized, tiles stepped, frames decoded) must be EQUAL before a bound is admitted, and a mismatch refuses the claim instead of reporting a speedup. ADJUDICATION (mandatory, both directions): `instruction-count-cache-blindness` (entries_79) owns the icount METRIC's locality blindness (equal instructions, divergent access — a metric critique); THIS owns arm VALIDITY (the census refuses unequal-work arms BEFORE any metric is read — an upstream gate to that rule, never merged); `wall-clock-ungated-shared-ci-noise-floor` (entries_79 — this batch's F1/F3 face host) owns which metric JUDGES in shared CI; THIS owns whether the PAIR is adjudicable at all. The graph clause is the same law at the profiling layer: a graph-off profile is a valid BREAKDOWN, never a valid step-time bound — time the replay. REGIME: profile-guided fixing (the hotpath-json lane), quantized-serving comparisons, JIT/AOT pairs, any 'lower bound' headline. ANTI-REGIMES: arms equal-by-construction (an ab-timing pair over the same function — the census is trivially equal); breakdowns explicitly labeled as not-bounds. FIXTURES: b369 F6 (the work-skipping arm's bound refused; equal-census arms admitted; the source-line census tamper — ops hidden behind an untaken branch — caught by executed-op counting). Plan 384 / Research 417. NOTE (Batch 398, Plan 398 / Research 432, count-neutral — the sparse-video-attention corroboration): ddalcu/mlx-serve's cell @ 96311b26 (MIT) measured a sparse video attention at µbench 15x -> live 1.33x (smeared tiles), with the source's own diagnosis 'SDPA and linears both run within ~12% of the dense-bf16 roofline, so wall clock moves only by doing fewer forwards' — the work-removed axis of this rule from a third engine: the removed-work lower bound must be the LIVE path's removed work, never the isolated kernel's.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b369_rules() -> Vec<PerfRule> {
    vec![
        r1_token_count_from_authoritative_counter_not_stream_chunks(),
        r2_degenerate_output_invalidates_throughput_claim(),
        r3_gpu_residency_summed_beyond_process_and_cgroup_accounting(),
        r4_specdec_composite_decomposed_into_per_step_time_and_accepted_length(),
        r5_unique_prompts_defeat_prefix_cache_and_cold_warm_reported_separately(),
        r6_work_removed_build_is_not_a_valid_lower_bound(),
    ]
}
