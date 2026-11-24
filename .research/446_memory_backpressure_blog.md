# Research 446 — distill verdict: totodore "Memory-bounded systems and backpressure in Rust"

**Status:** DISTILLED — pending owner decision (Plan 412 staged, nominal Batch 413; unclaimed — fire on owner go).
Source: <https://totodore.github.io/memory-backpressure/> — Théodore's blog, published 2026-11-22 (2 days old at inspection).
Inspection date: 2026-11-24, session zed-m3-distill-membp@m3 (epoch 1795517992). m3 QUIET; zero cargo, zero GPU.
Provenance posture: **POST-ONLY** (the B366/B408 precedent) — the live-fetch URL is the pin; no companion repo verified (batch T0 re-checks the author's GitHub for one); every quote below was read off the fetched page this session.

## Verdict

**YES B− corpus / lane-intel NONE / product-coverage FIRES — seeds the tower·axum admission family (0 rules today) and the jemalloc·allocator-observability family (0 rules today); tokio family (31 rules) gains the weighted-Semaphore face additively.**

Grade rationale: 6 candidates staged, two A−/B+ leads, all dedup-clean under both vocabularies, every candidate suggestion-only NotBenchable-by-domain (the win metric is bounded-memory/honest-census — no Pareto validator prices "the gate fires at the right RSS"; the dial9/B136 annotation posture, never a demotion). The corpus's measurement-honesty category (24+ rules) and the admission/memory-budget family (~14 rules) are exactly the landing surfaces. Yield 3–5 rules after falsification keeps it in B− band.

## What the source is

A production post-mortem turned how-to: bounding a live ingestion HTTP server's RSS (push model, 503 + `Retry-After`) and a NATS JetStream pull consumer (pull model, the stream as buffer). Three solutions — indirect (concurrency × max body size), byte budget (one permit per byte), global (allocator stats, jemalloc) — plus the measured footguns found while wiring the global one: prefixed-jemalloc C-dep census blindness, THP invisibility, narenas-scaled-to-host. The author rolled the two config fixes out and measured RSS dropping to track the `resident` metric. This is the QUALIFIED-CONFIGURATION + measurement-honesty class, not kernels.

## Ranked candidate table

| # | Candidate (proposed rule_id) | Grade | Category | Decision content | UPSTREAM state |
|---|---|---|---|---|---|
| R1 | `allocator-census-blind-to-unprefixed-c-deps` | **A− LEAD** | measurement-honesty | tikv-jemallocator defaults to PREFIXED symbols (`__rjem_*`) so the `#[global_allocator]` swap never overrides `malloc`; every C dep (zstd named in-source) keeps libc allocations — invisible to the census — and any allocator-stat-derived bound undercounts exactly the heavy-compression paths. Fix: `unprefixed_malloc_on_supported_platforms` + a verification arm (nm for `__rjem_malloc` interposition, or a runtime canary allocating through the C dep and asserting the census moved). Regime: allocator-stat bounding + C deps in the tree. Anti-regime: pure-Rust trees (census complete by construction). | present — the blog's own rolled-out fix |
| R2 | `thp-mappings-invisible-to-allocator-resident` | **B+** | measurement-honesty | THP `enabled=always` remaps 4 KiB pages onto 2 MiB pages behind the allocator; jemalloc does not count huge pages in `resident` → the calibrated `MAX_MEMORY = (limit − base) × factor` undercounts AND RSS stops falling (dirty huge pages pin; in-source: RSS flat under much smaller traffic). Fix: `MALLOC_CONF=thp:never` (unprefixed build reads `_RJEM_MALLOC_CONF`) + boot check of `/sys/kernel/mm/transparent_hugepage/enabled`. The PAGE-SIZE axis of the counter-divergence family — adjacent-DISTINCT from `macos-pressure-reads-footprint-not-rss` (OS-counter choice) and `gpu-residency-summed-beyond-process-and-cgroup-accounting` (device-residency summation), both directions in-rule. | present — rolled out, measured (RSS now tracks `resident`) |
| R3 | `sampled-gauge-extremum-between-sparse-exports` | B | measurement-honesty | A gauge sampled at 1 Hz and exported at 60 s loses every inter-export peak unless the sampler carries last/min/max (the in-source `Sample` struct: atomic `fetch_min`/`fetch_max` over the window). The load-bearing subtlety: read-and-reset must fall back to `last` when the window recorded NOTHING (a naive `swap(0)` prints a zero trough; `swap(u64::MAX)` a fake peak) — the min and max arms each carry their own empty-sentinel. ~30-line transcribable shape. Regime: export interval ≫ sample interval. Anti-regime: export == sample cadence. | present — their shipped OTEL wiring |
| R4 | `reactive-memory-gate-needs-hard-backstop` | B | measurement-honesty/admission | The composite admission law (the article's own conclusion): a sampled-metric gate is lagging TWICE — growth between samples (the margin must absorb it; the 0.75 factor is derived, not tuned) and slow fall (freed memory stays in dirty pages, so the gate stays shut after load ends) — so a burst can outrun the margin; the reactive global bound is PAIRED with a hard per-request bound (concurrency limit or byte budget), never stands alone. Owns the REACTIVE-vs-HARD admission-posture axis; in-rule adjudications vs the family (below). | present — their conclusion table |
| R5 | `jemalloc-narenas-scales-with-host-not-cgroup-quota` | B− | measurement-honesty | Default narenas = 4 × `sched_getaffinity`; cgroup CPU quota does not change affinity, so a 1-CPU pod on a 64-core host gets 256 arenas (in-source figure) → `epoch::advance()` (which walks every arena) and arena metadata scale with the HOST, making the sampled census's own cost box-dependent. Fix: explicit `narenas` sized to the quota in `MALLOC_CONF`. Adjacent: the STAGED-not-landed `memory-headroom-cgroup-honest-not-host-counter` (B371/codegraph staging — same container-honest genus, different surface: memory counters vs thread-derived arena sizing); re-check its landing status at batch T0 and fold as a face if it landed first. | present |
| R6 | `body-limit-before-decompression-bounds-the-wrong-size` | B− | measurement-honesty (wrong-quantity class) | Two conjuncts: (a) tower layer ORDER — `RequestBodyLimitLayer` before `RequestDecompressionLayer` bounds the COMPRESSED size; axum's own 2 MiB `DefaultBodyLimit` shadows the explicit layer unless disabled; (b) the byte-budget undercount — `Content-Length` is the compressed size, chunked bodies carry none: reserve the worst-case cap (`unwrap_or(cap).min(cap)`), decompress first. Faces: `load_shed` reject-vs-queue folds onto `bounded-channel-backpressure`; the pull-model corollary (stop pulling while over budget; the stream is the buffer) folds as a second face. | present |

**Drops** (recorded, transparency): the `CountingAlloc` wrapper section — measurement-infrastructure how-to (the `cpu-model-per-run-metadata` DROP precedent), and the corpus already uses a counting allocator as fixture instrumentation (entries_39), so the shape is known, not mineable; the indirect-bounding walkthrough — ambient architecture (tower/NATS config narrative, no bounded fix-space); NATS JetStream knob specifics (`max_ack_pending`, `max_bytes_per_batch`) — provider config, not Rust decision content.

## Dedup-gate evidence (both vocabularies, this session)

Source vocabulary `backpressure|jemalloc|thp|huge.?page|madvise|memory.?budget|counting.?alloc|load_shed|byte.?budget|try_acquire` over `src/domains/` — **zero hits**. Operator vocabulary: rule-id census of rust_perf (400 ids) — measurement-honesty family has NO allocator-census or extremum owner (`token-count-from-authoritative-counter-not-stream-chunks` owns the UNIT axis, not the WINDOW axis; `benchmark-spread-above-threshold-reports-unstable` owns stability verdicts); memory/capacity family (~14 ids) owns batch sizing, capacity accounting, undo availability, prompt-state budgets — NO owner of reactive-vs-hard pairing, byte-permit admission, or decompression ordering; tokio family (31 ids) owns `bounded-fanout-semaphore` (count permits) — the per-BYTE permit (`try_acquire_many`) is a distinct fix-space, adjacent; `grep global.?alloc|counting.?alloc` → fixtures only (entries_39 instrumentation) + a docker musl-allocator ANNOTATION rule (allocator-choice axis, not census-completeness). All six candidates pass. Adjacency reads done in-session: `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (entries_47), `reserve-then-commit-capacity-accounting` (entries_58), `capacity-gated-exact-undo-loud-degrade` (entries_82), `gpu-residency-summed...` (entries_84).

## GOAT adjudication (the two-class split, applied)

No candidate passes criterion 1 — and the failure is per-rule DOMAIN-shaped: the win metric is a correct census / a gate firing at the right threshold; no validator in this corpus prices bounded-memory or observation honesty. Per the dial9 correction this is the NotBenchable-by-domain ANNOTATION posture (`bench_spec: None`), never a demotion — the corpus's measurement-honesty category and the sccache/hydra-db/opal NotBenchable distills are the standing precedent. Criteria 2–4 (exact semantics, mechanism-level magnitude story, determinism) PASS for R1/R2 (deterministic config+census laws with in-source measured deltas) and R3 (exact arithmetic). Fixture plan: original minimal Rust reproductions (Constraints law) — a CountingAlloc-vs-C-dep census fixture, a THP-blindness census fixture (page-size stand-in), a Sample window fixture (the empty-window fallback as the tamper arm), a reactive-gate pairing fixture.

## Lane-intel axis — NONE (recorded honestly)

No league scoreboard row maps (the league rows are prefill/decode GPU lanes; this is host-RSS ops). Not a lane-mapped verdict; no sibling issue filing recommended. One internal-hygiene note, never a grade input: riir-refine's own `index_cache` byte cap (`RIIR_REFINE_INDEX_CACHE_MAX_MB`) and any future escalation-chain memory posture are consumers of R4's pairing law (reactive cap + hard bound).

## Product-coverage axis — FIRES

tower/axum: **0 corpus rules today** — a top-tier ecosystem surface (every axum service is tower middleware); this source seeds the family. jemalloc/allocator-observability: **0 rules** — tikv-jemallocator is the standard production allocator and its census/footgun surface is unowned. tokio: 31 rules — `try_acquire_many` weighted permits join additively. Follow-up pool (accumulation clause — pool sources read before the batch window fold into the same batch): tower-rs/tower (`limit`/`load_shed`/`buffer` docs + the guides), tokio-rs/axum middleware docs, tikv-jemallocator README + jemalloc `MALLOC_CONF` tuning docs, the redis THP doc the source cites.

## Yield estimate

4–6 staged → **3–5 rules + 1–2 count-neutral faces** after ~50% coordinator falsification. Lead with R1; R2 second (the two carry the measured deltas and the cleanest bounded fix-spaces).

## Honest caveats

- POST-ONLY: the URL + 2026-11-22 publication date is the provenance; batch T0 re-fetches and byte-verifies every quote against the live page before any corpus edit.
- The author's figures (+100 MiB resident vs +30 allocated over the same traffic window; 0.75 factor; narenas 256 on 64-core) are THEIR production measurements — provenance, never bars (the sybil/B369 disclosure convention).
- The blog claims "the allocator trait has only been stabilized in Rust 1.100; so far only Vec and Box support it" — UNVERIFIED toolchain claim; T0 checks before anything cites it; never carried into rule text unverified.
- tower API specifics (`GlobalConcurrencyLimitLayer` feature availability, `HandleErrorLayer` import path on current tower) — T0 verifies against the pinned tower version.
- Whether any sibling repo actually runs jemallocator (the "fires at home" honesty line) — T0 greps the workspace Cargo.tomls; if none do, the rules fire at consumers' repos (product-coverage posture, the dial9 lesson applied in the correct direction).
- The blog is one author's single-service experience — the R4 margin factor (0.75) is THEIR derivation; the rule carries the derivation LAW (margin = f(between-sample growth)), never the number.

## Queue slot

Plan 412 staged (nominal Batch 413), UNCLAIMED — fire on owner go or the idle loop's next window; batch T0: re-fetch + re-grep the six candidates against the corpus AS IT THEN STANDS (the Batch-409 lesson: a staged lead's distill-value claim expires against the corpus as batches land — re-verify at claim time).

Session: zed-m3-distill-membp@m3, epoch 1795517992. NOTHING INVENTED — every quote read off the fetched article this session; corpus facts from this session's greps.
