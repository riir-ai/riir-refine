//! Stage 2 proposal mechanism for the `rust_perf` domain — per-rule
//! `syn`-level AST rewrites (Plan 005 Phase 4 + Phase 6 T6.1).
//!
//! Mirrors [`crate::domains::clippy_lints::proposers`] in being a per-rule
//! codemod that matches the rule's anti-pattern in the caller's code — but
//! the rewrite mechanism is **span-preserving source edits** (Phase 6 T6.1):
//! each proposer walks the parsed file immutably and emits
//! [`SourceEdit`]s (byte range + replacement text). `apply_edits` splices
//! them into the ORIGINAL source — every byte outside the matched spans
//! stays identical. No whole-file token re-emission, no reformat.
//!
//! The replacement text is built from the *original source text* of the
//! matched nodes (via `proc-macro2` fallback spans + the `span-locations`
//! feature) plus short literal fragments — so even the edited regions keep
//! the caller's own formatting for every sub-expression they contained.
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
//! - **`--fix` only applies CONFIRMED visitor matches** (these proposers).
//!   Latent-space suggestions from `--suggest` never auto-apply — the
//!   proposer's detector is the gate, retrieval only reports/prioritizes.
//! - **`rustc-hash` must be a dependency** of the healed crate for the
//!   `fxhashmap` rewrite to compile; parse-only validation cannot check this.
//! - `static-str` + `bool-to-bitpack` change fn signatures — the rewrite is
//!   span-local (the one fn) but semantically ripples to callers. Same
//!   exposure class as the clippy proposers' rewrites; the pruner validates
//!   syntax, `rust_perf_verify` (cargo test) validates semantics.
//! - The clippy_lints proposers remain mutation-based (consumed by the
//!   `self_evolve` tuning loop); migrating them to the edit-splice emitter
//!   is follow-up work once `--fix` proves out.

use syn::spanned::Spanned;
use syn::{File, Item, Signature};

pub mod bool_to_bitpack;
pub mod entry_vs_contains;
pub mod fxhashmap;
pub mod static_str;
pub mod vec_with_capacity;

/// Rule ids with compiled-in proposers (`--fix` eligible). A visitor match
/// is the confirmation — retrieval scores only report/prioritize.
pub const FIXABLE_RULES: &[&str] = &[
    "vec-with-capacity",
    "entry-vs-contains-insert",
    "static-str",
    "bool-to-bitpack",
    "fxhashmap",
];

/// One span-preserving source edit: replace `source[start..end]` with
/// `replacement`. Byte offsets index the exact original source the file was
/// parsed from.
#[derive(Debug, Clone)]
pub struct SourceEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

