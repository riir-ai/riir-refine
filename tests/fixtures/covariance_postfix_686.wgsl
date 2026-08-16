// covariance.wgsl
// SpectralQuant covariance computation: compute per-(layer, head) covariance matrix
// from K/V tensors for calibration pass.
//
// Algorithm:
//   Computes Σ_i (x_i · x_i^T) where x_i are d_h-dimensional vectors from the K/V tensor.
//   Each 16×16 workgroup processes one tile of the d_h × d_h output covariance matrix,
//   iterating over all sequence positions to accumulate the outer products.
//
// Register accumulation core (output-owned grid):
//   - Thread (global_col, global_row) maps to covariance element (global_row, global_col)
//   - Each thread accumulates its output element in a register across ALL positions,
//     then writes it once — no shared memory, no barriers.
//
// The former shared-memory staging (s_row/s_col + two workgroupBarriers per
// position) was removed (Issue 686 H7): each thread reads exactly one row
// element and one column element per position — the same values a direct
// global read returns (same-address reads broadcast in one transaction) —
// so the staging saved no global traffic while costing 2 barriers/position.

struct CovarianceParams {
    seq_len: u32,
    d_h: u32,
    layer_offset: u32,
    head_offset: u32,
    total_heads: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read>       input_kv: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_cov: array<f32>;
@group(0) @binding(2) var<uniform>             params: CovarianceParams;

@compute @workgroup_size(16, 16, 1)
fn covariance(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global_row = gid.y;
    let global_col = gid.x;
    let d_h = params.d_h;

    if (global_row >= d_h || global_col >= d_h) {
        return;
    }

    // Stride between sequence positions: (n_heads, d_h) layout per position
    let pos_stride = params.total_heads * d_h;
    // Base offset for this (layer, head) in the flat KV tensor
    let base_offset = params.layer_offset + params.head_offset;

    var accum: f32 = 0.0;

    // Iterate over all sequence positions, accumulating the outer-product
    // element in-register.
    for (var i = 0u; i < params.seq_len; i = i + 1u) {
        let pos_base = base_offset + i * pos_stride;
        accum += input_kv[pos_base + global_row] * input_kv[pos_base + global_col];
    }

    // Write final accumulated covariance to output (row-major)
    output_cov[global_row * d_h + global_col] = accum;
}
