# Plan 111 — Batch 130: the CloudRift cuBLAS-5090 TMA distill

**Status:** COMPLETE (2026-09-08 — Batch 130 landed: 5 rules + 7 riders, corpus 575→580, GOAT all green)

Queue lead: CloudRift "Surfacing a 60% performance bug in cuBLAS" (Dmitry Trifonov,
2026-04-06) — filed in the snapshot 2026-09-08, renumbered 129→130. Repo located at
mine time: **`cloudrift-ai/emmy`** (NOT `DeploDock/Emmy` — that path 404s; the org is
`cloudrift-ai`, Apache-2.0, default branch `main` @ `4735c451`).

**Pin:** blog landing commit **`2aaa0328`** (#89, 2026-04-09 "Mini GPU compiler: TMA
SGEMM kernel beating cuBLAS on RTX 5090") — the blog's recipe dir
`recipes/sgemm_cublas_vs_tma/` + `deplodock/compiler/cuda/lower.py`
(`_lower_matmul_tma_db`) live at this pin; commit `8764cc2e` (#100, 2026-04-29)
restructured the repo (recipe dir removed, `deplodock/` → `emmy/compiler/`). The two
newer same-author articles (Modern GPU Matmul Optimization 05-29; FA Part 1/2
07-06/07-08) are FUTURE queue leads, not this batch.

## Tasks

- [x] T0: provenance — clone `cloudrift-ai/emmy` into `.raw/emmy`, checkout
      `2aaa0328`, verify LICENSE (Apache-2.0) + recipe dir + lower.py in-tree; record
      the pin in this plan BEFORE rule authoring. 4090 idle-checked pre-mine
      (18% util / 565 MiB, 2026-09-08).
- [x] T1: dedup grep — the 6 queue candidates (vendor-dispatch-verify,
      lds-cluster-stagger-warp-phase, tma-single-issuer-bulk-tile,
      compile-time-dim-bakes-kill-bounds, l2-raster-group-row-tiles,
      sub-ms-bench-boost-clock-never-engages) vs the existing corpus
      (`src/domains/kernel_opt/`, `src/domains/rust_perf/`); required checks:
      B127 (wgmma/mmq), B128 (Metal/FA + offline-device-tuned-table-dispatch for the
      dim-bake candidate), Research 107 (ktransformers), B20/B98/B113 riders.
- [x] T2: two parallel read lanes (subagents) — Lane A: the recipe dir
      (`recipes/sgemm_cublas_vs_tma/`: kernel, `ncu/batched_dispatch_finding.md`,
      bench scripts, methodology) → propose rules with pattern + bounded fix-space +
      category + rationale + verbatim quote + path@pin; Lane B:
      `deplodock/compiler/cuda/lower.py` `_lower_matmul_tma_db` + the TMA
      double-buffer architecture + compile-time specialization surface → same format.
- [x] T3: coordinator synthesis + quote re-verification against `.raw/emmy` at the
      pin (the B31 rule — every quoted line re-verified before landing).
- [x] T4: corpus entries (`entries_gpu_36.rs` or next free) + extend the standing
      syn-parse gate as `batch130_*`; walk ALL count pins the batch touches
      (totals ×5, category table, mod.rs lists, AGENTS, README, .docs rows).
- [x] T5: GOAT — G1 parse gates first-shot-or-fixed; G2 lib tests pass at
      kernel_opt (+latent if touched); G3 clippy 0 in all touched feature states;
      retrieval floors byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7)
      or re-pin + record delta in the Research note; score-bench appended FLAT or
      delta explained.
- [x] T6: Research note `.research/152_*.md` (RECORD) + AGENTS.md corpus-count bump
      + `.docs/11_domains/` rows + this plan → COMPLETE.
- [x] T7: commit `feat: batch 130: ...` + push; update
      `.distill/001_mining_queue_snapshot.md` in the SAME commit (landed entry,
      pins current, new leads queued: the two newer CloudRift articles); `rm -rf`
      the `.raw` clone.