/// The rule the edit came from (for `--fix` reporting).
#[derive(Debug, Clone)]
pub struct RuleEdit {
    pub rule_id: String,
    pub edit: SourceEdit,
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
    let edits = match rule_id {
        "vec-with-capacity" => vec_with_capacity::collect_edits(source, file),
        "entry-vs-contains-insert" => entry_vs_contains::collect_edits(source, file),
        "static-str" => static_str::collect_edits(source, file),
        "bool-to-bitpack" => bool_to_bitpack::collect_edits(source, file),
        "fxhashmap" => fxhashmap::collect_edits(source, file),
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

/// Splice non-overlapping edits into the source. Selection: edits are
/// sorted ascending by start (outer/larger spans first on ties) and an
/// edit starting before the running end of an applied region overlaps or
/// is contained by it and is dropped (e.g. a nested pattern inside an
/// already-replaced span). Application: the selected edits are applied
/// from the END of the file backwards so earlier byte offsets stay valid
/// as replacements change lengths. Invalid ranges (misaligned char
/// boundaries) are dropped, not panicked on.
#[must_use]
pub fn apply_edits(source: &str, mut edits: Vec<SourceEdit>) -> String {
    // Phase 1 — selection (outer-first, drop overlaps).
    edits.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut selected: Vec<SourceEdit> = Vec::with_capacity(edits.len());
    let mut applied_end = 0usize;
    for e in edits {
        if e.start < applied_end || e.start > e.end || source.get(e.start..e.end).is_none() {
            continue; // overlaps an applied region / malformed / misaligned
        }
        applied_end = applied_end.max(e.end);
        selected.push(e);
    }
    // Phase 2 — apply back-to-front (offsets before each edit stay valid).
    let mut out = source.to_string();
    for e in selected.into_iter().rev() {
        out.replace_range(e.start..e.end, &e.replacement);
    }
    out
}

// ── Shared helpers ───────────────────────────────────────────────────────

/// The original source text of a node (its exact formatting, comments
/// excluded — spans cover tokens). `None` if the span doesn't slice the
/// source cleanly (defensive: means span bookkeeping broke).
pub(crate) fn src(source: &str, node: &impl Spanned) -> Option<String> {
    source.get(node.span().byte_range()).map(str::to_string)
}

/// Build an edit replacing the node's span.
pub(crate) fn edit(node: &impl Spanned, replacement: String) -> SourceEdit {
    let r = node.span().byte_range();
    SourceEdit {
        start: r.start,
        end: r.end,
        replacement,
    }
}

/// Leading whitespace of the line containing byte offset `at` (for
/// multi-line replacement text that introduces new lines, e.g. the
/// fxhashmap use-append).
pub(crate) fn line_indent(source: &str, at: usize) -> String {
    let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    source[line_start..at]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect()
}

/// Span start/end of a node as (usize, usize).
pub(crate) fn span_range(node: &impl Spanned) -> (usize, usize) {
    let r = node.span().byte_range();
    (r.start, r.end)
}

/// Visit every fn-like item in the file (free fns, impl methods, fns in
/// nested inline `mod` blocks) with `(sig, block)`.
///
/// Shared by the four fn-scoped proposers (`vec-with-capacity`,
/// `entry-vs-contains-insert`, `static-str`, `bool-to-bitpack`); the
/// clippy_lints proposers scan `file.items` directly — this helper adds
/// impl-method + nested-mod coverage, which real files exercise.
pub(crate) fn for_each_fn<'a>(file: &'a File, mut f: impl FnMut(&'a Signature, &'a syn::Block)) {
    for_each_item(&file.items, &mut f);
}

fn for_each_item<'a>(items: &'a [Item], f: &mut impl FnMut(&'a Signature, &'a syn::Block)) {
    for item in items {
        match item {
            Item::Fn(fn_item) => f(&fn_item.sig, &fn_item.block),
            Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        f(&m.sig, &m.block);
                    }
                }
            }
            // Nested inline `mod x { ... }` blocks recurse; `mod x;` (content
            // None) is a different file — out of scope.
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    for_each_item(inner, f);
                }
            }
            _ => {}
        }
    }
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
                "FxHashMap",
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

    #[test]
    fn apply_edits_drops_overlaps_and_applies_bottom_up() {
        let src = "aaaa bbbb cccc";
        // Two disjoint edits: replace "bbbb" → "XX", "aaaa" → "YY".
        let out = apply_edits(
            src,
            vec![
                SourceEdit {
                    start: 0,
                    end: 4,
                    replacement: "YY".to_string(),
                },
                SourceEdit {
                    start: 5,
                    end: 9,
                    replacement: "XX".to_string(),
                },
            ],
        );
        assert_eq!(out, "YY XX cccc");
        // An edit contained in a replaced region is dropped.
        let out = apply_edits(
            src,
            vec![
                SourceEdit {
                    start: 0,
                    end: 9,
                    replacement: "ALL".to_string(),
                },
                SourceEdit {
                    start: 5,
                    end: 9,
                    replacement: "INNER".to_string(),
                },
            ],
        );
        assert_eq!(out, "ALL cccc");
    }

    #[test]
    fn span_text_round_trips() {
        // The whole emitter depends on node spans slicing the source back
        // to the node's own text. Pin it with a direct check.
        let source = "fn f() -> u32 { let x = (1 + 2) * 3; x }";
        let file = syn::parse_file(source).unwrap();
        let Item::Fn(f) = &file.items[0] else { panic!() };
        let syn::Stmt::Local(l) = &f.block.stmts[0] else { panic!() };
        let init = l.init.as_ref().unwrap();
        assert_eq!(src(source, &init.expr).as_deref(), Some("(1 + 2) * 3"));
    }
}
