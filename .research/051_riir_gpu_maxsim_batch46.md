# Research 051 — riir-gpu MaxSim cluster (Batch 46)

**Date:** 2026-08-15 · **Verdict:** 3 rules landed (corpus 337 → 340, A 79 / D 60) + 15 hazards filed as riir-ai Issue 700.

## Targets

- `crates/riir-gpu/src/maxsim/dispatch.rs` (449) — size-gated CPU/GPU MaxSim late-interaction scoring (Plan 085). CPU fallback = scalar triple loop; GPU path = per-call buffers → compute → download; dispatch rule `ld * batch < 256` → CPU else GPU.
- `crates/riir-gpu/src/maxsim/spectralquant.rs` (581) — fused SpectralQuant variable-bit KV dequantize + MaxSim scoring in one GPU pass (Plan 085 T5).
- `crates/riir-gpu/src/kernels/maxsim_score.wgsl` + `spectralquant_maxsim.wgsl` — both kernels read.
- `crates/riir-gpu/tests/bench_maxsim_gpu_dispatch.rs` — the empirical crossover bench (Benchmark 002).

Caller status (severity context): **zero production callers** — `MaxSimScorer` + `GpuSpectralQuantMaxSimScorer` appear only in definitions/tests/re-exports; feature `maxsim` is default-off. All hazards are latent-until-adopted; correctness IS test-pinned (GPU-vs-CPU sweep 1e-3) unlike the Issue-691 unpinned-math case.

## Rules landed (3)

1. **`size-gate-covers-every-cost-factor`** (D) — a CPU/GPU size gate must gate on the full work term (`lq * ld * dim * batch`), not a partial product (`ld * batch`); the pinned constant bakes in BOTH implementations' current overheads (SIMD-izing CPU or removing the GPU per-call alloc floor each silently move the crossover); bench citation must name hardware AND shape, crossover bench re-runs when either side changes. Two in-tree instances: dispatch.rs L215-216 + gpu_m2p.rs L307/L363. Anchored: Benchmark 002 M3 Max (GPU floor ≈ 1.6-1.8 ms; CPU ≈ 8.5 µs/unit at lq=8×dim=64; 73.86× at top; interpolated true crossover ≈ 220 at the benched shape but moves ~27…~6,900 across realistic shapes).
2. **`declared-max-array-taxed-per-thread-under-runtime-bound`** (A) — WGSL function-scope array sized to a hard max (`array<f32, 256>` = 1 KB/thread) but runtime-bounded (`d_eff`, default 6) pays the DECLARED max in indexable thread-local per thread (no unroll, no register promotion); fix = pipeline override constants (one pipeline per bound class); the tax multiplies with axis-splitting (more resident threads). Zero corpus matches across 6 vocabulary variants.
3. **`shared-operand-side-pair-batch-index-decomposition`** (A) — a `[batch × lq × d_h]` pair API bakes per-pair replication of the shared query side into the batch geometry (canonical ColBERT = 1 query vs N docs → N× host replication, N× upload, per-thread re-transform); fix = decompose the launch index `(query_idx, doc_idx)` + per-pair doc offsets — share-by-indexing, not replication. Honest caveat: no live caller exercises the shared-query shape today; grounded in the API's own ColBERT framing + the CPU substrate twin `maxsim_score_packed` (offsets + pair query-ids).

## Falsifications (14)

- Per-call `MaxSimGpuBuffers` churn — documented trade-off (comment L100-103) + covered by `mutable-input-scratch-residency-write-per-call` (B43), `immutable-operand-upload-at-set-creation` (B33), `staging-reuse-grow-only-map-prefix` (B31); zero callers → hazard not rule.
- debug_assert-only `validate()` — instance of the B29/B43 class, neither form present → hazard.
- One-thread-per-pair serial sweep — `grid-over-output-thread-private-sweep` (B33) at large batch; `row-dot-split-inner-axis-block-reduce` (B44) regime at threshold band — no measured pathology to distill (GPU already wins there).
- Query-row reload per j — `hoist-bit-address-out-of-packed-row-sweep` (B44) instance; runtime-length register hoist needs fixed-max arrays (B23 territory).
- NEG_INF max-init — correct, guarded `ld==0`; `mask-sentinel-matches-reduction-identity` (B35) covers.
- Doc-sharing cache amplification — falsified by layout: per-pair private doc slices, no cross-pair sharing expressible.
- Threshold-as-good-structure — ambient, not decision content.
- Per-call calibration upload (spectralquant) — exact B33 composition (`immutable-operand-upload-at-set-creation` + `buffer-set-struct-two-tier-dispatch`); `bitstream_offset` is a uniform scalar, not operand bytes → hazard H6.
- d_eff unpack-bounds gap — FALSIFIED: kernel unpack loop bound IS `d_eff` (wgsl L130); tail dims never unpacked; only the `total_bits` prefix sum walks d_h (required to stride).
- storage_read twin closures — thin rust_perf generic guidance, not kernel wisdom.
- bitstream_offset indirection — plain uniform field, no decision content; the units mismatch survives as H4.
- Zero-fill edge returns — correct code (host early-returns + kernel guard agree).
- Pipelining sibling dispatches — B36 measured non-application on Metal (Bench 437) noted; no proposal.
- empirical-threshold structure — folded into rule 1's calibration-lifecycle half.

