# Research 357 — Batch 320: the post-B319 staged remainder

**Status:** RECORD (landed with Batch 320 / Plan 320; the corpus landing commit is the provenance).

## Source set

| Source | Pin | License | Content |
|---|---|---|---|
| ggml-org/llama.cpp | `fb59972` (#31073) | MIT | the H2D-immutable placement quadrant (own rule A) |
| tracel-ai/cubecl | `07cd07a` (the #1975 squash) | Apache-2.0 OR MIT | #1976 (own rule N) + #1977 (own rule B) + #1986 (own rule D) |
| vllm-project/vllm | `8b10bda` (#64494) + `2bfb8f1` (#64467) | Apache-2.0 | 2 count-neutral rider faces (gpu_111, gpu_133) |

Clone: `.raw/b320/{vllm,llamacpp,cubecl}` (blobless shallow; removed at close).

## THE STRUCTURAL FINDING (the batch's method catch)

`c1d012517` is a CI-only commit (`ci.yml` +2/−2, "run CI only on PRs to main #1993") whose MESSAGE carries the #1974-series narrative. The actual diffs of the #1976/#1977/#1986 constituents live in the PARENT `07cd07a` ("feat: native Metal on par with wgpu-msl; tile shared-memory reuse (#1975)", 66 files). The 10-30e coverage-repair staging quoted from the squash BODY (commit-message-anchored, code-diff deferred to the batch lane per its own disclosure) — every quote re-verified HERE at the commit whose diff actually contains it. #1967 (`read_lazy`) is a separate commit `3820a33` (5 files), in neither.

## The four own rules

1. **`immutable-operand-device-resident-per-buffer-type`** (A) — llama.cpp fb59972: KV-cache rotation constants were host tensors fed as graph inputs; every ubatch re-uploaded them H2D (the parent TODO named the disease verbatim). The fix: device buffers of the consumer's own buffer type, weights usage class, init-once, per-layer. **B319's "host-side" adjudication REFUTED at T0** — the landing row's dedup-at-T0 caught it; the quadrant matrix (transfer-direction × lifecycle) is now complete: D2H-immutable (gpu_02), H2D-mutable (gpu_04), H2D-immutable (this). Provenance caveats carried: Assisted-by Codex; the PR added then dropped its own rotation-buffer tests (the landed tree carries no device-side residency test).
2. **`word-native-byte-pair-minifloat-conversion`** (N) — cubecl 07cd07a #1976: a word of byte-wide minifloat codes converts through `cvt.rn.f16x2.<pair>`'s own byte select reading out of the word's ONE register, one 128-bit result move the assembler folds away; the anti-shape is the compiler's PRMT/LOP3/SHF byte-isolation (five or six instructions a word). Admission capability-gated (packed minifloat ∧ byte-wide ∧ whole word ∧ `byte_pair_conversion`, armed arch_major 10/11/12). **Staged-text correction carried**: PRMT/LOP3/SHF is the OLD behavior the fix replaces, not the fix; "F2FP ... R.B1" is commit-message-only (the diff spells `cvt.rn.f16x2`).
3. **`block-scaled-operand-declared-form-refusal-ladder`** (B) — cubecl 07cd07a #1977: the block-scaled MMA operand contract is DECLARED data (`BlockScaledOperands {lhs, rhs, scales, block}`; `Packing::Words { field }` names the scale field inside scale words) and every mismatch refuses loud per axis — "instead of assuming e2m1 under e4m3 scales of sixteen". The hard-coded `E2M1_SCALE_BLOCK` deleted → `form.block` + `SCALE_READ = 16`. Six refusal strings verified verbatim; the run-depth law (B310, gpu_141) is the compose-side sibling — "the instruction defines the contract" family, two axes.
4. **`plane-storage-barrier-scope-covers-global-exchange`** (D) — cubecl 07cd07a #1986: units of a plane that write GLOBAL memory and read each other's writes need a barrier whose memory scope covers storage; the shared-only plane barrier does not promise it; `sync_storage` covers but is sufficient-and-stronger (cube extent). New `SyncPlaneStorageOp`, five backend lowerings verified in-diff (Metal `mem_device|mem_threadgroup`; SPIR-V subgroup barrier over UNIFORM|WORKGROUP with availability/visibility; WGSL doubled; CUDA `__syncwarp`; HIP/LLVM plane barrier). **SINGLE-SOURCE DISCLOSED** (one in-tree consumer: the plane-held quantize transport) — landed on the `table-reached-buffers-declare-residency` single-source correctness precedent; the second-instance trigger rides the watch.

## The two rider faces (count-neutral)

- **gpu_111 `pdl-admission-by-image-set-marker` += the SIZE-GATED ADMISSION face** — vllm 8b10bda #64494: `use_pdl = m*num_packed_cols <= multiProcessorCount*4096`, "PDL launch pays off at small M; at large M the early-launched blocks slow the chain down". PDL-sensitivity's third instance (gpu_57 measured-negative → B263's workload rider → this size gate); the mechanism completes the family's causal story.
- **gpu_133 `moe-tokens-per-expert-block-m-cap` += the PREPARE-KERNEL INSTANCE face** — vllm 2bfb8f1 #64467: the prepare pass's row tile ladders 1/8/32 at 64/1024 total tokens (GB200, "1.3-1.6x over 8 rows" — in-comment author claim, no bench artifacts; Co-authored-by Claude Opus 5.5, provenance weight carried). Different kernel + metric from the host (prepare quant/pad × total tokens vs routed GEMM × M·topk/E); the rung-ladder shape is the tie. The 1025 just-past-threshold test point joins the bitwise-exactness parametrize.

## Grades that moved the watch (no corpus)

- **#64553 Muse routing = the I6 watch row CONFIRMED** (`36ed432` — the 10-30b sweep's grade stands; the guard is Blackwell-window family-specific ≥1000 <1100, narrower than the row's "≥ sm_100"). New watch observations: (a) the mid-kernel PDL trigger before the kernel's own stores (placement to verify against gpu_23/B23's trigger-visibility law — flagged, not adjudicated a defect); (b) the Triton-parity asm transcription ("Triton lowers a/b on fp32 to div.full.f32, not the div.rn.f32 nvcc emits") is unowned — single instance, watched beside the redux row.
- **#1967 `read_lazy` = thin/watch CONFIRMED** (separate commit `3820a33`): the binding-vs-ownership atom is real and crisp ("bindings do not count toward Handle::can_mut") — a viable thin-D candidate if the lane wants it pinned; zero corpus owners today.
- **llama.cpp ab3768c #31368 = host-side CONFIRMED** (B319's verdict stands): the rollback depth-guarantee bound is host bookkeeping; kin gpu_105/gpu_10/gpu_110 distinct axes.
- **vllm window drops recorded**: #64630 (pinned CPU backup — host memory lifecycle, rust_perf at best), #63005 (TypeAdapters-once — atom owned by B38/B265), #62613 (recurrent-state self-describing events — connector wire), #64495 (cache-hit preservation under eviction — host LRU; cite glm-selected-prefix-move-before-evict distinct).

## Gates (all green, Plan 320)

batch320 needles+parse 2/2; kernel_opt lib 3327/0; default lib 2792/0; rust_perf_bench lib 2823/0; clippy −D ×4 postures 0 (6 doc_markdown mechanicals in the new module header healed by hand — the B192 class); kernel_opt+latent_retrieval lib 3520/0 (4076 s under the i1117 128k leg, isolated CARGO_TARGET_DIR=/tmp/b320_target); retrieval floors HELD (kernel self-variants 100.0% 30/30, blended 90.1/93.6/95.3, router annotation 102/171 59.6%, gate green — no re-pin); Layer 5b2 default-posture retrieval 27/0. freeze/score-bench NOT owed (P30, kernel_opt-only); binary rebuild NOT owed (kernel_opt outside the release set). Count pins 1033→1037 via ONE shape-asserted python pass (37 value sites + 37 message lines in tests.rs, 4 category rows + sum comment, the 4 mod.rs headers + member lists, router.rs expert total).

## Session

zed-m3-b320, epoch 1793356105 (claim) → landing. The i1117 128k dense collection owned ~10 cores throughout — every gate ran under the isolated target dir with default job caps; no GPU touched.
