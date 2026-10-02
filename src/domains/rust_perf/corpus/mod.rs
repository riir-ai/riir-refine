//! Seed corpus — rust performance rules distilled from the `rust-optimize`
//! skill checklist.
//!
//! Each entry is a `(rule_id, name, category, buggy_code, fixed_code, context_notes)`
//! tuple representing a real perf pattern from the skill. The corpus is
//! BLAKE3-committed via [`Corpus::new`](`crate::corpus::Corpus::new`). The
//! `context_notes` field is `Option<String>` — `Some(...)` for rules the bench
//! validator found ParetoDominated/Mixed/Tie in the default config (Bench 013 /
//! 038) or that are NotBenchable-with-a-metric-mismatch (Issue 014), `None` for
//!      unconditional wins + honestly `NotBenchable` rules.
//!
//! 163 entries across 12 categories (allocation 28, io 26, concurrency 23,
//! complexity 28, layout 13, iterators 11, simd 9, arithmetic 8, caching 8,
//! dispatch 6, bevy 2, wasm 1). 99 are benchable; 64 are honestly//! `NotBenchable` — the count repaired by Batch 135 (the pre-Batch-118 ledger
//! said 25 while the gated test said 14; the Batch 118–129 ds4 io
//! annotations were never counted) and again by the 09-24 housekeeping pass
//! (the header prose had gone stale at 145/92/53 through B172–B174; the
//! gated count tests + `cargo refine --corpus-stats` are the live ledgers)
//! (8 external-runtime/size-class + 2
//! metric-mismatch (Issue
//! 014) + the x86-only prefetch annotation (Batch 85) + the bitflag size
//!      axis (Batch 86) + the workload-level read-coalescing annotation (Batch
//! 94) + the measurement-honesty axis itself (Batch 109 — a timing gate
//!     whose work folds away has no nonzero baseline to ratio) + the PLE-gather
//!     workload annotation (Batch 118) + the two build-integrity annotations
//!     and the A/B-design annotation (Batch 121 — resume provenance, write-
//!     verify-publish, one-variable A/B; correctness-of-provenance and
//!     experimental-design content, not speed) + the three vision-delta
//!     annotations (Batch 122 — template-governed emission, provisioning
//!     completeness, per-turn diagnostics; correctness-of-emission and
//!     test-honesty content, not speed) + the two upstream-surge annotations
//!     (Batch 123 — full-vector comparison integrity, tri-state fit
//!     classification; measurement-integrity and robustness content, not speed)
//!     plus the evaluation-design annotation (Batch 124 — matched controls,
//!     proxy-vs-endpoint, null-arm identity; experimental-design content, not
//!     speed) plus the oracle-contract annotation (Batch 125 — baseline-
//!     reproduction triage, compute-shape-matched oracles, consumed-outputs
//!     comparison; oracle-contract content, not speed) plus the three agent-
//!     context annotations (Batch 141 — measured reserve for a destructive
//!     rewrite, semantic truncation boundaries, pattern-bounded stream holdback;
//!     budget/boundary/stream discipline, not speed) plus the two Batch-143
//!     1x) plus the two Batch-143
//!     annotations (the log-only hot-path computation behind a level gate — its
//!     note carries the #143684 wall-time-vs-icount measurement-honesty rider;
//!     and the macro-emission build-time rule — upstream-measured compile-time
//!     content, no runtime A/B axis). Batch 173 (Plan 167) added the two serde
//!     zero-copy rules — allocation 26 → 28 (the corpus's first dedicated
//!     serde family; both benchable + annotated). Batch 172 (Plan 166) added
//!     the mezura quartet — simd 8 → 9, io 24 → 26, concurrency 20 → 21 (one
//!     measured GOAT lead + three `NotBenchable` decision-content rules). Batch
//!     174 (Plan 168) added the host-lock + flush-hoist pair — concurrency
//!     21 → 23 (one measured `ParetoDominant` + one `NotBenchable` deadlock
//!     class). Batch 179 (Plan 172 / Research 210) added the rsconstmap trio —
//!     layout 12 → 13, iterators 10 → 11, complexity 22 → 23 (three measured
//!     `ParetoDominant` benchable rules; the first `[dep: constmap]` external
//!     map-family entry).
//!
//! Batch 48 (idle-queue item 11, 2026-08-16) added the 8 online-search
//! candidates (perf-book std-types + iterators chapters): collect-append-to-
//! extend, hashset-insert-bool-return, unwrap-or-else-lazy, filter-count-to-
//! any, vec-macro-zero-fill, iter-copied-deref-map, filter-map-fuse,
//! vec-insert-front-deque. Batch 53 (idle-queue item 11, 2026-08-17) added
//! the 4 online-search candidates: powi-vs-powf, distance-sq-no-sqrt,
//! hashmap-with-capacity, soa-hot-field. Batch 59 (2026-08-18, the first
//! EXTERNAL-source distill via the `distill` skill) added
//! division-by-invariant-multiplication (libdivide via the
//! raygon-renderer/thermite Rust port). Batch 61 (idle-queue item 11,
//! 2026-08-18) added the 2 oxc-article rules: keyword-map-to-match +
//! usize-index-to-u32. Batch 62 (2026-08-19, Research 070 microGPT-C
//! distill) added the 2 CPU-SIMD rules: transposed-weight-register-gemv +
//! newton-refinement-vs-hardware-transcendental. Batch 64 (2026-08-19,
//! Research 072 — the ARM Neon / bounds-check online-search pool) added
//! loop-order-innermost-contiguous + debug-assert-length-hoist. Batch 66
//! (2026-08-19, Research 073 — the polars kernel-idiom distill) added
//! sparse-set-bit-iteration-tz-blsr + sortedness-check-branchless-accumulate.
//! Batch 67 (2026-08-19, Research 074 — the regex-automata distill) added
//! byte-find-position-to-memchr. Batch 68 (2026-08-19, Research 075 — the
//! tantivy utility-layer distill) added varint-compose-word-single-store
//! (+ the tz-blsr xor-clear anti-shape rider note). Batch 69 (2026-08-19,
//! Research 075 §Secondary — the T6 refutation) added
//! unaligned-word-load-from-bytes. Batch 65 (2026-08-22, Research 081 —
//! the hotpath.rs profiler distill) added the 2 contention rules:
//! align-64-false-sharing + channel-send-batching-unpark (both measured
//! `ParetoDominant`, both context-annotated — the 128B-line arch caveat +
//! the park-storm regime condition).
//!
//! Batch 83 (2026-08-28, Research 097 — the arXiv:2608.26368 SIMD
//! bit-slicing predicate distill) added bitplane-transpose-repeated-
//! predicate-scan: encode-once bit-plane transposition + zero-decompression
//! bitwise predicate evaluation for REPEATEDLY filtered integer columns
//! (the O'Neil-Quass bit-sliced index as a primary representation).
//!
//! Batch 85 (2026-08-28, Research 098 — the arXiv:2608.23841 cflow
//! "Pipeline-Native Transformers" distill) added weight-tile-consumption-
//! order (MEASURED `ParetoDominant` 1.32-1.56x growing with working set —
//! consumption-ordered tiles keep the x slice L1-resident per tile) +
//! software-prefetch-storage-bound (the honest annotation: SW prefetch on
//! a bandwidth-bound stream is useless-to-harmful — the source's prefetch
//! thread burned 48% of decode time; disabling it alone = 1.92x; the
//! aarch64 PRFM analog unmeasured — x86 source-cited only, proposer-less
//! by design).
//!
//! Batch 86 (2026-08-29, Research 100 — the Cloudflare 1.1.1.1 DNS-cache
//! memory distill) added serialize-once-byte-buffer-store (frozen records
//! as ONE length-prefixed byte buffer + offset table — 2 allocations
//! total regardless of record count; the insert-path malloc storm is the
//! cost), context-equal-field-elide (a stored field that equals the
//! enclosing context — the lookup key — becomes Option<Box<T>> None,
//! resolved at read; ~80% of the source's records elided), and
//! bitflag-pack-when-footprint-dominates (the stored-at-scale footprint
//! regime; cross-cites the `kernel_opt` hot-read inverse; measure `size_of`
//! before/after). Two doc riders: box-str-immutable generalizes to
//! Box<[T]> for any frozen T; box-large-enum-variant gains the honest
//! costs (size-class rounding, locality scatter, the rare variant pays
//! more).
//!
//! Batch 91 (2026-08-31, Research 106 — the Rustdoc filter-first distill,
//! noahlev.org "Making Rustdoc 33% faster" + rust-lang PRs 159623/159779/
//! 159854) added filter-before-expensive-transform (Rustdoc built the
//!         display representation for EVERY trait impl in the dependency graph,
//!         THEN filtered; the fix re-expresses the relevance predicate on the raw
//!         input and filters first — win ∝ discard-fraction × transform cost,
//!         IMPOSSIBLE when the predicate is not re-expressible on the input) and
//!         decision-projection-before-construction (the decision consumes only a
//!         cheap projection that is INVARIANT under an expensive canonicalization
//!         — the head of a type — so compute the projection on the raw input and
//!         pay the canonicalization only behind the decision).
//!
//! Batch 94 (2026-09-01, Research 110 — the oMLX PR #3260 SSD expert
//! streaming distill) added coalesce-nearby-reads-bounded-gap-cacheability-
//! gated to io: merge random-ish reads within a bounded byte gap into ONE
//! pread; the CACHEABILITY COUPLING is the decision content — the over-read
//! bytes are only free when the OS page cache absorbs them, so a direct/
//! no-cache descriptor coalesces with gap 0 (exactly-adjacent rows only).
//! The source's serving numbers are workload-level, no isolated corpus
//! bench — regime-annotated, suggestion-only.
//!
//! Batch 118 (2026-09-06, Research 142 / Plan 097 — the antirez/ds4 PR #991
//! PLE-gather distill) added parallel-disjoint-pread-hides-cold-page-latency
//! to io: N known DISJOINT byte ranges -> thread-parallel positional reads on
//! the shared fd + indexed writes (byte-identical by construction); the
//! LATENCY-OVERLAP axis is the decision content — cold page-ins paid serially
//! at queue depth one (~20x measured) vs warm-neutral; near-adjacent ranges
//! are the OPPOSITE regime (coalesce-nearby-reads). Suggestion-only.
//!
//! Batch 109 (2026-09-05, post-mining intake P10 — the katgpt-rs Issue 723
//! Class A2 measurement-honesty distill) added dead-result-timing-loop-
//! folded-under-lto to complexity: a timed loop whose work result is
//! discarded (`let _ = f()`) measures ZERO under rustc 1.98.1 + fat LTO —
//! the fold eliminates inlined-callee work EVEN THROUGH `black_box` inside
//! the callee (only a CALLER-side sink of per-iteration data survives);
//! the fix pattern is xorshift data-dependent work + `black_box(sink)` at
//! the consumer + a loud nonzero-baseline assert before any ratio. The
//! first rule on the measurement-honesty axis: the "buggy" arm is a
//! harness that passes its own timing gate while timing nothing.
//!
//! ## Bench specs (behind `rust_perf_bench`)
//!
//! Benchable rules carry an optional [`BenchSpec`] — a self-contained harness
//! embedding both buggy + fixed code inline + defining `make_input` /
//! `run_buggy` / `run_fixed` entry points. The bench validator generates a
//! temp bin crate from the spec + runs a timing loop at multiple sizes.
//!
//! Currently benchable (90/121):
//! - `o1-lookup` (Vec scan vs `HashMap` lookup — clear O(n) vs O(1))
//! - `bsearch-sorted-slice` (linear scan vs binary search — O(n) vs O(log n) on sorted data)
//! - `dedupe-hashset-vs-vec` (`Vec::contains` loop vs `HashSet` collect — O(n²) vs O(n))
//! - `entry-vs-contains-insert` (`contains_key+insert` vs entry API — merge `HashMap` lookups)
//! - `loop-fusion` (two iterator passes vs one fused loop — cache locality, merge loops)
//! - `prefix-sum-2d` (O(n) region scan vs O(1) prefix sum — boundary-vs-volume, grows with N)
//! - `swap-remove` (`Vec::remove` O(n) shift vs `Vec::swap_remove` O(1) swap — grows with N)
//! - `sort-unstable` (stable sort allocates vs `sort_unstable` in-place — no allocation + faster algo)
//! - `sort-by-key` (`sort_by` repeated closure vs `sort_by_key` single extraction — N vs N·log N)
//! - `vec-remove-front-to-vecdeque` (`Vec::remove(0`) O(n²) drain vs `VecDeque::pop_front` O(n) — grows with N²)
//! - `btreemap-sorted-iteration` (`HashMap` + sort per query vs `BTreeMap` natural order — O(n log n) vs O(n) per query)
//! - `static-str` (String alloc vs &'static str — allocation cost)
//! - `serialize-once-byte-buffer-store` (per-record store malloc storm → ONE length-prefixed buffer + offsets — insert path, Batch 86)
//! - `context-equal-field-elide` (unconditional owner field → Option elided at ~80% context-equal — alloc axis, Batch 86)
//! - `simd-chunk` (scalar vs chunked loop — vectorization at large N)
//! - `branchless-conditional` (if vs bool-as-usize multiply — vectorization + branch prediction)
//! - `bool-to-bitpack` (Vec<bool> count vs packed Vec<u64> POPCNT — data density)
//! - `chunks-exact` (chunks variable-last vs `chunks_exact` uniform — enables vectorization)
//! - `copy-from-slice` (index-loop copy vs `copy_from_slice` — SIMD + no bounds check)
//! - `mutex-to-atomic` (Mutex<u32> vs `AtomicU32` — lock/unlock overhead)
//! - `f32-counter-to-int` (f32 accumulator vs u64 — serial dep chain)
//! - `collect-avoidance` (.`collect()` intermediate vs chained iterator — allocation + double pass)
//! - `dyn-to-enum` (Box<dyn Trait> vs enum — vtable indirection + no inlining)
//! - `ok-or-else` (`ok_or(expensive()`) eager vs `ok_or_else(closure`) lazy — avoids unused error alloc)
//! - `precompute-table` (recompute vs const lookup table — **context-dependent**,
//!   see `context_notes`; not a release-mode win per Bench 013)
//! - `papaya-lockfree` (`RwLock` vs papaya lock-free — **context-dependent**,
//!   see `context_notes`; only a win under multi-threaded contention)
//! - `vec-with-capacity` (`Vec::new()` growth vs `Vec::with_capacity` — realloc cost)
//! - `format-concat-loop` (format! concat in loop vs `push_str` — O(n²) vs O(n))
//! - `fxhashmap` (std `HashMap` `SipHash` vs `FxHashMap` `FxHash` — hasher speed, 2-4×)
//! - `retain-inplace` (`filter().collect()` new Vec vs `retain()` in-place — ~1.5-2×)
//! - `mem-take` (clone+clear vs `mem::take` move — ~3×, avoids deep clone)
//! - `avoid-clone` (fn(data: Vec<T>) clone → fn(data: &[T]) borrow — ~1.3-3×)
//! - `box-str-immutable` (Vec<String> 3-word → Vec<Box<str>> 2-word — ~1.1-1.3×; B86 rider: generalizes to Box<[T]> for ANY frozen T)
//! - `reuse-allocation-clear` (alloc-per-call → clear-reuse scratch buffer — **Mixed**, see `context_notes`)
//! - `box-avoid` (Vec<Box<i64>> heap per elem → Vec<i64> stack values — ~10-35×)
//! - `smallvec-stack` (Vec<T> always heap → stack array for ≤8 typical — ~7-12×)
//! - `buffered-io` (`File::read` byte-by-byte → `BufReader` 8KB batched — ~97×)
//! - `bufwriter-batched` (`File::write` per-record → `BufWriter` batched — syscall amortization)
//! - `log-write-direct` (`push_str(&format`!(...)) temp alloc → write! direct — ~1.7×)
//! - `cow-conditional-alloc` (String always-alloc → Cow<str> borrow when no transform — N vs 0.1N allocations)
//! - `regex-hoist-lazylock` (`Regex::new` per call → static `LazyLock` — compile once, Issue 014 T2.1)
//! - `arc-str-payload` (per-recipient payload clone → Arc<str> handle clone — N×bytes → 1 alloc, Issue 014 T2.2)
//! - `collect-append-to-extend` (collect+append staging → extend — no intermediate Vec, Batch 48)
//! - `hashset-insert-bool-return` (contains+insert double lookup → insert's bool return — Batch 48)
//! - `unwrap-or-else-lazy` (`unwrap_or(eager`) → `unwrap_or_else(closure`) — Batch 48)
//! - `filter-count-to-any` (`filter(p).count()` > 0 full scan → any(p) short-circuit — Batch 48)
//! - `vec-macro-zero-fill` (`Vec::new+resize` memset → vec![0; n] calloc — Batch 48)
//! - `iter-copied-deref-map` (.map(|x| *x) → .`copied()` `TrustedLen` — Batch 48)
//! - `filter-map-fuse` (filter(p).map(f) → `filter_map` fused — Batch 48)
//! - `vec-insert-front-deque` (`Vec::insert(0`,_) O(n²) → `VecDeque::push_front` — Batch 48)
//! - `powi-vs-powf` (x.powf(2.0) libm call → x.powi(2) multiply — Batch 53)
//! - `distance-sq-no-sqrt` (sqrt-then-compare → squared compare — Batch 53)
//! - `hashmap-with-capacity` (`HashMap::new()` rehash ladder → `with_capacity(n`) — Batch 53)
//! - `soa-hot-field` (`AoS` strided single-field scan → `SoA` contiguous column — Batch 53)
//! - `division-by-invariant-multiplication` (hardware integer div by loop-invariant RUNTIME d → libdivide-style magic mulhi+shift — Batch 59, thermite distill)
//! - `keyword-map-to-match` (static-keyword `HashMap` dispatch → plain match on literals — LLVM length-switch, Batch 61)
//! - `usize-index-to-u32` (16B usize spans → 8B u32 spans — cache density under bandwidth-bound scans, Batch 61)
//! - `transposed-weight-register-gemv` (row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA into output accumulators — 1.18-6.08x `ParetoDominant`, Batch 62, microGPT-C distill; the win GROWS at large nin — the scalar accumulator's serial FP chain is the real cost, not the horizontal reduce)
//! - `newton-refinement-vs-hardware-transcendental` (Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction `fdiv`/`fsqrt` on CPU — Mixed 1.01-1.09x M3 / `ParetoDominant` 1.64-1.66x x86 (re-gated), hardware never loses; Batch 62, annotated)
//! - `loop-order-innermost-contiguous` (i-j-k nested accumulation with a column-striding inner access → i-k-j reorder so the innermost varies the LAST index — 7.1-13.8x `ParetoDominant`, grows with n; Batch 64)
//! - `debug-assert-length-hoist` (`debug_assert!` mentioning `.len()` is erased in release, dropping the constraint bounds-check elision needed → `assert!` — Mixed regime-split 1.13x@1K / 0.67-0.71x@100K stable on aarch64 (x86 re-gate: 1.00x/0.92x, holds cross-arch); Batch 64, annotated)
//! - `sparse-set-bit-iteration-tz-blsr` (64-iteration set-bit scan → `trailing_zeros+BLSR`, popcount iterations — sparse-mask regime, Batch 66)
//! - `sortedness-check-branchless-accumulate` (early-exit sortedness validation → branchless `sorted &=` full scan, vectorizes — usually-sorted regime, Batch 66)
//! - `byte-find-position-to-memchr` (u8 slice iter().position byte scan → `memchr::memchr` SIMD block scan — position is NOT specialized in std, contains is; dep-gated, Batch 67)
//! - `varint-compose-word-single-store` (per-byte `write_all` varint loop → 5-threshold ladder + single `write_all` — MEASURED `ParetoDominated` on aarch64 AND x86 across writer regimes (the 4090-box re-gate: 0.40-0.50x there, HARDER): the honest-loser annotation stops the tantivy folk port; Batch 68)
//! - `unaligned-word-load-from-bytes` (shift/or byte-assembly word read → one unaligned `from_le_bytes` load — `ParetoDominant` cross-arch: u64 1.4-2.2x, u32 1.2-1.4x, both arches; the T6 defer refuted; Batch 69)
//! - `align-64-false-sharing` (adjacent 24 B per-thread atomic counter slots → `#[repr(align)]` to ARCH line granularity — `ParetoDominant` 4.7-9.4x at 2-12t, **1t = tie (honest)**; align(64) insufficient on 128B-line Apple Silicon (measured 1.16-2.44x of the win) — pad to the arch granularity or `CachePadded`; Batch 65)
//! - `channel-send-batching-unpark` (per-item MPSC sends to a parked `select!` receiver → per-64 batch flush via `mem::replace` — `ParetoDominant` 1.16-1.29x (pipeline-overlap honest margin; the source's 6.25x needs the park pair ON the critical path) + allocs 3232→1624; Batch 65)
//! - `swar-digit-chunk-parse` (per-digit decimal parse loop → SWAR 8-digit chunk fold, 3 multiplies per 8 digits + SWAR digit validation — the serial `val = val*10 + d` chain broken; regime: ≥8 digits win, <8 pay the guard (the source's own README); Batch 74, simd-json distill)
//! - `runtime-feature-detect-per-chunk` (CPU feature detection inside a per-chunk hot fn → resolve once via cached static / static cfg gate — the `std::detect` string resolution + detector read per call rivals small work units; Batch 74, simd-json distill)
//! - `runtime-int-exponent-powi` (runtime-integer-exponent `powf` libm call → `powi` binary exponentiation — log2(n) multiplies vs an opaque pow call; NOT bit-identical, rel-error-gated; Batch 77, lucebox rope distill)
//! - `bitplane-transpose-repeated-predicate-scan` (repeatedly filtered integer column → bit-plane transposition + bitwise predicate evaluation over the compressed planes, zero decompression — b/32 the bytes scanned + 64 values per word op; Batch 83, arXiv:2608.26368 distill)
//! - `weight-tile-consumption-order` (row-major single-token matvec x re-walked per row → consumption-ordered tiles, x slice L1-resident per tile — 1.32-1.56x growing with working set; Batch 85, cflow distill)
//! - `serialize-once-byte-buffer-store` (per-record heap fields → ONE length-prefixed byte buffer + offset table — 2 allocations total, copy-out rebuild; Batch 86, Cloudflare DNS-cache distill)
//! - `context-equal-field-elide` (per-record field equal to the enclosing context → Option<Box<T>> None, resolved at read — Batch 86, Cloudflare DNS-cache distill)
//! - `bitflag-pack-when-footprint-dominates` (bool cluster → one bitflag byte, stored-at-scale footprint regime — Batch 86, annotated)
//! - `filter-before-expensive-transform` (map(expensive).filter(relevant) → filter-first on the RAW input — win ∝ discard × transform cost; Batch 91, Rustdoc filter-first distill)
//! - `decision-projection-before-construction` (transform-to-decide → decide on the invariant raw projection, canonicalize only behind the decision — Batch 91, annotated)
//! - `mir-destprop-branch-site-merge` (known-fn-item conditional dispatch → two direct tail-jumps — MIR `DestinationPropagation` cmov-merges the merged form into one indirect site mixing branch-target history; Batch 99, wasmi distill)
//! - `dispatch-handler-pair-hoist` (runtime-table handler read per dispatch → hoist both entries once — register-resident select; cross-arch `ParetoDominant`, x86 re-gated 2026-09-04; Batch 101)
//! - `hot-byval-move-over-memcpy-threshold` (hot by-value move of a >128-byte type → shrink/ref — the threshold is an arch-shaped LLVM backend constant; Mixed on M3, x86 pays the memcpy call; Batch 112)
//! - `atomic-vec-zero-cost-convert` (`Atomic::new-per-element` collect from a borrowed vec → `into_iter().map` on the OWNED vec — same-layout collect reuses the buffer and the loop compiles out; alloc axis 2→1; Batch 135, Wild-linker article distill)
//! - `vec-reuse-across-lifetime-change` (fresh Vec<&'iter T> per iteration → owned buffer retargeted in place via a const size/align assert + `clear()` + empty collect — alloc axis 1→0; Batch 135, Wild-linker article distill)
//! - `partitioned-mut-slices-zero-sync` (shared Vec under one contended Mutex → disjoint `chunks_mut` partition, zero synchronization — sizes = thread counts; Batch 135, Wild-linker article distill)
//! - `dynamic-work-parallelism-ladder` (work-unknown-in-advance structure ladder: scoped spawning is the composition-safe default despite the boxing — `spawn_broadcast` starves the pool, `channel+par_bridge` does not compose; Batch 136, rayon-graph blog distill)
//! - `sender-per-work-item-terminates-channel` (dynamic queue shutdown is ownership-driven: seed sender clones per item, drop the original, the channel closes when the last item drops; Batch 136, rayon-graph blog distill)
//! - `nested-par-inside-channel-bridge-deadlocks` (a nested `par_iter` in par_bridge-fed work steals, blocks on the SAME channel, and the interrupted item's sender clone keeps it open — thread-tied tasks hang; Batch 136, rayon-graph blog distill)
//! - `compaction-reserve-before-prompt-build` (a destructive rewrite's reserve is MEASURED from the real prompt — tokenize it, add slack + output budget — and every headroom consumer reads that one figure; Batch 141, ds4 Part-B distill)
//! - `compaction-tail-boundary-prefers-complete-turn` (truncation boundaries are semantic: bounded walk-back to the latest complete user turn, atomic spans snap the boundary to their start; Batch 141, ds4 Part-B distill)
//! - `streaming-prefix-holdback-bounded-buffer` (a stream recognizer holds back at most the marker's own length, gated to the line boundary, flushing verbatim on mismatch — no line-length pending window; Batch 141, ds4 Part-B distill)
//! - `debug-level-gated-computation` (a hot-path binding whose only consumer is a `debug!`/`trace!` call pays its computation every iteration — the level check is a runtime atomic, not a compile-time fact; gate the computation with `log_enabled!` or move it into the macro's lazy args; Batch 143, nnethercote Dec-2025 distill)
//! - `macro-generated-code-optimize-at-generation-point` (proc-macro emitters trim/dedupe generated code where the context lives — downstream optimization machinery must reconstruct what the emitter already knew; measure with `-Zmacro-stats`; Batch 143, annotated)
//! - `prefault-admission-keyed-to-ram-headroom` (admission-policy annotation,
//!   source-reported — the DECISION CONTENT is the headroom predicate + the
//!   fail-closed direction + the mincore instrument refusal; page-cache state
//!   dominates any microbench; Batch 139, ds4-metal PR #5 PLE prefault)
//!
//! Not benchable (53/145 — count walked by Batch 154 (+3 tokio upstream-
//! surface rules); the line was repaired en passant by Batch 153 after it
//! below had sat stale at 36/127 since Batch 145 while B146/B147/B152/B153
//! grew both sides; the Batch-149 en-passant-pin-repair precedent) — honest
//! (count repaired by Batch 135; the Batch 136 rayon-queue + Batch 139 prefault-admission + Batch 141 ds4-Part-B agent-context + Batch 143 nnethercote build-time + Batch 145 mincore-routing rules are the post-repair additions; the ledger below is the early-corpus list — the Batch 118–129 source-reported io annotations are in the header ledger, all `bench_spec: None`):
//! - `layout-reorder` (struct size — deterministic `size_of`, not a runtime
//!   microbench; the cache-utilization claim needs a specific access pattern)
//! - `repr-u8-enum` (enum size — deterministic `size_of`, same class as
//!   `layout-reorder`)
//! - `box-large-enum-variant` (enum size — deterministic `size_of`, shrinks
//!   the common case by boxing the rare large variant)
//! - `shrink-to-fit-after-build` (memory win, not speed — releases excess
//!   Vec capacity after incremental construction)
//! - `async-yield-point` (fairness/latency-tail win, not throughput)
//! - `async-future-size` (future state-machine size — deterministic `size_of_val`)
//! - `bevy-reactive-loop` (needs the Bevy runtime — can't run in a temp crate)
//! - `wasm-batch-ffi` (needs a WASM runtime — can't run in a temp crate)
//! - `tokio-guard-across-await` (**metric mismatch**, Issue 014 T2.3 — cost is
//!   OTHER tasks' wait latency under contention, not this task's wall-clock)
//! - `bounded-channel-backpressure` (**metric mismatch + semantics-changing**,
//!   Issue 014 T2.4 — win is bounded queue depth; the buggy path is often
//!   FASTER on wall-clock precisely because it buffers unboundedly)
//! - `software-prefetch-storage-bound` (x86-only source evidence — no stable
//!   aarch64 prefetch intrinsic to re-gate with; the rule IS the annotation
//!   stopping a plausible-looking prefetch port; Batch 85, cflow distill)
//! - `bitflag-pack-when-footprint-dominates` (deterministic `size_of` delta,
//!   the `layout-reorder`/`box-large-enum-variant` class — the gate pins
//!   12 → 8 for the fixture layout; Batch 86, annotated)
//! - `coalesce-nearby-reads-bounded-gap-cacheability-gated` (workload-level
//!   read-coalescing annotation, source-reported — the DECISION CONTENT is the
//!   cacheability coupling: the gap budget collapses to 0 on `direct/F_NOCACHE`
//!   descriptors; Batch 94, oMLX safetensors reader)
//! - `parallel-disjoint-pread-hides-cold-page-latency` (workload-level
//!   latency-overlap annotation, source-reported — the DECISION CONTENT is the
//!   disjoint-vs-near regime split vs coalesce-nearby-reads: DISJOINT far
//!   ranges overlap cold page-ins across threads, warm is neutral; Batch 118,
//!   ds4 PLE gather)
//! - `resume-revalidates-provenance-before-reuse` (build-integrity annotation
//!   — a resumable multi-part build revalidates source + calibration + TOOL
//!   hashes against the manifest and re-digests each cached part before
//!   reuse; presence-is-not-identity; Batch 121, ds4 IQ2 tool)
//! - `write-verify-readback-before-publish` (build-integrity annotation —
//!   hidden-path write, independent-reader hash verify per part, atomic-rename
//!   publish + sidecar manifest, refuse-existing, template guards before
//!   byte one; Batch 121, ds4 IQ2 tool)
//! - `ab-isolate-one-component-by-baseline-byte-copy` (measurement-honesty —
//!   A/B on one component byte-copies the baseline for everything else,
//!   paired diffs, confounded variants out of the headline, un-instrumented
//!   memory counters labelled as partial budgets; Batch 121, ds4 IQ2 quality)
//! - `model-aware-format-over-hand-assembled-tokens` (emission-correctness
//!   annotation — structured transcript turns go through ONE model-aware
//!   formatter; hand-pushed token sequences drift from the chat template
//!   silently; Batch 122, ds4 vision delta)
//! - `implemented-surface-requires-provisioning-target` (provisioning
//!   completeness — a feature needing an external artifact ships its fetch
//!   target in the same change; load-time errors NAME the target; Batch 122,
//!   ds4 vision delta)
//! - `diagnostics-not-just-exit-code` (measurement-honesty — a multi-turn
//!   session regression scans captured stderr for per-turn failure patterns
//!   alongside the exit code and saves full transcripts; Batch 122, ds4
//!   vision delta)
//! - `full-vector-bit-identity-over-argmax` (measurement-integrity — a
//!   numeric-regression gate compares the FULL float vector bit-exactly with
//!   pre-validated artifacts and three statuses, never argmax-only; Batch
//!   123, ds4 upstream surge)
//! - `renderer-error-is-not-capacity-overflow` (robustness classification —
//!   a tri-state fit validator keeps render errors off the destructive
//!   compaction path; Batch 123, ds4 upstream surge)
//! - `matched-control-disentangles-policy-from-model` (evaluation-design —
//!   matched controls separate the admission gate from the learned model;
//!   proxy and endpoint metrics reported separately; gate-closed arm
//!   bit-identical to the no-action baseline; Batch 124, arXiv:2609.04040)
//! - `dead-result-timing-loop-folded-under-lto` (measurement-honesty axis —
//!   a timing gate whose work folds away under LTO has no nonzero baseline to
//!   ratio; the rule's content IS that fact; Batch 109, Plan 080)
//! - `mmap-settled-size-gated-read` (page-cache state dominates any map-vs-read
//!   microbench — the io family's B118/B123/B139/B145 standing, cited three
//!   times; decision content = the TWO-GATE admission (size ≥ 256 KiB AND mtime
//!   age ≥ 5 s vs the concurrent-truncation SIGBUS) + the expected+1 sentinel
//!   read protocol; Batch 172, mezura)
//! - `steal-batch-not-per-item` (deque-header contention under real scheduling
//!   — not hostable without a thread farm whose noise floor swallows the
//!   signal; decision content = `steal_batch_and_pop`'s half-not-all policy;
//!   Batch 172, mezura)
//! - `nonblocking-open-for-head-probes` (the failure is a HANG against a live
//!   kernel-notification endpoint — `NotBenchable` by construction; decision
//!   content = `O_NONBLOCK` probe opens + WouldBlock-as-end-of-head; Batch 172,
//!   mezura)
//!
//! ## Context-dependent rules (`context_notes`)
//!
//! Forty-four rules carry a `context_notes` annotation (as of Batch 112;
//! the count is pinned by `forty_three_annotated_rules_carry_context_notes`
//! — the fn name is one batch behind its own assert, which reads 44). The
//! class ledger below is the accounting as it grew batch by batch. Ten record that
//! the Pareto validator found them **`ParetoDominated`** or **Mixed** in the
//! default configuration: the original 4 from Bench 013 (`precompute-table`,
//! `papaya-lockfree`, `dedupe-hashset-vs-vec`, `reuse-allocation-clear`) + 2
//! from Bench 016 / Issue 012 (`sort-by-key`, `copy-from-slice`) + 2 from
//! Bench 48 (`filter-count-to-any` — dominated at mid-position hits by the
//! VECTORIZED count loop, `iter-copied-deref-map` — Mixed, 0.80× at 1M on
//! sum folds) + 2 from Batch 53 / Bench 038 (`powi-vs-powf` all-Tie,
//! `soa-hot-field` latency-bound Mixed). Two more (Issue 014 T2.3/T2.4)
//! annotate the `NotBenchable` metric-mismatch class: `tokio-guard-across-await` +
//! `bounded-channel-backpressure`. The regime annotations from Batch 62/64
//! (`newton-refinement-vs-hardware-transcendental`, `debug-assert-length-hoist`)
//! and Batch 65's two (the arch-granularity caveat on `align-64-false-sharing`,
//! a clean winner whose FIX is arch-dependent; and the pipeline-overlap
//! regime on `channel-send-batching-unpark`) complete the set; Batch 91's
//! two (the two-regime note on `filter-before-expensive-transform`, the
//! invariance-precondition note on `decision-projection-before-construction`)
//! extend it; Batch 94's cacheability note and Batch 99's measured-verdict +
//! regime-boundary note on `mir-destprop-branch-site-merge` complete the
//! current set; Batch 172 added four annotations (the dual-mask GOAT lead +
//! the three `NotBenchable` mezura decision-content rules) and flipped
//! `byte-find-position-to-memchr` from unannotated to annotated (the anchor-
//! cover selection rider) while extending `reuse-allocation-clear`'s note
//! with the retention-clamp rider. The patterns
//! are genuine wins in their respective regimes and are kept in the corpus as
//! documented patterns, but consumers should treat annotated rules as
//! context-dependent — not unconditional suggestions.

