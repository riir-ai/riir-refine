# Research 318 — Batch 281: the upstream llama.cpp trio

**Status:** RECORD — landed +3 own rules, corpus 966 → 969 (Plan 285)

## Provenance

Source: ggml-org/llama.cpp (MIT), three MERGED PRs — every quote taken from the
machine-fetched `.patch` text this session (the `pull/NNN.patch` URLs; verbatim by
construction, no local clone needed):

- **#30837** @ `60341eb74d482a2c` (thelittlefireman, merged 2026-10-23) — one line
  in `ggml/src/ggml-cuda/mmq.cuh`: the DP4A MMQ tile-height `static_assert`.
- **#30836** @ `c295a3d9dcbb41ae` (Carl Philipp Klemm, merged 2026-10-20) —
  `ggml_cuda_mul_mat_id` + `ggml_cuda_mul_mat_id_needs_sync` pass the DEVICE's
  warp size into `ggml_cuda_should_use_mmf` where they passed the compile-time
  `WARP_SIZE`.
- **#26631** (ngxson; 8-patch series, head `3332eff1fb170c18`, merged 2026-10-24)
  — "allow MUL_MAT_ID -1 index to skip the computation"; its patch 4/8 is a
  cherry-pick of `d8a4b9ef` (yomaytk, the webgpu barrier-uniformity repair).

## The claim's adjudications (from the 31530c184 claim row, discharged)

- **#30836 verify-out-of-lane RESOLVED**: the fix is behavior-neutral on NVIDIA
  (warp_size 32 == WARP_SIZE 32) and bites exactly on wave64 (RDNA/HIP) — as a
  LAW it is portable to any target where the physical warp differs from the
  compile-time constant. Landed as a FACE of the warp-size rule, not a separate
  rule: #30837 is the static arm (the instantiation asserts the tile against its
  physical warp), #30836 is the runtime arm (the dispatch predicate consumes the
  device's warp size). One law, two arms — the host-dispatch-predicate family's
  warp-size instance.
- **#26631 CUDA exclusion discovered at conversion**: the claim said "all
  backends" (from the PR title/scope) but patch 7/8 ("exclude CUDA from this
  PR", 2026-10-24) REMOVED the CUDA-side implementation — the landed state has
  CPU/Metal/OpenCL/SYCL/Vulkan/WebGPU/Hexagon/OpenVINO/ZendNN implementing the
  skip contract while CUDA/ROCm/MUSA are test-marked NOT_SUPPORTED with
  `TODO: [TAG_MUL_MAT_ID_SKIP]`. The exclusion became the rule's SURFACE 4
  (the adoption boundary fails loud) — an honest posture record, not a scope cut.

## The three rules

### 1. L `physical-warp-size-keys-tile-legality-and-dispatch` (entries_gpu_127)

