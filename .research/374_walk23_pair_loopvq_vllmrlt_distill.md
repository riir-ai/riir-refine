# Research 374 — the walk-23 B+ pair distill: loopvq (2611.05304) + vllm-rlt (2611.05150)

**Status:** CONVERTED — Batch 336 / Plan 345 landed (the pair's repo scans + conversion in one cycle; claim `7fa4be48` discharged by the landing).

## Sources

| # | repo | pin | license | what it is |
|---|---|---|---|---|
| 1 | `mscheong01/loopvq` | `38d5eff3c11379b24dbd9bca453bdfcedc5762b2` (2026-11-04) | Apache-2.0 | the arXiv 2611.05304 LoopVQ code release — an SGLang fork @ `c554dc5c` + TaSQ provenance (NOTICE-documented, all Apache-2.0) with the LoopVQ implementation: `python/sglang/srt/mem_cache/{vq_codebook.py, loopvq_pack.py, unified_kv_pool.py}`, `triton_ops/vqwide_{g,ng,v2}.py`, `models/{huginn,ouro}.py`, `calibration/` |
| 2 | `InferMatrixLab/vllm-rlt` | `444638beca89e95af6efb34a39fd3edf01c4020b` (2026-11-05) | Apache-2.0 | "vllm-rlt: a serving engine for looped language models" — the engine of arXiv 2611.05150 ("What Sets the Serving Speed of Looped Language Models?", the ExactLoop paper). The paper's techniques ARE current main (PR #150 pd/transfer-budget-ucx is the HEAD merge; `csrc/activation/silu_and_mul.cu` carries the fences). The paper's BASE pin `66920da` is provenance, never the mining pin; paper-only figures (5689 kernels/token, 1.55-1.59x, 0.40x, 18.4%, 1.88x) cited as paper claims, never in-source. |

Both clones lived at `.raw/` for the session, removed at the landing commit (pins are the provenance).

## Landed (kernel_opt 1056 → 1061, entries_gpu_149)

| rule | cat | source | grade | the law |
|---|---|---|---|---|
| `chunked-argmax-strict-gt-tie-identical` | B | loopvq `vq_codebook.py:887-911` | BENCHABLE (GOAT lead) | chunk a materialize-then-argmax scoring pass (the g8 spill form ran 88% of decode GPU time) keeping per-entry ops identical + the cross-chunk update STRICT > — the full argmax's first-index tie rule holds bit-identically |
| `repeat-axis-aliased-index-arena` | A | loopvq `loopvq_pack.py:3-47`, `unified_kv_pool.py:679-705, :976` | BENCHABLE | when per-repeat-instance values are stored linearly and a joint code covers several repeats, ONE arena row set per code is aliased by the Tc slots through the existing indirection; `Lc * D/c == L * D/G`, storage counted once — the corpus's first REPEAT-AXIS owner |
| `load-time-decode-blind-transform-absorption` | N | loopvq `loopvq_pack.py:78-91`, `vq_codebook.py:272-281, :349-366` | NotBenchable-by-regime (equivalence-pinned) | every quantization-space transform (coupling, sensitivity scale, channel weight) absorbed into the per-slot DECODE table at load; the hot kernel sees plain coordinates and "never learns about loops" |
| `graph-fallback-coverage-ladder` | D | vllm-rlt `worker/cuda_graph.py:22-63, :335-392`, `kernels/graph_kv.py:1-42`, `tests/test_cuda_graph.py:76` | BENCHABLE | a shape-keyed captured-graph cache under heterogeneous arrivals hits an eager-fallback CLIFF (77-93% of warmed speculative calls fall back at 16 entries); the landed remedy is DERIVED COVERAGE: bucket ladder from reachable rows, pre-captured, round-UP selection, active-row padding skip, fallbacks counted loud |
| `i64-index-promote-before-stride-multiply` | O | vllm-rlt `kernels/graph_kv.py:40-41`, `routing.py:172` | NotBenchable (correctness) | >2^31-byte pools wrap silently in 32-bit index arithmetic; the promotion must happen on the index BEFORE the stride multiply — the device-side twin of Batch 195's host-side prove half |

+1 count-neutral face: `inner-loop-select-division-shift-mask-vgpr-pressure` += the DTYPE-WIDTH packed-register cliff (forcing `__fmul2_rn` on FP32: 28→34 registers/thread, occupancy drop on SM103 — `silu_and_mul.cu:63-65`, `docs/kernels.md`).

Product-coverage: seeds the looped/recurrent-model KV-serving family (was EMPTY; first SGLang-fork source outside the sgl-nvidia-mega lineage).

## Corroborations (record, never re-land)

- **The silu-mul rounding fence** (`silu_and_mul.cu:76` "Match F.silu(gate) * up: round the activation before multiplying" + the `__float2bfloat16_rn` Math traits) — gpu_99's law's THIRD independent implementation, after the vllm #64509 face and the prism pinned-reference face. Recorded in the batch header, never a rule.
- **"Fuse only inside graph capture"** — the paper's 0.40× eager-scope regression is gpu_07/gpu_08's capture-scope family's external instance; in-source support is the opt-in posture (CHANGELOG), the figure itself paper-only.
- **PyTorch's reduction order as a compatibility contract** — the paper's summation-order discipline (W lanes, four strided accumulators, halving tree) externalizes the corpus's reduction-order family (gpu_99 faces, gpu_140's slice-order fold) and reflex's `candle_vec_sum` mirror law (different repo, same genus).
- **The exactness ladder r0-r3** — the grade-by-exactness ship/hold gate is the lossy-surface promotion rule's external echo; recorded as doctrine corroboration.
- **Staging-gate + decline-family + identity-gating corroborations** from the loopvq scan (gpu_04's smem-staging gate, gpu_16/38/68/97/99 decline family, gpu_05 runtime-identity gate) — in the loopvq scout report, none re-landed.

## Kills (already-owned classes, do not re-land)

- Graph LRU eviction + bucket amortization → gpu_57 (verified in-source as owning ONLY fixed-bucket/LRU/copy-in amortization — the paper's "LRU eviction" half is that law's instance).
- Order-preserving key packing → gpu_20 (B107). Flag formats → gpu_30. Wait ladders → gpu_118. Walk residency → gpu_23.
- Channel-axis KV quant + eviction families → gpu_147/gpu_34/gpu_20 (the loopvq scan confirmed zero REPEAT-AXIS owner before this batch — that gap is what rule 2 fills).
- Identity-gating (gpu_05), zero-stride broadcast views (gpu_65), staging gate (gpu_04), grid-capping genus (gpu_108/117/138) — the loopvq scout's kills, all verified owned.

## Not at the pin (re-scan triggers)

- **vllm-rlt live-slot plane packing** (the paper's 1.88× dead-KV-slot fix) — pool stays uniform at `444638be` (`kv_cache_manager.py` storage_depths); re-scan if a later pin lands it.
- **Position-keyed noise** (the paper's SplitMix64 counter-based RNG making prefix-cache hits exact) — the source REFUSES it at this pin (`models/huginn.py:135-136` "Huginn test-time noise is unsupported"); re-scan on a later pin.

## Staged defers (the B333 T3 precedent — authoring budget went to the leads)

| candidate | source | grade | where staged |
|---|---|---|---|
| `in-kernel-strided-steps-load-replaces-stack-copy` | loopvq `vq_codebook.py:1081-1155` | BENCHABLE (127 ms/16k-prefill provenance) | next kernel_opt lane |
| `broadcast-mul-reduce-to-dot-zero-padded-contraction` | loopvq `vq_codebook.py:936-992, :1176-1189` | BENCHABLE (drift-census honesty: 5 disagreements in 6.3M assignments, all sub-epsilon) | next kernel_opt lane |
| `shape-static-trash-row-write-routing` | loopvq `unified_kv_pool.py:664-668, :1248-1261`, `gpu_flush_int2.py:404-431` | BENCHABLE (skip half kin gpu_97; the trash-row ROUTE half unowned) | next kernel_opt lane |
| `fp16-head-remainder-two-mma-fp32-grade-scores` | loopvq `vq_codebook.py:1015-1056` | BENCHABLE-lite (kin gpu_102 storage-face; THIS the consumption face) | next kernel_opt lane |
| `request-group-shared-position-table-loads` | loopvq `vqwide_g.py:1-26`, `vqwide_ng.py:198-209, :567-582` | BENCHABLE-lite (kin gpu_104 share-genus; the no-exchange RoPE re-association vqwide_v2:171-176) | next kernel_opt lane |
| `exit-plane-fanout-fused-finalize` | vllm-rlt `kernels/routing.py:158-172`, `kv_cache_manager.py:763-770` | BENCHABLE (cleanest launch-count fixture: N×layers indexed copies vs one (row, target-plane)-grid kernel) | next kernel_opt lane |
| view-pins-backing-buffer compact-copy | loopvq `unified_kv_pool.py:1442-1446` | DOMAIN ROUTING call (rust_perf residency vs kernel_opt) | owner call at the next batch |

## Lane-intel (recorded, never acted here)

- **The commit-tail-rounds law** (`pd/config.py:10-15` "KV chunks are submitted, and their completion observed, only between engine steps, so a request whose KV exceeds the in-flight budget waits extra steps after its prefill (the commit tail)... In-flight bytes are not a buffer") — the plan-623 multi-host lane's sibling law (recv rings ≤ 3 MB or runtime-probed; this is the SENDER-side budget twin). Never a corpus slot.
- **UCX transport posture** (`transport.py:104-111` `{"ucx_error_handling_mode": "none"}` re-enables intra-node RMA because peer error handling evicts shared-memory lanes) — plan-623 intel.
- **Descriptor machinery** (`transport.py` `coalesce_segments` "Merging never moves a byte", `kv_segments` "unused tail bytes are never sent or advertised as initialized", `partition_segments`) — plan-623 lane.
- **C6 decode-blind absorption → riir-infer quant lanes** — the design template for any future lossy state-half scheme (the Issue-750-T3 successor): complexity lives on the write/load path, the serving kernel stays dumb.
- **R5's WGSL hazard** — wgpu index math is i32-based; Bonsai-2-27B PQ2 pools (~5 GB) exceed 2^31 bytes — live hazard class for our wgpu/CUDA lanes.
- **League read**: vllm-rlt is a serving-engine family (looped-LM serving); the paper's dispatch-tax census (5689 kernels/token) corroborates the launch-count-vs-real-work discipline our decode lane already prices. No league cell (different model class).

## GOAT / gate record

- Fixtures: `corpus/fixtures/b336_fixtures.rs` — 5/5 standalone `rustc --edition 2021 --test` green BEFORE entry; the built entries' 10 arms (5 ref + 5 opt) extracted + compiled + executed green by the arm checker (10/10 PASS). Reviewer independently re-ran the fixture file: 5/5 green.
- Gates at the isolated `/tmp/b336_target`: kernel_opt lib 3412/0; default lib 2856/0; clippy -D ×4 postures (default / no-default / kernel_opt / all-features) ZERO; retrieval floors HELD both lanes (kernel+latent 28/0 at 271s; default+latent 27/0 at 64s — no re-pin); G1 harness + census green (loop 1..=149, len pin 150, per-file membership — all three legs current at landing, the B335 lesson applied).
- pins_manifest --write (kernel_opt 1061) + AGENTS.md / README.md / .docs/11_domains/kernel_opt.md prose agree.
- Not owed: score-bench (P30 kernel_opt-only delta), freeze tables (zero clippy_lints delta), release-set binary rebuild (kernel_opt opt-in).
- Verdict ping-pong: round 1 REVISE (two wrong-file cites — R3 `vq_codebook.py:18-19` → actually loopvq_pack.py:26-28; R5 `routing.py:40-41` → actually graph_kv.py:40-41 — plus the plan-lease shape and two cosmetic labels), all fixed with re-verified quotes, round 2 AGREE (native subagent reviewer; the two non-blocking notes — the off-by-one range tail and the duplicated claim prose — also applied).

## Session

shikuwa-4090-w23pair / 4090 — claim `7fa4be48` + LF-normalize `74bafecf` + plan `8a161dee` pushed pre-work; zero GPU throughout; cargo at the gates only in the isolated `/tmp/b336_target`; two scout subagents (read-only) + one verdict reviewer (read-only, fixtures re-run independently); sibling i208pregate commit `99f809cc` rebased over mid-cycle without conflict.
