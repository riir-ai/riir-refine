# Research 031 — riir-gpu D2F Training Cluster (Batch 30): Zero-Init Frozen Teacher + Counter-Derived Seed + Empty-Workset Gate + Shared Pipeline Set + Uniform Pad Discipline + Fused Count

**Date:** 2026-08-14
**Source:** `riir-ai/crates/riir-gpu/src/dllm/mod.rs` (2040 lines — Plan 068: Discrete Diffusion Language Model GPU training infrastructure: the bidirectional dLLM trainer `GpuDllmTrainer`, the asymmetric distillation loop `GpuD2fDistill`, the direct block-causal trainer `GpuDirectBlockCausal`, and the `RePlaid` adaptive noise schedule from Plan 078) + `dllm/tests.rs` (108 lines) — the seventeenth batch mining the stack's own kernels, targeting the D2F training cluster the sibling is NOT working in.
**Corpus:** 203 → **209 rules** (C 21→22, D 29→31, E 12→13, M 6→7, N 11→12).

## 1. Coverage

Deep-read via two parallel mining subagents (lines 1-1027 + 1028-2040) + synthesizer verification of every load-bearing quote (all quotes below re-verified against the source by the synthesizer, including the `lora/mod.rs` L160 constructor contract that grounds the zero-init rule). Honest file count: **50 of ~200 mined** (~25%).

Identity observation vs expectation: the file was pre-assessed as "likely heavy Batch 26/27 overlap" (wgpu host-side orchestration) — and it is: the per-call bind-group creation, the naive per-call download staging, the CPU argmax over downloaded logits are all SLOW instances of already-covered rules (documented below as drops). The novel yield is the TRAINING-LOOP-shaped mechanisms with no corpus coverage: teacher/student mode collapse via adapter algebra, stateless per-iteration noise, whole-step elision on an empty workset, cross-subsystem resource sharing, and the uniform-struct layout discipline pinned by tests. The `RePlaid` section (the newest code, Plan 078) carried the two worst MATH bugs (unit-incoherent sign basis; tracker-state orphaning) but its patterns are host-side controller math — out of kernel_opt scope (candidates noted in §5).

## 2. The six new rules

### 2.1 `zero-init-side-branch-frozen-base-path` (M)

Source: dllm/mod.rs L1044 ("Teacher (frozen, bidirectional): full attention, no LoRA (base weights only)") + L1097-1098 ("── Teacher: separate forward pass with zero LoRA (base model only) ── / Standard LoRA init: A=random, B=zero → LoRA output = 0 (base weights only)") + L1100-1114 (`GpuLoraBuffers::new` + `GpuForwardPass::new` over the SAME Arc weights) + `lora/mod.rs` L160 ("A: [rank, in_dim] Kaiming init, B: [out_dim, rank] zero init." — the constructor contract).

Mechanism: to run a frozen un-adapted model (teacher) beside a trainable adapted model (student) over the same base weights, initialize the adapter's B to zero and run the SAME adapter-capable forward for both: `out = W·x + B·(A·x) = W·x` exactly. One code path, one uploaded weight set, both modes; the identity is mathematical, not maintained by discipline.

Distinct from `host-cache-immutable-device-weights` (caching — mode identity) and `enum-dispatch-separate-kernel-per-variant` (enumerating paths — this ELIMINATES the mode axis).

### 2.2 `counter-derived-seed-stateless-noise` (E)

