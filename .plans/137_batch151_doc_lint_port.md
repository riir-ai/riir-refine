# Plan 137 — Batch 151: the doc-line state-machine port (`doc_lazy_continuation` + `doc_overindented_list_items`)

**Status:** LANDED (2026-09-17)

The staged follow-up from Research 177 adjudication #7: both doc lints probed
against real clippy 0.1.98, authoring unblocked. One proposer module houses
both (they share ONE state machine — clippy's own `doc/lazy_continuation.rs`
checks both from one file; the `print_write_with_newline` one-module-two-names
precedent).

## Design input — the probe151 boundary matrix (this plan's T1, DONE first)

Fresh scratch crate `.raw/probe151/` (real clippy 0.1.98, JSON spans) closing
the gaps Research 177 left: exact suggestion spans/bytes + the content-column
math for `/// `-style docs.

Measured (all fixtures fn-wrapped, ` - item` list marker, conventional space
counted as leading — the cooked doc text is the attr value verbatim):

| fixture (doc text) | leading | content col | result | suggestion |
|---|---|---|---|---|
| `   two-space cont` after ` - item` | 3 | 3 | silent (exact cc) | — |
| `    three-space cont` | 4 | 3 | overindent | replace indent-run → 3 spaces, MaybeIncorrect |
| `     four-space cont` | 5 | 3 | overindent | same |
| `      six-space cont` | 6 | 3 | overindent | same |
| `       seven-space cont` (+4 = CommonMark code territory) | 7 | 3 | **overindent FIRES** | same — no code-block protection (pulldown reads it as paragraph text; rustdoc renders it as text, so the dedent is render-stable in rustdoc) |
| ` zero-indent cont` | 1 | 3 | lazy-list | insert 2 spaces at text start, MaybeIncorrect |
| ` lazy line` after `> quote line` | 1 | — | lazy-quote | insert `> ` at text start, **MachineApplicable** |
| blank-separated `after blank` | 1 | — | silent ✓ | — |
| fence block inside item + `tail` after | — | — | **all silent** (fence breaks paragraph continuity; the item ends) | — |
| `   two-space cont` after ` 1. numbered` | 3 | 4 | lazy-list | insert 1 space ✓ deficit math exact |
| ` # heading after` item | 1 | 3 | silent (block start) | — |
| `tail after second` after `- second` | 1 | 3 | lazy-list (container persists across consecutive items) | insert 2 |
| `lazy after q2` after `> q1`/`> q2` | 1 | — | lazy-quote, fires once | insert `> ` |
| ` not indented` after nested `   - nested`/`     deep` (cc 5) | 1 | 5 | lazy-list | insert 4 ✓ top-of-stack cc |

Net-effect byte-parity: all three fix shapes produce byte-identical output to
clippy's own suggestions (verified by span arithmetic on the JSON).

## Tasks

- [x] T1 — probe151 boundary matrix (above); scratch crate cleaned
- [x] T2 — extract the shared doc-line walk (`helpers::doc_lines`: the
      doc_markdown Collector + sorted adjacency walk) and refactor
      doc_markdown::collect_edits onto it (behavior pinned by its tests)
- [x] T3 — author `proposers/doc_lazy_continuation.rs`: the line state
      machine (fence / quote-depth / list-stack / paragraph-open), both
      entry points (`lazy_edits`, `overindent_edits`), all guards from the
      matrix; module tests = the full fire+silence matrix per P11 + fixpoint
- [x] T4 — wire mod.rs: module decl, 2 dispatch arms, FIXABLE_LINTS += 2
      (47), context_note entries for both, count pins walked
- [x] T5 — corpus.rs: 2 entries (quote shape + overindent shape, doc-flavored
      vocab for centroid defense), header doc 62→64, `corpus_size_is_64`,
      provenance list extended
- [x] T6 — AGENTS.md counts 62→64 / 45→47
- [x] T7 — gates: lib default all-green (incl. lockstep), latent_retrieval
      all-green, all-features check, clippy --all-targets clean on the write
      set, score-bench (append rows), `--corpus-stats` 64, `--coverage-report`
      covered 62→64 / fix-set-only 0 / evidenced-uncovered 71→69
- [x] T8 — HISTORY.md batch entry; Research 177 §7 pointer; `.distill/001`
      batch-state update; commit + push
- [x] T9 — global binary rebuild (release feature set, isolated worktree,
      symlink-verified) + scratch cleanup
