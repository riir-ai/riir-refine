# Plan 384 — sybil inference-optimisation-skill distill batch (rust_perf measurement-honesty)

**Status:** STAGED — unclaimed (verdict Research 417, YES B corpus / LANE INTEL HIGH; behind Plan 383/mlx-learn; batch number re-derived at start)

Source: sybil-solutions/inference-optimisation-skill @ `fda968e7fc1996aa23c67139c755fe541c9d4be0` (MIT, tip 2026-11-16; full clone inspected 2026-11-16, quotes verified at the pin, `.raw/` removed at verdict close). All candidates suggestion-only `NotBenchable` + `bench_spec: None` (the metric-mismatch class — the B367 opal tranche posture). Fixtures are ORIGINAL Rust reproductions; never paste source content.

- [ ] Re-read the mining queue snapshot + re-derive the batch number + corpus count from `src/domains/rust_perf/corpus/` at start (snapshot counts are claims).
- [ ] Claim lease: append `**Claimed:** <session>, <epoch>, expires <ISO>` + push BEFORE any corpus edit.
- [ ] Dedup re-verification at the landing HEAD (the corpus moves between verdict and batch): re-run the five dedup greps from Research 417 (chunk/usage counting, degenerate-output, driver-residency, specdec-composite, prefix-cache) under both vocabularies.
- [ ] R1 `token-count-from-authoritative-counter-not-stream-chunks` entry (authoritative measurement units; faces: real-tokenizer workload sizing vs word/byte heuristics) — fixtures/b369-style standalone fixture file, 4-arm discipline (buggy/fixed/all-safe/tamper).
- [ ] R2 `degenerate-output-invalidates-throughput-claim` entry (repeated-8-gram-share > 0.5 over > 50 grams + failed-request + idle-device flags beside any throughput verdict) — fixture with a looping-output arm vs a healthy arm.
- [ ] R3 `gpu-residency-summed-beyond-process-and-cgroup-accounting` entry (driver residency summed with `drm-client-id` dedup + anon; MemAvailable 14 vs 29.4 GiB case; adjudicated vs `macos-pressure-reads-footprint-not-rss` in-rule both directions) — fixture: a memory budget reading process counters only vs the sum.
- [ ] R4 `specdec-composite-decomposed-into-per-step-time-and-accepted-length` entry (tok/s = a ÷ step_time; higher acceptance can be slower; completion-scoped counter bias face) — adjudicated vs `alloc-proxy-inversion-work-ratio-caveat` + the B177 A/A rule in-rule.
- [ ] R5 `unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately` entry (unique per-run prompt head; never publish a cache-hit prefill as prefill; fixed seeded panel for A/B) — fixture with a reused-prompt arm hitting the cache.
- [ ] R6 author's call at authoring: `work-removed-build-is-not-a-valid-lower-bound` as its own small rule vs a count-neutral face on the b363 wall-clock rule (adjudication: instruction-count-blindness icount axis vs the codegen-changed axis).
- [ ] F1 face on `wall-clock-ungated-shared-ci-noise-floor` (b363): neighbour-sandwich comparison + end-of-run box-state record + void-on-change.
- [ ] F2 face on the B177 A/A-control rule (entries_08): fixed seeded input panel across arms (the control-side complement of conditioning disclosure).
- [ ] F3 face on the wall-clock/microbench family: interleaved min-of-N; cold-clock ramp made a kernel look 2.5× slower.
- [ ] Standing gates: fixtures standalone-compiled AND executed pre-entry; `cargo clippy -D warnings` (default + no-default); retrieval floors (29/0) no re-pin unless genuinely moved.
- [ ] `py scripts/pins_manifest.py --write` ONLY with the corpus delta; the prose count web (AGENTS + README + .docs/11_domains/rust_perf + health_views) updated in the same landing.
- [ ] fmt gate (bare `rustfmt` per touched file); score-bench/freeze disclosed per the B363/B366 precedents (zero clippy_lints surface → NOT owed; disclosed anyway).
- [ ] Research 417 stays the provenance record; queue snapshot line updated at landing (LANDED header, not the verdict line); AGENTS.md §Role rule-count line updated per the count web.
- [ ] Lane-intel follow-ups recorded as OFFERED (never auto-applied): specdec ceiling-template + draft-vocab-pruning addendum to riir-infer issue 1179 (gated on 1183's owner verdict re-arming the beat lane); ceiling-column + driver-residency box-state offers to the perf-rematch runbook owner; ceiling.py finding stays in Research 417 (upstream report optional per the owner).
- [ ] Commit + push on develop (conventional prefix), summary carries the corpus delta + the lane-intel disposition.
