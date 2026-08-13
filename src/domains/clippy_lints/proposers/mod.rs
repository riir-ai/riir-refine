//! Stage 2 proposal mechanism — per-lint `syn`-level AST rewrites.
//!
//! Issue 010 fix direction (Direction 2): "Treat a corpus entry as a `syn`-level
//! rewrite rule applied to the caller's AST, rather than a literal string to
//! hand back." Each proposer is a small codemod that matches the lint's pattern
//! in the caller's code and emits the idiomatic fix with the caller's own
//! identifiers by construction.
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
//!
//! ## Scope
//!
//! Each proposer handles ONE lint. Adding a new lint = adding a new proposer
//! fn + a row in `propose_fix`'s match. The 10 seed-corpus lints ship today;
//! future lints drop in by extending the match.

pub mod needless_return;
pub mod redundant_closure;
pub mod needless_lifetimes;
pub mod redundant_field_names;
pub mod useless_conversion;
pub mod needless_borrow;
pub mod unnecessary_mut_passed;
pub mod manual_map;
pub mod unnecessary_unwrap;
pub mod needless_range_loop;

/// Propose a fix for `(lint_name, buggy_code)` by applying the lint's
/// AST rewrite.
///
/// Returns `Some(fixed_code)` if a pattern matched and was rewritten,
/// `None` otherwise. The output is NOT verified here — the tuning loop
/// runs `cargo clippy` to confirm.
///
/// # Errors
///
/// Returns `None` if the code doesn't parse as a `syn::File` (caller should
/// fall back to other proposal paths or give up).
#[must_use]
pub fn propose_fix(lint_name: &str, buggy_code: &str) -> Option<String> {
    // Parse as a file. If the input is a snippet, callers should wrap it first.
    let mut file: syn::File = syn::parse_file(buggy_code).ok()?;

    let changed = match lint_name {
        "needless_return" => needless_return::rewrite(&mut file),
        "redundant_closure" => redundant_closure::rewrite(&mut file),
        "needless_lifetimes" => needless_lifetimes::rewrite(&mut file),
        "redundant_field_names" => redundant_field_names::rewrite(&mut file),
        "useless_conversion" => useless_conversion::rewrite(&mut file),
        "needless_borrow" => needless_borrow::rewrite(&mut file),
        "unnecessary_mut_passed" => unnecessary_mut_passed::rewrite(&mut file),
        "manual_map" => manual_map::rewrite(&mut file),
        "unnecessary_unwrap" => unnecessary_unwrap::rewrite(&mut file),
        "needless_range_loop" => needless_range_loop::rewrite(&mut file),
        _ => false,
    };

    if !changed {
        return None;
    }

    // Re-emit via `syn::File` → `ToTokens` → pretty (to_string). This is the
    // simplest modelless round-trip: no `prettyplease` dep, just `ToTokens`.
    // The output may not preserve the caller's exact formatting (whitespace,
    // comments) but it IS semantically equivalent + idiomatic Rust.
    Some(quote_file(&file))
}

/// Pretty-print a `syn::File` via `ToTokens`. NOT formatting-preserving.
pub(crate) fn quote_file(file: &syn::File) -> String {
    use quote::ToTokens;
    let mut tokens = proc_macro2::TokenStream::new();
    file.to_tokens(&mut tokens);
    tokens.to_string()
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
}
