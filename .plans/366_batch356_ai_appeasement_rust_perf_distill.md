# Plan 366 — Batch 356: AI borrow-checker-appeasement rust_perf distill (r/rust case study)

**Status:** STAGED UNCLAIMED → **CLAIMED 2026-11-12** — verdict YES B+ (Research 400); owner directive 2026-11-12 ("better catch if not yet"). Numbering note: plan 365 + nominal Batch 355 were taken by the landed `b706f9ed3` (sglang unified-memory) while this plan was in flight — renumbered 366/356 per first-allocation.

**Claimed:** zed-m3-b356-appease, 1794489505, expires 2026-11-14T13:18:25Z

Source: r/rust discussion, post-only, no repo (decision-content-only transcription — ORIGINAL Rust fixtures mandatory, never post code). Five candidates ranked in Research 399; every entry adjudicates in-rule against its named kin (the dedup table) at authoring. Nominal entries file `entries_72.rs` (71 highest at plan time — RE-DERIVE at landing, the B341/343 precedent).

## Tasks

- [ ] T0 dedup re-verification at landing HEAD — 3+ vocabularies per candidate, every hit read and ruled (`mem-take` polarity; `avoid-clone`/`collect-avoidance` position; `bounded-batch-buffer-pool` family + `slow-consumer-latest-value-single-slot`; dense/table family incl. `fxhashmap` + `per-scope-map-scan-hash-crossover` + `minimal-perfect-hash-static-lookup`'s mutable-keyset anti-regime; thread-spawn axes)
- [ ] T1 author R1 `mem-take-field-checkout-empty-window` — suggest-only, NotBenchable annotation; the empty-window hazard IS the decision content; in-rule adjudication vs `mem-take` (same primitive, OPPOSITE polarity — there take is the clone+clear remedy, here it is the appeasement)
- [ ] T2 author R3 `thread-spawn-per-call-pool-reuse` — the BENCH LEAD; rust_perf_bench spec (spawn/join per call vs a built-once worker pool; timing + exact alloc axes); anti-regimes: one-shot calls, tiny per-call work, the rayon ~5 µs threshold law
- [ ] T3 author R2 `clone-before-mutating-loop-borrow-appeasement` — suggest-only; the self-aliasing POSITION is the decision content; adjudicate vs `avoid-clone` (by-value signature) + `collect-avoidance` (chained iterators)
- [ ] T4 author-or-drop R4 `cross-thread-buffer-return-ring` + R5 `dense-id-domain-hashmap-to-slot-array` — benchable, expected-drop allowed on the family adjudications (the honest drop is recorded in the entries header, never silent)
- [ ] T5 fixtures `corpus/fixtures/b356_fixtures.rs` — ORIGINAL reproductions, standalone-compiled AND executed BEFORE the first corpus edit (4-arm discipline: buggy / fixed / all-safe stub / tampered-verdict)
- [ ] T6 gates at isolated CARGO_TARGET_DIR — batch356 needles+parse, rust_perf lib, default lib, clippy -D postures, retrieval floors both lanes (re-derive MEASURED if a span moves, the B341 law)
- [ ] T7 count pins — `pins_manifest --write` (rust_perf 316 → re-derived at landing) + the count web (AGENTS bullets, README, rust_perf.md, health_views, tests.rs narrative heads); freeze/score-bench NOT owed if the clippy_lints delta is zero (P30 law) — assert it
- [ ] T8 release-set-binary decision at landing per the B341/B343 precedent (rust_perf is in the shipped set; a corpus-only delta is invisible to the maintained binary until its next rebuild — disclose, owed at the next maintained-binary lane)
- [ ] T9 dual_allocation_gate + docs (Research 399 → DISTILLED-batch-landed; AGENTS counts; snapshot LANDING row) + ONE feat: commit + push; scratch removed
