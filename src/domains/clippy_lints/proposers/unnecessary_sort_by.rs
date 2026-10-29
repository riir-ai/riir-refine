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
//! - free-fn cmp form `Ord::cmp(&a, &b)` (oracle-silent)
//! - borrow-in-key: a BARE reference into the binding as a direct tuple/
//!   array element (`(a.0, &a.1)` — the live riir-chain persist.rs shape;
//!   probe 1.97: oracle-SILENT for the non-Copy tuple, the Copy twin, the
//!   array twin, AND the whole-binding element `(a.0, a)`) — the emitted
//!   `sort_by_key` key would hold the borrow itself and cannot compile
//!   (`FnMut(&T) -> K` returns K by value; a K tied to the input has no
//!   name). Consumed positions stay MA and keep firing: call args
//!   (`combine(&a.k)`), method args (`a.dist2(&ORIGIN)`), binary operands
//!   (`a.1 + &a.2`) — the callee/impl consumes the reference and the key
//!   value stays owned. Guard = `key_holds_borrow` (value-position walk).
//! - FIELD keys (2cb97410 escape class, 2026-08-31): any key that reads a
//!   field (`a.k`, `a.n.c`, `a.n.m()`, `(a.k, a.j)`, `a.k.max(MAX)`) is
//!   refused — `sort_by_key` returns K by VALUE, so the emitted key moves
//!   the field out of `&T`, which E0507s for every non-Copy field type
//!   (the katgpt-rs datrie.rs break). Real clippy is type-aware here (MA
//!   on Copy fields, SILENT on non-Copy); parse-only cannot prove Copy, so
//!   the whole subfamily is refused (healer-safer-than-clippy). Keys over
//!   the WHOLE binding (`a.abs()`, `a.max(MAX)` on `a` itself) keep firing.
//! - reference-RETURNING method keys (`a.kref()` — clippy is type-aware
//!   silent; parse-only cannot see return types — the `--verify` compile
//!   gate is the backstop if that shape ever appears)
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

/// True when a BARE reference into `binding` sits as a direct tuple/array
/// element of the key — the emitted key's VALUE would hold the borrow
/// itself (`(a.0, &a.1)`, the live riir-chain persist.rs shape) and the
/// `sort_by_key` suggestion cannot compile (`FnMut(&T) -> K` returns K by
/// value; a K holding a reference tied to the input has no name).
///
/// Probe-pinned (clippy 1.97): the bare-element class is oracle-SILENT for
/// the non-Copy tuple, the Copy twin, the array twin, AND the whole-binding
/// element `(a.0, a)` (same borrow, no `&` token — the Path arm, PLAIN
/// bindings only: under a deref pattern the binding is OWNED and the key
/// `|&a| a` is the probe-verified MA form). Consumed positions are
/// oracle-MA and keep firing: call args (`combine(&a.k)`), method args
/// (`a.dist2(&ORIGIN)`), binary operands (`a.1 + &a.2`) — the callee/impl
/// consumes the reference and the key value stays owned. The nested
/// tuple/array/paren recursion is a conservative structural extension of
/// the pinned flat class (false-negative-safe).
/// True when the expression tree contains ANY field access (`a.k`, `a.n.c`,
/// `a.0`) — the `sort_by_key` move-out risk class (see the guard site).
fn key_mentions_field(e: &Expr) -> bool {
    match e {
        Expr::Field(_) => true,
        Expr::Tuple(t) => t.elems.iter().any(key_mentions_field),
        Expr::Array(a) => a.elems.iter().any(key_mentions_field),
        Expr::Paren(p) => key_mentions_field(&p.expr),
        Expr::Group(g) => key_mentions_field(&g.expr),
        Expr::Binary(b) => key_mentions_field(&b.left) || key_mentions_field(&b.right),
        Expr::MethodCall(mc) => key_mentions_field(&mc.receiver),
        _ => false,
    }
}

