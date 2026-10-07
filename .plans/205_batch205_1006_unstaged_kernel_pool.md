# Plan 205 — Batch 205: the 10-06 unstaged kernel pool (llama.cpp ×4 + MLX ×2)

**Status:** COMPLETE 2026-10-07 — Batch 205 landed: 8 kernel_opt rules (D +3, B +4, F +1), corpus 761 → 769; all gates green (batch205 2/2, kernel_opt+latent_retrieval lib 3070/0, default lib 2471/0, clippy `-D` ×3 postures, retrieval 21/21 HELD); Research 240 §CONVERTED; commit at landing.

**Date:** 2026-10-07
**Box:** shikuwa/4090 (riir-refine idle loop, post-Batch-204)
**Queue provenance:** `.distill/001_mining_queue_snapshot.md` BATCH-204 row — "the 10-06 unstaged pool … fronts the next kernel slot"; staged 2026-10-06 in riir-ai `.benchmarks/rematch/2026-10-06_watch.md` (first-read grades, "quote verification owed at batch composition" — THIS batch pays that debt).

## Sources (all MIT)

| # | Repo | Sha | Subject | PR |
|---|---|---|---|---|
| S1 | ggml-org/llama.cpp | `9d3aba6` | CUDA: use MMVF for thin f16/bf16 mul_mat at small batch size | #29633 |
| S2 | ggml-org/llama.cpp | `d89651a` | CUDA: prefer whole-tile FlashAttention scheduling for efficient two-stage kernels | #29435 |
| S3 | ggml-org/llama.cpp | `1b43d31` | cuda: tile the lightning indexer over keys and tokens for 4 heads | #29901 |
| S4 | ggml-org/llama.cpp | `8b2fbaf` | CUDA: Optimize accumulation in mmq for NVFP4 type | #29857 |
| S5 | ml-explore/mlx | `f8aaf49` | fast qmv for outputs not ÷8 | #4516 |
| S6 | ml-explore/mlx | `0e3ff36` | sdpa_vector_2pass TK=4 unroll | #4596 |

**Dedup riders at composition:** `a3a1c47` (upstream metal few-row MMA #29869) = B196 fewrow family's upstream instance — dedup grep BOTH vocabularies before admitting anything fewrow-shaped. The fork `6bfcd79a2` member landed in Batch 204 (not re-mined here).

## Tasks

- [x] T0 Plan-first + dual_allocation_gate at allocation (DONE: exit 0, 0/0/0; highwater 204 → 205; research highwater 239 → 240 owed at note time)
- [x] T1 Ephemeral clones at the pins (`llama.cpp` @ the four shas, `mlx` @ the two shas) under `.raw/b205-*`; quote verification at the pins (NOTHING INVENTED)
- [x] T2 Dedup grep corpus (src/domains/kernel_opt/, rust_perf/) for each candidate concept — 3+ vocabulary variants
- [x] T3 Two parallel authoring subagents over disjoint sources (S1+S2+S3 vs S4+S5+S6), pattern/bounded-fix/category/quote per rule; coordinator re-verifies every quote verbatim
- [x] T4 Corpus entries (entries_18.rs or the current-next file) + batch205 syn-parse gate with per-rule needles
- [x] T5 GOAT: G1 parse gate · G2 kernel_opt lib tests (full slice, not narrow — the B203 annotation-pin lesson) · default lib · G3 clippy `-D` ×4 postures · retrieval floors (21/21 HELD or honest re-pin + Research-note delta)
- [x] T6 Research note `.research/240_*.md` (§CONVERTED disposition per member; grading corrections vs the 10-06 first-reads recorded)
- [x] T7 AGENTS.md corpus-count lines + README count lines + `.docs/11_domains/` paragraph(s) + snapshot BATCH-205 row — same commit
- [x] T8 Commit `feat: batch 205: ...` + push; `.raw/b205-*` removed at close
