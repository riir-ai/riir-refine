# `rust_perf` domain (Plan 001 Phase 2)

> The second `HealDomain` impl. Heals rust performance issues using rules
> distilled from the `rust-optimize` skill checklist.
>
> **Promoted to default-on 2026-08-13** after the perf-bench Pareto
> validator GOAT gate passed (Bench 013 G1+G5). The domain module now
> compiles by default; the validators (`rust_perf_verify`,
> `rust_perf_bench`) remain opt-in (slow subprocess-based checks).
> **Batch 86 (2026-08-29, Research 100)**: corpus 91 → 94 — the
> Cloudflare 1.1.1.1 DNS-cache memory distill (Plan 047): two measured
> ParetoDominant allocation rules + one NotBenchable-speed layout rule +
> two doc riders.

## Validator hierarchy (strongest last)

1. **L2 pruner** (always-on, modelless): `syn`-based syntax parse check.
2. **`rust_perf_verify`** (opt-in): `cargo test` re-run (compiles + tests pass).
3. **`rust_perf_bench`** (opt-in): generated microbenchmarks at multiple
   input sizes → Pareto verdict (faster on N of M sizes). This is the
   **real perf ground truth** — the promotion gate for the domain
   (Proposal 001 caveat 3). See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md).

## Corpus

