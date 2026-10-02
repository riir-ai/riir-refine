# Plan 103 — Batch 123: the ds4 upstream-surge distill (GLM/Metal SSD-expert caching + ROCm DSpark + agent tooling, 48 commits)

**Status:** RECORD 2026-09-07 — Batch 123 landed: kernel_opt +3 (552 → 555, A 117 / H 26) + rust_perf io +2 (108 → 110), all gates green (lib 1165/0, kernel_opt 1478/0/1i, kernel_opt+latent 1595/0/2i, retrieval floors byte-identical, score-bench FLAT 96.2/1.9, clippy 0 ×3 states), Research 146. 4 candidates recorded-not-landed with reasons (in-rule ledger).

## Source + pins

antirez/ds4 (MIT) main `9ab705347c1775e7599ede7eb81a6255ec7dccb5` → `c0a6119f363ef82125877142f13fb3fe491cba14` — 48 commits, 41 files, +6118/−1100. Pin re-verified via ls-remote at clone time (2026-09-07 07:08). Clone: `.raw/ds4` (gitignored; removed at landing).

## Lanes (surveyed by commit + diffstat)

1. **Metal SSD-expert caching + GLM streaming prefill** (~12 commits; `ds4_metal.m` ±321, `ds4.c` ±552, `metal/moe.metal`, `ds4_ssd.c/.h`) — the kernel_opt heart: IQ2 expert cache aging "from the first routed layer", cache adaptation beyond built-in preload ranking, safe-working-set expert caching, router readback reuse during IQ2 expert streaming, cached-IQ2-expert reuse for small prefills, attention scratch alignment for arbitrary context lengths, managed-storage notification removal from shared buffers, streaming-weight residency within runtime budget, parallel prefill reads without serial read-ahead, first-streaming-prefill overlap. ALSO the SSD-gains doc (SSD_STREAMING.md ±34) + `test_metal_ssd_experts.c` +239 / `test_ssd_cache.c` +33.
2. **ROCm gfx1151 DSpark** (~8 commits; `rocm/ds4_rocm_hipblaslt.cuh` ±184, `attention_launch.cuh`, `matmul.cuh`) — version-matched Lt prefill plans, verifier attention projection, IQ2 prefill input staging, DSpark target-feature alignment + rolling window, aligned Q8/expert vector kernels for small batches, partial-tile QA cleanliness.
3. **CUDA mmq** (`cuda/mmq/mmq.cuh` +62, `test_iq2_aligned_entry.cu` ±141) — IQ2 aligned entry.
4. **Agent/server tool-calling robustness** (~10 commits; `ds4_agent.c` ±1498, `ds4_server.c` ±615, `ds4_tool_text.h` +59) — split tool delimiters in the sampling tracker, literal tool/thinking markers inside argument data, no repair of truncated tool actions, tool-observation errors without conversation compaction, opportunistic drafts across tool sampling boundaries, request-budget tool-recovery bounds. ALSO `Preserve the final byte of in-memory session snapshots` (c0a6119, the tip — session-state +116 test).
5. **QA / score-validation tooling** (~8 commits; `gguf-tools/quality-testing/` NEW: validate_scores.py +236, compare_frontier_logits.py +228 + tests) — "validate complete score tables and frontier logits" — a measurement-integrity lane (scores/tables validated before publication).

## Dedup (MANDATORY before proposing — the SSD/expert lineage has corpus coverage)

Grep `ssd|state.space|expert|cache|prefill|stream|resid` across `src/domains/kernel_opt/corpus/` + rust_perf corpus BEFORE any entry. Known coverage to check for overlap: SSD dual-cache rules (the B-series ssd cache), expert-offload/streaming rules, prefill-overlap rules (B94 coalesce / B118 parallel-disjoint-pread), weight-residency rules (B22/B79/B107 family). New axes expected: expert-cache AGING policy, router-readback reuse, working-set-budget residency, first-prefill overlap.

## Rule budget

3–6 rules across lanes 1/2/4/5 (lane 3 likely folds into lane 2 or stands alone as one CUDA rule). NotBenchable annotations where source-reported. rust_perf io/methodology rules from lanes 4/5 are eligible (validate_scores = measurement-integrity, the B122 diagnostics lineage).

## Tasks

- [x] T0 — pin re-check + clone + lane survey (this file's header)
- [x] T1 — dedup greps recorded
- [x] T2 — 2 parallel subagents (lane 1+3 kernel / lanes 2+4+5 tooling+methodology) propose rules with pattern + bounded fix-space + source quotes; re-verify every quote at the pin
- [x] T3 — author entries + `batch123_pseudocode_parses` gate
- [x] T4 — count pins walked (kernel_opt corpus count, rust_perf if io rules land, header ledgers, doc lines)
- [x] T5 — GOAT: batch gates FIRST-SHOT + lib default / kernel_opt / kernel_opt+latent floors + retrieval re-run (kernel 100/100/100, blended ≥94.7/96.9/97.7) + clippy 3 states + score-bench FLAT
- [x] T6 — Research 146 + AGENTS.md counts + `.docs/11_domains/` ledger rows + snapshot row (same commit) + `.raw` removal + push
