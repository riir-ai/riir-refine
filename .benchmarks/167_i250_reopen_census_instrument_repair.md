# Bench 167 — i250 re-open census on the grown pool + the decline-guard cost-model repair

**Date:** 2026-11-25 (amo)
**Lane:** riir-refine Issue 250 T1/T2 — the pre-registered re-open of the 2026-11-24 escalation stop
**Verdict session:** delegated Claude verdict, 1 round **AGREE** (session `5ace8275-fe1f-4426-8c5e-6477e129f424`), five landing conditions all landed in this record's commit
**Box state:** amo, AC, no heavy local jobs (the bmo i1241 sweep and the fleet queue were the only consumers); CPU census work — no latency claims

## 1. Why this census ran

The 2026-11-24 T2 cell fired the pre-registered escalation STOP (v1 head ≈ similarity floor within noise ⇒ modelless wins, T3 GPU head does not open) and named its re-open bar: **FIT-side reverts ≥ ~5, then re-run BOTH arms on the larger pool before any close-out verdict.** Four real train fix runs landed 2026-11-25 (the pool grew 11 runs / 359 rows → **15 runs / 695 rows**), pushing the run-timestamp median so the deployer 1-revert run and the train 260-edit/7-revert run crossed to FIT:

- fit runs 7 (310 rows, **8 reverts** ≥ the ~5 bar) · eval runs 8 (385 rows, 23 reverts)
- eval-side revert composition: ndb 9 + ai 14 + deployer 0 + chain 0 + train 0 — **all 23 eval reverts live in 2 revert-dense runs**

## 2. The instrument repairs (2 defects, both found by the same tell class)

The re-run first exposed an instrument defect in `examples/edit_revert_head.rs::realized_reduction`:

- **Missing cost arm:** the pre-registered T1a semantics ("remaining edits all survive → 2 checks; nothing left → 0; **reverts still outstanding → the actual checks**") was never implemented — `sim` was only ever 0 or 2, keyed on remaining KEPT rows. Every run with outstanding un-declined reverts simulated to 2 checks. **Tell: red% identical at every τ including decline-nothing** (79.2% at τ=0.05 with 3 declines AND at τ=0.99 with 0 declines — a decline-driven column cannot be constant in the declines).
- **Sibling artifact in `examples/similarity_floor_census.rs`:** already-clean eval runs (no reverts; actual > canonical 2 from multi-package spend) were charged the canonical 2, inventing savings no decline caused (GLOBAL τ=0.05 red% 10.4% → 2.6% after the fix). Both instruments now charge: all-declined → 0 · outstanding revert → actual · all-reverts-declined mixed run → 2 · already-clean run → its actual.
- **PR-AUC convention unified** (both `pr_auc` functions): the first PR point's base (`recall₀ × precision₀`) now counts — a fully-tied pool reads 0.25 (its base rate), never 0.0. The head's 0.236 did not move (its first threshold group already carried precision 1.0, so the old anchor credited the same mass).
- **No recorded verdict ever relied on the broken columns:** every prior census verdict (T1 floor 11-24 series, T2 first cell 11-24) keyed on false-decline rates — unaffected by both repairs; Bench 162 T1a predates the census examples. Disclosed per the standing rule: future red% numbers move.
- **Self-tests extended to pin the repaired arms** (verdict condition 1): the floor census's `--self-test` gains a dedicated fixture (clean run actual-4 charged 4; a near-copy revert never declined charged its actual-3; ceiling 1/7 pinned; PR-AUC 0.25 pinned); the head example gains a `--self-test` arm (clean-at-actual · outstanding-revert-at-actual · whole-batch 7/9 · all-revert 1.0 · ceiling 98/108 ex-0). Both PASS.

## 3. The census (fixed instruments, both arms)

**T1 similarity floor: FAIL both variants.**
| variant | PR-AUC (EVAL) | min false-decline | best τ |
|---|---|---|---|
| GLOBAL | 0.163 (323 scored) | 4.97% > 1% bar | none adjudicable |
| RULE | 0.186 (293 scored) | 1.10% > 1% bar (near-miss: τ 0.05 corr 3 / false 4) | none adjudicable |

**T2 v1 head: FAIL G3 — and now provably so.**
- head PR-AUC 0.236 (up from 0.094 — real signal growth as fit-side reverts went 1 → 8)
- false-decline plateau: 3 declines, 3 correct, 0 false at τ 0.05–0.30 (clean)
- **realized reduction 0.0% at every τ** — the 3 caught reverts sit inside the ndb/ai revert-dense runs that still carry outstanding reverts, so no run's cost arm flips under the batch semantics. Declines buy zero batch savings on this pool.

**Perfect-catch ceiling (verdict condition 2, measured not inferred):**
- all-runs 71.4% · **outlier-excluded 20.0%** (1 outlier run: the ai 313-edit run)
- **20.0% ≥ the 10% G3 bar ⇒ this pool CAN adjudicate G3** — the head's 0.0% is a genuine **FAIL**, not UNADJUDICABLE. The ceiling requires whole-batch catches (catch ALL of a run's reverts); the pool's revert concentration (23/23 in 2 runs) is what makes that hard, not impossible.

**The Floor bar's both conjuncts (verdict condition 3):**
- PR-AUC conjunct: head 0.236 − floor 0.163 = **+0.073 (GLOBAL)** / 0.236 − 0.186 = **+0.050 (RULE)** — the head **clears** the +0.05 separation on both variants.
- G3 conjunct: **0.0% < 10% — FAILS.**
- The bar is a conjunction ("beat the similarity score by +0.05 PR-AUC AND deliver G3, else … the GPU budget stays unspent") ⇒ **the GPU budget stays unspent.**

## 4. Close-out verdict

Delegated verdict **AGREE** (1 round, conditions landed):

1. **The escalation STOP CONFIRMS on the re-run.** T3 (the 20–30M GPU head, 10–25 GPU-h on the 4090) **stays CLOSED**; T4 (the decline-guard posture) **stays CLOSED** — its precondition (shadow-measured ≥ 10% + false ≤ 1%) is unmet by both arms.
2. The instrument repairs are faithful to the pre-registered T1a text, not goalpost-moving; the goalposts' own tell (τ-independent red%) is recorded as the detection instrument.
3. The head's PR-AUC growth is recorded **as signal, not as a near-pass** — data-limited learning is real, value is not delivered.
4. **No batch-semantics change to rescue the head** (verdict condition 5): any per-revert / partial-batch credit posture needs its own pre-registered bar and must not be swapped into Issue 250 retroactively.
5. **The new re-open condition, numbered now (verdict condition 4): ≥ 5 eval-side runs with ≤ 2 reverts each, then re-run both arms under the unchanged batch semantics.** Pool-composition blocker stated: the eval side carries ZERO low-revert runs today (the deployer 1-revert run sits on the FIT side) — batch-granular value is structurally unobservable on this composition, whatever the scorer's quality. Until the condition fires, the lane rests with the negative recorded.

## 5. Receipts

- Instruments: `examples/similarity_floor_census.rs` (+PR-AUC, +ceiling, +repaired-arms fixture) · `examples/edit_revert_head.rs` (cost-model repair, +ceiling, +`--self-test`) — report-only census examples; no product code touched; bench-guard not implicated.
- Self-tests: both PASS (pinned arithmetic incl. the repaired branches).
- Census invocation: both examples over the 7 accumulated logs (refine, reflex, train, chain, neuron-db, ai, deployer).
- Session: amo-i250-census@amo, 1796192466 (lease folded at landing).
