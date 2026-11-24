# Research 443 — Batch 410: slice (3) the vendored ml-explore/mlx tree (the 2D-conv + col-reduce clusters)

**Status:** DISTILLED — batch LANDED same session (Plan 409, commit at the landing row in `.distill/001_mining_queue_snapshot.md`).

## Source

ml-explore/mlx @ `c69303b00f5f61860436e832f23fa8c34b96f93a` (MIT — LICENSE verified at the bytes), read as the
**vendored `lib/mlx-src` submodule of ddalcu/mlx-serve @ tip `2201ec9`** (ls-tree-verified: the submodule pin at
the CURRENT tip is `c69303b0`). This is Research 432's named follow-up pool item — "the vendored ml-explore/mlx
tree (its own scale)", the B398 visit's disclosed unread submodule residue ("Submodules not pulled: `lib/mlx-src`
... its own future distill surface") — staged by Research 436 as "slice (3)". The B398/B242 lanes never pulled the
submodule; this batch is its first read as a surface.

Recon numbers at the pin: `mlx/backend/metal/kernels/` 60+ files; the clusters read end-to-end this batch:
conv.metal (712) + conv.cpp (1815) + steel/conv/{kernels,loaders} (2204) + reduce.metal (176) +
reduction/{ops,reduce_col,reduce_row,reduce_all,reduce_init}.h (1138) + reduce.cpp (1074) + utils.h (truthiness
half). Excluded (recorded): quantized/steel-attention/SDPA surfaces (mined via the mlxfast vendor copies +
B404's `#4927` pack), fft (radix.h 363 + readwrite.h 712 — niche), hadamard.h (182 — the FWHT family is heavily
mined), scan.metal (110 — small), sort.metal (82 — ordinal-map family owns), CPU backend, python/ surface.

**Upstream-past-pin deltas: 64 commits** (`c69303b0..origin/main` at T0, three touching the mined files:
`5c8b09e8` contiguous-flag-after-reshape, `52e02c58` merged-axes-overflow-in-reduce, `777d7417` empty-output
handling). UNSWEPT — a future watch surface (the standing weekly watch already tracks upstream mlx commits; the
watch's next window should diff against THIS pin, not the last watch pin).

## Method

Protocol A plan-first (plan 409); claim pushed pre-work (`af5f66fd`); recon at a /tmp clone (removed at close);
T1 dedup greps at 3+ vocabularies (conv/winograd/depthwise/im2col/implicit-gemm; reduce/2pass/col-reduce/
partition/tier; bounds/mask/hoist; strided-view; capacity/budget) over `src/domains/kernel_opt/` +
`src/domains/rust_perf/`; T2 two parallel reader subagents over the two clusters' REMAINDES (reduce_row/ops/
arg_reduce/reduce_all + steel_conv/steel_conv_general/loader_channel_n/loader_general/params + the explicit-gemm
hosts), coordinator re-verified EVERY load-bearing quote at the pin before authoring (11 verification reads);
fixtures `b410_fixtures.rs` standalone-compiled AND executed BEFORE entry (debug AND -O, 7/7 green; the gate
caught FOUR fixture defects pre-entry: the R2 shape-gate inputs failed the fixture's own `c+o >= 256` conjunct,
the R1 scalar closed form had the transposed index order (`b[i][k]` vs `b[k][i]`), the R4 tamper predicate was
STRICTER not looser (`<` vs `<=` — a perf defect shape, not the OOB shape; the honest loose predicate is
`first < bound`), the R2 tamper's budget zeroed at the small working set so the honest contrast is None vs
Some(1)); 3 of the 6 rules' `r"..."` strings carried embedded quotes (the assert messages inside opt_pattern) —
the B402 raw-string class, converted to `r#"..."#`; 11 mechanicals (unnecessary_hashes after the conversion)
healed via `cargo clippy --fix` (the maintained binary is stale at e4b2dd31 — the rebuild folds into the next
maintained-binary lane, the B409 disclosure repeated).

## Landed (kernel_opt 1115 → 1121, entries_gpu_166)

1. **`winograd-fixed-transform-as-simdgroup-mma-chain`** (N, the LEAD) — a FIXED small transform rides the
   simdgroup/tensor matrix units as one mma chain per tile ((G·g)·Gt / (Bt·I)·B / Bt·(O·B)); the constants
   loaded once as two thread_elements per lane; the boundaries applied where data lands (zero-fill at pack,
   per-element store gate), never in the mma. Dedup: zero winograd owners; the butterfly/FWHT family owns
   pairwise-exchange transforms (no matrix product) — distinct substrate.
2. **`winograd-admission-budget-floor-routes-to-generalist`** (D) — shape conjuncts + working-set budget
   (recommendedMaxWorkingSetSize/4*3, used subtracted first) + the min_rows_per_tile=32 amortization floor
   that returns 0 (the caller falls through to implicit GEMM). The self-refusal pole vs
   sibling-kernel-admission-gap-absorbed-by-generalist. The unfold twin carries as faces inline
   (maxBufferLength division + loud single-row refusal + scratch-max-tile strided views).
