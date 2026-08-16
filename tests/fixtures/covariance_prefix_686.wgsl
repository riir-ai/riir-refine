// covariance.wgsl
// SpectralQuant covariance computation: compute per-(layer, head) covariance matrix
// from K/V tensors for calibration pass.
//
// Algorithm:
//   Computes Σ_i (x_i · x_i^T) where x_i are d_h-dimensional vectors from the K/V tensor.
//   Each 16×16 workgroup processes one tile of the d_h × d_h output covariance matrix,
//   iterating over all sequence positions to accumulate the outer products.
//
// Tiled accumulation with shared memory:
//   - Thread (local_col, local_row) maps to covariance element (global_row, global_col)
//   - For each sequence position, row/column elements are loaded into shared memory
//     by the first column/row of threads respectively (coalesced reads)
//   - Partial sums accumulate in registers across all positions, final result written once

struct CovarianceParams {
    seq_len: u32,
    d_h: u32,
    layer_offset: u32,
    head_offset: u32,
    total_layers: u32,
    total_heads: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read>       input_kv: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_cov: array<f32>;
@group(0) @binding(2) var<uniform>             params: CovarianceParams;

// Shared tile for coalesced loads: each sequence position loads 16 row + 16 column elements
var<workgroup> s_row: array<f32, 16>;
var<workgroup> s_col: array<f32, 16>;

@compute @workgroup_size(16, 16, 1)
fn covariance(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global_row = gid.y;
    let global_col = gid.x;
    let d_h = params.d_h;
    let local_row = gid.y % 16u;
    let local_col = gid.x % 16u;

    let in_bounds = global_row < d_h && global_col < d_h;

    // Stride between sequence positions: (n_heads, d_h) layout per position
    let pos_stride = params.total_heads * d_h;
    // Base offset for this (layer, head) in the flat KV tensor
    let base_offset = params.layer_offset + params.head_offset;

    var accum: f32 = 0.0;

    // Iterate over all sequence positions, accumulating outer product tiles
    for (var i = 0u; i < params.seq_len; i = i + 1u) {
        let pos_base = base_offset + i * pos_stride;

        // First column of threads loads row elements into shared memory
        if (local_col == 0u && global_row < d_h) {
            s_row[local_row] = input_kv[pos_base + global_row];
        }
        // First row of threads loads column elements into shared memory
        if (local_row == 0u && global_col < d_h) {
            s_col[local_col] = input_kv[pos_base + global_col];
        }

        workgroupBarrier();

        // Accumulate outer product element for this position
        if (in_bounds) {
            accum += s_row[local_row] * s_col[local_col];
        }

        // Ensure all threads have read shared memory before next iteration overwrites
        workgroupBarrier();
    }

    // Write final accumulated covariance to output (row-major)
    if (in_bounds) {
        output_cov[global_row * d_h + global_col] = accum;
    }
}
