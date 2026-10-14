# Research 265 — arxiv walk #15 OPENING-BALANCE close-out (2026-10-14)

**Status:** RECORD — the Research-258 instrument-correction leg executed: the
walk-#15 opening balance (submissions 2026-10-10T00:00Z → 10-13T23:59Z, the
weekend wave absent from walk #14's announce index) graded NOW, Wednesday
10-14 ~13:28–13:31Z probes (M3, riir-refine idle loop cycle 89, zero-cargo,
web-API only, claim `23c34623` pushed BEFORE any lane fetch per the walk
protocol). **Zero corpus delta: 0 rule-batches — the SIXTH consecutive
zero-delta walk.** Lane profile: spec-dec 2 · plain-KV 37 · quant-broad 219
(~190 quant-ph/physics bulk counted ungraded) · gpu-kernel 7 · cs-PF 71
(~45 edge/scheduling bulk) — **~60 in-lane rows graded/skip-counted** after
cross-lane dedup (MultiAlloc/AlignPred/BELT/E8/QJL appeared in two lanes
each). The fresh-week window (10-14→10-19 submissions) stays walk #15's
proper leg at the 10-20 announce — this note CLOSES the opening balance so
that walk reads clean-week only.

## Index-state line (the re-poll the correction asked for)

**The 5-day stall RESOLVED.** Walk #14's six probes (through 10-13 02:31Z)
read the newest index row at `2610.13935` (submitted 10-09T17:56:59Z); this
session's probes (13:28–13:31Z 10-14) read `2610.18871` (submitted
10-13T17:59:53Z) as the newest cs.LG row — the 10-10→10-13 wave IS in the
index. The fill-lag is variable and long (≥35 h past the announce at walk-#14
cutoff; filled by ≤35 h after that), so the recorded protocol STANDS: grade
the announce-day window only at ≥04:00 UTC probe time, else Wednesday. The
opening balance graded here was stable across three spaced lane queries —
final. Oldest graded row `2610.14448` (10-10T01:24Z) > walk #14's index
ceiling `2610.13935` — dedup clean BY CONSTRUCTION plus a walk-#14 graded+skip
ID cross-check (no overlap).

## Graded rows

