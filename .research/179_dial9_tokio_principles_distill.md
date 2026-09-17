# Research 179 — the dial9-rs "Principles for fast Tokio applications" distill verdict

**Status:** DISTILLED — pending owner decision (2026-09-17; verdict re-graded same day under owner challenge — both grades recorded below)

## Verdict (corrected)

**YES — B− corpus / no lane mapping.** Route `rust_perf` → concurrency (the corpus's existing tokio family), NOT `kernel_opt` (zero GPU content; the routing table's CPU/generic-Rust rows reject it outright — no trap here).

Yield after ~50% falsification: **1–2 rules**. Lead: `batch-blocking-pool-segment`.

### The re-grade record (honesty axis — kept because it is the reusable lesson)

First pass graded **MARGINAL B−** (2026-09-17): the GOAT candidate failed criterion 1 (benchable on the Pareto validator) and the skill's sentence "if no candidate passes all four, the source is MARGINAL as a corpus batch at best" was applied as a veto. Owner challenge ("why you ditch B grade?") forced re-examination. Two errors found:

1. **A DOMAIN property was used to demote a SOURCE.** Benchability fails for *every* tokio rule — fairness/coordination metrics live in a runtime the temp-crate validator cannot host. Under the MARGINAL reading, no tokio source could ever grade YES, yet the corpus contains a tokio family (`async-yield-point`, `tokio-guard-across-await` / Research 032 hotpath-rs, `bounded-channel-backpressure`) that somebody graded in.
2. **The corpus's own landing record contradicts the veto reading.** Batch 136 (the Lattimore "Graph Algorithms in Rayon" blog distill) landed 3 rules, ALL NotBenchable-annotated decision-content, from a qualitative source with NO measured numbers (queue-snapshot record, folded 2026-09-13 section). Batch 135 (Wild-linker article) and Batch 91 (rustdoc-33% article) are further blog-distill precedents. Research 176 graded an external serving source YES B+ the day before this verdict. The corpus demonstrably runs TWO tracks: measured Pareto rules AND suggestion-only decision-content rules; the GOAT gate's benchability criterion identifies the flagship of a MEASURED batch, it does not gate suggestion-only batches.

Correct application: benchability unavailability is recorded as a per-rule annotation (NotBenchable, `bench_spec: None`), and the verdict grades on decision content × novelty × dedup clearance × yield. Both grades are kept in this note so the next borderline call can cite the resolved precedent instead of re-litigating it.

## Source

- URL: `https://dial9-rs.github.io/blog/principles-for-fast-tokio-applications/` — inspected 2026-09-17, full text read (prose principles post; no code tree → no `.raw` clone; the promised sample app is not on the page).
- Author signs as "Russell" (the dial9 tracing-tool project); provenance chain: the post ← RustConf Unconf discussion + Alice Ryhl's "What is Blocking?" (the 10–100 µs poll guidance). Cite both at mine time.
- Stable canonical URL (GitHub Pages, canonical tag present); quotes re-verifiable live at mine time.

## The technique inventory → candidates

Ranked table (grades are decision-content grades after dedup):

| # | Candidate | Grade | Dedup evidence (both vocabularies) | Decision content |
|---|---|---|---|---|
| 1 | `batch-blocking-pool-segment` — "tokio::fs is considered harmful": N small `spawn_blocking`/fs ops in a loop → ONE large blocking segment (or a dedicated OS thread); the blocking pool is a GLOBAL runtime resource | **B (the lead)** | ZERO corpus hits: `spawn_blocking`, `block_in_place`, `tokio::fs`, blocking-pool batching — the only genuinely novel shape in the source. NOT covered by `buffered-io`/`bufwriter-batched` (those batch SYSCALLS via buffers; this batches BLOCKING-POOL COORDINATION via segment size — different mechanism, different fix space; record the boundary in-rule) | Two-regime: many-small-ops (pay per-op `spawn_blocking` coordination; saturation source-reported ~50k tasks/s @ 32-core) vs few-large segments. Bounded fix: hoist the loop into one closure. Suggestion-only: thread-wakeup timing is the nondeterministic platform class the Batch-78 contended-mutex probe documented |
| 2 | `adaptive-yield-consecutive-ready` — pipelined reads return instantly-Ready forever, starving other connections through the scheduler; yield after K consecutive immediately-ready polls (not every iteration) | **B+ content / standalone-or-rider** | `async-yield-point` EXISTS (entries_02) — but its buggy shape is a STATICALLY-await-less loop with a fixed every-100-iterations yield; this is DYNAMICALLY-instant awaits (the await exists but never suspends) + an adaptive threshold + a genuinely new ANTI-REGIME: long polls are benign under light load (work stealing compensates) and break only under (a) loaded runtime, (b) loaded OS. Standalone with loud distinct-from BOTH directions, or a rider on `async-yield-point` carrying trigger+regime — decide at mine time after re-reading that entry's exact fixture | Source's own measured figure: mini-redis pipelining p50 0.967→0.105 ms, p99 2.548→0.320 ms (10×/8×) — SOURCE-REPORTED, never ours |
| 3 | `bounded-fanout-semaphore` — unbounded task spawn over an external resource (source: 3,000 concurrent S3 connections); "a Semaphore is often enough" | **B− (expect drop)** | ADJACENT to `bounded-channel-backpressure` (entries_02): same family (bound unbounded concurrency), distinct fix-space (spawn-loop acquire vs channel bound). The corpus's same-family distinct-fix-space precedent (filter-before-expensive-transform vs eager-pointer-collect-lazy-fetch) permits coexistence — admit ONLY if the fixture-level distinctness survives the re-read | Semantics-changing under overload (like bounded-channel) → NotBenchable, adopt-deliberately annotation |

