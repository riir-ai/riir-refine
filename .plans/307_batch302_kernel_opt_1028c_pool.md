# Plan 307 — Batch 302: the 10-28c staged kernel_opt pool (TF + cubecl + llama.cpp)

**Status:** COMPLETE — Batch 302 LANDED (see the landing commit; Research 344 carries the record; the lease folds at the landing)

- [x] T0 — clone sources at pins into `.raw/b302/` (blobless/shallow, removed at close): ashhart/tensorfold (`fd058f5`, `a05d620`, `d59a895`, `15af3c0`, `c8084c4`, `0ecaf33`), tracel-ai/cubecl (`a38a125` #1949, `12c4840` #1948), ggml-org/llama.cpp (`f5222f2` #31188 tag b11962, `a7af149` #31288 tag b11953); re-verify every staged grade by reading the diffs at the pins (never grade an unread file)
- [x] T1 — dedup greps (3+ vocabularies) against kernel_opt 994 + rust_perf 274: the top_k sampler vs B269 two-pass/B223 radix; the multi-projection parts table vs the B299-landed stream-table rule + B293 co-residency; sector-order staging vs the staging-stride family (B259 lesson); whole-window admission vs the specialization-admission family; MoE-GLU vs Batch-287 GLU-fusion + #29181 staged lead; tune-window compile-batch + cmma-lhs fragment vs the B299-landed #1942/#1940 rules. Honest drops allowed.
- [x] T2 — author via 2 parallel subagents over disjoint slices (own rules + count-neutral rider faces; every quote byte-verified at the pin; fixtures standalone-compile per field)
- [x] T3 — coordinator wiring: mod.rs KERNEL_CATEGORIES + category comments, count pins (tests.rs + router.rs + Plan-003 category table — the B299 lesson: the pin surface is ~37 asserts, not 5), batch302 needle gates keyed to LANDED text
- [x] T4 — gates: batch302 needle+parse (green-zero class: run at `--features kernel_opt`); kernel_opt lib; default lib; clippy -D ×3 postures; retrieval floors RE-MEASURE (corpus delta) at both postures. Freeze/score-bench NOT owed (kernel_opt-only, P30/B293); binary rebuild NOT owed (kernel_opt outside the release set).
- [x] T5 — Research 344 at landing; AGENTS.md kernel_opt count bump; .docs/11_domains delta; queue snapshot landing row (same commit)
- [x] T6 — commit `feat: batch 302: ...` + push; `.raw/b302/` removed at close
- [-] ENV NOTE — this checkout's oracle files (clippy_oracle.rs / bin/heal.rs / fix_verify.rs) carry the SIBLING auto-oracle lane's live WIP; the union tree read E0433 at claim time. Gates run at an isolated CARGO_TARGET_DIR; if their WIP is still broken at gate time, gate in a detached worktree at HEAD + my corpus files. Zero overlap with this plan's write set (src/domains/kernel_opt/* + wiring).

## Scope (from the staged rows — the snapshot's staged enumeration IS the working inventory, re-graded at T0)

| # | Source | Candidate | Grade at staging |
|---|---|---|---|
| 24 | TF `fd058f5` (10-28T09:03Z) | top_k-specialized two-pass Metal sampler (`tf_topk_slices` 32 slices + `tf_topk_race` one-simdgroup merge, ~700 µs → ~29 µs at 248,320 logits; "with tf_sample_full's own expressions, sum orders and tie rule") | B+/A− HIGH — own rule; the sampling-family gap (within-one-kernel slices+simdgroup twin of B269's cross-dispatch two-pass) |
| 25 | TF `a05d620` (10-28T01:25Z) | one grid, up to four projections (`tf_qmm_prefill_many`, `__grid_constant__` PrefillParts table, blocks route+rebase, body shared verbatim; screened-shape allowlist; "Keep the shipped kernel body literal: sharing its helper changed Blackwell SASS under nvcc 13.0 and 13.3") | B+/A− HIGH — own rule or face on the B299 stream-table rule + the body-literal compiler-stability atom; dedup adjudicates |
| 26 | TF `d59a895` (10-27T11:0xZ) | staging preserves global-memory sector order; swizzle lives at the read (cp.async weight stage, MMA loop re-indexes the fragment; bit-identical; gate kernels −28% at T=512 GB10) | B+ HIGH-MED — own rule (the copy's unit is the SECTOR, the consumer's unit is the FRAGMENT) |
| 27 | TF `15af3c0`+`c8084c4` (10-27/28) | M5 prompt widening inside a reserved working set (2048 rows, per-chunk 512, reservation gate: prompt scratch AND every configured stream beside weights + margin FIRST; bit-parity disclosed) | B + lane intel — the RULE is the reservation-before-widening gate; the config datum is league-lane intel |
| 28 | cubecl `a38a125` #1949 | bounded tune round compiles its first window then the rest AT ONCE ("about twice the compile time of one batch" when the limit never fires) | MED-HIGH — face on the B299-landed `compile-run-rejection-persists-patience-compiles-on-reach` (#1942) |
| 29 | cubecl `12c4840` #1948 | cmma lhs fragment reads its layout off the tensor's trailing axis (as the rhs already did; keys-first scores read transposed) | MED — rider on the B299-landed `stage-fill-shared-physical-axis-reads-through-projection` (#1940), fragment-level twin |
| 30 | TF `0ecaf33` (10-27T16:13Z) | whole-window attention specialized WITHOUT page metadata (head-major tile loaders restored; paged/mixed keep page-aware instances; POISONED page-metadata verification — the absent-work class, kernel-side) | MED — own rule candidate (specialization-admission family) |
| 31 | llama.cpp `f5222f2` #31188 (MERGED, tag b11962) | MoE (MUL_MAT_ID) joins the fused gate/up+GLU MMQ path; kernel choice runs the unfused selection "with the nominal tiles of all experts" | MED-HIGH — face on the Batch-287 GLU-fusion rule + the nominal-shape admission atom |
| 32 | llama.cpp `a7af149` #31288 (MERGED, tag b11953) | 384-expert case joins the top-k MoE fusion template ladder | LOW — face on the still-unlanded #29181 staged lead (adjudicate: face now or hold with the lead) |
| 33 | TF `0afe9dd` | nvfp4 experts decode rows in equal waves, two warps per unit-row | MED-LOW — adjudicate at T0 |
| 34 | uzu `6d96616` #1026 | f32 matmul path REMOVED (dtype routing hygiene) | C+ — drop-leaning |
| 35 | vllm `6abdff5` #63018 | NVFP4 compressed-KV dequant-gather tiling (python host-side) | MED-LOW — host-loop family, drop-leaning for kernel_opt |

Lower-tier rows 33–35 adjudicate at T0 with honest drops allowed. Expected yield: ~4–6 own rules + 2–4 rider faces (the falsification band per Protocol A).

## Slot adjudication

- kernel_opt domain: B299 landed 10-28 (pool EMPTY; this batch = the refill from the 10-28c sweep) — slot FREE.
- B298 bbtree = rust_perf domain (live lease to 2026-10-29T09:44Z, respected — zero rust_perf rows in this batch's scope).
- B300 (perfect-hash, plan 305) = rust_perf, unclaimed, untouched.
- B301 (plan 306 Phase 3, leviathan) = rust_perf+sec, this lane's own staged batch, stays behind B298/B300 — untouched here.
- Cross-domain precedent: B293, applied by the b299-pool B299 claim the same day.
