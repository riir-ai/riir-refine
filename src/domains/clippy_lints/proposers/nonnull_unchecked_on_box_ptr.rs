//! `clippy::nonnull_unchecked_on_box_ptr` —
//! `unsafe { NonNull::new_unchecked(Box::into_raw(x)) }` →
//! `NonNull::from_mut(Box::leak(x))` (Batch 230 / Plan 232; complexity
//! group, NEW in clippy 1.99 / PR #17336).
//!
//! Probe note (2026-10-13, clippy 1.99.0 isolated toolchain): the shape
//! fires with a `MachineApplicable` suggestion and a `.fixed` file
//! upstream; the fixed form compile-verified at the repo's 1.98.1
//! toolchain (only the ancient `BOX_LEAK` MSRV gate applies upstream —
//! `from_mut` compiles everywhere we ship).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! Pointer identity: `Box::into_raw(x)` hands out the allocation's
//! address and deliberately leaks the Box; `Box::leak(x)` hands out a
//! `&mut` to the SAME allocation and leaks the Box the same way
//! (leak-on-both-sides — identical ownership consequences).
//! `NonNull::new_unchecked` re-wraps that address unchecked;
//! `NonNull::from_mut` re-wraps a `&mut T` — provably non-null, so the
//! rewrite produces the SAME pointer value with the same ownership while
//! replacing unsafe code with safe code (safe-replaces-unsafe — strictly
//! stronger than identity).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The unsafe block's body is EXACTLY one `Stmt::Expr(call, None)` —
//!   no semicolon. Extra statements, a semicolon'd tail, and attrs on
//!   the unsafe block or either call refuse (the splice replaces the
//!   whole `Expr::Unsafe` span and would drop them).
//! - Path normalization: the outer call must be
//!   `NonNull::new_unchecked` or `std::ptr::NonNull::new_unchecked`; the
//!   inner must be `Box::into_raw` or `std::boxed::Box::into_raw` — a
//!   last-two-segment match with the std prefix optional. qself,
//!   leading colon, ANY generic arguments (turbofish), and
//!   aliased/re-prefixed spellings (`ptr::NonNull::…`, or `NP::…` after
//!   `use std::ptr::NonNull as NP`) refuse — parse-only cannot resolve
//!   imports, so only the canonical spellings are trusted.
//! - EXACTLY ONE argument at both call levels; the innermost payload is
//!   spliced VERBATIM (any expression shape; text starting with `#` —
//!   an attribute riding the span — refuses).
//! - A `SAFETY` marker (case-insensitive) on the line(s) immediately
//!   above the unsafe block refuses — the rewrite removes the `unsafe`,
//!   which would orphan the justification comment.
//! - Comments inside the replaced span refuse (`drops_interior_comment`,
//!   kept = the payload's own span). An admitted block is not descended
//!   into: the payload rides the replacement verbatim, and nested sites
//!   heal on the next pass — never as overlapping edits.

use super::helpers::{byte_range, drops_interior_comment};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprPath, ExprUnsafe, File, Stmt};

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
    /// The original source (span → text lookups).
    source: &'a str,
    /// Splice edits accumulated by the traversal.
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_unsafe(&mut self, u: &'ast ExprUnsafe) {
        if let Some(e) = self.try_unwrap(u) {
            self.edits.push(edit(u, e));
            // The whole span is replaced — nothing inside survives as
            // original text except the payload (already spliced verbatim),
            // so do not descend (no overlapping edits).
            return;
        }
        syn::visit::visit_expr_unsafe(self, u);
    }
}

impl Collector<'_> {
    /// `NonNull::from_mut(Box::leak(<payload>))` when `u` is exactly the
    /// lint's shape; `None` refuses (see module guards).
    fn try_unwrap(&self, u: &ExprUnsafe) -> Option<String> {
        if !u.attrs.is_empty() {
            return None;
        }
        let [Stmt::Expr(Expr::Call(outer), None)] = u.block.stmts.as_slice() else {
            return None; // 0/2+ statements, or a semicolon'd tail
        };
        if !outer.attrs.is_empty() {
            return None;
        }
        if outer.args.len() != 1 {
            return None;
        }
        let Expr::Path(fp) = &*outer.func else {
            return None;
        };
        if !is_path_call(fp, &["std", "ptr"], "NonNull", "new_unchecked") {
            return None;
        }
        let Expr::Call(inner) = outer.args.first()? else {
            return None; // not a Box::into_raw call at all
        };
        if !inner.attrs.is_empty() {
            return None;
        }
        if inner.args.len() != 1 {
            return None;
        }
        let Expr::Path(ip) = &*inner.func else {
            return None;
        };
        if !is_path_call(ip, &["std", "boxed"], "Box", "into_raw") {
            return None;
        }
        let payload = inner.args.first()?;
        let payload_text = src(self.source, payload)?;
        if payload_text.trim_start().starts_with('#') {
            return None; // an attribute riding the payload span
        }
        let u_range = byte_range(u);
        if drops_interior_comment(self.source, u_range.clone(), &[byte_range(payload)]) {
            return None;
        }
        if safety_comment_above(self.source, u_range.start) {
            return None;
        }
        Some(format!("NonNull::from_mut(Box::leak({payload_text}))"))
    }
}

