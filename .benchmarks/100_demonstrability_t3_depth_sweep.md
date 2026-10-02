# Bench 100 — Issue 134 T3: the ρ-prediction depth sweep (standing REFUSAL)

**Status:** DONE — instrument landed; verdict REFUSED at the power floor (the designed answer at the current fixture population).
**Date:** 2026-09-24
**Source:** `Issue 134` T3 · [katgpt-rs Research 585](../../katgpt-rs/.research/585_SAT_Teamwork_Strategy_Banks_Demonstrability.md) · SAT arXiv:2609.22682
**Harness:** `examples/demonstrability_t3_sweep.rs` (`cargo run --release --features latent_retrieval,clippy_lints,rust_perf[,kernel_opt] --example demonstrability_t3_sweep`)
**Box:** 4090 workstation (i7-13700K), Windows 11, AC power; cargo build/release under light concurrent load (sibling agent eval running; the sweep itself is CPU-deterministic retrieval — no GPU, no timing claims).

## What was measured

Per-domain **gain-from-depth** (top-1 accuracy as the retrieval POOL deepens
28 → 56 → 112 — depth means pool, not truncation: a deeper pool changes the
rerank's candidate set; truncation cannot change top-1) against the **T1
demonstrability constant**, on the labeled fixture set, both axes from the
SAME run. Spearman ρ over the qualifying domain set with the issue's power
floor: **REFUSED below five qualifying domains** (at n=3 Spearman cannot
reject; at n=4 it cannot reach the paper's p — the tail-support rule applied
to a rank statistic).

## The sweep table (n=3 posture, `kernel_opt` on, 131 labeled spans)

| domain | labeled | top1@28 | top1@56 | top1@112 | demonstrability |
|---|---|---|---|---|---|
| clippy | 48 | 89.6% | 89.6% | 89.6% | 0.935 |
| rust_perf | 53 | 96.2% | 96.2% | 96.2% | 0.962 |
| kernel_opt | 30 | 100.0% | 100.0% | 100.0% | 1.000 |

**Verdict: REFUSED — power floor n=3 < 5.** The n=2 posture (default
features) is the same refusal with clippy + rust_perf only.

## Readings worth recording (measurement, not verdict)

1. **The constant column reproduces T1's standing readings exactly**
   (clippy 0.935 / rust_perf 0.962) — the harness's taxonomy rides the
   eval's own top-8 probe window (`take(8)` before `classify_choice_span`;
   a first draft classified over the full 28-deep list and read clippy at
   0.915 — the window IS part of the constant's semantics, silently widening
   `wrong_rule` at `wrong_span`'s expense. Fixed before landing).
2. **Every depth curve is FLAT** — zero gain-from-depth in all three
   domains, at constants 0.935–1.000. The rerank's head is stable under
   2× and 4× pool depth: the instantiation-first sort never promotes a
   rule from deeper in the pool past an already-instantiating head. At
   face value this is the paper's law holding (high demonstrability ⇒
   selection already lands the correct candidate ⇒ depth adds candidates
   selection never needs) — but at n=3 it is a description, not a verdict,
   which is exactly what the power floor is for.
3. **Consequence for T2's direction**: the flat curves say the CURRENT
   corpora give the gate's "deepen high-demonstrability domains" half
   nothing to buy at any factor — consistent with the standing constants
   rounding to inert (factors 0.990/1.011 at pool 28). The gate stays
   default-off regardless (T3 not passed); if a future corpus makes depth
   pay somewhere, the sweep is the instrument that will see it.
4. **kernel_opt reads 1.000** — its 30 fixtures are corpus-derived
   (`kernel_fixture_spans(30)`, mutated corpus patterns), so perfect
   selection is partly self-referential (the fixture IS the corpus entry).
   A future authored kernel fixture set would de-bias that row; noted, not
   fixed here (the row never reaches a verdict alone).

## Re-arm triggers

- `sec`/`docker`/`dist`/`shader_perf` gain authored labeled fixture sets
  (each adds a qualifying domain; n ≥ 5 unblocks the Spearman);
- a bench-half standing row lands (the next `--score-bench` run — the
  bench formula attributes through the same `RuleDomains` table and can
  qualify domains the eval half cannot reach);
- the sweep then re-runs and the verdict flips from REFUSED to
  PASS (ρ ≥ 0.5 → promote `demonstrability_gate` per the standard rule) or
  FAIL (record the negative; T1 survives as instrumentation).

## Gates

- clippy clean at the example's feature set (`-D warnings`);
- constants parity-pinned against T1's standing readings (0.935/0.962) by
  the run above;
- no corpus delta (freeze tables untouched); no production-path change
  (measurement-only example).
