# Plan 371 — Batch 360: polars revisit #2 delta (rust_perf)

> Status: OPEN — staged plan-first per the standing discipline (adopted Batch 60); executes Research 405 (`.research/405_polars_2x_streaming_delta_distill.md`). Verdict YES B− corpus-delta / lane-intel NONE / product-coverage YES. Source `pola-rs/polars` @ `61fdb30fa49c467cf50facfd894aaefe140979fa` (MIT; RE-CLONE + RE-PIN at mine time — fast-moving master). The cargo half awaits a free window; the claim row lands when the lane starts, the push is the arbiter.

## Scope

- [ ] T0 — Re-clone polars at master, re-pin sha + verify the Research 405 quotes byte-exact at the new pin (the release-note lines cite py-2.1.0-era bodies; the kernel quotes cite files that may have moved); re-check the P113 upstream state per row (all rows are decision-content — the outbound-PR lane is FAIL-CLOSED for this source per their AI policy #28206).
- [ ] T1 — Dedup re-verification at landing HEAD: 3+ vocabularies per candidate (flag-guard: MARKED/flag-in-entry/bit-steal/first-hit; char-count: continuation/0xC0/class-mask/popcount-identity; funnel: shrd/u128-combine/shift%64; sorted-run: adjacent-run/run-group; ctx-reuse: thread_local/codec-context; prefetch: block-probe/two-pass; hot-table: skew/hot-tier), every hit read and ruled in-row; the four shared-mechanism adjudications (B244 iter-count-bulk, software-prefetch-storage-bound, B334 single-entry, bitflag-pack-when-footprint) resolved both directions in-rule.
- [ ] T2 — R2 face-or-rule decision FIRST (the B66 float-sum precedent): if `utf8-char-count-class-mask-popcount` resolves to a positive-instance FACE on `iter-count-bulk-specialization` (entries_30), land it count-neutral there + carry the char-count IDENTITY as the face text; only grade it a standalone rule if the identity (len − continuation-popcount = char count) + the shifted-self inline mask survive as new fix-space.
- [ ] T3 — Corpus entries (target 2-4 after falsification; every candidate may drop with its reason recorded): R1 `first-hit-flag-guards-aux-structure` (caching or layout — follow the family grep at write time) + R4 `u128-funnel-shift-combine` (arithmetic, beside B69 with in-rule adjudication) + the R3/R5/R6/R7/R8 survivors. ORIGINAL minimal fixtures (MIT hygiene — never polars code).
- [ ] T4 — Pre-registered GOAT bars: R1 bench = bitset-mark-per-event vs flag-guarded mark over a seeded duplicate-density ladder [0%, 50%, 90%, 99%], sizes [1K, 16K, 256K]; G1 bit-identical marked/unmarked sets at every density; bar = ParetoDominant at ≥90% density AND no regression at 0% (the flag's load+branch tax). R4 = exhaustive shifts 0..=64 × boundary word pairs vs the two-shift reference (the shift=0 trap pinned in the tampered arm). R2 (if standalone) = chars().count() vs byte-class popcount over ASCII-heavy/mixed/multibyte-heavy ladders, G1 exact on valid UTF-8.
- [ ] T5 — Fixtures `b360_fixtures.rs` standalone-compiled AND executed BEFORE entry (four-arm discipline per law: buggy/fixed/all-safe/tampered).
- [ ] T6 — Standing gates at an isolated `CARGO_TARGET_DIR=/tmp/b360_target`: batch360 needles, default lib, rust_perf_bench lib (if benchable entries landed), clippy `-D warnings` ×3 postures, fmt gate, retrieval floors (re-pin only on a REAL regression), pins_manifest --write same commit as the corpus change.
- [ ] T7 — Count pins one-landing: AGENTS.md + README + `.docs/11_domains/rust_perf.md` head + health_views + mod.rs narrative + the rust_perf witness wherever the B355/B359 batch landed it; freeze/score-bench NOT owed if the clippy_lints surface is untouched (assert it).
- [ ] T8 — Docs: AGENTS.md batch entry; queue snapshot Batch 360 pool → DONE; Research 405 Status → RECORD (batch landed); `.raw/polars` removed at close (cleanup is part of the batch — a landed batch with a live .raw entry is an unfinished batch).

## Non-goals

- No polars outbound-PR lane (their AI policy forbids agent interaction; every row is PRESENT-state decision content).
- No polars dependency anywhere — fixtures are original std-only reproductions.
- The 2.0 streaming/OOC engine surface: engine-ambient, out of scope (recorded as drops in Research 405).
