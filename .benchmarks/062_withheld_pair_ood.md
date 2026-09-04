# Bench 062 — Issue 062 Phase 1: withheld-(role, filler) OOD generalization

**Status:** PREREGISTERED 2026-09-02 — prediction written BEFORE any run. Results appended below, prediction section never edited.

**Issue:** `.issues/062_tpr_binding_corpus_fusion.md` T1 (healer corpus) + T2 (rustc_errors).
**Research:** katgpt-rs `.research/527_TPR_Emergent_Symbolic_Structure_Binding_Algebra.md` (arXiv:2608.29530, McCoy/Soulos/Linzen/Smolensky 2026).
**Toolchain:** rustc 1.97.0 (2d8144b78 2026-07-07) / clippy 0.1.97, macOS aarch64 (M3 Max).

---

## 1. The unmeasured axis

Every retrieval floor this repo ships (the 14/14 gate, Bench 018/022/027, the
score-bench) is **in-distribution by construction**: the query is a variant of
a corpus entry whose (rule, shape) pair is present in the index. The paper's
central prediction is that a system trained/indexed on (role, filler) pairs
defaults to **atomic memorization** — it stores the conjunction, not the
factors — and therefore fails on *unseen combinations of seen roles and seen
fillers*. Nothing in this repo has ever measured that.

- **role** = the rule identity (clippy lint name / rustc error code).
- **filler** = the code shape the rule is embedded in (a **shape-class**,
  derived deterministically from the span text — see §3).

## 2. PREREGISTERED PREDICTIONS (frozen — written before any measurement)

Let `ID` = in-distribution top-1, `OOD` = withheld-pair top-1, on the SAME
query items, paired (same query, two indexes differing only by the withheld
cell).

| id | prediction | bar |
|---|---|---|
| **P1** | Pure-retrieval arm (`RerankMode::Off`) drops on withheld pairs | `ID − OOD ≥ 20.0 pp` |
| **P2** | Pure-retrieval OOD sits near the atomic floor | `OOD ≤ atomic-null ID top-1 + 15.0 pp` |
| **P3** | Atomic null on withheld pairs is exactly zero (by construction) | `= 0.0 %` — asserted, not measured |
| **P4** | Shipped arm (`RerankMode::Structural`) closes most of the gap, because `mark_instantiation` probes the SPAN and is index-independent | `ID − OOD ≤ 10.0 pp` |
| **P5** | Same directional result on `rustc_errors` (T2) | `ID − OOD ≥ 20.0 pp` in the retrieval arm |

### Verdict rule (also preregistered)

Applied to the **pure-retrieval arm** paired drop `Δ = ID − OOD`:

- `Δ ≥ 20.0 pp` → **Phase 2 (TPR structured retrieval) JUSTIFIED by our own measurement.**
- `Δ ≤ 5.0 pp` → **Phase 2 HONESTLY DEMOTED** (OOD at ceiling; the axis has no gap to close).
- `5.0 < Δ < 20.0 pp` → **PARTIAL** — recorded as-is; Phase 2 stays opt-in with the measured gap as its target, no promotion claim.

Additionally: if the **atomic null scores above 0 on withheld pairs**, the run
is declared a HARNESS BUG (withholding leak), not a finding, and no verdict is
issued until fixed.

## 3. Design (also written before running)

### 3.1 Shape-class classifier — deterministic, first-match-wins

The filler must be **orthogonal to the lint-identifying construct**, otherwise
role and filler are collinear and no unseen combination can exist. So the
class is derived from *containing context* features, not from the anti-pattern
itself. Priority order (most specific first, first match wins):

| # | class | predicate over the span text |
|---|---|---|
| 1 | `Macro` | invokes a macro (`format!`, `println!`, `write!`, `vec!`, `assert*!`, `panic!`) |
| 2 | `IteratorChain` | `.iter()` / `.into_iter()` / `.map(` / `.filter(` / `.collect(` / `.sort_by` |
| 3 | `Closure` | a closure header (`\|x\|` / `\|\|` / `move \|`) |
| 4 | `Generic` | generic or lifetime parameter list (`<'`, `<T`, `impl Fn`) |
| 5 | `ImplBlock` | `impl ` — the rule sits in a method, not a free fn |
| 6 | `Loop` | `for ` … ` in ` / `while ` |
| 7 | `MultiItem` | ≥2 top-level items (`fn `/`struct `/`enum `/`const ` count ≥ 2) |
| 8 | `PlainFn` | none of the above — a single free fn body |

