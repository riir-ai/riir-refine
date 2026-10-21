//! `clippy::unit_arg` — `touch({ hits.push(1); }, hits)` →
//! `{ hits.push(1); };\ntouch((), hits)` (the side-effect block HOISTED
//! before the call, the arg re-spelled as the unit literal) — Batch 161 /
//! Plan 152 — the frontier-lead mining pack VII; family fixture verified
//! to fire exactly this lint under all+pedantic+nursery AND the
//! default-groups arm (complexity/warn), 2026-09-18
//! (`scripts/plan152_frontier_probe.py`).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! A block with NO tail expression has type `()` by the language
//! definition — the last statement being a `;`-statement (or a
//! local/item/macro stmt) means the block evaluates to unit. So the
//! argument's VALUE is already `()`; spelling it as the literal `()` is
//! value-identical, and the block's side effects ride verbatim hoisted
//! BEFORE the call. Evaluation order is identical by construction: Rust
//! evaluates call arguments left-to-right, so hoisting the block at
//! argument index 0 (the only index where order is provable without type
//! knowledge) preserves the exact sequence — block, then the remaining
//! arguments, then the call. Borrow scoping is unchanged: the block ends
//! before the call either way. This is byte-for-byte the shape of
//! clippy's own `MachineApplicable` suggestion (measured by the plan152
//! probe: `'{ hits.push(1); };\ntouch((), hits)'`).
//!
//! ## The in-place arm
//!
//! An EMPTY block `{}` as an argument (any index — no side effects, no
//! order concern) is replaced by `()` in place (clippy's suggestion for
//! the shape is exactly `()`, measured).
//!
//! ## Guards (the bounded fix-space)
//!
//! - The unit block must be the FIRST argument (index 0) and the ONLY
//!   block argument of the call.
//! - The call must be a DIRECT statement-or-tail child of a block — the
//!   hoist emits two statements, which needs statement context. Calls
//!   nested in larger expressions (`let x = touch({...}, h);`, method
//!   chains, method calls) refuse.
//! - Block labels (`'a: { .. }`) and attributes on the block or call
//!   refuse.
//! - Comments in the deleted glue (outside the block span) refuse; the
//!   block text rides VERBATIM inside the replacement, so interior
//!   comments survive by construction.
//! - A block WITH a tail expression (`{ compute() }`) refuses — its value
//!   is live, and the lint stays silent on it (measured).

use syn::visit::Visit;
use syn::{Block, Expr, ExprCall, File, Stmt};

use super::helpers::byte_range;
use crate::edit_emitter::{drops_interior_comment, line_indent, SourceEdit};

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

impl Collector<'_> {
    /// Try the hoist/in-place rewrite on a call that sits in statement or
    /// tail position. Returns the (byte range, replacement) edit. `None` =
    /// refuse.
    fn try_rewrite(&self, c: &ExprCall) -> Option<(std::ops::Range<usize>, String)> {
        if !c.attrs.is_empty() || c.args.is_empty() {
            return None;
        }
        let first = c.args.first()?;
        // Index-0 gate + the only-block-arg guard.
        if c.args
            .iter()
            .enumerate()
            .any(|(i, a)| matches!(a, Expr::Block(_)) && !(i == 0 && std::ptr::eq(a, first)))
        {
            return None;
        }
        let Expr::Block(be) = first else { return None };
        if !be.attrs.is_empty() || be.label.is_some() {
            return None;
        }
        // The empty-block arm: in-place `()`, no hoist — the edit targets
        // the ARG span (the call text outside it rides untouched).
        if be.block.stmts.is_empty() {
            return Some((byte_range(first), "()".to_string()));
        }
        // The hoist arm needs the provably-unit block (no tail expr).
        if has_tail_expr(&be.block) {
            return None;
        }
        let block_span = byte_range(first);
        let call_span = byte_range(c);
        let arg_rel = block_span.start - call_span.start..block_span.end - call_span.start;
        // Comments in the re-spelled glue (call span minus the block span)
        // refuse; the block text rides verbatim.
        if drops_interior_comment(
            self.source,
            call_span.clone(),
            std::slice::from_ref(&block_span),
        ) {
            return None;
        }
        let block_text = self.source.get(block_span)?;
        let call_text = self.source.get(call_span.clone())?;
        let mut new_call = String::with_capacity(call_text.len());
        new_call.push_str(&call_text[..arg_rel.start]);
        new_call.push_str("()");
        new_call.push_str(&call_text[arg_rel.end..]);
        let indent = line_indent(self.source, call_span.start);
        Some((call_span, format!("{block_text};\n{indent}{new_call}")))
    }
}

