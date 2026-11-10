//! `clippy::map_clone` — `v.iter().map(|x| x.clone())` → `v.iter().cloned()`
//! and the deref twin `.map(|x| *x)` → `.copied()` (Batch 171 / Plan 163 —
//! the coverage-report pack XIV; envelope verified under the plain oracle
//! 2026-09-21, `scripts/plan163_pack14_probe.py`).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! For the clone shape: the input compiling PROVES the closure param is a
//! shared reference to a `Clone` type (`x.clone()` on `&T` with `T: Clone`
//! is exactly what `Iterator::cloned()` does per element). For the deref
//! shape: `*x` through a shared reference only compiles for `Copy` types —
//! which is exactly `Iterator::copied()`'s bound. Both adapter choices are
//! therefore TYPE-INDEPENDENT given the shape compiles — no signature
//! provenance table needed.
//!
//! clippy's own suggestion prefers `copied()` for the clone shape when the
//! element is `Copy` (probe: `v.iter().copied()` on u8) — we always emit
//! `cloned()` there, always-compilable and semantics-identical for Copy
//! types; the diverge-from-the-literal-suggestion precedent
//! (`string_lit_as_bytes`).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The receiver chain must contain a BY-REF ITERATOR ADAPTER (`iter`,
//!   `iter_mut`, `values`, `keys`) and must NOT contain `into_iter`
//!   ANYWHERE. Probe: clippy fires on `into_iter().map(|x| x.clone())`
//!   with an EMPTY suggestion (delete the map — valid only because the
//!   probe's element type was Copy); on an owned-`String` iterator that
//!   deletion would erase a real per-element clone. Refused wholesale —
//!   the documented narrower-than-clippy divergence.
//! - Single-param, attribute-free closures whose body is EXACTLY
//!   `param.clone()` (no args, no turbofish) or `*param` — nothing nested
//!   (probe `mc_nested`: a clone inside a larger expression is SILENT;
//!   mirrored).
//! - `map` itself must carry no turbofish and exactly one argument.
//! - The clone shape refuses a `clone` with a turbofish or arguments —
//!   those are not the identity closure.
//! - Fixpoint by construction: the output has no `.map` — stable.

use syn::visit::Visit;
use syn::{Expr, File};

use super::SourceEdit;
use super::helpers::byte_range;

const BY_REF_ADAPTERS: [&str; 3] = ["iter", "values", "keys"];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = Scan { edits: Vec::new() };
    scan.visit_file(file);
    scan.edits
}

struct Scan {
    edits: Vec<SourceEdit>,
}

/// The receiver chain contains a by-ref adapter and no `into_iter`.
fn chain_is_by_ref(recv: &Expr) -> bool {
    let mut cur = recv;
    let mut by_ref = false;
    while let Expr::MethodCall(mc) = cur {
        if mc.turbofish.is_some() {
            // a generic adapter's element type is not decidable here
            return false;
        }
        let name = mc.method.to_string();
        if name == "into_iter" {
            return false;
        }
        if BY_REF_ADAPTERS.contains(&name.as_str()) {
            by_ref = true;
        }
        cur = &mc.receiver;
    }
    by_ref
}

/// The single closure body shape: exactly `p.clone()` / `*p`.
enum Body {
    Clone,
    Deref,
}

