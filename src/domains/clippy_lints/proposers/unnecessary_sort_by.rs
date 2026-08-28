//! `clippy::unnecessary_sort_by` — `v.sort_by(|a, b| a.cmp(b))` → `v.sort()`.
//!
//! `sort_by` with the natural-order closure `|a, b| a.cmp(b)` is exactly
//! `sort()`. The closure must match in EXACT argument order — `|a, b|
//! b.cmp(a)` is a REVERSE sort and must not fire. Span-preserving (T6.4):
//! the whole `sort_by` call's byte range is replaced by `<recv>.sort()`.
//!
//! Oracle-probed shapes (clippy 1.97, P8 recon — `MachineApplicable` only):
//! - plain idents, by-ref arg:  `sort_by(|a, b| a.cmp(b))` → `sort()`
//! - typed params:              `sort_by(|a: &T, b: &T| a.cmp(b))` → `sort()`
//! - deref bindings, owned arg: `sort_by(|&a, &b| a.cmp(&b))` →
//!   `sort_by_key(|&a| a)` (NOT `sort()` — clippy keeps the key form; the
//!   deref pattern text is preserved verbatim in the suggested closure)
//! - `sort_unstable_by` mirrors both: plain → `sort_unstable()`, deref →
//!   `sort_unstable_by_key(|&a| a)`
//!
//! Skipped (oracle-confirmed, no autofire):
//! - reversed order (plain deref `b.cmp(&a)` gets `MaybeIncorrect`
//!   `sort_by_key(|&b| Reverse(b))` — our no-autofire discipline skips it;
//!   reversed plain is fully silent)
//! - mixed bindings (`|&a, b|` — oracle-silent)
//! - free-fn form `Ord::cmp(&a, &b)` (oracle-silent)
//! - the field-key shape `|&a, &b| a.k.cmp(&b.k)` (oracle MA
//!   `sort_by_key(|&a| a.k)`) — a separate pattern family, not landed here.
//! - `&mut` reference patterns, subpatterns, wildcard inners

use super::helpers::is_postfix_safe;
use super::{SourceEdit, edit, src};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Pat};

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
        if let Some(replacement) = self.try_sort(m) {
            self.edits.push(edit(m, replacement));
        }
    }
}

