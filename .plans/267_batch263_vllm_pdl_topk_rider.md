# Plan 267: Batch 263 — the vLLM #59737 PDL-for-MoE-topk second-instance rider

**Status:** In progress — claimed 2026-10-20T01:2xZ (zed-m3-idle/M3), expires 2026-10-22.

## Source (verified this session)

vllm-project/vllm **PR #59737** "[Perf] Enable PDL for fused MoE top-k gating" — MERGED
2026-10-19T19:17:28Z, merge commit `3c9beb82d1a92eb5a84cca4408a0a1b479d4dd03`, head
`8470dd48e75ca8ec392d689bbb49fa8dc76fbeef`, Apache-2.0. +24/−0, one file:
`csrc/moe/topk_softmax_kernels.cu`. Staged by the B262 watch pass ("vllm +40 since
08-28 — staged: #59737 PDL-for-MoE-topk = B257 second instance"); the PR body and the
full diff were fetched and read verbatim before this plan was written.

## The face

RIDER on `pdl-admission-by-image-set-marker` (gpu_111, Batch 257 host, TensorFold) —
the **CONSUMER-WAIT-DISCIPLINE face**, the same admission axis at SOURCE level:

- Kernel side: `cudaGridDependencySynchronize();` + `cudaTriggerProgrammaticLaunchCompletion();`
  placed at the kernel top under `#if defined(__CUDA_ARCH__) && __CUDA_ARCH__ >= 900`,
  BEFORE the row-dependent early exit — their comment: "PDL permits early launch, but
  logits/metadata still depend on the producer."; PR body law: "The dependency wait and
  completion signal precede dependent reads and row-dependent exits."
- Host side: the PDL attribute (`cudaLaunchAttributeProgrammaticStreamSerialization`,
  `programmaticStreamSerializationAllowed = 1`) set via `cudaLaunchKernelEx` WITHOUT a
  host architecture query; launch error checked loud (`STD_TORCH_CHECK(err == cudaSuccess,
  "PDL topkGating launch failed: ", ...)`); ROCm keeps the plain triple-chevron launch.
- Admission per-kernel-family: "matching `topk_softplus_sqrt` and `per_token_group_quant`".
- Honest cost face: router→gating chain −9.12% hot weights (0.486 µs, GB200 M=4
  MiniMax-M3), TPOT −0.61..−1.16% at C1/16/32, pure-prefill TTFT **regresses** 0.54%/0.49%
  — source-reported.

Kinship (do not double-count): gpu_23 owns in-kernel wait placement (confirmed + the
exits ordering added); gpu_18's rider owns the host-side per-launch capability probe
(this instance carries NO host query — the device-guard posture instead); THIS host owns
whether the launch carries the attribute at all — the axis the rider instantiates.

## Tasks

- [x] Source verified: PR #59737 fetched (API + .diff), mechanism read, quotes extracted
- [ ] Claim row in `.distill/001_mining_queue_snapshot.md` committed + pushed BEFORE authoring
- [ ] Dedup grep: confirm no other rule owns the vLLM PDL face (B257 host is the home)
- [ ] Rider text appended to the `pdl-admission-by-image-set-marker` gpu_concept (entries_gpu_111.rs) — count-neutral, surgical single-target edit (the B262 splice-corruption lesson: read the diff)
- [ ] Needle gate: `batch263_rider_landed` in corpus/tests.rs (verbatim needles + the 923 count witness)
- [ ] Gates: kernel_opt lib tests, default lib tests, clippy −D (default + no-default postures)
- [ ] Retrieval floors: count-neutral precedent (B246/B260/B262) = HELD no re-pin; re-measure only on a warning sign
- [ ] Verify NOT-owed claims: freeze tables (kernel_opt is not the clippy corpus), binary rebuild (kernel_opt outside the release set)
- [ ] Research note 299 + AGENTS.md kernel_opt bullet line + `.docs/11_domains/kernel_opt.md` entry
- [ ] Commit `feat: batch 263: ...` + push; snapshot landing row in the same commit

## Records

Plan 267 / Research 299. Corpus stands 923 (count-neutral rider, the B246/B260/B262
precedent). No GPU owed; no 4090 lane touched (the bout window stays theirs).
