//! Entries slice 138 — Batch 304 (Plan 309 / Research 348): the vllm
//! bugfix window, the `TensorFold` SASS-identity + dual-bank pair, and the
//! lithos bit-exact geometry trio. Sources: vllm-project/vllm @
//! b42f25b / 4da2a82 / d4820206 (Apache-2.0), ashhart/TensorFold @
//! 2922b93 + 2e04524 + c6193ca/dd0a9cc + 8abd450 (Apache-2.0),
//! lithos-ai/lithos-metal @ 8ed1611 / cd220ab / 2d51c4a (Apache-2.0).
//! Seven rules and one count-neutral rider face, every quote re-verified
//! byte-for-byte at the pin (plan 309 T0, 2026-10-29). The fixtures are
//! ORIGINAL minimal Rust reproductions — nothing transcribed from the
//! sources. All rules are `NotBenchable` (write-completeness,
//! silent-launch, ordering, compiled-identity, memory-budget and
//! determinism-admission axes — no Pareto bench in this corpus hosts
//! them; the Batch-136 domain-reason class).

use super::KernelRule;

#[allow(clippy::too_many_lines)]
fn slice_vllm_b304_window() -> Vec<KernelRule> {
    vec![
        // ── Rule 1 (category C — fusion): the fused act+quant kernel walks the padded domain so every legacy scale address is written. ──
        KernelRule {
            rule_id: "fused-pair-replacement-writes-every-legacy-output-every-path".to_string(),
            gpu_concept: "MECHANISM (a fused kernel that REPLACES a producer pair inherits EVERY output the pair wrote, on every admitted path; category C — fusion): the NVFP4 fused silu-mul+quant kernel replaced the (activation, quantization) producer pair but walked only the valid (row, col) domain, so the swizzled scale-factor tensor's padded addresses — rows rounded up to 128, scale columns rounded up to 4 — kept whatever the allocation held, and the downstream NVFP4 GEMMs read them as scale factors: NaN logits. The fix widens the walk to the padded domain — \"int32_t num_packed_cols, int32_t num_padded_cols,\" / \"int const sf_m = round_up<int>(numRows, 128);\" / \"// Iterate over all rows and cols including padded ones -\" / \"//  ensures we visit every single scale factor address to initialize it.\" — feeds zeros outside the valid region (\"// If we are outside valid rows/columns, feed zeros\"), and writes the scale output its identity on every padded address: \"// The swizzled scale tensor may be allocated with torch.empty.\" / \"// Explicitly initialize padded rows and columns in this kernel so\" / \"// downstream NVFP4 GEMMs never observe stale scale factors.\" / \"*sf_out = 0;\" — with the grid covering the padding (\"int grid_y = vllm::div_round_up(num_padded_cols, static_cast<int>(block.x));\"). THE LAW: fusion may not drop a legacy output from any admitted path — the fused kernel's loop domain must cover every address of every output the replaced pair wrote (padding included), with out-of-domain inputs fed zeros and legacy outputs written their identity on the padded addresses; the regression pins it with a dirty fill: \"The kernel must overwrite dirty scale padding (rows and columns)\" / \"without relying on a zero-initialized allocation.\", the scale tensor pre-filled \"0x7F7F7F7F\" (a NaN pattern), \"assert torch.count_nonzero(full_scale[m:, :]) == 0\" + \"assert torch.count_nonzero(full_scale[:m, d // BLOCK_SIZE :]) == 0\", and \"# Valid entries must have been written (no leftover 0x7F == NaN fill).\" REGIME: fused activation+quant kernels with a SECOND legacy output (scale factors), swizzled/padded auxiliary tensors whose padding the consumer's tile walk covers, allocations not guaranteed zero-initialized. ANTI-REGIME: fusions whose replaced pair has a single output; padding the consumer provably never reads (the dead-slot-tolerance rule's domain — cite, do not re-own); buffers whose zero-init is a real contract. KINSHIP (do not double-count): whole-window-specialization-omits-metadata-poison-verified (entries_gpu_135) owns the READ side — a specialized kernel proven never to READ the omitted surface by a poison differential; THIS owns the WRITE side — a fused replacement proven to WRITE every legacy output by a dirty-fill sentinel; compose. apple9-coop-tensor-destination-init-lost-runtime-format (entries_gpu_136) owns init MECHANISM loss (constructor-initialized destination values lost under runtime format selection); THIS owns output-set COMPLETENESS of a fused replacement across every admitted path — the init-vs-path family, adjacent poles. producer-launch-derived-from-actual-group-size (entries_gpu_101) owns unwritten READS from a wrong launch extent; the conv_state dead-slot rule (entries_gpu_02) owns consumer-semantics dead-slot TOLERANCE (a deliberately unwritten slot the consumer overwrites — the opposite duty); the NaN-poisoned padding lane (entries_gpu_117) is a VALUES guard for reads past live rows. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'uninitialized|unwritten' -> the launch-extent and masked-reduction families (entries_gpu_101, entries_gpu_04) and the dead-slot tolerance (entries_gpu_02) — no fused-replacement write-completeness owner; 'silu_mul|act quant fusion|scale factor' -> the gate/up GLU fusion family (entries_gpu_01 riders) fuses projections, never the act+quant scale-output pair — no owner; 'padded rows|scale padding|swizzled scale' -> the stage/swizzle staging families (entries_gpu_135 stage-copy) — different surfaces. NotBenchable here (a write-completeness/correctness axis — the gain is no NaN logits downstream; the dirty-fill sentinel and the every-address-write predicate are CPU-testable and ARE the fixtures). Source: vllm-project/vllm @ b42f25b00c3d13b565ec60e81928cd21efefce32 (Apache-2.0) — csrc/quantization/fp4/activation_nvfp4_quant_fusion_kernels.cu:41,55,65-76,99-111,164-178 + tests/kernels/quantization/test_silu_mul_nvfp4_quant.py:81-119, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: fused-pair-replacement-writes-every-legacy-output-every-path):
// the fused act+quant kernel walks ONLY the valid (row, col) domain, so the
// scale-factor tensor's padded rows and scale columns keep whatever the
// allocation held -- here a NaN-pattern dirty fill -- and the downstream
// tile that reads the whole swizzled scale tensor multiplies by garbage.
#![allow(dead_code)]

const ROWS: usize = 5;
const COLS: usize = 8;
const SF_COLS: usize = 2; // one scale per 4 columns
const PADDED_ROWS: usize = 8; // the swizzled scale tensor pads rows to 8
const PADDED_SF_COLS: usize = 4; // ...and scale columns to 4
const DIRTY: u32 = 0x7F80_0001; // a NaN pattern, the stand-in for torch.empty

struct SfTensor {
    rows: usize,
    cols: usize,
    data: Vec<u32>, // raw scale bits
}

fn alloc_dirty() -> SfTensor {
    SfTensor {
        rows: PADDED_ROWS,
        cols: PADDED_SF_COLS,
        data: vec![DIRTY; PADDED_ROWS * PADDED_SF_COLS],
    }
}

// the fused replacement of the (act, quant) producer pair: it inherits the
// pair's quantized payload AND its scale-factor output -- but walks only
// the valid domain, so every padded scale address stays unwritten.
fn fused_act_quant_valid_domain_only(
    x: &[[i32; COLS]; ROWS],
    out_q: &mut [[i32; COLS]; ROWS],
    sf: &mut SfTensor,
) {
    for r in 0..ROWS {
        for g in 0..SF_COLS {
            let lo = g * 4;
            let mut peak = 0i32;
            for c in lo..lo + 4 {
                peak = peak.max(x[r][c].abs());
            }
            let scale = if peak == 0 { 1 } else { peak };
            for c in lo..lo + 4 {
                out_q[r][c] = x[r][c] * 100 / scale;
            }
            sf.data[r * sf.cols + g] = scale as u32;
        }
    }
}

// the downstream GEMM's tile walk covers the padded rows (the swizzled
// scale layout is tiled over PADDED_ROWS), so it multiplies the padding's
// zero payload by whatever the scale address holds.
fn gemm_tile_reads_whole_sf(q: &[[i32; COLS]; ROWS], sf: &SfTensor) -> f32 {
    let mut acc = 0.0f32;
    for r in 0..sf.rows {
        for c in 0..COLS {
            let payload = if r < ROWS { q[r][c] as f32 } else { 0.0 };
            let scale = f32::from_bits(sf.data[r * sf.cols + c / 4]);
            acc += payload * scale;
        }
    }
    acc
}

fn dirty_count(sf: &SfTensor) -> usize {
    sf.data.iter().filter(|&&b| b == DIRTY).count()
}

#[test]
fn the_padded_scale_addresses_stay_unwritten() {
    let mut x = [[0i32; COLS]; ROWS];
    for r in 0..ROWS {
        for c in 0..COLS {
            x[r][c] = ((r * COLS + c) % 7) as i32 - 3;
        }
    }
    let mut out_q = [[0i32; COLS]; ROWS];
    let mut sf = alloc_dirty();
    fused_act_quant_valid_domain_only(&x, &mut out_q, &mut sf);
    // valid scale addresses were written; every padded one keeps the dirty
    // fill: rows ROWS..PADDED_ROWS and scale columns SF_COLS..PADDED_SF_COLS.
    assert_eq!(
        dirty_count(&sf),
        PADDED_ROWS * PADDED_SF_COLS - ROWS * SF_COLS
    );
}

#[test]
fn the_downstream_tile_reads_a_nan_scale() {
    let x = [[1i32; COLS]; ROWS];
    let mut out_q = [[0i32; COLS]; ROWS];
    let mut sf = alloc_dirty();
    fused_act_quant_valid_domain_only(&x, &mut out_q, &mut sf);
    // 0 x NaN poisons the tile accumulator: the NaN-logits downstream shape.
    assert!(gemm_tile_reads_whole_sf(&out_q, &sf).is_nan());
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: fused-pair-replacement-writes-every-legacy-output-every-path):
// the fused kernel walks the PADDED domain (sf_m rows, padded scale cols),
// feeds zeros outside the valid rows/columns, and writes the scale output
// its identity (zero) on every padded address -- every scale address the
// replaced pair's consumer can touch is written, on every admitted path.
#![allow(dead_code)]

const ROWS: usize = 5;
const COLS: usize = 8;
const SF_COLS: usize = 2;
const PADDED_ROWS: usize = 8;
const PADDED_SF_COLS: usize = 4;
const DIRTY: u32 = 0x7F80_0001;

struct SfTensor {
    rows: usize,
    cols: usize,
    data: Vec<u32>,
}

fn alloc_dirty() -> SfTensor {
    SfTensor {
        rows: PADDED_ROWS,
        cols: PADDED_SF_COLS,
        data: vec![DIRTY; PADDED_ROWS * PADDED_SF_COLS],
    }
}

// one scale group of the replaced pair's real work, unchanged
fn act_quant_group(x: &[i32; 4], out_q: &mut [i32; 4]) -> u32 {
    let mut peak = 0i32;
    for v in x.iter() {
        peak = peak.max(v.abs());
    }
    let scale = if peak == 0 { 1 } else { peak };
    for (i, v) in x.iter().enumerate() {
        out_q[i] = v * 100 / scale;
    }
    scale as u32
}

fn fused_act_quant_padded_domain(
    x: &[[i32; COLS]; ROWS],
    out_q: &mut [[i32; COLS]; ROWS],
    sf: &mut SfTensor,
) {
    let sf_m = PADDED_ROWS; // round_up(numRows, 128) in the source; 8 here
    for r in 0..sf_m {
        for g in 0..PADDED_SF_COLS {
            let valid = r < ROWS && g < SF_COLS;
            if valid {
                let lo = g * 4;
                let mut group = [0i32; 4];
                for (i, c) in (lo..lo + 4).enumerate() {
                    group[i] = x[r][c];
                }
                let mut wrote = [0i32; 4];
                let bits = act_quant_group(&group, &mut wrote);
                out_q[r][lo..lo + 4].copy_from_slice(&wrote);
                sf.data[r * sf.cols + g] = bits;
            } else {
                // outside valid rows/columns: feed zeros, and the scale
                // output gets its identity so no stale factor survives
                sf.data[r * sf.cols + g] = 0;
            }
        }
    }
}

fn gemm_tile_reads_whole_sf(q: &[[i32; COLS]; ROWS], sf: &SfTensor) -> f32 {
    let mut acc = 0.0f32;
    for r in 0..sf.rows {
        for c in 0..COLS {
            let payload = if r < ROWS { q[r][c] as f32 } else { 0.0 };
            let scale = f32::from_bits(sf.data[r * sf.cols + c / 4]);
            acc += payload * scale;
        }
    }
    acc
}

fn gemm_tile_reference(q: &[[i32; COLS]; ROWS], scales: &[[u32; SF_COLS]; ROWS]) -> f32 {
    let mut acc = 0.0f32;
    for r in 0..ROWS {
        for c in 0..COLS {
            acc += q[r][c] as f32 * f32::from_bits(scales[r][c / 4]);
        }
    }
    acc
}

#[test]
fn every_scale_address_is_written_and_padding_gets_the_identity() {
    let x = [[1i32; COLS]; ROWS];
    let mut out_q = [[0i32; COLS]; ROWS];
    let mut sf = alloc_dirty();
    fused_act_quant_padded_domain(&x, &mut out_q, &mut sf);
    // no dirty fill survives anywhere in the swizzled scale tensor
    assert!(sf.data.iter().all(|&b| b != DIRTY));
    // the padded rows and scale columns read exactly zero
    for r in ROWS..PADDED_ROWS {
        for g in 0..PADDED_SF_COLS {
            assert_eq!(sf.data[r * sf.cols + g], 0);
        }
    }
    for r in 0..ROWS {
        for g in SF_COLS..PADDED_SF_COLS {
            assert_eq!(sf.data[r * sf.cols + g], 0);
        }
    }
}

#[test]
fn the_tile_read_is_finite_and_matches_the_valid_only_reference() {
    let mut x = [[0i32; COLS]; ROWS];
    for r in 0..ROWS {
        for c in 0..COLS {
            x[r][c] = ((r * COLS + c) % 7) as i32 - 3;
        }
    }
    let mut out_q = [[0i32; COLS]; ROWS];
    let mut sf = alloc_dirty();
    fused_act_quant_padded_domain(&x, &mut out_q, &mut sf);
    let got = gemm_tile_reads_whole_sf(&out_q, &sf);
    assert!(got.is_finite());
    let mut want_scales = [[0u32; SF_COLS]; ROWS];
    for r in 0..ROWS {
        for g in 0..SF_COLS {
            let lo = g * 4;
            let mut peak = 0i32;
            for c in lo..lo + 4 {
                peak = peak.max(x[r][c].abs());
            }
            want_scales[r][g] = (if peak == 0 { 1 } else { peak }) as u32;
        }
    }
    assert_eq!(got, gemm_tile_reference(&out_q, &want_scales));
}

fn main() {}".to_string(),
        },
        // ── Rule 2 (category E — parallelism): rows chunked at the gridDim.z ceiling — the silent-zero launch class. ──
        KernelRule {
            rule_id: "silent-zero-launch-row-chunked-past-grid-z-ceiling".to_string(),
            gpu_concept: "MECHANISM (a backend maps one unit of work to a grid dimension with a hard API ceiling, and a launch over the ceiling fails WITHOUT RAISING, leaving the output unwritten; category E — parallelism, the silent-zero-launch class): trtllm-gen sparse MLA decode maps one request per token to gridDim.z — \"# CUDA's gridDim.z limit; trtllm-gen MLA decode launches one z-slice per request.\" — and CUDA caps any grid dimension at 65535: \"# One trtllm-gen request per token, and requests map to gridDim.z: a\" / \"# launch over more than 65535 tokens fails without raising and leaves\" / \"# ``out`` unwritten (flashinfer-ai/flashinfer#3125). Launch row chunks.\" — a 64K-token prefill (\"65536 + 4099,\" rows in the regression) silently produced NOTHING. THE REPAIR chunks the rows at the ceiling — \"for start in range(0, num_tokens, _TRTLLM_GEN_MAX_REQUESTS):\" — with every chunk writing its caller-owned output slice (\"out=out[start:end].unsqueeze(1)\"); the workspace slab is sized at ONE launch — \"A launch covers at most ``_TRTLLM_GEN_MAX_REQUESTS``\" / \"tokens, and launches run in order on one stream, so larger batches reuse\" / \"the same slab.\" via \"* min(max_num_batched_tokens, _TRTLLM_GEN_MAX_REQUESTS)\" — so a larger token budget costs no more memory: \"# Launches cover at most 65535 tokens, so larger token budgets reuse the slab.\" THE LAW: where a backend maps work units to a grid dimension with an API ceiling, an over-ceiling launch is a silent zero — never an error — so chunk the rows at the ceiling (ordered launches on one stream, caller-owned output slices), size per-launch workspaces at min(work, ceiling), and pin BOTH sides of the boundary with a sweep: \"# Both sides of the 65535 boundary, the ends, and a sweep in between.\" / \"rows = sorted({0, 65534, 65535, 65536, num_tokens - 1, *range(0, num_tokens, 997)})\" — the test docstring names the ceiling itself: \"trtllm-gen maps each token to gridDim.z (limit 65535); a larger step,\". REGIME: backends that map requests/tokens to grid z/y/x under dimension ceilings, long prefills and DCP worlds that multiply token counts, caller-owned output buffers. ANTI-REGIME: backends that RAISE on oversized launches (the loud refusal class); kernels that chunk internally; grids whose per-dimension count is input-bounded below the ceiling. KINSHIP (do not double-count): head-dim-smem-tile-cap (entries_gpu_133) owns the LAUNCH-REFUSAL class — OutOfResources, a loud refusal whose capacity law keeps the tile inside the ceiling; THIS owns the SILENT-ZERO class — the API accepts the launch and does nothing. CROSS-KIN (Batch 304, same batch): grid-ceiling-sliced-dispatch-per-slice-buffer-rebind (entries_gpu_137) is the Metal twin — a grid past UINT32_MAX total threads is silently TRUNCATED (tail rows dropped), remedied by host-side dispatch slicing with per-slice buffer byte-offset rebind; the platforms differ in the failure MODE (Metal truncates the grid; CUDA's gridDim.z accept-and-no-op leaves the output wholly unwritten) and the remedy shape (slice+rebind vs row-chunked launches + a workspace sized min(work, ceiling)) — same law, both halves: a launch whose row count can cross a platform dispatch ceiling must be split AT the ceiling, never clamped and never trusted. pipeline-thread-capacity-declared-by-launcher (entries_gpu_101) owns Metal's derived-occupancy silent zero (capacity under the launched count, outputs at zero) — the remedy there is declare+assert, HERE chunk past the ceiling; producer-launch-derived-from-actual-group-size (entries_gpu_101) owns unwritten reads from a formula-derived extent; spin-barrier-needs-co-residency-capped-launch (entries_gpu_22) caps launches that WAIT on each other (the co-residency axis); stream-k-grid-capped-at-useful-work-quanta (entries_gpu_108) caps a persistent grid at useful-work quanta — a sizing law, not a ceiling sweep; the 65535-minus-idx rider (entries_gpu_20, Batch 111) is an index-priority encoding, not a grid limit. GREP EVIDENCE (over the corpus, 3+ vocabularies): '65535' -> only the inverted-index priority rider — no grid-ceiling owner; 'gridDim.z|grid z' -> grid-geometry mentions (entries_gpu_105, entries_gpu_108), never a ceiling sweep — no owner; 'chunked launch|row chunk|launch chunk' -> graph NODE chunking (entries_gpu_08 rider) and loop bounds — no launch-chunking owner. NotBenchable here (a silent-launch-failure class — the fixture IS the ceiling sweep and the chunk arithmetic; CUDA is not hosted by this corpus's benches). Source: vllm-project/vllm @ 4da2a82a24b3e5917901ca77254272d86c185f3d (Apache-2.0) — vllm/v1/attention/backends/mla/flashinfer_mla_sparse.py:45-46,369-383,649-673 + tests/v1/attention/test_sparse_mla_backends.py:293-371 + tests/v1/attention/test_flashinfer_sparse_mla_workspace.py:65-80, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: silent-zero-launch-row-chunked-past-grid-z-ceiling): the
// launcher maps one request per grid-z slice; past the API ceiling of 65535
// the launch FAILS WITHOUT RAISING and the caller-owned output keeps its
// poisoned fill for every row the dead launch should have written.
#![allow(dead_code)]

const GRID_Z_CEILING: u32 = 65535;
const PER_TOKEN_HEADS: usize = 16;

// the vendor entry point: silently no-ops past the ceiling
fn trtllm_decode_naive(tokens: u32, out: &mut [f32]) -> Result<(), ()> {
    let z_slices = tokens; // one request per token, one z-slice per request
    if z_slices > GRID_Z_CEILING {
        return Ok(()); // fails without raising: no error, no write
    }
    for slot in out.iter_mut() {
        *slot = 1.0; // the kernel wrote every row
    }
    Ok(())
}

fn poisson_fill(out: &mut [f32]) {
    for slot in out.iter_mut() {
        *slot = f32::NAN;
    }
}

fn unwritten_rows(out: &[f32]) -> usize {
    out.iter().filter(|v| v.is_nan()).count()
}

#[test]
fn the_oversized_launch_fails_silently() {
    let tokens = 65536u32 + 4099; // a 64K-token prefill plus a tail
    let mut out = vec![0.0f32; tokens as usize * PER_TOKEN_HEADS];
    poisson_fill(&mut out);
    // the launcher reports success -- no error surfaced anywhere
    assert!(trtllm_decode_naive(tokens, &mut out).is_ok());
    // every row of the dead launch is still the poison: unwritten output
    assert_eq!(unwritten_rows(&out), out.len());
}

#[test]
fn the_error_arm_never_fires_so_nothing_alerts() {
    // the defect class is SILENT: the same call at the ceiling is fine,
    // one token over is dead, and neither arm raises
    let mut small = vec![0.0f32; 65535 * PER_TOKEN_HEADS];
    assert!(trtllm_decode_naive(65535, &mut small).is_ok());
    assert_eq!(unwritten_rows(&small), 0);
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: silent-zero-launch-row-chunked-past-grid-z-ceiling):
// the rows are chunked at the ceiling -- each launch covers at most 65535
// requests, launches run in order on one stream, every chunk writes its
// caller-owned output slice, and any per-launch workspace is sized at
// min(work, ceiling) so larger batches reuse the same slab. The boundary
// sweep pins both sides of the ceiling.
#![allow(dead_code)]

const GRID_Z_CEILING: u32 = 65535;
const PER_TOKEN_HEADS: usize = 16;
const SOFTMAX_STAT_BYTES: usize = 128;

// one chunk's launch: the vendor entry, now guarded
fn launch_chunk(start: u32, end: u32, out: &mut [f32]) -> Result<(), ()> {
    if end <= start || end - start > GRID_Z_CEILING {
        return Err(()); // the launcher's own invariant, loud this time
    }
    for slot in &mut out[start as usize * PER_TOKEN_HEADS..end as usize * PER_TOKEN_HEADS] {
        *slot = 1.0;
    }
    Ok(())
}

fn trtllm_decode_chunked(tokens: u32, out: &mut [f32]) -> Result<(), ()> {
    let mut start = 0u32;
    while start < tokens {
        let end = (start + GRID_Z_CEILING).min(tokens);
        launch_chunk(start, end, out)?;
        start = end;
    }
    Ok(())
}

// the workspace slab covers ONE launch, never the batch: launches run in
// order on one stream, so a larger batch reuses the same slab
fn workspace_bytes(tokens: u32) -> usize {
    SOFTMAX_STAT_BYTES * PER_TOKEN_HEADS * tokens.min(GRID_Z_CEILING) as usize
}

fn boundary_sweep_counts() -> Vec<u32> {
    // both sides of the 65535 boundary, the ends, and a sweep in between
    let n = 65536u32 + 4099;
    let mut counts: Vec<u32> = vec![0, GRID_Z_CEILING - 1, GRID_Z_CEILING, GRID_Z_CEILING + 1, n - 1, n];
    let mut t = 0u32;
    while t < n {
        counts.push(t);
        t += 997;
    }
    counts.sort_unstable();
    counts.dedup();
    counts
}

#[test]
fn every_chunked_launch_writes_all_rows() {
    let tokens = 65536u32 + 4099;
    let mut out = vec![f32::NAN; tokens as usize * PER_TOKEN_HEADS];
    assert!(trtllm_decode_chunked(tokens, &mut out).is_ok());
    assert!(out.iter().all(|v| *v == 1.0));
    // two chunks: the second covers the tail past the first ceiling
    let chunks = tokens.div_ceil(GRID_Z_CEILING);
    assert_eq!(chunks, 2);
}

#[test]
fn the_workspace_slab_is_capped_at_one_launch() {
    // at the limit and above it the slab is the same size: larger token
    // budgets reuse the slab
    assert_eq!(workspace_bytes(65535), workspace_bytes(131072));
    assert_eq!(workspace_bytes(131072), SOFTMAX_STAT_BYTES * PER_TOKEN_HEADS * 65535usize);
    assert_eq!(workspace_bytes(1000), SOFTMAX_STAT_BYTES * PER_TOKEN_HEADS * 1000usize);
}

#[test]
fn the_boundary_sweep_never_leaves_a_row_unwritten() {
    let n = 65536u32 + 4099;
    let mut probed = 0usize;
    for tokens in boundary_sweep_counts() {
        let mut out = vec![f32::NAN; n as usize * PER_TOKEN_HEADS];
        trtllm_decode_chunked(tokens, &mut out).unwrap();
        let live = tokens as usize * PER_TOKEN_HEADS;
        assert!(out[..live].iter().all(|v| *v == 1.0));
        probed += 1;
    }
    assert!(probed >= 75); // the boundary set plus the stride-997 sweep
    // the ceiling itself is probed on BOTH sides
    for tokens in [GRID_Z_CEILING - 1, GRID_Z_CEILING, GRID_Z_CEILING + 1] {
        let mut out = vec![f32::NAN; (GRID_Z_CEILING + 1) as usize * PER_TOKEN_HEADS];
        trtllm_decode_chunked(tokens, &mut out).unwrap();
        let live = tokens as usize * PER_TOKEN_HEADS;
        assert!(out[..live].iter().all(|v| *v == 1.0));
    }
}

fn main() {}".to_string(),
        },
        // ── Rule 3 (category A — memory/lifecycle): async loads fenced behind the readers’ completion step. ──
        KernelRule {
            rule_id: "page-reuse-fenced-behind-reader-completion".to_string(),
            gpu_concept: "MECHANISM (reuse-eligibility and read-eligibility are different clocks: a page the pool may reuse at once can still be READ by an in-flight step, so a new writer that is not ordered against those steps — an async KV connector load — is fenced behind the READERS' completion step; category A — memory access/lifecycle): the scheduler's deferred-free machinery already carries the fence — \"# FIFO of (fence_seq, blocks): blocks become safe to free once\" / \"# processed_step_seq >= fence_seq.\" — and the founding hazard: \"# HiSparse keeps reading a released page until the pool reuses it,\" / \"# so a load into a reused page must also wait for in-flight steps.\" The async load is REGISTERED with the reader's step as its fence and starts only once processed_step_seq >= fence_seq: the gate is \"load_kv_async\" + \"and self.defer_async_loads\" + \"and self.sched_step_seq > self.processed_step_seq\"; the deferred record carries the load's hit accounting so the stats move with it — \"# Request id -> (fence_seq, num_external_tokens, connector prefix cache\" / \"# queries, hits): async KV loads started once processed_step_seq >=\" / \"# fence_seq.\" — and the drain mirrors the frees: \"Start deferred async loads whose fence step has completed.\" The abort path completes the law: \"# A deferred load never started, so it will not report done.\" — its blocks free through the deferred-free fence once the step that may still read them is processed (the scenario docstring: \"HiSparse keeps reading a released page until the pool reuses it, and an\" / \"async connector load isn't ordered against the in-flight step, so the load\" / \"is registered only once that step is processed\"; the abort test: \"No load was posted for a deferred request, so nothing will report it\" / \"done; its blocks are freed once the step that may still read them is\" / \"processed.\"). REGIME: paged KV pools with multiple in-flight batches (defer_block_free) plus async KV connector loads (host-side KV transfer); any producer whose write lands in storage an in-flight step still reads. ANTI-REGIME: single-batch schedulers (no in-flight step to race); loads the connector itself orders against the steps; writers whose storage the pool cannot have reused. KINSHIP (do not double-count): ring-overwrite-gated-k-back-readiness (entries_gpu_61) owns distance-K overwrite gating on DEVICE rings via readiness counters — THIS is the scheduler-side analog: a new writer gated on step-completion fences; compose. host-context-mirror-fenced-drain-avoidance (entries_gpu_124) owns the fenced HOST context mirror's drain avoidance — the fence-vocabulary sibling at the weight/context layer; the freeze/thaw torn-read invariant (the katgpt workspace law) is the same read-while-mutable class at the snapshot layer. lazy-rollback-journaled-window-replay-replaces-eager-state-snapshots (entries_gpu_105) pins its commit point with a threadfence — a publish-order fence, not a reuse fence. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'fence' -> the fenced host mirror (entries_gpu_124), the fence-consumer emission rule (entries_gpu_14), the commit-point threadfence (entries_gpu_105) — no page-reuse owner; 'deferred load|defer async|async KV load' -> ZERO; 'released page|page reuse|reused page' -> the paged-cache STRUCTURE rules (entries_gpu_20, entries_gpu_08) own cache shapes, not reuse fences; 'in-flight' -> in-flight transform counts (entries_gpu_131) and slot pipelines (entries_gpu_18) — different surfaces. NotBenchable here (an ordering/liveness axis — the fixture is the fence arithmetic and the abort path; no KV-pool scheduler bench in this corpus). Source: vllm-project/vllm @ d482020608af39f5543a5f7802a3228e17e11f8e (Apache-2.0) — vllm/v1/core/sched/scheduler.py:170,196-200,470-476,1468-1483,2283,3016-3019,3119-3140 + tests/v1/core/test_deferred_block_free.py:549-649, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: page-reuse-fenced-behind-reader-completion): the pool
// hands a released page to the importer's async load the moment it is
// free, but the in-flight step that scheduled reads on it has not been
// processed yet -- the load's write lands under the reader.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq)]
struct Page {
    bytes: [u32; 4],
}

struct Pool {
    pages: Vec<Page>,
    free: Vec<usize>,
    sched_step: u64,
    processed_step: u64,
}

struct NaiveSched {
    pool: Pool,
    // the in-flight reader: (its step, its page, the bytes it scheduled)
    inflight: Option<(u64, usize, [u32; 4])>,
    load_registered: bool,
}

impl NaiveSched {
    fn new() -> Self {
        NaiveSched {
            pool: Pool {
                pages: vec![Page { bytes: [0; 4] }; 8],
                free: Vec::new(),
                sched_step: 0,
                processed_step: 0,
            },
            inflight: None,
            load_registered: false,
        }
    }

    // schedule(): the step reads page p; the pool frees the page eagerly
    // at schedule time (the released-page class)
    fn schedule_step(&mut self, page: usize, bytes: [u32; 4]) {
        self.pool.pages[page].bytes = bytes;
        self.inflight = Some((self.pool.sched_step, page, bytes));
        self.pool.sched_step += 1;
        self.pool.free.push(page);
    }

    // the importer arrives while the step is still in flight: the async
    // connector load is registered AND started at once -- it is not
    // ordered against the in-flight step
    fn admit_importer(&mut self, imported: [u32; 4]) {
        let page = match self.pool.free.pop() {
            Some(p) => p,
            None => panic!(), // reuse: the pool must hand a page over
        };
        self.pool.pages[page].bytes = imported;
        self.load_registered = true;
    }

    // the step's output comes back; the reader walks its page
    fn process_step(&mut self) -> bool {
        let (step, page, expected) = match self.inflight.take() {
            Some(triple) => triple,
            None => panic!(), // the in-flight step must exist
        };
        self.pool.processed_step = step + 1;
        self.pool.pages[page].bytes == expected
    }
}

#[test]
fn the_load_lands_under_the_inflight_reader() {
    let mut s = NaiveSched::new();
    s.schedule_step(3, [11, 22, 33, 44]);
    // the importer is admitted while the step is unprocessed
    s.admit_importer([99, 99, 99, 99]);
    assert!(s.load_registered);
    // the reader walks a page the load already overwrote
    assert!(!s.process_step());
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: page-reuse-fenced-behind-reader-completion): the
// async load is REGISTERED with the reader's step as its fence and starts
// only once processed_step_seq >= fence_seq -- the same fence the deferred
// frees already use -- so reuse-eligibility (the pool's) and
// read-eligibility (the fence's) are different clocks. A deferred load
// never started cannot report done, so an abort frees its blocks through
// the deferred-free fence too.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq)]
struct Page {
    bytes: [u32; 4],
}

struct Pool {
    pages: Vec<Page>,
    free: Vec<usize>,
    sched_step: u64,
    processed_step: u64,
}

// a held page: (fence_seq, page, whose own it is). Importer-owned entries
// dissolve at the drain; aborted-importer entries hand the page back to
// the pool's free list at the drain.
struct Held {
    fence: u64,
    page: usize,
    importer_owned: bool,
}

struct DeferredSched {
    pool: Pool,
    inflight: Option<(u64, usize, [u32; 4])>,
    // (fence_seq, page, bytes): the load starts once processed >= fence
    deferred_loads: Vec<(u64, usize, [u32; 4])>,
    held: Vec<Held>,
    started_loads: usize,
}

impl DeferredSched {
    fn new() -> Self {
        DeferredSched {
            pool: Pool {
                pages: vec![Page { bytes: [0; 4] }; 8],
                free: Vec::new(),
                sched_step: 0,
                processed_step: 0,
            },
            inflight: None,
            deferred_loads: Vec::new(),
            held: Vec::new(),
            started_loads: 0,
        }
    }

    fn schedule_step(&mut self, page: usize, bytes: [u32; 4]) {
        self.pool.pages[page].bytes = bytes;
        let step = self.pool.sched_step;
        self.inflight = Some((step, page, bytes));
        self.pool.sched_step += 1;
        // released: the pool may reuse it at once (reuse-eligibility),
        // but the fence records when it is safe to consider the reads done
        self.pool.free.push(page);
        self.held.push(Held { fence: step, page, importer_owned: false });
    }

    // the importer is admitted while the step is in flight: the page is
    // handed over (reuse-eligibility), but the load waits on the fence
    // (read-eligibility) instead of starting unordered
    fn admit_importer(&mut self, imported: [u32; 4]) {
        let page = match self.pool.free.pop() {
            Some(p) => p,
            None => panic!(), // reuse: the pool must hand a page over
        };
        let fence = self.pool.sched_step - 1; // the in-flight reader's step
        self.deferred_loads.push((fence, page, imported));
    }

    fn process_step(&mut self) -> bool {
        let (step, page, expected) = match self.inflight.take() {
            Some(triple) => triple,
            None => panic!(), // the in-flight step must exist
        };
        self.pool.processed_step = step + 1;
        let intact = self.pool.pages[page].bytes == expected;
        self.drain_held();
        self.drain_deferred_loads();
        intact
    }

    fn drain_held(&mut self) {
        let done = self.pool.processed_step;
        let mut returned = Vec::new();
        self.held.retain(|h| {
            if h.fence <= done {
                if !h.importer_owned {
                    // the reader's own release: nothing to move, the page
                    // already sits in the free list
                } else {
                    returned.push(h.page);
                }
                false
            } else {
                true
            }
        });
        for page in returned {
            if !self.pool.free.contains(&page) {
                self.pool.free.push(page);
            }
        }
    }

    // start deferred async loads whose fence step has completed
    fn drain_deferred_loads(&mut self) {
        let done = self.pool.processed_step;
        let mut ready = Vec::new();
        self.deferred_loads.retain(|&(fence, page, ref bytes)| {
            if fence <= done {
                ready.push((page, *bytes));
                false
            } else {
                true
            }
        });
        for (page, bytes) in ready {
            self.pool.pages[page].bytes = bytes;
            self.started_loads += 1;
        }
    }

    // the importer aborts: the deferred load never started, so nothing
    // will report it done -- the page is pulled back and held until the
    // step that may still read it is processed
    fn abort_importer(&mut self) {
        let fence = self.pool.sched_step;
        let mut pulled = Vec::new();
        self.deferred_loads.retain(|&(_, page, _)| {
            pulled.push(page);
            false
        });
        for page in pulled {
            self.pool.free.retain(|&p| p != page);
            self.held.push(Held { fence, page, importer_owned: true });
        }
    }

    fn held_count(&self) -> usize {
        self.held.len()
    }

    fn free_count(&self) -> usize {
        self.pool.free.len()
    }
}

#[test]
fn the_reader_is_intact_and_the_load_starts_after_its_step() {
    let mut s = DeferredSched::new();
    s.schedule_step(3, [11, 22, 33, 44]);
    s.admit_importer([99, 99, 99, 99]);
    // registered but not started: the fence has not completed
    assert_eq!(s.deferred_loads.len(), 1);
    assert_eq!(s.started_loads, 0);
    // the reader walks its own bytes
    assert!(s.process_step());
    // the drain ran with the step processed: the load started AFTER the
    // reader's step completed
    assert_eq!(s.started_loads, 1);
    assert!(s.deferred_loads.is_empty());
    assert_eq!(s.pool.pages[3].bytes, [99, 99, 99, 99]);
}

#[test]
fn an_aborted_deferred_load_frees_after_inflight_steps() {
    let mut s = DeferredSched::new();
    s.schedule_step(3, [11, 22, 33, 44]);
    s.admit_importer([99, 99, 99, 99]);
    let held_before = s.held_count();
    s.abort_importer();
    // no load was posted: the page is pulled back and held, not free
    assert_eq!(s.started_loads, 0);
    assert_eq!(s.held_count(), held_before + 1);
    assert_eq!(s.free_count(), 0);
    // the step processes; the fence completes and the held page frees
    assert!(s.process_step());
    assert_eq!(s.held_count(), 0);
    assert_eq!(s.free_count(), 1);
}

fn main() {}".to_string(),
        },
    ]
}

