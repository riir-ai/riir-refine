# Plan 345 — Batch 336: the walk-23 B+ pair kernel_opt distill (loopvq + vllm-rlt)

**Status:** IN PROGRESS — claimed 2026-11-05 by shikuwa-4090-w23pair (claim `7fa4be48` + LF-normalize `74bafecf`, pushed pre-work; expiry 2026-11-07T13:40:47+00:00). All tasks unchecked at claim — checked as they complete. — claimed 2026-11-05 by shikuwa-4090-w23pair (claim `7fa4be48` + LF-normalize `74bafecf`, pushed pre-work; expiry 2026-11-07T13:40:47+00:00)

Two-source batch (the B320–322 multi-source precedent): the walk-23 B+ pair repo scans
(the cycle's claimed queue head) completed with YES verdicts on BOTH sources; this batch
converts the strongest set. Scout reports (two parallel subagents, 2026-11-05) are the
candidate basis; every quote is re-verified at the pins by the batch lane (T0) before
entry.

## Sources

1. `mscheong01/loopvq` @ `38d5eff3c11379b24dbd9bca453bdfcedc5762b2` (Apache-2.0; HEAD
   2026-11-04) — the arXiv 2611.05304 LoopVQ code release (SGLang fork @ `c554dc5c` +
   TaSQ provenance, NOTICE-documented; the looped-model KV-serving family's first
   source). Clone: `.raw/loopvq`.
2. `InferMatrixLab/vllm-rlt` @ `444638beca89e95af6efb34a39fd3edf01c4020b` (Apache-2.0;
   HEAD 2026-11-05, the paper's ExactLoop techniques landing in main; the paper's BASE
   pin is `66920da` — provenance, never the mining pin). Clone: `.raw/vllm-rlt`.

## Landing shape (scout-ranked, T0-verified)

Five whole rules in `entries_gpu_149.rs` + one count-neutral rider face:

- [ ] R1 `chunked-argmax-strict-gt-tie-identical` (loopvq; **the batch GOAT lead**,
      BENCHABLE) — chunking a materialize-then-argmax scoring loop to bound a spilling
      intermediate keeps per-entry ops identical and the cross-chunk update strict `>`,
      so the full-reduction first-index tie rule holds bit-identically (88%-of-decode-
      time spill provenance, `vq_codebook.py:887-911`). Category B (kin gpu_03/gpu_20).
- [ ] R2 `repeat-axis-aliased-index-arena` (loopvq; BENCHABLE) — when per-repeat-
      instance values of one base computation are stored linearly and a joint code
      covers several repeats, ONE arena row set per code is aliased by the Tc slots
      through the existing indirection: consumers unchanged, index bytes at the
      redundancy-folded size, storage counted once (`loopvq_pack.py:3-47`,
      `unified_kv_pool.py:679-705`, `:976`). Category A (kin gpu_01 residency family).
- [ ] R3 `load-time-decode-blind-transform-absorption` (loopvq; NotBenchable-by-regime,
      equivalence-pinned) — every quantization-space transform (axis coupling,
      sensitivity scale, weight fold) is absorbed into the per-slot DECODE table at
      load; the hot decode kernel consumes plain coordinates and ships unchanged across
      all features (`loopvq_pack.py:78-91`, `vq_codebook.py:269-282`, `:349-365`).
      Category N (kin gpu_102 hi-lo-split) — L alternative noted at authoring.
- [ ] R4 `graph-fallback-coverage-ladder` (vllm-rlt; its GOAT lead, BENCHABLE) — a
      shape-keyed captured-graph cache under heterogeneous arrivals hits an eager-
      fallback CLIFF (16 entries: 77-93% of warmed speculative calls fall back); the
      landed remedy is DERIVED COVERAGE: bucket ladder from reachable row counts,
      pre-captured before serving, round-UP selection, padding rows carried through
      capture and skipped at execution via a device-side active-row mask, fallbacks
      counted loud (`worker/cuda_graph.py:22-63`, `:335-392`, `kernels/graph_kv.py:1-42`).
      gpu_57 owns the LRU-amortization half only — cite, never merge. Category D.
- [ ] R5 `i64-index-promote-before-stride-multiply` (vllm-rlt; NotBenchable —
      correctness class) — a byte address `index * stride` inside a kernel overflows
      32-bit index arithmetic on >2^31-byte pools; the promotion must happen on the
      INDEX BEFORE the multiply (`kernels/graph_kv.py:39-42` "Promote before
      multiplying: large cache offsets exceed int32", `routing.py:40-41`, `:172`).
      Host-side sibling: Batch 195 `strided-view-offset-fit32-launch-proof` (prove-or-
      promote = the two postures of one width law). Category O (kin in O).
- [ ] F1 rider face (count-neutral) — dtype-width packed-register cliff onto
      `inner-loop-select-division-shift-mask-vgpr-pressure`: packed vector products are
      free register-wise only where the packed type is NARROWER than the scalar
      register; forcing the packed form on FP32 doubled registers 28→34 and dropped
      resident warps (`silu_and_mul.cu:63-65`, docs/kernels.md).

## Tasks

- [ ] Claim pushed pre-work (`7fa4be48`) + LF-normalize (`74bafecf`)
- [ ] Code homes found (paper HTML + ls-remote), clones at the pins, license + NOTICE recorded
- [ ] Two parallel scout subagents (read-only reports; 3+-vocab dedup pre-checks)
- [ ] T0: every quote re-verified at the pins by the batch lane
- [ ] Fixtures authored as standalone Rust, compiled AND executed green BEFORE entry
- [ ] `entries_gpu_149.rs` written (5 rules, quotes + dedup + regime + kinship + P113)
- [ ] Count-pin web: corpus/mod.rs (mod decl + extend + KERNEL_CATEGORIES category lists
      + header counts), router.rs total 1056→1061, goat_g1.rs (expected_corpus_files
      1..=149, len pin 149→150, per-file membership list, + the new file in BOTH census
      paths — the B335 coverage-debt lesson applied in the same landing)
- [ ] batch336 needles + pseudocode-parse tests in corpus/tests.rs
- [ ] Gates at the isolated `/tmp/b336_target`: kernel_opt lib, default lib, clippy -D
      ×4 postures (default / no-default / kernel_opt / --all-features), batch336
      needles+parse, retrieval floors both lanes (kernel+latent + default+latent), G1
      fixture harness
- [ ] `pins_manifest --write` (kernel_opt 1061) + prose sites (AGENTS.md §Role,
      README.md table, .docs/11_domains/kernel_opt.md count + Batch 336 entry)
- [ ] Verdict ping-pong (subagent reviewer) BEFORE the landing commit
- [ ] Research note `.research/374` (the pair distill record; staged-defer table)
- [ ] Snapshot landing row (`.distill/001`) — claim discharged; `.raw/` clones REMOVED
- [ ] Commit `feat: batch 336: ...` + push (NO release-set rebuild — kernel_opt opt-in)

## Staged defers (- [-]) — the B333 T3 precedent (authoring budget → leads)

- [-] loopvq C2 `in-kernel-strided-steps-load-replaces-stack-copy` (BENCHABLE, 127ms
      provenance) — staged for the next kernel_opt lane
- [-] loopvq C3 `broadcast-mul-reduce-to-dot-zero-padded-contraction` (BENCHABLE,
      drift-census honesty) — staged
- [-] loopvq C4 `shape-static-trash-row-write-routing` (BENCHABLE; skip-half kin gpu_97)
      — staged
- [-] loopvq C7 `fp16-head-remainder-two-mma-fp32-grade-scores` (BENCHABLE-lite; kin
      gpu_102 storage-face) — staged
- [-] loopvq C8 `request-group-shared-position-table-loads` (BENCHABLE-lite; kin
      gpu_104 share-genus) — staged
- [-] vllm-rlt C2 `exit-plane-fanout-fused-finalize` (BENCHABLE; cleanest launch-count
      fixture) — staged
- [-] loopvq C10 view-pins-backing-buffer — DOMAIN ROUTING call (rust_perf vs
      kernel_opt) at the next batch
- [-] vllm-rlt not-at-pin re-scan triggers: live-slot plane packing (1.88x) +
      position-keyed noise (source refuses at this pin) — re-scan if a later pin lands
      them
- [-] lane-intel: commit-tail-rounds law (vllm-rlt pd/config.py:10-15 "In-flight bytes
      are not a buffer") → plan-623 multi-host lane, never a corpus slot
