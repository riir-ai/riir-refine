# The serving slot-state cache pack — llama.cpp 47f5a64ae→620d2225 distill

**Status:** RECORD — distilled (Batch 241 / Plan 243; corpus rust_perf 203 → 205, caching 10 → 12; +1 count-neutral rider on entries_27 P65)
**Date:** 2026-10-15
**Source:** ggml-org/llama.cpp, envelope delta `47f5a64ae` → `620d2225cc436ab0b001e3200e590e3b286ef9ab` (14 commits, all graded); the three convertible commits read in full (diffs + commit messages + tests) at the fetched F:/llama.cpp clone, every quote re-verified verbatim AT CONVERSION.

## Envelope grade (14 commits)

- **#30580 @ `61ad80bf`** — RULE-GRADE B+ (P71, converted). The rest of the window: NULL — `620d2225` version bump 0.7.0, `cd0a5b6a` Nemotron model registrations, `a11b6530` ci-status.sh, `a6b91d8c` Mellum model params, `c0cb53ac`+`4ed1c850` ggml sync-marker + version bump, `711780ff` CI ty bump, `fd55cb98` UI agentic-turns default, `3e4dfa8f` maion-coder registration, `08698c81` jinja context copy-ctor (a 14-line C++ tweak with no law-stating body — no generalizable content, no Rust host), `82e1e2fd` jinja nan/inf tojson semantics.

## The rules

### P71 — `reuse-admission-threshold-must-equal-window-semantics` (B+, caching, NotBenchable)

THE LAW: a state-reuse admission threshold must equal the EXACT bound the attention
semantics guarantees. With SWA, "The first evaluated token attends to the positions
above pos - n_swa, so a SWA state starting at pos - n_swa + 1 holds the whole window,
but the reuse threshold asked for two more cells. A saved SWA state keeps only the
cells inside its window, so a state read back from a slot file or from the RAM prompt
cache was always rejected and the slot rolled back to a checkpoint."

The failure signature is the decision content: a margin NEVER fails loudly — the
rollback path is correct (a checkpoint restore computes the same tokens), so the only
observable is a cache whose hit count is permanently zero. The detector class is the
test's own instrument: reuse pinned as EQUALITY OF CACHE COUNTERS between a restored
slot and an untouched slot (`cache_n`/`prompt_n` equal), via the new `override_kv`
harness (a window smaller than the test context —
`gemma3.attention.sliding_window=int:64`).

The repair: `pos_min_thold = max(0, n_swa > 0 ? pos_eval - n_swa + 2 : pos_eval)` —
SWA matches the window; no-SWA "keeps the exact bound" (the state must end before
pos_eval). The threshold is the ONE bound both the live state (`pos_min >= thold` →
reject) and every checkpoint (`cur.pos_min < thold || cur.pos_min == 0` → accept)
are judged by — which is exactly why two cells of margin reject the whole
saved-state lane.

### P72 — `empty-slot-first-victim-order-save-before-overwrite` (B, caching, NotBenchable)

TWO faces, one pair: (1) "Free slots are now ordered empty first, then least
recently used, for both the LRU fallback and the child slots" — an empty slot costs
nothing to take, so "child tasks often land on a slot holding a parent state while
empty slots are idle" is pure loss; (2) "a child task saves the state it overwrites"
— the displaced parent state is persisted to the prompt cache BEFORE the clobber.
Measured (source-reported): "With 4 grouped decision requests in flight and 12
states in rotation, prompt cache reads go from 5 to 36."

The mechanical trap beside the law: the collector's early exit was DELETED
(`if (free_slots.size() >= n_slots_needed) break;`) — the old collector stopped at
first-enough, fine for an unsorted take, but a sort over a TRUNCATED set ranks the
wrong candidates; truncation must happen AFTER the stable_sort. The test witness:
`f_keep = 1.000` log-line counting (full-prompt-reuse) growing by exactly the
number of displaced states.

### P73 — the draft-state slot-file rider (count-neutral, on entries_27 P65)

