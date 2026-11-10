//! `clippy::manual_ok_err` — the two-arm Result→Option identity match →
//! `Result::ok` / `Result::err` (Batch 228 / Plan 230 — the coverage-report
//! re-arm pack XIX; the firing/silence matrix verified against real clippy
//! 0.1.98, 2026-10-13, `scripts/plan230_pack19_probe.py` VERDICT PASS;
//! style/warn-by-default, group pinned by the probe's no-flags arm).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! `Option::ok` / `Option::err` are DEFINED by std as exactly the matches
//! they replace — the std source is
//! `pub fn ok(self) -> Option<T> { match self { Ok(v) => Some(v),
//! Err(_) => None } }` (the `err` twin mutatis mutandis:
//! `Ok(_) => None, Err(e) => Some(e)`). Substituting the definition's own
//! body with the function is value-identical by construction, and the
//! input's own compilation proves the scrutinee is a `Result<T, E>` — the
//! `Ok`/`Err` patterns type-checked against it, so the emitted call
//! resolves to the std conversion (zero type-blindness; the std
//! definition-identity class of Batch 160's radix-10 `FromStr`, Batch
//! 162's `bytes().count()`, Batch 191's `mem_replace_with_default`).
//!
//! ## Guards (the bounded fix-space)
//!
//! - EXACTLY two arms, the Ok arm first (both std definitions spell that
//!   order; a reversed order is upstream-unmeasured and refuses); any arm
//!   guard (`Ok(v) if v > 0 => ..`) is SILENT upstream and refuses.
//! - ok-form: arm 1 is `Ok(<ident>)` — a single ident binding, no `ref`,
//!   no subpattern/`@` — with body exactly `Some(<same ident>)`; arm 2 is
//!   EXACTLY `Err(_)` (a lone wildcard; a NAMED wildcard `Err(_e)` is
//!   upstream-unmeasured — the probe measured only `Err(_)` — and refuses
//!   v1) with body `None`. Emits `<scrutinee>.ok()`. The `.err()` twin
//!   mirrors it: `Ok(_)` / `None` first, `Err(<ident>)` / `Some(<same
//!   ident>)` second.
//! - Non-identity arm bodies (`Ok(v) => Some(v.to_string())`) are SILENT
//!   upstream (probe-measured) — parity refusal.
//! - The scrutinee must be a path, call, method-call, field, or index
//!   expression — the postfix `.ok()` binds correctly on each; unary
//!   (`*r`), casts, references, and binary operands refuse
//!   (paren-spelling is not worth the risk v1 — the bounded-space call,
//!   the `mem_replace_with_default` `<_>::default()` precedent).
//! - The if-let form (`if let Ok(v) = r { Some(v) } else { None }`) FIRES
//!   upstream (probe-measured) but refuses here v1 — the documented
//!   narrower-than-clippy divergence (the plan-169 `len_without_is_empty`
//!   v1-refuses-traits precedent; the identity holds, the bounded-space
//!   call).
//! - No attributes on the match or either arm; a comment inside the
//!   replaced span refuses (the span re-spells to `<scrutinee>.ok()`).

use syn::visit::Visit;
use syn::{Expr, ExprMatch, File, Ident, Pat};

use super::helpers::byte_range;
use super::{SourceEdit, edit, src};
use crate::edit_emitter::drops_interior_comment;

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

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(replacement) = self.try_rewrite(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    /// `match r { Ok(v) => Some(v), Err(_) => None }` → `r.ok()`, and the
    /// `r.err()` twin — both arms must carry the exact identity shape.
    fn try_rewrite(&self, m: &ExprMatch) -> Option<String> {
        if !m.attrs.is_empty() || m.arms.len() != 2 {
            return None;
        }
        let [ok_arm, err_arm] = m.arms.as_slice() else {
            return None;
        };
        if !ok_arm.attrs.is_empty() || !err_arm.attrs.is_empty() {
            return None;
        }
        if ok_arm.guard.is_some() || err_arm.guard.is_some() {
            return None;
        }
        let ok_ident = ctor_binding(&ok_arm.pat, "Ok");
        let err_ident = ctor_binding(&err_arm.pat, "Err");
        // ok-form: `Ok(v) => Some(v), Err(_) => None`.
        let ok_form = ok_ident.is_some_and(|v| is_some_of(&ok_arm.body, v))
            && ctor_wild(&err_arm.pat, "Err")
            && is_none_expr(&err_arm.body);
        // err-form: `Ok(_) => None, Err(e) => Some(e)`.
        let err_form = ctor_wild(&ok_arm.pat, "Ok")
            && is_none_expr(&ok_arm.body)
            && err_ident.is_some_and(|e| is_some_of(&err_arm.body, e));
        let suffix = match (ok_form, err_form) {
            (true, false) => "ok",
            (false, true) => "err",
            _ => return None,
        };
        // The postfix `.ok()`/`.err()` binds correctly on each allowed
        // shape; anything else (unary, cast, reference, binary) refuses.
        match &*m.expr {
            Expr::Path(_)
            | Expr::Call(_)
            | Expr::MethodCall(_)
            | Expr::Field(_)
            | Expr::Index(_) => {}
            _ => return None,
        }
        if drops_interior_comment(self.source, byte_range(m), &[]) {
            return None;
        }
        let scrutinee = src(self.source, &m.expr)?;
        Some(format!("{scrutinee}.{suffix}()"))
    }
}

