# riir-refine

> **Boundary contract:** [BOUNDARY.md](BOUNDARY.md) — what lives here, what may depend on it, known drift.

**The code fixer that learns** — a multi-domain modelless framework behind the
`HealDomain` trait that distills code rules into a ruliology-style search
space, drafts fixes via a modelless ternary SIMD matvec, and validates with a
`syn`-based syntax pruner. The shipped domain set (Plan 001 + Plans 018/030/019/085):

| Domain | Feature | Status | Corpus |
|---|---|---|---|
| **clippy_lints** (default) | `clippy_lints` | ✅ Production | 100 corpus lints (Batch 191 2026-09-30: +mem_replace_with_default FIXABLE [`std::mem::replace(dst, Default::default())` → `std::mem::take(dst)` — STD DEFINITION IDENTITY, `take` IS `replace(dest, T::default())`; the two qualified spellings only, the `mem::` form behind the legacy_numeric_constants whole-file shadow-guard; bare `replace(..)` refuses — the SAFER-than-clippy import-ambiguity class; style/default-on — Plan 188, the coverage-report pack XVIII; two gate catches en route: the lint_level driver-match refusal of my perf-group read + the freeze e2e's latent-E0308 in the corpus atom, both fixed at source] — the coverage-report re-arm; Batch 161 2026-09-18: +unit_arg FIXABLE [the syntactic-unit-typeness hoist — a block with no tail expr is `()` by the language definition; hoist + pass `()` is value/order-identical by construction, clippy's own suggestion shape; complexity/default-on — Plan 152, the fifth all-FIXABLE pack] +string_lit_as_bytes FIXABLE [`"abc".as_bytes()` → `b"abc"` — coercion identity; NURSERY group (the plan152 default-groups arm correcting the stale "default-level perf" comment); the `\u{` escape refused — clippy's own suggestion there is invalid Rust] — the frontier-lead pack VII, Plan 152; Batch 160 2026-09-18: +from_str_radix_10 FIXABLE [the construction-identity parse rewrite — std's int `FromStr` IS radix-10 `from_str_radix`, so `u32::from_str_radix(s,10)` → `s.parse::<u32>()` is resolution-identical, NO shadow guard — Plan 151, the fourth all-FIXABLE pack] +assign_op_pattern FIXABLE [the exactly-once-binding derivation compound-assign rewrite — params/annotated-lets/init-transfer/suffixed-literal legs; the swapped-operand arm probe-measured commutative-only (`y = 1 + y` fires, `y = 1 - y` silent)] — the frontier-lead pack VI, Plan 151; Batch 159 2026-09-18: +single_char_add_str FIXABLE [the `mut_string_param_heads_over` mutable-String derivation twin — `String`/`&mut String` params, VALUE-based single-char arg + `escape_default` emit] +manual_clamp FIXABLE [the lattice-absorption clamp rewrite, ints+char only — FLOATS refuse on the NaN max/min-vs-clamp divergence, inverted bounds refuse on clamp's release-mode assert — Plan 150, the third all-FIXABLE pack]; Batch 158: +to_string_in_format_args FIXABLE [perf-group, default-on — pinned by the plan149 default-groups probe; the signature-derivation Display-blanket scalar receiver + the bare-{} placeholder guard, composing with uninlined_format_args by design] +manual_range_contains FIXABLE [the contains rewrite closed by the same scalar derivation + literal bounds — Plan 149]; Batch 157 2026-09-18: +redundant_closure_for_method_calls FIXABLE [the signature-proven `v.iter().map(|s| s.len())` → `v.iter().map(String::len)` — the element type derived from the fn signature + a construction-identity whitelist, String/Vec/HashMap/HashSet × len/is_empty/capacity] +iter_skip_next FIXABLE [the two-arm rule: proven slice receivers heal `.get(n)`, the oracle-clean inverse one step better than clippy's own `.nth(n)`; every other receiver → `.nth(n)`, identical for all iterators] — the frontier-lead pack III, Plan 148; Batch 156: +io_other_error FIXABLE [the `Error::other` splice, inference-identical, legacy_numeric_constants shadow-guard pattern] +wrong_self_convention VERIFY [the `into_*(&self)`→`self` receiver flip, no-vis inherent-impl v1 scope] — the frontier-lead pack II, Plan 147; Batch 155: +let_and_return/+redundant_slicing/+map_flatten FIXABLE +ptr_arg VERIFY — the frontier-lead pack, Plan 146; Issue 120 2026-09-16: +doc_markdown FIXABLE — the pedantic doc-lint family's every-landing manual backtick pass, a faithful port of clippy's own word predicates over syn `#[doc]` attrs + the P21 oracle lane consuming clippy's spans verbatim; P14 2026-09-14: +unnecessary_map_or FIXABLE — the born-lint closure, `map_or(false,f)` → `is_some_and`/`is_none_or`/`is_ok_and` on receivers PROVEN by the shared `receiver_kind` substrate + the safe-let extension; Batch 91, the P9 clippy-1.98 batch: +byte_char_slices FIXABLE, +manual_slice_fill FIXABLE, +manual_checked_ops VERIFY, +chunks_exact_to_as_chunks VERIFY [MSRV floor: std 1.88]; Batch 90: +items_after_statements FIXABLE, +manual_let_else FIXABLE; option_if_let_else REFUTED per `.plans/051` — fixpoint ping-pong with single_match_else; Batch 89: +use_self FIXABLE, +suboptimal_flops VERIFY; missing_const_for_fn KILLED per `.plans/050`); 81 --fix eligible + 16 --verify + 3 self_evolve-only |
| **rust_perf** (default since 2026-08-13) | `rust_perf` | ✅ Phase 2 (GOAT G1+G5 PASS, Bench 013 promotion + Bench 015 finalization + Bench 016 Issue 012 + Bench 017 Issue 014 + Bench 020 Issue 014) | 242 rules (Batch 266, 2026-10-21, Plan 270 / Research 302 — +1 the uv marker-ISOP decision-diagram rule: `isop-cover-from-decision-diagram-not-path-simplification` (complexity, astral-sh/uv PR #22651 @ `551c17cbc` — the Minato–Morreale ISOP cover extraction over a hash-consed diagram, the (lower, upper) don't-care interval memo + the shared Rc-Cover intermediate; GOAT 19.02×/12.93×/5.61× ParetoDominant, BENCHABLE annotated; the decision-diagram family seed); prior: Batch 257, 2026-10-20, Plan 261 / Research 293 — +6 the TensorFold staged remainder (the B256 B+ pool at the same pin `d1018b31`): `first-message-sized-to-smallest-job` (dispatch, `10ff8779`) + `draft-cost-timed-at-served-widths-not-extrapolated` (dispatch, `abb860c5`) + `checkpoint-reads-skip-page-cache-with-refusal-fallback` (io, `221c2b5d`+`6bb9daac`) + `reserve-stream-caches-at-admission-warm-request-paths-at-load` (allocation, `84c58d3f` sibling) + `cache-budget-first-sessions-take-remainder-never-reshrink` (allocation, `406b2090`) + `kept-prompt-states-pinned-host-cache-budget` (caching, `a739444c`), ALL NotBenchable); Batch 256, 2026-10-19, Plan 260 / Research 291 — +3 the TensorFold `9ccf4727..d1018b31` +988-commit campaign: `persistent-digest-memo-stat-identity-keyed` (caching, `018e73c6`+`20bca340`, the NAMED lead — the on-disk file-digest memo keyed by full stat identity) + `launch-cost-scales-with-allocation-count-pack-one-arena` (allocation, `84c58d3f` — 1.2 ms/launch over 1,900 buffers) + `pinned-entry-tombstone-freed-at-last-unpin` (concurrency, `8c6b40a3`+`3aedf373`); ALL ANNOTATED NotBenchable; floors HELD; score-bench #191 flat. Prior: Batch 255, 2026-10-19, Plan 259 / Research 290 — +1 the llama.cpp MoE SLRU host-cache law: `slru-probation-protected-promote-on-second-touch` (caching, #30637 @ `33d6135c`, ANNOTATED NotBenchable — first touch = probation, second touch promotes (protected capped n_slots/2, overflow demotes the protected-LRU head), victims scan probation-head-first then protected under the request-scoped seen-pin, hits promote BEFORE victims, no-victim aborts loud; the with_spec pin repaired 119→120 — B250 slice 1's +1 benchable never re-pinned it, PROVEN at HEAD); floors HELD 125/131/133 vs 124/130/132; freeze NOT owed; score-bench #190 flat. Prior: Batch 250 slice 1, 2026-10-18, Plan 254 / Research 285 — +1 the llama.cpp envelope pool's hadd butterfly: `avx2-tiled-accumulator-butterfly-epilogue` (simd 12→13, #30296 @ `3dbcb7f`, ANNOTATED, x86 Pareto CLOSED 2026-10-19 PARETO-DOMINANT ×3 1.02-1.35× — this row's PENDING-x86 token was stale through the T6 close, repaired at B255); floors HELD; freeze NOT owed; score-bench #189 FLAT. Batch 248, 2026-10-16, Plan 252 / Research 280 — +6 the staged simdutf + unicode-normalization conversion, Research 275's pool items 3+4: `minimal-perfect-hash-static-lookup` (caching — the two-level salted MPH, packed 24-bit key + 8-bit value in one u32, ONE exact key compare; the fourth rung on the static-lookup ladder answering page-memoization's random-query anti-regime; GOAT 5.26-12.99x verdict-stable x4) + `flat-pool-start-len-static-rows` (layout — one flat static pool + packed (start,len) handles, zero-copy slice rows; NotBenchable design-time) + `dense-subdomain-arithmetic-before-table-lookup` (dispatch — the Hangul closed-form block answered by arithmetic BEFORE the sparse table; GOAT 1.93-3.58x) + `fanout-transform-emit-callback-not-vec` (allocation — the FnMut emit shape, one monomorphized driver; GOAT 4.06-15.82x, allocs 860→1/13975→1/224660→1) + `inline-buffer-index-shift-over-drain` (dispatch — the source's own 'faster than drain()' law on the inline buffer; GOAT 1.27-2.16x) + `shift-mask-adjacency-check` (simd — the surrogate pairing's shift-AND + fold-and-compare + the all-but-last-lane escape; GOAT 1.27-2.68x); + the count-neutral BOUNDED-ARITY rider on smallvec-stack (the arity constant IS the capacity argument of record; annotated +1, unannotated -1); DROPPED at measurement: deferred-error-mask-block-check (ParetoDominated 0/3/0 0.64-0.73x — the scalar mask build is extra work the branchy carry never pays; the parity instrument also caught a shift-direction fixture bug pre-measurement); caching 16→17, layout 16→17, dispatch 15→17, allocation 29→30, simd 11→12; retrieval floors HELD no re-pin; gen2-4 freeze e2e ok; score-bench #186). Prior: Batch 246, 2026-10-16, Plan 250 / Research 278 — +1 the prism fork delta, the serving state-cache family's FORK face: `seq-fork-carries-pending-rollback-record` (caching, PrismML-Eng/llama.cpp @ `5ae2e169e` #354, MIT — seq_cp shared the cell but not the pending-rollback record rs_idx, so a fork taken right after a rollback read pre-rollback state; the fork copies the record, and for snapshot-plane state copies the SNAPSHOT plane the rollback points to, never the live plane; plane-count sizing n_stream*(1+n_rs_seq) in size_t; the bitwise fork-vs-direct-rollback oracle; NotBenchable annotated; caching 15→16; floors HELD 90.6/94.2/95.3 no re-pin; freeze e2e ok; score-bench #185). Batch 244, 2026-10-16, Plan 247 / Research 276 — +10 the text/unicode family's FOLLOW-UP, the byte-scan/search mechanism batch, from three sources at release pins: `rust-lang/regex` @ `regex-automata-0.4.18` + `BurntSushi/memchr` @ `2.8.3` + `BurntSushi/bstr` @ `1.13.1`: `byte-class-equivalence-table-shrink` (layout — the byte-class law: equivalence-class the key space before the table, states×256 → states×classes; GOAT 1.39-1.39x on a 1024-state fixture, the first 24-state fixture Mixed — the table must exceed L1) + `prefilter-choice-shape-ladder` (dispatch — the cheapest-first accelerator ladder; GOAT 44-46.6x) + `alternation-prefix-collapse-at-build` (complexity — common-prefix collapse + preference-trie shadowing + the ATTEMPTS ladder; GOAT 334.8-639.7x) + `byte-rank-select-rarest-probe` (caching — the MERGED regex-syntax rank() + memchr packedpair law, same 250 threshold; GOAT 22.9-38.1x) + `prefilter-skip-density-demotion` (dispatch — the PrefilterState feedback-control law, the corpus's FIRST measure-and-demote rule; GOAT 3.76-5.40x, the first scalar-probe fixture Dominated — the probe must BE the accelerator) + `iter-count-bulk-specialization` (iterators — the Iterator::count override, bulk mask-popcount; GOAT 9.19-9.28x) + `mod64-byte-membership-skip` (caching — the Two-Way ApproximateByteSet, the Boyer–Moore shift for absent bytes; GOAT 5.27-5.84x) + `byteset-cardinality-dispatch` (dispatch — memchrN for 1–3 vs the 256-entry stack table for ≥4; GOAT 1.34-1.58x, the first 10%-density fixture Dominated — the SIMD regime is the span between hits) + `utf8-fast-lane-cold-diagnostics` (dispatch — validate bare, recompute diagnostics windowed on failure; GOAT 8.60-10.33x) + `bufread-scan-in-buffer-scratch-fragment` (io — zero-copy fill_buf drain + one reused scratch; GOAT 2.77-3.63x, alloc 10892/262145 B → 0/0), ALL benchable + annotated (layout 14→15, caching 13→15, dispatch 9→13, complexity 32→33, iterators 11→12, io 26→27); the memchr-rank + regex-rank laws MERGED (one law in two repos, same 250 threshold); DEFERRED to Batch 245 with reasons (self-loop-state-memchr-skip, pair-probe-confirm-fused, align-probe-overlap-tail, match-branch-cold-confirm, char-search-encode-once-stack, canonical-set-guarded-rebuild, nonmax-sentinel-option-niche, the memmem Finder-hoist pair adjacent to regex-hoist-lazylock); DROPPED: sentinel-run-word-broadcast-skip (ascii-run-swar genus) + cross-product-budget-degrade (fixture artificiality) + the memchr-3.x suspects (verified absent at 2.8.3); retrieval floors HELD no re-pin (125/131/133 vs 124/130); gen2+gen3 freeze e2e ok; score-bench run #184. Batch 243, 2026-10-16, Plan 245 / Research 275 — +2 the text/unicode processing FAMILY SEED, the corpus's founding member of that family, from Hugging Face's Tokenmaxxing-BERT work + `huggingface/tokenizers` @ `5b5a3fcc` (branch `perf/faster-unicode-normalizers`, Apache-2.0, still the branch tip — PR-stage): `ascii-run-swar-block-skip` (simd — the high-bit-gather block mask skips class-pure ASCII runs wholesale; the source's 16-byte `to_bitmask()` form, the u64 SWAR mask portable; GOAT 6.41-7.78x, alloc 0/0) + `range-table-page-memoization` (caching — lazy per-page predicate memoization, `CHARS_PER_PAGE` 256 / 4,352 pages, built on first touch; one read replaces the ~11-branch binary search; GOAT 9.82-10.01x, alloc 0/0), both benchable + annotated (simd 9→10, caching 12→13); the THIRD lead (transform-borrow-until-first-change) DROPPED HONESTLY — ParetoDominated 0/3/0 twice (the find phase cannot be shared with the caller in a synthetic; alloc won 2.6x, bytes won 6x), negative in Research 275. Batch 241, 2026-10-15, Plan 243 / Research 273 — +2 the serving slot-state cache pair from llama.cpp #30580 @ `61ad80bf` + #30567 @ `b0d23916` (envelope `47f5a64ae→620d2225`, both diffs read at the fetched clone): `reuse-admission-threshold-must-equal-window-semantics` (with SWA the first evaluated token attends to the positions above pos - n_swa, so a state starting at pos - n_swa + 1 holds the whole window, but the reuse threshold asked for two more cells — a saved SWA state keeps only its window cells, so every read-back was always rejected and the slot rolled back to a checkpoint: a cache that never hits by construction) + `empty-slot-first-victim-order-save-before-overwrite` (free slots ordered empty first then LRU for both the LRU fallback and the child slots; a child task saves the state it overwrites; 5→36 prompt cache reads), both caching (10→12), both NotBenchable annotated; + the count-neutral draft-state rider on P65 (#30574: the slot file never held the draft context — a restored slot drafted with an empty or stale draft KV; the SDFT appendix + the degrade arm); the row previously read 201 (Batch 237) — stale through B239's +2 and repaired here with the honest label. Batch 237, 2026-10-15, Plan 239 — +3 the P58–P61 conversion set, ALL NotBenchable annotated, quotes re-verified at the pin clones: `f16-to-f32-accumulator-lut-breaks-conversion-serialization` (simd, ggml-org/llama.cpp @ `685f7089` #30342+#30343: the F16C FP16→FP32 intrinsic lowers to a vpinsrw into the live accumulator register, serialising the loop — 19 vs 4 cycles/iter estimated, 45–65% slower Q8_0/Q4_0 MUL_MAT_ID measured; the table breaks the false dependency; PER-DIRECTION: table for FP16→FP32, F16C for FP32→FP16; the x86 F16C GOAT lane staged — the 4090 box was unreachable) + `work-buffer-sized-by-execution-width-not-plan-tasks` (concurrency, llama.cpp @ `5cdadfc3` #30454: SET_ROWS planned n_tasks=1 while every thread writes its own row of the conversion buffer — threads > 0 wrote past the work buffer; the sizing authority is the op's true parallelization width) + `discretionary-cache-growth-keys-on-spare-not-available` (io, incoai/splash @ `ffc2d4be` #418: discretionary cache growth keys on SPARE memory — free + file-backed + purgeable + half of the compression credit at most a quarter of anonymous — never the OS available estimate; 7% reserve (2–7 GiB) + 1 GiB warning margin; warning-pressure latch + ≤1 GiB/paced-pass reclaim; measured free 0.07 GB vs host_available_bytes 31.9 GiB); + the count-neutral kernel_opt rider (the autotune-cache HARDWARE-PATH axis, cubecl @ `5a4f5882` #1811); Batch 222 before it, 2026-10-10, Plan 216 / Research 252 — +2 the soroban wasm size pair from `stellar/rs-soroban-sdk` @ `0d72c4c9`, Apache-2.0: `wasm-unwrap-panic-machinery-elimination` (on wasm every `.unwrap()` carries core::panicking's message + fmt machinery even under panic = abort; the `UnwrapOptimized` trait swaps the impossible arm for a 1-byte `core::arch::wasm32::unreachable()` trap — native keeps the plain unwrap — plus the `UnwrapInfallible` never-destructure clause; never rewrite a reachable unwrap) + `linker-dce-custom-section-volatile-marker` (data-only `#[link_section]` custom-section statics with no compiled-code reference are GC'd by the linker — root via `read_volatile` in a reachable path; the #2114 spec-shaking fix incl. the `keep_reachable` fn-pointer form); both `wasm` (family 1 → 3), both NotBenchable annotated; Batch 221 before it, 2026-10-10, Plan 221 — +1 the bevy `runtime-bool-duplicates-inlinable-body` (dispatch: a runtime bool branching around a large inlinable body forces duplicate-or-deinline; the landed #25864 form hoists the branch into a conditional `Option<Guard>` tuple so the body compiles once, the restore panic-safe in Drop; NotBenchable annotated); Batch 219 before it, 2026-10-10, Plan 215 slot 2 / Research 251 — the TensorFold native-engine serving/timing pack: +11 ALL NotBenchable annotated from `ashhart/TensorFold` @ `f1aef9c3`, Apache-2.0 — the A− cost-table hygiene pair `fastest-of-n-dip-drift-retime-cost-table` + `pava-monotone-cost-pooling-modeled-cost-not-clock` (complexity — fastest-of-7-after-one-warmup capture + dip-pair + whole-table drift retimes + per-BUILD-identity persistence with an exe-free reference key; PAVA pooling so each extra row's ms never rises with rows + the round EMA from the MODELED cost, never a clock, so every rank chooses alike), the prompt-cache pair `prompt-cache-canonical-interval-keys` + `prompt-cache-rewind-states-min-gap-eviction` (the staged entry SPLIT at authoring: keys = token prefix AND canonical decoded intervals; rewind states min_gap-spaced before the latest user turn, the kept-under-min_gap state IS the rewind, budget-fitted history-first, last-evicted while live) + `prompt-state-fingerprint-probe-bits-keyed-persistence` (caching — the fingerprint carries a startup probe's output BITS: behavioral identity, not metadata), the T18 transport pair `bounded-peer-handoff-wait` (#512: a peer handoff wait bounded at 10 s with named TpPeerSilent/TpPeerFailed errors, peer failed/quitting/stop re-checked every iteration, never claiming an absent arrival) + `spin-lock-one-message-per-exchange` (one message per exchange through a spinning post lock, 14-19 µs → 2-7 µs, the four-Mac fabric lessons), `thermal-hysteresis-band-pacing-chunk-placement` (the corpus's THIRTEENTH category thermal: wait above high, release only at ≤ low, read before each prompt chunk, `heat_wait_s` logged, no-zone memoized once), `degrade-not-fail-load-qualification-probe` (dispatch: at load a real-weight probe compares two 16-row streams vs one-row execution — numerical mismatch disables wider lanes loudly, execution failure stops the load), `metal-keepalive-residency-unwire` (io: a 400 ms one-thread command-buffer heartbeat against macOS's 1-2 s residency unwire / 2.7 s K3 re-wire), `display-carveout-all-or-nothing-admission` (allocation: GB10's 1,792 MiB display reservation as KV memory, a short grant refuses with the failing step named, recurrent state stays out); quotes verified at the pin via git-show blob reads; T18 dispositions: MTLFence/simdgroup-tile/nvfp4/kv-quant DEFERRED to kernel slots, exl3-oracle-fixture dropped to lane-intel; the row previously read 183 (Batch 207) — stale through B211's +1, repaired here with the honest label. The prior batch history (B207 cubecl CSE crossover + bevy inline-hint wrapper, B204 PrismML NEON PQ2_0 vec_dot, B200 TensorFold v0.6.0 serving-half ELEVEN @ c4646171, B198 PrismML selector-lattice beam, B196 grammar-window + startup-estimate pair, B181/B182 mlxfast, B152/153 tokio family, B141/143/145/147, B135, B118–B129 io distills) lives in the per-batch ledger — see .docs/11_domains/rust_perf.md; 147 carry context_notes (Batch 237 +3 → 147; Batch 222 +2 → 144; the row previously read 141 — stale through B221's +1, repaired here with the honest label). Bench 020 added the deterministic alloc-count axis (exact integers, stable across repeat runs) | Prior: Batch 245, 2026-10-16, Plan 248 / Research 277 — +6 the Batch-244 DEFER LIST conversion (the eight staged leads convert 6 + rider + defer, ZERO dropped: self-loop-state-memchr-skip GOAT 5.5-6.4x + pair-probe-confirm-fused GOAT 8.1-9.9x — fixture RESTRUCTURED once + the PARITY ORACLE caught the bit-8j+7 SWAR extraction bug, the pre-fix 8.9x was an artifact of skipped verifies — + finder-hoist-reuse GOAT 3.0-6.4x + canonical-set-guarded-rebuild + nonmax-sentinel-option-niche + match-branch-cold-confirm, all NotBenchable annotated + the count-neutral CHAR rider on byte-find-position-to-memchr; dispatch 13→15, simd 10→11, caching 15→16, complexity 33→34, layout 15→16; retrieval floors HELD 21/0/1i; gen2+gen3 freeze e2e ok; score-bench run #185; the stale-pin repair: with_spec/without_spec re-pinned to measured truth 114/109→ the B239/B207 pin-hiding class).
| **kernel_opt** (opt-in) | `kernel_opt` | ✅ Phase 3 + Batch 1-250 corpus | 917 rules (Batch 257, 2026-10-20, Plan 261 / Research 293 — +7 the TensorFold staged remainder: F `cluster-dsmem-shared-head-one-row-decode-attention` (01b7f3bb) + D `pdl-admission-by-image-set-marker` (bef432bb) + D `copy-source-quarantine-unproven-free-pinned` (2ba81a06) + D `prefill-batch-atomic-fallback-individual` (c4995966) + D `teardown-takes-list-whole` (30f05c6d) + D `drain-device-work-before-free` (98571a84) + A `verify-row-tiles-share-loaded-weight-panel` (4d1a3f15); + 2 count-neutral riders on gpu_102 hosts); Batch 256, 2026-10-19, Plan 260 / Research 291 — +4 the TensorFold `9ccf4727..d1018b31` +988-commit campaign kernel half: F `attention-softmax-state-exchange-once-per-block` (c05ce72c+80d1aba4) + A `expert-prompt-row-batch-amortizes-weight-pool-stream` (7bcab59f, the weight-stream amortization first owner) + D `glm-selected-prefix-move-before-evict` (3c0d6d3c+df4289c7, the NAMED lead) + G `mtp-ring-rebase-covers-rollback-window` (89aa09a7); ALL NotBenchable; floors HELD 90.6/94.2/95.3, kernel 100%. Prior: Batch 251-255 landed 25 rules (B251 Yukon +16, B252 +2, B253 +2, B254 +1, B255 rust_perf-only) — the row previously read 885 (Batch 250 slice 2), stale through those landings, repaired here with the honest label. Batch 250 slice 2, 2026-10-18, Plan 254 / Research 286 — +6 B the envelope-pool slice-2 set: SoA Q8_0 MoE pair, rms_norm register carry, K-quant wide-store geometry, MMVQ low-bit VDR rework, MoE group-selection fusion, uzu row-mask bitmap; +4 platform watches; floors HELD; T6 x86 Pareto pending. Batch 250 slice 1, 2026-10-18, Plan 254 / Research 285 — +1 D the uzu route-retirement law `attention-route-retirement-by-measurement` (`2c16e967` #973, the same evidence standard run backwards) + 2 count-neutral riders (B233 k-pool hoist, B166 FWHT graph fusion); floors HELD. Batch 249 slices 1-3 the TensorFold drift-pool (+30 + 13 riders). Batch 242, 2026-10-15, Plan 244 / Research 274 — the mlx-serve GDN/QSA pack, the corpus's LARGEST single-source batch: +31 own rules (A +7, B +3, D +9, E +2, K +1, L +1, N +8) across three slices `entries_gpu_97/98/99` + 6 count-neutral riders; source ddalcu/mlx-serve @ `1015f785`, MIT, 2026-10-14 — the Zig GDN/QSA serving engine for Apple Silicon; the GOAT `masked-in-place-field-prescale-other-operand` [portable CPU-exact novel numerics — mask a packed field in place, pre-scale the OTHER operand by 2^-shift via the f32 bit pattern]; `decay-gate-dtype-one-point-zero-lock` [a bf16 gate near one is exactly 1.0 and that head never forgets — league-relevant, our GDN state-dtype lane]; `lossy-route-physical-error-bar-from-noise-floor` [Issue-750-T3 external validation #3]; per-generation tables measured on OUR Bonsai 27B; landed ABOVE the 12-16 estimate, disclosed per the lithos precedent); prior: 816 at Batch 240, 2026-10-15, Plan 242 / Research 271 — the P67–P70 conversion: E the subgroup tail-lane clamp + A the pad row-classification + B the f16 upcast at foreign-graph translation; prior: Batch 236, Plan 238 / Research 268 — the splash GDN mid-prefill state-capture pack +1: D `mid-prefill-recurrent-state-capture-boundary-unsplit` (incoai/splash @ 711f8e5+c3dcf53+985463c+79a4329, Apache-2.0 + the vLLM #60050 FlashInfer convergence face @ badfcdbf9 — boundary-unsplit prefill, capture-inside-the-command, reserve-before-write, retire-on-publish); prior: 812 at Batch 234, 2026-10-14, Plan 236 / Research 266 — the #29353 GDN-MMA merge-moment pack +6 (register-resident scan state G, bf16 residual-pair 3-mma N, in-register (I+L)^-1 G, occupancy-keyed value-split ladder D, union smem scratch A, warp-split overlap + prefetch E); prior: 806 at Batch 233, 2026-10-14, Plan 235 / Research 264 — L +1 `reshape-relabel-axis-scramble-content-oracle` (the layout-contract family law: a reshape re-declares bounds over fixed memory and cannot permute axes; the N=1 degenerate case hides it; the oracle is content vs the per-axis decomposition — llama.cpp #28738 `6868a52` + #30480 `e7268bf`) + 1 count-neutral rider: B224 moe-duplicate-id += the #30501 UB-contract face (`8c2134a`); the row previously read 795 (Batch 223) — stale through B224's +1 and every batch since, repaired here with the honest label; per-batch ledger in `.docs/11_domains/kernel_opt.md`; the prior narrative (Batch 223 lithos-metal pack) follows: 795 rules (Batch 223, 2026-10-10 — the lithos-metal pack (Research 249/248, lithos-ai/lithos-metal @ `236475fa`, Apache-2.0): 11 own rules — the coherence scope by write set + the attention two-knob shared partial workspace + the derived-pack immutable-original (A ×3), the fusion scope by barrier cost (C), the open-group barrier placement + the gpu-resident control loop + the cost-aware verify length (D ×3), the bounded cross-worker barrier protocol + the coprime-stride steal (E ×2), the MoE readiness queue (H), the autotune fidelity (K) — + 3 count-neutral riders (the gdn parity-slot commit-by-recompute, the STAT_OUT prefold half + honesty pair, the fp8-subnormal density-split face); re-pinned at Batch 221 — TWO count-neutral riders: llama.cpp #30209 the Metal FA set-mirror face (the (dk,dv) admission allowlist EQUALS the compiled template set both directions; the consent triple — admission + ops sparse table + tuning row — moves in ONE commit) + cubecl #1802 the k=8-MMA accumulator-as-A-fragment face (a chained contraction's second MMA reads the accumulator registers directly; f16 k=8 Turing+, bf16 Ampere+); Batch 220 before it, 2026-10-10 — Lead 13, the prism #107 MoE weighted-reduce fusion as an own D rule (`moe-router-weight-fold-into-reduce-never-materialize`, PrismML-Eng/llama.cpp `930ddba3`, MIT); re-pinned at Batch 215 — the TensorFold native-engine delta pack, slot 1: the expert member-rows width ladder, H +1 (2/3/4 rungs, bit-identical at every width, the equal-at-2 null point, three recorded negatives) + the zero-anchored prompt grid chunk identity, D +1 (an off-grid start returns to the grid — resumed ≡ cache-off by construction) + the verified logits-derived outputs path independence, D +1 (logprob rows read the verified logits; float64 oracle within 2e-6) + the HIP affine contraction-off row invariance, N +1 (-ffp-contract=off for any-M reduction identity + the single-bit-mutation comparator) + the staggered two-queue fences, E +1 (MTLFence a segment + MTLEvent between, the 4096-row gate) + 1 count-neutral rider: prefill-head-final-chunk → the B208 headless-draft rule; re-pinned at Batch 214 — llama.cpp #29453 the componentwise version guard rollover, D +1 + 4 count-neutral riders; re-pinned at Batch 210 — llama.cpp #29887 the MoE expert GPU-cache write-path regime gate, D +1: small batches plan-and-evict, large batches read-if-present and never touch the LRU; the update hook rides the scheduler's host-weight split boundary; re-pinned at Batch 208 — the 10-07 staged leads: llama.cpp #30100 fusion-residual-identity + #29171 MLA FA exact-shape admission + #29928 headless-draft crop, all D; Batch 209 was rider-only — the #29425 pool2d warp-window crossover rider on row-dot-split, count held; was 773 at Batch 207 — cubecl #1777 minifloat decode + #1751 profile-window drain; was 771 at Batch 206 — llama.cpp #29442 conversion-chunking + #28782 per-thread-stream memset; was 769 at Batch 205, 761 at Batch 203; popcorn Research 002-007; riir-gpu file-mining Batches 14-63 COMPLETE; measured-GOAT distills B49+; external distills incl. ktransformers/omlx/PrismML/mlx-lm/llama.cpp/ds4/uzu/lily/MTPLX/vLLM/qwen38-mtp/ds4-PR991/low-latency-nccl/ds4-rebase-surge/PrismML-CUDA-MMQ/PrismML-Metal-FA/ds4-Qwen-Q2/ds4-PR5/cloudrift-Emmy ×4 + the B878-884 self-distill ×5 + the B145 Metal-shape pack (EFQ + MTPLX split-K); per-batch ledger in `.docs/11_domains/kernel_opt.md`) — the Plan-001 "~300-500 rules in ~15-25 batches" projection long since exceeded |
| **sec** (opt-in) | `sec` | ✅ Phase 0+1+2 + Batch 131 + Batch 140 + Batch 165 + Batch 258 (Bench 046; Phase 2 Batch 88; P8 intake Plan 057; InputValidation pair Plan 112; advisory-db distill Plan 120; wire-arithmetic overflow sink Plan 156; web-trust seed Plan 262) | 19 rules — FFI panic containment, Drop-glue panic safety, ptr-null hygiene, `from_raw_parts` len-zero, Rc backedges + the Phase-2 unsafe-impl/ffi-export/crypto trio + `nan-unsafe-sort-comparator` (PanicSafety) + the Batch-131 `InputValidation` pair (`length-prefix-validate-before-alloc` + `secret-file-create-without-0600`, Girder distill) + the Batch-140 `unbounded-read-into-vec` (rustsec/advisory-db distill, RUSTSEC-2024-0003 / CVE-2019-9514 class; + the Batch-258 decoder see-through rider) + the Batch-165 `unchecked-wire-arithmetic` (the overflow sink on the shared `wire_taint` walker; Kani g2c/g2d property pair) + the Batch-258 web-trust seed (`untrusted-path-join-escape` path traversal + `webhook-signature-over-raw-bytes` signature coverage + `unvalidated-egress-url` SSRF URL choice — cloudflare/security-audit-skill distill) — ALL InputValidation/crypto-web rules suggest-only; 6 fixable proposers + 7 suggest-only detectors + the real-clippy boundary gate (Plan 018 / Proposal 004) |
| **docker** (default since 2026-08-28) | `docker` | ✅ Phase 0+1+2 (Plan 030) | 31 rules (14 Class-A hadolint overlays + 17 novel cache/PID1/platform/base detections), 14 with bounded fixes; the first non-`syn` domain (stanza scanner) and the first with an external-binary oracle (`docker_verify` → hadolint, plus the Tier-2 `docker_buildcheck` → `docker build --check`). Zero-fixable-finding negative controls on two in-tree production Dockerfiles; G5-real `docker build` green |
| **dist** (default since 2026-09-13) | `dist` | ✅ Phase 1 + B1/B2 (Issue 085; Bench 082 + §6 addendum) | 7 suggest-only rules over name-matched `Cargo.toml` manifests — release-profile size knobs (strip / `[profile.dist]` size profile / panic-abort) for crates that ship a binary + the workspace-member profile trap + the Batch-143 `-Zhint-mostly-unused` large-API-dependency hint + the Batch-146 PGO training-set hint + the Batch-221 `bench-targets-shared-dependency-cluster-hint` (bevy #25321: ≥3 ungated `[[bench]]` targets compile every target's heavy deps for any one; the split = default-on per-crate bench features + optional bench deps + per-bench required-features — the green-zero caveat rides the advice). Measured provenance: 11,903,168 → 4,546,448 bytes (−61%) on this tool itself. Second no-`syn` scanner (zero new deps); never edits `[profile.release]` (the `refuses()` table pins it). The row previously read 5 — stale through B146's sixth rule, repaired here with the honest label |
| **shader_perf** (opt-in) | `shader_perf` | ✅ Phase 0+1 (Plan 134 / Proposal 014) | 8 WGSL render-pipeline rules (`@vertex`/`@fragment`; `@compute` stays `kernel_opt`) — 1 bounded (the `unbounded-render-loop-guard` proposer, Tier-0-invariant-checked) + explicit-LOD two-regime + branch-divergent decision + const-fold + 3 Class B annotations + 1 corpus-only. Third no-`syn` scanner (no regex); `shader_verify` = in-process naga (already in the lock — zero new packages); negative control pins ZERO findings on the 190-file riir-shader vgpu port corpus. Consumers: the upcoming shader-toy editor's submit/save gate + the riir-dapps Proposal 002 Stage-2 mint gate |

Heavyweight LoRA is the last-resort fallback, not the co-equal path. Plan 005
(2026-08-14) added the **latent suggest layer** — `cargo refine --suggest`
retrieves ranked corpus rules for real code spans (GOAT G1–G4 PASS, Bench 018;
`--fix` CLI wiring is Phase 6 — the Phase 4 rust_perf proposers landed as
library code).

> **Distribution (binding, [Proposal 003](.proposals/003_binary_only_distribution.md)):**
> this source is **private forever** (Research 003: anything `riir-*` is
> internal). The public gets prebuilt `cargo-refine` binaries only, via GitHub
> Releases on a small source-free dist repo (`brew tap`, `curl | sh`,
> `install.ps1`, scoop). No crates.io, no source mirrors — `publish = false`
> is enforced in `Cargo.toml`.

## Status

**POC — multi-domain framework (Plan 001) + Phase 0 (rule distillation) +
L1 (drafter) + L2 (syntax validator) + additive self-evolution loop done.**

The clippy-healing pipeline that distills clippy rules into a ruliology-style
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
| SE — Self-evolution loop (additive) | ✅ Done (Issue 004 + Issue 008; `self_evolve` DEFAULT-ON since Issue 018 with `RIIR_REFINE_SELF_EVOLVE=0` opt-out; G1–G5 + G6/G7 ALL PASS — 50.0pp healing-rate improvement on real `cargo clippy` feedback; Plan 005 T9.3 selection is `SelectionMode::BetaPosterior` — −8.4% selection regret vs the raw-rate blend, Bench 028) |
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
├── ruliology_search.rs # Pareto-front candidate search (opt-in)
├── self_evolve.rs      # HealPipeline + LatentFixMemory + LatentSkillEvolution
└── domains/
    ├── clippy_lints/   # feature "clippy_lints" (DEFAULT) — clippy::* lint healing
    │   ├── corpus.rs   # 10 common clippy lints
    │   ├── verify.rs   # cargo clippy re-run (behind clippy_verify)
    │   └── proposers/  # 10 per-lint AST codemods (Issue 010 Stage 2)
    ├── rust_perf/      # feature "rust_perf" (default-on since 2026-08-13) — rust-optimize perf healing
    │   ├── corpus.rs   # 126 perf rules (rust-optimize skill + perf-book + measured-GOAT/external distills; ledger in .docs/11_domains/rust_perf.md)
    │   ├── verify.rs   # cargo test re-run (behind rust_perf_verify)
    │   └── bench.rs    # perf-bench Pareto validator (behind rust_perf_bench, the GOAT gate)
    └── kernel_opt/     # feature "kernel_opt" (opt-in, 4090 DEFERRED)
        │   └── corpus/     # 795 rules across entries_gpu_*.rs (Batch 1-223: popcorn + riir-gpu + measured-GOAT + external distills; ledger in .docs/11_domains/kernel_opt.md)
        ├── router.rs   # KernelExpertRouter wraps pick_domain<4,5> (Plan 558)
        ├── thermal.rs  # Plasma→Hot→Warm→Cold→Freeze tier dispatch (returns Plasma)
        └── verify.rs   # compile-only stub (behind kernel_verify)
```

**Cross-domain auto-routing — shipped as fan-out (Plan 005, 2026-08-14).**
The unified `cargo refine --suggest` CLI runs with `--domain auto`: it **fans
out across all feature-enabled domains**, with the router verdict as a
per-span annotation (NOT a gate — the 8-D structural centroids mis-routed
fixtures, so gating starved the correct domain). Each domain is still
feature-selected; requesting a feature-off domain is a hard error. The bin
stays suggest-only — the Phase 4 rust_perf proposers landed as library
code; `--fix` CLI wiring is Phase 6.

### Feature matrix

| Feature | Default | Implies | Purpose |
|---|---|---|---|
| `clippy_lints` | ✅ on | — | clippy lint healing domain |
| `clippy_verify` | off | `clippy_lints` | real `cargo clippy` lint resolution check |
| `rust_perf` | ✅ on | — | rust perf healing domain (Plan 001 Phase 2; promoted 2026-08-13, Bench 013 G1+G5) |
| `rust_perf_verify` | off | `rust_perf` | real `cargo test` test pass check |
| `rust_perf_bench` | off | `rust_perf` | perf-bench Pareto validator (the GOAT gate; Bench 013) |
| `kernel_opt` | off | `katgpt-core/variable_rank_domain_expert` | GPU kernel opt domain (Plan 001 Phase 3) |
| `kernel_verify` | off | `kernel_opt` | CubeCL compile check (STUB — full validator deferred) |
| `sec` | off | — | security-pattern healing domain (Plan 018; FFI/Drop/ptr hygiene + the Batch-131 InputValidation suggest-only pair + the Batch-140 advisory-db unbounded-read rule + the Batch-165 wire-arithmetic overflow sink + the Batch-258 web-trust seed, 19 rules + 6 fixable proposers + 7 suggest-only detectors; the `--domain sec` CLI + boundary gate) |
| `docker` | **on** | — | Dockerfile bounded-fix healing + retrieval (Plan 030 / Proposal 006; 27 rules, 14 bounded, `--domain docker` CLI + the DL3059-refusal gate). Zero new deps |
| `docker_verify` | off | `docker` | hadolint agreement oracle for the Class-A overlays; degrades to the Tier-0 stanza check with one log line when the binary is absent |
| `docker_buildcheck` | off | `docker_verify` | Tier-2 `docker build --check` (BuildKit's own linter) agreement oracle for the 4 rules whose incumbent is a check NAME, not a `DL` code. Stays opt-in on purpose: it needs a running daemon and a registry round-trip per non-`scratch` fixture (~1.3-2.2 s each), a strictly heavier dependency class than hadolint |
| `dist` | **on** | — | Issue 085: shipped-artifact build-config advice over name-matched `Cargo.toml` manifests (suggest-only, never a manifest write). **DEFAULT-ON since 2026-09-13** — all four GOAT promotion conditions PASS (Bench 082 + §6 addendum: precision fixtures, score-bench #92 FLAT, router floor 74/101 measured at default+dist, real consumer cites; B1/B2 scanner extensions `759e209d`) and the owner pull arrived (the Issue-085 fix directive). Zero new deps (the second hand-written no-`syn` scanner after docker) |
| `ruliology_search` | off | `clippy_lints` | L3 Pareto-front candidate search |
| `latent_retrieval` | off | `dep:riir-rag`, `dep:riir-neuron-db` | Plan 005: span-level latent retrieval + `cargo refine --suggest`/`--fix` CLI (Bench 018 G1–G4 + Bench 019; T3.5 operator-aware BM25 tokenizer, Bench 027) |
| `clippy_verify` + `latent_retrieval` | off | — | T3.5.5 real-`cargo clippy` labeling oracle (`ClippyOracle`) + the uncontaminated G1 harness (`tests/uncontaminated_g1.rs`) |
| `bonsai_retrieval` | off | `latent_retrieval`, `riir-rag/bonsai_embedder` | Plan 005 T3.5 option 3: 768-D Bonsai dense rerank. **HEAVY** — 6.7 GB GGUF, ~48 s/embed on CPU. Measurement-only (Bench 027) |
| `self_evolve` | on | — | additive self-evolution loop (memory + EMA + `EvolveRecorder`; DEFAULT-ON since Issue 018, opt-out `RIIR_REFINE_SELF_EVOLVE=0`; BLAKE3-committed memory since T7.4; Beta-posterior selection since T9.3) |
| `rustc_errors` | **ON** | — | **Plan 019**: `cargo refine --fix-compile` — oracle-driven compile-error repair (E0597/E0499/E0502 uncontested + the E0505 contested opening; delegated codes ship no template by the measured floor). GOAT at N=212: **105/105 uncontested fixed where `cargo fix` scores 0/105; 126/126 repaired cases behavior-green post-fix** (Bench 045). **PROMOTED DEFAULT-ON 2026-08-29 (owner call): driver-shaped (inert until invoked — zero runtime overhead), floor gate stands, dep-tree unchanged (`compile_oracle = []`)** |
| `ternary_inference` | off | `riir-engine`, `riir-gpu` | Bench 010: katgpt ternary generation on clippy prompts |
| `groq_first` | off | `dep:ureq` | Issue 051: groq-first free-tier LLM routing (`src/groq.rs`) — the Groq chat client + the groq→CF provider ladder (`router`, also needs `cf_workers_ai`). The daily drain reasons on the free tier FIRST and falls back to metered CF on limit; either leg may be absent (`GROQ_API_KEY` absent → pure CF, byte-identical; CF creds absent → a groq-only window whose exhausted free tier ends it cleanly), both absent → refused. Hardened: split connect/call timeouts, single-allocation auth header, token scrubbed from server-controlled error strings. `kat_drain --preflight` checks the ladder on an empty backlog at zero metered cost. Usage lands on the cost ledger's `providers.groq` block → the ops dashboard's absorption card |
| `compile_oracle` | off | — | Plan 019 T0.3: the real-`cargo` error-JSON capture plumbing for the `rustc_errors` domain (deliberately NOT implying `latent_retrieval` — a compile fixer must not grow a retrieval dep) |
| `fix_verify` | off | `compile_oracle`, `latent_retrieval` | Issue 015: `--fix --verify` — the compile gate as tooling (apply → cargo-check → revert only offending edits; real-repo GOAT Bench 031) |
| `score_bench` | off | `clippy_verify`, `latent_retrieval`, `clippy_lints`, `rust_perf` | Plan 014: `cargo refine --score-bench` — the frozen real-code healing bench + the committed score curve (`.benchmarks/score_history.jsonl`) |
| `margin_mining` | off | `score_bench`, `latent_retrieval` | Issue 077 T3: `e^{−2M}` margin-weighted carve-mining order (`score_bench_mine_v6 --order margin` / `--ab`) — RECORDED DEMOTE (Bench 080: tie at every budget; retrieval-similarity margins don't select lint-bearing raw spans), stays opt-in, miner default stays count-desc |
| `l4_lora` | off | — | Plan 336 T4.4/T5.3: the L4 generative-fallback SEAM (`L4Fallback` trait + `DaemonL4` transport) — inert unless a fallback is installed; Plan 336 G1 measured 0/60, stays UNARMED (see [`.docs/10_self_evolve/l4_fallback.md`](.docs/10_self_evolve/l4_fallback.md)) |
| `pav_data` | off | `l4_lora`, `score_bench` | Plan 356 A1: PAV (process-advantage) dataset curation for the L4 fixer campaign (real-daemon or seeded-mock source; opt-in — the campaign is a deliberate 30-40h exclusive-4090 budget) |
| `contrastive_scope` | off | `katgpt-core/contrastive_scope` | Bench 040 / Plan 016: the input-scope gate at the heal seam — non-Rust garbage is declined whole before it can burn an LLM call (G1 8/8 OOS declined, bit-identical in-distribution) |
| `kat_service` | off | `dep:ssh-key`, `dep:ed25519-dalek` | Proposal 005 Phase 0 (Plans 023/024/025): the KAT service client substrate — meter/charge/account/store + `kat_redact` (the leak-posture gate) + the `cargo refine vote` governance CLI (Proposal 008 T2.4, G9-GOV) + the onboarding CLI (kat-011 T0.2, [Plan 059](.plans/059_kat011_t02_and_corpus_runtime_counts.md): `login` adopts the default SSH key as the account identity, `account --history` renders the local earned/spent view; `--corpus-stats` counts at runtime with provenance stamps) |
| `kat_transport` | off | `dep:ureq`, `dep:qrcode`, `kat_service` | Proposal 005 Phase 1 T1.6 (Plan 040): the corpus-lease HTTP transport + the out-of-KAT top-up QR |
| `cf_workers_ai` | off | `dep:ureq`, `latent_retrieval`, `clippy_verify`, `l4_lora`, `kat_service` | Issue 040/046: the CF Workers AI REST client (`src/cf_workers_ai.rs`) — the fixer arms, the USD daily-cap ledger (reserve-then-reconcile), and the fixer model: DEFAULT `@cf/zai-org/glm-5.3-flash` (owner pivot 2026-08-28, second call — the prior default 4.7 FAILED the Issue 043 re-designation gate 3/9 and stays priced + selectable via `RIIR_CF_MODEL`; [Bench 057](.benchmarks/057_glm53_flash_fixer_regate.md) re-gated 5.3 at 7/9 strict-keep on BOTH arms, beating the earlier CF bar of [Bench 049](.benchmarks/049_fixer_regate_poc.md)) |
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
  Heals + records outcome → next-call retrieval benefits
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
  mining via CommittedFieldBlend deferred; 13/15 reachable)
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
- **L3** (opt-in): ruliology Pareto-front candidate search
- **SE** (opt-in, additive): self-evolution loop — latent retrieval + EMA
  direction update. No gradient descent; the only state mutations are
  append-only memory + EMA direction refresh, matching the global
  modelless-first mandate.

The LoRA fallback (L4) is deferred to riir-train and is the conservative last
resort, not a co-equal path.

## Quick Start

```sh
# Run the clippy healing demo (L0 corpus → L1 drafter → L2 pruner)
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
cargo run --features latent_retrieval --bin cargo-refine -- --suggest src/

# Span-preserving fix (review mode, then write)
cargo run --features latent_retrieval --bin cargo-refine -- --fix --quiet src/
cargo run --features latent_retrieval --bin cargo-refine -- --fix --quiet --write src/
```

## KAT network — the KAT service (mining + balance)

The service plane (Cloudflare Worker in the private `riir-dapps` repo) hosts
the KAT ledger, the corpus lease, and the mining queue. The client talks to
`https://heal.gist.rs` by default; point it anywhere else with:

```sh
export RIIR_REFINE_KAT_SERVICE_URL=http://127.0.0.1:8811   # a localnet from riir-dapps
```

The loop (every step is honest about what is local vs server):

```sh
cargo refine login      # account key (Ed25519, OpenSSH format) + once-per-key welcome grant
cargo refine --fix      # heal — metered locally (accounting-only projection)
cargo refine sync       # PUSH the redacted signed batch (mining) + PULL the corpus lease
cargo refine sync --export batch.ndjson  # WRITE that batch locally, send nothing (no login; throwaway key)
cargo refine account    # burn / balance / grant / network state
cargo refine --refill   # out of KAT? dev/test faucet refill (see below)
```

### `cargo refine --refill` — the dev/test faucet refill (Plan 117)

When the balance runs dry, one command refuels from the service's faucet:

```sh
cargo refine --refill                   # faucet request → balance read → gate input refreshed
cargo refine --refill --refill-wait 120 # + a bounded settle poll (default: one read, no poll)
```

It composes three shipped surfaces — the faucet request (`faucet --request`'s
wire, amount pinned by the deployment's daily cap), an authorized crank
trigger (`POST /admin/crank/daily`, ONLY when `RIIR_REFINE_KAT_CRANK_TOKEN` is
set — the bearer IS the authorization, and prod never distributes one; without
it the row settles at the next hourly alarm), and the public balance read.
The result is written into the soft gate's input (the burn ack's server
balance), so the gate lifts the moment fuel lands instead of waiting for the
next burn push. Exit 0 iff the final synced balance is positive. The credit
is treasury-funded and capped — on a deployment whose treasury is at/below
its grant floor the crank report SAYS so (`skipped_treasury`) and nothing
credits until it is funded (riir-dapps Issues 010/025).

Leave at any time:

```sh
cargo refine --unmine          # opt OUT of contributing — key + balance UNTOUCHED, reversible
cargo refine --unmine --purge  # DESTRUCTIVE: also remove the local identity (orphans the
                             # server-side balance forever — no recovery path)
```

`--unmine` stops the push half of `sync` (the lease pull — a read — still
runs) and clears the sync bookkeeping; your key, balance and rewards stay
exactly as they are, so `--mine` re-enables contributing with the SAME
account. `--purge` is the explicit destructive opt-out: the identity IS the
key, so the server-side balance becomes permanently unclaimable, and a
purge + re-login cycle is the fresh-grant farm shape (each new key gets a
fresh once-per-key welcome grant) — which is exactly why it is never the
default.

## Fuel model — burn, auto-report, hourly settle

Healing burns **1 KAT per code word token**, metered locally
(accounting-only projection). After every run that settles a charge, the
client AUTO-REPORTS the cumulative burn to the service (`POST /burn`,
account-key signed) and prints one line with the fresh server balance —
account billing, a ~5 s budget, never blocks the run and never changes its
exit code. It rides the login, so `--unmine` does not stop it (billing ≠
contribution).

Offline runs **stack like local commits**: the ack watermark
(`.refine/kat_burn_ack.json`) advances only on a 200-accepted response, so a
push that fails is a local commit — the next run (or `cargo refine sync`)
reports the FULL stacked total in one signed report, and the server's
monotone watermark makes a replayed report a no-op. The service debits the
100M KAT free grant **warn-and-pay** and commits burn to the mining pool on
its hourly settle bucket (Issue 076 — the 7-day epoch stays the staking
cadence). The push rides the same network build as `sync`; a build without
the transport compiles it to nothing.

### The soft gate (Issue 075 — depleted accounts, owner decision 2026-09-08)

Prepaid fuel, not an interactive gate: when the account's **last synced
server balance** is ≤ 0 and the machine has never contributed-and-synced,
scanning runs REFUSE at start (exit 1) until the user refuels by any of
three paths: `cargo refine --refill` (the dev/test faucet refill — Plan 117),
contributing (`cargo refine --mine`, then one accepted `cargo refine sync` — the
burn-push ack records the server's `contributed` truth, which lifts the gate
for good), or paying (the web payment page). Since Issue 080 the sticky
grace is **server truth only** (dated removal 2026-09-10,
fleet v3 live): the burn-ack v3 `contributed` field decides — `Some(true)`
lifts the gate; `false` refuses; `None` (a pre-v3 ack/server) is simply not
contributed. The former machine-local `contributed_once.json` marker is
RETIRED — the `kat_contribution` module is deleted, nothing reads or writes
the file, and a copy left on disk by a pre-removal install has zero effect.
Logged-out, offline, and never-synced machines stay warn-only —
no server authority, no honest gate. The refusal names all three refuel
paths. `RIIR_REFINE_KAT_GATE=0` (or the
shared `RIIR_REFINE_KAT_METER=0` billing switch) disarms the gate; the burn
push itself keeps its never-blocks contract. Contributed accounts are
never re-gated while depleted — mining payouts land at the closing settle
of the claim's own epoch (≤7 days after that epoch closes; dapps issue 035),
and bricking a contributor mid-epoch is the exact failure the gate
exists to avoid.

## Automatic rustfmt (`--fix --write`)

Healed files are **rustfmt'd automatically** after the write — default-on,
`--no-fmt` to disable. No manual post-heal formatting step, no SKILL
required — the discipline is built into the tool:

- **fmt-clean-at-HEAD gate**: rustfmt only touches a healed file when the
  *original* (pre-heal) content was already rustfmt-clean. Files with
  pre-existing formatting drift are healed but **left unfmt'd** — a
  mechanical heal never drags unrelated formatting churn into your diff.
- **CRLF skip**: CRLF files are reported (`fmt: skip (CRLF)`) and never
  converted (rustfmt would rewrite every line ending).
- **Edition-aware**: the rustfmt edition comes from the owning package's
  `Cargo.toml` (`edition = "..."`, default `2024`) — bare-file rustfmt
  defaults to a pre-2024 edition and fails on let-chains.
- **Never fatal**: rustfmt missing from PATH or a parse failure warns and
  skips; the healed file is always left as valid Rust.

Dry runs (`--fix` without `--write`) report the **fmt plan** per file
(`fmt: would-format` / `skip (pre-existing drift)` / `skip (CRLF)`) plus a
run summary:

```text
$ cargo-refine --fix src/crate/
fix src/crate/foo.rs (12 edit(s)):
  [uninlined_format_args   ] line 41
  fmt: would-format
fix src/crate/bar.rs (3 edit(s)):
  ...
  fmt: skip (pre-existing drift)
scanned 2 file(s), ... fmt: 1 would-format, 1 skip pre-existing drift
```

The same summary appears after `--write` (`fmt: 22 formatted, 70 skip
pre-existing drift, 3 skip CRLF`). Under `--verify` the fmt pass applies
after the compile gate, to files with kept edits only.

## clippy-bench — the frozen healing score + SVG curve (Plan 014)

The bench that answers "are we getting better?" with a number that can
only move by healing better — **the denominator is frozen** (34 fixtures:
real pre-sweep mined spans + the T3.5.5 realistic multi-lint composite set,
provenance recorded per fixture), and the ground truth is real
`cargo clippy` before/after, not our own proposers.

![clippy-bench score](.benchmarks/score_history.svg)

Axes: `heal_rate` (up) · `created_rate` (RED, down — false positives)
· `parse_safety` (healed output compiles) · `decline_correctness` (clean
code left untouched — declining is a correct answer). The console report
also prints the **miss map** (which lints persist, where healing fails).

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

`scripts/watch_heal.sh` wraps `cargo refine --fix-compile --write` in a watch
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

1. **Write semantics are clean by construction.** `--fix-compile` NEVER
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
   tree; the HEAL SCOPE stays the list.

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
| Housekeeping worker | `https://riir-kat-housekeeping.foxfox.workers.dev` | **LIVE** — hourly cron, T9 code deployed, `READ_TOKEN` gate on (`/hours` `/days` `/backfill` anon → 401), `CF_API_TOKEN` set (`/status` → `token_set:true`, neurons + containers collecting, 14-day backfill run) |
| Ops dashboard | `https://riir-ops-dashboard.foxfox.workers.dev` | LIVE — serves the self-contained HTML; fetches `/latest` from the housekeeping worker. **Auth gate ships OPEN** (owner decision 2026-08-28; one `wrangler secret put DASHBOARD_TOKEN` closes it) |
| Nightly generator | `scripts/ops_nightly.sh` + `scripts/com.riir.ops-nightly.plist` | **Shipped, NOT bootstrapped** — two commands: `cp scripts/com.riir.ops-nightly.plist ~/Library/LaunchAgents/ && launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.riir.ops-nightly.plist` |

**Credentials live in `.env` (gitignored — never commit, never print).**
[`.env.example`](.env.example) is the documented template: what each var is
for, which consumer reads it, and where each is minted. The four core vars:
`GROQ_API_KEY` (free-tier LLM routing, Issue 051) · `CF_API_TOKEN`
(Account Analytics:Read — worker secret AND nightly generator; the only one
that cannot be minted from this machine) · `CF_DEPLOY_API_TOKEN`
(least-privilege deploy token, preferred by the nightly) · `READ_TOKEN`
(the housekeeping read gate, 64-hex). Optional: `ALERT_WEBHOOK_URL` +
`MONTHLY_BUDGET_USD` (cap-crossing paging — inert until set; crossings
still land in KV + `/status alerts_today`) · `DASHBOARD_TOKEN`.

Deploy rule (Issue 044 T16.1, load-bearing): wrangler honours
`CF_API_TOKEN` as a deprecated alias of `CLOUDFLARE_API_TOKEN`, so an
analytics-scoped value in the deploy environment silently breaks auth —
deploys run under `env -u CLOUDFLARE_API_TOKEN -u CF_API_TOKEN …` (the
nightly and `cloudflare/housekeeping` runbooks already do; keep it that
way).

Full narratives: Issues 044 (dashboard + alerting + nightly) and 045
(housekeeping worker, rollout, backfill) RESOLVED + removed 2026-09-01
per the noise-reduction rule — the live state + owner-gated residuals
are documented in [AGENTS.md](AGENTS.md) §"CF ops surface"; the full
narratives live in git history (`git log -- .issues/044_*` /
`git log -- .issues/045_*`).

## Coverage (measured, Bench 045 — the honest product numbers)

Over the N=212 known-inverse mutation corpus (12 committed anchors + 200
generated; every seed carries a ground-truth behavior test):

| error family | `cargo fix --broken-code` | `cargo refine --fix-compile` | verdict |
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
