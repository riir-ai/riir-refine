//! `clippy::items_after_statements` — an item (`fn`/`const`/`static`/
//! `struct`/…) declared after a non-item statement in the same block (Batch
//! 90; the Plan 049 T0.1 fired histogram: 39 real instances).
//!
//! Probe-verified firing shapes (clippy 1.97, pedantic, JSON; applicability
//! `Unspecified` — real `cargo clippy --fix` does NOT apply it, so the
//! proposer is genuinely additive): fn, const, static, and struct items
//! after a `let`; silent when all items precede the statements.
//!
//! Fix: hoist the items to the top of the block's statement list. This is
//! provably semantics-preserving for every compiling input: a block item
//! cannot capture block locals (that is precisely why `const K: i32 = x;`
//! inside a block is a compile error), and Rust name resolution is
//! order-independent WITHIN a block — the hoist stays inside the block, so
//! scope and resolution are identical, and items are never evaluated in
//! statement order.
//!
//! Shape: ONE atomic edit per block covering `[first_non_item_stmt ..
//! last_moved_item]` — replacement is the moved items (original relative
//! order) + a blank line + the region's non-item statements byte-verbatim
//! with their original inter-statement gaps. The `needless_range_loop`
//! atomicity lesson (Issue 032): a multi-statement reorder is one edit, so
//! a foreign same-pass fix can never splice into the middle of the moved
//! set. Nested-block edits inside the region resolve outer-first (dropped
//! this pass, picked up by the next fixpoint iteration).
//!
//! Guards:
//! - **Macro items are never moved** (`Item::Macro` — token opacity; the
//!   `use_self` nested-items precedent). They stay in place; the unguarded
//!   items still hoist.
//! - **Comment guards** (Issue 027/028 class — a hoist would leave a
//!   comment attached to the wrong node): an item whose LEADING gap
//!   (previous node end → item start) carries a comment marker is skipped —
//!   the comment stays with the statements and the item would move away
//!   from its docs. An item with a same-line trailing comment is skipped —
//!   the note would be displaced onto whatever follows the moved span.
//!   Multi-item blocks hoist the unguarded subset (a partial hoist is still
//!   an improvement, and the fixpoint is idempotent).
//! - Comment-guarded leftovers are an honest divergence: real clippy keeps
//!   firing on an item we refuse to move (we are SAFER — the rewrite would
//!   displace the comment), the same class as the comment-guarded
//!   `single_match_else` refusals.

use super::helpers::{byte_range, comment_marker_positions};
use super::{SourceEdit, src};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, File, Item, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    /// Every block is checked (fn bodies, loop bodies, if branches, match
    /// arms). The hoist edit spans multiple statements, so nested-block
    /// edits inside the region are resolved outer-first by the emitter
    /// (dropped this pass; the fixpoint re-runs them).
    fn visit_block(&mut self, block: &'ast Block) {
        if let Some(e) = self.try_hoist(block) {
            self.edits.push(e);
        }
        syn::visit::visit_block(self, block);
    }
}

fn is_item(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Item(_))
}

/// Macro items stay in place (token opacity); every other item kind hoists.
fn movable(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Item(i) if !matches!(i, Item::Macro(_)))
}

