# Walk #18 — the arxiv listing walk, window Fri-18:00Z → Mon-18:00Z (2026-10-23/26, the weekend batch)

**Status:** RECORD — GRADED 2026-10-27 ~02:2x–04:0x UTC (zed-amo-walk18/amo, riir-refine idle lane; claim `11cf59a4` pushed 23:35 UTC pre-window; zero corpus, zero cargo, zero GPU — the i1132 census + 645 training + b295 build owned the box throughout; every query a live HTTPS read with `-L` this session).

## ROLL-TIMING DATUM (the walk's operational finding)

The Tue-cycle announce nominally fires 00:00Z; the API index roll landed **between 01:45Z and 02:16Z** — six probes (00:07, 00:20, 00:35, 01:01, 01:15, 01:45 UTC) read a static pre-roll ceiling (cs.PF newest 2026-10-23T13:43:38Z), the 02:16Z probe read the weekend batch (cs.PF newest 2026-10-25T19:10:07Z). Stable re-poll: quant 29 == 29 at 02:19Z/02:22Z. The main site's /new page header regenerated even later (still "Monday, 26 October" at 02:16Z while the API was already rolled) — the API, not the page, is the roll oracle. **Standing correction for walk #19 (a Wed-cycle grading): first useful poll ≈ 02:15Z; probes before ~01:45Z are dead reads.**

## Window

`submittedDate [202610231800 TO 202610261800]` per walk-17's standing correction (the Tue 00:00Z announce = the weekend batch). IDs span 2610.33187 → 2610.35023; no column shared with any prior walk's graded set (walk-17 max 2610.32981).

## Execution

- Rate-limit law held: five lane queries at 25–35 s spacing, no 429s.
- Lanes (query → rows): `abs:quantization` → 29 · `all:"speculative decoding"` → 5 · `all:"KV cache"` → 13 · `all:"GPU kernel"` → 1 · `cat:cs.PF` → 6 = 54 fetched, 49 unique, 5 cross-lane dups, **0 seed hits** (fresh batch).
- Dedup seed: the pre-staged `.distill/006_walk18_dedup_seed.txt` (180 IDs) was found DELETED from the working tree mid-walk (untracked scratch; never tracked — no git recovery). Rebuilt byte-equivalent from `.distill/004` (143) + walk-17's 37 graded IDs = 180 exactly, verified against walk-17's graded-ID list before use. Repaired at landing: `006` restored TRACKED + `007` pre-staged for walk #19 (229 IDs = 180 + this walk's 49).

## The grades (lane-relevant subset; the full 49 in the closeout dispositions table, `.research/334`)

