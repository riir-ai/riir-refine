# Issue 082: OOD fresh-family pairing — the convergent-validity experiment (740 promotion unblock)

**Status:** LANDED 2026-09-09 — experiment run, verdict recorded below; example `examples/ood_fresh_family.rs` + bench record `.benchmarks/081_ood_fresh_family_convergent_validity.md` (this commit); katgpt-rs 702 addendum updated cross-repo.
**Date:** 2026-09-09
**Parents:** [077](077_margin_weighted_mining_entropy_gap_ood.md) (untouched — its landed record stays) · katgpt-rs Issue 740 / Bench 702 addendum ("unblock = a reference/OOD pairing with disjoint provenance families")
**Example:** `examples/ood_fresh_family.rs` (print-only, oracle-free, required-features `score_bench,latent_retrieval` — the margin_spectrum precedent)

## Ask

Run #84's `anomalous_negative` (ref = v1-frozen 3.840 nats vs ood = gen6 3.110 nats, gap −0.729, d −1.11) was ATTRIBUTED to set composition (v1 mostly authored, gen6 all-real) — real code is sharper than authored composites. That attribution was UNTESTED. This issue runs the disambiguating pairing: **composition held constant (real vs real), provenance held disjoint.**

- **Reference (corpus-family, real):** the 17 gen6 spans (`RAW_SPANS_V6`) — real pre-heal states of the same sibling repos the rule corpus was mined/authored against.
- **OOD-A:** top-level `fn` carves from `../riir-neuron-db` @ `2f30800` (working-tree HEAD; absent from every known set).
- **OOD-B:** top-level `fn` carves from `../riir-dapps` @ `c4c6ffe` (working-tree HEAD; absent from every known set).

If the entropy gap tracks corpus-family proximity (convergent validity — the axis detects memorization), the corpus-family reference fires sharper than BOTH fresh families. If the gap vanishes on real-vs-real, the axis was only ever measuring authored-vs-real texture.

## Pre-registered readings (fixed BEFORE the first run; mirrored verbatim in the example's module doc)

- Sign convention: `gap = mean_ref − mean_ood` (the axis's internal `mean_gap` is the exact negation; both printed). `d` = the katgpt-core kernel's `standardized_gap`, sign-flipped to match.
- **R1 — convergent validity demonstrated:** `gap < 0` AND `|d| ≥ 0.5` on BOTH fresh families ⇒ the axis tracks corpus proximity on composition-matched real code; #84's anomalous_negative WAS composition, as attributed.
- **R2 — convergent validity fails on real data:** `|gap| < 0.5` nats OR `gap > 0` on BOTH ⇒ the axis measures authored-vs-real texture, not memorization; the probe stays a lab instrument (702 records it honestly).
- **R3 — inconclusive:** families disagree (mixed arms), or either family yields < 20 carves (pre-registered yield floor) ⇒ report per-family, stop.
- Per-family arm: R1-arm iff `gap < 0 && |d| ≥ 0.5`; R2-arm iff `|gap| < 0.5 || gap > 0`; else ambiguous. Both R1 ⇒ R1; both R2 ⇒ R2; any mix ⇒ R3.
- **Uncontested-pool edge (decided, pre-registered):** ALL parsed spans enter the entropy set — an all-flat edit-count vector is the legitimate "no rule fits" high-entropy signal (max ln V), not a measurement failure. The contested/uncontested split is reported per set under BOTH censuses: rule-vector (≥ 2 rules fired) and latent pool (`span_margin`, the margin_weight access pattern — contested_domains ≥ 1). Contested-only mean entropy rides alongside as a robustness view; it does NOT gate the verdict.
- **Oracle-free by construction** (a feature, not a shortcut): the probe consumes proposer edit-count score vectors (`rule_score_vector`, the OOD axis's own statistic) + the latent census; no clippy oracle runs anywhere.

## Carve protocol (deterministic; no sampling, no randomness, no seeds)

`collect_source_files_sorted` (hidden/`target`/`vendor` pruned) → `syn::parse_file` per file (failures counted + skipped) → top-level `syn::Item::Fn` only, non-empty body, sliced by `item.span().byte_range()` (the `score_bench_mine_v6` carve convention) → ≥ 200-byte floor → per-file cap 2 → stop at 24 per family. Fresh carves measured as raw item text, matching the reference (generation fixtures store RAW span text; `wrap()` is an oracle-harness concern, deliberately absent). Debug profile (the 702 convention — discriminative, not latency stats).

## Tasks

- [x] **T1** Pre-register the readings + ship the print-only experiment example (`examples/ood_fresh_family.rs` + the `[[example]]` row) before the first run.
- [x] **T2** Run it; record per-set means, gap, d, contested split, per-family breakdown, artifacts.
- [x] **T3** Record the verdict: bench doc `.benchmarks/081_ood_fresh_family_convergent_validity.md` + the katgpt-rs 702 addendum paragraph (with this repo's commit hash).

## Verdict (measured 2026-09-09)

**R1 — CONVERGENT VALIDITY DEMONSTRATED** (the first pre-registered reading fired, on both fresh families):

| pairing | mean_ref | mean_ood | gap (ref−ood) | d | arm |
|---|---|---|---|---|---|
| gen6 vs riir-neuron-db (24) | 3.1104 | 3.8712 | **−0.7608** | **−1.026** | R1-arm |
| gen6 vs riir-dapps (24) | 3.1104 | 3.8712 | **−0.7608** | **−1.026** | R1-arm |

The corpus-family reference fires measurably sharper than BOTH provenance-disjoint fresh families — the gap survives composition matching (real vs real), so run #84's anomalous_negative WAS set composition, as attributed. Full record + mechanism + caveats: `.benchmarks/081_ood_fresh_family_convergent_validity.md` (the honest headline inside the verdict: the fresh signal is all-or-nothing — 0/48 fresh carves fire ANY proposer rule, so the demonstrated validity is the coarse discriminative form, and gen6's mining-time lint selection remains a confound this pairing cannot remove).

## Notes

- Promotion is NOT on this axis: `regime_probe` stays opt-in regardless of the verdict — 740/HISTORY require a default-path consumer, which still does not exist (the score_bench consumer is itself feature-gated). This experiment settles convergent validity, not promotion.
- Boundary: zero new deps (syn/proc-macro2 span-locations/latent stack already present); no sibling repo was modified (both carved read-only at clean working-tree HEAD).