impl Collector<'_> {
    fn try_hoist(&self, block: &Block) -> Option<SourceEdit> {
        if block.stmts.len() < 2 {
            return None;
        }
        // The first non-item statement — nothing before it can fire.
        let first_stmt = block.stmts.iter().position(|s| !is_item(s))?;
        // Candidate items after it, comment-guarded per item.
        let mut moved: Vec<usize> = Vec::new();
        for (idx, stmt) in block.stmts.iter().enumerate().skip(first_stmt + 1) {
            if !movable(stmt) {
                continue;
            }
            if self.item_comment_guarded(&block.stmts, idx) {
                continue;
            }
            moved.push(idx);
        }
        if moved.is_empty() {
            return None;
        }
        let last_moved = *moved.last().expect("non-empty");
        let region_start = block.stmts[first_stmt].span().byte_range().start;
        let region_end = block.stmts[last_moved].span().byte_range().end;

        // Items text (original relative order, one per line).
        let mut parts: Vec<String> = Vec::with_capacity(moved.len());
        for &idx in &moved {
            parts.push(src(self.source, &block.stmts[idx])?);
        }
        let items_text = parts.join("\n");

        // Region text minus the moved spans, preserving every kept
        // statement and its preceding gap byte-verbatim.
        let mut kept = String::with_capacity(region_end - region_start);
        let mut cursor = region_start;
        for (offset, stmt) in block.stmts[first_stmt..].iter().enumerate() {
            let abs = first_stmt + offset;
            let r = stmt.span().byte_range();
            if r.end > region_end {
                break;
            }
            if moved.contains(&abs) {
                cursor = r.end; // consume the moved span and its leading gap
                continue;
            }
            kept.push_str(self.source.get(cursor..r.start)?);
            kept.push_str(self.source.get(r.clone())?);
            cursor = r.end;
        }

        let replacement = format!("{items_text}\n\n{kept}");
        Some(SourceEdit {
            start: region_start,
            end: region_end,
            replacement,
        })
    }

    /// Would moving `stmts[idx]` displace a comment? Two checks: the gap
    /// ABOVE the item (its leading docs stay behind) and the same-line gap
    /// BELOW it (a trailing note would land on the wrong node). Multi-line
    /// gaps below the item belong to the next node and are fine.
    fn item_comment_guarded(&self, stmts: &[Stmt], idx: usize) -> bool {
        let item_range = byte_range(&stmts[idx]);
        // Leading gap: from the previous node's end to this item's start.
        let prev_end = if idx == 0 {
            item_range.start
        } else {
            byte_range(&stmts[idx - 1]).end
        };
        if let Some(gap) = self.source.get(prev_end..item_range.start) {
            if !comment_marker_positions(gap).is_empty() {
                return true;
            }
        }
        // Trailing same-line gap: up to the next node's start (or end of
        // source); only the FIRST line can carry the item's own note.
        let next_start = stmts
            .get(idx + 1)
            .map_or(self.source.len(), |s| byte_range(s).start);
        if let Some(gap) = self.source.get(item_range.end..next_start) {
            let first_line = gap.split('\n').next().unwrap_or("");
            if !comment_marker_positions(first_line).is_empty() {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fn_after_let_hoists() {
        let out = propose_fix(
            "items_after_statements",
            "fn f() -> i32 { let x = 1; fn h(v: i32) -> i32 { v * 2 } h(x) }",
        )
        .expect("pattern matched");
        let h_pos = out.find("fn h").expect("h present");
        let x_pos = out.find("let x").expect("x present");
        assert!(h_pos < x_pos, "item must precede statements: {out}");
        assert!(out.contains("h(x) }"), "tail lost: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn const_and_static_hoist() {
        let out = propose_fix(
            "items_after_statements",
            "fn f() -> i32 { let x = 2; const K: i32 = 7; static S: i32 = 9; x + K + S }",
        )
        .expect("pattern matched");
        let k_pos = out.find("const K").expect("K");
        let s_pos = out.find("static S").expect("S");
        let x_pos = out.find("let x").expect("x");
        assert!(k_pos < x_pos && s_pos < x_pos, "{out}");
        assert!(k_pos < s_pos, "relative item order kept: {out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn struct_after_let_hoists() {
        let out = propose_fix(
            "items_after_statements",
            "fn f() -> u32 { let y = 3u32; struct P { a: u32 } P { a: y }.a }",
        )
        .expect("pattern matched");
        assert!(out.find("struct P").unwrap() < out.find("let y").unwrap(), "{out}");
        assert!(out.contains("P { a: y }.a }"), "{out}");
    }

    #[test]
    fn items_first_silent() {
        let out = propose_fix(
            "items_after_statements",
            "fn f() -> i32 { fn h(v: i32) -> i32 { v } let x = 1; h(x) }",
        );
        assert!(out.is_none(), "items-first must not fire: {out:?}");
    }

    #[test]
    fn two_items_hoist_in_order() {
        let out = propose_fix(
            "items_after_statements",
            "fn f() -> i32 { let x = 1; fn h1() -> i32 { 1 } fn h2() -> i32 { 2 } h1() + h2() + x }",
        )
        .expect("pattern matched");
        let h1 = out.find("fn h1").expect("h1");
        let h2 = out.find("fn h2").expect("h2");
        let x = out.find("let x").expect("x");
        assert!(h1 < h2 && h2 < x, "{out}");
        assert!(out.contains("h1() + h2() + x }"), "{out}");
    }

    #[test]
    fn interleaved_item_hoists_kept_stmts_untouched() {
        // [S1, I1, S2] — S2 must survive verbatim (gap included).
        let out = propose_fix(
            "items_after_statements",
            "fn f() -> i32 { let a = 1; fn h() -> i32 { 5 } let b = 2; a + b + h() }",
        )
        .expect("pattern matched");
        assert!(out.find("fn h").unwrap() < out.find("let a").unwrap(), "{out}");
        assert!(out.contains("let b = 2;"), "kept stmt lost: {out}");
        assert!(out.contains("a + b + h() }"), "{out}");
    }

    #[test]
    fn leading_comment_guards_the_item() {
        // The comment above h describes h — hoisting would displace it.
        let input = "fn f() -> i32 { let x = 1; // h doubles x\n fn h(v: i32) -> i32 { v * 2 } h(x) }";
        let out = propose_fix("items_after_statements", input);
        assert!(out.is_none(), "leading-comment item must be refused: {out:?}");
    }

    #[test]
    fn trailing_same_line_comment_guards_the_item() {
        let input = "fn f() -> i32 { let x = 1; fn h(v: i32) -> i32 { v * 2 } // helper\n h(x) }";
        let out = propose_fix("items_after_statements", input);
        assert!(out.is_none(), "trailing-comment item must be refused: {out:?}");
    }

    #[test]
    fn mid_gap_comment_guards_the_item() {
        // A comment in the item's LEADING gap would stay behind while the
        // item moves — the hoist is refused (conservative; real clippy keeps
        // firing on it, the documented we-stay-silent class).
        let input = "fn f() -> i32 { let a = 1; /* mid */ fn h() -> i32 { 5 } let b = 2; a + b + h() }";
        let out = propose_fix("items_after_statements", input);
        assert!(out.is_none(), "mid-gap comment must guard: {out:?}");
    }

    #[test]
    fn macro_item_stays_but_others_hoist() {
        let input = "macro_rules! m { () => {} }\nfn f() -> i32 { let x = 1; m!(); const K: i32 = 3; x + K }";
        // `m!();` is a statement (Stmt::Macro), not an item — the const
        // after it still fires and hoists past it.
        let out = propose_fix("items_after_statements", input).expect("pattern matched");
        assert!(out.find("const K").unwrap() < out.find("let x").unwrap(), "{out}");
        assert!(out.contains("m!();"), "statement must survive: {out}");
    }

    #[test]
    fn attributed_item_hoists_with_its_attr() {
        let input = "fn f() -> i32 { let x = 1; #[inline] fn h(v: i32) -> i32 { v } h(x) }";
        let out = propose_fix("items_after_statements", input).expect("pattern matched");
        assert!(
            out.contains("#[inline]"),
            "attribute must move with the item: {out}"
        );
        assert!(out.find("#[inline]").unwrap() < out.find("let x").unwrap(), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn nested_fn_block_hoists_independently() {
        let input = "fn f() -> i32 { fn outer() -> i32 { let y = 1; fn inner() -> i32 { 9 } inner() + y } let x = 2; outer() + x }";
        let out = propose_fix("items_after_statements", input).expect("pattern matched");
        // Both blocks hoist: inner (inside outer) and outer's own body.
        let inner = out.find("fn inner").expect("inner");
        let y = out.find("let y").expect("y");
        assert!(inner < y, "inner block hoisted: {out}");
        assert!(out.find("fn outer").unwrap() < out.find("let x").unwrap(), "{out}");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn idempotent_after_hoist() {
        let once = propose_fix(
            "items_after_statements",
            "fn f() -> i32 { let x = 1; fn h(v: i32) -> i32 { v } h(x) }",
        )
        .expect("pattern matched");
        let twice = propose_fix("items_after_statements", &once);
        assert!(twice.is_none(), "re-run must be a no-op: {twice:?}");
    }
}
