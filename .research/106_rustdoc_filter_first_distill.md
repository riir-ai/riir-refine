# Research 106 — Distill verdict: noahlev.org "How I made Rustdoc 33% faster" → rust_perf

**Status:** DISTILLED — pending owner decision

- **Source:** <https://noahlev.org/blog/2026/08/27/making-rustdoc-faster/> + the four linked rust-lang PRs: [#159623](https://github.com/rust-lang/rust/pull/159623) (filter-first, 20%), [#159721](https://github.com/rust-lang/rust/pull/159721) (local-primitives gate, 12%), [#159779](https://github.com/rust-lang/rust/pull/159779) (documented-types-only synthesis, 6%), [#159854](https://github.com/rust-lang/rust/pull/159854) (self-type head before `param_env`, 18%).
- **Inspection date:** 2026-08-29 (article fully read; PR diffs NOT read — unverified, re-read at mine time).
- **Verdict:** **YES, grade B** — one batch, lead with `filter-before-expensive-transform`.
- **Domain routing:** `rust_perf` — categories `iterators` (GOAT) + `complexity` (weak second). NOT `kernel_opt` (zero GPU content; this is host-side tooling), NOT `clippy_lints` (no parse-only lint shape — the rewrites are semantic).
- **Aggregate evidence:** 25% average wall-time reduction = 33% speedup across the rustc benchmark suite, up to 40% on real crates (hyper, bitmaps), 60% on microbenchmarks; max RSS −12%. Measured on perf.rust-lang.org, first-party author (Rustdoc team member), title corrected by nnethercote (whose perf-book the corpus already mines — same lineage, B48/B53 sources).

## Provenance chain

First-party engineering blog post (not a port; the mechanisms are described at the PR level with before/after flamegraphs). The four PRs are the primary artifacts. Credibility: high — the numbers are cross-checked against rust-lang's public perf infrastructure, and the author's process (profile → find dominant pass → question it → measure each PR separately) mirrors our own GOAT discipline.

## The four PRs → candidate mapping

