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
//! alone guarantees the rewrite preserves semantics) are `--fix` eligible:
//!
//! | Lint | Detection |
//! |---|---|
//! | `needless_return` | trailing `return expr;` → tail `expr` (syntax-local) |
//! | `redundant_closure` | `\|x\| f(x)` → `f` (same arg count, structural) |
//! | `redundant_field_names` | `Foo { x: x }` → `Foo { x }` (structural) |
//! | `manual_map` | exact `match`/`Some`/`None` shape (structural) |
//! | `unnecessary_unwrap` | exact `is_some`/`unwrap`/else shape (structural, receiver-matched) |
//! | `needless_range_loop` | `0..v.len()` + index-only usage (exact AST analysis) |
//! | `needless_lifetimes` | single elidable lifetime (elision-rule verified) |
//!
//! The remaining three proposers fire on **type-blind** detection — they are
//! correct when the types line up, but a parse-only validator cannot know
//! that. They stay available to the `self_evolve` tuning loop (whose output
//! IS verified by a real `cargo clippy` run) and are excluded from `--fix`:
//!
//! | Lint | Why not `--fix` |
//! |---|---|
//! | `needless_borrow` | whether `&x` is needless depends on the callee's expected type |
//! | `useless_conversion` | `.into()` is only useless if source type == annotated type |
//! | `unnecessary_mut_passed` | `&mut v` → `&v` depends on the callee's signature; the param-`mut` strip is file-wide |
//!
//! ## Scope
//!
//! Each proposer handles ONE lint. Adding a new lint = adding a new proposer
//! module + a row in `collect_rule_edits`' match (and, when detection is
//! structural, a row in [`FIXABLE_LINTS`]). The 10 seed-corpus lints ship
//! today; future lints drop in by extending the match.

pub mod manual_map;
pub mod needless_borrow;
pub mod needless_lifetimes;
pub mod needless_range_loop;
pub mod needless_return;
pub mod redundant_closure;
pub mod redundant_field_names;
pub mod unnecessary_mut_passed;
pub mod unnecessary_unwrap;
pub mod useless_conversion;

pub use crate::edit_emitter::{
    apply_edits, edit, for_each_fn, src, swallow_following_space, RuleEdit, SourceEdit,
};

/// Lints whose proposers are `cargo heal --fix` eligible — detection is
/// purely structural (see module docs). The other three proposers
/// (`needless_borrow`, `useless_conversion`, `unnecessary_mut_passed`) are
/// type-blind and only fire through the `self_evolve` verified loop.
pub const FIXABLE_LINTS: &[&str] = &[
    "needless_return",
    "redundant_closure",
    "redundant_field_names",
    "manual_map",
    "unnecessary_unwrap",
    "needless_range_loop",
    "needless_lifetimes",
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

/// Dispatch to one lint's proposer. All 10 lints are reachable here (the
/// `self_evolve` loop verifies output with cargo clippy); `--fix` callers
/// filter through [`FIXABLE_LINTS`] instead.
fn collect_rule_edits(lint_name: &str, source: &str, file: &syn::File) -> Vec<SourceEdit> {
    match lint_name {
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

    /// The fixable list is the structural-detection subset — the three
    /// type-blind lints must stay out (parse-only `--fix` cannot check types).
    #[test]
    fn fixable_lints_exclude_type_blind_rules() {
        for excluded in ["needless_borrow", "useless_conversion", "unnecessary_mut_passed"] {
            assert!(
                !FIXABLE_LINTS.contains(&excluded),
                "{excluded} must not be --fix eligible"
            );
        }
        assert_eq!(FIXABLE_LINTS.len(), 7);
    }

    /// Every proposer lint round-trips through the shared dispatch (fires on
    /// its fixture + output re-parses — the L2 pruner check `--fix` runs).
    #[test]
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
                "fn show(s: &str) {}\nfn go(s: &str) { show(&s) }",
                "show(s)",
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
