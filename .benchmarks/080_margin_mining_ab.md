# Bench 080 — the `e^{−2M}` margin-weighted mining A/B (issue 077 T3)

**Status:** RECORD — T3 landed as a recorded **DEMOTE** 2026-09-09,
riir-clippy `095686a` (protocol + arm), `2e3ce9a` (substrate + offline
spectrum). `margin_mining` stays opt-in; the miner's default stays
diff-count-desc; no generation freeze (the arm did not win).

## The paper's claim under test

arXiv:2604.26841: cross-entropy implicitly maximizes classification margins
and `w(M) ∝ e^{−2M}` concentrates effort on the smallest-margin items.
Issue 077 T3 asks the mining question: does ranking carve-mining effort by
the drafter's margin beat ranking it by diff magnitude, at the same file
budget, on measured score-bench heal rate?

## Pre-registered protocol (written before the first run)

Verbatim from the miner's module doc (`examples/score_bench_mine_v6.rs`,
landed in `095686a` BEFORE any measurement):

> Mine TWO budget-capped candidate sets at the SAME `--files` budget from
> the same diff universe — control = count-desc (today's order), arm =
> margin-weighted — and measure BOTH on the real score bench
> (`run_bench_with`, the generation-1 instrument: real clippy, the frozen
> scoring rules). **Verdict rule, fixed before the first run: the arm WINS
> iff its heal rate STRICTLY beats the control's; a tie is a loss** (no
> gain = demote). … Set composition (both arms): every labeled carve
> carrying ≥1 vocabulary lint (multi + k==1 controls; no 15-control cap —
> the budget is `--files`).

Arm definition: every `.rs` file in the SAME diff universe is carved
(syn-only, no oracle); each carve gets the pre-rerank pool margin
`M = top1 − top2` (the `rank_into` canon — `total_cmp` + `rule_id`,
margins ≥ 0 by construction; min across enabled domains; `INFINITY` →
weight 0); files rank by total `e^{−2M}` mass, path tie-break, truncated
to the same `--files` budget.

G1 (margin contract) passed in `src/draft/margin_weight.rs` unit tests:
non-negative margins under the canonical sort (incl. the real-pool
integration half over the frozen v1 set), `e^{−2M}` monotone decreasing in
M, the ≥50%-in-half concentration inequality pinned on graded / steep /
all-tied populations. G3: the arm's ranking pass is two in-place sorts over
pre-allocated buffers — no per-candidate allocation in the ranking loop.

## T2 first — the offline spectrum (the premise check)

`examples/margin_spectrum.rs` over the frozen v1 fixture set (35 fixtures,
domains [Clippy, Perf, Docker], min pre-rerank pool margin across domains):

| bucket | n | mass |
|---|---|---|
| M = 0 (exact ties) | 0 | 0.0% |
| 0 < M ≤ 0.01 | 0 | 0.0% |
| 0.01 < M ≤ 0.05 | 3 | 22.9% |
| 0.05 < M ≤ 0.1 | 1 | 6.9% |
| 0.1 < M ≤ 0.5 | 12 | 56.0% |
| M > 0.5 / uncontested | 19 | 14.2% |

- All 35 spans contested (every span has ≥2 candidates in some domain).
- **Lowest-margin half (18 spans): 91.4% of the total `e^{−2M}` mass** —
  far above the structural ≥50% floor (which is an identity for any
  non-negative descending weights; the measured question was how far above).
- Lowest-margin decile (3 spans): 22.9% (≈2.3× uniform).
- Margin range: 0.037 (`katgpt-rs-ica_lens-013-clean`) → 15.0
  (`katgpt-rs-ica_lens-004-clean`).

T2 verdict: **the concentration premise HOLDS on labeled material** — the
smallest-margin spans carry a disproportionate share of the law's mass.
This justified building the T3 arm. It did NOT guarantee the arm wins on
raw material — see the A/B.

## The A/B — three budget escalations, one honest verdict