3. **`identity-guarded-atomic-elision-packed-bool-word`** (B) — the identity guard elides the RMW; the packed
   bool4 word carries four lanes in ONE atomic (single-lane-clearing mask under fetch_and); Sum's bare
   fetch_add is the deliberate asymmetry (ops.h:131-134 — the no-guard pole is decision content).
4. **`whole-tile-inbounds-hoisted-loader-fast-path`** (D) — the tile-granularity predicate computed once at
   loader construction buys the unguarded unrolled copy; three tiers incl. whole-tile zero-fill; the boundary
   clause: the INPUT loaders keep per-row guards (their predicate is not a tile property). The col_reduce
   safe-split + all_reduce hoist cited as the reduction-side family instances.
5. **`nan-preflight-preserves-propagation-parity`** (N) — a NaN-lossy hardware lane intrinsic gets a
   `simd_any(val != val)` preflight at EVERY stage (lane + pairwise + butterfly agree, or the result depends
   on which stage observed the NaN); the integer enable_if carve-out; per-component complex propagation.
   Kin: nan-preserving-select-over-fmaxf (the select-form member — the family's second substrate).
6. **`logical-reduce-width-bitcast-float-carveout`** (N) — all/any remaps its read type by WIDTH; floats
   exempt (`-0.0 compares equal to zero but has a bit set`); the magnitude-bits truthiness test
   (`& 0x7FFFFFFF) != 0`); sub-4-byte integers read SIGNED for the bool-target constraint.

+5 count-neutral FACES on their OWNERS (quotes byte-verified at the pin):
- `attention-partition-two-knob-shared-partial-workspace` (gpu_86) += the all-reduce byte-tier two-pass
  instance (nbytes() vs 2^26 pins the partial width 32·READS/1024·READS + pass-2 threads 32/1024 —
  reduce.cpp:363-376).
- `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` (gpu_78) += the group-conv
  materialize-for-contiguous-slices pole (conv.cpp:236-244 + the view pole :131-138 — the consumer's slicing
  shape decides).
- `smem-staging-gated-by-cross-thread-reuse` (gpu_04) += the depthwise span-tile halo (TGH = th·2 + 6, 9
  outputs per staged element, the c_per_thr=8 channel-interleave fill — conv.metal:192-289).
- `tail-clamped-row-reuse-widens-fast-kernel-admission` (gpu_75) += the store-side backward realignment
  (reduce_row.h:282-286 — out_size − N_WRITES clamp-back, idempotent overwrite keeps the hot loop mask-free;
  the accumulating-sink anti-regime).
- `pad-tail-to-fixed-tile` (gpu_08) += the exhausted-axis zero-fill tier (loader_channel_n.h:135-145 —
  `weight_hw >= wS[1]·wS[0]` → structural zero-fill + return).

## Staged for the next batch (recorded, nothing hidden)

- The small-filter per-row bounds bitmask (loader_channel_l.h:273-297 — `mask_h[i] |= in_bounds << kh`
  construction + the bit-test load path) — the mask-precompute pole vs R4's hoisted branch.
- The dilation base tables + kernel odometer (conv.cpp:514-554 `base_h`/`base_w` + loader_general.h:361-377
  `next()` wrap-to-phase-base) — the tap-subsequence precompute for dilated/strided consumers.
- The simd-reduce width dispatch + the 8-byte launch cap (ops.h:8-20 enable_if split + reduce.cpp:494-496
  threadgroup 512 — the kernel fallback + host tightening pairing).
- The col-reduce tier dispatch (reduce.cpp:947-976 — small <32 / longcolumn stride<32 && size≥1024 / 2pass
  >256 && out/32<1024 / looped; longcolumn's 32→128 at total≥32768).
- Research 437's own 8 staged candidates (the mlxfast windows) remain queued ahead.

## Honest caveats

- ONE batch slice of a tree at "its own scale" — the excluded surfaces above stay unread; the CPU backend
  (blas/conv/einsum analogs) never entered any lane's T0.
- The upstream-past-pin deltas (64 commits) are UNSWEPT; the pin is what mlx-serve ships, and the watch owns
  upstream movement.
- All six rules + five faces are suggestion-only `NotBenchable-by-domain` (no Metal runner in the validator);
  every GOAT fixture arm is CPU-testable and pinned (the transform equality, the admission arithmetic, the
  elision counts, the tier dispatch, the parity clause, the carve-out classes).
- The reader subagents' quote sites were re-verified by the coordinator at the pin (11 reads); reader A's
  `commutative`-flag lead did not survive re-reading (no such flag exists — the Ops carry simd_reduce_impl +
  init + atomic_update only) — dropped pre-authoring.

Nothing invented — every quote read at the clone at pin `c69303b0` this session. Session: zed-amo-i410@amo,
epoch 1795492000.
