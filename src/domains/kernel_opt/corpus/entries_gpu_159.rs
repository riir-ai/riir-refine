//! Batch 379 — the maya-inkling-mini `kernel_opt` transcription (Plan 395 /
//! Research 429): three decision-content laws from
//! alesha-pro/maya-inkling-mini's `src/ik2/` engine — the decode-attention
//! register-direct admission (the reuse lives in the mma fragment's own
//! column geometry; the smem-tiled flash way pays occupancy and a narrow-
//! cache materialization tax), the FP32-input hi+lo limb pair riding two
//! idle MMA B-operand columns (one-sided precision recovery at zero extra
//! mma), and the bytes-in-flight concurrency law (split the rows across
//! warps before widening any request; at the memory ceiling neither lever
//! pays). All suggestion-only NotBenchable-by-domain — every rule's win
//! metric lives on an `sm_86` CUDA host this corpus does not run (ours are
//! Metal + `sm_89`; the league models are dense ternary / dense `qwen3.8`, no
//! looped `MoE`, no `FP4`-class weights — the Research-429 GOAT section; the
//! Batch-136 / B333 precedent), `bench_spec: None`, first-class decision
//! content with mechanism-level magnitudes (⅓-speed smem FP8 vs
//! memory-speed register FP8; 100→745 GB/s; 846/776 GB/s register-direct).
//!
//! Source: `alesha-pro/maya-inkling-mini` @
//! `be849daa81790b7d12b74ba9e2f7b78ed4ef43cd` (MIT; single pinned commit at
//! the verdict — T0 this batch re-cloned at the pin, UPSTREAM MOVED +3
//! commits past it (c520fd14 specdec-drafter tip, fc911481 dual-GPU EP,
//! 82de2288 `xq16_sup`) with every mined quote site byte-exact at BOTH pin
//! and tip — recorded, no re-pin; the moved surface is future re-distill
//! lane-intel). Per the transcription law the corpus consumes DECISION
//! CONTENT with ORIGINAL Rust fixtures (`corpus/fixtures/b379_fixtures.rs`,
//! standalone-compiled AND executed before entry — 17/17, zero warnings,
//! debug AND -O; the CUDA/MMA surface is modeled as a CPU mock, the fixture
//! never calls a GPU), never source verbatim. Faces landed count-neutral on
//! their owners: F1 on `lop3-int4-to-half2-dequant` (the signed-E2M1
//! `byte_perm` LUT + the `sm_86`-gated arm), F2 on
//! `kv-dequant-materialization-batch-threshold-gate` (the read-quantized
//! arm's decode-attention instance), F4 on
//! `token-local-only-approximate-accelerator` (the third lossy pathway:
//! discrete routing amplification). F3 (the graph-family positive-instance
//! cluster) is a RECORD, never a landing — it rides Research 429 §Lane-intel
//! and the queue snapshot line. The UNREAD residue closed this batch: the
//! `experts_mma_kernel` body (R2's operand layout confirmed: the g == 1
//! lane group reads the lo buffer as its B columns), the `ik2_main.cu`
//! graph-capture body (capture per `OutMode` × tier, replace after a stream
//! sync), and the prefill flash path (smem tiles + a Tq-row grid — R1's
//! anti-regime evidence). Batch 379 / Plan 395 / Research 429.

use crate::domains::kernel_opt::corpus::KernelRule;