Source: dllm/mod.rs L828-831 ("PCG32-style RNG (matches noise_corrupt.wgsl kernel). ... let mut rng_state = ((step as u64) << 32) | (epoch as u64);") + L839-842 (wrapping LCG + `>> 33` + const reciprocal) + L1526-1537 (direct trainer's verbatim twin) + L39 (`rng_seed: u32` — the same seed reaches the GPU launch as ONE uniform) + L583-596 (eval twin).

Mechanism: derive each iteration's RNG seed from loop counters packed into disjoint bit fields instead of threading one shared stream. Every iteration's noise becomes a pure function of its indices — standalone replay, skip-independence, and the same counter-derived shape parallelizes onto GPU threads (seed in the uniform, thread index mixed in-kernel). The `>> 33` high-bit extraction takes the statistically good half of the LCG.

Distinct from `derive-params-from-global-invocation-id` (per-THREAD identity from the grid — this is per-ITERATION seed from counters; the two compose).

### 2.3 `predicate-gated-skip-empty-workset-step` (D)

Source: dllm/mod.rs L851-853 ("if total_masked == 0 { continue; // Nothing masked — skip step }") + L1243-1245 (distill twin) + L1546-1548 (direct trainer: guard BEFORE the forward — the correct ordering) + the contrast at L485-487/L490 (a late guard ONLY inside `compute_loss`, leaving the rest of the step running and the 0/0 hazard one refactor away).

Mechanism: when a stochastic sampler can draw an empty workset, gate the ENTIRE dispatch batch (forward + loss + backward + optimizer) on a host-side "any work" check at the TOP of the step. One comparison elides the full GPU sequence AND guarantees the count-based loss normalizer never sees 0 — div-by-zero guarded by construction. Composes with 2.6: the fused count IS the predicate.

Distinct from `predicate-gated-backend` (chooses between kernels — this elides the batch) and `skip-all-zero-packed-words-via-or-reduce` (in-kernel skip — host-side batch elision).

### 2.4 `share-pipeline-set-across-subsystems` (D)

Source: dllm/mod.rs L1092 ("let pipelines = Arc::new(GpuPipelines::new(&self.ctx.device));") + L1134-1138 (`GpuBackwardPass::new(..., pipelines.clone())`) + L1148 (`AdamWOptimizer::new(..., pipelines.clone())`) + L1278 ("masked_loss.compute_loss(&pipelines, total_masked)") + the direct trainer mirror at L1433/L1459-1463/L1475.

Mechanism: wgpu pipeline creation validates WGSL + builds PSOs — expensive and identical across subsystems sharing shader modules. Build ONE set at orchestrator setup, Arc-share into every constructor + dispatch site: compile paid once per run, per-step cost a ref-count bump. The cross-subsystem analog of `prebuilt-bind-groups` (which pre-builds per dispatch site).

Distinct from `prebuilt-bind-groups-fixed-buffer-combos` (bind groups per site — the pipeline set itself shared across constructors) and `hot-swap-weights-preserve-autotune-caches` (cache preservation — single construction).

### 2.5 `pad-uniform-pod-to-min-binding-size` (N)

Source: dllm/mod.rs L33-60 (three param structs, explicit `_pad`s) + L74-82 ("create_uniform_buffer ... size: 16, // All param structs are 4 × u32") + dllm/tests.rs L30-49 (three layout tests: `size_of::<...>() == 16` and `% 16 == 0`).

Mechanism: pad every uniform param struct to an exact 4×u32 = 16 bytes so ONE fixed-size buffer helper serves all kernels; bind-group creation can never fail on an incidental 12-byte struct. Pin with `size_of` tests — adding a field without re-padding breaks CI instead of failing on the GPU at runtime. Zero runtime cost; the discipline is the value.

Distinct from `staging-packed-uniforms-encoder-ordered-copies` (uniform DISTRIBUTION — per-struct layout discipline) and `vectorized-pack-load` (load alignment — binding size class).

### 2.6 `fuse-count-into-mutation-pass` (C)

Source: dllm/mod.rs L828-830 ("Track total_masked during corruption to avoid a separate O(seq_len) filter scan afterwards.") + L843-847 (the masked-count increment inside the corruption branch) + L585-596 (eval twin). The count feeds BOTH the loss normalization and the empty-workset skip gate.

Mechanism: when a mutation pass writes a predicate-derived output and its consumer needs the count, accumulate INSIDE the loop — the increments ride already-hot cache lines and the second O(n) scan (or second reduction kernel) disappears. In kernel form the predicate write + reduction share one launch.

Distinct from `dual-grad-same-pass` (two gradient outputs — count fused into its producing mutation). Cross-domain adjacency noted: the host-loop shape overlaps rust_perf `loop-fusion`; the kernel_opt angle is the count's downstream role as dispatch/normalization gate — retained on that basis.

## 3. Dropped as duplicates (13 candidates)

- **GPU→GPU copy avoiding host roundtrip** (L874-880, L1254-1273; source comment itself cites "Issue 018") → triply covered by `pool-buffer-aliasing-dedicated-output`, `merge-dispatches-into-single-compute-pass`, `same-queue-implicit-ordering-no-sync`.
- **Per-call `create_bind_group`** (L309-320, L441-454) → `prebuilt-bind-groups-fixed-buffer-combos` (Batch 26). dllm is a new SLOW source instance.
- **Two sequential `download_u32` with fresh staging + submit each** (L343-344) → `batched-downloads-single-sync-cycle` (Batch 26); `buffer.rs` already ships `download_f32_reuse`/`download_f32_batched_reuse_into` that the local helper ignores.
- **CPU argmax over fully-downloaded logits** (L610-631, L1186-1207) → `device-side-argmax-minimal-download` (Batch 25). Two new SLOW instances.
- **Host scratch reuse across training steps** (`Vec::with_capacity` + `clear()`/`fill()`, L766-826, L1163-1167) → cross-domain duplicate of rust_perf `reuse-allocation-clear` (one of its 6 context-annotated bench rules). Not re-landed in kernel_opt.
- **Multiply-by-precomputed-reciprocal** (L778-779, L584) → thin; the named-const form is folded into rule 2.2's pseudocode where it is load-bearing (CPU/GPU bit-identity).
- **hoist-loop-invariant-block-count / bind-row-once** → below quality bar (CPU micro-opts on once-per-sample or downloaded data).

## 4. RePlaid candidates rejected as out-of-scope (host-side controller math)

Three sound-looking mechanisms from the Plan 078 section were rejected for kernel_opt because they are pure host-side ML-controller patterns, not GPU/kernel/host-orchestration mechanisms (recorded here for whoever builds a training-method corpus — they belong nearer `riir-train`):

- **O(1) EMA mean/variance tracker per schedule slot** (`VarianceMinimizer`, L1735-1768) — single-pass running stats replacing loss-history storage.
- **Variance-floor-gated self-annealing step** (`adapt()`, L1775-1789) — sign-only direction × `sqrt(running_var)` magnitude, floor + clamp; self-anneals as variance collapses. (Source impl has the sign-basis bug below.)
- **Sort-project-onto-monotone-cone** (`adapt_ratios()`, L1854-1866) — independent per-slot adaptation followed by a sort that restores the schedule family's ordering invariant. (Source impl orphans the tracker states — bug below.)

## 5. Bug yield (filed as a riir-ai issue)

1. **`unwrap_or_default()` swallows GPU download errors** (L611, L477-483): in `compute_loss`, an `Err` yields an empty `per_sample` → loss silently `0.0` → `best_loss = best_loss.min(0.0)` pins best to 0.0 forever, so the checkpoint-best gate can never fire again; in eval, the empty logits Vec panics at the slice. Both should propagate `Result`.
2. **RePlaid `adapt()` sign basis is unit-incoherent** (L1780-1786): `running_mean - param` compares an EMA of LOSSES against a mask RATIO; with losses ~2.0 every block reads "hard" and all ratios grind to `min_param`. The spec calls for deviation from the cross-step target κ.
3. **`adapt_ratios()` sort orphans the tracker state** (L1858-1867): `current_ratios` sorted, `step_trackers` not permuted — block↔tracker mapping permanently scrambled after the first call; the next `adapt_ratios()` overwrites the sorted vector with unsorted tracker outputs.
4. **RePlaid is unwired dead code re-exported as public API** (L1698-1699 `#![allow(dead_code)]` + L1896-1897 re-export): both trainers still use plain `NoiseSchedule::monotonic_ratios`; `test_gpu_adaptive_d2f_training_wiring` claims wiring validation but only constructs the schedule standalone. `test_adaptive_schedule_reduces_variance` asserts no variance reduction (bounds + adaptation count only).
5. **Zero-masked skip fires AFTER the expensive teacher work in the distill loop** (guard L1243-1245 after teacher forward + full logits download + CPU argmax at L1186-1207); the direct trainer has the correct order (L1546-1548 before forward at L1551).
6. **`DistillMode::D2fWeighted` is dead config** (L1033-1039 vs the always-hard-argmax loop L1189-1207 + TODO); selecting it silently runs Standard.
7. **Stale doc vs implementation** (struct doc L653 "GPU noise corruption" vs method doc L686 + code L828-849 CPU); `GpuNoiseCorrupt` is exported + benched but unused by the training loop.
8. **`DllmTrainingConfig::default()` bakes the build machine's absolute path** into `checkpoint_dir` (L197-202, `env!("CARGO_MANIFEST_DIR")`); `create_dir_all(...).ok()` (L764) swallows the failure.
9. **Eval conflates failure/skip with 0% accuracy** (L612 `Err(_) => continue`; L640-641 returns 0.0 when nothing masked).
10. **Two clip mechanisms, one silently disabled** (L1146 `grad_clip: 0.0` hardcoded vs external manual clip L1290-1292).
11. **~300-line loop skeletons + 35-line corruption blocks duplicated verbatim between the two trainers** (L1226-1241 vs L1529-1544 etc.) — drift risk already realized (bug 5's ordering differs between the copies).
12. **`observe()` variance EMA uses the post-update mean** (L1764-1768) — biases variance low, can freeze adaptation prematurely via the floor.

## 6. GOAT gate

| Gate | Verdict | Evidence |
|---|---|---|
| G1 correctness | **PASS** | `batch30_pseudocode_parses` standing syn-parse gate (6/6 ref_shape + opt_pattern parse via the L2 pruner); quotes verified against source by the synthesizer; 0 duplicates (2-way subagent + corpus greps) |
| G2 perf | **PASS** | 290 kernel_opt lib tests in 0.18s; router lookup O(1) unchanged |
| G3 no-regression | **PASS** | default 190 lib tests; clippy 0 warnings (`--features kernel_opt --all-targets`); full 5-layer `ci_feature_guard.sh` PASS (405 tests final layer) |
| G4 alloc-free | **PASS** | static corpus entries; no hot-path change (mechanism unchanged since Batch 1) |
| G5 candidate reduction | **PASS** | 209 flat → 13.9 avg per routed expert = **15.0× reduction maintained** (15 categories; was 15.0× at 203) |

## 7. Next

- Batch 31 candidates: `edge_lora/` (13 files, ~9K lines — assessed low kernel density, host-heavy), `ternary_deltanet_gpu_forward.rs` (234K — BLOCKED on sibling WIP landing), remaining `forward/` + `attention` stragglers, `kernels/` (the WGSL dispatch helpers), `dataloader.rs`, `optimizer.rs`, `lora/` (host-side but shares the dllm subsystem seams).
- The bug yield (§5) filed as a riir-ai issue with fix recipes — the `unwrap_or_default` loss-poisoning (bug 1) is the highest-value fix.
