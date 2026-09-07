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
use syn::{Expr, File, Item, Signature, Stmt};

/// One span-preserving source edit: replace `source[start..end]` with
/// `replacement`. Byte offsets index the exact original source the file was
/// parsed from.
///
/// `PartialEq`/`Eq` (added with the `docker` domain, Plan 030) so proposer
/// tests can assert on emitted edits directly instead of on their applied
/// output — purely additive, no behavior change.
#[derive(Debug, Clone, PartialEq, Eq)]
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
pub fn apply_edits(source: &str, edits: Vec<SourceEdit>) -> String {
    apply_edits_traced(source, edits).0
}

/// [`apply_edits`](fn.apply_edits.html) that also returns the splices it
/// actually applied (in INPUT-source coordinates, sorted ascending by
/// start). The caller uses these to remap positions recorded against this
/// pass's input through later passes — e.g. `cargo heal --fix` review line
/// numbers against the FINAL written source, not per-pass intermediates
/// (some fixes change line counts: `manual_map` collapses multi-line matches,
/// fxhashmap's use-append inserts a line).
#[must_use]
pub fn apply_edits_traced(source: &str, mut edits: Vec<SourceEdit>) -> (String, Vec<SourceEdit>) {
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
    for e in selected.iter().rev() {
        out.replace_range(e.start..e.end, &e.replacement);
    }
    (out, selected)
}

