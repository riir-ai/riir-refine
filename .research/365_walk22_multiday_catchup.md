# Research 365 — Walk #22: the multi-day catch-up (window 2026-10-30T03:00Z → 2026-11-03T03:00Z)

**Status:** RECORD — the arxiv announce-pipeline stall (origin Fri 10-31 ~03:00Z, the walk-21 empty-rebuild class re-firing) self-healed; this walk graded the whole unfrozen span in ONE batch (the walk-21 catch-up shape), 4470 unique entries, 23 pages, ≥12 s spacing, zero retries fired, zero seed hits (seed 011 = 298).

**Status line 2:** the stall lasted ~3 days (Fri 31 03:00Z → the Tue-roll announce landed ~Nov 3 03:00Z); the 5-day walk-14/15 alarm did NOT fire — day ~4.3 when probed. The Thu-30 20:00 ET announce (lost Fri) + Fri/Sat (no announces, calendar — the walk-22-poll correction held) + Sun roll + Mon roll all arrived inside this window.

## Lane profile (grade counts over 4470)

| grade | n | share |
|---|---|---|
| A− (league-lane primary) | 1 | quantized recurrent LMs |
| B+ (mining/league candidates) | 3 | ReWY, Weights-That-Stay, SHq |
| B | 6 | SparseGDN, blazing-smtlib, Quantizing-Coding-Agents(-ish), Fairway-adjacent, TVP-adjacent, GateTrim-adjacent |
| B− | 12 | Diagonal-Equivalence, Who-Asks-Who-Answers, DAMI, CISA, AttentionIndex, SpecPilot, SpecOPD, MatMoE, MXCode, BitLLM, FlyWheel, TileWeave, RankLane, SPTA, RLHF-forecast, APG, AgentPRO, LARK, MiniMax-H3 (release-watch) |
| C+/C | 11 | KV-eviction, Uni-LaSpec, Turbo-GMM, TVQ, outliers-SA, energy-cost, Diverse-Teacher-OPD, subliminal-preference, sensitivity-inflation, H3-adjacent |
| skip | ~4435 | the cs.CV/cs.RO/math bulk (outside every standing lane) |

(The B/B− boundary rows above are approximate buckets; the per-paper rows below are the record of truth.)

## The graded rows (family-routed)

### League lane (Bonsai ternary / GDN / PQ2 — riir-infer + riir-train)

- **2611.01864 · Quantizing Recurrent Language Models Is Easier Than You Think · A−** — the walk's primary. Linear-recurrence LMs vs the softmax-tuned quant stack; the extra-state vs recurrent-state quantization SPLIT as the analysis frame; controlled 1.3B finding: **decayed recurrences develop weaker residual outlier channels than softmax attention → easier to quantize**. League relevance: direct evidence FOR the Bonsai-PQ2 ease-of-quantization posture + a vocabulary split (extra-state vs state) that names what our PQ2_0 (state-half) vs dense/extra quant lanes already separate. Follow-up: read the full paper for the state-quantization findings against our lossy state-half successor lane (Issue-750-T3 law) — IF it carries a state-quant law we lack, it is a league-cell candidate; if not, it is corroboration only. No corpus conversion (model-based research, not a code pattern).
- **2610.41420 · SparseGDN: Chunk Sparse in Linear Attention · B+** — GDN long-context prefill sparsification: structured intra-chunk approximation + input-dependent recurrent-event selection, block-adjacent solver for the Delta-solve, GPU impl materializing only selected events. League relevance: the chunked-GDN MMA prefill lane (`RIIR_PREFILL_GDN_CHUNKED`, promotion REFUSED — the banked defense) fights exactly this cost; a sparsification lever is the escalation-ladder's next rung IF the opponent ever defaults their MMA copy on. Read-before-reach: the banked defense stands; this is a WATCH row (the paper's gains need their accuracy read before any kernel work).
- **2611.02966 · SHq: Learnable Grids for LLM Weight Quantization · B+** — learnable LUT grids vs fixed (NF/BOF/INT); "no single fixed grid is universally optimal — BOFS at 2 bits, NF at 4–5, INT at 6". League/claim relevance: our PQ2_0 is a fixed-grid LUT family; the bit-width-dependence datum is claim-shaping lore for any future grid retune (a riir-train act — the grid is trained, never runtime). Watch row.
- **2611.01940 · ReWY: Faster Gradient Computation for Delta-Rule Attention Family · B+** — algebraic backward reformulation (the chunk correction as ONE triangular residual system; 7 WY products → 3; centered contractions; scalar-gate reductions reuse key-gradient products), exact arithmetic, DeltaNet/GDN/KDA/GDN2, FLA port + reproduction code, 1.09–2.89× backward. Train lane: the backward-cost lever for any future GDN-family training run (riir-train owns it; nothing here touches serving). Exact-arithmetic identities = the same rigor class as our GDN spectral work.

