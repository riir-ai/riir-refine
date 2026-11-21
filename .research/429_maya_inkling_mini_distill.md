# 429 — maya-inkling-mini distill (alesha-pro/maya-inkling-mini)

**Status:** DISTILLED — pending owner decision

## Source

- URL: https://github.com/alesha-pro/maya-inkling-mini
- PIN: `be849daa81790b7d12b74ba9e2f7b78ed4ef43cd` (single squashed commit, tip commit-dated 2026-11-21T14:04:51+03:00)
- License: MIT (code, "Peasant Smith and the Project Maya contributors" + "Niko1221 and the Strata contributors"); model weights Apache 2.0 (not mined)
- Provenance chain: Project Maya v1.1.70 (mw00/project-maya, by Peasant Smith) ← Strata (Niko1221); `maya-ik2` began as Maya PRs #981/#1079's Inkling-2 engine; this fork adds the loop, the all-experts-resident path, TC kernels, FP8 KV, GPU sampling. Model: Thinking Machines Inkling-2-Mini (26B looped MoE, 24 layers, layers 6..17 ×3 = 48 visits/token, 128 experts ×8/token, UD-Q4_K_XL GGUF by Unsloth). Target: one RTX 3090 (sm_86), 24 GB, all 3,072 experts resident.
- Inspection date: 2026-11-21 (m3, box QUIET load 4.1/16, zero cargo, zero GPU)
- Shallow clone `riir-refine/.raw/maya-inkling-mini`, REMOVED at close; every quote below read at the pin this session.

## What the source is

A CUDA C++ single-GPU serving engine for one model family — a MEASURED-CONFIG +
KERNEL source (both halves real: an engine with its own kernels AND a guide that
reports its negatives). Module-level grading: only `src/ik2/` (~11K lines) +
`docs/INKLING2-MINI.md` + `docs/INKLING2.md` §6–8 are this verdict's surface. The
tree's other model families (`ds4/` GLM-DSA 23.8K lines, `glm_*`, `hy4/`, `mm3/`,
`exl3.cu`) are OTHER models' engines — out of scope, own verdicts if ever wanted.
`ref/upstream-llama.cpp/` is a vendored reference (not mineable surface). Quality
culture signals: every kernel held to a double-precision CPU reference
(`ik2_kernels_test`), GSM8K 96-question greedy cells per kernel family, KL
median cells vs the FP16 posture, every guide number tied to a command line
(in-source), `CMAKE_BUILD_TYPE=Release` defaults.

## Headline measured cells (provenance, never bars — their 3090, their model)

- Decode 128.7 tok/s @47-token prompt vs llama.cpp b12590 57.0 — 2.26×; prefill
  2,742 tok/s @4K. At 340K context decode 62.3 (the physical floor: 8 of 48
  visits attend full prefix, 16 KB cache/token-of-context read per new token).
- Launch arithmetic: llama.cpp ≈7,350 launches/token (graphless on this arch) ×
  "each 2 µs and more" ≈ 14.7 ms ≈ 84% of a 17.5 ms token. maya-ik2: fuse
  1,404 → 781 launches, replay as ONE CUDA graph.
- Bytes-in-flight pair: naive TC expert kernel 100 tok/s < scalar 118; after
  "8 warps on the same 16 rows, each taking its own stride of the row" →
  745 GB/s. Decode attention register-direct: 846 GB/s (FP16 cache) / 776 GB/s
  (FP8); the smem-tiled flash way reads "an FP8 one at a third of it".
- FP8 KV vs FP16: Mini median KL 1.1e-3, shipped default; Small: FP8 flips 5/48
  top-1 ("the router is full of near-ties") → FP16 default there. Per-model
  lossy verdicts, opposite defaults, same change.
- GSM8K: TC expert kernels 74/96 vs scalar 73/96 (lossy surface, per-family
  gate, tiny cost); int8+DP4A expert-input approximation 71/96 — refused.
- DSpark drafter: 2.0–4.2 accepted/step (prose 2.0, code 3.1), break-even 3.3,
  window 8: code 139 tok/s (+8.6%), prose 87 (−32%).

## Ranked candidates

