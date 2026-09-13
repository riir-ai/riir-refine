//! `clippy::useless_conversion` — two shapes, both type-blind:
//!
//! 1. `let x: T = y.into()` → `let x: T = y` (useless only when `y`'s type
//!    already equals the annotation `T`).
//! 2. `f(y.into_iter())` → `f(y)` where the callee param accepts
//!    `IntoIterator` — clippy 1.98 fires this shape as `useless_conversion`
//!    ("explicit call to `.into_iter()` in function argument accepting
//!    `IntoIterator`", machine-applicable). Probe 2026-09-14; live specimen:
//!    riir-ai `624f24958` (`preupload_all(projections.into_iter())`).
//!
//! Span-preserving (T6.4): the `.into()`/`.into_iter()` call's byte range is
//! replaced by the ORIGINAL source text of its receiver.
//!
//! **Type-blind** (not `--fix` eligible): the rewrite is only correct when
//! the types line up — shape 1 needs `y: T`, shape 2 needs the callee param
//! to be `IntoIterator`-generic. A parse-only validator cannot know either.
//! Every wrong application is a COMPILE error (dropping the call where the
//! callee wanted `Iterator`/`T` breaks resolution), never a silent behavior
//! change — the `self_evolve` loop (real `cargo clippy` verify) and the
//! `--clippy-json` fed lane (clippy's typeck is the authority) remain the
//! consumers.

use super::{edit, src, SourceEdit};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, File, Local, Token};

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
    fn visit_local(&mut self, local: &'ast Local) {
        syn::visit::visit_local(self, local);
        // Must have an explicit type annotation + an init.
        let Some(init) = &local.init else { return };
        if init.diverge.is_some() {
            return;
        }
        if !matches!(&local.pat, syn::Pat::Type(_)) {
            return;
        }
        // init.expr must be a `.into()` method call.
        if let Expr::MethodCall(mc) = init.expr.as_ref() {
            if mc.method == "into" && mc.args.is_empty() {
                if let Some(text) = src(self.source, &*mc.receiver) {
                    self.edits.push(edit(&init.expr, text));
                }
            }
        }
    }

    fn visit_expr_call(&mut self, expr: &'ast ExprCall) {
        syn::visit::visit_expr_call(self, expr);
        self.scan_call_args(&expr.args);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, expr);
        self.scan_call_args(&expr.args);
    }
}

