# Bench 136 — PRE-REGISTRATION: the whole-span query-text cap (issue 208 T7) — bounding the QUERY TEXT, not the query count (re-targeted by Bench 135's invariant-cost finding)

**Status:** PRE-REGISTRATION LANDED 2026-11-05 (zed-m3-i208pregate, lease on issue 208) — **NO TIMING RUN HAS HAPPENED.** Implementation-first lane, Bench 134's law verbatim: the cap lands as a code commit BEFORE any timed chunk. Nothing in this file may be amended after the first timed chunk. The run record is a SEPARATE later bench document (fresh number).

## The mechanism under test (from Bench 135's measured attribution, not a guess)

Bench 135 proved the G5-cost residual is **query-count-INVARIANT**: CAP4 (5 queries/span) vs CAP2 (3 queries/span) moved added p95 only ~5% (46.5 → 44.3 ms) — the binding constraint is the **TEXT MASS each query carries**, above all the whole-span query's `query_for` + `retrieve_scored_into` over the ENTIRE span text. The tail population is the kernel-corpus entries (`src/domains/kernel_opt/corpus/entries_*.rs`), whose rule texts live as kilobyte string literals inside single giant fns — one fn = one giant span = one massive whole-span query; a significant LINE can itself be a multi-KB literal. No per-line cap touches either.

T7 therefore bounds the **query TEXT**: every query issued by the kernel index's capped suggest path — the whole-span query AND every per-line query — carries at most `B` bytes of text. Query COUNT is untouched (the whole-span query always runs; the line queries stay stride-selected at the armed line cap exactly as Bench 134 pinned), so any cost delta is attributable to text mass alone and any utility delta to truncation alone. This is LOSSY by design (unlike Bench 134's stride selection, which was utility-neutral) — the utility floor + the paired rank-1 compare are the honesty instruments.

## The pinned design

1. **Truncation rule (head, deterministic, line-shaped):** `text.len() <= B` → unchanged (byte-identical). Else: take the first `B` bytes, walk back to a char boundary, then cut at the last `\n` within that prefix (the query stays line-shaped — BM25 never sees a torn token at the cut); if the only `\n` is at position 0 (or none exists), the char-safe byte cut stands. Zero allocation (a `&str` slice of the input). Applied to the whole-span text AND to every significant-line text handed to `retrieve_scored_into`; under `rule_embed_poc`, `phi_for` receives the same truncated whole-span text (the same query-side mass law — that feature is measurement-only, out of this bench's feature set, disclosed).
2. **Scope: the kernel index's capped suggest path ONLY** (`KernelIndex::suggest_deep_into`, flat arm — the same dispatch that honors the line cap). The public `RuleIndex::suggest_deep`/`suggest_deep_into` (every other domain's path) is byte-identical: no budget, no truncation. The route-narrow lane (Issue 196's measured-negative) is NOT in this registration.
3. **Arming:** `RIIR_KERNEL_PREGATE_QUERY_TEXT_CAP` — promote-only env, the exact literals `4096`, `2048`, `1024` arm (any other value = unset-equivalent, never armed; the env-switch law). Independent of and composable with `RIIR_KERNEL_PREGATE_SCOPE` and `RIIR_KERNEL_PREGATE_LINE_CAP`.
4. **Feature:** rides `kernel_pregate` (implies `kernel_opt`), default-off as before. Unarmed postures are byte-identical to Bench 135's binaries' behavior.

## The arms (one ON binary, env-separated; incumbent held FIXED across the compare)

| arm | posture | env |
|---|---|---|
| OFF | `latent_retrieval` | — |
| **INCUMBENT** | `latent_retrieval,kernel_opt,kernel_pregate` | `SCOPE=1` + `LINE_CAP=2` (Bench 135's best measured posture — cap4/cap2 were drift-adjacent, cap2 is the conservative-cost base) |
| **TC4096** | same binary | INCUMBENT + `QUERY_TEXT_CAP=4096` |
| **TC2048** | same binary | INCUMBENT + `QUERY_TEXT_CAP=2048` |
| **TC1024** | same binary | INCUMBENT + `QUERY_TEXT_CAP=1024` |

**Candidate-posture rule (pre-pinned):** the LARGEST budget whose BOTH cost legs clear with the utility floor held is the candidate. A clearing arm with 0 rank-1 flips vs the incumbent is claimed utility-neutral; a clearing arm with flips > 0 is recorded **cost-clear, utility-LOSSY** (flip count + cited-set delta named) and the bundle's promotion posture is the largest 0-flip clearing arm; if ONLY lossy arms clear, the promotion call is owner-gated with the flip counts named. A floor FAIL (below 15/15) refuses that arm outright (the Bench-131 kill criterion, verbatim).

## The pinned contract (Bench 134/135's, unchanged)

- Corpus: this repo's `src/**/*.rs` at the implementation commit, fresh same-session walk (all five arms one session, minutes apart — the Bench-129 posture).
- Ceilings: added (arm−OFF) p50 ≤ 1.0 ms / p95 ≤ 5.0 ms per file, nearest-rank, tail support disclosed — never re-registered (D2).
- Utility floor: 15/15 distinct rank-1 experts per armed arm; rule census disclosed per arm, never auto-demoted.
- Paired rank-1 compare: INCUMBENT ↔ each TC arm shares the span population (same posture base, only the text cap differs) — flips/lost/cited-set shrinkage disclosed per arm.
- Projection arithmetic (DISCLOSED EXPECTATION, not a claim): the embed + BM25-scan cost of a query scales with its token count, so a text cap cuts BOTH per-query embed and per-query scan. If the tail file's giant spans are ~10–50 KB, TC1024 projects an ~10–50× per-query mass cut → added p95 ~1–6 ms (marginal-green); TC2048 ~3–10 ms; TC4096 ~6–20 ms. Fixed per-query costs (rank merge, `LatentQuery` build) do not shrink — the projection is wide; the run decides.

## Verdict arithmetic (pre-pinned)

- The fence CLEARS iff the candidate arm's added p50 ≤ 1.0 AND p95 ≤ 5.0 with the utility floor held in that arm (the Bench-134 arithmetic, verbatim).
- ALL TC arms FAIL p95 → the fence verdict stays FAIL and **no further in-repo truncation rung is filed without a fresh attribution** — the named next lever becomes the substrate score-only retrieval lane (riir-neuron-db Issue 642, the complementary per-query-cost cut Bench 135 already named). Rung-chasing a non-text residual is forbidden by this pin.
- Marginal pass (< 2× headroom over either ceiling) → re-run alone before any green (the box-state law); a confirmed marginal pass is recorded marginal.

## Kill criteria

- Any armed arm's utility floor drops below 15/15 → that arm FAILS; the budget value is the finding (a deeper budget needs its own pre-registration, never an in-place amendment).
- Build red → LOUD, the leg reads BUILD-BLOCKED, never a silent skip.

## PROVENANCE law

Bench 132's disclosure block applies verbatim (m3 box state at start/end, toolchain, commit hash, isolated target dirs, wall clock, the sequential-process caveat + the alone-rerun rule).

## Refs

- Issue 208 T7 (this lane), Bench 135 (the run whose invariant-cost finding re-targeted the lever here), Bench 134 (the line-cap registration — the contract's source), Bench 132 (the pregate run), Bench 129 (the query-mass attribution), Bench 125 (the instrument + floors), Issue 191 (the re-campaign sequence), riir-neuron-db Issue 642 (the substrate lever's home).
