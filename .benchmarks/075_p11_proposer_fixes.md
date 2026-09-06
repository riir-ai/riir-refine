# Bench 075 — P11 intake proposer fixes: the nrl deref splice, fresh-binding uniquing, capacity nested-block descent

**Status:** RECORD — all three P11 classes closed; Plan 097 executed; the moving series re-measured
**Date:** 2026-09-06 · **Box:** 4090/shikuwa (Windows, quiet) · **Unit:** Protocol C post-mining sweep (queue item 14)

## The classes (from `.distill/001_mining_queue_snapshot.md` §P11, filed by Bench 074)

1. `needless_range_loop` emitted NON-COMPILING output on comparison-context slots (`if __elem > 2` E0308).
2. The `__elem` collision guard made nested `needless_range_loop` a hard persist (level 2 refuses after level 1 introduces `__elem`).
3. The capacity-hint nested-block blind spot: `vec-with-capacity`/`hashmap-with-capacity` silent when the collection sits inside an `if let`/match-arm block.

## T0 probes (pre-registration; temp crate `target/probe_p11_clippy`, clippy 1.98.1 all+pedantic+nursery JSON)

**Bare `e: &u32` binding fails E0308 in THREE classes, not one:**

| context | bare `e` | `(*e)` | parenless `*e` |
|---|---|---|---|
| comparison operand (`e > 2`) | E0308 | ok | ok, no lint |
| typed let (`let x: u32 = e`) | E0308 | ok (**rustc `unused_parens`**) | ok, no lint |
| value-arg method receiver (`e.max(3)`) | **E0308** (`&u32: Ord` resolves on the reference; the value arg mismatches) | ok, no lint | — |
| `+=` RHS (`n += e`) | ok (`AddAssign<&u32>`) | ok (**`unused_parens`**) | ok, no lint |
| arithmetic operand (`* 2`) | ok (ref impls) | ok | ok, no lint |
| format arg (`println!("{}", e)`) | ok | ok | ok, no lint; **`uninlined_format_args` does NOT fire on `*__elem`** (Unary ≠ bare ident) |

Design consequences, both probe-forced: (a) the splice is **universal** `*__elem` — desugar-identical to `v[i]`
(`*Index::index(&v, i)`) in every rvalue context, so it is correct wherever the original compiled — not
context-marked (the measured failure set is comparison + typed-let + value-arg receivers and is not
exhaustively enumerable); (b) **parenless everywhere except postfix bases** (method receiver / field base /
call func take `(*__elem)`) — bare parens trigger rustc `unused_parens` at assigned-value positions.

**Capacity probe** (`cargo-heal --fix` dry run at `c02e8357`): vec shape A (local+loop both nested) SILENT,
vec shape B (local fn-level, loop nested) SILENT, hashmap shape A **FIRING** (the `visit_block` visitor
already descends — the intake's "both silent" was the other shape), hashmap shape B SILENT (first-mention
gate broke on the block-bearing stmt). Probe-fixture note: the vec shapes as first written used a bare-path
iterable (`for it in items`), which vec's capacity source never accepted (hashmap does; vec requires
`&items`/`items.iter()`/ranges) — two stacked causes for vec's silence, both recorded here.

## Fixes (Plan 097 T1–T4)

- **T1** `needless_range_loop`: universal `*__elem` splice, `(*__elem)` only at postfix bases (`IndexUsage.paren`
  marked in `visit_expr_method_call`/`visit_expr_field`/`visit_expr_call`); the place-write guard extended to
  field/index place CHAINS (`v[i].field = x` now refuses — the splice is an rvalue).
- **T2** `needless_range_loop`: `fresh_binding_name` — taken-set enumeration over the family
  `__elem | __elem<digits>` (path idents, pattern bindings, macro token idents, format-family template
  literals for implicit-capture safety, per-candidate) + a 64-suffix bound; nonfamily prefix mentions
  (`__elem_x`) no longer block (distinct identifiers — the old guard was over-broad).