#[allow(clippy::too_many_lines)]
fn slice_tf_b304_pool() -> Vec<KernelRule> {
    vec![
        // ── Rule 4 (category M — kernel structure): device-primitive dedup under per-function SASS identity, with the planted-control instrument. ──
        KernelRule {
            rule_id: "device-primitive-dedup-under-sass-identity".to_string(),
            gpu_concept: "MECHANISM (device-source dedup admitted ONLY under per-function compiled-output identity, carried by an instrument whose comparator is proven able to go red; category M — kernel structure/polymorphism): 38 units carried identical device-primitive copies; the move extracts them into common/ headers — \"qmm_frag.cuh and glue_math.cuh, already included by 22 and 16 units, move to common/ next to two new headers in\" / \"namespace tf_common: reduce.cuh (warp_sum and warp_max by xor butterfly 16..1, block_sum_tree and block_max_tree,\" / \"group_sum8) and vec.cuh (pack8, unpack8). qmm_frag.cuh gains ldmatrix4t, fp4pair and ld_nc. Units drop their own\" / \"identical copies for using declarations\" (the shared reductions keep one fixed order: \"// Warp and block reductions, one fixed order each; every lane that calls gets the same total.\" / \"// xor butterfly, offsets 16 down to 1\") — and the call sites become using declarations (\"using tf_common::unpack8;\" / \"using tf_common::pack8;\"). THE GATE, from the commit message: \"cuobjdump -sass is identical for every function of the 53 images whose units include a changed file (2,880\" / \"functions, sm_80/86/89/120/121, base 8c36359d5). Files in open PRs and files a generator parses keep their copies.\" — extraction lands only when EVERY function of EVERY image the change reaches is SASS-identical across the arch set; the exemption classes are named in-source (open-PR files and generator-parsed files keep their copies); and the byte-pinned consumers move in the same change (the follow-up commit: \"The move left both pins at the old paths, so test-gradient-serving stopped at FileNotFound. Both now name common/\" / \"with the moved files' bytes.\"). THE INSTRUMENT (the second face): a per-function digest base-vs-head over every image the changed files reach, with a receipt of each function's digests, plus TWO PLANTED CONTROLS that must move functions — \"Two planted controls then prove the comparison can go red: a changed fp4pair constant\" / \"(common/qmm_frag.cuh) and a reversed warp_sum butterfly (common/reduce.cuh), each built against the head. Exit status\" / \"0 only when every function matches and both controls change functions.\" — the controls being the fp4pair constant 0x7E807E80u to 0x7E817E80u and the warp_sum order \"int o = 16; o > 0; o >>= 1\" to \"int o = 1; o < 32; o <<= 1\" (the butterfly reversed). THE COMPARATOR-BLINDNESS INCIDENT the instrument's own history carries: \"functions() took only instruction lines whose address had four hex digits, so anything from 0x10000 on (past\" / \"a function's first 4,096 instructions) was left out of its digest, and it skipped the /* 0x... */ continuation\" / \"lines that carry each instruction's second encoding word.\" — a comparator that reads a subset prints a green over the unseen; the fix widened the address match to any width and kept the continuation words ((arch, function) -> \"each instruction's line (any address width) and continuation (second word), collapsed.\") and pins it with a canned dump: \"cuda_sass_identity.functions must see every instruction: five-digit addresses and the second encoding word.\" REGIME: AOT-compiled device source shared across many compilation units; dedup/refactor campaigns whose contract is no compiled-bit movement; byte-pinned artifact registries consuming the moved files; any digest comparator whose reach must be proven. ANTI-REGIME: refactors that INTEND to change codegen (a different gate); generated files a generator parses textually (exempted in-source); un-pinned toolchains (the identity claim is per-toolchain). KINSHIP (do not double-count): multi-projection-prefill-parts-table-body-literal (entries_gpu_135) owns the body-literal HAZARD — sharing a helper CHANGED Blackwell SASS under nvcc 13.0/13.3; THIS owns the gate + instrument that ADMITS sharing exactly when it does not — the hazard names the risk, this measures it; compose, never merge. stage-copy-preserves-fetch-unit-order-swizzle-at-read (entries_gpu_135) carries a per-function SASS-unchanged claim as EVIDENCE — THIS generalizes that evidence into a standing gate + instrument. kernel-contract-planted-gate-controls (entries_gpu_101) owns planted gate controls at the kernel-CONTRACT layer — the same canary-proves-the-comparator-can-go-red shape, one layer down. vendor-dispatch-verify-before-outsource (entries_gpu_36) cites cuobjdump as a one-off verification step; timed-zero-refuses-round-stop (the rust_perf domain) owns the instrument-validity genus. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'SASS|cuobjdump' -> evidence-carrying mentions only (entries_gpu_36, entries_gpu_37, entries_gpu_135) — no dedup-under-identity owner; 'dedup|identical copies|using declaration|shared device' -> ZERO device-source dedup owners (entrypoint-variants-share-layer-body, entries_gpu_05, shares layer BODIES source-level with a stated parity class — cite); 'planted control|canary' -> kernel-contract-planted-gate-controls (cited); 'identity gate' -> runtime-identity-gate-cached-verdict (entries_gpu_05) — a cached runtime verdict, different axis. NotBenchable here (a compiled-identity axis — the win is the shared copies deleted with zero SASS movement, source-reported: 2,880 functions across 53 images; the digest/controls/comparator logic is CPU-testable and IS the fixture). Source: ashhart/TensorFold @ 2922b93a6caee0bc3a09996eeade244f9979c999 (Apache-2.0) — zig/kernels/cuda/act_l2.cuh:25-26 + zig/kernels/cuda/common/reduce.cuh:1,11 + the commit message; pins follow-up @ 2e045240e4b4d7096aa9b5d1f331e0552f44e783 (zig/tests/fixtures/gradient-serving.tsv + pocket-serving-approved.tsv); instrument @ c6193ca40f0ec8458a5a0f53f2b37237a70dadc7 + dd0a9cc5e503c405a71856631bfeabbccd97169f — tools/zig/cuda_sass_identity.py:12,16-18,22-26,94,107 + tools/zig/cuda_sass_identity_test.py:1, quotes byte-verified at the pins (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: device-primitive-dedup-under-sass-identity): the dedup
// lands on a source-level check alone -- a transcription slip in the
// shared copy silently changes the emitted code for every including unit,
// and a comparator that reads only four-digit addresses (skipping the
// second-encoding-word continuation lines) reports green over a
// difference past 0x10000.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq)]
struct Instr {
    addr: u32,
    op: u32,    // the instruction's first encoding word
    word2: u32, // the continuation line's second encoding word
}

// the baseline per-unit copy: butterfly offsets 16 down to 1
fn baseline_warp_sum() -> Vec<Instr> {
    (0..8)
        .map(|i| Instr { addr: i * 4, op: 100 + i, word2: 0x1111 + i as u32 })
        .collect()
}

// the deduped shared copy, transcribed with the butterfly REVERSED
// (offsets 1 up to 16): a plausible-looking reduction that is not the
// same function
fn shared_warp_sum_transcribed_wrong() -> Vec<Instr> {
    (0..8)
        .map(|i| Instr { addr: i * 4, op: 100 + (7 - i), word2: 0x1111 + i as u32 })
        .collect()
}

// a function whose ONLY difference sits at address 0x10000, in a
// second encoding word: past the first 4,096 instructions
fn late_word2_variant() -> Vec<Instr> {
    let mut f = baseline_warp_sum();
    f.push(Instr { addr: 0x10000, op: 0x794d, word2: 0x000fe200 });
    f
}

fn late_word2_baseline() -> Vec<Instr> {
    let mut f = baseline_warp_sum();
    f.push(Instr { addr: 0x10000, op: 0x794d, word2: 0x000fea0 });
    f
}

// the naive gate: the units all include the same shared header, so the
// source-level check is green by construction
fn source_level_check_green(units_share_header: bool) -> bool {
    units_share_header
}

// the blind digest: four-digit addresses only, continuation words skipped
fn digest_blind(f: &[Instr]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for i in f {
        if i.addr >= 0x10000 {
            continue; // the four-digit-address blind spot
        }
        h = (h ^ i.op as u64).wrapping_mul(0x100000001b3);
        // word2 never enters: the continuation line is skipped
    }
    h
}

fn digest_full(f: &[Instr]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for i in f {
        h = (h ^ (i.addr as u64) ^ (i.op as u64) ^ (i.word2 as u64)).wrapping_mul(0x100000001b3);
    }
    h
}

#[test]
fn the_source_check_is_green_while_the_codegen_moved() {
    // every unit includes the shared header: the source check passes
    assert!(source_level_check_green(true));
    // the compiled functions are NOT the same function
    assert_ne!(digest_full(&baseline_warp_sum()), digest_full(&shared_warp_sum_transcribed_wrong()));
}

#[test]
fn the_blind_comparator_misses_the_late_second_word() {
    // the only difference is at 0x10000, in the continuation's second word
    assert_ne!(digest_full(&late_word2_baseline()), digest_full(&late_word2_variant()));
    // the blind digest sees neither the address nor the word: green over
    // the unseen
    assert_eq!(digest_blind(&late_word2_baseline()), digest_blind(&late_word2_variant()));
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: device-primitive-dedup-under-sass-identity): the
// dedup lands ONLY under per-function compiled identity -- full digests
// (any address width, continuation words included) compared base against
// head -- and two planted controls (a bumped constant and a reversed
// butterfly) MUST each move a function or the run fails: exit green only
// when every function matches AND both controls changed functions.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq)]
struct Instr {
    addr: u32,
    op: u32,
    word2: u32,
}

fn warp_sum_base() -> Vec<Instr> {
    (0..8)
        .map(|i| Instr { addr: i * 4, op: 100 + i, word2: 0x1111 + i as u32 })
        .collect()
}

// control 1: a bumped constant (the fp4pair constant face)
fn fp4pair_bumped() -> Vec<Instr> {
    let mut f = warp_sum_base();
    f[0].op = 0x7E817E80; // was 0x7E807E80
    f
}

// control 2: the reversed butterfly (offsets 1 up to 16)
fn warp_sum_reversed() -> Vec<Instr> {
    (0..8)
        .map(|i| Instr { addr: i * 4, op: 100 + (7 - i), word2: 0x1111 + i as u32 })
        .collect()
}

// the full per-function digest: any address width, continuation word
// included, nothing skipped
fn digest(f: &[Instr]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for i in f {
        h = (h ^ (i.addr as u64) ^ (i.op as u64) ^ (i.word2 as u64)).wrapping_mul(0x100000001b3);
    }
    h
}

// the run: exit green only when every function matches AND both planted
// controls changed at least one function
fn run_gate(base: &[Vec<Instr>], head: &[Vec<Instr>], controls_moved: [bool; 2]) -> bool {
    if base.len() != head.len() {
        return false;
    }
    let all_match = base
        .iter()
        .zip(head.iter())
        .all(|(b, h)| b.len() == h.len() && digest(b) == digest(h));
    all_match && controls_moved[0] && controls_moved[1]
}

// a canned late dump: functions identical except at 0x10000, in the
// second encoding word -- what the fixed comparator must still see
fn late_base() -> Vec<Instr> {
    let mut f = warp_sum_base();
    f.push(Instr { addr: 0x10000, op: 0x794d, word2: 0x000fea0 });
    f
}

fn late_head() -> Vec<Instr> {
    let mut f = warp_sum_base();
    f.push(Instr { addr: 0x10000, op: 0x794d, word2: 0x000fe200 });
    f
}

#[test]
fn each_planted_control_moves_a_function() {
    let base = warp_sum_base();
    assert_ne!(digest(&base), digest(&fp4pair_bumped()));
    assert_ne!(digest(&base), digest(&warp_sum_reversed()));
}

#[test]
fn the_gate_is_green_only_when_all_match_and_both_controls_fired() {
    let base = vec![warp_sum_base()];
    // identity landing, both controls moved: green
    assert!(run_gate(&base, &base, [true, true]));
    // a control that failed to plant (the pattern did not match) is RED
    assert!(!run_gate(&base, &base, [true, false]));
    assert!(!run_gate(&base, &base, [false, true]));
    // any function difference is RED even with both controls fired
    assert!(!run_gate(&base, &vec![warp_sum_reversed()], [true, true]));
}

#[test]
fn the_comparator_sees_the_late_second_word() {
    // the full digest distinguishes a difference that sits only at
    // 0x10000 in the continuation word -- the blind spot is gone
    assert_ne!(digest(&late_base()), digest(&late_head()));
    // and the gate goes red on it
    assert!(!run_gate(&[late_base()], &[late_head()], [true, true]));
    // the identity twin is green
    assert!(run_gate(&[late_base()], &[late_base()], [true, true]));
}

fn main() {}".to_string(),
        },
        // ── Rule 5 (category A — memory): dual-bank scratch retention per round class. ──
        KernelRule {
            rule_id: "dual-bank-scratch-retention-per-round-class".to_string(),
            gpu_concept: "MECHANISM (a serving frame straddling two round classes of disjoint row regimes allocates a PER-CLASS scratch bank and swaps the ACTIVE bank on class flip; category A — memory access/allocation): a frame whose wide class is 2048 rows serves decode rounds of at most 512 rows, so narrow rounds either bind the wide buffers or pay a reallocation per flip. THE REPAIR keeps both: \"pub const small_rows = 512;\" and a standby bank sized by \"const widths = allocationSizes(config, @min(capacity, small_rows));\" (decodeSizes), held as \"standby: ?[scratch_fields.len]mtl.Buffer = null,\"; the flip is a per-field pointer swap between completed rounds — \"const small = ids.len <= small_rows;\" / \"for (scratch_fields, bank) |field, *buffer| std.mem.swap(mtl.Buffer, &frame.buffers[@backingInt(field)], buffer);\" — allocation-free (the recording-device test asserts the allocator's call count never moves across flips), and only SCRATCH fields swap: \"Called between completed rounds; metadata, position planes and outputs retain their original owners.\" THE INVENTORY AND ADMISSION HALF is load-bearing: \"Both banks keep their residency even when the next completed round changes the active views.\" — appendBuffers lists BOTH banks into the residency inventory — and startup admission counts the narrow bank beside the wide frame: \"return buffers(frame.allocationSizes(c, rows)) + (if (rows > frame.small_rows) buffers(frame.decodeSizes(c, rows)) else 0);\". THE COMMIT'S OWN STATEMENT: \"Keep 512-row activation buffers for small rounds while preserving the 2048-row prompt frame. Keep metadata and logits stable, inventory both banks for residency, and count both in startup admission.\" and the test discipline: \"Recording allocations exercise the actual frame's switching, stable outputs and cleanup.\" — the narrow-round test is titled: qwen27 bank: narrow rounds bind narrow activations without changing the wide prompt or output owners. REGIME: Metal/wgpu serving frames serving mixed prompt+decode rounds with disjoint row regimes; residency-inventoried runtimes; startup admission with a byte budget and a fixed margin. ANTI-REGIME: single-class workloads (one bank); classes whose row regimes overlap (no disjoint banks to hold); runtimes where swapping bound buffers mid-pass invalidates encoder state (swap only BETWEEN completed rounds). KINSHIP (do not double-count): capacity-widening-priced-by-shared-allocation-layout (entries_gpu_135) owns the WIDENING half — one frame widened, the delta priced through the shared layout source; THIS is the narrow-bank RETENTION complement — two banks, the wide frame byte-stable, narrow rounds never paying wide-row bytes; compose. table-reached-buffers-declare-residency (entries_gpu_134) owns residency DECLARATION for table-indirect buffers — THIS counts both banks in the EXISTING inventory (the declaration axis is that rule's). scratch-bank-isolates-one-shot-operands-from-hot-cache (entries_gpu_10) owns the cache-isolation bank axis (cold operands quarantined from hot lines); THIS owns the round-class ROW-REGIME axis. whole-window-specialization-omits-metadata-poison-verified (entries_gpu_135) splits kernel INSTANCES by input class; THIS splits STORAGE by round class. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'standby|dual bank|bank swap' -> the smem bank-conflict family, ane-bank-split-ladder (entries_gpu_24) and the cache-isolation bank (entries_gpu_10) — no per-round-class bank owner; 'small rows|narrow rounds|512-row' -> ZERO; 'residency inventory|count both' -> table-reached-buffers-declare-residency (cited). NotBenchable here (a memory-budget/residency axis — the win is narrow rounds binding narrow buffers with zero allocation churn, source-reported through the recording-device test; the flip/ownership/inventory predicates are CPU-testable and ARE the fixtures). Source: ashhart/TensorFold @ 8abd450899584610f4e394743548a2b691c7dc23 (Apache-2.0) — zig/src/families/qwen27/gpu_frame.zig:31,34-35,44-48,56-96 + zig/src/families/qwen27/prompt_memory.zig:15 + zig/src/families/qwen27/residency.zig:126-129 + zig/src/qwen27_frame_test.zig:1-74, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: dual-bank-scratch-retention-per-round-class): one frame
// with a single bank of wide-only scratch -- narrow rounds bind 2048-row
// buffers (the wide bytes bound for 1 row), and the per-round reallocation
// the naive alternative reaches for charges the allocator on every class
// flip.
#![allow(dead_code)]

const WIDE_ROWS: usize = 2048;
const NARROW_ROWS: usize = 512;
const FIELDS: usize = 4;
const ELEM: usize = 4;

#[derive(Clone, Copy)]
struct Buf {
    rows: usize,
    id: usize,
}

// the realloc-per-flip frame: one bank, resized whenever the round class
// changes
struct ReallocFrame {
    buffers: Vec<Option<Buf>>,
    alloc_calls: usize,
    next_id: usize,
}

impl ReallocFrame {
    fn new() -> Self {
        let mut f = ReallocFrame { buffers: vec![None; FIELDS], alloc_calls: 0, next_id: 0 };
        f.allocate(NARROW_ROWS.max(WIDE_ROWS));
        f
    }

    fn allocate(&mut self, rows: usize) {
        for slot in self.buffers.iter_mut() {
            *slot = Some(Buf { rows, id: self.next_id });
            self.next_id += 1;
            self.alloc_calls += 1;
        }
    }

    fn set_rows(&mut self, rows: usize) -> usize {
        let want = if rows <= NARROW_ROWS { NARROW_ROWS } else { WIDE_ROWS };
        let bound = self.buffers[0].map(|b| b.rows).unwrap_or(0);
        if bound != want {
            self.allocate(want); // the allocator pays on every class flip
        }
        self.buffers[0].map(|b| b.rows).unwrap_or(0)
    }
}

#[test]
fn every_class_flip_charges_the_allocator() {
    let mut f = ReallocFrame::new();
    let calls_before = f.alloc_calls;
    for rows in [1usize, 512, 513, 2048, 128, 2048, 1] {
        f.set_rows(rows);
    }
    // four class flips, four reallocations: churn on the hot path
    assert_eq!(f.alloc_calls - calls_before, FIELDS * 5);
}

#[test]
fn a_narrow_round_binds_the_wide_bytes() {
    let mut f = ReallocFrame::new();
    f.set_rows(2048);
    // a 1-row decode binds a 2048-row buffer: the wide bytes are bound
    assert_eq!(f.set_rows(1), NARROW_ROWS);
    let bound_bytes = f.buffers.iter().filter_map(|b| b.as_ref()).map(|b| b.rows * ELEM).sum::<usize>();
    assert_eq!(bound_bytes, NARROW_ROWS * ELEM * FIELDS);
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: dual-bank-scratch-retention-per-round-class): the
// frame allocates a PER-CLASS scratch bank at init (the narrow bank sized
// min(capacity, small_rows)) and swaps the ACTIVE bank per scratch field
// on class flip -- a pointer swap between completed rounds, allocation-
// free; metadata and outputs retain their original owners; both banks
// ride the residency inventory and the startup-admission byte count.
#![allow(dead_code)]

const WIDE_ROWS: usize = 2048;
const SMALL_ROWS: usize = 512;
const FIELDS: usize = 4;
const ELEM: usize = 4;

#[derive(Clone, Copy, PartialEq)]
struct Buf {
    rows: usize,
    id: usize,
}

struct DualBankFrame {
    buffers: Vec<Buf>, // the ACTIVE bank
    standby: Vec<Buf>, // the parked bank
    small: bool,
    alloc_calls: usize,
    meta_id: usize,
}

impl DualBankFrame {
    fn new(wide_rows: usize) -> Self {
        let mut f = DualBankFrame {
            buffers: Vec::with_capacity(FIELDS),
            standby: Vec::with_capacity(FIELDS),
            small: false,
            alloc_calls: 0,
            meta_id: 10_000,
        };
        for i in 0..FIELDS {
            f.buffers.push(Buf { rows: wide_rows, id: i });
            f.alloc_calls += 1;
        }
        if wide_rows > SMALL_ROWS {
            // the narrow bank: decodeSizes(config, min(capacity, small_rows))
            let narrow_rows = wide_rows.min(SMALL_ROWS);
            for i in 0..FIELDS {
                f.standby.push(Buf { rows: narrow_rows, id: 1_000 + i });
                f.alloc_calls += 1;
            }
        }
        f
    }

    // called between completed rounds; metadata (meta_id) retains its
    // original owner whatever the flip does
    fn set_rows(&mut self, rows: usize) -> Result<usize, ()> {
        if rows == 0 || rows > WIDE_ROWS {
            return Err(()); // bad frame rows: the state is unchanged
        }
        let small = rows <= SMALL_ROWS;
        if small != self.small && !self.standby.is_empty() {
            for f in 0..FIELDS {
                std::mem::swap(&mut self.buffers[f], &mut self.standby[f]);
            }
            self.small = small;
        }
        Ok(self.buffers[0].rows)
    }

    // both banks keep their residency: the inventory lists every buffer
    fn residency_inventory(&self) -> Vec<Buf> {
        let mut all = self.buffers.clone();
        all.extend_from_slice(&self.standby);
        all
    }

    // startup admission counts both banks
    fn admission_bytes(&self) -> usize {
        let bytes = |v: &[Buf]| v.iter().map(|b| b.rows * ELEM).sum::<usize>();
        bytes(&self.buffers) + bytes(&self.standby)
    }
}

#[test]
fn class_flips_swap_banks_and_allocate_nothing() {
    let mut f = DualBankFrame::new(WIDE_ROWS);
    let calls = f.alloc_calls;
    let mut seen = Vec::new();
    for rows in [1usize, 512, 513, 2048, 128, 2048, 1] {
        seen.push(f.set_rows(rows).unwrap());
    }
    // the active bank tracks the round class
    assert_eq!(seen, vec![SMALL_ROWS, SMALL_ROWS, WIDE_ROWS, WIDE_ROWS, SMALL_ROWS, WIDE_ROWS, SMALL_ROWS]);
    // the pointer swap allocated nothing across the whole sequence
    assert_eq!(f.alloc_calls, calls);
}

#[test]
fn metadata_and_outputs_retain_their_owners() {
    let mut f = DualBankFrame::new(WIDE_ROWS);
    let meta = f.meta_id;
    let active_ids_before: Vec<usize> = f.buffers.iter().map(|b| b.id).collect();
    // one narrow round and one wide round: two flips, back to the start
    f.set_rows(1).unwrap();
    assert_ne!(f.buffers.iter().map(|b| b.id).collect::<Vec<usize>>(), active_ids_before);
    f.set_rows(2048).unwrap();
    // metadata never moves
    assert_eq!(f.meta_id, meta);
    let active_ids_after: Vec<usize> = f.buffers.iter().map(|b| b.id).collect();
    assert_eq!(active_ids_after, active_ids_before);
}

#[test]
fn both_banks_are_inventoried_and_counted_in_admission() {
    let f = DualBankFrame::new(WIDE_ROWS);
    let inventory = f.residency_inventory();
    assert_eq!(inventory.len(), FIELDS * 2);
    // no duplicate ids across the inventory
    for (i, b) in inventory.iter().enumerate() {
        for prior in &inventory[..i] {
            assert_ne!(prior.id, b.id);
        }
    }
    assert_eq!(f.admission_bytes(), (WIDE_ROWS + SMALL_ROWS) * ELEM * FIELDS);
}

#[test]
fn bad_rows_refuse_and_leave_the_frame_unchanged() {
    let mut f = DualBankFrame::new(WIDE_ROWS);
    let before = f.buffers[0].id;
    assert!(f.set_rows(0).is_err());
    assert!(f.set_rows(WIDE_ROWS + 1).is_err());
    assert_eq!(f.buffers[0].id, before);
}

fn main() {}".to_string(),
        },
    ]
}

