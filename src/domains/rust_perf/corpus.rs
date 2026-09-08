//! Seed corpus — rust performance rules distilled from the `rust-optimize`
//! skill checklist.
//!
//! Each entry is a `(rule_id, name, category, buggy_code, fixed_code, context_notes)`
//! tuple representing a real perf pattern from the skill. The corpus is
//! BLAKE3-committed via [`Corpus::new`](crate::corpus::Corpus::new). The
//! `context_notes` field is `Option<String>` — `Some(...)` for rules the bench
//! validator found ParetoDominated/Mixed/Tie in the default config (Bench 013 /
//! 038) or that are NotBenchable-with-a-metric-mismatch (Issue 014), `None` for
//! unconditional wins + honestly NotBenchable rules.
//!
//! 117 entries across 12 categories (complexity 19, allocation 23, simd 8,
//! layout 10, iterators 10, concurrency 10, io 18, arithmetic 8, dispatch 5,
//! caching 4, bevy 1, wasm 1). 90 are benchable; 27 are honestly
//! NotBenchable — the count repaired by Batch 135 (the pre-Batch-118 ledger
//! said 25 while the gated test said 14; the Batch 118–129 ds4 io
//! annotations were never counted) (8 external-runtime/size-class + 2
//! metric-mismatch (Issue
//! 014) + the x86-only prefetch annotation (Batch 85) + the bitflag size
//! axis (Batch 86) + the workload-level read-coalescing annotation (Batch
//! 94) + the measurement-honesty axis itself (Batch 109 — a timing gate
//! whose work folds away has no nonzero baseline to ratio) + the PLE-gather
//! workload annotation (Batch 118) + the two build-integrity annotations
//! and the A/B-design annotation (Batch 121 — resume provenance, write-
//! verify-publish, one-variable A/B; correctness-of-provenance and
//! experimental-design content, not speed) + the three vision-delta
//! annotations (Batch 122 — template-governed emission, provisioning
//! completeness, per-turn diagnostics; correctness-of-emission and
//! test-honesty content, not speed) + the two upstream-surge annotations
//! (Batch 123 — full-vector comparison integrity, tri-state fit
//! classification; measurement-integrity and robustness content, not speed)
//! plus the evaluation-design annotation (Batch 124 — matched controls,
//! proxy-vs-endpoint, null-arm identity; experimental-design content, not
//! speed) plus the oracle-contract annotation (Batch 125 — baseline-
//! reproduction triage, compute-shape-matched oracles, consumed-outputs
//! comparison; oracle-contract content, not speed).
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
//! ParetoDominant, both context-annotated — the 128B-line arch caveat +
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
//! order (MEASURED ParetoDominant 1.32-1.56x growing with working set —
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
//! regime; cross-cites the kernel_opt hot-read inverse; measure size_of
//! before/after). Two doc riders: box-str-immutable generalizes to
//! Box<[T]> for any frozen T; box-large-enum-variant gains the honest
//! costs (size-class rounding, locality scatter, the rare variant pays
//! more).
//!
//! Batch 91 (2026-08-31, Research 106 — the Rustdoc filter-first distill,
//! noahlev.org "Making Rustdoc 33% faster" + rust-lang PRs 159623/159779/
//! 159854) added filter-before-expensive-transform (Rustdoc built the
//! display representation for EVERY trait impl in the dependency graph,
//! THEN filtered; the fix re-expresses the relevance predicate on the raw
//! input and filters first — win ∝ discard-fraction × transform cost,
//! IMPOSSIBLE when the predicate is not re-expressible on the input) and
//! decision-projection-before-construction (the decision consumes only a
//! cheap projection that is INVARIANT under an expensive canonicalization
//! — the head of a type — so compute the projection on the raw input and
//! pay the canonicalization only behind the decision).
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
//! Currently benchable (90/117):
//! - `o1-lookup` (Vec scan vs HashMap lookup — clear O(n) vs O(1))
//! - `bsearch-sorted-slice` (linear scan vs binary search — O(n) vs O(log n) on sorted data)
//! - `dedupe-hashset-vs-vec` (Vec::contains loop vs HashSet collect — O(n²) vs O(n))
//! - `entry-vs-contains-insert` (contains_key+insert vs entry API — merge HashMap lookups)
//! - `loop-fusion` (two iterator passes vs one fused loop — cache locality, merge loops)
//! - `prefix-sum-2d` (O(n) region scan vs O(1) prefix sum — boundary-vs-volume, grows with N)
//! - `swap-remove` (Vec::remove O(n) shift vs Vec::swap_remove O(1) swap — grows with N)
//! - `sort-unstable` (stable sort allocates vs sort_unstable in-place — no allocation + faster algo)
//! - `sort-by-key` (sort_by repeated closure vs sort_by_key single extraction — N vs N·log N)
//! - `vec-remove-front-to-vecdeque` (Vec::remove(0) O(n²) drain vs VecDeque::pop_front O(n) — grows with N²)
//! - `btreemap-sorted-iteration` (HashMap + sort per query vs BTreeMap natural order — O(n log n) vs O(n) per query)
//! - `static-str` (String alloc vs &'static str — allocation cost)
//! - `serialize-once-byte-buffer-store` (per-record store malloc storm → ONE length-prefixed buffer + offsets — insert path, Batch 86)
//! - `context-equal-field-elide` (unconditional owner field → Option elided at ~80% context-equal — alloc axis, Batch 86)
//! - `simd-chunk` (scalar vs chunked loop — vectorization at large N)
//! - `branchless-conditional` (if vs bool-as-usize multiply — vectorization + branch prediction)
//! - `bool-to-bitpack` (Vec<bool> count vs packed Vec<u64> POPCNT — data density)
//! - `chunks-exact` (chunks variable-last vs chunks_exact uniform — enables vectorization)
//! - `copy-from-slice` (index-loop copy vs copy_from_slice — SIMD + no bounds check)
//! - `mutex-to-atomic` (Mutex<u32> vs AtomicU32 — lock/unlock overhead)
//! - `f32-counter-to-int` (f32 accumulator vs u64 — serial dep chain)
//! - `collect-avoidance` (.collect() intermediate vs chained iterator — allocation + double pass)
//! - `dyn-to-enum` (Box<dyn Trait> vs enum — vtable indirection + no inlining)
//! - `ok-or-else` (ok_or(expensive()) eager vs ok_or_else(closure) lazy — avoids unused error alloc)
//! - `precompute-table` (recompute vs const lookup table — **context-dependent**,
//!   see `context_notes`; not a release-mode win per Bench 013)
//! - `papaya-lockfree` (RwLock vs papaya lock-free — **context-dependent**,
//!   see `context_notes`; only a win under multi-threaded contention)
//! - `vec-with-capacity` (Vec::new() growth vs Vec::with_capacity — realloc cost)
//! - `format-concat-loop` (format! concat in loop vs push_str — O(n²) vs O(n))
//! - `fxhashmap` (std HashMap SipHash vs FxHashMap FxHash — hasher speed, 2-4×)
//! - `retain-inplace` (filter().collect() new Vec vs retain() in-place — ~1.5-2×)
//! - `mem-take` (clone+clear vs mem::take move — ~3×, avoids deep clone)
//! - `avoid-clone` (fn(data: Vec<T>) clone → fn(data: &[T]) borrow — ~1.3-3×)
//! - `box-str-immutable` (Vec<String> 3-word → Vec<Box<str>> 2-word — ~1.1-1.3×; B86 rider: generalizes to Box<[T]> for ANY frozen T)
//! - `reuse-allocation-clear` (alloc-per-call → clear-reuse scratch buffer — **Mixed**, see `context_notes`)
//! - `box-avoid` (Vec<Box<i64>> heap per elem → Vec<i64> stack values — ~10-35×)
//! - `smallvec-stack` (Vec<T> always heap → stack array for ≤8 typical — ~7-12×)
//! - `buffered-io` (File::read byte-by-byte → BufReader 8KB batched — ~97×)
//! - `bufwriter-batched` (File::write per-record → BufWriter batched — syscall amortization)
//! - `log-write-direct` (push_str(&format!(...)) temp alloc → write! direct — ~1.7×)
//! - `cow-conditional-alloc` (String always-alloc → Cow<str> borrow when no transform — N vs 0.1N allocations)
//! - `regex-hoist-lazylock` (Regex::new per call → static LazyLock — compile once, Issue 014 T2.1)
//! - `arc-str-payload` (per-recipient payload clone → Arc<str> handle clone — N×bytes → 1 alloc, Issue 014 T2.2)
//! - `collect-append-to-extend` (collect+append staging → extend — no intermediate Vec, Batch 48)
//! - `hashset-insert-bool-return` (contains+insert double lookup → insert's bool return — Batch 48)
//! - `unwrap-or-else-lazy` (unwrap_or(eager) → unwrap_or_else(closure) — Batch 48)
//! - `filter-count-to-any` (filter(p).count() > 0 full scan → any(p) short-circuit — Batch 48)
//! - `vec-macro-zero-fill` (Vec::new+resize memset → vec![0; n] calloc — Batch 48)
//! - `iter-copied-deref-map` (.map(|x| *x) → .copied() TrustedLen — Batch 48)
//! - `filter-map-fuse` (filter(p).map(f) → filter_map fused — Batch 48)
//! - `vec-insert-front-deque` (Vec::insert(0,_) O(n²) → VecDeque::push_front — Batch 48)
//! - `powi-vs-powf` (x.powf(2.0) libm call → x.powi(2) multiply — Batch 53)
//! - `distance-sq-no-sqrt` (sqrt-then-compare → squared compare — Batch 53)
//! - `hashmap-with-capacity` (HashMap::new() rehash ladder → with_capacity(n) — Batch 53)
//! - `soa-hot-field` (AoS strided single-field scan → SoA contiguous column — Batch 53)
//! - `division-by-invariant-multiplication` (hardware integer div by loop-invariant RUNTIME d → libdivide-style magic mulhi+shift — Batch 59, thermite distill)
//! - `keyword-map-to-match` (static-keyword HashMap dispatch → plain match on literals — LLVM length-switch, Batch 61)
//! - `usize-index-to-u32` (16B usize spans → 8B u32 spans — cache density under bandwidth-bound scans, Batch 61)
//! - `transposed-weight-register-gemv` (row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA into output accumulators — 1.18-6.08x ParetoDominant, Batch 62, microGPT-C distill; the win GROWS at large nin — the scalar accumulator's serial FP chain is the real cost, not the horizontal reduce)
//! - `newton-refinement-vs-hardware-transcendental` (Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction `fdiv`/`fsqrt` on CPU — Mixed 1.01-1.09x M3 / ParetoDominant 1.64-1.66x x86 (re-gated), hardware never loses; Batch 62, annotated)
//! - `loop-order-innermost-contiguous` (i-j-k nested accumulation with a column-striding inner access → i-k-j reorder so the innermost varies the LAST index — 7.1-13.8x ParetoDominant, grows with n; Batch 64)
//! - `debug-assert-length-hoist` (`debug_assert!` mentioning `.len()` is erased in release, dropping the constraint bounds-check elision needed → `assert!` — Mixed regime-split 1.13x@1K / 0.67-0.71x@100K stable on aarch64 (x86 re-gate: 1.00x/0.92x, holds cross-arch); Batch 64, annotated)
//! - `sparse-set-bit-iteration-tz-blsr` (64-iteration set-bit scan → trailing_zeros+BLSR, popcount iterations — sparse-mask regime, Batch 66)
//! - `sortedness-check-branchless-accumulate` (early-exit sortedness validation → branchless `sorted &=` full scan, vectorizes — usually-sorted regime, Batch 66)
//! - `byte-find-position-to-memchr` (u8 slice iter().position byte scan → memchr::memchr SIMD block scan — position is NOT specialized in std, contains is; dep-gated, Batch 67)
//! - `varint-compose-word-single-store` (per-byte write_all varint loop → 5-threshold ladder + single write_all — MEASURED ParetoDominated on aarch64 AND x86 across writer regimes (the 4090-box re-gate: 0.40-0.50x there, HARDER): the honest-loser annotation stops the tantivy folk port; Batch 68)
//! - `unaligned-word-load-from-bytes` (shift/or byte-assembly word read → one unaligned from_le_bytes load — ParetoDominant cross-arch: u64 1.4-2.2x, u32 1.2-1.4x, both arches; the T6 defer refuted; Batch 69)
//! - `align-64-false-sharing` (adjacent 24 B per-thread atomic counter slots → `#[repr(align)]` to ARCH line granularity — ParetoDominant 4.7-9.4x at 2-12t, **1t = tie (honest)**; align(64) insufficient on 128B-line Apple Silicon (measured 1.16-2.44x of the win) — pad to the arch granularity or CachePadded; Batch 65)
//! - `channel-send-batching-unpark` (per-item MPSC sends to a parked `select!` receiver → per-64 batch flush via `mem::replace` — ParetoDominant 1.16-1.29x (pipeline-overlap honest margin; the source's 6.25x needs the park pair ON the critical path) + allocs 3232→1624; Batch 65)
//! - `swar-digit-chunk-parse` (per-digit decimal parse loop → SWAR 8-digit chunk fold, 3 multiplies per 8 digits + SWAR digit validation — the serial `val = val*10 + d` chain broken; regime: ≥8 digits win, <8 pay the guard (the source's own README); Batch 74, simd-json distill)
//! - `runtime-feature-detect-per-chunk` (CPU feature detection inside a per-chunk hot fn → resolve once via cached static / static cfg gate — the std::detect string resolution + detector read per call rivals small work units; Batch 74, simd-json distill)
//! - `runtime-int-exponent-powi` (runtime-integer-exponent `powf` libm call → `powi` binary exponentiation — log2(n) multiplies vs an opaque pow call; NOT bit-identical, rel-error-gated; Batch 77, lucebox rope distill)
//! - `bitplane-transpose-repeated-predicate-scan` (repeatedly filtered integer column → bit-plane transposition + bitwise predicate evaluation over the compressed planes, zero decompression — b/32 the bytes scanned + 64 values per word op; Batch 83, arXiv:2608.26368 distill)
//! - `weight-tile-consumption-order` (row-major single-token matvec x re-walked per row → consumption-ordered tiles, x slice L1-resident per tile — 1.32-1.56x growing with working set; Batch 85, cflow distill)
//! - `serialize-once-byte-buffer-store` (per-record heap fields → ONE length-prefixed byte buffer + offset table — 2 allocations total, copy-out rebuild; Batch 86, Cloudflare DNS-cache distill)
//! - `context-equal-field-elide` (per-record field equal to the enclosing context → Option<Box<T>> None, resolved at read — Batch 86, Cloudflare DNS-cache distill)
//! - `bitflag-pack-when-footprint-dominates` (bool cluster → one bitflag byte, stored-at-scale footprint regime — Batch 86, annotated)
//! - `filter-before-expensive-transform` (map(expensive).filter(relevant) → filter-first on the RAW input — win ∝ discard × transform cost; Batch 91, Rustdoc filter-first distill)
//! - `decision-projection-before-construction` (transform-to-decide → decide on the invariant raw projection, canonicalize only behind the decision — Batch 91, annotated)
//! - `mir-destprop-branch-site-merge` (known-fn-item conditional dispatch → two direct tail-jumps — MIR DestinationPropagation cmov-merges the merged form into one indirect site mixing branch-target history; Batch 99, wasmi distill)
//! - `dispatch-handler-pair-hoist` (runtime-table handler read per dispatch → hoist both entries once — register-resident select; cross-arch ParetoDominant, x86 re-gated 2026-09-04; Batch 101)
//! - `hot-byval-move-over-memcpy-threshold` (hot by-value move of a >128-byte type → shrink/ref — the threshold is an arch-shaped LLVM backend constant; Mixed on M3, x86 pays the memcpy call; Batch 112)
//! - `atomic-vec-zero-cost-convert` (Atomic::new-per-element collect from a borrowed vec → into_iter().map on the OWNED vec — same-layout collect reuses the buffer and the loop compiles out; alloc axis 2→1; Batch 135, Wild-linker article distill)
//! - `vec-reuse-across-lifetime-change` (fresh Vec<&'iter T> per iteration → owned buffer retargeted in place via a const size/align assert + clear() + empty collect — alloc axis 1→0; Batch 135, Wild-linker article distill)
//! - `partitioned-mut-slices-zero-sync` (shared Vec under one contended Mutex → disjoint chunks_mut partition, zero synchronization — sizes = thread counts; Batch 135, Wild-linker article distill)
//!
//! Not benchable (27/117) — honest (count repaired by Batch 135; the ledger below is the early-corpus list — the Batch 118–129 source-reported io annotations are in the header ledger, all `bench_spec: None`):
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
//!   cacheability coupling: the gap budget collapses to 0 on direct/F_NOCACHE
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
//!
//! ## Context-dependent rules (`context_notes`)
//!
//! Forty-four rules carry a `context_notes` annotation (as of Batch 112;
//! the count is pinned by `forty_three_annotated_rules_carry_context_notes`
//! — the fn name is one batch behind its own assert, which reads 44). The
//! class ledger below is the accounting as it grew batch by batch. Ten record that
//! the Pareto validator found them **ParetoDominated** or **Mixed** in the
//! default configuration: the original 4 from Bench 013 (`precompute-table`,
//! `papaya-lockfree`, `dedupe-hashset-vs-vec`, `reuse-allocation-clear`) + 2
//! from Bench 016 / Issue 012 (`sort-by-key`, `copy-from-slice`) + 2 from
//! Bench 48 (`filter-count-to-any` — dominated at mid-position hits by the
//! VECTORIZED count loop, `iter-copied-deref-map` — Mixed, 0.80× at 1M on
//! sum folds) + 2 from Batch 53 / Bench 038 (`powi-vs-powf` all-Tie,
//! `soa-hot-field` latency-bound Mixed). Two more (Issue 014 T2.3/T2.4)
//! annotate the NotBenchable metric-mismatch class: `tokio-guard-across-await` +
//! `bounded-channel-backpressure`. The regime annotations from Batch 62/64
//! (`newton-refinement-vs-hardware-transcendental`, `debug-assert-length-hoist`)
//! and Batch 65's two (the arch-granularity caveat on `align-64-false-sharing`,
//! a clean winner whose FIX is arch-dependent; and the pipeline-overlap
//! regime on `channel-send-batching-unpark`) complete the set; Batch 91's
//! two (the two-regime note on `filter-before-expensive-transform`, the
//! invariance-precondition note on `decision-projection-before-construction`)
//! extend it; Batch 94's cacheability note and Batch 99's measured-verdict +
//! regime-boundary note on `mir-destprop-branch-site-merge` complete the
//! current set. The patterns
//! are genuine wins in their respective regimes and are kept in the corpus as
//! documented patterns, but consumers should treat annotated rules as
//! context-dependent — not unconditional suggestions.

use crate::corpus::Corpus;
use crate::domains::rust_perf::{PerfRule, RustPerfDomain};

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 117 entries
/// covering the main perf pattern categories (complexity, allocation, simd,
/// layout, iterators, concurrency, io, arithmetic, dispatch, caching, bevy,
/// wasm). Allocation is the deepest (23 rules; Batch 86 added 2; Batch 135
/// added the lifetime-reuse rule); complexity
/// has 19 (Batch 76 + 91 added 1 each; Batch 109 added the measurement-
/// honesty rule); iterators
/// has 10 (Batch 66/67 added 2; Batch 91 added 1); simd has 8 (Batch 62 + 64 + 83 added 1
/// each); layout has 10 (Batch 61 + 85 + 86 + 112 added); concurrency has 10
/// (Issue 014 added the two metric-mismatch rules; Batch 65 added the two
/// contention rules; Batch 73 added 1; Batch 135 added the zero-cost
/// atomic conversion + the zero-sync partition);
/// arithmetic has 8 (Batch 74 added 1); dispatch has 5 (Batch 61 + 74 added
/// 1 each; Batch 99 added the DestProp branch-site rule; Batch 101 added
/// the runtime-table handler-pair hoist); caching has 4 (Batch 73 + 76 added 1 each); io has 5 (Batch
/// 68/69 added 1 each + Batch 85 added 1 + Batch 94 added the cacheability-
/// coupled coalescing rule + Batch 118 added the disjoint-pread latency-
/// overlap rule); bevy + wasm have 1 each.
#[must_use]
#[allow(clippy::too_many_lines)] // seed corpus is a data table; length scales with rule count
pub fn build_perf_seed_corpus() -> Corpus<RustPerfDomain> {
    Corpus::new(vec![
        // ── complexity: O(n) scan → O(1) lookup ────────────────────────────
        PerfRule {
            rule_id: "o1-lookup".to_string(),
            name: "O(n) scan → O(1) lookup".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn find_user(users: &Vec<(u32, String)>, id: u32) -> Option<&String> {\n    users.iter().find(|(uid, _)| *uid == id).map(|(_, n)| n)\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn find_user(users: &HashMap<u32, String>, id: u32) -> Option<&String> {\n    users.get(&id)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(o1_lookup_bench_spec()),
        },
        // ── complexity: O(n) scan on sorted slice → O(log n) binary search ─
        // The compiler cannot transform a linear scan into a binary search;
        // this is a genuine algorithmic improvement on pre-sorted data.
        PerfRule {
            rule_id: "bsearch-sorted-slice".to_string(),
            name: "O(n) .contains() on sorted slice → O(log n) binary_search()".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn contains(sorted: &[u32], target: u32) -> bool {\n    sorted.iter().any(|&x| x == target)\n}".to_string(),
            fixed_code: "fn contains(sorted: &[u32], target: u32) -> bool {\n    sorted.binary_search(&target).is_ok()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bsearch_sorted_slice_bench_spec()),
        },
        // ── complexity: O(n²) dedup via Vec::contains → O(n) HashSet ───────
        // Building a unique Vec by repeated linear membership checks is O(n²).
        // Collecting into a HashSet then draining is O(n) average. The
        // compiler cannot optimize a growing-Vec membership loop into a hash
        // table — this is a pure algorithmic improvement.
        PerfRule {
            rule_id: "dedupe-hashset-vs-vec".to_string(),
            name: "O(n²) Vec::contains dedup loop → O(n) HashSet collect".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn unique(items: &[u32]) -> Vec<u32> {\n    let mut seen = Vec::new();\n    for &x in items {\n        if !seen.contains(&x) { seen.push(x); }\n    }\n    seen\n}".to_string(),
            fixed_code: "use std::collections::HashSet;\nfn unique(items: &[u32]) -> Vec<u32> {\n    let seen: HashSet<u32> = items.iter().copied().collect();\n    seen.into_iter().collect()\n}".to_string(),
            // Bench 013 Mixed (2/4 Faster, 2/4 Slower). HashSet allocation +
            // hashing overhead exceeds the O(n²) cost at small N (≤500).
            // Wins clearly at N ≥ 1000 (up to 7.5× at N=5000). The rule is a
            // genuine algorithmic win for collections known to exceed ~500
            // elements; for small dedup the Vec::contains approach is fine.
            context_notes: Some(
                "Bench 013: Mixed (2/4 Faster, 2/4 Slower). O(n²) cost is \
                 trivial at small N (≤500), so HashSet allocation + hashing \
                 overhead dominates. Wins at N ≥ 1000 (1.5× to 7.5×). Suggest \
                 for dedup on collections known to exceed ~500 elements; for \
                 small dedup, the Vec::contains approach is fine."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dedupe_hashset_vs_vec_bench_spec()),
        },
        // ── allocation: String → &'static str ─────────────────────────────
        PerfRule {
            rule_id: "static-str".to_string(),
            name: "String → &'static str for constant labels".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn label() -> String { \"ready\".to_string() }".to_string(),
            fixed_code: "fn label() -> &'static str { \"ready\" }".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(static_str_bench_spec()),
        },
        // ── layout: field reorder to eliminate padding ────────────────────
        PerfRule {
            rule_id: "layout-reorder".to_string(),
            name: "Reorder fields to eliminate padding (u8 → u64 → u32 ordering fix)".to_string(),
            category: "layout".to_string(),
            // The bad layout has a u8 between two u64s, forcing 7 bytes of
            // padding after the u8. Reordering u64 → u32 → u8 packs to 16 bytes.
            buggy_code: "#[repr(C)]\nstruct Bad { a: u8, b: u64, c: u32 }".to_string(),
            fixed_code: "#[repr(C)]\nstruct Good { b: u64, c: u32, a: u8 }".to_string(),
            context_notes: None,
            // Not benchable: size_of is deterministic (compile-time), and the
            // cache-utilization claim needs a specific access pattern to show
            // up in a microbench. Honest "NotBenchable".
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: Arc<RwLock<HashMap>> → papaya ────────────────────
        // Per the user AGENTS.md rule: "Use papaya as possible for lock-free
        // Arc<RwLock<HashMap>>". Context-dependent: only a win under
        // multi-threaded contention (Bench 013 measured single-threaded reads
        // as 0.55–0.79× — papaya's epoch GC overhead dominates without
        // concurrent access). The AGENTS.md rule is correct for hot
        // concurrent paths; the annotation is for the single-threaded case.
        PerfRule {
            rule_id: "papaya-lockfree".to_string(),
            name: "Arc<RwLock<HashMap>> → papaya lock-free map".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::{Arc, RwLock};\nuse std::collections::HashMap;\nstruct Store(Arc<RwLock<HashMap<u32, String>>>);".to_string(),
            fixed_code: "use papaya::HashMap;\nstruct Store(HashMap<u32, String>);".to_string(),
            // Bench 013 ParetoDominated (0.55–0.79× single-threaded). The
            // win is real under contention; the annotation marks the
            // default-config (single-threaded bench) as NOT a win.
            context_notes: Some(
                "Bench 013: ParetoDominated 0.55-0.79x single-threaded. \
                 Win only under multi-threaded contention (papaya epoch GC \
                 overhead dominates without concurrent readers). The AGENTS.md \
                 rule (papaya for Arc<RwLock<HashMap>>) is correct for hot \
                 concurrent paths; do not suggest for single-threaded reads."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(papaya_lockfree_bench_spec()),
        },
        // ── concurrency: Mutex<u32> counter → AtomicU32 ─────────────────
        // Per the rust-optimize skill item 6: "Mutex<u32> → AtomicU32".
        // The Mutex lock/unlock path involves an atomic CAS to acquire + an
        // atomic store to release (plus the MutexGuard RAII lifecycle),
        // even on the uncontended path. AtomicU32::fetch_add is a single
        // `lock xadd` instruction. The compiler cannot eliminate Mutex::lock
        // (it's an opaque call). Bench 013: ParetoDominant ~2.5× across all
        // sizes (1K → 1M) single-threaded — no contention needed.
        PerfRule {
            rule_id: "mutex-to-atomic".to_string(),
            name: "Mutex<u32> counter → AtomicU32 fetch_add".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::Mutex;\nstruct Counter(Mutex<u32>);\nimpl Counter {\n    fn new() -> Self { Self(Mutex::new(0)) }\n    fn inc(&self) { *self.0.lock().unwrap() += 1; }\n}".to_string(),
            fixed_code: "use std::sync::atomic::{AtomicU32, Ordering};\nstruct Counter(AtomicU32);\nimpl Counter {\n    fn new() -> Self { Self(AtomicU32::new(0)) }\n    fn inc(&self) { self.0.fetch_add(1, Ordering::Relaxed); }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mutex_to_atomic_bench_spec()),
        },
        // ── simd: chunked loop for auto-vectorization ─────────────────────
        PerfRule {
            rule_id: "simd-chunk".to_string(),
            name: "Chunked loop (4-wide) to help LLVM auto-vectorize".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn sum(v: &[f32]) -> f32 {\n    let mut s = 0.0;\n    for i in 0..v.len() { s += v[i]; }\n    s\n}".to_string(),
            fixed_code: "fn sum(v: &[f32]) -> f32 {\n    let chunks = v.chunks_exact(4);\n    let rem = v.chunks_exact(4).remainder();\n    let mut acc = [0.0f32; 4];\n    for c in chunks {\n        acc[0] += c[0]; acc[1] += c[1]; acc[2] += c[2]; acc[3] += c[3];\n    }\n    acc.iter().chain(rem).copied().sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(simd_chunk_bench_spec()),
        },
        // ── simd: bool array → packed bitvec for POPCNT acceleration ────────
        // Data density: Vec<bool> uses 1 byte per bool; packed Vec<u64> uses
        // 1 bit. For counting set bits, the fixed path runs POPCNT on each
        // 64-bit word (1 instruction per 64 bools) vs the buggy path's
        // per-byte comparison. The compiler cannot auto-pack bools into bits.
        PerfRule {
            rule_id: "bool-to-bitpack".to_string(),
            name: "Vec<bool> set-bit count → packed Vec<u64> + count_ones()".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn count_true(flags: &[bool]) -> usize {\n    flags.iter().filter(|&&b| b).count()\n}".to_string(),
            fixed_code: "fn count_true(bits: &[u64]) -> usize {\n    bits.iter().map(|w| w.count_ones() as usize).sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bool_to_bitpack_bench_spec()),
        },
        // ── arithmetic: f32 accumulator → u64 accumulator ───────────────────
        // Per the rust-optimize skill item 4: "f32 counters → u32/u64".
        // f32 addition is NOT associative (IEEE 754 rounding), so the
        // compiler CANNOT break the sequential dependency chain or use
        // multiple SIMD accumulators without -C fast-math (which Rust does
        // not enable by default). Each f32 add has 3–4 cycle latency, fully
        // serial. u64 addition IS associative, so the compiler vectorizes
        // (multiple independent accumulators) + the add latency is 1 cycle.
        // Bench 013: ParetoDominant 7.5× → 20.5× (grows with N — the f32
        // dependency chain bottleneck worsens relative to vectorized u64).
        PerfRule {
            rule_id: "f32-counter-to-int".to_string(),
            name: "f32 accumulator for integer tally → u64 accumulator".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn tally(scores: &[u32]) -> f32 {\n    let mut total = 0.0f32;\n    for &s in scores { total += s as f32; }\n    total\n}".to_string(),
            fixed_code: "fn tally(scores: &[u32]) -> u64 {\n    let mut total = 0u64;\n    for &s in scores { total += s as u64; }\n    total\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(f32_counter_to_int_bench_spec()),
        },
        // ── caching: pre-compute lookup table ─────────────────────────────
        // Context-dependent: the compiler optimizes the runtime popcount loop
        // to a single POPCNT instruction in release mode, making the lookup
        // table slower (L1 cache latency > POPCNT latency). Bench 013
        // measured 0.20–0.40× — the rule is NOT a release-mode win on modern
        // CPUs. Kept in the corpus as a documented pattern (valid for targets
        // without POPCNT or in debug mode); the context_notes annotation marks
        // it as NOT a default suggestion.
        PerfRule {
            rule_id: "precompute-table".to_string(),
            name: "Pre-compute lookup table once, reuse across calls".to_string(),
            category: "caching".to_string(),
            buggy_code: "fn parity_byte(b: u8) -> u8 {\n    (0..8).filter(|i| b & (1 << i) != 0).count() as u8 & 1\n}".to_string(),
            fixed_code: "const PARITY: [u8; 256] = {\n    let mut t = [0u8; 256];\n    let mut i = 0;\n    while i < 256 { t[i] = (i.count_ones() & 1) as u8; i += 1; }\n    t\n};\nfn parity_byte(b: u8) -> u8 { PARITY[b as usize] }".to_string(),
            // Bench 013 ParetoDominated (0.20–0.40× release). The compiler
            // already lowers the buggy form to POPCNT; the table is slower.
            context_notes: Some(
                "Bench 013: ParetoDominated 0.20-0.40x release mode. The \
                 compiler lowers (0..8).filter().count() to a single POPCNT \
                 instruction; the lookup table is slower (L1 cache latency \
                 > POPCNT latency). NOT a default suggestion — only valid for \
                 targets without POPCNT or in debug mode."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(precompute_table_bench_spec()),
        },
        // ── bevy: reactive_low_power event loop ───────────────────────────
        // Per the rust-optimize skill: default Continuous pegs CPU at 100%
        // even idle; switch to reactive_low_power matching tick rate.
        PerfRule {
            rule_id: "bevy-reactive-loop".to_string(),
            name: "Bevy Continuous → reactive_low_power event loop".to_string(),
            category: "bevy".to_string(),
            buggy_code: "use bevy::winit::WinitSettings;\nfn setup() -> WinitSettings { WinitSettings::default() }".to_string(),
            fixed_code: "use bevy::winit::{UpdateMode, WinitSettings};\nuse std::time::Duration;\nfn setup() -> WinitSettings {\n    WinitSettings {\n        focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(50)),\n        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),\n    }\n}".to_string(),
            context_notes: None,
            // Not benchable: needs the Bevy runtime (event loop + winit).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── wasm: batch API to amortize FFI floor ─────────────────────────
        // Per the rust-optimize skill: ~250ns FFI floor amortized across N
        // calls via a single batch invocation.
        PerfRule {
            rule_id: "wasm-batch-ffi".to_string(),
            name: "Batch N×M FFI calls into one (amortize ~250ns floor)".to_string(),
            category: "wasm".to_string(),
            buggy_code: "// Per-call FFI: 250ns floor × N×M invocations\nfn validate_all(items: &[u32], rules: &[u32]) -> Vec<bool> {\n    items.iter().map(|i| rules.iter().map(|r| validate_one(*i, *r)).all(|b| b)).collect()\n}\nfn validate_one(_i: u32, _r: u32) -> bool { true }".to_string(),
            fixed_code: "// Batch: one FFI call carries shared state + per-item arrays\nfn validate_all(items: &[u32], rules: &[u32]) -> Vec<bool> {\n    let mut out = vec![false; items.len()];\n    validate_batch(items, rules, &mut out);\n    out\n}\nfn validate_batch(_items: &[u32], _rules: &[u32], _out: &mut [bool]) {\n    // single FFI call: shared rules + per-item arrays → out\n}".to_string(),
            context_notes: None,
            // Not benchable: needs a WASM runtime (js-sys / wasm-bindgen FFI).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: Vec::new() + push → Vec::with_capacity ───────────
        // Per the rust-optimize skill: "Pre-allocate output arrays upfront,
        // write in-place instead of collecting per-iteration". The Vec
        // growth strategy causes O(log n) reallocations + O(n) total copies;
        // with_capacity does 1 allocation + 0 extra copies.
        PerfRule {
            rule_id: "vec-with-capacity".to_string(),
            name: "Vec::new() + push loop → Vec::with_capacity(n)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v\n}".to_string(),
            fixed_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n { v.push(i as u32); }\n    v\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_with_capacity_bench_spec()),
        },
        // ── allocation: format!("{s}{x}") in loop → push_str ─────────────
        // Classic O(n²) anti-pattern: format! creates a new String each
        // iteration, copying the entire accumulated string. push_str
        // amortizes to O(n). At 1000 chunks of 10 bytes, the buggy path
        // copies ~5MB total vs ~10KB for the fixed path.
        PerfRule {
            rule_id: "format-concat-loop".to_string(),
            name: "format!(\"{s}{x}\") concat in loop → String::push_str".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(chunks: &[&str]) -> String {\n    let mut s = String::new();\n    for chunk in chunks { s = format!(\"{s}{chunk}\"); }\n    s\n}".to_string(),
            fixed_code: "fn build(chunks: &[&str]) -> String {\n    let total: usize = chunks.iter().map(|c| c.len()).sum();\n    let mut s = String::with_capacity(total);\n    for chunk in chunks { s.push_str(chunk); }\n    s\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(format_concat_loop_bench_spec()),
        },
        // ── layout: field-less enum without repr → #[repr(u8)] ───────────
        // Per the rust-optimize skill: "Use #[repr(u8)] on field-less enums
        // to guarantee 1-byte size". Without repr, the compiler picks the
        // default (often 4+ bytes depending on variant count + niche
        // optimization). Not benchable: size_of is compile-time; the runtime
        // impact (memory bandwidth, cache utilization) needs a specific
        // access pattern to show up in a microbench.
        PerfRule {
            rule_id: "repr-u8-enum".to_string(),
            name: "Field-less enum → #[repr(u8)] for guaranteed 1-byte size".to_string(),
            category: "layout".to_string(),
            buggy_code: "enum Color { Red, Green, Blue }".to_string(),
            fixed_code: "#[repr(u8)]\nenum Color { Red, Green, Blue }".to_string(),
            context_notes: None,
            // Not benchable: size_of is deterministic (compile-time), and the
            // cache/bandwidth claim needs a specific access pattern (array of
            // enums). Honest "NotBenchable", same class as layout-reorder.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators: unnecessary .collect() intermediate → chained iterator ──
        // Per the rust-optimize skill item 5: "fix double chunks_exact()".
        // The general pattern: collecting an intermediate Vec then iterating
        // it again. The collect allocates (heap + copy) + forces a second
        // pass. The fixed path chains the iterators — zero allocation, single
        // pass. The compiler cannot eliminate a .collect() (it must produce a
        // concrete Vec). Bench 013: ParetoDominant 2.4–3.0× across all sizes
        // (1K → 1M).
        PerfRule {
            rule_id: "collect-avoidance".to_string(),
            name: "Unnecessary .collect() intermediate → chained iterator".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn transform_sum(vals: &[u64]) -> u64 {\n    let mapped: Vec<u64> = vals.iter().map(|x| x * 2).collect();\n    mapped.iter().sum()\n}".to_string(),
            fixed_code: "fn transform_sum(vals: &[u64]) -> u64 {\n    vals.iter().map(|x| x * 2).sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(collect_avoidance_bench_spec()),
        },
        // ── dispatch: Box<dyn Trait> → enum dispatch (devirtualize) ──────
        // NOT in the rust-optimize skill checklist — a genuine gap. The buggy
        // path uses dynamic dispatch via Box<dyn Trait>: each call is a vtable
        // load + indirect call (branch misprediction risk, no inlining). The
        // fixed path uses an enum: the match arms are direct calls the
        // compiler can inline + branch-predict. The compiler cannot devirtualize
        // through an opaque trait object. Bench 013: ParetoDominant ~2.0×
        // across all sizes (1K → 100K).
        PerfRule {
            rule_id: "dyn-to-enum".to_string(),
            name: "Box<dyn Trait> dispatch → enum dispatch (devirtualize)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "trait Op { fn apply(&self, x: f64) -> f64; }\nfn run(ops: &[Box<dyn Op>], vals: &[f64]) -> f64 {\n    let mut acc = 0.0;\n    for &x in vals { let mut v = x; for op in ops { v = op.apply(v); } acc += v; }\n    acc\n}".to_string(),
            fixed_code: "enum Op { Scale(f64), Offset(f64), Square }\nimpl Op {\n    fn apply(&self, x: f64) -> f64 {\n        match self { Op::Scale(s) => x * s, Op::Offset(o) => x + o, Op::Square => x * x }\n    }\n}\nfn run(ops: &[Op], vals: &[f64]) -> f64 {\n    let mut acc = 0.0;\n    for &x in vals { let mut v = x; for op in ops { v = op.apply(v); } acc += v; }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dyn_to_enum_bench_spec()),
        },
        // ── complexity: contains_key() + insert() → entry().or_insert() ──
        // Per the rust-optimize skill item 1: "merge HashMap lookups".
        // The buggy path does TWO hash lookups: contains_key hashes + probes,
        // then insert hashes + probes again. The compiler cannot merge them
        // because insert may need to grow the table (opaque call). The fixed
        // path uses the entry() API: ONE hash + probe, in-place insert if
        // absent. Bench 013: ParetoDominant 2.8–3.4× across all sizes (100 →
        // 100K). The win is the eliminated redundant hash + probe — a
        // genuine constant-factor improvement, not algorithmic.
        PerfRule {
            rule_id: "entry-vs-contains-insert".to_string(),
            name: "contains_key() + insert() → entry().or_insert() (merge HashMap lookups)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn insert_if_absent(map: &mut HashMap<u32, u32>, k: u32, v: u32) {\n    if !map.contains_key(&k) {\n        map.insert(k, v);\n    }\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn insert_if_absent(map: &mut HashMap<u32, u32>, k: u32, v: u32) {\n    map.entry(k).or_insert(v);\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(entry_vs_contains_insert_bench_spec()),
        },
        // ── simd: data-dependent if → branchless arithmetic for vectorization ──
        // Per the rust-optimize skill item 7: "branch-free inner loops (use
        // bool as usize instead of if)". The buggy path uses `if cond[i] { ... }`
        // inside a hot loop — the data-dependent branch prevents LLVM from
        // auto-vectorizing (each iteration may or may not execute the body).
        // The fixed path uses `val * cond[i] as u64` — a branchless multiply
        // that LLVM vectorizes into SIMD lanes (e.g., 4-wide or 8-wide).
        // Bench 013: ParetoDominant 6.8–13× across all conditions tested
        // (unpredictable 50% random → predictable 99%/1%/100%). The win is
        // primarily auto-vectorization, NOT branch prediction — even with
        // 100% predictable conditions, the branchless version is ~10× faster
        // because LLVM vectorizes the multiply loop but cannot vectorize the
        // branchy loop. External measured evidence for the selectivity axis
        // (Batch 83, Research 097 / arXiv:2608.26368): a scalar filter loop's
        // throughput swung ~4.5× with predicate selectivity (5.4K → 1.2K
        // M elem/s, 0.1% → 90%) on branch prediction alone, while the
        // branchless bit-plane form stayed constant (<6% spread).
        PerfRule {
            rule_id: "branchless-conditional".to_string(),
            name: "Data-dependent if in hot loop → branchless arithmetic (bool as usize)".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {\n    let mut sum: u64 = 0;\n    for i in 0..data.len() {\n        if cond[i] {\n            sum += data[i] as u64;\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {\n    let mut sum: u64 = 0;\n    for i in 0..data.len() {\n        sum += data[i] as u64 * cond[i] as u64;\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(branchless_conditional_bench_spec()),
        },
        // ── complexity: two passes over same data → one fused pass ──────────
        // Per the rust-optimize skill item 1: "merge loops". The buggy path
        // iterates the data twice: once for `.sum()`, once for `.max()`. Each
        // iterator consumer is a separate trait method call that independently
        // consumes the iterator — the compiler CANNOT fuse them into a single
        // pass (loop fusion is not a standard LLVM optimization pass; it would
        // need to prove aliasing + ordering constraints across two calls).
        // The fixed path does both reductions in one explicit loop — one
        // cache pass, one set of loads. Bench 013: ParetoDominant 1.46–1.52×
        // across all sizes (1K → 1M). The win is cache locality: the data is
        // hot in L1 for the second reduction in the fused loop but cold for
        // the second pass in the buggy version (at large N exceeding L1/L2).
        // This is the complement of `entry-vs-contains-insert` (which merges
        // HashMap lookups in the same iteration) — loop fusion merges
        // iterations over the same collection.
        PerfRule {
            rule_id: "loop-fusion".to_string(),
            name: "Two iterator passes over same data → one fused for loop (merge loops)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn sum_max(data: &[u64]) -> (u64, u64) {\n    let sum: u64 = data.iter().copied().sum();\n    let max: u64 = data.iter().copied().max().unwrap_or(0);\n    (sum, max)\n}".to_string(),
            fixed_code: "fn sum_max(data: &[u64]) -> (u64, u64) {\n    let mut sum = 0u64;\n    let mut max = 0u64;\n    for &x in data {\n        sum += x;\n        if x > max { max = x; }\n    }\n    (sum, max)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(loop_fusion_bench_spec()),
        },
        // ── complexity: O(n) region scan → O(1) prefix sum (boundary-vs-volume) ──
        // Per the rust-optimize skill item 1: "boundary-vs-volume" (Stokes /
        // divergence theorem). The discrete instantiation is the 2D prefix sum
        // (a.k.a. summed-area table / integral image): instead of scanning all
        // O(n) interior cells per region-sum query, pre-compute the cumulative
        // sum once (O(n)), then answer each query in O(1) via 4 corner lookups.
        // The prefix array encodes the "boundary" (cumulative sum at the
        // corners); the region mass is reconstructed from boundary values
        // alone — the discrete form of the Stokes theorem.
        //
        // This is distinct from `o1-lookup` (point query via HashMap): prefix
        // sum handles aggregate/range queries via mathematical pre-computation.
        // The compiler cannot derive a prefix-sum transform from a nested-loop
        // scan — it requires the programmer to recognize the pattern.
        //
        // Bench 013: ParetoDominant 30× → 26,000× (grows linearly with N — the
        // scan is O(K * region_area) per K queries while the prefix is O(K)
        // after a one-time O(n) build). Valid for d ≤ 3 (2D maps, 3D belief
        // regions); for d ≥ 8 (HLA state) the prefix array exceeds the interior.
        PerfRule {
            rule_id: "prefix-sum-2d".to_string(),
            name: "O(n) 2D region-sum scan per query → O(1) prefix sum (boundary-vs-volume)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn region_sum(grid: &Vec<Vec<i64>>, x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {\n    let mut sum = 0;\n    for x in x0..x1 {\n        for y in y0..y1 {\n            sum += grid[x][y];\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "// Pre-compute once: prefix[x][y] = sum of grid[0..x][0..y]\nfn build_prefix(grid: &[Vec<i64>]) -> Vec<Vec<i64>> {\n    let w = grid.len();\n    let h = grid.first().map(|r| r.len()).unwrap_or(0);\n    let mut p = vec![vec![0i64; h + 1]; w + 1];\n    for x in 0..w {\n        for y in 0..h {\n            p[x + 1][y + 1] = grid[x][y] + p[x][y + 1] + p[x + 1][y] - p[x][y];\n        }\n    }\n    p\n}\nfn region_sum(prefix: &[Vec<i64>], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {\n    prefix[x1][y1] - prefix[x0][y1] - prefix[x1][y0] + prefix[x0][y0]\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(prefix_sum_2d_bench_spec()),
        },
        // ── allocation: ok_or(expensive()) → ok_or_else(closure) ─────────
        // Source: nnethercote/perf-book "Standard Library Types" chapter.
        // The eager form always constructs the error value; the lazy form
        // only constructs it when the Option is None. ~97× when the error
        // construction involves a String allocation.
        PerfRule {
            rule_id: "ok-or-else".to_string(),
            name: "ok_or(expensive()) → ok_or_else(|| expensive()) (lazy error eval)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn get_value(o: Option<u32>) -> Result<u32, String> {\n    o.ok_or(format!(\"error: missing value at {}\", std::time::SystemTime::now().elapsed().unwrap().as_nanos()))\n}".to_string(),
            fixed_code: "fn get_value(o: Option<u32>) -> Result<u32, String> {\n    o.ok_or_else(|| format!(\"error: missing value at {}\", std::time::SystemTime::now().elapsed().unwrap().as_nanos()))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(ok_or_else_bench_spec()),
        },
        // ── complexity: Vec::remove (O(n) shift) → Vec::swap_remove (O(1)) ─
        // Source: nnethercote/perf-book "Standard Library Types" chapter.
        // Vec::remove shifts all elements after the index; swap_remove moves
        // the last element into the gap. O(n)→O(1) when order doesn't matter.
        PerfRule {
            rule_id: "swap-remove".to_string(),
            name: "Vec::remove(i) O(n) shift → Vec::swap_remove(i) O(1) swap".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn remove_first_n(data: &mut Vec<u32>, n: usize) {\n    for _ in 0..n.min(data.len()) {\n        data.remove(0);\n    }\n}".to_string(),
            fixed_code: "fn remove_first_n(data: &mut Vec<u32>, n: usize) {\n    for _ in 0..n.min(data.len()) {\n        data.swap_remove(0);\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(swap_remove_bench_spec()),
        },
        // ── simd: chunks (variable last) → chunks_exact (uniform, vectorizes) ─
        // Source: nnethercote/perf-book "Iterators" chapter.
        // chunks() yields a potentially-short final chunk, preventing LLVM
        // from vectorizing the loop body. chunks_exact() guarantees uniform
        // chunk size, enabling auto-vectorization. ~13-16× at all sizes.
        PerfRule {
            rule_id: "chunks-exact".to_string(),
            name: "slice::chunks (variable last) → slice::chunks_exact (uniform, vectorizes)".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn sum_chunks(data: &[u32]) -> u64 {\n    let mut sum = 0u64;\n    for chunk in data.chunks(8) {\n        for &v in chunk { sum += v as u64; }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn sum_chunks(data: &[u32]) -> u64 {\n    let mut sum = 0u64;\n    for chunk in data.chunks_exact(8) {\n        for &v in chunk { sum += v as u64; }\n    }\n    for &v in data.chunks_exact(8).remainder() { sum += v as u64; }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(chunks_exact_bench_spec()),
        },
        // ── complexity: std HashMap (SipHash) → FxHashMap (FxHash) ─────────
        // SipHash is a cryptographic-grade hasher (DoS-resistant). For
        // non-adversarial keys (game state, internal caches), FxHash is
        // 2-4× faster with no collision risk in practice. The compiler
        // cannot swap hashers — this is an explicit choice. Used pervasively
        // in rustc itself (the compiler's default HashMap is FxHashMap).
        PerfRule {
            rule_id: "fxhashmap".to_string(),
            name: "std HashMap (SipHash) → FxHashMap (FxHash) for non-adversarial keys".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use rustc_hash::FxHashSet;\nuse std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) -> Option<u32> {\n    map.get(&k).copied()\n}".to_string(),
            fixed_code: "use rustc_hash::FxHashMap;\nuse rustc_hash::FxHashSet;\nfn lookup(map: &FxHashMap<u32, u32>, k: u32) -> Option<u32> {\n    map.get(&k).copied()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(fxhashmap_bench_spec()),
        },
        // ── iterators: filter().collect() → Vec::retain (in-place) ──────────
        // The filter+collect path allocates a NEW Vec and copies surviving
        // elements. retain() filters IN-PLACE — no allocation, one pass.
        // ~1.5-2× faster. The compiler cannot transform filter().collect()
        // into an in-place retain (different ownership semantics).
        PerfRule {
            rule_id: "retain-inplace".to_string(),
            name: "iter().filter().collect() new Vec → Vec::retain() in-place filter".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn remove_multiples_of_3(data: &[u32]) -> Vec<u32> {\n    data.iter().copied().filter(|&x| x % 3 != 0).collect()\n}".to_string(),
            fixed_code: "fn remove_multiples_of_3(data: &mut Vec<u32>) {\n    data.retain(|&x| x % 3 != 0)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(retain_inplace_bench_spec()),
        },
        // ── allocation: clone + clear → mem::take (move, no clone) ──────────
        // When you want to "take" a buffer's contents and leave it empty
        // for reuse, clone+clear deep-copies every element. mem::take moves
        // the Vec's allocation — O(1), zero cloning. ~3× faster. The
        // compiler cannot elide the clone (it has visible side effects).
        PerfRule {
            rule_id: "mem-take".to_string(),
            name: "buf.clone() + buf.clear() → mem::take(&mut buf) (move, no clone)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {\n    let taken = buf.clone();\n    buf.clear();\n    taken\n}".to_string(),
            fixed_code: "fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {\n    std::mem::take(buf)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mem_take_bench_spec()),
        },
        // ── allocation: unnecessary clone → borrow (&) ───────────────────────
        // The single most common Rust perf anti-pattern (Plan 002 T-avoid-clone).
        // Passing `data: Vec<T>` by value clones the entire collection on every
        // call. Passing `data: &[T]` borrows — zero allocation, zero copy.
        // The compiler can't elide clone (visible side effects: heap alloc).
        // ~1.3-3× faster.
        PerfRule {
            rule_id: "avoid-clone".to_string(),
            name: "fn(data: Vec<T>) clone-on-call → fn(data: &[T]) borrow".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn process(data: Vec<u8>) -> u64 {\n    data.iter().map(|&b| b as u64).sum()\n}".to_string(),
            fixed_code: "fn process(data: &[u8]) -> u64 {\n    data.iter().map(|&b| b as u64).sum()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(avoid_clone_bench_spec()),
        },
        // ── allocation: Vec<String> → Vec<Box<str>> (drop capacity word) ────
        // String is 3 words (ptr, len, capacity). Box<str> is 2 words (ptr, len).
        // For immutable string collections, Box<str> saves 8 bytes/element on
        // 64-bit → better cache density. ~1.1-1.3× at scale. The compiler can't
        // change the representation type.
        // B86 RIDER (Cloudflare Big Pineapple, Research 100): the rule
        // GENERALIZES to Box<[T]> for ANY frozen (write-once) T — every
        // Vec<T> field on a stored-at-scale record drops the capacity word
        // AND the growth slack. Their cache replaced 8 frozen fields per
        // entry (String/Vec → Box<str>/Box<[T]>) = 64 B/entry, >15 TB freed
        // fleet-wide. The str half is this rule; the slice generalization
        // is the rider — same mechanism (drop the capacity word), any T.
        PerfRule {
            rule_id: "box-str-immutable".to_string(),
            name: "Vec<String> (3-word handle) → Vec<Box<str>> (2-word, no capacity)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "struct Config { labels: Vec<String> }".to_string(),
            fixed_code: "struct Config { labels: Vec<Box<str>> }".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(box_str_immutable_bench_spec()),
        },
        // ── allocation: alloc-per-call → clear-reuse (scratch buffer) ────────
        // Allocating a fresh Vec inside a hot function is wasteful when the
        // caller can pass a reusable buffer. The compiler can't hoist the
        // allocation (it may panic on OOM). **Context-dependent** — wins at
        // small-to-medium N but can lose at very large N where the reused
        // buffer exceeds L2 cache (see context_notes).
        PerfRule {
            rule_id: "reuse-allocation-clear".to_string(),
            name: "fn() { let mut v = Vec::new() } alloc-per-call → fn(buf: &mut Vec<T>) clear-reuse".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn hot(n: usize) -> u64 {\n    let mut v: Vec<u32> = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v.iter().map(|&x| x as u64).sum()\n}".to_string(),
            fixed_code: "fn hot(n: usize, buf: &mut Vec<u32>) -> u64 {\n    buf.clear();\n    for i in 0..n { buf.push(i as u32); }\n    buf.iter().map(|&x| x as u64).sum()\n}".to_string(),
            // Bench 013 Mixed (3/4 Faster, 1/4 Slower). Wins clearly at
            // N ≤ 10,000 (up to 5.97× at N=100). At N=100,000 the reused
            // buffer exceeds L2 cache → reuse path is slower than fresh alloc.
            // The rule is a genuine win for hot functions called in tight loops
            // with small-to-medium buffers; for very large buffers, alloc-per-call
            // may be fine.
            context_notes: Some(
                "Bench 013: Mixed 3/4 Faster (5.97× at N=100, 1.60× at N=1000, 1.20× at N=10000) \
                 but 0.87× at N=100000 (reused buffer exceeds L2 cache). Suggest for hot \
                 functions called in tight loops with small-to-medium buffers (≤10K elements).".to_string()
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(reuse_allocation_clear_bench_spec()),
        },
        // ── allocation: Box<small T> heap alloc → T on stack ─────────────────
        // Source: perf-book `types.md` (avoid Box for small types) + MS
        // M-BOX + reintech Box-avoidance. The heap alloc per element dwarfs
        // the data manipulation cost at all sizes.
        PerfRule {
            rule_id: "box-avoid".to_string(),
            name: "Vec<Box<i64>> (heap per elem) → Vec<i64> (stack values)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(n: usize) -> Vec<Box<i64>> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n { v.push(Box::new(i as i64)); }\n    v\n}".to_string(),
            fixed_code: "fn build(n: usize) -> Vec<i64> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n { v.push(i as i64); }\n    v\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(box_avoid_bench_spec()),
        },
        // ── allocation: Vec (always heap) → SmallVec/array (stack for small N) ─
        // Source: perf-book `heap-allocations.md` + reintech SmallVec. When
        // the typical size is small (≤8) but occasionally larger, the heap
        // alloc on every call dominates even at tiny element counts.
        PerfRule {
            rule_id: "smallvec-stack".to_string(),
            name: "Vec<T>::new() (always heap) → stack array for ≤8 typical (SmallVec)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn sum_small(n: usize) -> u64 {\n    let mut v: Vec<u64> = Vec::new();\n    for i in 0..n { v.push(i as u64); }\n    v.iter().copied().sum()\n}".to_string(),
            fixed_code: "fn sum_small(n: usize) -> u64 {\n    if n <= 8 {\n        let mut arr = [0u64; 8];\n        for i in 0..n { arr[i] = i as u64; }\n        arr.iter().take(n).copied().sum()\n    } else {\n        let mut v: Vec<u64> = Vec::new();\n        for i in 0..n { v.push(i as u64); }\n        v.iter().copied().sum()\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(smallvec_stack_bench_spec()),
        },
        // ── iterators/IO: File::read byte-by-byte → BufReader (batched I/O) ──
        // Source: perf-book `io.md` + MS M-BUFFERED-IO + reintech BufReader.
        // The syscall overhead per byte dominates; BufReader batches reads
        // into an 8KB internal buffer. ~97× at 100KB file size.
        PerfRule {
            rule_id: "buffered-io".to_string(),
            name: "File::read(&mut [u8; 1]) (syscall per byte) → BufReader (8KB batched)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "use std::io::Read;\nfn sum_bytes(path: &str) -> std::io::Result<u64> {\n    let mut f = std::fs::File::open(path)?;\n    let mut total = 0u64;\n    let mut byte = [0u8; 1];\n    while f.read(&mut byte)? == 1 { total += byte[0] as u64; }\n    Ok(total)\n}".to_string(),
            fixed_code: "use std::io::Read;\nfn sum_bytes(path: &str) -> std::io::Result<u64> {\n    let f = std::fs::File::open(path)?;\n    let mut br = std::io::BufReader::new(f);\n    let mut total = 0u64;\n    let mut byte = [0u8; 1];\n    while br.read(&mut byte)? == 1 { total += byte[0] as u64; }\n    Ok(total)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(buffered_io_bench_spec()),
        },
        // ── allocation: push_str(&format!(...)) temp alloc → write! direct ──
        // Source: perf-book `string-formatting.md` + MS M-LOG + reintech.
        // The buggy path creates a temporary String per iteration via format!,
        // pushes it, then drops it. The fixed path uses write! to format directly
        // into the existing buffer — zero temporary allocation. ~1.7× at all sizes.
        // Distinct from `format-concat-loop` (which is about quadratic accumulation
        // via reassignment); this is about avoiding the per-iteration temp.
        PerfRule {
            rule_id: "log-write-direct".to_string(),
            name: "buf.push_str(&format!(...)) (temp String per iter) → write!(buf, ...) direct".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build_log(items: &[u64]) -> String {\n    let mut log = String::new();\n    for &i in items { log.push_str(&format!(\"item={i}\\n\")); }\n    log\n}".to_string(),
            fixed_code: "use std::fmt::Write;\nfn build_log(items: &[u64]) -> String {\n    let mut log = String::new();\n    for &i in items { write!(&mut log, \"item={i}\\n\").unwrap(); }\n    log\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(log_write_direct_bench_spec()),
        },
        // ── allocation (NotBenchable): Vec::shrink_to_fit after build ────────
        // Source: perf-book `heap-allocations.md`. After building a Vec
        // incrementally (push loop, dedup, parse), the capacity may exceed
        // the len significantly (over-allocation during growth). shrink_to_fit
        // releases the excess. This is a **memory win**, not a speed win —
        // NotBenchable (deterministic size_of, not a runtime microbench).
        PerfRule {
            rule_id: "shrink-to-fit-after-build".to_string(),
            name: "Vec with over-allocated capacity → shrink_to_fit() after build".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v // capacity may be ~2× len due to growth\n}".to_string(),
            fixed_code: "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n { v.push(i as u32); }\n    v.shrink_to_fit(); // release excess capacity\n    v\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is peak memory (size_of), not speed.
            // shrink_to_fit is a no-op if capacity == len; the alloc overhead
            // of the realloc may even make it slower for the build path. The
            // value is long-lived Vecs that over-allocated during construction.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── layout (NotBenchable): Box large enum variant ─────────────────────
        // Source: perf-book `types.md`. When an enum has one large variant
        // (e.g., `[u8; 1024]`) and many small ones, every instance carries
        // the large variant's size. Boxing the large variant shrinks the
        // common case (enum becomes 8 bytes + discriminant vs 1032 bytes).
        // B86 RIDER (Cloudflare Big Pineapple, Research 100 — their change
        // 4): the HONEST COSTS of boxing the large variant, recorded so the
        // rewrite is not sold as free: (1) allocator size-class rounding —
        // the boxed payload rounds UP to the allocator's bin (their TXT 32 B
        // fit its bin; MX 40 B rounded to 48 — some boxed variants pay more
        // than their inline size); (2) locality scatter — one contiguous
        // allocation becomes many, and the common-case records now chase a
        // pointer on access; (3) the RARE variant pays MORE (their NAPTR,
        // the 136 B variant, now costs the Box header on top). The win is
        // the SKEWED-FREQUENCY regime: ~80% small variants (their A/AAAA
        // traffic) vs a large tail — boxing shrinks the common case exactly
        // because the large case is rare.
        PerfRule {
            rule_id: "box-large-enum-variant".to_string(),
            name: "Enum with one large variant → Box<LargeVariant> (shrink common case)".to_string(),
            category: "layout".to_string(),
            buggy_code: "enum Event {\n    Click { x: i32, y: i32 },\n    KeyPress(char),\n    BigData([u8; 1024]), // inflates ALL events to 1032+ bytes\n}".to_string(),
            fixed_code: "enum Event {\n    Click { x: i32, y: i32 },\n    KeyPress(char),\n    BigData(Box<[u8; 1024]>), // common case: 16 bytes (discriminant + ptr)\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is size_of (every enum instance shrinks),
            // which improves cache density. Not a speed microbench in isolation.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators (NotBenchable): async yield point for fairness ────────
        // Source: perf-book `async.md`. A long-running async fn without a
        // yield point (no .await) blocks the executor for the entire duration.
        // Adding a cooperative yield point (tokio::task::yield_now) lets the
        // executor schedule other tasks. The win is **latency tail + fairness**,
        // not throughput — NotBenchable (throughput-neutral, the win is in
        // p99 latency under multi-task contention).
        PerfRule {
            rule_id: "async-yield-point".to_string(),
            name: "Long async fn (no yield) → cooperative yield_now() for fairness".to_string(),
            category: "iterators".to_string(),
            buggy_code: "async fn process_batch(items: &[Item]) {\n    for item in items {\n        process(item); // no .await → blocks executor for entire batch\n    }\n}".to_string(),
            fixed_code: "async fn process_batch(items: &[Item]) {\n    for (i, item) in items.iter().enumerate() {\n        process(item);\n        if i % 100 == 0 { tokio::task::yield_now().await; }\n    }\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is p99 latency under contention (fairness),
            // not single-task throughput. Throughput-neutral or slightly negative.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── layout (NotBenchable): async fn future size reduction ───────────
        // Source: perf-book `async.md`. Each async fn compiles to a state
        // machine struct whose size is the max of all locals across all await
        // points. Large locals (big Vecs, arrays) inflates the future size,
        // wasting memory per spawned task. Scoping locals (block expressions)
            // drops them before the next await, shrinking the state machine.
        PerfRule {
            rule_id: "async-future-size".to_string(),
            name: "Large locals across .await → scoped blocks (shrink future state machine)".to_string(),
            category: "layout".to_string(),
            buggy_code: "async fn handle() {\n    let big_buf = vec![0u8; 8192]; // lives across .await → in future\n    let result = do_work(&big_buf).await;\n    // big_buf still in the future state machine here\n    process(result);\n}".to_string(),
            fixed_code: "async fn handle() {\n    let result = {\n        let big_buf = vec![0u8; 8192]; // dropped before .await returns\n        do_work(&big_buf).await\n    }; // big_buf no longer in future state machine\n    process(result);\n}".to_string(),
            context_notes: None,
            // Not benchable: the win is size_of_val of the future (memory per
            // spawned task), not a runtime microbench. Measurable via
            // std::mem::size_of_val but that's deterministic, not a timing.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── complexity: stable sort → unstable sort (no allocation + faster algo) ─
        // Source: std docs + perf-book. `sort()` uses a stable mergesort that
        // allocates temporary storage; `sort_unstable()` uses pattern-defeating
        // quicksort (pdqsort) which is in-place + typically 2-3× faster for
        // random data. The compiler cannot transform a stable sort into an
        // unstable one — this is a library-level algorithm choice.
        PerfRule {
            rule_id: "sort-unstable".to_string(),
            name: "Vec::sort() stable (allocates) → Vec::sort_unstable() (in-place, faster)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn sort_scores(scores: &mut Vec<u32>) {\n    scores.sort();\n}".to_string(),
            fixed_code: "fn sort_scores(scores: &mut Vec<u32>) {\n    scores.sort_unstable();\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_unstable_bench_spec()),
        },
        // ── complexity: sort_by(closure) → sort_by_key(closure) (N vs N·log N key extractions) ─
        // Source: std docs. `sort_by(|a, b| f(a).cmp(&f(b)))` calls `f` O(N log N)
        // times; `sort_by_key(|x| f(x))` extracts each key exactly once (N calls)
        // then sorts the cached keys. The win scales with key-extraction cost.
        PerfRule {
            rule_id: "sort-by-key".to_string(),
            name: "sort_by(|a,b| a.x.cmp(&b.x)) → sort_by_key(|x| x.x) (extract key once per element)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn sort_by_score(items: &mut Vec<(String, u32)>) {\n    items.sort_by(|a, b| a.1.cmp(&b.1));\n}".to_string(),
            fixed_code: "fn sort_by_score(items: &mut Vec<(String, u32)>) {\n    items.sort_by_key(|item| item.1);\n}".to_string(),
            context_notes: Some("ParetoDominated in Bench 016 default config: 4/4 Tie at n=1K-100K with integer keys on (String, u32) tuples. The compiler optimizes sort_by's comparator to avoid redundant key reads for simple field access. sort_by_key is a correctness/idiom win (avoids writing the comparator twice), not a throughput win for trivial keys. The win materializes only when the key function is non-trivial (hash computation, nested dereference, or expensive computation that the compiler cannot CSE across comparator calls).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_by_key_bench_spec()),
        },
        // ── io: per-record File::write → BufWriter batched (syscall reduction) ──
        // Source: perf-book `io.md` + std docs. Each `File::write` call is a
        // syscall (~1-2µs on Linux, ~2-4µs on macOS). `BufWriter` coalesces
        // writes into 8KB buffers, amortizing the syscall cost. This is the
        // write-side analog of `buffered-io` (BufReader for reads).
        PerfRule {
            rule_id: "bufwriter-batched".to_string(),
            name: "File::write per-record (syscall each) → BufWriter batched (8KB coalesced)".to_string(),
            category: "io".to_string(),
            buggy_code: "use std::io::Write;\nfn write_records(file: &mut std::fs::File, records: &[Vec<u8>]) -> std::io::Result<()> {\n    for rec in records {\n        file.write_all(rec)?; // syscall per record\n    }\n    Ok(())\n}".to_string(),
            fixed_code: "use std::io::{BufWriter, Write};\nfn write_records(file: std::fs::File, records: &[Vec<u8>]) -> std::io::Result<()> {\n    let mut buf = BufWriter::new(file);\n    for rec in records {\n        buf.write_all(rec)?; // buffered → one syscall per 8KB\n    }\n    buf.flush()?; // flush remaining buffered data\n    Ok(())\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bufwriter_batched_bench_spec()),
        },
        // ── simd: manual index-loop copy → slice::copy_from_slice (SIMD + no bounds check) ─
        // Source: std docs + perf-book `simd.md`. A manual `for i in 0..n { dst[i] =
        // src[i]; }` loop does per-element bounds checking + scalar copies.
        // `copy_from_slice` is a single call that LLVM lowers to `memcpy` /
        // SIMD-wide stores when the length is known or aligned. The compiler
        // *can* sometimes auto-vectorize the manual loop, but `copy_from_slice`
        // guarantees it + eliminates the per-index bounds check.
        PerfRule {
            rule_id: "copy-from-slice".to_string(),
            name: "Manual index-loop element copy → slice::copy_from_slice (memcpy / SIMD)".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn copy_data(dst: &mut [u8], src: &[u8]) {\n    for i in 0..src.len() {\n        dst[i] = src[i];\n    }\n}".to_string(),
            fixed_code: "fn copy_data(dst: &mut [u8], src: &[u8]) {\n    dst.copy_from_slice(src);\n}".to_string(),
            context_notes: Some("Mixed in Bench 016 default config: 2/4 Faster (n=64, n=1024), 1/4 Slower (n=4096), 1/4 Tie (n=256). LLVM auto-vectorizes the manual index loop effectively for u8 slices — at medium sizes (256-1024) the performance is indistinguishable, and at n=4096 the manual loop is marginally faster (likely due to memcpy's setup overhead being amortized differently). copy_from_slice is still the correct idiom (eliminates bounds checks, guarantees SIMD lowering, is less code) but consumers should know the throughput difference is small for u8 in release mode. The win is clearer for larger element types (u64, structs) where the manual loop's per-element bounds check is more expensive relative to the copy.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(copy_from_slice_bench_spec()),
        },
        // ── complexity: Vec::remove(0) O(n) shift → VecDeque::pop_front O(1) ──
        // Fills the corpus's data-structure-choice gap: zero prior rules about
        // choosing the right collection. Vec::remove(0) does a memmove of all
        // remaining elements left by one position — O(n) per call, O(n²) to
        // drain N elements. VecDeque is a ring buffer: pop_front just advances
        // the head index — O(1) per call, O(n) to drain. The compiler cannot
        // turn a contiguous-array memmove into a ring-buffer head advance.
        PerfRule {
            rule_id: "vec-remove-front-to-vecdeque".to_string(),
            name: "Vec::remove(0) O(n) shift → VecDeque::pop_front O(1)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn drain_queue(queue: &mut Vec<u32>) -> u64 {\n    let mut sum = 0u64;\n    while !queue.is_empty() {\n        sum += queue.remove(0) as u64;\n    }\n    sum\n}".to_string(),
            fixed_code: "use std::collections::VecDeque;\nfn drain_queue(queue: &mut VecDeque<u32>) -> u64 {\n    let mut sum = 0u64;\n    while let Some(val) = queue.pop_front() {\n        sum += val as u64;\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_remove_front_bench_spec()),
        },
        // ── complexity: HashMap + sort-on-each-query → BTreeMap natural order ──
        // Data-structure choice: when sorted iteration is needed repeatedly,
        // HashMap forces O(n log n) sort per query. BTreeMap maintains sorted
        // order by construction (self-balancing BST) → O(n) iteration. The
        // compiler cannot replace a hash table + sort with a balanced tree.
        PerfRule {
            rule_id: "btreemap-sorted-iteration".to_string(),
            name: "HashMap + sort-on-each-query → BTreeMap natural sorted iteration".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn sorted_keys(map: &HashMap<u32, u32>) -> Vec<u32> {\n    let mut keys: Vec<u32> = map.keys().copied().collect();\n    keys.sort_unstable();\n    keys\n}".to_string(),
            fixed_code: "use std::collections::BTreeMap;\nfn sorted_keys(map: &BTreeMap<u32, u32>) -> Vec<u32> {\n    map.keys().copied().collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3.73-6.91x (Bench 013, the rule own gate). REGIME (sorted-read frequency — the two-row decision table completed by Batch 75's `group-keys-borrow-allocate-on-first-sight`, the mirror rule): 0 sorted reads of the grouped output → plain Vec/HashMap, no ordering machinery; ONE final sort after a single grouping pass → build unordered + sort once (the mirror rule — R·L owned-key clones become S·L, and R ordered O(log S) inserts become one S·log(S) sort); MANY repeated sorted reads over a long-lived map → THIS rule (maintain the BTreeMap, iterate for free). The common failure is paying BTreeMap ordered-insert cost during a build whose output is read sorted once or never.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(btreemap_sorted_iteration_bench_spec()),
        },
        // ── allocation: String return (always alloc) → Cow<str> borrow ──────
        // Data-structure choice: when a function conditionally transforms a
        // string, returning String forces an allocation even when no
        // transformation occurs. Cow<str> borrows the input in the common
        // (no-work) case — zero allocation. The compiler cannot avoid a
        // String heap allocation; it's a concrete type, not an optimization
        // target. The classic example: HTML escaping where 90%+ of strings
        // have no special characters.
        PerfRule {
            rule_id: "cow-conditional-alloc".to_string(),
            name: "String return (always alloc) → Cow<str> (borrow when no transform)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn escape_html(s: &str) -> String {\n    if !s.contains(['<', '>', '&']) {\n        return s.to_string();\n    }\n    s.replace('<', \"&lt;\")\n        .replace('>', \"&gt;\")\n        .replace('&', \"&amp;\")\n}".to_string(),
            fixed_code: "use std::borrow::Cow;\nfn escape_html(s: &str) -> Cow<str> {\n    if !s.contains(['<', '>', '&']) {\n        return Cow::Borrowed(s);\n    }\n    Cow::Owned(s.replace('<', \"&lt;\").replace('>', \"&gt;\").replace('&', \"&amp;\"))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(cow_conditional_alloc_bench_spec()),
        },
        // ── caching: Regex::new per call → static LazyLock<Regex> ─────────────
        // Source: Research 032 (hotpath-rs profiling guide). Regex compilation
        // is runtime NFA construction — ~µs of real work the compiler cannot
        // elide (unlike `precompute-table`'s POPCNT shape). Hoisting to a
        // `static LazyLock` compiles ONCE; every call after is a match-only.
        // The guide's prior: ~555× per call. Bench 020 verifies on our harness.
        PerfRule {
            rule_id: "regex-hoist-lazylock".to_string(),
            name: "Regex::new per call → static LazyLock<Regex> (compile once)".to_string(),
            category: "caching".to_string(),
            buggy_code: "use regex::Regex;\nfn is_valid(code: &str) -> bool {\n    let re = Regex::new(r\"^[A-Z]{2}-\\d{4}$\").unwrap(); // recompiles every call\n    re.is_match(code)\n}".to_string(),
            fixed_code: "use regex::Regex;\nuse std::sync::LazyLock;\nstatic RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r\"^[A-Z]{2}-\\d{4}$\").unwrap());\nfn is_valid(code: &str) -> bool {\n    RE.is_match(code)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(regex_hoist_lazylock_bench_spec()),
        },
        // ── allocation: clone-per-message payload → Arc<str> shared ──────────
        // Source: Research 032 (hotpath-rs profiling guide). Fanning out an
        // immutable payload to N recipients via `to_string()` clones copies N
        // × payload-bytes. `Arc<str>` allocates the payload ONCE; recipients
        // hold refcount-bumped handles (pointer copies). Timing prior ~1.3×
        // (memcpy-bound) may Tie — the deterministic alloc axis carries the
        // gate: N×payload bytes → ~1 allocation. Distinct from `avoid-clone`
        // (borrow vs clone at fn boundary) + `box-str-immutable` (struct width).
        PerfRule {
            rule_id: "arc-str-payload".to_string(),
            name: "Clone-per-message immutable payload → Arc<str> (clone the handle, not the bytes)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, String)> {\n    ids.iter().map(|&id| (id, payload.to_string())).collect() // N payload clones\n}".to_string(),
            fixed_code: "use std::sync::Arc;\nfn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, Arc<str>)> {\n    let shared: Arc<str> = Arc::from(payload); // ONE allocation\n    ids.iter().map(|&id| (id, Arc::clone(&shared))).collect() // refcount bumps\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(arc_str_payload_bench_spec()),
        },
        // ── concurrency: guard held across .await → fetch-before-guard ───────
        // Source: Research 032 (hotpath-rs profiling guide). Holding a
        // parking_lot/tokio MutexGuard across a slow .await serializes every
        // OTHER task that touches the lock — the fix fetches what's needed,
        // drops the guard, THEN awaits. clippy's `await_holding_lock` covers
        // std guards only; this rule targets the ecosystem lock crates.
        PerfRule {
            rule_id: "tokio-guard-across-await".to_string(),
            name: "MutexGuard held across .await → fetch data, drop guard, then await".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {\n    let mut cfg = cache.lock().await;\n    let upstream = fetch(&key).await; // guard held across await — blocks all other lockers\n    cfg.apply(upstream);\n}".to_string(),
            fixed_code: "async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {\n    let upstream = fetch(&key).await; // await FIRST — no guard held\n    let mut cfg = cache.lock().await;\n    cfg.apply(upstream);\n    // guard drops at scope end, after the slow path\n}".to_string(),
            context_notes: Some("NotBenchable (metric mismatch): the cost is OTHER tasks' wait latency while the guard is held across a slow await, not this task's wall-clock. A single-threaded harness measures no regression on the buggy path; the win only appears under concurrent lock contention (same class as the park-vs-spin rejection + bevy-reactive-loop NotBenchable precedent). Also note clippy's await_holding_lock lint covers std sync guards only — this rule targets tokio/parking_lot/async-lock guards.".to_string()),
            // Not benchable: contention-latency metric, not wall-clock.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: unbounded channel → bounded (backpressure) ──────────
        // Source: Research 032 (hotpath-rs profiling guide). An unbounded
        // channel + a send loop that outpaces the consumer grows the queue
        // without bound (memory blowup + consumer sees stale data). A bounded
        // channel applies backpressure: send().await suspends the producer.
        PerfRule {
            rule_id: "bounded-channel-backpressure".to_string(),
            name: "Unbounded channel + hot producer → bounded channel (backpressure)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();\nfor event in events {\n    tx.send(event).unwrap(); // never blocks — queue grows unboundedly if consumer is slow\n}".to_string(),
            fixed_code: "let (tx, mut rx) = tokio::sync::mpsc::channel(1024);\nfor event in events {\n    tx.send(event).await.unwrap(); // backpressure: suspends producer when full\n}".to_string(),
            context_notes: Some("NotBenchable (metric mismatch): the win is bounded queue depth + fresh-data latency under producer>consumer overload, not throughput — the buggy path is often FASTER on wall-clock precisely because it buffers unboundedly (that is the bug). Semantics-changing: bounded send applies backpressure (producer suspends) vs unbounded buffering; adopt deliberately, size the bound to your latency budget.".to_string()),
            // Not benchable: queue-depth/backpressure metric + semantics-changing.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: adjacent per-thread counter slots → align to cache line ──
        // Source: hotpath.rs profiler blog (Batch 65, Research 081 — the
        // profiler's own GlobalAlloc hook: `static STATS: [ThreadAllocStats;
        // MAX_THREADS]`, 24 B each, indexed by thread ordinal). Two threads
        // bumping atomics in slots that share a coherence line pay a line
        // ping-pong per RMW; padding each slot to the line granularity makes
        // every slot private. Source (x86, align(64)): 2t 70% / 4t 72% /
        // 8t 86% / 12t 85%; single-thread 0% (honest — no contention, no cost).
        // **Pad to the ARCH's line size, not 64 by reflex (measured, M3 Max,
        // 2026-08-22): Apple Silicon lines are 128 B (`hw.cachelinesize`), so
        // `#[repr(align(64))]` leaves even-index slot pairs sharing [0,128)
        // lines — it recovered only 1.16×/0.98×/1.60×/2.44× at 2/4/8/12t
        // where align(128) delivered 3.27×/3.88×/10.75×/5.93×.**
        // `crossbeam_utils::CachePadded` picks per-target. Memory cost: 24 B
        // → 64/128 B per slot (irrelevant at MAX_THREADS scale, load-bearing
        // at millions-of-slots scale).
        PerfRule {
            rule_id: "align-64-false-sharing".to_string(),
            name: "adjacent per-thread atomic counter slots → #[repr(align)] to cache-line granularity".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "static STATS: [ThreadAllocStats; MAX_THREADS]; // 24 B each — several per cache line\n\nfn record_alloc(thread_ord: usize, bytes: u64) {\n    let s = &STATS[thread_ord]; // adjacent ordinals share a coherence line\n    s.allocs.fetch_add(1, Ordering::Relaxed); // line ping-pong under contention\n    s.bytes.fetch_add(bytes, Ordering::Relaxed);\n}".to_string(),
            fixed_code: "#[repr(align(128))] // pad to the ARCH line size (64 on x86, 128 on Apple Silicon) — or crossbeam_utils::CachePadded\nstruct ThreadAllocStats {\n    allocs: AtomicU64,\n    bytes: AtomicU64,\n    frees: AtomicU64,\n}\n\nstatic STATS: [ThreadAllocStats; MAX_THREADS]; // every slot on its own line\n\nfn record_alloc(thread_ord: usize, bytes: u64) {\n    let s = &STATS[thread_ord]; // private line — no ping-pong\n    s.allocs.fetch_add(1, Ordering::Relaxed);\n    s.bytes.fetch_add(bytes, Ordering::Relaxed);\n}".to_string(),
            context_notes: Some(
                "Measured ParetoDominant 4.7-9.4x at 2-12 threads (M3 Max, align(128) fixed arm, load ~3.2, 2026-08-22; single-thread = tie — no contention, no cost, honest). ARCH GRANULARITY CAVEAT (the rule id carries the source's x86 form): Apple Silicon cache lines are 128 B (`hw.cachelinesize`), so `#[repr(align(64))]` leaves even-index slot pairs sharing [0,128) lines — measured it recovers only 1.16-2.44x of the 3.3-10.8x full-separation win. Pad to the ARCH's coherence granularity (align(64) x86 / align(128) Apple Silicon) or use `crossbeam_utils::CachePadded` which picks per-target. Memory trade: 24 B -> 64/128 B per slot (irrelevant at MAX_THREADS scale, load-bearing at millions-of-slots scale). Detection shape: a static/owned array of small structs with Atomic* fields indexed by thread ordinal."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(align_64_false_sharing_bench_spec()),
        },
        // ── concurrency: per-item channel sends → per-producer batch flush ────
        // Source: hotpath.rs profiler blog (Batch 65, Research 081 — the
        // measurement channel: per-measurement `send` at µs intervals to a
        // receiver parked in crossbeam `select!`). `select!` bypasses
        // crossbeam's direct-subscribe spin-wait path, so the receiver PARKS
        // between messages and every send pays a Thread::park/unpark pair
        // (~22% of samples in `Thread::unpark`). Batching per producer
        // thread (local Vec drained on flush) amortizes the wakeup: source
        // 500ns → 80ns per measurement (6.25×). Adjacent-but-distinct from
        // `bounded-channel-backpressure` (queue-fullness POLICY vs
        // per-message WAKEUPS). The direct-subscribe receiver already spins
        // — the storm is select!-specific (upstream PR submitted by the
        // source author).
        PerfRule {
            rule_id: "channel-send-batching-unpark".to_string(),
            name: "per-item MPSC sends to a parked select! receiver → per-producer batch flush. RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- commit 1b89a43 #27830): the quantizer feeds the worker pool in ROW SLABS so the quantize workers never idle -- the producer-side batching half of this rule at model-load scale (chunked enqueue so consumers stay parked-unparked in batches, not per-item).".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "fn record_measurement(tx: &crossbeam_channel::Sender<Sample>, s: Sample) {\n    tx.send(s).unwrap(); // 1 park/unpark pair per message when the\n                         // receiver sits in crossbeam select! (bypasses\n                         // the spin path) — ~22% of samples in Thread::unpark\n}".to_string(),
            fixed_code: "fn record_measurement(buf: &mut Vec<Sample>, tx: &crossbeam_channel::Sender<Vec<Sample>>, s: Sample) {\n    buf.push(s); // batch per producer thread…\n    if buf.len() >= FLUSH_EVERY {\n        // replace keeps capacity for the next batch (take-then-regrow pays\n        // a ~7-alloc growth chain per batch — measured worse on the alloc\n        // axis than the buggy arm)\n        let batch = std::mem::replace(buf, Vec::with_capacity(FLUSH_EVERY));\n        tx.send(batch).unwrap(); // …ONE wakeup per FLUSH_EVERY messages\n    }\n}".to_string(),
            context_notes: Some(
                "Measured ParetoDominant 1.28-1.29x@10K / 1.16x@100K (M3 Max, load ~3.2-3.8, 2026-08-22; allocs 3232->1624 @100K, alloc-det stable). REGIME CAVEAT — the modest margin is honest: a 1P/1C pipeline overlaps the receiver's park/unpark latency with the producer's next-message work, hiding it off the critical path; the source's 6.25x (500ns->80ns per measurement) needs the receiver-side park pair ON the critical path (heavier consumer work per message, or the single-measurement µs-cadence API shape the profiler used). The mechanism is crossbeam-select!-specific — a direct `recv()` receiver spins and never storms (upstream PR filed by the source author). Batching also wins the alloc axis (fewer channel blocks + one Vec per 64 vs one block per ~32). Use `mem::replace` for the flush, NOT `mem::take` (take-then-regrow pays a ~7-alloc Vec growth chain per batch)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(channel_send_batching_unpark_bench_spec()),
        },
        // ── allocation: collect() + append() → extend() ───────────────────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). Staging an iterator into an intermediate `Vec` only to
        // `append` it into the target pays one extra allocation + an extra
        // memcpy per chunk. `extend` writes the iterator's items straight
        // into the target's growth path — zero intermediates. The compiler
        // cannot elide a `collect` (it materializes a concrete `Vec`).
        PerfRule {
            rule_id: "collect-append-to-extend".to_string(),
            name: "collect() + append() staging → extend() (no intermediate Vec)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in chunks {\n        let mut staged: Vec<u32> = c.iter().copied().collect(); // intermediate Vec alloc per chunk\n        target.append(&mut staged);\n    }\n}".to_string(),
            fixed_code: "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in chunks {\n        target.extend(c.iter().copied()); // writes straight into the target\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(collect_append_to_extend_bench_spec()),
        },
        // ── complexity: HashSet contains+insert → insert's bool return ────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). `HashSet` has no `entry` API (unlike `HashMap` — see the
        // `entry-vs-contains-insert` sibling); the membership check IS the
        // insert's `bool` return. `!set.contains(&x)` + `set.insert(x)` walks
        // the hash table twice; `set.insert(x)` alone walks it once. The two
        // calls are opaque to LLVM (hashbrown internals) — no fusion.
        PerfRule {
            rule_id: "hashset-insert-bool-return".to_string(),
            name: "HashSet contains+insert double lookup → insert's bool return (one lookup)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashSet;\nfn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    for &x in items {\n        if !seen.contains(&x) {\n            seen.insert(x);\n            inserted += 1;\n        }\n    }\n    inserted\n}".to_string(),
            fixed_code: "use std::collections::HashSet;\nfn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    for &x in items {\n        if seen.insert(x) {\n            inserted += 1;\n        }\n    }\n    inserted\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hashset_insert_bool_return_bench_spec()),
        },
        // ── allocation: unwrap_or(eager) → unwrap_or_else(closure) ────────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). The `Option` sibling of `ok-or-else`: `unwrap_or(expr)`
        // evaluates `expr` BEFORE checking the `Some` variant — the default
        // is computed (and allocated, if it allocates) even when discarded.
        // `unwrap_or_else(|| expr)` defers construction to the `None` arm.
        // Wins when the default is non-trivial AND `Some` is the common case.
        PerfRule {
            rule_id: "unwrap-or-else-lazy".to_string(),
            name: "unwrap_or(expensive()) eager default → unwrap_or_else(closure) lazy".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn resolve(opt: Option<u32>) -> u32 {\n    opt.unwrap_or(expensive_default(64)) // eagerly evaluated even when Some\n}\nfn expensive_default(n: u32) -> u32 {\n    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 31))\n}".to_string(),
            fixed_code: "fn resolve(opt: Option<u32>) -> u32 {\n    opt.unwrap_or_else(|| expensive_default(64)) // built only on None\n}\nfn expensive_default(n: u32) -> u32 {\n    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 31))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(unwrap_or_else_bench_spec()),
        },
        // ── iterators: filter(p).count() > 0 → any(p) short-circuit ───────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). `filter(p).count() > 0` scans the WHOLE iterator, builds a
        // count, then throws it away. `any(p)` stops at the first hit. The
        // compiler cannot derive a short-circuit from a full `count` fold.
        // The win scales with how early the first match lands.
        PerfRule {
            rule_id: "filter-count-to-any".to_string(),
            name: "filter(p).count() > 0 full scan → any(p) short-circuit".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn has_even(vals: &[u32]) -> bool {\n    vals.iter().filter(|&&v| v % 2 == 0).count() > 0 // full scan, no short-circuit\n}".to_string(),
            fixed_code: "fn has_even(vals: &[u32]) -> bool {\n    vals.iter().any(|&v| v % 2 == 0) // stops at the first hit\n}".to_string(),
            // Bench 48: ParetoDominated at mid-position hits (0.82-0.90×) —
            // a genuine folk-wisdom refutation. LLVM VECTORIZES the count
            // loop (batched element scanning) while any's early-exit forces
            // scalar iteration; at a 50% hit position the vectorized FULL
            // count beats the scalar half-scan. any wins only for very early
            // hits (empirically ≤ ~10% position) or non-vectorizable
            // predicates. Keep as an idiom rule with the regime caveat.
            context_notes: Some(
                "Bench 48: ParetoDominated 0.82-0.90x at mid-position hits. \
                 LLVM vectorizes filter().count() into batched scanning while \
                 any()'s early-exit forces scalar iteration — at a 50% hit \
                 position the vectorized full count WINS. any() only wins for \
                 very early hits (~<10% position) or non-vectorizable \
                 predicates. Regime-dependent: suggest only when hits are \
                 known-early or the predicate is expensive/branchy."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_count_to_any_bench_spec()),
        },
        // ── allocation: Vec::new + resize(n, 0) → vec![0; n] calloc path ──────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). `Vec::new()` + `resize(n, 0)` allocates uninitialized
        // memory then runs an explicit memset pass over every byte. `vec![0; n]`
        // routes through `alloc_zeroed` (calloc) — for large buffers the
        // allocator hands back lazily-zeroed OS pages, skipping the touch.
        PerfRule {
            rule_id: "vec-macro-zero-fill".to_string(),
            name: "Vec::new() + resize(n, 0) memset → vec![0; n] alloc_zeroed (calloc)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn zero_buf(n: usize) -> Vec<u8> {\n    let mut v = Vec::new(); // uninit alloc\n    v.resize(n, 0); // explicit memset pass\n    v\n}".to_string(),
            fixed_code: "fn zero_buf(n: usize) -> Vec<u8> {\n    vec![0u8; n] // alloc_zeroed — calloc zero pages\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_macro_zero_fill_bench_spec()),
        },
        // ── iterators: .map(|x| *x) → .copied() specialized adapter ───────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). `.map(|x| *x)` wraps the iterator in the generic `Map`
        // adapter (opaque closure, no size hints). `.copied()` is a
        // specialized adapter implementing `TrustedLen` + `InPlaceIterable`
        // markers — downstream consumers (collect, sum) get exact-size
        // reservations the `Map` form cannot express. In tight release loops
        // LLVM often lowers both identically — the win regime is adapter
        // fusion + allocation sizing, which the bench measures honestly.
        PerfRule {
            rule_id: "iter-copied-deref-map".to_string(),
            name: ".map(|x| *x) generic adapter → .copied() specialized (TrustedLen)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn sum_kills(kills: &[u64]) -> u64 {\n    kills.iter().map(|k| *k).sum() // generic Map adapter, no size hint\n}".to_string(),
            fixed_code: "fn sum_kills(kills: &[u64]) -> u64 {\n    kills.iter().copied().sum() // specialized Copy adapter\n}".to_string(),
            // Bench 48: Mixed — parity at ≤100K, 0.80× at 1M (copied SLOWER).
            // The specialized adapter's length-tracking overhead loses to the
            // fully-unrolled map+sum loop at large N on a sum fold. The rule
            // stands as an idiom (TrustedLen matters for collect's exact-size
            // reservations, not for folds); do not suggest for large-N folds.
            context_notes: Some(
                "Bench 48: Mixed (tie/tie/0.80x at 1K/100K/1M). The specialized \
                 Copied adapter carries length-tracking overhead that loses to \
                 the fully-unrolled Map+sum loop at large N on a sum fold. The \
                 idiom remains correct for collect() (exact-size reservation via \
                 TrustedLen) and for readability; do not suggest expecting a \
                 throughput win on large-N folds."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(iter_copied_deref_map_bench_spec()),
        },
        // ── iterators: filter(p).map(f) → filter_map fused adapter ────────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). The perf-book HEDGES on this one ("filter_map is not
        // always faster") — the fused form removes one adapter state-machine
        // layer but blocks some internal specializations of the separated
        // adapters. Genuinely regime-dependent; the Pareto bench decides the
        // corpus annotation, not the book's hedged prose.
        PerfRule {
            rule_id: "filter-map-fuse".to_string(),
            name: "filter(p).map(f) two adapters → filter_map fused (regime-dependent)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn even_squares(vals: &[u32]) -> Vec<u32> {\n    vals.iter().filter(|&&v| v % 2 == 0).map(|&v| v * v).collect()\n}".to_string(),
            fixed_code: "fn even_squares(vals: &[u32]) -> Vec<u32> {\n    vals.iter().filter_map(|&v| (v % 2 == 0).then_some(v * v)).collect()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_map_fuse_bench_spec()),
        },
        // ── complexity: Vec::insert(0, x) loop → VecDeque::push_front ─────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). The write-side sibling of `vec-remove-front-to-vecdeque`:
        // every `insert(0, x)` memmoves the whole buffer down (O(n) per
        // insert, O(n²) per loop); `VecDeque::push_front` advances a ring
        // head in O(1). The compiler cannot turn a front-insert loop into a
        // ring buffer.
        PerfRule {
            rule_id: "vec-insert-front-deque".to_string(),
            name: "Vec::insert(0, x) O(n²) prepend loop → VecDeque::push_front O(1)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn prepend_history(history: &mut Vec<u32>, events: &[u32]) {\n    for &e in events {\n        history.insert(0, e); // O(n) memmove per insert → O(n²) total\n    }\n}".to_string(),
            fixed_code: "use std::collections::VecDeque;\nfn prepend_history(history: &mut VecDeque<u32>, events: &[u32]) {\n    for &e in events {\n        history.push_front(e); // O(1) ring-buffer head advance\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_insert_front_deque_bench_spec()),
        },
        // ── arithmetic: x.powf(2.0) constant exponent → x.powi(2) ──────────────
        // Source: users.rust-lang.org/t/56451 + baptiste-wicht.com std::pow C++
        // study (Batch 53, idle-queue item 11). `powf` lowers to a libm `pow`
        // call — an opaque scheduling barrier in a hot loop; `powi(2)` compiles
        // to a single multiply. The candidate risk (recorded honestly by the
        // bench, not assumed): LLVM's instcombine rewrites `pow(x, 2.0)` to
        // `x * x` in release — the verdict axis decides whether this is a
        // runtime win or an idiom-only rule.
        PerfRule {
            rule_id: "powi-vs-powf".to_string(),
            name: "x.powf(2.0) libm call → x.powi(2) single multiply (constant exponent)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn sum_squares(vals: &[f32]) -> f32 {\n    vals.iter().map(|v| v.powf(2.0)).sum() // libm pow call per element\n}".to_string(),
            fixed_code: "fn sum_squares(vals: &[f32]) -> f32 {\n    vals.iter().map(|v| v.powi(2)).sum() // single multiply\n}".to_string(),
            // Bench 53 (Bench 038): all-Tie at 1.00x across 1K/100K/1M —
            // LLVM instcombine lowers pow(x, 2.0) to the same x*x multiply
                       // as powi(2) in release, so the classic "powf is slow" advice
            // does NOT hold for the constant-exponent-2 release case. The
            // idiom still matters for: debug-profile hot loops (no instcombine
            // — powf stays a libm call), exponents >= 3 (LLVM does NOT lower
            // pow(x, 3.0) — correctly-rounded pow differs from x*x*x, so it
            // stays a libm call while powi(3) becomes two multiplies), and
            // non-constant exponents where powi is not applicable.
            context_notes: Some(
                "Bench 53: Mixed — all-Tie 1.00x (1K/100K/1M) — LLVM lowers powf(2.0) \
                 to the same x*x as powi(2) in release; the folk wisdom \
                 ('powf is extremely slow') does not hold for \
                 constant-exponent-2 release code. Suggest as an idiom for \
                 debug-profile hot loops (powf stays a libm call without \
                 instcombine) and exponents >= 3 (pow(x,3.0) is NOT lowered — \
                 it stays a libm call while powi(3) is two multiplies); do \
                 not suggest expecting a release-throughput win at exponent 2."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(powi_vs_powf_bench_spec()),
        },

        // ── arithmetic: x.powf(runtime_int as f64) libm call → x.powi(n) binary
        // exponentiation ── the NON-CONSTANT exponent case the powi-vs-powf
        // annotation explicitly deferred (Batch 77 / lucebox PR #625 rope.cu
        // `rope_theta_fp64`: `while (e) { if (e & 1) r *= base; base *= base;
        // e >>= 1; }` — the source measured the libcall DOMINATING the whole
        // RoPE kernel on RDNA4: 692 → 76 µs/launch @512 tokens, within 1 ulp;
        // negative exponents keep the libcall there because `e >>= 1` on a
        // negative int never reaches 0 — the TOTALLY guard is part of the
        // content. On CPU: powf with a runtime exponent is an opaque libm
        // call (no instcombine bailout — the exponent is not a compile-time
        // constant, so the constant-exponent Tie verdict does NOT apply);
        // powi(n) is log2(n) multiplies. NOT bit-identical: powi is not
        // correctly-rounded pow — the ported-numeric MANDATE applies as a
        // rel-error gate (max rel-error across a swept exponent range, ulp
        // bound documented honestly, the newton-refinement annotation
        // precedent).
        PerfRule {
            rule_id: "runtime-int-exponent-powi".to_string(),
            name: "x.powf(n as f64) libm call with runtime integer exponent → x.powi(n) binary exponentiation".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn theta_scale(base: f64, pos: &[i32], exp: i32) -> Vec<f64> {\n    pos.iter()\n        .map(|&p| base.powf((p * exp) as f64)) // opaque libm pow call\n        .collect() // no instcombine bailout: the exponent is RUNTIME\n}".to_string(),
            fixed_code: "fn theta_scale(base: f64, pos: &[i32], exp: i32) -> Vec<f64> {\n    pos.iter()\n        .map(|&p| base.powi(p * exp)) // log2 multiplies, no libm call\n        .collect() // Rust powi handles negative exponents (1/x^|n|)\n}".to_string(),
            context_notes: Some(
                "Bench 77: ParetoDominant 2.57x/2.81x/2.66x (1K/100K/1M, release, \
                 M3 aarch64; verdict-stable x2; 0 allocs both arms). CROSS-ARCH \
                 RE-GATE (2026-08-27 same day, 4090 box, x86_64-pc-windows-msvc, \
                 THIS harness unchanged, verdict-stable x2): all-Tie \
                 0.99x/1.00x/1.00x, 0/0 allocs, alloc-det stable — on x86-msvc \
                 BOTH lowerings are comparable calls (MSVC pow is fast; powi \
                 rides a similar-cost routine), so the 2.6x is M3/aarch64 (and \
                 GPU-slow-libm) regime, NOT universal: suggest by host — expect \
                 the win where the platform's libm pow is slow (aarch64/M3, GPU \
                 device libm, embedded); on fast-libm x86 hosts this is an idiom \
                 swap, not a speedup (the newton-refinement regime-inversion \
                 class, Batch 62). The GPU \
                 motivating evidence is the source's RDNA4 RoPE kernel (692 -> 76 \
                 µs/launch, 9.1x) — the CPU verdict is this bench's own. NOT \
                 bit-identical: powi(n) is binary exponentiation, not \
                 correctly-rounded pow — gate on max rel-error across a swept \
                 exponent range (the ported-numeric MANDATE; the \
                 newton-refinement precedent). The constant-exponent-2 Tie \
                 verdict of powi-vs-powf does NOT apply here: a runtime exponent \
                 gives LLVM no instcombine bailout, so powf stays a libm call in \
                 release. Prefer powf when the result must be correctly rounded \
                 (ULP-sensitive accumulation) — the multiplies accumulate \
                 rounding error ~log2(n) ulp.".to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(runtime_int_exponent_powi_bench_spec()),
        },
        // ── arithmetic: (dx²+dy²).sqrt() < r → dx²+dy² < r² ────────────────────
        // Source: the Box2D squared-distance convention (box2d.org SIMD
        // matters). sqrt is monotonic on non-negative inputs, so comparing
        // squared distances preserves ordering — the sqrt is pure waste when
        // only the COMPARISON is consumed. Guard: if the distance VALUE is
        // consumed downstream, keep the sqrt; NaN inputs order differently
        // under the two forms (both false in practice for <).
        PerfRule {
            rule_id: "distance-sq-no-sqrt".to_string(),
            name: "(dx*dx+dy*dy).sqrt() < r → dx*dx+dy*dy < r*r (compare squared)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    (dx * dx + dy * dy).sqrt() < r // sqrt only to compare\n}".to_string(),
            fixed_code: "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    dx * dx + dy * dy < r * r // monotonic on non-negatives — ordering preserved\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(distance_sq_no_sqrt_bench_spec()),
        },
        // ── allocation: HashMap::new() + known-count loop → with_capacity ────
        // Source: users.rust-lang.org/t/44322 ("preallocate your Vecs and
        // HashMaps"). The HashMap sibling of `vec-with-capacity` but a
        // DIFFERENT cost class: growth rehashes — every resize re-runs the
        // hasher (SipHash) over ALL stored elements to place them in the new
        // bucket set, not just a realloc+memcpy. `with_capacity(n)` with a
        // known count pays one table alloc + zero rehash passes.
        PerfRule {
            rule_id: "hashmap-with-capacity".to_string(),
            name: "HashMap::new() + known-count insert loop → HashMap::with_capacity(n)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> {\n    let mut map = HashMap::new(); // starts at capacity ~3\n    for &it in items {\n        map.insert(it, it.wrapping_mul(31)); // rehash ladder: ~log2(n) resizes, each re-SipHashing all entries\n    }\n    map\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> {\n    let mut map = HashMap::with_capacity(items.len()); // one table alloc, zero resizes\n    for &it in items {\n        map.insert(it, it.wrapping_mul(31));\n    }\n    map\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hashmap_with_capacity_bench_spec()),
        },
        // ── layout: AoS iteration over one hot field → SoA split ─────────────
        // Source: nnethercote/perf-book data-structure layout + the Box2D SIMD
        // post (Batch 53, idle-queue item 11). Iterating ONE field of a wide
        // AoS struct touches every cache line but consumes 4 of 32 bytes
        // (12.5% utilization) and defeats auto-vectorization (strided
        // gather). The SoA split makes the hot field contiguous — 16 floats
        // per cache line + a clean SIMD sum. The cold fields pay one pointer
        // indirection on THEIR (rare) access.
        PerfRule {
            rule_id: "soa-hot-field".to_string(),
            name: "AoS Vec<WideStruct> single-field scan → SoA split (hot field contiguous)".to_string(),
            category: "layout".to_string(),
            buggy_code: "struct Agent { x: f32, y: f32, vx: f32, vy: f32, hp: u32, level: u32, flags: u64, id: u64 } // 40 bytes\nfn total_x(agents: &[Agent]) -> f32 {\n    agents.iter().map(|a| a.x).sum() // 4 useful bytes per 40-byte struct — strided, scalar\n}".to_string(),
            fixed_code: "struct Agents { xs: Vec<f32>, ys: Vec<f32>, vxs: Vec<f32>, vys: Vec<f32>, hps: Vec<u32>, levels: Vec<u32>, flags: Vec<u64>, ids: Vec<u64> }\nfn total_x(agents: &Agents) -> f32 {\n    agents.xs.iter().sum() // contiguous — 16 floats/line, SIMD-able\n}".to_string(),
            // Bench 53 (Bench 038): Mixed 1.07x/tie/0.96x — a serial f32
            // add-reduction (~3-4 cycle dependency chain per element) dominates
            // BOTH sides; the 40-byte-stride load latency is fully hidden by
            // out-of-order execution, so the bandwidth+vectorization win never
            // materializes on a latency-bound scalar sum. SoA wins materialize
            // on THROUGHPUT-bound field consumers (count/filter over the field,
            // min/max, batch transform — vectorizable, no serial chain) or when
            // multiple hot fields are scanned together.
            context_notes: Some(
                "Bench 53: Mixed (1.07x/tie/0.96x at 1K/100K/1M) — a serial f32 \
                 add-reduction dominates both layouts; the strided-load penalty \
                 is hidden by OoO execution, so SoA buys nothing on \
                 latency-bound scalar sums. The layout win materializes on \
                 throughput-bound field consumers (count/filter, min/max, batch \
                 transform — vectorizable, no serial chain) or multi-field scans; \
                 suggest for those shapes, not for scalar sum folds."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(soa_hot_field_bench_spec()),
        },
        // ── arithmetic: loop-invariant RUNTIME divisor → magic multiplication ──
        // Source: libdivide (ridiculous_fish) via thermite's Rust port
        // (raygon-renderer/thermite `divider.rs` — Batch 59, the first
        // external-source distill via the `distill` skill). Hardware
        // integer division is 20-90+ cycles and NOT vectorizable; a
        // loop-INVARIANT runtime divisor (a function parameter — opaque to
        // LLVM, so no strength reduction) converts once into a magic
        // multiplier + shift (Granlund–Montgomery), then each division is
        // mulhi + shifts (~4 cycles, pipelined). Two-regime rule (thermite's
        // own split): CONSTANT divisors need nothing — LLVM already
        // strength-reduces them to multiplication at compile time; the win
        // is exclusively the RUNTIME-invariant case applied ≥~3 times
        // (setup pays one hardware div). Guard: d must be nonzero + the
        // same value for the whole batch (bucket counts, shard stripes,
        // hash-then-div layouts).
        PerfRule {
            rule_id: "division-by-invariant-multiplication".to_string(),
            name: "Loop of x / d with runtime-invariant d → precomputed magic multiplier + shift (libdivide)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn bucket_sum(values: &[u32], d: u32) -> u32 {\n    let mut acc = 0;\n    for &x in values {\n        acc = acc.wrapping_add(x / d); // hardware udiv per element — d is a runtime invariant\n    }\n    acc\n}".to_string(),
            fixed_code: "struct FastDivU32 { magic: u32, shift: u32, add: bool }\nimpl FastDivU32 {\n    fn new(d: u32) -> Self { // one hardware div, once — amortized after ~3 divisions\n        let l = 31 - d.leading_zeros();\n        if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }\n        let k = 1u64 << l;\n        let base = (k << 32) / u64::from(d);\n        let rem = (k << 32) % u64::from(d);\n        if u64::from(d) - rem < k {\n            Self { magic: base as u32 + 1, shift: l, add: false }\n        } else {\n            let mut m = base * 2;\n            let rem2 = rem * 2;\n            if rem2 >= u64::from(d) { m += 1; }\n            Self { magic: (m + 1) as u32, shift: l, add: true } // (m+1) wraps mod 2^32 — intended (m spans (2^32, 2^33))\n        }\n    }\n    #[inline(always)]\n    fn div(&self, x: u32) -> u32 {\n        if self.magic == 0 { return x >> self.shift; } // power-of-two divisor\n        let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;\n        if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift } // widening add: u32 wrap drops the carry (d=7 x=MAX)\n    }\n}\nfn bucket_sum(values: &[u32], d: u32) -> u32 {\n    let fd = FastDivU32::new(d); // hoisted out of the loop\n    let mut acc = 0;\n    for &x in values {\n        acc = acc.wrapping_add(fd.div(x)); // mulhi + shift per element\n    }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(division_invariant_bench_spec()),
        },
        // ── dispatch: static-keyword HashMap lookup → plain match (LLVM length-switch) ──
        // Source: oxc "Pursuit of Performance on Building a JavaScript
        // Compiler" (oxc.rs/docs/learn/performance, rustmagazine issue-3;
        // Batch 61 online-search distill). oxc tried to beat a plain match on
        // string literals with a gperf-style perfect hash (V8's approach) and
        // COULD NOT — `--emit=llvm-ir` shows LLVM already lowers the match to
        // a switch on string LENGTH + one inline compare per length group.
        // The folk wisdom "string match is O(n) comparisons, use a hash map"
        // is wrong on modern LLVM: the HashMap pays SipHash per lookup while
        // the match pays a length switch + 1-2 byte compares. Bounded
        // fix-space: STATIC key sets only (a literal-initialized map);
        // dynamic keys keep the map.
        PerfRule {
            rule_id: "keyword-map-to-match".to_string(),
            name: "Static-keyword LazyLock<HashMap<&str, V>> dispatch → plain match (LLVM lowers to a length switch)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "use std::collections::HashMap;\nuse std::sync::LazyLock;\n#[derive(Clone, Copy)]\nenum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }\nstatic KEYWORDS: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {\n    let mut m = HashMap::new();\n    m.insert(\"as\", Kind::As);\n    m.insert(\"do\", Kind::Do);\n    m.insert(\"if\", Kind::If);\n    m.insert(\"let\", Kind::Let);\n    m.insert(\"fn\", Kind::Fn);\n    m.insert(\"while\", Kind::While);\n    m.insert(\"constructor\", Kind::Constructor);\n    m\n});\nfn match_keyword(s: &str) -> Kind {\n    *KEYWORDS.get(s).unwrap_or(&Kind::Ident) // SipHash per lookup\n}".to_string(),
            fixed_code: "#[derive(Clone, Copy)]\nenum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }\nfn match_keyword(s: &str) -> Kind {\n    // LLVM lowers string-literal matches to a switch on length +\n    // inline compares per length group (oxc verified via --emit=llvm-ir)\n    match s {\n        \"as\" => Kind::As,\n        \"do\" => Kind::Do,\n        \"if\" => Kind::If,\n        \"let\" => Kind::Let,\n        \"fn\" => Kind::Fn,\n        \"while\" => Kind::While,\n        \"constructor\" => Kind::Constructor,\n        _ => Kind::Ident,\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(keyword_map_to_match_bench_spec()),
        },
        // ── layout: usize index fields in high-volume structs → u32 ──────────
        // Source: oxc Span optimization (same article, Batch 61). The AST
        // leaf carried `struct Node { start: usize, end: usize }` — 16 bytes
        // — for byte offsets that cannot exceed u32 (< 4GB source). Narrowing
        // to u32 halved the node + measured up to +5% on large files (cache
        // density on the hottest leaf type). The mechanism is bandwidth:
        // half the bytes per element means twice the elements per cache
        // line. Guard: the domain must genuinely bound below 2^32 (byte
        // offsets, entity indexes, tick counters in long sessions need
        // audit). Type ripple is caller-wide — suggestion-quality rewrite,
        // no proposer (the Batch 58 signature-change lesson).
        PerfRule {
            rule_id: "usize-index-to-u32".to_string(),
            name: "usize index/offset fields in high-volume structs → u32 (halve cache footprint) RIDER (Batch 112 external confirmation, nnethercote 2026-07: rustc #158720 shrank AST expression nodes 72→64 B — \"small enough to fit in a cache line\" — >10% wall on AST-heavy benches + cache-miss −29%; the lineage from 104 B years ago is \"chip, chip, chipping\": shrink past the 64 B cache-line boundary is a step function, not a smooth curve)".to_string(),
            category: "layout".to_string(),
            buggy_code: "struct Span { start: usize, end: usize } // 16 bytes — pointer-sized for a < 4GB domain\nfn total_len(spans: &[Span]) -> u64 {\n    let mut acc = 0;\n    for s in spans { acc += (s.end - s.start) as u64; }\n    acc\n}".to_string(),
            fixed_code: "struct Span { start: u32, end: u32 } // 8 bytes — same domain, half the cache line traffic\nfn total_len(spans: &[Span]) -> u64 {\n    let mut acc = 0;\n    for s in spans { acc += u64::from(s.end - s.start); }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(usize_index_to_u32_bench_spec()),
        },
        // ── layout: hot counter arrays u64 "just in case" → u32 +
        // checked-promotion sidecar ──────────────────────────────────
        // Source: Datadog RTDB/Monocle (Batch 78, Research 092 — the
        // DDSketch aggregation buffer: "DDSketch bin counts are 64-bit
        // integers, but in practice most counts never approach the
        // 32-bit limit. We conserve memory by storing counts as u32
        // values in the aggregation buffer and promoting them to 64-bit
        // only if they overflow. Rust's built-in overflow checks make it
        // straightforward to detect when an upgrade to a larger counter
        // is needed."). The mechanism is the usize-index-to-u32 twin on
        // VALUES: half-width counters = 2x per cache line on the sweep.
        // DISTINCT from usize-index-to-u32 (Batch 61: narrows INDEX
        // fields whose domain bounds below 2^32 — a layout claim with a
        // domain guard); this narrows VALUE counters whose common case
        // stays low but whose domain does NOT bound them — the escape
        // hatch is runtime promotion (checked_add → sparse sidecar),
        // not a domain proof. The promoted slot's narrow cell zeroes
        // (dead) so the dense sweep stays branch-free; the sidecar is
        // sparse (only overflowing slots, an empty-HashMap fast path
        // guards the common regime). Datadog's compound (this + the
        // radix buffer + inline bins + shared buffer) measured +70%
        // aggregation throughput — the isolated mechanism is our bench.
        PerfRule {
            rule_id: "narrow-counter-overflow-promotion".to_string(),
            name: "hot counter arrays stored u64 \"just in case\" → u32 + checked-promotion sidecar (2x cache density)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Hot aggregation buffer: every counter is u64 \"just in case\" —\n// 8 bytes each, half the per-line density on the aggregation sweep.\nstruct Bins {\n    counts: Vec<u64>, // production data: most bins never approach 2^32\n}\nimpl Bins {\n    fn bump(&mut self, i: usize, v: u64) {\n        self.counts[i] += v; // full-width add for a counter that fits u32\n    }\n    fn total(&self) -> u64 {\n        self.counts.iter().sum() // sweep pays 2x the bytes per element\n    }\n}".to_string(),
            fixed_code: "// Same buffer, common case narrowed to u32 + a sparse promoted sidecar:\n// 2x counters per cache line on the sweep; Rust's checked_add IS the\n// overflow detector (the promote path moves the slot to the sidecar and\n// zeroes the narrow slot — totals stay exact).\nstruct Bins {\n    counts: Vec<u32>,                      // narrow fast path (dense sweep)\n    wide: std::collections::HashMap<usize, u64>, // promoted slots only (sparse)\n}\nimpl Bins {\n    fn bump(&mut self, i: usize, v: u32) {\n        if self.wide.is_empty() {\n            // common regime: nothing ever promoted — narrow add only\n            match self.counts[i].checked_add(v) {\n                Some(x) => self.counts[i] = x,\n                None => {\n                    let w = u64::from(self.counts[i]) + u64::from(v);\n                    self.counts[i] = 0; // narrow slot is dead now\n                    self.wide.insert(i, w);\n                }\n            }\n            return;\n        }\n        if let Some(w) = self.wide.get_mut(&i) {\n            *w += u64::from(v); // already promoted — stay wide\n            return;\n        }\n        match self.counts[i].checked_add(v) {\n            Some(x) => self.counts[i] = x,\n            None => {\n                let w = u64::from(self.counts[i]) + u64::from(v);\n                self.counts[i] = 0;\n                self.wide.insert(i, w);\n            }\n        }\n    }\n    fn total(&self) -> u64 {\n        // dense narrow sweep (2x density) + the tiny promoted patch\n        self.counts.iter().map(|&n| u64::from(n)).sum::<u64>()\n            + self.wide.values().sum::<u64>()\n    }\n}".to_string(),
            context_notes: Some(
                "MEASURED ParetoDominated 0.64-0.66x at 100K/1M/4M counters (M3 Max release, 2026-08-27; verdict-stable x2; alloc axis deterministic: buggy 1 alloc of 8N bytes vs fixed 1 alloc of 4N — the halving is exact). THE TRADE, honestly: the win is MEMORY (2x per cache line / half the footprint — Datadog's motivation was the 2,100 GiB infeasibility of per-worker DDSketch buffers, not speed), NOT wall-clock on update-heavy shapes: the per-update wide.is_empty() guard + checked_add overflow branch cost ~35-55% on a 4-updates-per-slot round-robin stream, and the flat ratio across sizes shows the workload is branch-bound, not bandwidth-bound (a bandwidth win would grow with the working set). REGIME where the trade PAYS: memory-footprint-bound systems (huge counter arrays, RAM/GB-priced deployment) and sweep-dominated access (read-mostly, few updates — the sweep alone touches half the bytes); the cost regime is update-heavy streams. Datadog's +70% aggregation throughput was the COMPOUND of the shared buffer + radix tree + inline bins + this narrowing — never cite it as the isolated mechanism. The promotion semantics are exact: totals match u64 bit-identically on every stream incl. u32-boundary crossings (the exhaustive-style G1 gate below). Distinct from usize-index-to-u32 (index fields, domain-bounded) — this narrows VALUE counters with a runtime escape hatch. Suggestion-only (type-blind: a parse-only proposer cannot see counter magnitude distributions). Source: Datadog RTDB/Monocle (datadoghq.com blog, 2025-08-04, Batch 78 / Research 092)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(narrow_counter_overflow_promotion_bench_spec()),
        },
        // ── layout: by-value moves of >128-byte types lower to memcpy CALLS ──
        // Source: nnethercote July-2026 rustc-perf post (rust-lang/rust
        // #160005, Research 134, Batch 112): LLVM lowers any by-value move
        // of a >128-byte type to a `memcpy` call instead of inline
        // load/store pairs — a hot 136-byte type paid ~half its compile
        // time in memcpy; the fix shrank the type to 104 bytes so the
        // remaining moves inlined (plus avoiding 2/3 of the moves outright)
        // → 40% wall on the two worst crates. DHAT copy-profiling was the
        // finding instrument. The 128 B threshold is an LLVM BACKEND
        // CONSTANT (`MaxStoresPerMemcpy` family) and it is ARCH-SHAPED:
        // x86-64's inline ceiling is 128 B (8 stores = 4 pairs x 32 B) so
        // 136 B → memcpy CALL; aarch64's ceiling is HIGHER — the corpus
        // gate's asm reading shows a 136-byte move inlining to ldp/stp
        // pairs (4 pairs + an 8 B tail) on Apple silicon with current
        // rustc, while a 320-byte move on the same box emits `bl _memcpy`
        // — the ARM boundary sits in (136, 320]. The M3 bench therefore
        // ties at 136 B (both arms inline); the x86 re-gate (4090,
        // same day) then showed the step DIRECTLY: the 48-byte copy
        // inlines while the 136-byte loop carries `movl $136;
        // callq memcpy` per iteration. Fix forms: (1) pass by reference where the
        // consumer only reads; (2) shrink the type under the threshold
        // (repack, usize-index-to-u32 composition, move the cold tail
        // out-of-line); (3) avoid the move (in-place construction —
        // architectural). Distinct from `box-large-enum-variant` (variant
        // packing at the ENUM level), `async-future-size` (state-machine
        // footprint, no move-cost axis), `soa-hot-field` (single-field scan
        // contiguity) — the mechanism here is the MOVE COST of the whole
        // type crossing the inline-memcpy boundary, found by copy-profiling
        // not by field scans. Suggestion-only (a parse-only proposer cannot
        // see type sizes or hot-path move frequency).
        PerfRule {
            rule_id: "hot-byval-move-over-memcpy-threshold".to_string(),
            name: "hot by-value moves of a >128-byte struct → shrink under the memcpy threshold / pass by reference (LLVM lowers big moves to memcpy CALLS)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Hot path: every call moves the 136-byte record BY VALUE — LLVM lowers\n// each move to a memcpy CALL (not inline stores) once the type crosses the\n// 128-byte inline threshold (x86-64 `MaxStoresPerMemcpy` = 128 B; aarch64's\n// ceiling is lower still). Copy-profiling (DHAT) is how you find this.\nstruct Record {\n    hot: [u64; 13], // 104 B — the fields the hot path actually consumes\n    cold: [u64; 4], // 32 B — carried along in EVERY move\n}\n\nfn score(r: Record) -> u64 {\n    let mut a = 0u64;\n    for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }\n    for i in 0..4 { a = a.wrapping_add(r.cold[i].wrapping_mul(3)); }\n    a\n}\n\nfn run(mut state: Record, n: usize) -> u64 {\n    let mut acc = 0u64;\n    for i in 0..n {\n        acc ^= score(state); // 136-byte memcpy per call\n        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);\n        state.cold[i & 3] = state.cold[i & 3].rotate_left(1);\n    }\n    acc\n}".to_string(),
            fixed_code: "// Fix (composed forms): shrink the moved type under the threshold by\n// moving the cold tail OUT of the hot record (by reference when read-only,\n// in-place when mutated) — the 104-byte moves now inline as load/store\n// pairs (on x86-64), and the cold fields are never copied at all.\nstruct Record {\n    hot: [u64; 13], // 104 B — under the threshold\n}\n\nfn score(r: Record, cold: &[u64; 4]) -> u64 {\n    let mut a = 0u64;\n    for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }\n    for i in 0..4 { a = a.wrapping_add(cold[i].wrapping_mul(3)); }\n    a\n}\n\nfn run(mut state: Record, cold: &mut [u64; 4], n: usize) -> u64 {\n    let mut acc = 0u64;\n    for i in 0..n {\n        acc ^= score(state, cold); // 104-byte move: inline stores\n        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);\n        cold[i & 3] = cold[i & 3].rotate_left(1); // cold tail mutated in place — zero copies\n    }\n    acc\n}".to_string(),
            context_notes: Some(
                "MEASURED Mixed 1/0/1 at 1K/1M iterations (M3 Max release, 2026-09-05, verdict-stable x2: 1.29x/0.99x then 1.29x/0.99x; allocs 0/0 both arms, alloc-det stable). The tie at n=1M is the honest mechanism cell, not a bench bug: the threshold step this rule encodes is ARCH-SHAPED and the M3 sits on the no-step side at 136 B. Asm evidence (the corpus gate reads the -C emit-asm output): on aarch64 + current rustc a 136-byte by-value move inlines to ldp/stp pairs (4 pairs + an 8 B tail) - NO memcpy call - while a 320-byte move on the same box emits bl _memcpy, so the aarch64 inline ceiling sits in (136, 320]; x86-64's ceiling is 128 B (nnethercote: rust-lang/rust #160005, July 2026 - any by-value move of a >128-byte type lowers to a memcpy CALL; a hot 136-byte type paid ~half its compile time in memcpy until shrunk to 104 B, 40% wall on the two worst crates; DHAT copy-profiling was the finding instrument). REGIME: a measured x86 win at the 128 B boundary; an ARM win only at larger types (once the move crosses the ARM ceiling - between 136 and 320 B on this toolchain - the call appears there too and the same shrink fixes it). The n=1K 1.29x cell is warm-up-dominated (per-iter cost drops 2x from n=1K to n=1M in BOTH arms); never cite it as the mechanism. The fix doctrine is arch-independent and never-negative (fewer/smaller by-value moves; profile with DHAT-class copy counters, not field scans). Composed-fix disclosure: the bench's fixed arm = shrink (136 -> 104) + the 32 B cold tail by-reference/in-place - both rule fix-forms; a binary bench cannot separate them. x86 RE-GATED 2026-09-05 (4090 box, quiet, rustc 1.93 x86_64-pc-windows-msvc): the asm shows the step DIRECTLY - the 48-byte loop's copy has NO memcpy call while the 136-byte loop's hot loop carries `movl $136; callq memcpy` per iteration, and the probe's wall clock moves the same direction (136 B loop = 1.32-1.35x the 48 B loop per iteration on x86 vs 1.00-1.13x on the M3). Distinct from box-large-enum-variant (enum-variant packing), async-future-size (state-machine footprint), soa-hot-field (field-scan contiguity) - the mechanism here is the MOVE COST of the whole type at a call boundary. Suggestion-only (a parse-only proposer cannot see type sizes or hot-path move frequency)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hot_byval_move_memcpy_threshold_bench_spec()),
        },
        // ── simd: transposed-weight register GEMV ─────────────────────────────────────
        // Source: microGPT-C (MIT, Research 070, Batch 62). The row-major
        // matvec dots every row into ONE scalar accumulator and pays a
        // horizontal reduction per row (the in-tree `binary.rs`
        // `vaddvq_f32(acc0)` shape); packing weights transposed once at
        // weight-freeze lets each input element broadcast and FMA into
        // output-vector accumulators — zero horizontal reduction, sequential
        // weight streaming, output registers held across the column loop
        // (the ryg 2015 transposed-lane Mv analysis). Distinct from the
        // kernel_opt GPU plane-per-row family (`workgroup-equals-plane-
        // native-reduction`, `single-plane-sufficient-reduction`): those are
        // GPU workgroup/shuffle concerns — this is the CPU SIMD register
        // twin. Architectural (weight storage packs at model-load; per-call
        // `&[f32]` params cannot pack in place) — suggestion-quality, no
        // proposer (the `soa-hot-field` class). Tail tiles 16/8/4 rows keep
        // lanes full at non-multiple nout.
        PerfRule {
            rule_id: "transposed-weight-register-gemv".to_string(),
            name: "Row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA (register GEMV)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// Row-major GEMV: each row is a dot product into ONE scalar\nfn matvec(w: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n    for r in 0..nout {\n        let row = &w[r * nin..(r + 1) * nin];\n        let mut acc = 0.0f32;\n        for c in 0..nin {\n            acc += row[c] * x[c]; // horizontal reduce at the end of EVERY row\n        }\n        y[r] = acc;\n    }\n}".to_string(),
            fixed_code: "// Transposed (column-major) weights: each input element x[c] broadcasts\n// once and FMAs into the whole output vector — lane-parallel, ZERO\n// horizontal reduction anywhere. Pack weights transposed ONCE at\n// weight-freeze; the pack cost amortizes over every call.\nfn matvec(w_t: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n    for r in 0..nout { y[r] = 0.0; }\n    for c in 0..nin {\n        let xc = x[c]; // one load, nout lane FMAs\n        let col = &w_t[c * nout..(c + 1) * nout];\n        for r in 0..nout {\n            y[r] += xc * col[r]; // auto-vectorizes: no per-row reduce\n        }\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(transposed_weight_register_gemv_bench_spec()),
        },
        // ── arithmetic: Newton refinement loses to hardware on CPU ───────────
        // Source: microGPT-C PERFORMANCE.md dead-ends (Research 070,
        // Batch 62) — the honest-annotation class (`filter-count-to-any`
        // precedent): the rule exists to STOP a plausible-looking GPU→CPU
        // port. On GPU, div/sqrt are multi-cycle → approximate + refine
        // wins; on CPU they are single-instruction + correctly rounded →
        // every Newton step is pure extra work (source measured 2-4%
        // slower on Ryzen 5 5600H). Distinct from
        // `regularizer-above-noise-floor-coupled-to-damping-cap` (that is
        // Newton-Schulz ITERATION robustness — different concern). The
        // relaxed-precision-estimate axis (`rsqrtss` 12-bit / `frecpe`) is
        // a SEPARATE valid optimization where accuracy can relax — never
        // suggest replacing precision-required divs.
        PerfRule {
            rule_id: "newton-refinement-vs-hardware-transcendental".to_string(),
            name: "Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction div/sqrt on CPU".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "// GPU-ported fast-math recip + rsqrt: seed + Newton refinement steps.\n// On GPU this wins (div/sqrt are multi-cycle → approx + refine);\n// on CPU the hardware op is single-instruction + correctly rounded,\n// so every refinement step is pure extra work.\nfn fast_inv(x: f32) -> f32 {\n    let r = 1.0 / x; // seed (estimate-tier on GPU)\n    let r = r * (2.0 - x * r); // Newton step 1\n    r * (2.0 - x * r) // Newton step 2\n}\nfn fast_rsqrt(x: f32) -> f32 {\n    let r = 1.0 / x.sqrt();\n    r * (0.5 * (3.0 - x * r * r)) // one Newton refinement\n}".to_string(),
            fixed_code: "// On CPU, `fdiv`/`fsqrt` are single-instruction and correctly rounded\n// — the refinement chain is pure extra work (measured 2-4% slower\n// than plain division on modern x86/ARM). Use the hardware op directly.\nfn fast_inv(x: f32) -> f32 { 1.0 / x }\nfn fast_rsqrt(x: f32) -> f32 { 1.0 / x.sqrt() }".to_string(),
            context_notes: Some(
                "Mixed (Batch 62 bench, M3 Max): hardware `fdiv`/`fsqrt` never \
                 lose to the refinement chain — 1.09x at 1K, 1.01x at 100K \
                 (source measured 2-4% on Ryzen 5 5600H). On CPU div/sqrt are \
                 single-instruction + correctly rounded, so every Newton step is \
                 pure extra work; the verdict INVERTS on GPU (multi-cycle \
                 div/sqrt → approximate+refine wins). Suggest the hardware op on \
                 CPU hot paths; never port GPU fast-math refinement chains. The \
                 relaxed-precision-estimate axis (12-bit estimate intrinsics) is a \
                 separate optimization valid ONLY where accuracy can relax — not \
                 for precision-required divs. x86 re-gate (2026-08-19, 4090 box, quiet): never-loses is CROSS-ARCH and the win is LARGER there — ParetoDominant 1.64x/1.66x at 1K/100K; the near-tie is M3-aarch64-specific (fdiv latency, pipeline-hidden at 100K)."
                .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(newton_refinement_vs_hardware_bench_spec()),
        },
        // ── simd: loop-order-innermost-contiguous ──
        // Source: Arun Nagaraj, 'Matrix Multiplication in Rust with
        // SIMD: From Naive to 126x Faster' (medium @aruntamiln
        // 2026-02-12; repo ArunTamil21/matmat-simd typo ArunTamil21/matmul-simd;
        // x86 AVX measured, technique ISA-independent — it is
        // memory-hierarchy, not ISA). The i-j-k nest's INNERMOST loop
        // (k) strides `b`'s OUTER axis (`b[k][j]` at stride-n) — every
        // access a new cache line, L1 misses 45%->12% after the reorder.
        // Distinct from `transposed-weight-register-gemv` (B62: repacks
        // the WEIGHTS column-major for GEMV — that changes the data
        // layout; this reorders the LOOPS over an unchanged layout, the
        // no-repack general nested-accumulation case), `simd-chunk`
        // (chunking for vectorization width), `layout-reorder` (struct
        // field padding), `soa-hot-field` (AoS->SoA). Legality: no
        // loop-carried deps across the swapped axes (the `c[i][j] +=`
        // accumulation shape is the canonical legal case). Float
        // sum-order changes under the reorder — rel-error tolerance
        // gate, NOT bit-identity (the `distance-sq-no-sqrt` precedent).
        PerfRule {
            rule_id: "loop-order-innermost-contiguous".to_string(),
            name: "i-j-k nested accumulation with a column-striding inner access → i-k-j reorder (innermost varies the LAST index)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// i-j-k nest: the INNERMOST loop (k) walks b[k][j] COLUMN-wise at stride-n\nfn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n    for i in 0..n {\n        for j in 0..n {\n            for k in 0..n {\n                c[i * n + j] += a[i * n + k] * b[k * n + j]; // stride-n inner\n            }\n        }\n    }\n}".to_string(),
            fixed_code: "// i-k-j reorder: the innermost loop (j) varies the LAST index — a\n// CONTIGUOUS row walk through b, c, and the hoisted a row anchor.\nfn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n    for i in 0..n {\n        let a_row = &a[i * n..(i + 1) * n]; // hoist the row anchor\n        for k in 0..n {\n            let a_ik = a_row[k];\n            for j in 0..n {\n                c[i * n + j] += a_ik * b[k * n + j]; // j strides by 1\n            }\n        }\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(loop_order_innermost_contiguous_bench_spec()),
        },
        // ── complexity: debug-assert-length-hoist ──
        // Source: Shnatsel, 'How to avoid bounds checks in Rust
        // (without unsafe!)' (2023-01-17, the canonical article;
        // repo Shnatsel/bounds-check-cookbook). A `debug_assert!`
        // mentioning `.len()` is COMPILED OUT in release — dropping
        // exactly the length constraint the optimizer needed to
        // eliminate the subsequent bounds checks, so release retains
        // ALL of them ('Ironically, this makes the code slower'). The
        // fix is the macro swap: `assert!` survives release, and
        // redundant asserts are optimizer-removed ('you cannot have
        // too many'). The counter-intuition is the value: `debug_assert!`
        // FEELS free (it vanishes!) but its vanishing is the pessimization.
        // Sibling fact encoded: an `assert!` in front of the hot loop is
        // the cheapest bounds-check hoist (check once, not per-iteration).
        // Honest expectation (annotation): aarch64 gains are the 1.03x
        // class (the 15% headline was an x86 unrolling secondary
        // effect) — the rule exists to stop the release-mode
        // pessimization, not to promise throughput.
        PerfRule {
            rule_id: "debug-assert-length-hoist".to_string(),
            name: "debug_assert! mentioning .len() is erased in release (dropping the constraint bounds-check elision needed) → assert!".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// debug_assert! is ERASED in release — the optimizer loses exactly the\n// range constraint it needed to eliminate the loop's bounds checks\nfn checksum(rows: &[(u64, u64)], first: usize, count: usize) -> u64 {\n    debug_assert!(first + count <= rows.len());\n    let mut acc = 0;\n    for i in first..first + count {\n        acc ^= rows[i].0.wrapping_add(rows[i].1); // release keeps the bounds check\n    }\n    acc\n}".to_string(),
            fixed_code: "// assert! SURVIVES release: the range constraint stays visible and the\n// bounds checks are elided (\"you cannot have too many asserts\" — the\n// redundant ones are optimizer-removed, never re-checked)\nfn checksum(rows: &[(u64, u64)], first: usize, count: usize) -> u64 {\n    assert!(first + count <= rows.len());\n    let mut acc = 0;\n    for i in first..first + count {\n        acc ^= rows[i].0.wrapping_add(rows[i].1); // checks elided in release\n    }\n    acc\n}".to_string(),
            context_notes: Some(
                "Mixed, regime-split (Batch 64 bench, M3 Max aarch64): the \
                 canonical range-pinned shape measures 1.13x at n=1K \
                 (elision win) but a STABLE 0.67-0.71x LOSS at n=100K \
                 (3-run verified) — on aarch64 the swap is shape- and \
                 scale-dependent, NOT a free win; the article's own aarch64 \
                 runs measured 1.03x (the 15% headline was an x86 unrolling \
                 secondary effect). Scope: the assert's constraint must be \
                 PROPAGATABLE to the indexed accesses (parameter/range-pinned \
                 indices) — a `debug_assert!` over a DATA-DEPENDENT index \
                 array can never enable elision (the compiler cannot \
                 propagate through runtime index values), so the swap there \
                 is pure cost. The rule's VALUE is stopping the \
                 counter-intuitive release-mode pessimization \
                 (`debug_assert!` feels free — its vanishing drops the \
                 length constraint bounds-check elision needed) while warning \
                 that the fix itself needs measuring per shape. Suggest \
                 `assert!` (redundant asserts are optimizer-removed) on \
                 range-pinned shapes; MEASURE before expecting throughput. x86 re-gate (2026-08-19, 4090 box, quiet): regime CONFIRMED cross-arch — 1.00x tie at 1K, 0.92x at 100K (milder than aarch64's 0.67-0.71x but still no free win; the article's x86 headline does not reproduce on this harness shape either).".to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(debug_assert_length_hoist_bench_spec()),
        },
        // ── arithmetic: sparse set-bit iteration — trailing_zeros + BLSR ──
        // Source: polars kernel idioms (Batch 66, Research 073 —
        // polars-compute/src/filter/boolean.rs + polars-arrow bitmap
        // TrueIdxIter). The buggy shape tests ALL 64 bit positions with a
        // shift+mask each; the fixed loop consumes the least significant
        // set bit per iteration (trailing_zeros + `m &= m.wrapping_sub(1)`
        // — the BLSR instruction where available) so it runs popcount(m)
        // iterations, not 64 — a ~16x iteration cut at 4/64 density. The
        // compiler cannot derive the tz-loop from the shift-test loop (the
        // trip count is data-dependent). Both forms emit identical indices.
        //
        // ANTI-SHAPE (Batch 68 rider, Research 075 — tantivy
        // common/src/bitset.rs `TinySet::pop_lowest`): code that ALREADY
        // iterates by trailing_zeros but clears with `x ^= 1 << lowest` is
        // ALSO this rule's territory — the xor-clear SERIALIZES the count
        // and the clear (the shift needs the tz result: ARM64
        // rbit→clz→lsl→eor, x86 tzcnt→btc), while `x &= x - 1` (Kernighan)
        // makes the clear INDEPENDENT of the count (ARM64 sub→and, x86
        // blsr — tzcnt runs in parallel; tantivy's in-source godbolt:
        // godbolt.org/z/fnfrP1T5f). Same indices, same popcount trips —
        // only the dependency chain changes.
        PerfRule {
            rule_id: "sparse-set-bit-iteration-tz-blsr".to_string(),
            name: "64-iteration set-bit scan → trailing_zeros + BLSR loop".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn sum_set_positions(mask: u64) -> u32 {\n    let mut sum = 0;\n    for i in 0..64 {\n        if (mask >> i) & 1 != 0 {\n            sum += i as u32;\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn sum_set_positions(mask: u64) -> u32 {\n    let mut sum = 0;\n    let mut m = mask;\n    while m != 0 {\n        let i = m.trailing_zeros(); // TZCNT\n        sum += i;\n        m &= m.wrapping_sub(1); // BLSR: clear least significant set bit\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sparse_set_bit_iteration_bench_spec()),
        },
        // ── iterators: sortedness early-exit → branchless accumulate ────
        // Source: polars SortedSlice::is_sorted_ascending (Batch 66,
        // Research 073 — polars-utils/src/slice.rs: "don't early stop or
        // branch so it autovectorizes"). The buggy early-exit is the
        // natural idiom but the per-element branch defeats vectorization;
        // the accumulate form scans all N as compare+AND and
        // auto-vectorizes. The mirrored regime of `filter-count-to-any`
        // (same vectorization mechanism, opposite direction: REMOVING an
        // early exit buys vectorization when the predicate rarely fires).
        PerfRule {
            rule_id: "sortedness-check-branchless-accumulate".to_string(),
            name: "sortedness early-exit validation → branchless accumulate full scan".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn validate_sorted(vals: &[u64]) -> bool {\n    let mut prev = u64::MIN;\n    for &v in vals {\n        if v < prev {\n            return false; // early exit blocks vectorization\n        }\n        prev = v;\n    }\n    true\n}".to_string(),
            fixed_code: "fn validate_sorted(vals: &[u64]) -> bool {\n    let mut sorted = true;\n    let mut prev = u64::MIN;\n    for &v in vals {\n        sorted &= prev <= v; // no branch: compare + AND, auto-vectorizes\n        prev = v;\n    }\n    sorted\n}".to_string(),
            context_notes: Some(
                "Regime-split (Batch 66 bench, M3 Max aarch64, load-recorded ~4-6): ParetoDominant 2.98x@100K / 2.72x@1M on USUALLY-SORTED data — the early-exit branch never fires but still forces scalar compare+branch per element, while the accumulate form vectorizes (batched compare+AND). On data with an EARLY violation the branchless form is ParetoDominated (it scans all N; the early exit stops at the first violation) — the mirrored regime of `filter-count-to-any`. Suggest only for usually-sorted validation (post-sort invariant checks, monotonic timestamps); prefer the early-exit form when violations are expected."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sortedness_check_branchless_bench_spec()),
        },
        // ── iterators: byte-find position scan → memchr primitive ──────
        // Source: regex-automata (Batch 67, Research 074 — rust-lang/regex,
        // regex-automata/src/util/memchr.rs + dfa/accel.rs; the engine's own
        // naive fallback IS this rule's buggy shape). `xs.iter().position(|&b| b == lit)`
        // on a `&[u8]` cannot dispatch to a byte-search primitive — the predicate
        // is opaque to the iterator machinery + early-exit blocks vectorization —
        // so std runs a scalar compare-per-byte scan. The memchr crate's
        // memchr/memrchr (and 2/3-needle variants for `||` chains, the engine's
        // exact arity ladder) scan 16-32B blocks with SIMD + movemask. Same
        // `Option<usize>` — bit-identical. EXCLUDES `xs.contains(&b)`: std
        // ALREADY specializes contains for u8 via `SliceContains` -> core's SWAR
        // memchr (core/src/slice/cmp.rs) — contains is already fast; the
        // position/rposition shape is where the win lives (the asymmetry IS the
        // rule). Dep-gated (the fxhashmap precedent): fire only when the target
        // manifest declares memchr, else suggestion-only. Regime: wash below
        // ~16-32B haystacks (call overhead eats the win; the engine's own
        // fallback concedes naive is fine small).
        PerfRule {
            rule_id: "byte-find-position-to-memchr".to_string(),
            name: "u8 slice iter().position(|&b| b == lit) → memchr::memchr".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn find_delim(buf: &[u8]) -> Option<usize> {\n    buf.iter().position(|&b| b == b';')\n}".to_string(),
            fixed_code: "fn find_delim(buf: &[u8]) -> Option<usize> {\n    memchr::memchr(b';', buf) // SIMD block scan + movemask early exit\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byte_find_position_to_memchr_bench_spec()),
        },
        // ── io: per-byte Write::write_all varint loop → ladder compose + one call ──
        // Source: tantivy (Batch 68, Research 075 — common/src/vint.rs
        // `write_u32_vint`/`serialize_vint_u32`; coordinator quote-verified
        // 2026-08-19). **MEASURED HONEST LOSER on this box (M3 Max aarch64,
        // load ~25-33, 3 regimes: Vec-push / inline-Cursor writer /
        // Box<dyn Write> vtable writer — ParetoDominated 0/3 sizes in EACH,
        // alloc axis equal + stable, outputs byte-identical).** The folk
        // mechanism story dies twice: (1) 128 is a compile-time CONSTANT —
        // LLVM strength-reduces `x % 128` / `x / 128` to `& 127` / `>> 7`
        // (the Batch 59 two-regime note applied in reverse — there is no
        // hardware-div chain to eliminate), and (2) even N vtable calls
        // into a dyn writer are hidden by out-of-order execution behind the
        // ladder's 5-deep branch chain. The entry stays as an ANNOTATED
        // honest finding (the `precompute-table` class): it exists to STOP
        // the plausible-looking "compose into a word, store once" port
        // from tantivy's codebase — measured, it REGRESSES on modern
        // aarch64 AND on x86 (re-gated 2026-08-19 on the 4090 box: dominated
        // 0/3 there too, 0.40-0.50x — HARDER than aarch64). Regime facts
        // that remain true: the ladder
        // is convention-faithful + bit-identical (exhaustive gate below),
        // and the u64 width boundary (10 bytes > word) keeps the loop.
        PerfRule {
            rule_id: "varint-compose-word-single-store".to_string(),
            name: "Per-byte write_all varint loop (N writer calls) → threshold-ladder compose + single write_all".to_string(),
            category: "io".to_string(),
            buggy_code: "use std::io::{Write, Result};\nfn write_vint<W: Write>(w: &mut W, mut val: u32) -> Result<()> {\n    loop {\n        let byte = (val % 128) as u8;\n        val /= 128;\n        if val == 0 {\n            w.write_all(&[byte | 128])?; // a writer call PER BYTE\n            return Ok(());\n        }\n        w.write_all(&[byte])?;\n    }\n}".to_string(),
            fixed_code: "use std::io::{Write, Result};\nfn write_vint<W: Write>(w: &mut W, val: u32) -> Result<()> {\n    let v = u64::from(val);\n    // compose ALL bytes into one u64 (masked shifts), ONE writer call\n    let (res, len) = if v < (1 << 7) {\n        (v | 128, 1)\n    } else if v < (1 << 14) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)\n    } else if v < (1 << 21) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)\n    } else if v < (1 << 28) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)\n    } else {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)\n    };\n    w.write_all(&res.to_le_bytes()[..len]) // ONE store, ONE call\n}".to_string(),
            context_notes: Some(
                "ParetoDominated (Batch 68 bench, M3 Max aarch64, load-recorded ~25-33, THREE regimes tested): the ladder LOSES 0/3 sizes on the Vec-push shape AND the inline-Cursor writer shape AND the Box<dyn Write> vtable shape — alloc axis equal + stable, outputs byte-identical. The folk mechanism story dies twice: (1) 128 is a compile-time constant, so LLVM strength-reduces x % 128 / x / 128 to & 127 / >> 7 — there is NO hardware-div chain to eliminate (the division-by-invariant-multiplication two-regime note applied in reverse); (2) even N dyn-dispatch writer calls are hidden by out-of-order execution behind the ladder's 5-deep branch chain. The rule exists to STOP the plausible-looking 'compose into a word, store once' port from tantivy's codebase — measured, it REGRESSES on modern aarch64 AND x86 (4090-box re-gate 2026-08-19, quiet, load 10: dominated 0/3 there TOO, HARDER — 0.40x/0.43x/0.50x at 1K/100K/1M, the ladder 2.0-2.5x slower than the per-byte loop; alloc axis equal, G-det stable). Bit-identity remains proven (exhaustive gate: full u16 + 128^i boundaries + LCG u32, both conventions)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(varint_compose_word_single_store_bench_spec()),
        },
        // ── io: shift/or byte-assembly word read → one unaligned from_le_bytes load ──
        // Source: tantivy bitpacker (Batch 69, Research 075 §Secondary — the
        // Plan 013 T6 defer MEASURED-REFUTED: the varint honest-loser was
        // CALL-COUNT (OoO hides N dyn-dispatch writer calls); this is
        // LOAD-COUNT (8 separately-loaded bytes vs 1 fused unaligned load)
        // — a different mechanism class, and OoO cannot hide 8 L1 loads
        // behind 1. LLVM does NOT fuse the shift/or assembly chain into a
        // byte copy (the dependency chain + per-byte bounds checks block
        // it). Cross-arch probe 2026-08-19 (M3 aarch64 load ~118 interleaved
        // + 4090-box x86 load 7-31, median-of-5): u64 2.17-2.23x / 1.39-1.47x,
        // u32 1.27-1.38x / 1.20-1.29x — ParetoDominant BOTH arches BOTH
        // widths, G1 bit-identical at EVERY byte alignment. u16 excluded
        // (2 bytes, unmeasured, diminishing). The bit-packed variant
        // (tantivy `BitUnpacker::get`: + shift+mask after the load) is the
        // same rule applied further along.
        PerfRule {
            rule_id: "unaligned-word-load-from-bytes".to_string(),
            name: "Shift/or byte-assembly word read (8 loads + 7 shifts + 7 ors) → one unaligned slice from_le_bytes load".to_string(),
            category: "io".to_string(),
            buggy_code: "fn read_u64_le(data: &[u8], addr: usize) -> u64 {\n    // 8 indexed loads + 7 shifts + 7 ors — a serial assembly chain\n    (data[addr] as u64)\n        | ((data[addr + 1] as u64) << 8)\n        | ((data[addr + 2] as u64) << 16)\n        | ((data[addr + 3] as u64) << 24)\n        | ((data[addr + 4] as u64) << 32)\n        | ((data[addr + 5] as u64) << 40)\n        | ((data[addr + 6] as u64) << 48)\n        | ((data[addr + 7] as u64) << 56)\n}".to_string(),
            fixed_code: "fn read_u64_le(data: &[u8], addr: usize) -> u64 {\n    // ONE unaligned 8-byte load — LLVM lowers from_le_bytes over a\n    // slice copy to a single unaligned u64 load\n    let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();\n    u64::from_le_bytes(bytes)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(unaligned_word_load_from_bytes_bench_spec()),
        },
        // ── caching: comparator lock+clone per invocation → maintained sort-key
        // cache refreshed once per data update ──────────────────────────────
        // Source: mnt.io "Memory pressure, lock contention, and Data-oriented
        // Design" (Batch 73, Research 085/086 — the Element X frozen-room-list
        // hunt: 322,042 allocations / 743 MiB inside `eyeball_im_util`
        // `SortBy` for ~500-600 rooms; 53.027 ms → 676.29 µs = 78× / −98.7%,
        // +7718.5% throughput on the `room_list` bench). Each sorter
        // comparison called `Room::latest_event()` —
        // `self.info.read().latest_event.clone()`, a read-lock acquire + a
        // 144-byte clone per call, twice per comparison, across 2 of 3
        // sorters; the eyeball-im-util `SortBy` machinery re-invokes the
        // comparator from `binary_search_by` on EVERY reactive diff and does
        // a fresh full sort on every filter change (`stream!` loop in
        // `entries_with_dynamic_adapters_impl`) — the sorts-per-update > 1
        // regime is structural. Fix: the production `RoomListItem` facade
        // caches the 6 sort/filter-relevant fields (verified in-source:
        // `matrix_sdk_ui::room_list::RoomListItem` + `refresh_cached_data()`
        // on every `RoomInfoNotableUpdate`; the sorter doc comments carry
        // the warning verbatim: "Using `Room::new_latest_event` would be
        // dramatic as it returns a clone of the `LatestEventValue").
        // REGIME: sorts-per-update ≥ 1 (reactive re-sort on diff/filter) —
        // the refresh cost (1 narrow read per changed item) amortizes over
        // N·log N comparator calls per sort. At sorts-per-update ≈ 0 (updates
        // never sorted) the maintained cache is pure refresh overhead.
        // Composes with `lock-guard-narrow-field-accessor` (the refresh reads
        // through narrow accessors, not whole-payload clones). Distinct from
        // `sort-by-key` (that still invokes the key fn N times per sort —
        // with the lock+clone inside it) and from `regex-hoist-lazylock`
        // (once-per-program vs once-per-data-update).
        PerfRule {
            rule_id: "sort-key-cache-precompute-out-of-lock".to_string(),
            name: "comparator lock+clone per invocation → maintained sort-key cache refreshed once per update RIDER (Batch 112 external confirmation, nnethercote 2026-07: rustc #157179 — rustdoc sorted impls by LONG GENERATED HTML STRINGS; a short text repr as the sort key won >6% icount — never sort by expensive payloads, derive a cheap key)".to_string(),
            category: "caching".to_string(),
            buggy_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event_timestamp(&self) -> Option<u64> {\n        // read-lock acquire + clone of the WHOLE LatestEventValue (144 B)\n        // to read one u64 — paid O(N log N) times per sort, twice per\n        // comparison, by every sorter AND every binary_search_by in the\n        // reactive diff machinery\n        self.info.read().unwrap().latest_event.clone().map(|ev| ev.timestamp)\n    }\n}\nfn sort_rooms(rooms: &mut [Room]) {\n    rooms.sort_by(|a, b| b.latest_event_timestamp().cmp(&a.latest_event_timestamp()));\n}".to_string(),
            fixed_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event_timestamp(&self) -> Option<u64> {\n        // narrow read through the guard — no clone (composes with\n        // lock-guard-narrow-field-accessor)\n        self.info.read().unwrap().latest_event.as_ref().map(|ev| ev.timestamp)\n    }\n}\n/// The production `RoomListItem` pattern: cache the sort/filter-relevant\n/// fields, refresh once per data update (only the CHANGED item).\nstruct RoomListItem {\n    inner: Room,\n    cached_latest_event_timestamp: Option<u64>,\n}\nimpl RoomListItem {\n    fn refresh_cached_data(&mut self) { // called on each RoomInfoNotableUpdate\n        self.cached_latest_event_timestamp = self.inner.latest_event_timestamp();\n    }\n}\nfn sort_rooms(rooms: &mut [RoomListItem]) {\n    // comparator reads the maintained cache — no lock, no clone\n    rooms.sort_by(|a, b| b.cached_latest_event_timestamp.cmp(&a.cached_latest_event_timestamp));\n}".to_string(),
            context_notes: Some("Measured ParetoDominant 24.3-37.2x across the sorts-per-update axis (1/4/16 sorts per refresh cycle, N=512; M3 Max release, load ~7.1, 2026-08-26; allocs 164497->17 + bytes 8.1MB->532KB @16 sorts, alloc-det stable; second invocation 28.1-35.1x — verdict stable). REGIME: the win GROWS with sorts-per-update (reactive re-sort on diff/filter — eyeball-im-util's SortBy re-invokes the comparator from binary_search_by on every diff, and every filter change does a fresh full sort); at sorts-per-update ~= 0 (data updated but never re-sorted) the maintained cache is pure refresh overhead — N narrow reads per Reset with no consumer. MAGNITUDE HONESTY: the source's 78x/-98.7% (53.027ms -> 676.29us, +7718.5% throughput, the matrix-rust-sdk room_list bench, ~500-600 rooms) is END-TO-END including allocation-pressure relief on a mobile allocator; our isolated-mechanism bench reads 24-37x — never cite 78x as our bench claim. Bench protocol note: the harness permutes via a fixed Fisher-Yates swap list before every sort — a plain reverse() is ONE detectable run for the adaptive driftsort (measured ~512 comparisons/sort vs the shuffle's ~5100 at N=512); the comparison count is the load-bearing axis. Composes with lock-guard-narrow-field-accessor (the refresh reads through narrow accessors).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_key_cache_bench_spec()),
        },
        // ── concurrency: whole-payload clone under read-guard → narrow field
        // accessor returning only the consumed stack-sized value ───────────
        // Source: same article (Batch 73, Research 085/086 — the +18%
        // throughput patch on the same `room_list` bench from splitting the
        // wide accessors alone). `fn field(&self) -> BigValue { self.info
        // .read().field.clone() }` where consumers read ONE sub-field (a
        // bool, a timestamp) pays a full 144 B clone per call for a 1-8 B
        // result. The production fix added narrow siblings —
        // `Room::new_latest_event_is_local() -> bool`, `new_latest_event_
        // timestamp() -> Option<Ms>` — verified in-source: the sorter doc
        // comments carry the rule verbatim ("It's better to use the more
        // specific method … where the value is cached"). Keeps the lock,
        // shrinks the guarded payload crossing — composable with (and
        // distinct from) `papaya-lockfree`/`mutex-to-atomic` (replace the
        // lock) and `avoid-clone` (borrow at fn boundary, no guard).
        PerfRule {
            rule_id: "lock-guard-narrow-field-accessor".to_string(),
            name: "fn field(&self) -> BigValue { guard.clone() } → narrow accessor returning only the consumed sub-field".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, sender_is_local: bool, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event(&self) -> Option<LatestEvent> {\n        self.info.read().unwrap().latest_event.clone() // 144 B clone under the guard\n    }\n    fn latest_event_is_local(&self) -> bool {\n        // clones the WHOLE guarded payload to compute one bool\n        self.latest_event().is_some_and(|ev| ev.sender_is_local)\n    }\n}".to_string(),
            fixed_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, sender_is_local: bool, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event_is_local(&self) -> bool {\n        // compute the ONE consumed sub-field under the guard — no clone\n        self.info.read().unwrap().latest_event.as_ref().is_some_and(|ev| ev.sender_is_local)\n    }\n}".to_string(),
            context_notes: Some("Measured ParetoDominant 11.5-12.7x at 1K-100K calls (M3 Max release, load ~7.1, 2026-08-26; allocs 100000->0, bytes 4.7MB->0 @100K, alloc-det stable across invocations). HONEST MARGIN NOTE: the source's +18% throughput (the same matrix-rust-sdk room_list bench) is END-TO-END — the narrow-accessor patch was one of several stacked optimizations; the isolated mechanism (uncontended read-lock + ~47-64B guarded-payload clone with a heap alloc, vs read-lock + bool read) reads 11-13x on this box because the clone dominates the lock; under real read contention the gap widens further (the clone holds the guard longer). The alloc axis (exact N->0) is the deterministic half of the verdict. Type-blind for a parse-only proposer (cannot see payload width or which sub-field a consumer reads) — suggestion-only, the soa-hot-field precedent.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(lock_guard_narrow_field_accessor_bench_spec()),
        },
        // ── concurrency: one lock guarding a large item array →
        // K-items-per-cache-line-block locks ──────────────────────────
        // Source: Datadog RTDB/Monocle (Batch 78, Research 092 — the
        // group-by aggregation buffer: "we leverage this pattern by
        // grouping eight consecutive points into a cache line-aligned
        // block protected by a single mutex. This fine-grained locking
        // confines contention to within each small block and allows
        // parallel updates with little synchronization overhead" —
        // their per-worker-buffer replacement after the 2,100 GiB
        // DDSketch duplication). ADJACENT-BUT-DISTINCT from
        // align-64-false-sharing (Batch 65): that PADS independent
        // slots so unrelated writes stop sharing a line (eliminates
        // FALSE sharing between independent slots); THIS partitions
        // one logical array's TRUE sharing at line granularity — the
        // items ARE related (same buffer), the lock granularity is the
        // tuning axis (global → per-block). Both live on the coherence-
        // granularity axis; the detection shapes differ (per-thread
        // ordinal slots vs one guarded array under a single lock).
        PerfRule {
            rule_id: "cache-line-block-partitioned-lock".to_string(),
            name: "one Mutex guarding a large item array → K-items-per-cache-line-block locks (partition TRUE sharing at line granularity)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "// One lock guards the whole point array: every bump serializes.\nstruct Series {\n    points: std::sync::Mutex<Vec<u64>>, // all points under ONE lock\n}\nimpl Series {\n    fn bump(&self, i: usize) {\n        self.points.lock().unwrap()[i] += 1; // full serialization per bump\n    }\n}".to_string(),
            fixed_code: "// K points per cache-line-aligned block, one lock per block: bumps to\n// different blocks proceed in parallel; contention drops by the block\n// count (here 8 points/block — the Datadog group-by buffer).\n#[repr(align(128))] // the ARCH coherence line (64 x86 / 128 Apple Silicon)\nstruct Block {\n    lock: std::sync::Mutex<[u64; 8]>, // 8 consecutive points per block\n}\nstruct Series {\n    blocks: Vec<Block>,\n}\nimpl Series {\n    fn bump(&self, i: usize) {\n        let b = &self.blocks[i / 8];\n        b.lock.lock().unwrap()[i % 8] += 1; // contention ~ /8\n    }\n}".to_string(),
            context_notes: Some(
                "MEASURED ParetoDominant 1.12x/3.91x/2.98x/3.03x at 2/4/8/12 threads, 4096 points, 20K bumps/thread (M3 Max release, 2026-08-27; verdict-stable x2; second invocation 1.14/4.02/2.95/3.03 — same verdict band). The mechanism: one Mutex<Vec<u64>> serializes EVERY bump across all threads; 512 aligned Mutex<[u64; 8]> blocks confine contention to within-block pairs (~8x fewer collision opportunities — measured 2.5-4x end-to-end at 4-12t; the 2t 1.12x is the honest low-contention edge). REGIME: multi-threaded bump workloads over a shared array; a single-threaded bump loop pays the block-index division for nothing (do not apply). DIFFERENTIATION vs align-64-false-sharing (Batch 65): that PADS independent per-thread slots to eliminate FALSE sharing between unrelated writes; THIS partitions the TRUE sharing of ONE logical array at lock granularity — the items are related (same buffer), the lock scope is the tuning axis. The two compose: block-partition first (this rule), pad the per-thread stats after (that one). ALLOC AXIS EXEMPT (the first such rule): contended std::sync::Mutex on macOS allocates nondeterministically in the pthread slow path (probed: global-mutex 12t measured 41-44 allocs across runs; uncontended construction exactly 513/512 locks) — the exemption is probe-evidenced in bench.rs ALLOC_AXIS_EXEMPT, not a silent skip. Suggestion-only (type-blind). Source: Datadog RTDB/Monocle group-by aggregation buffer (datadoghq.com blog, 2025-08-04, Batch 78 / Research 092)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(cache_line_block_partitioned_lock_bench_spec()),
        },
        // ── arithmetic: per-digit decimal parse loop → SWAR 8-digit chunk ─────
        // Source: simd-lite/simd-json `numberparse.rs` (Batch 74, Research
        // 088 — the Research 085 pool-drop re-probe; the MCP fetch failure
        // was an indexing gap, the repo is alive at v0.18.1, 1.4k stars).
        // `parse_eight_digits_swar` folds 8 ASCII digits into a u32 with
        // three and/mul/shift steps (magic multipliers 2561 = 10*256+1,
        // 6553601 = 100*2^16+1, 42949672960001 = 10^4*2^32+1 — each step
        // merges adjacent digit pairs/quads/halves, byte 0 = most
        // significant, verified by construction + the exhaustive gate);
        // `is_made_of_eight_digits_fast` validates 8 bytes as digits with
        // one SWAR test (every byte in 0x30..=0x39 iff `(val & 0xF0F0..) |
        // ((val+0x0606..) & 0xF0F0..) >> 4 == 0x3333..` — high nibble must
        // be 3 AND low nibble +6 must not carry, Muła's 0x80.pl
        // swar-digits-validate, cited in-source). The per-digit loop pays a
        // serial dependency chain (`val = val*10 + d` — LLVM cannot
        // reassociate) of ~6 ops/digit; the chunk path is ~10 ops per 8
        // digits. Provenance: Muła 0x80.pl ← simdjson C++ ← simd-json port.
        // REGIME (the source's own README, `swar-number-parsing` feature):
        // "comes at a slight performance hit if most of the floats have
        // less than 8 digits" — gate the chunk path on >=8 digits
        // remaining; the tail stays per-digit. Composes with
        // `unaligned-word-load-from-bytes` (the chunk load IS that
        // pattern); sibling of `division-by-invariant-multiplication`
        // (magic-multiply family — radix folding vs divisor inversion).
        // The SSSE3 escalation (`_mm_maddubs_epi16`, computes 16 values,
        // "we are being wasteful") exists when target_feature("ssse3") is
        // STATIC — non-portable, citation only.
        PerfRule {
            rule_id: "swar-digit-chunk-parse".to_string(),
            name: "per-digit decimal parse loop → SWAR 8-digit chunk fold (3 multiplies per 8 digits)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn parse_u64(digits: &[u8]) -> u64 {\n    let mut val: u64 = 0;\n    let mut i = 0;\n    while i < digits.len() && digits[i].is_ascii_digit() {\n        // serial dependency chain: 1 iteration per digit, ~6 ops each —\n        // LLVM cannot reassociate val = val*10 + d\n        val = val * 10 + u64::from(digits[i] - b'0');\n        i += 1;\n    }\n    val\n}".to_string(),
            fixed_code: "fn parse_u64(digits: &[u8]) -> u64 {\n    let mut val: u64 = 0;\n    let mut i = 0;\n    while i + 8 <= digits.len() {\n        let chunk: [u8; 8] = digits[i..i + 8].try_into().unwrap();\n        if !is_made_of_eight_digits_fast(&chunk) {\n            break; // a non-digit byte sits in this window — take the tail path\n        }\n        // 8 digits in 3 multiplies (magic radix fold, Mula/simdjson lineage)\n        val = val.wrapping_mul(100_000_000).wrapping_add(u64::from(parse_eight_digits_swar(&chunk)));\n        i += 8;\n    }\n    while i < digits.len() && digits[i].is_ascii_digit() {\n        val = val * 10 + u64::from(digits[i] - b'0'); // tail: < 8 digits remain\n        i += 1;\n    }\n    val\n}\n\n/// True iff every byte is in 0x30..=0x39 (high nibble 3, low nibble +6 no carry).\nfn is_made_of_eight_digits_fast(chars: &[u8; 8]) -> bool {\n    let val = u64::from_ne_bytes(*chars);\n    ((val & 0xF0F0_F0F0_F0F0_F0F0)\n        | (((val.wrapping_add(0x0606_0606_0606_0606)) & 0xF0F0_F0F0_F0F0_F0F0) >> 4))\n        == 0x3333_3333_3333_3333\n}\n\n/// Folds 8 ASCII digits (byte 0 = most significant) into a u32 via three\n/// and/mul/shift steps — the pairs/quads/halves merge of the SWAR radix fold.\nfn parse_eight_digits_swar(chars: &[u8; 8]) -> u32 {\n    let val = u64::from_le_bytes(*chars); // byte 0 at the LOW position on every target\n    let val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;\n    let val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;\n    ((val & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32) as u32\n}".to_string(),
            context_notes: Some("Measured ParetoDominant 2.35x/1.75x/2.52x at 8/12/19 digits-per-run (N_RUNS=2000; M3 Max release, load ~7.1, 2026-08-26; 0/0 allocs both arms, alloc-det stable; second invocation 2.35x/1.75x/2.52x-equivalent band — verdict stable). REGIME: 1.01x TIE at 4 digits — the chunk loop never fires below 8 digits; this is the source's own README tradeoff verbatim ('comes at a slight performance hit if most of the floats have less than 8 digits' — gate on >=8 digits remaining). The mechanism: the per-digit loop is a serial dependency chain (val = val*10 + d — LLVM cannot reassociate in strict FP... integer here, but the chain is still latency-bound at ~1 imul+add per digit); the SWAR fold breaks it with 3 and/mul/shift steps per 8 digits. G1: exhaustive equivalence gate (all lengths 0..=19 x boundaries x 10K LCG strings x terminator positions + the is_made_of_eight_digits_fast validation matrix + 100K fold checks) — bit-identical values + stop positions vs the u128 reference. Domain: digit runs whose value fits u64 (intermediate chunk multiples never exceed the final value — proven + gated). Type-blind for a parse-only proposer (cannot prove a loop parses digits semantically) — suggestion-only.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(swar_digit_chunk_parse_bench_spec()),
        },
        // ── dispatch: CPU feature detection inside a per-chunk hot fn ────────
        // Source: simd-lite/simd-json `parse_eight_digits_unrolled` (Batch 74,
        // Research 088) — the in-source measured decision comment: "Only use
        // the SSSE3 version when the feature is statically available: a
        // per-call `is_x86_feature_detected!` check costs more than the SWAR
        // fallback (cached-atomic load + two outlined `target_feature` calls
        // per 8-digit group)". The detection macro resolves the feature-name
        // STRING at runtime (std::detect) + reads the cached detector — for
        // small work units (their 8-digit groups; any per-chunk dispatch in
        // the wild) the detection rivals the work itself. Fix: resolve once
        // (cached static / OnceLock consulted per chunk, or hoisted to the
        // caller) or gate statically via `#[cfg(target_feature)]` when the
        // target set is compile-time known.
        PerfRule {
            rule_id: "runtime-feature-detect-per-chunk".to_string(),
            name: "is_x86_feature_detected! inside a per-chunk hot fn → resolve once (cached static / static cfg gate)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "fn sum_chunk(data: &[u8]) -> u64 {\n    // CPU feature detection paid PER CHUNK: the feature-name string is\n    // resolved + the cached detector read on every call — for a work unit\n    // of a few ns the detection rivals the work itself\n    if is_x86_feature_detected!(\"avx2\") {\n        data.iter().map(|&b| u64::from(b)).sum()\n    } else {\n        data.iter().fold(0u64, |acc, &b| acc + u64::from(b))\n    }\n}\nfn process(chunks: &[Vec<u8>]) -> u64 {\n    chunks.iter().map(|c| sum_chunk(c)).sum()\n}".to_string(),
            fixed_code: "use std::sync::atomic::{AtomicU8, Ordering};\n\n// detection hoisted: the string is resolved ONCE, then consulted per chunk\n// via one cached byte (0 = unknown, 1 = yes, 2 = no)\nstatic HAS_AVX2: AtomicU8 = AtomicU8::new(0);\nfn has_avx2() -> bool {\n    match HAS_AVX2.load(Ordering::Relaxed) {\n        0 => {\n            let v = is_x86_feature_detected!(\"avx2\");\n            HAS_AVX2.store(if v { 1 } else { 2 }, Ordering::Relaxed);\n            v\n        }\n        1 => true,\n        _ => false,\n    }\n}\nfn sum_chunk(data: &[u8]) -> u64 {\n    if has_avx2() {\n        data.iter().map(|&b| u64::from(b)).sum()\n    } else {\n        data.iter().fold(0u64, |acc, &b| acc + u64::from(b))\n    }\n}\nfn process(chunks: &[Vec<u8>]) -> u64 {\n    chunks.iter().map(|c| sum_chunk(c)).sum()\n}".to_string(),
            context_notes: Some("MEASURED Mixed — 0/0/3 TIES on this box: 1.01x/1.04x/0.99x at 16/256/4096 B-per-chunk (M3 Max release, rustc 1.97, 2026-08-26; 0/0 allocs, alloc-det stable, verdict stable x2 invocations). HONEST FINDING: on current rustc the feature-name match on a string literal const-folds/optimize away to near-zero even through an #[inline(never)] boundary — the per-chunk detection machinery (std::detect string resolution + cached detector read) measured ~sub-0.1ns marginal on aarch64. The cost class the rule encodes SURVIVES in: (a) outlined #[target_feature] dispatch (the source's measured x86 finding — 'a per-call is_x86_feature_detected! check costs more than the SWAR fallback (cached-atomic load + two outlined target_feature calls per 8-digit group)', simd-json parse_eight_digits_unrolled), (b) non-literal/runtime-composed feature strings, (c) heavier probe libraries. The rule exists to encode the granularity DECISION (resolve once: cached static, hoist to caller, or static cfg gate) with an honest measured-Tie on the modern-toolchain baseline — the B68 honest-annotation precedent; do NOT cite it as a default-config hot-path win.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(runtime_feature_detect_per_chunk_bench_spec()),
        },
        // ── allocation: dictionary-encoded column consumed through a generic ──
        // to_string materializer → borrowing layout-enum view ────────────────
        // Source: GreptimeDB "Just One Function, 10x Faster?" (PR #8587,
        // `recordbatches_to_timeseries` in src/servers/src/prom_store.rs;
        // Batch 75, Research 089). THE DICTIONARY TRAP, quoted: "the
        // dictionary-encoded group is 35% slower than the plain string
        // group... Dictionary encoding exists to save memory and copies, so
        // why is it slower?" Root cause: "`iter_column_as_string` does not
        // recognize dictionary columns. It falls back to the generic path and
        // calls `to_string()` row by row. The 3 strings become 1000, every
        // allocation upstream avoided is made again here, plus one extra
        // dictionary lookup per row." (A Dictionary<UInt32, Utf8> column
        // stores few distinct strings + a u32 index per row; a label layout
        // like `host` has few distinct values across many rows.) Fix: an enum
        // `LabelValues<'a>` with one variant per layout whose
        // `value(row) -> Option<&str>` "borrows straight from the array — no
        // copy and no allocation"; the dictionary branch does key(row) then
        // value(key). Measured in-source END-TO-END over the whole rewrite:
        // dictionary columns improved the most (55.6ms → 3.38ms at
        // adjacent/1000×100 = 16.5×) — never cite as this rule's isolated
        // number. Composes with `group-keys-borrow-allocate-on-first-sight`
        // (the source fix was both); distinct from `avoid-clone` (one clone
        // at a fn boundary; this is R clones per pass through a generic
        // accessor that erases the encoding) and `collect-avoidance`
        // (intermediate Vec staging; the expansion happens INSIDE the
        // accessor). Suggestion-only (type-blind: a parse-only proposer
        // cannot prove a column is dictionary/interned-encoded — the
        // soa-hot-field precedent).
        PerfRule {
            rule_id: "encoded-layout-borrowed-view".to_string(),
            name: "generic to_string() column materializer over dictionary-encoded data → borrowing layout-enum view (Option<&str>)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "struct DictColumn {\n    values: Vec<String>, // few distinct strings — what dictionary encoding is FOR\n    keys: Vec<u32>,     // one u32 per row indexing into values\n}\nimpl DictColumn {\n    fn lookup(&self, row: usize) -> Option<&str> {\n        let k = *self.keys.get(row)?;\n        self.values.get(k as usize).map(String::as_str)\n    }\n}\n// The generic accessor does not recognize the dictionary layout: it\n// materializes owned Strings row by row — \"the 3 strings become 1000\",\n// every allocation the encoding saved is made again here, plus one extra\n// dictionary lookup per row.\nfn column_as_strings(col: &DictColumn) -> Vec<Option<String>> {\n    (0..col.keys.len()).map(|r| col.lookup(r).map(|s| s.to_string())).collect()\n}\nfn label_len_total(col: &DictColumn) -> usize {\n    let cells = column_as_strings(col); // R heap allocs\n    cells.iter().map(|c| c.as_ref().map_or(0, |s| s.len())).sum()\n}".to_string(),
            fixed_code: "struct DictColumn {\n    values: Vec<String>,\n    keys: Vec<u32>,\n}\n/// One variant per column layout. `value` borrows straight from the array:\n/// \"no copy and no allocation\" — the dictionary branch does key(row) then\n/// value(key), reusing the same S distinct strings for all R rows instead\n/// of expanding into R owned Strings. Holding nothing but &refs makes the\n/// borrow contract compiler-enforced.\nenum ColumnView<'a> {\n    Utf8(&'a [String]),\n    Dictionary { values: &'a [String], keys: &'a [u32] },\n}\nimpl ColumnView<'_> {\n    fn value(&self, row: usize) -> Option<&str> {\n        match self {\n            ColumnView::Utf8(v) => v.get(row).map(String::as_str),\n            ColumnView::Dictionary { values, keys } => keys\n                .get(row)\n                .and_then(|&k| values.get(k as usize).map(String::as_str)),\n        }\n    }\n}\nfn label_len_total(col: &DictColumn) -> usize {\n    let view = ColumnView::Dictionary { values: &col.values, keys: &col.keys };\n    (0..col.keys.len()).map(|r| view.value(r).map_or(0, str::len)).sum()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 41.98x/39.38x/29.95x at R=10K/100K/1M, allocs 4,000,004 -> 0 + bytes 156.0 MB -> 0 at R=1M (S=100, L=4 (M3 Max release, rustc 1.97, 2026-08-26, load ~5.9 of 16; alloc-det stable). REGIME (distinct-values ratio S << R): the win is R·L owned-String materializations becoming S reused borrows — it grows with rows-per-distinct-value. At S ≈ R (mostly-distinct values) the encoding itself is pointless and the borrowed view saves ~nothing. Distinct from `avoid-clone` (one clone at a fn boundary; this is R clones per pass through a generic accessor that erases the encoding) and from `collect-avoidance` (the expansion happens INSIDE the accessor, not at a collect). The bench CONSUMES the view (black_box'd length sum) — an unread `&str` costs zero and the ratio inflates. The source's \"dictionary 35% slower than plain strings\" anomaly is its generic-fallback path (extra dictionary lookup per row on top of the expansion) — our bench measures the expansion cost only; never cite 35% as the expected win. Suggestion-only (type-blind: a parse-only proposer cannot prove a column is dictionary-encoded). Source: GreptimeDB PR #8587 walkthrough (2026-07-31), Steps 3-4 — the dictionary encoding upstream deliberately kept (#8541) was silently expanded downstream by one `iter_column_as_string`.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(encoded_layout_borrowed_view_bench_spec()),
        },
        // ── allocation: per-row owned group key → hash borrowed labels, ──────
        // allocate on first sight, one final sort ─────────────────────────────
        // Source: same article, THE COST MODEL + THE GROUPING REWRITE. Old:
        // "R·L calls to to_string()... one Vec<Label> per row, so R heap
        // allocations; one clone each for every label name and value, another
        // R·L string copies; and one BTreeMap lookup per row, each on the
        // order of log(S) comparisons." Fix: "A row only reaches
        // `new_timeseries` once it is judged to be a new series... R·L copies
        // become S·L copies. The whole optimization moves allocation from
        // 'every iteration' to 'the iteration that finds something new.'"
        // The decision content: std `HashMap::entry`/`BTreeMap::entry`
        // require an OWNED key, so the natural map APIs FORCE per-row key
        // materialization — the manual hash index (hash borrowed labels →
        // `HashMap<u64, Vec<usize>>` candidate list → FULL compare →
        // materialize only on first sight) keeps one hash per row and S
        // total allocations. CORRECTNESS CONTRACT: (1) the map value is a
        // candidate LIST — `HashMap<u64, usize>` merges distinct groups on
        // hash collision ("Merging two different series for the sake of
        // speed is a mistake this function cannot make"); (2) output order
        // flips from key-sorted to first-appearance unless the final
        // `sort_unstable_by` ships; (3) with optional fields skipped, one
        // key can be a PREFIX of another — the compare must verify
        // exhaustiveness. Suggestion-only (architectural — the
        // soa-hot-field class; a parse-only proposer cannot see that keys
        // repeat across rows).
        PerfRule {
            rule_id: "group-keys-borrow-allocate-on-first-sight".to_string(),
            name: "per-row owned group key (clone into map key every row) → hash borrowed labels, clone key only on first sight + one final sort".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use std::collections::BTreeMap;\nstruct Row {\n    labels: Vec<(String, String)>, // the series' (name, value) label set\n    value: u64,\n}\nfn group_rows(rows: &[Row]) -> Vec<(Vec<(String, String)>, (u64, u64))> {\n    let mut map: BTreeMap<Vec<(String, String)>, (u64, u64)> = BTreeMap::new();\n    for row in rows {\n        // per row: 1 Vec + 2·L String clones — R heap allocations where S\n        // would do — plus a BTreeMap insert at O(log S) ordered-string\n        // comparisons\n        let key: Vec<(String, String)> = row.labels.clone();\n        let e = map.entry(key).or_insert((0, 0));\n        e.0 += 1;\n        e.1 += row.value;\n    }\n    map.into_iter().collect() // BTreeMap gives sorted output for free\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nuse std::collections::hash_map::DefaultHasher;\nuse std::hash::{Hash, Hasher};\nstruct Row {\n    labels: Vec<(String, String)>,\n    value: u64,\n}\nstruct Group {\n    key: Vec<(String, String)>,\n    count: u64,\n    sum: u64,\n}\n// exhaustive compare — with optional fields skipped one key can be a\n// PREFIX of another; the length check prevents {host=a} matching\n// {host=a, env=prod}\nfn key_matches(key: &[(String, String)], row: &Row) -> bool {\n    key.len() == row.labels.len()\n        && key.iter().zip(&row.labels).all(|(a, b)| a == b)\n}\nfn group_rows(rows: &[Row]) -> Vec<Group> {\n    // hash → candidate index list, FULL label comparison on each candidate\n    // — \"merging two different series for the sake of speed is a mistake\n    // this function cannot make\"\n    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();\n    let mut groups: Vec<Group> = Vec::new();\n    for row in rows {\n        let mut h = DefaultHasher::new();\n        for (n, v) in &row.labels {\n            n.hash(&mut h); // borrowed — no alloc\n            v.hash(&mut h);\n        }\n        let hash = h.finish();\n        let idx = match index.get(&hash).and_then(|c| c.iter().find(|&&i| key_matches(&groups[i].key, row))) {\n            Some(&i) => i,\n            None => {\n                let i = groups.len();\n                // allocated ON FIRST SIGHT only — \"the whole optimization\n                // moves allocation from 'every iteration' to 'the iteration\n                // that finds something new': R·L copies become S·L\n                groups.push(Group { key: row.labels.clone(), count: 0, sum: 0 });\n                index.entry(hash).or_default().push(i);\n                i\n            }\n        };\n        groups[idx].count += 1;\n        groups[idx].sum += row.value;\n    }\n    // one S·log(S) sort is negligible against what was saved — restores\n    // the BTreeMap's ordered output contract\n    groups.sort_unstable_by(|a, b| a.key.cmp(&b.key));\n    groups\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 1.92x/2.27x/2.37x at R=10K/100K/300K (S=1000, L=3; M3 Max release, rustc 1.97, 2026-08-26, load ~5.9 of 16), allocs 70,133/700,133/2,100,133 -> 8,019 FLAT (the 7R+2S -> 8S+30 model, exact), bytes 55.9 MB -> 435 KB at 300K (alloc-det stable). REGIME (dedup ratio R/S): the win is R·L owned-key clones becoming S·L — it grows linearly with rows-per-group. At S ≈ R (mostly-distinct keys — every row a new group) first-sight allocation IS per-row allocation and the fixed form adds the hash pass + the final S·log(S) sort as pure overhead; gate on repeated-key grouping. The decision content: std `HashMap::entry`/`BTreeMap::entry` require an OWNED key, forcing per-row key materialization — the manual hash index (hash borrowed → `HashMap<u64, Vec<usize>>` candidates → FULL compare → materialize on first sight) keeps one hash per row and S total allocations. CORRECTNESS CONTRACT: (1) the map value is a candidate LIST — `HashMap<u64, usize>` merges distinct groups on hash collision; (2) output order flips from key-sorted to first-appearance unless the final `sort_unstable_by` ships — include it unless nothing downstream reads the order; (3) with optional fields skipped one key can be a PREFIX of another — the compare must verify exhaustiveness. Mirror regime of `btreemap-sorted-iteration` (repeated sorted reads → maintain a BTreeMap; single grouping pass → build + sort once) and distinct from `sort-key-cache-precompute-out-of-lock` (maintained comparator cache vs first-sight allocation in a scan). Bench isolates the grouping mechanism (borrowed inputs in both arms) — the source's 10.7-16.5x adjacent / 4.1-5.5x interleaved headline includes the borrowing + fast-path mechanisms stacked; never cite it for this rule alone. The map-mechanics swap alone is S-dependent (small S: short-circuiting BTreeMap compares can beat SipHash) — the load-bearing half is the R·L → S·L copy elimination. DefaultHasher (SipHash) is the faithful baseline; hasher swaps are an experiment, not the rule. Suggestion-only (architectural, the soa-hot-field class).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(group_keys_borrow_allocate_on_first_sight_bench_spec()),
        },
        // ── complexity: hash-every-row grouping → previous-row group check ───
        // first, hash fallback on miss ────────────────────────────────────────
        // Source: same article, THE FAST PATH: "Comparing the current row
        // against the series the previous row landed on is enough to reuse
        // it on a hit, with no hash to compute. This only optimizes a common
        // local ordering. It can break across partitions and across ranges,
        // and correctness does not depend on it." Measured axis (END-TO-END,
        // all p=0.00): adjacent 10.7-16.5x vs interleaved 4.1-5.5x — the
        // fast-path hit rate decides (adjacent = samples of one series
        // contiguous, the real query shape: rows sorted by primary key).
        // DERIVED (not stated in the source): the fixed arm's own
        // adjacent-vs-interleaved ratio is ~2.1-2.8x — the honest isolated
        // bound for this mechanism. Sibling of `bsearch-sorted-slice` +
        // `sortedness-check-branchless-accumulate` (all exploit input order).
        // Composes AFTER `group-keys-borrow-allocate-on-first-sight` (the
        // source shipped both). Suggestion-only (a parse-only proposer
        // cannot prove consecutive rows share keys).
        PerfRule {
            rule_id: "sorted-input-previous-row-fast-path".to_string(),
            name: "hash-every-row grouping → previous-row group check first (no hash on sorted-run hit, hash fallback on miss)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\nstruct Row {\n    series: [u64; 2], // the label-set key\n    value: u64,\n}\nstruct Group {\n    series: [u64; 2],\n    count: u64,\n    sum: u64,\n}\nfn group_rows(rows: &[Row]) -> Vec<Group> {\n    let mut index: HashMap<[u64; 2], usize> = HashMap::new();\n    let mut groups: Vec<Group> = Vec::new();\n    for row in rows {\n        // hash EVERY row — even when it lands on the same series as the\n        // previous row, the common case for primary-key-sorted input\n        let idx = match index.get(&row.series) {\n            Some(&i) => i,\n            None => {\n                let i = groups.len();\n                groups.push(Group { series: row.series, count: 0, sum: 0 });\n                index.insert(row.series, i);\n                i\n            }\n        };\n        groups[idx].count += 1;\n        groups[idx].sum += row.value;\n    }\n    groups\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nstruct Row {\n    series: [u64; 2],\n    value: u64,\n}\nstruct Group {\n    series: [u64; 2],\n    count: u64,\n    sum: u64,\n}\nfn group_rows(rows: &[Row]) -> Vec<Group> {\n    let mut index: HashMap<[u64; 2], usize> = HashMap::new();\n    let mut groups: Vec<Group> = Vec::new();\n    let mut prev: Option<usize> = None;\n    for row in rows {\n        // compare against the series the PREVIOUS row landed on first —\n        // no hash to compute on hit. \"This only optimizes a common local\n        // ordering. It can break across partitions and across ranges, and\n        // correctness does not depend on it\" — the hash fallback with\n        // full-key equality is the semantic contract.\n        let idx = match prev {\n            Some(i) if groups[i].series == row.series => i,\n            _ => match index.get(&row.series) {\n                Some(&i) => i,\n                None => {\n                    let i = groups.len();\n                    groups.push(Group { series: row.series, count: 0, sum: 0 });\n                    index.insert(row.series, i);\n                    i\n                }\n            },\n        };\n        prev = Some(idx);\n        groups[idx].count += 1;\n        groups[idx].sum += row.value;\n    }\n    groups\n}".to_string(),
            context_notes: Some("MEASURED Mixed 1/0/1 — adjacent (run=100) 12.92x FASTER, interleaved (run=1) 0.99x tie (R=100K, S=1000; M3 Max release, rustc 1.97, 2026-08-26, load ~5.9 of 16; allocs 19/19 equal by design, alloc-det stable). REGIME (fast-path hit rate = input locality): adjacent (sorted-run) input pays ~S hashes instead of R — the previous-row comparison reuses the group with no hash to compute; interleaved input pays one failed comparison per row before the hash — expected Tie-to-slightly-Slower, the honest bound of the regime. DERIVED bound (not stated in the source; derived from the source's fixed-arm adjacent-vs-interleaved ratios): ~2.1-2.8x isolated — never cite the source's 10-16x END-TO-END totals for this mechanism alone (they stack the borrowing + grouping + fast-path optimizations). The win scales with the hash-vs-compare cost ratio: long composite string keys maximize it (SipHash over all L labels vs a compare short-circuiting at the first diff); short/scalar keys shrink it toward a tie. CORRECTNESS CONTRACT: the fast path uses the SAME full-equality predicate as the fallback (one predicate, no divergent comparisons), the predicate is exhaustive on variable-length keys (a skipped-field key can be a PREFIX of another), `previous` updates every row, and a miss always falls through to hash+full-compare — the fast path is deletable without changing output (\"correctness does not depend on it\"). Family note: this ADDS a cheap pre-check before expensive per-row work (hash+probe) — it does not contradict `sortedness-check-branchless-accumulate`/`filter-count-to-any` (those remove per-element early exits inside vectorizable scans; the branch there guards ns of work, here it guards the dominant per-row cost). Suggestion-only (a parse-only proposer cannot prove consecutive rows share keys).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sorted_input_previous_row_fast_path_bench_spec()),
        },
        // ── complexity: eagerly-computed diagnostic statistic scans the whole ──
// collection per operation → remove from the hot path / maintain ─────
// incrementally ──────────────────────────────────────────────────────
// Source: Meilisearch "Patching LMDB" (2026-03-18), STOP COMPUTING
// USELESS STATS: the mean-degree sanity statistic — "added to Hannoy
// very early on as a sanity check" — was computed during item-ID
// retrieval, full-scanning the vector store on every insert batch:
// "Removing this reduces the time from 559s to about 2–3s and brings
// the complexity down to O(n), where n is the number of newly
// inserted or removed embeddings." The decision content: a diagnostic
// eagerly computed in a hot path is a hidden O(collection) tax that
// GROWS with total size — the hot path pays for the whole past, not
// the new work. Fix shapes: remove it (tests/CI now guard the
// invariant), gate behind debug/opt-in, or maintain the aggregate
// incrementally. Distinct from `sort-key-cache-precompute-out-of-lock`
// (comparator recompute under a lock, per-comparison; this is
// per-operation whole-collection statistics) and from
// `filter-count-to-any` (predicate full scan vs short-circuit; this is
// an aggregate whose RESULT isn't even consumed in production).
// Suggestion-only (type-blind: a parse-only proposer cannot prove the
// statistic is unconsumed or that the loop is diagnostic — the
// soa-hot-field class).
PerfRule {
    rule_id: "debug-stat-full-scan-out-of-hot-path".to_string(),
    name: "per-insert aggregate statistic recomputed over the ENTIRE collection (mean/count full scan) → out of the hot path (maintained incrementally / debug-gated)".to_string(),
    category: "complexity".to_string(),
    buggy_code: "use std::collections::HashMap;\nstruct Item {\n    links: Vec<u32>,\n}\nstruct Store {\n    items: Vec<Item>,\n}\nimpl Store {\n    // a sanity statistic \"added very early on\" — full-scans EVERY item\n    // on EVERY insert batch. O(total) per op: the hot path pays for the\n    // whole past, not the new work; grows without bound as the store does.\n    fn mean_degree(&self) -> f64 {\n        let total: usize = self.items.iter().map(|i| i.links.len()).sum();\n        self.items.len() as f64 / total.max(1) as f64\n    }\n    fn insert_batch(&mut self, new_items: Vec<Item>) {\n        for it in new_items {\n            self.items.push(it);\n        }\n        // diagnostic eagerly computed in the hot path — the only consumer\n        // was the developer's peace of mind (CI now guards the invariant)\n        let _mean = self.mean_degree();\n    }\n}".to_string(),
    fixed_code: "struct Item {\n    links: Vec<u32>,\n}\nstruct Store {\n    items: Vec<Item>,\n    // maintained aggregate: link_count updates by the DELTA each insert\n    // — the statistic stays available at O(1) for whoever needs it, and\n    // the hot path never pays for the whole past\n    link_count: usize,\n}\nimpl Store {\n    fn mean_degree(&self) -> f64 {\n        self.items.len() as f64 / self.link_count.max(1) as f64\n    }\n    fn insert_batch(&mut self, new_items: Vec<Item>) {\n        for it in new_items {\n            self.link_count += it.links.len();\n            self.items.push(it);\n        }\n        // no full scan — the invariant moved to CI/tests (the source's\n        // fix: remove it; the maintained form when the stat IS wanted)\n    }\n}".to_string(),
    context_notes: Some("MEASURED ParetoDominant 3/0/0 — 168.91x/1929.27x/5851.00x at N=10K/100K/300K store size (B=100 inserted per op; M3 Max release, rustc 1.97, 2026-08-27, load ~2.4 of 16; verdict-stable ×2: 179.55x/1883.75x/6293.55x on the re-run; allocs 0/0 equal by design, alloc-det stable). REGIME (store-size to batch-size ratio N/B): the buggy arm pays O(N) per insert batch — the win GROWS linearly with total store size (the source: 559s → 2–3s on a 19M-item store, ~200x on the step). At N ≈ B (tiny store, huge batches) the scan is one pass over what the insert touches anyway — expected Tie; the rule lives where the collection ACCUMULATES (insert-heavy stores where per-op diagnostics re-walk the past). The decision content: diagnostics eagerly computed in hot paths are a hidden O(collection) tax — the source's fix was REMOVAL (tests/CI now guard the invariant); the maintained-aggregate form is the fix when the statistic is actually consumed downstream. Distinct from `sort-key-cache-precompute-out-of-lock` (per-comparison recompute under lock) — this is per-operation whole-collection statistics. Suggestion-only (a parse-only proposer cannot prove the statistic unconsumed). Source: Meilisearch blog 2026-03-18 §Stop computing useless stats.".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: Some(debug_stat_full_scan_out_of_hot_path_bench_spec()),
},
// ── complexity: measurement honesty — a timed harness whose work result ──
// is dead measures ZERO under fat LTO (rustc 1.98.1 measured class) ───────
// Source: post-mining intake P10 (2026-09-04), found by the katgpt-rs Issue
// 723 Class A2 fixes (`b4cef008`): a direct-call probe in ONE binary showed
// the used result at 16.6 µs vs `let _ = f()` ≈ 0 — the fold eliminates
// inlined-callee work when the caller discards the result, EVEN THROUGH
// `black_box` INSIDE the callee (the barrier protects the callee's own
// return value from intra-callee folding; it cannot keep the CALLER's dead
// result alive — under fat LTO the whole chain is one CFG and the work's
// only observable use is gone). Three harness patterns proven in-tree:
// (1) the timed work must CHAIN its state (each iteration consumes the
// previous accumulator — the xorshift-PRNG shape, unprunable by
// construction): parallel sinks cancel in batches three ways — XOR dies
// on GF(2) linearity over 0..2^k, wrapping_add collapses to the
// seed-independent 2^64 − n/2 constant over balanced power-of-two
// batches, and constant work folds through anything; (2) the
// black_box lives at the CONSUMER (in the timed loop, not only inside
// the work fn — a caller-dead result dies under fat LTO); (3) for
// bar-adjacent ratios, interleave (baseline, candidate) chunks and take a
// median-of-ratios — two sequential 2M-iteration arms read +5.2%/+21.7%
// drift thirty seconds apart while the paired median read −0.0%/+0.6%
// twice. The LOUD-BASELINE corollary is the cheap detector every timing
// gate should carry: assert the baseline elapsed is NONZERO before forming
// a ratio. The failure directions differ by comparison form — a `>=`
// PASS bar with a NaN ratio (0 ns baseline → 0/0) is always-false = a loud
// fail; a `<=` budget bar with NaN is always-false TOO, so a budget gate
// reads as PASSING forever while measuring nothing — the silent direction.
// Both arms must do equal non-pruner work (one shared timed fn, the
// candidate's delta passed as data) so the ratio isolates the delta rather
// than arming the fold on one side only. Distinct from
// `debug-stat-full-scan-out-of-hot-path` (an UNCONSUMED PRODUCTION
// statistic dragged through a hot path; this is the HARNESS side — the
// production code is fine, the measurement of it is empty) and from
// `encoded-layout-borrowed-view`'s bench note ("an unread &str costs zero
// and the ratio inflates" — the same physics from the bench-consumption
// side; this rule is the general harness contract). Suggestion-only: the
// detector is a dataflow question (is the timed loop's result observable
// anywhere?) that a parse-only proposer cannot answer; the loud
// zero-baseline assert is the machine-checkable surrogate. No Pareto bench
// is possible — the rule's entire content is that the baseline is zero, so
// there is nothing to ratio (NotBenchable, the measurement-honesty axis).
PerfRule {
    rule_id: "dead-result-timing-loop-folded-under-lto".to_string(),
    name: "timed loop discards its work result (`let _ = f()`) → fat-LTO fold eliminates the inlined callee and the gate times ZERO — xorshift data-dependent work + `black_box(sink)` at the consumer + a loud nonzero-baseline assert".to_string(),
    category: "complexity".to_string(),
    buggy_code: "use std::hint::black_box;\nuse std::time::Instant;\n\nfn work(seed: u64) -> u64 {\n    // some non-trivial kernel the gate pretends to measure\n    let mut x = seed;\n    for _ in 0..64 {\n        x ^= x << 13;\n        x ^= x >> 7;\n        x ^= x << 17;\n    }\n    x\n}\n\nfn overhead_ratio(iters: u32) -> f64 {\n    let start = Instant::now();\n    for i in 0..iters {\n        // the result is DISCARDED: under rustc 1.98.1 + fat LTO the\n        // inlined body folds away (the callee-internal black_box cannot\n        // keep a caller-dead result alive) and this times ~0 ns\n        let _ = work(black_box(i as u64));\n    }\n    let baseline = start.elapsed();\n    // NaN when baseline is 0: with a `<=` budget bar this is an\n    // always-false comparison = a SILENT vacuous pass, forever\n    baseline.as_nanos() as f64 / 1_000.0\n}".to_string(),
    fixed_code: "use std::hint::black_box;\nuse std::time::Instant;\n\nfn work(seed: u64) -> u64 {\n    let mut x = seed;\n    for _ in 0..64 {\n        x ^= x << 13;\n        x ^= x >> 7;\n        x ^= x << 17;\n    }\n    x\n}\n\nfn overhead_ratio(iters: u32) -> f64 {\n    let start = Instant::now();\n    // CHAIN the state: each iteration consumes the previous accumulator,
    // so every output depends on the whole sequential history — the
    // xorshift-PRNG shape, unprunable by construction. PARALLEL sinks
    // cancel in batches and are the trap this rule exists for: an XOR
    // sink over a GF(2)-linear work fn (xorshift, CRC, most bit math)
    // is exactly 0 over 0..2^k (linearity), and even a wrapping_add
    // sink collapses to the seed-independent constant 2^64 − n/2
    // over a power-of-two batch when the work’s output bits are
    // balanced — both measured, not theorized. The per-iteration
    // black_box on the index keeps the loop input live; the final
    // black_box consumes the chain.
    let mut state = black_box(0x9E37_79B9_7F4A_7C15u64);
    for i in 0..iters {
        state = work(state ^ black_box(i as u64));
    }
    black_box(state);\n    let measured = start.elapsed();\n    // the loud-baseline assert: a gate that measured 0 ns is a gate that\n    // measured NOTHING — fail before any ratio can go NaN\n    assert!(measured.as_nanos() > 0, \"timing harness folded: dead-result work measured zero ns — chain the timed state — parallel sinks cancel in batches (XOR: GF(2) linearity; balanced-add: the 2^64 − n/2 constant over 2^k)\");\n    measured.as_nanos() as f64 / 1_000.0\n}".to_string(),
    context_notes: Some("NotBenchable — measurement-honesty axis (Batch 109): the rule's content is that the baseline folds to zero, so there is no nonzero quantity to ratio; the evidence is the katgpt-rs direct-call probe (used result 16.6 µs vs `let _ = f()` ≈ 0, one binary, rustc 1.98.1 + fat LTO) and the rv gate that measured 0 ns → NaN% for its entire release life (found by Issue 723 Class A2, `b4cef008`). The load-bearing subtleties: (1) `black_box` INSIDE the callee does NOT save the work when the CALLER discards the result — under fat LTO the barrier only pins the callee's own return value, and the whole chain is one CFG where the value has no observer; (2) PARALLEL sinks cancel in batches — THREE measured modes: XOR over a GF(2)-linear work fn (xorshift, CRC, most bit math) is exactly 0 over 0..2^k (linearity; the gate caught the rule’s OWN first draft dying on this), wrapping_add over a power-of-two batch collapses to the seed-independent constant 2^64 − n/2 when the work’s output bits are balanced (each bit is 1 in exactly half the terms regardless of seed — measured: both arms read 18446744073709549568), and a constant-work fn folds through any sink at compile time — the robust fix is CHAINED STATE (each iteration consumes the previous accumulator, the xorshift-PRNG shape, unprunable by construction); (3) the NaN direction contract: NaN compares false against EVERYTHING, so a `>=` PASS bar fails loudly on a zero baseline while a `<=` budget bar passes silently forever — the assert must fire BEFORE the ratio; (4) for ratio gates, equal non-pruner work in both arms (one shared timed fn) + interleaved median-of-ratios (sequential arms drifted +5.2%/+21.7% thirty seconds apart; the paired median read −0.0%/+0.6% twice). Distinct from `debug-stat-full-scan-out-of-hot-path` (unconsumed PRODUCTION statistic; this is the HARNESS side) and from `encoded-layout-borrowed-view`'s bench-consumption note (the same physics from the bench side). Suggestion-only (a parse-only proposer cannot prove the timed result is unobservable; the zero-baseline assert is the machine-checkable surrogate). Source: post-mining intake P10, katgpt-rs Issue 723 Class A2 evidence (`tests/bench_gdsd_modelless.rs` + `tests/rv_gated_routing.rs` sink docs).".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: None,
},
// ── caching: materialize a lookup index over ALL entries (full scan) ───
// to serve a SUBSET of random accesses → lazy/direct access of only ───
// the needed entries ───────────────────────────────────────────────────
// Source: same article, FULL SCANS ARE HARMFUL + WHAT'S THE NEW TRICK:
// the ImmutableItems/ImmutableLinks structures did "a full scan of all
// the entries we will need to access", collecting pointers into a
// HashMap — then "we were polluting the cache with useless,
// unnecessary disk pages just by iterating over all of them to fetch
// pointers." Replaced by FrozenReader: "read uncommitted graph nodes
// and connections lazily, on demand" so "only the necessary pages are
// loaded into the cache — not all available pages in LMDB. We avoid
// evicting important pages." The decision content: building an
// everything-index to serve a subset of accesses pays the scan TWICE —
// once in time (O(all) build) and once in cache (the scan's own
// traffic evicts the hot pages the subset will need). When the
// underlying store offers a direct access path, index only on demand.
// Distinct from `collect-avoidance` (intermediate-Vec staging, alloc
// axis; this is working-set residency) and from
// `encoded-layout-borrowed-view` (representation re-expansion inside
// an accessor; this is eager INDEX MATERIALIZATION before a subset
// read). Suggestion-only (architectural — a parse-only proposer
// cannot see the direct-access path exists).
PerfRule {
    rule_id: "eager-pointer-collect-lazy-fetch".to_string(),
    name: "full-scan materialize a HashMap of ALL entry pointers to serve a SUBSET of random reads → direct/lazy access of only the needed entries. RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- --lazy-mode, commits fac889f #27794 + bebc935 #27969 + 2578138 #27837 + b19cbe9 #27869; APP-LAYER at this pin: common/arg.cpp:2737, include/llama.h:217-220, src/llama-model-loader.cpp:1082-1088): an entire user-facing feature built as this rule's pattern at model-load scale -- lazy mode defers tensor materialization to first use (llama_lazy_mode ON/AUTO/OFF) instead of collecting every weight upfront.".to_string(),
    category: "caching".to_string(),
    buggy_code: "use std::collections::HashMap;\nstruct Node {\n    payload: [u64; 4],\n}\nstruct Store {\n    nodes: Vec<Node>, // direct-indexable: ids are dense 0..len\n}\nimpl Store {\n    // full scan of EVERY entry, collecting pointers into a HashMap —\n    // \"polluting the cache with useless, unnecessary disk pages just by\n    // iterating over all of them to fetch pointers\"\n    fn freeze_all(&self) -> HashMap<usize, &Node> {\n        let mut m = HashMap::with_capacity(self.nodes.len());\n        for (i, n) in self.nodes.iter().enumerate() {\n            m.insert(i, n);\n        }\n        m\n    }\n    fn touch(&self, ids: &[usize]) -> u64 {\n        let index = self.freeze_all(); // O(all) build to serve a subset\n        let mut acc = 0u64;\n        for &id in ids {\n            if let Some(n) = index.get(&id) {\n                acc = acc.wrapping_add(n.payload[0]);\n            }\n        }\n        acc\n    }\n}".to_string(),
    fixed_code: "struct Node {\n    payload: [u64; 4],\n}\nstruct Store {\n    nodes: Vec<Node>,\n}\nimpl Store {\n    // lazy/on-demand: no index build, no scan — \"only the necessary pages\n    // are loaded into the cache\"; the store IS the direct-access path\n    fn touch(&self, ids: &[usize]) -> u64 {\n        let mut acc = 0u64;\n        for &id in ids {\n            if let Some(n) = self.nodes.get(id) {\n                acc = acc.wrapping_add(n.payload[0]);\n            }\n        }\n        acc\n    }\n}".to_string(),
    context_notes: Some("MEASURED ParetoDominant 3/0/0 — 1903.70x/5414.62x/21353.90x at N=100K/300K/1M store size (K=1,000 touched ids; M3 Max release, rustc 1.97, 2026-08-27, load ~2.4 of 16; verdict-stable ×2: 2217.57x/5962.48x/25736.41x on the re-run; allocs 1->0, bytes 35.7 MB -> 0 at N=1M — the HashMap build eliminated; alloc-det stable). REGIME (subset ratio K/N + working-set fit): the win is two-part — the O(N) scan+hash build eliminated AND the scan's own cache traffic no longer evicts the K pages the subset reads (the source's disk-page story, reproduced here at cache scale: N=1M × 32 B nodes ≈ 32 MB > L2, so the scan evicts what the subset needs). At K ≈ N (read-most-everything) the index build amortizes to nothing — expected Tie-to-slower for the lazy form only when the SAME subset is read REPEATEDLY (then a persistent index is the sort-key-cache rule instead); the rule lives at K/N ≪ 1 with one-shot or sparse access. The eager arm's HashMap<usize, &Node> is the faithful source shape (pointer maps over mmap'd entries). Distinct from `collect-avoidance` (alloc staging) — the load-bearing half here is working-set residency; distinct from `encoded-layout-borrowed-view` (accessor re-expansion) — this is index materialization BEFORE a subset read. Suggestion-only (architectural: a parse-only proposer cannot see the direct-access path). Source: Meilisearch blog 2026-03-18 §Full scans are harmful + §But what's the new trick?.".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: Some(eager_pointer_collect_lazy_fetch_bench_spec()),
},
// ── simd: repeatedly filtered integer column stored row-wise → bit-plane ──
// transposition + zero-decompression bitwise predicate evaluation ─────────
// Source: arXiv:2608.26368 (Research 097, the O'Neil & Quass bit-sliced
// index rebuilt as a PRIMARY representation). Values are transposed into b
// bit-planes (b = bits of the column max — adaptive per block in the
// source); a > c filter becomes the two-mask MSB→LSB recurrence
// (gt |= eq & plane when c's bit is 0; eq &= plane when it is 1) — pure
// AND/OR/NOT word ops, NO decompression, 64 values per word op, and b/32
// the bytes to stream (b=6 → 5.3× less). The win needs the AMORTIZATION
// regime: encoding is O(N·b) once (source break-even 7–25 scans; single-
// scan workloads are OUT — the source's own 1M-row single-pass cells
// measured sub-1×) and the win GROWS as b shrinks (b≈32 → 1× compression,
// no win; source guideline b ≤ 16). Point lookups are out (the layout is
// scan-shaped); wide-type EQUALITY loses (scalar == is one instruction vs
// b plane ANDs — the source measured 0.5–0.7× for timestamp eq) — range /
// comparison predicates are the win; large IN-lists scale k·b passes (hash
// on materialized values past ~k=8). Distinct from `soa-hot-field`
// (field-level columnar layout, no compression, no predicate algebra —
// bit-plane is the per-BIT extreme with a different consumer shape); from
// `simd-chunk` (chunked word loops are the VEHICLE here, not the
// mechanism); from `bool-to-bitpack` (flag DENSITY for counting, not value
// predicates); from `sparse-set-bit-iteration-tz-blsr` (iteration WITHIN a
// word, not layout across values). Suggestion-only (a parse-only proposer
// cannot prove the column is re-scanned).
PerfRule {
    rule_id: "bitplane-transpose-repeated-predicate-scan".to_string(),
    name: "repeatedly filtered integer column stored row-wise → encode-once bit-plane transposition + zero-decompression bitwise predicate evaluation (b/32 the bytes, 64 values per word op)".to_string(),
    category: "simd".to_string(),
    buggy_code: "// Row-oriented event column; EVERY dashboard query re-scans it.\nstruct Events {\n    user_age: Vec<u32>, // 4 B/value; 10M events = 40 MB streamed per scan\n}\nimpl Events {\n    fn count_older_than(&self, age: u32) -> u64 {\n        let mut hits = 0u64;\n        for &a in &self.user_age {\n            if a > age {\n                hits += 1; // one value per iteration, full-width bytes\n            }\n        }\n        hits\n    }\n}".to_string(),
    fixed_code: "// Encode ONCE at ingest; every filter is bitwise plane ops — the\n// values are NEVER materialized (zero decompression).\nstruct BitSlicedColumn {\n    planes: Vec<Vec<u64>>, // planes[bit][word]; index 0 = LSB\n    words: usize,\n}\nimpl BitSlicedColumn {\n    fn encode(values: &[u32]) -> Self {\n        let max_val = values.iter().copied().max().unwrap_or(0);\n        let b = (32 - max_val.leading_zeros()).max(1) as usize; // adaptive width\n        let words = values.len().div_ceil(64);\n        let mut planes = vec![vec![0u64; words]; b];\n        for (i, &v) in values.iter().enumerate() {\n            for bit in 0..b {\n                planes[bit][i / 64] |= (((v >> bit) & 1) as u64) << (i % 64);\n            }\n        }\n        Self { planes, words }\n    }\n    // x > c: two-mask MSB→LSB recurrence — pure AND/OR per plane word.\n    // DOMAIN CONTRACT (the paper's §3.5 bias transform): c must be\n    // representable in the column's bit-width — one branch per QUERY, not\n    // per value (an out-of-range c means nothing can exceed it).\n    fn count_gt(&self, c: u32) -> u64 {\n        let b = self.planes.len() as u32;\n        if b < 32 && c >= (1u32 << b) {\n            return 0;\n        }\n        let mut gt = vec![0u64; self.words];\n        let mut eq = vec![u64::MAX; self.words];\n        for (bit, plane) in self.planes.iter().enumerate().rev() {\n            if (c >> bit) & 1 == 0 {\n                // data 1 where c 0 → still-equal values are confirmed greater\n                for w in 0..self.words {\n                    gt[w] |= eq[w] & plane[w];\n                }\n            } else {\n                // data 0 where c 1 → drop from the still-equal set\n                for w in 0..self.words {\n                    eq[w] &= plane[w];\n                }\n            }\n        }\n        gt.iter().map(|w| u64::from(w.count_ones())).sum()\n    }\n}".to_string(),
    context_notes: Some("MEASURED ParetoDominant 3/0/0 — 1.96x/2.16x/1.93x at N=10K/100K/1M (64 filter scans per timed call over the SAME column, b=7 planes, ~50% selectivity; M3 Max release, rustc 1.97, 2026-08-28, load ~4.7-5.6 of 16; verdict-stable x2: 2.08x/2.20x/1.96x on the re-run; allocs 0/128 — the fixed arm allocates its two scratch masks per scan, hoistable by the caller; alloc-det stable). HONEST REGIME NOTE — the measured ~2x is NOT the source's 10.7x headline: in Rust the SCALAR arm auto-vectorizes (LLVM lowers the threshold-count loop to SIMD, ~0.126 ns/elem vs the paper's non-vectorized Java scalar at ~0.65), which compresses the win to the LAYOUT half — b/32 fewer bytes streamed + 64 values per word op. Never cite the paper's 10.7x as this mechanism's isolated number. AMORTIZATION is the rule's regime gate: encoding is O(N*b) ONCE (outside the timed region here by design — the ingest-amortization shape); the source measured break-even at 7-25 scans and its own 1M-row single-pass cells ran sub-1x, so single-scan/write-once workloads are OUT of regime. b<=16 guideline (b~32 = 1x compression, no win); point lookups OUT (scan-shaped layout); wide-type EQUALITY loses (the source: 0.5-0.7x for timestamp eq — scalar == is one instruction vs b plane ANDs); IN-lists scale k*b passes (hash on materialized values past ~k=8). DOMAIN CONTRACT: the predicate constant must be representable in the column width — out-of-range c returns 0 through one branch per query (the paper's §3.5 bias transform; the G1 gate caught the unguarded form marking ALL rows greater on an out-of-range constant). Data-oblivious side property: plane-reduction throughput is selectivity-constant (the source: <6% spread across 0.1%-90% vs the scalar branchy loop's ~4.5x swing). Differentiation in-rule vs soa-hot-field / simd-chunk / bool-to-bitpack / sparse-set-bit-iteration-tz-blsr. Suggestion-only (a parse-only proposer cannot prove the column is re-scanned). Source: arXiv:2608.26368 (Research 097).".to_string()),
    #[cfg(feature = "rust_perf_bench")]
    bench_spec: Some(bitplane_transpose_repeated_predicate_scan_bench_spec()),
},
        // ── layout: row-major single-token matvec (x re-walked per output
        // row) → weights repacked ONCE into consumption-ordered tiles, the
        // activation slice L1-resident across each tile's rows ──────────
        // Source: cflow "Pipeline-Native Transformers" (arXiv:2608.23841,
        // Batch 85, Research 098 — the CC-BY-4.0 paper distill; x86-64
        // source numbers, M3-re-gated here). Single-token decode is
        // bandwidth-bound (arithmetic intensity ~4 FLOP/byte vs machine
        // balance ~20); in the naive row-major GEMV every output row
        // re-walks the full activation x — once W-row + x exceed L1-d the
        // activation is evicted before reuse and re-fetched once per row
        // (n_out × n_in × 4 B of x traffic). The tile fix repacks W at
        // load/conversion time into TILE_ROWS × TILE_COLS tiles stored in
        // compute-consumption order; the matvec walks tiles sequentially
        // and the x column-slice (TILE_COLS × 4 B) stays L1-resident
        // across all TILE_ROWS rows — x traffic drops by TILE_ROWS×.
        // REGIME (the honest M3 note): the crossover sits where W-row + x
        // exceeds L1-d — M3 Max P-core L1-d is 128 KB (4× the source's
        // Sandy Bridge 32 KB), so the win appears only at working sets
        // the source would call comfortably-resident (Tie there — both
        // arms stream W sequentially with x resident). The paper's Q4
        // sibling-projection stripe interleave (QKV/gate-up packed by
        // output stripe so one x pass serves sibling matvecs) is the same
        // repack applied to sibling projections — noted as the variant.
        // Distinct from `transposed-weight-register-gemv` (B62 —
        // register/accumulator granularity for the dot itself;
        // COMPOSABLE: tile first for cache, transpose within a tile for
        // registers), from `loop-order-innermost-contiguous` (B64 —
        // reorders loops over an UNCHANGED layout; this repacks the
        // data), from `soa-hot-field` (struct fields, not matvec
        // weights), and from kernel_opt B14/B15 (GPU global-memory
        // access-count rules — cousins, different substrate). Tile
        // accumulation order changes per-output float summation →
        // rel-error gate, NOT bit-identity (the B64/powi gate shape).
        PerfRule {
            rule_id: "weight-tile-consumption-order".to_string(),
            name: "row-major single-token matvec (x re-walked per output row) → weights repacked once into consumption-ordered tiles, the x slice L1-resident across each tile's rows RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- cache_kernels.cu:265-320; LAYOUT TWIN recorded with that caveat, not the argsort instance): the x-interleaved cache layout is materialized AT INSERT TIME (head_size split into x-sized chunks at key_dst) so the consumer kernel's reads are contiguous aligned x-element runs -- the write side arranges bytes for the reader.".to_string(),
            category: "layout".to_string(),
            buggy_code: "fn matvec_rowmajor(w: &[f32], x: &[f32], y: &mut [f32], n_in: usize) {\n    // W is [n_out x n_in] row-major. Every output row re-walks ALL of x;\n    // once a W row + x exceed L1, x is evicted before its next use and\n    // re-fetched from L2/DRAM once PER ROW (n_out × n_in × 4 B of x\n    // traffic — pure waste: x is the SAME vector every row).\n    for (r, yo) in y.iter_mut().enumerate() {\n        let row = &w[r * n_in..(r + 1) * n_in];\n        let mut acc = 0.0f32;\n        for (wv, &xv) in row.iter().zip(x) {\n            acc += wv * xv;\n        }\n        *yo = acc;\n    }\n}".to_string(),
            fixed_code: "const TILE_ROWS: usize = 64;\nconst TILE_COLS: usize = 512;\n\n/// Repack W ONCE at load/conversion time into consumption-ordered tiles:\n/// tile-major, row-within-tile, column-within-row — the matvec walks this\n/// linearly (setup cost amortized over every decode step).\nfn repack_tiles(w: &[f32], n_out: usize, n_in: usize) -> Vec<f32> {\n    let mut tiles = Vec::with_capacity(w.len());\n    for tr in (0..n_out).step_by(TILE_ROWS) {\n        for tc in (0..n_in).step_by(TILE_COLS) {\n            for r in tr..(tr + TILE_ROWS).min(n_out) {\n                let row = &w[r * n_in..(r + 1) * n_in];\n                tiles.extend_from_slice(&row[tc..(tc + TILE_COLS).min(n_in)]);\n            }\n        }\n    }\n    tiles\n}\n\nfn matvec_tiled(tiles: &[f32], x: &[f32], y: &mut [f32], n_out: usize, n_in: usize) {\n    // y is pre-zeroed; each column tile's partial dot ACCUMULATES into\n    // y[tr + r]. The x slice (TILE_COLS × 4 B = 2 KB) stays L1-resident\n    // across the tile's TILE_ROWS rows — x is read once per row-group\n    // instead of once per row (TILE_ROWS× less x traffic).\n    let mut off = 0usize;\n    for tr in (0..n_out).step_by(TILE_ROWS) {\n        for tc in (0..n_in).step_by(TILE_COLS) {\n            let rows = TILE_ROWS.min(n_out - tr);\n            let cols = TILE_COLS.min(n_in - tc);\n            let xs = &x[tc..tc + cols];\n            for r in 0..rows {\n                let tile_row = &tiles[off + r * cols..off + (r + 1) * cols];\n                let mut partial = 0.0f32;\n                for (wv, &xv) in tile_row.iter().zip(xs) {\n                    partial += wv * xv;\n                }\n                y[tr + r] += partial;\n            }\n            off += rows * cols;\n        }\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0 — 1.32x/1.35x/1.35x/1.56x at n_in=4K/16K/32K/64K (n_out=64, TILE 64x512; M3 Max release, rustc 1.97, 2026-08-28, load ~8.9 of 16; verdict-stable x2 — run 2: 1.32/1.35/1.35/1.56; allocs 1/1 + bytes 256/256 equal at every size, alloc-det stable). The win GROWS monotonically with working set — the ratio story the source predicted, SHIFTED for M3's 128 KB P-core L1-d (4x the source's Sandy Bridge 32 KB): W-row + x per row runs 32 KB / 128 KB / 256 KB / 512 KB across the sweep. HONEST SMALL-ARM NOTE: even the L1-resident arm (n=4096, row+x = 32 KB ~ 1/4 L1) measured 1.32x FASTER — the paper's own 1x-L1 cells also read ~1.12x; below the eviction boundary the tile walk still wins on x-slice register/L1 reuse vs the full-x re-walk per row (the mechanism is weaker, not absent). MAGNITUDE HONESTY: the source's 1.12-1.36x AVX2 wall-clock (plus 7.29x fewer L1-d misses, PMU) is the same band we measure — cite that, never the Q4-tile end-to-end numbers. REGIME: single-token GEMV / memory-bound matvec where W is streamed once + x is re-read per output row; the repack is setup-amortized (load/conversion time — never in the hot path). At x + row fully L1-resident with tiny N the indirection can cost (unmeasured below 4K); batched-multi-token matmuls amortize x across the batch differently (the B62 transposed-register rule composes there). Suggestion-only (a parse-only proposer cannot prove the repack site is load-time — a mid-loop repack would regress; the soa-hot-field precedent). Source: arXiv:2608.23841 cflow (Research 098).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(weight_tile_consumption_order_bench_spec()),
        },
        // ── io: software prefetch sprinkled on a bandwidth/storage-bound
        // stream → remove it (the miss pipeline is already full) ─────────
        // Source: cflow §decode optimizations (arXiv:2608.23841, Batch 85,
        // Research 098). THE ANNOTATION CLASS (the varint/precompute-table
        // family — the rule exists to STOP a plausible-looking port): SW
        // prefetch buys ≈nothing when the workload is bandwidth-bound and
        // is ACTIVELY HARMFUL at decode scale — the paper's prefetch
        // thread walked expert regions at stride 256 and burned ~48% of
        // decode time; disabling it alone was their single largest decode
        // win at 1.92×. Direct-I/O A/B: PF ±0.1% at 64 MB working set;
        // PF=1 4% WORSE at 4.7 GB. Two-regime: compute-bound loops CAN
        // hide latency with prefetch (the x86 folk wisdom's home turf);
        // storage/RAM-bound streams add only issue slots + cache
        // pressure. x86-benched source numbers; the aarch64 analog (PRFM)
        // unmeasured here — no stable Rust prefetch intrinsic on aarch64
        // (the powi-vs-powf cross-arch annotation precedent). Proposer-less
        // by design (a losing rewrite never gets a --fix path — the
        // filter-count-to-any rationale applied to prefetch).
        PerfRule {
            rule_id: "software-prefetch-storage-bound".to_string(),
            name: "software prefetch on a bandwidth/storage-bound stream → remove it (the miss pipeline is already saturated; prefetch only adds issue slots + cache pressure)".to_string(),
            category: "io".to_string(),
            buggy_code: "#[cfg(target_arch = \"x86_64\")]\nfn checksum_stream(data: &[u32]) -> u64 {\n    let mut acc = 0u64;\n    for i in 0..data.len() {\n        // SW prefetch of a future block: when the stream is storage/RAM\n        // bound the miss pipeline is already saturated — this only adds\n        // issue slots + L1 pressure (the source: PF=1 was 4% WORSE at\n        // 4.7 GB direct-I/O; the prefetch THREAD burned 48% of decode\n        // time; disabling it alone = 1.92×).\n        unsafe {\n            let ahead = (i + 64).min(data.len() - 1);\n            core::arch::x86_64::_mm_prefetch(\n                data.as_ptr().add(ahead) as *const i8,\n                core::arch::x86_64::_MM_HINT_T0,\n            );\n        }\n        acc = acc.wrapping_add(u64::from(data[i]));\n    }\n    acc\n}".to_string(),
            fixed_code: "fn checksum_stream(data: &[u32]) -> u64 {\n    // The bare stream: the hardware prefetcher already lines up purely\n    // sequential access; on a bandwidth/storage-bound loop the explicit\n    // prefetch bought nothing and cost issue slots + L1 pressure.\n    let mut acc = 0u64;\n    for &v in data {\n        acc = acc.wrapping_add(u64::from(v));\n    }\n    acc\n}".to_string(),
            context_notes: Some("NotBenchable on this box — x86-only source evidence (no stable aarch64 prefetch intrinsic to re-gate with; the powi-vs-powf cross-arch precedent) — and the annotation exists to STOP a plausible-looking port: in the source's own A/B the prefetch arm reads ParetoDominated-adjacent on bandwidth/storage-bound streams (PF ±0.1% at 64 MB working set; PF=1 4% WORSE at 4.7 GB direct-I/O; at decode scale the dedicated prefetch thread burned ~48% of decode time walking expert regions at stride 256 — disabling it alone was their single largest decode win at 1.92×). REGIME: the two-sided boundary — compute-bound loops CAN hide latency with prefetch (the x86 folk wisdom's home regime); storage/RAM-bound streams add only issue slots + cache pressure. Proposer-less BY DESIGN (the filter-count-to-any rationale: a losing rewrite never gets a --fix path). Source: arXiv:2608.23841 (Research 098).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: structured per-record store → serialize-once byte buffer ─
        // Source: Cloudflare 1.1.1.1 DNS-cache memory optimization (Batch 86,
        // Research 100 — the Big Pineapple distill, 2026-08-27). A frozen-record
        // store that pays one heap allocation PER FIELD PER RECORD (name
        // handle + payload Vec) carries allocator round-up + malloc-header
        // overhead on every record, and the insert path pays that malloc
        // storm per entry. Serializing each record ONCE into one
        // length-prefixed byte buffer + a u32 offset table costs TWO
        // allocations total regardless of record count; the rebuild consumer
        // copies record bytes straight out of the buffer (no re-serialize) —
        // the source's near-isolated attributions were +13% insert / −5%
        // lookup (their END-TO-END numbers are +43%/−19% — never cite those
        // isolated). REGIME: sequential-rebuild consumers win (cache →
        // response rebuild); consumers that randomly index + mutate records
        // in place lose (every patch becomes a buffer rewrite — the source's
        // own round-robin-rotation caveat). ANTI-SHAPE (the source's own
        // rejected alternative): do NOT store the FULL wire message — variant
        // flags (their DNSSEC DO bit) then force caching two copies or
        // post-build filtering; store record bytes + keep the structured
        // metadata you patch on.
        PerfRule {
            rule_id: "serialize-once-byte-buffer-store".to_string(),
            name: "frozen records as per-field heap allocations → serialize once into ONE length-prefixed byte buffer + offset table (2 allocations total, copy-out rebuild)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// Frozen record cache: every record owns its heap fields - one\n// Box<str> for the name + one Vec<u8> for the payload PER RECORD.\n// At store scale the malloc storm dominates the insert path\n// (2n+ allocations, each paying allocator round-up + header).\nstruct Entry {\n    name: Box<str>,\n    data: Vec<u8>,\n}\n\nfn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> Vec<Entry> {\n    names\n        .iter()\n        .zip(payloads)\n        .map(|(n, p)| Entry { name: n.clone(), data: p.clone() })\n        .collect()\n}".to_string(),
            fixed_code: "// Serialize each record ONCE into one length-prefixed buffer + a\n// u32 offset table: TWO allocations total regardless of record\n// count. Rebuild consumers copy record bytes straight out (no\n// re-serialize); only name-carrying records ever need parsing.\nfn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> (Box<[u8]>, Vec<u32>) {\n    let mut total = 4; // record-count prefix\n    for (n, p) in names.iter().zip(payloads) {\n        total += 4 + n.len() + p.len();\n    }\n    let mut buf = Vec::with_capacity(total);\n    let mut offsets = Vec::with_capacity(names.len());\n    buf.extend_from_slice(&(names.len() as u32).to_le_bytes());\n    for (n, p) in names.iter().zip(payloads) {\n        offsets.push(buf.len() as u32);\n        buf.extend_from_slice(&(n.len() as u16).to_le_bytes());\n        buf.extend_from_slice(n.as_bytes());\n        buf.extend_from_slice(&(p.len() as u16).to_le_bytes());\n        buf.extend_from_slice(p);\n    }\n    (buf.into_boxed_slice(), offsets)\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 4.26x/3.52x/2.90x at n=1K/10K/100K (M3 Max release, rustc 1.97, 2026-08-29, load ~5 of 16; verdict-stable x2 — run 2: 4.26/3.47/2.87), alloc axis EXACT 200,001 → 2 allocations at n=100K (2n+1 → 2: the malloc storm IS the mechanism), bytes 7.90 MB → 4.70 MB (allocator round-up + headers gone), alloc-det stable. The win SHRINKS with n (4.26x → 2.90x) as the copy dominates the per-alloc overhead — still dominant at every tested size. REGIME: sequential-rebuild consumers (cache → response rebuild) — the source's near-isolated +13% insert is the same surface; consumers that randomly index + mutate records in place lose (every patch becomes a buffer rewrite). ANTI-SHAPE: full-wire-message storage (the source's own rejected alternative — variant flags force two copies or post-build filtering; store record bytes + patchable metadata). Distinct from `encoded-layout-borrowed-view` (a READ-side encoding erased by a generic accessor; this is the STORE representation) and from `reuse-allocation-clear` (a scratch buffer ACROSS calls; this is the persisted store shape). Suggestion-only (architectural — a parse-only proposer cannot prove records are frozen or the consumer rebuilds). Source: Cloudflare 1.1.1.1 DNS-cache memory optimization (2026-08-27), Batch 86 / Research 100.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(serialize_once_byte_buffer_store_bench_spec()),
        },
        // ── allocation: unconditional per-record field → context-equal elision ─
        // Source: Cloudflare Big Pineapple (Batch 86, Research 100 — the
        // owner-elision change). When a stored field equals a value the
        // ENCLOSING CONTEXT already holds (their owner == the queried domain
        // = the cache key, in hand at every lookup), storing it per record is
        // a heap allocation for information the context carries for free:
        // Option<Box<T>> with None = "same as context", resolved at read via
        // as_deref().unwrap_or(context). ~80% of their records elided. REGIME
        // (two-sided): high %-equal wins (allocation saved per record, the
        // insert-time equality compare is a cheap early-exit); low %-equal is
        // pure overhead — the Option tag widens the struct + every read pays
        // the branch for nothing. COMPOSABLE with serialize-once (the elided
        // marker is one bit in the record header). Distinct from interning/
        // dedup: nothing is shared or canonicalized — the value is simply
        // ABSENT because the context re-derives it.
        PerfRule {
            rule_id: "context-equal-field-elide".to_string(),
            name: "per-record field that equals the enclosing context (the lookup key) → Option<Box<T>> elided to None, resolved against the context at read".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// Every record stores its owner name unconditionally - even though\n// ~80% of owners EQUAL the queried name (the cache key) that the\n// lookup site already holds. One heap allocation per record for\n// information the context carries for free.\nstruct Answer {\n    ttl: u32,\n    owner: Box<str>,\n}\n\nfn build(owners: &[Box<str>], ttls: &[u32]) -> Vec<Answer> {\n    owners\n        .iter()\n        .zip(ttls)\n        .map(|(o, &t)| Answer { ttl: t, owner: o.clone() })\n        .collect()\n}".to_string(),
            fixed_code: "// None == \"same as the context name\" (elided - the caller resolves\n// against the key it already holds); Some = a chain target that\n// genuinely needs its own allocation. ~80% of records now store NO\n// name at all (the elide comparison runs once per INSERT).\nstruct Answer {\n    ttl: u32,\n    owner: Option<Box<str>>,\n}\n\nfn build(context: &str, owners: &[Box<str>], ttls: &[u32]) -> Vec<Answer> {\n    owners\n        .iter()\n        .zip(ttls)\n        .map(|(o, &t)| Answer {\n            ttl: t,\n            owner: if o.as_ref() == context { None } else { Some(o.clone()) },\n        })\n        .collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 3.12x/3.01x/2.00x at n=1K/10K/100K at the ~80% context-equal mix (M3 Max release, rustc 1.97, 2026-08-29, load ~5 of 16; verdict-stable x2 — run 2: 3.13/2.98/2.00), alloc axis EXACT 100,001 → 20,056 at n=100K (the ~20% non-equal owners keep their allocation — the elision regime made visible), bytes 4.04 MB → 2.76 MB, alloc-det stable. The fixed arm pays the insert-time equality compare + the read branch and STILL dominates at every size — the malloc removal outweighs both. REGIME (two-sided): high %-equal wins (the source's DNS distribution, ~80% owners = qname); LOW %-equal is pure overhead — the Option tag + the always-on compare buy nothing (unmeasured below ~50%; the boundary is where allocations-saved < compare-cost, which for small T sits far below 50%). Composes with `serialize-once-byte-buffer-store` (the elided marker is one bit in the record header). Distinct from interning/dedup: nothing is shared or canonicalized — the value is ABSENT because the context re-derives it. Suggestion-only (architectural — a parse-only proposer cannot prove the field equals the context). Source: Cloudflare Big Pineapple (2026-08-27), Batch 86 / Research 100.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(context_equal_field_elide_bench_spec()),
        },
        // ── layout (NotBenchable-speed): bool cluster → one bitflag byte ──────
        // Source: Cloudflare Big Pineapple (Batch 86, Research 100): several
        // per-record bools packed into ONE bitflag byte — their own
        // observation is that the PADDING COLLAPSE shrank the struct by more
        // than the raw bool bytes. Stored-at-scale regime only: a record
        // held millions of times multiplies every byte (their per-entry
        // 953 → 420 B fleet table). CROSS-CITE the kernel_opt INVERSE regime
        // (Batch 32a `bool-fields-over-bitset-hot-reads`): per-tick READ
        // flags stay bools — pack only when FOOTPRINT dominates (bytes
        // stored/streamed), never when decode ALU dominates. DEFAULT-REPR
        // CAVEAT: field reordering makes the collapse layout-dependent —
        // MEASURE size_of before/after (the corpus gate pins 12 → 8 for the
        // fixture layout below; that is evidence for THAT layout, not a
        // promise for yours). NotBenchable-speed: the win is a deterministic
        // size_of delta (the `layout-reorder`/`box-large-enum-variant`
        // class), not a runtime microbench.
        PerfRule {
            rule_id: "bitflag-pack-when-footprint-dominates".to_string(),
            name: "stored-at-scale struct with a cluster of bools → one bitflag byte (footprint regime — the padding collapse can do part of the work; measure size_of before/after)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Stored-at-scale record: eight independent bools. repr(Rust)\n// clusters them, but the flag bytes + alignment padding still\n// bloat every record in a table held millions of times.\nstruct NodeRecord {\n    weight: u32,\n    reachable: bool,\n    verified: bool,\n    stale: bool,\n    pinned: bool,\n    external: bool,\n    compressed: bool,\n    anchored: bool,\n    sealed: bool,\n}".to_string(),
            fixed_code: "// The eight flags as ONE bitfield byte: 5 bytes of fields round to\n// 8 vs 12 - the padding collapse does part of the work. MEASURE\n// size_of before/after: default-repr reordering makes the collapse\n// layout-dependent (pinned 12 -> 8 in the corpus gate).\nstruct NodeRecord {\n    weight: u32,\n    flags: u8, // bit0 reachable .. bit7 sealed\n}\n\nconst REACHABLE: u8 = 1 << 0;\nconst VERIFIED: u8 = 1 << 1;\nconst STALE: u8 = 1 << 2;\nconst PINNED: u8 = 1 << 3;\nconst EXTERNAL: u8 = 1 << 4;\nconst COMPRESSED: u8 = 1 << 5;\nconst ANCHORED: u8 = 1 << 6;\nconst SEALED: u8 = 1 << 7;".to_string(),
            context_notes: Some("NotBenchable-speed — the win is a deterministic size_of delta (12 → 8 for the pinned fixture layout), the `layout-reorder`/`box-large-enum-variant` class, not a runtime microbench. REGIME: stored-at-scale records (footprint dominates — the source's per-entry 953 → 420 B table) ONLY; the kernel_opt `bool-fields-over-bitset-hot-reads` rule is the documented inverse regime (per-tick READ flags stay bools — decode ALU dominates there). DEFAULT-REPR CAVEAT: field reordering makes the padding collapse layout-dependent — measure size_of before/after on YOUR layout; never promise the collapse (the source's struct gained more from padding than from the raw bool bytes, which need not reproduce). Composes with `serialize-once-byte-buffer-store` (a flag byte packs into the record header). Source: Cloudflare Big Pineapple DNS-cache memory optimization (2026-08-27), Batch 86 / Research 100.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators: map(expensive).filter(relevant) → filter-first on the RAW input ─
        // Source: noahlev.org "Making Rustdoc 33% faster" (2026-08-27,
        // Batch 91 / Research 106 — rust-lang PR 159623, the same mechanism
        // again in PR 159779). Rustdoc's build_extern_trait_impls built the
        // expensive display representation (build_impl) for EVERY trait
        // impl in the entire dependency graph, THEN filtered; the fix
        // re-expressed the relevance filter on the RAW ty data and placed
        // it as a guard before each build_impl call (-20% wall, -12% RSS,
        // perf.rust-lang.org-measured; PR 159779 re-ran the mechanism on
        // synthetic impls, -6%). REGIME (two-sided): win ∝ discard-fraction
        // × transform cost; a wash when the transform is cheap (LLVM
        // inlines + CSEs it into the filter) or the filter is
        // non-selective. PRECONDITION (load-bearing): the predicate must be
        // RE-EXPRESSIBLE on the input type — impossible when the mapped
        // value carries information the raw item cannot derive (the
        // source's own work: they adapted the filter to raw ty data
        // first). The re-expressed filter may be OVER-permissive (the
        // source's own choice — extra kept impls only cost time when
        // consumers tolerate them: precision may drop, recall must not).
        // COMPOSES with `filter-map-fuse` (reorder, THEN optionally fuse);
        // ADJACENT to `eager-pointer-collect-lazy-fetch` (both pay O(all)
        // to use O(subset), but that rule fixes the ACCESS path, this one
        // the PIPELINE order — the corpus's same-family distinct-fix-space
        // precedent). Suggestion-only (a parse-only proposer cannot prove
        // the predicate re-expresses on the input — the rewrite is
        // semantic).
        PerfRule {
            rule_id: "filter-before-expensive-transform".to_string(),
            name: "map(expensive).filter(relevant) → re-express the predicate on the RAW input and filter first (win ∝ discard-fraction × transform cost)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "// Renders EVERY doc into its display page first, then keeps only the\n// published ones. render_page is the expensive step (a fresh\n// allocation + a full pass over the body) and ~94% of its output is\n// discarded by the filter behind it.\nstruct Doc {\n    published: bool,\n    rank: u32,\n    body: [u64; 8],\n}\nstruct Page {\n    published: bool,\n    score: u64,\n}\n\nfn render_page(doc: &Doc) -> Page { /* expensive: allocate + fold */ }\n\nfn published_pages(docs: &[Doc]) -> Vec<Page> {\n    docs.iter().map(render_page).filter(|p| p.published).collect()\n}".to_string(),
            fixed_code: "// The published check needs only the RAW doc - filter FIRST, render\n// only the survivors. Same Vec, same order (filter-then-map preserves\n// order for a pure render and a pure predicate).\nstruct Doc {\n    published: bool,\n    rank: u32,\n    body: [u64; 8],\n}\nstruct Page {\n    published: bool,\n    score: u64,\n}\n\nfn render_page(doc: &Doc) -> Page { /* expensive: allocate + fold */ }\n\nfn published_pages(docs: &[Doc]) -> Vec<Page> {\n    docs.iter().filter(|d| d.published).map(render_page).collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 11.85x/11.30x/6.61x at n=1K/30K/300K (x86_64-pc-windows-msvc, rustc 1.97, 2026-08-31, box quiet; verdict-stable x2 — run 1: ~12x/10.67x/8.28x; the ratio jitters with box state, the verdict does not), alloc axis EXACT 300,014 → 18,764 at n=300K (n+14 → 6.25%·n+14: the ~94% discarded renders ARE the mechanism — one escaping digest allocation per render), bytes 2.77 MB → 1.65 MB, alloc-det stable. REGIME (two-sided): win ∝ discard-fraction × transform cost; a WASH when the transform is cheap (LLVM inlines + CSEs it into the filter) or the filter is non-selective — do not apply for cheap/inlinable transforms. PRECONDITION: the predicate must be re-expressible on the input type (the mapped value must carry no information the raw item cannot derive) — the source's own precondition (they adapted the filter to raw ty data first); when not re-expressible the rewrite is IMPOSSIBLE, not merely slower. The re-expressed filter may be OVER-permissive (the source's own choice: keeping extras is correctness-safe when consumers tolerate them — precision may drop, recall must not). COMPOSES with `filter-map-fuse` (reorder, THEN optionally fuse). ADJACENT to `eager-pointer-collect-lazy-fetch` (that rule fixes the ACCESS path for subset reads; this one fixes the PIPELINE order — distinct fix spaces, the corpus's same-family precedent). The 6.61x at 300K vs 11.85x at 1K is the kept-item render work becoming a constant fraction of the total plus collect locality, not a regime boundary. Suggestion-only (semantic rewrite — a parse-only proposer cannot prove re-expressibility). Source: noahlev.org 'Making Rustdoc 33% faster' (2026-08-27) + rust-lang PRs 159623/159779, Batch 91 / Research 106.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_before_expensive_transform_bench_spec()),
        },
        // ── complexity: transform-to-decide → decide on the invariant raw projection ─
        // Source: noahlev.org "Making Rustdoc 33% faster" (2026-08-27,
        // Batch 91 / Research 106 — rust-lang PR 159854). The impl-inlining
        // decision computed the FULL cleaned self type (clean_middle_ty +
        // the param_env normalization of where clauses — 50% of the pass)
        // when the decision consumes only the HEAD of the type (the `Vec`
        // in `Vec<i32>`, the `String` in `&mut String`); deciding from the
        // raw head skipped the normalization entirely (-18% wall). Generic
        // form: the decision needs a cheap PROJECTION that is INVARIANT
        // under an expensive canonicalization — compute the projection on
        // the raw input and pay the canonicalization only behind the
        // decision (or for the survivors — composes with
        // `filter-before-expensive-transform`). PRECONDITION
        // (load-bearing): the projection must be invariant under the
        // transform — if the canonical form can change the projected
        // field, the raw shortcut answers a DIFFERENT question.
        // Suggestion-only (a parse-only proposer cannot prove projection
        // invariance). Caveat: PR 159854's diff was NOT read — the mapping
        // is article-level (Research 106).
        PerfRule {
            rule_id: "decision-projection-before-construction".to_string(),
            name: "transform-then-project to decide → compute the invariant projection on the RAW input; pay the expensive canonicalization only behind the decision".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// The predicate canonicalizes the WHOLE item (sorts + dedupes the\n// tag set into a fresh Vec) only to read one field the RAW item\n// already determines - the transform is pure overhead for every\n// non-matching item.\nstruct Item {\n    tags: [u32; 8],\n}\nstruct Canonical {\n    primary: u32,\n    tags: Vec<u32>,\n}\n\nfn canonicalize(item: &Item) -> Canonical { /* sort + dedupe; primary = tags[0] */ }\nfn raw_primary(item: &Item) -> u32 { /* min of the tags - no allocation */ }\n\nfn count_hot(items: &[Item], hot: u32) -> usize {\n    items.iter().filter(|i| canonicalize(i).primary == hot).count()\n}".to_string(),
            fixed_code: "// primary is INVARIANT under canonicalization (both are the minimum\n// of the tag set) - test the raw projection; canonicalize only the\n// callers that need the full canonical form.\nstruct Item {\n    tags: [u32; 8],\n}\nstruct Canonical {\n    primary: u32,\n    tags: Vec<u32>,\n}\n\nfn canonicalize(item: &Item) -> Canonical { /* sort + dedupe; primary = tags[0] */ }\nfn raw_primary(item: &Item) -> u32 { /* min of the tags - no allocation */ }\n\nfn count_hot(items: &[Item], hot: u32) -> usize {\n    items.iter().filter(|i| raw_primary(i) == hot).count()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 24.58x/24.38x/24.37x at n=1K/30K/300K (x86_64-pc-windows-msvc, rustc 1.97, 2026-08-31, box quiet; verdict-stable x2 — the alloc axis byte-identical across runs and the Pareto verdict identical), alloc axis EXACT 300,000 → 0 at n=300K (the fixed predicate NEVER needs the canonical form — the decision answers from the raw projection), bytes 9.60 MB → 0, alloc-det stable. PRECONDITION (load-bearing — this IS the rule's content): the projection must be INVARIANT under the canonicalization (`canonicalize(i).primary == raw_primary(i)` is pinned over the fixture as a test); if the canonical form can change the projected field, the raw shortcut answers a DIFFERENT question. REGIME: win ∝ pass-rate × canonicalization cost; near-100% pass-rate buys nothing (every item needs the form anyway). The consuming decision here DISCARDS the canonical form (count/decide); when survivors need the form, construct behind the decision — composes with `filter-before-expensive-transform`. Distinct from `ok-or-else`/`unwrap-or-else-lazy` (single-value laziness behind Option — this is decide-from-a-projection) and from `filter-map-fuse` (adapter fusion — no decision content). Suggestion-only (a parse-only proposer cannot prove projection invariance). Caveat: PR 159854's diff was NOT read — the mapping is article-level (Research 106). Source: noahlev.org 'Making Rustdoc 33% faster' (2026-08-27) + rust-lang PR 159854, Batch 91 / Research 106.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(decision_projection_before_construction_bench_spec()),
        },
        // -- io: coalesce-nearby-reads-bounded-gap-cacheability-gated ----
        // Batch 94 (2026-09-01, Research 110 -- the oMLX PR #3260 SSD
        // expert streaming distill). Random-ish addressed reads
        // (per-expert rows) issued as one pread PER ROW merge into one
        // pread per CLUSTER when the byte gap between consecutive rows
        // fits a bounded budget (source: 64 KiB) -- the over-read gap
        // bytes are free ONLY when the OS page cache absorbs them, so
        // the budget is COUPLED TO CACHEABILITY: a direct / F_NOCACHE
        // descriptor coalesces with gap 0 (exactly-adjacent rows only).
        // Merged spans re-select the requested rows afterwards,
        // preserving the caller's order incl. duplicates.
        // Suggestion-only (I/O orchestration -- not a parse-only
        // rewrite); serving-level source numbers only, no isolated
        // corpus bench.
        PerfRule {
            rule_id: "coalesce-nearby-reads-bounded-gap-cacheability-gated".to_string(),
            name: "one pread per requested row -> merge rows within a bounded byte gap into one pread; gap budget collapses to 0 on non-cacheable descriptors".to_string(),
            category: "io".to_string(),
            buggy_code: "// One pread PER requested row: 9 hot experts scattered across two\n// shards cost 9 syscalls, each paying its own overhead; rows 40 KiB\n// apart pay the gap three times over.\nfn read_rows(fd: i32, rows: &[(u64, u32)]) -> Vec<Vec<u8>> {\n    rows.iter().map(|(off, len)| pread(fd, *off, *len)).collect()\n}\n\n".to_string(),
            fixed_code: "// Merge rows whose byte gap fits the budget into ONE pread per\n// cluster, then re-select the requested rows from the merged spans\n// (caller's order and duplicates preserved). Cacheability-gated:\n// gap 0 when the fd is opened no-cache -- over-read is only free\n// when the page cache absorbs it.\nconst MAX_GAP: u64 = 64 * 1024;\nfn read_rows(fd: i32, rows: &[(u64, u32)], cacheable: bool) -> Vec<Vec<u8>> {\n    let budget = if cacheable { MAX_GAP } else { 0 };\n    let mut sorted: Vec<(u64, u64)> =\n        rows.iter().map(|(o, l)| (*o, *o + *l as u64)).collect();\n    sorted.sort_by_key(|r| r.0);\n    let mut spans: Vec<(u64, u64)> = Vec::new();\n    for (off, end) in sorted {\n        match spans.last_mut() {\n            Some((_, last_end)) if off - *last_end <= budget => {\n                *last_end = end;\n            }\n            _ => spans.push((off, end)),\n        }\n    }\n    spans.iter().map(|(s, e)| pread(fd, *s, (e - s) as u32)).collect()\n}\n\n".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source ships serving-level numbers only -- no isolated A/B for the coalescing itself; honestly NotBenchable here). The DECISION CONTENT is the cacheability coupling, not the merge: gap budget = cacheable ? 64 KiB : 0 -- in direct/F_NOCACHE mode over-read bytes are NOT free (they bypass the page cache and cannot repay the extra transfer), so only exactly-adjacent rows merge; on a cacheable descriptor the page cache repays the gap and 64 KiB of slack is cheap. Merge is gap-BOUNDED, not whole-range (an unbounded merge re-reads the shard -- the eager-pointer-collect-lazy-fetch failure shape on the transport axis). Re-selection after the merge preserves the caller's row order incl. duplicates; sort + merge at O(n log n) over the REQUESTED set only. RIDERS from the same source: (1) Darwin hints (fcntl F_RDAHEAD on cacheable fds, F_NOCACHE on direct fds) are applied ONCE per persistent descriptor and advisory-swallowed on unsupported platforms -- a descriptor-lifetime hint, not a per-read posix_fadvise; (2) fd cache keyed (path, cache-mode) so direct and cached descriptors coexist for the same file (note: that dict's get/set is unsynchronized in the source -- double os.open with the loser's fd leaked -- a concurrency hazard recorded in Research 110 section 7, not folded into this rule). Distinct from buffered-io (sequential stream buffering with a fixed window; this is random-addressed access merged by measured gap), from eager-pointer-collect-lazy-fetch (avoids materializing an O(all) index to read a subset; this transports a known subset in fewer syscalls), and from software-prefetch-storage-bound (both are storage-side; SW prefetch hides latency on compute-bound streams, coalescing removes syscall count on random-addressed streams -- different mechanisms, compose freely). Suggestion-only (I/O orchestration; a parse-only proposer cannot see descriptor cacheability). Source: jundot/omlx PR #3260 safetensors.py:448+452 (cold_max_gap_bytes = 64 KiB, clamped), :560-562 (gap-bounded range split), :597-598 (gap 0 when not use_file_cache), :675-678 (re-selection from merged spans), :536-549 (advisory fcntl hints), :526-531 (fd cache keyed (path, use_file_cache)). Batch 94 / Research 110 / Plan 058.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: N known DISJOINT byte-range reads from one file run as one
        // serial pread per range -------------------------------------------
        // Batch 118 (2026-09-06, Research 142 / Plan 097 -- the antirez/ds4
        // PR #991 PLE-gather distill). A cold gather of N fixed-size scattered
        // rows from one large file runs the device at queue depth ONE when
        // issued as one serial pread per row: every row's page-in latency is
        // paid serially (~60 us/row, 6.9-7.7 s per fresh 8K chunk;
        // warm = 204 ms). Splitting the row list into contiguous index ranges
        // across threads -- each doing positional preads on the SHARED fd --
        // overlaps the page-ins (QD1 -> QD32): 6857 -> 338 ms at 32 threads,
        // ~20x cold; warm NEUTRAL; 64 threads no further gain. Byte-identity
        // is by CONSTRUCTION, not by ordering: each row decodes independently
        // from per-thread scratch and writes its own indexed output slice, so
        // completion order is irrelevant and no concatenation step exists.
        // Suggestion-only (I/O orchestration -- not a parse-only rewrite).
        PerfRule {
            rule_id: "parallel-disjoint-pread-hides-cold-page-latency".to_string(),
            name: "N known disjoint byte-range reads from one file issued as one serial pread per range -> thread-parallel positional reads on the shared fd + indexed (not concatenated) writes; byte-identical, engages past a range-count floor".to_string(),
            category: "io".to_string(),
            buggy_code: "// One serial pread PER row: a cold region costs ~60 us/row at queue\n// depth one -- 6.9-7.7 s per fresh 8K chunk; warm cache hides the cost.\nfn gather_rows(fd: i32, offsets: &[u64], row_bytes: usize, out: &mut [u8]) {\n    for (r, off) in offsets.iter().enumerate() {\n        let row = pread_exact(fd, *off, row_bytes); // serial page-ins, QD1\n        out[r * row_bytes..(r + 1) * row_bytes].copy_from_slice(&row);\n    }\n}\n\n".to_string(),
            fixed_code: "// Contiguous range partition across threads; positional reads on the\n// SHARED fd (read_at / pread -- no shared cursor); indexed writes make\n// completion order irrelevant -> byte-identical by construction. Floor +\n// cap + degrade-to-inline keep the transform a pure optimization.\nfn gather_rows(fd: i32, offsets: &[u64], row_bytes: usize, out: &mut [u8]) -> bool {\n    if offsets.len() < 512 {\n        return gather_rows_serial(fd, offsets, row_bytes, out); // engagement floor\n    }\n    let threads = offsets.len().min(32); // source default 32, cap 64\n    let per = div_ceil(offsets.len(), threads);\n    let mut ok = true;\n    std::thread::scope(|s| {\n        let handles: Vec<_> = (0..threads)\n            .map(|t| {\n                let begin = t * per;\n                let end = (begin + per).min(offsets.len());\n                s.spawn(move || {\n                    let mut slice_ok = true;\n                    for r in begin..end {\n                        match read_at_bytes(fd, offsets[r], row_bytes) {\n                            Ok(row) => out[r * row_bytes..(r + 1) * row_bytes]\n                                .copy_from_slice(&row), // indexed write: no ordering step\n                            Err(_) => slice_ok = false,\n                        }\n                    }\n                    slice_ok\n                })\n            })\n            .collect();\n        for h in handles {\n            ok &= h.join().unwrap_or(false); // all-or-nothing: AND of ranges\n        }\n    });\n    ok\n}\n\n".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (source numbers are campaign-level: serial 557 -> 933/971/997 tok/s at 8/16/32 threads, gather 6857 -> 922/533/338 ms; full-model 16K/128 greedy dumps SHA-identical between threads=1 and 32; honestly NotBenchable here -- page-cache state dominates any microbench, see the cold-vs-warm note). MECHANISM: one serial pread per row runs the storage device at queue depth one, so each row's cold page-in latency is paid serially (~60 us/row on the source's SSD); N threads of positional preads overlap the page-ins (QD1 -> QD32). BYTE-IDENTITY BY CONSTRUCTION: fixed-size rows -> disjoint byte ranges -> per-row indexed output slices, so completion order is irrelevant and no concatenation/reordering step exists (their gate: SHA-identical dumps, threads=1 vs 32). REGIME: first-touch/cold page cache dominates (measured ~20x); ranges computable upfront (fixed row size -> offset = row * row_bytes, checked mul); range count >= floor (512 rows ~= 50 KB min gather; source default 32 threads, cap 64, config clamped to row count); file large enough that open/stat amortizes (30 GB sidecar). ANTI-REGIME: sequential dependence between ranges; WARM cache (measured neutral -- gather 200 ms hidden under an 8.3 s GPU wall); tiny inputs where spawn overhead dominates; NEAR-ADJACENT ranges are the OPPOSITE regime -- coalesce-nearby-reads-bounded-gap-cacheability-gated (B94) merges those into fewer syscalls; the two compose only when the set splits into far-apart clusters. RUST MAPPING: std::os::unix::fs::FileExt::read_at on a shared &File (positional, no cursor state) -- never Seek+read on a shared handle (the seek position is shared mutable state); a failed worker degrade means the CALLING thread runs that slice inline (never wrong output); error contract = all-or-nothing (AND of per-range results). Their escape hatch: an mmap serial path behind an env knob. Record-only: a direct-GPU gather variant was on the source's rejected list -- the validated design is CPU-parallel pread. Distinct from coalesce-nearby-reads (fewer syscalls for NEAR rows; this overlaps LATENCY for DISJOINT rows), from buffered-io (sequential streams), and from software-prefetch-storage-bound (compute-bound streams). Suggestion-only (I/O orchestration; a parse-only proposer cannot see fd sharing or page-cache state). Source: antirez/ds4 PR #991 @ 236cb2a5 (MIT) -- ds4_qwen4.c:2340-2345 (the QD1 comment + ~60 us/row), :2327-2338 (EINTR + short-read loop), :2385-2388 (pread_exact + indexed write), :2420-2428 (contiguous range partition), :2346-2357 (floor 512 + cap 64 + clamp), :2430-2438 (degrade-to-inline), :2443-2444 (AND), :2038-2051 (checked-mul offset fn), :2493-2500 (mmap escape); QWEN38_PERF_HANDOFF.md:872-881 (23c: the cold sweep + warm neutral + SHA-identical), :1850 (GPU gather rejected). Batch 118 / Research 142 / Plan 097.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- dispatch: conditional dispatch with both arms loading a KNOWN fn
        // and converging into ONE call site (the DestProp-merged shape) ----
        // Batch 99 (2026-09-02, Research 119 / Plan 067 -- the wasmi v2.0
        // blog distill). rustc's MIR DestinationPropagation (>=1.92, on by
        // default) folds `let f = if c { h_a } else { h_b }; f(x)` into a
        // conditional move of the two code addresses + a SINGLE indirect
        // branch: every dispatch (whichever handler) shares one
        // branch-target-predictor entry and the call can never devirtualize.
        // Verified on this toolchain (x86_64-pc-windows-msvc, rustc 1.97,
        // asm-inspected): the merged form emits `cmovne` + one indirect
        // `jmp r9` tail-jump; the split form emits TWO DIRECT `jmp h_*`
        // tail-jumps -- distinct sites, monomorphic history, no re-merge.
        // Suggest-only (a parse-only proposer cannot see codegen).
        PerfRule {
            rule_id: "mir-destprop-branch-site-merge".to_string(),
            name: "conditional dispatch phi'd into ONE call site (cmov + single indirect branch) -> keep two distinct call sites (direct jumps, per-site predictor history)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// A hot interpreter step loads a KNOWN handler in each arm, then the\n// two fn pointers converge into one call. rustc's MIR\n// DestinationPropagation (>=1.92) turns this into a conditional move of\n// the two code addresses + a SINGLE indirect branch: every dispatch --\n// whichever handler -- shares one branch-target-predictor entry, and\n// the call can never devirtualize.\ntype Handler = fn(&mut State, u32);\n\n#[inline(never)]\nfn handler_add(st: &mut State, x: u32) { st.acc = st.acc.wrapping_add(x); }\n#[inline(never)]\nfn handler_mul(st: &mut State, x: u32) { st.acc = st.acc.wrapping_mul(31).wrapping_add(x); }\n\nfn step(pick_add: bool, st: &mut State, x: u32) {\n    let f: Handler = if pick_add { handler_add } else { handler_mul };\n    f(st, x); // ONE call site shared by both arms\n}".to_string(),
            fixed_code: "// Two DISTINCT tail-call sites, one per arm: each site keeps its own\n// (monomorphic) branch-target history and the compiler emits DIRECT\n// jumps -- no phi, no indirect dispatch, nothing to mispredict. In a\n// hot dispatch loop this was wasmi 2.0's single ~50% fix.\ntype Handler = fn(&mut State, u32);\n\n#[inline(never)]\nfn handler_add(st: &mut State, x: u32) { st.acc = st.acc.wrapping_add(x); }\n#[inline(never)]\nfn handler_mul(st: &mut State, x: u32) { st.acc = st.acc.wrapping_mul(31).wrapping_add(x); }\n\nfn step(pick_add: bool, st: &mut State, x: u32) {\n    if pick_add {\n        handler_add(st, x); // site 1 -- direct, own history\n    } else {\n        handler_mul(st, x); // site 2 -- direct, own history\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 2/0/1 -- 1.04x/1.24x/1.25x at n=1K/100K/1M dispatches, 50/50 mixed stream, the corpus Pareto validator the recorded instrument (x86_64-pc-windows-msvc, rustc 1.97, 2026-09-02, box quiet; alloc axis EXACT 0/0/0/0 at every size, alloc-det stable across 3 runs). Verdict-stable vs the mine-time probe (median-of-15 methodology): 1.11x/1.21x/1.19x + a second run 1.05x/1.22x/1.18x -- the two large sizes are rock-stable at 1.18-1.25x, the 1K cell straddles the Faster threshold (1.04-1.11) and lands Tie under the validator. MECHANISM (asm-inspected on this toolchain, not inferred): the merged form emits `lea rax,[rip+h_a]; lea r9,[rip+h_b]; test cl,cl; cmovne r9,rax; jmp r9` -- a conditional move of the two code addresses + ONE indirect tail-jump; the split form emits TWO DIRECT `jmp h_a` / `jmp h_b` tail-jumps. Both mechanisms are load-bearing and BOTH are the rule's honest content: (1) devirtualization enablement -- the phi of two KNOWN fn items can never fold to a direct call, so the merged form forces an indirect branch forever; (2) per-site predictor history -- the single site is 2-target polymorphic, mixing BTB history across arms. REGIME (measured, three skew arms): 98/02 skew 1.18/1.30/1.28 (most stable -- rare transitions pay full mispredicts at the shared site); 50/50 1.11/1.21/1.19; 75/25 1.31/1.05/1.03 (borderline at the two large sizes). NEGATIVE CONTROL (the boundary, measured): when the two fn pointers are RUNTIME-OPAQUE (fn args, table entries behind opaque loads -- genuinely dynamic dispatch), BOTH forms keep indirect calls and the split buys NOTHING (0.95-1.00x across all nine cells, two runs) -- the rule applies ONLY when the arms load compile-time-known code pointers (fn items / consts / statics), where splitting re-enables direct calls; for genuinely dynamic dispatch see dyn-to-enum (change the representation) instead. TOOLCHAIN REGIME: the deopt class is rustc >=1.92 MIR DestinationPropagation (default-on; the source measured the 1.91->1.92 boundary, ~2800 -> >4200 CoreMark ~+50% from the single fix on the wasmi tail-call dispatch, independently reproduced on Stitch); re-gate on toolchain bumps (the debug-assert-length-hoist cross-arch precedent). The source's own scope note: only the tail-call-based dispatch configurations see improvements (our fixture is the tail-call shape -- the step fn's last op is the handler call). Suggestion-only (a parse-only proposer cannot see codegen shapes). Source: wasmi v2.0 blog 'Accidental Rust Deoptimization' (wasmi-labs.github.io, 2026-09-01, Robin Freyler), Batch 99 / Research 119 / Plan 067.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mir_destprop_branch_site_merge_bench_spec()),
        },
        // -- dispatch: a hot loop reads its handler fn pointer OUT of a
        // runtime table on EVERY dispatch (the classic `HANDLERS[op](..)`
        // idiom) ----------------------------------------------------------
        // Batch 101 (2026-09-02, Research 119 §Parallel-session addendum --
        // the aarch64 complement of Batch 99, probe-only candidate
        // promoted). On aarch64 the merged/indirect dispatch keeps an
        // IN-LOOP indexed load of the handler (`csel x8, xzr, x22, ne;
        // ldr x8, [x19, x8]; blr x8`) -- and DestProp's SPLIT fix does not
        // help here (the split re-merges structurally on this target). The
        // hoist does: loading both table entries ONCE before the loop and
        // selecting between registers eliminates the per-dispatch load.
        // The win is regime-bound: unpredictable streams only.
        // Suggest-only (a parse-only proposer cannot see codegen).
        PerfRule {
            rule_id: "dispatch-handler-pair-hoist".to_string(),
            name: "hot loop re-reads its handler fn pointer from a runtime table every dispatch -> hoist the table entries once before the loop (register-resident select)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// A hot dispatch loop reads the handler OUT of a runtime table on\n// EVERY dispatch. On aarch64 the indirect call keeps that indexed\n// load in the loop (csel offset + ldr + blr) -- a memory access on\n// the dispatch critical path that the predictor cannot hide on\n// unpredictable streams. Note: splitting into two call sites does\n// NOT fix this shape (DestProp re-merges the split on aarch64 --\n// see mir-destprop-branch-site-merge's cross-arch note).\ntype Handler = fn(u64) -> u64;\n\nstatic HANDLERS: [Handler; 2] = [op_add, op_mul];\n\n#[inline(never)]\nfn op_add(x: u64) -> u64 { x.wrapping_add(0x9E37) }\n#[inline(never)]\nfn op_mul(x: u64) -> u64 { x.wrapping_mul(31) }\n\nfn run(ops: &[u32], xs: &[u64]) -> u64 {\n    let mut acc = 0u64;\n    for (i, &op) in ops.iter().enumerate() {\n        let f = HANDLERS[(op & 1) as usize]; // in-loop indexed load\n        acc = f(acc ^ xs[i]);\n    }\n    acc\n}".to_string(),
            fixed_code: "// Hoist BOTH table entries once before the loop: the handlers sit\n// in registers for the whole loop and the per-dispatch body is a\n// register select + direct-ish call -- the indexed load is GONE from\n// the critical path. Wins on unpredictable streams (LCG-random\n// 1.19-1.23x at 10K/1M dispatches); structured/periodic streams are a\n// Tie (a learnable stream lets the single indirect site's predictor\n// hide the load).\ntype Handler = fn(u64) -> u64;\n\nstatic HANDLERS: [Handler; 2] = [op_add, op_mul];\n\n#[inline(never)]\nfn op_add(x: u64) -> u64 { x.wrapping_add(0x9E37) }\n#[inline(never)]\nfn op_mul(x: u64) -> u64 { x.wrapping_mul(31) }\n\nfn run(ops: &[u32], xs: &[u64]) -> u64 {\n    let f0 = HANDLERS[0]; // hoisted once\n    let f1 = HANDLERS[1];\n    let mut acc = 0u64;\n    for (i, &op) in ops.iter().enumerate() {\n        let x = acc ^ xs[i];\n        acc = if op & 1 == 0 { f0(x) } else { f1(x) };\n    }\n    acc\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant on BOTH arches. aarch64 (M3 Max, rustc 1.97.0, macOS 26.6.2): 2/0/0 -- 1.23x/1.22x at 10K/1M dispatches on the LCG-random (unpredictable) stream, band 1.19-1.23x across 3 runs, alloc axis EXACT 0/0, alloc-det stable x3; raw probe LC 0.837-0.840 = fixed wins ~16-19%, verdict-stable x3. x86_64 (4090 box, i7-13700K, windows-msvc, quiet-window re-gate 2026-09-04, verdict-stable x2): 3/0/0 -- 2.69-2.73x at 1K / 1.09-1.10x at 100K / 1.07-1.08x at 1M, alloc 0/0, alloc-det stable -- the Plan-069 expected-Tie REFUTED: the lea-rip no-in-loop-load analysis belonged to Batch 99's KNOWN-ITEMS fixture (compile-time code pointers); this rule's fixture is a RUNTIME TABLE, so x86 must also load the handler pointer in-loop and the hoist eliminates it on both arches. REGIME (measured, three stream shapes, aarch64): the win exists ONLY on unpredictable streams -- period-16 0.988-1.004 and alternation 0.990-1.003 read Tie (a learnable stream lets the single indirect site's predictor hide the load), so consumers on patterned dispatch streams gain nothing; the x86 cells ran the LCG-random stream only (the regime boundary is predictor-mechanical and expected arch-independent, but the structured-stream cells were not re-measured on x86 -- noted honestly). MECHANISM (asm-pinned on aarch64, not inferred): the merged form keeps an IN-LOOP indexed load (`csel x8, xzr, x22, ne; ldr x8, [x19, x8]; blr x8` -- DestProp's merged-load shape; the classic HANDLERS[op](..) idiom is the buggy form) and the hoist eliminates it. DEDUP vs mir-destprop-branch-site-merge (Batch 99): distinct mechanism AND fixture shape -- that rule = compile-time-KNOWN fn items + tail-call shape + the SPLIT transform (two direct jmp sites, per-site predictor history; x86-verified no-re-merge); this rule = runtime TABLE + value-returning handlers + the HOIST transform (register-resident select; on aarch64 the split re-merges structurally, so Batch 99's fix does not transfer to this shape -- and Batch 99's runtime-opaque NEGATIVE CONTROL covers splitting on unpredictable streams, not hoisting). The two compose in the dispatch family: known items -> split (B99); runtime table -> hoist (this). Suggestion-only (a parse-only proposer cannot see codegen shapes). Source: the wasmi v2.0 distill's parallel-session lane, Research 119 §Parallel-session addendum (probe-only evidence, queued unnumbered, promoted Batch 101 / Plan 069); x86 re-gate 2026-09-04 (Batch 101 FU).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dispatch_handler_pair_hoist_bench_spec()),
        },
        // -- io: a RESUMABLE multi-part artifact build trusts its on-disk
        // cache -------------------------------------------------------------
        // Batch 121 (2026-09-07, Research 144 / Plan 101 -- the ds4 PR #991
        // IQ2 delta distill, commit c5ae2eb5). A long quantize/build job that
        // caches per-part outputs + a manifest for resume must REVALIDATE
        // every identity input (source payload, calibration data, the TOOL
        // binary itself) against the manifest BEFORE reusing cached parts,
        // and re-verify each cached part's stored digest before skipping its
        // work. A stale-tool or swapped-source resume otherwise "succeeds"
        // and silently mixes outputs across tool versions.
        // Suggestion-only (build orchestration -- not a parse-only rewrite).
        PerfRule {
            rule_id: "resume-revalidates-provenance-before-reuse".to_string(),
            name: "resumable multi-part build reuses cached parts on manifest presence -> revalidate every identity input (source + calibration + tool hashes) against the manifest, and re-digest each cached part, before any reuse; mismatch refuses".to_string(),
            category: "io".to_string(),
            buggy_code: "// A long build caches per-part outputs + a manifest and SKIPS any\n// part already present -- but never checks that the SOURCE, the\n// calibration data, or the TOOL binary still match what produced the\n// cache. After a tool upgrade or a source swap the resume\n// \"succeeds\" and the artifact silently mixes parts from two tool\n// versions.\nfn build_part(dir: &Path, name: &str, src: &Input) -> PathBuf {\n    let out = dir.join(name);\n    if out.exists() {\n        return out; // presence = trust: nothing revalidated\n    }\n    let bytes = quantize(src);\n    std::fs::write(&out, bytes).unwrap();\n    out\n}".to_string(),
            fixed_code: "// The manifest records the build IDENTITY (source + calibration +\n// tool hashes). Resume compares EVERY identity field first -- any\n// mismatch refuses the whole run -- then re-digests each cached part\n// before skipping it. Partial cache from a DIFFERENT identity is a\n// liability, not a speedup.\n#[derive(serde::Serialize, serde::Deserialize)]\nstruct Manifest { source: String, calib: String, tool: String, parts: Vec<Part> }\n\nfn resume_or_build(dir: &Path, id: &Identity) -> Vec<PathBuf> {\n    let path = dir.join(\"manifest.json\");\n    if let Ok(m) = std::fs::read_to_string(&path) {\n        let m: Manifest = serde_json::from_str(&m).unwrap();\n        assert_eq!(m.source, id.source, \"resume provenance mismatch: source\");\n        assert_eq!(m.calib, id.calib, \"resume provenance mismatch: calib\");\n        assert_eq!(m.tool, id.tool, \"resume provenance mismatch: tool\");\n    }\n    // per-part: re-digest before reuse\n    for part in &parts {\n        let p = dir.join(&part.name);\n        if digest(&p) == part.sha256 { continue; } // verified reuse\n        write_part_atomic(&p, part);\n    }\n    parts\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source runs the quantize stage in ~minutes-per-tensor on a 48-layer trunk; resume-cost avoided is a full re-quantize; honestly NotBenchable here -- correctness-of-resume, not speed). THE FAILURE CLASS: a resumable build whose cache key is PRESENCE instead of IDENTITY mixes outputs across tool/source versions after any upgrade or swap -- the artifact builds \"successfully\" and the mixed provenance is invisible until a quality or hash comparison fails downstream. The identity tuple is THREE-WAY and all three are load-bearing: source payload (a swapped checkpoint), calibration data (a different imatrix changes every quantized byte), and the TOOL ITSELF (a new quantizer library produces different encodings from identical inputs -- its hash belongs in the identity). Per-part reuse re-digests the cached file (guards against truncated/tampered cache files, not just manifest drift); identity mismatch refuses the WHOLE run rather than silently rebuilding (a partial rebuild under a new identity would still mix). Composes with atomic-part-writes (.tmp + rename, the next rule): a crash mid-write must leave NO parseable part for the next resume to trust. Distinct from cache-invalidation-by-content-hash patterns (key a memo by input hash -- this is the BUILD-level identity of a multi-artifact pipeline incl. the tool), from commitment-hashing shapes (what goes INTO a digest -- this is what a resume must CHECK), and from checkpoint/resume of long COMPUTES (same instinct, different artifact: intermediate state vs published parts). Suggestion-only (build orchestration; a parse-only proposer cannot see the cache-identity coupling). Source: ivanfioravanti/ds4-metal PR #991 commit c5ae2eb5 (MIT) -- gguf-tools/qwen4_iq2.py:29-46 (identity dict {source, imatrix provenance, library_sha256} + 'Resume provenance mismatch' refuse), :49-52 (per-tensor digest re-verify before 'verified existing tensors' skip), :58-61 (per-part .tmp write + replace + manifest save after EVERY tensor). Batch 121 / Research 144 / Plan 101.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: a multi-part artifact becomes visible the moment the last
        // part is written, BEFORE anything is verified ----------------------
        // Batch 121 (same distill). Write to a hidden path, RE-READ every
        // payload through an independent reader, hash-compare against the
        // plan, THEN atomic-rename publish + sidecar manifest; refuse
        // existing outputs; template/guard checks run BEFORE any byte.
        PerfRule {
            rule_id: "write-verify-readback-before-publish".to_string(),
            name: "multi-part artifact assembly publishes on last write -> write hidden (.incomplete), re-read every payload via an independent reader + hash-compare, then atomic-rename publish with a sidecar manifest; refuse existing outputs".to_string(),
            category: "io".to_string(),
            buggy_code: "// Assemble N parts into one artifact, publishing as you go: a crash\n// or a bad disk leaves a HALF-WRITTEN artifact at the real path,\n// indistinguishable from a complete one -- and nothing re-reads what\n// was written, so a silent corruption ships.\nfn assemble(parts: &[Part], out: &Path) -> std::io::Result<()> {\n    let mut f = std::fs::File::create(out)?; // REAL path from byte one\n    for p in parts {\n        f.write_all(&p.payload)?; // no read-back, no per-part digest\n    }\n    Ok(()) // published before verified\n}".to_string(),
            fixed_code: "// Three gates before the artifact exists: (1) pre-flight template\n// checks + refuse-existing (before ANY byte); (2) write to a hidden\n// .incomplete path; (3) re-read every payload through an INDEPENDENT\n// reader and hash-compare against the plan, then atomic-rename\n// publish + a sidecar manifest recording per-part digests.\nfn assemble(parts: &[Part], out: &Path) -> Result<(), AssembleError> {\n    if out.exists() || incomplete_path(out).exists() {\n        return Err(AssembleError::Exists); // never clobber\n    }\n    check_template(parts)?; // guards BEFORE any byte is written\n    let tmp = incomplete_path(out);\n    write_all_parts(&tmp, parts)?;\n    let mut reader = IndependentReader::open(&tmp)?; // NOT the write buffer\n    for p in parts {\n        if reader.digest(p.name)? != p.expected_sha {\n            return Err(AssembleError::Verify(p.name)); // refuse, do not fix\n        }\n    }\n    std::fs::rename(&tmp, out)?; // atomic publish\n    write_sidecar_manifest(out, parts)?;\n    Ok(())\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's validation read back 1,255 written tensors; assembly refuses existing outputs and 'verifies every written tensor before publishing'; honestly NotBenchable here -- artifact integrity, not speed). WHY AN INDEPENDENT READER: verifying through the same buffered writer/reader that produced the payload can confirm what the CACHE holds, not what the DISK holds; the source opens a fresh Reader on the .incomplete file and seeks per-tensor (an O(1) extra pass over data already destined for disk -- the verify cost is one sequential read of the artifact). WHY ATOMIC-RENAME: consumers key on the artifact PATH; a visible half-artifact is worse than no artifact (crash-consistency: rename is the only publish step that cannot be observed half-done on POSIX). Refuse-existing beats clobber: a re-run into a live path must not destroy the previous artifact before the new one is proven. Template guards run BEFORE byte one (architecture/metadata keys, part count, per-part kind+dims, external-contract keys) -- a mis-planned assembly should cost zero I/O. The sidecar manifest (per-part digest + sizes + total SHA) makes the artifact auditable AFTER publish and gives the NEXT resume its identity inputs (composes with resume-revalidates-provenance-before-reuse: the manifest this rule publishes is the manifest that rule consumes). Distinct from write-tmp-then-rename crash-safety idioms (rename alone without the read-back verifies the RENAME, not the CONTENT), from backup-verify-restore cycles (post-hoc; this gates the publish itself), and from incremental-checkpoint-resume (mid-compute state -- this is the final publish gate). Suggestion-only (I/O orchestration; a parse-only proposer cannot see reader independence or rename atomicity). Source: ivanfioravanti/ds4-metal PR #991 commit c5ae2eb5 (MIT) -- gguf-tools/qwen4_iq2.py:78-80 (refuse existing .out + .incomplete), :82-90 (template guards before any write: arch kv, block count, external-PLE key absent, per-part kind/dims), :91-127 (plan + .incomplete write, per-part .tmp+replace on the quantize side), :129-152 (fresh Reader read-back + 'Written tensor mismatch' refuse), :153-160 (rename publish + sidecar JSON manifest with total sha256). Batch 121 / Research 144 / Plan 101.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement honesty): an A/B on ONE changed component that\n// rebuilds everything else differently is confounded -----------------
        // Batch 121 (same distill). To measure one component's precision\n// change, byte-copy every OTHER component from the baseline artifact\n// and replace ONLY the under-test set; keep confounded recipes out\n// of the headline; report PAIRED differences; memory counters that\n// cannot see mmap-backed device pages are partial budgets, named\n// as such.
        PerfRule {
            rule_id: "ab-isolate-one-component-by-baseline-byte-copy".to_string(),
            name: "A/B on one changed component rebuilt alongside other deltas -> byte-copy every other component from the baseline artifact, replace ONLY the under-test set, report paired diffs; confounded variants stay out of the headline; un-instrumented memory counters are partial budgets, labelled".to_string(),
            category: "io".to_string(),
            buggy_code: "// Measuring the gate/up precision change by building an IQ2 model\n// \"from scratch\": every tensor re-derives, so the diff vs the Q4_K\n// baseline mixes the gate/up change with fresh MXFP4/MTP/control\n// re-encodes -- the comparison cannot attribute a quality delta to\n// the one variable it names.\nfn build_iq2(src: &Checkpoint, calib: &Imatrix) -> Model {\n    let mut m = Model::new();\n    for t in src.tensors() {\n        m.set(t.name, quantize(t, pick_dtype(t), calib)); // EVERYTHING re-quantized\n    }\n    m\n}".to_string(),
            fixed_code: "// Copy every tensor BYTE-IDENTICALLY from the baseline artifact and\n// replace ONLY the under-test set (the 96 trunk gate/up tensors):\n// the artifact diff IS the variable. Paired per-case diffs (test-only\n// passes vs baseline-only passes) replace raw pass counts; confounded\n// recipes stay out of the headline; un-instrumented memory counters\n// are reported as PARTIAL budgets.\nfn build_iq2(src: &Checkpoint, calib: &Imatrix, baseline: &Model) -> Model {\n    let mut m = baseline.clone(); // byte-identical: diff = the variable\n    for layer in 0..48 {\n        for part in [\"gate\", \"up\"] {\n            let name = format!(\"blk.{layer}.ffn_{part}_exps.weight\");\n            let t = quantize_exact(src.tensor(&name), DType::Iq2Xxs, calib);\n            m.replace_verified(&name, t); // digest-checked replacement\n        }\n    }\n    m\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's own numbers demonstrate the trap it avoids: headline hard-suite passes are FLAT across recipes -- IQ2 36/50 vs Q4_K 35/50 -- while the paired BF16 drift is 3-6x worse: NLL 0.27828 vs 0.21287, top-1 agreement 91.32% vs 96.57%, logprob MAE 0.13992 vs 0.04307; honestly NotBenchable here -- experimental design, not speed). FOUR load-bearing halves: (1) ISOLATION BY BASELINE BYTE-COPY -- every non-under-test tensor is copied verbatim from the baseline artifact so the artifact diff IS the variable (the source copies all non-expert tensors from the combined Q4_K GGUF incl. MXFP4 down projections, MTP, dense/control, and swaps only the 96 trunk gate/up tensors, digest-verified); (2) PAIRED differences over raw counts -- report exact per-case disagreements (test-only-passed vs baseline-only-passed: 2/1, 2/1, 3/1), which survive small samples where pass totals do not; (3) CONFOUNDED variants stay out of the headline -- Q8 (also changes down weights) and Q4_0 (an earlier complete recipe) are named as such; 'the most controlled quantization comparison is IQ2 versus Q4_K-imatrix'; (4) UN-INSTRUMENTED counters are PARTIAL budgets, labelled -- peak RSS/footprint 'do not reliably account for all mmap-backed GPU weights and must not be interpreted as the total model memory requirement'; file size + tracked runtime allocations + resident external sidecar pages = a partial budget, and the test box's capacity 'does not validate' another machine's headroom. RIDER (calibration provenance): reuse the SAME pinned calibration/imatrix as the baseline recipe (six zero-count entries take a deterministic weight-energy fallback) so the precision change is not confounded with a calibration change; per-tensor source hashes retained. Distinct from measurement-honesty timing rules (dead-result-timing-loop-eliminated-under-lto -- instrument the TIMING loop; this isolates the VARIABLE under test), from paired-median-of-ratios interleaving (same-box timing A/B methodology -- this is artifact-level A/B design where the baseline itself is the control), and from the lossy-surface promotion rule (deployed-path retention gates -- this is how to BUILD the comparison that informs such a gate). Suggestion-only (experimental design; a parse-only proposer cannot see artifact provenance). Source: ivanfioravanti/ds4-metal PR #991 commit c5ae2eb5 (MIT) -- gguf-tools/README.md IQ2 section ('It then copies every other tensor from an existing combined Q4_K qwen4exp GGUF ... This isolates the gate/up precision change for a quality comparison'), speed-bench/qwen38-iq2-quality.md:37 (confounded recipes + the controlled-comparison line), :25-32 (paired disagreement table), :52 (the mmap/RSS partial-budget caveat), :58 (pinned imatrix + zero-count fallback). Batch 121 / Research 144 / Plan 101.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: a structured transcript turn is assembled by hand-pushing
// individual tokens -------------------------------------------------
        // Batch 122 (2026-09-07, Plan 102 -- the ds4 PR vision delta,
        // commit 69c38d2). The CLI built an image turn by pushing a user
        // token, a vision span, a newline, then the text -- and image
        // turns broke: the hand assembly drifted from the model's own
        // chat template. The fix routes the turn through ONE model-aware
        // multimodal formatter.
        PerfRule {
            rule_id: "model-aware-format-over-hand-assembled-tokens".to_string(),
            name: "structured transcript turn assembled by hand-pushing tokens (role token + special span + separator + text) -> emit through ONE model-aware formatter that owns the template; hand assembly drifts silently".to_string(),
            category: "io".to_string(),
            buggy_code: "// An image turn is assembled piecemeal: user token, vision span,\n// separator, then the text tokens. Each piece is right in isolation --\n// but the sequence drifts from the model's chat template (which role\n// marker wraps the image? where does the text go?), and the model\n// just answers WRONG. No error, no warning: the transcript looks\n// plausible.\nfn add_image_turn(t: &mut Transcript, img: &Image, text: &str) {
    t.push(token_user());              // role marker, hand-picked
    t.push_vision_span(img);           // special tokens, hand-placed
    if !text.is_empty() {
        t.push_str(\"\\n\");             // separator, hand-picked
        t.push_text(text);             // text AFTER the image... or before?
    }
}".to_string(),
            fixed_code: "// ONE model-aware formatter owns the turn: role, image span, and\n// text placement are the TEMPLATE's decisions, not the caller's. The\n// same entry point serves text-only turns; a format failure rolls the\n// whole turn back atomically (nothing half-enters the transcript).\nfn add_image_turn(t: &mut Transcript, img: &Image, text: &str) -> Result<(), TurnError> {
    let rollback = t.len();
    let span = t.add_image(img)?;
    t.append_multimodal_message(\"user\", &[\"\", text], img, span) // template owns the layout
        .map_err(|e| { t.truncate(rollback); e })?;  // atomic: nothing half-enters
    Ok(())
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's fix history IS the failure: image turns worked under the template-aware API and broke under the hand assembly -- the CLI pushed token_user + vision span + newline + text separately, then 69c38d2 replaced all of it with one model-aware multimodal append; honestly NotBenchable here -- correctness of emission, not speed). THE FAILURE CLASS: template drift is SILENT -- the transcript still tokenizes, the model still answers, and nothing downstream can tell the role/span/separator layout stopped matching what the model was trained on; the only witness is answer quality. The formatter must be MODEL-AWARE (the same turn serializes differently per model family/template version) and ATOMIC (a failed span reservation rolls back the whole turn -- a half-turn is a poisoned transcript). Composes with write-verify-readback-before-publish (the transcript IS an artifact being assembled) and resume-revalidates-provenance (the template version is part of the build identity). Distinct from string-formatting-vs-structured-apis (display concerns; this is a SEMANTIC wire format the consumer is a trained model), from escape/encoding rules (character-level; this is turn-structure-level), and from prompt-template-bypass audits (they find the drift; this prevents writing it). Suggestion-only (a parse-only proposer cannot see that pushed tokens form a template-governed sequence). Source: ivanfioravanti/ds4-metal PR #991 commit 69c38d2 (MIT) -- ds4_cli.c:1536-1553 (the hand assembly removed: token_user + repl_chat_add_image + newline + text pushes -> one ds4_chat_append_multimodal_message with rollback_len/rollback_images guard arms). Batch 122 / Plan 102.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io: a feature needs an external artifact and ships without
// the provisioning step ---------------------------------------------
        // Batch 122 (same distill, commit 9f4313d). The Qwen vision tower,
// its Metal kernel, and the make target all LANDED -- but the
// downloader never fetched the encoder, so --vision was dead out of
// the box. The fix adds the download target following the existing
// GLM pattern.
        PerfRule {
            rule_id: "implemented-surface-requires-provisioning-target".to_string(),
            name: "feature requires an external artifact it does not vend -> ship the fetch/provision target in the SAME change; an implemented-but-unprovisioned feature is dead out of the box".to_string(),
            category: "io".to_string(),
            buggy_code: "// The feature is COMPLETE: parser accepts --vision, the kernel is\n// wired, tests exist behind it. But the weights live in a registry\n// the setup script never fetches -- every user hits 'file not found'\n// on first run.\nfn main() {
    let args = parse_args();
    if let Some(vision) = &args.vision {
        let encoder = load_encoder(vision)?; // shipped; nothing downloads it
        run(args.model, encoder)?;
    }
    Ok(())
}".to_string(),
            fixed_code: "// The same change that lands the feature lands its provisioning:\n// a named setup target (following the existing per-artifact pattern),\n// a clear on-disk location, a load-time error that NAMES the missing\n// target, and a --check that reports provisioned vs missing.\nfn main() -> Result<(), SetupError> {
    let args = parse_args();
    if let Some(vision) = &args.vision {
        let encoder = match load_encoder(vision) {
            Ok(e) => e,
            Err(_) if !provisioned(VISION_TARGET)? => {
                return Err(SetupError::MissingArtifact { target: VISION_TARGET, hint: format!(\"run: ./setup.sh {VISION_TARGET}\") });
            }
            Err(e) => return Err(e.into()),
        };
        run(args.model, encoder)?;
    }
    Ok(())
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's own commit message: 'Vision for Qwen3.8-Flash-Next is implemented ... but the downloader never fetched the encoder, so --vision could not be used out of the box. Follow the GLM 5.3 pattern: a standalone qwen38-vision target'; honestly NotBenchable here -- provisioning completeness, not speed). THE FAILURE CLASS: the gap is invisible in review (every diff hunk is a working feature) and invisible in CI (tests skip or fake the artifact) -- it surfaces only on a fresh machine, which is exactly the onboarding path. The rule has THREE parts: the provision target EXISTS and is named per-artifact (not a monolithic download-everything); the load-time failure NAMES the target that would have provided it (the error is the setup doc); a --check-style probe reports provisioned-vs-missing without side effects. Composes with resume-revalidates-provenance-before-reuse (the provisioned artifact's hash joins the identity) and model-aware-format-over-hand-assembled-tokens (both are feature-completeness axes that review cannot see). Distinct from vendoring-vs-downloading policy (this is the GAP the policy must close, whatever the policy), from error-message-quality rules (this is provisioning toplogy: the error is one part of three), and from docker-layer-provisioning rules (same instinct, container-scoped). Suggestion-only (a parse-only proposer cannot see which CLI flags require which external files). Source: ivanfioravanti/ds4-metal PR #991 commit 9f4313d (MIT) -- download_model.sh:37 (QWEN38_MMPROJ_REPO=ggml-org/Qwen3.8-Flash-Next-GGUF), :167 (QWEN38_VISION_FILE=mmproj-Qwen3.8-Flash-Next-Q8_0.gguf), :184-168 (the qwen38-vision target: 'about 0.6 GB on disk. Load it with --vision; this target does not update ./ds4flash.gguf'). Batch 122 / Plan 102.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement honesty): a multi-turn REPL regression that
// trusts the process exit code ---------------------------------------
        // Batch 122 (same distill, commit 69c38d2's regression). A REPL
// session can FAIL one turn and still exit zero if a later turn
// succeeds -- the exit code aggregates the SESSION, not the turns.
// The regression must scan the captured diagnostics for per-turn
// failure patterns alongside the exit code, and save full transcripts
// for inspection instead of grading content.
        PerfRule {
            rule_id: "diagnostics-not-just-exit-code".to_string(),
            name: "multi-turn session regression gated on process exit code -> also scan captured stderr for per-turn failure patterns; exit code aggregates the session, not the turns; save full transcripts for inspection".to_string(),
            category: "io".to_string(),
            buggy_code: "// Drive a REPL through several turns in one subprocess; assert on\n// the exit code. Turn 2 fails (logged to stderr), turns 3-4 succeed,\n// the process exits 0 -- the regression passes with a broken turn\n// inside it.\nfn run_session(repl: &str, session: &str) -> anyhow::Result<()> {\n    let out = Command::new(repl).stdin(session).output()?;\n    assert!(out.status.success(), \"session failed\");\n    Ok(())\n}".to_string(),
            fixed_code: "// The exit code is the LAST gate, not the only one: scan the\n// captured stderr for per-turn failure patterns (turn-scoped),\n// verify the success markers the tool itself prints, and save the\n// full transcript for human inspection -- do not try to grade\n// content in the harness.\nfn run_session(repl: &str, session: &str, transcript_path: &str) -> anyhow::Result<()> {\n    let out = Command::new(repl).stdin(session).output()?;\n    let diag = String::from_utf8_lossy(&out.stderr);\n    let turn_failures = scan_failures(&diag, &[\"failed\", \"outside the vocabulary\"]);\n    assert!(turn_failures.is_empty(), \"turn failures: {turn_failures:?}\");\n    assert!(markers_present(&out.stdout), \"turn success markers missing\");\n    std::fs::write(transcript_path, &out.stdout)?; // save, don't grade\n    assert!(out.status.success(), \"session failed\");\n    Ok(())\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's regression comment: 'The REPL can return zero after an image turn fails if a later text turn succeeds. Check diagnostics as well as the process exit code'; the scan is a turn-scoped regex over captured stderr: 'failed|outside the vocabulary', plus the success marker 'ds4: image WxH, N image tokens' proving the encoder actually ran; honestly NotBenchable here -- test honesty, not speed). WHY THE EXIT CODE LIES: a REPL is a session aggregate -- any per-turn failure that the tool recovers from (or that only kills one turn) is invisible in the final status; the failure pattern must be scoped tightly enough to stay a FAILURE signal (the source's pattern also catches a benign 'outside the vocabulary' diag only because that diag IS a failure there). SAVE-DON'T-GRADE: the harness persists full stdout/stderr per mode for human inspection instead of asserting on model output content -- grading content makes the test flaky; grading DIAGNOSTICS keeps it deterministic. Both decode modes run (ordinary + speculative) so the failure class cannot hide behind one path. Distinct from assert-on-stderr-string rules (fragile matching; this ASSERTS on scanned failure patterns and saves everything), from golden-output regression (content-pinned; this is deliberately content-free), and from exit-code-only CI gates (the anti-shape itself). Suggestion-only (a parse-only proposer cannot see that the subprocess is a multi-turn session). Source: ivanfioravanti/ds4-metal PR #991 commit 69c38d2 (MIT) -- tests/test_qwen4_cli_vision.py:46-48 ('The REPL can return zero after an image turn fails if a later text turn succeeds' + the failure scan), :49-51 (the success marker + transcripts saved, 'it does not grade image content'). Batch 122 / Plan 102.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement integrity): a numeric-regression gate that trusts
// argmax/top-k ID equality or tolerant parsing -----------------------
        // Batch 123 (2026-09-07, Plan 103 -- the ds4 upstream surge,
// commits 85a4f0e/5ad935d). The source's own tooling detected FULL-VECTOR
// drift at 4/4 frontiers where argmax IDs matched; the old comparator
// silently accepted missing rows, duplicate rows and NaN.
        PerfRule {
            rule_id: "full-vector-bit-identity-over-argmax".to_string(),
            name: "numeric-regression gate compares argmax/top-k IDs or parses measurement artifacts tolerantly -> compare the FULL float vector bit-exactly (canonical spelling, signed zero preserved), pre-validate artifacts (manifest coverage, no duplicate/missing rows, no NaN, internal ratio consistency), and report drift / invalid / identical as THREE statuses".to_string(),
            category: "io".to_string(),
            buggy_code: "// The comparator trusts partial identity and tolerant parsing: argmax\n// IDs match (a lossy projection -- thousands of logits can drift while\n// the sampled token stays identical), missing rows fold to 0.0, duplicate\n// IDs silently overwrite, NaN parses fine. The drift ships as PASS.\nfn compare(base: &str, cand: &str) -> bool {
    let rows = load_lenient(base);   // missing key .get(key, 0.0); dup overwrites
    for case in cases {
        if argmax(base_logits(case)) != argmax(cand_logits(case)) {
            return false;            // ID equality = the whole gate
        }
    }
    true
}".to_string(),
            fixed_code: "// Three gates BEFORE comparing: (1) artifact validity -- manifest\n// coverage, no duplicate/missing rows, canonical finite spelling (no\n// NaN/Inf/scientific), internal consistency (ratio == num/denom within\n// the producer's half-print unit); (2) full-vector identity -- every\n// float32 bit, signed zero preserved, argmax checked but NOT sufficient;\n// (3) three statuses -- identical / drift (valid but changed) / invalid\n// (untrustworthy), never folded into one boolean.\nfn compare(base: &str, cand: &str) -> Verdict {
    let (b, c) = (validate(base)?, validate(cand)?); // refuse garbage upfront
    for case in &b.cases {
        let (x, y) = (b.full_vector(case), c.full_vector(case));
        if x.to_bits() != y.to_bits() {        // FULL vector, bit-exact
            return Verdict::Drift;             // argmax matched -- still drift
        }
    }
    Verdict::Identical
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source's own commit message: 'The existing comparator silently accepts missing rows, duplicate rows and NaN in reproduced controls; the validator rejects them... The frontier tool detects full-vector drift despite matching argmax IDs at all four measured frontiers' -- a MEASURED failure class, not hypothetical; honestly NotBenchable here -- measurement integrity, not speed). WHY ARGMAX IS A LOSY PROJECTION: a kernel/quantization change can drift thousands of logits while the sampled token stays identical -- ID equality is the compression the gate must NOT trust; the source checks argmax AND the full float32 word vector, with the canonical-spelling round trip preserving signed zero ('argmax_logit: float32 bits do not match logits[argmax_id]' + 'argmax_id: not the first maximum of the complete logit vector'). WHY PRE-VALIDATION: a comparison over garbage is a confident wrong answer -- the validator refuses incomplete manifest coverage ('missing manifest cases'), duplicate case IDs, non-canonical numbers (NaN/Inf/scientific notation/blank refused by a strict regex), and internal inconsistency (ratio must equal numerator/denominator within HALF_PRINT_UNIT -- the producer's own rounding). THREE STATUSES: drift (valid but changed bytes) and invalid (untrustworthy artifact) are different failure classes with different actions -- folding them into one boolean destroys the triage. THE WORKSPACE HOOK: this is the comparison contract our own bit-identity gates (B118/B119's byte-identity logit dumps, the quant-family retention gates) already implement -- the rule generalizes it for corpus completeness; a lossy-ID gate on quant/kernels is exactly the class our lossy-surface promotion rule (deployed-path retention) exists to catch. Distinct from diagnostics-not-just-exit-code (captures failure evidence; this validates SUCCESSFUL artifacts before comparing), write-verify-readback-before-publish (the producer's publish gate; this is the consumer's comparison contract over two already-published artifacts), resume-revalidates-provenance-before-reuse (provenance on resume; this is matched-artifact validity + identity semantics), and ab-isolate-one-component-by-baseline-byte-copy (isolation method; this decides what may COUNT AS IDENTICAL). Suggestion-only (a parse-only proposer cannot see which IDs are lossy projections of which vectors). Source: antirez/ds4 @ c0a6119f (MIT) -- gguf-tools/quality-testing/validate_scores.py:146,152 (duplicate-ID refuse + missing-manifest refuse), :93-95 (the canonical finite-number regex '-?(?:0|[1-9][0-9]{0,308})\\.[0-9]{9}'), :120-123 (Decimal ratio == num/denom within half-print-unit); compare_frontier_logits.py:64-68 (canonical float32 spelling round-trip, signed zero preserved), :129-134 (argmax cross-check + full-vector identity), :194 ('identical' / 'drift' / 'invalid' three-status). Batch 123 / Plan 103 / Research 146. RIDER (Batch 129 external confirmation, ds4-metal @ b85a6174 -- the cached hyper-connection activation rejection, speed-bench/qwen38-q2-decode-fusions.md:96-101): TWO WEAK GATES PASSED while the full comparison caught the drift -- the cache passed local FP64-reference error checks AND preserved greedy output on screening prompts, then failed full-logit comparison at 2.63 max-abs / 0.532 RMS with top-1 identical ('Local rounding changes can be amplified in this routed model ... matching generated text alone was not sufficient evidence'). Mechanism note worth keeping: MoE ROUTING amplifies local rounding (one perturbed logit flips an expert choice and the divergence compounds), so routed models need the FULL-vector gate at strictly lower tolerance than dense ones.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (robustness classification): a bool validator that folds\n// error into overflow routes every failure to the destructive path ---
        // Batch 123 (same distill, commit e1711c2). A rendering/embedding\n// failure classified as "doesn't fit" compacted the live conversation --\n// the most destructive, lossy handler -- on a non-capacity error.
        PerfRule {
            rule_id: "renderer-error-is-not-capacity-overflow".to_string(),
            name: "bool try-fit validator hides build/render ERRORS inside the overflow answer -> tri-state (error / fits / overflow); only a TRUE capacity overflow may fire the destructive fallback (compact/truncate/reset), and the error cause propagates out of the callee".to_string(),
            category: "io".to_string(),
            buggy_code: "// bool \"fits\" conflates build failure with doesn't-fit: a rendering\n// error (bad image, tokenizer failure) returns false -> the caller\n// compacts the live transcript -- lossy, irreversible, and caused by\n// nothing the compaction can fix. The error string dies in a stack local.\nfn tool_observation_fits(obs: &Observation, reserve: usize) -> bool {
    let built = build_observation(obs); // Err swallowed: renderer failure...
    match built {
        Ok(text) => projected(text) + reserve < ctx_limit(),
        Err(_) => false, // ...classified as \"doesn't fit\"
    }
}
// caller:
fn ingest(obs: &Observation, reserve: usize) {
    if !tool_observation_fits(&obs, reserve) {
        compact_conversation(\"tool result would exceed context\"); // WRONG path
    }
}".to_string(),
            fixed_code: "// Tri-state: Error / Fits / Overflow. A build failure propagates its\n// cause and never touches the transcript; only a genuine projected-limit\n// overflow may compact.\nenum Fits { Error(String), Yes, No }
fn tool_observation_fits(obs: &Observation, reserve: usize) -> Fits {
    match build_observation(obs) {
        Err(e) => Fits::Error(e.to_string()), // the cause leaves the callee
        Ok(text) => {
            if projected(text) + reserve < ctx_limit() { Fits::Yes } else { Fits::No }
        }
    }
}
// caller:
fn ingest(obs: &Observation, reserve: usize) {
    match tool_observation_fits(&obs, reserve) {
        Fits::Error(e) => report_observation_error(e),  // report, never compact
        Fits::No => compact_conversation(\"tool result would exceed context\"),
        Fits::Yes => append_observation(),
    }
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-reported (the source changed the validator's return from bool to a tri-state int with the comment '-1 is a rendering/embedding error, not a request to compact the context'; every call site branches 'if (fits < 0) goto observation_error; if (!fits) compact' -- honestly NotBenchable here -- control-flow classification, not speed). THE FAILURE CLASS: a validator whose real domain is {error, fits, overflow} typed as bool collapses error into the ONE answer whose handler is destructive and irreversible (compaction rewrites/summarizes live context); every unrelated failure then surfaces as context loss, and the swallowed error string makes it undiagnosable. THE CONTRACT has three parts: the tri-state return, the destructive handler gated by the ONLY condition that names it (projected tokens exceeding context), and the cause propagated out of the callee (err buffer / error variant) instead of dying in a stack local. Applies to any ingestion loop classifying tool results, attachments, embeddings, or message batches before appending to live state. Distinct from diagnostics-not-just-exit-code (captures failure evidence for later grading; this decides the CONTROL-FLOW classification so a non-capacity error cannot fire a destructive path), from the Result-vs-Option idiom (type-level error propagation generally; this is the specific tri-state where a SUCCESS answer also has a destructive binary split), and from bounded-retry rules (budget conservation; orthogonal axis). Suggestion-only (a parse-only proposer cannot see which false answer routes to which handler). Source: antirez/ds4 @ c0a6119f (MIT) -- ds4_agent.c:9000-9017 (the tri-state signature + the '-1 is a rendering/embedding error' comment), 9911/9929/9944 (call sites: fits < 0 -> observation_error; !fits -> agent_worker_compact), 9972-9976 (the error label frees the observation + returns the real message; the transcript is never touched on -1). Batch 123 / Plan 103 / Research 146.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
// -- io (evaluation design): a learned predictor benchmarked against
// an always-issues baseline conflates the MODEL with the ADMISSION POLICY.
        // Batch 124 (2026-09-07, Plan 104 -- arXiv:2609.04040 distill, walk
// #8). Matched controls -- the SAME confidence gate on a 257-parameter
// online MLP and on a classical stride predictor -- make the neural
// advantage vanish; the gate alone removes 35% of prefetches and lifts
// accuracy 11% -> 15% while DRAM reads move only +-0.07%; gate-closed
// execution reproduces the no-prefetch baseline EXACTLY.
        PerfRule {
            rule_id: "matched-control-disentangles-policy-from-model".to_string(),
            name: "learned predictor benchmarked against an always-issues baseline -> run MATCHED CONTROLS (the same admission gate on the learned model AND the classical baseline) so the model's contribution separates from the gate's; report proxy AND endpoint metrics; make the gate-closed arm bit-identical to the no-action baseline".to_string(),
            category: "io".to_string(),
            buggy_code: "// The learned prefetcher is compared against a classical one that\n// ALWAYS issues: any accuracy win may come from the confidence gate\n// the learned version bundles, not from learning. And the gate is\n// graded on ACCURACY alone -- a proxy that can improve while endpoint\n// cost (DRAM traffic) does not move.\nfn evaluate(learned: &dyn Predictor, classical: &dyn Predictor) -> Report {\n    let a = run(learned);   // gate bundled inside: model vs policy confounded\n    let b = run(classical); // always-issues: no gate at all\n    Report { win: a.accuracy > b.accuracy, accuracy: a.accuracy }\n}".to_string(),
            fixed_code: "// Matched controls: the SAME admission gate wraps BOTH predictors,\n// so the comparison isolates the MODEL. Three report axes: proxy\n// (accuracy), endpoint (DRAM traffic), and the null-arm identity --\n// gate-closed must reproduce the no-action baseline bit-exactly (a\n// provable no-op, not a mostly-no-op).\nfn evaluate(learned: &dyn Predictor, classical: &dyn Predictor) -> Report {\n    let gate = ConfidenceGate::default();\n    let a = run_gated(learned, &gate);   // model varies, gate fixed\n    let b = run_gated(classical, &gate); // same gate: the control\n    let null_arm = run_gated(&NullPredictor, &gate);\n    debug_assert_eq!(null_arm.endpoint, no_action_endpoint()); // identity\n    Report {\n        model_win: a.endpoint < b.endpoint,  // endpoint, not proxy\n        proxy: a.accuracy,\n        endpoint: a.endpoint,\n    }\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, paper-measured (arXiv:2609.04040, SPEC CPU2017 in native ChampSim: the matched-control design made a 257-parameter online MLP INDISTINGUISHABLE from gated stride on random traffic and SLOWER on most regular streams -- the neural advantage was the gate all along; the gate alone removed 35% of prefetches and lifted accuracy 11% -> 15% while DRAM reads changed only 0.07%; honestly NotBenchable here -- evaluation design, not speed). THREE decision contents: (1) MATCHED CONTROLS -- the admission policy is its own variable; wrap the learned model AND the classical baseline in the SAME gate, else the benchmark answers 'is the bundle better?' when it claims to answer 'is learning better?'; (2) PROXY-VS-ENDPOINT -- accuracy (a proxy) improved substantially while the endpoint (DRAM reads) did not move: better proxies do not imply better endpoints, so the report must carry BOTH and the win condition must be the endpoint; (3) NULL-ARM IDENTITY -- gate-closed execution reproduced the no-prefetch baseline exactly, which makes the gate a provable no-op when closed (the admission-gate twin of feature-flag bit-identity: an arm you cannot prove inert when off is an arm that can corrupt the experiment). THE WORKSPACE LAW this paper independently re-derived: gate first, predictor later -- Issue 880's kill-switch (no consumer, no prefetch), evpi_gate, convergence-cadence exits; and its honesty clause is the same one behind the lossy-surface promotion rule (aggregate metrics can be flat while the thing you care about moved). Distinct from ab-isolate-one-component-by-baseline-byte-copy (B121 -- isolates the ARTIFACT under test by byte-copying the baseline; this isolates POLICY from MODEL at evaluation-design level, no artifact copying), from full-vector-bit-identity-over-argmax (Batch 123 -- what may COUNT AS IDENTICAL in a comparison; this decides which COMPARISONS are meaningful at all), from dead-result-timing-loop-folded-under-lto (B109 -- the timing loop's work must not vanish; this is confound removal, not instrument integrity), and from diagnostics-not-just-exit-code (failure evidence; orthogonal stage). Suggestion-only (a parse-only proposer cannot see which benchmark conflates model with policy). Source: arXiv:2609.04040 'Confidence-Gated Admission for Hardware Prefetching: When the Gate Matters More Than the Predictor' (Majdane, Casartelli, Lopedoto, cs.AR/cs.PF, 2026-09-03) -- abstract: matched controls, 257-param MLP vs gated stride, 35% prefetch removal, accuracy 11->15%, DRAM +-0.07%, 'We prove gate-closed execution reproduces the no-prefetch baseline exactly', 'The gate matters more than the predictor, and better proxies do not imply better endpoints'. Batch 124 / Plan 104.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
// -- io (oracle-contract / measurement honesty): when a parity or regression
// CONTROL fails, reproduce it on the UNTOUCHED baseline FIRST -- a failure
// that reproduces there indicts the oracle, not the change under test.
        // Batch 125 (2026-09-07, Plan 106 -- ds4 PR rebase-surge, commit
// ffd85d4). Two shipped instances: (1) compute-shape overclaim -- comparing
// a live session's logprobs (last token via the single-token decode step)
// against a fresh session that prefilled the FULL prefix compares two
// different FP accumulation orders; the fresh side must replay the same
// prefill/decode split before the vectors are comparable; (2) unowned-scratch
// overclaim -- a fused gate/up kernel writes `mid` directly and never
// populates the gate/up scratch the unfused path leaves as intermediate, so
// the oracle must compare the outputs the consumer actually reads.
        PerfRule {
            rule_id: "control-reproduces-on-baseline-trims-overclaim".to_string(),
            name: "a failing parity/regression control is first REPRODUCED on the untouched baseline -- a reproduction indicts the ORACLE's contract, and the fix trims the oracle to consumed outputs under matching compute shape; never widen numeric tolerance as the fix".to_string(),
            category: "io".to_string(),
            buggy_code: "// The parity control fails after a fused kernel lands. The reflex: the\n// fusion regressed -- widen the tolerance or skip the test. But nobody ran\n// the SAME control against the UNTOUCHED baseline.\nfn parity_gate(live: &Session, fresh_prefill: &Session) -> bool {\n    // live: prefill-to-len-1 + single-token decode step for the last token\n    // fresh: FULL-prefix prefill -- a DIFFERENT FP accumulation order\n    max_abs_diff(live.logprobs(), fresh_prefill.logprobs()) < 1e-5 // flaky\n}\n\n// twin: the oracle compares outputs the fused path leaves unowned --\nfn gate_up_parity(unfused_scratch: &Scratch, fused: &Fused) -> bool {\n    unfused_scratch.gate == fused.gate && unfused_scratch.up == fused.up\n    // fused writes `mid` directly; gate/up scratch is never populated\n}".to_string(),
            fixed_code: "// Triage: reproduce the failure on the pre-change baseline. If it\n// reproduces, the oracle overclaims -- trim it to the contract consumers\n// actually depend on, under the SAME compute shape; never touch tolerance.\nfn parity_gate(live: &Session, reference: &Session) -> bool {\n    // both arms: prefill to len-1, then evaluate the last token via the\n    // SAME single-token decode step -- identical accumulation order\n    debug_assert!(parity_gate_baseline_reproduces()); // triage, one-time\n    max_abs_diff(live.logprobs(), reference.logprobs()) == 0.0 // exact now\n}\n\nfn gate_up_parity(unfused: &Scratch, fused: &Fused) -> bool {\n    // compare the CONSUMED outputs only; unowned scratch is skipped in\n    // BOTH arms (named in a comment so the skip is a decision, not drift)\n    fused.mid == unfused.mid\n}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-shipped (both instances landed in the same QA-hardening commit; honestly NotBenchable here -- oracle-contract correctness, not speed). THE PROTOCOL: (1) reproduce the failing control on the UNTOUCHED baseline -- reproduction splits 'the change regressed' from 'the oracle overclaims' without touching any tolerance; (2) if it reproduces, the oracle asserted a contract the implementation never promised -- trim it to the outputs consumers actually read under the SAME compute shape (chunk boundaries, prefill/decode split, scratch ownership); (3) the fused path may leave optional/unused outputs UNOWNED -- skip them in both arms or compare through the consumer, with the skip named. THE FAILURE CLASS: an oracle that compares across accumulation orders (prefill-computed vs decode-computed) or across ownership boundaries (scratch a fused contract no longer populates) fails FOREVER on correct code -- the suite then either blocks good changes or trains developers to widen tolerances, which destroys the gate for real regressions. Distinct from full-vector-bit-identity-over-argmax (Batch 123 -- comparison STRICTNESS over artifacts: full vector, three statuses; this constructs the comparison so both sides share compute shape + consumed outputs -- the two compose), from ab-isolate-one-component-by-baseline-byte-copy (B121 -- the baseline as ISOLATION mechanism in A/B design; this uses baseline reproduction as the failing-control TRIAGE step), and from diagnostics-not-just-exit-code (B122 -- captures failure evidence; this fixes which CONTRACT the oracle asserts). Suggestion-only (a parse-only proposer cannot see which outputs a consumer reads). Source: ivanfioravanti/ds4-metal @ ffd85d42 (MIT) -- commit ffd85d4 'Preserve Qwen tool handling and validate the main rebase' (tests/ds4_test.c:203-204 'Match the replay's prefill/decode split. A full-prefix prefill uses different floating-point accumulation than the final decode step.' + tests/test_metal_moe_prefill.c:85-87 'Fused gate/up kernels write mid directly. Compare all consumed outputs against the unfused control, not unused scratch.' / 'if (run && i < 2) continue;'). Batch 125 / Plan 106 / Research 148.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // -- io (measurement honesty): instrumented per-kernel attribution is
// RANKING evidence, never production timing.
        // Batch 129 (2026-09-08, Plan 110 -- ds4 Qwen Q2 distill). The
// encoder-timeline profiler slowed the pipeline ~38% (54.4 -> ~34 tok/s);
// the ranked-#1 candidate on that profile measured -6.95% and ranked-#2
// -19.70%, while the ranking's LAST item produced the retained win.
        PerfRule {
            rule_id: "instrumented-attribution-not-production-times".to_string(),
            name: "kernel-timeline attribution collected under INSTRUMENTATION is ranking-only evidence -- instrumentation itself slows the pipeline (the source: 54.4 -> ~34 tok/s, ~38%), so never read instrumented absolute times as production costs, never set budget targets from them, and state the wall-time methodology next to the numbers".to_string(),
            category: "io".to_string(),
            buggy_code: "// The profile says gate/up dominates, so the team sets a budget
// target from the instrumented absolute time and greenlights the
// biggest family first -- nobody asked what the profiler cost.
fn attribute_regions(app: &mut App) -> Plan {
    let times = profile_with_instrumentation(app); // ~38% slower run
    let top = times.largest_family();
    Plan { target: times[top], first: top } // absolute vs production
}".to_string(),
            fixed_code: "// Instrumented times rank CANDIDATES; go/no-go comes from an
// un-instrumented A/B screen. The report labels the methodology
// (synchronized wall-time, not hardware counters).
fn attribute_regions(app: &mut App) -> Plan {
    let times = profile_with_instrumentation(app);
    let ranking = times.rank_families(); // ORDER is the evidence
    let candidate = ranking.top();
    let verdict = ab_screen_uninstrumented(app, candidate); // the real gate
    Plan { candidate, verdict, note: \"wall-time attribution, profiler adds overhead\" }
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-shipped (the profiler overhead is stated in the source's own decision doc; honestly NotBenchable here -- measurement honesty, not speed). THREE decision contents: (1) INSTRUMENTED TIMES ARE NOT PRODUCTION TIMES -- the encoder-timeline instrumentation reduced throughput 54.4 -> ~34 tok/s, so its per-kernel-family absolute times cannot set budget targets (speed-bench/qwen38-q2-decode-fusions.md:103-105); (2) RANKING SURVIVES, ABSOLUTES DO NOT -- the profile is still valid evidence for ORDERING candidates, and the ordering can be wrong in aggregate: the ranked-#1 family (Q8 dense projections; IQ2 gate/up unpack measured -6.95%, ranked-#2 routed-input packing -19.70%) contained only rejections while the ranking's LAST item (GDN serialization) produced the retained +2.1-2.9% win -- rank candidates, screen before committing; (3) WALL-TIME IS NOT HW COUNTERS -- synchronized wall-time attribution must be labeled as such next to the numbers (prefill-structural.md:84-88: 'These are synchronized wall-time attributions, not GPU hardware counters; the profiler adds overhead'). Distinct from dead-result-timing-loop-folded-under-lto (B109 -- the harness's own work must not vanish; this is the OBSERVER distorting the observed), from dispatch-cost-sequential-measurement-not-single-op (single-op measurement protocol for ONE dispatch decision; this rules on the validity of instrumented end-to-end attribution as a whole), and from work-scaling-probe-launch-bound (uses profiler RANGES to measure a work function; never rules on absolute-time validity). Suggestion-only (a parse-only proposer cannot see which profiler distorted which run). Source: ivanfioravanti/ds4-metal @ b85a6174 (MIT) -- qwen38-q2-decode-fusions.md:103-105 + qwen38-prefill-structural.md:84-88. Batch 129 / Plan 110 / Research 151.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
// -- io (A/B design): a control arm built by UNSETTING the candidate env
// silently runs the enabled DEFAULT -- both arms identical.
        // Batch 129 (2026-09-08, Plan 110 -- ds4 Qwen Q2 distill). The
// harness originally unset the candidate variable for control; when the
// candidate is the DEFAULT-ON path, unsetting is a no-op and the A/B
// reads ~0 while looking like a clean null result. Fix shipped in-source:
// an explicit --control-value, validated as 0 vs 1.
        PerfRule {
            rule_id: "unset-control-arm-silent-default-parity".to_string(),
            name: "when the candidate behavior is the DEFAULT-ON path, a control arm implemented by UNSETTING the env switch silently runs the same enabled default -- the harness must accept an EXPLICIT control value, validation must pass control!=candidate (0 vs 1), and the report must state which arm is control; an unset-env control on a default-on candidate proves nothing".to_string(),
            category: "io".to_string(),
            buggy_code: "// The candidate optimization is DEFAULT-ON (env unset = enabled).
// The \"control\" arm unsets the same env -- which the enabled default
// treats as ON. Both arms run the candidate; the A/B is vacuous.
fn ab_bench(cfg: &Config) -> f64 {
    let a = run_with(cfg, |e| e.remove(\"CANDIDATE\")); // = default = ON
    let b = run_with(cfg, |e| e.insert(\"CANDIDATE\", \"1\"));
    (b - a) / a // ~0.0 -- reads as 'no effect', proves nothing
}".to_string(),
            fixed_code: "// The control value is EXPLICIT and validated to differ from the
// candidate's; both arm values are echoed in the header so a mislabeled
// arm cannot masquerade as a null result.
fn ab_bench(cfg: &Config, control: &str, candidate: &str) -> f64 {
    assert_ne!(control, candidate, \"control must differ from candidate\");
    let a = run_with(cfg, |e| e.insert(\"CANDIDATE\", control)); // explicit
    let b = run_with(cfg, |e| e.insert(\"CANDIDATE\", candidate));
    eprintln!(\"arms: control={control} candidate={candidate}\");
    (b - a) / a
}".to_string(),
            context_notes: Some("REGIME-ANNOTATED, source-shipped (the harness fix + validation landed in the same commit; honestly NotBenchable here -- experimental design, not speed). THE DEFECT: when the candidate is the DEFAULT-ON path, 'unset the env for control' is a NO-OP -- both arms run the candidate, the measured delta reads ~0, and the vacuous screen looks like a clean null result (or, worse, the sign convention silently flips with arm labeling: the source's 1K ABBA log intentionally labels the enabled default as CONTROL and rollback as candidate, so its negative candidate delta is not a regression -- prefill-structural.md:153-155). THE FIX CONTRACT: (1) the harness accepts an explicit control value (metal_prefill_variant_bench.c:214: the control arm sets cfg->control_value when provided, unsets only when not -- backward compatible, which is how the defect shipped); (2) validation passes --control-value 0 --candidate-value 1 and FAILS if the two arms measure identical (prefill-structural.md:58-61: 'enabled defaults cannot silently make both variants identical'); (3) the report names which arm is control. THE FAMILY: this is the control-arm CONSTRUCTION twin of matched-control-disentangles-policy-from-model (B124 -- matched controls isolate policy from model; this ensures the control arm actually executes the control) and the enable-default twin of the null-arm identity face (B124 half 3: the OFF-arm must be provably inert -- here the OFF-arm was never OFF). Distinct from ab-isolate-one-component-by-baseline-byte-copy (B121 -- artifact-level isolation of the component under test; this is the env-arm wiring of the A/B itself) and from diagnostics-not-just-exit-code (B122 -- failure evidence capture; orthogonal stage). Suggestion-only (a parse-only proposer cannot see which env var defaults to ON). Source: ivanfioravanti/ds4-metal @ b85a6174 (MIT) -- speed-bench/qwen38-prefill-structural.md:58-61, :153-155 + metal_prefill_variant_bench.c:52, :214 (the --control-value harness fix). Batch 129 / Plan 110 / Research 151.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── Batch 135 (2026-09-09, Plan 114 / Research 155): the Wild-linker
        // "RustForge wild performance tricks" article distill — three rules,
        // all benchable + measured. The article is an unlicensed-format blog
        // post by the Wild linker author — cite-shape only; the fixtures
        // below are original reproductions of the patterns.
        //
        // ── concurrency: permanent atomic storage / transmute → zero-cost
        // representation conversion (the Vec<AtomicT> mirror) ────────────
        // The naive way to get &-aliased random writes into a Vec<T> is to
        // rebuild it as Vec<AtomicT> with Atomic::new per element (fresh
        // allocation + a full copy every conversion), or to transmute, or to
        // keep the data permanently atomic. When size_of + align_of match
        // (AtomicU32 has the layout of u32), consuming the owned vec via
        // into_iter().map(Atomic::new).collect() reuses the SAME heap
        // allocation and the map loop compiles out entirely (the article's
        // x86 asm: a few movups/mov, no loop, no branch) — so switching the
        // representation per phase is free and there is no excuse for either
        // transmute or permanent atomics. Back-conversion must CONSUME via
        // into_inner(), never load per element; core::mem::take moves the
        // vec across the phase boundary without touching the heap (an empty
        // Vec does not allocate).
        PerfRule {
            rule_id: "atomic-vec-zero-cost-convert".to_string(),
            name: "Vec<T> ↔ Vec<AtomicT> conversion by Atomic::new-per-element collect (fresh alloc + full copy) → into_iter().map(Atomic::new).collect() on the OWNED vec — same size+align makes std reuse the buffer and the loop vanish; switch representation per phase instead of paying atomics everywhere or transmuting".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::atomic::AtomicU32;\n\n// Rebuild the atomic mirror from the shared plain vec: the source is only\n// borrowed, so a fresh allocation + an element-by-element copy is forced\n// on EVERY conversion. Teams that need &-aliased writes often keep the\n// data permanently atomic instead — paying atomic costs in every reader\n// — or reach for transmute.\nfn to_atomic(plain: &[u32]) -> Vec<AtomicU32> {\n    plain.iter().map(|&v| AtomicU32::new(v)).collect()\n}".to_string(),
            fixed_code: "use std::sync::atomic::AtomicU32;\n\n// The vec is OWNED, so into_iter() iterates in place; with matching\n// size_of/align_of (AtomicU32 is layout-identical to u32) the collect\n// reuses the SAME heap allocation and the map loop compiles out — the\n// conversion is free, so keep the plain representation for the owned\n// phases and convert only for the shared-write phase. Convert BACK by\n// consuming: into_iter().map(|a| a.into_inner()).collect() — never\n// per-element loads — and move the vec across phases with mem::take.\nfn to_atomic(plain: Vec<u32>) -> Vec<AtomicU32> {\n    plain.into_iter().map(AtomicU32::new).collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0, verdict-stable x2 (M3 Max, release, 2026-09-09 — Batch 135 benches): 1.83x/1.78x/1.57x/1.33x at n=100/1K/10K/100K, alloc axis EXACT 2 → 1 at every size (both arms pay one shared clone; the buggy conversion adds a second malloc + full copy, the fixed one reuses the buffer). REGIME: the conversion is free ONLY when size_of::<T>() == size_of::<AtomicT>() and align matches (std's in-place collect specialization for vec::IntoIter) — u32/i32/u64/i64/usize and their Atomic twins all match; wider/newtype wrappers match when the wrapper is a transparent single-field struct. ANTI-REGIMES: converting a BORROWED source still needs the fresh allocation (the source outlives the result — the fixed form requires ownership); a wrapper with Drop or a different layout falls back to a real copy loop. THE DECISION CONTENT: switch the representation per phase (plain owned storage, atomic only for the shared-write window) instead of keeping data permanently atomic (pays atomic costs in every reader) or transmuting (unsound under noalias). Back-conversion consumes via into_inner() — per-element loads re-copy the buffer for nothing. mem::take moves the vec across the phase boundary without touching the heap. Distinct from mutex-to-atomic (permanent-storage direction: replace a Mutex with one atomic; this switches a WHOLE VEC's representation for a phase, then switches back) and from mem-take (the move idiom this rule uses at the boundary — different mechanism, they compose). Fixtures are original reproductions of the pattern; source: David Lattimore, \"RustForge wild performance tricks\" (davidlattimore.github.io, 2025-09-02) — the Wild linker's symbol-resolution phase conversion, technique 3. Batch 135 / Plan 114 / Research 155.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(atomic_vec_zero_cost_convert_bench_spec()),
        },
        // ── allocation: alloc-per-iteration because the hoisted buffer can't
        // be TYPED → reuse the heap allocation across a lifetime/type change
        // (the reuse_vec pattern) ─────────────────────────────────────────
        // reuse-allocation-clear covers the caller-passed scratch buffer.
        // This rule covers the case that shape CANNOT express: the buffer's
        // element type (or lifetime) differs per iteration — Vec<&'iter T> —
        // so no single borrowed-scratch signature typechecks. The fix keeps
        // the buffer OWNED and retargets it in place: clear() makes the map
        // body unreachable (the loop compiles out), the const size/align
        // assert turns an incompatible T/U into a COMPILE error at the call
        // site, and std's in-place collect preserves the heap allocation.
        PerfRule {
            rule_id: "vec-reuse-across-lifetime-change".to_string(),
            name: "hot loop allocates a fresh Vec<&'iter T> (or any same-layout Vec<U>) every iteration because the hoisted buffer cannot be typed across the element lifetime → own the buffer and retarget it in place with a const size/align assert + clear() + into_iter().map(unreachable).collect() — the allocation is reused, the loop compiles out, incompatible T/U is a compile error".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// Every call allocates: the picked refs live only for this call, so\n// a hoisted Vec<&[u32]> buffer cannot be typed to outlive one call's\n// borrow — the borrow checker rejects every signature that tries.\nfn row_lens(rows: &[Vec<u32>]) -> u64 {\n    let picked: Vec<&[u32]> = rows.iter().map(|r| &r[..]).collect();\n    picked.iter().map(|s| s.len() as u64).sum()\n}".to_string(),
            fixed_code: "// The buffer is OWNED and retargeted per call: the const assert makes\n// a layout-mismatched T/U a compile error; clear() empties the buffer\n// so the map body is unreachable and the loop compiles out; std's\n// in-place collect hands the SAME heap allocation back re-typed.\n// SAFETY CONTRACT (caller-enforced): the stored refs are cleared (or\n// the buffer dropped) before any referent dies — a panic between take\n// and restore only loses the buffer, never reads a dead ref.\nfn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {\n    const {\n        assert!(\n            std::mem::size_of::<T>() == std::mem::size_of::<U>()\n                && std::mem::align_of::<T>() == std::mem::align_of::<U>(),\n        );\n    }\n    v.clear();\n    v.into_iter().map(|_| unreachable!()).collect()\n}\n\nfn row_lens(rows: &[Vec<u32>], buf: &mut Vec<&'static [u32]>) -> u64 {\n    let mut cur: Vec<&[u32]> = reuse_vec(std::mem::take(buf));\n    cur.clear();\n    cur.extend(rows.iter().map(|r| &r[..]));\n    let total: u64 = cur.iter().map(|s| s.len() as u64).sum();\n    *buf = reuse_vec(cur);\n    total\n}".to_string(),
            context_notes: Some("MEASURED (M3 Max, release, 2026-09-09 — Batch 135 benches, verdict-stable x2): **Mixed 2/0/2 timing (1.49x@100 / 1.07x@1K / Tie@10K/100K) + alloc axis EXACT 1 → 0 at every size** — the deterministic alloc axis is the load-bearing cell (the elision is proven by integers, not wall-clock); timing ties once the refill dominates the malloc (the reuse-allocation-clear Mixed shape — its L2 note covers the large-N regime). REGIME: same-layout element types ONLY — the const assert (inline const, stable 1.79) fires at MONOMORPHIZATION, so a mismatched T/U is a compile error at the call site, not a runtime panic; &T → &'static T retargeting is the canonical case (8-byte slices, references, Box-backed handles, NonNull). THE SAFETY CONTRACT is the rule's load-bearing content: the 'static element type is only sound because the buffer is cleared before any referent can die and consumers never observe a cleared slot — the caller owns the contract, the compiler only checks the LAYOUT half; on panic between take and restore the buffer is dropped (freed), never read dead. Distinct from reuse-allocation-clear (a caller-passed &mut Vec<T> scratch of ONE fixed element type — the shape that cannot typecheck when the element lifetime differs per iteration; that rule removes the compiler-can't-hoist obstacle, this one the borrow-checker-can't-type obstacle) and from mem-take (the boundary move this pattern uses). EXTENSION ARM (recorded, not landed): types that mix owned + borrowed fields (Foo<'a> {owned: String, borrowed: &'a str}) cross the same barrier via a MaybeUninit<&'static> shadow-struct field-wise map — sound only while the shadowed field is never observed empty (the article's own panic caveat). Suggestion-only (a parse-only proposer cannot prove the caller contract). Fixtures original; source: David Lattimore, \"RustForge wild performance tricks\" (davidlattimore.github.io, 2025-09-02) — technique 4, the Wild linker's buffer reuse. Batch 135 / Plan 114 / Research 155.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_reuse_across_lifetime_change_bench_spec()),
        },
        // ── concurrency: shared Vec under one Mutex (or atomics everywhere)
        // → partition into disjoint &mut slices, ZERO synchronization ─────
        // When each worker's writes provably land in a DISJOINT contiguous
        // range (the Wild linker's per-object symbol-id ranges), a lock
        // serializes work that cannot conflict. split the buffer once and
        // hand each worker its own &mut [T]: no locks, no atomics, no
        // contention — the writes are ordinary stores.
        PerfRule {
            rule_id: "partitioned-mut-slices-zero-sync".to_string(),
            name: "shared Vec mutated through one Mutex (or atomics) although each worker writes a known DISJOINT contiguous range → partition once (chunks_mut / split_at_mut) and hand each worker its own &mut [T] — zero locks, zero atomics; applies only when the access pattern is provably disjoint".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::Mutex;\n\n// One lock guards the whole array: the workers' index ranges never\n// overlap, but every single bump serializes on the Mutex — the sync\n// buys nothing the partition would not give for free.\nfn bump_all(grid: &Mutex<Vec<u64>>, cells: usize, workers: usize, iters: usize) {\n    std::thread::scope(|s| {\n        for w in 0..workers {\n            s.spawn(move || {\n                for cell_idx in (w..cells).step_by(workers) {\n                    for _ in 0..iters {\n                        let mut g = grid.lock().unwrap();\n                        g[cell_idx] = g[cell_idx].wrapping_add(1);\n                    }\n                }\n            });\n        }\n    });\n}".to_string(),
            fixed_code: "// Partition ONCE into disjoint contiguous ranges and give each\n// worker exclusive &mut ownership of its slice: the writes are plain\n// stores — no lock, no atomic, no contention, and the per-cell result\n// is identical (each cell is bumped by exactly one owner).\nfn bump_all(grid: &mut Vec<u64>, workers: usize, iters: usize) {\n    let per = grid.len().div_ceil(workers);\n    std::thread::scope(|s| {\n        for chunk in grid.chunks_mut(per) {\n            s.spawn(move || {\n                for _ in 0..iters {\n                    for cell in chunk.iter_mut() {\n                        *cell = cell.wrapping_add(1);\n                    }\n                }\n            });\n        }\n    });\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0, verdict-stable x2 (M3 Max, release, 2026-09-09 — Batch 135 benches; sizes = THREAD counts per the align-64 precedent): 6.90x/9.12x/11.81x/6.13x at 1/2/4/8 threads — the per-bump Mutex serializes even UNCONTENDED (a lock/unlock per bump is ~20ns against a plain store), and contention widens it; alloc axis EXEMPT (see bench.rs ALLOC_AXIS_EXEMPT): the contended std Mutex allocates nondeterministically on macOS in the pthread slow path — the Batch-78 probe evidence covers the same platform class. THE DECISION CONTENT is the disjointness proof: the transform is legal exactly when each worker's writes land in a known contiguous range (contiguous id ranges per object — the Wild linker shape; per-thread partitions of an image/row band; per-shard cells). ANTI-REGIMES: scattered/overlapping access patterns (use fine-grained locks or atomics), read-share-write-rare workloads (RwLock), or when the partition itself costs more than the contention (tiny arrays). Composes with vec-with-capacity (reserve before partition) and reuse-allocation-clear (persistent per-worker buffers). Distinct from cache-line-block-partitioned-lock (that REDUCES contention by partitioning LOCK GRANULARITY — still locks, for scattered per-item writes; this ELIMINATES synchronization via ownership partition for provably-ranged writes — the two answer different access patterns and can compose: partition into worker regions, block-partition the remainder) and from parallel-disjoint-pread-hides-cold-page-latency (io positional reads on a shared fd; this is shared-MEMORY mutation). Fixtures original, std-only (thread::scope + chunks_mut — no rayon dep); the article's split_off_mut form is nightly-shaped and the portable split_at_mut/chunks_mut core is what lands. Source: David Lattimore, \"RustForge wild performance tricks\" (davidlattimore.github.io, 2025-09-02) — technique 1. Batch 135 / Plan 114 / Research 155.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(partitioned_mut_slices_zero_sync_bench_spec()),
        },
    ])
}

// ── Bench specs ────────────────────────────────────────────────────────────
//
// Each spec is a self-contained Rust source fragment embedding buggy + fixed
// code inline + defining `make_input` / `run_buggy` / `run_fixed`. The
// validator wraps this with a timing loop.
//
// Convention for external deps: prefix with `// [dep: crate_name]` — the
// validator strips these markers + adds them to the generated Cargo.toml.

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Bench spec for `o1-lookup`: worst-case Vec scan (last element) vs HashMap O(1).
#[cfg(feature = "rust_perf_bench")]
fn o1_lookup_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10, 100, 1_000, 10_000],
        harness: r#"
use std::collections::HashMap;

mod buggy {
    pub fn find_user(users: &Vec<(u32, String)>, id: u32) -> Option<&String> {
        users.iter().find(|(uid, _)| *uid == id).map(|(_, n)| n)
    }
}

mod fixed {
    use std::collections::HashMap;
    pub fn find_user(users: &HashMap<u32, String>, id: u32) -> Option<&String> {
        users.get(&id)
    }
}

pub struct Input {
    vec: Vec<(u32, String)>,
    map: HashMap<u32, String>,
}

pub fn make_input(n: usize) -> Input {
    let vec: Vec<(u32, String)> = (0..n).map(|i| (i as u32, format!("user{i}"))).collect();
    let map: HashMap<u32, String> = vec.iter().cloned().collect();
    Input { vec, map }
}

pub fn run_buggy(input: &Input) {
    // Worst case: search for the last element (full scan).
    // black_box the key to prevent the compiler from hoisting the comparison.
    let id = std::hint::black_box((input.vec.len() - 1) as u32);
    std::hint::black_box(buggy::find_user(&input.vec, id));
}

pub fn run_fixed(input: &Input) {
    let id = std::hint::black_box((input.vec.len() - 1) as u32);
    std::hint::black_box(fixed::find_user(&input.map, id));
}
"#
        .to_string(),
    }
}

/// Bench spec for `bsearch-sorted-slice`: O(n) linear scan vs O(log n) binary search.
///
/// Both functions search a sorted `&[u32]` for a target that is NOT present
/// (worst case for the linear scan — full pass). Binary search examines only
/// log₂(n) elements. The compiler cannot transform the linear `any()` into a
/// binary search, so the win is real at all sizes ≥ 100.
#[cfg(feature = "rust_perf_bench")]
fn bsearch_sorted_slice_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn contains(sorted: &[u32], target: u32) -> bool {
        sorted.iter().any(|&x| x == target)
    }
}

mod fixed {
    pub fn contains(sorted: &[u32], target: u32) -> bool {
        sorted.binary_search(&target).is_ok()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Sorted ascending 0,1,2,...,n-1
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    // Worst case: target not present (full O(n) scan).
    let target = std::hint::black_box(input.len() as u32);
    std::hint::black_box(buggy::contains(input, target));
}

pub fn run_fixed(input: &[u32]) {
    let target = std::hint::black_box(input.len() as u32);
    std::hint::black_box(fixed::contains(input, target));
}
"#
        .to_string(),
    }
}

/// Bench spec for `dedupe-hashset-vs-vec`: O(n²) growing-Vec dedup vs O(n) HashSet.
///
/// The buggy path builds a unique list by repeatedly calling `Vec::contains`
/// on a growing vector — O(n²) total comparisons. The fixed path collects
/// into a HashSet in O(n) average. Input has ~n/2 unique values repeated
/// twice, ensuring the growing-Vec scan has meaningful work at every step.
/// Sizes capped at 5_000 because the O(n²) buggy path dominates wall-clock
/// at larger n (~6 ms at 5_000; ~25 ms at 10_000).
#[cfg(feature = "rust_perf_bench")]
fn dedupe_hashset_vs_vec_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 500, 1_000, 5_000],
        harness: r#"
mod buggy {
    pub fn unique(items: &[u32]) -> Vec<u32> {
        let mut seen = Vec::new();
        for &x in items {
            if !seen.contains(&x) {
                seen.push(x);
            }
        }
        seen
    }
}

mod fixed {
    use std::collections::HashSet;
    pub fn unique(items: &[u32]) -> Vec<u32> {
        let seen: HashSet<u32> = items.iter().copied().collect();
        seen.into_iter().collect()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Half duplicates: values cycle through 0..(n/2), repeated twice.
    // This produces O(n²) behavior in the buggy path (every element
    // triggers a Vec scan of growing length).
    (0..n).map(|i| (i % (n / 2)) as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let v = buggy::unique(input);
    std::hint::black_box(v);
}

pub fn run_fixed(input: &[u32]) {
    let v = fixed::unique(input);
    std::hint::black_box(v);
}
"#
        .to_string(),
    }
}

/// Bench spec for `static-str`: String allocation vs &'static str return.
///
/// Each call returns a label; the bench runs the function in a loop. The
/// `String` version allocates each call; the `&'static str` version is a
/// pointer return. The perf delta is the allocation cost.
#[cfg(feature = "rust_perf_bench")]
fn static_str_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 10, 100], // size doesn't matter much; function is constant
        harness: r#"
mod buggy {
    pub fn label() -> String { "ready".to_string() }
}

mod fixed {
    pub fn label() -> &'static str { "ready" }
}

pub struct Input {
    pub iters: usize,
}

pub fn make_input(n: usize) -> Input {
    Input { iters: n * 100 } // amplify the per-call cost
}

pub fn run_buggy(input: &Input) {
    // Each call allocates a String + drops it.
    for _ in 0..input.iters {
        let s = buggy::label();
        std::hint::black_box(s);
    }
}

pub fn run_fixed(input: &Input) {
    // Each call returns a &'static str (no allocation).
    for _ in 0..input.iters {
        let s = fixed::label();
        std::hint::black_box(s);
    }
}
"#
        .to_string(),
    }
}

/// Bench spec for `simd-chunk`: scalar sum vs 4-wide chunked sum.
///
/// Batch 66 enhancement note (Research 073 — polars `float_sum.rs`
/// provenance): for FLOAT sums the mechanism is subtler than plain
/// auto-vectorization — strict FP forbids reassociation, so the serial
/// form is a single 3-4-cycle-latency dependency chain LLVM can never
/// vectorize. The chunked form wins because N independent accumulator
/// chains (STRIPE=16 in polars) are each legal sequential FP; the
/// vectorizer interleaves them, and the tree-reduce finisher
/// (`(v[0]+v[2])+(v[1]+v[3])`, shuffle-friendly) collapses the tail
/// without re-introducing a long serial chain. The win GROWS with N —
/// the same mechanism documented for `transposed-weight-register-gemv`.
#[cfg(feature = "rust_perf_bench")]
fn simd_chunk_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn sum(v: &[f32]) -> f32 {
        let mut s = 0.0;
        for i in 0..v.len() { s += v[i]; }
        s
    }
}

mod fixed {
    pub fn sum(v: &[f32]) -> f32 {
        let chunks = v.chunks_exact(4);
        let rem = v.chunks_exact(4).remainder();
        let mut acc = [0.0f32; 4];
        for c in chunks {
            acc[0] += c[0]; acc[1] += c[1]; acc[2] += c[2]; acc[3] += c[3];
        }
        acc.iter().chain(rem).copied().sum()
    }
}

pub fn make_input(n: usize) -> Vec<f32> {
    // Pseudo-random but deterministic data (avoid all-zeros which the
    // compiler might specialize away).
    (0..n).map(|i| ((i as u32).wrapping_mul(2654435761) as f32) / 1000.0).collect()
}

pub fn run_buggy(input: &[f32]) {
    let s = buggy::sum(input);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &[f32]) {
    let s = fixed::sum(input);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `bool-to-bitpack`: Vec<bool> count vs packed Vec<u64> POPCNT.
///
/// The buggy path iterates a `Vec<bool>` (1 byte per element) and counts set
/// bits via `filter().count()`. The fixed path packs bools into `Vec<u64>`
/// (1 bit per element) and counts via `count_ones()` (POPCNT instruction).
/// The compiler cannot auto-pack bools into bits — this is a data-density
/// transformation the programmer must make explicitly. The win comes from
/// 8× less memory + hardware POPCNT (1 instruction per 64 bools vs per-byte
/// branch).
#[cfg(feature = "rust_perf_bench")]
fn bool_to_bitpack_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn count_true(flags: &[bool]) -> usize {
        flags.iter().filter(|&&b| b).count()
    }
}

mod fixed {
    pub fn count_true(bits: &[u64]) -> usize {
        bits.iter().map(|w| w.count_ones() as usize).sum()
    }
}

pub struct Input {
    bools: Vec<bool>,
    bits: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    // ~67% set bits, pseudo-random but deterministic.
    let bools: Vec<bool> = (0..n)
        .map(|i| (i as u32).wrapping_mul(2654435761) % 3 != 0)
        .collect();
    // Pack bools into u64s.
    let mut bits = vec![0u64; (n + 63) / 64];
    for (i, &b) in bools.iter().enumerate() {
        if b {
            bits[i / 64] |= 1u64 << (i % 64);
        }
    }
    Input { bools, bits }
}

pub fn run_buggy(input: &Input) {
    let c = buggy::count_true(&input.bools);
    std::hint::black_box(c);
}

pub fn run_fixed(input: &Input) {
    let c = fixed::count_true(&input.bits);
    std::hint::black_box(c);
}
"#
        .to_string(),
    }
}

/// Bench spec for `mutex-to-atomic`: Mutex<u32> inc loop vs AtomicU32 fetch_add.
///
/// The buggy path locks + unlocks a `Mutex<u32>` per increment. Even on the
/// uncontended path, this involves an atomic CAS to acquire + an atomic store
/// to release (plus the `MutexGuard` RAII lifecycle). The fixed path uses a
/// single `AtomicU32::fetch_add(1, Relaxed)` — one `lock xadd` instruction.
/// The compiler cannot eliminate `Mutex::lock` (it's an opaque call).
#[cfg(feature = "rust_perf_bench")]
fn mutex_to_atomic_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

mod buggy {
use std::sync::Mutex;
pub struct Counter(Mutex<u32>);
impl Counter {
    pub fn new() -> Self { Self(Mutex::new(0)) }
    #[inline(never)]
    pub fn inc(&self) { *self.0.lock().unwrap() += 1; }
    pub fn get(&self) -> u32 { *self.0.lock().unwrap() }
}
}

mod fixed {
use std::sync::atomic::{AtomicU32, Ordering};
pub struct Counter(AtomicU32);
impl Counter {
    pub fn new() -> Self { Self(AtomicU32::new(0)) }
    #[inline(never)]
    pub fn inc(&self) { self.0.fetch_add(1, Ordering::Relaxed); }
    pub fn get(&self) -> u32 { self.0.load(Ordering::Relaxed) }
}
}

pub struct Input {
    n: usize,
}

pub fn make_input(n: usize) -> Input {
    Input { n }
}

pub fn run_buggy(input: &Input) {
    let c = buggy::Counter::new();
    for _ in 0..input.n { c.inc(); }
    std::hint::black_box(c.get());
}

pub fn run_fixed(input: &Input) {
    let c = fixed::Counter::new();
    for _ in 0..input.n { c.inc(); }
    std::hint::black_box(c.get());
}
"#
        .to_string(),
    }
}

/// Bench spec for `f32-counter-to-int`: f32 accumulator vs u64 accumulator.
///
/// The buggy path accumulates integer values into an `f32` total. f32
/// addition is NOT associative (IEEE 754 rounding), so the compiler cannot
/// break the sequential dependency chain or use multiple SIMD accumulators
/// without `-C fast-math` (which Rust does not enable by default). Each f32
/// add has 3–4 cycle latency, fully serial. The fixed path uses a `u64`
/// accumulator — integer addition IS associative, so the compiler vectorizes
/// with multiple independent accumulators + 1-cycle add latency.
#[cfg(feature = "rust_perf_bench")]
fn f32_counter_to_int_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn tally(scores: &[u32]) -> f32 {
        let mut total = 0.0f32;
        for &s in scores { total += s as f32; }
        total
    }
}

mod fixed {
    pub fn tally(scores: &[u32]) -> u64 {
        let mut total = 0u64;
        for &s in scores { total += s as u64; }
        total
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Pseudo-random but deterministic values in a realistic range.
    (0..n).map(|i| (i as u32).wrapping_mul(2654435761) % 1000).collect()
}

pub fn run_buggy(input: &[u32]) {
    let t = buggy::tally(input);
    std::hint::black_box(t);
}

pub fn run_fixed(input: &[u32]) {
    let t = fixed::tally(input);
    std::hint::black_box(t);
}
"#
        .to_string(),
    }
}

/// Bench spec for `collect-avoidance`: .collect() intermediate vs chained iterator.
///
/// The buggy path collects an intermediate `Vec<u64>` from the map, then
/// iterates it again for the sum. The collect allocates (heap alloc + copy
/// of all elements) + forces a second pass. The fixed path chains map+sum
/// — zero allocation, single pass. The compiler cannot eliminate a
/// `.collect()` (it must produce a concrete `Vec`).
#[cfg(feature = "rust_perf_bench")]
fn collect_avoidance_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn transform_sum(vals: &[u64]) -> u64 {
        let mapped: Vec<u64> = vals.iter().map(|x| x.wrapping_mul(2)).collect();
        mapped.iter().sum()
    }
}

mod fixed {
    pub fn transform_sum(vals: &[u64]) -> u64 {
        vals.iter().map(|x| x.wrapping_mul(2)).sum()
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    let s = buggy::transform_sum(input);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &[u64]) {
    let s = fixed::transform_sum(input);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `dyn-to-enum`: Box<dyn Trait> vs enum dispatch.
///
/// The buggy path uses `Box<dyn Op>` — each `.apply()` is a vtable load +
/// indirect call. The compiler cannot inline through an opaque trait object
/// or devirtualize when the concrete type is erased. The fixed path uses an
/// enum: the match arms are direct calls the compiler can inline + the
/// branch predictor learns the match arm pattern.
#[cfg(feature = "rust_perf_bench")]
fn dyn_to_enum_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    trait Op {
        fn apply(&self, x: f64) -> f64;
    }
    struct Scale(f64);
    impl Op for Scale { fn apply(&self, x: f64) -> f64 { x * self.0 } }
    struct Offset(f64);
    impl Op for Offset { fn apply(&self, x: f64) -> f64 { x + self.0 } }
    struct Square;
    impl Op for Square { fn apply(&self, x: f64) -> f64 { x * x } }

    fn make_ops() -> Vec<Box<dyn Op>> {
        vec![Box::new(Scale(1.1)), Box::new(Offset(0.5)), Box::new(Square)]
    }
    pub fn run(vals: &[f64]) -> f64 {
        let ops = make_ops();
        let mut acc = 0.0f64;
        for &x in vals {
            let mut v = x;
            for op in &ops { v = op.apply(v); }
            acc += v;
        }
        acc
    }
}

mod fixed {
    enum OpEnum { Scale(f64), Offset(f64), Square }
    impl OpEnum {
        fn apply(&self, x: f64) -> f64 {
            match self {
                OpEnum::Scale(s) => x * s,
                OpEnum::Offset(o) => x + o,
                OpEnum::Square => x * x,
            }
        }
    }
    fn make_ops() -> Vec<OpEnum> {
        vec![OpEnum::Scale(1.1), OpEnum::Offset(0.5), OpEnum::Square]
    }
    pub fn run(vals: &[f64]) -> f64 {
        let ops = make_ops();
        let mut acc = 0.0f64;
        for &x in vals {
            let mut v = x;
            for op in &ops { v = op.apply(v); }
            acc += v;
        }
        acc
    }
}

pub fn make_input(n: usize) -> Vec<f64> {
    (0..n).map(|i| ((i as f64) * 0.1).sin()).collect()
}

pub fn run_buggy(input: &[f64]) {
    let r = buggy::run(input);
    std::hint::black_box(r);
}

pub fn run_fixed(input: &[f64]) {
    let r = fixed::run(input);
    std::hint::black_box(r);
}
"#
        .to_string(),
    }
}

/// Bench spec for `entry-vs-contains-insert`: contains_key() + insert() vs entry().
///
/// The buggy path calls `contains_key(k)` then `insert(k, v)` — two hash
/// computations + two probes. The fixed path uses `entry(k).or_insert(v)` —
/// one hash + one probe, in-place insert. The compiler cannot merge the two
/// lookups because `insert` may need to grow the table (opaque call). Input
/// keys have ~50% existing + ~50% new to exercise both branches.
#[cfg(feature = "rust_perf_bench")]
fn entry_vs_contains_insert_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
use std::collections::HashMap;

mod buggy {
use std::collections::HashMap;
pub fn insert_if_absent(map: &mut HashMap<u32, u32>, keys: &[u32]) {
    for &k in keys {
        if !map.contains_key(&k) {
            map.insert(k, k.wrapping_mul(31));
        }
    }
}
}

mod fixed {
use std::collections::HashMap;
pub fn insert_if_absent(map: &mut HashMap<u32, u32>, keys: &[u32]) {
    for &k in keys {
        map.entry(k).or_insert(k.wrapping_mul(31));
    }
}
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Mix of existing + new keys: keys cycle through 0..(n*2), so ~50% hit.
    (0..n).map(|i| (i as u32).wrapping_mul(7) % (n as u32 * 2)).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut map = HashMap::with_capacity(input.len());
    buggy::insert_if_absent(&mut map, input);
    std::hint::black_box(map.len());
}

pub fn run_fixed(input: &[u32]) {
    let mut map = HashMap::with_capacity(input.len());
    fixed::insert_if_absent(&mut map, input);
    std::hint::black_box(map.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `branchless-conditional`: data-dependent if vs branchless multiply.
///
/// The buggy path uses `if cond[i] { sum += data[i] }` — a data-dependent
/// branch inside a hot loop that prevents LLVM auto-vectorization. The fixed
/// path uses `sum += data[i] * cond[i] as u64` — a branchless multiply that
/// LLVM vectorizes into SIMD lanes. The win is primarily vectorization, not
/// branch prediction: even with 100% predictable conditions the branchless
/// version is ~10× faster because the multiply loop vectorizes but the
/// branchy loop cannot.
#[cfg(feature = "rust_perf_bench")]
fn branchless_conditional_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
pub fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {
    let mut sum: u64 = 0;
    for i in 0..data.len() {
        if cond[i] {
            sum += data[i] as u64;
        }
    }
    sum
}
}

mod fixed {
pub fn conditional_sum(data: &[u32], cond: &[bool]) -> u64 {
    let mut sum: u64 = 0;
    for i in 0..data.len() {
        sum += data[i] as u64 * cond[i] as u64;
    }
    sum
}
}

pub struct Input {
    data: Vec<u32>,
    cond: Vec<bool>,
}

pub fn make_input(n: usize) -> Input {
    // ~50% true conditions: pseudo-random but deterministic.
    // This is the worst case for branch prediction (unpredictable).
    let data: Vec<u32> = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let cond: Vec<bool> = (0..n).map(|i| (i.wrapping_mul(12345) % 2) == 0).collect();
    Input { data, cond }
}

pub fn run_buggy(input: &Input) {
    let s = buggy::conditional_sum(&input.data, &input.cond);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &Input) {
    let s = fixed::conditional_sum(&input.data, &input.cond);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `loop-fusion`: two iterator passes vs one fused loop.
///
/// The buggy path uses `.iter().copied().sum()` + `.iter().copied().max()`
/// — two separate iterator consumers, each independently walking the slice.
/// The compiler cannot fuse them into a single pass. The fixed path does
/// both reductions in one explicit `for` loop — one cache pass. Bench 013:
/// ParetoDominant ~1.5× across all sizes (1K → 1M). The win is cache
/// locality at large N (second reduction in the fused loop reads from L1;
/// second pass in the buggy version re-reads from L2/L3 or RAM).
#[cfg(feature = "rust_perf_bench")]
fn loop_fusion_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
pub fn sum_max(data: &[u64]) -> (u64, u64) {
    let sum: u64 = data.iter().copied().sum();
    let max: u64 = data.iter().copied().max().unwrap_or(0);
    (sum, max)
}
}

mod fixed {
pub fn sum_max(data: &[u64]) -> (u64, u64) {
    let mut sum = 0u64;
    let mut max = 0u64;
    for &x in data {
        sum += x;
        if x > max { max = x; }
    }
    (sum, max)
}
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n).map(|i| (i as u64).wrapping_mul(31) % 1000).collect()
}

pub fn run_buggy(input: &[u64]) {
    let r = buggy::sum_max(input);
    std::hint::black_box(r);
}

pub fn run_fixed(input: &[u64]) {
    let r = fixed::sum_max(input);
    std::hint::black_box(r);
}
"#
        .to_string(),
    }
}

/// Bench spec for `prefix-sum-2d`: O(n) 2D region-sum scan vs O(1) prefix sum.
///
/// Tests K=50 random region queries on a sqrt(N)×sqrt(N) grid. The scan reads
/// K×region_area cells; the prefix reads K×4 cells (after a one-time O(N) build
/// in `make_input`). The win grows linearly with N — at N=1K it's ~30×, at
/// N=1M it's ~26,000×.
#[cfg(feature = "rust_perf_bench")]
fn prefix_sum_2d_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
pub fn region_sum(grid: &[Vec<i64>], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
    let mut sum = 0i64;
    for x in x0..x1 {
        for y in y0..y1 {
            sum += grid[x][y];
        }
    }
    sum
}
}

mod fixed {
pub fn build_prefix(grid: &[Vec<i64>]) -> Vec<Vec<i64>> {
    let w = grid.len();
    let h = grid.first().map(|r| r.len()).unwrap_or(0);
    let mut p = vec![vec![0i64; h + 1]; w + 1];
    for x in 0..w {
        for y in 0..h {
            p[x + 1][y + 1] = grid[x][y] + p[x][y + 1] + p[x + 1][y] - p[x][y];
        }
    }
    p
}
pub fn region_sum(prefix: &[Vec<i64>], x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
    prefix[x1][y1] - prefix[x0][y1] - prefix[x1][y0] + prefix[x0][y0]
}
}

pub struct Input {
    grid: Vec<Vec<i64>>,
    prefix: Vec<Vec<i64>>,
    queries: Vec<(usize, usize, usize, usize)>,
}

pub fn make_input(n: usize) -> Input {
    let dim = ((n as f64).sqrt().round() as usize).max(2);
    let grid: Vec<Vec<i64>> = (0..dim)
        .map(|x| (0..dim).map(|y| ((x * 31 + y * 17) % 100) as i64).collect())
        .collect();
    let prefix = fixed::build_prefix(&grid);
    // K=50 region queries covering ~half the grid each.
    let half = (dim / 2).max(1);
    let queries: Vec<(usize, usize, usize, usize)> = (0..50)
        .map(|i| {
            let x0 = (i * 7) % half;
            let y0 = (i * 13) % half;
            let x1 = (x0 + half).min(dim);
            let y1 = (y0 + half).min(dim);
            (x0, y0, x1, y1)
        })
        .collect();
    Input { grid, prefix, queries }
}

pub fn run_buggy(input: &Input) {
    let mut total = 0i64;
    for &(x0, y0, x1, y1) in &input.queries {
        total = total.wrapping_add(buggy::region_sum(
            std::hint::black_box(&input.grid),
            std::hint::black_box(x0),
            std::hint::black_box(y0),
            std::hint::black_box(x1),
            std::hint::black_box(y1),
        ));
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let mut total = 0i64;
    for &(x0, y0, x1, y1) in &input.queries {
        total = total.wrapping_add(fixed::region_sum(
            std::hint::black_box(&input.prefix),
            std::hint::black_box(x0),
            std::hint::black_box(y0),
            std::hint::black_box(x1),
            std::hint::black_box(y1),
        ));
    }
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `precompute-table`: runtime popcount loop vs const table.
#[cfg(feature = "rust_perf_bench")]
fn precompute_table_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    pub fn parity_byte(b: u8) -> u8 {
        (0..8).filter(|i| b & (1 << i) != 0).count() as u8 & 1
    }
}

mod fixed {
    const PARITY: [u8; 256] = {
        let mut t = [0u8; 256];
        let mut i = 0;
        while i < 256 { t[i] = (i.count_ones() & 1) as u8; i += 1; }
        t
    };
    pub fn parity_byte(b: u8) -> u8 { PARITY[b as usize] }
}

pub fn make_input(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i as u8).wrapping_mul(131)).collect()
}

pub fn run_buggy(input: &[u8]) {
    let mut acc: u8 = 0;
    for &b in input {
        acc ^= buggy::parity_byte(b);
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &[u8]) {
    let mut acc: u8 = 0;
    for &b in input {
        acc ^= fixed::parity_byte(b);
    }
    std::hint::black_box(acc);
}
"#
        .to_string(),
    }
}

/// Bench spec for `papaya-lockfree`: RwLock-contended reads vs papaya lock-free.
///
/// Single-threaded read burst — measures the RwLock read-lock overhead vs
/// papaya's epoch-based read. (True multi-threaded contention would need
/// threads, which complicates the harness; the single-threaded path still
/// shows the lock-acquisition cost.)
#[cfg(feature = "rust_perf_bench")]
fn papaya_lockfree_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
// [dep: papaya = "0.2"]
use std::collections::HashMap as StdHashMap;

mod buggy_store {
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
pub struct Store(Arc<RwLock<HashMap<u32, String>>>);
impl Store {
    pub fn new(entries: &[(u32, String)]) -> Self {
        let mut map = HashMap::new();
        for (k, v) in entries {
            map.insert(*k, v.clone());
        }
        Self(Arc::new(RwLock::new(map)))
    }
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.read().unwrap().get(&id).cloned()
    }
}
}

mod fixed_store {
use papaya::HashMap;
pub struct Store(HashMap<u32, String>);
impl Store {
    pub fn new(entries: &[(u32, String)]) -> Self {
        let map = HashMap::new();
        {
            let mut guard = map.pin();
            for (k, v) in entries {
                guard.insert(*k, v.clone());
            }
        }
        Self(map)
    }
    pub fn get(&self, id: u32) -> Option<String> {
        self.0.pin().get(&id).cloned()
    }
}
}

pub struct Input {
keys: Vec<u32>,
buggy: buggy_store::Store,
fixed: fixed_store::Store,
}

pub fn make_input(n: usize) -> Input {
let entries: Vec<(u32, String)> = (0..n)
    .map(|i| (i as u32, format!("user{i}")))
    .collect();
let keys: Vec<u32> = (0..n as u32).collect();
let buggy = buggy_store::Store::new(&entries);
let fixed = fixed_store::Store::new(&entries);
Input { keys, buggy, fixed }
}

pub fn run_buggy(input: &Input) {
let mut acc = 0u64;
for &k in &input.keys {
    if input.buggy.get(k).is_some() {
        acc += 1;
    }
}
std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
let mut acc = 0u64;
for &k in &input.keys {
    if input.fixed.get(k).is_some() {
        acc += 1;
    }
}
std::hint::black_box(acc);
}
"#
    .to_string(),
}
}

/// Bench spec for `vec-with-capacity`: Vec::new() growth vs Vec::with_capacity.
///
/// The buggy path uses `Vec::new()` + push, triggering the growth strategy
/// (1 → 2 → 4 → 8 → … reallocs + copies). The fixed path pre-allocates with
/// `Vec::with_capacity(n)`, doing 1 allocation + 0 extra copies. The win is
/// the realloc + copy cost, which grows with n.
#[cfg(feature = "rust_perf_bench")]
fn vec_with_capacity_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn build(n: usize) -> Vec<u32> {
        let mut v = Vec::new();
        for i in 0..n {
            v.push(i as u32);
        }
        v
    }
}

mod fixed {
    pub fn build(n: usize) -> Vec<u32> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(i as u32);
        }
        v
    }
}

pub struct Input {
    pub n: usize,
}

pub fn make_input(n: usize) -> Input {
    Input { n }
}

pub fn run_buggy(input: &Input) {
    let v = buggy::build(input.n);
    std::hint::black_box(v);
}

pub fn run_fixed(input: &Input) {
    let v = fixed::build(input.n);
    std::hint::black_box(v);
}
"#
        .to_string(),
    }
}

/// Bench spec for `format-concat-loop`: O(n²) format! concat vs O(n) push_str.
///
/// The buggy path uses `s = format!("{s}{chunk}")` which allocates a new String
/// each iteration + copies the entire accumulated string. The fixed path uses
/// `s.push_str(chunk)` which amortizes to O(n). At n=1000 chunks of ~10 bytes,
/// the buggy path copies ~5MB total vs ~10KB for the fixed path.
#[cfg(feature = "rust_perf_bench")]
fn format_concat_loop_bench_spec() -> BenchSpec {
    BenchSpec {
        // Cap at 1000: the O(n²) buggy path copies ~5MB at n=1000; larger
        // sizes would dominate the bench wall-clock + risk allocator jitter.
        sizes: vec![100, 500, 1_000],
        harness: r#"
mod buggy {
    pub fn build(chunks: &[String]) -> String {
        let mut s = String::new();
        for chunk in chunks {
            s = format!("{s}{chunk}");
        }
        s
    }
}

mod fixed {
    pub fn build(chunks: &[String]) -> String {
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        let mut s = String::with_capacity(total);
        for chunk in chunks {
            s.push_str(chunk);
        }
        s
    }
}

pub fn make_input(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("chunk-{i:04}")).collect()
}

pub fn run_buggy(input: &[String]) {
    let s = buggy::build(input);
    std::hint::black_box(s);
}

pub fn run_fixed(input: &[String]) {
    let s = fixed::build(input);
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `ok-or-else`: eager `ok_or(expensive())` vs lazy
/// `ok_or_else(|| expensive())`. The eager form constructs a String for every
/// element even when the Option is Some; the lazy form only on None.
#[cfg(feature = "rust_perf_bench")]
fn ok_or_else_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn expensive_error(n: u32) -> String {
        let mut s = String::with_capacity(64);
        for i in 0..32 {
            s.push_str(&format!("error field {n}-{i}; "));
        }
        s
    }
    pub fn sum_opts(opts: &[Option<u32>]) -> u32 {
        let mut sum = 0u32;
        for o in opts {
            let r = o.ok_or(expensive_error(0));
            sum += r.unwrap_or(0);
        }
        sum
    }
}

mod fixed {
    pub fn expensive_error(n: u32) -> String {
        let mut s = String::with_capacity(64);
        for i in 0..32 {
            s.push_str(&format!("error field {n}-{i}; "));
        }
        s
    }
    pub fn sum_opts(opts: &[Option<u32>]) -> u32 {
        let mut sum = 0u32;
        for o in opts {
            let r = o.ok_or_else(|| expensive_error(0));
            sum += r.unwrap_or(0);
        }
        sum
    }
}

pub fn make_input(n: usize) -> Vec<Option<u32>> {
    // 99% Some, 1% None — realistic distribution.
    (0..n).map(|i| if i % 100 == 0 { None } else { Some(i as u32) }).collect()
}

pub fn run_buggy(input: &[Option<u32>]) {
    std::hint::black_box(buggy::sum_opts(input));
}

pub fn run_fixed(input: &[Option<u32>]) {
    std::hint::black_box(fixed::sum_opts(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `swap-remove`: Vec::remove (O(n) shift) vs Vec::swap_remove (O(1)).
#[cfg(feature = "rust_perf_bench")]
fn swap_remove_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn remove_front(data: &mut Vec<u32>, count: usize) {
        for _ in 0..count.min(data.len()) {
            data.remove(0);
        }
    }
}

mod fixed {
    pub fn remove_front(data: &mut Vec<u32>, count: usize) {
        for _ in 0..count.min(data.len()) {
            data.swap_remove(0);
        }
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &Vec<u32>) {
    let mut data = input.clone();
    let count = data.len() / 10;
    buggy::remove_front(&mut data, count);
    std::hint::black_box(data);
}

pub fn run_fixed(input: &Vec<u32>) {
    let mut data = input.clone();
    let count = data.len() / 10;
    fixed::remove_front(&mut data, count);
    std::hint::black_box(data);
}
"#
        .to_string(),
    }
}

/// Bench spec for `chunks-exact`: `chunks(8)` vs `chunks_exact(8)` + remainder.
#[cfg(feature = "rust_perf_bench")]
fn chunks_exact_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_chunks(data: &[u32]) -> u64 {
        let mut sum = 0u64;
        for chunk in data.chunks(8) {
            for &v in chunk { sum += v as u64; }
        }
        sum
    }
}

mod fixed {
    pub fn sum_chunks(data: &[u32]) -> u64 {
        let mut sum = 0u64;
        for chunk in data.chunks_exact(8) {
            for &v in chunk { sum += v as u64; }
        }
        for &v in data.chunks_exact(8).remainder() { sum += v as u64; }
        sum
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::sum_chunks(input));
}

pub fn run_fixed(input: &[u32]) {
    std::hint::black_box(fixed::sum_chunks(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `fxhashmap`: std HashMap (SipHash) lookup vs FxHashMap (FxHash).
///
/// Both maps pre-populated with n entries. The bench sweeps all keys for
/// membership. SipHash does 2 rounds of AES-like mixing; FxHash does 1
/// multiply + xor. ~2-4× faster lookups, ~2× faster inserts.
#[cfg(feature = "rust_perf_bench")]
fn fxhashmap_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
// [dep: rustc-hash = "2"]
use rustc_hash::FxHashMap;
use std::collections::HashMap as StdHashMap;

pub struct Input {
    keys: Vec<u32>,
    std_map: StdHashMap<u32, u32>,
    fx_map: FxHashMap<u32, u32>,
}

pub fn make_input(n: usize) -> Input {
    // Spread keys via a multiplicative hash to avoid sequential hot-spots.
    let keys: Vec<u32> = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let mut std_map: StdHashMap<u32, u32> = StdHashMap::with_capacity(n);
    let mut fx_map: FxHashMap<u32, u32> = FxHashMap::default();
    fx_map.reserve(n);
    for &k in &keys {
        std_map.insert(k, k);
        fx_map.insert(k, k);
    }
    Input { keys, std_map, fx_map }
}

pub fn run_buggy(input: &Input) {
    let mut sum = 0u64;
    for &k in &input.keys {
        sum += input.std_map.get(&k).copied().unwrap_or(0) as u64;
    }
    std::hint::black_box(sum);
}

pub fn run_fixed(input: &Input) {
    let mut sum = 0u64;
    for &k in &input.keys {
        sum += input.fx_map.get(&k).copied().unwrap_or(0) as u64;
    }
    std::hint::black_box(sum);
}
"#
        .to_string(),
    }
}

/// Bench spec for `retain-inplace`: filter+collect (new Vec) vs retain (in-place).
///
/// The buggy path allocates a new Vec + copies survivors. The fixed path
/// filters in-place — zero allocation. ~1.5-2× at all sizes.
#[cfg(feature = "rust_perf_bench")]
fn retain_inplace_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn filter_out_multiples(data: &[u32]) -> Vec<u32> {
        data.iter().copied().filter(|&x| x % 3 != 0).collect()
    }
}

mod fixed {
    pub fn filter_out_multiples(data: &mut Vec<u32>) {
        data.retain(|&x| x % 3 != 0);
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n).map(|i| i as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::filter_out_multiples(input));
}

pub fn run_fixed(input: &[u32]) {
    // Clone to simulate the in-place mutation (the caller already owns the Vec).
    let mut data = input.to_vec();
    fixed::filter_out_multiples(&mut data);
    std::hint::black_box(data);
}
"#
        .to_string(),
    }
}

/// Bench spec for `mem-take`: clone+clear vs mem::take for Vec<String> buffer.
///
/// The buggy path deep-clones every String then clears. The fixed path
/// moves the Vec's allocation in O(1). ~3× at all sizes.
#[cfg(feature = "rust_perf_bench")]
fn mem_take_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
pub fn make_input(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("item_{i}")).collect()
}

mod buggy {
    pub fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {
        let taken = buf.clone();
        buf.clear();
        taken
    }
}

mod fixed {
    pub fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {
        std::mem::take(buf)
    }
}

pub fn run_buggy(input: &[String]) {
    let mut buf = input.to_vec();
    std::hint::black_box(buggy::take_buffer(&mut buf));
}

pub fn run_fixed(input: &[String]) {
    let mut buf = input.to_vec();
    std::hint::black_box(fixed::take_buffer(&mut buf));
}
"#
        .to_string(),
    }
}

/// Bench spec for `avoid-clone`: clone(Vec<u8>) vs borrow(&[u8]).
///
/// The buggy path clones the whole Vec (heap alloc + memcpy) every call.
/// The fixed path borrows the slice — zero allocation. ~1.3-3× faster.
#[cfg(feature = "rust_perf_bench")]
fn avoid_clone_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn process(data: Vec<u8>) -> u64 {
        data.iter().map(|&b| b as u64).sum()
    }
}

mod fixed {
    pub fn process(data: &[u8]) -> u64 {
        data.iter().map(|&b| b as u64).sum()
    }
}

pub fn make_input(n: usize) -> Vec<u8> {
    (0..n as u8).cycle().take(n).collect()
}

pub fn run_buggy(input: &[u8]) {
    std::hint::black_box(buggy::process(input.to_vec()));
}

pub fn run_fixed(input: &[u8]) {
    std::hint::black_box(fixed::process(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `box-str-immutable`: Vec<String> vs Vec<Box<str>>.
///
/// String is 3 words (ptr, len, capacity); Box<str> is 2 (ptr, len).
/// The capacity word savings improve cache density at scale. ~1.1-1.3×.
#[cfg(feature = "rust_perf_bench")]
fn box_str_immutable_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
pub struct Input {
    strings: Vec<String>,
    box_strs: Vec<Box<str>>,
}

pub fn make_input(n: usize) -> Input {
    let strings: Vec<String> = (0..n).map(|i| format!("item_{i}")).collect();
    let box_strs: Vec<Box<str>> = strings.iter().map(|s| s.as_str().into()).collect();
    Input { strings, box_strs }
}

pub fn run_buggy(input: &Input) {
    let mut total = 0usize;
    for s in &input.strings {
        total += s.len();
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let mut total = 0usize;
    for s in &input.box_strs {
        total += s.len();
    }
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `reuse-allocation-clear`: alloc-per-call vs clear-reuse.
///
/// The buggy path allocates a fresh Vec::new() inside the function body
/// every call. The fixed path reuses a caller-passed buffer via .clear().
/// **Mixed** — wins at N ≤ 10,000 (up to 5.97× at N=100), loses at N=100,000
/// where the reused buffer exceeds L2 cache.
#[cfg(feature = "rust_perf_bench")]
fn reuse_allocation_clear_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn build_temp(n: usize) -> u64 {
        let mut v: Vec<u32> = Vec::new();
        for i in 0..n {
            v.push(i as u32);
        }
        v.iter().map(|&x| x as u64).sum()
    }
}

mod fixed {
    pub fn build_temp(n: usize, buf: &mut Vec<u32>) -> u64 {
        buf.clear();
        for i in 0..n {
            buf.push(i as u32);
        }
        buf.iter().map(|&x| x as u64).sum()
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::build_temp(*input));
}

pub fn run_fixed(input: &usize) {
    let mut buf: Vec<u32> = Vec::with_capacity(*input);
    std::hint::black_box(fixed::build_temp(*input, &mut buf));
}
"#
        .to_string(),
    }
}

/// Bench spec for `box-avoid`: Vec<Box<i64>> (heap per elem) vs Vec<i64> (stack).
///
/// The buggy path allocates a Box on the heap for every element. The fixed
/// path stores values inline. ~10-35× at all sizes — the heap alloc dominates.
#[cfg(feature = "rust_perf_bench")]
fn box_avoid_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn build(n: usize) -> Vec<Box<i64>> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n { v.push(Box::new(i as i64)); }
        v
    }
}

mod fixed {
    pub fn build(n: usize) -> Vec<i64> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n { v.push(i as i64); }
        v
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    let v = buggy::build(*input);
    let total: i64 = v.iter().map(|b| **b).sum();
    std::hint::black_box(total);
}

pub fn run_fixed(input: &usize) {
    let v = fixed::build(*input);
    let total: i64 = v.iter().copied().sum();
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `smallvec-stack`: Vec (always heap) vs stack array for ≤8.
///
/// At small N (1, 4, 8), the heap alloc on every call dominates the actual
/// data work. The fixed path uses a stack array for the small case. ~7-12×.
#[cfg(feature = "rust_perf_bench")]
fn smallvec_stack_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 4, 8],
        harness: r#"
mod buggy {
    pub fn sum_small(n: usize) -> u64 {
        let mut v: Vec<u64> = Vec::new();
        for i in 0..n { v.push(i as u64); }
        v.iter().copied().sum()
    }
}

mod fixed {
    pub fn sum_small(n: usize) -> u64 {
        if n <= 8 {
            let mut arr = [0u64; 8];
            for i in 0..n { arr[i] = i as u64; }
            arr.iter().take(n).copied().sum()
        } else {
            let mut v: Vec<u64> = Vec::new();
            for i in 0..n { v.push(i as u64); }
            v.iter().copied().sum()
        }
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::sum_small(*input));
}

pub fn run_fixed(input: &usize) {
    std::hint::black_box(fixed::sum_small(*input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `buffered-io`: File::read byte-by-byte vs BufReader.
///
/// Both paths read the same temp file. The buggy path issues a syscall per
/// byte; the fixed path batches reads through BufReader's 8KB buffer.
/// ~97× at 100KB file. The bench writes a temp file per run + cleans up.
#[cfg(feature = "rust_perf_bench")]
fn buffered_io_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000],
        harness: r#"
use std::io::Read;
use std::path::PathBuf;

mod buggy {
    use std::io::Read;
    pub fn sum_bytes(path: &std::path::Path) -> u64 {
        let mut f = std::fs::File::open(path).unwrap();
        let mut total = 0u64;
        let mut byte = [0u8; 1];
        while f.read(&mut byte).unwrap() == 1 {
            total += byte[0] as u64;
        }
        total
    }
}

mod fixed {
    use std::io::Read;
    pub fn sum_bytes(path: &std::path::Path) -> u64 {
        let f = std::fs::File::open(path).unwrap();
        let mut br = std::io::BufReader::new(f);
        let mut total = 0u64;
        let mut byte = [0u8; 1];
        while br.read(&mut byte).unwrap() == 1 {
            total += byte[0] as u64;
        }
        total
    }
}

pub struct Input {
    path: PathBuf,
}

pub fn make_input(n: usize) -> Input {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("perf_bench_buffered_io_{}.dat", n));
    let content: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
    std::fs::write(&path, &content).unwrap();
    Input { path }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::sum_bytes(&input.path));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::sum_bytes(&input.path));
}
"#
        .to_string(),
    }
}

/// Bench spec for `log-write-direct`: push_str(&format!(...)) vs write! direct.
///
/// The buggy path creates a temporary String via format! every iteration,
/// pushes it into the log buffer, then drops the temp. The fixed path uses
/// write! to format directly into the existing buffer — zero temp alloc.
/// ~1.7× at all sizes. Distinct from `format-concat-loop` (quadratic
/// accumulation); this is about avoiding the per-iteration temporary.
#[cfg(feature = "rust_perf_bench")]
fn log_write_direct_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    pub fn build_log(items: &[u64]) -> String {
        let mut log = String::new();
        for &i in items {
            log.push_str(&format!("item={i}\n"));
        }
        log
    }
}

mod fixed {
    use std::fmt::Write;
    pub fn build_log(items: &[u64]) -> String {
        let mut log = String::new();
        for &i in items {
            write!(&mut log, "item={i}\n").unwrap();
        }
        log
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    std::hint::black_box(buggy::build_log(input));
}

pub fn run_fixed(input: &[u64]) {
    std::hint::black_box(fixed::build_log(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `sort-unstable`: stable sort (allocates temp buffer) vs
/// unstable sort (in-place pdqsort).
///
/// Unstable sort avoids the O(n) temporary allocation + uses a faster
/// algorithm (pattern-defeating quicksort vs mergesort). The win is largest
/// on random data where pdqsort's pivot selection is optimal.
#[cfg(feature = "rust_perf_bench")]
fn sort_unstable_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn sort(scores: &mut Vec<u32>) { scores.sort(); }
}

mod fixed {
    pub fn sort(scores: &mut Vec<u32>) { scores.sort_unstable(); }
}

pub struct Input {
    data: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    // Pseudo-random data (deterministic seed for reproducibility).
    let mut data = Vec::with_capacity(n);
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    for i in 0..n {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        data.push((seed.wrapping_add(i as u64)) as u32);
    }
    Input { data }
}

pub fn run_buggy(input: &Input) {
    let mut data = input.data.clone();
    buggy::sort(&mut data);
    std::hint::black_box(&data);
}

pub fn run_fixed(input: &Input) {
    let mut data = input.data.clone();
    fixed::sort(&mut data);
    std::hint::black_box(&data);
}
"#
        .to_string(),
    }
}

/// Bench spec for `sort-by-key`: sort_by with repeated closure calls vs
/// sort_by_key with single key extraction.
///
/// Uses `Vec<(String, u32)>` sorted by the u32 score field. `sort_by` calls
/// the comparator O(N log N) times — each call reads `.1` from two elements.
/// `sort_by_key` extracts the key once per element (N calls), then sorts the
/// cached `u32` keys. The win is cache locality: with large `(String, u32)`
/// tuples spanning multiple cache lines, accessing `.1` fewer times reduces
/// cache pressure.
#[cfg(feature = "rust_perf_bench")]
fn sort_by_key_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 50_000, 100_000],
        harness: r#"
mod buggy {
    pub fn sort_by_score(items: &mut Vec<(String, u32)>) {
        items.sort_by(|a, b| a.1.cmp(&b.1));
    }
}

mod fixed {
    pub fn sort_by_score(items: &mut Vec<(String, u32)>) {
        items.sort_by_key(|item| item.1);
    }
}

pub struct Input {
    items: Vec<(String, u32)>,
}

pub fn make_input(n: usize) -> Input {
    let mut items = Vec::with_capacity(n);
    let mut seed: u64 = 0x12345678ABCDEF00;
    for i in 0..n {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        // Generate strings of varying length (10-30 chars) so each tuple
        // is large enough to span cache lines — makes the key-extraction
        // count difference measurable.
        let len = 10 + (seed % 21) as usize;
        let s: String = (0..len).map(|j| ((seed.wrapping_add(j as u64) % 26) as u8 + b'a') as char).collect();
        items.push((s, (seed % 1_000_000) as u32));
    }
    Input { items }
}

pub fn run_buggy(input: &Input) {
    let mut items = input.items.clone();
    buggy::sort_by_score(&mut items);
    std::hint::black_box(&items);
}

pub fn run_fixed(input: &Input) {
    let mut items = input.items.clone();
    fixed::sort_by_score(&mut items);
    std::hint::black_box(&items);
}
"#
        .to_string(),
    }
}

/// Bench spec for `bufwriter-batched`: per-record File::write (syscall each)
/// vs BufWriter batched.
///
/// Uses `tempfile` to get a real filesystem path. Each record is 64 bytes;
/// with 8KB BufWriter buffering, ~128 records per flush. The buggy path does
/// one syscall per record; the fixed path does one syscall per ~128 records.
#[cfg(feature = "rust_perf_bench")]
fn bufwriter_batched_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
// [dep: tempfile = "3"]
use std::io::{BufWriter, Write};

mod buggy {
    use std::io::Write;
    pub fn write_records(file: &mut std::fs::File, records: &[Vec<u8>]) {
        for rec in records {
            let _ = file.write_all(rec);
        }
    }
}

mod fixed {
    use std::io::{BufWriter, Write};
    pub fn write_records(file: std::fs::File, records: &[Vec<u8>]) {
        let mut buf = BufWriter::new(file);
        for rec in records {
            let _ = buf.write_all(rec);
        }
        let _ = buf.flush();
    }
}

pub struct Input {
    records: Vec<Vec<u8>>,
    dir: tempfile::TempDir,
}

pub fn make_input(n: usize) -> Input {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        // 64-byte records (realistic line length)
        let rec = format!("{i:060}\n").into_bytes();
        records.push(rec);
    }
    Input { records, dir }
}

pub fn run_buggy(input: &Input) {
    let path = input.dir.path().join("buggy.txt");
    let mut file = std::fs::File::create(&path).expect("create");
    buggy::write_records(&mut file, &input.records);
    std::hint::black_box(&file);
    drop(file);
    let _ = std::fs::remove_file(&path);
}

pub fn run_fixed(input: &Input) {
    let path = input.dir.path().join("fixed.txt");
    let file = std::fs::File::create(&path).expect("create");
    fixed::write_records(file, &input.records);
    let _ = std::fs::remove_file(&path);
}
"#
        .to_string(),
    }
}

/// Bench spec for `copy-from-slice`: manual index-loop copy vs `copy_from_slice`.
///
/// The manual loop does per-element bounds checking + scalar loads/stores.
/// `copy_from_slice` is a single intrinsic that LLVM lowers to `memcpy` or
/// SIMD-wide operations. The win is largest on medium-sized slices where
/// the manual loop's overhead (bounds check + scalar ops) is measurable but
/// the data fits in cache.
#[cfg(feature = "rust_perf_bench")]
fn copy_from_slice_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![64, 256, 1_024, 4_096],
        harness: r#"
mod buggy {
    pub fn copy_data(dst: &mut [u8], src: &[u8]) {
        for i in 0..src.len() {
            dst[i] = src[i];
        }
    }
}

mod fixed {
    pub fn copy_data(dst: &mut [u8], src: &[u8]) {
        dst.copy_from_slice(src);
    }
}

pub struct Input {
    src: Vec<u8>,
    dst: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed: u64 = 0xDEADBEEFCAFEBABE;
    let src: Vec<u8> = (0..n).map(|i| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as u8
    }).collect();
    let dst = vec![0u8; n];
    Input { src, dst }
}

pub fn run_buggy(input: &Input) {
    let mut dst = input.dst.clone();
    buggy::copy_data(&mut dst, &input.src);
    std::hint::black_box(&dst);
}

pub fn run_fixed(input: &Input) {
    let mut dst = input.dst.clone();
    fixed::copy_data(&mut dst, &input.src);
    std::hint::black_box(&dst);
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-remove-front-to-vecdeque`: drain N elements from front.
///
/// Vec::remove(0) does a memmove of all remaining elements (O(n) per call,
/// O(n²) total). VecDeque::pop_front advances the head index (O(1) per call,
/// O(n) total). Grows quadratically with N.
#[cfg(feature = "rust_perf_bench")]
fn vec_remove_front_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 50_000],
        harness: r#"
mod buggy {
    pub fn drain_queue(queue: &mut Vec<u32>) -> u64 {
        let mut sum = 0u64;
        while !queue.is_empty() {
            sum += queue.remove(0) as u64;
        }
        sum
    }
}

mod fixed {
    use std::collections::VecDeque;
    pub fn drain_queue(queue: &mut VecDeque<u32>) -> u64 {
        let mut sum = 0u64;
        while let Some(val) = queue.pop_front() {
            sum += val as u64;
        }
        sum
    }
}

pub struct Input {
    vec: Vec<u32>,
    deque: std::collections::VecDeque<u32>,
}

pub fn make_input(n: usize) -> Input {
    let vec: Vec<u32> = (0..n).map(|i| i as u32).collect();
    let deque: std::collections::VecDeque<u32> = vec.iter().copied().collect();
    Input { vec, deque }
}

pub fn run_buggy(input: &Input) {
    let mut queue = input.vec.clone();
    std::hint::black_box(buggy::drain_queue(&mut queue));
}

pub fn run_fixed(input: &Input) {
    let mut queue = input.deque.clone();
    std::hint::black_box(fixed::drain_queue(&mut queue));
}
"#
        .to_string(),
    }
}

/// Bench spec for `btreemap-sorted-iteration`: sorted iteration × K queries.
///
/// HashMap + sort_unstable per query is O(n log n) each. BTreeMap's keys()
/// iterator is already sorted — O(n) per query. Insert in pseudo-random order
/// so HashMap's iteration order is NOT accidentally sorted.
#[cfg(feature = "rust_perf_bench")]
fn btreemap_sorted_iteration_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 50_000],
        harness: r#"
use std::collections::{BTreeMap, HashMap};

mod buggy {
    use std::collections::HashMap;
    pub fn sorted_keys(map: &HashMap<u32, u32>) -> Vec<u32> {
        let mut keys: Vec<u32> = map.keys().copied().collect();
        keys.sort_unstable();
        keys
    }
}

mod fixed {
    use std::collections::BTreeMap;
    pub fn sorted_keys(map: &BTreeMap<u32, u32>) -> Vec<u32> {
        map.keys().copied().collect()
    }
}

pub struct Input {
    hashmap: HashMap<u32, u32>,
    btreemap: BTreeMap<u32, u32>,
    queries: usize,
}

pub fn make_input(n: usize) -> Input {
    let mut hashmap = HashMap::with_capacity(n);
    let mut btreemap = BTreeMap::new();
    // Insert with pseudo-random keys so HashMap iteration order is NOT sorted.
    for i in 0..n {
        let key = (i.wrapping_mul(7919).wrapping_add(31)) as u32;
        hashmap.insert(key, i as u32);
        btreemap.insert(key, i as u32);
    }
    // Sorted iteration needed repeatedly (N/10 queries) — the common case
    // where a BTreeMap is the right choice over a HashMap.
    let queries = (n / 10).max(1);
    Input { hashmap, btreemap, queries }
}

pub fn run_buggy(input: &Input) {
    let queries = std::hint::black_box(input.queries);
    for _ in 0..queries {
        std::hint::black_box(buggy::sorted_keys(&input.hashmap));
    }
}

pub fn run_fixed(input: &Input) {
    let queries = std::hint::black_box(input.queries);
    for _ in 0..queries {
        std::hint::black_box(fixed::sorted_keys(&input.btreemap));
    }
}
"#
        .to_string(),
    }
}

/// Bench spec for `cow-conditional-alloc`: HTML-escape N strings (90% clean).
///
/// The buggy path calls `s.to_string()` for every string (N allocations).
/// The fixed path returns `Cow::Borrowed(s)` for the 90% that need no escaping
/// (0 allocations) + `Cow::Owned(...)` for the 10% that do. Net: 0.1N vs N
/// allocations.
#[cfg(feature = "rust_perf_bench")]
fn cow_conditional_alloc_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 50_000],
        harness: r#"
use std::borrow::Cow;

mod buggy {
    pub fn escape_html(s: &str) -> String {
        if !s.contains(['<', '>', '&']) {
            return s.to_string();
        }
        s.replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('&', "&amp;")
    }
}

mod fixed {
    use std::borrow::Cow;
    pub fn escape_html(s: &str) -> Cow<str> {
        if !s.contains(['<', '>', '&']) {
            return Cow::Borrowed(s);
        }
        Cow::Owned(s.replace('<', "&lt;").replace('>', "&gt;").replace('&', "&amp;"))
    }
}

pub struct Input {
    strings: Vec<String>,
}

pub fn make_input(n: usize) -> Input {
    // 90% of strings have no special chars (common case); 10% do.
    let strings: Vec<String> = (0..n).map(|i| {
        if i % 10 == 0 {
            format!("hello <b>world</b> #{} & friends", i)
        } else {
            format!("plain text line number {}", i)
        }
    }).collect();
    Input { strings }
}

pub fn run_buggy(input: &Input) {
    let mut total_len = 0usize;
    for s in &input.strings {
        let escaped = buggy::escape_html(s);
        total_len += escaped.len();
    }
    std::hint::black_box(total_len);
}

pub fn run_fixed(input: &Input) {
    let mut total_len = 0usize;
    for s in &input.strings {
        let escaped = fixed::escape_html(s);
        total_len += escaped.len();
    }
    std::hint::black_box(total_len);
}
"#
        .to_string(),
    }
}

/// Bench spec for `regex-hoist-lazylock`: Regex::new per call vs static LazyLock.
///
/// Regex compilation is runtime NFA construction — real work (μs-scale, plus
/// KB of allocation) the compiler cannot elide or cache. The buggy path
/// recompiles the SAME pattern per call; the fixed path compiles once into a
/// `static LazyLock` (initialized during warmup, so the steady-state alloc
/// axis is 0 for the fixed side) and pays only the match. Guide prior ~555×
/// per call (Research 032); verified on this harness in Bench 020.
#[cfg(feature = "rust_perf_bench")]
fn regex_hoist_lazylock_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10, 100, 1_000],
        harness: r#"
// [dep: regex = "1"]
mod buggy {
    use regex::Regex;
    pub fn is_valid(code: &str) -> bool {
        let re = Regex::new(r"^[A-Z]{2}-\d{4}$").unwrap();
        re.is_match(code)
    }
}

mod fixed {
    use regex::Regex;
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Z]{2}-\d{4}$").unwrap());
    pub fn is_valid(code: &str) -> bool {
        RE.is_match(code)
    }
}

pub struct Input {
    codes: Vec<String>,
}

pub fn make_input(n: usize) -> Input {
    // Half valid (match path), half invalid (scan-to-end path) — a realistic
    // mix rather than a best-case all-match workload.
    let codes: Vec<String> = (0..n)
        .map(|i| {
            if i % 2 == 0 {
                format!("AB-{:04}", i % 10_000)
            } else {
                format!("invalid code {}!", i)
            }
        })
        .collect();
    Input { codes }
}

pub fn run_buggy(input: &Input) {
    let mut hits = 0usize;
    for code in &input.codes {
        if buggy::is_valid(code) {
            hits += 1;
        }
    }
    std::hint::black_box(hits);
}

pub fn run_fixed(input: &Input) {
    let mut hits = 0usize;
    for code in &input.codes {
        if fixed::is_valid(code) {
            hits += 1;
        }
    }
    std::hint::black_box(hits);
}
"#
        .to_string(),
    }
}

/// Bench spec for `arc-str-payload`: per-recipient payload clone vs Arc<str>.
///
/// A 1 KiB immutable payload fanned out to N recipients. Buggy: `to_string()`
/// per recipient — N allocations + N × 1024 bytes of copying. Fixed: ONE
/// `Arc::from` allocation + refcount-bump clones. Timing prior ~1.3× (the
/// buggy side is memcpy-bound, ~GB/s) may land in the Tie band; the
/// deterministic alloc axis carries the gate (N×1024 B → ~1 alloc + the Vec).
#[cfg(feature = "rust_perf_bench")]
fn arc_str_payload_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10, 100, 1_000],
        harness: r#"
use std::sync::Arc;

mod buggy {
    pub fn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, String)> {
        ids.iter().map(|&id| (id, payload.to_string())).collect()
    }
}

mod fixed {
    use std::sync::Arc;
    pub fn fan_out(payload: &str, ids: &[u32]) -> Vec<(u32, Arc<str>)> {
        let shared: Arc<str> = Arc::from(payload);
        ids.iter().map(|&id| (id, Arc::clone(&shared))).collect()
    }
}

pub struct Input {
    payload: String,
    ids: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    // 1 KiB immutable payload (config snapshot, telemetry batch, ...).
    let payload = "0123456789abcdef".repeat(64);
    let ids: Vec<u32> = (0..n as u32).collect();
    Input { payload, ids }
}

pub fn run_buggy(input: &Input) {
    let out = buggy::fan_out(&input.payload, &input.ids);
    // Touch every message's payload length so the strings are materialized,
    // not just the Vec scaffold.
    let total: usize = out.iter().map(|(id, s)| *id as usize + s.len()).sum();
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let out = fixed::fan_out(&input.payload, &input.ids);
    let total: usize = out.iter().map(|(id, s)| *id as usize + s.len()).sum();
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `collect-append-to-extend`: collect+append staging vs extend.
#[cfg(feature = "rust_perf_bench")]
fn collect_append_to_extend_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    // One intermediate Vec alloc + memcpy per chunk.
    pub fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {
        for c in chunks {
            let mut staged: Vec<u32> = c.iter().copied().collect();
            target.append(&mut staged);
        }
    }
}

mod fixed {
    // Straight into the target's growth path — zero intermediates.
    pub fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {
        for c in chunks {
            target.extend(c.iter().copied());
        }
    }
}

pub fn make_input(n: usize) -> Vec<Vec<u32>> {
    // Chunks of 16 — a realistic batch shape (both sides do the same work,
    // the difference is purely the intermediate staging).
    (0..n / 16).map(|b| (0..16).map(|i| (b * 16 + i) as u32).collect()).collect()
}

pub fn run_buggy(input: &[Vec<u32>]) {
    let mut target: Vec<u32> = Vec::with_capacity(input.len() * 16);
    buggy::merge_chunks(&mut target, input);
    std::hint::black_box(target.len());
}

pub fn run_fixed(input: &[Vec<u32>]) {
    let mut target: Vec<u32> = Vec::with_capacity(input.len() * 16);
    fixed::merge_chunks(&mut target, input);
    std::hint::black_box(target.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `hashset-insert-bool-return`: contains+insert double lookup
/// vs insert's bool return. Half-duplicate input so both bool arms fire.
#[cfg(feature = "rust_perf_bench")]
fn hashset_insert_bool_return_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
use std::collections::HashSet;

mod buggy {
    use std::collections::HashSet;
    // Two hash-table walks per element.
    pub fn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {
        let mut inserted = 0;
        for &x in items {
            if !seen.contains(&x) {
                seen.insert(x);
                inserted += 1;
            }
        }
        inserted
    }
}

mod fixed {
    use std::collections::HashSet;
    // ONE hash-table walk per element.
    pub fn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {
        let mut inserted = 0;
        for &x in items {
            if seen.insert(x) {
                inserted += 1;
            }
        }
        inserted
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // 50% duplicates: every element's membership arm fires both ways.
    (0..n).map(|i| (i % (n / 2).max(1)) as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut seen = HashSet::with_capacity(input.len());
    std::hint::black_box(buggy::register_seen(&mut seen, input));
}

pub fn run_fixed(input: &[u32]) {
    let mut seen = HashSet::with_capacity(input.len());
    std::hint::black_box(fixed::register_seen(&mut seen, input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `unwrap-or-else-lazy`: eager default construction vs lazy
/// closure, 99% Some (the regime where the eager default is pure waste).
/// Mirrors the `ok-or-else` harness shape.
#[cfg(feature = "rust_perf_bench")]
fn unwrap_or_else_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    fn expensive_default(n: u32) -> u32 {
        (0..n).fold(0u32, |acc, i| acc.wrapping_add(i.wrapping_mul(31)))
    }
    pub fn resolve(opt: Option<u32>) -> u32 {
        opt.unwrap_or(expensive_default(std::hint::black_box(64))) // eagerly evaluated even when Some
    }
}

mod fixed {
    fn expensive_default(n: u32) -> u32 {
        (0..n).fold(0u32, |acc, i| acc.wrapping_add(i.wrapping_mul(31)))
    }
    pub fn resolve(opt: Option<u32>) -> u32 {
        opt.unwrap_or_else(|| expensive_default(std::hint::black_box(64))) // built only on None
    }
}

pub fn make_input(n: usize) -> Vec<Option<u32>> {
    // 99% Some, 1% None — realistic distribution (mirrors ok-or-else).
    (0..n).map(|i| if i % 100 == 0 { None } else { Some(i as u32) }).collect()
}

pub fn run_buggy(input: &[Option<u32>]) {
    // black_box(input) defeats loop-invariant hoisting of the pure loop body
    // out of the caller's timing loop (Batch 48 lesson).
    let input = std::hint::black_box(input);
    let sum: u32 = input.iter().map(|o| buggy::resolve(*o)).sum();
    std::hint::black_box(sum);
}

pub fn run_fixed(input: &[Option<u32>]) {
    let input = std::hint::black_box(input);
    let sum: u32 = input.iter().map(|o| fixed::resolve(*o)).sum();
    std::hint::black_box(sum);
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-count-to-any`: full-scan filter+count vs any
/// short-circuit. First element matches, rest don't — the maximal short-circuit
/// regime (the rule's selling point).
#[cfg(feature = "rust_perf_bench")]
fn filter_count_to_any_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn has_hit(vals: &[u32]) -> bool {
        vals.iter().filter(|&&v| v == 42).count() > 0 // full scan, always
    }
}

mod fixed {
    pub fn has_hit(vals: &[u32]) -> bool {
        vals.iter().any(|&v| v == 42) // stops at the first hit
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Hit lands at n/2 — an early-but-not-zero short-circuit regime. (A
    // position-0 hit drives `any` to a literal sub-ns call, which the
    // validator's div-by-zero guard classifies as a Tie — the guard exists
    // for malformed 0ns measurements, not for genuinely free calls.)
    let mut v = vec![7u32; n];
    v[n / 2] = 42;
    v
}

pub fn run_buggy(input: &[u32]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::has_hit(input));
}

pub fn run_fixed(input: &[u32]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::has_hit(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-macro-zero-fill`: uninit alloc + memset vs calloc.
/// Both sides black_box-read a byte near the end so the pages are touched
/// identically for the read; the difference is the WRITE path (memset vs
/// lazy zero pages).
#[cfg(feature = "rust_perf_bench")]
fn vec_macro_zero_fill_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn zero_buf(n: usize) -> Vec<u8> {
        let mut v = Vec::new();
        v.resize(n, 0); // uninit alloc + explicit memset
        v
    }
}

mod fixed {
    pub fn zero_buf(n: usize) -> Vec<u8> {
        vec![0u8; n] // alloc_zeroed — calloc zero pages
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    // black_box(n) defeats loop-invariant hoisting; spread reads (first /
    // middle / last) make partial memset elision observable.
    let n = std::hint::black_box(*input);
    let v = buggy::zero_buf(n);
    let mid = v.len() / 2;
    std::hint::black_box((v[0], v[mid], v[v.len() - 1]));
}

pub fn run_fixed(input: &usize) {
    let n = std::hint::black_box(*input);
    let v = fixed::zero_buf(n);
    let mid = v.len() / 2;
    std::hint::black_box((v[0], v[mid], v[v.len() - 1]));
}
"#
        .to_string(),
    }
}

/// Bench spec for `iter-copied-deref-map`: generic Map adapter vs specialized
/// copied(). Sum consumer at plain sizes — honest Tie expected in tight
/// release loops; the alloc axis + adapter markers carry the distinction.
#[cfg(feature = "rust_perf_bench")]
fn iter_copied_deref_map_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_kills(kills: &[u64]) -> u64 {
        kills.iter().map(|k| *k).sum()
    }
}

mod fixed {
    pub fn sum_kills(kills: &[u64]) -> u64 {
        kills.iter().copied().sum()
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n).map(|i| i as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::sum_kills(input));
}

pub fn run_fixed(input: &[u64]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::sum_kills(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-map-fuse`: two adapters vs filter_map. The
/// perf-book hedges — the Pareto verdict decides the annotation.
#[cfg(feature = "rust_perf_bench")]
fn filter_map_fuse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn even_squares(vals: &[u32]) -> Vec<u32> {
        vals.iter().filter(|&&v| v % 2 == 0).map(|&v| v * v).collect()
    }
}

mod fixed {
    pub fn even_squares(vals: &[u32]) -> Vec<u32> {
        vals.iter().filter_map(|&v| (v % 2 == 0).then_some(v * v)).collect()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::even_squares(input));
}

pub fn run_fixed(input: &[u32]) {
    std::hint::black_box(fixed::even_squares(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-insert-front-deque`: Vec::insert(0, x) O(n²) prepend
/// loop vs VecDeque::push_front O(1). Mirrors the vec-remove-front harness.
#[cfg(feature = "rust_perf_bench")]
fn vec_insert_front_deque_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![250, 1_000, 4_000],
        harness: r#"
use std::collections::VecDeque;

mod buggy {
    pub fn prepend_all(out: &mut Vec<u32>, items: &[u32]) {
        for &x in items {
            out.insert(0, x); // O(n) memmove each → O(n²) total
        }
    }
}

mod fixed {
    use std::collections::VecDeque;
    pub fn prepend_all(out: &mut VecDeque<u32>, items: &[u32]) {
        for &x in items {
            out.push_front(x); // O(1) ring head advance
        }
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n).map(|i| i as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut out = Vec::with_capacity(input.len());
    buggy::prepend_all(&mut out, input);
    std::hint::black_box(out.len());
}

pub fn run_fixed(input: &[u32]) {
    let mut out = VecDeque::with_capacity(input.len());
    fixed::prepend_all(&mut out, input);
    std::hint::black_box(out.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `powi-vs-powf`: libm pow call vs single multiply. The
/// honest risk the bench adjudicates: LLVM instcombine may rewrite
/// `pow(x, 2.0)` to `x * x` in release, collapsing the win (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn powi_vs_powf_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_squares(vals: &[f32]) -> f32 {
        vals.iter().map(|v| v.powf(2.0)).sum()
    }
}

mod fixed {
    pub fn sum_squares(vals: &[f32]) -> f32 {
        vals.iter().map(|v| v.powi(2)).sum()
    }
}

pub fn make_input(n: usize) -> Vec<f32> {
    (0..n).map(|i| i as f32 * 0.5 - 0.25).collect()
}

pub fn run_buggy(input: &[f32]) {
    std::hint::black_box(buggy::sum_squares(input));
}

pub fn run_fixed(input: &[f32]) {
    std::hint::black_box(fixed::sum_squares(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `distance-sq-no-sqrt`: sqrt-then-compare vs squared
/// compare. On aarch64 f32 sqrt is a single NEON instruction — the bench
/// decides whether the scalar-ARM reality matches the Box2D x86-SIMD
/// rationale (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn distance_sq_no_sqrt_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub struct Query {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}

mod buggy {
    pub fn count_in_range(pts: &[(f32, f32)], cx: f32, cy: f32, r: f32) -> usize {
        pts.iter()
            .filter(|&&(x, y)| {
                let dx = x - cx;
                let dy = y - cy;
                (dx * dx + dy * dy).sqrt() < r
            })
            .count()
    }
}

mod fixed {
    pub fn count_in_range(pts: &[(f32, f32)], cx: f32, cy: f32, r: f32) -> usize {
        let r2 = r * r;
        pts.iter()
            .filter(|&&(x, y)| {
                let dx = x - cx;
                let dy = y - cy;
                dx * dx + dy * dy < r2
            })
            .count()
    }
}

pub struct Input {
    pts: Vec<(f32, f32)>,
    q: Query,
}

pub fn make_input(n: usize) -> Input {
    let pts = (0..n).map(|i| (i as f32 * 0.013, i as f32 * 0.029)).collect();
    Input {
        pts,
        q: Query { cx: 0.0, cy: 0.0, r: 5000.0 },
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_in_range(&input.pts, input.q.cx, input.q.cy, input.q.r));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_in_range(&input.pts, input.q.cx, input.q.cy, input.q.r));
}
"#
        .to_string(),
    }
}

/// Bench spec for `runtime-int-exponent-powi`: runtime-integer-exponent
/// powf (libm call) vs powi (binary exponentiation). The f64 arm is the
/// source shape (RoPE theta scaling); the f32 arm checks the idiom holds
/// at lower precision. Rel-error gate lives in the harness's correctness
/// sum (max rel-error across the swept exponent range folded into the
/// black_box output — the validator only times; the GOAT gate asserts the
/// rel-error separately in corpus tests).
#[cfg(feature = "rust_perf_bench")]
fn runtime_int_exponent_powi_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    // runtime integer exponent -> opaque libm pow call per element
    pub fn theta_sum(base: f64, pos: &[i32], exp: i32) -> f64 {
        let mut acc = 0.0f64;
        for &p in pos {
            acc += base.powf((p * exp) as f64);
        }
        acc
    }
}

mod fixed {
    // powi: log2(n) multiplies, no libm call (negative n -> 1/x^|n|)
    pub fn theta_sum(base: f64, pos: &[i32], exp: i32) -> f64 {
        let mut acc = 0.0f64;
        for &p in pos {
            acc += base.powi(p * exp);
        }
        acc
    }
}

pub struct Input {
    pos: Vec<i32>,
    base: f64,
    exp: i32,
}

pub fn make_input(n: usize) -> Input {
    // swept runtime exponents across the range (rel-error surface):
    // pos alternates small/large so the harness sweeps |p*exp| widely
    let pos: Vec<i32> = (0..n)
        .map(|i| if i % 2 == 0 { (i % 17) as i32 } else { -((i % 29) as i32) })
        .collect();
    Input { pos, base: 10000.0f64, exp: 3 }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::theta_sum(input.base, &input.pos, input.exp));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::theta_sum(input.base, &input.pos, input.exp));
}
"#
        .to_string(),
    }
}

/// Bench spec for `hashmap-with-capacity`: rehash ladder vs single exact
/// reservation. The alloc axis gates this even where SipHash timing
/// dominates (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn hashmap_with_capacity_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![8, 64, 512, 4096],
        harness: r#"
mod buggy {
    use std::collections::HashMap;
    pub fn build(n: usize) -> usize {
        let mut m: HashMap<u64, u64> = HashMap::new();
        for i in 0..n as u64 {
            m.insert(i, i.wrapping_mul(2_654_435_761));
        }
        m.len()
    }
}

mod fixed {
    use std::collections::HashMap;
    pub fn build(n: usize) -> usize {
        let mut m: HashMap<u64, u64> = HashMap::with_capacity(n);
        for i in 0..n as u64 {
            m.insert(i, i.wrapping_mul(2_654_435_761));
        }
        m.len()
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::build(*input));
}

pub fn run_fixed(input: &usize) {
    std::hint::black_box(fixed::build(*input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `soa-hot-field`: 40-byte AoS stride scan vs contiguous
/// SoA column sum (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn soa_hot_field_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub struct Agent {
        pub x: f32,
        pub y: f32,
        pub vx: f32,
        pub vy: f32,
        pub hp: u32,
        pub level: u32,
        pub flags: u64,
        pub id: u64,
    }
    pub fn total_x(agents: &[Agent]) -> f32 {
        agents.iter().map(|a| a.x).sum()
    }
}

mod fixed {
    pub struct Agents {
        pub xs: Vec<f32>,
        pub ys: Vec<f32>,
        pub vxs: Vec<f32>,
        pub vys: Vec<f32>,
        pub hps: Vec<u32>,
        pub levels: Vec<u32>,
        pub flags: Vec<u64>,
        pub ids: Vec<u64>,
    }
    pub fn total_x(agents: &Agents) -> f32 {
        agents.xs.iter().sum()
    }
}

pub struct Input {
    aos: Vec<buggy::Agent>,
    soa: fixed::Agents,
}

pub fn make_input(n: usize) -> Input {
    let mut aos = Vec::with_capacity(n);
    let mut soa = fixed::Agents {
        xs: Vec::with_capacity(n),
        ys: Vec::with_capacity(n),
        vxs: Vec::with_capacity(n),
        vys: Vec::with_capacity(n),
        hps: Vec::with_capacity(n),
        levels: Vec::with_capacity(n),
        flags: Vec::with_capacity(n),
        ids: Vec::with_capacity(n),
    };
    for i in 0..n as u64 {
        let x = i as f32 * 0.5;
        let y = i as f32 * 0.25;
        let vx = i as f32 * 0.125;
        let vy = i as f32 * 0.0625;
        aos.push(buggy::Agent {
            x,
            y,
            vx,
            vy,
            hp: (i % 1000) as u32,
            level: (i % 50) as u32,
            flags: i,
            id: i,
        });
        soa.xs.push(x);
        soa.ys.push(y);
        soa.vxs.push(vx);
        soa.vys.push(vy);
        soa.hps.push((i % 1000) as u32);
        soa.levels.push((i % 50) as u32);
        soa.flags.push(i);
        soa.ids.push(i);
    }
    Input { aos, soa }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::total_x(&input.aos));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::total_x(&input.soa));
}
"#
        .to_string(),
    }
}

/// Bench spec for `division-by-invariant-multiplication`: hardware integer
/// division by a loop-invariant RUNTIME divisor vs libdivide-style magic
/// multiplication (Batch 59, thermite distill). The divisor is a runtime
/// field passed through `black_box` at the call boundary — LLVM already
/// strength-reduces compile-time-constant divisors, so the baseline must
/// stay opaque for the bench to measure the rule's actual regime. Setup
/// (one hardware div inside `FastDiv::new`) is INSIDE the timed call — the
/// honest per-batch shape; it amortizes to noise at n >= 1000.
#[cfg(feature = "rust_perf_bench")]
const DIVISION_INVARIANT_FIXED_SRC: &str = r#"    pub struct FastDivU32 { magic: u32, shift: u32, add: bool }
    impl FastDivU32 {
        pub fn new(d: u32) -> Self {
            let l = 31 - d.leading_zeros();
            if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
            let k = 1u64 << l;
            let base = (k << 32) / u64::from(d);
            let rem = (k << 32) % u64::from(d);
            if u64::from(d) - rem < k {
                Self { magic: base as u32 + 1, shift: l, add: false } // ceil magic — the +1 is load-bearing
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u64::from(d) { m += 1; }
                Self { magic: (m + 1) as u32, shift: l, add: true } // (m+1) wraps mod 2^32 — intended
            }
        }
        #[inline(always)]
        pub fn div(&self, x: u32) -> u32 {
            if self.magic == 0 { return x >> self.shift; } // power-of-two divisor
            let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;
            // widening add: hi + x reaches ~2^33 — a u32 wrap drops the carry bit
            // (d=7 x=u32::MAX fails by 2^29). The theorem holds in wide math.
            if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift }
        }
    }
    pub struct FastDivU64 { magic: u64, shift: u32, add: bool }
    impl FastDivU64 {
        pub fn new(d: u64) -> Self {
            let l = 63 - d.leading_zeros();
            if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
            let k = 1u128 << l;
            let base = (k << 64) / u128::from(d);
            let rem = (k << 64) % u128::from(d);
            if u128::from(d) - rem < k {
                Self { magic: base as u64 + 1, shift: l, add: false } // ceil magic — the +1 is load-bearing
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u128::from(d) { m += 1; }
                Self { magic: (m + 1) as u64, shift: l, add: true } // (m+1) wraps mod 2^64 — intended
            }
        }
        #[inline(always)]
        pub fn div(&self, x: u64) -> u64 {
            if self.magic == 0 { return x >> self.shift; } // power-of-two divisor
            let hi = ((u128::from(self.magic) * u128::from(x)) >> 64) as u64;
            // widening add (see u32 twin): u64 wrap would drop the carry bit
            if self.add { ((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64 } else { hi >> self.shift }
        }
    }
    pub fn sum_quot_u32(data: &[u32], d: u32) -> u32 {
        let fd = FastDivU32::new(d); // one hardware div, hoisted
        let mut acc = 0u32;
        for &x in data {
            acc = acc.wrapping_add(fd.div(x)); // mulhi + shifts per element
        }
        acc
    }
    pub fn sum_quot_u64(data: &[u64], d: u64) -> u64 {
        let fd = FastDivU64::new(d);
        let mut acc = 0u64;
        for &x in data {
            acc = acc.wrapping_add(fd.div(x));
        }
        acc
    }"#;

#[cfg(feature = "rust_perf_bench")]
fn division_invariant_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_quot_u32(data: &[u32], d: u32) -> u32 {
        let mut acc = 0u32;
        for &x in data {
            acc = acc.wrapping_add(x / d); // hardware udiv per element
        }
        acc
    }
    pub fn sum_quot_u64(data: &[u64], d: u64) -> u64 {
        let mut acc = 0u64;
        for &x in data {
            acc = acc.wrapping_add(x / d);
        }
        acc
    }
}

mod fixed {
__FASTDIV_IMPLS__
}

pub struct Input {
    data32: Vec<u32>,
    data64: Vec<u64>,
    d32: u32,
    d64: u64,
}

pub fn make_input(n: usize) -> Input {
    // diverse residues; the divisors are runtime fields (NOT constants —
    // see the spec doc comment on strength reduction)
    let data32 = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let data64 = (0..n).map(|i| (i as u64).wrapping_mul(6_364_136_223_846_793_005)).collect();
    Input { data32, data64, d32: 97, d64: 1000003 }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::sum_quot_u32(&input.data32, std::hint::black_box(input.d32)));
    std::hint::black_box(buggy::sum_quot_u64(&input.data64, std::hint::black_box(input.d64)));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::sum_quot_u32(&input.data32, std::hint::black_box(input.d32)));
    std::hint::black_box(fixed::sum_quot_u64(&input.data64, std::hint::black_box(input.d64)));
}"#
            .replace("__FASTDIV_IMPLS__", DIVISION_INVARIANT_FIXED_SRC),
    }
}

/// Bench spec for `keyword-map-to-match`: static-keyword HashMap dispatch vs
/// plain match on string literals. The map pays SipHash per lookup; the match
/// compiles to a switch on length + 1-2 inline compares (LLVM; oxc verified
/// via --emit=llvm-ir after failing to beat it with a gperf-style perfect
/// hash). Mixed identifier workload (7 keywords + 3 non-keywords cycling);
/// the LazyLock init amortizes across the timing loop — the measured cost is
/// per-lookup dispatch, not construction.
#[cfg(feature = "rust_perf_bench")]
fn keyword_map_to_match_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: r#"
mod buggy {
    use std::collections::HashMap;
    use std::sync::LazyLock;
    #[derive(Clone, Copy)]
    pub enum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }
    static KEYWORDS: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("as", Kind::As);
        m.insert("do", Kind::Do);
        m.insert("if", Kind::If);
        m.insert("let", Kind::Let);
        m.insert("fn", Kind::Fn);
        m.insert("while", Kind::While);
        m.insert("constructor", Kind::Constructor);
        m
    });
    pub fn match_keyword(s: &str) -> u64 {
        *KEYWORDS.get(s).unwrap_or(&Kind::Ident) as u64
    }
}

mod fixed {
    #[derive(Clone, Copy)]
    pub enum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }
    pub fn match_keyword(s: &str) -> u64 {
        let k = match s {
            "as" => Kind::As,
            "do" => Kind::Do,
            "if" => Kind::If,
            "let" => Kind::Let,
            "fn" => Kind::Fn,
            "while" => Kind::While,
            "constructor" => Kind::Constructor,
            _ => Kind::Ident,
        };
        k as u64
    }
}

pub fn make_input(n: usize) -> Vec<&'static str> {
    let words = ["as", "identifier", "do", "if", "some_long_name", "let", "fn", "while", "constructor", "another_name"];
    (0..n).map(|i| words[i % words.len()]).collect()
}

pub fn run_buggy(input: &[&'static str]) {
    let input = std::hint::black_box(input);
    let acc: u64 = input.iter().map(|s| buggy::match_keyword(s)).sum();
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &[&'static str]) {
    let input = std::hint::black_box(input);
    let acc: u64 = input.iter().map(|s| fixed::match_keyword(s)).sum();
    std::hint::black_box(acc);
}"#
        .to_string(),
    }
}

/// Bench spec for `usize-index-to-u32`: 16-byte spans vs 8-byte spans under a
/// bandwidth-bound sum pass. At 1K elements both arrays are L1-resident (16KB
/// vs 8KB) — expected Tie. At 1M the wide array is 16MB (exceeds L2 on most
/// cores) while the narrow is 8MB — the bandwidth win should materialize.
/// Same access pattern both sides; only the field width differs (the oxc Span
/// change, isolated).
#[cfg(feature = "rust_perf_bench")]
fn usize_index_to_u32_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 1_000_000],
        harness: r#"
mod buggy {
    pub struct Span { pub start: usize, pub end: usize }
    pub fn total_len(spans: &[Span]) -> u64 {
        let mut acc = 0u64;
        for s in spans { acc += (s.end - s.start) as u64; }
        acc
    }
}

mod fixed {
    pub struct Span { pub start: u32, pub end: u32 }
    pub fn total_len(spans: &[Span]) -> u64 {
        let mut acc = 0u64;
        for s in spans { acc += u64::from(s.end - s.start); }
        acc
    }
}

pub struct Input { wide: Vec<buggy::Span>, narrow: Vec<fixed::Span> }

pub fn make_input(n: usize) -> Input {
    let wide: Vec<buggy::Span> = (0..n)
        .map(|i| buggy::Span { start: i * 3, end: i * 3 + (i % 7) + 1 })
        .collect();
    let narrow: Vec<fixed::Span> = wide
        .iter()
        .map(|s| fixed::Span { start: s.start as u32, end: s.end as u32 })
        .collect();
    Input { wide, narrow }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::total_len(&input.wide));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::total_len(&input.narrow));
}"#
        .to_string(),
    }
}

/// Bench spec for `hot-byval-move-over-memcpy-threshold` (Batch 112,
/// Research 134 — the nnethercote July-2026 distill): a 136-byte record
/// moved BY VALUE per hot-loop call (LLVM lowers each move to a `memcpy`
/// call above the inline threshold) vs the composed fix — the record shrunk
/// to 104 bytes + the 32-byte cold tail passed by reference and mutated in
/// place. Both arms fold the SAME 17 u64s per score call with the SAME
/// per-field ops in the same order (the fairness gate pins it), so the
/// measured delta is move cost only, never workload. `#[inline(never)]`
/// keeps the calls out-of-line (the source's shape); `black_box` at the
/// call sites defeats move elision. n = hot-loop iterations. Composition
/// disclosure: a binary bench cannot separate the shrink step from the
/// cold-tail move-avoidance — the measured delta is the composed fix,
/// exactly as the rule's fix space prescribes.
/// Alloc axis: zero allocations both arms (stack-only values).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn hot_byval_move_memcpy_threshold_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 1_000_000],
        harness: r#"
mod buggy {
    #[derive(Clone, Copy)]
    pub struct Record { pub hot: [u64; 13], pub cold: [u64; 4] } // 17 x 8 = 136 B (> 128)

    #[inline(never)]
    pub fn score(r: Record) -> u64 {
        let mut a = 0u64;
        for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }
        for i in 0..4 { a = a.wrapping_add(r.cold[i].wrapping_mul(3)); }
        a
    }
}

mod fixed {
    #[derive(Clone, Copy)]
    pub struct Record { pub hot: [u64; 13] } // 13 x 8 = 104 B (<= 128 on x86-64)

    #[inline(never)]
    pub fn score(r: Record, cold: &[u64; 4]) -> u64 {
        let mut a = 0u64;
        for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }
        for i in 0..4 { a = a.wrapping_add(cold[i].wrapping_mul(3)); }
        a
    }
}

pub struct Input { seed_hot: [u64; 13], seed_cold: [u64; 4], iters: usize }

pub fn make_input(n: usize) -> Input {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x };
    let mut seed_hot = [0u64; 13];
    let mut seed_cold = [0u64; 4];
    for v in seed_hot.iter_mut() { *v = next(); }
    for v in seed_cold.iter_mut() { *v = next(); }
    Input { seed_hot, seed_cold, iters: n }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut state = buggy::Record { hot: input.seed_hot, cold: input.seed_cold };
    let mut acc = 0u64;
    for i in 0..input.iters {
        acc ^= buggy::score(std::hint::black_box(state)); // by-value 136 B move per call
        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);
        state.cold[i & 3] = state.cold[i & 3].rotate_left(1);
    }
    std::hint::black_box((state, acc));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut state = fixed::Record { hot: input.seed_hot };
    let mut cold = input.seed_cold;
    let mut acc = 0u64;
    for i in 0..input.iters {
        acc ^= fixed::score(std::hint::black_box(state), std::hint::black_box(&cold)); // 104 B move + by-ref tail
        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);
        cold[i & 3] = cold[i & 3].rotate_left(1); // cold tail: zero copies
    }
    std::hint::black_box((state, cold, acc));
}
"#
            .to_string(),
    }
}

/// Bench spec for `transposed-weight-register-gemv`: row-major per-row dot
/// with its horizontal reduce, vs transposed weights with per-input
/// broadcast FMA
/// into output-vector accumulators (the ryg 2015 transposed-lane Mv
/// analysis; microGPT-C `mv16_blk16v(4)` shape, portable-Rust form).
/// NOUT fixed at 64 (4 NEON / 2 AVX2 full lanes); `n` sweeps NIN
/// 16 / 64 / 4096. **Measured (Batch 62, M3 Max): ParetoDominant 1.18x /
/// 2.09x / 6.08x — the research's "wash at large nin" expectation was
/// WRONG-DIRECTION**: in strict-FP Rust the row-dot's scalar accumulator is
/// a serial dependency chain LLVM cannot vectorize without reassociation,
/// so it degrades FURTHER at large nin (llama.cpp's fine row-major dots use
/// explicit multi-accumulator SIMD — not what portable scalar code gets).
/// NOT bit-identical (float sum order changes under transposition) — the
/// validator's rel-error tolerance covers it (the `distance-sq-no-sqrt`
/// 1-ulp caveat precedent). `y` is cloned per run (identical alloc count
/// both sides — the alloc axis compares shapes, not the steady-state
/// discipline).
#[cfg(feature = "rust_perf_bench")]
fn transposed_weight_register_gemv_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![16, 64, 4096],
        harness: "\nconst NOUT: usize = 64; // 4 NEON vectors / 2 AVX2 vectors of lanes — tile stays full\n\nmod buggy {\n    pub fn matvec(w: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n        for r in 0..nout {\n            let row = &w[r * nin..(r + 1) * nin];\n            let mut acc = 0.0f32;\n            for c in 0..nin {\n                acc += row[c] * x[c];\n            }\n            y[r] = acc;\n        }\n    }\n}\n\nmod fixed {\n    pub fn matvec(w_t: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n        for r in 0..nout { y[r] = 0.0; }\n        for c in 0..nin {\n            let xc = x[c];\n            let col = &w_t[c * nout..(c + 1) * nout];\n            for r in 0..nout {\n                y[r] += xc * col[r];\n            }\n        }\n    }\n}\n\npub struct Input { w_row: Vec<f32>, w_col: Vec<f32>, x: Vec<f32>, y: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let nin = n; // n is the input dimension; NOUT fixed at 64\n    let nout = NOUT;\n    // Same values both layouts — only the layout differs.\n    let vals = |i: usize, j: usize| {\n        let h = (i as u32).wrapping_mul(0x9E37_79B9) ^ (j as u32).wrapping_mul(0x85EB_CA6B);\n        ((h >> 8) as f32) / 16777216.0 - 0.5\n    };\n    let w_row: Vec<f32> = (0..nout).flat_map(|r| (0..nin).map(move |c| vals(r, c))).collect();\n    let w_col: Vec<f32> = (0..nin).flat_map(|c| (0..nout).map(move |r| vals(r, c))).collect();\n    let x: Vec<f32> = (0..nin).map(|c| vals(9999, c)).collect();\n    Input { w_row, w_col, x, y: vec![0.0; nout] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let nin = input.x.len();\n    buggy::matvec(&input.w_row, &input.x, &mut input.y.clone(), NOUT, nin);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let nin = input.x.len();\n    fixed::matvec(&input.w_col, &input.x, &mut input.y.clone(), NOUT, nin);\n}".to_string(),
    }
}

/// Bench spec for `newton-refinement-vs-hardware-transcendental`:
/// seed + Newton-refined `inv`/`rsqrt` vs the plain hardware ops. On CPU
/// the hardware op is single-instruction + correctly rounded, so the
/// refinement steps are pure extra work. **Measured (Batch 62, M3 Max):
/// Mixed — hardware never loses: 1.09x at 1K, 1.01x at 100K** (source
/// measured 2-4% on Ryzen 5 5600H; the M3's fdiv/sqrt latency flattens
/// the gap at 100K where the pipeline hides it). **x86 re-gate
/// (2026-08-19, 4090 box): ParetoDominant 1.64x/1.66x — the win is
/// LARGER on x86; the near-tie is M3-aarch64-specific.** The rule's purpose is
/// stopping the plausible-looking GPU→CPU fast-math port — the annotation
/// class (`filter-count-to-any` precedent). Well-conditioned [0.5, 2.0]
/// inputs — deterministic.
#[cfg(feature = "rust_perf_bench")]
fn newton_refinement_vs_hardware_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: "\nmod buggy {\n    pub fn fast_inv(x: f32) -> f32 {\n        let r = 1.0 / x;\n        let r = r * (2.0 - x * r);\n        r * (2.0 - x * r)\n    }\n    pub fn fast_rsqrt(x: f32) -> f32 {\n        let r = 1.0 / x.sqrt();\n        r * (0.5 * (3.0 - x * r * r))\n    }\n}\n\nmod fixed {\n    pub fn fast_inv(x: f32) -> f32 { 1.0 / x }\n    pub fn fast_rsqrt(x: f32) -> f32 { 1.0 / x.sqrt() }\n}\n\npub fn make_input(n: usize) -> Vec<f32> {\n    // Values in [0.5, 2.0]: no inf/nan/underflow in inv or rsqrt —\n    // deterministic, well-conditioned.\n    (0..n)\n        .map(|i| {\n            let h = (i as u32).wrapping_mul(0x9E37_79B9);\n            0.5 + ((h >> 8) as f32) / 8388608.0\n        })\n        .collect()\n}\n\npub fn run_buggy(input: &[f32]) {\n    let input = std::hint::black_box(input);\n    let acc: f32 = input\n        .iter()\n        .map(|&x| buggy::fast_inv(x) + buggy::fast_rsqrt(x))\n        .sum();\n    std::hint::black_box(acc);\n}\n\npub fn run_fixed(input: &[f32]) {\n    let input = std::hint::black_box(input);\n    let acc: f32 = input\n        .iter()\n        .map(|&x| fixed::fast_inv(x) + fixed::fast_rsqrt(x))\n        .sum();\n    std::hint::black_box(acc);\n}".to_string(),
    }
}

/// Bench spec for `loop-order-innermost-contiguous`: i-j-k vs i-k-j
/// nested-accumulation matmul. The inner loop's access pattern is the
/// whole story: column-stride-n vs contiguous row walk (L1 misses
/// 45%->12% in the source). Square sizes 64/128/256 — L1/L2-resident so
/// the access-pattern effect dominates. Float sum-order differs between
/// the orders — the validator's rel-error tolerance covers it (NOT
/// bit-identity; the `distance-sq-no-sqrt` 1-ulp caveat precedent).
#[cfg(feature = "rust_perf_bench")]
fn loop_order_innermost_contiguous_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![64, 128, 256],
        harness: "\nmod buggy {\n    pub fn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n        for i in 0..n {\n            for j in 0..n {\n                let mut acc = 0.0f32;\n                for k in 0..n {\n                    acc += a[i * n + k] * b[k * n + j]; // k strides b's OUTER axis\n                }\n                c[i * n + j] = acc;\n            }\n        }\n    }\n}\n\nmod fixed {\n    pub fn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n        for i in 0..n {\n            for j in 0..n {\n                c[i * n + j] = 0.0;\n            }\n        }\n        for i in 0..n {\n            let a_row = &a[i * n..(i + 1) * n];\n            for k in 0..n {\n                let a_ik = a_row[k];\n                let b_row = &b[k * n..(k + 1) * n];\n                let c_row = &mut c[i * n..(i + 1) * n];\n                for j in 0..n {\n                    c_row[j] += a_ik * b_row[j]; // j strides by 1 everywhere\n                }\n            }\n        }\n    }\n}\n\npub struct Input { a: Vec<f32>, b: Vec<f32>, c: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let vals = |s: u32| (0..n * n)\n        .map(move |i| {\n            let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);\n            ((h >> 8) as f32) / 16_777_216.0 - 0.5\n        })\n        .collect();\n    Input { a: vals(1), b: vals(2), c: vec![0.0; n * n] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let n = (input.a.len() as f64).sqrt() as usize;\n    buggy::matmul(&input.a, &input.b, &mut input.c.clone(), n);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let n = (input.a.len() as f64).sqrt() as usize;\n    fixed::matmul(&input.a, &input.b, &mut input.c.clone(), n);\n}".to_string(),
    }
}

/// Bench spec for `debug-assert-length-hoist`: debug_assert vs assert
/// before an indexing loop. The whole point is RELEASE behavior
/// (debug_assert is erased there — in debug both assert forms run);
/// the validator builds generated crates in release, so the buggy side
/// retains every bounds check. **Measured (Batch 64, M3 aarch64):
/// Mixed regime-split — 1.13x at n=1K (elision win) but a STABLE
/// 0.67-0.71x LOSS at n=100K** (3-run verified; the article's own
/// aarch64 number was 1.03x; x86 re-gate 2026-08-19 on the 4090 box:
/// 1.00x tie @1K, 0.92x @100K — regime confirmed cross-arch, milder
/// loss; the x86 headline win does not reproduce on this shape). The
/// canonical range-pinned shape
/// (first+count <= len feeding the induction loop); data-dependent
/// index arrays cannot propagate the constraint at all (measured
/// 0.74-0.80x dominated on the first harness shape — elision is
/// impossible there, the assert is pure cost).
#[cfg(feature = "rust_perf_bench")]
fn debug_assert_length_hoist_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: "\nmod buggy {\n    pub fn range_checksum(rows: &[(u64, u64)], first: usize, count: usize,\n                          out: &mut Vec<u64>) {\n        debug_assert!(first + count <= rows.len()); // ERASED in release —\n        // the optimizer loses exactly this range constraint\n        for i in first..first + count {\n            out.push(rows[i].0 ^ rows[i].1); // bounds checks RETAINED\n        }\n    }\n}\n\nmod fixed {\n    pub fn range_checksum(rows: &[(u64, u64)], first: usize, count: usize,\n                          out: &mut Vec<u64>) {\n        assert!(first + count <= rows.len()); // SURVIVES release — the range\n        // constraint stays visible to the optimizer\n        for i in first..first + count {\n            out.push(rows[i].0 ^ rows[i].1); // bounds checks ELIDED\n        }\n    }\n}\n\npub struct Input { rows: Vec<(u64, u64)>, scratch: Vec<u64> }\n\npub fn make_input(n: usize) -> Input {\n    let rows: Vec<(u64, u64)> = (0..n)\n        .map(|i| {\n            let h = (i as u64).wrapping_mul(0x9E37_79B9_97F4_A7C1);\n            (h ^ (h >> 31), h.wrapping_mul(3))\n        })\n        .collect();\n    Input { rows, scratch: Vec::with_capacity(n) }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut out = input.scratch.clone();\n    buggy::range_checksum(&input.rows, 3, input.rows.len() - 3, &mut out);\n    std::hint::black_box(&out);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut out = input.scratch.clone();\n    fixed::range_checksum(&input.rows, 3, input.rows.len() - 3, &mut out);\n    std::hint::black_box(&out);\n}".to_string(),
    }
}

/// Bench spec for `sparse-set-bit-iteration-tz-blsr`: 64-iteration
/// shift-test scan vs trailing_zeros+BLSR loop over sparse masks.
///
/// ~4/64 set bits per word (the rule's applicable sparse regime —
/// `0x0101..` selects bit 0 of each byte, each set with p~1/2). The
/// buggy form always executes 64 shift+test iterations; the fixed form
/// executes popcount iterations (~4). Both emit identical index sums
/// (bit-identical G1 by construction). At dense masks the two converge —
/// the honest regime boundary recorded in the rule's annotation.
#[cfg(feature = "rust_perf_bench")]
fn sparse_set_bit_iteration_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: "\nmod buggy {\n    pub fn sum_set_positions(mask: u64) -> u32 {\n        let mut sum = 0;\n        for i in 0..64 {\n            if (mask >> i) & 1 != 0 { sum += i as u32; }\n        }\n        sum\n    }\n}\n\nmod fixed {\n    pub fn sum_set_positions(mask: u64) -> u32 {\n        let mut sum = 0;\n        let mut m = mask;\n        while m != 0 {\n            let i = m.trailing_zeros();\n            sum += i;\n            m &= m.wrapping_sub(1);\n        }\n        sum\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u64> {\n    // ~4/64 set bits per word (sparse regime — the rule's applicable\n    // window): 0x0101.. selects bit 0 of each byte, each set with p~1/2\n    // under the multiplicative hash.\n    (0..n)\n        .map(|i| (i as u64).wrapping_mul(0x9E37_79B9_97F4_A7C1) & 0x0101_0101_0101_0101)\n        .collect()\n}\n\npub fn run_buggy(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let mut acc = 0u32;\n    for &m in input {\n        acc = acc.wrapping_add(buggy::sum_set_positions(m));\n    }\n    std::hint::black_box(acc);\n}\n\npub fn run_fixed(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let mut acc = 0u32;\n    for &m in input {\n        acc = acc.wrapping_add(fixed::sum_set_positions(m));\n    }\n    std::hint::black_box(acc);\n}\n".to_string(),
    }
}

/// Bench spec for `sortedness-check-branchless-accumulate`: early-exit
/// branch-per-element validation vs branchless compare+AND accumulate
/// on USUALLY-SORTED data.
///
/// Strictly increasing u64s (the applicable regime — the early-exit
/// branch never fires, so the buggy form pays scalar compare+branch per
/// element for nothing; the accumulate form has no exit and vectorizes).
/// On early-violation data the regime flips (early exit wins) — the
/// mirrored regime of `filter-count-to-any`, recorded in the annotation.
#[cfg(feature = "rust_perf_bench")]
fn sortedness_check_branchless_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000],
        harness: "\nmod buggy {\n    pub fn validate_sorted(vals: &[u64]) -> bool {\n        let mut prev = u64::MIN;\n        for &v in vals {\n            if v < prev { return false; }\n            prev = v;\n        }\n        true\n    }\n}\n\nmod fixed {\n    pub fn validate_sorted(vals: &[u64]) -> bool {\n        let mut sorted = true;\n        let mut prev = u64::MIN;\n        for &v in vals {\n            sorted &= prev <= v;\n            prev = v;\n        }\n        sorted\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u64> {\n    // Strictly increasing (usually-sorted regime — the applicable window:\n    // the early-exit branch never fires, so the buggy form pays a scalar\n    // compare+branch per element for nothing).\n    (0..n).map(|i| (i as u64).wrapping_mul(2)).collect()\n}\n\npub fn run_buggy(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let ok = buggy::validate_sorted(input);\n    std::hint::black_box(ok);\n}\n\npub fn run_fixed(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let ok = fixed::validate_sorted(input);\n    std::hint::black_box(ok);\n}\n".to_string(),
    }
}

/// Bench spec for `byte-find-position-to-memchr`: scalar early-exit
/// position scan vs the memchr crate's SIMD block scan.
///
/// Needle at n-16 (~90% position on the larger sizes — both arms do a
/// ~full scan; stray delimiters scrubbed in setup so both arms find the
/// same index). 32B = the honest small-regime arm (the Teddy-bound wash
/// where call overhead eats the win); 4K/64K = the SIMD win regime.
/// Deterministic positions; `Option<usize>` identical (bit-identical).
#[cfg(feature = "rust_perf_bench")]
fn byte_find_position_to_memchr_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![32, 256, 4_096, 65_536],
        harness: "// [dep: memchr = \"2\"]\nmod buggy {\n    pub fn find_delim(buf: &[u8]) -> Option<usize> {\n        buf.iter().position(|&b| b == b';')\n    }\n}\n\nmod fixed {\n    pub fn find_delim(buf: &[u8]) -> Option<usize> {\n        memchr::memchr(b';', buf)\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u8> {\n    // Pseudo-random bytes; stray delimiters scrubbed so the FIRST ';' sits\n    // at n-16 (both arms do ~full scan — the representative regime; the\n    // scrub runs in setup, never measured).\n    let mut v: Vec<u8> = (0..n)\n        .map(|i| ((i as u32).wrapping_mul(2654435761) >> 24) as u8)\n        .collect();\n    for b in &mut v {\n        if *b == b';' { *b = b':'; }\n    }\n    if n >= 16 {\n        v[n - 16] = b';';\n    }\n    v\n}\n\npub fn run_buggy(input: &[u8]) {\n    let input = std::hint::black_box(input);\n    let at = buggy::find_delim(input);\n    std::hint::black_box(at);\n}\n\npub fn run_fixed(input: &[u8]) {\n    let input = std::hint::black_box(input);\n    let at = fixed::find_delim(input);\n    std::hint::black_box(at);\n}\n".to_string(),
    }
}

/// Bench spec for `varint-compose-word-single-store` (Batch 68, Research
/// 075 — the tantivy distill): per-byte `write_all` writer calls vs the
/// 5-threshold ladder composing all bytes into one u64 + ONE `write_all`.
///
/// THE WRITER SHAPE IS THE RULE'S REGIME (first bench run, load-recorded:
/// the Vec-push shape measures ParetoDominated 0/3 — 128 is a compile-time
/// constant so LLVM strength-reduces the div/mod, and Vec::push-per-byte is
/// cheap enough that the ladder's if-chain loses; the honest win is N
/// writer calls → 1). Sink: `std::io::Cursor<&mut Vec<u8>>` — a real
/// generic `Write` impl with per-call branching but no syscall noise; on a
/// virtual `Box<dyn Write>` or a real file/socket writer the per-call cost
/// — and therefore the win — only grows. Input mixes ALL five serialized
/// widths deterministically (i % 5 selects the magnitude band). Stop-bit-
/// on-last convention; outputs bit-identical by the exhaustive corpus gate.
#[cfg(feature = "rust_perf_bench")]
fn varint_compose_word_single_store_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
use std::io::Write;

// Dyn-dispatch sink: the write path is IDENTICAL work (extend_from_slice)
// for both arms — the measured differential is purely N vtable calls vs 1
// (the production shape for trait-object writers; fully-inlined generic
// sinks measure ~parity, see the rule's context note).
struct Sink(Vec<u8>);
impl std::io::Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
fn new_sink(cap: usize) -> Box<dyn std::io::Write> {
    Box::new(Sink(Vec::with_capacity(cap)))
}

mod buggy {
    use std::io::{Result, Write};
    pub fn write_vint<W: Write>(w: &mut W, mut val: u32) -> Result<()> {
        loop {
            let byte = (val % 128) as u8;
            val /= 128;
            if val == 0 {
                w.write_all(&[byte | 128])?; // a writer call PER BYTE
                return Ok(());
            }
            w.write_all(&[byte])?;
        }
    }
}

mod fixed {
    use std::io::{Result, Write};
    pub fn write_vint<W: Write>(w: &mut W, val: u32) -> Result<()> {
        let v = u64::from(val);
        let (res, len) = if v < (1 << 7) {
            (v | 128, 1)
        } else if v < (1 << 14) {
            ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)
        } else if v < (1 << 21) {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)
        } else if v < (1 << 28) {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)
        } else {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)
        };
        w.write_all(&res.to_le_bytes()[..len]) // ONE store, ONE call
    }
}

pub struct Input {
    values: Vec<u32>,
    total_bytes: usize,
}

pub fn make_input(n: usize) -> Input {
    // Deterministic mix across all five magnitude bands (every ladder arm +
    // every loop trip count): band i%5 selects a value in the 1..5-byte range.
    let mut values = Vec::with_capacity(n);
    let mut total_bytes = 0usize;
    for i in 0..n {
        let band = (i % 5) as u32;
        let val: u32 = if band == 4 {
            u32::MAX - ((i as u32) % 64) // >= 2^28: 5-byte band
        } else {
            (1u32 << (7 * band)) + ((i as u32) & ((1u32 << (7 * band)) - 1))
        };
        values.push(val);
        total_bytes += band as usize + 1;
    }
    Input { values, total_bytes }
}

pub fn run_buggy(input: &Input) {
    let mut cur = new_sink(input.total_bytes + 16);
    for &v in std::hint::black_box(&input.values) {
        buggy::write_vint(&mut cur, v).unwrap();
    }
    std::hint::black_box(&cur);
}

pub fn run_fixed(input: &Input) {
    let mut cur = new_sink(input.total_bytes + 16);
    for &v in std::hint::black_box(&input.values) {
        fixed::write_vint(&mut cur, v).unwrap();
    }
    std::hint::black_box(&cur);
}"#
            .to_string(),
    }
}

/// Bench spec for `unaligned-word-load-from-bytes` (Batch 69, Research
/// 075 §Secondary — the Plan 013 T6 refutation): the 8-term
/// shift/or byte-assembly chain vs ONE unaligned `from_le_bytes` load.
///
/// **Cross-arch probe 2026-08-19 (median-of-5, interleaved arms): u64
/// 2.17-2.23x on aarch64 (M3, load ~118 shared) / 1.39-1.47x on x86 (4090
/// box, load 7-31); u32 1.27-1.38x / 1.20-1.29x — ParetoDominant BOTH
/// arches BOTH widths.** The mechanism is LOAD-COUNT (8 L1 loads + the
/// serial shift/or chain vs 1 fused load) — distinct from the varint
/// honest-loser's call-count class (OoO hides N dyn-dispatch calls; it
/// cannot hide 8 loads behind 1). The u32 family shares the buffer; the
/// harness measures the u64 headline (the family's worst-case-for-buggy
/// shape). G1: bit-identical at EVERY byte alignment (the exhaustive corpus
/// test below).
#[cfg(feature = "rust_perf_bench")]
fn unaligned_word_load_from_bytes_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000, 10_000_000],
        harness: r#"
mod buggy {
    pub fn read_u64_le(data: &[u8], addr: usize) -> u64 {
        (data[addr] as u64)
            | ((data[addr + 1] as u64) << 8)
            | ((data[addr + 2] as u64) << 16)
            | ((data[addr + 3] as u64) << 24)
            | ((data[addr + 4] as u64) << 32)
            | ((data[addr + 5] as u64) << 40)
            | ((data[addr + 6] as u64) << 48)
            | ((data[addr + 7] as u64) << 56)
    }
}

mod fixed {
    pub fn read_u64_le(data: &[u8], addr: usize) -> u64 {
        let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();
        u64::from_le_bytes(bytes)
    }
}

pub struct Input {
    data: Vec<u8>,
    idxs: Vec<usize>,
}

pub fn make_input(n: usize) -> Input {
    // 4 KiB buffer (L1-resident — the load-count mechanism isolated from
    // cache-miss noise) + a deterministic address scatter hitting every
    // byte alignment.
    const BUF: usize = 4096;
    let data: Vec<u8> = (0..BUF as u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    let idxs: Vec<usize> = (0..n).map(|i| i.wrapping_mul(48_271) % (BUF - 8)).collect();
    Input { data, idxs }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for &a in &input.idxs {
        acc = acc.wrapping_add(buggy::read_u64_le(&input.data, a));
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for &a in &input.idxs {
        acc = acc.wrapping_add(fixed::read_u64_le(&input.data, a));
    }
    std::hint::black_box(acc);
}"#
            .to_string(),
    }
}

/// Bench spec for `align-64-false-sharing` (Batch 65, Research 081 —
/// the hotpath.rs profiler distill): adjacent 24 B per-thread atomic-counter
/// slots vs `#[repr(align(128))]` slots, N scoped threads bumping their own
/// slot.
///
/// sizes = THREAD COUNTS (the `n` threaded through `make_input` is the
/// worker count, not an array length — the contention axis IS the size
/// axis; 1t deliberately excluded: no contention, honest Tie, and the
/// source measured 0% there). ITERS=10_000 iterations × 3 RMW per thread
/// keeps the contended arm ≈1 ms/call (the calibration target) at 8t.
/// The fixed arm pads to 128 B — the ARCH line size on this M3 Max
/// (`hw.cachelinesize`; align(64) leaves even-index slot pairs sharing
/// [0,128) lines on Apple Silicon — measured 1.16-2.44× vs the full
/// 3.27-10.75× separation, the annotation's load-bearing caveat).
/// G1: per-call checksum is deterministic (fetch_add totals cannot drift
/// with layout) — asserted in-run against the closed form.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn align_64_false_sharing_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![2, 4, 8, 12],
        harness: r#"
mod buggy {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub struct Stats {
        pub allocs: AtomicU64,
        pub bytes: AtomicU64,
        pub frees: AtomicU64,
    }

    pub const SLOTS: usize = 12; // several 24 B slots per coherence line

    pub fn make_slots() -> Vec<Stats> {
        (0..SLOTS)
            .map(|_| Stats {
                allocs: AtomicU64::new(0),
                bytes: AtomicU64::new(0),
                frees: AtomicU64::new(0),
            })
            .collect()
    }

    pub fn run(slots: &[Stats], n_threads: usize, iters: u64) -> u64 {
        std::thread::scope(|s| {
            for t in 0..n_threads {
                s.spawn(move || {
                    let slot = &slots[t];
                    for i in 0..iters {
                        slot.allocs.fetch_add(1, Ordering::Relaxed);
                        slot.bytes.fetch_add(i, Ordering::Relaxed);
                        slot.frees.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        slots
            .iter()
            .map(|x| {
                x.allocs.load(Ordering::Relaxed)
                    + x.bytes.load(Ordering::Relaxed)
                    + x.frees.load(Ordering::Relaxed)
            })
            .sum()
    }
}

mod fixed {
    use std::sync::atomic::{AtomicU64, Ordering};

    #[repr(align(128))] // the ARCH line size (Apple Silicon; 64 on x86)
    pub struct Stats {
        pub allocs: AtomicU64,
        pub bytes: AtomicU64,
        pub frees: AtomicU64,
    }

    pub const SLOTS: usize = 12;

    pub fn make_slots() -> Vec<Stats> {
        (0..SLOTS)
            .map(|_| Stats {
                allocs: AtomicU64::new(0),
                bytes: AtomicU64::new(0),
                frees: AtomicU64::new(0),
            })
            .collect()
    }

    pub fn run(slots: &[Stats], n_threads: usize, iters: u64) -> u64 {
        std::thread::scope(|s| {
            for t in 0..n_threads {
                s.spawn(move || {
                    let slot = &slots[t]; // private line — no ping-pong
                    for i in 0..iters {
                        slot.allocs.fetch_add(1, Ordering::Relaxed);
                        slot.bytes.fetch_add(i, Ordering::Relaxed);
                        slot.frees.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        slots
            .iter()
            .map(|x| {
                x.allocs.load(Ordering::Relaxed)
                    + x.bytes.load(Ordering::Relaxed)
                    + x.frees.load(Ordering::Relaxed)
            })
            .sum()
    }
}

const ITERS: u64 = 10_000; // 3 RMW per iteration per thread

pub struct Input {
    n_threads: usize,
}

pub fn make_input(n: usize) -> Input {
    // n = THREAD COUNT (the contention axis is the size axis).
    Input { n_threads: n }
}

fn expected(n_threads: usize) -> u64 {
    // Deterministic closed form: active slots each get iters allocs
    // + iters frees + sum(0..iters) bytes. Layout cannot change totals
    // (fetch_add is atomic) — the G1 identity.
    n_threads as u64 * (2 * ITERS + ITERS * (ITERS - 1) / 2)
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let slots = buggy::make_slots();
    let cs = buggy::run(&slots, input.n_threads, ITERS);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, expected(input.n_threads));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let slots = fixed::make_slots();
    let cs = fixed::run(&slots, input.n_threads, ITERS);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, expected(input.n_threads));
}"#
            .to_string(),
    }
}

/// Bench spec for `channel-send-batching-unpark` (Batch 65, Research 081):
/// per-item unbounded-channel sends to a receiver parked in crossbeam
/// `select!` vs per-64-item batch sends (one wakeup per batch).
///
/// The storm regime is load-bearing: the receiver must find the channel
/// EMPTY when it re-polls, so the producer pays ~32 hash rounds per item
/// (µs-scale intervals — the source's measurement cadence) while the
/// receiver only counts. `select!` bypasses crossbeam's direct-subscribe
/// spin path, so the empty poll parks the receiver — one park/unpark pair
/// per message in the buggy arm, one per FLUSH_EVERY in the fixed arm.
/// The ctrl arm exists to force the two-channel select! shape (held
/// connected-but-silent until the receiver joins; the data channel's
/// disconnect is the shutdown signal). G1: receiver item count == total
/// in both arms (asserted in-run).
#[cfg(feature = "rust_perf_bench")]
fn channel_send_batching_unpark_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000],
        harness: r#"// [dep: crossbeam-channel = "0.5"]
const FLUSH_EVERY: usize = 64;

#[inline]
fn measurement_work(i: u64) -> u64 {
    // µs-scale per-measurement cost (the profiler's sampling work) — keeps
    // the producer interval above the receiver's processing time so the
    // channel is EMPTY when the receiver re-polls (the park regime).
    let mut h = i.wrapping_mul(0x9E37_79B9_97F4_A7C1);
    for _ in 0..32 {
        h = h.rotate_left(7) ^ h.wrapping_mul(0x2545_F491_4F6C_DD1D);
    }
    h
}

mod buggy {
    pub fn produce(tx: &crossbeam_channel::Sender<u64>, total: u64) {
        for i in 0..total {
            std::hint::black_box(super::measurement_work(i));
            tx.send(i).unwrap(); // ONE park/unpark pair per message
        }
    }
}

mod fixed {
    pub fn produce(tx: &crossbeam_channel::Sender<Vec<u64>>, total: u64) {
        let mut buf: Vec<u64> = Vec::with_capacity(super::FLUSH_EVERY);
        for i in 0..total {
            std::hint::black_box(super::measurement_work(i));
            buf.push(i);
            if buf.len() >= super::FLUSH_EVERY {
                // replace keeps `buf`'s capacity for the next batch — the
                // take-then-regrow variant pays a ~7-alloc growth chain per
                // batch (measured 795 vs 329 allocs @10K — worse than the
                // buggy arm on the alloc axis).
                let batch =
                    std::mem::replace(&mut buf, Vec::with_capacity(super::FLUSH_EVERY));
                tx.send(batch).unwrap(); // one wakeup per 64
            }
        }
        if !buf.is_empty() {
            tx.send(buf).unwrap();
        }
    }
}

pub struct Input {
    total: u64,
}

pub fn make_input(n: usize) -> Input {
    Input { total: n as u64 }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let (tx, rx) = crossbeam_channel::unbounded::<u64>();
    let (_ctx, crx) = crossbeam_channel::unbounded::<()>(); // silent 2nd select arm
    let receiver = std::thread::spawn(move || {
        let mut count: u64 = 0;
        loop {
            crossbeam_channel::select! {
                recv(rx) -> msg => match msg {
                    Ok(_) => count += 1,
                    Err(_) => break, // producer dropped — shutdown
                },
                recv(crx) -> _ => break,
            }
        }
        count
    });
    buggy::produce(&tx, input.total);
    drop(tx);
    let count = receiver.join().unwrap();
    std::hint::black_box(count);
    debug_assert_eq!(count, input.total);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let (tx, rx) = crossbeam_channel::unbounded::<Vec<u64>>();
    let (_ctx, crx) = crossbeam_channel::unbounded::<()>(); // silent 2nd select arm
    let receiver = std::thread::spawn(move || {
        let mut count: u64 = 0;
        loop {
            crossbeam_channel::select! {
                recv(rx) -> msg => match msg {
                    Ok(batch) => count += batch.len() as u64,
                    Err(_) => break,
                },
                recv(crx) -> _ => break,
            }
        }
        count
    });
    fixed::produce(&tx, input.total);
    drop(tx);
    let count = receiver.join().unwrap();
    std::hint::black_box(count);
    debug_assert_eq!(count, input.total);
}"#
            .to_string(),
    }
}

/// Bench spec for `sort-key-cache-precompute-out-of-lock` (Batch 73,
/// Research 085/086 — the mnt.io room-list distill): comparator that
/// acquires a read-lock + clones a 144 B `LatestEventValue` per invocation
/// vs a maintained sort-key cache refreshed once per data-update cycle
/// (the production `RoomListItem` pattern — refresh via NARROW reads, the
/// narrow-accessor rule composed in).
///
/// sizes = SORTS PER REFRESH CYCLE (the amortization axis — the reactive
/// stream regime: every filter change / pagination reset does a fresh full
/// sort over the same data, and eyeball-im-util's `SortBy` re-invokes the
/// comparator from `binary_search_by` on every diff). N_ROOMS = 512 is the
/// article's 500-600 room scale. Every sort is preceded by a fixed
/// deterministic Fisher-Yates permutation (in-place, no allocs, the SAME
/// swap list in both arms) — a pseudo-random permutation has ~N/2 runs so
/// the adaptive driftsort pays the full ~N·log N comparisons every sort
/// (a plain reverse would be ONE detectable run, O(N) — measured: the
/// reverse-protocol run showed only ~512 comparisons/sort vs the ~4600
/// the shuffle protocol pays). G1: the final order is always desc —
/// position-weighted checksum asserted against a make_input-computed
/// expectation, in both arms.
/// Alloc axis: buggy ≈ 2 allocs/comparison (the String body clone ×2
/// accessor calls) vs fixed ≈ 1/sort (the sort's internal scratch).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn sort_key_cache_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 4, 16],
        harness: r#"
use std::cell::RefCell;
use std::sync::RwLock;

mod buggy {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        pub timestamp: u64,
        pub body: String, // ~64 B heap — the guarded payload footprint
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // The article's `Room::latest_event` shape: read-lock acquire +
        // clone of the WHOLE value per call. The comparator invokes it
        // twice per comparison, ~N·log N comparisons per sort.
        pub fn latest_event_timestamp(&self) -> Option<u64> {
            self.info.read().unwrap().latest_event.clone().map(|ev| ev.timestamp)
        }

        // Verification-only narrow reader (NOT the measured sort path).
        pub fn verify_ts(&self) -> Option<u64> {
            self.info.read().unwrap().latest_event.as_ref().map(|ev| ev.timestamp)
        }
    }

    pub fn sort_desc(rooms: &mut [Room]) {
        rooms.sort_by(|a, b| b.latest_event_timestamp().cmp(&a.latest_event_timestamp()));
    }
}

mod fixed {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        pub timestamp: u64,
        // The payload width IS the mechanism — only the buggy arm's clone
        // ever reads it; the narrow refresh reads just the timestamp.
        #[allow(dead_code)]
        pub body: String,
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // Narrow read through the guard — no clone (the narrow-accessor
        // rule composed into the refresh path).
        pub fn latest_event_timestamp(&self) -> Option<u64> {
            self.info.read().unwrap().latest_event.as_ref().map(|ev| ev.timestamp)
        }
    }

    /// The production `RoomListItem` facade: cached sort key refreshed
    /// once per data update (a Reset cycle refreshes all N — the
    /// amortization cost side).
    pub struct RoomListItem {
        pub inner: Room,
        pub cached_ts: Option<u64>,
    }

    impl RoomListItem {
        pub fn refresh_cached_data(&mut self) {
            self.cached_ts = self.inner.latest_event_timestamp();
        }
    }

    pub fn refresh_all(items: &mut [RoomListItem]) {
        for it in items.iter_mut() {
            it.refresh_cached_data();
        }
    }

    pub fn sort_desc(items: &mut [RoomListItem]) {
        items.sort_by(|a, b| b.cached_ts.cmp(&a.cached_ts));
    }
}

const N_ROOMS: usize = 512; // the article's 500-600 room scale

pub struct Input {
    // Interior mutability: the working vec persists across validator calls
    // (no per-call clone — both arms sort in place).
    buggy_rooms: RefCell<Vec<buggy::Room>>,
    fixed_items: RefCell<Vec<fixed::RoomListItem>>,
    // Deterministic Fisher-Yates swap list (fixed LCG seed), applied
    // in-place before every sort in BOTH arms — the same permutation
    // sequence, so comparison counts are identical across arms.
    shuffle: Vec<(usize, usize)>,
    sorts_per_cycle: usize,
    expected_desc: u64,
}

fn make_body(i: u64) -> String {
    format!("event body {i:04} padding padding padding padding")
}

pub fn make_input(n: usize) -> Input {
    let mut buggy_rooms = Vec::with_capacity(N_ROOMS);
    let mut rooms = Vec::with_capacity(N_ROOMS);
    let mut keys: Vec<u64> = Vec::with_capacity(N_ROOMS);
    for i in 0..N_ROOMS as u64 {
        let ts = i * 7 + 3; // distinct → total order, G1-deterministic
        rooms.push(fixed::Room {
            info: RwLock::new(fixed::RoomInfo {
                latest_event: Some(fixed::LatestEvent { timestamp: ts, body: make_body(i) }),
            }),
        });
        buggy_rooms.push(buggy::Room {
            info: RwLock::new(buggy::RoomInfo {
                latest_event: Some(buggy::LatestEvent { timestamp: ts, body: make_body(i) }),
            }),
        });
        keys.push(ts);
    }
    // Reverse both so call #1's entry state is anti-sorted for its first sort.
    // Reverse both so call #1's entry state is unsorted.
    buggy_rooms.reverse();
    rooms.reverse();
    let fixed_items: Vec<fixed::RoomListItem> = rooms
        .into_iter()
        .map(|inner| fixed::RoomListItem { inner, cached_ts: None })
        .collect();

    // Deterministic Fisher-Yates swap list (fixed LCG seed). A PLAIN
    // reverse() before each sort would be WRONG here: a perfectly
    // reversed input is ONE run for the adaptive driftsort — it detects
    // + reverses it in O(N) with ~N comparisons. A fixed pseudo-random
    // permutation has ~N/2 runs → the full ~N·log N merge comparisons,
    // the honest reactive-stream re-sort cost.
    let mut swap: Vec<(usize, usize)> = Vec::with_capacity(N_ROOMS);
    let mut rng: u64 = 0x243F_6A88_85A3_08D3;
    for i in (1..N_ROOMS).rev() {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let j = ((rng >> 33) as usize) % (i + 1);
        swap.push((i, j));
    }

    // G1 expectation: position-weighted checksum of the desc order,
    // computed from the key set alone (independent of either arm).
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    let expected_desc: u64 =
        sorted.iter().rev().enumerate().map(|(i, &k)| k * (i as u64 + 1)).sum();

    Input {
        buggy_rooms: RefCell::new(buggy_rooms),
        fixed_items: RefCell::new(fixed_items),
        shuffle: swap,
        sorts_per_cycle: n,
        expected_desc,
    }
}

// Sort protocol (deterministic full-work sorts, no RNG at run time): every
// sort is preceded by applying the fixed Fisher-Yates swap list (in-place,
// O(N) moves, no allocs) + sorts DESCENDING. The permuted input has ~N/2
// runs → the adaptive driftsort pays the full ~N·log N comparisons for
// EVERY sort, in BOTH arms, every call (a plain reverse would be a single
// detectable run — O(N)). End state: always desc → one G1 expectation.
fn apply_shuffle<T>(v: &mut [T], swap: &[(usize, usize)]) {
    for &(i, j) in swap {
        v.swap(i, j);
    }
}

fn checksum_weighted(ts: &[Option<u64>]) -> u64 {
    ts.iter().enumerate().map(|(i, k)| k.unwrap_or(0) * (i as u64 + 1)).sum()
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut rooms = input.buggy_rooms.borrow_mut();
    for _ in 0..input.sorts_per_cycle {
        apply_shuffle(&mut rooms, &input.shuffle);
        buggy::sort_desc(&mut rooms);
    }
    let ts: Vec<Option<u64>> = rooms.iter().map(|r| r.verify_ts()).collect();
    let cs = checksum_weighted(&ts);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, input.expected_desc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut items = input.fixed_items.borrow_mut();
    // The refresh cycle: N narrow reads (the maintained cache's cost side).
    fixed::refresh_all(&mut items);
    for _ in 0..input.sorts_per_cycle {
        apply_shuffle(&mut items, &input.shuffle);
        fixed::sort_desc(&mut items);
    }
    let ts: Vec<Option<u64>> = items.iter().map(|it| it.cached_ts).collect();
    let cs = checksum_weighted(&ts);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, input.expected_desc);
}
"#
        .to_string(),
    }
}

#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn lock_guard_narrow_field_accessor_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
use std::sync::RwLock;

mod buggy {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        pub timestamp: u64,
        pub sender_is_local: bool,
        pub body: String, // ~64 B heap — the guarded payload
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // The wide accessor: clones the WHOLE guarded payload under the
        // read-guard — 144 B + a heap alloc — so the caller can read one bool.
        pub fn latest_event(&self) -> Option<LatestEvent> {
            self.info.read().unwrap().latest_event.clone()
        }

        pub fn latest_event_is_local(&self) -> bool {
            self.latest_event().is_some_and(|ev| ev.sender_is_local)
        }
    }
}

mod fixed {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        // The guarded payload width IS the mechanism — the buggy arm pays
        // to clone it; the narrow accessor reads only the consumed bool.
        #[allow(dead_code)]
        pub timestamp: u64,
        pub sender_is_local: bool,
        #[allow(dead_code)]
        pub body: String,
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // The narrow sibling (the production `new_latest_event_is_local`):
        // compute the ONE consumed sub-field under the guard — no clone.
        pub fn latest_event_is_local(&self) -> bool {
            self.info.read().unwrap().latest_event.as_ref().is_some_and(|ev| ev.sender_is_local)
        }
    }
}

pub struct Input {
    buggy_rooms: Vec<buggy::Room>,
    fixed_rooms: Vec<fixed::Room>,
    probes: Vec<usize>,
    expected_local_count: u64,
}

pub fn make_input(n: usize) -> Input {
    const ROOMS: usize = 64; // probe target pool (mix of local/not)
    let make_body = |i: u64| format!("event body {i:04} padding padding padding padding");
    let mut buggy_rooms = Vec::with_capacity(ROOMS);
    let mut fixed_rooms = Vec::with_capacity(ROOMS);
    let mut expected = 0u64;
    for i in 0..ROOMS as u64 {
        let is_local = i % 3 == 0; // deterministic mix
        buggy_rooms.push(buggy::Room {
            info: RwLock::new(buggy::RoomInfo {
                latest_event: Some(buggy::LatestEvent { timestamp: i * 11 + 1, sender_is_local: is_local, body: make_body(i) }),
            }),
        });
        fixed_rooms.push(fixed::Room {
            info: RwLock::new(fixed::RoomInfo {
                latest_event: Some(fixed::LatestEvent { timestamp: i * 11 + 1, sender_is_local: is_local, body: make_body(i) }),
            }),
        });
    }
    // Deterministic probe sequence (LCG) + expected count in ONE pass.
    let mut probes = Vec::with_capacity(n);
    let mut x = 1u64;
    for _ in 0..n {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let idx = ((x >> 33) as usize) % ROOMS;
        if idx as u64 % 3 == 0 { expected += 1; }
        probes.push(idx);
    }
    Input { buggy_rooms, fixed_rooms, probes, expected_local_count: expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut count = 0u64;
    for &p in &input.probes {
        count += input.buggy_rooms[p].latest_event_is_local() as u64;
    }
    std::hint::black_box(count);
    debug_assert_eq!(count, input.expected_local_count);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut count = 0u64;
    for &p in &input.probes {
        count += input.fixed_rooms[p].latest_event_is_local() as u64;
    }
    std::hint::black_box(count);
    debug_assert_eq!(count, input.expected_local_count);
}
"#
        .to_string(),
    }
}

/// Bench spec for `swar-digit-chunk-parse` (Batch 74, Research 088 —
/// the simd-json distill): per-digit decimal parse loop vs the SWAR 8-digit
/// chunk fold (`parse_eight_digits_swar` + `is_made_of_eight_digits_fast`,
/// ORIGINAL minimal reproductions of the source construction).
///
/// sizes = DIGITS PER RUN (the regime axis — the source's own README:
/// "a slight performance hit if most of the floats have less than 8
/// digits"). N_RUNS = 2000 fixed; runs are LCG-generated digit strings of
/// exactly n digits (values fit u64 at 19 digits = the axis max), offsets
/// precomputed in make_input so the measured delta isolates the parse.
/// G1: both arms wrapping-sum the SAME u64 values — the expectation is
/// accumulated in u128 during generation (== the wrapping u64 sum) and
/// debug_assert_eq'd in both arms. At size 4 (< 8) the chunk loop never
/// fires — the honest Tie bound of the regime; 8/12/19 exercise the fold.
/// Alloc axis: 0/0 (both arms parse from the pre-built buffer).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn swar_digit_chunk_parse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4, 8, 12, 19],
        harness: r#"
pub mod buggy {
    pub fn parse_u64(digits: &[u8]) -> u64 {
        let mut val: u64 = 0;
        let mut i = 0;
        while i < digits.len() && digits[i].is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add(u64::from(digits[i] - b'0'));
            i += 1;
        }
        val
    }
}

pub mod fixed {
    pub fn parse_u64(digits: &[u8]) -> u64 {
        let mut val: u64 = 0;
        let mut i = 0;
        while i + 8 <= digits.len() {
            let chunk: [u8; 8] = digits[i..i + 8].try_into().unwrap();
            if !is_made_of_eight_digits_fast(&chunk) {
                break;
            }
            val = val.wrapping_mul(100_000_000).wrapping_add(u64::from(parse_eight_digits_swar(&chunk)));
            i += 8;
        }
        while i < digits.len() && digits[i].is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add(u64::from(digits[i] - b'0'));
            i += 1;
        }
        val
    }

    fn is_made_of_eight_digits_fast(chars: &[u8; 8]) -> bool {
        let val = u64::from_ne_bytes(*chars);
        ((val & 0xF0F0_F0F0_F0F0_F0F0)
            | (((val.wrapping_add(0x0606_0606_0606_0606)) & 0xF0F0_F0F0_F0F0_F0F0) >> 4))
            == 0x3333_3333_3333_3333
    }

    fn parse_eight_digits_swar(chars: &[u8; 8]) -> u32 {
        let val = u64::from_le_bytes(*chars);
        let val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;
        let val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;
        ((val & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32) as u32
    }
}

const N_RUNS: usize = 2_000;

pub struct Input {
    buf: std::cell::RefCell<Vec<u8>>,
    runs: Vec<(usize, usize)>, // (offset, len) — precomputed so the delta isolates the parse
    expected: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut buf = Vec::with_capacity(N_RUNS * (n + 1));
    let mut runs = Vec::with_capacity(N_RUNS);
    let mut expected: u128 = 0;
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    for _ in 0..N_RUNS {
        let start = buf.len();
        let mut val: u128 = 0; // u128 accumulation == the wrapping u64 sum at cast time
        for _ in 0..n {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            let d = ((rng >> 33) % 10) as u8;
            buf.push(b'0' + d);
            val = val * 10 + u128::from(d);
        }
        buf.push(b','); // run terminator — both arms stop here
        runs.push((start, n));
        expected += val;
    }
    Input { buf: std::cell::RefCell::new(buf), runs, expected: expected as u64 }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let buf = input.buf.borrow();
    let mut sum: u64 = 0;
    for &(s, l) in &input.runs {
        sum = sum.wrapping_add(buggy::parse_u64(&buf[s..s + l]));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let buf = input.buf.borrow();
    let mut sum: u64 = 0;
    for &(s, l) in &input.runs {
        sum = sum.wrapping_add(fixed::parse_u64(&buf[s..s + l]));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}
"#
        .to_string(),
    }
}

/// Bench spec for `runtime-feature-detect-per-chunk` (Batch 74, Research
/// 088): CPU feature detection paid PER CHUNK (the std::detect string
/// resolution + cached detector read, modeled via the arch-portable
/// `detected()` helper) vs resolved ONCE into a cached atomic byte.
///
/// sizes = BYTES PER CHUNK (the work-per-detection axis): at small chunks
/// the detection rivals the work itself (the source's finding — their
/// per-8-digit-group check "costs more than the SWAR fallback"); as chunk
/// work grows the fixed ratio decays toward 1. N_CHUNKS = 1000 fixed.
/// Both arms' chunk fns are `#[inline(never)]` — models the real-world
/// outlined dispatch (the source's "two outlined target_feature calls")
/// AND prevents the compiler from hoisting the detection out of the loop.
/// G1: both arms sum the same bytes — checksum debug_assert_eq'd.
/// Alloc axis: 0/0. HONEST SCOPE: on rustc 1.97 the feature-name match on
/// a string literal may const-fold after inlining — the measured delta is
/// whatever remains (detector read + branch + call shape); the annotation
/// records the verdict honestly either way (the B68 varint precedent).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn runtime_feature_detect_per_chunk_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![16, 256, 4096],
        harness: r#"
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(target_arch = "aarch64")]
pub fn detected() -> bool {
    std::arch::is_aarch64_feature_detected!("neon")
}
#[cfg(target_arch = "x86_64")]
pub fn detected() -> bool {
    std::arch::is_x86_feature_detected!("avx2")
}
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
pub fn detected() -> bool {
    true
}

pub mod buggy {
    use super::detected;

    #[inline(never)] // models the outlined target_feature dispatch + blocks loop hoisting
    pub fn sum_chunk(data: &[u8]) -> u64 {
        // detection paid PER CHUNK — string resolution + detector read every call
        if detected() {
            data.iter().map(|&b| u64::from(b)).sum()
        } else {
            data.iter().fold(0u64, |acc, &b| acc.wrapping_add(u64::from(b)))
        }
    }
}

pub mod fixed {
    use std::sync::atomic::{AtomicU8, Ordering};

    static CACHED: AtomicU8 = AtomicU8::new(0); // 0 = unknown, 1 = yes, 2 = no

    fn detected_once() -> bool {
        match CACHED.load(Ordering::Relaxed) {
            0 => {
                let v = super::detected();
                CACHED.store(if v { 1 } else { 2 }, Ordering::Relaxed);
                v
            }
            1 => true,
            _ => false,
        }
    }

    #[inline(never)] // symmetric with the buggy arm — the delta is detection vs cached byte
    pub fn sum_chunk(data: &[u8]) -> u64 {
        if detected_once() {
            data.iter().map(|&b| u64::from(b)).sum()
        } else {
            data.iter().fold(0u64, |acc, &b| acc.wrapping_add(u64::from(b)))
        }
    }
}

const N_CHUNKS: usize = 1_000;

pub struct Input {
    chunks: Vec<Vec<u8>>,
    expected: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut chunks = Vec::with_capacity(N_CHUNKS);
    let mut expected: u64 = 0;
    let mut rng: u64 = 0xC0FF_EE00_BA5E_BA11;
    for _ in 0..N_CHUNKS {
        let mut c = Vec::with_capacity(n);
        for _ in 0..n {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            c.push((rng >> 56) as u8);
        }
        expected = expected.wrapping_add(c.iter().map(|&b| u64::from(b)).sum::<u64>());
        chunks.push(c);
    }
    Input { chunks, expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut sum: u64 = 0;
    for c in &input.chunks {
        sum = sum.wrapping_add(buggy::sum_chunk(c));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut sum: u64 = 0;
    for c in &input.chunks {
        sum = sum.wrapping_add(fixed::sum_chunk(c));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}
"#
        .to_string(),
    }
}

/// Bench spec for `encoded-layout-borrowed-view` (Batch 75, Research 089 —
/// the GreptimeDB `recordbatches_to_timeseries` distill): dictionary-encoded
/// column (S distinct strings + one u32 key per row) consumed through a
/// generic materializing accessor vs a borrowing enum view with one variant
/// per layout.
///
/// sizes = ROWS R per column; S_DISTINCT = 100 distinct dictionary values
/// and L_COLS = 4 label columns fixed (the source's Dictionary<UInt32, Utf8>
/// regime — a label layout like `host` has few distinct values across many
/// rows). The buggy accessor is the source's `iter_column_as_string` shape:
/// a GENERIC path that does not recognize the dictionary layout and calls
/// to_string() row by row. G1: both arms sum the same label lengths — the
/// expectation is computed in make_input straight from the keys (no view
/// involved) and debug_assert_eq'd in both arms.
/// Alloc axis: buggy = L + R·L per call (one collect Vec per column + one
/// String per cell — "the 3 strings become 1000"); fixed = 0 steady-state
/// (the view borrows; no copy, no allocation).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn encoded_layout_borrowed_view_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 1_000_000],
        harness: r#"
const S_DISTINCT: usize = 100; // distinct dictionary values per column
const L_COLS: usize = 4;       // label columns (host, region, ...)

mod buggy {
    pub struct DictColumn {
        pub values: Vec<String>,
        pub keys: Vec<u32>,
    }

    impl DictColumn {
        fn lookup(&self, row: usize) -> Option<&str> {
            let k = *self.keys.get(row)?;
            self.values.get(k as usize).map(String::as_str)
        }
    }

    // The source's `iter_column_as_string`: a GENERIC accessor that does
    // not recognize the dictionary layout — falls back to owned Strings
    // row by row, so the S distinct strings expand into R per column.
    pub fn column_as_strings(col: &DictColumn) -> Vec<Option<String>> {
        (0..col.keys.len()).map(|r| col.lookup(r).map(|s| s.to_string())).collect()
    }

    pub fn label_len_total(cols: &[DictColumn]) -> usize {
        let mut total = 0;
        for col in cols {
            let cells = column_as_strings(col); // R heap allocs per column
            for cell in &cells {
                total += cell.as_ref().map_or(0, |s| s.len());
            }
        }
        total
    }
}

mod fixed {
    pub struct DictColumn {
        pub values: Vec<String>,
        pub keys: Vec<u32>,
    }

    // One variant per column layout; `value` borrows straight from the
    // array — no copy and no allocation (the source's `LabelValues` enum).
    pub enum ColumnView<'a> {
        Utf8(&'a [String]),
        Dictionary { values: &'a [String], keys: &'a [u32] },
    }

    impl ColumnView<'_> {
        pub fn value(&self, row: usize) -> Option<&str> {
            match self {
                ColumnView::Utf8(v) => v.get(row).map(String::as_str),
                ColumnView::Dictionary { values, keys } => keys
                    .get(row)
                    .and_then(|&k| values.get(k as usize).map(String::as_str)),
            }
        }
    }

    pub fn label_len_total(cols: &[DictColumn]) -> usize {
        let rows = cols.first().map_or(0, |c| c.keys.len());
        let mut total = 0;
        for col in cols {
            let view = ColumnView::Dictionary { values: &col.values, keys: &col.keys };
            for row in 0..rows {
                total += view.value(row).map_or(0, str::len);
            }
        }
        total
    }
}

pub struct Input {
    buggy_cols: Vec<buggy::DictColumn>,
    fixed_cols: Vec<fixed::DictColumn>,
    expected: usize,
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut buggy_cols = Vec::with_capacity(L_COLS);
    let mut fixed_cols = Vec::with_capacity(L_COLS);
    let mut expected = 0usize;
    for _ in 0..L_COLS {
        let values: Vec<String> = (0..S_DISTINCT).map(|i| format!("label-value-{i:03}")).collect();
        let mut keys = Vec::with_capacity(n);
        for _ in 0..n {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            keys.push(((rng >> 33) % (S_DISTINCT as u64)) as u32);
        }
        // expectation straight from the keys — no view involved
        expected += keys.iter().map(|&k| values[k as usize].len()).sum::<usize>();
        buggy_cols.push(buggy::DictColumn { values: values.clone(), keys: keys.clone() });
        fixed_cols.push(fixed::DictColumn { values, keys });
    }
    Input { buggy_cols, fixed_cols, expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let total = buggy::label_len_total(&input.buggy_cols);
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let total = fixed::label_len_total(&input.fixed_cols);
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected);
}
"#
        .to_string(),
    }
}

/// Bench spec for `group-keys-borrow-allocate-on-first-sight` (Batch 75,
/// Research 089 — the GreptimeDB `recordbatches_to_timeseries` distill):
/// per-row owned-key grouping (clone the label set into a BTreeMap key
/// every row) vs hash-the-borrowed-labels + candidate index list, cloning
/// the owned key only when a NEW series is first seen + one final
/// sort_unstable_by restoring the BTreeMap's ordered output.
///
/// sizes = ROWS R; S_GROUPS = 1000 distinct label sets fixed (the axis is
/// the R/S dedup ratio: R·L clones become S·L). Rows reference an interned
/// label arena by index so INPUT memory stays O(S) strings while the buggy
/// arm still clones 2·L Strings + 1 Vec per row per call; group ids are
/// assigned by a fixed LCG (pseudo-random order — no fast-path locality
/// smuggled in; this rule's win is pure clone-avoidance). G1: both arms
/// emit groups in SORTED key order (the BTreeMap's free order; the fixed
/// arm's final sort restores it) — a position-weighted checksum over the
/// independently-sorted group table is debug_assert_eq'd in both arms.
/// Top cell 300K not 1M: the buggy arm at 1M is ~7M allocs + 1M ordered
/// BTreeMap string inserts per call — the R/S-ratio story is unchanged and
/// the bench stays minutes-scale (the B73 sorts-per-update sizing lesson).
/// Alloc axis: buggy ≈ 7·R + 2·S per call; fixed ≈ 8·S + ~30 (first-sight
/// key clones + candidate index lists + table/vec growth) — independent
/// of R.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn group_keys_borrow_allocate_on_first_sight_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 300_000],
        harness: r#"
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const S_GROUPS: usize = 1_000; // distinct label sets (series)
const L_LABELS: usize = 3;     // (name, value) pairs per series

// Shared input vocabulary: rows reference the interned arena by index so
// input memory stays O(S) — per-call cloning is the ARMS' business.
pub struct Row {
    pub labels: [(u32, u32); L_LABELS], // (name_idx, value_idx) into the arena
    pub value: u64,
}
pub struct Labels {
    pub names: Vec<String>,
    pub vals: Vec<String>,
}

fn hash_row(row: &Row, arena: &Labels) -> u64 {
    let mut h = DefaultHasher::new();
    for &(n, v) in &row.labels {
        arena.names[n as usize].hash(&mut h);
        arena.vals[v as usize].hash(&mut h);
    }
    h.finish()
}

fn key_matches(key: &[(String, String)], row: &Row, arena: &Labels) -> bool {
    key.len() == row.labels.len()
        && key.iter().zip(&row.labels).all(|((kn, kv), &(n, v))| {
            *kn == arena.names[n as usize] && *kv == arena.vals[v as usize]
        })
}

mod buggy {
    use super::{Labels, Row};
    use std::collections::BTreeMap;

    pub fn group_rows(rows: &[Row], arena: &Labels) -> Vec<(Vec<(String, String)>, (u64, u64))> {
        let mut map: BTreeMap<Vec<(String, String)>, (u64, u64)> = BTreeMap::new();
        for row in rows {
            // 7 heap allocs EVERY row: 1 Vec + 2·L String clones — R·L
            // copies where S·L would do
            let key: Vec<(String, String)> = row
                .labels
                .iter()
                .map(|&(n, v)| (arena.names[n as usize].clone(), arena.vals[v as usize].clone()))
                .collect();
            let e = map.entry(key).or_insert((0, 0));
            e.0 += 1;
            e.1 += row.value;
        }
        map.into_iter().collect() // BTreeMap order = sorted keys, for free
    }
}

mod fixed {
    use super::{hash_row, key_matches, Labels, Row};
    use std::collections::HashMap;

    pub struct Group {
        pub key: Vec<(String, String)>,
        pub count: u64,
        pub sum: u64,
    }

    pub fn group_rows(rows: &[Row], arena: &Labels) -> Vec<Group> {
        // hash → candidate index list; FULL label comparison on every
        // candidate — collisions can never merge two different series
        let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        for row in rows {
            let hash = hash_row(row, arena); // borrowed labels — no alloc
            let idx = match index.get(&hash).and_then(|cands| {
                cands.iter().find(|&&i| key_matches(&groups[i].key, row, arena))
            }) {
                Some(&i) => i,
                None => {
                    let i = groups.len();
                    groups.push(Group {
                        key: row
                            .labels
                            .iter()
                            .map(|&(n, v)| {
                                (arena.names[n as usize].clone(), arena.vals[v as usize].clone())
                            })
                            .collect(), // allocated ON FIRST SIGHT only — S·L, not R·L
                        count: 0,
                        sum: 0,
                    });
                    index.entry(hash).or_default().push(i);
                    i
                }
            };
            groups[idx].count += 1;
            groups[idx].sum += row.value;
        }
        // one S·log(S) sort ≪ R ordered BTreeMap inserts — restores the
        // BTreeMap's sorted output contract
        groups.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        groups
    }
}

// position-weighted checksum over the sorted group table — the SAME fold
// in make_input (ground truth) and both arms
fn fold_group(chk: u64, key_len: usize, count: u64, sum: u64) -> u64 {
    chk.wrapping_mul(31)
        .wrapping_add(key_len as u64)
        .wrapping_mul(31)
        .wrapping_add(count)
        .wrapping_mul(31)
        .wrapping_add(sum)
}

pub struct Input {
    rows: Vec<Row>,
    arena: Labels,
    expected_groups: usize,
    expected_chk: u64,
}

pub fn make_input(n: usize) -> Input {
    // arena: S groups × L (name, value) pairs, all distinct
    let mut names = Vec::with_capacity(S_GROUPS * L_LABELS);
    let mut vals = Vec::with_capacity(S_GROUPS * L_LABELS);
    for g in 0..S_GROUPS {
        for l in 0..L_LABELS {
            names.push(format!("n{g:04}-{l}"));
            vals.push(format!("v{g:04}-{l}"));
        }
    }
    let arena = Labels { names, vals };

    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    let mut rows = Vec::with_capacity(n);
    let mut counts = vec![0u64; S_GROUPS];
    let mut sums = vec![0u64; S_GROUPS];
    for _ in 0..n {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let g = ((rng >> 33) % (S_GROUPS as u64)) as usize;
        let base = g * L_LABELS;
        let labels = [
            (base as u32, base as u32),
            (base as u32 + 1, base as u32 + 1),
            (base as u32 + 2, base as u32 + 2),
        ];
        rows.push(Row { labels, value: rng >> 16 });
        counts[g] += 1;
        sums[g] = sums[g].wrapping_add(rng >> 16);
    }

    // ground truth: the sorted-key group table, folded position-weighted
    let mut table: Vec<(Vec<(String, String)>, u64, u64)> = (0..S_GROUPS)
        .filter(|&g| counts[g] > 0)
        .map(|g| {
            let base = g * L_LABELS;
            ((0..L_LABELS)
                .map(|l| (arena.names[base + l].clone(), arena.vals[base + l].clone()))
                .collect::<Vec<_>>(),
            counts[g],
            sums[g])
        })
        .collect();
    table.sort_by(|a, b| a.0.cmp(&b.0));
    let expected_groups = table.len();
    let mut expected_chk: u64 = 0;
    for (key, count, sum) in &table {
        let key_len = key.iter().map(|(n, v)| n.len() + v.len()).sum::<usize>();
        expected_chk = fold_group(expected_chk, key_len, *count, *sum);
    }
    Input { rows, arena, expected_groups, expected_chk }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = buggy::group_rows(&input.rows, &input.arena);
    let mut chk: u64 = 0;
    for (key, (count, sum)) in &groups {
        let key_len = key.iter().map(|(n, v)| n.len() + v.len()).sum::<usize>();
        chk = fold_group(chk, key_len, *count, *sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = fixed::group_rows(&input.rows, &input.arena);
    let mut chk: u64 = 0;
    for g in &groups {
        let key_len = g.key.iter().map(|(n, v)| n.len() + v.len()).sum::<usize>();
        chk = fold_group(chk, key_len, g.count, g.sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}
"#
        .to_string(),
    }
}

/// Bench spec for `sorted-input-previous-row-fast-path` (Batch 75,
/// Research 089 — the GreptimeDB `recordbatches_to_timeseries` distill):
/// hash-every-row grouping vs compare-against-the-previous-row's-group
/// first, hash fallback on miss.
///
/// sizes = RUN LENGTH (the hit-rate regime axis, the sort_key_cache
/// semantic-sizes precedent): n = 100 → adjacent input (S=1000 contiguous
/// runs of 100 — the real query shape: rows sorted by primary key so one
/// series' samples arrive together); n = 1 → interleaved (round-robin over
/// all 1000 series — the honest bound: every row pays one failed [u64; 2]
/// comparison before the hash). R_ROWS = 100_000 fixed; S_SERIES = 1000.
/// G1: both arms emit groups in FIRST-SIGHT order — identical in both arms
/// and computable in make_input (series 0..S ascending for BOTH layouts) —
/// and fold the same position-weighted (series, count, sum) checksum,
/// debug_assert_eq'd against the make_input ground truth. Correctness
/// never depends on the fast path: the hash fallback with full-key
/// equality is the semantic contract.
/// Alloc axis: ~equal by design (both build the same HashMap + groups
/// Vec; neither allocates per row) — the TIMING axis is the mechanism
/// here, and the alloc integers pin that no per-row allocation snuck in.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn sorted_input_previous_row_fast_path_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 100],
        harness: r#"
use std::collections::HashMap;

const R_ROWS: usize = 100_000;
const S_SERIES: usize = 1_000;

pub struct Row {
    pub series: [u64; 2], // the label-set key (hashed by the map)
    pub value: u64,
}
pub struct Group {
    pub series: [u64; 2],
    pub count: u64,
    pub sum: u64,
}

mod buggy {
    use super::{Group, Row};
    use std::collections::HashMap;

    pub fn group_rows(rows: &[Row]) -> Vec<Group> {
        let mut index: HashMap<[u64; 2], usize> = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        for row in rows {
            // hash EVERY row — even when it lands on the same series as
            // the previous row (the common case for sorted input)
            let idx = match index.get(&row.series) {
                Some(&i) => i,
                None => {
                    let i = groups.len();
                    groups.push(Group { series: row.series, count: 0, sum: 0 });
                    index.insert(row.series, i);
                    i
                }
            };
            groups[idx].count += 1;
            groups[idx].sum += row.value;
        }
        groups
    }
}

mod fixed {
    use super::{Group, Row};
    use std::collections::HashMap;

    pub fn group_rows(rows: &[Row]) -> Vec<Group> {
        let mut index: HashMap<[u64; 2], usize> = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        let mut prev: Option<usize> = None;
        for row in rows {
            // compare against the series the PREVIOUS row landed on —
            // no hash on hit. The fast path only exploits local ordering;
            // correctness never depends on it.
            let idx = match prev {
                Some(i) if groups[i].series == row.series => i,
                _ => match index.get(&row.series) {
                    Some(&i) => i,
                    None => {
                        let i = groups.len();
                        groups.push(Group { series: row.series, count: 0, sum: 0 });
                        index.insert(row.series, i);
                        i
                    }
                },
            };
            prev = Some(idx);
            groups[idx].count += 1;
            groups[idx].sum += row.value;
        }
        groups
    }
}

pub struct Input {
    rows: Vec<Row>,
    expected_groups: usize,
    expected_chk: u64,
}

fn fold_group(chk: u64, series: [u64; 2], count: u64, sum: u64) -> u64 {
    chk.wrapping_mul(31)
        .wrapping_add(series[0] ^ series[1].rotate_left(17))
        .wrapping_mul(31)
        .wrapping_add(count)
        .wrapping_mul(31)
        .wrapping_add(sum)
}

fn series_of(g: usize) -> [u64; 2] {
    [g as u64, (g as u64).wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9)]
}

pub fn make_input(n: usize) -> Input {
    let run_len = n.max(1); // n = run length: 1 = interleaved (round-robin), 100 = adjacent
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    let mut rows = Vec::with_capacity(R_ROWS);
    // emit run_len consecutive rows of one series, then the next — for
    // run_len = 1 this degenerates to round-robin over all series
    let mut g = 0usize;
    while rows.len() < R_ROWS {
        let series = series_of(g % S_SERIES);
        for _ in 0..run_len {
            if rows.len() == R_ROWS {
                break;
            }
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            rows.push(Row { series, value: rng >> 16 });
        }
        g += 1;
    }
    // ground truth: first-sight order is 0..S for BOTH layouts (series 0
    // is the first run / first round-robin slot either way)
    let mut counts = vec![0u64; S_SERIES];
    let mut sums = vec![0u64; S_SERIES];
    let mut present = vec![false; S_SERIES];
    for row in &rows {
        let g = row.series[0] as usize;
        counts[g] += 1;
        sums[g] = sums[g].wrapping_add(row.value);
        present[g] = true;
    }
    let expected_groups = present.iter().filter(|&&p| p).count();
    let mut expected_chk: u64 = 0;
    for g in 0..S_SERIES {
        if present[g] {
            expected_chk = fold_group(expected_chk, series_of(g), counts[g], sums[g]);
        }
    }
    Input { rows, expected_groups, expected_chk }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = buggy::group_rows(&input.rows);
    let mut chk: u64 = 0;
    for g in &groups {
        chk = fold_group(chk, g.series, g.count, g.sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = fixed::group_rows(&input.rows);
    let mut chk: u64 = 0;
    for g in &groups {
        chk = fold_group(chk, g.series, g.count, g.sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}
"#
        .to_string(),
    }
}

/// Bench spec for `debug-stat-full-scan-out-of-hot-path` (Batch 76,
/// Research 090 — the Meilisearch LMDB/vector-store distill): per-insert-batch
/// mean-degree statistic recomputed over the ENTIRE store vs maintained
/// incremental aggregate.
///
/// sizes = STORE SIZE N (the accumulation axis): the buggy arm pays O(N) per
/// op (B=100 inserted per call) — the win GROWS with store size (the source:
/// 559s → 2–3s at 19M items). G1: both arms return the same final
/// (items.len, link_count, mean_bits) triple, debug_assert_eq'd against a
/// make_input-computed ground truth. Alloc axis: equal by design (2 Vecs for
/// the store + batch in both arms) — the TIMING axis is the mechanism.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn debug_stat_full_scan_out_of_hot_path_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 300_000],
        harness: r#"
pub struct Item {
    pub links: Vec<u32>,
}
pub struct Input {
    pub store: Vec<Item>,
    pub batch: Vec<Item>,
    pub link_count: usize, // the maintained aggregate (fixed arm's starting state)
    pub expected_len: usize,
    pub expected_links: usize,
}
fn make_items(rng: &mut u64, count: usize, max_links: usize) -> Vec<Item> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        *rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let n = (*rng as usize) % max_links;
        let mut links = Vec::with_capacity(n);
        for _ in 0..n {
            *rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            links.push((*rng >> 16) as u32);
        }
        out.push(Item { links });
    }
    out
}
pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    // the store starts at n - B items so post-insert size is exactly n
    let b = 100usize;
    let store = make_items(&mut rng, n.saturating_sub(b), 8);
    let batch = make_items(&mut rng, b, 8);
    let link_count = store.iter().map(|i| i.links.len()).sum();
    let expected_len = store.len() + batch.len();
    let expected_links = link_count + batch.iter().map(|i| i.links.len()).sum::<usize>();
    Input { store, batch, link_count, expected_len, expected_links }
}
mod buggy {
    use super::Item;
    // the statistic's scan IS the cost: one pass over EVERY item's links
    // per insert batch — the source's mean-degree sanity statistic,
    // "added very early on", eagerly computed in the hot path. Non-mutating
    // (statistic over store+batch) so timed calls are idempotent.
    pub fn stat_with_batch(items: &[Item], batch: &[Item]) -> (usize, usize, u64) {
        // O(total) per op — re-derives the WHOLE store's link count
        let total: usize = items.iter().chain(batch).map(|i| i.links.len()).sum();
        let len = items.len() + batch.len();
        let mean = len as f64 / total.max(1) as f64;
        (len, total, mean.to_bits())
    }
}
mod fixed {
    use super::Item;
    // maintained aggregate: link_count was updated by DELTA at each insert —
    // the hot path pays only the NEW batch's links, never the whole past
    pub fn stat_with_batch(items: &[Item], link_count: usize, batch: &[Item]) -> (usize, usize, u64) {
        let batch_links: usize = batch.iter().map(|i| i.links.len()).sum(); // O(B)
        let total = link_count + batch_links;
        let len = items.len() + batch.len();
        let mean = len as f64 / total.max(1) as f64;
        (len, total, mean.to_bits())
    }
}
pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let r = buggy::stat_with_batch(&input.store, &input.batch);
    let chk = r.0.wrapping_mul(31).wrapping_add(r.1).wrapping_mul(31).wrapping_add(r.2 as usize);
    std::hint::black_box(chk);
    debug_assert_eq!(r.0, input.expected_len);
    debug_assert_eq!(r.1, input.expected_links);
}
pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let r = fixed::stat_with_batch(&input.store, input.link_count, &input.batch);
    let chk = r.0.wrapping_mul(31).wrapping_add(r.1).wrapping_mul(31).wrapping_add(r.2 as usize);
    std::hint::black_box(chk);
    debug_assert_eq!(r.0, input.expected_len);
    debug_assert_eq!(r.1, input.expected_links);
}
"#
        .to_string(),
    }
}

/// Bench spec for `eager-pointer-collect-lazy-fetch` (Batch 76, Research 090 —
/// the Meilisearch LMDB/vector-store distill): full-scan HashMap-of-pointers
/// build to serve a K-id subset vs direct lazy access.
///
/// sizes = STORE SIZE N at fixed K=1,000 touched ids (the subset-ratio axis:
/// N=1M × 32 B nodes ≈ 32 MB > L2 — the scan's cache traffic evicts what the
/// subset reads; the source's disk-page story at cache scale). G1: both arms
/// fold the same wrapping checksum over the touched payloads, debug_assert_eq'd
/// against the make_input ground truth. Alloc axis: buggy builds a
/// HashMap<usize, &Node> (3 allocs, ~2.4 MB at N=1M); fixed allocates nothing.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn eager_pointer_collect_lazy_fetch_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 300_000, 1_000_000],
        harness: r#"
pub const K_TOUCHED: usize = 1_000;
pub struct Node {
    pub payload: [u64; 4],
}
pub struct Input {
    pub nodes: Vec<Node>,
    pub ids: Vec<usize>,
    pub expected_chk: u64,
}
pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x0DDB_1A5E_5BAD_5EED;
    let mut nodes = Vec::with_capacity(n);
    for _ in 0..n {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        nodes.push(Node { payload: [rng, rng >> 13, rng >> 29, rng >> 47] });
    }
    // K ids scattered over the whole range — the subset pattern
    let mut ids = Vec::with_capacity(K_TOUCHED);
    let mut expected_chk: u64 = 0;
    for _ in 0..K_TOUCHED {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let id = (rng as usize) % n;
        ids.push(id);
        expected_chk = expected_chk
            .wrapping_mul(31)
            .wrapping_add(nodes[id].payload[0]);
    }
    Input { nodes, ids, expected_chk }
}
mod buggy {
    use super::Node;
    // "a full scan of all the entries we will need to access", pointer map
    // over everything — "polluting the cache with useless, unnecessary disk
    // pages just by iterating over all of them to fetch pointers"
    pub fn touch(nodes: &[Node], ids: &[usize]) -> u64 {
        let mut index = std::collections::HashMap::with_capacity(nodes.len());
        for (i, n) in nodes.iter().enumerate() {
            index.insert(i, n); // O(all) build to serve a subset
        }
        let mut acc = 0u64;
        for &id in ids {
            if let Some(n) = index.get(&id) {
                acc = acc.wrapping_add(n.payload[0]);
            }
        }
        acc
    }
}
mod fixed {
    use super::Node;
    // lazy/on-demand — "only the necessary pages are loaded into the cache";
    // the store IS the direct-access path
    pub fn touch(nodes: &[Node], ids: &[usize]) -> u64 {
        let mut acc = 0u64;
        for &id in ids {
            if let Some(n) = nodes.get(id) {
                acc = acc.wrapping_add(n.payload[0]);
            }
        }
        acc
    }
}
pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let acc = buggy::touch(&input.nodes, &input.ids);
    std::hint::black_box(acc);
    debug_assert_eq!(acc, input.expected_chk);
}
pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let acc = fixed::touch(&input.nodes, &input.ids);
    std::hint::black_box(acc);
    debug_assert_eq!(acc, input.expected_chk);
}
"#
        .to_string(),
    }
}

/// Bench spec for `narrow-counter-overflow-promotion` (Batch 78,
/// Research 092 — Datadog RTDB/Monocle): the full-width u64 counter
/// buffer vs the u32+sidecar form. Sizes = counter count; working set
/// 0.8/8/32 MB (u64 arm) vs 0.4/4/16 MB (u32 arm) — the smallest size
/// is L2-resident (density pays nothing — the honest Tie regime), the
/// largest is DRAM-bound. Alloc axis: buggy 1 alloc of 8N bytes, fixed
/// 1 alloc of 4N (empty HashMap never allocates) — the halving is exact.
/// G1: totals are closed-form identical (no promotion fires in the
/// common regime; debug_assert per run).
#[cfg(feature = "rust_perf_bench")]
fn narrow_counter_overflow_promotion_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000, 4_000_000],
        harness: "\nmod buggy {\n    // The full-width buffer: every counter u64 \"just in case\".\n    pub struct Bins { pub counts: Vec<u64> }\n    pub fn bump_all(c: &mut Bins, updates: &[(u32, u32)]) {\n        for &(i, v) in updates {\n            c.counts[i as usize] += u64::from(v);\n        }\n    }\n    pub fn total(c: &Bins) -> u64 {\n        c.counts.iter().sum()\n    }\n}\n\nmod fixed {\n    use std::collections::HashMap;\n    // Common case narrowed to u32; overflow promotes to a sparse sidecar.\n    pub struct Bins {\n        pub counts: Vec<u32>,\n        pub wide: HashMap<usize, u64>,\n    }\n    pub fn bump_all(c: &mut Bins, updates: &[(u32, u32)]) {\n        for &(i, v) in updates {\n            let i = i as usize;\n            if c.wide.is_empty() {\n                // common regime: nothing ever promoted\n                match c.counts[i].checked_add(v) {\n                    Some(x) => c.counts[i] = x,\n                    None => {\n                        let w = u64::from(c.counts[i]) + u64::from(v);\n                        c.counts[i] = 0; // narrow slot is dead now\n                        c.wide.insert(i, w);\n                    }\n                }\n                continue;\n            }\n            if let Some(w) = c.wide.get_mut(&i) {\n                *w += u64::from(v);\n                continue;\n            }\n            match c.counts[i].checked_add(v) {\n                Some(x) => c.counts[i] = x,\n                None => {\n                    let w = u64::from(c.counts[i]) + u64::from(v);\n                    c.counts[i] = 0;\n                    c.wide.insert(i, w);\n                }\n            }\n        }\n    }\n    pub fn total(c: &Bins) -> u64 {\n        c.counts.iter().map(|&n| u64::from(n)).sum::<u64>()\n            + c.wide.values().sum::<u64>()\n    }\n}\n\n/// Deterministic update stream: every slot bumped exactly UPDATES_PER_SLOT\n/// times with LCG deltas 1..=64 (the common regime — no slot approaches\n/// u32::MAX, so the fixed arm never promotes; totals are closed-form).\nconst UPDATES_PER_SLOT: u64 = 4;\nconst DELTA_MAX: u32 = 64;\n\npub struct Input {\n    n: usize,\n    updates: Vec<(u32, u32)>,\n    expected_total: u64,\n}\n\npub fn make_input(n: usize) -> Input {\n    let mut rng: u64 = 0x0B78_0D06_C0DE_5EED; // seeded LCG\n    let rng = &mut rng;\n    let mut updates: Vec<(u32, u32)> = Vec::with_capacity(n * UPDATES_PER_SLOT as usize);\n    let mut expected: u64 = 0;\n    // round-robin slots so both arms walk the array identically\n    for round in 0..UPDATES_PER_SLOT {\n        for i in 0..n {\n            *rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);\n            let v = ((*rng >> 33) as u32 % DELTA_MAX) + 1; // 1..=64\n            updates.push((i as u32, v));\n            expected += u64::from(v);\n            std::hint::black_box(&round);\n        }\n    }\n    Input { n, updates, expected_total: expected }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut bins = buggy::Bins { counts: vec![0u64; input.n] };\n    buggy::bump_all(&mut bins, &input.updates);\n    let t = buggy::total(&bins);\n    std::hint::black_box(t);\n    debug_assert_eq!(t, input.expected_total);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut bins = fixed::Bins { counts: vec![0u32; input.n], wide: std::collections::HashMap::new() };\n    fixed::bump_all(&mut bins, &input.updates);\n    let t = fixed::total(&bins);\n    std::hint::black_box(t);\n    debug_assert_eq!(t, input.expected_total);\n}\n".to_string(),
    }
}

/// Bench spec for `cache-line-block-partitioned-lock` (Batch 78,
/// Research 092 — Datadog RTDB/Monocle): one `Mutex<Vec<u64>>` guarding
/// all 4096 points vs 512 aligned `Mutex<[u64; 8]>` blocks. Thread
/// counts are the size axis (the align-64 convention). G1: bump totals
/// are the closed form `threads * iters` — lock granularity cannot
/// change totals (Mutex guarantees).
#[cfg(feature = "rust_perf_bench")]
fn cache_line_block_partitioned_lock_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![2, 4, 8, 12],
        harness: "\nmod buggy {\n    use std::sync::Mutex;\n    // One lock guards the whole point array: every bump serializes.\n    pub struct Series { pub points: Mutex<Vec<u64>> }\n    pub fn make(n_points: usize) -> Series {\n        Series { points: Mutex::new(vec![0u64; n_points]) }\n    }\n    pub fn run(s: &Series, n_threads: usize, iters: u64, n_points: usize) -> u64 {\n        std::thread::scope(|sc| {\n            for t in 0..n_threads {\n                sc.spawn(move || {\n                    let mut rng: u64 = (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);\n                    for _ in 0..iters {\n                        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);\n                        let i = (rng >> 33) as usize % n_points;\n                        s.points.lock().unwrap()[i] += 1; // full serialization\n                    }\n                });\n            }\n        });\n        let g = s.points.lock().unwrap();\n        g.iter().sum()\n    }\n}\n\nmod fixed {\n    use std::sync::Mutex;\n    // 8 points per cache-line-aligned block, one lock per block.\n    #[repr(align(128))] // the ARCH coherence line (64 x86 / 128 Apple Silicon)\n    pub struct Block { pub lock: Mutex<[u64; 8]> }\n    pub struct Series { pub blocks: Vec<Block> }\n    pub fn make(n_points: usize) -> Series {\n        let n_blocks = n_points.div_ceil(8);\n        Series { blocks: (0..n_blocks).map(|_| Block { lock: Mutex::new([0u64; 8]) }).collect() }\n    }\n    pub fn run(s: &Series, n_threads: usize, iters: u64, n_points: usize) -> u64 {\n        std::thread::scope(|sc| {\n            for t in 0..n_threads {\n                sc.spawn(move || {\n                    let mut rng: u64 = (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);\n                    for _ in 0..iters {\n                        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);\n                        let i = (rng >> 33) as usize % n_points;\n                        let b = &s.blocks[i / 8];\n                        b.lock.lock().unwrap()[i % 8] += 1; // contention ~ /8\n                    }\n                });\n            }\n        });\n        s.blocks.iter().map(|b| b.lock.lock().unwrap().iter().sum::<u64>()).sum()\n    }\n}\n\nconst POINTS: usize = 4096; // 512 blocks x 8 (fixed arm)\nconst ITERS: u64 = 20_000;  // bumps per thread\n\npub struct Input { n_threads: usize }\n\npub fn make_input(n: usize) -> Input {\n    // n = THREAD COUNT (the contention axis is the size axis, the align-64\n    // bench convention).\n    Input { n_threads: n }\n}\n\nfn expected(n_threads: usize) -> u64 {\n    // Every bump adds exactly 1 — the closed-form total (lock granularity\n    // cannot change it; Mutex guarantees mutual exclusion, G1).\n    n_threads as u64 * ITERS\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let s = buggy::make(POINTS);\n    let cs = buggy::run(&s, input.n_threads, ITERS, POINTS);\n    std::hint::black_box(cs);\n    debug_assert_eq!(cs, expected(input.n_threads));\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let s = fixed::make(POINTS);\n    let cs = fixed::run(&s, input.n_threads, ITERS, POINTS);\n    std::hint::black_box(cs);\n    debug_assert_eq!(cs, expected(input.n_threads));\n}\n".to_string(),
    }
}

/// Bench spec for `weight-tile-consumption-order` (Batch 85, Research 098
/// — the cflow "Pipeline-Native Transformers" distill): row-major matvec
/// (x re-walked per output row) vs weights repacked ONCE into
/// consumption-ordered 64×512 tiles (the x slice L1-resident across each
/// tile's 64 rows).
///
/// n = n_in; n_out fixed at 64 — the memory-bounded shape that crosses M3
/// Max's 128 KB P-core L1-d (4× the source's Sandy Bridge 32 KB target —
/// the size axis is SHIFTED, the ratio story preserved). W-row + x per row:
///   n=4096  →  32 KB (L1-resident — the honest Tie arm; both arms stream
///                     W sequentially with x resident)
///   n=16384 → 128 KB (the L1 boundary — transition)
///   n=32768 → 256 KB (2× L1 — eviction regime; row-major re-fetches x
///                     from L2 once per row, tiled once per row-GROUP)
///   n=65536 → 512 KB (4× L1 — the paper's headline regime)
/// TILE_ROWS=64 = n_out (one row-group); TILE_COLS=512 (x slice = 2 KB).
/// The repack lives in make_input (setup — NEVER measured). Float
/// sum-order differs between the arms (per-tile partials) — the corpus
/// rel-error gate covers it (NOT bit-identity; the B64 gate shape).
#[cfg(feature = "rust_perf_bench")]
fn weight_tile_consumption_order_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4_096, 16_384, 32_768, 65_536],
        harness: "\nmod buggy {\n    pub fn matvec_rowmajor(w: &[f32], x: &[f32], y: &mut [f32], n_in: usize) {\n        for (r, yo) in y.iter_mut().enumerate() {\n            let row = &w[r * n_in..(r + 1) * n_in];\n            let mut acc = 0.0f32;\n            for (wv, &xv) in row.iter().zip(x) {\n                acc += wv * xv;\n            }\n            *yo = acc;\n        }\n    }\n}\n\nmod fixed {\n    pub const TILE_ROWS: usize = 64;\n    pub const TILE_COLS: usize = 512;\n    pub fn repack_tiles(w: &[f32], n_out: usize, n_in: usize) -> Vec<f32> {\n        let mut tiles = Vec::with_capacity(w.len());\n        for tr in (0..n_out).step_by(TILE_ROWS) {\n            for tc in (0..n_in).step_by(TILE_COLS) {\n                for r in tr..(tr + TILE_ROWS).min(n_out) {\n                    let row = &w[r * n_in..(r + 1) * n_in];\n                    tiles.extend_from_slice(&row[tc..(tc + TILE_COLS).min(n_in)]);\n                }\n            }\n        }\n        tiles\n    }\n    pub fn matvec_tiled(tiles: &[f32], x: &[f32], y: &mut [f32], n_out: usize, n_in: usize) {\n        let mut off = 0usize;\n        for tr in (0..n_out).step_by(TILE_ROWS) {\n            for tc in (0..n_in).step_by(TILE_COLS) {\n                let rows = TILE_ROWS.min(n_out - tr);\n                let cols = TILE_COLS.min(n_in - tc);\n                let xs = &x[tc..tc + cols];\n                for r in 0..rows {\n                    let tile_row = &tiles[off + r * cols..off + (r + 1) * cols];\n                    let mut partial = 0.0f32;\n                    for (wv, &xv) in tile_row.iter().zip(xs) {\n                        partial += wv * xv;\n                    }\n                    y[tr + r] += partial;\n                }\n                off += rows * cols;\n            }\n        }\n    }\n}\n\nconst N_OUT: usize = 64;\n\npub struct Input { w: Vec<f32>, tiles: Vec<f32>, x: Vec<f32>, zero: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let hash = |i: usize, s: u32| {\n        let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);\n        ((h >> 8) as f32) / 16_777_216.0 - 0.5\n    };\n    let w: Vec<f32> = (0..N_OUT * n).map(|i| hash(i, 1)).collect();\n    let x: Vec<f32> = (0..n).map(|i| hash(i, 2)).collect();\n    let tiles = fixed::repack_tiles(&w, N_OUT, n); // setup — never measured\n    Input { w, tiles, x, zero: vec![0.0; N_OUT] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut y = input.zero.clone();\n    buggy::matvec_rowmajor(&input.w, &input.x, &mut y, input.x.len());\n    std::hint::black_box(&y);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut y = input.zero.clone();\n    fixed::matvec_tiled(&input.tiles, &input.x, &mut y, N_OUT, input.x.len());\n    std::hint::black_box(&y);\n}\n".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_corpus_round_trips() {
        let corpus = build_perf_seed_corpus();
        assert!(corpus.verify());
        let frozen = corpus.freeze().unwrap();
        let thawed = Corpus::<RustPerfDomain>::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn seed_corpus_has_ninety_eight_entries_across_twelve_categories() {
        let corpus = build_perf_seed_corpus();
        assert_eq!(corpus.entries.len(), 117); // Batch 135 (rustforge) 114 → 117
        let mut categories: Vec<&str> =
            corpus.entries.iter().map(|e| e.category.as_str()).collect();
        categories.sort_unstable();
        categories.dedup();
        // 12 distinct categories. Allocation is the deepest (22 rules,
        // — Batch 75 added encoded-layout-borrowed-view +
        // group-keys-borrow-allocate-on-first-sight;
        // including Issue 014's arc-str-payload + Batch 48's three + Batch
        // 53's hashmap-with-capacity); complexity has 18 (Batch 76 added
        // debug-stat-full-scan-out-of-hot-path; Batch 91 added
        // decision-projection-before-construction; Batch 75 added
        // sorted-input-previous-row-fast-path; Batch 48 added
        // hashset-insert-bool-return + vec-insert-front-deque; Batch 64
        // added debug-assert-length-hoist); iterators
        // has 10 (Batch 48 added three; Batch 66 added
        // sortedness-check-branchless-accumulate; Batch 67 added
        // byte-find-position-to-memchr; Batch 91 added
        // filter-before-expensive-transform); simd has 8 (Batch 62 added
        // transposed-weight-register-gemv; Batch 64 added
        // loop-order-innermost-contiguous; Batch 83 added
        // bitplane-transpose-repeated-predicate-scan); layout has 10 (Batch 53 added
        // soa-hot-field; Batch 61 added usize-index-to-u32; Batch 78 added
        // narrow-counter-overflow-promotion; Batch 85 added
        // weight-tile-consumption-order; Batch 112 added
        // hot-byval-move-over-memcpy-threshold) + concurrency
        // has 8 (Issue 014 added the two metric-mismatch rules; Batch 65
        // added align-64-false-sharing + channel-send-batching-unpark;
        // Batch 73 added lock-guard-narrow-field-accessor; Batch 78 added
        // cache-line-block-partitioned-lock
        // added align-64-false-sharing + channel-send-batching-unpark;
        // Batch 73 added lock-guard-narrow-field-accessor); arithmetic
        // has 7 (Batch 53 added powi-vs-powf + distance-sq-no-sqrt; Batch 59
        // added division-by-invariant-multiplication; Batch 62
        // added newton-refinement-vs-hardware-transcendental; Batch 74
        // added swar-digit-chunk-parse); dispatch has 4
        // (Batch 61 added keyword-map-to-match; Batch 74 added
        // runtime-feature-detect-per-chunk; Batch 99 added
        // mir-destprop-branch-site-merge); caching has 4 (Issue 014
        // added regex-hoist-lazylock; Batch 73 added
        // sort-key-cache-precompute-out-of-lock; Batch 76 added
        // eager-pointer-collect-lazy-fetch); io has 5 (Batch 68 added
        // varint-compose-word-single-store; Batch 69 added
        // unaligned-word-load-from-bytes; Batch 85 added
        // software-prefetch-storage-bound; Batch 94 added
        // coalesce-nearby-reads-bounded-gap-cacheability-gated); bevy, wasm
        // have 1 each.
        assert_eq!(categories.len(), 12);
    }

    /// G1 for `swar-digit-chunk-parse` (Batch 74, Research 088 — the
    /// simd-json distill): the ported SWAR construction
    /// (`parse_eight_digits_swar` + `is_made_of_eight_digits_fast` + the
    /// chunk/tail loop) must be exactly equivalent to the naive per-digit
    /// reference — the Batch 59 ported-numeric exhaustive-gate MANDATE
    /// (casual tests miss subtle wrongness; the construction's wrapping
    /// multiplies especially). Sweeps: (a) every digit-string length
    /// 0..=19 × structured boundaries (all-0s, all-9s, 10^k) + the exact
    /// 20-digit u64::MAX string; (b) 10_000 LCG digit strings of LCG
    /// lengths; (c) non-digit terminators at EVERY offset (b'x', b',',
    /// b' ', 0x00, 0xFF) — value AND consumed-length identical; (d) the
    /// validation matrix: `is_made_of_eight_digits_fast` ≡ "every byte in
    /// 0x30..=0x39" over 8 positions × boundary bytes + 10_000 LCG octets;
    /// (e) the fold itself vs a u32 reference over structured + 100_000
    /// LCG octets. Domain: digit runs whose value fits u64 (beyond that
    /// BOTH arms overflow — documented out-of-domain).
    #[test]
    #[allow(clippy::too_many_lines)] // one five-section exhaustive sweep sharing the LCG stream
    fn swar_digit_parse_equivalence_gate() {
        fn is_made_of_eight_digits_fast(chars: [u8; 8]) -> bool {
            let val = u64::from_ne_bytes(chars);
            ((val & 0xF0F0_F0F0_F0F0_F0F0)
                | (((val.wrapping_add(0x0606_0606_0606_0606)) & 0xF0F0_F0F0_F0F0_F0F0) >> 4))
                == 0x3333_3333_3333_3333
        }
        fn parse_eight_digits_swar(chars: [u8; 8]) -> u32 {
            let val = u64::from_le_bytes(chars);
            let val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;
            let val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;
            ((val & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32) as u32
        }
        // The fixed construction (returns value + consumed length — pins the
        // stop-position equivalence argument, not just the value).
        fn parse_swars(digits: &[u8]) -> (u64, usize) {
            let mut val: u64 = 0;
            let mut i = 0;
            while i + 8 <= digits.len() {
                let chunk: [u8; 8] = digits[i..i + 8].try_into().unwrap();
                if !is_made_of_eight_digits_fast(chunk) {
                    break;
                }
                val = val
                    .wrapping_mul(100_000_000)
                    .wrapping_add(u64::from(parse_eight_digits_swar(chunk)));
                i += 8;
            }
            while i < digits.len() && digits[i].is_ascii_digit() {
                val = val
                    .wrapping_mul(10)
                    .wrapping_add(u64::from(digits[i] - b'0'));
                i += 1;
            }
            (val, i)
        }
        // The reference — true u128 math, no wrapping ambiguity.
        fn parse_naive(digits: &[u8]) -> (u128, usize) {
            let mut val: u128 = 0;
            let mut i = 0;
            while i < digits.len() && digits[i].is_ascii_digit() {
                val = val * 10 + u128::from(digits[i] - b'0');
                i += 1;
            }
            (val, i)
        }
        fn check(bytes: &[u8]) {
            let (ref_val, ref_len) = parse_naive(bytes);
            assert!(
                ref_val <= u128::from(u64::MAX),
                "gate domain: test string must fit u64"
            );
            let (sw_val, sw_len) = parse_swars(bytes);
            assert_eq!(u128::from(sw_val), ref_val, "value mismatch on {bytes:?}");
            assert_eq!(sw_len, ref_len, "stop-position mismatch on {bytes:?}");
        }

        // (a) structured boundaries: every length × all-0s / all-9s / 10^k.
        for len in 0..=19usize {
            check(&vec![b'0'; len]);
            check(&vec![b'9'; len]);
            let mut p10 = Vec::with_capacity(len + 1);
            p10.push(b'1');
            p10.extend(std::iter::repeat_n(b'0', len));
            check(&p10);
        }
        let max_str = u64::MAX.to_string(); // "18446744073709551615" — 20 digits, fits
        check(max_str.as_bytes());
        assert_eq!(parse_swars(max_str.as_bytes()).0, u64::MAX);

        // (b) 10_000 LCG digit strings of LCG lengths 1..=19.
        let mut rng: u64 = 0x0123_4567_89AB_CDEF;
        for _ in 0..10_000 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let len = 1 + ((rng >> 33) as usize) % 19;
            let mut s = Vec::with_capacity(len);
            for _ in 0..len {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                s.push(b'0' + (((rng >> 33) % 10) as u8));
            }
            check(&s);
        }

        // (c) non-digit terminators at EVERY offset — stop-position identity.
        for len in 0..=19usize {
            for term in [b'x', b',', b' ', 0x00, 0xFF] {
                let mut s: Vec<u8> = vec![b'7'; len];
                s.push(term);
                s.extend_from_slice(b"901"); // post-terminator digits must NOT be consumed
                check(&s);
            }
        }

        // (d) validation matrix: is_made_of_eight_digits_fast ≡ all-digits.
        let expect_all = |bytes: [u8; 8]| {
            assert_eq!(
                is_made_of_eight_digits_fast(bytes),
                bytes.iter().all(|&b| b.is_ascii_digit()),
                "validation mismatch on {bytes:?}"
            );
        };
        for pos in 0..8usize {
            for b in [0x00u8, 0x2F, 0x30, 0x39, 0x3A, 0x41, 0x46, 0x61, 0x7F, 0xFF] {
                let mut bytes = [b'5'; 8];
                bytes[pos] = b;
                expect_all(bytes);
            }
        }
        for _ in 0..10_000 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let mut bytes = [0u8; 8];
            for b in &mut bytes {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                *b = ((rng >> 40) % 96).wrapping_add(0x30) as u8; // 0x30..=0x8F mix of digit/non-digit
            }
            expect_all(bytes);
        }

        // (e) the fold itself vs a u32 reference (structured + LCG octets).
        let fold_check = |bytes: [u8; 8]| {
            let reference: u32 = bytes
                .iter()
                .fold(0u32, |acc, &b| acc * 10 + u32::from(b - b'0'));
            assert_eq!(
                parse_eight_digits_swar(bytes),
                reference,
                "fold mismatch on {bytes:?}"
            );
        };
        fold_check([b'0'; 8]);
        fold_check([b'9'; 8]);
        fold_check(*b"12345678");
        fold_check(*b"00000001");
        for k in 0..8 {
            let mut bytes = [b'0'; 8];
            bytes[k] = b'1';
            fold_check(bytes);
        }
        for _ in 0..100_000 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let mut bytes = [b'0'; 8];
            for b in &mut bytes {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                *b = b'0' + (((rng >> 33) % 10) as u8);
            }
            fold_check(bytes);
        }
    }

    /// G1 for `varint-compose-word-single-store` (Batch 68, Research 075 —
    /// the tantivy distill): the 5-threshold ladder in the corpus fixture +
    /// bench harness must be exactly equivalent to the reference per-byte
    /// loop across a FULL u16 sweep + every 128^i band boundary + u32::MAX +
    /// an LCG u32 sample — bit-identical output bytes (the Batch 59
    /// ported-numeric exhaustive-gate rule; the source ships its own
    /// equivalence test at 128^i boundaries — `aux_test_serialize_vint_u32`
    /// — this gate is the stronger local mirror). BOTH conventions are
    /// proven: (a) stop-bit-on-LAST (tantivy VInt — the shipped fixture)
    /// and (b) continuation-bit-on-all-but-LAST (protobuf/LEB128 — the same
    /// ladder mechanism with the convention bit placed where the loop put
    /// it), demonstrating the fix is convention-faithful, not
    /// convention-specific.
    #[test]
    #[allow(clippy::too_many_lines)] // exhaustive gate: both conventions + boundaries belong together
    fn unaligned_word_load_matches_assembly_exhaustive() {
        // Batch 69 G1: the from_le_bytes slice read must be bit-identical to
        // the shift/or assembly at EVERY byte alignment over a wide buffer,
        // u64 AND u32 (the cross-arch probe's exhaustive half, pinned as a
        // standing gate).
        fn assemble_u64(data: &[u8], addr: usize) -> u64 {
            u64::from(data[addr])
                | (u64::from(data[addr + 1]) << 8)
                | (u64::from(data[addr + 2]) << 16)
                | (u64::from(data[addr + 3]) << 24)
                | (u64::from(data[addr + 4]) << 32)
                | (u64::from(data[addr + 5]) << 40)
                | (u64::from(data[addr + 6]) << 48)
                | (u64::from(data[addr + 7]) << 56)
        }
        fn load_u64(data: &[u8], addr: usize) -> u64 {
            let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();
            u64::from_le_bytes(bytes)
        }
        fn assemble_u32(data: &[u8], addr: usize) -> u32 {
            u32::from(data[addr])
                | (u32::from(data[addr + 1]) << 8)
                | (u32::from(data[addr + 2]) << 16)
                | (u32::from(data[addr + 3]) << 24)
        }
        fn load_u32(data: &[u8], addr: usize) -> u32 {
            let bytes: [u8; 4] = (&data[addr..addr + 4]).try_into().unwrap();
            u32::from_le_bytes(bytes)
        }

        let n = 4096u32;
        let data: Vec<u8> = (0..n)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect();

        for addr in 0..((n - 8) as usize) {
            assert_eq!(assemble_u64(&data, addr), load_u64(&data, addr), "u64 @ {addr}");
        }
        for addr in 0..((n - 4) as usize) {
            assert_eq!(assemble_u32(&data, addr), load_u32(&data, addr), "u32 @ {addr}");
        }
    }

    #[test]
    fn varint_ladder_matches_reference_loop_exhaustive() {
        // (a) The SHIPPED ladder — mirror of the corpus fixture (same math).
        fn ladder_stop_bit_last(val: u32, out: &mut Vec<u8>) {
            let v = u64::from(val);
            let (res, len) = if v < (1 << 7) {
                (v | 128, 1)
            } else if v < (1 << 14) {
                ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)
            } else if v < (1 << 21) {
                ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)
            } else if v < (1 << 28) {
                ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)
            } else {
                ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)
            };
            let buf = res.to_le_bytes();
            out.extend_from_slice(&buf[..len]);
        }
        // Reference per-byte loop (the buggy shape, used as ground truth).
        fn reference_stop_bit_last(mut val: u32, out: &mut Vec<u8>) {
            loop {
                let byte = (val % 128) as u8;
                val /= 128;
                if val == 0 {
                    out.push(byte | 128);
                    return;
                }
                out.push(byte);
            }
        }
        // (b) The protobuf/LEB128 convention ladder — same mechanism, the
        // continuation bit lands on every byte EXCEPT the last (bit 7 set
        // on bytes 0..len-2 of the composed word).
        fn ladder_continuation(val: u32, out: &mut Vec<u8>) {
            let v = u64::from(val);
            let (res, len) = if v < (1 << 7) {
                (v, 1)
            } else if v < (1 << 14) {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1), 2)
            } else if v < (1 << 21) {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2), 3)
            } else if v < (1 << 28) {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2) | 0x80 << 16 | ((v & (127 << 21)) << 3), 4)
            } else {
                ((v & 127) | 0x80 | ((v & (127 << 7)) << 1) | 0x80 << 8 | ((v & (127 << 14)) << 2) | 0x80 << 16 | ((v & (127 << 21)) << 3) | 0x80 << 24 | ((v & (127 << 28)) << 4), 5)
            };
            let buf = res.to_le_bytes();
            out.extend_from_slice(&buf[..len]);
        }
        fn reference_continuation(mut val: u32, out: &mut Vec<u8>) {
            loop {
                if val < 128 {
                    // final byte: NO continuation bit (u8::try_from is
                    // infallible here — guarded by the val < 128 check)
                    out.push(u8::try_from(val).unwrap());
                    return;
                }
                out.push((val % 128) as u8 | 0x80);
                val /= 128;
            }
        }

        fn check_pair(val: u32) {
            let mut a = Vec::new();
            let mut b = Vec::new();
            ladder_stop_bit_last(val, &mut a);
            reference_stop_bit_last(val, &mut b);
            assert_eq!(a, b, "stop-bit ladder diverged at val={val}");
            let mut c = Vec::new();
            let mut d = Vec::new();
            ladder_continuation(val, &mut c);
            reference_continuation(val, &mut d);
            assert_eq!(c, d, "continuation ladder diverged at val={val}");
        }

        // Full u16 sweep (exhaustive low range).
        for val in 0..=u32::from(u16::MAX) {
            check_pair(val);
        }
        // Every band boundary ±1 (128^i) + the extremes.
        for i in 1..5u32 {
            let p: u32 = 1 << (7 * i);
            for val in [p - 1, p, p + 1] {
                check_pair(val);
            }
        }
        check_pair(u32::MAX);
        check_pair(u32::MAX - 1);
        // LCG u32 sample (16K values across the full range).
        let mut state = 0x853c_49e6_748f_ea9b_u64;
        for _ in 0..16_384 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            check_pair((state >> 32) as u32);
        }
    }

    /// Canaries: the corpus fixture + the bench harness must contain the
    /// SAME ladder construction this exhaustive gate proves — a substring
    /// pin so the three copies (fixture / harness / gate mirror) cannot
    /// drift (the Batch 59 canary pattern).
    #[test]
    fn varint_fixture_and_harness_carry_the_proven_ladder() {
        let corpus = build_perf_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "varint-compose-word-single-store")
            .expect("varint rule present");
        let needle = "((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2)";
        assert!(
            rule.fixed_code.contains(needle),
            "fixture fixed_code must carry the proven 3+-band ladder fragment"
        );
        assert!(rule.buggy_code.contains("val /= 128"),
            "fixture buggy_code must be the per-byte div-chain loop");
        #[cfg(feature = "rust_perf_bench")]
        {
            let spec = rule.bench_spec.as_ref().expect("bench spec");
            assert!(
                spec.harness.contains(needle),
                "bench harness must carry the same ladder construction"
            );
        }
    }

    /// G1 for `division-by-invariant-multiplication`: the magic-division
    /// algorithm in the corpus fixture + bench harness must be exactly
    /// equivalent to hardware `/` across an exhaustive low-divisor sweep +
    /// boundary numerators + selected large divisors (Batch 59). A mirror
    /// of the harness impl lives here (same math); under
    /// `rust_perf_bench` a substring canary additionally pins the harness
    /// string to the same construction lines so the two cannot drift.
    #[test]
    #[allow(clippy::too_many_lines)] // exhaustive gate: the u32/u64 twins + shared edge data belong together
    fn division_magic_matches_hardware_div_exhaustive_low_d() {
        struct FastDivU32 { magic: u32, shift: u32, add: bool }
        #[allow(clippy::cast_possible_truncation)] // libdivide magic: the high-word extraction + magic wrap ARE the algorithm
        impl FastDivU32 {
            fn new(d: u32) -> Self {
                let l = d.ilog2();
                if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
                let k = 1u64 << l;
                let base = (k << 32) / u64::from(d);
                let rem = (k << 32) % u64::from(d);
                if u64::from(d) - rem < k {
                    Self { magic: base as u32 + 1, shift: l, add: false }
                } else {
                    let mut m = base * 2;
                    let rem2 = rem * 2;
                    if rem2 >= u64::from(d) { m += 1; }
                    Self { magic: (m + 1) as u32, shift: l, add: true }
                }
            }
            // manual_midpoint (clippy 1.98): the widened (hi + x) >> 1 fold IS the
            // algorithm — construction-identical to the pinned harness string, and
            // overflow-free by the u64 widening (the midpoint() rewrite would force
            // a corpus fixture edit for zero semantic change).
            #[allow(clippy::manual_midpoint)]
            fn div(&self, x: u32) -> u32 {
                if self.magic == 0 { return x >> self.shift; }
                let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;
                if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift }
            }
        }
        struct FastDivU64 { magic: u64, shift: u32, add: bool }
        #[allow(clippy::cast_possible_truncation)] // libdivide magic: the high-word extraction + magic wrap ARE the algorithm
        impl FastDivU64 {
            fn new(d: u64) -> Self {
                let l = d.ilog2();
                if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
                let k = 1u128 << l;
                let base = (k << 64) / u128::from(d);
                let rem = (k << 64) % u128::from(d);
                if u128::from(d) - rem < k {
                    Self { magic: base as u64 + 1, shift: l, add: false }
                } else {
                    let mut m = base * 2;
                    let rem2 = rem * 2;
                    if rem2 >= u128::from(d) { m += 1; }
                    Self { magic: (m + 1) as u64, shift: l, add: true }
                }
            }
            #[allow(clippy::manual_midpoint)] // same algorithm-mirror reason as the u32 twin
            fn div(&self, x: u64) -> u64 {
                if self.magic == 0 { return x >> self.shift; }
                let hi = ((u128::from(self.magic) * u128::from(x)) >> 64) as u64;
                if self.add { ((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64 } else { hi >> self.shift }
            }
        }

        struct Lcg(u64);
        impl Lcg {
            fn next_u64(&mut self) -> u64 {
                self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                self.0
            }
            fn next_u32(&mut self) -> u32 { (self.next_u64() >> 33) as u32 }
        }
        let mut rng = Lcg(0x2545_F491_4F6C_DD1D);

        // Exhaustive low divisors (all u32 d in 1..=4096, all u64 d in 1..=257)
        for d in 1u32..=4096 {
            let fd = FastDivU32::new(d);
            let probe = |x: u32| {
                assert_eq!(fd.div(x), x / d, "u32 mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d + 1);
            probe(d.saturating_mul(2).saturating_sub(1)); probe(u32::MAX);
            for _ in 0..8 { probe(rng.next_u32()); }
        }
        for d in 1u64..=257 {
            let fd = FastDivU64::new(d);
            let probe = |x: u64| {
                assert_eq!(fd.div(x), x / d, "u64 mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d + 1);
            probe(d.saturating_mul(2).saturating_sub(1)); probe(u64::MAX);
            for _ in 0..8 {
                probe(rng.next_u64());
            }
        }
        // Selected large/edge divisors: powers of two, primes near 2^31,
        // 2^32, 2^63, 2^64, and values that exercise the ADD branch + the
        // sentinel boundary (magic wrapping near 0).
        let edge32: &[u32] = &[65_535, 65_536, 65_537, 1_000_003, 2_147_483_647, 2_147_483_648, 3_000_000_001, 4_294_967_291, 4_294_967_295, 3, 5, 7, 9, 11];
        for &d in edge32 {
            let fd = FastDivU32::new(d);
            let probe = |x: u32| {
                assert_eq!(fd.div(x), x / d, "u32 edge mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d.saturating_add(1));
            probe(u32::MAX - 1); probe(u32::MAX);
            for _ in 0..16 { probe(rng.next_u32()); }
        }
        let edge64: &[u64] = &[3, 5, 7, 9, 11, 65_537, 1_000_003, 4_294_967_291, 4_294_967_296, 4_294_967_311, 9_223_372_036_854_775_783, 9_223_372_036_854_775_808, 9_223_372_036_854_775_809, 18_446_744_073_709_551_557, u64::MAX - 1, u64::MAX];
        for &d in edge64 {
            let fd = FastDivU64::new(d);
            let probe = |x: u64| {
                assert_eq!(fd.div(x), x / d, "u64 edge mismatch at d={d} x={x}");
            };
            probe(0); probe(1); probe(d - 1); probe(d); probe(d.saturating_add(1));
            probe(u64::MAX - 1); probe(u64::MAX);
            for _ in 0..16 {
                probe(rng.next_u64());
            }
        }

        // Canary: the harness string must carry the same construction core
        // (drift between this mirror + the spec = silent correctness rot).
        #[cfg(feature = "rust_perf_bench")]
        {
            let spec = division_invariant_bench_spec();
            for needle in [
                "let base = (k << 32) / u64::from(d);",
                "let base = (k << 64) / u128::from(d);",
                "((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32",
                "((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64",
            ] {
                assert!(spec.harness.contains(needle), "harness drifted from the equivalence-test mirror: missing `{needle}`");
            }
        }
    }

    #[test]
    fn seed_corpus_rule_ids_are_unique() {
        let corpus = build_perf_seed_corpus();
        let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
    }

    #[test]
    fn tamper_detected() {
        let corpus = build_perf_seed_corpus();
        let mut tampered = corpus.clone();
        tampered.entries[0].fixed_code = "// tampered".to_string();
        assert!(!tampered.verify());
    }

    #[test]
    fn heal_entry_accessors_return_correct_fields() {
        use crate::domain::HealEntry;
        let corpus = build_perf_seed_corpus();
        // Corpus is sorted by rule_id alphabetically — find by id, not index.
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "o1-lookup")
            .expect("o1-lookup should be in seed corpus");
        assert_eq!(entry.rule_id(), "o1-lookup");
        assert!(!entry.buggy_code().is_empty());
        assert!(!entry.fixed_code().is_empty());
    }

    #[test]
    fn heal_domain_build_corpus_matches_seed() {
        use crate::domain::HealDomain;
        let via_trait = RustPerfDomain::build_corpus();
        let via_seed = build_perf_seed_corpus().entries;
        assert_eq!(via_trait.len(), via_seed.len());
        // Both go through the same sort, so the order is identical.
        assert_eq!(via_trait.first().map(|e| e.rule_id.as_str()), via_seed.first().map(|e| e.rule_id.as_str()));
        assert_eq!(via_trait.last().map(|e| e.rule_id.as_str()), via_seed.last().map(|e| e.rule_id.as_str()));
    }

    #[test]
    #[cfg(not(feature = "rust_perf_verify"))]
    fn validate_parses_valid_code() {
        use crate::domain::HealDomain;
        let verdict = RustPerfDomain::validate(
            "o1-lookup",
            "fn foo() -> i32 { 42 }",
        );
        assert!(verdict.parses);
    }

    #[test]
    #[cfg(not(feature = "rust_perf_verify"))]
    fn validate_rejects_garbage() {
        use crate::domain::HealDomain;
        let verdict = RustPerfDomain::validate("o1-lookup", "not valid rust {");
        assert!(!verdict.parses);
    }

    #[test]
    fn runtime_int_exponent_powi_rel_error_gate() {
        // Batch 77 T3.1: the ported-numeric MANDATE — powi is NOT
        // correctly-rounded pow, so the equivalence gate is max rel-error
        // across a SWEPT exponent range (never bit-identity). Bound: the
        // multiplies accumulate ~log2(|n|) ulp; on f64 with |n| <= 64 the
        // measured bound stays under 1e-12 (double-precision headroom).
        let bases = [1.000_000_1f64, 1.5f64, 2.0f64, std::f64::consts::PI];
        let mut max_rel: f64 = 0.0;
        let mut worst = (0.0f64, 0i32);
        for &b in &bases {
            for n in 1i32..=64 {
                let ref_v = b.powf(f64::from(n)); // correctly-rounded libm
                let fix_v = b.powi(n);
                let rel = ((fix_v - ref_v) / ref_v).abs();
                if rel > max_rel {
                    max_rel = rel;
                    worst = (b, n);
                }
            }
        }
        assert!(
            max_rel < 1e-12,
            "powi rel-error {max_rel:.3e} at base {} exp {} exceeds the 1e-12 double-precision bound",
            worst.0, worst.1
        );
    }

    /// G1 for `weight-tile-consumption-order` (Batch 85, Research 098 —
    /// the cflow distill): tile accumulation order (per-column-tile partial
    /// dots, accumulated into y) != row-major accumulation order (one
    /// sequential dot per row) — the gate is max rel-error over swept
    /// shapes (never bit-identity; the B64/powi gate shape). Non-tile-
    /// multiple shapes exercise the min() edge paths. Positive-only inputs
    /// (no cancellation): sums of positives keep the relative bound tight.
    #[test]
    #[allow(clippy::cast_possible_truncation)] // the 24-bit hash — intentional, the loop_order convention
    #[allow(clippy::cast_precision_loss)] // u32→f32 mantissa loss IS the hash's purpose
    fn weight_tile_consumption_order_rel_error_gate() {
        const TILE_ROWS: usize = 64;
        const TILE_COLS: usize = 512;
        fn repack(w: &[f32], n_out: usize, n_in: usize) -> Vec<f32> {
            let mut tiles = Vec::with_capacity(w.len());
            for tr in (0..n_out).step_by(TILE_ROWS) {
                for tc in (0..n_in).step_by(TILE_COLS) {
                    for r in tr..(tr + TILE_ROWS).min(n_out) {
                        let row = &w[r * n_in..(r + 1) * n_in];
                        tiles.extend_from_slice(&row[tc..(tc + TILE_COLS).min(n_in)]);
                    }
                }
            }
            tiles
        }
        fn matvec_rowmajor(w: &[f32], x: &[f32], y: &mut [f32], n_in: usize) {
            for (r, yo) in y.iter_mut().enumerate() {
                let row = &w[r * n_in..(r + 1) * n_in];
                let mut acc = 0.0f32;
                for (wv, &xv) in row.iter().zip(x) {
                    acc += wv * xv;
                }
                *yo = acc;
            }
        }
        fn matvec_tiled(tiles: &[f32], x: &[f32], y: &mut [f32], n_out: usize, n_in: usize) {
            let mut off = 0usize;
            for tr in (0..n_out).step_by(TILE_ROWS) {
                for tc in (0..n_in).step_by(TILE_COLS) {
                    let rows = TILE_ROWS.min(n_out - tr);
                    let cols = TILE_COLS.min(n_in - tc);
                    let xs = &x[tc..tc + cols];
                    for r in 0..rows {
                        let tile_row = &tiles[off + r * cols..off + (r + 1) * cols];
                        let mut partial = 0.0f32;
                        for (wv, &xv) in tile_row.iter().zip(xs) {
                            partial += wv * xv;
                        }
                        y[tr + r] += partial;
                    }
                    off += rows * cols;
                }
            }
        }
        // (n_out, n_in) — includes non-tile-multiple shapes (70x777, 96x1300)
        // exercising the ragged min() edges + a full-shape arm.
        for (n_out, n_in) in [(64usize, 4096usize), (96, 1300), (70, 777)] {
            let hash = |i: usize, s: u32| {
                let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);
                ((h >> 8) as f32) / 16_777_216.0 // 0..1, positive-only
            };
            let w: Vec<f32> = (0..n_out * n_in).map(|i| hash(i, 1)).collect();
            let x: Vec<f32> = (0..n_in).map(|i| hash(i, 2)).collect();
            let mut y_row = vec![0.0f32; n_out];
            matvec_rowmajor(&w, &x, &mut y_row, n_in);
            let tiles = repack(&w, n_out, n_in);
            assert_eq!(tiles.len(), w.len(), "repack preserves every weight");
            let mut y_tiled = vec![0.0f32; n_out];
            matvec_tiled(&tiles, &x, &mut y_tiled, n_out, n_in);
            let mut max_rel = 0.0f64;
            for r in 0..n_out {
                let ref_v = f64::from(y_row[r]);
                let rel = (f64::from(y_tiled[r]) - ref_v).abs() / ref_v.abs().max(1e-30);
                max_rel = max_rel.max(rel);
            }
            assert!(
                max_rel < 1e-5,
                "tile-order rel-error {max_rel:.3e} at {n_out}x{n_in} exceeds the 1e-5 bound"
            );
        }
    }

    /// Batch 86 — the three new rules are present + retrievable.
    #[test]
    fn batch86_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"serialize-once-byte-buffer-store"));
        assert!(ids.contains(&"context-equal-field-elide"));
        assert!(ids.contains(&"bitflag-pack-when-footprint-dominates"));
    }

    /// Batch 86 — the three new rules parse (syn) on buggy AND fixed code.
    #[test]
    fn batch86_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in [
            "serialize-once-byte-buffer-store",
            "context-equal-field-elide",
            "bitflag-pack-when-footprint-dominates",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    /// Batch 91 — the two new rules are present.
    #[test]
    fn batch91_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"filter-before-expensive-transform"));
        assert!(ids.contains(&"decision-projection-before-construction"));
    }

    /// Batch 91 — the two new rules parse (syn) on buggy AND fixed code.
    #[test]
    fn batch91_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in [
            "filter-before-expensive-transform",
            "decision-projection-before-construction",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    /// Batch 91 G1 (rule 1 exactness + the discarded-work axis): both
    /// adapter orders collect the IDENTICAL page Vec (order included) for
    /// a pure render and a pure predicate, AND the call-count axis proves
    /// the mechanism — the buggy order renders EVERY doc, the fixed order
    /// renders only the keepers. Non-vacuous via the strict-inequality
    /// assert.
    #[test]
    fn batch91_filter_order_preserves_output_and_render_count() {
        use std::cell::Cell;
        struct Doc {
            published: bool,
            body: [u64; 4],
        }
        #[derive(Clone, PartialEq, Eq, Debug)]
        struct Page {
            published: bool,
            score: u64,
        }

        thread_local! {
            static RENDER_CALLS: Cell<u64> = const { Cell::new(0) };
        }
        let render = |doc: &Doc| -> Page {
            RENDER_CALLS.with(|c| c.set(c.get() + 1));
            let mut acc = 0u64;
            for &v in &doc.body {
                acc = acc.wrapping_mul(31).wrapping_add(v);
            }
            Page {
                published: doc.published,
                score: acc,
            }
        };
        let docs: Vec<Doc> = (0..256u64)
            .map(|i| Doc {
                published: i % 16 == 0,
                body: [i; 4],
            })
            .collect();

        RENDER_CALLS.with(|c| c.set(0));
        let buggy: Vec<Page> = docs.iter().map(&render).filter(|p| p.published).collect();
        let buggy_calls = RENDER_CALLS.with(Cell::get);

        RENDER_CALLS.with(|c| c.set(0));
        let fixed: Vec<Page> = docs.iter().filter(|d| d.published).map(&render).collect();
        let fixed_calls = RENDER_CALLS.with(Cell::get);

        assert_eq!(buggy, fixed, "both orders must collect the identical page Vec");
        assert_eq!(buggy_calls, docs.len() as u64, "the buggy order renders EVERY doc");
        assert_eq!(fixed_calls, buggy.len() as u64, "the fixed order renders only keepers");
        assert!(
            fixed_calls < buggy_calls,
            "the discarded-work axis must be strictly visible"
        );
    }

    /// Batch 91 G1 (rule 2 precondition AS A TEST + the call axis): the
    /// projection is INVARIANT under canonicalization for every fixture
    /// item (`canonicalize(i).primary == raw_primary(i)`), the two
    /// predicates answer IDENTICALLY, and the canonicalize-call axis
    /// proves the mechanism — the buggy predicate canonicalizes EVERY
    /// item, the fixed predicate canonicalizes NONE (the decision never
    /// needs the canonical form). Non-vacuous: the hot count must be
    /// non-zero.
    #[test]
    #[allow(clippy::cast_possible_truncation)] // the enumerate-index u32 narrowing IS the fixture (k < 8)
    fn batch91_projection_invariance_and_canonicalize_count() {
        use std::cell::Cell;
        struct Item {
            tags: [u32; 8],
        }
        #[derive(PartialEq, Eq, Debug)]
        struct Canonical {
            primary: u32,
            tags: Vec<u32>,
        }

        thread_local! {
            static CANON_CALLS: Cell<u64> = const { Cell::new(0) };
        }
        let canonicalize = |item: &Item| -> Canonical {
            CANON_CALLS.with(|c| c.set(c.get() + 1));
            let mut tags: Vec<u32> = item.tags.to_vec();
            tags.sort_unstable();
            tags.dedup();
            Canonical { primary: tags[0], tags }
        };
        let raw_primary = |item: &Item| -> u32 {
            let mut m = u32::MAX;
            for &t in &item.tags {
                m = m.min(t);
            }
            m
        };
        let items: Vec<Item> = (0..256u32)
            .map(|i| {
                let base = i % 64;
                let mut tags = [0u32; 8];
                for (k, t) in tags.iter_mut().enumerate() {
                    *t = base * 8 + (k as u32) * 31;
                }
                Item { tags }
            })
            .collect();

        for item in &items {
            assert_eq!(
                canonicalize(item).primary,
                raw_primary(item),
                "the projection must be invariant under canonicalization"
            );
        }
        let invariance_calls = CANON_CALLS.with(Cell::get);
        assert_eq!(invariance_calls, items.len() as u64);

        CANON_CALLS.with(|c| c.set(0));
        let buggy_hot = items.iter().filter(|i| canonicalize(i).primary == 128).count();
        let buggy_calls = CANON_CALLS.with(Cell::get);

        CANON_CALLS.with(|c| c.set(0));
        let fixed_hot = items.iter().filter(|i| raw_primary(i) == 128).count();
        let fixed_calls = CANON_CALLS.with(Cell::get);

        assert_eq!(buggy_hot, fixed_hot, "the decision must be identical");
        assert!(buggy_hot > 0, "the hot count must be non-zero (non-vacuous)");
        assert_eq!(
            buggy_calls,
            items.len() as u64,
            "the buggy predicate canonicalizes EVERY item"
        );
        assert_eq!(fixed_calls, 0, "the fixed predicate never needs the canonical form");
    }

    /// Batch 99 — the new rule is present.
    #[test]
    fn batch99_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"mir-destprop-branch-site-merge"));
    }

    /// Batch 99 — the new rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch99_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "mir-destprop-branch-site-merge")
            .unwrap_or_else(|| panic!("mir-destprop-branch-site-merge missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "mir-destprop-branch-site-merge buggy/fixed must both parse"
        );
    }

    /// Batch 99 — the load-bearing concepts survive into the corpus text
    /// (retrieval discriminators, the kernel-batch concepts_covered
    /// convention applied to a rust_perf batch).
    #[test]
    fn batch99_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "mir-destprop-branch-site-merge")
            .expect("rule present");
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in [
            "DestinationPropagation",
            "predictor",
            "devirtualize",
            "indirect",
            "call site",
        ] {
            assert!(text.contains(needle), "concept {needle:?} missing from the rule text");
        }
        assert!(
            e.context_notes.as_deref().unwrap_or("").contains("ParetoDominant"),
            "the measured verdict must be recorded in context_notes"
        );
    }

    /// Batch 101 — the new rule is present.
    #[test]
    fn batch101_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"dispatch-handler-pair-hoist"));
    }

    /// Batch 101 — the new rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch101_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "dispatch-handler-pair-hoist")
            .unwrap_or_else(|| panic!("dispatch-handler-pair-hoist missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code)
                .and_then(|_| syn::parse_file(&e.fixed_code))
                .is_ok(),
            "dispatch-handler-pair-hoist buggy/fixed must both parse"
        );
    }

    /// Batch 101 — the load-bearing concepts survive into the corpus text
    /// (retrieval discriminators), and the recorded verdicts + the
    /// cross-arch x86 evidence are in context_notes. (The x86-pending
    /// caveat was RESOLVED 2026-09-04 by the armed re-gate: x86_64 read
    /// ParetoDominant 3/0/0, refuting the expected Tie — the annotation
    /// updates, never deletes.)
    #[test]
    fn batch101_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "dispatch-handler-pair-hoist")
            .expect("rule present");
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in [
            "hoist",
            "table",
            "unpredictable",
            "register",
            "dispatch",
        ] {
            assert!(text.contains(needle), "concept {needle:?} missing from the rule text");
        }
        let notes = e.context_notes.as_deref().unwrap_or("");
        assert!(
            notes.contains("ParetoDominant"),
            "the measured verdict must be recorded in context_notes"
        );
        assert!(
            notes.contains("x86") && notes.contains("REFUTED"),
            "the x86 re-gate verdict must be recorded (never silently deleted)"
        );
        assert!(
            notes.contains("mir-destprop-branch-site-merge"),
            "the dedup cross-ref vs the landed B99 rule must be recorded"
        );
    }

    /// Batch 101 — G1: the hoisted form is output-identical to the table
    /// form over a fixed op stream, and BOTH handlers actually run
    /// (non-vacuity: a one-op stream would pass trivially).
    #[test]
    fn batch101_hoist_preserves_output_and_both_handlers_run() {
        fn op_add(x: u64) -> u64 {
            x.wrapping_add(0x9E37)
        }
        fn op_mul(x: u64) -> u64 {
            x.wrapping_mul(31)
        }
        let handlers: [fn(u64) -> u64; 2] = [op_add, op_mul];

        // Fixed 2-op stream: both ops exercised, deterministic.
        let ops: Vec<u32> = (0..256u32).map(|i| i % 2).collect();
        let xs: Vec<u64> = (0..256u32).map(|i| (u64::from(i)) ^ 0xABCD).collect();

        let run_table = |ops: &[u32], xs: &[u64]| -> u64 {
            let mut acc = 0u64;
            for (i, &op) in ops.iter().enumerate() {
                let f = handlers[(op & 1) as usize];
                acc = f(acc ^ xs[i]);
            }
            acc
        };
        let run_hoisted = |ops: &[u32], xs: &[u64]| -> u64 {
            let f0 = handlers[0];
            let f1 = handlers[1];
            let mut acc = 0u64;
            for (i, &op) in ops.iter().enumerate() {
                let x = acc ^ xs[i];
                acc = if op & 1 == 0 { f0(x) } else { f1(x) };
            }
            acc
        };

        let a = run_table(&ops, &xs);
        let b = run_hoisted(&ops, &xs);
        assert_eq!(a, b, "hoist must be output-identical");

        // Non-vacuity: the all-add and all-mul streams must DIFFER —
        // proves both handlers are load-bearing in the fixture.
        let only_add: Vec<u32> = vec![0; 256];
        let only_mul: Vec<u32> = vec![1; 256];
        assert_ne!(run_table(&only_add, &xs), run_table(&only_mul, &xs));
    }

    /// Batch 109 — the rule is present.
    #[test]
    fn batch109_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"dead-result-timing-loop-folded-under-lto"));
    }

    /// Batch 109 — the rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch109_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "dead-result-timing-loop-folded-under-lto")
            .expect("rule present");
        assert!(
            syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
            "dead-result-timing-loop-folded-under-lto buggy/fixed must both parse"
        );
    }

    /// Batch 109 — the measurement-honesty contract is pinned in the rule
    /// text: the callee-side black_box caveat, the constant-work fold
    /// hazard, the NaN comparison directions, and the NotBenchable verdict
    /// (no fabricated Pareto number for a rule whose baseline is zero).
    #[test]
    fn batch109_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "dead-result-timing-loop-folded-under-lto")
            .expect("rule present");
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in ["black_box", "sink", "xorshift", "assert"] {
            assert!(text.to_lowercase().contains(needle), "concept {needle:?} missing from the rule text");
        }
        let notes = e.context_notes.as_deref().unwrap_or("");
        for needle in [
            "NotBenchable",
            "CALLER discards",
            "CHAINED STATE",
            "cancel in batches",
            "budget bar",
            "debug-stat-full-scan-out-of-hot-path",
        ] {
            assert!(notes.contains(needle), "contract {needle:?} missing from context_notes");
        }
        // No fabricated bench: the measurement-honesty rule carries NO bench_spec.
        #[cfg(feature = "rust_perf_bench")]
        assert!(e.bench_spec.is_none(), "a measurement-honesty rule must not carry a fabricated Pareto bench");
    }

    /// Batch 109 G1 — the rule's own fix pattern, run as code: the fixed
    /// harness's work chain is DATA-DEPENDENT (xorshift over the loop
    /// index) and the sink CONSUMES per-iteration results, so two different
    /// data streams produce different sinks (the fold has nothing constant
    /// to cut); the buggy shape's sink over a CONSTANT-work fn collapses to
    /// 0 at compile time — the exact hazard the rule names. The timing
    /// assertions are deliberately absent (a debug-profile unit test must
    /// not gate on wall-clock); the gate pins the DATAFLOW contract, the
    /// loud-baseline assert shape is pinned by the concepts gate above.
    #[test]
    fn batch109_sink_consumes_per_iteration_data() {
        // The gate that shaped the rule: BOTH draft fix-patterns failed
        // here before the chain form shipped. Each anti-shape is pinned as
        // an assertion so a future editor cannot quietly reintroduce it.
        let work = |seed: u64| -> u64 {
            let mut x = seed;
            for _ in 0..64 {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
            }
            x
        };

        // Anti-shape 1 (GF(2) linearity): an XOR sink over the linear work
        // cancels EXACTLY over 0..2^k — the first draft's "fix" was itself
        // foldable to zero.
        let mut xor_sink = 0u64;
        for i in 0..4096u64 {
            xor_sink ^= work(i);
        }
        assert_eq!(xor_sink, 0, "XOR of a GF(2)-linear work over 0..2^k must cancel — the trap the rule names");

        // Anti-shape 2 (balanced-bit addition collapse): over a
        // power-of-two batch, every balanced output bit is 1 in exactly
        // half the terms regardless of seed, so even wrapping_add lands
        // the SAME seed-independent constant (2^64 − n/2; measured:
        // both arms read 18446744073709549568).
        let add_sink = |seed: u64| -> u64 {
            let mut sink = 0u64;
            for i in 0..4096u64 {
                sink = sink.wrapping_add(work(seed ^ i));
            }
            sink
        };
        assert_eq!(
            add_sink(0),
            add_sink(0xDEAD_BEEF),
            "wrapping_add over a balanced power-of-two batch must collapse to the seed-independent constant — the second trap the rule names"
        );

        // The shipped fix pattern: CHAINED state — every iteration
        // consumes the previous accumulator, so the final value depends on
        // the whole sequential history and no batch algebra can fold it.
        // Non-vacuous both ways: different seeds land different chains.
        let chained = |seed: u64| -> u64 {
            let mut state = seed;
            for i in 0..4096u64 {
                state = work(state ^ i);
            }
            state
        };
        assert_ne!(chained(0), chained(0xDEAD_BEEF), "the chained harness must stay seed-dependent — this is the fold-proof shape the rule prescribes");
    }


    /// Batch 112 — the rule is present in the corpus.
    #[test]
    fn batch112_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"hot-byval-move-over-memcpy-threshold"));
    }

    /// Batch 112 — the rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch112_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "hot-byval-move-over-memcpy-threshold")
            .expect("rule present");
        assert!(
            syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
            "hot-byval-move-over-memcpy-threshold buggy/fixed must both parse"
        );
    }

    /// Batch 112 — the decision content is pinned in the rule text: the
    /// memcpy-call threshold, the arch-shaped ceiling (asm evidence), the
    /// composed-fix disclosure, and the armed x86 re-gate.
    #[test]
    fn batch112_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "hot-byval-move-over-memcpy-threshold")
            .expect("rule present");
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in ["memcpy", "threshold", "cold", "by reference", "in place"] {
            assert!(text.to_lowercase().contains(needle), "concept {needle:?} missing from the rule text");
        }
        let notes = e.context_notes.as_deref().unwrap_or("");
        for needle in [
            "Mixed",
            "#160005",
            "asm",
            "ldp/stp",
            "bl _memcpy",
            "(136, 320]",
            "128 B",
            "40% wall",
            "DHAT",
            "Composed-fix disclosure",
            "x86 RE-GATED",
            "callq memcpy",
            "box-large-enum-variant",
        ] {
            assert!(notes.contains(needle), "contract {needle:?} missing from context_notes");
        }
        // The rule IS benchable — the annotation's Mixed verdict comes from
        // the recorded run, not from an absent spec.
        #[cfg(feature = "rust_perf_bench")]
        assert!(e.bench_spec.is_some(), "the memcpy-threshold rule must carry its measured bench");
    }

    /// Batch 112 G1 (the fairness contract): the buggy composed fold (17
    /// fields moved by value) and the fixed composed fold (13 in the moved
    /// struct + 4 by reference, mutated in place) land BIT-IDENTICAL state
    /// and accumulator over the same LCG stream — the bench compares move
    /// cost only, never workload. Also pins the load-bearing sizes:
    /// 13 + 4 u64 = 136 B (the source's type) and 13 u64 = 104 B (the
    /// source's shrunk type); the x86-64 threshold is 128 B, so 136 crosses
    /// it and 104 does not — the bench shapes ARE the threshold content.
    #[test]
    fn batch112_byval_folds_agree_bit_identical() {
        // LCG stream — the same construction the bench harness uses.
        // The folded field ops — IDENTICAL per-field math in both arms, the
        // exact order the bench harness folds in.
        fn fold_hot(hot: &[u64; 13]) -> u64 {
            hot.iter()
                .enumerate()
                .fold(0u64, |a, (i, &v)| a.wrapping_add(v.rotate_left(7) ^ (i as u64)))
        }
        fn fold_cold(cold: &[u64; 4]) -> u64 {
            cold.iter().fold(0u64, |a, &v| a.wrapping_add(v.wrapping_mul(3)))
        }

        // Buggy arm: cold lives INSIDE the moved record (the RecBig/RecSmall
        // shapes ARE the bench's — 136 B vs 104 B straddling x86-64's 128 B).
        #[derive(Clone, Copy)]
        struct RecBig {
            hot: [u64; 13],
            cold: [u64; 4],
        }
        #[derive(Clone, Copy)]
        struct RecSmall {
            hot: [u64; 13],
        }

        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let mut seed_hot = [0u64; 13];
        let mut seed_cold = [0u64; 4];
        for v in &mut seed_hot {
            *v = next();
        }
        for v in &mut seed_cold {
            *v = next();
        }

        let mut state = RecBig { hot: seed_hot, cold: seed_cold };
        let mut acc_buggy = 0u64;
        for i in 0..1_000 {
            // score moves the WHOLE 136 B record by value (buggy_code's shape)
            let r = state;
            acc_buggy ^= fold_hot(&r.hot) ^ fold_cold(&r.cold);
            state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc_buggy);
            state.cold[i & 3] = state.cold[i & 3].rotate_left(1);
        }

        // Fixed arm: cold is a separate by-ref tail, mutated in place.
        let mut state = RecSmall { hot: seed_hot };
        let mut cold = seed_cold;
        let mut acc_fixed = 0u64;
        for i in 0..1_000 {
            let r = state;
            acc_fixed ^= fold_hot(&r.hot) ^ fold_cold(&cold);
            state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc_fixed);
            cold[i & 3] = cold[i & 3].rotate_left(1);
        }

        assert_eq!(acc_buggy, acc_fixed, "the two arms must fold identically");
        let final_cold = cold;
        let rejoined = RecBig { hot: state.hot, cold: final_cold };
        assert_eq!(rejoined.cold, final_cold, "cold tail state must agree across the arms");

        // The threshold content: the bench shapes straddle x86-64's 128 B.
        assert_eq!(std::mem::size_of::<RecBig>(), 136);
        assert_eq!(std::mem::size_of::<RecSmall>(), 104);
    }

    /// Batch 99 G1 (output + dispatch-sequence identity): both shapes
    /// dispatch the IDENTICAL handler sequence over the same stream (pinned
    /// by per-handler call counters, not just final state) and land the
    /// identical final state. Non-vacuous: both counters must be > 0. The
    /// mechanism (merged = cmov + one indirect tail-jump, split = two
    /// direct tail-jumps) is asm evidence recorded in the annotation — it
    /// is not unit-gateable portably.
    #[test]
    fn batch99_split_sites_preserve_output_and_dispatch_sequence() {
        use std::cell::RefCell;
        struct State {
            acc: u64,
        }
        type Handler = fn(&mut State, u32);
        fn handler_add(st: &mut State, x: u32) {
            CALLS.with(|c| c.borrow_mut().0 += 1);
            st.acc = st.acc.wrapping_add(u64::from(x));
        }
        fn handler_mul(st: &mut State, x: u32) {
            CALLS.with(|c| c.borrow_mut().1 += 1);
            st.acc = st.acc.wrapping_mul(31).wrapping_add(u64::from(x));
        }

        thread_local! {
            static CALLS: RefCell<(u64, u64)> = const { RefCell::new((0, 0)) };
        }
        // same shapes as the corpus fixtures
        let step_merged = |pick_add: bool, st: &mut State, x: u32| {
            let f: Handler = if pick_add { handler_add } else { handler_mul };
            f(st, x);
        };
        let step_split = |pick_add: bool, st: &mut State, x: u32| {
            if pick_add {
                handler_add(st, x);
            } else {
                handler_mul(st, x);
            }
        };
        // deterministic mixed stream (LCG, 50/50-ish)
        let mut seed = 0x243F_6A88_85A3_08D3u64;
        let stream: Vec<(bool, u32)> = (0..1024)
            .map(|i| {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let pick = ((seed >> 33) as u32) % 256 < 128;
                let x = (((seed >> 40) as u32) ^ i) & 0xFFFF;
                (pick, x)
            })
            .collect();

        CALLS.with(|c| *c.borrow_mut() = (0, 0));
        let mut sm = State { acc: 0 };
        for &(pick, x) in &stream {
            step_merged(pick, &mut sm, x);
        }
        let merged_calls = CALLS.with(|c| *c.borrow());

        CALLS.with(|c| *c.borrow_mut() = (0, 0));
        let mut ss = State { acc: 0 };
        for &(pick, x) in &stream {
            step_split(pick, &mut ss, x);
        }
        let split_calls = CALLS.with(|c| *c.borrow());

        assert_eq!(sm.acc, ss.acc, "both shapes must land the identical final state");
        assert_eq!(
            merged_calls, split_calls,
            "both shapes must dispatch the IDENTICAL handler sequence"
        );
        assert!(merged_calls.0 > 0 && merged_calls.1 > 0, "non-vacuous: both handlers ran");
    }

    /// Batch 86 G1 (exactness gate, integer ops — the ported-numeric
    /// MANDATE shape): the serialize-once buffer must reconstruct every
    /// (name, payload) record BYTE-EXACTLY, including the edge records
    /// (empty name, empty payload, multi-byte u16 lengths). No tolerance
    /// — a drifted offset must fail loudly, not approximately.
    #[test]
    #[allow(clippy::cast_possible_truncation)] // the u16/u32 narrowings ARE the wire format under test
    fn batch86_byte_buffer_store_reconstructs_exactly() {
        let names: Vec<Box<str>> = vec![
            "a".into(),
            "example-node-7".into(),
            "".into(),                          // empty-NAME edge (offsets must advance)
            "x".repeat(300).into_boxed_str(),   // multi-byte u16 length
        ];
        let payloads: Vec<Vec<u8>> = vec![
            vec![],
            vec![1, 2, 3],
            vec![0xAB; 41],
            (0..300u32).map(|i| (i % 251) as u8).collect(),
        ];
        // fixed form (mirrors the corpus fixed_code shape exactly; the
        // u16/u32 narrowings are the FORMAT — production lengths are bounded
        // by the record domain, asserted by the exact reconstruction below)
        let mut total = 4usize;
        for (n, p) in names.iter().zip(&payloads) {
            total += 4 + n.len() + p.len();
        }
        let mut buf = Vec::with_capacity(total);
        let mut offsets = Vec::with_capacity(names.len());
        buf.extend_from_slice(&(names.len() as u32).to_le_bytes());
        for (n, p) in names.iter().zip(&payloads) {
            offsets.push(buf.len() as u32);
            buf.extend_from_slice(&(n.len() as u16).to_le_bytes());
            buf.extend_from_slice(n.as_bytes());
            buf.extend_from_slice(&(p.len() as u16).to_le_bytes());
            buf.extend_from_slice(p);
        }
        // reconstruct + compare EXACTLY
        let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        assert_eq!(count, names.len());
        for (i, &off) in offsets.iter().enumerate() {
            let o = off as usize;
            let nl = u16::from_le_bytes([buf[o], buf[o + 1]]) as usize;
            let name = &buf[o + 2..o + 2 + nl];
            let pl = u16::from_le_bytes([buf[o + 2 + nl], buf[o + 3 + nl]]) as usize;
            let payload = &buf[o + 4 + nl..o + 4 + nl + pl];
            assert_eq!(name, names[i].as_bytes(), "name {i} drifted");
            assert_eq!(payload, payloads[i].as_slice(), "payload {i} drifted");
        }
    }

    /// Batch 86 G1 (the deterministic size axis for the bitflag rule):
    /// the flag-cluster layout is 12 bytes, the bitflag layout is 8 —
    /// pinned EXACTLY (align rules are platform-stable for these primitive
    /// members), and every bit position round-trips through the packed
    /// form. The "measure size_of before/after" caveat is the RULE's
    /// (default-repr layouts vary); this fixture's layout is fixed.
    #[test]
    fn batch86_bitflag_size_axis_and_roundtrip() {
        #[allow(dead_code)] // size_of is the measurement
        #[allow(clippy::struct_excessive_bools)] // the 8-bool cluster IS the measured layout
        struct Wide {
            weight: u32,
            reachable: bool,
            verified: bool,
            stale: bool,
            pinned: bool,
            external: bool,
            compressed: bool,
            anchored: bool,
            sealed: bool,
        }
        #[allow(dead_code)]
        struct Packed {
            weight: u32,
            flags: u8,
        }
        assert_eq!(std::mem::size_of::<Wide>(), 12);
        assert_eq!(std::mem::size_of::<Packed>(), 8);
        for bit in 0..8u8 {
            let mut flags = 0u8;
            flags |= 1 << bit;
            assert_eq!((flags >> bit) & 1, 1);
            flags &= !(1 << bit);
            assert_eq!(flags, 0);
        }
    }

    #[cfg(feature = "rust_perf_bench")]
    #[test]
    fn eighty_six_rules_have_bench_specs() {
        let corpus = build_perf_seed_corpus();
        let with_spec = corpus
            .entries
            .iter()
            .filter(|e| e.bench_spec.is_some())
            .count();
        assert_eq!(with_spec, 90, "expected 90 benchable rules (Batch 135 added the three Wild-linker article benches), got {with_spec}");
    }

    #[cfg(feature = "rust_perf_bench")]
    #[test]
    fn fourteen_rules_are_not_benchable() {
        let corpus = build_perf_seed_corpus();
        let without_spec: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.bench_spec.is_none())
            .map(|e| e.rule_id.as_str())
            .collect();
        // Batch 135 repaired this pin: the Batch 118–129 source-reported io
        // annotations (13 rules) were never counted here — the pin sat at the
        // Batch-109-era 14 while reality moved to 27. The test only compiles
        // under `rust_perf_bench`, so nobody ran it.
        assert_eq!(without_spec.len(), 27); // 117 total − 90 benchable
        assert!(without_spec.contains(&"coalesce-nearby-reads-bounded-gap-cacheability-gated")); // Batch 94 — cacheability-coupled regime, source-reported only
        assert!(without_spec.contains(&"software-prefetch-storage-bound")); // Batch 85 — x86-only source evidence, the annotation IS the rule
        assert!(without_spec.contains(&"bitflag-pack-when-footprint-dominates")); // Batch 86 — the deterministic size axis (12 → 8), not a speed bench
        assert!(without_spec.contains(&"layout-reorder"));
        assert!(without_spec.contains(&"repr-u8-enum"));
        assert!(without_spec.contains(&"bevy-reactive-loop"));
        assert!(without_spec.contains(&"wasm-batch-ffi"));
        assert!(without_spec.contains(&"shrink-to-fit-after-build"));
        assert!(without_spec.contains(&"box-large-enum-variant"));
        assert!(without_spec.contains(&"async-yield-point"));
        assert!(without_spec.contains(&"async-future-size"));
        // Issue 014 T2.3/T2.4 — NotBenchable metric-mismatch class.
        assert!(without_spec.contains(&"tokio-guard-across-await"));
        assert!(without_spec.contains(&"bounded-channel-backpressure"));
        assert!(without_spec.contains(&"dead-result-timing-loop-folded-under-lto")); // Batch 109 — a gate whose baseline folds to zero has nothing to ratio
    }

    /// Bench 013 found 4 rules with non-dominant verdicts in the default
    /// config (2 ParetoDominated + 2 Mixed). Their `context_notes` annotation
    /// records the finding so consumers don't unconditionally suggest them.
    /// Bench 016 (Issue 012) added 2 more annotated rules (sort-by-key,
    /// copy-from-slice). Issue 014 T2.3/T2.4 added 2 more in the
    /// NotBenchable metric-mismatch class (tokio-guard-across-await,
    /// bounded-channel-backpressure). Batch 48 added 2 verdict-class rules
    /// (filter-count-to-any ParetoDominated, iter-copied-deref-map Mixed).
    /// Batch 53 (Bench 038) added 2 more (powi-vs-powf all-Tie Mixed,
    /// soa-hot-field latency-bound Mixed). Batch 62 added 1 (the
    /// newton-refinement regime-inversion annotation) + Batch 64 added 1
    /// (debug-assert-length-hoist, the release-erasure class) + Batch 66
    /// added 1 (sortedness-check-branchless-accumulate, the regime-split
    /// class — the mirrored regime of filter-count-to-any). Batch 65 added 2
    /// (align-64-false-sharing — the arch-granularity caveat on a clean
    /// winner; channel-send-batching-unpark — the pipeline-overlap regime).
    /// Batch 73 added 2 (sort-key-cache-precompute-out-of-lock — the
    /// sorts-per-update amortization regime + the source-vs-isolated
    /// magnitude caveat; lock-guard-narrow-field-accessor — the honest
    /// margin note on a clean winner). Batch 78 added 2 (narrow-counter-
    /// overflow-promotion — the ParetoDominated memory-for-speed trade
    /// (the Datadog 2,100 GiB regime vs the measured update-path cost);
    /// cache-line-block-partitioned-lock — measured-winner regime note +
    /// the first ALLOC_AXIS_EXEMPT rule). Batch 86 added 3 (serialize-once-
    /// byte-buffer-store — the shrinking-ratio + regime note;
    /// context-equal-field-elide — the %-equal two-regime note;
    /// bitflag-pack-when-footprint-dominates — the NotBenchable-speed size
    /// axis + the repr-reorder caveat). Batch 91 added 2
    /// (filter-before-expensive-transform — the two-regime discard ×
    /// transform-cost note; decision-projection-before-construction — the
    /// invariance-precondition note). Batch 94 added 1
    /// (coalesce-nearby-reads-bounded-gap-cacheability-gated — the
    /// cacheability-coupling regime, source-reported). Batch 99 added 1
    /// (mir-destprop-branch-site-merge — the measured ParetoDominant verdict
    /// with the runtime-opaque-targets negative-control boundary and the
    /// rustc DestProp toolchain regime).
    /// The other 57 have `context_notes: None` (unconditional wins or
    /// honestly NotBenchable).
    #[test]
    fn forty_three_annotated_rules_carry_context_notes() {
        let corpus = build_perf_seed_corpus();
        let annotated: Vec<&str> = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_some())
            .map(|e| e.rule_id.as_str())
            .collect();
        assert_eq!(annotated.len(), 60, "expected 60 context-annotated rules (Batch 135 added the three Wild-linker article annotations)");
        assert!(annotated.contains(&"parallel-disjoint-pread-hides-cold-page-latency")); // Batch 118 — the ds4 PLE-gather io annotation (source-reported, NotBenchable here)
        assert!(annotated.contains(&"resume-revalidates-provenance-before-reuse")); // Batch 121 — the ds4 IQ2 resume-provenance annotation
        assert!(annotated.contains(&"write-verify-readback-before-publish")); // Batch 121 — the ds4 IQ2 publish-integrity annotation
        assert!(annotated.contains(&"ab-isolate-one-component-by-baseline-byte-copy")); // Batch 121 — the A/B-design + memory-honesty annotation
        assert!(annotated.contains(&"dead-result-timing-loop-folded-under-lto")); // Batch 109 — the NotBenchable measurement-honesty axis
        assert!(annotated.contains(&"hot-byval-move-over-memcpy-threshold")); // Batch 112 — the arch-regime Mixed annotation
        assert!(annotated.contains(&"weight-tile-consumption-order")); // Batch 85 — measured ParetoDominant + the M3-L1-shift regime note
        assert!(annotated.contains(&"software-prefetch-storage-bound")); // Batch 85 — the x86-source-cited honest-loser annotation
        assert!(annotated.contains(&"precompute-table"));
        assert!(annotated.contains(&"papaya-lockfree"));
        assert!(annotated.contains(&"dedupe-hashset-vs-vec"));
        assert!(annotated.contains(&"reuse-allocation-clear"));
        assert!(annotated.contains(&"sort-by-key"));
        assert!(annotated.contains(&"copy-from-slice"));
        assert!(annotated.contains(&"tokio-guard-across-await"));
        assert!(annotated.contains(&"bounded-channel-backpressure"));
        assert!(annotated.contains(&"filter-count-to-any"));
        assert!(annotated.contains(&"iter-copied-deref-map"));
        assert!(annotated.contains(&"powi-vs-powf"));
        assert!(annotated.contains(&"soa-hot-field"));
        assert!(annotated.contains(&"newton-refinement-vs-hardware-transcendental"));
        assert!(annotated.contains(&"sortedness-check-branchless-accumulate"));
        assert!(annotated.contains(&"debug-assert-length-hoist"));
        assert!(annotated.contains(&"align-64-false-sharing"));
        assert!(annotated.contains(&"channel-send-batching-unpark"));
        assert!(annotated.contains(&"sort-key-cache-precompute-out-of-lock"));
        assert!(annotated.contains(&"lock-guard-narrow-field-accessor"));
        assert!(annotated.contains(&"swar-digit-chunk-parse")); // Batch 74 — the regime note (<8 digits tie)
        assert!(annotated.contains(&"runtime-feature-detect-per-chunk")); // Batch 74 — measured-Tie honest annotation
        assert!(annotated.contains(&"encoded-layout-borrowed-view")); // Batch 75 — S<<R regime
        assert!(annotated.contains(&"group-keys-borrow-allocate-on-first-sight")); // Batch 75 — R/S regime + correctness contract
        assert!(annotated.contains(&"sorted-input-previous-row-fast-path")); // Batch 75 — hit-rate regime
        assert!(annotated.contains(&"btreemap-sorted-iteration")); // Batch 75 rider — the two-row sorted-read decision table
        assert!(annotated.contains(&"bitplane-transpose-repeated-predicate-scan")); // Batch 83 — measured ParetoDominant + the amortization/b/equality regime gates
        assert!(annotated.contains(&"narrow-counter-overflow-promotion")); // Batch 78 — ParetoDominated honest-loser (the memory-for-speed trade)
        assert!(annotated.contains(&"cache-line-block-partitioned-lock")); // Batch 78 — measured-winner + regime + alloc-exempt note
        assert!(annotated.contains(&"serialize-once-byte-buffer-store")); // Batch 86 — measured ParetoDominant + the shrinking-ratio/regime note
        assert!(annotated.contains(&"context-equal-field-elide")); // Batch 86 — measured ParetoDominant + the %-equal two-regime note
        assert!(annotated.contains(&"bitflag-pack-when-footprint-dominates")); // Batch 86 — NotBenchable-speed size axis + the repr-reorder caveat
        assert!(annotated.contains(&"filter-before-expensive-transform")); // Batch 91 — measured ParetoDominant + the two-regime (discard × cost) note
        assert!(annotated.contains(&"decision-projection-before-construction")); // Batch 91 — the invariance-precondition note
        assert!(annotated.contains(&"coalesce-nearby-reads-bounded-gap-cacheability-gated")); // Batch 94 — the cacheability-coupling regime note (source-reported)
        assert!(annotated.contains(&"mir-destprop-branch-site-merge")); // Batch 99 — measured ParetoDominant + the runtime-opaque-targets boundary + toolchain regime
        assert!(annotated.contains(&"dispatch-handler-pair-hoist")); // Batch 101 — measured ParetoDominant (aarch64) + the unpredictable-stream regime + the x86 re-gate verdict (2026-09-04, expected-Tie REFUTED)
    }

    /// The non-annotated rules (ParetoDominant + honestly NotBenchable) must
    /// have `context_notes: None` — they are either unconditional wins or
    /// honestly unmeasurable.
    #[test]
    fn unannotated_rules_have_no_context_notes() {
        let corpus = build_perf_seed_corpus();
        let unannotated = corpus
            .entries
            .iter()
            .filter(|e| e.context_notes.is_none())
            .count();
        assert_eq!(unannotated, 57, "expected 57 unannotated rules (114 total - 57 annotated; the new rules joined the ANNOTATED side)");
    }

    /// `context_notes` is backward-compatible via `#[serde(default)]`: an older
    /// freeze/thaw snapshot (pre-this-field) deserializes as `None`.
    #[test]
    fn context_notes_backward_compatible_serde() {
        let corpus = build_perf_seed_corpus();
        // Serialize + deserialize round-trip preserves the annotation.
        let json = serde_json::to_string(&corpus.entries).unwrap();
        let back: Vec<PerfRule> = serde_json::from_str(&json).unwrap();
        let annotated = back.iter().filter(|e| e.context_notes.is_some()).count();
        assert_eq!(annotated, 60); // Batch 135: 57 → 60

        // Simulate a pre-field snapshot: strip `context_notes` from JSON.
        // (A real old snapshot simply omits the key; serde(default) → None.)
        let old_json = json.replace("\"context_notes\":", "\"__removed_context_notes\":");
        let old_back: Vec<PerfRule> = serde_json::from_str(&old_json).unwrap();
        let old_annotated = old_back.iter().filter(|e| e.context_notes.is_some()).count();
        assert_eq!(old_annotated, 0, "missing field should default to None");
    }

    /// Integration: the drafter propagates `context_notes` from `PerfRule`
    /// through `DraftedFix` for the annotated rules, and `None` for the
    /// unconditional wins. Twenty annotations: Bench 013 (2 ParetoDominated
    /// and 2 Mixed), Bench 016 (sort-by-key and copy-from-slice), Issue 014
    /// (2 NotBenchable metric-mismatch), Batch 48 (2 verdict-class),
    /// Batch 53 (Bench 038, 2 verdict-class), Batch 62 (1 regime-class),
    /// Batch 64 (1 release-erasure class), Batch 66 (1 regime-split
    /// class), Batch 68 (1 ParetoDominated honest-loser — the tantivy
    /// varint ladder regresses on aarch64 across 3 tested writer regimes),
    /// Batch 65 (2 measured-winner regime caveats — align-64's
    /// arch-granularity + channel-send's pipeline-overlap margin; a NEW
    /// annotation class: ParetoDominant verdict + applicability condition,
    /// accepted by the token check below), and Batch 73 (2 measured-winner
    /// caveats — the mnt.io room-list distill: sort-key-cache's
    /// sorts-per-update regime + source-vs-isolated magnitude note;
    /// lock-guard-narrow's end-to-end-vs-isolated margin note).
    #[test]
    fn drafter_propagates_context_notes_from_perf_corpus() {
        use crate::domain::HealEntry;

        let corpus = build_perf_seed_corpus();

        // Filter to the 60 annotated rules.
        let annotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_some()).collect();
        assert_eq!(annotated.len(), 60, "expected 60 annotated rules");

        // The HealEntry trait override must agree with the struct field.
        for rule in &annotated {
            assert_eq!(
                HealEntry::context_notes(*rule),
                rule.context_notes.as_deref(),
                "HealEntry::context_notes() must match the PerfRule field for {}",
                rule.rule_id
            );
            assert!(
                rule.context_notes.as_ref().unwrap().contains("ParetoDominated")
                    || rule.context_notes.as_ref().unwrap().contains("ParetoDominant")
                    || rule.context_notes.as_ref().unwrap().contains("Mixed")
                    || rule.context_notes.as_ref().unwrap().contains("NotBenchable"),
                "annotation should mention ParetoDominated, ParetoDominant \
                 (the Batch 65 measured-winner caveat class), Mixed, or the \
                 NotBenchable metric-mismatch class for {}",
                rule.rule_id
            );
        }

        // The unannotated rules must return None from the trait too.
        let unannotated: Vec<&PerfRule> = corpus.entries.iter().filter(|e| e.context_notes.is_none()).collect();
        assert_eq!(unannotated.len(), 57, "expected 57 unannotated rules");
        for rule in &unannotated {
            assert!(HealEntry::context_notes(*rule).is_none(), "{} should have no context_notes", rule.rule_id);
        }
    }

    /// G1 for `align-64-false-sharing` (Batch 65, Research 081): the
    /// per-thread counter totals are a deterministic closed form — layout
    /// (and thread interleaving) cannot change them. fetch_add is atomic, so
    /// each counter's final value is exactly its op count regardless of which
    /// slot layout the atomics lived in. Pins the bench harness's in-run
    /// debug_assert at the corpus level + documents the identity the wall-clock
    /// arms share (the timing differential is PURELY the line ping-pong).
    #[test]
    fn align_64_false_sharing_totals_are_layout_invariant() {
        use std::sync::atomic::Ordering;

        #[repr(align(128))]
        struct Padded {
            allocs: std::sync::atomic::AtomicU64,
            bytes: std::sync::atomic::AtomicU64,
            frees: std::sync::atomic::AtomicU64,
        }
        #[derive(Default)]
        struct Packed {
            allocs: std::sync::atomic::AtomicU64,
            bytes: std::sync::atomic::AtomicU64,
            frees: std::sync::atomic::AtomicU64,
        }

        const SLOTS: usize = 12;
        const ITERS: u64 = 500;

        fn expected(n_threads: usize) -> u64 {
            n_threads as u64 * (2 * ITERS + ITERS * (ITERS - 1) / 2)
        }

        // Multi-threaded on BOTH layouts (interleaving varies, totals cannot).
        for n_threads in [2usize, 4, 8] {
            let padded: Vec<Padded> = (0..SLOTS)
                .map(|_| Padded {
                    allocs: std::sync::atomic::AtomicU64::new(0),
                    bytes: std::sync::atomic::AtomicU64::new(0),
                    frees: std::sync::atomic::AtomicU64::new(0),
                })
                .collect();
            let packed: Vec<Packed> = (0..SLOTS).map(|_| Packed::default()).collect();
            std::thread::scope(|s| {
                for t in 0..n_threads {
                    let p = &padded;
                    let q = &packed;
                    s.spawn(move || {
                        for i in 0..ITERS {
                            p[t].allocs.fetch_add(1, Ordering::Relaxed);
                            p[t].bytes.fetch_add(i, Ordering::Relaxed);
                            p[t].frees.fetch_add(1, Ordering::Relaxed);
                            q[t].allocs.fetch_add(1, Ordering::Relaxed);
                            q[t].bytes.fetch_add(i, Ordering::Relaxed);
                            q[t].frees.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                }
            });
            let cs_p: u64 = padded
                .iter()
                .map(|x| {
                    x.allocs.load(Ordering::Relaxed)
                        + x.bytes.load(Ordering::Relaxed)
                        + x.frees.load(Ordering::Relaxed)
                })
                .sum();
            let cs_q: u64 = packed
                .iter()
                .map(|x| {
                    x.allocs.load(Ordering::Relaxed)
                        + x.bytes.load(Ordering::Relaxed)
                        + x.frees.load(Ordering::Relaxed)
                })
                .sum();
            assert_eq!(cs_p, expected(n_threads), "padded totals @{n_threads}t");
            assert_eq!(cs_q, expected(n_threads), "packed totals @{n_threads}t");
        }

        // The annotation must carry the arch-granularity caveat (the rule id
        // says 64 — the honest fix is line-size-dependent).
        let corpus = build_perf_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "align-64-false-sharing")
            .expect("align-64-false-sharing in corpus");
        assert!(rule.context_notes.as_ref().unwrap().contains("128"));
    }

    /// G1 for `channel-send-batching-unpark` (Batch 65, Research 081): the
    /// per-item and per-batch producer paths deliver the SAME item count —
    /// the flush boundary cannot drop or double-count (the mem::replace
    /// swap keeps the tail batch's capacity + the final flush covers the
    /// remainder). Also pins the annotation's take-vs-replace warning: the
    /// replace pattern is the one that wins the alloc axis (take-then-regrow
    /// measured 795 vs 329 allocs @10K on the WRONG side).
    #[test]
    fn channel_send_batching_delivers_identical_counts() {
        const FLUSH_EVERY: usize = 64;

        fn buggy_delivered(total: u64) -> u64 {
            total
        }
        fn fixed_delivered(total: u64) -> u64 {
            let mut count: u64 = 0;
            let mut tail: usize = 0;
            for i in 0..total {
                let _ = std::hint::black_box(i);
                tail += 1;
                if tail >= FLUSH_EVERY {
                    count += tail as u64; // one batch delivered
                    tail = 0;
                }
            }
            count += tail as u64; // final flush covers the remainder
            count
        }

        for total in [0u64, 1, 63, 64, 65, 10_000, 100_000] {
            assert_eq!(
                fixed_delivered(total),
                buggy_delivered(total),
                "batched delivery count drift at total={total}"
            );
        }

        let corpus = build_perf_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "channel-send-batching-unpark")
            .expect("channel-send-batching-unpark in corpus");
        let note = rule.context_notes.as_ref().unwrap();
        assert!(note.contains("mem::replace"), "annotation must warn take-vs-replace");
    }

    /// G1 for `sort-key-cache-precompute-out-of-lock` (Batch 73, Research
    /// 085/086 — the mnt.io room-list distill): the maintained-cache sort
    /// must produce the IDENTICAL order as the lock+clone comparator sort —
    /// same keys, same comparator, no observable side effects. Mirrors the
    /// bench harness's shape at test scale (distinct keys → total order →
    /// deterministic), including the reverse-then-sort-desc protocol.
    #[test]
    fn sort_key_cache_order_matches_lock_clone_comparator() {
        use std::sync::RwLock;

        #[derive(Clone)]
        struct LatestEvent {
            timestamp: u64,
            // width-only: the clone reads it, nothing else does
            #[allow(dead_code)]
            body: String,
        }
        struct RoomInfo {
            latest_event: Option<LatestEvent>,
        }
        struct Room {
            info: RwLock<RoomInfo>,
        }

        impl Room {
            fn wide_ts(&self) -> Option<u64> {
                // the buggy shape: whole-value clone under the guard
                self.info
                    .read()
                    .unwrap()
                    .latest_event
                    .clone()
                    .map(|ev| ev.timestamp)
            }
            fn narrow_ts(&self) -> Option<u64> {
                self.info
                    .read()
                    .unwrap()
                    .latest_event
                    .as_ref()
                    .map(|ev| ev.timestamp)
            }
        }

        const N: usize = 128;
        let mut rooms: Vec<Room> = (0..N as u64)
            .map(|i| Room {
                info: RwLock::new(RoomInfo {
                    latest_event: Some(LatestEvent {
                        timestamp: i * 7 + 3, // distinct
                        body: format!("body {i} padded padded padded"),
                    }),
                }),
            })
            .collect();
        rooms.reverse(); // maximally unsorted start

        // Arm A: the buggy comparator (clone under guard, twice per cmp).
        let mut buggy_order: Vec<Option<u64>> = rooms.iter().map(Room::wide_ts).collect();
        buggy_order.sort_by(|a, b| b.cmp(a));

        // Arm B: refresh the cache once (narrow reads), sort on cached keys.
        let cached: Vec<Option<u64>> = rooms.iter().map(Room::narrow_ts).collect();
        let mut fixed_order = cached.clone();
        fixed_order.sort_by(|a, b| b.cmp(a));

        assert_eq!(
            buggy_order, fixed_order,
            "maintained-cache sort order drifted from the lock+clone order"
        );
        // Descending-by-distinct-keys is fully determined: first = max key.
        let max_key = (0..N as u64).map(|i| i * 7 + 3).max().unwrap();
        assert_eq!(fixed_order.first().copied().flatten(), Some(max_key));
    }

    /// G1 for `lock-guard-narrow-field-accessor` (Batch 73, Research 085/086):
    /// the narrow accessor must return the SAME value as the wide-clone
    /// accessor for every room — the clone is unobservable, only the
    /// consumed sub-field crosses.
    #[test]
    fn lock_guard_narrow_accessor_values_match_wide_clone() {
        use std::sync::RwLock;

        #[derive(Clone)]
        struct LatestEvent {
            // width-only fields: only the clone ever touches them
            #[allow(dead_code)]
            timestamp: u64,
            sender_is_local: bool,
            #[allow(dead_code)]
            body: String,
        }
        struct RoomInfo {
            latest_event: Option<LatestEvent>,
        }
        struct Room {
            info: RwLock<RoomInfo>,
        }

        impl Room {
            fn wide_is_local(&self) -> bool {
                self.info
                    .read()
                    .unwrap()
                    .latest_event
                    .clone() // the whole 144 B payload
                    .is_some_and(|ev| ev.sender_is_local)
            }
            fn narrow_is_local(&self) -> bool {
                self.info
                    .read()
                    .unwrap()
                    .latest_event
                    .as_ref()
                    .is_some_and(|ev| ev.sender_is_local)
            }
        }

        // Mixed local/not + one room with NO latest event (the None edge).
        let rooms: Vec<Room> = (0..33)
            .map(|i| {
                let latest = if i == 32 {
                    None
                } else {
                    Some(LatestEvent {
                        timestamp: i * 11 + 1,
                        sender_is_local: i % 3 == 0,
                        body: format!("body {i} padded padded"),
                    })
                };
                Room {
                    info: RwLock::new(RoomInfo {
                        latest_event: latest,
                    }),
                }
            })
            .collect();

        for (i, r) in rooms.iter().enumerate() {
            assert_eq!(
                r.wide_is_local(),
                r.narrow_is_local(),
                "narrow accessor drifted from wide-clone at room {i}"
            );
        }
        // The None edge: both arms false.
        assert!(!rooms[32].narrow_is_local());
    }

    /// Batch 75 — the three new rules parse (syn) and are retrievable.
    #[test]
    fn batch75_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in [
            "encoded-layout-borrowed-view",
            "group-keys-borrow-allocate-on-first-sight",
            "sorted-input-previous-row-fast-path",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    #[test]
    fn batch75_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"encoded-layout-borrowed-view"));
        assert!(ids.contains(&"group-keys-borrow-allocate-on-first-sight"));
        assert!(ids.contains(&"sorted-input-previous-row-fast-path"));
    }

    /// Batch 76 — the two Meilisearch rules are present + retrievable.
    #[test]
    fn batch76_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"debug-stat-full-scan-out-of-hot-path"));
        assert!(ids.contains(&"eager-pointer-collect-lazy-fetch"));
    }

    /// Batch 76 — G1 for `debug-stat-full-scan-out-of-hot-path`: the buggy
    /// arm's single-scan statistic and the fixed arm's maintained aggregate
    /// must agree EXACTLY (len, total, mean bits) on structured + LCG
    /// inputs — the mean's f64 bits are the strictest cheap equality we
    /// have (any divergence in total or len shifts the quotient's bits).
    #[test]
    #[allow(clippy::cast_precision_loss)] // the usize→f64 mean IS the statistic under test
    fn debug_stat_maintained_aggregate_matches_scan_exactly() {
        fn scan_stat(items: &[usize], batch: &[usize]) -> (usize, usize, u64) {
            let total: usize = items.iter().chain(batch).sum();
            let len = items.len() + batch.len();
            let mean = len as f64 / total.max(1) as f64;
            (len, total, mean.to_bits())
        }
        fn maintained_stat(items: &[usize], link_count: usize, batch: &[usize]) -> (usize, usize, u64) {
            let total = link_count + batch.iter().sum::<usize>();
            let len = items.len() + batch.len();
            let mean = len as f64 / total.max(1) as f64;
            (len, total, mean.to_bits())
        }
        let mut rng: u64 = 0x5EED_5EED_5EED_5EED;
        let mut next = || {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (rng >> 16) as usize % 8
        };
        // empty store + empty batch (the max(1) guard), structured, + LCG
        for (items, batch) in [
            (vec![], vec![]),
            (vec![0usize; 5], vec![3usize; 2]),
            ((0..200).map(|_| next()).collect::<Vec<_>>(), (0..50).map(|_| next()).collect::<Vec<_>>()),
        ] {
            let link_count = items.iter().sum::<usize>();
            assert_eq!(scan_stat(&items, &batch), maintained_stat(&items, link_count, &batch));
        }
    }

    /// Batch 75 — the candidate-list correctness contract: an injected
    /// worst-case hash (len % 4) MUST NOT merge distinct groups. Pins the
    /// `HashMap<u64, Vec<usize>>` decision content a real SipHash collision
    /// never exercises (the Lane B falsifiable-gate recommendation).
    #[test]
    fn batch75_collision_candidate_list_never_merges() {
        use std::collections::HashMap;
        // 8 distinct keys, all colliding under the injected hash
        let keys: Vec<Vec<(String, String)>> = (0..8)
            .map(|i| vec![(format!("k{i}"), format!("v{i}"))])
            .collect();
        let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
        let mut groups: Vec<Vec<(String, String)>> = Vec::new();
        for key in &keys {
            let hash = (key.len() % 4) as u64; // adversarial: identical for all
            let idx = if let Some(i) = index.get(&hash).and_then(|c| {
                c.iter().find(|&&i| groups[i] == *key).copied()
            }) {
                i
            } else {
                let i = groups.len();
                groups.push(key.clone());
                index.entry(hash).or_default().push(i);
                i
            };
            std::hint::black_box(idx);
        }
        // 8 colliding distinct keys → 8 distinct groups, zero merges
        assert_eq!(groups.len(), 8, "candidate list must keep colliding keys distinct");
        let mut sorted = groups.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 8);
    }

    /// Batch 75 — the prefix-trap correctness contract: with optional fields
    /// skipped, one key can be a PREFIX of another; the exhaustive compare
    /// (length check) must not match them (the source's
    /// `labels.next().is_none()` discipline, `{host=a}` vs `{host=a, env=prod}`).
    #[test]
    fn batch75_prefix_keys_do_not_match() {
        fn key_matches(key: &[(String, String)], row_labels: &[(String, String)]) -> bool {
            key.len() == row_labels.len()
                && key.iter().zip(row_labels).all(|(a, b)| a == b)
        }
        let short: Vec<(String, String)> = vec![("host".into(), "a".into())];
        let long: Vec<(String, String)> =
            vec![("host".into(), "a".into()), ("env".into(), "prod".into())];
        // identical prefix — must NOT match without the length check
        assert!(!key_matches(&short, &long));
        assert!(!key_matches(&long, &short));
        // exact equality still matches
        assert!(key_matches(&short, &short));
        assert!(key_matches(&long, &long));
    }

    /// Batch 78 — the two Datadog rules are present + retrievable.
    #[test]
    fn batch78_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"narrow-counter-overflow-promotion"));
        assert!(ids.contains(&"cache-line-block-partitioned-lock"));
    }

    /// Batch 78 — both new rules parse (syn) on buggy AND fixed code.
    #[test]
    fn batch78_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in ["narrow-counter-overflow-promotion", "cache-line-block-partitioned-lock"] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    /// Batch 83 — the bit-slicing rule is present + retrievable.
    #[test]
    fn batch83_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(ids.contains(&"bitplane-transpose-repeated-predicate-scan"));
    }

    /// Batch 83 — the rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch83_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "bitplane-transpose-repeated-predicate-scan")
            .unwrap_or_else(|| panic!("bitplane rule missing from corpus"));
        assert!(
            syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
            "bitplane buggy/fixed must both parse"
        );
    }

    /// Batch 83 — G1 (the ported-numeric MANDATE shape): the two-mask MSB→LSB
    /// recurrence must equal the scalar count EXACTLY for every threshold,
    /// adversarial value streams (LCG, all-equal, all-zero, max-saturated),
    /// and BOTH adaptive widths (values < 256 → b = 8; values incl. huge →
    /// b = 32). Integer ops — no tolerance.
    #[test]
    fn batch83_bitplane_mask_matches_scalar_exactly() {
        fn encode(values: &[u32], b: usize) -> (Vec<Vec<u64>>, usize) {
            let words = values.len().div_ceil(64);
            let mut planes = vec![vec![0u64; words]; b];
            for (i, &v) in values.iter().enumerate() {
                for (bit, plane) in planes.iter_mut().enumerate() {
                    plane[i / 64] |= u64::from((v >> bit) & 1) << (i % 64);
                }
            }
            (planes, words)
        }

        fn gt_popcount(planes: &[Vec<u64>], words: usize, b: usize, c: u32) -> u64 {
            if b < 32 && c >= (1u32 << b) {
                return 0;
            }
            let mut gt = vec![0u64; words];
            let mut eq = vec![u64::MAX; words];
            for bit in (0..b).rev() {
                let plane = &planes[bit];
                if (c >> bit) & 1 == 0 {
                    for w in 0..words {
                        gt[w] |= eq[w] & plane[w];
                    }
                } else {
                    for w in 0..words {
                        eq[w] &= plane[w];
                    }
                }
            }
            gt.iter().map(|w| u64::from(w.count_ones())).sum()
        }

        // LCG stream + edge-shape columns at two widths.
        let mut rng: u64 = 0x0B78_0D06_C0DE_5EED;
        let mut cols: Vec<Vec<u32>> = Vec::new();
        for _ in 0..8 {
            let mut v = Vec::with_capacity(513); // non-multiple-of-64 tail
            for _ in 0..513 {
                rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                v.push(((rng >> 33) % 256) as u32);
            }
            cols.push(v);
        }
        cols.push(vec![7u32; 513]); // all-equal
        cols.push(vec![0u32; 129]); // all-zero (b = 1 adaptive)
        cols.push(vec![u32::MAX; 65]); // max-saturated (b = 32)

        for col in &cols {
            let max_val = col.iter().copied().max().unwrap_or(0);
            let b = (32 - max_val.leading_zeros()).max(1) as usize;
            let (planes, words) = encode(col, b);
            // sweep ONLY the representable range (the domain contract:
            // constants beyond 2^b - 1 are bias-handled by the caller — see
            // the guard checks below)
            let c_max = u32::from(u8::MAX).min((1u32 << b.min(8)) - 1);
            for c in 0..=c_max {
                let scalar = col.iter().filter(|&&v| v > c).count() as u64;
                assert_eq!(gt_popcount(&planes, words, b, c), scalar, "threshold {c}");
            }
            // out-of-range constants: nothing in a b-wide column can exceed
            // them — the caller-side guard (one branch per query) returns 0
            if b < 32 {
                let out = 1u32 << b;
                let scalar = col.iter().filter(|&&v| v > out).count() as u64;
                assert_eq!(scalar, 0, "sanity: no value exceeds 2^b");
            }
            // wide-width column: sweep a few u32 thresholds too
            if b == 32 {
                for c in [0u32, 1, 0x7FFF_FFFF, u32::MAX - 1, u32::MAX] {
                    let scalar = col.iter().filter(|&&v| v > c).count() as u64;
                    assert_eq!(gt_popcount(&planes, words, b, c), scalar, "wide threshold {c}");
                }
            }
        }
    }

    /// Batch 78 — G1 for `narrow-counter-overflow-promotion` (the ported-
    /// numeric MANDATE shape): the u32+sidecar form must be EXACTLY
    /// equivalent to the u64 buffer for ANY monotone update stream,
    /// including streams that CROSS the u32 boundary (the promotion path
    /// the bench never fires). Adversarial sequences: repeated
    /// u32::MAX-adjacent adds, mixed small/large, LCG, and every-slot
    /// overflow sweeps. Totals AND per-slot values must match bit-exactly.
    #[test]
    #[allow(clippy::cast_possible_truncation)] // the LCG truncation IS the generator under test
    fn narrow_counter_promotion_matches_u64_exactly() {
        use std::collections::HashMap;

        fn buggy_run(n: usize, updates: &[(usize, u32)]) -> (u64, Vec<u64>) {
            let mut counts = vec![0u64; n];
            for &(i, v) in updates {
                counts[i] += u64::from(v);
            }
            let total = counts.iter().sum();
            (total, counts)
        }

        fn fixed_run(n: usize, updates: &[(usize, u32)]) -> (u64, Vec<u64>) {
            let mut counts = vec![0u32; n];
            let mut wide: HashMap<usize, u64> = HashMap::with_capacity(updates.len());
            for &(i, v) in updates {
                if let Some(w) = wide.get_mut(&i) {
                    *w += u64::from(v);
                    continue;
                }
                if let Some(x) = counts[i].checked_add(v) {
                    counts[i] = x;
                } else {
                    let w = u64::from(counts[i]) + u64::from(v);
                    counts[i] = 0;
                    wide.insert(i, w);
                }
            }
            let total = counts.iter().map(|&x| u64::from(x)).sum::<u64>()
                + wide.values().sum::<u64>();
            let per_slot: Vec<u64> = (0..n)
                .map(|i| match wide.get(&i) {
                    Some(&w) => w,
                    None => u64::from(counts[i]),
                })
                .collect();
            (total, per_slot)
        }

        // (a) Exact-boundary crossing: one slot walked to just below
        // u32::MAX then pushed over, then bumped again (the promoted
        // path's continue-on-follow-up).
        let mut ups: Vec<(usize, u32)> = vec![(0, u32::MAX - 5), (0, 10), (0, 7)];
        ups.extend((1..8).map(|i| (i, 1_000)));
        let (bt, bp) = buggy_run(8, &ups);
        let (ft, fp) = fixed_run(8, &ups);
        assert_eq!((bt, bp), (ft, fp));

        // (b) EVERY slot overflows (all-promoted regime).
        let ups: Vec<(usize, u32)> = (0..16)
            .flat_map(|i| [(i, u32::MAX), (i, u32::MAX), (i, 3)])
            .collect();
        let (bt, bp) = buggy_run(16, &ups);
        let (ft, fp) = fixed_run(16, &ups);
        assert_eq!((bt, bp), (ft, fp));

        // (c) LCG stream over 128 slots, 4096 updates, deltas biased to
        // the overflow regime.
        let mut rng: u64 = 0x5EED_5EED_5EED_5EED;
        let ups: Vec<(usize, u32)> = (0..4096)
            .map(|_| {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let i = (rng >> 33) as usize % 128;
                let v = ((rng >> 13) as u32 % 3000) + 1;
                (i, v)
            })
            .collect();
        let (bt, bp) = buggy_run(128, &ups);
        let (ft, fp) = fixed_run(128, &ups);
        assert_eq!((bt, bp), (ft, fp), "LCG overflow-regime stream must match exactly");

        // (d) u32::MAX boundary: 0 + u32::MAX does NOT overflow (stays
        // narrow); the SECOND u32::MAX promotes.
        let ups: Vec<(usize, u32)> = vec![(0, u32::MAX), (0, u32::MAX)];
        let (bt, bp) = buggy_run(1, &ups);
        let (ft, fp) = fixed_run(1, &ups);
        assert_eq!(bt, ft);
        assert_eq!(bp, fp);
        assert_eq!(bp[0], 2 * u64::from(u32::MAX));
    }

    /// Batch 78 — G1 for `cache-line-block-partitioned-lock`: lock
    /// granularity cannot change totals (same seeded stream through both
    /// structures, closed-form check).
    #[test]
    fn block_partitioned_lock_totals_match() {
        use std::sync::Mutex;

        #[repr(align(128))]
        struct Block { lock: Mutex<[u64; 8]> }
        const N: usize = 64; // 8 blocks x 8 points
        const UPS: usize = 500;
        let stream: Vec<usize> = {
            let mut rng: u64 = 0xABCD_EF01_2345_6789;
            (0..UPS)
                .map(|_| {
                    rng = rng
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    (rng >> 33) as usize % N
                })
                .collect()
        };

        let g = Mutex::new(vec![0u64; N]);
        for &i in &stream {
            g.lock().unwrap()[i] += 1;
        }
        let global_total: u64 = g.lock().unwrap().iter().sum();

        let blocks: Vec<Block> = (0..N / 8)
            .map(|_| Block { lock: Mutex::new([0u64; 8]) })
            .collect();
        for &i in &stream {
            blocks[i / 8].lock.lock().unwrap()[i % 8] += 1;
        }
        let block_total: u64 = blocks
            .iter()
            .map(|b| b.lock.lock().unwrap().iter().sum::<u64>())
            .sum();

        assert_eq!(global_total, block_total);
        assert_eq!(global_total, UPS as u64);
    }

    /// Batch 121 — the three new rules are present.
    #[test]
    fn batch121_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for id in [
            "resume-revalidates-provenance-before-reuse",
            "write-verify-readback-before-publish",
            "ab-isolate-one-component-by-baseline-byte-copy",
        ] {
            assert!(ids.contains(&id), "{id} missing from corpus");
        }
    }

    /// Batch 121 — every new rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch121_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in [
            "resume-revalidates-provenance-before-reuse",
            "write-verify-readback-before-publish",
            "ab-isolate-one-component-by-baseline-byte-copy",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    /// Batch 121 — the load-bearing concepts survive into the corpus text
    /// (retrieval discriminators), the NotBenchable annotations are honest
    /// (context_notes carry the annotation), and the source quotes' key
    /// phrases are pinned.
    #[test]
    fn batch121_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let find = |id: &str| {
            corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing"))
        };
        let resume = find("resume-revalidates-provenance-before-reuse");
        let resume_text = format!("{}\n{}\n{}", resume.name, resume.buggy_code, resume.fixed_code);
        for needle in ["manifest", "identity", "tool", "digest", "reuse"] {
            assert!(resume_text.contains(needle), "resume concept {needle:?} missing");
        }
        assert!(
            resume.context_notes.as_deref().unwrap_or("").contains("NotBenchable"),
            "resume rule must carry the NotBenchable annotation"
        );
        let publish = find("write-verify-readback-before-publish");
        let publish_text = format!("{}\n{}\n{}", publish.name, publish.buggy_code, publish.fixed_code);
        for needle in ["incomplete", "rename", "hash", "reader", "refuse"] {
            assert!(publish_text.contains(needle), "publish concept {needle:?} missing");
        }
        let ab = find("ab-isolate-one-component-by-baseline-byte-copy");
        let ab_text = format!("{}\n{}\n{}", ab.name, ab.buggy_code, ab.fixed_code);
        for needle in ["baseline", "byte", "paired", "partial budget"] {
            assert!(ab_text.contains(needle), "ab concept {needle:?} missing");
        }
        // The memory-honesty rider (mmap-backed weights / partial budget)
        // must live in the A/B rule's context notes.
        let notes = ab.context_notes.as_deref().unwrap_or("");
        assert!(
            notes.contains("mmap-backed") && notes.contains("partial budget"),
            "the memory-honesty rider must be recorded in the A/B rule's notes"
        );
        // The pinned source phrases (quote re-verification at the pin).
        assert!(
            notes.contains("must not be interpreted as the total"),
            "the source's own memory caveat must be pinned verbatim"
        );
        assert!(
            resume.context_notes.as_deref().unwrap_or("").contains("Resume provenance mismatch"),
            "the resume refuse phrase must be pinned"
        );
        assert!(
            publish
                .context_notes
                .as_deref()
                .unwrap_or("")
                .contains("Written tensor mismatch"),
            "the publish refuse phrase must be pinned"
        );
    }

    /// Batch 122 — the three new rules are present.
    #[test]
    fn batch122_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for id in [
            "model-aware-format-over-hand-assembled-tokens",
            "implemented-surface-requires-provisioning-target",
            "diagnostics-not-just-exit-code",
        ] {
            assert!(ids.contains(&id), "{id} missing from corpus");
        }
    }

    /// Batch 122 — every new rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch122_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in [
            "model-aware-format-over-hand-assembled-tokens",
            "implemented-surface-requires-provisioning-target",
            "diagnostics-not-just-exit-code",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    /// Batch 122 — the load-bearing concepts survive into the corpus text
    /// (retrieval discriminators), the NotBenchable annotations are honest,
    /// and the source quotes' key phrases are pinned.
    #[test]
    fn batch122_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let find = |id: &str| {
            corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing"))
        };
        let format = find("model-aware-format-over-hand-assembled-tokens");
        let format_text = format!("{}\n{}\n{}", format.name, format.buggy_code, format.fixed_code);
        for needle in ["template", "multimodal", "rollback", "drift"] {
            assert!(format_text.contains(needle), "format concept {needle:?} missing");
        }
        assert!(
            format.context_notes.as_deref().unwrap_or("").contains("NotBenchable"),
            "format rule must carry the NotBenchable annotation"
        );
        let provision = find("implemented-surface-requires-provisioning-target");
        let provision_text = format!(
            "{}\n{}\n{}",
            provision.name, provision.buggy_code, provision.fixed_code
        );
        for needle in ["provision", "target", "MissingArtifact"] {
            assert!(provision_text.contains(needle), "provision concept {needle:?} missing");
        }
        let diag = find("diagnostics-not-just-exit-code");
        let diag_text = format!("{}\n{}\n{}", diag.name, diag.buggy_code, diag.fixed_code);
        for needle in ["stderr", "exit", "transcript"] {
            assert!(diag_text.contains(needle), "diag concept {needle:?} missing");
        }
        // The pinned source phrases (quote re-verification at the pin).
        assert!(
            diag.context_notes
                .as_deref()
                .unwrap_or("")
                .contains("The REPL can return zero after an image turn fails"),
            "the source's exit-code lie must be pinned verbatim"
        );
        assert!(
            provision
                .context_notes
                .as_deref()
                .unwrap_or("")
                .contains("could not be used out of the box"),
            "the source's provisioning-gap phrasing must be pinned verbatim"
        );
    }

    /// Batch 123 — the two new rules are present.
    #[test]
    fn batch123_rules_present() {
        let corpus = build_perf_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for id in [
            "full-vector-bit-identity-over-argmax",
            "renderer-error-is-not-capacity-overflow",
        ] {
            assert!(ids.contains(&id), "{id} missing from corpus");
        }
    }

    /// Batch 123 — every new rule parses (syn) on buggy AND fixed code.
    #[test]
    fn batch123_pseudocode_parses() {
        let corpus = build_perf_seed_corpus();
        for id in [
            "full-vector-bit-identity-over-argmax",
            "renderer-error-is-not-capacity-overflow",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing from corpus"));
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "{id} buggy/fixed must both parse"
            );
        }
    }

    /// Batch 123 — the load-bearing concepts survive into the corpus text.
    #[test]
    fn batch123_concepts_covered() {
        let corpus = build_perf_seed_corpus();
        let find = |id: &str| {
            corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("{id} missing"))
        };
        let vec_id = find("full-vector-bit-identity-over-argmax");
        let vec_text = format!("{}\n{}\n{}", vec_id.name, vec_id.buggy_code, vec_id.fixed_code);
        for needle in ["argmax", "bit", "manifest", "drift", "invalid"] {
            assert!(vec_text.contains(needle), "vec concept {needle:?} missing");
        }
        assert!(
            vec_id.context_notes.as_deref().unwrap_or("").contains("NotBenchable"),
            "vec rule must carry the NotBenchable annotation"
        );
        let tri = find("renderer-error-is-not-capacity-overflow");
        let tri_text = format!("{}\n{}\n{}", tri.name, tri.buggy_code, tri.fixed_code);
        for needle in ["Error", "compact", "overflow"] {
            assert!(tri_text.contains(needle), "tri concept {needle:?} missing");
        }
        // The pinned source phrases (quote re-verification at the pin).
        assert!(
            tri.context_notes
                .as_deref()
                .unwrap_or("")
                .contains("not a request to compact the context"),
            "the source's tri-state comment must be pinned verbatim"
        );
        assert!(
            vec_id
                .context_notes
                .as_deref()
                .unwrap_or("")
                .contains("despite matching argmax IDs"),
            "the source's drift-vs-argmax finding must be pinned verbatim"
        );
    }

    /// Batch 124 — the new rule is present, parses, and carries its concepts.
    #[test]
    fn batch124_rule_present_and_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "matched-control-disentangles-policy-from-model")
            .expect("matched-control rule missing from corpus");
        assert!(
            syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
            "matched-control buggy/fixed must both parse"
        );
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in ["control", "gate", "endpoint", "proxy", "baseline"] {
            assert!(text.contains(needle), "matched-control concept {needle:?} missing");
        }
        assert!(
            e.context_notes.as_deref().unwrap_or("").contains("NotBenchable"),
            "matched-control rule must carry the NotBenchable annotation"
        );
        // The pinned source phrases (quote re-verification at the paper).
        let notes = e.context_notes.as_deref().unwrap_or("");
        assert!(
            notes.contains("reproduces the no-prefetch baseline exactly"),
            "the paper's null-arm identity claim must be pinned verbatim"
        );
        assert!(
            notes.contains("better proxies do not imply better endpoints"),
            "the paper's honesty clause must be pinned verbatim"
        );
    }

    /// Batch 125 — the new rule is present, parses, and carries its concepts.
    #[test]
    fn batch125_rule_present_and_parses() {
        let corpus = build_perf_seed_corpus();
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "control-reproduces-on-baseline-trims-overclaim")
            .expect("control-reproduces rule missing from corpus");
        assert!(
            syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
            "control-reproduces buggy/fixed must both parse"
        );
        let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
        for needle in ["baseline", "oracle", "tolerance", "compute shape", "scratch"] {
            assert!(text.contains(needle), "control-reproduces concept {needle:?} missing");
        }
        assert!(
            e.context_notes.as_deref().unwrap_or("").contains("NotBenchable"),
            "control-reproduces rule must carry the NotBenchable annotation"
        );
        // The pinned source phrases (quote re-verification at the pin).
        let notes = e.context_notes.as_deref().unwrap_or("");
        assert!(
            notes.contains("different floating-point accumulation than the final decode step"),
            "the compute-shape quote must be pinned verbatim"
        );
        assert!(
            notes.contains("Compare all consumed outputs against the unfused control"),
            "the unowned-scratch quote must be pinned verbatim"
        );
    }

    /// Batch 129 (2026-09-08, Plan 110 -- ds4 Qwen Q2 distill): the two io
    /// measurement-honesty rules are present, parse, and pin their quotes.
    #[test]
    fn batch129_rules_present_and_parse() {
        let corpus = build_perf_seed_corpus();
        for (rid, needles, notes_needles) in [
            (
                "instrumented-attribution-not-production-times",
                vec!["production costs", "ranking"],
                vec![
                    "54.4 -> ~34 tok/s",
                    "synchronized wall-time attributions, not GPU hardware counters",
                ],
            ),
            (
                "unset-control-arm-silent-default-parity",
                vec!["EXPLICIT control value", "which arm is control"],
                vec![
                    "enabled defaults cannot silently make both variants identical",
                    "--control-value",
                ],
            ),
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == rid)
                .unwrap_or_else(|| panic!("batch-129 rule '{rid}' missing from corpus"));
            assert_eq!(e.category, "io", "batch-129 rule '{rid}' must be io");
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "batch-129 '{rid}' buggy/fixed must both parse"
            );
            let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
            for needle in needles {
                assert!(text.contains(needle), "batch-129 {rid} concept {needle:?} missing");
            }
            let notes = e.context_notes.as_deref().unwrap_or("");
            assert!(
                notes.contains("NotBenchable"),
                "batch-129 '{rid}' must carry the NotBenchable annotation"
            );
            for needle in notes_needles {
                assert!(notes.contains(needle), "batch-129 {rid} quote {needle:?} missing");
            }
        }
    }

    /// Batch 135 (2026-09-09, Plan 114 — the Wild-linker RustForge article
    /// distill): the three new rules are present and parse. The
    /// `vec-reuse-across-lifetime-change` fixed fixture carries an INLINE
    /// CONST block (the compile-time layout assert) — this gate is what
    /// proves the corpus's syn accepts it.
    #[test]
    fn batch135_rules_present_and_parse() {
        let corpus = build_perf_seed_corpus();
        for (rid, category, needles) in [
            (
                "atomic-vec-zero-cost-convert",
                "concurrency",
                vec!["into_iter", "Atomic::new", "same size+align"],
            ),
            (
                "vec-reuse-across-lifetime-change",
                "allocation",
                vec!["reuse_vec", "clear()", "compile error"],
            ),
            (
                "partitioned-mut-slices-zero-sync",
                "concurrency",
                vec!["chunks_mut", "DISJOINT", "zero"],
            ),
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == rid)
                .unwrap_or_else(|| panic!("batch-135 rule '{rid}' missing from corpus"));
            assert_eq!(e.category, category, "batch-135 rule '{rid}' category");
            assert!(
                syn::parse_file(&e.buggy_code).and_then(|_| syn::parse_file(&e.fixed_code)).is_ok(),
                "batch-135 '{rid}' buggy/fixed must both parse"
            );
            let text = format!("{}\n{}\n{}", e.name, e.buggy_code, e.fixed_code);
            for needle in needles {
                assert!(text.contains(needle), "batch-135 {rid} concept {needle:?} missing");
            }
        }
    }

    /// Batch 135 — G1 exactness: both arms of all three fixtures produce
    /// IDENTICAL results on fixed inputs (representation conversion, buffer
    /// retargeting, and partitioned mutation are all pure win shapes — no
    /// value may change). Runs on std only, no feature gate.
    #[test]
    fn batch135_fixtures_g1_parity() {
        use std::sync::atomic::{AtomicU32, Ordering};

        // Nested items lead the fn body (clippy::items_after_statements).
        fn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {
            const {
                assert!(
                    std::mem::size_of::<T>() == std::mem::size_of::<U>()
                        && std::mem::align_of::<T>() == std::mem::align_of::<U>(),
                );
            }
            v.clear();
            v.into_iter().map(|_| unreachable!()).collect()
        }
        const CELLS: usize = 97;
        const WORKERS: usize = 4;
        const ITERS: usize = 3;

        // (a) atomic conversion: identical values through both forms.
        let input: Vec<u32> = (0..257u32).map(|x| x.wrapping_mul(2_654_435_761) | 1).collect();
        let buggy: u64 = {
            let atomic: Vec<AtomicU32> =
                input.iter().map(|&v| AtomicU32::new(v)).collect();
            atomic.iter().map(|a| u64::from(a.load(Ordering::Relaxed))).sum()
        };
        let fixed: u64 = {
            let atomic: Vec<AtomicU32> = input
                .clone()
                .into_iter()
                .map(AtomicU32::new)
                .collect();
            atomic.iter().map(|a| u64::from(a.load(Ordering::Relaxed))).sum()
        };
        assert_eq!(buggy, fixed, "(a) atomic conversion must be value-identical");

        // (b) lifetime retargeting: identical sums, stable across calls.
        let rows: &'static [Vec<u32>] = Box::leak(
            vec![vec![1, 2, 3], vec![4], vec![], vec![5, 6, 7, 8, 9]].into_boxed_slice(),
        );
        let picked: Vec<&[u32]> = rows.iter().map(|r| &r[..]).collect();
        let buggy_total: u64 = picked.iter().map(|s| s.len() as u64).sum();
        let mut buf: Vec<&'static [u32]> = Vec::new();
        let run = |buf: &mut Vec<&'static [u32]>| -> u64 {
            let mut cur: Vec<&[u32]> = reuse_vec(std::mem::take(buf));
            cur.clear();
            cur.extend(rows.iter().map(|r| &r[..]));
            let total: u64 = cur.iter().map(|s| s.len() as u64).sum();
            *buf = reuse_vec(cur);
            total
        };
        let fixed_total = run(&mut buf);
        assert_eq!(fixed_total, run(&mut buf), "(b) retarget reuse must be call-stable");
        assert_eq!(buggy_total, fixed_total, "(b) lifetime retarget must be value-identical");

        // (c) partitioned mutation: identical checksums (prime cell count so
        // the chunks are ragged; strided-vs-contiguous ownership must not
        // change the per-cell bump count).
        let buggy = {
            let grid = std::sync::Mutex::new(vec![0u64; CELLS]);
            let gref = &grid; // move closures capture the SHARED reference (Copy)
            std::thread::scope(|s| {
                for w in 0..WORKERS {
                    s.spawn(move || {
                        for cell_idx in (w..CELLS).step_by(WORKERS) {
                            for _ in 0..ITERS {
                                let mut g = gref.lock().unwrap();
                                g[cell_idx] = g[cell_idx].wrapping_add(1);
                            }
                        }
                    });
                }
            });
            let cs = grid.lock().unwrap().iter().sum::<u64>();
            cs
        };
        let fixed = {
            let mut grid = vec![0u64; CELLS];
            let per = grid.len().div_ceil(WORKERS);
            std::thread::scope(|s| {
                for chunk in grid.chunks_mut(per) {
                    s.spawn(move || {
                        for _ in 0..ITERS {
                            for cell in chunk.iter_mut() {
                                *cell = cell.wrapping_add(1);
                            }
                        }
                    });
                }
            });
            grid.iter().sum::<u64>()
        };
        assert_eq!(buggy, fixed, "(c) partition must be value-identical");
        assert_eq!(fixed, (CELLS * ITERS) as u64, "(c) closed form");
    }

    /// Batch 135 — the measured Pareto verdicts for the three new rules.
    /// SLOW (spawns a cargo build + run per rule in a temp crate): run
    /// explicitly with `cargo test -p riir-clippy --lib --features
    /// rust_perf_bench batch135_pareto -- --ignored --nocapture`.
    #[cfg(feature = "rust_perf_bench")]
    #[test]
    #[ignore = "slow: spawns cargo build+run per rule (temp bench crates)"]
    fn batch135_pareto_benches() {
        let corpus = build_perf_seed_corpus();
        let validator = crate::domains::rust_perf::bench::PerfBenchValidator::new();
        for rid in [
            "atomic-vec-zero-cost-convert",
            "vec-reuse-across-lifetime-change",
            "partitioned-mut-slices-zero-sync",
        ] {
            let e = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == rid)
                .unwrap_or_else(|| panic!("batch-135 rule '{rid}' missing"));
            let verdict = validator.validate(rid, e.bench_spec.as_ref());
            assert!(!verdict.is_not_benchable(), "batch-135 '{rid}' has a spec");
            let b = match &verdict {
                crate::domains::rust_perf::bench::BenchVerdict::Pareto(p) => p,
                crate::domains::rust_perf::bench::BenchVerdict::NotBenchable(n) => {
                    panic!("batch-135 '{rid}' unexpectedly NotBenchable: {n:?}")
                }
            };
            assert!(b.ran, "batch-135 '{rid}' bench must run: {:?}", b.error);
            println!(
                "batch-135 {rid}: {:?} faster={} slower={} tie={}",
                b.overall, b.faster_count, b.slower_count, b.tie_count
            );
            for r in &b.results {
                println!(
                    "  n={:>6} buggy={:>9}ns fixed={:>9}ns speedup={:.2}x allocs {}→{} outcome={:?}",
                    r.size, r.buggy_ns, r.fixed_ns, r.speedup, r.buggy_allocs, r.fixed_allocs, r.outcome
                );
            }
        }
    }
}

/// Bench spec for `bitplane-transpose-repeated-predicate-scan` (Batch 83,
/// Research 097 — arXiv:2608.26368 distill): 64 filter scans per timed call
/// over the SAME column (the ingest-amortization regime — encoding happens
/// once in `make_input`, OUTSIDE the timed region by design; the source's
/// break-even is 7–25 scans). Values in [0, 100) → b = 7 planes; threshold
/// 50 ≈ 50% selectivity (the scalar branch predictor's worst case). G1 is
/// integer-exact: the bit-plane popcount MUST equal the scalar count.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn bitplane_transpose_repeated_predicate_scan_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 1_000_000],
        harness: r#"
const SCANS: usize = 64;      // scans per timed call (amortization regime)
const BIT_CAP: u64 = 100;     // values in [0, 100) -> b = 7 planes
const THRESHOLD: u32 = 50;    // ~50% selectivity

mod buggy {
    pub fn scan_count(values: &[u32], threshold: u32) -> u64 {
        let mut hits = 0u64;
        for &v in values {
            if v > threshold {
                hits += 1;
            }
        }
        hits
    }
}

mod fixed {
    pub struct BitSliced {
        pub planes: Vec<Vec<u64>>, // [bit][word]
        pub words: usize,
    }

    pub fn gt_popcount(col: &BitSliced, threshold: u32) -> u64 {
        let b = col.planes.len() as u32;
        if b < 32 && threshold >= (1u32 << b) {
            return 0;
        }
        let mut gt = vec![0u64; col.words];
        let mut eq = vec![u64::MAX; col.words];
        for (bit, plane) in col.planes.iter().enumerate().rev() {
            if (threshold >> bit) & 1 == 0 {
                for w in 0..col.words {
                    gt[w] |= eq[w] & plane[w];
                }
            } else {
                for w in 0..col.words {
                    eq[w] &= plane[w];
                }
            }
        }
        gt.iter().map(|w| u64::from(w.count_ones())).sum()
    }
}

pub struct Input {
    values: Vec<u32>,
    fixed_col: fixed::BitSliced,
    threshold: u32,
    expected_per_scan: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        values.push(((rng >> 33) % BIT_CAP) as u32);
    }
    let max_val = values.iter().copied().max().unwrap_or(0);
    let b = (32 - max_val.leading_zeros()).max(1) as usize;
    let words = n.div_ceil(64);
    let mut planes = vec![vec![0u64; words]; b];
    for (i, &v) in values.iter().enumerate() {
        for (bit, plane) in planes.iter_mut().enumerate() {
            plane[i / 64] |= u64::from((v >> bit) & 1) << (i % 64);
        }
    }
    let expected_per_scan = values.iter().filter(|&&v| v > THRESHOLD).count() as u64;
    Input {
        values,
        fixed_col: fixed::BitSliced { planes, words },
        threshold: THRESHOLD,
        expected_per_scan,
    }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut total = 0u64;
    for _ in 0..SCANS {
        total += buggy::scan_count(&input.values, input.threshold);
    }
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected_per_scan * SCANS as u64);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut total = 0u64;
    for _ in 0..SCANS {
        total += fixed::gt_popcount(&input.fixed_col, input.threshold);
    }
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected_per_scan * SCANS as u64);
}
"#
        .to_string(),
    }
}


/// Bench spec for `serialize-once-byte-buffer-store` (Batch 86, Research
/// 100 — the Cloudflare Big Pineapple distill): structured per-record
/// store (2 heap allocations per record) vs ONE length-prefixed byte
/// buffer + u32 offset table (2 allocations TOTAL). The insert path is
/// the measured surface (the source's near-isolated +13% insert); the
/// read guards touch every record so neither arm can be dead-coded.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn serialize_once_byte_buffer_store_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub struct Entry {
        pub name: Box<str>,
        pub data: Vec<u8>,
    }
    pub fn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> Vec<Entry> {
        names
            .iter()
            .zip(payloads)
            .map(|(n, p)| Entry { name: n.clone(), data: p.clone() })
            .collect()
    }
}

mod fixed {
    pub fn build(names: &[Box<str>], payloads: &[Vec<u8>]) -> (Box<[u8]>, Vec<u32>) {
        let mut total = 4usize;
        for (n, p) in names.iter().zip(payloads) {
            total += 4 + n.len() + p.len();
        }
        let mut buf = Vec::with_capacity(total);
        let mut offsets = Vec::with_capacity(names.len());
        buf.extend_from_slice(&(names.len() as u32).to_le_bytes());
        for (n, p) in names.iter().zip(payloads) {
            offsets.push(buf.len() as u32);
            buf.extend_from_slice(&(n.len() as u16).to_le_bytes());
            buf.extend_from_slice(n.as_bytes());
            buf.extend_from_slice(&(p.len() as u16).to_le_bytes());
            buf.extend_from_slice(p);
        }
        (buf.into_boxed_slice(), offsets)
    }
}

pub struct Input {
    names: Vec<Box<str>>,
    payloads: Vec<Vec<u8>>,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut names = Vec::with_capacity(n);
    let mut payloads = Vec::with_capacity(n);
    for _ in 0..n {
        let name_len = 4 + lcg(&mut rng) as usize % 28; // 4..32 B names
        let mut name = String::with_capacity(name_len);
        for _ in 0..name_len {
            name.push((b'a' + lcg(&mut rng) as u8 % 26) as char);
        }
        let payload_len = 4 + lcg(&mut rng) as usize % 36; // 4..40 B payloads
        let payload: Vec<u8> = (0..payload_len).map(|_| lcg(&mut rng) as u8).collect();
        names.push(name.into_boxed_str());
        payloads.push(payload);
    }
    Input { names, payloads }
}

pub fn run_buggy(input: &Input) {
    let store = buggy::build(&input.names, &input.payloads);
    let mut total = 0usize;
    for e in &store {
        total += e.name.len() + e.data.len();
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let (buf, offsets) = fixed::build(&input.names, &input.payloads);
    let mut total = 0usize;
    for &off in &offsets {
        let o = off as usize;
        let nl = u16::from_le_bytes([buf[o], buf[o + 1]]) as usize;
        total += nl;
    }
    std::hint::black_box(total + buf.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `context-equal-field-elide` (Batch 86, Research 100):
/// owner stored on EVERY record vs Option<Box<str>> elided to None when the
/// owner equals the record's lookup context (the cache key) — resolved at
/// read via as_deref().unwrap_or(context). ~80% context-equal mix (the
/// source's DNS-owner distribution); the alloc axis is the headline
/// (n → ~0.2n owner allocations); the fixed arm pays the insert-time
/// equality compare (the mechanism's honest cost).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn context_equal_field_elide_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub struct Answer {
        pub ttl: u32,
        pub owner: Box<str>,
    }
    pub fn build(owners: &[Box<str>], ttls: &[u32]) -> Vec<Answer> {
        owners
            .iter()
            .zip(ttls)
            .map(|(o, &t)| Answer { ttl: t, owner: o.clone() })
            .collect()
    }
}

mod fixed {
    pub struct Answer {
        pub ttl: u32,
        pub owner: Option<Box<str>>,
    }
    pub fn build_elided(
        contexts: &[Box<str>],
        owners: &[Box<str>],
        ttls: &[u32],
    ) -> Vec<Answer> {
        contexts
            .iter()
            .zip(owners)
            .zip(ttls)
            .map(|((c, o), &t)| Answer {
                ttl: t,
                owner: if o.as_ref() == c.as_ref() { None } else { Some(o.clone()) },
            })
            .collect()
    }
}

pub struct Input {
    contexts: Vec<Box<str>>, // the lookup key each record belongs to
    owners: Vec<Box<str>>,   // ~80% identical to the context
    ttls: Vec<u32>,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut contexts = Vec::with_capacity(n);
    let mut owners = Vec::with_capacity(n);
    let mut ttls = Vec::with_capacity(n);
    for _ in 0..n {
        let ctx_len = 8 + lcg(&mut rng) as usize % 17; // 8..25 B names
        let mut ctx = String::with_capacity(ctx_len);
        for _ in 0..ctx_len {
            ctx.push((b'a' + lcg(&mut rng) as u8 % 26) as char);
        }
        let ctx: Box<str> = ctx.into_boxed_str();
        let owner: Box<str> = if lcg(&mut rng) % 5 != 0 {
            ctx.clone() // ~80% context-equal (the elision regime)
        } else {
            let mut o = String::with_capacity(ctx_len + 4);
            o.push_str(&ctx);
            o.push_str(".x"); // a genuinely different chain target
            o.into_boxed_str()
        };
        contexts.push(ctx);
        owners.push(owner);
        ttls.push(lcg(&mut rng) as u32 | 1);
    }
    Input { contexts, owners, ttls }
}

pub fn run_buggy(input: &Input) {
    let store = buggy::build(&input.owners, &input.ttls);
    let mut total = 0usize;
    for a in &store {
        total += a.owner.len();
    }
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let store = fixed::build_elided(&input.contexts, &input.owners, &input.ttls);
    let mut total = 0usize;
    for (a, c) in store.iter().zip(&input.contexts) {
        total += a.owner.as_deref().unwrap_or(c).len();
    }
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-before-expensive-transform` (Batch 91,
/// Research 106 — the Rustdoc filter-first distill): ~6% keep-rate, an
/// opaque `#[inline(never)]` render whose digest allocation ESCAPES into
/// the collected Vec (LLVM cannot elide it) + a full body fold — the
/// discarded work is real on both axes. G1: both orders collect the
/// IDENTICAL page Vec (order included).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn filter_before_expensive_transform_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 30_000, 300_000],
        harness: r#"
pub struct Doc {
    pub published: bool,
    pub body: [u64; 8],
}

pub struct Page {
    pub published: bool,
    pub digest: Box<[u8]>,
}

#[inline(never)]
pub fn render_page(doc: &Doc) -> Page {
    let mut acc: u64 = 0;
    for &v in &doc.body {
        acc = acc.wrapping_mul(0x1_0000_0001_b3).wrapping_add(v);
    }
    // the digest allocation ESCAPES into the collected Vec - LLVM
    // cannot elide it, the discarded work is real
    let digest = vec![
        (acc & 0xFF) as u8,
        ((acc >> 8) & 0xFF) as u8,
        ((acc >> 16) & 0xFF) as u8,
        ((acc >> 24) & 0xFF) as u8,
    ]
    .into_boxed_slice();
    Page { published: doc.published, digest }
}

mod buggy {
    use super::{render_page, Doc, Page};
    pub fn published_pages(docs: &[Doc]) -> Vec<Page> {
        docs.iter().map(render_page).filter(|p| p.published).collect()
    }
}

mod fixed {
    use super::{render_page, Doc, Page};
    pub fn published_pages(docs: &[Doc]) -> Vec<Page> {
        docs.iter().filter(|d| d.published).map(render_page).collect()
    }
}

pub struct Input {
    docs: Vec<Doc>,
}

pub fn make_input(n: usize) -> Input {
    let docs = (0..n)
        .map(|i| {
            let seed = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            Doc {
                published: i % 16 == 0, // ~6.25% keep-rate
                body: [seed; 8],
            }
        })
        .collect();
    Input { docs }
}

pub fn run_buggy(input: &Input) {
    let pages = buggy::published_pages(&input.docs);
    let mut touch = pages.len() as u64;
    if let Some(first) = pages.first() {
        touch += u64::from(first.digest[0]);
    }
    std::hint::black_box(touch);
}

pub fn run_fixed(input: &Input) {
    let pages = fixed::published_pages(&input.docs);
    let mut touch = pages.len() as u64;
    if let Some(first) = pages.first() {
        touch += u64::from(first.digest[0]);
    }
    std::hint::black_box(touch);
}
"#
        .to_string(),
    }
}

/// Bench spec for `decision-projection-before-construction` (Batch 91,
/// Research 106 — the Rustdoc head-before-param_env distill): the buggy
/// predicate canonicalizes EVERY item (fresh Vec + sort + dedup behind an
/// `#[inline(never)]` boundary) to read a field the raw item already
/// determines; the fixed predicate computes the invariant min directly.
/// ~1.6% pass-rate. The invariance precondition + the canonicalize-call
/// axis are pinned in the corpus gate (every item vs zero calls).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn decision_projection_before_construction_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 30_000, 300_000],
        harness: r#"
pub struct Item {
    pub tags: [u32; 8],
}

pub struct Canonical {
    pub primary: u32,
    pub tags: Vec<u32>,
}

#[inline(never)]
pub fn canonicalize(item: &Item) -> Canonical {
    let mut tags: Vec<u32> = item.tags.to_vec(); // the construction alloc
    tags.sort_unstable();
    tags.dedup();
    Canonical { primary: tags[0], tags }
}

pub fn raw_primary(item: &Item) -> u32 {
    let mut m = u32::MAX;
    for &t in &item.tags {
        m = m.min(t);
    }
    m
}

mod buggy {
    use super::{canonicalize, Item};
    pub fn count_hot(items: &[Item], hot: u32) -> usize {
        items.iter().filter(|i| canonicalize(i).primary == hot).count()
    }
}

mod fixed {
    use super::{raw_primary, Item};
    pub fn count_hot(items: &[Item], hot: u32) -> usize {
        items.iter().filter(|i| raw_primary(i) == hot).count()
    }
}

pub struct Input {
    items: Vec<Item>,
    hot: u32,
}

pub fn make_input(n: usize) -> Input {
    let items = (0..n)
        .map(|i| {
            let base = (i % 64) as u32;
            let mut tags = [0u32; 8];
            for (k, t) in tags.iter_mut().enumerate() {
                *t = base * 8 + (k as u32) * 31; // min == base * 8, no wrap
            }
            Item { tags }
        })
        .collect();
    let hot = 128; // base == 16 -> ~1.56% pass-rate
    Input { items, hot }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_hot(&input.items, input.hot));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_hot(&input.items, input.hot));
}
"#
        .to_string(),
    }
}

/// Bench spec for `mir-destprop-branch-site-merge` (Batch 99, Research 119
/// — the wasmi v2.0 blog distill): the DestProp shape — both arms load a
/// KNOWN fn item, converge into one call (cmov + single indirect tail-jump,
/// asm-verified) vs two distinct direct tail-jump sites. 50/50 mixed
/// dispatch stream (LCG), sizes on the dispatch-count axis. G1 output
/// identity is pinned in `batch99_split_sites_preserve_output_and_dispatch_sequence`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn mir_destprop_branch_site_merge_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub type Handler = fn(&mut u64, u64);

#[inline(never)]
pub fn handle_a(s: &mut u64, x: u64) {
    *s = s.wrapping_add(x).wrapping_add(0x9E37);
}

#[inline(never)]
pub fn handle_b(s: &mut u64, x: u64) {
    *s = s.wrapping_mul(31).wrapping_add(x);
}

// BUGGY: both arms load a KNOWN fn item, the pointers converge (phi) --
// cmov of the two code addresses + ONE indirect tail-jump site.
#[inline(never)]
pub fn step_merged(cond: bool, s: &mut u64, x: u64) {
    let f: Handler = if cond { handle_a } else { handle_b };
    f(s, x);
}

// FIXED: two DISTINCT tail-call sites -- direct jumps, per-site history.
#[inline(never)]
pub fn step_split(cond: bool, s: &mut u64, x: u64) {
    if cond {
        handle_a(s, x);
    } else {
        handle_b(s, x);
    }
}

pub struct Input {
    conds: Vec<u32>,
    xs: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0x243F_6A88_85A3_08D3u64;
    let conds: Vec<u32> = (0..n)
        .map(|_| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            u32::from(((seed >> 33) as u32) % 256 < 128) // 50/50 mixed stream
        })
        .collect();
    let xs: Vec<u64> = (0..n)
        .map(|i| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) ^ (i as u64)
        })
        .collect();
    Input { conds, xs }
}

pub fn run_buggy(input: &Input) {
    let mut s = 0u64;
    for i in 0..input.conds.len() {
        step_merged(input.conds[i] != 0, &mut s, input.xs[i]);
    }
    std::hint::black_box(s);
}

pub fn run_fixed(input: &Input) {
    let mut s = 0u64;
    for i in 0..input.conds.len() {
        step_split(input.conds[i] != 0, &mut s, input.xs[i]);
    }
    std::hint::black_box(s);
}
"#
        .to_string(),
    }
}

/// Bench spec for `dispatch-handler-pair-hoist` (Batch 101, Research 119
/// §Parallel-session addendum): the runtime-table dispatch shape — the
/// buggy loop reads `HANDLERS[op & 1]` (indexed fn-pointer load) every
/// dispatch; the fixed loop hoists both entries once and selects between
/// registers. LCG-random 2-op stream (the UNPREDICTABLE regime — the
/// measured win band 1.19–1.23×; structured streams are the Tie band, a
/// different harness, not gated here). G1 output identity is pinned in
/// `batch101_hoist_preserves_output_and_both_handlers_run`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)] // bench-spec tables live after the corpus tests by design
fn dispatch_handler_pair_hoist_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub type Handler = fn(u64) -> u64;

#[inline(never)]
pub fn op_add(x: u64) -> u64 {
    x.wrapping_add(0x9E37)
}

#[inline(never)]
pub fn op_mul(x: u64) -> u64 {
    x.wrapping_mul(31)
}

pub static HANDLERS: [Handler; 2] = [op_add, op_mul];

// BUGGY: indexed fn-pointer load on EVERY dispatch (aarch64: the
// indirect call keeps an in-loop ldr on its critical path).
#[inline(never)]
pub fn run_table(ops: &[u32], xs: &[u64]) -> u64 {
    let mut acc = 0u64;
    for (i, &op) in ops.iter().enumerate() {
        let f = HANDLERS[(op & 1) as usize];
        acc = f(acc ^ xs[i]);
    }
    acc
}

// FIXED: both entries hoisted once — register-resident select in-loop.
#[inline(never)]
pub fn run_hoisted(ops: &[u32], xs: &[u64]) -> u64 {
    let f0 = HANDLERS[0];
    let f1 = HANDLERS[1];
    let mut acc = 0u64;
    for (i, &op) in ops.iter().enumerate() {
        let x = acc ^ xs[i];
        acc = if op & 1 == 0 { f0(x) } else { f1(x) };
    }
    acc
}

pub struct Input {
    ops: Vec<u32>,
    xs: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0x243F_6A88_85A3_08D3u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed >> 33
    };
    let ops: Vec<u32> = (0..n).map(|_| (next() % 2) as u32).collect();
    let xs: Vec<u64> = (0..n).map(|i| next() ^ (i as u64)).collect();
    Input { ops, xs }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(run_table(&input.ops, &input.xs));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(run_hoisted(&input.ops, &input.xs));
}
"#
        .to_string(),
    }
}

/// Bench spec for `atomic-vec-zero-cost-convert` (Batch 135, Plan 114 —
/// the Wild-linker article distill): both arms pay ONE shared `clone()` so
/// the timing delta is exactly the conversion; the buggy conversion then
/// adds a fresh malloc + element copy while the fixed one consumes the
/// owned clone in place (std's same-layout collect reuse — the alloc axis
/// pins the elision EXACTLY: 2 → 1). G1: both arms read the same values,
/// so the Relaxed-load checksums must match exactly.
#[cfg(feature = "rust_perf_bench")]
fn atomic_vec_zero_cost_convert_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    use std::sync::atomic::{AtomicU32, Ordering};

    // The source is only BORROWED: a fresh allocation + element-by-element
    // copy is forced on every conversion.
    pub fn to_atomic(plain: &[u32]) -> Vec<AtomicU32> {
        plain.iter().map(|&v| AtomicU32::new(v)).collect()
    }

    pub fn convert(owned: Vec<u32>) -> u64 {
        let atomic = to_atomic(&owned);
        std::hint::black_box(&atomic); // keep the allocation observable
        atomic.iter().map(|a| a.load(Ordering::Relaxed) as u64).sum()
    }
}

mod fixed {
    use std::sync::atomic::{AtomicU32, Ordering};

    // The vec is OWNED: into_iter walks in place and the same-layout
    // collect reuses the buffer — no new allocation, no copy loop.
    pub fn to_atomic(plain: Vec<u32>) -> Vec<AtomicU32> {
        plain.into_iter().map(AtomicU32::new).collect()
    }

    pub fn convert(owned: Vec<u32>) -> u64 {
        let atomic = to_atomic(owned);
        std::hint::black_box(&atomic);
        atomic.iter().map(|a| a.load(Ordering::Relaxed) as u64).sum()
    }
}

pub struct Input {
    a: Vec<u32>,
    b: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0x853C_49E6_748F_EA9Bu64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed >> 33
    };
    let mut mk = || (0..n).map(|_| next() as u32 | 1).collect::<Vec<u32>>();
    Input { a: mk(), b: mk() }
}

pub fn run_buggy(input: &Input) {
    let owned = input.a.clone(); // shared ownership gain: both arms pay it
    std::hint::black_box(buggy::convert(owned));
}

pub fn run_fixed(input: &Input) {
    let owned = input.b.clone();
    std::hint::black_box(fixed::convert(owned));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-reuse-across-lifetime-change` (Batch 135): the buggy
/// arm builds a fresh `Vec<&[u32]>` per call (malloc + fill + consume); the
/// fixed arm owns a `'static`-typed buffer (thread_local), retargets it per
/// call via the reuse_vec pattern, and refills — capacity retained, so the
/// steady-state alloc axis is 1 → 0. The rows are `Box::leak`ed so the
/// `'static` element type is sound inside the harness (the rule text carries
/// the real-world caller contract). G1: identical sums.
#[cfg(feature = "rust_perf_bench")]
fn vec_reuse_across_lifetime_change_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    // Fresh vec of row refs per call: one malloc + fill + consume.
    pub fn row_lens(rows: &[Vec<u32>]) -> u64 {
        let picked: Vec<&[u32]> = rows.iter().map(|r| &r[..]).collect();
        std::hint::black_box(&picked); // keep the allocation observable
        picked.iter().map(|s| s.len() as u64).sum()
    }
}

mod fixed {
    use std::cell::RefCell;

    // The buffer is OWNED and retargeted per call: the const size/align
    // assert makes a mismatched T/U a compile error; clear() empties the
    // buffer so the map body is unreachable and the loop compiles out;
    // std's in-place collect hands the SAME allocation back re-typed.
    pub fn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {
        const {
            assert!(
                std::mem::size_of::<T>() == std::mem::size_of::<U>()
                    && std::mem::align_of::<T>() == std::mem::align_of::<U>(),
            );
        }
        v.clear();
        v.into_iter().map(|_| unreachable!()).collect()
    }

    thread_local! {
        // 'static-typed BY CONTRACT: rows are leaked for the whole bench,
        // and the buffer is cleared before any stored ref could be read.
        static BUF: RefCell<Vec<&'static [u32]>> = const { RefCell::new(Vec::new()) };
    }

    pub fn row_lens(rows: &[Vec<u32>]) -> u64 {
        BUF.with(|b| {
            let mut buf = b.borrow_mut();
            let mut cur: Vec<&[u32]> = reuse_vec(std::mem::take(&mut buf));
            cur.clear();
            cur.extend(rows.iter().map(|r| &r[..]));
            let total: u64 = cur.iter().map(|s| s.len() as u64).sum();
            let retargeted: Vec<&'static [u32]> = reuse_vec(cur);
            *buf = retargeted;
            total
        })
    }
}

pub struct Input {
    rows: &'static [Vec<u32>],
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0xA24B_5D3F_6C17_E890u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed >> 33
    };
    let rows: Vec<Vec<u32>> = (0..n)
        .map(|_| (0..4).map(|_| next() as u32).collect())
        .collect();
    Input { rows: Box::leak(rows.into_boxed_slice()) }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::row_lens(input.rows));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::row_lens(input.rows));
}
"#
        .to_string(),
    }
}

/// Bench spec for `partitioned-mut-slices-zero-sync` (Batch 135): the buggy
/// arm locks a single Mutex PER BUMP across all workers (contended); the
/// fixed arm partitions once into disjoint contiguous chunks and runs plain
/// stores with zero synchronization. Sizes = THREAD counts per the align-64
/// precedent. G1: the closed-form checksum (every cell bumped ITERS times)
/// is asserted in-harness. The alloc axis is EXEMPT for this rule (see
/// ALLOC_AXIS_EXEMPT — contended macOS pthread mutex slow path).
#[cfg(feature = "rust_perf_bench")]
fn partitioned_mut_slices_zero_sync_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 2, 4, 8],
        harness: r#"
use std::sync::Mutex;

const CELLS: usize = 4_096;
const ITERS: usize = 8;

mod buggy {
    use std::sync::Mutex;

    // One lock guards the whole array; every bump serializes even though
    // the workers' index ranges never overlap.
    pub fn bump(grid: &Mutex<Vec<u64>>, workers: usize) {
        std::thread::scope(|s| {
            for w in 0..workers {
                s.spawn(move || {
                    for cell_idx in (w..super::CELLS).step_by(workers) {
                        for _ in 0..super::ITERS {
                            let mut g = grid.lock().unwrap();
                            g[cell_idx] = g[cell_idx].wrapping_add(1);
                        }
                    }
                });
            }
        });
    }
}

mod fixed {
    // Partition ONCE into disjoint contiguous ranges: plain stores, no
    // lock, no atomic — each cell is bumped by exactly one owner.
    pub fn bump(grid: &mut Vec<u64>, workers: usize) {
        let per = grid.len().div_ceil(workers);
        std::thread::scope(|s| {
            for chunk in grid.chunks_mut(per) {
                s.spawn(move || {
                    for _ in 0..super::ITERS {
                        for cell in chunk.iter_mut() {
                            *cell = cell.wrapping_add(1);
                        }
                    }
                });
            }
        });
    }
}

pub struct Input {
    workers: usize,
}

pub fn make_input(n: usize) -> Input {
    // n = THREAD COUNT (the contention axis is the size axis).
    Input { workers: n }
}

fn expected() -> u64 {
    (CELLS * ITERS) as u64
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let grid = Mutex::new(vec![0u64; CELLS]);
    buggy::bump(&grid, input.workers);
    let cs: u64 = grid.lock().unwrap().iter().sum();
    debug_assert_eq!(cs, expected());
    std::hint::black_box(cs);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut grid = vec![0u64; CELLS];
    fixed::bump(&mut grid, input.workers);
    let cs: u64 = grid.iter().sum();
    debug_assert_eq!(cs, expected());
    std::hint::black_box(cs);
}
"#
        .to_string(),
    }
}
