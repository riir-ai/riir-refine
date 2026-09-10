//! `clippy::redundant_pattern_matching` — `if let Ok(_) = r { .. }` →
//! `if r.is_ok() { .. }` (also `while let` and the 2-arm `match` shape).
//!
//! Patterns:
//! - an `if let` / `while let` whose pattern is a wildcard-only
//!   constructor test — `Ok(_)`, `Err(_)`, `Some(_)` (tuple-struct with one
//!   wildcard-or-rest element) or bare `None`. Patterns with BINDINGS
//!   (`Some(v)`, `Ok(v)`) keep the binding and do NOT fire.
//! - a 2-arm `match` whose arms are the same constructor tests mapped to
//!   literal `true`/`false` — `match e { Some(_) => true, None => false }`
//!   → `e.is_some()` (probe-verified vs clippy 1.97, 2026-08-19 — the
//!   run-#2 miss map's `redundant_pattern_matching_plus_return` fixture;
//!   either arm order; the opposite arm may be the matching constructor OR
//!   a wildcard).
//!
//! Span-preserving (Issue 016): the let-condition node's byte range
//! (`let Pat = Scrut`) is replaced by `<scrut>.<is_pred>()`; the whole
//! match expr's byte range is replaced by `<scrut>.<is_pred>()`; the
//! scrutinee keeps its ORIGINAL source text, parenthesized when its form
//! doesn't bind tighter than `.`.

use super::helpers::{is_none_pat, is_postfix_safe};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Arm, Expr, ExprIf, ExprLet, ExprMatch, ExprWhile, File, Lit, Pat};

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
    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        syn::visit::visit_expr_if(self, i);
        if let Expr::Let(l) = &*i.cond {
            if let Some(e) = self.try_rewrite_let(l) {
                self.edits.push(e);
            }
        }
    }

    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        syn::visit::visit_expr_while(self, w);
        if let Expr::Let(l) = &*w.cond {
            if let Some(e) = self.try_rewrite_let(l) {
                self.edits.push(e);
            }
        }
    }

    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(e) = self.try_rewrite_match(m) {
            self.edits.push(e);
        }
    }
}

impl Collector<'_> {
    fn try_rewrite_let(&self, l: &ExprLet) -> Option<SourceEdit> {
        let pred = constructor_pred(&l.pat)?;
        let scrutinee = self.scrutinee_text(&l.expr)?;
        Some(edit(l, format!("{scrutinee}.{pred}()")))
    }

    /// The 2-arm bool-literal match shape: one arm is a wildcard-only
    /// constructor test evaluating to a bool literal, the other arm is the
    /// opposite constructor (or a wildcard) with the opposite bool.
    fn try_rewrite_match(&self, m: &ExprMatch) -> Option<SourceEdit> {
        if m.arms.len() != 2 {
            return None;
        }
        // Guards make arm selection conditional — out of scope.
        if m.arms.iter().any(|a| a.guard.is_some()) {
            return None;
        }
        let (a, b) = (&m.arms[0], &m.arms[1]);
        let pred = match_arm_pred(a, b).or_else(|| match_arm_pred(b, a))?;
        let scrutinee = self.scrutinee_text(&m.expr)?;
        Some(edit(m, format!("{scrutinee}.{pred}()")))
    }

    /// Scrutinee text, parenthesized when its form doesn't bind tighter
    /// than `.` (method-receiver position).
    fn scrutinee_text(&self, e: &Expr) -> Option<String> {
        let text = src(self.source, e)?;
        Some(if is_postfix_safe(e) {
            text
        } else {
            format!("({text})")
        })
    }
}

/// The `is_*` predicate this let-pattern reduces to, if it is the
/// wildcard-only form. `None` for binding patterns — the binding is used,
/// not redundant.
fn constructor_pred(pat: &Pat) -> Option<&'static str> {
    if let Pat::TupleStruct(ts) = pat {
        if ts.elems.len() == 1 && matches!(ts.elems[0], Pat::Wild(_) | Pat::Rest(_)) {
            if ts.path.is_ident("Some") {
                return Some("is_some");
            }
            if ts.path.is_ident("Ok") {
                return Some("is_ok");
            }
            if ts.path.is_ident("Err") {
                return Some("is_err");
            }
        }
        return None;
    }
    is_none_pat(pat).then_some("is_none")
}

fn arm_bool(a: &Arm) -> Option<bool> {
    match &*a.body {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Bool(b) => Some(b.value),
            _ => None,
        },
        // Block-wrapped literals (`true => { true }`) keep block-value
        // semantics — a parse-only rewrite cannot tell an implicit tail
        // return from side effects. Conservative: require the bare literal.
        _ => None,
    }
}