fn closure_body_shape(body: &Expr, param: &str) -> Option<Body> {
    let is_param = |e: &Expr| -> bool {
        let Expr::Path(p) = e else { return false };
        p.qself.is_none()
            && p.path.leading_colon.is_none()
            && p.path.segments.len() == 1
            && p.path.segments[0].arguments.is_none()
            && p.path.segments[0].ident == param
    };
    match body {
        Expr::MethodCall(mc) => {
            if mc.method == "clone"
                && mc.args.is_empty()
                && mc.turbofish.is_none()
                && mc.attrs.is_empty()
                && is_param(&mc.receiver)
            {
                Some(Body::Clone)
            } else {
                None
            }
        }
        Expr::Unary(u) => {
            if matches!(u.op, syn::UnOp::Deref(_)) && u.attrs.is_empty() && is_param(&u.expr) {
                Some(Body::Deref)
            } else {
                None
            }
        }
        _ => None,
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_expr_method_call(&mut self, mc: &'ast syn::ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, mc);
        if mc.method != "map"
            || mc.turbofish.is_some()
            || mc.args.len() != 1
            || !mc.attrs.is_empty()
        {
            return;
        }
        let Expr::Closure(c) = &mc.args[0] else {
            return;
        };
        if !c.attrs.is_empty() {
            return;
        }
        // exactly one input, and the ident must be a plain binding
        if c.inputs.len() != 1 {
            return;
        }
        let syn::Pat::Ident(pi) = &c.inputs[0] else {
            return;
        };
        if pi.by_ref.is_some() || pi.mutability.is_some() || pi.subpat.is_some() {
            return;
        }
        let param = pi.ident.to_string();
        let Some(shape) = closure_body_shape(&c.body, &param) else {
            return;
        };
        if !chain_is_by_ref(&mc.receiver) {
            return;
        }
        let recv_end = byte_range(&mc.receiver).end;
        let call_end = byte_range(mc).end;
        if call_end > recv_end {
            let name = match shape {
                Body::Clone => ".cloned()",
                Body::Deref => ".copied()",
            };
            self.edits.push(SourceEdit {
                start: recv_end,
                end: call_end,
                replacement: name.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const SEED: &str =
        "pub fn f(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| x.clone()).collect()\n}";

    #[test]
    fn seed_fires_and_rewrites() {
        let out = propose_fix("map_clone", SEED).expect("matched");
        assert!(out.contains("v.iter().cloned()"), "cloned missing: {out}");
        assert!(!out.contains(".map("), "map survived: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn deref_body_becomes_copied() {
        // `*x` through a shared ref only compiles for Copy — copied()'s
        // exact bound (type-independent by construction)
        let src = "pub fn f(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| *x).collect()\n}";
        let out = propose_fix("map_clone", src).expect("matched");
        assert!(out.contains("v.iter().copied()"), "copied missing: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn values_chain_fires() {
        let src = "pub fn f(m: &std::collections::BTreeMap<u8, u8>) -> Vec<u8> {\n    m.values().map(|x| x.clone()).collect()\n}";
        let out = propose_fix("map_clone", src).expect("matched");
        assert!(out.contains("m.values().cloned()"), "values chain: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn iter_mut_chain_refuses() {
        // unprobed adapter (the probe covered iter/values only) — the
        // conservative v1: `x.clone()` on `&mut T` derefs to T::clone and
        // IterMut::cloned() exists, but the shape is not probe-verified;
        // refuse until the oracle pins it (the later-widening class)
        let src = "pub fn f(v: &mut Vec<u8>) -> Vec<u8> {\n    v.iter_mut().map(|x| x.clone()).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn into_iter_chain_refuses() {
        // probe: clippy fires with an EMPTY suggestion (Copy-only
        // deletion); on owned non-Copy iterators that would erase a real
        // clone — refused (the documented divergence)
        let src =
            "pub fn f(v: Vec<u8>) -> Vec<u8> {\n    v.into_iter().map(|x| x.clone()).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn nested_body_refuses() {
        // probe mc_nested: clone inside a larger expression is SILENT
        let src = "pub fn f(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| (x.clone(), 1).0).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn arbitrary_method_refuses() {
        // probe mc_arbitrary: a different method is not the identity
        let src =
            "pub fn f(v: &Vec<String>) -> Vec<usize> {\n    v.iter().map(|x| x.len()).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn no_adapter_refuses() {
        // no by-ref adapter in the chain — element kind undecidable
        let src =
            "pub fn f(v: &Vec<u8>) -> Vec<u8> {\n    v.clone().map(|x| x.clone()).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn clone_with_args_refuses() {
        let src = "pub fn f(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x| x.clone_other()).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn multi_param_refuses() {
        let src =
            "pub fn f(v: &Vec<u8>) -> Vec<u8> {\n    v.iter().map(|x, _| x.clone()).collect()\n}";
        assert!(propose_fix("map_clone", src).is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let first = propose_fix("map_clone", SEED).expect("matched");
        assert!(propose_fix("map_clone", &first).is_none());
    }
}
