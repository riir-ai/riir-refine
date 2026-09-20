# Research 170 — arxiv pool refill walk #9 (2026-09-16, backfill window 09-07 → 09-16)

**Status:** RECORD — zero corpus delta, no batch. Yield: **2 rule-LEADS** (queued for a
future Metal-shape/kernel pack; neither batch-justifying alone) + **4 doctrine
citations** (queued for the next corpus/doc touch — `src/` contended this cycle by
the TSD lane, katgpt-rs AGENTS.md contended by the bf16/meld/audit sibling agents)
+ **2 league-intel rows**. Sources trending thin over a 9-day window — consistent
with the 09-13 scout's SOURCES DRY verdict.

**Update 2026-09-20:** citation queue drained — the riir-clippy corpus half landed
2026-09-16 (same-day); the katgpt-rs AGENTS.md lossy-surface half landed
2026-09-20 (katgpt-rs `87854232`, the sibling contention cleared). 2609.11687
rides this note as rotation-tax echo #4 (no separate surface).

## Method + window mechanics (honest notes)

- Same five standing lanes as walk #7 (spec-dec unrestricted / plain-KV /
  quant+inference + the block-scaling second sweep / GPU-kernel × cs.DC-PF-AR /
  cs.PF listing), run through `curl` + stdlib XML parse.
- **New instrument lessons this walk:** (1) the export API answers `http://` with a
  bare **301** (size 0) — use `https://export.arxiv.org` directly or `-L`
  (walk #7 recorded the range-filter + MCP-fetch defects; this is the redirect
  leg of the same story); (2) a `paste - - -` extraction MISALIGNS on the feed
  header's own `<id>/<title>/<updated>` lines — the robust form is a small awk
  state machine keyed on `<entry>`/`</entry>` (kept at `/tmp/arxiv_walk9/`, not
  committed; ~6 lines, re-derivable). API healthy all lanes, no 429/503.
- **Window:** walk #8 was consumed by Batch 124 (2026-09-07); nothing walked
  since. This walk's differential = submissions 2026-09-07 → 2026-09-16 (~7
  announcement batches). ~90 deduped rows across lanes; ~18 graded in the table
  below, the rest counted skips by class. NOTE: the walk ran EARLY in-loop (the
  snapshot queued it for the "sibling lane" on 09-14 and that lane never
  landed it) — taken as this cycle's item-12 unit because every other queue
  item was contended/gated/dry.

## Graded differential table (notable rows)

