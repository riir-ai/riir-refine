# 076 — Barrier-Free Multi-Engine Synchronization (arXiv:2608.13757) — distill verdict

**Status:** DISTILLED — pending owner decision (Batch 70 candidate pool OPENED in `.distill/001_mining_queue_snapshot.md`; Plan at batch start per the B60 plan-first discipline)

**Source:** Sung, Shyamsunder, Zhang, Kroening, Choi — "A Barrier-Free Synchronization Algorithm for Multi-Engine AI Accelerators" ([arXiv:2608.13757](https://arxiv.org/abs/2608.13757), cs.PL, 2026-08-13; Amazon + Cornell). Peer-review-quality provenance: Lean 4 bisimulation proof (~15K lines, **0 admitted lemmas**, CompCert-style `R_alloc` relation), measured on real Trainium 2 hardware (trn2.48xlarge) over production NKI kernels (tiled matmul, RMSNorm, FlashAttention, AdamW, Dropless MoE).

**TL;DR:** Trainium compiler pass that eliminates all-engine barriers in loops in favor of **per-dependency semaphore waits whose thresholds are computed at runtime in closed form from two loop counters** — a monotone total-entry register `r_ℓ` + a per-trip reset register `r̂_ℓ`. 10–45% kernel latency reduction vs the barrier baseline; 3.3× on a sync-bound microbenchmark; matches or beats hand-tuned manual allocation on most kernels. Route: **kernel_opt (D dispatch/sync)**, yield estimate **2–3 rules**. The two components with NO published prior art AND no shipped in-stack analog are (a) the closed-form dynamic wait thresholds, (b) the shared-counter decode `s ≥ N·r − (N−K)`.

## The mechanism (distilled)

1. **Dependency-satisfaction condition** (§4): a consumer at iteration vector `v` may issue iff the target `t = v − δ` **underflows** (some component ≤ 0 → no producer owed → no wait) OR `R(producer) ≥ Σ_{i⪯t} H(i, P)` (cumulative producer retirements through the target iteration). This is the *minimum synchronization for correctness* — issue earlier violates, later over-synchronizes.
2. **Two registers per loop** (§5.1): monotone `r_ℓ` (incremented at every entry, never reset — total entries) + trip `r̂_ℓ` (reset when the parent loop re-enters — entries in the current trip). Restricting the distance vector's nonzero component to the innermost common loop `S` makes every check a register comparison. Materializing full `H` is exponential in nesting depth — the restriction is what makes it 2 registers.
3. **Three allocatable cases** (§5.2): producer in `S` (forward or backward, any offset `k`) → wait `r_P − k`; producer nested in `P ⊂ S`, forward (offset 0) → `r_P`; producer nested, backward (offset 1) → `r_P`. Everything else falls back to barriers.
4. **Branchless wait collapse** (§5.3): `waitVal = isGT(r̂_S, k) · expr` — the no-wait disjunct becomes a 0/1 mask multiplied into the threshold; a single semaphore wait with a register threshold, 3 fixed ops, no branch.
5. **Per-loop semaphore sharing** (§5.5): one semaphore per (loop, engine) pair; in-order retirement keeps all retirement counts in the loop within 1 of each other, so per-producer progress decodes as `s_loop ≥ N·r − (N−K)` (N = body instruction count, K = producer position). 5 ops/issue.
6. **Granularity ladder** (§6): per-instruction `O(N)` semaphores / 3 ALU ops; per-loop `O(L·E)` / 5 ops; per-engine `O(E)` / `O(D)` ops (D = loops on the path). LICM hoists per-loop partial sums out of deeper loops; CSE shares the mask + partial sums across consumers with a common ancestor. A compiler may MIX granularities per region.

**Measured shape worth encoding:** the ONE loss case (TiledMatmul 3-loop, 0.90 vs manual 0.71) is where the wait-value ALU lands on the critical path of the *same engine* running a tight compute-bound inner loop — resolved by **composing with selective inner-loop unrolling** (0.55 = par with unrolled), not by abandoning the scheme. Unrolling loses on code size (6.1 KB → 508.6 KB) and cannot handle dynamically-bounded loops at all (Dropless MoE N/A).

## Prior-art verdict (web search, 2026-08-19)

| Component | Prior art? | Closest |
|---|---|---|
| Compiler barrier elimination | Yes (abundant, 1995–2006) | Tseng PPOPP'95; Han & Tseng IJPP'98; Kandemir 2006 — static, SPMD/shared-memory |
| Point-to-point dependency sync | Partial | Weft/Singe 2015 (GPU CTAs, static dataflow, verifier ≠ verified compiler) |
| Dependency-driven sync insertion on Trainium | Concept (docs) | Neuron Compiler docs — no published algorithm |
| **Runtime-computed closed-form wait thresholds from loop counters** | **None found** | PLDI'23 sparse fusion is adjacent, different purpose |
| **Shared-counter decode `s ≥ N·r − (N−K)`** | **None found** | — |
| Lean bisimulation of a sync-elimination pass | None found | IronFleet/Verus are different provers/domains |

The **combination** is novel; the two headline ancestors (barrier elimination, p2p sync) are honestly old. Per-component: treat (a)+(b) as the transferable GOAT candidates; the Trainium numbers are vendor-platform validation, not the contribution.

## Codebase audit (2026-08-19, grep audit)

| Paper technique | Ships? | Where / gap |
|---|---|---|
| 1. Per-dependency precise waits | Host-side PARTIAL; device-side NO | Corpus: `chain-dispatches-single-sync-point` (N syncs → 1 at chain END — still one coarse sync, no loop), `sibling-dispatches-before-any-readback`, `first-readback-pays-queue-drain` — all queue-ordering heuristics, not per-dependency thresholds. The 129 `workgroupBarrier` sites (35 WGSL files) are intra-kernel smem visibility — a different level, and B57 proved some irreducible. |
| 2. Dynamic wait thresholds from loop counters | **NO analog** | Nothing computes wait values from monotone/per-trip counters. |
| 3. Shared-counter decoding | **NO analog** | No shared-counter → per-producer progress recovery. |
| 4. Masked branchless wait | **NO analog** | Nearest: count-bounded consumption (`flashprefill_score_select.wgsl`), reduction-identity masking. |

riir-net's `Semaphore(64)` pipelining (`MAX_INFLIGHT_STREAM_WRITES`, the io_uring SQ-depth pattern) is related vocabulary but a different mechanism — fixed-cap backpressure, no per-dependency thresholds, no loop counters.

## Candidate rules (Batch 70 pool)

1. **`barrier-free-per-dependency-wait-from-loop-counters`** (D; STRONG) — when a loop over heterogeneous workers carries cross-iteration dependencies, do NOT synchronize all workers at iteration boundaries (drain + handshake + reset); track a monotone entry counter + a per-trip counter per worker and compute each dependency's wait threshold in closed form (`r_P − k` for producer-in-loop; `r_P` for the nested forward/backward cases), collapsing the no-wait disjunct with a 0/1 mask. Ref shape: the paper's motivating example — 3 engines, ONE loop-carried dependency at offset 2, yet every iteration boundary drains all three. **Distinct from** `chain-dispatches-single-sync-point` (acyclic chain, single end sync — no loop, no counters), `expert-loop-two-phase-split-batch-across-iterations` (RESTRUCTURES the loop into 2 batched phases to avoid per-iteration syncs — this computes the exact per-dependency moment instead), `sibling-dispatches-before-any-readback` (ordering, not thresholds). **Applicability caveat (honest):** wgpu/CubeCL/Metal host APIs expose no fine-grained semaphore — the live surfaces are (a) cudarc CUDA events + multi-stream (`cudaStreamWaitEvent` IS a per-dependency wait), (b) host orchestration loops (experts/layers/positions) where per-iteration `poll` is the barrier analog, (c) multi-threaded worker loops. The 3.3× number is Trainium-specific — encode the MECHANISM, cite the number as source evidence.
2. **`shared-counter-decode-producer-progress`** (D; MEDIUM) — one shared counter per (loop, worker) + in-order retirement → decode individual producer progress via `s ≥ N·r − (N−K)`; the granularity ladder (per-instruction `O(N)` semas / per-loop `O(L·E)` / per-engine `O(E)` + `O(D)` ALU) is the decision content — trade shared-resource count against per-issue decode ALU, mixable per region.
3. **`masked-branchless-wait-collapse`** (A; MEDIUM) — a disjunctive wait predicate (no-wait OR threshold) → single threshold via 0/1 mask multiply (`isGT`); 3 fixed ops, no branch. The branchless-conditional family applied to sync predicates.
4. **Drops (recorded, with reasons):** LICM + CSE of wait values (§6.2) — ambient compiler knowledge, the corpus's hoisting rules already carry the shape; "selective unrolling composes with precise sync" — corpus ground (B35-class lesson shape); the Lean proof structure — methodology, not a rule (fusion cross-ref below).

**Yield estimate: 2–3** (1 strong + 1–2 medium; dedup risk LOW on the lead — the D category was audited, no threshold-from-counters content exists).

## Fusion angles

- **FV moat extension** (cross-ref `katgpt-rs/.research/351_cross_repo_lean4_fv_pattern.md`): the paper's proof shape — spec transition system with the dependency-satisfaction predicate ↔ impl transition system with counters/semaphores, bisimulated through a CompCert-style `R_alloc` — is exactly our 4-instance FV pattern. IF a barrier-free orchestrator ever ships as rule 1's consumer, the proof template is ready-made and smaller than the paper's 15K lines (no ISA modeling needed for a host-side orchestrator).
- **kimi_k3 MoE expert loop** — the live consumer candidate: B40's `expert-loop-two-phase-split-batch-across-iterations` was the restructure-and-batch answer (2+2k_r+2 syncs → ~3); a CUDA-events multi-stream variant is the precise-wait answer (→ per-dependency). Reopen trigger: the expert loop's sync count becoming the measured bottleneck again.
- **rust_perf cross-ref**: the pattern also covers host multi-threaded loops (join-per-iteration → per-task counters) — no live in-tree consumer identified; kernel_opt D is the home, note the cross-domain vocabulary (`join_all`, `Notify`, epoch counters) for retrieval.

## Verdict

**Gain — Batch 70 candidate pool OPENED (kernel_opt D, yield 2–3).** Not Super-GOAT: Q2 fails (sync perf, not a new behavior class), Q3 fails (no product selling point). Q1 is strong for corpus purposes — components (a)+(b) have no published prior art and no shipped analog. Game-context reframe run and honestly negative: the 20 Hz tick is a cadence, not a barrier to eliminate; per-NPC cognitive scheduling is already adaptive (salience-gated, `decay_confidence`) — confirms the kernel_opt routing.

> Route: kernel_opt (D dispatch/sync) · Batch 70 pool · Plan at batch start (`.plans/NNN`) per the B60 plan-first discipline.
