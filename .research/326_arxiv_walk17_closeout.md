# Research 326 — arXiv pool-refill walk #17 (closeout; the Fri-cutoff announce batch)

**Status:** RECORD — pool refilled; **zero corpus delta at walk time (NINTH consecutive zero-delta walk)** — lane-intel rich this window (ternary-lane format sibling, KV-tier systems cluster, spec-dec acceptance economics all moved), but no rule candidate carries a bounded Rust fix-space; verdict and the closest calls recorded below. Zero cargo, zero GPU throughout (M3; sibling lanes hot — the amo capture + healer install live per its 06:4x cycle row).

**Filed:** 2026-10-26 ~07:0x–08:5x +07, zed-m3-walk17/M3, riir-refine idle loop. **Re-claim discipline:** the walk was claimed by the shikuwa/4090 lane (10-25 ~04:3x, expiry 10-27 ~04:3x) with execution "at/after due 07:00 +07"; the holder was **provably offline at its own due window** — 4090 unreachable from BOTH sibling boxes this morning (this lane: LAN + Tailscale timeouts 06:4x +07; the amo lane: `Permission denied (publickey,password)` auth rejection, its 06:4x cycle row) — so the posted re-claim `7e2999eb` fired per the 10-25 claim row's own handoff provision ("graded … or handed off via a posted re-claim"); push is the arbiter; the 4090 lane on revival yields per the same protocol it wrote.

**⚠ WINDOW CORRECTION (empirical, supersedes the opening-balance row's arithmetic):** the 10-25 ~00:2x row wrote the window as `submittedDate:[202610231800 TO <poll>]` reading 10-23 as Thursday — but **10-23 is FRIDAY** (Mon = 10-26), so the Fri 14:00 ET cutoff that the Mon 00:00Z announce delivers is `submittedDate [202610221800 → 202610231800]` (Thu-eve + Friday). Proven empirically at 08:2x +07: band [10-22T18:00Z → 10-23T18:00Z] returned **882 total submissions (111 cs.LG)** post-announce while band [10-23T18:00Z → 10-24T18:00Z] returned ZERO (Sat/Sun submissions wait for the Tue 00:00Z announce — outside this session, the next walk's opening balance, the standing disclosed-partial pattern). The opening-balance row's start would have **skipped this entire announce batch**. Walk-16's correction row ("effective graded ceiling Thu-18:00Z" = 10-22T18:00Z) was consistent with the corrected reading all along.

**Window:** submissions 2026-10-22T18:00Z → 2026-10-23T18:00Z (the Fri-cutoff announce batch), five lanes over the search API (spec-dec / plain-KV / quant-broad / gpu-kernel / cs-PF), graded ~36 h after the window close, ~1.5 h after the announce, index roll settled (stable re-poll at 08:4x).

**Dedup:** clean — **23 fetched / 23 unique / 0 cross-lane duplication beyond the two 2-lane hits counted once (2610.32243 quant+gpukernel, 2610.31827 plainkv+cspf)**, **zero overlap with Research 294 + 311** (143-ID seed set extracted from both notes and grep-verified). Ceiling discipline: walk-16's oldest fresh row was 2610.27438+; this walk's rows are 2610.318xx–2610.324xx — no column shared with any prior walk's graded set.

**Lane profile (fetched):** spec-dec 2 · plain-KV 5 · quant 12 · gpu-kernel 2 · cs-PF 4 = 23 unique. **Graded: 2 B+ + 3 B + 3 B− + 3 doctrine notes + 2 lane notes + 10 skip-class** (vision, optimizer theory, IoT/LoRaWAN, math/physics-quant ×5, dLLM editing, KV-attribution cognitive study, HPC scheduling, robot-demo segmentation).

## Headline rows

### B+/lane-HIGH (2)

