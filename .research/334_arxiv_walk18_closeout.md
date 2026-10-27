# Research 334 — arXiv pool-refill walk #18 (closeout; the weekend announce batch)

**Status:** RECORD — pool refilled; **zero corpus delta at walk time (TENTH consecutive zero-delta walk)** — the window's yield is lane-rich (ternary mixed-tier sibling, determinism-routing external confirmation, spec-dec bytes-model with Lean-checked bounds), corpus-empty; verdict and the closest calls recorded below. Zero cargo, zero GPU throughout (amo; sibling lanes own the box — the i1132 census + the 645 training + the b295 batch build all live per the 02:1x reads).

**Filed:** 2026-10-27 ~02:2x–04:0x UTC, zed-amo-walk18/amo, riir-refine idle loop. Claim `11cf59a4` (epoch 1793057728) pushed 2026-10-26 23:35 UTC — BEFORE the 00:00Z window opened, per the walk-claim law; push is the arbiter; no counter-claim materialized.

**⚠ ROLL-TIMING DATUM (extends the walk-12 law; the Tue cycle's empirical answer):** the nominal announce is 00:00Z but the API index roll for the Tue cycle landed **between 01:45Z and 02:16Z** — probes at 00:07 / 00:20 / 00:35 / 01:01 / 01:15 / 01:45 UTC all read the pre-roll ceiling (cs.PF newest 2026-10-23T13:43:38Z, static across six reads, ~2.25 h), and the 02:16Z probe read the weekend batch (cs.PF newest 2026-10-25T19:10:07Z). Walk-17's "~1.5 h after the announce, settled" was read AT 01:4x–02:03Z, not measured earlier — this walk brackets it: **for a Tue-cycle grading, first useful poll ≈ 02:15Z; probes before ~01:45Z are dead reads.** Stable re-poll: quant lane 29 rows at 02:19Z == 29 at 02:22Z.

**Window:** submittedDate [2026-10-23T18:00Z → 2026-10-26T18:00Z] (the Fri-post-cutoff + weekend + Mon batch delivered by the Tue announce), five lanes over the search API (spec-dec / plain-KV / quant-broad / gpu-kernel / cs-PF), graded ~8.3–10 h after the window close (2026-10-26T18:00Z) and ~2.3–4 h after the 00:00Z announce — the roll settled between the 01:45Z and 02:16Z probes (see the roll-timing datum below), stable re-poll confirmed. IDs span 2610.33187 (10-23T18:29Z) → 2610.35023 (10-26T08:35Z) — no column shared with walk-17's graded set (its max was 2610.32981); ceiling discipline holds.

**Dedup:** clean — **54 fetched / 49 unique / 5 cross-lane dups counted once / 0 seed hits** (the 180-ID seed: Research 294+311 union + walk-17's 37). **Seed-file incident (recorded, repaired at landing):** the pre-staged `.distill/006_walk18_dedup_seed.txt` (180 IDs) was UNTRACKED scratch and vanished from the tree mid-walk (never tracked; some lane's cleanup — no git recovery). Rebuilt byte-equivalent from `.distill/004` (143) + walk-17's 37 graded IDs = 180 exactly. Durable fix shipped with this walk: the seed family is now TRACKED — `006` restored (walk-18's seed) + `007` pre-staged (walk-19's seed = 180 + this walk's 49 graded IDs = 229). Lesson: a seed that lives only in the working tree is one `git clean` away from a silent dedup hole.

**Lane profile (fetched):** quant 29 · plain-KV 13 · spec-dec 5 · cs-PF 6 · gpu-kernel 1 = 54 → 49 unique. **Graded: 3 B+ + 3 B + 15 B−/doctrine/lane notes + 28 skip-class** (vision ×3, physics/math-quant ×6, wireless/LEO ×2, VQ-VAE training, unlearning, SBI, tutorials/surveys ×2, NAS, optical flow, 3DGS, power flow, SAR, ECG, clinical deid, coding-agent ER, HPC bridging model, MoE policy emulation, voltage-scaling quant).

## Headline rows

### B+ (3)

