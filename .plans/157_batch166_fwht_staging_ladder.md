# Plan 157 — Batch 166: the FWHT staging-ladder rule (kernel_opt)

**Status:** COMPLETE — Batch 166 landed; all gates green; see T11 for the commit hash.

## Context

Research 194 (2026-09-21) promoted the FWHT staging-ladder staged lead to the
kernel_opt mining queue: riir-ai Plan 600 / Issue 980 landed the GPU
runtime-Hadamard lane (`bonsai2_hadamard` DEFAULT-ON 09-19; `deltanet_rotation_{cudarc,cubecl}.rs`
+ the 5-kernel fused rotate+quantize family + dense-ab GEMM prefill), which is
the second butterfly-transform instance the 09-17 "STAGE, don't admit"
condition waited for. The benchable axis is in-repo (sm_89 decode + M3 Metal
twin); the corpus rule itself needs no novelty — decision content + bounded
fix-space (Research 194 §2).

**DEDUP CORRECTION (this plan's first finding):** Research 194 §5 recorded the
dedup grep as `fwht|walsh|hadamard|butterfly → 0 entries (re-verified)`. That is
WRONG — the recursive grep finds `fwht-shuffle-xor-butterfly-register-resident`
(entries_gpu_11.rs, Batch 96 / Research 114 / Plan 060) plus butterfly-reduction
riders in entries_gpu_01/04. Likely mechanism: a non-recursive grep over
`src/domains/kernel_opt/*.rs` misses the `corpus/` subdirectory. The new rule is
NOT a duplicate (the Batch-96 rule scopes the ALL-REGISTER regime, N ≤ LANES ×
budget, and punts "larger → smem tiles or multi-pass" without content — the
ladder IS that fallback's decision content), but the entry must carry the
distinctness/composition paragraph and the Research note + .distill row must be
corrected in the same batch.

## Tasks

- [x] T0 — plan-first: this file; batch number 166 / plan 157 read from `.highwater` at write time (Batch 165 / Plan 156 landed 09-20; re-scanned).
- [x] T1 — dedup adjudication (done, recorded in-plan + Research 194 addendum + the .distill row correction): the new rule is complementary-regime, distinctness paragraph mandated; the false-clean dedup record corrected in Research 194 + `.distill/001`.
- [x] T2 — author `src/domains/kernel_opt/corpus/entries_gpu_45.rs` (1 rule: `fwht-butterfly-staging-ladder-warp-smem-register`): the staging ladder (sub-warp spans → warp shuffles; genuinely cross-warp spans → smem, one barrier each, ≤ ~3 at service geometries; per-lane-contiguous-run top stages → register butterflies, zero syncs), the incumbent defect shape (all 10 stages through smem, 2 syncs/stage, `__shared__ float smem[1024]`, 256 threads, W=1024), fork evidence (+17%/+11% decode 9B/2B H100 folded bands; Hadamard = 25.8% of GPU time at block 4096; #161 Metal twin +4–36% decode at +2.2–4.8% pp — risk pricing, not transferable), the sign-fold-already-shipped note, the bit-identity gate (FFMA-contraction precedent `d07542ee3`; parity pins decide; fused-family "butterfly: fwht_rotate_forward_f32 verbatim" contract → shared device-inline body or lockstep 5-kernel rewrite), in-repo consumers, W>2048 rider (only with a wider-block model), source pins.
- [x] T3 — register in `corpus/mod.rs`: `mod entries_gpu_45;` + `entries.extend(...)`; join category **B: Compute** (list + comment 64→66 — the 64 was ALREADY stale vs the pinned 65, fixed en passant with a note); doc comment 619→620 (line ~2002).
- [x] T4 — count pins walked (every site): final tally ×9 — tests.rs `:104` (+message), `:5342` (comment), Compute pin 65→66 (+"updated for Batch 166" comment), `:5415-5417` (sum), `:5428-5429` (grouped), `:9536-9537` (+message); **router.rs:472 (found by the G2 full-suite run, invisible to the grep that found the rest)**; mod.rs doc comment + B-category header (already stale 64-vs-pinned-65 → 66 en passant); AGENTS.md `:19` + `:162`; `.docs/11_domains/kernel_opt.md` `:13` (+ lineage) + new Batch 166 paragraph.
- [x] T5 — gate tests ×3 in `corpus/tests.rs` (after batch149's): `batch166_rules_present`, `batch166_concepts_covered` (exact-phrase pins incl. the distinctness), `batch166_pseudocode_parses` (G1, both ref + opt through `verdict_passes`).
- [x] T6 — GOAT: G1 batch166 3/3 + corpus suite 305/0; G2 lib kernel_opt 2390/0 + default 2037/0; G3 clippy `-D warnings` both states (one doc-list `+`-marker fix in the new file header). Freeze tables untouched (kernel_opt corpus ≠ `build_seed_corpus()`).
- [x] T7 — retrieval-precision re-measure: floors HELD — 18/18 retrieval tests green (90.6s); sweep: clippy top-1 89.6% ≥ 80% bar; blended op-bigram 94.7/96.2 vs the 77.1/86.4/92.4 floors.
- [x] T8 — score-bench: heal_rate 96.4% ≈ FLAT vs the recorded 96.2% (row #119) — the expected near-flat (kernel rules don't move the clippy fixture bench); one jsonl row appended (gen-8 frontier snapshot deduped on the second run), artifacts committed with the batch.
- [x] T9 — Research 194 addendum: dedup correction (the Batch-96 rule exists; complementary-regime adjudication; the grep-mechanism lesson), GOAT results, the ×9 count-pin walk (router.rs found by execution).
- [x] T10 — `.distill/001`: my-blob committed (HEAD + my hunks only — the LANDED row + the intake-row dedup correction/CONSUMED note); the sibling's 09-20 fix_verify intake block stays uncommitted in the worktree (verified: staged diff = 3 insertions/1 deletion, their marker absent from the staged blob).
- [x] T11 — commit `feat: Batch 166: FWHT butterfly staging ladder (kernel_opt 619→620)` + push; the snapshot updated in the SAME commit. **Final commit hash: `ecc8d5a5`** (the `23a1a92c` originally written here was the pre-amend hash — an in-commit document cannot cite its own amended hash; recorded in this follow-up).

## Constraints

- 4090 has two cargo.exe processes live (checked 09-21 via Tailscale) — zero GPU/cargo work scheduled there; this batch is M3-local by construction (kernel_verify is a STUB; the GOAT is G1–G3 + retrieval, no GPU).
- Sibling WIP in `src/fix_verify.rs` + `.distill/001` is quarantined: staged commits name files; never `-A`.
- Queue-1 gate: issues 089/102 re-adjudicated OPEN-but-accumulation/parked (089 T4/T8 are accumulation-clocked, self-winding; 102 is library-drift watch) — mining admissible per the handoff's cycle verdict.
