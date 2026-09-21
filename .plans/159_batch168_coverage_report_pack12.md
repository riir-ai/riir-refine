# Plan 159 — Batch 168: the coverage-report re-arm pack XII

**Status:** IN PROGRESS (2026-09-21, the 06:11 +07 session; the standing
post-Batch-167 derivation pool — `.distill/001` next-pool pointer)

The fresh `cargo heal --coverage-report` derivation at develop `1b5a87f3`
(clippy 0.1.98, post-Batch-167 corpus 88): 88 covered · 0 fix-set-only ·
**734 uncovered · 421 fire by default · 114 with evidence**. The
dual-evidence head (`missing_errors_doc` 90 / `missing_panics_doc` 78) and
`missing_safety_doc` 186 are the Research 191 doc-authoring skip class. The
probe-rejected rows (`needless_borrowed_reference`, `double_ended_iterator_last`
— silent under the full oracle) and the `len_zero` alias gap
(`comparison_to_empty`, report-only) stay recorded. The mechanical
default-firing candidates probed this pack (probe crate
`.probe/pack12`, real clippy 0.1.98 JSON, 2026-09-21):

| lint | group | joined count | probe | verdict |
|---|---|---|---|---|
| `empty_line_after_doc_comments` | suspicious | 15 | FIRES (blank run, any length; **SILENT when an attr intervenes** — probed) | **LEAD** — pure blank-line deletion, zero semantic surface |
| `new_without_default` | style | 14 | FIRES | **LEAD** — purely additive `impl Default`, clippy's own suggestion shape |
| `match_ref_pats` | style | 17 | FIRES | staged alternate — rewrite changes binding modes when patterns bind; bounded only under a no-bindings scope (pack XIII candidate) |
| `into_iter_on_ref` | style | 14 | FIRES (co-fires `iter_count`) | staged alternate — bounded only for the parenthesized `(&e).into_iter()` receiver; typeck shapes out of reach |
| `manual_div_ceil` | complexity | 13 | FIRES | **REJECT** — `(a+b-1)/b` overflows the intermediate at the extremes while `div_ceil` does not: a semantic divergence (the `manual_clamp` float-refusal precedent), not identity |

- [x] T3 `empty_line_after_doc_comments` proposer (item-level `#[doc]` attrs via
      visit_item/visit_impl_item/visit_trait_item — string-literal `///` and
      statement-position stray docs invisible by construction) + module tests
- [x] T4 `new_without_default` proposer (additive impl; guards: plain-ident type, zero-arg
      `pub fn new() -> Self`, no in-file Default impl / derive(Default) / inherent `default`
      fn, no impl generics; derive check = RECURSIVE token-stream descent — the idents
      live inside the attr's bracket GROUP) + module tests
- [x] T5 registration: proposers/mod.rs doc table + `pub mod` + FIXABLE_LINTS (69→71) +
      dispatch arms + fire-and-parse fixture rows
- [x] T6 corpus.rs entries ×2 + count test 88→90 (+ the count-doc paragraph rewritten
      without line-leading `+` connectors — CommonMark list-marker collision fired
      doc_lazy_continuation under --all-targets; the pre-existing duplicated
      "Batch 164 rows" line in the same block fixed)
- [x] T7 G1: clippy_lints module + corpus tests green (1135 passed)
- [x] T8 tracked probe record `scripts/plan159_pack12_probe.py` — PASS (5 firings + the
      attr-intervened silence)
- [x] T9 freeze regen gen2/gen3 — nwd is a BAD ATOM BY PREMISE (the harness's private
      `mod gen_N` wrap defeats the lint's effective-exportation premise; k1+composites
      excluded v2+v3); the stride shuffle staled 6+6 exclusion ids and re-exposed ep's
      unreadable_literal co-fire in new eld+ep windows; tables 92/15 v2, 91/21 v3;
      e2es PASS ×2 (31.7s / 20.3s)
- [x] T10 G2: lib default 2083/0 (floor 2060) + kernel_heal 2436/0 (floor 2413)
- [x] T11 retrieval gate: latent_retrieval suite 17/17 + full latent_retrieval,score_bench
      lib 2396/0
- [x] T12 G3: clippy -D clean both states (default + all-features; the all-features lane
      surfaced a pre-existing-at-HEAD doc_markdown in score_bench/mod.rs — one-token
      backtick fix taken, it gated the lane)
- [x] T13 score-bench #148 96.4% FLAT (0.96363634, byte-identical to #147) + frontier
      snapshot appended (generation 8) + vocab_size 71 on the row
- [x] T14 Research 196 + AGENTS.md counts (88→90, FIXABLE 69→71, suspicious group noted)
      + .distill/001 next-pool pointer (my-blob staging — sibling WIP in that file)
      (`.docs/02_corpus/seed_corpus.md` is wholesale stale at 53 entries — batches
      150+ never updated it; recorded as a doc-sync unit, not fixed mid-batch)
- [ ] T15 commit `feat: batch 168: ...` + push

