# Research 096 — TreeWY + Launch-Bound distill (the 2026-08-28 arXiv pool)

**Status:** RECORD (verdict + Batch 82 / Plan 043 landed same day)
**Sources:** arXiv:2608.20961 (TreeWY, Ghantasala, submitted 08-21, 10pp) · arXiv:2608.26612 (Launch-Bound and Substitutable, Sakthivel et al., UMD, submitted 08-27)
**Lane:** idle-loop queue item 11 (online perf search) — the MCP search tools were quota-exhausted (reset 2026-09-15), so the pool came from walking `arxiv.org/list/cs.PF+cs.DC/recent` via the native fetch tool.

## 1. Why these two

Both papers sit EXACTLY on our substrate and both carry measured numbers with
committed artifacts — the two intake criteria the snapshot records as live
("arxiv systems papers + measured-GOAT distillations remain the live intake
lanes"). Between them they externally cross-validate three of our own measured
verdicts, which is the strongest provenance signal a distill source can carry.

## 2. TreeWY (arXiv:2608.20961) — spec verification for Gated DeltaNet hybrids

### 2.1 Mechanism (the load-bearing half)

The gated delta rule rewrites as **decayed additive attention with a corrected
value**: `S_t = α_t·S_{t-1} + ṽ_t·k_t^T` with pseudo-value
`ṽ_t = β_t(v_t − α_t·S_{t-1}·k_t)`. Given the ṽ, every state and output is a
decay-weighted SUM over preceding tokens — so obtaining the ṽ without walking
the recurrence removes the need for intermediate states.

Lay out the draft's N nodes in **DFS pre-order from the committed state S₀**
(every ancestor of node t has a smaller index). Chaining the rewrite along
every root-to-node path turns verification into ONE linear system for all
pseudo-values at once:

```
(I + diag(β)·G)·Ṽ = R
G[t,i] = (g_t/g_i)·(k_tᵀ·k_i)  for i ≺ t     (strictly lower-triangular)
R[t]   = β_t·v_t − β_t·g_t·(S₀·k_t)
```

One **forward substitution** solves the whole draft (chain OR tree — the
system does not change as the tree widens). On commit, reconstruct ONLY the
accepted state: `S_a = g_a·S₀ + Σ_{i⪯a}(g_a/g_i)·ṽ_i·k_iᵀ`. Storage:
`Ṽ ∈ O(N·d_v)` instead of N+1 full state blocks — a **128× smaller object
per head** at d_k=d_v=128. Correctness: closed form matches the per-node
recurrence to ~1e-15 (fp64) / ~1e-7 (fp32).

### 2.2 Measured verdict (honest, conditional)

Qwen3.5-35B-A3B / 397B-A17B on B200, vLLM fork, 175 matched points:
- Peak KV usage **2–3× lower** at the same load (one state block vs k+1).
- Wins **only where memory binds**: up to 1.49× throughput, ~40× lower p99
  TTFT past the concurrency knee; **0.97–0.99× parity** below it (the
  per-token dip is the price of admitted concurrency, r=−0.88 vs batch
  growth — Appendix C).
- Acceptance length essentially identical (mean |Δ|=0.039 over 175 points).
- Trees: **affordable but NOT a throughput win** — "a wider,
  higher-acceptance draft becomes possible, though not yet a throughput win";
  the non-capturable tree mask forces piecewise capture that "evicts the GDN
  mixer from graphs, making the cost far larger than the mask itself".
- vs ReplaySSM (same triangular solve, deferred state writes): ReplaySSM
  leads per-token at every load — the paper labels its own deferred-write
  attribution "an attribution, not a validated claim".

### 2.3 Cross-validation of OUR verdicts (the provenance signal)

- **Issue 721 (tree-masked batched verify, closed-negative G2):** our
  structural finding — verify cost ~linear in T, B/S ≈ T/(1+acc) — is
  externally confirmed by a different stack: even with the memory problem
  SOLVED, trees are "not yet a throughput win". No reopen.
- **Issue 717 G3b (tree degenerates to chain under sequential verify):**
  unchanged — TreeWY's chain path is the graph-capturable one that pays.
- **B58 `capture-region-dispatch-only`:** TreeWY's piecewise-capture
  eviction cost ("far larger than the mask itself") is an independent
  instance of exactly the hazard that rule encodes.

## 3. Launch-Bound and Substitutable (arXiv:2608.26612)

### 3.1 The launch-bound diagnosis (sub-study 1)

Fused Triton kernels (RMSNorm + router softmax) reach **5.6–9.0× isolated**
but **0.999× end-to-end** against a measured **1.072× Amdahl ceiling** on
OLMoE-1B-7B. The gap Amdahl does not explain is the launch queue: the expert
dispatch is a Python loop over 64 experts × 16 layers ≈ **a thousand small
sequential kernel launches per forward pass**. The decisive probe: going from
128 to 4096 tokens — **32× the work — raises latency only 1.55×**. "A model
whose latency is nearly independent of how much arithmetic it performs is not
waiting on arithmetic." The kernels are *integration-bound, not Amdahl-bound*.

Amdahl-f discipline: f (=7.87%) obtained by wrapping the **actual modules in
profiler `record_function` ranges (81 modules)** — NOT by matching CUDA kernel
names, which "undercounts twice over": it misses the variance-reduction
kernel (name mismatch) and the extra QK-norm modules (two per attention layer
beyond the two in the residual stream).

### 3.2 The capture honest-negative (sub-study 3)

Removing **all 23** torch.compile graph breaks (via
`capture_dynamic_output_shape_ops`) makes the model **3× SLOWER** (0.613×/
0.327× at the two shapes) and raises the first forward from 1 s to ~40 min:
unbacked symbolic shapes force Inductor to generate code handling ANY size,
and the guards + fallbacks cost more than the eager dispatch they replace.
"Zero graph breaks is three times slower than eager… the break count is a
traceability diagnostic and not a performance target." The productive lever
is restructuring the data-dependent dispatch (torch.nonzero → a single
batched grouped op), not forcing capture.

### 3.3 The substitutability result (recorded, not mined)

INT4 changes 0.53 of 8 selected experts per token, but the **causal
route-replay intervention** (FP16 weights forced to follow INT4's recorded
routes; control reproduces the baseline to all printed digits) shows routing
changes account for **2.7%** of the quality loss — weight error 97.3%. Drift
correlates with quality at +0.907 while being 98% collinear with gate KL
divergence: "a variable that tracks the damage without causing it".
Router-exemption inverts: FP16 routers lower drift 20% and RAISE loss.

**Why not a corpus rule:** model-quality territory with no bounded kernel
fix-space. Kept here as measurement discipline — the intervention-vs-collinear-
surrogate method is the paper's best transferable lesson and mirrors our own
vacuous-gate/negative-control GOAT discipline.

### 3.4 Cross-validation of OUR verdicts

- **B76 `dispatch-cost-sequential-measurement-not-single-op` + B80
  `fusion-payback-gated-on-per-dispatch-overhead`:** the ~1000
  launches/forward ≈ our own 577 submits/token (Bench 761, M3 decode); the
  5.6–9.0×-isolated → 0.999×-end-to-end collapse is the MoE instance of
  B80's payback law, measured on a different stack.
- **Bench 761's M3 decode host-bound finding:** independently re-derived.

## 4. Corpus dedup (pre-proposal grep — the distill-skill gate)

Grep over `src/domains/kernel_opt/` for
`wy.transform|triangular.solve|pseudo.value|state.snapshot|launch.bound|work.scaling|amdahl|graph.break|record_function`:
ONE hit — B77 `vendor-batched-gemm-contiguity-staging` (triangular-solve as a
vendor-API chunk-size guard). Not adjacent to any candidate below. **All
three candidates are new decision content.**

## 5. Rule candidates (GOAT-lead ranked)

1. **`wy-transform-tree-verify-state-free`** (G: LinearAttnRecurrence) —
   DFS pre-order makes the transition system strictly lower-triangular; ONE
   forward substitution verifies the whole draft state-free; reconstruct
   only the accepted state on commit; O(N·d_v) pseudo-values vs N+1 state
   blocks (128×/head at d=128). REGIME: memory-conditional; acceptance
   unchanged; trees affordable but not a throughput win. Composes with B81's
   replay-log rule (ReplaySSM = same solve + deferred writes).
2. **`work-scaling-probe-launch-bound`** (D: Dispatch) — sweep work ×32; if
   latency rises ≤~1.5×, the pass is launch-bound and isolated kernel gains
   will not surface end-to-end; measure the Amdahl ceiling by wrapping
   modules in record_function ranges, never by kernel-name matching
   (undercounts twice). "Integration-bound, not Amdahl-bound."
3. **`capture-break-count-not-perf-target`** (E: Parallelism) — a count of
   capture/trace-breaking sites is a diagnostic, not a target: forcing full
   capture through dynamic-shape machinery inverted the sign (3× slower;
   unbacked symbolic-shape guards + fallbacks cost more than the dispatch
   they replace). Restructure the data-dependent op shape-static (batched
   grouped form) or accept the break — never force capture. Corroborated by
   TreeWY's piecewise-capture eviction cost.

## 6. Batch 82 record (Plan 043)

All three candidates LANDED as rules 394–396 (corpus 393 → 396; D 82→83,
E 23→24, G 22→23) in `entries_gpu_08.rs` + `KERNEL_CATEGORIES` + the three
standing gates (`batch82_rules_present` / `batch82_concepts_covered` /
`batch82_pseudocode_parses`). Generator: `target/gen_b82.py` (the B77
rustlit()-escape path; needle + anchor assertions). One anchor bug caught
pre-apply: the first run's `isascii()` assertion fired on the `──` comment
rulers (house style since B81) — the check is now scoped to literal content
only. One category-wiring miss caught by the count-pin test (the E-slice
anchor landed `capture-break-count-not-perf-target` in F; the fix's first
pass over-deleted two F entries — both restored, verified by the clean
git diff).

**GOAT (measured, isolated same-parent worktree at `3d567f9` + exactly the
4 batch files):** kernel_heal lib **944/0/1i** (941+3 gates) + default lib
**715/0/0** + clippy `--all-targets --features kernel_heal` **0 warnings in
the batch files** (5 doc_markdown backtick fixes pre-commit — the B37
line-family lesson struck again: `TreeWY`, `O(N*d_v)`, `N+1`,
`record_function`) + retrieval floors **14/14 HELD** (kernel self-variants
100/100/100 — 11th consecutive) + CI guard Layer 1 **26/28** — the 2
failures (`--all-features`, `--features ternary_inference`) are the
**riir-ai sibling's live riir-gpu WIP** (`write_many` mid-refactor in
`../riir-ai/crates/riir-gpu`), proven not-mine: the errors are in
riir-gpu's lib, my diff is corpus-data-only, and the main tree's
all-features lib run **PASSED 1426/0/13i ~25 min earlier** before their
breaking edit landed (both timestamps recorded). The known l4_daemon
env failures from B79–B81 records did NOT reproduce this session
(all-features 0 failed).

