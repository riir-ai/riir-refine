# Batch 288 — the b10826 fork decode quartet + the grammar stack-walk lead

**Status:** CLAIMED 2026-10-26 ~11:4x +07 (shikuwa/4090, riir-refine idle loop — walk-17-continuation session; the 10-26 ~03:4x WATCH POLL staged leads, slot fresh — B287 landed `d871a0fc7` this morning, B288 unclaimed at claim time; zero GPU throughout — the sibling QRR cell holds it, kernel_opt corpus work is CPU-only; box: CPU 15%, ~14 GiB free RAM, C: 78 GB / E: 348 GB free). Plan + claim land in one commit, PUSH before authoring.

## Sources (verify EVERY quote verbatim at these revs at T0 — the B96-98 law)

- **PrismML fork @ `c9c34bc9`** (the scored tag `prism-b10826-c9c34bc9`, = b10822 + exactly 4 fork decode commits, merge-base == b10822, NO upstream merges in the delta — the watch poll's compare `bf8c069f..c9c34bc9` ahead_by 4 / behind 0):
  - #395 `77a0affd` — producer-side quantize fold: the fused ADD+RMS_NORM+MUL+FWHT kernel writes q8_1 rows DIRECTLY (AoS at 1 column, PQ2 layout at multi-column widths); consumers find them via `ctx.fwht_q8()` and skip their quantize launch; bit-identical; `GGML_CUDA_NORM_FWHT_Q8` DEFAULT-ON; consumer scan extracted to shared helper `ggml_cuda_fwht_q8_find_uses`; the norm-fusion × FWHT_FUSION_PQ2 env mutual-exclusion DELETED.
  - #397 `9f914845` — PQ2_0 multi-col mat-vec serves 1-2 columns on HOPPER with bias + gate/GLU epilogue fusion as TEMPLATE PARAMS (`GGML_CUDA_PQ2_MC_MIN`: Hopper 1 / elsewhere 3, env override; +6% 9B PQ2_0 decode; NOT bit-identical to the generic kernel — accumulation order; the runtime-bias-check-costs-the-8-col-verify-~12% constexpr law; HOPPER-GATED new arm — Ada keeps 3).
  - #401 `dc21b2cf` — block-diagonal Hadamard in the fused norm+transform kernel: blk ∈ {512,1024,2048,4096} ≤ ncols; only the butterfly stages with stride < blk run; each segment bit-identical to a width-blk transform (`fwht_block_butterfly<N,NT,B>` template widening); 2B decode +6-7%, 4B/9B unaffected.
  - #402 `c9c34bc9` (THE TAG COMMIT) — o_proj/ffn_down Hadamard→q8_1 fold for PQ2_0 PROMOTED DEFAULT-ON (`GGML_CUDA_FWHT_FUSION_PQ2=0` restores; H200: 2B +5.6-5.9% / 4B +8.1-9.2% / 9B +0.4-0.7% / **27B UNAFFECTED**; validated on dc21b2cf job 32242). With #395: the producer-registers-prequantized-rows + consumer-lookup-skips-launch pair.
- **ggml-org/llama.cpp #31153 @ `48c9f08b`** — grammar-reject-recursion→explicit-stack-walk (correctness: the depth limit counted a vocab token's code points, so ANY ≥256-char token was ALWAYS rejected under a grammar; candidates-as-indices-into-input-not-copies; "faster than master on GLM-4.7, Laguna, Qwen3.6, gemma-4, same masks"; thin rust_perf lead).
- **SKIP (documented):** upstream `d854d98b` #30463 — CUDA VLA→std::string (init-path modernization, allocation ADDED on an init path — SKIP-class per the watch poll).

## Candidates (the watch poll's staged split is INPUT; T1 greps + T2 authoring adjudicate final disposition)

- **P84 #395+#402** — producer-registers-prequantized-rows-in-a-context-table + consumer-lookup-skips-launch: the #30489 consumer-fold family's PRODUCER-side twin → rider on `fused-activation-quantize-for-gemv-consumer` (entries_gpu_01, the watch poll's named dedup anchor). Faces: the aliases[8] reshape-view walk + use-count guard + the cuBLAS-padded-view refusal + the default-on promotion posture (the old opt-in flipped).
- **P85 #401** — block-diagonal-transform-runs-only-stride<blk-stages-bit-identical-per-segment → rider on the FWHT ladder family, B116 host `fwht-wide-block-row-per-threadgroup-exchange-ladder` (entries_gpu_27) per the watch poll's adjudication; T1 re-checks against `fwht-butterfly-staging-ladder-warp-smem-register` (gpu_45, B166) + the B283 look-ahead rule (gpu_128, the `fwht_block_butterfly` shared-transcription lineage).
- **P86 #397a** — per-cc-min-column-dispatch-table (the min-column floor is PER-COMPUTE-CAPABILITY + env-overridable, never one global constant) → candidate host `regime-batch-floor-force-escape` (gpu_54); T1 adjudicates.
- **P87 #397b** — fusion-as-template-param-vs-runtime-branch (the runtime-bias-check-costs-the-fused-path-its-win constexpr law) → host TBD by T1 grep (template-param / constexpr / runtime-branch family); may fold INTO P86 as one rider's second face if the same host owns both.
- **P88 #31153** — grammar-reject-recursion→explicit-stack-walk → rust_perf lane; candidate host `grammar-window-draft-prune-path-local-mask` (rust_perf entries_11) or thin own rule — T1 decides; the depth-limit-misapplied-to-the-wrong-axis class + candidates-as-indices-not-copies are the decision content.
- **SKIP #30463** — documented in the research note, no entry.

## Tasks

- [ ] T0 Clone `.raw/b288-llamacpp` (blob-filtered; dump+grep per file — tree-wide `git grep` hangs on blob-filtered clones) at the fork pin `c9c34bc9`; fetch upstream #31153's diff (`48c9f08b`); re-verify every quote verbatim at T2 authoring time (per-file `git show`).
- [ ] T1 Dedup re-verification, 3+ vocabulary variants per candidate face (corpus stands 977 kernel_opt + 273 rust_perf since B287/B284): producer-fold/prequantized/context-table/lookup-skip; block-diagonal/stage-skip/segment-identity; per-cc/min-column/dispatch-table/floor; template-param/constexpr/runtime-branch; grammar/stack-walk/depth-limit/indices-not-copies. Confirm or re-adjudicate the watch poll's host dispositions.
- [ ] T2 Author: riders on their owned hosts + any own rule (renumber per first-allocation-keeps; count-neutral honest if T1 lands all-rider) + ORIGINAL minimal fixtures if owed + the batch288 needle + parse gates.
- [ ] T3 Count pins ONLY if an own rule lands (kernel_opt total, category counts, router expert total, Plan-003 table if touched, AGENTS.md ×2; rust_perf count + AGENTS if a rust_perf rule lands) — count-neutral riders skip.
- [ ] T4 Standing gates: batch288 needle + parse gates; kernel_opt lib (`--features kernel_opt`) 0F; default lib 0F; clippy `-D warnings` ×3 postures (default / all-features / no-default, --all-targets).
- [ ] T5 Retrieval floors re-measure (corpus delta even count-neutral — riders edit rule bodies): kernel self-variants + blended floors HELD at BOTH postures (kernel_opt + the default-posture `retrieval_` lane, Issue 170 T3), no re-pin expected; a decay FAILS the batch.
- [ ] T6 Record owed/not-owed: freeze gen2/gen3 NOT owed (zero clippy-corpus delta, P30); score-bench owed ONLY if the rust_perf corpus delta lands (P88 own rule); release-set binary rebuild NOT owed (kernel_opt/rust_perf outside the release set) — re-verify at landing.
- [ ] T7 Research note `.research/327_batch288_b10826_quartet_grammar_stack.md` (`.research/.highwater` 326→327; re-scan at write + dual_allocation_gate) + `.docs/11_domains/kernel_opt.md` batch row (landed WITH the batch — the B286 missing-row repair precedent) + the snapshot landing row + the watch-poll staged-lead marks; `rm -rf .raw/b288-*` at close.
- [ ] T8 Commit `feat: batch 288: ...` + push (claim discharged by the landing row).

## Sequencing

- The b10826 bout (`.distill/003` runbook) is queued behind the sibling QRR cell (~14:40 +07 ETA) — DISJOINT from this batch (zero GPU here); if the GPU frees mid-batch, the bout takes the box and the batch resumes after (the gates are CPU-bound, interleaving is safe — the B286 claim row's sequencing law).
- Walk #18 (Tue 00:00Z +07 07:00, claim-first) is time-gated past this batch's window — zero conflict.
- #30489 watch-join: still OPEN upstream (head `784fd016` unchanged at the watch poll's re-check) — B287's rider disclosure stands.
