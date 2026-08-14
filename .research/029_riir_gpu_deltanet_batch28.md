# Research 029 — riir-gpu Deltanet CubeCL Cluster (Batch 28): Row-Owned Register Blocking + Plane-Native Reduction + Broadcast-Scatter + Token Flattening + Fused Conv + Softplus Clamp + Reverse Running Decay + Split Outputs + Carry Dual Layout + Nonlinearity Buffer Contract

**Date:** 2026-08-14
**Source:** `riir-ai/crates/riir-gpu/src/deltanet_cubecl.rs` (2548 lines — decode-time DeltaNet recurrence, Plan 182 / Issue 619 rowpar) + `riir-ai/crates/riir-gpu/src/deltanet_chunked_cubecl.rs` (2057 lines — chunkwise-parallel prefill, Issue 652 / Plan 533, Yang & Wang 2024) — the fifteenth batch mining the stack's own kernels, and the first mining the two largest single files in riir-gpu.
**Corpus:** 183 → **193 rules** (B 32→33, C 18→19, D 23→26, E 9→11, G 17→19, L 5→6).

## 1. Coverage

Deep-read via two parallel mining subagents + synthesizer verification of every load-bearing quote (all quotes below re-verified against the source by the synthesizer): the legacy workgroup-per-head recurrence (Issue 610 smem-tree fix + Issue 616 broadcast-scatter fix), the row-parallel register-blocked recurrence (Issue 619 — the headline kernel), conv1d, gating (+concat), beta/decay (+batched), z-gating, L2 normalize (+expand +batched), `DeltaNetStateBuffers`, and on the chunked side: chunked conv1d (Phase 1), chunk decay (+split), weighted_v, state transition, intra output, forward decay, cross chunk, ΔS, extract/scatter bridge + the full-pipeline G1 test. Honest file count: **44 of ~200 mined** (~22%).

Identity correction vs expectation: Batch 27's note predicted "Householder/decay-matrix territory — heavy Batch 11/12 overlap". Half-true. The ALGEBRA (delta-rule family) is indeed Batch 11/12 territory — but the two files' value is the GPU EXECUTION SHAPES (row-ownership, plane-native reduction, dematerialized decay, consumer-shaped outputs), which had no corpus coverage. The chunked file is the SCALAR-decay gated delta rule (`S_t = α_t·S_{t-1} + β_t·v_t⊗k_t^T`), NOT the Householder-product variant — no W matrices, no triangular solve (the subagent honestly reported the hypotheses absent).

## 2. The ten new rules

### 2.1 `row-owned-register-blocked-recurrence` (G)

Source: deltanet_cubecl.rs L255-294 (rowpar doc: "Because a cube now *owns* its row exclusively, the row is loaded into registers once and every step operates on registers" + the table "global passes over `S` | 4 read + 2 write | **1 read + 1 write**" + "This kernel is memory-bound, so cutting 6 passes to 2 is a ~3× reduction in the quantity that actually gates it") + L264-267 ("legacy dispatch is `n_head` cubes (48 for Bonsai-27B → 192 SIMD-groups). This dispatch is `n_head × head_dim` cubes (6144 → 6144 SIMD-groups), a 32× increase in latency-hiding headroom") + L285-286 (dispatch contract) + L338-342/L382-386 (the single read/write passes).

Mechanism: rows of the state matrix S are INDEPENDENT within one decode step — narrow whole-step ownership from head granularity to (head, row) granularity, and ownership is what enables register blocking: the row never leaves registers between its one global read and its one global write. All five algebra steps (decay, retrieve, delta, update, readout) run on registers; the only cross-lane op is the hardware plane reduction.

Distinct from `chunkwise-parallel-recurrence`/`recurrent-vs-chunk-dispatch` (sequence-axis parallelism — this is state-matrix parallelism within one decode step) and `row-tile-amortize-input-load` (input-load amortization — the owned tile here is the persistent state itself, read-modify-write across steps).

