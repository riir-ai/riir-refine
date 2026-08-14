# Bench 019 — Plan 005 Phase 6 T6.1: span-preserving `--fix` GOAT (G1-G4)

**Date:** 2026-08-14
**Scope:** the `cargo heal --fix` wiring — `SourceEdit` splice emitter +
CLI `--fix [--write] [--quiet]` + the 5 rust_perf proposers refactored from
AST-mutation to edit-collecting.
**G5 (real fix in a sibling repo):** ~~BLOCKED~~ — **PASS 2026-08-14, this
bench's G5 addendum below.** The original target (riir-ai) stayed blocked
(sibling session actively working: riir-net webtransport/hub edits, dirty
qvu_lora/cudarc WIP); the gate landed in **katgpt-rs** (`katgpt-pruners`),
which is a sibling repo per this repo's layout (and the public one — the
fix is ordinary Rust, no IP concern). Recorded as T6.2.

## The emitter

`proc-macro2` `span-locations` (1.0.107 has `Span::byte_range()`; verified:
`syn::parse_file` in a normal lib populates real byte offsets — pinned by
the `span_text_round_trips` test). Each proposer walks the parsed file
**immutably** and emits `SourceEdit { start, end, replacement }` where the
replacement text is built from the **original source text of the matched
nodes** (`src(source, node)`) plus short literals — the edited regions keep
the caller's own formatting for every sub-expression they contained.

`apply_edits`: selection (sort asc-by-start, outer-first on ties; drop any
edit starting before the running end — overlaps/containments), then apply
**back-to-front** so earlier byte offsets stay valid as replacements change
lengths. (The one-pass asc-order version corrupted offsets — caught by the
fxhashmap corpus fixture; the two-phase fix is load-bearing.)

No `prettyplease`, no whole-file token re-emission, no reformat.

## Gates

