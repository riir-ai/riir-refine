# Plan 227 — Batch 225: the Q1_0 trio kernel_opt distill + three lead adjudications

**Status:** COMPLETE — landed 2026-10-13 ~09:0x +07 (m3-max idle loop; corpus 796→801, 5 rules + 5 riders + 1 watch)

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
- [x] Fresh `.raw` clone of PrismML-Eng/llama.cpp at the pins (HEAD == `59f2b8485`; all three pins present; the PrismML-Eng org resolves — no 404 trap) + llama.cpp upstream leads at `48e9857`/`08ffba21e` from the local clone + vLLM `deedc7be` via the GitHub patch endpoint — rm of the clone at close.
- [x] Dedup grep BEFORE proposing: the Prism lineage is HEAVILY pre-mined (13+ batches cite later pins); the founding-infrastructure content is the new half — classified per candidate (5 NEW / 5 RIDER / rest OWNED-or-skip).
- [x] 2 parallel authoring subagents over disjoint slices (A: CUDA+Metal 11 files; B: CPU 8 files), verbatim quotes + dedup verdicts each.
- [x] Re-verified EVERY quoted line at the pin (git-show blob reads at `59f2b8485` + `43e67bda3`, full set, this session).
- [x] Corpus entries (entries_gpu_89.rs ×5) + KERNEL_CATEGORIES wiring (A/B/D/N) + count pins (tests.rs ×4 + router.rs, 796→801; per-category A 163/B 87/D 187/N 65) + batch225 parse gate + needles (5 rules + 5 rider hosts).
- [x] Lead adjudications: #28748 → gpu_02 vendor-view rider; vLLM #59340 → popcorn processing-set predicate rider; #30164 → WATCH-ONLY (glslang-specific, no wgpu/naga evidence; re-open trigger recorded in Research 255).
- [x] GOAT: G1 parse gate PASS · G2 lib 2945/0 (kernel_opt) + 2504/0 (default) · G3 clippy -D ×4 postures clean (default/kernel_opt/kernel_opt+latent_retrieval libs + all-targets) + retrieval floors HELD (kernel 100.0%, blended 91.2/94.7/95.3 — no re-pin) · freeze/score-bench NOT owed (zero clippy-corpus delta, P30, the B223 precedent) — recorded in Research 255.
- [x] Research note `.research/255_batch225_q10_trio_distill.md` + AGENTS.md corpus count bump (801) + `.docs/11_domains/kernel_opt.md` ledger update + mining-queue snapshot update (same commit).
- [x] Commit `feat: batch 225: ...` + push (the claim discharged).
- [-] The M3-side full-workspace gates beyond the batch's own suite — NOT owed for a corpus-add batch (the batch gates above are the G1-G3 surface; the ci_feature_guard full run rides the next guard lane). Deferred with reason: scope discipline, the B223 precedent ran the same surface.

## Non-goals

- No riir-train contact (routing restriction).
- No league/perf-rematch edits (watch-only M3 owner call stands).
- No score-bench/freeze regeneration unless a clippy-corpus delta appears (it should not).
