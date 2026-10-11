# Research 242 — Batch 207: the 10-07 staged leads (cubecl #1777 + #1751, cubecl #1767, bevy #26040)

**Status:** CONVERTED (Batch 207 landed 2026-10-07, Plan 207 — 2 kernel_opt rules + 2 rust_perf rules, corpus 771 → 775 total / kernel_opt 773 / rust_perf 183).

## Sources

- `tracel-ai/cubecl` @ `9f6bb84fb8da51cae518f756e6adb598de949dde` — #1777 "Perf/minifloat decode" (3-part commit; cherry-picks of 53a56c5a + 2e0e6f48), MIT OR Apache-2.0, merged 2026-10-07.
- `tracel-ai/cubecl` @ `04a894b2a9922de56df4b17a07cca335cd4165e1` — #1751 "Fix/metal profile window drains", MIT OR Apache-2.0, merged 2026-10-06.
- `tracel-ai/cubecl` @ `d307dd46efbe80ae133d299eadd6f794e4c82b01` — #1767 "fix(opt): make simple CSE linear in the operations of a region" (2-part commit), MIT OR Apache-2.0, merged 2026-10-07.
- `bevyengine/bevy` @ `1ba0c9f93969ff5a07a7a1ca3dc5168be03ef63e` — #26040 "Re-inline `QueryManyIter` fetch functions" (fixes #25840), MIT OR Apache-2.0, merged 2026-10-06.

Provenance: staged at the 10-07 grading row (`.distill/001_mining_queue_snapshot.md`, first-read grades via the GitHub API commit messages); full quote verification done AT this batch per the standing law — every quote re-verified VERBATIM at the pins (`.raw/b207-cubecl` full clone + the bevy `.patch` fetch), line numbers read from `git show <sha>:<path>` at the post-commit files, NOT hunk-derived. Zero quote corrections vs the grading row; the diffs carried RICHER in-source content than the grading row (the carry-clause, the specials restructure, the `read_through` test kernel, the 4th benchmark row) — all folded in-rule. The clone was removed at close.

## What landed

### kernel_opt (entries_gpu_77.rs; B +1, D +1)