- **T3** `vec_with_capacity`: block recursion (shape A) + `find_loop_capacity` descent with per-level
  shadow-break (shape B, first-hit in document order); nested-block locals join the scope gate (also
  declines a capacity source naming a param shadowed inside the descended block).
- **T4** `hashmap_with_capacity`: `find_first_insert_loop` — the first-mention walk descends block-bearing
  statements with the same per-level gates; `bound_locals` collected across levels drives the
  use-before-decl gate (the bomber_17 E0425 class, one level deeper).
- Shared: `stmt_nested_blocks`/`expr_nested_blocks` moved to `rust_perf/proposers/mod.rs` (DRY, one copy).

## Series movement (the designed ratchet)

**Gen-7 re-run** (16 specs after the roster moves; freeze re-landed first — see below):

| metric | Bench 074 (admitted) | this run | |
|---|---|---|---|
| heal_rate | 78.26% | **87.5%** (deterministic ×2) | +9.2pp |
| chain territory | 86.7% | **100%** (15/15) | the collision persist now solves |
| flat territory | 100% | **100%** (5/5) | created-chain dissolved, still solves |
| parse_safety | 93.3% | **100%** | the splice output compiles |
| created_rate | 12.5% (3 lints / 2 fixtures) | **12.5%** (same 3) | unchanged — intake item 4, context-not-fix |
| capped | 33.3% | 33.3% | MAX_FIX_PASSES untouched |

Rows appended to `.benchmarks/score_history.jsonl` (gen-7 runs 4–6: admission runs + the two
determinism re-runs) + `score_history_v7.svg`.

**Roster moves (T5):** `gen7-ref-nrl-nrl` → `gen7-d2-nrl-nrl` (the persist became a real 2-pass chain);
`gen7-d2-nrl-format` → `gen7-flat-nrl-format` (the created-chain subtype DISSOLVED by T1 — the fixture is
retained as its regression pin); NEW `gen7-ref-nrl-write` (inner loop writes through its slot — the
place-write refusal is uniquing-independent, keeping the refused territory honest). **Freeze e2e re-run:
16/16 verified, 0 exclusions, 0 relabels** — the re-landed `FROZEN_LABELS_V7` is byte-identical to the
printed paste-ready record.

**V1 gate:** 96.2% heal / 1.9% created / 100% parse / 100% decline — the E4 floor HOLDS exactly
(the splice creates no new lint classes, probe-predicted).

## Validation

- lib default **1142/0** (30 nrl incl. 6 new fixtures; 33 vec incl. 3 new; 29 hashmap incl. 5 new)
- lib `score_bench` **1341+2→0** (the two red gen-7 gates were the ratchet firing; green after T5)
- integration: goat_gate_004 + latent_suggest_goat + stage2_goat (latent_retrieval) **5/0**
- freeze e2e `--release --ignored`: **16/16, 0 exclusions, 0 relabels**, 2.38 s
- clippy `-D warnings` lib + lib+score_bench: zero findings in touched files (3 pre-existing findings
  untouched: `rustc_errors/composite.rs:299`, `score_bench/generations.rs:823` — landed by earlier units)

## Documented-not-fixed (P12 intake note)

Derived during T0, undecidable parse-only or out of the measured classes — the `--verify` compile gate
auto-reverts them in production:

- `&mut`-method receivers through a slot on a SHARED iteration (`v[i].push(x)` with owned `v` — E0596);
  the `&mut`-param bare-iter form is fine (`__elem: &mut T`).
- `assert_eq!(v[i], 2)` slots — the macro takes references; the expansion type-mismatches.
- `let x = v[i]` transitive typed uses (`let x = v[i]; if x > 2` — the comparison is on `x`, invisible).

## Records

Plan 097 (RECORD) · snapshot P11 → DONE with hashes · this file. Commits: implementation, gen-7 series,
docs — see `git log --grep 097`.
