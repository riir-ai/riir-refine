# Bench 063: TPR structure fit on the healer corpus — admissibility first (Issue 062 T3)

**Status:** RUN 2026-09-02. Verdict **T4 BLOCKED-ON-STATE-WIDTH** (§6 branch 2).
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

The vacuity is detected (`per_state_shuffle_is_vacuous`) and asserted, not
assumed: the test fails if the corpus ever grows multi-binding states (at which
point the upstream control becomes real and should be re-enabled), and fails if
the "vacuous" arm ever moves the residual.

Filed upstream as katgpt-rs `.issues/710` — any single-binding consumer of
`shuffled_role_control` gets a silent pass from it.

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

_(pending run)_
