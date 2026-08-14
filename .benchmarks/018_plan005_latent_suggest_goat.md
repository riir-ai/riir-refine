# Bench 018 — Plan 005 Phase 3: latent `--suggest` GOAT (G1-G4)

**Date:** 2026-08-14
**Feature:** `latent_retrieval` (opt-in)
**Hardware:** M3 Max (Apple Silicon), release build
**Tests:** `tests/latent_suggest_goat.rs`

## Result: ALL PASS

| Gate | Target | Measured | Verdict |
|---|---|---|---|
| **G1** retrieval precision (top-1, per domain) | ≥ 80% | **100% (12/12)** | ✅ PASS |
| **G2** end-to-end latency per chunk | < 1 ms | **349 µs** | ✅ PASS (2.9× headroom) |
| **G3** no-regression | lib suite green | **104/104** | ✅ PASS |
| **G4** alloc-free steady state | scratch capacity stable | **stable across 50 repeat scans** | ✅ PASS |

## G1 — fixture + method

12 known anti-patterns (6 clippy + 6 perf), each a **renamed-identifier
variant** of a corpus rule's `buggy_code` (not verbatim — the variant must
retrieve via structural + BM25 similarity, not exact-match). Query the
OWNING domain's index, assert top-1 rule id.

- Clippy: needless_return, redundant_closure, manual_map,
  unnecessary_unwrap, needless_borrow, useless_conversion.
- Perf: o1-lookup, bsearch-sorted-slice, vec-with-capacity,
  format-concat-loop, entry-vs-contains-insert, bool-to-bitpack.

### The first run FAILED (75%, 9/12) — and that was the fixture's fault, not retrieval's

Three misses, all top-1'ing `needless_range_loop`. Fixture audit showed the
three "variants" did NOT instantiate their rules:

