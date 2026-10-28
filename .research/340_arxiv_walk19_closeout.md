# Research 340 — arXiv pool-refill walk #19 (closeout; the Mon-eve + Tue batch)

**Status:** RECORD — pool refilled; **zero corpus delta at walk time (ELEVENTH consecutive zero-delta walk)** — but the lane yield is the richest since walk #17: 6 B+ rows across five lanes (hybrid-LM KV codec, batch-invariance kernel law, decision-serving systems row, Mamba-2 subspace law, fold-into-standard-inputs KV law, kernel-agent self-evolution). Zero cargo, zero GPU throughout (4090; walk-only session).

**Filed:** 2026-10-28 ~01:4x–02:4x UTC, shikuwa-4090-walk19/4090, riir-refine idle loop. Claim `9027f028`-lineage — the walk-19 claim row pushed 2026-10-27 ~04:2x UTC (epoch 1793074943, `shikuwa-4090-walk19claim`), ~22 h before execution; push is the arbiter; no counter-claim materialized.

**⚠ ROLL-TIMING DATUM (the Wed cycle; tightens walk-18's Tue-cycle bracket):** pre-roll ceiling held static at cs.PF newest 2026-10-26T12:32:49Z through the 23:42Z and 00:40Z probes; the 01:43Z probe read the batch (cs.PF newest 2026-10-27T08:05:58Z) — **the Wed-cycle index roll landed between 00:40Z and 01:43Z, i.e. up to ~1.5 h EARLIER than the Tue cycle's 01:45–02:16Z bracket.** Standing correction for walk #20 (a Thu-cycle grading): probe from ~00:45Z in ~30–60 min cadence; reads before ~00:40Z are dead. The claim row's "first poll ~02:15Z" guidance was conservative for this cycle — the batch was gradable a full half-hour sooner. Stable re-poll: identical cs.PF newest at 01:43:25Z and 01:43:26Z.

**Window:** submittedDate [2026-10-26T18:00Z → 2026-10-27T18:00Z] (the Mon-eve + Tue batch delivered by the Wed announce), five lanes over the search API (quant `abs:quantization` / spec-dec `all:"speculative decoding"` / plain-KV `all:"KV cache"` / gpu-kernel `all:"GPU kernel"` / cs-PF `cat:cs.PF`), graded ~7.5–8.5 h after the window close and ~1.7 h after the announce.

**Dedup:** clean — **28 lane rows fetched / 27 unique / 1 cross-lane dup counted once (2610.36950 quant+plain-KV) / 0 seed hits** (the 229-ID tracked seed `.distill/007`). IDs span 2610.36135 (10-26T18:45Z) → 2610.36950 (10-27T10:11Z) — no column shared with walk-18's graded set (its max was 2610.35023); ceiling discipline holds.

**Lane profile (fetched):** quant 15 · gpu-kernel 5 · plain-KV 4 · cs-PF 4 · spec-dec 0 = 28 rows → 27 unique. **Graded: 6 B+ + 4 B + 5 B−/lane/doctrine notes + 12 skip-class** (vision ×2, recommenders, physics/math ×5, FPGA trigger, crowd-counting bench, VLM survey, robotics tokenization).

## Headline rows

### B+ (6)

| # | ID | Title | Lane | Record |
|---|---|---|---|---|
| 1 | **2610.36623** | God Does Not Play Dice: Enforcing Batch Invariance in Agentic Kernel Generation | gpu-kernel | **B+ lane record (the determinism law's kernel-generation sibling; Issue 940 external confirmation)** |
| 2 | **2610.36617** | Balanced State Subspaces for Mamba-2 State Compression | quant | **B+ lane record (the SST substrate's anatomy law, externally re-derived on Mamba-2 states)** |
| 3 | **2610.36950** | DCKV: Learned Transform Coding of the Prompt KV Cache in Hybrid Language Models | quant + plain-KV | **B+ lane record (the KV-codec family's trained member; hybrid-LM = our GDN decode shape; Qwen3.5 named)** |
| 4 | **2610.36520** | Folding Attention Biases into Keys Makes Compacted KV Caches Run on Standard Kernels | plain-KV | **B+ lane record (the fold-into-standard-inputs deployment law)** |
| 5 | **2610.36804** | DFIP: Decision-Focused Inference Path for System One Models | cs-PF | **B+ (our decision-model comparison family measured as a class; reflex/rethink serving intel)** |
| 6 | **2610.36848** | Evolving Agent Teams | gpu-kernel | **B+ lane record (the healer/self-evolve program's kernel-agent sibling)** |

**1. God Does Not Play Dice (2610.36623)** — dynamic batching means the same request runs beside different neighbors across runs; without **batch invariance (BI)** the numerical output for an unchanged input depends on the batch context, and reproducibility breaks. Census: **19 confirmed BI violations among 83 batch-applicable released kernel artifacts**; the tool combines contract-guided static dependency analysis with dynamic validation of bitwise consistency across batch contexts. This is Issue 940's determinism law (synced f32 reductions pin reduction order + tie rule) re-derived at the kernel-generation layer: **the evaluated/deployed object must be bitwise-stable under batch-context churn**. The corpus's reduction-order and timed-region families own the sequential-fold face; the batch-context face is the new citation. Detection-only for us (our kernels are hand-rolled, not agent-generated) — but the 19/83 artifact violation rate is the standing argument for the x86_64 execution matrix's execute-don't-read posture.

**2. Balanced State Subspaces (2610.36617)** — Mamba-2's fixed recurrent state dominates decode memory/bandwidth; trained states are **almost rank-one in energy, but the output does not read the energetic direction** — content lives in a low-dim subspace NOT aligned with the state's coordinates, so energy-ranked (and even output-aware coordinate-wise) compression misses the directions that determine the input–output map; compress in the rotated, output-read basis. This is the riir-neuron-db SST substrate (`sst_project`/`factor_tail`, protected-anchor + tail-span anatomy) externally re-derived on Mamba-2 states: **energy ≠ content; the compressor must sit in the subspace the consumer reads, and coordinate-wise scoring cannot find it.** League-adjacent: Mamba-2/GDN is our decode family (qwen3.8 GDN layers; the Bonsai ternary league model). Train-side compression research; the rotate-then-compress face is the citation at the ndb/train lanes.

**3. DCKV (2610.36950)** — hybrid LMs keep a KV cache only in their few full-attention layers; DCKV is a **differentiable, end-to-end trained codec for exactly that prompt cache** — transform, quantizer, and entropy models (conditioned on prompt tokens + decoded neighbors) optimized jointly against the FROZEN LM on code length AND output change; >55× compression on Qwen3.5-2B. The KV-compression family (walk-18's ZipFusion lossless-register-reconstruct + walk-17's tile-decompression watch rows) gains its TRAINED member. Direct shape-match to our stack: the qwen3.8/GDN decode path is a hybrid LM whose full-attention prompt KV is the exact object DCKV compresses; league-family model named. Train-side codec research; consumption-side format faces are the lane citation (riir-infer read-only; any adoption is the freeze/thaw sidecar class).

**4. Folding Attention Biases into Keys (2610.36520)** — Attention Matching compacts KV caches by keeping keys per head, refitting values, and adding a per-key LOGIT BIAS — a format standard kernels (FlashAttention, vLLM paged) cannot consume. The fix: fold the bias into the keys along one closed-form per-head direction (fitted on context queries), giving a **plain cache with standard head geometry that unmodified vLLM serves** — error bounded by each query's projection on that direction; matches HF answers on Qwen3-4B, 180/200 greedy on Qwen3-14B LongBench. The deployment law: **a compressed format that requires kernel changes doesn't deploy; fold the deviation into the existing input surface.** The same law our format gates encode (lossy-surface promotion = deployed-path behavior; the fused-dequant family owns the weight-side shape). The closed-form per-head fold direction is the transferable atom.

**5. DFIP (2610.36804)** — decision models (PPLX-Decider, **Liquid d1-3B, Clef-Flash, GLiNER2.5-Decide** — our comparison-lane family, measured as a class) retain host dispatch, GPU-launch overhead, and full-vocabulary projection; DFIP applies selective CUDA-graph replay, **candidate-token projection** (project only the option tokens' logits), and schema-compatible length-aware batching at model boundaries: **2.47× short-Boolean resident speedups over strengthened native baselines**. Board context for the decision-index lanes (rethink decision-index-beat): the systems tier of the same board we score on; the candidate-token-projection law is the transferable atom (a decision model never needs the full-vocab projection). MEASUREMENT posture: informs how the comparison lanes' latencies should be read (strengthened-native baselines, resident-request comparisons) — our perm-probe/box-state discipline already encodes the honesty half.

**6. Evolving Agent Teams (2610.36848)** — coding agents converge to LOCAL OPTIMA on expert-written kernels (FlashInfer named); the framework: a central supervisor that **diagnoses each local optimum and restructures the agent team**, grounded in a persistent store (learning document + evolution history + per-team artifacts) under a matched token budget. The riir-refine self-evolve program's kernel-agent sibling: our LatentSkillEvolution/trajectory store/tier-guard + the corpus-window program (B298) are the same shape (accumulated experience grounding restructuring). The **diagnose-then-restructure law** (a stuck optimizer needs a structural intervention, not more iterations) is the healer improvement-queue citation; the persistent-store face is the usage-artifact intake's external confirmation.

### B (4)

- **2610.36587 Structured Subspace Randomness for SR** (quant/train) — per-element stochastic rounding saturates accelerator vector units (random gen lags matrix units); **sharing SR thresholds across tensor elements** removes the cost but adds a common rounding offset that degrades training through a specific channel; structured subspace randomness resolves it. Train intel; walk-18's error-diffusion row's sibling (the training-state budget family).
- **2610.36576 Less is More: Learning What to Profile** (gpu-kernel) — more profiling metrics ≠ better kernel optimization (non-monotone); the useful profile varies per kernel; kernel-conditioned profiling selection learned. Measurement-doctrine adjacent (our timed-region/loud-zero work owns the which-regions-to-instrument face); the kernel-conditioned-selection law is the gpu-kernel citation.
- **2610.36434 JustGoal** (gpu-kernel) — a goal-ONLY agent (no domain knowledge, no workflows, builds its own eval loop) is **above the expert-kernel baseline** as foundation models strengthen. The corpus-window thesis question ("is the corpus the value?") gains its external datum; the refine healer's corpus+draft pattern should read this as the counter-hypothesis to over-instrumentation. Doctrine row for the self-evolve lane.
- **2610.36153 Stratum** (plain-KV) — coding-agent repository memory: persistent, experience-aware memory beyond context; **storing knowledge in the KV cache costs 10^4–10^5× more bytes than indexed text**. The agent-memory economics law; refine's memory surfaces (trajectories, misses queues, active-set) already live as indexed stores — external confirmation of the posture.

### B− / doctrine / lane notes (5)

- **2610.36873 RangeRectQuant** (quant) — quantization for downstream COMPUTATION: pixel ranges preserved while every future rectangular SUM (4 prefix sums) stays within budget; error-interval feasibility characterized per row position. The quantize-for-reductions atom is the determinism family's data-format sibling; scientific-raster domain, no in-tree consumer.
- **2610.36636 Agar** (cs-PF) — microservice what-if analysis: per-component models predict BOUNDS not point values, so prediction error doesn't accumulate combinatorially. The bounds-compose law is UQ-doctrine adjacent (interval outputs compose where point predictions error-amplify); systems domain.
- **2610.36564 DeepInfer-Mac** (cs-PF) — 750B-class DeepSeek-V4.1-Flash on a 24 GB laptop: 7.6 GiB resident, experts load on demand, **reads fill reusable GPU buffers directly**, cache allocation coupled to buffer lifetime; 0.21–0.72 tok/s. Extreme-offload lane intel; league-family model named.
- **2610.36370 IOSage** (cs-PF) — HPC I/O bottleneck triage: a trained classifier decides WHICH patterns the log contains; the LLM only explains them (a single log gives an LLM no abnormality baseline). The decide-then-explain split is the modelless-first posture's serving sibling.
- **2610.36355 WAKMan** (plain-KV) — workload-aware KV eviction: agentic steps return in seconds and favor recency; popular prompts/long conversations favor frequency; **no single policy wins across 20 mixes**. The mix-sensitive eviction law; neuron-db tier-policy adjacent.

## Conversion staging (for the next claimant's batch — dedup + GOAT owed there, never here)

- **God Does Not Play Dice (2610.36623)** — the batch-invariance contract/validation face, cited beside Issue 940's reduction-order law if a kernel_opt batch owns a reproducibility-class rule; the 19/83 artifact census is the "why execute, not read" citation.
- **Balanced State Subspaces (2610.36617)** — the energy-vs-content basis split, cited at the ndb SST lane (`sst_project`/`factor_tail` anatomy) and riir-train's state-compression research; no corpus face (a compression-strategy citation, not a code-fix pattern).
- **DCKV (2610.36950)** — the hybrid-LM prompt-KV codec face routed to riir-infer/riir-train lane intel; freeze/thaw sidecar class if ever adopted.
- **Folding Biases (2610.36520)** — the fold-into-standard-inputs law cited at the format/gate families (lossy-surface promotion's deployment face).
- **DFIP (2610.36804)** — decision-serving intel for the rethink/reflex lanes (candidate-token projection; strengthened-native baseline posture).
- **Evolving Agent Teams (2610.36848)** — the diagnose-then-restructure law into the healer improvement-queue posture (post-mining intake framing).

## Corpus-delta verdict — ZERO (eleventh consecutive)

**NONE.** Every graded row is lane-intel, doctrine citation, or routing note; no rust_perf/kernel_opt/sec rule candidate carries a bounded Rust fix-space from this window. Closest calls: the batch-invariance validation face (2610.36623 — a validation harness shape, not a code-fix pattern; our kernels are hand-rolled) and DCKV's codec (train-side, format-level). The window's yield is lane-rich, corpus-empty — 2610.36623, 2610.36617 and 2610.36520 are the three rows most worth citing at their lanes.

## Dispositions table (graded rows)

| ID | Grade | Lane(s) | Disposition |
|---|---|---|---|
| 2610.36623 | B+ | gpu-kernel/determinism | POOLED — Issue-940 external confirmation; conversion staged |
| 2610.36617 | B+ | quant/state-subspace | POOLED — SST anatomy external confirmation; ndb/train citation |
| 2610.36950 | B+ | quant + plain-KV | POOLED — hybrid-LM KV codec; riir-infer/train lane intel |
| 2610.36520 | B+ | plain-KV | POOLED — fold-into-standard-inputs deployment law |
| 2610.36804 | B+ | cs-PF/decision-serving | POOLED — comparison-family systems row; rethink/reflex intel |
| 2610.36848 | B+ | gpu-kernel/agent-evolution | POOLED — self-evolve sibling; healer-queue citation |
| 2610.36587 | B | quant/train | POOLED — SR structured-subspace train intel |
| 2610.36576 | B | gpu-kernel | POOLED — kernel-conditioned profiling-selection law |
| 2610.36434 | B | gpu-kernel | POOLED — goal-only datum; corpus-thesis counter-hypothesis |
| 2610.36153 | B | plain-KV/agent-memory | POOLED — indexed-text-over-KV economics law |
| 2610.36873 | B− | quant | POOLED — quantize-for-reductions atom |
| 2610.36636 | B− | cs-PF/doctrine | POOLED — bounds-compose law |
| 2610.36564 | B− | cs-PF | POOLED — extreme-offload lane note |
| 2610.36370 | B− | cs-PF/lane | POOLED — decide-then-explain split |
| 2610.36355 | B− | plain-KV | POOLED — mix-sensitive eviction law |
| 2610.36844 | skip | vision/COD quant | COUNTED |
| 2610.36836 | skip | recommenders | COUNTED |
| 2610.36720 | skip | FPGA/physics trigger | COUNTED |
| 2610.36452 | skip | robotics tokenization | COUNTED |
| 2610.36421 | skip | math-ph (B-S quantization) | COUNTED |
| 2610.36387 | skip | math (homological) | COUNTED |
| 2610.36385 | skip | physics (cavity QNM) | COUNTED |
| 2610.36373 | skip | optics fabrication | COUNTED |
| 2610.36340 | skip | vision bench (crowd counting) | COUNTED |
| 2610.36327 | skip | survey (video VLM) | COUNTED |
| 2610.36199 | skip | physics (3D QH) | COUNTED |
| 2610.36135 | skip | physics ML (Hamiltonian) | COUNTED |

**NOTHING INVENTED** — every graded row read from its fetched API abstract this session (title + abstract captured for all 27 in the same search responses); grades are this lane's adjudication, not measurements. Ephemeral scratch (`Temp/walk19/`) removed at close per the ephemeral law.

**Landing extras this walk:** (1) `.research/.highwater` found STALE at 338 while `339_issue186_structure_grounded_synth_fixtures.md` exists on disk — the 339 lane landed without the bump; this walk allocates 340 and writes the highwater to 340 (the incident recorded here). (2) Seed family continuity: `.distill/008_walk20_dedup_seed.txt` pre-staged TRACKED = 229 + this walk's 27 = **256 IDs** (the walk-18 lesson holds: the seed lives in git, never only in the working tree).
