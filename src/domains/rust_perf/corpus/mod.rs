//! Seed corpus — rust performance rules distilled from the `rust-optimize`
//! skill checklist.
//!
//! Each entry is a `(rule_id, name, category, buggy_code, fixed_code, context_notes)`
//! tuple representing a real perf pattern from the skill. The corpus is
//! BLAKE3-committed via [`Corpus::new`](`crate::corpus::Corpus::new`). The
//! `context_notes` field is `Option<String>` — `Some(...)` for rules the bench
//! validator found ParetoDominated/Mixed/Tie in the default config (Bench 013 /
//! 038) or that are NotBenchable-with-a-metric-mismatch (Issue 014), `None` for
//!      unconditional wins + honestly `NotBenchable` rules.
//!
//! 318 entries across 15 categories as of Batch 356 (+4: allocation +2
//! `mem-take-field-checkout-empty-window` + `clone-before-mutating-loop-borrow-appeasement`,
//! concurrency +1 `thread-spawn-per-call-pool-reuse` (BENCHABLE, the
//! batch lead), layout +1 `dense-id-domain-hashmap-to-slot-array`
//! (BENCHABLE) — the r/rust AI borrow-checker-appeasement case study
//! (Plan 366 / Research 400, `entries_72`; post-only source,
//! decision-content-only transcription; +1 count-neutral rider face on
//! `bounded-batch-buffer-pool`, `entries_06` — the cross-thread
//! return-ring enforcement form; the batch's R4
//! `cross-thread-buffer-return-ring` DROPPED at authoring on that
//! adjudication, the drop recorded in the `entries_72` header);
//! Batches 350–355 added zero `rust_perf` rules (`kernel_opt`/sec lanes);
//! prior: 314 entries across 15 categories as of Batch 349 (+2: arithmetic +1
//! `deferred-integer-lane-pooling-per-row-exponent`, complexity +1
//! `incremental-prefix-hash-wordpiece-scan` — the encoders distill
//! (Plan 360 / Research 393, `entries_71` — `flowercomputers/encoders` @
//! `d46c90e45f`; +1 count-neutral rider face on `ascii-run-swar-block-skip`,
//! `entries_29` — the pretokenizer stage-fusion face, its atoms owned);
//! prior: 312 entries across 15 categories as of Batch 345 (+5: two
//! measurement-honesty (`benchmark-spread-above-threshold-reports-unstable`,
//! `macos-pressure-reads-footprint-not-rss`), one concurrency
//! (`slow-consumer-latest-value-single-slot`, the tokio family's latest-wins
//! coalescing axis), two io (`untrusted-download-verify-then-atomic-install`,
//! `ui-event-stream-sampled-gate-terminal-forward`); the `LocalSR` distill
//! (Plan 357 / Research 389, `entries_70` — `HerRei/local-upscale` @
//! `cf6f8b55`); prior: 307 at Batch 343 (+2: caching —
//! `raw-bytes-with-hit-backfill-once` +
//! `vanished-entry-tolerant-eviction-panic-else`, the sccache cache-layer
//! RE-distill (Plan 355 / Research 387, `entries_69` — the tiered-hit
//! backfill family, the files B324 did not mine); prior: 300 entries across
//! 14 categories as of Batch 332 (+2: dispatch —
//! `byte-flag-table-masked-run-scan` + `token-batch-cursor-handover`, the
//! blazing-smtlib distill (Plan 340 / Research 370); prior: 298 at Batch 331
//! (+1: concurrency —
//! `batch-retry-ladder-classifies-failure-before-permanent-state`, the
//! vectorize-io/hindsight round-2 pool (Plan 339 / Research 368 + addendum);
//! prior: 297 at Batch 330 (+1: concurrency —
//! `fanout-first-error-leaves-siblings-running`, the vectorize-io/hindsight
//! distill (Plan 338 / Research 368, `entries_63`); prior: 296 at Batch 329
//! (+2: dispatch +1
//! `event-damage-map-selective-partition-skip-with-consumer-hints` +
//! concurrency +1 `online-cost-fit-thread-ladder-reopen-hysteresis`, the
//! tembo/voxl distill (Plan 337 / Research 367, `entries_61` + `entries_62`);
//! prior: 294 at Batch 326 (+1: layout —
//! `adaptive-sparse-blocked-table-encoding`, the rust-lang/rust #164484
//! distill (Plan 330 / Research 363, `entries_60`); prior: 293 at Batch 327
//! (+1: concurrency — `batch-reserve-monotonic-counters-offload-bulk`, the
//! firezone/sanstun distill (Plan 332 / Research 364, `entries_59`)); prior:
//! 292 at Batch 324 (+3: io — the sccache distill (Plan 328 / Research 362,
//! `entries_58`); prior: 289 at Batch 318 (+2: concurrency
//! +1 `generation-counter-stale-timer-invalidation` + io +1
//! `exclusive-mkdir-temp-resource-reservation`, `entries_56` -- the
//! impeccable io stretch-tier remainder (plan 316's `- [-]` rows),
//! Plan 318 / Research 356, upstream pbakaus/impeccable @
//! `02523aec3e4c4580102d996d1b6a71fbe062e03f`, Apache-2.0, both ANNOTATED
//! `NotBenchable` `bench_spec` `None`) on the prior state: Batch 316
//! (+5: io +4,
//! concurrency +1, `entries_55` -- the impeccable io/process robustness
//! pool, Plan 316 / Research 356, upstream pbakaus/impeccable @
//! `02523aec3e4c4580102d996d1b6a71fbe062e03f`, Apache-2.0, all ANNOTATED
//! `NotBenchable` `bench_spec` `None` + ONE count-neutral rider face: the
//! box-filter area-average face on `prefix-sum-2d` (`entries_01`, the
//! same 2D-window-sum family -- the source's resize is area-averaging
//! down, bilinear up)) -- prior: Batch 308 (+2: caching +2, `entries_54`
//! -- the 10-29b staged serving pool + `rust_perf` row: the cross-tier
//! LRU-recency law + the refusal-quarantine verdict law, Plan 313 /
//! Research 352) -- prior: Batch 305 (+3: concurrency +2,
//! allocation +1, `entries_53` -- the bbtree ordered-structure-internals
//! distill COMPLETION, Plan 289 / Research 322: `batch-miss-interleave-tiny-executor`
//! (concurrency, BENCHABLE latency bar — N independent dependent-chain
//! lookups interleaved through a no-alloc bit-mask round-robin executor,
//! one chain's miss overlaps the other lanes' work; 10.7-11.3x measured on
//! the landing box) + `seqlock-copy-read-unlatched` (concurrency, ANNOTATED
//! `NotBenchable` — a `Copy` value under optimistic concurrency is read by
//! copy-then-version-check, never by taking the shared latch; sound ONLY
//! for Copy values) + `page-self-aligned-hot-node` (allocation, ANNOTATED
//! `NotBenchable` — hot >=512 B nodes (`>=512 B` hot floor) sized to
//! power-of-two page multiples,
//! self-aligned: one TLB entry, the prefetcher streams the node; the
//! source's own 1 KB sweep is the measured basis, bigger is not better);
//! Plus the two rider faces the partial landing promised: the compile-time
//! hand-specialization face (dtolnay typeid trick) folded as a citation on
//! the `entries_52` window rule, the D28 kinship cite on
//! `loop-invariant-dispatch-resolution-hoist` (`entries_48`) — upstream
//! XiangpengHao/bbtree @ `8adae97a`) — prior: Batch 298 (+1: caching,
//! `entries_52` — the bbtree ordered-structure-internals distill, Plan 289 /
//! Research 322: `order-preserving-inline-key-encoding` (the fences-route,
//! windows-rank law); the batch's SECOND candidate
//! `count-based-simd-sorted-slot-search` DROPPED AT MEASUREMENT —
//! `ParetoDominated` 0/4/1 on the `PerfBenchValidator` + a standalone large-n
//! probe (the O(n) count never recovers; the negative lives in the
//! `entries_52` header), upstream XiangpengHao/bbtree @ `8adae97a`) — prior:
//! Batch 288 (+1: complexity, `entries_51`
//! — the grammar rejection recursion-to-explicit-stack law, upstream llama.cpp
//! #31153 @ `48c9f08b7`, Research 327) — prior: Batch 284 (+2: measurement + caching, `entries_50` — the burn/cubek autotune-family laws: `timed-zero-refuses-round-stop` + `tune-cache-keyed-by-spelled-variant-names`, Research 321; +1 NEW category: measurement, the instrument-validity family) — prior: Batch 283 (+1: caching, `entries_49`
//! — the certifying-identity-token revalidation skip, `PrismML` #393) — prior:
//! Batch 282 (+3: arithmetic, dispatch,
//! iterators, `entries_48` — the `PerfR` C-to-Rust translation classes, arXiv
//! 2610.23044v1) — prior: Batch 274 (+1: allocation, `entries_47`) — prior: Batch 271 (2026-10-21,
//! Plan 275 / Research 307: +12 — the splash-wave SECOND-TIER serving pool
//! (incoai/splash, the B268/B270 staged second tier, Apache-2.0, author Jian
//! Chen). Allocation +3: `resume-admission-sized-to-per-command-window-not-suspended-peak`
//! (@ fa63d97f8956113b067dfdce46caf1fcd22213e1 + rider face
//! 593f2a6d7427382a88a474a6a27c6dcc61b7415e, the anchor-bound output face),
//! `phase-scoped-staging-released-at-successor-reservation-when-plan-budgets-max`
//! (@ 88d0e57ee3a37305a895197538554ba9f3ea83d8),
//! `derived-reservation-taken-once-after-admission-settles`
//! (@ 81ae003e18d9362f6adf4131cf88a17a17803f65) — `entries_43`.
//! Dispatch +3: `size-class-keyed-reader-cap-bimodal-reads`
//! (@ 4d1799564d2ff9092a7963b914b9fc86f20fd640),
//! `fixed-read-command-runs-alone-sized-by-its-own-reads`
//! (@ 68b9e7069df0a64c2ad95cc322f4d293f4d0ce53),
//! `reject-plan-runs-before-setup-with-outliving-side-effects`
//! (@ dab88419a70128eaa9d61c1c05d866cef7fec1f6) — `entries_44`.
//! Dispatch +2: `admission-tests-the-bound-the-admitted-mode-serves`
//! (@ 5ea4043fbecf7612a9b0066e7f9c10481eae4e1b + enabler
//! 82b93c5f32a39865ac945a507134f3ab01b59d92),
//! `in-use-state-retry-under-serving-mark-keyed-on-refusal-cause`
//! (@ b7844dcf745909fb5caf79f82dd985d64041393b); concurrency +1:
//! `one-event-per-writer-consumer-waits-on-the-side-it-needs`
//! (@ e2c0af27e8cb31ffd5990fac001b111e0894da93) — `entries_45`.
//! Concurrency +1: `finished-command-copy-leaves-at-once-slot-returns-at-last-reader`
//! (@ 91d4ddb8d8125eb89aac12762b2c8bcf727fd7e3); complexity +2:
//! `part-cost-measured-by-planning-ablation-not-component-sum`
//! (@ ed1092f8631e579e09906805b9dd4544ebf327a4),
//! `plan-input-precomputed-per-output-variant-bounded-convergence`
//! (@ 2f5fe8dd3d4cbd986812beee4652f4d4e0f2d765) — `entries_46`.
//! ALL ANNOTATED `NotBenchable` + 4 count-neutral rider faces (the
//! anchor-bound output face on slice A's own rule 1; the spill-tier
//! victim-exclusion face @ 253506f393b17c9253e0a105f479908c7a3d15d7 and the
//! durable-label-at-reclassification face @ 4fc7465e85648402bbe8d2db5d8da8e2bb443841
//! on `restore-point-chain-copies-keyed-on-disk-state-count`, `entries_39`;
//! the task-scoped-read-buffer face @ 4d1799564d2ff9092a7963b914b9fc86f20fd640
//! on `reuse-allocation-clear`, `entries_02`). Drops at the T0 re-read:
//! 6fcd9b1 (error-copy), 25502fd (DRY, 'Effect: none'), 7e51fcb (error-copy).
//! Prior state: 254 entries as of Batch 270
//! (2026-10-21, Plan 274 /
//! Research 306: +12 — the splash-wave serving pool (incoai/splash, the
//! Batch-268 staged leads, Apache-2.0): `restore-point-chain-copies-keyed-on-disk-state-count`
//! (caching) + `persistent-cache-teardown-needs-second-witness` (caching) +
//! `hot-slot-recycler-flat-tables-zero-alloc-claims` (caching) +
//! `cache-share-capped-at-measured-distinct-working-set` (caching) +
//! `cache-reuse-assertion-gated-on-eviction-counter` (caching) +
//! `publication-takes-over-condemned-cache-entry` (caching) +
//! `reclaim-progress-keyed-on-refused-resource-returned` (allocation) +
//! `refusal-wait-keyed-on-shortfall-returning-transfer` (concurrency) +
//! `watchdog-deadline-moves-on-command-progress` (dispatch) +
//! `page-miss-blocks-one-ranged-read-scatter` (io) +
//! `tier-write-defers-to-urgent-reads-except-a-read-behind-it` (io) +
//! `constrained-unconstrained-lanes-one-decode-command` (dispatch, the f52d1ad
//! grade LAND verdict — -23.7%/-34.0% time/token source-reported), entries_39-42
//! (four authoring slices), ALL ANNOTATED `NotBenchable` + 4 count-neutral rider
//! faces (3 on this batch's own rules, 1 on `discretionary-cache-growth-keys-on-spare-not-available`).
//! Prior state: 242 entries as of Batch 266 (2026-10-21, Plan 270 /
//! Research 302: +1 complexity — `isop-cover-from-decision-diagram-not-path-
//! simplification`, the decision-diagram family seed from astral-sh/uv PR
//! #22651; BENCHABLE with a Pareto verdict; +2 count-neutral rider faces in
//! its `context_notes`). Prior state: 241 entries as of Batch 257 (2026-10-20,
//! Plan 261 /
//! Research 293: +6 — the `TensorFold` staged remainder (the B256 read agents'
//! B+ pool at the same pin `d1018b31`): `first-message-sized-to-smallest-job`
//! (dispatch, @ `10ff8779`) + `draft-cost-timed-at-served-widths-not-extrapolated`
//! (dispatch, @ `abb860c5`) + `checkpoint-reads-skip-page-cache-with-refusal-fallback`
//! (io, @ `221c2b5d`+`6bb9daac`) + `reserve-stream-caches-at-admission-warm-request-paths-at-load`
//! (allocation, @ `84c58d3f` sibling) + `cache-budget-first-sessions-take-remainder-never-reshrink`
//! (allocation, @ `406b2090`) + `kept-prompt-states-pinned-host-cache-budget`
//! (caching, @ `a739444c`), ALL ANNOTATED `NotBenchable`; Batch 256 (2026-10-19, Plan 260 /
//! Research 291: +1 caching — `slru-probation-protected-promote-on-second-touch`,
//! the llama.cpp `MoE` host-cache law @ `33d6135c` #30637 — a recency cache of
//! EXPENSIVE entries runs a 50/50 SLRU, not one plain LRU list: first touch
//! = probation, second touch promotes (protected capped `n_slots/2`, overflow
//! demotes the protected-LRU head), victims scan probation-head-first then
//! protected while the request-scoped seen-set pins the current request's
//! own selections, hits promote BEFORE victims are chosen, and the no-victim
//! case is a loud impossibility terminal; `NotBenchable` — the upload-traffic
//! win metric, the B136 decision-content class; the module header was stale
//! at 230 through Batch 250 slice 1's +1 — repaired with this batch's
//! landing).
//!
//! 231 entries across 13 categories as of Batch 250 slice 1 (2026-10-18,
//! Plan 254 / Research 285: +1 simd —
//! `avx2-tiled-accumulator-butterfly-epilogue`, the llama.cpp envelope
//! pool's hadd butterfly @ `3dbcb7f` #30296 — the header paragraph above
//! was not updated at that landing; this paragraph records it). Prior:
//! 230 entries across 13 categories as of Batch 248 (2026-10-16, Plan 252 /
//! Research 280: the staged simdutf + unicode-normalization conversion —
//! minimal-perfect-hash-static-lookup, flat-pool-start-len-static-rows,
//! dense-subdomain-arithmetic-before-table-lookup,
//! fanout-transform-emit-callback-not-vec, inline-buffer-index-shift-over-drain,
//! shift-mask-adjacency-check — deferred-error-mask-block-check DROPPED at
//! measurement, the negative lives in Research 280; was 224 at Batch 246, Plan 250 /
//! Research 278 — the prism fork delta: +1 caching, the serving state-cache
//! family's FORK face — `seq-fork-carries-pending-rollback-record` (a
//! sequence-fork carries PENDING mutation records, not just committed
//! state: the `rs_idx` deferred rollback travels with the copy; for
//! snapshot-plane state the copy takes the plane the rollback points to,
//! never the live plane; planes counted `n_stream*(1+n_rs_seq`) in `size_t`
//! arithmetic) — `NotBenchable` annotated; caching 15 → 16; categories
//! UNCHANGED 13.
//! 223 entries across 13 categories as of Batch 245 (2026-10-16, Plan 248 /
//! Research 277 — the Batch-244 DEFER LIST conversion, +6 (3 benchable + 3
//! NotBenchable-annotated) + 1 count-neutral rider + 1 defer confirmed, from
//! the same three Batch-244 sources re-read at their release pins:
//! `self-loop-state-memchr-skip` (dispatch, the dfa/accel.rs self-loop law:
//! one memchr over the ≤3 exit bytes replaces N state steps — with the
//! common-byte admission refusal) + `pair-probe-confirm-fused` (simd, the
//! generic packedpair fused AND: two offsets, one candidate mask, ~p₁·p₂
//! false positives) + `finder-hoist-reuse` (caching, ONE law in two repos —
//! memmem + bstr Finder construction amortized across haystacks) +
//! `canonical-set-guarded-rebuild` (complexity, the invariant-guarded
//! normalize — premise-corrected: the guard is canonicalize/folded/union,
//! push is sort-free at this pin) + `nonmax-sentinel-option-niche` (layout,
//! the Option niche via the `isize::MAX-provable` sentinel — payoff RELABELED
//! memory-first) + `match-branch-cold-confirm` (dispatch, the #[cold]
//! placement law — benchmark-cited positive on the rabinkarp confirm,
//! measured wash on tiny helpers; suggest-only). The rider: the CHAR face on
//! `byte-find-position-to-memchr` (bstr `find_char` encode-once). DEFER
//! confirmed: `align-probe-overlap-tail` (align half source-silent, tail
//! thin). The staged 8 leads convert 6 + rider + defer — zero dropped.
//!
//! 217 entries across 13 categories as of Batch 244 (2026-10-16, Plan 247 /
//! Research 276 — the text/unicode family's follow-up, the byte-scan/search
//! mechanism batch, +10 — ALL benchable + annotated — from three sources read
//! at release pins: `rust-lang/regex` @ `regex-automata-0.4.18`,
//! `BurntSushi/memchr` @ `2.8.3`, `BurntSushi/bstr` @ `1.13.1`:
//! `byte-class-equivalence-table-shrink` (layout, the regex byte-class law:
//! collapse the key space to equivalence classes before the table —
//! states×256 → states×classes, cache-resident) +
//! `prefilter-choice-shape-ladder` (dispatch, cheapest-first accelerator
//! selection with the empty-set/empty-string refusals) +
//! `alternation-prefix-collapse-at-build` (complexity, the common-prefix
//! collapse + preference-trie shadowing + truncation ladder) +
//! `byte-rank-select-rarest-probe` (caching, the merged regex-syntax `rank()`
//! + memchr Pair law — rarest-byte probe + the rank-250 poison refusal) +
//!   `prefilter-skip-density-demotion` (dispatch, the `PrefilterState`
//!   feedback-control law — track skip density, latch inert, sentinel 0) +
//!   `iter-count-bulk-specialization` (iterators, the `Iterator::count` override
//!   with a bulk mask-popcount sweep) + `mod64-byte-membership-skip` (caching,
//!   the Two-Way `ApproximateByteSet` — the Boyer–Moore shift for bytes absent
//!   from the needle) + `byteset-cardinality-dispatch` (dispatch, route
//!   find-first-of by set cardinality — memchrN for 1–3, 256-entry stack table
//!   for ≥4) + `utf8-fast-lane-cold-diagnostics` (dispatch, validate bare and
//!   recompute diagnostics windowed on failure) +
//!   `bufread-scan-in-buffer-scratch-fragment` (io, the zero-copy `fill_buf`
//!   drain + one reused scratch).
//!
//! 207 entries across 13 categories as of Batch 243 (2026-10-16, Plan 245 /
//! Research 275 — the text/unicode processing family seed, the corpus's
//! founding member of that family, from Hugging Face's Tokenmaxxing-BERT
//! work: +2 — `ascii-run-swar-block-skip` (simd, tokenizers @ `5b5a3fcc`
//! `ascii_fastpath.rs`: the high-bit-gather block mask skips class-pure ASCII
//! runs wholesale — never decoded, looked up, or copied; the source's 16-byte
//! `wide::u8x16::to_bitmask()` form, the u64 SWAR mask as the portable
//! spelling) + `range-table-page-memoization` (caching,
//! `character_cache.rs`: lazy per-page predicate memoization, 256 answers
//! per page built on first touch — one read replaces the ~11-branch binary
//! search over the huge dense code-point domain). Both BENCHABLE + annotated.
//! A third lead (`transform-borrow-until-first-change`, allocation,
//! `rewriter.rs`) was authored, benched, and DROPPED HONESTLY:
//! `ParetoDominated` 0/3/0 twice — the find phase cannot be shared with the
//! caller in a synthetic; the negative rides Research 275. Prior —
//! 205 entries as of Batch 241 (2026-10-15, Plan 243 / Research 273 — the serving slot-state cache pair from the llama.cpp envelope delta `47f5a64ae → 620d2225`: +2 caching —
//! `reuse-admission-threshold-must-equal-window-semantics` (#30580) +
//! `empty-slot-first-victim-order-save-before-overwrite` (#30567), plus the
//! count-neutral draft-state rider on `entries_27` P65 (#30574). Prior —
//! Batch 239 (2026-10-15, Plan 241 /
//! Research 270 — the serving state-cache pair from the llama.cpp envelope
//! delta `843e7dcc → e467e5b5e5ba`: +2 caching —
//! `partial-state-serialization-honors-rollback-contract-at-leaf-cache`
//! (#30514: partial = what rollback cannot undo; the flag honored AT THE
//! LEAF cache keyed on `swa_type`, wrappers delegate; the size-monotonicity
//! invariant) + `cache-write-admission-requires-restoring-consumer`
//! (#30498: `is_stateless` kinds never write, both save paths share the one
//! predicate home; decision models default-off with `cache_ram_explicit`
//! opt-in) — both `NotBenchable` annotated; annotated 147 → 149, caching 8
//! → 10) on the Batch 237 base (2026-10-15, Plan 239 —
//! the P58–P61 conversion set: +3 `rust_perf` rules — `f16-to-f32-accumulator-lut-breaks-conversion-serialization`
//! (simd, ggml-org/llama.cpp @ `685f7089` #30342+#30343: the F16C
//! FP16→FP32 intrinsic lowers to a vpinsrw into the live accumulator,
//! serializing the loop — the table breaks the false dependency;
//! per-direction) + `work-buffer-sized-by-execution-width-not-plan-tasks`
//! (concurrency, llama.cpp @ `5cdadfc3` #30454: the buffer is sized by the
//! width that executes, never the planner's task count) +
//! `discretionary-cache-growth-keys-on-spare-not-available` (io,
//! incoai/splash @ `ffc2d4be` #418: growth no request needs keys on SPARE
//! memory, never the OS available estimate; warning-pressure latch + paced
//! reclaim) — ALL THREE `NotBenchable` annotated; +1 count-neutral
//! `kernel_opt` rider (the autotune-cache HARDWARE-PATH axis, cubecl #1811
//! @ `5a4f5882`); annotated 144 → 147, `without_spec` 99 → 102; Batch 222
//! before it, 2026-10-10, Plan 216 /
//! Research 252 — the soroban wasm size pair from `stellar/rs-soroban-sdk` @
//! `0d72c4c9`: +2 `wasm-unwrap-panic-machinery-elimination` +
//! `linker-dce-custom-section-volatile-marker`, BOTH category `wasm`, BOTH
//! `NotBenchable` annotated (the size-axis domain reason; annotated 142 → 144,
//! `without_spec` 97 → 99, the wasm family 1 → 3); Batch 221 before it,
//! 2026-10-10, Plan 221 —
//! the rider-tier conversion set: +1 the bevy #25864 runtime-bool
//! conditional-guard rule, dispatch, `NotBenchable` annotated; the live
//! per-category counts are pinned by tests, never this
//! prose: the figures
//! here went stale repeatedly — 163 through the B182-era, before that
//! 145/92/53 through B172–B174, 183 through B211 — repaired here at B218
//! with the honest
//! label; the gated count tests + `cargo refine --corpus-stats` are the
//! live ledgers). The `NotBenchable` ledger was repaired by Batch 135 (the
//! pre-Batch-118 ledger said 25 while the gated test said 14; the Batch
//! 118–129 ds4 io annotations were never counted) and again by the 09-24
//! housekeeping pass
//! (8 external-runtime/size-class + 2
//! metric-mismatch (Issue
//! 014) + the x86-only prefetch annotation (Batch 85) + the bitflag size
//!      axis (Batch 86) + the workload-level read-coalescing annotation (Batch
//! 94) + the measurement-honesty axis itself (Batch 109 — a timing gate
//!     whose work folds away has no nonzero baseline to ratio) + the PLE-gather
//!     workload annotation (Batch 118) + the two build-integrity annotations
//!     and the A/B-design annotation (Batch 121 — resume provenance, write-
//!     verify-publish, one-variable A/B; correctness-of-provenance and
//!     experimental-design content, not speed) + the three vision-delta
//!     annotations (Batch 122 — template-governed emission, provisioning
//!     completeness, per-turn diagnostics; correctness-of-emission and
//!     test-honesty content, not speed) + the two upstream-surge annotations
//!     (Batch 123 — full-vector comparison integrity, tri-state fit
//!     classification; measurement-integrity and robustness content, not speed)
//!     plus the evaluation-design annotation (Batch 124 — matched controls,
//!     proxy-vs-endpoint, null-arm identity; experimental-design content, not
//!     speed) plus the oracle-contract annotation (Batch 125 — baseline-
//!     reproduction triage, compute-shape-matched oracles, consumed-outputs
//!     comparison; oracle-contract content, not speed) plus the three agent-
//!     context annotations (Batch 141 — measured reserve for a destructive
//!     rewrite, semantic truncation boundaries, pattern-bounded stream holdback;
//!     budget/boundary/stream discipline, not speed) plus the two Batch-143
//!     1x) plus the two Batch-143
//!     annotations (the log-only hot-path computation behind a level gate — its
//!     note carries the #143684 wall-time-vs-icount measurement-honesty rider;
//!     and the macro-emission build-time rule — upstream-measured compile-time
//!     content, no runtime A/B axis). Batch 173 (Plan 167) added the two serde
//!     zero-copy rules — allocation 26 → 28 (the corpus's first dedicated
//!     serde family; both benchable + annotated). Batch 172 (Plan 166) added
//!     the mezura quartet — simd 8 → 9, io 24 → 26, concurrency 20 → 21 (one
//!     measured GOAT lead + three `NotBenchable` decision-content rules). Batch
//!     174 (Plan 168) added the host-lock + flush-hoist pair — concurrency
//!     21 → 23 (one measured `ParetoDominant` + one `NotBenchable` deadlock
//!     class). Batch 179 (Plan 172 / Research 210) added the rsconstmap trio —
//!     layout 12 → 13, iterators 10 → 11, complexity 22 → 23 (three measured
//!     `ParetoDominant` benchable rules; the first `[dep: constmap]` external
//!     map-family entry). Batch 219 (Plan 215 slot 2 / Research 251, the
//!     `TensorFold` native-engine serving/timing pack) added nine `NotBenchable`
//!     annotated rules — complexity +2 (the cost-table hygiene pair), caching
//!     +3 (the prompt-cache pair + the fingerprint persistence), thermal +1
//!     (the corpus's THIRTEENTH category), dispatch +1, io +1, allocation +1.
//!     Batch 221 (Plan 221) added one `NotBenchable` annotated rule — dispatch
//!     +1 (the bevy #25864 runtime-bool conditional-guard rule: a runtime
//!     bool branching around a large inlinable body forces
//!     duplicate-or-deinline; the landed form hoists the branch into a
//!     conditional `Option<Guard>` tuple so the body compiles once).
//!
//! Batch 48 (idle-queue item 11, 2026-08-16) added the 8 online-search
//! candidates (perf-book std-types + iterators chapters): collect-append-to-
//! extend, hashset-insert-bool-return, unwrap-or-else-lazy, filter-count-to-
//! any, vec-macro-zero-fill, iter-copied-deref-map, filter-map-fuse,
//! vec-insert-front-deque. Batch 53 (idle-queue item 11, 2026-08-17) added
//! the 4 online-search candidates: powi-vs-powf, distance-sq-no-sqrt,
//! hashmap-with-capacity, soa-hot-field. Batch 59 (2026-08-18, the first
//! EXTERNAL-source distill via the `distill` skill) added
//! division-by-invariant-multiplication (libdivide via the
//! raygon-renderer/thermite Rust port). Batch 61 (idle-queue item 11,
//! 2026-08-18) added the 2 oxc-article rules: keyword-map-to-match +
//! usize-index-to-u32. Batch 62 (2026-08-19, Research 070 microGPT-C
//! distill) added the 2 CPU-SIMD rules: transposed-weight-register-gemv +
//! newton-refinement-vs-hardware-transcendental. Batch 64 (2026-08-19,
//! Research 072 — the ARM Neon / bounds-check online-search pool) added
//! loop-order-innermost-contiguous + debug-assert-length-hoist. Batch 66
//! (2026-08-19, Research 073 — the polars kernel-idiom distill) added
//! sparse-set-bit-iteration-tz-blsr + sortedness-check-branchless-accumulate.
//! Batch 67 (2026-08-19, Research 074 — the regex-automata distill) added
//! byte-find-position-to-memchr. Batch 68 (2026-08-19, Research 075 — the
//! tantivy utility-layer distill) added varint-compose-word-single-store
//! (+ the tz-blsr xor-clear anti-shape rider note). Batch 69 (2026-08-19,
//! Research 075 §Secondary — the T6 refutation) added
//! unaligned-word-load-from-bytes. Batch 65 (2026-08-22, Research 081 —
//! the hotpath.rs profiler distill) added the 2 contention rules:
//! align-64-false-sharing + channel-send-batching-unpark (both measured
//! `ParetoDominant`, both context-annotated — the 128B-line arch caveat +
//! the park-storm regime condition).
//!
//! Batch 83 (2026-08-28, Research 097 — the arXiv:2608.26368 SIMD
//! bit-slicing predicate distill) added bitplane-transpose-repeated-
//! predicate-scan: encode-once bit-plane transposition + zero-decompression
//! bitwise predicate evaluation for REPEATEDLY filtered integer columns
//! (the O'Neil-Quass bit-sliced index as a primary representation).
//!
//! Batch 85 (2026-08-28, Research 098 — the arXiv:2608.23841 cflow
//! "Pipeline-Native Transformers" distill) added weight-tile-consumption-
//! order (MEASURED `ParetoDominant` 1.32-1.56x growing with working set —
//! consumption-ordered tiles keep the x slice L1-resident per tile) +
//! software-prefetch-storage-bound (the honest annotation: SW prefetch on
//! a bandwidth-bound stream is useless-to-harmful — the source's prefetch
//! thread burned 48% of decode time; disabling it alone = 1.92x; the
//! aarch64 PRFM analog unmeasured — x86 source-cited only, proposer-less
//! by design).
//!
//! Batch 86 (2026-08-29, Research 100 — the Cloudflare 1.1.1.1 DNS-cache
//! memory distill) added serialize-once-byte-buffer-store (frozen records
//! as ONE length-prefixed byte buffer + offset table — 2 allocations
//! total regardless of record count; the insert-path malloc storm is the
//! cost), context-equal-field-elide (a stored field that equals the
//! enclosing context — the lookup key — becomes Option<Box<T>> None,
//! resolved at read; ~80% of the source's records elided), and
//! bitflag-pack-when-footprint-dominates (the stored-at-scale footprint
//! regime; cross-cites the `kernel_opt` hot-read inverse; measure `size_of`
//! before/after). Two doc riders: box-str-immutable generalizes to
//! Box<[T]> for any frozen T; box-large-enum-variant gains the honest
//! costs (size-class rounding, locality scatter, the rare variant pays
//! more).
//!
//! Batch 91 (2026-08-31, Research 106 — the Rustdoc filter-first distill,
//! noahlev.org "Making Rustdoc 33% faster" + rust-lang PRs 159623/159779/
//! 159854) added filter-before-expensive-transform (Rustdoc built the
//!         display representation for EVERY trait impl in the dependency graph,
//!         THEN filtered; the fix re-expresses the relevance predicate on the raw
//!         input and filters first — win ∝ discard-fraction × transform cost,
//!         IMPOSSIBLE when the predicate is not re-expressible on the input) and
//!         decision-projection-before-construction (the decision consumes only a
//!         cheap projection that is INVARIANT under an expensive canonicalization
//!         — the head of a type — so compute the projection on the raw input and
//!         pay the canonicalization only behind the decision).
//!
//! Batch 94 (2026-09-01, Research 110 — the oMLX PR #3260 SSD expert
//! streaming distill) added coalesce-nearby-reads-bounded-gap-cacheability-
//! gated to io: merge random-ish reads within a bounded byte gap into ONE
//! pread; the CACHEABILITY COUPLING is the decision content — the over-read
//! bytes are only free when the OS page cache absorbs them, so a direct/
//! no-cache descriptor coalesces with gap 0 (exactly-adjacent rows only).
//! The source's serving numbers are workload-level, no isolated corpus
//! bench — regime-annotated, suggestion-only.
//!
//! Batch 118 (2026-09-06, Research 142 / Plan 097 — the antirez/ds4 PR #991
//! PLE-gather distill) added parallel-disjoint-pread-hides-cold-page-latency
//! to io: N known DISJOINT byte ranges -> thread-parallel positional reads on
//! the shared fd + indexed writes (byte-identical by construction); the
//! LATENCY-OVERLAP axis is the decision content — cold page-ins paid serially
//! at queue depth one (~20x measured) vs warm-neutral; near-adjacent ranges
//! are the OPPOSITE regime (coalesce-nearby-reads). Suggestion-only.
//!
//! Batch 109 (2026-09-05, post-mining intake P10 — the katgpt-rs Issue 723
//! Class A2 measurement-honesty distill) added dead-result-timing-loop-
//! folded-under-lto to complexity: a timed loop whose work result is
//! discarded (`let _ = f()`) measures ZERO under rustc 1.98.1 + fat LTO —
//! the fold eliminates inlined-callee work EVEN THROUGH `black_box` inside
//! the callee (only a CALLER-side sink of per-iteration data survives);
//! the fix pattern is xorshift data-dependent work + `black_box(sink)` at
//! the consumer + a loud nonzero-baseline assert before any ratio. The
//! first rule on the measurement-honesty axis: the "buggy" arm is a
//! harness that passes its own timing gate while timing nothing.
//!
//! ## Bench specs (behind `rust_perf_bench`)
//!
//! Benchable rules carry an optional [`BenchSpec`] — a self-contained harness
//! embedding both buggy + fixed code inline + defining `make_input` /
//! `run_buggy` / `run_fixed` entry points. The bench validator generates a
//! temp bin crate from the spec + runs a timing loop at multiple sizes.
//!
//! Currently benchable (90/121):
//! - `o1-lookup` (Vec scan vs `HashMap` lookup — clear O(n) vs O(1))
//! - `bsearch-sorted-slice` (linear scan vs binary search — O(n) vs O(log n) on sorted data)
//! - `dedupe-hashset-vs-vec` (`Vec::contains` loop vs `HashSet` collect — O(n²) vs O(n))
//! - `entry-vs-contains-insert` (`contains_key+insert` vs entry API — merge `HashMap` lookups)
//! - `loop-fusion` (two iterator passes vs one fused loop — cache locality, merge loops)
//! - `prefix-sum-2d` (O(n) region scan vs O(1) prefix sum — boundary-vs-volume, grows with N)
//! - `swap-remove` (`Vec::remove` O(n) shift vs `Vec::swap_remove` O(1) swap — grows with N)
//! - `sort-unstable` (stable sort allocates vs `sort_unstable` in-place — no allocation + faster algo)
//! - `sort-by-key` (`sort_by` repeated closure vs `sort_by_key` single extraction — N vs N·log N)
//! - `vec-remove-front-to-vecdeque` (`Vec::remove(0`) O(n²) drain vs `VecDeque::pop_front` O(n) — grows with N²)
//! - `btreemap-sorted-iteration` (`HashMap` + sort per query vs `BTreeMap` natural order — O(n log n) vs O(n) per query)
//! - `static-str` (String alloc vs &'static str — allocation cost)
//! - `serialize-once-byte-buffer-store` (per-record store malloc storm → ONE length-prefixed buffer + offsets — insert path, Batch 86)
//! - `context-equal-field-elide` (unconditional owner field → Option elided at ~80% context-equal — alloc axis, Batch 86)
//! - `simd-chunk` (scalar vs chunked loop — vectorization at large N)
//! - `branchless-conditional` (if vs bool-as-usize multiply — vectorization + branch prediction)
//! - `bool-to-bitpack` (Vec<bool> count vs packed Vec<u64> POPCNT — data density)
//! - `chunks-exact` (chunks variable-last vs `chunks_exact` uniform — enables vectorization)
//! - `copy-from-slice` (index-loop copy vs `copy_from_slice` — SIMD + no bounds check)
//! - `mutex-to-atomic` (Mutex<u32> vs `AtomicU32` — lock/unlock overhead)
//! - `f32-counter-to-int` (f32 accumulator vs u64 — serial dep chain)
//! - `collect-avoidance` (.`collect()` intermediate vs chained iterator — allocation + double pass)
//! - `dyn-to-enum` (Box<dyn Trait> vs enum — vtable indirection + no inlining)
//! - `ok-or-else` (`ok_or(expensive()`) eager vs `ok_or_else(closure`) lazy — avoids unused error alloc)
//! - `precompute-table` (recompute vs const lookup table — **context-dependent**,
//!   see `context_notes`; not a release-mode win per Bench 013)
//! - `papaya-lockfree` (`RwLock` vs papaya lock-free — **context-dependent**,
//!   see `context_notes`; only a win under multi-threaded contention)
//! - `vec-with-capacity` (`Vec::new()` growth vs `Vec::with_capacity` — realloc cost)
//! - `format-concat-loop` (format! concat in loop vs `push_str` — O(n²) vs O(n))
//! - `fxhashmap` (std `HashMap` `SipHash` vs `FxHashMap` `FxHash` — hasher speed, 2-4×)
//! - `retain-inplace` (`filter().collect()` new Vec vs `retain()` in-place — ~1.5-2×)
//! - `mem-take` (clone+clear vs `mem::take` move — ~3×, avoids deep clone)
//! - `avoid-clone` (fn(data: Vec<T>) clone → fn(data: &[T]) borrow — ~1.3-3×)
//! - `box-str-immutable` (Vec<String> 3-word → Vec<Box<str>> 2-word — ~1.1-1.3×; B86 rider: generalizes to Box<[T]> for ANY frozen T)
//! - `reuse-allocation-clear` (alloc-per-call → clear-reuse scratch buffer — **Mixed**, see `context_notes`)
//! - `box-avoid` (Vec<Box<i64>> heap per elem → Vec<i64> stack values — ~10-35×)
//! - `smallvec-stack` (Vec<T> always heap → stack array for ≤8 typical — ~7-12×)
//! - `buffered-io` (`File::read` byte-by-byte → `BufReader` 8KB batched — ~97×)
//! - `bufwriter-batched` (`File::write` per-record → `BufWriter` batched — syscall amortization)
//! - `log-write-direct` (`push_str(&format`!(...)) temp alloc → write! direct — ~1.7×)
//! - `cow-conditional-alloc` (String always-alloc → Cow<str> borrow when no transform — N vs 0.1N allocations)
//! - `regex-hoist-lazylock` (`Regex::new` per call → static `LazyLock` — compile once, Issue 014 T2.1)
//! - `arc-str-payload` (per-recipient payload clone → Arc<str> handle clone — N×bytes → 1 alloc, Issue 014 T2.2)
//! - `collect-append-to-extend` (collect+append staging → extend — no intermediate Vec, Batch 48)
//! - `hashset-insert-bool-return` (contains+insert double lookup → insert's bool return — Batch 48)
//! - `unwrap-or-else-lazy` (`unwrap_or(eager`) → `unwrap_or_else(closure`) — Batch 48)
//! - `filter-count-to-any` (`filter(p).count()` > 0 full scan → any(p) short-circuit — Batch 48)
//! - `vec-macro-zero-fill` (`Vec::new+resize` memset → vec![0; n] calloc — Batch 48)
//! - `iter-copied-deref-map` (.map(|x| *x) → .`copied()` `TrustedLen` — Batch 48)
//! - `filter-map-fuse` (filter(p).map(f) → `filter_map` fused — Batch 48)
//! - `vec-insert-front-deque` (`Vec::insert(0`,_) O(n²) → `VecDeque::push_front` — Batch 48)
//! - `powi-vs-powf` (x.powf(2.0) libm call → x.powi(2) multiply — Batch 53)
//! - `distance-sq-no-sqrt` (sqrt-then-compare → squared compare — Batch 53)
//! - `hashmap-with-capacity` (`HashMap::new()` rehash ladder → `with_capacity(n`) — Batch 53)
//! - `soa-hot-field` (`AoS` strided single-field scan → `SoA` contiguous column — Batch 53)
//! - `division-by-invariant-multiplication` (hardware integer div by loop-invariant RUNTIME d → libdivide-style magic mulhi+shift — Batch 59, thermite distill)
//! - `keyword-map-to-match` (static-keyword `HashMap` dispatch → plain match on literals — LLVM length-switch, Batch 61)
//! - `usize-index-to-u32` (16B usize spans → 8B u32 spans — cache density under bandwidth-bound scans, Batch 61)
//! - `transposed-weight-register-gemv` (row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA into output accumulators — 1.18-6.08x `ParetoDominant`, Batch 62, microGPT-C distill; the win GROWS at large nin — the scalar accumulator's serial FP chain is the real cost, not the horizontal reduce)
//! - `newton-refinement-vs-hardware-transcendental` (Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction `fdiv`/`fsqrt` on CPU — Mixed 1.01-1.09x M3 / `ParetoDominant` 1.64-1.66x x86 (re-gated), hardware never loses; Batch 62, annotated)
//! - `loop-order-innermost-contiguous` (i-j-k nested accumulation with a column-striding inner access → i-k-j reorder so the innermost varies the LAST index — 7.1-13.8x `ParetoDominant`, grows with n; Batch 64)
//! - `debug-assert-length-hoist` (`debug_assert!` mentioning `.len()` is erased in release, dropping the constraint bounds-check elision needed → `assert!` — Mixed regime-split 1.13x@1K / 0.67-0.71x@100K stable on aarch64 (x86 re-gate: 1.00x/0.92x, holds cross-arch); Batch 64, annotated)
//! - `sparse-set-bit-iteration-tz-blsr` (64-iteration set-bit scan → `trailing_zeros+BLSR`, popcount iterations — sparse-mask regime, Batch 66)
//! - `sortedness-check-branchless-accumulate` (early-exit sortedness validation → branchless `sorted &=` full scan, vectorizes — usually-sorted regime, Batch 66)
//! - `byte-find-position-to-memchr` (u8 slice iter().position byte scan → `memchr::memchr` SIMD block scan — position is NOT specialized in std, contains is; dep-gated, Batch 67)
//! - `varint-compose-word-single-store` (per-byte `write_all` varint loop → 5-threshold ladder + single `write_all` — MEASURED `ParetoDominated` on aarch64 AND x86 across writer regimes (the 4090-box re-gate: 0.40-0.50x there, HARDER): the honest-loser annotation stops the tantivy folk port; Batch 68)
//! - `unaligned-word-load-from-bytes` (shift/or byte-assembly word read → one unaligned `from_le_bytes` load — `ParetoDominant` cross-arch: u64 1.4-2.2x, u32 1.2-1.4x, both arches; the T6 defer refuted; Batch 69)
//! - `align-64-false-sharing` (adjacent 24 B per-thread atomic counter slots → `#[repr(align)]` to ARCH line granularity — `ParetoDominant` 4.7-9.4x at 2-12t, **1t = tie (honest)**; align(64) insufficient on 128B-line Apple Silicon (measured 1.16-2.44x of the win) — pad to the arch granularity or `CachePadded`; Batch 65)
//! - `channel-send-batching-unpark` (per-item MPSC sends to a parked `select!` receiver → per-64 batch flush via `mem::replace` — `ParetoDominant` 1.16-1.29x (pipeline-overlap honest margin; the source's 6.25x needs the park pair ON the critical path) + allocs 3232→1624; Batch 65)
//! - `swar-digit-chunk-parse` (per-digit decimal parse loop → SWAR 8-digit chunk fold, 3 multiplies per 8 digits + SWAR digit validation — the serial `val = val*10 + d` chain broken; regime: ≥8 digits win, <8 pay the guard (the source's own README); Batch 74, simd-json distill)
//! - `runtime-feature-detect-per-chunk` (CPU feature detection inside a per-chunk hot fn → resolve once via cached static / static cfg gate — the `std::detect` string resolution + detector read per call rivals small work units; Batch 74, simd-json distill)
//! - `runtime-int-exponent-powi` (runtime-integer-exponent `powf` libm call → `powi` binary exponentiation — log2(n) multiplies vs an opaque pow call; NOT bit-identical, rel-error-gated; Batch 77, lucebox rope distill)
//! - `bitplane-transpose-repeated-predicate-scan` (repeatedly filtered integer column → bit-plane transposition + bitwise predicate evaluation over the compressed planes, zero decompression — b/32 the bytes scanned + 64 values per word op; Batch 83, arXiv:2608.26368 distill)
//! - `weight-tile-consumption-order` (row-major single-token matvec x re-walked per row → consumption-ordered tiles, x slice L1-resident per tile — 1.32-1.56x growing with working set; Batch 85, cflow distill)
//! - `serialize-once-byte-buffer-store` (per-record heap fields → ONE length-prefixed byte buffer + offset table — 2 allocations total, copy-out rebuild; Batch 86, Cloudflare DNS-cache distill)
//! - `context-equal-field-elide` (per-record field equal to the enclosing context → Option<Box<T>> None, resolved at read — Batch 86, Cloudflare DNS-cache distill)
//! - `bitflag-pack-when-footprint-dominates` (bool cluster → one bitflag byte, stored-at-scale footprint regime — Batch 86, annotated)
//! - `filter-before-expensive-transform` (map(expensive).filter(relevant) → filter-first on the RAW input — win ∝ discard × transform cost; Batch 91, Rustdoc filter-first distill)
//! - `decision-projection-before-construction` (transform-to-decide → decide on the invariant raw projection, canonicalize only behind the decision — Batch 91, annotated)
//! - `mir-destprop-branch-site-merge` (known-fn-item conditional dispatch → two direct tail-jumps — MIR `DestinationPropagation` cmov-merges the merged form into one indirect site mixing branch-target history; Batch 99, wasmi distill)
//! - `dispatch-handler-pair-hoist` (runtime-table handler read per dispatch → hoist both entries once — register-resident select; cross-arch `ParetoDominant`, x86 re-gated 2026-09-04; Batch 101)
//! - `hot-byval-move-over-memcpy-threshold` (hot by-value move of a >128-byte type → shrink/ref — the threshold is an arch-shaped LLVM backend constant; Mixed on M3, x86 pays the memcpy call; Batch 112)
//! - `atomic-vec-zero-cost-convert` (`Atomic::new-per-element` collect from a borrowed vec → `into_iter().map` on the OWNED vec — same-layout collect reuses the buffer and the loop compiles out; alloc axis 2→1; Batch 135, Wild-linker article distill)
//! - `vec-reuse-across-lifetime-change` (fresh Vec<&'iter T> per iteration → owned buffer retargeted in place via a const size/align assert + `clear()` + empty collect — alloc axis 1→0; Batch 135, Wild-linker article distill)
//! - `partitioned-mut-slices-zero-sync` (shared Vec under one contended Mutex → disjoint `chunks_mut` partition, zero synchronization — sizes = thread counts; Batch 135, Wild-linker article distill)
//! - `dynamic-work-parallelism-ladder` (work-unknown-in-advance structure ladder: scoped spawning is the composition-safe default despite the boxing — `spawn_broadcast` starves the pool, `channel+par_bridge` does not compose; Batch 136, rayon-graph blog distill)
//! - `sender-per-work-item-terminates-channel` (dynamic queue shutdown is ownership-driven: seed sender clones per item, drop the original, the channel closes when the last item drops; Batch 136, rayon-graph blog distill)
//! - `nested-par-inside-channel-bridge-deadlocks` (a nested `par_iter` in par_bridge-fed work steals, blocks on the SAME channel, and the interrupted item's sender clone keeps it open — thread-tied tasks hang; Batch 136, rayon-graph blog distill)
//! - `compaction-reserve-before-prompt-build` (a destructive rewrite's reserve is MEASURED from the real prompt — tokenize it, add slack + output budget — and every headroom consumer reads that one figure; Batch 141, ds4 Part-B distill)
//! - `compaction-tail-boundary-prefers-complete-turn` (truncation boundaries are semantic: bounded walk-back to the latest complete user turn, atomic spans snap the boundary to their start; Batch 141, ds4 Part-B distill)
//! - `streaming-prefix-holdback-bounded-buffer` (a stream recognizer holds back at most the marker's own length, gated to the line boundary, flushing verbatim on mismatch — no line-length pending window; Batch 141, ds4 Part-B distill)
//! - `debug-level-gated-computation` (a hot-path binding whose only consumer is a `debug!`/`trace!` call pays its computation every iteration — the level check is a runtime atomic, not a compile-time fact; gate the computation with `log_enabled!` or move it into the macro's lazy args; Batch 143, nnethercote Dec-2025 distill)
//! - `macro-generated-code-optimize-at-generation-point` (proc-macro emitters trim/dedupe generated code where the context lives — downstream optimization machinery must reconstruct what the emitter already knew; measure with `-Zmacro-stats`; Batch 143, annotated)
//! - `prefault-admission-keyed-to-ram-headroom` (admission-policy annotation,
//!   source-reported — the DECISION CONTENT is the headroom predicate + the
//!   fail-closed direction + the mincore instrument refusal; page-cache state
//!   dominates any microbench; Batch 139, ds4-metal PR #5 PLE prefault)
//!
//! Not benchable (53/145 — count walked by Batch 154 (+3 tokio upstream-
//! surface rules); the line was repaired en passant by Batch 153 after it
//! below had sat stale at 36/127 since Batch 145 while B146/B147/B152/B153
//! grew both sides; the Batch-149 en-passant-pin-repair precedent) — honest
//! (count repaired by Batch 135; the Batch 136 rayon-queue + Batch 139 prefault-admission + Batch 141 ds4-Part-B agent-context + Batch 143 nnethercote build-time + Batch 145 mincore-routing rules are the post-repair additions; the ledger below is the early-corpus list — the Batch 118–129 source-reported io annotations are in the header ledger, all `bench_spec: None`):
//! - `layout-reorder` (struct size — deterministic `size_of`, not a runtime
//!   microbench; the cache-utilization claim needs a specific access pattern)
//! - `repr-u8-enum` (enum size — deterministic `size_of`, same class as
//!   `layout-reorder`)
//! - `box-large-enum-variant` (enum size — deterministic `size_of`, shrinks
//!   the common case by boxing the rare large variant)
//! - `shrink-to-fit-after-build` (memory win, not speed — releases excess
//!   Vec capacity after incremental construction)
//! - `async-yield-point` (fairness/latency-tail win, not throughput)
//! - `async-future-size` (future state-machine size — deterministic `size_of_val`)
//! - `bevy-reactive-loop` (needs the Bevy runtime — can't run in a temp crate)
//! - `wasm-batch-ffi` (needs a WASM runtime — can't run in a temp crate)
//! - `wasm-unwrap-panic-machinery-elimination` (wasm BINARY SIZE is the win
//!   metric — no native axis hosts it; Batch 222, the dial9 class)
//! - `linker-dce-custom-section-volatile-marker` (same size-axis domain reason
//!   — the win is the section surviving the linker into the artifact; Batch
//!   222)
//! - `tokio-guard-across-await` (**metric mismatch**, Issue 014 T2.3 — cost is
//!   OTHER tasks' wait latency under contention, not this task's wall-clock)
//! - `bounded-channel-backpressure` (**metric mismatch + semantics-changing**,
//!   Issue 014 T2.4 — win is bounded queue depth; the buggy path is often
//!   FASTER on wall-clock precisely because it buffers unboundedly)
//! - `software-prefetch-storage-bound` (x86-only source evidence — no stable
//!   aarch64 prefetch intrinsic to re-gate with; the rule IS the annotation
//!   stopping a plausible-looking prefetch port; Batch 85, cflow distill)
//! - `bitflag-pack-when-footprint-dominates` (deterministic `size_of` delta,
//!   the `layout-reorder`/`box-large-enum-variant` class — the gate pins
//!   12 → 8 for the fixture layout; Batch 86, annotated)
//! - `coalesce-nearby-reads-bounded-gap-cacheability-gated` (workload-level
//!   read-coalescing annotation, source-reported — the DECISION CONTENT is the
//!   cacheability coupling: the gap budget collapses to 0 on `direct/F_NOCACHE`
//!   descriptors; Batch 94, oMLX safetensors reader)
//! - `parallel-disjoint-pread-hides-cold-page-latency` (workload-level
//!   latency-overlap annotation, source-reported — the DECISION CONTENT is the
//!   disjoint-vs-near regime split vs coalesce-nearby-reads: DISJOINT far
//!   ranges overlap cold page-ins across threads, warm is neutral; Batch 118,
//!   ds4 PLE gather)
//! - `resume-revalidates-provenance-before-reuse` (build-integrity annotation
//!   — a resumable multi-part build revalidates source + calibration + TOOL
//!   hashes against the manifest and re-digests each cached part before
//!   reuse; presence-is-not-identity; Batch 121, ds4 IQ2 tool)
//! - `write-verify-readback-before-publish` (build-integrity annotation —
//!   hidden-path write, independent-reader hash verify per part, atomic-rename
//!   publish + sidecar manifest, refuse-existing, template guards before
//!   byte one; Batch 121, ds4 IQ2 tool)
//! - `ab-isolate-one-component-by-baseline-byte-copy` (measurement-honesty —
//!   A/B on one component byte-copies the baseline for everything else,
//!   paired diffs, confounded variants out of the headline, un-instrumented
//!   memory counters labelled as partial budgets; Batch 121, ds4 IQ2 quality)
//! - `model-aware-format-over-hand-assembled-tokens` (emission-correctness
//!   annotation — structured transcript turns go through ONE model-aware
//!   formatter; hand-pushed token sequences drift from the chat template
//!   silently; Batch 122, ds4 vision delta)
//! - `implemented-surface-requires-provisioning-target` (provisioning
//!   completeness — a feature needing an external artifact ships its fetch
//!   target in the same change; load-time errors NAME the target; Batch 122,
//!   ds4 vision delta)
//! - `diagnostics-not-just-exit-code` (measurement-honesty — a multi-turn
//!   session regression scans captured stderr for per-turn failure patterns
//!   alongside the exit code and saves full transcripts; Batch 122, ds4
//!   vision delta)
//! - `full-vector-bit-identity-over-argmax` (measurement-integrity — a
//!   numeric-regression gate compares the FULL float vector bit-exactly with
//!   pre-validated artifacts and three statuses, never argmax-only; Batch
//!   123, ds4 upstream surge)
//! - `renderer-error-is-not-capacity-overflow` (robustness classification —
//!   a tri-state fit validator keeps render errors off the destructive
//!   compaction path; Batch 123, ds4 upstream surge)
//! - `matched-control-disentangles-policy-from-model` (evaluation-design —
//!   matched controls separate the admission gate from the learned model;
//!   proxy and endpoint metrics reported separately; gate-closed arm
//!   bit-identical to the no-action baseline; Batch 124, arXiv:2609.04040)
//! - `dead-result-timing-loop-folded-under-lto` (measurement-honesty axis —
//!   a timing gate whose work folds away under LTO has no nonzero baseline to
//!   ratio; the rule's content IS that fact; Batch 109, Plan 080)
//! - `mmap-settled-size-gated-read` (page-cache state dominates any map-vs-read
//!   microbench — the io family's B118/B123/B139/B145 standing, cited three
//!   times; decision content = the TWO-GATE admission (size ≥ 256 KiB AND mtime
//!   age ≥ 5 s vs the concurrent-truncation SIGBUS) + the expected+1 sentinel
//!   read protocol; Batch 172, mezura)
//! - `steal-batch-not-per-item` (deque-header contention under real scheduling
//!   — not hostable without a thread farm whose noise floor swallows the
//!   signal; decision content = `steal_batch_and_pop`'s half-not-all policy;
//!   Batch 172, mezura)
//! - `nonblocking-open-for-head-probes` (the failure is a HANG against a live
//!   kernel-notification endpoint — `NotBenchable` by construction; decision
//!   content = `O_NONBLOCK` probe opens + WouldBlock-as-end-of-head; Batch 172,
//!   mezura)
//!
//! ## Context-dependent rules (`context_notes`)
//!
//! Forty-four rules carry a `context_notes` annotation (as of Batch 112;
//! the count is pinned by `forty_three_annotated_rules_carry_context_notes`
//! — the fn name is one batch behind its own assert, which reads 44). The
//! class ledger below is the accounting as it grew batch by batch. Ten record that
//! the Pareto validator found them **`ParetoDominated`** or **Mixed** in the
//! default configuration: the original 4 from Bench 013 (`precompute-table`,
//! `papaya-lockfree`, `dedupe-hashset-vs-vec`, `reuse-allocation-clear`) + 2
//! from Bench 016 / Issue 012 (`sort-by-key`, `copy-from-slice`) + 2 from
//! Bench 48 (`filter-count-to-any` — dominated at mid-position hits by the
//! VECTORIZED count loop, `iter-copied-deref-map` — Mixed, 0.80× at 1M on
//! sum folds) + 2 from Batch 53 / Bench 038 (`powi-vs-powf` all-Tie,
//! `soa-hot-field` latency-bound Mixed). Two more (Issue 014 T2.3/T2.4)
//! annotate the `NotBenchable` metric-mismatch class: `tokio-guard-across-await` +
//! `bounded-channel-backpressure`. The regime annotations from Batch 62/64
//! (`newton-refinement-vs-hardware-transcendental`, `debug-assert-length-hoist`)
//! and Batch 65's two (the arch-granularity caveat on `align-64-false-sharing`,
//! a clean winner whose FIX is arch-dependent; and the pipeline-overlap
//! regime on `channel-send-batching-unpark`) complete the set; Batch 91's
//! two (the two-regime note on `filter-before-expensive-transform`, the
//! invariance-precondition note on `decision-projection-before-construction`)
//! extend it; Batch 94's cacheability note and Batch 99's measured-verdict +
//! regime-boundary note on `mir-destprop-branch-site-merge` complete the
//! current set; Batch 172 added four annotations (the dual-mask GOAT lead +
//! the three `NotBenchable` mezura decision-content rules) and flipped
//! `byte-find-position-to-memchr` from unannotated to annotated (the anchor-
//! cover selection rider) while extending `reuse-allocation-clear`'s note
//! with the retention-clamp rider. The patterns
//! are genuine wins in their respective regimes and are kept in the corpus as
//! documented patterns, but consumers should treat annotated rules as
//! context-dependent — not unconditional suggestions.

