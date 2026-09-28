# Research 216 — mlxfast-bonsai2 DELTA distill verdict (Batch 182 / Plan 177)

**Status:** RECORD (batch landed 2026-09-28; the armed Research 212 Addendum 3 rider FIRED and was consumed)

## Source

`Layr-Labs/mlxfast-bonsai2-27b-engine` @ `831fae740de35a106e85768d8b0534b4af422b13` (MIT),
delta `cfb35b0d..HEAD` — the POST-B181 accept wave. This is the same source as Research 212 /
Batch 181, re-read at the new pin per the Addendum-3 armed rider ("trigger = any new accept OR
weekly floor 2026-10-04" — new accepts confirmed: the leaderboard moved 297.5% → 381.9%
(+28%) with the drafter STILL `stock DFlash 10.7 tok/round` on every promoted row, so the wins
are engine-side again — verify-path/cache/host-side, exactly where the delta's diffs sit).

Delta shape: 40 commits, 24 files, ~19k insertions — the MTP-verify / CBv2-cache /
steel-nax-kernel arc the 09-27 watch census named (LayerCacheV2, FullSequenceKV,
RecurrentStateV2, AttentionV1, EngineLoopV2+MTP*, MTPContractsV2, CBv2PromptLookupDraft [NEW],
steel_attention_nax [NEW], Hadamard.swift, the five model files). The 09-27 watch's "NEW model
files" wording was imprecise (PrismHadamardQwen35/Qwen35A3BTargetVerify/Qwen35MTPTopTwo all
EXIST at the pin `cfb35b0d` — as small stubs; the delta grew them into the 2.2k-5.5k-line
verify engines). The per-commit `--name-status` log was the authoritative census instrument
(the merged-stream path sort misleads — the watcher's own recorded lesson, re-confirmed).

## Verdict: YES — A− corpus / LANE INTEL HIGH (the Bonsai league lane, second consecutive
distill from the same moving source) / route kernel_opt + rust_perf.

Every quote re-verified VERBATIM at HEAD this batch (fresh clone at `831fae7`, the
ephemeral-clone law; `.raw/mlxfast-bonsai2-delta` removed at close). All perf numbers
SOURCE-REPORTED (M4/M5 Max class) — none measured on our box.

## Landed (Batch 182, entries_gpu_52.rs + entries_10.rs + 6 riders)

kernel_opt 649 → 655 (A 137→139, C 48→49, D 142→143, E 62→64):

1. `verify-fold-gqa-repeats-into-row-axis-tile-fill` (A) — GQA broadcast at L < tile-M widths
   folds repeats into the row axis ([B,H,L,D] → [B,kvHeads,repeats·L,D], a free view): 6 tiles
   at 25% fill → 1.5, each K/V tile read serves six heads; bit-identity by the fixed BK/SK
   reduction order (in-source contract); anti-regimes L ≥ tile M / repeats=1 / non-contiguous q.
