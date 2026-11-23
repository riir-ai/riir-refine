//! Batch 402 — the mlxfast-bonsai2-27b-engine 113-commit delta residue
//! (Plan 402 / Research 437): Layr-Labs/mlxfast-bonsai2-27b-engine,
//! delta `cfb35b0d`→`0c7c3923` (113 commits, 85 Accept + 24 Validate
//! submissions + 4 operator/reorg, window 09-25→10-02, the arena's close
//! phase + the arena reorg), MIT.
//!
//! PRIOR-COVERAGE CORRECTION (this batch's controlling finding): the pin
//! `0c7c392` surface was already PROGRESSIVELY MINED during October by the
//! staged-lead batches — `entries_gpu_51/60/63/70/71/72` cite this exact pin
//! (the io32/lazy-admission/accept-packet/continuation-scan/pad-ladder
//! waves), and `rust_perf` `entries_10` cites it too. Batch 401's slice-(1)
//! delta check counted commits/files against the B181 pin and missed that
//! intermediate batches had consumed the close-phase surface as it landed.
//! This batch lands the UN-OWNED RESIDUE only — two kernel rules whose
//! mechanisms zero owners hold after 3+-vocabulary greps + in-row reads —
//! plus the correction row in the snapshot; the remaining staged
//! candidates (8) are recorded in Research 437 for the next windows.
//!
//! - `verify-window-weight-load-amortization-exact` — the W4 verify-window
//!   load-sharing law: positions share the weight LOAD, never the
//!   accumulator, so every verify column stays bit-identical to the serial
//!   row while the weight bytes are read once for all k.
//! - `splitk-reduce-consumed-in-place-by-consumer-kernel` — the split-K
//!   reduce with exactly one narrow consumer folds INTO that consumer's
//!   reads (chunk order pinned); the reduce launch disappears
//!   bit-identically, gated per geometry at load.
//!
//! Faces (count-neutral, same commit): `gpu_14` += the arena's vendored NAX
//! dsplit instance (steel_attention_nax.h:750-758 @ 0c7c3923 + the
//! naxAvailable capability probe); ane += the capability-nil fail-safe
//! twin face on the procedure-bank rule.
//!
//! Fixtures: `corpus/fixtures/b402_fixtures.rs` — standalone-compiled AND
//! executed BEFORE entry (debug AND -O), the four-arm discipline.
//! NOTHING INVENTED — every quote read at the clone this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 402 — the mlxfast-27b delta residue's `kernel_opt` half.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the W4 verify-window load-sharing lead (A−): k speculative
        // positions share ONE weight read; each keeps its own accumulator in
        // the stock one-row order, so every column is bit-identical to the
        // serial row. The exactness half is load-bearing, not the speed. ──
        KernelRule {
            rule_id: "verify-window-weight-load-amortization-exact".to_string(),
            gpu_concept: "MECHANISM (verify-window weight-load amortization with per-position accumulators; category B — layout/load-sharing, the exactness-critical amortization axis): a short SPECULATIVE VERIFY WINDOW scores k draft positions against the same quantized weight matrix; the amortizable cost is the weight LOAD, never the reduction — so one codepath reads each packed weight value ONCE for all k positions while every position keeps its own accumulator advanced in the stock single-row reduction ORDER, making every verify column BIT-IDENTICAL to running the row alone. The source's header is the whole law, verbatim (Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/Qwen35A3BTargetVerify.swift:7-10 @ 0c7c3923): 'Exact M1 arithmetic over a short verify window without rereading each W4 matrix once per position. One SIMD lane owns the same quantized values and reduction order as the stock one-row QMV; the verify positions share the weight load but retain independent accumulators.' THE OWNERSHIP DISCIPLINE is the mechanism: 'one SIMD lane owns the same quantized values' — the lane that touches a weight byte owns its fan-out to all k accumulator sets, so the load happens once and the ORDER of additions per position is the serial row's order exactly (no cross-position accumulation, no reassociation, no shared partial). THE RUST-SHAPED FIX SPACE (wgpu/compute-shape scoring of k columns against one packed quantized weight — speculative verify, batched per-row scoring, any k-abIt weight-resident pass): loop the k columns INSIDE the weight-resident pass — each (row, lane) reads the packed weight bytes once and fans them to k per-position accumulator sets, each following the single-row pass's summation order exactly. NEVER: a shared or reassociated accumulator across columns (changes bits — the lossy-surface law's exactness leg); a per-column weight reread (the cost being amortized — the whole point); a materialized wide dequant copy (a different rule's gate — the dequant-materialization family owns that axis); skipping the exactness argument because 'verify is approximate' (greedy parity is the contract — non-exact verify breaks the accept-walk). THE ADMISSION SHAPE: the column-invariance claim is proven at load/admission (per-instance coverage over the enumerable kernel variants), never assumed at the call site — the same source folds a self-test discipline around every such variant (the split-K fold rule's 'checked at load (prepare) on the layer's geometry for every kernel variant the verify can take, with the reduce launch kept on any mismatch' is the sibling admission). ⛔ KINSHIP (adjudicated both directions): whole-matrix-unpack-pass-rows-per-tile-gate (gpu_148) amortizes a MATERIALIZED unpack pass across rows under a perf gate (R//E >= 256) — no materialization here, in-register load sharing, and exactness (not a threshold) is the load-bearing half — compose on a packed-verify lane; verify-row-tiles-share-loaded-weight-panel (gpu_04, Batch 257) + expert-prompt-row-batch-amortizes-weight-pool-stream (Batch 256) own the LOAD-SHARING MECHANISM axis in this category (the weight panel loaded once and tiled across verify rows / the weight-stream amortization over row batch) — THIS adds the EXACTNESS CONTRACT neither carries: per-position accumulators in the stock single-row REDUCTION ORDER (bit-identical columns), the never-shared/reassociated-accumulator refusal, and the k-column loop INSIDE the weight-resident pass (vs the panel's row tiling) — the family's correctness half; dual-accumulator-single-reduction-sweep (gpu_02) fuses two DIFFERENT reductions over the same data — THIS replicates the SAME reduction across verify columns; independent-accumulator-chains-hide-load-latency (gpu_21) REASSOCIATES one reduction's accumulators under a tolerance — opposite polarity: here per-position accumulators stay stock-serial and bit-exact — the two rules are the tolerance-vs-exactness fork of the same latency-hiding instinct, cite both; TK-batched-prefetch (gpu_75) amortizes loads TEMPORALLY (prefetch ahead of one stream) — HERE spatially across columns of one pass; verify-parity-position-pure-grid (gpu_152) guarantees the GRID cannot perturb a verify column — THIS guarantees the MEMORY PATH cannot either — compose (grid purity + load purity = a verify row comparable end to end); mxu-op-exactness-scoping (gpu_161) scopes exact wide-op admission — the admission-discipline kin. REGIME: speculative-decode verify windows scoring k drafts against quantized weights; batched per-row scoring passes; any weight-resident kernel multiplying one packed operand into k independent outputs where bit-identity per output is the contract. ANTI-REGIME: cross-position state that legitimately accumulates (the accepted prefix's KV — shared by contract, not by amortization); lossy verify modes with their own divergence budget (the source's non-greedy rows never speculate — the greedy-only losslessness invariant); k=1 (no window, the serial row IS the law). NotBenchable here (the win metric lives in a spec-decode verify window over W4 weights — no such serving lane in this corpus's validator; the exactness contract, not a cycle count, is the decision content; the Batch-136 annotation class). The fixture models the law deterministically: the buggy arm rereads the weight per column (slow) AND shares one accumulator across columns (wrong bits — red), the fixed arm shares the load and keeps per-position serial-order accumulators (bit-identical columns — green), the tampered arm claims exactness with a reassociated order — red, the admission arm refuses an uncovered variant — green refusal. Source: Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923 (`qwen35_a3b_exact_w4_g64_verify_narrow2` kernel, Qwen35A3BTargetVerify.swift:7-33 header + the kernel constants PACK_FACTOR 8 / VALUES_PER_THREAD 16 / BLOCK_SIZE 512). MIT. Batch 402 / Plan 402 / Research 437."
                .to_string(),
            ref_shape: r#"// NAIVE (both defects): the verify pass scores each of the k draft
// positions against the packed W4 weights independently — the weight
// bytes are re-read once per position (the cost the window exists to
// amortize), and a tempting "shortcut" shares ONE accumulator across
// positions (wrong: cross-position accumulation reassociates the sum,
// so verify columns stop matching the serial row and the accept-walk
// walks off parity).
struct VerifyPass<'w> {
    w_packed: &'w [u8], // K/2 bytes per row, 8 values per byte
}

