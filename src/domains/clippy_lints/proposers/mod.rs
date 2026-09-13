//! Stage 2 proposal mechanism — per-lint `syn`-level AST rewrites.
//!
//! Issue 010 fix direction (Direction 2): "Treat a corpus entry as a `syn`-level
//! rewrite rule applied to the caller's AST, rather than a literal string to
//! hand back." Each proposer is a small codemod that matches the lint's pattern
//! in the caller's code and emits the idiomatic fix with the caller's own
//! identifiers by construction.
//!
//! ## Emitter (Plan 005 Phase 6 T6.4)
//!
//! Since T6.4 every proposer is **edit-collecting**: it walks the parsed file
//! immutably and emits [`SourceEdit`]s (byte range + replacement text) through
//! the shared [`crate::edit_emitter`] substrate — the same emitter the
//! `rust_perf` proposers use. `apply_edits` splices them into the ORIGINAL
//! source: every byte outside the matched spans stays identical (no
//! whole-file `ToTokens` re-emission — that was the pre-T6.4 behavior, and
//! it destroyed the caller's formatting).
//!
//! The replacement text is built from the original source text of the matched
//! nodes, so even the edited regions keep the caller's own formatting for
//! every sub-expression they contained.
//!
//! ## Why this is modelless
//!
//! Each proposer is a deterministic pattern-match + rewrite. No training, no
//! backprop, no weights. The lint's signature IS the rule. This is the
//! **modelless generalization path** that must be exhausted before L4 `LoRA`
//! (per the modelless-first mandate).
//!
//! ## How it composes with Stage 2
//!
//! The proposer is called by `HealPipeline::tune_against_fixture` when Stage 1
//! declines (identity guard fails). The proposer's output is verified by
//! `cargo clippy` (~130 ms, tuning-phase only). Verified fixes join the
//! `ClippyAdaptiveDrafter` pool; subsequent hot-path calls serve them in O(1).
//! Since T6.4 the structurally-safe subset is also `cargo heal --fix`
//! eligible (see [`FIXABLE_LINTS`]).
//!
//! ## `--fix` eligibility ([`FIXABLE_LINTS`])
//!
//! `--fix` applies edits without a type checker — the L2 pruner only validates
//! syntax. Rules whose **detection is purely structural** (the pattern match
//! alone guarantees the rewrite preserves semantics) are `--fix` eligible.
//! The 38 rows below (6 seed + 27 from the Issue 016 expansion + 1 Batch 89
//! `use_self` + 2 Batch 90, selected
//! frequency-ranked from kornel's 100K-crate clippy run + Li et al.'s
//! 8.09M-warning crates.io study ∩ clippy's `MachineApplicable` axis ∩
//! parse-only provability; see `.benchmarks/021`):
//!
//! | Lint | Detection |
//!|---|---|
//! | `needless_return` | trailing `return expr;` → tail `expr` (syntax-local) |
//! | `redundant_field_names` | `Foo { x: x }` → `Foo { x }` (structural) |
//! | `manual_map` | exact `match`/`Some`/`None` shape (structural) |
//! | `unnecessary_unwrap` | exact `is_some`/`unwrap`/else shape (structural, receiver-matched) |
//! | `needless_range_loop` | `0..v.len()` + index-only usage (exact AST analysis) |
//! | `needless_lifetimes` | single elidable lifetime (elision-rule verified) |
//! | `semicolon_if_nothing_returned` | unit fn + bare non-block tail stmt → append `;` (41,601× in kornel's 100K-crate run — the most frequent structural lint) |
//! | `collapsible_if` | outer if, no else, single-if then-block → `if a && b` (structural; logic-paren reconstruction) |
//! | `collapsible_else_if` | `else { if .. }` single-stmt block → `else if` (structural) |
//! | `single_match` | 2-arm match, wildcard arm unit → `if let` (structural) |
//! | `single_match_else` | 2-arm match, wildcard arm statement BLOCK → `if let .. else` (structural; Issue 027 — expression wildcards never fire; Issue 029 T3 — on std Option/Result scrutinees the no-binding ctor arms `None`/`Err(_)`/`Err(..)`/`Some(_)`/`Ok(_)` count as the wildcard, gated on a std-shaped first arm) |
//! | `redundant_pattern_matching` | `if let Ok(_) = x` → `x.is_ok()` (wildcard-only variant pats; bindings never fire) |
//! | `question_mark` | `if x.is_none() { return None; }` / identity match → `x?` (structural; compilability of the original proves the shapes) |
//! | `range_plus_one` | `a..b+1` → `a..=b` (structural; `Lit::Int`-only) |
//! | `uninlined_format_args` | `{}` + plain-ident args → `{ident}` (trait bound identical — if the original compiled, the inline form compiles; aborts on any spec/named/positional placeholder or non-ident arg) |
//! | `identity_op` | `x * 1`, `x / 1`, `x - 0` (+ `0 + x` int-suffixed) → operand (IEEE-754-verified float-safety split: `+0.0` is NOT identity for `-0.0`, so unsuffixed `+ 0` never fires) |
//! | `needless_bool` | bool-literal if arms → `cond` / `!cond` (structural; else-position retraction) |
//! | `needless_bool_assign` | branch assignments `if c { x = true } else { x = false }` → `x = c` (the assign-RHS form is `needless_bool`, Issue 017 fix) |
//! | `match_bool` | `true`/`false` arm pats → `if`/`else` (structural) |
//! | `needless_match` | identity-passthrough match (Some/Ok/bool) → scrutinee (structural) |
//! | `redundant_static_lifetimes` | `&'static` in static/const type → elide (elision is always `'static` there) |
//! | `double_parens` | nested `((x))` → `(x)` (structural) |
//! | `unnecessary_lazy_evaluations` | lazy-twin closure with PURE body → eager twin (purity whitelist: no calls/macros/indexing) |
//! | `map_unwrap_or` | `map(f).unwrap_or(d)` → `map_or(d, f)` (both evaluate `d` eagerly, `f` lazily — semantics preserved) |
//! | `filter_next` | `filter(p).next()` → `find(p)` (structural method chain) |
//! | `iter_nth_zero` | `.nth(0)` → `.next()` (structural) |
//! | `manual_unwrap_or` | identity `Some` arm + PURE default → `unwrap_or` (purity gate: match-evaluated → arg-evaluated must be side-effect-free) |
//! | `unused_unit` | `-> ()` return type → elide (structural) |
//! | `useless_format` | `format!("lit")` with no braces → literal `.to_string()` / `String::new()` (structural) |
//! | `deref_addrof` | `*&x` → `x` (structural) |
//! | `wildcard_in_or_patterns` | `A | _ =>` → `_ =>` (structural) |
//! | `unnecessary_sort_by` | `sort_by(|a, b| a.cmp(b))` → `sort()` (exact closure shape; compilability proves `Ord`) |
//! | `use_self` | inherent-impl `TypeName` → `Self` path rewrite (guards: nested items, bounds, attrs, macros, generic-arg identity, value-namespace ambiguity) |
//! | `items_after_statements` | block items hoisted above the statements (order-independent resolution; guards: macro items, leading-gap / same-line-trailing comments) |
//! | `manual_let_else` | let-bound 2-arm match with visibly diverging fallback → `let .. else` (guards: binding-use fallbacks, non-identity arm-1 bodies, 3-arm/guard shapes, visible-divergence whitelist) |
//!
//! The remaining proposers fire on **type-blind** detection — they are
//! correct when the types line up, but a parse-only validator cannot know
//! that. They stay available to the `self_evolve` tuning loop (whose output
//! IS verified by a real `cargo clippy` run) and are excluded from `--fix`:
//!
//! | Lint | Why not `--fix` |
//!|---|---|
//! | `needless_borrow` | whether `&x` is needless depends on the callee's expected type |
//! | `useless_conversion` | `.into()` is only useless if source type == annotated type |
//! | `unnecessary_mut_passed` | `&mut v` → `&v` depends on the callee's signature; the param-`mut` strip is file-wide |
//!
//! `redundant_closure` sits between the two sets: demoted from
//! `FIXABLE_LINTS` by the G5 run, but restored under `--verify` via
//! [`VERIFY_LINTS`] — the E0631 failure class is exactly what the
//! verify loop reverts. Guarded 2026-08-29: the proposer now skips
//! bare-ident callees that same-file resolve to an annotated closure
//! binding or a reference-param fn (the annotation/reference IS the
//! deref-coercion site), so the verify loop sees fewer breaking edits.
//!
//! ## `--verify` tier ([`VERIFY_LINTS`], Issue 015)
//!
//! Type-blind rewrites whose structural shape is still worth attempting
//! under the Issue 015 compile gate (rustc auto-reverts the edits that fail
//! to compile): `redundant_closure` (E0631 deref-coercion closures),
//! `comparison_to_empty` (`is_empty` existence — 12 live hits in our own
//! workspace), `while_let_on_iterator` (receiver could be a non-`Iterator`
//! custom type with a `next()` method), `single_char_pattern` (method
//! resolution is receiver-type-dependent — a custom type can have
//! `contains(&str)` without `contains(char)`), `manual_is_multiple_of`
//! (dividend signedness), and `suboptimal_flops` (receiver type invisible —
//! a custom `powf`/`log` would make the rewrite semantic).
//!
//! REFUTED candidates (probed, no proposer, no corpus entry):
//! `missing_const_for_fn` (Batch 89 — const-ability is typeck-driven and the
//! lint is BROAD: an allowlist proposer fires on virtually every trivial fn,
//! cross-firing every eval fixture and the clean-span set, and cargo-fix
//! already applies it), `option_if_let_else` (Batch 90 — nursery-group pure
//! style, and its rewrite output re-enters `single_match_else`'s territory →
//! fixpoint ping-pong with our own FIXABLE proposer).
//!
//! ## Scope
//!
//! Each proposer handles ONE lint. Adding a new lint = adding a new proposer
//! module + a row in `collect_rule_edits`' match (and, when detection is
//! structural, a row in [`FIXABLE_LINTS`]). 51 lints ship today (10 seed +
//! 31 from the Issue 016 expansion and the post-mining P7 verify-tier row + the
//! Batch 89/90/91 rows + the P10 post-mining intake pair); future lints drop in by extending the
//! match.