use crate::corpus::Corpus;
use crate::domains::rust_perf::RustPerfDomain;

mod entries_01;
mod entries_02;
mod entries_03;
mod entries_04;
mod entries_05;
mod entries_06;
mod entries_07;
mod entries_08;
mod entries_09;
mod entries_10;
mod entries_11;
mod entries_12;
mod entries_13;
mod entries_14;
mod entries_15;
mod entries_16;
mod entries_17;
mod entries_18;
mod entries_19;
mod entries_20;
mod entries_21;
mod entries_22;
mod entries_23;
mod entries_24;
mod entries_25;
mod entries_26;
mod entries_27;
mod entries_28;
mod entries_29;
mod entries_30;
mod entries_31;
mod entries_32;
mod entries_33;
mod entries_34;
mod entries_35;
mod entries_36;
mod entries_37;
mod entries_38;
mod entries_39;
mod entries_40;
mod entries_41;
mod entries_42;
mod entries_43;
mod entries_44;
mod entries_45;
mod entries_46;
mod entries_47;
mod entries_48;
mod entries_49;
mod entries_50;
mod entries_51;
mod entries_52;
mod entries_53;
mod entries_54;
mod entries_55;
mod entries_56;
mod entries_57;
mod entries_58;
mod entries_59;
mod entries_60;
mod entries_61;
mod entries_62;
mod entries_63;
mod entries_64;
mod entries_65;
mod entries_66;
mod entries_67;
mod entries_68;
mod entries_69;
mod entries_70;
mod entries_71;
mod entries_72;
mod entries_73;

