//! `sigmoid-minus-one` (Issue 185 T1, arXiv:2610.31642) — the MECHANICAL
//! member of the `softmax-collapse` family.
//!
//! `σ(z) - 1.0` rounds to EXACTLY 0.0 in f32 once `z` passes ~16.6 (the
//! collapse cliff): `σ(z)` rounds to 1.0 and the difference is lost. The
//! same quantity written as `-σ(-z)` keeps full f32 range (it underflows
//! only at ~103). It is the one-vs-rest / BCE positive-slot gradient, so a
//! collapsed slot silently stops learning while the negatives keep pushing.
//!
//! ## The two rewrites
//!
//! | Before | After |
//! |---|---|
//! | `σ(z) - 1.0` | `-σ(-z)` |
//! | `σ(z) - if d == label { 1.0 } else { 0.0 }` | `if d == label { -σ(-z) } else { σ(z) }` |
//! | `σ(z) - f32::from(d == label)` | `if d == label { -σ(-z) } else { σ(z) }` |
//!
//! The identity `σ(z) - 1 = -σ(-z)` is exact for the logistic function —
//! the rewrite is safe in the same sense as `nan-unsafe-sort-comparator`'s
//! `total_cmp`: equal where the old form was representable, correct where
//! it rounded away. (riir-reflex `6ba895d` is the shipped repair, verbatim
//! the split form above.)
//!
//! ## Gates (conservative — refuse more than fires)
//!
//! - **Logistic σ only, by name ALLOWLIST**: the callee's last path segment
//!   must split on `_` into `sigmoid` plus only {`exact`, `stable`, `f32`,
//!   `f64`, `scalar`, `safe`, `precise`, `logistic`, `std`, `ref`}. The
//!   identity needs `σ(-z) = 1 - σ(z)`, which an APPROXIMATION need not
//!   satisfy — `fast_sigmoid`, `hard_sigmoid`, `sigmoid_lut`,
//!   `sigmoid_deriv`, `log_sigmoid` and every repo-prefixed name refuse.
//!   Exactly one argument (`sigmoid(x, temperature)` refuses).
//! - **Free-call form only**: the left operand is the call itself, so
//!   `2.0 * sigmoid(x) - 1.0` (the tanh form) never matches by parse shape.
//! - **Not inside a comparison or `.abs()`**: `(σ(z) - 1.0).abs() < eps`
//!   is a saturation CHECK, not a gradient; its author asked about `1.0`.
//! - **The split form needs a bare context** (a `let` init, an assignment
//!   right-hand side, or inside parens): an `if` expression spliced into an
//!   arbitrary operator position can re-associate; refuse elsewhere.
//! - **Comment guard**: any comment inside the replaced span outside the
//!   kept fragments (callee, argument, selector) refuses.
//! - **Macro args refuse** (token-stream spans are unreliable), and macro
//!   bodies are never parsed (`assert!((sigmoid(100.0) - 1.0).abs() < ..)`).
//! - **Tests are fixture habitat**: `#[test]` fns and `#[cfg(test)]` mods
//!   are skipped — a "premise: the naive form collapses" test MUST keep the
//!   naive form.
//! - **Allow marker** (Issue 185 T4b): a `// softmax-collapse: allow —
//!   <reason>` comment on the site's line or the line directly above
//!   withholds the edit (the family's one spelling,
//!   [`super::softmax_collapse::ALLOW_MARKER`]); [`collect_edits_with_allow`]
//!   still reports it so a sweep can count it.
//! - Every applied fix rides the `--fix --verify` compile gate.

use std::ops::Range;

use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Expr, File, UnOp};

use super::softmax_collapse::{
    allow_reason, is_float_one, is_primitive_fn, is_test_fn, is_test_mod, onehot_shape, peel_paren,
};
use super::{src, SourceEdit};
use crate::edit_emitter::drops_interior_comment;

/// The rule id.
pub const RULE: &str = "sigmoid-minus-one";

/// Segments a logistic-σ name may carry besides `sigmoid` (the allowlist).
const SIGMOID_SEGMENTS: &[&str] = &[
    "exact", "stable", "f32", "f64", "scalar", "safe", "precise", "logistic", "std", "ref",
];

/// Is `name` the logistic σ by the allowlist?
#[must_use]
pub fn is_logistic_sigmoid_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let mut has = false;
    for seg in n.split('_') {
        match seg {
            "sigmoid" => has = true,
            s if SIGMOID_SEGMENTS.contains(&s) => {}
            _ => return false,
        }
    }
    has
}

pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    collect_edits_with_allow(source, file)
        .into_iter()
        .filter_map(|(e, allowed)| allowed.is_none().then_some(e))
        .collect()
}

/// Every candidate edit paired with its allow-marker reason (`Some` = the
/// site is marked and [`collect_edits`] withholds it) — for sweeps that
/// must count suppressed sites rather than fold them into zero.
#[must_use]
pub fn collect_edits_with_allow(source: &str, file: &File) -> Vec<(SourceEdit, Option<String>)> {
    let mut w = Walker {
        source,
        edits: Vec::new(),
        bare_ok: Vec::new(),
        guarded: Vec::new(),
        cmp_depth: 0,
    };
    w.visit_file(file);
    w.edits
        .into_iter()
        .map(|e| {
            let allowed = allow_reason(source, e.start);
            (e, allowed)
        })
        .collect()
}

struct Walker<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Spans where an `if` expression may replace the node verbatim.
    bare_ok: Vec<Range<usize>>,
    /// Spans that are an `.abs()` receiver (a saturation check).
    guarded: Vec<Range<usize>>,
    /// > 0 while inside a comparison operand.
    cmp_depth: usize,
}

fn span_of(e: &Expr) -> Range<usize> {
    e.span().byte_range()
}

/// `-z` as source: strip a leading unary minus (`σ(-y) - 1.0` → `-σ(y)`),
/// keep atoms bare, parenthesize everything else.
fn negated(source: &str, z: &Expr) -> Option<String> {
    match z {
        Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)) => src(source, peel_paren(&u.expr)),
        Expr::Path(_)
        | Expr::Lit(_)
        | Expr::Field(_)
        | Expr::MethodCall(_)
        | Expr::Call(_)
        | Expr::Index(_)
        | Expr::Paren(_) => Some(format!("-{}", src(source, z)?)),
        _ => Some(format!("-({})", src(source, z)?)),
    }
}

