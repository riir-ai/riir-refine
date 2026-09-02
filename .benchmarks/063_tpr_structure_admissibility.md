# Bench 063: TPR structure fit on the healer corpus — admissibility first (Issue 062 T3)

**Status:** RUN 2026-09-02, addendum RUN same day, §12.6 RUN same day on the
upstream covariate (katgpt-rs `4af2b3cf`) — the consumer-local predicate is
deleted and the reported MEAN weakens §11.3's margin without overturning it.
Verdict
**T4 BLOCKED-ON-STATE-CONTENT** (§6 branch 5 — a branch preregistration did not
anticipate; §11.6 amends §8's branch-2 verdict to it, same consequence,
stronger evidence).
Preregistration commit `c6d0452` — predictions P1-P6, the scheme decision and
the verdict rule were committed before the harness existed. **P2 was refuted by
its own bound and P1 was confirmed by a different bound than the one it
argued**; both are recorded as stated rather than retro-fitted. Harness
`src/draft/tpr_fit.rs` + `tests/t063_tpr_structure.rs`, feature
`tpr_structure`. Bit-identical over 5 processes.

## 1. Why this bench exists before T4

Issue 062 Phase 2 T4 proposes to score retrieval candidates against a **TPR
projection** of the span state instead of the raw embedding. That proposal has
a precondition nobody measured: **can a TPR artifact be fitted to this corpus
at all, in the state space the retrieval consumer actually uses?**

A TPR fit maps a core `c ∈ R^{m·d}` to the observed state `e ∈ R^D` through a
fitted `W ∈ R^{D×(m·d)}`. When `m·d ≳ D` that map is under-determined: it
reproduces every training state exactly, the residual collapses to ~0, and the
fit's apparent quality says nothing about generalization. Running T4 on such a
fit would produce a confident number with no content — the "verify the
instrument first" failure, in the one place it is cheapest to catch.

The shipped consumer state is **8-D** (`riir_rag::ModellessEmbedder`,
`LATENT_DIM = 8`; the crate-local `embedder::EMBED_DIM` is also 8 and
additionally mixes the lint name into the vector, which would leak the role
into the state and is therefore not usable here at all).

So this bench measures admissibility, and only then structure.

## 2. The scheme decision (resolves the open question Issue 062 recorded)

Issue 062's Phase 2 preamble left the role scheme undecided, noting that a
role-per-rule scheme over `kernel_opt`'s 426 rules is not expressible. It is
resolved here, and the axes are **inverted relative to Phase 1's naming**:

| axis | Phase 1 (T1/T2 retrieval arms) | this fit |
|---|---|---|
| role (`m` blocks) | rule identity (45 / 8) | **shape-class** (`m = 8`) |
| filler (table rows) | shape-class (8) | **rule identity** (45 / 8 / 426) |

Three reasons, in order of force:

1. **`Orthogonal { arity: m }` has `n_bind_slots == arity`**, so the role count
   IS the block count. Role = rule identity means `m = 45` and `K = m·d ≥ 45`
   at any `d ≥ 1` — already 5.6x the `D = 8` state dimension, i.e. inadmissible
   before `d` is even chosen. Role = shape-class fixes `m = 8`.
