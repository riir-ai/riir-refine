//! Stage 2 proposal mechanism for the `rust_perf` domain — per-rule
//! `syn`-level AST rewrites (Plan 005 Phase 4 + Phase 6 T6.1).
//!
//! Mirrors [`crate::domains::clippy_lints::proposers`] in being a per-rule
//! codemod that matches the rule's anti-pattern in the caller's code — the
//! rewrite mechanism is **span-preserving source edits** (Phase 6 T6.1 +
//! T6.4): each proposer walks the parsed file immutably and emits
//! [`SourceEdit`]s (byte range + replacement text). `apply_edits` splices
//! them into the ORIGINAL source — every byte outside the matched spans
//! stays identical. No whole-file token re-emission, no reformat.
//!
//! The emitter substrate ([`crate::edit_emitter`]) is shared with the
//! `clippy_lints` proposers since T6.4 — both domains re-export it from
//! here for API stability.
//!
//! ## Phase 4 scope (T4.1-T4.7): the top-5 Pareto-dominant perf rules
//!
//! | Rule | Rewrite | Signature-visible change? |
//! |---|---|---|
//! | `vec-with-capacity` | `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(<cap>)` | no |
//! | `entry-vs-contains-insert` | `if !m.contains_key(&k) { m.insert(k, v); }` → `m.entry(k).or_insert(v);` | no |
//! | `static-str` | `-> String { "lit".to_string() }` → `-> &'static str { "lit" }` | **yes** (return type) |
//! | `bool-to-bitpack` | `fn(flags: &[bool]) { flags.iter().filter(..).count() }` → `fn(flags: &[u64]) { .. count_ones().sum() }` | **yes** (param type) |
//! | `fxhashmap` | `use std::collections::HashMap;` + body `HashMap` refs → `rustc_hash::FxHashMap` | no (self-gated: fires only when the file already imports `rustc_hash` — a splice cannot add the Cargo.toml dep) |
//!
//! ## Batch 48 follow-up (2026-08-17): +3 proposer-covered rules
//!
//! The three parse-clean ParetoDominant candidates whose proposers were
//! deferred pending bench verdicts (Bench 020/B48 — all three measured
//! ParetoDominant, so they are now `--fix` eligible):
//!
//! | Rule | Rewrite | Signature-visible change? |
//! |---|---|---|
//! | `hashset-insert-bool-return` | `if !s.contains(k) { s.insert(k); .. }` → `if s.insert(k) { .. }` (insert's bool return; the insert stmt is deleted) | no |
//! | `collect-append-to-extend` | `let mut staged = it.collect(); recv.append(&mut staged);` → `recv.extend(it);` (the staging `let` is deleted) | no |
//! | `vec-macro-zero-fill` | `let mut v = Vec::new(); v.resize(n, 0);` → `let mut v = vec![0; n];` (resize stmt deleted; `v` stays alive) | no |
//!
//! `filter-count-to-any` stays proposer-less BY DESIGN (ParetoDominated
//! 0.82-0.90x — never give a losing rewrite a `--fix` path).
//!
//! ## Honest caveats (Proposal 002 caveats 3-4, restated for this domain)
//!
//! - **`--fix` only applies CONFIRMED visitor matches** (these proposers).
//!   Latent-space suggestions from `--suggest` never auto-apply — the
//!   proposer's detector is the gate, retrieval only reports/prioritizes.
//! - **`fxhashmap` self-gates on an existing `rustc_hash` import** in the
//!   target file (Gate 0 in the proposer): a span-preserving splice can
//!   add a `use` line but cannot add the `rustc-hash` dependency to the
//!   crate's `Cargo.toml`, so the rewrite only fires where the crate path
//!   already resolves. Enforced since the 2026-08-15 game-sdk G5 smoke
//!   caught an E0432 (`bevy_shared.rs`, crate without `rustc-hash`).
//!   Cold-start adoption (first file in a crate + the dep addition) stays
//!   manual.
//! - `static-str` + `bool-to-bitpack` change fn signatures — the rewrite is
//!   span-local (the one fn) but semantically ripples to callers. Same
//!   exposure class as the clippy proposers' rewrites; the pruner validates
//!   syntax, `rust_perf_verify` (cargo test) validates semantics.
//! - Since T6.4 the clippy_lints proposers use the same edit-splice emitter
//!   (they were mutation-based + whole-file `ToTokens` re-emission before).

use syn::File;

pub mod bool_to_bitpack;
pub mod collect_append_to_extend;
pub mod entry_vs_contains;
pub mod fxhashmap;
pub mod hashset_insert_bool_return;
pub mod static_str;
pub mod vec_macro_zero_fill;
pub mod vec_with_capacity;

// Shared emitter substrate (T6.4): re-exported so the pre-T6.4 import path
// `rust_perf::proposers::{apply_edits, collect_edits, SourceEdit, ...}`
// keeps compiling. The helpers stay `pub(crate)`-visible via the crate root.
pub use crate::edit_emitter::{
    apply_edits, delete_stmt_edit, edit, for_each_fn, line_indent, src, span_range,
    swallow_following_space, RuleEdit, SourceEdit,
};

/// Rule ids with compiled-in proposers (`--fix` eligible). A visitor match
/// is the confirmation — retrieval scores only report/prioritize.
pub const FIXABLE_RULES: &[&str] = &[
    "vec-with-capacity",
    "entry-vs-contains-insert",
    "static-str",
    "bool-to-bitpack",
    "fxhashmap",
    "hashset-insert-bool-return",
    "collect-append-to-extend",
    "vec-macro-zero-fill",
];

