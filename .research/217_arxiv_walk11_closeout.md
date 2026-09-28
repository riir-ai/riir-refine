# Research 217 — arxiv pool refill walk #11 close-out (2026-09-28)

**Status:** RECORD — walk executed 2026-09-28 08:05–08:15 +07 (M3, the due-today
clock; closes Research 201's OPEN balance). **Zero corpus delta: 0 rule-batches**
— 5 substantive lane records + 1 doctrine echo + counted skip classes. Differential
base 2609.23388 (2026-09-20T06:11Z) → window 09-20 06:11Z → 09-29 00:00Z, five
standing lanes (spec-dec / plain-KV / quant+inference / GPU-kernel × cs.DC-PF-AR /
cs.PF listing), dedup against Research 192 + 201 graded/skip IDs + the five
early-catches (23900 Tree-Scan / 24797 KDA / 25802 LEMA / 26590 GTR / 27470 DeltaS)
+ 2609.20100 TWT (user-graded → riir-infer 022).

## Lane reads (per-lane totals → fresh after dedup)

| Lane | total | fresh | note |
|---|---|---|---|
| spec-dec | 9 | 8 | closed-negative lane; records only |
| plain-KV | 31 | 30 | the 746 out-of-stack class dominates |
| quant+inference | 27 | 27 | 2 substantive rows (DQ, softmax reparam) |
| gpu-kernel | 4 | 4 | KernelOPT + KREX substantive |
| cs.PF | 20 | 20 | bulk off-lane |

78 fresh rows total (union across lanes); ~70 graded/skip-classed below, the rest
off-lane bulk counted ungraded.

## Graded rows

| # | ID | Title (short) | Lane | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|---|
| 1 | **2609.26333** | Disaggregated Quantization: Specializing LLM Prefill and Decode | quant+inference | **B+ league-intel HIGH** | Prefill wants COMPUTE-native formats (NVFP4), decode wants COMPACT weights — a two-format two-weight serving posture; "removing activation quantization specifically on decode improves accuracy on decode-heavy tasks without cost"; trained NVFP4 prefiller over released **Qwen3.8-27B GGUF decoders** (our league model family, accuracy axis) improves 1-bit MMLU-Pro +32.5 pts without touching the decode checkpoint. | Lane record. The two-format phase-keyed serving posture is the league watch's format-evolution axis (same class as the 971 Hadamard-v2 format watch): if the llama.cpp fork ever ships a prefill-format≠decode-format serving lane, that's a scored-surface change. The accuracy half is training-track (riir-train routing). No corpus rule (serving recipe, not a kernel code-shape). |
| 2 | **2609.31291** | Softmax Reparameterization for Output-Head Quantization | quant+inference | **B (B181-family kinship)** | W′ = W − α·W_mean (vocab-mean row, rank-one weight change) preserves softmax EXACTLY (per-position constant logit shift ⇒ exp-invariance), so the head re-centers for free before quantization; α selected by validation KL per quantizer (RTN/AW-MSE/GPTQ); **rank-one correction handles nonlinear logit paths such as soft-capping** (Gemma-2's softcap — our riir-train gemma-2 lane); W4 KL 0.936→0.256 on Phi-4-mini. | The exact-invariance-licensed writer-side transform is the B181 `signed-activation-codes-drop-dequant-offset-term` mechanism class (offset family, writer-side member #2). Rider/annotation candidate at the next quant-family touchpoint — not a batch trigger alone (quant-time transform, not a kernel loop shape). |
| 3 | **2609.30059** | KernelOPT: Dispatch-Aware Agentic Search for GPU Kernel Optimization | gpu-kernel | **C+ lane record** | Multi-agent LLM system optimizing GENERATED Triton sub-kernels inside compiled models (preserves vendor cuBLAS/cuDNN calls), 4-gate verification cascade: static validation → multi-seed correctness → model-level float64-fallback → [end-to-end]. | Ecosystem validation of our kernel_opt domain's existence; the model-level float64-fallback verify arm is a measurement-honesty pattern adjacent to our G1 gates. Agent architecture, not a code-shape — no corpus rule by construction. |
| 4 | **2609.30884** | CacheReforge: Bounded Recovery for Stale KV Caches under Evolving Adapters | plain-KV | **B+ lane record (hot-swap adjacency)** | Adapter (LoRA) updates make cached KV states stale; layerwise mixed-version cache objects + per-layer adapter anchors + calibrated sensitivity + executable restart boundaries select direct-reuse vs bounded-recompute vs full-refill. | The cache-VALIDITY-under-hot-swap problem is the riir-instinct A2 monotonic-hot-swap lane's shadow (currently no cross-request KV reuse in our serving lanes, so no live consumer). Standing record; actionable if a prefix-cache + artifact-hot-swap serving lane materializes. NOT the 746 eviction class — validity, not capacity. |
| 5 | **2609.30057** | KREX: Concurrent Kernel Benchmarking on Shared GPUs via Region-Granular Exclusivity | gpu-kernel | **B− (measurement-posture datum)** | Agents mark timing-critical regions; the runtime enforces GPU exclusivity ONLY inside marked regions and allows concurrency outside — process-granular reservation wastes utilization that region-granular exclusivity recovers, fidelity preserved. | Refines our GPU-exclusivity discipline (Bench-649 / the bench-idle-gate posture / B182's idle-residency rule): the exclusivity requirement is a property of TIMING REGIONS, not of the whole gate run. Our gates could legally overlap non-timing phases with other work. Posture note only — we don't build multi-agent GPU-sharing runtimes. |
| 6 | **2609.24698** | Adapting Tree-Structured Speculative Decoding to DeepSeek-V4 | spec-dec | **C lane record (971 orbit)** | Branch-aware causal verification + temporary state isolation + accepted-path state refresh keep compressed-attention (CSA/HCA) verify consistent across tree branches. | The tree-spec ecosystem keeps widening (TreeWY → Tree-Scan → DeepSeek-V4); none of it touches our models or the scored surface. Spec lane closed-negative (717 G3a) stands. |
| 7 | **2609.26828** | Bridging LLM Serving and CXL-SSDs with Chunk-Aware KV Cache Management | plain-KV | **C lane record (Strata LEAD-B adjacency)** | Stock CXL-SSD ≈ 3× slower than local DRAM and no faster than NVMe for KV prefix caching; generic prefetching little benefit; specialized device (LM-CXD) recovers some. | A NEGATIVE storage-tier datum for the Strata distill's LEAD-B (`ssd-resident-table-row-granular-read`, already graded B−) — consumer hardware cannot make SSD-resident tables fast; the specialized-device caveat is the honest boundary. KV hierarchy = the 746 out-of-stack class regardless. |

## Doctrine echo

- **2609.30474** Mentored Decoding (spec-dec): formally proves lossy speculative
  decoding can beat the TARGET on quality (boosting theory, f-divergences, the
  total-variation case geometric). **C− doctrine echo** — the theoretical companion
  to the Issue-750-T3 lossy-surface law (lossy ≠ wrong; lossy is GATEABLE per-family).
  Spec lane stays closed-negative; the echo sharpens WHY the law exists, no action.

## Skip classes (counted, not graded)

- **KV eviction/compression family (746 out-of-stack)** — 2609.30738 (diversity-aware
  eviction scoring), 2609.27981 (risk-controlled eviction), 2609.26300 (CompKV),
  2609.26368 (HySparse2), 2609.24298 (KV-COBRA bit-rank), 2609.29913 (MILO low-rank),
  2609.26425 (QuantWM 2-bit KV), 2609.31224 (acoustic KV), 2609.28029 (KV spectral
  decomposition — analysis), 2609.25537 (answer-aligned memory), 2609.30854 (KV =
  the new memory wall — position paper, cross-lane), 2609.23816 (SPLASH HBF+HBM —
  hardware off-lane), 2609.27746 (KV working-set capacity planning), 2609.24991
  (KV cost attribution), 2609.24635 (KV-cache interpretation — cs.CL analysis).
- **Spec-dec records (closed-negative)** — 2609.29364 (NebulaSD many-for-many),
  2609.27396 (parallel drafter ∥ parallel spec), 2609.26638 (diffusion drafts,
  AR verifies — OCR), 2609.24847 (SPECTRA tiled-arch spec), 2609.24197 (H-Spec
  drafter-side-KV-free), 2609.24150 (acceptance-aware draft training — training
  track).
- **MoE (no in-stack consumer — the standing reason)** — 2609.29032 (flash-backed
  MoE on iPhone; the cache-policy/workload-interaction datum noted, model class
  absent).
- **Serving infra (C sightings)** — 2609.29160 (cross-model autoscaling),
  2609.26763 (SARA SLO disagg), 2609.24639 (power-aware prefill-decode disagg —
  the DQ-adjacent infra half), 2609.30379 (NestedTensor ragged batching),
  2609.23790 (agent memory-injection cost attribution).
- **Tooling/hardware sightings** — 2609.28769 (Xtrace binary-level GPU probe
  insertion — post-compile tracing via hazard tables; profiling-tool lane),
  2609.24270 (die-scaling scheduling asymmetry — same-box league cells cancel),
  2609.28724 (Canonical Parallel Form compiler substrate), 2609.26644 (TPU
  near-threshold clocking).
- **Off-lane bulk** — speech/audio (30784/30785/29866/25379/27378), vision/VLA
  (30245/29382/24526/24433/26474/25881/28262/29999-adjacent), FPGA/quantum/hardware
  (31177/28144/30862/29406/26824/26644), edge/IoT (29084/27202), recsys/IR
  (29180/23718), biology (26474), misc infra (30468/30448/30580/26616/29032-MoE).

## Yield

0 rule-batches · 1 B+ league-intel (DQ) · 1 B kinship record (softmax reparam) ·
1 B+ hot-swap-adjacent record (CacheReforge) · 1 B− posture datum (KREX) ·
1 C+ ecosystem record (KernelOPT) · 1 C orbit (tree-spec) · 1 C storage datum ·
1 C− doctrine echo · ~65 skip-class rows counted. Walk #11 DRY for the corpus —
the second consecutive zero-delta walk; the staged queue (ggml-cpu tiled mul_mat
lead + P40/P41 pack) is unchanged by this walk.

## Method notes

- Query harness: `/tmp/arxiv_walk11/walk11.py` (scratch, not committed), 5 lanes
  × `submittedDate:"202609200611 TO 202609290000"`, 3.5 s inter-query spacing,
  https export.arxiv.org (the M3 path worked first-try — the two-path
  non-atomicity caveat from 201 stands for future walks).
- Window end extended past now (09-29 00:00Z) — the freshest row returned is
  09-25 14:43Z; the Mon-announce batch (Fri-cutoff → Sun submissions, announce
  Mon 20:00 ET = Mon 09-28 08:00–09:00 +07) had NOT landed at read time (UTC
  01:11 = 08:11 +07, announce propagation in flight). **The next idle pass
  should re-probe for the announce tail** (freshest-id check; if 09-26/09-27
  rows appear, they are walk #12's opening balance per the Research-201
  catch-early precedent — no re-walk owed this session).