#[cfg(test)]
mod tests;

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 300 entries
/// (Batch 332, 2026-11-05, Plan 340 / Research 370: +2 -- the blazing-smtlib
/// distill, `entries_65`: the `byte-flag-table-masked-run-scan` law
/// (dispatch, BENCHABLE -- a const-eval 256-entry u8 class-flag table + a
/// masked-prefix run scan replaces the per-byte predicate chain; the
/// regime-complementary sibling of B243's `ascii-run-swar-block-skip`) +
/// the `token-batch-cursor-handover` law (dispatch, suggestion-only -- a
/// fixed-capacity staging array of zero-copy views + a cursor bulk handover
/// with boundary-applied abort actions); prior: Batch 331, 2026-11-05,
/// Plan 339 / Research 368 + addendum: +1 -- the hindsight round-2 pool,
/// `entries_64`: the
/// `batch-retry-ladder-classifies-failure-before-permanent-state` law
/// (concurrency, NotBenchable-by-domain -- classify before the ladder:
/// PROPAGATE a quota-reset control signal at first sight, `FAIL_FAST` a
/// deterministic failure with the permanent type, the bounded ladder for
/// transport-shaped only; the permanent-flag-from-transient orphaning
/// harm); Batch 330, 2026-11-04, Plan 338 / Research 368: +1 -- the
/// vectorize-io/hindsight distill, `entries_63`: the
/// `fanout-first-error-leaves-siblings-running` law (concurrency,
/// NotBenchable-by-domain -- cancel-then-await on the fan-out error path,
/// the two-sided shared-state discriminator, the error-envelope
/// reclassification caveat); Batch 329, 2026-11-04, Plan 337 / Research
/// 367: +2 -- the tembo/voxl distill, `entries_61` + `entries_62`: the
/// `event-damage-map-selective-partition-skip-with-consumer-hints` law
/// (dispatch, BENCHABLE -- the batch's GOAT-eligible lead, MEASURED
/// `ParetoDominant` 8.18x/6.96x/6.99x at 256/4096/65536 cells, allocs 0/0,
/// the 12% dirty band tracking the 1/dirty-fraction ceiling) + the
/// `online-cost-fit-thread-ladder-reopen-hysteresis` law (concurrency,
/// NotBenchable-by-regime); +1 count-neutral rider face on `reserve-then-commit-capacity-
/// accounting` (`entries_58`, the wire-header reserve-in-place law);
/// prior: Batch 326, 2026-11-04, Plan 330 / Research 363: +1 -- the
/// rust-lang/rust #164484 distill, `entries_60`: `adaptive-sparse-blocked-table-
/// encoding`, layout, NotBenchable-annotated; prior: Batch 327,
/// 2026-11-03, Plan 332 / Research 364: +1 -- the firezone/
/// sanstun distill, `entries_59`: `batch-reserve-monotonic-counters-
/// offload-bulk`, concurrency, NotBenchable-annotated; prior: Batch 318,
/// 2026-10-30, Plan 318 / Research 356: +2 -- the impeccable
/// io stretch-tier remainder, `entries_56` (plan 316's `- [-]` rows):
/// the generation-counter timer law + the exclusive-mkdir reservation
/// law, both ANNOTATED `NotBenchable`; PRIOR-STATE NOTE: this fn doc's
/// re-pin was MISSED by the Batch 316 lane -- that landing repaired the
/// module header 278->285 but left this doc at its B308-era 280 --
/// repaired en-route here, the missed-pin class) on the prior state:
/// Batch 308, 2026-10-30, Plan 313 / Research 352: +2 caching — the
/// 10-29b staged serving pool + `rust_perf` row, `entries_54`: the
/// cross-tier LRU-recency law + the refusal-quarantine verdict law,
/// both ANNOTATED `NotBenchable`; +2 count-neutral rider faces — the
/// REMOTE-writer arm on `kernel_opt` `drain-device-work-before-free`
/// (vllm #64256) + the CONSUMING-SIZE face on `admission-tests-the-
/// bound-the-admitted-mode-serves` (`entries_45`, llama.cpp #31350);
/// prior — Batch 305, Plan 289 / Research 322: +3, `entries_53`, the
/// bbtree completion (THIS doc's re-pin was MISSED by that lane —
/// repaired en-route here, the missed-pin class); prior — Batch 298,
/// 2026-10-29, Plan 289 / Research 322: +1 — the bbtree
/// ordered-structure-internals distill, `entries_52`; the batch's simd
/// candidate dropped at Pareto measurement — the negative lives in the
/// `entries_52` header; prior — Batch 288,
/// 2026-10-26, Plan 293 / Research 327: +1 complexity — the llama.cpp #31153
/// grammar rejection recursion-to-explicit-stack law; prior — Batch 282,
/// 2026-10-25, Plan 286 / Research 319: +3 — the `PerfR` C-to-Rust
/// translation classes, arXiv 2610.23044v1 — arithmetic +1 the
/// saturating-f2i range-proven conversion, dispatch +1 the loop-invariant
/// dispatch resolution hoist, iterators +1 the raw-pointer slice-view
/// recovery; all `NotBenchable` annotated; prior — Batch 274, Plan 278 /
/// Research 312: +1 allocation; prior — Batch 271, 2026-10-21, Plan 275 / Research 307: +12, the splash-wave
/// SECOND-TIER serving pool — see the module header for the batch chain;
/// prior — Batch 270, Plan 274 / Research 306: +12, the splash-wave serving
/// pool; prior — Batch 266, Plan 270 / Research 302: +1 complexity — the uv
/// marker-ISOP decision-diagram rule, benchable; prior — Batch 257, Plan 261
/// / Research 293: +6, the `TensorFold` staged remainder; prior — Batch 255,
/// 2026-10-19, Plan 259 / Research 290: +1 caching — the llama.cpp
/// #30637 `MoE` SLRU host-cache law, `NotBenchable` annotated; prior — Batch 250
/// slice 1, Plan 254 / Research 285: +1 simd, the hadd butterfly epilogue
/// (this doc was stale at 223 through B246/248/250s1 — repaired at the B255
/// landing with the honest label). Prior — Batch 248, 2026-10-16, Plan 252 /
/// Research 280: +6, the staged simdutf + unicode-normalization conversion
/// (the batch chain continues in the module header); prior — Batch 245,
/// 2026-10-16, Plan 248 / Research 277: +6, the Batch-244 defer-list
/// conversion + the count-neutral char rider on `byte-find-position-to-memchr`;
/// prior — Batch 244: +10, the byte-scan/search batch, Plan 247; Batch 243:
/// +2, the serving slot-state cache pair from llama.cpp #30580 + #30567 — both `caching`, both
/// `NotBenchable`; + the count-neutral draft-state rider on P65. Prior —
/// Batch 239: +2, the serving state-cache pair from llama.cpp #30514 + #30498 — both `caching`, both
/// `NotBenchable` annotated; on the Batch 237 base, 2026-10-15, Plan 239:
/// +3 — the P58–P61 conversion set, all `NotBenchable` annotated; see the
/// module header for the batch chain;
/// Batch 222 before it, 2026-10-10, Plan 216 / Research 252: +2, the soroban wasm size
/// pair — both `wasm`, both `NotBenchable` annotated; Batch 219 before the
/// pair, 2026-10-10, Plan 215 slot 2 / Research 251: +11, the
/// `TensorFold` native-engine serving/timing pack — complexity +2 the
/// cost-table hygiene pair, caching +3 the prompt-cache pair + the
/// fingerprint persistence, concurrency +2 the bounded peer-handoff wait
/// + the spinning one-message-per-exchange post lock (the T18 transport
///   cluster's landed members), dispatch +1 the load-qualification probe,
///   io +1 the Metal keepalive, allocation +1 the display carveout, and
///   thermal +1 the hysteresis band — the corpus's THIRTEENTH category.
///   The prior headline said 181 at Batch 202 and had gone stale through
///   B204/207/211's +1/+2/+1; every hand-typed per-category figure in this
///   doc has gone stale at least once — the gated count test +
///   `cargo refine --corpus-stats` are the live ledgers) covering the main
///   perf pattern categories (complexity, allocation, simd, layout,
///   iterators, concurrency, io, arithmetic, dispatch, caching, bevy, wasm,
///   thermal).
///   Allocation is the deepest family (29 — Batch 173 added the two
///   serde zero-copy rules; Batch 196 added the startup-estimate
///   allocation-shapes rule); complexity has 31 (the measurement-honesty family
///   lives here — B109 the work, the 2609.04040 rule the design, B115 the
///   config, Batch 177 the comparator (A/A control) + the observer-consumer
///   bias); concurrency has 23
///   (Issue 014 added the two metric-mismatch rules; Batch 65 added the two
///   contention rules; Batch 135 added the zero-cost atomic conversion + the
///   zero-sync partition; Batch 136 added the dynamic-work ladder + the
///   sender-termination idiom + the `par_bridge` deadlock; Batch 152 added the
///   three dial9 tokio rules — the blocking-pool segment hoist, the adaptive
///   consecutive-ready yield, the semaphore fan-out bound; Batch 153 added the
///   blocking-sleep async twin + the rayon oneshot bridge — the corpus's first
///   tokio×rayon boundary rule; Batch 154 added the oneshot response-send
///   hygiene + the bounded-channel-cycle deadlock; Batch 172 added the
///   crossbeam steal-batch rule; Batch 174 added the host-lock re-entrant
///   release rule + the deferred-command flush-hoist);
///   arithmetic has 8 (Batch 74 added 1); dispatch has 6 (Batch 61 + 74 added
///   1 each; Batch 99 added the `DestProp` branch-site rule; Batch 101 added
///   runtime-table handler-pair hoist; Batch 182 added the host-mirror
///   shared-scalar rule); caching has 10 (Batch 181 added the mask-memo owner-lifetime
///   rule; Batch 239 added the state-serialization + admission pair); io has 26 (the
///   measurement-honesty io rules ride Batch 122/123/129; Batch 139 added the
///   prefault-admission rule; Batch 141 added the tail-boundary and
///   stream-holdback rules; Batch 153 added the `TCP_NODELAY` request-response
///   rule; Batch 154 added the vectored-write capability flag; Batch 172 added
///   the mmap-settled-size-gated read + the nonblocking head-probe open); bevy
///   has 2; wasm has 1. simd has 9 (Batch 172 added the dual-mask fused block
///   scan — the corpus's first hand-SIMD `std::arch` entry with both AVX2 and
///   NEON arms).
#[must_use]
pub fn build_perf_seed_corpus() -> Corpus<RustPerfDomain> {
    let mut entries = entries_01::entries();
    entries.extend(entries_02::entries());
    entries.extend(entries_03::entries());
    entries.extend(entries_04::entries());
    entries.extend(entries_05::entries());
    entries.extend(entries_06::entries());
    entries.extend(entries_07::entries());
    entries.extend(entries_08::entries());
    entries.extend(entries_09::entries());
    entries.extend(entries_10::entries());
    entries.extend(entries_11::entries());
    entries.extend(entries_12::entries());
    entries.extend(entries_13::entries());
    entries.extend(entries_14::entries());
    entries.extend(entries_15::entries());
    entries.extend(entries_16::entries());
    entries.extend(entries_17::entries());
    entries.extend(entries_18::entries());
    entries.extend(entries_19::entries());
    entries.extend(entries_20::entries());
    entries.extend(entries_21::entries());
    entries.extend(entries_22::entries());
    entries.extend(entries_23::entries());
    entries.extend(entries_24::entries());
    entries.extend(entries_25::entries());
    entries.extend(entries_26::entries());
    entries.extend(entries_27::entries());
    entries.extend(entries_28::entries());
    entries.extend(entries_29::entries());
    entries.extend(entries_30::entries());
    entries.extend(entries_31::entries());
    entries.extend(entries_32::entries());
    entries.extend(entries_33::entries());
    entries.extend(entries_34::entries());
    entries.extend(entries_35::entries());
    entries.extend(entries_36::entries());
    entries.extend(entries_37::entries());
    entries.extend(entries_38::entries());
    entries.extend(entries_39::entries());
    entries.extend(entries_40::entries());
    entries.extend(entries_41::entries());
    entries.extend(entries_42::entries());
    entries.extend(entries_43::entries());
    entries.extend(entries_44::entries());
    entries.extend(entries_45::entries());
    entries.extend(entries_46::entries());
    entries.extend(entries_47::entries());
    entries.extend(entries_48::entries());
    entries.extend(entries_49::entries());
    entries.extend(entries_50::entries());
    entries.extend(entries_51::entries());
    entries.extend(entries_52::entries());
    entries.extend(entries_53::entries());
    entries.extend(entries_54::entries());
    entries.extend(entries_55::entries());
    entries.extend(entries_56::entries());
    entries.extend(entries_57::entries());
    entries.extend(entries_58::entries());
    entries.extend(entries_59::entries());
    entries.extend(entries_60::entries());
    entries.extend(entries_61::entries());
    entries.extend(entries_62::entries());
    entries.extend(entries_63::entries());
    entries.extend(entries_64::entries());
    entries.extend(entries_65::entries());
    entries.extend(entries_66::entries());
    entries.extend(entries_67::entries());
    entries.extend(entries_68::entries());
    entries.extend(entries_69::entries());
    entries.extend(entries_70::entries());
    entries.extend(entries_71::entries());
    entries.extend(entries_72::entries());
    entries.extend(entries_73::entries());
    Corpus::new(entries)
}

