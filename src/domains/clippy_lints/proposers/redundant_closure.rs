//! `clippy::redundant_closure` — `|x| f(x)` → `f` when the closure body is a
//! single call passing the closure's only parameter as the only argument.
//!
//! Span-preserving (T6.4): the closure's byte range is replaced by the
//! ORIGINAL source text of the callee path.
//!
//! E0631 guard (the 2026-08-29 sweep class): a `|w| upload(w)` where
//! `upload`'s parameter type was pinned by an ANNOTATION is NOT redundant —
//! the annotation is what makes the item type deref-coerce at the call
//! (`Option<Vec<f32>>.as_ref().map(|w| upload(w))` with
//! `let upload = |data: &[f32]| ..` breaks as `.map(upload)`: expected
//! `&[f32]`, found `&Vec<f32>`). An UNANNOTATED closure binding is safe: its
//! param type is inferred FROM this call site, so the bare path re-derives
//! the same type. The proposer therefore skips a bare-ident callee that
//! resolves (same file) to a closure binding with any annotated param, or
//! to a `fn` item with any reference-typed parameter (a reference param CAN
//! be a deref-coercion target; by-value params never coerce). Unknown
//! callees (qualified paths, external fns) still fire — a generic or exact
//! match is implied by the original call compiling. VERIFY-tier backstop
//! unchanged: the compile gate reverts anything this guard's heuristics
//! miss.

use super::{SourceEdit, edit, src};
use std::collections::HashSet;
use syn::visit::Visit;
use syn::{Expr, ExprClosure, File};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let (annotated_closures, ref_param_fns, fnvals, fnval_multi_use) = collect_callee_kinds(file);
    let mut v = Collector {
        source,
        edits: Vec::new(),
        annotated_closures,
        ref_param_fns,
        fnvals,
        fnval_multi_use,
        loop_depth: 0,
    };
    v.visit_file(file);
    v.edits
}

/// Same-file callee knowledge: closure `let` bindings with any annotated
/// param, and `fn` items with any reference-typed parameter. Name-based and
/// file-wide — shadowing across scopes is accepted as a conservative skip.
///
/// `fnval_multi_use` (the E0382 guard, riir-refine issue 240 / dapps
/// 2dde9e9): the names of every non-Copy fn-VALUE binding in the file —
/// closure `let`s (annotated or not) and `impl Fn*`/`dyn Fn*`-typed fn
/// parameters — that occur at 2+ single-ident path positions. Moving such
/// a callee into the rewrite's by-value slot makes the second occurrence
/// a use of a moved value; once (the rewrite's own site) is safe.
fn collect_callee_kinds(
    file: &File,
) -> (
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
) {
    let mut annotated = HashSet::new();
    let mut ref_fns = HashSet::new();
    let mut fnvals = HashSet::new();
    let mut v = CalleeScanner {
        annotated: &mut annotated,
        ref_fns: &mut ref_fns,
        fnvals: &mut fnvals,
    };
    v.visit_file(file);
    // Use census: only names appearing at 2+ single-ident path positions
    // are hazardous. One occurrence = the rewrite consumes the only use.
    let mut fnval_multi_use = HashSet::new();
    for name in &fnvals {
        if count_ident_uses(file, name) >= 2 {
            fnval_multi_use.insert(name.clone());
        }
    }
    (annotated, ref_fns, fnvals, fnval_multi_use)
}

/// Whole-file count of single-ident path expressions spelling `name`.
/// Path expressions only — strings, comments, types (`f32`) and method
/// names never count; a loop's repeated ITERATION of one occurrence is
/// handled separately by the collector's loop depth.
fn count_ident_uses(file: &File, name: &str) -> usize {
    let mut count = 0usize;
    let mut v = IdentUseCounter {
        name,
        count: &mut count,
    };
    v.visit_file(file);
    count
}

struct IdentUseCounter<'a> {
    name: &'a str,
    count: &'a mut usize,
}

impl<'ast> Visit<'ast> for IdentUseCounter<'_> {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path
            .get_ident()
            .map(std::string::ToString::to_string)
            .as_deref()
            == Some(self.name)
        {
            *self.count += 1;
        }
        syn::visit::visit_expr_path(self, p);
    }
}

struct CalleeScanner<'a> {
    annotated: &'a mut HashSet<String>,
    ref_fns: &'a mut HashSet<String>,
    fnvals: &'a mut HashSet<String>,
}

