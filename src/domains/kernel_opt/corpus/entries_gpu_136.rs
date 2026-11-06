//! Entries slice 136 — Batch 303 (Plan 308 / Research 345): the splash
//! return-visit delta. Source: incoai/splash @ cabbad27 (Apache-2.0),
//! Metal decode kernels + paged attention tiles. Two coordinator slices
//! (A: golden-ratio spread / fp32-pipe role split / GDN tape / PV rescale;
//! B: Metal fma emission / cross-phase parity / Apple9 coop-tensor init /
//! quad-lane softmax), every quote re-verified byte-for-byte at the pin
//! (plan 308 T0, 2026-10-28). The fixtures are ORIGINAL minimal Rust
//! reproductions — nothing transcribed from the sources. All eight rules
//! are `NotBenchable` (latency-overlap, pipe-occupancy, traffic-model,
//! page-mix, determinism, correctness, and lane-geometry axes — no Pareto
//! bench in this corpus hosts them; the Batch-136 domain-reason class).

use super::KernelRule;

#[allow(clippy::too_many_lines)]
fn slice_splash_delta_a() -> Vec<KernelRule> {
    vec![
        // ── Rule 1 (category A — memory): a golden-ratio spread of decode
        // tile start steps hides the first-touch DRAM latency of a
        // freshly-written activation across co-resident tiles. ──
        KernelRule {
            rule_id: "golden-ratio-decode-tile-phase-spread".to_string(),
            gpu_concept: "MECHANISM (golden-ratio decode tile phase spread; category A — memory): \"Walking in lockstep (spread 0), every column tile of a projection fetches the same slice of the input at each step, which the operator before has just written and no cache holds yet, so the fetch sits on every tile's critical path. Spread (1), tile t starts (t * 40503 mod 2^16) / 2^16 of the way in, the golden ratio's fraction in 16 bits, which spreads the starts of any run of tiles evenly over the partition, and most tiles find their slice brought in by others. ops::Linear picks the walk (LinearConfig::spread); the MoE expert tiles and the prefill tile walk in lockstep.\" The implementation is one multiply, one mask, one shift — a wrapping, deterministic walk: \"if (!spread) return step_begin; return step_begin + uint(ulong((tile * 40503u) & 0xFFFFu) * (step_end - step_begin) >> 16);\" THE LAW: decode tiles streaming the SAME freshly-written activation stagger their START steps — reordering tile starts, never the arithmetic, so semantics are exact (every tile still runs every step once); co-resident tiles then sit on different slices at any tick, first-touch DRAM latency overlaps across tiles, and most tiles find their slice already brought in by another. Same tile id -> same offset: deterministic, replay-safe. REGIME: decode tiles over a step loop whose input the immediately preceding operator just wrote (cache-cold), many co-resident tiles, per-step accumulation order-independent. ANTI-REGIME: the source names it itself — \"the MoE expert tiles and the prefill tile walk in lockstep\" (per-expert tiles read disjoint slices; prefill has no freshly-streamed activation), and a lone tile gains nothing. KINSHIP (do not double-count): swa-evicted-prefix-tile-analytic-skip (entries_gpu_133) derives an analytic scan BOUND, shrinking the loop domain — THIS staggers starts WITHIN the same domain; compose, never re-own. cpasync-multistage-tile-pipeline (entries_gpu_09) owns the smem-ring STAGING axis. apple10-narrow-grid-register-tile-threadgroups-per-core (entries_gpu_117, the splash B268 wave — same source) owns the occupancy axis. mma-lane-map-whole-run-vector-loads-split-accumulator-chains (entries_gpu_117) owns the operand lane map, not tile phase. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'golden|40503|lockstep|phase spread' -> zero owners (the hits were other rules' passing words in entries_gpu_45/gpu_133/gpu_15); 'tile start|stagger|first step' -> no scan-phase owner. NotBenchable here (the gain is DRAM-latency overlap across CONCURRENTLY RUNNING tiles — a wall-clock effect no single-tile microbench can see; the fixture pins determinism, coverage-exactness, and the wall under a simulated cold-cache memory system). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/common/gguf_staged_tile.h:45-53, quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345.".to_string(),
            ref_shape: r"#![allow(dead_code)]

// ref_shape (rule: golden-ratio-decode-tile-phase-spread): decode tiles walk
// a freshly-written activation in LOCKSTEP -- every column tile starts at
// step 0, so at every step all TILES tiles fetch the same slice of an input
// no cache holds yet (the operator before just wrote it, and the run is long
// enough that no slice is ever revisited within it). One cold fetch serves
// them all, but it sits on EVERY tile's critical path: the wall time carries
// a full DRAM latency per step and nothing overlaps.

const TILES: usize = 8;
const STEPS: usize = 32;
const SLICES: usize = 32; // the streamed input: one cold slice per step
const LATENCY: usize = 3;

fn slice_of_step(step: usize) -> usize {
    step % SLICES // every step a distinct slice: nothing is ever warm
}

fn lockstep_phases() -> [usize; TILES] {
    [0; TILES]
}

/// One tick of the simulated cold-cache memory system. Returns how many
/// tiles issued or joined a cold fetch this tick (the burst size).
fn tick_sim(
    held: &mut [bool; SLICES],
    eta: &mut [usize; SLICES],
    phase: &[usize; TILES],
    pos: &mut [usize; TILES],
    wait: &mut [usize; TILES],
) -> usize {
    for sl in 0..SLICES {
        if eta[sl] > 0 {
            eta[sl] -= 1;
            if eta[sl] == 0 {
                held[sl] = true;
            }
        }
    }
    for t in 0..TILES {
        if wait[t] > 0 {
            wait[t] -= 1;
        }
    }
    let mut burst = 0usize;
    for t in 0..TILES {
        if wait[t] > 0 || pos[t] >= STEPS {
            continue;
        }
        let step = (phase[t] + pos[t]) % STEPS;
        let sl = slice_of_step(step);
        if held[sl] {
            pos[t] += 1; // hit: the step completes this tick
        } else if eta[sl] == 0 {
            eta[sl] = LATENCY; // this tile issues the cold fetch
            wait[t] = LATENCY;
            burst += 1;
        } else {
            wait[t] = eta[sl]; // rides a fetch another tile already issued
            burst += 1;
        }
    }
    burst
}

fn run(phases: &[usize; TILES]) -> (usize, usize, usize) {
    let mut held = [false; SLICES];
    let mut eta = [0usize; SLICES];
    let mut pos = [0usize; TILES];
    let mut wait = [0usize; TILES];
    let mut tick = 0usize;
    let mut burst_ticks = 0usize;
    let mut max_burst = 0usize;
    while pos.iter().any(|&p| p < STEPS) {
        let burst = tick_sim(&mut held, &mut eta, phases, &mut pos, &mut wait);
        tick += 1;
        if burst >= 2 {
            burst_ticks += 1;
            max_burst = max_burst.max(burst);
        }
    }
    (tick, burst_ticks, max_burst)
}

#[test]
fn lockstep_pays_full_latency_on_every_step() {
    let (wall, _, _) = run(&lockstep_phases());
    // every step stalls all tiles for the whole fetch: nothing overlaps, so
    // the wall is exactly STEPS cold-miss round trips plus STEPS steps.
    assert_eq!(wall, STEPS * (LATENCY + 1));
}

#[test]
fn lockstep_every_step_is_a_full_tile_burst_on_one_cold_slice() {
    let (_, burst_ticks, max_burst) = run(&lockstep_phases());
    // at every step ALL tiles miss the same cold slice together
    assert_eq!(burst_ticks, STEPS);
    assert_eq!(max_burst, TILES);
}

fn main() {}".to_string(),
            opt_pattern: r#"#![allow(dead_code)]

// opt_pattern (rule: golden-ratio-decode-tile-phase-spread): tile t begins
// (t * 40503 mod 2^16)/2^16 of the way in and wraps, so at any tick tiles sit
// on DIFFERENT slices -- cold fetches overlap, later tiles find their slice
// brought in. Starts reorder, never the arithmetic: every tile still runs
// every step exactly once.

const TILES: usize = 8;
const STEPS: usize = 32;
const SLICES: usize = 32;
const LATENCY: usize = 3;

fn slice_of_step(step: usize) -> usize {
    step % SLICES
}

// wrapping u32 math: same tile -> same offset, deterministic
fn golden_phase(tile: usize) -> usize {
    ((((tile as u32).wrapping_mul(40503) & 0xFFFF) as u64 * STEPS as u64) >> 16) as usize
}

fn spread_phases() -> [usize; TILES] {
    let mut p = [0usize; TILES];
    for (t, slot) in p.iter_mut().enumerate() {
        *slot = golden_phase(t);
    }
    p
}
fn tick_sim(
    held: &mut [bool; SLICES],
    eta: &mut [usize; SLICES],
    phase: &[usize; TILES],
    pos: &mut [usize; TILES],
    wait: &mut [usize; TILES],
    visited: &mut [[bool; STEPS]; TILES],
) -> usize {
    for sl in 0..SLICES {
        if eta[sl] > 0 {
            eta[sl] -= 1;
            if eta[sl] == 0 {
                held[sl] = true;
            }
        }
    }
    for t in 0..TILES {
        if wait[t] > 0 {
            wait[t] -= 1;
        }
    }
    let mut burst = 0usize;
    for t in 0..TILES {
        if wait[t] > 0 || pos[t] >= STEPS {
            continue;
        }
        let step = (phase[t] + pos[t]) % STEPS;
        let sl = slice_of_step(step);
        if held[sl] {
            pos[t] += 1;
            visited[t][step] = true;
        } else if eta[sl] == 0 {
            eta[sl] = LATENCY;
            wait[t] = LATENCY;
            burst += 1;
        } else {
            wait[t] = eta[sl];
            burst += 1;
        }
    }
    burst
}

fn run(phases: &[usize; TILES]) -> (usize, usize, usize, [[bool; STEPS]; TILES]) {
    let mut held = [false; SLICES];
    let mut eta = [0usize; SLICES];
    let mut pos = [0usize; TILES];
    let mut wait = [0usize; TILES];
    let mut visited = [[false; STEPS]; TILES];
    let mut tick = 0usize;
    let mut burst_ticks = 0usize;
    let mut max_burst = 0usize;
    while pos.iter().any(|&p| p < STEPS) {
        let burst = tick_sim(&mut held, &mut eta, phases, &mut pos, &mut wait, &mut visited);
        tick += 1;
        if burst >= 2 {
            burst_ticks += 1;
            max_burst = max_burst.max(burst);
        }
    }
    (tick, burst_ticks, max_burst, visited)
}

#[test]
fn golden_phase_is_deterministic_and_spreads_the_partition() {
    for t in 0..TILES {
        assert_eq!(golden_phase(t), golden_phase(t));
        assert!(golden_phase(t) < STEPS);
    }
    let mut offsets: Vec<usize> = (0..TILES).map(golden_phase).collect();
    offsets.sort_unstable();
    offsets.dedup();
    assert!(offsets.len() >= 6, "phases clustered: {:?}", offsets);
}

#[test]
fn spread_runs_every_step_exactly_once() {
    let (_, _, _, visited) = run(&spread_phases());
    for (t, row) in visited.iter().enumerate() {
        assert!(row.iter().all(|&v| v), "tile {t} skipped steps");
    }
}

#[test]
fn spread_overlaps_the_cold_fetches() {
    let (lock, spread) = (run(&[0usize; TILES]), run(&spread_phases()));
    assert!(spread.0 * 2 <= lock.0, "wall {} vs lockstep {}", spread.0, lock.0);
    assert!(spread.1 < lock.1, "burst ticks {} vs {}", spread.1, lock.1);
}

fn main() {}"#.to_string(),
        },
        // ── Rule 2 (category B — compute): the MMA owns Apple9's FP32 pipe —
        // every other FP32 op held to the minimum a group scale needs, exact
        // bf16 operands via one add, seeded span sums close the chains. ──
        KernelRule {
            rule_id: "fp32-pipe-mma-role-split-biased-bf16-operands".to_string(),
            gpu_concept: "MECHANISM (fp32-pipe role split with biased-exact bf16 MMA operands; category B — compute): \"The 8x8x8 MMA runs on the FP32 pipe there, so the kernel keeps every other FP32 operation to the minimum a group scale needs, and does the rest on the integer pipe: - weights enter the MMA as exact bf16: 128 + code for linear codes with a min, 160 + code - zero for seeded linear codes (one add), the code itself (converted) for 8-bit linear codes, or the codebook, int8 or grid value; - one MMA chain per 32 inputs (per 16 for 16-input scales), closed by the fp32 epilogue s * chain; a format with a min adds b * sum once per coefficient group... but at least once per 64-input span of the table, with the last chain of either (b = m - 128 s, or b = m for 8-bit codes; sum those inputs, from the span's sums of 32), so that it reads the current span alone; with a zero point, that chain starts from -160 * sum instead (the operand being 160 + code - zero), the table's seed of 16 inputs for 16-input scales, and Q2_K's min per 16 inputs adds b * seed with b = m / -160;\" — the seeded per-span input sums close each chain so the epilogue reads the current span alone (gguf_sgmatrix.h). THE LAW: on Apple9 the MMA occupies the FP32 pipe, so every non-MMA FP32 op is budgeted to the minimum a group scale needs (~5 per GROUP, not the six per ELEMENT a naive dequant-MAC pays) and the per-element work runs on the integer pipe: the bf16 operand is ONE integer add (128 + code, exact for codes <= 127 — past 255 the ninth significant bit is lost), the chain is exact integer arithmetic over 32 inputs, and the bias closes it once per group as (b - 128 s) * sum_x — bit-equal to the per-element form wherever the arithmetic stays exact. REGIME: weight-quantized GEMV/GEMM on Apple9-class tiles where the MMA and the scalar FP32 pipe are the same hardware. ANTI-REGIME: architectures with a separate FMA pipe (the role split buys nothing); codes outside the bf16-exact operand domain (the exactness argument dies); consumers whose accumulation must stay fp32 end to end (no integer chain to close). KINSHIP (do not double-count): one-pack-serves-all-schedules-nibble-order-contract (entries_gpu_125) + integer-mma-span-epilogue-dequant (entries_gpu_20) — the bias-trick owners (the Batch 277/B251 lane), cite, never re-own the bias algebra; magic-anchor-mma-exact-int-float-ffma-epilogue (entries_gpu_106) owns accumulator ANCHORING, a different trick; mma-lane-map-whole-run-vector-loads-split-accumulator-chains (entries_gpu_117) owns the lane map, not pipe roles. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'fp32 pipe|integer pipe|role split' -> zero owners (hits were passing words); '128 + code|160 + code' -> zero. NotBenchable here (the gain is pipe OCCUPANCY — FP32 scalar work displaced to the integer pipe while the MMA runs; a microbench of the epilogue alone cannot see it; the fixture pins the op-count model, the one-add operand domain, and bit-equality vs naive dequant-MAC). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/decode/linear_gguf_sgmatrix.metal:1-24 + runtime/metal/kernels/common/gguf_sgmatrix.h:13-20 (the seeded per-span sums), quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345.".to_string(),
            ref_shape: r#"#![allow(dead_code)]

// ref_shape (rule: fp32-pipe-mma-role-split-biased-bf16-operands): every
// weight is dequantized on the FP32 pipe and every product-accumulate is an
// FP32 scalar op -- six FP32 ops per input BEFORE the MMA's own work, all
// contending with the pipe the MMA occupies on Apple9. The bias folds in per
// element. The integer pipe: unused.

const G: usize = 32; // inputs per coefficient group

#[derive(Default)]
struct Ops {
    f32_ops: u32,
    int_ops: u32,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn workload(n: usize, seed: u64) -> (Vec<i32>, Vec<i32>) {
    let mut st = seed | 1;
    let codes: Vec<i32> = (0..n).map(|_| (lcg(&mut st) % 128) as i32).collect();
    let x: Vec<i32> = (0..n).map(|_| (lcg(&mut st) % 17) as i32 - 8).collect();
    (codes, x)
}

fn naive_dequant_mac(codes: &[i32], x: &[i32], s: f32, b: f32, ops: &mut Ops) -> f32 {
    let mut acc = 0.0f32;
    for j in 0..codes.len() {
        let c = codes[j] as f32;
        ops.f32_ops += 1; // cvt code -> float
        let w = s * c;
        ops.f32_ops += 1; // the scale
        let wb = w + b;
        ops.f32_ops += 1; // the bias fold, PER ELEMENT
        let p = wb * (x[j] as f32);
        ops.f32_ops += 2; // the multiply + the activation's cvt
        acc += p;
        ops.f32_ops += 1; // the accumulate
    }
    acc
}

#[test]
fn naive_burns_six_f32_ops_per_input_and_never_leaves_fp32() {
    let n = 4 * G;
    let (codes, x) = workload(n, 7);
    let mut ops = Ops::default();
    let sum = naive_dequant_mac(&codes, &x, 0.25, 2.0, &mut ops);
    assert_eq!(ops.f32_ops, 6 * n as u32, "six FP32 scalar ops per input");
    assert_eq!(ops.int_ops, 0, "the integer pipe sits idle");
    assert!(sum.is_finite());
}"#.to_string(),
            opt_pattern: r#"#![allow(dead_code)]

// opt_pattern (rule: fp32-pipe-mma-role-split-biased-bf16-operands): the MMA
// owns the FP32 pipe, so every other FP32 op is held to the minimum a group
// scale needs and the per-element work moves to the integer pipe: weights
// enter as exact bf16 via ONE add (128 + code), one MMA chain per 32 inputs,
// closed by the FP32 epilogue s * chain plus the bias once per group as
// (b - 128 s) * sum(x) read from the span's input sums.

const G: usize = 32; // inputs per MMA chain / coefficient group

#[derive(Default)]
struct Ops {
    f32_ops: u32,
    int_ops: u32,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn workload(n: usize, seed: u64) -> (Vec<i32>, Vec<i32>) {
    let mut st = seed | 1;
    let codes: Vec<i32> = (0..n).map(|_| (lcg(&mut st) % 128) as i32).collect();
    let x: Vec<i32> = (0..n).map(|_| (lcg(&mut st) % 17) as i32 - 8).collect();
    (codes, x)
}

/// truncation to bf16 is lossless iff the value is bf16-representable
fn bf16_exact(v: f32) -> bool {
    f32::from_bits(v.to_bits() & 0xFFFF_0000) == v
}

fn naive_dequant_mac(codes: &[i32], x: &[i32], s: f32, b: f32, ops: &mut Ops) -> f32 {
    let mut acc = 0.0f32;
    for j in 0..codes.len() {
        let c = codes[j] as f32;
        ops.f32_ops += 1;
        let w = s * c;
        ops.f32_ops += 1;
        let wb = w + b;
        ops.f32_ops += 1;
        let p = wb * (x[j] as f32);
        ops.f32_ops += 2;
        acc += p;
        ops.f32_ops += 1;
    }
    acc
}

fn mma_role_split(codes: &[i32], x: &[i32], s: f32, b: f32, ops: &mut Ops) -> f32 {
    // the bias correction, computed once -- not per element
    let b_corr = b - 128.0 * s;
    ops.f32_ops += 2;
    let mut acc = 0.0f32;
    for (gi, chunk) in codes.chunks(G).enumerate() {
        let lo = gi * G;
        let mut chain: i64 = 0;
        let mut sum_x: i64 = 0;
        for j in 0..chunk.len() {
            chain += ((128 + codes[lo + j]) * x[lo + j]) as i64;
            ops.int_ops += 2; // ONE add makes the bf16 operand; the MAC is integer
            sum_x += x[lo + j] as i64;
            ops.int_ops += 1; // the span's seeded input sums
        }
        let cf = chain as f32;
        ops.f32_ops += 1;
        acc += s * cf;
        ops.f32_ops += 2; // the FP32 epilogue: one scale per group
        acc += b_corr * (sum_x as f32);
        ops.f32_ops += 2; // the bias closes the chain once per group
    }
    acc
}

#[test]
fn the_operand_is_one_add_and_exact_bf16() {
    // 128 + code for codes 0..=127 stays inside the bf16-exact integers
    for code in 0..128i32 {
        assert!(bf16_exact((128 + code) as f32), "128+{code} not exact bf16");
    }
    // past 255 the ninth significant bit is lost: the domain is a contract
    assert!(!bf16_exact(383.0));
}

#[test]
fn role_split_is_bit_equal_to_naive_dequant_mac() {
    // dyadic scales + small integers keep every intermediate exact on both
    // paths, so different summation orders still produce identical bits
    for (seed, s, b) in [(7u64, 0.25f32, 2.0f32), (11, 0.5, -1.0)] {
        let n = 2 * G;
        let (codes, x) = workload(n, seed);
        let mut o1 = Ops::default();
        let mut o2 = Ops::default();
        let naive = naive_dequant_mac(&codes, &x, s, b, &mut o1);
        let split = mma_role_split(&codes, &x, s, b, &mut o2);
        assert_eq!(naive.to_bits(), split.to_bits(), "pipe reordering moved a bit");
    }
}

#[test]
fn role_split_moves_the_work_to_the_integer_pipe() {
    let n = 2 * G;
    let (codes, x) = workload(n, 7);
    let mut o1 = Ops::default();
    let mut o2 = Ops::default();
    let _ = naive_dequant_mac(&codes, &x, 0.25, 2.0, &mut o1);
    let _ = mma_role_split(&codes, &x, 0.25, 2.0, &mut o2);
    // ~5 FP32 ops per GROUP (the epilogue), plus the one bias correction
    assert_eq!(o2.f32_ops, 2 + 5 * (n / G) as u32);
    assert!(o2.f32_ops * 8 < o1.f32_ops, "FP32 ops must shrink by >= 8x");
    // operands + chains + input sums run on the integer pipe
    assert_eq!(o2.int_ops, 3 * n as u32);
}

fn main() {}"#.to_string(),
        },
        // ── Rule 3 (category G): a GDN decode step leaves its rows on the
        // lane's tape; the next step folds the retained ones into the state
        // before its own rows — one state RMW per step, not per row. ──
        KernelRule {
            rule_id: "gdn-decode-tape-deferred-retained-row-fold".to_string(),
            gpu_concept: "MECHANISM (GDN decode tape: deferred retained-row fold; category G — recurrent state-write scheduling): \"Which of a step's rows a lane retains is known only after the step, so the state a step writes is the one before its rows: it leaves the rows on the lane's tape, and the lane's next step folds the retained ones into the state before its own rows. A step reads and writes the state once.\" THE LAW: under speculative verify a decode step's retained-row set is only known AFTER the step, so instead of read-modify-writing the recurrent state per retained row (two state-memory ops per row — worst case two per row in the step), the lane parks the step's rows on its own tape (lane-local memory — no state traffic) and the NEXT step folds the retained ones into the state before its own rows: one read + one write per STEP, with a stream-end flush for the last tape. Worst-case state traffic drops by the rows-per-step factor; the fold order (step-major, row-in-step) is preserved exactly, so the final state is bit-identical. REGIME: recurrent decode with step-dependent retained counts — speculative verification, where acceptance is decided per row after the step ran. ANTI-REGIME: non-speculative decode (retention deterministic before the write — fold inline); prefill (own capture path — mined B236); single-row steps (nothing to defer). KINSHIP (do not double-count): whole-step-gdn-decode-window-one-launch (entries_gpu_126) owns the launch shape — the whole step as ONE launch with register-resident state; THIS owns the state-write SCHEDULING across steps. gdn-conv-chain-fusion-gather-relocation-done-set (entries_gpu_131, the B295 backward conv chain) owns the backward pass. lazy-rollback-journaled-window-replay-replaces-eager-state-snapshots (entries_gpu_105) is journaled REPLAY of a window — a different mechanism (undo log, not deferred fold). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'tape|retained row|deferred fold|next step folds' -> zero owners; 'gdn' -> the three kins above, none owns deferred state-write scheduling. NotBenchable here (the gain is state-memory traffic per step under a verifier's retention pattern — a speculative workload is needed to observe it; the fixture pins the op model (2 per folding step + flush vs 2 per retained row), the preserved application order, and final-state equality on a seeded retention stream). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/decode/gdn.metal:22-26, quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345.".to_string(),
            ref_shape: r#"#![allow(dead_code)]

// ref_shape (rule: gdn-decode-tape-deferred-retained-row-fold): a
// speculative-verify decode step learns which of its rows are retained only
// AFTER the step, and this shape read-modify-writes the recurrent state once
// per RETAINED row -- a read and a write of state per row, so the worst case
// (every row retained) pays two state-memory ops per row per step.

const STEPS: usize = 16;
const ROWS: usize = 4;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

/// deterministic per-(step, row) retention: the verifier's answer
fn retained(step: usize, row: usize) -> bool {
    let mut g = Lcg(((step as u64) << 32) ^ row as u64 ^ 0x9E37);
    g.next() % 3 != 0
}

/// the row's fold contribution
fn row_value(step: usize, row: usize) -> i64 {
    let mut g = Lcg(0xA5A5 ^ ((step as u64) << 8) ^ row as u64);
    (g.next() % 97 + 1) as i64
}

/// per-row RMW: read + write the state once per RETAINED row
fn per_row_rmw() -> (i64, usize, Vec<(usize, usize)>) {
    let mut state = 0i64;
    let mut ops = 0usize;
    let mut order = Vec::new();
    for step in 0..STEPS {
        for row in 0..ROWS {
            if retained(step, row) {
                let v = state; // the READ
                state = v + row_value(step, row); // the WRITE
                ops += 2;
                order.push((step, row));
            }
        }
    }
    (state, ops, order)
}

#[test]
fn per_row_rmw_costs_two_state_ops_per_retained_row() {
    let (_state, ops, order) = per_row_rmw();
    assert_eq!(ops, 2 * order.len(), "a read + a write per retained row");
    // the worst case is per-ROW, not per-step: with 4 rows per step the
    // retained count dwarfs the step count
    assert!(
        order.len() > STEPS,
        "{} retained rows over {} steps: per-row traffic dominates",
        order.len(),
        STEPS
    );
}"#.to_string(),
            opt_pattern: r#"#![allow(dead_code)]

// opt_pattern (rule: gdn-decode-tape-deferred-retained-row-fold): the step
// leaves its rows on the lane's tape (lane-local -- no state-memory
// traffic); the lane's NEXT step folds the retained ones into the state
// before its own rows, and a stream-end flush folds the last tape. One read
// and one write of state per STEP, not per row.

const STEPS: usize = 16;
const ROWS: usize = 4;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

fn retained(step: usize, row: usize) -> bool {
    let mut g = Lcg(((step as u64) << 32) ^ row as u64 ^ 0x9E37);
    g.next() % 3 != 0
}

fn row_value(step: usize, row: usize) -> i64 {
    let mut g = Lcg(0xA5A5 ^ ((step as u64) << 8) ^ row as u64);
    (g.next() % 97 + 1) as i64
}

fn per_row_rmw() -> (i64, usize, Vec<(usize, usize)>) {
    let mut state = 0i64;
    let mut ops = 0usize;
    let mut order = Vec::new();
    for step in 0..STEPS {
        for row in 0..ROWS {
            if retained(step, row) {
                let v = state;
                state = v + row_value(step, row);
                ops += 2;
                order.push((step, row));
            }
        }
    }
    (state, ops, order)
}

fn tape_sum(tape: &[(usize, usize)]) -> i64 {
    tape.iter().map(|&(s, r)| row_value(s, r)).sum()
}

/// deferred fold: one state read + write per step that folds, plus the flush
fn taped_deferral() -> (i64, usize, Vec<(usize, usize)>) {
    let mut state = 0i64;
    let mut ops = 0usize;
    let mut order = Vec::new();
    let mut tape: Vec<(usize, usize)> = Vec::new();
    for step in 0..STEPS {
        // fold the PREVIOUS step's retained rows first
        if !tape.is_empty() {
            let acc = tape_sum(&tape);
            let v = state; // the one READ
            state = v + acc; // the one WRITE
            ops += 2;
            order.extend(tape.drain(..));
        }
        // then this step's own rows land on the tape
        for row in 0..ROWS {
            if retained(step, row) {
                tape.push((step, row));
            }
        }
    }
    // stream end: the flush folds the last step's tape
    if !tape.is_empty() {
        let acc = tape_sum(&tape);
        let v = state;
        state = v + acc;
        ops += 2;
        order.extend(tape.drain(..));
    }
    (state, ops, order)
}

#[test]
fn tape_folds_defer_to_one_state_rmw_per_step() {
    let (s1, ops_ref, _) = per_row_rmw();
    let (s2, ops_opt, _) = taped_deferral();
    // one read + one write per folding step, bounded by STEPS folds + flush
    assert!(ops_opt <= 2 * (STEPS + 1), "per-step RMW: got {ops_opt}");
    assert!(ops_opt < ops_ref, "deferral must beat per-row RMW: {ops_opt} vs {ops_ref}");
    assert_eq!(s1, s2, "the same rows reach the state");
}

#[test]
fn deferred_folds_preserve_the_application_order() {
    // the tape folds in step-major, row-in-step order -- exactly the order
    // the per-row form applies them in
    let (_, _, o1) = per_row_rmw();
    let (_, _, o2) = taped_deferral();
    assert_eq!(o1, o2);
}"#.to_string(),
        },
        // ── Rule 4 (category B — compute): the PV accumulator's rescale pass
        // is gated on a shared flag that only a grown running maximum sets —
        // scale-1.0 pages skip the whole multiply. ──
        KernelRule {
            rule_id: "conditional-pv-accumulator-rescale".to_string(),
            gpu_concept: "MECHANISM (conditional PV accumulator rescale; category B — compute): the scores side: \"previous_scale[fused_row] = scale; row_sum[fused_row] = row_sum[fused_row] * scale + local_sum; row_max[fused_row] = next_max; if (scale != 1.0f) atomic_store_explicit(rescale, 1u, memory_order_relaxed);\" the PV side: \"if (atomic_load_explicit(rescale, memory_order_relaxed))\" gates a pass that, per accumulator element (skipping invalid slots when the running tile is not full), reads the element's row coordinate and does \"running[index] *= previous_scale[coordinates[1]];\" — \"Rows whose running maximum grew atomically set the shared boolean rescale\". THE LAW: the M x D PV accumulator's rescale multiply is gated on a SHARED flag that only a row whose running maximum actually grew can set — a page where no max moved has every previous_scale exactly 1.0 (exp(0)), so skipping the whole multiply pass is arithmetic-preserving BY CONSTRUCTION: the multiply happens iff it must. REGIME: long stable-context pages where the running max rarely moves (established contexts, low-delta keys). ANTI-REGIME: first pages and high-dynamic scores — the flag store is pure overhead and the rescale fires every page anyway. KINSHIP (do not double-count): per-block-int32-chunk-rescale-fp32-accum (entries_gpu_33) owns the EXACTNESS-side accumulator rescale on tensor-core paths — a different axis (exactness, not conditional skip); softmax-denominator-hoist-out-of-weighted-v (entries_gpu_06) owns denominator hoisting; softmax-stats-precompute-exp-in-gemm-loader (entries_gpu_46) owns stats precompute. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'rescale' -> the exactness family + the softmax denominator rules; NO conditional-flag-gated rescale-skip owner; 'previous_scale|running maximum grew' -> zero. NotBenchable here (the win is skipped multiplies on pages where nothing moved — a page-mix-dependent effect; the fixture pins the flag semantics (flagged pages == growth pages), the op model, and bit-equality vs the unconditional rescale over a seeded page stream). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/common/paged_attention_tile.h:170-172, :244-248 (scores side), :359-374 (page reset + PV side), quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345. BATCH-339 FACE (TriAttention 805fae2c, Plan 348 / Research 380 -- the WARP-BALLOT ELISION face, the gate-form axis this host's conditional gate does not carry): the same 'the rescale fires iff it must' law at WARP-BALLOT scope -- the rescale's precondition is one warp-uniform vote, '__all_sync(0xffffffffu,alpha[0]==1.f && alpha[1]==1.f)' (prefill_sm89.cu:375, verbatim): the fold 'fma(out, alpha, -corr*alpha)' runs only when SOME lane's max moved, and the ride-along deferred correction (this batch's own rule) pays zero accumulator touches on a fully-stable warp -- the elision is the ballot, the shared-atomic-flag gate THIS host owns is the cross-warp form; same law, different synchronization scope, cite both ways never merge. Quotes verbatim at the pin. Batch 339 / Plan 348, count-neutral.".to_string(),
            ref_shape: r"#![allow(dead_code)]

// ref_shape (rule: conditional-pv-accumulator-rescale): after EVERY page the
// PV accumulator is rescaled row-by-row -- rows*DIM multiplies -- even when
// no running maximum moved on that page and every previous_scale is exactly
// 1.0. Those passes are dead work the conditional form skips.

const ROWS: usize = 4;
const DIM: usize = 8;
const PAGES: usize = 10;

/// the page's peak score per row: two pages move the running maxima
fn page_peak(page: usize, row: usize) -> f32 {
    const PEAKS: [f32; PAGES] = [40.0, 10.0, 10.0, 10.0, 10.0, 45.0, 10.0, 10.0, 10.0, 44.0];
    PEAKS[page] + (row as f32) * 0.25
}

fn local_pv(row: usize, d: usize) -> f32 {
    (((row * 3 + d) % 5) + 1) as f32 * 0.5
}

/// pages where at least one row's running maximum grew
fn count_any_growth_pages() -> usize {
    let mut running = [-1000.0f32; ROWS];
    let mut n = 0;
    for page in 0..PAGES {
        let mut grew = false;
        for r in 0..ROWS {
            let m = running[r].max(page_peak(page, r));
            if m > running[r] {
                grew = true;
            }
            running[r] = m;
        }
        if grew {
            n += 1;
        }
    }
    n
}

/// unconditional rescale: rows*DIM multiplies on EVERY page
fn unconditional() -> ([[f32; DIM]; ROWS], u32) {
    let mut acc = [[0.0f32; DIM]; ROWS];
    let mut running = [-1000.0f32; ROWS];
    let mut mults = 0u32;
    for page in 0..PAGES {
        let mut scale = [1.0f32; ROWS];
        for r in 0..ROWS {
            let next_max = running[r].max(page_peak(page, r));
            scale[r] = (running[r] - next_max).exp();
            running[r] = next_max;
        }
        for r in 0..ROWS {
            for d in 0..DIM {
                acc[r][d] = acc[r][d] * scale[r] + local_pv(r, d);
                mults += 1; // the multiply ALWAYS happens
            }
        }
    }
    (acc, mults)
}

#[test]
fn unconditional_rescales_every_page() {
    let (_acc, mults) = unconditional();
    assert_eq!(mults as usize, PAGES * ROWS * DIM);
}

#[test]
fn most_pages_multiply_by_an_exact_one() {
    // 8 of the 10 pages grew nothing: every scale on them is exactly 1.0,
    // so rows*DIM multiplies per such page are provably dead work
    let growth = count_any_growth_pages();
    let (_, mults) = unconditional();
    assert_eq!(growth, 2);
    assert!((PAGES - growth) * ROWS * DIM > 0);
    assert!(growth * ROWS * DIM < mults as usize);
}".to_string(),
            opt_pattern: r#"#![allow(dead_code)]

// opt_pattern (rule: conditional-pv-accumulator-rescale): rows whose running
// maximum grew set a SHARED boolean in the scores phase; the PV phase reads
// the flag ONCE and skips the whole multiply pass over the M x D accumulator
// when no row grew -- every skipped scale is exactly 1.0 (exp(0)), so the
// skip is arithmetic-preserving by construction.

const ROWS: usize = 4;
const DIM: usize = 8;
const PAGES: usize = 10;

fn page_peak(page: usize, row: usize) -> f32 {
    const PEAKS: [f32; PAGES] = [40.0, 10.0, 10.0, 10.0, 10.0, 45.0, 10.0, 10.0, 10.0, 44.0];
    PEAKS[page] + (row as f32) * 0.25
}

fn local_pv(row: usize, d: usize) -> f32 {
    (((row * 3 + d) % 5) + 1) as f32 * 0.5
}

fn count_any_growth_pages() -> usize {
    let mut running = [-1000.0f32; ROWS];
    let mut n = 0;
    for page in 0..PAGES {
        let mut grew = false;
        for r in 0..ROWS {
            let m = running[r].max(page_peak(page, r));
            if m > running[r] {
                grew = true;
            }
            running[r] = m;
        }
        if grew {
            n += 1;
        }
    }
    n
}

fn unconditional() -> ([[f32; DIM]; ROWS], u32) {
    let mut acc = [[0.0f32; DIM]; ROWS];
    let mut running = [-1000.0f32; ROWS];
    let mut mults = 0u32;
    for page in 0..PAGES {
        let mut scale = [1.0f32; ROWS];
        for r in 0..ROWS {
            let next_max = running[r].max(page_peak(page, r));
            scale[r] = (running[r] - next_max).exp();
            running[r] = next_max;
        }
        for r in 0..ROWS {
            for d in 0..DIM {
                acc[r][d] = acc[r][d] * scale[r] + local_pv(r, d);
                mults += 1;
            }
        }
    }
    (acc, mults)
}

/// conditional: a row whose scale != 1.0 stores the shared flag; the PV pass
/// is gated on one load of it
fn conditional() -> ([[f32; DIM]; ROWS], u32, usize) {
    let mut acc = [[0.0f32; DIM]; ROWS];
    let mut running = [-1000.0f32; ROWS];
    let mut mults = 0u32;
    let mut flagged = 0usize;
    for page in 0..PAGES {
        let mut scale = [1.0f32; ROWS];
        let mut rescale = false;
        for r in 0..ROWS {
            let next_max = running[r].max(page_peak(page, r));
            scale[r] = (running[r] - next_max).exp();
            if scale[r] != 1.0 {
                rescale = true; // the relaxed atomic store
            }
            running[r] = next_max;
        }
        if rescale {
            // the flag read gates the WHOLE multiply pass
            flagged += 1;
            for r in 0..ROWS {
                for d in 0..DIM {
                    acc[r][d] *= scale[r];
                    mults += 1;
                }
            }
        }
        for r in 0..ROWS {
            for d in 0..DIM {
                acc[r][d] += local_pv(r, d);
            }
        }
    }
    (acc, mults, flagged)
}

#[test]
fn the_shared_flag_gates_exactly_the_growth_pages() {
    let (_acc, mults, flagged) = conditional();
    let growth = count_any_growth_pages();
    assert_eq!(flagged, growth, "flagged pages == pages where a max grew");
    assert_eq!(mults as usize, growth * ROWS * DIM);
    let (_, uncond) = unconditional();
    assert!(mults < uncond, "{mults} vs {uncond} multiply ops");
}

#[test]
fn skipping_the_scale_one_pass_is_bit_exact() {
    let (a1, _) = unconditional();
    let (a2, _, _) = conditional();
    for r in 0..ROWS {
        for d in 0..DIM {
            assert_eq!(a1[r][d].to_bits(), a2[r][d].to_bits(), "r{r} d{d}");
        }
    }
}

fn main() {}"#.to_string(),
        },
    ]
}

#[allow(clippy::too_many_lines)]
fn slice_splash_delta_b() -> Vec<KernelRule> {
    vec![
        // ── Rule 5 (category N — numerics): Metal's fma() is emitted with
        //    every fast-math flag including reassociation, so an
        //    order-pinned chain must be WRITTEN as explicit multiplies
        //    and adds — the pin is a writing discipline, not a pragma. ──
        KernelRule {
            rule_id: "metal-fma-carries-fast-math-reassociate-off-cannot-pin".to_string(),
            gpu_concept: r#"MECHANISM (Metal fma() emission vs order pins; category N — numerics): On Metal, an order-pinned fp32 chain must be WRITTEN as explicit multiplies and adds — calling fma() inside a chain that must reproduce bitwise defeats the pin: "The chain is written as multiplies and adds, whose order `reassociate(off)` keeps: Metal's fma() is emitted with every fast-math flag, reassociation among them, which the pragma does not clear." THE LAW: a bitwise order pin on Metal is a WRITING discipline, not a flag setting — reassociate(off) keeps the order of a chain of multiplies and adds but cannot strip the fast-math set (reassociation included) the emitter attaches to fma(), so the only executable that reproduces the pinned order is one whose text spells the multiplies and adds. Bonus exactness datum (when contraction is free): "Each product of a half weight and a bf16 input is exact in fp32, so fused or not, every step rounds once (and as fast as the fma() chain)." — with product-exact inputs fma and explicit mul-add round identically per step, so the explicit writing costs nothing in the halfxbf16 decode shape. Row-independence discipline: "Every row sums alone, so a row's result does not depend on the other rows, on its place among them or on Rows." — row-local folds keep a pin per-row reproducible whatever the dispatch width. REGIME: Metal (fma fast-math emission); fp32 accumulate chains whose result crosses a sync/quorum/replay boundary; halfxbf16 product-exact decode shapes. ANTI-REGIME: chains where order is genuinely free (fast-math is a feature there — the flag-axis rule's lane); CUDA (different pragma semantics — flag-axis territory). This rule and rule 6 of this batch are the Issue-940 determinism law's Metal face (the workspace determinism law). KINSHIP (do not double-count): fast-math-contraction-behind-feature-flag (entries_gpu_03, category B — owns the FLAG-AXIS law: contraction gated per kernel via cfg_attr, the CUDA split rider, the source-restructure rider; THIS owns the Metal fma()-emission reality that makes order pins a WRITING discipline, not a flag setting); exact-op-parity-for-analytic-backward (op-parity discipline). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'fma()|reassociate|fast-math flag' hits the flag-axis rule only — NO Metal-emission owner; 'reassociate(off)' -> zero. NotBenchable here (determinism law — the gain is bit-reproducibility, not cycles; the source measures explicit-vs-fma as equal-speed with exact products). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/common/gguf_column_tile.h:16-19, quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345."#.to_string(),
            ref_shape: r#"#![allow(dead_code)]
// DEFECT SHAPE - an order-pinned fp32 chain written with fma() contractions
// and "protected" by reassociate(off). On Metal, fma() is emitted with every
// fast-math flag (reassociation among them) and the pragma does not clear
// it, so a fma()-written chain cannot reproduce a bitwise order pin: the
// fused op rounds once where the pinned explicit order rounds twice, and a
// regroup of its adds stays legally available to the emitter.

// The order-pinned REFERENCE step: multiply + round, then add + round.
fn pinned_step(w: f32, x: f32, acc: f32) -> f32 {
    acc + w * x
}

// What a Metal fma() chain executes: ONE rounding for the whole step.
fn fused_emission(w: f32, x: f32, acc: f32) -> f32 {
    w.mul_add(x, acc)
}

// The pinned fold of four simd partials, in channel order.
fn pinned_partial_order(p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    ((p0 + p1) + p2) + p3
}

// A regrouping that stays LEGAL under Metal's fma() fast-math flags:
// reassociate(off) does not clear them, so the emitter may keep this form.
fn reassociated_emission(p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    (p0 + p1) + (p2 + p3)
}

// "Half weight x bf16 input" pairs: at most 12 significant bits each, so
// every product below is EXACT in fp32 - the regime where contraction
// cannot move a rounding.
fn exact_pair(seed: u32) -> (f32, f32) {
    let w = f32::from_bits(0x3F00_0000 | (((seed * 7 + 1) & 0x07FF) << 12));
    let x = f32::from_bits(0x3F80_0000 | (((seed * 13 + 5) & 0x07FF) << 12));
    (w, x)
}

#[test]
fn fma_emission_cannot_reproduce_the_pinned_bits() {
    // a = b = 1 + 2^-23: the product is inexact (1 + 2^-22 + 2^-46); the
    // pinned order rounds it to 1 + 2^-22, then adds exactly that with a
    // negated copy: +0.0. The fma keeps the 2^-46 residual in its single
    // rounding. No pragma turns one rounding back into two.
    let a = f32::from_bits(0x3F80_0001);
    let c = -(a * a);
    let pinned = pinned_step(a, a, c);
    let fused = fused_emission(a, a, c);
    assert_eq!(pinned, 0.0, "pinned order must land on +0.0");
    assert_ne!(fused.to_bits(), pinned.to_bits(), "fma emission diverges");
}

#[test]
fn reassociation_off_does_not_pin_the_emitted_grouping() {
    let (p0, p1, p2, p3) = (1.0f32, 2f32.powi(-25), -1.0f32, 2f32.powi(-25));
    let pinned = pinned_partial_order(p0, p1, p2, p3);
    let regrouped = reassociated_emission(p0, p1, p2, p3);
    assert_ne!(
        pinned.to_bits(),
        regrouped.to_bits(),
        "a legal regroup must be able to move the bits"
    );
}

#[test]
fn exact_products_contract_for_free() {
    let mut acc_e = 0.0f32;
    let mut acc_f = 0.0f32;
    for s in 0..64u32 {
        let (w, x) = exact_pair(s);
        acc_e = pinned_step(w, x, acc_e);
        acc_f = fused_emission(w, x, acc_f);
    }
    assert_eq!(
        acc_e.to_bits(),
        acc_f.to_bits(),
        "exact products: fused and explicit round identically"
    );
}

#[test]
fn every_row_sums_alone() {
    let rows: Vec<Vec<(f32, f32)>> = (0..4u32)
        .map(|r| (0..8u32).map(|s| exact_pair(r * 8 + s)).collect())
        .collect();
    let sum = |row: &[(f32, f32)]| row.iter().fold(0.0f32, |a, &(w, x)| a + w * x);
    let full: Vec<u32> = rows.iter().map(|r| sum(r).to_bits()).collect();
    // The same row, recomputed alone and with other rows removed:
    let alone = sum(&rows[1]);
    let mut pruned = rows.clone();
    pruned.remove(0);
    pruned.remove(2);
    let pruned_row = sum(&pruned[0]); // was rows[1]
    assert_eq!(full[1], alone.to_bits());
    assert_eq!(full[1], pruned_row.to_bits());
}
"#.to_string(),
            opt_pattern: r#"#![allow(dead_code)]
// FIXED SHAPE - the order-pinned kernel path is WRITTEN as explicit
// multiplies and adds, never fma(), so the executed order IS the written
// order and the bit pin holds without any pragma. Exact products (half x
// bf16) contract for free, so the explicit writing costs nothing there; and
// every row sums alone, so a row's result never depends on the other rows,
// on their place among them or on Rows.

// The shipped kernel step: explicit multiply + round, then add + round.
fn kernel_step(w: f32, x: f32, acc: f32) -> f32 {
    acc + w * x
}

// The kernel chain: one lane's fold, its own loop shape.
fn kernel_chain(input: &[(f32, f32)]) -> f32 {
    input.iter().fold(0.0f32, |acc, &(w, x)| acc + w * x)
}

// A replay consumer: SAME written order, different loop SHAPE (unrolled by
// two). The pin pins order - loop shape must not move a bit.
fn replay_chain(input: &[(f32, f32)]) -> f32 {
    let mut acc = 0.0f32;
    let mut i = 0;
    while i + 1 < input.len() {
        let (w0, x0) = input[i];
        let (w1, x1) = input[i + 1];
        acc = kernel_step(w0, x0, acc);
        acc = kernel_step(w1, x1, acc);
        i += 2;
    }
    if i < input.len() {
        let (w, x) = input[i];
        acc = kernel_step(w, x, acc);
    }
    acc
}

// The fma emission, kept for contrast: what the discipline forbids.
fn fused_chain(input: &[(f32, f32)]) -> f32 {
    input.iter().fold(0.0f32, |acc, &(w, x)| w.mul_add(x, acc))
}

// Rounding-product inputs: 17 significant bits each, so the products ROUND.
fn rounding_inputs(n: u32, kind: u32) -> Vec<(f32, f32)> {
    (0..n)
        .map(|s| {
            let w = f32::from_bits(0x3F00_0000 | ((s ^ kind) & 0xFFFF));
            let x = f32::from_bits(0x3F80_0000 | ((s.wrapping_mul(2654435761) >> 7) & 0xFFFF));
            (w, x)
        })
        .collect()
}

#[test]
fn explicit_writing_reproduces_the_replay_bitwise() {
    for kind in 0..8u32 {
        let input = rounding_inputs(128, kind);
        assert_eq!(
            kernel_chain(&input).to_bits(),
            replay_chain(&input).to_bits(),
            "written order must survive every consumer shape"
        );
    }
}

#[test]
fn the_discipline_is_load_bearing_where_products_round() {
    // The crafted rounding-product step: pinned +0.0 vs the fma's 2^-46.
    let a = f32::from_bits(0x3F80_0001);
    let c = -(a * a);
    assert_ne!((c + a * a).to_bits(), a.mul_add(a, c).to_bits());
    // And over whole chains with rounding products the fma emission lands
    // somewhere else than the pinned explicit fold:
    let divergent = (0..8u32).any(|kind| {
        let input = rounding_inputs(256, kind);
        kernel_chain(&input).to_bits() != fused_chain(&input).to_bits()
    });
    assert!(divergent, "expected the fma emission to leave the pinned bits");
}

#[test]
fn rows_sum_alone_so_the_pin_is_per_row() {
    let rows: Vec<Vec<(f32, f32)>> =
        (0..4u32).map(|r| rounding_inputs(8, 16 * r + 1)).collect();
    let full: Vec<u32> = rows.iter().map(|r| kernel_chain(r).to_bits()).collect();
    for (r, row) in rows.iter().enumerate() {
        assert_eq!(full[r], kernel_chain(row).to_bits(), "row {r} moved alone");
    }
}
"#.to_string(),
        },
        // ── Rule 6 (category N — numerics): decode must reproduce the
        //    prefill gate BITWISE — same lane assignment, same
        //    channel-order partial fold, ops in the reference compiler's
        //    order — or ~1 in 10^5 outputs desync the shared state. ──
        KernelRule {
            rule_id: "gdn-cross-phase-gate-bitwise-parity".to_string(),
            gpu_concept: r#"MECHANISM (decode-vs-prefill bitwise parity on a shared recurrent gate; category N — numerics): when a decode kernel and a prefill kernel feed the SAME recurrent state, decode must reproduce prefill's arithmetic bitwise — same lane assignment, partials summed in the same channel order, operations written in the order the reference compiler emits: "the lane assignment and the channel-order sum of the four simd_sum partials reproduce the prefill gate, gdn_gate_phase, whose four simdgroups add their partials in that order. Reassociation is off and the operations are written in the order the compiler emits for gdn_gate_phase, so the rows are bitwise the same: with fast-math reassociation this shape rounded about one output in 10^5 differently." THE LAW: cross-phase parity is a construction requirement, not a test hope — any deviation (lane assignment, partial fold order, op order, reassociation) leaks a low-rate desync into shared state; the measured cost of skipping it is ~1 output in 10^5 rounding differently (source-measured), and each desynced gate poisons every later tick through the recurrence. REGIME: GDN/linear-attention decode vs prefill over one gate/state; four-simdgroup partial reductions folded in channel order; reassociation off. ANTI-REGIME: phases that never share state (per-phase independent outputs may diverge freely); backward-vs-forward pairs (the op-parity rule's lane). Together with rule 5 of this batch, the Issue-940 determinism law's Metal face. KINSHIP (do not double-count): exact-op-parity-for-analytic-backward (owns backward-vs-forward parity — THIS is decode-vs-prefill parity feeding shared state); metal-fma-carries-fast-math-reassociate-off-cannot-pin (rule 5 of this batch — the writing discipline this rule applies); bit-equality-oracle-keyed-on-reference-dispatch (the B242 oracle family). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'bitwise|cross-phase|decode.*prefill.*parity' hits the analytic-backward parity rule only; '10^5|one in' -> zero owners. NotBenchable here (correctness law — the 1-in-10^5 figure is the source's measurement of the DEFECT rate, not a speedup; the fixture proves parity construction, no cycle claim). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/decode/gdn.metal:289-295 (gdn_decode_gate vs the gdn_gate_phase prefill gate), quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345."#.to_string(),
            ref_shape: r#"#![allow(dead_code)]
// DEFECT SHAPE - decode reproduces the prefill gate only approximately.
// Prefill sums its four simdgroup partials in channel order with
// reassociation off; this decode assigns its lanes differently and folds the
// partials in the wrong channel order. On real gate data the source measured
// about one output in 10^5 rounding differently under fast-math
// reassociation - and each desync tick poisons every later tick through the
// shared recurrent state.

const CHANNELS: usize = 4; // four simdgroups, one partial each
const CHUNK: usize = 8; // channels per simdgroup partial (32 total)

// PREFILL reference: partial c sums its 8 channels in order; the gate folds
// the partials in channel order.
fn prefill_gate(x: &[f32; 32]) -> f32 {
    let mut gate = 0.0f32;
    for c in 0..CHANNELS {
        let mut partial = 0.0f32;
        for i in 0..CHUNK {
            partial += x[c * CHUNK + i];
        }
        gate += partial;
    }
    gate
}

// DECODE (buggy): same math, but the quad delivers and folds the four
// partials in a different channel order.
fn decode_gate_buggy(x: &[f32; 32]) -> f32 {
    let mut partials = [0.0f32; CHANNELS];
    for c in 0..CHANNELS {
        for i in 0..CHUNK {
            partials[c] += x[c * CHUNK + i];
        }
    }
    ((partials[3] + partials[2]) + partials[1]) + partials[0]
}

// Deterministic draw (LCG): log-uniform magnitudes, random signs.
struct Lcg(u64);
impl Lcg {
    fn next_value(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let e = ((self.0 >> 33) % 25) as i32; // magnitude 2^0 .. 2^-24
        let m = ((self.0 >> 40) & 0x00FF_FFFF) as f32 / 16_777_216.0;
        let v = m * 2f32.powi(-e);
        if self.0 >> 63 == 1 { -v } else { v }
    }
}

#[test]
fn crafted_input_desyncs_the_phases() {
    // partials 1.0, 2^-25, -1.0, 2^-25: the channel-order fold = 2^-25, the
    // quad's reversed fold = 0.0 - one desynced gate from one permuted fold.
    let mut x = [0.0f32; 32];
    x[0] = 1.0;
    x[8] = 2f32.powi(-25);
    x[16] = -1.0;
    x[24] = 2f32.powi(-25);
    assert_ne!(prefill_gate(&x).to_bits(), decode_gate_buggy(&x).to_bits());
}

#[test]
fn random_census_finds_desyncs_too() {
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
    let trials = 20_000;
    let mut mismatches = 0u32;
    for _ in 0..trials {
        let mut x = [0.0f32; 32];
        for v in x.iter_mut() {
            *v = rng.next_value();
        }
        if prefill_gate(&x).to_bits() != decode_gate_buggy(&x).to_bits() {
            mismatches += 1;
        }
    }
    assert!(mismatches > 0, "defect not reproduced ({mismatches} desyncs)");
}
"#.to_string(),
            opt_pattern: r#"#![allow(dead_code)]
// FIXED SHAPE - the decode gate reproduces the prefill gate BITWISE: the
// same lane assignment, the four simdgroup partials folded in the same
// channel order, and the operations written in the order the reference
// compiler emits (no fma contraction, no regrouping). Reassociation is off,
// so the rows are bitwise the same across phases and the shared recurrent
// state never desyncs.

const CHANNELS: usize = 4;
const CHUNK: usize = 8;

// PREFILL reference (gdn_gate_phase shape): partials in channel order.
fn prefill_gate(x: &[f32; 32]) -> f32 {
    let mut gate = 0.0f32;
    for c in 0..CHANNELS {
        let mut partial = 0.0f32;
        for i in 0..CHUNK {
            partial += x[c * CHUNK + i];
        }
        gate += partial;
    }
    gate
}

// DECODE (fixed): the SAME lane assignment, the SAME channel-order fold,
// the ops in the reference order - the quad's partials land where prefill's
// simdgroups put them and are added in that order.
fn decode_gate(x: &[f32; 32]) -> f32 {
    let mut partials = [0.0f32; CHANNELS];
    for c in 0..CHANNELS {
        for i in 0..CHUNK {
            partials[c] += x[c * CHUNK + i];
        }
    }
    let mut gate = partials[0];
    gate = gate + partials[1];
    gate = gate + partials[2];
    gate = gate + partials[3];
    gate
}

// Deterministic draw (LCG): log-uniform magnitudes, random signs.
struct Lcg(u64);
impl Lcg {
    fn next_value(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let e = ((self.0 >> 33) % 25) as i32;
        let m = ((self.0 >> 40) & 0x00FF_FFFF) as f32 / 16_777_216.0;
        let v = m * 2f32.powi(-e);
        if self.0 >> 63 == 1 { -v } else { v }
    }
}

#[test]
fn decode_reproduces_prefill_bitwise_across_the_census() {
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
    let trials = 50_000;
    let mut mismatches = 0u32;
    for _ in 0..trials {
        let mut x = [0.0f32; 32];
        for v in x.iter_mut() {
            *v = rng.next_value();
        }
        if prefill_gate(&x).to_bits() != decode_gate(&x).to_bits() {
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0, "cross-phase parity must hold bitwise");
}

#[test]
fn shared_state_stays_desync_free_over_the_trajectory() {
    // Each tick's gate feeds the next state; the decode trajectory must
    // equal the prefill-computed one bitwise, tick after tick.
    let mut rng = Lcg(0x0123_4567_89AB_CDEF);
    let mut state = [0.0f32; 32];
    for _ in 0..64 {
        for v in state.iter_mut() {
            *v = rng.next_value();
        }
        let g_pre = prefill_gate(&state);
        let g_dec = decode_gate(&state);
        assert_eq!(g_pre.to_bits(), g_dec.to_bits());
        for v in state.iter_mut() {
            *v = *v * 0.5 + g_pre * 1.0e-3;
        }
    }
}
"#.to_string(),
        },
        // ── Rule 7 (category M — Metal): on Apple9 runtime-format kernels
        //    a cooperative-tensor destination loses its constructor init —
        //    zero it ELEMENT-WISE before use, shader validation or not. ──
        KernelRule {
            rule_id: "apple9-coop-tensor-destination-init-lost-runtime-format".to_string(),
            gpu_concept: r#"MECHANISM (Apple9 cooperative-tensor destination init loss under runtime format selection; category M — kernel parametric polymorphism): on Apple9, in kernels whose tensor format is selected at RUNTIME, a cooperative-tensor destination that relies on constructor-initialized/returned-with-values LOSES those values: "returning an initialized cooperative tensor loses its initial values on Apple9 in runtime-format kernels (also with shader validation)". The source fix zeroes the destination element-wise before use — "for (ushort i = 0; i < acc.get_capacity(); ++i) acc[i] = 0.0f;" — the destination "zeroed by the caller". THE LAW: constructor/return-value initialization of a coop-tensor accumulator is not load-bearing on Apple9 runtime-format kernels — zero ELEMENT-WISE before first use; shader validation does not restore the lost init (the source found this via test failure, not a validator diagnostic). Correctness class: garbage accumulators — stale tile-pool contents flow into every output the kernel writes. REGIME: Apple9-family GPUs + runtime (not compile-time) format selection + cooperative-tensor DESTINATIONS. ANTI-REGIME: compile-time-format kernels (init survives — the constructor path is safe there); non-cooperative destinations (ordinary buffers keep their writes). KINSHIP (do not double-count): apple10-narrow-grid-register-tile-threadgroups-per-core (entries_gpu_117 — different chip, occupancy axis); coop-tensor-layout-measured-per-chip-family (the lithos-metal B227 rule — LAYOUT contract; THIS is the INIT-LOSS correctness hazard); extent-one-dim-skips-every-layout-check-symmetrically (entries_gpu_134 — layout-check family). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'cooperative tensor|runtime.format|initial value' hits the layout-contract rule only — NO init-loss owner. NotBenchable here (correctness hazard — the gain is outputs free of stale-pool garbage; no cycle claim to bench). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/common/gguf_staged_tile.h:19-22 (staged_accumulator's destination contract + gguf_zero), quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345."#.to_string(),
            ref_shape: r#"#![allow(dead_code)]
// DEFECT SHAPE - a cooperative-tensor destination that trusts its
// constructor initialization in a RUNTIME-FORMAT kernel. On Apple9 the
// initialized values are lost when the tensor format is selected at
// runtime: the tile comes back carrying stale tile-pool contents
// ("returning an initialized cooperative tensor loses its initial values
// on Apple9 in runtime-format kernels (also with shader validation)"), so
// the accumulator starts from garbage. Compile-time formats keep the init;
// shader validation rescues nothing.

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatSel {
    CompileTime,
    Runtime,
}

// Fake device: the tile pool holds whatever the PREVIOUS kernel left.
struct FakeDevice {
    stale_pool: [f32; 16],
    shader_validation: bool,
}

// A cooperative-tensor destination; CoopDest::new(init) initializes every
// slot - the shape a caller trusts.
#[derive(Clone)]
struct CoopDest {
    data: [f32; 16],
}

impl CoopDest {
    fn new(init: f32) -> Self {
        Self { data: [init; 16] }
    }
}

// Acquire a destination tile. On the Apple9 runtime-format path the
// initialization does NOT survive the acquire; the compile-time path keeps
// it; shader validation makes no difference to the loss.
fn acquire_dest(dev: &FakeDevice, sel: FormatSel) -> CoopDest {
    let _ = dev.shader_validation; // "also with shader validation": no rescue
    match sel {
        FormatSel::CompileTime => CoopDest::new(0.0),
        FormatSel::Runtime => CoopDest { data: dev.stale_pool }, // init LOST
    }
}

// The accumulate kernel: adds one page of values into the destination.
fn accumulate(dest: &mut CoopDest, page: &[f32; 16]) -> [f32; 16] {
    for (slot, v) in dest.data.iter_mut().zip(page.iter()) {
        *slot += *v;
    }
    dest.data
}

#[test]
fn runtime_format_loses_the_constructor_init() {
    let dev = FakeDevice { stale_pool: [7.5; 16], shader_validation: true };
    let page = [1.0f32; 16];
    let mut dest = acquire_dest(&dev, FormatSel::Runtime); // caller trusts new(0.0)
    let out = accumulate(&mut dest, &page);
    // Validation was ON and the stale tile still reached the accumulator.
    assert_eq!(out[0], 8.5, "stale pool contents leaked into the output");
    assert!(out.iter().all(|&v| v == 8.5));
}

#[test]
fn compile_time_format_keeps_init_the_regime_boundary() {
    let dev = FakeDevice { stale_pool: [7.5; 16], shader_validation: true };
    let page = [1.0f32; 16];
    let mut dest = acquire_dest(&dev, FormatSel::CompileTime);
    let out = accumulate(&mut dest, &page);
    assert!(out.iter().all(|&v| v == 1.0), "compile-time format keeps init");
}
"#.to_string(),
            opt_pattern: r#"#![allow(dead_code)]
// FIXED SHAPE - the destination is zeroed ELEMENT-WISE before use (the
// source fix: for (ushort i = 0; i < acc.get_capacity(); ++i) acc[i] =
// 0.0f, the destination "zeroed by the caller"), so the constructor's
// promise is never load-bearing. The same kernel is correct on the Apple9
// runtime-format path AND the compile-time path, shader validation on or
// off.

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatSel {
    CompileTime,
    Runtime,
}

struct FakeDevice {
    stale_pool: [f32; 16],
    shader_validation: bool,
}

struct CoopDest {
    data: [f32; 16],
}

fn acquire_dest(dev: &FakeDevice, sel: FormatSel) -> CoopDest {
    let _ = dev.shader_validation;
    match sel {
        FormatSel::CompileTime => CoopDest { data: [0.0; 16] },
        FormatSel::Runtime => CoopDest { data: dev.stale_pool }, // init lost
    }
}

// The fix: zero every slot before first use - never trust the constructor.
fn zero_elementwise(dest: &mut CoopDest) {
    for slot in dest.data.iter_mut() {
        *slot = 0.0;
    }
}

fn accumulate_fixed(dev: &FakeDevice, sel: FormatSel, page: &[f32; 16]) -> [f32; 16] {
    let mut dest = acquire_dest(dev, sel);
    zero_elementwise(&mut dest);
    for (slot, v) in dest.data.iter_mut().zip(page.iter()) {
        *slot += *v;
    }
    dest.data
}

#[test]
fn elementwise_zeroing_holds_on_both_format_paths() {
    let dev = FakeDevice { stale_pool: [7.5; 16], shader_validation: true };
    let page = [1.0f32; 16];
    let runtime = accumulate_fixed(&dev, FormatSel::Runtime, &page);
    let compile = accumulate_fixed(&dev, FormatSel::CompileTime, &page);
    assert!(runtime.iter().all(|&v| v == 1.0));
    assert!(compile.iter().all(|&v| v == 1.0));
}

#[test]
fn zeroing_clears_arbitrary_stale_tiles() {
    let dev = FakeDevice { stale_pool: [-1.0e9; 16], shader_validation: false };
    let page = [0.5f32; 16];
    let out = accumulate_fixed(&dev, FormatSel::Runtime, &page);
    assert!(out.iter().all(|&v| v == 0.5));
}
"#.to_string(),
        },
        // ── Rule 8 (category E — parallelism): four lanes own one fused
        //    row (eight tokens each) — row max/sum cut to TWO xor
        //    shuffles, eight bf16 probabilities leave as ONE 16-byte
        //    store; threads past 4 x FusedRows join caller barriers only. ──
        KernelRule {
            rule_id: "quad-lane-softmax-two-shuffle-16b-store".to_string(),
            gpu_concept: r#"MECHANISM (quad lane geometry co-designed for reduction depth AND store width; category E — parallelism): attention row statistics (max + sum over a 32-token page) and the probability store co-design the LANE GEOMETRY: "Four lanes own one fused row, eight consecutive tokens each, so the row maximum and sum are two xor shuffles instead of a 32-lane reduction per row and the eight bf16 probabilities leave as one 16-byte store." — the quad shape cuts the reduction to two xor-shuffle rounds (offsets 1 then 2 reach all four lanes) and turns each lane's eight bf16 probabilities into ONE 16-byte store, instead of a full 32-lane butterfly per row plus 32 scalar stores. THE LAW: co-design the lane geometry for reduction depth AND store width at the same time — one geometry decision buys both; a shape that fixes only one axis (narrow lanes, or wide stores without narrow reduction) leaves the other on the table. Inactive lanes do no private work: "Threads past 4 x FusedRows (kv4_g6: simdgroups 6 and 7) only take part in the caller's barriers." REGIME: 32-token pages, 256-thread threadgroups (N == 32 asserted in source), bf16 probability output. ANTI-REGIME: wider pages (a full-width butterfly wins when no 4-lane quad covers the row); threads past 4 x FusedRows — caller barriers only, never private work. KINSHIP (do not double-count): attention-softmax-state-exchange-once-per-block (entries_gpu_117, the B256 host — cross-BLOCK state exchange; THIS is intra-block lane geometry); warp-role-split-phase-overlap-cross-chunk-prefetch (the E family — role split across phases); subgroup-tail-lane-clamp (the collective-lane family). GREP EVIDENCE (over the corpus, 3+ vocabularies): 'xor shuffle|simd_shuffle|four lanes|fused row|16-byte store' -> zero owners. NotBenchable here (the lane geometry IS the benchmark — the fixture prices the op counts, which is the shape's whole claim; end-to-end cycle measurement belongs to the GOAT bench lane). Source: incoai/splash @ cabbad2732b027540f5a141511544e0bb62224b7 (Apache-2.0) — runtime/metal/kernels/common/paged_attention_tile.h:160-167 (kv4_g6 fused-row quads), quotes byte-verified at the pin (plan 308 T0, 2026-10-28). Batch 303 / Plan 308 / Research 345."#.to_string(),
            ref_shape: r"#![allow(dead_code)]
// DEFECT SHAPE - one 32-token attention page whose row statistics (max +
// sum) run a FULL 32-lane butterfly and whose probabilities leave
// token-by-token. Four lanes owning the fused row (eight consecutive tokens
// each) would cut the reduction to two xor shuffles and pack each lane's
// eight bf16 probabilities into one 16-byte store; this geometry pays five
// reduction rounds across all 32 lanes and 32 scalar stores per page.

const LANES: usize = 32;
const PAGE: usize = 32;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
struct Cost {
    shuffle_lane_ops: u64,
    scalar_bf16_stores: u64,
    wide16_bf16_stores: u64,
    lanes_used: u64,
}

// Online-softmax merge: the (max, sum) state one lane exchange carries.
fn merge(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let m = a.0.max(b.0);
    (m, a.1 * (a.0 - m).exp() + b.1 * (b.0 - m).exp())
}

fn bf16(v: f32) -> u16 {
    (v.to_bits() >> 16) as u16
}

// SLOW: five xor-shuffle rounds over all 32 lanes, then 32 scalar stores.
fn butterfly_page(x: &[f32; PAGE]) -> ([f32; PAGE], Cost) {
    let mut cost = Cost::default();
    let mut st = [(0.0f32, 0.0f32); LANES];
    for (l, s) in st.iter_mut().enumerate() {
        *s = (x[l], 1.0);
        cost.lanes_used += 1;
    }
    let mut off = LANES / 2;
    while off > 0 {
        for l in 0..LANES {
            st[l] = merge(st[l], st[l ^ off]);
            cost.shuffle_lane_ops += 1; // one exchange per lane per round
        }
        off /= 2;
    }
    let (m, s) = st[0];
    let mut probs = [0.0f32; PAGE];
    for t in 0..PAGE {
        probs[t] = (x[t] - m).exp() / s;
        let _ = bf16(probs[t]);
        cost.scalar_bf16_stores += 1; // one scalar bf16 store per token
    }
    (probs, cost)
}

fn page_inputs() -> [f32; PAGE] {
    let mut x = [0.0f32; PAGE];
    for (t, v) in x.iter_mut().enumerate() {
        *v = ((t * 37) % 13) as f32 * 0.1 - 0.6;
    }
    x
}

fn direct_probs(x: &[f32; PAGE]) -> [f32; PAGE] {
    let m = x.iter().cloned().fold(f32::MIN, f32::max);
    let s: f32 = x.iter().map(|&v| (v - m).exp()).sum();
    let mut p = [0.0f32; PAGE];
    for (t, slot) in p.iter_mut().enumerate() {
        *slot = (x[t] - m).exp() / s;
    }
    p
}

#[test]
fn full_butterfly_pays_the_wide_reduction_and_scalar_stores() {
    let (_, cost) = butterfly_page(&page_inputs());
    assert_eq!(cost.shuffle_lane_ops, 5 * LANES as u64); // 5 rounds x 32 lanes
    assert_eq!(cost.scalar_bf16_stores, PAGE as u64); // token-by-token stores
    assert_eq!(cost.wide16_bf16_stores, 0);
    assert_eq!(cost.lanes_used, LANES as u64); // every lane works every round
}

#[test]
fn butterfly_answer_is_correct_the_geometry_is_the_cost() {
    let x = page_inputs();
    let (probs, _) = butterfly_page(&x);
    let want = direct_probs(&x);
    for t in 0..PAGE {
        assert!((probs[t] - want[t]).abs() < 1e-5);
    }
}
".to_string(),
            opt_pattern: r"#![allow(dead_code)]
// FIXED SHAPE - the fused-row quad geometry. Four lanes own one fused row,
// eight consecutive tokens each: the row maximum and sum reduce in TWO xor
// shuffles (offsets 1 then 2 reach all four lanes) instead of a 32-lane
// reduction, and each lane's eight bf16 probabilities leave as ONE 16-byte
// store. Threads past 4 x FusedRows take part in the caller's barriers
// only - no private shuffles, no private stores.

const LANES: usize = 32; // the threadgroup's simd width
const QUAD: usize = 4; // lanes per fused row
const TOK_PER_LANE: usize = 8; // consecutive tokens per lane
const FUSED_ROWS: usize = 1; // rows fused per pass (N == 32 pages)
const PAGE: usize = QUAD * TOK_PER_LANE; // 32 tokens per page

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
struct Cost {
    shuffle_lane_ops: u64,
    scalar_bf16_stores: u64,
    wide16_bf16_stores: u64,
    lanes_used: u64,
}

// Online-softmax merge: the (max, sum) state one xor shuffle carries.
fn merge(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let m = a.0.max(b.0);
    (m, a.1 * (a.0 - m).exp() + b.1 * (b.0 - m).exp())
}

fn bf16(v: f32) -> u16 {
    (v.to_bits() >> 16) as u16
}

// FAST: quad reduction (offsets 1, 2) + one 16-byte store per active lane.
fn quad_page(x: &[f32; PAGE]) -> ([f32; PAGE], Cost) {
    let mut cost = Cost::default();
    let mut probs = [0.0f32; PAGE];
    let active = QUAD * FUSED_ROWS;
    for q in 0..(active / QUAD) {
        let base = q * QUAD * TOK_PER_LANE;
        let mut st = [(f32::MIN, 0.0f32); QUAD];
        for (l, s) in st.iter_mut().enumerate() {
            let mut m = f32::MIN;
            for t in 0..TOK_PER_LANE {
                m = m.max(x[base + l * TOK_PER_LANE + t]);
            }
            let mut sum = 0.0f32;
            for t in 0..TOK_PER_LANE {
                sum += (x[base + l * TOK_PER_LANE + t] - m).exp();
            }
            *s = (m, sum);
            cost.lanes_used += 1;
        }
        for off in [1usize, 2] {
            for l in 0..QUAD {
                st[l] = merge(st[l], st[l ^ off]);
                cost.shuffle_lane_ops += 1; // one xor shuffle per lane per round
            }
        }
        let (m, s) = st[0];
        for l in 0..QUAD {
            let mut pack = [0u16; TOK_PER_LANE]; // 8 bf16 = one 16-byte store
            for t in 0..TOK_PER_LANE {
                let p = (x[base + l * TOK_PER_LANE + t] - m).exp() / s;
                probs[base + l * TOK_PER_LANE + t] = p;
                pack[t] = bf16(p);
            }
            let _ = pack;
            cost.wide16_bf16_stores += 1;
        }
    }
    // Lanes past 4 x FusedRows never entered: caller's barriers only.
    (probs, cost)
}

fn page_inputs() -> [f32; 32] {
    let mut x = [0.0f32; 32];
    for (t, v) in x.iter_mut().enumerate() {
        *v = ((t * 37) % 13) as f32 * 0.1 - 0.6;
    }
    x
}

fn direct_probs(x: &[f32; 32]) -> [f32; 32] {
    let m = x.iter().cloned().fold(f32::MIN, f32::max);
    let s: f32 = x.iter().map(|&v| (v - m).exp()).sum();
    let mut p = [0.0f32; 32];
    for (t, slot) in p.iter_mut().enumerate() {
        *slot = (x[t] - m).exp() / s;
    }
    p
}

#[test]
fn quad_cuts_the_reduction_to_two_shuffles_and_one_wide_store() {
    let (_, cost) = quad_page(&page_inputs());
    assert_eq!(cost.shuffle_lane_ops, 2 * QUAD as u64); // offsets 1 then 2
    assert_eq!(cost.wide16_bf16_stores, QUAD as u64); // one 16-byte store/lane
    assert_eq!(cost.scalar_bf16_stores, 0);
    assert_eq!(cost.lanes_used, QUAD as u64); // threads past 4 x FusedRows idle
}

#[test]
fn quad_matches_the_row_answer() {
    let x = page_inputs();
    let (probs, _) = quad_page(&x);
    let want = direct_probs(&x);
    for t in 0..32 {
        assert!((probs[t] - want[t]).abs() < 1e-5);
    }
}
".to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    let mut v = slice_splash_delta_a();
    v.extend(slice_splash_delta_b());
    v
}
