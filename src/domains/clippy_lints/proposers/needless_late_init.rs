//! `clippy::needless_late_init` — `let x[: T]; x = <expr>;` →
//! `let x[: T] = <expr>;` (Batch 163 / Plan 154 — the frontier-lead mining
//! pack IX; the family fixture verified to fire exactly this lint under
//! all+pedantic+nursery with a clean seed, 2026-09-19
//! (`scripts/plan154_frontier_probe.py`); the twice-used-binding `x + x`
//! tail dodges our own `let_and_return` on the seed — the recorded
//! plan152 spare dodge).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! The let initializer evaluates at EXACTLY the position the assignment
//! statement evaluated: same statement position, same temporaries, same
//! drop order. There is no intermediate value to drop — the original pair
//! binds `expr` to `x` once, and so does the rewrite (a non-`mut` binding
//! cannot be re-assigned at all — E0384 — so an adjacent assignment to a
//! non-`mut` binding IS the initialization, by the language's own
//! definition).
//!
//! ## Guards (the bounded fix-space — clippy's own scope, probe-pinned)
//!
//! - ADJACENT pair in the same block: the `let` is immediately followed by
//!   the assignment statement. An intervening statement SILENCES clippy
//!   (probe: `nli-intervening` `[]`) — mirrored exactly.
//! - The binding must be non-`mut`: `let mut x; x = v;` is SILENT upstream
//!   (probe: `nli-mut` — the lint's fix would also drop the unneeded `mut`,
//!   a different seam) — refused here, which also makes the exactly-once
//!   assignment a language fact (any further mutation requires `mut`).
//!   (`let x; x = 1; x = 2;` is E0384 without `mut` — the shape cannot
//!   exist in compiling code.)
//! - The pattern must be a plain ident (no `ref`, no subpattern, no
//!   tuple/struct destructuring); branch initializers (`if c { x = 1; }
//!   else { x = 2; }`) refuse — clippy's actionable suggestion there comes
//!   from `useless_let_if_seq`, a different seam (probe-pinned).
//! - The type annotation rides verbatim (`let x: u32; x = 5;` →
//!   `let x: u32 = 5;`, probe-pinned); the initializer re-emits verbatim
//!   (`?`/`.await`/blocks stay in place — the probe confirms clippy
//!   suggests moving them, and the adjacency guard makes the move
//!   order-identical).
//! - Attributes on the `let`, or on the assignment expression, refuse
//!   (attrs on the disappearing statement would be dropped).
//! - Comments between the two statements refuse (the pair re-spells to one
//!   statement; comments inside the re-emitted fragments survive verbatim).

use syn::visit::Visit;
use syn::{Expr, File, Pat, Stmt};

use super::helpers::byte_range;
use super::SourceEdit;
use crate::edit_emitter::drops_interior_comment;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { source, edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_block(&mut self, b: &'ast syn::Block) {
        // Pair scan (no recursion into the pair decision — the guard is
        // statement adjacency within ONE block); nested blocks are visited
        // by the default walk and get their own scan.
        for pair in b.stmts.windows(2) {
            if let Some(edit) = self.try_pair(&pair[0], &pair[1]) {
                self.edits.push(edit);
            }
        }
        syn::visit::visit_block(self, b);
    }
}