Drops (positive instances / not rule-shaped — the dedup gate working):

| Source section | Verdict | Reason |
|---|---|---|
| "Do not hold the lock while flushing, I/O, or awaiting" | KILL | Positive instance of `tokio-guard-across-await` (Research 032). The metrics-registry-flush scenario is fixture-comment color, not a new rule |
| RWLock read-path contention → lock-free map | KILL | Covered by `papaya-lockfree` incl. its Bench-013 single-threaded anti-regime annotation |
| tokio::sync::Mutex only for multi-ms critical sections | KILL | Routing advice; no bounded mechanical fix; overlaps guard-across-await's fix space |
| Spin ~50 µs instead of yielding | KILL | The corpus already carries a park-vs-spin REJECTION (named in tokio-guard-across-await's context notes); the article itself concedes "probably wrong for most applications" |
| Multiple runtimes / niceness / cgroups core pinning | KILL | Deployment topology — not code-shaped, no bounded fix |
| Global queue depth / schedule-latency diagnostics | KILL | Diagnosis methodology (dial9's product surface); zero fix content |

## Lane-intel axis — does NOT fire (workspace lanes; deliberately narrow)

Checked before recording: (a) league scoreboard rows are all GPU kernel lanes (M3 Metal decode, 4090 CUDA prefill/decode) — no tokio scheduling row; (b) `../riir-ai/.issues/*.md` greps for `tokio|yield_now|schedule latency|blocking pool|spawn_blocking` = **0 hits**; (c) no source-shelf row names async-runtime serving.

⚠ Read that zero correctly — it is an ISSUE-LEDGER fact, not a surface fact. Our own repos run tokio (`riir-chaind` RPC, `riir-rest`, the `riir-net` relay, `riir-mmorpg-examples` authority), so this batch's rules fire at heal time on our own code too; the zero only means nobody has filed a tokio perf issue yet. The lane-intel axis is issue-based by design and stays narrow; it does not measure world-facing value.

## Product-coverage axis (added same day, owner directive — the axis the verdict is actually FOR)

`cargo heal` ships publicly (gist-rs/cargo-heal releases + homebrew tap + scoop bucket + the KAT mining network); its consumers are the world's Rust repos, and tokio is top-tier ecosystem infrastructure — hyper, axum, reqwest, sqlx, tonic, aws-sdk all stand on it. The corpus's entire tokio family before this batch is **3 rules** — thin against that surface. "Famous lib" coverage is a first-class corpus goal: the corpus already carries bevy + wasm + rayon families for the same reason. This batch seeds the tokio family; the pool must keep feeding it.

**Async-perf source pool (extends the mining queue — same distill discipline per source):**

1. Alice Ryhl, "What is Blocking?" — the article's own upstream cite; the canonical source for the 10–100 µs poll guidance + the blocking taxonomy. The family's regime boundary lives here.
2. tokio official docs: "Bridging with sync code" + the blocking-pool pages (`tokio.rs/tokio/tutorial` + `Runtime`/`spawn_blocking` docs) — the runtime's own prescriptions, prime suggestion-only rule material.
3. `tokio-rs/mini-redis` — the demo repo the article benchmarks; in-source comments explain the fairness yield. License-check at clone; `.raw` discipline.
4. carllerche/Johnny Törnblom/tokio-discord SIGperf material as leads surface (opportunistic, same two-vocabulary dedup gate).

**Accumulation clause:** if any pool source is read before this batch's window opens, fold its candidates into the same batch — the Batch-145/146 staged-charter precedent ("not batch-justifying alone" is a staging note, not a gate).

## Honest caveats

- **No benchable candidate** — the batch lands NotBenchable suggestion-only entries per domain precedent; its GOAT is decision-content strength + the batch gates (presence/parse/needles/NotBenchable-membership + lib floors + retrieval floors + clippy + score-bench FLAT), the Batch-146/147 shape, NOT a measured Pareto promotion.
- **All numbers source-reported** — the mini-redis latency distributions and the ~50k tasks/s ceiling were never measured on our box; carry them as source-reported with the absolute-numbers caveat.
- **Fixtures ORIGINAL** — the article's handler snippet is mini-redis-adjacent code; never paste verbatim (license hygiene + the binary-only distribution rule). Reconstruct minimal original reproductions.
- **Prose source** — no sha pin; the URL is the provenance; re-verify quotes against the live page at mine time (the source-class note for prose distills).
- **Batch-149 race lesson** — the snapshot's staged row is as raceable as a queue-1 issue; re-read snapshot state ON ORIGIN at batch-plan time.

## Queue slot

Staged in `.distill/001_mining_queue_snapshot.md` (2026-09-17 entry). Execution: next mining window, plan-first — **Plan 142** (`.plans/142_batch152_dial9_tokio_distill.md`); the batch takes the next free batch number at execution (150 and 151 already claimed; nominally 152 — claim at landing per the staged-intake race rule). Owner priority note (2026-09-17): this batch is the SEED of world-facing tokio coverage, not a workspace-lane response — the product-coverage axis above governs the family's continuation.
