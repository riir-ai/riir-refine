# Research 262 — Batch 231: the P52+P53 conversion (llama.cpp #30405 + #30447)

**Status:** RECORD

Claim `68be73c3` (m3 idle cycle 75) → plan 233 → this note. Two graded intake
rows (P52 B+/B, P53 B) converted per Protocol A. Sources: ggml-org/llama.cpp
@ `b9f3599fb1c429e89248f042699ea2e062a32e57` (#30405, MIT, merged 2026-10-13)
+ `0a731cced9ed826349838333a767ccae63164c2c` (#30447, MIT). Every quote
verified verbatim at the fetched `.patch` files this session
(`/tmp/b231_p52.patch`, `/tmp/b231_p53.patch` — deleted with the landing).

## What landed

1. **`host-pointer-buffer-foreign-range-subtraction`** (kernel_opt, category
   A, entries_gpu_90.rs — the batch-227 convention: new rules join the latest
   slice; A 164 → 165). THE LAW: a backend buffer created from a host pointer
   pins everything it wraps; when several contexts share one mapped file
   (lazy tensors, `-cmoe` CPU-resident experts), the single `[first, last)`
   span also wraps the ranges the OTHER contexts own — pinning data the
   backend never reads (27 GiB for Qwen3.8-Flash-Next IQ4_NL per-layer token
   embeddings, defeating the point of reading them lazily). The repair:
   merge own ranges − merge foreign ranges (`merge_ranges` sort+coalesce,
   `sub_ranges` walk-and-cut, both sorted non-overlapping), then ONE
   `buffer_from_host_ptr` PER disjoint owned range; the shared-weight
   carve-out is an own-set membership test (`token_embd.weight` is never
   foreign); the buffer container changed from an idx-keyed map to a LIST —
   "the tensors of a file can be spread across several buffers - find the one
   containing this tensor's data" (a containment search at load).
2. **`madvise-random-for-key-gather-tables` += the macOS WILLNEED-PREFETCH
   NO-OP face** (rust_perf, entries_15.rs context_notes — count-neutral).
   The prefetch guard `#if defined(__linux__) || (defined(_WIN32) && ...)` 
   silently took the #else on macOS; the repair is ONE platform constant
   (`+defined(__APPLE__)`); measured (source-reported, cold file cache)
   356/375/475 → 735/749/788 t/s at pp512/2048/4096 — ~2× pp from making the
   EXISTING hint actually execute. The named class: a platform-guard whose
   ELSE branch is a silent no-op is the green-zero shape one layer down (the
   arm exists, runs, reaches nothing). The in-#else TODO records the
   follow-up (`posix_madvise(.., POSIX_MADV_WILLNEED)`).
3. **`lut-free-bit-spread-vadd4-unpack` += the x86 SIMD face of the CPU 2-bit
   crumb spread** (kernel_opt, entries_gpu_33.rs gpu_concept — count-neutral,
   the FOURTH ISA spelling). Three mechanisms, quotes verbatim: the
   OR-NOT-ADD spread ("byte b becomes b & 3, (b >> 2) & 3, (b >> 4) & 3,
   (b >> 6) & 3 (OR, never add: the overlapping copies only touch masked-off
   bits)" — cvtepu8_epi32 + or(slli 12) + or(slli 6), AND 0x03030303); the
   maddubs algebra ("sum((v - 1) * y) = sum(v * y) - sum(y), exact for every
   int8 y" — one constant register vs a subtract pass, working WITHIN the
   u8×s8 constraint B181's REGIME 2 names); two blocks per step "each into
   its own accumulator (two FMA chains)". Also in-commit: arch-fallback.h's
   x86 generic alias DELETED — the native body retires the fallback it
   replaces (the surface-shrinks-with-predicate discipline's sibling motion).
   **The staged x86 GOAT RAN mid-batch** — the 4090 lane saw this batch's
   claim and executed the companion bench ahead of the landing
   (`.benchmarks/116_avx2_q2_x86_goat.md`, `benches/bench_007_avx2_q2_x86_goat.rs`,
   i7-13700K, PARETO-PASS): G1 311 cases bit-identical, G2 25/25 shapes
   strictly faster at 2.20x–2.94x vs the naive per-element scalar (the B62
   law), G4 zero-alloc both arms, provenance PASSED (one transient-load
   refusal correctly re-run quiet). The rider's NotBenchable-on-M3 note
   carries the bench reference; no corpus edit from the 4090 (entries_gpu_33
   was this lane's surface mid-batch — the bench_005/106 precedent).

## Dedup (grep-verified at authoring)

`pins everything` / `buffer_from_host_ptr` / `foreign range` / `from a host
pointer` — zero corpus hits. `maddubs`/`cvtepu8`/`0x03030303` hits are
prose-consumers (gpu_75/60/50/89) and the B107 selector — no Q2_0 x86 dot
owner. The taxonomy line before this batch read CUDA vadd4 / ARM b2b-LUT /
x86 shuffle-fanout+cmpeq (the Q1_0 1-bit face); P53 adds the Q2_0 2-bit row.

## Gates (all green)

- G1: `batch231_rules_parse_and_needles_landed` (13 new-rule needles +
  ref/opt parse + 9 rider needles) + `batch231_rider_landed_on_madvise_host`
  (8 needles) — both pass.
- G2: kernel_opt+rust_perf gated lib **3095 passed / 0 failed / 1 ignored**;
  default lib **2650 passed / 0 failed**.
- G3: clippy 0 findings default; all-features 0 code warnings (only the
  pre-existing `block v0.1.6` future-incompat dep notice).
- Retrieval floors HELD, no re-pin: kernel self-variants **100.0%** (30),
  blended **91.2 / 94.7 / 95.3** (171).
- Count pins moved at SEVEN sites: the corpus total ×5 (tests 112, 5500,
  5511, 9619, router.rs 472), the per-category table (A 164 → 165, the
  `kernel_categories_counts_match_plan_003_table` pin the first run caught),
  the batch-227 historical landing assert (re-pinned with a note), the
  KERNEL_CATEGORIES A-slice + its header comment (165) + the module doc
  (804). rust_perf stands at 198 (rider count-neutral, noted in its pin).
- NO freeze regen owed (zero clippy-corpus delta — the cycle-74 summary's
  "one freeze tail" expectation was wrong: batch 230 `3f2d87a6` + P51a
  `3ff0013f` both already landed their v2+v3 regens; P30 applies).
  Score-bench not owed (same P30 line, the batch-229 precedent).

## Lessons

- The per-category expected-count table (`(KernelCategory::MemoryAccess,
  164)`) is an EIGHTH count site the grep-for-803 sweep missed — the first
  gated-suite run caught it. Future count-moving batches: grep the category
  enum name, not just the number.
- The batch-225 x86 face (Q1_0, shuffle-fanout+cmpeq) and this batch's x86
  face (Q2_0, slli/OR+maddubs) are DIFFERENT mechanisms on different field
  widths — the "x86 face" label alone does not make a duplicate; dedup on
  the mechanism, not the ISA label.
- Subagent A's kinship correction: `pinned-upload-staging-ring` is the B213
  rider on `ring-overwrite-gated-k-back-readiness`, not a standalone
  rule_id — dictated kinship names must be rule-id-verified before landing.

Batch 231 / Plan 233. Landing commit: this commit.
