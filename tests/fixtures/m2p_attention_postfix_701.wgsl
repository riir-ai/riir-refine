// M2P Alternating Attention kernels for SHINE hypernetwork (Plan 098).
// Column attention: mix across rows (L layers) per column (token position).
// Row attention: mix across columns (M tokens) per row (layer).
// Post-layernorm architecture (not pre-layernorm).
//
// Issue 701 H-C: the output projection runs as a SEPARATE PHASE
// (`m2p_out_projection`). The per-(pos, head) attention kernels stage only
// their own head's output to `attn_scratch`; the out-projection consumes the
// CONCATENATION of all heads' outputs, so it runs as a second dispatch where
// one thread owns the full H-length row (corpus rule
// `multi-head-out-proj-consumes-head-concat`). The previous fused form
// computed `attn_val * rowsum(wo)` — invariant in the summation index — which
// is NOT the matvec.
//
// Host-side caps (Issue 701 H-B, validated in `gpu_m2p.rs` before dispatch):
//   head_dim <= 128  (fixed private `var q: array<f32, 128>`)
//   seq_len  <= 256  (fixed private `var scores: array<f32, 256>`)
// Follow-up (not built here): pipeline override constants per bound class
// would lift both caps at the cost of one pipeline per bound class.

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

// Shared resource declarations — one set for all three entry points. The
// column and row kernels consume different SLICES, selected by which buffer
// the host binds, not by different shader variables.
@group(0) @binding(0) var<storage, read>       qkv_weight: array<f32>;      // [3*H, H]
@group(0) @binding(1) var<storage, read>       out_weight: array<f32>;      // [H, H]
@group(0) @binding(2) var<storage, read>       input_seq: array<f32>;       // [seq*H]
@group(0) @binding(3) var<storage, read_write> attn_scratch: array<f32>;    // [seq*H] staged per-head outputs
@group(0) @binding(4) var<storage, read_write> output_seq: array<f32>;      // [seq*H] final projected output
@group(0) @binding(5) var<uniform>             params: M2PAttnParams;

// ── Column Attention (phase 1) ────────────────────────────────────
// Mix across rows (L layers) for a single column (token position j).
// Input: column slice [L, H] extracted from grid [L, M, H].
// Stages per-head attention outputs to `attn_scratch` — NO out-projection.

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

    // Q projection for this position: Q = Wq * input[pos]
    var q: array<f32, 128>;
    for (var d = 0u; d < hd; d = d + 1u) {
        var sum = 0.0;
        for (var k = 0u; k < h; k = k + 1u) {
            let inp = input_seq[pos * h + k];
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
                let inp = input_seq[t * h + kk];
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

    // Weighted sum of V → stage this head's slice of the attention output.
    // The out-projection is phase 2 (m2p_out_projection).
    for (var d_out = 0u; d_out < hd; d_out = d_out + 1u) {
        var attn_val: f32 = 0.0;
        for (var t = 0u; t < seq; t = t + 1u) {
            // V for position t
            var v_val: f32 = 0.0;
            for (var kk = 0u; kk < h; kk = kk + 1u) {
                let inp = input_seq[t * h + kk];
                let wv = qkv_weight[(2u * h + head_off + d_out) * h + kk]; // V weights
                v_val = v_val + inp * wv;
            }
            attn_val = attn_val + (scores[t] / sum_exp) * v_val;
        }
        attn_scratch[pos * h + head_off + d_out] = attn_val;
    }
}

// ── Row Attention (phase 1) ───────────────────────────────────────
// Mix across columns (M tokens) for a single row (layer i).
// Identical algorithm to column attention but operates on a row slice
// [M, H] (the host binds the row's buffer as `input_seq`).

@compute @workgroup_size(1, 1, 1)
fn m2p_row_attention(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;   // position in sequence (0..M)
    let head = gid.y;  // head index (0..num_heads)
    if (pos >= params.seq_len || head >= params.num_heads) { return; }

    let h = params.hidden_dim;
    let hd = params.head_dim;
    let seq = params.seq_len;
    let head_off = head * hd;
    let scale = 1.0 / sqrt(f32(hd));

    // Q projection
    var q: array<f32, 128>;
    for (var d = 0u; d < hd; d = d + 1u) {
        var sum = 0.0;
        for (var k = 0u; k < h; k = k + 1u) {
            sum = sum + input_seq[pos * h + k] * qkv_weight[(head_off + d) * h + k];
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
                k_val = k_val + input_seq[t * h + kk] * qkv_weight[(h + head_off + d) * h + kk];
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

    // Weighted sum of V → stage this head's slice
    for (var d_out = 0u; d_out < hd; d_out = d_out + 1u) {
        var attn_val: f32 = 0.0;
        for (var t = 0u; t < seq; t = t + 1u) {
            var v_val: f32 = 0.0;
            for (var kk = 0u; kk < h; kk = kk + 1u) {
                v_val = v_val + input_seq[t * h + kk] * qkv_weight[(2u * h + head_off + d_out) * h + kk];
            }
            attn_val = attn_val + (scores[t] / sum_exp) * v_val;
        }
        attn_scratch[pos * h + head_off + d_out] = attn_val;
    }
}

// ── Out-Projection (phase 2, shared by column + row) ──────────────
// Issue 701 H-C: the out-projection consumes the CONCATENATION of all
// heads' staged outputs, so one thread owns one position's full H-length
// row: out[pos][d] = Σ_k attn_scratch[pos][k] · wo[d][k]. The previous fused
// per-(pos, head) form computed attn_val · rowsum(wo), which is not the
// matvec (and is only coincidentally correct for identity `wo`).

@compute @workgroup_size(1, 1, 1)
fn m2p_out_projection(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;
    if (pos >= params.seq_len) { return; }

    let h = params.hidden_dim;
    for (var d = 0u; d < h; d = d + 1u) {
        var sum: f32 = 0.0;
        for (var k = 0u; k < h; k = k + 1u) {
            sum = sum + attn_scratch[pos * h + k] * out_weight[d * h + k];
        }
        output_seq[pos * h + d] = sum;
    }
}
