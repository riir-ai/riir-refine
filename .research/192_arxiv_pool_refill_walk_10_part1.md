# Research 192 — arxiv pool refill walk #10, part 1 (2026-09-20, window 09-16 → Sat-announce)

**Status:** RECORD (part 1 of 2) — zero corpus delta, no batch. Part 2 = the
09-18 → 09-20 submission announce delta (lands Mon 09-21 ~07:00 +07 with the
Sunday 20:00 ET announce cycle); this note is APPENDED to, not superseded.

**Why split:** the walk's read half is zero-contention and ran inside the
Plan-337 box-saturation window (p335 x025 arm in final eval, load 12-21,
swap 16.1/17.4 GB); the freshest submissions (09-18+) are not yet queryable
— arviz announces at 20:00 ET = 07:00 +07, so everything submitted after
~09-17 is invisible to the API until tonight's cycle. Part 1 captures the
09-16 → 09-17 differential (2 announce batches, ~40 deduped rows across the
five standing lanes — walk #9's window closed 09-16).

## Method

Same five lanes as walk #9 (spec-dec / plain-KV / quant+inference / GPU-kernel
× cs.DC-PF-AR / cs.PF listing), `https://export.arxiv.org` API + entry-keyed
extraction. All lanes healthy, no 429/503. The walk-#9 awk state machine was
re-derived at `/tmp/arxiv_walk10/` (scratch, not committed).

## Graded differential table

