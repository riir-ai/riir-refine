# Plan 279 — Batch 275: the MLXFAST staged pair conversion

**Status:** IN PROGRESS — claimed 2026-10-25 ~01:4x +07 (zed-m3/M3, expiry 2026-10-27T01:45Z)

Sources (staged by the 2026-10-23 watch readout, `.distill/001_mining_queue_snapshot.md`):

1. **Layr-Labs/mlxfast-bonsai2-9b-engine @ `8cee3ef`** — the MLX sibling of the already-mined
   Batch-251 CUDA source (cudafast @ `36c9536`, Research 284). League model family
   ("Qwen 3.5 9B geometry" = the Ternary-Bonsai-2-9B sibling of our 27B). 123 promoted
   submissions / 13 solvers; the DFlash drafter arc (stock head through #58 at 3.5→4.1
   tok/round, custom drafter #59+ at 7.8→9.5) = LANE INTEL, not corpus.
2. **Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ `59160bd`** — Qwen 3.8 Flash Next 125B-A6B
   MoE (6B active); 65 promoted submissions; stock head 1.0 tok/round everywhere = ZERO
   speculative decoding; the whole 2.11× record is kernel-side.

⚠ BOTH carry "Other"/NOASSERTION licences at API read — **licence verification at clone is
the batch's first act; a non-permissive or unverifiable licence = the batch DISSOLVES with a
recorded NO-GO verdict.** (The cudafast twin carried MIT via `licenses/yukon-cudafast`.)

Tasks (Protocol A; mark `[x]` as complete, `- [-]` for defers with reasons):

- [ ] T0 — clone both to `.raw/b275/`, verify pins EXACT (`8cee3ef` / `59160bd` via rev-parse),
      **licence check FIRST** (LICENSE files + licenses/ dirs at both roots + the vendored
      engine trees' own licences; every mined file's provenance chain permissive)
- [ ] T1 — enumerate both trees (file inventory, kernel/shader inventory, the submission
      diff-history shape if the repo carries it); scope adjudication: one batch or split
      (B275 bonsai2 + B276 qwen38) if the pair exceeds ~16 rules of scope
- [ ] T2 — dedup grep BEFORE proposing: every candidate concept × 3+ vocabulary variants
      against `src/domains/kernel_opt/` (941 standing) + `src/domains/rust_perf/` (266
      standing), with the B251 CUDA-sibling entries (`entries_gpu_105`/`entries_gpu_106`)
      read in full — Metal faces that duplicate a B251 CUDA rule are RIDERS on the same
      host, not new rules
- [ ] T3 — authoring slices (subagents over disjoint slices, ≤3 rules each per the B268
      slice-timeout lesson); every quoted line machine-verified against the real file at
      the pin; standalone fixture compile per rule
- [ ] T4 — coordinator wiring: `entries_gpu_1NN.rs` NEW file(s), `KERNEL_CATEGORIES` slices,
      mod.rs registration, count pins via ONE shape-asserted python pass (the B265 lesson:
      multi-site count re-pins are a script job, never fuzzy edits); byte-level CRLF-safe
      edits to `.distill/001_mining_queue_snapshot.md` (the file is natively CRLF —
      the 2026-10-25 repair commit `86bea58a`)
- [ ] T5 — gates: batch275 needle + parse gates (feature-gated → `--features kernel_opt`),
      kernel_opt lib, default lib, clippy −D ×3 postures (default / kernel_opt /
      no-default, all --all-targets), retrieval floors RE-MEASURE (corpus delta owed);
      freeze/score-bench NOT owed (kernel_opt-only corpus, P30/B250) unless clippy_lints
      sneaks in; binary rebuild NOT owed (kernel_opt outside the release set)
- [ ] T6 — Research note `.research/312_*` (`.research/.highwater` 311→312 at landing),
      AGENTS.md kernel_opt bullet re-pin, `.docs/11_domains/kernel_opt.md` if owed,
      queue-snapshot landing row (byte-level splice), mining-queue snapshot update in the
      same commit
- [ ] T7 — commit `feat: batch 275: ...` + push; `.raw/b275/` removed at close;
      claim discharged by the landing row
