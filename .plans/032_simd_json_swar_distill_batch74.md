# Plan 032 — Batch 74: simd-json SWAR distill (rust_perf external)

Re-probe of the Research 085 §Pool-drops simd-json candidate (the MCP
repo-structure fetch failure was a tool indexing gap — the repo is alive and
directly fetchable). Distill verdict **YES B+** (recorded in Research 088,
same commit):

- Source: `simd-lite/simd-json` v0.18.1 (1.4k stars, 897 commits, MIT/Apache,
  edition 2024, `lto=true codegen-units=1` release profile, criterion benches
  over the classic simdjson datasets — they benchmark seriously).
- Route: **rust_perf** (NOT kernel_opt — portable SWAR arithmetic + a
  dispatch-cost decision; the x86 SSSE3 escalation is a citation note).
- Lead: `swar-digit-chunk-parse` (A, GOAT) — per-digit `val = val*10 +
  (b-b'0')` loop → 8-digit SWAR fold (`parse_eight_digits_swar`: 3×
  and/mul/shift with magic 2561 / 6553601 / 42949672960001) guarded by
  `is_made_of_eight_digits_fast` (the `0x3333` byte-class test), per-digit
  tail. Provenance: Muła 0x80.pl (cited in-source) ← simdjson C++ ← the port.
- Secondary: `runtime-feature-detect-per-chunk` (B+) — per-call
  `is_x86_feature_detected!` inside a per-chunk hot fn costs more than the
  portable fallback for small work units (the in-source measured comment in
  `parse_eight_digits_unrolled`: "cached-atomic load + two outlined
  `target_feature` calls per 8-digit group") → hoist to a cached static or
  gate statically.
- Drops (recorded in Research 088): hex-LUT sentinel fold (niche,
  precompute-table-adjacent), `get_kinda_unchecked` (suggest-unsafe class —
  parse-only healer cannot prove the invariant; `debug-assert-length-hoist`
  is the safe complement, B64), KnownKey memoized hash (requires non-std
  `raw_entry`), tape/two-stage architecture + Eisel-Lemire floats + impls/
  arch-dispatch framework (architectural kills).

Dedup: zero corpus coverage for both candidates (full 78-rule list grepped;
`division-by-invariant-multiplication` is magic-multiply for division — this
is radix folding; the single "SWAR" hit is B67 memchr's doc note about
core's `contains` specialization — a different mechanism application).

## Tasks

- [x] T1: Author the 2 corpus entries (arithmetic + dispatch categories) with
      ORIGINAL minimal fixtures (license hygiene), provenance comments, and
      regime-annotated `context_notes` filled from the measured verdicts.
- [x] T2: Author 2 bench specs (`swar_digit_chunk_parse_bench_spec` — sizes =
      digits-per-run axis [4, 8, 12, 19], N_RUNS fixed; detection spec —
      sizes = work-per-chunk axis, detection-per-chunk vs cached-once).
- [x] T3: G1 exhaustive equivalence gate for the SWAR construction (the
      ported-numeric-algorithm MANDATE): all digit-string lengths 0..=20 ×
      boundary values (all-0s, all-9s, 10^k, u64::MAX string) × LCG digit
      strings × non-digit stop positions at every offset × the
      `is_made_of_eight_digits_fast` validation matrix (8 positions ×
      non-digit byte classes) — bit-identical values + stop positions vs
      the naive reference. (Scoped 0..=19 + the exact u64::MAX 20-digit
      string — the full in-u64 domain.) **Caught a real transcription bug
      pre-landing: `u64::from_le` vs `from_le_bytes` — fixed in all 3 sites.**
- [x] T4: Count pins: `entries.len()` 78→80, fn name
      `seed_corpus_has_seventy_eight_...`→`eighty`, category-count comment
      (arithmetic 6→7, dispatch 2→3), benchable count 68→70 (+ header
      comments — found one batch STALE at 76/66, fixed to 80/70), annotated
      20→22 (both rules carry measured context_notes).
- [x] T5: GOAT — default lib 680/0 + bench-feature lib 698/0, clippy 0 in
      BOTH states (`--all-targets`), retrieval floors HELD zero decay
      (clippy@1 89.6, perf@1 96.2), bench verdicts measured + honest
      annotations (SWAR ParetoDominant 2.35/1.75/2.52 + tie@4; detection
      Mixed 0/0/3 ties), verdict-stability ×2. **Full CI guard ALL LAYERS
      on the exact commit tree** (worktree at origin/develop `0b1992e` +
      patch — the shared tree carries sibling issue-047 WIP; two worktree
      traps re-hit + recorded in Research 088: /tmp breaks sibling paths,
      untracked Cargo.lock re-resolves a drifted graph).
- [x] T6: Research note 088 + queue snapshot entry + AGENTS.md Batch 74 row
      + `.docs/11_domains/rust_perf.md` row. Commit + push develop.

## Non-goals

- Mining `impls/`, `numberparse/{correct,approx}/`, `stage2.rs`,
  `portability.rs` (unread at verdict; architectural/intrinsic kill-class —
  re-scan only if a future candidate needs them).
- A portable-simd (`std::simd`) variant (the source's `portable` feature is
  "currently disabled" upstream — nothing to verify against).
- Proposers for either rule (type-blind shapes — suggestion-only, the
  B73/B65/B59 precedent; the SWAR fix requires knowing the loop parses
  digits semantically).
