# Bench 113 — Plan 225 T1: the heal-seat case-set harvest + the T1.6 feasibility verdict

**Status:** RECORD — 2026-10-11. T1.6 **BLOCKED ON CASE SUPPLY**: the frozen
population carries **3 deciding cases against ≥ 621 required** at the most
optimistic pre-registered separability (p_d = 0.20) — a ~200× shortfall, and
**0 deciding cases under the full three-clause oracle**. Per the plan's own
gate, T2 (mint) and T3 (encoder head) do not run — the block costs **zero GPU**,
which is what the gate exists to buy. The root cause is STRUCTURAL, not
volumetric, and it amends Bench 108's revisit condition (below).

## The rig

| half | what ran |
|---|---|
| Tool | `fixer_regate --arm HARVEST` (new, this landing) — the plan's "NO second harness" law: same bin, same redaction gate, same `score_case` oracle the arm-H bench scores picks with |
| Population | `--source v2edit_all`: the FULL deduped v2edit corpus (7 test chunks + `sft_train` + `sft_test` + indist, deduped by module blake3, first-seen ids) — **1500 unique real mined modules** (the 13-lint labelled slice + the 1092 unlabelled SFT rows, labelled here by scaffold discovery) |
| Pool capture | `RegateHarness::classify_outcome` → `heal_with_pool` (the R6 seam — the same pool the seat reranks), replayed IN ORDER with memory warming via `report_outcome` per served case (production shape) |
| Oracle | T1.4's three clauses: (a) merge ∧ compile ∧ lint-clears (`score_case` + the new `after_compiled` gate), (c) contract ∧ no added allow/expect, (b) per-fn behavioural differential driver on a 40-case pilot |
| Box | m3 (M3 Max, macOS), clippy 0.1.98 (48a229ceae 2026-09-01), AC; box load carried by siblings (load ~15 during the run — irrelevant to a census, all verdicts are deterministic compiler outputs) |
| Wall | 687.6 s (labelling 296 s + replay 371 s), content-addressed scaffold cache shared with every other verifier consumer |

## The census (the committed evidence)

| quantity | value |
|---|---|
| modules harvested | 1500 (1494 with a firing target, 6 without an oracle) |
| pool ≥ 2 (rerank-eligible) | **643** |
| — deciding (some-but-not-all gold a∧c) | **3** |
| — none-gold (rewards abstaining) | 640 |
| — all-gold (unscored structure) | 0 |
| singleton pools | 851 |
| candidates scored | 2400 |
| — merge-failed | **2360 (98.3%)** |
| — merged but compile-failed | 17 |
| — merged, compiled, lint not cleared | 11 |
| — contract-fail | 8 |
| — attr-silence | 0 |
| **GOLD (a∧c) candidates in the whole population** | **4** |
| behavioural pilot (the 3 deciding cases, all fns) | 3 × `unresolved` → **0 full-gold candidates** |

Per-lint table + the lint-disjoint split live in
[`113_plan225_t1_harvest_census/census.json`](113_plan225_t1_harvest_census/census.json)
(+ `split.json`); every lint is corpus-covered, so coverage is not the
bottleneck. The full `cases.jsonl` (1.46 MB, 1500 rows with pool verdicts,
sha256 `ab0c4533335eaad86ba46af5d04cb82865cc6cbca984277fa079b8f46ade8da2`)
regenerates deterministically from the committed tool + the riir-train v2edit
corpus + clippy 0.1.98 and stays uncommitted (machine-local bulk, the census
is the record).

## The T1.6 arithmetic (inputs pinned in the plan, pre-data)

Paired per-case score, δ₀ = +0.05, α = 0.05 two-sided (Holm, per T4.2),
power 0.80:

N(p_d) = (z_{0.025} + z_{0.20})² · (p_d − δ₀²)/δ₀² = 7.849 · (p_d − 0.0025)/0.0025

| p_d | N required | supply (a∧c level) | supply (full oracle) |
|---|---|---|---|
| 0.20 (most optimistic) | **621** | 3 | 0 |
| 0.35 | 1092 | 3 | 0 |
| 0.50 | 1562 | 3 | 0 |