2. `draft-block-simdgroup-kv-split-smem-softmax-exchange` (E) — one query block ≤16 real rows
   in the 64-row tile, no causal/sinks → 4 KV blocks one-per-simdgroup, (max, rescale, P)
   exchanged through threadgroup memory, each simdgroup consumes all 4 blocks in key order for
   ITS OWN head-dim fragments (id = 2·sgi); exactness in-source ("every value that reaches the
   output is the same operation on the same operands"); two barriers/round.
3. `simdgroup-redundant-small-reduction-over-barrier-broadcast` (E) — second stage ≤ simdgroup
   width → every simdgroup redundantly runs the identical ordered tree over the 32 partials
   instead of group-0-broadcast (2 barriers + a hop); companion: initialize only unused slots
   (disjoint stores need no barrier).
4. `argmax-epilogue-replaces-output-materialization` (C) — top-k-consumer matmul fuses the
   running top-k into the epilogue: [rows, k] instead of [16, 248320] FP32 = 15.9 MB/round;
   identity by JIT-splice-from-stock-text + a total order (value desc, lower id ties, NaN
   last); LAZY stock logits stay in the graph unevaluated; anchor-gated splice + load-time
   self-test. Benchable in our stack (alloc axis).
5. `verify-window-prework-loads-before-stores` (A) — a short-window kernel reorders EVERY read
   before ANY store, staged in per-thread register arrays; register-budget regime boundary
   (2·(2+GRP)·KS floats); the consumer-unread conv-tail ELIDED (it IS the conv input's last NK
   rows); identity "every formula is the stock kernel's, in its order" + construction-time
   bitwise self-test → verdict cache.
6. `idle-gpu-residency-restore-overlapped-behind-queued-work` (D) — an idle Metal GPU drops
   residency; the next command buffer pays ~0.1 ms/10 MB INSIDE the timed window (the
   bench-idle-gate posture; the seed-gather instance measured 4.8–8.5 ms on the 358 MB
   embedding table); touch kernels queued BEHIND early submissions (probe: 89.2 vs 89.4 ms
   queued vs ~5.5 ms each unqueued) + the host-side-first-read arm. Regime-gated Metal.

rust_perf 160 → 163 (all three NotBenchable, all annotated):

7. `host-mirror-shared-scalar-advance-one-time-verdict` (dispatch) — N layers advancing the
   SAME +L scalar = 16 one-element launches/verify-window → a host mirror advanced at the same
   code sites + ONE shared array + a ONE-TIME device-vs-host equivalence verdict (match ⇒ host
   path for the process, mismatch ⇒ device op forever); mirror-at-every-site is the
   single-source-of-truth invariant.
8. `prompt-continuation-draft-override-measured-min-match` (complexity) — override ONLY draft
   ids on a UNIQUE ≥16-token match (floor 8); the measured NEGATIVE at n=2..4 rank-merge
   (12→15 rounds) is the floor's evidence; SKIP the drafter block on the round after a
   from-prompt proposal; on-device-scored splice in the early rounds; the target still
   verifies everything.
9. `whole-round-in-situ-adoption-trial` (complexity) — the measurement-honesty family's
   ADOPTION member (B177 priced the COMPARATOR, B181 warmed the SEAM, this decides the
   MEASUREMENT UNIT + FLIP THRESHOLD): kernel-variant selection on WHOLE workload rounds,
   interleaved — first round discarded, >1.5× median dropped, >0.5% margin, ≥4 kept rounds,
   correctness-free because every candidate passed the bitwise self-test; pinned constants
   (adoptMargin 0.005 / stage2Slack 0.02 / outlierFactor 1.5 / minimumRounds 4) + the L18
   orthogonal-array per-shape sub-arm.

Riders (6): `fused-transform-quantize-writer` (narrow-producer fold at verify width +
fusedInverse + tensorPackedMatmulHead + leadingRows + the blocks-in-place zero-copy instance),
`engine-work-interval-performance-core-pin` (renewal 59 vs 82 ms + the never-start-on-unjoined
anti-regime, 13%/76% measured + the idle-step instance), `split-submission-in-flight-cap-overlap`
(the 10-CB-cap draft-before-readback adopt-or-drop contract),
`gdn-transition-replay-log-commit-many` (the deferred replay FUSED into the next scan's step
loop — one launch + one state round-trip saved),
`defer-promoted-write-to-first-consumer-dispatch` (the resolve-or-build synthesis — exactly one
materialization path ever used), `in-place-write-through-input-emits-fence-consumer` (the MLX
graph-dependency fence spelling).

## Dedup record (the drops)

- `make_uniform(params->scale) * make_uniform(1.44269504089f)` — DUPLICATE of
  `derived-scalar-host-computed-in-uniform` (gpu_04).
- The work-interval + in-flight-cap deltas — RIDERS on B181 rules (predicted by the watch row,
  confirmed by the read).
- `never-written-constant-operand-cache-per-shape` (the cachedNarrowZeros buffer) — C-grade
  candidate, dropped (low priority; revisit if a batch touches the narrow-producer family).
- `consume-block-views-skip-concat` — folded into the fused-transform rider (the
  blocks-in-place instance carries it).
- DFlash2 speculative-KV-cache rows (installSpeculative/commitSpeculativeContext) — kin to
  `staged-append-rollback-lifo-retry-blocks` (gpu_08); noted, not a new decision.

## Gates (all green this session)

- batch182 ×2 (rules_present incl. the 6 rider assertions + pseudocode_parses) — 2/2
- lib default 2340/0; kernel_heal slice 2716/0; rust_perf annotation pins 106→109 ×3 sites
- clippy -D clean ×3 postures (default / kernel_heal / all-features; 2 doc_markdown
  mechanicals healed in-passing)
- retrieval 21/21 floors HELD (277 s)
- score-bench NOT owed (zero clippy-corpus delta, P30); freeze regen NOT owed (kernel corpus)

## Lane intel (not acted on — recorded for the owning lanes)

- The verify-path wins (rules 1/2/4/5) are M3-lane league-adjacent: their metal attention
  shapes map to ourbonsai decode/prefill windows; the Issue-846/971 watch lanes may cite this
  pin at owner discretion (the B178 T10-lane-intel OFFER convention — not made).
- `whole-round-in-situ-adoption-trial` is directly consumable by OUR bench harnesses: any
  kernel-variant A/B that picks on synthetic bursts risks the same mispick the source measured.
- The idle-residency rule (6) is the formal statement of this workspace's own bench_preflight
  idle-gate discipline — an external confirmation of the box-state law.

## Provenance

- Clone: `.raw/mlxfast-bonsai2-delta` @ `831fae7` (full history), REMOVED at close.
- Quotes: re-verified byte-exact against the clone BEFORE authoring (the Batch 31 discipline;
  ~20 quote sites across 9 rules + 6 riders).
- Watch lineage: Research 212 (the 09-26 distill) → Addendum 3 (the 09-27 armed rider) → this
  batch (the 09-28 slot; the slot adjudication picked mlxfast over the staged ggml-cpu tiled
  mul_mat lead `d834d44e6` and the pre-graded P40/P41 fixture pack — both stand for the next
  slots; bonsai/ternary-first priority).