| Gate | Target | Measured | Verdict |
|---|---|---|---|
| **G1 correctness** | fixture round-trips: detect → rewrite → output parses + carries the fixed-form marker | 147 lib tests (all 5 proposers' fixtures + the corpus end-to-end case; L2 pruner parse-check on every output) | **PASS** |
| **G1' span preservation** | everything outside matched spans byte-identical | `output_preserves_untouched_bytes`: comments, blank structure, untouched stmts survive verbatim; exactly 1 line differs on the vec fixture. Live: `--fix --write` on a real 700-line file (`src/adaptive_drafter.rs`) changed exactly 1 line (`Vec::new()` → `Vec::with_capacity(4)` at L197) | **PASS** |
| **G2 perf** | proposer pass cheap per file | parse + 5 proposer scans over 46 repo files, ~0.4s total incl. index build + suggest machinery (unchanged from Bench 018 regime) | **PASS** |
| **G3 no-regression** | existing suite green | 147 lib tests (141 pre-existing + 6 new emitter/splice tests), clippy --all-targets clean, `--features rust_perf_verify` check clean | **PASS** |
| **G4 alloc-free hot path** | — | N/A at this layer: the proposers allocate only for the `Vec<SourceEdit>` outputs (bounded by matches) — the suggest hot path is unchanged (Bench 018 §G4 stands). Honest note, not a claimed gate | n/a |

## Real-world smoke (review mode, no writes)

`cargo heal --fix --quiet src/` on this repo: 4 files / 12 candidate edits.
Inspected: `adaptive_drafter.rs:197` (`Vec::new()` + `0..4` push loop) —
**true positive**, textbook `with_capacity(4)`. The `fxhashmap` hits on
`draft/corpus_index.rs` etc. are syntactically true but **require the
`rustc-hash` dep** — exactly the documented caveat (parse-only validation
cannot check deps; review mode is the safety net).

## Honest limits

- ~~`--fix` covers the 5 rust_perf proposers only (T6.1 scope)~~ — superseded
  by the T6.4 addendum below.
- Kernel: no proposers (Phase 5 deferred — concept-grade corpus, see Plan
  005 §Phase 5).
- `--write` trusts the proposers' visitor gates; signature-visible rewrites
  (`static-str`, `bool-to-bitpack`) ripple to callers — `rust_perf_verify`
  (cargo test) is the semantic gate, run by the consumer.

## T6.4 addendum (2026-08-14, commit `b83230e` + same-day polish)

`--fix` extended from the 5 rust_perf proposers to **5 + 7** — the 10
clippy_lints proposers migrated to this emitter (see
[Plan 005](../.plans/005_real_file_span_healing.md) §Phase 6 T6.4 for the
four latent detection bugs the migration fixed). The G1/G1'/G3 regime is
unchanged: proposer fixtures + L2 pruner parse-check on every output,
span preservation by construction, clippy clean. Same-day polish:
review line numbers now reference the FINAL written source (positions
remapped through every pass's applied splices via `apply_edits_traced` +
`map_through_splices`) + pass-1 overlap-dropped edits re-applied on pass 2
are no longer double-counted. Verified live on a nested fixture
(`return Cfg { x: x };` → `Cfg { x }` across 2 passes + a manual_map
collapse): all reported lines correct in the final file, output
type-checks clean under rustc.

## G5 addendum (2026-08-14, T6.2/T6.3 — the real-file gate)

**Target:** `katgpt-rs` `katgpt-pruners` (clean tree, no sibling activity;
riir-ai remained sibling-active with dirty WIP). Scan: `--fix --quiet`
review over `katgpt-rs/crates/` — **1445 files, 104 files / 498 candidate
edits** (422 fxhashmap + 62 vec-with-capacity + 14 structural-clippy).

**Selection:** all 422 `fxhashmap` edits were excluded — no katgpt-rs crate
depends on `rustc-hash`, so every rewrite would fail to link (the documented
parse-only-cannot-see-Cargo.toml caveat, now with a measured instance).
One crate (`katgpt-pruners`) was chosen for a surgical, verifiable batch.

**Run 1 (pre-fix proposers): 19 files / 30 edits written.** The compile gate
(17-feature set covering every touched module — a default-features check
is VACUOUS here, `default = []` gates every edited module; the riir-chain
"green, and unverified" lesson applies) caught **6 errors in 4 files**:

| Error | Site | Root cause |
|---|---|---|
| E0425 ×3 | `bfcp_region_cache.rs:377`, `problem_mutator.rs:75`, `reflection.rs:323` | capacity text references a `let` declared AFTER the vec binding (use-before-decl) |
| E0308 ×1 | `monopoly/players.rs:80` | `0..BOARD_SIZE` with a `u8` const — `with_capacity` needs `usize` |
| E0631/E0599 ×2 | `roaring_membership.rs:355` | `\|v\| from_bool_vec(v)` → fn path loses the deref coercion (`&Vec<bool>` → `&[bool]`) |

These are **three latent proposer bug classes the 36 fixtures missed** —
the exact failure mode G5 exists to catch. All three fixed at root cause:

1. **Scope gate** (`vec_with_capacity.rs`): any single-segment path in the
   capacity expression that is let-bound anywhere after the vec binding
   skips the rewrite (shadowing resolved conservatively).
2. **usize gate**: the capacity must be parse-only-provably `usize` — int
   literal, `x.len()`, len-rooted `saturating_/wrapping_` arithmetic with a
   literal arg, or a plain path naming an explicitly-`usize` fn param.
   Plain-path consts/locals/associated consts are skipped (syn cannot know
   their type).
3. **`redundant_closure` demoted from `FIXABLE_LINTS`** (7 → 6 structural
   lints): the fn-path rewrite `\|x\| f(x)` → `f` is type-blind — the
   closure body may rely on a coercion a fn item does not get. The
   proposer stays for `self_evolve` (whose loop verifies with real cargo).

4 new fixture tests pin the gates (later-let path + len forms skipped,
   non-usize param skipped, non-len method bound skipped, len-rooted
   arithmetic kept).

**Run 2 (gated proposers): 12 files / 20 edits, compiled + tested in one
pass.** Landed as katgpt-rs `8c331cd9` (pushed):

- `vec-with-capacity` ×18 — `Vec::new()` + push-loop → `with_capacity`
  (literals 9/10/15/30/50, `x.len()` forms, len-rooted `saturating_sub(1)`,
  usize params). Honest note: in multi-loop fns the capacity is the FIRST
  pushing loop's bound (a conservative lower bound — capacity is advisory,
  never a correctness claim); `sudoku_pruner`'s `with_capacity(9)` is the
  outer-loop bound of a 9×9 scan.
- `needless_range_loop` ×2 — `max_token_share`/`max_format_share` in
  `thicket_variance_probe` iterate by reference (bounds-check removal,
  production code). Loop var is the capture-proof `__elem` default —
  deliberate: a derived name could capture an existing binding.

**Verification:** `cargo check` + `cargo clippy --all-targets` (only a
pre-existing `dead_code` warning on `sudoku_pruner::candidate_set` under
this 17-feature combo — unrelated to the edits) + **1233/1233 lib tests
pass** under the feature set covering every touched module. GPU: none used
(CPU-only pipeline; sibling's concurrent mmorpg test run is CPU-only).

**Verdict: G5 PASS.** The full loop ran end-to-end on a real repo: latent
corpus scan → proposer detection → review selection → apply → compile gate
CAUGHT real bugs → root-cause proposer fixes → clean re-apply → tests pass
→ landed + pushed. The gate did not merely prove the happy path — it
hardened the tool (3 bug classes fixed, 1 lint demoted) before landing.

## Commands

```bash
cargo test -p riir-clippy --lib                    # 147 pass
cargo clippy --all-targets                         # clean
cargo run --features latent_retrieval --bin cargo-heal -- --fix --quiet src/   # review
cargo run --features latent_retrieval --bin cargo-heal -- --fix --quiet --write src/
```
