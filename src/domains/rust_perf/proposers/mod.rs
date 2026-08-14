//! Stage 2 proposal mechanism for the `rust_perf` domain — per-rule
//! `syn`-level AST rewrites (Plan 005 Phase 4).
//!
//! Mirrors [`crate::domains::clippy_lints::proposers`]: each proposer is a
//! small codemod that matches the rule's anti-pattern in the caller's code
//! and rewrites it in place, emitting the idiomatic fix with the caller's
//! own identifiers by construction. The latent matcher (`--suggest`) picks
//! the rule; the proposer applies it; the L2 pruner validates the result
//! parses.
//!
//! ## Phase 4 scope (T4.1-T4.7): the top-5 Pareto-dominant perf rules
//!
//! | Rule | Rewrite | Signature-visible change? |
//! |---|---|---|
//! | `vec-with-capacity` | `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(<cap>)` | no |
//! | `entry-vs-contains-insert` | `if !m.contains_key(&k) { m.insert(k, v); }` → `m.entry(k).or_insert(v);` | no |
//! | `static-str` | `-> String { "lit".to_string() }` → `-> &'static str { "lit" }` | **yes** (return type) |
//! | `bool-to-bitpack` | `fn(flags: &[bool]) { flags.iter().filter(..).count() }` → `fn(flags: &[u64]) { .. count_ones().sum() }` | **yes** (param type) |
//! | `fxhashmap` | `use std::collections::HashMap;` + body `HashMap` refs → `rustc_hash::FxHashMap` | no (needs the `rustc-hash` dep in the healed crate) |
//!
//! ## Honest caveats (Proposal 002 caveats 3-4, restated for this domain)
//!
//! - **`quote_file` is NOT formatting-preserving** — the output is a
//!   token-stream re-emission. A production `--fix` (Phase 6) will need the
//!   span-preserving patch emitter; the proposers here operate on the AST.
//! - **`--fix` only applies CONFIRMED visitor matches** (these proposers).
//!   Latent-space suggestions from `--suggest` never auto-apply.
//! - **`rustc-hash` must be a dependency** of the healed crate for the
//!   `fxhashmap` rewrite to compile; parse-only validation cannot check this.
//! - `static-str` + `bool-to-bitpack` change fn signatures — the rewrite is
//!   span-local (the one fn) but semantically ripples to callers. Same
//!   exposure class as the clippy proposers' rewrites; the pruner validates
//!   syntax, `rust_perf_verify` (cargo test) validates semantics.

use syn::{Block, File, ImplItem, Item, Signature};

pub mod bool_to_bitpack;
pub mod entry_vs_contains;
pub mod fxhashmap;
pub mod static_str;
pub mod vec_with_capacity;

/// Propose a fix for `(rule_id, buggy_code)` by applying the rule's AST
/// rewrite.
///
/// Returns `Some(fixed_code)` if a pattern matched and was rewritten,
/// `None` otherwise. The output is NOT verified here — the caller runs the
/// L2 pruner (parse check) and, behind `rust_perf_verify`, `cargo test`.
///
/// # Errors
///
/// Returns `None` if the code doesn't parse as a `syn::File` (the caller
/// should fall back to `--suggest` output for that rule).
#[must_use]
pub fn propose_fix(rule_id: &str, buggy_code: &str) -> Option<String> {
    // Parse as a file. If the input is a snippet, callers should wrap it first.
    let mut file: syn::File = syn::parse_file(buggy_code).ok()?;

    let changed = match rule_id {
        "vec-with-capacity" => vec_with_capacity::rewrite(&mut file),
        "entry-vs-contains-insert" => entry_vs_contains::rewrite(&mut file),
        "static-str" => static_str::rewrite(&mut file),
        "bool-to-bitpack" => bool_to_bitpack::rewrite(&mut file),
        "fxhashmap" => fxhashmap::rewrite(&mut file),
        _ => false,
    };

    if !changed {
        return None;
    }

    Some(quote_file(&file))
}

