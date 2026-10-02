# Research 063 — Batch 56: cross-path init lock + superseded-format live-fields constructor

**Status:** RECORD (batch landed, closed)

**Date:** 2026-08-18 · **Source:** riir-ai Issues 719 + 714/720 (both resolved same-day 2026-08-17/18 on this box) · **Corpus:** 351 → 353 (A 81 → 82, D 64 → 65) · **Kind:** measured-GOAT distillation, fourth after the corpus-complete declaration (the B49/B52/B54/B55 reopen path (c)).

## 1. Why these two, why now

Both source fixes are same-day measured GOATs from the 4090 box that postdate Batch 55 and were not yet distilled:

- **Issue 719** (`97bcf4738`): the parallel-mode full-suite freeze root-caused to the "race-benign" first-call pattern in `GpuContext::new()` — the OnceLock process-global cache (itself the Issue 714 fix) did not serialize concurrent CONSTRUCTION. Post-fix: the exact command that froze at 101/604 completed **605 passed / 0 failed / 2 ignored in 1664 s** — the first green parallel-mode full suite on the box.
- **Issue 720** (Bench 692): the WGSL Q4_K constructors uploaded BOTH weight sets — the full f32 set (~9.7 GiB device) dead on top of the ~1.4 GiB Q4_K set. Fix = `GpuGemmaWeightBuffers::norms_only()` wired automatically in `new_internal`. Measured: **−10.3 GiB device per construction** (14,089 → ~3,500 MiB) + −2.2 GiB host, LOGITS_HASH bit-identical. Explicitly recorded in the issue as "the Issue 714 class, missed on the WGSL Q4_K constructors" — a two-occurrence class (714's `skipped()` was the cubecl half), which is exactly the recurrence that earns a corpus rule.

## 2. The two rules

### 2.1 `concurrent-context-init-needs-process-wide-lock` (D 64 → 65)

Decision content (from `crates/riir-gpu/src/context.rs` L71-141 + `tests/context_init_race.rs`):

1. **The trap**: the init-outside-publish-on-success pattern (required for failure-not-cached semantics — an adapter may appear mid-run, so an Err must NOT pin the cache) does NOT serialize construction. Publication is atomic; construction is not: every first-caller that misses the fast path runs the FULL driver-level sequence (Instance/Adapter/Device + compute-server registration) concurrently.
2. **Failure half 1 — the wedge**: concurrent execution of that sequence wedges NON-DETERMINISTICALLY at the driver level on 4090/Windows/Vulkan. 7-thread race: 4 wedged forever inside `adapter.request_device()`; another run 1/7 inside `init_device`; **the stuck point MOVES between runs** (driver-level race, not a single lock site — bisection across adjacent test sets could not isolate it). The lib suite under default parallelism froze at 101/604 with every worker blocked behind first-call `GpuContext::new()`.
3. **Failure half 2 — the leak**: every racer that completes but loses the publication slot leaks its Instance/Adapter/Device + registered server for process lifetime when the substrate has no deregistration (CubeCL has none) — the partial reintroduction of the Issue-714 46-live-device leak.
4. **The cross-path half** (the load-bearing subtlety): the same driver-level sequence is reachable from MULTIPLE constructors (`GpuContext::new` AND `CubeCLContext::new`), each with its OWN OnceLock. Each path serializes its own callers; the two paths race each other. A per-path lock is insufficient — ONE process-wide init Mutex must guard ALL entry points running the same sequence.
5. **The fix shape**: const-constructible `Mutex<()>`, acquired ONLY after the fast-path cache read (steady state = one atomic load), DOUBLE-CHECK after acquiring, release before returning Err (failure still not cached), poison de-fanged (`lock().unwrap_or_else(|e| e.into_inner())` — a panicking init leaves nothing partial worth blocking over).
6. **The regression gate**: a race test covering BOTH same-path AND cross-path racers (`context_init_race.rs`: 8-thread same-path + 8-thread cross-path, 3× green ~1s).

Distinct from B38 `env-flag-resolved-once-not-per-dispatch` (WHERE a decision input is resolved — this is concurrency safety of shared-state construction), B26 `chunk-encoder-submits-below-driver-limit` (a per-encoder submission ceiling), B39 `autotune-cache-extract-outlives-owner` (lifetime salvage across owner destruction).

### 2.2 `superseded-format-constructor-live-fields-only` (A 81 → 82)

Decision content (from `crates/riir-gpu/src/gemma2_forward/weight_buffers.rs` L184-291 + Bench 692):

1. **The trap**: a constructor accepting an OPTIONAL second weight format (quantized Q4_K set, alternative runtime's pool) while unconditionally uploading the primary f32 set uploads DEAD WEIGHT — every dispatch site branches on the live format, so the primary projections are never bound in the superseded arms, but the memory is paid in full.
2. **Scale**: ~9.7 GB device f32 (2.2 wte + 7.5 projections, Gemma 2 2B) on ~1.4 GB Q4_K; or ~6 GB device + 2.4 GB host on the cubecl pool → 23.8 GB peak on a 24 GB 4090. Correctness-adjacent allocation bug, not a tuning miss.
3. **The proof method**: the dead-set proof comes from a DISPATCH-SITE audit (grep every bind of the primary buffers — all inside the superseded format's match arms or guarded entry points), never from the constructor alone.
4. **The fix shape — tiered live-fields-only variants**: `skipped()` (everything dead) and `norms_only()` (the small live subset: per-layer norm gammas + final_norm + host wte_cpu the decode loop reads per token), zero-size placeholder buffers for dead fields. The placeholder is a TRIPWIRE, not a silent zero: accidental bind fails wgpu validation LOUDLY. (Raw wgpu accepts size 0; substrates that validate non-empty need B31's minimum-legal-footprint instead — noted in-rule.)
5. **The wiring**: variant selection INSIDE the shared constructor (`new_internal` picks it when `weights_q4k.is_some()`) so callers cannot forget; `forward_layer_by_layer` REFUSES Q4_K passes rather than silently mixing formats.
6. **Recurrence**: shipped twice back-to-back (714 cubecl half-skip → 720 the WGSL Q4_K constructors) + the Issue-690 "2× GPU weight residency" hazard in the Q4_K trainer. A constructor-format pair is the trigger, not any one runtime.
7. **Measured**: bit-identical logits (LOGITS_HASH equal), −10.3 GiB device + −2.2 GiB host, strictly fewer allocs; the path had ZERO e2e test coverage before (the fix added the first determinism gate — the recurring gate-coverage lesson).

Distinct from B37 `verbatim-upload-on-format-match` (HOW to upload the LIVE format — raw blocks vs dequant-requantize; this is NOT uploading the dead one), B31 `min-rank-inert-adapter-zero-alpha` (structurally-REQUIRED-but-inert vs OPTIONAL-superseded), B41 `superseded-primitive-machinery-zombie` (dead machinery still EXECUTING per step vs dead memory allocated once), B26 `host-cache-immutable-device-weights` (caching to avoid per-call download vs which format's buffers exist at all).

## 3. Dedup analysis (Protocol A step 2)

Vocabulary-translation greps over the corpus BEFORE proposing (terminal grep — the project `grep` tool silently returns no matches on this multi-root workspace, the standing Batch-51 lesson):

- `OnceLock|oncelock|get_or_init` → only B38's config-flag resolution (different decision content). `init_lock|process-wide|deadlock|race` → only B26's encoder ceiling (different). **NEW** confirmed.
- `superseded` → B41 zombie (per-step machinery vs construction-time dual upload). `zero-size|placeholder` → B31 inert-adapter (required-but-inert vs optional-superseded) + one popcorn comment. `norms_only` → zero hits. **NEW** confirmed.
- Considered and rejected as instances of existing rules: Issue 718 H2's delta_routing variant bug (an instance of B39 `entrypoint-variants-share-layer-body` — the fix literally mirrors the sibling's delegation) and 718 H1's argmax tie-break oracle (test-oracle lesson, covered by B32c `order-stable-topk-via-index-sort` decision content).

## 4. GOAT

- **G1** syn-parse: `batch56_pseudocode_parses` PASS (both rules, ref + opt).
- **G2** lib: kernel_opt **760 / 0 / 1** (757 + 3 new gates; 568 default / 0 / 0 — the default count is up from B55's 549 by sibling fixture additions since, not this batch).
- **G2-corpus**: `corpus::tests` **166 / 0 / 0** (163 + 3).
- **G3** clippy: **0** warnings in BOTH default and kernel_opt states (6 `doc_markdown` backtick fixes applied pre-commit — `request_device`/`init_device`/`Q4_K` ×2/`skipped()`/`norms_only()` in the new `//!` header block; the standing `//!`-prose gotcha family).
- **Retrieval re-measure** (Issue 017 gate, `kernel_opt,latent_retrieval`): **13 / 0 PASS, floors HELD with zero decay** — op-aware clippy@1 89.6, blend 94.6/97.7 — identical to the B53 re-pin that B55 also reproduced.

## 5. Generator + landing notes

- One-shot Python generator (`scripts/gen_batch56.py`, run + deleted): **all anchors unique on the FIRST dry run** (second consecutive clean landing after B55); `json.dumps(..., ensure_ascii=False)` for every string literal; content computed before file open (the B46 truncation lesson); the awk orphan-`rule_id` scan clean post-write; first compile clean.
- Count-pin updates applied at ALL sites: entries.len() assert (351→353, "through Batch 56"), KERNEL_CATEGORIES A/D rows + per-category comments (81→82, 64→65), Plan-003 doc-comment corpus size, `kernel_categories_counts_match_plan_003_table` (updated for Batch 56 + sum), `build_corpus_categorized_round_trips_all_rules` (×2), `kernel_categories_cover_all_corpus_rules_exhaustively` comment, router.rs ×4 (doc header ×2 + rules_for_expert total ×2), the batch-30 historical Total comment, `.docs/11_domains/kernel_opt.md` Total.
- `entries_gpu_06.rs` at ~950 lines after this batch — **Batch 57 opens `entries_gpu_07.rs`** (the B55 note's threshold, now actually crossed).
- `.research/.highwater` 62 → 63.

## 6. Verdict

Both rules are honest measured-GOAT distillations: each names a trap that shipped twice in production code (719: the freeze + the leak; 714/720: the dead 9.7 GiB), each carries a measured repair with a regression gate, and neither duplicates existing corpus decision content. The measured-GOAT reopen path (c) remains the standing kernel_opt input alongside rust_perf + issue-driven work.
