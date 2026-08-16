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
/// One thread per output element (D*K threads total). Each thread loops over
/// `h` elements. At production (D=4, K=32, h=2048), this is 128 threads doing
/// 2048 FMAs each — a single workgroup of 256 threads suffices.
///
/// ## Dispatch
///
/// `CubeCount::Static(ceil(D*K / 256), 1, 1)`, `CubeDim::new_1d(WEAVER_CUBE_SIZE)`.
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

    // Use CUBE_POS_X + UNIT_POS (both u32) instead of ABSOLUTE_POS (usize)
    // to avoid usize/u32 mixed-type arithmetic errors in CubeCL.
    let cube_size = WEAVER_CUBE_SIZE;
    let tid = CUBE_POS_X * cube_size + UNIT_POS;
    let total = params[2usize] as u32 * k; // d_depth * K

    if tid >= total {
        terminate!();
    }

    // Decode di from tid (ki is implicit: gathered row base = tid * h
    // already folds ki in — Issue 698 A-H11 deleted the dead `_ki` decode).
    let di = tid / k;

    // u_final row: position di+1 (skip verifier pos 0).
    let uf_base = (di + 1u32) * h;
    // gathered row: di * K * h + ki * h = tid * h.
    let g_base = tid * h;

    let mut acc = f32::new(0.0f32);
    let mut j = 0u32;
    while j < h {
        acc = acc + u_final[(uf_base + j) as usize] * gathered[(g_base + j) as usize];
        j += 1u32;
    }
    residual[tid as usize] = acc;
}