impl Collector<'_> {
    fn try_pair(&mut self, first: &Stmt, second: &Stmt) -> Option<SourceEdit> {
        // Statement 1: the init-less local, plain non-mut ident pattern
        // (optionally type-annotated — syn 2.x folds `pat: Type` into
        // Pat::Type), no attributes.
        let Stmt::Local(local) = first else { return None };
        if local.init.is_some() || !local.attrs.is_empty() {
            return None;
        }
        let ident_pat;
        let _ty = match &local.pat {
            Pat::Ident(pi) => {
                ident_pat = pi;
                None
            }
            Pat::Type(pt) => match &*pt.pat {
                Pat::Ident(pi) => {
                    ident_pat = pi;
                    Some(&pt.ty)
                }
                _ => return None,
            },
            _ => return None,
        };
        if ident_pat.by_ref.is_some() || ident_pat.mutability.is_some() || ident_pat.subpat.is_some()
        {
            return None;
        }
        let ident = &ident_pat.ident;

        // Statement 2: `x = <expr>;` — a semicolon-terminated assignment
        // statement, bare-ident lhs matching the local, no attributes.
        let Stmt::Expr(Expr::Assign(assign), Some(_semi)) = second else {
            return None;
        };
        if !assign.attrs.is_empty() {
            return None;
        }
        let Expr::Path(lhs) = &*assign.left else { return None };
        if lhs.qself.is_some()
            || lhs.path.leading_colon.is_some()
            || lhs.path.segments.len() != 1
            || !lhs.path.segments[0].arguments.is_none()
            || lhs.path.segments[0].ident != *ident
        {
            return None;
        }

        // syn folds statement attributes into the STATEMENT span and
        // RESPANS the inner expression nodes to cover it (the lhs path's
        // span starts AT the attribute — measured), so no structural span
        // comparison can see the tokens. Textual check instead: the
        // statement's first non-whitespace text must BE the lhs ident — an
        // attribute (`#[…]`) or any other token before it refuses, because
        // the rewrite would drop it.
        let head = self
            .source
            .get(byte_range(second).start..)?
            .trim_start();
        if !head.starts_with(ident.to_string().as_str()) {
            return None;
        }

        // The pattern + type annotation are carried verbatim, so their
        // ranges are kept; the pat range is the pattern node (Pat::Type
        // covers `x: u32` — the whole annotated pattern rides verbatim).
        let pat_range = byte_range(&local.pat);
        let rhs_range = byte_range(&assign.right);
        let span_start = byte_range(first).start;
        let span_end = byte_range(second).end;
        let kept = [pat_range.clone(), rhs_range.clone()];
        if drops_interior_comment(self.source, span_start..span_end, &kept) {
            return None;
        }

        let pat_src = self.source.get(pat_range)?.to_string();
        let rhs_src = self.source.get(rhs_range)?.to_string();
        let replacement = format!("let {pat_src} = {rhs_src};");
        // The edit's span comes from the SECOND node's node-span machinery
        // only for sizing; build the explicit pair span here.
        Some(SourceEdit {
            start: span_start,
            end: span_end,
            replacement,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x + x }",
        )
        .is_none());
    }

    #[test]
    fn adjacent_pair_fires() {
        let out = propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x + x }",
        )
        .expect("matched");
        assert!(out.contains("let x = 5u32;"), "not merged: {out}");
        assert!(!out.contains("let x;"), "bare let survived: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn annotation_rides_verbatim() {
        let out = propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x: u32; x = 5u32; x + x }",
        )
        .expect("matched");
        assert!(out.contains("let x: u32 = 5u32;"), "annotation lost: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn call_initializer_fires() {
        let out = propose_fix(
            "needless_late_init",
            "fn mk(hits: &mut Vec<u8>) -> u32 { hits.push(9); 7 }\nfn late(hits: &mut Vec<u8>) -> u32 { let x; x = mk(hits); x + x }",
        )
        .expect("matched");
        assert!(out.contains("let x = mk(hits);"), "call not moved: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_block_pair_fires_in_place() {
        let out = propose_fix(
            "needless_late_init",
            "fn late(c: bool, hits: &mut Vec<u8>) -> u32 { hits.push(0); if c { let y; y = 3u32; y + y } else { 0 } }",
        )
        .expect("matched");
        assert!(out.contains("let y = 3u32;"), "nested pair not merged: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mut_binding_refuses() {
        // clippy is SILENT on `let mut x; x = v;` (probe: nli-mut) — the
        // fix would drop the mut, a different seam.
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let mut x; x = 5u32; x + x }",
        )
        .is_none());
    }

    #[test]
    fn intervening_statement_refuses() {
        // clippy is SILENT with anything between the pair (probe).
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { let x; hits.push(1); x = 5u32; hits.push(x); x + x }",
        )
        .is_none());
    }

    #[test]
    fn branch_initializer_refuses() {
        // The actionable upstream suggestion is useless_let_if_seq's — a
        // different seam (probe-pinned).
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>, c: bool) -> u32 { hits.push(0); let x; if c { x = 1u32; } else { x = 2u32; } x + x }",
        )
        .is_none());
    }

    #[test]
    fn ref_pattern_refuses() {
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let ref x; x = &1u32; 3 }",
        )
        .is_none());
    }

    #[test]
    fn destructure_pattern_refuses() {
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let (a, b); (a, b) = (1u32, 2u32); a + b }",
        )
        .is_none());
    }

    #[test]
    fn field_assignment_statement_refuses() {
        // The assignment statement itself is a field place — not the
        // bare-ident shape.
        assert!(propose_fix(
            "needless_late_init",
            "struct S { f: u32 }\nfn late(hits: &mut Vec<u8>, s: &mut S) { hits.push(0); let f; s.f = 5; }",
        )
        .is_none());
    }

    #[test]
    fn non_semi_assign_tail_refuses() {
        // An assignment as a block's TAIL (no semicolon) is not the
        // statement shape.
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) { hits.push(0); let x; x = hits.len() as u32 }",
        )
        .is_none());
    }

    #[test]
    fn comment_between_refuses() {
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; /* init */ x = 5u32; x + x }",
        )
        .is_none());
    }

    #[test]
    fn attr_on_assign_refuses() {
        // syn folds statement attributes into the STATEMENT span and
        // RESPANS the inner expressions over it (Expr::Assign.attrs reads
        // empty, the lhs span starts AT the attribute — measured) — the
        // textual leading-check is what catches it.
        assert!(propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; #[allow(unused)] x = 5u32; x + x }",
        )
        .is_none());
    }

    #[test]
    fn comment_inside_initializer_survives() {
        // A comment INSIDE the initializer expression rides verbatim (the
        // rhs re-emits from source); only the gap between the statements
        // refuses.
        let out = propose_fix(
            "needless_late_init",
            "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 1u32 /* keep */ + 2u32; x + x }",
        )
        .expect("matched");
        assert!(out.contains("let x = 1u32 /* keep */ + 2u32;"), "rhs comment lost: {out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