### kernel_opt mining candidates

- **2611.00506 · Weights That Stay: On-Chip Megakernels for Serial Heads in Speculative Decoding · B+** — the transferable kernel LAW: a serial small-state walk (per-step ~state-update) repeatedly re-reads a large IMMUTABLE projection each step; the fix keeps projection shards resident on-chip across the whole serial walk in a persistent megakernel (weight residency + GPU-side token selection coupled). The pattern is NOT specdec-specific: any serial-head × immutable-big-projection shape (draft heads, per-step control heads, speculative verify loops) hits the same HBM-re-read tax. Distill candidate for the next kernel_opt batch (needs the repo scan — the paper's own GPU baseline + a source anchor; the corpus has persistent-kernel kin — dedup required: `persistent.*kernel|weight.*residen|megakernel`).

### rust_perf / lint-adjacent

- **2611.00185 · Blazing-Fast SMT-LIB Parsing for Rust and Lean · B** — blazing-smtlib: a Rust parser with fast-lexing/parsing implementation decisions + Lean-verified equivalence for the Lean port. Distill-source candidate for the rust_perf parsing family (the repo scan decides yield; the Lean-proof-of-parser-equivalence angle is FV lore for our proof lanes). Watch row for a future distill batch (needs the .raw clone + HEAD pin; NOT fetched this walk — grading only).

### The already-distilled twin (dedup note, load-bearing)

- **2610.42147 · Latent Sparse Attention · ALREADY IN THE CORPUS** — this id appears in the walk window (published 10-30T17:59, after walk-21's cutoff) but was ALREADY distilled via the league lane as riir-infer `.research/020` (the LSA monotone crossover law; the 1139→1145 decode campaign rode it). The walk does NOT re-grade it; recorded here so the seed/note trail explains why the league's find and the walk's window overlap. Seed 013 carries the id.

### The walk's release-watch row

- **2611.00843 · MiniMax H3 (20.38B video DiT) on Apple M5 Pro / MLX · B−** — a systems/empirical deployment report (Q8 g64 path, 576×1024 clip 4838 s, MLX peak 27.47 GiB, blind human review 3.12/3.18/2.94). Release-watch relevance: an independent M5-class MLX measurement datum on OUR silicon family (amo/bmo are M5 Ultra) — comparison-point lore for any future video-DiT or MLX-vs-ours cell; no action.

### B− tier (family-mapped, one line each)

- 2611.03101 Diagonal Equivalence in SSMs — SSM diagonal-equivalence classification + Mamba-3 MIMO lifting (architecture theory; watch).
- 2611.01909 Who Asks, Who Answers — hybrid LM circuits: linear attention BUILDS the softmax query (Qwen3.5-GDN Encode–Address circuit; architecture lore for Bonsai hybrids).
- 2610.41994 DAMI — GDN A_log/dt_bias init shapes retention (riir-train init lore).
- 2611.03200 CISA — retrofit dense→chunk-indexed sparse attention without gradient updates (long-ctx lane).
- 2611.02508 AttentionIndex — causal inverted-file index cuts indexer cost at 1M ctx (retrieval/index kin; the corpus has the crossover law — LSA/020).
- 2611.02501 SpecPilot — autonomous SD algorithm discovery (specdec lane).
- 2611.02846 SpecOPD — branch-parallel OPD for drafters (specdec+train).
- 2611.00588 TVP — TV-distance draft training matches/beats CE/KL (specdec; our spec lanes are closed-negative — value only if the league re-opens spec).
- 2611.01704 MatMoE — Matryoshka MoE + per-step dynamic mixed precision (quant+MoE).
- 2611.00403 MXCode — VQ codebooks onto MXFP4-native compute (~2-bit weights on MX hardware; future-HW watch).
- 2611.00547 BitLLM — conversion-hiding low-bit decode on mobile GPUs (mobile kernel; not our surface).
- 2611.01426 FlyWheel — TPU MXU attention kernels (softmax + triangular inversion reformulation; TPU-only).
- 2611.00408 TileWeave — NPU/coarse-tile vs GPU/fine-tile sparse prefill split on mobile SoCs (mobile kernel).
- 2611.00421 RankLane — ranked fixed-capacity expert lanes for MoE prefill (serving/kernel).
- 2611.03133 SPTA — teacher-side adaptation to student prefixes in OPD (the distill-from-winner teacher-quality axis).
- 2611.03244 Forecasting RLHF response to RM edits pre-training — first-order/price reading of RM edits (train methodology lore).
- 2611.02349 APG — LLVM-IR graph DB atomic analysis capabilities for agents (the 38329 corpus-authoring kin; C+/B−).
- 2611.00628 AgentPRO — agent-workflow compiler: model decisions only where observation is needed (decision-hiding lore).
- 2611.02028 LARK — Lean-assisted reasoning over LLM-generated GPU kernels (FV/kernel-authoring lore).

### C+/C tier (recorded, no action)

2611.00391 KV-eviction-for-agentic · 2611.01617 Uni-LaSpec (load-adaptive VLM spec) · 2611.02449 Turbo-GMM (FlashAttention-style GMM) · 2610.41822 TVQ (STE-trained VQ) · 2611.00286 outliers+sparsity SA-PTQ · 2610.41672 energy cost of quantized agents (A100 shared-GPU) · 2611.03272 diverse-teacher OPD (rec-sys bias) · 2611.00665 quantization reshapes subliminal preference · 2611.02368 quantization affects coding agents (the lossy-surface promotion rule's behavior axis — lore for cargo-refine quantized repair) · 2611.01411 sensitivity inflation in private quantized FT · 2611.00654 GateTrim (adaptive-rank W4A8 low-rank+residual — the SST-family shape, train lane) · 2611.00660 TopoQuant · 2611.01198-adjacent audio/Mamba rows (MFlowAudio, ResMamba, PAM-Mamba — domain lanes we don't run).

## What the walk does NOT claim

- Zero corpus conversion (13th+ consecutive zero-corpus-delta walk) — mining candidates (2611.00506, 2611.00185) go through the plan-first batch pipeline with a .raw clone at a HEAD pin, never from abstracts.
- The league rows (2611.01864, 2610.41420) are READ-AND-WATCH, not bench actions: the banked defenses (the chunked-GDN refusal; the PQ2 incumbent) stand until a bout or an owner call re-opens them.
- No GPU, no cargo — network-only lane per the Plan-337 posture (amo quiet at run time).

## Artifacts

- Fetch: `.distill/012_walk22_extract.py` VERBATIM (both walk-21 corrections verified live: term-regex + required FROM), END=202611030300 FROM=202610300300, seed 011 (298), out `/tmp/walk22` — 4470 seen / 4470 unique / 0 seed hits / 0 retries.
- Seed for walk #23: `.distill/013_walk23_dedup_seed.txt` (4768 = 298 + 4470) — pre-staged this landing; walk-23 REUSES the 012 tool verbatim (FROM is a required arg since walk-22).
- This note (`.research/.highwater` 364 → 365) + the snapshot landing row.

Session: zed-riir-refine-195, epoch 1793706957
