# Research 030 — riir-gpu Decode-Path Fusion Cluster (Batch 29): Readback Size-Gate + Pointer-Identity Handle Cache + Queue-Drain Cost Model + Register-Phase Epilogue + Delayed RMS Scale + Packed Params + Enum Kernel Variants + Workgroup-Local Fusion + Unchecked-Launch Contract + Fast-Math Feature Gate

**Date:** 2026-08-14
**Source:** `riir-ai/crates/riir-gpu/src/deltanet_input_proj_fused.rs` (590 lines — Issue 602: the 4 DeltaNet input-projection ternary GEMVs `qkv`/`z`/`a`/`b` chained into one command buffer) + the `epilogue/` cluster (2664 lines, 4 files — Plan 106 Track 3, CODA = Research 67: `mod.rs` + `coda_primitives_cubecl.rs` + `gemv_residual_cubecl.rs` + `norm_residual_cubecl.rs`) — the sixteenth batch mining the stack's own kernels, targeting the decode-path dispatch-fusion surface the sibling is NOT working in (`ternary_deltanet_gpu_forward.rs` left for a later batch to avoid collision).
**Corpus:** 193 → **203 rules** (A 27→29, B 33→34, C 19→21, D 26→29, E 11→12, M 5→6).

## 1. Coverage

Deep-read via two parallel mining subagents + synthesizer verification of every load-bearing quote (all quotes below re-verified against the source by the synthesizer). On the input-proj side: the fused dispatch launcher (`dispatch_input_projections_noread`/`_with_readback`), the Bench 437 contrast block, the 4 sequential `read_one` calls, the `TernaryInputProjHandles` pre-upload shape, and the `GpuTernaryInputProj` hook with its papaya pointer-key cache + preupload/on-the-fly miss paths. On the epilogue side: the `GemvEpilogue` enum + metadata methods, the `DispatchBudget` checkpoint consts + tests, the three kernel families (GEMV+residual f32/f16 twins, norm+residual decode/batched, PartialRMS/NormWeightScale/RowScale/SwiGLU primitives), the `CodaReparameterization` identity doc, and the fused-vs-separate equivalence tests. Honest file count: **49 of ~200 mined** (~25%).

Identity observation vs expectation: the epilogue cluster looked like pure fusion territory (heavy `fuse-*` corpus overlap), but its real value is the CONDITIONAL structure of fusion — WHEN it is legal (workgroup-local ownership), WHEN it is profitable (dispatch/command-buffer overhead, not bandwidth, per the source's own correction), WHEN deferral backfires (the Bench 437 size gate) — plus the DSL-constraint-driven authoring shapes (enum variants, packed params, unchecked launches) that have no corpus coverage. The cluster is also partially unwired (`PartialRmsCubeCL`/`NormWeightScaleCubeCL`/`RowScaleCubeCL` are module-private, `#![allow(dead_code)]`, "await epilogue-visitor wiring") — the rules describe shipped kernel code, not shipped forward-pass wiring.

## 2. The ten new rules

### 2.1 `defer-readback-gated-by-live-output-size` (A)

Source: deltanet_input_proj_fused.rs L17-18 ("All 4 dispatches are queued without `read_one`. Only the final outputs are read back — **ONE GPU sync point instead of 4**.") + L20-32 (the Bench 437 contrast: "Bench 437 kept Q/K/V/gate/up outputs ALL alive simultaneously (5 concurrent buffers each ~70 KB for the FFN case), which increased Metal memory pressure → 16% slower." vs "Total intermediate: ~8 KB. Negligible memory pressure.").

Mechanism: deferring all sibling readbacks to one final drain is NOT an unconditional win — the failure boundary is total simultaneously-live output bytes. ~8 KB of input-projection outputs chain freely; ~350 KB of concurrent FFN buffers made the same transformation 16% slower via memory pressure. The rule converts "defer readback" from an always-apply transformation into a footprint-gated one.

Distinct from `defer-readback-until-after-submit` (the timing rule — this is the precondition), `chain-dispatches-single-sync-point` (the dispatch-side mechanism — this is the sizing decision), `fan-out-shared-input-single-sync` (the pattern instance — this is its failure boundary).

### 2.2 `weight-handle-cache-keyed-by-allocation-identity` (A)

Source: deltanet_input_proj_fused.rs L279-281 ("a handle cache keyed by the qkv weight's pointer identity (each layer has a unique qkv allocation)") + L301-302 (`papaya::HashMap<(usize, usize), TernaryInputProjHandles>`) + L323-327 (key = `(qkv.pos_bits.as_ptr() as usize, qkv.neg_bits.as_ptr() as usize)`) + L361-372 (hot-path lookup + on-the-fly miss upload).

Mechanism: when a GPU offload hook's API must accept plain CPU weight refs (can't require callers to thread device handles), key the pre-uploaded handle cache by the CPU allocation's pointer identity. O(1) lock-free hot path; graceful on-the-fly miss. The keying is what keeps the cache invisible to the API surface.

