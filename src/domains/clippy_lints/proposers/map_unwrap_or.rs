//! `clippy::map_unwrap_or` — `x.map(f).unwrap_or(d)` → `x.map_or(d, f)`, or
//! `is_some_and(<F>)` when the default is `false` and the closure provably
//! returns bool (real clippy 1.97's own suggestion for that shape — its
//! `map_or` output would be re-flagged by `unnecessary_map_or`, a style
//! lint, so the fused form is NOT the terminal state there).
//!
//! The `unwrap_or_else` arm (riir-train slice 8, 2026-08-18): real clippy
//! fires the SAME lint on `x.map(f).unwrap_or_else(g)` and suggests
//! `x.map_or_else(g, f)` — default closure FIRST, mapper SECOND. Both
//! evaluate both closures lazily, so the fused form preserves semantics
//! for ANY shapes. The bool-literal special cases do not apply (the
//! default is a closure, never a bool literal).
//!
//! `Option::map_or(default, f)` is the fused form of `map(f).unwrap_or(d)`:
//! both evaluate `d` eagerly and `f` lazily, so the rewrite preserves
//! semantics for ANY default/mapper shapes (no purity requirement).
//! Span-preserving (T6.4): the whole `unwrap_or`/`unwrap_or_else` call's
//! byte range is replaced by `<recv-of-map>.map_or(<D>, <F>)` (or
//! `map_or_else(<G>, <F>)`), each fragment being the ORIGINAL source text
//! of the matched node.
//!
//! Bool-default guard (Issue 027 probe class, 2026-08-18): a provably-bool
//! closure with a `true` default is SKIPPED — real clippy suggests `map_or`
//! there, but that output triggers `unnecessary_map_or` → `is_none_or`, so
//! no parse-only fix we can emit matches both of clippy's steps. Closures
//! whose bool-ness cannot be proven syntactically (e.g. a bare path) keep
//! the `map_or` form — a parse-only limitation, not a wrong fix.

use super::helpers::is_postfix_safe;
use super::uninlined_format_args::post_template_expr_args;
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ExprMacro, File, Macro, StmtMacro};
use proc_macro2::TokenStream;

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
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        if let Some(replacement) = try_map_or(self.source, m) {
            self.edits.push(edit(m, replacement));
        }
    }

    /// Macro arguments are OPAQUE token streams to syn — the default
    /// visitor never descends into them, so `.map(f).unwrap_or_else(g)`
    /// inside a `writeln!`/`println!`/`format!` arg (the `bench_241` shape)
    /// is invisible to the method-call visit above. Real clippy matches
    /// there (post-expansion HIR); we parse the expression args of the
    /// known format-family macros + run the same matcher over them. The
    /// cloned tokens keep their file-relative spans, so edits collected
    /// on the parsed args slice the ORIGINAL source correctly.
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro_args(&m.mac);
    }

    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro_args(&m.mac);
    }
}

impl Collector<'_> {
    fn scan_macro_args(&mut self, mac: &Macro) {
        let Some(args) = post_template_expr_args(mac) else {
            return;
        };
        for arg in args {
            let stream: TokenStream = arg.toks.iter().cloned().collect();
            let Ok(expr) = syn::parse2::<Expr>(stream) else {
                continue; // named args / non-expression tokens — skip
            };
            scan_expr_for_matches(self.source, &mut self.edits, &expr);
        }
    }
}

/// Walk one macro-arg expression for `map(..).unwrap_or(_or_else)(..)`
/// matches. The lifetimes tie the AST borrow to the source/edits borrows
/// (all share the caller's frame); spans are file-relative by token
/// round-trip, so `src`/`edit` operate on the WHOLE-file source.
fn scan_expr_for_matches(source: &str, edits: &mut Vec<SourceEdit>, expr: &Expr) {
    struct Sub<'a> {
        source: &'a str,
        edits: &'a mut Vec<SourceEdit>,
    }
    impl<'ast> Visit<'ast> for Sub<'ast> {
        fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
            syn::visit::visit_expr_method_call(self, m);
            if let Some(replacement) = try_map_or(self.source, m) {
                self.edits.push(edit(m, replacement));
            }
        }
    }
    let mut sub = Sub { source, edits };
    sub.visit_expr(expr);
}