| # | ID | Title | Lane | Record |
|---|---|---|---|---|
| 1 | **2610.31931** | Every Weight Is a Path of Shared Discrete Decisions (TWIG) | quant | **B+ lane record (the ternary lane's format-level sibling)** |
| 2 | **2610.32243** | BiLoCo: Binary Low-Rank Corrections for Post-Training Quantization | quant + gpu-kernel | **B+ lane record (concurrent-lane correction composition; the SST substrate's ±1 sibling)** |

**1. TWIG (2610.31931)** — every weight is the end of a path of small discrete decisions: **a sign, a shift by a power of two, a zero gate, an index into a shared table** — and weights that belong together **share the upper part of their paths, each decision stored once and applied once**. Three consequences: (a) powers-of-two decision trees make weight layers circuits of wires-and-adders (multiplier-free dequant — fixed-weight chips need no constant multipliers); (b) pruning, quantization, low-rank, codebooks and weight sharing become **edits of one object** — one optimizer chooses among all of them network-wide; (c) choosing the decisions = the ground-state problem of a higher-order Boltzmann machine (annealer/quantum-optimizer class). League relevance DIRECT: our league model is ternary (PQ2_0); the {sign, pow2-shift, gate, codebook-index} decision set is the ternary weight vocabulary generalized to a shared-prefix tree — the trie-compression of the decision space is the novel storage atom. Riir-train/riir-infer read-only; any adoption is a weight-FORMAT change (freeze/thaw-compatible, BLAKE3-committed sidecar class). **Staged conversion note (dedup + GOAT at any claiming batch):** the shared-path inheritance + pow2-shift multiplier-free dequant faces, routed to the ternary lanes — the corpus's kernel_opt quantization-decode faces own the compute geometry, a weight-encoding owner is expected absent.

**2. BiLoCo (2610.32243)** — SVDQuant's high-precision low-rank FP4 correction re-stored as **binary {±1} sign-vector factors**: 16× less storage per rank-1 component, the correction becomes additions/subtractions runnable on **CUDA cores concurrent with the FP4 tensor-core GEMM in the same launch**, combined in the GEMM epilogue. Measured: matches-or-improves memory-matched SVDQuant on 4B–72B; B300 fused single kernel ≤11% slower than FP4-only GEMM at batch 1 up to rank 512. Three atoms: (a) **binary-factor correction** = the riir-neuron-db SST substrate's ±1 sibling (`sst_project`/`factor_tail` ship ΔW = U_⊥CV_⊥ᵀ; a ±1-factor variant makes the correction multiplier-free and quantization-safe by construction — the tail span closes under sign-flips exactly as it does under the product); (b) the **same-launch heterogeneous-lane composition** (bit-packed ALU work concurrent with tensor-core GEMM, epilogue combine) — the corpus's fusion/concurrent-lane families' composition face, rider-grade at a claiming batch at most; (c) B300/SM100 silicon disclosed — league-adjacent, not our SM89/Metal class; the epilogue-combine composition transfers as a rule shape regardless.

### B (3)

| # | ID | Title | Lane | Record |
|---|---|---|---|---|
| 3 | **2610.32057** | HyScale: Scaling Long-Context Training for Learned Sparse Attention | plain-KV + gpu-kernel | **B (selection-output-is-indices + selection replay)** |
| 4 | **2610.31827** | KVStream: Adaptive Parallel Storage Management for KV Caching | plain-KV + cs-PF | **B (the aggregate-bandwidth-scheduling law; the SSD-KV tier cluster's systems member)** |
| 5 | **2610.31840** | SPECMEM: Accelerating Agentic Tool Calling via Live Memory Management | spec-dec | **B (the live-store law + the side-effect gate)** |

**3. HyScale (2610.32057)** — learned-sparse-attention training's bottleneck is SELECTION (scores every previous token; sparse attention reads only k); current systems materialize a full score table AND repeat selection during recomputation. Key insight: **"the output of selection is not an activation, but rather a set of indices"** — HyTopK finds exact top-k on the fly without the score table; **selection replay reuses forward-pass selection in recomputation**. GLM-5.1 1M-token training 2.07× over Megatron-LM on 512 H20; **DeepSeek-V4-Flash CSA +2.01×, Qwen3.8-Flash-Next QSA +1.57× at 1M on 32 H800** (league-family models named). The index-set law is corpus-grade adjacent to the sparse-attention family; the replay atom is the transactional-family sibling (walk-16's SubTernary KV-rollback grade).

**4. KVStream (2610.31827)** — prefix retrieval recast as **aggregate-bandwidth scheduling across storage tiers**: batch a step's reads, defer writes behind the forward pass, replicate each chunk on every tier, let each tier **adaptively claim the next unread chunks once free** — attains the aggregate-bandwidth bound **without prediction or tuning**, while any static split misses it by a margin that GROWS as per-tier bandwidths drift. TTFT gain grows with load, disappears when host memory keeps reuse in cache. Within 5% of the bound on LMCache integration (up to 128 GPUs); 1.49× offline throughput, 6.1× mean TTFT under bursty arrivals. The walk-16 SSD-KV cluster's (Gandalf/SARP/SARIA/RISK-KV) systems-grade member — neuron-db Warm/Cold tier + turso-cold design lane intel; the adaptive-claim-over-static-split law is the transferable decision content.

**5. SPECMEM (2610.31840)** — speculative decoding for tool-calling agents with a **LIVE draft store**: a frozen (build-once) store steadily loses acceptance — measured **27% gap in mean accepted tokens** (+11–34% across benchmarks, tracking post-build novelty; 1.74× wall-clock). Verified calls written back online into bounded per-user LRU stores; the **side-effect gate** — gating on DECLARED side effects rather than retrieval confidence — is what eliminates irreversible mistakes; per-user scoping adds no acceptance but closes a measured cross-user leakage channel. The acceptance-economics cluster (walk-16's named center-of-mass move) gains its systems member; the side-effect/consent-gate atom is doctrine-adjacent to our contribution-consent + freeze/thaw posture (the irreversibility axis, not the acceptance axis, is what's gated).

### B− (3) + doctrine notes (3)

- **2610.32127 MPQ** (quant) — commit-late law: GPTQ/LDLQ/BCD commit each weight block to a single codeword before its neighbors are considered; MPQ preserves soft blockwise posteriors (VAMP) and commits hard only at the output stage; improves ppl + zero-shot at 2-bit lattice / 3–4-bit scalar on Llama/Qwen/Mistral. Quantization-solver lane intel.
- **2610.31911 LazyFlash** (plain-KV) — forward-dense, backward-sparse fine-tuning: gradient-aware block sparsification ranks query blocks by attention-output gradient norms, a gradient-mass budget adapts skipping per head/step, importance sampling preserves the dense gradient in expectation; skipped blocks never loaded in the tiled FA backward kernel. The "skipped-blocks-never-loaded" wall-clock atom is the corpus skip-class sibling; training-side.
- **2610.31897** (gpu-kernel) — structured sparsity for local/edge execution (learned sparsity bridging compressibility and efficient inference); thin decision content from the abstract; edge-deployment lane note only.
- **Doctrine 2610.32417** — quantized-model energy measured per-token misses agent-loop effects (quantization changes generation length + task success); the unit-of-measurement changes the verdict — external confirmation of the lossy-surface promotion rule's deployed-path-behavior law.
- **Doctrine 2610.31819** — speculative diffusion formalizes the two-tradeoffs axis: autoregressive spec-dec is lossless-by-verify; dLLM speedups are lossy-by-design — the acceptance/quality axis split our freeze/verify doctrine already encodes (G1-correctness-before-G2-perf).
- **Doctrine 2610.32245** — MDL compression ratio as an unsupervised vetting signal separating law-grade from statistical-grade agent discoveries (86% of a self-improving agent's admissions were statistical) — adjacent to the grammar-not-answers doctrine's "score the artifact, not the prose" posture; watch-only.
- **Lane note 2610.32229 ALTO** — MXFP4 MoE training recipe (GPT-OSS-20B, MLPerf Small) closing to BF16; riir-train read-only lane intel (the fp4/microscaling training posture).
- **Lane note 2610.32247 MetaDB** — filesystem-aware metadata DB (directory-tree locality, differentiated consistency, clustered deletes); storage-lane intel for the neuron-db/chain metadata surfaces; thin.

## Conversion staging (for the next claimant's batch — dedup + GOAT owed there, never here)

- **TWIG (2610.31931)** — the ternary weight-encoding faces (shared-prefix decision inheritance; pow2-shift multiplier-free dequant; shared-table index as codebook). Corpus dedup pre-check owed: the kernel_opt quantization-decode faces own the compute geometry; a weight-ENCODING owner is expected absent. League-posture note: any adoption is a freeze/thaw-compatible format change (BLAKE3-committed sidecar class), never a runtime mint (A8).
- **BiLoCo (2610.32243)** — the ±1-factor correction sibling of `factor_tail`/`sst_project` (ndb lane) + the same-launch heterogeneous-lane composition face (kernel_opt fusion families, rider-grade at most). B300 silicon disclosed; the composition shape transfers.
- **HyScale (2610.32057)** — the selection-output-is-indices law for any sparse-attention lane work (our `sparse-kv-index-list-sentinel-padded`/attention families own the sentinel/index side; verify the activation-vs-index framing is unowned before staging).

## Corpus-delta verdict — ZERO (ninth consecutive)

**NONE.** Every graded row is lane-intel, doctrine citation, or routing note; no rust_perf/kernel_opt/sec rule candidate carries a bounded Rust fix-space from this window. Closest calls: BiLoCo's concurrent-lane composition (rider-grade; the corpus's fusion families own the compose-form) and HyScale's index-set law (design law, not a code-fix pattern). The window's yield is lane-rich, corpus-empty — the ternary-lane format sibling (TWIG) and the SST ±1 sibling (BiLoCo) are the two rows worth citing at their lanes.

## Dispositions table (graded rows)

| ID | Grade | Lane | Disposition |
|---|---|---|---|
| 2610.31931 | B+ | quant/ternary-format | POOLED — riir-train/riir-infer lane; conversion staged (dedup+GOAT at claiming batch) |
| 2610.32243 | B+ | quant + gpu-kernel | POOLED — ndb SST-lane sibling + kernel_opt composition face; conversion staged |
| 2610.32057 | B | plain-KV + gpu-kernel | POOLED — selection-index law; league-family training numbers |
| 2610.31827 | B | plain-KV + cs-PF | POOLED — neuron-db tier-design lane; adaptive-claim law |
| 2610.31840 | B | spec-dec | POOLED — acceptance-economics cluster + side-effect-gate doctrine |
| 2610.32127 | B− | quant | POOLED — solver lane intel |
| 2610.31911 | B− | plain-KV | POOLED — training-side sparsity |
| 2610.31897 | B− | gpu-kernel | POOLED — edge sparsity note |
| 2610.32417 | B− | doctrine | POOLED — cite beside the lossy-surface rule |
| 2610.31819 | B− | doctrine | POOLED — freeze/verify doctrine external confirmation |
| 2610.32245 | B− | doctrine | POOLED — watch-only (grammar-not-answers adjacency) |
| 2610.32229 | B− | lane | NOTE-ONLY — riir-train MXFP4 recipe intel |
| 2610.32247 | B− | lane | NOTE-ONLY — storage-lane intel |
| 2610.32465 | skip | optimizer theory | COUNTED |
| 2610.32473 | skip | vision | COUNTED |
| 2610.32438 | skip | IoT | COUNTED |
| 2610.32400 | skip | math | COUNTED |
| 2610.32030 | skip | physics-quant | COUNTED |
| 2610.31839 | skip | physics-quant | COUNTED |
| 2610.32096 | skip | dLLM editing | COUNTED |
| 2610.32109 | skip | KV-attribution | COUNTED |
| 2610.32027 | skip | HPC scheduling | COUNTED |
| 2610.31801 | skip | robot segmentation | COUNTED |

**NOTHING INVENTED** — every graded row read from its fetched API abstract this session (full abstracts pulled for every B+/B row); grades are this lane's adjudication, not measurements. Ephemeral scratch (`.raw/walk17/`) removed at close per the ephemeral law.
