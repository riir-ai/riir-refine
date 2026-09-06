# Plan 097: P11 intake proposer fixes — the nrl deref splice, fresh-binding uniquing, capacity nested-block descent

Status: IN PROGRESS — probes measured 2026-09-06 (T0 below, results baked into the design); implementation next

**Source:** `.distill/001_mining_queue_snapshot.md` §Post-mining intake P11 (filed by Bench 074 / Plan 096 gen-7): three measured classes, all pre-existing, none gen-7-caused. Protocol C (post-mining sweep) unit — probe first, fix the proposer at root cause, fixture regression test, re-run the moving series.

## The three classes (intake wording)

1. The `needless_range_loop` rewrite produces NON-COMPILING output when a substituted slot sits in a comparison context (`if __elem > 2` E0308 — the fresh `&u32` binding vs an integer literal needs a deref). Fix-space: "a context-aware slot guard or a `*__elem` splice for value-context slots".
2. The `__elem` collision guard makes NESTED needless_range_loop a hard persist (level 2 refuses after level 1 introduces the binding). Fix-space: fresh-binding uniquing (`__elem2`).
3. The capacity-hint nested-block blind spot: `vec-with-capacity` and `hashmap-with-capacity` are SILENT when the collection sits inside an `if let`/match-arm block (measured edits=0 at the gen-7 probe).

Intake item (4) — the cascade EXPOSING deeper non-vocabulary lints — is context, not a fix: not this unit.

## T0 — probes (DONE 2026-09-06, before this plan's implementation; results are the design's ground truth)

- [x] **Clippy oracle probe** (`target/probe_p11_clippy`, clippy 1.98.1, all+pedantic+nursery, JSON):
  - Bare binding `e: &u32` fails E0308 in THREE classes, not one: comparison operand (`if e > 2`), typed let (`let x: u32 = e`), AND method receiver with value args (`e.max(3)` — `&u32: Ord` resolves on the reference and the value-arg mismatches). `n += e` compiles (`AddAssign<&u32>`).
  - The deref splice compiles in EVERY position tested; `(*e)` parenthesized forms trigger rustc `unused_parens` where parens are redundant (typed-let RHS, `+=` RHS) — so the splice must be PARENLESS where `*` binds tightly enough.
  - Parenless forms (`if *e > 2`, `let x: u32 = *e`, `n += *e`, `check(*e)`, `*e * 2`): compile, ZERO clippy lints (no `explicit_auto_deref`, no `unused_parens`).
  - Postfix receiver keeps required parens: `(*e).max(3)` compiles, zero lints.
  - The planned healed atom shapes (d2-nrl-semi deref form, nested `__elem2` form, `println!("{}", *__elem)`) all compile; `format!("{}", *__elem)` does NOT fire `uninlined_format_args` (the arg is a Unary deref, not a bare ident) — **the gen-7 created-chain subtype (`gen7-d2-nrl-format`) is dissolved by this fix**.
- [x] **Capacity probe** (`target/probe_p11_caps`, global `cargo-heal --fix` dry run, HEAD `c02e8357`):
  - vec shape A (local+loop both inside the if-block): SILENT — `collect_block_edits` scans only fn-body top-level stmts.
  - vec shape B (local at fn level, loop nested): SILENT — the later-scan never descends.
  - hashmap shape A (both nested): **FIRES** (the `visit_block` visitor already descends) — the intake's "both silent" was measured on the other shape.
  - hashmap shape B (local at fn level, loop nested): SILENT — the first-mention gate `break`s on the block-bearing stmt.
  - Controls (top-level): both fire ✓.

## Design decisions (from the probes)