impl Collector<'_> {
    /// Shape 2: an argument that is exactly `<expr>.into_iter()` → replace
    /// the whole argument span with the receiver's original text. Only a
    /// DIRECT argument matches (the shape clippy fires on); a chained
    /// `v.into_iter().map(..)` argument keeps its `into_iter` (its removal
    /// would break the chain), and a bare `let it = v.into_iter()` binding
    /// is not a call argument at all.
    fn scan_call_args(&mut self, args: &Punctuated<Expr, Token![,]>) {
        for arg in args {
            if let Expr::MethodCall(mc) = arg {
                let plain_call = mc.method == "into_iter"
                    && mc.args.is_empty()
                    && mc.turbofish.is_none();
                if plain_call {
                    if let Some(text) = src(self.source, &*mc.receiver) {
                        self.edits.push(edit(arg, text));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn drops_redundant_into() {
        let input = "fn twice(n: u64) -> u64 {\n    let m: u64 = n.into();\n    m * 2\n}";
        let out = propose_fix("useless_conversion", input).expect("matched");
        assert!(out.contains("let m: u64 = n;"), "into not dropped: {out}");
        assert!(!out.contains(".into"), "{out}");
    }

    #[test]
    fn leaves_non_into_let_unchanged() {
        let input = "fn twice(n: u64) -> u64 {\n    let m: u64 = n + 1;\n    m\n}";
        let out = propose_fix("useless_conversion", input);
        assert!(out.is_none(), "non-into let touched: {out:?}");
    }

    #[test]
    fn leaves_let_without_type_annotation() {
        let input = "fn twice(n: u64) -> u64 {\n    let m = n.into();\n    m\n}";
        let out = propose_fix("useless_conversion", input);
        assert!(out.is_none(), "no-type-annot let touched: {out:?}");
    }

    #[test]
    fn parenthesized_receiver_keeps_parens() {
        // The receiver of `(a + b).into()` is the Paren node — its ORIGINAL
        // text (parens included) re-parses standalone in the let-init slot.
        let input = "fn f(a: u64, b: u64) -> u64 { let m: u64 = (a + b).into(); m }";
        let out = propose_fix("useless_conversion", input).expect("matched");
        assert!(out.contains("let m: u64 = (a + b);"), "{out}");
    }

    // ── Shape 2: `.into_iter()` as a direct call argument (2026-09-14
    //    intake, the riir-ai 624f24958 coverage miss). ──

    #[test]
    fn drops_into_iter_fn_arg() {
        // Probe shape: clippy 1.98 fires useless_conversion on the direct
        // argument of an IntoIterator-accepting free fn.
        let input = "fn sink<I: IntoIterator<Item = u32>>(items: I) {}
fn caller(v: Vec<u32>) { sink(v.into_iter()); }";
        let out = propose_fix("useless_conversion", input).expect("matched");
        assert!(out.contains("sink(v);"), "into_iter not dropped: {out}");
        assert!(!out.contains(".into_iter"), "{out}");
    }

    #[test]
    fn drops_into_iter_method_arg_specimen() {
        // The live specimen's exact shape: method-call receiver + method arg.
        let input =
            "fn run(hook: &mut Hook, projections: Vec<Layer>) { hook.preupload_all(projections.into_iter()); }";
        let out = propose_fix("useless_conversion", input).expect("matched");
        assert!(
            out.contains("hook.preupload_all(projections)"),
            "specimen not healed: {out}"
        );
    }

    #[test]
    fn leaves_let_bound_into_iter_unchanged() {
        // Not a call argument: `let it = v.into_iter()` is a legitimate
        // binding (and `for x in v.into_iter()` is a DIFFERENT lint's shape).
        let input = "fn caller(v: Vec<u32>) { let it = v.into_iter(); for x in it {} }";
        let out = propose_fix("useless_conversion", input);
        assert!(out.is_none(), "let-bound into_iter touched: {out:?}");
    }

    #[test]
    fn leaves_for_loop_into_iter_unchanged() {
        // `for x in v.into_iter()` — clippy's for-loop shape is
        // `explicit_into_iter_loop` (pedantic tier), NOT this rule.
        let input = "fn caller(v: Vec<u32>) { for x in v.into_iter() {} }";
        let out = propose_fix("useless_conversion", input);
        assert!(out.is_none(), "for-loop into_iter touched: {out:?}");
    }

    #[test]
    fn leaves_chained_into_iter_arg_unchanged() {
        // `v.into_iter().map(..)` is NOT a direct argument — dropping the
        // inner call would break the chain; clippy does not fire this shape.
        let input =
            "fn sink<I: IntoIterator<Item = u32>>(items: I) {}
fn caller(v: Vec<u32>) { sink(v.into_iter().map(|x| x * 2)); }";
        let out = propose_fix("useless_conversion", input);
        assert!(out.is_none(), "chained into_iter touched: {out:?}");
    }

    #[test]
    fn arg_shape_overmatch_on_iterator_param_is_type_blind() {
        // HONEST LIMITATION PIN: `sink_iter` takes `Iterator` (NOT
        // `IntoIterator`), so dropping `.into_iter()` BREAKS this call — the
        // edit must NOT compile. A parse-only proposer cannot distinguish
        // the two signatures; that is exactly the documented type-blind tier
        // (suggest/oracle lanes only, never `--fix`). The probe crate's
        // negative control (2026-09-14) proved real clippy stays silent
        // here — in the `--clippy-json` fed lane clippy's typeck is the
        // authority and this span never surfaces.
        let input = "fn sink_iter<I: Iterator<Item = u32>>(items: I) {}
fn caller(v: Vec<u32>) { sink_iter(v.into_iter()); }";
        let out = propose_fix("useless_conversion", input).expect("matched");
        assert!(out.contains("sink_iter(v)"), "{out}");
    }
}
