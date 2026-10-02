# Research 229 — mlxfast-bonsai2 wave-2 accept-wave distill (Batch 195 conversion)

**Status:** RECORD (Batch 195 landed 2026-10-02; corpus 717→726, commit `feat: batch 195`).

## Source + method

- Source: `Layr-Labs/mlxfast-bonsai2-27b-engine` @ `0c7c392` (MIT; the standing vendor clone
  `/Users/katopz/git/vendor/mlxfast-bonsai2`). NET delta `0c61273..0c7c392` — 25 commits, 22 files,
  +9150/−777 (the 09-30..10-02 accept wave; NO tools/ reorg this time — the B190 exclusion class
  measured absent).
- Method: plan-first (Plan 194, Protocol A step 0); 2 staging subagents over disjoint slices
  (MTP/draft/accept vs KV/layout/numerics); **delta-freshness check on every candidate** — quotes
  verified against `git diff 0c61273..origin/main | grep "^+"` before authoring, not just the tip.
- The freshness check EARNED its keep immediately: the staging's `gpu-wake-scalar-touch` lead was
  KILLED as a **B193 duplicate** — `seed-first-command-buffer-earliest-submission` (entries_gpu_60)
  already cites `ContiguousKVBackend.swift:27-44` (CBv2GPUWake) verbatim; the lead rode the delta's
  diff CONTEXT, not its + lines. The same check confirmed `lazy-admission-reservation-atomic-register`
  is **present at base `0c61273`** (3 `bytesReserved` hits at both pins) — landed with the honest
  "present at base, first surfaced this wave" provenance note instead of a delta claim.

## Landed (9 rules + 3 riders; `entries_gpu_63.rs`)

| # | rule | cat | one-line |
|---|---|---|---|
| 1 | `io32-count-pinned-text-narrowing` | O | 64→32-bit kernel-source-text narrowing, count-pinned + launch bounds; stock-text fallback on drift |
| 2 | `strided-view-offset-fit32-launch-proof` | O | the family's general member: prove arbitrary strided views fit 2^31 by the span sum, then dispatch |
| 3 | `write-only-touch-side-queue` | D | provably write-only touches on a second queue + serial side thread + owner-thread availability + bounded drain |
| 4 | `last-query-rect-projection` | B | q|gate projection on the consumed rows only via rect-partitioned packed matmul; bit-checked |
| 5 | `roundfuse-packed-embedding-gather` | C | codes/scales/biases gathers → one launch, pure-copy contract, per-geometry verdict cache |
| 6 | `enumerable-domain-bit-identity-admission-gate` | N | enumerate-to-ADMIT (2^16 bitwise) vs B115's enumerate-to-PATCH; load-time binary gate |
| 7 | `cross-queue-output-retention-until-completion` | E | allocator hazard tracking does not span unordered queues — retain until the CB event signals |
| 8 | `atexit-registration-order-shutdown` | D | teardown by registration order; shared-lock handoff refusal; bounded final wait (NEW law) |
| 9 | `lazy-admission-reservation-atomic-register` | D | max(actual, estimate) reservations + atomic check+register (the TOCTOU overshoot fix) |