impl<'w> VerifyPass<'w> {
    /// per-column loop OUTSIDE the weight pass: k full weight reads
    fn score_column(&self, row: usize, x: &[f32], k_of: usize) -> f32 {
        let mut acc = 0.0f32;
        for byte in 0..self.w_packed.len() / 1 {
            let w0 = dequant_nibble(self.w_packed[byte], false);
            let w1 = dequant_nibble(self.w_packed[byte], true);
            acc += w0 * x[column_x(k_of, byte)] + w1 * x[column_x(k_of, byte + 1)];
        }
        acc
    }

    /// the wrong amortization: one accumulator shared by all k columns
    fn score_window_shared_acc(&self, row: usize, xs: &[&[f32]]) -> Vec<f32> {
        let mut shared = 0.0f32;
        let mut out = vec![0.0; xs.len()];
        for (k_of, x) in xs.iter().enumerate() {
            shared += self.score_column(row, x, k_of); // cross-position sum
            out[k_of] = shared; // column k carries columns 0..k — NOT the row's score
        }
        out
    }
}

fn dequant_nibble(_byte: u8, _hi: bool) -> f32 {
    0.0 // fixture stand-in
}

fn column_x(_k_of: usize, _idx: usize) -> usize {
    0
}"#.to_string(),
            opt_pattern: r"// FIXED: the k columns are looped INSIDE the weight-resident pass;
