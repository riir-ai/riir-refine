//! `clippy::unnecessary_sort_by` — `v.sort_by(|a, b| a.cmp(b))` → `v.sort()`
//! plus the key-expression family: `v.sort_by(|a, b| a.k.cmp(&b.k))` →
//! `v.sort_by_key(|a| a.k)`.
//!
//! `sort_by` with the natural-order closure `|a, b| a.cmp(b)` is exactly
//! `sort()`. The closure must match in EXACT argument order — `|a, b|
//! b.cmp(a)` is a REVERSE sort and must not fire. Span-preserving (T6.4):
//! the whole `sort_by` call's byte range is replaced.
//!
//! Oracle-probed shapes (clippy 1.97 — P8 + P8b probes, `MachineApplicable`
//! only). The KEY family is general: the cmp receiver and the reference
//! argument's inner expression must be identical modulo an `a`↔`b` swap
//! (checked textually at word boundaries — asymmetric args, mixed fields,
//! and different methods fail the swap and stay silent, matching oracle):
//!
//! - plain idents, by-ref arg:  `sort_by(|a, b| a.cmp(b))` → `sort()`
//! - typed params:              `sort_by(|a: &T, b: &T| a.cmp(b))` → `sort()`
//! - field-key plain:           `sort_by(|a, b| a.k.cmp(&b.k))` →
//!   `sort_by_key(|a| a.k)` — the TYPE ANNOTATION is dropped from the
//!   suggested closure (oracle fact: `|a: &T, b: &T| a.k.cmp(&b.k)` →
//!   `sort_by_key(|a| a.k)`)
//! - field-key deref:           `sort_by(|&a, &b| a.k.cmp(&b.k))` →
//!   `sort_by_key(|&a| a.k)` — the deref pattern text is preserved verbatim
//! - bare-ident deref:          `sort_by(|&a, &b| a.cmp(&b))` →
//!   `sort_by_key(|&a| a)` (the P8 deref family, subsumed by the key check)
//! - `sort_unstable_by` mirrors all of the above with
//!   `sort_unstable[_by_key]`
//! - general key expressions all fire MA when the swap holds: multi-level
//!   fields (`a.n.c`), method-call keys (`a.n.m()`), postfix chains
//!   (`a.k.abs()` — NOTE: the old `custom_comparator` skip encoded a FALSE
//!   oracle fact; the probe showed MA `sort_by_key(|a| a.abs())`),
//!   tuple keys (`(a.k, a.j)` — parens kept), shared constants and
//!   param-referencing method args inside the key (`a.k.max(MAX)`,
//!   `a.k.wrapping_add(a.j)`), external idents (`a.k.max(w.len() as i32)`)
//!
//! Skipped (oracle facts):
//! - reversed order — plain fully silent; reversed field-key gets
//!   `MaybeIncorrect` `sort_by_key(|b| Reverse(b.k))` — the no-autofire
//!   discipline skips it (our word guard rejects recv-mentions-`b`)
//! - mixed bindings `|&a, b|` / `|a, &b|` — oracle-silent for key forms too
//! - asymmetric args (`a.k.max(1)` vs `b.k.max(2)`) and different methods
//!   (`max` vs `min`) — oracle-silent; the swap check rejects them
//! - Paren-grouped key recv `(a.k + 1)` — oracle FIRES but de-parens the
//!   rendered key (`|a| a.k + 1`); our byte-for-byte parity standard cannot
//!   reproduce HIR de-parening, so the shape is deliberately skipped
//!   (false-negative-safe)
//! - free-fn form `Ord::cmp(&a, &b)` (oracle-silent)
//! - `&mut` reference args, `|a, b| a.cmp(&b)` with plain bindings (that
//!   shape is `&&T` — E0599, cannot compile), subpatterns, wildcard inners
//! - `r#raw` ident params (not modeled)

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

