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
//! Since T6.4 the structurally-safe subset is also `cargo refine --fix`
//! eligible (see [`FIXABLE_LINTS`]).
//!
//! ## `--fix` eligibility ([`FIXABLE_LINTS`])
//!
//! `--fix` applies edits without a type checker — the L2 pruner only validates
//! syntax. Rules whose **detection is purely structural** (the pattern match
//! alone guarantees the rewrite preserves semantics) are `--fix` eligible.
//! The 89 rows below (the batch-enumeration tail: … + 3 Batch 228 + 5 Batch
//! 230, the clippy-1.99 toolchain-move re-arm — the authoritative count is
//! the `fixable_lints_count` assert, never this sentence) — selected
//! frequency-ranked from kornel's 100K-crate clippy run + Li et al.'s
//! 8.09M-warning crates.io study ∩ clippy's `MachineApplicable` axis ∩
//! parse-only provability; see `.benchmarks/021`; + 2 Batch 91 + 1 P15
//! `unnecessary_map_or`):
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
//! | `question_mark` | `if x.is_none() { return None; }` / identity match / `let Some(v) = e else { return None }` → `x?` / `let v = e?;` (structural; compilability of the original proves the shapes) |
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
//! | `doc_markdown` | identifier-shaped words in `///`/`//!` docs → backtick wrap (comment-local — the edit cannot change compilation; clippy's own word predicates ported verbatim, context approximated per line) |
//! `manual_range_patterns` | or-pattern → `min..=max` (clippy's own algorithm ported verbatim incl. range alternatives, duplicates, negatives + the coverage walk; boundary literals spliced verbatim — probe-matrixed on 0.1.93 + 0.1.98) |
//! `to_string_in_format_args` | `format!("{}", x.to_string())` → `format!("{}", x)` on signature-proven scalar receivers + the bare-`{}` placeholder guard (Batch 158) |
//! `manual_range_contains` | `x >= 3 && x <= 5` → `(3..=5).contains(&x)` on proven scalar operands + literal bounds (Batch 158) |
//! `single_char_add_str` | `s.push_str("!")` → `s.push('!')` on signature-proven `String`/`&mut String` receivers, VALUE-based single-char arg + `escape_default` emit (Batch 159) |
//! `manual_clamp` | `x.max(3).min(5)` → `x.clamp(3, 5)` (+ the mirrored order twin) on proven int/char receivers, LITERAL bounds + the NaN-float and inverted-bounds panic guards (Batch 159) |
//! `from_str_radix_10` | `u32::from_str_radix(s, 10)` → `s.parse::<u32>()` — construction identity (std's int `FromStr` IS radix-10 `from_str_radix`), receiver verbatim + paren-wrap (Batch 160) |
//! `assign_op_pattern` | `y = y + 1` → `y += 1` on exactly-once-proven arithmetic scalars (params/annotated/init-transfer/suffixed-literal bindings), commutative swapped twin `+`/`*` only (Batch 160) |
//! `unit_arg` | `touch({ hits.push(1); }, hits)` → hoist the syntactically-unit block + pass `()` — value/order identity by construction, clippy's own suggestion shape (Batch 161) |
//! `string_lit_as_bytes` | `"abc".as_bytes()` → `b"abc"` — coercion identity; ASCII-only, `\u{` escape refused (clippy's suggestion there is invalid Rust — upstream bug); nursery group (Batch 161) |
//! `bytes_count_to_len` | `x.bytes().count()` → `x.len()` — std-definition identity (`str::bytes` = the `as_bytes` elements; `count()` = `len()`); exact two-call chain only (Batch 162) |
//! `bool_comparison` | `x == true` → `x`, `x != true` → `!x` (+ the `false` twins, both operand orders) — boolean algebra on a proven-bool ident (Batch 162) |
//! `manual_string_new` | `String::from("")` → `String::new()` — empty-String construction identity (zero capacity, no allocation); 2-segment path + explicit turbofish only, empty-literal arg only (Batch 163) |
//! `needless_late_init` | `let x[: T]; x = v;` → `let x[: T] = v;` — the adjacent non-mut pair merged (E0384 makes the adjacent assignment THE initialization); annotation + initializer verbatim (Batch 163) |
//! `match_like_matches_macro` | `match x { P => true, _ => false }` → `matches!(x, P)` (+ the `!` twin + the arm-1 guard) — the macro IS that match; scrutinee/pattern/guard verbatim (Batch 164) |
//! `unnecessary_cast` | `x as T` → `x` on a proven-same-type ident; `5 as u64` → `5_u64` on an int/float literal — identity cast / literal-suffix construction identity; param provenance only in v1, cross-type + pointer targets refuse (Batch 167) |
//! `clone_on_copy` | `x.clone()` → `x` on a proven-Copy receiver (primitives + `&str`/`&String`) — the std Copy/Clone contract makes the call a no-op; `String` refuses (`redundant_clone`'s territory) (Batch 167) |
//! `empty_line_after_doc_comments` | `/// docs` + blank run + item → delete the blank run (pure whitespace; the doc re-attaches); attr-intervened silence mirrored (Batch 168) |
//! `match_ref_pats` | `match f { &A => .., &B => .. }` → `match *f { A => .., B => .. }` — deref the ident scrutinee + strip each arm's leading `&`; binding-free sub-patterns only (binding modes are the staged hazard); mixed/wildcard/or arms are upstream-silent and refused (Batch 169) |
//! `into_iter_on_ref` | `(&v).into_iter()` → `(&v).iter()` (+ the `(&mut v)` → `iter_mut` twin) — method rename only, clippy's own suggestion shape; the receiver ident proven container-typed by the fn signature (std's `IntoIterator for &T` IS `iter()`); for-heads refuse (the rename births `explicit_iter_loop`) (Batch 169) |
//! `tabs_in_doc_comments` | every TAB in a doc comment → four spaces — comment bytes carry zero semantics; `///`, `//!`, `/** */` AND fenced code samples fire (probe); regular `//`/`////` stay silent; item/file-level doc attrs only (Batch 171) |
//! `map_clone` | `v.iter().map(|x| x.clone())` → `.cloned()` / `.map(|x| *x)` → `.copied()` — the shape compiling PROVES the adapter's bound (T: Clone / T: Copy); by-ref adapter chains only (`into_iter`'s empty-suggestion deletion is Copy-only — refused, the documented divergence) (Batch 171) |
//! `new_without_default` | `impl W { pub fn new() -> Self }` with no Default → insert `impl Default for W` delegating to `new()` — purely additive, clippy's own suggestion shape (Batch 168) |
//! `len_without_is_empty` | inherent impl with a public `len(&self) -> <int>` and no `is_empty` on the type → INSERT `pub fn is_empty(&self) -> bool { self.len() == 0 }` before the impl's closing brace — the additive-insertion precedent; traits refuse v1 (Batch 175) |
//! `cast_abs_to_unsigned` | `x.abs() as uN` → `x.unsigned_abs()` on a signature-proven signed-int param — whole-expr replacement iff the unsigned twin width equals the cast target, else the cast survives (narrow/wide); release-identical + debug-strictly-better (the lint's own "can panic" motivation) (Batch 175) |
//! `ptr_offset_with_cast` | `p.offset(i as isize)` → `p.add(i)` (+ `wrapping_offset` → `wrapping_add`) — receiver proven a raw pointer by the fn head (`*const/*mut` param) or an `as_ptr`/`as_mut_ptr`-initialized local (never-rebound, never-assigned); operand a `usize` param (upstream's own firing precondition, probe-measured) (Batch 184) |
//! `needless_borrowed_reference` | `let &ref y = e;` → `let ref y = e;` — the leading `&` deleted (upstream's own one-span `MachineApplicable`); LET-bindings only (the match shape co-fires `match_ref_pats`, which owns it); exactly `&ref IDENT`, non-mut, no subpat, no let-else (Batch 184) |
//! `multiple_bound_locations` | `<T: Default>` + `where T: Default` → the duplicated bound consolidated into ONE location — subset-driven (inline ⊆ where deletes the inline segment; where ⊆ inline deletes the predicate/whole clause); fn-only surface = upstream parity (trait impls/structs probe-silent); disjoint sets refuse — the merge case, the documented v1 divergence (Batch 185) |
//! `excessive_precision` | `2.141592653589793238462643f32` → `2.1415927f32` — suffix-pinned shortest-round-trip rewrite, bit-identity proven in-proposer (Batch 164) |
//! `match_same_arms` | adjacent arms with byte-identical bodies + binding-free patterns + no guards → `p1 | p2 | .. => B` — merging adjacent arms is execution-equivalent by construction (one arm runs either way); clippy's non-adjacent/typed-equality/wildcard-delete shapes are refused (the conservative subset, P51a 2026-10-14) |
//!
//! The remaining proposers fire on **type-blind** detection — they are
//! correct when the types line up, but a parse-only validator cannot know
//! that. They stay available to the `self_evolve` tuning loop (whose output
//! IS verified by a real `cargo clippy` run) and are excluded from `--fix`:
//!
//! | Lint | Why not `--fix` |
//!|---|---|
//! | `needless_borrow` | whether `&x` is needless depends on the callee's expected type |
//! | `useless_conversion` | `.into()` is only useless if source type == annotated type; `.into_iter()` as a direct call arg is only useless if the callee param is `IntoIterator`-generic; `.into()` as a block TAIL is only useless if the block's expected type == receiver type (wrong drops are compile errors, never silent) |
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
//! module + a row in `collect_rule_edits`'s match (and, when detection is
//! structural, a row in [`FIXABLE_LINTS`]). 88 lints ship today (10 seed +
//! 31 from the Issue 016 expansion and the post-mining P7 verify-tier row + the
//! Batch 89/90/91 rows + the P10 post-mining intake pair + the P11 fold rule +
//! the P12 nursery receiver rule + the P13 chain-root extension (same lint,
//! `chunks_exact_to_as_chunks`) + the P15 `unnecessary_map_or` rule + the
//! Issue 120 `doc_markdown` rule + the P31 `manual_range_patterns` rule + the
//! the 6 Batch 150 rows + the 2 Batch 151 doc-lint rows + the 4 Batch 155
//! frontier-lead rules — `let_and_return` / `ptr_arg` / `redundant_slicing` /
//! `map_flatten`, Plan 146 — and the 2 Batch 156 frontier-lead rules —
//! `io_other_error` / `wrong_self_convention`, Plan 147 — and the 2 Batch 157
//! frontier-lead rules — `redundant_closure_for_method_calls` /
//! `iter_skip_next`, Plan 148 — and the 2 Batch 158 frontier-lead rules —
//! `to_string_in_format_args` / `manual_range_contains`, Plan 149 — and
//! the 2 Batch 159 frontier-lead rules — `single_char_add_str` /
//! `manual_clamp`, Plan 150 — and the 2 Batch 160 frontier-lead rules —
//! `from_str_radix_10` / `assign_op_pattern`, Plan 151 — and the 2 Batch
//! 161 frontier-lead rules — `unit_arg` / `string_lit_as_bytes`, Plan
//! 152 — and the 2 Batch 162 frontier-lead rules — `bytes_count_to_len` /
//! `bool_comparison`, Plan 153 — and the 2 Batch 163 frontier-lead rules —
//! `manual_string_new` / `needless_late_init`, Plan 154 — and the 2 Batch
//! 164 frontier-lead rules — `match_like_matches_macro` /
//! `excessive_precision`, Plan 155 — and the 2 Batch 167 coverage-report
//! re-arm rules — `unnecessary_cast` / `clone_on_copy`, Plan 158 — and
//! the 2 Batch 168 coverage-report re-arm rules —
//! `empty_line_after_doc_comments` / `new_without_default`, Plan 159 —
//! the 2 Batch 175 coverage-report re-arm rules — `len_without_is_empty` /
//! `cast_abs_to_unsigned`, Plan 169, the pack-XV overturns of two Plan-163
//! refusals on NEW evidence: the landed `new_without_default` insertion
//! precedent and the `scalar_param_heads_over` signature-provenance
//! substrate, both gate-proven since — and the 2 Batch 184 pack-XVI rules
//! (`ptr_offset_with_cast` / `needless_borrowed_reference`, Plan 179 — the
//! Plan-169 staged IOU pair); future lints drop in by extending
//! the match.

