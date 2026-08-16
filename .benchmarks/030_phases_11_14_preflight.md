# Bench 030 — Plan 005 Phases 11/12/14: substrate-first pre-flight

**Date:** 2026-08-16
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phases 11, 12, 14
**One command:** `cargo test --release --test phases_11_14_preflight -- --nocapture --test-threads=1`

**Verdict: all three phases are blocked on a MEASURED fact, not an opinion.
None is built.** Unlike Phases 9/10, the substrate here genuinely exists — the
blockers are about *data shape* and *empty target sets*, which is a different
and more interesting failure than a missing symbol.

## Substrate check (this time it passes)

| phase | substrate | status |
|---|---|---|
| 12 | `SweTrajectoryFreezer<N, D>` + `freeze_attempt` | **EXISTS** — `katgpt-core/src/swe_trajectory_freeze.rs:929`, exported |
| 14 | `HebbianKernelMemory<D>` | **EXISTS** — `katgpt-core/src/hebbian_kernel_memory.rs:277` |
| 14 | `DeltaMemoryState` | **EXISTS** — `katgpt-core/src/delta_mem/state.rs:55` |
| 14 | `MemorySteeredPruner<P>` | **EXISTS** — `katgpt-pruners/src/delta_mem/pruner.rs:45` (note: a *different crate* than katgpt-core) |
| 14 | `ExperienceReuseHook` / `query_priors_dual` | **EXISTS** — riir-ai `riir-engine/src/arg_runtime/experience_reuse.rs` |
| 13 | `GraphBuilder` / `ConvexSolver` / `Rounder` / `Certifier` | **EXISTS** — `katgpt-tokenizer/src/convex_{graph,solver,rounding,certify}.rs` |

## P11 — Phase 11's gate targets an empty set

Phase 11's rescoped G1 is *"imagination rejects ≥30% of would-fail-L2 fixes"*.
Fed real proposer output (what `--fix` actually hands L2), across every
`FIXABLE_LINTS` rule that produces a fix:

| | |
|---|---:|
| proposer outputs fed to L2 | 33 |
| L2 **accepted** | **33 (100.0%)** |
| L2 **rejected** | **0 (0.0%)** |

**The would-fail-L2 set is empty**, so G1 is a recall over zero positives —
undefined, not merely hard to hit. L2 is `RustAnalyzerPruner::validate`, a
`syn` parse; span-spliced proposer output is valid Rust *by construction*. The
validator that actually rejects is `fix_verify`'s `cargo check` (Issue 015),
which Phase 11 does not target.

This supersedes the previously-recorded block (a dependency on Phase 10
T10.1). That dependency is also softer than stated — T11.1 needs
`(pre, post)` embedding pairs, and both are derivable from the existing flat
`FixTrajectory` (`buggy_code`, `fixed_code`) without any PTG shape. **The empty
target set is the real blocker**, and it is measured.

## P12 — Phase 12 has no trajectory shape to classify

`SweTrajectoryFreezer::freeze_attempt` takes `trajectory: &[&[f32]]` — a
**sequence** of states, from which it computes displacement geometry. Phase 12
classifies those sequences into `converged_correct` / `committed_wrong` /
`oscillation`.

Measured, by re-applying each proposer to its own output (the `--fix` fixpoint
loop's second pass):

| | |
|---|---:|
| fixable rules producing a fix | 33 |
| **one-step** attempts | **33 (100.0%)** |
| **multi-step** attempts | **0 (0.0%)** |

**Every fix attempt converges in exactly one step.** A one-element sequence has
no displacement geometry, so the archetypes collapse — `oscillation` in
particular *cannot occur*, since oscillation is defined by a fix that keeps
changing across passes.

This is not a defect in the healer; it is a consequence of the proposers being
structural and idempotent by design (`FIXABLE_LINTS` = rules whose detection
alone guarantees semantics). The plan's Phase 6 note that "fix application
iterates to a bounded fixpoint so nested fixes compose" describes *different
rules* composing across passes, not one rule iterating — a distinction that
matters exactly here.

## P14 — T14.2's stated motivation does not exist

T14.2 proposes replacing `LatentSkillEvolution`'s `HashMap<String, Direction>`
with `HebbianKernelMemory<8>` for *"O(1) associative recall vs O(N) hash
scan"*. Measured lookup latency against a 100× increase in store size:

| store size | lookup |
|---:|---:|
| 40 keys | 10.8 ns |
| 4000 keys | 15.1 ns |
| **time ratio for a 100× size ratio** | **1.40×** |

**Lookup is already O(1)** — it is a `HashMap`, not a scan. There is no O(N)
cost to remove, so the swap's stated benefit is not available. (1.40× over 100×
is cache-hierarchy drift, not complexity.)

T14.5's gate is separately unreachable by the same arithmetic that closed
T10.6: it asks for **≥5% G1 precision improvement**, while measured remaining
headroom on the held-out multi-lint set is **4.0pp total** (24/25 spans already
correct at 96.0% top-1). Even a perfect implementation is under the bar.

## The pattern, now at seven

Tasks in this plan whose premise failed on inspection or measurement:
T3.5.1 (router can't move G1), T7.1 (EMA ≤ centroid), T7.3 (zero headroom),
Phase 9 pre-flight (`DraftedFix.confidence` unread), T9.4 (`clr_vote` absent +
effective K = 0.12), Phase 10 (`ConsolidationPipeline::can_freeze` absent +
gate unreachable), and now Phases 11/12/14.

The common cause is drift: the plan was written against a snapshot, and the
codebase moved — substrates were replaced (`pick_domain` → `TfCosine`),
validators changed shape, and, most importantly, **the retrieval baseline moved
from 54.2% to 96.0%**, which quietly invalidated every downstream gate phrased
as a ≥5–10% improvement. A gate is a claim about headroom, and headroom is a
moving target.
