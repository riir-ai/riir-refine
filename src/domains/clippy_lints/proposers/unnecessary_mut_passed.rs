//! `clippy::unnecessary_mut_passed` — `f(&mut v)` → `f(&v)` + drop `mut`.
//!
//! Span-preserving (T6.4): each `&mut <path>` call arg's byte range is
//! replaced by `&<path text>`; each `mut <ident>` fn-param pattern's byte
//! range is replaced by the bare ident.
//!
//! **Type-blind + file-wide** (not `--fix` eligible): whether `&mut v` is
//! unnecessary depends on the callee's signature, and the param-`mut` strip
//! applies to every fn in the file once any call-site matches (the original
//! proposer's behavior, preserved). Parse-only validation cannot decide
//! either. The `self_evolve` loop (real `cargo clippy` verify) remains the
//! consumer.

use super::{SourceEdit, edit, for_each_fn, src};
use syn::visit::Visit;
use syn::{Expr, File, Pat};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = CallScan {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);

    // Second pass: drop `mut` from fn parameter patterns (file-wide, when
    // any call-site edit fired — the original proposer's behavior).
    if !v.edits.is_empty() {
        for_each_fn(file, |sig, _block| {
            for input in &sig.inputs {
                if let syn::FnArg::Typed(pat) = input {
                    if let Pat::Ident(pi) = &*pat.pat {
                        if pi.mutability.is_some() {
                            v.edits.push(edit(pi, pi.ident.to_string()));
                        }
                    }
                }
            }
        });
    }
    v.edits
}

struct CallScan<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for CallScan<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, call);
        for arg in &call.args {
            if let Expr::Reference(r) = arg {
                if r.mutability.is_some() && matches!(&*r.expr, Expr::Path(..)) {
                    if let Some(text) = src(self.source, &r.expr) {
                        self.edits.push(edit(arg, format!("&{text}")));
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
    fn drops_mut_from_call_and_binding() {
        let input = "fn count(v: &Vec<u8>) -> usize { v.len() }\nfn go(mut v: Vec<u8>) -> usize { count(&mut v) }";
        let out = propose_fix("unnecessary_mut_passed", input).expect("matched");
        assert!(
            out.contains("count(&v)"),
            "call-site mut not dropped: {out}"
        );
        assert!(
            out.contains("fn go(v: Vec<u8>)"),
            "binding mut not dropped: {out}"
        );
    }

    #[test]
    fn leaves_immutable_calls_unchanged() {
        let input =
            "fn count(v: &Vec<u8>) -> usize { v.len() }\nfn go(v: Vec<u8>) -> usize { count(&v) }";
        let out = propose_fix("unnecessary_mut_passed", input);
        assert!(out.is_none(), "non-mut call touched: {out:?}");
    }
}
