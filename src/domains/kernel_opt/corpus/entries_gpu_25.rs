//! Entries slice -- Batch 113 (4 rules) -- the llama.cpp v0.4.0 delta
//! re-mine (Research 137 / Plan 086). Source: ggml-org/llama.cpp @
//! 5266f24da75dc449bd56cbed7addb9c8e4a6a73e (v0.4.0, MIT; re-cloned at
//! the pin for this batch; a quote-verification lane re-read all five
//! surfaces with exact line numbers and corrected three attributions --
//! the XOR-swizzle and MoE-fusion commits are CUDA-side, --lazy-mode is
//! app-layer). This file lands the GOAT lead pair from the Mamba-2
//! chunked SSD-MMA kernel (rules #1/#2 of the verdict table), the
//! shader fastdiv cluster (#3), and the sparse-KV index-list idiom (#4,
//! graded B-, landed as a rule in category I after the T1 fa.metal
//! read confirmed the sentinel + clamp + single-source-of-truth
//! contracts). The ~17-member rider sweep lands in the ridden rules'
//! name-fields, not here. Fixtures are original minimal Rust
//! reproductions (parse-only; no --fix path). Category membership lives
//! ONLY in `KERNEL_CATEGORIES` (mod.rs) -- never encode it here.
use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic -- grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![

// ============================================================
// -- chunk-transition-consumed-by-mma -- Batch 113 / Research 137 / Plan 086.
        KernelRule {
            rule_id: "chunk-transition-consumed-by-mma".to_string(),
            gpu_concept: "In a chunked linear-recurrence prefill, put the heavy per-chunk products on the MATRIX UNIT and build each chunk's shared product ONCE for reuse across every consumer tile: the per-chunk Gram C*B^T is computed in each chunk one time and reused across the head_dim channel tiles (the kernel header's own contract: 'C*B^T computed in each chunk one time and reused across the head_dim channel tiles'), the masked decay fold (exp(acs_i - acs_j), causal mask) is applied to the STORED Gram once per tile pair -- never per output element -- and both output halves ride the matrix unit (intra-chunk y_diag = masked-Gram . dtX; inter-chunk y_inter = C . S_prev). Eligibility is a FIVE-CLAUSE gate naming every cost factor (the B46 external confirmation): mma_tokens > 0 (work left after the reserve), scalar-decay shape (A is {1, n_head}), hardware capability (simdgroup MMA), tile alignment (d_state % 8), and head-dim SPECIALIZATION (the kernel is written for one head dim) -- a kernel this specialized must say what it specializes for, in the gate, per clause. The dispatch tail is the load-bearing shape: mma_tokens = the largest multiple of CHUNK that leaves snap_reserve for the tail ('tokens reserved for sequential kernel rollback snapshots') -- run the MMA kernel over the chunk multiple, then hand the remainder to the SEQUENTIAL kernel family with a token_offset (a kernel-FAMILY fallback, not padding; snap_reserve exists because the rollback-snapshot kernel owns that region -- a spec-decode interop constraint on chunked fast paths, the B71/B58 family). Semantics: chunk OUTPUTS reassociate (the Gram path reorders products vs the sequential form) => tolerance-gated, stated; the CARRIED STATE does NOT reassociate -- that partition is carried-state-reduction-stays-scalar's content, the two rules compose. Magnitude: O(CS^2 . nc + CS . nc . HD) work moves from scalar lanes to the matrix unit per chunk (upstream ships it as the Mamba-2 prefill optimization). Deterministic: fixed chunk/tile order + the deterministic tail split. Distinct from first-order-affine-recurrence-associative-scan (B79 -- the scan/parallelization axis for scan-ELIGIBLE pair recurrences; this is the chunk family's MMA member -- the recurrence stays sequential in time, the per-chunk PRODUCTS parallelize), from m-group-shared-weight-scan-register-relief (B110 -- kTGroups sharing ONE weight K-slice in a skinny GEMM; this is the chunk-Gram member of the same build-once-share-everywhere shape), from smem-envelope-gate-kernel-variant (B108 -- a capacity predicate admitting a VARIANT; this is the eligibility + tail-dispatch shape for a matrix-unit variant), and from tile-coo-sparse-tile-aligned-format (a FORMAT designed for its consumer's tiles; this is a PLACEMENT decision for recurrence products). Source: ggml-org/llama.cpp @ 5266f24da (v0.4.0, MIT) -- ggml/src/ggml-metal/kernels/ssm.metal:300-302 (the header contract), :390-415 (build one 8x64 row tile of SAM per simdgroup, reused across channel tiles), :406-413 (the causal exp(acs_i - acs_j) mask fold on the stored Gram), :417-427 (y_diag MMA), :429-435 (y_inter = C . S_prev MMA); ggml/src/ggml-metal/ggml-metal-ops.cpp:1773-1782 (the five-clause gate + snap_reserve + mma_tokens), :1802-1819 (the sequential fallback + the token_offset tail). Batch 113 / Research 137 / Plan 086.".to_string(),
            ref_shape: "// naive: chunked SSM prefill where every channel tile rebuilds the
// chunk Gram and every output element is a scalar-lane product.
fn chunk_outputs_naive(c: &[f32], b: &[f32], dtx: &[f32],
                       chunks: usize, cs: usize, hd: usize) -> Vec<f32> {
    let mut y = Vec::new();
    for ch in 0..hd {
        for t in 0..chunks * cs {
            let mut acc = 0.0f32;
            for j in 0..cs {
                // the Gram product recomputed per (channel, token, j):
                // no build-once reuse, no matrix unit
                acc += c[t * cs + j] * b[j * cs] * dtx[ch];
            }
            y.push(acc);
        }
    }
    y
}
".to_string(),
            opt_pattern: "// optimized: build the chunk Gram ONCE per (tile-pair), reuse it across
// every channel tile; outputs ride the matrix unit; the tail beyond the
// chunk multiple falls back to the sequential kernel family.
const CHUNK: usize = 256;

fn chunk_outputs_mma(c: &[f32], b: &[f32], dtx: &[f32],
                     n_tokens: usize, hd: usize) -> (Vec<f32>, usize) {
    let mma_tokens = (n_tokens / CHUNK) * CHUNK; // largest multiple; tail reserved
    let mut y = Vec::with_capacity(mma_tokens * hd);
    for ch in 0..hd {
        for t0 in (0..mma_tokens).step_by(CHUNK) {
            // Gram for this chunk, built once (matrix unit), then reused
            // by every channel tile of the chunk
            let gram = build_gram(&c[t0 * CHUNK..], &b[t0 * CHUNK..], CHUNK);
            for t in 0..CHUNK {
                let mut acc = 0.0f32;
                for j in 0..CHUNK {
                    acc += gram[t * CHUNK + j] * dtx[ch]; // decay folded once
                }
                y.push(acc);
            }
        }
    }
    (y, mma_tokens) // caller hands [mma_tokens..] to the sequential kernel
}

fn build_gram(_c: &[f32], _b: &[f32], cs: usize) -> Vec<f32> {
    vec![0.0; cs * cs] // simdgroup MMA in the real kernel
}
".to_string(),
        },

// ============================================================
// -- carried-state-reduction-stays-scalar -- Batch 113 / Research 137 / Plan 086.
        KernelRule {
            rule_id: "carried-state-reduction-stays-scalar".to_string(),
            gpu_concept: "In a chunked linear recurrence, partition the per-chunk products by WHAT CONSUMES THEM: products feeding PER-CHUNK OUTPUTS (non-compounding -- each output is consumed once, so a rounding difference is a local tolerance question) may ride the matrix unit; products feeding the CARRIED STATE (compounding -- last-bit drift crosses every chunk boundary through the recurrence and compounds over the sequence) stay SCALAR in token order even though the same matrix unit is idle-ready. The source's own contract, verbatim: 'Keep the carried-state reduction in token order. Reassociating this particular product with MMA compounds rounding differences at every chunk boundary; CB, y_diag, and C*S remain on the matrix unit.' This is a per-product PLACEMENT decision with a numerics rationale -- not a global fast/slow path choice and not a bit-identity certificate: the rule tells you WHICH products may reassociate at all, and the answer is decided by the consumer's accumulation depth, not by the operator's cost. Generalizes to every chunked-recurrence MMA implementation (GDN, RWKV, SSM all make this partition; our own GDN rule family carries the same shape). Distinct from reduction-restructure-needs-source-owned-comparator (B95 -- pins a RESTRUCTURED reduction bit-identical against a source-owned oracle; this DECIDES which products may reassociate at all -- the oracle-half and the partition-half compose), from first-order-affine-recurrence-associative-scan (B79 -- the scan-eligibility gate for the whole recurrence; this is the within-chunk placement axis once a chunked form exists), from exact-op-parity-for-analytic-backward (training fwd/bwd operator parity; this is inference-side carry numerics), and from gdn-transition-replay-log-commit-many (B81 -- replays accepted tokens SEQUENTIALLY by selection-awareness; this keeps the CARRIED STATE sequential by numerics -- different reason, same sequential-in-token-order shape, declare both when citing either). Source: ggml-org/llama.cpp @ 5266f24da (v0.4.0, MIT) -- ggml/src/ggml-metal/kernels/ssm.metal:454-456 (the contract, verbatim), :457-468 (the scalar carried-state update: chunk_decay * state + state_c in token order), vs :417-435 (CB, y_diag, C*S on the matrix unit). Batch 113 / Research 137 / Plan 086.".to_string(),
            ref_shape: "// naive: the carried-state update rides the matrix unit with the chunk
// outputs -- the tile matmul accumulates the carry in ITS reduction
// order, and the last-bit drift re-enters the recurrence at every
// chunk boundary, compounding over the sequence.
fn carry_via_mma(state: &mut [f32], gram_tile: &[f32], chunks: usize, n: usize) {
    for _c in 0..chunks {
        for i in 0..n {
            let mut acc = 0.0f32;
            for j in 0..n {
                acc += gram_tile[i * n + j] * state[j]; // tile-order sum
            }
            state[i] = acc; // reassociated carry: drift compounds per chunk
        }
    }
}
".to_string(),
            opt_pattern: "// optimized: the CARRIED STATE stays a scalar loop in token order; only
// the chunk OUTPUT products (non-compounding consumers) ride the matrix
// unit. The source's contract: 'Keep the carried-state reduction in
// token order. Reassociating this particular product with MMA compounds
// rounding differences at every chunk boundary.'
fn carry_scalar_token_order(state: &mut [f32], chunk_decay: f32,
                            b_row: &[f32], dtx: &[f32], chunks: usize) {
    for _c in 0..chunks {
        for (i, s) in state.iter_mut().enumerate() {
            // token-order scalar accumulation: t = 0, 1, ... -- never a
            // tile-tree sum, so chunk boundaries see one rounding, not a
            // reassociated one
            let state_c = b_row[i] * dtx[i];
            *s = chunk_decay * *s + state_c;
        }
    }
    // y_diag and C*S stay on the matrix unit (see
    // chunk-transition-consumed-by-mma) -- the partition, not a speed path
}
".to_string(),
        },

// ============================================================
// -- shader-fastdiv-mulhi -- Batch 113 / Research 137 / Plan 086.
        KernelRule {
            rule_id: "shader-fastdiv-mulhi".to_string(),
            gpu_concept: "Replace shader hardware div/mod with the three-part fast-division cluster, because coordinates and dimensions are mostly small, power-of-two, or repeated: (1) fastmod -- when the divisor is a power of two, mod is a mask (a & (b-1)); the leading comment names the regime: 'mod and div are expensive and coordinates/dimensions are often power of 2 or equal to 1'. (2) fastdiv -- the ROUND-UP magic-multiplier form built from the mulhi intrinsic, because 64-bit multiply does not exist in the shader language so umulExtended IS the construction: fastdiv(n, mp, L) = (umulExtended-mulhi(n, mp) + n) >> L (the +n IS the round-up correction; the magic pair (mp, L) is precomputed host-side -- 'see init_fastdiv_values'). (3) fastdiv_L -- pack the per-divisor shift amounts so one uniform carries many: eight BYTE-SPACED slots in one u32, each holding a 6-bit value ((packed >> (slot * 8)) & 0x3F). (4) fastdiv_small -- the quotient-zero branch-skip: (a < b) ? 0 : a / b, because in coordinate decomposition the leading (high-order) divisions usually have quotient 0 and the hardware divide is skipped for them entirely. Regime: shader coordinate decomposition over tensor shapes (get_indices-style index math), general divisors, and quotient-0-heavy leading divisions. Distinct from division-by-invariant-multiplication (rust_perf B59 -- the CPU u64 widening-magic form; this is the GPU mulhi-intrinsic + round-up + L-pack + zero-guard cluster, a different substrate with different construction constraints), and from strength-reduce-div-pow2 (the pow-2-only arm -- fastmod generalizes the family with the general-divisor mulhi path). No commit-reference comment exists in-source; provenance is the v0.4.0 delta (commit c9ca51c #27526 per the release notes). Source: ggml-org/llama.cpp @ 5266f24da (v0.4.0, MIT) -- ggml/src/ggml-vulkan/vulkan-shaders/utils.glsl:4-10 (fastmod + the regime comment), :12-18 (fastdiv umulExtended + round-up + the init_fastdiv_values pointer), :20-22 (fastdiv_L byte-spaced 8-slot / 6-bit mask packing), :24-26 (fastdiv_small quotient-zero). Batch 113 / Research 137 / Plan 086.".to_string(),
            ref_shape: "// naive: raw hardware div/mod in the shader's coordinate decomposition
// -- every index math op pays the full-width divide, even when the
// divisor is a power of two or the quotient is zero.
fn get_indices_naive(idx: u32, ne0: u32, ne1: u32, ne2: u32) -> (u32, u32, u32, u32) {
    let i00 = idx % ne0;
    let i01 = (idx / ne0) % ne1;
    let i02 = (idx / (ne0 * ne1)) % ne2;
    let i03 = idx / (ne0 * ne1 * ne2); // usually 0 -- full divide anyway
    (i00, i01, i02, i03)
}
".to_string(),
            opt_pattern: "// optimized: the three-part fast-division cluster -- pow2 mod as a
// mask, round-up mulhi fastdiv for general divisors, packed L slots,
// and the quotient-zero skip for leading divisions.
fn fastmod(a: u32, b: u32) -> u32 {
    if b & (b - 1) == 0 {
        a & (b - 1) // power of two: mod is a mask
    } else {
        a % b
    }
}

fn fastdiv(n: u32, mp: u32, l: u32) -> u32 {
    // msbs = mulhi(n, mp) via umulExtended; (msbs + n) >> L is the
    // round-up magic form; (mp, L) precomputed host-side
    let msbs = mulhi(n, mp);
    (msbs + n) >> l
}

fn fastdiv_l(packed: u32, slot: u32) -> u32 {
    (packed >> (slot * 8)) & 0x3F // eight byte-spaced slots, 6-bit values
}

fn fastdiv_small(a: u32, b: u32) -> u32 {
    if a < b { 0 } else { a / b } // leading divisions usually have quotient 0
}

fn mulhi(_a: u32, _b: u32) -> u32 {
    0 // umulExtended hi word in the real shader
}
".to_string(),
        },

// ============================================================
// -- sparse-kv-index-list-sentinel-padded -- Batch 113 / Research 137 / Plan 086.
        KernelRule {
            rule_id: "sparse-kv-index-list-sentinel-padded".to_string(),
            gpu_concept: "When attention may attend to only a SUBSET of KV positions per row (a mask, a sparse hint, a selected set), compress the selection ONCE into an explicit index list and make the attention kernel gather by that list, instead of re-scanning or re-masking the full KV extent every pass: a producer kernel walks each mask row, compresses the finite entries into an ascending KV-index list, and pads the tail with a -1 SENTINEL up to n_kv_max_padded -- a multiple of the consumer's tile size, so the gather stays tile-aligned without a ragged tail; the consumer reads the list and skips sentinel lanes. Three contracts make it safe. (1) The mask remains the SINGLE SOURCE OF TRUTH for the values -- the list is an acceleration hint derived from it, never an authority. (2) TRUST-BUT-CLAMP the hint: the producer writes positions in order and clamps at the declared capacity ('if the hint is violated, keep only the first n_kv_max entries') -- a wrong hint degrades to prefix-correct attention, never out-of-bounds or silently-wrong attention. (3) The dense overflow arm: a row with more selected positions than the register cache re-reads the mask to write the rest -- the producer, not the consumer, absorbs the slow path. Regime: decode-side sparse/selected attention (Q == 1 enforced by the host in-source); a dense workload gains nothing and pays the list pass. Distinct from block-sparse-gather (the popcorn block-sparse family -- block-granular gather by computed extent; this is row-granular EXPLICIT index lists derived from a mask, with the sentinel + clamp contracts), from backing-provenance-causal-shadowed-extent-widen (B103 -- reads padded rows without copying when causally unreachable; this COMPRESSES the selection into a list so the padded tail is sentinel-skipped gather work), and from sliding-window-plus-sparse (window shapes; this is the mask-derived-list substrate any window/sparse shape can produce). Source: ggml-org/llama.cpp @ 5266f24da (v0.4.0, MIT) -- ggml/src/ggml-metal/kernels/fa.metal:1076-1078 (the producer header: 'compress the finite entries of each KQ mask row into a list of KV indices (ascending order), padded with -1 up to n_kv_max_padded (a multiple of OP_FLASH_ATTN_EXT_VEC_NCPSG)' + 'the mask remains the single source of truth for the values'), :1152-1156 (the clamp: 'if the hint is violated, keep only the first n_kv_max entries'), :1158-1172 (the dense-overflow re-read arm), :1174-1179 (the -1 sentinel padding), :1318-1324 (the consumer gathering by the list). Batch 113 / Research 137 / Plan 086.".to_string(),
            ref_shape: "// naive: sparse attention re-scans the FULL KV extent every pass and
// re-applies the mask per element -- the same selection work repeated
// per query head, and the tail is ragged against the tile size.
fn attend_scan_all(mask: &[bool], k: &[f32], v: &[f32], q: f32) -> f32 {
    let mut acc = 0.0f32;
    for (j, m) in mask.iter().enumerate() {
        if *m {
            acc += q * k[j] * v[j]; // full scan + per-element mask test
        }
    }
    acc
}
".to_string(),
            opt_pattern: "// optimized: the producer compresses the mask into an ascending index
// list ONCE, -1 sentinel-padded to the tile multiple; the consumer
// gathers by the list and skips sentinels; a violated hint clamps to
// the declared capacity (prefix-correct, never out-of-bounds).
const TILE: usize = 256;

fn compress_indices(mask: &[bool], cap: usize) -> Vec<i32> {
    let mut list = Vec::with_capacity(cap);
    for (j, m) in mask.iter().enumerate() {
        if *m && list.len() < cap {
            list.push(j as i32); // ascending, in order
        }
        // 'if the hint is violated, keep only the first n_kv_max entries'
    }
    let padded = list.len().div_ceil(TILE) * TILE;
    list.resize(padded, -1); // -1 sentinel up to the tile multiple
    list
}

fn attend_by_list(list: &[i32], k: &[f32], v: &[f32], q: f32) -> f32 {
    let mut acc = 0.0f32;
    for &j in list {
        if j >= 0 {
            acc += q * k[j as usize] * v[j as usize]; // gather by list
        } // sentinel lanes skipped: no mask test, no ragged tail
    }
    acc
}
".to_string(),
        },
    ]
}
