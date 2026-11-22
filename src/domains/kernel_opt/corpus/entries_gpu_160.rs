//! Batch 380 — the maya-inkling-mini moved-surface re-distill (Plan 397 /
//! Research 429 §Delta): ONE decision-content law from the 8-GB tier's
//! heterogeneous expert serving — `expert-miss-computes-on-cpu-overlap-gpu-cache`
//! — the per-miss lane dispatcher that prices the CPU pool lane against the
//! PCIe copy lane by WHOEVER IS FREE FIRST, with both prices measured AS THEY
//! RUN (online EMAs), the computed expert's blocks following on the copy
//! stream later ("lazy"), still-arriving experts diverted to the CPU instead
//! of the GPU waiting, and the copy admission bounded per call and in flight
//! so a narrow link is never queued up. All suggestion-only
//! NotBenchable-by-domain (the win is a heterogeneous CPU+GPU overlap
//! latency on a PCIe-hosted expert tier — no CPU validator hosts it; the
//! entries_gpu_65 / dial9 / Batch-136 class), `bench_spec: None`, measured
//! source provenance carried first-class and FOREIGN (40–43 tok/s on their
//! 8-GB RTX 5070 Laptop vs llama.cpp 12.5; the miss lane at 37–41 GB/s of
//! the RAM's 58 — regime data, never our magnitude, never a bar).
//!
//! Faces landed COUNT-NEUTRAL on their owners: F1 on
//! `speculation-scheduler-break-even-flip` (gpu_29) — THE CONTRAST ARM,
//! never a positive instance: gpu_29's bypass is monotone within a request;
//! the source's SpecCtl re-probes after 8 plain tokens because its
//! acceptance evidence is decayed and acceptance is a prompt-family
//! property; F2 on `fp32-input-hi-lo-two-fp16-mma-columns` (B379-R2) — the
//! DP4A INT8-limb sibling arm at the same file (`expert_limb_mv`,
//! ik2_kernels.cuh:173-177 at the tip, quote re-verified) + the exactness
//! laws ("a code times its scale is exact in FP16", INKLING2-MINI.md:165)
//! folded in-rule there; F3 is NOT a separate landing — the FP8-KV drift
//! disclosure (INKLING2.md:268-269, "any two FP8 runs part within a few
//! tokens: a difference of 1e-5 flips codes of a 3-bit mantissa") rides
//! THIS rule's measurement-honesty note in-rule (the 8-GB tier's cache IS
//! the FP8-KV carrier; the Issue-940 deterministic-fold carve-out + the
//! riir-infer drift-guard posture corroborated, no rust_perf surface
//! touched). T0: re-clone at tip `e819852f822479af783638a467bfdfe2ed6882ec`
//! (MIT unchanged, weights Apache-2.0 never mined) — upstream rewrote
//! history past B379's pin `be849daa` (the rev-list count no longer
//! resolves in the unshallowed clone; recorded verbatim), every mined
//! quote site byte-verified at the tip. Fixture:
//! `corpus/fixtures/b380_fixtures.rs` — the original four-arm model (hit /
//! miss-computed-overlapped / miss-computed-serialized negative / tampered
//! pool refuses loud) + the S3-FIFO/ghost cache shape, standalone-compiled
//! AND executed before entry, debug AND -O. The UNREAD residue closed this
//! batch: the 8-GB cache body (ik2_main.cu:825-1150 — per-size-class
//! pools, the O(1) S3-FIFO lists, the ghost window) and the miss-pool
//! thread lifecycle (:305-330, :3485-3520, :3655-3670). The G1 census
//! repair rides this landing: entries_gpu_159 joined
//! expected_corpus_files()/per_file_entries() here (the B354/B357 class,
//! B362 precedent — B379 landed the file without arming the census; both
//! lists moved in the same commit so the comparison stays consistent).
//! Batch 380 / Plan 397 / Research 429 §Delta.

use crate::domains::kernel_opt::corpus::KernelRule;