## 7. Dropped / pooled (with reasons)

- **Substitutability + router-exemption** (§3.3) — no bounded kernel
  fix-space; measurement-discipline record only.
- **ReplaySSM deferred-write lever** — the paper itself labels the
  attribution unvalidated; recorded as reopen intel on B81's
  `gdn-transition-replay-log-commit-many`, not new content.
- **Swaps/token top-k-corrected drift metric** — MoE routing methodology,
  not our substrate.
- **arXiv:2608.22110 (ANE measurement study)** — POOLED, not read: directly
  relevant to riir-ai Issue 726's owner-gated ANE reopen (the 7–8× gap
  diagnosis + program hot-set), flagged for that lane's owner; 23pp.
- **arXiv:2608.26368 (SIMD bit-slicing, 25pp)** — POOLED: rust_perf
  candidate, unranked until read.
- **arXiv:2608.23841 (Pipeline-Native Transformers, 77pp)** — POOLED: CPU
  decode bandwidth co-design; heavy read, deferred.
- **arXiv:2608.26523 (VPP chunked prefill)** — scheduling/dataflow across
  GPUs; our chunked prefill (Bench 710) is single-device. Low yield.
- **arXiv:2608.25062 (FLINT flash-resident LLM)** — NVMe scheduling; out of
  scope.

## 8. Provenance caveats

- TreeWY: B200 + vLLM fork, Qwen3.5 hybrids; numbers are evidence of the
  MECHANISM class, not transferable multipliers. The paper is single-author
  with concurrent-work honesty (Bole, ReplaySSM) — good sign.
- Launch-Bound: A100 serverless (Modal), torch 2.5 / Triton 3.1; the
  torch.compile specifics do not transfer to CubeCL/cudarc, but the
  launch-bound LAW and the capture-negative do (both are stack-independent
  structure claims, and both match our own measurements).