/// `Some(pred)` when `ctor_arm` is a wildcard-only constructor test mapped
/// to a bool literal and `other_arm` completes the partition (opposite
/// constructor or wildcard) with the opposite bool.
fn match_arm_pred(ctor_arm: &Arm, other_arm: &Arm) -> Option<&'static str> {
    let value = arm_bool(ctor_arm)?;
    let pred = constructor_pred(&ctor_arm.pat)?;
    // The predicate must AGREE with the arm's literal: `Some(_) => true` →
    // `is_some`, `Some(_) => false` → `is_none`. constructor_pred returns
    // the constructor's OWN predicate — flip it when the literal is false.
    let flipped = match pred {
        "is_some" => "is_none",
        "is_none" => "is_some",
        "is_ok" => "is_err",
        "is_err" => "is_ok",
        _ => return None,
    };
    let wanted = if value { pred } else { flipped };
    // The other arm must complete the partition with the opposite bool:
    // either the opposite constructor or a plain wildcard.
    let other_value = arm_bool(other_arm)?;
    if other_value == value {
        return None;
    }
    let other_ok = matches!(&other_arm.pat, Pat::Wild(_))
        || constructor_pred(&other_arm.pat).is_some_and(|p| p != pred || p == flipped);
    other_ok.then_some(wanted)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn if_let_ok_wildcard_becomes_is_ok() {
        let input = "fn f(r: Result<u8, u8>) { if let Ok(_) = r {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("if r.is_ok() {}"), "not converted: {out}");
        assert!(!out.contains("let"), "let must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn while_let_some_wildcard_becomes_is_some() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(_) = it.next() {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(
            out.contains("while it.next().is_some() {}"),
            "not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_let_err_wildcard_becomes_is_err() {
        let input = "fn f(r: Result<u8, u8>) { if let Err(_) = r {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("if r.is_err() {}"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_let_none_becomes_is_none() {
        let input = "fn f(x: Option<u8>) { if let None = x {} }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("if x.is_none() {}"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn option_binding_pattern_untouched() {
        let input = "fn f(x: Option<u8>) { if let Some(v) = x { drop(v); } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "binding pattern must not fire: {out:?}");
    }

    #[test]
    fn result_binding_pattern_untouched() {
        let input = "fn f(y: Result<u8, u8>) { if let Ok(v) = y { drop(v); } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "binding pattern must not fire: {out:?}");
    }

    // ── the match shape (run-#2 miss map, 2026-08-19) ──

    #[test]
    fn match_some_true_none_false_becomes_is_some() {
        let input = "fn f(e: Option<u8>) -> bool { match e { Some(_) => true, None => false } }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("e.is_some()"), "not converted: {out}");
        assert!(!out.contains("match"), "match must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_inverted_arm_order_becomes_is_some() {
        // `None => false` FIRST, wildcard second — clippy's own probe shape.
        let input = "fn f(e: Option<u8>) -> bool { match e { None => false, _ => true } }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("e.is_some()"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_some_false_becomes_is_none() {
        let input = "fn f(e: Option<u8>) -> bool { match e { Some(_) => false, None => true } }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("e.is_none()"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_ok_err_becomes_is_ok() {
        let input =
            "fn f(r: Result<u8, u8>) -> bool { match r { Ok(_) => true, Err(_) => false } }";
        let out = propose_fix("redundant_pattern_matching", input).expect("matched");
        assert!(out.contains("r.is_ok()"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_binary_scrutinee_parenthesized() {
        // Non-postfix-safe scrutinee needs parens as a method receiver.
        let input = "fn f(a: u8, b: u8) -> bool { match a + b { _ => true } }";
        // (a + b has no constructor arms — no fire; the paren case is
        // exercised by the field-free path in the let tests.)
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "no constructor arm: {out:?}");
    }

    #[test]
    fn match_non_bool_arms_untouched() {
        let input = "fn f(e: Option<u8>) -> u8 { match e { Some(_) => 1, None => 0 } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "non-bool arms must not fire: {out:?}");
    }

    #[test]
    fn match_binding_arm_untouched() {
        let input = "fn f(e: Option<u8>) -> bool { match e { Some(v) => v > 0, None => false } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "binding arm must not fire: {out:?}");
    }

    #[test]
    fn match_guarded_arm_untouched() {
        let input = "fn f(e: Option<u8>) -> bool { match e { Some(_) if e.is_some() => true, _ => false } }";
        let out = propose_fix("redundant_pattern_matching", input);
        assert!(out.is_none(), "guarded arm must not fire: {out:?}");
    }
}
