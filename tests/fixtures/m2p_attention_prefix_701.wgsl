// M2P Alternating Attention kernels for SHINE hypernetwork (Plan 098).
// Column attention: mix across rows (L layers) per column (token position).
// Row attention: mix across columns (M tokens) per row (layer).
// Post-layernorm architecture (not pre-layernorm).

struct M2PAttnParams {
    seq_len: u32,      // L for column, M for row
    hidden_dim: u32,   // H
    num_heads: u32,
    head_dim: u32,     // H / num_heads
    grid_stride: u32,  // cols * hidden_dim for column, hidden_dim for row
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

// ── Column Attention ──────────────────────────────────────────────
// Mix across rows (L layers) for a single column (token position j).
// Input: column slice [L, H] extracted from grid [L, M, H].
// Output: attention output [L, H].

@group(0) @binding(0) var<storage, read>       qkv_weight: array<f32>;   // [3*H, H]
@group(0) @binding(1) var<storage, read>       out_weight: array<f32>;   // [H, H]
@group(0) @binding(2) var<storage, read>       input_col: array<f32>;    // [L*H]
@group(0) @binding(3) var<storage, read_write> output_col: array<f32>;   // [L*H]
@group(0) @binding(4) var<uniform>             params: M2PAttnParams;

@compute @workgroup_size(1, 1, 1)
fn m2p_column_attention(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;   // position in sequence (0..L)
    let head = gid.y;  // head index (0..num_heads)
    if (pos >= params.seq_len || head >= params.num_heads) { return; }

    let h = params.hidden_dim;
    let hd = params.head_dim;
    let seq = params.seq_len;
    let head_off = head * hd;
    let scale = 1.0 / sqrt(f32(hd));

    // QKV projection for this position: Q = Wq * input[pos], K = Wk * input[pos], V = Wv * input[pos]
    var q: array<f32, 128>;
    for (var d = 0u; d < hd; d = d + 1u) {
        var sum = 0.0;
        for (var k = 0u; k < h; k = k + 1u) {
            let inp = input_col[pos * h + k];
            let wq = qkv_weight[(head_off + d) * h + k];          // Q weights
            sum = sum + inp * wq;
        }
        q[d] = sum;
    }

    // Attention scores against all positions
    var max_score: f32 = -1e30;
    var scores: array<f32, 256>;
    for (var t = 0u; t < seq; t = t + 1u) {
        var dot: f32 = 0.0;
        for (var d = 0u; d < hd; d = d + 1u) {
            // K for position t
            var k_val: f32 = 0.0;
            for (var kk = 0u; kk < h; kk = kk + 1u) {
                let inp = input_col[t * h + kk];
                let wk = qkv_weight[(h + head_off + d) * h + kk]; // K weights
                k_val = k_val + inp * wk;
            }
            dot = dot + q[d] * k_val;
        }
        let s = dot * scale;
        scores[t] = s;
        if (s > max_score) { max_score = s; }
    }

    // Softmax
    var sum_exp: f32 = 0.0;
    for (var t = 0u; t < seq; t = t + 1u) {
        scores[t] = exp(scores[t] - max_score);
        sum_exp = sum_exp + scores[t];
    }

    // Weighted sum of V + output projection
    for (var d_out = 0u; d_out < hd; d_out = d_out + 1u) {
        var attn_val: f32 = 0.0;
        for (var t = 0u; t < seq; t = t + 1u) {
            // V for position t
            var v_val: f32 = 0.0;
            for (var kk = 0u; kk < h; kk = kk + 1u) {
                let inp = input_col[t * h + kk];
                let wv = qkv_weight[(2u * h + head_off + d_out) * h + kk]; // V weights
                v_val = v_val + inp * wv;
            }
            attn_val = attn_val + (scores[t] / sum_exp) * v_val;
        }
        // Output projection
        var proj_val: f32 = 0.0;
        for (var kk = 0u; kk < h; kk = kk + 1u) {
            proj_val = proj_val + attn_val * out_weight[d_out * h + kk];
        }
        output_col[pos * h + head_off + d_out] = proj_val;
    }
}

// ── Row Attention ──────────────────────────────────────────────────
// Mix across columns (M tokens) for a single row (layer i).
// Identical to column attention but operates on a row slice [M, H].

@group(0) @binding(0) var<storage, read>       qkv_weight_r: array<f32>; // [3*H, H]
@group(0) @binding(1) var<storage, read>       out_weight_r: array<f32>; // [H, H]
@group(0) @binding(2) var<storage, read>       input_row: array<f32>;    // [M*H]
@group(0) @binding(3) var<storage, read_write> output_row: array<f32>;   // [M*H]
@group(0) @binding(4) var<uniform>             params_r: M2PAttnParams;

@compute @workgroup_size(1, 1, 1)
fn m2p_row_attention(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;
    let head = gid.y;
    if (pos >= params_r.seq_len || head >= params_r.num_heads) { return; }

    let h = params_r.hidden_dim;
    let hd = params_r.head_dim;
    let seq = params_r.seq_len;
    let head_off = head * hd;
    let scale = 1.0 / sqrt(f32(hd));

    // Q projection
    var q: array<f32, 128>;
    for (var d = 0u; d < hd; d = d + 1u) {
        var sum = 0.0;
        for (var k = 0u; k < h; k = k + 1u) {
            sum = sum + input_row[pos * h + k] * qkv_weight_r[(head_off + d) * h + k];
        }
        q[d] = sum;
    }

    // Attention scores
    var max_score: f32 = -1e30;
    var scores: array<f32, 256>;
    for (var t = 0u; t < seq; t = t + 1u) {
        var dot: f32 = 0.0;
        for (var d = 0u; d < hd; d = d + 1u) {
            var k_val: f32 = 0.0;
            for (var kk = 0u; kk < h; kk = kk + 1u) {
                k_val = k_val + input_row[t * h + kk] * qkv_weight_r[(h + head_off + d) * h + kk];
            }
            dot = dot + q[d] * k_val;
        }
        let s = dot * scale;
        scores[t] = s;
        if (s > max_score) { max_score = s; }
    }

    // Softmax
    var sum_exp: f32 = 0.0;
    for (var t = 0u; t < seq; t = t + 1u) {
        scores[t] = exp(scores[t] - max_score);
        sum_exp = sum_exp + scores[t];
    }

    // Weighted sum + output projection
    for (var d_out = 0u; d_out < hd; d_out = d_out + 1u) {
        var attn_val: f32 = 0.0;
        for (var t = 0u; t < seq; t = t + 1u) {
            var v_val: f32 = 0.0;
            for (var kk = 0u; kk < h; kk = kk + 1u) {
                v_val = v_val + input_row[t * h + kk] * qkv_weight_r[(2u * h + head_off + d_out) * h + kk];
            }
            attn_val = attn_val + (scores[t] / sum_exp) * v_val;
        }
        var proj_val: f32 = 0.0;
        for (var kk = 0u; kk < h; kk = kk + 1u) {
            proj_val = proj_val + attn_val * out_weight_r[d_out * h + kk];
        }
        output_row[pos * h + head_off + d_out] = proj_val;
    }
}
