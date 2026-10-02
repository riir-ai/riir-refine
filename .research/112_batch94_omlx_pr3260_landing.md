# Research 112 — Batch 94: oMLX PR #3260 SSD Expert Streaming + Soft-REAP distill (kernel_opt D ×8 + rust_perf io ×1 + 3 riders)

**Status:** RECORD — batch landed 2026-09-01 (Plan 058; consumed Research 110's
queued verdict; 8 kernel_opt D rules + 1 rust_perf io rule + 3 riders; corpus
414 → 422 kernel_opt / 96 → 97 rust_perf)

- **Source:** [jundot/omlx PR #3260](https://github.com/jundot/omlx/pull/3260)
  "feat(moe): SSD Expert Streaming and Soft-REAP for MoE models", head
  `3343e44` at research time AND at mine time (re-cloned
  `refs/pull/3260/head` to `target/ext/omlx3260` — gitignored, disposable;
  **no force-push occurred**, so the research-time coordinator verification
  carried over and only needed confirmation).
- **Inspection:** Protocol A. Two parallel verification lanes (not fresh
  proposals — the rule candidates were Research 110 §6) re-read all 10 module
  files + the doc (3,503 LOC) and re-verified ~35 anchors byte-accurate.
  Dedup grep before authoring: all 9 candidates survive (nearest neighbors:
  B39 allocation-identity keying, B63-deferred-residual, B40 slot-order
  family, B71/B79 rollback family — all distinct, cross-cited in-rule).

## 1. Landed rules (kernel_opt D ×8)

1. `cross-run-profiles-key-on-content-not-stat-triplets` — warm-start
   profiles keyed `sha256(path\0size\0mtime_ns)`; mtime-preserving copy of a
   different artifact = foreign warm start, touch of the same artifact =
   silent learning loss. Key on content (BLAKE3); guard the derivation like
   its save/load siblings (`_hotlist_identity` is the one unguarded line in
   its module — latent in production call order, reachable via TOCTOU/direct
   callers).
2. `fill-gate-speculation-until-working-set-resident` — admission control
   for the speculative regime: latch readiness on all pools at capacity,
   count refused passes. Measured both sides: pre-gate 166.965 s speculative
   vs 14.709 s checked; honest ceiling 0.579 s vs 0.556 s at 100% hit.
3. `decayed-frequency-eviction-recency-tiebreak-route-protection` — halve
   hotness every 16 routed tokens; min-hotness victim with recency tie-break;
   note routes BEFORE hit/miss classification (misses are demand signal);
   current route protected; fail loud when all-protected. Warm-start hazard
   folded in as decision content: lifetime counts seeded into the DECAYED
   field = ~64,000× immunity skew + split-brain vs the persisted undecayed
   hotlist ranking.
4. `defer-promoted-write-to-first-consumer-dispatch` — bind (slot metadata)
   now, materialize (bank bytes) at the first consumer dispatch; the next
   layer's router evaluation IS the fence; explicit eval after every dynamic
   update only materializes the same writes twice; bind vs materialize kept
   as separate stats; eager for warm-start/pinned-preload only.
5. `rollback-snapshot-covers-all-mutable-state` — the coverage-contract
   defense rule: mutable-content leaves (ndarray) by-reference are safe ONLY
   under the append-only write contract; `__dict__`-less objects silently
   skipped; scope pinned to the conventionally-named `cache` argument.
   First-miss-only trust + rejected passes never returned + retry ×1 pinned.
6. `scratch-bank-isolates-one-shot-operands-from-hot-cache` — one-shot cold
   groups execute in a reserved scratch region (`bank = pool_size +
   scratch_slots`) that never mutates slot maps or residency masks: hot rows
   structurally untouchable. Measured: evictions 42 → 0, miss-heavy fixture
   2.12-2.19x.
7. `sort-consumer-routes-by-physical-slot` — remap-scrambled routes defeat
   the vendor sorted-gather fast path; threshold-gated (64 rows) argsort +
   inverse scatter restores it, bit-identical, +26.6% prefill at 1024x256.
8. `row-addressable-operand-index-not-full-tensor-map` — header-only
   row-address index (stacked + fragmented layouts), addressed rows only, no
   payload mmap. THE HAZARD TWIN in-rule: the legacy path guards uneven
   division (`total % rows`) while the schema path floor-divides UNGUARDED —
   silent weight corruption; the fragmented path's exact-byte check is the
   third instance of the same contract (merkle_root lesson class).

## 2. rust_perf io ×1

- `coalesce-nearby-reads-bounded-gap-cacheability-gated` — merge addressed
  rows within a bounded byte gap (64 KiB) into one pread; **the decision
  content is the cacheability coupling**: gap budget collapses to 0 on
  direct/F_NOCACHE descriptors because over-read bytes are only free when the
  page cache absorbs them. Re-selection preserves caller order incl.
  duplicates. Riders in the annotation: Darwin fcntl hints (F_RDAHEAD /
  F_NOCACHE) applied ONCE per persistent descriptor, advisory-swallowed; fd
  cache keyed (path, cache-mode). Source-reported only (serving-level
  numbers, no isolated A/B) — honestly NotBenchable, regime-annotated.
  The reader's fd race (unsynchronized `_fds` dict: double `os.open`, loser
  fd leaks; non-atomic stats counters; `close()` vs in-flight read) recorded
  in the annotation + Research 110 §7, deliberately NOT folded into this
  perf rule (it is a concurrency hazard, not an I/O-shape one).