/// Remap a byte offset recorded against a pass's INPUT through that pass's
/// applied splices (as returned by [`apply_edits_traced`]) into the pass's
/// OUTPUT coordinates. All comparisons use the ORIGINAL offset — a walking
/// position compared against unshifted splice starts mis-orders
/// deletion-then-insertion pairs. A position inside a replaced region clamps
/// to the region's output start (the old content is gone). Safe for a
/// splice's OWN start too (it breaks at `start >= p` before self-applying).
#[must_use]
pub fn map_through_splices(p: usize, splices: &[SourceEdit]) -> usize {
    let mut delta: isize = 0;
    for s in splices {
        if s.start >= p {
            break; // sorted ascending, non-overlapping — later splices can't move p
        }
        if s.end <= p {
            // Splice ends before p — shifts it by its length delta
            // (negative for deletions).
            delta += s.replacement.len().cast_signed() - (s.end - s.start).cast_signed();
        } else {
            // p lies inside the replaced region — clamp to its output start,
            // shifted by the strictly-earlier splices accumulated so far.
            return (s.start.cast_signed() + delta).cast_unsigned();
        }
    }
    (p.cast_signed() + delta).cast_unsigned()
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

/// Does a whole-node splice replacement DROP a comment? Scans the replaced
/// span's source for comment markers and reports any that is NOT inside a
/// kept fragment (Issue 027/028/029 class — real clippy stays silent on
/// comment-displacing rewrites, and our emitter deletes the comment
/// outright). Kept fragments are byte ranges that survive verbatim in the
/// replacement (e.g. scrutinee, patterns, arm bodies). Hoisted here (from
/// `clippy_lints::proposers::helpers`, Plan 057 salvage) so the `sec`
/// domain can share it WITHOUT pulling `clippy_lints` into
/// `--no-default-features --features sec` — this module is the
/// feature-neutral substrate.
#[must_use]
pub fn drops_interior_comment(
    source: &str,
    span: std::ops::Range<usize>,
    kept: &[std::ops::Range<usize>],
) -> bool {
    let Some(text) = source.get(span.clone()) else {
        return true; // broken span bookkeeping — refuse rather than displace
    };
    let offset = span.start;
    for pos in comment_marker_positions(text) {
        let abs = offset + pos;
        if !kept.iter().any(|k| k.contains(&abs)) {
            return true;
        }
    }
    false
}

/// Byte offsets of `//` and `/*` comment OPENERS in `text` (syn has no
/// comment nodes). Dropped-gap regions of a rewritten node contain only
/// keywords/punctuation, so string-literal false positives cannot occur
/// there; kept fragments are checked verbatim so a `//` inside a string in a
/// kept fragment is covered by its own range.
#[must_use]
pub fn comment_marker_positions(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && (bytes[i + 1] == b'/' || bytes[i + 1] == b'*') {
            out.push(i);
            i += 2;
        } else {
            i += 1;
        }
    }
    out.shrink_to_fit();
    out
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

/// Build a deletion edit for a statement that also removes the line's
/// scaffolding when the stmt owns its line: trailing same-line spaces are
/// always swallowed; when the rest of the line is empty AND the stmt's line
/// prefix is all whitespace, the deletion extends back over the indentation
/// and the preceding newline — the line vanishes with no blank-line residue.
/// A stmt sharing its line with other code (or a trailing comment) deletes
/// only its own span + trailing spaces (the `collect-append-to-extend`
/// proposer's `let .. // comment` case: the comment survives on its own
/// line, still attached to the statement that replaced the pattern).
pub fn delete_stmt_edit(source: &str, node: &impl Spanned) -> SourceEdit {
    let (mut start, mut end) = span_range(node);
    // Swallow trailing same-line spaces only — the newline's fate is decided
    // by the line-ownership branch below.
    while end < source.len() && source.as_bytes()[end] == b' ' {
        end += 1;
    }
    let after = &source[end..];
    let line_rest_empty =
        after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n");
    if line_rest_empty {
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        let prefix_ws = source[line_start..start].chars().all(|c| c == ' ' || c == '\t');
        if prefix_ws && line_start > 0 {
            start = line_start;
            // Eat the preceding newline (and its \r under CRLF) so the whole
            // line disappears instead of leaving an indentation-only line.
            if source.as_bytes()[start - 1] == b'\n' {
                start -= 1;
                if start > 0 && source.as_bytes()[start - 1] == b'\r' {
                    start -= 1;
                }
            }
        }
    }
    SourceEdit { start, end, replacement: String::new() }
}

/// Visit every fn-like item in the file (free fns, impl methods, fns in
/// nested inline `mod` blocks, AND fns declared inside another fn's body)
/// with `(sig, block)`.
///
/// Shared by the fn-scoped proposers (both domains); a plain `file.items`
/// scan misses impl methods + nested mods, which real files exercise. The
/// nested-fn half (`Stmt::Item(Item::Fn)` at any block depth) was added after
/// the katgpt-attn `block_topk.rs` miss: `fn argtopk_simd() { unsafe fn
/// inner() { … } inner(…) }` — the unsafe-wrapper pattern — put 2 live
/// semicolon sites outside every fn-scoped proposer's reach.
pub fn for_each_fn<'a>(file: &'a File, mut f: impl FnMut(&'a Signature, &'a syn::Block)) {
    for_each_item(&file.items, &mut f);
}

fn for_each_item<'a>(items: &'a [Item], f: &mut impl FnMut(&'a Signature, &'a syn::Block)) {
    for item in items {
        match item {
            Item::Fn(fn_item) => {
                f(&fn_item.sig, &fn_item.block);
                nested_fns_in_block(&fn_item.block, f);
            }
            Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        f(&m.sig, &m.block);
                        nested_fns_in_block(&m.block, f);
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

/// Fns declared inside another fn's body (`fn outer() { fn inner() {} }`).
/// Item statements are legal at any block depth; the unsafe-wrapper pattern
/// puts real code there. Walks every block-bearing statement recursively.
/// `let x = { … }` initializer blocks and closure bodies are a documented
/// under-approximation (no observed production instances).
fn nested_fns_in_block<'a>(block: &'a syn::Block, f: &mut impl FnMut(&'a Signature, &'a syn::Block)) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Item(Item::Fn(nested)) => {
                f(&nested.sig, &nested.block);
                nested_fns_in_block(&nested.block, f);
            }
            Stmt::Expr(e, _) => nested_fns_in_expr(e, f),
            _ => {}
        }
    }
}

