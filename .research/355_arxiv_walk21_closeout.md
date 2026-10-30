# Research 355 — arXiv pool-refill walk #21 (closeout; the index UNFROZE — the multi-day catch-up graded)

**Status:** RECORD — **the stall BROKE: 1748 rows over the catch-up window [202610271800 TO 202610300300] graded at the title/abstract level; 23 graded rows: 4 B + 10 B− + 8 C+/C + 1725 skip. Zero corpus delta at walk time (THIRTEENTH consecutive zero-delta walk)** — grades and corpus-routing candidates recorded here; any batch conversion is a future slot's claim. Zero cargo, zero GPU throughout (M3; web-API only; the i1117 T4 collection holds the box — Plan-337 posture).

**Filed:** 2026-10-30 ~03:0x–04:0x UTC, zed-m3-walk21probe/M3, riir-refine idle loop. Claim row pushed 2026-10-30 ~03:0xZ (epoch 1793329775, `1586b9f3`) BEFORE any lane fetch — walk #21 was UNCLAIMED (no claim row landed upstream; the zed-glm-walk21 10-29 adjudication left the probe cadence carrying to the next lane; this is that lane). Push is the arbiter; no counter-claim materialized.

## THE HEADLINE DATUM — THE STALL BROKE at the 10-30 Thu roll

The stall record stood at 4-for-5 (walks 14/15 + the 10-29 probes). This cycle's probe, at the walk-20 standing correction (probe ~02:15Z, fall back ≥04:00Z; probe fired **03:09Z Thu 10-30**):

- `submittedDate:[202610271800 TO 202610300300]` → **totalResults=1748** (cache-busted, one sparse query — the 429 law held throughout; zero blocks this session, ≥60 s between the probe and the extract's first page).

**The Thu-cycle roll LANDED.** The freeze at the 10-27T17:59:59Z announce cutoff is gone; the index now carries the full 10-28 + 10-29 + 10-30-early surface. The stall's total span: first frozen read 10-29 ~00:35Z (the walk20yield corroboration rows; the stall began after the 10-27T18:00Z announce), unfreeze observed 10-30T03:09Z — **~2 days frozen, breaking the walk-14/15 5-day-class fear**. The 10-29T04:00Z RSS rebuild (EMPTY, zero items, all three feeds — the walk20yield instrument fact) remains consistent with an announce-pipeline stall that self-healed upstream; today's 04:00Z RSS build was NOT re-probed (one canary was owed and the API unfreeze is dispositive — the RSS surface is lagging-index, not the arbiter).

**Walk #21 discharged as designed: the multi-day catch-up, one batch.** Fetch = the pre-staged `.distill/010` extract verbatim (bare submittedDate range, all categories, 9 pages × 200, 12 s spacing, 30 s backoff ladder — never fired; seed `.distill/009` = 275). **Dedup: clean — 1748 fetched / 1748 unique / 0 seed hits** (the seed's 275 IDs all predate the window; the walk-20 tail graded rows ≤10-27T17:59Z, outside `[…1800 TO …]`). Category extraction FAILED wholesale (the script's `category="…"` regex — the API returns `term="…"`; every row `[?]`) — lanes classified by title/abstract read, disclosed; the script's next copy should match `term=`.

## Graded rows (23)

### B (4)

- **2610.38983 · Beyond atol and rtol: Sound Test Oracles for Low-Precision GPU Kernels (numoracle)** — B+. THE family-methodology row of the batch: per-element acceptance regions derived from precision-annotated reference programs; a calculus tracking shared rounding errors, provenance-ordered products, block-scaled quantizers, calibrated math-library errors; a rounding-interval check catching faulty stores that symmetric bounds admit. This is exactly the family's G1/spec-match gate class (B305's `1u64<<64` fixture catch, the B307/B310 exactness gates, the spec-match Rust mirrors) — a principled upgrade path over hand-picked tolerances. Corpus-routing candidate: kernel_opt gate-methodology face or rust_perf (CPU-side test infra); MINING CANDIDATE for the next rust_perf slot.
- **2610.39418 · LinQuant: Functional Invariances for Low-Precision Linear Recurrences** — B. The GDN/linear-recurrence lane goes low-precision: chunkwise-parallel form suggests dense-matmul quantization, naive quantization is fragile, and functional invariances (reparameterizations preserving full-precision semantics) make operands quantization-amenable. DIRECT league-family intel (Bonsai is the hybrid GDN class; PQ2_0 is the served quantization) — the invariance-before-quantize law pairs with the lossy-surface promotion rule (per-family conditional retention).
- **2610.38578 · Dual DeltaNet: Value-Side Memory Reorganization for Linear Attention** — B. The delta-rule lane: state as a bank of d_k addresses; key-side reads retrieve per-key values but related value features scatter across the state — value-side reorganization to limit interference. The architecture family's own lane (Bonsai GDN decode + the delta-roll journal in riir-infer); watch for serving-side implications (state reorganization = different state layouts the journal/roll machinery must know).
- **2610.38329 · A Case Study of a Multi-Agent Kernel-Optimization Loop That Learns a New AI Accelerator** — B. Methodology intel: an LLM agent loop writing kernels for a sparsely-documented Rust-DSL NPU (FuriosaAI RNGD), building its own target knowledge — doc summaries, microbenchmarks, COMPILER-DERIVED LEGALITY RULES, toolchain-change re-verification — over 12 days on a shared scarce device, optimizing three Gemma-4-12B decode kernels. The loop's knowledge shapes are literally the kernel_opt corpus-rule shapes (admission guards, geometry laws, measured crossovers); the self-evolve trajectory store is our version. RECORD for the corpus's authoring-method lore.