### 2.2 `workgroup-equals-plane-native-reduction` (E)

Source: deltanet_cubecl.rs L256-262 ("Scattered threadgroup access costs up to 3.2× effective bandwidth. The legacy kernel runs a 7-level strided smem tree reduction per row — 1792 scattered smem round-trips per head. This kernel uses **zero threadgroup memory**: one cube is exactly one plane, so `plane_sum` reduces the whole dot product natively in registers") + L356-360 ("The cube is exactly one plane, so plane_sum covers the FULL dot product. (The Issue 610 smem tree existed only because the cube was 128 threads, i.e. 4 planes, and plane_sum caught just 1/4 of it.)") + L362-363 ("plane_sum broadcasts, so every lane has it — no barrier, no thread-0 bottleneck, no smem hand-off").

Mechanism: size the workgroup to EXACTLY one hardware plane (32 threads) and compensate the lost logical width via register blocking — the native shuffle reduction then covers the full logical reduction, broadcasts for free, and no threadgroup memory or barrier exists at all. This inverts the usual size-to-data-width choice; the Issue 610 bug (plane_sum silently covering 32 of 128 terms) is the cautionary tale for the inverse choice.

Distinct from `warp-shuffle-intrinsics-for-sub-warp-reduction` (the shuffle-instead-of-smem rule for blocks ≤ 32 — this is the LAUNCH GEOMETRY corollary: size the workgroup so the condition holds by construction) and `single-plane-sufficient-reduction` (the reduction-placement decision — this is the workgroup-SIZING decision).

### 2.3 `smem-broadcast-scalar-then-parallel-scatter` (E)

Source: deltanet_cubecl.rs L165-173 (Issue 616: "the original port (Issue 610) did the ENTIRE state update (S[row, :] += k[:] * delta_row) serially in thread 0 — head_dim serial global memory writes per row, with all other threads idle. This was the single biggest bottleneck in both the cudarc and CubeCL forwards. Fix: thread 0 computes delta, broadcasts via smem[0], then ALL threads update their own column in parallel — 1 global write per thread instead of head_dim serial writes.")

Mechanism: the write-side mirror of broadcast: after a workgroup reduction, the leader publishes the scalar through the already-reduced slot (one barrier), and every thread scatters its own column of the dependent update — 1 global write per thread instead of width serial writes from one thread. The reduced slot doubles as the broadcast channel.

Distinct from `broadcast-uniform-skip-reduce` (the READ side — uniform values broadcast, not reduced; this is the WRITE side: broadcast the reduced scalar so the scatter parallelizes).

### 2.4 `token-flattened-elementwise-modulo-shared-weights` (D)

