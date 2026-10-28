# Plan 304 — Batch 299: the staged kernel_opt pool (TensorFold + uzu + cubecl + llama.cpp)

**Status:** LANDED 2026-10-28 — Batch 299, corpus 986 → 994 (+8 + 1 rider face), `entries_gpu_134.rs`; gates ALL GREEN (G1 2/2, G2 3265/0 + 2749/0, G3 clippy ×3, T7 retrieval floors HELD both postures); see Research 342 + the snapshot landing row.

**Lease closed:** zed-m3-b299-pool, 1793180044, expires 2026-10-30T09:34:04+00:00 — discharged by the Batch 299 landing (this plan, 2026-10-28).

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

- [x] T1 — dedup re-verification (3+-vocabulary greps over `src/domains/kernel_opt/corpus/` + rust_perf): shared-round vs gpu-resident-routing (B105)/co-residency-capped-launch (B293)/launch-count families; MMA-window floor vs B296 width-validity + B297 capacity-tiles; mode-specialized merge vs fusion-admission + dispatch-count + pipeline-cache-key (B293) + the p11 #31230 kin; drain-partials vs reduction/sum families; degenerate-dim vs B296 admission-validity (ZERO hits — novel); rejection-persistence vs B284 autotune family; projection-fill vs staging/layout families (gpu_60 = prose only); stale-fusion vs fusion-admission hosts.
- [x] T2 — re-clone at pins into `.raw/b299/` {tensorfold, uzu, cubecl, llamalp}; re-verify EVERY quote byte-for-byte against the pinned commits (all ~30 quotes verified: tensorfold/uzu/cubecl/llamalp worktrees + `git show` at 893c8d7b).
- [x] T3 — two parallel subagent proposal lanes (A: TensorFold+uzu; B: cubecl+llama.cpp); coordinator re-verification caught ONE lane-B citation drift (rank-partial is gpu_102, not gpu_64) + ONE needle paraphrase (run along, not runs along).
- [x] T4 — authoring: `entries_gpu_134.rs` (8 rules, original fixtures), registered in `corpus/mod.rs` (mod + extend + 7 category lists + 7 head-comment counts), `batch299_rules_landed` + `batch299_pseudocode_parses` gates in `corpus/tests.rs`; the R9 rider face on gpu_109.
- [x] T5 — all 8 rules NotBenchable in-rule (launch-count / residency-liveness / admission / dispatch-count / reduction-count / layout-crash / tuner-state / staging-correctness axes — the Batch-136 domain-reason class; no claimed benchable).
- [x] T6 — GOAT: G1 (2/2), G2 (kernel_opt lib 3265/0; default 2749/0), G3 (clippy `-D` ×3 postures clean, 4 pedantic doc-backticks fixed en route). Freeze NOT owed (kernel_opt-only, P30/B293); score-bench NOT owed; binary rebuild NOT owed (kernel_opt outside the release set). EN-ROUTE: the corpus-count pin surface proved to be 37 asserts + the Plan-003 table + router.rs total (all re-pinned, the batch-299 chain note prepended); the embedded-quote defect class fired twice (5 + 32 sites) — fix recorded in Research 342.
- [x] T7 — retrieval re-measure BOTH postures: `kernel_opt,latent_retrieval` 28/0 (279.55s) + Layer-5b2 default posture 27/0 (66.16s) — floors HELD, no re-pin.
- [x] T8 — count pins: 37 corpus-count asserts (tests.rs) + Plan-003 table (A 193, B 111, C 65, D 237, E 89, K 27, N 88) + router.rs total 994 + AGENTS.md ×2 sites + 7 category head comments.
- [x] T9 — research note `.research/342_batch299_staged_pool_kernel_opt.md` (341 was taken by the perfect-hash lane mid-flight — the write-time re-scan held) + landing row prepended to `.distill/001_mining_queue_snapshot.md` + AGENTS updates; `rm -rf .raw/b299` at close.
- [x] T10 — commit + push on `develop` (`feat: batch 299: ...`, named paths only, never `-A`).
