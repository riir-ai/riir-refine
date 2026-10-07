# Plan 200: Batch 201 — the mlxfast MTP-slice leads (kernel slot)

**Status:** COMPLETE 2026-10-07 (shikuwa/4090, the resuming session) — 12 rules landed (slices A/B/C, 2 authoring subagents over disjoint slices + the coordinator; every quote byte-verified at the pin `0c7c392`, ~15 staged line drifts corrected in-rule, the "ContinuousBatchingV2/MTP/DFlash2Draft.swift" staged path does not exist at the pin — DFlash2Draft.swift lives under Models/, corrected in-rule) + 2 riders (T4.1 rust_perf exact-form device trial + T4.2 kernel_opt BoundaryBlocks element-count/anchorJoin fit32 twins); gates batch201_pseudocode_parses 12 + batch201_contracts 12 + batch201_riders 2 GREEN (kernel_opt lib 2879 passed at kernel_opt, 2467 at default); clippy `-D`-class clean ×3 postures (default / kernel_opt / rust_perf+kernel_opt); router 746→758 re-pin + tests.rs 5-site pin + per-category count table +6 rows; score-bench + freeze NOT owed (zero clippy-corpus delta, P30); .raw/mlxfast-b201 removed at close.
**Date:** 2026-10-03
**Queue:** `.distill/001` — the MTP-slice leads (12 verified, Research 229 §quote-re-verification) = the next kernel slot; the 10-03 fork pool stages behind
**Source:** `Layr-Labs/mlxfast-bonsai2-27b-engine` @ `0c7c392` (MIT) — the SAME pin as B195's wave-2 delta `0c61273..0c7c392`; ephemeral clone `.raw/mlxfast-b201` (fresh clone HEAD == the pin; remove at close)
**Verification record:** Research 229 §"Quote re-verification pass — DONE 2026-10-02 (shikuwa/4090)": **12 verified / 2 corrected / 3 killed / 0 not-found**; process note — use `git -c gc.auto=0 show <pin>:<path>`, never bare `git grep` (the auto-gc race)

## Composition (12 new kernel_opt rules + 2 riders)

Slice A → `entries_gpu_70.rs` (engine-loop/verify, D×2 + N×2):
- [x] T1.1 `deferred-block-release` (D) — EngineLoopV2+MTPFinalize.swift:4 + env :16; dedup vs cross-queue-output-retention (B195) + kv-first-append (B179)
- [x] T1.2 `first-layer-ahead-submission` (D) — DFlash2Draft.swift:7258 (69µs NEW half; the SLICE_LAYERS-negative half is PRESENT-AT-BASE — land with the provenance label, never wave-2-new)
- [x] T1.3 `draft-tape-behavior-gated-trial` (N) — PRESENT-AT-BASE (base :2392 == pin :4560, text identical); land with the provenance label, never wave-2-new
- [x] T1.4 `per-layer-verification-coverage` (N) — Qwen35A3BTargetVerify.swift:6486,6585,6593; `covered` has no base hits → NEW; distinct axis from enumerable-domain (B195) + construction-identity (B182)

Slice B → `entries_gpu_71.rs` (pack/encode/accept, N×2 + B + C):
- [x] T2.1 `pack-worthiness-shape-admission` (N) — DFlash2Draft.swift:3277-3288; the M5 Max measured table + `worthPacking(n:k:)` + the 16-row counter-table
- [x] T2.2 `lossless-11bit-repack-trial` (N) — DFlash2Draft.swift:3978-3994; adoptMargin 0.005 / trialRounds 16 / `MLXFAST_P11`; distinct from B181 staging + B195 exhaustive-domain (operand+synthetic sampling vs enumeration)
- [x] T2.3 `dflash2-pack11-pack12-encode-arithmetic` (B) — :3434-3437 / :3509-3511 / :4158-4161; the ENCODE half is the clean content (decode ≈ B181 instance); pack12 = 8-bit sign|mantissa + 4-bit code, "1-15, or 0 (an escape)"; pack11 twin = 3-bit code
- [x] T2.4 `accept-packet-one-launch` (C) — CBv2AcceptGlue.swift:27; five launches → one thread (leading-match count + anchor + confirmed count); "31" = 15 drafts + 16 targets, NOT an API arg limit; kinship argmax-epilogue (B182)