Distinct from `host-cache-immutable-device-weights` (the upload-once rule — this is the identity-keyed transparent lookup layered on top) and `per-dispatch-pointer-re-extraction` (device pointers re-read per dispatch — CPU heap pointers as a persistent key, opposite direction).

### 2.3 `first-readback-pays-queue-drain` (D)

Source: deltanet_input_proj_fused.rs L242-245 ("CubeCL's `read_one` forces a sync; the 4 calls here all drain the same command queue (the dispatches were already queued by dispatch_input_projections_noread), so the cost is ~1 sync + 4 small readbacks rather than 4 independent syncs.").

Mechanism: after one dispatch burst, the FIRST blocking read drains the queue; later reads of completed buffers return immediately. Sync count is a function of dispatch BURSTS, not read CALLS — no per-read defensive syncs, no batched-download machinery needed when reads follow one burst.

Distinct from `batched-downloads-single-sync-cycle` (restructures into one submit/map/poll — this is the ordering-only alternative that makes it unnecessary) and `chain-dispatches-single-sync-point` (dispatch half — this is the read-side cost model).

### 2.4 `epilogue-at-register-write-phase` (C)

Source: epilogue/mod.rs L8-11 ("CODA (Research 67) reparameterizes Transformer computation as GEMM-plus-epilogue programs. The key insight: memory-bound operations (normalization, residual adds, activations) can execute *while the GEMV output is still in registers*, avoiding global memory round-trips.") + gemv_residual_cubecl.rs L103-109 (`let result = plane_sum(partial); if lane == 0u32 { output[row] = result + residual[row]; }`) + L24-31 (2→1 dispatches per site; 52/26 layers; 104 combined).

Mechanism: in a plane-cooperative GEMV (one plane per output row), `plane_sum` leaves the full row dot in every lane's registers — the elementwise epilogue rides lane 0's existing single global store. One extra scalar read amortized over the n-length cooperative dot; one dispatch eliminated per use site. The CONDITION is register-residency at the write phase (inverse of `smem-broadcast-scalar-then-parallel-scatter`, where all threads scatter — here exactly one lane writes because there is one output element per row).

Distinct from `fuse-residual-via-in-place-output-array` (buffer aliasing for a param limit) and `merge-dispatches-into-single-compute-pass` (host-side, zero kernel changes).

### 2.5 `delay-rms-scale-past-gemv-epilogue` (C)

Source: epilogue/mod.rs L227-235 (the CODA §3.2.1 identity `RMSNorm(x @ W + z) * γ @ W' = r * ((x @ W + z) * γ) @ W'`; "This lets us compute `r` once and apply it *after* the next GEMV, eliminating a separate RMSNorm dispatch.") + L118-124 (`RMSNorm(x) @ W = r * (x * gamma) @ W`) + L251-262 (the honest accounting: "Wait — same count? The savings come from **fusing r into the last GEMV of the group** and **eliminating the pre-GEMV rmsnorm write**... The main benefit is reducing CubeCL dispatch overhead (command buffer submissions) and kernel launch latency.") + coda_primitives_cubecl.rs L20-24 (the decomposition: PartialRMS / NormWeightScale / RowScale).

