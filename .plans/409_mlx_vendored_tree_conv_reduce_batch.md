# Plan 409 — Batch 410: slice (3) the vendored ml-explore/mlx tree — the 2D-conv + col-reduce clusters at the mlx-serve submodule pin

**Status:** LANDED — 2026-11-24, session zed-amo-i410@amo (Research 443; claim `af5f66fd` closed by the landing; the six rules + five faces in entries_gpu_166, gates green, fixtures 7/7 pre-entry at debug AND -O).

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

- [x] T0 — re-verify the submodule pin at the current mlx-serve tip (`2201ec9` → `c69303b0`) and the
      clone at the pin; LICENSE re-check (MIT); count the upstream commits past the pin (64, recorded,
      unswept); per-file line counts (recon numbers verified)
- [x] T1 — dedup greps at 3+ vocabularies over `src/domains/kernel_opt/` + `src/domains/rust_perf/`
      (conv/winograd/depthwise/im2col/implicit-gemm; reduce/2pass/col-reduce/fixed-partition/tier;
      bounds/mask/hoist; strided-view; capacity/budget) — zero owners on the six landed axes; re-run at
      landing HEAD (the batch re-greps at the live corpus during authoring)
- [x] T2 — two parallel reader subagents over the two clusters' remaining surfaces; coordinator
      re-verified every quoted line at the clone (11 verification reads; reader A's commutative-flag lead
      dropped pre-authoring — no such flag exists)
- [x] T3 — author corpus entries (`entries_gpu_166.rs`, 6 rules) + 5 count-neutral faces on their OWNERS;
      per-entry adjudications vs the named kin, in-rule both directions; suggestion-only NotBenchable
      annotations with the domain reason
- [x] T4 — fixtures `b410_fixtures` standalone-compiled AND executed pre-entry (debug AND -O, 7/7); the
      gate caught FOUR fixture defects (the R2 shape-gate inputs, the R1 scalar closed-form index order,
      the R4 tamper predicate direction, the R2 tamper budget arithmetic) + the B402 raw-string class
      (3 strings with embedded quotes) — all fixed pre-entry
- [x] T5 — gates at the isolated CARGO_TARGET_DIR (/tmp/b410_tgt): default lib 3980/0; kernel+latent
      4127/0; retrieval floors BOTH lanes 30/0 HELD no re-pin; clippy -D × 3 postures clean (11
      mechanicals healed via cargo clippy --fix — the stale-binary fallback, disclosed); fmt gate PASSED;
      numbering gate PASSED; pins_manifest agrees (kernel_opt.rules 1121); count web one-landing (router
      1121 + category table B134/D271/N104 + health_views kernel 1121/total 1791 + README + AGENTS +
      kernel_opt.md + the re-pin trail); score-bench/freeze NOT owed (zero clippy_lints delta); release-set
      binary NOT owed (corpus-only suggestion-only delta — the B402/B409 disclosure; the binary is already
      stale at e4b2dd31, the rebuild folds into the next maintained-binary lane)
- [x] T6 — Research 443 batch record; snapshot claim → LANDED reshape; commit + push; teardown the clone
