# Plan 010 — Batch 64: the ARM Neon / CPU-SIMD / bounds-check pool (2 online-search candidates)

**Created:** 2026-08-19 (plan-first — BEFORE any corpus edit)
**Source:** the 2026-08-18 queue-item-11 online perf-search pool (recorded in `.distill/001_mining_queue_snapshot.md` with full provenance + dedup analysis + caveats). Both candidates verified ABSENT from the rust_perf list at sweep time; re-verify against the CURRENT 67-rule list at authoring time (Batch 62 added 2 since the sweep).

## Candidates

1. **`loop-order-innermost-contiguous`** (simd; STRONG) — i-j-k nested accumulation where the innermost loop strides the OUTER axis of a 2D access (`b[k][j]` at stride-n) → reorder to i-k-j so the innermost varies the LAST index (contiguous row walk). Source: Arun Nagaraj matmul-simd (9× alone, L1 misses 45%→12%, 0.47→5 GFLOPS; memory-hierarchy not ISA). **Scope: non-repacked shapes** (B62's `transposed-weight-register-gemv` is the repack route — this is the no-repack general nested-accumulation case; distinct-from clause required). Caveats: float sum-order changes → rel-error tolerance gate (NOT bit-identity); legal only without loop-carried deps across swapped axes (the `c[i][j] +=` canonical case).
2. **`debug-assert-length-hoist`** (complexity; MEDIUM, annotation-risk) — `debug_assert!` mentioning `.len()` is ERASED in release, dropping exactly the length constraint the optimizer needed to elide bounds checks → release RETAINS all of them ("Ironically, this makes the code slower") → swap to `assert!` (redundant asserts are optimizer-removed). Source: Shnatsel bounds-check-cookbook (the canonical article). Honest expectation: 1.03×-3% class on aarch64 (the 15% headline was an x86 unrolling secondary effect) — may land Mixed/annotated; the rule's VALUE is stopping the counter-intuitive release pessimization. Encodes the sibling fact: an `assert!` before the hot loop is the cheapest bounds-check hoist.

## Phases

### Phase 0 — dedup re-verification
- [x] Re-grep — DONE (both absent from the 67-rule list; B62's lead landed with GEMV-repack scoping — the loop-reorder axis confirmed distinct). (both vocabularies); re-read the B62 lead rule's final text (it LANDED — confirm the no-repack scoping is distinct); re-read `simd-chunk`/`layout-reorder`/`soa-hot-field` for the distinct-from clauses.

### Phase 1 — entries (corpus.rs; category: simd + complexity)
- [x] Rule 1 — DONE. Measured ParetoDominant 7.14x/11.32x/13.80x (BETTER than the source's 9x; grows with n). buggy = i-j-k nest with column-striding inner access; fixed = i-k-j reorder + hoisted row anchor. context_notes: None unless bench says Mixed.
- [x] Rule 2 — DONE, annotated with the measured regime-split (1.13x@1K win / stable 0.67-0.71x@100K loss on aarch64) + the data-dependent-index scope finding (asserts can never propagate through runtime index values — pure cost there, measured 0.74-0.80x on the first harness shape). buggy = debug_assert!(idx < v.len()) + release indexing; fixed = assert!. context_notes: the release-erasure mechanism + expected small gains + the loop-front assert sibling fact (annotation expected).

### Phase 2 — bench specs + GOAT
- [x] Rule 1 bench — DONE (rel-error tolerance honored; L1-resident sizes). nested-accumulation matmul at sizes where L1 traffic dominates (e.g. 64/128/256 square) — buggy i-j-k vs fixed i-k-j; rel-error tolerance (sum-order changes); expect ParetoDominant at the L1-bound sizes.
- [x] Rule 2 bench — DONE (canonical range-pinned shape after the first harness was refuted; 3-run stability on the 0.67x). release-mode indexing loop with debug_assert vs assert — NOTE: the bench harness runs in RELEASE (the whole point — debug_assert is erased there); expect 1.0x-1.05x; honest Mixed-annotation landing if so.
- [x] GOAT — ALL PASS (corpus 15/15 bench-state + 607 default + 659 latent + floors HELD + clippy 0 all-targets both states). G2 alloc-det stable ×3; G3 full lib suites (default + rust_perf_bench + latent_retrieval — INCLUDING the bench-feature lib suite, the Batch-61 lesson) + clippy 0 all-targets; G4 no new deps; retrieval floors re-run.

### Phase 3 — pins + records
- [x] Count pins — DONE (69 everywhere; benchable 59; simd 7 + complexity 15; annotated 14; the header's stale '65 entries' + 'simd has 5' fixed in passing — B62 had missed them). (size test rename, benchable 57→59, category counts simd 6→7 + complexity 14→15, annotated 13→14 if Rule 2 annotates, unannotated, serde, drafter).
- [x] Records — Research 072 + AGENTS.md Batch 64 entry + .docs + queue snapshot (pool -> DONE) in the landing commit. + AGENTS.md Batch 64 entry + `.docs/11_domains/rust_perf.md` + mining queue snapshot (pool → DONE). `.research/.highwater` 71→72.
- [x] Commit + push on develop.: ...` + push on `develop`.

## Non-goals
- The pool's recorded DROPS stay dropped (cache blocking — architectural; `& mask` bounds — correctness-first excludes; iterator retrofit — style; `#[inline(always)]` — cross-function; SME — no hardware).
