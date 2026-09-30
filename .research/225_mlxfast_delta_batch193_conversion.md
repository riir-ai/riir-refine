# Research 225 — Batch 193: the mlxfast-bonsai2 post-reorg accept-wave conversion (Plan 190)

**Status:** RECORD — Batch 193 landed 2026-09-30 (+14 kernel_opt rules, 689→703; +2 kernel_opt riders on B178/B180 hosts; +1 rust_perf family-host amendment; 4 drops + 5 folds adjudicated).

## Source + method

`Layr-Labs/mlxfast-bonsai2-27b-engine` @ `0c61273` (MIT; the standing vendor clone `/Users/katopz/git/vendor/mlxfast-bonsai2`). NET delta `831fae7..0c61273` (46 commits, 41 accepts). The arena was REORGANIZED 2026-09-28 (merge `3c56d01`: tree reset to starter + Seatbelt sandboxing, ~2500 lines of tools/ infra — EXCLUDED per the staging verdict); the B181/B182 content re-landed identically and vanishes from the net diff. Net-new ≈ 3,700 engine lines.

Method: the staging pass (verdict AGREE r2, `.distill/001` L1-11) graded the net diff via 2 subagents; THIS conversion re-authorized via 2 fresh authoring subagents over disjoint slices (submission/scheduling/KV vs kernel/numerics/layout), every load-bearing quote re-verified VERBATIM at the pin (`git show 0c61273:<path>` — the clone is a promisor/partial clone, `git grep <pin>` fails), + coordinator spot-verification of the adoption-family sites.

## Staging-claim corrections found at conversion (the B192 pattern, 4 instances)

1. **File paths**: the staged paths `Sources/Libraries/MLXLLM/Models/*.swift` do not exist at the pin — the real tree is `Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/*.swift` (+ `Vendor/mlx-swift/Source/MLXNN/Hadamard.swift`, `Vendor/mlx-swift-lm/Libraries/MLXLMCommon/ContinuousBatchingV2/`). All CITEs use the real paths.
2. **`Hadamard.swift` is NOT new** — 2,525 lines at the pin; +218 is this window's net (FewRowPackedMatmul + drafterHeadInt8 + the plane-route hook).
3. **`headTop2` (the fused top-2 splice) is pre-existing at the START pin** (36 refs at `831fae7`, zero net-new lines) — the anchored-splice candidate's presumed mechanism was B181/B182-era, not this window's content. This killed candidate #8 (anchored-splice) as OWNED by B189's clause.
4. **The adoption-family host is `rust_perf/corpus/entries_10.rs:56`** — NOT `entries_gpu_50.rs` (that file carries only a rider mention of the rule). The amend target was corrected before editing.

## The landed 14 (categories in parentheses)