## Hazards → riir-ai Issue 700 (15)

Lane A (dispatch.rs): H1 release-OOB window (debug_assert-only + unchecked runtime-indexed storage reads); H2 substrate-first violation — "no dependency on katgpt-rs" premise FALSE (katgpt-core is a non-optional dep L76; `maxsim = ["katgpt-core/maxsim"]` one feature-forward away from the SIMD twin + `maxsim_score_packed` ragged/shared-docs API); H3 threshold calibrated against two known-suboptimal implementations (compounds H2 + H5); H4 doc-fiction — the advertised 800K-pair example is ~133 GB memory-infeasible; H5 `download_f32_reuse` ships unused 55 lines away (the ~1.8 ms floor the constant encodes is largely per-call alloc + staging churn the crate already has substrate to remove); H6 the bench's own crossover finder warns against the shipped constant every run (granularity artifact; slope interpolation actually vindicates 256 — stated nowhere); H7 dead code (zero callers); H8 wrong-location doc reference in Plan 104 + stale `#[allow(dead_code)]` in gpu_m2p.

Lane B (spectralquant.rs): H1 `validate()` never takes/checks `compressed_k` (nor `ld`, `bitstream_offset` bounds) — too-short bitstream = silent garbage in every build mode; H2 the documented-but-unenforced "+1 padding" one-word OOB read executes in tests today (benign on common backends); H3 `d_eff ≤ 256` and `d_eff ≤ d_h` unchecked ANYWHERE (not even debug_assert); H4 word-vs-byte `bitstream_offset` doc-lie (WGSL comment vs kernel indexing); H5 unpinned dim-ordering contract (kernel scores dims 0..d_eff assuming top-eigenvalue-first ordering — nothing asserts/documents it); H6 regression of the B33-mined positive two files away; H7 `1u << max_bits` shift-overflow edge (note only).

## GOAT

- G1: `batch46_pseudocode_parses` PASS (3/3 rules syn-parse both shapes).
- G2: 354 kernel_heal (351 + 3) + 201 default lib tests PASS.
- G3: clippy 0 in both feature states (15 doc warnings fixed pre-commit — the B37 line-initial `+` family RECURRED plus the CamelCase/underscore backtick family: `MaxSim`/`SpectralQuant`/`gpu_m2p`/`d_eff`/`d_h`/`validate()`/`compressed_k`/`bitstream_offset`).
- G4: N/A (corpus data only).

## Generator lessons (2 new)

1. **Python evaluation-order truncation (NEW class, live-catched):** `open(path, "w").write(apply(...))` — `open()` truncates at open-time BEFORE `apply()` runs; the anchor gate then failed on the emptied file and exited, LEAVING the 0-byte file. Recovered via `git checkout HEAD --` (tree was clean pre-run). Fix: compute the full new content FIRST, then open for write. The dry-run passed because it only reads — dry-run green ≠ write-safe write-path.
2. **Line-initial `+` in generated `//!` prose (B37 recurrence):** the known doc_lazy_continuation root cause re-landed via fresh generator prose — two wrapped lines happened to start with `+`. Pre-commit grep for `^//!   \+ ` in generated header blocks should join the standing pre-compile scan.

## Follow-ups

- gpu_m2p.rs per-column dispatch-inside-a-loop (L298-319) = likely B40 `expert-loop-two-phase-split-batch-across-iterations` instance — candidate for the hypernet/ second-half batch.
- The corpus.rs → corpus/ directory split (owner-approved this session) lands as a separate `refactor:` commit after this batch: `entries_popcorn.rs` (B1-13, frozen) + `entries_gpu_NN.rs` tail slices + `tests/`; categories stay in the single KERNEL_CATEGORIES table.