Mechanism: the per-row norm scalar commutes with the downstream matmul — extract r (partial norm), apply only γ elementwise, land r in the downstream GEMV's row-scale epilogue (composable with residual). The file's own honesty is the lesson: the pure reparameterization saves ZERO dispatches (3→3); the win is the last-GEMV fusion + the eliminated normed-vector write, and with handle-to-handle passing the dominant benefit is dispatch overhead, NOT bandwidth.

Distinct from `fold-norm-scalar-into-matmul` (folds into matmul internals — this keeps the GEMV body untouched and adds the producer-side decomposition) and `fuse-add-rms-norm` (fuses residual INTO a norm — this moves the scalar ACROSS a GEMV boundary).

### 2.6 `pack-scalar-params-into-combined-buffer` (D)

Source: epilogue/mod.rs L48 ("Limited Array parameters (~4-5 per kernel) — pack params into combined buffers") + L148-154 (`total_array_count()`) + coda_primitives_cubecl.rs L51-55 (`params: [f32; 2] — [inv_dim, eps]`) + L144-146 (host pack) + norm_residual_cubecl.rs L351-354 (batched packs `dim_f` as a third f32) + L179-181 (in-kernel positional unpack incl. `let dim = params[2usize] as u32;`).

Mechanism: DSL Array-param budget met by ONE small f32 params buffer per launch; derived values (`inv_dim`) host-computed once instead of per-thread; u32s bitcast through the f32 pack when scalar args run out; variant choice coupled to the budget via metadata.

Distinct from `staging-packed-uniforms-encoder-ordered-copies` (upload-batching driver — this is argument-count-limit driver), `derive-params-from-global-invocation-id` (per-item grid params — per-launch scalars), `identical-uniforms-share-single-slot` (dedupes identical — packs distinct).

### 2.7 `enum-dispatch-separate-kernel-per-variant` (M)

Source: epilogue/mod.rs L47 ("No generics in `#[cube]` kernels — each fused variant is a separate kernel function") + L99-103 ("the enum is used at the Rust level to organize dispatch logic and kernel selection") + L137-154 (`extra_input_count`/`total_array_count` metadata) + gemv_residual_cubecl.rs L76/L140 (the f32/f16 kernel twins).

Mechanism: DSL limitation (no kernel generics) → enumerate the fusion space as a host-level `#[repr(u8)]` enum, compile ONE branch-free kernel per variant (each with exactly its own Array count), drive plumbing from enum metadata. The enum organizes dispatch; the kernel list stays flat.

Distinct from `dtype-specialized-float-kind`/`hardcoded-fast-path-jit-specialization` (dtype/shape perf specialization — DSL-limitation-driven here) and `op-family-type-dispatch` (op FAMILIES — same op, different epilogue suffix).

### 2.8 `epilogue-must-be-workgroup-local` (E)

Source: epilogue/mod.rs L49 ("No cross-workgroup sync — epilogue must be workgroup-local") + gemv_residual_cubecl.rs L50-55 ("Each plane (subgroup) handles one output row... Lane 0 performs the residual add") + norm_residual_cubecl.rs L57-59 (single-workgroup reduction is what makes the norm fusion possible) + mod.rs L371-375 (the constraint visible in the budget: attention 9 unchanged / post-attn 1 / MLP 4).

Mechanism: fusion legality is a data-OWNERSHIP question, not just dependency — an epilogue may only consume data the executing workgroup owns (per-row scalars for plane-per-row GEMV; whole-row reduction inside one workgroup for the norm). Cross-workgroup consumers (attention over KV) cannot be epilogued.

Distinct from `nonlinearity-forces-raw-input-buffers` (buffer-aliasing legality — ownership-scope legality) and `workgroup-equals-plane-native-reduction` (workgroup sizing — the fusion boundary that scope defines).

### 2.9 `launch-unchecked-caller-owned-size-contract` (D)

Source: epilogue/mod.rs L50 ("`launch_unchecked` for performance — caller must ensure buffer sizes") + gemv_residual_cubecl.rs L205-213 (the documented per-handle element-count contract) + L225-226 ("// SAFETY: Caller guarantees correct buffer sizes and plane support.").

Mechanism: elide per-launch validation (host-side overhead dominating a 14-dispatches/layer × 26-layer loop of microsecond dispatches) into an `unsafe` launcher seam with an enumerated `# Safety` size contract, verified by caller + equivalence tests instead of per dispatch.

