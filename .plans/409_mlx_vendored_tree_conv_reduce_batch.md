# Plan 409 — Batch 410: slice (3) the vendored ml-explore/mlx tree — the 2D-conv + col-reduce clusters at the mlx-serve submodule pin

**Status:** OPEN — claimed by zed-amo-i410@amo (the amo idle loop, Protocol H fresh-passed 2026-11-24); expiry 48 h from claim push.

Queue head: Research 432's named follow-up pool item, "the vendored ml-explore/mlx tree (its own scale)" — the
B398 visit's disclosed unread submodule residue (`lib/mlx-src`, never pulled), confirmed staged by Research 436
("slice (3)"). The handoff's other staged row (the mlxfast-bonsai2-27b arena-close 113-commit walk) is STALE —
landed as Batch 402 (plan 402), tip re-verified UNMOVED at `0c7c3923` via ls-remote 2026-11-24. The vendored
submodule pin at the CURRENT mlx-serve tip `2201ec9` is **`c69303b0`** (ls-tree verified at recon).

Scope: ONE batch slice of the vendored upstream MLX tree — the two least-corpus-overlapped kernel clusters,
NOT the whole tree (which stays a multi-batch surface):

- **Cluster A — 2D conv** (`mlx/backend/metal/kernels/conv.metal` 712 lines + `mlx/backend/metal/conv.cpp`
  1815 lines): Winograd F(6,3) weight/input/output transforms done ON-GPU via 8×8 `simdgroup_matrix`
  chains — `(G·g)·Gt` per channel-block with a two-thread-element packing and the transposed
  `A×A×C×O` output layout; `depthwise_conv_2d`'s threadgroup staging tile (TGH/TGW span halo, the
  8-channel `c_per_thr` interleave, `thr_per_hw`/`hw_per_group` thread mapping, the in-bounds/zero-fill
  staged load); `naive_unfold_Nd` (im2col) valid/zero-fill row unfold; the host dispatch (conv-as-GEMM
  weight reshape + restride, group-conv shapes).
- **Cluster B — col-reduce tiers** (`reduce.metal` + `kernels/reduction/*.h` 1138 lines + `reduce.cpp`
  1074): the `col_reduce_2pass` fixed `outer_blocks = 32` partition (intermediate shaped `(32, out)`,
  per-thread `totals[n_reads]`, simd-reduce fold across BM, lane-0 strided write) + the tier dispatch
  (`small < 32` / `longcolumn stride<32 && size≥1024` / `2pass >256 && out/32<1024` / looped fallback)
  in `strided_reduce_general_dispatch`.

Excluded this batch (recorded as the residue pool): quantized/steel attention/SDPA surfaces (mined via the
mlxfast vendor copies + B404), fft/radix (niche), hadamard (FWHT family heavily mined), scan/sort (small,
partly mined), CPU backend, the upstream-post-pin delta (upstream tip is ahead of `c69303b0` — a future
watch surface, UNSWEPT here).

License: ml-explore/mlx MIT (re-verify at clone). Transcription law: Metal/C++ source → ORIGINAL Rust
fixtures, suggestion-only `NotBenchable-by-domain` (no Metal runner in the validator) unless a rule is
host-benchable.

- [ ] T0 — re-verify the submodule pin at the current mlx-serve tip (`2201ec9` → `c69303b0`) and the
      clone at the pin; LICENSE re-check; count the upstream commits past the pin (recorded, unswept);
      per-file line counts (recon numbers re-verified at claim)
- [ ] T1 — dedup greps at 3+ vocabularies over `src/domains/kernel_opt/` + `src/domains/rust_perf/`
      (conv/winograd/depthwise/im2col/implicit-gemm; reduce/2pass/col-reduce/fixed-partition/tier),
      every hit read and ruled in-row; prior mined set named per candidate; re-run at landing HEAD
- [ ] T2 — two parallel reader subagents over the two clusters; coordinator re-verifies every quoted
      line at the clone (file:line + verbatim text)
- [ ] T3 — author corpus entries (`entries_gpu_166.rs`, kernel_opt primary; rust_perf only if a
      measurement rule emerges) + riders; per-entry adjudications vs the named kin, in-rule both
      directions; suggestion-only annotations with the domain reason
- [ ] T4 — fixtures `b410_fixtures` standalone-compiled AND executed pre-entry (debug AND -O); the
      gate's caught defects fixed before entry
- [ ] T5 — gates at the isolated CARGO_TARGET_DIR: batch parse gate, default lib, kernel_opt lib,
      clippy -D × postures, retrieval floors BOTH lanes, fmt gate (bare rustfmt on touched files),
      `pins_manifest --write`, count web one-landing; score-bench/freeze NOT owed if zero clippy_lints
      delta; release-set binary NOT owed (suggestion-only kernel delta, corpus-only — the B402/B409
      disclosure convention; the maintained binary is already stale at e4b2dd31, the rebuild folds into
      the next maintained-binary lane)
- [ ] T6 — Research 443 batch record; snapshot claim → LANDED reshape; commit `feat: batch 410: ...` +
      push; teardown the clone