Slice C → `entries_gpu_72.rs` (hazards/scan/folds, E×2 + C + O):
- [x] T3.1 `env-polarity-migration` (E) — TWO flips verified: `MLXFAST_VERIFY_TOKEN_STACK` (EngineLoopV2+MTPTargetVerification.swift:18) + `MLXFAST_DFLASH_SWIGLU` (DFlash2Draft.swift:8398, the flip makes code match its own doc); the ~40-env audit: exactly these two `-/+` pairs; no landed env-polarity rule exists
- [x] T3.2 `gather-signed-index-wrap-semantics` (E) — DFlash2Draft.swift:5667-5670; the WALKROWS fold must reproduce MLX gather's signed-index wrap ("every edge bit for bit and every path id" :5673-5675); same class as roundfuse's negative-id wrap, DISTINCT site
- [x] T3.3 `producer-folds-single-consumer-family` (C) — 7 fusions: WALKROWS (:5663-5667, `MLXFAST_ROUND_NEXT=0` escape), key-mask fold + anchorJoin cast0 (:7700-7714), PROD 1/2/3 (Qwen35.swift:7420-7422), producer-formed-in-rotation-read (Qwen35A3BTargetVerify.swift:3839-3841, the measured NEGATIVE rides the rule: SwiGLU 19% slower), v-fold (:4572-4574)
- [x] T3.4 `single-pass-continuation-scan` (O) — CBv2PromptLookupDraft.swift:626; the delta REPLACES the two-phase scan (miss-proof pre-scan + per-length stride loop) with the single endpoint pass ("only the global longest match needs selection")

Riders (coordinator, after slices land):
- [x] T4.1 `whole-round-in-situ-adoption-trial` (rust_perf entries_10.rs) += `exact-form-device-trial` instance — Qwen35A3BTargetVerify.swift:5032-5082 (roundsPerArm 8 / adoptMargin 0.005 / warm untimed; 4 items w/ kill switches; the device-form axis is the instance delta)
- [x] T4.2 `strided-view-offset-fit32-launch-proof` (entries_gpu_63.rs) += BoundaryBlocks element-count instance — :3801-3803 guard + :7530 anchor/masks twin (PRE-EXISTS, the guard is NEW)

## Wiring + pins (coordinator)

- [x] T5.1 mod.rs: `mod entries_gpu_70/71/72;` + `entries.extend(...)` ×3
- [x] T5.2 router.rs:472 expert-coverage pin 746 → 758 (+ honest label)
- [x] T5.3 corpus/tests.rs: total pin :111 746→758 "through Batch 201" · coverage comment :5369 · category-sum pin :5487 (+ delta note) · KERNEL_CATEGORIES rosters +12 rule ids · batch roster comment line
- [x] T5.4 gates: `batch201_pseudocode_parses` (12) + `batch201_contracts` (12 markers) + `batch201_riders` (2 hosts) in kernel tests; rust_perf rider gate for T4.1's host

## Gates

- [x] T6.1 batch201 parse 12 + contracts 12 + riders 2 · kernel_opt lib slice
- [x] T6.2 lib default · clippy `-D` ×3 postures (default / kernel_opt / rust_perf+kernel_opt)
- [x] T6.3 score-bench + freeze NOT owed (zero clippy-corpus delta, P30); retrieval floors NOT owed (kernel_opt + one rust_perf rider, the B195 precedent)

## Close

- [x] T7.1 commit + push; `.distill/001` landing row + queue-after; `.raw/mlxfast-b201` removed