Riders: `kv-first-append-unzeroed-alloc-offset-gated-tail` += the adopt/install contract + `ready = enabled && verified` gate (B179);
`dtype-native-source-read-with-shared-admission-predicate` += the Metal BF16 head-rotation pad-in-read instance (B176);
`prompt-continuation-draft-override-measured-min-match` (rust_perf) += the draft-vocab-cutoff honesty law
("The cutoff can change draft acceptance" — the record's own correction of an acceptance-neutral claim).

## Staged (the MTP-slice leads — next kernel slot, pending their own quote re-verification)

The MTP/draft/accept staging slice returned ~49 candidates at SUMMARY level (its detail pass was
truncated by the subagent's output budget); per the standing discipline (staged = quoted in the
snapshot, never landed unverified), the strongest are staged for the next kernel slot:
`exact-form-device-trial` (device-ranked re-timing of ported constants, bit-identical alternatives),
`pack-worthiness-shape-admission` (packed-format admission by weight shape, M5 Max measured table),
`lossless-11bit-repack-trial` (lossless re-encode, per-shape-class adoption),
`draft-tape-behavior-gated-trial` (timing trial requires proposal-identity vs a recorded tape),
`walk-parallel-edge-precompute` (dependency-free edge scoring, 80µs→13µs measured),
`first-layer-ahead-submission` (+69µs measured; SLICE_LAYERS recorded negative),
`deferred-block-release`, `alias-scratch-reuse`, `accept-packet-one-launch` + the MLX metalKernel
signature gotcha (E-class), `per-layer-verification-coverage`, the producer-fold family (7 fusions
with the standard fixture spread), `single-pass-continuation-scan`, the io32 local twins, the pack11/
pack12 arithmetic leads. Plus E-class hazards: the env-polarity migration hazard (opt-in→opt-out
flips default behavior fleet-wide), the gather wrap-semantics hazard, the `x.size <= Int32.max`
admission the delta added to BoundaryBlocks.

## Drops (recorded per the standing ledger discipline)

- `gpu-wake-scalar-touch` — **B193 duplicate** (the freshness check's catch).
- `prompt-swizzle-supertile` — measured **NEGATIVE +1.45%** in-source (the tile-order cache-locality
  experiment lost; keep the anti-lead beside B181's negative ledger).
- `verify-slice-third-boundary-experiment` — an OPEN experiment with a prior-negative sibling
  ("every extra command buffer at verify width has a cost"); mine only after its own verdict.
- The bank-pad removal in `PrismHadamardQwen35.swift:75` (`red[4*16*33]` → `red[4*16*32]`) —
  HAZARD-WATCH, not evidence: the +1 bank-pad word was removed with no comment and no measurement;
  correctness unaffected (warp-consecutive indexing + a barrier), perf direction unrecorded. If the
  corpus ever wants a bank-padding rule, this delta is the counter-example to interview.

## Lane intel

The wave's MTP/top-two lane rides DFlash2 spec-decode — riir-ai Issue 1011's decode-axis watch may
tick again on the next scored build. Detection-only from this repo (no bench run; Plan 337 + the
M3 light-work rule).

## Gates (all green)

batch195 parse gate (9) + contracts gate + riders gate · kernel slice 396/0 · lib default 2423/0 ·
clippy `-D` clean ×3 postures (default / kernel_opt / rust_perf+kernel_opt) · retrieval 21/21
floors HELD (234s) · score-bench + freeze NOT owed (kernel_opt-only, zero clippy-corpus delta, P30).

## Quote re-verification pass — DONE 2026-10-02 (shikuwa/4090)

Two read-only staging agents over disjoint slices + the coordinator; every quote taken AT the pin
`0c7c392` via `git show 0c7c392:<path>` (the clone HEAD has moved past the pin to `2471492`; the 25
delta commit messages are generic "Accept/Validate submission" GUIDs, so every quote below is
IN-SOURCE). Tally: **12 verified / 2 corrected / 3 killed / 0 not-found.** Process note: one agent's
`git grep` on the clone triggered an auto-gc race (unlink warnings, concurrent pack locks) — switched
to `git -c gc.auto=0 show <pin>:<path> | grep`; nothing written to the clone.

### Batch-ready (verified, new-rule-grade)

- `deferred-block-release` (D) — `Vendor/mlx-swift-lm/Libraries/MLXLMCommon/ContinuousBatchingV2/MTP/EngineLoopV2+MTPFinalize.swift:4`: "Its graph used to be torn down when `finalizeMTPRound` returned, between the drafter's submission and the next verify build, on the host path the GPU waits on (about 0.1 ms of array releases on an M4 Max). It is held here instead and released at the end of the engine step… only the release moves." Env `MLXFAST_DEFER_BLOCK_RELEASE=0` (:16). Dedup: clean (distinct from cross-queue-output-retention and kv-first-append).
- `accept-packet-one-launch` (C/D) + metalKernel signature gotcha (E rider) — new file `.../MTP/CBv2AcceptGlue.swift:27`: "The comparison, the scan, the sum, the anchor's gather and the confirmed count's add were five launches over 31 ids; one thread now counts the leading matches m and writes the anchor (target m) and the confirmed count m + 1." (the staged "31" = 15 drafts + 16 targets, NOT an API arg limit); gotcha `:73`: "The grid attribute keeps MLX's generated signature well formed: with shape/stride buffers, two outputs and no attribute it closes the parameter list after the first output." Kinship: argmax-epilogue-replaces-output-materialization (B182) — same accept-packet vocabulary, different seam.
- `single-pass-continuation-scan` (O/D) — `.../MTP/CBv2PromptLookupDraft.swift:626`: "Each continuation endpoint contributes its longest matching suffix. Shorter suffixes add endpoints but cannot remove existing ambiguity… Thus only the global longest match needs selection." The delta REPLACES base's two-phase scan (miss-proof pre-scan + per-length stride loop) with the single endpoint pass. No corpus hits.
- `per-layer-verification-coverage` (N) — `Qwen35A3BTargetVerify.swift:6486,6585,6593`: "return verdict == true && covered.contains(ObjectIdentifier(layer))" — admission is PER-INSTANCE: a global self-test pass admits only the layers that individually had 0 mismatches; untested layers keep the chain (`covered` has no base hits → NEW). Distinct axis from enumerable-domain-bit-identity-admission-gate (exhaustive input domain) and construction-identity-row-classes-replace-probe.
- env-polarity migration hazard (E) — TWO flips, both NEW-in-delta: `MLXFAST_VERIFY_TOKEN_STACK` opt-in→opt-out (`EngineLoopV2+MTPTargetVerification.swift:18`: base `return ["1", "true", "yes", "on"].contains(value ?? "")` → pin `return !["0", "false", "no", "off"].contains(value ?? "")`, doc gains "Default on;"); `MLXFAST_DFLASH_SWIGLU` (`DFlash2Draft.swift:8398`) opt-in→opt-out — and the flip makes the CODE match its own doc (base doc already claimed "Default on (P6)" while base code was opt-in). Full ~40-env audit of the delta: exactly these two `-/+` pairs; the new envs (`MLXFAST_SEEDGAP`, `MLXFAST_P11`, `MLXFAST_IO32_GDN`, `MLXFAST_DW1`, …) are born opt-out default-on, not flips. No landed env-polarity rule exists — candidate hazard/annotation rule.
- `pack-worthiness-shape-admission` (N) — `DFlash2Draft.swift:3277-3288`: "On an M5 Max, chained launches over the real weights … packed against the stored layout: gate|up -20%, down_proj -3%, fc +1%, the q|k|v stack +2% (32 rows +1%), o_proj +7% (+8%), and the tap projections +63% (+64%)." + `packAll || cols == 16 || n >= 16384 || k >= 16384` (`worthPacking(n:k:)`, :3287-3288) + the 16-row counter-table (:3282-3285). No landed pack-shape admission — clean.
- `lossless-11bit-repack-trial` (B/N) — `DFlash2Draft.swift:3978-3994`: "a class takes the 11-bit form only when its faster kernel's median chain is more than `adoptMargin` faster, and runs that kernel" + "The trial alternates the two exact forms over `trialRounds` chains; a 0.5% median win is enough because both forms compute the same bits." (adoptMargin = 0.005, trialRounds = 16, env `MLXFAST_P11=0|force`, per-shape-class verdict log :2828-2830). Distinct from B181 decode-staging and B195's exhaustive-domain gate (operand+synthetic sampling vs enumeration).
- producer-fold family (C, 7 fusions) — ① WALKROWS `DFlash2Draft.swift:5663-5667` ("seven launches whose outputs only the edge kernel reads"; `MLXFAST_ROUND_NEXT=0` escape); ②③ ROUNDFUSE2 `:7700-7714` (key-mask fold: "`arange(keys) .< (bound + c)` … is `arange(-bound, keys - bound) .< c`: the bound is folded into the host range" + anchorJoin cast0 twin); ④⑤⑥ PROD 1/2/3 `Qwen35.swift:7420-7422` ("`(a * sigmoid(a)) * b` (SwiGLU), 2 `a * sigmoid(b)`…") + producer-formed-in-rotation-read `Qwen35A3BTargetVerify.swift:3839-3841` (measured :3871-3874: "M4 (6144 wide): -8% … -3 us per call … the SwiGLU one read 19% slower" — the honest negative rides the rule); ⑦ v-fold `:4572-4574` (fresh scan reads the value columns' conv input).
- pack11/pack12 encode arithmetic (B) — `DFlash2Draft.swift:3434-3437` `dflash2_pack12_code`: "1-15, or 0 (an escape)" — 8-bit sign|mantissa + 4-bit code = 12-bit class; encode `:3509-3511`; pack11 twin `:4158-4161` (3-bit code); decode `:4050-4088` ≈ an instance of tensor-core-mma-staged-quantized-codes-double-buffered (B181) — the ENCODE half is the clean new content.
- gather wrap-semantics hazard (E) — `DFlash2Draft.swift:5667-5670`: "the anchor's int32 id wrapped from the end as MLX's gather wraps a signed index" — the WALKROWS fold must reproduce MLX gather's signed-index wrap or paths change (guarded by "every edge bit for bit and every path id", :5673-5675). Same MLX-gather class as roundfuse-packed-embedding-gather's negative-id wrap, DISTINCT site (walk edge codebook gather).

### Fold-instances (verified — land as instances, not new rules)

- `exact-form-device-trial` → instance of `whole-round-in-situ-adoption-trial` (B182 rust_perf): `Qwen35A3BTargetVerify.swift:5032-5082` — "every launch form the record's tile passed, bit for bit against the stock launch, three dtypes", raced at the real prompt shape; `roundsPerArm = 8` / `adoptMargin = 0.005` / warm rounds untimed (`roundsNeeded = 2 + arms.count * (1 + roundsPerArm)`); 4 items (GDN DVPL rows/lane, rotation TPB, boundary blocks, prompt row-tile) with per-item kill switches. Same interleaved-rounds decision content as B182; the device-form axis is the instance delta.
- BoundaryBlocks `x.size <= Int32.max` admission → element-count instance of the fit32/io32 family: `Qwen35A3BTargetVerify.swift:3801-3803` — "guard active, width % 1024 == 0, width > 4096, width <= 7168, x.size % width == 0, x.size <= Int(Int32.max)" (BoundaryBlocks PRE-EXISTS; the guard is NEW); twin `DFlash2Draft.swift:7530` `anchor.size + masks.size < Int(Int32.max)`.

### Corrected (mechanism real, staged wording wrong)

- `draft-tape-behavior-gated-trial` — PRESENT-AT-BASE: the delta only MOVES the enum (base `DFlash2Draft.swift:2392` = pin `:4560`, text identical: "A variant is adopted only if every one of its rounds proposed exactly the tape's ids from the tape's position, the pick's own rounds matched the tape as well, and it beats the pick; otherwise the pick stays."). Landable only with the provenance label, never as wave-2-new.
- `first-layer-ahead-submission` — the 69µs half is NEW + verified: `DFlash2Draft.swift:7258` — "On an M4 Max it commits the block's first command buffer 69 us sooner after the readback (median of 39 rounds). `MLXFAST_DRAFT_FIRST_LAYER_AHEAD=0` submits the leading layers together, as before." (enum `DFlash2DraftSubmission`, :7247); the SLICE_LAYERS-negative half is PRESENT-AT-BASE (base `DFlash2Draft.swift:4677` "off by default here: submission slices lengthened the window on this lineage", `Qwen35.swift:246`); the delta's third verify boundary (`[8, 24, 40]`, `Qwen35.swift:243`) is the already-dropped open experiment.

### Killed

- `walk-parallel-edge-precompute` — walkParallel/edgeKernel/`MLXFAST_DFLASH_WALK_PARALLEL` PRESENT-AT-BASE (`0c61273` `DFlash2Draft.swift:3421-3445`); the staged "80µs→13µs" found NOWHERE in the delta (full-delta greps for 80/13/µs/measured over walk/edge contexts: no timing number); the delta's only new walk content is WALKROWS (verified in the producer-fold family above).
- `alias-scratch-reuse` — the only ALIAS in the delta is Pack11's WITHIN-launch threadgroup scratch (`red` partials in simdgroup escape scratch, 9 KB vs 12/15 KB at four splits — "the trial times both"), not cross-step reuse; closest cross-step mechanism = the tape-gate reuse (`Qwen35A3BTargetVerify.swift:650`: "Reuse this tape's verify gates when their layout fits the staged read.") — an UNSTAGED candidate for the next stager.
- io32 local twins — DUPLICATE: the landed B195 rules `io32-count-pinned-text-narrowing` + `strided-view-offset-fit32-launch-proof` were mined from this very delta (Hadamard.swift anchors); the twin sites (`DFlash2Draft.swift:2315` `"IO32", n > 0 && n <= Int(Int32.max) / 32 ? 1 : 0`, `Qwen35A3BTargetVerify.swift:3229,3286`) are the same mechanism.

**Net for the queue:** 10 new-rule-grade batch-ready + 2 fold-instances + 2 corrected-with-labels; 3 dead. Queue order unchanged (behind B196/B197/B198).
