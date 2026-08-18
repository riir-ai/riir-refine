# Plan 011 — Batch 66: polars kernel-idiom distill (rust_perf)

> Executes Research 073 (`.research/073_batch66_polars_kernel_idioms_pool.md`) — the
> polars targeted-file distill verdict. Plan-first per the standing discipline
> (adopted Batch 60): created BEFORE any corpus edit.

## Scope

Land the two coordinator-verified co-leads from the Research 073 pool + the simd-chunk
enhancement rider. The B-tier candidates (transmute-map / funnel / grouping) are
evaluated during the batch but expected to defer — detection-risk or kill-risk noted
in the pool.

- [x] T1 — Dedup re-verification at write time (fixture BODIES grep — the Research 073
      lesson: a rule whose fixture IS the candidate's shape is coverage). Re-grep
      `buggy_code`/`fixed_code` bodies for: bit-iteration, sortedness, blsr, trailing_zeros,
      early-exit validation shapes.
- [x] T2 — Corpus entries (2 rules):
      `sparse-set-bit-iteration-tz-blsr` (category: arithmetic — the bit-trick family
      beside `division-by-invariant-multiplication`; if corpus convention says simd
      beside bool-to-bitpack, follow the family grep at write time) +
      `sortedness-check-branchless-accumulate` (follow `filter-count-to-any`'s category).
      ORIGINAL minimal fixtures (MIT hygiene — never polars code).
- [x] T3 — Bench specs for both (the `bool_to_bitpack_bench_spec` family shape):
      tz-blsr — u64 masks at densities {1, 8, 16, 64} × 1K masks; density 64 = the
      honest wash arm (regime boundary). sortedness — u64 arrays (256K elems) sorted
      (win arm) + violation-at-front (honest lose arm → context_notes regime split,
      the `filter-count-to-any` annotation precedent).
- [x] T4 — simd-chunk enhancement rider: fold polars' float_sum mechanism content into
      simd-chunk's comment block (N-lane independent accumulators beat the serial FP
      chain BECAUSE strict-FP forbids reassociation — not auto-vectorization of the
      serial form; the tree-reduce finisher note). NO new rule (killed at coordination).
- [x] T5 — Count-pin updates: grep every pin site that references the rust_perf corpus
      count (69 → 71) or benchable count — the 18-count-pin-site lesson; find them via
      `grep -rn "69\|65 rules\|benchable" tests/ src/` + the corpus header doc-comment.
      (FOUND 21 total — 2 NEW sites the grep pattern missed: the unannotated/annotated
      count pins in `drafter_propagates_context_notes` + `unannotated_rules_have_no_context_notes`.)
- [x] T6 — GOAT: G1 fixtures fire + fixed compiles; G2 Pareto bench (single-threaded,
      load-recorded — p335 active, note in the record); G3 lib tests default +
      rust_perf_bench + latent_retrieval states; G4 alloc-count axis (neither rule
      allocates); retrieval floors HELD.
- [x] T7 — Clippy 0 in touched files (the //! backtick gotchas: B37 line-initial `+`,
      `.rs` filenames, `O(...)` expressions).
- [x] T8 — Docs: AGENTS.md Batch 66 entry; queue snapshot Batch 66 pool → DONE;
      Research 073 Status → RECORD (batch landed). No commit to polars (external source).
- [x] T9 — Commit + push (`feat:` prefix).
- [-] T3-note — the density-sweep arm (densities {1,8,16,64}) was simplified to the
      single ~4/64 sparse regime (the applicable window) + the convergence prose in
      the spec doc; a full density sweep is harness depth a future re-gate can add.

## Non-goals

- The memchr rule (Batch 67, separate pool/plan).
- Proposers for the new rules (proposer work is per-rule deferred by default; both new
  rules are detection-subtle — `[-]` defer with the standing backlog note).
- The B-tier candidates (transmute/funnel/grouping) — evaluated, expected defer.