| # | ID | Title (short) | Lane | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|---|
| 1 | **2609.11356** | Taming Bitwise Behavior in GPU Kernels with Tensor Core | bit-identity / tensor-core | **A− doctrine + rule-lead** | Reduction order, partial-sum precision, FMA placement, rounding = the bit-divergence axes; **tile shape chosen for performance determines the arithmetic — breaking batch invariance**; fixed order costs up to 20%; an autotuner cannot identify which configs are bitwise equivalent (black-box reconstruction + compiler enforcement + static verification offered). | QUEUED citation for the bit-identity doctrine family (`reduction-restructure-needs-source-owned-comparator`, `proven-shape-static-sibling-identical-results-fallback`) + the batch-invariance axis (serving-level, not pinned in our corpus). The autotuner-blindness datum is a fresh decision content for any future tile-shape rule. CUDA/tensor-core — Metal unbenchable, GOAT unpassable as-is. |
| 2 | **2609.09721** | EFQ-Softmax: Exp-Free Quantization for Softmax | kernel math / low-bit attention | **B rule-lead** | Exp-then-quantize path = high-precision probability producer feeding a low-bit matrix consumer; EFQ maps shifted scores DIRECTLY to block-scaled E2M1 codes (exponent-only scale from the local max + one affine rule), used consistently in both the P·V numerator and the P·1 denominator updates; FA row-max/rescale/normalization unchanged. **−40.33% vector-stage latency** (16K–128K, A5 vector unit); Qwen3-8B 7-task mean 0.6749→0.6773 vs MXFP4. | Rule-LEAD: `direct-low-bit-generation-replaces-exp-quantize-roundtrip` — the producer/consumer precision-mismatch shape. **Dedup needed at mine time** vs `operand-precision-gated-by-consumer-invariance` (B110/B118 N — that rule chooses WHICH precision; EFQ skips the high-precision intermediate entirely). Our PV path is f16/f32 today (Q2_0 dense GGUF) — not live in our serving path; a future FP8/FP4 lane or a CubeCL-shape pack consumes it. |
| 3 | **2609.12923** | Dissecting GPU Utilization for LLM Inference on Hopper | measurement / league | **B− doctrine (corroboration)** | Single SM-utilization % collapses several mechanisms; decode dense GEMMs are small-row MMAs — on Hopper the bf16 GMMA path executes **fixed 64-row matrix fragments, so small-batch decode fills only a fraction of each fragment with real token rows**. | Independent hardware-side confirmation of the **underfilled-grid** concept recorded in the MTPLX PR391 verdict (riir-clippy `09657f66`, landed earlier today) — the MTPLX v1 `(1024,S,1)` cell losing 13.2 vs 67.8 tok/s IS this class on Metal. Strengthens the future grid-fill/underfill rule's external evidence. Cite with it. |
| 4 | **2609.15504** | How Lossless Is Lossless Speculative Decoding? (Orthrus repro) | spec-dec / numerics | **B− doctrine citation** | Independently reproduced Orthrus: under BF16, "lossless" spec-dec matches the AR trajectory exactly in only **43–45%** of 1,190 prompts; exact match probability tracks response-conditional perplexity; **FP32 restores 100% exact matching**; downstream lm-eval shows no systematic degradation despite divergence. | External confirmation that "lossless" is PRECISION-CONDITIONAL — pairs with our verify-numerics law (chunked prefill reorders f32; verify runs serial numerics; Research-09-13 watch row) and the bit-identity doctrine. Citation-queue for the next corpus/doc touch. Our spec lane stays closed-negative (717 G3a FAIL); no trigger fires. |
| 5 | **2609.11687** | Structured Transforms for Low-Overhead Quantization (Kashin-DCT) | quant format / rotation axis | **B− doctrine citation** | Kashin decomposition with a **sign-randomized DCT replacing the dense random orthogonal matrix** (O(N²)→O(N log N)); greedy alternating updates guarantee the four-peak distribution; **closed-form cluster-center init** removes multi-restart k-means; numerically STABLE where QuIP variants hit 4-digit PPL or NaN (LDL back-substitution); two 2-bit factor codes per channel at inference. | **Rotation-tax axis echo #4** (walk #6 LightRot/GyRot; Bench-716 Hadamard-sink refutation; UE5M3 scale-format): the transform FORMAT is where outlier machinery is bought or refunded, and structured transforms now measured cheaper than random ones at equal stability. PTQ pipeline = riir-train-adjacent (routing-restricted); citation only. |
| 6 | **2609.14864** | GGUF-Metadata Prediction of Single-Sequence llama.cpp Throughput | league intel | **B− league record** | Roofline-shaped predictors from GGUF metadata + quant-specific scale factors; 318 measurements, 53 host-file configs, 2×M4 Max + RTX 5080; **active-parameter decode model 13–36% MAPE vs 49–56% charging total params**; "a low-bit model ladder changes ordering across runtime stacks"; fitted efficiencies NOT universal across hosts/runtimes. | League-methodology confirmation: active-param (not total) carries decode throughput (MoE economics), and cross-stack prediction is host+runtime-specific — the same-box same-model pinned-cell rule our league already runs. No scored-lane impact (no same-box cells on our models). |
| 7 | **2609.15030** | Validating Hybrid-State Cache Recovery for GLM-5.3-Flash (vLLM+LMCache) | opponent lane / correctness | **C+ lane record** | "A complete-hit recovery mismatch restored state for the full prompt while **the scheduler credited one fewer token**" — state-restore vs token-accounting desync; fixed by strict-prefix lookup; output equality re-established via shared-computation corrections + matched checkpoint scheduling + fixed per-rank kernel configs; CPU reload cut TTFT 46–64% cold. | The alignment CLASS (restored state and counter credit derived from the same prefix) is the transferable nugget — echoes our accounting-alignment instincts in the sync/commit paths. GLM-5.3-Flash = the dao advisory model + an opponent lane; intel value moderate. No rule (vLLM/LMCache-specific). |
| 8 | 2609.10515 | PASCAL: Phase-Aware Shared-Cache Model for Parallel Scans | cs.PF / cache model | C+ | Analytic shared-cache miss model for "parallel scan" access patterns (attention Q-tiles sharing K/V blocks, GEMM panels); progress-divergence-aware, no-trace prediction, policy-independent lower bound; MAPE 13.84% vs 44.79/54.16 baselines on GB10. | Model/intel only. **Vocabulary collision note for future dedup greps:** their "parallel scan" = shared-cache access pattern, NOT the prefix-scan recurrence our `first-order-affine-recurrence-associative-scan` family means — a `parallel.scan` grep hits both; disambiguate at mine time. |
| 9 | 2609.10937 | The Art of Closed-Formula Defaults: Search-Free Code Generation for Tensor Operators | codegen doctrine | C+/B− | Application-first hierarchical codegen from operator spec → GPU instructions; "clearly defined computational model makes the optimization problem tractable"; closed-form defaults over search. | Doctrine echo of modelless-first (closed-form beats search when the computational model is clear). Thin abstract; not fetched deeper this walk — 12-page cs.MS paper, re-read only if a codegen-shape batch opens. |
| 10 | 2609.15021 | Shared KV Caching for Replicated 27B Inference: Correctness Failures | serving correctness | C+ | Packed-page patch carried a raw-pointer fallback omitting the **CUDA-stream dependency**; controlled byte tests FAIL under imposed delay and pass when restored; mixed allocator = working path; cross-replica TTFT 31.7s→0.6s @128K. | The imposed-delay byte-test methodology echoes our chaos/torn-read discipline; the stream-dependency class is vLLM-specific. Our KV is full-length append-only (Issue-746 pinned contract) — cross-replica sharing out-of-stack. |
| 11 | 2609.14717 | Carryover Drafting: Recycling Rejected States for Speculative Decoding | spec-dec | C+/B− | Rejected target hidden states recycled as temporary KV the drafter can attend to (one learned embedding, bounded by one proposal block); +6.5–14.7% acceptance length, +7.9–14.4% e2e vLLM; needs parallel draft-verify-draft TRAINING. | DFlash-adjacent (our 4090 prefill route rode the DFlash+938 wave) but the mechanism is model-based (learned embedding + drafter training) → riir-train routing-restricted, and our spec-dec verify lane is closed-negative. Lane record; no trigger fires. |
| 12 | 2609.13544 | PEAT: Pseudo-Error Assessment for GPU Kernel Validation | instrument | C+ | Pseudo-error (fault-injection-shaped) assessment for GPU kernel validation in DNN training. | Instrument-adjacent to our kernel_verify STUB (CubeCL compile-only). Parked as a pointer for any future validator-upgrade lane; no rule. |
| 13 | 2609.12208 | Vortex: Bridging Extreme Compression and Efficient LLM Inference | quant lane | UNREAD — honesty row | Abstract not fetched this walk; title suggests extreme-compression (ternary-adjacent?) quant. | Re-check only if a ternary/quant-format batch opens; routing for execution would be riir-train-restricted regardless. |
| 14 | 2609.15627 | DeepSeek-V4-Flash on AMD gfx90a: Correctness Recovery and Inference Perf Engineering | league-adjacent | C | AMD MI200-class correctness recovery + perf engineering for a Flash-class model. | Not our silicon; correctness-recovery engineering echo only. |
| 15 | 2609.15807 | Predictive LLVM IR Performance Ranking for Transfer-Learning Autotuning | compiler | C | IR-feature performance predictors for autotuning. | Predictor-class tooling; no code-shape rule for our corpora. |