// The corpus entry IS the payload (data, not logic) — the kernel build fn
// convention (the entries_popcorn precedent).
#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the batch lead: decode-attention reads each cache row
        // straight into the registers and lets the mma fragment consume it
        // for every query head of the K/V head group — no smem tile, no
        // barriers; the admission is reuse LOCATION + occupancy + the
        // narrow-cache materialization tax, never reuse count. ────────────
        KernelRule {
            rule_id: "decode-attn-cache-rows-register-direct-occupancy-not-smem-tile".to_string(),
            gpu_concept: "MECHANISM (decode attention reads the cache rows register-direct when the per-row consumers live inside the mma fragment's own column geometry; category A — memory access): a decode-attention block whose warps share one K / V head (GQA: R = H / Hkv query heads per cache head, R <= 8) stages NOTHING — each warp takes a run of its head's keys, loads each key's K / V row ONCE into registers, and the R consumers are the mma's own N columns (C [16 keys][8 heads] += A [16 keys][16 dims] · B [16 dims][8 heads]: A is the K row as it lies in the cache, B is the heads' q — one load serves the whole fragment width). The kernel's own statement, verbatim (ik2_kernels.cu:1352-1356 at be849daa): 'A block's warps share a K / V head; a warp takes a run of its keys, 16 a step, for the R = H / Hkv <= 8 heads of' + 'that K / V head - a key's K and V rows are read once for them all'. THE ADMISSION IS NOT REUSE ARITHMETIC — the rows DO have R consumers; it is three other legs: (a) reuse LOCATION: the consumers sit inside the fragment's column geometry, so a shared-memory tile adds no reuse the fragment lacks; (b) OCCUPANCY, verbatim (:1352-1354): 'A warp at a time and straight from the cache into the registers: no shared memory, so a dozen and more warps an' + 'SM run it, and a warp's wait for its rows is another warp's arithmetic - the kernel reads at the memory's speed' — register-direct keeps a dozen warps resident per SM where the flash way keeps 'tiles in shared memory, one block of four warps an SM, one of them computing'; (c) the NARROW-CACHE MATERIALIZATION TAX, verbatim (:1354-1356): 'an FP8 cache too (its bytes made FP16 in the registers).  (attn_flash's way - tiles in shared memory, one block of four warps an SM, one of them computing - read an FP16 cache at the memory's speed and an FP8 one at a third' + 'of it.)' — the smem form converts every staged element into an FP16 tile (its working format), the register form makes the FP8 bytes FP16 per-lane at the load and never materializes a working copy; the measured pair on their 3090 (provenance, never a bar): 846 GB/s FP16 / 776 GB/s FP8 register-direct, the smem FP8 path at a third of memory speed. THE TWO-REGIME IS KEYED ON QUERY ROWS: prefill has Tq query rows per block — the K / V tiles then have real cross-query reuse and the smem flash kernel keeps them (attn_flash_f16_kernel: cp_async16 tiles, the kFaSmem dynamic-smem attribute, the grid over Tq-row blocks — read in full this batch, the T0 residue) — decode has one query row group and the tiles would stage rows the fragment already consumes internally. The online softmax runs in the log2 domain with per-warp (max, sum) partials and a combine kernel merging warps + sink + the layer gate (one launch less). DECISION CONTENT: (a) the admission reads reuse LOCATION + occupancy + the cache-format materialization tax — never reuse COUNT alone (R > 1 still stages nothing when the reuse is fragment-internal); (b) the two-regime keyed on query rows: Tq == 1 decode register-direct, Tq > 1 prefill tiles; (c) rows shared ACROSS warps is the tile pole (a tile adds reuse only cross-warp); (d) the narrow-cache decode reads quantized rows and converts per-lane — the materialized working copy is the smem path's tax, not a prerequisite. CLASS SIGNATURE: a decode-attention whose cache read is the wall, with per-row consumers inside the mma fragment and a narrow cache format. REGIME: decode attention over a grouped-query KV cache (R <= 8 fragment-consumed heads), long contexts where the cache read binds, FP8 / FP16 cache formats. ANTI-REGIME: prefill / verify windows (Tq > 1 — the tiles win the census and the admission picks them); rows shared across warps (a smem tile adds real reuse); head dims so wide the register fragment spills; consumers that need the decoded cache materialized for several passes. ⛔ KINSHIP (adjudicated both directions): `j16-warp-fragment-direct-no-smem-tile` (gpu_106) is the nearest kin and the adjudication the verdict front-loaded — THAT owns the reuse-ARITHMETIC pole: at J = 16 activation columns the weight bytes have NO cross-column reuse an smem tile can add, so stage nothing; THIS owns the R > 1 case where the reuse EXISTS but is consumed inside the fragment — its admission names occupancy and the materialization tax, and its contrast kernel (flash) IS a smem stager, so the rule carries a two-regime boundary gpu_106 does not (compose when both apply: fragment-internal reuse + R = 1 is the shared no-stage answer); `gqa-kv-head-sharing-bandwidth-reduction` (gpu_01) owns the DATA-LAYOUT choice (share K / V heads across query heads, the R-way byte reduction) — THIS owns the kernel STRUCTURE given that layout (register-direct vs tiles), they compose; `fold-query-tokens-for-occupancy` (gpu_01) and `occupancy-keyed-value-split-register-budget-ladder` (gpu_94) own occupancy via WORK-SHAPE (fold query tokens into workgroups / split values to fill SMs) — THIS buys occupancy by DROPPING staging (smem out = more resident warps), a different lever on the same counter; `kv-dequant-materialization-batch-threshold-gate` (gpu_34) owns WHEN a quantized cache materializes vs reads quantized (the chunk-threshold sweep) — THIS is the read-quantized arm's decode-attention instance (the F2 face rides it); `cpasync-multistage-tile-pipeline` (gpu_09) and `one-barrier-a-tile-prefetch-occupancy-pair` (gpu_102) own the tile pole's HOW — the prefill side of this rule's two-regime. GREP EVIDENCE (re-run at the landing HEAD): 'no.smem|register.*direct|decode.*attn' over the corpus -> gpu_106 (adjudicated above) ONLY; 'gqa|kv.*head.*shar' -> gpu_01 (compose); 'flash' -> the tile-pole families; zero other owner of the occupancy + materialization admission. NotBenchable-here (the entries_gpu_65 / dial9 / Batch-136 class — the law's home is an `sm_86` CUDA decode path; a Metal/CPU-shaped sibling would fork the law across domains). The measurement instrument is the CPU fixture: the read census (each row read once whatever R is; the tile form wins the census at Tq > 1 and the admission picks it; the cross-warp arm flips to tiles) + the materialization census (the register form converts per consumed fragment, never a full staged tile). Source: alesha-pro/maya-inkling-mini @ be849daa (MIT) — src/ik2/ik2_kernels.cu:1349-1375 (the kernel comment block: the register-direct statement, the occupancy statement, the FP8-in-registers parenthetical with the flash contrast, the K / V head sharing + the two mma layouts) + attn_decode_mma_combine + the flash contrast path attn_flash_f16_kernel:1084-1110 with attn_flash_launch:4504-4516 (the smem tile + the Tq-row grid) + docs/INKLING2-MINI.md:121 (846 / 776 GB/s provenance). Batch 379 / Plan 395 / Research 429."
                .to_string(),
            ref_shape: r"// ref_shape: the decode-attention stages every cache row through a
// shared-memory FP16 tile — one block of four warps per SM, one of them
// computing — and pays the narrow-cache materialization tax on every
// staged element, whatever the per-row consumers are.

const CACHE_ROWS: usize = 512;
const WIDTH: usize = 128;
const R_HEADS: usize = 8; // query heads sharing this cache head
const TILE_PAD_ROWS: usize = 16;

struct SmemTile {
    staged: Vec<u16>, // the FP16 working copy of the tile
}

/// the flash-way staging: EVERY row materialized to the tile format.
fn stage_tile(cache: &[f32], narrow: bool) -> SmemTile {
    let rows = CACHE_ROWS + TILE_PAD_ROWS; // window padding rides the tile
    let mut staged = vec![0u16; rows * WIDTH];
    for (i, slot) in staged.iter_mut().enumerate() {
        let v = cache[i % CACHE_ROWS];
        // narrow (FP8-class) caches are made FP16 IN SHARED MEMORY — the
        // materialization the register path never pays
        *slot = f16_round(v, narrow);
    }
    SmemTile { staged }
}

/// every query head re-reads the staged tile: R reads per row at decode,
/// and the census never improves with head grouping because the tile owns
/// the working copy.
fn per_head_reads_from_tile() -> usize {
    CACHE_ROWS * R_HEADS
}

fn f16_round(v: f32, _narrow: bool) -> u16 {
    v.to_bits() as u16 // the materialized working copy (shape only)
}

// the naive admission: stage whenever rows have > 1 consumer — R = 8
// fires it at decode, paying occupancy (4 warps/SM) + the full-tile
// conversion for a reuse the mma fragment would have consumed anyway.
let _tile = stage_tile(&vec![1.0f32; CACHE_ROWS * WIDTH], true);
let reads = per_head_reads_from_tile();
let _ = (reads, f16_round);
".to_string(),
            opt_pattern: r"// opt_pattern: decode reads each cache row ONCE into registers — the
// R consumers are the mma fragment's own columns — and the admission
// keys on reuse LOCATION + query rows + the materialization tax, never
// on the consumer count. Tq > 1 (prefill) or cross-warp shared rows
// flip to the staged form; narrow caches convert per-lane at the load.

const CACHE_ROWS: usize = 512;
const WIDTH: usize = 128;
const R_HEADS: usize = 8;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum AttnForm {
    RegisterDirect, // decode: the fragment consumes the rows internally
    SmemTiles,      // prefill / cross-warp reuse: stage what many consume
}

fn attn_form(r_heads: usize, query_rows: usize, rows_shared_across_warps: bool) -> AttnForm {
    if query_rows > 1 || rows_shared_across_warps {
        return AttnForm::SmemTiles;
    }
    let _ = r_heads; // consumed inside the fragment's columns either way
    AttnForm::RegisterDirect
}

/// the register-direct census: one read per cache row, whatever R is —
/// the reuse lives in the fragment geometry, not in a staging buffer.
fn register_direct_row_reads(n_cache_rows: usize, r_heads: usize) -> usize {
    let _ = r_heads;
    n_cache_rows
}

/// the narrow-cache tax census: the register form converts per consumed
/// element; it never materializes the padded tile the smem form stages.
fn register_conversions(consumed_elements: usize) -> usize {
    consumed_elements
}

fn smem_conversions(rows: usize, width: usize, pad_rows: usize) -> usize {
    (rows + pad_rows) * width // the full staged tile, padding included
}

// the admission at decode: R = 8 fragment-internal consumers, one query
// row group, no cross-warp sharing — register-direct, a dozen warps an
// SM, the FP8 bytes made FP16 in the registers at memory speed.
let form = attn_form(R_HEADS, 1, false);
assert_eq!(form, AttnForm::RegisterDirect);
assert_eq!(register_direct_row_reads(CACHE_ROWS, R_HEADS), CACHE_ROWS);
// the census arms the prefill anti-regime lives on: Tq > 1 flips to
// tiles, where the staged tile's single read beats per-query re-reads.
assert_eq!(attn_form(R_HEADS, 16, false), AttnForm::SmemTiles);
let _ = (register_conversions, smem_conversions);
".to_string(),
        },
        // ── R2 — the FP32 input rides two idle MMA B columns as a Dekker
        // hi+lo limb pair; the row's two columns sum in the FP32
        // accumulator — FP32-input precision at ZERO extra mma. ───────────
        KernelRule {
            rule_id: "fp32-input-hi-lo-two-fp16-mma-columns".to_string(),
            gpu_concept: "MECHANISM (an FP32 operand crosses an FP16-input MMA as a Dekker hi+lo limb pair riding two of the B operand's columns; category N — dtype/precision): the FP32 activation is split hi = f16(x), lo = f16(x − hi) — the residual x − f16(x) is exact in the wide lane because hi agrees with x in the top mantissa bits — and the two limbs sit in a [2][C] buffer that the MMA's B operand consumes as TWO OF ITS COLUMNS: lane group g == 0 reads the hi buffer, g == 1 the lo buffer, and the row's two column partials are summed in the mma's FP32 accumulator. The limb build, verbatim (ik2_kernels.cu:3842-3846 at be849daa): 'if (x_limbs) {   // x = hi + lo in FP16, [2][width] (experts_gate_up_swiglu_mma's input)' + 'const uint16_t hi = f2h(x[i]);' + 'x_limbs[i] = hi, x_limbs[width + i] = f2h(x[i] - h2f(hi));'. The operand-side layout, verbatim (the experts_mma_kernel body comment, :2814-2816 — read in full this batch, the T0 residue that closes the claim): '(gate / up: the input as two FP16 limbs, x = hi + lo, [2][C] - column 0 takes hi, column 1 lo, and a row's' + 'two columns add up to the product with the FP32 input; down: h is FP16 itself, every column the same)' — the lane-group read 'xp = (const uint4*) (x16 + (Gate ? (g == 1 ? (size_t) C : 0) : ...))' selects the limb buffer per lane group, and the epilogue folds the columns ('make_float2(v[0] + v[1], ...)'). The README's own cost statement, verbatim (docs/INKLING2-MINI.md, the NVFP4 paragraph): 'The input goes in as 2 FP16 parts in 2 of the 8 tensor core columns, which costs nothing and gives back the precision of an FP32 input.' DECISION CONTENT: (a) ONE-SIDED — only the imprecise-side operand is limb-split; the NVFP4 weights need no pair because 'A code times its sub-block's scale is exact in FP16' (:2774), so the split covers exactly the side that loses bits; (b) ZERO extra mma — the limbs ride columns the B operand already has (the replicated-input geometry leaves them idle), the cost is one extra B-fragment load + the [2][C] buffer, never a second mma; (c) Dekker exactness — lo is computed from the WIDE residual (x − f16(x) in f32), never re-derived from the f16 rounding, and the fold happens in the FP32 accumulator domain (after the mma), never in f16; (d) the accuracy envelope: exact recovery on the dual-representable class, error bounded at the residual's ulp scale (~2^(e − 22) at scale 2^e, vs the naive cast's half-ulp-of-hi 2^(e − 11) — ~2048× tighter) elsewhere. CLASS SIGNATURE: an FP32 activation feeding an FP16-MMA consumer whose B columns are not all distinct-output-bearing. REGIME: FP32 activations into FP16-input MMA experts where the B operand has idle columns (the replicated-input geometry: 'down: h is FP16 itself, every column the same') and one side decodes exactly to FP16; anywhere the naive f16 cast's 11-bit mantissa is the accuracy wall. ANTI-REGIME: both operands imprecise and no idle columns (the residual-pair three-mma form is the price there — gpu_93); consumers whose accumulator is not FP32 (the fold has nowhere wide to land); bandwidth-bound shapes where the [2][C] limb buffer's 2× activation read dominates (price it at the memory ceiling). ⛔ KINSHIP (adjudicated both directions): `bf16-residual-pair-three-mma-near-f32` (gpu_93) is the family's both-operands general form — THAT stores EVERY operand as a residual pair and pays THREE mma per step (the two cross terms + the hi term, lo·lo dropped) for near-F32 products on bf16 cores; THIS rides IDLE B COLUMNS of ONE mma for ONE-SIDED recovery at zero added mma — when the input is the only imprecise operand AND the geometry leaves columns idle, this form is strictly cheaper; when no idle columns exist or both sides are imprecise, that form is what remains (the fixture pins the admission both ways); `hi-lo-split-pow2-prescale-clears-subnormal-floor` (gpu_102) shares the hi-lo NAME but owns the POW2-PRESCALE axis (scale both sides by 2^k to clear the subnormal floor — a scale, not a residual pair; different fix, same vocabulary — the grep must see this adjudication); `masked-in-place-field-prescale-other-operand` (B242, entries_gpu_98) owns the OTHER-OPERAND prescale axis (fold a constant into the other side in place) — THIS splits THIS side into limbs instead of prescaling the other. GREP EVIDENCE (re-run at the landing HEAD): 'two fp16|two-term|hi.lo|fp32 emulation|split.*precision' over the corpus -> gpu_93 + gpu_102 (adjudicated above) + B242 (the prescale axis) ONLY; 'limb|spare column|idle column' -> zero prior owner; '8 tensor core columns' -> nothing. NotBenchable-here (the entries_gpu_65 / dial9 / Batch-136 class — the law's home is an `sm_86` MMA expert path; a Metal/CPU-shaped sibling would fork the law). The measurement instrument is the CPU fixture: the Dekker-exactness arm (dual-representable rows recover bit-for-bit; the 1.0 + 2^-13 adversarial value the naive cast rounds away), the general-class bound arm, the mma census (1 step vs the pair form's 3), the spare-columns admission, and the end-to-end row-dot arm against the f32 reference. Source: alesha-pro/maya-inkling-mini @ be849daa (MIT) — src/ik2/ik2_kernels.cu:3842-3846 (the limb build in route_token_kernel) + :2803-2810 + :2814-2816 + :2820-2850 (the experts_mma_kernel operand comment + the g == 1 lane-group limb read + the column fold) + :2769-2775 (the gate / up warp geometry + 'A code times its sub-block's scale is exact in FP16') + docs/INKLING2-MINI.md (the NVFP4-without-FP4-hardware paragraph, the 2-of-8-columns sentence). Batch 379 / Plan 395 / Research 429. FACE (Batch 380, Plan 397 / Research 429 §Delta, count-neutral — the DP4A SIBLING arm + the exactness law, both at the same file family): the same hi-lo trick has a DP4A spelling beside the MMA one — verbatim (src/ik2/ik2_kernels.cuh:173-177 at e819852f, quote re-verified at the tip): 'Two signed INT8 limbs approximate each activation for DP4A with the serialized NVFP4 or Q8 weights.' + 'Scratch uses nine float-sized words per 16 inputs, one row for gate/up or S rows for down.' (expert_limb_mv — where the MMA arm THIS rule mined rides the tensor-core path at ik2_kernels.cu:3842-3846, the DP4A sibling serves the scalar/dp4a path with INT8 limbs at half the byte width per limb); the sibling's admission is the same spare-capacity shape (dp4a lanes accept two packed INT8s where the MMA arm accepted two idle FP16 B columns), so the two compose as the hi-lo family's per-execution-unit arms, never a second rule. THE EXACTNESS LAW rides here too: 'a code times its scale exact in FP16, the sum FP32' (ik2_kernels.cuh:189, verbatim) + 'an NVFP4 weight is exact in FP16' (:5, the prompt-path header) — the NVFP4 decode: 4-bit code × block scale fits FP16 exactly (a code is at most 12, the ue4m3 scale at most 240, the product inside FP16's integer-exact range, ik2_kernels.cu:2779), so no rounding at the dequant step and the limb split's input is the EXACT product, while the mma sums in FP32 — the precision boundary the fixture's bound arm already prices: exact WHERE the format multiplies, accumulated wider WHERE the hardware sums."
                .to_string(),
            ref_shape: r"// ref_shape: the FP32 input is cast to f16 once — the MMA consumes
// the 11-bit-mantissa rounding and the low bits are gone; the row dot
// misses the f32 reference by the naive cast's error.

fn f16_rt(x: f32) -> f32 {
    f16_bits_to_f32_bits(f32_bits_to_f16_bits(x.to_bits()))
}

/// the naive path: one f16 cast, no recovery.
fn naive_dot(w: &[f32], x: &[f32]) -> f32 {
    w.iter().zip(x.iter()).map(|(a, b)| a * f16_rt(*b)).sum()
}

/// the adversarial element: below f16's ulp at 1.0 (2^-10) but far above
/// the residual floor — the cast rounds it to 1.0 and the low bits (the
/// whole 2^-13 delta) are lost from every product it feeds.
fn adversarial_input(n: usize) -> Vec<f32> {
    let mut x = vec![1.0f32; n];
    x[17] = 1.0 + 2.0f32.powi(-13);
    x
}

fn f32_bits_to_f16_bits(_v: u32) -> u16 {
    0 // the cast (shape only)
}
fn f16_bits_to_f32_bits(_h: u16) -> f32 {
    0.0
}

let w = vec![1.0f32; 64];
let x = adversarial_input(64);
let reference: f32 = w.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
let got = naive_dot(&w, &x);
// the naive dot misses element 17's contribution entirely
let _ = (reference - got).abs();
".to_string(),
            opt_pattern: r"// opt_pattern: split the FP32 input Dekker-style — hi = f16(x), lo =
// f16(x − hi) from the WIDE residual — store both limbs in a [2][C]
// buffer the MMA's B operand consumes as two columns, and sum the two
// columns' partials in the FP32 accumulator. Zero extra mma; the
// weights stay single f16 (their decode is exact); the fold is wide.

fn f16_rt(x: f32) -> f32 {
    f16_bits_to_f32_bits(f32_bits_to_f16_bits(x.to_bits()))
}

/// the route_token_kernel limb build, verbatim arithmetic.
fn build_limbs(x: f32) -> (f32, f32) {
    let hi = f16_rt(x);
    let lo = f16_rt(x - hi);
    (hi, lo)
}

/// the row fold: column 0 (hi) + column 1 (lo) — in the wide lane,
/// never summed in f16.
fn limb_row_sum(hi: f32, lo: f32) -> f32 {
    hi + lo
}

/// the limb-form dot: one product pair per element, ONE mma per k-step
/// (both limbs live in the same B operand).
fn limb_dot(w: &[f32], x: &[f32]) -> f32 {
    w.iter()
        .zip(x.iter())
        .map(|(a, b)| {
            let (hi, lo) = build_limbs(*b);
            a * hi + a * lo // the mma's two column partials, folded wide
        })
        .sum()
}

fn f32_bits_to_f16_bits(_v: u32) -> u16 {
    0 // the cast (shape only)
}
fn f16_bits_to_f32_bits(_h: u16) -> f32 {
    0.0
}

let w = vec![1.0f32; 64];
let mut x = vec![1.0f32; 64];
x[17] = 1.0 + 2.0f32.powi(-13); // the adversarial element
let reference: f32 = w.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
let got = limb_dot(&w, &x);
// the pair recovers what the naive cast rounded away: exact here
assert_eq!(got.to_bits(), reference.to_bits());
// and the split is idempotent on its own class: rebuilding limbs from
// the recovered sum reproduces the same (hi, lo).
let (hi, lo) = build_limbs(x[17]);
assert_eq!(limb_row_sum(hi, lo).to_bits(), x[17].to_bits());
".to_string(),
        },
        // ── R3 — the bytes-in-flight law: a memory-bound kernel's speed is
        // outstanding request streams × per-stream reach; split the rows
        // across warps (concurrency) before widening any request, and stop
        // at the memory ceiling. ──────────────────────────────────────────
        KernelRule {
            rule_id: "bytes-in-flight-concurrency-not-prefetch-ahead".to_string(),
            gpu_concept: "MECHANISM (a memory-bound kernel's speed is set by bytes requested AT ONCE — outstanding request streams × per-stream reach; the lever is CONCURRENCY, not wider requests; category A — memory access): when a warp waits for a whole block's bytes before asking for the next ones, its in-flight bytes cap at one warp's outstanding-load reach — the memory pipe runs on one stream's width, and the kernel is latency-bound no matter how wide each request is. The remedy splits the SAME rows across SPLIT warps, each taking its own stride of the K blocks: SPLIT concurrent request streams of the SAME per-stream width. The kernel's own statement, verbatim (ik2_kernels.cu:2803-2804 at be849daa): 'A block is a group of 16 rows, its SPLIT warps each a stride of the K blocks (more rows of weights in flight than' + 'a warp alone keeps: a warp waits for a block's bytes before it asks for more than the next one's).' The guide's measured form, verbatim (docs/INKLING2-MINI.md:119): 'The first tensor core kernel was slower than the scalar one: 100 tok/s against 118. A warp waits for a block's bytes before it asks for the next ones, so the speed of a kernel here is set by how many bytes are requested at once. With 8 warps on the same 16 rows, each taking its own stride of the row, the same code runs at 745 GB/s.' The code form (read in full this batch): template<int SPLIT> with `for (int k = split; k < nk; k += SPLIT)`, per-warp software pipelining of exactly ONE stride ahead (the `NvStep nxt` handshake — ahead is per-warp LOCAL, never a deeper ring), partials in `__shared__ float2 part[SPLIT][8]` (disjoint slots, no atomics), and one warp folding them in ascending split order at the end; the split is a measured per-direction env knob (IK2_EXPERT_MMA_SPLIT default 8 gate / 4 down — the down projection runs a smaller split). THE CEILING ANTI-REGIME, BOTH DIRECTIONS MEASURED: once the kernel is at the memory's limit neither lever pays — verbatim (docs/INKLING2-MINI.md:131): 'Asking for more bytes ahead in the dense Q8 kernels. They already run at 715 GB/s, the memory's limit.' — and layer-level prefetch is WORSE than flat (docs/INKLING2.md:196-197): 'prefetching two or three layers ahead, or before the routing round trip (both crowd' + 'the cache: a 21.8k prompt on an RTX PRO 6000 went 8.05 -> 9.7 s)'. DECISION CONTENT: (a) the metric is bytes-in-flight (concurrency × per-stream reach), not SM occupancy and not per-thread trip count — a grid can be full of warps all waiting on block-granular loads; (b) the lever ORDER: spread the same rows across warps BEFORE widening any single request — a single stream cannot buy in-flight bytes by asking wider, its reach is hardware-capped (the fixture's MLP-reach clamp); (c) per-warp lookahead is exactly ONE stride (the nxt handshake) — depth beyond the latency window adds nothing proportional; (d) the ceiling test: at the measured memory limit, stop — neither width nor concurrency moves throughput, and a prefetch stream that touches ANOTHER working set evicts the live one (the 8.05 -> 9.7 s regression class); (e) the fold: strided warps write disjoint slots, one warp folds in ascending split order — deterministic, no atomics. CLASS SIGNATURE: a latency-bound weight-streaming kernel whose per-warp wait is block-granular. REGIME: memory-bound quantized-weight streaming (expert GEMV / MMA rows) below the memory ceiling, per-warp waits block-granular. ANTI-REGIME: at/above the memory ceiling (measured: 715 GB/s flat; cache-crowding regression when the prefetch crosses working sets); compute-bound consumers (the occupancy-starved vs high-occupancy regime boundary — consult the staging ring's Batch-148 rider before deepening anything); splits that bust the register budget or the fold's slot count. ⛔ KINSHIP (adjudicated both directions): `occupancy-keyed-value-split-register-budget-ladder` (gpu_94) keys on OCCUPANCY of the whole-work GRID (a starved grid fills SMs by splitting values across more blocks); THIS keys on BYTES-IN-FLIGHT of the memory PIPELINE — the lens is the memory counter, and the two compose when both starve (a split that fills SMs AND multiplies streams); the uint4 face on `strided-loop-batched-prefetch-unroll-tail` (gpu_150) owns per-thread TRIP-COUNT reduction (batch dependent per-lane loads into one wide load) — THIS keeps per-load width fixed and multiplies REQUESTERS; if the loads are independent per lane, batch them (that rule); if each warp waits for a whole block, spread the warps (this rule); `one-barrier-a-tile-prefetch-occupancy-pair` (gpu_102) owns the staged-tile pole's barrier/prefetch coupling and `cpasync-multistage-tile-pipeline` (gpu_09) the async-copy ring — both are the smem pole's HOW; THIS is the register path's concurrency split, and the ceiling anti-regime is the lever they share (the Batch-130/148 riders: a deep ring at the ceiling buys nothing); `fold-query-tokens-for-occupancy` (gpu_01) folds query tokens into workgroups for occupancy — the same SM-fill lens as gpu_94, distinct from the memory-pipeline lens. GREP EVIDENCE (re-run at the landing HEAD): 'bytes.*flight|flight.*bytes|requested at once' over the corpus -> zero phrase owners (the one 'in flight' hit is gpu_07's command-buffer rule — unrelated); 'prefetch.*ahead|more bytes ahead' -> the staging families (adjudicated above); '745|715 GB' -> nothing; 'stride of the row|warp.*stride' -> nothing. NotBenchable-here (the entries_gpu_65 / dial9 / Batch-136 class — the law's home is a CUDA weight-streaming path; the win metric is GB/s on their silicon). The measurement instrument is the CPU fixture: the latency-bound pipe model with the per-stream MLP-reach clamp (the split multiplies throughput below the ceiling; a single wider stream cannot — the reach clamps it), the ceiling arm (flat at the limit, both knobs), the cache-crowding arm (the other working set evicts the live one), the deterministic fold arm, and the one-step-lookahead arm (depth beyond the latency window adds nothing proportional). Source: alesha-pro/maya-inkling-mini @ be849daa (MIT) — src/ik2/ik2_kernels.cu:2803-2806 (the SPLIT comment + the launch_bounds) + :2820-2842 (the k-stride loop + the one-step nxt pipelining) + :2845-2852 (the part[SPLIT][8] fold) + :4875-4895 (experts_mma_split: the env knob, 8 gate / 4 down) + docs/INKLING2-MINI.md:119 (the 100 tok/s -> 745 GB/s cell) + :131 (the 715 GB/s ceiling negative) + docs/INKLING2.md:196-197 (the layer-prefetch crowd negative). Batch 379 / Plan 395 / Research 429."
                .to_string(),
            ref_shape: r#"// ref_shape: one warp streams the whole K range of its row group,
// waiting for each block's bytes before the next block's loads — the
// memory pipe runs on one stream's reach, and the kernel is
// latency-bound no matter how wide each request is.

struct Pipe {
    latency_ticks: f64,
    bandwidth_bytes_per_tick: f64,
    per_stream_reach: f64, // one warp's outstanding-load reach (hardware)
}

impl Pipe {
    fn throughput(&self, streams: f64, width_bytes: f64) -> f64 {
        let in_flight = streams * width_bytes.min(self.per_stream_reach);
        (in_flight / self.latency_ticks).min(self.bandwidth_bytes_per_tick)
    }
}

/// the naive form: ONE warp, its whole block — the wait is
/// block-granular, so the next block's loads never overlap.
fn naive_throughput(pipe: &Pipe) -> f64 {
    pipe.throughput(1.0, 512.0)
}

// asking wider does not help: the reach clamps the single stream
fn wider_request_throughput(pipe: &Pipe) -> f64 {
    pipe.throughput(1.0, 512.0 * 8.0)
}

let pipe = Pipe {
    latency_ticks: 8.0,
    bandwidth_bytes_per_tick: 1.0e6,
    per_stream_reach: 512.0,
};
let naive = naive_throughput(&pipe);
let wider = wider_request_throughput(&pipe);
assert_eq!(naive, wider, "a wider single request is clamped to the same reach");
"#.to_string(),
            opt_pattern: r#"// opt_pattern: split the SAME rows across SPLIT warps, each taking its
// own stride of the K blocks — SPLIT concurrent request streams of the
// SAME per-stream width — with per-warp software pipelining of exactly
// ONE stride ahead and a deterministic disjoint-slot fold.

struct Pipe {
    latency_ticks: f64,
    bandwidth_bytes_per_tick: f64,
    per_stream_reach: f64,
}

impl Pipe {
    fn throughput(&self, streams: f64, width_bytes: f64) -> f64 {
        let in_flight = streams * width_bytes.min(self.per_stream_reach);
        (in_flight / self.latency_ticks).min(self.bandwidth_bytes_per_tick)
    }

    /// the ceiling test: neither lever moves throughput at the limit.
    fn at_ceiling(&self) -> bool {
        self.throughput(1.0, self.per_stream_reach) >= self.bandwidth_bytes_per_tick
    }
}

/// the split form: SPLIT warps, same per-stream width — the concurrency
/// lever, not the width lever.
fn split_throughput(pipe: &Pipe, split: f64) -> f64 {
    pipe.throughput(split, 512.0)
}

/// the deterministic fold: strided writers take disjoint slots; one
/// warp folds them in ascending split order.
fn split_fold(partials: &[f64]) -> f64 {
    let mut tot = 0.0;
    for p in partials {
        tot += *p;
    }
    tot
}

let pipe = Pipe {
    latency_ticks: 8.0,
    bandwidth_bytes_per_tick: 1.0e6,
    per_stream_reach: 512.0,
};
let naive = pipe.throughput(1.0, 512.0);
let split8 = split_throughput(&pipe, 8.0);
assert!(split8 > naive * 4.0, "the split multiplies the naive stream");
assert!(!pipe.at_ceiling(), "below the ceiling the split pays");
// at the ceiling neither lever pays — stop.
let ceiling_pipe = Pipe {
    latency_ticks: 1.0,
    bandwidth_bytes_per_tick: 715.0 * 1024.0 * 1024.0,
    per_stream_reach: 512.0,
};
assert!(ceiling_pipe.at_ceiling());
assert_eq!(split_throughput(&ceiling_pipe, 8.0), ceiling_pipe.bandwidth_bytes_per_tick);
let _ = split_fold(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
"#.to_string(),
        },
    ]
}
