# Research 181 — tokio upstream-surface refresh distill (CHANGELOG 1.47–1.52.3 + "Actors with Tokio")

**Status:** DISTILLED — pending owner decision (executing as Batch 154 / Plan 144 in the same session)

**Sources** (both surfaces read live 2026-09-18; the async-perf pool's designated NEXT POOL):

1. **tokio-rs/tokio CHANGELOG** — `tokio/CHANGELOG.md` @ master, releases 1.47.0 (2025-07-25) through 1.52.3 (2026-05-08), read via GitHub 2026-09-18. Plus **PR #7871** fetched and its diff read in the review thread (by Darksonn/Alice Ryhl, merged 2026-01-30, "verified in CI that the new test fails without the implementation change").
2. **Alice Ryhl, "Actors with Tokio"** — https://ryhl.io/blog/actors-with-tokio/ (2021-02-13, revised 2023-02-12), fetched live 2026-09-18. The blog index was swept in full (11 articles): the other unconsumed candidates were read and are adjudicated below (in-place initialization; temporarily opt-in to shared mutation); RFC posts are compiler-territory (not corpus-shaped); "What is Blocking?" was consumed as Research 180.

## Verdict

**YES — B− corpus / no lane mapping / tokio-family continuation (the standing product-coverage goal).** Route `rust_perf` (io + concurrency). The tokio family grows 9 → 11-12 rules; io 23 → 24, concurrency 18 → 20.

**GOAT, the two-class split applied:** no candidate is Pareto-benchable — ALL THREE fail criterion 1 for DOMAIN reasons (io syscall latency; panic/liveness; dataflow deadlock — none host on the CPU/alloc validator), which per the Batch-136/152/153 precedent is a per-rule NotBenchable annotation, never a source demotion. Decision content + novelty + dedup clearance all stand.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `write-vectored-capability-flag` — custom AsyncWrite/Write impl with a real vectored path but `is_write_vectored()` left at default false; adapters (tokio 1.50 `write_buf`/`write_all_buf`, std BufWriter) gate on the flag → the writev path is DEAD CODE, every write degrades to first-buffer-only | CHANGELOG 1.50.0 #7871 + PR diff verified | **B− lead** | Bounded ONE-LINE fix (`fn is_write_vectored(&self) -> bool { true }`); parse-visible trigger (impl block with `poll_write_vectored` but no `is_write_vectored`); std twin since 1.36; regime datum from the PR review (vectored helps when `chunk().len() < remaining()`); dedup ZERO corpus hits |
| 2 | `oneshot-response-send-unwrap-panics` — `respond_to.send(v).unwrap()` in an actor/handler panics when the requester was CANCELLED (select!/timeout/abort dropped the receiver) — a benign common event; the panic kills the ACTOR (a whole subsystem), not the requester → `let _ =` | Actors article | **C+/B−** | The article's own comment: "The `let _ =` ignores any errors when sending. This can happen if the `select!` macro is used to cancel waiting for the response"; `rayon-bridge-oneshot`'s fixed code already exhibits the idiom; panic/liveness class → NotBenchable |
| 3 | `actor-bounded-channel-cycle-deadlock` — cycles of channels WITH BOUNDED capacity deadlock (bounded `send` can suspend); oneshot/try_send/unbounded cannot participate; fix = break the TOPOLOGY asymmetrically (one channel's None breaks the select loop) or try_send/abort — never un-bound the channel | Actors article (§"Beware of cycles", matklad-credited) | **C+** | Source-shipped decision content (the dynamic-work-parallelism-ladder standing); IN TENSION with bounded-channel-backpressure — that tension IS the content (bounded for backpressure, but bounded cycles deadlock); distinct-from nested-par (rayon steal-block) |
| D1 | In-place initialization article | ryhl.io 2025-06-07 | **DECLINED** | Language-design proposal (Init/PinInit traits, `init {}` syntax, the 2025h2 project goal); consumer-side mitigation TODAY requires unsafe `Box::new_uninit` + write or the unstable pin-init crate — unsafe/unstable-shaped (the user_data packed-tag precedent). The `Box::new` stack-then-move motivation paragraph is a good annotation cite for `hot-byval-move-over-memcpy-threshold` (same mechanism family: large-value moves) — recorded here, not folded (the rule's notes are already at length) |
| D2 | Cell::from_mut + as_slice_of_cells (retain-without-indexes) | ryhl.io 2020-08-15 | **DECLINED** | The article's worked problem is a BORROW-CHECKER compile error (E0502) — the buggy shape doesn't compile, so there is no silent perf bug for a healer to find; the working index-juggling version is equally fast. The RefCell→Cell-for-Copy shape (real, zero-cost vs borrow-counter) is type-info-dependent (needs `T: Copy` + never-`borrow()` proof) — clippy/oracle territory, not a syntax-corpus rule |
| D3 | Actor spawn-placement (spawn in handle constructor, not `run`) + reader/writer task split | Actors article | **DECLINED** | Spawn-in-`run` is a COMPILE ERROR (borrow of `self` into a `'static` task) — rustc IS the rule; the reader/writer split is an architectural prescription with no bounded buggy→fixed shape |
| D4 | `coop::cooperative`/`poll_proceed` public API | CHANGELOG 1.47.0 #7405 | **ANNOTATION** | Extends the 128-op budget note on `async-yield-point` (the budget system is now public — wrap non-cooperative leaf ops for a fresh budget); not a rule (API-usage guidance, no buggy shape) |
| D5 | LIFO-slot-steal revert (1.51.0 #7431 → 1.52.2 #8100, "performance impact") + spawn_blocking sharded-queue revert (1.52.0 #7757 → 1.52.1 #8057, hang) | CHANGELOG | **ANNOTATIONS ×2** | Measured-negative stories from the runtime's own authors: (a) the fairness family's measure-don't-assume datum; (b) directly annotates `batch-blocking-pool-segment` — the pool's own maintainers' scalability sharding attempt HUNG, contention-shaped resource |
| D6 | Socket-option family movement: quickack added 1.48 (#7490), `set_linger` DEPRECATED 1.49 (#7752), `set_zero_linger` added 1.50 (#7837) | CHANGELOG | **ANNOTATION** | Extends `tcp-nodelay-request-response-accepted-socket`'s cite chain: the accepted-socket option family keeps growing upstream — one syscall per connection buys per-exchange latency in all of them |

## Dedup-grep evidence (2026-09-18, `src/` under both vocabularies)

- `write_vectored|is_write_vectored|vectored` → **zero hits** (rust_perf corpus + whole src tree). `buffered-io`/`bufwriter-batched` coalesce bytes at USER level — different mechanism (kernel scatter/gather capability vs user copy amortization).
- `oneshot` → 8 hits, all `rayon-bridge-oneshot` (the bridge rule; its FIXED code uses `let _ = tx.send(total)` — the idiom in vivo) + the mod.rs family ledger + tests. No rule owns the response-send unwrap-panic shape.
- `try_send|deadlock|cycle` → `nested-par-inside-channel-bridge-deadlocks` (rayon par_bridge steal-block, thread-tied — different mechanism: steal-blocking vs suspend-in-send); `channel-send-batching-unpark` (park/unpark amortization); no tokio bounded-cycle rule.
- `quickack|linger` → zero hits; `nodelay` → only the B153 rule itself.
- `coop|budget|128` → the async-yield-point annotation (B153) already carries the 128-op note — D4 extends it rather than duplicating.

## Honest caveats

- The CHANGELOG is read at master, not at a release pin — quotes are per-release headers (dated, stable); re-grep at mine time per Plan 144. PR #7871's merge SHA is `9dc9b53` (recorded in the PR page).
- The Actors article is 2021 (revised 2023-02-12): pre-dates `LocalSet`/`JoinSet` evolution; its channel/select content is current (the mpsc/oneshot/select! semantics it relies on are unchanged through 1.52).
- Rule 3's trigger shape is NOT parse-visible (actor cycles are dataflow) — it lands as source-shipped decision content on the dynamic-work-parallelism-ladder standing, suggestion-only forever.
- No lane mapping (tokio does not map to prefill/decode); the product-coverage axis IS the justification — the family continuation the completed pool designated.

## Yield

2-3 rules after adjudication (1 lands near-certainly; 2 and 3 staged with standalone-vs-annotation adjudication at mine time — the pool-choice precedent) + 4 annotation folds. Batch 154 / Plan 144.
