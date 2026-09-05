//! Entries slice -- Batch 114 (2 rules) -- the oMLX ANE-prefill + GDN
//! blocked-sequential distill (Research 138 / Plan 087). Source:
//! jundot/omlx @ e467261edc786efd33b1e9023d5c4a827f8aa1c1 (Apache-2.0;
//! re-cloned at the pin for this batch; a quote-verification lane
//! re-read all ten surfaces with exact line numbers). This file lands
//! rules #1 and #2 of the verdict table: the A- GOAT lead (the
//! exact-vs-reformulation boundary) and the highest-value D rule (the
//! bank-split ladder). Candidates #3-#6 landed as riders on B84 (x2),
//! B22, B107, B79; #7 (tri-engine CPU-fraction split +
//! warm-first-dispatch) stayed lane-intel (769's f-split + R4a warm
//! rows already track it; the warm class exists). Fixtures are original
//! minimal Rust reproductions (parse-only; no --fix path). Category
//! membership lives ONLY in `KERNEL_CATEGORIES` (mod.rs) -- never
//! encode it here.
use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic -- grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![

// ============================================================
// -- recurrence-prefill-exact-restructure-before-reformulation -- Batch 114 / Research 138 / Plan 087.
        KernelRule {
            rule_id: "recurrence-prefill-exact-restructure-before-reformulation".to_string(),
            gpu_concept: "When a sequential recurrence's prefill is bandwidth-bound on Apple-GPU-class silicon, restructure the EXACT recurrence's memory layout (token-block shared-memory staging + register-resident state) BEFORE reaching for an algebraic reformulation: the stock per-slice kernel re-reads k/q from device once per value-slice threadgroup => 32x redundant traffic (~13 GB per 16k-token layer, source-reported); the restructure stages k/q/v/gate/beta into threadgroup memory in TB-token blocks with cooperative coalesced loads, splits each value head into Dv/16 blocks (8x fewer threadgroups touching the same k/q rows, each row read from device exactly once per threadgroup), and keeps the state in per-thread register fragments (each thread owns one value-row's 16-wide d-range) -- the SAME-FLOPs form measured ~2x at 16K (14.9 vs 29.7 ms/layer) on M3 ULTRA (source-reported numbers -- never cite as M3-Max-scoped or as our measurement). THE BOUNDARY IS THE RULE: the FLOP-halving WY/FLA chunked reformulation measured a LOSER three independent times (oMLX, the Rapid-MLX port, the MTPLX port) and twice in our own campaign (riir-ai Benches 734/769 GDN-chunked arms 0.97x/0.98x -- the state-dependent work dominates ~6:1 over the solve the chunking parallelizes), so restructure-first is an empirically separated ORDERING, not a style preference. Decision content: (1) stage-once-per-block with a single threadgroup barrier per block; the hot time loop reads ONLY shared memory; (2) state register-resident per thread; (3) reduce the per-token contraction across the owning lanes with shuffles, never barriers (the sub-simdgroup radius -- see the B22 rider); (4) reach for the chunked form only with a measured e2e win on the target silicon. Distinct from first-order-affine-recurrence-associative-scan (B79 -- the parallelization axis for scan-ELIGIBLE recurrences; this rule says restructure the sequential form even when a scan exists, because the combine cost can exceed the re-read savings), from wy-transform-tree-verify-state-free (B82 -- the reformulation family's SOLVE strategy, adopted for tree-verify MEMORY pressure; this rule's regime is bandwidth-bound sequential PREFILL where that family measured a loser), from chunkwise-parallel-recurrence (the chunked form this rule says to hold), and from reduction-restructure-needs-source-owned-comparator (B95 -- certifying a restructured REDUCTION bit-exact; this rule chooses which FORM to build at all). Source: jundot/omlx @ e467261 (Apache-2.0) -- custom_kernels/qwen35_prefill/gdn.py:411-426 (the quantifying module comment), :429-448 (TB/DB constants + smem decls), :465-481 (cooperative staging + one barrier per block), :483-518 (the hot loop reading only smem, state in registers). Batch 114 / Research 138 / Plan 087.".to_string(),
            ref_shape: "// naive: per-slice sequential prefill -- every value-slice threadgroup
// re-reads the SAME k/q rows from device for its slice of every token.
// With Dv/4 slices that is 32x redundant k/q traffic: FLOPs unchanged,
// the bandwidth bill multiplied.
fn kq_device_reads(t_len: usize, dk: usize, slices: usize) -> u64 {
    // k and q each read once per token per slice
    (2 * t_len * dk * slices) as u64
}

fn prefill_per_slice(t_len: usize, dk: usize, dv: usize) -> u64 {
    kq_device_reads(t_len, dk, dv / 4) // one threadgroup per value slice
}
".to_string(),
            opt_pattern: "// optimized: block-sequential prefill -- stage k/q once per TB-token
// block into threadgroup memory (cooperative coalesced loads, ONE
// barrier per block), split value heads into Dv/16 blocks (8x fewer
// threadgroups per k/q row), and keep the state in register fragments;
// the hot time loop reads ONLY shared memory.
const TB: usize = 32;

fn staged_kq_device_reads(t_len: usize, dk: usize, dv: usize) -> u64 {
    // each row read exactly once per threadgroup; Dv/16 threadgroups
    // touch the same k/q rows instead of Dv/4 => 8x fewer touches
    (2 * t_len * dk * (dv / 16)) as u64
}

fn stage_block(k: &[f32], t0: usize, t_len: usize, dk: usize, smem: &mut [f32]) {
    let rows = TB.min(t_len - t0);
    for p in 0..rows * dk {
        // coalesced cooperative staging: one device read per (row, column)
        smem[p] = k[(t0 + p / dk) * dk + (p % dk)];
    }
    // threadgroup_barrier here; the hot loop below reads smem only
}
".to_string(),
        },

// ============================================================
// -- ane-bank-split-ladder -- Batch 114 / Research 138 / Plan 087.
        KernelRule {
            rule_id: "ane-bank-split-ladder".to_string(),
            gpu_concept: "When a monolithic accelerator procedure/weight bank fails to map at PROGRAM-CREATE (the device's finite address window -- the ANE's ~4 GiB device-address limit, load failure 0x20004; our own 0x50004 cascade at ~105 resident programs is the same class), split it with a NEAR-HALF-FIRST retry ladder instead of re-attempting the monolith: (1) the first retry aims at two near-halves per instance -- the fewest banks that can load where the monolith cannot AND the fewest resident programs; bank-count is a DETERMINISM axis (measured bit-stable across greedy reruns at two banks while many small banks occasionally diverged at a greedy tie), so prefer the fewest banks; (2) HALVE the per-bank byte cap on each subsequent retry, floored at a max-bytes bound; (3) an env-forced initial cap gives ops a lever without a rebuild. THE MEASUREMENT-HONESTY HALF (load-bearing): the driver's release is ASYNCHRONOUS -- after a failed attempt, drop the partially-loaded banks, force a GC pass, and SETTLE (a bounded sleep) BEFORE the next attempt's headroom re-measure; otherwise the check reads not-yet-reclaimed memory and either falsely blocks the next attempt or lets it pile on top. THE GATE HALF: gate EVERY attempt (including the first) on a physical-footprint fraction of total system memory (70% in source) and STOP the ladder before the OS jetsam killer -- a 4-attempt ladder with each attempt individually bounded still grew process RSS to 48.3 GB on a 48 GB machine and was jetsam-killed mid-retry (source-measured); degrade to the per-layer fallback, snapshot the (current, total) pair so 'why did it stop on this box' is answerable from one log line, and fail OPEN when the measurement is unavailable (a query failure never blocks the fast path -- a circuit breaker for a known failure mode, not a new hard dependency). Distinct from residency-set-wired-once-post-warm-persistent-footprint (B100 -- wiring the residency SET of an ALREADY-mapped footprint; this is the split ladder for a bank that cannot map at all), from fill-gate-speculation-until-working-set-resident (B94 -- admission gating for speculative execution against a resident working set; this is program-create mapping + bounded retry), from scratch-bank-isolates-one-shot-operands-from-hot-cache (B94 -- scratch-bank lifetime isolation, not a mapping ladder), and from chunk-encoder-submits-below-driver-limit (B26 -- encoder-level chunking below a driver limit; this is a compile/mapping ladder with an async-release settle between attempts). Adoption rows recorded: riir-ai Issue 876 (the prefill-first M3 queue) T-probes and Issue 769 T2/R1/R4a-R7 (the 0x50004 cascade at ~105 programs, jetsam-before-ladder, the program budget) point at exactly this ladder. Source: jundot/omlx @ e467261 (Apache-2.0) -- patches/qwen35_ane_prefill.py:2163-2271 (_bank_split_ladder: env cap :2184-2192, the headroom gate :2196-2209, drop+gc+settle :2222-2233, near-half + the determinism comment + halving :2242-2258), :61-82 (the footprint snapshot + headroom gate + the fail-open docstring), :50-58 (the measured jetsam justification + the 0.70/0.5s constants), :2300-2378 (the single-instance twin). Batch 114 / Research 138 / Plan 087.".to_string(),
            ref_shape: "// naive: compile the monolithic procedure bank; on failure retry the
// SAME monolith. Every attempt is individually bounded; the PROCESS is
// not -- partial loads pile up, the headroom check reads not-yet-
// reclaimed memory, and the OS killer decides for us mid-retry.
fn compile_monolith(weights: &[u8], attempts: usize) -> Option<Bank> {
    for _ in 0..attempts {
        if let Some(b) = map_bank(weights) {
            return Some(b);
        }
        // no release, no settle, no headroom gate
    }
    None
}

struct Bank;

fn map_bank(_w: &[u8]) -> Option<Bank> {
    None // 0x20004: exceeds the device address window
}
".to_string(),
            opt_pattern: "// optimized: near-half-first split ladder -- release + settle between
// attempts (the driver's reclaim is asynchronous), a phys-footprint
// fraction gate stops BEFORE the OS killer, an env cap for ops, and
// the per-layer fallback as the terminal answer.
const MAX_ATTEMPTS: usize = 4;
const FOOTPRINT_FRACTION: f64 = 0.70;
const SETTLE_SECS: f64 = 0.5;
const MIN_BANK_BYTES: usize = 1 << 30;

struct Bank;

fn compile_bank_ladder(weights: &[u8]) -> Option<Vec<Bank>> {
    let mut cap = 0usize; // 0 = unset: the first retry aims at near-halves
    for _ in 0..MAX_ATTEMPTS {
        let (current, total) = footprint_snapshot();
        if total > 0 && current as f64 >= FOOTPRINT_FRACTION * total as f64 {
            break; // stop the ladder before jetsam; fall back per-layer
        }
        match try_compile_split(weights, cap) {
            Ok(banks) => return Some(banks),
            Err(partial) => {
                drop(partial); // release the mappings we got
                settle(SETTLE_SECS); // gc + sleep: reclaim is asynchronous
                cap = if cap == 0 {
                    weights.len() / 2
                } else {
                    (cap / 2).max(MIN_BANK_BYTES)
                };
            }
        }
    }
    None
}

fn footprint_snapshot() -> (u64, u64) {
    (0, 0) // fail-open pair: a query failure never blocks the fast path
}

fn settle(_secs: f64) {}

fn try_compile_split(_w: &[u8], _cap: usize) -> Result<Vec<Bank>, Vec<Bank>> {
    Err(vec![])
}
".to_string(),
        },
    ]
}
