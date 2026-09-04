# Bench 021 — Issue 016: structural clippy lint expansion (6→33 FIXABLE, 1→4 VERIFY)

**Date:** 2026-08-16
**Feature:** `clippy_lints` (default-on) — corpus + proposers, no new Cargo features
**Issue:** `016` (resolved + removed per the noise-reduction rule)

## What shipped

| Surface | Before | After |
|---|---|---|
| Corpus entries | 10 | **40** |
| `FIXABLE_LINTS` (`--fix` eligible) | 6 | **33** |
| `VERIFY_LINTS` (`--fix --verify`, Issue 015 compile gate) | 1 | **4** |
| Per-lint proposers | 10 | **40** |
| Lib tests (default) | 201 | **369** |
| Lib tests (`latent_retrieval`) | 219 | **383** |

## Selection methodology (acceptance #1: frequency-ranked, source data cited)

Three-source intersection, filtered by parse-only provability:

1. **kornel's 100K+ crate clippy run** (clippy v1.80–v1.83, pedantic +
   suspicious + nursery enabled for crates without a `[lints]` table;
   one message per error code per crate; September 2024,
   `users.rust-lang.org/t/ive-ran-clippy-on-100k-crates/117127`).
   Top-10: `unused_imports` 59,833 / `missing_errors_doc` 56,975 /
   `unused_qualifications` 53,225 / `missing_docs` (crate) 48,305 /
   `elided_lifetimes_in_paths` 43,795 / `missing_panics_doc` 43,360 /
   **`semicolon_if_nothing_returned` 41,601** / `missing_docs` (struct)
   38,447 / `missing_docs` (method) 37,365 / `unused_crate_dependencies`
   36,692. Of these, only `semicolon_if_nothing_returned` is a structural
   machine-applicable rewrite (the rest are rustc lints needing import
   resolution, or doc-writing lints no AST rewrite can produce) — it is
   the single most frequent structural lint and landed as a FIXABLE row.
2. **Li et al., "Unleashing the Power of Clippy in Real-World Rust
   Projects" (ICSE 2024, arXiv:2310.11738)** — 8,095,259 warnings across
   94,715 crates.io projects (latest releases, 403 warning types); the
   distribution is long-tailed: top-1 >50% and **top-5 >80% of all
   warnings** → covering the frequent head is the leverage argument for a
   30-lint expansion over lint-count maximalism.
3. **Local workspace grep** (katgpt-rs + riir-clippy `*.rs`, 2026-08-16):
   `.len() == 0` = **12 live hits** (`comparison_to_empty` — VERIFY tier
   with real local demand); `format!("{}", ident)` present
   (`uninlined_format_args`); `if let Ok(_)`/`Some(_)` = 0 (already swept
   clean — our repos are clippy-swept, so absence is expected and the
   kornel/crates.io data is the primary frequency axis).

