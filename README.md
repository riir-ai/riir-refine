# riir-refine

> **Boundary contract:** [BOUNDARY.md](BOUNDARY.md) — what lives here, what may depend on it, known drift.

> **TL;DR — release:** `scripts/release.sh v0.5.1` — one command, one
> Touch ID fingerprint. It refuses unless the tree is clean, HEAD carries
> exactly that tag, `Cargo.toml` matches, and `release/notes-<tag>.md`
> carries a `corpus:` line; then it builds + packages + scans
> (`manual_release.sh --no-publish`), smokes the built binary's
> `--version`, and publishes via the sealed `gh-release-pat` token
> (`kat-secret exec`) — the token is never on argv, in a file, or in a
> log. Tap/scoop bumps stay manual (hashes in
> `/tmp/manual_release_dist/SHA256SUMS`).

**The code fixer that learns** — a multi-domain modelless framework behind the
`HealDomain` trait that distills code rules into a ruliology-style search
space, drafts fixes via a modelless ternary SIMD matvec, and validates with a
`syn`-based syntax pruner. The shipped domain set (Plan 001 + Plans 018/030/019/085):

| Domain | Feature | Status | Corpus |
|---|---|---|---|
| **clippy_lints** (default) | `clippy_lints` | ✅ Production | 120 corpus lints (live count manifest-derived — `.benchmarks/pins_manifest.json` via `scripts/pins_manifest.py`; Batch 230 carried 108, P51a +4 = 112, P118 +3 = 115 — plan 353; Batch 348 +2 = 120 (`approx_constant` FIXABLE + `useless_vec` VERIFY — the P111 + P112 corpus gaps, plan 359: the bit-identity gate makes every approx rewrite value-preserving, the std-consts-sourced table, the type-blind `&vec![..]` VERIFY tier); Batch 347 +1 = 118 (`let_unit_value` FIXABLE — the P120 rulewright-miss trio, plan 358: the provably-unit shapes only — unit std macro allowlist / unit-tail blocks / if-no-else / loops / unique same-scope file-local unit fns; `let _ = <unit-chain>` method-chain shapes stay the documented divergence — clippy's diagnostics for the lint are help-only, probed 1.99 sugg-null, so the corpus proposer is the only fix path); prior: plan 354 +2 = 117 (the doc-section pair — missing_errors_doc + missing_panics_doc, the Bench-141 dual-evidence heads; VERIFY-tier: the append generates content and real-clippy proves section presence, the probe-pinned `^#+\s+(errors|panics)` predicate refuses compliant docs, concrete-E + recognized-literal panic sources only); the Batch-191-era "100" row sat stale until the pins-manifest gate caught it, plan 329 P1 [`std::mem::replace(dst, Default::default())` → `std::mem::take(dst)` — STD DEFINITION IDENTITY, `take` IS `replace(dest, T::default())`; the two qualified spellings only, the `mem::` form behind the legacy_numeric_constants whole-file shadow-guard; bare `replace(..)` refuses — the SAFER-than-clippy import-ambiguity class; style/default-on — Plan 188, the coverage-report pack XVIII; two gate catches en route: the lint_level driver-match refusal of my perf-group read + the freeze e2e's latent-E0308 in the corpus atom, both fixed at source] — the coverage-report re-arm; Batch 161 2026-09-18: +unit_arg FIXABLE [the syntactic-unit-typeness hoist — a block with no tail expr is `()` by the language definition; hoist + pass `()` is value/order-identical by construction, clippy's own suggestion shape; complexity/default-on — Plan 152, the fifth all-FIXABLE pack] +string_lit_as_bytes FIXABLE [`"abc".as_bytes()` → `b"abc"` — coercion identity; NURSERY group (the plan152 default-groups arm correcting the stale "default-level perf" comment); the `\u{` escape refused — clippy's own suggestion there is invalid Rust] — the frontier-lead pack VII, Plan 152; Batch 160 2026-09-18: +from_str_radix_10 FIXABLE [the construction-identity parse rewrite — std's int `FromStr` IS radix-10 `from_str_radix`, so `u32::from_str_radix(s,10)` → `s.parse::<u32>()` is resolution-identical, NO shadow guard — Plan 151, the fourth all-FIXABLE pack] +assign_op_pattern FIXABLE [the exactly-once-binding derivation compound-assign rewrite — params/annotated-lets/init-transfer/suffixed-literal legs; the swapped-operand arm probe-measured commutative-only (`y = 1 + y` fires, `y = 1 - y` silent)] — the frontier-lead pack VI, Plan 151; Batch 159 2026-09-18: +single_char_add_str FIXABLE [the `mut_string_param_heads_over` mutable-String derivation twin — `String`/`&mut String` params, VALUE-based single-char arg + `escape_default` emit] +manual_clamp FIXABLE [the lattice-absorption clamp rewrite, ints+char only — FLOATS refuse on the NaN max/min-vs-clamp divergence, inverted bounds refuse on clamp's release-mode assert — Plan 150, the third all-FIXABLE pack]; Batch 158: +to_string_in_format_args FIXABLE [perf-group, default-on — pinned by the plan149 default-groups probe; the signature-derivation Display-blanket scalar receiver + the bare-{} placeholder guard, composing with uninlined_format_args by design] +manual_range_contains FIXABLE [the contains rewrite closed by the same scalar derivation + literal bounds — Plan 149]; Batch 157 2026-09-18: +redundant_closure_for_method_calls FIXABLE [the signature-proven `v.iter().map(|s| s.len())` → `v.iter().map(String::len)` — the element type derived from the fn signature + a construction-identity whitelist, String/Vec/HashMap/HashSet × len/is_empty/capacity] +iter_skip_next FIXABLE [the two-arm rule: proven slice receivers heal `.get(n)`, the oracle-clean inverse one step better than clippy's own `.nth(n)`; every other receiver → `.nth(n)`, identical for all iterators] — the frontier-lead pack III, Plan 148; Batch 156: +io_other_error FIXABLE [the `Error::other` splice, inference-identical, legacy_numeric_constants shadow-guard pattern] +wrong_self_convention VERIFY [the `into_*(&self)`→`self` receiver flip, no-vis inherent-impl v1 scope] — the frontier-lead pack II, Plan 147; Batch 155: +let_and_return/+redundant_slicing/+map_flatten FIXABLE +ptr_arg VERIFY — the frontier-lead pack, Plan 146; Issue 120 2026-09-16: +doc_markdown FIXABLE — the pedantic doc-lint family's every-landing manual backtick pass, a faithful port of clippy's own word predicates over syn `#[doc]` attrs + the P21 oracle lane consuming clippy's spans verbatim; P14 2026-09-14: +unnecessary_map_or FIXABLE — the born-lint closure, `map_or(false,f)` → `is_some_and`/`is_none_or`/`is_ok_and` on receivers PROVEN by the shared `receiver_kind` substrate + the safe-let extension; Batch 91, the P9 clippy-1.98 batch: +byte_char_slices FIXABLE, +manual_slice_fill FIXABLE, +manual_checked_ops VERIFY, +chunks_exact_to_as_chunks VERIFY [MSRV floor: std 1.88]; Batch 90: +items_after_statements FIXABLE, +manual_let_else FIXABLE; option_if_let_else REFUTED per `.plans/051` — fixpoint ping-pong with single_match_else; Batch 89: +use_self FIXABLE, +suboptimal_flops VERIFY; missing_const_for_fn KILLED per `.plans/050`); 97 --fix eligible + 20 --verify + 3 self_evolve-only (the split the AGENTS.md domain row publishes — plan 329 P1 repaired this row's stale Batch-191 figures; Batch 347 +1 fix) |
| **rust_perf** (default since 2026-08-13) | `rust_perf` | ✅ Phase 2 (GOAT G1+G5 PASS, Bench 013 promotion + Bench 015 finalization + Bench 016 Issue 012 + Bench 017 Issue 014 + Bench 020 Issue 014) | 334 rules (Batch 360, 2026-11-13, Plan 371 / Research 405 — the polars revisit #2 delta, entries_76 @ pola-rs/polars 437948302 MIT: first-hit-flag-guards-aux-structure [caching, BENCHABLE, the batch's GOAT lead — MEASURED ParetoDominant 3/0/0 two-run stable 12.77–63.83x at [1K,16K,256K], allocs 0→0 — a one-bit flag stolen from the row-index word's top bit gates the side-structure write on the first-hit transition; polars matched_rows.rs MARKED + mark_rows] + u128-funnel-shift-combine [arithmetic, BENCHABLE measured Mixed two-run, landed CORRECTNESS-FIRST — the two-shift combine is UB at s=0; the u128 funnel's %64 makes s=0 exact AND is periodic, one wide shift = SHRD] + 1 count-neutral FACE on B244 iter-count-bulk-specialization [the char-count identity: len − continuation-popcount = char count, class-mask + shifted-self u128 inline mask as face text, entries_30]; fixtures b360 6/6 executed pre-entry; prior: Batch 359, 2026-11-13, Plan 370 / Research 403 — the ajika persistent-incrementality family, entries_75: named-outputs-lazy-handles-partial-decode [BENCHABLE, the GOAT lead — named outputs + cheap address handles, a one-field read never deserializes the unread fields, MEASURED ParetoDominant 3/0/0, read-all anti-regime Tie] + pack-store-gc-decides-by-recovered-index + derived-cache-throwaway-version-reset-recovery + persisted-interner-assignments-checkpoint-with-refering-values + scope-scoped-memo-release-idle-holds-no-decoded-state; rust-glancer/ajika @ 70c1e26d MIT OR Apache-2.0 + the handwritten persistent-incrementality blog, tip unmoved at T0; prior Batch 358, 2026-11-13, Plan 369 / Research 345 lane B — the splash lane-B ANE/host residue, entries_74: one-program-many-functions-accelerator-service-memory + ane-split-calibration-fit-bandwidth-interference-term + accelerator-lost-breaker-forgets-calibration + writeclass-share-partition-inflight-writes + spin-keep-awake-checked-capability-degrade-loud; prior Batch 355, 2026-11-13, Plan 365 / Research 399 — the sglang unified-memory distill, entries_73: co-lifecycle-rows-pack-one-entry-strided-views [BENCHABLE] + logical-free-immediate-physical-reuse-waits-settle + joint-demand-admission-single-pool-free-bytes-logs-only + fixed-ratio-pool-partition-strands-demand; prior Batch 356, 2026-11-12, Plan 366 / Research 400 -- the r/rust AI borrow-checker-appeasement case study (entries_72, POST-ONLY source, decision-content-only transcription): allocation +2 (mem-take-field-checkout-empty-window -- the take-window hazard, opposite polarity of mem-take, in-rule adjudicated; clone-before-mutating-loop-borrow-appeasement -- the POSITION is the signal, nightly redundant_clone passes it) + concurrency +1 (thread-spawn-per-call-pool-reuse -- the BENCHABLE lead, MEASURED ParetoDominant 3/0/0: 4.51x/1.96x/10.81x, allocs 14->2 at every size) + layout +1 (dense-id-domain-hashmap-to-slot-array -- BENCHABLE, MEASURED ParetoDominant 3/0/0: 70.21x/62.23x/75.57x, allocs 0->0); R4 cross-thread-buffer-return-ring DROPPED at authoring (bounded-batch-buffer-pool owns the recycle law; landed as a count-neutral face on it); fixtures b356 16/16 executed pre-entry (four-arm per law); retrieval floors HELD 29/0 no re-pin; prior: Batch 349, 2026-11-10, Plan 360 / Research 393 — the encoders distill: deferred-integer-lane-pooling-per-row-exponent + incremental-prefix-hash-wordpiece-scan + 1 rider face; prior: Batch 345, 2026-11-09, Plan 357 / Research 389 -- +5 the LocalSR distill (entries_70, `HerRei/local-upscale` @ cf6f8b55 MIT -- measurement-honesty +2, concurrency +1 [the tokio-family coalescing axis], io +2; all NotBenchable decision-content, fixtures executed pre-entry); prior: 307 at Batch 343 -- +2 the sccache cache-layer RE-distill (entries_69, mozilla/sccache @ c1d7204a Apache-2.0 PIN-CORRECTED -- Research 387's c1d7204f90a9ed1d does not exist server-side, the real main-tip sha is c1d7204a0f14909c83a9cd983646d5c09149cddd -- the files B324 never mined, zero file overlap with entries_58, caching +2 both NotBenchable decision-content ALL-ANNOTATED: `raw-bytes-with-hit-backfill-once` the tiered-hit backfill law -- a slower-tier hit reads ONCE in a form both consumers share: the parse views the bytes while the same Bytes handle rides to the DETACHED backfill write, the fastest-tier hit skips the fill, only tiers with a faster neighbor request raw (idx > 0 gets raw, idx == 0 plain get), a level without raw capability degrades to a logged skip never a second read, multilevel.rs 491-526/571-601 + disk.rs 126-136 byte-verified + `vanished-entry-tolerant-eviction-panic-else` the shared-directory eviction law -- a peer's NotFound TOLERATES with a debug log (the in-tree issue-2092 cite), ANY other error PANICS loud, peer-caused vs self-caused classification instead of one flattened arm, make_space 257-288 byte-verified; the batch's candidate-2 dedup KILL recorded as the gate working -- advertised-size-staged-write-reconciles-at-commit was a DUPLICATE of reserve-then-commit-capacity-accounting (entries_58, the same source's prepare_add/commit law; Research 387's dedup note mis-read it as a scan-time container rule); fixtures b343_fixtures 11/11 standalone-executed pre-entry, retrieval floors HELD both lanes 29/0 + the priced synth floor PASS); prior: Batch 341, 2026-11-08, Plan 350 / Research 383 -- +1 the metal-graph measurement-honesty rule (entries_68, robbiemu/metal-graph @ 0cd855b7 Apache-2.0, the corpus's 15th category measurement-honesty: `amortized-reuse-claim-break-even-artifact` -- an amortized-reuse claim reports break_even = bake/per_launch_savings as a first-class artifact value, savings <= 0 a LOUD named verdict, the fair-baseline pin + the claim boundary in-rule); prior: Batch 340, 2026-11-06, Plan 349 / Research 381 -- +2 the SPILLWAY agent-egress-guard distill (entries_67, saintmachines/spillway @ 0fc9f55f Apache-2.0, complexity +2 both NotBenchable census-shaped: `paired-closer-precollect-gates-lazy-wildcard` the closer-existence gate over a two-anchor lazy-wildcard hand-rolled scan, measured ladder 6.3x/48x/121x/1655x at k=128/512/2048/8192 + `rare-delimiter-anchored-token-scan` the rare-delimiter anchor over a per-start token scan, run-pole 5e4x, dense anti-regime 0.75-0.81x in-rule); prior: Batch 334, 2026-11-05, Plan 343 / Research 372 -- +2 the nnethercote Nov-2026 rustc-perf distill (entries_66, rust-lang/rust PRs #163628+#163857+#163971, caching +1: `single-entry-last-key-lookup-cache-hash-avoidance` BENCHABLE ParetoDominant 3/0/0 6.79x/6.24x/5.49x at 1K/16K/262K allocs 0/0 -- a one-slot last-key memo in front of a hot hash table, the read-stable-window constraint in-rule; layout +1: `skewed-payload-inline-encode-intern-overflow-tail` suggestion-only NotBenchable size-axis -- skewed payload packs inline behind a high-bit prefix, overflow tail interned cold-field-deduped, hot field inline everywhere); prior: Batch 332, 2026-11-05, Plan 340 / Research 370 -- +2 the blazing-smtlib distill (entries_65, onderjan/blazing-smtlib @ 42b45c20 MIT OR Apache-2.0, dispatch +2: `byte-flag-table-masked-run-scan` BENCHABLE -- the batch's GOAT-eligible lead, a const-eval 256-entry u8 class-flag table + a masked-prefix run scan replaces the per-byte predicate chain, measured ParetoDominant 3/0/0 -- 2.62x/2.94x/2.10x at 1K/16K/262K, allocs 0/0, regime-complementary with B243's block-skip sibling, lane-intel mapped to riir-infer's tokenizer pre-tokenizer; `token-batch-cursor-handover` suggestion-only NotBenchable -- a fixed-capacity staging array of zero-copy views + a cursor bulk handover with boundary-applied abort actions; prior: 298 at Batch 331, 2026-11-05, Plan 339 / Research 368 + addendum -- +1 the hindsight round-2 pool (entries_64, vectorize-io/hindsight @ 841979df MIT, concurrency +1: `batch-retry-ladder-classifies-failure-before-permanent-state` NotBenchable-by-domain -- a batch retry loop that catches EVERY failure into one posture (identical re-sends + durable failure flags on exhaustion) turns a provider control signal (quota reset carrying retry_at) into identical re-sends plus a PERMANENT exclusion that orphans the pending items and burns the ladder on a deterministic failure whose byte-identical replay earns the byte-identical refusal; classify FIRST: PROPAGATE the control signal at first sight (surface retry_at, nothing flags), FAIL_FAST the deterministic failure with the permanent type (zero re-sends, caller bisects), the bounded ladder for transport-shaped only; +1 count-neutral face: the fact_extraction.py promote-permanent-type law, same fixture; pool candidates A/B recorded as cited drops -- decision content without a fix-space, Research 368's own Drops rows; prior: 297 at Batch 330, 2026-11-04, Plan 338 / Research 368 -- +1 the vectorize-io/hindsight distill (entries_63, vectorize-io/hindsight @ 841979df MIT, concurrency +1: `fanout-first-error-leaves-siblings-running` NotBenchable-by-domain -- cancel-then-await on the fan-out error path, the two-sided shared-state discriminator, the error-envelope caveat; std-thread fixture, the law names the async JoinSet spelling; prior: 296 at Batch 329, 2026-11-04, Plan 337 / Research 367 -- +2 the tembo/voxl distill (entries_61 + entries_62, tembo/voxl @ eba3b7ce MIT, dispatch +1 + concurrency +1: `event-damage-map-selective-partition-skip-with-consumer-hints` BENCHABLE -- the batch's GOAT-eligible lead, one event-sourced change map drives EVERY pipeline stage: whole-partition selection + work budgets from map density + skip-hint forwarding into the heavy consumer's own analyzer, measured ParetoDominant 3/0/0 -- 8.18x/6.96x/6.99x at 256/4096/65536 cells at the landing (reproduced 8.66x/6.93x/7.06x on a re-run at load ~21, verdict class stable; the ratio tracks the ~1/dirty-fraction ceiling at the 12% band), allocs 0/0, dirty band 12% posture; `online-cost-fit-thread-ladder-reopen-hysteresis` NotBenchable-by-regime -- EWLS fit of ms = a + b*changed-work (decay 0.995, w<30 gate), asymmetric two-bar ladder (UP 20-consecutive 65%-budget EWMA breaches / DOWN predicted-half-budget sustained 2 s) because a mode switch reopens the encoder and costs a keyframe; +1 count-neutral rider face on `reserve-then-commit-capacity-accounting` (entries_58): the wire-header reserve-in-place law -- out.resize(HDR,0) before the encode, in-place writes after, Bytes::from zero-copy handoff; prior: 294 at Batch 326, 2026-11-04, Plan 330 / Research 363 -- +1 the rust-lang/rust #164484 distill (entries_60, compiler-internal rustc @ merge 026faeb1, layout +1: `adaptive-sparse-blocked-table-encoding` NotBenchable-by-domain -- flush-time census + smaller-of dense/sparse gate over an index-keyed default-heavy table: per-64-block presence bitmap + cumulative u32 base + packed non-defaults, mode flag in the header word's low bit; upstream -20.15% metadata / -3.8% max-RSS for an accepted +0.3-0.4% instruction cost; prior: 293 at Batch 327, 2026-11-03, Plan 332 / Research 364 -- +1 the firezone/sanstun distill (entries_59, sans-IO WireGuard @ 35cb1932 Apache-2.0, concurrency +1: `batch-reserve-monotonic-counters-offload-bulk` NotBenchable-by-domain -- one bound-checked range reservation fronts per-item bulk work on any thread in any order; prior: 292 at Batch 324, 2026-11-02, Plan 328 / Research 362 -- +3 the sccache io/cache/process distill (entries_58, mozilla/sccache @ 71d107e Apache-2.0, io +3: `pre-exec-disables-posix-spawn` BENCHABLE with the measured 2.62x spawn-path spec (parent-RSS-pinned fixture) + `reserve-then-commit-capacity-accounting` + `mtime-encoded-lru-recency-persistence` both NotBenchable decision-content; the io-copy-write-adapter candidate DROPPED POSITIVE-INSTANCE at the probe); prior: 289 at Batch 300, 2026-10-31, Plan 305 / Research 341 -- +2 the ConstexprCore/perfect_hash micro-layer (entries_57, ConstexprCore/perfect_hash @ 4fdf1b7f MIT, both BENCHABLE: the fold-key-len one-word verify compare + the fused-value spare-lane packing; prior: 285->287 Batch 318, 2026-10-30, Plan 318 / Research 356 -- +2 the impeccable io stretch-tier remainder (entries_56, pbakaus/impeccable @ 02523aec Apache-2.0, both NotBenchable: the generation-counter stale-timer invalidation law + the exclusive-mkdir temp-resource reservation law, completing plan 316's honest partial); prior: 280->285 Batch 316 (2026-10-30, Plan 316 / Research 356 -- +5 the impeccable io/process robustness pool (entries_55, pbakaus/impeccable @ 02523aec Apache-2.0, ALL NotBenchable: the per-read-vs-request-deadline lead + the piped-stderr drain + kill-without-wait + the connect walk + the arrival-order turnstile, +1 count-neutral rider on prefix-sum-2d); prior: 278->280 Batch 308 (2026-10-30, Plan 313 / Research 352 -- +2 the 10-29b staged serving pool + rust_perf row (entries_54: the cross-tier LRU-recency law, vllm #62444 @ c1b7dffa; the refusal-quarantine verdict law, TensorFold #1539 @ d1fcb9d; +2 count-neutral rider faces: the REMOTE-writer ack arm on gpu_111 + the CONSUMING-SIZE face on entries_45); prior: 266->278 across B274/282/283/284/288/298/305 -- the row was STALE at the pre-B274 266, repaired en-route here, the missed-pin class) | Prior: Batch 245, 2026-10-16, Plan 248 / Research 277 — +6 the Batch-244 DEFER LIST conversion (the eight staged leads convert 6 + rider + defer, ZERO dropped: self-loop-state-memchr-skip GOAT 5.5-6.4x + pair-probe-confirm-fused GOAT 8.1-9.9x — fixture RESTRUCTURED once + the PARITY ORACLE caught the bit-8j+7 SWAR extraction bug, the pre-fix 8.9x was an artifact of skipped verifies — + finder-hoist-reuse GOAT 3.0-6.4x + canonical-set-guarded-rebuild + nonmax-sentinel-option-niche + match-branch-cold-confirm, all NotBenchable annotated + the count-neutral CHAR rider on byte-find-position-to-memchr; dispatch 13→15, simd 10→11, caching 15→16, complexity 33→34, layout 15→16; retrieval floors HELD 21/0/1i; gen2+gen3 freeze e2e ok; score-bench run #185; the stale-pin repair: with_spec/without_spec re-pinned to measured truth 114/109→ the B239/B207 pin-hiding class).
| **kernel_opt** (default-on via `kernel_pregate` since 2026-11-06) | `kernel_opt` | ✅ Phase 3 + Batch 1-345 corpus + PROMOTED 2026-11-06 (owner call "any gain no perf hurt must enabled by default, dont make good goat sad" — the rules are fixture-tested + staged-lead byte-verified, the pregate fence is measured; the real-corpus GOAT validator stays open as hygiene) | 1081 rules (Batch 354, 2026-11-13, Plan 364 — the sgl-muse-mega code-cache + cross-step-routing distill, entries_gpu_156: launch-time-code-as-data-l2-warm + dry-pass-icache-warm-before-real + speculative-expert-prefetch-prev-step-routing, +1 count-neutral face, the C+ self-stamp entry dropped at authoring; prior Batch 357, 2026-11-12, Plan 367 — +2 the moeshift distill, entries_gpu_155, the clobbered row REPAIRED here; prior Batch 345, 2026-11-09, Plan 357 / Research 389 — +5 the jitengine MLX/Metal distill, entries_gpu_152, `skydiscover-ai/jitengine` @ 2c34508d Apache-2.0 (the oldest staged slot landed out of numeric order): E `verify-parity-position-pure-grid` (the grid = f(position) law + the PASS_M1 verify twin; 4-way genus adjudication) + N `compiled-vs-eager-per-op-exactness-admission` (per-op-class admission with the per-pair check; the flint-m2 DETAILS:102 file attribution corrected) + G `gdn-state-snapshot-per-position-verify-early-exit` (the snapshots ARE the early-exit substrate; adversarial vs the journaling pole) + D `command-buffer-declared-input-pacing` (the declared-input lever vs the cap lever, compose) + H `compute-experts-before-router-lookup-fixup` (the device-side slot snapshot + miss-only rebuild); +5 count-neutral faces on their OWNERS; C9 dropped (niche); fixtures 6/6 standalone-compiled AND executed BEFORE entry; T0 5/10 quotes exact as recorded (5 corrections carried); prior: Batch 339, 2026-11-06, Plan 348 / Research 380 — +4 the TriAttention mixed-precision-attention distill, entries_gpu_151, `HanzhiZhang-Ulrica/TriAttention` @ 805fae2c NO LICENSE (decision-content-only, original fixtures; SAME AUTHOR as TileQuant — the family's outbound-PR posture stays FAIL-CLOSED; arXiv 2611.07256 code release): N `tied-rotation-bilinear-invariance-quant-isotropy` the batch's GOAT-eligible lead (the SAME pinned sign-Hadamard rotation on BOTH operands before per-row absmax — the dot INVARIANT, each side's error ISOTROPIZED, the outlier stops governing the scale; the decode re-rotate keeps the tie across quantize AND decode) + F `diagonal-boundary-pair-o1-policy-storage` (a per-head attention policy monotone in diagonal tile distance collapses to TWO integer boundaries — O(1) storage independent of context length, closed-form col_at expansion; the NEAR_V8 recency arm rides in-rule: latest 16 decode rows PLUS row 0 stay 8-bit) + B `deferred-correction-rides-rescale-periodic-flush` (the exact anchor correction accumulates in a side register, rides the rescale the softmax already pays (fma(out, α, -corr·α)), flushes every 16 int8-PV tiles when dormant, out reads (out − corr) once; bit-identical inside the anchor domain) + K `lagrangian-shared-budget-policy-allocator` (suggestion-only NotBenchable-by-domain: per-head argmin(err + λ·cost) over a logspace(-9,1,400) sweep, the frontier cut by a MEAN-cost fraction, loud failure; the Gram-weighted consumer-sensitivity error; the config grid counts 86, T0-corrected from the verdict's 66); +5 count-neutral faces on their OWNERS (conditional-pv-accumulator-rescale += the warp-ballot elision __all_sync face; magic-anchor += the PMagicHalf float→int rounding-at-quantize face; attention-row-mask-bitmap += the kinfo bit-packed descriptor + ragged cumsum face; occupancy-keyed-value-split += the occupancy-wave decode partition face; setmaxnreg-fit-gate += the __maxnreg__(128) declared per-kernel cap face); fixtures 8/8 standalone-compiled AND executed BEFORE entry; T0 26/30 quotes exact at the pin (4 record corrections carried: the twin at 308-313, the selection block at 71-74, the grid 86-not-66, distance code-implicit); prior: Batch 338, 2026-11-06, Plan 347 / Research 379 — +3 the TileQuant distill, entries_gpu_150, the corpus's FIRST PURE-Triton source (D `ascending-tile-arena-address-is-loop-index` + N `batch-wide-column-tile-actmax-scale-pass` + D `compiled-handle-direct-launch-key-completeness`, +3 faces on their owners); prior: Batch 336, 2026-11-05, Plan 345 / Research 374 — +5 the walk-23 B+ PAIR distill, entries_gpu_149, two sources (mscheong01/loopvq @ 38d5eff3 Apache-2.0 + InferMatrixLab/vllm-rlt @ 444638be Apache-2.0): B `chunked-argmax-strict-gt-tie-identical` the batch's GOAT lead BENCHABLE ParetoDominant 3/0/0 + A `repeat-axis-aliased-index-arena` + N `load-time-decode-blind-transform-absorption` + D `graph-fallback-coverage-ladder` + O `i64-index-promote-before-stride-multiply`, +1 count-neutral face; prior: Batch 335, 2026-11-05, Plan 344 — +3 the mxfp4-int8-m5 W4A8 distill (entries_gpu_148 append, shinyoungkang1/mxfp4-int8-m5 @ 3d060c6, the arXiv 2611.00486 code release): A `whole-matrix-unpack-pass-rows-per-tile-gate` (the cross-grid unpack-amortization gate, the auto line two-pass once R // E >= 256, both arms in-source) + L `spread-constrained-scale-fold-int8-admission` (q = mag << shift under the exponent-window admission, '12 * 2**3 = 96 <= 127', raise-to-hi-3 keeps a valid MXFP4 file, heavy-tailed rows refuse) + H `counting-sort-route-permute-one-kernel` (one kernel emits (pos, tok, esorted, off, tstart); order-free by rows independence); +2 rider faces fuse-gate-up-gemv-activation x2 (the GEMV-prologue act-quant ANTI-face 0.61-0.74x + the in-register epilogue face 1.04-1.12x); the G1 harness coverage repair en-route (entries_gpu_148 was missing from both censuses since B333 — population 248/72 -> 252/73); prior Batch 333, 2026-11-05, Plan 341 — +2 the sgl-nvidia-mega distill, entries_gpu_148, `fail-stop-deadline-trap-bounded-cross-rank-wait` (the co-resident-launch fail-stop enforcement law: poll-subset %globaltimer deadline + out-of-line store-to-faulting-address-then-trap + watchdog-derived deadline compiled into every reachable wait path — the gpu_118 policy/ladder's enforcement layer, unowned) + `peer-mailbox-rotation-depth-reader-residency-plus-one` (cross-launch mailbox rotation: depth = proven reader residency + 1, never-zero per-writer epoch from launch counts, one atomic-width {tag, payload} word — composes gpu_61/gpu_138; kills: key packing = gpu_20's instance, flag formats = gpu_30, two-shot reduce = gpu_142); sgl-project/sgl-nvidia-mega @ c396f2ab, seeds the SGLang family; prior Batch 323, 2026-10-31, Plan 323 — +7 the 10-30i + 10-30j staged leads conversion, entries_gpu_147, kernel_opt-only: N `nan-preserving-select-over-fmaxf-in-scale-reductions` (tensorfold cef366c9+2b587959 — the comparison-select keeps NaN, fmaxf launders; the amax propagates a NaN scale) + D `banded-launch-order-shares-weight-tiles` (tensorfold 19c491ce — a pure launch-order permutation, kGemmBand=4 row-blocks share one column tile's weights in L2, byte-identical) + L `quant-block-keyed-index-space` (vllm 6c19299fc #64509 — the lattice mirrors the [G, 8, 2] (block, pair, lane) nesting, no reshapes) + A `simdgroup-fragment-whole-store-generation-keyed` (tensorfold 0afc366c+bfb0f5a2 #1590 — whole vec-reinterpret fragment stores bit-identical, admission per GPU generation 13/14) + A `stream-through-load-no-l1-allocate-once-read` (tensorfold acb21860+839778d4+5e151065 #1534 — L1 allocation keyed on REUSE, the once-read stream takes ld.global.nc.L1::no_allocate) + B `inner-loop-select-division-shift-mask-vgpr-pressure` (tensorfold 33b443da #1663 — shift/mask + uniform branch when per ∈ {1,2,4}, the K-loop no longer spills) + L `activation-layout-mirrors-weight-bit-serialism` (PrismML-Eng/llama.cpp 01ea7fcf #420 — the activation layout permuted to the weight's bit decomposition, eight aligned dp4a, exact int32); +6 count-neutral rider faces gpu_99 (vllm dtype-boundary rounding-points + prism pinned-reference/window-downgrade) + gpu_140 B309 (all-slices-issued-before-sums) + gpu_111 (dependents-before-loads early trigger + PDL-on-5120) + gpu_146 (pinned-mapped staging) + gpu_51 (persistent-topk score-read) + gpu_141 B310 (arch-conditional rows + fusion-context geometry); T0 14/14 byte-verified; prior Batch 322, kernel_opt-only: A `checkpoint-boot-staged-upload-bounded-bytes` (tensorfold 7fce15ce #1643 — the REUSABLE 64 MiB host-stage upload law) + B `boot-identity-hash-read-once` (tensorfold 618625fd — the digest-threading law, one hash per byte per boot) + D `vendor-intrinsic-staged-operands-by-value-scheduling-attribute` (llama.cpp 792fcf4e #31341 — the Adreno X2 vendor-compiler workaround: staged operands BY VALUE + `qcom_wave_pair_mode(1)`); +2 count-neutral rider faces gpu_02 (vllm 2f6f7861b #63761) + gpu_17 (tensorfold 297a182d); the DSpark lead graded host-shaped → rust_perf behind plan-310; prior Batch 321 — +4 the 10-30f staged leads conversion, entries_gpu_145, kernel_opt-only: B `minifloat-cast-routing-native-vs-software-by-target-direction` (cubecl 6bff8ad4 #2008 — native iff decode direction on a native-decoding target; per-(arch, format, direction); the no-fp6 removal half) + B `block-scaled-mma-scale-register-owns-its-rows` (cubecl a4616247 #1995 — one scale register per unit, selectors zero as the C++ backend's `{0, 0}`, the ScaledMmaUnsupported refusal) + B `exl3-span-view-plans-at-planning-granularity` (tensorfold cef784e2+6a8dfec0 #1507 — the view snaps to the planning boundary so plan(view) == plan(whole)) + C `comms-boundary-consumer-chain-fuses-into-gather` (vllm becbaea9f #64638 — the collective's elementwise chain fuses INTO the gather, fused-first + declared preconditions + fallback parity); +2 count-neutral rider faces gpu_141 (cubecl f48e9f53 #2006 A-operand side-invariance, the family's third instance) + gpu_144 (a4616247 ScaledMmaUnsupported supported-form admission); the S-f9 dispatch faces DROPPED — owned by gpu_94 at the same source PR (llama.cpp #29353); prior Batch 320, 2026-10-30, Plan 320 — +4 the post-B319 staged remainder, entries_gpu_144: A `immutable-operand-device-resident-per-buffer-type` (llama.cpp fb59972 #31073) + N `word-native-byte-pair-minifloat-conversion` + B `block-scaled-operand-declared-form-refusal-ladder` + D `plane-storage-barrier-scope-covers-global-exchange` (all cubecl 07cd07a) + the gpu_111/gpu_133 rider faces; prior Batch 319, 2026-10-30, Plan 319 — +4 the 10-30 staged kernel_opt pool, entries_gpu_143; prior Batch 317, 2026-10-30, Plan 317 — +3 the 10-30d staged leads, entries_gpu_142); — the Plan-001 "~300-500 rules in ~15-25 batches" projection long since exceeded |

| **kernel_pregate** (DEFAULT-ON + release set since 2026-11-06, implies `kernel_opt`) | `kernel_pregate` | ✅ PROMOTED 2026-11-06 (Issue 208; Benches 129–137) | The file-level kernel-signal gate + ±24-line span scope + the line-cap pair + the query-text cap — the composed Bench-137 posture ships as the in-code defaults (SCOPE armed, LINE_CAP=2, QUERY_TEXT_CAP=2048): added p50 GREEN 0.191/0.036 ms (flat 33.1 — a 173×/920× median cut), the p95 fence CLEARED by the composed posture (4.685 ms at TC2048, 1.07× headroom, confirmed in an alone re-run; 1024 → 3.366 ms, 1.49×), utility floor 15/15 held in every arm; the cap stays deterministic-lossy by paired rank-1 (109/664 flips at TC2048) — disclosed, accepted by the owner call; the three envs are DEMOTE-ONLY (the exact literal `0` restores each axis's pre-promotion behavior byte-identically) |
| **sec** (opt-in) | `sec` | ✅ Phase 0+1+2 + Batch 131 + Batch 140 + Batch 165 + Batch 258 + Batch 291 + Batch 292 + Batch 294 + Issue 185 + Batch 325 + Batch 327 + Batch 328 + Batch 329 + Batch 340 + Batch 342 (Bench 046; Phase 2 Batch 88; P8 intake Plan 057; InputValidation pair Plan 112; advisory-db distill Plan 120; wire-arithmetic overflow sink Plan 156; web-trust seed Plan 262; penthera distill Plan 295; CVEngine distill Plan 296; RustSec transcription Plan 298; softmax-collapse family Issue 185; uniffi-rs FFI-hygiene Plan 327; sanstun distill Plan 332; oss-fuzz distill Plan 336; voxl distill Plan 337; SPILLWAY Plan 349; harness-adapters Plan 352; 0sec distill Plan 356) | 69 rules — Batch 353 +4 (the FORGE Lab std memory-safety post, Plan 363 / Research 397, CC BY 4.0: `unsafe-invariant-panic-exposed` the invariant-across-user-call-unwind lead riding PanicSafety's widened doc (origin #156501) + `user-impl-call-once-keep-result` + `user-impl-derived-property-validate` seeding the NEW TrustedImpl family (the AsRef-twice get example, join CVE-2020-36323 + #155708; the 0x80 Pattern corruption #156491) + `process-global-write-under-shared-lock` seeding the NEW ProcessState family (exec's environ under the read lock #156951); two honest drops: #161018 + the Allocator no-unwind contract; fixtures 17/17 standalone-executed); prior: Batch 352 +4 (the LoNE ANE-numerics transcription, Plan 362 / Research 395: `fp16-subnormal-product-power-of-two-rescale` (weight ×2^k / output ×2^-k below the fp16 flush boundary on wide reductions; regime-conditioned k > 1024) + `wide-reduction-precision-split` (halves summed AT THE END for parity, in-rule adjudicated vs the split-K family) + `fused-activation-approximation-expansion` (expand to the primitive composition off the approximate fused op; in-rule adjudicated vs the GLU-fusion family both directions) + `position-keyed-additive-cache-mask` (absolute-position masks, never buffer index — the rotated newest row) — all suggest-only Numerics-family, fixtures 15/15 executed pre-entry); prior Batch 351 +5 (the 0sec-labs/foxguard distill, Plan 361 / Research 394: `sql-query-value-interpolation-unbound` (the VALUE-position half of the sqlx sink family — mandatory in-rule adjudication vs the B350 identifier rule, both directions) + `quantum-vulnerable-primitive-selection` (the PQ family seed: skip list + boundary law, suggest-only, fires at home on the SSH Ed25519 surface) + `regex-backtracking-engine-untrusted-pattern` (the regex-crate-linear anti-regime IS the decision content; explicit-path precedence: a crate-prefixed `regex::Regex::new` never fires, a bare `Regex::new` in a fancy-importing file fires and review decides) + `server-connection-without-read-timeout` (the slowloris law; per-CONNECTION vs peer-KEYED adjudication vs untrusted-keyed-state-unbounded) + `open-redirect-unvalidated-target` (the route-name safe half + the constant-binding law); the R6 program-identity face on shell-string-command-interpolation + the jwt decode-without-verify honest drop; issue 227 carries the D1–D7 fidelity adjudication (block descent, const suppression, explicit-path precedence, byte-range spans — the verification must-fixes that landed unapplied, now fixed)); prior: Batch 350 +5 (the 0sec-labs/0 distill, Plan 356 / Research 388: `sql-identifier-interpolation-unallowlisted` the corpus's first SQLi rule — the identifier-position two-regime law — + `predictable-rng-for-secret-material` + `ecb-block-mode-selection` + `jwt-alg-family-confusion-allowlist` (the DISABLED-vs-WRONG-allowlist adjudication vs jwt-insecure-validation-flags) + `untrusted-content-interpolated-into-prompt`, the R6+R7 conjunction's lone survivor — NO new SecFamily; the weak-hash co-occurrence rider face; 6 recorded drops incl. R5 SSTI + R7; suggest-only); prior: the Batch-345 execution-artifact-trust-ladder rule (research 389, entries landed same batch) rides the Batch-342 harness-adapters/safeagents agent-egress-guard pair (`credential-material-in-command-argv` SensitiveData — credential material in a spawned command's argv is world-readable via ps//proc, token shapes + header values + secret-named env interpolation, the fix is the TRANSPORT never a cleaner argv, adjacent-DISTINCT from shell-string-command-interpolation — injection vs exposure axes, huggingface/safeagents @ 0de8e22d Apache-2.0 + atif-scan @ 0228cd9d MIT + `egress-control-bypass-flag` InputValidation — a flag that DEFEATS an egress control: wildcard proxy exclusions, proxy vars removed/emptied, TLS-verification opt-outs, the source's own local-only + name-list carve-outs carried in-rule, sink-distinct from unvalidated-egress-url — URL CHOICE vs policy-layer defeat; +2 count-neutral faces on the spillway read-output-trust host — host-sanction-provenance (human-turn-derived sanction, model-echo never self-authorizes) + authored-only scoping (the per-field-class trust inversion); candidate 3 verifier-self-modification honestly DROPPED per the plan's expected-drop allowance; suggest-only); prior: the Batch-340 SPILLWAY agent-egress-guard pair (`removed-span-veto-outscopes-recovery-path` sanitization evidence outscopes the recovery path — a target riding a removed span stays refused no matter what a downstream judge scores; the veto gates the referral, fail-closed + `read-output-trust-inherits-argument-taint` a read's output inherits its ARGUMENTS' trust — typed fields are not provenance; both saintmachines/spillway @ 0fc9f55f Apache-2.0, suggest-only); prior: the Batch-337 jitengine InputValidation rule (`conditional-reuseaddr-after-liveness-probe` — the explicit SO_REUSEADDR call site is the take-over surface: bind WITHOUT reuse first, on EADDRINUSE probe, rebind with reuse ONLY when no live server answers on the port — the wildcard-shadow hijack refuses, the quick restart keeps working; std-safe-by-default anti-regime in-rule; skydiscover-ai/jitengine @ 2c34508d Apache-2.0; suggest-only) + the Batch-331 hindsight round-2 InputValidation rule (`externally-authored-edits-by-id-not-counted-index` — an edit API addressed by an index the AUTHOR COUNTED from a rendered view cannot be range-validated into correctness: an off-by-one is still in range, silently overwrites an unrelated block, is recorded as success, leaves length unchanged so no shrink guard notices; address externally-authored edits by a COPIED id, resolve it, skip-and-report a non-resolving one — vectorize-io/hindsight @ 841979df MIT; suggest-only) + the Batch-330 hindsight `SensitiveData` trio (`ascii-token-unicode-word-boundary-scan-miss` Unicode-boundary scan miss + `luhn-validate-numeric-pii-candidates-before-redact` checksum-before-substitute with the overlap rescan + `redaction-audit-hit-carries-raw-match` fingerprinted audit previews — vectorize-io/hindsight @ 841979df, MIT; all suggest-only) — FFI panic containment, Drop-glue panic safety, ptr-null hygiene, `from_raw_parts` len-zero, Rc backedges + the Phase-2 unsafe-impl/ffi-export/crypto trio + `nan-unsafe-sort-comparator` (PanicSafety) + the Batch-131 `InputValidation` pair (`length-prefix-validate-before-alloc` + `secret-file-create-without-0600`, Girder distill) + the Batch-140 `unbounded-read-into-vec` (rustsec/advisory-db distill, RUSTSEC-2024-0003 / CVE-2019-9514 class; + the Batch-258 decoder see-through rider) + the Batch-165 `unchecked-wire-arithmetic` (the overflow sink on the shared `wire_taint` walker; Kani g2c/g2d property pair) + the Batch-258 web-trust seed (`untrusted-path-join-escape` path traversal + `webhook-signature-over-raw-bytes` signature coverage + `unvalidated-egress-url` SSRF URL choice + the Batch-292 use-time rider — cloudflare/security-audit-skill distill) + the Batch-291 penthera distill (`cors-any-origin-with-credentials` + `jwt-insecure-validation-flags` + `auth-cookie-without-secure-flags` — the CORS/JWT/cookie classes) + the Batch-292 CVEngine distill (`client-ip-from-untrusted-forwarded-header` + `shell-string-command-interpolation` + `ipv4-mapped-range-check-unwrap` + `abuse-strike-counts-cross-site-requests` — the std `IpAddr`/`Command` + identity/strike classes) + the Batch-294 RustSec transcription (`packed-field-silent-overflow-wrong-key` masked-key packing + `capacity-exceeded-silent-fallback-not-refusal` silent-cap degrade — the RUSTSEC-2026-0380/0379 classes) + the Issue-185 softmax-collapse family (`Numerics`, arXiv:2610.31642 — `sigmoid-minus-one` mechanical + `softmax-onehot-collapse` / `neg-ln-softmax-prob` suggest-only) + the Batch-325 uniffi-rs FFI-hygiene quartet (`arc-refcount-across-raw-boundary` raw-Arc custody two-regime + `ffi-callback-invoke-outside-lock` cross-FFI ABBA deadlock + `out-param-manuallydrop-not-maybeuninit` caller-init repr(C) contract + `panic-handler-nested-catch-unwind` handler self-guard — mozilla/uniffi-rs @ ef9625b6, MPL-2.0) + the Batch-327 sanstun trio (`secret-key-without-zeroize` secret lifetime/erase — CryptoHygiene's first lifetime member, dep-gated on a zeroize import + `untrusted-keyed-state-unbounded` the resource-exhaustion family's CARDINALITY face, fail-closed full-table refusal + `gateway-type-forces-security-check` checkpoint bypass via constructor reachability — firezone/sanstun @ 35cb1932, Apache-2.0) + the Batch-328 oss-fuzz archive-extract rule (`untrusted-archive-extract-paths` zip-slip — the crate's own `enclosed_name()` screened accessor is the bounded repair, sink-distinct from path-join-escape — google/oss-fuzz @ a380569f, Apache-2.0) + the Batch-329 voxl pair (`browser-reachable-service-host-origin-rebinding-gate` — the request-side handshake gate against DNS rebinding + cross-origin WS drive-by, WS handshakes not CORS-gated so the server itself must gate + `custom-header-mutation-gate-unanswered-preflight` — the mechanical CSRF form, a custom header the browser cannot attach cross-site plus an unanswered preflight closes the write path; the B258 session-architecture bulk adjudicated in-rule — tembo/voxl @ eba3b7ce, MIT) — ALL InputValidation/crypto-web/FFI-hygiene rules suggest-only; 7 fixable proposers + 31 suggest-only detectors + the real-clippy boundary gate (Plan 018 / Proposal 004) |
| **docker** (default since 2026-08-28) | `docker` | ✅ Phase 0+1+2 (Plan 030) | 31 rules (14 Class-A hadolint overlays + 17 novel cache/PID1/platform/base detections), 14 with bounded fixes; the first non-`syn` domain (stanza scanner) and the first with an external-binary oracle (`docker_verify` → hadolint, plus the Tier-2 `docker_buildcheck` → `docker build --check`). Zero-fixable-finding negative controls on two in-tree production Dockerfiles; G5-real `docker build` green |
| **dist** (default since 2026-09-13) | `dist` | ✅ Phase 1 + B1/B2 (Issue 085; Bench 082 + §6 addendum) | 8 suggest-only rules over name-matched `Cargo.toml` manifests — release-profile size knobs (strip / `[profile.dist]` size profile / panic-abort) for crates that ship a binary + the workspace-member profile trap + the Batch-143 `-Zhint-mostly-unused` large-API-dependency hint + the Batch-146 PGO training-set hint + the Batch-221 `bench-targets-shared-dependency-cluster-hint` (bevy #25321: ≥3 ungated `[[bench]]` targets compile every target's heavy deps for any one; the split = default-on per-crate bench features + optional bench deps + per-bench required-features — the green-zero caveat rides the advice) + the Batch-328 `profiling-profile-debug-assertions` (the oss-fuzz cargo-wrapper law — a profiling-named profile inheriting dev samples a binary whose asserts can abort mid-sample; pin `debug-assertions`/`overflow-checks` false; the scanner's ONE value-shaped exception). Measured provenance: 11,903,168 → 4,546,448 bytes (−61%) on this tool itself. Second no-`syn` scanner (zero new deps); never edits `[profile.release]` (the `refuses()` table pins it). The row previously read 5 — stale through B146's sixth rule, repaired here with the honest label |
| **shader_perf** (default-on since 2026-11-07, Issue 213 / Bench 143) | `shader_perf` | ✅ Phase 0+1 (Plan 134 / Proposal 014) | 8 WGSL render-pipeline rules (`@vertex`/`@fragment`; `@compute` stays `kernel_opt`) — 1 bounded (the `unbounded-render-loop-guard` proposer, Tier-0-invariant-checked) + explicit-LOD two-regime + branch-divergent decision + const-fold + 3 Class B annotations + 1 corpus-only. Third no-`syn` scanner (no regex); `shader_verify` = in-process naga (already in the lock — zero new packages); negative control pins ZERO findings on the 190-file riir-shader vgpu port corpus. Consumers: the upcoming shader-toy editor's submit/save gate + the riir-dapps Proposal 002 consumer gate |

Heavyweight LoRA is the last-resort fallback, not the co-equal path. Plan 005
(2026-08-14) added the **latent suggest layer** — `cargo refine --suggest`
retrieves ranked corpus rules for real code spans (GOAT G1–G4 PASS, Bench 018;
`--fix` CLI wiring is Phase 6 — the Phase 4 rust_perf proposers landed as
library code).

> **Distribution & open posture:** the repo is PUBLIC (MIT) and `cargo refine`
> is **on crates.io** (`cargo install cargo-refine`, 0.5.1 published
> 2026-11-11 — issues 221+225; the published manifest is generated by
> `scripts/cratesio_publish.sh` and carries ZERO private deps: the protocol
> tier is vendored in-repo, `src/katplane/`). It also ships as
> checksum-verified prebuilt binaries via GitHub Releases on the dist repo
> (macOS/Linux `install.sh` + Homebrew tap; Windows `install.ps1` + scoop)
> — the binaries additionally carry the hosted contribution surface (join,
> leaderboard, latent retrieval). The community repair corpus is **MIT open
> data today**.
>
> **The charitable posture is the shipped posture** (v0.4.0+): the release
> set carries the points plane (join/contribute/rank/corpus/update — the
> public network half, mainnet-only) and compiles out every payment/token
> surface (no `kat_service`, no `kat_transport`, no billing). Fixing is
> free and local; sharing fix data is a separate explicit opt-in; contribution
> points are non-transferable, expiring reputation — no payment, no token,
> no speculation.

## Status

**POC — multi-domain framework (Plan 001) + Phase 0 (rule distillation) +
L1 (drafter) + L2 (syntax validator) + additive self-evolution loop done.**

The rulewright pipeline that distills clippy rules into a ruliology-style
search space, drafts fixes via a modelless ternary SIMD matvec, and validates
with a `syn`-based syntax pruner. Heavyweight LoRA is the last-resort
fallback, not the co-equal path. Plan 001 generalized the framework to be
domain-pluggable behind the `HealDomain` trait.

| Layer | Status |
|---|---|
| L0 — Corpus (rule distillation) | ✅ Done (BLAKE3-committed freeze/thaw + seed corpus with 10 common lints + 46 perf rules + 2 kernel rules) |
| L1 — Ternary SIMD drafter | ✅ Done (Issue 001, zero-alloc matvec, p50=2.3µs) — ⚠️ **corpus-resident lints only**, see Issue 010 |
| L2 — Syntax validator | ✅ Done (Issue 002, `syn`-based parse check, p50=6.8µs) |
| L2 — Lint resolution check (clippy) | ✅ Done (Issue 003, `clippy_verify` feature, real `cargo clippy` JSON parsing, ~130ms/verify) |
| L2 — Test pass check (rust_perf) | ✅ Done (Plan 001 Phase 2, `rust_perf_verify` feature, real `cargo test` + compile-error detection via exit status + JSON parsing) |
| L2 — Compile check (kernel_opt) | ✅ Stub (Plan 001 Phase 3, `kernel_verify` feature, delegates to shared `RustAnalyzerPruner`; full CubeCL compile + numeric compare deferred per Proposal 012 §Phase 0) |
| L3 — Ruliology search | ✅ Done (Issue 005, `ruliology_search` feature, Pareto-front from katgpt-ruliology pattern, ~400µs/10-candidate search) |
| SE — Self-evolution loop (additive) | ✅ Done (Issue 004 + Issue 008; `self_evolve` DEFAULT-ON since Issue 018 with `RIIR_REFINE_SELF_EVOLVE=0` opt-out; G1–G5 + G6/G7 ALL PASS — 50.0pp fix-rate improvement on real `cargo clippy` feedback; Plan 005 T9.3 selection is `SelectionMode::BetaPosterior` — −8.4% selection regret vs the raw-rate blend, Bench 028) |
| Latent — span retrieval + suggest/fix CLI (Plan 005) | ✅ Phases 1–4 + 6 T6.1/T6.4 Done (`latent_retrieval` feature, opt-in; `cargo refine --suggest` — riir-rag `AstChunker`/`ModellessEmbedder`/`NeuronDbRetriever`, fan-out routing; GOAT G1–G4 PASS, Bench 018. Phase 4: top-5 rust_perf `propose_fix` proposers. Phase 6 T6.1: `cargo refine --fix [--write]` — **span-preserving** edit splice, visitor-gated; GOAT G1–G5 PASS, Bench 019 — G5 landed in katgpt-rs `katgpt-pruners` (commit `8c331cd9`, 12 files / 20 edits, 1233 tests green). T6.4: clippy proposers migrated to the shared emitter (`src/edit_emitter.rs`); `--fix` covers 5 perf rules + 33 structural clippy lints with fixpoint composition (Issue 016 expanded 7->33 + a 4-lint --verify tier, Bench 021); 3 type-blind clippy lints stay suggest-only). Phase 5 kernel proposers DEFERRED (concept-grade corpus — see Plan 005 §Phase 5). **Phase 3 REOPENED then partially recovered**: G1 is dead at scale (Bench 022), T3.5 lifted clippy top-1 54.2%→70.8% + perf→97.9% via the operator-aware tokenizer (Bench 027), still under the 80% bar; **Phase 3 CLOSED 2026-08-16**: T3.5.5's real-`cargo clippy` oracle + authored multi-lint spans gave the first uncontaminated measurement (+12.0pp, off 84.0%→96.0%), promoting `RerankMode::Structural`; clippy top-1 now 87.5% on the committed eval and 96.0% uncontaminated — both over the 80% bar  **Plan 005 COMPLETE as a document 2026-08-16**: Phase 7 closed as a negative result (T7.4 BLAKE3 commitment shipped), Phase 9 closed (T9.3 BetaPosterior default, Bench 028; T9.4 refuted, Bench 029), Phases 10/11/12/14 deferred on measured blockers (Bench 030), Phase 13 research half met (Research 058) |
| Router — kernel_opt expert routing | ✅ Corpus-wired (Plan 003, `KernelExpertRouter` wraps `pick_domain<15,5>` from Plan 558; 15 category centroids wired to 68-rule corpus; 13/15 reachable, 2 documented unreachable) |
| Thermal — kernel_opt tier dispatch | ✅ Stub (Plan 001 Phase 3, `KernelThermalRouter` + `ThermalTier` enum; Plasma→Hot→Warm→Cold→Freeze; returns Plasma for all calls) |
| L4 — LoRA fallback (all domains) | ⏳ Deferred to riir-train |

## Scope limit (measured, Bench 009)

The pipeline is a **fast cache over corpus-resident lint fixes**, not a general
clippy fixer. Measured on 10 lints (the seed corpus at bench time — since

grown to 40 via the Issue 016 structural expansion; the measurement's
scope-limit conclusion is unchanged):

| | in-distribution (corpus holds the fix) | held-out (same lints, unseen code) |
|---|---|---|
| exact-match | **10/10** | **0/10** |
| e2e throughput vs Bonsai-27B | ~74,000× tok/s | ~68,000× tok/s |

The L1 drafter returns corpus `fixed_code` **verbatim** — the input code selects
*which* stored fix to return but never influences the text (`Issue 010`).
On unseen code it therefore emits a valid-but-wrong function, and L2 reports
`parses = true` on it, so **a cache miss is currently silent**. Deploy it as a
cache with a fallback path, and do not read the throughput number as a
capability claim. For reference, Ternary-Bonsai-27B scored 10/10 on both
fixtures at ~16 tok/s.

## Multi-domain architecture (Plan 001)

The framework is domain-pluggable behind the [`HealDomain`](src/domain.rs)
trait. Each domain owns: (1) what counts as a fixable rule, (2) how to
validate a proposed fix against ground truth. The domain-agnostic core
(drafter + pruner + ruliology + self-evolve) operates on the corpus entries
this trait produces.

```text
src/
├── domain.rs           # HealDomain + HealEntry traits (the one new abstraction)
├── corpus.rs           # Corpus<D: HealDomain> — BLAKE3-committed freeze/thaw
├── drafter.rs          # TernaryDrafter (Stage 1) — generic over E: HealEntry
├── adaptive_drafter.rs # AdaptiveDrafter (Stage 2) — domain-agnostic
├── draft/              # Plan 005 latent retrieval (RuleIndex + DomainRouter + LatentMatcher)
├── pruner.rs           # RustAnalyzerPruner — domain-agnostic syntax check
├── ruliology_search.rs # Pareto-front candidate search (default-on)
├── self_evolve.rs      # HealPipeline + LatentFixMemory + LatentSkillEvolution
└── domains/
    ├── clippy_lints/   # feature "clippy_lints" (DEFAULT) — clippy::* lint fixing
    │   ├── corpus.rs   # the compiled-in clippy seed (120 corpus lints live — see the domain table)
    │   ├── verify.rs   # cargo clippy re-run (behind clippy_verify)
    │   └── proposers/  # 10 per-lint AST codemods (Issue 010 Stage 2)
    ├── rust_perf/      # feature "rust_perf" (default-on since 2026-08-13) — rust-optimize perf fixing
    │   ├── corpus.rs   # 302 perf rules (rust-optimize skill + perf-book + measured-GOAT/external distills; ledger in .docs/11_domains/rust_perf.md)
    │   ├── verify.rs   # cargo test re-run (behind rust_perf_verify)
    │   └── bench.rs    # perf-bench Pareto validator (behind rust_perf_bench, the GOAT gate)
    └── kernel_opt/     # feature "kernel_opt" (opt-in, 4090 DEFERRED)
        │   └── corpus/     # 1056 rules across entries_gpu_*.rs (Batch 1-335: popcorn + riir-gpu + measured-GOAT + external distills; ledger in .docs/11_domains/kernel_opt.md)
        ├── router.rs   # KernelExpertRouter wraps pick_domain<4,5> (Plan 558)
        ├── thermal.rs  # Plasma→Hot→Warm→Cold→Freeze tier dispatch (returns Plasma)
        └── verify.rs   # compile-only stub (behind kernel_verify)
```

**Cross-domain auto-routing — shipped as fan-out (Plan 005, 2026-08-14).**
The unified `cargo refine suggest` CLI runs with `--domain auto`: it **fans
out across all feature-enabled domains**, with the router verdict as a
per-span annotation (NOT a gate — the 8-D structural centroids mis-routed
fixtures, so gating starved the correct domain). Each domain is still
feature-selected; requesting a feature-off domain is a hard error. The bin
stays suggest-only — the Phase 4 rust_perf proposers landed as library
code; `--fix` CLI wiring is Phase 6.

### Feature matrix

| Feature | Default | Implies | Purpose |
|---|---|---|---|
| `batch_diversity` | off | `katgpt-core/diffusion_diversity` | Issue 222 (T1 landed 2026-11-11, axis (b) `e2d8744b`): the REPORT-ONLY batch-intake diversity readout (`Div_P^τ` per mined batch over the rule-embedding kNN graph) — one summary line per domain in the score-bench tail + `RIIR_REFINE_BATCH_DIV_FULL=1` the full table; never an exit-code change. Axis (b) MEASURED NEGATIVE: the corpus-relative ratio spans 0.9942–1.0065 (kernel_opt) / 0.9871–1.0055 (rust_perf) — the 8-D hashed embedding saturates at corpus granularity too; T2's correlation cell stays closed pending a richer rule embedding or batches ≥ 32. The axis stays standing as the future-corpus tripwire |
| `clippy_lints` | ✅ on | — | clippy lint-fixing domain |
| `clippy_verify` | off | `clippy_lints` | real `cargo clippy` lint resolution check |
| `rust_perf` | ✅ on | — | rust perf-fixing domain (Plan 001 Phase 2; promoted 2026-08-13, Bench 013 G1+G5) |
| `rust_perf_verify` | off | `rust_perf` | real `cargo test` test pass check |
| `rust_perf_bench` | off | `rust_perf` | perf-bench Pareto validator (the GOAT gate; Bench 013) |
| `kernel_opt` | ✅ on (via `kernel_pregate`) | `katgpt-core/variable_rank_domain_expert` | GPU kernel opt domain (Plan 001 Phase 3) — **DEFAULT-ON via `kernel_pregate` since 2026-11-06** (owner call; the real-corpus GOAT validator stays open as hygiene) |
| `kernel_pregate` | ✅ on (release set) | `kernel_opt` | Issue 208: the file-level kernel-signal gate + ±24-line span scope + the line/query-text caps (Bench-137 composed posture = the in-code defaults; the three envs DEMOTE-ONLY) — **DEFAULT-ON + shipped release set since 2026-11-06** |
| `kernel_verify` | off | `kernel_opt` | CubeCL compile check (STUB — full validator deferred) |
| `sec` | off | — | security-pattern fixing domain (Plan 018; FFI/Drop/ptr hygiene + the Batch-131 InputValidation suggest-only pair + the Batch-140 advisory-db unbounded-read rule + the Batch-165 wire-arithmetic overflow sink + the Batch-258 web-trust seed + the Batch-291 penthera CORS/JWT/cookie trio + the Batch-292 CVEngine client-IP/shell-string/mapped-IP/strike-count quartet + the Batch-294 RustSec masked-key/silent-cap pair + the Issue-185 softmax-collapse trio + the Batch-325 uniffi-rs FFI-hygiene quartet + the Batch-327 sanstun zeroize/cardinality/gateway-ctor trio + the Batch-328 oss-fuzz zip-slip rule + the Batch-329 voxl handshake-gate/mutation-gate pair + the Batch-330 hindsight SensitiveData trio (boundary-scan miss + Luhn-before-redact + fingerprinted audit previews, vectorize-io/hindsight @ 841979df MIT) + the Batch-331 hindsight round-2 InputValidation rule (externally-authored-edits-by-id-not-counted-index, id-addressing for externally-authored edits, same pin) + the Batch-340 SPILLWAY agent-egress-guard pair (removed-span-veto-outscopes-recovery-path + read-output-trust-inherits-argument-taint, saintmachines/spillway @ 0fc9f55f Apache-2.0) + the Batch-342 harness-adapters/safeagents pair (credential-material-in-command-argv + egress-control-bypass-flag, huggingface/safeagents @ 0de8e22d Apache-2.0 + atif-scan @ 0228cd9d MIT) + the Batch-337 jitengine reuseaddr probe-gate rule (conditional-reuseaddr-after-liveness-probe, skydiscover-ai/jitengine @ 2c34508d), 50 rules + 7 fixable proposers + 31 suggest-only detectors; the `--domain sec` CLI + boundary gate) |
| `docker` | **on** | — | Dockerfile bounded-fix drafting + retrieval (Plan 030 / Proposal 006; 27 rules, 14 bounded, `--domain docker` CLI + the DL3059-refusal gate). Zero new deps |
| `docker_verify` | off | `docker` | hadolint agreement oracle for the Class-A overlays; degrades to the Tier-0 stanza check with one log line when the binary is absent |
| `docker_buildcheck` | off | `docker_verify` | Tier-2 `docker build --check` (BuildKit's own linter) agreement oracle for the 4 rules whose incumbent is a check NAME, not a `DL` code. Stays opt-in on purpose: it needs a running daemon and a registry round-trip per non-`scratch` fixture (~1.3-2.2 s each), a strictly heavier dependency class than hadolint |
| `dist` | **on** | — | Issue 085: shipped-artifact build-config advice over name-matched `Cargo.toml` manifests (suggest-only, never a manifest write). **DEFAULT-ON since 2026-09-13** — all four GOAT promotion conditions PASS (Bench 082 + §6 addendum: precision fixtures, score-bench #92 FLAT, router floor 74/101 measured at default+dist, real consumer cites; B1/B2 scanner extensions `759e209d`) and the owner pull arrived (the Issue-085 fix directive). Zero new deps (the second hand-written no-`syn` scanner after docker) |
| `ruliology_search` | on | `clippy_lints` | L3 Pareto-front candidate search (DEFAULT-ON since 2026-11-06, Issue 212) |
| `elo_k_eff` | ✅ on | — | Bench 145 delegated-owner verdict AGREE (landed `280f15766` — that commit's "issue 190" is a double-allocated number; riir-refine Issue 190 is the grant/charitable item): the evidence-damped Elo schedule on the batch rating replay — `elo_k_eff = K·min(1, n/n_floor)`, n_floor = `ELO_K_EFF_FLOOR` (16) — + the REPORT-ONLY `regime:` line on the score-bench stdout. **DEFAULT-ON since 2026-11-07**; forwards `katgpt-core/step_noise` (consume, never fork); feature-off byte-identical (the DEMOTE); the damp rides `--recompute-elo`, the readout the clippy score-bench lane |
| `adoption_gate` | off | `katgpt-core/adoption_gate` | Plan 633 T1 / Issue 220 (landed 2026-11-10): the adoption/retention regression gate's consumer intake — the (rule, fix verdict) outcome window (`.refine/adoption_outcomes.jsonl`, blake3 outcome identities, never-double-file), the active-set demote/readmit gate (the challenger = the window minus the rule's rows; a `Keep` demote or a `repeated > 0` readmit refuses loud, an empty window abstains), and the seat head-swap watch (seat rows carry the receipt's build fingerprint; a build whose rows repeat known-rejected outputs stands the seat down for the run — in-memory, the manifest is never written). Consume, never fork: `katgpt-core/adoption_gate` is the primitive. Feature-off byte-identical by construction; G1 seam replay + gates in `tests/adoption_gate_gates.rs`; the REAL-store GOAT (plan 633 T3) opens with this consumer — no promotion talk before it |
| `span_shape_dispatch` | off (✅ in the release set) | `latent_retrieval`, `rust_perf` | Issue 216 T1 (landed 2026-11-07): the span-shape-keyed dispatch mask — the categorical AST-kind + needle classifier over the Issue-213 `DomainMask` path contract, touching ONLY annotated rust_perf rules; fail-open by construction (RerankMode::Off / parse failure / no recognized root item = all domains). **In the release set since 2026-11-11** (T4 cost PASS ×2 on quiet amo, Bench 144 — worst round p95 2.017 ms vs the 5.0 bar, six rounds under the p50 ≤ 1.0 / p95 ≤ 5.0 bars; NOT in cargo default BY DECISION: it implies `latent_retrieval` and the default build stays light of the rag tree); demote `RIIR_REFINE_SPAN_SHAPE=0` (the exact literal, DEMOTE-ONLY) |
| `consumed_window_ledger` | off | — | Issue 232 T1 (landed 2026-11-14): the consumed-window ledger — WHICH outcome windows each `--recompute-elo` replay consumed + a per-window REUSE counter (the poison variable is the reuse count, a non-monotone valid→poisoned curve — a boolean "burnt" flag would miss it); a reused window's re-validation verdict prints `UNDECIDABLE`, never folded into a refusal. Report-only; no implies (std + blake3); the traj-store hook composes `all(consumed_window_ledger, latent_retrieval)`; the correlation cell is the issue's standing gate |
| `half_split_agreement` | off | — | Issue 232 T2 (landed 2026-11-14): the half-split agreement readout — the corpus-is-the-model scorer built once per corpus view (trigger A = `buggy_code`, resolution Q = `fixed_code`) over a neutral probe set: pick-agreement / flip-rate + the same-scale top-1 gap (both views share the embedder — calibration by construction, no transform, no softmax). `--half-split-report` refuses exit 1 on BLIND reads (never a green zero); T3's predicate owns arming; report-only; no implies (std + blake3) |
| `multiplicity_gate` | off | — | Issue 232 T3 (landed 2026-11-14): the multiplicity-gated arming predicate — a holdout-flavored arm fires only past a LOCALLY MEASURED knee (the encounter-mass median, integer-exact; exact-half ties arm late), sigmoid-gated over the unit's OWN counter (never softmax — no other unit's counter can move it; `armed ⟺ gate ≥ 0.5` is the sigmoid's own half); the fence is the law's negative direction: a surface failing reuse-dominance (1-pass, the chain/quorum shape) never arms at any multiplicity. `--multiplicity-report` measures the real fixer-encounter surface in `.refine/trajectories.kv` (composes `all(multiplicity_gate, latent_retrieval)`); this box's store measured REUSE_DOMINANT, knee 51. Report-only; std only |
| `source_case_gate` | off | — | Issue 232 T4 (landed 2026-11-14, **CLOSED MEASURED-NEGATIVE** per the preregistered G2 bar): the source-case-first gate — a drafted fix must improve on the data that generated it (NoOp declines: fix byte-equals the trigger or is empty; Uphill declines: signed EXACT-integer cosine prefers the buggy exemplar) BEFORE the compile gate spends the Q half. No distance floor (the random axis stays a continuous reported metric — the preregistered bar owns any future decline class); the confusion readout measures the gate against the fixer's own outcome law. Report-only, no wiring; std only |
| `two_signal_composition` | off | — | Issue 232 T5 (landed 2026-11-14): the two-signal composition — a lone held-out window never single-handedly reverses an armed lane: a reversal commits only when BOTH landed signals agree (T1's window half FRESH — a consumed window's signal is UNDECIDABLE and corroborates nothing — AND T3's arming half corroborates below the knee or on the OnePass fence); the committed decision is ONE artifact carrying every fact that produced it, candidate probes evaluate on side state, the serving artifact mutates ONLY through the committed write path. No new threshold or constant — the composition ANDs two landed predicates. Report-only; std only; the correlation cell stays the issue's standing gate |
| `calibrated_dispatch` | off | `span_shape_dispatch` | Issue 233 T3 (landed 2026-11-14): the evidence-gated variant of the dispatch — the pinned `EVIDENCE_GROUPS` table carries per-(domain × shape) RAW census counts and derives the verdict (Wilson 95% LCB ≥ τ 0.6 over ≥ 20 labeled spans → Calibrated; measured under-floor = Fallback; < 20 or absent = Sparse — the paper's small-group rule); the Perf retain arms ONLY where EVERY carried bit is Calibrated (one Sparse/Fallback bit falls the whole span back to the full fan-out — the dispatch narrows, never widens). Landing census (static fixtures, 180 labeled spans): perf FnDecl 49/49 (LCB 0.927) + perf HotLoop 24/24 (LCB 0.862) Calibrated — the only consultable rows; perf TypeDecl n=10 Sparse (the three TYPE_DECL_ONLY annotations stay unnarrowed under the feature). GOAT recall half PASS (46/46 Perf spans narrowed, zero losses); the G2 cost leg (quiet-box window) owns promotion — opt-in until it passes. Evidence lane: `examples/calibrated_dispatch_census.rs` (report-only re-run instrument) |
| `index_cache` | off (✅ in the release set) | `latent_retrieval` | **In the release set since issue 226 T4 (2026-11-11)**: G3 restated as time-until-results ≤ +5% PASSED at +1.4% (warm −46%); demote with `RIIR_REFINE_INDEX_CACHE=0`, byte cap `RIIR_REFINE_INDEX_CACHE_MAX_MB` (default 256). Issue 224 T1–T4 (landed 2026-11-11): the content-addressed on-disk matcher index cache — blake3 key over the resolved per-domain corpus entries + retrieval config + embedder id + format versions (the leased epoch NUMBER deliberately excluded: identical content under a fresh lease hits); per-domain snapshots (rows/zone-map/rule-ids/bm25/anchor-mean) stored atomically (tmp+rename), corrupt/stale entries refuse LOUD and rebuild (digest verified BEFORE parse); LRU keeps 8 + touch-on-hit; T3 moves the auto-corpus 12 h stamp per-user (crate-local fallback byte-identical). Disarms under `kernel_route_narrow` + the Bonsai dense posture. Measured (amo QUIET AC, release, dapps sample crate): G1 PASS (gate test bit-identical + binary diff identical), G2 PASS (warm matcher gap 118–120 ms vs OFF 273–296, bar ≤150), G4 PASS (18.8 MiB disclosed), **G3 FAIL as written** (cold +69% — the one-time store costs ~233 ms on the exit path) — promotion BLOCKED on the G3 redesign (issue 226, delegated-owner verdict (b): keep opt-in; restated bar = time-until-results ≤ +5%). Demote `RIIR_REFINE_INDEX_CACHE=0` (the exact literal, DEMOTE-ONLY, promotion-ready) |
| `latent_retrieval` | off | `dep:riir-rag`, `dep:riir-neuron-db` | Plan 005: span-level latent retrieval + the `cargo refine suggest`/`fix` CLI (Bench 018 G1–G4 + Bench 019; T3.5 operator-aware BM25 tokenizer, Bench 027) |
| `clippy_verify` + `latent_retrieval` | off | — | T3.5.5 real-`cargo clippy` labeling oracle (`ClippyOracle`) + the uncontaminated G1 harness (`tests/uncontaminated_g1.rs`) |
| `bonsai_retrieval` | off | `latent_retrieval`, `riir-rag/bonsai_embedder` | Plan 005 T3.5 option 3: 768-D Bonsai dense rerank. **HEAVY** — 6.7 GB GGUF, ~48 s/embed on CPU. Measurement-only (Bench 027) |
| `self_evolve` | on | — | additive self-evolution loop (memory + EMA + `EvolveRecorder`; DEFAULT-ON since Issue 018, opt-out `RIIR_REFINE_SELF_EVOLVE=0`; BLAKE3-committed memory since T7.4; Beta-posterior selection since T9.3) |
| `rustc_errors` | **ON** | — | **Plan 019**: `cargo refine fix-compile` — oracle-driven compile-error repair (E0597/E0499/E0502 uncontested + the E0505 contested opening; delegated codes ship no template by the measured floor). GOAT at N=212: **105/105 uncontested fixed where `cargo fix` scores 0/105; 126/126 repaired cases behavior-green post-fix** (Bench 045). **PROMOTED DEFAULT-ON 2026-08-29 (owner call): driver-shaped (inert until invoked — zero runtime overhead), floor gate stands, dep-tree unchanged (`compile_oracle = []`)** |
| `ternary_inference` | off | `riir-engine`, `riir-gpu` | Bench 010: katgpt ternary generation on clippy prompts |
| `groq_first` | off | `dep:ureq` | Issue 051: groq-first free-tier LLM routing (`src/groq.rs`) — the Groq chat client + the groq→CF provider ladder (`router`, also needs `cf_workers_ai`). The daily drain reasons on the free tier FIRST and falls back to metered CF on limit; either leg may be absent (`GROQ_API_KEY` absent → pure CF, byte-identical; CF creds absent → a groq-only window whose exhausted free tier ends it cleanly), both absent → refused. Hardened: split connect/call timeouts, single-allocation auth header, token scrubbed from server-controlled error strings. `kat_drain --preflight` checks the ladder on an empty backlog at zero metered cost. Usage lands on the cost ledger's `providers.groq` block → the ops dashboard's absorption card |
| `compile_oracle` | off | — | Plan 019 T0.3: the real-`cargo` error-JSON capture plumbing for the `rustc_errors` domain (deliberately NOT implying `latent_retrieval` — a compile fixer must not grow a retrieval dep) |
| `fix_verify` | off | `compile_oracle`, `latent_retrieval` | Issue 015: the compile gate as tooling (fix applies → cargo-check → reverts only offending edits; real-repo GOAT Bench 031) |
| `score_bench` | off | `clippy_verify`, `latent_retrieval`, `clippy_lints`, `rust_perf` | Plan 014: `cargo refine --score-bench` — the frozen real-code fix bench + the committed score curve
| `margin_mining` | off | `score_bench`, `latent_retrieval` | Issue 077 T3: `e^{−2M}` margin-weighted carve order (`score_bench_mine_v6 --order margin` / `--ab`) — RECORDED DEMOTE (Bench 080: tie at every budget; retrieval-similarity margins don't select lint-bearing raw spans), stays opt-in, the carve default stays count-desc |
| `l4_lora` | off | — | Plan 336 T4.4/T5.3: the L4 generative-fallback SEAM (`L4Fallback` trait + `DaemonL4` transport) — inert unless a fallback is installed; Plan 336 G1 measured 0/60, stays UNARMED (see [`.docs/10_self_evolve/l4_fallback.md`](.docs/10_self_evolve/l4_fallback.md)) |
| `pav_data` | off | `l4_lora`, `score_bench` | Plan 356 A1: PAV (process-advantage) dataset curation for the L4 fixer campaign (real-daemon or seeded-mock source; opt-in — the campaign is a deliberate 30-40h exclusive-4090 budget) |
| `contrastive_scope` | off | `katgpt-core/contrastive_scope` | Bench 040 / Plan 016: the input-scope gate at the fix seam — non-Rust garbage is declined whole before it can burn an LLM call (G1 8/8 OOS declined, bit-identical in-distribution) |
| `kat_service` | off | `dep:ssh-key`, `dep:ed25519-dalek` | INTERNAL-ONLY (the retired KAT credit plane's client substrate; compiled out of every shipped build since the v0.4.0 pivot — the public network half is `points` below) |
| `kat_transport` | off | `dep:ureq`, `dep:qrcode`, `kat_service` | INTERNAL-ONLY (the retired KAT transport lane; the corpus-lease READ moved to `points`/`lease_http` — issue 203 — so the shipped build never carries this) |
| `cf_workers_ai` | off | `dep:ureq`, `latent_retrieval`, `clippy_verify`, `l4_lora`, `kat_service` | Issue 040/046: the CF Workers AI REST client (`src/cf_workers_ai.rs`) — the fixer arms, the USD daily-cap ledger (reserve-then-reconcile), and the fixer model: DEFAULT `@cf/zai-org/glm-5.3-flash` (owner pivot 2026-08-28, second call — the prior default 4.7 FAILED the Issue 043 re-designation gate 3/9 and stays priced + selectable via `RIIR_CF_MODEL`; [Bench 057](.benchmarks/057_glm53_flash_fixer_regate.md) re-gated 5.3 at 7/9 strict-keep on BOTH arms, beating the earlier CF bar of [Bench 049](.benchmarks/049_fixer_regate_poc.md)) |
| `points` | off (✅ in the release set) | `dep:riir-ledger`, `riir-kat/lease_http` | The PUBLIC network half (Plan 325 / Proposal 026): the contribution-points client — `join` (GitHub device flow) / `leave` / `rank` / `info` / `corpus` / the issue-204 contribute feed (joined machines auto-share run outcomes — per-rule counts only) + the issue-203 corpus-lease read (GET-only, never billed). Points are non-transferable, expiring reputation |
| `wasm_pruner` | off | — | WASM rust-analyzer pruner (L2 sandboxed variant — deferred on rubrc maturity) |

## Architecture

```text
Domain corpus (BLAKE3-committed freeze/thaw)
  clippy_lints: (lint_name, lint_group, buggy_code, fixed_code) tuples
  rust_perf:    (rule_id, name, category, buggy_code, fixed_code) tuples
  kernel_opt:   (rule_id, gpu_concept, ref_shape, opt_pattern) tuples
         │
         ▼
L1 — MODELLESS DRAFTER (PlasmaPath ternary SIMD matvec)
  Frozen direction vectors (.bits files)
  Template fallback when no weights loaded
  Zero-alloc hot path
         │
         ▼
L2 — VALIDATOR (always-on, domain-specific verifier opt-in)
  `syn` in-process parse check (fast, modelless, always-on)
  clippy_lints: `cargo clippy` re-run (feature `clippy_verify`)
  rust_perf:    `cargo test` re-run (feature `rust_perf_verify`)
  kernel_opt:   CubeCL compile (feature `kernel_verify`, STUB)
         │
         ▼
  [ validated fix ]
         │
         ▼ (opt-in, additive — feature `self_evolve`)
SE — SELF-EVOLUTION LOOP (composes on top of L0–L2)
  LatentFixMemory   : 4-axis retrieval over past fix trajectories
                     (latent dot+sigmoid, BM25 lexical, evidential-tier
                     trust prior — Certified/Heuristic/Withdrawn — with
                     Withdrawn never suggested; identical-triple dedup)
  LatentSkillEvolution: EMA per-lint direction-vector update
  Fixes + records outcome → next-call retrieval benefits
  Degenerates to static-corpus path when memory is empty
```

### kernel_opt-specific layers (Plan 001 Phase 3)

```text
Kernel signature (M, N, K, dtype, hw)
         │
         ▼
Router — KERNEL EXPERT ROUTER (pick_domain<15,5>, Plan 558)
  Activity vector · category_centroids → expert index → category → rule_ids
  Plan 003: 15 category experts wired to 68-rule corpus (hand-crafted centroids;
  corpus distillation via CommittedFieldBlend deferred; 13/15 reachable)
         │
         ▼
Thermal — KERNEL THERMAL ROUTER (5-tier escalation)
  Plasma (modelless cache, M3 Metal) → Hot (Stage 2 adaptive) →
  Warm (LoRA draft, 4090 CUDA, DEFERRED) → Cold (full validate, DEFERRED) →
  Freeze (archive, DEFERRED)
  Scaffold: always returns Plasma (modelless-first mandate)
```

## L3 — Ruliology candidate search

When the L1 drafter proposes multiple candidates and L2 accepts more than one,
L3 enumerates the Pareto-front of (lint_satisfied, AST delta size) and picks
the minimum-complexity fix among the Pareto-optimal set.

The Pareto-front algorithm is reimplemented locally (~30 LOC) from
`katgpt-rs/crates/katgpt-ruliology/src/types.rs::WinMatrix::pareto_front`
(GOAT PASS, Bench 572). The substrate's version is coupled to `WinMatrix`
(2-player tournament semantics) + non-optionally pulls `katgpt-pruners`
(g_zero feature) — too heavy for clippy's needs. Clippy fixes have ground-truth
payoff (lint_satisfied: bool), not pairwise tournament payoffs, so the
`WinMatrix` type doesn't fit. This mirrors the repo's existing pattern:
`ClippyTernaryDrafter` reuses the `TernaryDraftModel` pattern from
`quest_grammar` without a code-level dep.

## Modelless-first mandate

The primary path (L0–L3) is entirely modelless — no training, no backprop:
- **L0**: rule distillation (named rules + bounded fix-space)
- **L1**: ternary SIMD matvec drafter (frozen direction vectors)
- **L2**: rust-analyzer validation
- **L3** (default-on since 2026-11-06): ruliology Pareto-front candidate search
- **SE** (opt-in, additive): self-evolution loop — latent retrieval + EMA
  direction update. No gradient descent; the only state mutations are
  append-only memory + EMA direction refresh, matching the global
  modelless-first mandate.

The LoRA fallback (L4) is deferred to riir-train and is the conservative last
resort, not a co-equal path.

## Install

Any OS with a Rust toolchain — from crates.io (the code-fixing build):

```sh
cargo install cargo-refine
```

Prebuilt binaries (no toolchain needed to install; they additionally carry
the hosted contribution surface — `cargo refine join`, the leaderboard,
latent retrieval) — macOS / Linux (Homebrew — `brew tap` resolves `github.com/riir-ai/homebrew-tap`;
Homebrew 6+ refuses untrusted taps, so the trust step is part of the install):

```sh
brew tap riir-ai/tap
brew trust riir-ai/tap
brew install riir-ai/tap/cargo-refine
```

or the checksum-verified script (macOS/Linux):

```sh
curl -fsSL https://raw.githubusercontent.com/riir-ai/cargo-refine/main/install.sh | sh
```

Windows (PowerShell):

```powershell
scoop bucket add riir-ai https://github.com/riir-ai/scoop-bucket
scoop install cargo-refine
# or the script:
irm https://raw.githubusercontent.com/riir-ai/cargo-refine/main/install.ps1 | iex
```

Every prebuilt install is checksum-verified against the release's
`SHA256SUMS`; each release carries `THIRD_PARTY_LICENSES.md` + the EULA beside
the binary. The crates.io crate is MIT; upgrade it by re-running
`cargo install cargo-refine`.

## Quick Start

```sh
# The everyday user path (the shipped binary): dry run, then fix
cargo refine            # dry run — lists what it would fix, changes nothing
cargo refine fix        # apply the fixes (compile-checked; review first)

# Developer demos from source:

# Run the clippy fix demo (L0 corpus → L1 drafter → L2 pruner)
cargo run --example heal_demo

# Run the rust_perf domain demo (Plan 001 Phase 2; default-on since 2026-08-13)
cargo run --example perf_heal_demo

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# Re-run GOAT benches (results persisted in .benchmarks/001_goat_consolidated.md)
cargo bench --bench bench_001_l1_matvec      # L1 hot path
cargo bench --bench bench_002_l2_pruner_syn   # L2 hot path
cargo run --example ruliology_search_demo --features ruliology_search --release  # L3 opt-in

# Self-evolution real-world validation (~30s; spawns real cargo clippy)
cargo run --example self_evolve_real_world_demo --features "self_evolve clippy_verify" --release

# Latent suggest over real files (Plan 005; opt-in — pulls riir-rag)
cargo run --features latent_retrieval --bin cargo-refine -- suggest src/

# Span-preserving fix (dry run, then the real fix — writes + compile-gates)
cargo run --features latent_retrieval --bin cargo-refine -- src/
cargo run --features latent_retrieval --bin cargo-refine -- fix --quiet src/
```

## Contributing fixes (opt-in)

Fixing is local: it runs offline and needs no account. Sharing fix data is
a separate, explicit opt-in. Contributions are anonymized and secret-blind —
the redaction layer never lets credentials or secrets leave the machine —
and they build the MIT-licensed community corpus: every contributed repair
is reviewed, never scraped, and improves fixing for everyone.

```sh
cargo refine join       # join the contribution network (GitHub device flow, once)
cargo refine fix        # heal the crate (free, local, compile-checked)
                        #   — joined machines share run outcomes automatically;
                        #     not joined: nothing is ever sent
cargo refine rank       # the public leaderboard + your rank (joined identities)
cargo refine info       # the one-command stats view (local + corpus)
cargo refine corpus     # corpus info + the rule-set refresh offer
cargo refine update     # refresh the rule set (the corpus lease read — free, never billed)
cargo refine leave      # leave the network — unbinds the identity, zero data sent
cargo refine upgrade    # update the binary itself (SHA256SUMS-checked)
```

The consent screen at first `fix` explains exactly what a contribution
carries (run outcomes — per-rule written/declined counts; never spans,
source or paths) before anything is ever shared. `cargo refine scan --blind
FILE` shows what the scanner would redact in your own files. `leave`
unwrites the join locally and server-side — zero data flows afterwards.

**Offline mode.** `RIIR_REFINE_OFFLINE=1` (or `cargo refine offline` to
persist the choice) turns every network lane off — local healing never
needs the network and keeps working fully offline.

**What the tool keeps on your machine:** fix history and the rule-set cache
under your repo's `.refine/`; the machine-local account/identity state under
`~/.config/` (deleted by `cargo refine leave --purge`, which is explicit and
never the default).

## Automatic rustfmt (`cargo refine fix`)

Fixed files are **rustfmt'd automatically** after the write — default-on,
`--no-fmt` to disable. No manual post-fix formatting step, no SKILL
required — the discipline is built into the tool:

> **This repo's own posture (issue 223 T4):** format every `.rs` you touch
> with bare `rustfmt <file>` before committing — the repo is style-2024
> gated (guard Layer 8, `scripts/fmt_gate.py`; never `cargo fmt --all` or
> `cargo fmt -p X -- a.rs` here — see AGENTS.md §Formatting).

- **fmt-clean-at-HEAD gate**: rustfmt only touches a fixed file when the
  *original* (pre-fix) content was already rustfmt-clean. Files with
  pre-existing formatting drift are fixed but **left unfmt'd** — a
  mechanical fix never drags unrelated formatting churn into your diff.
- **CRLF skip**: CRLF files are reported (`fmt: skip (CRLF)`) and never
  converted (rustfmt would rewrite every line ending).
- **Edition-aware**: the rustfmt edition comes from the owning package's
  `Cargo.toml` (`edition = "..."`, default `2024`) — bare-file rustfmt
  defaults to a pre-2024 edition and fails on let-chains.
- **Never fatal**: rustfmt missing from PATH or a parse failure warns and
  skips; the fixed file is always left as valid Rust.

A dry run (bare `cargo refine`, no `fix`) reports the **fmt plan** per file
(`fmt: would-format` / `skip (pre-existing drift)` / `skip (CRLF)`) plus a
run summary:

```text
$ cargo-refine src/crate/
fix src/crate/foo.rs (12 edit(s)):
  [uninlined_format_args   ] line 41
  fmt: would-format
fix src/crate/bar.rs (3 edit(s)):
  ...
  fmt: skip (pre-existing drift)
scanned 2 file(s), ... fmt: 1 would-format, 1 skip pre-existing drift
```

The same summary appears after a real fix (`fmt: 22 formatted, 70 skip
pre-existing drift, 3 skip CRLF`). Under the compile gate the fmt pass applies
after the check, to files with kept edits only.

## clippy-bench — the frozen fix score + SVG curve (Plan 014)

The bench that answers "are we getting better?" with a number that can
only move by fixing better — **the denominator is frozen** (34 fixtures:
real pre-sweep extracted spans + the T3.5.5 realistic multi-lint composite set,
provenance recorded per fixture), and the ground truth is real
`cargo clippy` before/after, not our own proposers.

![clippy-bench score](.benchmarks/score_history.svg)

Axes: `heal_rate` (up) · `created_rate` (RED, down — false positives)
· `parse_safety` (fixed output compiles) · `decline_correctness` (clean
code left untouched — declining is a correct answer). The console report
also prints the **miss map** (which lints persist, where fixing fails).

Run it after any proposer/corpus change; each run appends one point to
`.benchmarks/score_history.jsonl` and re-renders the SVG:

```text
cargo run --features score_bench --bin cargo-refine -- --score-bench
```

Run #1 (2026-08-19): heal 84.6% · created 3 · safety 92.9% · decline 100% —
and it caught a REAL proposer bug on its first execution (Issue 032:
`needless_range_loop` × `identity_op` same-pass composition half-applied
the rewrite → dangling loop variable). **Run #2 (fix `86ba7f1`, same day):
heal 90.4% · safety 100%** — the first upward step on the curve. **Run #3
(`c8d0d34`, same day): heal 98.1% · created 1 (1.9%)** — the run-#2 miss map
triaged to root causes: 5 probe-verified proposer extensions (bare `return;`,
call-arg paren wraps, the 2-arm bool match shape, macro-arg descent for
`needless_range_loop`, `useless_format` single-arg) + `static-str` emitting
`const fn` (closing both `missing_const_for_fn` creations) + a latent
`static-str` trait-impl soundness fix. The two residuals are documented
boundaries: `single_option_map` (nursery — clippy's own `manual_map` fix
produces the same cascade) and one String-typed `useless_format` persist
(the type-aware terminal fix is beyond parse-only reach). Expect a STEP
curve at root-cause fixes, not a training curve — the self-learning
mechanisms measured negative and were killed; improvement is
engineering-iterative and the curve makes it honest.

## Watch mode (real use) — Plan 019 Phase 5

`scripts/watch_heal.sh` wraps `cargo refine fix-compile --write` in a watch
loop for real session use: your editor saves, the oracle repairs what it
can, the miss queue collects what it can't.

```bash
./scripts/watch_heal.sh                    # session-touched .rs files only
./scripts/watch_heal.sh src/foo.rs ...     # an explicit scope
HEAL_FEATURES="--features my-gate" ./scripts/watch_heal.sh   # the repo's real feature set
HEAL_BIN="cargo run --features rustc_errors --bin cargo-refine --" ./scripts/watch_heal.sh  # dev build
```

Three recorded caveats are REQUIREMENTS of any wrapper — the script
encodes them, and a hand-rolled variant must too:

1. **Write semantics are clean by construction.** `fix-compile` NEVER
   writes without `--write` (pinned by test) and a reverted candidate is
   byte-identical to its pre-run content. There is no unconditional-write
   trap.
2. **`--verify-args` MUST carry the repo's real feature set.** The oracle
   is `cargo check`; a fix that compiles under default features can break
   a gated config (the Issue 031 H3 class). Default: `--all-features` (the
   safe superset) — narrow for speed only once you know your repo's shape.
3. **NEVER whole-repo scoping on a shared checkout.** Default scope =
   session-touched files (`git diff --name-only HEAD` + untracked `.rs`),
   the recorded sibling-collision discipline. The watch TRIGGER may be the
   tree; the fix scope stays the list.

Self-evolution state lives at `<repo>/.refine/`: `trajectories.kv/` (the
durable neuron-db store, `latent_retrieval` builds — encrypted WAL + BM25
+ the 8-D latent axis), `memory.json` (the default-build fallback), and
`misses/` (the template-authoring queue — each miss is a self-contained
authoring request with the rendered diagnostic). Export either backend:

```bash
cargo refine --dump-trajectories > trajectories.jsonl
```

Debug-query the loop state (Issue 037 observability — one command, no jq,
works from any repo with the installed binary; summarizes the JSON backend
always + the repo-local kv store when present, incl. the E1 record / E2
verdict / E3 learn loop-state verdict):

```bash
cargo refine --memory-stats
```

Concurrent sibling builds: export a private `CARGO_TARGET_DIR` before
launching (see the script header for the isolation recipe — the global
rule; the oracle shares your target dir by default).

## Ops surface (Cloudflare workers + nightly) — Issues 044/045

This repo also hosts the **LLM ops plane**: the cost/budget housekeeping
worker, the ops dashboard, and the nightly generator. State as of
2026-08-29 (verified live):

| Piece | URL / where | State |
|---|---|---|
| Housekeeping worker | `https://riir-kat-housekeeping.foxfox.workers.dev` | **LIVE** — hourly cron, T9 code deployed, `READ_TOKEN` gate on (`/hours` `/days` `/backfill` anon → 401), `CLOUDFLARE_OPS_API_TOKEN` set (`/status` → `token_set:true`, neurons + containers collecting, 14-day backfill run) |
| Ops dashboard | `https://riir-ops-dashboard.foxfox.workers.dev` | LIVE — serves the self-contained HTML; fetches `/latest` from the housekeeping worker. **Auth gate ships OPEN** (owner decision 2026-08-28; one `wrangler secret put DASHBOARD_TOKEN` closes it) |
| Nightly generator | `scripts/ops_nightly.sh` | **ON-DEMAND ONLY** — the `com.riir.ops-nightly` launchd schedule was RETIRED 2026-10-16 (riir-dapps Issue 127; the plist file is deleted from the tree — the bootstrap command below no longer applies). Run the script by hand when a dashboard refresh is wanted; it sources `.env` itself. The one cheap sub-step that had to KEEP a schedule — the corpus-days display push — lives in riir-dapps as its own job (`riir-dapps/scripts/launchd/com.gist.rs.corpus-days-sync.plist`, daily 04:17 on the M3; the gen-2 display-loss incident, plan 060 5th pass, is why it exists) |

**Credentials live in `.env` (gitignored — never commit, never print).**
[`.env.example`](.env.example) is the documented template: what each var is
for, which consumer reads it, and where each is minted. The four core vars:
`GROQ_API_KEY` (free-tier LLM routing, Issue 051) · `CLOUDFLARE_OPS_API_TOKEN`
(Account Analytics:Read + Workers AI — nightly generator, Rust CF leg, and
pushed as the housekeeping Worker secret `CLOUDFLARE_OPS_API_TOKEN`; the only one that
cannot be minted from this machine) · `CLOUDFLARE_API_TOKEN` (least-privilege
deploy token wrangler uses natively; absent → `wrangler login` OAuth) · `READ_TOKEN`
(the housekeeping read gate, 64-hex). Optional: `ALERT_WEBHOOK_URL` +
`MONTHLY_BUDGET_USD` (cap-crossing paging — inert until set; crossings
still land in KV + `/status alerts_today`) · `DASHBOARD_TOKEN`.

Deploy rule: `CLOUDFLARE_API_TOKEN` is the only token wrangler uses —
plain `npx wrangler deploy`. The analytics token is `CLOUDFLARE_OPS_API_TOKEN`, a
name wrangler never reads, so it cannot shadow a deploy.

Full narratives: Issues 044 (dashboard + alerting + nightly) and 045
(housekeeping worker, rollout, backfill) RESOLVED + removed 2026-09-01
per the noise-reduction rule — the live state + owner-gated residuals
are documented in [AGENTS.md](AGENTS.md) §"CF ops surface"; the full
narratives live in git history (`git log -- .issues/044_*` /
`git log -- .issues/045_*`).

## Coverage (measured, Bench 045 — the honest product numbers)

Over the N=212 known-inverse mutation corpus (12 committed anchors + 200
generated; every seed carries a ground-truth behavior test):

| error family | `cargo fix --broken-code` | `cargo refine fix-compile` | verdict |
|---|---|---|---|
| E0597 (borrow outlives owner) | **0%** | **100%** | net-new (uncontested) |
| E0499 (two `&mut` alive) | **0%** | **100%** | net-new (uncontested) |
| E0502 (borrow across mutation) | **0%** | **100%** | net-new (uncontested) |
| E0505 (move out of borrowed) | 100% but **100% silently diverges** (`&v.clone()` changes aliasing) | **100% matched** | the contested win |
| E0596 / E0382 / E0507 / E0373 | 100% matched | no template — delegated | `cargo fix` owns these |

- **Uncontested stratum: 105/105 fixed** where the floor is 0/105.
- **Never-wrong by construction:** every kept fix strictly reduces the
  coded-error count (full `cargo check` verdict); everything else reverts
  byte-exact. Non-compiling edits surviving a run: **0** (pinned by test).
- **Semantics honest:** 126/126 repaired cases pass their seeds' behavior
  tests post-fix; every semantics-changing template (clone-instead-of-
  borrow, drop-order, ownership transfer, lifetime loosening) prints its
  `⚠ semantic change` note in the diff — never silent.
- Coverage is partial BY DESIGN and per-family numbers are the product
  claim, not a blended total. Misses land in `.refine/misses/` — the
  template-authoring queue.

## Usage

```rust
use riir_refine::{build_seed_corpus, ClippyTernaryDrafter, RustAnalyzerPruner};

// Build the corpus from seed entries (or your own)
let corpus = build_seed_corpus();

// Draft a fix (fallback mode = corpus template lookup)
let mut drafter = ClippyTernaryDrafter::fallback();
let candidates: Vec<_> = corpus.lookup("needless_return").cloned().collect();
let drafted = drafter.draft_fix("needless_return", buggy_code, &candidates)?;

// Validate with the syn-based syntax pruner
let pruner = RustAnalyzerPruner::new();
let result = pruner.validate(&drafted.code, "needless_return");
if result.is_valid() {
    // Apply the fix
}
```

## References

- [Proposal 001](.proposals/001_multi_domain_modelless_code_healer.md) — the multi-domain meta-architecture argument
- [Plan 001](.plans/001_multi_domain_framework_refactor.md) — execution plan (Phases 1–5)
- [Bench 001](.benchmarks/001_goat_consolidated.md) — consolidated GOAT gate results (L1 + L2 + Phase 3 + L3)
- [Bench 008](.benchmarks/008_real_world_validation.md) — real-world validation of `self_evolve` via `cargo clippy` (G6 + G7)
- [Bench 009](.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md) — e2e tok/s vs Ternary-Bonsai-27B on llama.cpp (~74,000× throughput; 0/10 held-out quality)
- [Bench 011](.benchmarks/011_stage2_goat.md) — Stage 2 adaptive drafter GOAT (Issue 010 resolution)
- [Plan 005](.plans/005_real_file_span_healing.md) — real-file span-level healing (latent suggest Phases 1–3, proposers Phase 4, `--fix` Phase 6 T6.1; kernel Phase 5 deferred with scope verdict)
- [Bench 018](.benchmarks/018_plan005_latent_suggest_goat.md) — latent `--suggest` GOAT (G1–G4 ALL PASS)
- [Bench 019](.benchmarks/019_plan005_phase6_fix_span_preserving_goat.md) — span-preserving `--fix` GOAT (G1–G4 PASS; G5 blocked on sibling rebase)
- [`.docs/10_self_evolve/`](.docs/10_self_evolve/) — design notes for the self-evolution loop (HealPipeline, LatentFixMemory, LatentSkillEvolution)
- [Proposal 034](../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the original clippy-healing design argument
- [Proposal 012](.proposals/) — GPU kernel optimization (absorbed into Plan 001 Phase 3 as `kernel_opt` domain)
- [Plan 558](../katgpt-rs/.plans/) — `pick_domain` substrate (consumed by `KernelExpertRouter`)
- [Plan 331 Phase A](../riir-train/.plans/331_lora_rust_coder_candidates_consolidated.md) — Gemma-2 r16 production (LoRA fallback base)
- [Plan 333](../riir-train/.plans/333_bitnet_ternary_moe_neuro_symbolic_poc.md) — Ternary-Bonsai-27B (LoRA fallback primary base)
- [RustAssistant](https://arxiv.org/abs/2412.16358) (Microsoft Research, 2024) — 75% prior art ceiling
- [Proposal 011](../katgpt-rs/.proposals/011_rust_swe_bench_latent_space_via_wasm_pruner.md) — WASM rustc pruner (rubrc)

## License

MIT OR Apache-2.0