Source: deltanet_cubecl.rs L922-941 ("The single-token kernel is pure per-head elementwise arithmetic with no cross-head or cross-token coupling, so batching is just a wider index space. `prefill` calls it once per token per DeltaNet layer — 128 x 48 = 6144 dispatches at P=128 — and each dispatch costs ~25 CPU allocations plus launch latency (Issue 638). Collapsing them to 48 is the point." + layout: per-token buffers `[p, n_head]` flat, `a_log`/`dt_bias` `[n_head]` shared, "indexed `i % n_head`" + dispatch: 256-thread workgroups, avoiding the single-token form's one-workgroup-of-n_head-threads workgroup-size ceiling) + L991-993 (bit-identical by independence).

Mechanism: mixed-arity flattening — flat per-token buffers index by `i`, per-head weight tables index by `i % n_head` — collapses thousands of independent per-token launches into ONE dispatch, bit-identical by construction.

Distinct from `batched-upcast-map` (pure wide map — this adds the mixed-arity modulo-indexed shared-weights layout).

### 2.5 `fused-conv-state-shift-dot-silu` (C)

Source: deltanet_cubecl.rs L398-448 (one thread per channel: shift conv state left by 1 + append input → depthwise dot → SiLU → write back over the input; `conv_state` sliding window modified in-place; zero intermediate buffers; state persists in GPU global memory across decode steps).

Mechanism: the decode-time stateful conv is ONE kernel, not three passes with intermediates: in-place window slide + dot from the freshly shifted state + activation written back over the input.

Distinct from `stateful-shift-via-cat-prefix` (the host-side allocating window update — this is the in-kernel in-place slide fused with dot + activation). NOTE: this in-place form is decode-only — the batched/prefill form CANNOT alias (rule 2.10).

### 2.6 `softplus-piecewise-saturation-clamp` (B)

Source: deltanet_cubecl.rs L863-874 (+batched L970-980): "softplus with clamping (matches CPU: x > 20 → x, x < -20 → 0)".

Mechanism: piecewise saturation — x for x > 20 (softplus == x to f32 precision), 0 for x < -20, transcendental only in the safe band; thresholds pinned to the CPU reference so GPU/CPU stay bit-comparable. Feeds `exp(g)` decay on a PERSISTENT recurrent state — one inf poisons the state permanently.

Distinct from `expm1-for-near-zero-exp-minus-one` (near-zero cancellation — this is far-tail saturation) and `exp2-log2-base2` (base conversion — this is domain clamping).

### 2.7 `reverse-running-decay-weight` (G)

Source: deltanet_chunked_cubecl.rs L640-643 (loop contract) + L683-712 ("Loop over j from token downto 0, accumulating inter_decay incrementally. weight starts at 1 (inter_decay[t,t] = empty product = 1)." + "Update weight for j-1: inter_decay[t, j-1] = α[j] * inter_decay[t, j]" — multiply AFTER use).

Mechanism: the intra-chunk pairwise decay matrix dematerialized into ONE running scalar carried by the consumer loop running j DOWNWARD — the empty product (=1 at j=t) anchors the recursion; O(C) multiplies per token instead of O(C) per (t,j) pair or a cumsum materialization pass.

The exact INVERSE of `cumsum-gate-to-decay-matrix` / `pairwise-distance-decay-matrix` (both materialize the matrix for a batched matmul — this dematerializes it when the consumer streams).

### 2.8 `split-decay-outputs-eliminate-readback` (D)

Source: deltanet_chunked_cubecl.rs L342-349 ("Split variant of chunk decay: writes per-token decay [n_head, C] and total decay [n_head] to SEPARATE buffers (no readback needed for production use). This avoids the [n_head, C+1] packed layout that the original `DeltanetChunkDecayCubeCL` produces, which requires a CPU readback + re-upload to extract the two components. The split variant writes directly to the layouts expected by `DeltanetDeltaSCubeCL` (reads [n_head, C]) and `DeltanetStateTransitionCubeCL` (reads [n_head]).") — and the pipeline test L1949-1969 documents the exact readback hack + the wrong-offset hazard it eliminates.

Mechanism: compact packing is a pessimization when it breaks consumer layouts — write each component directly into the buffer shape its consumer reads; the host round trip disappears entirely.

Distinct from `per-layer-strided-grad-destination` (consumer-selected stride within one buffer — this is consumer-selected buffer SPLIT) and from `defer-readback-until-after-submit`/`batched-downloads-single-sync-cycle` (both mitigate readback cost — this ELIMINATES it at the source).

### 2.9 `carry-dual-layout-dead-slot` (L)

Source: deltanet_chunked_cubecl.rs L77-85 ("carry_stride: the per-channel stride in the carry buffer. kernel_size - 1: the original chunked-carry layout. kernel_size: the conv_state layout (full ks-wide sliding window; the chunked kernel reads/writes positions 1..ks-1, leaving position 0 untouched — it holds a stale value that the next decode's shift overwrites before use)") + L98 (`carry_off = ch * carry_stride + carry_idx_offset`).

Mechanism: the prefill carry producer parameterized (stride + offset — two scalars) to write the decode consumer's native sliding-window layout directly; slot 0 left stale — safely, because the decode consumer's shift-left provably overwrites it before any read. No conversion pass at the prefill→decode boundary.

Distinct from `per-layer-strided-grad-destination` (runtime Option dispatch — this is one parameterized producer writing consumer-native layout with a provably-dead slot).

### 2.10 `nonlinearity-forces-raw-input-buffers` (D)

Source: deltanet_chunked_cubecl.rs L27-36 ("The key correctness constraint: token t's convolution needs the RAW (pre-SiLU) inputs of tokens t-1, t-2, t-3. If those were overwritten in-place (as the single-token kernel does), the convolution would read SiLU'd values (incorrect). So the chunked kernel uses SEPARATE read-only input + write-only output buffers.") + L43-47 (the carry holds RAW inputs).

Mechanism: the negative space of in-place aliasing — a sequential in-place producer trick is INCOMPATIBLE with batched/widened execution whenever a nonlinearity sits between producer and windowed consumer (later tokens read earlier tokens' POST-activation values — silently wrong, same dtype, no error). The "why you can't just widen the grid" rule.

Distinct from `inplace-output-alias` (when aliasing WORKS — this is when it cannot).

## 3. Candidates dropped as duplicates (substrate-first checks)

| Candidate | Closest existing rule | Verdict |
|---|---|---|
| ΔS rank-1 chain per-row lane-owned register accumulation (chunked L1012-1078: one cube per (head,row), lanes own 4 columns, loop C tokens, `v[j,row]` loaded once + register-broadcast) | `row-owned-register-blocked-recurrence` (2.1) + `weight-grad-batched-outer-product` | **Dropped** — the per-row lane-owned register tiling IS 2.1's mechanism applied to accumulation instead of RMW; the outer-product batching is the H rule. |
| Unique-writer carry partition (chunked L62-64: "each (token, channel) pair writes a unique carry slot ... so there is no race" — carry updated inside the main compute kernel, no barrier/atomics/second dispatch) | `chunkwise-parallel-recurrence` | **Dropped (enrichment)** — the state-carry concept + a write-ownership partition that is implicit in any grid partition (Batch 27's precedent: "the write-ownership partition is implicit in any grid partition; noted as an enrichment"). |
| Per-thread suffix product, no scan (chunked L262-265: "This is a reverse cumulative product per head. Implemented as a single elementwise dispatch — each thread computes one decay_to_end value by looping over the relevant α's. For C=64 this is 64 multiplies per thread, acceptable for the one-per-chunk-call cost.") | `precompute-scalar-reduction` + `tree-reduction-depth-over-sequential-accumulate` | **Dropped** — the recompute-vs-scan trade-off at small C; O(C²) total work but zero inter-thread dependencies. The corpus carries the trade-off shape twice; the brute-force-scan instance adds no mechanism. |
| Streamed intra-chunk attention, no [H,C,C] matrix (chunked L588-594: the doc's own two-step alternative materializes attn then matmuls; the shipped kernel streams j accumulating into output registers) | `online-softmax-streaming` + `kv-block-streaming` | **Dropped** — the flash-attn streaming principle transplanted to linear attention (no max/sum rescaling, just decay); F already carries the streaming shape. |
| Scatter fusing add + scale + transpose (chunked L1296/L1325: `out = (intra + cross) * scale` + head-major→token-major in one pass) | generic fusion family | **Dropped (near-DROP)** — the transpose-as-fused-epilogue cost argument (O(C·n_head·d) negligible vs O(C·d²)) is the only non-generic content. |
| GQA-style expand + L2-normalize + V-copy fused (deltanet L1192-1195: "fuses the expansion + L2-normalization + V copy into one dispatch, reading from the compact GEMV output") | `merge-dispatches` + `gqa-kv-head-sharing` | **Dropped** — fusion instance; the norm-on-compact-head detail is minor. |
| GPU-resident derived per-step scalars, no CPU sync (deltanet L537-539: `launch_with_gpu_handles` — "This variant avoids any CPU sync — beta/decay are already on GPU") | `offload-cpu-bottleneck-to-gpu-kernel` + `host-cache-immutable-device-weights` | **Dropped** — derived-activation residency is an instance of the two existing rules composed. |
| L2-normalize per-thread redundant head-norm recompute (deltanet L1107-1109: "Simple O(n_head × head_dim) per-thread approach ... Correctness first; shared-memory reduction optimization deferred.") | `precompute-scalar-reduction` | **Dropped** — the file itself labels this the deferred/naive form; flagged as perf debt, not a rule. |
| Gating-concat single-buffer two-GEMV input path (deltanet L756-758) | `fuse-gate-up-gemv-activation` + `combined-kv-buffer-keys-then-values` | **Dropped** — producer fusion + `[a | b]` concat layout, both carried. |
| z-gating in-place multiply (deltanet L1060) | `inplace-output-alias` | **Dropped.** |
| Persistent per-layer state handles (deltanet L1448-1450: "Pre-allocated once, reused across all decode steps") | `pool-buffer-aliasing-dedicated-output` / `host-cache-immutable-device-weights` | **Dropped** — device-residency applied to recurrent state. |
| Fixed workgroup 128 under Metal 1024 limit (deltanet L689-693) | `block-count-clamp` | **Dropped** — portability API usage. |
| smem tree cross-plane bridge (deltanet L108-161 — the Issue 610 fix itself) | `tree-reduction-depth-over-sequential-accumulate` + 2.2 | **Dropped** — the tree is the B rule; its raison d'être (plane scope < cube scope) is 2.2's inverse. |
| params-as-f32 runtime buffer (deltanet L559-562, Issue 604 T1) | `cuda-graph-scalar-via-device-pointer` | **Dropped** — API usage. |
| shape-gated fast path + fallback contract (deltanet L594-604 `supports()`) | `shape-kind-dispatch` / `hardcoded-fast-path-jit-specialization` | **Dropped** — but see Bug B: the LEGACY kernel lacks the guard its own fallback contract requires. |

## 4. Confirmed live bugs found during mining (filed as a riir-ai issue)

**A (major, production-wired): `deltanet_cross_chunk_f32` computes a staircase-diagonal reduction instead of a matvec.** The kernel's per-lane accumulators each cover a DIFFERENT column (lane l: `acc0` over rows {l, l+32, l+64, l+96} at column l), so `plane_sum` sums partials of DIFFERENT columns: `dot0 = Σ_r q[r]·S[r, r mod 32]` — a mod-32 diagonal staircase, identical for all 32 output positions of each block (output is block-constant with period 32), NOT `Σ_r q[r]·S[r, c]` per column. Contrast the intra_output kernel where every lane holds partials of the SAME scalar dot (correct usage). The existing G1 pipeline test cannot catch it: magnitudes are tiny (q ≈ 0.01, s_init ∈ ±0.005) and `REC_TOL = 1e-3`; the error lands ~20× under tolerance (verified by estimate: per-term q·S ≈ 3e-5, √128 summing → ~4e-5 after decay × 1/√128 scale vs 1e-3 tolerance). Production-wired: `ternary_deltanet_gpu_forward.rs::prefill_deltanet_recurrence_chunked` (L3850-3860) calls it behind `PREFILL_CHUNKED` + `supports()` — silently wrong chunked prefill whenever enabled. Also note the kernel only READS 512 of the 16384 S elements per (head,token) cube — a correct matvec must read all of S. Fix direction: drop `plane_sum`; each lane loops all d rows for its 4 owned columns (the `delta_s` structure — per-lane full ownership, zero cross-lane reduction; same total S traffic, each element read exactly once per cube).

**B (latent): the legacy `deltanet_recurrence_f32` is silently wrong for any `head_dim ≠ 128`, and it is the DOCUMENTED FALLBACK for the rowpar kernel's `supports() == false`.** L80 hardcodes `cube_size = 128` for `head_idx = ABSOLUTE_POS / cube_size` + a 128-slot smem tree, but the launcher dispatches `CubeDim::new_1d(head_dim)` with runtime `head_dim` (L570-571). For head_dim=64: two workgroups compute head 0 (race) + heads misassigned + the tree reads uninitialized `smem[64..127]`. For head_dim > 128: OOB smem writes. Both current models use head_dim=128 so it is latent — but the fallback contract ("callers must fall back to [`deltanet_recurrence_f32`] otherwise", rowpar doc L288-289) is unsafe: any head_dim≠128 model has NO working recurrence kernel. Fix: mirror the rowpar `supports()` guard on the legacy launcher (head_dim == 128) or derive cube_size from the actual CubeDim.

**C (edge): partial-chunk carry (`c < ks_m1`) diverges GPU-vs-CPU.** GPU `window_start = c.saturating_sub(ks_m1)` = 0 → new inputs at carry slots 0..c-1 unshifted; the CPU reference (L1451-1466) correctly merges `[old_carry[c..ks_m1], new[0..c]]`. Fires when total tokens mod C ∈ {1,2} at ks=4; silently corrupts the final partial chunk's carry into decode. No test covers `c < ks_m1`.

**D (edge): GPU L2-normalize lacks the CPU reference's zero-norm guard** — GPU computes `1/sqrt(0) = inf` then `0·inf = NaN` on an all-zero head; CPU guards `if sum_sq > 0.0`. Tests never exercise a zero head.

Minor: the single-token beta/decay launcher uses `CubeDim::new_1d(n_head)` — exceeds Metal's 1024-thread workgroup limit for n_head > 1024 (unreachable with current models; noted for completeness).

Filed as riir-ai `.issues/NNN` (see the issue for the failing-test recipes: amplify s_init/q magnitudes or tighten tolerance to ~1e-5 for Bug A; head_dim=64 smoke test for Bug B; a `c < ks_m1` case for Bug C; a zero-head case for Bug D).

## 5. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 correctness | 10 rules valid pseudocode (syn-parsed by the L2 pruner); 0 duplicates (16 substrate-first checks with explicit distinct-from/drop notes; 2-way subagent + synthesizer verification of every load-bearing quote) | ✅ PASS |
| G2 perf | corpus build O(n); router O(1); 284 kernel_heal-scoped lib tests in 0.11s | ✅ PASS |
| G3 no-regression | default 190 lib tests + kernel_heal 284 + clippy 0 warnings on `--features kernel_heal --all-targets` | ✅ PASS |
| G4 alloc-free | static `KernelRule` entries; zero hot-path allocation (unchanged mechanism) | ✅ PASS |
| G5 routed demo | 193 flat → 12.9 avg = **15.0× candidate reduction maintained** | ✅ PASS |

## 6. Status

- Corpus: 183 → **193** (B 32→33, C 18→19, D 23→26, E 9→11, G 17→19, L 5→6; 15 categories unchanged — no router changes needed beyond the count pin).
- riir-gpu remains OPEN: ~44 of ~200 files mined. Next-richest for Batch 29+: `ternary_deltanet_gpu_forward.rs` (234K — the production consumer of this cluster; host-orchestration of everything mined here), `deltanet_input_proj_fused.rs` (22K), `dllm/`, `epilogue/`, `edge_lora/` — smaller clusters.
- Bug A (cross-chunk staircase) is the second production-silent correctness bug found by mining (after Batch 27's Issue 671 backward_dflash finding) — the mining pipeline's bug-yield is now a standing deliverable.
- Centroid mining via `CommittedFieldBlend` — blocked on real kernel benchmarking data (unchanged). `kernel_perf_bench` numeric validator — blocked on GPU (unchanged).