The classifier is pinned by a unit test (fixed inputs → fixed classes) and by
a determinism assert (same text → same class across calls).

### 3.2 Population (the binding matrix)

Index population `P` for T1 = single-label bindings with DISTINCT texts:

- **C** — 45 `clippy_lints` corpus entries (`(lint, buggy_code)`).
- **F** — 49 `eval_fixtures_clippy::CLIPPY_SPANS` (hand-authored, real-clippy-verified).
- **R** — the 20 oracle-labeled REAL-repo spans from `.benchmarks/data/049_t01/spans.jsonl` (Plan 049 T0.1) — genuine production shapes, the strongest filler diversity available.

`eval_fixtures_composite::COMPOSITE_SPANS` (28) are **deliberately excluded
from the INDEX**: they are multi-label, and `RuleIndex::build_with`
deduplicates identical chunk content by `zone_hash` (documented substrate
contract), so the same text cannot be indexed under two rule ids — the second
binding would silently vanish. They are reported as an auxiliary query set
instead.

### 3.3 Withholding — from the INDEX, and eligibility

Cell `c = (rule R, shape S)`. A query item `q ∈ c` is **eligible** iff:

1. `|c| ≥ 2` — otherwise the ID arm (`P \ {q}`) and the OOD arm (`P \ c`) are the same index and there is no contrast to measure.
2. **role survives**: some item in `P \ c` has rule `R` (a different shape).
3. **filler survives**: some item in `P \ c` has shape `S` (a different rule).

Conditions 2+3 are exactly the paper's "unseen combination of a *seen* role
and a *seen* filler". Rules failing them are **role/filler-collinear** and are
reported as such — that is itself a finding about the corpus.

- **ID arm** index = `P \ {q}` (leave-one-out; the pair is still represented).
- **OOD arm** index = `P \ c` (leave-cell-out; the pair is absent).
- **Asserted**, not assumed: no item of the OOD index carries rule `R` with shape `S`.

Both arms are measured **in the same run, per query**, and reported as paired
deltas — never `median(A)` vs `median(B)`.

### 3.4 Atomic null

A pure memorizer with no factorization: `BLAKE3(shape_class ‖ 0x00 ‖ key)` →
set of rules, built from the same index population as the arm it scores.

- `sig` key — the normalized token stream (identifiers → `I`, literals → `L`, keywords + maximal punctuation runs kept, order preserved).
- `bag` key — the *sorted multiset* of the same tokens (order-insensitive; strictly coarser, so a strictly stronger null).

Both are **shape-class-prefixed**, so signature equality implies shape-class
equality. Therefore no item of the OOD index can produce rule `R` for a query
in cell `(R, S)`: every remaining `R`-item has shape `≠ S`, hence a different
prefix. P3 is a *construction*, and the harness asserts it.

### 3.5 Instrument verification (mandatory, per the workspace failure ledger)

- **Canary**: re-inject the withheld cell into the OOD index and assert the aggregate score MOVES. A withholding that changes nothing is a dead instrument.
- **Non-uniformity**: assert the per-query score vector is not constant.
- **Nonzero-count**: assert the eligible-query count is > 0 and print it (a cfg-gated harness that compiles to nothing "passes").

### 3.6 T2 — rustc_errors

Same protocol, deterministic, no cargo/oracle needed:

- role = `expect_code` (E0597/E0499/E0502/E0505/E0596/E0382/E0507/E0373).
- population = `corpus_gen::generate_corpus()` (200 stratified cases) + `floor::MUTANT_CASES` (12 anchors); item text = the BROKEN source.
- filler = shape-class from a rustc-specific classifier (documented in §3.6 of the results section) cross-tabbed against `MutatorKind`.
- measures whether **repair-playbook selection** (which is keyed by error code) can be driven by pattern on an unseen (code, pattern) combination.

---

# RESULTS (measured 2026-09-02 — appended, §2 never edited)

Harness: `tests/t062_withheld_pair_ood.rs` + `src/draft/withheld_pair_ood.rs`.

```bash
CARGO_TARGET_DIR=/tmp/riir_clippy_062 cargo test --release \
  --features latent_retrieval --test t062_withheld_pair_ood -- --nocapture --test-threads=1
```

