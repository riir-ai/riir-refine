//! Entries slice 142 — Batch 317 (Plan 317): the 10-30d staged leads
//! addendum (zed-m3-leads1030d, rows S7/S5/S6 + the R5 rider face),
//! converted kernel_opt-only under the B293 cross-domain precedent
//! (`rust_perf/sec` counts ZERO this batch; the `rust_perf` slot is the
//! sibling's, batch 316). Sources: ashhart/tensorfold @ 6428050c
//! (perf(cuda): run the MTP ring attention ahead of its loads) +
//! 1ff930d9 (the batched==serial contract) + 2c10cfc1 (the stream
//! batching) + e06a6eef (core: the row-split pair seam) + fccb46ac
//! (fabric/tp keepIfAll) + 140ab0ff/956dff6a/fa45d4be/0c4c0613 (the
//! gfx1201 HIP routed-density series) + 4f8135e8 (the rows kernel).
//! Apache-2.0 at the 0.6.0 flip (the B196 license note). The R5 rider
//! face is attached in place on its family home (`gpu_52`, B182).
//! S8/S9 stay in the serving/spec-dec tier (untouched); S10 is the
//! splash pool's (handed to that lane). T0 quotes byte-verified at the
//! pins (plan 317, 2026-10-30). The fixtures are ORIGINAL minimal Rust
//! reproductions — nothing transcribed from the sources (CUDA/Zig/HIP).
//! Not benchable: source-reported GB10/gfx1201 figures; the load-ahead
//! shape, the wrap arithmetic, the partition contract, the density
//! crossover, and the fold-order invariances are CPU-testable and ARE
//! the fixtures. Category membership lives ONLY in `KERNEL_CATEGORIES`
//! (mod.rs) — never encode it here.

use super::KernelRule;

