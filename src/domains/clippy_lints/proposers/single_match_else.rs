//! `clippy::single_match_else` — `match x { Some(v) => f(v), _ => { fallback() } }`
//! → `if let Some(v) = x { f(v) } else { fallback() }`.
//!
//! Pattern: a two-arm match, no attributes, no guards, where the SECOND
//! arm is a wildcard arm whose body is a NON-UNIT STATEMENT BLOCK (the
//! unit shape is `single_match`, a sibling lint).
//!
//! Wildcard-arm shapes (Issue 027 + Issue 029 T3 probe matrix vs real
//! clippy 1.97 pedantic):
//! - **`_`** — any scrutinee, any non-wild first arm (the 027-validated
//!   shape);
//! - **std Option/Result no-binding arms** — when the FIRST arm pins a std
//!   Option/Result scrutinee (`Some(v)`, `Ok(_)`, `Err(e)`, `None`,
//!   `Option::…`/`Result::…`-qualified): the second arm fires iff it has
//!   NO bindings — `None`, `Option::None`, `Err(_)`, `Err(..)`, `Some(_)`,
//!   `Ok(_)`. Silent (probe-verified): binding payloads (`Err(e)`,
//!   `Some(v)` in arm2), user-enum arms (`R2::Ok(_)`, `E3::None`,
//!   glob-imported `None` — clippy is type-informed), and expression arm2
//!   bodies. The emitted `if let P1 = scrut { .. } else { .. }` needs no
//!   trait bounds, so it is sound for every shape that compiles (a
//!   compiling 2-arm exhaustive match proves P2 ≡ ¬P1).
//!
//! Two fix shapes (Issue 027/029, empirical vs real clippy):
//! - **non-literal first pattern** → the `if let` form (the 027-validated
//!   shape);
//! - **INTEGER-literal first pattern** (`0 => ..`, non-negative) → the
//!   **equality** form `if x == 0 { .. } else { .. }` (the 027 probe P1
//!   finding: literal arms fire under a different fix shape; the 029 fix
//!   closes the miss). Bool/char/str literal arms stay skipped (unprobed).
//!
//! Interior-comment guard (Issue 029, the 027/028 comment-deletion class in
//! a third proposer): the whole-match splice keeps only the scrutinee,
//! pattern, and arm-body fragments verbatim — a comment anywhere else in
//! the match (e.g. between arms, `0 => true, // why`) would be deleted.
//! Such matches are refused.
//!
//! Span-preserving (Issue 016): the whole match's byte range is replaced —
//! pattern, scrutinee, and both bodies keep their ORIGINAL source text;
//! non-block bodies are braced.

use super::helpers::{byte_range, drops_interior_comment, is_postfix_safe, is_unit_expr};
use super::{edit, src, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprMatch, File, Lit, Pat};

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

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        if let Some(replacement) = self.try_single_match_else(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_single_match_else(&self, m: &ExprMatch) -> Option<String> {
        if m.arms.len() != 2 || !m.attrs.is_empty() {
            return None;
        }
        if m.arms.iter().any(|a| !a.attrs.is_empty() || a.guard.is_some()) {
            return None; // guards/arm-attrs would be lost by the splice
        }
        if !is_wildcard_second_arm(&m.arms[0].pat, &m.arms[1].pat) || is_unit_expr(&m.arms[1].body) {
            return None; // unit wildcard is `single_match`, not this lint
        }
        // Issue 027: real clippy only fires when the wildcard arm is a
        // statement BLOCK (expression wildcards are silent even under
        // pedantic) — probe P2/P4 vs P3.
        if !matches!(&*m.arms[1].body, Expr::Block(_)) {
            return None;
        }
        if matches!(m.arms[0].pat, Pat::Wild(_)) {
            return None;
        }
        // Issue 029: only the scrutinee, pattern, and arm-body fragments
        // survive the splice — a comment in any dropped gap would be
        // deleted outright. Refuse those matches (real clippy stays silent
        // on comment-displacing rewrites).
        let kept = [
            byte_range(&*m.expr),
            byte_range(&m.arms[0].pat),
            byte_range(&*m.arms[0].body),
            byte_range(&*m.arms[1].body),
        ];
        if drops_interior_comment(self.source, m.span().byte_range(), &kept) {
            return None;
        }
        let scrutinee = src(self.source, &*m.expr)?;
        let then = braced_body(self.source, &m.arms[0].body)?;
        let else_ = braced_body(self.source, &m.arms[1].body)?;
        match &m.arms[0].pat {
            // Issue 029: literal INTEGER first arms fire under the equality
            // form (`if x == 0`). This syn's `PatLit` holds a `Lit` whose
            // token text preserves any leading `-` — non-negative only
            // (negatives unprobed); bool/char/str literals stay skipped.
            Pat::Lit(l) if is_nonnegative_int_lit(&l.lit) => {
                let lit = src(self.source, &l.lit)?;
                let cond = if is_postfix_safe(&m.expr) {
                    format!("{scrutinee} == {lit}")
                } else {
                    format!("({scrutinee}) == {lit}")
                };
                Some(format!("if {cond} {then} else {else_}"))
            }
            Pat::Lit(_) => None, // bool/char/str literal — unprobed, skip
            _ => {
                let pat = src(self.source, &m.arms[0].pat)?;
                Some(format!("if let {pat} = {scrutinee} {then} else {else_}"))
            }
        }
    }
}

