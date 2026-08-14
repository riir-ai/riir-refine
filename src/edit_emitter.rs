//! Span-preserving source-edit emitter — the shared `--fix` substrate.
//!
//! Extracted from `domains::rust_perf::proposers` (Plan 005 Phase 6 T6.4)
//! so BOTH domain proposer sets (`rust_perf` + `clippy_lints`, each behind
//! its own Cargo feature) can emit edits through one implementation. This
//! module depends only on `syn` + `proc-macro2` (always-on deps) and is
//! therefore compiled unconditionally.
//!
//! ## The protocol
//!
//! A proposer walks the parsed file **immutably** and emits [`SourceEdit`]s
//! (byte range + replacement text). [`apply_edits`] splices them into the
//! ORIGINAL source — every byte outside the matched spans stays identical.
//! No whole-file token re-emission, no reformat.
//!
//! The replacement text is built from the *original source text* of the
//! matched nodes (via `proc-macro2` fallback spans + the `span-locations`
//! feature) plus short literal fragments — so even the edited regions keep
//! the caller's own formatting for every sub-expression they contained.
//!
//! ## Application discipline
//!
//! Selection is **outer-first** (ascending start, larger span first on
//! ties); an edit starting inside an already-selected region is dropped
//! (nested pattern inside a replaced span). Application runs
//! **back-to-front** so byte offsets stay valid as replacement lengths
//! change. Invalid ranges (misaligned char boundaries) are dropped, not
//! panicked on.

use syn::spanned::Spanned;
use syn::{File, Item, Signature};

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
pub fn src(source: &str, node: &impl Spanned) -> Option<String> {
    source.get(node.span().byte_range()).map(str::to_string)
}

/// Build an edit replacing the node's span.
pub fn edit(node: &impl Spanned, replacement: String) -> SourceEdit {
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
#[must_use]
pub fn line_indent(source: &str, at: usize) -> String {
    let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    source[line_start..at]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect()
}

/// Span start/end of a node as (usize, usize).
pub fn span_range(node: &impl Spanned) -> (usize, usize) {
    let r = node.span().byte_range();
    (r.start, r.end)
}

/// Extend an edit's end past ONE following ASCII space, when present —
/// deleting `'a` from `&'a u8` should yield `&u8`, not `& u8`.
#[must_use]
pub fn swallow_following_space(source: &str, mut e: SourceEdit) -> SourceEdit {
    if source[e.end..].starts_with(' ') {
        e.end += 1;
    }
    e
}

/// Visit every fn-like item in the file (free fns, impl methods, fns in
/// nested inline `mod` blocks) with `(sig, block)`.
///
/// Shared by the fn-scoped proposers (both domains); a plain `file.items`
/// scan misses impl methods + nested mods, which real files exercise.
pub fn for_each_fn<'a>(file: &'a File, mut f: impl FnMut(&'a Signature, &'a syn::Block)) {
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

    #[test]
    fn swallow_following_space_eats_one_space() {
        // The lifetime must be located with offsets relative to THIS source
        // string (spans index the exact string the file was parsed from).
        let source = "fn f(x: &'a u8) { }";
        let file = syn::parse_file(source).unwrap();
        let Item::Fn(f) = &file.items[0] else { panic!() };
        let syn::FnArg::Typed(pat) = &f.sig.inputs[0] else { panic!() };
        let syn::Type::Reference(r) = &*pat.ty else { panic!() };
        let lt = r.lifetime.as_ref().expect("lifetime present");
        let e = swallow_following_space(source, edit(lt, String::new()));
        assert_eq!(&source[e.start..e.end], "'a ");
    }
}