// The corpus entry IS the payload (data, not logic) — the kernel build fn
// convention (the entries_popcorn precedent).
#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R-A — the batch lead (the Delta's B+): a VRAM-tight MoE tier
        // resolves each router miss on the CHEAPER OF TWO HOST LANES — the
        // PCIe copy lane or the CPU compute pool — by free-first comparison
        // of the lanes' cumulative busy-until times, both prices re-measured
        // online; the computed expert's blocks ride the copy stream AFTER
        // the compute (lazy) so the next visit hits; the GPU never waits
        // on its own copy stream mid-token. ──────────────────────────────
        KernelRule {
            rule_id: "expert-miss-computes-on-cpu-overlap-gpu-cache".to_string(),
            gpu_concept: "MECHANISM (a VRAM-tight MoE expert tier resolves every router miss on ONE OF TWO HOST LANES priced by free-first comparison — the PCIe copy lane or the CPU compute pool — with the CPU's result overlapped against the GPU's cached execution; category H — MoE dispatch, the expert-cache-miss family's home; the plan's draft said category G, a slip, corrected at authoring): when the experts do not all fit in VRAM, the source keeps them in PINNED HOST MEMORY and gives the GPU an expert cache, then dispatches each miss to the lane that frees first. The tier statement, verbatim (docs/INKLING2-MINI.md:146 at e819852f): 'The experts stay in pinned host memory (14.7 GiB of RAM), the GPU keeps a 3.8 GiB cache of them, and the CPU computes each missing expert from host memory while the GPU runs the cached ones. Measured on a laptop with an RTX 5070 Laptop GPU (8 GB, PCIe 4.0 x8), a Ryzen 9 270 (AVX-512) and DDR5 that reads at 58 GB/s: 40 to 43 tok/s through the server on five chat prompts of 500 to 600 tokens, at a 16,384 token context. llama.cpp with its experts on the CPU gets 12.5 tok/s on the same laptop.' THE DISPATCH IS THE UNOWNED AXIS (the verdict's B+ finding) — not the FIFO policy, not the cache structures: the per-miss lane choice is a FREE-FIRST comparison over the two host lanes' cumulative busy-until accumulators, verbatim (src/ik2/ik2_main.cu:3498-3499): '// a miss: the CPU's or the link's, whichever is free first' + 'if (cpu_lane && acc_cpu + t_cpu <= acc_up + t_up) {' — acc_cpu/acc_up are the lanes' running busy-until times within the token's miss batch, t_cpu/t_up the per-expert prices, and the prices are NOT config constants: they are measured AS THE ENGINE RUNS, verbatim (:312 + :315): 'double g_t_cpu_f[2] = {0.5e-3, 1.0e-3};   // seconds an expert on the pool, NVFP4 and Q8_0 (measured as it runs)' + 'double g_t_up = 0.36e-3;   // seconds an expert: the pool's (measured as it runs), the link's' — and re-measured every CPU batch by an online EMA (:3664, after the pool's wait): 't_cpu = 0.9 * t_cpu + 0.1 * (now() - tc0) / n_jobs;' — a 0.9/0.1 EMA over the measured per-job seconds, so a box whose CPU or link speed differs from the prior re-prices itself within a few batches and the lane choice re-decides. THE OVERLAP IS THE WHOLE WIN, verbatim (:309-310): 'The CPU lane (IK2_CPU=1: on): a missing NVFP4 expert computed in host memory by the pool while the others' blocks are copied; its own blocks follow on the copy stream without being waited for (\"lazy\"), so it is in VRAM next time.' — the token's other experts proceed on the GPU (the ~15 ms of GPU kernels a token takes, INKLING2-MINI.md:157) WHILE the pool computes the miss; the miss's expert OUTPUT rows come back to the device for THIS token, and the expert's own BLOCKS follow on the copy stream for the NEXT visit — the miss is paid once, the cache wins the rest (the measured hit-rate delta of exactly this machinery: 'The cache still follows the text: 84% hits against 49% with no copies', INKLING2-MINI.md:155). THREE DECISION CONTENTS ride the dispatcher: (a) the STILL-ARRIVING diversion — a slot whose blocks are still in flight from a lazy copy does not stall the GPU on its own stream; with the CPU lane up it is computed from host memory instead, verbatim (:3492-3493): '// (IK2_LAZY_CPU=1: the CPU computes it from host memory instead of the GPU waiting)' — the slot's arrival is visible as slot_sseq > g_seq_done (the lazy ring's not-yet-done sequence, :3488); (b) the COPY ADMISSION IS BOUNDED — a computed miss takes a cache slot and a lazy copy only when the in-flight bound allows, verbatim (:3501-3502): '// (IK2_LAZY_MAX / IK2_LAZY_PER: at most that many lazy batches in flight and copies a call, so' + '//  a narrow link is not queued up with them; past either the expert is only computed)' — past the bounds the expert is compute-only; (c) A COMPUTE-ONLY MISS TAKES NO CACHE SLOT, verbatim (:3509): '} else {            // no slot taken: nothing in VRAM is evicted for it' — the dispatcher never evicts a cached expert to make room for bytes it is not going to wait for. THE BOUNDARY KNOB is the honest two-regime witness: IK2_UP_MS prices the copy lane so high the CPU computes EVERY miss, verbatim (INKLING2-MINI.md:152): 'A copy over PCIe priced so high that the CPU computes every miss. A narrow link stalls the GPU on each copy' — the free-first comparison degenerates gracefully to a pure-compute tier on a narrow link; the 24-GB card is the other pole, verbatim (INKLING2-MINI.md:163): 'On a 24 GB card that leaves room for the KV cache, so a token needs nothing from the host between its embedding and its logits' — the dispatcher is DEAD CODE there (the anti-regime: no misses, no lanes). THE CACHE SHAPE the dispatcher serves (carried, not the lead): 'the expert cache in VRAM: per-size-class pools, O(1) S3-FIFO with ghost history' (ik2_main.cu:825) — per-size-class pools so a small expert never evicts a big one's slot class; the S3-FIFO lists (prob/main/free intrusive lists, :839-860) with a GHOST HISTORY window: on eviction the victim's id is stamped 'ghost_at[old_idx] = P.ghost_seq' (:1132), a re-admission inside one cache generation is recognized 'is_ghost' (:1135) and re-enters the PROBATION segment at freq 1, not the main segment — 'P.freq[best] = is_ghost ? 1 : 0' (:1139) — a second chance for recently-evicted experts without scanning. THE MEASUREMENT-HONESTY NOTE (F3, folded in-rule — the 8-GB tier's cache IS the FP8-KV carrier): the source's own docs disclose that the FP8 cache's 2x byte win is a DRIFT surface, verbatim (docs/INKLING2.md:268-269): 'An FP8 cache is as far from the FP16 logits whichever kernels read it (KL 1.9e-3 on the tensor cores, 2.0e-3 scalar, over 199 forced tokens; FP16 on the tensor cores 2.2e-5) - but any two FP8 runs part within a few tokens: a difference of 1e-5 flips codes of a 3-bit mantissa.' — aggregate fidelity (KL vs FP16) and RUN-TO-RUN determinism are different claims, and only the first survives FP8; the drift surface corroborates the katgpt-rs Issue-940 deterministic-fold law (sync-crossing reductions pin their fold order; locally-consumed latent stays latent) and the riir-infer drift-guard posture (Issue 1182: exact-path fallback when serving-posture divergence crosses its sigmoid band). A rule carrying THIS cache into a quorum/replay context must re-classify the surface before quoting the 2x. CLASS SIGNATURE: a MoE (or any many-tenant weight) serving tier where the working set exceeds VRAM, the host has spare CPU + a pinned-host copy of the weights, and the per-miss cost splits between a copy lane and a compute lane. REGIME: VRAM-tight decode (the 8-GB tier: 14.7 GiB experts + 3.8 GiB cache + a 58-GB/s DDR5 pinned pool), chat-shaped traffic where the cache follows the text (84% hits), AVX-512-class host compute; the measured tier summary (provenance, their laptop, never a bar): 40-43 tok/s vs llama.cpp's experts-on-CPU 12.5, the miss lane at '37 to 41 GB/s of the 58 GB/s the RAM gives' (INKLING2-MINI.md:157), context scaling 32,768 tokens ~36 tok/s / 65,536 ~32 (less VRAM for the cache — the tier's own sensitivity curve). ANTI-REGIME: all-experts-resident cards (no misses — the dispatcher is dead code); prefill (the cache path streams every missed expert once — 'each pass streams every expert it misses once, the next layer's prefetched while this one' runs, ik2_main.cu:10 — the CPU lane is decode's, :13 'a missing expert is computed on the CPU (AVX-512) when that is sooner'); a fast link + slow host (the free-first comparison picks the copy lane every time — the honest answer, the IK2_UP_MS knob exists only to override it when the link is the wall); quorum/replay consumers of the FP8 cache (the F3 drift note — re-classify first). ⛔ KINSHIP (adjudicated both directions): `ghost-table-*` (entries_39, rust_perf) owns the GHOST-TABLE DATA STRUCTURES of admission prediction (the keyed ghost-entry containers) — THIS owns the per-miss heterogeneous DISPATCH between copy and compute lanes (the ghost HISTORY here is the S3-FIFO cache's own second-chance window, a cache-policy detail, never a prediction structure; compose when both exist); `whole-matrix-unpack-pass-rows-per-tile-gate` (entries_gpu_148) owns the unpack-vs-tile CROSSOVER for dequantization work placement — a different axis entirely (this tier dequantizes on the CPU pool as its compute lane, but the gate there prices WHERE a dequant pass runs, not WHICH lane serves a miss); `speculative-expert-prefetch-prev-step-routing` (gpu_79) is the same goal on a different lane — hide the next miss by prefetching last step's hot experts into L2 during attention; THIS computes or copies THIS step's actual miss on the host lanes (compose: the prefetch cuts the miss RATE, the dispatcher prices the misses that remain); `compute-experts-before-router-lookup-fixup` (B337) + `readiness-partitioned-expert-resident-missing-submit` (B357) own the SUBMIT-TIME snapshot/readiness split (known vs pending at submit) — THIS owns the EXECUTION-time lane choice per resolved miss (compose: what B357 holds back as pending, THIS prices when it lands); `moe-readiness-queue-no-grid-barrier-producer-scans-before-retire` (H) is the within-kernel readiness queue — a different tier of the same latency fight (kernel scheduling vs host-lane choice); `moe-per-expert-mixed-dispatch-worst-case-workspace` (H) owns the workspace sizing under mixed dispatch — the lane dispatcher must respect its envelope but the laws are disjoint. GREP EVIDENCE (re-run at the landing HEAD): 'S3.FIFO|miss.*compute|pinned.*expert|host memory.*expert' over the corpus -> ZERO owners of the overlap axis (the S3-FIFO spelling hits only this rule + the cache-shape prose); 'SpecCtl|re.probe' -> gpu_29 (the F1 face's owner, adjudicated there); 'DP4A' -> B379-R2's family + lop3 (the F2 face's owner, adjudicated there); 'pinned host' -> zero prior owners. NotBenchable-here (the entries_gpu_65 / dial9 / Batch-136 class — the law's home is a heterogeneous CPU+GPU host tier over PCIe; a Metal/CPU-shaped sibling would fork the law, and no CPU validator prices a GPU-overlap latency). The measurement instrument is the CPU fixture: the free-first dispatcher model (lane accumulators + online-EMA prices; the lane flips when the prices flip), the overlap-vs-serial wall divergence (the law's arm: overlapped wall = max(gpu, cpu) < the serialized sum), the lazy bounds (in-flight + per-call; past them compute-only), the no-slot rule (a compute-only miss evicts nothing), the still-arriving diversion, the tampered-pool refusal (a pool that posts but never completes refuses LOUD, never a silent stall), and the S3-FIFO/ghost cache shape (ghost re-admission into probation at freq 1; per-size-class isolation). Source: alesha-pro/maya-inkling-mini @ e819852f822479af783638a467bfdfe2ed6882ec (tip 2026-11-22, MIT; the delta pin be849daa no longer resolves — upstream rewrote history past it, recorded verbatim, the mined surface tip-verified) — docs/INKLING2-MINI.md:146 (the tier statement + the measured pair) + :150-157 (the knobs table: IK2_UP_MS :152, IK2_CPU_Q8 :153, IK2_LAZY_CPU :154, IK2_LAZY_MAX/LAZY_PER :155 with the 84%-vs-49% hit delta, the limits :157) + :163 (the all-resident anti-regime) — src/ik2/ik2_main.cu:10-13 (the decode header), :305-330 (the CPU lane + the lazy ring), :825-865 (the cache: per-size-class pools, S3-FIFO, the Pool struct), :1108-1140 (get(): the hit path + the miss path + the ghost re-admission), :3492-3516 (the miss dispatcher: still-arriving diversion, free-first, lazy bounds, no-slot), :3655-3670 (the pool wait + the t_cpu EMA) — docs/INKLING2.md:265-269 (the FP8-KV drift disclosure, F3). Batch 380 / Plan 397 / Research 429 §Delta."
                .to_string(),
            ref_shape: r#"// ref_shape: every miss is resolved by STATIC precedence — copy from
// pinned host first, and a compute fallback only after the copy is
// WAITED FOR; the prices are config constants that never re-measure, a
// compute-only miss still evicts a cache slot, and a still-arriving
// lazy slot stalls the GPU on its own copy stream.

const PCIE_COPY_S: f64 = 0.36e-3; // config constant, never re-measured
const CPU_POOL_S: f64 = 0.50e-3;

struct NaiveTier {
    cached: std::collections::HashSet<u32>,
}

impl NaiveTier {
    /// every miss: wait for the copy; compute only if the copy is refused.
    /// The GPU idles behind the link the whole time — the serialized shape.
    fn miss(&mut self, id: u32) -> f64 {
        if self.cached.contains(&id) {
            return 0.0; // hit
        }
        let wall = PCIE_COPY_S; // static precedence: copy ALWAYS first
        self.cached.insert(id); // compute-only misses evict too
        wall
    }
}

// the serialized negative: cpu + copy back-to-back on every miss —
// 40-43 tok/s becomes 12.5 (llama.cpp's experts-on-CPU posture).
let mut tier = NaiveTier { cached: Default::default() };
let mut wall = 0.0;
for id in 0..8u32 {
    wall += tier.miss(id);
}
let _ = (wall, CPU_POOL_S);
"#.to_string(),
            opt_pattern: r#"// opt_pattern: the miss dispatcher prices BOTH host lanes by free-first
// comparison of their cumulative busy-until times, re-measured online
// (the 0.9/0.1 EMA); a computed miss's blocks ride the copy stream
// later (lazy) under in-flight + per-call bounds; a compute-only miss
// takes NO slot; a still-arriving slot diverts to the CPU; a pool that
// cannot complete refuses LOUD.

#[derive(Clone, Copy, PartialEq)]
enum Lane {
    Copy,
    Compute,
}

struct Tier {
    cached: std::collections::HashSet<u32>,
    acc_cpu: f64,
    acc_up: f64,
    t_cpu: f64, // EMA price, measured as it runs
    t_up: f64,
    lazy_in_flight: usize,
    lazy_max: usize,  // in-flight bound (IK2_LAZY_MAX)
    lazy_per: usize,  // per-call bound (IK2_LAZY_PER)
    lazy_this_call: usize,
    pool_alive: bool,
}

impl Tier {
    /// the free-first law: the lane that frees first takes the miss.
    fn dispatch(&mut self, id: u32) -> Result<Option<Lane>, &'static str> {
        if self.cached.contains(&id) {
            return Ok(None); // hit
        }
        if !self.pool_alive {
            // a pool that posts but never completes must refuse LOUD,
            // never silently stall the tier on a dead lane.
            return Err("cpu pool dead: lane prices unmeasurable, refuse");
        }
        let lane = if self.acc_cpu + self.t_cpu <= self.acc_up + self.t_up {
            self.acc_cpu += self.t_cpu;
            Lane::Compute
        } else {
            self.acc_up += self.t_up;
            Lane::Copy
        };
        if lane == Lane::Compute {
            // lazy: blocks follow on the copy stream, bounded per call
            // and in flight; past either bound: compute-only, NO slot —
            // nothing in VRAM is evicted for bytes we will not wait for.
            let ok = self.lazy_in_flight < self.lazy_max
                && self.lazy_this_call < self.lazy_per;
            if ok {
                self.cached.insert(id);
                self.lazy_in_flight += 1;
                self.lazy_this_call += 1;
            }
        } else {
            self.cached.insert(id);
        }
        Ok(Some(lane))
    }

    /// the online re-price: every completed CPU batch moves the EMA.
    fn reprice_cpu(&mut self, batch_s: f64, n_jobs: f64) {
        self.t_cpu = 0.9 * self.t_cpu + 0.1 * (batch_s / n_jobs);
    }
}