| # | PR | What changed | Measured | Candidate grade |
|---|---|---|---|---|
| 1 | 159623 | `build_extern_trait_impls` called expensive `build_impl` for EVERY trait impl in the entire dependency graph, THEN filtered. Fix: adapted the relevance filter to work on raw `rustc_middle::ty` data and placed it BEFORE `build_impl`; deleted the post-collection filter + the Deref-chain follow code | −20% wall, −12% RSS | **A−** — the GOAT |
| 2 | 159721 | primitive-impl inlining pass ran in EVERY crate even when the crate declares no `#[rustc_doc_primitive]` (nearly all don't). Fix: run only on local primitives | −12% wall, −6% RSS | KILL (dedup — see below) |
| 3 | 159779 | synthetic-impl synthesis (auto traits + blanket impls) analyzed EVERY type incl. private types that never appear in docs. Fix: filter to documented types first | −6% wall (13% hyper) | Same mechanism as #1 — evidence for the rule's generality, not a second rule |
| 4 | 159854 | `param_env` (compute + normalize where clauses) was 50% of the pass — invoked on every impl. Fix: the inlining decision needs only the HEAD of the self type (`Vec` in `Vec<i32>`); skip full normalization unless the head requires it | −18% wall | **B−** — weak second |

Also in PR 1: deleting the Deref-chain code exposed + fixed a latent `#[doc(notable_trait)]` bug (the tooltip passed by accident through a `Vec<T> → &[T]` deref). Not a perf pattern — dropped.

## Ranked candidate table

| Pattern | Source | Grade | Disposition |
|---|---|---|---|
| **`filter-before-expensive-transform`** — per-item EXPENSIVE construction runs before a relevance filter that discards most items; fix = re-express the predicate on the RAW input type and filter first (`map(f).filter(g)` → `filter(p).map(f)` where f is opaque/expensive). Two-regime: win ∝ discard-fraction × transform cost; wash when the transform is cheap (LLVM inlines+CSEs) or the filter is non-selective; IMPOSSIBLE when the predicate is not re-expressible on the input (the mapped value carries non-derivable information) — that precondition is load-bearing and is exactly what the source did ("adapting an overly permissive version of the filtering logic to work on Rustc's raw ty data structures") | PR 159623 + 159779 | **A−** | **KEEP — GOAT candidate** |
| decision-projection-before-construction — the consuming decision needs only a cheap projection of a value (the head of a type); defer the expensive canonicalization/normalization until the cheap projection proves insufficient | PR 159854 | B− | KEEP-WEAK — pipeline-level generalization of the laziness family (`ok-or-else`/`unwrap-or-else-lazy` are single-value; `eager-pointer-collect-lazy-fetch` is index-vs-subset); architectural, suggestion-only, ~40–50% survival at falsification |
| skip-the-pass-when-consumer-set-empty — an expensive pass gated on a cheap presence check of the only thing that makes it relevant | PR 159721 | — | **KILL as a new rule** — same decision content as kernel_opt B30 `predicate-gated-skip-empty-workset-step` (one host-side integer check elides the entire step batch, `entries_gpu_03.rs` L177) + B37 `lm-head-loss-positions-only` (dispatch gated to the consumer-relevant subset, `entries_gpu_04.rs` L874). Cite as external validation on B30 at mine time (the opponent-watch ADJACENT-COVERED pattern) |
| dead-code deletion exposes latent bug (Deref chain → notable-trait tooltip) | PR 159623 | — | DROP — code-hygiene anecdote, no bounded fix-space |

## Dedup gate evidence (both vocabularies, 2026-08-29)

- `filter-map-fuse` (B48, `rust_perf/corpus.rs` L1305–1323): fuses `filter(p).map(f)` → `filter_map`. The buggy shape there is ALREADY filter-first — fusion-of-adapters, NOT ordering-of-expense. The candidate is the opposite ordering (`map(expensive).filter(g)` → reorder). Zero corpus hits for map-before-filter reordering under `filter.*map|map.*filter` vocabulary. Novel; the two COMPOSE (reorder, then optionally fuse).
- `filter-count-to-any` (B48): short-circuit vs full scan — different decision content.
- `eager-pointer-collect-lazy-fetch` (B76/Meilisearch, caching): eager INDEX MATERIALIZATION to serve subset READS; fix = direct/lazy access. Adjacent (both are "pay O(all) to use O(subset)") but the fix space differs (access path vs pipeline reordering) and the regime differs (K/N access pattern vs selectivity × transform cost). Record the adjacency in the entry, keep separate — the corpus's own precedent for same-family rules with distinct fix spaces (`debug-stat` vs `sort-key-cache`; `eager-pointer` vs `collect-avoidance`).
- `ok-or-else` / `unwrap-or-else-lazy` (B48): single-value laziness behind Option — not pipeline-level.
- kernel_opt B30/B37 (grep `workset` project-wide): cover the "gate the pass" candidate completely → KILL with external-validation citation.

## GOAT candidate test — `filter-before-expensive-transform`

1. **Benchable:** yes — perf-bench Pareto validator. N items of a small raw struct; buggy arm maps every item through an opaque/expensive transform (e.g. `#[inline(never)]` fold or an allocation), then filters (keep ~6%); fixed arm filters on the raw field first, then maps. Sizes 1K/100K/1M.
2. **Exact semantics:** integer math → bit-identical trivially (filter-then-map preserves sequence order for pure f/p).
3. **Magnitude story:** the discarded elements' transform work is skipped ENTIRELY — win ∝ (1 − keep-rate) × transform cost. Mechanism-level (a discarded 50 ns/op × 94% of N), not "probably faster". Expect 10×+ at 94% discard with an opaque transform.
4. **Deterministic:** yes.

## Honest caveats

- The source is a narrative blog about rustc-internal passes; rules must be reconstructed as ORIGINAL minimal generic fixtures (the Meilisearch/Greptime/polars precedent). Never paste rustdoc code (license hygiene + binary-only rule).
- PR diffs unread (article-level description only) — mark "re-read at mine time"; the PR links are in the header.
- The GOAT rewrite is NOT mechanical — the predicate must be re-expressible on the raw input type. Suggestion-only (the `soa-hot-field` class). This is also the honest scope limit: for cheap transforms the rule is churn (annotate).
- LLVM can flatten the demo if the transform inlines and CSEs — the bench must use an opaque call or an allocation so the discarded work is real.
- Batch timing: lands as Batch 89 when a slot opens; `rust_perf/corpus.rs` state must be re-scanned at write time (the B85/B86 same-night dual-alloc precedent — header says 94 rules after B86, sec Phase 2 (B88) landed 2026-08-29 morning and touched the `sec` domain, not `rust_perf`).

## Queue slot

Batch 89 candidate — lead `filter-before-expensive-transform` (iterators), optional second `decision-projection-before-construction` (complexity) if it survives authoring-time falsification against the laziness family. Verdict recorded 2026-08-29; mining awaits a batch slot + quiet-box window.
