# Plan 227 — Batch 225: the Q1_0 trio kernel_opt distill + three lead adjudications

**Status:** IN PROGRESS — claimed 2026-10-13 ~07:2x +07 (m3-max idle loop)

## Goal

Convert the walk #14-verified Q1_0 trio quote set (PrismML-Eng/llama.cpp v0.0.1-prism line)
into kernel_opt corpus rules, and adjudicate the three staged leads (#28748 Metal mul_mm
int32-offset 2GiB wrap; vLLM #59340 MoE w13 input-copy skip; llama.cpp #30164 vulkan
switch→ternary lowering) to rider/rule/watch verdicts.

Sources (pins, verified at walk #14):
- `59f2b8485` — v0.0.1-prism founding commit (03-02): 33 files +1439/−1 — `vecdotq.cuh` +117,
  CPU `quants.c` +101 + `arch/arm/quants.c` +140 + `arch/x86/quants.c` +159,
  `ggml-metal.metal` +308, CUDA mmq/mmvq/dequantize/common +218.
- `43e67bda3` — 03-13 mmq/mma fix, 7 CUDA files +128/−71.
- `bc8122e43a` — 03-19 merge.
- `48e9857` (#28748) — Metal kernel_mul_mm int32-offset wrap fix.
- vLLM `deedc7be` (#59340) — MoE w13 input-copy skip.
- llama.cpp `08ffba21e` (#30164) — vulkan CM2 quant switch→ternary lowering.

## Tasks

- [x] Claim row appended to `.distill/001_mining_queue_snapshot.md` + pushed (the arbiter).
- [x] Plan-first (this file, `.plans/.highwater` 226→227, dual_allocation_gate 0/0/0).
- [ ] Fresh `.raw` clone of PrismML-Eng/llama.cpp at the pins (verify HEAD == pin per commit
      checkout; the Batch-204 lesson: verify the PrismML 404 trap at clone time) + llama.cpp
      upstream at `48e9857`/`08ffba21e` (quote re-verification) — rm when done.
- [ ] Dedup grep BEFORE proposing: `Q1_0|q1_0|prism|vec_dot.*q1|mul_mat.*q1` + the Batch-204
      NEON PQ2_0 vec_dot rule (`6bfcd79a`) family read — the arm quants.c content may overlap
      the landed rule; classify each candidate rule as NEW / RIDER / DEDUP.
- [ ] 2 parallel authoring subagents over disjoint slices (A: Metal +308 + CUDA +218 + the
      43e67bda3 mma fix; B: CPU quants.c +101 + arm +140 + x86 +159 + vecdotq.cuh +117),
      each proposing rules with pattern / bounded fix-space / category / rationale / quote.
- [ ] Re-verify EVERY quoted line at the pin (git-show blob reads; the B223 discipline).
- [ ] Corpus entries + KERNEL_CATEGORIES wiring + count pins (×5 sites per the B223 pin-set
      lesson) + batch225 parse gate + needles.
- [ ] Lead adjudications: #28748 (correctness-class, thin adoption surface — grade against
      the corpus's offset/int32-wrap family), vLLM #59340 (staging-family — dedup vs the
      staging/alias family), #30164 (grade against a real wgpu shader posture or watch-only
      if compiler paths diverge — the GLSL→SPIR-V transferability disclosure stands).
- [ ] GOAT: G1 parse/syn (all new rules) · G2 lib tests · G3 clippy -D ×2 postures +
      retrieval floors (kernel self-variants + blended; re-pin only with pre-batch A/B
      attribution per the B223 convention) · freeze/score-bench NOT owed for a kernel_opt-
      only corpus delta (P30, the B223 precedent) — record the check.
- [ ] Research note `.research/255_*.md` + AGENTS.md corpus count bump + `.docs/11_domains/`
      update + mining-queue snapshot update (same commit as the batch).
- [ ] Commit `feat: batch 225: ...` + push. M3-side cargo gates only after the sibling
      riir-infer release test clears (Plan-337 zero-contention discipline).

## Non-goals

- No riir-train contact (routing restriction).
- No league/perf-rematch edits (watch-only M3 owner call stands).
- No score-bench/freeze regeneration unless a clippy-corpus delta appears (it should not).
