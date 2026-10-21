# Plan 272 — Batch 269: the llama.cpp Metal reduction conversion (two-pass argmax + the pool_1d cooperative face)

**Status:** IN_PROGRESS — claimed 2026-10-21 ~22:4x +07 (zed-m3/M3, riir-refine idle loop; renumbered from a live collision — the shikuwa/4090 lane's batch 267 claim `ac1ae01a` landed first and keeps 267/271; this batch's lead fell into their llama.cpp "rest NULL" bucket and renumbers here)
**Claim:** zed-m3/M3 idle lane, expires 2026-10-23 ~22:4x +07 (48 h); push is the arbiter; other lanes re-fetch before staging and yield to this row.

## Source

ggml-org/llama.cpp @ `df11702136d02b9e8fa37971de0bf0e95a91b2de` — PR #30862 "metal : optimize
argmax and pool_1d" (MIT, merged 2026-10-20T13:46Z). Clone at `.raw/b269/llamacpp` (ephemeral,
removed at close). Every quote re-verified at the pin this session (the full diff read:
`kernels/misc.metal` +134, `kernels/pool.metal` +122/−44, `ggml-metal-ops.cpp` +79, 
`ggml-metal-impl.h` +7).

## The mechanism (read at the pin)

**Own rule — `two-pass-partial-reduction-inter-dispatch-barrier` (category B — Compute):**
when a row-reduction axis is too long for one cooperative threadgroup to chew efficiently
(ne00 > N_ARGMAX_CHUNK = 4096), split it ACROSS DISPATCHES: a part kernel (grid = npart ×
nrows, npart = min(256, ceil(ne00/4096)) bounded) reduces each chunk to a per-(row, part)
(max, arg) partial pair in scratch, then a final kernel (one threadgroup per row, nth widened
to the next pow2 ≥ npart capped 256) reduces the partials. The barrier is LOAD-BEARING: the
two dispatches share one command buffer and would otherwise overlap —
`ggml_metal_op_concurrency_reset(ctx)` = `ggml_metal_encoder_memory_barrier(enc)` + mem-ranges
reset sits between them (the stale-partials law). Scratch = (sizeof(float)+sizeof(int32_t)) ×
npart × ne01, allocated through the graph's buffer allocator, riding PAST the dst tensor in
the same buffer (`bid_tmp.offs += ggml_nbytes(op)`). Admission: small rows keep the single-pass
kernel (measured: 32,10 3.01→3.00 µs — no regression). The (value, index) pair reduces with a
select-mask index reduction: value by `simd_max`, index by `simd_max(select(-1, larg, lmax ==
max_val))` — the index rides only on lanes holding the max.

**Count-neutral rider — the Metal pool_1d face on `grid-over-output-thread-private-sweep`
(E — Parallelism, entries_gpu_06):** the same commit's pool_1d re-maps thread-per-output →
ONE THREADGROUP per output (nth = min(256, max(1, k0))), strided window loop, simd_max +
the shared-memory per-simdgroup tree (the same two-level shape), smem sized per op (max:
32 floats; avg: 2×32 = sum + count partials, the count incremented only for in-bounds
elements so padding never skews the average). This is the THIRD face of the window law
(Batch-209 CUDA pool2d warp-radius face is the second): same axis, threadgroup radius,
Metal ISA.

## MEASURED (source-reported, the PR body — test-backend-ops perf, never our bar)

M2 Ultra: POOL_1D max full-width 247.51→6.35 µs (~39×), 2303.73→36.70 (~63×),
2379.66→80.76 (~29×), 9613.05→368.94 (~26×); ARGMAX 32000,512 453.17→78.87 µs (5.7×);
small shapes flat (3.01→3.00). M5 Max: POOL_1D 110.52→2.79 (~40×), 316.13→35.37 (~8.9×),
440.14→5.37 (~82×); OUR SILICON FAMILY both boxes.

## Tasks

- [x] T0: claim + plan-first (this file; `.plans/.highwater` 271→272; collision yield row `7c98ecf1` in the snapshot)
- [x] T1: research note 304 (the poll + window record; `.research/.highwater` 302→304 — 303 left to the B267 lane)
- [x] T2: `entries_gpu_116.rs` — the own rule (B/Compute), quotes verbatim at the pin
- [x] T3: the count-neutral rider on `grid-over-output-thread-private-sweep` (entries_gpu_06)
- [x] T4: wiring — mod.rs (module registration + B category rule_id list + KERNEL_CATEGORIES comment), tests.rs count pins (924→925 ×3 sites + the category-tuple B count), router.rs expert-total pin (924→925)
- [x] T5: `batch269` needle test in tests.rs (the rule's verbatim quotes + the rider's needle + the count witness)
- [x] T6: gates — kernel_opt lib (floor 3203→3205 incl. new tests), default lib (2724→2725), clippy −D ×3 postures, retrieval floors RE-MEASURE (own rule lands); freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild NOT owed (kernel_opt outside the release set)
- [x] T7: records — snapshot landing row, AGENTS.md kernel_opt bullet count (924→925), README/.docs count rows, plan tick; commit + push; `.raw/b269` removed

## Honest scope notes

- The B267 lane (4090) staged the splash wave as B268's pool — untouched here; batch 268 stays
  theirs, hence the 269 number (a deliberate hole, never a reuse).
- The select-mask index reduction gives LAST-index-wins across lanes on exact ties (the
  sequential reference's `>` scan is first-index-within-chunk) — recorded in-rule as the
  tie-rule surface disclosure, not adjudicated here (upstream's own semantics question).
- wgpu was UNREACHABLE 3× this poll (2× ls-remote timeout + 1× API error) — transient,
  disclosed in the snapshot; the B267 lane read it fine from their box.

## Landing record (2026-10-21 ~23:5x +07)

Gates ALL GREEN: batch269 needle 1/1 FIRST-RUN; kernel_opt lib 3206/0; default lib 2726/0;
clippy −D ×3 postures clean (default/kernel_opt/no-default); combo kernel_opt+latent_retrieval
3393/0 (2121s) — retrieval floors HELD (kernel 100.0% top-3 30/30, blended 90.6/94.2/95.3,
no re-pin); freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild NOT owed
(kernel_opt outside the release set). En-route: the splice-defect class re-fired on the entry
file (the python opt_pattern append landed escaped-newline junk at the string junction — repaired
byte-exact; the B265 lesson holds: string-literal splices are a byte-level script job); the rider
initially needle-tested against the wrong host (grid-over-output — the B209 pool2d face's family
home is row-dot-split-inner-axis-block-reduce; test corrected, rider text unchanged); the B266
lane's residue repaired en route and disclosed: 2 stale rust_perf count pins + 1 doc_markdown.
