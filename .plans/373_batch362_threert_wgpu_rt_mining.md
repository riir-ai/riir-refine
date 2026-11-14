# Plan 373 — Batch 362: the ThreeRT wgpu-RT mining batch (Research 406)

**Status:** STAGED — plan-first; cargo half awaits a free batch window (the claim row lands when the lane starts; `- [ ]` open, `- [x]` done, `- [-]` defer with reason)
**Claimed:** zed-m3-b362-threert/m3, 1794643107, expires 2026-11-16T07:58:27Z (authoring half now; T5 GOAT + the T7 gate battery hold for a quiet read; queue-snapshot row is the live claim)

## Source + scope

`meshy-dev/ThreeRT` @ `10fdc7738835d5b4c19438141fe28009a1d66d6b` (MIT, pinned 2026-11-14, tip moves daily — re-pin at T0). Rust + wgpu 30 compute path tracer: HW ray queries + software-BVH fallback, WESL kernel variants, à-trous/SVGF denoiser, sealed driver pipeline cache. Verdict YES B+ (Research 406): nominal Batch 362, kernel_opt-led mixed batch (R6 may land rust_perf io). Eight candidates + drops in the research note's ranked table — that table is this plan's contract; re-grade only at T0 evidence.

## Tasks

- [ ] T0: re-clone + re-pin + T0 quote re-verification (every Research 406 quote byte-checked at the new pin; the repo moves daily). Also the two re-read rows: `view.rs`/`live/*` (grade the dynamic-resolution drop) and `merged.rs` body (the browser binding-budget gate — face-or-drop).
- [ ] T1: dedup re-verification at the landing HEAD (re-run every Research 406 grep; new staged leads landed since may collide — adjudicate in-row).
- [ ] T2: R2b face-or-rule resolve (FILL walk-terminating sentinel vs `sentinel-readiness-over-separate-flag` — walk-termination vs readiness signaling; decide one-rule-or-two with R2).
- [ ] T3: R1 `hw-rayquery-candidate-loop-loses-hits-metal` authoring (the Metal law + the live/offline determinism split + per-mode step budgets; fixture = the CPU ordered-walk model + the exhaustion arm; decide the single-rule-with-three-clauses vs split; category: dispatch or numerics at authoring).
- [ ] T4: R2 `sah-tree-depth-bounded-by-traversal-stack` authoring (depth budget = walker stack − shared levels; median-split depth-bounded fallback; the 42-vs-21 measured basis; fixture per bvh.rs's own test shape).
- [ ] T5: R3 GOAT cell `refit-slack-loose-tree-vs-rebuild` (original moving-boxes fixture on the Pareto validator; pre-registered bars: latency + alloc, G1 = validity-equivalence NOT bit-identity — if the validator demands bit-identity, land decision-content first-class and record the swap; two-run stability).
- [ ] T6: the swing rows — R4 à-trous/SVGF variance-guided denoise (suggestion-only) · R5 WESL flag-totality (face-or-rule vs gpu_35, mandatory both-directions adjudication) · R6 sealed driver pipeline cache (rust_perf io; adjudicate vs B345 trust-ladder + index_cache positive-instance) · R7 finite-screen-as-variant · R8 24-bit budget fallback (keep/drop).
- [ ] T7: gates + count pins (batch needles; fixtures standalone-compiled AND executed BEFORE entry; retrieval floors both lanes; clippy -D at the four postures; fmt gate; pins_manifest --write same commit; dual_allocation + numbering_gate; `.raw/ThreeRT` removed at close).
- [ ] T8: docs + commit + push (AGENTS + README + `.docs` heads if counts moved; queue-snapshot landing row; Research 406 status line updated to batch-362-landed with the same-commit landing record).

## Deferred (not this batch's work)

- [-] The seal-* adoption surfaces (picking / blocks-shots keep-analytic / riir-engine LOS / capture-diff ΔE / browser pacing): NOT corpus work — they ride the seal-sprints filing recommendation recorded in Research 406's lane-intel; the seal lane files it.
- [-] The voxel sub-engine and the QuickJS/runtime layer: own verdicts if wanted (not queued).

## Verification

T7's gate battery is the acceptance test; T5's pre-registered bars are the GOAT arbiter; the research note's status line carries the landing record with ref hashes.