| # | pattern | source site | grade | notes |
|---|---|---|---|---|
| R1 | `decode-attn-cache-rows-register-direct-occupancy-not-smem-tile` | `ik2_kernels.cu:1349–1356` | B+ LEAD | two-regime by per-row consumer count; adjudicate vs gpu_106 (below) |
| R2 | `fp32-input-hi-lo-two-fp16-mma-columns` | `ik2_kernels.cu:3842` + :2770 + README | B− | the OPERAND-side precision-recovery axis; fresh dedup |
| R3 | `bytes-in-flight-concurrency-not-prefetch-ahead` | README "Weights in flight" + both negatives sections | B− (face-or-rule at authoring) | the two-knob regime law, both directions measured |
| F1 | face on `lop3-int4-to-half2-dequant` | `ik2_kernels.cu:2077–2110` | count-neutral | signed-E2M1 `__byte_perm` LUT decode, sm_86-gated arm |
| F2 | face on `kv-dequant-materialization-batch-threshold-gate` (gpu_34) | `ik2_kernels.cu:1349` + fp8 helpers :69–99 | count-neutral | the read-quantized arm's decode-attention instance |
| F3 | positive-instance record on the graph family (gpu_01/07/08/131) | `ik2_main.cu:2082–2096`, :2169, :2450–2461 | record, never land | device-side routing/position → token-invariant sequence → whole-token capture |
| F4 | face on gpu_08's state-horizon lossy-surface doctrine | INKLING2.md §6.1 FP8 vs INKLING2-MINI config | count-neutral | THIRD lossy pathway: discrete routing amplification |

