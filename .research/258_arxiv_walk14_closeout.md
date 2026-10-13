# Research 258 — arxiv pool refill walk #14 close-out (2026-10-13)

**Status:** RECORD — RENUMBERED TWICE (255→256→258): the M3's same-hour
batch landings consumed 255 (batch 225), 256 (batch 226 p49-p50 riders), and
257 (batch 227 lithos wave-2) while this walk ran — their committed docs keep
all three per the collision rule. Walk executed 2026-10-13 ~07:4x–10:4x +07
(00:4x–03:4x UTC,
shikuwa/4090, riir-refine idle loop cycle 46, the due clock; window
2026-10-06T00:00Z → 2026-10-13, covering the walk-#13 boundary day in full —
zero overlap with Research 237's graded/skip IDs, all of which sit at
2610.0365x or earlier). **Zero corpus delta: 0 rule-batches — the FIFTH
consecutive zero-delta walk.** Lane profile: spec-dec 13 · plain-KV 28 ·
quant-broad 94 (~60 physics/quant-ph bulk counted ungraded) · gpu-kernel 13 ·
cs-PF 10 — **~118 fresh unique rows** after cross-lane dedup (8 rows appeared
in two lanes each: SPIN, ReadKV, TME, CXDVirt, ResidualQuant, Dual-QK,
ReplaySSM, CORAL-KV). Index-state note: the announce (Tue 10-13 00:00
UTC) preceded the walk by ~50 min; the weekend wave (10-10→10-12
submissions) was ABSENT at every probe — the walk-#12 "fills over ~2h" law
DID NOT reproduce (see the final index-state line): graded on the stable
in-week state, with the weekend wave carried EXPLICITLY as walk #15's
opening balance (the walk-#11 precedent).

## Graded rows

| # | ID | Title (short) | Lane | Grade | Reading |
|---|---|---|---|---|---|
| 1 | **2610.13778** | ReplaySSM: Cache SSM Inputs, Not State | spec-dec/quant | **B+ lane record (GDN serving intel HIGH)** | Unroll the recurrence: state = checkpoint + buffer of recent inputs. Separates output computation from checkpoint updates — state writes skipped on most steps (**"roughly halves the dominant state memory traffic"**), speculative rollback becomes a buffer-pointer op, output-only verification without materializing intermediate states, and **dual-precision checkpoints: a low-precision copy serves output computation while a higher-precision copy serves recurrent updates, "so rounding error in the low-precision copy never enters later updates"** (the STEPQuant when/where law from walk #13, answered by mechanism). Implemented in vLLM for **Mamba-2 and Gated DeltaNet** (our league arch family); 1.20–1.47× decode at bs=256, ~1.8× over vLLM spec at 120B. **Routing: the GDN-recurrence decode lane (riir-infer issue 045 + katgpt-rs 936/937 — the LIVE sibling lane) and the 971/919 decode orbits are the consumers — detection-only here, no issue edits (the 045 session owns that tree).** The state-traffic-halving and the input-buffer-rollback are the two atoms any chunked-GDN decode re-arm must price. |
| 2 | **2610.07853** | Lost in the bf16 Cast: Exporting Ternary LMs Can Revert Most Low-LR Code Changes | quant-broad | **B+ doctrine (ternary export determinism)** | The labs' documented ternary export pipelines cast fp32 latents to **bf16 before thresholding** — bf16 rounding lands products exactly ON the tie threshold, ties-to-even maps them to zero, and 0.83–1.77% of deployed codes disagree with fp32 quantization of the same latents. At fine-tuned endpoints (LR sized to the bf16-ULP ratio) the documented export collapses GSM8K strict accuracy **58.79% → 0.78%**; a bare bf16 save+reload costs BitNet-2B-4T 27.54 points. Remedies (both non-inferior): write the training quantizer's codes directly, or adjust bf16 inputs until the unchanged tools emit them. **The export-path determinism hazard class for every ternary lane we run** (the PQ2_0 codes ride an export step; our G1 bit-identity gates are the correct defense and this is their external validation). Doctrine record; routing note for the riir-train export lane (read-only — routing restriction). |
| 3 | **2610.13325** | GraphTok: Taking Tokenization off the Host for Agentic LLM Serving | cs-PF | **B lane record (host-stage bottleneck; prefill-first priority)** | Under prefix caching, prefill skips most compute but **every request still tokenizes in full before the cache can be consulted — tokenization becomes the largest part of TTFT**, and one CPU frontend tokenizing for a GPU fleet bounds the fleet it can feed. GraphTok runs the whole BPE pipeline (pre-tokenization → merges → ID assembly) as ONE replayable CUDA graph, bit-for-bit on GPT-2/Llama-3/Qwen-2.5/DeepSeek-V3/Gemma-3, hands short inputs back to CPU, 1M GPT-2 tokens in 1.7–3.2 ms — 4.7–6.0× over the strongest CPU tokenizer; one frontend sustains 518 req/s of 121k-token traffic (56 GPUs' busiest-hour load) vs 18.5 default. **The host-stage-is-the-bottleneck law on the exact stage our serving lanes ignore**; also the strongest public instance of the whole-pipeline-as-one-CUDA-graph shape (our cuda-graph family laws). Lane intel for the prefill-first priority; our reflex/instinct serving tokenizes on host per request. |
| 4 | **2610.08403** | SSR: Sparse Segment Reduction for Ternary GEMM Acceleration | quant-broad | **B lane record (ternary sparsity axis)** | Ternary LLMs carry 50–90% structural zero weights that BitNet/RSR/RSR++ ignore and sparse formats that ignore ternary structure; SSR's dedicated ternary format + sparsity-scaled computation trees give 2.1–11.3× over RSR++ at 45–95% sparsity, 3.5–6.3× e2e on Llama-3-1B. **Our Bonsai-2 PQ2_0 codes are dense-ternary — the zero-fraction of the code distribution is the axis this paper prices** (if the folded weights carry ≥45% zeros, the sparse-segment axis opens for our GEMV/GEMM lanes). Watch record for the ternary lane; no adoption without measuring our own zero-fraction first. |
| 5 | **2610.10924** | The Missing Fourth Term for the Emulation TME Model | gpu-kernel | **B− doctrine (cost-model honesty)** | The "FP8 is All You Need" fp64-emulation cost model omits the per-INPUT deconstruction cost (every streamed fp64 operand scaled/rounded/reduced modulo r moduli on SIMT pipes BEFORE any matmul issues); adding it yields a closed-form OI* threshold **below which emulation cannot match native regardless of tensor-core throughput** — the claimed 3.1× on memory-bound kernels is actually 0.3–0.9×. The cost-table family's external doctrine: a per-op cost model must price the CONVERSION path, not just the compute and traffic terms (kinship: our B219 fastest-of-n cost-table rules + pava-monotone-cost-pooling — "the rank-identical-decision determinism atom" is what this paper defends). |
| 6 | **2610.09212** | CurveTQ: Rotation-Free Trellis Quantization via Curvature-Weighted Search | quant-broad | **B− lane record (transform-before-quant counter-family, second datum)** | The 2-bit trellis quantizers' random rotation exists only to FLATTEN within-block curvature — putting the Hessian-LDL diagonal weights INTO the Viterbi branch metric makes "weighting in the native basis and rotating **substitutes**"; no rotation to undo at every decode step, fastest decoder of the three, 1–3 points better at 2 bits. **The second "avoid the rotation" answer in two weeks** (walk #13's XOR-Trellis was the first): the RotaryQuant/Issue-716 "rotation cost strictly larger" premise now has a measured substitute-class alternative. Feeds the standing ternary-quant transform watch. |
| 7 | **2610.09778** | Reproducible LLM Inference Benchmarking: Sequential Isolation Protocol | cs-PF | **B− measurement record (bench discipline echo)** | Controlled regression-testing protocol (sequential isolation + deliberate concurrency variation) takes median-TTFT CV from **15.2% → 2.2%** on vLLM/A100. External echo of our bench-preflight/exclusivity/GPU-idle-gate discipline; the "78.5% of configurations under 3% CV" bar is a citable external reference for what a controlled protocol achieves. Record. |
| 8 | **2610.11226** | When Lower Reconstruction Loss Hurts (DRQ) | quant-broad | **B− doctrine (calibration honesty)** | Weights that minimize calibration reconstruction loss can be WORSE downstream — even on the same calibration data — because low-reconstruction weights are brittle to input-distribution shifts; distributionally-robust refinement over activation-distribution sets fixes it post-hoc within the existing grid. The measurement-honesty family's calibration axis: the optimizing objective ≠ the deployment objective (echoes the aa-control/lossy-surface class one lane over). |
| 9 | **2610.11201** | PageWeaver: KV-Guided Query Unions for Sparse Attention | gpu-kernel | **B− kernel-lane record** | Dynamic sparse attention's small per-query support does not yield efficient GPU work; query UNIONS share page loads and populate Tensor Core tiles, assembled by selected-page affinity via a bounded GPU search consumed by an ID-aware two-CTA kernel — **no materialized Q reorder, no cross-page partial outputs**; FP8-KV H200 1.70× over FlashInfer. Our league models are dense-attention — watch record for any sparse-KV lane opening; the "execution-group reuse vs the complete cost of exploiting it online" honesty split (their own median 0.47–0.73% at 32K/64K vs 3.26–7.66% selected) is the adoption bar. |
| 10 | **2610.12570** | DCOCO: Contextual Correction of Parallel Drafts | spec-dec | **C+ (DFlash2 ecosystem)** | Parallel drafters lose intra-block dependencies; DCOCO restores them by retaining multiple candidates per position + parallel contextual scoring across adjacent positions — up to 6.6× over vanilla, 6–23% over the strongest competitor under concurrent serving. The DFlash2 draft-quality lane keeps widening (DCOCO/EDR/DLoop this window); ecosystem record for the 1011 watch. |

## C+ records (counted individually)

- **2610.07780 APEX** — request-level drafter routing + block-level draft depth via
  **censored survival feedback with position-wise rejection hazards and block
  execution costs** — the THIRD external instance of our B223
  `cost-aware-verify-length-survival-cost-table` rule family (lithos R6 was the
  second); the family is now independently discovered three times. Doctrine
  echo only, no corpus edit.
- **2610.11245 ReadKV** — progressive-code KV storage where each query allocates
  key-channel prefixes then value-token prefixes; separate retained-bits vs
  fetched-bits budgets; proven strict separation from every query-independent
  reader. The query-adaptive-read axis (kinship: our narrow/wide dispatch
  families key on shape; this keys on the QUERY). Watch.
- **2610.13767 DualQuant** — calibration-free dual-scale W4A4 for MX/NVFP4,
  validated on an **RTX 4090** (2.46× over BF16, packed real inference path).
  Quant-family record; our box class.
- **2610.13708 Adaptive DHiSparse** — spec-dec over hierarchical sparse KV;
  DSpark/DFlash2 keep proposal+acceptance rules; window-bounded draft-KV ring
  (the 1011 spec-decode stance watch's ecosystem keeps growing).
- **2610.11358 RaReCache** — cross-model KV reuse via rank-disagreement
  selective recomputation (30% recompute retains 95–99% at 23× size gap);
  the cache-key-provenance hazard class (walk #13 #6) gains its repair lane.
- **2610.13060 KV-Repurpose** — non-prefix KV reuse testbed (Zipf workload:
  segment reuse 63.9% hit vs prefix-caching 3.3% at 30 GB); system record.
- **2610.13474 CompileMask** — runtime-metadata-dependent attention masks
  compiled to mask-specific preprocess kernels + compact execution
  descriptors (3.8×); the mask-predicate axis our FA admission rules key on.
- **2610.12133 Attic-KV** — "rehearse everything, remember nothing": under
  tight budgets, full-context rehearsal scores KEEP the wrong entries;
  rehearse what will be read (self-quiz) instead. KV-eviction class (746
  out-of-stack) but the counter-law doctrine is broadly citable: the
  importance signal must come from the CONSUMER, not the encoder.
- **2610.09025 SPIN** — shadow predictive indexer: history-based prediction
  replaces full-KV indexer scoring per step (30–40% sparsity, +14.9%
  throughput vLLM); sparse-attention lane record.
- **2610.07782 Persistent memory in multi-agent inference** — the measurement
  honesty row: "four measurement corrections — three inflating the apparent
  benefit — none visible in the results table"; the agent-memory-ablation
  conditions. External echo of our pre-registration discipline.
- **2610.12657 KPA** — Kronecker-product attention (generalizes TPA, 10× KV
  compression); architecture class, no league consumer.

## C records (counted)

Spec-dec lane (stays closed-negative for us, 717 G3a): TEV/ExitTrain
`2610.11750` (exit-law theory, +13% block length) · NSD `2610.07822`
(nucleus-relaxed verification, fidelity bounds) · DLoop `2610.07659`
(multi-stage drafting while confident, 5–41%) · layer-selection `2610.12939`
(entropy/CKA) · EDR `2610.10411` (training) · Speedbumps `2610.10929`
(rejection attacks) · SecureSD `2610.08678` (security) · S³R `2610.13202`
(RL rollouts) · EAGLE-ASR `2610.13506` (speech). KV/compression: VFold
`2610.12338` (cross-layer value merging) · ResidualQuant `2610.10381`
(looped-transformer 2-bit residuals) · Dual-QK `2610.09827` (2-bit +
query-channel pruning) · CanonQ `2610.09202` (W2A4KV2 QAT) · Universal
Attention `2610.09051` · LORE-KV `2610.07643` · EncBank `2610.10058` ·
RAM-Net `2610.11214`. Quant/training-track (riir-train routing): OnlineQAT
`2610.09346` · Q-PACE `2610.09183` · TRACE `2610.07767` · ZIP-SR `2610.12444`
· RAMP `2610.13041` · HSVDQuant `2610.11315` · Align-then-Correct `2610.08164`
· ApexQuant `2610.07904` · OSFP4 `2610.08231` · Layerwise-Error-Attribution
`2610.09877`. Systems/infra: Ember `2610.13408` (UVM oversubscription, LLM
4.7–16× at sizes manual placement cannot run) · Lachesis `2610.08378` ·
CXDVirt `2610.08899` · 2610.07781 (quantized-agent eval sensitivity flips
sign across prompts/designs — evaluation-design honesty) · ELR `2610.12582`
· HLA `2610.07940` · DySCo `2610.08268` · ReFold `2610.07863` ·
ByteExpress `2610.12815` · RustPower `2610.12654` (Rust power-flow solver,
24–30× Jacobian assembly — off-domain but our language).

## Skip classes (counted, ~60 rows)

- Physics/quant-ph/hep/quantum-foundations bulk in the quant lane: 2610.13727,
  2610.13358, 2610.13250, 2610.13213, 2610.13019, 2610.13017, 2610.10923,
  2610.10861, 2610.10791, 2610.10717, 2610.10220, 2610.10151, 2610.09900,
  2610.09855, 2610.09673, 2610.09436, 2610.09373, 2610.10621, 2610.08924,
  2610.10588, 2610.07719, + ~40 more of the same class.
- Vision/multimodal/video/TTS: 2610.13732, 2610.13433, 2610.13357, 2610.12156,
  2610.11756, 2610.12514, 2610.12266, 2610.12182, 2610.11105, 2610.10358,
  2610.10343, 2610.10342, 2610.11437, 2610.09027.
- Robotics/edge/embedded/communications: 2610.10886, 2610.08444, 2610.08412,
  2610.08358, 2610.09098, 2610.09178, 2610.11583, 2610.08174, 2610.10032,
  2610.12588, 2610.08732, 2610.08876, 2610.13311, 2610.10738.
- Theory/other: 2610.12716, 2610.11960, 2610.12809, 2610.08256, 2610.08884,
  2610.09292, 2610.09963, 2610.10933, 2610.10630, 2610.10641, 2610.07859,
  2610.12004, 2610.09802, 2610.12371, 2610.12300, 2610.11492, 2610.08483,
  2610.08238, 2610.09476, 2610.09202 (counted C above), 2610.11482.

## Yield

0 rule-batches · 2 B+ lane records (ReplaySSM → the GDN-decode lane
detection-only note, consumers named, no tree edits — the 045/936/937 sibling
lane owns that area; bf16-cast export doctrine → the ternary-export hazard
class) · 2 B lane records (GraphTok host-stage; SSR ternary sparsity) ·
5 B− (TME cost-model honesty; CurveTQ rotation-free #2; Sequential-Isolation
bench echo; DRQ calibration honesty; PageWeaver) · ~12 C+ · the rest C /
skip-class counted. **Walk #14 DRY for the corpus — the staged queue is
unchanged (the M3's batch-225 Q1_0 trio claim stands; the cubecl pin-bump +
fsearch distill rows idle behind it).** New differential base for walk #15:
everything above + the 10-06→10-13 window IDs; next announce Tue 10-20
00:00 UTC (~10-20 +07 grading window, fill-aware).

## Index-state line (final — measured, 5 probes)

**No weekend movement through 02:31Z** — probes at 00:47Z, 00:49Z, 01:14Z,
01:43Z, 02:12Z, and 02:31Z all read the same newest index row
(`2610.13935`, submitted 2026-10-09T17:56:59Z); the ENTIRE 10-10→10-12
weekend wave (zero rows across all five lanes for 3+ days against a ~13
rows/day baseline in the quant lane alone) is absent from the announce
index 2h31m past the nominal announce time. **Instrument correction for the
fill law**: walk #12's "the index fills over ~2h after the announce" was ONE
observation, not a law — this cycle the announce-wave index update had not
landed at all by 02:31Z (either the lag is variable and much longer this
cycle, or the index update lands later in the UTC morning). **Walk #15's
opening balance is the 10-10→10-13-submitted rows** — its grading pass must
re-poll the lanes LATER in the announce day (≥04:00 UTC) or on Wednesday,
and the window base is this note's graded+skip IDs PLUS the entire
10-10→10-12 submission range (not yet visible). The in-week rows graded here
(10-06→10-09 submissions) were stable across every probe — they are final.
