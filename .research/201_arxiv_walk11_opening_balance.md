# Research 201 — arxiv pool refill walk #11: opening balance (the Mon-announce tail, caught 2026-09-22)

**Status:** OPEN — opening balance BANKED 2026-09-22 08:29 +07 (M3); walk
proper opens Mon 09-28 with the ordinary five-lane window; differential
base = **2609.23388** (submitted 2026-09-20T06:11Z). Zero corpus delta at
open: 0 rule-batches, 3 lane records, 1 league-intel row, 1 riir-train
doctrine echo.

## Provenance — why the opening balance was caught early

Research 192's close-out (4090 idle pass 07:01–07:08 +07 today) discharged
the walk-#10 trigger at **0 visible delta** and pre-declared the
disposition: *"A later idle pass catching the batch early would just be
walk #11's first rows."* This note is exactly that catch.

**Announce timeline (the two-path protocol, both directions now measured):**

| Time (+07 / ET) | Path | Reading |
|---|---|---|
| 07:58 / 20:58 Mon | export.arxiv.org (M3) | 000000-window total=928, newest 2609.22086 @ 09-18T17:59Z — the Fri 14:00 ET cutoff boundary, exactly pre-announce |
| 07:58 / 20:58 Mon | fetch tool → arxiv.org/api | 181800-window total=**0** — both paths agree, batch not indexed ~1h past nominal |
| 08:29 / 21:29 Mon | export.arxiv.org (M3) | 181800-window total=**969** — batch landed between the two readings |

The batch is CONTIGUOUS: IDs 2609.22420 → 2609.23388 (= 969 exactly),
first entry submitted 2026-09-18T18:00Z — **the Fri 14:00 ET cutoff
boundary to the minute** — through Sun 06:11Z. The Mon 20:00 ET announce
covers Fri-cutoff → Sun submissions as predicted; export.arxiv.org
propagated ~90 min after nominal (the Sun batch had gone the other way:
fetch-tool path first, export ~1h20m later). Announce propagation is
confirmed NON-ATOMIC in both directions across frontends; neither path is
canonical.

Method: the same five standing lanes (spec-dec / plain-KV / quant+inference
/ GPU-kernel × cs.DC-PF-AR / cs.PF listing), window
`submittedDate:"202609181800 TO 202609221300"`, dedup against every graded
+ skip ID in Research 192. Harness: `/tmp/arxiv_walk10_closeout/` (scratch,
not committed). Off-lane bulk ~955 rows (astro-ph / math / quant-ph / hep /
cs.HC-class) — counted, ungraded, categories as returned.

## Graded rows

| # | ID | Title (short) | Lane | Grade | Mechanism / relevance | Action |
|---|---|---|---|---|---|---|
| 1 | **2609.21858** | Watermarkable Multi-Draft Speculative Sampling via Poisson Processes | spec-dec | **C+ lane record** | Multi-draft spec sampling from Poisson processes; embeds an UNBIASED watermark with ZERO speculative-acceptance degradation; drafter-invariance via exact list-coupling-without-communication. The provenance×efficiency co-design frontier — the first sampled shape where watermarking stops costing acceptance. | Sunday-batch lane miss (the 07:01 close-out's lane set keyed "speculative decoding"; this title carries "Speculative Sampling" — the OR-arm lesson one more time). Spec lane stays closed-negative (717 G3a); record + doctrine note for any spec revival: the acceptance-cost argument against watermarking is no longer structural. |
| 2 | **2609.23033** | WaveFront Decoding: Parallelized Self-Speculative Decoding for Looped Language Models | spec-dec | **C+ lane record** | Self-speculation parallelized ACROSS the loop iterations of looped/recurrent-depth LMs — the draft is the model's own shallower loop count, verified in wavefront. | Lane record (spec closed-negative). Adjacency to our ternary depth lanes is conceptual only — looped-LM depth scheduling, not a Rust code-shape. |
| 3 | **2609.22471** | Efficient Mixture-of-Experts with Speculative Decoding via Expert Coactivation | spec-dec ∩ MoE | **C lane record** | Expert-coactivation statistics drive which experts speculatively activate; spec-dec over the MoE routing path. | No MoE consumer in-stack (Bonsai/qwen dense — the Colla-Q row's standing reason). Record only. |
| 4 | **2609.23130** | From Inference Engine to Inference Control Plane: Connecting vLLM, llm-d, and the Evolution of Efficient Distributed Inference | serving infra | **C league-intel** | Position/survey on the vLLM→llm-d control-plane evolution (disagg serving, scheduling planes). | Opponent-ecosystem context for the perf-rematch league doc's serving-infra section; no scored cell, no trigger. |
| 5 | **2609.22870** | Towards Full Pipeline FP8 Reinforcement Learning for LLMs | training track | **C− doctrine echo (riir-train-restricted)** | End-to-end FP8 RL training pipeline (rollout, reward, backward all FP8) — the execution-numeric-instrumentation question at pipeline scale. | riir-train-restricted per the routing rule; citation-only echo of the Issue-549 execution-vs-compile instrument class. No action on this box. |

## Skip classes (counted, not graded)

- **KV eviction family** — ValueDiff 2609.23314 (value-geometric eviction
  for sink-suppressed attention): the Issue-746 full-append pinned contract
  keeps eviction structurally out-of-stack (the TierKV precedent, third
  instance this arc).
- **Off-lane acceleration** — SparkDiffusion 2609.23153 (sparsity for
  diffusion, ≤265× single-GPU: diffusion ≠ league's text models, no
  local-capacity cell); FPGA plasma prediction 2609.23141 (FPGA, scientific
  computing); ZYT-World 2609.21712 (Sunday-batch miss, driving world model,
  quantization incidental — L3 query matched its abstract); LFP-snapshot
  dequantization 2609.22658 (neuroscience); FORM codegen 2609.23367
  (training-track symbolic codegen); Presage 2609.22636 (agent search).
- **Serving/energy** — Measured Joules 2609.23085 (learned routing for
  energy-efficient serving — scheduler family; no scored lane).
- **cs.PF record** — io_uring in Oracle Database 2609.22781 (hybrid storage
  I/O at production scale): no Rust code-shape rule, but the io_uring
  production-scale datum is a one-line marker for any future rust_perf
  async-I/O lane.
- **Theory adjacency, NOT a trigger** — Restricted isometry of sampled
  Fourier/Hadamard matrices via entropic descent 2609.22568 (math.IT):
  theoretical backdrop for random-Hadamard sampling; adjacent to the
  F16-FWHT dtype-nativity rider's MATH, not its ADMISSION question (the
  rider waits on riir-ai Plan 602 B5 or a second dtype-native instance —
  unchanged by this row).
- **Bulk off-lane** — ~955 rows, astro-ph/math/quant-ph/hep/cs.HC-class,
  counted ungraded.

## Yield at open

0 rule-batches · 3 spec/lane records (21858, 23033, 22471) · 1 league-intel
row (23130) · 1 riir-train-restricted doctrine echo (22870) · 10 named skip
rows + bulk. The walk's differential base is 2609.23388 — everything
submitted after 2026-09-20T06:11Z is walk #11 proper's window when it
opens Mon 09-28.