| # | ID | Title (short) | Lane | Grade | Reading |
|---|---|---|---|---|---|
| 1 | **2610.14754** | GateSight: GDN memory anticipates attention KV | plain-KV | **B+ lane record (league arch family HIGH)** | Hybrid GDN+attention serving on **Qwen3.8-27B** + Qwen3.8-Flash-Next — OUR league arch family (the Qwen3.8 decode lane, the 971/919 orbits). The atom: GDN layers' own prefill signals (write gate, decay, write magnitude) anticipate the future utility of attention-KV entries BEFORE any query exists; per-head MLPs read them into a query-agnostic eviction score, cache scored once post-prefill, discarded entries physically released. Two structural facts travel: (a) "which GDN heads keep long memories is fixed by the WEIGHTS rather than by the document" — a per-checkpoint constant, calibratable once at load; (b) the per-layer keep-fraction is set by the model's native attention output gate (mean differs >10× across layers) — the consumer model already carries the allocator. 1/10 KV at best-of-five accuracy, leads KVzip 9.5–47.2 pts. Routing DETECTION-ONLY — consumers are the live GDN-recurrence serving lanes (riir-infer 045/936/937 line); no tree edits. |
| 2 | **2610.15709** | Preserving Updates in Low-Bit Recurrent State Quant | quant-broad | **B+ lane record (GDN state-quant family, 2nd datum pair)** | The recurrent-state quantization family's mechanism half (walk #13's STEPQuant was the error-propagation half): state is REQUANTIZED every step, so errors accumulate coherently — driven by SMALL UPDATES LOST IN ROUNDING and by REVERSALS (an element moving opposite to its update; direction matters more than per-step rounding error). Repairs training-free: LRB (low-rank bypass buffer accumulates small updates, periodically merges the grown ones) + BDR (round reversed elements toward their updates, zero storage). Across **Gated DeltaNet, GLA, Mamba2, RetNet**: beats INT4 substantially, 5.3× state-memory cut, 1.9× decode vs FP32 states. Directly our GDN-decode/ternary-recurrence lane intel. |
| 3 | **2610.15139** | When Dynamic Scaling Fails: scale-state feedback | quant-broad | **B+ doctrine (the certification-horizon law)** | Recurrent-state quant's failure mechanism NAMED: a feedback between the quantizer scale and the state the scale is derived from (INT4 intact at 1K destroys recall by 32K). Contain the scale (yoke to an FP reference) kills the norm explosion but recovers only 1.8 of 5.4 nats — **numerical instability and functional degradation are SEPARABLE failures**; outlier-location loses to plain persistent scaling everywhere tested. The operable law: **"report the decode horizon at which a bit-width was certified"** — the direct external validation of our lossy-surface per-family-conditional gate (a bit-width claim without its horizon is the p99-without-tail-support class). Learned cross-layer bit allocation collapses onto "protect the first few layers" under article-disjoint splits. |
| 4 | **2610.15715** | NoteBank: long-context linear attention via replay | plain-KV | **B lane record (GDN long-context)** | The capacity-ceiling refutation: "recurrent computation need not bear the full burden of long-term storage." Training-free episodic text archive + native-feature write/address + replay through the backbone's own forward. GDN RULER 1.7%→70.3% at 1M tokens, ≤0.025% FLOPs, persistent store ~1/2600 of MHA KV. GDN-family long-context intel beside GateSight (eviction side) — the two together sketch the hybrid-serving KV/state split. |
| 5 | **2610.14745** | SAGE: 2-bit vector-quantized LLMs | gpu-kernel/quant | **B lane record (VQ weight quant)** | 2-bit weight-only VQ SOTA: Energy-Stratified Grouping (sensitive channels prioritized inside each jointly-encoded subvector, activation-weighted fitting) + Adaptive Bit Split (per-matrix direction-codebook vs radius-precision budget under a fixed split, activation-weighted reconstruction chooses). 95.3% BF16 retention on Qwen3-30B-A3B with a custom CUDA kernel at 2.55× non-streaming speedup. Adjacent to our PQ2_0 family — the direction/radius SPLIT axis and the energy-tiered channel grouping are the two unowned atoms; the family-conditional retention gate (real-task suites, not ppl) is their eval discipline. |
| 6 | **2610.15417** | SpecFit: draft-tree selection by predicted throughput | spec-dec | **B− doctrine (cost-aware-verify-length, 4th instance)** | Offloaded-MoE spec-dec: each verified token activates its OWN experts, so a larger draft tree transfers more H2D bytes — acceptance length is the wrong objective; select the verified subtree maximizing predicted THROUGHPUT (expected committed tokens ÷ cycle time, the cycle priced by the per-layer UNION of activated experts — a monotone submodular coverage function, each node pays only the experts it adds; exact ancestor-closed-subtree search doubles as the stop-drafting rule). The FOURTH independent instance of our B223 `cost-aware-verify-length-survival-cost-table` family (lithos R6, APEX were 2+3); the union-of-experts cost model is the new atom. 21.6%/7.0% over EAGLE-3 on Qwen3-30B-A3B. |
| 7 | **2610.16020** | When does compensating inherited quant error help? | quant-broad | **B− doctrine (compensation-guarantee honesty)** | Layer-wise PTQ compensation (each layer corrects inherited error) is guaranteed no-worse only in a narrow setting (linear nets, zero-conditional-mean rounding, deterministic second moments — dither-class). With ReLU it fails TWO ways (noise changes activation states; the correction moves error from inactive to active samples); bounded codebooks can fail even at unique exact layer-wise minimizers; a fixed-error counterexample refutes a published universal guarantee at two scalar linear layers. Empirics: fixed half-strength compensation beat none on 8B models at 4 bits, but NO strength rule beat a layer-wise rule consistently, and tuning on the calibration text did not transfer. The calibration-doesn't-transfer datum beside DRQ (walk #14). |
| 8 | **2610.14475** | EdgeLLM-Nano: LCDF vs perplexity for KV damage | plain-KV | **B− measurement record (family-conditional gate echo)** | A 33.6 MB testbed isolates KV-compression damage end-to-end: perplexity gives the SAME 5.461 score to all methods while their downstream damage differs by orders — the token-by-token teacher-forced LCDF metric (paired stats across seeds/effect sizes) separates them. Negative verdicts for a polar-guided key-cache quantizer (RTN control does an order less damage) and a consistency-anchored pruner (one-shot magnitude matches). The aggregate-metric-blinds-while-family-conditional-flips law, measured in miniature — our lossy-surface gate's external validation #2 this walk (beside 15139's horizon law). |
| 9 | **2610.15186** | SAFE-KV: causal suffix repair for non-prefix reuse | plain-KV | **B− doctrine (the causal-state contract)** | The non-prefix-reuse hazard family's cleanest statement: "a cached document state is not reusable text: it is a context-conditioned causal state" — reuse is a BEHAVIOR-PRESERVATION problem with full prefill on the exact prompt as the reference (our bit-identity/reference law in their vocabulary). Risk-guided suffix re-encoding restores 98-100% where document-only/window repairs fail (post-repair states keep histories that omit the repaired region); the honest operating rule: below a workload threshold, full prefill WINS — repair is not free. |
| 10 | **2610.15509** | CacheLink: fidelity is a poor proxy for utility | plain-KV | **B− doctrine (proxy-vs-utility scoring law)** | Cross-model KV transport: reconstruction fidelity (K/V squared error) is a POOR PROXY for receiver utility — similar reconstruction errors yield substantially different attention patterns, distributions, and accuracy; learning the correction through RECEIVER BEHAVIOR changes error by <0.4% while substantially changing behavior. The score-what-you-ship law (our oracle/verdict discipline) in a third domain this quarter. 7.27× handoff-latency win at 32K. |
| 11 | **2610.15629 / 2610.15640** | Price of Precision + Context-Aware Precision Scheduling | cs-PF/plain-KV | **B− doctrine (binding-resource cost model, series pair)** | "Fewer bits do not always mean faster": served speed is limited by compute, weight loading, or KV access depending on phase/batch/context — a byte-based proxy misses all three; a calibrated binding-resource cost model predicts latency across three hardware classes and drives mixed-precision allocation under latency OR accuracy budgets (the SLM version runs on a laptop GPU, one KV-quant error estimate re-budgeting weights). The byte-proxy≠binding-resource law — the B211 one-decision-two-costs family's serving-face echo. |