## 4. Instrument verification — done FIRST, before reading any number

| check | result |
|---|---|
| eligible-query count > 0 (a cfg-gated harness compiles to nothing) | T1 **n=25**, T2 **n=43** — asserted |
| withholding assertion (no `(R,S)` item survives in the OOD index) | asserted per cell, plus a count identity `|OOD| + |cell| = |P|` |
| **NOROLE canary** (role removed → must be 0%) | **0.0% / 0.0% / 0.0%** at top-1/3/5 on every arm of both tasks |
| per-item score vector non-uniform | asserted (T1 ID scores span 17.1 – 1042.2) |
| atomic null on withheld pairs == 0 (leak detector) | **0.0%** on both keys, both tasks |
| run-to-run determinism | 12 independent processes — see §7 |

**One classifier defect was found and fixed by this check, not by inspection.**
The first T2 run reported `index-expr` absorbing four of eight error codes.
Cause: every generated case appends a `#[cfg(test)] mod ground_truth` harness
containing `#[test]`, and the index-expression predicate read `[test]` as an
index. The filler axis was an artifact of the generator's scaffolding. Fixed by
[`rustc_payload`] (classify the pre-`#[cfg(test)]` payload) + `strip_attributes`,
pinned by `attributes_do_not_masquerade_as_index_expressions`. All T2 numbers
below are post-fix; the pre-fix matrix is not reported as a result.

## 5. T1 — healer corpus (clippy_lints)

Population **113** distinct-text bindings = 45 corpus + 48 single-lint
fixtures + 20 oracle-labeled real-repo spans. **45 roles × 8 fillers → 62
occupied cells, 10 ELIGIBLE, n=25 queries.**

**32 of 45 roles (71%) are role/filler-COLLINEAR** — they occur under exactly
one shape-class, so no unseen combination of that role can exist at all. This
is a first-order finding about the corpus and it caps the protocol's reach: the
healer corpus is very close to a **one-filler-per-role table**.

| arm | top-1 | top-3 | top-5 |
|---|---:|---:|---:|
| ID — pair present (`P \ {q}`) | 52.0% | 76.0% | 84.0% |
| **OOD — pair withheld (`P \ cell`)** | **4.0%** | 16.0% | 28.0% |
| NOROLE canary (must be 0) | 0.0% | 0.0% | 0.0% |
| CHANCE (uniform over 45 roles) | 2.2% | 6.7% | 11.1% |
| atomic null `sig` / `bag`, ID | 0.0% / 0.0% | — | — |
| atomic null `sig` / `bag`, OOD | 0.0% / 0.0% | — | — |

**PAIRED DROP: +48.0 pp top-1** (paired rank moves ID→OOD: 0 better, 3 same,
**22 worse** of 25). Pure-retrieval OOD top-1 (4.0% = 1/25) is **at the chance
floor** (2.2%).

Shipped arm (`RerankMode::Structural`, proposer-visitor evidence — probes the
SPAN, so it is index-independent):

| arm | top-1 | top-3 | top-5 |
|---|---:|---:|---:|
| ID | 92.0% | 96.0% | 96.0% |
| **OOD** | **52.0%** | 52.0% | 56.0% |

**PAIRED DROP: +40.0 pp top-1** (0 better, 13 same, 12 worse).

### OOD by the role's distinct-filler count — the mechanism

| role fillers | n | ID top-1 | OOD top-1 (retrieval) | OOD top-1 (Structural) |
|---:|---:|---:|---:|---:|
| 2 | 11 | 36.4% | 9.1% | 63.6% |
| 3 | 11 | 81.8% | 0.0% | 45.5% |
| 4 | 3 | 0.0% | 0.0% | 33.3% |

### Per-cell (retrieval arm)