/// Widening or re-bitting any row = a new registration: re-verify the row's
/// `buggy_code` shape claim + re-run the G2 cost leg before promotion.
#[cfg(feature = "span_shape_dispatch")]
use crate::domain::{SpanShape, SpanShapeSet};

#[cfg(feature = "span_shape_dispatch")]
const TYPE_DECL_ONLY: SpanShapeSet = SpanShapeSet::empty().with(SpanShape::TypeDecl);
#[cfg(feature = "span_shape_dispatch")]
const FN_AND_ASYNC: SpanShapeSet = SpanShapeSet::empty()
    .with(SpanShape::FnDecl)
    .with(SpanShape::AsyncFn);
#[cfg(feature = "span_shape_dispatch")]
const FN_AND_HOT_LOOP: SpanShapeSet = SpanShapeSet::empty()
    .with(SpanShape::FnDecl)
    .with(SpanShape::HotLoop);

/// The span-shape annotation table (Issue 216, the codegram POC): `rule_id` →
/// the [`SpanShapeSet`] a span must intersect for the rule to stay in the
/// fan-out. The draft-side dispatch mask
/// (`draft::latent_matcher::span_shape::rule_admits`) consults this table
/// per Perf candidate; rules absent from the table are UNANNOTATED and admit
/// every span (safe default — the annotation roll-out is gradual and never a
/// cliff).
///
/// The draft→domains import direction is the established precedent
/// (`instantiation_rerank`/`headroom_probe` import domain proposers), so the
/// table lives beside the corpus it annotates — one home, no crosswalk copy.
/// A `rule_id` here that drifts from the corpus fails the crosswalk test below
/// loudly.
///
/// V1 rows (2026-11-07) are the SINGLE-SHAPE rules only — each row's
/// `buggy_code` lives entirely inside one shape, so the mask is provably
/// safe for it. Mixed-shape patterns (e.g. `usize-index-to-u32`: struct, fn
/// and loop in one pattern) are deliberately NOT annotated until the
/// multi-bit rows are justified by measured dispatch savings (the
/// annotation-priority map is issue 217 T9(b)'s curve).
///
/// Widening or re-bitting any row = a new registration: re-verify the row's
/// `buggy_code` shape claim + re-run the G2 cost leg before promotion.
#[cfg(feature = "span_shape_dispatch")]
#[must_use]
pub fn span_shape_annotations() -> &'static [(&'static str, SpanShapeSet)] {
    // layout: the pattern IS a type declaration — a fn/impl chunk cannot match.
    // async: the pattern is an async fn — a sync-only chunk cannot match.
    // hot-loop simd: the patterns are fns whose body is a for-loop — a chunk
    // with no fn and no loop cannot match (a bare type decl).
    SPAN_SHAPE_ANNOTATIONS
}