// the lane reads each packed byte ONCE and fans it to k independent
// accumulators, each advanced in the serial row's order — every
// column is bit-identical to scoring the row alone.
struct VerifyPassExact<'w> {
    w_packed: &'w [u8],
}

impl<'w> VerifyPassExact<'w> {
    /// one weight read, k independent serial-order accumulators
    fn score_window(&self, xs: &[&[f32]]) -> Vec<f32> {
        let k = xs.len();
        let mut acc = vec![0.0f32; k]; // per-position, never shared
        for byte in 0..self.w_packed.len() {
            let w0 = dequant_nibble(self.w_packed[byte], false);
            let w1 = dequant_nibble(self.w_packed[byte], true);
            // the weight bytes are touched ONCE for the whole window;
            // each column's accumulator adds in the same order the
            // single-row pass would have
            for (k_of, x) in xs.iter().enumerate() {
                acc[k_of] += w0 * x[2 * byte] + w1 * x[2 * byte + 1];
            }
        }
        acc
    }
}

// ADMISSION: the column-invariance claim is proven per variant at
// load, never assumed — an uncovered variant keeps the per-column
// serial path.
struct VerifyAdmission {
    covered_variants: std::collections::BTreeSet<&'static str>,
}

impl VerifyAdmission {
    fn admits(&self, variant: &str) -> bool {
        self.covered_variants.contains(variant)
    }
}".to_string(),
        },
        // ── R2 — the split-K reduce fold (A−): exactly one narrow consumer
        // + a tape keeper → the consumer sums the chunk partials itself in
        // the reduce kernel's own order; the reduce launch disappears
        // bit-identically, load-gated per geometry. ──────────────────────
        KernelRule {
            rule_id: "splitk-reduce-consumed-in-place-by-consumer-kernel".to_string(),
            gpu_concept: "MECHANISM (split-K reduce folded into its single narrow consumer; category A — dispatch/launch elimination, the reduce-execution-site axis): a split-K product ships as TWO kernels — the partial kernel (K in fixed-width chunks) and a reduce kernel that adds each output's chunk partials in chunk order — but when the reduce has EXACTLY ONE narrow consumer (reads one value per output element) plus a tape keeper, the reduce kernel is not collapsed or batched: it is ELIMINATED — the consumer's own threads read the chunk partials and add them in the reduce kernel's own order, then write the sums in the shape the tape expects, so the reduce launch disappears from the round entirely, bit-identically. The source's block is the whole law, verbatim (Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/PrismHadamardQwen35.swift:196-213 @ 0c7c3923): 'The verify-width b | a split-K (Qwen35SmallNMatmul) is two launches per GDN layer: the partial kernel (K in 128-wide chunks) and a reduce that adds the 40 chunk partials of each of the 16 x 96 outputs in chunk order. The only consumer of that product in a verify round is the GDN prework launch (Qwen35GDNPrework), which reads one a and one b value per (row, value head) to form the gates, and the replay tape, which keeps the a and b rows. Here the prework's gate threads read the chunk partials themselves and add them in the reduce kernel's order (from 0.0f, chunk 0 first), then write the sums out as the a and b arrays the tape keeps. The reduce launch disappears from the round (48 launches, each a small grid with its own ramp and drain, behind a dependency barrier), the partial kernel is unchanged, and every value is the same bit: the same FP32 additions in the same order, checked at load (prepare) on the layer's geometry for every kernel variant the verify can take, with the reduce launch kept on any mismatch.' THE THREE ADMISSIBILITY CONDITIONS are the rule (all must hold): (1) the reduce has EXACTLY ONE narrow consumer in the round (one value per output element — a gate projection, a router score, a scalar readout) beside a tape keeper that only STORES the rows (a keeper that also computes on partials is a second consumer — refuse); (2) the fold reproduces the reduce's ORDER exactly (from 0.0, chunk 0 first — the pinned fold order is what makes it bit-identical, the Issue-940 discipline); (3) the fold is gated per geometry per variant at load with the reduce kept on any mismatch (fail-open to the incumbent launch — the demote-switch law's shape, kill-switch DARKBLOOM_QWEN35_SPLITK_BA_FOLD=0). THE SCHEDULING RIDER is in-source too (PrismHadamardQwen35.swift:53-55): 'The reduce takes the qkv|z product as an unread input (after), so MLX encodes it after that product and the partial runs beside the product instead of alone. DARKBLOOM_QWEN35_SPLITK_BA_OVERLAP=0 drops it.' — an unread-input dependency edge re-orders the partial kernel BESIDE its producer instead of after it (overlap without touching the arithmetic); when the reduce is folded away the same edge trick keeps the consumer launch beside the partials' producer. THE WIN IS LAUNCH-COUNT AND DRAIN: 48 launches per verify window, 'each a small grid with its own ramp and drain, behind a dependency barrier' — small-grid ramp/drain cost is the measured mechanism, not FLOPs. ⛔ KINSHIP (adjudicated both directions): splitk-reduce-one-launch-slice-order-pinned (gpu_67) owns collapsing a reduce that STILL RUNS (31 elementwise launches -> one kernel, slice order pinned) — THIS eliminates the reduce launch entirely, the consumer does the summing; the order discipline ('from 0.0f, chunk 0 first' vs 'slice order pinned') is shared kinship, cite never merge; splitk-tier-ladder-per-tier-ceiling-min-partition-inputs (gpu_117) owns split DEPTH selection — THIS owns the reduce's EXECUTION SITE; producer-folds-single-consumer-family (gpu_72) folds a PRODUCER into one launch (its SwiGLU fold measured NEGATIVE — the honest kinship note) — THIS folds a REDUCE into a CONSUMER, the mirror direction, and the only-one-narrow-consumer admission is what makes it safe where the producer fold was not; command-buffer-declared-input-pacing (gpu_152) owns the commit-watermark pacing axis — different lifecycle; the Issue-940 pinned-fold-order discipline is the correctness substrate both reduce rules stand on. REGIME: split-K / chunked-partial products whose consumer reads one scalar per output (GDN gate projections, MoE router scores, normalization statistics feeding one stage) with a replay/capture tape that needs the sums in a known shape; any two-launch (partial + reduce) pipeline whose reduce output is consumed exactly once per element. ANTI-REGIME: multiple or wide consumers of the reduce output (a second reader would re-sum the partials per reader — the reduce kernel IS the sharing); consumers needing the sums under a DIFFERENT association (the fold must reproduce the reduce's order, not a better one); dynamically-chosen consumers (the admission is per geometry per variant at load — a consumer picked at runtime has no load-time gate); reductions whose order is genuinely free because nothing replays or compares them (no tape, no parity contract — then a plain fused reduction rule owns the axis). NotBenchable here (the win is launch-count + drain inside a spec-decode verify round over a vendor kernel family; the bit-identity + admission contract is the decision content; the Batch-136 annotation class). The fixture models the law deterministically: the two-launch incumbent (partial + reduce), the fold arm (consumer sums partials in chunk order, bit-identical sums, one launch gone), the violated-admission arm (a SECOND consumer appears -> the fold refuses and the reduce launch stays — green refusal), the order-tamper arm (fold sums in reverse chunk order — bits differ, red — proving the pinned order is load-bearing). Source: Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923 — PrismHadamardQwen35.swift:196-226 (the fold block + the DARKBLOOM_QWEN35_SPLITK_BA_FOLD=0 kill switch), :53-55 (the unread-input-after overlap rider + DARKBLOOM_QWEN35_SPLITK_BA_OVERLAP=0). MIT. Batch 402 / Plan 402 / Research 437."
                .to_string(),
            ref_shape: r"// NAIVE: the split-K product always ships two launches — the partial
// kernel and a reduce kernel — even when exactly ONE narrow consumer
// reads the sums; the consumer then reads the reduce's output, paying
// a whole extra small-grid launch (ramp + drain + dependency barrier)
// per layer for values it could have summed itself in the same order.
const CHUNKS: usize = 40;

fn splitk_partial(_k_chunk: usize, _out: &mut [Vec<f32>]) {
    // the partial kernel: chunk partials per output (unchanged below)
}

/// the reduce kernel: a separate launch, small grid, own ramp/drain
fn splitk_reduce(partials: &[Vec<f32>], out: &mut [f32]) {
    for (i, slot) in out.iter_mut().enumerate() {
        let mut acc = 0.0f32;
        for chunk in 0..CHUNKS {
            acc += partials[chunk][i]; // chunk order lives HERE,
        }
        *slot = acc; // in a launch the consumer must wait for
    }
}

/// the consumer: reads the reduce OUTPUT (one value per element),
/// behind the reduce launch's dependency barrier
fn gdn_prework(a: &[f32], b: &[f32]) {
    let gates: Vec<f32> = a.iter().zip(b).map(|(a, b)| a * sigmoid(b)).collect();
    let _ = gates;
}

fn sigmoid(_x: f32) -> f32 {
    0.0
}".to_string(),
            opt_pattern: r"// FIXED: the fold. The consumer's threads read the chunk partials
// themselves and add them IN THE REDUCE'S ORDER (from 0.0, chunk 0
// first), write the sums in the tape's shape, and the reduce launch
// disappears — bit-identical, gated per geometry at load.
struct SplitkFold<'p> {
    partials: &'p [Vec<f32>], // CHUNKS rows of per-output partials
}

impl<'p> SplitkFold<'p> {
    /// the consumer sums IN PLACE, in the reduce kernel's own order
    fn consume_folded(&self, tape: &mut [f32]) -> Vec<f32> {
        let n = self.partials[0].len();
        let mut sums = vec![0.0f32; n];
        for (i, slot) in sums.iter_mut().enumerate() {
            let mut acc = 0.0f32;
            for chunk in 0..self.partials.len() {
                acc += self.partials[chunk][i]; // 0.0 start, chunk 0 first
            }
            *slot = acc; // the same bit, no reduce launch
        }
        tape.copy_from_slice(&sums); // the tape keeps the rows as before
        sums
    }
}

/// ADMISSION: exactly one narrow consumer + a storing tape; a second
/// consumer (or a computing keeper) refuses the fold and the reduce
/// launch stays.
struct FoldAdmission {
    narrow_consumers: usize,
    keeper_only_stores: bool,
}

impl FoldAdmission {
    fn admits(&self) -> bool {
        self.narrow_consumers == 1 && self.keeper_only_stores
    }
}

fn gdn_prework_folded(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(a, b)| a * sigmoid(b)).collect()
}".to_string(),
        },
    ]
}
