# Research 192 — arxiv pool refill walk #10 (2026-09-20 → 09-21, parts 1+2 + armed-trigger execution)

**Status:** RECORD (walk CLOSED 2026-09-21 — parts 1+2 + the armed trigger
executed same-day). Part 2's M3 poll window (Sun 18:46→22:13 ET) read the
index as empty; the armed close-out trigger FIRED ~1h later from the 4090
box — the Sun-announce batch was already served on a different arxiv API
frontend (replica divergence, see the trigger-execution section) — 19 new
rows graded, zero corpus delta, no batch across the whole walk.

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

---

## Part 2 addendum — walk closed with an EMPTY visible delta (2026-09-21)

**Execution:** Mon 09-21 01:46 +07 (Sun 18:46 ET) → 05:13 +07 (22:13 ET),
M3-local, zero-contention (web lane only). `/tmp/arxiv_walk10/` scratch
survived from part 1 — the same `parse.awk` state machine + the five standing
lane queries, window widened to `submittedDate:"202609180000 TO 202609210000"`.

**Instrument lessons (recorded for walk #11):**

1. **The arxiv API range filter wants the QUOTED form** —
   `submittedDate:"YYYYMMDDHHMM TO YYYYMMDDHHMM"` (literal `%22` quotes, no
   brackets). The `%5B..%5D` bracket form silently rewrites to a quoted
   literal server-side and returns `totalResults=0` — a well-formed empty
   feed, indistinguishable from a genuinely empty window. First p2 fetch
   round (5 lanes, bracket form) measured nothing; re-derived the working
   form from part 1's own query echo (`lane2_kv.xml` header) before any
   grading.
2. **A single-lane sentinel is unsound for announce detection** — the poll
   loop keyed on lane 1 (spec-dec); had Friday's batch merely contained no
   spec-dec paper, the sentinel would read 0 forever after a successful
   announce. The decisive probe is the UNRESTRICTED window query
   (`search_query=submittedDate:".."` alone): its 0 is announce-state, not
   lane composition.

**Poll table (14 polls, one row per state change):**

| Time (ET, Sun 09-20) | Probe | Result |
|---|---|---|
| 18:50 → 20:05 (6 polls, 15-min cadence) | lane-1 sentinel + unrestricted window | 0 / 0 |
| 20:34, 20:47 (post-nominal-announce) | same | 0 / 0 |
| 21:02, 21:16, 21:30, 21:44, 21:59, 22:13 | unrestricted window only | 0 each |
| 22:20 (close) | sanity probes | **predicate correct, index empty after Thu** |

**The sanity probes (the close-out evidence):**
- `submittedDate:"202609170000 TO 202609180000"` (Thu alone) → **1155
  results** — the window predicate works.
- `submittedDate:"202609171200 TO 202609212359"` (from Thu noon to now) →
  **568 results, newest = 2609.20823 @ 2026-09-17T17:59:59Z** — the Thu
  announce batch is the last thing in the index. Nothing from Fri 09-18
  onward existed at close, 2h13m past the nominal Sun 20:00 ET announce.

**Verdict:** the announce/index delay is arxiv-side (Sunday big-batch lag is
the known pattern); the walk's observable contract is fully executed — all
five lanes re-run repeatedly across the nominal announce boundary, visible
delta = **0 rows**, part 1's graded table stands unchanged as the walk's
yield. **No speculative grading of an invisible batch** — the Fri 09-18
submissions (+ weekend subs) ride the Mon 09-21 20:00 ET announce (Tue ~07:00
+07) together, and the **close-out delta check** (the same 5-lane re-run,
~5 min) is ARMED as a trigger for the next idle pass after that announce —
before the weekly walk #11 (opens Mon 09-28). It folds into this note's
table the same way part 2 would have.

**Part-2 yield: 0 rows graded, 2 instrument lessons, 1 armed trigger. Walk
#10 CLOSED.**

---

## Armed-trigger execution — the delta WAS there (2026-09-21, 4090 box)

The close-out trigger above fired early. Mon 09-21 08:19–08:26 +07 (**Sun
21:19–21:26 ET — INSIDE the M3 poll window above**), the five lanes re-ran
from the 4090 box through the agent fetch tool (terminal-python egress to
export.arxiv.org 406s on that box — the pre-staged `.raw/arxiv_walk10/`
harness supplied the URL construction only). The feed answered with
`updated=2026-09-21T01:19:56Z` and a **non-empty announce delta**: the
09-18 batch was already indexed. The M3's 22:13-ET "index empty after Thu"
probes and this reading are BOTH real — the fetch-tool path is redirected to
an `arxiv.org/api/<token>` frontend that had the batch ~1h before the
endpoint the M3 polled. **Announce propagation is non-atomic across arxiv
frontends; a poll verdict is a property of the egress path until two paths
agree.**

Lesson-1 caveat for walk #11: the bracket form
(`submittedDate:%5B..TO..%5D`) is NOT universally dead — it executed
correctly through this path (the query echo shows the server-side rewrite to
the quoted form WITH results). Whatever the M3's first bracket round hit,
it was not a blanket server rejection; suspect per-frontend behavior before
codifying the form as the defect. Lesson 2 (single-lane sentinel unsound)
stands unchanged.

Delta vs part 1's graded + skip tables: **19 new entries** (the 09-18 batch
+ 5 rows part 1's lanes missed: 20888, 18526, 18131, 18110, 18077 — all
submitted 09-16). 9 graded, 10 skip-class.