## 3. Riders

- **B63** `same-input-gemv-weight-concat-at-upload` — FOURTH MEMBER:
  bank-level gate/up fusion (one preallocated quantized bank, shape doubled
  at upload; one gather-QMM + split; 3 → 2 decode QMM launches/layer, +8.6%
  single-token, no bank-memory increase; fusion gated on identical
  parts/quant params/dtypes/shapes).
- **B71** `gpu-api-thread-affinity-snapshot-metrics` — PRODUCER COMPLEMENT:
  the staging half — prefetch workers decode to HOST bytes only; device array
  construction stays on the owner inference thread (docstrings: "without
  creating thread-bound MLX arrays" / "Create MLX arrays on the caller's
  inference thread" / "materialize them on the calling thread").
- **B84** `token-local-only-approximate-accelerator` — ROUTING-SHAPED MEMBER:
  resident substitution with a max-relative-loss threshold, original weights
  retained for scoring + renormalized after selection, Qwen-only, 0% exact /
  100% resident-set-only, shipped disabled (default 0.0) pending quality eval.

## 4. Mine-time corrections to Research 110 (verification-lane yields)

- Line-range fixes: `execution.py` 33-34 → 31-40 (shallow container copies;
  the by-reference fallthrough is line 40) and 96-97 → 93-95 (the
  `__dict__`-less skip); `pool.py` 852-902 splits into anti-shape 852-858 +
  fix 896-1011 (902 is the FIX docstring); `safetensors.py` 242-253 is the
  stacked fused-half path only (fragmented = 274-329).
- Two paraphrased phrases replaced with the source's own words: "thread-local
  Metal streams are never transferred between threads" (NOT in source) → the
  docstrings at `safetensors.py:591/688/706`; "Eager RAM saturation" (NOT in
  source) → `optimistic_preloaded` / "total startup-fill coverage"
  vocabulary.
- The 15.0-17.4 t/s streaming figure is NOT in the repo — it comes from
  pedroraft's PR review thread (Research 110 §2 already attributed it
  correctly); kept out of corpus text.
- New measured justification found for R2: the pre-fill-gate fixture
  (166.965 s vs 14.709 s) was in the doc but not in the research note.
- R1's crash-hazard reachability refined: the production call order raises a
  clean error earlier (index constructor), so the raw stat crash is latent —
  the rule's content is guard INCONSISTENCY within a module, not an
  always-firing bug.

## 5. GOAT (all pass)

- G1 `batch94_pseudocode_parses` (cfg(not(kernel_verify)) per B70) —
  8/8 ref+opt parse.
- Presence + concept-needle gates: `batch94_rules_present`,
  `batch94_concepts_covered`.
- G2: kernel_opt lib **1238/0** (+3 gates = 1235 baseline) · default lib
  **1001/0** · all-features lib **1728/0**.
- G3: clippy `--all-targets` **0 findings** in default and kernel_opt
  states (4 doc_markdown backtick warnings in the new file fixed
  pre-commit — the B37 lesson, 8th strike; `MoE-Infinity` etc. need
  backticks in `//!` prose) · retrieval floors **14/14 HELD** (kernel
  self-variants 100/100/100 band, no re-pins) · score-bench **#25 flat**
  (heal 96.23 / created 1.89 / parse 100 / decline 100 — identical to #24).
- Count pins walked: tests.rs (corpus 422, category-array D 93, sums 422
  ×2), router.rs 461 pin + header, mod.rs header + D comment (85 → 93),
  rust_perf corpus.rs (97 entries / 40 annotated / 13 NotBenchable / io 5 /
  fn name ninety-seven / 84-benchable header).

## 6. Authoring notes (generator + landing)

- Entries + wiring + riders + gates emitted by a Python generator
  (`target/ext/gen_batch94.py`) per the B77 discipline (runtime escaping via
  chr(); ASCII-only content; needle self-verification before any write).
  Two generator bugs caught by the compile gate: missing `encoding="utf-8"`
  on writes (cp874 codec crashed on the `×` char in mod.rs and TRUNCATED it
  mid-write — recovered via `git checkout` + re-applying the manual edits;
  the lesson: an open-for-write without encoding is a two-failure mode on
  this box, crash AND truncation) and a doubled `.to_string(),,` from
  composing block_lit + explicit comma.
- Arithmetic correction at gate time: the batch total is 414 + 8 = **422**
  (Plan 058's draft said 423 — a transposition); caught by
  `kernel_categories_counts_match_plan_003_table` (422 ≠ 423), fixed in the
  same session before any commit.
- Full CI guard not run this cycle (the affected states — kernel_opt,
  default, all-features, clippy both states, retrieval, score-bench — were
  each validated; same honest-skip shape as Batch 95). The B84-guard repair
  commit already established Layer-5a green at a nearby HEAD.

## 7. Queue state after the batch

- The oMLX external-distill pool (Research 110) is CONSUMED. Remaining
  intake channels: arxiv listing walks (next window 09-02+), the 09-15 MCP
  search-quota reset, measured-GOAT distillations, post-mining intake
  (0 open classes).
- The 4 corpus-grade bug-yield hazards are DEFENSE-recorded (folded into
  rules 1/2/3/5 + the R9 annotation) per Research 110 §7 — not ours to file
  upstream.