**LEAGUE/TRAIN INTEL (riir-train + the perf league):**
- **2610.34886 ThinkBit** — scalar 2-bit weights (integer codeword × bf16 group scale) with a sensitivity-metric per-layer TIER law (ternary/2-bit/3-bit) and a reasoning-axis eval. The ternary lane's mixed-tier sibling (walk-17's TWIG was the format sibling; this is the graduated-tier sibling). TRAIN-side codeword search; FORMAT faces staged for the ternary lanes.
- **2610.34915 MXFP8 RL matching** — sampler on the LEARNER'S FP8 grid beats a BF16 sampler on consistency (KL −11–16%, byte-identical forwards). CONFIRMS walk-17's 2610.32951 rounding-location watch row as a measured law: numerics agreement > precision. Determinism-law adjacent (Issue 940's training-side face).
- **2610.34566 Error-diffusion binary weights** — 19× less training state (2–8-bit stochastic-rounding residual buffer replaces the fp32 latent); threshold-alignment rule. Train intel.

**CORPUS-ADJACENT (kernel_opt/perf lanes) — all WATCH-grade, no corpus landing this walk:**
- **2610.33719 ZipFusion** — attention consumes the losslessly compressed KV cache DIRECTLY (register bit-op reconstruction, no decompression pass). The dequant-in-the-consumer family's KV instance; joins walk-17's tile-decompression watch row. No in-tree KV lane → WATCH.
- **2610.34824 SPARTA** — ternary accelerator: unified ternary-SpMM + bit-serial attention datapath, phase-aware sparsity. Ternary-lane design-law citation; silicon out of scope.
- **2610.34907 TMA** — workload-grounded operand-level choice law (TMA vs cp.async vs DSM vs multicast) from FlashAttention-3. Hopper-class, detection-only.

**DETERMINISM DOCTRINE (external confirmation):**
- **2610.34721 Prefetch-Steered Routing** — outputs must be a function of LOGICAL state, never transfer timing (deterministic logical cache; physical copies affect only latency); RTX 4090 measured, 2.2–2.3×. The determinism law re-derived as MoE-offload routing design.

**SPEC-DEC LANE:**
- **2610.34440 Spec-Dec for MoE** — dedup engine reads each selected expert once per PASS (bytes law); best draft length = monotone staircase in acceptance; Lean-checked bounds; measured on llama.cpp Metal/CUDA + MLX-LM. The acceptance-economics cluster's systems member; the opponent is the bench.
- **2610.34103 T2D** — target-derived draft models (latency-aware pruning + distillation). Train-side lane note.
- **2610.33379 Packed-SS** — one-call self-spec + diffusion drafting, linear input. NOTE-ONLY (diffusion drafting off-lane per walk-16 doctrine).

**NULL/OFF-LANE (title/abstract-grade, no abstract read owed beyond the fetched summary):** 2610.34993 (traffic vision), 2610.34991 (speech LoRA), 2610.34890 (agentic NPU deployment — healer-adjacent framing, no modelless rule), 2610.34828 (FlowVQ VQ-VAE), 2610.34742 (Sentinel-2 SR), 2610.34703 (SBI), 2610.34675 (model fingerprinting — freeze-lineage adjacency note), 2610.34585 (tutorial), 2610.34509 (power flow), 2610.34495 (GLVQ unlearning), 2610.34479 (ST-SIM), 2610.34318/2610.34125/2610.34122/2610.34063 (physics/math-ph quant ×4), 2610.34304 (conveyor YOLO), 2610.34263 (3DGS), 2610.34241 (coding-agent ER), 2610.34135 (wireless KV — compute-once instance), 2610.33995 (bridging model), 2610.33964 (ECG), 2610.33932 (SAR), 2610.33923 (preemption queues — B− lane note), 2610.33839 (clinical deid), 2610.33824 (voltage-scaling quant), 2610.33800 (quantum encodings), 2610.33625 (RAQ-MLA — B− lane note), 2610.33501 (DualPace — B− lane note), 2610.33475 (cloud survey), 2610.33404 (COPE emulation), 2610.33360 (looped shared-KV), 2610.33358 (MOSHO NAS), 2610.33338 (TinyFlow), 2610.33298 (composable transform attack — doctrine citation), 2610.33187 (Bohmian cosmology — also the span's lower bound, line 11), 2610.35023/2610.35020 (BellowsKV + anisotropy theory — B− notes), 2610.34436 (RIPE — B− note), 2610.34907 (above).

**Verdict: ZERO corpus rules this walk** (54 rows → 0 landings; TENTH consecutive zero-delta walk; the lane output is intel + watch rows + doctrine citations). Walk #19 (Wed-cycle grading) grades `[202610261800 TO 202610271800]` with the seed = `.distill/007_walk19_dedup_seed.txt` (229 IDs, tracked at this walk's landing).

**Graded IDs this walk (49):** 2610.35023 2610.35020 2610.34993 2610.34991 2610.34915 2610.34907 2610.34890 2610.34886 2610.34828 2610.34824 2610.34742 2610.34731 2610.34721 2610.34703 2610.34675 2610.34585 2610.34566 2610.34509 2610.34495 2610.34479 2610.34440 2610.34436 2610.34318 2610.34304 2610.34263 2610.34241 2610.34135 2610.34125 2610.34122 2610.34103 2610.34063 2610.33995 2610.33964 2610.33932 2610.33923 2610.33839 2610.33824 2610.33800 2610.33719 2610.33625 2610.33501 2610.33475 2610.33404 2610.33379 2610.33360 2610.33358 2610.33338 2610.33298 2610.33187

NOTHING INVENTED — every verdict a live API read this session (the roll-timing probes + the five in-window lane queries + the stable re-poll + the Monday-subset probe); the window correction stands (walk-17's, exercised unchanged); the roll-timing datum is measurement-pinned (six static probes + the 02:16Z jump + the stable re-poll).
