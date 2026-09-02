# Bench 062 — Issue 062 Phase 1: withheld-(role, filler) OOD generalization

**Status:** PREREGISTERED 2026-09-02 — prediction written BEFORE any run. Results appended below, prediction section never edited.

**Issue:** [.issues/062_tpr_binding_corpus_fusion.md](../.issues/062_tpr_binding_corpus_fusion.md) T1 (healer corpus) + T2 (rustc_errors).
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