/// A match-arm body as an if-let body: block bodies keep their ORIGINAL
/// braces; expression bodies get fresh ones.
fn braced_body(source: &str, body: &Expr) -> Option<String> {
    let text = src(source, body)?;
    Some(match body {
        Expr::Block(_) => text,
        _ => format!("{{ {text} }}"),
    })
}

/// A `Lit::Int` whose token text carries no leading `-` (the token can hold
/// a sign; negative-literal arms are unprobed vs real clippy and skipped).
fn is_nonnegative_int_lit(l: &Lit) -> bool {
    match l {
        Lit::Int(i) => !i.token().to_string().starts_with('-'),
        _ => false,
    }
}

/// Does the second arm act as the wildcard arm of a `single_match_else`?
///
/// `_` is unconditionally wildcard (any scrutinee). Additionally, on a std
/// Option/Result scrutinee clippy fires on a NO-BINDING constructor arm
/// (`None`, `Err(_)`, `Err(..)`, `Some(_)`, `Ok(_)`) — approximated here by
/// requiring the FIRST arm to be a std-shaped Some/Ok/Err/None pattern that
/// pins the scrutinee type. Probe matrix (Issue 029 T3, clippy 1.97
/// pedantic): fires on Some(v)+None, Ok(v)+Err(_), None+Some(_),
/// `Err(_)`+`Ok(_)`, Ok(v)+Err(..), Some(_)+None, `Option::None`; silent on
/// binding arm2s, user-enum arms (`R2::Ok(_)`, `E3::None`), and expression
/// arm2 bodies.
fn is_wildcard_second_arm(first: &Pat, second: &Pat) -> bool {
    if matches!(second, Pat::Wild(_)) {
        return true;
    }
    is_std_optres_variant(first) && is_std_no_binding_ctor(second)
}

/// A pattern that pins a std Option/Result scrutinee: `Some(..)`, `Ok(..)`,
/// `Err(..)`, `None` — bare or `Option::`/`Result::`-qualified. Or-patterns
/// and anything else (user enums) return false, keeping the lint silent
/// where type-informed clippy is (parse-only approximation).
fn is_std_optres_variant(p: &Pat) -> bool {
    match p {
        Pat::Ident(pi) => pi.ident == "None" && pi.subpat.is_none(),
        Pat::Path(pp) => is_none_path(&pp.path),
        Pat::TupleStruct(ts) => !ts.elems.is_empty() && is_ctor_path(&ts.path),
        _ => false,
    }
}

/// A std Option/Result constructor arm with NO bindings — the
/// probe-verified wildcard-equivalent second arms. `Err(_e)`/`Some(v)`
/// (bindings) and user-enum constructors stay out.
fn is_std_no_binding_ctor(p: &Pat) -> bool {
    match p {
        Pat::Ident(pi) => pi.ident == "None" && pi.subpat.is_none(),
        Pat::Path(pp) => is_none_path(&pp.path),
        Pat::TupleStruct(ts) => {
            !ts.elems.is_empty()
                && is_ctor_path(&ts.path)
                && ts.elems.iter().all(|e| matches!(e, Pat::Wild(_) | Pat::Rest(_)))
        }
        _ => false,
    }
}

fn is_none_path(path: &syn::Path) -> bool {
    path.is_ident("None") || is_std_qualified(path, "None", "Option")
}

fn is_ctor_path(path: &syn::Path) -> bool {
    ["Some", "Ok", "Err"].iter().any(|n| path.is_ident(n))
        || is_std_qualified(path, "Some", "Option")
        || is_std_qualified(path, "Ok", "Result")
        || is_std_qualified(path, "Err", "Result")
}