| # | ID | Title | Lane | Record |
|---|---|---|---|---|
| 1 | **2610.34886** | ThinkBit: Search Codewords, Recalibrate Scales, Select by Reasoning | quant | **B+ lane record (the ternary lane's mixed-tier sibling)** |
| 2 | **2610.34721** | Prefetch-Steered Routing: Deterministic Cache-Aware Expert Selection for Offloaded MoE Decoding | spec-dec | **B+ (the determinism law as a routing design; RTX 4090 measured)** |
| 3 | **2610.34440** | Speculative Decoding for MoE Models: Expert Bytes, Kernel Bandwidth and the Best Draft Length | cs-PF + spec-dec | **B+ lane record (the acceptance-economics cluster's systems member; llama.cpp named)** |

**1. ThinkBit (2610.34886)** — strictly scalar two-bit weights: every weight is an **integer codeword × a bf16 group-shared scale**; a sensitivity metric assigns each layer a **ternary / 2-bit / 3-bit TIER** (mixed-tier allocation, ternary as the floor tier); a layerwise Gumbel-softmax codeword search optimizes block by block; scale recalibration follows; and the EVAL axis is reasoning-through-hard-problems, not ppl/MCQ ("both say little about whether a model can still reason"). League relevance DIRECT: our league model is ternary (PQ2_0); ThinkBit's tier law says the ternary tier is where insensitive layers want to live, with 2-bit reserved for sensitive ones — a per-layer mixed-tier format is the PQ2 lineage's graduated sibling. Riir-train/riir-infer read-only; any adoption is a weight-FORMAT change (freeze/thaw-compatible, BLAKE3-committed sidecar class). **Staged conversion note (dedup + GOAT at any claiming batch, never here):** the sensitivity-tier-assignment face and the codeword-search face are TRAIN-side (Gumbel-softmax); the mixed-tier FORMAT + reasoning-eval axis are the lane citations.

**2. Prefetch-Steered Routing (2610.34721)** — MoE offload decode: cache-aware routing + async prefetch makes outputs depend on TRANSFER TIMING, so a harness that doesn't simulate timing no longer evaluates the deployed model. PSR routes on a **DETERMINISTIC LOGICAL CACHE** (≤n experts per layer/token inserted by prediction; "physical copies affect only latency") — 2.2–2.3× vs exact routing + LRU, 25–30% vs Cache-Prior, on OLMoE-1B-7B with 50%/25% resident **on an RTX 4090**. This is the workspace's determinism discipline (raw sync, pinned reduction order, replay-identical) re-derived as a routing design: **the evaluated object must be a function of logical state, never of physical timing.** External confirmation, systems-grade, our league box named. No in-tree MoE-offload lane — doctrine citation + the info-brain/think-brain parallel recorded.

**3. Spec-Dec for MoE (2610.34440)** — consecutive tokens often select the same experts; a **deduplicating engine reads each selected expert once per PASS**, a per-slot engine once per token; step time = requested bytes ÷ effective bandwidth + fixed cost; under geometric acceptance: when speculation wins, and the **best draft length is a monotone staircase in the acceptance rate**; bandwidth-error bounds on each result; **main steps checked in Lean**; measured on **llama.cpp Metal, MLX-LM, llama.cpp CUDA** (modified to deduplicate). The acceptance-economics cluster (walk-16's center-of-mass) gains its systems member, and the opponent's codebase is the measurement bench. Detection-only (our spec-dec lane is closed-negative), but the dedup-bytes law is the transferable content.

### B (3)

- **2610.34824 SPARTA** (plain-KV, cs.AR) — sparse **ternary** LLM accelerator for long-context edge: phase-aware sparse dataflow (heterogeneous sparsity across compute-bound prefill vs memory-bound decode), a **unified compute fabric mapping ternary SpMM/SpMV AND bit-serial attention onto shared processing elements** (zero-weight + zero-bit sparsity), adaptive two-stage attention (page-level Top-k on K-cache + token-level Top-p pruning). The ternary league's hardware sibling: the unified-datapath and phase-aware-sparsity atoms are design-law citations for the ternary lanes; silicon itself is out of scope.
- **2610.33719 ZipFusion** (plain-KV, cs.AR) — **attention consumes the LOSSLESSLY COMPRESSED KV cache directly**: kernel reconstructs original values **in registers using simple bit operations** — no decompression pass, no uncompressed restore; compression format and attention kernel co-designed to cut bytes read during decode. This is the dequant-in-the-consumer family's KV instance — the same shape our corpus owns for weights (the fused-quantize family, the trellis/LUT decodes) and walk-17's tile-decompression watch row named for query engines. WATCH-grade: no lossless-KV lane exists in-tree (riir-infer's KV work is laya/GDN); the operator-boundary framing is the citation if that lane ever opens.
- **2610.34915 End-to-End MXFP8 RL on Blackwell** (plain-KV+quant) — sampler/learner numerics agreement, measured: **a sampler on the LEARNER'S FP8 GRID is more consistent than a higher-precision BF16 sampler** (train-rollout KL 11–16% lower at fixed weights; synchronized weights byte-identical to the learner's FP8 forwards). This CONFIRMS walk-17's rounding-location watch row (2610.32951, "Where Rounding Happens Matters") as a measured law: numerics agreement beats precision. Doctrine-adjacent to the Issue-940 determinism law (pinned reduction/order beats accidental precision); riir-train read-only TRAIN INTEL.

### B− / doctrine / lane notes (15)

- **2610.34731 MCF** (plain-KV) — chunked-prefill KV handling: selectively replicate highly-reused blocks while memory headroom exists, progressively drain replicas into the sharded decode-ready layout as pressure grows (no prefill→decode boundary conversion); decode is query-centric (KV shards stationary, queries routed). The replicate-then-drain layout-transition law; KV-systems lane intel.
- **2610.35023 BellowsKV** — cross-layer KV sharing: adjacent layers' KV representations differ, but their INPUT hidden states are highly similar → one latent for many layers, **expanded on demand**. The latent-expanded-at-consumer atom; KV-lane intel, no in-tree host.
- **2610.35020 Bounded Key-Query Similarity** — space complexity of attention approximation UNDER ANISOTROPY (real transformers' key/query cosine similarities stay small; worst-case bounds assume perfect alignment). The theory-grade grounding for why KV compression works in practice; doctrine note for the cosine-geometry lanes (embed/routing).
- **2610.34907 TMA characterization** (gpu-kernel) — workload-grounded: WHICH operands in FlashAttention-3 forward use TMA vs cp.async vs DSM vs cluster multicast, and the discriminating features. Hopper-class (detection-only for SM89/Metal); the operand-level async-copy choice law is the gpu-kernel lane citation.
- **2610.34436 RIPE Caching** (plain-KV+spec-dec) — looped LMs (Huginn/Raven): prefix K/V planes matter LATE (loops 25–32 are e^15/e^12.8 × more sensitive than 2–9; planes approach the last geometrically, 0.88/0.81 per loop) → store the deepest plane everywhere except a full-depth recent window. Depth-sensitivity-graded caching; looped-LM-specific, no host.
- **2610.34103 T2D** (spec-dec) — draft models constructed FROM the target via latency-aware structured pruning + distillation, jointly optimized for alignment AND measured latency. Train-side draft construction; lane note.
- **2610.34566 Error-Diffusion Binary Weights** (quant) — training binary-weight LMs with a 2–8-bit stochastic-rounding residual buffer instead of a full-precision latent (97 → 3–9 bits/weight, 19×); the halftoning error-diffusion loop moved onto the weight-update path, with a **threshold-alignment rule** (works only when the grid has a level exactly at the flip threshold). Train intel; the threshold-alignment atom is the quant-grid vocabulary's training-side sibling.
- **2610.33625 RAQ-MLA** (plain-KV+quant) — MLA caches two tensors with different ROLES (latent supplies values; positional key dominates scores) → role-aware per-tensor quantization. MLA-specific (our league model is not MLA); lane intel.
- **2610.33501 DualPace** (plain-KV) — the host becomes an independently scheduled decode engine (AMX CPU batches + advances CPU-resident decodes; scheduling authority decoupled from the GPU engine). Serving-systems lane note.
- **2610.33923 Queuing for Inference** (cs-PF) — dynamic memory footprints and the cost of preemption in inference queues. Serving-systems lane note.
- **2610.33379 Packed-SS** (spec-dec) — self-speculative + diffusion drafting in ONE model call, input linear in the speculative width (quadratic avoided). The scheduling face is neat; diffusion drafting stays off-lane (walk-16's lossless-AR vs lossy-diffusion doctrine row) and our tree-verify lane is closed-negative. NOTE-ONLY.
- **2610.33298 All-in-One composable attack** (quant/security) — models trained to pass safety checks pre-transform and turn malicious only after quantization/pruning/merging, with ONE jointly-trained trigger across transformations. The **lossy-surface promotion rule's threat-model face**: a transformation is not neutral to the artifact's behavior — external confirmation for "deployed-path behavior, not bit-identity" as the gate. Watch-only (model-security, not code-security).

## Conversion staging (for the next claimant's batch — dedup + GOAT owed there, never here)

- **ThinkBit (2610.34886)** — the mixed-tier ternary FORMAT faces (sensitivity-tier assignment; codeword × group-scale scalar decode; reasoning-axis eval) routed to the ternary lanes; the tier law is the PQ2 lineage's graduated sibling. Train-side faces (Gumbel-softmax codeword search) stay riir-train.
- **PSR (2610.34721)** — the deterministic-logical-cache law: any future MoE/offload/prefetch lane must make outputs a function of logical state only; doctrine citation beside the determinism law (Issue 940) + the two-brain model.
- **Spec-Dec for MoE (2610.34440)** — the dedup-bytes law + draft-length staircase: the acceptance-economics cluster's systems member; cite if the spec lane re-opens (llama.cpp is the bench AND the opponent).
- **ZipFusion (2610.33719)** — the lossless-KV-consumed-directly face joins walk-17's tile-decompression WATCH row at the dequant-in-the-consumer family; both wait for a lossless-KV lane to exist in-tree.

## Corpus-delta verdict — ZERO (tenth consecutive)

**NONE.** Every graded row is lane-intel, doctrine citation, or routing note; no rust_perf/kernel_opt/sec rule candidate carries a bounded Rust fix-space from this window. Closest calls: ZipFusion's register-reconstruct attention (the fused-quantize family owns the weight-side shape; a KV-side rule has no in-tree consumer lane) and ThinkBit's tier-assignment (a TRAIN-side search, not a code-fix pattern). The window's yield is lane-rich, corpus-empty — ThinkBit, PSR and the MoE spec-dec bytes-model are the three rows worth citing at their lanes.

## Dispositions table (graded rows)

| ID | Grade | Lane | Disposition |
|---|---|---|---|
| 2610.34886 | B+ | quant/ternary-format | POOLED — riir-train/riir-infer ternary lane; conversion staged |
| 2610.34721 | B+ | spec-dec/determinism | POOLED — doctrine citation (determinism law external confirmation) |
| 2610.34440 | B+ | cs-PF + spec-dec | POOLED — acceptance-economics systems member |
| 2610.34824 | B | plain-KV/hardware | POOLED — ternary-lane design-law citation |
| 2610.33719 | B | plain-KV | POOLED — dequant-in-consumer family watch row |
| 2610.34915 | B | quant/train | POOLED — TRAIN INTEL; walk-17 watch row CONFIRMED |
| 2610.34731 | B− | plain-KV | POOLED — replicate-then-drain layout law |
| 2610.35023 | B− | plain-KV | POOLED — cross-layer latent KV note |
| 2610.35020 | B− | plain-KV/theory | POOLED — anisotropy grounding note |
| 2610.34907 | B− | gpu-kernel | POOLED — TMA operand-choice law |
| 2610.34436 | B− | plain-KV+spec-dec | POOLED — looped-LM depth-sensitivity note |
| 2610.34103 | B− | spec-dec | POOLED — draft-construction lane note |
| 2610.34566 | B− | quant/train | POOLED — error-diffusion training state note |
| 2610.33625 | B− | plain-KV+quant | POOLED — MLA role-aware note |
| 2610.33501 | B− | plain-KV | POOLED — serving-systems note |
| 2610.33923 | B− | cs-PF | POOLED — preemption-cost note |
| 2610.33379 | B− | spec-dec | NOTE-ONLY — single-call self-spec |
| 2610.33298 | B− | doctrine | POOLED — lossy-surface threat-model citation |
| 2610.34890 | B− | lane | NOTE-ONLY — agentic deployment (healer-adjacent framing only) |
| 2610.34675 | B− | lane | NOTE-ONLY — provenance/freeze-lineage adjacency |
| 2610.34135 | B− | lane | NOTE-ONLY — compute-once-per-context instance (wireless) |
| 2610.34241 | skip | coding-agent ER | COUNTED |
| 2610.33404 | skip | MoE emulation | COUNTED |
| 2610.34993 | skip | vision+quant incidental | COUNTED |
| 2610.34991 | skip | speech | COUNTED |
| 2610.34828 | skip | VQ-VAE training | COUNTED |
| 2610.34742 | skip | remote sensing | COUNTED |
| 2610.34703 | skip | SBI | COUNTED |
| 2610.34585 | skip | tutorial | COUNTED |
| 2610.34509 | skip | power flow | COUNTED |
| 2610.34495 | skip | unlearning | COUNTED |
| 2610.34479 | skip | metasurface | COUNTED |
| 2610.34318 | skip | physics | COUNTED |
| 2610.34304 | skip | edge vision | COUNTED |
| 2610.34263 | skip | 3DGS | COUNTED |
| 2610.34125 | skip | math-ph | COUNTED |
| 2610.34122 | skip | math-ph | COUNTED |
| 2610.34063 | skip | math-ph | COUNTED |
| 2610.33995 | skip | HPC theory | COUNTED |
| 2610.33964 | skip | ECG | COUNTED |
| 2610.33932 | skip | SAR | COUNTED |
| 2610.33839 | skip | clinical deid | COUNTED |
| 2610.33824 | skip | voltage-scaling quant | COUNTED |
| 2610.33800 | skip | quantum | COUNTED |
| 2610.33360 | skip | looped-LM training | COUNTED |
| 2610.33358 | skip | NAS | COUNTED |
| 2610.33338 | skip | optical flow | COUNTED |
| 2610.33475 | skip | cloud survey | COUNTED |
| 2610.33187 | skip | cosmology | COUNTED |

**NOTHING INVENTED** — every graded row read from its fetched API abstract this session (title + abstract captured for all 49; full abstracts pulled in the same search response); grades are this lane's adjudication, not measurements. Ephemeral scratch (`/tmp/walk18/`) removed at close per the ephemeral law.
