# Plan 068 — Batch 100 candidate: mine Layr-Labs/qwen-3.8-mtp-challenge into kernel_opt

**Status:** COMPLETE — landed 2026-09-02 (Batch 100, 8 rules + 1 rider, corpus 445 -> 453; all GOAT green)

Source: https://github.com/Layr-Labs/qwen-3.8-mtp-challenge — MIT, pin `@ 0863b06ac16e26e48fc06e97444095b00feb66d4` (verify + re-pin at mine time; beta-quality fast-moving repo). Verdict record: `.research/120_qwen38_mtp_challenge_verdict.md`.

## Tasks

- [x] Re-clone into `.raw/qwen-3.8-mtp-challenge` at the CURRENT tip; `rev-parse` + record the new pin + license in the batch landing note (the 0863b06a pin is the verdict pin; the tree was static 2026-08-23 → 09-02 but re-verify). — CONFIRMED byte-identical to the verdict pin.
- [x] Read the 4 unread-at-verdict files before authoring any entry that cites them: `QwenRuntimeMTPDriver.swift`, `DFlashBatchedEngine.swift` (honest zero), `SchedulerV2.swift`, `pagedattention.metal` pass-B body (+ `CompiledDecodeV2.swift` body if R6/R4 quotes reach there). — Done via 2 parallel verification lanes (appendix in Research 120).
- [x] Dedup re-verification at mine time (both vocabularies per candidate): donation/slice-update/COW (expect zero), fence-token-ordering (expect zero vs CUDA-fence hits), Metal residency-set (expect zero vs register-residency hits), head-split register-budget (expect zero vs B44/B07/B33), command-buffer byte budget (expect zero). — All confirmed zero-relevant-hits; fence hits are CUDA-side (B92/B94/B97) + B70 counters, all cross-refed in-rule.
- [x] Author R1 `functional-cache-versioning-loses-donation-under-kernel-read` (D) — the GOAT. — LANDED, category corrected D->A (its siblings append-cache-rewind + kv-window-ring-append live in A); 23-layer correction + throttle rider + >=2-layers fixture corollary folded.
- [x] Author R2 `fused-kv-write-single-writer-reads-newest-from-input` (A/D) — single-writer-per-byte + newest tile loaded from the `knew`/`vnew` INPUTS (bit-identical bytes), ring-alias impossibility argument; race-free without cross-threadgroup primitives. — LANDED in A.
- [x] Author R3 `in-place-write-through-input-emits-fence-consumer` (D) — hazard tracking at declared-buffer granularity makes in-place writes through "input" declarations invisible; fence token (prev+1, chained) consumed by every subsequent reader = scheduling order + memory barrier; decode arm rides graph edges. — LANDED in D, + the <8-element CONSTANT-address-space rider + the wfence consumer evidence.
- [x] Author R4 `residency-set-wired-once-post-warm-persistent-footprint` (D) — capacity zero until wired → driver re-establishes residency per command buffer; wire once post-warm at persistent footprint + page allowance; never end/shrink the ticket. Explicitly Metal-scoped in-rule. — LANDED in D, anchor re-verified at L202-212.
- [x] Author R5 `register-budget-head-split-gqa-reread-trade` (E) — HPT chosen so `HPT·D/32 ≤ 32` acc floats; GQA/HPT threadgroups; smem formula modeled in host eligibility + test-pinned; ~30% re-read trade for static eligibility. — LANDED in E, with the uncatchable-fatal/byte-for-byte host-model content folded + the test file confirmed (CBv2PagedEligibilityTests).
- [-] Optional R6 (owner call at batch time): command-buffer referenced-byte commit threshold (C+, config-shaped) — SKIPPED per the task's own skip clause: config-knob/env-var shaped (`MLX_MAX_MB_PER_BUFFER`), single framework; Lane A grep-confirmed its anchors are NOT in the metal/kernel-research files.
- [x] Rider candidate: B22 `warp-shuffle-intrinsics-for-sub-warp-reduction` += the Metal measured-cost note (`simd_sum` ~14.5 cyc / quad ~2× cheaper / shuffle tree can beat the builtin — §1.1). Confirm the source line at mine time before landing. — LANDED; source line confirmed §1.1 L57-60; the batch22 full-concept pin moved WITH the rider (the gate caught the change as designed).
- [x] Standing gates: `batch100_pseudocode_parses` (cfg(not(kernel_verify)) per the B70 convention) + rules_present + concepts_covered; fix any case-sensitivity needle misses (the B51 lesson). — All 3 landed + green first run (needles generator-verified as exact substrings pre-emit).
- [x] Count pins: tests.rs corpus-count 445→453 + expected-array D/E/A + sums + router.rs expert total + mod.rs doc line + category headers — walk ALL pin sites (the 18th-count-pin-site lesson); check for stale headers while there. — All walked: tests.rs (4 sites) + router.rs + mod.rs (doc line + 3 category comments + 3 list appends) + the batch22 concept pin; no other stale sites found (.docs/11_domains/kernel_opt.md carries no count).
- [x] Drop-row discipline: Qwen35GatedDelta (B95 instance), DFlash spec-decode (B71/B81/B84), MTP micro-patterns (B30/B25/B26/B27/B36 instances), evidence-ledger doctrine, window-as-offset, sinks-denominator — do NOT author any of these; ledger stays in Research 120. — Held; Lane B additionally confirmed B42-family + B38 positives + B25 ANTI-instance (full-logits download is challenge-contractual).
- [x] GOAT: kernel lib count-identical + default lib + clippy 0 both states + retrieval floors 14/14 (no re-pins expected); R1's measurement citations quoted from the doc, never paraphrased into fake precision. — kernel_opt 1262/0/1i, default 1012/0, clippy exit 0 both states, floors 14/14 HELD (bands identical).
- [x] Update `.research/120` status line → RECORD at batch landing; update AGENTS.md Batch 100 entry + queue snapshot "Last landed" line; docs corpus count if the .docs book carries one. — Research 120 -> RECORD + appendix; AGENTS.md + snapshot updated in the landing commit; .docs carries no count.
- [x] `rm -rf .raw/qwen-3.8-mtp-challenge` as part of the landing commit (a landed batch with a live .raw entry is an unfinished batch).
- [x] Commit+push on develop (`feat:` prefix), queue snapshot updated in the same commit.

## Non-goals

- The vendored `Vendor/mlx-swift` tree (upstream Apple MLX; CUDA backend unused by the ranked track).
- The challenge's scoring/anti-cheat/evidence-ledger design (benchmark doctrine, not perf rules).
- The leaderboard perf numbers (league intelligence — routed to perf-rematch opponent-watch, not the corpus).