pub mod assert_is_empty;
pub mod assign_op_pattern;
pub mod bit_width;
pub mod block_scrutinee;
pub mod bool_comparison;
pub mod byte_char_slices;
pub mod collapsible_match;
pub mod bytes_count_to_len;
pub mod cast_abs_to_unsigned;
pub mod cast_lossless;
pub mod chunks_exact_to_as_chunks;
pub mod clone_on_copy;
pub mod collapsible_else_if;
pub mod collapsible_if;
pub mod comparison_to_empty;
pub mod deref_addrof;
pub mod doc_lazy_continuation;
pub mod doc_markdown;
pub mod double_parens;
pub mod empty_line_after_doc_comments;
pub mod excessive_precision;
pub mod explicit_auto_deref;
pub mod filter_map_bool_then;
pub mod filter_next;
pub mod format_collect;
pub mod from_str_radix_10;
pub mod helpers;
pub mod identity_op;
pub mod into_iter_on_ref;
pub mod io_other_error;
pub mod items_after_statements;
pub mod iter_next_loop;
pub mod iter_nth_zero;
pub mod iter_skip_next;
pub mod legacy_numeric_constants;
pub mod len_without_is_empty;
pub mod let_and_return;
pub mod manual_checked_ops;
pub mod manual_clamp;
pub mod manual_is_multiple_of;
pub mod manual_let_else;
pub mod manual_map;
pub mod manual_next_back;
pub mod manual_ok_err;
pub mod manual_range_contains;
pub mod manual_range_patterns;
pub mod manual_repeat_n;
pub mod manual_slice_fill;
pub mod manual_string_new;
pub mod manual_unwrap_or;
pub mod map_clone;
pub mod map_flatten;
pub mod map_or_identity;
pub mod map_unwrap_or;
pub mod match_bool;
pub mod match_like_matches_macro;
pub mod match_ref_pats;
pub mod match_same_arms;
pub mod mem_replace_with_default;
pub mod multiple_bound_locations;
pub mod needless_bool;
pub mod needless_bool_assign;
pub mod needless_borrow;
pub mod needless_borrowed_reference;
pub mod needless_late_init;
pub mod needless_lifetimes;
pub mod needless_match;
pub mod needless_pass_by_ref_mut;
pub mod needless_range_loop;
pub mod needless_return;
pub mod new_without_default;
pub mod nonnull_unchecked_on_box_ptr;
pub mod partialeq_to_none;
pub mod print_write_with_newline;
pub mod ptr_arg;
pub mod ptr_offset_with_cast;
pub mod question_mark;
pub mod range_plus_one;
pub mod receiver_kind;
pub mod redundant_closure;
pub mod redundant_closure_for_method_calls;
pub mod redundant_field_names;
pub mod redundant_pattern_matching;
pub mod redundant_slicing;
pub mod redundant_static_lifetimes;
pub mod semicolon_if_nothing_returned;
pub mod single_char_add_str;
pub mod single_char_pattern;
pub mod single_match;
pub mod single_match_else;
pub mod string_lit_as_bytes;
pub mod suboptimal_flops;
pub mod tabs_in_doc_comments;
pub mod to_string_in_format_args;
pub mod uninlined_format_args;
pub mod unit_arg;
pub mod unnecessary_cast;
pub mod unnecessary_get_then_check;
pub mod unnecessary_min_or_max;
pub mod unnecessary_lazy_evaluations;
pub mod unnecessary_map_or;
pub mod unnecessary_mut_passed;
pub mod unnecessary_sort_by;
pub mod unnecessary_unwrap;
pub mod unreadable_literal;
pub mod unused_unit;
pub mod use_self;
pub mod used_underscore_binding;
pub mod useless_conversion;
pub mod useless_format;
pub mod vec_init_then_push;
pub mod while_let_on_iterator;
pub mod wildcard_in_or_patterns;
pub mod wrong_self_convention;

pub use crate::edit_emitter::{
    apply_edits, delete_stmt_edit, edit, for_each_fn, src, swallow_following_space, RuleEdit,
    SourceEdit,
};