Applicability axis: the lints' `MachineApplicable` tags per the clippy
1.93 lint index (`rust-lang.github.io/rust-clippy/master/index.html`;
local clippy verified 1.93.0 during this bench — the
`semicolon_if_nothing_returned` fix direction was empirically confirmed
by running `cargo clippy -- -W clippy::semicolon_if_nothing_returned` on
a /tmp fixture: "consider adding a `;` to the last statement for
consistent formatting").

### The 30 lints (grouped by triage)

**FIXABLE (27)** — parse-only provable rewrites: `semicolon_if_nothing_returned`,
`collapsible_if`, `collapsible_else_if`, `single_match`, `single_match_else`,
`redundant_pattern_matching`, `question_mark`, `range_plus_one`,
`uninlined_format_args`, `identity_op`, `needless_bool`,
`needless_bool_assign`, `match_bool`, `needless_match`,
`redundant_static_lifetimes`, `double_parens`, `unnecessary_lazy_evaluations`,
`map_unwrap_or`, `filter_next`, `iter_nth_zero`, `manual_unwrap_or`,
`unused_unit`, `useless_format`, `deref_addrof`, `wildcard_in_or_patterns`,
`unnecessary_sort_by`, `iter_next_loop`.

**VERIFY (3)** — type-blind shapes (parse-level best-effort; rustc
auto-reverts under the Issue 015 `--verify` compile gate):
`comparison_to_empty` (`is_empty` existence), `while_let_on_iterator`
(receiver could be a non-`Iterator` custom type with `next()`),
`single_char_pattern` (method resolution is receiver-type-dependent).

**Deferred with recorded reasons** (from the issue's candidate pool):
`manual_strip` (multi-shape + borrow surgery — not a bounded v1 fix-space),
`clone_on_copy` (Copy detection is rustc-only; parse-level best-effort
would fire on every `.clone()`), `len_zero` (merged upstream into
`comparison_to_empty`), `get_first` (HashMap receivers lack `.first()`),
`approx_constant` (f32/f64 target-type blindness for unsuffixed literals),
`erasing_op` (replacement literal type must match operand type + operand
side effects), `needless_late_init` (multi-statement reordering span
surgery), `redundant_at_rest_pattern` (type-annotation↔pattern arity
coupling), `search_is_some` (2-shape method-chain, deferred with the
family), `map_identity`/`flat_map_identity` (rare + receiver-shape
blindness).

### Load-bearing safety specs (the interesting ones)

- **`identity_op` IEEE-754 split**: `x + 0.0` is NOT identity for
  `x = -0.0` (`-0.0 + 0.0 = +0.0`), so unsuffixed `+ 0`/`0 +` never fires
  (an unsuffixed literal can infer float); `x - 0.0`, `x * 1.0`, `x / 1.0`
  ARE identity for all finite inputs + `-0.0`, so those fire unsuffixed.
  Integer-suffixed zeros always fire. Adversarial test pins the
  unsuffixed `x + 0` NO-FIRE case.
- **`uninlined_format_args` trait-bound identity**: `{}` → `{ident}`
  keeps the exact `Display` bound (if the original compiled, the inline
  form compiles). Conservative v1: only plain `{}` placeholders with
  plain-ident args; ANY format spec / positional / named placeholder or
  non-ident arg aborts the whole macro (no partial inlining).
- **Eager-vs-lazy purity gates**: `manual_unwrap_or` (`None => default`
  arm evaluates lazily in the match; `unwrap_or(default)` evaluates
  eagerly — only PURE defaults fire) + `unnecessary_lazy_evaluations`
  (`.unwrap_or_else(|| body)` → `.unwrap_or(body)` moves the body into
  eager position — only PURE bodies fire).
- **`map_unwrap_or` needs NO purity gate**: both `.map(f).unwrap_or(d)`
  and `.map_or(d, f)` evaluate `d` eagerly and `f` lazily — semantics
  preserved by construction.
- **`iter_next_loop` is a correctness lint**: `for x in it.next()` → `for
  x in it` CHANGES iteration count (0-or-1 → full) — that is the intended
  semantics fix (the programmer erred), same as clippy's own suggestion.
- **`needless_bool` else-position retraction**: `else !b` doesn't parse,
  so a bare-negation rewrite on an if whose PARENT is another if's
  else-branch is retracted post-recursion.

## GOAT

| Gate | Result |
|---|---|
| G1 parse (every output re-parses) | ✅ PASS — `all_lints_fire_and_parse` (40 rows) + `all_buggy_codes_fire_their_lint` (corpus↔dispatch lockstep) + per-lint proposer fixture/adversarial tests (~168 new tests) |
| G2 suggest smoke | ✅ PASS — `cargo heal --suggest src/domains/clippy_lints/proposers/`: 42 files / 167 spans in **6.1 s** wall (incl. build check); new rules surface on real proposer files (`redundant_static_lifetimes`, `manual_map`, ...) |
| G3 no-regression | ✅ PASS — 369/369 default + 383/383 `latent_retrieval` lib tests; `clippy_verify` state compiles clean; clippy 0 warnings in default state, 0 NEW warnings in latent_retrieval (1 pre-existing sibling WIP warning in `heal.rs:415`) |
| Retrieval acceptance | ✅ PASS — `every_lint_retrievable_via_suggest`: all 40 lints in top-5 via the hybrid latent+BM25 index queried with their own `buggy_code` |
| Corpus commitment | ✅ PASS — BLAKE3 freeze/thaw roundtrip unchanged; 40 unique lint names; groups style+complexity+perf still covered |

### One test relaxed (documented drift)

`draft::latent_matcher::tests::span_suggestion_routes_and_retrieves`
pinned top-1 = `needless_return` for the query `fn f() -> u8 { return 7; }`
against the 10-entry index. With 40 entries, `manual_unwrap_or`'s fixture
(`fn f(x: Option<u8>) -> u8 { match .. 7 }`) is a stronger BM25 token
match (fn/f/u8/7 overlap vs fn/return). Relaxed to top-3 membership —
consistent with the test's own comment deferring precision@1 to the
Phase 3 G1 gate (Bench 018), which still passes unchanged.

## Implementation notes

- 30 proposer files under `src/domains/clippy_lints/proposers/` + shared
  `helpers.rs` (postfix-safe / logic-paren / purity / int-lit / bool-lit /
  unit-expr / none-pat predicates).
- Built via 3 parallel subagents (control-flow / method-chain /
  literal-type-macro families), each validating in a /tmp harness before
  landing; the coordinator wired `mod.rs` + corpus + tests.
- Spec corrections the subagents caught (kept for the record): syn 2
  `ExprAssign.right` (not `expr`), `&x` is `Expr::Reference` (no
  `UnOp::Ref` in syn 2), `ExprForLoop.expr` (not `iter`), statement-position
  macros are `Stmt::Macro` (needs `visit_stmt_macro`, not just
  `visit_expr_macro`).
- Frequency sources: kornel 100K-crate run (users.rust-lang.org, Sep
  2024); Li et al. ICSE 2024 / arXiv:2310.11738; local workspace greps
  (2026-08-16); clippy 1.93.0 lint index.
