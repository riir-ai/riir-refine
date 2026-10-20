# Plan 271 — Batch 267: the cubecl autotune/barrier/fold wave + the vllm MLA-prefill prep fusion

**Status:** IN_PROGRESS — claimed 2026-10-21 ~04:5x +07 (shikuwa/4090, riir-refine idle loop, standing-watch re-poll 21:2xZ 10-20)
**Claim:** shikuwa/4090 idle lane, expires 2026-10-23 ~04:5x +07 (48 h); push is the arbiter; other lanes re-fetch before staging and yield to this row.

## Source window (the post-B265 poll delta)

The standing-watch re-poll (blobless clones, `.raw/bwatch1021/`, removed at close) enumerated
everything past the B265 pins. prism branch + tag top (`prism-b10787-73bfe6c`) STATIC — the 971
tripwire NOT fired; lithos/TensorFold static; uzu consumed (B264); wgpu NULL (5 fix/infra commits);
mlxfast not re-polled (weekly floor ~10-24, B265 read it unchanged 13 h prior).

- **cubecl `ec701b5f..81f254ee` (11 commits)** — the batch's cohesive core:
  - `1ad5040f` #1816 **drop IEEE-invalid float constant folds + keep FMA formation real** —
    OWN-RULE candidate: the partial folds are unsound (`0.0+x≠x` via `+0.0+−0.0=+0.0`; `x−x≠0` via
    NaN/inf; `0*x≠0` via −0.0/NaN; `0/x≠0` at x=0; `0%x≠0`; `x%1.0` partial) — only the
    both-constants case folds. Second face: FMA formation must test the scalar type — complex
    has no fma overload, the rewrite emits a call that does not compile.
  - `c8a4728c` #1848 **share autotune results between devices of the same hardware** — rider
    candidate on gpu_87 `autotune-fidelity-synthetic-carries-producer-params` (the P61
    hardware-path face's host): the hardware identity in the namespace key is what MAKES the
    same-hardware share sound; device-instance keys re-tune per device.
  - `1b4099b2` #1876 **never fallback on rejected candidate** — rider candidate on gpu_34
    `tuned-config-resource-limit-fallback`: a candidate whose own function REJECTED the problem
    (returned an error, queued no work) is kernel-unusability evidence — "deciding a rejection
    would run it for real and fail the same way" — excluded from the executed-unmeasured fallback
    like a launch failure or manual skip.
  - `aad86f45` #1871 **a plane barrier orders the workgroup's memory** — rider candidate on the
    barrier-family host (gpu_85 `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder`,
    the B262/B264 faces' host — fit adjudicated at authoring): Vulkan has no subgroup storage
    class, `SUBGROUP_MEMORY` semantics hit VUID-StandaloneSpirv-MemorySemantics-10870; a plane
    shares memory through the workgroup's, as the cube does — `WORKGROUP_MEMORY`.
  - `3a7c0e06` #1879 AutotuneKey shorthand — pre-graded NULL (clippy cosmetic, macro init form).
- **vllm `e10ecb6b..9d4192aa` (65, ~63 new past B265's `e4a676cd`/`bc01c0692`)**:
  - `09fd02721` #59634 **fuse the GLA gate GEMM and the MLA FP8 prefill q/k/v prep** (Kimi-K3,
    ROCm/CDNA4) — rider candidate on B259-C `precompute-chain-fused-intermediates-never-materialized`
    (entries_gpu_112): "The k_pe concat, the head padding and the FP8 cast in one launch" — the
    prep chain's intermediates never born; admission = cached capability probe (`lru_cache(1)`,
    CDNA4 + import + `not any(d & (d-1) for d in head_dims)`) + padded-heads strided view.
  - `9da5f2d31` #60250 rms_norm_gated custom op default-on (Qwen3.8-27B, ROCm) — LEAGUE lane
    intel (watch row, not corpus).
  - `7b569ec89` #62029 trtllm-gen for the FlashInfer DCP prefill context pass — watch row.
  - `fce64bc5c` #35911 NCCL weight metadata sent once and reused — T-CHOICE at authoring: small
    own rust_perf rule (metadata-once amortization) or watch; ⚠ IF a rust_perf rule lands:
    gen2+gen3 freeze regen + binary rebuild become OWED (release-set surface).
  - `34ac4f96a` #62282 Mooncake KV write-after-free peer-wait — watch row or thin concurrency
    rider (grade at authoring).
  - Rust Frontend series (~15 commits) + CI/docs/ROCm-quant — out of lane.
- **bevy `ddd047d56..e144654d` (30)**: `59a4d89f9` #25869 **reduce unnecessary GPU buffer
  allocations in the clustering pipeline** — rider candidate on the buffer-alloc family
  (home adjudicated at authoring); rest NULL (UI/reflect/docs/CI).
- **llama.cpp upstream `3bb63cbc..84e66d03` (31, ~10 past B265's poll)**: `39da130fc` #30873
  ssm_conv kernel size 7 (Mamba widening — watch/thin), `7e3eeee04` #30361 H2O-Lightning-4B
  **decision model** (llama.cpp hosts another decision model after Granite Switch #30676 —
  reflex/rethink serving intel), `839a572a4` #30823 qwen output reshape 2d (model lane), rest
  NULL/server/mtmd/hexagon.
- **MLX `f41bec7..bff10220` (1)**: `bff10220` #4682 staged affine_qmm_t_nax split-k LANDED
  (the B265 watch row's landing — M3 intel, recorded).
- **splash `be3b083c..1e532541` (205 non-merge!)** — the release/01-memory→11-cache-quota wave:
  STAGED AS BATCH 268's pool (split-K sixteen ways `6198321`, constrained+unconstrained decode
  lanes one command `f52d1ad`, QSA q/k projection one dispatch `4944799`, Flash-Next decode table
  `55b0071`, PLE row cache budget `18f9c69`, repeated-failure cache invalidation `6e21cc9`,
  eviction-count-gated re-encode `2712e67`, the Spill series). Needs its own session.

## Tasks

- [ ] T0 — dedup re-verified in-tree (autotune/barrier/fold/gate-fusion families grepped; the
      homes named above; #1879 pre-dropped NULL).
- [ ] T1 — plan-first landed + claim pushed (this commit).
- [ ] T2 — Slice A (subagent): cubecl wave — R1 the IEEE partial-fold own rule (+ the FMA
      complex-admission face adjudicated in-rule or as second face), rider #1848 → gpu_87,
      rider #1876 → gpu_34, rider #1871 → the barrier host. Every quote re-verified at the pin
      clone (`.raw/bwatch1021/cubecl`).
- [ ] T3 — Slice B (subagent): vllm #59634 rider → entries_gpu_112's B259-C host; #35911
      T-choice (own rust_perf rule vs watch, drop-honestly allowed); #62282 grade (watch vs
      thin rider); bevy #25869 rider (family read, drop allowed); llama.cpp #30873 grade.
- [ ] T4 — coordinator wiring: mod.rs KERNEL_CATEGORIES slices (if new file), count pins
      (tests.rs + router totals if counts move), needle gates batch267.
- [ ] T5 — gates: kernel_opt lib, default lib, clippy −D ×3 postures, retrieval floors
      re-measure (OWED — own rule lands); freeze/score-bench NOT owed UNLESS a rust_perf rule
      lands (then gen2+gen3 regen + binary rebuild + cp owed, the §0 law).
- [ ] T6 — Research 303 note + AGENTS.md corpus counts (if moved) + `.docs/11_domains/` +
      watch rows (MLX #4682 landed, H2O decision model, rms_norm_gated default-on, splash
      staged-B268) in the SAME landing commit.
- [ ] T7 — snapshot landing row + commit `feat: batch 267: ...` + push; `.raw/bwatch1021/`
      removed at close.