#[cfg(feature = "span_shape_dispatch")]
static SPAN_SHAPE_ANNOTATIONS: &[(&str, SpanShapeSet)] = &[
    ("layout-reorder", TYPE_DECL_ONLY),
    ("repr-u8-enum", TYPE_DECL_ONLY),
    ("box-large-enum-variant", TYPE_DECL_ONLY),
    ("async-future-size", FN_AND_ASYNC),
    ("simd-chunk", FN_AND_HOT_LOOP),
    ("chunks-exact", FN_AND_HOT_LOOP),
];

#[cfg(all(test, feature = "span_shape_dispatch"))]
mod span_shape_annotation_tests {
    use super::span_shape_annotations;

    /// The crosswalk pin: every annotated `rule_id` EXISTS in the seed corpus.
    /// A rename that leaves a stale table row fails here loudly (the
    /// corpus-binding crosswalk convention).
    #[test]
    fn every_annotated_rule_id_exists_in_the_seed_corpus() {
        let corpus = super::build_perf_seed_corpus();
        for (rule_id, shapes) in span_shape_annotations() {
            let hit = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == *rule_id)
                .unwrap_or_else(|| panic!("stale annotation: rule `{rule_id}` left the corpus"));
            // The annotation must be non-empty (an empty set masks the rule
            // EVERYWHERE — the algebra makes `intersects(empty)` false).
            assert!(
                shapes.intersects(*shapes),
                "annotation for `{rule_id}` is the empty set"
            );
            assert_ne!(hit.fixed_code, "");
        }
    }
}