| # | ID | Title (short) | Lane | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|---|
| 1 | **2609.19743** | SProbe: Syndrome Decoding for SDC in Quantized Integer GPU Arithmetic | bit-identity / verification | **B− doctrine pair (with .19758)** | INT32 tensor-core accumulators carry no parity/ECC; a transient fault returns a valid-but-wrong integer, no interrupt. Checksum ABFT is BINARY (no element, no magnitude) and **row+column checksums are blind by construction to errors that cancel on both axes**. SProbe = trailing verification kernel over an UNMODIFIED vendor GEMM: randomized Freivalds gate (3 independent draws) + syndrome decoding to localize. | Doctrine citation for the bit-identity/verify-numerics family — the "checksums blind to canceling errors" datum strengthens any future verification-kernel rule (our numeric_drift taps + len_derived identically-zero class are adjacent). NOT a corpus rule: CUDA/tensor-core context, Metal-unbenchable, GOAT unpassable as-is. Dedup grep: `freivalds|syndrome|SDC` → 0 corpus hits (novelty stands). |
| 2 | **2609.19758** | FP-Sketch: Post Hoc Fault Recovery for Half-Precision GPU MatMul | bit-identity / verification | **B− doctrine pair (with .19743)** | Same post-hoc family: sum sketch DETECTS on every call; hashed first-moment sketches + independent recomputation LOCALIZE several corrupted entries **with no false positives by construction**; each fault yields coordinate + magnitude. | Pairs with row 1. The detect-then-localize split with an FP-free-by-construction guarantee is the transferable decision content for any future verify-kernel lane. |
| 3 | **2609.19410** | Splyce: SIMD Vectorization of Sparse Coiteration | SIMD / sparse loops | **C+ rule-lead (thin)** | Sparse-sparse coiteration loops resist vectorization via data-dependent branches; Splyce DECOUPLES coordinate intersection from pointer management via selective predication → branch elimination as a side effect, exposing ILP + hiding memory latency. Dual-path execution model, MLIR. | Lead only: the SHAPE (branch-free sparse intersection via predication/select instead of data-dependent branch) maps to Rust sparse-merge loops, but the paper is compiler-auto-vectorization, not a hand-pattern; needs a real Rust specimen before any batch. Dedup grep: `coiteration|branch-free sparse` → 0 hits; our predicate-gated rules gate WORK, not branches-in-loops — distinct axis. |
| 4 | **2609.18935** | Long-Lived Characters, Local Inference: Incremental Memory Maintenance for Game NPCs | **game-NPC cognition (riir-ai lane)** | **B+ LANE INTEL — watch + citation (gate run 09-20, see below)** | Long-lived locally-deployed quantized Qwen hybrid recurrent-attention NPC characters; revising a few memories invalidates a long reusable prefix; runtime removes superseded KV entries, computes replacements at the TRUE sequence tail, preserves recurrent state + unchanged KV. Findings: independent block composition weakens query-conditioned selection; slot-preserving alternatives repeat a double-subtraction error; attention-proximity does not explain semantics. Contains the raw/latent boundary doctrine VERBATIM: "a fluent but incorrect account of who owns an item... can corrupt the input to otherwise deterministic rules." | **PRIOR-ART GATE RUN 09-20 (same session):** exact vocabulary 0 hits (`memory maintenance\|incremental memory\|prefix invalidation`); nearest substrate = ExperienceGraph (riir-poc + riir-agents shared_context) + the Raven/δ-Mem consolidation cycle (riir-neuron-db) — neither is LM-KV-state maintenance. **VERDICT: WATCH + CITATION, not a lane.** Reasons: (1) npc_brain (Plan 596) is a frozen-brain ternary plasma_path substrate — the paper's hybrid recurrent-attention KV surgery does not map onto it; (2) the serving contract pins full-append KV (Issue 746) — the paper's superseded-entry removal is exactly the out-of-stack family; (3) no open lane consumes it. **Promotion trigger:** an NPC-memory lane opening (npc_brain gaining a memory feature, or a seal quest_sim dialogue-memory extension) — this paper is that lane's first citation + the true-tail-vs-slot-preserving placement finding is its load-bearing doctrine. NOT riir-clippy corpus (runtime cognition, not a heal pattern). |
| 5 | **2609.18005** | A Calibrated Instrument for Measuring How Inference Optimizations Affect Output Quality | league / GOAT methodology | **B− doctrine citation** | Cross-technique quality measurement (quant/early-exit/spec-dec) with a FORMALLY CALIBRATED LLM judge: verify the judge shows no systematic preference between statistically-equivalent outputs (two ordinary runs, same prompts). | Citation-queue for the GOAT G1 / lossy-surface promotion rule (per-family conditional retention) — the judge-calibration protocol is the missing rigor step for cross-system quality claims in our league doc. |
| 6 | **2609.19657** | PrefixBench-H100: Prefix Reuse and TTFT in H100 Serving | league intel | **B− league record** | Reproducible characterization of WHEN prefix reuse materially improves serving: benefits limited by scheduling, cache granularity, concurrency, memory pressure. | Our uniform-content cache is prefix-reuse-shaped (promoted default-on); the boundary conditions (granularity/concurrency limits) are the league-doc citation for any future cache-lane re-gate. |
| 7 | **2609.20807** | Score Centering Stabilizes Off-Policy RL (training-inference mismatch) | numeric drift / riir-train lane | **C+/B− doctrine echo** | RL instability under TIM is primarily DRIFT: a persistent bias between train/inference engines accumulating every step; an additive "score centering" correction cancels it. | Doctrine echo for the riir-train numeric-drift lane (Issue-549 class: execution vs compile-only instrumentation). Riir-train-restricted for execution; citation only. |
| 8 | **2609.19969** | DeepSeek-V4.1-Flash: Pushing the Limits of KV Cache Compression | opponent lane | **C+ league watch** | 552B MoE multimodal, KV-compression-forward architecture. | Oppponent-ecosystem watch row only (league is Bonsai/qwen on our boxes); no scored cell. |
| 9 | **2609.19868** | Zarya: Hybrid AR–Masked-Diffusion LM, Dual-Mode Inference | diffusion-LM / Jev ecosystem | **C+ watch** | Hybrid AR+MDM objectives, flexible slot-structured training, dual-mode inference; KV-reuse for the diffusion mode. | The diffusion-LM ecosystem (Jev, Research 574 / Issue 859) continues to widen — watch row; no action. |
| 10 | 2609.20186 | To Copy or Not to Copy: Controlling Spec-Dec via Intrinsic Model Signals | spec-dec | C+ | Intrinsic signals gate copy-vs-neural drafting; accidental-repetition false triggers analyzed. | Spec lane closed-negative (717 G3a); lane record only. |
| 11 | 2609.17943 | ASPIRE: Async Batched Self-Speculative Decoding | spec-dec | C+ | Non-synchronized per-request draft-verify schedules in a batch. | Same — lane record. |
| 12 | 2609.18662 | Automated Instruction Encoding Synthesis for GPU ISA Compression | kernel_opt-adjacent | C | SASS instruction layout as constrained slot assignment; instruction-supply pressure. | Compiler tooling; no code-shape rule for our corpora. |
| 13 | 2609.20483 | Scaling Fourier-Based Sparse Matrix Analysis on GPUs | cs.PF | C | FFT spectral signatures of sparsity patterns → perf prediction. | Analysis tooling; no rule. |