- **T1 — universal `*__elem` splice.** `*__elem` is desugar-identical to `v[i]` (`*Index::index(&v, i)`) in EVERY rvalue context — the root-cause-correct form, strictly better than context-marking (whack-a-mole: the measured failure set is comparison + typed-let + value-arg receivers and grows). Parens `(*__elem)` ONLY where the slot is the direct base of a postfix expr (method receiver, field base, call func) — everywhere else parenless `*__elem` (avoids `unused_parens`, measured).
- **T2 — fresh-binding uniquing.** Replace the binary `body_mentions_prefix` refuse with a taken-set enumeration over the family `__elem` | `__elem<digits>` (path idents + macro token idents + format-family template literals — the implicit-capture shadow hole); pick the smallest free index ≥ 1. Nonfamily prefix mentions (`__elem_x`) no longer block (distinct identifiers — the old guard was over-broad). Bound the search at 64 candidates then refuse.
- **T3 — vec nested-block descent (shapes A + B).** `collect_block_edits` recurses into non-Item, non-closure nested blocks (shape A); the later-scan becomes `find_loop_capacity` which descends block-bearing stmts per level with the same break-on-shadow semantics (shape B). First-hit semantics preserved (the first capacity-bearing loop wins, even if `cap_replacement` declines).
- **T4 — hashmap first-mention descent (shape B).** When the first same-block mention of VAR is a block-bearing stmt, descend into its nested blocks with the same first-mention gates per level (bind-break, mention-must-be-loop, insert-body, len-expr, root use-before-decl checked per level, opaque-init gate against the level where the root is bound).
- **T5 — gen-7 series updates (the designed ratchet movement).**
  - `gen7-ref-nrl-nrl` → `gen7-d2-nrl-nrl` (uniquing turns the refused persist into a solved 2-pass chain; the classifier keys on `-d2-`).
  - `gen7-d2-nrl-format` → `gen7-flat-nrl-format` (the created-chain subtype dissolves — the deref splice's Unary arg is not a bare ident; expected passes 2 → 1; the fixture is retained as the regression PIN for the dissolved subtype).
  - NEW `gen7-ref-nrl-write` (nested nrl whose inner loop writes through the slot `inner[j] += 1` — the place-write guard refuses regardless of uniquing; keeps the refused territory populated with a real guard refusal).
  - EXPECTED_LINTS_V7 + FROZEN_LABELS_V7 updated; freeze e2e re-run re-lands the frozen record (16 specs).
- **Pre-registered prediction (falsifiable):** the gen-7 re-run measures the refused fixture at 0-solve/1-pass, `gen7-d2-nrl-nrl` at solve/2-passes, chain territory 10/10, overall rate ABOVE the admitted 78.26%; the v1 score-bench E4 gate holds at 96.2/≤1.9 (the splice creates no new lint classes — probe-measured).

## Tasks

- [ ] **T1** — nrl universal deref splice: `IndexUsage` gains `paren: BTreeSet<(usize,usize)>` (postfix-parent slots, marked in `visit_expr_method_call`/`visit_expr_field`/`visit_expr_call`); slot substitution `*{name}` / `(*{name})`; extend the place-write guard to Field/Index chains (`v[i].field = x` blocks — `place_chain_contains_slot`); update the ~10 splice-text test expectations; new fixtures: comparison slot, typed let, postfix receiver paren, place-field-write refusal, universal form compile-ready re-parse.
- [ ] **T2** — nrl uniquing: `fresh_binding_name(body) -> String` (taken-set enumeration + template scan + 64 bound); nested-nrl converts at pass 2 (`__elem2`); fixtures: nested-nrl converts, prefix-local uniques to `__elem2`, nonfamily mention converts, template implicit-capture bumps the name; update `fresh_prefix_collision_skips` / `fresh_prefix_collision_inside_macro_refuses` (both now convert — the tests pinned the old conservative behavior).
- [ ] **T3** — vec-with-capacity: `stmt_nested_blocks` + block recursion in `collect_block_edits` (skip Item + closures; root block stays the collision domain for `temp_binding_edits`); `find_loop_capacity` descent for shape B; fixtures: shape A (both nested), shape B (local outer / loop nested), shadow-bind inside the nested block stops the scan.
- [ ] **T4** — hashmap-with-capacity: first-mention descent into block-bearing stmts with per-level gates; fixtures: shape B fires, nested shadow breaks, nested first-mention-not-a-loop breaks.
- [ ] **T5** — gen-7 fixture/table updates (renames, new ref fixture, EXPECTED_LINTS_V7) + freeze e2e re-run re-lands FROZEN_LABELS_V7 (16 specs, 0 exclusions) + chain-truth gate green.
- [ ] **T6** — validation: `cargo test --lib` default + `--features score_bench`; gen-7 bench re-run (`--score-bench --generation 7`, deterministic ×2 — new rows + SVG v7); v1 score-bench (E4 gate 96.2/≤1.9 holds); clippy zero findings on touched lines; cargo heal on own touched files.
- [ ] **T7** — records: `.benchmarks/075_p11_proposer_fixes.md` (probe tables + measured movement); snapshot P11 → DONE with hashes + a P12 note for the derived classes this unit deliberately leaves (documented-not-fixed): `&mut`-method receivers through slots (undecidable parse-only — verify-reverted), `assert_eq!(v[i], 2)` slots (macro-reference expansion), `let x = v[i]` transitive typed uses; plan marked RECORD.
- [ ] **T8** — commit + push (plan → implementation → gen-7 series → docs; `feat:`/`fix:`/`docs:` prefixes, develop, only-own-hunks staged).

## Non-goals

- The cascade-exposure finding (intake item 4) — context, not a fix.
- `&mut`-method receivers, `assert_eq!` slots, transitive typed-let uses — derived during T0, undecidable parse-only or out of the measured classes; recorded as P12, `--verify` backstops them.
- Gen-2/3/6 series re-freezes — their frozen labels are PRE-heal clippy labels, unchanged by emitter output; the gen-7 set is the only one whose fixture SPEC changes.
- The `l4_lora`/backend lanes (Issues 073/074) — sibling-owned; 073's T3 run is box/backend-held, 074 is closeout-ready with a consumer-ask-gated T4.

## References

- `.distill/001_mining_queue_snapshot.md` §Post-mining intake P11
- Bench 074 / Plan 096 (the gen-7 ladder + probe record)
- `src/domains/clippy_lints/proposers/needless_range_loop.rs` (the splice + collision guard)
- `src/domains/rust_perf/proposers/{vec,hashmap}_with_capacity.rs` (the block scans)
- `src/score_bench/generations.rs` (V7 tables + chain-truth gate)
- `tests/generation_v7_freeze.rs` (the freeze e2e)