/// Is `p` the call-target path `tail0::tail1`, optionally prefixed by the
/// full `prefix` chain (`std::ptr::NonNull::new_unchecked`)? Refuses
/// qself, leading colon, generic arguments (turbofish), and every other
/// length/spelling — parse-only cannot resolve imports, so aliased or
/// re-prefixed spellings (`ptr::…`, `NP::…`) stay untouched.
fn is_path_call(p: &ExprPath, prefix: &[&str], tail0: &str, tail1: &str) -> bool {
    if p.qself.is_some() || p.path.leading_colon.is_some() {
        return false;
    }
    let segs = &p.path.segments;
    let n = segs.len();
    if n != 2 && n != prefix.len() + 2 {
        return false;
    }
    if segs.iter().any(|s| !s.arguments.is_none()) {
        return false;
    }
    if segs[n - 2].ident != tail0 || segs[n - 1].ident != tail1 {
        return false;
    }
    if n == 2 {
        return true;
    }
    prefix
        .iter()
        .zip(segs.iter())
        .all(|(want, seg)| seg.ident == *want)
}

/// Does a `SAFETY` marker (case-insensitive) sit on the first content
/// line above the byte offset `at` (skipping one blank line)? Removing an
/// `unsafe` block under such a comment would orphan the justification.
fn safety_comment_above(source: &str, at: usize) -> bool {
    let mut line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
    for _ in 0..2 {
        if line_start == 0 {
            return false;
        }
        let prev_end = line_start - 1; // the '\n' byte itself
        let prev_start = source[..prev_end].rfind('\n').map_or(0, |i| i + 1);
        let line = &source[prev_start..prev_end];
        if !line.trim().is_empty() {
            return line.to_ascii_lowercase().contains("safety");
        }
        line_start = prev_start;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "nonnull_unchecked_on_box_ptr";

    #[test]
    fn seed_is_clean() {
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u8>, x: u8) -> u8 { hits.push(0); x }",
        )
        .is_none());
    }

    #[test]
    fn basic_shape_fires() {
        let out = propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> { unsafe { NonNull::new_unchecked(Box::into_raw(x)) } }",
        )
        .expect("matched");
        assert!(out.contains("NonNull::from_mut(Box::leak(x))"), "not rewritten: {out}");
        assert!(!out.contains("unsafe"), "unsafe block not removed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn payload_call_fires() {
        let out = propose_fix(
            RULE,
            "fn identity<T>(v: T) -> T { v } fn f(one: u8) -> NonNull<u8> { unsafe { NonNull::new_unchecked(Box::into_raw(identity(one))) } }",
        )
        .expect("matched");
        assert!(out.contains("Box::leak(identity(one))"), "payload not spliced verbatim: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn payload_method_call_fires() {
        let out = propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> { unsafe { NonNull::new_unchecked(Box::into_raw(x.clone())) } }",
        )
        .expect("matched");
        assert!(out.contains("Box::leak(x.clone())"), "payload not spliced verbatim: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn std_qualified_both_paths_fires() {
        let out = propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> { unsafe { std::ptr::NonNull::new_unchecked(std::boxed::Box::into_raw(x)) } }",
        )
        .expect("matched");
        assert!(out.contains("NonNull::from_mut(Box::leak(x))"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn inside_let_fires() {
        let out = propose_fix(
            RULE,
            "fn f(x: Box<u8>) { let p = unsafe { NonNull::new_unchecked(Box::into_raw(x)) }; drop(p); }",
        )
        .expect("matched");
        assert!(out.contains("let p = NonNull::from_mut(Box::leak(x));"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn extra_statement_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> { unsafe { check(); NonNull::new_unchecked(Box::into_raw(x)) } }",
        )
        .is_none());
    }

    #[test]
    fn semicolon_tail_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(x: Box<u8>) { unsafe { NonNull::new_unchecked(Box::into_raw(x)); } }",
        )
        .is_none());
    }

    #[test]
    fn safety_comment_above_refuses() {
        // Removing the unsafe block would orphan its justification.
        assert!(propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> {\n    // SAFETY: into_raw never returns null.\n    unsafe { NonNull::new_unchecked(Box::into_raw(x)) }\n}",
        )
        .is_none());
    }

    #[test]
    fn non_box_arg_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(r: *mut u8) -> NonNull<u8> { unsafe { NonNull::new_unchecked(r) } }",
        )
        .is_none());
    }

    #[test]
    fn aliased_import_refuses() {
        // `NP::new_unchecked` is unresolvable at parse time — only the
        // canonical spellings are trusted.
        assert!(propose_fix(
            RULE,
            "use std::ptr::NonNull as NP; fn f(x: Box<u8>) -> NP<u8> { unsafe { NP::new_unchecked(Box::into_raw(x)) } }",
        )
        .is_none());
    }

    #[test]
    fn turbofish_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> { unsafe { NonNull::<u8>::new_unchecked(Box::into_raw(x)) } }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(x: Box<u8>) -> NonNull<u8> { unsafe { /* keep */ NonNull::new_unchecked(Box::into_raw(x)) } }",
        )
        .is_none());
    }

    #[test]
    fn box_into_raw_two_args_refuses() {
        assert!(propose_fix(
            RULE,
            "fn f(x: Box<u8>, y: u8) -> NonNull<u8> { unsafe { NonNull::new_unchecked(Box::into_raw(x, y)) } }",
        )
        .is_none());
    }
}
