# Bench 063: TPR structure fit on the healer corpus — admissibility first (Issue 062 T3)

**Status:** PREREGISTERED 2026-09-02 — predictions P1-P6, the scheme decision
and the verdict rule below were committed BEFORE the harness ran. No numbers
in §6 onward at preregistration time.

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

_(pending run)_
