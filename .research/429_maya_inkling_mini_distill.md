# 429 — maya-inkling-mini distill (alesha-pro/maya-inkling-mini)

**Status:** LANDED 2026-11-22 — Batch 379 (Plan 395, zed-m3-maya@m3): 3 rules + 3 count-neutral faces (see the queue snapshot LANDING row); the T0/T1 findings folded below.

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

## Caveats

**LANDED-UPDATE 2026-11-22 (the batch lane's T0/T1 folds):**

- **Upstream MOVED +3 commits past the pin** (`c520fd14` the specdec-drafter
  tip + README/guide/config, `fc911481` dual-GPU EP, `82de2288` the
  xq16_sup buffer fix) — every mined quote site verified BYTE-EXACT at BOTH
  pin and tip (R1's two lines at pin :1349/:1355 = tip :1353/:1359; R2's
  "x = hi + lo in FP16" both; F1's byte_perm sites both); the "single
  squashed commit" reading was a depth-1 artifact (the pin has a docs-only
  parent). No re-pin (the pin is the arbiter; the moved surface — specdec
  drafter kernels ~1000 lines + dual-GPU EP — is future re-distill
  lane-intel, never re-mined blind).
- **UNREAD residue closed** (T0): the `experts_mma_kernel` body read (R2's
  operand layout CONFIRMED — the :2814-2816 comment + the
  `g == 1 ? C : 0` lane-group limb read + the `v[0] + v[1]` column fold);
  the `ik2_main.cu` graph body read (capture per OutMode × tier state,
  replace after `cudaStreamSynchronize`, `g_fgraph[2][4]`); the prefill
  flash path read (`attn_flash_f16_kernel` smem tiles + the Tq-row grid —
  R1's anti-regime evidence).
- **T1 verdicts:** R1 = RULE (not a face) — the gpu_106 kinship resolves
  ADJACENT-DISTINCT (gpu_106 owns the reuse-ARITHMETIC pole R=1; R1's rows
  have R <= 8 consumers whose reuse is fragment-internal, admission =
  occupancy + materialization tax, two-regime vs the smem flash prefill);
  R2 = RULE adjacent-distinct vs gpu_93 (the 3-mma both-operand form vs
  the spare-columns zero-mma one-sided form — adjudicated in-rule both
  directions; gpu_102's pow2-prescale + B242's other-operand prescale
  named); R3 = RULE (the two-knob bytes-in-flight law with the measured
  ceiling anti-regime; the prefetch families own HOW, this owns
  WHETHER/WHERE — distinct lens from gpu_94's SM-occupancy). All three
  suggestion-only NotBenchable, `bench_spec: None`, category A/N/A.
- **Faces:** F1 + F2 + F4 landed count-neutral on their owners (the exact
  sites in the queue snapshot LANDING row); F3 stays a RECORD (this note's
  §Lane-intel row 2 + the snapshot line) — never a landing.
- **Fixture gate:** 17/17 standalone (b379_fixtures.rs, debug AND -O,
  zero warnings) — caught FOUR arm defects (the subnormal shift math
  `1u32 << 37` overflow, a coarse sweep step, an over-strict exactness
  assert — residuals with 12-13 significant bits are NOT f16-exact, they
  are bounded — and the R3 model's missing per-stream MLP-reach clamp,
  which IS the mechanism's reason concurrency is the lever).

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

---

## Delta 2026-11-22 — the moved surface re-distilled (pin `be849daa` → tip `e819852f822479af783638a467bfdfe2ed6882ec`, 4090-lane pass)

**Status (Batch 380 landing, 2026-11-22):** LANDED as Batch 380 (Plan 397, zed-m3-maya-redistill@m3) — kernel_opt 1096→1097 (entries_gpu_160: R-A `expert-miss-computes-on-cpu-overlap-gpu-cache`, category H; F1 the SpecCtl contrast arm on gpu_29, F2 the DP4A limb sibling + exactness law on B379-R2, F3 folded in-rule); the G1 census repair rode the landing (entries_gpu_159 joined the census lists B379's landing missed); measured receipts in the queue snapshot LANDING row. CORRECTION OF RECORD: upstream rewrote history past `be849daa` — the "exact `rev-list --count` at landing" this section reserved no longer resolves (the pin's ancestry is gone); the pin is RECORD-ONLY now, the delta content tip-verified at `e819852f`. The landing note's batch number: 380 (the omarchy-mlx queue slot that nominally held it is owner-gated; the re-derive clause governs).

Re-clone at tip `e819852f` (shallow, MIT unchanged, weights still Apache-2.0). The pin's post-landing upstream drift (B379 T0 named +3 commits) grew well past that (a log-window read; exact `rev-list --count` at landing per the verdict), and the delta is exactly the 4090 lane's story: **the engine now runs the model on ONE RTX 3090 (sm_86, 24 GB) with every expert resident — 128 tok/s plain decode, 150–210 with the official drafter, 2,742 tok/s prefill@4K, 344K context FP8 KV — vs llama.cpp b12590's 57 tok/s on the same card** (README headline + numbers table, byte-verified this session). Provenance completed: "This is Project Maya v1.1.70… most of it is offered upstream in mw00/project-maya#1095"; the Windows/CUDA-13 build + dual-GPU EP by @cobra91, the 8 GB path by @ImagineNotChetng (PR #4).

### What moved (commit census at tip)

`b57ce28` RTX-3090 README/guide/configs · `c43ca24`+`a2f0c66`+`c520fd1` speculative decoding with the OFFICIAL drafter (Thinking Machines ships `mtp.safetensors`, 0.6 GB, 3 DSpark layers, taps at visits 5/23/41 + final hidden) as the default · `fc91148` native Windows MSVC + CUDA 13.x + dual-GPU EP (`IK2_EP=1`, tested RTX 5080 + 4060 Ti) · `82de228` xq16_sup buffer allocated on dual-GPU for prompt prefill · `8dd60ff`+`69dc06d` the 8-GB-card path · `997c84f` the VRAM-fit resident guard · `81b8130`+`28b6658` MSVC/CUDA13 hardening · `549d734` tool-call comma repair.

### Re-distill candidates (dedup at 1745-rule corpus, this session)

- **R-A `expert-miss-computes-on-cpu-overlap-gpu-cache` B+ LEAD** — the 8-GB tier: "The experts stay in pinned host memory (14.7 GiB of RAM), the GPU keeps a 3.8 GiB cache of them, and the CPU computes each missing expert from host memory while the GPU runs the cached ones" — measured 40–43 tok/s on an 8 GB RTX 5070 Laptop where llama.cpp-with-experts-on-CPU gets 12.5 (3.2×). The unowned axis is the OVERLAP (a host pool computes a miss concurrent with the GPU's cached execution; measured 37–41 GB/s of the RAM's 58), not the FIFO policy — entries_39 owns ghost-table DATA STRUCTURES, `whole-matrix-unpack-pass` (entries_gpu_148) owns the unpack crossover; grep `S3.FIFO|miss.*compute|pinned host.*expert` = zero owners. NotBenchable-by-domain (the win is a heterogeneous-overlap latency, no CPU validator hosts it) — decision content + measured-source provenance.
- **F1 face on gpu_29 `speculation-scheduler-break-even-flip` — the CONTRAST arm, never a positive instance** — gpu_29's remedy is a MONOTONE bypass (once it flips to plain decode it never comes back); SpecCtl is the opposite policy for the same break-even problem: "A window of B tokens is worth (1 − a^B) / (1 − a) tokens… drafts when that, over the window's measured time, beats a plain token's time, and after 8 plain tokens it drafts once to look again" — the re-probe exists because acceptance varies by prompt family (the face's regime condition: when acceptance is text-class-dependent, re-probing beats a permanent bypass; gpu_29's own acceptance spread shows the variation). Count-neutral face.
- **F2 face on B379-R2 `fp32-input-hi-lo-two-fp16-mma-columns`** — the DP4A sibling arm at the same file: "Two signed INT8 limbs approximate each activation for DP4A with the serialized NVFP4 or Q8 weights" (`expert_limb_mv`, ik2_kernels.cuh :173–177 — the MMA arm R2 mined is the fp16-limb twin at :3846–3849, quote re-verified at tip). Count-neutral.
- **F3 face on the measurement-honesty family** — the FP8-KV drift disclosure: "any two FP8 runs part within a few tokens: a difference of 1e-5 flips codes of a 3-bit mantissa" (INKLING2.md §8) — corroborates the drift-guard/determinism posture (Issue 1182, Issue-940 carve-out). Also "an NVFP4 weight is exact in FP16" + "a code times its scale exact in FP16, the sum FP32" — the exactness argument rides F2 in-rule.
- **Drops:** the resident guard (997c84f — a boot-time VRAM-fit diagnostic with the exact `--max-context` hint; config-class, no Rust hot-path shape), dual-GPU EP + xq16_sup (config-class), the tool-call comma repair (a parser-tolerance fix in the serve layer — lane-intel for the decision family, not corpus).

### The 4090-lane mapping (league + runnability)

- **Runnability datum:** our 4090 (24 GB, sm_89) is the same CLASS the engine targets (sm_86 builds) — an sm_89 build is T0-able at the 4090 (arch-upward expected: DP4A/mma/cp.async all exist on Ada — unproven here, never claimed until compiled); Inkling-2-Mini resident fits 23.6–23.8 GiB at 344K context. The launch-count datum: llama.cpp ~7,350 launches/token → maya 757 replayed as one CUDA graph. ⚠ Model-scope law: running Inkling on the 4090 stays LOW-priority lane intel — the league owns the card (Bonsai/qwen3.8).
- **League read-across (foreign-model caveat, never a bar):** the posture stack that buys 2.2× over llama.cpp on one card is register-direct decode attention + single-graph replay + all-experts-resident + FP8 KV — the same families our 4090 decode lane (Bonsai tg128 ~0.970×) already mines. The 8-GB tier is offload intel for any future VRAM-tight cell.
- **Specdec lanes:** the OFFICIAL drafter exists as a shipped artifact (mtp.safetensors) — for Inkling. Our paused 1179/1183 verdict stands (a Bonsai-target drafter still ranks first); SpecCtl's economics face is the transferable content.
- **Answers katgpt-infer Research 038 §3.1 T0 condition (a):** a GGUF FP4 carrier EXISTS — "UD-Q4_K_XL (17.6 GB) holds the experts of layers 0-19 in NVFP4 — their original bits" (Unsloth), read by llama.cpp b12590+ (PR #32695) and by this engine without requantizing. The dense-lane NVFP4 gate's first condition is measured TRUE for the Inkling family; the qwen3.8/Bonsai applicability question is unchanged.

**Verdict: YES B− re-distill batch** (1 B+ rule + 3 faces; nominal Batch number re-derives at landing), 4090-lane execution posture (CUDA fixtures/cells on the 4090, sm_89 arch bump is T0-able), Plan 397 staged unclaimed. Quotes above byte-verified at `e819852f` this session; clone removed at record commit.