Tile-geometry legality + dispatch granularity key on the PHYSICAL warp size.
Static arm verbatim: `constexpr int warp_size = ggml_cuda_get_physical_warp_size();`
+ `static_assert(I % warp_size == 0, "DP4A MMQ tile height must be a multiple of
the physical warp size");` — per-instantiation, at build time. Runtime arm
verbatim: `const int warp_size = ggml_cuda_info().devices[ggml_cuda_get_device()].warp_size;`
+ the should-use call reading `warp_size` where `WARP_SIZE` stood — the wave64
repair (tile width "not devisiable by 64" [sic] in mul_mat_id's mmf reduction).

### 2. H `sentinel-id-skip-shrinks-work-set-zero-unowned-rows` (entries_gpu_127)

The op-level skip contract, four surfaces: (1) OWNERSHIP — the matmul writes only
expert-owned rows; skipped rows get a dedicated zero pass (metal: "zero the rows
that no expert owns - disjoint from what kernel_mul_mm_id writes"; CPU: the
`memset` + `continue` minimal form). (2) RE-BOUND — the shrunk compact set is
never consumed at its dense length (metal map0: `sids[i20] = id == -1 ? 0xFFFF :
(uint16_t) id;`; vulkan: clamp + mask-at-reduce; SYCL:
`routed_row_src.resize(expert_row_offsets[n_as]);`). (3) UNIFORM EXIT — the skip
is per-block uniform, licensing the whole-block/warp early exit (mmvf/mmvq
comments verbatim in-rule). (4) ADOPTION BOUNDARY — lagging backends refuse loud
(NOT_SUPPORTED), never silently wrong.

### 3. E `all-threads-reach-workgroup-barrier-no-pre-barrier-return` (entries_gpu_127)

Arrival universality: a workgroup barrier is a rendezvous; the pre-barrier
`if (!found) { return; }` is a group hang. The repair moves the gate to the
post-barrier stores (`if (found && subgroup_invocation_id == 0) {`,
`if (found && thread_id < OUTPUTS_PER_WG> {`) and optionally the accumulate. The
line it draws with rule 2: block-uniform skip = whole-group exit before a phase
nothing waits on; lane-local predicate = gate the stores, never the arrivals.

## Count-neutral rider

The vulkan clamp-then-mask face (`expert_id = expert_skip ? 0 : uint(expert); //
clamp, so that the offsets below stay in range` + the `FLOAT_TYPE(0)` mask) rode
B17 `clamp-tile-indices-to-sentinel-discard-at-write` (entries_gpu_01) — the same
compute-on-safe-index, discard-at-write split at the op boundary. The batch17
concepts pin moved from full-string `contains` to a seed-prefix `starts_with`
(append-only riders otherwise re-break it every rider batch).

## Dedup verdicts (grep pass pre-authoring)

- physical warp / tile height / should_use_mmf: ZERO owners; `wave64` appeared
  only as gpu_15's passing constants note.
- `gather-signed-index-wrap-semantics` (gpu_72) owns the WRAP arm of the
  signed-id family (−1 → last row, still computes); rule 2 owns the SKIP arm
  (no row, zeroed output) — named as kin, the confusion is the failure mode.
- `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` (gpu_88) owns
  the compact-prepass STRUCTURE; rule 2 owns the sentinel ID SEMANTIC.
- `broadcast-uniform-skip-reduce` (gpu_02), `batch-composition-invariant-grid-
  early-return` (gpu_14): different uniformity/early-return classes; named.
- workgroup-barrier arrival universality: ZERO owners.

## Fixtures

Original minimal Rust reproductions (no upstream string literals), compiled AND
test-run pre-assembly — 6/6 PASS after 3 fixture bugs caught by running (the
B277 lesson again): a missing `&` borrow, a wrong filter-destructure pattern
(`&&(_, id)` vs `&(_, &id)` on enumerate), and a WRONG WRONG-TILE for the
constant-blessed-illegal-geometry story (48 is a multiple of neither warp; 96 is
the correct tile: blessed by 32, refused by 64). The opt-pattern static arm
models the per-instantiation const assert with a generic associated const
forced at `new()`.

## Gates

batch281 needles+parse+category wiring FIRST-RUN PASS (2/2) after the count
pass; kernel_opt lib 3228/0 (+2 the new gates); default lib 2729/0; clippy −D
×3 postures clean (the box's hard-link incremental notices only); retrieval
floors HELD 22/22 (kernel_opt+latent_retrieval) + 21/21 (default posture) — no
re-pin; freeze/score-bench NOT owed (kernel_opt-only, P30/B250/B276 precedent);
binary rebuild NOT owed (kernel_opt outside the release set). Count pins via
ONE shape-asserted python pass: 37 replacements (tests.rs 36 + router.rs 1)
with ZERO residue; the batch280 witness message prepended; the Plan-003
per-category expected table re-pinned (E 87→88, H 40→41, L 28→29); the stale
"Sum must equal 953" comment (drifted since B275) repaired to the current chain.

Session: shikuwa-4090-idle, 2026-10-25T~22:xx+07