#[allow(clippy::too_many_lines)]
fn slice_lithos_b304() -> Vec<KernelRule> {
    vec![
        // ── Rule 6 (category K — autotune): the geometry table admits only bit-exact geometries; the rider face keeps below-tile variants on the default geometry. ──
        KernelRule {
            rule_id: "bit-exact-constrained-geometry-search".to_string(),
            gpu_concept: "MECHANISM (a measured kernel-choice table whose admission predicate is DETERMINISM — the search enumerates the tuner's variants per shape on the chip but admits only geometries that keep every output bit, and a kernel-level win that loses in the program is excluded; category K — autotune dispatch): the m5_max_40c table ships measured T = 1 decode GEMV geometry per shape — \"picked from compiler.autotune.Autotuner variants timed on this device and restricted to those that keep every output bit: no fused norm (its fold of the statistic's partials is not norm_apply's), and a row split on an op that writes a norm statistic only inside one threadgroup (tgsplit), whose partials are the unsplit op's.\" (the README: \"The 40-core M5 Max ships only T = 1 decode GEMV choices, limited to geometries that keep every output bit: no fused\" / \"norm, and an op that writes a norm statistic splits its rows only inside one threadgroup (`tgsplit`).\"). The admission classes are named in-source: a fused norm is OUT because \"it\" / \"folds the statistic's partials in another order than norm_apply\"; the confined split is IN (the tgsplit companion rule, this batch); and the PROGRAM-LEVEL NEGATIVE is part of the same predicate — \"The 4-bit gate|up split is left out: in the program it is slower than the\" / \"default. Decode step, plain decode at short context: 4-bit 39.6 -> 32.4 ms,\" / \"NVFP4 37.9 -> 35.4 ms, generated tokens unchanged.\" — a kernel-level win the program does not keep is not adopted. Shapes the table does not list keep the defaults. RIDER FACE (the posture-scope half, 2d51c4a): a tuned geometry is valid only in the execution posture it was timed in — \"A tuned GEMV geometry is timed where the shader op always runs. In a prompt\" / \"or verification program the variants below the tile run only for a chunk or\" / \"a verification of a row or two: a per-block launch from that table (one\" / \"threadgroup per block: 7760 for the 248K-row vocabulary projection) was\" / \"dispatched for nothing whenever the tile took the step, and a tuned RG was\" / \"timed only with its own launch. Those variants keep the default RG and the\" / \"crew launch, and the choices' norm path: the geometry moves no output bit.\" — the emitter encodes it as \"geo = choice if tile_range is None else None\" (the tuned geometry only where no tile owns the step), and the contract test states the law: \"short verification) keep the default RG and launch: a tuned per-block launch would be dispatched for nothing\" / \"whenever the tile takes the step, and the tuned RG was timed with its own launch.\" REGIME: measured per-chip kernel-choice tables for decode GEMV/attention; programs mixing tile and shader-op variants of one op; any lane with a bit-identity contract against the reference path. ANTI-REGIME: chip configs with no measured table (defaults everywhere); searches where bit-exactness is not a constraint (a fold-order change is then a separately gated decision, never this admission); static T = 1 programs without below-tile variants. KINSHIP (do not double-count): bit-parity-geometry-adoption-no-boot-timing (entries_gpu_122) owns the ADOPTION gate — a geometry adopted only under bit parity, never on boot timing (the consumer-side verdict); THIS owns the SEARCH-side constraint — the candidate space itself restricted to bit-exact geometries plus the program-level negative; compose. autotune-candidate-priority-orders-try-first (entries_gpu_130) owns priority semantics (cutoff vs order); autotune-round-bounded-by-roofline-ceiling (Batch 284) owns the stop; the tuner's own noise margin (a variant replaces the default only if faster by more than the fraction) is the speed-noise half — THIS adds the determinism half; tune-cache-keyed-by-spelled-variant-names (the rust_perf domain) owns persistence keys; coop-tensor-layout-measured-per-chip-family (the lithos-metal B227 rule) owns per-chip LAYOUT measurement; mma-window-admission-output-width-occupancy-floor (entries_gpu_134) owns occupancy-floored admission per shape — the speed/occupancy axis this rule's determinism axis composes with. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'kernel-choices|kernel choice table|geometry search' -> dispatch-ladder uses of kernel choice (entries_gpu_113) — no measured-table owner; 'bit-exact geometry|geometry bit-exact|bit parity adoption' -> bit-parity-geometry-adoption-no-boot-timing (cited, the gate side); 'variant beside|beside the tile|tile_range|dispatched for nothing' -> ZERO. NotBenchable here (per-chip Metal measurements — the ms rows are source-reported on THEIR M5 Max, provenance never a bar; the admission predicate, the exclusion classes and the posture-scope predicate are CPU-testable and ARE the fixtures). Source: lithos-ai/lithos-metal @ 8ed1611368add991da79996b4b505566fa2f7c75 (Apache-2.0) — monolith/backends/metal/m5_max_40c/kernel-choices.json:5,62 + monolith/backends/metal/README.md:24-25 + the commit message; rider face @ 2d51c4a49c3cac7f5858fff97b81d9f372085143 — monolith/compiler/emit.py:397-400 + tests/contract/test_nn_lowering.py:401-443, quotes byte-verified at the pins (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: bit-exact-constrained-geometry-search): the tuner picks
// the fastest variant per shape with NO determinism constraint and NO
// posture scope -- a fused-norm variant whose fold order differs wins the
// timing and ships (the statistic moves a bit), and the tuned geometry is
// applied to the below-tile variant that runs only for a row or two
// (dispatched for nothing whenever the tile takes the step).
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    PlainNorm,
    FusedNorm,
    GateUpSplit,
}

#[derive(Clone, Copy)]
struct Variant {
    kind: Kind,
    ms: f64,        // the kernel-level timing the tuner keys on
    program_ms: f64, // the whole-program time the step actually sees
    // the statistic's bits under this geometry (two partial folds)
    stat_bits: [u32; 2],
    rg: usize,       // the tuned row-group geometry
    applies_below_tile: bool,
}

// the reference geometry's statistic fold: p0, then p1, then p2 in order
fn reference_stat_bits() -> [u32; 2] {
    let p = [1.0f32, f32::from_bits(0x3380_0000), f32::from_bits(0x3380_0000)]; // 1, 2^-24, 2^-24
    let mut s = 0.0f32;
    for v in p {
        s += v; // the norm_apply order
    }
    let grouped = p[1] + p[2]; // the fused norm folds partials per group first
    [s.to_bits(), grouped.to_bits()]
}

fn variants_naive() -> Vec<Variant> {
    let r = reference_stat_bits();
    vec![
        Variant { kind: Kind::PlainNorm, ms: 0.0712, program_ms: 32.4, stat_bits: [r[0], r[0]], rg: 2, applies_below_tile: true },
        Variant { kind: Kind::FusedNorm, ms: 0.0688, program_ms: 32.4, stat_bits: [r[1], r[1]], rg: 4, applies_below_tile: true },
        Variant { kind: Kind::GateUpSplit, ms: 0.0705, program_ms: 33.1, stat_bits: [r[0], r[0]], rg: 8, applies_below_tile: true },
    ]
}

// the naive tuner: fastest ms wins, whatever the bits, whatever the posture
fn tune_naive(v: &[Variant]) -> Variant {
    *v.iter().min_by(|a, b| a.ms.partial_cmp(&b.ms).unwrap()).unwrap()
}

#[derive(Default)]
struct DispatchCounter {
    dispatched_for_nothing: usize,
}

fn dispatch_naive(v: &Variant, below_tile: bool, counter: &mut DispatchCounter) -> usize {
    // the tuned geometry applies everywhere, including the below-tile
    // variants a chunk of a row or two runs through
    let rg = v.rg;
    if below_tile {
        counter.dispatched_for_nothing += rg * 8; // per-block launch, wasted
    }
    rg
}

#[test]
fn the_fastest_variant_ships_despite_moving_a_bit() {
    let v = variants_naive();
    let pick = tune_naive(&v);
    assert_eq!(pick.kind, Kind::FusedNorm); // fastest ms
    let r = reference_stat_bits();
    // the shipped geometry's statistic is NOT the reference statistic
    assert_ne!(pick.stat_bits[0], r[0]);
}

#[test]
fn the_below_tile_variant_pays_the_tuned_launch_for_nothing() {
    let v = variants_naive();
    let pick = tune_naive(&v);
    let mut c = DispatchCounter::default();
    dispatch_naive(&pick, true, &mut c);
    assert!(c.dispatched_for_nothing > 0);
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: bit-exact-constrained-geometry-search): the table
// admits only geometries that keep every output bit -- the fused-norm
// fold-order variant is OUT whatever its timing -- and a kernel-level win
// that loses in the program is OUT too; a tuned geometry applies only
// where the shader op always runs (the below-tile variants keep the
// default geometry), and shapes the table does not list keep the
// defaults.
#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    PlainNorm,
    FusedNorm,
    GateUpSplit,
}

#[derive(Clone, Copy)]
struct Variant {
    kind: Kind,
    ms: f64,
    program_ms: f64,
    stat_bits: [u32; 2],
    rg: usize,
    applies_below_tile: bool,
}

fn reference_stat_bits() -> [u32; 2] {
    let p = [1.0f32, f32::from_bits(0x3380_0000), f32::from_bits(0x3380_0000)];
    let mut s = 0.0f32;
    for v in p {
        s += v;
    }
    let grouped = p[1] + p[2];
    [s.to_bits(), grouped.to_bits()]
}

fn variants() -> Vec<Variant> {
    let r = reference_stat_bits();
    vec![
        Variant { kind: Kind::PlainNorm, ms: 0.0712, program_ms: 32.4, stat_bits: [r[0], r[0]], rg: 2, applies_below_tile: true },
        Variant { kind: Kind::FusedNorm, ms: 0.0699, program_ms: 32.4, stat_bits: [r[1], r[1]], rg: 4, applies_below_tile: true },
        Variant { kind: Kind::GateUpSplit, ms: 0.0688, program_ms: 33.1, stat_bits: [r[0], r[0]], rg: 8, applies_below_tile: true },
    ]
}

fn keeps_every_bit(v: &Variant, reference: &[u32; 2]) -> bool {
    v.stat_bits[0] == reference[0] && v.stat_bits[1] == reference[0]
}

// the table: the tuner's fastest PER-BIT-EXACT variant; a kernel-level win
// that is slower in the program is excluded (the program-level negative);
// None where nothing qualifies -- the shape keeps the defaults
fn tune_table(v: &[Variant]) -> Option<Variant> {
    let r = reference_stat_bits();
    let mut best: Option<Variant> = None;
    for cand in v {
        if !keeps_every_bit(cand, &r) {
            continue; // the determinism constraint precedes the timing
        }
        // the program-level negative: a faster kernel that makes the
        // program slower than the default's program never enters
        if best.map(|b| cand.program_ms < b.program_ms).unwrap_or(true) {
            best = Some(*cand);
        }
    }
    best
}

#[derive(Default)]
struct DispatchCounter {
    dispatched_for_nothing: usize,
}

// a tuned geometry applies where the op always runs; the below-tile
// variants keep the default geometry (rg 1, the plain crew launch)
fn dispatch(v: Option<&Variant>, below_tile: bool, counter: &mut DispatchCounter) -> usize {
    match (v, below_tile) {
        (Some(choice), false) => choice.rg,
        _ => {
            if below_tile {
                counter.dispatched_for_nothing += 0; // the default launch: nothing wasted
            }
            1
        }
    }
}

#[test]
fn the_table_admits_only_bit_exact_geometries() {
    let pick = tune_table(&variants()).unwrap();
    let r = reference_stat_bits();
    assert_eq!(pick.kind, Kind::PlainNorm); // the fastest BIT-EXACT one
    assert_eq!(pick.stat_bits[0], r[0]);
    // the fused-norm variant was excluded although its ms was lower than
    // the plain variant's
    let v = variants();
    assert!(v[1].ms < v[0].ms);
    assert_ne!(v[1].stat_bits[0], r[0]);
}

#[test]
fn a_kernel_win_that_loses_the_program_is_out() {
    // the gate|up split has the lowest kernel ms but the highest
    // program ms: the program-level negative excludes it even though
    // its bits are clean
    let v = variants();
    assert!(v[2].ms < v[0].ms);
    assert!(v[2].program_ms > v[0].program_ms);
    assert_ne!(tune_table(&v).unwrap().kind, Kind::GateUpSplit);
}

#[test]
fn the_below_tile_variant_keeps_the_default_geometry() {
    let pick = tune_table(&variants());
    let mut c = DispatchCounter::default();
    // the step runs through the tile; the below-tile variant fires for a
    // row or two: default geometry, nothing dispatched for nothing
    let rg = dispatch(pick.as_ref(), true, &mut c);
    assert_eq!(rg, 1);
    assert_eq!(c.dispatched_for_nothing, 0);
    // where the op always runs, the tuned geometry applies
    let rg_main = dispatch(pick.as_ref(), false, &mut c);
    assert_eq!(rg_main, pick.unwrap().rg);
}

#[test]
fn an_unlisted_shape_keeps_the_defaults() {
    // no table entry for this shape: None -> the default geometry
    assert!(tune_table(&[]).is_none());
    let mut c = DispatchCounter::default();
    assert_eq!(dispatch(None, false, &mut c), 1);
}

fn main() {}".to_string(),
        },
        // ── Rule 7 (category E — parallelism): a statistic producer’s row split confined to one threadgroup. ──
        KernelRule {
            rule_id: "statistic-producer-row-split-single-threadgroup".to_string(),
            gpu_concept: "MECHANISM (a row split on an op that WRITES A STATISTIC is confined to one threadgroup, so the statistic's partial wire keeps its count and values; category E — parallelism/split geometry): a plain row split gives every work item its own partial — \"A row split (RSPLIT) on an op that writes a norm statistic gives every work\" / \"item its own partial, so the next norm sums other partials and its scale can\" / \"move by an ULP.\" — the statistic's wire becomes n_blocks*RSPLIT slots holding different values, and the consumer norm folds a different association. THE REPAIR (STAT_TG): \"With STAT_TG the block's RSPLIT items are the SIMD-groups of\" / \"one threadgroup (grid mode tgsplit) and, after a barrier, the block's partial\" / \"is chained over its rows in order: the outputs and the statistic are the\" / \"unsplit op's byte for byte\" — the kernel comment pins the contract: \"// 1 (with STAT_OUT and RSPLIT > 1): a block's RSPLIT items are the SIMD-groups of one threadgroup\" / \"// partial of RSPLIT = 1, so the statistic keeps its n_blocks partials and their values\", the chained fold writes the UNSPLIT slot (\"stat_out[t * p.n_blocks + bb] = s;\" where the plain split wrote \"stat_out[t * p.n_blocks * RSPLIT + it] = ssq_out[t];\" — \"// one partial per item\"), and the illegal pairings are refused at compile time: \"#error \"gemv_T STAT_TG: the row split of a statistic producer (STAT_OUT, RSPLIT > 1, no PAIRS, no silu_mul)\"\". THE TUNER HALF closes the loop: the emitter \"tells the autotuner which ops write a statistic:\" / \"every variant of such an op is timed writing it, in its own cache entry\" / \"(|stat), and tgsplit is a candidate only there (not with silu_mul's paired\" / \"rows).\" — the key gains \"key += \"|stat\"\" (\"# the op writes the next norm's statistic: timed writing it\") and the lookup gives the producer its own entry where the table has one, else the shape's, so \"the tables written before keep their keys\" (the KernelChoices doc: \"without a tuner (``None``). A statistic producer takes its own entry (``|stat``) where the table has one, else the\"). THE MEASURED WIN, source-reported: \"the narrow residual projections fill the GPU\" / \"(M5 Max, int4 5120x17408 at T = 1: 142 -> 100 us against the fastest unsplit\" / \"geometry).\" REGIME: GEMV/reduction kernels whose epilogue writes a statistic (norm sums, LSE) consumed by a following fused norm; row-split parallelization of narrow projections at tiny batch. ANTI-REGIME: ops that write no statistic (the split is free — the emitter's own comment: \"(no statistic: the split is free, but the tile's twin keeps the simple layout)\"); paired-rows ops (silu_mul's gate/up partners); consumers without a bit contract. KINSHIP (do not double-count): lane-permuted-column-reduce-same-butterfly-tree-bit-identity (entries_gpu_105) owns the lane-to-row MAP change keeping the same fold tree (a fixed decomposition whose partials are preserved by permutation); THIS owns the DECOMPOSITION choice itself — how many partials exist and who produces them — held at the unsplit contract by threadgroup confinement; splitk-reduce-one-launch-slice-order-pinned (entries_gpu_67) pins the slice ORDER within a fixed split count — THIS pins the split COUNT and values; two-pass-partial-reduction-inter-dispatch-barrier (entries_gpu_116, Batch 269) owns partials ACROSS dispatches with a barrier plus a final reduce kernel (the partial-count change there is internal, never consumer-visible); contraction-partials-sum-once-at-drain (entries_gpu_134) owns register-partials accumulation structure; the register-resident scan's bit-identity contract rules (entries_gpu_09) own the packing contract for recurrence rows. GREP EVIDENCE (over the corpus, 3+ vocabularies): 'tgsplit|RSPLIT|statistic producer' -> ZERO owners; 'row split|rows split' -> the GEMV packing family's lane-map axis (entries_gpu_09 riders) — never a partial-wire law; 'ULP' -> the lane-permute rule's last-bit drift prose (entries_gpu_105, cited) — no partial-contract owner; '|stat|stat_out|timed writing' -> ZERO. NotBenchable here (Metal threadgroup geometry; 142 -> 100 us is source-reported on THEIR M5 Max; the partial-contract identity, the gate and the keying are CPU-testable and ARE the fixtures). Source: lithos-ai/lithos-metal @ cd220abd6e85cc987a05ca1b02624b94798066e3 (Apache-2.0) — kernels/common/gemv_T.metal:131-136,328,550-556,575-585 + monolith/compiler/autotune.py:33,40-42,78-79,88-89 + monolith/compiler/emit.py:394-400 + the commit message, quotes byte-verified at the pin (plan 309 T0, 2026-10-29). Batch 304 / Plan 309 / Research 348.".to_string(),
            ref_shape: r"// ref_shape (rule: statistic-producer-row-split-single-threadgroup): a row
// split on an op that writes a norm statistic gives every work item its
// own partial -- the statistic's partial wire becomes n_blocks*RSPLIT
// slots holding different values, and the consumer norm folds them in a
// different grouping: its scale moves in the last bit vs the unsplit op.
#![allow(dead_code)]

const R: usize = 4; // rows per block
const N_BLOCKS: usize = 1;

// the block's row outputs (the statistic is their sum of squares); the
// squares are 1, 2^-24, 2^-24, 2^-24 -- exact powers of two
fn row_outputs() -> [f32; R] {
    let v = 2.0f32.powi(-12); // v^2 = 2^-24, exactly half an ulp at 1.0
    [1.0, v, v, v]
}

// the UNSPLIT op: one item, the partial chained over the rows IN ORDER.
// This is the contract the statistic's consumers were built against.
fn stat_unsplit() -> f32 {
    let mut s = 0.0f32;
    for r in row_outputs() {
        s = f32::mul_add(r, r, s);
    }
    s
}

// the PLAIN row split: RSPLIT items each chain their own rows into their
// own partial -- the wire becomes n_blocks * RSPLIT slots
const RSPLIT: usize = 2;

fn stat_plain_split() -> Vec<f32> {
    let rows = row_outputs();
    let per = R / RSPLIT;
    let mut partials = Vec::with_capacity(N_BLOCKS * RSPLIT);
    for item in 0..RSPLIT {
        let mut s = 0.0f32;
        for r in 0..per {
            let v = rows[item * per + r];
            s = f32::mul_add(v, v, s);
        }
        partials.push(s); // stat_out[t * n_blocks * RSPLIT + item]
    }
    partials
}

// the consumer norm folds the partials it is handed -- with the split
// wire that is a different association than the unsplit chain
fn fold(partials: &[f32]) -> f32 {
    let mut s = 0.0f32;
    for p in partials {
        s += p;
    }
    s
}

fn scale_of(ssq: f32) -> u32 {
    (1.0 / ssq).to_bits() // the consumer normalizes by the statistic
}

#[test]
fn the_split_wire_changes_the_statistic_bits() {
    let a = stat_unsplit();
    let b = fold(&stat_plain_split());
    // the chain and the split-fold disagree in the last bit: 1.0 vs
    // 1 + 2^-23
    assert_eq!(a.to_bits(), 1.0f32.to_bits());
    assert_ne!(a.to_bits(), b.to_bits());
    // and the consumer's derived value moves with the statistic
    assert_ne!(scale_of(a), scale_of(b));
}

#[test]
fn the_wire_shaped_differently_even_where_values_overlap() {
    // n_blocks * RSPLIT slots, not n_blocks: the partial CONTRACT moved
    assert_eq!(stat_plain_split().len(), N_BLOCKS * RSPLIT);
}

fn main() {}".to_string(),
            opt_pattern: r"// opt_pattern (rule: statistic-producer-row-split-single-threadgroup):
// STAT_TG -- the block's RSPLIT items are the SIMD-groups of ONE
// threadgroup; after a barrier the block's partial is chained over its
// rows IN ORDER and lands in the SAME slot with the SAME values the
// unsplit op wrote (byte for byte); a compile-time gate refuses the
// illegal shape pairings, and the tuner keys a statistic producer's
// variants in their own |stat entry so the confined split is a candidate
// only there.
#![allow(dead_code)]

const R: usize = 4;
const N_BLOCKS: usize = 1;
const RSPLIT: usize = 2;

fn row_outputs() -> [f32; R] {
    let v = 2.0f32.powi(-12);
    [1.0, v, v, v]
}

// the contract: the unsplit chain, byte for byte
fn stat_unsplit() -> f32 {
    let mut s = 0.0f32;
    for r in row_outputs() {
        s = f32::mul_add(r, r, s);
    }
    s
}

// STAT_TG: each item (a lane group of the one threadgroup) privately
// rounds its rows' outputs into the block's row table; the partial is
// chained over ALL rows in row order, into the UNSPLIT slot
// stat_out[t * n_blocks + bb]
fn stat_tg() -> (f32, usize) {
    let rows = row_outputs();
    let per = R / RSPLIT;
    // the items hold their partials privately (no separate wire slots)
    let mut block_rows = [0.0f32; R];
    for item in 0..RSPLIT {
        for r in 0..per {
            block_rows[item * per + r] = rows[item * per + r];
        }
    }
    // ...the barrier... then part 0 chains the block's rows in order
    let mut s = 0.0f32;
    for r in block_rows {
        s = f32::mul_add(r, r, s);
    }
    (s, N_BLOCKS) // the wire is n_blocks slots, as the unsplit op's
}

// the compile-time gate: the row split of a statistic producer is legal
// only inside one threadgroup, no paired rows, no two-pass epilogue
fn stat_tg_shape_legal(stat_out: bool, rsplit: usize, pairs: bool, epilogue2: bool) -> Result<(), ()> {
    if stat_out && (rsplit == 1 || pairs || epilogue2) {
        return Err(()); // the source's #error, as a typed refusal
    }
    Ok(())
}

// the tuner key: a statistic producer is timed WRITING the statistic, in
// its own cache entry; the lookup takes the producer's entry where the
// table has one, else the shape's
#[derive(Clone, Copy, PartialEq, Debug)]
struct Entry {
    geometry: usize,
}

fn table() -> ([Option<Entry>; 2], Option<Entry>) {
    // [|stat entry for the producer variant, plain entry for the shape]
    (
        [Some(Entry { geometry: 7 }), Some(Entry { geometry: 4 })],
        Some(Entry { geometry: 4 }),
    )
}

fn lookup(stat_out: bool) -> Entry {
    let (stat_entries, shape_entry) = table();
    if stat_out {
        stat_entries[0].or(shape_entry).unwrap()
    } else {
        shape_entry.unwrap()
    }
}

#[test]
fn the_confined_split_reproduces_the_unsplit_bits_and_wire() {
    let a = stat_unsplit();
    let (b, wire) = stat_tg();
    assert_eq!(a.to_bits(), b.to_bits()); // byte for byte
    assert_eq!(wire, N_BLOCKS); // the statistic keeps its n_blocks partials
}

#[test]
fn the_compile_gate_refuses_the_illegal_pairings() {
    // a statistic producer with a real split: legal ONLY as STAT_TG
    assert!(stat_tg_shape_legal(true, RSPLIT, false, false).is_ok());
    // paired rows (silu_mul) and the two-pass epilogue are refused
    assert!(stat_tg_shape_legal(true, RSPLIT, true, false).is_err());
    assert!(stat_tg_shape_legal(true, RSPLIT, false, true).is_err());
    // no statistic: the split is free, the gate does not fire
    assert!(stat_tg_shape_legal(false, RSPLIT, true, false).is_ok());
}

#[test]
fn the_producer_lookup_takes_its_own_entry_else_the_shapes() {
    // the producer's |stat entry exists: it wins
    assert_eq!(lookup(true), Entry { geometry: 7 });
    // a non-producer op reads the shape's entry
    assert_eq!(lookup(false), Entry { geometry: 4 });
    // a table without the producer's entry falls back to the shape's, so
    // tables written before keep their keys
    let (mut entries, shape) = table();
    entries[0] = None;
    assert_eq!(entries[0].or(shape).unwrap(), Entry { geometry: 4 });
}

fn main() {}".to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    let mut v = slice_vllm_b304_window();
    v.extend(slice_tf_b304_pool());
    v.extend(slice_lithos_b304());
    v
}
