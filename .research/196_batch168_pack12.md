# Research 196 — Batch 168: the coverage-report re-arm pack XII

**Status:** RECORD (batch landed 2026-09-21; plan 159, corpus 88→90,
FIXABLE 69→71 — the TENTH consecutive all-FIXABLE pack)

## Derivation (post-Batch-167, develop `1b5a87f3`)

Fresh `cargo heal --coverage-report` (clippy 0.1.98): 88 covered · 0
fix-set-only · 734 uncovered · 421 fire by default · 114 with evidence
(114 via the default-eco histogram — the same-level source). The
dual-evidence head (`missing_errors_doc` 90 local+study, `missing_panics_doc`
78 local+study) and `missing_safety_doc` 186 (default-eco) are the Research
191 doc-authoring skip class — the fix content (prose) is unknowable, the
rewrite space is not mechanical.

## The pack XII leads (probe: `scripts/plan159_pack12_probe.py`)

| lint | group | evidence | verdict |
|---|---|---|---|
| `empty_line_after_doc_comments` | suspicious | 15 (default-eco) | **LANDED** — pure blank-run deletion, zero semantic surface |
| `new_without_default` | style | 14 (default-eco) | **LANDED** — purely additive `impl Default`, clippy's own suggestion shape |
| `match_ref_pats` | style | 17 (default-eco, wave-4 staged) | STAGED — fires, but the rewrite changes binding modes when patterns carry bindings; bounded only under a no-bindings scope (pack XIII candidate) |
| `into_iter_on_ref` | style | 14 (default-eco) | STAGED — bounded only for the parenthesized `(&e).into_iter()` receiver; the receiver-ident-of-ref-type shape needs typeck we do not have |
| `manual_div_ceil` | complexity | 13 (default-eco) | **REJECTED** — `(a+b-1)/b` overflows the intermediate at the extremes while `div_ceil` does not; the rewrite is NOT panic-identity (the `manual_clamp` float-refusal divergence class, construction proposes / semantics dispose) |

### The probe's measured silence (the eld guard, mirrored)

`/// Docs for bar.` + blank + `#[derive(Debug)]` + struct → **clippy never
fires** `empty_line_after_doc_comments` when an attribute intervenes
between the doc block and the item. The proposer mirrors this exactly (the
next non-blank line starting with `#` refuses) rather than fixing a shape
the lint itself stays silent on. A blank run of ANY length fires (triple
blank probed) — one deletion collapses it.

### The nwd design points worth remembering

- The fix is purely ADDITIVE (`impl Default for W { fn default() -> Self {
  Self::new() } }` inserted after the inherent impl, carrying its
  indentation): no existing byte moves, and every pre-existing use of
  `Default` for W was a compile error, so nothing working changes meaning.
- Inherent `default` fns refuse up front — inherent methods shadow trait
  methods for `.default()` calls, the one ambiguity shape.
- `#[derive(Debug, Default)]` detection needs RECURSIVE token-stream
  descent: `Attribute::to_token_stream()` serializes to
  `# [derive (Debug , Default)]` — the idents live inside the bracket
  GROUP, so a top-level iteration sees only `#` + the group and matches
  NOTHING (found by the module test, debugged with a token dump before the
  fix shipped). Whole-ident matching only — `DefaultHasher` never reads as
  `Default`.
- The doc comment on the corpus count test is a TEXT trap: lines starting
  with `+ ` are CommonMark list markers, so every following line is a lazy
  continuation (`doc_lazy_continuation` fires under `--all-targets`, where
  the `#[cfg(test)]` module actually compiles; plain `cargo clippy --lib`
  never sees it — which is why the pre-existing duplicated
  "the 2 Batch 164 rows" line survived its own landing's gate). Rewritten
  without line-leading `+` connectors.

## Freeze regen (the corpus delta — plan 137 T7.5)

- `new_without_default` is a **BAD ATOM BY PREMISE**: the generations
  harness wraps atoms in private `mod gen_N { }` for name collision
  avoidance, the lint requires effective EXPORTATION, so it can NEVER fire
  under the harness — the oracle returns `[use_self]` (nursery, the
  fixture's `W { x: 1 }` constructor body) and never the target lint, in
  any fixture spelling. The `needless_match` bad-atom precedent: k1 +
  every composite containing it excluded (v2 + v3).
- The stride shuffle re-picked windows around the two new sorted atom
  positions: **6 v2 / 6 v3 old exclusion ids went stale** (their windows
  no longer exist — the iter_skip_next+legacy_numeric_constants family
  moved), and the NEW windows re-exposed ep's known `unreadable_literal`
  co-fire inside the eld+ep family (4 v2 rows) plus one genuinely new v3
  interference (`gen3-k3-iter_nth_zero+iter_skip_next+legacy_numeric_constants`
  — a `let_and_return` co-fire, a window that never existed before this
  shuffle). Tables: **92 v2 labels / 15 v2 exclusions** (107 specs at 71
  atoms), **91 v3 labels / 21 v3 exclusions**. Regen example lesson: it
  COLLECTS fresh/stale exclusions and prints them instead of panicking —
  a stride shuffle needs 2 runs to converge (exclude, then re-derive);
  panic-on-first would have landed a half-migrated table.
- Only v2/v3 derive from the clippy corpus (`atoms()`); v4 (rustc error
  codes), v5 (rust_perf corpus), v6–v8 (raw spans) are uncoupled and were
  not touched.
- Both freeze e2es re-run green: v2 31.7 s, v3 20.3 s.

## Gates

G1 module + corpus tests green (23 new: 12 eld + 11 nwd). G2 lib default
2083/0 (floor 2060) + kernel_heal 2436/0 (floor 2413). Retrieval gate
green (17 tests) + the full latent_retrieval,score_bench lib suite 2396/0.
G3 clippy `-D` clean ×2 states. Score-bench #148: heal_rate 96.4% FLAT
(byte-identical to #147's 0.96363634), parse_safety 100%, frontier
snapshot appended (generation 8), vocab_size 71 on the row.
Pre-existing-at-HEAD one-token fix taken where it blocked the G3
all-features lane (`score_bench/mod.rs` doc_markdown backticks — a
`score_bench`-gated module, invisible to the default-features lane).

## Standing pool after this pack

The default-firing evidenced head is now dominated by rows that are either
doc-authoring (skip per Research 191), typeck/architectural (skip), or
already covered. Pack XIII's best candidates are the two STAGED alternates
above (`match_ref_pats` under a no-bindings scope,
`into_iter_on_ref` under a parenthesized-receiver scope) or a fresh
default-eco histogram wave (wave 5) to surface new evidenced rows — the
wave-4 dataset's remaining rows are mostly consumed or single-crate
anecdotes.