/// Does this block END in a tail expression (a `Stmt::Expr` with no
/// semicolon)? Such a block's value is the tail's value — NOT unit.
fn has_tail_expr(b: &Block) -> bool {
    matches!(b.stmts.last(), Some(Stmt::Expr(_, None)))
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    /// Only calls that are DIRECT children of a block (statement or tail)
    /// are hoist-eligible — the rewrite emits two statements. Nested
    /// contexts (let inits, method chains) never reach a block-stmt match.
    fn visit_block(&mut self, b: &'ast Block) {
        for stmt in &b.stmts {
            let expr = match stmt {
                Stmt::Expr(e, _) => e,
                Stmt::Local(_) | Stmt::Item(_) | Stmt::Macro(_) => continue,
            };
            // Method calls are the lint's other firing shape, but the hoist
            // was probed on plain calls only — refuse (future work, the
            // shape arm list in Plan 152 T1).
            if let Expr::Call(c) = expr {
                if let Some((range, replacement)) = self.try_rewrite(c) {
                    self.edits.push(SourceEdit {
                        start: range.start,
                        end: range.end,
                        replacement,
                    });
                }
            }
        }
        syn::visit::visit_block(self, b);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const TOUCH_SEED: &str = "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }";
    const GO: &str = "fn go(hits: &mut Vec<u8>) {";

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "unit_arg",
            "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch((), hits) }",
        )
        .is_none());
    }

    #[test]
    fn hoist_arm_fires() {
        let out = propose_fix(
            "unit_arg",
            "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }",
        )
        .expect("matched");
        assert!(
            out.contains("{ hits.push(1); };") && out.contains("touch((), hits)"),
            "not hoisted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn second_family_case_fires() {
        let out = propose_fix(
            "unit_arg",
            "fn tap(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn run(hits: &mut Vec<u8>) { tap({ hits.push(2); }, hits) }",
        )
        .expect("matched");
        assert!(
            out.contains("{ hits.push(2); };") && out.contains("tap((), hits)"),
            "not hoisted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn stmt_with_semi_position_fires() {
        let out = propose_fix(
            "unit_arg",
            "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits); }",
        )
        .expect("matched");
        assert!(out.contains("touch((), hits);"), "not hoisted: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn empty_block_in_place() {
        let out = propose_fix(
            "unit_arg",
            "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({}, hits) }",
        )
        .expect("matched");
        assert!(out.contains("touch((), hits)"), "not in-place: {out}");
        assert!(!out.contains(";\n"), "empty block must not hoist: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn block_at_index_1_refuses() {
        // Hoisting past arg 0 would reorder evaluation — refuse (clippy
        // itself stays silent on the trailing-block shape, measured).
        assert!(propose_fix(
            "unit_arg",
            "fn touch(hits: &mut Vec<u8>, x: ()) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch(hits, { hits.push(1); }) }",
        )
        .is_none());
    }

    #[test]
    fn value_tail_block_refuses() {
        // `{ 1 }` has a live tail value — not unit, and the lint is silent.
        assert!(propose_fix(
            "unit_arg",
            "fn touch(x: u32, hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({ 1 }, hits) }",
        )
        .is_none());
    }

    #[test]
    fn labeled_block_refuses() {
        assert!(propose_fix(
            "unit_arg",
            &format!("{TOUCH_SEED}\n{GO} touch('a: {{ hits.push(1); }}, hits) }}"),
        )
        .is_none());
    }

    #[test]
    fn second_block_arg_refuses() {
        // Only-block-arg guard: two block args → refuse (the empty-block
        // in-place arm must not fire half the shape).
        assert!(propose_fix(
            "unit_arg",
            "fn touch(x: (), y: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({}, {}, hits) }",
        )
        .is_none());
    }

    #[test]
    fn nested_call_context_refuses() {
        // A let-init call has no statement context for the hoist.
        assert!(propose_fix(
            "unit_arg",
            &format!(
                "{TOUCH_SEED}\nfn go(hits: &mut Vec<u8>) {{ let x = touch({{ hits.push(1); }}, hits); }}",
            ),
        )
        .is_none());
    }

    #[test]
    fn method_call_refuses() {
        assert!(propose_fix(
            "unit_arg",
            "fn go(hits: &mut Vec<u8>) { hits.push({ hits.push(1); }) }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_glue_refuses() {
        assert!(propose_fix(
            "unit_arg",
            &format!("{TOUCH_SEED}\n{GO} touch(/* unit */ {{ hits.push(1); }}, hits) }}"),
        )
        .is_none());
    }

    #[test]
    fn comment_inside_block_rides() {
        let out = propose_fix(
            "unit_arg",
            &format!("{TOUCH_SEED}\n{GO} touch({{ /* tick */ hits.push(1); }}, hits) }}"),
        )
        .expect("matched");
        assert!(
            out.contains("/* tick */"),
            "interior comment dropped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