/// Proposer option flags (Issue 015 verify-mode).
///
/// `--fix` defaults to the conservative parse-only gates; `--fix
/// --verify` (rustc backing the revert) passes `untyped_paths = true` to
/// attempt plain-path capacity bounds the usize gate cannot prove.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProposerOpts {
    /// Attempt plain-path capacity bounds whose type syn cannot prove
    /// (consts, associated consts, locals). rustc's E0308 backs the
    /// auto-revert in verify-mode. The scope gate (no later-let
    /// references) stays ON regardless.
    pub untyped_paths: bool,
}

/// Propose a fix for `(rule_id, buggy_code)` by applying the rule's
/// span-preserving edits.
///
/// Returns `Some(fixed_code)` if a pattern matched and was rewritten,
/// `None` otherwise. The output preserves the input's formatting
/// byte-identically outside the matched spans. NOT verified here — the
/// caller runs the L2 pruner (parse check) and, behind `rust_perf_verify`,
/// `cargo test`.
///
/// # Errors
///
/// Returns `None` if the code doesn't parse as a `syn::File` or the rule
/// has no compiled-in proposer (the caller should fall back to `--suggest`
/// output for that rule).
#[must_use]
pub fn propose_fix(rule_id: &str, buggy_code: &str) -> Option<String> {
    let file: File = syn::parse_file(buggy_code).ok()?;
    let edits = collect_edits(rule_id, buggy_code, &file);
    if edits.is_empty() {
        return None;
    }
    let out = apply_edits(buggy_code, edits.iter().map(|e| e.edit.clone()).collect());
    Some(out)
}

/// Collect one rule's edits against the original source. Empty = no match.
#[must_use]
pub fn collect_edits(rule_id: &str, source: &str, file: &File) -> Vec<RuleEdit> {
    collect_edits_opts(rule_id, source, file, ProposerOpts::default())
}

/// [`collect_edits`] with explicit proposer options (Issue 015: the CLI's
/// `--verify` path loosens the vec-with-capacity usize gate — see
/// [`ProposerOpts::untyped_paths`]).
#[must_use]
pub fn collect_edits_opts(
    rule_id: &str,
    source: &str,
    file: &File,
    opts: ProposerOpts,
) -> Vec<RuleEdit> {
    let edits = match rule_id {
        "vec-with-capacity" => vec_with_capacity::collect_edits_opts(source, file, opts),
        "entry-vs-contains-insert" => entry_vs_contains::collect_edits(source, file),
        "static-str" => static_str::collect_edits(source, file),
        "bool-to-bitpack" => bool_to_bitpack::collect_edits(source, file),
        "fxhashmap" => fxhashmap::collect_edits(source, file),
        "hashset-insert-bool-return" => hashset_insert_bool_return::collect_edits(source, file),
        "collect-append-to-extend" => collect_append_to_extend::collect_edits(source, file),
        "vec-macro-zero-fill" => vec_macro_zero_fill::collect_edits(source, file),
        _ => Vec::new(),
    };
    edits
        .into_iter()
        .map(|edit| RuleEdit {
            rule_id: rule_id.to_string(),
            edit,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Squash all whitespace so assertions survive token spacing in
    /// generated fragments.
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

    /// The load-bearing invariant of the span-preserving emitter: everything
    /// outside the matched span is byte-identical — original whitespace,
    /// line breaks, and untouched statements survive verbatim.
    #[test]
    fn output_preserves_untouched_bytes() {
        let src = "// header comment\nfn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
                   // fill it\n    for i in 0..n { v.push(i as u32); }\n    let tail = 7;\n    v\n}\n";
        let out = propose_fix("vec-with-capacity", src).expect("pattern matched");
        assert!(out.contains("// header comment\n"), "{out}");
        assert!(out.contains("// fill it\n    for i in 0..n { v.push(i as u32); }"), "{out}");
        assert!(out.contains("let tail = 7;"), "{out}");
        assert!(out.contains("let mut v = Vec::with_capacity(n);"), "{out}");
        // Exactly one line differs in structure: the init line.
        let orig_lines: Vec<&str> = src.lines().collect();
        let out_lines: Vec<&str> = out.lines().collect();
        assert_eq!(orig_lines.len(), out_lines.len(), "line count changed: {out}");
        let diffs = orig_lines
            .iter()
            .zip(&out_lines)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(diffs, 1, "expected exactly one changed line: {out}");
    }

    /// End-to-end: each proposer-covered corpus fixture fires its proposer,
    /// the output carries the fixed-form marker, AND the output parses (L2
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
                "use rustc_hash::FxHashSet;\nuse std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) \
                 -> Option<u32> {\n    map.get(&k).copied()\n}",
                "FxHashMap",
            ),
            (
                "hashset-insert-bool-return",
                "use std::collections::HashSet;\nfn register_seen(seen: &mut \
                 HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    \
                 for &x in items {\n        if !seen.contains(&x) {\n            \
                 seen.insert(x);\n            inserted += 1;\n        }\n    }\n    inserted\n}",
                "if seen.insert(x) {",
            ),
            (
                "collect-append-to-extend",
                "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in \
                 chunks {\n        let mut staged: Vec<u32> = c.iter().copied().collect();\n        \
                 target.append(&mut staged);\n    }\n}",
                "target.extend(c.iter().copied())",
            ),
            (
                "vec-macro-zero-fill",
                "fn zero_buf(n: usize) -> Vec<u8> {\n    let mut v = Vec::new();\n    \
                 v.resize(n, 0);\n    v\n}",
                "vec![0; n]",
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
