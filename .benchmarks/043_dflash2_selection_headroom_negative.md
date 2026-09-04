# Bench 043 — DFlash2 selection-headroom POC: T1 measured NEGATIVE

**Date:** 2026-08-21 · **Repo:** riir-clippy · **Verdict:** T1.2 stop rule FIRES — no selector built
**Refs:** `Issue 034` (closed by this doc) ·
[Plan 017](../.plans/017_dflash2_selection_headroom_poc.md) ·
[Research 077](../.research/077_dflash2_draft_only_distillation.md)
**Harness:** `src/draft/headroom_probe.rs` (fixture surface) ·
`tests/uncontaminated_g1.rs` (real-clippy surface) · commit `78d1934` + this commit

## The question

Research 077 established the released DFlash2 GGUF cannot generate standalone (no `tok_embd`,
no `lm_head`, encoder consumes 5×5120 target features/position), so the only part of DFlash 2
that transfers to this crate without target weights is its **selector**. DFlash 2's Table 1
reports Recall@1 85.4% vs Recall@16 99.5% — a ~14pp band where the correct answer is already in
the candidate pool and per-candidate scoring fails to put it first.

A selector can only recover headroom that exists. T1 measures whether it exists here.

## Reproduce

```bash
# Surface A — Issue 017 labeled fixtures (101 spans), our labels
cargo test --release --features latent_retrieval --lib draft::headroom_probe -- --nocapture

# Surface B — real `cargo clippy` ground truth (slow: one clippy run per span)
cargo test --release --features latent_retrieval,clippy_verify \
  --test uncontaminated_g1 t1_dflash2 -- --nocapture --test-threads=1
```

## Method

Each pool depth gets its **own query**. The matcher retrieves `max(top_k, RERANK_POOL)` and
re-ranks *the pool*, so depth is a configuration, not a slice of one deep list — a depth-16 probe
can promote a rule the depth-12 production path never saw. `recall@1` is always the production
selection at `RERANK_POOL = 12`.

Misses split into **selection** (correct rule in the pool, not chosen) vs **retrieval** (never
retrieved — corpus/embedder work no selector can touch).

## Raw table 1 — surface A, headroom vs the SHIPPED re-rank knob (101 spans)

| mode | recall@1 | pool@8 | pool@12 | pool@16 | headroom | sel-misses | ret-misses |
|---|---|---|---|---|---|---|---|
| `RerankMode::Off` | 84.2% | — | — | 100.0% | +15.8pp | 16 | 0 |
| `Structural` (shipped default) | 93.1% | 99.0% | 99.0% | 100.0% | +6.9pp | 7 | 0 |
| `All` | 97.0% | — | — | 100.0% | +3.0pp | 3 | 0 |

Per-domain at the shipped default: clippy 89.6% → 100.0% (+10.4pp, 48 spans),
perf 96.2% → 100.0% (+3.8pp, 53 spans). **Zero retrieval misses at any mode** — a 16-deep pool
always contains the correct rule, so the band is entirely an ordering question.

## Raw table 2 — surface B, real `cargo clippy` ground truth

| criterion | spans | recall@1 | pool@8 | pool@12 | pool@16 | headroom |
|---|---|---|---|---|---|---|
| composite multi-lint, any-true | 25 | 100.0% | 100.0% | 100.0% | 100.0% | +0.0pp |
| composite multi-lint, fixable-true | 25 | 100.0% | 100.0% | 100.0% | 100.0% | +0.0pp |
| composite, **control** `rerank=off` | 25 | 84.0% | 100.0% | 100.0% | 100.0% | **+16.0pp** |
| Issue 017 clippy fixtures, our label | 43 | 90.7% | 97.7% | 97.7% | 100.0% | +9.3pp |
| Issue 017 clippy fixtures, **real clippy label** | 43 | 90.7% | 97.7% | 97.7% | 100.0% | +9.3pp |

Two instrument checks, both of which had to pass before any of this could be believed:

