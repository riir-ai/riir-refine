# Plan 309 — Batch 304: the kernel_opt delta pool (leadstage remainder + 10-28d/e/10-29a)

**Status:** ACTIVE — claimed 2026-10-29 (amo-idle-b304; 48 h expiry per protocol; push is the arbiter).

## Scope

The unconsumed kernel_opt staged leads, oldest-first, post cross-pass dedup (the 10-25
correction row already adjudicated #30553/#30573 mined B253, #26771 mined B265, mlx #4712
mined B273, #30804 rider-faced B274/B280, #29882 NO-GO — recorded as drops with
provenance, never re-staged). rust_perf stays OFF this batch: bbtree B305 (plan 289
continuation) holds the rust_perf head to 10-29T09:44Z, B300 (perfect-hash, plan 305) and
B301 (leviathan Phase 3, plan 306) are foreign-claimed to 10-30 — the B293 cross-domain
precedent: kernel_opt-only, zero rust_perf/sec rule counts.

## Candidate slate (T0 grades bind; honest drops allowed)

| # | Lead | Source | Grade |
|---|---|---|---|
| A | #30072 dst-offset 64-bit sweep law (`9b8446b`) | llama.cpp | HIGH |
| B | #31179 concat dst-row parallelism + transposed row tiles (`4d71b42`) | llama.cpp | HIGH |
| C | #4717 grid-dimension ceiling → grid-stride (sha at T0) | mlx | HIGH |
| D | #63382 `b42f25b` fused-pair-replacement-writes-every-legacy-output-every-path | vllm | STRONG B |
| E | #63791 `4da2a82` silent-zero-launch gridDim.z + boundary-sweep test shape | vllm | MED-STRONG |
| F | #31201 `c3df4b7` IQ4_NL branch-free midpoint rank | llama.cpp | MED-STRONG |
| G | SASS-identity extraction + instrument (`2922b93`+`2e04524`, `c6193ca`/`dd0a9cc`) | tensorfold | STRONG |
| H | lithos trio (`8ed1611` bit-exact geometry search / `cd220ab` intra-tg row split / `2d51c4a` variant-beside-default) | lithos-metal | MED-STRONG |
| I | #1478 `8abd450` dual-bank scratch retention per round class | tensorfold | MED-STRONG |
| J | #30795 moe_reduce fusion refusal under dst-aliasing (sha at T0) | llama.cpp | MED-HIGH |
| K | #31336a+b `cf24317b` bounds hazards (unbounded parsed index + narrow-int offsets; merge-inside-stride OOB) | llama.cpp | B− ×2 NotBenchable |
| L | #63827 `d48202060` fence-deferred async loads over reused pages | vllm | MED |

Reserve (T0-adjudicate only if the core thins below 8): cubecl #1914+#1915+#1907
measurement pair, TF `6ac981b` per-arch tuning table, #30412 OpenCL dp4a, #30585 SVE
VL-gate, #63047 armed-paths budget, mlx #4764 gguf size audit arm.

Pre-named drops: #30804 residue (rider-faced B274/B280 — rider-or-drop), #29882 (the
NO-GO audit verdict stands), #30553/#30573 (B253), mlx #4712 (B273), #31255 (dropped
B299), #31230/#1941 (rider faces / record-only per B299).

## Tasks

- [ ] T0a clone sources at pins into `.raw/b304/` (blobless shallow: ggml-org/llama.cpp, ml-explore/mlx, ashhart/tensorfold, lithos-ai/lithos-metal, vllm-project/vllm); licence re-verify per repo at clone
- [ ] T0b discover the unstaged shas (#4717, #30795) + re-verify EVERY staged quote at its pin; full diff reads on the Tier-1 leads
- [ ] T0c cross-pass dedup adjudication recorded (the pre-mined rows above)
- [ ] T1 dedup greps at 3+ vocabularies vs the kernel_opt corpus at HEAD (dedup-at-mining binds)
- [ ] T2 authoring: 2 subagents over disjoint slices; entries_gpu_137.rs (+138 if needed); every rule carries pattern, bounded fix-space, category, rationale, source quote, distinct-from kinships
- [ ] T3 wiring: mod.rs KERNEL_CATEGORIES slices + category comments; router expert-total untouched unless counts move (they will)
- [ ] T4 count pins via ONE shape-asserted python pass (tests.rs sites + the Plan-003 per-category table + category headers + AGENTS.md + README rows)
- [ ] T5 gates: G1 batch304 needle+parse (`--features kernel_opt`); G2 kernel_opt lib + default lib; G3 clippy -D ×3 postures (native 1.99.0 pin); T7 retrieval floors both postures (kernel_opt,latent_retrieval + default); freeze/score-bench NOT owed (P30); binary rebuild NOT owed (kernel_opt outside the release set)
- [ ] T6 records: Research 348; AGENTS.md kernel_opt count; .docs/11_domains/kernel_opt.md; snapshot landing row (this file's tasks ticked)
- [ ] T7 commit `feat: batch 304: ...` + push; `.raw/b304/` removed at close

## Gates, criteria, risks

- Shared-tree hazard: the bbtree b305 lane may resume on this checkout before
  16:44+07 — commit ONLY this batch's paths; if the tree is dirty at gate time, gate on
  an isolated CARGO_TARGET_DIR and land via the temp-index recipe (B302 precedent).
- The embedded-quote defect class (B299's twice-fired lesson): never replace an old
  prefix WITH its leading quote when prepending chain notes.
- Fixtures carry no string literals (the B274 lesson); needles keyed to landed text
  AFTER the entries are written (the Batch-259 lesson).
- Count pins surface wider than the corpus grep: the lib-test failing list is the only
  reliable enumerator (the B274 lesson).