**now 337 rules across 15 categories** (Batch 361 +3 +1 face + 1 drop, Plan 372 / Research 405 residue, 2026-11-14 — the polars staged-leads conversion: map-inplace-same-size-transmute [layout, MEASURED ParetoDominant 2/0/1 two-run at the standalone preflight, allocs 1→0, the consume contract; the crate validator cannot host the consume — spec None, preflight the instrument of record] + thread-local-codec-context-reuse [caching, suggestion — the size-admission two-regime] + batch-hash-probe-prefetch-two-pass [io, decision-content on aarch64 — the cache-residency gate] + the SORTED-pole face on sorted-input-previous-row-fast-path; hot-key-skew-split-table DROPPED at the pre-registered bars two-run stable [skew 0.465-0.467x, uniform 0.001x]; pola-rs/polars @ 5c36cdd75; prior: Batch 360 +2 +1 face, Plan 371 / Research 405, 2026-11-13 — the polars revisit #2 delta, `entries_76`, `pola-rs/polars` @ `437948302` MIT (master tip 2026-11-13T15:59:46Z; all nine Research-405 quote sites verified byte-exact at the new pin; clone removed at close; the B66 surface re-opened by the 2.0 kernel layer): `first-hit-flag-guards-aux-structure` (caching — the BENCHABLE GOAT lead: a duplicate-heavy hit stream writes a shared bitset per hit → a one-bit flag STOLEN from the row-index word's top bit (`MARKED: IdxSize = 1 << (IdxSize::BITS - 1)`, matched_rows.rs) gates the side-structure walk on the first-hit TRANSITION, repeat hits pay one load+branch; MEASURED ParetoDominant 3/0/0 TWO-RUN STABLE: 13.62x/52.32x/43.03x then 12.77x/50.47x/63.83x at [1K,16K,256K], allocs 0→0; the single_key.rs `nulls_marked` second instance) + `u128-funnel-shift-combine` (arithmetic — BENCHABLE measured MIXED two-run (2/0/1 then 1/0/2: 1.15x/1.08x/0.98x then 1.08x/1.03x/tie), landed CORRECTNESS-FIRST with the timing disclosed: the two-shift combine `(lo >> s) | (hi << (64 - s))` is UB at s=0 → the u128 funnel `((hi as u128) << 64 | lo) >> (s % 64)` (iterator.rs:429-435, both comments verbatim), the %64 making s=0 exact AND the combine PERIODIC (s=64 wraps, fixture-pinned; the tampered arm pins the clamp-heal returning the wrong window silently), one wide shift = the single SHRD on x86-64 / extr on aarch64) + 1 count-neutral FACE on B244 `iter-count-bulk-specialization` (entries_30 — the T2 face-resolve per the B66 float-sum precedent: the char-count IDENTITY (len − continuation-popcount = char count for valid UTF-8), the byte-CLASS mask `b & 0xC0 == 0x80` predicate, and the shifted-self u128 inline-view mask `data & !(data << 1) & INLINE_CONTINUATION_BITS & used` as face text; string.rs `string_len_chars` #29972); fixtures `corpus/fixtures/b360_fixtures.rs` 6/6 standalone-compiled AND executed pre-entry (four-arm per law); prior: **332 rules across 15 categories** (Batch 359 +5, Plan 370 / Research 403, 2026-11-13 — the ajika persistent-incrementality family, `entries_75`, `rust-glancer/ajika` @ `70c1e26d` MIT OR Apache-2.0 + the handwritten persistent-incrementality blog (tip unmoved at T0, the verdict pin IS the batch pin; clone removed at close; the corpus's FIRST persistent-incremental-computation family — salsa-shaped memoization made durable, 0 prior owners): `named-outputs-lazy-handles-partial-decode` (io — the BENCHABLE GOAT lead: a persisted computation producing several answers together emits NAMED, independently readable outputs and returns cheap address handles; a one-answer read decodes one field on demand and the unread fields never deserialize; a handle is an ADDRESS not a snapshot ('a handle kept from an earlier revision asks for the new answer'); MEASURED ParetoDominant 3/0/0 (two-run stable) at [8, 64, 256] fields of n words — speedups 7.18x-7.48x / 49.65x-55.10x / 158.11x-161.96x, allocs 16→2 / 128→2 / 512→2, alloc bytes 1,024→128 / 65,536→1,024 / 1,048,576→4,096 (the bytes axis = K:1 at every size, the ≥3× bytes-read bar cleared); the anti-regime read-all posture measured Tie/Tie/Tie both runs — the framing tax does not regress the full consumer; in-rule adjudications vs kernel_opt load-time-decode-blind-transform-absorption (absorption-at-load vs decode-on-demand), entries_36/entries_73 pack-one-arena laws (packing at write/lifecycle vs addressing at partial read), entries_05 eager-pointer (fetch-time vs durable read volume)) + `derived-cache-throwaway-version-reset-recovery` (caching — 'The directory is a disposable cache, exclusively owned until `Database` drops'; writer+app schema versions key the reset, corruption discards-and-recovers, uncertain commit recovers via `Database::reopen` on the same owner; in-rule adjudications vs sec execution-artifact-trust-ladder (OPPOSITE polarity — execution-capable vs recomputable), entries_05 resume-revalidates (refuse-and-stop vs discard-and-continue), entries_19 cost-table (noise repair vs version/corruption reset); the home positive instance recorded: refine's own index_cache) + `pack-store-gc-decides-by-recovered-index` (io — 'Append values to private files, then seal them before publishing their locations … Once commit has been attempted, only the recovered index can decide which files are safe to delete'; TARGET_BYTES = 64 MiB rollover; in-rule adjudications vs write-verify-readback-before-publish (single-artifact publish gate vs a store's crash-safe GC) + persistent-cache-teardown-needs-second-witness (refuses-to-delete-without-a-witness vs the recovered index AS the authority)) + `persisted-interner-assignments-checkpoint-with-refering-values` (caching — 'New assignments stay in memory until a durable query checkpoint saves them alongside the values that may refer to them'; IDs never recycled within a live interner; epoch reset discards assignments + referring values as one unit; DROP-OR-KEEP resolved KEEP — zero owners of checkpoint-ordering, the three intern* families own adjacent axes) + `scope-scoped-memo-release-idle-holds-no-decoded-state` (caching — 'these working objects do not keep their keys, output metadata or locks resident while the database is idle' + the blog's 'released memos don't occupy RAM'; release by SCOPE at the operation boundary, never by capacity; DROP-OR-FACE resolved LAND — zero owners, eager-pointer-collect-lazy-fetch owns fetch-time not boundary-release, the GPU transient family owns device-buffer lifetimes (vocabulary adjudicated in-rule)); fixtures `corpus/fixtures/b359_fixtures.rs` 5/5 standalone-compiled AND executed pre-entry (four-arm per law, F1 doubles as the bench harness input); prior: **327 rules across 15 categories** (Batch 358 +5, Plan 369 / Research 345 lane B, 2026-11-13 — the splash ANE/host residue, `entries_74`, `incoai/splash` @ `cabbad27` Apache-2.0 (main tip, tag 1.5.0; T0 re-cloned at the pin, every quote byte-verified, removed at close; the staged-since-2026-10-28 lane whose release-set-binary pin-block lifted under the B355 precedent): `one-program-many-functions-accelerator-service-memory` (layout — the accelerator service bills intermediate memory per LOADED program: 13 one-function programs held the surface set 13× (3.25 GB at the M6's share vs 0.67 GB) vs ONE program with 13 row-length functions (512..2048 step 128) sharing one surface set sized for 2048 rows; compile once per source hash (FNV-1a of the source names the program directory, Program.mm:629), warm load 50-150 ms on an M5 Max; suggestion-NotBenchable) + `ane-split-calibration-fit-bandwidth-interference-term` (measurement — the live split fit T(s)=max(A(s), G(s)+u·A(s)) per device: the GPU line through two alone points, the ANE line off the high share, u the bandwidth the ANE's part takes from the GPU's beside it clamped [0,1] (M6 0.1-0.33 / M5 Pro 3-7% / M5 Max 0); choice = argmin over ±1 unit with a 5% gain floor (kMinimumGain), no gain ⇒ the split REFUSED; per-function minimum-rows chain at kChunkNoise 2% ('640 rows took 93% of 768's'); median-of-3 over a first run with the GPU-alone FFN stand-in subtraction; never a static knob; suggestion-NotBenchable) + `accelerator-lost-breaker-forgets-calibration` (caching — the bounded breaker: fast-path ms vs the ACCUMULATED GPU-alone baseline over kWindow = 8 commands; the sums read BEFORE the reset, the window resets every window; ratio ≥ 1.0 ⇒ retire the split AND fire forget() — the remembered calibration is DELETED so the next start re-derives (AneFfnStartup.cpp:188-195 setBreaker + the forget closure); the measured headroom 'a minute of prefill slows the M6's ANE by a third: far short of the trip point'; the amnesia axis ZERO-OWNER before this rule; suggestion-NotBenchable) + `writeclass-share-partition-inflight-writes` (concurrency — the in-flight budget (kWrites = 64) partitioned per write class: Reclaim all / Persist max(1, ¼) / Spill max(1, ½), 'one kind never starves another and reclaim always keeps a quarter' (KvPageTier.hpp:32 verbatim); admission requires BOTH the class share and the total; suggestion-NotBenchable) + `spin-keep-awake-checked-capability-degrade-loud` (thermal — the clock-holding spin through a GPU-wake-up-scale step needs a memory scope Metal's public headers leave out, so it builds apart from the metallib (host_step.metal) and a compiler that refuses it only warns; BEFORE the first command relying on it the backend checks the spin sees the CPU end it; without the library or on a failed check the steps run without the spin, with a warning — the CHECKED-DEGRADE half is the rule; the entries_02 park law is the OPPOSITE pole (idle: park) and the anti-regime, the two compose; suggestion-NotBenchable); fixtures `corpus/fixtures/b358_fixtures.rs` 5/5 standalone-compiled AND executed pre-entry; ROUTING per row: rust_perf — the wave's serving leads landed here (B270/271), kernel_opt's host-orchestration clause loses (no kernel we author); prior: **322 rules across 15 categories** (Batch 355 +4, Plan 365 / Research 399, 2026-11-13 — the sglang unified-memory distill, `entries_73`, LMSYS blog + `sgl-project/sglang` @ `daa171846f` Apache-2.0 (T0 clone at the pin, removed at close): `co-lifecycle-rows-pack-one-entry-strided-views` (layout — the BENCHABLE GOAT lead: N same-lifetime row-groups of heterogeneous shapes pack ONE entry, per-group strided views keep every consumer shape byte-identical; alloc axis 2N→1, one bulk move/free; in-rule BOTH-directions adjudication vs kernel_opt `split-decay-outputs-eliminate-readback` — the readback-pole anti-regime) + `logical-free-immediate-physical-reuse-waits-settle` (concurrency — the deferred-reuse/epoch-lifecycle FAMILY SEED, 0 prior owners: logical free is immediate, physical reuse waits for the settle; move-with-old-reader = copy + remap, only source reuse waits; move-with-old-writer = drain ordered before the copy; suggestion-NotBenchable) + `joint-demand-admission-single-pool-free-bytes-logs-only` (measurement — the CROSS-POOL coupling axis: page rounding couples one token demand across pools, per-pool free-byte reads on a decision path admit what the rounded joint refuses; the logs-only carve-out + check-mode face; suggestion-NotBenchable) + `fixed-ratio-pool-partition-strands-demand` (caching — the boot ratio is a sizing TARGET, never a run-time WALL; the blog stranding datum: wrong ratios cost 12%/14% throughput, cap concurrency 24 vs 51; suggestion-NotBenchable, declared architecture-adjacent); fixtures `corpus/fixtures/b355_fixtures.rs` 4/4 standalone-compiled AND executed pre-entry (four-arm per law); prior: **318 rules across 15 categories** (Batch 356 +4, Plan 366 / Research 400, 2026-11-12 — the r/rust AI borrow-checker-appeasement case study, `entries_72`, POST-ONLY source (no repo — decision-content-only transcription): `mem-take-field-checkout-empty-window` (allocation — the take check-out/check-in WINDOW hazard: the field sits at its empty default between take and write-back, every reader entering the window observes empty data; the remedy is the disjoint sub-struct split; OPPOSITE polarity of `mem-take`, in-rule adjudicated; suggest-only NotBenchable-by-domain) + `clone-before-mutating-loop-borrow-appeasement` (allocation — the clone whose ONLY purpose is ending a borrow before a mutating loop over the same owner; it type-checks, the value IS used, nightly redundant_clone passes it — the structural POSITION is the signal; suggest-only NotBenchable) + `thread-spawn-per-call-pool-reuse` (concurrency — the BENCHABLE lead, MEASURED ParetoDominant 3/0/0: 4.51x/1.96x/10.81x at [1024,16384,65536], allocs 14→2 bytes 648→104 at every size — spawn/join per call vs a built-once OnceLock pool) + `dense-id-domain-hashmap-to-slot-array` (layout — BENCHABLE, MEASURED ParetoDominant 3/0/0: 70.21x/62.23x/75.57x at [1024,16384,131072], allocs 0→0 — the dense-id tick hashes every id, the slot array indexes; `minimal-perfect-hash`'s own anti-regime names the mutable dense carve unowned); R4 `cross-thread-buffer-return-ring` DROPPED at authoring (`bounded-batch-buffer-pool` owns the recycle law — the post's 5-buffer pool is its positive instance; the concurrent form landed as a count-neutral FACE on it); the god-file/19-impl-block candidate KILLED at verdict (ambient architecture, no bounded fix-space); fixtures `corpus/fixtures/b356_fixtures.rs` 16/16 standalone-compiled AND executed pre-entry (four-arm discipline per law: buggy / fixed / all-safe / tampered); retrieval floors HELD both lanes 29/0 no re-pin; prior: **314 rules across 15 categories** (Batch 349 +2, Plan 360 / Research 393, 2026-11-10 — the encoders distill, `entries_71`: `deferred-integer-lane-pooling-per-row-exponent` (arithmetic — MEASURED ParetoDominant 9.35x/9.28x/15.9x, the dequant temp axis n→0) + `incremental-prefix-hash-wordpiece-scan` (complexity — MEASURED ParetoDominant 1.73x/2.01x/4.30x, allocs 0/0) + 1 count-neutral rider face on `ascii-run-swar-block-skip` (entries_29 — the pretokenizer stage-fusion instance, its atoms owned); prior: **312 rules across 15 categories** (Batch 345 +5, Plan 357 / Research 389, 2026-11-09 — the `LocalSR` distill, entries_70, `HerRei/local-upscale` @ `cf6f8b55` MIT — measurement-honesty +2 (`benchmark-spread-above-threshold-reports-unstable` — the stability-verdict law: warm-up until the rolling CV settles for three consecutive samples, spread over threshold reports UNSTABLE; `macos-pressure-reads-footprint-not-rss` — the counter-choice law, 6.4 GiB RSS vs 2.6 GiB phys_footprint, proc_pid_rusage) + concurrency +1 (`slow-consumer-latest-value-single-slot` — the tokio family's latest-wins COALESCING axis, adjudicated vs entries_02's bounded-BACKPRESSURE sibling) + io +2 (`untrusted-download-verify-then-atomic-install` — verify-before-promote + the use-site re-verify + the install canary; `ui-event-stream-sampled-gate-terminal-forward` — terminal-always / structural-pairs-never / last-item-force); R8 spawn-hygiene dropped at authoring; all NotBenchable decision-content, fixtures b345 18/18 executed pre-entry; prior: 307 at Batch 343 +2 (Plan 355 / Research 387, 2026-11-08 — the sccache cache-layer RE-distill, entries_69, mozilla/sccache @ `c1d7204a` Apache-2.0 PIN-CORRECTED (Research 387's `c1d7204f90a9ed1d` does not exist server-side; the real main-tip sha is `c1d7204a0f14909c83a9cd983646d5c09149cddd`, every quote byte-verified at it) — the tiered-hit backfill family, the files B324 never mined (zero file overlap with entries_58), caching +2 BOTH suggestion-NotBenchable decision-content ALL-ANNOTATED: `raw-bytes-with-hit-backfill-once` (the tiered-hit backfill law — a slower-tier hit reads ONCE in a form both consumers share: `get_with_raw` parses a Cursor VIEW while the same Bytes handle rides to the backfill; the write-back spawned DETACHED so the read path never waits; the fastest-tier hit SKIPS the fill (`hit_level == 0` early return); only tiers with a faster neighbor request raw (idx > 0 gets raw, idx == 0 plain get — the single-tier deployment never pays the raw-copy arm); a level without raw capability degrades to a logged skip, NEVER a second read; multilevel.rs :491-526/:571-601 + disk.rs :126-136 byte-verified; the in-code law: 'Raw bytes obtained above are reused for backfilling; no second read is needed for a raw-capable level.') + `vanished-entry-tolerant-eviction-panic-else` (the shared-directory eviction law — an eviction loop over a shared cache root races other daemons' evictions: remove_file NotFound = a peer evicted first, TOLERATE + debug log (the in-tree issue-2092 cite), ANY other error = our own ledger corruption, PANIC loud (make_space :257-288 byte-verified); peer-caused vs self-caused classification instead of one flattened arm — flattening hides corruption, failing on peers breaks the deployment; adjacent-distinct from persistent-cache-teardown-needs-second-witness, in-rule). THE BATCH'S DEDUP KILL recorded as the gate working: candidate 2 `advertised-size-staged-write-reconciles-at-commit` (Research 387 Law 2) was a DUPLICATE of `reserve-then-commit-capacity-accounting` (entries_58 — the FULL prepare_add/commit law verbatim from the SAME source at the B324 pin; Research 387's dedup note mis-read it as a 'scan-time container rule'); the mtime-walk KILL also held (get_all_files :75-95 = mtime-encoded-lru-recency-persistence's boot-init half); candidate 4 per-tier-lockfree-stat-slots dropped as expected (debug-stat-full-scan-out-of-hot-path owns the axis). Fixtures `corpus/fixtures/b343_fixtures.rs` 11/11 standalone-compiled AND executed pre-entry (read-census one-read pin, answer-before-fill, fastest-tier skip, capability skip, the all-safe stub + tampered-verdict arms); retrieval floors HELD both lanes 29/0 + the priced synth floor PASS, no re-pin; prior: Batch 341 +1 (Plan 350 / Research 383, 2026-11-08 — the metal-graph measurement-honesty half, entries_68); prior: Batch 340 +2, Plan 349 / Research 381, 2026-11-06 — the SPILLWAY agent-egress-guard distill, entries_67, saintmachines/spillway @ `0fc9f55f` Apache-2.0 (arXiv 2611.07222), complexity +2 BOTH suggestion-NotBenchable census-shaped: `paired-closer-precollect-gates-lazy-wildcard` (the closer-existence gate over a two-anchor lazy-wildcard hand-rolled scan — precollect closers once, admit only openers with a closer at/after their end; measured ladder k=128..8192 → 6.3x/48x/121x/1655x, naive scales k·n, the gate stays ~n; the regex-crate linearity domain precision in-rule) + `rare-delimiter-anchored-token-scan` (the rare-delimiter anchor over a per-start token scan — one probe per delimiter at the run head subsumes the run; measured run-pole r=240K ≈ 5e4x, the DENSE anti-regime pole measured 0.75-0.81x and carried in-rule); prior: Batch 334 +2, Plan 343 / Research 372, 2026-11-05 — the nnethercote Nov-2026 rustc-perf distill, entries_66, rust-lang/rust PRs #163628+#163857+#163971, caching +1: `single-entry-last-key-lookup-cache-hash-avoidance` the batch's GOAT lead BENCHABLE ParetoDominant 3/0/0 (6.79×/6.24×/5.49× at 1K/16K/262K, allocs 0/0) — a one-slot last-key memo (`Option<(K, V)>`) in front of a hot hash table: one cheap Eq before hashing+probing, the read-stable-window correctness constraint carried in-rule (upstream #163628's >4% instruction reduction at whole-crate granularity is provenance) + `skewed-payload-inline-encode-intern-overflow-tail` suggestion-only NotBenchable (size-axis class) — skewed multi-field payload packs inline behind a variable-length high-bit prefix (first-fit across formats), the overflow tail interned DEDUPED ON THE COLD FIELDS, the hot read-alone field inline in EVERY format including the interned one (upstream: interning 20-40% → ~1% incremental; the #164291 abandoned-niche negative result carried as anti-regime honesty); prior: Batch 332 +2, Plan 340 / Research 370, 2026-11-05 — the blazing-smtlib distill, entries_65, onderjan/blazing-smtlib @ `42b45c20` MIT OR Apache-2.0, dispatch +2: `byte-flag-table-masked-run-scan` the batch's GOAT-eligible lead BENCHABLE ParetoDominant 3/0/0 (2.62×/2.94×/2.10× at 1K/16K/262K, allocs 0/0) — a const-eval 256-entry u8 class-flag table + a masked-prefix run scan replaces the per-byte predicate chain (regime-complementary sibling of B243's `ascii-run-swar-block-skip`, lane-intel mapped to riir-infer's tokenizer pre-tokenizer) + `token-batch-cursor-handover` suggestion-only NotBenchable (fixed-capacity staging + cursor bulk handover, boundary-applied abort actions). Prior: 298 at Batch 331 +1 (Plan 339 / Research 368 + addendum, 2026-11-05 — the hindsight round-2 pool, entries_64, vectorize-io/hindsight @ `841979df` MIT, concurrency +1: `batch-retry-ladder-classifies-failure-before-permanent-state` NotBenchable-by-domain — a batch retry loop that catches EVERY failure into one posture (identical re-sends up the ladder + durable failure flags on exhaustion) turns a provider CONTROL SIGNAL (quota reset carrying retry_at) into identical re-sends plus a PERMANENT exclusion that orphans the pending items, and burns the ladder on a deterministic failure whose byte-identical replay earns the byte-identical refusal; classify FIRST: PROPAGATE the control signal at first sight (surface retry_at, nothing flags), FAIL_FAST the deterministic failure with the permanent type (zero re-sends, caller bisects), the bounded ladder for transport-shaped only; +1 count-neutral face: the fact_extraction.py promote-permanent-type law, same fixture; composes with entries_63's fanout rule (same source, liveness vs classification). Prior: 297 at Batch 330 +1 (Plan 338 / Research 368, 2026-11-04)).

160 rules across 12 categories (rust-optimize skill checklist +
nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs
profiling guide, Research 032; **Batch 330: 297 rules across 14 categories at the landing (Batch 330 +1, Plan 338 / Research 368, 2026-11-04 — the vectorize-io/hindsight distill, entries_63, vectorize-io/hindsight @ `841979df` MIT (an agent-memory product; the memory-learning engine), concurrency +1: `fanout-first-error-leaves-siblings-running` NotBenchable-by-domain — a task fan-out whose first error propagates while the spawned siblings keep running detached (the orchestrator marks the operation failed and re-queues it while orphans keep side-effecting, breaking the serial-per-scope invariant exactly when it matters); the repair is TWO-step and the second step is load-bearing: cancel the outstanding work AND await the cancellations BEFORE propagating (JoinSet::abort_all + drain every join handle; std twin: a cancellation flag set before the early return, then join); the two-sided discriminator (cancel-then-await when siblings commit SHARED state — write-groups/stamps/marks — let-finish-then-propagate when their work is standalone and valuable, the source's own mirror image at fact_extraction's return_exceptions=True); the error-envelope caveat carried as context (ExceptionGroup reclassification is the Python face; the Rust face is narrower — an envelope that REPLACES the error type breaks retry classification); prior: 296 rules across 14 categories (Batch 329 +2, Plan 337 / Research 367, 2026-11-04 — the tembo/voxl distill, entries_61 + entries_62, tembo/voxl @ `eba3b7ce` MIT (the web-remote-desktop streaming server; crates.io voxl-server 0.3.2), dispatch +1 + concurrency +1: `event-damage-map-selective-partition-skip-with-consumer-hints` (BENCHABLE — the batch's GOAT-eligible lead: one event-sourced per-cell change map drained during the event pump drives EVERY pipeline stage — whole-partition selection (a partition whose row band misses the damage band skips whole; the verbatim predicate `selected = job.band.is_some_and(|b| (b.y0 as usize) < y + h && (b.y1 as usize) > y)` + the whole-frame early-out), work budgets read from map density, and SKIP-HINT FORWARDING into the black-box heavy consumer as per-cell unchanged hints (`mb_info` + X264_MBINFO_CONSTANT) so its own analysis elides static areas; the source's own anchor verbatim: 'x264 pays ~9 ms of CPU per 2560x1600 frame even when nothing changed, and sliced threading triples that'; MEASURED ParetoDominant 2.14x/3.20x/3.14x (3/0/0) at 256/4096/65536 cells, dirty band 12.5%/11.7%/12.0% (the pre-registered 10-15% posture), alloc axis 0/0, alloc-det x3, both arms checksum-identical asserted in-harness — M3 Max / Mac15,8 release, 2026-11-04, load ~4.7 of 16, AC power; the ratios sit under the ~1/dirty-fraction asymptote because the checksum fold stays in BOTH arms + the hint/carried loads are a real per-cell floor — disclosed in-rule) + `online-cost-fit-thread-ladder-reopen-hysteresis` (NotBenchable-by-regime — per-partition parallelism from an ONLINE learned cost model, never a static knob: EWLS fit of encode ms = a + b*changed-work, decay 0.995, the w<30.0 gate on the DECAYED weight (predictability at the 33rd sample), degenerate spread → weighted mean; the ASYMMETRIC two-bar ladder: UP after 20 consecutive frames whose measured-ms EWMA 0.8/0.2 exceeds 65% of the frame budget, DOWN only when the fit's predicted one-thread ms (itself EW'd, thin-fit falls back to measured) sits under HALF the budget SUSTAINED 2 s, the latch reset by any prediction above — because a mode switch reopens the encoder ('A fresh encoder opens with an IDR on its next frame') and costs a keyframe; source anchor verbatim: 'x264's sliced threading costs ~3x the CPU of one thread on desktop content'; +1 count-neutral rider face on `reserve-then-commit-capacity-accounting` (entries_58): the wire-header reserve-in-place law — `out.resize(HDR, 0)` before the encode fills the tail, in-place `copy_from_slice` writes after, `Bytes::from(out)` zero-copy handoff — a prepend/concat copy per stripe-frame avoided). Fixtures both arms standalone-compiled AND executed before entry (the fixed arm's #[test] runs green: consumer-visible byte-identity over moving-band steps + the empty step; the governor's 3+7 decision-shape tests). Prior — Batch 326 +1, Plan 330 / Research 363, 2026-11-04 — the rust-lang/rust #164484 distill, entries_60, rust-lang/rust @ merge `026faeb1` (compiler internals, Apache-2.0/MIT -- the rmeta metadata table, landed upstream 2026-11-01 r=nnethercote), layout +1: `adaptive-sparse-blocked-table-encoding` (NotBenchable-by-domain -- the size-axis annotation class: an index-keyed table whose payload slots are mostly one default value stops paying full width for every default slot -- flush-time census (counting non-defaults only when a presence index could fit in less space than the dense body), smaller-of dense/sparse gate (>25% defaults AND u32-fits AND index_bytes + non_defaults*width < len*width), per-64-entry-block presence bitmap (u64) + cumulative u32 base (a lookup's packed position is base + (bitmap & (bit-1)).count_ones(), never a scan) + non-default values packed in table order, the mode flag riding the header word's low bit ((width << 1) | sparse); upstream measured -20.15% metadata / -3.7-3.8% max-RSS for an ACCEPTED +0.3-0.4% instruction cost (perf-regression-triaged: 'cycles and wall-time were neutral' -- the anti-regime evidence: read-latency-critical random access over nearly-full tables stays dense by arithmetic); the deleted FIXME(eddyb) names the origin (michaelwoerister's HAMT popcnt trick); METADATA_VERSION 11 -> 12 rode the same PR -- the version-bump discipline face; fixtures both arms standalone-compiled AND executed (dense 32,768 B vs adaptive 2,424 B at the fixture's 95%-default shape, all 4096 slots bit-exact round-trip); distinct-from minimal-perfect-hash-static-lookup (LOOKUP vs STORAGE side), distinct-index-bitmap-ascending-compaction (distinct-emitted-indices vs default-heavy-table-slots), the bit-plane transposition rule (columnar values vs presence encoding)). Fixtures original machinery. Gates: count pins via the manifest pass; release-set binary REBUILT + installed (rust_perf IS release-set). Prior — Batch 327 +1, Plan 332 / Research 364, 2026-11-03 — the firezone/sanstun distill, entries_59, firezone/sanstun @ `35cb1932` Apache-2.0 (the sans-IO WireGuard protocol implementation; the pin is the tip squash of #129 'feat!: reserve counters and decrypt data messages in batches', 2026-11-03 — brand-new sanstun-original surface), concurrency +1: `batch-reserve-monotonic-counters-offload-bulk` (NotBenchable-by-domain — the coordination-law annotation class: N per-item synchronized counter trips collapse to ONE bound-checked range reservation (the count checked_add-ed, the end filtered against REJECT_AFTER_MESSAGES, overflow/past-limit refuses the WHOLE batch loud and mints NOTHING) and the immutable reservation (first + len fixed at mint) fronts per-item bulk work (encrypt/sign) on ANY thread in ANY order via disjoint contiguous sub-range splits; the regime precondition is a receiver tolerating gaps/reorder (the replay window — REJECT_AFTER_MESSAGES's own doc IS the regime made constant: 'The exclusive counter limit, leaving room for the replay window's arithmetic.'); the source's offload contract verbatim: 'The reserved counters fix the nonces, so encryption can run on any thread and in any order.'; blog-measured provenance, never bars: begin_encapsulate 20.8 ns vs encrypt 933 ns @1280B ('the bookkeeping part ... only 2%') with the blog's own 45x hedge carried as the honest upper bound; the Send honesty note in-rule (the blog says 'implements Send' but NO explicit impl exists at the pin — auto-implied); the checked_add idiom is a POSITIVE INSTANCE of sec's `unchecked-wire-arithmetic` (cited, never re-owned); distinct-from the B283 certifying-token rule (validation-SKIP vs offload-BULK); candidate 5 (reject-before-expensive-sink) DROPPED at T0 (the B258 invented-check class) and candidates 2-4 landed sec-side (the Batch-327 sec trio, see sec.md). Fixtures both arms standalone-compiled AND executed (buggy 4/4, fixed 5/5, zero warnings) before entry. Gates: batch327 needles+parse 2/2; count pins via the manifest pass; retrieval floors re-measured at the landing; score-bench appended; release-set binary REBUILT + installed (rust_perf IS release-set). Prior — Batch 324 +3, Plan 328 / Research 362, 2026-11-02 — the sccache io/cache/process distill, entries_58, upstream mozilla/sccache @ `71d107e` Apache-2.0, io +3: `pre-exec-disables-posix-spawn` (BENCHABLE with a bench_spec — the GOAT lead: ANY `pre_exec` closure on a std::process::Command makes std abandon the `posix_spawn` fast path for fork+exec, and fork prices against the PARENT's RSS — the law: fd-inheritance setup is PER-CHILD OPT-IN (`share_jobserver` default-off at src/mock_command.rs:260-282, the mechanism comment verbatim: 'any `pre_exec` makes `std` fall back from `posix_spawn` to `fork`+`exec`'; MEASURED on amo/M5 Ultra run-ALONE, parent pinned 256 MiB per the portability caveat: 400 spawns x 8 interleaved rounds best-of = plain 0.642 ms/spawn vs pre_exec 1.683 ms/spawn = 2.62x; the standalone paired-median validator probe reproduced 2.62x) + `reserve-then-commit-capacity-accounting` (NotBenchable decision-content — the byte-budgeted store's two-phase insert: reserve the ADVERTISED size under the ledger, write lock-free, commit RE-MEASURES the real size and evicts only the delta (lru_disk_cache/mod.rs:327-367 prepare_add/commit verbatim); the concurrency witness: 97 overshoot events / 200 inserts on the insert-first arm vs a held budget on the reserved arm; distinct from exclusive-mkdir-temp-resource-reservation — an in-memory CAPACITY LEDGER, not an external-resource claim) + `mtime-encoded-lru-recency-persistence` (NotBenchable decision-content — LRU recency stored AS the filesystem: every hit touches the file's mtime (`set_file_times(&path, t, t)`), boot init sorts by mtime ('Sort by last-modified-time, so oldest file first.'), zero metadata file; the cross-RESTART axis distinct from tier-hit-refreshes (cross-tier) + slru-probation (within-cache); two regimes: single-process long-lived owner = the touch is pure syscall cost (in-memory LRU wins), crash/restart- or multi-process-shared cache = mtime IS the recency store); the io-copy-write-adapter candidate DROPPED POSITIVE-INSTANCE at the probe (the A/B measured the fixture's allocator noise, not a transferable fix decision — the drop recorded in the entries_58 header); the get-with-raw-single-read-parse candidate (B−) DEFERRED to a future io batch. Fixtures 5 arms standalone-compiled AND executed (probe binaries green) before entry. firezone/sanstun @ `35cb1932` Apache-2.0 (the sans-IO WireGuard protocol implementation; the pin is the tip squash of #129 'feat!: reserve counters and decrypt data messages in batches', 2026-11-03 — brand-new sanstun-original surface), concurrency +1: `batch-reserve-monotonic-counters-offload-bulk` (NotBenchable-by-domain — the coordination-law annotation class: N per-item synchronized counter trips collapse to ONE bound-checked range reservation (the count checked_add-ed, the end filtered against REJECT_AFTER_MESSAGES, overflow/past-limit refuses the WHOLE batch loud and mints NOTHING) and the immutable reservation (first + len fixed at mint) fronts per-item bulk work (encrypt/sign) on ANY thread in ANY order via disjoint contiguous sub-range splits; the regime precondition is a receiver tolerating gaps/reorder (the replay window — REJECT_AFTER_MESSAGES's own doc IS the regime made constant: 'The exclusive counter limit, leaving room for the replay window's arithmetic.'); the source's offload contract verbatim: 'The reserved counters fix the nonces, so encryption can run on any thread and in any order.'; blog-measured provenance, never bars: begin_encapsulate 20.8 ns vs encrypt 933 ns @1280B ('the bookkeeping part ... only 2%') with the blog's own 45x hedge carried as the honest upper bound; the Send honesty note in-rule (the blog says 'implements Send' but NO explicit impl exists at the pin — auto-implied); the checked_add idiom is a POSITIVE INSTANCE of sec's `unchecked-wire-arithmetic` (cited, never re-owned); distinct-from the B283 certifying-token rule (validation-SKIP vs offload-BULK); candidate 5 (reject-before-expensive-sink) DROPPED at T0 (the B258 invented-check class) and candidates 2-4 landed sec-side (the Batch-327 sec trio, see sec.md). Fixtures both arms standalone-compiled AND executed (buggy 4/4, fixed 5/5, zero warnings) before entry. Gates: batch327 needles+parse 2/2; count pins via the manifest pass; retrieval floors re-measured at the landing; score-bench appended; release-set binary REBUILT + installed (rust_perf IS release-set). Prior — Batch 324 +3, Plan 328 / Research 362, 2026-11-02 — the sccache io/cache/process distill, entries_58, upstream mozilla/sccache @ `71d107e` Apache-2.0, io +3: `pre-exec-disables-posix-spawn` (BENCHABLE with a bench_spec — the GOAT lead: ANY `pre_exec` closure on a std::process::Command makes std abandon the `posix_spawn` fast path for fork+exec, and fork prices against the PARENT's RSS — the law: fd-inheritance setup is PER-CHILD OPT-IN (`share_jobserver` default-off at src/mock_command.rs:260-282, the mechanism comment verbatim: 'any `pre_exec` makes `std` fall back from `posix_spawn` to `fork`+`exec`'; MEASURED on amo/M5 Ultra run-ALONE, parent pinned 256 MiB per the portability caveat: 400 spawns x 8 interleaved rounds best-of = plain 0.642 ms/spawn vs pre_exec 1.683 ms/spawn = 2.62x; the standalone paired-median validator probe reproduced 2.62x) + `reserve-then-commit-capacity-accounting` (NotBenchable decision-content — the byte-budgeted store's two-phase insert: reserve the ADVERTISED size under the ledger, write lock-free, commit RE-MEASURES the real size and evicts only the delta (lru_disk_cache/mod.rs:327-367 prepare_add/commit verbatim); the concurrency witness: 97 overshoot events / 200 inserts on the insert-first arm vs a held budget on the reserved arm; distinct from exclusive-mkdir-temp-resource-reservation — an in-memory CAPACITY LEDGER, not an external-resource claim) + `mtime-encoded-lru-recency-persistence` (NotBenchable decision-content — LRU recency stored AS the filesystem: every hit touches the file's mtime (`set_file_times(&path, t, t)`), boot init sorts by mtime ('Sort by last-modified-time, so oldest file first.'), zero metadata file; the cross-RESTART axis distinct from tier-hit-refreshes (cross-tier) + slru-probation (within-cache); two regimes: single-process long-lived owner = the touch is pure syscall cost (in-memory LRU wins), crash/restart- or multi-process-shared cache = mtime IS the recency store); the io-copy-write-adapter candidate DROPPED POSITIVE-INSTANCE at the probe (the A/B measured the fixture's allocator noise, not a transferable fix decision — the drop recorded in the entries_58 header); the get-with-raw-single-read-parse candidate (B−) DEFERRED to a future io batch. Fixtures 5 arms standalone-compiled AND executed (probe binaries green) before entry. Gates: batch324 needles+parse 3/3; default lib 2800/0; rust_perf 2800/0; rust_perf_bench 2831/0; clippy −D ×4 postures 0; retrieval floors HELD (latent 2996/0, no re-pin); score-bench run #219 heal 0.9636 FLAT parse 1.0; release-set binary REBUILT + installed (rust_perf IS release-set). Prior — Batch 300 +2, Plan 305 / Research 341, 2026-10-31 — the ConstexprCore/perfect_hash rust_perf micro-layer, entries_57, upstream ConstexprCore/perfect_hash @ `4fdf1b7f` MIT, caching +2 BOTH BENCHABLE with a bench_spec each: `fold-key-len-into-one-word-compare` (head+tail+len folded into one u64 — the length rides the compared word, the coverage law bounds the regime at <= 4 bytes; the T0b llvm-ir falsification probe ran FIRST: the byte-loop arm stays scalar with per-byte branches, the folded arm is 2 u16 loads + one compare, ~2.6x interleaved A/B — NOT the oxc positive-instance kill) + `fused-value-in-spare-lane-bits` (the small value rides the key lane's spare high bits, masked out of the equality — one load returns key+value, the value array leaves the hot footprint; the two-regime gate is the rule) on the prior state: Batch 318 +2, Plan 318 / Research 356, 2026-10-30 — the impeccable io stretch-tier remainder (plan 316's `- [-]` rows), entries_56, upstream pbakaus/impeccable @ `02523aec` Apache-2.0, concurrency +1 io +1 BOTH ANNOTATED NotBenchable bench_spec None: `generation-counter-stale-timer-invalidation` (crates/live/src/server_state.rs schedule_lease_flush — a detached timer thread cannot be cancelled: JS rescheduling cancels the prior timer, a spawned thread cannot, so the port supplies the clearTimeout itself — the generation bumped under the state lock on EVERY schedule ('self.lease_timer_gen += 1;' + 'self.lease_timer_active = false;'), captured at arm time AFTER the bump ('let gen = self.lease_timer_gen;'), re-checked after the wake ('if st.lease_timer_active && st.lease_timer_gen == gen {' — a superseded timer exits WITHOUT acting), the state held through 'self_ref: Weak<Mutex<ServerState>>' so an owner that drops first makes the upgrade fail (no resurrection, no shutdown delayed by a sleep); the pattern is the file's timer DISCIPLINE — four gen counters guard its detached-timer sites; fixtures: a LeaseBoard with a flush log + Drop witness — the buggy arm's superseded timer fires (2 flushes for 1 owed) and its strong capture holds the state past the owner's drop, the fixed arm leaves exactly the fresh flush and the orphan exits on a failed upgrade) + `exclusive-mkdir-temp-resource-reservation` (crates/browser/src/cdp.rs reserve_profile — the law is the comment verbatim: 'A timestamp is not a reservation: concurrent calls can observe the same tick.' + 'Exclusive mkdir also avoids reusing a stale profile from a previous process.' — static NEXT AtomicU64, a bounded 128-attempt loop, the serial from fetch_add(Relaxed), the name carrying pid + stamp + serial, create_dir the exclusive claim (Ok = owned the create; AlreadyExists => continue; other errors real; exhaustion errors); fixtures: a fixed injected clock — the buggy arm's two same-tick callers share one dir and clobber (create_dir_all) and a previous run's leftover is handed out again, the fixed arm's eight concurrent callers at one stamp get eight DISTINCT dirs and the deterministically-collided leftover is skipped untouched). Fixtures 4 arms standalone-compiled AND executed (8/8 green, timing-stable) before entry. Gates: batch318 needles+parse 2/2; default lib 2792/0; rust_perf_bench 2823/0; clippy −D ×4 postures 0; retrieval floors HELD at latent_retrieval (2984/0); STALE-PIN REPAIRS EN-ROUTE (the B239/B207 pin-hiding class — the pins B316's own landing missed): len 285→287 at 5 witness sites the B316 landing did NOT list (tests.rs 7494/9115/9155/9212/9261 — the era-witness asserts) + annotated 232→234 ×3 (the serde round-trip pin at 2031 among them) + mod.rs fn doc 280→287 (the B316 lane repaired the module header but MISSED this fn doc, still at its B308-era 280 — the missed-pin class, disclosed in the doc itself); freeze/score-bench NOT owed (P30 — rust_perf-only delta); release-set binary REBUILT + installed at the landing HEAD (rust_perf IS release-set; --info prints rust_perf 287). Prior — Batch 316 +5, Plan 316 / Research 356, 2026-10-30 — the impeccable io/process robustness pool, entries_55, upstream pbakaus/impeccable @ `02523aec` Apache-2.0, io +4 concurrency +1 ALL ANNOTATED NotBenchable bench_spec None: `per-read-timeout-vs-request-deadline` (the A− lead: SO_RCVTIMEO bounds ONE blocked read and any byte resets it — 'a slow-drip or silent peer keeps each read returning Ok(1) ... pinning the connection's ticket and wedging every later mutation behind it'; the fix arms the total deadline at BOTH layers — the socket timeout set to the deadline ('Windows does not unblock a `recv` already parked in the kernel when another thread calls `shutdown` on the same socket') + the 50 ms-poll watchdog calling shutdown(Both) — plus the prior-timeout restore; the SEC CROSS-ROUTE is recorded IN-RULE (the B293 precedent): the slow-drip shape is slowloris-class, a later sec batch routes here, never re-mines; PRODUCT COVERAGE VERIFIED AT HOME: instinct serve_edge.rs:943 + reflex serve.rs:1197 carry the exact pre-fix posture — issues filed in both repos) + `child-piped-stderr-drain-for-process-lifetime` ('Drain stderr for the life of the process so a chatty browser never blocks on a full pipe' — cdp.rs; the two-posture contrast IN-SOURCE: svelte_bridge.rs's deliberate Stdio::null for a quiet helper) + `kill-without-wait-zombie` (kill+wait paired EVERYWHERE in-source incl. the Drop 'Belt and braces for the error paths that skip close()'; the HOME FIRING DATUM is this repo's own l4_daemon.rs:111 EOF path — the other three sites adjudicated paired) + `connect-timeout-walk-over-resolved-addrs` ('TcpStream::connect over every address host:port resolves to, each with a deadline' — cdp.rs connect_tcp; home precedent seat_rerank.rs:331 already walks) + `arrival-order-turnstile-lane` (the FIFO ticket lane serializing state mutations in ARRIVAL order — the Node end-callback motivation, the 404 unknown_session race; 'There is no timeout: a checkpoint waits for the earlier generate however long that generate's body takes' — the wait is unbounded ON PURPOSE, bounded upstream by the request deadline; poison recovery via into_inner); +1 count-neutral rider face: the BOX-FILTER AREA-AVERAGE face on `prefix-sum-2d` entries_01 (crates/comp/src/raster.rs: 'Area averaging shrinking, bilinear growing' — the same 2D-window-sum family, the grow path bilinear untouched). Quotes byte-verified in the .raw clone HEAD-equal to the pin this session. Gates: batch316 needles+parse 3/3; default lib 2790/0; rust_perf_bench 2821/0; clippy −D ×4 postures 0; stale-pin repairs en-route: len 280→285 ×5 historical sites + annotated 227→232 ×3 + NotBenchable 158→163 ×1 (the B239/B207 pin-hiding class — the rust_perf_bench-only pins sat stale through B308's default-only gates); freeze/score-bench NOT owed (P30 — rust_perf-only delta, verify at the landing window); release-set binary REBUILD owed at the landing HEAD (rust_perf IS release-set). Prior — Batch 308 +2, Plan 313 / Research 352, 2026-10-30 — the 10-29b staged serving pool + rust_perf row, entries_54: caching `tier-hit-refreshes-lru-recency-at-owning-tier` (vllm-project/vllm @ `c1b7dffa` #62444, Apache-2.0 — the CROSS-TIER recency-coherence law: a GPU-tier prefix hit bypassed the CPU store path so the CPU LRU never saw the touch and evicted in the wrong order; the hit is recorded at admission into a pending map, touched + re-freed at the MRU end at the serving step, the map drained on every terminal path; "Eager CPU is inclusive: a GPU prefix hit also uses the CPU copy, but eager stores skip it, so move it to the MRU end here"; the corpus's touch/promote family is within-one-cache — this is coherence BETWEEN tiers) + caching `refusal-quarantines-entry-transient-passes` (ashhart/tensorfold @ `d1fcb9d` #1539, Apache-2.0 — a store/scatter REFUSAL (SnapshotSize/BadKeptRange/DiskWords/KeptWordsDiffer) is a failed check of the ENTRY: one `refused()` switch routes it to bad() (counted, logged with the file named, unlinked ONCE, every naming state dropped, the read-ahead entry dropped AHEAD of the removal), while transient errors pass through un-quarantined keeping the state and its read; "a state that is gone takes its read ahead with it"); +2 count-neutral rider faces: the REMOTE-writer ack arm on kernel_opt gpu_111 `drain-device-work-before-free` (vllm #64256 `16777da` — a timed-out P2P load fails at once but its primary chunk stays out of the free pool until the peer's AbortAck; a missed ack faults the session sticky) + the CONSUMING-SIZE face on entries_45 `admission-tests-the-bound-the-admitted-mode-serves` (llama.cpp #31350 `7df8b87` — the admission tested n_rollback (derived) where the recurrent-snapshot guarantee is scoped to n_draft (the consuming size)). Quotes byte-verified at the per-commit .patch pins (the PR-bundle SHAs of #62444/#64256/#31350 moved vs the staged pins — rebases; the per-commit patches are the verification surface). Gates: batch308 needles+parse 2/2; default lib 2787/0; rust_perf_bench 2818/0; kernel_opt 3312/0; clippy −D ×3 postures; retrieval floors HELD both postures (27/0 + 27/0; perf self-variants 92.9/94.6/98.2, blended 87.9/92.2/94.3 — above the 77.1/86.4/92.4 floors, no re-pin); freeze/score-bench NOT owed (P30 — rust_perf-only delta); release-set binary REBUILT + installed at the landing HEAD (rust_perf IS release-set). Count pins: tests.rs len 280 (5 sites) + annotated 227 (3 sites — the B298/B305-era pins sat STALE at 222/225, repaired to the measured truth en-route) + NotBenchable 158 + benchable 122 UNCHANGED + the kernel-side witness 280; AGENTS ×1 + README + this doc + mod.rs doc (280, the B305 miss repaired en-route). Prior — Batch 288 +1, the grammar explicit-stack rule; 14th category `measurement` since Batch 284) — Batch 274 (Plan 278 / Research 312, 2026-10-25 — riding the uzu trellis batch, Plan 278): **corpus 266→267** — allocation `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (incoai/splash @ `28d0e98` "Refuse a weight-digests load the Metal memory budget cannot hold", Apache-2.0 — a batch load admits against the planner's HARD budget BEFORE the first unit loads: the count sized from FILES, each streaming store at min(SIZE, part whole) — the planner's own resolve; the refusal names both counts + the streaming options; no false refusals since `--max-memory` only lowers the ceiling; NotBenchable annotated, bench_spec None, Grade B+ context_notes, fixtures standalone-compile 5/5 + 7/7) + 1 count-neutral rider: the TRANSIENT-SHORTFALL face on entries_41 `refusal-wait-keyed-on-shortfall-returning-transfer` (splash `e88a0d0` — see the kernel_opt.md Batch-274 section). 7 honest drops at T0: `1bf2f1a` (intra-kernel DRY refactor, no serving decision), `0341410` (parser-acceptance compat), `2c645f3` (comments only), `772ba56` (start-failure reason = diagnostics/UX), `84b0e16` (test-oracle honesty, not serving-path), `720f5fd` (= the PR-merge of `7b21db3`, not independent), `9ff0d7e` (count-neutral build-fix). Gates: rust_perf lib green in the full-lib runs; the cross-domain pin (kernel-side `batch249_slice3_rust_perf_riders`) re-pinned 266→267; release-set binary REBUILT + installed at the landing HEAD (rust_perf IS release-set — the B255 lesson). Records: Plan 278 / Research 312.

Batch 282, 2026-10-25, Plan 286 / Research 319 — +3 the PerfR C-to-Rust translation classes (arXiv 2610.23044v1, Yuan/Wang/Zhou/Liu/Peng, Fudan + NTU, submitted 2026-10-16; the paper's nine recurring classes over 20 real C projects + c2rust translations pair an Observed Pattern with an Optimization Direction — the corpus drafter+pruner+verify shape, and the paper's two-gate validation (incremental 1.0% hot-fn-weighted + cumulative 0.3% aggregate + severe-regression guard, paired interleaved runs CV<=3%, Tukey 1.5x IQR) mirrors rust_perf_bench Pareto + the Issue-940 determinism fold-order pins; entries_48, ALL ANNOTATED NotBenchable bench_spec None, fixtures ORIGINAL standalone-compiled+executed 6/6 green): arithmetic `saturating-f2i-cast-proven-range-to-int-unchecked` (class 2 Redundant Conversion Saturation — a range-PROVEN float→int `as` cast still pays llvm.fptosi.sat per evaluation; `to_int_unchecked` drops it; the heman fastFloor instance; UNSAFE-adjacent → suggest-only BY CONSTRUCTION, proof obligations in context_notes) + dispatch `loop-invariant-dispatch-resolution-hoist` (class 5 Repeated Invariant Dispatch — an invariant handler/mode re-resolved per iteration because the opaque indirect call's clobber assumption blocks LICM; resolve once before the loop; the mode-specialized-loop face disclosed manual) + iterators `raw-pointer-traversal-slice-view-recovery-load-elimination` (class 9 Obscured Memory Access Properties — raw-pointer traversal hides extent + aliasing so GVN/LICM cannot eliminate/hoist loads; libzahl zrsh 73 vs 24 static loads, TBAA missing, 46-vs-16 GVN remarks; recover `&[T]`/`&mut [T]` views; the paper's MOST WIDESPREAD rule at 89% of hot functions, 46/62 accepted rewrites; UNSAFE-adjacent → suggest-only) + 3 count-neutral rider faces: TRANSLATION-DROPS-ATTRIBUTES on `inline-hint-not-through-unmarked-wrapper` (class 4, 16% of hot fns) + vectorization-BARRIER on `simd-chunk` (class 3, 24%) + BOUNDS-CHECK-ELIMINATION on `chunks-exact` (class 1, 18%); 3 honest drops at T1: class 6 (arena/pool/capacity families own it), class 7 (byte-oriented — c2rust-specific; copy/byte-order faces owned), class 8 (chunk/block families own it); riders live in corpus DATA (name/context_notes) never comments — the batch282 gate's hay-closure caught the comment-only first attempt. Gates ALL GREEN: batch282 needle gate (incl. the nine paper-class-name negative asserts), default lib 2730/0, rust_perf_bench lib 2761/0, clippy −D ×3 postures, retrieval floors HELD (latent lib 2916/0, no re-pin), freeze gen2+gen3 e2es PASS no table movement (P30), score-bench #199 heal 0.9636 FLAT parse 1.0, release-set binary REBUILT + installed (--info serves rust_perf 270).

Batch 271, 2026-10-21, Plan 275 / Research 307 — +12 the splash-wave SECOND-TIER serving pool (the B268/B270 staged second tier converted, incoai/splash author Jian Chen, Apache-2.0; entries_43-46, four authoring slices; ALL NotBenchable annotated; +4 count-neutral rider faces: 253506f spill-tier victim-exclusion + 4fc7465 durable-label-at-reclassification on restore-point-chain-copies-keyed-on-disk-state-count, 4d17995 task-scoped-read-buffer (the FOURTH retention policy) on reuse-allocation-clear, 593f2a6 anchor-bound on slice A's own rule): allocation +3 resume-admission-sized-to-per-command-window-not-suspended-peak (@ fa63d97) + phase-scoped-staging-released-at-successor-reservation-when-plan-budgets-max (@ 88d0e57) + derived-reservation-taken-once-after-admission-settles (@ 81ae003); dispatch +3 size-class-keyed-reader-cap-bimodal-reads (@ 4d17995) + fixed-read-command-runs-alone-sized-by-its-own-reads (@ 68b9e70) + reject-plan-runs-before-setup-with-outliving-side-effects (@ dab8841); dispatch +2 admission-tests-the-bound-the-admitted-mode-serves (@ 5ea4043+82b93c5) + in-use-state-retry-under-serving-mark-keyed-on-refusal-cause (@ b7844dc); concurrency +1 one-event-per-writer-consumer-waits-on-the-side-it-needs (@ e2c0af2) + finished-command-copy-leaves-at-once-slot-returns-at-last-reader (@ 91d4ddb); complexity +2 part-cost-measured-by-planning-ablation-not-component-sum (@ ed1092f) + plan-input-precomputed-per-output-variant-bounded-convergence (@ 2f5fe8d); drops at the T0 re-read: 6fcd9b1 (error-copy), 25502fd (DRY 'Effect: none'), 7e51fcb (error-copy); retrieval floors HELD 22/22 no re-pin; gen2+gen3 freeze e2es PASS no table movement; clippy -D clean ×4 postures (the 4th, kernel_opt+latent, swept 3 PRE-EXISTING B270 residues: batch251/batch268 gates' too_many_lines + retrieval_eval doc_markdown); score-bench #198 heal 0.9636 FLAT parse 1.0; allocation 38→41, concurrency 30→32, dispatch 23→28, complexity 33→35). Batch 270 (2026-10-21, Plan 274 / Research 306 — +12 the splash-wave serving pool (incoai/splash @ the staged window, Apache-2.0; the B268 Slice-B staged pool converted; entries_39-42, four authoring slices; ALL NotBenchable annotated; +4 count-neutral rider faces): caching +6 restore-point-chain-copies-keyed-on-disk-state-count + persistent-cache-teardown-needs-second-witness + hot-slot-recycler-flat-tables-zero-alloc-claims + cache-share-capped-at-measured-distinct-working-set + cache-reuse-assertion-gated-on-eviction-counter + publication-takes-over-condemned-cache-entry; allocation +1 reclaim-progress-keyed-on-refused-resource-returned; concurrency +1 refusal-wait-keyed-on-shortfall-returning-transfer; dispatch +2 watchdog-deadline-moves-on-command-progress + constrained-unconstrained-lanes-one-decode-command (the f52d1ad LAND verdict, -23.7%/-34.0% time/token source-reported); io +2 page-miss-blocks-one-ranged-read-scatter + tier-write-defers-to-urgent-reads-except-a-read-behind-it; retrieval floors HELD 22/22 no re-pin (blended 90.6/94.2/95.3); gen2+gen3 freeze e2e PASS no table movement; score-bench #197 heal 0.9636 FLAT parse 1.0; caching 25→31, allocation 37→38, concurrency 29→30, dispatch 21→23, io 31→33; the second tier (13 B/B− candidates) stayed staged for Batch 271 — landed above). Batch 266 (2026-10-21, Plan 270 / Research 302: +1 complexity — `isop-cover-from-decision-diagram-not-path-simplification`, the decision-diagram FAMILY SEED from astral-sh/uv PR #22651 @ `551c17cbc` (MIT OR Apache-2.0): the memoized Minato–Morreale ISOP cover extraction replaces enumerate-paths-then-quadratic-simplify; 2 count-neutral rider faces (shared-Cover intermediate; the (lower, upper) don't-care interval memo key); GOAT 19.02×/12.93×/5.61× at 1K/4K/16K ParetoDominant 3/0/0, M3 --release, PROVENANCE powermode=2(high) load=3.73; upstream 1.5×–11,900× source-reported with the −5.5% tiny-marker anti-regime carried in-rule; alloc trade disclosed (fixed 1.22M allocs/73 MB vs buggy 215K/17 MB at max size — the memo tables buy the time win); the fixture is original machinery, exhaustive-oracle-validated (equivalence at n≤12 + prime/irredundant + roundtrip) BEFORE landing; retrieval floors HELD (21/21) + self-variant top-1 pinned; score #196 flat; freeze not owed; complexity 32→33). Prior: Batch 257 (2026-10-20, Plan 261 / Research 293: +6 the TensorFold staged remainder at pin `d1018b31` — `first-message-sized-to-smallest-job` (dispatch, `10ff8779`, measured 165,440 → 10,560 B/step) + `draft-cost-timed-at-served-widths-not-extrapolated` (dispatch, `abb860c5`, "twice the truth") + `checkpoint-reads-skip-page-cache-with-refusal-fallback` (io, `221c2b5d`+`6bb9daac`) + `reserve-stream-caches-at-admission-warm-request-paths-at-load` (allocation, `84c58d3f` sibling) + `cache-budget-first-sessions-take-remainder-never-reshrink` (allocation, `406b2090`) + `kept-prompt-states-pinned-host-cache-budget` (caching, `a739444c`), ALL NotBenchable); Batch 256 (2026-10-19, Plan 260 / Research 291: +3 the TensorFold `9ccf4727..d1018b31` +988-commit campaign (the 1.1.0/1.1.1 Zig waves), rust_perf half — `persistent-digest-memo-stat-identity-keyed` (caching, ashhart/tensorfold @ `018e73c6`+`20bca340`, Apache-2.0, the NAMED lead: the on-disk file-digest memo keyed by full stat identity device/inode/size/mtime/ctime-ns, ≤1,024 records, versioned + checksummed, atomic publication; a corrupt index refuses LOUD (error.CorruptDigestCache) — never a trusted value, never a silent recompute; only absent/foreign-key indexes recompute; the mtime-restored rewrite must miss — ctime joins the key) + `launch-cost-scales-with-allocation-count-pack-one-arena` (allocation, `84c58d3f`: per-launch driver overhead grows with the process allocation count — 1.2 ms/launch over 1,900 weight buffers — so weights+scratch pack one arena through a reused staging buffer, load 25 s vs 200 s, RTX 4090 source-reported) + `pinned-entry-tombstone-freed-at-last-unpin` (concurrency, `8c6b40a3`+`3aedf373`: the 4-face pin lifecycle — pin-at-lookup/unpin-at-report across async ops, eviction skips pinned, removal-while-pinned marks dead (no lookup finds it, the last unpin frees), over-commit fails loud RestoresPending never force-frees); ALL ANNOTATED NotBenchable; the kernel half (F+A+D+G) recorded in kernel_opt.md; floors HELD 90.6/94.2/95.3 kernel 100% no re-pin; freeze NOT owed; score-bench #191 flat). Batch 255 (2026-10-19, Plan 259 / Research 290: +1 the llama.cpp MoE SLRU host-cache law — `slru-probation-protected-promote-on-second-touch` (caching 17→18, ggml-org/llama.cpp @ `33d6135c` #30637, MIT, ANNOTATED NotBenchable): a recency cache of EXPENSIVE entries (eviction costs a megabyte-scale refill) runs a 50/50 SLRU, not one plain LRU list — first touch = probation (`move(s, false)` on fill), second touch promotes (`touch` → `move(s, true)`) with protected capped at n_slots/2 and overflow demoting the protected-LRU head; victims scan probation-head-first then protected while the request-scoped seen-set pins the current request's own selections (a miss must not evict a slot this request still needs); hits promote BEFORE victims are chosen; `GGML_ABORT` is the loud impossibility terminal; distinct from B103 evict-oldest (the anti-LRU verdict for zero-cost-rebuild contexts — the opposite regime) and B241 empty-slot-first-victim-order (the victim order WITHIN one list); NotBenchable — the upload-traffic win metric, the B136 decision-content class; the with_spec pin repaired 119→120 — B250 slice 1's +1 benchable never re-pinned it, PROVEN at HEAD). Batch 250 slice 1 (2026-10-18, Plan 254 / Research 285: +1 the llama.cpp envelope pool's hadd butterfly — `avx2-tiled-accumulator-butterfly-epilogue` (simd 12→13, ggml-org/llama.cpp @ `3dbcb7f` #30296, MIT, ANNOTATED; reduce a fixed register-resident accumulator bank by an intra-register `_mm256_hadd_epi32` butterfly (pairs → hadd → `permute2x128` lo/hi → one add, five vector ops for the whole bank) replacing 8× scalar `hsum256_epi32` store+reload round-trips; distinct from the broadcast-FMA GEMV family (entries_03 — that removes the horizontal reduce from the LOOP via transposed layout; this is the EPILOGUE when per-column accumulators are already live in registers) and from the fwht transform butterflies (many-to-one exchange network vs many-to-one convergence); the reassociation caveat in-rule — local-to-the-tile reductions only, never a synced/order-pinned fold (the determinism law); x86 Pareto CLOSED 2026-10-19: PARETO-DOMINANT ×3 on the 4090 box, 1.02–1.35×, allocs 0→0 (plan 254 T6; the first run caught the fixture's row-vs-bank parity defect pre-timing, fixed `435d853c`; this paragraph's PENDING-x86 token was stale through the T6 close — repaired at the B255 landing); retrieval floors HELD no re-pin; freeze NOT owed; score-bench #189 FLAT). Batch 248 (2026-10-16, Plan 252 / Research 280: +6 the staged simdutf + unicode-normalization conversion — Research 275's follow-up pool items 3+4, the last two staged sources — `minimal-perfect-hash-static-lookup` (caching 16→17, the two-level salted MPH with the packed 24-bit key + 8-bit value in one u32 and ONE exact key compare — the fourth rung on the static-lookup ladder answering range-table-page-memoization's random-query anti-regime; GOAT 5.26-12.99x verdict-stable x4, alloc 0/0) + `flat-pool-start-len-static-rows` (layout 16→17, one flat static pool + packed (start,len) handles, zero-copy rows; NotBenchable design-time layout) + `dense-subdomain-arithmetic-before-table-lookup` (dispatch 15→17, the Hangul closed-form block answered by arithmetic BEFORE the sparse table — compose_hangul(a,b).or_else(composition_table) IS the law; GOAT 1.93-3.58x) + `fanout-transform-emit-callback-not-vec` (allocation 29→30, the F: FnMut(char) emit shape, one monomorphized driver, zero per-element allocations; GOAT 4.06-15.82x, allocs 860→1/13975→1/224660→1) + `inline-buffer-index-shift-over-drain` (dispatch, the source's own 'faster than drain()' law on the inline buffer — the harness on ArrayVec the always-inline member, the authoring tie-risk RETIRED by measurement; GOAT 1.27-2.16x) + `shift-mask-adjacency-check` (simd 11→12, the vendored C++ amalgamation's surrogate pairing — pairwise adjacency as shift-AND on lane masks + fold-and-compare + the all-but-last-lane boundary escape; GOAT 1.27-2.68x) — ALL annotated + the count-neutral BOUNDED-ARITY rider on smallvec-stack (replace.rs:20-22, the arity constant IS the capacity argument of record; that rule gains its FIRST context_notes — annotated +1, unannotated -1) — DROPPED at measurement: deferred-error-mask-block-check (ParetoDominated 0/3/0, 0.64-0.73x, after one honest restructure — the scalar mask build is extra work the branchy carry never pays; the sample-parity instrument caught a shift-direction fixture bug 1-vs-25 pre-measurement, the B245 lesson working); the simdutf DEFER list (signature-ladder, pack-plan-table, popcount-lane, pair-key, extremum, margin) recorded in Research 280; retrieval floors HELD no re-pin (20/0/1i); gen2+gen3+gen4 freeze e2e ok; score-bench run #186). Batch 246 (2026-10-16, Plan 250 / Research 278: +1 the prism fork delta, the serving state-cache family's FORK face — `seq-fork-carries-pending-rollback-record` (caching, PrismML-Eng/llama.cpp @ `5ae2e169e` #354, MIT) — seq_cp shared the cell but not the pending-rollback record rs_idx, so 'a sequence forked right after a rollback read the state from before the rollback'; the fork copies the record, and for snapshot-plane state copies the SNAPSHOT plane the rollback points to into the destination's current plane (never the live plane); plane-count sizing n_stream*(1+n_rs_seq) in size_t arithmetic; the bitwise fork-vs-direct-rollback state oracle; NotBenchable annotated; caching 15→16; retrieval floors HELD no re-pin (90.6/94.2/95.3); gen2+gen3 freeze e2e ok; score-bench run #185). Batch 244 (2026-10-16, Plan 247 / Research 276: +10 the text/unicode family's FOLLOW-UP — the byte-scan/search mechanism batch — from three sources at release pins `rust-lang/regex` @ `regex-automata-0.4.18` + `BurntSushi/memchr` @ `2.8.3` + `BurntSushi/bstr` @ `1.13.1`: `byte-class-equivalence-table-shrink` (layout, the byte-class law — equivalence-class the key space before the table; GOAT 1.39-1.39x on a 1024-state fixture, the first 24-state fixture Mixed — the table must exceed L1) + `prefilter-choice-shape-ladder` (dispatch, the cheapest-first accelerator ladder; GOAT 44-46.6x) + `alternation-prefix-collapse-at-build` (complexity, the common-prefix collapse + shadowing + ATTEMPTS ladder; GOAT 334.8-639.7x) + `byte-rank-select-rarest-probe` (caching, the MERGED regex-syntax rank() + memchr packedpair law, same 250 threshold; GOAT 22.9-38.1x) + `prefilter-skip-density-demotion` (dispatch, the PrefilterState feedback-control law — the corpus's FIRST measure-and-demote rule; GOAT 3.76-5.40x, the first scalar-probe fixture Dominated) + `iter-count-bulk-specialization` (iterators, the Iterator::count override; GOAT 9.19-9.28x) + `mod64-byte-membership-skip` (caching, the Two-Way ApproximateByteSet; GOAT 5.27-5.84x) + `byteset-cardinality-dispatch` (dispatch, memchrN for 1–3 vs the stack table for ≥4; GOAT 1.34-1.58x, the first 10%-density fixture Dominated) + `utf8-fast-lane-cold-diagnostics` (dispatch, the fast/cold diagnostic lane split; GOAT 8.60-10.33x) + `bufread-scan-in-buffer-scratch-fragment` (io, the zero-copy fill_buf drain; GOAT 2.77-3.63x, alloc 10892→0), ALL benchable + annotated; layout 14→15, caching 13→15, dispatch 9→13, complexity 32→33, iterators 11→12, io 26→27; the memchr-rank + regex-rank laws MERGED; DEFERRED to Batch 245 with reasons (8 leads); DROPPED sentinel-run-word-broadcast-skip + cross-product-budget-degrade + the memchr-3.x suspects (absent at the pin); retrieval floors HELD no re-pin (125/131/133 vs 124/130); gen2+gen3 freeze e2e ok; score-bench run #184). Batch 243 (2026-10-16, Plan 245 / Research 275: +2 the text/unicode processing family seed — the corpus's founding member of that family — `ascii-run-swar-block-skip` (simd, the high-bit-gather block mask; GOAT 6.41-7.78x) + `range-table-page-memoization` (caching, the lazy per-page predicate cache; GOAT 9.82-10.01x), both benchable + annotated, from Hugging Face tokenizers @ `5b5a3fcc` (branch `perf/faster-unicode-normalizers`, Apache-2.0); the third lead (transform-borrow-until-first-change) dropped dominated 0/3/0 twice — the negative in Research 275; retrieval floors re-pinned with attribution + gen2/gen3 freeze-tables regenerated). Batch 241 (+2 the serving slot-state cache pair #30580+#30567, both caching 10→12; + the count-neutral draft-state rider on P65 #30574; the row previously read 201 — stale through B239's +2, repaired here). Batch 237
**Batch 245 (2026-10-16, Plan 248 / Research 277: +6 the Batch-244 DEFER LIST conversion — the eight staged leads convert 6 + rider + defer, ZERO dropped, the same three sources re-read at their release pins: `self-loop-state-memchr-skip` (dispatch, the dfa/accel.rs self-loop law — one memchr over the ≤3 exit bytes replaces N state steps, the b' ' admission refusal is the source's own; GOAT 5.5-6.4x at exit-density 5%, alloc 0/0) + `pair-probe-confirm-fused` (simd, the generic packedpair fused AND — two offsets, ~p₁·p₂ candidates; GOAT 8.1-9.9x in the common-byte regime; fixture RESTRUCTURED once (rare-singles shape Dominated) and a standalone PARITY ORACLE caught the bit-8j+7 SWAR extraction bug the timing-only validator cannot see — the pre-fix 8.9x was an artifact of skipped verifies, disclosed in-rule) + `finder-hoist-reuse` (caching, the MERGED memmem+bstr Finder-construction amortization; GOAT 3.0-6.4x at ~200B lines; first ~30B-line shape Dominated — below the 64B threshold the per-call path never constructs the meta searcher) + `canonical-set-guarded-rebuild` (complexity, the invariant-guarded normalize: is_canonical early-return + the folded applied-flag + union's equality short-circuit; premise-corrected — push is sort-free at this pin; NotBenchable) + `nonmax-sentinel-option-niche` (layout, the in-domain-impossible usize::MAX as Option's niche — the isize::MAX slice bound IS the proof; payoff RELABELED memory-first; NotBenchable) + `match-branch-cold-confirm` (dispatch, #[cold] on the rare HEAVYWEIGHT confirm = benchmark-cited win, tiny helpers = measured wash, generic/target_feature code = unexpressible; suggest-only, NotBenchable) + the count-neutral CHAR rider on byte-find-position-to-memchr (bstr find_char encode-once-into-stack; the U+FFFD caveat); dispatch 13→15, simd 10→11, caching 15→16, complexity 33→34, layout 15→16; retrieval floors HELD 21/0/1i; gen2+gen3 freeze e2e ok; score-bench run #185; the stale-pin repair: with_spec read 101 through B244, without_spec's B244 re-pin did its -10 on the wrong axis — both re-pinned to measured truth 114/109). Batch 244 (2026-10-16, Plan 247 / Research 276: +10 the text/unicode family's FOLLOW-UP — the byte-scan/search mechanism batch — from three sources at release pins `rust-lang/regex` @ `regex-automata-0.4.18` + `BurntSushi/memchr` @ `2.8.3` + `BurntSushi/bstr` @ `1.13.1`: `byte-class-equivalence-table-shrink` (layout, the byte-class law — equivalence-class the key space before the table; GOAT 1.39-1.39x on a 1024-state fixture, the first 24-state fixture Mixed — the table must exceed L1) + `prefilter-choice-shape-ladder` (dispatch, the cheapest-first accelerator ladder; GOAT 44-46.6x) + `alternation-prefix-collapse-at-build` (complexity, the common-prefix collapse + shadowing + ATTEMPTS ladder; GOAT 334.8-639.7x) + `byte-rank-select-rarest-probe` (caching, the MERGED regex-syntax rank() + memchr packedpair law, same 250 threshold; GOAT 22.9-38.1x) + `prefilter-skip-density-demotion` (dispatch, the PrefilterState feedback-control law — the corpus's FIRST measure-and-demote rule; GOAT 3.76-5.40x, the first scalar-probe fixture Dominated) + `iter-count-bulk-specialization` (iterators, the Iterator::count override; GOAT 9.19-9.28x) + `mod64-byte-membership-skip` (caching, the Two-Way ApproximateByteSet; GOAT 5.27-5.84x) + `byteset-cardinality-dispatch` (dispatch, memchrN for 1–3 vs the stack table for ≥4; GOAT 1.34-1.58x, the first 10%-density fixture Dominated) + `utf8-fast-lane-cold-diagnostics` (dispatch, the fast/cold diagnostic lane split; GOAT 8.60-10.33x) + `bufread-scan-in-buffer-scratch-fragment` (io, the zero-copy fill_buf drain; GOAT 2.77-3.63x, alloc 10892→0), ALL benchable + annotated; layout 14→15, caching 13→15, dispatch 9→13, complexity 32→33, iterators 11→12, io 26→27; the memchr-rank + regex-rank laws MERGED; DEFERRED to Batch 245 with reasons (8 leads); DROPPED sentinel-run-word-broadcast-skip + cross-product-budget-degrade + the memchr-3.x suspects (absent at the pin); retrieval floors HELD no re-pin (125/131/133 vs 124/130); gen2+gen3 freeze e2e ok; score-bench run #184). Batch 243 (2026-10-16, Plan 245 / Research 275: +2 the text/unicode processing family seed — the corpus's founding member of that family — `ascii-run-swar-block-skip` (simd, the high-bit-gather block mask; GOAT 6.41-7.78x) + `range-table-page-memoization` (caching, the lazy per-page predicate cache; GOAT 9.82-10.01x), both benchable + annotated, from Hugging Face tokenizers @ `5b5a3fcc` (branch `perf/faster-unicode-normalizers`, Apache-2.0); the third lead (transform-borrow-until-first-change) dropped dominated 0/3/0 twice — the negative in Research 275; retrieval floors re-pinned with attribution + gen2/gen3 freeze-tables regenerated). Batch 241 (+2 the serving slot-state cache pair #30580+#30567, both caching 10→12; + the count-neutral draft-state rider on P65 #30574; the row previously read 201 — stale through B239's +2, repaired here). Batch 237
(2026-10-15, Plan 239 — +3 the P58–P61 conversion set, ALL NotBenchable
annotated, quotes re-verified at the pin clones:
`f16-to-f32-accumulator-lut-breaks-conversion-serialization` (simd,
ggml-org/llama.cpp @ `685f7089` #30342+#30343, MIT: the F16C FP16→FP32
intrinsic lowers (Clang 20.1.8) to a vpinsrw merging into the live accumulator
register of the quantized dot products, serialising the loop — 19 vs 4
cycles/iteration estimated, 45–65% slower Q8_0/Q4_0 MUL_MAT_ID measured; the
lookup table breaks the false dependency; PER-DIRECTION: table for
FP16→FP32, F16C for FP32→FP16; the commented-out define is the deliberate
re-enable record; the x86 F16C GOAT lane was staged at landing — it LANDED
2026-10-16 (Bench 118 / bench_009 / plan 249, the i7-13700K 4090 box:
PARETO-PASS — dot intr/lut 5.00–6.16× with the vpinsrw accumulator merge
reproduced at T0 on rustc 1.98.1, the source's 45–65% measured 400–516%
here; store soft/intr 24–35× confirming PER-DIRECTION; the fixture's
store_f16 subnormal shift repaired count-neutral after the bench G1
executed and caught the always-negative shift) + `work-buffer-sized-by-execution-width-not-plan-tasks`
(concurrency, llama.cpp @ `5cdadfc3` #30454, MIT: the sizing authority is the
op's true parallelization width, never the planner's task count — 'threads > 0
wrote past the work buffer when the graph had another multi-threaded op') +
`discretionary-cache-growth-keys-on-spare-not-available` (io, incoai/splash @
`ffc2d4be` #418, Apache-2.0: growth that no request in service needs keys on
the host's SPARE memory — free + file-backed + purgeable + half the
compression credit, at most a quarter of the anonymous pages — a 7% reserve
(2–7 GiB) + the 1 GiB warning margin, warning-pressure latch + ≤1
GiB/paced-pass reclaim, unavailable telemetry pauses growth; measured 'free
memory read 0.07 GB at 64 GB of engine footprint while host_available_bytes
still read 31.9 GiB'); annotated 144 → 147, without_spec 99 → 102; the
count-neutral kernel_opt half: the autotune-cache HARDWARE-PATH axis rider on
`autotune-fidelity-synthetic-carries-producer-params` (cubecl @ `5a4f5882`
#1811 — the tune-cache namespace gains the resolved adapter's fingerprint,
the wgpu runtime name carries its API); retrieval floors HELD, corpus 813
kernel / 201 rust_perf stand).
Batch 222 before it
(2026-10-10, Plan 216 / Research 252 — +2 the soroban wasm size pair, source
`stellar/rs-soroban-sdk` @ `0d72c4c9`, Apache-2.0: `wasm-unwrap-panic-machinery-elimination`
(on wasm every `.unwrap()` carries core::panicking's message + fmt machinery
into the artifact even under panic = abort; the `UnwrapOptimized` trait swaps
the precondition-impossible arm for a 1-byte `core::arch::wasm32::unreachable()`
trap — native arm keeps the plain unwrap — plus the `UnwrapInfallible`
`Err(never) => match never {}` statically-checked-infallible clause; the
MANDATORY anti-regime: never rewrite a reachable unwrap arm) +
`linker-dce-custom-section-volatile-marker` (a data-only `#[link_section]`
custom-section static with no compiled-code reference is shaken out by the
linker's GC — root it in a reachable path via `read_volatile`; the #2114 'Fix
event spec shaking' fix incl. the `keep_reachable` fn-pointer form — 'Keeps
`f`'s symbol alive through the linker's dead-code-elimination pass without
invoking `f` at runtime.'); both category `wasm` — the family 1 → 3 — both
NotBenchable annotated (the size-axis domain reason; annotated 142 → 144,
without_spec 97 → 99); quotes re-verified at the pin via the .raw clone).
Batch 231 rider (2026-10-14, Plan 233 / Research 262 — count-NEUTRAL: the
`madvise-random-for-key-gather-tables` io rule += the macOS WILLNEED-PREFETCH
NO-OP face — ggml-org/llama.cpp @ `b9f3599f` #30405 second sub-commit, MIT,
quotes verbatim at the fetched .patch: the prefetch path's platform guard
`#if defined(__linux__) || (defined(_WIN32) && ...)` silently took the #else
on macOS — 'The prefetch hint of the tensors marked as prefetchable is a
no-op on macOS, so their pages are faulted in one by one while the graph
runs. madvise (MADV_WILLNEED) behaves as expected here, so add the platform
to the guard'; the repair is ONE platform constant (`+defined(__APPLE__)`);
measured (commit message, cold file cache) 356/375/475 → 735/749/788 t/s at
pp512/2048/4096 — ~2× pp from making the EXISTING hint actually execute; the
platform-guard-silent-else class named (the arm exists and runs and reaches
nothing — the green-zero shape one layer down); the portable SPAN-prefetch
half of this rule and the MADVISE half compose; same-commit PIN-axis sibling:
kernel_opt `host-pointer-buffer-foreign-range-subtraction`; retrieval floors
HELD, corpus 804 kernel / 198 rust_perf stand).
Batch 232 rider (2026-10-14, Plan 234 / Research 263 — count-NEUTRAL: the
`degrade-not-fail-load-qualification-probe` dispatch rule += the
HEADROOM-ADMISSION face — ashhart/TensorFold @ `b264f534` "The 27B's Neural
Engine share loads only inside the 70% load limit", Apache-2.0, quotes
verbatim at the fetched .patch: the load-time two-class admission posture
with the predicate axis widened from measured exactness to RESOURCE HEADROOM
— `fitsLoadLimit` = `bytes +| MARGIN <= total / 100 * SHARE_PERCENT`
(saturating add; fail-closed on an unknown RAM reading — 0 admits nothing);
the defect economics: 'The share keeps an fp16 copy of 3,072 MLP columns of
every layer, 5,760 MiB for the 27B. On a 32 GB Mac that copy took the server
past its memory, and it swapped for about 5% faster prompts (#643)' — a 5%
prompt gain bought WITH SWAP is a net loss; the served host answers with the
70% load limit and its 2 GiB margin, so 'a 32 GB Mac runs prompts on the GPU
alone and says so, and a 48 GB or larger Mac keeps the share'; direct-loader
tools keep it wherever it can load — the limit is a SERVED-host contract;
same-commit PIN-axis sibling: kernel_opt
`warm-background-pass-quiet-hold-yield-chunking-cut-invariance`; retrieval
floors HELD, corpus 805 kernel / 198 rust_perf stand).
Batch 226 rider (2026-10-13, Plan 228 / Research 256 — count-NEUTRAL: the
B211 `repr-align-register-partition-store-forwarding` layout rule += the
DEVICE-CONTRACT view — the layout decision's THIRD audience, bevy @
`48ae9fc0a1` #26090: `UninitBufferVec` sized entries by Rust's `size_of`
(168 B under glam scalar-math) where the shader declares 176; the stride
takes from the consumer: `item_size: T::SHADER_SIZE.get() as usize`;
single-entry = LOUD (binding-size error at draw), multi-entry = SILENT —
capacity mistaken for contract; host faces fail at compile time/local
bench, this face fails at runtime on the GPU and its divergence trigger is
a feature flag; corpus 801/198 stand; retrieval floors HELD).
Previously 196 — Batch 221
(2026-10-10, Plan 221 — +1 the bevy `runtime-bool-duplicates-inlinable-body`
(dispatch, NotBenchable annotated; annotated 141 → 142), source bevyengine/bevy
PR #25864 @ merge `74884ffd`, patches `a4d1789` + `8328184`, Apache-2.0/MIT,
quotes verified at the pins this session: a runtime bool branching around a
large inlinable body forces the inliner into duplicate-or-deinline; the landed
form hoists the branch into a conditional `Option<Guard>` tuple (the restore in
a Drop impl, panic-safe) so the body compiles once — the patch-1
branch-around-closure disclosed in-rule as the anti-shape itself).
Previously 195 rules — Batch 219
(2026-10-10, Plan 215 slot 2 / Research 251 — the TensorFold native-engine
serving/timing pack, +11 ALL NotBenchable annotated (annotated 130 → 141),
source `ashhart/TensorFold` @ `f1aef9c3`, Apache-2.0, quotes verified at the
pin via git-show blob reads): the A− cost-table hygiene pair
`fastest-of-n-dip-drift-retime-cost-table` +
`pava-monotone-cost-pooling-modeled-cost-not-clock` (complexity — capture
fastest-of-7-after-one-warmup, dip-pair + whole-table drift retimes, persist
per BUILD identity beside an exe-free reference key; PAVA-pool per-row verify
costs before any depth decision reads them, the round EMA follows the MODELED
cost never a clock so every rank chooses alike) + the prompt-cache pair
`prompt-cache-canonical-interval-keys` +
`prompt-cache-rewind-states-min-gap-eviction` (the staged B+ entry SPLIT at
authoring — cache keys are the token prefix AND the canonical decoded
intervals; rewind states sit min_gap-spaced before the latest user turn, the
kept-under-min_gap state IS the rewind, budget-fitted history-first, refusals
before the pass, last-evicted while its turn lasts) +
`prompt-state-fingerprint-probe-bits-keyed-persistence` (caching — a
whole-build fingerprint incl. a startup probe's output BITS: behavioral
identity, not metadata) + the T18 transport pair `bounded-peer-handoff-wait`
(#512 — a peer handoff wait bounded at 10 s with named TpPeerSilent/
TpPeerFailed errors, peer failed/quitting/stop re-checked every iteration,
never claiming an absent arrival) + `spin-lock-one-message-per-exchange`
(one message per exchange through a spinning post lock, 14-19 µs → 2-7 µs) +
`thermal-hysteresis-band-pacing-chunk-placement` (the corpus's THIRTEENTH
category thermal — wait while the hottest zone > high, release only at ≤ low,
read before each prompt chunk, cancel ends the wait, `heat_wait_s` logged,
unreadable zone memoized once; #339, three DGX Sparks) +
`degrade-not-fail-load-qualification-probe` (dispatch — at LOAD a real-weight
probe compares two 16-row streams vs one-row execution; numerical mismatch
disables wider lanes loudly, one-row rounds still serve, GPU execution failure
stops the load) + `metal-keepalive-residency-unwire` (io — a one-thread
400 ms command-buffer heartbeat holds macOS's residency against the 1-2 s
unwire / 2.7 s K3 re-wire; `--keep-warm` seconds window) +
`display-carveout-all-or-nothing-admission` (allocation — GB10's 1,792 MiB
display reservation as KV memory via a DRM dumb buffer registered with CUDA,
all-or-nothing, a short grant refuses with the failing step named, recurrent
state stays out). T18 dispositions: MTLFence/simdgroup-tile/nvfp4/kv-quant
DEFERRED to kernel slots, exl3-oracle-fixture dropped to lane-intel. T0 drift
disclosed: tip moved `f1aef9c3` → `2f8284e` (72 commits) with
prompt_cache.zig + learned_prompts.zig touched post-pin (the
learned-disk-admission series — future lead pool). Retrieval floors HELD (the
precision regression gate green, rust_perf demonstrability constant 0.964
unchanged, no re-pin); freeze/score-bench NOT owed (zero clippy-corpus delta,
P30). Method: 3 authoring subagents over disjoint slices (A: N1+N2, B: N5+N9,
C: N7+N8+N10+N11) + the coordinator authoring the T18 transport pair; every
quote verified at the pin twice (slice agents, then coordinator needles).
Previous: Batch 207 (2026-10-07,
Plan 207 / Research 242 — the 10-07 staged leads' rust_perf half, +2, both
NotBenchable annotated, annotated 127 → 129): `per-scope-map-scan-hash-crossover`
(complexity; source `tracel-ai/cubecl` #1767 @ `d307dd46`, MIT OR Apache-2.0 —
a scope-keyed lookup table scanned linearly on every lookup AND insert is
quadratic in the operations of a region ("A large unrolled kernel spent
minutes in it"); each scope becomes a hash map — but the MEASURED NEGATIVE is
the rule's second half: "many regions of a few additions ran 15% slower than
main", so the adopted shape is pliron's SmallMap hybrid — inline-scan the
first eight entries without allocating (`INLINE_ENTRIES = 8`), hash past
them; the key-type clause: `ExpressionKey` hashes everything but its
attributes, which have no Hash and stay in the equality — keys differing
only in attributes share a bucket by design) + `inline-hint-not-through-unmarked-wrapper`
(dispatch; source `bevyengine/bevy` #26040 @ `1ba0c9f9`, MIT OR Apache-2.0 —
#25200 added wrappers around `fetch_next_aliased_unchecked_internal`, which
is marked `#[inline(always)]`; the new wrappers weren't marked, so the
function stopped getting inlined — the four-row laptop table reads
+30–90 µs on every row and the fix restores (and on one row beats) the
pre-refactor numbers; the class: an inline hint is per-function, never
transitive — a refactor adding wrappers needs the wrapper-attribute audit +
the bench, because only the benchmark saw it; the LLM-assisted root-cause
disclosure with manual diff verification rides as provenance). Was 181 —
Batch 204 (2026-10-07, Plan 204 / Research 239 — the PrismML NEON PQ2_0
vec_dot rule, source `PrismML-Eng/llama.cpp` #265 @ `6bfcd79a`, MIT —
simd 9→10, the corpus's first NEON decode/codec member:
`neon-tbl-replicate-2bit-codec-sdot-block-rescale` — one `vqtbl1q_u8`
with an index table replicates each packed byte across the vector,
in-lane shift/mask/sub lands each 2-bit code in the lane that owns it as
{−1,0,+1}, the block's sub-block dots accumulate in int32 via sdot, and
the block scale applies ONCE per 128-weight block; regime = small-domain
integer codes whose consumer can stay integer (Q8_K's one-scale-per-256
is what makes the int32 accumulation + single rescale legal);
anti-regime = float-domain outputs where NEON's missing gather forces
the scalar-extraction path; NotBenchable on the x86_64 host box — the
M3-Max GOAT is staged with the B62 naive-codec-reference law; plus 3
kernel_opt riders: the B116-addendum ISA boundary (the same u16-lane
widening measured SLOWER on NEON — M5 Pro + Snapdragon 7 Gen 4, in-PR
drop — dp4a-widen / NEON-scalar / i8mm-SMMLA is now the three-arm
decision), the B127 CPU-NEON instance + the exact vec_dot test
methodology, and the B103-addendum pairing-migration orphan +
`ggml_cpu_pq2_needs_q8_0` row-extent escape hatch). Previously
180 rules — Batch 202 (2026-10-07,
Plan 201 — the nnethercote min-rank worklist rule, source rust-lang/rust
`#160193` "Improve CFG traversal" @ merge `cb9d1b06`, MIT/Apache — the
author's 2026-09-30 perf blog; `dataflow-fixpoint-convergence-traversal-order`
(complexity, NotBenchable annotated — a fixpoint's convergence is
traversal-order dependent: re-process re-dirtied nodes in dataflow order
(min-rank), never FIFO push-back; 1.5M → 90,000 apply_effects_in_block calls,
~30% check-build wall on cranelift-codegen; the 17% figure is #160033's,
NOT this PR's — the drift correction rides the entry). Previously
179 rules — Batch 200 (2026-10-03,
Plan 198 — the TensorFold staged pool's serving half, source
`ashhart/tensorfold` @ `c4646171` v0.6.0, Apache-2.0 — the kernel half
landed as Batch 199; all eleven NotBenchable annotated: prefix-cache-entry,
butterfly-piggyback, doorbell, per-kernel-measured item width, parity
construction, mode switch, snapshot-release, pool handle, AIMD ramp,
pinned-staging release, madvise-random; most serving-stack decision
content — the B182/B186 precedent; ~10 staging drifts disclosed). Previously
168 rules — Batch 198 (2026-10-02,
Plan 196 — the PrismML fork #284 pack, source `PrismML-Eng/llama.cpp` @
`3285e757a6`, MIT): `selector-lattice-beam-over-greedy-walk`
(complexity, NotBenchable annotated — the NOT-ISOLATED caveat rides
context_notes, the commit's end-to-end number is the fewrow kernel's):
a per-step greedy walk over a small scored selector lattice throws away
paths that lose locally but win over the full block; a WIDTH-2..4 beam
(env `LLAMA_DFLASH2_BEAM_WIDTH`, band-validated else fail-open to
greedy) expands each beam path over the top_k slots per step, scores by
per-step log-softmax, keeps the best paths via partial_sort, NaN→−inf +
lexicographic slots tie-break (both verified at the pin), deterministic;
the winning beam replays the SAME p_min/n_min acceptance contract greedy
used — only the walk changed, never the acceptance); Batch 196 (2026-10-02,
Plan 195 — the TensorFold v0.6.0 A− serving/host distill, source
`ashhart/tensorfold` @ `c4646171`, Apache-2.0 — license flipped
MIT→Apache-2.0 at 0.6.0): two NotBenchable annotated rules —
`grammar-window-draft-prune-path-local-mask` (complexity — a
grammar-constrained reply walks the drafted verify tree depth-first
BEFORE the verify forward (accept going down, rollback coming back up),
drops every draft no accepted path can hold (a grammar-rejected token, a
stop token, and the rows under either — "the parent's masked row could
never choose it"), fills each kept row's allowed-token bitmask from its
path, −infs the rest before sampling; row-locality is the exactness law
("A row's logits do not depend on the other rows, and its mask depends
only on its path"), so the prune is exact and the forward is paid only
for rows an accepted path could use; bounded refusal surface (400: no
engine / xgrammar missing / schema won't compile) + the TP face ("both
ranks then walk and mask the same rows") — the corpus's FIRST
grammar/constrained-decoding content, NOVEL) +
`startup-estimate-derived-from-allocation-shapes` (allocation — a startup
capacity plan derives EVERY term from the allocation shapes the engine
allocates (a helper mirroring the allocator's own formula,
`exl3_expert_scratch`, consumed where the plan sums terms) and proves it
with a FAKE-ALLOCATOR RECONCILIATION at real shapes (allocated ≤
estimate, == per shape; the harm: the EXL3 term was a fixed 1.125 GiB/rank
guess while the engine allocated a Scratch per buffer set + 1 GiB prompt
split-K partials nothing counted → ~2.5 GiB under, admission lied;
measured correction +2.52/+2.60 GiB, commit-message); sub-arms folded:
NVFP4-widening estimator blindness 4.00×→1.01× ("the admission estimate,
which counts file bytes, could not see why") + the `--checkpoint-slots`
knob joins the estimate and is refused pre-download); both NotBenchable,
both corpus-only (the B182/B186 precedent — no proposer files)) — Batch
186 (2026-09-28, +2 measurement-honesty members from the ggml-cpu tiled mul_mat distill, Plan 181 / Research 218: `fallthrough-toggle-oracle` (the ORACLE member — same graph through one admission switch, magnitude-scaled tolerance) + `cold-l3-evict-min-of-n` (the COLD member — 256MB LLC flush between reps, min-of-5; inverse polarity of B181's seed-warm); both NotBenchable, both complexity-annotated) — Batch 182 (2026-09-28,
Plan 177 / Research 216 — the mlxfast-bonsai2 DELTA distill, delta
cfb35b0d..831fae7): three NotBenchable annotated rules —
`host-mirror-shared-scalar-advance-one-time-verdict` (dispatch — N layers
advancing the SAME +L scalar = 16 one-element launches/verify-window → host
mirror at the same code sites + ONE shared array + a ONE-TIME
device-vs-host equivalence verdict: match ⇒ host path for the process,
mismatch ⇒ device op forever) + `prompt-continuation-draft-override-measured-min-match`
(complexity — override ONLY draft ids on a UNIQUE ≥16-token match (floor 8;
the measured NEGATIVE at n=2..4: 12→15 rounds — the floor is evidence),
SKIP the drafter block on the round after a from-prompt proposal,
on-device-scored splice in the early rounds; the target still verifies
everything) + `whole-round-in-situ-adoption-trial` (complexity — the
measurement-honesty family's ADOPTION member: kernel-variant selection on
WHOLE workload rounds interleaved — first round discarded, >1.5× median
dropped, >0.5% margin, ≥4 kept rounds; 'a 0.5-0.7 ms burst never reaches
the clock state of the decode window, so it mispicks'; pinned constants +
the L18 orthogonal-array per-shape sub-arm). Previously 160 rules — Batch
181 (2026-09-27,
Plan 174 / Research 212 — the mlxfast-bonsai2 arena distill): two
NotBenchable annotated rules — `seed-warm-the-scored-seam` (complexity,
the measurement-honesty family's SEAM-WARM member beside B177's
COMPARATOR side; run the EXACT scored seam once at load on throwaway
state — fixed token pattern, greedy, state released after; in-source
pricing: the un-warmed engine round cost 143–208 ms vs 36–41 and the
seed window 30–42 ms slower per leg; carries the per-depth-oracle
measurement-law rider — spec-lane pins oracle PER DEPTH, never against
the serial tape) + `mask-memo-hoist-to-owner-lifetime` (caching — a
pure-function-of-geometry artifact rebuilt per layer/per call → memoize
at the OWNER's lifetime keyed on EVERY input — repeated rounds reuse the
same value AND the same graph node; 'Sliding masks depend only on block
geometry'); then Batch 179 (2026-09-25,
Plan 172 / Research 210 — the rsconstmap distill): three measured
`ParetoDominant` benchable rules completing the std-map-probe family's
regime lattice — `paired-probe-layout-present-vs-absent` (layout: zip the
fingerprint + value arrays into 16-byte `[u64; 2]` slots, 6 → 3 cache lines
per present-key probe, one 128-bit XOR on baseline SSE2/NEON; the absent-key
regression IS the regime gate — the split layout's fingerprint array is half
the map and early-exits before the values; ParetoDominant 3/0/0 at
1.68×/1.45×/1.80×) + `block-hash-before-gather-mlp` (iterators: resolve the
whole block of 8's indices first, then gather — memory-level parallelism;
ParetoDominant 2/0/1 at 1.68×/1.30×/1.02×, the win at cache-resident tables
on this box, the 256 MB cell ties — the regime divergence from the source's
cold-first telling recorded honestly, plus the multiply-high index-form
fixture lesson: a `% m` divider chain masked the whole regime in the first
draft) + `immutable-static-map-filter-selection` (complexity: the
construct-once binary-fuse static map via `[dep: constmap]` — the family's
runtime-built-immutable cell, keys dropped ~9 vs 41-52 B/key, lookup 4.3×
at 1M keys growing with key count; absent-keys-undefined + no-enumeration
contracts in the note). All three carry in-bench G1 (live in every
invocation's make_input) + flat alloc axes. Batch 177 (2026-09-25,
Plan 170 / Research 207 — the EXL3/GB10 serving-recipe distill): the
measurement-honesty family grows to four members with the pair
`aa-control-prices-the-fidelity-instrument` (#3, the COMPARATOR — the A/A
control priced the text-diff fidelity gate at ZERO power: the engine is
non-deterministic at temp 0, and spec-on vs spec-on scored the SAME 2-of-6
as spec-on vs spec-off; the fallback is acceptance counters, and the
divergence is prompt-class-conditional so a constrained-only suite reports
a clean instrument with no power; NotBenchable, the B109 precedent) +
`observer-sync-bias-differs-by-change-class` (#4, the OBSERVER-CONSUMER
interface — the in-process harness's per-token console write is a host sync
whose bias SCALES WITH CHANGE CLASS ~4 tok/s kernel / ~10 host-side, so it
does NOT cancel in ratios; quote the harness matching the deployed consumer
and say which one you quoted; the TTFT-absorbs-k+1 clause licenses ratios
never absolutes; NotBenchable); both complexity, suggestion-only, annotated;
Batch 174 (2026-09-24,
Plan 168 / Research 205 — the cubecl/bevy merged-fix distill): the lead
`deferred-command-flush-hoist-out-of-batch-loop` (concurrency — MEASURED
ParetoDominant 3/0/0 at 21.3×/13.3×/16.5× n=1k/10k/100k, verdict-stable ×2,
alloc axis EXACTLY FLAT by construction — pure-timing isolation of the
barrier's fixed-cost share; BOTH toy model bounds disclosed — a vectorizable
linear hook-sum reads 1.1-1.6×, the data-dependent dispatch chain that ships
reads the numbers above — with bevy's source-measured 2.4-10.6% END-TO-END
as the app-layer citation; the no-observer semantic contract carried as the
load-bearing adoption caveat) + `host-lock-across-reentrant-blocking-call`
(concurrency — NotBenchable, deadlock/hang class: decide under the lock, drop
the guard, issue the blocking side-effect after release; the probed-flag
skip-no-op sub-decision + the distinct-from-await-guard note; cubecl #1678)
+ the kernel_opt B56 rider (cubecl #1686 double-checked-lookup second site,
recorded in `.docs/11_domains/kernel_opt.md`); both suggest-only (+1
BenchSpec; `entries_08.rs` opened; the batch174 pareto e2e +
present-and-parse gate landed; all freeze tables verified UNMOVED at landing
— zero clippy_lints files touched; score row #162 — heal 96.36% FLAT,
created 1.82%; kernel retrieval re-measure 18/18 floors hold). Previously 151 —
Batch 173 (2026-09-23,
Plan 167 / Research 204 — the serde zero-copy family distill, the corpus's
FIRST dedicated serde family, the product-coverage axis: 67 tokio entries
beside zero serde; source the serde official docs — serde.rs lifetimes +
`#[serde(borrow)]` — and the serde_json `RawValue` docs): the lead
`serde-borrow-deserialize` (MEASURED ParetoDominant 4/0/0 — alloc axis EXACT
3R → 0, 300016 → 16 bytes at n=100k; the escaped-payload regime witness
printed in-bench: 3R→1R, two thirds of the win survives the anti-regime; G1
live-in-release asserts) + `rawvalue-passthrough-defer-parse` (MEASURED
ParetoDominant 3/0/0 — 950000 → 100000 at n=50k, 19/env → 2/env; G1 =
re-parsed-VALUE equality, the formatting-retention behavioral delta
documented); both suggest-only (+2 BenchSpecs; `entries_07.rs` opened per
the ~1800-line rule; the batch173 pareto e2e + present-and-parse gate
landed; all freeze tables verified UNMOVED at landing; score row #162 —
heal 96.4%, decline_correctness 100%, shaped 94.5%). Previously 149 —
Batch 172 (2026-09-23,
Plan 166 / Research 202 — the mezura line-counter distill, the corpus's
first hand-SIMD `std::arch` entry with both arms): the lead
`dual-mask-fused-block-scan` (simd — one fused 64-B block pass yields BOTH
the newline bitmask and the any-of-needle-set bitmask; AVX2 2×32
cmpeq+movemask / NEON 4×16 vceq + weighted vpaddq fold; MEASURED
ParetoDominant 4/0/0 verdict-stable ×2 on the 4090 AVX2, quiet —
43×/44×/23×/18× at 4 KiB/64 KiB/256 KiB/1 MiB, alloc axis 0/0, NEON arm
cross-compile-verified) + three NotBenchable suggestion-only rules
(`mmap-settled-size-gated-read` io — two-gate mmap admission [size ≥ 256 KiB
AND mtime-age ≥ 5 s settle] vs the concurrent-truncation SIGBUS + the
expected+1 sentinel read; `steal-batch-not-per-item` concurrency —
half-not-all batch steals; `nonblocking-open-for-head-probes` io — the
kernel-notification hang class) + 2 riders (anchor-cover selection on
`byte-find-position-to-memchr` — the first None→Some flip since
async-yield-point; the retention clamp on `reuse-allocation-clear`) + 1
drop.
Batch 154 (2026-09-18,
Plan 144 / Research 181 — the tokio upstream-surface refresh, the async-perf
pool's designated NEXT POOL: the CHANGELOG 1.47–1.52.3 surface + Alice Ryhl's
"Actors with Tokio") added three NotBenchable suggestion-only rules:
`write-vectored-capability-flag` (io — a custom AsyncWrite/Write impl with a
real vectored path but `is_write_vectored()` at default false leaves the
writev path DEAD CODE; tokio 1.50's write_buf/write_all_buf now gate on the
flag — PR #7871 by Darksonn/Ryhl, merge 9dc9b53, diff verified; one-line
bounded fix; the std twin since 1.36; the PR-review regime datum: vectored
pays when `chunk().len() < remaining()`),
`oneshot-response-send-unwrap-panics` (concurrency — unwrap on a response
oneshot send inside a handler panics when the requester was CANCELLED via
select!/timeout/abort; the panic kills the ACTOR, not the requester; `let _ =`
is the canonical idiom, already exhibited in rayon-bridge-oneshot's fixed
code), and `actor-bounded-channel-cycle-deadlock` (concurrency — cycles of
channels with BOUNDED capacity deadlock: each full inbox suspends its sender
awaiting the other; the fix breaks the TOPOLOGY (try_send back-edge /
asymmetric select), never the boundedness — in deliberate TENSION with
bounded-channel-backpressure, which is the content; matklad-credited
analysis) — plus four annotation folds (the spawn_blocking sharded-queue
REVERT 1.52.0 #7757 → 1.52.1 #8057 hang on batch-blocking-pool-segment — the
pool's own maintainers' sharding attempt shipped broken; the LIFO-slot-steal
REVERT 1.51.0 #7431 → 1.52.2 #8100 perf-impact on async-yield-point — the
measure-don't-assume datum; the coop::cooperative/poll_proceed public API
1.47.0 #7405 on async-yield-point — the 128-op budget is now public; the
socket-option family movement — quickack 1.48, set_linger DEPRECATED 1.49,
set_zero_linger 1.50 — on tcp-nodelay). No benchable candidate
(io-syscall/panic-liveness/dataflow metric classes — the Batch-136/152/153
precedent); gen2/gen3 freeze tables UNTOUCHED (rust_perf-only). Declined +
recorded in Research 181: in-place-init (unsafe/unstable-shaped), the
Cell::from_mut retain idiom (compile-error-shaped), spawn-placement +
reader/writer split (compile-error/architectural). Previously 142 —
Batch 153 (2026-09-18,
Plan 143 / Research 180 — the async-blocking distill, the async-perf pool's
execution) added three NotBenchable suggestion-only rules:
`blocking-sleep-in-async` (concurrency — the canonical blocking bug:
std::thread::sleep in an async body parks the WORKER thread so join!
siblings serialize; the async twin suspends the task; the family's missing
most-basic member), `rayon-bridge-oneshot` (concurrency — the corpus's FIRST
tokio×rayon boundary rule: inline par_iter/join/block_on in an async body →
rayon::spawn + tokio oneshot + .await; the small-work-stays-inline
anti-regime; distinct from the four Lattimore rayon-INTERNAL rules), and
`tcp-nodelay-request-response-accepted-socket` (io — accepted sockets in
small-write request-response servers without set_nodelay pay the
Nagle×delayed-ACK 40–200 ms stall class; bulk-streaming anti-regime; pool
source 5, tokio-perf-experiments @ f3a6d3b8, ideas-only/no-license) — plus
five annotation folds (the Research-180 C+ `spawn-blocking-cpu-pool-choice`
adjudicated ANNOTATION into batch-blocking-pool-segment's anti-regimes —
CPU-bound closures belong on rayon, the blocking pool is for blocking IO;
the 10–100 µs boundary + the 128-op runtime budget on async-yield-point
(previously unannotated); the budget-regime note on
adaptive-yield-consecutive-ready (K=4 user-side vs 128 runtime-side);
the mini-redis accept-loop canonical cite on bounded-fanout-semaphore
(acquire_owned BEFORE accept — the listener itself backpressures); the
io_uring provided-buffer-group kernel-side twin cite on
bounded-batch-buffer-pool — the third enforcement form, annotation only)
and the en-passant repair of the mod.rs '36/127' NotBenchable ledger line
(stale since Batch 145 → 50/142). No benchable candidate (scheduler/network
metric classes — the Batch-136 precedent); gen2/gen3 freeze tables
UNTOUCHED (the atom table derives from the CLIPPY seed corpus — rust_perf-
only). Previously 139 — Batch 152 (2026-09-18, Plan 142 / Research 179 —
the dial9-rs "Principles for fast Tokio applications" distill) added three
NotBenchable suggestion-only concurrency rules seeding the world-facing
tokio family (batch-blocking-pool-segment, adaptive-yield-consecutive-ready,
bounded-fanout-semaphore; the mini-redis p50/p99 figures NOT on the live
page at re-verification — only ≈10× + K=4 carried).
Previously 136 — Batch 147 (the agave distill, five rules); before that 127 — Batch 145 (2026-09-16,
Plan 130 / Research 171 — the MTPLX `ple_row_gather` host lane of the
Metal-shape pack) added the suggest-only caching rule
`mincore-probe-routes-gather-arm` (per-call residency-probe ROUTING — the
~0.5 ms sampled mincore probe picks warm→fancy-index vs cold→pooled pread
against a ~165 ms warm pass; threshold 0.99, worst-of-all-maps,
fail-safe-to-pread, small-gather inversion anti-regime, the macOS
FILE-WIDE mincore caveat encoded; Batch 143 (2026-09-10,
Plan 123 / Research 164 — the nnethercote "How to speed up the Rust compiler
in December 2025" post distill, the PERF-SEARCH intake row staged at
`cf9e44ae`; one MEASURED benchable rule + one NotBenchable-annotated rule,
both complexity):
`debug-level-gated-computation` (complexity, BENCHABLE — a hot-path binding
whose only consumer is a `debug!`/`trace!` call pays its computation every
iteration: the binding is a real statement and the level check is a runtime
atomic load LLVM cannot const-fold (without release_max_level_* features);
fix = `log_enabled!` around the computation, or move it into the macro's
lazy format args; source #147293 verbatim ("more than 3% in the best case"),
the mechanism home the perf-book Logging-and-Debugging chapter; the note
also carries #143684's wall-time-vs-icount inversion — icount −1.70% /
cycles −0.90% but wall +0.26%: "Wall-time is the true metric" — the corpus's
wall-clock GOAT discipline externally confirmed), and
`macro-generated-code-optimize-at-generation-point` (complexity,
NotBenchable annotated — proc-macro emitters trim/dedupe generated code AT
the generation point, where the field-set context makes redundancy visible;
measure with `-Zmacro-stats`; the Bevy `#[derive(Reflect)]` series:
bevy_ui 27,000 generated lines vs a 16,000-line crate, bevy_window −39%
generated → `cargo check` wall −16% → peak mem −5%; the load-bearing
sentence: optimize at generation rather than "relying on sophisticated
optimization machinery further down the compilation pipeline").
Also rode the batch: the dist domain's fifth rule (`large-api-dependency-
mostly-unused-hint`, a new `BuildTime` family — see dist.md). Previously
124 — Batch 141 (2026-09-09,
Plan 121 / Research 162 — the antirez/ds4 Part-B walk, the batch Research 162
staged behind the Issue-085 lane; three NotBenchable-annotated decision-content
rules from the agent/context layer at pin `6289c516`):
`compaction-reserve-before-prompt-build` (allocation — a destructive context
rewrite's reserve is MEASURED from the real prompt (build it, tokenize it,
`tokens.len + 64 + summary_budget`), and every free-threshold consumer reads
that ONE figure — a hand-picked constant is the drift failure; ds4_agent.c
:9354-9364 + four call sites), `compaction-tail-boundary-prefers-complete-turn`
(io — truncation boundaries are semantic: a bounded walk-back (2× budget,
floor sys_len) to the latest COMPLETE user turn — the source's own rule:
"A slightly longer complete turn is preferable to losing the user's
constraints and keeping only a fragment of our answer" — and atomic spans
(image blocks) snap a strictly-inside boundary to their START; :9286-9300 +
:9367-9377), and `streaming-prefix-holdback-bounded-buffer` (io — a stream
recognizer holds back at most the marker's own length in a type-sized buffer
(`sizeof("> **Hint:**") - 1`), matches incrementally at the prose-line-boundary
gate, flushes verbatim on first mismatch — ordinary text streams immediately,
no line-length pending window; :225-226 + :3294-3325). Previously
121 — Batch 139 (2026-09-09,
Plan 119 / Research 154 — the ds4-metal PR #5 distill, part (c) of the ds4
lanes; one NotBenchable-annotated io rule, decision-content): `prefault-admission-keyed-to-ram-headroom`
(a whole-file WILLNEED prefault of a huge mmap sidecar must be ADMITTED by
a RAM-headroom predicate — `hw.memsize >= model + sidecar + 16 GiB margin`
(the source's own worked cell: 64 < 41.7 + 29.8 + 16 stays demand-paged,
128 passes and prefaults again); fail CLOSED when RAM cannot be queried;
log the decision; keep a force override (`DS4_QWEN4_PLE_PREFETCH_FULL`);
the companion DONTNEED eviction sweep (`DS4_QWEN4_PLE_EVICT_TOKENS`,
717 -> 482 -> 129 MiB) is DEFAULT-OFF with the source's own unbenched-cost
caveat; and the mincore instrument refusal: macOS residency counters report
CACHE-WIDE file residency, not task-mapped pages — the sweep REFUSES to log
a residency figure and names vmmap as the reliable instrument). Previously
120 — Batch 136 (2026-09-09,
Plan 115 — the Lattimore "Graph Algorithms in Rayon" blog distill, three
NotBenchable-annotated concurrency rules, all decision-content — the source
is a qualitative approach comparison with NO measured numbers and the
author's own closing verdict IS the ladder): `dynamic-work-parallelism-ladder`
(work unknown in advance → pick the structure by COMPOSITION requirements
first, allocation second: scoped spawning is the composition-safe default
despite the per-task heap boxing; spawn_broadcast + private job control
STARVES the pool — parked threads are unavailable to rayon; channel +
par_bridge does not compose — an owned value created in one work item
cannot be split into borrowed work items), `sender-per-work-item-terminates-channel`
(the dynamic-queue shutdown idiom: every work item carries a sender clone,
drop the ORIGINAL after seeding, the channel closes when the LAST item
drops — termination by ownership, no sentinel/counter/close call;
the drop(tx) is load-bearing — forget it and the receiver hangs forever),
`nested-par-inside-channel-bridge-deadlocks` (the correctness lead: a
nested par_iter inside bridge-fed work can STEAL a task from another
worker, block pulling the next item from the SAME channel, and the
interrupted item's sender clone keeps the channel open — rayon tasks are
THREAD-TIED so the blocked stealer never releases it; fixes in order:
serialize the inner section / pre-materialize its inputs / move the whole
traversal to scoped spawning; the async/await axis recorded as the
author's unimplemented speculation). Previously 117 — Batch 135 (2026-09-09,
Research 155 / Plan 114 — the Wild-linker "RustForge wild performance
tricks" article distill, three MEASURED benchable rules, one allocation +
two concurrency): `atomic-vec-zero-cost-convert` (Vec<T> ↔ Vec<AtomicT>
representation switching is FREE when size+align match — into_iter().map on
the OWNED vec reuses the buffer and the loop compiles out; alloc axis 2→1
exact; switch representation per phase instead of paying atomics everywhere
or transmuting), `vec-reuse-across-lifetime-change` (a hot loop allocating
Vec<&'iter T> per iteration because the hoisted buffer cannot be TYPED →
own the buffer and retarget it in place via a const size/align assert +
clear() + empty collect — alloc axis 1→0; the 'static element type is a
caller contract, the panic story is buffer-loss never UB; MaybeUninit
shadow-struct extension recorded in-rule), `partitioned-mut-slices-zero-sync`
(shared Vec under one contended Mutex although each worker writes a known
DISJOINT contiguous range → chunks_mut partition, zero locks/atomics;
sizes = thread counts; alloc axis EXEMPT per the Batch-78 contended-mutex
probe class). Dedup gate: `reuse-allocation-clear` covers the caller-passed
scratch (this is the lifetime-TYPING obstacle it cannot express);
`cache-line-block-partitioned-lock` partitions LOCK GRANULARITY (this
eliminates synchronization via ownership partition — they compose).
DROPPED candidates (Research 155): sharded-vec parallel init (third-party
dep + composition), defer-drop-on-thread (allocator-dependent, no
deterministic verdict — lead recorded), MaybeUninit shadow struct (folded
into the lifetime rule as the extension arm). Batch 129 (2026-09-08,
Research 151 / Plan 110 — the ds4 Qwen Q2 distill, pins ds4-metal `b85a6174`
+ upstream `f62ca29a`) had made it 114 — added two io rules, NotBenchable-annotated:
`instrumented-attribution-not-production-times` (instrumented profiling
slowed the pipeline ~38% in-source — instrumented absolute times are
ranking-only evidence; the ranked-#1 family measured NEGATIVE while the
ranking's LAST item produced the retained win) and
`unset-control-arm-silent-default-parity` (a control arm built by
UNSETTING the candidate env on a DEFAULT-ON candidate runs the same
enabled code — the harness must accept an explicit control value and
validation must pass control 0 / candidate 1; the fix shipped in-source
as `--control-value`). Plus the R4 rider on
`full-vector-bit-identity-over-argmax` (two weak gates passed — local
FP64 checks + greedy-text identity — while full-logit comparison caught
2.63 max-abs drift; MoE routing amplifies local rounding). Batch 125
(2026-09-07, Research 148 / Plan 106 — the ds4 PR rebase-surge, commit
`ffd85d4`) had added one io rule, NotBenchable-annotated: `control-reproduces-on-baseline-
trims-overclaim` (a failing parity/regression control is reproduced on the
untouched baseline FIRST — a reproduction indicts the ORACLE, and the fix
trims it to consumed outputs under matching compute shape, never widens
tolerance). Batch 124 (Research 147 / Plan 104 — arXiv:2609.04040) had
added one io rule: `matched-control-disentangles-policy-from-model`.
Batch 123 (Research 146 / Plan 103 — the ds4 upstream surge, commit
`c0a6119f`) had added two io rules, all NotBenchable-annotated:
`full-vector-bit-identity-over-argmax` (a numeric-regression gate compares
the FULL float vector bit-exactly with pre-validated artifacts and three
statuses — identical/drift/invalid — never argmax-only), `renderer-error-
is-not-capacity-overflow` (a tri-state fit validator keeps render errors
off the destructive compaction path; only a true capacity overflow may
compact). Batch 122 (Research 145 / Plan 102 — the ds4 PR vision delta,
commit `69c38d2`) had added three io rules, all NotBenchable-annotated:
`model-aware-format-over-hand-assembled-tokens` (one model-aware formatter
owns a structured transcript turn; hand-pushed token sequences drift from
the chat template silently), `implemented-surface-requires-provisioning-
target` (a feature needing an external artifact ships its fetch target in
the same change), `diagnostics-not-just-exit-code` (a multi-turn session
regression scans captured stderr for per-turn failure patterns alongside
the exit code, saves full transcripts, does not grade content). Batch 121
(Research 144 / Plan 101 — the ds4 IQ2 delta, commit `c5ae2eb5`) had added
three io rules, all NotBenchable-annotated:
`resume-revalidates-provenance-before-reuse` (a resumable multi-part build
revalidates source + calibration + TOOL hashes against the manifest and
re-digests each cached part before reuse — presence is not identity),
`write-verify-readback-before-publish` (hidden-path write, independent-reader
hash verify per part, atomic-rename publish + sidecar manifest,
refuse-existing, template guards before byte one), and
`ab-isolate-one-component-by-baseline-byte-copy` (measurement honesty — A/B
on one component byte-copies the baseline for everything else, paired diffs,
confounded variants out of the headline, un-instrumented memory counters
labelled as partial budgets). Batch 118 (2026-09-06,
Research 142 / Plan 097 — the antirez/ds4 PR #991 PLE-gather distill) added
`parallel-disjoint-pread-hides-cold-page-latency` (io; REGIME-ANNOTATED
NotBenchable per the B94 precedent — N known DISJOINT byte ranges →
thread-parallel positional reads on the SHARED fd + indexed writes,
byte-identical by construction; QD1 → QD32 cold page-in overlap ~20×
source-reported, warm neutral; floor 512 rows + cap 64 + degrade-to-inline;
NEAR-ADJACENT ranges are coalesce-nearby-reads' OPPOSITE regime). Batch 112
(2026-09-05,
Research 134 / Plan 083 — the nnethercote July-2026 byval-move
distill) added `hot-byval-move-over-memcpy-threshold` (layout; Mixed
1/0/1 at 1K/1M, verdict-stable ×2, allocs 0/0 — the memcpy-call
threshold is ARCH-SHAPED: x86-64's inline ceiling is 128 B
(rustc #160005: >128-byte by-value moves lower to memcpy CALLS; the
136 B→104 B shrink + move-avoidance won 40% wall on the two worst
rustc crates, DHAT copy-profiling was the instrument) while the
corpus gate's asm reading shows aarch64 INLINING a 136-byte move as
ldp/stp pairs + an 8 B tail (a 320-byte move emits `bl _memcpy` —
the ARM ceiling sits in (136, 320]). The M3 1M tie is the honest
mechanism cell; the x86 re-gate ran SAME-DAY on the 4090 (quiet,
rustc 1.93) and showed the step directly — the 48-byte copy
inlines while the 136-byte loop's hot loop carries `movl $136;
callq memcpy` per iteration (wall: 136 B = 1.32-1.35x the 48 B
loop per iter on x86 vs 1.00-1.13x on the M3). Suggestion-only. Riders:
`usize-index-to-u32` += the #158720 cache-line-boundary confirmation;
`sort-key-cache-precompute-out-of-lock` += the #157179 short-sort-key
confirmation.) Batch 113 (2026-09-05, Research 137 / Plan 086 — the llama.cpp
v0.4.0 delta re-mine) added NO new rust_perf rules — two name-field riders:
`eager-pointer-collect-lazy-fetch` += the `--lazy-mode` app-layer lazy-fetch
posture (llama.cpp loads model tensors on demand behind a flag — the same
fetch-only-what-is-needed contract one layer up);
`channel-send-batching-unpark` += the quantizer row-slab stream (#27830 —
quantized rows flow to the GPU as batched slabs, not per-row sends).
Batch 109 (2026-09-05,
Plan 080 — the post-mining P10 measurement-honesty distill from the
katgpt-rs Issue 723 Class A2 evidence) added
`dead-result-timing-loop-folded-under-lto` (complexity; NotBenchable — the
FIRST measurement-honesty axis rule: a timed loop discarding its work
result measures ZERO under fat LTO even through callee-side `black_box`;
the fix is chained state + a consumer sink + the loud nonzero-baseline
assert; the gate refuted BOTH draft fix-patterns — XOR sinks cancel over
GF(2)-linear work in power-of-two batches, and wrapping_add sinks collapse
to the seed-independent constant 2^64 − n/2 over balanced batches — both
pinned as anti-shape assertions) — Batch 99 (2026-09-02,
Research 119 — the wasmi v2.0 blog distill, Plan 067) added
`mir-destprop-branch-site-merge` (dispatch; **ParetoDominant 2/0/1 —
1.04/1.24/1.25× at 1K/100K/1M dispatches, alloc axis EXACT 0/0, alloc-det
stable ×3** — a hot conditional dispatch whose two arms load a KNOWN fn
item and converge into ONE call site lowers [asm-verified] to a cmov of
the two code addresses + a SINGLE indirect tail-jump (mixed
branch-target history, no devirtualization possible); the split form
emits TWO DIRECT tail-jumps. Negative control measured: with
RUNTIME-OPAQUE fn pointers (genuinely dynamic dispatch) both forms keep
indirect calls and the split buys NOTHING (0.95-1.00×) — the rule
applies only to compile-time-known code pointers; toolchain regime:
rustc ≥1.92 MIR DestinationPropagation, re-gate on toolchain bumps);
the C+ secondary `hot-dispatch-gpr-arg-budget` was probed and honestly
NOT landed (Tie on windows-msvc x64 — 1.009/0.999×; sysv64-specific
evidence, queued-secondary kill-free) — Batch 94 (2026-09-01,
Research 110/112 — the oMLX PR #3260 SSD expert-streaming distill, Plan 058)
added `coalesce-nearby-reads-bounded-gap-cacheability-gated` (io;
source-reported regime-annotation, honestly NotBenchable — merge addressed
rows within a bounded byte gap (64 KiB) into one pread; the DECISION CONTENT
is the cacheability coupling: gap budget collapses to 0 on direct/F_NOCACHE
descriptors because over-read bytes are only free when the page cache absorbs
them; re-selection preserves the caller's order incl. duplicates; riders:
Darwin fcntl hints once per persistent descriptor, advisory-swallowed) —
Batch 91 (2026-08-31,
Research 106 — the Rustdoc filter-first distill, Plan 052, from noahlev.org
"Making Rustdoc 33% faster" + rust-lang PRs 159623/159779/159854) added
`filter-before-expensive-transform` (iterators; **ParetoDominant
11.85/11.30/6.61× at 1K/30K/300K, verdict-stable ×2, alloc axis EXACT
300,014 → 18,764** — map(expensive).filter(relevant) → re-express the
predicate on the RAW input and filter first; win ∝ discard-fraction ×
transform cost; IMPOSSIBLE when the predicate is not re-expressible on the
input; composes with filter-map-fuse) and
`decision-projection-before-construction` (complexity; **ParetoDominant
24.58/24.38/24.37×, alloc axis EXACT 300,000 → 0** — transform-then-project
to decide → compute the invariant projection on the raw input; the
invariance precondition is pinned as a test); Batch 86 (2026-08-29,
Research 100 — the Cloudflare Big Pineapple DNS-cache memory distill, Plan
047, found via online perf search) added `serialize-once-byte-buffer-store`
(allocation; **ParetoDominant 4.26/3.52/2.90× at 1K/10K/100K, verdict-stable
×2, alloc axis EXACT 200,001 → 2** — per-record heap fields → ONE
length-prefixed byte buffer + u32 offset table; the insert-path malloc
storm is the mechanism; bytes 7.90 MB → 4.70 MB; regime: sequential-rebuild
consumers win, in-place random-mutators lose; the full-wire-message form is
the source's own REJECTED alternative — recorded as the anti-shape),
`context-equal-field-elide` (allocation; **ParetoDominant 3.12/3.01/2.00×
at the ~80% context-equal mix, verdict-stable ×2, allocs 100,001 → 20,056**
— a stored field that equals the enclosing context (the lookup key)
becomes `Option<Box<T>>` None, resolved at read; the two-regime note: low
%-equal is pure overhead; composes with serialize-once), and
`bitflag-pack-when-footprint-dominates` (layout; **NotBenchable-speed — the
size axis 12 → 8 pinned by gate** — stored-at-scale bool cluster → one
bitflag byte; cross-cites the kernel_opt hot-read INVERSE regime;
measure-size_of-before/after caveat; rider: `box-str-immutable`
generalizes to Box<[T]> for any frozen T, and `box-large-enum-variant`
gains the honest costs — size-class rounding, locality scatter, the rare
variant pays more)); Batch 48 added the 8 online-search candidates
from the perf-book std-types + iterators chapters; Batch 53 added the 4
online-search candidates of [Bench 038](../../.benchmarks/038_batch53_rust_perf_online_search_goat.md);
Batch 59 (2026-08-18) added the first EXTERNAL-source distill via the
`distill` skill: `division-by-invariant-multiplication`, from libdivide via
the raygon-renderer/thermite Rust port; Batch 61 (2026-08-18) added the 2
oxc-article rules: `keyword-map-to-match` + `usize-index-to-u32`; Batch 62
(2026-08-19, Research 070 microGPT-C distill) added the 2 CPU-SIMD rules:
`transposed-weight-register-gemv` + `newton-refinement-vs-hardware-transcendental`; Batch 64 (2026-08-19, Research 072 — the ARM Neon / bounds-check pool) added `loop-order-innermost-contiguous` + `debug-assert-length-hoist`; Batch 66
(2026-08-19, Research 073 — the polars kernel-idiom distill) added
`sparse-set-bit-iteration-tz-blsr` + `sortedness-check-branchless-accumulate`; Batch 67 (2026-08-19, Research 074 — the regex-automata distill) added
`byte-find-position-to-memchr`; Batch 68 (2026-08-19, Research 075 — the
tantivy utility-layer distill) added the annotated `varint-compose-word-single-store`; Batch 69 added `unaligned-word-load-from-bytes` — the T6 defer refuted, a cross-arch WINNER; Batch 65 (2026-08-22, Research 081 — the hotpath.rs profiler distill) added the 2 contention rules: `align-64-false-sharing` + `channel-send-batching-unpark`, both measured ParetoDominant; Batch 73 (2026-08-26, Research 085/086 — the mnt.io Matrix-RSDK room-list DoD distill, Plan 029) added `sort-key-cache-precompute-out-of-lock` + `lock-guard-narrow-field-accessor`, both measured ParetoDominant with regime annotations; Batch 74 (2026-08-26, Research 088 — the simd-lite/simd-json SWAR distill, Plan 032, the Research 085 pool-drop re-probe) added `swar-digit-chunk-parse` (ParetoDominant, regime-annotated) + `runtime-feature-detect-per-chunk` (measured Mixed-ties, honest-annotated); Batch 75 (2026-08-26, Research 089 — the GreptimeDB prom-read conversion distill, Plan 033, found via online perf search) added the 3 grouping/borrowing rules: `encoded-layout-borrowed-view` (ParetoDominant 29.95-41.98x, allocs 4M→0 — the dictionary-expansion trap), `group-keys-borrow-allocate-on-first-sight` (ParetoDominant 1.92-2.37x, allocs 7R+2S→8S+30 exact — the entry()-forces-owned-key escape), + `sorted-input-previous-row-fast-path` (Mixed: adjacent 12.92x / interleaved 0.99x tie — the hit-rate regime), all suggestion-only, + the `btreemap-sorted-iteration` regime-table rider); Batch 76 (2026-08-27, Research 090 — the Meilisearch LMDB/vector-store distill, Plan 034, found via online perf search) added `debug-stat-full-scan-out-of-hot-path` (ParetoDominant 168.91-5851.00x — the eager diagnostic statistic full-scan) + `eager-pointer-collect-lazy-fetch` (ParetoDominant 1903.70-21353.90x, allocs 1→0 — the full-scan pointer-map vs lazy on-demand fetch), both suggestion-only; pool drop: the Qdrant Gridstore article (architecture piece, dedup-adjacent candidate); Batch 77 (2026-08-27, Research 091 — the lucebox PR #625 rope.cu distill, Plan 035's crossover) added `runtime-int-exponent-powi` (arithmetic; **ParetoDominant 2.57x/2.81x/2.66x @ 1K/100K/1M on M3 aarch64 — CROSS-ARCH RE-GATED same day on x86_64-pc-windows-msvc: all-Tie 0.99/1.00/1.00 (verdict-stable ×2, same harness) — the win is the slow-libm-host regime (aarch64/GPU/embedded), an idiom swap on fast-libm x86 (the newton-refinement inversion class); verdict-stable x2, 0 allocs, rel-error gate <1e-12** — the runtime-integer-exponent case `powi-vs-powf` deferred: a runtime exponent gives LLVM no instcombine bailout so `powf` STAYS a libm call; GPU evidence: RDNA4 RoPE 692 → 76 µs/launch). Batch 78 (2026-08-27, Research 092 — the Datadog RTDB/Monocle distill, Plan 036, found via online perf search) added `narrow-counter-overflow-promotion` (layout; **ParetoDominated 0.64-0.66x honest-annotated** — the memory-for-speed trade: the alloc axis halves exactly (8N→4N bytes) but update-heavy streams pay 35-55% for the checked_add branch; the trade pays in memory-footprint-bound systems, Datadog's 2,100 GiB regime) + `cache-line-block-partitioned-lock` (concurrency; **ParetoDominant 1.12/3.91/2.98/3.03x at 2-12t** — one Mutex<Vec> → 512 aligned Mutex<[u64;8]> blocks; the first ALLOC_AXIS_EXEMPT rule: contended macOS pthread_mutex allocates nondeterministically in the slow path, probe-evidenced in bench.rs). Batch 83 (2026-08-28, Research 097 — the arXiv:2608.26368 SIMD bit-slicing predicate distill, Plan 044, found via the queue-11 arXiv listing walk) added `bitplane-transpose-repeated-predicate-scan` (simd; **ParetoDominant 1.96/2.16/1.93x at 10K/100K/1M, verdict-stable x2: 2.08/2.20/1.96x** — encode-once bit-plane transposition + zero-decompression bitwise predicate evaluation for REPEATEDLY filtered integer columns: the two-mask MSB→LSB recurrence is pure AND/OR word ops, b/32 the bytes, 64 values per word op; **honest gap vs the source's 10.7x NEON headline: in Rust the scalar arm auto-vectorizes (~0.126 ns/elem), compressing the win to the layout half** — never cite the source number isolated; regime gates: amortization (source break-even 7-25 scans; single-scan out-of-regime), b≤16, wide-type equality loses (source 0.5-0.7x), IN-lists k·b passes, point-lookups out; the DOMAIN CONTRACT (constant must fit the column width — out-of-range returns 0 via one branch per query) was CAUGHT by the G1 gate on an out-of-range threshold; plus the `branchless-conditional` rider: the source's scalar loop swung ~4.5x with selectivity on branch prediction while the plane form stayed constant). Batch 85 (2026-08-28, Research 098 — the arXiv:2608.23841 cflow "Pipeline-Native Transformers" distill, Plan 046) added `weight-tile-consumption-order` (layout; **ParetoDominant 1.32/1.35/1.35/1.56× at n_in=4K/16K/32K/64K, verdict-stable ×2, allocs equal** — row-major single-token matvec re-walks x per output row; repacking W once into consumption-ordered 64×512 tiles keeps the x slice L1-resident per tile, TILE_ROWS× less x traffic; the win GROWS with working set, M3's 128 KB L1-d shifting the source's Sandy-Bridge crossover 4× right; even the L1-resident arm measured 1.32×) + `software-prefetch-storage-bound` (io; **the honest annotation — SW prefetch on a bandwidth/storage-bound stream is useless-to-harmful: PF ±0.1% at 64 MB, PF=1 4% WORSE at 4.7 GB, the paper's prefetch thread burned 48% of decode time, disabling alone = 1.92×; x86 source-cited only (no stable aarch64 prefetch intrinsic); proposer-less by design**).
Allocation is the deepest
category (23 rules; Batch 75 added `encoded-layout-borrowed-view` + `group-keys-borrow-allocate-on-first-sight`; Batch 86 added `serialize-once-byte-buffer-store` + `context-equal-field-elide`; Batch 135 added `vec-reuse-across-lifetime-change`). Complexity has 19 (Batch 64 added `debug-assert-length-hoist`; Batch 75 added `sorted-input-previous-row-fast-path`; Batch 76 added `debug-stat-full-scan-out-of-hot-path`; Batch 91 added `decision-projection-before-construction`; Batch 109 added `dead-result-timing-loop-folded-under-lto`). Iterators has 10 (Batch 66 added `sortedness-check-branchless-accumulate`; Batch 67 added `byte-find-position-to-memchr`; Batch 91 added `filter-before-expensive-transform`). Simd 8 (Batch 62 added `transposed-weight-register-gemv`; Batch 64 added `loop-order-innermost-contiguous`; Batch 83 added `bitplane-transpose-repeated-predicate-scan`), layout 10
(Batch 61 added `usize-index-to-u32`; Batch 78 added
`narrow-counter-overflow-promotion`; Batch 85 added
`weight-tile-consumption-order`; Batch 86 added
`bitflag-pack-when-footprint-dominates`);
concurrency has 13 (Issue 014 added the two metric-mismatch rules; Batch 65
added the two measured contention rules; Batch 73 added
`lock-guard-narrow-field-accessor`; Batch 78 added
`cache-line-block-partitioned-lock`; Batch 135 added
`atomic-vec-zero-cost-convert` + `partitioned-mut-slices-zero-sync`; Batch
136 added the rayon-queue trio `dynamic-work-parallelism-ladder` +
`sender-per-work-item-terminates-channel` +
`nested-par-inside-channel-bridge-deadlocks`, all NotBenchable-annotated);
caching has 4 (Issue 014 added `regex-hoist-lazylock`; Batch 73 added
`sort-key-cache-precompute-out-of-lock`; Batch 76 added
`eager-pointer-collect-lazy-fetch`); arithmetic has 8
(Batch 53 added `powi-vs-powf` + `distance-sq-no-sqrt`; Batch 59 added
`division-by-invariant-multiplication`; Batch 62 added the annotated
`newton-refinement-vs-hardware-transcendental`; Batch 66 added
`sparse-set-bit-iteration-tz-blsr`; Batch 74 added `swar-digit-chunk-parse`; Batch 77 added
`runtime-int-exponent-powi`); dispatch has 5 (Batch 61 added
`keyword-map-to-match`; Batch 74 added `runtime-feature-detect-per-chunk`; Batch 99 added
`mir-destprop-branch-site-merge`; Batch 101 added `dispatch-handler-pair-hoist`); io is 5 (Batch 68 added
`varint-compose-word-single-store`; Batch 69 added `unaligned-word-load-from-bytes`; Batch 85 added
`software-prefetch-storage-bound`; Batch 94 added `coalesce-nearby-reads-bounded-gap-cacheability-gated`); the rest have 1
each. 90 are benchable (carry a `BenchSpec`); 27 are honestly
`NotBenchable` — 8 need external runtimes or are structural (size_of,
memory, fairness), 2 are **metric-mismatch** (Issue 014 T2.3/T2.4: the cost
is contention latency / queue depth, not wall-clock), the Batch 85 prefetch
annotation is x86-source-cited only, the Batch 86 bitflag rule is a
deterministic size axis, the Batch 94 read-coalescing annotation is
workload-level (source-reported, the cacheability coupling is the decision
content), and the Batch 109 rule is the measurement-honesty axis itself (a
timing gate whose work folds away has no nonzero baseline to ratio). 70 of 127 carry
`context_notes` — the class ledger that follows is the early-corpus
accounting; Batches 83–112 kept adding to the annotated set, and the per-rule
verdicts live in the table below — 11 verdict-class (4 `ParetoDominated` + 7
regime-fragile/Mixed — Batch 48 added the `filter-count-to-any` folk-wisdom
refutation: the VECTORIZED count loop beats `any()`'s scalar early-exit at
mid-position hits; Batch 53 added the `powi-vs-powf` refutation — LLVM
lowers `powf(2.0)` to the same multiply as `powi(2)` in release, the folk
wisdom only holds for debug profiles / exponents ≥ 3 — and the
`soa-hot-field` latency-bound scoping — a serial f32 add-reduction hides the
layout win; SoA pays on throughput-bound field consumers) + 2
metric-mismatch-class + 5 regime-caveat (Batch 62/64/66 winner-regime notes +
Batch 65's two: the **arch-granularity caveat** on `align-64-false-sharing` —
Apple Silicon lines are 128 B so `align(64)` recovers only 1.16-2.44× of
the 3.3-10.8× win; and the pipeline-overlap margin on
`channel-send-batching-unpark` — a 1P/1C harness hides the receiver's park
latency off the critical path). Consumers should treat
annotated rules as context-dependent, not unconditional suggestions.

| Rule ID | Category | Bench verdict | Context? | Pattern |
|---|---|---|---|---|
| `o1-lookup` | complexity | ✅ 570x ParetoDominant | — | O(n) `Vec` scan → O(1) `HashMap` lookup |
| `bsearch-sorted-slice` | complexity | ✅ 16-2709x ParetoDominant | — | O(n) `.contains()` on sorted slice → O(log n) `binary_search()` |
| `dedupe-hashset-vs-vec` | complexity | ~ ParetoDominant (regime-fragile) | ⚠️ `context_notes` | O(n²) `Vec::contains` dedup loop → O(n) `HashSet` collect (wins at N ≥ 1000; HashSet overhead dominates at small N — see Bench 015/020) |
| `entry-vs-contains-insert` | complexity | ✅ 1.5x ParetoDominant | — | `contains_key()` + `insert()` → `entry().or_insert()` (merge HashMap lookups: 2 hashes → 1 hash) |
| `loop-fusion` | complexity | ✅ 1.5x ParetoDominant | — | Two iterator passes (`.sum()` + `.max()`) over same data → one fused `for` loop (cache locality; compiler can't fuse separate iterator consumers) |
| `prefix-sum-2d` | complexity | ✅ 34-30000x ParetoDominant | — | O(n) 2D region-sum scan per query → O(1) prefix sum (boundary-vs-volume: prefix encodes boundary cumulative sum, each query reads 4 corners instead of scanning interior) |
| `swap-remove` | complexity | ✅ 2-18485x ParetoDominant | — | `Vec::remove(i)` O(n) shift → `Vec::swap_remove(i)` O(1) swap (order doesn't matter; win grows with N) |
| `fxhashmap` | complexity | ✅ 1.6-4.2x ParetoDominant | — | std `HashMap` (SipHash, DoS-resistant) → `FxHashMap` (FxHash, 1 mul + xor) for non-adversarial keys (compiler can't swap hashers; used pervasively in rustc) |
| `sort-unstable` | complexity | ✅ 1.08-1.25x ParetoDominant | — | Stable `sort()` (allocates) → `sort_unstable()` in-place (Issue 012 / Bench 016) |
| `vec-remove-front-to-vecdeque` | complexity | ✅ 3.65-1270x ParetoDominant | — | `Vec::remove(0)` O(n²) drain → `VecDeque::pop_front` O(n) (Issue 013 / Bench 017) |
| `btreemap-sorted-iteration` | complexity | ✅ 3.73-6.91x ParetoDominant | — | `HashMap` + sort per query → `BTreeMap` natural order (Issue 013 / Bench 017) |
| `static-str` | allocation | ✅ 42x ParetoDominant | — | `String` return → `&'static str` for constant labels |
| `vec-with-capacity` | allocation | ✅ 1.1-4.3x ParetoDominant | — | `Vec::new()` + push loop → `Vec::with_capacity(n)` (realloc cost; Bench 020 alloc axis: 16→1 allocs, 1,048,560→400,000 B) |
| `format-concat-loop` | allocation | ✅ 42-95x ParetoDominant | — | `format!("{s}{x}")` concat in loop → `String::push_str` (O(n²) → O(n); Bench 020 alloc axis: 1999→1 allocs, 1498× bytes) |
| `ok-or-else` | allocation | ✅ 97x ParetoDominant | — | `ok_or(expensive())` eager error construction → `ok_or_else(|| expensive())` lazy closure (avoids unused error allocation; Bench 020 alloc axis: 3.7M→37K allocs, 100×) |
| `mem-take` | allocation | ✅ 1.9-2.6x ParetoDominant | — | `buf.clone()` + `buf.clear()` → `mem::take(&mut buf)` (moves Vec allocation O(1), no deep clone; Bench 020 alloc axis: exactly 2× both axes) |
| `avoid-clone` | allocation | ✅ 1.3-3.0x ParetoDominant | — | `fn(data: Vec<T>)` clone-on-call → `fn(data: &[T])` borrow (clone does heap alloc + memcpy; compiler can't elide) |
| `box-str-immutable` | allocation | ✅ 1.1-1.3x ParetoDominant | — | `Vec<String>` (3-word handle) → `Vec<Box<str>>` (2-word, drops capacity field) for immutable string collections (Bench 020: no steady-state alloc delta — the win is width/cache density) |
| `reuse-allocation-clear` | allocation | ~ ParetoDominant (regime-fragile) | ⚠️ `context_notes` | `fn() { Vec::new() }` alloc-per-call → `fn(buf: &mut Vec<T>)` clear-reuse (wins ≤10K; loses at 100K due to L2 cache pressure — see Bench 015/020) |
| `box-avoid` | allocation | ✅ 9.6-42x ParetoDominant | — | `Vec<Box<i64>>` (heap per elem) → `Vec<i64>` (stack values) — heap alloc per element dwarfs the data work |
| `smallvec-stack` | allocation | ✅ 27-75x ParetoDominant | — | `Vec<T>::new()` (always heap) → stack array for ≤8 typical (SmallVec pattern); heap alloc dominates at small N |
| `log-write-direct` | allocation | ✅ 1.9-2.0x ParetoDominant | — | `buf.push_str(&format!(...))` (temp String per iter) → `write!(buf, ...)` direct (zero temp alloc; Bench 020 alloc axis: 10,015→15 allocs) |
| `cow-conditional-alloc` | allocation | ✅ 1.32-1.49x ParetoDominant | — | `String` return (always alloc) → `Cow<str>` borrow when no transform (Issue 013 / Bench 017; Bench 020 alloc axis: 75,000→30,000) |
| `arc-str-payload` | allocation | ✅ 5.38-11.38x ParetoDominant | — | Clone-per-message immutable payload → `Arc<str>` (clone the handle, not the bytes; Issue 014 / Bench 020: allocs 1001→2, bytes 42×) |
| `buffered-io` | iterators | ✅ 99-132x ParetoDominant | — | `File::read(&mut [u8; 1])` syscall per byte → `BufReader` 8KB batched reads (syscall overhead dominates) |
| `layout-reorder` | layout | ❌ NotBenchable | — | Field reorder to eliminate padding (deterministic `size_of`, not a runtime microbench) |
| `repr-u8-enum` | layout | ❌ NotBenchable | — | Field-less enum → `#[repr(u8)]` (deterministic `size_of`, same class as layout-reorder) |
| `mutex-to-atomic` | concurrency | ✅ 2.5x ParetoDominant | — | `Mutex<u32>` counter inc → `AtomicU32::fetch_add` (Mutex lock/unlock is opaque; ~2.5× single-threaded; Bench 020 alloc axis: 1→0 allocs) |
| `papaya-lockfree` | concurrency | ❌ 0.55-0.79x ParetoDominated | ⚠️ `context_notes` | `Arc<RwLock<HashMap>>` → `papaya` lock-free map (benefit needs multi-threaded contention) |
| `tokio-guard-across-await` | concurrency | ❌ NotBenchable (**metric mismatch**) | ⚠️ `context_notes` | Guard held across `.await` → fetch-before-guard (cost is OTHER tasks' wait latency, not wall-clock — Issue 014 T2.3; clippy `await_holding_lock` covers std guards only) |
| `bounded-channel-backpressure` | concurrency | ❌ NotBenchable (**metric mismatch**) | ⚠️ `context_notes` | Unbounded channel + hot producer → bounded `channel(cap)` (win is bounded queue depth; semantics-changing backpressure — Issue 014 T2.4) |
| `align-64-false-sharing` | concurrency | ✅ 4.7-9.4x ParetoDominant (2-12t; 1t tie) | ⚠️ `context_notes` | Adjacent 24 B per-thread atomic-counter slots → `#[repr(align)]` to **ARCH** line granularity (Batch 65 / Research 081 — hotpath.rs distill; **align(64) insufficient on 128B-line Apple Silicon**: recovers only 1.16-2.44× of the win; use align(128) there or `crossbeam_utils::CachePadded`) |
| `channel-send-batching-unpark` | concurrency | ✅ 1.16-1.29x ParetoDominant + allocs 3232→1624 | ⚠️ `context_notes` | Per-item MPSC sends to a parked `select!` receiver → per-64 batch flush via `mem::replace` (Batch 65 / Research 081; the modest margin is pipeline-overlap-honest — the source's 6.25× needs the park pair ON the critical path; **never `mem::take`** — regrow costs ~7 allocs/batch) |
| `simd-chunk` | simd | ✅ 3.9x ParetoDominant | — | Scalar loop → 4-wide chunked loop for LLVM auto-vectorization |
| `bool-to-bitpack` | simd | ✅ 7-11x ParetoDominant | — | `Vec<bool>` set-bit count → packed `Vec<u64>` + `count_ones()` (data density + hardware POPCNT) |
| `branchless-conditional` | simd | ✅ 10x ParetoDominant | — | Data-dependent `if` in hot loop → `val * cond as u64` branchless multiply (LLVM vectorizes the multiply but cannot vectorize the branchy loop) |
| `chunks-exact` | simd | ✅ 13-16x ParetoDominant | — | `slice::chunks` (variable last chunk) → `slice::chunks_exact` (uniform chunks, enables vectorization; source: nnethercote/perf-book) |
| `copy-from-slice` | simd | ~ Mixed (regime-fragile) | ⚠️ `context_notes` | Manual index-loop copy → `slice::copy_from_slice` (LLVM auto-vectorizes the manual loop for u8 — small throughput delta; the idiom still guarantees SIMD + no bounds check — Issue 012 / Bench 016) |
| `f32-counter-to-int` | arithmetic | ✅ 7.5-11x ParetoDominant | — | f32 accumulator for integer tally → u64 accumulator (f32 add dep chain can't be broken without fast-math) |
| `collect-avoidance` | iterators | ✅ 2.3-3.0x ParetoDominant | — | Unnecessary `.collect()` intermediate → chained iterator (allocation + double pass → single pass, zero alloc; Bench 020 alloc axis: 8,000,000→0 B) |
| `retain-inplace` | iterators | ✅ 1.5-2.1x ParetoDominant | — | `iter().filter().collect()` new Vec → `Vec::retain()` in-place filter (no allocation, single pass; ~1.5-2× across all sizes) |
| `sort-by-key` | complexity | ~ Mixed (regime-fragile) | ⚠️ `context_notes` | `sort_by` repeated closure → `sort_by_key` single extraction (Issue 012 / Bench 016) |
| `dyn-to-enum` | dispatch | ✅ 2.5-2.9x ParetoDominant | — | `Box<dyn Trait>` dispatch → enum dispatch (devirtualize: vtable indirection → direct match, compiler can inline) |
| `dispatch-handler-pair-hoist` | dispatch | ✅ CROSS-ARCH ParetoDominant (aarch64 2.89/1.21/1.23×; x86 re-gate 2026-09-04: 2.69-2.73/1.09-1.10/1.07-1.08× — the expected Tie REFUTED: the runtime-table fixture loads the handler in-loop on BOTH arches; the structured-stream Tie cells not re-measured on x86, predictor-mechanical) | — | hot loop reads its handler from a runtime table every dispatch → hoist both entries once before the loop (register-resident select); wins on UNPREDICTABLE streams only (structured/periodic = Tie); Batch 101 |
| `precompute-table` | caching | ❌ 0.20-0.40x ParetoDominated | ⚠️ `context_notes` | Recompute-per-call → `const` lookup table (compiler optimizes popcount to POPCNT in release) |
| `regex-hoist-lazylock` | caching | ✅ 5443-5675x ParetoDominant | — | `Regex::new` per call → `static LazyLock<Regex>` (compile once; Issue 014 / Bench 020: allocs 1.3M→0, bytes 563.6M→0 per 1000 calls — the largest deterministic win in the corpus) |
| `bevy-reactive-loop` | bevy | ❌ NotBenchable | — | `WinitSettings::default()` (Continuous, 100% CPU) → `reactive_low_power` (needs Bevy runtime) |
| `wasm-batch-ffi` | wasm | ❌ NotBenchable | — | Per-call FFI (250ns floor × N×M) → single batch invocation (needs WASM runtime) |
| `collect-append-to-extend` | allocation | ✅ 9.8-13.1x ParetoDominant | — | `collect()` + `append()` staging → `extend()` (no intermediate Vec; Batch 48 alloc axis: 6251→1 allocs, bytes halved @100K) |
| `hashset-insert-bool-return` | complexity | ✅ 1.19-1.31x ParetoDominant | — | `!set.contains(&x)` + `set.insert(x)` double lookup → `insert`'s bool return (one hash-table walk; Batch 48) |
| `unwrap-or-else-lazy` | allocation | ✅ 1.9-2.3x ParetoDominant | — | `unwrap_or(expensive())` eager default → `unwrap_or_else(closure)` lazy (99%-Some regime; Batch 48 — sibling of `ok-or-else`) |
| `filter-count-to-any` | iterators | ❌ 0.82-0.90x ParetoDominated | ⚠️ `context_notes` | `filter(p).count() > 0` → `any(p)` — **refuted at mid-position hits** (Batch 48): LLVM vectorizes the count loop while `any`'s early-exit forces scalar iteration; wins only for ≲10%-position hits or non-vectorizable predicates |
| `vec-macro-zero-fill` | allocation | ✅ 32-2505x ParetoDominant | — | `Vec::new()` + `resize(n, 0)` explicit memset → `vec![0; n]` `alloc_zeroed`/calloc lazy zero pages (Batch 48) |
| `iter-copied-deref-map` | iterators | ⚠️ Mixed (0.80x @1M) | ⚠️ `context_notes` | `.map(|x| *x)` → `.copied()` (Batch 48): parity ≤100K, SLOWER at 1M on sum folds — the idiom's TrustedLen matters for `collect`, not folds |
| `filter-map-fuse` | iterators | ✅ ParetoDominant | — | `filter(p).map(f)` two adapters → `filter_map` fused (Batch 48: timing-only win, allocs parity — the perf-book's hedge resolved by measurement) |
| `vec-insert-front-deque` | complexity | ✅ 7.8-115.3x ParetoDominant | — | `Vec::insert(0, x)` O(n²) prepend loop → `VecDeque::push_front` O(1) (Batch 48 — the write-side sibling of `vec-remove-front-to-vecdeque`) |
| `distance-sq-no-sqrt` | arithmetic | ✅ 1.21-1.24x ParetoDominant | — | `(dx²+dy²).sqrt() < r` → `dx²+dy² < r²` compare-squared (Batch 53 — monotonic sqrt; keep the sqrt when the VALUE is consumed) |
| `hashmap-with-capacity` | allocation | ✅ 2.4-3.1x ParetoDominant | — | `HashMap::new()` + known-count insert loop → `HashMap::with_capacity(n)` (Batch 53 — rehash ladder re-SipHashes all entries per resize; allocs 12→1) |
| `division-by-invariant-multiplication` | arithmetic | ✅ 2.03-2.13x ParetoDominant (M3 aarch64) | — | loop of `x / d` with RUNTIME-invariant `d` → precomputed libdivide-style magic multiplier + shift (Batch 59, thermite distill — CONSTANT divisors are already strength-reduced by LLVM, the regime is exclusively the runtime-invariant case; setup pays one hardware div, amortized after ~3 applications; expected wider on x86 where DIV is 30-90+ cycles vs aarch64 UDIV) |
| `keyword-map-to-match` | dispatch | ✅ 5.23-5.58x ParetoDominant | — | static-keyword `LazyLock<HashMap<&str, V>>` dispatch → plain `match` on string literals (Batch 61, oxc distill — LLVM lowers the match to a length switch + inline compares; the map pays SipHash per lookup; STATIC key sets only) |
| `usize-index-to-u32` | layout | ✅ 1.81-1.99x ParetoDominant | — | usize index/offset fields in high-volume structs → u32 (Batch 61, oxc Span — halve the cache footprint; domain must bound below 2^32) |
| `transposed-weight-register-gemv` | simd | ✅ 1.18/2.09/6.08x ParetoDominant (NIN 16/64/4096) | — | row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA into output registers (Batch 62, microGPT-C distill — architectural: pack at weight-freeze, no proposer) |
| `newton-refinement-vs-hardware-transcendental` | arithmetic | ⚠️ Mixed (1.09x/1.01x M3; x86 re-gate: ParetoDominant 1.64x/1.66x — hardware never loses cross-arch) | ⚠️ `context_notes` | GPU fast-math Newton-refined recip/rsqrt → hardware single-instruction div/sqrt on CPU (Batch 62, microGPT-C distill — the honest-annotation class: exists to STOP the GPU→CPU fast-math port) |
| `loop-order-innermost-contiguous` | simd | ✅ 7.14-13.80x ParetoDominant | — | i-j-k nested accumulation whose innermost loop strides a 2D access's OUTER axis → i-k-j reorder (Batch 64 — innermost varies the LAST index; float sum-order changes → rel-error gate, not bit-identity) |
| `debug-assert-length-hoist` | complexity | ⚠️ Mixed regime-split (1.13x@1K / 0.67-0.71x@100K aarch64; x86 re-gate: 1.00x/0.92x — holds cross-arch) | ⚠️ `context_notes` | `debug_assert!` mentioning `.len()` erased in release drops the bounds-check-elision constraint → `assert!` (Batch 64 — the release-mode pessimization stop; scope: parameter/range-pinned indices only) |
| `sparse-set-bit-iteration-tz-blsr` | arithmetic | ✅ 4.08/3.90/1.54/1.34x ParetoDominant (1K/10K/100K/1M) | — | `for i in 0..64 { if (mask>>i)&1 != 0 }` → trailing_zeros + `m &= m-1` (BLSR) loop, popcount iterations (Batch 66, polars distill — the B68 rider documents the `x ^= 1 << tz` anti-shape: the xor-clear SERIALIZES count+clear vs the parallel Kernighan chains) |
| `sortedness-check-branchless-accumulate` | iterators | ✅ 2.98/2.72x ParetoDominant @100K/1M | ⚠️ `context_notes` | sortedness early-exit validation → branchless `sorted &= prev <= v` full scan (Batch 66, polars distill — usually-sorted regime; early-violation data flips it, the mirrored regime of `filter-count-to-any`) |
| `byte-find-position-to-memchr` | iterators | ✅ 5.00/19.25/25.31/25.37x ParetoDominant (32B/256B/4K/64K) | — | `xs.iter().position(|&b| b == lit)` on `&[u8]` → `memchr::memchr` (Batch 67, regex-automata distill — position is NOT std-specialized, contains IS; dep-gated on a direct memchr dep) |
| `varint-compose-word-single-store` | io | ❌ ParetoDominated (0/3 sizes × 3 regimes) | ⚠️ `context_notes` | per-byte `write_all` varint loop → 5-threshold ladder compose + single `write_all` (Batch 68, tantivy distill — **MEASURED HONEST LOSER on aarch64 AND x86 (0.40-0.50x there — harder; cross-arch re-gated 2026-08-19)**: LLVM strength-reduces the constant `%128 /128`, and OoO hides the call chain; the entry exists to STOP the folk port — the `precompute-table` class) |
| `unaligned-word-load-from-bytes` | io | ✅ 3.67-3.76x ParetoDominant (cross-arch: u64 1.4-2.2x, u32 1.2-1.4x both arches) | — | shift/or byte-assembly word read (8 loads + 7 shifts + 7 ors) → ONE unaligned `from_le_bytes` slice load (Batch 69, tantivy bitpacker distill — the Plan 013 T6 defer refuted: LOAD-count vs the varint rule's CALL-count; OoO cannot hide 8 L1 loads behind 1) |
| `keyword-map-to-match` | dispatch | ✅ 5.23-5.58x ParetoDominant | — | static-keyword `LazyLock<HashMap<&str, V>>` dispatch → plain `match` on string literals (Batch 61, oxc distill — LLVM lowers the match to a switch on string LENGTH + inline compares; the HashMap pays SipHash per lookup; oxc failed to beat the match with a gperf-style perfect hash, then found the length-switch in `--emit=llvm-ir`; static key sets only) |
| `usize-index-to-u32` | layout | ✅ 1.81-1.99x ParetoDominant | — | `usize` index/offset fields in high-volume structs → `u32` (Batch 61, oxc Span distill — halves the struct: 16B → 8B, twice the elements per cache line; oxc measured +5% on large parser files; domain must genuinely bound below 2³²) |
| `transposed-weight-register-gemv` | simd | ✅ 1.18-6.08x ParetoDominant | — | row-major per-row dot (serial scalar accumulator chain + horizontal reduce) → transposed weights, per-input broadcast FMA into output-vector accumulators (Batch 62, microGPT-C distill — the win GROWS at large nin: strict-FP Rust cannot vectorize the scalar accumulator chain without reassociation, so the row-dot degrades further; llama.cpp's fine row-major dots use explicit multi-accumulator SIMD — not what portable scalar code gets; architectural rule — weights pack transposed once at weight-freeze, no proposer) |
| `loop-order-innermost-contiguous` | simd | ✅ 7.14-13.80x ParetoDominant | — | i-j-k nested accumulation with a column-striding inner access → i-k-j reorder (Batch 64, Arun Nagaraj matmul-simd distill — the innermost loop must vary the LAST index: contiguous row walk; L1 misses 45%→12% in source; the win GROWS with n; float sum-order changes → rel-error gate; distinct from B62's repack rule — this reorders loops over an unchanged layout) |
| `debug-assert-length-hoist` | complexity | ⚠️ Mixed regime-split (annotated) | — | `debug_assert!` mentioning `.len()` is erased in release, dropping the constraint bounds-check elision needed → `assert!` (Batch 64, Shnatsel bounds-check-cookbook — 1.13x@1K / stable 0.67-0.71x@100K on aarch64; scope: parameter/range-pinned indices ONLY — data-dependent index arrays can never propagate the constraint) |
| `newton-refinement-vs-hardware-transcendental` | arithmetic | ⚠️ Mixed (annotated) | — | Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction `fdiv`/`fsqrt` on CPU (Batch 62, microGPT-C distill — 1.09x at 1K / 1.01x at 100K, hardware never loses; the annotation carries the regime inversion: on GPU div/sqrt are multi-cycle so approximate+refine inverts the verdict — the rule exists to STOP the plausible-looking GPU→CPU fast-math port; the relaxed-precision-estimate axis is separate + valid only where accuracy can relax) |
| `powi-vs-powf` | arithmetic | ~ Mixed (all-Tie) | ⚠️ `context_notes` | `x.powf(2.0)` → `x.powi(2)` (Batch 53 refutation: LLVM lowers both identically in release; idiom value = debug profiles + exponents ≥ 3) |
| `soa-hot-field` | layout | ~ Mixed (latency-bound) | ⚠️ `context_notes` | AoS wide-struct single-field scan → SoA split (Batch 53: serial f32 add-chains hide the win; pays on throughput-bound field consumers — count/min-max/transform) |
| `shrink-to-fit-after-build` | allocation | ❌ NotBenchable | — | Vec with over-allocated capacity → `shrink_to_fit()` after build (memory win, not speed; the realloc may even make build slower) |
| `box-large-enum-variant` | layout | ❌ NotBenchable | — | Enum with one `[u8; 1024]` variant → `Box<[u8; 1024]>` (shrinks common case from 1032→16 bytes; size_of win improves cache density) |
| `async-yield-point` | iterators | ❌ NotBenchable | — | Long async fn (no `.await`) → cooperative `yield_now()` every 100 iters (fairness/p99-latency win under contention, not single-task throughput) |
| `async-future-size` | layout | ❌ NotBenchable | — | Large locals across `.await` → scoped blocks (drops before next await; shrinks future state machine via `size_of_val`) |
| `bufwriter-batched` | io | ✅ 2.48-21.57x ParetoDominant | — | `File::write` per record → `BufWriter` batched (syscall amortization; Issue 012 / Bench 016) |
| `sort-key-cache-precompute-out-of-lock` | caching | ✅ 24.3-37.2x ParetoDominant | ⚠️ `context_notes` | comparator lock+clone per invocation → maintained sort-key cache refreshed once per data update (Batch 73, mnt.io room-list distill — the amortization curve visible in the sorts-per-update axis 1/4/16; allocs 164497→17 @16 sorts; REGIME: sorts-per-update ≈ 0 = pure refresh overhead; the source's 78× is end-to-end, our isolated mechanism reads 24-37×; composes with the narrow-accessor rule — production `RoomListItem.refresh_cached_data` reads through narrow accessors) |
| `lock-guard-narrow-field-accessor` | concurrency | ✅ 11.5-12.7x ParetoDominant | ⚠️ `context_notes` | whole-payload clone under read-guard → narrow accessor returning only the consumed sub-field (Batch 73, mnt.io room-list distill — allocs 100000→0 @100K; the source's +18% is end-to-end, the isolated mechanism reads 11-13×; type-blind for a parse-only proposer — suggestion-only) |
| `swar-digit-chunk-parse` | arithmetic | ✅ 2.35/1.75/2.52× ParetoDominant | ⚠️ `context_notes` | per-digit decimal parse loop (`val = val*10 + (b-b'0')`, serial chain) → SWAR 8-digit chunk fold — 3 and/mul/shift steps per 8 digits (magic 2561/6553601/42949672960001) guarded by the `0x3333` SWAR digit validation (Batch 74, simd-json distill — Muła ← simdjson ← simd-json provenance; **honest 1.01× tie @ 4 digits** — the source's own README regime note: gate on ≥8 digits remaining; composes with `unaligned-word-load-from-bytes`, sibling of `division-by-invariant-multiplication`; exhaustive gate incl. value AND stop-position identity) |
| `runtime-feature-detect-per-chunk` | dispatch | ~ Mixed (0/0/3 ties) | ⚠️ `context_notes` | CPU feature detection inside a per-chunk hot fn → resolve once (cached static / hoist / static cfg gate) — **measured Tie on rustc 1.97 aarch64** (the std::detect machinery const-folds through an `#[inline(never)]` boundary); the source's cost class (outlined `target_feature` calls per 8-digit group — their reason for static-gating SSSE3) survives for outlined dispatch / non-literal feature strings / heavier probes (Batch 74, simd-json distill — the honest-annotation record, the B68 varint precedent) |

| `bitplane-transpose-repeated-predicate-scan` | simd | ✅ 1.96/2.16/1.93× ParetoDominant (64 scans/call; verdict-stable ×2) | ⚠️ `context_notes` | repeatedly filtered integer column stored row-wise → encode-once bit-plane transposition (adaptive b from the column max) + zero-decompression bitwise predicate evaluation — the two-mask MSB→LSB recurrence (`gt |= eq & plane` / `eq &= plane`) is pure AND/OR word ops; b/32 the bytes + 64 values per word op (Batch 83, arXiv:2608.26368 distill, Research 097). **The honest ~2× is the LAYOUT half — Rust's scalar arm auto-vectorizes (~0.126 ns/elem vs the source's non-vectorized 0.65), never cite the source's 10.7× NEON headline isolated.** Regime: amortization (encode O(N·b) once; source break-even 7-25 scans; single-scan/write-once OUT — source's own 1M-row single-pass cells sub-1×), b ≤ 16 (b≈32 = 1× compression), range predicates (wide-type equality LOSES: source 0.5-0.7×), IN-lists ≤ ~k=8 (k·b passes), no point lookups. DOMAIN CONTRACT: predicate constant must fit the column width — out-of-range c returns 0 via one branch per query (paper §3.5 bias; **the G1 gate caught the unguarded form**). Data-oblivious: plane throughput is selectivity-constant (source <6% spread vs scalar ~4.5× branch-prediction swing — external evidence for `branchless-conditional`) |
| `weight-tile-consumption-order` | layout | ✅ 1.32/1.35/1.35/1.56× ParetoDominant (grows with working set; verdict-stable ×2; allocs equal) | suggestion-only | row-major single-token matvec (each output row re-walks the full activation x; once W-row + x exceed L1 the activation is evicted and re-fetched once PER ROW) → weights repacked ONCE at load time into consumption-ordered 64×512 tiles — the matvec walks tiles sequentially, the x slice (2 KB) L1-resident across the tile's 64 rows, TILE_ROWS× less x traffic. M3 shift: 128 KB P-core L1-d (4× the source's Sandy Bridge) moves the crossover right — measured even the L1-resident arm at 1.32× (the source's own 1×-L1 cells ~1.12×; below the boundary the mechanism is weaker, not absent). Regime: single-token GEMV / memory-bound matvec, repack setup-amortized; composes with `transposed-weight-register-gemv` (tile for cache, transpose within for registers); rel-error gate (tile partials ≠ row-major order). Proposer-less: cannot prove the repack site is load-time — a mid-loop repack would regress (Batch 85, arXiv:2608.23841 cflow distill, Research 098) |
| `software-prefetch-storage-bound` | io | ⚠️ NotBenchable here / source-cited ParetoDominated-adjacent (x86-only evidence) | ⚠️ `context_notes`, proposer-less by design | software prefetch sprinkled on a bandwidth/storage-bound stream → remove it: the miss pipeline is already saturated, prefetch adds only issue slots + cache pressure. Source A/B: PF ±0.1% at 64 MB, PF=1 4% WORSE at 4.7 GB direct-I/O; at decode scale the dedicated prefetch thread burned ~48% of decode time (stride-256 expert walks) — disabling it alone was the source's single largest decode win at 1.92×. Two-regime: compute-bound loops CAN hide latency (the x86 folk-wisdom home); storage/RAM-bound streams lose. No aarch64 re-gate (no stable Rust prefetch intrinsic — the powi-vs-powf cross-arch annotation precedent); the rule exists to STOP the plausible-looking prefetch port (Batch 85, arXiv:2608.23841 cflow distill, Research 098) |
| `encoded-layout-borrowed-view` | allocation | ✅ 41.98/39.38/29.95× ParetoDominant | ⚠️ `context_notes` | generic to_string() materializer over dictionary-encoded data (few distinct + index/row) → borrowing layout-enum view `Option<&str>` — "the 3 strings become 1000" trap: a generic accessor silently re-expands the compact encoding (allocs 4,000,004→0, bytes 156 MB→0 @ R=1M/S=100/L=4; REGIME S≪R — at S≈R the encoding is pointless; distinct from `avoid-clone`/`collect-avoidance`; never cite the source's 35% dictionary anomaly — that's its fallback's extra lookup; Batch 75, GreptimeDB PR #8587 distill, Research 089) |
| `group-keys-borrow-allocate-on-first-sight` | allocation | ✅ 1.92/2.27/2.37× ParetoDominant | ⚠️ `context_notes` | per-row owned group key (clone into map key every row) → hash borrowed labels + `HashMap<u64, Vec<usize>>` candidate list + FULL compare + clone only on first sight + one final sort — R·L copies become S·L (allocs 70,133/700,133/2,100,133 → 8,019 FLAT at S=1000 — the 7R+2S→8S+30 model exact). THE DECISION CONTENT: std `entry()` APIs require an OWNED key, forcing per-row materialization — the manual hash index is the escape. Correctness contracts: candidate LIST not single index (collision merge), final sort preserves ordered output, exhaustive compare (prefix keys); mirror regime of `btreemap-sorted-iteration`; REGIME R/S — at S≈R first-sight IS per-row + the sort is pure overhead (Batch 75, Research 089) |
| `sorted-input-previous-row-fast-path` | complexity | ~ Mixed (adjacent 12.92× / interleaved 0.99× tie) | ⚠️ `context_notes` | hash-every-row grouping over (mostly) sorted input → compare against the previous row's group FIRST, hash fallback on miss — ~S hashes instead of R on sorted runs ("correctness does not depend on it" — the fallback is the contract); DERIVED isolated bound ~2.1-2.8x (the source's 10-16x stacks all three mechanisms); win scales with hash-vs-compare cost ratio (long string keys max, scalar keys → tie); family note: ADDS a pre-check before expensive work — does not contradict the early-exit-removal rules; REGIME = hit rate (Batch 75, Research 089) |
| `debug-stat-full-scan-out-of-hot-path` | complexity | ✅ 168.91/1929.27/5851.00× ParetoDominant | ⚠️ `context_notes` | per-insert aggregate statistic recomputed over the ENTIRE collection → out of the hot path (removed / debug-gated / maintained incrementally) — the mean-degree sanity stat full-scanned Meilisearch's vector store per insert batch: "Removing this reduces the time from 559s to about 2–3s and brings the complexity down to O(n)". A diagnostic eagerly computed in a hot path is a hidden O(collection) tax that GROWS with store size; the maintained-aggregate form keeps the stat at O(1). G1 unit gate: maintained == scan on mean-bits. REGIME N/B (at N≈B the scan touches what the insert touches anyway — Tie); distinct from `sort-key-cache-precompute-out-of-lock` (per-comparison under lock) (Batch 76, Research 090) |
| `eager-pointer-collect-lazy-fetch` | caching | ✅ 1903.70/5414.62/21353.90× ParetoDominant | ⚠️ `context_notes` | full-scan materialize a HashMap of ALL entry pointers to serve a SUBSET of random reads → direct/lazy access of only the needed entries — "polluting the cache with useless, unnecessary disk pages just by iterating over all of them to fetch pointers" → "only the necessary pages are loaded". An everything-index for a subset pays the scan TWICE: time + cache eviction of the pages the subset needs (allocs 1→0, bytes 35.7 MB→0 @ N=1M/K=1K). REGIME K/N ≪ 1 one-shot access; at K≈N the build amortizes; repeated-same-subset wants the persistent index (`sort-key-cache-precompute-out-of-lock`) instead; distinct from `collect-avoidance` (alloc staging — the load-bearing half here is working-set residency) (Batch 76, Research 090) |

| `narrow-counter-overflow-promotion` | layout | ⚠️ ParetoDominated 0.64/0.64/0.65× (honest-annotated — the memory-for-speed trade) | ⚠️ `context_notes` | hot counter arrays u64 "just in case" → u32 + checked-promotion sparse sidecar — the win is MEMORY (2x counters/line, allocs 8N→4N bytes exact; Datadog's motivation: the 2,100 GiB DDSketch infeasibility), NOT update-path wall-clock (the per-update `is_empty` guard + `checked_add` branch cost 35-55%; flat ratio across sizes = branch-bound, not bandwidth-bound). REGIME: memory-bound systems + sweep-dominated access. Distinct from `usize-index-to-u32` (domain-bounded index fields vs runtime-escape-hatch value counters). G1: totals bit-identical incl. u32-boundary crossings (Batch 78, Research 092) |
| `cache-line-block-partitioned-lock` | concurrency | ✅ 1.12/3.91/2.98/3.03× ParetoDominant (2-12t) | ⚠️ `context_notes` | one `Mutex<Vec<u64>>` guarding a shared point array → K-items-per-cache-line-block locks (`Mutex<[u64; 8]>` blocks, `#[repr(align(128))]`) — contention confined to within-block pairs (~8x fewer collisions). DIFFERENTIATION vs `align-64-false-sharing`: that pads independent slots against FALSE sharing; this partitions the TRUE sharing of ONE array at lock granularity (they compose). The first `ALLOC_AXIS_EXEMPT` rule (contended macOS pthread_mutex allocates nondeterministically — probed 41-44/512-lock runs). REGIME: multithreaded bump workloads; single-threaded pays the division for nothing (Batch 78, Research 092) |
| `serialize-once-byte-buffer-store` | allocation | ✅ 4.26/3.52/2.90× ParetoDominant (verdict-stable ×2) | ⚠️ `context_notes`, suggestion-only | frozen records as per-field heap allocations (name `Box<str>` + payload `Vec<u8>` PER record) → serialize each record ONCE into ONE length-prefixed byte buffer + u32 offset table — 2 allocations TOTAL regardless of record count (alloc axis EXACT 200,001 → 2 @ n=100K; bytes 7.90 MB → 4.70 MB — allocator round-up + headers gone). The insert path is the measured surface (the source's near-isolated +13% insert; END-TO-END +43%/−19% — never cite isolated). The win SHRINKS with n (copy dominates per-alloc overhead at scale) — still dominant everywhere. REGIME: sequential-rebuild consumers (cache → response rebuild); in-place random-mutators lose (every patch = buffer rewrite). ANTI-SHAPE: full-wire-message storage (the source's own rejected alternative — variant flags force two copies or post-build filtering). Distinct from `encoded-layout-borrowed-view` (read-side accessor; this is the store representation) (Batch 86, Cloudflare Big Pineapple distill, Research 100) |
| `context-equal-field-elide` | allocation | ✅ 3.12/3.01/2.00× ParetoDominant at ~80% context-equal (verdict-stable ×2) | ⚠️ `context_notes`, suggestion-only | per-record field that equals the enclosing context (their DNS owner == the queried domain = the cache key, in hand at every lookup) → `Option<Box<T>>` with None = "same as context", resolved at read via `as_deref().unwrap_or(context)` — ~80% of records store NO name (allocs 100,001 → 20,056 @ n=100K — the ~20% non-equal keep theirs, the regime made visible; bytes 4.04 MB → 2.76 MB). The fixed arm pays the insert-time equality compare + the read branch and STILL dominates. REGIME (two-sided): high %-equal wins; LOW %-equal is pure overhead (unmeasured below ~50%). Composes with `serialize-once-byte-buffer-store` (the elided marker is one bit in the record header). Distinct from interning/dedup: nothing is shared or canonicalized — the value is ABSENT because the context re-derives it (Batch 86, Research 100) |
| `bitflag-pack-when-footprint-dominates` | layout | 📐 NotBenchable-speed — size axis 12 → 8 pinned by gate | ⚠️ `context_notes` | stored-at-scale struct with a cluster of bools → ONE bitflag byte — a record held millions of times multiplies every byte (the source's per-entry 953 → 420 B fleet table; their own observation: the PADDING COLLAPSE shrank the struct by MORE than the raw bool bytes). CROSS-CITE the kernel_opt `bool-fields-over-bitset-hot-reads` INVERSE regime: per-tick READ flags stay bools (decode ALU dominates) — pack only when FOOTPRINT dominates. DEFAULT-REPR CAVEAT: field reordering makes the collapse layout-dependent — MEASURE size_of before/after (the gate pins 12 → 8 for THAT layout; evidence, not a promise). Composes with `serialize-once-byte-buffer-store` (a flag byte packs into the record header) (Batch 86, Research 100) |
| `runtime-int-exponent-powi` | arithmetic | ✅ ParetoDominant 2.57/2.81/2.66× @1K/100K/1M aarch64 — cross-arch re-gated same day: all-Tie 0.99/1.00/1.00 on x86 | ⚠️ `context_notes` | `powf(x, n as f32)` with a RUNTIME integer exponent → `x.powi(n)` binary exponentiation — a runtime exponent gives LLVM no instcombine bailout, so `powf` stays an opaque libm call (the case `powi-vs-powf`'s annotation explicitly deferred); the win is the slow-libm-host regime (aarch64/GPU/embedded — the newton-refinement inversion class), an idiom swap on fast-libm x86; NOT bit-identical, rel-error-gated; GPU evidence: RDNA4 RoPE 692 → 76 µs/launch (Batch 77, lucebox rope.cu distill, Research 091) |
| `filter-before-expensive-transform` | iterators | ✅ ParetoDominant 11.85/11.30/6.61× @1K/30K/300K (verdict-stable ×2; allocs 300,014 → 18,764 — the ~94% discarded renders ARE the mechanism) | ⚠️ `context_notes` | `map(expensive).filter(relevant)` → re-express the predicate on the RAW input and filter FIRST — win ∝ discard-fraction × transform cost; IMPOSSIBLE when the predicate is not re-expressible on the input (the source's own precondition); over-permissive re-expression is correctness-safe when consumers tolerate extras; composes with `filter-map-fuse` (Batch 91, Rustdoc filter-first distill — PRs 159623/159779/159854, Research 106) |
| `decision-projection-before-construction` | complexity | ✅ ParetoDominant 24.58/24.38/24.37× (verdict-stable ×2; allocs 300,000 → 0, bytes 9.60 → 0 MB) | ⚠️ `context_notes` | transform-then-project to decide → compute the INVARIANT projection on the raw input, pay the canonicalization only behind the decision (PR 159854's head-before-param_env, −18%); invariance precondition pinned AS A TEST; distinct from `ok-or-else`/`unwrap-or-else-lazy` and `filter-map-fuse` (Batch 91, Research 106) |
| `coalesce-nearby-reads-bounded-gap-cacheability-gated` | io | ❌ NotBenchable (source-reported; the cacheability coupling is the decision content) | ⚠️ `context_notes` | random-addressed row reads within a bounded byte gap (64 KiB) → ONE pread merge; the gap budget COLLAPSES to 0 on direct/F_NOCACHE descriptors (over-read bypasses the page cache and is never free) and is cheap only where the page cache repays it; gap-BOUNDED not whole-range (an unbounded merge re-reads the shard — the `eager-pointer-collect-lazy-fetch` failure shape on the transport axis); re-selection preserves caller order incl. duplicates; distinct from `buffered-io` (sequential window) / `software-prefetch-storage-bound` (hides latency vs removes syscalls — compose freely) (Batch 94, oMLX PR #3260 safetensors reader, Research 110) |
| `mir-destprop-branch-site-merge` | dispatch | ✅ ParetoDominant 1.04/1.24/1.25× @1K/100K/1M (50/50 mixed stream; verdict-stable; 0 allocs) | ⚠️ `context_notes` | a hot conditional dispatch whose arms load KNOWN fn items and converge into one call → split into TWO DIRECT tail-jumps: rustc ≥1.92 MIR DestinationPropagation lowers the merged form to a cmov of the two code addresses + ONE indirect jmp site mixing branch-target history forever; the split form emits two direct jumps with per-site history (source: wasmi v2.0 tail-call dispatch, ~2800 → >4200 CoreMark); REGIME: compile-time-known code pointers ONLY — runtime-opaque dispatch reads Tie (both forms keep indirect calls); TOOLCHAIN REGIME: re-gate on rustc bumps (Batch 99, wasmi blog distill, Research 119) |
| `dead-result-timing-loop-folded-under-lto` | complexity | ❌ NotBenchable (measurement-honesty axis — bench_spec None; the rule's content IS that the baseline folds to zero) | ⚠️ `context_notes` | a timed loop discarding its work result (`let _ = f()`) measures ZERO under rustc 1.98.1 + fat LTO — the fold eliminates inlined-callee work even through callee-side `black_box`; fix = chained state (xorshift-PRNG shape, each iteration consumes the previous accumulator — unprunable) + consumer-side `black_box` + the loud nonzero-baseline assert (NaN compares false against everything: a `>=` PASS bar fails loud on a zero baseline while a `<=` budget bar passes silently — assert BEFORE the ratio). Both draft fix-patterns REFUTED by the G1 fixture (XOR sink cancels over GF(2)-linear work; wrapping_add sink collapses to the seed-independent constant) (Batch 109, Plan 080, Research 132) |
| `hot-byval-move-over-memcpy-threshold` | layout | ~ Mixed 1/0/1 on M3 (1.29×@1K / 0.99×@1M, verdict-stable ×2, allocs 0/0 — both arms inline at 136 B; the n=1K cell is warm-up-dominated, never cite it); x86: the 136 B loop = 1.32-1.35× the 48 B loop per iteration | ⚠️ `context_notes` | a hot by-value move of a >128-byte type → shrink under the target's ceiling / pass by ref / avoid the move (rustc #160005: a hot 136 B type paid ~half its compile time in memcpy calls; DHAT copy-profiling the finding instrument). THE ARCH FINDING: the threshold is an LLVM BACKEND CONSTANT and arch-shaped — aarch64 INLINES a 136-byte move as ldp/stp pairs (ceiling in (136, 320], a 320-byte move emits `bl _memcpy`) while x86-64 emits `movl $136; callq memcpy` per iteration; fix doctrine arch-independent + never-negative (Batch 112, nnethercote byval-move distill, Research 134 / Plan 083; suggestion-only) |
| `atomic-vec-zero-cost-convert` | concurrency | ✅ MEASURED ParetoDominant 4/0/0 (M3 Max release, Batch 135 benches, verdict-stable ×2: 1.83/1.78/1.57/1.33× at n=100/1K/10K/100K; alloc axis EXACT 2→1 — both arms pay one shared clone, the buggy conversion adds the malloc + copy, the fixed one reuses the buffer) | ⚠️ `context_notes` | Vec<T> ↔ Vec<AtomicT> conversion by Atomic::new-per-element collect from a BORROWED source (fresh malloc + full copy every conversion — or permanent atomics, or transmute) → into_iter().map(Atomic::new).collect() on the OWNED vec: same size+align (AtomicU32 ≡ u32) makes std's in-place collect reuse the buffer and the loop compile out (the article's x86 asm: movups, no loop). Switch the representation PER PHASE — plain owned storage, atomic only for the shared-write window; convert back by CONSUMING via into_inner(), move across phases with mem::take. FREE only when layouts match; a borrowed source still forces the fresh allocation; a wrapper with Drop falls back to a real copy. Distinct from `mutex-to-atomic` (permanent-storage direction) and `mem-take` (the boundary move — they compose). Fixtures original; source: David Lattimore, "RustForge wild performance tricks" (2025-09-02), technique 3 (Batch 135 / Plan 114 / Research 155) |
| `vec-reuse-across-lifetime-change` | allocation | ⚠️ MEASURED Mixed 2/0/2 (M3 Max release, Batch 135 benches: 1.49/1.07× @100/1K then Tie@10K/100K) + **alloc axis EXACT 1→0 at every size — the deterministic axis is the load-bearing cell** (the elision is proven by integers, not wall-clock; timing ties once the refill dominates the malloc — the `reuse-allocation-clear` Mixed shape) | ⚠️ `context_notes`, suggestion-only | a hot loop allocates a fresh Vec<&'iter T> (or any same-layout Vec<U>) every iteration because the hoisted buffer cannot be TYPED across the element lifetime (no borrowed-scratch signature typechecks) → OWN the buffer and retarget it in place: const size/align assert (inline const — mismatch = compile error at the call site) + clear() (map body unreachable, loop compiles out) + into_iter().map(unreachable).collect() — std's same-layout collect hands the SAME allocation back re-typed. The 'static element type is a CALLER CONTRACT: cleared before any referent dies; panic between take and restore loses the buffer, never UB. Distinct from `reuse-allocation-clear` (caller-passed scratch of ONE fixed element type — the compiler-can't-hoist obstacle; this is the borrow-checker-can't-type obstacle). Extension arm recorded in-rule: MaybeUninit<&'static> shadow struct for owned+borrowed field mixes (never observe the shadowed field empty). Fixtures original; source: David Lattimore, "RustForge wild performance tricks" (2025-09-02), technique 4 (Batch 135 / Plan 114 / Research 155) |
| `partitioned-mut-slices-zero-sync` | concurrency | ✅ MEASURED ParetoDominant 4/0/0 (M3 Max release, Batch 135 benches; sizes = THREAD counts 1/2/4/8: 6.90/9.12/11.81/6.13×; alloc axis EXEMPT — contended macOS pthread mutex slow path, the Batch-78 probe class) | ⚠️ `context_notes` | shared Vec mutated through one Mutex (lock per bump) although each worker writes a known DISJOINT contiguous range → partition ONCE (chunks_mut / split_at_mut) and hand each worker its own &mut [T] — zero locks, zero atomics, plain stores; per-cell results identical (one owner per cell). Regime: provably-ranged writes (contiguous id ranges per object, row bands, shards); ANTI-REGIME: scattered/overlapping access (fine-grained locks/atomics), read-mostly (RwLock), tiny arrays. Distinct from `cache-line-block-partitioned-lock` (partitions LOCK GRANULARITY for scattered per-item writes; this ELIMINATES synchronization via ownership — they compose), from `parallel-disjoint-pread-hides-cold-page-latency` (io positional reads), and from `mutex-to-atomic`. std-only fixtures (thread::scope + chunks_mut — no rayon dep; the article's split_off_mut is nightly-shaped, the portable core is what lands). Source: David Lattimore, "RustForge wild performance tricks" (2025-09-02), technique 1 (Batch 135 / Plan 114 / Research 155) |

## Validator (cargo test — behind `rust_perf_verify`)

`PerfVerifier`:
1. Create a temp crate under `std::env::temp_dir()/riir_perf_verify/<blake3_hash>/`
2. Write `Cargo.toml` + `src/lib.rs` (the code being verified)
3. Run `cargo test --message-format=json`
4. Parse JSON for `compiler-message` errors + `suite` pass/fail counts
5. `test_passes = exit_ok && failed == 0 && compile_errors == 0`

**Why both exit status AND JSON parsing:** a compilation failure produces NO
`suite` event (the test runner never runs), so parsing pass/fail counts alone
returns `(0, 0)` → `test_passes = true` (vacuously). The exit status +
`compiler-message` error count distinguish "compile failure" from "no tests,
all good".

## Bench validator (Pareto verdict + deterministic alloc axis — behind `rust_perf_bench`)

`PerfBenchValidator` (the **promotion gate** for the domain):
1. Generate a temp **bin** crate embedding the rule's `BenchSpec.harness` +
   a timing loop (dynamic calibration → warmup → median-of-samples) + a
   counting `#[global_allocator]` (Issue 014 T1 — fully-qualified paths, zero
   `use` statements, so injected code can never collide with harness imports).
2. Run `cargo run --release` on the temp crate.
3. Parse JSON lines:
   `{"size":N,"buggy_ns":X,"fixed_ns":Y,"buggy_allocs":A,"buggy_bytes":B,"fixed_allocs":C,"fixed_bytes":D}`.
4. Compute per-size outcome (Faster/Slower/Tie at ±5% threshold).
5. Aggregate into a `ParetoVerdict` (ParetoDominant if faster on majority).
6. **Alloc axis** (Issue 014 T1): after the timing loops, one steady-state
   call of each side with the counters reset — exact integers (warmup has
   settled one-time init like LazyLock compilation).
   `run_bench_with_determinism(n)` re-executes + asserts the axis is
   bit-identical across runs — the determinism proof (variance = harness
   bug, not noise).

Rules without a `BenchSpec` are honestly marked `NotBenchable` — they need
external runtimes (Bevy, WASM) a temp crate cannot provide, or (the Issue 014
class) their cost metric is not wall-clock.

See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) for
the original promotion-gate results, [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md)
for the Phase 4 finalization, [Bench 016](../../.benchmarks/016_rust_perf_v2_sorting_io_copy_goat.md)
+ [Bench 017](../../.benchmarks/017_rust_perf_v3_data_structure_choice_goat.md)
for the Issue 012/013 expansions, and [Bench 020](../../.benchmarks/020_rust_perf_v4_alloc_axis_goat.md)
for the alloc-axis sweep on the 50-rule corpus (G-det: 40/40 rules stable
across 3 repeat runs).

## Caveat (Proposal 001 caveat 3 — closed by Bench 013, re-confirmed by Benches 015/016/017/020)

The original caveat called perf "the weakest link" because perf has no
single ground truth. **Bench 013 closes this**: the Pareto validator now
provides the real perf ground truth. **Bench 015 re-confirms on the expanded
corpus**: 29/31 benchable rules are Pareto-dominant (clear wins); 2/31 are
honestly Pareto-dominated (compiler optimizations, regime-dependent benefits);
2 of the 29 ParetoDominant are regime-fragile (1 slower size each, kept
annotated) — exactly the kind of honest signal the caveat predicted.
**Bench 020 adds the deterministic complement**: the alloc axis (exact
integers, stable across repeat runs on 40/40 rules) removes the noise floor
from the second half of every perf claim — timing still decides the verdict,
but allocation cost is now a provable integer, not a probabilistic estimate.

## Drafter + pipeline integration (`context_notes` propagation, 2026-08-13)

The 4 annotated rules (2 `ParetoDominated` + 2 regime-fragile) carry a
`context_notes: Some(...)` annotation on
[`PerfRule`](../../src/domains/rust_perf/mod.rs). The annotation is **rule-level
metadata** — it describes the RULE, not a specific fix. The pipeline looks it
up from the corpus at fix time + surfaces it on ALL paths (drafter / memory /
Stage2):

```text
PerfRule.context_notes
      (struct field — the source of truth for rule metadata)
         │
         ▼
HealEntry::context_notes()          (trait method, default None)
         │
         ├──→ DraftedFix.context_notes  (L1 drafter output — for standalone drafter use)
         │
         ▼
HealPipeline::heal()                (corpus lookup at the top of heal())
         │
         ▼
HealResult.context_notes            (pipeline output — same annotation on all paths)
         │
         ▼
consumer                            (CLI/IDE/CI displays ⚠️ when Some)
```

**The drafter + pipeline do NOT skip or downgrade annotated rules.** The rule
is a genuine pattern — just not an unconditional win in the default
configuration. The consumer (CLI caller, IDE plugin, CI gate) reads
`DraftedFix::context_notes` (direct drafter API) or `HealResult::context_notes`
(self-evolve pipeline API) and decides whether their context matches
(multi-threaded? POPCNT target? large N?).

This is the honest design: the pipeline surfaces the annotation; it does not
make policy decisions about whether a rule applies. The policy belongs to the
consumer, who knows their context.

### Pipeline path coverage

| Path | `context_notes` | How |
|---|---|---|
| **Drafter** (`HealSource::Drafter`) | ✅ from corpus lookup | `heal()` looks up the corpus entry by `rule_id` + calls `HealEntry::context_notes()`. |
| **Memory** (`HealSource::Memory`) | ✅ from corpus lookup | Same corpus lookup — `FixTrajectory` doesn't carry the annotation (it's fix-level data), but the corpus is the source of truth for rule-level metadata. |
| **Stage2** (`HealSource::Stage2Adaptive`) | ✅ from corpus lookup | Same corpus lookup — the adaptive drafter serves a cached verified fix; the annotation is still the rule-level metadata from the corpus. |

### Consumer display

Examples that print fix output now surface the annotation:
- `heal_demo.rs` — prints `⚠️ Context-dependent: <notes>` when `DraftedFix::context_notes` is `Some`
- `self_evolve_real_world_demo.rs` — appends `⚠️` marker to the per-lint line when `HealResult::context_notes` is `Some`

Non-breaking: `HealEntry::context_notes()` has a default `None` impl, so
`clippy_lints` + `kernel_opt` entries need no changes — they are unconditional
wins (or honestly `NotBenchable`) and return `None`. The new
`HealResult::context_notes` + `Candidate.context_notes` fields are additive
(existing test assertions only read `.source`/`.evolution_score`/`.fixed_code`/`.parses`).

## Cargo-in-cargo test constraint

Tests that spawn `cargo test`/`cargo run` are `#[ignore]`'d ("slow: runs
real cargo subprocess (seconds); also needs `--test-threads=1` under
cargo-in-cargo"). This mirrors the `clippy_verify` test pattern — concurrent
cargo invocations against temp crates race on cargo's build cache lock.

## Build-config recommendations (not fixable)

The rulewright edits `.rs` via `syn`. PGO / LTO / codegen-units / BOLT are
`Cargo.toml` + CI-workflow changes — **out of scope for the rulewright.** This
section documents them as recommendations, not corpus rules. Source:
nnethercote/perf-book `build-configuration.md` + Kobzol (cargo-pgo).

| Config | What it does | Expected gain | Trade-off |
|---|---|---|---|
| `codegen-units = 1` | Lets LLVM see the entire crate for better optimization | 1-5% CPU | Slower compile (LLVM can't parallelize per-crate) |
| `lto = "thin"` | Cross-crate inlining (LTO) — sees through `pub fn` boundaries | 2-10% | Slower compile (link-time analysis across all deps) |
| `panic = "abort"` | Removes unwinding machinery (~6KB binary size + some codegen paths) | 1-3% binary | Can't catch panics; no unwinding for Drop |
| `strip = true` | Strips debug symbols from the release binary | 30-70% binary size | No stack traces in production |
| **PGO** (`cargo-pgo`) | Profile-Guided Optimization: instrument → run representative workload → rebuild with profile | 10-20% CPU | Requires a representative workload run; CI integration complexity |
| **BOLT** (post-link) | Binary Optimization Layout Tool — reorders binary layout post-link for better i-cache | 2-5% on top of PGO | Requires LLVM BOLT; Linux x86-64 only |

**Recommendation:** for CPU-bound production binaries, the ROI hierarchy is:
1. `codegen-units = 1` + `lto = "thin"` (easy, always worth it)
2. PGO if the workload is stable + representable (biggest single gain)
3. BOLT if on Linux x86-64 + already doing PGO (incremental on top)

The rulewright does NOT suggest these — they're Cargo.toml/CI changes, not code
patterns. A separate "build-config audit" tool (out of scope) could check for
them.

## Proposers (Plan 005 Phase 4 + Phase 6 T6.1 — the `--fix` upgrade)

**DONE 2026-08-14.** [`src/domains/rust_perf/proposers/`](../../src/domains/rust_perf/proposers/)
ships the top-5 Pareto-dominant rules as `syn` AST detectors +
**span-preserving source edits**: each proposer walks the parsed file
immutably and emits `SourceEdit` byte ranges (via `proc-macro2`
`span-locations`); `apply_edits` splices them back-to-front. Everything
outside a matched span stays byte-identical — no whole-file reformat
(Bench 019). The latent matcher (`--suggest`) picks/reports rules; the
proposer's visitor is the application gate — retrieval scores never
auto-apply.

| Rule | Rewrite | Notes |
|---|---|---|
| `vec-with-capacity` | `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(<cap>)` | capacity only from unambiguous sources (`0..n`, `0..=n`, `&path`, `path.iter()`); bare-path iterables skipped (could be `.len()`-less iterators) |
| `entry-vs-contains-insert` | `if !m.contains_key(&k) { m.insert(k, v); }` → `m.entry(k).or_insert(v);` | statement-level rewrite (preserves `()` type); walks all stmts file-wide |
| `static-str` | `-> String { "lit".to_string() }` → `-> &'static str { "lit" }` | signature-visible (return type); `.to_owned()` + `String::from` forms too |
| `bool-to-bitpack` | `flags: &[bool]` count → `flags: &[u64]` + `count_ones().sum()` | signature-visible (param type); param name preserved (no rename hazards) |
| `fxhashmap` | std `HashMap` → `rustc_hash::FxHashMap` (use swap + group slimming + path rewrite) | fixed crate needs the `rustc-hash` dep; glob/`pub use`/fn-local forms skipped |
| `hashset-insert-bool-return` | `if !s.contains(k) { s.insert(k); .. }` → `if s.insert(k) { .. }` | insert stmt deleted via the shared `delete_stmt_edit`; key equivalence modulo leading `&`/`*` layers (2026-08-17) |
| `collect-append-to-extend` | `let mut staged = it.collect(); recv.append(&mut staged);` → `recv.extend(it);` | single-use staging gate (exactly 2 ident occurrences file-wide); first-mention adjacency in one block (2026-08-17) |
| `vec-macro-zero-fill` | `let mut v = Vec::new(); v.resize(n, 0);` → `let mut v = vec![0; n];` | zero int/float literals only; `v` stays alive for later uses; a read-only-after binding may surface a rustc `unused_mut` warning — dropping `mut` unconditionally would risk E0596 on unknown mutators, which is strictly worse (2026-08-17) |
| `distance-sq-no-sqrt` | `(x*x + y*y).sqrt() < r` → `(x*x + y*y) < r * r` (either operand order) | sum-of-squares receiver gate (textually-identical mul operands — provably non-negative); pure threshold only (path or non-negative literal — the text is duplicated as `t * t`, so side-effecting exprs skip); negative threshold + 1-ulp boundary caveats documented in the proposer (2026-08-17) |
| `hashmap-with-capacity` | `let mut m = HashMap::new();` + first-mention for-loop over a len-able iterable → `HashMap::with_capacity(iter.len())` | capacity is semantically a hint (any n is behavior-neutral); bare-path/`&path`/`iter()`/`iter_mut()` iterables with path-or-field-chain bases; range loops + `into_iter` skip; plain `HashMap::new()` path only (turbofish/qualified skip — the splice must not rewrite the path) (2026-08-17) |

The 2026-08-17 additions are the Batch 48 follow-up: the three parse-clean
ParetoDominant rules whose proposers were deferred pending bench verdicts
(measured ParetoDominant in Bench 020 — now `--fix` eligible). Two syn facts
learned landing them: **a type annotation wraps the pattern** (`let mut v:
Vec<u8> = ..` parses as `Pat::Type` around the ident — unwrap before
matching), and **`LitFloat::base10_digits` keeps `.`/`e`/sign separators**
(`"0.0"` → `"0.0"` — filter to digit chars before the all-zero check).
`filter-count-to-any` stays proposer-less BY DESIGN (ParetoDominated
0.82-0.90x — a losing rewrite never gets a `--fix` path).

The Batch 53 follow-up (same day) landed the two Batch 53 ParetoDominant
proposers (`distance-sq-no-sqrt` 1.21-1.24x, `hashmap-with-capacity`
2.4-3.1x — Bench 038). The other two Batch 53 rules stay proposer-less BY
DESIGN with measured reasons: `powi-vs-powf` is ALL-TIE 1.00x in release
(zero gain gets no `--fix` path — the `filter-count-to-any` rationale
applied to a tie) and `soa-hot-field` is architectural (struct layout,
suggestion-only). One precedence pitfall learned landing the distance-sq
proposer: the reversed form must emit `t * t OP sos` — emitting `t OP
sos` after a bare `t` (`t OP t * sos`) parses as `t OP (t * sos)` because
`*` binds tighter than every comparison; the squared threshold must be a
complete left operand before the op appears (caught by the
`reversed_operands_fire` fixture).

The `for_each_fn` shared helper (now in
[`edit_emitter.rs`](../../src/edit_emitter.rs)) scans free fns + impl
methods + nested inline `mod` blocks. `--fix` CLI wiring shipped with
Phase 6 T6.1 (Bench 019); the clippy proposers joined the same emitter in
T6.4; `delete_stmt_edit` (the line-scaffolding-aware statement deletion)
joined in the 2026-08-17 batch. G5 real-repo validation precedent: Bench 019
(katgpt-pruners, 20 edits landed) + Bench 031 (`--fix --verify`, 14/12/2).

## Source

- [`src/domains/rust_perf/mod.rs`](../../src/domains/rust_perf/mod.rs) — `PerfRule` + `RustPerfDomain`
- [`src/domains/rust_perf/corpus/`](../../src/domains/rust_perf/corpus/) — `build_perf_seed_corpus()` in `mod.rs` + 22 verbatim entry slices (`entries_01..22`, one `BenchSpec` helper set per slice) + `tests.rs` — the 2026-09-10 scripted split of the former `corpus.rs` monolith (`scripts/split_rust_perf_corpus.py`; entry order + BLAKE3 commitment unchanged at split time; now 195 rules / 99 `BenchSpec`s — Batch 219 (2026-10-10, Plan 215 slot 2 / Research 251): 11 NotBenchable suggestion-only rules (the TensorFold native-engine serving/timing pack @ `f1aef9c3` — the cost-table hygiene pair, the prompt-cache pair + fingerprint persistence, the transport pair, the thermal hysteresis band (new 13th category), the load-qualification probe, the Metal keepalive, the display carveout), no new BenchSpecs; Batch 211 (the 4-lead conversion mini-batch, 2026-10-08): 1 NotBenchable suggestion-only layout rule (`repr-align-register-partition-store-forwarding` — the bevy #26042 EntityLocation alignment law: a hot Copy struct whose register-load partition disagrees with the consumer's store partition pays a stack round-trip on every partial-field read; one `repr(align(8))` tips the partition; the rust #163916 alignment-drop face rides as the rider — one law, two faces: register partition + container footprint), no new BenchSpecs; Batch 181 (the mlxfast-bonsai2 arena distill): 2 NotBenchable suggestion-only rules (`seed-warm-the-scored-seam` complexity — the measurement-honesty family's SEAM-WARM member, + the per-depth-oracle rider; `mask-memo-hoist-to-owner-lifetime` caching — the owner-lifetime geometry memo), no new BenchSpecs; Batch 179 (the rsconstmap distill): 3 MEASURED ParetoDominant benchable rules (`paired-probe-layout-present-vs-absent` layout + `block-hash-before-gather-mlp` iterators + `immutable-static-map-filter-selection` complexity, +3 BenchSpecs — the first `[dep: constmap]` external-map entry, all with in-bench G1); Batch 177 (the EXL3/GB10 serving-recipe distill): 2 NotBenchable suggestion-only complexity rules (the measurement-honesty pair, no new BenchSpecs); Batch 174 (the cubecl/bevy merged-fix distill): 1 MEASURED ParetoDominant benchable rule (`deferred-command-flush-hoist-out-of-batch-loop` concurrency, +1 BenchSpec) + 1 NotBenchable suggestion-only rule (`host-lock-across-reentrant-blocking-call` concurrency — deadlock class); Batch 173 (the serde zero-copy family distill): 2 MEASURED ParetoDominant suggest-only rules (`serde-borrow-deserialize` + `rawvalue-passthrough-defer-parse`, +2 BenchSpecs — the corpus's first dedicated serde family); Batch 172 (the mezura distill): 1 MEASURED ParetoDominant benchable rule (`dual-mask-fused-block-scan` simd — the corpus's first hand-SIMD `std::arch` entry with AVX2+NEON arms, +1 BenchSpec) + 3 NotBenchable suggestion-only rules (mmap-settled-size-gated-read io + steal-batch-not-per-item concurrency + nonblocking-open-for-head-probes io), no new BenchSpecs beyond the lead + 2 riders (byte-find-position-to-memchr anchor-cover, reuse-allocation-clear retention clamp) + 1 drop; Batch 154 (the tokio upstream-surface refresh): 3 NotBenchable suggestion-only rules (write-vectored-capability-flag io + oneshot-response-send-unwrap-panics + actor-bounded-channel-cycle-deadlock concurrency), no new BenchSpecs + 4 annotation folds (all three new rules annotated); Batch 153 (the async-blocking distill): 3 NotBenchable suggestion-only rules (blocking-sleep-in-async, rayon-bridge-oneshot concurrency + tcp-nodelay io), no new BenchSpecs + 5 annotation folds (async-yield-point annotated, previously None); Batch 152 (the dial9 tokio distill): 3 NotBenchable suggestion-only concurrency rules, no new BenchSpecs; Batch 147 (the staged agave distill): 1 measured-ParetoDominant benchable allocation rule + 1 measured-ParetoDominated rule demoted to suggestion-only WITH its negative as the pinned anti-regime witness + 3 NotBenchable suggestion-only rules)
- [`src/domains/rust_perf/proposers/`](../../src/domains/rust_perf/proposers/) — the 10 `syn` AST proposers (Phase 4 top-5 + the 2026-08-17 Batch 48 + Batch 53 follow-ups) + `propose_fix` dispatch
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier` (cargo test)
- [`src/domains/rust_perf/bench.rs`](../../src/domains/rust_perf/bench.rs) — `PerfBenchValidator` (Pareto verdict)
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — scaffold end-to-end demo
- [`examples/perf_bench_demo.rs`](../../examples/perf_bench_demo.rs) — the GOAT gate (Bench 013 promotion + Bench 015 Phase 4 finalization)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
Batch 284 (2026-10-25, Plan 288 / Research 321 — ROW REPAIRED 2026-10-26 by the Batch-288 lane: the landing omitted this ledger row, the same class the Batch-286 lane repaired on the kernel_opt side) — +2 the burn/cubek autotune-family laws (entries_50; burn @ v0.22.1 = 907ab111, cubek @ tip 08659297 + the burn-pin cubek rev 51c43a1a, MIT OR Apache-2.0, quotes re-verified at the pins in plan 288 T0; corpus 271→273): measurement `timed-zero-refuses-round-stop` (the corpus's 14th category `measurement`, the instrument-validity family — a device-timer zero reading is an INSTRUMENT FAILURE, never a fast candidate: a zero clears any limit so the bounded round stops at it and persists an unmeasured winner; the stop waits until a zero cannot be reported as a time; the katgpt-rs Issue-855 loud-zero class at the device-timer layer; the CAUSE-side twin is kernel_opt autotune-sample-read-cold-not-cache-fill) + caching `tune-cache-keyed-by-spelled-variant-names` (persisted tune decisions key on STABLE SPELLED STRINGS, never derived/structural identity — a variant rename must not silently invalidate a recorded winner, and a priority reorder must not silently repoint a stored index; the layout-relevant fact rides a candidate whose name IS its layout, in a group never planned — a string in the checksum, no measurement); BOTH ANNOTATED NotBenchable bench_spec None, fixtures ORIGINAL standalone-compiled+executed. The count-neutral half: the candidate-G rider (derived-from-anchored + candidate-set faces) landed on the kernel_opt popcorn anchor-shape-bucketing host.

Batch 288 (2026-10-26, Plan 293 / Research 327 — the b10826 quartet batch's rust_perf half; corpus 273→274): +1 complexity `recursive-input-walk-explicit-stack-indices-not-copies` (upstream ggml-org/llama.cpp #31153 @ 48c9f08b7 "common, grammar: add recursive depth limit, make it a requirement", MIT, merged 2026-10-25, quotes verbatim at the fetched patch): the AXIS law — a recursion depth limit must count the RECURSING axis (stack frames), never a per-input quantity that travels with the data ('The depth limit in llama_grammar_reject_candidates_for_stack counted the code points of a vocab token, so any token of 256 chars or more was always rejected under a grammar, such as the 512-space token of GLM-4.7-Flash or the long space tokens of Laguna-S-2.1' — a stack-overflow guard converted into a silent input-length rejection, a correctness hole wearing a perf guard's clothing) — and the walk restructure — 'The rejection now walks the code points with its own stack of levels instead of recursing: there is no limit and no stack risk, whatever the length of a token from the GGUF, and the masks match master', the levels carrying 'indices into the input candidates instead of copies' ('At a given code point a candidate only differs from its input by the position of its code point pointer... the rejects map back to the input directly'); MEASURED (source-reported): 'Grammar sampling gets faster than master on GLM-4.7, Laguna, Qwen3.6 and gemma-4 with the json, list and spaces grammars, same masks'; the in-code level struct { stacks, i, cur, rejects } seeded via std::iota (ALL candidates as indices), reject_step threading levels.size()-1 as the CODE-POINT POSITION (the old depth variable became the input position); DISTINCT-FROM grammar-window-draft-prune-path-local-mask (entries_11 — the verify-tree prune BEFORE a forward; this is the rejection walk itself, one layer down); ANNOTATED NotBenchable bench_spec None, fixtures ORIGINAL standalone-compiled+executed 2/2 green. Gates: batch288 rust_perf needle FIRST-RUN; default lib 2734/0; kernel_opt lib 3241/0; clippy −D ×3 postures; retrieval floors HELD both postures no re-pin; score-bench #202 heal 0.9636 FLAT parse 1.0; release-set binary REBUILT + installed (--info serves rust_perf 274). The kernel half (five count-neutral riders) is in kernel_opt.md.

- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (now closed)
- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the original promotion-gate GOAT results (historical; pre-expansion corpus snapshots)
- [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md) — Phase 4 finalization on the expanded 39-rule corpus (29/31 ParetoDominant)

Batch 283, 2026-10-25, Plan 287 / Research 320 — +1 the certifying-identity-token revalidation skip (caching, entries_49, PrismML llama.cpp fork @ `63f16c408f` #393 "cut ~55-65 us of host time per decode step (batched sched input uploads, no node scans on CUDA graph replay; byte-identical)", MIT lineage, read in full at the vendor clone AT the pin): `revalidation-skipped-by-certifying-identity-token` — a per-step re-validation over an UNCHANGED input skips when a CERTIFYING IDENTITY TOKEN exists and matches, where the token is (1) written ONLY by the validation's own success path, (2) ABSENCE-GUARDED (`uid != 0` — a never-stamped input against a fresh slot reads 0==0 and would skip validating an input NO scan ever passed, the trap the buggy fixture demonstrates), (3) minted by the BUILDER (monotonic per re-derivation, so token equality implies the same instance), (4) backed by the full scan as the sound fallback; SECONDARY content: the derived consumer map rebuilds only at the update boundary ("only read while the nodes are evaluated or captured: not built when a captured CUDA graph is replayed"); HONESTY in-rule: the 55-65 µs headline covers the WHOLE commit (the copy-batching half is the kernel_opt-side rider face, not this rule) and the token site predates the commit (this commit wired the token into the ADMISSION). OUR-BOX: riir-ai Bench 1000 (4090 sm_89 vs the scored tag carrying this commit) measured the opponent +2.11% decode naming this host-cut. ANNOTATED NotBenchable bench_spec None, fixtures ORIGINAL standalone-compiled+executed 4/4 green. Gates: batch283 rust_perf gate, default lib 2731/0, count pins (271 total / 150 without_spec / 218 annotated) via shape-asserted passes, release-set binary rebuilt + installed (--info serves rust_perf 271).

Batch 332 (2026-11-05, Plan 340 / Research 370 — the blazing-smtlib distill conversion, entries_65, corpus 298→300, dispatch +2): source onderjan/blazing-smtlib @ `42b45c20b0045495a40b9eaf0a717e0f1c976bed` (MIT OR Apache-2.0, v0.1.0, HEAD 2026-10-29; paper provenance arXiv 2611.00185 "Blazing-Fast SMT-LIB Parsing for Rust and Lean" — a NEW library, no upstream port; the >6× geomean vs previous parsing libraries is source-reported, provenance never bars), quotes re-verified at the pin at plan-340 authoring. `byte-flag-table-masked-run-scan` (the batch's GOAT-eligible lead, BENCHABLE): a const-eval 256-entry u8 class-flag table (8 run-flag bits, one per class, the compile-time coverage assert "run flags must cover every bit of the byte") + a masked-prefix run scan — "Position of the first byte carrying no flags from the mask, slice length if none. In other words, the longest prefix where all bytes carry at least one flag from the mask." — a 4-byte-unrolled SCALAR loop with the in-source perf note "Measured as superior to iter().position(), chunks, and SWAR."; the folded half: the packed FAST dispatch byte (FAST_DELIM/ON/EMIT/PIPE + the 3-bit run mask in ONE u8, "Fast dispatch bitmasks for every input byte.", const-eval with coverage + agreement asserts) so the hot loop "is determined by one table lookup"; the hot-loop discipline verbatim: "This loop is VERY HOT and even small changes can drastically affect performance. This especially applies to unintentional panic paths introduced by maybe-panicking bounds checks." MEASURED ParetoDominant 3/0/0 (2.62×/2.94×/2.10× at 1K/16K/262K, allocs 0/0, standalone direction check 2.87×). The TWO-REGIME cross-cite (never merge): B243's `ascii-run-swar-block-skip` owns LONG pure-ASCII blocks (one class axis, SIMD gather); THIS owns SHORT mixed-class runs (many class axes, scalar masked scan, mask picked per token kind) — the source's own measurement is the bridge (SWAR loses at lexer run lengths). LANE-INTEL: riir-infer `src/tokenizer.rs:1514-1595` GPT-2-style split rules walk per-char Unicode predicate chains on the laya encode path — the rule's masked-run shape maps directly; the sibling issue is filed by the owning lane (Plan 340 T5). `token-batch-cursor-handover` (suggestion-only, NotBenchable-by-consumer-cost): "Maximal number of tokens that can be batched." — BATCH_CAPACITY 256, a fixed array of zero-copy `(category, &slice)` views, the PULL cursor handover with the consumer's action (None/Relex/Discard) applying at the BATCH boundary ("the stop takes effect after the consumed tokens"; "the batch is spent once handed over, no matter what the consumer decides") — distinct from all four batching families (bufwriter/channel-unpark/deferred-command-flush/blocking-pool-segment) + `fanout-transform-emit-callback-not-vec` as the opposite pole (PUSH-callback emit, allocation axis). The source's interleaved compare bench (benches/compare.rs) is an independent implementation of the workspace's sequential-AB law (katgpt-rs `scripts/sequential_ab_timing_audit.py`) — recorded in Research 370 as external CORROBORATION, never a rule. Fixtures ORIGINAL plain-Rust, standalone rustc-compiled AND executed pre-landing (rule 1: per-byte census 0..=255 + block edges + generated-corpus equivalence; rule 2: identical token streams + the ceil(tokens/256) dispatch census). Yield 2/2 (above the ~50% rate). Gates: batch332 needles+parse 1/1; default lib 2838/0; rust_perf_bench 2869/0 (8i); score_bench suite 3221/0 (11i); clippy −D ×4 postures 0; retrieval floors: top-1/top-3 UNCHANGED (floors 123/129 held), top-5 133→132 of 142 — ONE span (`repr-u8-enum`) demoted by legitimate corpus growth (the P7/B200 class), floor re-derived measured − ONE span → 131/142 (the Issue-063 law); retrieval suite 27/0; score-bench run #227 heal 0.9636 FLAT parse 1.0; freeze NOT owed (zero clippy_lints delta, P30); pins manifest --write (300/127/173) + 3 prose sites + the history-row disambiguation; release-set binary REBUILT + installed at the charitable set (`latent_retrieval,points,net-mainnet,fix_verify,update_check`); `.raw` scratch removed at the landing. Batch 332 / Plan 340 / Research 370.
