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

120 rules across 12 categories (rust-optimize skill checklist +
nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs
profiling guide, Research 032; **now 120 rules — Batch 136 (2026-09-09,
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
timing gate whose work folds away has no nonzero baseline to ratio). 63 of 120 carry
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
up from the corpus at heal time + surfaces it on ALL paths (drafter / memory /
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

## Build-config recommendations (not healable)

The code healer edits `.rs` via `syn`. PGO / LTO / codegen-units / BOLT are
`Cargo.toml` + CI-workflow changes — **out of scope for the healer.** This
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

The healer does NOT suggest these — they're Cargo.toml/CI changes, not code
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
| `fxhashmap` | std `HashMap` → `rustc_hash::FxHashMap` (use swap + group slimming + path rewrite) | healed crate needs the `rustc-hash` dep; glob/`pub use`/fn-local forms skipped |
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
- [`src/domains/rust_perf/corpus.rs`](../../src/domains/rust_perf/corpus.rs) — `build_perf_seed_corpus()` + 78 `BenchSpec`s (10 rules are honestly NotBenchable)
- [`src/domains/rust_perf/proposers/`](../../src/domains/rust_perf/proposers/) — the 10 `syn` AST proposers (Phase 4 top-5 + the 2026-08-17 Batch 48 + Batch 53 follow-ups) + `propose_fix` dispatch
- [`src/domains/rust_perf/verify.rs`](../../src/domains/rust_perf/verify.rs) — `PerfVerifier` (cargo test)
- [`src/domains/rust_perf/bench.rs`](../../src/domains/rust_perf/bench.rs) — `PerfBenchValidator` (Pareto verdict)
- [`examples/perf_heal_demo.rs`](../../examples/perf_heal_demo.rs) — scaffold end-to-end demo
- [`examples/perf_bench_demo.rs`](../../examples/perf_bench_demo.rs) — the GOAT gate (Bench 013 promotion + Bench 015 Phase 4 finalization)

## See also

- [`heal_domain_trait.md`](heal_domain_trait.md) — the trait this implements
- [Proposal 001 caveat 3](../../.proposals/001_multi_domain_modelless_code_healer.md) — the perf validator weakness (now closed)
- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the original promotion-gate GOAT results (historical; pre-expansion corpus snapshots)
- [Bench 015](../../.benchmarks/015_rust_perf_expansion_phase4_goat.md) — Phase 4 finalization on the expanded 39-rule corpus (29/31 ParetoDominant)
