# Research 068 — Batch 61: oxc measured-lesson distill (keyword-map-to-match + usize-index-to-u32)

**Status:** RECORD
**Date:** 2026-08-18
**Source:** oxc "Pursuit of Performance on Building a JavaScript Compiler"
(https://oxc.rs/docs/learn/performance, originally rustmagazine.org issue-3)
— fetched via idle-queue item 11 (online rust/kernel perf search). The first
batch fed by a FRESH online search after B48/B53/B59 (the prior cycle found
no candidate; this cycle's search hit the oxc article, which is dense with
measured decision-content and honest refutations).
**Plan:** [007](../.plans/007_batch61_oxc_perf_distill.md) — the second
plan-FIRST batch (after Batch 60's retroactive Plan 006).

## What landed (corpus 63 → 65)

1. **`keyword-map-to-match`** (dispatch) — static-keyword dispatch through
   `LazyLock<HashMap<&'static str, Kind>>` → plain `match s { "kw" => ... }`.
   **Bench 61 verdict: ParetoDominant 5.58× @1K, 5.23× @100K** (zero allocs
   both sides, alloc-det stable). The mechanism is the oxc finding: LLVM
   lowers string-literal matches to a switch on string LENGTH + one inline
   compare per length group — oxc tried to beat it with a gperf-style perfect
   hash (V8's approach), failed, then found the length-switch in
   `--emit=llvm-ir` ("the compiler is smarter than us"). The HashMap pays
   SipHash per lookup on short strings. The folk wisdom "string match is O(n)
   comparisons, use a hash map" is refuted for STATIC key sets. Bounded
   fix-space: literal-initialized maps only; dynamic keys keep the map.
   Proposer deferred (key-set extraction from map initializers — the Batch 59
   architectural-follow-up pattern).

2. **`usize-index-to-u32`** (layout) — `usize` index/offset fields in
   high-volume structs → `u32`. **Bench 61 verdict: ParetoDominant 1.99×
   @1K, 1.81× @1M** (zero allocs, stable). The oxc Span change: the AST leaf
   carried 2×usize (16B) for byte offsets that cannot exceed u32; narrowing
   halved the node + measured up to +5% on large files. My bench isolates the
   mechanism (bandwidth-bound sum pass over span arrays): the 8B struct wins
   ~2× even at 1K (both arrays L1-resident) — better than the predicted
   Tie-at-small-N, because the narrower struct packs twice the elements per
   cache line even inside L1. Domain guard: values must genuinely bound below
   2³² (byte offsets, entity indexes; tick counters in long sessions need
   audit). Proposer deferred (type ripple — the Batch 58 caller-gate lesson
   class; suggestion-quality only).

## Dedup verdicts (the oxc candidates that did NOT land)

| oxc lesson | Measured | Verdict |
|---|---|---|
| box enum variants (matklad advice) | +10% | **DUPLICATE** — `box-large-enum-variant` (perf-book types.md) |
| arena AST (bumpalo) | +20% | architectural (lifetime parameterization) — not a bounded fix-space |
| string interning / string-cache lock sharding | +30% parallel | architectural (global-lock redesign) |
| string inlining (compact_str etc.) | large alloc cut | external-crate dep, no in-tree gate |
| token kind/value split | small win | niche; subsumed by the enum-size family |
| `Box<str>` for immutable owned | — | **DUPLICATE** — `box-str-immutable` |
| SIMD whitespace skip | few % | **DUPLICATE** — `simd-chunk` family |
| BufWriter/lock stdout | large | **DUPLICATE** — `buffered-io` + `log-write-direct` |
| jbecker.dev loop-hoisted buffer + clear() | ~30× | **DUPLICATE** — `reuse-allocation-clear` |

The dedup filter confirmed the corpus already covers most of the article —
the two survivors are the two rules with BOTH (a) measured provenance AND
(b) a bounded fix-space not already claimed.

## GOAT

- **G1 (correctness):** both buggy/fixed snippet pairs parse (syn) — the
  corpus-truthfulness tests pass; the keyword bench harness initially FAILED
  compile (`match {...} as u64` needs parens — fixed by binding `let k =
  match ...; k as u64`; the corpus snippets were never affected).
- **G2 (Pareto bench):** `PERF_BENCH_FILTER=keyword-map,usize-index cargo run
  --example perf_bench_demo --features rust_perf_bench --release` — both
  **ParetoDominant**, 0 allocs buggy+fixed at all sizes, alloc-det stable
  across 3 runs.
- **G3 (no-regression):** 595 default lib tests / 0 failed; clippy 0 in
  default + `rust_perf_bench` states (`--lib`). Pre-existing note: 31
  test-target warnings exist under `--all-targets` in Batch 59's
  division-test code (long literals, u128 casts, ilog2, unused-mut) — present
  before this batch, out of scope, recorded here so the next session doesn't
  attribute them to Batch 61.
- **Retrieval floors:** the Issue 017 regression gate PASSED (12/12 retrieval
  tests) — perf@1 94.3%, blend@1 92.1% (structural) — floors held with the
  2 new rules; both surface in clean-span suggestions (expected: no
  minimum-score threshold is a documented known limit) without crowding
  fixtures.

## Count pins updated

- `seed_corpus_has_sixty_five_entries_across_twelve_categories` (63 → 65;
  renamed), `unannotated_rules_have_no_context_notes` (51 → 53), the drafter
  propagation test's unannotated count (51 → 53). Annotated stays 12 — both
  new rules measured ParetoDominant, no annotations.
- Header docs: entry/category/benchable counts + Batch 61 provenance; also
  fixed TWO STALE counts found in passing (the header paragraph said "Ten
  rules" + omitted Batch 53's 2 annotations while the test pinned 12; the
  `build_perf_seed_corpus` doc comment said "58 entries" at a 63-rule corpus).

## Generator lessons (twice-struck, now three times)

1. **The B37 line-initial `+` lesson struck AGAIN** (5th workspace
   occurrence, 1st for this agent): rewrapping the header paragraph moved
   `+ bounded-channel-backpressure` to a line-initial `+ ` — parses as a new
   markdown list item, flagging every subsequent line (`doc list item without
   indentation` × 3). Fix = keep the `+` at the END of the previous line.
2. **edit_file fuzzy matching can silently DROP a character from a raw-string
   terminator**: my `old_text`/`new_text` wrote `}"` where the file had
   `}"#` — the fuzzy matcher accepted the near-match and removed the `#`,
   un-terminating an UNRELATED r#-string 100 lines away (the division
   harness swallowed the new keyword fn → E0425 + phantom E0308s). The B37
   family's sibling: raw-string `#` terminators are load-bearing characters;
   when editing near `}"#` boundaries, re-read the file state after every
   edit before compiling.
3. **`match {...} as u64` does not parse** (statement-position match + cast
   needs parens or a let-binding) — caught by the bench temp-crate compile,
   not by the syn corpus parse (the corpus snippets returned `Kind` directly).

## Batch 61 ledger

- Corpus: 63 → 65 rust_perf (kernel stays 364).
- Categories: layout 5 → 6, dispatch 1 → 2.
- Annotations: 12 (unchanged — both new rules ParetoDominant).
- Tests: 595 default lib (was 577 at Batch 60; includes sibling additions
  + the renamed count test).
