# Plan 100 — P12 slot-substitution classes: mutator-header, mutable-iter exclusivity guard, template-less family-macro descent

**Status:** RECORD — all tasks complete 2026-09-07; records in Bench 077. T0 falsified the intake's "parse-only undecidable" premise for every class; one NEW broken-output class (C4, `&mut`-param + non-slot body use = E0382) found + fixed beyond the intake. The plan was authored plan-first with the T0 evidence in-line; predictions (gen-7 unchanged 87.5%, v1 E4 96.2/≤1.9) both met exactly.

## Source + grounding

`riir-clippy/.distill/001_mining_queue_snapshot.md` §Post-mining intake P12: three
classes "documented-not-fixed (parse-only undecidable; `--verify` auto-reverts in
production)" derived during Plan 099 T0. **T0 falsified the "undecidable" claim
for every class** — each has a sound parse-only fix-space or is already fixed.

Probe rig: `/tmp/p12_probe` (scratch crate, global `cargo-heal` binary
@ target/release built 09-06 23:38 — AFTER the last proposer change `784b673`
21:45; `propose`-path current), `--fix --write` to a copy + `cargo check` of the
healed output. Zero writes to any shared target dir.

## T0 probe results (2026-09-07, the design's ground truth)

| # | Input | Current output | Compiles? | Verdict |
|---|---|---|---|---|
| C1 | `fn f(mut v: Vec<Vec<u32>>)` + `v[i].push(1)` | `for __elem in &v { (*__elem).push(1); }` | **E0596** | broken output → FIX (mutator header) |
| C1b | `v: &mut Vec<Vec<u32>>` + `v[i].push(1)` | `for __elem in v { (*__elem).push(1); }` | ✓ | bare iter of `&mut` yields `&mut T` — heals today, unchanged |
| C4 | `v: &mut Vec<u32>` + body `s += v[i] + v.len() as u32` | `for __elem in v` + `v.len()` untouched | **E0382 borrow of moved `v`** | broken output (pre-existing, NOT in the intake) → FIX (exclusivity guard) |
| C2 | `assert_eq!(v[i], 2)` | silent refuse (missed heal) | n/a | persist → FIX (template-less family descent) |
| C2b | `assert!(v[i] > 2)` (cond only, no message) | silent refuse | n/a | persist → same fix |
| C3 | `let x = v[i]; if x > 2` | `let x = *__elem;` | ✓ | **already fixed by P11** — the transitive typed use rides the universal splice; Copy-only input universe (non-Copy `let x = v[i]` is E0507 in the ORIGINAL, so clippy never sees it). Pin + close. |
| — | `w[i]` other-collection same-index use | refuse (`total != exact.len()`) | n/a | existing gate already covers the parallel-index E0502 shape |

Candied-form compile proof: `assert_eq!(*__elem, 2)` / `assert_ne!(*__elem, 0)` /
`assert!(*__elem > 2)` all check clean (probe3.rs) — the eq-family expansion takes
`&($left)`, so the deref splice yields `&u32` vs `&{integer}` → unifies.

## Design

### T1 — mutator-receiver header (`&mut v`) + mutable-iter exclusivity guard

- `IndexUsage` gains: `needs_mut: bool` (a slot is the receiver of a known
  `&mut self` method) and `collection_uses: usize` (every body `Expr::Path`
  mention of the collection ident; exact-slot bases each contribute exactly 1,
  so `non_slot = collection_uses - exact.len()`).
- Known-mutator set (bounded, documented non-exhaustive — custom `&mut self`
  methods stay `--verify`-backstopped): `push insert pop remove swap truncate
  clear retain retain_mut sort sort_by sort_by_key sort_unstable sort_unstable_by
  sort_unstable_by_key dedup dedup_by dedup_by_key dedup_by_key_and_value reverse
  resize resize_with extend extend_from_slice append split_off drain splice
  push_str insert_str remove remove_prefix remove_suffix replace_range`.
  Consuming methods (`into_iter`, `into_boxed_slice`, …) deliberately EXCLUDED —
  moving out of a `&mut` deref is E0507 in every header shape.
- `param_is_reference` → `param_iter_form`: `Shared` (`&T`) / `Mut` (`&mut T`,
  unwrapping `Type::Paren`) / `Owned { mut_decl }` (reads `Pat::Ident.mutability`).
- Header selection in `try_rewrite_for`:
  - `Shared` + `needs_mut` → refuse (unreachable input — the original would be
    E0596 — but refuse defensively rather than emit).
  - `Shared` / `Owned { mut_decl: _ }` without `needs_mut` → today's header
    (bare / `&v`).
  - `Owned { mut_decl: true }` + `needs_mut` + `non_slot == 0` → **`&mut v`**.
  - `Owned { mut_decl: false }` + `needs_mut` → refuse (unreachable: original
    E0596; defensive).
  - `Mut` + `non_slot > 0` → **refuse (the C4 fix — today this emits E0382)**.
  - `Mut` + `needs_mut` → bare iter already correct (C1b).
