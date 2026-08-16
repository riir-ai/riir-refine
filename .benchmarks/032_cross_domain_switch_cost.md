# Bench 032 — Issue 024: cross-domain switch-cost measurement (retrieval axis)

**Commit:** `e34ade9` (measurement + mitigation) · this doc (verdict record)
**Filed from:** [Issue 024](../.issues/024_cross_domain_switch_cost_measurement.md) · [katgpt-rs Research 484](../../katgpt-rs/.research/484_Skill_Entropy_Directed_Switch_Cost.md)
**Question (falsifiable):** does an anti-pattern retrieve its rule worse when a *different domain's* anti-pattern is co-present in the span than in isolation — the Skill-Entropy paper's switch-cost failure class, asked of our dense fan-out retrieval?

## Method (paired, fresh-baseline)

12 directed clippy×perf pairs × 2 textual orders = **24 merged spans** (`src/draft/eval_fixtures_cross_domain.rs`; constituents are the exact verified solo fixture texts, so the solo baseline is measured in the same process against the same corpus — immune to corpus drift). For each span: the **switching** (second-position) rule and the **lead** (first-position) control, each ranked solo vs co-present at probe depth 8.

Fixture authoring excluded every pair where a constituent fires a second proposer or fails the no-dep oracle crate (details in the file header); `cross_domain_fixture_labels_verify_via_proposers` pins zero cross-fires, and `tests/cross_domain_oracle_truth.rs` pins **real-clippy truth**: all 24 merged spans compile and carry *exactly* the one labeled corpus lint (24/24 PASS).

## Result at `RERANK_POOL = 8` (pre-mitigation)

| direction | switch solo @1/3/5 | switch co @1/3/5 | Δ pp |
|---|---|---|---|
| clippy→perf (perf switches in) | 100/100/100 | 100/100/100 | **0.0** |
| perf→clippy (clippy switches in) | 100/100/100 | 91.7/91.7/91.7 | **+8.3** |

Signal criterion (≥5pp): **met** — but by exactly one pair: `needless_return`+`vec-with-capacity`, in BOTH textual orders (order-independent ⇒ crowding, not a direction effect). 11/12 pairs clean; the perf-switching direction perfectly robust.

### Mechanism (probe, not conjecture)

On the merged span the clippy-domain retrieval floods with loop-shaped rules — `range_plus_one` 45.6, `needless_range_loop` 36.9, `iter_next_loop` 30.9, `while_let_on_iterator` 27.1, `iter_nth_zero` 25.9 — versus the solo winner's 15.4. All of them `instantiates = false`. `needless_return` (which *does* instantiate — pinned by the label test) is crowded to **raw similarity rank 9**, one position past the re-rank pool of 8: evicted before the instantiation re-ranker ever sees it. The re-ranker is the designed crowding safety net, but it can only promote what was retrieved.

The pool-depth probe (pool swept 8→40, one step at a time) pins the number exactly: raw rank **9**, re-entry at rank **0** the moment the pool reaches 9 — both orders.

## Mitigation: `RERANK_POOL` 8 → 12

Deeper pools only add visitor walks, never new promotions (a structural proposer that does not match stays `false` at any depth; `clean_spans_fire_no_lint` still guards clean input). Committed in `e34ade9` with `pool_depth_rescue_guard` pinning the rescue (rank 0 at eval probe depth, both orders).

### Post-mitigation measurements

| check | result |
|---|---|
| switch cost, both directions | **0.0pp, zero drops** — dense fan-out is switching-cost-free at the corrected pool depth |
| Issue 017 eval (2-domain) | clippy **89.6/95.8/95.8** (was 87.5/93.8/93.8), perf 97.9 flat, blended **93.8/96.9/96.9** (was 92.7/95.8/95.8), router annotation 68.8% — the deeper pool rescued *other* latent near-misses too; floors re-pinned (blended 89/92/92, clippy 42/45/45, perf 46/46/46) per the measured−1-span convention |
| Issue 017 eval (3-domain) | kernel self-variants 100/100/100 (crowding alarm quiet), blended 95.2/97.6/97.6, router 70.6% |
| T3.5.5 uncontaminated composite G1 | structural **96.0%** / off 84.0% → **+12.0pp — identical to the pool-8 promoted number**; pool recall @12 = 100% (25/25); degeneracy check still GENUINE RE-RANKING |
| T3.5.5 original-fixture set | pool recall @12 = 97.7% (42/43); still degenerate as documented (single-lint fixtures) |
| Rerank latency (G2) | Off 559.5µs → Structural 599.1µs/span = **+39.6µs (+7.1%)** — same envelope as the pool-8 promotion (+5.6–7.2%) |
| Lib tests | 419 default / 707 all-features, clippy 0 warnings in this crate both states |

## Verdict (T4)

1. **Signal detected** by the issue's letter (8.3pp ≥ 5pp) — but at n=12 per direction one span *is* 8.3pp, and the per-span detail shows it is exactly one pair, both orders.
2. **Mechanism = retrieval pool crowding, not fix interference.** The correct rule was never lost — it was one position outside the pool the re-ranker reads.
3. **The issue's anticipated conclusion is refuted**: a gap does *not* license the fix-ordering `SwitchCostTable` consumer (katgpt-rs Issue 663). Ordering fixes by switch cost addresses a different mechanism than the one measured. The indicated mitigation was pool depth, and it is landed.
4. **Dense fan-out vindicated** (the issue's no-gap branch): at the corrected pool depth both directions are 0.0pp with zero drops — and the standing eval improved.

### T3 (fix-axis measurement) — skipped, honestly

The fix axis (`--fix --verify` revert rates cross-domain vs same-domain) measures *edit interference between proposers*. The measured gap lives entirely on the retrieval axis, and the standing evidence for the fix axis is already strong: the cross-fire label test asserts the two domains' proposers never both fire on a merged span (disjoint spans, disjoint edits — nothing to interfere), and `fix_verify`'s compile gate auto-reverts regardless of domain. Measuring a null mechanism would be bench theater.

## Granularity caveat

One span = 8.3pp at n=12. The aggregate table is the verdict; the per-pair probe is the evidence. If future corpus growth moves the crowding frontier past 12, `pool_depth_rescue_guard` fires and the raw-rank discovery sweep prints the new depth.

## Research 484 back-reference (T5)

Appended to [Research 484](../../katgpt-rs/.research/484_Skill_Entropy_Directed_Switch_Cost.md)'s riir-clippy consumer row: measured +8.3pp → mechanism pool-crowding at raw rank 9 → mitigated (`RERANK_POOL` 12) → 0.0pp; P2 gating resolves **no consumer warranted** on the fix-ordering axis.