pub mod byte_char_slices;
pub mod chunks_exact_to_as_chunks;
pub mod collapsible_else_if;
pub mod collapsible_if;
pub mod comparison_to_empty;
pub mod deref_addrof;
pub mod double_parens;
pub mod filter_map_bool_then;
pub mod filter_next;
pub mod format_collect;
pub mod helpers;
pub mod identity_op;
pub mod items_after_statements;
pub mod iter_next_loop;
pub mod iter_nth_zero;
pub mod manual_checked_ops;
pub mod manual_is_multiple_of;
pub mod manual_let_else;
pub mod manual_map;
pub mod manual_slice_fill;
pub mod manual_unwrap_or;
pub mod map_unwrap_or;
pub mod match_bool;
pub mod needless_bool;
pub mod needless_bool_assign;
pub mod needless_borrow;
pub mod needless_lifetimes;
pub mod needless_match;
pub mod needless_range_loop;
pub mod needless_return;
pub mod question_mark;
pub mod range_plus_one;
pub mod redundant_closure;
pub mod redundant_field_names;
pub mod redundant_pattern_matching;
pub mod redundant_static_lifetimes;
pub mod semicolon_if_nothing_returned;
pub mod single_char_pattern;
pub mod single_match;
pub mod single_match_else;
pub mod suboptimal_flops;
pub mod uninlined_format_args;
pub mod unnecessary_lazy_evaluations;
pub mod unnecessary_mut_passed;
pub mod unnecessary_sort_by;
pub mod unnecessary_unwrap;
pub mod unused_unit;
pub mod use_self;
pub mod useless_conversion;
pub mod useless_format;
pub mod vec_init_then_push;
pub mod while_let_on_iterator;
pub mod wildcard_in_or_patterns;