**Skip classes (counted, not graded):** KV eviction/compression family (AgentKV
2609.14872, Jacap 2609.08131, MetaKV 2609.07966, FlexComp 2609.11192, Grouped-Value
2609.13285, KVShareArena 2609.10266, prefix-sharing-sorting 2609.13692, py-kvcache
2609.11744, KV-concat-vs-recompute 2609.09768) — Issue-746 full-append pinned
contract makes eviction/residency structurally out-of-stack (walk-#7 row-2
boundary restated); hardware/PIM/CIM (PATTON 2609.11392, AMEND 2609.09823,
NOVA-CIM 2609.07059, Ozaki-2.5 2609.09095); drafter-training (Osprey 2609.09338,
draft-co-training 2609.07108) — riir-train-restricted; serving/infra (CXL
2609.10790, HBM repartition 2609.13537, AI gateway 2609.06940, thermal pair
2609.15230/2609.12449, ETCInfer-class schedulers); off-lane hits (multimodal
embeddings 2609.15296, 6G 2609.10714, spec-RAG 2609.10046, backdoor detection
2609.11799 — not our sec shape, mobile-MoE 2609.14643, PELM/DVFS 2609.09662
device-tier, music/adapters 2609.12774, biometric-KV 2609.08566, block-diffusion
2609.11998, tabular-quant 2609.13031, harm-mitigation 2609.13624, OLTP
2609.12597, RoofLang 2609.12551, RISC-V SpMV 2609.11352, InstantMimic 2609.09821,
OneLA 2609.12399, AttnFuse DSL 2609.13612, Dream-RSI 2609.14858, Elastoformer
2609.10018, depth-pruning 2609.09883, edge-continuum 2609.08307, VLA-latency
2609.14146, liver-PEFT 2609.14106, HoliBench 2609.12412, expert-pruning-WMT
2609.12310, power-control 2609.11133, ZipCodec 2609.11642, UniStream 2609.09866,
DiffLUT 2609.09254, ReMoMask 2609.08365, Marigold 2609.08084, MoE-overlap
2609.07536, quantum pair 2609.12980/2609.12146, Argus 2609.12299, Turkish-MMLU
2609.15467, ZGC 2609.15558, object-detect-latency 2609.12920, atmospheric-BNN
2609.12815, robot-factories 2609.12075, metastable 2609.14712, LEO-NTN
2609.14735, KAN-HPC 2609.07740, TASTE 2609.07444, digital-shadow 2609.07275,
T-LoopFormer 2609.15160 — model-arch, recorded as a possible katgpt-rs research
lane, not a mining batch, self-orchestrating 2609.14850, SpliTEE 2609.15039).