2. **Filler count is unconstrained.** Rule identity as the filler is the only
   assignment under which `kernel_opt`'s 426 rules are expressible at all.
   This is what katgpt-rs Issue 707's handoff note recommended verbatim
   ("`Orthogonal` with a small block count and put rule identity in the
   filler"); this bench is where that recommendation is priced.
3. **It keeps the composition claim intact.** The (role, filler) *pair* is
   unchanged — only which side is a block and which is a table row. An unseen
   (shape, rule) combination is still answerable by composition, which is the
   whole systematicity claim.

This is a naming inversion, not a change of the measured object: the cells,
the eligibility filter and the withheld-pair protocol of Bench 062 all carry
over unchanged.

## 3. Two state spaces, and why the second one is a control, not a second bet

| arm | `D` | role in this bench |
|---|---|---|
| `latent8` | 8 | **the consumer space.** The only arm whose result can feed T4. |
| `tokenbag256` | 256 | **the diagnostic control.** Signed BLAKE3 hashing of the shipped `normalized_tokens()` stream into 256 bins, L2-normalized. Modelless, deterministic, no new deps, reuses the exact token stream Bench 062's atomic null already keys on. |

The control exists because an inadmissible `latent8` result is **uninterpretable
on its own**. "TPR cannot help this corpus" and "the 8-D state is too narrow for
any structured method to be fitted in it" predict the identical `latent8`
observation and have opposite consequences for T4. Only a space where the fit IS
admissible can separate them. The control is ~20 lines (an embedder), not a
second harness, and it is explicitly **not** a candidate for the T4 consumer
path — a 256-D state is not what the reranker scores in.

## 4. Admissibility is two-sided

A one-sided check ("did the fit converge") passes on a vacuous fit. Both bounds
are asserted, per role as well as globally:

- **over-parameterization** — `params < n_states · D` (scalar observations).
  `params = D·K + D` (the `W, b` block) `+ n_fillers·d` (the filler table).
- **per-role starvation** — every role block must see at least `d + 1` states.
  A role with `d` or fewer states has a `D×d` readout fitted from fewer than
  `d` independent state vectors: locally under-determined even when the global
  count passes.

A fit that fails either bound is reported as **INADMISSIBLE** and its residual
/ OOD numbers are **not** read as evidence in either direction. That is the
whole point of running this first.

## 5. Predictions (preregistered)

- **P1** `latent8` is INADMISSIBLE at every `(m, d)` with `m = 8, d ≥ 1`:
  `K = 8d ≥ 8 = D`, so `params ≥ 8·8 + 8 + 45 = 117` against
  `n_states·D = 113·8 = 904` — the global bound passes, but `d = 1` is the only
  `d` where `K ≤ D`, and the per-role bound is expected to fail on the sparse
  shape-classes. Direction of the claim: **`latent8` cannot support a
  generalizing fit**, and T4 as written is blocked on the state width, not on
  TPR.
- **P2** `tokenbag256` is ADMISSIBLE at `m = 8` for `d ≤ 8`
  (`params = 256·64 + 256 + 45·8 = 16,999` vs `113·256 = 28,928`).
- **P3** BIC over `d ∈ {1, 2, 4, 8}` prefers a **small** `d` (≤ 4) on both
  spaces — the corpus has ~113 states over 45 fillers, ~2.5 states per filler,
  so wide filler rows are unaffordable.
- **P4** `bow_router` on `tokenbag256` reports `structured = true` for
  `clippy_lints` (13 multi-filler roles per Bench 062 §10) and
  `structured = false` or a ratio within `eps` for `rustc_errors` (6/8 error
  codes single-filler). This is Bench 062 §10's prior, now tested.
- **P5** `shuffled_role_control` degrades on any arm where `bow_router` says
  structured. A structured verdict with a non-degrading shuffle is a harness
  defect, not a finding (roles cannot matter and be permutable at once).
- **P6** The double fit is **bit-identical** on both spaces (artifact
  commitment equality), and `monotone_violations == 0` — the upstream G1
  contract, re-checked at this consumer rather than assumed.

## 6. Verdict rule (preregistered)

- **T4 PROCEEDS** only if `latent8` is ADMISSIBLE **and** `bow_router` reports
  structured on `latent8`. Anything else and T4 is not runnable as specified.
- **T4 BLOCKED-ON-STATE-WIDTH** if `latent8` is INADMISSIBLE while
  `tokenbag256` is ADMISSIBLE and structured. The blocker is then a *state
  dimension* finding: the fix is a wider consumer state (a bridge), which is a
  riir-rag/`LATENT_DIM` concern and must be filed there, not worked around here.
- **T4 DEMOTED (honest no-gain)** if `tokenbag256` is admissible and
  `bow_router` says **not** structured. That would mean the healer corpus
  carries no role-filler binding structure a sum of embeddings does not already
  carry, and the structured machinery should not be adopted for it — the
  demote-loser branch Issue 062's G8 already reserves.
- Any INADMISSIBLE arm contributes **no** evidence to any of the three.

Nothing below this line existed at preregistration.

## 7. Results

```
CARGO_TARGET_DIR=/tmp/riir_clippy_063 cargo test --release \
  --features tpr_structure --test t063_tpr_structure -- --nocapture
```

Grid `d ∈ {1, 2, 4, 8}`, all four arms. `obs = n_states · D`.

### 7.1 `latent8` — the consumer space: INADMISSIBLE everywhere

| domain | n_states | m | n_fillers | d | K | params | obs | resid | verdict |
|---|---|---|---|---|---|---|---|---|---|
| clippy_lints | 113 | 8 | 45 | 1 | 8 | 117 | 904 | 8.745e-1 | INADMISSIBLE — K ≥ D |
| clippy_lints | 113 | 8 | 45 | 2 | 16 | 226 | 904 | 7.784e-1 | INADMISSIBLE — K ≥ D; `generic`=2 |
| clippy_lints | 113 | 8 | 45 | 4 | 32 | 444 | 904 | 6.802e-1 | INADMISSIBLE — K ≥ D; 2 starved |
| clippy_lints | 113 | 8 | 45 | 8 | 64 | 880 | 904 | 5.721e-1 | INADMISSIBLE — K ≥ D; 3 starved |
| rustc_errors | 212 | 10 | 8 | 1 | 10 | 96 | 1696 | 9.313e-1 | INADMISSIBLE — K ≥ D; `plain-binding`=1 |
| rustc_errors | 212 | 10 | 8 | 8 | 80 | 712 | 1696 | 9.180e-1 | INADMISSIBLE — K ≥ D; 3 starved |

Role census (clippy): `generic`=2 `closure`=4 `impl-block`=5 `iter-chain`=9
`loop`=9 `multi-item`=18 `macro`=20 `plain-fn`=46.
Role census (rustc): `plain-binding`=1 `as-view`=5 `format-macro`=5
`slice-borrow`=10 `string-owner`=16 `struct-field`=16 `mut-borrow`=26
`closure-capture`=27 `index-expr`=47 `vec-owner`=59.

### 7.2 `tokenbag256` — the control: admissible at `d = 1` only, and STRUCTURED

| domain | d | K | params | obs | resid | verdict |
|---|---|---|---|---|---|---|
| clippy_lints | 1 | 8 | 2349 | 28928 | 7.397e-1 | **ADMISSIBLE** |
| clippy_lints | 2 | 16 | 4442 | 28928 | 5.529e-1 | INADMISSIBLE — `generic`=2 |
| clippy_lints | 4 | 32 | 8628 | 28928 | 3.795e-1 | INADMISSIBLE — `closure`=4, `generic`=2 |
| clippy_lints | 8 | 64 | 17000 | 28928 | 2.834e-1 | INADMISSIBLE — 3 starved |
| rustc_errors | 1 | 10 | 2824 | 54272 | 2.884e-1 | INADMISSIBLE — `plain-binding`=1 |

- BIC over the admissible set `{1}` → `d = 1`, label `orthogonal:m8:d1`, score −203383.3.
- `bow_router`: `r_bow` 8.7091e-1, `r_full` 7.3973e-1, **ratio 1.1773, structured = true**.
- `cross_state_role` (see §7.4): `r_true` 7.3973e-1, `r_permuted` 7.9982e-1,
  **ratio 1.0812, degraded = true**, moved 81/113.
- `shuffled_role` (upstream): ratio **1.0000**, degraded = false — **VACUOUS**, §7.4.

### 7.3 Predictions, scored honestly

| | prediction | outcome |
|---|---|---|
| **P1** | `latent8` inadmissible; T4 blocked on state width | **CONFIRMED — by a different bound than P1 argued.** P1 reasoned via the global params bound, which *passes* (117 < 904). The bound that actually fires is `K = m·d ≥ D`, at `d = 1` already (K=8 vs D=8 clippy; K=10 > 8 rustc). Right conclusion, partly wrong mechanism; recorded rather than rewritten. |
| **P2** | `tokenbag256` admissible for `d ≤ 8` | **REFUTED.** Admissible at `d = 1` only. P2's arithmetic priced the global bound (16,999 < 28,928 — correct) and omitted the per-role bound, which kills `d ≥ 2` on `generic`=2. The two-sided check refuted the prediction of the person who wrote it. |
| **P3** | BIC prefers small `d` (≤ 4) | **CONFIRMED but WEAK** — `d = 1` was the only admissible candidate, so BIC had nothing to choose between. Not evidence about `d`. |
| **P4** | clippy structured; rustc not | **HALF CONFIRMED.** clippy structured (ratio 1.1773 + cross-state 1.0812). rustc is **unmeasurable, not unstructured**: inadmissible at every `d` because `plain-binding` has exactly 1 state. Bench 062 §10's rustc prior remains untested. |
| **P5** | structured ⇒ role shuffle degrades | **the control was VACUOUS — §7.4.** |
| **P6** | double fit bit-identical, `monotone_violations == 0` | **first half CONFIRMED** (all 16 grid points bit-identical, 5 processes). Second half **WRONG AS WRITTEN** — §7.5. |

### 7.4 P5: the upstream role-shuffle control cannot fail on this corpus

`shuffled_role_control` permutes each state's role list **within that state**:

```rust
for i in (1..n).rev() { ... roles.swap(i, j); }
```

Every state in this corpus carries exactly **one** binding (one span, one rule,
one shape), so `n == 1` and the loop body never executes. The permutation is a
**provable identity**: measured `r_shuffled == r_true` bit-exactly, ratio
1.0000, `degraded = false`.

That output is **indistinguishable from "the roles carry nothing"** — the same
vacuous-control signature Bench 062 §caveat-1 found in the atomic null, one
level up. Read naively it would have contradicted the router and looked like a
harness defect on our side.

Replaced by `cross_state_role_control` (`src/draft/tpr_fit.rs`): permute the
role assignment **across** states, fillers untouched, refit. That is what "are
the roles load-bearing?" means when no state has two bindings to swap. It moved
81 of 113 roles and the residual rose 8.1% (`degraded = true`), so the clippy
structure verdict survives a control that *can* fail. The `structured()`
verdict now requires the router **and** this control to agree — a router
verdict alone would credit any residual drop extra per-block parameters buy.

The vacuity is detected and asserted, not assumed: the test fails if the corpus
ever grows multi-binding states (at which point the within-state control becomes
real), and fails if the "vacuous" arm ever moves the residual.

### 7.4a Upstreamed — this section describes a consumer-local fix that no longer exists

Filed upstream as katgpt-rs `.issues/710`; **RESOLVED there in `af3bca25`**
(issue closed + removed), which is the last commit this section's
`cross_state_role_control` / `per_state_shuffle_is_vacuous` names are valid at.
Upstream now ships:

- `RoleShuffleMode::{WithinState, CrossState}` + `shuffled_role_control_with`,
  with `role_shuffle_mode_for` resolving the non-vacuous arm automatically, so
  a single-binding consumer gets a control that can fail *by default*.
- `ShuffledRoleReport::{vacuous, moved, mode}` — vacuity is a reported
  quantity, not something a consumer infers by reading the permutation loop.
  Vacuity is judged on **distinct role labels**, not list length, so an
  all-same-role corpus is caught on both arms (a case this bench's local
  predicate would have missed).
- The same discipline extended to the other two instruments: `bow_router`
  gains `vacuous` (at `arity == 1` with all-zero roles its "null" IS the
  caller's fit), and `candidate_pool_coverage` is the answerability check
  `withheld_pair_top1`'s doc had always demanded with nothing to demand it of.

`tpr_fit.rs` now calls the upstream function on both arms (~90 lines of
duplicated control deleted) and **reproduces every number in this document
bit-identically** — clippy `tokenbag256` 1.1773 / 1.0812 moved 81/113,
coalesced 1.1417 / 1.0693 moved 79/113, `latent8` coalesced 0.9961 / 0.9897
moved 79/113. Same SplitMix64, same draw order, so the port is behaviour-
preserving and the §6 verdict below is unchanged.

One verdict rule did change, and it is a strengthening: `SpaceReport::structured()`
now returns `None` — UNKNOWN — when **either** instrument reports itself
vacuous, rather than the `Some(false)` a vacuous control would otherwise
manufacture. That is the whole lesson of this section applied to the verdict
that reads it.

### 7.5 P6: one monotone violation, and why asserting zero was wrong

`clippy_lints/latent8 d=1` reports `monotone_violations = 1`. The first cut of
the test asserted `== 0` and went red on it.

That assertion misread the upstream contract: an uphill sweep is **rejected and
rolled back**, so a violation is the descent guard *working*. It fires here
because `K == D` makes the fit exactly determined, so f32 noise at the
convergence floor exceeds the `1e-9·ssr_0` band. The assertion is now the
**rollback certificate** — `final_ssr ≤ min(ssr_per_sweep)` — which is what the
guard actually promises. Asserting the counter would have been a permanent
false red on a correct fit.

`param_count` from the artifact is asserted equal to this module's analytic
`params` at every grid point, so the admissibility verdict — computed from
geometry *before* any fit — is not decided by local arithmetic that drifted
from upstream's.

## 8. Verdict: T4 BLOCKED-ON-STATE-WIDTH

§6 branch 2, unambiguously: the control arm is admissible **and** structured
(two agreeing probes), while the consumer arm is inadmissible at **every** `d`
in the grid — and not marginally. `K = m·d ≥ D` cannot be satisfied at
`m = 8, d ≥ 1` against `D = 8` except at the boundary, and the boundary is
where `W` reproduces any state.

So the finding is **not** "TPR does not help the healer corpus". The corpus
carries role-filler binding structure that a bag of fillers does not
(ratio 1.1773) and that survives breaking the pairing (ratio 1.0812). The
blocker is the **width of the state the reranker scores in**: 8 dimensions
cannot host a role-filler decomposition over 8 shape-classes, because the core
is already 8-wide at the smallest possible filler dimension.

This is a **dimension** finding, and it is why running T3 before T4 was worth
doing: T4 as specified ("score against the TPR projection instead of the raw
8-D embedding") is not runnable, and the OOD number it would have produced
would have come off a fit that reproduces its own training states.

### What unblocks T4 — two routes, neither taken here

1. **Widen the consumer state.** `riir_rag::query::LATENT_DIM = 8` is a
   cross-repo contract (riir-rag + riir-neuron-db indexes + every committed
   floor); widening it is a riir-ai concern with a large blast radius and must
   be filed there, not worked around in this repo.
2. **Bridge instead of widen** (cheaper, and the repo's own pattern). Keep the
   8-D scoring state; fit TPR in the wider token-bag space; emit a **scalar**
   role-conditioned match (`unbind_state_into` against the candidate rule's
   filler row) and fuse that scalar into the existing rank. Latent structural
   evidence → one scalar → the raw score, which is exactly the documented
   latent→raw bridge. It needs no change to `LATENT_DIM` and no index rebuild.

Route 2 is the recommended T4 re-scope. It changes T4 from "replace the
scoring space" to "add a structural evidence channel", which is also what the
Bench 062 §10 headroom description already pointed at (the closable part is
*retrieved-but-demoted* rows — a re-ranking shape, not a retrieval shape).

## 9. What this bench does NOT establish

- **No OOD number.** The withheld-pair OOD arm (`withheld_pair_top1`) was not
  run: on the consumer space it would be read off an inadmissible fit, and on
  the control space it would measure a state the reranker never sees. Bench
  062's T1/T2 OOD baselines stand unchallenged and unimproved.
- **Nothing about `rustc_errors`.** Inadmissible at every `d` (one role has a
  single state). Its Bench 062 §10 BoW prior is still a prior.
- **Nothing about `kernel_opt`.** The scheme decision (§2) makes its 426 rules
  *expressible* as fillers, which is what it was blocked on, but no
  kernel_opt population was fitted here.
- **No floors touched.** `tpr_structure` is opt-in, adds no default code path,
  and modifies no existing eval. The 14/14 retrieval gate and score-bench are
  untouched by construction, not by measurement.
- **`d = 1` is a narrow admissible window.** One extra `generic`-shape span
  would open `d = 2`. The window is corpus-limited, so the structure verdict is
  a verdict at the sparsest admissible geometry — the *weakest* setting the
  claim could have been made in, which cuts in its favour but bounds its size.

---

## 10. Addendum (preregistered 2026-09-02, after §1-9 landed at `74d99e9`)

### 10.1 Why an addendum rather than a re-run

Two of §7's results are limited by the same thing, and it is fixable:

- **`rustc_errors` is unmeasurable at every `d`** because `plain-binding` holds
  exactly **1** of 212 spans. It is `classify_rustc_shape`'s final `else`
  branch — the catch-all — so a population of 1 means the classifier is nearly
  *total* on this corpus. Good classifier, fatal for the per-role bound.
- **P3 was confirmed only weakly**: `d = 1` was the sole admissible candidate,
  so BIC had nothing to choose between and its "preference" was vacuous.

Both are consequences of one or two near-empty role blocks, not of the corpus.
A role with `≤ d` states carries no fittable readout — so give it none.

### 10.2 The coalescing rule (mechanical, fixed before running)

For each `d`, before fitting:

1. Every role with `≤ d` states is merged into a single synthetic role
   `other`.
2. If `other` itself ends with `≤ d` states, its states are **dropped** and the
   count is reported. Dropping is preferred to fitting a block that cannot be
   fitted; a silent drop would be the real defect, so the count travels in the
   report.
3. Surviving role ids are remapped to a contiguous `0..m'` (required by
   `Orthogonal`, whose block index *is* the role id).

This is the "fold rules to categories" move katgpt-rs Issue 707's handoff note
offered as the alternative to a wide role vocabulary, applied to the axis that
actually needed it. It is **not** free: it lowers role resolution, so the
structure claim it supports is a claim about the coarser vocabulary.

Projected geometry (arithmetic, pre-run — the per-role bound needs `> d`):

| arm | d | roles ≤ d | `other` | m' | dropped | expected |
|---|---|---|---|---|---|---|
| clippy | 1 | — | — | 8 | 0 | admissible (already) |
| clippy | 2 | generic(2) | 2 | 7 | 2 | admissible after drop |
| clippy | 4 | generic, closure | 6 | 7 | 0 | admissible |
| clippy | 8 | generic, closure, impl-block | 11 | 6 | 0 | admissible |
| rustc | 1 | plain-binding(1) | 1 | 9 | 1 | admissible after drop |
| rustc | 4 | plain-binding | 1 | 9 | 1 | admissible after drop |
| rustc | 8 | as-view, format-macro, plain-binding | 11 | 8 | 0 | admissible |

### 10.3 Predictions

- **P7** Coalescing makes `rustc_errors` admissible at ≥1 `d`, so its BoW
  verdict becomes **measurable**. §7 recorded it as *unmeasurable*, not
  unstructured; this is where that distinction gets cashed. Direction claimed:
  Bench 062 §10's prior says rustc should show **little** role gain (6/8 error
  codes single-filler), so a ratio near 1 is the *expected* outcome and would
  confirm the prior rather than refute the method.
- **P8** With a genuinely multi-candidate admissible set, BIC selects `d > 1`
  on at least one arm — a real test of P3, which §7.3 could not give it.
- **P9** The clippy structure verdict **survives** coalescing: both probes
  still fire (`bow_router` ratio > 1.05 AND `cross_state_role_control`
  degraded). If it does not, §8's conclusion narrows to the fine-grained role
  set and must say so — the verdict would then have been carried by the sparse
  roles coalescing removes.
- **P10** The clippy BoW ratio **falls** under coalescing (coarser roles carry
  less than the full set). Direction only; no magnitude claimed.
- **P11** The uncoalesced numbers are **unchanged** by this addendum — the
  coalescing path is additive, so §7's table must reproduce bit-identically in
  the same run. A shift there is a harness defect, not a finding.

### 10.4 Verdict rule

- T5's **per-domain structure verdict** is read from the coalesced arm at BIC's
  chosen `d`, recorded per domain, with `m'` and the dropped count alongside —
  a verdict over a coalesced vocabulary is not a verdict over the full one.
- §8's `T4 BLOCKED-ON-STATE-WIDTH` verdict is **not** revisitable by
  coalescing: `latent8` fails on `K = m·d ≥ D`, and coalescing only *shrinks*
  `m`, so at best it reaches the same `K == D` boundary. If some coalesced
  `latent8` arm nonetheless reports admissible, that is a bug in the bound and
  must be treated as one.
- If P9 fails, §8 is amended in place rather than left standing.

Nothing below this line existed at addendum preregistration.

## 11. Addendum results

Same run, same process, all eight arms. Uncoalesced §7 rows reproduce
bit-identically (P11, asserted in the harness). Coalesced threshold `T = 8`.

### 11.1 Coalescing did what §10.2a projected — zero drops on both domains

| arm | merged into `other` | `other` | m' | dropped | n_states |
|---|---|---|---|---|---|
| clippy | `closure`(4), `generic`(2), `impl-block`(5) | 11 | **6** | 0 | 113 |
| rustc | `as-view`(5), `format-macro`(5), `plain-binding`(1) | 11 | **8** | 0 | 212 |

### 11.2 The consumer space became measurable — and measures FLAT

`clippy_lints/latent8 [coalesced T=8]`, m'=6:

| d | K | params | obs | resid | verdict |
|---|---|---|---|---|---|
| 1 | 6 | 101 | 904 | 8.738e-1 | **ADMISSIBLE** |
| 2 | 12 | 194 | 904 | 8.118e-1 | INADMISSIBLE — K ≥ D |
| 4 | 24 | 380 | 904 | 6.695e-1 | INADMISSIBLE — K ≥ D |
| 8 | 48 | 752 | 904 | 5.957e-1 | INADMISSIBLE — K ≥ D |

- BIC over `{1}` → `d = 1`, label `orthogonal:m6:d1`, score −2094.8.
- `bow_router`: `r_bow` 8.7039e-1, `r_full` 8.7381e-1, **ratio 0.9961,
  structured = false**.
- `cross_state_role`: `r_true` 8.7381e-1, `r_permuted` 8.6483e-1, **ratio
  0.9897, degraded = false**, moved 79/113.

Both probes land **below 1.0**, and that is the informative part. `r_bow`
*beats* `r_full`: the `m = 1` bag-of-fillers fit reconstructs the 8-D state
slightly better than the 6-block structured fit, and permuting the role
assignment across states made the fit slightly **better** still. At `d = 1` and
`D = 8` the roles are not merely unhelpful — the role assignment behaves like
noise in this space.

`rustc_errors/latent8 [coalesced]` stays inadmissible at every `d`: it coalesces
to m'=**8**, so `K = 8 == D = 8` exactly at `d = 1`. The consumer measurement
exists for clippy only.

### 11.3 The control: all four `d` admissible, and rustc is the MOST structured arm

| arm | d admissible | BIC scores (d=1,2,4,8) | chosen | `bow` ratio | `cross_state` ratio | structured |
|---|---|---|---|---|---|---|
| clippy/tokenbag256 [coalesced] | 1,2,4,8 | −207755.1, −199354.3, −176567.4, −122511.8 | **1** | 1.1417 | 1.0693 (79/212 moved) | **true** |
| rustc/tokenbag256 [coalesced] | 1,2,4,8 | −500429.6, −485136.7, −440309.1, −350653.0 | **1** | **2.2096** | **2.6661** (178/212 moved) | **true** |

### 11.4 Predictions, scored

| | prediction | outcome |
|---|---|---|
| **P7** | rustc becomes measurable; expected ratio **near 1** per Bench 062 §10's prior | **half CONFIRMED, direction REFUTED.** Measurable: yes. Near 1: no — rustc is the **most** structured arm measured (`bow` 2.2096, `cross_state` 2.6661), well above clippy. **Bench 062 §10's rustc prior does not transfer**, and §11.6 explains why: it was a statement about the *error-code* axis, which this scheme puts on the **filler** side. A prior about one axis was applied to the other. |
| **P8** | BIC selects `d > 1` on ≥1 arm | **REFUTED, and now strongly.** With all four `d` admissible on both control arms, BIC still picks `d = 1`, and the scores rise **monotonically** with `d` on both — the extra parameters never pay for themselves. This converts §7.3's weak P3 into a real result: `d = 1` is preferred, not merely the only option. |
| **P9** | the clippy structure verdict survives coalescing | **CONFIRMED** on the arm that had a verdict (control: 1.1417 + 1.0693, both probes still firing). §8 therefore does **not** need narrowing to the fine-grained role set. |
| **P10** | the clippy BoW ratio falls under coalescing | **CONFIRMED.** 1.1773 → 1.1417. Coarser roles carry less, as claimed, and by little. |
| **P11** | uncoalesced numbers unchanged | **CONFIRMED**, asserted in-harness rather than eyeballed. |

### 11.5 §10.4's own guard was REFUTED — by the arithmetic it asserted

§10.4 stated that coalescing "cannot revisit" the §8 verdict because it "only
shrinks `m`, so at best it reaches the same `K == D` boundary", and instructed
that an admissible coalesced `latent8` arm be treated as a **bug in the bound**.

The run produced exactly that arm, and the bound is correct: shrinking `m` is
precisely what takes `K = m'·d` **below** `D`. Clippy coalesces to m'=6, so
`K = 6 < 8`. The prediction had silently held `m = 8` fixed while varying it.

The guard is replaced by the invariant it was reaching for — *admissible implies
all three conditions*, asserted on every arm — which catches a bound that has
drifted from the verdict it decides without forbidding a legitimate result. The
original wording is left above rather than corrected in place.

This is the second time in this bench that a preregistered claim was wrong in
the same direction: reasoning about one bound while a different one decided the
outcome (§7.3 P1, §7.3 P2, and now §10.4). The two-sided check is what caught
all three.

### 11.6 §8 AMENDED: the verdict is stronger than BLOCKED-ON-STATE-WIDTH

§6 offered four branches. The run produced a fifth, named
**T4 BLOCKED-ON-STATE-CONTENT**:

> the consumer state **IS** measurable (clippy, coalesced, `d = 1`) and carries
> **no** role-filler structure — both probes below 1.0 — while the wider control
> carries strong structure on both domains.

That is strictly stronger evidence for the same consequence as §8's branch 2.
§8 inferred "the 8-D state cannot host the structure" from that space being
*unmeasurable*; §11 **measures** it there and finds it flat. T4's re-scope to
the bridge route (Bench 063 §8 route b) is unchanged, and its justification no
longer depends on an inadmissibility argument.

Route **(a) widen `LATENT_DIM`** is correspondingly *more* attractive than §8
implied, not less: the structure demonstrably exists in the code text (control
`bow` 1.14 clippy / 2.21 rustc) and demonstrably does not survive projection to
8 dimensions. But the blast radius argument stands, so the recommendation is
unchanged — file it in riir-ai, do not work around it here.

### 11.7 The confound this bench cannot remove, stated plainly

The shape-class role is computed **from the span text**
(`classify_rustc_shape` looks for `format!`, `[..]`, `&mut `, …), and the
token-bag state is hashed **from the same text**. So the role is partly readable
off the state by construction, and a per-role readout is fitting a partition
that correlates with what it must reconstruct.

This bounds what "structured" means here: it means *the role partition carries
information the bag of fillers does not*, which is what `bow_router` compares
and what `cross_state_role_control` confirms by destroying the pairing. It does
**not** establish that a role derived independently of the span would do as
well. Any modelless role is a function of the input, so this confound is not
removable within the modelless mandate — it is a bound to be stated, not a
defect to be fixed, and it is the same collinearity concern Bench 062
§caveat-2 raised, in a new form.

It also explains rustc's 2.2/2.7 ratios without appeal to deeper structure:
`rustc_errors` has only **8** fillers against 8 role blocks, so the role
partition is a large share of the total signal available. Read the *ordering*
(rustc > clippy) as a property of the two corpora's role/filler ratio, not as
"rustc code is more compositional".

### 11.8 What §11 hands T5

T5's per-domain BoW verdict, over the **coalesced** vocabulary (§10.4 requires
saying so), at BIC's `d = 1`:

| domain | space | m' | verdict |
|---|---|---|---|
| clippy_lints | consumer 8-D | 6 | **NOT structured** (0.9961 / 0.9897) |
| clippy_lints | control 256-D | 6 | **structured** (1.1417 / 1.0693) |
| rustc_errors | consumer 8-D | 8 | unmeasurable (`K == D` at `d = 1`) |
| rustc_errors | control 256-D | 8 | **structured** (2.2096 / 2.6661) |

`kernel_opt` is still unmeasured — the §2 scheme decision makes its 426 rules
*expressible* (as fillers), which is what it was blocked on, but no kernel_opt
population exists in this harness. That is a corpus-plumbing task, filed as
Issue 062 T5b.

---

## 12. Addendum: the `kernel_opt` arm (Issue 062 T5b)

**Preregistered 2026-09-02**, after §1–11 landed and after the run was
launched but **before any of its output was read** — stated plainly because
"before the run" is the stronger claim and this is not it. Nothing below was
written with a number in hand.

### 12.1 Why this arm and not another

`kernel_opt` was §2's *motivating* corpus (the largest; 445 distinct states at
the revision measured here, and actively growing — read every count in this
section as of `02e907e`, not as a property of the corpus) and the
reason the scheme decision came out the way it did — rule identity had to go on
the filler axis because `n_bind_slots == arity` caps roles at the block count
and 426 roles against `D = 8` is inadmissible on its face. But no
`kernel_opt_population()` existed, so the §11 table has no row for it and T5's
original prediction about it ("categories possibly weaker than rule-level")
was never tested. This adds the population; the mechanism is unchanged.

**It is also the only arm outside §11.7's confound.** The clippy and rustc
roles are computed by `classify_shape` / `classify_rustc_shape` from the same
span text the state is hashed from, so a role there is partly readable off the
state, and §11.7 recorded that as bounding what "structured" may claim.
`kernel_opt`'s role is `KernelCategory` — **declared corpus metadata**, authored
alongside the rule and not a function of `ref_shape`. A structure verdict here
is not exposed to that objection. That makes this arm worth more than a third
data point: it is the one that can say whether §11's control-space structure
survives when the role stops being derivable from the state.

### 12.2 Preregistered predictions

| # | prediction | reasoning |
|---|---|---|
| **P12** | `kernel_opt` is **INADMISSIBLE at every `d` in the 8-D consumer space, coalesced and not** | `K = m·d ≥ D` fires at `d = 1` with `m = 15` vs `D = 8`. Coalescing at `T = 8` merges only categories holding ≤ 8 of ~426 states; with ~28 states/category on average, few merge, so `m'` stays well above 8. If this is wrong it is because the category sizes are far more skewed than the mean suggests. |
| **P13** | the 256-D control is **admissible at every `d`** | `K = 15·8 = 120 < 256` at the worst `d`; `params ≈ 4.5 k` against `~109 k` observations at `d = 1`. The binding constraint, if any, is the per-role one — a category holding ≤ `d` states kills that `d`. |
| **P14** | BIC picks **`d = 1`** | Both other arms did, monotonically, and §11.3 retired the "K too shallow" class of hypothesis for this fit. A different answer here would be the first evidence that result was corpus-specific rather than structural. |
| **P15** | the control arm is **structured**, and by a ratio **above clippy's 1.1417** | The category partitions rules by GPU concept, and reference kernels genuinely differ in text by concept. Registered as the directional call even though it cuts AGAINST T5's original "categories possibly weaker than rule-level" prior — which was written before §2 forced categories onto the *role* axis, so it was a prediction about a different axis assignment and should not be imported. |
| **P16** | `moved > 0` and `vacuous == false` on the cross-state control for every arm that runs | 15 distinct role labels. This is the katgpt-rs Issue 710 check, now upstream, and it is asserted rather than read. |

### 12.3 What would make this arm change a conclusion

- **P15 wrong and the ratio is FLAT (≤ 1 + ε) on the 256-D control.** That is
  the outcome that matters most, because it is the one §11.7 predicted as a
  possibility and could not rule out: if the two confounded arms are structured
  and the one unconfounded arm is not, then §11's control-space structure is
  substantially an artifact of the role being readable off the state, and §11.3's
  "rustc is the most structured arm" should be discounted accordingly.
- **P12 wrong** (the consumer space IS admissible here) would give T4 a second
  measurable consumer arm and reopen the question §11.6 closed. It would not
  change T4's recommendation on its own — the clippy 8-D measurement stands —
  but it would mean "the 8-D state cannot host this structure" needs restating
  per domain rather than as a property of the width.

### 12.4 Results — P15 refuted, and the refutation does NOT mean what §12.3 said it would

Run: `--release --features tpr_structure --test t063_tpr_structure`, in an
isolated worktree (a concurrent session's in-flight `src/modelability.rs` did
not compile at the time; the worktree is `--detach` at `02e907e` with only this
change applied, so the isolation is from a broken sibling file, not from the
tree under test).

| # | prediction | outcome |
|---|---|---|
| **P12** | `kernel_opt` inadmissible at every `d` in the 8-D consumer space, coalesced and not | **CONFIRMED.** Both arms `admissible_d = []`. `K = m·d ≥ D` at `d = 1` with `m = 15` vs `D = 8`, and coalescing at `T = 8` merged only 4 categories (sizes 4, 5, 5, 4) to `m' = 12`, still far above 8. |
| **P13** | the 256-D control admissible at every `d` | **CONFIRMED coalesced** (all four `d`, `K = 96 < 256` at the worst, 3,773–28,392 params against 113,920 observations); **HALF on the uncoalesced arm** — `d ∈ {1, 2}` only, killed at `d ∈ {4, 8}` by the per-role bound exactly as the prediction's own caveat said it would be. |
| **P14** | BIC picks `d = 1` | **CONFIRMED**, a third time, monotone in `d`. |
| **P15** | control arm structured, ratio above clippy's 1.1417 | **REFUTED, and in the direction that matters.** `bow_router` **0.9307** (`structured = false`) and cross-state **1.0147** (`degraded = false`). Not merely below clippy — below 1.0, the same `r_bow < r_full` signature §11.2 found in the flat 8-D consumer space. |
| **P16** | `moved > 0`, `vacuous = false` on every arm that ran | **CONFIRMED.** `moved = 383/445` on `kernel_opt`. The Issue 710 machinery worked exactly as intended — which is what makes this section necessary. |

**§12.3's first bullet is hereby retracted, not triggered.** It said a flat
unconfounded arm would mean §11's control-space structure is substantially the
§11.7 confound, and that §11.3's "rustc is the most structured arm" should be
discounted. That inference requires `kernel_opt` to be a *valid test of role
structure*, and it is not:

```
             states/filler   bow_router   cross-state
kernel_opt          1.00        0.9307        1.0147
clippy_lints        2.51        1.1417        1.0693
rustc_errors       26.50        2.2096        2.6661
```

`kernel_opt` has `n_fillers == n_states == 445`: every rule id occurs in
exactly one state, and every rule belongs to exactly one declared category, so
**the role is a deterministic function of the filler**. TPR's claim is
systematicity — generalizing to unseen `(role, filler)` pairs — and here there
are none. The corpus is **outside the scheme's domain of applicability**, not
inside it and unstructured. A flat reading off it is not evidence about the
confound; it is not evidence about anything.

So the honest verdict is narrower than either §12.2 or §12.3 anticipated:

- **T5's original prediction remains untested.** "kernel_opt categories
  possibly weaker than rule-level" cannot be tested on this population under
  this scheme, because the two axes are not independent. Testing it needs a
  corpus where one rule id appears under more than one category, or a different
  role axis (a *derived* shape class over `ref_shape`, which would work — and
  would re-import §11.7's confound, which is presumably why the declared
  category looked attractive).
- **§11.7 is neither confirmed nor discounted.** This arm cannot speak to it.
- **A new caution applies to §11.3.** Both probes are monotone in
  states-per-filler across all three arms. That is the shape of a capacity
  effect: the `m`-role model has `m` core blocks against the null's 1, so with
  few fillers over many states (rustc: 8 over 212) the extra blocks buy
  expressiveness whether or not any composition is being learned. This does not
  overturn §11.3 — the cross-state control is params-neutral and rustc degrades
  hard on it (2.6661) — but "most structured arm" and "arm with the most
  filler reuse" are not distinguished by anything measured here, and §11.3
  should not be cited as if they were.

### 12.5 A third vacuity mode, and it is now instrumented

Issue 710 established that a control's report must carry whether the control
could have failed. This arm is the sharper case: **a control can be perfectly
capable of failing and still measure the wrong thing.** `moved = 383/445` — the
permutation worked, neither `vacuous` flag fired, and the reported number does
not answer the question being asked of it.

Shipped here rather than left as prose:

- `FitCorpus::role_is_a_function_of_filler()` — every filler seen with `≤ 1`
  distinct role — with `filler_roles` (distinct roles per filler) as the
  underlying covariate, computed once by `filler_role_counts` and used by both
  the plain and the coalesced corpus (coalescing can only lower it, and
  lowering it to 1 is exactly what would make a verdict unreadable).
- `SpaceReport::structured()` returns **`None`** on it, joining the two Issue
  710 vacuity flags. `kernel_opt` therefore reports `structured = None`, not
  `Some(false)`.
- `print_space_report` prints `max roles/filler` and `states/filler` on every
  arm, so a *near*-degenerate corpus is visible too — the threshold alone would
  have called `kernel_opt` and a healthy corpus identical until the last filler
  tipped over.

Filed upstream as katgpt-rs **`.issues/711`** (`08310b4c`), which is where the
predicate belongs: any consumer of `bow_router` / `shuffled_role_control` /
`withheld_pair_top1` can hit this, and `withheld_pair_top1` is affected hardest
— withholding a pair withholds the whole filler, so its OOD arm is not a harder
version of the ID arm but a different question.

### 12.6 The covariate moved upstream — and reporting the MEAN found something

§12.5's machinery was consumer-local. It has since landed in `katgpt_core::tpr`
(katgpt-rs `4af2b3cf`, Issue 711 T1–T3) and this harness now **consumes** it
instead of carrying its own copy: `FillerRoleSpread { max, mean, fillers }` +
`filler_role_spread()`, riding on `BowRouterReport::spread` and
`ShuffledRoleReport::spread`, with `verdict() -> Option<bool>` on each report.

`FitCorpus::role_is_a_function_of_filler`, `filler_role_counts` and
`SpaceReport::{role_is_fn_of_filler, max_filler_roles}` are **deleted**.
`SpaceReport::structured()` is now the conjunction of the two upstream
`verdict()`s and does not re-derive either gate, so it cannot drift from the
numbers it reads — the same DRY move Issue 710's consumer step made one level
down. Two new assertions pin `report.spread == corpus.spread` on both probes:
each report measures the covariate from its *own* input, so a mismatch means
the probes did not see the corpus the row describes.

**What the mean says that the max did not.** §12.5 shipped `max` only. With the
mean printed:

| arm | `max` | **`mean`** | fillers | states/filler | `structured()` |
|---|---|---|---|---|---|
| `clippy_lints` | 4 | **1.378** | 45 | 2.51 | `Some(true)` |
| `rustc_errors` | 6 | **1.750** | 8 | 26.50 | `Some(true)` — coalesced/256 |
| `kernel_opt` | 1 | **1.000** | 453 | 1.00 | **`None`** |

Both arms that report structure are **near**-degenerate on the mean: on
`clippy_lints` the typical filler is seen with 1.38 roles, so most fillers
contribute no `(role, filler)` variation at all and the `max = 4` is carried by
a handful. That is exactly the case T2 argued the threshold could not see, and
it further weakens §11.3's "most structured arm" reading — already flagged
there as not separated from "arm with the most filler reuse". It does **not**
overturn §11.3: `structured()` is still `Some(true)` there and the guard is
still two-sided across the arm set. It does mean the margin is thinner than
`max` alone suggested, and §11.3 should be read with this column beside it.

**The numbers moved, and not because of this refactor.** A sibling's distill
batch (`93b7448`) grew `kernel_opt` 445 → 453 states between §12.4's run and
this one, so `bow_router` on `kernel_opt/tokenbag256` reads 0.9351 (was 0.9307)
and the coalesced arm 0.9525 (was 0.9307's sibling row).
**[AMENDED — see §12.8b: this sentence is arm-mismatched. `0.9307` is the
COALESCED arm's 445 reading, verified four ways in the run output, so it pairs
with 0.9525 and not with 0.9351. The unrecorded value is the UNCOALESCED arm's
at 445, the reverse of what this says.]** The covariate did
**not** move: `max = 1, mean = 1.000` at 453 states as at 445. The degeneracy
is a property of the `kernel_opt` *scheme* — one declared `KernelCategory` per
rule — not of a particular corpus size, which is the stronger version of
§12.4's claim and the reason it is worth stating separately from the ratios.

`tests/t063_tpr_structure.rs` passes on the ported harness; the two-sided
canary (`kernel_opt` fires the guard, `clippy_lints`/`rustc_errors` do not)
holds unchanged.

### 12.7 The evidence bound, measured: rustc can host the test on 2 of 8 fillers

`.issues/062` T4 asked for one number before either route is built —
`FillerRoleSpread::multi_role_fillers` on the rustc arm — because withholding a
pair from a single-role filler withholds the whole **filler**, so only fillers
seen with 2+ roles can host a withheld-pair test at all. The consumer did not
surface it; it does now, on the arm each number belongs to rather than in a
roll-up (§12.6's arm-mismatch is why).

Coalesced, 256-D, at BIC's `d = 1`:

| arm | testable fillers | distinct pairs | pair space | covered | states/pair |
|---|---|---|---|---|---|
| `clippy_lints` | **13 / 45** | 62 | 270 | 23.0% | 1.8 |
| `rustc_errors` | **2 / 8** | 12 | 64 | 18.8% | 17.7 |
| `kernel_opt` | **0 / 453** | 453 | 5436 | 8.3% | 1.0 |

**The answer is 2 — the bottom of the range T4 named.** The rustc arm's 2.2096
is the strongest structure signal in this bench, and the OOD test that would
tell us whether it generalizes can draw on two fillers. That is not a verdict
against route (b); it is the absence of the evidence route (b) would be fused
on, which is what an evidence bound means. The uncoalesced rustc arm is no
better — max 6 / mean 1.750 looks healthier on the threshold and still yields
**2 of 8** testable, which is precisely the `max`-vs-`multi_role_fillers` gap
katgpt-rs `149a0c02` was added to expose.

`clippy_lints` is the opposite shape and the one route (b) can actually
interrogate: 13 testable fillers, but 1.8 states per pair, so each pair's
contribution is roughly a single sample. Thinly observed, not thinly covered —
and unlike rustc, more corpus fixes it.

**`distinct_pairs` is counted from the bindings, and it AGREES with §12.6's
derived `fillers × mean`** (clippy 62 vs 62.01, rustc 12 vs 12.00). Recorded
because the point of counting primarily was never that the derivation was
suspect — it was that a derived aggregate cannot be arm-checked after the fact.
Here it confirms rather than corrects.

### 12.8 What corpus growth did to the guarded arm

> **Read §12.8a then §12.8b.** One claim in this section is corrected: the
> verdict is `None` (not `Some(false)`), so the covariate guard is load-bearing
> rather than redundant. §12.8a also alleged an arm mismatch in the
> `1.0147 → 1.0512` crossing; **that allegation is WITHDRAWN** — the crossing is
> same-arm and real (§12.8b; §12.8a(2) is struck at source).

Between the §12.4 run (445 `kernel_opt` states) and this one (453, sibling
batch `93b7448`), the cross-state control on `kernel_opt / tokenbag256
[coalesced]` moved **1.0147 → 1.0512** and crossed `STRUCTURE_EPS = 0.05`, so
that probe now reports `degraded = true` on the arm whose role is a
deterministic function of its filler.

Stated precisely, because the tempting overstatement is wrong: this is **not** a
false positive that the Issue 711 guard caught — the verdict was already being
withheld before this probe moved at all. What it does show is
what "unreadable, not false" means operationally — a probe on a corpus where
the structure question is not posed produced a **threshold crossing from corpus
growth alone**, with the covariate completely unmoved (`max = 1`,
`mean = 1.000` at 453 exactly as at 445). Read on its own, that row now says
"breaking the pairing costs something" about a corpus with no pairing to break.

The guard's value is therefore that `STRUCTURED` reads `None`, not that any
single number changed. A consumer reading `cross_state.degraded` directly —
which is what this harness did before Issue 711 — would now be reading that
crossing as signal.

#### 12.8a Two corrections to 12.8, both measured

**(1) The guard IS the load-bearing mechanism here, not the conjunction.** 12.8
first said the arm "lands on `Some(false)` with or without the guard", which
contradicts its own closing sentence. Measured, both `kernel_opt / tokenbag256`
arms report `STRUCTURED (router AND cross-state): None`. Without the covariate
guard the conjunction would be
`Some(bow.structured && cross.degraded)` = `Some(false && true)` = **`Some(false)`**
on the coalesced arm — a confident negative, which is exactly the
"largest corpus we have carries no binding structure" misread Issue 711 exists
to prevent. So 12.8 undersells itself: the finding is stronger than stated,
because nothing except the covariate guard produces the `None`.

**(2) ~~The `1.0147 -> 1.0512` crossing spans two different ARMS.~~ WITHDRAWN
2026-09-02.**

Wrong, and kept struck rather than deleted because *how* it was wrong is the
finding. The crossing is same-arm and real. The 445 block carrying **both**
`0.9307` and `1.0147` is self-labelled `[coalesced T=8]` in its header, with
`m(roles)=12`, `m'=12`, admissible `[1, 2, 4, 8]` and label
`orthogonal:m12:d1` — and two of those lines are emitted by
`print_space_report` **only** when `r.coalesce.is_some()` (`tpr_fit.rs:868` for
the header suffix, `:874` for the `coalesced: merged … m'=` line). So the arm
identity is the printer's own gating, not an inference from the numbers, which
is why this is not close. What is genuinely uncaptured is the **uncoalesced**
arm's 445 values — it ran, a `tail -45` just did not reach it.

The defect was inherited, not invented: §12.8a(2) cited §12.6's parenthetical,
which had itself paired a 453-uncoalesced reading with a 445-**coalesced** one
and then called the coalesced 445 value unrecorded when it is the one on hand
(§12.6 now carries that amendment). **A correction that inherits its premise
from the text it corrects is the same failure wearing the reviewer's badge** —
the fourth and sharpest instance of the session's recurring shape, and the one
rule that catches all four is: re-read the run, not the section that summarised
it. This document has now mislabelled itself twice, so a number cited from §11
or §12 prose without going back to the run output should be treated as
unverified.

**What survives is stronger than the struck claim.** Same arm, 445 → 453:
`bow_router` 0.9307 → 0.9525 **and** cross-state 1.0147 → 1.0512, the latter
crossing `STRUCTURE_EPS`. **Both** probes drift with corpus size on an arm whose
covariate is pinned at `max 1 / mean 1.000`, and neither drift means anything —
§12.8's point without needing any arm-mismatch claim. Reversal and drift
pairing: `86f34a4`.

Struck text follows, unedited.

> §12.6 already established that §12.4's `0.9307` was the **uncoalesced**
> `kernel_opt / tokenbag256` row, and `1.0147` is that same row's cross-state
> partner in the P15 table. Compared within one arm at 453 states:

> | arm | cross-state ratio | `degraded` | `structured()` |
> |---|---|---|---|
> | `kernel_opt / tokenbag256` (uncoalesced, the 1.0147 row's arm) | 1.0310 | false | `None` |
> | `kernel_opt / tokenbag256 [coalesced T=8]` | 1.0512 | true | `None` |
>
> So the arm that read 1.0147 at 445 reads 1.0310 at 453 and did not cross.
> The coalesced arm's 445-state value was never recorded, so whether it crossed
> is unknown, and "a signal manufactured by eight extra rules" is not
> supported by these numbers.
>
> *(Both sentences are false. 1.0310 is the UNCOALESCED arm at 453 — a
> different arm from the 1.0147 row — and the coalesced 445 value is exactly
> the one that was captured. The `degraded`/`structured()` column values are
> correct as measured; only the arm attribution and the conclusion are wrong.)*

Third instance today of quoting a number from a different arm than the claim it
supports (after §11.6's `rustc_errors` covariate, `1ddf4796`), and the first one
inside a finding *about* that failure. The arm label is part of the number.

#### 12.8b §12.8a(1) accepted; §12.8a(2) REVERSED — the mislabel is in §12.6

**(1) stands, and I accept it against my own text.** §12.8's "lands on
`Some(false)` with or without the guard" contradicted its own closing sentence
and undersold the finding. Without the covariate guard the coalesced arm reads
`Some(false && true)` = **`Some(false)`** — a confident negative on the largest
corpus in the bench, which is precisely the misread Issue 711 exists to
prevent. Nothing but the guard produces the `None`.

**(2) does not stand, and the arm label it corrects was already wrong one
section earlier.** The 445-state run's block carrying BOTH `0.9307` and
`1.0147` is self-labelled, four independent ways:

```
── kernel_opt / tokenbag256 [coalesced T=8] ──  n_states=445 m(roles)=12 ...
  coalesced: merged [("I: NSA block-sparse", 4), ...] -> `other`  m'=12 dropped=0
  BIC over admissible d [1, 2, 4, 8] → d=1
    label: orthogonal:m12:d1
  bow_router: ... ratio=0.9307 structured=false
  role_shuffle[cross-state]: ... ratio=1.0147 degraded=false moved=383/445
```

`[coalesced T=8]`, `m(roles)=12`, `orthogonal:m12:d1`, and `admissible_d =
[1,2,4,8]`. At 453 the **uncoalesced** arm is `m(roles)=15` with
`admissible_d = [1,2]`; the coalesced one is `m=12` with `[1,2,4,8]`. So
`0.9307` and `1.0147` are the **coalesced** arm, and:

| coalesced `kernel_opt / tokenbag256` | 445 | 453 |
|---|---|---|
| `bow_router` | 0.9307 | 0.9525 |
| cross-state | **1.0147** (`degraded = false`) | **1.0512** (`degraded = true`) |

**§12.8's crossing is same-arm and real.** What is genuinely unrecorded is the
**uncoalesced** arm's 445 values — `0.9351`'s predecessor — because the 445
capture was a `tail -45` and that block fell off the top. It ran (the verdict
table lists `admissible_d=[1,2]`), it just was not captured.

The mislabel entered at §12.6: *"`bow_router` on `kernel_opt/tokenbag256` reads
0.9351 (was 0.9307) and the coalesced arm 0.9525 (was 0.9307's sibling row)"* —
which pairs a 453 **uncoalesced** reading with a 445 **coalesced** one and then
names the coalesced 445 value as unrecorded when it is the one we have. §12.8a
inherited that pairing rather than re-reading the run, and I had cited §12.6
myself without checking it.

So this is the **fourth** arm-mismatch of the session, it sits in §12.6, and
§12.8a's closing line — *"the arm label is part of the number"* — is exactly
right and exactly what §12.6 violated. A correction that inherits its premise
from the text it is correcting is the same failure wearing the reviewer's
badge. The rule that would have caught all four is the same one: **re-read the
run, not the section that summarised it.**

`bow_router` 0.9307 → 0.9525 on the same arm is also a real drift worth having:
the router moved 2.3% on a corpus whose covariate is pinned at `max 1 /
mean 1.000`, which is §12.8's point in its stronger form — both probes drift
with corpus size where the structure question is not posed, and neither drift
means anything.