/// Pretty-print a `syn::File` via `ToTokens`. NOT formatting-preserving
/// (same contract as the clippy_lints proposer helper).
pub(crate) fn quote_file(file: &File) -> String {
    use quote::ToTokens;
    let mut tokens = proc_macro2::TokenStream::new();
    file.to_tokens(&mut tokens);
    tokens.to_string()
}

/// Visit every fn-like item in the file (free fns, impl methods, fns in
/// nested inline `mod` blocks) with `(sig, block)`.
///
/// Shared by the four fn-scoped proposers (`vec-with-capacity`,
/// `entry-vs-contains-insert`, `static-str`, `bool-to-bitpack`); the
/// clippy_lints proposers scan `file.items` directly — this helper adds
/// impl-method + nested-mod coverage, which real files exercise.
pub(crate) fn for_each_fn_mut(file: &mut File, mut f: impl FnMut(&mut Signature, &mut Block)) {
    for_each_item_mut(&mut file.items, &mut f);
}

fn for_each_item_mut(items: &mut [Item], f: &mut impl FnMut(&mut Signature, &mut Block)) {
    for item in items {
        match item {
            Item::Fn(fn_item) => f(&mut fn_item.sig, &mut fn_item.block),
            Item::Impl(imp) => {
                for ii in &mut imp.items {
                    if let ImplItem::Fn(m) = ii {
                        f(&mut m.sig, &mut m.block);
                    }
                }
            }
            // Nested inline `mod x { ... }` blocks recurse; `mod x;` (content
            // None) is a different file — out of scope.
            Item::Mod(m) => {
                if let Some((_, inner)) = &mut m.content {
                    for_each_item_mut(inner, f);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Squash all whitespace so assertions survive token-stream spacing
    /// (`Vec :: with_capacity (n)` vs `Vec::with_capacity(n)`).
    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn unknown_rule_returns_none() {
        assert_eq!(propose_fix("not_a_real_rule", "fn foo() {}"), None);
    }

    #[test]
    fn unparseable_code_returns_none() {
        assert_eq!(propose_fix("vec-with-capacity", "this is not rust {{{"), None);
    }

    /// End-to-end: each of the 5 corpus fixtures fires its proposer, the
    /// output carries the fixed-form marker, AND the output parses (L2
    /// pruner — the same validation `RustPerfDomain::validate` runs).
    #[test]
    fn corpus_fixtures_fire_and_parse() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "vec-with-capacity",
                "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
                 for i in 0..n { v.push(i as u32); }\n    v\n}",
                "with_capacity(n)",
            ),
            (
                "entry-vs-contains-insert",
                "use std::collections::HashMap;\nfn insert_if_absent(map: &mut \
                 HashMap<u32, u32>, k: u32, v: u32) {\n    if !map.contains_key(&k) {\n        \
                 map.insert(k, v);\n    }\n}",
                "entry(k).or_insert(v)",
            ),
            (
                "static-str",
                "fn label() -> String { \"ready\".to_string() }",
                "&'static str",
            ),
            (
                "bool-to-bitpack",
                "fn count_true(flags: &[bool]) -> usize {\n    \
                 flags.iter().filter(|&&b| b).count()\n}",
                "count_ones() as usize",
            ),
            (
                "fxhashmap",
                "use std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) \
                 -> Option<u32> {\n    map.get(&k).copied()\n}",
                "rustc_hash::FxHashMap",
            ),
        ];
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        for (rule, buggy, marker) in cases {
            let out = propose_fix(rule, buggy).unwrap_or_else(|| {
                panic!("proposer did not fire for corpus fixture `{rule}`")
            });
            assert!(
                squash(&out).contains(&squash(marker)),
                "`{rule}` output missing marker `{marker}`: {out}"
            );
            let verdict = pruner.validate(&out, rule);
            assert!(verdict.parses, "`{rule}` output does not parse: {out}");
        }
    }
}
