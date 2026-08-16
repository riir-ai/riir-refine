# Bench 029 — Plan 005 T9.4: K=8 drafts + CLR voting, refuted on four counts

**Date:** 2026-08-16
**Plan:** [005](../.plans/005_real_file_span_healing.md) Phase 9, T9.4
**One command:** `cargo test --release --test t94_clr_vote_preflight -- --nocapture`

**Verdict: T9.4 is not implementable as specified, and the version that could
be built has nothing to vote over. NOT BUILT.** Four independent refutations,
three of them measured.

## The proposal

> *"Generate K=8 drafted fixes (currently 1) via QMC perturbation of the
> ternary matvec, cluster by syntactic similarity (Levenshtein on AST
> node-kind sequence), run L2 pruner on each, `clr_vote` picks the
> most-reliable cluster's winner."*

## R1 — `clr_vote` does not exist (substrate check)

The plan lists `clr_vote` as verified-to-ship. Grep across `katgpt-core`: there
is **no `clr_vote` function**. What exists is
`clr_reliability_scores(states, directions, m_exp, n, d, output)` (feature
`clr_weighted_set_attention`, default-on) — per-entity reliability from probe
directions — plus a `clr_vote: Option<f32>` *parameter* on
`compaction::gate`, which is an input to a gate, not a voting primitive. The
vote would have to be written, not composed.

Third stale substrate claim in this plan, after T7.2's `pick_domain<3,8>`
(replaced by `TfCosine` in Issue 020) and T3.5.3's `BonsaiEmbedder` 768-D
(the `Embedder` trait returns 8-D).

## R2 — a "draft" is a selected corpus row, not a generated fix

`TernaryDrafter` does not generate code. It BLAKE3-hashes
`(rule_id, buggy_code)`, runs a ternary matvec, **FNV-1a folds the output into
an index**, and returns `candidates[idx].fixed_code()` verbatim
(`src/drafter.rs`). So "K=8 drafts" means 8 corpus rows chosen by 8 hashes —
there is no generative diversity to sample.

## R3 — the ensemble collapses: effective K = 0.12, not 8 (measured)

24 corpus fixtures × K=8 perturbations = 192 attempts:

| | |
|---|---:|
| identity guard **declined** | **189/192 (98.4%)** |
| fixtures yielding any draft at all | 3/24 |
| distinct drafts per producing case | **1.00 of 8** |
| **effective K** | **0.12** |
| drafts that are the target lint's fix | **0/3 (0.0%)** |

The identity guard (Issue 010: decline when the emitted fix shares no
identifier with the buggy code) exists precisely because hash-selection is
"confidently wrong" most of the time. It rejects 98.4% of perturbations, so the
ensemble T9.4 wants to vote over never materializes — and the survivors are
*other lints'* fixes that happen to share a token.

Voting requires alternatives. There is 1 survivor per producing case, and it is
wrong 100% of the time.

## R4 — the L2 filter is a no-op by construction (measured)

T9.4 filters candidates by "run L2 pruner on each". L2 is
`RustAnalyzerPruner::validate` — a `syn` parse, with `lint_resolved` hardcoded
`true` on the fast path. Every corpus `fixed_code` is valid Rust by
construction, so:

**L2 accepted 3/3 (100%) of drafts.** It cannot discriminate between them.

## Why this was measured instead of built

The four tasks measured-before-built in this plan (T3.5.1 centroids, T7.1
evolved directions, T7.3 memory axis, and the Phase 9 pre-flight on
`DraftedFix.confidence`) each rested on a premise that no longer held. T9.4
makes five. The recurring shape is worth naming: **a plan task that names a
substrate and a metric is not evidence that either still connects to the
shipped path.** Grep for the function; grep for the consumer; measure the
mechanism's headroom — before writing the feature.

## Reopen conditions

T9.4 becomes meaningful only if the drafter becomes *generative* (producing
distinct candidate fixes rather than indexing corpus rows) **and** a validator
with real discriminative power replaces the parse-only L2 — `clippy_verify` or
`fix_verify` both qualify, since they run real `cargo clippy`/`cargo check`.
Until both hold, there is nothing to cluster and nothing to vote on.