/// Lints whose proposers are `cargo refine --fix` eligible — detection is
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
    // ── P15 post-mining intake (2026-09-14, the born-lint closure — the
    // healer's own `map_unwrap_or` fallback `map_or(false, f)` is re-flagged
    // by real clippy as this lint; probe `.raw/mapor_probe`, clippy 1.98:
    // Option+false→`is_some_and`, Option+true→`is_none_or`, Result+false→
    // `is_ok_and`, all MachineApplicable; Result+true and `map_or_else` are
    // clippy-SILENT, we mirror). FIXABLE: fires ONLY on receivers PROVEN
    // Option/Result by the shared `receiver_kind` substrate (fn params,
    // same-file fn returns incl. forward refs, shape allowlists, and the
    // P15 safe-let classification — single-binding, never-assigned `let x =
    // <provable init>`). Unknown receivers REFUSE: the type-blind
    // standalone pick is a hard E0599 (the bench_440 class) — the
    // documented divergence; the `--clippy-json` oracle lane is the close.
    // MSRV floors: `is_some_and`/`is_ok_and` 1.70, `is_none_or` 1.82.
    "unnecessary_map_or",
    // ── Issue 120 intake (2026-09-16, the pedantic doc-lint family) ──
    // Comment-local backtick wrap — the edit cannot change compilation,
    // and detection is a faithful port of clippy's own word predicates
    // (the module docs carry the context approximations).
    "doc_markdown",
    // ── P31 post-mining intake (2026-09-17, the riir-chain 51b456f MISS
    // artifact): `0 | 1 | 2` → `0..=2`. Clippy's own algorithm ported
    // verbatim (rust-lang/rust@1.98.1) — the coverage walk admits
    // scrambled/descending/duplicate/negative/range-bearing or-patterns;
    // the boundary literals are spliced VERBATIM (sign/separators/suffix),
    // so the rewrite is set-identical + inference-identical and every
    // accepted input compiles after it. See the module doc for the probe
    // matrix and the documented conservative divergences.
    "manual_range_patterns",
    // NOTE — `option_if_let_else` (38 in the fired histogram) was PROBED
    // then REFUTED (Batch 90, see `.plans/051`): nursery-group pure style
    // with zero perf/correctness content, applicability Unspecified, its
    // let-match half is already covered by manual_unwrap_or(_default), and
    // its rewrite output (match with a std no-binding second arm, statement
    // position) re-enters `single_match_else`'s Issue-029-T3 territory →
    // fixpoint ping-pong with our own FIXABLE proposer. No corpus entry.
    // ── Batch 150 (the staged default-level + pedantic probe pack) ──
    // `print!("x\n")` → `println!("x")` (+ the `eprint!` twin). FIXABLE:
    // parse-only provable — the literal ends in the `\n` ESCAPE and the
    // macro family (print/eprint → println/eprintln) carries the newline
    // for free. Probe-verified silent shapes mirrored (println! itself,
    // interior \n, raw strings, escaped-backslash endings). Full-literal
    // form produces the cascade terminal directly (`print!("\n")` →
    // `println!()` — never the re-firing `println!("")`).
    "print_with_newline",
    // `write!(f, "x\n")` → `writeln!(f, "x")` — the fmt::Write sibling;
    // same proposer module, same probe discipline (writeln! never fires
    // — nothing to suggest).
    "write_with_newline",
    // `1234567` → `1_234_567` (+ the float/fraction form). FIXABLE: the
    // only parse-only-PROVABLE pedantic row in the staged head — digit
    // grouping is value-preserving by construction (no type knowledge, no
    // scope resolution). Digit-COUNT threshold ≥6 (measured boundary:
    // 10000 silent, 999999 fires); any existing `_` refuses (that is
    // large_digit_groups' territory); radix prefixes + exponents refuse.
    "unreadable_literal",
    // `std::u32::MAX` → `u32::MAX` (+ the flat `use std::u32::MAX;` import
    // form). FIXABLE with a SHADOW GUARD: path resolution prefers an
    // in-scope item named like a primitive, and the rewrite compiles with
    // a different meaning — the guard refuses any file carrying an item
    // or non-std use-alias/last-segment named like one of the 14
    // primitives (the compile gate cannot arbitrate this class; real
    // clippy marks its own suggestion MaybeIncorrect for exactly this
    // reason). Const table only names legacy constants with assoc-const
    // twins (std::f64::MAX has none — refused).
    "legacy_numeric_constants",
    // ── Batch 151 (the doc-line state-machine port, Plan 137 — the staged
    // follow-up from the Batch 150 probe pack, Research 177 #7) ──
    // Lazy markdown continuation lines → explicit markers. `> quote` then a
    // bare line → `> ` insert (MachineApplicable in real clippy);
    // `- item` then an under-indented line → deficit-spaces insert
    // (MaybeIncorrect there — clippy's own help names the blank-line
    // alternative, surfaced as our context_note). FIXABLE per the
    // doc_markdown precedent: comment-local edits can never change
    // compilation, so the compile gate has nothing to revert — the guards
    // are the measured state machine (probe151 boundary matrix: exact-col,
    // blank-gap, heading/fence block starts, marker lines, the `>`-count
    // quirk) plus render-stability in rustdoc (pulldown reads the fixed
    // line exactly as it read the lazy one).
    "doc_lazy_continuation",
    // Continuation indented past the list item's content col → dedent to
    // exactly the content col (MaybeIncorrect in real clippy). One state
    // machine with doc_lazy_continuation (clippy checks both from one
    // file); +4-over (CommonMark code-block territory) still fires —
    // pulldown reads it as paragraph text, so the dedent is render-stable
    // where it matters (the module doc carries the eyes-open note).
    "doc_overindented_list_items",
    // ── Batch 155 (the frontier-lead mining pack, Plan 146 — the frontier
    // miner's first 5 consumed leads; each family fixture verified to fire
    // EXACTLY its lint under all+pedantic+nursery, 2026-09-18) ──
    // `let x = expr; x` tail → `expr`. FIXABLE: simple-let guards (no
    // annotation/mut/attrs/else, binding used exactly once — the tail,
    // method-position excluded), parse-provable + inference-identical.
    "let_and_return",
    // `f(&s[..])` → `f(s)`. FIXABLE: fires only when the ident resolves to
    // a same-file fn param typed `&str`/`&[T]` (the receiver_kind
    // resolution precedent) — then the full-range slice is provably
    // redundant. Locals/owned params refuse.
    "redundant_slicing",
    // `it.map(f).flatten()` → `it.flat_map(f)`. FIXABLE: Map's Item IS f's
    // output and both adapters flatten through the same IntoIterator
    // bound — semantics-identical whenever both compile (MachineApplicable
    // in real clippy; attrs/turbofish/arg-count guards).
    "map_flatten",
    // ── Batch 156 (the frontier-lead mining pack II — Plan 147 — the
    // frontier miner's re-armed leads, 2026-09-18) ──
    // `io::Error::new(io::ErrorKind::Other, e)` → `io::Error::other(e)`.
    // FIXABLE: std's `new` and `other` carry the IDENTICAL
    // `E: Into<Box<dyn Error + Send + Sync>>` payload bound —
    // inference-identical whenever the kind is the literal
    // `ErrorKind::Other` path; shadow guard refuses files carrying items
    // or non-std use-aliases named Error/ErrorKind/io. MSRV floor 1.74.
    "io_other_error",
    // ── Batch 157 (the frontier-lead mining pack III — Plan 148 — the
    // Batch 156 re-arm's leads, 2026-09-18) ──
    // `v.iter().map(|s| s.len())` → `v.iter().map(String::len)` — FIXABLE
    // via the SIGNATURE-PROVEN element type (`&[T]`/`&[T; N]`/`&Vec<T>`
    // param, never rebound) + a construction-identity whitelist
    // (String/Vec/HashMap/HashSet × len/is_empty/capacity, inherent &self
    // 0-arg std methods): the emitted fn item carries exactly the closure's
    // `fn(&T) -> _` signature. Shadow guard on the four head names.
    "redundant_closure_for_method_calls",
    // `x.skip(n).next()` → `x.nth(n)` (every iterator — `Iterator::nth`
    // IS `skip(n).next()`), with the ORACLE-CLEAN slice upgrade
    // `v.iter().skip(n).next()` → `v.get(n)` on signature-proven
    // slice-container receivers (`.nth(n)` itself fires `iter_nth` on
    // slice iters — measured, the Batch 156 round-3 probe).
    "iter_skip_next",
    // ── Batch 158 (the frontier-lead mining pack IV — Plan 149 — the
    // Batch 157 re-arm's leads, 2026-09-18) ──
    // `format!("n={}", x.to_string())` → `format!("n={}", x)` — FIXABLE
    // via the SIGNATURE-PROVEN scalar receiver (primitive/`&str`/`String`
    // param annotation, never rebound: the only in-scope `to_string` for
    // such a type IS the std Display blanket — a second in-scope trait
    // `to_string` would be E0031 ambiguity in the ORIGINAL) + the
    // bare-`{}` placeholder guard (spec placeholders change the trait —
    // Debug String ≠ Debug u32 — and refuse). The string-literal
    // receiver arm is construction-identical. Perf-group (default-on;
    // group pinned by the plan149 probe). Composes with
    // `uninlined_format_args` by design: uninlined skips method-call
    // args, this strips exactly there, uninlined inlines the result on
    // the next fixpoint iteration.
    "to_string_in_format_args",
    // `x >= 3 && x <= 5` → `(3..=5).contains(&x)` (+ the exclusive twin
    // and both mirrored operand orders) — FIXABLE via std's own
    // `contains` definition (exactly the two leg comparisons, NaN/∞
    // included) + the SIGNATURE-PROVEN scalar operand + literal bounds
    // (both range ends unify with the operand's type — every impl is
    // same-type, closing the mixed-PartialOrd compile-direction hole).
    // Variable bounds refuse (the documented narrower-than-clippy
    // divergence, pending the same derivation for bound idents).
    "manual_range_contains",
    // ── Batch 159 (the frontier-lead mining pack V — Plan 150 — the
    // Batch 158 re-arm's leads, 2026-09-18) ──
    // `s.push_str("!")` → `s.push('!')` — FIXABLE via the
    // SIGNATURE-PROVEN mutable-String receiver (`String`/`&mut String`
    // param, never rebound: the only in-scope `push_str`/`push` pair is
    // `String`'s own inherent `&str`/`char` methods — a custom type's
    // `push_str` is excluded by the derivation itself) + the VALUE-based
    // single-`char` arg (escapes resolve through `LitStr::value()`; the
    // emit re-escapes via `char::escape_default`). Style-group
    // (default-on; group pinned by the plan150 probe's default-groups
    // arm). Local `String::new()` bindings + UFCS refuse (the documented
    // divergence, pending a local-binding derivation).
    "single_char_add_str",
    // `x.max(3).min(5)` → `x.clamp(3, 5)` (+ the mirrored
    // `x.min(5).max(3)` order twin) — FIXABLE via the lattice absorption
    // law on TOTAL orders (ints + `char`, same `partial_cmp` under
    // max/min/clamp) + the SIGNATURE-PROVEN scalar receiver + LITERAL
    // bounds with the decidable `lo <= hi` panic guard. TWO divergences
    // REFUSE by design: `f32`/`f64` (NaN.max(3).min(5) == 3.0 but
    // NaN.clamp == NaN — the anti-regime witness) and inverted bounds
    // (clamp asserts `min <= max` release-too; the manual chain never
    // panics). Variable bounds refuse (the manual_range_contains
    // divergence precedent). Style-group (default-on; group pinned by
    // the plan150 probe's default-groups arm).
    "manual_clamp",
    // ── Batch 160 (the frontier-lead mining pack VI — Plan 151 — the
    // Batch 159 re-arm's leads, 2026-09-18) ──
    // `u32::from_str_radix(s, 10)` → `s.parse::<u32>()` — FIXABLE by
    // CONSTRUCTION IDENTITY: std's `impl FromStr for {int}` IS
    // `from_str_radix(src, 10)`, so `parse::<T>()` calls the SAME fn
    // with the SAME input and returns the SAME `Result<T,
    // ParseIntError>`; the turbofish re-spells the type the original
    // already named (RESOLUTION-IDENTICAL — no new name resolution, no
    // shadow guard needed, unlike legacy_numeric_constants which
    // INTRODUCED the bare primitive path). Receiver = arg0 verbatim
    // (paren-wrapped when not postfix-safe — autoderef reaches
    // `str::parse` on every shape the `&str` coercion compiled on).
    // Radix = int LITERAL value 10 (suffix rides). The 12 int
    // primitives (exactly where both `from_str_radix` and the radix-10
    // `FromStr` impl exist). Style-group (default-on; group pinned by
    // the plan151 probe's default-groups arm).
    "from_str_radix_10",
    // `y = y + 1` → `y += 1` (+ the commutative swapped twin
    // `y = 1 + y`, `+`/`*` only — the probe-measured boundary: `y = 1 - y`
    // is clippy-SILENT and refused) — FIXABLE by construction on
    // BUILT-IN ops: compound assignment IS the binary op + store (same
    // overflow checks). The overloaded-operator hole is closed by TYPE
    // DERIVATION (the LHS ident proven an ARITHMETIC scalar — ints +
    // floats — via a NEW exactly-once-binding derivation: params,
    // annotated lets, init-transfer lets `let mut y = <proven ident>`,
    // and SUFFIXED-literal inits; bare `0`/`0.0` refuse the
    // `{integer}`/f32-unification ambiguity class). Span-order guard +
    // nested-item skip (fns don't capture). Style-group (default-on;
    // group pinned by the plan151 probe's default-groups arm).
    "assign_op_pattern",
    // ── Batch 161 (the frontier-lead mining pack VII — Plan 152 — the
    // Batch 160 re-arm's leads, 2026-09-18) ──
    // `touch({ hits.push(1); }, hits)` → `{ hits.push(1); };
    // touch((), hits)` — FIXABLE by SYNTACTIC UNIT-TYPERNESS: a block
    // with NO tail expression has type `()` by the language definition,
    // so the arg's value is already unit and re-spelling it as the
    // literal is value-identical; the block HOISTS before the call with
    // evaluation order identical BY CONSTRUCTION (args evaluate
    // left-to-right, so hoisting the index-0 block preserves the exact
    // sequence; later positions refuse — and clippy itself is SILENT on
    // the trailing-block shape, measured). The empty-block arm
    // `touch({}, hits)` → `touch((), hits)` is in-place. Byte-for-byte
    // clippy's own MachineApplicable suggestion shape (measured by the
    // plan152 probe). Complexity-group (default-on; group pinned by the
    // plan152 probe's default-groups arm).
    "unit_arg",
    // `"abc".as_bytes()` → `b"abc"` — construction-identical in every
    // coercion context (`&[u8; N]` → `&[u8]`); clippy's own documented
    // known problem (inference-sensitive contexts) is reverted by the
    // `--fix --verify` compile gate — the upstream MachineApplicable
    // posture. Guards: literal receiver only; ASCII-only content
    // (clippy's own scope, measured silent on non-ASCII); NO `\u{`
    // escape (clippy's suggestion for the shape is `b"\u{41}"` —
    // INVALID RUST, the upstream suggestion bug we are deliberately
    // stricter than); raw strings prefix `b` before `r` (measured:
    // `br"a\nc"`); escapes ride verbatim. NURSERY-group
    // (allow-by-default; pinned by the plan152 probe's default-groups
    // arm — SILENT at default level, the Batch 160 miner comment's
    // "default-level perf" was wrong on both counts).
    "string_lit_as_bytes",
    // ── Batch 162 (the frontier-lead mining pack VIII — Plan 153 — the
    // Batch 161 re-arm's leads, 2026-09-19) ──
    // `x.bytes().count()` → `x.len()` — FIXABLE by STD'S OWN DEFINITION
    // IDENTITY: `str::bytes()` yields exactly the `as_bytes()` elements
    // and `count()` on an ExactSizeIterator is `len()`, so the chain IS
    // `as_bytes().len()` — which for `str` IS `len()` (the same u8
    // slice); `&[u8]::bytes()` re-yields the slice's own items likewise.
    // Bounded to the EXACT two-call chain — `.bytes().count()`, no args,
    // no turbofish, no adapter between (the adapter-class shapes refuse;
    // `chars().count()` is NOT the family — chars ≠ bytes).
    // Complexity-group (default-on; group pinned by the plan152 probe's
    // full-oracle arm + the lint's upstream group).
    "bytes_count_to_len",
    // `x == true` → `x`, `x != true` → `!x` (+ the `false` twins and both
    // operand orders) — FIXABLE by BOOLEAN ALGEBRA on a proven-`bool`
    // operand: the equality/inequality with a literal is the
    // identity/negation law of the two-element algebra, and Rust's bool is
    // exactly that (no third value, no NaN class). The non-literal operand
    // must be a BARE IDENT proven `bool` by the enclosing fn's signature
    // (the scalar_param_heads_over derivation admits `bool`; the
    // never-rebound guard is the soundness leg). Calls/fields/casts and
    // literal-only pairs refuse. Complexity-group (default-on; pinned by
    // the plan152 probe arm).
    "bool_comparison",
    // ── Batch 163 (the frontier-lead mining pack IX — Plan 154 — the
    // Batch 162 re-arm's recorded spares, 2026-09-19) ──
    // `String::from("")` → `String::new()` — FIXABLE by EMPTY-STRING
    // CONSTRUCTION IDENTITY: `String::from("")` is `"".to_owned()` = an
    // empty String over `Vec::with_capacity(0)` (no allocation) — the same
    // value, capacity, and cost as `String::new()`, no side effects on
    // either side; the rewrite mirrors clippy's own MachineApplicable
    // suggestion (probe-measured). Bounded to the 2-segment path
    // `String::from` + the explicit turbofish `<String as From<..>>::from`
    // (probe: clippy fires both) with the EMPTY literal arg only
    // (`from("x")`/`from(s)`/`let s = ""; from(s)` are SILENT upstream);
    // `From::from` (inference could be Cow/Box) and longer paths refuse;
    // `"".to_string()` fires upstream but is a different seam — refused
    // (documented narrower-than-clippy divergence). Pedantic-group
    // (allow-by-default; pinned by the plan154 probe's group arms — fires
    // under -W clippy::pedantic only; the bench oracle passes that flag).
    "manual_string_new",
    // `let x[: T]; x = v;` → `let x[: T] = v;` — FIXABLE by STATEMENT-
    // POSITION IDENTITY: the let initializer evaluates at exactly the
    // position the assignment statement evaluated (same position, same
    // temporaries, same drops — there is no intermediate value), and for a
    // NON-mut binding an adjacent assignment IS the initialization by the
    // language's own definition (E0384 forbids re-assignment). Bounded to
    // the ADJACENT pair (an intervening statement silences clippy —
    // probe-pinned); `mut` bindings refuse (clippy's own scope — its fix
    // would also drop the mut); plain-ident patterns only (destructure/
    // ref refuse); branch initializers refuse (the actionable upstream
    // suggestion there is useless_let_if_seq's — a different seam);
    // annotation + initializer re-emit verbatim. Style-group (default-on;
    // pinned by the plan154 probe's default-groups arm).
    "needless_late_init",
    // ── Batch 164 (the frontier-lead mining pack X — Plan 155 — the
    // coverage-report re-arm leads, 2026-09-19) ──
    // `match x { P => true, _ => false }` → `matches!(x, P)` (+ the `!`
    // twin + the arm-1 guard carried verbatim) — FIXABLE because the macro
    // IS that match: `matches!` expands to exactly the two-arm match, the
    // scrutinee evaluates once at the same position, and the pattern/guard
    // ride through `$pattern:pat` / `$guard:expr` unchanged. Guards:
    // exactly 2 arms, wildcard-only arm 2 with NO guard, bool-literal
    // bodies of opposite polarity, arm 1 not `_`, match/arm attrs refuse,
    // guard-on-wildcard refuses. Named catch-alls + 3-arm + non-bool shapes
    // are SILENT upstream (probe-pinned) and refuse — upstream-exact scope.
    // Style-group (default-on; pinned by the plan155 probe's no-flags arm).
    "match_like_matches_macro",
    // `2.141592653589793238462643f32` → `2.1415927f32` — FIXABLE by
    // SUFFIX-PINNED SHORTEST-ROUND-TRIP: the literal's suffix names its
    // type (no type check anywhere), `format!("{:?}")` is std's shortest
    // round-trip formatting, and the proposer RE-PARSES the shortened text
    // and compares bits — the edit lands only when the two literals are the
    // identical f32/f64 value (same suffix + same value ⇒ inference,
    // const-eval, consumers unchanged). Guards: suffix REQUIRED (upstream
    // also fires on annotation-typed literals — the documented
    // narrower-than-clippy divergence); digit separators refuse
    // (upstream-silent, unreadable_literal's territory); INTEGRAL values
    // refuse (`1.0000000f32` silent upstream — the check is on the VALUE's
    // fract, not the text, because `3.000…e-10f32` fires despite all-zero
    // mantissa fraction); exponent-presence must agree between original and
    // shortest; already-shortest literals refuse; the bit re-parse is the
    // soundness leg. Style-group (default-on; pinned by the plan155 probe's
    // no-flags arm).
    "excessive_precision",
    // ── Batch 167 (the coverage-report re-arm pack XI — Plan 158 — the
    // 2026-09-21 derivation leads, probe scripts/plan158_frontier_probe.py) ──
    // `x as T` → `x` on a proven-same-type ident; `5 as u64` → `5_u64` on
    // an int/float literal — FIXABLE by CONSTRUCTION IDENTITY: a same-type
    // cast changes neither value, type, nor place-expression status (the
    // identity cast), and an unsized literal can carry its type as a suffix
    // (`5 as u64` IS `5_u64` bit-and-type-identical). Guards: the operand
    // must be a BARE IDENT proven by the enclosing fn's signature with the
    // proven type text EXACTLY the cast target (cross-type casts fire
    // cast_lossless/needless_type_cast/cast_possible_wrap — different
    // lints, probe-pinned), OR a direct int/float literal (existing suffix
    // stripped to the canonical form; char/bool targets refuse — no suffix
    // spelling exists; unary/paren literals refuse). Param provenance only
    // in v1 — annotated-let widening is the later rider (the documented
    // narrower-than-clippy divergence). Complexity-group (default-on;
    // pinned by the plan158 probe's no-flags arm).
    "unnecessary_cast",
    // `x.clone()` → `x` on a proven-Copy receiver — FIXABLE by the std
    // Copy/Clone CONTRACT: Clone for a Copy type is required to be `*x`
    // ("should not be overridden"), so on Copy the call is a semantic
    // no-op with the same type/value/move semantics. Guards: receiver a
    // BARE IDENT proven by the enclosing fn's signature over the Copy
    // scalar set (all numeric primitives + bool + char + the shared-ref
    // spellings `&str`/`&String` — `&T` is Copy); `String` is NOT Copy and
    // refuses (the probe showed non-Copy clones fire redundant_clone — a
    // different value-flow lint); arrays/tuples/&mut are not in the
    // derivation and refuse; exactly `clone`, zero args, no turbofish;
    // chains/fields/index/call receivers refuse. Complexity-group
    // (default-on; pinned by the plan158 probe's no-flags arm).
    "clone_on_copy",
    // `/// docs` + blank line(s) + item → delete the blank run — the doc
    // re-attaches; pure whitespace deletion, zero semantic surface.
    // Guards: comment-style OUTER `///` only (`//!` refuses; `#[doc]`
    // attribute-form and `/** */` block docs are invisible via
    // `doc_lines`); the blank run must resolve to a non-doc, non-attr,
    // non-`}` line — the attr case is MEASURED upstream silence (the
    // plan159 probe: `/// docs` + blank + `#[derive]` never fires);
    // EOF-resolving runs refuse. Suspicious-group (default-on) — the
    // group's FIRST corpus row.
    "empty_line_after_doc_comments",
    // `impl W { pub fn new() -> Self { .. } }` with no Default → INSERT
    // `impl Default for W { fn default() -> Self { Self::new() } }` —
    // purely additive, byte-for-byte clippy's own suggestion shape; no
    // existing byte moves and every prior Default use was a compile
    // error. Guards: inherent impl, plain-ident self type, zero-arg
    // non-generic non-unsafe `pub fn new() -> Self` (const fn admitted);
    // no in-file `impl Default` / `derive(Default)` / inherent `default`
    // fn (inherent shadowing refused up front); generic impls refuse —
    // cross-file Default is the verify-caught E0119 revert, never
    // silent. Style-group (default-on).
    "new_without_default",
    // ── Batch 169 (the coverage-report pack XIII — Plan 160 — the
    // Batch-168 staged alternates, probe scripts/plan160_pack13_probe.py) ──
    // `match f { &A => .., &B => .. }` → `match *f { A => .., B => .. }` —
    // FIXABLE by CONSTRUCTION IDENTITY: on any compiling input the
    // all-`&`-armed scrutinee is a reference, and with BINDING-FREE
    // sub-patterns the deref-place match selects the same arms with zero
    // moves (no binding can move); byte-for-byte clippy's own
    // MachineApplicable suggestion (probe: `*f` + stripped patterns).
    // Guards: EVERY arm top-level `&`-pattern (the measured upstream
    // silences mirrored — bare `_` arm, mixed arm, or-pattern arm);
    // binding-free only (clippy fires on `&Some(ref x)` too — probe —
    // but the rewrite changes binding modes: the documented
    // narrower-than-clippy divergence, the staged scope); no nested `&`
    // (a `&&A` output would re-fire our own fixpoint); plain-ident
    // scrutinee only. Style-group (default-on; pinned by the plan160
    // probe's no-flags run).
    "match_ref_pats",
    // `(&v).into_iter()` → `(&v).iter()` (+ `(&mut v)` → `iter_mut`) —
    // FIXABLE by the std DELEGATION LAW: `impl IntoIterator for &T` IS
    // `self.iter()` for every container in the proven set (std defines
    // the trait by delegating to the inherent method), so the method
    // rename calls the SAME function on the SAME receiver — clippy's own
    // suggestion ("call directly: `iter`", receiver parens kept).
    // Guards: the PARENTHESIZED `(&e)` receiver only (the typeck
    // `r.into_iter()` shape clippy also catches is out of parse reach);
    // the receiver ident proven by the enclosing fn's PARAM typed
    // Vec/[T; N]/HashMap/HashSet/BTreeMap/BTreeSet/VecDeque/BinaryHeap
    // (behind at most one reference; the `param_heads_over` precedent,
    // never-rebound guard, closures refuse — shadowing); for-heads refuse
    // (the rename births explicit_iter_loop — a style-for-style trade,
    // the documented divergence); zero args, no turbofish. The `.count()`
    // co-fire (probe: `iter_count`) rides as the gen-8 shaped-verdict
    // class. Style-group (default-on; pinned by the plan160 probe's
    // no-flags run).
    "into_iter_on_ref",
    // Every TAB in a doc comment → four spaces. FIXABLE by construction:
    // comment bytes carry zero semantics and the replacement is clippy's
    // own per-occurrence suggestion shape (probe: one `\t` → '    ', two →
    // eight spaces; code samples inside fenced blocks fire too — mirrored
    // verbatim). Guards: doc-comment ATTRS only (item/impl/trait level +
    // the file-level `//!` — probe tid_inner fires, unlike
    // empty_line_after_doc_comments' outer-only scope), the `////` and
    // `/***` regular-comment prefixes refuse, the `#[doc = "..."]`
    // attribute form refuses (escape-encoded literals make byte-level tab
    // edits wrong — the documented divergence). Style-group (default-on;
    // pinned by the plan163 probe's no-flags run).
    "tabs_in_doc_comments",
    // `v.iter().map(|x| x.clone())` → `.cloned()` (+ the `|x| *x` →
    // `.copied()` twin). FIXABLE by the input's own compilation: the
    // clone shape PROVES T: Clone (`x.clone()` on `&T` is what cloned()
    // does per element), the deref shape PROVES T: Copy (a move out of a
    // shared ref only compiles for Copy — copied()'s exact bound) — the
    // adapter choice is TYPE-INDEPENDENT given the shape. clippy prefers
    // `copied()` for Copy clone-shapes; we emit `cloned()` always —
    // always-compilable, semantics-identical (the
    // diverge-from-the-literal-suggestion precedent). Guards: by-ref
    // adapter chain (iter/values/keys — probe-verified shapes only;
    // iter_mut unprobed, refused) with NO into_iter anywhere (probe:
    // clippy's empty suggestion there deletes the map — valid only for
    // Copy elements, an owned-String chain would erase a real clone);
    // single-param attr-free closures, body EXACTLY `p.clone()`/`*p`
    // (probe mc_nested: nested clones are SILENT — mirrored).
    // Perf-group (default-on; pinned by the plan163 probe's no-flags run).
    "map_clone",
    "len_without_is_empty",
    "cast_abs_to_unsigned",
    // ── Batch 184 (the staged IOU pair, Plan 179): the pointer-provenance
    // offset rewrite (usize-only operand = upstream's own firing
    // precondition, measured) + the one-`&`-deletion let shape (the match
    // shape stays match_ref_pats' — the measured co-fire). ──
    "ptr_offset_with_cast",
    "needless_borrowed_reference",
    // ── Batch 185 (the coverage-report pack XVII — Plan 180): the
    // subset-driven generic-bound consolidation (fn-only surface =
    // upstream parity, probe-measured; synthesized deletion — upstream
    // emits no replacement). ──
    "multiple_bound_locations",
    // ── Batch 191 (the coverage-report pack XVIII — Plan 188):
    // `std::mem::replace(dst, Default::default())` → `std::mem::take(dst)`
    // — FIXABLE by STD'S OWN DEFINITION IDENTITY: `take` is literally
    // defined as `replace(dest, T::default())` (the std source), and the
    // input's own compilation proves `T: Default` — zero type-blindness
    // (the `from_str_radix_10` / `bytes_count_to_len` construction-identity
    // class). Bounded to the two QUALIFIED path spellings
    // (`std::mem::replace` + the `mem::replace` import-verified form with
    // the legacy_numeric_constants shadow-guard — probe-measured upstream
    // SILENT on a local `mem::replace`, so the guard is parity); the
    // second arg EXACTLY the bare `Default::default()` path call (the
    // `<T as Default>::default()` turbofish is SILENT upstream — parity
    // refusal; `<_>::default()` FIRES upstream but refuses v1, the
    // documented narrower-than-clippy divergence). Bare `replace(..)`
    // FIRES upstream but refuses here — a syntax-only proposer cannot
    // prove the one-name import (a local `fn replace` would make the
    // rewrite broken code) — the SAFER-than-clippy divergence class.
    // Upstream's own suggestion is whole-call MachineApplicable
    // (probe-measured); we keep the author's path prefix where clippy
    // rewrites to the full `std::mem::take` spelling (the map_clone
    // cloned-over-copied precedent). Style-group (default-on; the
    // installed driver is the authority).
    "mem_replace_with_default",
    // ── Batch 228 (the coverage-report re-arm pack XIX — Plan 230): the
    // std definition-identity trio. manual_repeat_n: `repeat(x).take(n)` →
    // `repeat_n(x, n)` (the adapter is DEFINED as the expansion; the input's
    // compilation proves `T: Clone`; ⚠ MSRV floor — `iter::repeat_n`
    // stabilized 1.82, the rewrite fails to compile on older toolchains);
    // manual_ok_err / manual_next_back: the Option/Rev counterpart twins —
    // std-definition and adapter-definition identities respectively, both
    // eq-by-construction on the measured two-arm match / `.rev().next()`
    // shapes (probe plan230_pack19_probe.py VERDICT PASS). Groups per the
    // INSTALLED DRIVER (the lint_level gate's own derivation — the
    // plan-188 GROUP CORRECTION class): manual_repeat_n = STYLE,
    // manual_ok_err = COMPLEXITY, manual_next_back = style. ALL
    // default-on; the installed driver is the authority.
    "manual_repeat_n",
    "manual_ok_err",
    "manual_next_back",
    // ── Batch 230 (the clippy-1.99 new-lint wave — Plan 232; the
    // toolchain-move re-arm: upstream stable 1.98.1→1.99.0 shipped
    // 2026-09-28 with 8 new lints, 5 probe-admitted + the clone_on_copy
    // UFCS rider; probe scripts/plan232_batch230_probe/, isolated
    // +1.99.0 toolchain, 2026-10-13. Lint-level rows live in
    // lint_level.rs's AHEAD_OF_PIN_1_99 (the pinned 1.98.1 driver cannot
    // see them — the documented ahead-of-pin derivation) ──
    // assert_is_empty: `assert!(x.is_empty())` → `assert_eq!(x, <empty>)`
    // on signature-proven containers — pass-behavior identical, the
    // value-printing panic message IS the lint's purpose. Pedantic group.
    "assert_is_empty",
    // One module (bit_width.rs) serves BOTH 1.99 bit-width lints — the fix
    // is receiver-kind-driven either way (`<Ty>::BITS - x.leading_zeros()`
    // → `x.bit_width()` / `.cast_unsigned().bit_width()` / NonZero `.get()`;
    // bit_width() stabilized 1.97, the LINT ships 1.99 — clippy is
    // MSRV-aware so the lint only fires where the fix compiles (the
    // chunks_exact_to_as_chunks precedent).
    "manual_bit_width",
    "mismatched_bit_width_type",
    // The un-brace of a plain single-expression block scrutinee
    // (match/if-let/while-let) — construction identity; HELP-ONLY upstream
    // (no .fixed), the identity fix is ours. Suspicious group.
    "block_scrutinee",
    // `unsafe { NonNull::new_unchecked(Box::into_raw(p)) }` →
    // `NonNull::from_mut(Box::leak(p))` — same pointer, leak-on-both-sides,
    // safe-replaces-unsafe; only BOX_LEAK MSRV-gated upstream (ancient);
    // from_mut compile-verified at 1.98.1. Complexity group.
    "nonnull_unchecked_on_box_ptr",
    // ── P51a post-mining intake (2026-10-14 — the batch-230 self-heal
    // run's no-corpus-rule classes; specimens = the healer's own proposers,
    // cycle 71) ──
    // Adjacent arms with byte-identical bodies + binding-free patterns +
    // no guards → `p1 | p2 | .. => B`. Merging ADJACENT arms is
    // execution-equivalent by construction (exactly one arm runs either
    // way; the or-pattern tries alternatives in source order over the
    // same union of values) — the conservative subset of upstream's
    // overlap-analysis merge (non-adjacent runs, typed body equality,
    // same-guard merges, and the wildcard-DELETE suggestion all refuse).
    // Pedantic group. The three P51a siblings that are type-blind
    // (unnecessary_get_then_check, explicit_auto_deref,
    // used_underscore_binding) ride the --verify tier below.
    "match_same_arms",
    // ── P118 post-mining intake (2026-11-07, the outbound readiness re-scan
    // of the Mozilla quartet — the rca real-world diagnostics + the probe
    // matrix at clippy 1.99.0, plan 353) ──
    // Match-arm plain-if-no-else → guard form (`P => { if C { B } }` →
    // `P if C => B`) — the rca cyclomatic.rs:120-127 specimen; if-let arms
    // (the pattern-merge class) DIVERGE (binding-mode analysis is typeck
    // territory; the oracle lane covers when fed), if-with-else is
    // upstream-silent (upstream-exact guard).
    "collapsible_match",
    // `.map_or(d, |x| x)` → `.unwrap_or(d)` — the rca main.rs:67 specimen;
    // receiver proven Option/Result by the shared receiver_kind substrate
    // (the unnecessary_map_or P15 precedent — type-blind is E0599-unsafe).
    "map_or_identity",
    // `min`/`max` between two int literals folds to the winning literal —
    // the P116 residual (`100.min(97)` → `97`, riir-infer's gate sweep);
    // floats/idents/const paths refuse (upstream-silent, mirrored).
    "unnecessary_min_or_max",
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
        // The line is absorbed into the preceding quote/list item (Batch
        // 151). Render-stable in rustdoc, but if a SEPARATE paragraph was
        // intended, clippy's own help names the right fix — a blank line
        // (or the `\>` escape in quotes).
        "doc_lazy_continuation" => Some("paragraph join — the line is absorbed into the preceding quote/list item; if it was meant as its own paragraph, add a blank line instead (in quotes, escape with \\>)"),
        // Dedent to the item's content col (Batch 151). A deliberately
        // indented code-block continuation would be dedented too — verify
        // the line is prose (clippy's MaybeIncorrect hedge, same standing).
        "doc_overindented_list_items" => Some("indent dedent — the continuation joins the list item's paragraph; verify it is not a deliberately indented code block"),
        // Issue 149 T3 (2026-10-06, probed on clippy 0.1.98): the lint's live
        // firing surface narrowed to BOTH bounds LITERAL — variable bounds,
        // field-projection receivers, and every if/else clamp shape read
        // SILENT, so the auto-oracle anchors none of those and a
        // --repo-posture run declines the proposer's wider shape family
        // there. The proposer stays pattern-based and still applies the
        // wider shapes under plain --fix.
        "manual_clamp" => Some("clippy 0.1.98 firing surface — only `x.max(L).min(L)` chains with LITERAL bounds still fire; variable bounds and every if/else clamp shape are SILENT upstream (no oracle anchor, and --repo-posture declines those shapes)"),
        // Issue 149 T3 (2026-10-06, probed on clippy 0.1.98): fires iff the
        // receiver is a LOCAL IDENT and BOTH bounds are LITERAL; a variable
        // bound or a field-projection receiver (item.f >= a && item.f <= b)
        // is SILENT upstream — no oracle anchor, and --repo-posture declines
        // those shapes. The proposer stays pattern-based under plain --fix.
        "manual_range_contains" => Some("clippy 0.1.98 firing surface — fires only with a local-ident receiver and BOTH bounds literal; variable bounds and field-projection receivers are SILENT upstream (no oracle anchor, and --repo-posture declines those shapes)"),
        // The rewrite TARGETS a newer std API: `iter::repeat_n` stabilized
        // in Rust 1.82 — on an older toolchain the fix fails to compile
        // (the fix_verify loop auto-reverts it there). Eq-by-construction
        // otherwise (the adapter is defined as the expansion).
        "manual_repeat_n" => Some("MSRV floor — iter::repeat_n stabilized Rust 1.82; the rewrite needs a ≥1.82 toolchain (fix_verify auto-reverts on older)"),
        // ── Batch 230 (clippy-1.99-only lints — the pinned 1.98.1 driver
        // cannot fire any of these; see lint_level.rs AHEAD_OF_PIN_1_99) ──
        "assert_is_empty" => Some("clippy 1.99 lint, pedantic group — fires only in crates enabling pedantic; the panic message CHANGES to print the asserted value (that difference is the lint's purpose)"),
        "manual_bit_width" => Some("clippy 1.99 lint; bit_width() stabilized Rust 1.97 — clippy is MSRV-aware so the lint fires only where the fix compiles"),
        "mismatched_bit_width_type" => Some("clippy 1.99 lint; the fix follows the RECEIVER's type kind — if the mismatched LHS type was intentional, decline the edit (the lint reads it as a probable bug)"),
        "block_scrutinee" => Some("clippy 1.99 lint, help-only upstream; the un-brace is construction-identity — multi-statement blocks and unsafe/async wrappers refuse"),
        "nonnull_unchecked_on_box_ptr" => Some("clippy 1.99 lint; same pointer value and leak-on-both-sides ownership, safe-replaces-unsafe — a SAFETY comment above the block refuses (orphaned justification)"),
        _ => None,
    }
}

