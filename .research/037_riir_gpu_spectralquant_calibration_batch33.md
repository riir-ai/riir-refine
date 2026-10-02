# Research 037 — riir-gpu spectralquant calibration cluster (Batch 33)

**Date:** 2026-08-15
**Batch:** 33 (kernel_opt domain)
**Sources:** `riir-gpu/src/spectralquant/{calibration,rotation,qjl}.rs` (1,164 LOC) + the WGSL kernels they drive (`covariance.wgsl`, `covariance_reduce.wgsl`, `eigenbasis_rotation.wgsl`, `selective_qjl.wgsl`) + the sole consumer `benches/spectralquant_bench.rs`
**Corpus:** 251 → **258 rules** (A 44, B 44, C 27, D 40, E 16, F 10, G 19, H 10, I 4, J 5, K 4, L 8, M 9, N 14, O 4)

## 1. Why this cluster

Research 016 (Batch 15) mined `spectralquant/attention.rs` + `dequant.rs` and
explicitly deferred `calibration.rs` + `rotation.rs` + `qjl.rs` as "the
calibration path remains unmined". This batch closes that deferral — the
spectralquant directory is now FULLY MINED. The cluster is **bench-only
today** (grep-verified: the sole consumer is `benches/spectralquant_bench.rs`;
zero production callers), which means (a) hazards found here are landmines
for the future `calibrate()` wiring, not live bugs, and (b) rules mined here
describe the cluster's shipped patterns, not hot-path wins.

## 2. The 7 rules landed

| # | rule_id | cat | concept |
|---|---------|-----|---------|
| 1 | `grid-over-output-thread-private-sweep` | E | grid over the OUTPUT matrix; the long `seq_len` axis is swept thread-private in a register accumulator, one store per element — zero cross-thread reduction, no partials buffer, no reduce kernel (the dual of split-K) |
| 2 | `buffer-set-struct-two-tier-dispatch` | D | all buffers a kernel family needs hoisted into ONE struct bound to the config; the preferred entry only WRITES uniform content, the raw loose-buffer entry self-allocates per call (explicitly secondary) |
| 3 | `slot-offset-partial-buffer-write` | A | update one `(pos, head)` slot via a computed byte offset mirroring the kernel's base index — `d_h`×4 bytes over the bus, not the whole tensor |
| 4 | `immutable-operand-upload-at-set-creation` | A | V^T / codebooks / sign-matrix uploaded ONCE inside the buffer-set constructor; mutable buffers created empty; the dispatch path never sees operand bytes again |
| 5 | `selective-projection-effective-rank-head` | B | after an energy-concentrating transform, the projection operator runs on the top `d_eff` coords only; operator + FLOPs scale with effective rank (~6), not ambient dim (128) — ~21× on both |
| 6 | `derived-scalar-host-computed-in-uniform` | N | config-derived scalars computed once on the host + shipped in the uniform: CPU reference and kernel consume bit-identical f32 by construction (WGSL sqrt is 2.5-ULP, not correctly rounded) |
| 7 | `rademacher-signs-over-gaussian-projection` | B | ±1 entries: one coin flip per entry, ±1.0 exact in f32 so every product is exact; conditional-negate strength reduction licensed but unrealized in the source kernel (honest caveat in the rule) |

## 3. The seed REJECTED on verification (the load-bearing call of this batch)

My pre-mining candidate `two-stage-tile-accumulate-then-reduce` (stage-1
kernel accumulates per-tile partials, stage-2 reduce kernel sums them) is
**NOT what the shipped code does**, and mining it would have encoded a bug
as wisdom:

- `covariance.wgsl` L53-74 sweeps **all** `seq_len` positions in-register
  and L78 writes `output_cov[global_row * d_h + global_col] = accum;` —
  **no tile index in the output offset**. The producer never writes
  per-tile partials.
- `GpuCovarianceBuffers::new` sizes `output_cov` as `[n_tiles * d_h * d_h]`
  and `covariance_reduce.wgsl` L33-35 sums `partials[t * tile_stride +
  out_idx]` over `n_tiles` — for `n_tiles > 1`, tiles 1..n are **never
  written**, so the reduce stage sums uninitialized GPU memory. Silent
  numerical corruption (Issue 686 H1).
- The only consumer pins `n_tiles = 1` (`spectralquant_bench.rs` L166:
  `let n_tiles = 1; // single-pass for d_h=128`) and **never dispatches the
  reduce stage at all**.

The *sound* half of the cluster — what the kernel actually does — is mined
as rule 1 (`grid-over-output-thread-private-sweep`). The incoherent
scaffolding around it (the `n_tiles` parameter, the reduce kernel, the
"multi-pass accumulation" docs) is filed as Issue 686 H1.

## 4. Bug yield — riir-ai Issue 686 (filed this batch)

- **H1 (P1, silent numerical corruption):** multi-tile covariance path
  incoherent — producer writes no tile partials; reduce sums uninitialized
  memory for `n_tiles > 1`; `calibrate()`'s TODO marks the reduce step
  "(DONE)" — semantically false.
