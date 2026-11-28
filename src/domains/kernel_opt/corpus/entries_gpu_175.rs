//! Plan 421 — the mlx 83-commit window kernel_opt tranche (Research 350's
//! window over the B410-412-era pin `c69303b0`; T0 clone `9cb7f69b`, +84
//! commits, MIT). THREE rules — the int8 affine NAX lane (confirmed absent
//! from B410-412: all their landed members are the fp/Winograd/reduce
//! slices), the CUDA row-reduce split admission, and the duration-keyed
//! split cap — plus the count-neutral faces riding their owners in the
//! same commit (mma-operand-limb-hi-lo → entries_gpu_159's Dekker-limb
//! rule; biased-code-to-signed-decode → entries_gpu_09's decode family;
//! tiles-per-core-admission-floor → entries_gpu_119's admission family;
//! the anchor half → the sec tranche's magic-anchor rule in-rule). ALL
//! suggestion-only NotBenchable-by-platform (CUDA/Metal occupancy and
//! wall metrics — the kernel_verify STUB posture; the admission
//! arithmetic is CPU-testable and IS the fixture). Fixtures preflight:
//! `b421_kernel_fixtures` standalone-compiled AND executed pre-entry
//! (debug AND -O).
use crate::domains::kernel_opt::KernelRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![

        KernelRule {
            rule_id: "group-boundary-int8-mma-deferred-descaling".to_string(),
            gpu_concept: r"MECHANISM (quantized-GEMM accumulator discipline; the int-exact-accumulate class): an affine-quantized GEMM keeps its hot
            accumulation in EXACT integer MMA lanes — int8 operands into int32 accumulator fragments, no precision lost inside the group — and pays
            the dequantizing scalar ONCE per group boundary (the rescale point the loop already has), folding the integer fragment into the float
            output tile at the boundary; the affine offset rides as ONE extra MMA against precomputed operand sums at the end, never a per-element
            bias pass. THE SOURCE LAW, verbatim (ml-explore/mlx @ 9cb7f69b, mlx/backend/metal/kernels/quantized_nax.h:2466-2470, the
            qmm_t_nax_staged_impl header): 'qmm_t_nax_staged_impl with 8-bit integer MMAs. x comes from affine_quantize_rows as x = sx * c, with
            rows padded to a multiple of BM. The weights are w = s * e + f, so when a group ends it adds sx * s * sum(c * e). At the end one MMA adds
            f * sum(x) of all groups.' — the algebra is the whole law: x = sx*c and w = s*e + f give x·w = sx*s*(c·e) + f*(sx*c·1), so the int
            product c·e runs EXACT in the MMA pipe, the scale product sx*s folds once per group, and the affine term f*sum(x) is a single extra MMA
            against the precomputed row sum — three rescalings per GROUP, not per element. THE HI/LO SUM LIMB rides the same window
            (quantized_nax.h:2346-2350, verbatim): 'const float sum = scale * qsum; const T sum_hi = static_cast<T>(sum);' + 'xt[K_g + g] =
            static_cast<T>(sum - float(sum_hi));' — the per-group sum carried as hi + lo limbs so the extra MMA recovers what one narrow operand
            would lose (THE DEKKER LIMB LAW — count-neutral FACE on entries_gpu_159's fp32-input-hi-lo-two-fp16-mma-columns, same mechanism,
            the precomputed-stored-in-tensor twist; landed there, never duplicated here). OVERFLOW SAFETY is a regime condition, not an
            assumption: int8 x int8 sums into int32 saturate only past ~2^24/8 MACs — the group size (<= 128 at the 4-bit/8-bit affine groups) keeps
            the exact envelope by construction. ⛔ ADJUDICATIONS (both directions, dedup greps at landing HEAD): distinct-from gpu_136's
            deferred-correction-rides-rescale-periodic-flush — THAT folds a correction INTO an existing periodic rescale (the anchor algebra,
            numerics); THIS defers the DEQUANTIZING SCALE to the group boundary the quantized loop already has (accumulator-discipline axis) —
            adjacent periodic-boundary genus, compose on a kernel that has both hazards. Distinct-from B116's swar-lane-widened-base3-dp4a-dot +
            the isum family — THOSE decode-and-dot in scalar SIM lanes; THIS rides the hardware int8 MMA accumulator (the same quantized-dot
            genus, a different execution substrate; cite). Distinct-from gpu_67 splitk-reduce-one-launch / gpu_117 splitk-tier-ladder — the split
            depth is entries_gpu_175's OTHER rule (class-relative-split-cap-min-partition-work), never this one. REGIME: quantized GEMMs on
            hardware with integer MMA paths where the quantization group gives a natural rescale boundary and the int accumulator cannot overflow
            within a group. ANTI-REGIME: scalar-dot decode paths with no MMA accumulator (the scale folds per element there anyway); fp paths
            with no dequant step; groups so large the int accumulator could saturate (the envelope is a regime condition — review confirms, the
            fixture pins the small-group exactness). NotBenchable-by-platform (the win metric is Metal NAX MMA-vs-FMA throughput — no corpus
            validator host; the int-exact-in-group + boundary-rescale arithmetic is CPU-testable and IS the fixture). Lane context: the int8
            affine NAX lane was confirmed ABSENT from B410-412's mlx members (all their landed slices are fp/Winograd/reduce — verified at the
            T0 diff, theBatch-409 prior-coverage lesson). UPSTREAM: ml-explore/mlx MIT, PRESENT at the pin — decision content only, never a PR.
            Source: ml-explore/mlx @ 9cb7f69b (T0 2026-11-28; every quote byte-verified at the clone). Batch 430-tranche / Plan 421 / Research
            350's window.            ref_shape: r"// NAIVE: dequantize to float and accumulate in the float pipe —
// the int8 inputs pay a convert per element and the accumulator
// never sees the integer MMA at all, or (the other naive arm)
// rescales into the accumulator per element.
fn qmm_tile(acc: &mut Tile, x: &[i8], w: &[i8], sx: f32, sw: f32) {
    for k in 0..K {
        let xf = (x[k] as f32) * sx; // dequant per element
        let wf = (w[k] as f32) * sw; // ... twice
        acc.fma(xf, wf); // the float pipe does all the work
    }
}".to_string(),
            opt_pattern: r"// FIXED: integer-exact MMA inside the group, the dequantizing
// scalar paid ONCE per group boundary (the rescale the loop
// already has), and the affine bias rides one extra MMA against
// precomputed operand sums at the end.
fn qmm_tile(acc: &mut Tile, x: &[i8], w: &[i8], sx: f32, sw: f32, f: f32, xsum: i32) {
    let mut acc_i = 0i32; // int8 x int8 -> int32 fragments, exact in-group
    for k in 0..K {
        acc_i += (x[k] as i32) * (w[k] as i32); // the MMA pipe, exact
    }
    acc.fold_in(acc_i, sx * sw); // the descaling, ONCE per group boundary
    acc.fold_in_bias(f * xsum as f32); // the affine term: one MMA vs sum(x)
}".to_string(),
        },
        // rule 2 and rule 3 follow
    ],
}