/// One closure param classified against the oracle's binding forms.
enum Binding<'a> {
    /// `a` / `a: &T` — the plain form (sorted by reference comparison; the
    /// type annotation is DROPPED in key-form suggestions, oracle fact).
    Plain(&'a syn::Ident),
    /// `&a` — the deref form; carries the pattern's verbatim source text
    /// (clippy's suggestion preserves it: `sort_by_key(|&a| a)`).
    Deref(&'a syn::Ident),
}

/// Word-boundary occurrence of `word` in `text` (Rust ident chars only).
fn contains_word(text: &str, word: &str) -> bool {
    let (tb, wb) = (text.as_bytes(), word.as_bytes());
    let (n, m) = (tb.len(), wb.len());
    if m == 0 {
        return false;
    }
    let mut i = 0;
    while i + m <= n {
        if &tb[i..i + m] == wb {
            let left_ok = i == 0 || !is_ident_char(tb[i - 1]);
            let right_ok = i + m == n || !is_ident_char(tb[i + m]);
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Replace every word-boundary `from` with `to` (Rust ident chars only).
fn replace_word(text: &str, from: &str, to: &str) -> String {
    let tb = text.as_bytes();
    let m = from.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < tb.len() {
        if i + m <= tb.len() && &tb[i..i + m] == from.as_bytes() {
            let left_ok = i == 0 || !is_ident_char(tb[i - 1]);
            let right_ok = i + m == tb.len() || !is_ident_char(tb[i + m]);
            if left_ok && right_ok {
                out.push_str(to);
                i += m;
                continue;
            }
        }
        // Copy one full UTF-8 scalar (ident texts are ASCII but the file
        // text around them need not be).
        let ch_len = utf8_len(tb[i]);
        out.push_str(&text[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn utf8_len(b: u8) -> usize {
    match b {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

fn is_ident_char(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric()
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
        let Expr::Closure(closure) = &m.args[0] else {
            return None;
        };
        if !closure.attrs.is_empty() || closure.inputs.len() != 2 {
            return None;
        }
        if closure.asyncness.is_some() || closure.constness.is_some() || closure.movability.is_some()
        {
            return None; // dropping the modifier changes the closure type
        }
        // Both bindings must be the SAME oracle form — mixed (`|&a, b|`) is
        // oracle-silent for the ident AND the key families.
        let (deref, pa, pb) = match (
            Self::classify_input(&closure.inputs[0])?,
            Self::classify_input(&closure.inputs[1])?,
        ) {
            (Binding::Deref(a), Binding::Deref(b)) => (true, a, b),
            (Binding::Plain(a), Binding::Plain(b)) => (false, a, b),
            _ => return None,
        };
        let Expr::MethodCall(cmp) = &*closure.body else {
            return None;
        };
        if !cmp.attrs.is_empty() || cmp.turbofish.is_some() || cmp.method != "cmp" {
            return None;
        }
        if cmp.args.len() != 1 {
            return None;
        }
        let recv_src = src(self.source, &*m.receiver)?;
        let recv_src = if is_postfix_safe(&m.receiver) {
            recv_src
        } else {
            format!("({recv_src})")
        };
        let plain_method = if unstable { "sort_unstable" } else { "sort" };

        // Case 1 — natural order on the bare params: `a.cmp(b)` → `sort()`.
        // Only valid for plain bindings (owned `a.cmp(b)` is E0308).
        if !deref {
            if let (Expr::Path(rp), Expr::Path(ap)) = (&*cmp.receiver, &cmp.args[0]) {
                if rp.path.get_ident() == Some(pa) && ap.path.get_ident() == Some(pb) {
                    return Some(format!("{recv_src}.{plain_method}()"));
                }
            }
        }

        // Case 2 — the key family: `EXPR(a).cmp(&EXPR(b))` where EXPR swaps
        // a↔b cleanly → `sort_by_key(|PARAM| EXPR(a))`.
        let Expr::Reference(argref) = &cmp.args[0] else {
            return None;
        };
        if argref.mutability.is_some() || !argref.attrs.is_empty() {
            return None;
        }
        // `|a, b| a.cmp(&b)` with plain bindings is `&&T` — E0599, cannot
        // compile; the deref twin is the only legal bare form.
        if !deref {
            if let (Expr::Path(rp), Expr::Path(ip)) = (&*cmp.receiver, &*argref.expr) {
                if rp.path.get_ident().is_some() && ip.path.get_ident().is_some() {
                    return None;
                }
            }
        }
        // Oracle de-parens Paren-grouped keys (HIR rendering) — our
        // byte-for-byte parity standard cannot reproduce that, so skip
        // (false-negative-safe). Tuple parens are part of the HIR rendering
        // and are kept.
        if matches!(&*cmp.receiver, Expr::Paren(_)) {
            return None;
        }
        let recv_text = src(self.source, &*cmp.receiver)?;
        let inner_text = src(self.source, &*argref.expr)?;
        let a_s = pa.to_string();
        let b_s = pb.to_string();
        // Forward order: the receiver mentions `a` (never `b`); the swap
        // `a→b` must reproduce the argument's inner text exactly. This
        // rejects reversed sorts (recv mentions `b`), mixed fields,
        // asymmetric args, and different methods — all oracle-silent.
        if !contains_word(&recv_text, &a_s) || contains_word(&recv_text, &b_s) {
            return None;
        }
        if !contains_word(&inner_text, &b_s) {
            return None;
        }
        if replace_word(&recv_text, &a_s, &b_s) != inner_text {
            return None;
        }
        // Plain bindings: the suggested closure keeps the bare ident (the
        // type annotation is dropped — oracle fact). Deref bindings: the
        // pattern text is preserved verbatim (`|&a|`).
        let param = if deref {
            src(self.source, &closure.inputs[0])?
        } else {
            a_s
        };
        let key_method = if unstable {
            "sort_unstable_by_key"
        } else {
            "sort_by_key"
        };
        Some(format!("{recv_src}.{key_method}(|{param}| {recv_text})"))
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
        // P8 oracle: `|a: &u8, b: &u8| a.cmp(b)` → `sort()` (MA)
        let input = "fn f(v: &mut Vec<u8>) { v.sort_by(|a: &u8, b: &u8| a.cmp(b)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort()"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_key_plain_fires() {
        // P8b oracle: `|a, b| a.k.cmp(&b.k)` → `sort_by_key(|a| a.k)` (MA)
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(
            out.contains("v.sort_by_key(|a| a.k)"),
            "field key must produce sort_by_key: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_key_deref_fires_verbatim_pattern() {
        // P8b oracle: `|&a, &b| a.k.cmp(&b.k)` → `sort_by_key(|&a| a.k)` (MA)
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|&a, &b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|&a| a.k)"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_key_unstable_fires() {
        // P8b oracle: `sort_unstable_by(|a, b| a.k.cmp(&b.k))` →
        // `sort_unstable_by_key(|a| a.k)` (MA)
        let input = "fn f(v: &mut Vec<K>) { v.sort_unstable_by(|a, b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_unstable_by_key(|a| a.k)"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_key_typed_drops_annotation() {
        // P8b oracle: `|a: &K, b: &K| a.k.cmp(&b.k)` → `sort_by_key(|a| a.k)`
        // — the annotation is DROPPED from the suggested closure (MA).
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a: &K, b: &K| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| a.k)"), "got: {out}");
        assert!(!out.contains("&K|"), "annotation must be dropped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn multilevel_field_key_fires() {
        // P8b oracle: `a.n.c.cmp(&b.n.c)` → `sort_by_key(|a| a.n.c)` (MA)
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.n.c.cmp(&b.n.c)); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| a.n.c)"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_call_key_fires() {
        // P8b oracle: `a.n.m().cmp(&b.n.m())` → `sort_by_key(|a| a.n.m())` (MA)
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.n.m().cmp(&b.n.m())); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| a.n.m())"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn postfix_method_key_fires() {
        // P8b oracle CORRECTION: the old `custom_comparator_unchanged` test
        // asserted `a.abs().cmp(&b.abs())` is silent — the probe showed MA
        // `sort_by_key(|a| a.abs())`. The skip encoded a false oracle fact.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.abs().cmp(&b.k.abs())); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| a.k.abs())"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn tuple_key_fires_parens_kept() {
        // P8b oracle: `(a.k, a.j).cmp(&(b.k, b.j))` →
        // `sort_by_key(|a| (a.k, a.j))` — tuple parens KEPT (MA).
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| (a.k, a.j).cmp(&(b.k, b.j))); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| (a.k, a.j))"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn shared_const_key_fires() {
        // P8b oracle: `a.k.max(MAX).cmp(&b.k.max(MAX))` →
        // `sort_by_key(|a| a.k.max(MAX))` (MA) — shared constants swap-clean.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.max(MAX).cmp(&b.k.max(MAX))); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| a.k.max(MAX))"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn param_arg_key_fires() {
        // P8b oracle: `a.k.wrapping_add(a.j).cmp(&b.k.wrapping_add(b.j))` →
        // `sort_by_key(|a| a.k.wrapping_add(a.j))` (MA) — param-referencing
        // method args inside the key swap cleanly.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.wrapping_add(a.j).cmp(&b.k.wrapping_add(b.j))); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(
            out.contains("v.sort_by_key(|a| a.k.wrapping_add(a.j))"),
            "got: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn external_ident_key_fires() {
        // P8b oracle: `a.k.max(w.len() as i32)` swaps clean → MA. Also pins
        // the word-boundary scan: `a` inside `as` is NOT a word occurrence.
        let input =
            "fn f(v: &mut Vec<K>, w: &Vec<K>) { v.sort_by(|a, b| a.k.max(w.len() as i32).cmp(&b.k.max(w.len() as i32))); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(
            out.contains("v.sort_by_key(|a| a.k.max(w.len() as i32))"),
            "got: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn paren_grouped_key_skipped() {
        // P8b oracle fact: `(a.k + 1)` FIRES but the oracle de-parens the
        // rendered key (`|a| a.k + 1`) — HIR rendering our byte-parity
        // standard cannot reproduce. Deliberate skip.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| (a.k + 1).cmp(&(b.k + 1))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "paren-grouped key must not fire: {out:?}");
    }

    #[test]
    fn mixed_bindings_key_silent() {
        // P8b oracle: `|&a, b| a.k.cmp(&b.k)` is SILENT (key forms included).
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|&a, b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "mixed bindings must not fire: {out:?}");
    }

    #[test]
    fn mixed_fields_silent() {
        // P8b oracle: `a.k.cmp(&b.j)` is SILENT — not swap-identical.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.cmp(&b.j)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "mixed fields must not fire: {out:?}");
    }

    #[test]
    fn asymmetric_args_silent() {
        // P8b oracle: `a.k.max(1).cmp(&b.k.max(2))` is SILENT.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.max(1).cmp(&b.k.max(2))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "asymmetric args must not fire: {out:?}");
    }

    #[test]
    fn different_methods_silent() {
        // P8b oracle: `a.k.max(0).cmp(&b.k.min(0))` is SILENT.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.max(0).cmp(&b.k.min(0))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "different methods must not fire: {out:?}");
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
    fn reversed_field_key_not_fired() {
        // P8b oracle: reversed field key gets MaybeIncorrect
        // `sort_by_key(|b| Reverse(b.k))` — skipped (no-autofire).
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| b.k.cmp(&a.k)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "reversed field key must not fire: {out:?}");
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
    fn plain_ref_arg_impossible_shape_not_fired() {
        // P8b oracle fact: `|a, b| a.cmp(&b)` with plain bindings is `&&T`
        // — E0599, cannot compile. The bare/bare exclusion keeps us off it.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.cmp(&b)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "plain ref-arg bare form must not fire: {out:?}");
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
}