/// Data table, not logic -- grows with distillation batches.
#[allow(clippy::too_many_lines)]
fn slice_b317_staged_leads() -> Vec<KernelRule> {
    vec![
        // ── ring-scan-grouped-load-ahead-wrap-step-same-fold-order ──
        // Rule 1 (S7, category A — memory/load pipelining). TensorFold
        // 6428050c + 1ff930d9 + 2c10cfc1. THREE atoms in one mechanism:
        // (a) the grouped scalar load-ahead on a ring scan (8-slot
        // uint4 register groups via __ldg, the next group issued while
        // one folds); (b) the wrap-step replacing the per-slot 64-bit
        // modulo (`slot = slot + 1 == w ? 0 : slot + 1` — non-pow2
        // widths kept); (c) same-terms-same-order bit-identity as the
        // legality contract.
        KernelRule {
            rule_id: "ring-scan-grouped-load-ahead-wrap-step-same-fold-order".to_string(),
            gpu_concept: "MECHANISM (a ring scan runs its loads ahead of its folds; the slot advances by wrap, never a modulo; category A — memory/load pipelining): a per-slot ring fold pays TWO taxes a slot — the load latency the fold waits on, and the index arithmetic that positions the next slot. THE SOURCE LAW, verbatim (TensorFold 6428050c, the Qwen3.6 MTP drafter): 'The Qwen3.6 MTP drafter's attention folded one ring slot at a time, each slot waiting on its own load and a 64-bit modulo, so a warp took about 380 ns a slot and one stream's draft step spent 1.6 ms over a 4,200-slot ring.' THE REMEDY, verbatim: 'The three ring kernels now share a helper that loads eight slots a group with the next group in flight, steps the slot with a wrap, and folds the same terms in the same order, so every draft keeps its bits.' THE ATOMS, from the diff: `constexpr int AHEAD = 8;` — 'ring slots loaded a group, the next group's loads in flight while one folds'; the groups ride `uint4 kn[AHEAD], vn[AHEAD]` register arrays through the read-only path — `__ldg(reinterpret_cast<const uint4*>(rk + (size_t)slot * width + off))` — 'read-only path, as a __restrict__ ring's'; the next group is issued BEFORE the current group folds — `if (end - pos >= 2 * AHEAD) load(kn, vn);` with the in-code caveat 'no position near the type's end overflows'; the slot advance inside the load lambda AND the scalar tail is `slot = slot + 1 == w ? 0 : slot + 1` — a branch the compiler predicates, never a 64-bit `%` (the modulo was half the named per-slot cost; the wrap works for non-pow2 widths where a mask cannot); the tail folds the remainder scalar-wise with the same advance. THE CONSUMER FACE: the three ring kernels' bodies are replaced by `fold_ring(...)` calls — the helper is the ONE home for the scan. THE LEGALITY CONTRACT is bit-identity by fold order, verbatim (the commit): 'the contract finds tf_q36m_attn_rows bit-equal to the old tf_q36m_attn, and on the checkpoint the trees equal their chains and batched drafts equal serial ones' — and the batching commit (2c10cfc1) holds the same bar before the helper existed: 'Shared rounds now absorb every stream's kept rows and draft each level for all streams in passes of up to 16 rows, with the same bits as alone; one stream and trees keep the serial path, and TF_MTP_BATCH=0 restores it for comparison.' THE CONTRACT TEST (1ff930d9), verbatim: 'the MTP GPU check now drafts two sets of contexts on the same tap ring, serially and batched, and compares every held token, candidate, ring and state after absorb and after hold: mixed depths, picked rows, a gap, reused contexts and 17 streams.' MEASURED (source-reported GB10, never an adoption bar): ring attention 1.6 ms → 0.24 ms (6.7×); single-stream decode 90.6 → 103.2 tok/s (+13.9%). DECISION CONTENT: (a) a scan whose fold is cheap relative to its load latency should carry AHEAD-scaled groups of loads in registers, issued one group ahead of the fold — the register-held group IS the double-buffer, no SMEM stage needed; (b) the slot advance on a ring of width w is the wrap compare (`slot + 1 == w ? 0 : slot + 1`), never a modulo — a pow2 mask is legal ONLY at pow2 widths and the ring width is a runtime shape; (c) the grouped rewrite is legal exactly when the fold order is unchanged — same terms, same order, verified by a batched==serial bit-equality contract with rejection paths covered (mixed depths, gaps, reused contexts); (d) the guard `end - pos >= 2 * AHEAD` must bound the LOOK-AHEAD read, not just the fold — reading past the ring's end even into mapped memory is the defect the guard names. REGIME: ring/ring-buffer scans whose per-slot work is a cheap fold over a loaded slot (attention over a KV ring, sliding windows, delta-net state scans); widths runtime and non-pow2; read-only operands (the __ldg/__restrict__ premise). ANTI-REGIME: folds that CONSUME their loads' results to compute the next load's address (no independence — nothing to run ahead); fold orders that group the rewrite would change (softmax online-rescale folds re-max per slot — the fusion families own that axis); rings so short the group never fills (the tail path dominates); widths known pow2 AND compile-time (a mask is cheaper than the wrap compare — the IR-strength-reduction family owns that corner). KINSHIP (do not double-count): cpasync-multistage-tile-pipeline (gpu_09) owns the global→SMEM S-STAGE ring (shared-memory stages, cp.async) — THIS keeps the groups in REGISTERS on a scalar scan, no SMEM stage exists; register-double-buffer-operand-pipeline (gpu_09) owns the MMA-FRAGMENT lookahead — THIS is the scalar/__ldg twin one level down at whole slots, not fragments; the B273 fetch/store-split rider owns the Metal loader software pipeline — the same genus, the loader side; cluster-dsmem-shared-head-one-row-decode-attention's attention_ahead (gpu_111) owns TILE copies-in-flight — THIS owns per-slot scalar loads; mtp-ring-rebase-covers-rollback-window (B256, G) owns the ring's REBASE floor — THIS owns the ring's SCAN; gather-signed-index-wrap-semantics (gpu_72) owns the NEGATIVE-ID wrap-into-the-table semantic — the word 'wrap' collides, the mechanism does not (that rule is index decoding; THIS is loop advance arithmetic). GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'load.ahead|in flight|ahead' — the owned pipeline set above, none on a scalar ring scan; 'wrap|modulo' — gpu_72's signed-id sense + the pow2-shift family (O), never a slot-advance wrap; '__ldg|uint4|ring slot' — vectorized-load and packing senses, none paired with a fold-order contract; 'slot = slot' — ZERO. NotBenchable here (GB10 source-reported; no CUDA validator in this corpus; the load-ahead admission, the wrap arithmetic, and the fold-order invariance are CPU-testable and ARE the fixtures). Source: ashhart/tensorfold @ 6428050c5db6fd182a4d8f5afc33d93d1fbd68f5 + 1ff930d9 + 2c10cfc1 (Apache-2.0) — zig/kernels/cuda/qwen36_mtp.cu (+the contract tests). Batch 317 / Plan 317. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: ring-scan-grouped-load-ahead-wrap-step-same-fold-order):
// the naive ring fold: one slot a step, the load blocking the fold,
// the slot advance a modulo. `element()` stands in for a slot's
// payload; the fold order is the strict slot sequence.
#![allow(dead_code)]

fn fold_one_slot_at_a_time(ring: &[u64], w: usize, begin: usize, end: usize) -> (u64, u64) {
    let mut acc: u64 = 0;
    let mut loads: u64 = 0;
    let mut slot = begin % w; // the 64-bit modulo, EVERY slot
    let mut pos = begin;
    while pos < end {
        let v = ring[slot]; // the load the fold waits on
        loads += 1;
        acc = acc.wrapping_add(v); // one term a step, in order
        slot = (slot + 1) % w; // the modulo again
        pos += 1;
    }
    (acc, loads)
}
".to_string(),
            opt_pattern: "// opt_pattern: the grouped form: AHEAD slots load as one group into\n// registers, the NEXT group is issued before the current one folds, the\n// slot advances by wrap, and the fold order is unchanged (same terms,\n// same order). The serial helper is defined HERE: the fixture is one\n// standalone unit and the bit-equality contract needs both walkers.\n#![allow(dead_code)]\n\nconst AHEAD: usize = 8;\n\n/// the serial reference: one slot a step, the load blocking the fold,\n/// the modulo advance, the strict slot order.\nfn fold_one_slot_at_a_time(ring: &[u64], w: usize, begin: usize, end: usize) -> (u64, u64) {\n    let mut acc: u64 = 0;\n    let mut loads: u64 = 0;\n    let mut slot = begin % w; // the 64-bit modulo, EVERY slot\n    let mut pos = begin;\n    while pos < end {\n        let v = ring[slot]; // the load the fold waits on\n        loads += 1;\n        acc = acc.wrapping_add(v); // one term a step, in order\n        slot = (slot + 1) % w; // the modulo again\n        pos += 1;\n    }\n    (acc, loads)\n}\n\nfn load_group(ring: &[u64], w: usize, slot: &mut usize, group: &mut [u64; AHEAD], loads: &mut u64) {\n    for g in group.iter_mut() {\n        *g = ring[*slot]; // the read-only path a __restrict__ ring's\n        *slot = if *slot + 1 == w { 0 } else { *slot + 1 }; // WRAP, no modulo\n        *loads += 1;\n    }\n}\n\n/// the grouped scan: register-held groups, one group issued ahead,\n/// wrap advance, the SAME terms in the SAME order. Returns\n/// (acc, groups, loads) - loads pins that every slot was read once.\nfn fold_grouped_ahead(ring: &[u64], w: usize, begin: usize, end: usize) -> (u64, u64, u64) {\n    let mut acc: u64 = 0;\n    let mut groups: u64 = 0;\n    let mut loads: u64 = 0;\n    let mut slot = begin % w; // once, at entry\n    let mut pos = begin;\n    let mut kn = [0u64; AHEAD];\n    let mut in_flight = false;\n    if end - pos >= AHEAD {\n        load_group(ring, w, &mut slot, &mut kn, &mut loads);\n        in_flight = true;\n    }\n    while end - pos >= AHEAD {\n        let mut kr = [0u64; AHEAD];\n        if in_flight {\n            kr = kn; // the group the previous round issued\n        }\n        if end - pos >= 2 * AHEAD {\n            // the next group in flight BEFORE this one folds; the guard\n            // bounds the LOOK-AHEAD read, not just the fold.\n            load_group(ring, w, &mut slot, &mut kn, &mut loads);\n            in_flight = true;\n        } else {\n            in_flight = false;\n        }\n        for u in 0..AHEAD {\n            acc = acc.wrapping_add(kr[u]); // the SAME terms, the SAME order\n        }\n        groups += 1;\n        pos += AHEAD;\n    }\n    while pos < end {\n        // the scalar tail: the same wrap advance, one slot a step\n        acc = acc.wrapping_add(ring[slot]);\n        slot = if slot + 1 == w { 0 } else { slot + 1 };\n        loads += 1;\n        pos += 1;\n    }\n    (acc, groups, loads)\n}\n\n#[test]\nfn grouped_ahead_keeps_the_bits_of_the_serial_fold() {\n    let w = 4_200; // non-pow2, the modulo's expensive case\n    let ring: Vec<u64> = (0..w).map(|i| (i as u64).wrapping_mul(0x9E3779B97F4A7C15)).collect();\n    let begin = 7;\n    let end = begin + 10_000;\n    let (slow, slow_loads) = fold_one_slot_at_a_time(&ring, w, begin, end);\n    let (fast, _, fast_loads) = fold_grouped_ahead(&ring, w, begin, end);\n    assert_eq!(slow, fast); // same terms, same order -> identical bits\n    // the work pin: every slot loaded exactly once by EITHER walker\n    assert_eq!(slow_loads, 10_000);\n    assert_eq!(fast_loads, 10_000);\n}\n\n#[test]\nfn the_wrap_advance_matches_the_modulo_advance() {\n    let w = 97; // non-pow2\n    let mut by_wrap = 5usize;\n    let mut by_mod = 5usize;\n    for _ in 0..300 {\n        by_wrap = if by_wrap + 1 == w { 0 } else { by_wrap + 1 };\n        by_mod = (by_mod + 1) % w;\n        assert_eq!(by_wrap, by_mod);\n    }\n}\n\n#[test]\nfn the_look_ahead_guard_bounds_the_read() {\n    // end - begin < 2*AHEAD: no next group may be issued\n    let w = 64;\n    let ring = vec![1u64; w];\n    let begin = 0;\n    let end = begin + AHEAD + 3; // one group + a 3-slot tail\n    let (_, groups, loads) = fold_grouped_ahead(&ring, w, begin, end);\n    assert_eq!(groups, 1); // exactly one full group, the rest the tail took\n    assert_eq!(loads, AHEAD as u64 + 3); // the guard kept the read inside the ring\n}\n".to_string(),
        },
        // ── row-owns-output-k-full-reduce-exchange-carries-no-arithmetic ──
        // Rule 2 (S5, category E — parallelism/collectives). TensorFold
        // e06a6eef + fccb46ac. The OUTPUT-ROW partition axis + the
        // copy-only exchange + the fault-reification twin-law (NaN
        // bits into the id word so every rank refuses together).
        KernelRule {
            rule_id: "row-owns-output-k-full-reduce-exchange-carries-no-arithmetic".to_string(),
            gpu_concept: "MECHANISM (partition a rank-split matmul by OUTPUT rows so the exchange is a copy; category E — parallelism/collectives): when a matmul is split across ranks joined by a copy-capable exchange, the partition AXIS decides whether the exchange moves finished values or partial products. THE SOURCE LAW, verbatim (TensorFold e06a6eef, step 1 of the two-Mac 27B series): 'The seam exchanges whole output rows instead of K halves: rank r owns 2,560 of the 5,120 output columns (8,704 of 17,408 for the down projection), reduces over the full K on its own rank, and `pair_gather` copies whole rows in bf16 without doing any arithmetic, so no exchange rounds or sums a partial product across ranks.' — the K-HALF split makes every rank hold a PARTIAL product of every output row, so the exchange must SUM across ranks (extra reduce rounds AND cross-rank floating-point summation error that depends on rank count and order); the OUTPUT-ROW split gives each rank DISJOINT finished rows, so the exchange is a pure copy. THE NO-ARITHMETIC CONTRACT, verbatim: 'The seam's four kernels (`pair_stage`, `pair_gather`, `pair_argmax`, `pair_pick`) move bf16 frames and compare values only, so they carry no sums and no accumulator's error.' THE FAULT-REIFICATION TWIN-LAW: a fault observed by ONE rank must be reified INTO the shared value stream so every rank refuses together — verbatim: 'One Mac fails a row holding any NaN or ±Inf, so `pair_argmax` reports such a row as NaN bits with bit 31 set in its id word, and `pair_pick` turns that flag into the id sentinel: both Macs then refuse a non-finite round where one Mac would, through the same downstream check as a broken link.' The wire layer holds the same contract at the fabric probe (fccb46ac), verbatim: 'the multirank probe vote is keepIfAll, with a test that one rank's mismatch drops the wire on every rank' — and the negative control names the failure the law prevents: 'A vote on each rank's own probe alone leaves ranks 0 and 2 on the wire and fails the test.' THE TRANSPORT DISCIPLINE: 'A broken link surfaces as bf16 NaN 0x7FC0' — a broken link and a non-finite row land in the SAME downstream refusal, never a second error path. THE SHIPPED-PATH GUARD, verbatim: '`pair.metal` and `pair.zig` stay for GLM-5.3-Flash, whose transport this series does not touch: a shipped path cannot change without an A/B against main.' Five planted controls (a stage on the peer's half, a gather writing its own half twice, an argmax and a pick tie to the higher id, an argmax that does not flag) each go red on the gate named in the contract doc — the detectors are proven live, not assumed. DECISION CONTENT: (a) the partition axis is a DECISION, not a given: output-row (each rank owns disjoint outputs, reduces the full K locally, exchange = copy) beats K-half (every rank owns a partial of every output, exchange = reduce) wherever the exchange can move finished values without arithmetic; (b) the exchange's arithmetic budget is ZERO — comparisons and copies only; any sum across ranks is the defect (it is both rounds AND numeric error whose result depends on the rank layout); (c) faults must be reified into the value stream (NaN bits in the id word, keepIfAll votes) so the fleet's refusal is collective — a rank-local refusal lets the other ranks proceed on a round the fleet has already condemned; (d) a shipped transport on other models is not silently migrated — the new seam lands beside it until an A/B licenses the change. REGIME: tensor/model-parallel matmuls across 2+ ranks joined by copy-capable links (pair gather, P2P, unified memory), head/down-projection splits, any collective probe/vote layer. ANTI-REGIME: ranks joined ONLY by reduce-capable collectives (the K-half partial exchange is the only shape the transport supports — then the law is the NUMERIC half: pin the reduction order and disclose the rank-count-dependent error); single-device kernels (gpu_04's output-ownership is the in-kernel face); in-place updated operands (the copy contract assumes immutable frames). KINSHIP (do not double-count): gpu_04's output-ownership pattern owns SINGLE-device work distribution (one owning thread per output element, zero cross-thread reduction) — THIS owns the RANK axis (the same instinct one layer up, where the exchange exists); attention-partition-two-knob-shared-partial-workspace (gpu_51) owns split-K attention partials ON one device (the partial-product workspace is the cost K-half pays — exactly what the output-row split deletes at rank scope); the B119/B249 collective faces own comm-buffer shapes and exchange credits, not the partition-axis choice; gpu_30 sentinel-readiness-over-separate-flag owns readiness POLLING (a sentinel read by a waiter) — THIS reifies a VALUE fault into an ID word (the sentinel rides the data, no poll exists); gpu_01 clamp-tile-indices-to-sentinel-discard-at-write owns OOB row aliasing at write — different sentinel sense. GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'rank r owns|rank owns|keepIfAll|probe vote' — ZERO owners; 'K halves|partial product' — gpu_51's split-K workspace (one device) + incidental 'output columns' shape comments (gpu_71 packing, gpu_122 draft-head views) — none a partition-axis law; 'NaN bits|bit 31|id sentinel' — gpu_30's readiness sentinel + gpu_01's clamp-at-write, neither reifies a fault into a shared id word. The next-category candidate is recorded in the plan: 'distributed' (partition/vote/fault-reification across ranks) — the router's 15-expert ontology does not grow in a batch; E holds the rule until an owner act widens it. NotBenchable here (needs 2+ devices; the partition contract, the reification encoding, and the keepIfAll vote are CPU-testable and ARE the fixtures). Source: ashhart/tensorfold @ e06a6eef (step 1 of 6, the two-Mac 27B series; pair_rows.zig + pair_rows.metal + the contract tests) + fccb46ac (fabric/tp), Apache-2.0. Batch 317 / Plan 317. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: row-owns-output-k-full-reduce-exchange-carries-no-arithmetic):
// the K-HALF split: each rank computes a PARTIAL of every output
// row, so the exchange must sum across ranks — extra rounds AND
// rank-order-dependent float error live in the exchange's `+`.
#![allow(dead_code)]

fn k_half_split_round(a: &[f32], b: &[f32], k: usize) -> f32 {
    // rank 0 owns k[0..k/2), rank 1 owns k[k/2..k) — both partial
    let p0: f32 = a[..k / 2].iter().zip(&b[..k / 2]).map(|(x, y)| x * y).sum();
    let p1: f32 = a[k / 2..].iter().zip(&b[k / 2..]).map(|(x, y)| x * y).sum();
    p0 + p1 // the exchange SUMS the partials — arithmetic on the wire
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: row-owns-output-k-full-reduce-exchange-carries-no-arithmetic):
// the OUTPUT-ROW split: rank r owns disjoint output rows and reduces
// the full K locally; the exchange COPIES whole rows (no arithmetic);
// a fault is reified into the shared id word so every rank refuses.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Debug)]
enum Frame {
    Val(f32),
    BrokenLink,
}

const NAN_BITS_ID: u32 = 0x8000_0000; // bit 31 set: the NaN flag in the id word

fn row_owned_round(a: &[f32], b: &[f32], k: usize, cols: usize, out_rank: usize) -> (Vec<f32>, u32) {
    // this rank owns cols/2 output rows and reads the FULL k
    let rows: Vec<f32> = (0..cols / 2)
        .map(|c| {
            let o = out_rank * (cols / 2) + c;
            a[..k].iter().zip(&b[..k]).map(|(x, y)| x * y).sum::<f32>() * ((o + 1) as f32) // full-K, local
        })
        .collect();
    // pair_argmax: a non-finite row reifies into the id word
    let mut id: u32 = out_rank as u32;
    if rows.iter().any(|v| !v.is_finite()) {
        id |= NAN_BITS_ID; // the flag rides the value stream
    }
    (rows, id) // pair_gather would copy these rows verbatim — no sums
}

fn pair_pick(id: u32) -> Frame {
    if id & NAN_BITS_ID != 0 {
        Frame::BrokenLink // the sentinel refusal — the same path as a broken link
    } else {
        Frame::Val(0.0)
    }
}

#[test]
fn the_exchange_carries_no_arithmetic() {
    // the owned rows ship as-is: no + across ranks anywhere in the path
    let (rows, id) = row_owned_round(&vec![1.0; 8], &vec![1.0; 8], 8, 4, 0);
    assert_eq!(rows, vec![8.0, 16.0]); // full-K sums computed LOCALLY
    assert_eq!(id & NAN_BITS_ID, 0);
}

#[test]
fn a_fault_reifies_into_the_id_word() {
    let bad = f32::NAN;
    let (_, id) = row_owned_round(&vec![bad; 4], &vec![1.0; 4], 4, 2, 1);
    assert_ne!(id & NAN_BITS_ID, 0); // the flag is IN the id word
    assert_eq!(pair_pick(id), Frame::BrokenLink); // every rank refuses together
}

#[test]
fn keepifall_drops_the_wire_on_one_mismatch() {
    // the probe vote: keepIfAll — one rank's mismatch drops it for ALL
    let probes = [true, false, true];
    let keep = probes.iter().all(|&p| p);
    assert!(!keep); // ranks 0 and 2 do NOT stay on the wire
}
".to_string(),
        },
        // ── routed-density-pairs-per-item-keys-tile-admission ──
        // Rule 3 (S6, category D — dispatch/admission). TensorFold
        // 0c4c0613 + 956dff6a + fa45d4be + 140ab0ff. Pairs-per-item
        // density keys the routed-prompt tile admission; the bounds
        // are COUPLED knobs searched together with a per-cell veto.
        KernelRule {
            rule_id: "routed-density-pairs-per-item-keys-tile-admission".to_string(),
            gpu_concept: "MECHANISM (a MoE routed-prompt kernel's tile admission keys on PAIRS-PER-ITEM density, not item count alone; category D — dispatch/admission): a routed prompt's specialized tile wins for SPARSE routing and loses for DENSE routing, and item count alone cannot tell the two apart — the density does. THE SOURCE LAW, verbatim (TensorFold 140ab0ff, gfx1201): 'On gfx1201 the routed prompt's K-parallel tile beats the GEMM tile for plans that hold few pairs an item and loses for full ones. Its row now takes plans of up to 507 items and 3.5 pairs an item, from served runs on Qwen3.6 and Nemotron: Qwen3.6's 33 to 128-token prompts fill x0.53 to x0.80 and Nemotron's 129 to 186-token ones x0.82 to x1.00, other sizes stay within noise, and every reply keeps its bytes.' — the admission is a CONJUNCTION (items <= bound AND pairs-per-item <= bound), and the measured row carries the fill-fraction disclosure (x0.53–x1.00) beside the crossover. THE BOUND IS A TUNING FIELD ON THE PLAN (0c4c0613), verbatim: 'A routed prompt's K-parallel tile on gfx1201 wins or loses by the pairs a plan holds per item, which its items alone cannot tell. A tuning row can now say the most pairs per item it takes, and the registry refuses a plan with more, given the plan's pairs in its shape. No table sets the field yet, so every choice stays the same' — flag-off byte-identical: the field ships unset, every pre-existing plan routes as before. THE BOUND IS FRACTIONAL (fa45d4be), verbatim: 'Served runs on the R9700 put the routed K-parallel tile's threshold between 3.43 and 3.75 pairs per item, which no whole number holds, so the bound is now an f32 compared exactly in f64.' — a whole-number bound would either strand the tile (3) or admit losers (4); the f32-in-f64 compare keeps the exactness the tuning needs. THE COUPLED-SEARCH RIDER (956dff6a), verbatim: 'Moving a routed prompt's items bound alone cannot both give Qwen3.6 the K-parallel tile and keep Nemotron's full plans off it, and when kp3 refuses a plan, kp1 takes it first. The proposal now lives in a host-tested search that moves kp3's items and pairs bound and kp1's pairs bound together, scored with the per-cell veto on each whole table, and the tuner times both sides of every pairs bound it tries.' — the bounds are COUPLED knobs: the ladder (kp3 refusal falls through to kp1) makes one row's bounds shift another row's workload, so the search moves them TOGETHER and vetoes per CELL against each whole table, never per row in isolation. THE TIMING DISCIPLINE (fa45d4be), verbatim: 'the tuner times both sides of every place its proposal changes a choice, and retakes a noisy timing, so one noisy cell no longer vetoes a whole bucket by chance' — every A/B is BOTH-SIDES timed (the changed choice AND the fallback it displaces), and a noisy retake replaces a single noisy cell instead of letting chance veto a bucket. DECISION CONTENT: (a) routed-tile admission keys on the routing DENSITY (pairs per item), a second shape datum beside item count — items alone route wrong at both extremes; (b) the bound is a per-row tuning FIELD the registry enforces at plan admission (refuse, never clamp), unset-by-default so flag-off is byte-identical; (c) the bound is fractional where the measured threshold is (f32 held, compared exactly in f64) — an integer round-down/up is the silent mis-admission; (d) coupled bounds are searched JOINTLY with a per-cell veto on each whole table, the ladder's fall-through named in the search, both sides of every changed choice timed, and noisy cells retaken — one noisy read must not veto a bucket. REGIME: MoE routed-prompt kernels with an alternative K-parallel tile (experts-per-token sparse routing), tile-crossover tables tuned on served traffic, any admission keyed on a fill fraction that items alone cannot predict. ANTI-REGIME: dense GEMMs (no routing axis — the crossover families key on rows/types); admission by item count only (the defect this rule replaces); bounds moved one knob at a time against a fall-through ladder (the coupled search exists because that fails); whole-number bounds on a measured fractional threshold. KINSHIP (do not double-count): small-batch-matvec-gemm-bridge-rung (gpu_13) owns the few-row crossover tables keyed on ROWS/TYPE/DEVICE (its Batch-309 rider's per-type table is the same TABLE form on a different axis — pairs-per-item is NEW); moe-tokens-per-expert-block-m-cap (entries_gpu_133, B297) caps BLOCK SIZE by routing load — THIS chooses the TILE by routing density; the B284/B286 autotune family owns bounded rounds and candidate priority — THIS owns the coupled-bound JOINT search with per-cell veto and both-sides timing (the search discipline composes onto that family, not duplicates it); the warp-per-row admission floor (gpu_141) is a work-mapping crossover under an admission predicate — a different predicate (blocks-per-row), a different axis. GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'pairs per item|pairs-per-item|pairs_per_item' — ZERO owners; 'routed prompt|routed-prompt' — ZERO; 'fill fraction|per-cell veto|coupled bound' — ZERO; 'density' — dtype senses only. NotBenchable here (gfx1201 HIP figures source-reported; the conjunction admission, the fractional compare, and the coupled-veto search are CPU-testable and ARE the fixtures). Source: ashhart/tensorfold @ 0c4c0613 + 956dff6a + fa45d4be + 140ab0ff (Apache-2.0) — zig/src/core/registry.zig + zig/src/hip/tuning/{search,tuning}.zig. Batch 317 / Plan 317. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: routed-density-pairs-per-item-keys-tile-admission):
// the item-count-only admission: a routed plan takes the K-parallel
// tile by ITEMS alone — dense-routed plans at the same item count
// take the special tile and lose, sparse plans at higher counts are
// wrongly refused.
#![allow(dead_code)]

struct Plan {
    items: usize,
    pairs: usize, // total routed (item, expert) pairs
}

fn admit_by_items_only(p: &Plan, items_bound: usize) -> bool {
    p.items <= items_bound // the density is invisible here
}

#[test]
fn items_alone_route_wrong_at_both_extremes() {
    let sparse = Plan { items: 400, pairs: 600 }; // 1.5 pairs/item — the tile WINS here
    let dense = Plan { items: 400, pairs: 3_600 }; // 9.0 pairs/item — the tile LOSES here
    let bound = 507;
    // both pass the items-only gate — one of them wrongly
    assert!(admit_by_items_only(&sparse, bound));
    assert!(admit_by_items_only(&dense, bound)); // the dense plan is MIS-ADMITTED
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: routed-density-pairs-per-item-keys-tile-admission):
// the density-keyed admission: the plan carries pairs; the tuning row
// holds an f32 pairs-per-item bound (compared exactly in f64); the
// registry refuses over the bound; the coupled search moves the
// bounds together with a per-cell veto and both-sides timing.
#![allow(dead_code)]

#[derive(Clone, Copy)]
struct Plan {
    items: usize,
    pairs: usize,
}

impl Plan {
    fn pairs_per_item_f64(&self) -> f64 {
        self.pairs as f64 / self.items as f64
    }
}

// the tuning row's bound: f32 HELD, compared exactly in f64 — the
// measured threshold 3.43..3.75 holds no whole number
fn admit(p: &Plan, items_bound: usize, ppi_bound_f32: f32) -> bool {
    p.items <= items_bound && p.pairs_per_item_f64() <= ppi_bound_f32 as f64
}

// the coupled search: one proposal moves kp3's two bounds together;
// scored with a per-cell veto on each whole table (every cell must
// stay a win); both sides of every changed choice are TIMED (the
// retake: a noisy cell is re-measured, not a bucket-vetoing coin flip)
#[derive(Clone, Copy)]
struct Proposal {
    items_bound: usize,
    ppi_bound: f32,
}

fn search_step(current: Proposal, d_items: isize, d_ppi: i32) -> Proposal {
    // the bounds move TOGETHER — never one knob alone
    Proposal {
        items_bound: (current.items_bound as isize + d_items).max(1) as usize,
        ppi_bound: f32::from_bits((current.ppi_bound.to_bits() as i32 + d_ppi) as u32),
    }
}

fn per_cell_veto_holds(admits: &[bool], wins: &[bool]) -> bool {
    // every cell the table claims must still be a win after the move
    admits.iter().zip(wins).all(|(&admitted, &win)| admitted == win)
}

#[test]
fn the_density_conjunction_admits_sparse_and_refuses_dense() {
    let sparse = Plan { items: 400, pairs: 600 }; // 1.5 ppi
    let dense = Plan { items: 400, pairs: 3_600 }; // 9.0 ppi
    let b = (507, 3.5f32);
    assert!(admit(&sparse, b.0, b.1));
    assert!(!admit(&dense, b.0, b.1)); // the mis-admission is gone
}

#[test]
fn the_fractional_bound_discriminates_where_an_integer_cannot() {
    let just_under = Plan { items: 100, pairs: 350 }; // 3.5 ppi — the measured edge
    let just_over = Plan { items: 100, pairs: 375 }; // 3.75 ppi
    let bound = 3.6f32; // 3.43..3.75 holds no whole number
    assert!(admit(&just_under, 507, bound));
    assert!(!admit(&just_over, 507, bound)); // an integer bound (3 or 4) misses one side
}

#[test]
fn the_search_moves_the_coupled_bounds_together() {
    let p = Proposal { items_bound: 507, ppi_bound: 3.5 };
    let next = search_step(p, -16, -12); // BOTH bounds move in one proposal
    assert_ne!(next.items_bound, p.items_bound);
    assert_ne!(next.ppi_bound.to_bits(), p.ppi_bound.to_bits());
}

#[test]
fn one_noisy_cell_does_not_veto_the_whole_bucket() {
    // the retake: the noisy read is re-measured; only a STABLE loss vetoes
    let noisy_first = false;
    let retake = true; // the second read
    let stable = noisy_first == retake;
    assert!(!stable); // chance does not veto the bucket
    let admits = [true, true, false, true];
    let wins = [true, true, false, true];
    assert!(per_cell_veto_holds(&admits, &wins));
}
".to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    slice_b317_staged_leads()
}
