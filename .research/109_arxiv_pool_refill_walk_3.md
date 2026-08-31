# 109 — arXiv pool refill walk #3 (fetch fallback; zero corpus delta, two doctrine citations)

**Status:** RECORD — pool walked 2026-08-31 ~23:20 +07 (the 09-01+ window's first minutes; the
Sunday-20:00-ET announcement batch arrived Mon ~07:00 +07 and was NEVER seen by walk #2, which
swept 08-24→08-29). Verdicts below; **zero corpus deltas, zero new batches** — the strongest
candidates are doctrine citations and one v2 delta on an already-consumed paper. Method identical
to walk #2 (`export.arxiv.org/api/query` via the raw fetch tool; MCP search still quota-gated to
09-15; zero builds, zero GPU — a Plan-337 zero-contention unit, safe beside the sibling lanes).

## Already consumed (walk #2 overlap — confirmations, NOT re-verdicts)

The top of every query returned walk #2's population unchanged: 2608.27046 (RLM survey),
2608.27029 (FoldPipe), 2608.26912 (TOPIQ → Issue 750 T3 doctrine), 2608.26368 (SIMD bit-slicing
→ Batch 83), 2608.26612 (Launch-Bound → Batch 82), 2608.26235 (Reasoning Tax), 2608.25053
(Hydra), 2608.24070 (Compression Trinity), 2608.26402 (SILK → neuron-db strategic note),
2608.26418 (Redwood), + the whole spec-dec lane (TailSieve v2 / TreeWY / DARTree / Intern-S2 /
EdgeXpert / SpecRoll / ASD v2 / AcceptMoE) — **the spec-dec+cs.LG query returned ZERO new
entries since walk #2** (all eight are pre-08-29 submissions; the league lane is quiet).

## New-to-verdict entries (this window)

### Doctrine citations (routed, no fix-space — owner-visible notes only)