**Skip classes (counted, not graded):** KV eviction/quant/repair family
(D-Quant 2609.19880, Contiguity-after-edits 2609.17983, MeshKV 2609.19207,
Recency Forcing 2609.19729, On-Demand Attention 2609.20734, Ask-the-Tool
2609.18849, HBFlex 2609.18675) — Issue-746 full-append pinned contract keeps
eviction/residency structurally out-of-stack; training/parallelism (Block
Parallelism BDLM 2609.19242, FL+FSDP 2609.20359, FFL 2609.19972) —
riir-train-restricted; serving/infra schedulers (FairInference 2609.18112,
agent-resource 2609.19947, Life-of-a-Token 2609.19924, Whittle 2609.19792);
off-lane (VLA tokenizers 2609.18259/2609.18487, flood-seg distillation
2609.20441, forecasting 2609.20156, privacy 2609.17995, world model
2609.20034, quantum routing 2609.19561, ad insertion 2609.19899, test-time
scaling 2609.19499, Rosetta LLM-perf-models 2609.19376, diagonal-attention
image gen 2609.19702, DeepSeek-V4.1 read as KV family + opponent row above).

## Listing cross-check (cs.PF recent, the walk-#9 method's second half)

The category LISTING surfaced 3 boundary rows announced Wed 09-16 (submitted ~09-14/15 —
nominally walk #9's window, absent from its graded table; graded here for completeness):

| # | ID | Title (short) | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|
| 14 | **2609.16085** | Is INT8 Portable? Cross-Platform Quantized Inference Measurement | **B doctrine** | Controlled study across 7 hardware classes (ARM/x86 CPUs, dGPU, Jetson iGPU+NVDLA, 2 NPUs) with the ONNX artifact + quantization scales HELD FIXED — only the integer kernel/ISA varies. **The SIGN of the INT8 speedup is set by the CPU's dot-product ISA** (ARM dotprod/SDOT, x86 VNNI: with it up to 2.1×, without it a slowdown). | External confirmation of our platform-axis doctrine — "a platform is part of the claim, exactly as the profile is" (katgpt-rs full_gate / x86_64_execution_matrix / wasm32-simd128 double-gating family). Cite at the next platform-lane doc touch. Also league-methodology adjacent (same artifact, different ISA → different verdict — the pinned-cell rule). |
| 15 | 2609.17179 | Discovering Performance Archetypes: Critical-Path-Aware Regression Detection | C+ | Static complexity metrics explain only **10.4%** of critical-path execution-time variance across 6 real C/C++ apps; multi-signal (static + dynamic traces + kernel resource data) required. | Doctrine echo of our static-vs-execution rule (arm_reach's "arm QUALITY is not statically decidable"; the check_validation_gate limit). No action. |
| 16 | **2609.17475** | JustFit: 200K-Token LLM Serving on a 24 GiB Laptop (MLX) | **B− league watch** | MLX runtime (KVExec compressed-KV execution + PhaseSwap component residency + StateTrans): 196,608-in/16,384-out tokens on **Qwen3.8-27B MXFP4 on a 24 GiB M4 Pro**, 6.93× the mlx-vlm baseline context. | Our exact model + silicon class on a different stack — local-capacity league watch row (same-model/same-silicon reference, the mlx-serve class). Capacity engineering, not a scored lane; no trigger. |

Other listing rows were duplicates of the graded table or skip classes (queueing,
FPGA, thermal/serving schedulers, cross-list off-lane).

## Yield summary (part 1)

**0 rule-batch** (nothing batch-justifying — consistent with sources thin) ·
**1 thin rule-lead** (Splyce predication shape, needs a Rust specimen) ·
**2 doctrine pairs/citations queued** (SDC post-hoc verification pair →
bit-identity family; judge-calibration → GOAT/league; PrefixBench → cache
lane; score-centering → riir-train drift; INT8-portability → platform-axis
doctrine) · **1 high-value lane-intel route** (game-NPC memory maintenance →
watch+citation with promotion trigger, gate run) · 3 watch rows (DeepSeek-
V4.1-Flash, Zarya/diffusion-LM, JustFit/MLX local-capacity).

Part 2 (Mon 09-21): re-run the five lanes with the window extended through
09-20, grade the delta, fold into this table, close the walk.