/// Lints attempted ONLY under `cargo refine --fix --verify` (Issue 015) —
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
    // P12 post-mining intake (2026-09-14, the 2026-09-09 nursery-gap usage
    // artifact — ~80 findings in one riir-ai sweep): `&mut self` → `&self`
    // on non-`pub` inherent-impl methods with read-only bodies (+ the
    // same-file `let mut` caller cascade). VERIFY: the lint is nursery /
    // default-`allow` (it only appears under an explicit `-W` stream —
    // the `--clippy-json` oracle lane), real clippy's suggestion is
    // applicability `Unspecified` (`--fix` applied 0 — probed), and body
    // "never mutates self" is approximated parse-only (std-`&self`
    // approximated parse-only (std-`&self`
    // allowlist + same-file unique-&self resolution) where clippy is
    // type-aware — every wrong fire is an E0596/E0384 check-visible
    // compile error the gate reverts.
    "needless_pass_by_ref_mut",
    // ── Batch 150 (the staged default-level + pedantic probe pack) ──
    // `x == None` → `x.is_none()` (+ `!= None` → `is_some()`, the
    // `&None` and `Option::None` forms). VERIFY: type-blind — a non-Option
    // receiver with a custom `PartialEq<Option>` impl has no `is_none`
    // (E0599, check-visible, the gate reverts); real clippy is type-aware
    // and stays silent there (probe-verified). Receiver safety: postfix
    // receivers go bare, everything else parenthesized (method call binds
    // tighter than any binary operator). MachineApplicable in real clippy
    // → the P21 oracle lane consumes the residue shapes.
    "partialeq_to_none",
    // `x as u64` → `u64::from(x)` where provably widening. VERIFY via the
    // SAFE-LET gate: the operand type is only followed from an EXPLICIT
    // annotation (unique `let x: T = …` binding, or a uniquely-named fn
    // param); the rewrite emits only when `From<Src> for Dst` is one of
    // std's primitive impls (explicit table — incl. the measured
    // sign-change u32→i64 case), so the rewrite is value-identical to the
    // `as` cast whenever it compiles. A second binding of the same name
    // A second binding of the same name
    // refuses (shadow). All other operands (fields, generics, complex
    // exprs) are the oracle lane's residue (clippy's suggestion is
    // MachineApplicable there); a wrong fire is E0277 — check-visible.
    "cast_lossless",
    // ── Batch 155 (the frontier-lead mining pack, Plan 146) ──
    // `&Vec<T>`/`&String` params → `&[T]`/`&str`. VERIFY: type-blind callee
    // + body shapes (Vec-only methods like `capacity()`; `v` passed to a
    // `&Vec`-expecting callee) are E0308/E0599 — check-visible, the compile
    // gate reverts. The mut/lifetime/attr forms refuse up front; the inner
    // type is spliced verbatim from the pointee's own span.
    "ptr_arg",
    // ── Batch 156 (the frontier-lead mining pack II, Plan 147) ──
    // `fn into_*(&self)` → `fn into_*(self)`. VERIFY: the receiver flip is
    // type-blind (body borrows self past the move → E0515; callers passing
    // borrows → E0308/E0382) — all check-visible, the compile gate reverts
    // (the ptr_arg precedent). Inherent impls, `into_` prefix, no-vis
    // methods only (clippy's avoid_breaking_exported_api default skips
    // exported items, and the API break compiles — refused up front).
    "wrong_self_convention",
    // ── P51a post-mining intake (2026-10-14, the batch-230 self-heal run's
    // no-corpus-rule classes; the specimens are the healer's own proposers) ──
    // `m.get(k).is_none()` → `!m.contains_key(k)` (+ the `is_some` twin).
    // VERIFY: the receiver is type-blind (clippy type-checks for the four
    // std map/set types and stays SILENT on `&map` receivers — probe-
    // verified; we fire wider, the divergences disclosed in the module
    // doc: slice/Vec/set receivers are E0599 the compile gate reverts,
    // the custom-get-vs-contains_key class is the pathological residual).
    "unnecessary_get_then_check",
    // `f(&*e)` → `f(&e` / `f(&**e)` → `f(e)` (argument position only; the
    // deref-COUNT selects the splice — both MachineApplicable upstream,
    // probe-verified). VERIFY: concrete callees are proof-safe (the
    // original compiling proves the coercion chain); the GENERIC-callee
    // class binds T differently and is SILENT — the disclosed residual,
    // bounded by how rare `&*` args to generic callees are (both cycle-71
    // specimens were concrete-typed helpers).
    "explicit_auto_deref",
    // `_x` (used) → `x` at the binding + every use (multi-span rename,
    // upstream is diagnosis-only in 1.98.1 — the rewrite is ours).
    // VERIFY: the multi-span P15 structural class — the file-level
    // collision census + macro-token + shorthand guards make the rename
    // resolution-preserving; per-fn scoping keeps UNUSED `_x` bindings
    // untouched (renaming them would birth unused-var warnings).
    "used_underscore_binding",
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