/// Exactly `Ty::Name` with no generic arguments (`Option::None`,
/// `Result::Ok`).
fn is_std_qualified(path: &syn::Path, last: &str, ty: &str) -> bool {
    path.segments.len() == 2
        && path.segments[0].ident == ty
        && path.segments[1].ident == last
        && path.segments.iter().all(|s| s.arguments.is_none())
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_to_if_let_else() {
        // Issue 027: the wildcard arm must be a statement BLOCK (real clippy
        // is silent on expression wildcards even under pedantic).
        let input = "fn g(x: Option<u8>) { match x { Some(v) => drop(v), _ => { let f = fallback(); f } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let Some(v) = x { drop(v) } else { let f = fallback(); f }"),
            "not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn block_bodies_keep_original_braces() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) => { drop(v); }, _ => { fallback(); } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let Some(v) = x { drop(v); } else { fallback(); }"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn expression_wildcard_arm_untouched() {
        // Issue 027 probe P2/P4: real clippy is SILENT on expression
        // wildcards (`_ => 0`) — the canonical false-positive shape from
        // the katgpt-rs cluster_build.rs run (`match bits & 1 { 0 => -1.0, _ => 1.0 }`).
        let input = "fn g(x: Option<u8>) -> u8 { match x { Some(v) => v + 1, _ => 0 } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "expression wildcard must not fire: {out:?}");
    }

    #[test]
    fn literal_first_arm_untouched() {
        // Negative int literals parse as Unary inside Pat::Lit — excluded
        // from the equality form (unprobed vs real clippy).
        let input = "fn g(x: i64) -> u8 { match x { -1 => 1, _ => { let v = 2; v } } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "negative literal arm must stay skipped: {out:?}");
    }

    #[test]
    fn int_literal_with_binary_scrutinee_gets_parens() {
        // Issue 027 probe P1 fixture — it FIRES under the equality form
        // (Issue 029); the Binary scrutinee is parenthesized (Rust's `&`
        // binds tighter than `==`, but explicit parens are unambiguous).
        let input =
            "fn g(x: u64) -> f32 { match x & 1 { 0 => -1.0, _ => { let f = 1.0; f } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if (x & 1) == 0 { -1.0 } else { let f = 1.0; f }"),
            "parenthesized equality not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn int_literal_first_arm_becomes_equality() {
        // Issue 029: the decomp_reviewer.rs:62 miss shape — int-literal first
        // arm + block wildcard fires real clippy under the EQUALITY form.
        let input = "fn g(total: u32) -> bool { match total { 0 => true, _ => { let n = 1.0; n >= 0.5 } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if total == 0 { true } else { let n = 1.0; n >= 0.5 }"),
            "equality form not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn complex_scrutinee_gets_parens_in_equality() {
        // A scrutinee that binds looser than `==` must be parenthesized.
        let input =
            "fn g(x: u8, y: u8) -> u8 { match x | y { 0 => 1, _ => { let v = 2; v } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(out.contains("if (x | y) == 0"), "scrutinee not parenthesized: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bool_literal_first_arm_still_skipped() {
        // Unprobed vs real clippy — bool literal arms stay out of scope.
        let input =
            "fn g(x: bool) -> u8 { match x { true => 1, _ => { let v = 2; v } } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "bool literal arm must stay skipped: {out:?}");
    }

    #[test]
    fn interior_comment_blocks_both_forms() {
        // Issue 029 (the 027/028 comment-deletion class in this proposer):
        // a comment between arms sits in a DROPPED gap — the whole-match
        // splice would delete it. The decomp_reviewer.rs:62 site carries
        // exactly this comment and must stay silent.
        let equality =
            "fn g(total: u32) -> bool { match total { 0 => true, // no data yet\n            _ => { let n = 1.0; n >= 0.5 } } }";
        assert!(propose_fix("single_match_else", equality).is_none());
        let if_let = "fn g(x: Option<u8>) { match x { // leading comment\n            Some(v) => drop(v), _ => { fallback(); } } }";
        assert!(propose_fix("single_match_else", if_let).is_none());
    }

    #[test]
    fn comment_inside_arm_body_survives() {
        // A comment INSIDE a kept fragment (arm body) survives the splice —
        // must keep firing.
        let input =
            "fn g(x: Option<u8>) { match x { Some(v) => { drop(v); // why\n }, _ => { fallback(); } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(out.contains("// why"), "interior arm-body comment lost: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unit_wildcard_is_single_match() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) => drop(v), _ => () } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "unit wildcard arm is single_match, not this lint: {out:?}");
    }

    #[test]
    fn three_arms_untouched() {
        let input = "fn g(x: u8) { match x { 1 => a(), 2 => b(), _ => fallback() } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "3-arm match must not fire: {out:?}");
    }

    #[test]
    fn guarded_arm_untouched() {
        let input = "fn g(x: Option<u8>) { match x { Some(v) if v > 0 => drop(v), _ => { fallback() } } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "guarded arm must not fire: {out:?}");
    }

    #[test]
    fn none_second_arm_fires() {
        // Issue 029 T3 probe P1/P2: Some-first + None-second fires real
        // clippy (MachineApplicable `if let Some(v) = x .. else ..`).
        let input =
            "fn g(x: Option<u8>) -> u8 { match x { Some(v) => v + 1, None => { let b = 2; b } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let Some(v) = x { v + 1 } else { let b = 2; b }"),
            "None-second not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn err_wildcard_second_fires() {
        // Issue 029 T3 probe P4: Ok-first + Err(_) second fires.
        let input =
            "fn g(r: Result<u8, u8>) -> u8 { match r { Ok(v) => v, Err(_) => { let b = 2; b } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let Ok(v) = r { v } else { let b = 2; b }"),
            "Err(_) not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn err_rest_payload_second_fires() {
        // Issue 029 T3 probe P15: `Err(..)` rest payload fires too.
        let input =
            "fn g(r: Result<u8, u8>) -> u8 { match r { Ok(v) => v, Err(..) => { let b = 2; b } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let Ok(v) = r { v } else { let b = 2; b }"),
            "Err(..) not converted: {out}"
        );
    }

    #[test]
    fn success_wildcard_second_fires_reversed() {
        // Issue 029 T3 probe P13/P14: the DEFAULT first + success-wildcard
        // second also fires (clippy suggests `if x == None` — we emit the
        // trait-free `if let None = x` form, sound for every payload type).
        let option =
            "fn g(x: Option<u8>) -> u8 { match x { None => { let b = 2; b }, Some(_) => { let a = 1; a } } }";
        let out = propose_fix("single_match_else", option).expect("matched");
        assert!(
            out.contains("if let None = x { let b = 2; b } else { let a = 1; a }"),
            "reversed Some(_) not converted: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
        let result =
            "fn g(r: Result<u8, u8>) -> u8 { match r { Err(_) => { let b = 2; b }, Ok(_) => { let a = 1; a } } }";
        let out = propose_fix("single_match_else", result).expect("matched");
        assert!(
            out.contains("if let Err(_) = r { let b = 2; b } else { let a = 1; a }"),
            "reversed Ok(_) not converted: {out}"
        );
    }

    #[test]
    fn qualified_option_none_second_fires() {
        // Issue 029 T3 probe P9: qualified `Option::None` fires.
        let input =
            "fn g(x: Option<u8>) -> u8 { match x { Some(v) => v, Option::None => { let b = 2; b } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let Some(v) = x { v } else { let b = 2; b }"),
            "Option::None not converted: {out}"
        );
    }

    #[test]
    fn binding_second_arm_stays_silent() {
        // Issue 029 T3 probes P5/P6/P18/P19: a BINDING in the second arm
        // loses the binding under `else` — real clippy stays silent.
        let err_e =
            "fn g(r: Result<u8, u8>) -> u8 { match r { Ok(v) => v, Err(e) => { let b = e as u8; b } } }";
        assert!(propose_fix("single_match_else", err_e).is_none());
        let some_v =
            "fn g(x: Option<u8>) -> u8 { match x { None => { let b = 2; b }, Some(v) => { let a = v + 1; a } } }";
        assert!(propose_fix("single_match_else", some_v).is_none());
    }

    #[test]
    fn expression_none_second_body_stays_silent() {
        // Issue 029 T3 probes P3/P12: expression arm2 bodies are silent
        // (consistent with the 027 expression-wildcard finding).
        let input =
            "fn g(x: Option<u8>) -> u8 { match x { Some(v) => v + 1, None => 2 } }";
        let out = propose_fix("single_match_else", input);
        assert!(out.is_none(), "expression None arm must not fire: {out:?}");
    }

    #[test]
    fn user_enum_arms_stay_silent() {
        // Issue 029 T3 probes P7/P16/P17: user-enum arms never fire —
        // clippy is type-informed (only std Option/Result scrutineers fire);
        // the parse-only approximation gates on std-shaped FIRST arms.
        let qualified = "enum R2 { Ok(u8), Err(u8) }\nfn g(r: R2) -> u8 { match r { R2::Ok(_) => { let a = 1; a }, R2::Err(_) => { let b = 2; b } } }";
        assert!(propose_fix("single_match_else", qualified).is_none());
        let none_named = "enum E3 { Foo, None }\nfn g(e: E3) -> u8 { match e { E3::Foo => { let a = 1; a }, E3::None => { let b = 2; b } } }";
        assert!(propose_fix("single_match_else", none_named).is_none());
    }

    #[test]
    fn none_first_with_wildcard_second_still_fires() {
        // Pre-existing `_`-second behavior extends to a None FIRST arm —
        // the if-let form keeps the unit-path pattern verbatim.
        let input =
            "fn g(x: Option<u8>) -> u8 { match x { None => 2, _ => { let a = 1; a } } }";
        let out = propose_fix("single_match_else", input).expect("matched");
        assert!(
            out.contains("if let None = x { 2 } else { let a = 1; a }"),
            "None-first wildcard not converted: {out}"
        );
    }
}