1. **2608.27975 LAH / S4-FIFO (OSDI '26) — Learning-Augmented Heuristics for cache
   eviction.** Decouples data plane from control plane: a simple static heuristic
   (S3-FIFO) serves every request at full speed while an occasional ASYNC control-plane
   learner re-tunes the heuristic's cache-level parameters from a pre-trained model; +26%
   mean efficiency over S3-FIFO, robustness held (worst-trace miss +0.8% vs 3L-Cache's
   +8.8%). **Route: riir-dao doctrine** — the dao's architecture is the same shape at the
   ledger layer: the fast KAT ledger path never waits on the modelless agent; the agent
   asynchronously re-tunes static constants (decay, cap, split) under hard bounds, and the
   bounds ARE the robustness story. Also echoes in-stack: `gemv_autotune`'s process-global
   cache (bench-once at first use, never in the hot loop) is the data/control split for
   kernel tuning. Cite externally at the next dao design-doc touch; no batch.
2. **2608.27874 TerraceMoE — hierarchical MoE all-to-all cost model with executable
   validation gates.** The gates "withdraw a capability in code when they fail rather than
   reporting a caveat"; the step-level gate FAILED, so the paper ships **no
   training-throughput prediction at all** and says so in the abstract. External
   confirmation of the fail-closed-gate doctrine (the "gate nobody runs is documentation"
   lesson + the GOAT honest-FAIL discipline: a gate that cannot fail proves nothing, and a
   failed gate kills the claim rather than footnoting it). Cite as external validation at
   the next gates-doctrine doc touch; no batch.

### v2 delta on a consumed paper (record-only)

3. **2608.26612v2 Launch-Bound (Batch 82 consumed v1).** The v2 adds the router-precision
   finding: leaving MoE routers in FP16 **lowers routing drift 20% while RAISING loss** —
   "routing fidelity and output quality are separable objectives." Not MoE here, but the
   separable-objectives shape is the lossy-surface family's cousin (a precision cut on a
   component can improve that component's own metric while hurting the end-to-end one) —
   consistent with the Issue 750 T3 / Bench 696 doctrine that component-local metrics don't
   certify end-to-end behavior. Fold as a one-line citation into the next lossy-surface doc
   touch alongside TOPIQ; no batch.

### Drops (with reasons)

- **2608.28044 LLM request/token energy characterization (IISWC '26)** — token-normalized
  metrics incomplete (per-token energy falls while total rises). The metric-completeness
  shape echoes our own bench lessons (isolated-kernel speedups returning 0.999x e2e —
  Batch 82), but energy accounting on H100/H200 is not our lane; the doctrine echo is
  already carried by 26612v2 + TerraceMoE. Drop.
- **2608.28352 GaussDB 30TB TPC-H (+40% over best published)** — pipeline execution,
  inter-node shuffle, cost-based + streaming Bloom filters. Distributed SQL-engine
  internals; `riir-neuron-db` is a KV/shard store, not a join engine. The Bloom-streaming
  bit is retrieval-adjacent but arrives at DB-architecture grade with no bounded fix-space.
  Drop.
- **2608.28183 memory-efficient GPU pipelines for NLOS reconstruction (42x)** — mechanism
  census: fused kernels, warp-level binning, batched transforms, CUDA graph replay,
  FP16-storage applied ONLY where it reduces the actual bottleneck. **Every named mechanism
  is already a corpus rule** (fusion B19, CUDA graphs B22/B58, fp16-storage-regime B84/B86,
  batched transforms B25/B26). External validation of coverage; nothing new to distill. Drop.
- **2608.28097 / 2608.28076 RVV 1.0 HPC + juFFTe** — RISC-V vector silicon benchmarks; no
  lane (we ship no RVV targets). Drop.
- **2608.28048 AI hardware accelerator survey / 2608.27199 vision-gen HW perspective** —
  surveys; "the memory system has become the computer" is our own decode-path doctrine
  restated. Drop.
- **2608.28158 HARTS — agentic RL over rollout trees on hybrid attention** — training-systems
  (prefix-sharing microbatching for linear-attention hybrids); riir-train adjacent at
  systems grade, modelless-first excludes; zero inference-primitive content. Drop.
- **2608.27865 FFSlim multi-modal storage format / 2608.27642 HPC I/O simulation /
  2608.27278 WSI pipelines** — training-corpus / HPC-I/O / pathology-image I/O; no lane.
  Drop.
- **2608.27710 SPEC CPU2026 adaptation fidelity / 2608.24370 ABSTRACTS Clifford+T suite /
  2608.27099 quantum partitioning eval** — benchmark-methodology papers; the
  measure-the-measurement genre has no consumer lane here (our own benches already carry
  the lessons they formalize: warm-arms, load-disclosure, vacuous-guard). Drop.
- **2608.27655 Jetson NVJPEG decode offload / 2608.26498 PRO-RAN / 2608.24841 O-RAN CNF
  MILP** — edge vision preprocessing, O-RAN characterization/placement; no lane. Drop.
- **2608.28529 CBDC zero-knowledge settlement / 2608.27836 FISGuard membership-inference
  defense / 2608.28387 neuromorphic numerical solvers / 2608.28188 DeepSeq3 EDA GNN /
  2608.28160 Gen-TAS FPGA allocation / 2608.26758 HOLMES yield estimation / 2608.26342 DNA
  storage / 2608.26100 Ising annealing** — no lane (each outside the workspace's domains;
  cs.AR block confirms walk #2's finding that the architecture query is mostly EDA/quantum
  noise). Drop.

## Queue state after this walk

- Corpus: UNCHANGED (411 kernel_opt + 96 rust_perf + 45 clippy_lints + 11 sec + 31 docker —
  post-Batch-92 counts, Batch 93 was zero-delta). No batch queued from this walk.
- **Next re-walk window: 09-02+** — Monday 08-31's announcement lands Tue 09-01 ~07:00 +07;
  this walk covered the Sunday-08-30 announcement only. The daily differential cadence is
  now established: each walk sweeps exactly one announcement window, so ~24h cadence is
  honest and non-duplicative.
- League lane: quiet (zero new spec-dec entries since walk #2). Opponent-watch stays at the
  08-31 "history note" state (re-arm = a ≤24-GB-feasible Flash-Next-class variant with
  native MTP).
- MCP search/webReader quota: still gated until 09-15; the fetch fallback (this note) is
  the standing refill path per R355.
