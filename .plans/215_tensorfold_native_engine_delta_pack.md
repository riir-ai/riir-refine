# Plan 215 — the TensorFold native-engine delta pack (Research 251)

**Status:** STAGED — pending batch slot (Research 251 graded the c464617..f1aef9c delta YES B+; this plan stages the conversion across TWO slots per the B199/200 kernel-vs-serving split precedent)
**Date:** 2026-10-10
**Queue:** `.distill/001_mining_queue_snapshot.md` — the TensorFold weekly-catch-up row (this session) + the standing weekly cadence
**Source:** `ashhart/tensorfold` @ pin `f1aef9c3f2ab85b2a68f2eed817e45eecbd04781` (2026-10-10 tip, **Apache-2.0** from 0.6.0). Standing vendor clone `/Users/katopz/git/vendor/tensorfold` (ff'd to the pin this session); ⛔ the Python tree is REMOVED at tip — every Python-era quote verifies at COMMIT sha via `git show <sha>:<path>`, never at tip.
**Dedup baseline:** Batches 177 + 188 + 196 + 199 + 200 already landed from this source (22+ rules). Killed-by-baseline candidates are listed in Research 251 §"already landed" — do NOT re-stage them.
**Verification record:** Research 251 §caveats — coordinator verified 9 quote sites verbatim at the pin; everything else re-verifies at authoring. Lane-B unread list re-reads first.

## Slot 1 — kernel_opt pack (next kernel slot)

- [ ] T0 drift re-check: `git -C /Users/katopz/git/vendor/tensorfold fetch origin` + log `HEAD..origin/main` — if the tip moved, re-pin the new sha in THIS plan's header (a moved tip does not invalidate the graded candidates; it changes the pin the quotes verify against)
- [ ] T1 N3 `expert-member-rows-width-ladder-bit-identical` (A−/B+, Metal MoE experts) — re-verify at `zig/src/families/nemotron/layers.zig` + `nemotron_experts.metal` + commits `2985ed7`/`6cb5374`; carry the measured ladder + the NULL point @2 rows + the three in-rule negatives ("member blocks over grid z / activations once a forward / vector loads"); distinct-from gpu_09-family + B198 fewrow + B213 #30087 + B214 #225 per-path axis
- [ ] T2 N4 `zero-anchored-prompt-grid-chunk-identity` (B+) — re-verify `cuda_prompt_grid.zig` verbatim (header + `end`/`resumable` already verified 2026-10-10); distinct-from `prefix-cache-entry-one-token-early` + `concurrent-round-parity-solo-equal` (both B200)
- [ ] T3 N6 `verified-logits-derived-outputs-path-independence` (B+) — re-verify `logprob.cu` (verified verbatim 2026-10-10); carry the 2e-6 float64 oracle + the 7% cost disclosure
- [ ] T4 N12 `hip-affine-contract-off-row-invariance` (B−/C+) — re-read `zig/kernels/hip/affine.hip` + `zig/src/hip/affine.zig` (lane-B unread → full read owed); the `-ffp-contract=off` M-independence atom; adjudicate I-category vs quant-format at authoring
- [ ] T5 N13 `staggered-segments-two-queue-fences` (B−/C+) — re-read `zig/src/core/segments.zig` internals (lane-B unread); the fence law + the ≥4096-row gate + the 3-queue/4-queue fan-out bounds
- [ ] T6 riders — `row-cost-model-rebroadcast` (rides T1) · `prefill-head-final-chunk-only` (check vs landed B196–200 set FIRST) · `kv-independent-quantizer-verification` · `exl3-reference-oracle-fixture` (I-category; cite fixture, never embed — checkpoint-derived bytes)
- [ ] T7 gates — batch215 parse/contracts/riders gate · kernel_opt lib floor (≥ current) · default lib == baseline · clippy `-D` ×3 postures · retrieval floors (kernel_opt-only batch: NOT owed per P30 unless rust_perf joins the slot) · freeze + score-bench NOT owed (zero clippy-corpus delta, P30)
- [ ] T8 count pins — corpus count + category table + tests.rs + router + AGENTS.md ×2 + README ×2, re-pinned honestly
- [ ] T9 research-note status line + queue-snapshot row for the slot landing

## Slot 2 — rust_perf serving/timing pack (the following slot)

- [ ] T10 N1 `fastest-of-n-dip-drift-retime-cost-table` (A−) — re-verify `cost_rule.zig` (verified verbatim 2026-10-10: reps=7/dip=0.03/drift=0.15 + build-identity persistence); the NotBenchable decision-content posture (B136 shape)
- [ ] T11 N2 `pava-monotone-cost-pooling-modeled-cost-not-clock` (A−) — re-verify `draft_depth.zig` (PAVA test verified); the rank-identical-decision atom
- [ ] T12 N5 `prompt-cache-canonical-interval-keys-min-gap-rewind` (B+) — re-read `prompt_cache.zig` Rules/lookupRewind in full (header verified; body unread); split interval-keys vs rewind/eviction if the entry overgrows
- [ ] T13 N7 `thermal-hysteresis-band-pacing-chunk-placement` (B) — re-verify `heat.zig` (verified); NEW thermal family
- [ ] T14 N8 `degrade-not-fail-load-qualification-probe` (B) — recipe-sourced; verify the qualified envelope + the two failure classes
- [ ] T15 N9 `prompt-state-fingerprint-probe-bits-keyed-persistence` (B) — re-read `learned_prompts.zig` body (lane-B unread)
- [ ] T16 N10 `metal-keepalive-residency-unwire` (B−) + T17 N11 `display-carveout-all-or-nothing-admission` (B−/C+) — recipe-sourced, verify at the named files
- [ ] T18 riders — `bounded-peer-handoff-wait` (10 s, #512) · `nvfp4-per-sm-w4a4-w4a16-dispatch` (arch-keyed + the per-family lossy-quality table) · transport-lessons cluster (spin-lock µs / MTLFence / simdgroup tile) as one entry or drops, slot session adjudicates
- [ ] T19 gates + count pins + retrieval floors (rust_perf joins → floors OWED) — same gate set as slot 1 with the rust_perf slice
- [ ] T20 research-note status + queue rows + commit+push (the idle-loop one-unit-per-commit discipline per slot)

## Standing (this session, done)

- [x] Vendor clone ff'd to the pin; pin recorded in Research 251
- [x] Research 251 landed (the verdict note)
- [x] Queue-snapshot row + the WEEKLY catch-up cadence line
- [x] This plan (plan-first, before any corpus edit)