| role | filler | n | ID | OOD | Δ |
|---|---|---:|---:|---:|---:|
| `single_match_else` | impl-block | 2 | 100.0% | 0.0% | +100.0 |
| `single_match_else` | multi-item | 2 | 100.0% | 0.0% | +100.0 |
| `unnecessary_sort_by` | iter-chain | 2 | 100.0% | 0.0% | +100.0 |
| `match_bool` | macro | 5 | 80.0% | 0.0% | +80.0 |
| `match_bool` | plain-fn | 2 | 50.0% | 0.0% | +50.0 |
| `map_unwrap_or` | iter-chain | 3 | 33.3% | 0.0% | +33.3 |
| `single_match` | multi-item | 2 | 50.0% | 50.0% | +0.0 |
| `collapsible_if` | multi-item | 2 | 0.0% | 0.0% | +0.0 |
| `needless_return` | plain-fn | 2 | 0.0% | 0.0% | +0.0 |
| `semicolon_if_nothing_returned` | macro | 3 | 0.0% | 0.0% | +0.0 |

## 6. T2 — rustc_errors (error code × borrow pattern)

Population **212** = 200 generated stratified cases + 12 committed anchors.
**8 roles × 10 fillers → 14 occupied cells, 4 ELIGIBLE, n=43 queries.**
**6 of 8 codes are collinear** (E0373, E0382, E0499, E0502, E0505, E0507 each
ship exactly one pattern family); only E0596 (2 fillers) and E0597 (6 fillers)
admit an unseen combination.

| arm | top-1 | top-3 | top-5 |
|---|---:|---:|---:|
| ID — pair present | 97.7% | 97.7% | 100.0% |
| **OOD — pair withheld** | **48.8%** | 51.2% | 67.4% |
| NOROLE canary (must be 0) | 0.0% | 0.0% | 0.0% |
| CHANCE (uniform over 8 roles) | 12.5% | 37.5% | 62.5% |
| **atomic null `sig` / `bag`, ID** | **79.1% / 79.1%** | — | — |
| atomic null `sig` / `bag`, OOD | 0.0% / 0.0% | — | — |

**PAIRED DROP: +48.8 pp top-1** (0 better, 21 same, 22 worse of 43).

### OOD by the role's distinct-filler count — a step function

| role fillers | n | ID top-1 | OOD top-1 |
|---:|---:|---:|---:|
| 2 (E0596) | 22 | 95.5% | **0.0%** |
| 6 (E0597) | 21 | 100.0% | **100.0%** |

| role | filler | n | ID | OOD | Δ |
|---|---|---:|---:|---:|---:|
| E0596 | string-owner | 5 | 100.0% | 0.0% | +100.0 |
| E0596 | vec-owner | 17 | 94.1% | 0.0% | +94.1 |
| E0597 | index-expr | 10 | 100.0% | 100.0% | +0.0 |
| E0597 | string-owner | 11 | 100.0% | 100.0% | +0.0 |

**The per-row ranks say WHICH failure this is.** Every E0596 OOD row keeps the
true code in the pool at rank **2–7** — the right answer is present and
demoted, not missing. That is a *re-ranking* shape, not a *retrieval* shape,
which is exactly the class Plan 005 T3.5.4 already identified as
"different-evidence" territory and exactly what Phase 2 T4's role-conditioned
match claims to address.

## 7. Determinism — 12 independent processes

| quantity | result |
|---|---|
| T1 pure-retrieval OOD top-1/3/5 | **4.0 / 16.0 / 28.0 — identical 12/12** |
| T1 pure-retrieval ID, all cells, matrix, eligibility | identical 12/12 |
| T2 OOD top-1 / top-3 | **48.8 / 51.2 — identical 12/12** |
| T1 **shipped (Structural)** OOD top-1 | **52.0% in 9/12, 56.0% in 3/12** |
| T2 top-5 paired drop | +32.6 pp in 11/12, +30.2 pp in 1/12 |

The Structural flip is localized to **exactly one row** by the per-row dump:
`(needless_return, plain-fn)` fixture query, OOD arm — `needless_return` is
either absent from the `RERANK_POOL = 12` pool (`rank = -`) or at rank 0. The
ID score is bit-identical across runs, so the ID arm is not involved. Since
`corpus_index::rank_into` already applies a canonical `total_cmp` + `rule_id`
tie-break, the flip is **upstream of it** — in the candidate set
`NeuronDbRetriever::retrieve` returns — i.e. substrate-level, per-process
(`RandomState`), not a defect of this harness. It also means the shipped
`--suggest` pool can silently omit a boundary rule depending on the process;
recorded as a follow-up on Issue 062, not fixed here (riir-rag is a sibling).

The headline verdict rests on the pure-retrieval arm, which is bit-stable.

## 8. Verdict against the preregistered bars

