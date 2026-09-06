# Plan 098: P11 intake proposer fixes — the nrl deref splice, fresh-binding uniquing, capacity nested-block descent

Status: RECORD — all tasks complete 2026-09-06; implementation `766da16`, records in Bench 075
**Numbering note (the 848 class, live):** authored as Plan 097 from a pre-push highwater read while a sibling session concurrently allocated Plan 097 (Batch 118 / Research 142, pushed first). Push-wins tie-breaker: the sibling keeps 097; this plan renumbered → 098 before pushing. Commit messages citing "plan 097 T1-T5" refer to THIS plan.

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

- [x] **T1** — nrl universal deref splice: `IndexUsage.paren: BTreeSet<(usize,usize)>` (postfix-parent slots, marked in `visit_expr_method_call`/`visit_expr_field`/`visit_expr_call`); slot substitution `*{name}` / `(*{name})`; place-write guard extended to Field/Index chains (`place_chain_contains_slot`); splice-text test expectations updated; new fixtures: `comparison_slot_splices_deref`, `typed_let_parenless_postfix_parenthesized`, `slot_field_write_refuses`.
- [x] **T2** — nrl uniquing: `fresh_binding_name` (family taken-set: paths, pattern bindings, macro token idents, format-family templates per-candidate; 64 bound); `nested_range_loop_uniques_fresh_binding` (the former refused shape heals at pass 2), `fresh_prefix_collision_uniques_fresh_name`, `template_implicit_capture_bumps_name`, `nonfamily_prefix_mention_no_longer_blocks` (the two old refuse-pins re-pinned as conversion pins).
- [x] **T3** — vec-with-capacity: shared `stmt_nested_blocks` (moved to `proposers/mod.rs`), block recursion (shape A), `find_loop_capacity` descent (shape B), `collect_nested_locals` scope-gate extension; fixtures: `nested_block_local_and_loop_get_capacity`, `outer_local_nested_loop_gets_capacity`, `nested_param_shadow_declines_capacity`. Probe note: bare-path iterables were never a vec capacity source — a second stacked cause for the measured silence.
- [x] **T4** — hashmap-with-capacity: `find_first_insert_loop` (first-mention descent, per-level gates, `bound_locals` across levels for the use-before-decl gate) + `stmt_local_bind_name`; fixtures: `nested_first_mention_loop_fires`, `same_nested_block_map_and_loop_fires` (the pre-existing shape-A behavior, now pinned), `nested_shadow_outer_stops_nested_fires`, `nested_non_loop_first_mention_breaks`, `root_bound_across_levels_still_refuses`.
- [x] **T5** — gen-7 roster moves + EXPECTED_LINTS_V7 + FROZEN_LABELS_V7; freeze e2e re-run re-landed: **16/16 verified, 0 exclusions, 0 relabels** (2.38 s, release) — the printed paste-ready record byte-identical to the landed table. Chain-truth + refire gates green (the two deliberate reds during implementation were the ratchet firing).
- [x] **T6** — validation: lib default **1142/0**; lib score_bench **1341/0/8i** after T5; integration (goat_gate_004, latent_suggest_goat, stage2_goat) **5/0**; gen-7 bench re-run **87.5%** deterministic ×2 (was 78.26%; chain 100%, parse 100%, created unmoved 12.5%); v1 bench **96.2/1.9/100/100** (E4 holds); clippy `-D warnings` both states: zero findings in touched files (3 pre-existing untouched: `rustc_errors/composite.rs:299`, `score_bench/generations.rs:823`). Hand-fixed en route: the doc-markdown backtick lesson (14th strike) + one question_mark let-else on my own lines.
- [x] **T7** — records: `.benchmarks/075_p11_proposer_fixes.md`; snapshot P11 → DONE + P12 note (the `&mut`-receiver / assert_eq! / transitive typed-let classes); this plan RECORD.
- [x] **T8** — commit + push (plan `95021f5` → implementation `ef2129a` → docs; only-own-hunks staged; staged-set verified).

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
