# Research 299 — Batch 263: the vLLM #59737 PDL-for-MoE-topk second-instance rider

**Status:** RECORD — landed as Batch 263 (Plan 267), corpus stands 923, count-neutral.

## Source

vllm-project/vllm **PR #59737** — "[Perf] Enable PDL for fused MoE top-k gating"
(wolfcomos, merged 2026-10-19T19:17:28Z by vadiklyutiy, merge
`3c9beb82d1a92eb5a84cca4408a0a1b479d4dd03`, head `8470dd48e75ca8ec392d689bbb49fa8dc76fbeef`,
Apache-2.0). +24/−0 in ONE file: `csrc/moe/topk_softmax_kernels.cu`. Staged by the
B262 watch pass ("vllm +40 since 08-28 — staged: #59737 PDL-for-MoE-topk = B257
second instance"); the staged verdict was verified BEFORE the claim: PR body + full
.diff fetched and read this session.

## Why a rider on `pdl-admission-by-image-set-marker` (gpu_111, Batch 257)

The PR instantiates the SAME admission axis TensorFold's rule owns — whether a
launch carries the programmatic-dependent-launch attribute — at SOURCE level
instead of from prebuilt-image markers:

| Axis | TensorFold (the host rule) | vLLM #59737 (this face) |
|---|---|---|
| Admission source | image-set markers read at LOAD (`Module.holds`, both images) | the kernel's own wait discipline, visible in source |
| Set precondition | every participating image carries `drama_pdl`; ONE stale image → whole chain plain | per-kernel-family: "matching `topk_softplus_sqrt` and `per_token_group_quant`" — the already-admitted PDL consumers |
| Failure posture | fail-CLOSED at load: probe false / mixed set → plain launch (perf worst case) | fail-LOUD at launch: `STD_TORCH_CHECK(err == cudaSuccess, "PDL topkGating launch failed: ", ...)` |
| Capability axis | gpu_18's rider: host-side `sm >= 90` probe per launch | NO host query: attribute set unconditionally, device instructions `#if defined(__CUDA_ARCH__) && __CUDA_ARCH__ >= 900` |
| Platform posture | standalone kernel directories launch plain | `USE_ROCM` keeps the plain triple-chevron launch |

## The mechanism (verbatim, the diff at the merge)

Kernel side — the wait+trigger pair at the kernel TOP, ABOVE the row-dependent
early exit:

```cuda
#if defined(__CUDA_ARCH__) && __CUDA_ARCH__ >= 900
    // PDL permits early launch, but logits/metadata still depend on the producer.
    cudaGridDependencySynchronize();
    cudaTriggerProgrammaticLaunchCompletion();
#endif

    // Threads with indices out of bounds should early exit here.
    if (thread_row >= num_rows)
```

PR body law: "The dependency wait and completion signal precede dependent reads
and row-dependent exits." — a block exiting before the pair would strand the
chain the attribute admits. Placement-law ownership stays
`grid-dependency-wait-scoped-to-dependent-operands` (gpu_23); this face records
the admission's demonstrated precondition, never a substitute.

Host side — the attribute without an architecture query:

```cuda
cudaLaunchAttribute attr{};
attr.id = cudaLaunchAttributeProgrammaticStreamSerialization;
attr.val.programmaticStreamSerializationAllowed = 1;
config.attrs = &attr;
config.numAttrs = 1;
auto err = cudaLaunchKernelEx(&config, topkGating<...>, ...);
STD_TORCH_CHECK(err == cudaSuccess, "PDL topkGating launch failed: ", cudaGetErrorString(err));
```

## Measured (source-reported, GB200, MiniMax-M3 NVFP4 TP8, 8x GPU two-node)

- Router-to-gating CUDA-graph chain (M=4): 5.330 → 4.844 µs hot weights
  (**−9.12%**), −0.466 µs (−7.77%) rotating weights.
- Serving TPOT −0.61..−1.16% at request concurrency 1/16/32 (ISL/OSL 128/1024 and
  8192/1024); **the honest cost face: pure-prefill C16/C32 mean TTFT REGRESSES
  0.54%/0.49%** — the overlap pays on short decode-side chains, costs pure prefill
  (the gpu_57 measured-negative axis, here scoped to a workload class, not a stack).
- Accuracy: full GSM8K 1319 q ×2 runs, mean Δ −0.152 pp with baseline repeats
  differing 0.531 pp (the PR's own honesty: "these two repeats per variant do not
  establish accuracy equivalence").
- Correctness at the merge: 866 upstream fused-topk + FP32-router tests unchanged;
  360 original-vs-patched operator cases at ZERO tolerance (FP32/FP16/BF16, experts
  6–512, ties, NaN payload bits excluded); 1200 preallocated-chain + 960
  runtime-dispatch graph checks incl. poisoned intermediates and the M=32/33
  dispatch boundary; Compute Sanitizer memcheck clean.
- Overlap proof: untimed traces — PDL gating on all eight TP ranks; adjacent
  router/gating overlap-in-residence 6720/6720 patched vs 0/6720 baseline.

## Dedup

`59737|topkGating|topk_softmax_kernels|ProgrammaticStreamSerialization|GridDependencySynchronize`
over both corpora at claim: zero other owners — the only PDL hosts are gpu_111
(this rule), gpu_23 (wait placement, 2 Yukon riders), gpu_18's capability rider,
gpu_57 (the measured-negative ledger). The rider is on the staged host per the
B262 watch verdict.

## Gates (all green)

batch263 needle gate FIRST-RUN PASS (17 verbatim needles + the 923 count witness);
kernel_opt lib **3192/0** (3191 floor + the new gate); default lib **2715/0**;
clippy `-D warnings` clean ×3 postures (default / kernel_opt / no-default, all
`--all-targets`); retrieval floors **21/21 measured HELD** (487 s, no re-pin);
freeze/score-bench NOT owed (zero clippy-corpus delta, the P30/B250 precedent);
maintained-binary rebuild NOT owed (kernel_opt outside the release set).

En-route: 3 PRE-EXISTING `doc_markdown` findings at HEAD in `entries_gpu_113/114.rs`
(B261's module headers — "PrismML"/"PQ2_0" unbackticked, flagged by this box's
clippy) blocked the `-D` gate; healed via the maintained binary
(`cargo refine --fix --write --verify`, 3 edits compile-gated verified) in the same
landing, disclosed as pre-existing residue, not this batch's.

## Records

Plan 267 / this note. Landing row: `.distill/001_mining_queue_snapshot.md`.
NOTBENCHABLE — all figures source-reported on GB200 (no CUDA validator in this
corpus); the decision content is the admission-law face.