/// `Ctor(<ident>)` — a single-segment tuple-struct path wrapping exactly
/// one ident binding (no `ref`, no subpattern/`@`); `Ctor(_)` is not a
/// binding and returns `None` here.
fn ctor_binding<'a>(pat: &'a Pat, ctor: &str) -> Option<&'a Ident> {
    let Pat::TupleStruct(ts) = pat else {
        return None;
    };
    if !ts.attrs.is_empty() || ts.qself.is_some() || !ts.path.is_ident(ctor) || ts.elems.len() != 1
    {
        return None;
    }
    let Pat::Ident(id) = ts.elems.first()? else {
        return None;
    };
    if id.by_ref.is_some() || id.subpat.is_some() {
        return None;
    }
    Some(&id.ident)
}

/// `Ctor(_)` — the same tuple-struct shape with a lone wildcard field.
fn ctor_wild(pat: &Pat, ctor: &str) -> bool {
    let Pat::TupleStruct(ts) = pat else {
        return false;
    };
    ts.attrs.is_empty()
        && ts.qself.is_none()
        && ts.path.is_ident(ctor)
        && ts.elems.len() == 1
        && matches!(ts.elems.first(), Some(Pat::Wild(_)))
}

/// Exactly `Some(<ident>)` — a bare single-segment path call whose only
/// argument is a bare path naming the same ident.
fn is_some_of(body: &Expr, ident: &Ident) -> bool {
    let Expr::Call(c) = body else { return false };
    if !c.attrs.is_empty() || c.args.len() != 1 {
        return false;
    }
    let Expr::Path(func) = &*c.func else {
        return false;
    };
    if !func.attrs.is_empty() || func.qself.is_some() || !func.path.is_ident("Some") {
        return false;
    }
    let Some(Expr::Path(arg)) = c.args.first() else {
        return false;
    };
    arg.attrs.is_empty() && arg.qself.is_none() && arg.path.is_ident(ident)
}

/// Exactly the bare `None` path — no attributes, no qself, single
/// segment, no turbofish.
fn is_none_expr(body: &Expr) -> bool {
    let Expr::Path(p) = body else { return false };
    p.attrs.is_empty() && p.qself.is_none() && p.path.is_ident("None")
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "manual_ok_err";

    #[test]
    fn seed_is_clean() {
        assert!(
            propose_fix(
                RULE,
                "fn t(r: Result<u32, String>, sink: &mut Vec<u32>) { sink.push(r.unwrap_or(0)); }",
            )
            .is_none()
        );
    }

    #[test]
    fn ok_form_fires() {
        let out = propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<u32> { match r { Ok(v) => Some(v), Err(_) => None } }",
        )
        .expect("matched");
        assert!(out.contains("r.ok()"), "not rewritten: {out}");
        assert!(!out.contains("match"), "match not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn err_form_fires() {
        let out = propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<String> { match r { Ok(_) => None, Err(e) => Some(e) } }",
        )
        .expect("matched");
        assert!(out.contains("r.err()"), "not rewritten: {out}");
        assert!(!out.contains("match"), "match not collapsed: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn call_scrutinee_fires() {
        // A call scrutinee needs no parens — the postfix `.ok()` binds to
        // the call directly.
        let out = propose_fix(
            RULE,
            "fn check(x: u32) -> Result<u32, String> { Ok(x) }\nfn t(x: u32) -> Option<u32> { match check(x) { Ok(v) => Some(v), Err(_) => None } }",
        )
        .expect("matched");
        assert!(out.contains("check(x).ok()"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nonidentity_arms_refuse() {
        // SILENT upstream (probe-measured) — parity refusal.
        assert!(propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<String> { match r { Ok(v) => Some(v.to_string()), Err(_) => None } }",
        )
        .is_none());
    }

    #[test]
    fn three_arms_refuse() {
        assert!(propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<u32> { match r { Ok(v) => Some(v), Err(_) => None, _ => None } }",
        )
        .is_none());
    }

    #[test]
    fn arm_guard_refuses() {
        // SILENT upstream — parity refusal.
        assert!(propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<u32> { match r { Ok(v) if v > 0 => Some(v), Err(_) => None } }",
        )
        .is_none());
    }

    #[test]
    fn named_wildcard_err_refuses() {
        // The probe measured only `Err(_)` — a named wildcard is
        // upstream-unmeasured and refuses v1.
        assert!(propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<u32> { match r { Ok(v) => Some(v), Err(_e) => None } }",
        )
        .is_none());
    }

    #[test]
    fn if_let_form_refuses() {
        // FIRES upstream (probe-measured) but refused v1 — the documented
        // narrower-than-clippy divergence (it never reaches the match
        // visitor).
        assert!(propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<u32> { if let Ok(v) = r { Some(v) } else { None } }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(propose_fix(
            RULE,
            "fn t(r: Result<u32, String>) -> Option<u32> { match r { /* now */ Ok(v) => Some(v), Err(_) => None } }",
        )
        .is_none());
    }
}