pub use crate::edit_emitter::{
    apply_edits, delete_stmt_edit, edit, for_each_fn, src, swallow_following_space, RuleEdit,
    SourceEdit,
};

/// Lints whose proposers are `cargo heal --fix` eligible — detection is
/// purely structural (see module docs). The seed six + the 27 Issue 016
/// structural expansion rows. The four type-blind lints
/// (`redundant_closure`, `needless_borrow`, `useless_conversion`,
/// `unnecessary_mut_passed`) are excluded from `--fix` and only fire
/// through the `self_evolve` verified loop / the `--verify` tier
/// (`redundant_closure` was demoted after the Bench 019 G5 run caught an
/// E0631 on a deref-coercion closure, katgpt-pruners `roaring_membership.rs`).
pub const FIXABLE_LINTS: &[&str] = &[
    // ── seed six (T6.4) ──
    "needless_return",
    "redundant_field_names",
    "manual_map",
    "unnecessary_unwrap",
    "needless_range_loop",
    "needless_lifetimes",
    // ── Issue 016 expansion (27) ──
    "semicolon_if_nothing_returned",
    "collapsible_if",
    "collapsible_else_if",
    "single_match",
    "single_match_else",
    "redundant_pattern_matching",
    "question_mark",
    "range_plus_one",
    "uninlined_format_args",
    "identity_op",
    "needless_bool",
    "needless_bool_assign",
    "match_bool",
    "needless_match",
    "redundant_static_lifetimes",
    "double_parens",
    "unnecessary_lazy_evaluations",
    "map_unwrap_or",
    "filter_next",
    "iter_nth_zero",
    "manual_unwrap_or",
    "unused_unit",
    "useless_format",
    "deref_addrof",
    "wildcard_in_or_patterns",
    "unnecessary_sort_by",
    "iter_next_loop",
    // ── Batch 89 (fired-histogram expansion, Plan 049 successor) ──
    // Parse-provable path rewrite inside inherent impls (guards: nested
    // items, bounds, attrs, macros, generic-arg identity, and the
    // value-namespace ambiguity — see the module docs).
    "use_self",
    // ── Batch 90 (the fired-histogram pedantic tail, Plan 051) ──
    // Block-item hoist: items cannot capture block locals and resolution is
    // order-independent WITHIN a block, so the hoist is semantics-preserving
    // for every compiling input (guards: macro items, leading-gap /
    // same-line-trailing comments). Applicability `Unspecified` — cargo
    // clippy --fix does NOT cover this lint.
    "items_after_statements",
    // Let-bound 2-arm match with a VISIBLY diverging fallback → let-else.
    // The probe mapped real clippy's silent shapes 1:1 onto parse-only
    // guards: binding-use fallbacks (let-else cannot see arm bindings),
    // non-identity arm-1 bodies, 3-arm/guard shapes, non-diverging
    // fallbacks. Divergence is judged visibly (return/break/continue/
    // panic-family macro) — a `!`-returning CALL fallback stays silent
    // (parse-only cannot prove the return type).
    "manual_let_else",
    // ── Batch 91 (the 2026-09-05 P9 clippy-1.98 new-lint batch) ──
    // `[b'F', b'R', b'Z', b'E']` → `*b"FRZE"` — bit-identical by
    // construction (same bytes, same `[u8; N]` type, `*b".."` legal in
    // every position the array literal was). Guards: every element a byte
    // literal, non-empty, no interior comments. FIXABLE.
    "byte_char_slices",
    // `for v in &mut place { *v = val; }` → `place.fill(val);` — pure-value
    // gate (fill evaluates ONCE, the loop per-iteration), ref-form iterable
    // gate (`iter_mut()` is silent in real clippy too), single-assign body.
    // FIXABLE.
    "manual_slice_fill",
    // NOTE — `option_if_let_else` (38 in the fired histogram) was PROBED
    // then REFUTED (Batch 90, see `.plans/051`): nursery-group pure style
    // with zero perf/correctness content, applicability Unspecified, its
    // let-match half is already covered by manual_unwrap_or(_default), and
    // its rewrite output (match with a std no-binding second arm, statement
    // position) re-enters `single_match_else`'s Issue-029-T3 territory →
    // fixpoint ping-pong with our own FIXABLE proposer. No corpus entry.
];

