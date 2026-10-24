# Plan 277 — Batch 273 (nominal): mlx #4712 few-row qmv_mma rider + #4682 NAX split-K riders

**Status:** STAGED — not claimed (Batch 272 sonar lane is in flight; the next claimant opens this per the queue)

Distill source: Research 310 (ml-explore/mlx merges `c6abf0db` #4712 + `bff10220` #4682, pins + quotes verified there). Verdict: YES B− corpus staging, rider-shaped; strong source numbers (paired A/B + CIs), NotBenchable-here (Metal/NAX).

- [ ] T0 — re-verify pins: clone `.raw/mlx` fresh if removed, confirm `c6abf0db` + `bff10220` reachable, spot-check the quotes this plan cites (the `.raw`-entry-untrusted rule)
- [ ] T1 — dedup re-verification (mandatory even though Research 310 ran it): grep `small-batch-matvec-gemm-bridge-rung` (B98), `fewrow-tensor-tile-decode-into-rhs-operand` (B66), `splitk-tier-ladder-per-tier-ceiling-min-partition-inputs` (B268), `dequant-once-per-k-tile` (B16) for any face the riders below would duplicate; re-run both-vocabulary greps for anything landed since 2026-10-24
- [ ] T2 — decide the lead's host: rider on B98 (recommended — it owns the ladder + middle-mechanism class, B116 is the precedent) OR own rule with two-direction ladder kinship; record the decision in the corpus entry's distinct-from chain
- [ ] T3 — author entry/rider set (all NotBenchable-annotated, source-reported numbers quoted with provenance, never adoption bars):
  - [ ] T3a — lead: few-row matvec via mma fragments (no tensor API carrier) + per-silicon three-tier thresholds (NAX N,K≥2048 N%32 / M3-class ≥4096 / older ≥512 + N·K≥2^22) + partial-row masking (`x_rows[TM][2]`) + test-covered alignment refusals + WK−1 cross-simdgroup partial-exchange epilogue
  - [ ] T3b — rider: NAX split-K capacity-bounded ladder second instance on B268 (`≤4`, `tiles*split_k*2≤256`, `K/(split_k*2)≥256`, `K%(split_k*2*k_align)==0`, the `!biases → false` refusal, fp32 partials single-rounding, bit-identity-at-unchanged-split contract)
  - [ ] T3c — rider face: staged fetch/store block loader on the B16 family (register prefetch step k+1, double-buffered Ws, ONE barrier per K step, the fp32 smem-budget exclusion arm)
  - [ ] T3d — optional count-neutral face: 16-row schedule / flat `(128,1,1)` threadgroup dispatch
- [ ] T4 — fixtures: original minimal Rust reproductions only (no Metal source pasted — licence hygiene); every quote needle-gated verbatim at the pin; the constant-index unpack (`(q & 0x00070007) << 9`-class) shown in pseudocode, not transcribed wholesale
- [ ] T5 — standing gates (run only on a drained box — the Plan-337 load lesson; CARGO_TARGET_DIR isolation if siblings hold the lock): batch needle gates FIRST-RUN; kernel_opt lib count floor re-pinned with attribution (937 → N); default lib floor; clippy −D ×3 postures; retrieval floors HELD or re-pinned WITH attribution; freeze/score-bench NOT owed (kernel_opt-only corpus, the P30 precedent) unless a clippy-corpus rule rides
- [ ] T6 — binary rebuild NOT owed (kernel_opt outside the release set) — verify the stamp anyway if any shared file moved
- [ ] T7 — docs: `.research/310` stays the provenance record; append the batch row to `AGENTS.md` kernel_opt bullet + `rust_perf`… (kernel_opt Role bullet only); queue snapshot block updated (STAGED → LANDED, commit hash); league note: the MLX batched-decode movement (Qwen3.8-27B fp4 1.66–2.10× batch 8/16, M5 Max) recorded beside the next matrix run — matrix cells (pp/tg128) are OUTSIDE the 6..16-row regime, no escalation trigger
- [ ] T8 — numbering: batch number confirmed against the queue at claim time (272 in flight ⇒ 273 nominal; bump if the sibling's lane consumed 273 first); `.research/.highwater` already at 310; plan highwater 277 written by this file's landing
- [ ] T9 — commit + push on `develop`, `feat:` prefix, plan-first discipline satisfied (this file landed before any corpus edit)
- [-] T10 — DEFERRED: measuring the M3-class qmv_mma tier on our own M3 Max (would need an fp4/fp8 MLX-free Rust port — no consumer today; re-open only if the serving lane batches 6–16)