Oracle health probe before interpreting (the instrument first): a known
`uninlined_format_args` span labels `{"uninlined_format_args"}`, a known
`suboptimal_flops` span labels — the oracle labels correctly in this
environment. The empty results below are the material, not the instrument.

| run | repo @ sweep | budget | control carves | arm carves | control compiled/labeled | arm compiled/labeled | heal rates | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | riir-chain @ `7421245a` | `--files 6` | 19 | 46 | 7 / 1 | 5 / 0 | 100.0% (n=1) vs 0.0% (n=0) | vacuous — degenerate sets |
| 2 | katgpt-rs @ `5b2d42970` | `--files 10` | 85 | 91 | 0 / 0 | 18 / 0 | 0.0% vs 0.0% | tie → demote, escalated |
| 3 | katgpt-rs @ `5b2d42970` | `--files 30` | 225 | 242 | 22 / 0 | 18 / 0 | 0.0% vs 0.0% | tie → **DEMOTE (final)** |

**Verdict: DEMOTE.** The arm did not strictly beat the control at any
budget (the pre-registered tie rule); at the only budget with non-empty
material the comparison never became measurable. Per the rule: a tie is a
loss — no gain, demote. `margin_mining` stays opt-in; the miner default
stays count-desc (bit-identical with the feature off); no generation is
frozen (the freeze convention admits a new generation only on a win).

## What the starvation actually says (the honest finding)

At --files 30, **40 carves compiled across both arms and 100% of them were
lint-clean** under the current toolchain (40 labeled-carve yield: 0). Two
compounding mechanisms:

1. **Retrieval-similarity margin ≠ lint presence.** The margin is a
   text-similarity gap (the fused latent/BM25 score of the best matching
   corpus chunk). Bench 018's addendum already measured that adversarial
   CLEAN snippets score as high as or higher than true positives — there is
   no minimum-score threshold separating them. The margin law inherits that
   failure mode at the mining boundary: the smallest-margin raw spans are
   dominated by pattern-SHAPED but clean code (self-contained POC/demo fns
   that resemble two corpus rules while firing neither).
2. **Standalone-compile is the dominant filter on real material**, and it
   is order-insensitive: interdependent files (game loops, state machines)
   fail every tier regardless of ordering (control 0/85 at --files 10;
   22/225 at 30). The label yield of the survivors is ~0 under today's
   clippy (v6-era frozen labels partly reflect the August toolchain — the
   bench's own drift warnings).

Coherent overall story, honestly recorded:

- On LABELED (lint-bearing) material, margins concentrate (T2: 91.4% mass
  in the lowest half) — the law's premise holds where lints are known.
- On RAW mining material, the similarity margin does not select
  lint-bearing spans (T3: demote) — text similarity measures resemblance,
  not fixability.

## What would re-open the lane (pre-registered conditions, not vibes)

1. **An instantiation-aware margin**: the rerank layer's instantiation flag
   is the evidence signal similarity lacks; a margin computed on
   instantiation-first evidence (the `jury_disagreement::rerank_margin`
   view, or a hybrid) is a DIFFERENT ordering and would need its own
   pre-registered A/B.
2. **Mining over labeled material** (the v1-style corpus): where T2 shows
   the concentration is real, `e^{−2M}` ordering is a plausible
   review-priority policy — measurable without the carve-compile pipeline.
3. A toolchain-stable label source (the drift problem is not this issue's
   to fix).

## Provenance

- Protocol + arm: riir-clippy `095686a` (examples/score_bench_mine_v6.rs,
  `--order margin` + `--ab`, `margin_mining` feature).
- Substrate + T2: riir-clippy `2e3ce9a` (src/draft/margin_weight.rs,
  examples/margin_spectrum.rs).
- Provenance surface (the OOD-row family summary this doc's sibling
  reading rides on): riir-clippy `80e8667`.
- Runs executed 2026-09-09 on the M3 (CARGO_TARGET_DIR=/tmp/margin_mining,
  release profile; oracle spawns real clippy per batch). Oracle health
  probe: PASS (see above). No GPU involvement.
