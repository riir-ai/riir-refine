# Plan 207 — Batch 207: the 10-07 staged leads (cubecl #1777 + #1751, cubecl #1767, bevy #26040)

**Status:** COMPLETE (Batch 207 landed; commit recorded in the snapshot row)

Source: staged at the 10-07 grading row (`.distill/001_mining_queue_snapshot.md`, first-read
grades at the fetched shas; **full quote verification owed at THIS batch composition** — the
standing law). Four leads, 2 rules per domain:
- **kernel_opt slot** — `tracel-ai/cubecl` (MIT OR Apache-2.0):
  - `9f6bb84fb8` #1777 "perf/minifloat decode" (B+/A−): fp8 normal-code decode as ONE shift
    + ONE bias-difference add; subnormal arm by magnitude alone; the placed-decode arm
    (`e2m1_words_to_f16_placed`, E2M1_F16_LIFT offset-folds-into-existing-multiply, NOVEL);
    the `Types::emulated_conversion` + `Features::conversion_is_emulated` admission arm;
    exhaustive e4m3/e5m2 decode tests incl. the off-by-one-bias admission guard; ~3% of an
    NVFP4 gemv (ue4m3 scale per 16 values, M2 Pro, source-reported).
  - `04a894b2` #1751 "Metal profile window drains" (A−): timestamps begin at ENCODER START;
    a window opened behind unrelated queued work timed its tail+contention as its own;
    autotune queued a read past the caches before every sample — M2 Pro samples read ~3×
    their duration and the tuner elected the outliers; fix names `TimestampSampling` once:
    resolve waits for completion (as before) + a window DRAINS the queue before it opens.
- **rust_perf slot**:
  - `tracel-ai/cubecl` `d307dd46` #1767 "simple CSE made linear" (B+): ScopedMap Vec-per-scope
    linear scan → hash per scope + ExpressionKey hashing everything but attributes; the
    measured NEGATIVE ('many regions of a few additions ran 15% slower than main') flips it
    into a HYBRID rule (SmallMap: inline-scan first 8, hash past).
  - `bevyengine/bevy` `1ba0c9f9` #26040 "Re-inline QueryManyIter fetch functions" (B−):
    #25200 wrappers around an `#[inline(always)]` fn stopped it inlining; laptop table
    propagate_parent_transforms 300.6→389.8/575.7 μs fixed 308.4/488.5; transform_pipeline
    428.5→516.1 fixed 407.4. Class: an inline(always) hint does not propagate through an
    unmarked wrapper.

Method: Protocol A plan-first (Skill §Protocol A); ephemeral clone `.raw/b207-cubecl`
(tracel-ai/cubecl) at the pins; bevy commit verified via the commit patch (API/.patch);
clone removed at close.

- [x] T0 — clone cubecl; locate the three commits; confirm class grades hold at the diffs;
      read all diffs IN FULL + fetch the bevy #26040 patch.
- [x] T1 — dedup grep BOTH vocabularies against the corpus (minifloat/fp8/decode/placed/
      lift family; profile-window/timestamp/drain/autotune family; CSE/scoped-map/hash-scope
      family; inline/wrapper/hint-propagation family) + kinship reads of the nearest
      neighbors (B194 LUT-gather decode, B176 shared-admission, the measurement-honesty
      D-class precedent, B33 LICM/CSE prose).
- [x] T2 — author the rules: 2 kernel_opt entries (`entries_gpu_77.rs`) + 2 rust_perf
      entries (`entries_18.rs`); every quote VERBATIM at the pins via `git show` /
      the bevy patch; riders where the mechanism is already owned (the emulated-conversion
      admission adjudicated IN-RULE beside B176, not as a rider).
- [x] T3 — batch207 parse + contract gates (needles from the real quotes) in both
      domains' tests.rs (3/3 first shot).
- [x] T4 — count pins re-pinned (kernel_opt 771 → 773, rust_perf 181 → 183) with honest
      labels (tests.rs ×4 per domain + router.rs + mod.rs categories + doc lines + the
      two stale header docs repaired with disclosure).
- [x] T5 — GOAT: G1 parse/syn (T3), G2 kernel_opt+latent_retrieval lib 3075/0 + default
      lib 2472/0 (ONE full run per command), G3 clippy `-D` at default/all-features/no-default
      (all clean, 0 doc mechanicals).
- [x] T6 — retrieval-precision re-measure (21/21 floors HELD, 211.1 s, no decay, no re-pin).
- [x] T7 — Research note 242 (`.research/.highwater` 241 → 242); §CONVERTED.
- [x] T8 — docs: AGENTS.md counts (×4) + `.docs/11_domains/kernel_opt.md` + `rust_perf.md`
      Batch-207 paragraphs + README rows ×2 + the tree count.
- [x] T9 — snapshot `.distill/001_mining_queue_snapshot.md` BATCH-207 row + queue-after
      line (same commit).
- [x] T10 — commit `feat: batch 207: ...` + push; `.raw/b207-cubecl` removed at close.