1. **`minifloat-normal-code-shift-bias-diff-decode`** (B, grade B+/A−) — the fp8 normal-code decode as ONE shift + ONE bias-difference add on the whole magnitude:
   - The old decode masked exponent + mantissa apart, rebiased, re-shifted both, kept a separate subnormal path selected on exponent==0, and OR'd the sign into BOTH arms.
   - The new decode: `magnitude = bits & FP8_MAGNITUDE_MASK`; `normal = (magnitude << mantissa_shift) + rebias` with `rebias = (F32_EXPONENT_BIAS - format.bias()) << F32_MANTISSA_BITS` (the bias difference pre-shifted); subnormal from `f32(magnitude) * subnormal_step`; select by `magnitude < smallest_normal` (1<<mantissa_bits) — MAGNITUDE alone; specials by `magnitude >= special`; sign applied ONCE at the end (`result | sign`).
   - The in-source doc (minifloat.rs L34-40): "A code's magnitude, its exponent and mantissa together, is already an `f32`'s exponent and mantissa in order, so a normal code is one shift and one add of the bias difference on the exponent field, rather than the two fields taken apart and put back. A zero exponent leaves the magnitude as the mantissa alone, a whole number of the format's smallest step, which is the one place the layouts disagree and the one select the values owe. The sign rides as its bit. Every code decodes exactly, and only through `u32` and `f32`."
   - Carry-safety: "The add cannot carry out of the exponent field: the largest exponent, rebiased, still fits it." (the clause that makes the fused form exact).
   - The exactness instrument: "the exhaustive decode tests pass for e4m3 and e5m2 on Metal, and fail when the bias is off by one" (commit message) — the off-by-one-bias test IS the admission guard (the enumerable-domain-bit-identity-admission-gate family's instance).
   - **The placed-decode arm** (fp4.rs): `pub const E2M1_F16_LIFT: f32 = 16384.0` (renamed from the private F16_LIFT); `e2m1_words_to_f16_placed` decodes each e2m1 code as f16 placed 2^14 short of its value, EXACTLY, "for a caller that multiplies by a block scale anyway and folds the lift into it rather than paying a multiply a value. e2m1_words_to_f16 is it lifted." Bit-PLACEMENT + offset-folding — the decode family's third mechanism (B194 = replicated-LUT gather; B199 rider = parity + field-arithmetic tables; ue8m0 = scale ENCODING, not decode placement).
   - **The emulated-conversion admission arm** (features.rs/properties.rs/metal.rs): `Types::emulated_conversion: BTreeSet<ElemType>` + `Features::conversion_is_emulated` + `DeviceProperties::register_emulated_conversion`; Metal registers its fp8/fp4. "A kernel decoding such a type itself pays no more than the conversion, and may fold what the emulation spends into work it already does." A NEW admission-predicate member beside B176's shared-admission family (composing, not merging — B176 licenses exact kernel-path widening under a shared admission; this predicates decode-from-bits economics on the conversion being software-emulated).
   - Measured: "about 3% of an NVFP4 gemv on an M2 Pro, where one ue4m3 scale is decoded per sixteen values" — SOURCE-REPORTED, never a bar.

2. **`profile-window-drains-queued-work-before-open`** (D, grade A−) — the measurement-honesty rule:
   - The mechanism, in-commit verbatim: "Metal samples a pass's begin when its encoder starts and orders an encoder after earlier work only through a buffer the two share. A window opened behind work it shares nothing with started beside it and timed its tail and its contention as the window's own. Autotune queues exactly that work, a read past the caches, before every sample: on an M2 Pro a decode gemv's samples read about three times their duration, except where the read happened to finish first, and the tuner elected those outliers."
   - The DOUBLE damage: corrupted measurement AND a wrong autotune election derived from it.
   - The fix names the backend fact ONCE: `TimestampAvailability` → `TimestampSampling` (InCommandStream — Vulkan writes the begin at the bottom of the pipe behind every earlier command; AtStageBoundaries — Metal samples at the pass encoder's stage boundaries) with BOTH consequences on it: `resolves_after_completion` (as before) + `drains_before_window` (NEW: flush + `device.poll(PollType::wait_indefinitely())` in start_profile — "Waits like system timing's sync, for the reason `drains_before_window` gives: work still running when the window's first pass starts is timed as its own.").
   - Admission polarity: only the AtStageBoundaries backend drains; InCommandStream backends cannot fail this and keep the cheap flush.
   - The new macOS-only regression test (tests/profile_window.rs) queues a nothing-shared `read_through` kernel before each sample and asserts the window times its own work.
   - Kinship: fused-serving-stack-measured-negative-ledger (the negative-ledger sibling), seed-warm-the-scored-seam (opposite polarity of the same hygiene), construction-identity-row-classes-replace-probe (correctness vs timing axis), fallthrough-toggle-oracle (work outside the timed region vs a toggled-off path inside it), hot-swap-weights-preserve-autotune-caches (caches vs measurement).

### rust_perf (entries_18.rs; complexity +1, dispatch +1)

3. **`per-scope-map-scan-hash-crossover`** (complexity, grade B+) — the two-sided container-choice law:
   - "SimpleCSE kept the expressions it had seen in a ScopedMap backed by a Vec per scope, scanned linearly on every lookup and insert, so the pass was quadratic in the operations of a region. A large unrolled kernel spent minutes in it. Each scope is now a hash map, and ExpressionKey hashes everything but its attributes, which have no Hash and stay in the equality." (part 1)
   - The MEASURED NEGATIVE that flips it into a hybrid: "Most scopes hold a handful of expressions, where a hash table per scope costs more than the scan it replaces: many regions of a few additions ran 15% slower than main. SmallMap scans its first eight entries inline, without allocating, and hashes past them." (part 2)
   - `const INLINE_ENTRIES: usize = 8` with doc "Most scopes stay under this many entries and are scanned inline; larger ones hash."; the key-type clause: `AttributeDict` has no Hash — keys differing only in attributes share a bucket BY DESIGN and the equality tells them apart.
   - The old before-state doc ("for small sets linear lookup tends to be faster") was the intuition asserted wholesale; INLINE_ENTRIES=8 is that intuition MEASURED and BOUNDED.
   - Kinship: bsearch-sorted-slice (search-order over one sorted array vs a per-scope container choice — distinct axis), fallthrough-toggle-oracle + construction-identity-row-classes-replace-probe (the measurement-honesty ethos this rule's negative arm instantiates), aa-control-prices-the-fidelity-instrument.

4. **`inline-hint-not-through-unmarked-wrapper`** (dispatch, grade B−) — the hint-propagation law:
   - "#25200 added wrappers around `fetch_next_aliased_unchecked_internal`, which is marked as `#[inline(always)]`. The new wrappers aren't marked as `#[inline(always)]`, so the function stopped getting inlined." (commit message)
   - The four-row laptop table (μs, ± spreads in-rule): propagate_parent_transforms roots 300.6±18.5 → 389.8±14.7 → 308.4±21.3; leaves 482.0±11.5 → 575.7±10.7 → 488.5±8.5; transform_pipeline static-optimized 428.5±12.6 → 516.1±16.4 → 407.4±17.0; no-static-optimization 303.4±10.1 → 395.9±14.1 → 281.9±14.2. The fix restores — and on the last row BEATS — the pre-refactor numbers. (Note: the grading row carried 3 of the 4 rows; the 4th surfaced at diff read and is folded.)
   - The class: an inline hint is PER-FUNCTION, never transitive — a marked callee behind an unmarked wrapper is a silent opt-out; hot-path refactors adding indirection layers need the wrapper-attribute audit + a bench before/after (this regression shipped and lived until a benchmark caught it).
   - Provenance note rides: the author disclosed LLM-assisted root-cause finding ("I used a LLM to find the cause of the regression. I then read the #25200 diff to verify its claims and ran the benchmarks.") — the verification discipline is the model, not the tool.
   - Kinship: dyn-to-enum (entries_01 — same eligibility logic: an opaque trait object kills inlining at the CALL, an unmarked wrapper kills it at the HOP).

## Dedup (both vocabularies, run at authoring)

- minifloat/decode/placed/lift/emulated: zero prior owners (B194 gather-LUT, B199-rider field-arithmetic, ue8m0 encoding, bit-exact-dispatch-fusion-boundary-emulation all distinct axes — grep hits read and ruled distinct).
- profile-window/timestamp/drain/autotune: zero prior owners (the only autotune rule is hot-swap-weights-preserve-autotune-caches — runtime caches, different axis).
- CSE/scoped-map/SmallMap/inline-scan: zero rust_perf owners (kernel_opt CSE hits are B33's LICM/CSE prose + B116's graph-build memo — different axes); bsearch-sorted-slice distinct.
- inline/wrapper/hint: zero owners (entries_01's dispatch rules are the trait-object/devirtualization axis).

## Riders considered and NOT filed

- The emulated-conversion admission was a candidate RIDER on B176 (`dtype-native-source-read-with-shared-admission-predicate`); adjudicated IN-RULE instead — it is integral to the same commit's decode story (decode-from-bits economics), and a rider would have touched a shared host file for one clause. The distinct-from + composition note carries the family membership.

## Gates (all green)

- batch207 parse+contracts 3/3 (2 kernel + 1 rust_perf) — FIRST SHOT after one compile fix (the batch gate's `bench_spec` assert is cfg-gated: PerfRule compiles the field out at the default posture) and one sha-lengthening (the cubecl CITE carried the short sha; the needle wanted the full pin).
- kernel_opt+latent_retrieval lib **3075/0** (3 ignored = the documented freeze-regen e2es; 1112.9 s on this box — the suite is SLOW here; one full run per command, the earlier watchdog-halt class).
- default lib **2472/0** (2471 + the batch207 rust_perf gate).
- clippy `-D` ×3 postures clean (default / all-features / no-default) — **0 doc_markdown mechanicals** (the authoring brief carried the backtick law; first batch with zero heal-needed headers since the B192 class started).
- Retrieval **21/21 floors HELD** (211.1 s, no decay, no re-pin).
- Freeze + score-bench NOT owed (zero clippy-corpus delta, P30).
- Count pins re-pinned with honest labels: kernel tests.rs ×4 (total ×2 + category sum + grouped) + category array (B 86, D 174) + batch116 pin + router.rs + mod.rs B/D headers + the 739-rule exhaustive doc; rust_perf tests.rs ×4 (total + without_spec 83 + annotated 129 ×3 sites) + membership asserts ×2 + the category prose disclosure; AGENTS.md ×4; README rows ×2 + the tree count; rust_perf mod.rs stale 163-header repaired with disclosure; kernel mod.rs stale 739-doc repaired.

## Queue after

- nnethercote R134 still UNSTAGED (no new lead at the 10-07 re-test; next re-test ~10-08 or on the October post).
- The Research-239 follow-up pool stays in intake (same-file `q2_0_q8_0`, ggml-org arm quants, karusrus i8mm pin-before-reading, SYCL XMX `a14c7de9` watch-only).
- The cubecl post-bump-shift ledger carries +9 members (the riders/watch from the grading row: #1773 dry-run-untimed, #1756 tuner ordered-group early-stop, #1750 TF32-CUDA lane intel, #1759 native-bf16-LLVM, #1761 buffer-alias-promise, #1766-#1773 span); wgpu Send/Sync recursion series (C+ compile-time watch).
- Walk #14 opens Tue 10-13; Plan 204 T11 (M3-Max GOAT) rides the M3 idle loop.
