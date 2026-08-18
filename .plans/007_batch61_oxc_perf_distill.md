# Plan 007 — Batch 61: oxc measured-lesson distill (str-match-vs-hashmap + usize→u32 narrowing)

**Status:** COMPLETE (2026-08-18)
**Source:** oxc "Pursuit of Performance on Building a JavaScript Compiler"
(https://oxc.rs/docs/learn/performance, originally rustmagazine.org issue-3) —
fetched 2026-08-18 via queue item 11 (online rust/kernel perf search).
**Feeding path:** the first batch fed by a fresh online search after B48/B53/B59
(the prior cycle found no measured-GOAT candidate; this cycle's search hit the
oxc article, which is dense with measured decision-content).

## Why these two rules (and why the other oxc candidates were dropped)

| oxc lesson | Measured | Verdict |
|---|---|---|
| static keyword `match` — LLVM lowers to length-switch | tried perfect hash, could NOT beat the match; IR shows `switch i64 %s.1` (length) | **RULE 1** — `keyword-map-to-match` |
| Span `usize`→`u32` | up to **+5% on large files** (halves node size) | **RULE 2** — `usize-index-to-u32` |
| box enum variants (matklad advice) | +10% | **DUPLICATE** — `box-large-enum-variant` already in corpus (perf-book types.md) |
| arena AST (bumpalo) | +20% | architectural (lifetime parameterization) — not a bounded fix-space |
| string interning / string-cache lock sharding | +30% parallel | architectural (global-lock redesign) |
| string inlining (compact_str etc.) | large alloc cut | external-crate dep, no in-tree gate (fxhashmap precedent requires an existing dep) |
| token kind/value split | small win | niche; subsumed by the enum-size family |
| `Box<str>` for immutable owned | — | **DUPLICATE** — `box-str-immutable` already in corpus |
| SIMD whitespace skip | few % | **DUPLICATE** — `simd-chunk` family |
| BufWriter/lock stdout | large | **DUPLICATE** — `buffered-io` + `log-write-direct` |
| jbecker.dev loop-hoisted buffer + clear() | ~30× | **DUPLICATE** — `reuse-allocation-clear` (Mixed-annotated) |

## Rules

1. **`keyword-map-to-match`** (dispatch) — static-keyword dispatch through
   `LazyLock<HashMap<&'static str, Kind>>` → plain `match s { "kw" => .. }`.
   LLVM lowers string-literal matches to a switch on length + inline compares;
   the HashMap pays SipHash per lookup. oxc provenance: they tried a gperf-style
   perfect hash (V8's approach) and could not beat the compiler — the IR shows
   the length switch. Bench: match vs HashMap<&str, Kind> over a mixed
   identifier workload. Expected: match dominates.
2. **`usize-index-to-u32`** (layout) — index/offset fields as `usize` in
   high-volume structs → `u32`. oxc Span: 2×usize → 2×u32, halves the leaf
   node, +5% on large files (cache density). Bench: sum spans over arrays that
   exceed L2 (16B vs 8B per element). Expected: Faster at large N, Tie small N
   → honest regime annotation if Mixed.

Proposers: **deferred** for both (Batch 59 precedent — rule 1 needs key-set
extraction from literal map initializers; rule 2 is a type ripple, the Batch 58
caller-gate lesson class).

## Tasks

- [x] Author corpus entries (2 rules) + 2 bench specs in `corpus.rs`
- [x] Pareto bench run (`rust_perf_bench`) → verdicts + `context_notes`
      annotations on regime-fragile/Mixed results (both ParetoDominant — no
      annotations needed)
- [x] Count pins: corpus-count assertions + annotated-count test updated
      (if annotations land) (size test 63→65 renamed; unannotated 51→53;
      annotated stays 12)
- [x] G1: buggy/fixed snippets parse (syn) — corpus-truthfulness style checks
- [x] G2: lib tests pass (default + feature states) — 595/0 default
- [x] G3: clippy 0 in both states (also fixed 2 stale header counts found in
      passing: "Ten rules"→Twelve + "58 entries"→65)
- [x] Retrieval floors: run the Issue 017 regression gate
      (`latent_retrieval` + rust_perf corpus); re-pin + record delta if shifted
      (12/12 PASSED — floors held, no re-pin)
- [x] Research note `.research/068_batch61_oxc_perf_distill.md` (Status: RECORD)
- [x] AGENTS.md: Batch 61 entry + rust_perf count 63 → 65
- [x] `.docs/11_domains/rust_perf.md` rule table update
- [x] SKILL.md mining-queue snapshot update (same commit)
- [ ] Commit `feat: batch 61: oxc distill — keyword-map-to-match + usize-index-to-u32 (2 rules, corpus 63→65)` + push