- **H2 (P1/P2, unwired uniform field):** `layer_offset` is dead in
  `eigenbasis_rotation.wgsl` (kernel reads only `seq_len`/`d_h`/`n_heads`/
  `total_dim`; `base = pos * total_dim + head * d_h` never adds it) —
  multi-layer rotation silently reads layer 0. Plus a byte-vs-element doc
  contradiction between the field doc and the constructor doc.
- **H3-H15 (P3/P4):** `total_layers` dead in `covariance.wgsl`; per-dispatch
  bind-group creation in all four dispatchers (missed
  `prebuilt-bind-groups` application — the buffer combos are fixed struct
  fields); `create_uniform_buffer` doc-lie ("initialized with raw bytes" —
  it only sizes); host/kernel stride contract duplicated with no shared
  constant + no validation in `upload_input`; 2 `workgroupBarrier()`s per
  sequence position in `covariance.wgsl` for 16+16-float smem staging
  (same-address global reads broadcast anyway — likely a net loss);
  symmetric-matrix full compute (triangle+mirror would halve); dead
  `let _d_h`; unreachable bounds guard; `todo!()` doc wording; a
  tautological test; latent shared-uniform ordering hazard on
  multi-dispatch encoders; non-compiling + disfavored-path doc example;
  dead raw-dispatch path (zero callers, attractive nuisance).

## 5. Candidates dropped as duplicates/thin (11)

- per-field `offset_of!` layout-pin tests → **completion** of
  `pad-uniform-pod-to-min-binding-size` (Batch 30), not a new rule
- flat-1D modulo decode of `(pos, head, out_dim)` → covered by
  `derive-params-from-global-invocation-id` + `token-flattened-elementwise-modulo-shared-weights`
- sign-matrix bitpacking → correctly rejected per the
  `bool-fields-over-bitset-hot-reads` regime boundary (192 floats = 768 B,
  cache-resident; bytes streamed do not dominate reads)
- smem preload of the d_eff-vector per (pos,head) → same-address warp reads
  broadcast in one transaction; premature
- sign-matrix row-major layout matching per-thread rows → adjacent to
  `block-contiguous-weight-layout`
- axis ordering for coalesced output stores → textbook, implicit in the
  Batch 28 flatten rule
- immutable sign matrix uploaded at creation → the *placement* half became
  rule 4; the *mechanism* half is `mapped-at-creation-init-upload`
- bounds guard + `div_ceil` dispatch → textbook
- `make_storage` closure dedup of buffer creation → thin code hygiene
- `max_bits` derived via `iter().max()` → trivial
- hierarchical `format!` debug labels → cold-path; the
  `debug-only-gpu-label-macro` rule covers the release-mode concern

## 6. Method notes (standing)

- **Two parallel subagents** (calibration+rotation / qjl+bench) with the
  dedup matrix provided up-front; both quoted verbatim — zero quote drift
  on my line-by-line re-verification of every quoted line against the
  real files (including all four WGSL kernels — the kernel side was
  load-bearing for the seed rejection).
- **The subagent verification protocol earned its keep again**: subagent 1
  read the WGSL kernels on its own initiative and overturned my seed
  candidate with concrete line evidence, which I then re-verified
  independently before accepting the rejection.
- **One-shot Python generator** (run → verify → delete): clean first
  compile, 258/258 entries, 0 orphan `rule_id` lines on the standing scan.
  One anchor fix needed pre-run (a `///` vs `//!` doc-comment prefix —
  the assert-before-write design caught it with zero repo pollution).
- 8 clippy doc warnings fixed pre-commit (7 doc_markdown backticks in the
  `//!` header block + 1 accidental `///` source-quote prefix).
- Drive-by accuracy fix: the stale `// M: ... (7 rules)` category comment
  corrected to 9 (actual count since Batch 31/32a; the test pin was
  already correct — comment-only drift).

## 7. GOAT

- **G1:** `batch33_pseudocode_parses` 7/7 (all ref_shape + opt_pattern
  syn-parse) + `batch33_rules_present` + `batch33_concepts_covered` (3 new
  tests).
- **G2:** 315 kernel_opt (was 312) + 201 default lib tests PASS.
- **G3:** clippy 0 warnings in both feature states, `--lib` and
  `--all-targets` alike.

## 8. Progress

riir-gpu: **~72 of ~200 files mined** (spectralquant/ now FULLY MINED:
Batch 15 attention+dequant, Batch 33 calibration+rotation+qjl). Corpus:
**258 rules**. Next candidates: `gemma*_train/` + `gemma2_d2f*` +
`gemma4_q4k_train` (the Gemma training residuals), `kimi_k3_gpu_*` (4
files), `weaver_gpu*` (4 files), `kvarn/`, `hypernet/`, `maxsim/`,
`spectral/`. `ternary_deltanet_gpu_forward.rs` + `_cudarc.rs` still
BLOCKED on sibling WIP.