/// One-line semantic-shift annotations for the fixable lints whose rewrite
/// is bounded-but-real behavior-adjacent (Issue 074 T2). Printed by the
/// `--fix` review as `⚠ semantic change: {note}` so the CLI has ONE
/// annotation dialect with the `rustc_errors` renderer. Mechanical classes
/// (`format-args`, `identity_op`, `semicolon`, ...) are deliberately absent — an
/// annotation that fires on 90% of fixes is warning fatigue (Issue 074
/// scope guard).
#[must_use]
pub fn context_note(rule_id: &str) -> Option<&'static str> {
    match rule_id {
        // The Some-arm body moves into a NEW closure: ?/return/break/
        // continue/await/yield arms change propagation there and are
        // refused (the manual_map control-flow guard).
        "manual_map" => Some("control-flow capture — the Some-arm body moves into a closure; ?/return/break/continue arms are refused"),
        // Borrow shape is resolved against fn params without a type
        // checker: indexed writes / address-of slots refuse, and the
        // iterable form (&v vs v) rides param-resolution.
        "needless_range_loop" => Some("borrow shape — indexed writes and &v[i]/&mut v[i] slots refuse; the iterable form resolves against fn params, not types"),
        // Option-only short-circuit: rewrites `return None` shapes to
        // try; reference scrutinees re-emit via .as_ref() to satisfy the
        // by-value move the ? operator needs.
        "question_mark" => Some("short-circuit scope — Option/None return shapes only; reference scrutinees re-emit via .as_ref()"),
        // map_or infers U from BOTH args, so unsize coercions that fired
        // through unwrap_or's pre-fixed Option param no longer fire
        // (E0308 on trait-object/array-literal defaults) — those shapes
        // are skipped (the P9 divergence).
        "map_unwrap_or" => Some("inference shape — map_or infers the default's type from both args; unsize-coercion defaults (trait-object/array literal) are refused"),
        // lazy twin -> eager twin (unwrap_or_else(|| x) -> unwrap_or(x)):
        // the default computes on EVERY path instead of only on
        // None/Err. Gated on a syntactically-pure closure body (no
        // calls/macros/indexing), so no behavior change — but an
        // expensive pure default now costs the hot path too.
        "unnecessary_lazy_evaluations" => Some("evaluation timing — the default computes EAGERLY on every path (was: only on None/Err); pure-body-gated (no behavior change), an expensive default now costs the hot path"),
        _ => None,
    }
}

/// Lints attempted ONLY under `cargo heal --fix --verify` (Issue 015) —
/// type-blind rewrites whose failure mode (E0631/E0599) the
/// [`crate::fix_verify`] loop auto-reverts. `redundant_closure` was demoted
/// from `FIXABLE_LINTS` by the Bench 019 G5 run (the deref-coercion
/// E0631 on katgpt-pruners `roaring_membership.rs`); verify-mode restores
/// it as an attempted edit with rustc backing the revert. Issue 016 added
/// three more type-blind shapes whose structural detection is still worth
/// attempting under the compile gate.
pub const VERIFY_LINTS: &[&str] = &[
    "redundant_closure",
    "comparison_to_empty",
    "while_let_on_iterator",
    "single_char_pattern",
    // P7 (post-mining): `x % d == 0` → `x.is_multiple_of(d)`. Guarded
    // (2026-08-29): the proposer now fires only on a PROVABLY-unsigned
    // dividend — the ambiguous-`{integer}` class (bare loop vars, untyped
    // lets, unsuffixed literals) passes plain `cargo check` while CLIPPY's
    // typeck raises E0689, so the check-only --verify gate is structurally
    // blind to it and must never see the proposal at all.
    "manual_is_multiple_of",
    // Batch 91 (P9 clippy-1.98): `if n == 0 { v } else { total / n }` →
    // `total.checked_div(n).unwrap_or(v)`. VERIFY: the signed `MIN / -1`
    // corner changes panic→guard-value and a custom `Div` type has no
    // `checked_div` (E0599) — provably-signed divisors are refused in the
    // proposer, the rest rides the compile gate. Real clippy fires ONLY on
    // unsigned (probe-verified) and is MaybeIncorrect (`--fix` applied 0/17
    // on the 2026-09-04 riir-dao run); our proposer CAN see the else arm.
    "manual_checked_ops",
    // Batch 91 (P9 clippy-1.98): `chunks_exact(4)` → `as_chunks::<4>().0`
    // (and the `_mut` twin). VERIFY: the chunk changes `&[u8]` → `&[u8; N]`
    // and a non-const Path size compiles one way not the other — both
    // E0599/E0277 classes the compile gate reverts. MSRV floor: `as_chunks`
    // stabilized in std 1.88 (the workspace floats 1.98.1; real clippy is
    // MSRV-aware so the suggest path is inherently safe — documented in the
    // module doc).
    "chunks_exact_to_as_chunks",
    // P10 post-mining intake (2026-09-13, the 089-T3 usage artifact — the
    // healer needed both by hand on `src/bin/heal.rs`). VERIFY: real
    // clippy's suggestion is MachineApplicable but the emitted `|&x|`
    // filter pattern binds only COPY items (String items are silent in real
    // clippy — probed); syn cannot see the item type, so a non-Copy site is
    // an E0507 the check-level compile gate reverts (the E0507 class is
    // check-visible, unlike E0689 — see the manual_is_multiple_of lesson).
    "filter_map_bool_then",
    // P10 post-mining intake (2026-09-13): `let mut v = Vec::new();
    // v.push(a); v.push(b);` → `let v = vec![a, b];`. VERIFY: real clippy's
    // suggestion is help-only (MaybeIncorrect — `--fix` applied 0/3 even on the
    // plain shape, probed 2026-09-13) because later mutable uses make
    // dropping `mut` an E0384; this proposer refuses every parse-visible
    // later use (any method call on `v`, assignment targets, `&mut`) and
    // refuses attributed/cfg'd pushes outright (a `vec![]` literal cannot
    // carry cfg'd elements — the documented divergence; the correct fix for
    // that shape is a comment-guarded allow).
    "vec_init_then_push",
    // P11 post-mining intake (2026-09-13, the Bench-088 usage artifact):
    // `.map(|x| format!(..)).collect::<String>()` → `fold` + `write!`.
    // VERIFY: real clippy's suggestion is help-only (no machine-applicable
    // replacement — probed), and this is the FIRST two-span clippy rule
    // (the fold rewrite + a `use std::fmt::Write as _;` insertion, skipped
    // when the trait is already imported) — the import rides the per-edit
    // revert (insertion restore = ""). The anonymous `as _` form cannot
    // collide (E0252) and cannot be ambiguous (`String` implements
    // io::Write nowhere in std — probed E0599 on write_all).
    "format_collect",
    // Batch 89 (fired-histogram expansion): `suboptimal_flops` — the
    // receiver's type is invisible to syn (a custom `powf`/`log` would
    // make the rewrite semantic; the `single_char_pattern` precedent).
    // NOTE — `missing_const_for_fn` was PROPOSED then KILLED (Batch 89,
    // see `.plans/050`): const-ability is typeck-driven and the lint is
    // BROAD — an allowlist proposer fires on virtually every trivial fn,
    // cross-firing every eval fixture and the clean-span set (the
    // `needless_borrow` trio over-match precedent). Real `cargo clippy
    // --fix` applies its MachineApplicable suggestion already, so the
    // healer's marginal value was near zero. Corpus entry withheld too
    // (the corpus-lockstep gate forbids entries without proposers).
    "suboptimal_flops",
];