### B− (10)

- **2610.39216 · PDScope: Breaking the Prefill-Decode Seesaw for Memory-Bound MoE Inference** — CPU-GPU MoE local deployment; prefill compute-bound (aggressive loading) vs decode I/O-bound (conservative transfers); phase-specific expert-routing PREDICTABILITY decouples the scheduler. Serving-family league intel (the routing-predictability datum is the load-bearing part).
- **2610.38981 · Why Speculative Decoding Works Better Than Predicted on Sparse MoE Models** — 10 sparse MoE models (30B–1T, 8–512 experts): the predicted concave speedup-batch curve BREAKS on fine-grained MoE (higher speedup at low batch); verification cost governed by activated-expert bytes AND temporal overlap across draft tokens + GPU execution dynamics. Spec-dec lane intel.
- **2610.38968 · TableTune: Measured, Self-Correcting Draft-Length Tables for Dynamic Speculative Decoding** — the draft-length-per-batch table is timed on vLLM's own serving path, computed in minutes, and REBUILT live by an acceptance-tracking controller. The measured-lookup-table-beats-hand-written shape is the corpus's launch-table family (gpu_17 swept-shape-bucket tables; the B307 TP-class + static_k face) applied to spec-dec; the live re-measure loop is the autotune-family's tune-cache law.
- **2610.38809 · CACPU: Causal Attention on CPU — Cache-Aware, Load-Balanced Scheduling Kernel for Prefill** — tiered cache blocking + in-kernel task dispatching for CPU prefill; 2.01× over SDPA/FlexAttention/SGLang; TTFT gains in SGLang. rust_perf lane intel (the laya CPU posture's attention lane is the home surface).
- **2610.37810 · ConfluxServe: Conflux Parallelism (XP)** — co-executes prefill and decode with DIFFERENT parallelism inside one batch via operator-level heterogeneous execution paths; resolves the utilization-vs-efficiency dilemma of separate phases vs uniform co-batching. Serving family (the split-parallelism abstraction is the league's chunked-prefill邻域 intel).
- **2610.38355 · FlatCake: A Flat KVCache Storage Architecture for LLM Serving** — production workloads are BIMODAL (one-shot long-reuse-distance blocks vs multi-GB agentic sessions), defeating DRAM temporal filtering and putting SSD on the critical read path; flat storage replaces the DRAM→SSD hierarchy; staging-buffer contention aborts prefetches. ndb lane record (the tiered Warm/Cold + R2-WAL family — the bimodal-reuse-defeats-temporal-filtering law is the portable part).
- **2610.38277 · The Role of Quantization in Loss Spikes** — "time-bomb" loss spikes: delayed terminal divergences PRECEDED by rapidly growing quantization error in the LM head; reducing precision accelerates them, increasing precision delays. Train-lane intel (riir-train) + the lossy-surface doctrine's training-stability face (the quantization-error-growth predictor is the portable datum).
- **2610.39442 · Softmax Routing Dynamics in MoE Models: Collapse and Recovery** — routing concentration: stationary-loss barrier identifies the optimal-output face; exact normal-velocity decomposition separates complementary-expert covariance; recovery dynamics characterized. The softmax-numerics family's training face (Issue 959's collapse family is inference-side; this is the training-side twin).
- **2610.39020 · Distilling the Shape of Uncertainty: What On-Policy Distillation Learns** — OPD learns "where to branch" (the entropy shape — the sequence of next-token entropies), not just "how to branch" (token overlap); stronger teachers can fail to improve students, predictable from the entropy shape. Distill-lane intel (the teacher lanes' selection signal).
- **2610.39223 · Where Position Matters: Profiling-Guided Layer-Wise RoPE Allocation** — rotary coverage is heterogeneous per layer; phase-off interventions reveal model-specific dependence; coverage allocation under a fixed budget from early benefit profiles; MLA physical rotary-dim allocation supported; normalized-depth mapping transfers profiles small→large models. Long-context lane + the tokengate mid-layer-placement family echo (L=mid-attention placement was OUR pin too).

### C+ (6)

- **2610.38661 · Quantize by Doing: Task-Guided Mixed-Precision Quantization of VLA Models** — task-success from short closed-loop evaluations guides per-component bit allocation. The lossy-surface doctrine echo (per-family, conditional — here per-task) in a robotics domain.
- **2610.37989 · GAUSS: Workload-Aware Analytical + Monte-Carlo Simulator for LLM Inference Serving** — predicts full TTFT/TBT distributions, tail latency degrading before throughput saturates. Record-only (our benches measure, not simulate).
- **2610.37914 · Muon Makes the First MoE Layer Harder to Balance** — Muon roughly doubles the first-layer routing-bias span at tuned learning rates. Train record.
- **2610.38095 · Nemotron 3.5 Super + Lightning** — release watch: 120B/12B-active multimodal + 30B/3B-active MoE HYBRID Mamba-Transformer with native speculative decoding, large-scale distillation. The hybrid-Mamba league family's newest open entrant (league-opponent add is an owner call, perf-rematch charter).
- **2610.39056 · When Does Closed-Loop Curriculum Adaptation Help GRPO?** — too-hard → uniformly failed rollouts, too-easy → no advantage signal; adaptive curriculum on Countdown. Train record (the riir-train GRPO lanes).
- **2610.38681 · Weaken to Strengthen (WISD)** — self-distillation via a deliberately WEAKENED counterpart (low-bit quantization or magnitude pruning) as the contrast signal, no privileged information. The quantized-weaker-self trick is the portable datum (train lane).

### C (3)

- **2610.38637 · On-Policy Self-Distillation with Teacher Evolution (TESD)** — self-teacher reliability underlies OPSD stagnation; evolve the teacher during training. Train record.
- **2610.39222 · When Does Heterogeneity Pay Off? GPU–WSE Complementarity** — hierarchical benchmarking + analytical model for GPU/WSE co-execution. Systems record; no lane mapping.
- **2610.38558 · REVBRAIN: Real-time Probabilistic EEG Forecasting Using Residual Vector Quantization** — RVQ applied outside LLMs. Record-only.

Skips: 1725 (the bare all-categories window is dominated by non-family fields — math/physics/astro/bio bulk).

## Queue disposition + the corpus reads

**Zero corpus delta at walk time** — grades only. The mining candidates flagged for the next rust_perf/kernel_opt slot: **numoracle (2610.38983)** is the strongest conversion lead (the sound-oracle calculus maps to the gate-methodology faces the corpus already carries — the B305 fixture-catch class); TableTune's measured-table law is likely a rider face on the launch-table family (gpu_17) rather than a new rule. Any conversion is plan-first per protocol — NOTHING staged here, the pools are B301-foreign until ≥10-31T13:41Z and this note claims nothing.

**Lane profile:** quant 4 · gpu-kernel 5 · specdec 2 · kv-gdn 2 · serving 2 · train 6 · long-ctx 1 · release-watch 1 (= 23).

## Artifacts this push

- `.research/355` (this note) — `.research/.highwater` 354→355; dual_allocation_gate run at allocation.
- `.distill/011_walk22_dedup_seed.txt` pre-staged = seed 009 (275) + the 23 graded IDs = **298**.
- `.distill/.highwater` REPAIRED 1→11 (stale since Aug 18 while the folder max is 010; folder-max-derived, disclosed — the .distill counter's standing drift, not this walk's).
- Snapshot landing row in `.distill/001` (discharges the claim `1586b9f3`).
- `/tmp/walk21/` scratch (candidates/abstracts/debug) — NOT committed, regenerable; removed at close.

## Standing corrections for walk #22

1. **Probe from ~02:15Z Thu-adjacent cadence; the stall class is now 2-day, not 5-day** — the multi-day catch-up mechanism works; keep the ≥04:00Z Wednesday fallback in the tool.
2. **The extract's category regex is broken** (`category=` vs the API's `term=`) — fix the next copy before any lane-classified-by-category run; title/abstract classification is the fallback that worked here.
3. Seed 011 = 298 IDs is the dedup base; window starts at the close of this one (202610300300) — the next extract invocation needs its `[FROM` raised from the hard-coded `202610271800` (the script pins the FROM; walk-22's copy should parameterize both ends).

NOTHING INVENTED — every grade a fetched-abstract read this session; every probe a live fetch with its timestamp. Session: zed-m3-walk21probe, 1793329775.