Distinct from `cuda-graph-scalar-via-device-pointer` (moves scalars to device memory — elides validation) and `no-blocking-sync-in-async-pipeline` (elides syncs — elides checks).

### 2.10 `fast-math-contraction-behind-feature-flag` (B)

Source: gemv_residual_cubecl.rs L67-75 + L131-139 (the dual `cfg_attr` on the same kernel: `#[cfg_attr(feature = "gemv_fma_contract", cube(launch_unchecked, fast_math = FastMath::AllowContraction.into()))]` paired with the `not(...)` strict default) + L94-101 (the strided mul+add dot loop that contracts to FMA on the opt-in build).

Mechanism: a numerics-affecting compiler flag must be neither global (breaks bit-comparisons everywhere) nor absent forever — attach it to the kernel attribute via a `cfg_attr` pair on a dedicated Cargo feature: strict-FP stays the byte-identical default, correctness gates run strict, perf builds opt in. Intent inferred from the dual-compilation existence (consistent with repo feature-flag discipline; noted as inference).

Distinct from `dtype-specialized-float-kind` (dtype — compiler numerics mode) and in-kernel numeric transforms like `softplus-piecewise-saturation-clamp` (algorithm changes — zero algorithm change here).

## 3. Candidates dropped as duplicates (substrate-first checks)

| Candidate | Closest existing rule | Verdict |
|---|---|---|
| 4 sibling GEMVs share one uploaded input, dispatched back-to-back, x_handle cloned ×3 + moved on last (input_proj L172-197) | `fan-out-shared-input-single-sync` + `chain-dispatches-single-sync-point` + `same-queue-implicit-ordering-no-sync` | **Dropped** — the pattern is fully carried; the clone-N−1-move-last micro-pattern is ownership trivia. |
| Upload immutable weights once at model load (input_proj L57-60, L315-344) | `host-cache-immutable-device-weights` | **Dropped** — kept as the substrate 2.2 keys into. |
| Readback after submit, never interleaved with dispatch building (input_proj L238-246) | `defer-readback-until-after-submit` | **Dropped.** |
| f16 weights cast to f32 per-element before accumulation ("2× bandwidth savings with f32 accumulation precision") | `upcast-before-reduction` | **Dropped.** |
| Leader writes `inv_rms` into reduced `smem[0]`, sync, all threads read (norm_residual L138-145) | `smem-broadcast-scalar-then-parallel-scatter` | **Dropped** — exact instance. |
| 8-step hardcoded smem tree reduction (norm_residual L104-136; coda_primitives L83-114) | `tree-reduction-depth-over-sequential-accumulate` + `unroll-fixed-inner` | **Dropped.** |
| Strided accumulation for dim > block size (norm_residual L87-96) | `grid-stride-loop` | **Dropped.** |
| Batched norm: one workgroup per row, `row = CUBE_POS_X` (norm_residual L160-252, Plan 482 T4) | `multi-dim-grid-launch` / row-per-workgroup | **Dropped** — standard geometry; kept as a coverage fact. |
| `plane_sum()` hardware subgroup reduction (gemv_residual L104) | `warp-shuffle-intrinsics-for-sub-warp-reduction` / `single-plane-sufficient-reduction` | **Dropped.** |
| NormResidualCubeCL (norm+residual in one elementwise kernel) | `fuse-add-rms-norm` | **Dropped** — direct instance; corroborates 2.4's workgroup-scale analog. |
| `Option<&Hook>` offload seam with CPU fallback + per-site composition (input_proj L285-297) | `offload-cpu-bottleneck-to-gpu-kernel` | **Dropped** — adoption mechanics / API architecture, not a kernel mechanism. 3rd instance in the repo (Issues 599/601/602); noted as a coverage fact. |
| `DispatchBudget` const checkpoints + derived % + pinned tests (mod.rs L294-500) | `activation-recompute-budget` (budgets recompute-vs-store) | **Dropped** — quantification METHODOLOGY, not a healer-applicable transform; recorded here + in the corpus doc note instead. |
| `debug_assert` sibling dims share n_embd (input_proj L87-89) | — | **Dropped** — generic API validation. |