1. `redundant_closure` variant used `map(|u| u.to_string())` — a *method*
   closure (that's `redundant_closure_for_method_calls`, a different lint),
   not this rule's `|v| f(v)` → `f` shape.
2. `needless_borrow` variant took `&n` of a *value* — the rule is `&x` where
   `x` is already a reference.
3. `useless_conversion` variant buried the `.into()` in an iterator chain
   instead of the canonical `let y: T = x.into();` form.

Rewritten as faithful instantiations (renamed identifiers only): **12/12**.
Lesson (same class as Issue 669's G1 lesson): a retrieval gate is only as
honest as its fixture — a failing fixture must be audited for rule-faithfulness
BEFORE tuning the retriever.

## G2 — method

240 chunk queries (12 fixture snippets × 20 rounds), each = AstChunker-style
span → embed → centroid route → **fan-out across both enabled domains** →
multi-scale retrieve (whole + per-line) → rank. Mean **349 µs/chunk** in
release. (Bench 575 measured the substrate alone at 123 µs p50 @ N=1000 —
the delta is the multi-scale line queries + the second domain's index +
scratch bookkeeping. All within the 1 ms gate with 2.9× headroom.)

## G4 — method

`SuggestScratch` (retrieved Vec + best HashMap + ranked Vec, shared across
domains and spans): capacities captured after warm-up, then 50 repeat scans
of the full fixture — `retrieved_capacity()` + `ranked_capacity()`
unchanged. Remaining bounded allocations (documented in the struct):
substrate's per-`retrieve()` candidate Vec (≤ latent_k + bm25_k), ≤ 1
rule-id String per distinct retrieved rule, the output clone.

## Known limitations (honest scope)

- Routing is **fan-out + annotation**, not gating (Phase 1 empirical
  finding — centroid mis-routing starved the correct domain). The 8-D
  structural centroids are too coarse to gate; a Phase 3+ tuning option is
  re-gating with tuned directions or the 768-D Bonsai embedder.
- ~~`--suggest` prints suggestions for EVERY span (scores are relative; no
  minimum-score threshold yet).~~ **Threshold calibration attempted +
  REFUTED** — see the addendum below. The every-span noise is inherent to
  text-similarity scoring, not a missing constant.
- G1's per-domain fixture is small (12). The ≥80% bar at N=12 means ≥10
  hits; expanding the fixture tightens the confidence interval.

## Addendum (2026-08-14, same day) — minimum-score threshold: REFUTED

Bench 018's top follow-up ("threshold calibration needs a negative-example
fixture — clean code → no suggestions") was executed the same day. The
negative fixture shipped (14 adversarial clean snippets — the CORRECT version
of each corpus anti-pattern, plus innocent struct/trait shapes) in
`tests/latent_suggest_goat.rs::NEGATIVE_FIXTURE`, with two calibration
tests as the permanent measurement instrument
(`threshold_calibration_distribution` +
`scoring_mode_calibration_whole_vs_deep`; run with `--nocapture`).

### Result: OVERLAP — no threshold exists

| Set | Score range (best-rule, multi-scale) |
|---|---|
| Positives (G1 fixtures, owning-domain top-1) | **[3.19 ..= 34.71]** |
| Negatives (clean adversarial snippets) | **[6.08 ..= 69.37]** |

Specific refutations (adversarial negative ≥ its positive counterpart):

- The **correct** `Vec::with_capacity(n)` fn scored **34.16** on
  `vec-with-capacity` — HIGHER than the true positive's 33.57 (the correct
  sibling shares the whole+push-loop lines; only the allocation line differs
  by one token).
- `bool-to-bitpack` scored **identically** (23.14) on the buggy
  `if *b { 1u8 } else { 0u8 }` and the fixed `*b as u8`.
- The perf domain fires 13–69 on **every** negative (generic Rust token
  overlap: fn/Vec/HashMap/iter) — cross-domain noise is structural.
- An innocent `struct Point {...}` impl block hit **69.37** (`dyn-to-enum`).

A floor keeping all positives (≤3.19) kills **zero** negatives (min 6.08). A
floor killing the worst negatives (≥69) kills **all** positives. The
trade-off frontier is useless at every point.

### Root cause

The fused `latent + BM25` score measures **text similarity, not rule
instantiation**. A rule-bearing token difference (`Vec::new()` vs
`Vec::with_capacity(n)`, `if/else` vs `as`) is invisible to token overlap at
these pattern sizes. The multi-scale accumulation hypothesis was ALSO tested
and refuted: whole-span-only scoring produces near-identical scores (the
top-1 comes from the whole-text query; per-line queries add ~nothing).

### Consequence

- No `MIN_SUGGEST_SCORE` shipped (a constant that fails its own floor gate is
  worse than none). `ScoredRule::score`'s doc now carries the warning.
- `--suggest` output is a **ranked review queue for a human**, exactly as
  documented; `--fix`'s proposer AST visitors remain the ONLY rule-
  instantiation gate (they already see the structural difference the score
  cannot).
- The instrument stays: re-run the calibration whenever the corpus/embedder
  changes, or when prototyping a discriminative successor (e.g.
  distinctive-line coverage, or proposer-gated suggestions).

## Addendum 2 (2026-08-14) — G1 fixture expanded 12 → 24

The known-limitation "12-item G1 fixture" was the weakest leg of the gate.
Expanded with renamed-identifier variants for previously-uncovered rules
(clippy: needless_lifetimes, redundant_field_names, needless_range_loop,
unnecessary_mut_passed — now ALL 10 clippy rules covered; perf: static-str,
dedupe-hashset-vs-vec, ok-or-else, swap-remove, chunks-exact, retain-inplace,
mem-take, avoid-clone — 14 of 46 perf rules covered).

**Result: 23/24 = 96% (gate ≥80% PASS; was 12/12 = 100% on the smaller set).**

The single miss is the predicted confusable pair: the `retain-inplace`
variant's top-1 is `collect-avoidance` (16.25) — both buggy forms are
`iter().…().collect()` chains; the distinguishing feature (collect-avoidance
collects into a NAMED BINDING then re-iterates it vs retain-inplace's direct
filter-collect return) is invisible to the fused latent+BM25 text similarity.
This is the same root cause as the threshold refutation (Addendum 1), now
measured on the retrieval side: near-miss sibling rules within the corpus are
the matcher's real confusion class — cross-rule, not clean-vs-buggy.

Calibration distributions with n=24 positives (unchanged conclusion):
positives [3.193 ..= 34.709] (n=24), negatives [6.081 ..= 69.372] (n=14),
separation: OVERLAP — refutation stands.

G2 re-run (release, expanded fixture): **290 µs/chunk** (480 chunk queries) —
PASS, 3.4× headroom (was 349 µs at n=12; the added snippets are on average
shorter, slightly improving the mean).

Confidence interval context: at 24 items the one-sided 95% lower bound on
precision (Wilson) is ~0.80 — the gate floor — versus ~0.76 at 12 items
(12/12). The expansion therefore buys exact coverage breadth (all clippy
rules) more than statistical tightness; the honest statement is "≥80% with
95% confidence on a 24-item fixture covering all 10 clippy + 14 perf rules".

## Reproduce

```sh
cargo test --features latent_retrieval --test latent_suggest_goat -- --nocapture
cargo test --release --features latent_retrieval --test latent_suggest_goat -- --nocapture   # + G2
```