/// Dispatch to one lint's proposer. All 86 lints are reachable here (the
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
        // ── P15 post-mining intake (2026-09-14, the born-lint closure) ──
        "unnecessary_map_or" => unnecessary_map_or::collect_edits(source, file),
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
        // ── P12 post-mining intake (2026-09-14, the 2026-09-09 nursery-gap row) ──
        "needless_pass_by_ref_mut" => needless_pass_by_ref_mut::collect_edits(source, file),
        // ── Batch 150 (the staged default-level + pedantic probe pack) ──
        "print_with_newline" => print_write_with_newline::collect_edits(source, file),
        "write_with_newline" => print_write_with_newline::collect_writeln_edits(source, file),
        "unreadable_literal" => unreadable_literal::collect_edits(source, file),
        "legacy_numeric_constants" => legacy_numeric_constants::collect_edits(source, file),
        "partialeq_to_none" => partialeq_to_none::collect_edits(source, file),
        "cast_lossless" => cast_lossless::collect_edits(source, file),
        // ── Issue 120 intake (2026-09-16, the pedantic doc-lint family) ──
        "doc_markdown" => doc_markdown::collect_edits(source, file),
        // ── Batch 151 (the doc-line state-machine port, Plan 137) ──
        "doc_lazy_continuation" => doc_lazy_continuation::collect_edits(source, file),
        "doc_overindented_list_items" => {
            doc_lazy_continuation::collect_overindent_edits(source, file)
        }
        // ── P31 post-mining intake (2026-09-17, the riir-chain 51b456f MISS artifact) ──
        "manual_range_patterns" => manual_range_patterns::collect_edits(source, file),
        // ── Batch 155 (the frontier-lead mining pack, Plan 146) ──
        "let_and_return" => let_and_return::collect_edits(source, file),
        "ptr_arg" => ptr_arg::collect_edits(source, file),
        "redundant_slicing" => redundant_slicing::collect_edits(source, file),
        "map_flatten" => map_flatten::collect_edits(source, file),
        // ── Batch 156 (the frontier-lead mining pack II, Plan 147) ──
        "io_other_error" => io_other_error::collect_edits(source, file),
        "wrong_self_convention" => wrong_self_convention::collect_edits(source, file),
        // ── Batch 157 (the frontier-lead mining pack III, Plan 148) ──
        "redundant_closure_for_method_calls" => {
            redundant_closure_for_method_calls::collect_edits(source, file)
        }
        "iter_skip_next" => iter_skip_next::collect_edits(source, file),
        // ── Batch 158 (the frontier-lead mining pack IV, Plan 149) ──
        "to_string_in_format_args" => to_string_in_format_args::collect_edits(source, file),
        "manual_range_contains" => manual_range_contains::collect_edits(source, file),
        // ── Batch 159 (the frontier-lead mining pack V, Plan 150) ──
        "single_char_add_str" => single_char_add_str::collect_edits(source, file),
        "manual_clamp" => manual_clamp::collect_edits(source, file),
        // ── Batch 160 (the frontier-lead mining pack VI — Plan 151) ──
        "from_str_radix_10" => from_str_radix_10::collect_edits(source, file),
        "assign_op_pattern" => assign_op_pattern::collect_edits(source, file),
        // ── Batch 161 (the frontier-lead mining pack VII — Plan 152) ──
        "unit_arg" => unit_arg::collect_edits(source, file),
        "string_lit_as_bytes" => string_lit_as_bytes::collect_edits(source, file),
        // ── Batch 162 (the frontier-lead mining pack VIII — Plan 153) ──
        "bytes_count_to_len" => bytes_count_to_len::collect_edits(source, file),
        "bool_comparison" => bool_comparison::collect_edits(source, file),
        // ── Batch 163 (the frontier-lead mining pack IX — Plan 154) ──
        "manual_string_new" => manual_string_new::collect_edits(source, file),
        "needless_late_init" => needless_late_init::collect_edits(source, file),
        // ── Batch 164 (the frontier-lead mining pack X — Plan 155) ──
        "match_like_matches_macro" => match_like_matches_macro::collect_edits(source, file),
        "excessive_precision" => excessive_precision::collect_edits(source, file),
        // ── Batch 167 (the coverage-report re-arm pack XI — Plan 158) ──
        "unnecessary_cast" => unnecessary_cast::collect_edits(source, file),
        "clone_on_copy" => clone_on_copy::collect_edits(source, file),
        // ── Batch 168 (the coverage-report re-arm pack XII — Plan 159) ──
        "empty_line_after_doc_comments" => {
            empty_line_after_doc_comments::collect_edits(source, file)
        }
        "new_without_default" => new_without_default::collect_edits(source, file),
        // ── Batch 169 (the coverage-report pack XIII — Plan 160) ──
        "match_ref_pats" => match_ref_pats::collect_edits(source, file),
        "into_iter_on_ref" => into_iter_on_ref::collect_edits(source, file),
        // ── Batch 171 (the coverage-report pack XIV — Plan 163) ──
        "tabs_in_doc_comments" => tabs_in_doc_comments::collect_edits(source, file),
        "map_clone" => map_clone::collect_edits(source, file),
        // ── Batch 175 (the coverage-report pack XV — Plan 169) ──
        "len_without_is_empty" => len_without_is_empty::collect_edits(source, file),
        "cast_abs_to_unsigned" => cast_abs_to_unsigned::collect_edits(source, file),
        // ── Batch 184 (the coverage-report pack XVI — Plan 179, the
        // staged IOU pair) ──
        "ptr_offset_with_cast" => ptr_offset_with_cast::collect_edits(source, file),
        "needless_borrowed_reference" => needless_borrowed_reference::collect_edits(source, file),
        // ── Batch 185 (the coverage-report pack XVII — Plan 180) ──
        "multiple_bound_locations" => multiple_bound_locations::collect_edits(source, file),
        // ── Batch 191 (the coverage-report pack XVIII — Plan 188) ──
        "mem_replace_with_default" => mem_replace_with_default::collect_edits(source, file),
        // ── Batch 228 (the coverage-report re-arm pack XIX — Plan 230) ──
        "manual_repeat_n" => manual_repeat_n::collect_edits(source, file),
        "manual_ok_err" => manual_ok_err::collect_edits(source, file),
        "manual_next_back" => manual_next_back::collect_edits(source, file),
        // ── Batch 230 (the clippy-1.99 new-lint wave — Plan 232) ──
        "assert_is_empty" => assert_is_empty::collect_edits(source, file),
        "manual_bit_width" | "mismatched_bit_width_type" => bit_width::collect_edits(source, file),
        "block_scrutinee" => block_scrutinee::collect_edits(source, file),
        "nonnull_unchecked_on_box_ptr" => nonnull_unchecked_on_box_ptr::collect_edits(source, file),
        // ── P51a post-mining intake (2026-10-14, the batch-230 self-heal
        // run's no-corpus-rule classes) ──
        "unnecessary_get_then_check" => unnecessary_get_then_check::collect_edits(source, file),
        "explicit_auto_deref" => explicit_auto_deref::collect_edits(source, file),
        "match_same_arms" => match_same_arms::collect_edits(source, file),
        // ── P118 post-mining intake (2026-11-07, plan 353) ──
        "collapsible_match" => collapsible_match::collect_edits(source, file),
        "map_or_identity" => map_or_identity::collect_edits(source, file),
        "unnecessary_min_or_max" => unnecessary_min_or_max::collect_edits(source, file),
        "used_underscore_binding" => used_underscore_binding::collect_edits(source, file),
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
            // P12 post-mining intake: nursery default-allow + parse-only body
            // analysis — VERIFY-tier only.
            "needless_pass_by_ref_mut",
            // Batch 155: type-blind callee/body shapes (Vec-only methods,
            // &Vec-expecting callees) — VERIFY-tier only.
            "ptr_arg",
            // Batch 156: the receiver flip's body/caller shapes are type-blind
            // (E0515/E0308/E0382) — VERIFY-tier only.
            "wrong_self_convention",
        ] {
            assert!(
                !FIXABLE_LINTS.contains(&excluded),
                "{excluded} must not be --fix eligible"
            );
        }
        // 6 seed + 27 Issue 016 structural rows + 1 Batch 89 use_self
        // + 2 Batch 90 (items_after_statements + manual_let_else)
        // + 2 Batch 91 (byte_char_slices + manual_slice_fill)
        // + 1 P15 unnecessary_map_or (classified-receiver-only terminal
        // forms — the born-lint closure, 2026-09-14)
        // + 1 Issue 120 doc_markdown (comment-local backtick wrap —
        // cannot change compilation, 2026-09-16).
        // + 1 P31 manual_range_patterns (verbatim-boundary set-identical
        // rewrite — the riir-chain 51b456f MISS artifact, 2026-09-17).
        // + 4 Batch 150 (print_with_newline + write_with_newline +
        // unreadable_literal + legacy_numeric_constants — the staged
        // default-level + pedantic probe pack, probe-first per P10/P11;
        // the two VERIFY rows carry their own module fixtures).
        // + 2 Batch 151 (doc_lazy_continuation + doc_overindented_list_items
        // — the doc-line state-machine port, one module both lints, the
        // probe151 boundary matrix in Plan 137).
        // + 3 Batch 155 (let_and_return + redundant_slicing + map_flatten —
        // the frontier-lead mining pack, Plan 146; each family fixture
        // oracle-verified to fire exactly its lint, 2026-09-18).
        // + 1 Batch 156 (io_other_error — the frontier-lead pack II, Plan
        // 147; inference-identical `Error::other` splice with the
        // legacy_numeric_constants shadow-guard pattern).
        // + 2 Batch 157 (redundant_closure_for_method_calls + iter_skip_next
        // — the frontier-lead pack III, Plan 148; the signature-proven
        // element type + construction-identity whitelist, and the
        // two-arm skip/nth rule with the oracle-clean `.get` slice arm).
        // + 2 Batch 158 (to_string_in_format_args + manual_range_contains
        // — the frontier-lead pack IV, Plan 149; the signature-proven
        // Display-blanket scalar receiver with the bare-`{}` placeholder
        // guard, and the contains rewrite closed by the same scalar
        // derivation + literal bounds).
        // + 2 Batch 159 (single_char_add_str + manual_clamp — the
        // frontier-lead pack V, Plan 150; the mut-string derivation twin
        // with the VALUE-based single-char arg + escape_default emit, and
        // the lattice-absorption clamp rewrite with the NaN-float +
        // inverted-bounds panic guards).
        // + 2 Batch 160 (from_str_radix_10 + assign_op_pattern — the
        // frontier-lead pack VI, Plan 151; the construction-identity parse
        // rewrite + the exactly-once-binding derivation compound-assign
        // rewrite with the commutative-swapped boundary).
        // + 2 Batch 161 (unit_arg + string_lit_as_bytes — the frontier-lead
        // pack VII, Plan 152; the syntactic-unit-typeness hoist (clippy's
        // own suggestion shape, measured) + the coercion-identity byte
        // literal with the `\u{` upstream-suggestion-bug refusal).
        // + 2 Batch 162 (bytes_count_to_len + bool_comparison — the
        // frontier-lead pack VIII, Plan 153; the std-definition-identity
        // chain collapse + the boolean-algebra literal comparison on a
        // proven-bool ident).
        // + 2 Batch 163 (manual_string_new + needless_late_init — the
        // frontier-lead pack IX, Plan 154; the empty-String construction
        // identity + the adjacent non-mut let/assign merge).
        // + 2 Batch 164 (match_like_matches_macro + excessive_precision —
        // the frontier-lead pack X, Plan 155; the macro-IS-the-match
        // verbatim rebuild + the suffix-pinned shortest-round-trip literal
        // rewrite with the in-proposer bit-identity proof).
        // + 2 Batch 167 (unnecessary_cast + clone_on_copy — the
        // coverage-report re-arm pack XI, Plan 158; the identity-cast
        // drop + literal-suffix substitution + the Copy/Clone-contract
        // receiver no-op).
        // + 2 Batch 168 (empty_line_after_doc_comments +
        // new_without_default — the coverage-report re-arm pack XII,
        // Plan 159; the blank-run deletion (suspicious-group's first
        // corpus row) + the additive Default-impl insertion).
        // + 2 Batch 169 (match_ref_pats + into_iter_on_ref — the
        // coverage-report pack XIII, Plan 160; the deref-scrutinee `&`-strip
        // + the container-proven method rename).
        // + 2 Batch 171 (tabs_in_doc_comments + map_clone — the
        // coverage-report pack XIV, Plan 163; the comment-byte tab spacing
        // + the shape-proven adapter rename).
        // + 2 Batch 175 (len_without_is_empty + cast_abs_to_unsigned — the
        // coverage-report pack XV, Plan 169; the additive is_empty insertion
        // (the NWD insertion precedent, overturned Plan-163 refusal) + the
        // signature-proven unsigned_abs rewrite in three measured regimes
        // (the Plan-149 provenance law, overturned Plan-163 typeck
        // refusal)).
        // + 2 Batch 184 (ptr_offset_with_cast + needless_borrowed_reference
        // — the coverage-report pack XVI, Plan 179; the Plan-169 staged IOU
        // pair — the pointer-provenance offset rewrite + the one-&-deletion
        // let shape).
        // + 1 Batch 185 (multiple_bound_locations — the coverage-report
        // pack XVII, Plan 180; the subset-driven generic-bound
        // consolidation, MBL solo pack).
        // + 1 Batch 191 (mem_replace_with_default — the coverage-report
        // pack XVIII, Plan 188; the std definition-identity take()
        // rewrite).
        // + 3 Batch 228 (the coverage-report re-arm pack XIX, Plan 230:
        // manual_repeat_n + manual_ok_err + manual_next_back — the
        // std/adapter definition-identity trio from the fresh 0.1.98
        // derivation's evidence tail).
        // + 5 Batch 230 (the clippy-1.99 new-lint wave, Plan 232 — the
        // toolchain-move re-arm: assert_is_empty + manual_bit_width +
        // mismatched_bit_width_type + block_scrutinee +
        // nonnull_unchecked_on_box_ptr; the clone_on_copy UFCS rider is
        // count-neutral).
        // P118 (2026-11-07, plan 353 — the outbound re-scan's corpus gaps):
        // + 3 (collapsible_match + map_or_identity + unnecessary_min_or_max
        // — the rca real-world specimens + the P116 riir-infer residual;
        // FIXABLE 90→93).
    }

    /// Every proposer lint round-trips through the shared dispatch (fires on
    /// its fixture + output re-parses — the L2 pruner check `--fix` runs).
    /// 104-row fixture table — length is the point.
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
            // ── P118 post-mining intake (2026-11-07, plan 353) ──
            (
                "collapsible_match",
                "fn log(n: u8) {}\nfn f(n: u8, w: fn(u8) -> bool) { match n { 1 => log(1), 2 => { if w(n) { log(2) } }, _ => {} } }",
                "2 if w(n)",
            ),
            (
                "map_or_identity",
                "fn f(r: Result<u8, String>) -> u8 { r.map_or(0, |x| x) }",
                "r.unwrap_or(0)",
            ),
            (
                "unnecessary_min_or_max",
                "fn f() -> u32 { 100u32.min(97) }",
                "{ 97 }",
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
            // ── P12 post-mining intake (2026-09-14, the 2026-09-09 nursery-gap row) ──
            (
                "needless_pass_by_ref_mut",
                "struct C { v: u32 }\nimpl C {\n    fn peek(&mut self) -> u32 { self.v }\n    fn demo() -> u32 { let mut c = C { v: 1 }; c.peek() }\n}\n",
                "fn peek(&self)",
            ),
            // ── Issue 120 intake (2026-09-16, the pedantic doc-lint family) ──
            (
                "doc_markdown",
                "/// Runs the shader_perf stage over a HashMap of buckets.\nfn stage() {}",
                "`shader_perf`",
            ),
            // ── P31 post-mining intake (2026-09-17, the riir-chain 51b456f MISS artifact) ──
            (
                "manual_range_patterns",
                "fn f(n: u32) -> u8 { match n { 0 | 1 | 2 => 9, _ => 0 } }",
                "0..=2 => 9",
            ),
            // ── Batch 150 (the staged default-level + pedantic probe pack) ──
            (
                "print_with_newline",
                "fn f() { print!(\"ready\\n\"); }",
                "println!(\"ready\")",
            ),
            (
                "write_with_newline",
                "fn f(f: &mut String) { write!(f, \"line\\n\"); }",
                "writeln!(f, \"line\")",
            ),
            ("unreadable_literal", "fn f() { let pages = 1234567; }", "1_234_567"),
            (
                "legacy_numeric_constants",
                "fn f() { let cap = std::u32::MAX; }",
                "u32::MAX",
            ),
            (
                "partialeq_to_none",
                "fn check(x: Option<u32>) -> bool { x == None }",
                "x.is_none()",
            ),
            (
                "cast_lossless",
                "fn widen(x: u8) -> u64 { x as u64 }",
                "u64::from(x)",
            ),
            // ── Batch 151 (the doc-line state-machine port, Plan 137) ──
            (
                "doc_lazy_continuation",
                "/// > The quote paragraph.\n/// lazy continuation line.\nfn quoted() {}",
                "> lazy continuation line.",
            ),
            (
                "doc_overindented_list_items",
                "/// - the item text.\n///     an overindented cont.\nfn listed() {}",
                "///   an overindented cont.",
            ),
            // ── Batch 155 (the frontier-lead mining pack, Plan 146) ──
            (
                "let_and_return",
                "fn shout(s: &str) -> String { let out = s.to_uppercase(); out }",
                "fn shout(s: &str) -> String { s.to_uppercase() }",
            ),
            (
                "ptr_arg",
                "fn total(v: &Vec<i32>) -> i32 { v.iter().sum() }",
                "fn total(v: &[i32]) -> i32",
            ),
            (
                "redundant_slicing",
                "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: &str) -> char { first_char(&s[..]) }",
                "first_char(s)",
            ),
            (
                "map_flatten",
                "fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().map(|x| x.split(' ')).flatten().collect() }",
                "xs.iter().flat_map(|x| x.split(' '))",
            ),
            // ── Batch 156 (the frontier-lead mining pack II, Plan 147) ──
            (
                "io_other_error",
                "fn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e) }",
                "std::io::Error::other(e)",
            ),
            (
                "wrong_self_convention",
                "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}",
                "fn into_label(self)",
            ),
            // ── Batch 157 (the frontier-lead mining pack III, Plan 148) ──
            (
                "redundant_closure_for_method_calls",
                "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }",
                "v.iter().map(String::len)",
            ),
            (
                "iter_skip_next",
                "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n).next() }",
                "v.get(n)",
            ),
            // ── Batch 158 (the frontier-lead mining pack IV, Plan 149) ──
            (
                "to_string_in_format_args",
                "fn show(x: u32) -> String { format!(\"n={}\", x.to_string()) }",
                "format!(\"n={}\", x)",
            ),
            (
                "manual_range_contains",
                "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x <= 5 }",
                "(3..=5).contains(&x)",
            ),
            // ── Batch 159 (the frontier-lead mining pack V, Plan 150) ──
            (
                "single_char_add_str",
                "fn mark(s: &mut String) { s.push_str(\"!\") }",
                "s.push('!')",
            ),
            (
                "manual_clamp",
                "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(3).min(5) }",
                "x.clamp(3, 5)",
            ),
            // ── Batch 160 (the frontier-lead mining pack VI, Plan 151) ──
            (
                "from_str_radix_10",
                "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(s, 10) }",
                "s.parse::<u32>()",
            ),
            (
                "assign_op_pattern",
                "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = y + 1; y }",
                "y += 1",
            ),
            // ── Batch 161 (the frontier-lead mining pack VII, Plan 152) ──
            (
                "unit_arg",
                "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }",
                "touch((), hits)",
            ),
            (
                "string_lit_as_bytes",
                "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\".as_bytes() }",
                "b\"abc\"",
            ),
            // ── Batch 162 (the frontier-lead mining pack VIII, Plan 153) ──
            (
                "bytes_count_to_len",
                "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().count() }",
                "s.len()",
            ),
            (
                "bool_comparison",
                "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == true }",
                "hits.push(x); x }",
            ),
            // ── Batch 163 (the frontier-lead mining pack IX, Plan 154) ──
            (
                "manual_string_new",
                "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(\"\") }",
                "String::new()",
            ),
            (
                "needless_late_init",
                "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x + x }",
                "let x = 5u32;",
            ),
            // ── Batch 164 (the frontier-lead mining pack X, Plan 155) ──
            (
                "match_like_matches_macro",
                "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, _ => false } }",
                "matches!(x, 0)",
            ),
            (
                "excessive_precision",
                "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 2.141592653589793238462643f32 }",
                "2.1415927f32",
            ),
            (
                "unnecessary_cast",
                "fn f(hits: &mut Vec<u8>, p: u32) -> u32 { hits.push(0); p as u32 }",
                "hits.push(0); p }",
            ),
            (
                "clone_on_copy",
                "fn f(hits: &mut Vec<u8>, x: u32) -> u32 { hits.push(0); x.clone() }",
                "hits.push(0); x }",
            ),
            (
                "empty_line_after_doc_comments",
                "/// Adds one.\n\nfn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); 1 }",
                "/// Adds one.\nfn f(hits",
            ),
            (
                "new_without_default",
                "pub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } }",
                "impl Default for W",
            ),
            (
                "match_ref_pats",
                "pub enum Fruit { Apple, Pear }\nfn f(hits: &mut Vec<u8>, x: &Fruit) -> u64 { hits.push(0); match x { &Fruit::Apple => 1, &Fruit::Pear => 2 } }",
                "match *x { Fruit::Apple => 1, Fruit::Pear => 2 }",
            ),
            (
                "into_iter_on_ref",
                "fn f(hits: &mut Vec<u8>, v: Vec<u64>) -> Option<u64> { hits.push(0); (&v).into_iter().next().copied() }",
                "v.iter().next()",
            ),
            (
                "tabs_in_doc_comments",
                "///\ttabbed doc\nfn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 1 }",
                "///    tabbed doc",
            ),
            (
                "map_clone",
                "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Vec<u8> { hits.push(0); v.iter().map(|x| *x).collect() }",
                "v.iter().copied()",
            ),
            // ── Batch 175 (the coverage-report pack XV, Plan 169) ──
            (
                "len_without_is_empty",
                "pub struct Bag { items: Vec<u8> }\nimpl Bag {\n    pub fn len(&self) -> usize {\n        self.items.len()\n    }\n}",
                "pub fn is_empty(&self) -> bool",
            ),
            (
                "cast_abs_to_unsigned",
                "fn f(hits: &mut Vec<u8>, x: i64) -> u64 { hits.push(0); x.abs() as u64 }",
                "hits.push(0); x.unsigned_abs() }",
            ),
            // ── Batch 185 (the coverage-report pack XVII — Plan 180) ──
            (
                "multiple_bound_locations",
                "pub fn m<T: Default + Clone>(t: T) -> T\nwhere\n    T: Default + Clone,\n{\n    t.clone()\n}",
                "pub fn m<T>(t: T)",
            ),
            // ── Batch 191 (the coverage-report pack XVIII — Plan 188) ──
            (
                "mem_replace_with_default",
                "fn t(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, Default::default())); }",
                "std::mem::take(dst)",
            ),
            // ── Batch 228 (the coverage-report re-arm pack XIX — Plan 230) ──
            (
                "manual_repeat_n",
                "fn t(n: usize) -> Vec<u32> { std::iter::repeat(0u32).take(n).collect() }",
                "std::iter::repeat_n(0u32, n)",
            ),
            (
                "manual_ok_err",
                "fn t(r: Result<u32, String>) -> Option<u32> { match r { Ok(v) => Some(v), Err(_) => None } }",
                "r.ok()",
            ),
            (
                "manual_next_back",
                "fn t(vals: &[u32]) -> Option<u32> { vals.iter().rev().next().copied() }",
                ".next_back()",
            ),
            // ── Batch 230 (the clippy-1.99 new-lint wave — Plan 232) ──
            (
                "assert_is_empty",
                "fn t(v: &Vec<u32>) { assert!(v.is_empty()); }",
                "assert_eq!(v.as_slice(), [])",
            ),
            (
                "manual_bit_width",
                "fn t(w: u32) -> u32 { u32::BITS - w.leading_zeros() }",
                "w.bit_width()",
            ),
            (
                "mismatched_bit_width_type",
                "fn t(w: u8) -> u32 { u32::BITS - w.leading_zeros() }",
                "w.bit_width()",
            ),
            (
                "block_scrutinee",
                "fn t(o: Option<u32>) -> u32 { match { o.unwrap() } { n => n + 1 } }",
                "match o.unwrap()",
            ),
            (
                "nonnull_unchecked_on_box_ptr",
                "fn t(one: Box<u32>) -> std::ptr::NonNull<u32> { unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(one)) } }",
                "NonNull::from_mut(Box::leak(one))",
            ),
            // ── P51a post-mining intake (2026-10-14, the batch-230 self-heal
            // run's no-corpus-rule classes) ──
            (
                "unnecessary_get_then_check",
                "fn f(m: std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get(&k).is_none() }",
                "!m.contains_key(&k)",
            ),
            (
                "explicit_auto_deref",
                "struct EadM { field: Box<u8> } fn f(m: &EadM) -> u8 { ident(&*m.field) } fn ident(e: &u8) -> u8 { *e }",
                "ident(&m.field)",
            ),
            (
                "match_same_arms",
                "fn f(x: &str) -> u8 { match x { \"a\" => 1, \"b\" => 1, _ => 2 } }",
                "\"a\" | \"b\" => 1",
            ),
            (
                "used_underscore_binding",
                "fn f(segs: &[String]) -> bool { let [_root, _mod_name, c, m] = segs else { return false }; _mod_name == \"clone\" && c == m }",
                "mod_name == \"clone\"",
            ),
            // ── P118 post-mining intake (2026-11-07, plan 353) ──
            (
                "collapsible_match",
                "fn log(n: u8) {}\nfn f(n: u8, w: fn(u8) -> bool) { match n { 1 => log(1), 2 => { if w(n) { log(2) } }, _ => {} } }",
                "2 if w(n)",
            ),
            (
                "map_or_identity",
                "fn f(r: Result<u8, String>) -> u8 { r.map_or(0, |x| x) }",
                "r.unwrap_or(0)",
            ),
            (
                "unnecessary_min_or_max",
                "fn f() -> u32 { 100u32.min(97) }",
                "{ 97 }",
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