/// Propose a fix for `(lint_name, buggy_code)` by applying the lint's
/// span-preserving edits.
///
/// Returns `Some(fixed_code)` if a pattern matched and was rewritten,
/// `None` otherwise. The output preserves the input's formatting
/// byte-identically outside the matched spans. The output is NOT verified
/// here — the tuning loop runs `cargo clippy` to confirm.
///
/// # Errors
///
/// Returns `None` if the code doesn't parse as a `syn::File` (caller should
/// fall back to other proposal paths or give up).
#[must_use]
pub fn propose_fix(lint_name: &str, buggy_code: &str) -> Option<String> {
    let file: syn::File = syn::parse_file(buggy_code).ok()?;
    let edits = collect_rule_edits(lint_name, buggy_code, &file);
    if edits.is_empty() {
        return None;
    }
    Some(apply_edits(buggy_code, edits))
}

/// Collect one lint's edits against the original source, tagged with the
/// lint name (`--fix` reporting). Empty = no match.
#[must_use]
pub fn collect_edits(lint_name: &str, source: &str, file: &syn::File) -> Vec<RuleEdit> {
    collect_rule_edits(lint_name, source, file)
        .into_iter()
        .map(|edit| RuleEdit {
            rule_id: lint_name.to_string(),
            edit,
        })
        .collect()
}

