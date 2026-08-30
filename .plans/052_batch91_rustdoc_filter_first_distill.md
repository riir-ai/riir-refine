# Plan 052 — Batch 91: the Rustdoc filter-first distill → rust_perf

**Status:** COMPLETE (2026-08-31 — both rules landed ParetoDominant; see the queue snapshot entry for the full GOAT record)

Consumes the 2026-08-29 armed lead (Research 106, `DISTILLED — pending owner
decision`): noahlev.org "Making Rustdoc 33% faster" + rust-lang PRs
159623/159721/159779/159854. The owner's continuation decision ("continue
remains, best perf/sec prod grade") is the dispatch.

## Mine-time source verification

- [x] Article re-read in full (noahlev.org, 2026-08-27) — mechanism claims
      confirmed at article level: the filter was re-expressed on raw `ty`
      data and placed as a guard BEFORE `build_impl`; the post-collection
      filter deleted; over-permissive filtering is correctness-safe ("extra
      impls just slow Rustdoc down but don't affect correctness").
- [x] PR 159623 diff READ (patch-diff.githubusercontent.com) — the guard is
      `impl_opt_trait_ref` + un-normalized `type_of(...).instantiate_identity()`
      (+ `is_doc_notable_trait`, the latent-bug fix); the ENTIRE
      `BadImplStripper` + `add_deref_target` Deref-chain machinery (~100
      lines) deleted; snapshot tests updated (tooltips now appear where they
      were previously missing-by-accident). `build_impl` gained
      `#[instrument(level = "debug")]`.
- [x] PR 159854 (self-type head before `param_env`) — verified at article
      level + footnote 2 (head matching never needed normalization at all);
      diff NOT read (weak-second candidate; article + the note's mapping
      suffice for a B− rule; recorded as a caveat).
- [x] PRs 159721/159779 — KILL/evidence-only per Research 106 (159721 =
      kernel_opt B30/B37 dedup kill with external-validation citation;
      159779 = same mechanism as the GOAT, generality evidence).

## Tasks

- [x] Plan first (this file; `.plans/.highwater` 51 → 52, folder re-scanned
      at write time — 051 is the newest file).
- [x] Write-time dedup grep (both vocabularies: `filter.*map|map.*filter`,
      `projection|canonicaliz|normaliz`, `lazy`) — Research 106's dedup
      evidence re-confirmed against the CURRENT corpus.
- [x] Author rule 1 `filter-before-expensive-transform` (category
      `iterators`, suggestion-only, context_notes regime annotation,
      benchable): `map(expensive).filter(relevant)` → `filter(p).map(f)`
      where the predicate is re-expressible on the RAW input. Guards
      in-rule: two-regime (win ∝ discard-fraction × transform cost; wash
      for cheap transforms / non-selective filters); IMPOSSIBLE when the
      predicate is not re-expressible on the input (the source's own
      precondition). Adjacency to `eager-pointer-collect-lazy-fetch`
      (index-vs-subset, fix space + regime differ) recorded in-entry;
      composition with `filter-map-fuse` (reorder THEN optionally fuse)
      recorded.
- [x] Author rule 2 `decision-projection-before-construction` (category
      `complexity`, suggestion-only, annotated, benchable): the consuming
      decision needs only a cheap PROJECTION of a value (the head of a
      type); compute the projection on the raw input; pay the expensive
      canonicalization/normalization only behind the decision (or for
      survivors — composes with rule 1). DROP CONDITION (authoring-time
      falsification, per Research 106's ~40–50% survival): if the bench
      cannot show a distinct regime from rule 1 (transform-inside-predicate
      vs pipeline-reorder) or the fixture shape collapses into rule 1's,
      drop rule 2 and record the falsification here + in the research note.
- [x] Bench specs ×2 (rules-Vec entry + file-end spec fns with
      `#[allow(clippy::items_after_test_module)]`, the B83/B85/B86
      pattern): opaque `#[inline(never)]` transforms so the discarded work
      is real (LLVM cannot reorder across a call it must assume has side
      effects); exact deterministic alloc axes; top cell 300K (the B73
      sizing lesson — buggy arm at 1M is minutes); rule 1 sizes 1K/30K/300K
      with ~6% keep-rate; rule 2 sizes 1K/30K/300K with 1/16 pass-rate.
- [x] Count-pin walk (the run finds what grep misses — grep FIRST, then the
      full-suite run): corpus `//!` header counts line (94→96 entries,
      complexity 17→18, iterators 9→10, benchable 82→84) + "Currently
      benchable (82/94)" + "Not benchable (12/94)" + the
      context-notes prose ("Thirty-seven rules... as of Batch 86") + the
      benchable/not-benchable bullets + batch-history paragraph + test sites
      (`seed_corpus_has_ninety_four...` → ninety_six 96,
      `eighty_two_rules_have_bench_specs` → eighty_four 84,
      `thirty_seven_annotated...` → thirty_nine 39 ×4 sites incl. serde +
      drafter, `unannotated...57` comment "(94 - 37)" → "(96 - 39)" — the
      57 count itself UNCHANGED by coincidence) + AGENTS.md + `.docs/11_
      domains/rust_perf.md` + any other "94" count references.
- [x] batch91 standing gates: `batch91_rules_present` +
      `batch91_pseudocode_parses` + rule-specific semantic gates:
      `batch91_filter_order_preserves_output` (map-then-filter ≡
      filter-then-map, bit-identical on the fixture),
      `batch91_render_call_count_axis` (render invoked N times in the
      buggy order, keepers-only in the fixed order — the discarded-work
      mechanism pinned as a COUNT, non-vacuous),
      `batch91_projection_invariance` (canonicalize(...).primary ==
      raw-primary over the fixture items — rule 2's precondition AS A
      TEST, plus the canonicalize-call-count axis).
- [x] Fixture purity: both buggy/fixed codes must fire ZERO cross-domain
      proposers (clippy proposers on perf spans — the Issue 056 gate — and
      the switch_cost (fixture,lint) allowlist gate). Design: bare fn paths
      in `map(...)` (no `|x| f(x)` closure — redundant_closure), field-read
      closures, no flops/match/modulo patterns.
- [x] GOAT: G1 parse gates + semantic gates (992/0 default, 1010/0 bench-state, 1091/0 latent, 1136/0/8i triple); G2 lib tests count-verified in
      FOUR states (default / `rust_perf_bench` / `latent_retrieval` /
      `latent_retrieval,fix_verify,clippy_verify`); G3 clippy 0
      `--all-targets` in both feature states; G4 alloc-det stable +
      verdict-stable ×2 bench invocations with box load recorded (GPU
      exclusivity N/A — CPU-only bench; record box state in the research
      note per the discipline).
- [x] Retrieval-precision floors re-measure (`latent_retrieval` regression
      gate incl. cross-fire + clean-span) — a decay FAILS the batch → fix
      retrieval or re-pin with the delta recorded (never silently).
- [x] Independent verification pass (subagent): harness honesty (LLVM
      cannot eliminate the discarded work), fixture purity re-scan, count
      pins, rule-text guard accuracy vs the source claims.
- [x] score-bench run (cadence row; expect flat — the clippy `--fix` path
      is untouched by a rust_perf corpus add).
- [x] Research 106 status: DISTILLED → RECORD with the batch commit hash +
      the measured verdicts + any falsification outcome for rule 2.
- [x] AGENTS.md: rust_perf rule count chain (→ 96) + Batch 91 clause;
      `.docs/11_domains/rust_perf.md` counts + table rows.
- [x] Queue snapshot (`.distill/001_mining_queue_snapshot.md`) — Batch 91
      entry, same commit.
- [x] Commit `feat: batch 91: ...` + push (develop, rebase non-interactive
      only, staged-path guard).

## Honest caveats carried into the entries

- Original-fixture reconstruction (no rustdoc code pasted — license hygiene
  + the binary-only distribution rule); source numbers are
  perf.rust-lang.org END-TO-END pass-level, never cite as this mechanism's
  isolated number.
- PR 159854's diff unread (article-level mapping only) — recorded on rule 2.
- Rule 1 is NOT mechanical (suggestion-only, the `soa-hot-field` class): the
  predicate must be re-expressible on the input type; for cheap transforms
  the rule is churn (annotated).
- The re-expressed filter may be OVER-permissive (the source's own choice) —
  keeping extras is correctness-safe when consumers tolerate them; precision
  may drop, recall must not. In-rule note.
