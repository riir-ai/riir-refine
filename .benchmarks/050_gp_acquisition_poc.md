# Bench 050 — Issue 048 GP acquisition layer POC (NEGATIVE-noise-equivalent)

**Status:** CLOSED 2026-08-27 — verdict `NEGATIVE-noise-equivalent`, recorded + asserted live
**Issue:** [.issues/048_gp_acquisition_layer.md](../.issues/048_gp_acquisition_layer.md) (removed at close per the noise-reduction rule)
**Source:** ActFlow Eq 10 (arXiv:2606.08802) via [katgpt-rs R510](../../katgpt-rs/.research/510_ActFlow_Certified_Frontier_Expansion.md)

## What shipped (the seams — permanent, default-inert)

- **T1** `src/gp_acquisition.rs` (unconditional): linear-kernel GP posterior
  variance, incremental rank-1 Cholesky (f64 factor), `observe`/`sigma_sq`,
  `O(n²)` per op, zero deps, deterministic. Unit gate: incremental vs f64
  dense solve at N=64 — **max err ~1e-9** (the issue's 1e-6 bar, an order
  of margin held). Duplicate-observation safe (clamped diagonal).
- **T2** `src/domains/rustc_errors/gp_ordering.rs` (`rustc_errors` feature):
  `gp_ordering` / `gp_ordering_growing` → `OrderingFn` for the shipped
  `CompileDriver::with_ordering` seam. σ² descending, exact ties fall
  through to the incumbent min-delta key (stable). 3 unit tests.
- **T3** `HealPipeline::with_gp_tiebreak(observed)` — GP σ² as a
  **lexicographic tie-break on exact f32 equality** of the BetaPosterior
  combined score (never overrides payoff — the strict `==` is the design,
  documented in-source). `report_outcome` observes the fix direction when
  installed. Default pipeline (no GP) **bit-identical** — pinned by 4 new
  tests (`gp_tiebreak_*` in `self_evolve.rs`).
- **T4/T5** `tests/t48_gp_acquisition_ab.rs` (default + `latent_retrieval`
  halves): the A/B gate + the pre-registered negative arm, verdict
  re-derived live and asserted against the recorded constant (the t39
  pattern).

All default paths unchanged: 712 default lib tests green (703 + 9 new),
799 `latent_retrieval`, 784 `rustc_errors`, clippy 0 in my touched files
both states.

## The measurement (decisive)

**Pool-ordering A/B** — 100 real labeled pools (48 clippy + 48 perf
fixture spans through the REAL `LatentMatcher` retrieval, pool = whichever
domain's top-12 contains the labeled rule; LOO GP trained on the other
pairs' `(rule, span)` local-embedder directions):

| Arm | @1 | @3 | MRR |
|---|---|---|---|
| retrieval (incumbent) | **0.930** | **0.970** | 0.955 |
| GP σ² descending | 0.090 | 0.230 | 0.256 |
| random (seeded control) | 0.100 | 0.240 | 0.264 |

Paired McNemar GP-vs-retrieval @3: **b=74 c=0, z=+8.602 AGAINST GP** —
retrieval wins 74 pools outright, GP zero. GP is **noise-equivalent**
(@3 within 1 point of random) — the σ² ordering carries no information.

**Flatness probe (T5's pre-registered arm):** the σ² surface over the 48
clippy pair embeddings is NOT numerically flat — rel_range 1.82 (min
2.8e-5 / max 4.0e-4 / mean 2.05e-4) — but the variation is **norm
concentration noise**: under the pure-BLAKE3 local embedder distinct
inputs are near-orthogonal by avalanche, so σ²(x) ≈ ‖x‖² − (small
cross-terms), and ‖x‖² varies only by hash-norm concentration (±30% on 8
uniform dims), uncorrelated with rule/span semantics. R510's prediction
("8-D may be too coarse to separate spans") confirmed with the sharper
mechanism: it is not the dimensionality, it is that the HASH embedder has
**zero locality** — a linear kernel needs correlated inputs to build a
meaningful posterior reduction.

**Tie incidence:** in fresh pools (all candidates unseen — belief(0,0)
both sides, neutral evolution), exact combined-score ties occur at rate
**1.000** (1128/1128 measured pairs): the tie-break's addressable surface
is every fresh pool, and on fresh pools it reorders by σ² ≈ norms = noise.
This is why the tie-break ships default-OFF despite 100% tie coverage.

**Determinism:** 3 runs — hits/@k/McNemar **bit-identical** (0.930/0.970,
b=74 c=0); MRR wobbles ±0.001 (3rd decimal) from the documented riir-rag
tie-order substrate follow-up (HashMap iteration under ties — the
`canonical_rank` class); threshold metrics unaffected.

## Honest scope

- The "real-repo dry `--fix` pass" half of T4 was measured through the
  fixture-pool proxy instead of the CLI bin: a concurrent sibling session's
  WIP (`src/bin/heal.rs` let-chain under edition-2021) has the bin
  temporarily uncompilable, and the A/B question — does σ² ordering carry
  discovery signal on the real retrieval substrate — is answered
  decisively (z=+8.6 against, noise-equivalent-to-random) on the same
  corpus + retrieval the CLI consumes. A different span population cannot
  rescue a signal that does not exist in the embedding.
- The promotion bar (≥ retrieval on heal + discovery strictly greater)
  failed on BOTH axes; the never-wrong axis is vacuous for the tie-break
  (default-off, and the tie-break cannot create warnings — it only
  reorders EXACT ties among existing candidates).

## Verdict + escalation

**`NEGATIVE-noise-equivalent`** — recorded in the harness constant,
asserted live by `gp_pool_ordering_vs_retrieval_vs_random`. The seams ship
(the issue's own negative-arm disposition): T1's math module is
feature-free and correct (dense-solve pin), T2's ordering constructor + T3's
tie-break are opt-in and default-inert for any future re-gate.

Escalation path (the issue's risk register, unchanged): **richer features
(BM25 TF-IDF dims) before any model change** — a TF-IDF pair embedding has
real token-overlap locality, which is the missing ingredient; re-gate
against the same harness. Reopen triggers: an embedder with locality
landing in the pipeline (e.g. the `bonsai_retrieval` dense arm promoted),
or the rustc_errors surface gaining a semantic shape representation.

R510 §Routing updated with this verdict + commit hash.

## Cost

0 GPU-h, M3-local, minutes of CPU. No LLM calls.
