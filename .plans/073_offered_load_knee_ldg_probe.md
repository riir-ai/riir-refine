# Plan 073 — offered-load-knee LDG probe batch (2608.15764 → kernel_opt)

**Status:** ARMED — T0b (GEMV inner-loop K audit, source-read half) DONE 2026-09-04; T0a (4090 probe) BLOCKED behind the NoRA Issue-504 T3 confirm matrix (GPU-exclusive window, expected clear ~10:00–12:30 2026-09-04). T1/T2 verdicts pend T0a.

Source: arXiv:2608.15764 (A−, CONFIRMED verdict in [Research 121](../.research/121_arxiv_pool_refill_walk_6.md) §discharges) — H100 plain-load bandwidth peaks at K≈2 outstanding loads/thread, −35% by K=8 (fixed-work + order + 2-die + allocation-sweep + ncu-controlled; authors leave the mechanism unidentified — only the operational curve distills). Ada transfer explicitly OPEN (paper is H100-only; Blackwell ≠ Ada).

## Pre-registered batch shape (from Research 121)

- **T0a — 4090 LDG-response probe** (microbench, GPU-exclusive window): replicate the paper's operational curve on Ada — bandwidth vs outstanding-plain-load count K ∈ {1,2,4,8,16}, fixed total work, order-controlled, allocation-swept. Decide: does the Ada knee sit near K≈2?
- **T0b — cudarc GEMV inner-loop K audit** (source read, no GPU) — **DONE 2026-09-04, findings below.**
- **T1 — rule candidate `offered-load-knee-plain-loads`** (kernel_opt, N or measurement class): the Ada probe verdict decides landing vs annotation-only vs H100-scoped demote-loser.
- **T2 — B92 rider** (the occupancy×depth WHEN-to-use axis B92 lacks) only if the combine question survives dedup.

## T0b — GEMV inner-loop K audit: FINDINGS (2026-09-04, 4090 box, source read)

Target: `../riir-ai/crates/riir-gpu/src/gemv_ternary_cuda_raw.rs` (the cudarc decode-GEMV family; all entrypoints share `gemv_ternary_row`, the Issue-705 dedup).

1. **Production decode fast path (`gemv_ternary_row`, ablock==16):** per lane per stride-32 iteration issues exactly TWO global loads back-to-back — `qpair` (4B codes) + `int4 uv` (16B activations) — then consumes (byte_perm + 4× `__dp4a`); then two small scalar loads (`row_scale[...]` 2B, `ascale[blk]` 4B) feed one fma. The only loop-carried dependency is the `acc` fma; the stride-32 loop has a runtime trip count (no unroll possible), so the compiler issues loads in program order: **natural K ≈ 2** (≈4 counting the scalars).
2. **This sits AT the paper's measured optimum** (H100 knee K≈2). Corroborating throughput evidence: lm_head's measured 854 GB/s (Bench 684/697 era) ≈ 85% of the 4090's ~1008 GB/s peak — the residual gap is plausibly ECC/effectivity + the small per-lane scale gathers, not under-issuance.
3. **Adjacent measured Ada data point already in-source:** the Issue-734-T5 `__ldcs` A/B on the qpair load measured a **1.2% tok/s REGRESSION** ("Ada's default caching wins for the 128 B/warp-iteration streaming window"). That is cache-POLICY, not MLP depth — adjacent evidence, not conclusive for the knee, but consistent with "don't touch the streaming load recipe on Ada."
4. **`gemv_ternary_dp4a_fused` phase 2:** act/ascale ride SHARED memory → global-load K ≈ 1–2 (codes + wscale only). Phase 1 quantize is a scalar-serialized absmax loop — different shape, out of the audit's K scope.
5. **`gemv_ternary_transposed_f32` (backward):** K ≈ 1 scalar loop, latency-bound by design; not a hot decode surface.
6. **Scope boundary:** the audit covers the decode GEMV family per Research 121. The PREFILL staged-tile mma kernels (Issue 734's rowtiled8/mma arms) own their own cp.async surface — that is B92's territory, explicitly not re-audited here.

**T2 rider implication (pre-read, probe decides):** adding cp.async/load DEPTH to the decode dp4a GEMV would push K past the knee into the paper's degradation regime — the decode surface is likely a NEGATIVE case for the rider. The rider's live home remains the prefill staged-tile kernels.

## Tasks

- [x] **T0b — GEMV inner-loop K audit** (source read; findings above — no GPU needed)
- [ ] **Dedup pass** — `offered-load-knee-plain-loads` vs corpus: B92 `cpasync-multistage-tile-pipeline` (HOW vs this rule's WHEN-NOT — complementary by design), B20 lane-pinned byte coalescing, the B58 same-input family, the honest-loser annotation precedents (`filter-count-to-any`, `newton-refinement`). Confirm the rule lands as a depth-cap/stop rule, not a rewrite recipe.
- [-] **T0a — 4090 LDG-response probe** — BLOCKED behind the Issue-504 T3 confirm matrix (GPU-exclusive). Unblock: matrix `ALL_DONE` in `../riir-train/target/t504_confirm/status.txt` + GPU quiet check. Probe shape: K ∈ {1,2,4,8,16} outstanding independent 16B plain loads/thread, fixed-work, ≥3 interleaved reps, ncu bandwidth counters if available; record Ada knee vs H100's K≈2.
- [ ] **T1 — rule candidate verdict** — from T0a: (a) Ada knee ≈ 2 → land the rule (N/measurement class) with cross-arch knee table; (b) Ada knee ≠ 2 → H100-scoped annotation-only; (c) Ada refutes the curve shape → demote-loser, record honestly.
- [ ] **T2 — B92 rider** (occupancy×depth axis) — only if T1 lands as a rule and the combine survives the T0b implication above (decode likely negative; prefill-only applicability may make the rider a one-line scope note on B92 instead of a batch member).
- [ ] **Standing gates + count pins + GOAT** per Protocol A (corpus tests, retrieval floors, clippy both states, AGENTS.md count bump, Research note numbering via `.research/.highwater`).
- [ ] **Commit + push + queue-snapshot update** (same commit).

## Non-goals

- No change to the decode dp4a GEMV inner loop — T0b says it already sits at the knee; the batch produces knowledge (a rule/annotation), not a kernel rewrite.
- No H100 claims beyond citing the paper's numbers (we have no H100); Ada numbers come only from our own probe.