Drops (recorded, the gate working): conv4/sink/norm/message launch-fusions
(model-shape-specific instances of the owned single-launch families
`stacked-gate-up-single-launch-split-store` gpu_106, `same-input-gemv-segments-single-launch`
gpu_07, `whole-step-gdn-decode-window-one-launch` gpu_126 — the 2 µs/launch floor
constant rides F3's record); `mv_q8p` values/scales-apart Q8_0 layout (our loaders
take quants apart already; adjacent, no new axis); short-prompt expert-once
prefill path (the batch-threshold class, gpu_34's gate owns the axis); 1M-context
RAM↔VRAM expert swap + AVX2 CPU expert lane (off-config experimental); GPU
sampling histogram (adjacent to owned sampler families); DSpark drafter CODE
(third-party graft — the measured negative is the datum, not the code).

## Dedup evidence (both vocabularies, greps this session at the live corpus)

- R1: `decode.step|one.unit|no.smem|register.*direct|occupancy` over
  kernel_opt corpus → nearest kin `j16-warp-fragment-direct-no-smem-tile`
  (gpu_106) owns the small-J MMQ STAGING twin: "at J=16 activation columns the
  weight bytes have NO cross-column reuse an smem tile can add… stage NOTHING…
  No shared memory, no barriers" — the reuse-arithmetic admission. maya's
  attention variant is ADJACENT-DISTINCT: its admission is the decode cache-row
  single-read per KV-head group ("A block's warps share a K / V head… a key's K
  and V rows are read once for them all"), its mechanism names OCCUPANCY
  explicitly ("a dozen and more warps an SM… a warp's wait for its rows is
  another warp's arithmetic"), and it carries the measured smem-path contrast
  (FP8 at ⅓ speed). Mandatory in-rule adjudication both directions at authoring;
  face-or-rule is the batch lane's call (the gpu_106 kinship may resolve it a
  face — disclosed).
- R2: `two fp16|two-term|hi.lo|fp32 emulation|split.*precision` → zero owners;
  `lop3-int4-to-half2-dequant` family (with the 2-bit ladder + twin-decode +
  prescale-underflow-negative riders) owns the WEIGHT-side mantissa embedding;
  B242 `masked-in-place-field-prescale-other-operand` (entries_gpu_98) owns the
  OTHER-OPERAND prescale axis. The operand-side hi+lo split-sum is unowned.
- R3: `bytes.*flight|flight.*bytes|requested at once` → zero phrase owners (the
  one "in flight" hit is gpu_07's command-buffer rule — unrelated); kin rules
  `one-barrier-a-tile-prefetch-occupancy-pair` (gpu_102, barrier-staging
  coupling — different axis), the uint4 face on `strided-loop-batched-prefetch-unroll-tail`
  (gpu_150, per-thread dependent trips), `fold-query-tokens-for-occupancy` (gpu_01).
  The two-knob regime split (concurrency vs ahead) with BOTH directions measured
  is unowned; may still dissolve into a face on the prefetch families at authoring.
- Graph family: `cuda-graph-scalar-via-device-pointer` (gpu_01) + `captured-graph-bakes-buffer-addresses`
  (gpu_07) + `graph-replay-skips-host-node-scans` (gpu_131) + `capture-break-count-not-perf-target`
  (gpu_08) — maya's device-tables/whole-token capture is a POSITIVE INSTANCE
  cluster, recorded never re-landed (F3).

## GOAT check

No benchable candidate: every rule's win metric lives on an sm_86 CUDA host we
do not run (ours are Metal + sm_89, and our league models are dense ternary /
dense qwen3.8 — no looped MoE, no FP4-class weights). Criterion 1 fails for a
DOMAIN reason → per-rule `NotBenchable`, `bench_spec: None`, suggestion-only,
first-class decision content (Batch-136 / B333 precedent). Criteria 2–4: R1/R2
carry exact-semantics stories (bit-identity by construction claims in-source;
the hi+lo split is Dekker-exact) and mechanism-level magnitudes (⅓-speed smem
FP8 vs memory-speed register FP8; 100→745 GB/s) — the pattern-weak class does
NOT fire.

## Lane-intel (mandatory output)

MEDIUM — corroborative; NO open issue moves, NO re-open fires.

1. **League decode row (4090):** our decode hold is 1.02–1.06× vs the prism
   fork with BOTH sides already on CUDA-graph replay (the fork's #393 was
   literally "no node scans on CUDA-graph replay"; our steady host+submit
   overhead ≈ 334 µs/token, Bench 976 — the submit-amortization lane DEMOTED,
   "no 1 ms pot"). maya's 2.26× is the launch-amortization of a GRAPHLESS
   opponent on a high-visit-count arch (7,350 × 2 µs ≈ 84% of the token) — a
   third-party magnitude confirmation of why graph capture is table stakes and
   why graphless serving dies specifically on looped/deep-MoE shapes. It does
   NOT re-open our demote. No league action; this line is the record.
2. **K-i1 `decode-step-inflight-chaining`** (riir-ai Research 378's staged law):
   FOURTH engine instance — uzu in-flight chaining, splash unit-submission, the
   prism fork's graph replay, maya's whole-token graph. Corroboration row;
   never re-staged.
3. **Specdec (riir-infer Issue 1179, PAUSED):** the DSpark prose negative
   (2.0 accepted, −32% on prose vs +8.6% on code at a 128 tok/s base)
   corroborates the pause; re-open triggers unchanged.
4. **Q8KV (Issue 1211, default-on Metal):** FP8 KV halves the depth-term cache
   reads; the 16 KB/token-of-context arithmetic corroborates the bandwidth law
   cross-vendor. No change.
5. **GEMV occupancy rule (default-on):** bytes-in-flight is the same class;
   maya's prefetch-ahead negative is a clean third-party anti-regime datum.
6. NOT transferable: the resident-expert MoE plane (our league models are
   dense); the looped-arch message passing (no looped model); their absolute
   tok/s (different model + silicon; provenance only).

## Product-coverage axis

NONE. The CUDA LLM-serving kernel family is deeply covered
(sgl-nvidia-mega B333, TensorFold, ThreeRT, the vllm/llama.cpp PR lanes); no
famous-lib gap seeded; no follow-up pool owed.

## Caveats

- Single squashed commit: no git archaeology; every quote verified against the
  TREE at the pin only. Upstream-state (P113): the ik2 engine IS this fork's own
  landed decision — every row decision-content, the outbound-PR lane empty by
  construction (ThreeRT precedent).
- Young fast-moving lineage (Maya v1.1.70 base; re-clone + re-pin at T0; the
  guide's numbers are source-reported on THEIR 3090/driver/power-limit posture).
- UNREAD residue (re-read rows at mine time): `experts_mma_kernel` body
  (the hi+lo input site is read as the :3842 comment + launcher; the kernel
  body itself unread), `ik2_main.cu` graph-capture body beyond the excerpts
  (:2082–2096, :2169, :2450–2461), the prefill chunk path, `tools/strata_tokenizer.py`,
  `serve/server.py`, all `ds4/glm/hy4/mm3` families (out of scope by routing).
- The `nvfp4_sub_dot` LUT decode quoted at :2077–2110 is the SCALAR dot's
  decoder (float4 x FP32 inputs); the TC expert kernel's operand layout is R2's
  claim and is confirmed by the :3842 comment + README — the full body read is
  the T0 row that closes it.
- m3 was QUIET throughout (load 4.1/16); zero cargo, zero GPU.

## Queue slot

Staged as the next kernel_opt transcription batch (nominal Batch 378 window,
behind whatever the anemll B377 follow-on claims); Plan 395 staged plan-first —
the cargo half awaits a free window, the claim row lands when the lane starts.
Corroboration package (measured negatives + K-i1 fourth instance + launch-floor
arithmetic) rides the queue line, not the corpus.
