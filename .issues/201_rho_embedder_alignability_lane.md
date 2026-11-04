# Issue 201: ρ alignability read per embedder over format_pairs — the Issue 978 T5 consumer lane (riir-refine)

**Status:** Open — consumer-measurement lane authorized by owner verdict 2026-11-04 (Claude AGREE, katgpt-rs Issue 978 T5 GO; filed riir-refine-first per the verdict's ordering). **The savings arm below is PRE-REGISTERED — it is pinned BEFORE any measurement runs.** T4 promotion of the upstream primitive is decided only after this lane's result AND the riir-reflex lane's result are recorded under katgpt-rs Issue 978 with their commit hashes.
**Claimed:** zed-b860ef72@amo, 1793760320, expires 2026-11-06T02:45:20Z
**Date:** 2026-11-04
**Upstream primitive:** katgpt-rs `katgpt-core::nuisance_fraction` (opt-in feature `nuisance_fraction`, landed `e572a2281` 2026-11-03) — ρ = tr(C_B^{1/2})/k over grouped multi-view embeddings + `alignment_floor` = 2k(1 − min ρ) (arXiv:2610.39302 Thm 3, Balestriero & LeCun). Concurrent art to cite in any output doc: Schnaus et al. arXiv:2610.09411 (cross-set predictability) — this lane is adoption engineering, not a novelty claim.
**Boundary:** scoped contract run 2026-11-04 GREEN (`ci_boundary_contract.sh --repo riir-refine`, exit 0, 19 cross-repo dep edges measured). The lane adds **no new dep edge**: `katgpt-core` is already the default build's only public dep; the lane turns on the upstream opt-in feature via a forward in this repo's own manifest (`nuisance_fraction = ["katgpt-core/nuisance_fraction"]`), never a new crate or sibling edge. Dual-allocation gate run at allocation time: 0 twin / 0 independent / 0 counter.

## The lane

ρ per embedder over [`assets/format_pairs_v1.jsonl`](../assets/format_pairs_v1.jsonl) (4,213 rows) grouped by rule family — the rule-explained variance fraction of each candidate embedding space — ranking embedder candidates beside the FLINT architecture axis (Research 309). Groups = rule families (the pairs' rule identity is the shared world state z; the two formats of one pair are the views). Refusal law inherited from the primitive: a family with < 2 members or < 2 families refuses loud, never reads 0.

Implementation consumes the primitive, never forks it: report-vehicle per this repo's report-only lane discipline (`--rho-report` arm on an existing measurement bin or a new example, feature-gated, suggest-only class — no exit-code coupling to any fixer lane). The G≪k regime caveat rides along: ρ is monotone-informative WITHIN one embedding dimension, not calibrated ACROSS embedders of different k — cross-embedder ranking normalizes by k or reports per-k, pinned at implementation.

## Pre-registered savings arm (the promotion-evidence bar — pinned BEFORE measuring)

The upstream T4 promotion bar (katgpt-rs Issue 978): "the gate is real when a consumer shows refusing a doomed transport/rotation SAVES measurable compute or prevents a wrong verdict." **A ρ ranking that only correlates with something does not count.** This lane's arm:

- **The replayed decision:** Issue 162 / [Bench 115](../.benchmarks/115_issue162_format_flip_t2.md) — the formatting-churn certificate measured NO transferable formatting subspace and the projected-KNN lane was NOT ARMED. That refusal cost the full Bench-115 harness run.
- **The claim under test:** the ρ read over the SAME grouped pairs (same fixture, same grouping) predicts that refusal BEFORE the harness runs — low rule-family ρ ⇒ no transferable formatting subspace ⇒ do not arm. Ground truth already exists (Bench 115's verdict), so the replay is a genuine savings measurement, not a correlation: the ρ read must reproduce the NOT-ARMED verdict and the ledger records wall-clock(ρ read) vs wall-clock(Bench-115 harness) — box state beside both per the latency-claim law (this repo's `bench_preflight` class provenance; free RAM, load class, power source).
- **Pass:** ρ reproduces the refusal AND the saved compute is measurable (the harness cost avoided exceeds the ρ read's cost by the pre-registered margin — wall-clock or calls skipped, both recorded). **Fail:** ρ disagrees with the Bench-115 verdict, or the ρ read costs as much as the harness it was supposed to replace → the primitive stays opt-in upstream, the negative is recorded under katgpt-rs Issue 978 with this issue's commit hash, and the lane closes honest.
- **Wrong-verdict half (optional second arm):** if any candidate embedder shows HIGH rule-family ρ where Bench 115's fixture-level verdict said no-transfer, that is a "prevents a wrong verdict" demonstration instead — recorded with the same rigor.

## Discipline

- Claim-lease before implementing (`py ../katgpt-rs/scripts/claim_lease_check.py --item 201` after appending the Claimed line; 48 h expiry, the push is the arbiter).
- Numbering: allocated as 201 from `.issues/.highwater` (200 → 201, written back same commit).
- Results land under katgpt-rs Issue 978 (cross-repo citation WITH the repo name) + this issue's Status; T4 promotion is decided there, not here.