Measured separability proxy p̂_d = deciding/scored = 3/643 = **0.0047**.
**VERDICT: BLOCKED ON CASE SUPPLY** — and δ₀ was not lowered post hoc.

The gate's pre-registered design also killed the degenerate win before it
could exist: with 640 none-gold vs 3 deciding cases, an expert that ALWAYS
abstains would beat A0 on the paired read (640 × abstain=1 vs A0's
pick-non-gold=0) while healing nothing. T1.5's two-stratum score + the
deciding-case supply gate exist for exactly this case; the bench never runs
against a split that could reward it.

## The structural finding (what actually blocks)

**The pool substrate, not the case volume, is the blocker.** The L0–L1
drafter is a RETRIEVER: `draft_fix` emits the corpus entry's `fixed_code`
verbatim (`src/drafter.rs` — template/matvec selection among corpus
candidates), so every pool member is a corpus-shaped snippet — a complete fn
with the CORPUS's own names (`fn f(..)`), not a transformation of the target
module. `apply_edit` (the same merge instrument arm H scores picks with)
cannot place them: 98.3% merge-fail, and of the 40 that merged, 36 fail
compile/lint/contract. The 3 deciding cases are structural coincidences
(modules whose fn mirrors a corpus entry closely enough for fn-span
substitution).

Memory warming does not rescue this: on real modules nothing is ever kept, so
the trajectory store stays corpus-shaped and pools stay corpus-shaped.

**Amended revisit condition** (supersedes Bench 108's "revisit when a real
hosted heal expert exists"): a rerank expert can only matter when the pool
CONTAINS module-applicable candidates. The natural substrate already exists —
the per-lint AST proposers (`domains::clippy_lints::proposers` — the
span-preserving edits that power `--fix`) and the oracle feed's
MachineApplicable suggestions (the P21 lane's verbatim-apply shape). A
`heal_with_pool` whose pool mixes {proposer edit, clippy suggestion, drafter
snippet} would carry real gold content; THEN this harvest re-runs (the tool is
landed and deterministic), the gate re-computes, and the mint decision gets a
population that can actually separate pickers. Until then the `heal` seat
stays empty — **decline is a correct answer** (the L4/603 precedent).

## The oracle hardening that rode in (a live hole, closed at the seam)

The harvest's gold clause (a) exposed that `ClippyVerifier` read a
**compile-failed** probe as `lint_resolved = true`: on `CompileFailed` the
witness is trustworthy, but the error output carries no clippy codes, so "the
target code did not appear" read as "resolved" — and `score_case`'s
`strictly_fewer`/`target_resolved` inherited the same hole (an empty lint list
from a broken compile looked like a findings reduction). Both are closed:

- `ClippyVerifyResult.compiled` (new field) + `lint_resolved = false` on
  compile failure — the documented contract, now delivered;
- `CaseScore.after_compiled` (new field) gating `strictly_fewer` and
  `target_resolved`;
- pinned by `a_type_error_is_a_refusal_not_a_resolution` (real cargo run).

Every consumer can only refuse MORE (the tuning loop's Stage-2 admission, the
P21 apply lane, the regate keep filters) — never admit a broken fix. In this
census 17 candidates were refused by the new gate that would previously have
counted as findings-reductions.

## Artifacts

- `fixer_regate --arm HARVEST --source v2edit_all --out <dir>` — rerun for the
  full row set (deterministic; ~11 min on the M3).
- [`113_plan225_t1_harvest_census/census.json`](113_plan225_t1_harvest_census/census.json)
  · [`split.json`](113_plan225_t1_harvest_census/split.json) (lint-disjoint:
  340 test / 303 train scored cases — leakage-safe by construction, the
  v2edit rows carry no source-repo/time provenance for a row-adjacent split).
- New substrate: `ClippyVerifier::discover_lints` (the labelling pass),
  `cf_workers_ai::load_v2edit_full`, `cf_workers_ai::attr_silence_added`
  (clause (c)) + tests.