impl Walker<'_> {
    fn check(&mut self, b: &syn::ExprBinary, node: &Expr) {
        if !matches!(b.op, BinOp::Sub(_)) || self.cmp_depth > 0 {
            return;
        }
        let span = span_of(node);
        if self.guarded.contains(&span) {
            return;
        }
        let Expr::Call(call) = b.left.as_ref() else {
            return;
        };
        let Expr::Path(func) = call.func.as_ref() else {
            return;
        };
        let Some(last) = func.path.segments.last() else {
            return;
        };
        if !is_logistic_sigmoid_name(&last.ident.to_string()) || call.args.len() != 1 {
            return;
        }
        let Some(z) = call.args.first() else { return };
        if matches!(z, Expr::Macro(_)) {
            return;
        }
        let (Some(func_text), Some(z_text), Some(neg_z)) = (
            src(self.source, func),
            src(self.source, z),
            negated(self.source, z),
        ) else {
            return;
        };
        let pos = format!("{func_text}({z_text})");
        let flipped = format!("-{func_text}({neg_z})");
        let mut kept = vec![span_of(&call.func), span_of(z)];
        let replacement = if is_float_one(peel_paren(&b.right)) {
            flipped
        } else if let Some(oh) = onehot_shape(&b.right) {
            if !self.bare_ok.contains(&span) || matches!(oh.cond, Expr::Macro(_)) {
                return;
            }
            let cond = peel_paren(oh.cond);
            let Some(cond_text) = src(self.source, cond) else {
                return;
            };
            kept.push(span_of(cond));
            if oh.one_when_true {
                format!("if {cond_text} {{ {flipped} }} else {{ {pos} }}")
            } else {
                format!("if {cond_text} {{ {pos} }} else {{ {flipped} }}")
            }
        } else {
            return;
        };
        if drops_interior_comment(self.source, span.clone(), &kept) {
            return;
        }
        self.edits.push(SourceEdit {
            start: span.start,
            end: span.end,
            replacement,
        });
    }
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if is_test_fn(&f.attrs) || is_primitive_fn(&f.sig.ident.to_string()) {
            return;
        }
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        if is_test_fn(&f.attrs) || is_primitive_fn(&f.sig.ident.to_string()) {
            return;
        }
        syn::visit::visit_impl_item_fn(self, f);
    }

    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if is_test_fn(&f.attrs) {
            return;
        }
        syn::visit::visit_trait_item_fn(self, f);
    }

    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        if is_test_mod(m) {
            return;
        }
        syn::visit::visit_item_mod(self, m);
    }

    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let Some(init) = &l.init {
            self.bare_ok.push(span_of(&init.expr));
        }
        syn::visit::visit_local(self, l);
    }

    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        self.bare_ok.push(span_of(&a.right));
        syn::visit::visit_expr_assign(self, a);
    }

    fn visit_expr_paren(&mut self, p: &'ast syn::ExprParen) {
        self.bare_ok.push(span_of(&p.expr));
        syn::visit::visit_expr_paren(self, p);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "abs" {
            self.guarded.push(span_of(peel_paren(&m.receiver)));
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        if let Expr::Binary(b) = e {
            let is_cmp = matches!(
                b.op,
                BinOp::Eq(_)
                    | BinOp::Ne(_)
                    | BinOp::Lt(_)
                    | BinOp::Le(_)
                    | BinOp::Gt(_)
                    | BinOp::Ge(_)
            );
            if matches!(
                b.op,
                BinOp::AddAssign(_)
                    | BinOp::SubAssign(_)
                    | BinOp::MulAssign(_)
                    | BinOp::DivAssign(_)
            ) {
                self.bare_ok.push(span_of(&b.right));
            }
            self.check(b, e);
            if is_cmp {
                self.cmp_depth += 1;
                syn::visit::visit_expr(self, e);
                self.cmp_depth -= 1;
                return;
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(src_text: &str) -> (String, usize) {
        let file = syn::parse_file(src_text).expect("parse");
        let edits = collect_edits(src_text, &file);
        let n = edits.len();
        (super::super::apply_edits(src_text, edits), n)
    }

    /// riir-reflex 6ba895d (`label_heads.rs`) — the shipped repair is the
    /// exact split-form output.
    #[test]
    fn reflex_split_form_rewrites_to_the_shipped_repair() {
        let src = "fn step(z: f32, d: usize, label: usize) -> f32 {\n    let g = exact_sigmoid(z) - if d == label { 1.0 } else { 0.0 };\n    g\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1, "{out}");
        assert!(
            out.contains("let g = if d == label { -exact_sigmoid(-z) } else { exact_sigmoid(z) };"),
            "{out}"
        );
        // Idempotent: the repaired form never fires again.
        assert_eq!(applied(&out).1, 0);
    }

    #[test]
    fn literal_form_rewrites() {
        let src = "fn g(z: f32) -> f32 {\n    sigmoid(z) - 1.0\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1);
        assert!(out.contains("-sigmoid(-z)"), "{out}");
        assert!(!out.contains("1.0"), "{out}");
    }

    #[test]
    fn qualified_path_and_suffixed_literal_rewrite() {
        let src =
            "fn g(z: f32) -> f32 {\n    let a = crate::math::sigmoid_f32(z) - 1.0_f32;\n    a\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1);
        assert!(out.contains("-crate::math::sigmoid_f32(-z)"), "{out}");
    }

    #[test]
    fn compound_argument_is_parenthesized_and_neg_argument_is_stripped() {
        let src = "fn g(a: f32, b: f32, y: f32) -> (f32, f32) {\n    let p = sigmoid(a * b + 1.0) - 1.0;\n    let q = sigmoid(-y) - 1.0;\n    (p, q)\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 2);
        assert!(out.contains("-sigmoid(-(a * b + 1.0))"), "{out}");
        assert!(out.contains("let q = -sigmoid(y);"), "{out}");
    }

    #[test]
    fn swapped_arms_and_f32_from_forms_rewrite() {
        let src = "fn g(z: f32, d: usize, l: usize) -> (f32, f32) {\n    let a = sigmoid(z) - if d != l { 0.0 } else { 1.0 };\n    let b = sigmoid(z) - f32::from(d == l);\n    (a, b)\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 2, "{out}");
        assert!(
            out.contains("let a = if d != l { sigmoid(z) } else { -sigmoid(-z) };"),
            "{out}"
        );
        assert!(
            out.contains("let b = if d == l { -sigmoid(-z) } else { sigmoid(z) };"),
            "{out}"
        );
    }

    #[test]
    fn split_form_outside_a_bare_context_refuses() {
        let src = "fn g(z: f32, d: usize, l: usize, w: f32) -> f32 {\n    w * (sigmoid(z) - if d == l { 1.0 } else { 0.0 }) + call(sigmoid(z) - f32::from(d == l))\n}\n";
        let (out, n) = applied(src);
        // The parenthesized operand is bare; the call argument is not.
        assert_eq!(n, 1, "{out}");
        assert!(out.contains("w * (if d == l"), "{out}");
    }

    #[test]
    fn approximations_and_other_names_refuse() {
        for name in [
            "fast_sigmoid",
            "hard_sigmoid",
            "sigmoid_lut",
            "sigmoid_deriv",
            "log_sigmoid",
            "recfm_sigmoid",
            "dsigmoid",
            "sigmoid_grad",
        ] {
            let src = format!("fn g(z: f32) -> f32 {{\n    {name}(z) - 1.0\n}}\n");
            assert_eq!(applied(&src).1, 0, "{name} must refuse");
        }
        for name in [
            "sigmoid",
            "exact_sigmoid",
            "stable_sigmoid",
            "sigmoid_f64",
            "logistic_sigmoid",
        ] {
            assert!(is_logistic_sigmoid_name(name), "{name}");
        }
    }

    #[test]
    fn tanh_form_and_two_arg_sigmoid_refuse() {
        let src = "fn g(z: f32, t: f32) -> (f32, f32) {\n    (2.0 * sigmoid(z) - 1.0, sigmoid(z, t) - 1.0)\n}\n";
        assert_eq!(applied(src).1, 0);
    }

    #[test]
    fn saturation_checks_refuse() {
        let src = "fn g(z: f32) -> bool {\n    let a = (sigmoid(z) - 1.0).abs() < 1e-6;\n    let b = sigmoid(z) - 1.0 == 0.0;\n    a && b\n}\n";
        assert_eq!(applied(src).1, 0);
    }

    #[test]
    fn macro_bodies_and_macro_args_refuse() {
        let src = "fn g(z: f32) {\n    assert!((sigmoid(100.0) - 1.0).abs() < 1e-6);\n    let _ = sigmoid(m!()) - 1.0;\n}\n";
        assert_eq!(applied(src).1, 0);
    }

    #[test]
    fn interior_comment_refuses() {
        let src = "fn g(z: f32) -> f32 {\n    sigmoid(z) /* pos */ - 1.0\n}\n";
        assert_eq!(applied(src).1, 0);
    }

    #[test]
    fn test_fns_and_cfg_test_mods_are_skipped() {
        let src = "#[test]\nfn premise() {\n    let g = sigmoid(20.0) - 1.0;\n    let _ = g;\n}\n#[cfg(test)]\nmod tests {\n    fn h(z: f32) -> f32 { sigmoid(z) - 1.0 }\n}\n";
        assert_eq!(applied(src).1, 0);
    }

    #[test]
    fn complement_and_other_shapes_are_out_of_scope() {
        // `1.0 - σ(z)` (the complement) and `σ(z) - y` (a soft label) are
        // not this rule's mechanical identity.
        let src = "fn g(z: f32, y: f32) -> (f32, f32) {\n    let a = 1.0 - sigmoid(z);\n    let b = sigmoid(z) - y;\n    (a, b)\n}\n";
        assert_eq!(applied(src).1, 0);
    }

    #[test]
    fn impl_methods_and_closures_fire() {
        let src = "impl H {\n    fn grads(&self, zs: &[f32]) -> Vec<f32> {\n        zs.iter().map(|&z| sigmoid(z) - 1.0).collect()\n    }\n}\n";
        let (out, n) = applied(src);
        assert_eq!(n, 1);
        assert!(out.contains("|&z| -sigmoid(-z)"), "{out}");
    }

    #[test]
    fn allow_marker_withholds_the_edit_both_ways() {
        let marked = "fn g(z: f32) -> f32 {\n    // softmax-collapse: allow — incumbent arm, kill switch\n    sigmoid(z) - 1.0\n}\n";
        assert_eq!(applied(marked).1, 0, "marked → no edit");
        let file = syn::parse_file(marked).expect("parses");
        let all = collect_edits_with_allow(marked, &file);
        assert_eq!(all.len(), 1, "still reported for the sweep");
        assert_eq!(all[0].1.as_deref(), Some("incumbent arm, kill switch"));
        let unmarked = marked.replace(
            "softmax-collapse: allow — incumbent arm, kill switch",
            "plain",
        );
        assert_eq!(applied(&unmarked).1, 1, "unmarked → fires");
        let bare = marked.replace(" — incumbent arm, kill switch", "");
        assert_eq!(
            applied(&bare).1,
            1,
            "a reason-less marker does not suppress"
        );
    }

    #[test]
    fn corpus_round_trip() {
        use crate::domain::HealDomain;
        use crate::domains::sec::SecDomain;
        let corpus = SecDomain::build_corpus();
        let rule = corpus
            .iter()
            .find(|r| r.rule_id == RULE)
            .expect("rule present");
        let (out, n) = applied(&rule.buggy_code);
        assert_eq!(n, 1);
        assert_eq!(
            out, rule.fixed_code,
            "the corpus fixed fixture is the exact rewrite"
        );
    }
}
