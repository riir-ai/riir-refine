# Plan 304 — Batch 299: the staged kernel_opt pool (TensorFold + uzu + cubecl + llama.cpp)

**Status:** IN FLIGHT — claim-first (snapshot claim row landed before authoring); slot adjudicated free per the B293 cross-domain precedent (bbtree B298 = rust_perf domain, lease to 2026-10-29T09:44Z, untouched).

**Claimed:** zed-m3-b299-pool, 1793180044, expires 2026-10-30T09:34:04+00:00

Cross-source batch (the Batch-105 combine mandate) over the ENTIRE staged kernel_opt pool — the 10-28b tfdelta rows 13–15 + the 10-28 p11 rows 1–5 + the #31146 merge-confirmed lead. Sources:

- **TensorFold** (`ashhart/tensorfold`, Apache-2.0):
  - `893c8d7b` shared-round cross-stream consolidation via address table (HIGH) + residency rider
  - `85571acb` MMA-window dispatch admission by output-count floor (MED-HIGH)
- **uzu** (`trymirai/uzu`, MIT): `13b138eb` #1022 mode-specialized merged kernel (MED)
- **cubecl** (`tracel-ai/cubecl`):
  - `a5e8143` #1937 contract-run partials deferred to the drain (HIGH)
  - `f4055ea` + `01ae331` #1947+#1936 degenerate-dim pair (MED-HIGH)
  - `2774afa` #1942 rejection persistence (MED-HIGH)
  - `50041a2` #1940 projection-aware stage fill (MED)
- **llama.cpp** (`ggml-org/llama.cpp`): `de5122d879` #31146 precondition-relaxation-refuses-stale-fusion (rider-grade) + lower-tier #31255 (pragma-unroll register bloat) + #31230 (TOP_K_MASK sentinel-fill + function-constant) adjudicated as riders at authoring.

NOT in this batch (stay staged / other domains): TF numerics row 16 (relu2 NaN — verification category), serving rows 17–19, rust_perf row 20 (borrowed-pool — rust_perf domain, behind the bbtree slot), bevy #26336 (rust_perf), measurement rows 8–9 (vllm Zipf/warmup), vllm #58305 (not actionable), corroborations 10/21/22 (record-only).

## Tasks

- [ ] T1 — dedup re-verification (3+-vocabulary greps over `src/domains/kernel_opt/corpus/` + rust_perf): shared-round vs gpu-resident-routing (B105)/co-residency-capped-launch (B293)/launch-count families; MMA-window floor vs B296 width-validity + B297 capacity-tiles; mode-specialized merge vs fusion-admission + dispatch-count + pipeline-cache-key (B293) + the p11 #31230 kin; drain-partials vs reduction/sum families; degenerate-dim vs B296 admission-validity; rejection-persistence vs B284 autotune family; projection-fill vs staging/layout families; stale-fusion vs fusion-admission hosts (wave-efficiency, borrows-the-dispatch).
- [ ] T2 — re-clone at pins into `.raw/b299/` {tensorfold, uzu, cubecl, llama.cpp}; re-verify EVERY quote byte-for-byte against the pinned commits (the B275 verify_quotes discipline) before any entry lands.
- [ ] T3 — two parallel subagent proposal lanes (A: TensorFold+uzu; B: cubecl+llama.cpp), pattern + bounded fix-space + category + rationale + source quote each; coordinator re-verification of every quote.
- [ ] T4 — authoring: new `entries_gpu_128.rs` (≤3 rules per authoring slice, fixtures standalone-compiled before landing), register in `corpus/mod.rs`, `batch299_rules_landed` + `batch299_pseudocode_parses` gates in `corpus/tests.rs` (mirroring the batch297 shape).
- [ ] T5 — NotBenchable annotations where the mechanism is admission/dispatch policy (no claimed benchable this batch — kernel_opt house norm; any exception carries its Pareto arm at `--release` with box-state provenance).
- [ ] T6 — GOAT: G1 parse/syn (batch299 gates), G2 lib tests (kernel_opt + default), G3 clippy `-D` ×3 postures. Freeze NOT owed (kernel_opt-only, the P30/B293 precedent); score-bench NOT owed (no rust_perf delta); binary rebuild NOT owed (kernel_opt outside the release set).
- [ ] T7 — retrieval re-measure at BOTH postures (kernel delta): `--features kernel_opt,latent_retrieval` floors + the Layer-5b2 default-posture gate (`--features latent_retrieval retrieval_`); a decay FAILS the batch.
- [ ] T8 — count pins via ONE shape-asserted python pass: kernel_opt total, category counts, corpus/tests totals + expected array, `mod.rs` header, `router.rs`, AGENTS.md, README, `.docs/11_domains/kernel_opt.md`.
- [ ] T9 — research note `.research/341_batch299_*.md` (next free number, re-scan at write) + landing row prepended to `.distill/001_mining_queue_snapshot.md` (queue state: pool consumed; remaining staged = rust_perf/measurement/serving rows) + AGENTS/README/.docs count updates in the same commit; `rm -rf .raw/b299` at close (ephemeral law).
- [ ] T10 — commit + push on `develop` (`feat: batch 299: ...`, named paths only, never `-A`).