The STATE-STREAM COMPLETENESS face: "The slot file held the target context and the
context checkpoints, but never the draft context, so a restored slot drafted with an
empty or stale draft KV until the cache refilled." The repair is a versioned
appendix (SLOT_DRAFT_MAGIC "SDFT" + VERSION 1) carrying "the live draft state, the
full draft context and the speculative state, in the same appendix style as the
checkpoints", with the degrade arm: "A draft state that does not fit the draft
context is dropped and the draft starts empty, while older files and older servers
keep working." P65 owns WHICH cells a partial write owes; this owns that a
persisted-state FILE owes one appendix per state STREAM — the completeness axis the
leaf-flag law does not reach.

## Dedup (re-run at authoring)

- reuse-threshold/window-semantics/off-by-two: zero corpus hits — P65 owns
  partial-serialization, B219 owns keying/eviction, P66 owns admission-by-
  reusability; the reuse-admission PREDICATE was the family's unowned fourth face.
- empty-first / victim order / save-before-overwrite: the victim family
  (`decayed-frequency`, `expert-cache-write-path-regime-gated`, B219
  rewind-protection) all order OCCUPIED slots; empty-before-occupied + the
  save-before-overwrite duty were unowned.
- draft state / slot file: only P65's own context (the rule this rider extends).

## Family read

The serving state-cache family now has all four faces + the completeness axis:
ADMISSION (P66 — which states earn entry), KEYING (B219 — what makes two states
the same state), EVICTION (B219 — min-gap rewind states), REUSE-ADMISSION (P71 —
when a saved state may serve), SLOT SELECTION + overwrite-persistence (P72), and
STREAM COMPLETENESS of persisted state (P73 rider). All from the same upstream
serving lane (ServeurpersoCom's decision-serving arc) — the corpus now covers the
prompt-cache lifecycle end to end.

## Gates (this batch)

- G1 batch241 needles + parse: PASS (first run caught one needle CASE drift —
  'the SWA threshold' vs the verbatim 'The SWA threshold'; the corpus is the
  authority, the needle was fixed to the verbatim capitalization).
- G2 default lib: 2672/2672 (2670 standing + 2 gates).
- G2b `rust_perf_bench` posture corpus suite: 246/246 (+4 ignored standing) —
  includes the REPAIRED `without_spec` pin 102 → 106 (stale through B239's +2;
  only the feature postures compile it — the B207 lesson repeating, repaired with
  the honest label) and the held `with_spec` 99.
- G3 clippy `-D warnings`: default / no-default / all-features ALL CLEAN (the 5
  doc_markdown findings in the fresh `//!` headers healed via
  `cargo refine --fix --write --verify` — the B192 class, 10th+ strike; one bare-URL
  `F:/llama.cpp` needed the manual backtick form after the healer's `<>` wrap still
  tripped the scheme-parse — recorded as a healer miss class).
- Retrieval floors 21/21 HELD (256.18 s; blend@3 93.6/94.2, clippy@3 89.6 vs floors
  89.4/93.6/93.6 — the +2 rules added no perf spans; NO re-pin).
- Freeze/score-bench: NOT owed (zero clippy-corpus delta, P30).

## Count pins (203 → 205)

mod.rs ×2 (module doc + Returns doc) · tests.rs ×7 (the master count, without_spec
102→106 with the stale-repair label, annotated 149→151 ×3, unannotated 54 notes ×2)
· AGENTS.md ×2 · README (the row read 201 — stale through B239's +2, repaired with
the honest label) · `.docs/11_domains/rust_perf.md` (same stale-201 repair).
Zero stale 203/201 references remain (wide grep).

## Staging notes

- The claim `738081164` (plan 243 + the .distill/001 claim row) pushed and was
  accepted first try — zero contention.
- Watch legs at the same poll: prism `e3de715222` = pin UNCHANGED; prism-b ladder
  top 10777 UNCHANGED (numeric read); cubecl v0.11.0 NOT FIRED.
- New envelope pin: `620d2225`.
