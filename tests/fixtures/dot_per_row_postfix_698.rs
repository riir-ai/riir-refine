/// CubeCL dot-product-per-row kernel for Weaver residual logits (Plan 436 T3.3).
///
/// Computes `residual[di*K + ki] = u_final[(di+1)*h..] · gathered[di*K*h + ki*h ..]`
/// — one dot product of length `h` per (di, ki) pair.
///
/// Mirrors CPU step 6b (`weaver_forward_into` L1230-1233):
/// ```text
/// residual_flat[r_off + ki] = dot(h_weaver, grow);
/// ```
///
/// ## Plane-parallel layout (Issue 698 A-H1 restructure, 2026-08-16)
///
/// One PLANE per output row (not one thread): `row = ABSOLUTE_POS_X /
/// PLANE_DIM`, lane-strided accumulation `k = lane, lane + PLANE_DIM, ...`,
/// hardware `plane_sum` reduction, lane 0 writes. This is the same pattern
/// as `gemv_plane_f32` — the pre-restructure kernel pinned the whole op to
/// ONE thread per row doing `h` serial FMAs (at production D=4, K=32, h=2048:
/// 128 rows on 128 threads of a single workgroup, each striding `gathered`
/// un-coalesced — the Issue 698 A-H1 "16× read amplification" cliff). The
/// plane layout spreads each row across 32 coalesced lanes and the rows
/// across `ceil(D*K/8)` workgroups.
///
/// ## Dispatch
///
/// `CubeCount::Static(ceil(D*K / rows_per_wg), 1, 1)`, `CubeDim::new_1d(WEAVER_CUBE_SIZE)`
/// where `rows_per_wg = WEAVER_CUBE_SIZE / plane_size` (8 on Metal).
///
/// ## Params
///
/// `params = [h, K, D]`.
#[cfg(feature = "cubecl_runtime")]
#[allow(clippy::assign_op_pattern, reason = "CubeCL macro expansion generates this pattern; compound assign on cubecl types triggers NativeExpand bug")]
#[cube(launch_unchecked)]
fn dot_per_row_f32(
    u_final: &[f32],
    gathered: &[f32],
    residual: &mut [f32],
    params: &[f32],
) {
    let h = params[0usize] as u32;
    let k = params[1usize] as u32;

    // One plane per output row (Issue 698 A-H1): lane-strided dot + plane_sum,
    // mirroring gemv_plane_f32. Adjacent lanes read contiguous elements of
    // the gathered row — fully coalesced.
    let row = ABSOLUTE_POS_X / PLANE_DIM;
    let lane = UNIT_POS_PLANE;

    let total = params[2usize] as u32 * k; // d_depth * K

    if row >= total {
        terminate!();
    }

    // Decode di from row (ki is implicit: gathered row base = row * h
    // already folds ki in — Issue 698 A-H11 deleted the dead `_ki` decode).
    let di = row / k;

    // u_final row: position di+1 (skip verifier pos 0).
    let uf_base = (di + 1u32) * h;
    // gathered row: di * K * h + ki * h = row * h.
    let g_base = row * h;

    // Cooperative dot: lane j handles elements j, j + PLANE_DIM, ...
    let mut acc = f32::new(0.0f32);
    let mut j = lane;
    while j < h {
        acc = acc + u_final[(uf_base + j) as usize] * gathered[(g_base + j) as usize];
        j += PLANE_DIM;
    }

    let result = plane_sum(acc);

    if lane == 0 {
        residual[row as usize] = result;
    }
}