/// `x.map(f).unwrap_or(d)` → `x.map_or(d, f)` / `is_some_and(f)`;
/// `x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)`.
fn try_map_or(source: &str, m: &ExprMethodCall) -> Option<String> {
        let else_form = if m.method == "unwrap_or" {
            false
        } else if m.method == "unwrap_or_else" {
            true
        } else {
            return None;
        };
        if !m.attrs.is_empty() || m.turbofish.is_some() {
            return None;
        }
        if m.args.len() != 1 {
            return None;
        }
        let Expr::MethodCall(map) = &*m.receiver else {
            return None; // receiver isn't a method call
        };
        if !map.attrs.is_empty() || map.turbofish.is_some() || map.method != "map" {
            return None;
        }
        if map.args.len() != 1 {
            return None;
        }
        let recv = src(source, &*map.receiver)?;
        let default = src(source, &m.args[0])?;
        let mapper = src(source, &map.args[0])?;
        let recv = if is_postfix_safe(&map.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        if else_form {
            // `x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)` — real
            // clippy's own suggestion for the else arm; default closure
            // FIRST, mapper SECOND. No bool-literal special cases (the
            // default is always a closure here).
            return Some(format!("{recv}.map_or_else({default}, {mapper})"));
        }
        let bool_default = bool_literal(&m.args[0]);
        let mapper_is_bool = closure_body_is_bool(&map.args[0]);
        if bool_default == Some(true) && mapper_is_bool {
            // Real clippy's own suggestion here (`map_or(true, f)`) is
            // re-flagged by `unnecessary_map_or` → `is_none_or`; no single
            // parse-only step matches clippy, so skip (probe-verified).
            return None;
        }
        if bool_default == Some(false) && mapper_is_bool {
            // Real clippy 1.97 suggests `is_some_and(<f>)` for this shape
            // (the `map_or` output would be re-flagged by
            // `unnecessary_map_or`, a default-group lint).
            return Some(format!("{recv}.is_some_and({mapper})"));
        }
        Some(format!("{recv}.map_or({default}, {mapper})"))
}

/// `Some(false)`/`Some(true)` for a `false`/`true` literal, else `None`.
fn bool_literal(e: &Expr) -> Option<bool> {
    match e {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Bool(b) => Some(b.value),
            _ => None,
        },
        Expr::Paren(p) => bool_literal(&p.expr),
        _ => None,
    }
}

/// Syntactically PROVE the expression is bool-typed. Conservative: only
/// shapes that are bool whenever the code compiles (comparisons, `!`,
/// `&&`/`||`, bool literals) plus a small allowlist of bool-returning method
/// names. Unknown shapes return false (the proposer then fires `map_or`,
/// which is real clippy's suggestion for everything it cannot see through).
fn expr_is_bool(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(&l.lit, syn::Lit::Bool(_)),
        Expr::Paren(p) => expr_is_bool(&p.expr),
        Expr::Unary(u) => matches!(u.op, syn::UnOp::Not(_)),
        Expr::Binary(b) => matches!(
            b.op,
            syn::BinOp::And(_)
                | syn::BinOp::Or(_)
                | syn::BinOp::Eq(_)
                | syn::BinOp::Ne(_)
                | syn::BinOp::Lt(_)
                | syn::BinOp::Le(_)
                | syn::BinOp::Gt(_)
                | syn::BinOp::Ge(_)
        ),
        Expr::MethodCall(m) => is_bool_method(&m.method),
        _ => false,
    }
}

fn is_bool_method(name: &syn::Ident) -> bool {
    let n = name.to_string();
    n.starts_with("is_")
        || matches!(
            n.as_str(),
            "contains" | "starts_with" | "ends_with" | "any" | "all" | "eq" | "ne" | "lt"
                | "le" | "gt" | "ge"
        )
}