## 4. Latent bugs found during mining (filed as riir-ai Issue 674)

**B1 (latent, stale-cache-on-reuse): `GpuTernaryInputProj`'s pointer-key cache has no invalidation.** The documented assumption ("each layer has a unique qkv allocation", L279-280) holds within one model instance, but there is no invalidate/clear API — a long-running process that drops a model and loads another can hit allocator address reuse and silently compute with the previous model's handles. Same class as the Batch 28 finds: documented assumption, unenforced invariant, silent wrong answer.

**B2 (latent, partial key): the cache key covers only qkv — z/a/b are not keyed.** `input_projections(qkv_w, z_w, a_w, b_w, ...)` looks up solely by the qkv pointer pair (L361). A caller passing the same qkv with different z/a/b (partial LoRA swap, test fixtures) receives handles built from the first-seen z/a/b. `preupload_layers` passes consistent tuples so the intended path is safe; nothing asserts it.

**B3 (doc bug): stale dispatch numbers in epilogue/mod.rs L43** — "Total potential: ~4 dispatches saved per layer (18 → 14), ~22% reduction" contradicts the pinned consts + tests in the same file (`BASELINE.total = 19`, `CODA_TARGET.total = 14`, reduction 26.32% asserted at L499; `coda_primitives_cubecl.rs` L6 says "14 dispatches per layer (26.3% reduction vs baseline 19)").

Latent-hazard NOTES (not filed): SwiGLU≠GeGLU — `SwigluCubeCL` computes `x·sigmoid(x)` while the budget's Gemma-2 MLP step is GeGLU (tanh approx); divergent semantics if ever wired as that step's replacement (currently unwired, `#![allow(dead_code)]`). `dim` transported as f32 (exact only ≤ 2^24 — fine at Gemma dims). Hardcoded `plane_size = 32` (Metal simdgroup) vs in-kernel `PLANE_DIM` — a mismatch on other backends corrupts row indexing. Redundant final `sync_cube()` in `partial_rms_f32` (harmless).

## 5. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 correctness | 10 rules valid pseudocode — NEW standing test `batch29_pseudocode_parses` syn-parses every new rule's ref_shape AND opt_pattern via the L2 pruner; 0 duplicates (13 substrate-first checks with explicit distinct-from/drop notes; 2-way subagent + synthesizer verification of every load-bearing quote) | ✅ PASS |
| G2 perf | corpus build O(n); router O(1); 287 kernel_opt-scoped lib tests in 0.12s | ✅ PASS |
| G3 no-regression | default 190 lib tests + kernel_opt 287 + clippy 0 warnings on `--features kernel_opt --all-targets` | ✅ PASS |
| G4 alloc-free | static `KernelRule` entries; zero hot-path allocation (unchanged mechanism) | ✅ PASS |
| G5 routed demo | 203 flat → 13.5 avg = **15.0× candidate reduction maintained** | ✅ PASS |

## 6. Status

- Corpus: 193 → **203** (A 27→29, B 33→34, C 19→21, D 26→29, E 11→12, M 5→6; 15 categories unchanged — no router changes needed beyond the count pins; also fixed two stale `183` doc comments in the corpus + one in the Plan 003 invariant block).
- riir-gpu remains OPEN: ~49 of ~200 files mined. Next-richest for Batch 30+: `dllm/mod.rs` (2040 lines, Plan 068 d2f training — wgpu host orchestration, likely heavy Batch 26/27 overlap), `edge_lora/` (13 files, ~9K lines — mostly host-side training/self-play, low kernel density), or return to `ternary_deltanet_gpu_forward.rs` (234K) once the sibling lands its WIP.
- The batch-29 `batch29_pseudocode_parses` test is the first STANDING syn-parse gate on new corpus entries (batches 1-28 validated ad hoc) — future batches should extend the pattern.
- Latent-bug yield continues (B1/B2 same class as Batch 28's: documented-assumption violations). Filed as riir-ai Issue 674.
- Blocked (unchanged): centroid mining via `CommittedFieldBlend`; `kernel_perf_bench` numeric validator.