fn is_fn_valued_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::ImplTrait(it) => it
            .bounds
            .iter()
            .any(|b| matches!(b, syn::TypeParamBound::Trait(tb) if is_fn_bound(&tb.path))),
        syn::Type::TraitObject(to) => to
            .bounds
            .iter()
            .any(|b| matches!(b, syn::TypeParamBound::Trait(tb) if is_fn_bound(&tb.path))),
        _ => false,
    }
}

fn is_fn_bound(path: &syn::Path) -> bool {
    path.segments
        .last()
        .is_some_and(|s| matches!(s.ident.to_string().as_str(), "Fn" | "FnMut" | "FnOnce"))
}

impl<'ast> Visit<'ast> for CalleeScanner<'_> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let syn::Pat::Ident(pi) = &l.pat {
            if let Some(syn::Expr::Closure(c)) = l.init.as_ref().map(|i| &*i.expr) {
                if c.inputs.iter().any(|p| matches!(p, syn::Pat::Type(_))) {
                    self.annotated.insert(pi.ident.to_string());
                }
                // Every closure let is a non-Copy fn value — the E0382
                // census includes it, annotated or not.
                self.fnvals.insert(pi.ident.to_string());
            }
        }
        syn::visit::visit_local(self, l);
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        for arg in &i.sig.inputs {
            if let syn::FnArg::Typed(pt) = arg {
                if is_fn_valued_type(&pt.ty) {
                    if let syn::Pat::Ident(pi) = &*pt.pat {
                        self.fnvals.insert(pi.ident.to_string());
                    }
                }
            }
        }
        if has_ref_param(&i.sig) {
            self.ref_fns.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        if has_ref_param(&i.sig) {
            self.ref_fns.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_impl_item_fn(self, i);
    }

    fn visit_trait_item_fn(&mut self, i: &'ast syn::TraitItemFn) {
        if has_ref_param(&i.sig) {
            self.ref_fns.insert(i.sig.ident.to_string());
        }
        syn::visit::visit_trait_item_fn(self, i);
    }
}

fn has_ref_param(sig: &syn::Signature) -> bool {
    sig.inputs.iter().any(|arg| {
        matches!(
            arg,
            syn::FnArg::Typed(pt) if matches!(&*pt.ty, syn::Type::Reference(_))
        )
    })
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    annotated_closures: HashSet<String>,
    ref_param_fns: HashSet<String>,
    /// Every non-Copy fn-value binding name in the file (closure `let`s —
    /// annotated or not — and `impl Fn*`/`dyn Fn*`-typed fn params).
    fnvals: HashSet<String>,
    /// The fnval names occurring at 2+ single-ident path positions.
    fnval_multi_use: HashSet<String>,
    /// Loop nesting depth at the current visit position — a rewrite inside
    /// a loop body re-executes its move every iteration.
    loop_depth: usize,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_for_loop(&mut self, l: &'ast syn::ExprForLoop) {
        self.loop_depth += 1;
        syn::visit::visit_expr_for_loop(self, l);
        self.loop_depth -= 1;
    }

    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        self.loop_depth += 1;
        syn::visit::visit_expr_while(self, w);
        self.loop_depth -= 1;
    }

    fn visit_expr_loop(&mut self, l: &'ast syn::ExprLoop) {
        self.loop_depth += 1;
        syn::visit::visit_expr_loop(self, l);
        self.loop_depth -= 1;
    }

    fn visit_expr_closure(&mut self, c: &'ast ExprClosure) {
        // Depth-first: inner redundant closures collect their own edits (an
        // outer match's replacement region contains no other bare-id use, so
        // overlap selection cannot drop a live inner fix here).
        syn::visit::visit_expr_closure(self, c);
        if let Some(callee) = self.simplify_closure(c) {
            if let Some(text) = src(self.source, callee) {
                self.edits.push(edit(c, text));
            }
        }
    }
}