## C+ records (counted individually, ~30)

E8-lattice KV keys `2610.15384` (k2v2 at throughput parity, table-based
scoring avoids codebook dequant — the structured-VQ primitive) · Fast
Quantized JL `2610.16289` (Hadamard-composed QJL projections, Gaussian
dithering vs random permutation — Hadamard-family adjacent, KV/QJL domain) ·
MultiAlloc `2610.18642` · ChampionKV `2610.18026` · BELT `2610.17966` ·
AlignPred `2610.17970` (orthogonal alignment + predictive coding KV codec) ·
MLAfication `2610.17202` · VHA `2610.17959` (2-KV-head GQA, inference-time
folding — the fold-at-inference discipline echo) · Hashrope `2610.16144`
(persistent verified-content-identity rope — the content-addressed class) ·
RadixFlow `2610.18143` (hybrid prefix-serving state-ready traversal,
Qwen3.5 hybrids — head-of-line blocking fix) · Budgeted KV repair `2610.17338`
(SUR — coherent-region refresh beats isolated tokens at equal budget) ·
LatentRemedy `2610.18014` · SwaT `2610.15095` · CacheFuse `2610.16165` ·
VReparo `2610.15721` (the non-prefix/cross-model reuse family — 8 rows this
wave, the largest family clustering since the walk series began; SAFE-KV #9
is its doctrine carrier) · KV4-RL `2610.14448` · FlashMask-3 `2610.17901` ·
GPU Goldmines `2610.17321` (WebGPU LLM kernels, 1.41× over llama.cpp WebGPU
— our wasm lane's opponent surface) · CSNB `2610.18557` (Cauchy-Schwarz SDC
detection, 99% of ABFT at 87% less overhead; checksum stays right for MoE
routers) · Native Dual Pools `2610.15755` (hybrid attn+SSM memory
organizations, 4 designs measured — the GateSight/RadixFlow serving
companion) · LinMorph `2610.17914` (softmax→GDN weight-space morphism;
initialization-transfer decides linearization success) · DeltaNet-PC
`2610.15185` (PC-native verbatim memory, DeltaNet update-rule kinship) ·
LoopQuant `2610.16368` · TGQ-MambaSR `2610.16261` · NDA `2610.17969`
(dimensionally-consistent arch: int8 loss 0.009 vs 5.51 nats) · Mixed-Precision
Scaling Laws `2610.17637` (fwd/bwd separation; optimal precision RISES with
compute budget) · Error-Feedback GPTQ `2610.18139` ("trace optimality need
not improve finite-level performance") · Green-AI quant energy `2610.17531`
(quantization peak-efficient at long-input/short-output; runtime is a
reliable energy proxy) · SP²ec `2610.18735` · SuPER-Mamba `2610.16636`
(surprisal-routed global-channel updates, training-free +94.7% rel) ·
Tremulant `2610.18532` (warm-start tails hundreds× median — the tail-class
doctrine echo).

## Yield

**0 rule-batches** — nothing grades A/B-kernel against the corpora; the wave
is lane/doctrine intel. **3 B+ lane records** (GateSight → the GDN-serving
lane detection-only note, consumers named; recurrent-state-quant PAIR 15709 +
15139 → the GDN-decode/ternary-recurrence lane, the certification-horizon law
rides our lossy-surface gate doctrine) · 2 B (NoteBank, SAGE) · 6 B−
doctrine · ~30 C+ · the rest skip-class counted (quant-ph bulk ~190, edge/
scheduling/RL-infra bulk ~45, video/agentic classes). **New differential base
for walk #15 proper (10-20 announce): everything above + walk #14's
graded+skip IDs; the fresh week (10-14→10-19 submissions) is its only
remaining window.**

## Walk-family instrument notes

1. **The opening-balance split WORKS** — grading the stalled wave at the
   re-poll (rather than at the 10-20 walk) keeps the weekly walk's window
   clean and cuts its grading load by the wave's size. Record as the
   standing procedure: when a walk grades on a stalled index, the NEXT
   session at ≥04:00 UTC / Wednesday runs the opening-balance pass first.
2. **Fill-lag spread is now 2 observations** (walk #12 ~2 h; walk #14 ≥35 h
   then filled) — the "≥04:00 UTC else Wednesday" rule covers both; keep
   grading announce-day windows on the Wednesday fallback, never same-hour.
3. The API path (export.arxiv.org, submittedDate-sorted, keyword-scoped per
   lane, 9 s+ natural spacing) ran 8 queries with zero 429s at 13:28–13:31Z —
   the throttle is hour-of-day-dependent (walk #12/#13 note stands).