- Place-write refusal, `&v[i]` address-of refusal, complex/other-collection
  index refusals: ALL unchanged (`gen7-ref-nrl-write` territory intact).

### T2 — template-less family-macro descent (the C2/C2b fix)

- In `Scan::scan_macro`: when the macro is in the family table but the template
  position holds no plain string literal (eq-family ALWAYS; assert!/ensure!
  with no message; write!/writeln! never — they always carry a template, and a
  missing one is malformed → today's conservative path), descend
  `all_expr_args` (already exported by `uninlined_format_args`) instead of the
  token-walk refusal, splicing slots in the operand/condition args.
- Preconditions to keep soundness: every parsed arg visits through the SAME
  scan (nested macros recurse via `visit_expr`); unparseable args mentioning
  the idx still refuse.
- The existing template-present path (println/format family; writeln with
  template) is byte-identical — zero churn for the 30+ existing fixtures.

### T3 — C3 regression pin

- One fixture test: `let x = v[i]; if x > 2` heals to `let x = *__elem;` and
  re-parses; a comment pins WHY the input universe is Copy-only (non-Copy
  `let x = v[i]` is E0507 in the original → never a clippy input).
- Snapshot P12 → DONE with the C3-covered-by-P11 note + the C4 amendment.

## Pre-registered predictions (falsifiable)

1. Gen-7 series re-run: **unchanged at 87.5%** (no gen-7 fixture carries a
   mutator receiver or a template-less family macro).
2. v1 score-bench E4: **holds at 96.2 / ≤1.9** (the new header shape appears
   in no v1 fixture; the C4 guard can only flip broken emissions to honest
   refuses — a miss-axis move at most, and only for shapes the corpus lacks).
3. The new fixtures heal to compiling output, verified by `cargo check` on the
   healed scratch crate (the same probe rig, reversed).

## Tasks

- [x] **T0** — probes (DONE 2026-09-07, results above; `/tmp/p12_probe`, binary-freshness verified against `784b673`).
- [x] **T1** — mutator header + exclusivity guard: `IndexUsage.{needs_mut, collection_uses}`, `MUTATOR_METHODS` (37 `&mut self` receivers, consuming methods excluded), `param_iter_form` (Shared/Mut/Owned{mut_decl}, one paren layer unwrapped), header selection arms, C4 guard. Fixtures (14 new, 44 total nrl tests, FIRST-SHOT green): mutator/owned-mut mut-header; shared-ref + non-mut-owned mutator refusals; `&mut`-param non-slot-use refusal (C4 pin); `&mut`-param bare-iter unchanged (C1b pin); owned-mut plain read keeps `&v`; consuming-method exclusion; mut-header + non-slot use refusal.
- [x] **T2** — template-less family descent: `scan_macro` falls back to `all_expr_args` when family-matched but template-less; literal-token capture guard (closes a latent hole — the old walk never looked inside string literals). Fixtures: `assert_eq!`/`debug_assert_eq!`/`assert!`-cond deref splices; hidden-capture refusal; tracing named-target form now HEALS (bonus, formerly conservative-refused).
- [x] **T3** — C3 regression pin (`let_init_transitive_typed_use_splices_deref`) + snapshot P12 → DONE (with the C4 amendment row + the covered-by-P11 note) + this plan → RECORD.
- [x] **T4** — validation: lib default **1156/0** (floor 1142 held); lib+score_bench **1357/0/8i**; integration trio **13/0/2i**; gen-7 ×3 **87.5% deterministic** (prediction met); v1 E4 **96.2/1.9/100** (prediction met); clippy `-D warnings` both states clean (incl. the pre-existing `generations.rs:823` doc-markdown straggler fixed en route); all-targets 0 findings; healer release binary rebuilt in `/tmp/p12_target` and the e2e probe re-run (all four healed shapes compile, both broken shapes refuse).
- [x] **T5** — commit + push on `develop` (named paths only; `fix:` prefix for the proposer code).

## Non-goals

- Unlocking place-writes through the slot (`v[i] = x` under a `&mut` header) —
  `gen7-ref-nrl-write`'s refused territory is a designed pin; changing it is a
  separate unit with its own series movement.
- Extending iterable resolution to LOCALS (`let mut v = vec![...]`) — the
  param-only contract stands (Issue-032 atomicity and the `&`-vs-owned
  resolution both assume the fn-signature source of truth).
- Custom/non-std `&mut self` receiver methods — unlistable; `--verify`
  backstops them (documented in-rule, the intake's residual core).
- The `&mut`-param + `w[j]`-style OTHER-collection use — already refused by the
  `total != exact.len()` gate (probe table, last row).
