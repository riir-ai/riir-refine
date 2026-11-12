# Research 400 — AI borrow-checker appeasement patterns (r/rust case-study transcription)

**Status:** DISTILLED-batch-landed — Batch 356 landed 2026-11-12 (plan 366, `entries_72`: 4 rules + 1 face + R4 dropped at authoring; fixtures 16/16; ParetoDominant both benchable; retrieval floors held 29/0). Numbering note: 399/365 + nominal Batch 355 were INDEPENDENTLY allocated by the landed sglang unified-memory verdict (`b706f9ed3`) while this note was in flight — renumbered 400/366/356 per first-allocation; theirs untouched, on-disk duplicates resolved before commit.

## Source

- r/rust discussion: "We designed Rust to fix 40 years of C++ tech debt, and now AI is
  inventing 'borrow-checker appeasement debt' (a 12,349-line case study)". Quoted in full
  in conversation 2026-11-12. **No repo URL** — the author offered "can provide git if
  wanted"; not provided. Post-only source (the forge-blog precedent): the quoted post is
  the pin; there is no clone target.
- Class: audit narrative + perf case study. A Claude-ported TypeScript→Rust deterministic
  RTS replay engine: **3,091/3,091 bit-exact FNV-1a checkpoint hashes over 30,000+ ticks**
  while carrying, at a scale humans don't reach:
  1. 19 OOP classes flattened into 19 `impl Game` blocks on ONE 12,349-line file;
  2. 175 clone/collect sites whose only purpose is dropping a borrow before a mutating loop;
  3. `std::mem::take` check-out/check-in windows (the field is secretly empty during the
     nested calls);
  4. full pixel buffers cloned into an `Arc` per frame (5.9 GB heap per 900-frame video);
  5. a 4-thread `scope` spawned per frame inside `rgba_to_i420` (3,840 OS threads per video).
- The author's refactor (3-crate split, disjoint sub-structs, flat slot arrays, a 5-buffer
  recycled pool) measured 1.90× sim / 2.6–3.0× draw / 8.5 s→0.77 s incremental check, all
  hashes still matching.

## The question this verdict answers (owner, 2026-11-12)

"i think riir-refine should catch this? or rust-optimize / rust-developer catch this?
better catch if not yet."

**Coverage verdict — partial generic coverage, zero specific-tell coverage:**

| Anti-pattern | riir-refine rust_perf (316 live) | clippy_lints | rust-optimize | rust-developer |
|---|---|---|---|---|
| mem::take checkout window | ✗ (`mem-take` owns the OPPOSITE polarity) | ✗ | ✗ | ✗ → **footgun bullet landed with this note** |
| clone/collect before mutating loop | adjacent only (`avoid-clone`, `collect-avoidance`) | ✗ (nightly `redundant_clone` passes most — the value IS used) | generic hot-loop alloc | generic hot-loop law |
| god-file / N impl blocks | correctly out of scope (ambient) | ✗ | ✗ | ✓ file-size rule (≤3200 abs) |
| per-frame big-buffer Arc clone | remedy owned (`bounded-batch-buffer-pool`); clone-into-Arc axis adjacent | ✗ | ✗ | ✗ → **footgun bullet** |
| spawn-threads-per-frame | ✗ zero owner | ✗ | partial (rayon overhead notes) | partial (rayon discipline) |
| dense-id HashMap → slot array | adjacent family, carve unowned | ✗ | ✗ | ✗ → **footgun bullet** |