use crate::corpus::Corpus;
use crate::domains::rust_perf::RustPerfDomain;

mod entries_01;
mod entries_02;
mod entries_03;
mod entries_04;
mod entries_05;
mod entries_06;
mod entries_07;
mod entries_08;
mod entries_09;
mod entries_10;
mod entries_11;

#[cfg(test)]
mod tests;

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 167 entries
/// (Batch 196, 2026-10-02, Plan 195: +2 from the `TensorFold` v0.6.0 distill —
/// the grammar-window draft prune (complexity) + the startup-estimate
/// allocation-shapes rule; the 163 figure below was the Batch 182 state and
/// had gone stale — B186's +2 never moved it) covering the main perf
/// pattern categories (complexity, allocation, simd,
/// layout, iterators, concurrency, io, arithmetic, dispatch, caching, bevy,
/// wasm; live counts repaired at Batch 196 — complexity 31 (24 in the prior
/// prose was stale through B177/B179/B181/B182/B186's complexity adds),
/// allocation 29, io 26, concurrency 23, layout 13, iterators 11, simd 9,
/// caching 8, arithmetic 8, dispatch 6, bevy 2, wasm 1).
/// Allocation is the deepest family (29 — Batch 173 added the two
/// serde zero-copy rules; Batch 196 added the startup-estimate
/// allocation-shapes rule); complexity has 31 (the measurement-honesty family
/// lives here — B109 the work, the 2609.04040 rule the design, B115 the
/// config, Batch 177 the comparator (A/A control) + the observer-consumer
/// bias); concurrency has 23
/// (Issue 014 added the two metric-mismatch rules; Batch 65 added the two
/// contention rules; Batch 135 added the zero-cost atomic conversion + the
/// zero-sync partition; Batch 136 added the dynamic-work ladder + the
/// sender-termination idiom + the `par_bridge` deadlock; Batch 152 added the
/// three dial9 tokio rules — the blocking-pool segment hoist, the adaptive
/// consecutive-ready yield, the semaphore fan-out bound; Batch 153 added the
/// blocking-sleep async twin + the rayon oneshot bridge — the corpus's first
/// tokio×rayon boundary rule; Batch 154 added the oneshot response-send
/// hygiene + the bounded-channel-cycle deadlock; Batch 172 added the
/// crossbeam steal-batch rule; Batch 174 added the host-lock re-entrant
/// release rule + the deferred-command flush-hoist);
/// arithmetic has 8 (Batch 74 added 1); dispatch has 6 (Batch 61 + 74 added
/// 1 each; Batch 99 added the `DestProp` branch-site rule; Batch 101 added
/// the runtime-table handler-pair hoist; Batch 182 added the host-mirror
/// shared-scalar rule); caching has 8 (Batch 181 added the mask-memo owner-lifetime
/// rule); io has 26 (the
/// measurement-honesty io rules ride Batch 122/123/129; Batch 139 added the
/// prefault-admission rule; Batch 141 added the tail-boundary and
/// stream-holdback rules; Batch 153 added the `TCP_NODELAY` request-response
/// rule; Batch 154 added the vectored-write capability flag; Batch 172 added
/// the mmap-settled-size-gated read + the nonblocking head-probe open); bevy
/// has 2; wasm has 1. simd has 9 (Batch 172 added the dual-mask fused block
/// scan — the corpus's first hand-SIMD `std::arch` entry with both AVX2 and
/// NEON arms).
#[must_use]
pub fn build_perf_seed_corpus() -> Corpus<RustPerfDomain> {
    let mut entries = entries_01::entries();
    entries.extend(entries_02::entries());
    entries.extend(entries_03::entries());
    entries.extend(entries_04::entries());
    entries.extend(entries_05::entries());
    entries.extend(entries_06::entries());
    entries.extend(entries_07::entries());
    entries.extend(entries_08::entries());
    entries.extend(entries_09::entries());
    entries.extend(entries_10::entries());
    entries.extend(entries_11::entries());
    Corpus::new(entries)
}
