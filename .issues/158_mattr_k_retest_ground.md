# Issue 158: Matryoshka-k re-test ground (the 133 B2 follow-up)

> **Status:** OPEN — filed 2026-10-08 (the optional B2 verdict from Issue 133's closure; no work owed until a trigger fires)
**Date:** 2026-10-08
**Supersedes-carry:** Issue 133 (closed NEGATIVE 2026-10-08, removed per the noise rule — `.benchmarks/109_mattr_k_t1_negative.md` is the record; closure commit `1e447f53`)

---

## Why this exists (the honest boundary of 133's negative)

Issue 133's T1 measured the Matryoshka-k (nested-width corpus training) study NEGATIVE on the current corpus, but the negative has a **named boundary**, not a law:

- The shipped FIXED objective wins shallow-k everywhere (rig 134 vs ≤130).
- Deep-k was **corpus-limited**: 1.16 labels/span mean support — the deep-k arms never saw the label diversity the paper's regime assumes.
- The withheld-width **nesting effect is REAL** (29.3 → 49.6 @ k=20 vs 92.0% full) — the mechanism exists; it trailed because the corpus could not feed it.

A negative on starved evidence is not a negative on the mechanism. This issue pins the **re-test triggers** so nobody re-runs blind (and nobody quotes 133's negative outside its boundary).

## Re-test triggers (ANY one fires the re-test)

- [ ] **T-A — corpus depth:** a clippy-corpus generation whose mined spans carry mean label-support ≥ 3 labels/span at the deep-k widths (the paper's regime; today 1.16).
- [ ] **T-B — rule-space growth:** the corpus doubling in rules-per-domain such that per-domain shallow-k retrieval saturates (the rig's shallow ceiling measurably binding — the signal the nested objective exists to break).
- [ ] **T-C — a consumer that needs the nesting:** a downstream lane (reflex seat, retrieval fan-out) that actually CONSUMES k-subvectors at serve time — without one, the nesting effect is a benchmark artifact by construction (the 133 G1/G2-OOD verdict's own logic).

## If a trigger fires

- [ ] Re-run the T1 harness (landed at `007435e5`, `.benchmarks/109` is the protocol) on the new corpus; the G1 (quality) + G2-OOD gates bind as before.
- [ ] The control is the shipped fixed objective AT THE SAME corpus generation — never the old 134 number across corpus boundaries.
- [ ] Record the verdict in `.benchmarks/` under a NEW number; this file only carries the trigger + the pointer.

## Non-goals

- No standing work. No re-tuning of the 133 arms. This file is a tripwire, not a plan — if none of T-A/T-B/T-C fires, the correct action on review is NONE.