**Slice A — submission/scheduling/KV (7):**
1. `first-command-buffer-fresh-buffer-mapping-split` (D) — a drained allocator's first CB maps all-fresh buffers with the GPU awake-but-idle (7.4 ms M4 Max vs 1.1 ms warm); split layer 0 after q|k|v so part 1's execution hides the rest's mapping.
2. `kv-first-append-unzeroed-alloc-offset-gated-tail` (A) — first KV append = ONE launch; both zero-fills elided; the safety instrument IS the rule (every view is offset-gated `..<absoluteOffset`, so the unwritten tail is structurally unreachable; first-use bit-for-bit verdict across FP32/FP16/BF16 keeps the fills on mismatch).
3. `deadline-bounded-poll-of-submitted-completion-event` (D) — poll (don't sleep on) an already-submitted completion event on the critical path; 0.25 s deadline guards never-submitted paths ("after a quarter of a second the poll falls through to the blocking read").
4. `full-accept-state-store-host-observable-predictor` (G) — verify conditionally stores the window's final state keyed on a HOST-observable predictor ("does not count, since the host cannot tell"); + the hold-speculation RIDER (`holdsSpeculation`/`lookupPreempts` share the same predictor).
5. `readback-domain-excludes-cold-tap-copies` (D) — every array in one evaluation shares its completion event; the cold 26 MB tap concat rides the LATE submission of the work that consumes it.
6. `adoption-staged-leading-layers-gpu-idle-fill` (D) — a row class that can't run its lead pre-readback keeps hidden state after its leading layers; adoption submits as soon as writes install — filling the GPU idle between verify end and adoption.
7. `seed-first-command-buffer-earliest-submission` (D) — the 3-arm seam rule: dependency-free no-op wake at assembly ("Nothing reads it"), deferred freezable host reads (0.5-0.6 ms off the pre-first-CB path), the embedding submitted at the graph top with a memcmp ids-identity take gate ("a scheduling hint, never a change of any value").

**Slice B — kernel/numerics/layout (6):**
8. `construction-identity-row-classes-replace-probe` (N) — BF16-output SAMPLING probes cannot certify kernel bit-identity (7/43 blocks diverged via accumulation-order differences, route-off); row classes hold by construction (padded one-product or per-count). The measurement-honesty family's CERTIFICATION member.
9. `redundant-row-reduction-per-width-block-grid-split` (E) — 16 TGs @ 1024 thr → 80 @ 256 (row × 1024-block); bit-identity via virtualizing the 1024-lane reduction (the real simd_sum IS the virtual simdgroup), one barrier.
10. `branch-free-top2-insert-order-invariant-guard` (B) — increasing-id scan + id-ascending tie order license 2 compares + selects, guarded by sticky `count == 2 && !isnan(local.second)`; bit-for-bit self-test incl. ties/signed-zeros/NaN/all-NaN.
11. `register-built-right-operand-lane-contiguous-plane-layout` (L) — weight permuted once into lane-contiguous planes → each lane builds its MMA B-operand in REGISTERS from two uint4 (zero staging/shuffles/barriers); ADMISSION REFUSES without the prompt route's plane-copy byproduct ("rb passed but the prompt route builds no plane copy here: not in the trial"); head-EXCLUDED at 248320-wide (`!best.variant.rb`). Prefetch rider included (370.3 vs 373.5 ms).
12. `dual-layout-transpose-at-stage` (A) — K staged in BOTH layouts (Ksh + KTsh) from ONE register read; the /8 grouping flips with core count (kt/8 −11% M4 Max 32-core vs +9% 40-core part) → load-time trial (bitwise + margin 0.02 + confirmation), never a pinned pick.
13. `plane-form-activation-reuse-topology` (A) — the SGN/CT/AT consumer-sharing axes of an activation slice (core-cache residency vs register-paired tiles vs tgmem staging; cols = 32·sgn·ct, sgn·ct ≤ 16) picked per shape by a double-bitwise-gated streaming trial (adopt 0.02 / drop 0.05). The coin-flip verdict: STANDALONE — see below.

**Coordinator (1):**
14. `adoption-trial-rotate-operands-per-launch-dram-streaming` (D) — the family's OPERAND-STREAM axis: cached-set bursts rank variants the other way than DRAM-streamed deployment (k32pd1 ahead of k32pd2 on all 5 tower shapes streamed — down 52.2 vs 54.1 µs etc. — behind cached); the remedy rotates REAL operand sets per launch ("each launch on the next of the shape's projections so the words stream from memory as in a forward"); the base is window-measured (`..._TZOO_BASE`); the bit gate is the double-reference form and must hold by CONSTRUCTION (never a sampled probe).

## The coin-flip adjudications (staged conversion duties)

**`plane-form-activation-reuse-topology` vs `narrow-m-l1-panel-sizing` (B186) — STANDALONE.** B180 owns the activation BYTE layout (planes so per-thread loads align; composes); B186 owns the K-extent FORMULA from an L1 budget for thin-M. The new mechanism is a THIRD axis: how many CONSUMERS share one activation slice (SGN simdgroups on core-cache residency with zero barriers / CT register-level second consumer / AT explicit tgmem arm) + the bounded fan-out (32·sgn·ct ≤ 16) + the per-shape streaming trial. Closest kin `multi-simdgroup-shared-staged-tile` (B21) stages by construction — here staging is one ARM of the pick, not the mechanism. Distinct-from chain in-rule: B21, B01, B180, B186, B111.

**`drafter-head-rides-verify-int8-via-exact-widening` vs `exact-widening-enables-fp16-pv-accumulation` (B178) — FOLDED as a RIDER on the B178 host.** B178's own text claims the class ("instantiates on any silicon wherever a packed dtype widens losslessly"); BF16→FP32 (always exact) licensing a ROUTE to the int8 verify head is squarely that class, with one new guard: the lossy-surface risk is bounded by verification ("never an emitted token" — the target verifies every draft). The rider sentence landed on the B178 entry (544 vs 305 GB/s head-row streaming, source-reported).

## The instance-or-extension reads (staged conversion duties)

**`5244ceade` (PrismML-Eng/llama.cpp #221) vs B180's planar family — INSTANCE, rider landed.** The fork's commit "Add: planar-transposed activation layout for the PTQ1_0 mat-vec path" is the SAME lineage (same 9-plane layout, same block_q8_1 stride law, same aligned-load count; B180's own CITE is the same `mmvq-ptq1_0.cuh` file family). Two genuinely NEW atoms beyond B180's text, landed as the B180 rider: (1) the unsigned-offset decode spelling "(q + 0x7F7F7F7F) ^ 0x80808080 instead of __vsub4"; (2) the SUMMATION-ORDER STABILITY clause — "nwarps is pinned to 4 for every column count so the K partition, and with it the fp32 summation order, does not depend on the batch" (determinism pins the warp count even where another might time faster) — plus the regime note "HIP keeps the old layout". No new rule.

**`18bbc46` (ggml-org/llama.cpp #29602 "metal: FWHT perf optimizations") vs the B116/B176 ladder — CONSUMED, nothing landed.** The commit is a 3-line diff: `GGML_METAL_FWHT_TG_MIN_N 1024 → 512` + moving the 512-width template from the simdgroup to the threadgroup form (both dtypes). That is LITERALLY the boundary the B176 rider already records ("64-256 simdgroup / 512-8192 tg-gated", dated). No new mechanism (no exchange-network change, no barrier change). Side-note: the author is `bri-prism` — the fork upstreaming its own metal work, consistent with the rider's fork-origin story.

## The rust_perf family-host amendment (entries_10.rs `whole-round-in-situ-adoption-trial`)

Three corrections, all carried verbatim in the amended context_notes + name + fixed_code:
1. **The margin**: adoptMargin 0.005 → 0.015 — "the record's pick is now a base the box's own window measured (`..._TZOO_BASE`), and a same-box reading showed the trial's picks can be slower than such a base" (the b497a150 reading put its window 1.85% under the trial's own picks with identical acceptance). The pinned `ADOPT_MARGIN_PCT` in fixed_code moved 0.5 → 1.5. AMENDED, never a competing threshold (the staging verdict's instruction).
2. **The premise**: "correctness-free because every candidate passed the bitwise self-test" is RETIRED — a SAMPLED self-test cannot certify bit-identity (the 7/43 datum); the gate must hold by CONSTRUCTION, and the source's own gate is the double-reference form (plane kernel bit-for-bit AND staged8 bit-for-bit).
3. **The regime axis**: rounds must run over ROTATED real operand sets (the DRAM-streaming deployment regime) — cross-referenced to the new kernel_opt rule #14 which owns the axis.

## Folds (5) + drops (4) — the adjudication ledger

Folds: deferred-host-read → arm 2 of rule 7 · prompt-embed-early-submit → arm 3 of rule 7 · dependency-free-prefix-gpu-wake → arm 1 of rule 7 · hold-speculation → rider on rule 4 · adoption-base-measured-in-deployed-regime → the rust_perf amend + rule 14's base half.
Drops: `staged-fp32-splitk-float4-runs` (thin composite — the double-buffered staging owned by B181, the split-K gate by the splitk family, cooperative float4 loading by B21's family) · `anchored-splice-lower-bar-risk-bounded-by-prefix-agreement` (OWNED by B189's clause + the mechanism pre-existing at the START pin) · `few-row-m-amortized-weight-streaming-in-fragment-dequant` (all atoms owned: the dequant is the third dtype instance of the lop3/prmt magic-constant recipe; the epilogue by integer-mma-span/dp4a; M-amortization by row-tile-amortize-input-load) · `CheckpointShardList.swift` (trusted-module security split — the Seatbelt reorg, not a kernel mechanism).

## Anti-lead ledger (verified source numbers — research-note material, never corpus)

- **Double-buffered staging in the GDN fresh scan REJECTED: +28% M4 Max** — "Double-buffered staging (one barrier per chunk) and v / decay-factor prefetch were measured and lose (M4 Max: +28% and +0-7%), so they are not offered" (Qwen35A3BTargetVerify.swift:3686-3688). THE standing counter-example against B181's double-buffered-staging rule being read as a family default (cited in rule 12's kinship).
- **v/decay-factor prefetch: +0-7%, rejected** (same line).
- **kt/8 core-count divergence: −11% (M4 Max 32-core, 48 heads) vs +9% (40-head part on a 40-core)** — the regime divergence IS rule 12's content; no vendor key separates the two boxes.
- **RB head exclusion + admission refusal**: family "every tower shape but the head"; "rb passed but the prompt route builds no plane copy here: not in the trial" — all in rule 11.
- **gate|up exclusion 3% slower in situ** while "the record's few-row core streams it faster in isolation" (Qwen35MTPTopTwo.swift:671-676) — the in-situ-vs-isolated divergence class.
- **The unbounded-poll hang class** — the 0.25 s deadline is load-bearing (rule 3's safety content).
- **SwiGLU producer rotation 19% slower at prompt width / gated-norm −8%**: NOT found in this delta window — that datum belongs to an earlier window (B182's fused-transform-quantize-writer notes). Not re-claimed.

## Gates (all GREEN, 2026-09-30, isolated `/tmp/b193tgt`)

- batch193_pseudocode_parses + concepts_carry_contracts + riders_landed: 3/3.
- Kernel slice (`--features kernel_heal kernel`): 389 passed / 0 failed.
- lib default: 2398 passed / 0 failed.
- clippy `-D warnings` ×3 postures (default / no-default / all-features): CLEAN (1 doc_markback backtick fix in the new header en route — the B192 class).
- Retrieval floors (`--features kernel_heal,latent_retrieval`): 21/21 HELD, kernel 30/30 self-variants 100.0% ×3 columns (235.41s).
- Score-bench + freeze regen NOT owed (kernel_opt + one rust_perf AMEND — zero clippy-corpus delta, P30).
- Corpus pins: 689 → 703 (A 148→151, B 75→76, D 152→158, E 65→66, G 30→31, L 17→18, N 53→54); router expert-coverage pin re-pinned 703 with the honest label.

## Rider state + queue

The mlxfast delta rider was consumed by the staging (this batch is its product) and RE-ARMED for the next trigger (weekly floor 2026-10-04 OR any-new-accept at the next watch pass — the post-reorg accept rate is HIGH). TensorFold intake candidates queue behind. Clocks unchanged: nnethercote ~10-01, walk #13 ~10-05, soak read ~10-07.

NOTHING INVENTED — every quote read at the pin (staging + authoring + this conversion); the four staging-claim corrections are recorded above.
