# Research 237 — arxiv pool refill walk #13 close-out (2026-10-06)

**Status:** RECORD — walk executed 2026-10-06 ~06:3x–07:0x UTC (13:3x–14:0x +07,
M3, the due clock; window 2026-09-29T00:00Z → 2026-10-06, graded on the
post-announce-filled index per the walk-#12 lesson). **Zero corpus delta: 0
rule-batches — the FOURTH consecutive zero-delta walk.** Lane profile:
spec-dec 23 · plain-KV 41 · quant-broad 103 (60 fetched first-pass — the
unfetched tail is the physics/quant-ph/hep class, counted as bulk) ·
gpu-kernel 15 · cs-PF 14 — **97 fresh rows** after dedup against Research 219
(walk #12's graded + skip IDs) + the in-window catches (Research 236 triadic
`2609.36529`; Research 228 RLTL;DR `2609.37633`; Research 216's
`2609.39827` rider — none of the three appears in the lane rows, the dedup
is belt-and-braces). First-try clean on all five lanes (the walk-#12 429
throttle did NOT reproduce at this hour; 9 s inter-query spacing held) —
the walk-#12 instrument note stands: throttling is hour-of-day-dependent,
not a standing property.

## Graded rows

| # | ID | Title (short) | Lane | Grade | Reading |
|---|---|---|---|---|---|
| 1 | **2609.38169** | STEPQuant: When and Where Errors Matter in Delta-Rule Recurrent State Quantization | quant-broad | **B+ lane record (league intel HIGH)** | Linear-attention recurrent-state quantization: "quantization errors propagate through successive state updates"; error impact depends on two conditions (when/where in the state — abstract cut at the condition list). **The league arch (GDN/Bonsai-27B) is a delta-rule recurrent net — state quantization is the PQ2_0-class axis one tier deeper** (per-state-component / per-update-phase precision). Routing: riir-ai Issue 971 dated entry + riir-train lane pointer (state-quant for GDN, beside Plan 442's upcycle watch). Source read at dispatch, plan-first — never mined from the abstract. |
| 2 | **2609.38981** | Vosti: Specifying, Implementing, and Verifying Deterministic LLM Inference | gpu-kernel/cs-PF | **B doctrine record (determinism lane)** | Formal system-level specification: batch composition, prompt chunking, prefill/decode execution, KV reuse/eviction/recomputation "should not affect system outputs" — vLLM batch-invariant mode + SGLang deterministic mode as informal targets, formalized + verified. **This is B200's `concurrent-round-parity-solo-equal` law and the workspace's raw-domain sync-boundary law, formalized externally** (batch-composition-invariance as a first-class verified property, not a test posture). Echo recorded; no corpus change — our kernels already carry the construction-level law. |
| 3 | **2610.00432** | XOR-Trellis: Ultra-Low-Complexity Dequantization and Curvature-Aware Hadamard-Free LLM Quantization | quant-broad | **B lane record (transform-before-quant family)** | Trellis-coded quantization at ultra-low bit widths + (a) parallel-throughput dequantization so it is not the inference bottleneck, (b) curvature-aware **Hadamard-FREE** transform. The counter-family datum for the RotaryQuant lineage (walk #12 P1): rotation-before-quantization now has a branch that does NOT pay the rotation — kinship with our Issue-716 sink-guard Hadamard rejection (the "rotation cost strictly larger" premise has a measured third answer: avoid the rotation, keep the isotropization). Family instances this walk: WUSH-KV `2609.38121` (second-order-statistics data-adaptive transform) + 1-bit KV `2610.03027` (codebook-tailored VQ). No consumer today; the family note rides the ternary quant watch. |
| 4 | **2609.39816** | Beyond Accuracy: Prefix-Invariant Realizations of Low-Precision Fast Matrix Multiplication | quant-broad | **B measurement datum (invariance law)** | Fast matmul's exact cancellations mix token rows under rounding — "a scored likelihood must depend only on its allowed prefix" breaks; two fast FP8 realizations repaired to prefix-invariance on Qwen2.5-14B. **The prefix-invariance requirement is our measurement-honesty family's invariance axis** (B200 parity, Vosti above, B109 aa-control): "beyond accuracy" IS the lossy-surface law's sibling — a numerics choice can be accuracy-flat and invariance-broken. Datum recorded beside the family. |
| 5 | **2610.00694** | How Divergence Becomes Decision Flips in Compressed Language Models | quant-broad | **B− doctrine datum (lossy-surface law, one lane over)** | Across 802 compressed/perturbed copies of 19 models × 5 corpora: **total variation, not KL, predicts decision flips directly** — the quantitative form of the Issue-750-T3/Orthrus class (aggregate metrics flat while per-prompt behavior flips). Deployment-side doctrine: report TV when consumers rely on decisions, not KL. External confirmation of the promotion rule's framing; record only. |
| 6 | **2609.38706** | Preserving Provenance in Shared KV Caches for LLM Serving | gpu-kernel/plain-KV | **B− lane record (cache-key provenance hazard class)** | Shared KV tier keyed only by token content + coarse metadata erases provenance — "identical tokens under incompatible computational [contexts collide]" (adapter/weight-config/sharing-domain mismatch). **A RULE-SHAPED engineering law: a shared cache key must carry every axis that changes the computation** — kinship with B198's `graph-pool-handle-fresh-per-generation` (handle recycling) and the freeze/thaw versioned readers. Rule seed noted for a future rust_perf/caching conversion; never mined from the abstract alone. |
| 7 | **2610.03228** | Execution-Path Qualification and Realized Costs in Speculative Decoding on Consumer Systems | cs-PF | **B− measurement record (pre-registration family)** | "Numerical execution paths can change greedy output; a mismatch alone does not predict the remaining trajectory" — draft-free target-only predictor selected on exposed discovery data, **frozen before reserved live speculation**, matches 72/72 capped sequences. The freeze-before-live protocol is our pre-registration/twin-discard discipline, external instance on consumer GPUs (Intel B570 stack). Record. |
| 8 | **2609.39361** | LampAttention: Look-Ahead Mixed-Precision FlashAttention for Dedicated Accelerators | gpu-kernel | **B− kernel-lane intel (attention numerics)** | Most attention logits survive 8-bit accumulation; the kernel accumulates KQ products + exponentials in 8-bit and **adaptively re-elevates sensitive sub-blocks** (look-ahead identification). Decision content: numerics-adaptive precision escalation inside one kernel — the MMA-accumulator-family's dynamic-precision axis (B178 exact-widening's adaptive sibling). Dedicated-accelerator regime; our 4090 FA arm is the watch lane. Record. |
| 9 | **2609.37241** | Trident: Unifying Guarded Dispatch and Host Execution for PyTorch Triton Workloads | gpu-kernel | **B− lane record (host-orchestration family)** | Short-kernel e2e latency dominated by host-side orchestration: runtime specialization lookup + guard evaluation + preparation per invocation; Trident bakes native host wrappers past the guarded-dispatch path. Kinship: B189 `shape-bucketed-graph-cache-lru` + B178 `graph-exec-update-gated-by-captured-topology-signature` + B200 `lone-stream-graph-replay-mode-switch` — the "host overhead dominates short device execution" law, PyTorch/Triton instance. Record. |
| 10 | **2609.39358** | Galahad: Byte-Exact Memory Makes LLM Reading a One-Time Cost | cs-PF | **C+ (prefix-reuse record; framing caveat)** | Byte-exact intermediate memoization across requests so repeated reading pays once — the prefix-cache law carried to the request boundary ("a model that answers a second question…" reuses the first's exact bytes). ⚠ The abstract anchors on an external compute-ceiling thesis (Sikka arXiv:2507.07505) — promotional framing; the byte-exact-reuse mechanism is the transferable part. C+ record, mechanism only. |

**C+ records (counted individually):** SPLASH `2609.37626` (attention layout
switching + seamless handoff — the dispatch/lane family, no single layout
serves all loads) · AVSG `2609.37538` (vectorized sparse-gather overlap of
H2D on cross-step-selected-KV overlap) · QuantMLA `2609.36760` (MLA
dual-path quant error model — off-model, DeepSeek family) · WUSH-KV
`2609.38121` + 1-bit-KV `2610.03027` (transform-before-quant family
instances under XOR-Trellis #3) · WakeKV `2610.02713` (head reading
behavior changes mid-generation; reversible residency) · PatchKV
`2609.39329` (training-free weight-space KV compensation — a new axis:
compensate in weights, not the cache) · Page-EntroKV `2610.03135`
(GQA-aligned eviction: shared physical KV buffer forbids divergent
per-head token choices — page-granularity law) · D2K-Bench `2610.03226`
(agentic kernel-gen ecosystem diagnostic — KernelOPT C+ lineage) ·
Self-evolving algorithm-design agents `2609.38757` (population-curated
policy optimization vs in-context stagnation) · GPU-Initiated
Communication `2610.01380` (RDMA-from-GPU-threads cost dissection — MoE
comm lane intel, off our dense league models) · KVTether `2609.39819`
(ReAct agent KV lifecycle — the harness/serving semantic gap) ·
Capability Scaling-Down Laws `2610.02462` (prune/quant/distill capability
laws — doctrine meta-study).

**C records (counted):** spec-dec lane — LongSpark `2609.37029` (fixed-cost
drafter), CAST `2610.00321` (cost-aware trees from block drafters), UBTree
`2609.39972` (unigram+bigram parallel drafting; entropy-diversity), BitNest
`2610.02800`, DRelay `2610.01439`, DEdit `2609.38510`, DScale `2609.37532`
(block-diffusion adaptive verification), Recovering Off-Policy
`2609.38795`, Rank-Aware diffusion trees `2610.02251`, AgSpec `2610.01108`,
Taming Speculative Search `2609.39334`, SEED `2609.36590`, HAWK
`2610.00623` (multimodal) — the spec-dec lane stays closed-negative for us
(717 G3a); all ecosystem records. KV lane — KV² `2610.03198`, SlimKV
`2610.02953`, iS-KV `2610.02815`, AvoKV-E `2610.03007`, ARC-KV
`2609.36835`, EchoPress `2610.00412`, Persistent Context Graphs
`2609.40118`, DeCoPrune `2609.39096` (video). Infra/serving — HB flash
`2609.39131`, SSD sparse KV `2609.36938`, ServeTwin `2610.02732`, Cascadia
`2609.38697`, EdgeAgent `2610.03394`, vSkipper `2609.37062`, VenusRL
`2610.03286`, MoSE `2609.39138`. Theory/meta — Efficiently Approximating
Attention Is Hard `2609.37261`, What Does a Token Cost `2610.02491`,
NinaXander `2609.38261` (frozen cross-architecture composition), RATIO
`2609.39801`, Q-MINO `2610.00738`, reconstruction-loss-scale `2610.00983`.

## Skip classes (counted, ~34 rows)

- Physics/quant-ph/hep/chem quant-lane bulk: 2610.02916, 2610.02656,
  2610.01841, 2610.01771, 2610.01068, 2610.00521, 2610.00569 (hep-ex
  collider tokens — ML-adjacent, off-lane), 2609.40337, 2609.40239,
  2609.40217, 2609.40025, 2609.39951, 2609.39506, 2609.39471, 2609.39452,
  2609.39728, 2609.39612, 2610.03402 (MCU), 2609.39451 (video-codec).
- Vision/multimodal/video: 2610.03218 (VisionMX), 2610.02666 (CHASE-VLA),
  2610.02418 (Whiteout privacy), 2610.01558 (SNN), 2610.00930 (diffusion
  activation), 2610.00613 (VQ geodesics agents), 2610.00569 (dup, listed),
  2609.38795 (dup), 2610.02440 (bandits), 2610.01537 (FedFit), 2610.01139
  (semantic IDs), 2610.03651 (MRVQ vector search), 2610.02652
  (RaBitQ-SSD), 2610.02511 (weather PTQ), 2610.03012/2610.03123/2610.03120/
  2610.02914/2610.01434/2610.02914 (video/VLM class), 2609.37831 (ReCaVSR),
  2609.37001 (stripe attention video), 2609.38978 (PARK video), 2609.39601
  (GroundingPI).
- Speech/audio/edge/agents-off-lane: 2609.37617 (AS²D), 2609.37007 (RVQ
  TTS), 2609.38897 (FFASR), 2609.36860 (IronLLM), 2610.00465 (AIR-LLM RF),
  2610.02670 (LEAP agents), 2610.01892 (multimodal search agents),
  2610.02413 (probe generalization), 2609.39549 (safety honeypot),
  2610.01058 (MOMAT jailbreak), 2610.00610/2610.00664 (region-language
  class), 2610.02491 (counted C above — listed once), 2609.40127
  (functional subspaces), 2609.38652 (AgBench), 2610.02121 (Catscan),
  2609.40141 (LLTA WCET), 2609.39667 (AoI Erlang), 2609.37728 (formal perf
  models), 2609.37399 (MEDEM), 2610.01587 (MPI), 2610.00845 (fusion
  reactor), 2610.01761 (A-GHEST hep streaming), 2609.36525 (medical
  distributed), 2610.02598 (offloaded-weight sparsity), 2609.39131
  (counted C), 2610.03039 (HyperThink), 2609.40362 (multimodal flow),
  2610.01320 (ProtoFlow), 2610.00899 (TOAST robotics), 2610.03039 (dup).

## Yield

0 rule-batches · 1 B+ league-intel HIGH (STEPQuant → Issue-971 dated entry
+ riir-train pointer) · 1 B doctrine (Vosti determinism) · 2 B lane records
(XOR-Trellis transform family; prefix-invariant matmul) · 4 B− · ~13 C+ ·
the rest C / skip-class counted. **Walk #13 DRY for the corpus — the staged
queue is unchanged (MTP/B201 in flight on the 4090; nnethercote R134 next
rust_perf slot; the 10-03 fork pool behind those).** New differential base
for walk #14: everything above + the 09-29→10-06 window IDs; next announce
Tue 10-13 00:00 UTC (~10-12 +07 grading window).