// the overlap arm: the token's GPU work proceeds while the host lanes
// serve the miss — wall = max(gpu, lane), not the serialized sum. The
// fixture's four-miss ledger (t_cpu 0.50 ms, t_up 0.36 ms, both accs
// zero): miss 1 prices COPY (0.50 > 0.36 — the honest pick), miss 2
// prices COMPUTE and takes its lazy slot (the next visit HITS), miss 3
// prices COPY again (the accumulators keep the honest ledger), miss 4
// prices COMPUTE past the per-call bound: compute-only, NO slot.
let mut tier = Tier {
    cached: Default::default(),
    acc_cpu: 0.0,
    acc_up: 0.0,
    t_cpu: 0.50e-3,
    t_up: 0.36e-3,
    lazy_in_flight: 0,
    lazy_max: 2,
    lazy_per: 1,
    lazy_this_call: 0,
    pool_alive: true,
};
let gpu_s: f64 = 15.0e-3; // the token's own GPU kernels
let lane_s: f64 = 0.36e-3; // the copy lane's price (the free-first pick here)
let overlapped = gpu_s.max(lane_s); // the law's arm
let serialized = gpu_s + lane_s; // the ref shape
assert!(overlapped < serialized, "the overlap is the win");
let _ = tier.dispatch(0u32);
"#.to_string(),
        },
    ]
}