fn nested_fns_in_expr<'a>(e: &'a Expr, f: &mut impl FnMut(&'a Signature, &'a syn::Block)) {
    match e {
        Expr::Block(b) => nested_fns_in_block(&b.block, f),
        Expr::Unsafe(u) => nested_fns_in_block(&u.block, f),
        Expr::If(i) => {
            nested_fns_in_block(&i.then_branch, f);
            if let Some((_, e)) = &i.else_branch {
                nested_fns_in_expr(e, f);
            }
        }
        Expr::Loop(l) => nested_fns_in_block(&l.body, f),
        Expr::While(w) => nested_fns_in_block(&w.body, f),
        Expr::ForLoop(fl) => nested_fns_in_block(&fl.body, f),
        Expr::Match(m) => {
            for arm in &m.arms {
                nested_fns_in_expr(&arm.body, f);
            }
        }
        _ => {}
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

    #[test]
    fn delete_stmt_edit_removes_own_line() {
        let source = "fn f() {\n    a();\n    b();\n}\n";
        let file = syn::parse_file(source).unwrap();
        let Item::Fn(f) = &file.items[0] else { panic!() };
        // a(); is the first stmt of the body.
        let e = delete_stmt_edit(source, &f.block.stmts[0]);
        let out = apply_edits(source, vec![e]);
        assert_eq!(out, "fn f() {\n    b();\n}\n");
    }

    #[test]
    fn delete_stmt_edit_consecutive_lines_compose() {
        // Two deletions computed against the ORIGINAL source must both apply
        // cleanly (disjoint ranges) and leave no blank lines behind.
        let source = "fn f() {\n    a();\n    b();\n    c();\n}\n";
        let file = syn::parse_file(source).unwrap();
        let Item::Fn(f) = &file.items[0] else { panic!() };
        let e1 = delete_stmt_edit(source, &f.block.stmts[0]);
        let e2 = delete_stmt_edit(source, &f.block.stmts[1]);
        let out = apply_edits(source, vec![e1, e2]);
        assert_eq!(out, "fn f() {\n    c();\n}\n");
    }

    #[test]
    fn delete_stmt_edit_mid_line_keeps_siblings() {
        let source = "fn f() { a(); b(); }";
        let file = syn::parse_file(source).unwrap();
        let Item::Fn(f) = &file.items[0] else { panic!() };
        let e = delete_stmt_edit(source, &f.block.stmts[0]);
        let out = apply_edits(source, vec![e]);
        assert_eq!(out, "fn f() { b(); }");
    }

    #[test]
    fn delete_stmt_edit_trailing_comment_survives() {
        // The `collect-append-to-extend` corpus shape: a comment after the
        // `;` keeps the deletion mid-line; the comment stays (on the line's
        // remaining indentation), byte-identical.
        let source = "fn f(x: u32) {\n    let s = x; // note\n    x\n}\n";
        let file = syn::parse_file(source).unwrap();
        let Item::Fn(f) = &file.items[0] else { panic!() };
        let e = delete_stmt_edit(source, &f.block.stmts[0]);
        let out = apply_edits(source, vec![e]);
        assert!(out.contains("    // note\n    x"), "{out}");
        assert!(!out.contains("let s"), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn traced_variant_returns_the_applied_splices() {
        let src = "aaaa bbbb cccc";
        let (out, splices) = apply_edits_traced(
            src,
            vec![
                SourceEdit { start: 5, end: 9, replacement: "XX".to_string() },
                // overlaps "bbbb" — must be dropped from the splice record
                SourceEdit { start: 6, end: 8, replacement: "Y".to_string() },
            ],
        );
        assert_eq!(out, "aaaa XX cccc");
        assert_eq!(splices.len(), 1);
        assert_eq!((splices[0].start, splices[0].end), (5, 9));
        assert_eq!(splices[0].replacement, "XX");
    }

    #[test]
    fn map_through_splices_tracks_insertions_and_deletions() {
        // One insertion (grows +3) + one deletion (shrinks −4), sorted asc.
        let splices = vec![
            SourceEdit { start: 2, end: 2, replacement: "xyz".to_string() },  // insert
            SourceEdit { start: 10, end: 14, replacement: String::new() },   // delete
        ];
        // Before everything: unmoved.
        assert_eq!(map_through_splices(0, &splices), 0);
        // At the insertion point: unmoved (p <= start).
        assert_eq!(map_through_splices(2, &splices), 2);
        // Past the insertion: +3.
        assert_eq!(map_through_splices(3, &splices), 6);
        // Past the deletion: +3 − 4.
        assert_eq!(map_through_splices(15, &splices), 14);
        // Inside the deleted region: clamps to its output start (10 + 3).
        assert_eq!(map_through_splices(12, &splices), 13);
    }

    #[test]
    fn map_orders_deletion_then_insertion_correctly() {
        // The regression case for the original-p comparison frame: a deletion
        // BEFORE an insertion. A walking position compared against unshifted
        // starts would break at the insertion and drop its shift.
        let splices = vec![
            SourceEdit { start: 0, end: 10, replacement: String::new() },       // delete 10
            SourceEdit { start: 12, end: 12, replacement: "ZZZZ".to_string() }, // insert 4
        ];
        // p=20 (past both): −10 +4 → 14.
        assert_eq!(map_through_splices(20, &splices), 14);
        // p=11 (between them): only the deletion applies → 1.
        assert_eq!(map_through_splices(11, &splices), 1);
        // p=5 (inside the deleted region): clamps to 0.
        assert_eq!(map_through_splices(5, &splices), 0);
    }

    #[test]
    fn map_composes_across_two_passes_like_the_fix_loop() {
        // Mirrors the bin's fix loop: offsets are computed (not hardcoded) so
        // the test survives string edits. Pass 1 strips a `return` in fn a;
        // pass 2 collapses fn b to one line (line count −2). A pass-1 edit
        // recorded INSIDE fn b must remap to fn b's final position.
        let pass1_src = "fn a() -> u8 {\n    return 7;\n}\n\nfn b() -> u8 {\n    return 8;\n}\n";
        let ret_a = pass1_src.find("return 7;").expect("ret a");
        let (pass2_src, s1) = apply_edits_traced(
            pass1_src,
            vec![SourceEdit { start: ret_a, end: ret_a + "return 7;".len(), replacement: "7".to_string() }],
        );
        let ret_b_in_pass1 = pass1_src.find("return 8;").expect("ret b");
        let fn_b_in_pass2 = pass2_src.find("fn b").expect("fn b");
        let close_rel = pass2_src[fn_b_in_pass2..].find("}\n").expect("close");
        let fn_b_end = fn_b_in_pass2 + close_rel + 1; // include the `}`
        let (final_src, s2) = apply_edits_traced(
            &pass2_src,
            vec![SourceEdit {
                start: fn_b_in_pass2,
                end: fn_b_end,
                replacement: "fn b() -> u8 { 8 }".to_string(),
            }],
        );
        assert!(final_src.contains("fn b() -> u8 { 8 }"));
        assert_eq!(final_src.matches('\n').count(), 5);

        // A pass-1 edit at fn b's `return` (pass-1 INPUT coords): first map
        // through pass 1's own splices (the −8 shrink before it), then pass 2
        // collapses the region containing it → clamps to fn b's start.
        let pos_after_pass1 = map_through_splices(ret_b_in_pass1, &s1);
        let pos_final = map_through_splices(pos_after_pass1, &s2);
        assert_eq!(pos_final, fn_b_in_pass2);
        let line = final_src[..pos_final].matches('\n').count() + 1;
        assert_eq!(line, 5, "fn b starts on line 5 of the final text");

        // A pass-1 edit BEFORE the collapse (fn a's `return`): unmoved by
        // pass 2 (the splice starts after it); line 2 in the final text.
        let pos_a = map_through_splices(ret_a, &s1);
        let pos_a = map_through_splices(pos_a, &s2);
        assert_eq!(pos_a, ret_a); // same length replacement → no shift
        let line = final_src[..pos_a].matches('\n').count() + 1;
        assert_eq!(line, 2);
    }
}