Closure: the corpus batch (plan 365, scan-time detection) + the two skill additions
(write/review-time detection), landed with this note: rust-developer §7 footgun bullet
(seal-game-editor — the global symlink's canonical home), rust-optimize checklist item 13
(katgpt-rs).

## Why this is corpus-worthy (the decision content)

The post's own diagnosis, adopted: **"known micro-tricks producing a brand-new macro
anti-pattern."** Individually each clone type-checks and usually survives clippy's nightly
`redundant_clone` because the cloned value IS used afterward. The signal is STRUCTURAL
POSITION — relative to mutation, ownership windows, per-call resource construction — which
is corpus-rule territory, not single-lint territory. `cargo refine` is the right detector:
the scan sees the whole file's shape, and `--hotpath-json` (profile-guided) can rank the
sites by measured time share (the post's own 31 s/video profile would rank the pixel-clone
and tick-loop clone sites first).

## Provenance caveats

- Single-box anecdote: 1.90× / 2.6–3.0× / 8.5 s→0.77 s are the author's measurements on
  their box. NEVER citable as magnitude evidence; our fixtures measure their own numbers.
- ZERO code quotes byte-verified — there is no repo. Decision-content-only transcription
  (the penthera law): the post contributes PATTERN SHAPES + regime conditions; every
  fixture is an original Rust reproduction.

## Dedup evidence (2026-11-12, develop `7e56acf33`, rust_perf 316 live rules)

| grep (both vocabularies) | result |
|---|---|
| `mem::take\|mem_take\|checkout` over riir-refine/src | ZERO corpus hits (only KAT payment-lane prose) |
| rule names matching clone/take/spawn/thread/pool/arc/dense/slot/hash/borrow/frame/pixel | 38 hits, all read; load-bearing rulings below |
| `mem-take` (entries_02) | owns `buf.clone() + buf.clear() → mem::take` — take is the REMEDY. Opposite polarity; the checkout-window HAZARD is unowned. In-rule adjudication mandatory. |
| `avoid-clone` (entries_02) | owns `fn(data: Vec<T>) → fn(data: &[T])` (by-value signature). NOT the self-aliasing position. |
| `collect-avoidance` (entries_01) | owns chained iterators vs a collect intermediate. NOT the borrow-appeasement position. |
| `bounded-batch-buffer-pool` (entries_06, measured ParetoDominant 2/0/1, allocs 48000→4) | OWNS the hot alloc/free recycle pool — the post's 5-buffer pool remedy is its positive instance. Residual unowned axis: the CROSS-THREAD return ring. |
| thread-spawn | zero owner (`pre-exec-disables-posix-spawn` = process spawn; entries_56 = timer staleness; `online-cost-fit-thread-ladder-reopen-hysteresis` = adaptive ladder). |
| dense/table family | `fxhashmap` (keep hashing, cheaper), `minimal-perfect-hash-static-lookup` (frozen; its own anti-regime: "runtime-MUTABLE keysets → a real hash map"), `per-scope-map-scan-hash-crossover`, `dense-table-bulk-update-via-contiguous-iter`, `flat-pool-start-len-static-rows` — the mutable dense-id slot-array carve is unowned. |
| clippy_lints corpus | zero clone/collect/borrow rule ids. |

## Ranked candidates

| # | rule_id (proposed) | grade | bench | notes |
|---|---|---|---|---|
| R1 | `mem-take-field-checkout-empty-window` | B | NotBenchable-by-domain (hazard window — the dial9/136 lane) | adjudicate vs `mem-take`: same primitive, OPPOSITE polarity |
| R2 | `clone-before-mutating-loop-borrow-appeasement` | B− | NotBenchable (the win is the surrounding restructure — architectural) | adjudicate vs `avoid-clone` + `collect-avoidance` |
| R3 | `thread-spawn-per-call-pool-reuse` | **B+ LEAD** | Benchable (spawn/join per call vs built-once pool; timing + exact alloc axes) | dedup clean |
| R4 | `cross-thread-buffer-return-ring` | B− | Benchable (alloc axis); expected-drop allowed | MANDATORY adjudication vs `bounded-batch-buffer-pool` + `slow-consumer-latest-value-single-slot` |
| R5 | `dense-id-domain-hashmap-to-slot-array` | B− | Benchable (hash-vs-index) | the mutable-keyset carve `minimal-perfect-hash`'s anti-regime excludes; adjudicate vs `fxhashmap` |

KILL (recorded): god-file / N-impl-block flattening — ambient architecture, no bounded
fix-space (the decision-content test's first kill class). Routes to the skill layer, where
the file-size rule already owns it.

## GOAT check

R3 passes all four: benchable on rust_perf_bench; exact semantics (same work both arms);
magnitude mechanism (thread spawn+join ≈ 10–60 µs per thread per call vs amortized ≈ 0
for a built-once pool); deterministic. R4/R5 benchable via the alloc axis. R1/R2 fail
criterion 1 for a DOMAIN reason (hazard/architecture classes, no Pareto axis) →
suggestion-only NotBenchable annotations per B136/258/291, graded on decision content ×
novelty × dedup × yield.

## Lane-intel axis

LOW — no open workspace lane maps to these shapes: the workspace's own hot loops already
follow the rust-developer hot-loop law, and the seal-remake render path already recycles
buffers. No sibling filing.

## Product-coverage axis

YES — the family is new and its consumers are every AI-assisted Rust repo (game engines,
media pipelines — the post's own example is a Discord MP4 replay renderer). Follow-up pool
(accumulation clause): nightly `redundant_clone`, clippy `needless_collect`, the
borrow-checker-hell human threads the post cites ("Idiomatic approach to borrow checker
hell"), future AI-port audit posts.

## Yield estimate

3–4 rules after ~50% coordinator falsification (R1 near-certain; R3 the bench lead; one of
R4/R5 likely drops on the family adjudication — the honest drop is recorded, never silent).

## Queue + plan

Queue row + `.plans/366_batch356_ai_appeasement_rust_perf_distill.md` (Batch 356 STAGED
unclaimed) landed in the same commit as this note. Skill additions landed in the same
window: rust-developer §7 (seal-game-editor — the symlink's canonical home), rust-optimize
checklist item 13 (katgpt-rs).

Session: amo (user distill lane), epoch 1794487465