/// The mapper arg must be a closure whose body provably returns bool.
fn closure_body_is_bool(arg: &Expr) -> bool {
    let Expr::Closure(c) = arg else {
        return false;
    };
    expr_is_bool(&c.body)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fuses_map_unwrap_or() {
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_preserved() {
        let input = "fn f(v: Vec<u8>) -> u8 { v.first().map(|x| *x + 1).unwrap_or(9) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("v.first().map_or(9, |x| *x + 1)"),
            "chain receiver text not preserved: {out}"
        );
    }

    #[test]
    fn non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn other_method_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.filter(|v| *v > 0).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "filter receiver must not fire: {out:?}");
    }

    // Bool-default guards (Issue 027 probe class, 2026-08-18; real clippy
    // 1.97 pedantic verified):
    // - `.map(bool-f).unwrap_or(false)` → clippy suggests `is_some_and(f)`;
    //   the old `map_or(false, f)` output was re-flagged by the DEFAULT-group
    //   `unnecessary_map_or` (a new warning where the original had none).
    // - `.map(bool-f).unwrap_or(true)` → clippy suggests `map_or`, whose
    //   output is itself flagged → no clean single step; skip.
    #[test]
    fn bool_closure_false_default_becomes_is_some_and() {
        let input =
            "fn f(x: Option<(bool, bool)>) -> bool { x.map(|s| !s.0 && !s.1).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|s| !s.0 && !s.1)"),
            "is_some_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bool_closure_true_default_skipped() {
        let input =
            "fn f(x: Option<(bool, bool)>) -> bool { x.map(|s| s.0 || s.1).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "bool+true must not fire (no clean fix): {out:?}");
    }

    #[test]
    fn comparison_closure_false_default_becomes_is_some_and() {
        let input = "fn f(x: Option<u8>) -> bool { x.map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|v| v > 3)"),
            "is_some_and not produced: {out}"
        );
    }

    #[test]
    fn bool_method_closure_false_default_becomes_is_some_and() {
        let input = "fn f(x: Option<String>) -> bool { x.map(|s| s.contains('a')).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|s| s.contains('a'))"),
            "is_some_and not produced: {out}"
        );
    }

    #[test]
    fn unprovable_closure_false_default_keeps_map_or() {
        // A bare-path body's bool-ness is invisible parse-only → the
        // conservative `map_or` form (real clippy's suggestion for what it
        // can see through type-independently).
        let input = "fn f(x: Option<bool>, b: bool) -> bool { x.map(|v| v).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(false, |v| v)"),
            "map_or not produced: {out}"
        );
    }

    #[test]
    fn non_bool_default_with_bool_shaped_body_still_fuses() {
        // `0` is not a bool literal → the fused map_or form regardless of
        // the closure body's shape (type-mismatched bool body is impossible).
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
    }

    // `unwrap_or_else` arm (riir-train slice 8): same lint, suggestion
    // `map_or_else(<default-closure>, <mapper>)` — arg ORDER is the
    // load-bearing fact (default first).
    #[test]
    fn fuses_map_unwrap_or_else_default_first() {
        let input =
            "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or_else(|| 0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or_else(|| 0, |v| v + 1)"),
            "map_or_else not produced with default FIRST: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_else_chained_receiver_preserved() {
        // The bench_241 shape: a field-chain receiver under the map.
        let input = "fn f(s: S) -> String { s.opt.step.map(|v| v.to_string()).unwrap_or_else(|| \"none\".into()) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.opt.step.map_or_else(|| \"none\".into(), |v| v.to_string())"),
            "chained receiver text not preserved: {out}"
        );
    }

    #[test]
    fn or_else_non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| 0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn or_else_non_closure_default_still_fuses() {
        // A function-path default (`unwrap_or_else(Default::default)` shape)
        // is still the fused form — map_or_else takes any callable.
        let input =
            "fn f(x: Option<Vec<u8>>) -> Vec<u8> { x.map(|v| v.clone()).unwrap_or_else(Vec::new) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or_else(Vec::new, |v| v.clone())"),
            "map_or_else not produced for path default: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    // Macro-argument descent (riir-train slice 8, bench_241 shape): the
    // `.map(f).unwrap_or_else(g)` sites live inside `writeln!` args —
    // opaque tokens to syn's visitor, visible only via the format-family
    // macro scan.
    #[test]
    fn or_else_inside_writeln_args_fires() {
        let input = "fn f(o: impl std::fmt::Write, s: S) { \
            writeln!(o, \"| {} |\", s.step.map(|v| v.to_string()).unwrap_or_else(|| \"never\".into())).unwrap(); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.step.map_or_else(|| \"never\".into(), |v| v.to_string())"),
            "macro-arg match not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_else_multiline_writeln_chain_fires() {
        // The exact bench_241 shape: a multi-line field-chain receiver
        // inside a multi-line writeln! statement.
        let input = "fn f(w: W, s: S) { \
            writeln!( \
                w, \
                \"| Convergence | {} |\", \
                s.with.convergence_step \
                    .map(|st| format!(\"step {st}\")) \
                    .unwrap_or_else(|| \"never\".into()) \
            ).unwrap(); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("map_or_else(|| \"never\".into(), |st| format!(\"step {st}\"))"),
            "multiline macro-arg match not produced: {out}"
        );
        // The receiver text survives verbatim (newlines included).
        assert!(
            out.contains("s.with.convergence_step"),
            "receiver text not preserved: {out}"
        );
    }

    #[test]
    fn plain_map_unwrap_or_inside_println_fires() {
        let input = "fn f(s: S) { println!(\"{}\", s.opt.map(|v| v + 1).unwrap_or(0)); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.opt.map_or(0, |v| v + 1)"),
            "macro-arg map_or not produced: {out}"
        );
    }

    #[test]
    fn no_false_fire_on_clean_macro_args() {
        // A clean arg (no map/unwrap chain) + a named arg that doesn't
        // parse as an expression — neither fires, and the named arg does
        // not abort scanning of later args.
        let input = "fn f(s: S) { println!(\"{a} {b}\", a = 1, b = s.v.map(|x| x).unwrap_or(2)); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.v.map_or(2, |x| x)"),
            "arg after a named arg not scanned: {out}"
        );
    }
}