impl Collector<'_> {
    /// If the closure is `|x| f(x)`, return the callee path `f`. Otherwise None.
    fn simplify_closure<'a>(&self, closure: &'a ExprClosure) -> Option<&'a syn::ExprPath> {
        if closure.inputs.len() != 1 {
            return None;
        }
        // The body is `Box<Expr>`; for `|x| f(x)`, the body is the call directly.
        // For `|x| { f(x) }`, the body is an `ExprBlock`. Handle the bare-call form.
        let call_expr = match &*closure.body {
            Expr::Call(_) => &*closure.body,
            Expr::Block(b) if b.block.stmts.len() == 1 => match &b.block.stmts[0] {
                syn::Stmt::Expr(e, None) => e,
                _ => return None,
            },
            _ => return None,
        };
        let Expr::Call(call) = call_expr else {
            return None;
        };
        if call.args.len() != 1 {
            return None;
        }
        let arg = &call.args[0];
        let syn::Pat::Ident(param_pat) = &closure.inputs[0] else {
            return None;
        };
        let arg_ident = match arg {
            Expr::Path(p) => p.path.get_ident(),
            _ => return None,
        };
        let arg_ident = arg_ident?;
        if arg_ident != &param_pat.ident {
            return None;
        }
        let Expr::Path(callee_path) = &*call.func else {
            return None;
        };
        // E0631 guard: a bare-ident callee whose same-file definition pins a
        // type that can diverge from the item type (an annotated closure
        // binding, or a fn item taking a reference) stays a closure — the
        // annotation/reference IS the coercion site.
        if let Some(seg) = callee_path.path.segments.last() {
            let name = seg.ident.to_string();
            if callee_path.path.leading_colon.is_none()
                && callee_path.path.segments.len() == 1
                && (self.annotated_closures.contains(&name) || self.ref_param_fns.contains(&name))
            {
                return None;
            }
            // E0382 guard (riir-refine issue 240 / dapps 2dde9e9): a bare
            // lowercase single-ident callee that is a known non-Copy fn
            // value (a closure `let`, or an `impl Fn*`/`dyn Fn*`-typed fn
            // parameter) MOVES when the rewrite places it in the by-value
            // slot. Inside a loop body the move re-executes every iteration
            // (the deposit.rs/pay.rs filter-in-loop sweep specimens — the
            // source spelling shows ONE use, so a use census alone cannot
            // see it); two or more in-file occurrences make the second a
            // use-after-move even without a loop. Uppercase idents are
            // enum/tuple-struct constructors (fn items, `Copy`) and unknown
            // lowercase names are imported items (`Copy`) — both keep
            // firing, matching upstream's MachineApplicable suggestion.
            let is_uppercase = name.starts_with(|c: char| c.is_ascii_uppercase());
            if !is_uppercase
                && self.fnvals.contains(&name)
                && (self.loop_depth > 0 || self.fnval_multi_use.contains(&name))
            {
                return None;
            }
        }
        Some(callee_path)
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn simplifies_redundant_closure_in_call() {
        let input = "fn sq(x: u32) -> u32 { x * x }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| sq(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(
            out.contains(".map(sq)"),
            "closure not replaced by bare path: {out}"
        );
        assert!(
            out.contains("fn sq(x: u32) -> u32 { x * x }"),
            "unrelated fn must be byte-identical: {out}"
        );
    }

    #[test]
    fn impl_fn_param_in_loop_declines() {
        // riir-refine issue 240 / dapps 2dde9e9 (deposit.rs:722,
        // pay.rs:970): the sweep rewrote `.filter(|row| keep(row))` into
        // `.filter(keep)` inside a for loop — `keep: impl Fn(&Row) -> bool`
        // is a non-Copy fn VALUE, the move re-executes every iteration,
        // E0382. The source spelling shows ONE use, so only the loop
        // context can see it. MUST NOT fire.
        let input = concat!(
            "struct Row;\n",
            "struct Store;\n",
            "impl Store { fn scan(&self) -> Vec<(usize, Vec<u8>)> { Vec::new() } }\n",
            "fn sweep(store: &Store, keep: impl Fn(&Row) -> bool) -> Vec<Row> {\n",
            "    let mut out = Vec::new();\n",
            "    for (_k, v) in store.scan() {\n",
            "        if let Some(row) = Option::<Row>::None.filter(|r| keep(r)) {\n",
            "            out.push(row);\n",
            "        }\n",
            "    }\n",
            "    out\n",
            "}\n",
        );
        let out = propose_fix("redundant_closure", input);
        assert!(
            out.is_none(),
            "in-loop impl-Fn callee must decline: {out:?}"
        );
    }

    #[test]
    fn impl_fn_param_second_use_declines() {
        // The loop-free half of the same class: two sequential uses of the
        // fn VALUE — the rewrite moves it at the first, E0382 at the
        // second. File-wide census, both sites decline.
        let input = concat!(
            "struct Row;\n",
            "fn tally(rows: &[Row], keep: impl Fn(&Row) -> bool) -> usize {\n",
            "    let a = rows.iter().filter(|r| keep(r)).count();\n",
            "    let b = rows.iter().map(|r| keep(r)).count();\n",
            "    a + b\n",
            "}\n",
        );
        let out = propose_fix("redundant_closure", input);
        assert!(
            out.is_none(),
            "second-use impl-Fn callee must decline: {out:?}"
        );
    }

    #[test]
    fn single_use_fnval_outside_loop_still_fires() {
        // Regression floor: ONE use of a fn VALUE, no loop — the rewrite
        // consumes the only use, the move is safe, upstream's suggestion
        // stands. (An annotated closure `let` still declines via the E0631
        // guard; this pins the UNANNOTATED single-use shape.)
        let input = concat!(
            "fn run(rows: &[u32]) -> Vec<u32> {\n",
            "    let score = |r: &u32| *r;\n",
            "    rows.iter().map(|r| score(r)).collect()\n",
            "}\n",
        );
        // The annotated param makes it an E0631 decline — swap to prove
        // the single-use floor on the UNANNOTATED form instead:
        let unannotated = concat!(
            "fn run(rows: &[u32], score: impl Fn(&u32) -> u32) -> Vec<u32> {\n",
            "    rows.iter().map(|r| score(r)).collect()\n",
            "}\n",
        );
        let out = propose_fix("redundant_closure", unannotated).expect("single-use fires");
        assert!(
            out.contains("map(score)"),
            "single-use fn value must still fire: {out}"
        );
        let _ = input; // the annotated twin above is covered by the E0631 fixtures
    }

    #[test]
    fn uppercase_constructor_in_loop_still_fires() {
        // Enum/tuple-struct constructors are fn ITEMS (Copy) — the in-loop
        // rewrite stays upstream-clean even at multi-use.
        let input = concat!(
            "fn run(rows: &[u32]) -> Vec<Option<u32>> {\n",
            "    let mut out = rows.iter().map(|r| Some(r)).collect::<Vec<_>>();\n",
            "    out.extend(rows.iter().map(|r| Some(r)));\n",
            "    out\n",
            "}\n",
        );
        let out = propose_fix("redundant_closure", input).expect("constructors fire");
        assert!(
            out.contains("map(Some)"),
            "Copy constructor must still fire: {out}"
        );
    }

    #[test]
    fn leaves_non_redundant_closure() {
        let input = "fn go() { let f = |x: i32| x + 1; }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "non-redundant closure touched: {out:?}");
    }

    #[test]
    fn leaves_two_arg_closure() {
        let input = "fn go() { let f = |x: i32, y: i32| add(x, y); }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "two-param closure touched: {out:?}");
    }

    #[test]
    fn leaves_method_call_closure() {
        let input = "fn go() { let f = |x: i32| x.abs(); }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "method-call closure touched: {out:?}");
    }

    #[test]
    fn qualified_path_callee_preserved() {
        let input =
            "fn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| u32::from(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(out.contains("map(u32::from)"), "{out}");
    }

    #[test]
    fn skips_annotated_closure_binding_callee() {
        // THE 2026-08-29 sweep E0631 class: `upload`'s `&[f32]` annotation is
        // the coercion site — `.map(upload)` needs upload: Fn(&Vec<f32>) and
        // fails. Must stay silent.
        let input = "fn run(w: Option<Vec<f32>>) -> Option<u32> {\n    let upload = |data: &[f32]| -> u32 { data.len() as u32 };\n    w.as_ref().map(|x| upload(x))\n}";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "annotated-closure callee fired: {out:?}");
    }

    #[test]
    fn unannotated_closure_binding_callee_still_fires() {
        // Positive control: an unannotated binding infers its param FROM this
        // call site, so the bare path re-derives the same type — safe.
        let input = "fn run(xs: Vec<u32>) -> Vec<u32> {\n    let double = |v| v * 2;\n    xs.iter().map(|n| double(n)).collect()\n}";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(
            out.contains("map(double)"),
            "unannotated callee lost: {out}"
        );
    }

    #[test]
    fn skips_fn_with_reference_param() {
        // `first(v)` coerces &Vec<u32> → &[u32]; `.map(first)` would need
        // Fn(&Vec<u32>) -> u32 — E0631. Reference-typed params can always be
        // a coercion target, so the fn-item form skips too.
        let input = "fn first(s: &[u32]) -> u32 { s[0] }\nfn run(vv: Vec<Vec<u32>>) -> Vec<u32> { vv.iter().map(|v| first(v)).collect() }";
        let out = propose_fix("redundant_closure", input);
        assert!(out.is_none(), "reference-param fn callee fired: {out:?}");
    }

    #[test]
    fn fn_with_value_param_still_fires() {
        // By-value params never deref-coerce — the rewrite is safe.
        let input = "fn inc(x: u32) -> u32 { x + 1 }\nfn run(v: Vec<u32>) -> Vec<u32> { v.into_iter().map(|n| inc(n)).collect() }";
        let out = propose_fix("redundant_closure", input).expect("matched");
        assert!(out.contains("map(inc)"), "by-value fn callee lost: {out}");
    }
}
