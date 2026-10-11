# 113 — arXiv pool refill walk #4 (fetch fallback; zero corpus delta; the spec-dec lane wakes up)

**Status:** RECORD — pool walked 2026-09-02 ~07:15 +07 (the Tuesday-09-01 announcement window:
the newest submission captured is 08-31T15:55Z, exactly one submission-day past walk #3's
08-30 frontier — the daily differential cadence held). Method identical to walk #3
(`export.arxiv.org/api/query` via the raw fetch tool; four queries: cs.PF ×15, cs.DC ×15,
cs.AR ×12, `all:"speculative decoding" AND cat:cs.LG` ×8; zero builds, zero GPU — a
Plan-337 zero-contention unit). **Zero corpus deltas, zero new batches.** The headline is
the league lane's first new entries since walk #2 — measured, routed, no re-arm trigger fired.

## The spec-dec league lane (2 new + 1 audit)

1. **2608.30427 Ceiling-Clipped Acceptance Histograms / DBloom (block-diffusion SD).**
   Block-diffusion drafters (DFlash/DFlare) hitting the ceiling bin = "stranded speed-up";
   the **acceptance histogram (ceiling-bin spike), not the mean committed length**, is the
   preflight diagnostic before spending training compute on drafter expansion. The
   load-bearing finding: naively widening the block at inference does NOT recover the
   stranded speed-up — "once the block outgrows its training size, the drafter's
   bidirectional attention shifts its distribution even at early positions and erodes
   front-of-block verification." **Third independent arrival at our DFlash verdict class**:
   our Bench 746/747 found the DFlash2 head loses composed-with-lookup on doc-repro and its
   chat acceptance (0.49) sits below loop breakeven; Bench 696 G2-full found the trained
   block-denoising drafter has the wrong standalone contract (a low-rank logit bias composed
   under conditioning, not a standalone drafter). The paper's erosion mechanism explains WHY
   block-horizon economics don't transfer at inference — the same family our chain-seam
   negatives (Issue 717 G3a/G3b) measured. **Adoptable at zero cost: the histogram, not the
   mean, is the right acceptance telemetry for our verify-loop benches** (bench_754's
   acceptance rows report means; a ceiling-bin fraction is one `u32` counter per cycle).
   Cite at the next verify-loop bench touch. No batch (measurement convention, not a rule).
2. **2608.30252 Strong Drafts Need Compact Memories (long-context SD, EMNLP Findings).**
   Memory-augmented drafting: a trained adaptor builds compressed draft-side KV while the
   target verifier keeps its full cache (losslessness preserved by the standard
   accept/reject). 2.08×/3.33× at 32K prefixes on Llama 3.1. **Route: riir-train lane** —
   the adaptor is a trained component; our qwen38 long-context verify loop uses lookup
   drafting and the f16-KV lever (Issue 753) on the TARGET side, not a draft-side memory.
   No fix-space here; note for the long-context league cells if that lane re-opens.