fn key_holds_borrow(source: &str, e: &Expr, binding: &str, deref: bool) -> bool {
    match e {
        Expr::Tuple(t) => t
            .elems
            .iter()
            .any(|x| key_holds_borrow(source, x, binding, deref)),
        Expr::Array(a) => a
            .elems
            .iter()
            .any(|x| key_holds_borrow(source, x, binding, deref)),
        Expr::Paren(p) => key_holds_borrow(source, &p.expr, binding, deref),
        Expr::Group(g) => key_holds_borrow(source, &g.expr, binding, deref),
        Expr::Reference(r) => src(source, &r.expr).is_some_and(|t| contains_word(&t, binding)),
        // `(a.0, a)` — the whole binding as an element: the key copies the
        // reference VALUE into K (probe: oracle-silent, same class). Deref
        // bindings own their binding — the bare ident is the MA key form.
        Expr::Path(p) => !deref && p.path.get_ident().is_some_and(|id| id == binding),
        _ => false,
    }
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
        if closure.asyncness.is_some()
            || closure.constness.is_some()
            || closure.movability.is_some()
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
        let recv_src = src(self.source, &m.receiver)?;
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
        let recv_text = src(self.source, &cmp.receiver)?;
        let inner_text = src(self.source, &argref.expr)?;
        let a_s = pa.to_string();
        let b_s = pb.to_string();
        // Borrow-in-key: a bare reference into the binding as a tuple/array
        // element makes the emitted key hold the borrow — the suggestion
        // cannot compile and the oracle is silent on the whole class.
        if key_holds_borrow(self.source, &cmp.receiver, &a_s, deref) {
            return None;
        }
        // Field-access keys (`a.k`, `a.n.c`, tuple elements `(a.k, a.j)`):
        // `sort_by_key` returns K BY VALUE, so the emitted key MOVES the field
        // out of `&T` — E0507 for every non-Copy field type (the katgpt-rs
        // datrie.rs escape: `sort_by(|a, b| a.0.cmp(&b.0))` over `Vec<u8>`
        // keys broke the build). The oracle is type-aware: MA on Copy fields,
        // SILENT on non-Copy — parse-only cannot separate them, so the whole
        // field subfamily is refused (the healer-safer-than-clippy class;
        // keys over the WHOLE binding — method calls like `a.abs()`, shared
        // constants — keep firing: no field is moved).
        if key_mentions_field(&cmp.receiver) {
            return None;
        }
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
    fn field_key_plain_refused_non_copy_move_risk() {
        // E0507 probe + the katgpt-rs datrie.rs break: `sort_by_key(|a| a.k)`
        // MOVES `a.k` out of `&K` — compiles only for Copy fields, and
        // parse-only cannot prove Copy (real clippy is type-aware: MA on
        // Copy, SILENT on non-Copy). The whole field subfamily is refused.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "field key must be refused: {out:?}");
    }

    #[test]
    fn field_key_deref_refused_same_move_risk() {
        // The deref twin moves the field identically (`|&a| a.k` still owns
        // `a` after the pattern deref) — refused for the same reason.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|&a, &b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "deref field key must be refused: {out:?}");
    }

    #[test]
    fn field_key_unstable_refused() {
        let input = "fn f(v: &mut Vec<K>) { v.sort_unstable_by(|a, b| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "unstable field key must be refused: {out:?}");
    }

    #[test]
    fn field_key_typed_refused() {
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a: &K, b: &K| a.k.cmp(&b.k)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "typed field key must be refused: {out:?}");
    }

    #[test]
    fn multilevel_field_key_refused() {
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.n.c.cmp(&b.n.c)); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "multilevel field key must be refused: {out:?}"
        );
    }

    #[test]
    fn method_call_on_field_key_refused() {
        // `a.n.m()` reads the field `a.n` before calling — same move-out
        // risk at the field read, refused with the family.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.n.m().cmp(&b.n.m())); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "field method-call key must be refused: {out:?}"
        );
    }

    #[test]
    fn postfix_method_on_field_key_refused() {
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.abs().cmp(&b.k.abs())); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "postfix field key must be refused: {out:?}");
    }

    #[test]
    fn tuple_field_key_refused() {
        // `(a.k, a.j)` moves both fields — refused with the family.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| (a.k, a.j).cmp(&(b.k, b.j))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "tuple field key must be refused: {out:?}");
    }

    #[test]
    fn borrow_tuple_key_not_fired() {
        // Borrow-in-key probe (clippy 1.97): `(a.0, &a.1)` — the key VALUE
        // holds the borrow, oracle-SILENT; our emitted `sort_by_key` could
        // not compile (K tied to the closure input). The live riir-chain
        // persist.rs shape — guarded by `key_holds_borrow`.
        let input =
            "fn f(v: &mut Vec<(u8, Vec<u8>)>) { v.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "borrow tuple key must not fire: {out:?}");
    }

    #[test]
    fn borrow_tuple_key_copy_twin_not_fired() {
        // Probe: the Copy twin is oracle-silent too — the ref element is the
        // trigger, not the non-Copy payload.
        let input =
            "fn f(v: &mut Vec<(u8, u8)>) { v.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "copy-twin borrow key must not fire: {out:?}");
    }

    #[test]
    fn borrow_array_key_not_fired() {
        // Probe: bare-reference ARRAY elements are oracle-silent — same
        // value-position class as the tuple.
        let input =
            "fn f(v: &mut Vec<[u8; 2]>) { v.sort_by(|a, b| [&a[0], &a[1]].cmp(&[&b[0], &b[1]])); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(out.is_none(), "borrow array key must not fire: {out:?}");
    }

    #[test]
    fn whole_binding_tuple_element_not_fired() {
        // Probe: `(a.0, a)` — the whole binding as an element — is
        // oracle-silent: the key copies the reference VALUE into K (no `&`
        // token, same borrow). The Path arm of the guard.
        let input =
            "fn f(v: &mut Vec<(u8, Vec<u8>)>) { v.sort_by(|a, b| (a.0, a).cmp(&(b.0, b))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "whole-binding element must not fire: {out:?}"
        );
    }

    #[test]
    fn consumed_ref_arg_key_still_fires() {
        // Probe: a reference as a CALL argument is oracle-MA — the callee
        // consumes it and the key value stays owned. Must keep firing.
        let input = "fn g(s: &u8) -> u8 { s.wrapping_add(1) }\n\
            fn f(v: &mut Vec<K>) { v.sort_by(|a, b| g(&a.k).cmp(&g(&b.k))); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| g(&a.k))"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_binding_ref_method_arg_key_still_fires() {
        // Probe: a reference to a NON-binding is oracle-MA — no borrow into
        // the closure param exists. Must keep firing (the guard is the AST
        // value-position walk, not the `&` token).
        let input = "fn f(v: &mut Vec<P>) { v.sort_by(|a, b| a.dist(&O).cmp(&b.dist(&O))); }";
        let out = propose_fix("unnecessary_sort_by", input).expect("matched");
        assert!(out.contains("v.sort_by_key(|a| a.dist(&O))"), "got: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_binary_operand_key_refused() {
        // `(a.0, a.1 + &a.2)` — field reads at value position, refused with
        // the family (the E0507 move risk exists at every field read).
        let input = "fn f(v: &mut Vec<(u8, u8, u8)>) { v.sort_by(|a, b| (a.0, a.1 + &a.2).cmp(&(b.0, b.1 + &b.2))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "field binary-operand key must be refused: {out:?}"
        );
    }

    #[test]
    fn field_method_call_with_const_arg_refused() {
        // `a.k.max(MAX)` reads the field `a.k` — refused with the family.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.max(MAX).cmp(&b.k.max(MAX))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "field-reading method key must be refused: {out:?}"
        );
    }

    #[test]
    fn field_method_param_arg_refused() {
        // `a.k.wrapping_add(a.j)` — field reads, refused.
        let input = "fn f(v: &mut Vec<K>) { v.sort_by(|a, b| a.k.wrapping_add(a.j).cmp(&b.k.wrapping_add(b.j))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "field method-arg key must be refused: {out:?}"
        );
    }

    #[test]
    fn field_method_external_ident_refused() {
        // `a.k.max(w.len() as i32)` — field read, refused (the external-ident
        // swap check itself still stands; the field guard fires first).
        let input = "fn f(v: &mut Vec<K>, w: &Vec<K>) { v.sort_by(|a, b| a.k.max(w.len() as i32).cmp(&b.k.max(w.len() as i32))); }";
        let out = propose_fix("unnecessary_sort_by", input);
        assert!(
            out.is_none(),
            "external-ident field key must be refused: {out:?}"
        );
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
        assert!(
            out.is_none(),
            "plain ref-arg bare form must not fire: {out:?}"
        );
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