/// One closure param classified against the oracle's two firing forms.
enum Binding<'a> {
    /// `a` / `a: &T` — the plain form (sorted by reference comparison).
    Plain(&'a syn::Ident),
    /// `&a` — the deref form; carries the pattern's verbatim source text
    /// (clippy's suggestion preserves it: `sort_by_key(|&a| a)`).
    Deref(&'a syn::Ident),
}

impl Collector<'_> {
    fn classify_input(pat: &Pat) -> Option<Binding<'_>> {
        match pat {
            Pat::Ident(pi) if pi.subpat.is_none() && pi.by_ref.is_none() => {
                Some(Binding::Plain(&pi.ident))
            }
            // `a: &T` — the type annotation wraps the ident pattern
            // (the Pat::Type-around-ident syn fact, the vec-macro precedent).
            Pat::Type(pt) => match &*pt.pat {
                Pat::Ident(pi) if pi.subpat.is_none() && pi.by_ref.is_none() => {
                    Some(Binding::Plain(&pi.ident))
                }
                _ => None,
            },
            Pat::Reference(pr) if pr.mutability.is_none() && pr.attrs.is_empty() => {
                match &*pr.pat {
                    Pat::Ident(pi) if pi.subpat.is_none() && pi.by_ref.is_none() => {
                        Some(Binding::Deref(&pi.ident))
                    }
                    _ => None, // wildcard / tuple / subpat under `&` — skip
                }
            }
            _ => None,
        }
    }

    /// `a.cmp(b)` in exact order (plain form) or `a.cmp(&b)` (deref form).
    fn body_matches(body: &Expr, a: &syn::Ident, b: &syn::Ident, deref: bool) -> bool {
        let Expr::MethodCall(cmp) = body else {
            return false;
        };
        if !cmp.attrs.is_empty() || cmp.turbofish.is_some() || cmp.method != "cmp" {
            return false;
        }
        if cmp.args.len() != 1 {
            return false;
        }
        let Expr::Path(recv_path) = &*cmp.receiver else {
            return false;
        };
        if recv_path.path.get_ident() != Some(a) {
            return false;
        }
        if deref {
            // owned-form arg: `&b`
            let Expr::Reference(r) = &cmp.args[0] else {
                return false;
            };
            if r.mutability.is_some() || !r.attrs.is_empty() {
                return false;
            }
            let Expr::Path(arg_path) = &*r.expr else {
                return false;
            };
            arg_path.path.get_ident() == Some(b)
        } else {
            let Expr::Path(arg_path) = &cmp.args[0] else {
                return false;
            };
            arg_path.path.get_ident() == Some(b)
        }
    }

    fn try_sort(&self, m: &ExprMethodCall) -> Option<String> {
        if !m.attrs.is_empty() || m.turbofish.is_some() {
            return None;
        }
        let unstable = match m.method.to_string().as_str() {
            "sort_by" => false,
            "sort_unstable_by" => true,
            _ => return None,
        };
        if m.args.len() != 1 {
            return None;
        }
        let Expr::Closure(c) = &m.args[0] else {
            return None;
        };
        if !c.attrs.is_empty() || c.inputs.len() != 2 {
            return None;
        }
        if c.asyncness.is_some() || c.constness.is_some() || c.movability.is_some() {
            return None; // dropping the modifier changes the closure type
        }
        // Both bindings must be the SAME oracle form — mixed (`|&a, b|`) is
        // oracle-silent, and deref + plain bodies cannot both compile.
        let (deref, a, b) = match (
            Self::classify_input(&c.inputs[0])?,
            Self::classify_input(&c.inputs[1])?,
        ) {
            (Binding::Deref(a), Binding::Deref(b)) => (true, a, b),
            (Binding::Plain(a), Binding::Plain(b)) => (false, a, b),
            _ => return None,
        };
        if !Self::body_matches(&c.body, a, b, deref) {
            return None;
        }
        let recv = src(self.source, &*m.receiver)?;
        let recv = if is_postfix_safe(&m.receiver) {
            recv
        } else {
            format!("({recv})")
        };
        if deref {
            // clippy keeps the KEY form for deref bindings and preserves
            // the pattern text verbatim: `sort_by_key(|&a| a)`.
            let pat_text = src(self.source, &c.inputs[0])?;
            let method = if unstable {
                "sort_unstable_by_key"
            } else {
                "sort_by_key"
            };
            Some(format!("{recv}.{method}(|{pat_text}| {a})"))
        } else {
            let method = if unstable { "sort_unstable" } else { "sort" };
            Some(format!("{recv}.{method}()"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_natural_order_sort_by() {
        let input = "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort()"), "sort not produced: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn deref_bindings_fire_sort_by_key() {
        // P8 (clippy 1.97 oracle, MachineApplicable): the deref form keeps
        // the KEY form with the pattern text verbatim — `sort_by_key(|&a| a)`.
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&a, &b| a.cmp(&b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(
            out.contains("v.sort_by_key(|&a| a)"),
            "deref form must produce sort_by_key: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn sort_unstable_by_plain_fires() {
        let input = "fn f(v: &mut Vec<u8>) { v.sort_unstable_by(|a, b| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_unstable()"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn sort_unstable_by_deref_fires_key_form() {
        let input = "fn f(v: &mut Vec<u8>) { v.sort_unstable_by(|&a, &b| a.cmp(&b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_unstable_by_key(|&a| a)"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn typed_params_fire_sort() {
        // oracle line 41: `|a: &u8, b: &u8| a.cmp(b)` → `sort()` (MA)
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|a: &u8, b: &u8| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort()"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn reversed_deref_not_fired() {
        // oracle: fires with `sort_by_key(|&b| Reverse(b))` at MAYBEINCORRECT
        // — our no-autofire discipline skips MaybeIncorrect suggestions.
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&a, &b| b.cmp(&a)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "reversed deref must not fire: {out:?}");
    }

    #[test]
    fn mixed_deref_and_plain_not_fired() {
        // oracle-silent: `|&a, b| a.cmp(b)` gets NO finding at 1.97.
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&a, b| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "mixed bindings must not fire: {out:?}");
    }

    #[test]
    fn deref_body_with_plain_arg_not_fired() {
        // owned bindings take `&b` in compiling code; `a.cmp(b)` with both
        // deref bindings is E0308 — never a real shape, never fired.
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&a, &b| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "deref + plain arg must not fire: {out:?}");
    }

    #[test]
    fn deref_wildcard_inner_not_fired() {
        // non-ident under `&` (wildcard / tuple / subpat) — outside the shape.
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&_, &b| b.cmp(&b)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "wildcard deref must not fire: {out:?}");
    }

    #[test]
    fn free_fn_cmp_form_not_fired() {
        // oracle-silent: `Ord::cmp(&a, &b)` is a Call body, not a MethodCall.
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|&a, &b| Ord::cmp(&a, &b)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "free-fn cmp must not fire: {out:?}");
    }

    #[test]
    fn reversed_order_unchanged() {
        let input = "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| b.cmp(a)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "reverse sort must not fire: {out:?}");
    }

    #[test]
    fn already_sorted_unchanged() {
        let input = "fn f(mut v: Vec<u8>) { v.sort(); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "already sort() must not fire: {out:?}");
    }

    #[test]
    fn custom_comparator_unchanged() {
        let input = "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| a.abs().cmp(&b.abs())); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "non-cmp body must not fire: {out:?}");
    }
}