| id | bar | measured | outcome |
|---|---|---|---|
| **P1** | retrieval `Δ ≥ 20.0 pp` | T1 **+48.0 pp** | **HELD** |
| **P2** | retrieval OOD ≤ null_ID(bag) + 15 pp | 4.0% ≤ 0.0% + 15 | **HELD** (but see the caveat) |
| **P3** | atomic null OOD == 0 | 0.0% both keys, both tasks | **HELD** (asserted, by construction) |
| **P4** | Structural `Δ ≤ 10.0 pp` | **+40.0 pp** | **REFUTED** |
| **P5** | T2 retrieval `Δ ≥ 20.0 pp` | **+48.8 pp** | **HELD** |

### VERDICT: **Phase 2's TPR axis is JUSTIFIED by our own measurement** — T1 Δ = +48.0 pp, T2 Δ = +48.8 pp, both ≥ the preregistered 20.0 pp bar, on the pure-retrieval arm, with the atomic null at 0 and the NOROLE canary at 0.

This is NOT the "OOD at ceiling → honestly demote" outcome. The paper's
prediction that the default failure mode is atomic-memorization-shaped **held
directionally**, and on T1 it held at full strength: pure-retrieval OOD top-1
(4.0%) is indistinguishable from chance (2.2%).

## 9. Three honest caveats that qualify the verdict

1. **P2 passed vacuously on T1.** The atomic null scores **0.0% on the ID arm
   too**: no two bindings in the healer corpus share a normalized token
   structure, so the strict memorizer cannot even fit its own training set.
   The consequence matters — T1's OOD failure is therefore **not** "the system
   memorized conjunctions". It is that the fused BM25+latent similarity is
   **filler-dominated**: it retrieves by code shape, and the role axis is not
   separable in the raw signal. That is R527's finding-7 framing rather than
   its finding-1 framing, and it is the same defect the CHANCE floor (not the
   null) is needed to establish. **T2 is where the null is informative** —
   79.1% ID / 0.0% OOD — and there the memorization reading is direct.
2. **n is small and the corpus is nearly collinear.** T1 n=25 over 10 cells;
   T2 n=43 over 4 cells; 71% of clippy roles and 75% of error codes admit no
   unseen combination at all. The verdict is a verdict about the eligible
   subset. Widening it requires **more fillers per role**, which is a corpus
   task (the 20 real-repo spans did all the work here — they are the only
   source of genuine shape diversity), not a retrieval task.
3. **The gap is not uniform — it is a step function in the role's filler
   count.** E0597 (6 fillers) generalizes **perfectly** (OOD 100.0%); E0596
   (2 fillers) collapses to **0.0%**. Same shape on T1. A role seen with one
   remaining filler has nothing to compose from, so part of the measured Δ is
   *structurally* unclosable by any mechanism — Phase 2 must not claim it.
   The closable part is the E0596-shaped one: right answer retrieved, demoted
   to rank 2–7.

## 10. What this hands Phase 2 (T3–T6) — a preregistered target, not a hope

- **The OOD baseline to beat**: T1 retrieval 4.0% / Structural 52.0% top-1;
  T2 retrieval 48.8% top-1, 51.2% top-3.
- **Where the headroom actually is**: roles with ≥2 remaining fillers whose
  true rule is retrieved but demoted (all 22 E0596 rows; the `match_bool` and
  `map_unwrap_or` T1 cells). Roles with one remaining filler are out of scope
  and any gain claimed there should be treated as leakage.
- **T5's per-domain BoW verdict has a prior now**: `rustc_errors` roles are
  overwhelmingly single-filler (6/8), so the m=1 shared-role ALS should show
  little residual gain there; `clippy_lints` has 13 multi-filler roles.
- **Floors are untouched**: this bench adds no default-feature code path. The
  module is behind `latent_retrieval` and no existing eval was modified.

## 11. Unrelated pre-existing red gate (recorded so it is not attributed here)

`tests/latent_suggest_goat.rs::g2_latency_under_1ms_per_chunk` fails on this
box at **1189–1203 µs/chunk** against its 1000 µs bar. Verified pre-existing by
running it from a worktree pinned at this benchmark's preregistration commit
(`ce9d395`, before any code landed): **1193 µs** — the same figure. Bench 062
adds no code to any retrieval path, so this is a machine-speed gate, not a
regression from this work.