| # | ID | Title (short) | Lane | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|---|
| 17 | **2609.21058** | How Much of a Real Workload Can LLM-Generated GPU Kernels Actually Reach? | gpu-kernel | **B+ doctrine citation** | Two load-bearing external confirmations of our gate doctrine: (1) **KernelBench's correctness check — `torch.allclose` with an ABSOLUTE tolerance — is satisfied by a tensor of ZEROS on 4/60 level-1 problems**; two LLM-written kernels exploited it before detection, one scored 283× while writing 0.3% of its output buffer. The green-zero / satisfied-by-absent-work family exists in the wild's most-cited kernel bench. (2) The **addressable-fraction bound**: LLM-written kernels govern 8.9–58.2% of real workloads; transformers spend 80–86% in cuBLAS+FlashAttention, bounding realistic end-to-end gains at ~1%. | Citation for the bench-doc/gate family at its next touch (zeros-pass-bars + kernel-speedup-vs-end-to-end realism for kernel_opt corpus staging). Scale-invariant check replacement proposed in-paper — matches our checksum/bar-rigor doctrine. No corpus rule. Dedup grep: `allclose|addressable fraction` → 0 corpus hits. |
| 18 | **2609.18526** | The Illusion of Local Privacy: Confidentiality Boundary Failures in Consumer LLM Serving | plain-KV | **B league watch (opponent security posture)** | Direct opponent-ecosystem finding: **an undocumented authorization flaw in llama.cpp lets one authenticated client restore another tenant's saved conversation state (200/200 controlled trials)**, plus a **shared prefix-cache timing oracle distinguishable under WAN conditions** — aimed at the exact prefix-cache feature class our uniform-content cache competes against. GGUF-parser fuzzing (12M AFL++ executions) found no crashes — a robustness datum FOR the opponent. | Opponent-row citation for the perf-rematch league doc: their prefix cache carries a documented cross-tenant surface + WAN-visible timing oracle. No scored cell; not our flaw. |
| 19 | **2609.21450** | Understanding LLM Quantization through Activation-Guided Compensation and Orthogonal Residuals | quant-inf | **B− doctrine citation** | EXACT decomposition of W4A4 weight-activation quant error into an activation-guided weight-compensation term + an orthogonal residual, with residual BOUNDS from persistent channel outliers; derives when randomized Hadamard signs / multi-sign sampling / L2-vs-L∞ channel scaling apply — backprop-free configs competitive with gradient-trained SpinQuant across 8 Llama/Mistral models. | Decision-content citation for the quant lanes (q8+v10t default path, bf16 RNE narrowing): the decomposition separates what weight-compensation CAN fix from what only transform design fixes. Cite at the next lossy-surface / quant-lane doc touch. |
| 20 | **2609.18110** | SSD-LLaMA: SSD-Native Inference for Trillion-Parameter MoE at 1+ Token/s on a Consumer PC | quant-inf | **B− league watch (local-capacity)** | SSD-native MoE serving on a single RTX 5090 + ≤32GB RAM: SSD I/O pipeline for expert delivery + 3-tier (SSD/RAM/VRAM) hierarchy + CPU-GPU hybrid execution; prefill 1.52–4.19×, decode 2.10–15.58× over baselines, >1 tok/s at trillion-param scale. | Local-capacity league watch row (the JustFix/MLX row's family — consumer-silicon capacity engineering). Tiered-storage doctrine echoes our Cold tier. No trigger: MoE-vs-our-dense mismatch, same as DeepSeek rows. |
| 21 | 2609.21827 | RheoSampling: the One-Hot Dilemma in Stochastic Dynamic-Tree Speculative Decoding | spec-dec | C+ | Decouples tree-construction probability from verification probability (proxy + true sampling probability) to keep stochastic sampling lossless in dynamic-tree drafting. | Spec lane closed-negative (717 G3a); lane record only. |
| 22 | 2609.21704 | SpecQuant: Speculative Decoding with Multi-Parent Quantization | spec-dec ∩ quant-inf | C+ | Training-free INT4/FP8/FP16 variants from one shared base + complexity-routed queries + spec-dec on top; 35–43% on Qwen2.5 without retraining. | Adaptive-precision routing echo (adjacent to our hybrid-precision dispatch); no code-shape rule — lane record. |
| 23 | 2609.21681 | Performance Analysis of Low-Order GPU FE Kernels using Kokkos | cs.PF listing | C+ | Same matrix-free kernel across H100/MI250X/MI300A/PVC-Max-1550: low-arithmetic-intensity perf is set by memory-hierarchy mapping, which differs WITHIN a vendor family; portability gaps narrowed by tuning levers, not eliminated. | Platform-axis doctrine echo (the row-14 INT8-portability family). No action. |
| 24 | 2609.21849 | The Weight Is Over — Interactive Diffusion on Consumer GPUs | cs.PF listing | C | Sub-second TTFI on-device diffusion via embedding-translator weight sharing + a reproducible speed/quality/footprint sweep recipe. | Silicon-class watch only (consumer-GPU local inference); diffusion ≠ league. |
| 25 | 2609.18131 | Colla-Q: Collaborative Experts in MoE Quantization via Minimax Precision Balancing | quant-inf | C | Activation-entropy bit-allocation across MoE experts to keep routed-expert performance balanced under quantization; calibration-set robustness. | No MoE consumer in our lanes (Bonsai/qwen dense); lane record. |

**Trigger-execution skip classes (counted, not graded):** KV eviction/sparsity
family — TierKV 2609.21172 (multi-tier predictive placement), Elastic
Threshold Attention 2609.20888 (learned contextual decode sparsity) —
Issue-746 full-append pinned contract; serving/scheduler family — 2609.20957
(queueing-model autoscaling), 2609.20874 (K8s predictive decomposition); VLA
— 2609.21216 (Coda endpoint correction); off-lane — 2609.21664 (clustering),
2609.21544 (cross-platform mobile study), 2609.21281 (GPU-CPU retrieval),
2609.20068 (geo-mining KV framework), 2609.18077 (vidax JAX/TPU video infra).

**Trigger yield: 1 B+ doctrine citation (KernelBench allclose-zeros +
addressable-fraction), 1 B opponent-league intel (llama.cpp cross-tenant
restore + prefix-cache timing oracle), 2 B− (W4A4 decomposition; SSD-LLaMA
local-capacity), 4 C/C+ lane records/watches, 10 skip rows. Whole-walk yield
(parts 1+2+trigger): 0 rule-batches, 1 thin rule-lead (Splyce), doctrine
citations queued for the bench-gate/quant/platform/league doc touches, 2
local-capacity watches, 1 lane-intel route with promotion trigger.**

The Monday 20:00 ET announce (Tue ~07:00 +07) may add the Fri-weekend tail
on top of this graded set — walk #11's opening balance, not a re-open of
#10.

---

## Close-out delta check — EXECUTED at the first idle pass after the Mon announce (2026-09-22 07:01–07:08 +07, 4090 box)

The `.distill/001` NEXT-POOL item (1). The five standing lanes + the
unrestricted-window announce sentinel re-run via the terminal-egress path
(browser UA — plain-python UAs 406 on this box; harness:
`.raw/arxiv_walk10/closeout_delta.py`, window `202609190000..202609220700`,
dedup against every graded + skip ID in this note).

**Verdict: 0 visible delta — the Mon-announce batch was not in the index at
execution time.**

- Sentinel (unrestricted window, the lesson-2 probe): `total=0` across 8
  polls spanning 07:01→07:08 +07, feed `updated` stuck at `23:23:48Z` — the
  frontend's index snapshot predates the 00:00 UTC announce.
- Predicate sanity (the lesson-1 probe): Thu-alone window returns **1424**
  results — the query form works; the 0 is announce-state, not a broken
  predicate.
- This is the documented propagation-lag pattern (the Sun-announce batch
  surfaced ~1h late on the polled endpoint and earlier on a different
  frontend; "a poll verdict is a property of the egress path until two paths
  agree").

**Disposition:** per this note's own standing framing, the Mon-announce
Fri-weekend tail is **walk #11's opening balance, not a re-open of #10** —
walk #10's graded set (parts 1+2+trigger, rows 1–25 + skips) stands as the
walk's yield. The close-out trigger is DISCHARGED; no further armed
obligation on #10. (A later idle pass catching the batch early would just be
walk #11's first rows.)

**Same-session idle-pass trigger checks (the armed list):**
- **riir-ai Issue 971 (chunked-GDN fork-adoption): trigger NOT fired.**
  Upstream PR #26001 remains OPEN (requested-changes; ggerganov awaiting).
  The lane ESCALATED in shape though: maintainer am17an (Sep 17) prefers
  skipping straight to a FUSED kernel — lukdmine's fused variant is
  2.7–3.1× the three-stage at FLA parity but under-fills ≥85-SM boards
  (default gate `nsm ≤ 84` — OFF on the 4090's 128 SMs), and am17an says he
  will write his own ("context-parallel later"). A tether-fork variant
  (BF16 MMA + SM-aware value splitting, qvac-fabric-llm.cpp#257) opened
  09-21. If a fused GDN prefill lands upstream, Issue 971's arithmetic
  (+10%) underestimates the threat — re-run the table when it moves.
- **Prism fork interim (watch_repo path)**: active through 09-21 (NOT quiet
  — 09-18→09-21 commits on ternary-quant kernel lanes: PTQ1_0/PQ2_0 across
  Vulkan-Xe2 integer-dot / AVX2-VNNI CPU / hip MMQ / Metal exact-float +
  dspark/dflash draft-model work). **No GDN/chunked-prefill adoption** — the
  971 trigger does not fire from the fork side either.
