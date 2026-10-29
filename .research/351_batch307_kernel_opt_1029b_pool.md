# Research 351 — Batch 307: the 10-29b staged kernel_opt pool

**Status:** RECORD — Batch 307 landed from this note (Plan 312); the verdict record is the
10-29b staged addendum in `.distill/001_mining_queue_snapshot.md` (shikuwa-4090-leads1029b,
2026-10-29T12:1xZ); quotes re-verified at the fetched .patch pins (plan 312 T0, 2026-10-29).

## Provenance

| repo | licence | commits mined | pin form |
|---|---|---|---|
| vllm-project/vllm (main) | Apache-2.0 | `fa1cf1d` #64419, `9c7e8ff` #64345, `8126ebd` #64482 | .patch fetches |
| ml-explore/mlx (main) | MIT | `9e2e8e2` #4791 | .patch fetch |
| ggml-org/llama.cpp (master) | MIT | `6d19453` #31075 | .patch fetch |
| trymirai/uzu (main) | MIT | `54ce0e3` #1030 | .patch fetch |

Patches at `.raw/b307/` (the uzu PR carried two commits; the mined atom is in 54ce0e3's
diff), REMOVED at close per the ephemeral-clone law.

## The landed set — 2 own rules (corpus 1020 → 1022) + 4 count-neutral rider faces

| # | rule_id | cat | source | one-line law |
|---|---|---|---|---|
| 1 | write-coverage-proven-uninitialized-alloc | A | vllm `fa1cf1d` + `9c7e8ff` | skip a zero-fill only when the write coverage is enumerable AND the never-written region is provably unread — enumerate the coverage, name the gap, classify its readers or pin the producer ordering; the FLA replay-zeros pole and the gpu_138 write-every-output complement bind the opposite duties |
| 2 | gate-rank-dispatch-function-constant-variant | D | mlx `9e2e8e2` | an auxiliary operand whose RANK varies (scalar vs per-channel) selects an indexing VARIANT at dispatch — function constant + variant-suffixed kernel + the matching chunk geometry; one stride math for both ranks is silent wrong indexing |

Riders (count-neutral): the TP-class + static_k face on `swept-shape-bucket-launch-tables`
(gpu_17, vllm #64482 — device × dtype × shape × PARALLELISM-CLASS keying, static_k pins the
(N,K) pair, the hand-tuned rows carry the sweep-harness obligation) + the
hardware-arithmetic-replaces-LUT face on `lut-free-bit-spread-vadd4-unpack` (gpu_33,
llama.cpp #31075a — the fp16 bit trick decodes MXFP4 E2M1, aligned u32 nibble words, no
threadgroup memory) + the ≤256-thread geometry face on
`concat-row-space-thread-split-not-singleton-outer-dim` (gpu_137, llama.cpp #31075b — the
grid re-derives ceil(n/nth), nth_max QUERIED from the pipeline) + the non-distributive
cap-once-at-the-drain face on `contraction-partials-sum-once-at-drain` (gpu_134, uzu #1030 —
cap-then-sum ≠ sum-then-cap, WRONG VALUES pre-fix; ordered drain placement
partials→sum→bias→cap; correctness-carrying).

## Dedup evidence (slot re-verification, 3+ vocabularies per atom)

- write-coverage: the entries_gpu_26 island rule owns OVERWRITE-coverage dead-compute (a
  different lifetime axis); `identity-keyed-graph-retention-stable-arena` owns zeroing-resize
  without a coverage contract; the FLA `cuda-graph-static-buffer-zero-replay` is the opposite
  pole; grep 'MaybeUninit|set_len|coverage proof' → zero owners pre-landing.
- rank-dispatch: `pipeline-cache-key-covers-every-function-constant` (gpu_35) owns the
  CACHE-KEY side; `adjacent-step-kernels-fuse-mode-specialized` (gpu_134) owns FUSION
  mode-specialization; `extent-one-dim-skips-every-layout-check-symmetrically` is the
  degenerate-rank cousin; grep 'ndim|rank variant|_vecg' → zero owners pre-landing.

## Gates (all green)

batch307 needles+parse 2/2; kernel_opt lib 3309/0; default lib 2785/0; clippy −D ×3
postures (kernel_opt / default / no-default, native 1.99.0 pin); retrieval floors HELD both
postures (28/0 at kernel_opt+latent_retrieval, 27/0 at default+latent — no decay, no re-pin);
freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild NOT owed
(kernel_opt outside the release set).

EN-ROUTE: the B305 missed cross-domain rust_perf witness repaired (275 → 278 in
`batch249_slice3_rust_perf_riders` — the batch-266/271/282/298 missed-pin class this gate's
own message documents, fifth instance; count-neutral for kernel_opt).

## Method note

The 10-29b sweep fetched the five PRs as .patch files into `.raw/b307/` (no clones needed —
single-commit reads); the uzu repo is `trymirai/uzu` (the sweep's "uzu" shorthand). The uzu
PR #1030 carries two commits at the .patch URL; the mined softcap atom sits in `54ce0e3`
(commit 1 of 2).
