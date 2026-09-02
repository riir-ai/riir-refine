# Plan 068 — Batch 100 candidate: mine Layr-Labs/qwen-3.8-mtp-challenge into kernel_opt

**Status:** QUEUED — pending owner go (verdict YES B+, Research 120; corpus 445 kernel_opt rules → est. 449–449+3)

Source: https://github.com/Layr-Labs/qwen-3.8-mtp-challenge — MIT, pin `@ 0863b06ac16e26e48fc06e97444095b00feb66d4` (verify + re-pin at mine time; beta-quality fast-moving repo). Verdict record: `.research/120_qwen38_mtp_challenge_verdict.md`.

## Tasks

- [ ] Re-clone into `.raw/qwen-3.8-mtp-challenge` at the CURRENT tip; `rev-parse` + record the new pin + license in the batch landing note (the 0863b06a pin is the verdict pin; the tree was static 2026-08-23 → 09-02 but re-verify).
- [ ] Read the 4 unread-at-verdict files before authoring any entry that cites them: `QwenRuntimeMTPDriver.swift`, `DFlashBatchedEngine.swift`, `SchedulerV2.swift`, `pagedattention.metal` pass-B body (+ `CompiledDecodeV2.swift` body if R6/R4 quotes reach there).
- [ ] Dedup re-verification at mine time (both vocabularies per candidate): donation/slice-update/COW (expect zero), fence-token-ordering (expect zero vs CUDA-fence hits), Metal residency-set (expect zero vs register-residency hits), head-split register-budget (expect zero vs B44/B07/B33), command-buffer byte budget (expect zero).
- [ ] Author R1 `functional-cache-versioning-loses-donation-under-kernel-read` (D) — the GOAT. Ref-shape: slice-update versioning of a slab cache + a custom-kernel reader pinning the buffer in an in-flight submission → donation lost → full-buffer copy per update. Opt-shape: stable buffer + kernel-side in-place write. Carry the two-regime evidence (write-only chains donate linear even 24 deep; interleaved read+write degrades) + the measured numbers (25,470→13.6 ms/step; ≈370 GiB/token copies) + the "dead on arrival" corollary. Cross-ref: B37 (transfer axis), B63 (reset), B39 (weight slots), B21 (aliasing).
- [ ] Author R2 `fused-kv-write-single-writer-reads-newest-from-input` (A/D) — single-writer-per-byte + newest tile loaded from the `knew`/`vnew` INPUTS (bit-identical bytes), ring-alias impossibility argument; race-free without cross-threadgroup primitives.
- [ ] Author R3 `in-place-write-through-input-emits-fence-consumer` (D) — hazard tracking at declared-buffer granularity makes in-place writes through "input" declarations invisible; fence token (prev+1, chained) consumed by every subsequent reader = scheduling order + memory barrier; decode arm rides graph edges.
- [ ] Author R4 `residency-set-wired-once-post-warm-persistent-footprint` (D) — capacity zero until wired → driver re-establishes residency per command buffer; wire once post-warm at persistent footprint + page allowance; never end/shrink the ticket. Explicitly Metal-scoped in-rule.
- [ ] Author R5 `register-budget-head-split-gqa-reread-trade` (E) — HPT chosen so `HPT·D/32 ≤ 32` acc floats; GQA/HPT threadgroups; smem formula modeled in host eligibility + test-pinned; ~30% re-read trade for static eligibility.
- [ ] Optional R6 (owner call at batch time): command-buffer referenced-byte commit threshold (C+, config-shaped) — mine only as a low-tier entry or fold as a hazard note into R1's batch; skip without ceremony if it reads config-only on re-read.
- [ ] Rider candidate: B22 `warp-shuffle-intrinsics-for-sub-warp-reduction` += the Metal measured-cost note (`simd_sum` ~14.5 cyc / quad ~2× cheaper / shuffle tree can beat the builtin — §1.1). Confirm the source line at mine time before landing.
- [ ] Standing gates: `batch100_pseudocode_parses` (cfg(not(kernel_verify)) per the B70 convention) + rules_present + concepts_covered; fix any case-sensitivity needle misses (the B51 lesson).
- [ ] Count pins: tests.rs corpus-count 445→(new total) + expected-array D/E/A + sums + router.rs expert total + mod.rs doc line + category headers — walk ALL pin sites (the 18th-count-pin-site lesson); check for stale headers while there.
- [ ] Drop-row discipline: Qwen35GatedDelta (B95 instance), DFlash spec-decode (B71/B81/B84), MTP micro-patterns (B30/B25/B26/B27 instances), evidence-ledger doctrine, window-as-offset, sinks-denominator — do NOT author any of these; ledger stays in Research 120.
- [ ] GOAT: kernel lib count-identical + default lib + clippy 0 both states + retrieval floors 14/14 (no re-pins expected); R1's measurement citations quoted from the doc, never paraphrased into fake precision.
- [ ] Update `.research/120` status line → RECORD at batch landing; update AGENTS.md Batch 100 entry + queue snapshot "Last landed" line; docs corpus count if the .docs book carries one.
- [ ] `rm -rf .raw/qwen-3.8-mtp-challenge` as part of the landing commit (a landed batch with a live .raw entry is an unfinished batch).
- [ ] Commit+push on develop (`feat:` prefix), queue snapshot updated in the same commit.

## Non-goals

- The vendored `Vendor/mlx-swift` tree (upstream Apple MLX; CUDA backend unused by the ranked track).
- The challenge's scoring/anti-cheat/evidence-ledger design (benchmark doctrine, not perf rules).
- The leaderboard perf numbers (league intelligence — routed to perf-rematch opponent-watch, not the corpus).
