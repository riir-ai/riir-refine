//! `clippy::useless_conversion` — `let x: T = y.into()` → `let x: T = y`.
//!
//! Span-preserving (T6.4): the `.into()` call's byte range is replaced by
//! the ORIGINAL source text of its receiver.
//!
//! **Type-blind** (not `--fix` eligible): the rewrite is only correct when
//! `y`'s type already equals the annotation `T` — a parse-only validator
//! cannot know that. The `self_evolve` loop (real `cargo clippy` verify)
//! remains the consumer.

use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, File, Local};

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
}