1. **Saturation is not blindness.** The composite surface reads 0.0pp with recall@1 pinned at
   100%, which alone cannot distinguish "no headroom" from "cannot see". The `rerank=off` control
   on the *same spans with the same metric* reads +16.0pp with 4 selection misses — the metric has
   power, so the shipped 0.0pp is a real negative. The shipped T3.5.5 instantiation re-ranker has
   already consumed that entire band.
2. **Surface A's band is not a labeling artifact.** Surface A scores against ONE hand-authored
   label per span, so a top-1 that clippy also reports would count as a miss spuriously. Re-labeling
   the identical span subset with real clippy gives **the identical +9.3pp and the identical four
   misses** — `identity_op` and `unnecessary_mut_passed` are genuinely not lints on those spans.
   The band survives real ground truth.

## Raw table 3 — the decomposition that decides it

The raw band is NOT the band a new selector could win. Attributing each of the 7 residual
selection misses under the shipped default to what would actually fix it:

| cause | spans | pp | detail |
|---|---|---|---|
| type-blind exclusion (`RerankMode::All` recovers) | 4 | 4.0 | `useless_conversion` r1, `needless_borrow` r6, `single_char_pattern` r1, `comparison_to_empty` r1 |
| pool depth (deeper `RERANK_POOL` recovers) | 1 | 1.0 | `redundant_closure` at rank 15 — outside the pool of 12 |
| unfixable target (no proposer → selecting it heals nothing) | 2 | 2.0 | `Perf/dyn-to-enum` r3, `Perf/shrink-to-fit-after-build` r5 |
| **SELECTOR-ADDRESSABLE** | **0** | **0.0** | none |

- The 4 **type-blind** rows are the measured *price* of a decision this crate already made:
  `RerankMode::Structural` counts only `FIXABLE_LINTS` as instantiation evidence, deliberately
  excluding `needless_borrow` / `useless_conversion` / `unnecessary_mut_passed` /
  `redundant_closure` because Bench 022 measured them over-matching on ordinary production code and
  Bench 027 kept the exclusion (`All` only *ties* `Structural` on quality). A pairwise-coherence
  selector reading the same parse-only latent features has no type information either, so it cannot
  beat that precision/recall trade — it can only re-pay its price, which is exactly the
  `created_rate` regression Plan 017's G2 forbids.
- The 1 **pool-depth** row is a `RERANK_POOL` 12→16 question, not a selection question.
- The 2 **unfixable** rows target perf rules absent from `FIXABLE_RULES` — as are the rules that
  outranked them. Even a perfect pick yields a `--suggest` reorder that `--fix` declines: zero
  `heal_rate` gain. Writing the proposer is the fix, not selecting better.

## Verdict — T1.2 stop rule FIRES

**Selector-addressable band: 0 of 101 spans (0.0pp), against a 2.0pp bar.** Every residual miss is
attributable to an already-shipped knob (re-rank mode, pool depth) or to missing proposer coverage.
Plan 017 Phase 2-4 (`dflash_selector`, gates G1-G5, promotion) are closed unbuilt: G1 is
unreachable by construction, since "recover ≥50% of the addressable headroom" has an empty
denominator.

The DFlash2 selector premise **is real in our domain** — it is just already harvested. The
`Off → Structural` step recovered 8.9pp of a 15.8pp band and saturates the uncontaminated surface
outright. Our T3.5.5 instantiation re-ranker *is* the selector DFlash 2 contributes, arrived at
independently and with strictly better evidence (proposer-visitor instantiation beats latent
pairwise coherence because it is a *semantic* witness, not a similarity).

This closes Issue 034. POC-B (artifact L4) stays deferred on the Research 077 + DSpark
show-stopper; local weights untouched for the conditional reopen.

## Follow-ups (not selector work — do NOT reopen as such)

- `RERANK_POOL` 12→16: recovers `redundant_closure` (1 span). Costs visitor walks on 4 more
  candidates/domain/span. Worth a measurement against the Bench 018 latency budget; file separately.
- Perf proposer coverage for `dyn-to-enum` + `shrink-to-fit-after-build`: turns 2 currently
  unfixable top-1 candidates into healable ones. Ordinary corpus work.
- The `Structural` vs `All` trade is re-measurable, not re-litigated: Bench 022/027 own it.