## Shortlist (coordinator decisions)

- **No batch.** Rows 1–2 are leads (each yield-1, Metal/GOAT-constrained); rows
  3–5 are citations; rows 6–7 league records. The queue's standing input
  trigger stays EMPTY — next batch still needs either post-mining accumulation
  or a stronger source.
- **Citation queue (execute at the next corpus/doc touch, NOT this cycle —
  `src/` contended by the TSD lane; katgpt-rs AGENTS.md contended by sibling
  agents):**
  - 2609.11356 → bit-identity doctrine family rider (batch-invariance axis +
    autotuner bit-equivalence blindness; the ≤20% fixed-order cost datum).
  - 2609.15504 → verify-numerics/precision-conditional-losslessness external
    (BF16 43–45% vs FP32 100% exact-match).
  - 2609.11687 → rotation-tax axis echo #4 (Kashin-DCT structured transforms).
  - 2609.12923 → cite alongside the MTPLX underfilled-grid verdict block
    (Hopper 64-row GMMA fragment fill = the hardware mechanism).
- **Leads carried in the snapshot:** EFQ direct-low-bit-generation (dedup vs
  operand-precision-gated-by-consumer-invariance at mine time) + the
  underfill/grid-fill axis (now dual-sourced: MTPLX Metal negative cell +
  Hopper fragment-fill measurement).
- **Queue-snapshot updates:** walk #9 DONE entry (this file), next window
  **Mon 09-21 ~07:00 +07** (Sun 09-20 20:00 ET announcement).

## Numbering note

`.research/.highwater` read **169** at walk time; folder re-scanned (no 170 on
disk, no sibling WIP under `.research/`/`.distill/`). This walk takes **170**
and writes highwater = 170.