/// Dispatch to one lint's proposer. All 51 lints are reachable here (the
/// `self_evolve` loop verifies output with cargo clippy); `--fix` callers
/// filter through [`FIXABLE_LINTS`] instead.
#[allow(clippy::too_many_lines)]
fn collect_rule_edits(lint_name: &str, source: &str, file: &syn::File) -> Vec<SourceEdit> {
    match lint_name {
        // ── seed ten (T6.4 emitter migration) ──
        "needless_return" => needless_return::collect_edits(source, file),
        "redundant_closure" => redundant_closure::collect_edits(source, file),
        "needless_lifetimes" => needless_lifetimes::collect_edits(source, file),
        "redundant_field_names" => redundant_field_names::collect_edits(source, file),
        "useless_conversion" => useless_conversion::collect_edits(source, file),
        "needless_borrow" => needless_borrow::collect_edits(source, file),
        "unnecessary_mut_passed" => unnecessary_mut_passed::collect_edits(source, file),
        "manual_map" => manual_map::collect_edits(source, file),
        "unnecessary_unwrap" => unnecessary_unwrap::collect_edits(source, file),
        "needless_range_loop" => needless_range_loop::collect_edits(source, file),
        // ── Issue 016 expansion (30) ──
        "semicolon_if_nothing_returned" => {
            semicolon_if_nothing_returned::collect_edits(source, file)
        }
        "collapsible_if" => collapsible_if::collect_edits(source, file),
        "collapsible_else_if" => collapsible_else_if::collect_edits(source, file),
        "single_match" => single_match::collect_edits(source, file),
        "single_match_else" => single_match_else::collect_edits(source, file),
        "redundant_pattern_matching" => redundant_pattern_matching::collect_edits(source, file),
        "question_mark" => question_mark::collect_edits(source, file),
        "range_plus_one" => range_plus_one::collect_edits(source, file),
        "uninlined_format_args" => uninlined_format_args::collect_edits(source, file),
        "identity_op" => identity_op::collect_edits(source, file),
        "needless_bool" => needless_bool::collect_edits(source, file),
        "needless_bool_assign" => needless_bool_assign::collect_edits(source, file),
        "match_bool" => match_bool::collect_edits(source, file),
        "needless_match" => needless_match::collect_edits(source, file),
        "redundant_static_lifetimes" => redundant_static_lifetimes::collect_edits(source, file),
        "double_parens" => double_parens::collect_edits(source, file),
        "unnecessary_lazy_evaluations" => unnecessary_lazy_evaluations::collect_edits(source, file),
        "map_unwrap_or" => map_unwrap_or::collect_edits(source, file),
        "filter_next" => filter_next::collect_edits(source, file),
        "iter_nth_zero" => iter_nth_zero::collect_edits(source, file),
        "manual_unwrap_or" => manual_unwrap_or::collect_edits(source, file),
        "unused_unit" => unused_unit::collect_edits(source, file),
        "useless_format" => useless_format::collect_edits(source, file),
        "deref_addrof" => deref_addrof::collect_edits(source, file),
        "wildcard_in_or_patterns" => wildcard_in_or_patterns::collect_edits(source, file),
        "unnecessary_sort_by" => unnecessary_sort_by::collect_edits(source, file),
        "iter_next_loop" => iter_next_loop::collect_edits(source, file),
        "comparison_to_empty" => comparison_to_empty::collect_edits(source, file),
        "while_let_on_iterator" => while_let_on_iterator::collect_edits(source, file),
        "single_char_pattern" => single_char_pattern::collect_edits(source, file),
        "manual_is_multiple_of" => manual_is_multiple_of::collect_edits(source, file),
        // ── Batch 89 (fired-histogram expansion) ──
        "use_self" => use_self::collect_edits(source, file),
        "suboptimal_flops" => suboptimal_flops::collect_edits(source, file),
        // ── Batch 90 (fired-histogram pedantic tail) ──
        "items_after_statements" => items_after_statements::collect_edits(source, file),
        "manual_let_else" => manual_let_else::collect_edits(source, file),
        // ── Batch 91 (the 2026-09-05 P9 clippy-1.98 new-lint batch) ──
        "byte_char_slices" => byte_char_slices::collect_edits(source, file),
        "manual_slice_fill" => manual_slice_fill::collect_edits(source, file),
        "manual_checked_ops" => manual_checked_ops::collect_edits(source, file),
        "chunks_exact_to_as_chunks" => chunks_exact_to_as_chunks::collect_edits(source, file),
        // ── P10 post-mining intake (2026-09-13, the 089-T3 usage artifact) ──
        "filter_map_bool_then" => filter_map_bool_then::collect_edits(source, file),
        "vec_init_then_push" => vec_init_then_push::collect_edits(source, file),
        // ── P11 post-mining intake (2026-09-13, the Bench-088 usage artifact) ──
        "format_collect" => format_collect::collect_edits(source, file),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_lint_returns_none() {
        assert_eq!(propose_fix("not_a_real_lint", "fn foo() {}"), None);
    }

    #[test]
    fn unparseable_code_returns_none() {
        assert_eq!(propose_fix("needless_return", "this is not rust {{{"), None);
    }

    /// The fixable list is the structural-detection subset — the four
    /// type-blind lints must stay out (parse-only `--fix` cannot check
    /// types). `redundant_closure` joined this set after the Bench 019 G5
    /// real-file run caught an E0631 (deref-coercion closure → fn path).
    /// The three Issue 016 VERIFY-tier lints (type-blind shapes) are
    /// excluded here too — they only fire under `--fix --verify`.
    #[test]
    fn fixable_lints_exclude_type_blind_rules() {
        for excluded in [
            "needless_borrow",
            "useless_conversion",
            "unnecessary_mut_passed",
            "redundant_closure",
            "comparison_to_empty",
            "while_let_on_iterator",
            "single_char_pattern",
            "manual_is_multiple_of",
            // Batch 91: semantic-corner (signed MIN/-1) + E0599 classes —
            // VERIFY-tier only.
            "manual_checked_ops",
            "chunks_exact_to_as_chunks",
            // Batch 89: invisible receiver types — VERIFY-tier only.
            // (missing_const_for_fn was proposed then KILLED — see the
            // VERIFY_LINTS note.)
            "suboptimal_flops",
            // P10 post-mining intake: Copy-item binding (E0507) + the
            // later-mutable-use corner (E0384) — VERIFY-tier only.
            "filter_map_bool_then",
            "vec_init_then_push",
            // P11 post-mining intake: help-only suggestion + the two-span
            // import insertion — VERIFY-tier only.
            "format_collect",
        ] {
            assert!(
                !FIXABLE_LINTS.contains(&excluded),
                "{excluded} must not be --fix eligible"
            );
        }
        // 6 seed + 27 Issue 016 structural rows + 1 Batch 89 use_self
        // + 2 Batch 90 (items_after_statements + manual_let_else)
        // + 2 Batch 91 (byte_char_slices + manual_slice_fill).
        assert_eq!(FIXABLE_LINTS.len(), 38);
    }

    /// Every proposer lint round-trips through the shared dispatch (fires on
    /// its fixture + output re-parses — the L2 pruner check `--fix` runs).
    /// 50-row fixture table — length is the point.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn all_lints_fire_and_parse() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "needless_return",
                "fn bar(x: i32) -> i32 { return x + 1; }",
                "x + 1",
            ),
            (
                "redundant_closure",
                "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| sq(n)).collect() }",
                "map(sq)",
            ),
            (
                "needless_lifetimes",
                "fn head<'a>(v: &'a Vec<u8>) -> &'a u8 { &v[0] }",
                "&u8",
            ),
            (
                "redundant_field_names",
                "struct C { x: u32 }\nfn mk(x: u32) -> C { C { x: x } }",
                "C { x }",
            ),
            (
                "useless_conversion",
                "fn twice(n: u64) -> u64 { let m: u64 = n.into(); m * 2 }",
                "let m: u64 = n;",
            ),
            (
                "needless_borrow",
                "fn show(s: &str) {}\nfn go(s: &str) { show(&s) }\nfn recv(v: Vec<u8>) -> usize { let n = (&v).as_slice().len(); n }",
                "show(s)",
            ),
            // P11 post-mining intake (2026-09-13): the fold+import rule.
            (
                "format_collect",
                "fn render(xs: &[u32]) -> String { xs.iter().map(|x| format!(\"{}\", x)).collect::<String>() }",
                ".fold(String::new()",
            ),
            (
                "unnecessary_mut_passed",
                "fn cnt(v: &Vec<u8>) -> usize { v.len() }\nfn go(mut v: Vec<u8>) -> usize { cnt(&mut v) }",
                "cnt(&v)",
            ),
            (
                "manual_map",
                "fn t(x: Option<u8>) -> Option<String> { match x { Some(v) => Some(v.to_string()), None => None, } }",
                ".map(|v|",
            ),
            (
                "unnecessary_unwrap",
                "fn g(r: Option<u32>) -> u32 { if r.is_some() { r.unwrap() } else { 7 } }",
                "r.unwrap_or(7)",
            ),
            (
                "needless_range_loop",
                "fn d(v: &Vec<i32>) -> i32 { let mut s = 0; for i in 0..v.len() { s += v[i]; } s }",
                "in v",
            ),
            // ── Issue 016 expansion (30) ──
            (
                "semicolon_if_nothing_returned",
                "fn greet() {}
fn main() {
    greet()
}",
                "greet();",
            ),
            (
                "collapsible_if",
                "fn work() {}
fn f(a: bool, b: bool) { if a { if b { work(); } } }",
                "if a && b {",
            ),
            (
                "collapsible_else_if",
                "fn work() {}
fn f(a: bool, b: bool) { if a { work(); } else { if b { work(); } } }",
                "else if b {",
            ),
            (
                "single_match",
                "fn use_v(v: u8) {}
fn f(x: Option<u8>) { match x { Some(v) => use_v(v), _ => () } }",
                "if let Some(v) = x {",
            ),
            (
                "single_match_else",
                // Issue 027: real clippy only fires when the wildcard arm is a
                // statement BLOCK (expression wildcards are silent even under
                // pedantic) — the fixture shape matches the corpus entry.
                "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v, _ => { let d = 0; d } } }",
                "else { let d = 0; d }",
            ),
            (
                "redundant_pattern_matching",
                "fn f(r: Result<u8, u8>) -> bool { if let Ok(_) = r { true } else { false } }",
                "r.is_ok()",
            ),
            (
                "question_mark",
                "fn f(x: Option<u8>) -> Option<u8> { if x.is_none() { return None; } Some(x.unwrap() + 1) }",
                "x?;",
            ),
            (
                "range_plus_one",
                "fn f(n: usize) -> usize { let mut s = 0; for i in 0..n+1 { s += i; } s }",
                "0..=n",
            ),
            (
                "uninlined_format_args",
                "fn f(name: &str) -> String { format!(\"{}\", name) }",
                "format!(\"{name}\")",
            ),
            (
                "identity_op",
                "fn f(x: u32) -> u32 { x * 1 }",
                "{ x }",
            ),
            (
                "needless_bool",
                "fn f(b: bool) -> bool { if b { true } else { false } }",
                "{ b }",
            ),
            (
                "needless_bool_assign",
                "fn f(c: bool) { let mut x = false; if c { x = true; } else { x = false; } }",
                "x = c;",
            ),
            (
                "match_bool",
                "fn f(b: bool) -> u8 { match b { true => 1, false => 0 } }",
                "if b { 1 } else { 0 }",
            ),
            (
                "needless_match",
                "fn f(x: Option<u8>) -> Option<u8> { match x { Some(v) => Some(v), None => None } }",
                "{ x }",
            ),
            (
                "redundant_static_lifetimes",
                "static GREETING: &'static str = \"hi\";",
                "&str",
            ),
            (
                "double_parens",
                "fn f(x: i32) -> i32 { ((x + 1)) }",
                "(x + 1)",
            ),
            (
                "unnecessary_lazy_evaluations",
                "fn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| 5) }",
                "x.unwrap_or(5)",
            ),
            (
                "map_unwrap_or",
                "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }",
                "x.map_or(0, |v| v + 1)",
            ),
            (
                "filter_next",
                "fn f(xs: &[u8]) -> Option<&u8> { xs.iter().filter(|c| **c > 3).next() }",
                ".find(|c| **c > 3)",
            ),
            (
                "iter_nth_zero",
                "fn f(xs: &[u8]) -> Option<&u8> { xs.iter().nth(0) }",
                ".next()",
            ),
            (
                "manual_unwrap_or",
                "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v, None => 7 } }",
                "x.unwrap_or(7)",
            ),
            (
                "unused_unit",
                "struct S;
impl S {
    fn m(&self) -> () {}
}",
                "fn m(&self) {}",
            ),
            (
                "useless_format",
                "fn f() -> String { format!(\"done\") }",
                "\"done\".to_string()",
            ),
            (
                "deref_addrof",
                "fn f(x: u8) -> u8 { let y = *&x; y }",
                "let y = x;",
            ),
            (
                "wildcard_in_or_patterns",
                "fn f(n: u8) -> u8 { match n { 1 | _ => 9, } }",
                "{ _ => 9, }",
            ),
            (
                "unnecessary_sort_by",
                "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| a.cmp(b)); }",
                "v.sort();",
            ),
            (
                "iter_next_loop",
                "fn f(it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it.next() { s += x; } s }",
                "for x in it {",
            ),
            (
                "comparison_to_empty",
                "fn f(v: &[u8]) -> bool { v.len() == 0 }",
                "v.is_empty()",
            ),
            (
                "while_let_on_iterator",
                "fn f(it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; while let Some(x) = it.next() { s += x; } s }",
                "for x in it {",
            ),
            (
                "single_char_pattern",
                "fn f(s: &str) -> bool { s.contains(\"x\") }",
                "s.contains('x')",
            ),
            // ── Batch 89 (fired-histogram expansion) ──
            (
                "use_self",
                "struct V { n: u32 }\nimpl V { fn new() -> V { V { n: 1 } } }",
                "Self { n: 1 }",
            ),
            (
                "suboptimal_flops",
                "fn f(x: f64) -> f64 { x.powf(0.5) }",
                "x.sqrt()",
            ),
            // ── Batch 90 (fired-histogram pedantic tail) ──
            (
                "items_after_statements",
                "fn f() -> i32 { let x = 1; fn h(v: i32) -> i32 { v * 2 } h(x) }",
                "fn h(v: i32) -> i32 { v * 2 }\n\nlet x = 1;",
            ),
            (
                "manual_let_else",
                "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => return 0 }; v + 1 }",
                "let Some(v) = x else { return 0 };",
            ),
            // ── Batch 91 (the 2026-09-05 P9 clippy-1.98 new-lint batch) ──
            (
                "byte_char_slices",
                "const SHARD_MAGIC: [u8; 4] = [b'S', b'H', b'R', b'D'];",
                "*b\"SHRD\"",
            ),
            (
                "manual_slice_fill",
                "struct Terrain { data: Vec<f32> }\nfn reset(t: &mut Terrain) {\n    for v in &mut t.data {\n        *v = 0.5;\n    }\n}",
                "t.data.fill(0.5);",
            ),
            (
                "manual_checked_ops",
                "fn share(total: u64, n: u64) -> u64 { if n == 0 { 0 } else { total / n } }",
                "total.checked_div(n).unwrap_or(0)",
            ),
            (
                "chunks_exact_to_as_chunks",
                "fn ids(bytes: &[u8]) -> Vec<u32> { let mut out = Vec::new(); for chunk in bytes.chunks_exact(4) { out.push(u32::from_le_bytes(chunk.try_into().unwrap())); } out }",
                "bytes.as_chunks::<4>().0",
            ),
            // ── P10 post-mining intake (2026-09-13, the 089-T3 usage artifact) ──
            (
                "filter_map_bool_then",
                "fn kept(xs: Vec<i32>) -> Vec<i32> { xs.into_iter().filter_map(|x| (x > 0).then(|| x * 2)).collect() }",
                "filter(|&x| (x > 0)).map(|x| x * 2)",
            ),
            (
                "vec_init_then_push",
                "fn build() -> Vec<u8> { let mut v = Vec::new(); v.push(1); v.push(2); v }",
                "let v = vec![1, 2];",
            ),
        ];
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        for (lint, buggy, marker) in cases {
            let out = propose_fix(lint, buggy)
                .unwrap_or_else(|| panic!("proposer did not fire for `{lint}`"));
            assert!(out.contains(marker), "`{lint}` missing `{marker}`: {out}");
            let verdict = pruner.validate(&out, lint);
            assert!(verdict.parses, "`{lint}` output does not parse: {out}");
        }
    }

    /// The load-bearing invariant of the T6.4 emitter migration: untouched
    /// bytes survive verbatim (comments, blank lines, unrelated statements).
    #[test]
    fn output_preserves_untouched_bytes() {
        let src = "// header\nfn bar(x: i32) -> i32 {\n    // keep me\n    let y = x * 2;\n    \
                   return x + y;\n}\n\nfn other() -> u8 { 1 }\n";
        let out = propose_fix("needless_return", src).expect("pattern matched");
        assert!(out.contains("// header\n"), "{out}");
        assert!(out.contains("    // keep me\n"), "{out}");
        assert!(out.contains("let y = x * 2;"), "{out}");
        assert!(out.contains("x + y\n"), "{out}");
        assert!(out.contains("fn other() -> u8 { 1 }"), "{out}");
        assert!(!out.contains("return"), "{out}");
        // Line count unchanged: the return stmt became a tail expr in place.
        assert_eq!(src.lines().count(), out.lines().count(), "{out}");
    }
}