3. **2608.28846 Rigor-matched audit of periodic-step layer skipping (ConfLayers vs SWIFT).**
   Separating online-search overhead from pure inference cost REVERSED the naive wall-clock
   ranking in 3 of 4 cells (SWIFT's search overhead up to 28.7% of cost). External
   confirmation of the measurement doctrine we keep re-learning (Bench 749's unwarmed
   first-arm 41× artifact; the Bench-728 interleaved discipline; Issue 831's
   load-order effects): **cost attribution must split the search/verify phases before any
   ranking is claimed.** The paper releases its audit protocol as a template — same genre
   as our own GOAT-gate methodology docs. Cite as external validation at the next
   measurement-doctrine doc touch. No batch.

## Doctrine citations (routed, no fix-space — owner-visible notes only)

4. **2608.30923 Stream Learning on Embedded Systems (memory-consumption benchmark).**
   Benchmarks 7 stream classifiers under explicit model-size budgets (128 KiB–8 MiB) and
   finds two failure modes (initial-footprint overflow vs unbounded growth); the call to
   action: "make bounded resource usage a first-class design objective" + "an API through
   which learners can explicitly expose and respect resource budgets." **This is our G4
   alloc-axis doctrine restated from the ML side**: the TrackingAllocator counters, the
   exact-alloc-count gates, and the grow-only-scratch rules (B32a family) are exactly the
   "expose and respect budgets" API the paper asks for. Cite as external validation at the
   next G4/alloc-doctrine doc touch. No batch.
5. **2608.28955 Algorithmic Simplification for Diffusion History Reconstruction (Battus-Z).**
   Removes the learned GNN proposal, the MCMC stage, and parameter fitting from a published
   pipeline one at a time; the remaining deterministic mean-field forward-backward variant
   MATCHES the published aggregate quality at 5.1–20.3× lower algorithm interval. **The
   modelless-first mandate confirmed externally, again**: before accepting a learned stage,
   measure the deterministic ablation (our Issue-026/035 lineage: starved-evidence learned
   selection losing to count-updating; the Gemma2-directions GOAT FAIL; L4's 0/60). No batch.

## Drops (with reasons)

- **2608.30226 LaMoC loss-aware modular compression (EMNLP Findings)** — training-pipeline
  compression selection via Empirical Fisher blending; the lossy-surface doctrine already
  governs our side (deployed-path per-family behavior, Issue 750 T3), and the selection
  method itself needs a training loop. riir-train adjacent, no inference-primitive content.
- **2608.29362 Spectral Analysis for Sparse Matrix Computation** — FFT spectral signatures
  as SpMV format-selection features; ML-based selection (train a classifier over formats),
  and the LLM-decoding kernel speedup is 1.035–1.245× (marginal). Modelless-first excludes
  the learned selector; our kernel_opt router is an 8-D activity-vector annotation, not a
  trained format picker. Drop.
- **2608.31004 Storage-centric genomic analyses (dissertation)** — storage architecture for
  genomics; no lane. **2608.30299 distributed MST (CONGEST-KT₁ theory)** — message/round
  trade-off theory; no lane. **2608.30159 projection-free bandit online optimization** —
  control-theoretic multi-agent OCO; no lane. **2608.29656 SNF-ICON HPC job scheduling** —
  energy/wake-time scheduling; no lane. **2608.29581 SemSpot (LLM spot-capacity service
  model)** — MaaS economics for agent workloads; interesting for KAT-adjacent thinking but
  zero fix-space and zero kernel content. **2608.29528 MedCache (clinical agent memory)** —
  agent-memory design study; echoes our retrieval substrate at agent-architecture grade,
  no bounded fix. **2608.29103 CLASP serverless stream scaling / 2608.29053 RED-ONION disk
  transfer / 2608.28988 B³-PWL GPU branch-and-bound MIP** — infrastructure domains, no lane
  (our pareto-front search is microscopic; no MIP solver surface).
- **cs.AR block (2608.30954 clock-gating RTL-vs-ICG, 2608.30932/30659 LLM-driven hardware
  design, 2608.30509 CHIPSMORE CIM chiplet, 2608.30444 SNN memory banking, 2608.30435 AXI4
  monitoring, 2608.30379 KORD FSS hardware, 2608.30268 FABO routing EDA, 2608.30058
  adiabatic FinFET, + walk-#3 population 2608.28387/28188)** — confirms walk #3's finding:
  the architecture query is EDA/quantum/hardware noise for our corpus. One aside: 30954's
  "behaviorally-correct RTL gate fails at gate level (hold race)" is the formal-verification
  vs simulation gap in hardware form — same shape as our syn-parse-passes-but-typeck-rejects
  healer lessons; noted, no batch.

## Queue state after this walk

- Corpus: UNCHANGED (422 kernel_opt + 97 rust_perf + 45 clippy_lints + 12 sec + 31 docker —
  post-Batch-94 counts). No batch queued from this walk.
- **Next re-walk window: 09-03+** — this walk covered the Tuesday-09-01 announcement window;
  Wednesday-09-02's submissions land Thu ~07:00 +07. Daily differential cadence continues.
- League lane: **awake** (2 new entries) but the opponent-watch re-arm condition still not
  met (no ≤24-GB-feasible Flash-Next-class variant with native MTP; DBloom strengthens the
  DFlash lane rather than replacing it). The zero-cost adoption candidate (ceiling-bin
  histogram telemetry) rides the next verify-loop bench touch.
- MCP search/webReader quota: still gated until 09-15; the fetch fallback remains the
  standing refill path per R355.
