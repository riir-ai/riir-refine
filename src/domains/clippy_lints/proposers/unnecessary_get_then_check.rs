//! `clippy::unnecessary_get_then_check` — `map.get(k).is_some()` →
//! `map.contains_key(k)` and `map.get(k).is_none()` → `!map.contains_key(k)`
//! (P51a, 2026-10-14; complexity group per the pinned 1.98.1 driver — probe
//! re-verified against the driver's own suggestion shape, `MaybeIncorrect`).
//!
//! ## The rewrite
//!
//! The outer `is_some`/`is_none` (zero args, no turbofish) on a receiver
//! that is an inner `get` call (EXACTLY one argument, no turbofish):
//! `get(<args>).is_some()` is replaced by `contains_key(<args>)` — the
//! receiver and the argument bytes survive VERBATIM (both methods take the
//! key by the same borrowed form; `get` returns `Option<&V>` that the check
//! then discards, `contains_key` does the one lookup the check needs).
//! `is_none` adds a leading `!` on the RECEIVER span (clippy's own splice
//! covers caller-through-call; `!` binds tighter than every operator that
//! can legally surround a `bool` operand, so no parens are ever needed).
//!
//! ## Why it is safe when it compiles (the VERIFY bar)
//!
//! Both calls are `&self` methods with the SAME key parameter class
//! (`Q: Borrow<K>` for the std maps) — for `HashMap`/`BTreeMap` (and
//! anything forwarding to them) the rewrite is lookup-identical. The
//! parse-blind mis-fire classes are all E0599-visible: slices/`Vec`
//! (`v.get(i).is_none()` — no `contains_key`), `HashSet` (no
//! `contains_key`) are compile errors the `--verify` gate reverts.
//!
//! ## Documented divergences
//!
//! - **Safer than clippy**: the real lint type-checks the caller to the
//!   four std map/set types and stays SILENT on a `&map` receiver (the
//!   expr type is `&HashMap`, not `HashMap` — probe-verified 2026-10-14);
//!   this proposer fires on the syntactic shape, so `&map`/`&mut map`
//!   receivers (identically valid rewrites — `contains_key` auto-derefs)
//!   are healed too.
//! - **Map-form only**: a `HashSet` receiver (`set.get(k).is_some()` →
//!   `set.contains(k)` upstream) cannot be distinguished parse-time from
//!   the map form; this proposer always emits `contains_key` — on a set
//!   receiver that is an E0599 the compile gate reverts (the set shape
//!   rides upstream clippy only).
//! - A custom type whose `get` and `contains_key` DISAGREE semantically
//!   would compile both ways — the pathological class, disclosed here;
//!   every std shape is lookup-identical.
//! - `get` with a turbofish (`m.get::<K>(k)`) refuses — upstream's own
//!   splice would preserve it; the shape is vanishingly rare and the
//!   re-splice cost is not paid.
//!
//! Interior-comment guard: the receiver and the single argument survive
//! verbatim; a comment anywhere else in the replaced span refuses.

use syn::visit::Visit;
use syn::{Expr, File};

use super::helpers::{byte_range, drops_interior_comment};
use super::{edit, src, SourceEdit};

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
    fn visit_expr(&mut self, e: &'ast Expr) {
        syn::visit::visit_expr(self, e);
        if let Some(se) = self.try_check_shape(e) {
            self.edits.push(se);
        }
    }
}

impl Collector<'_> {
    /// `<recv>.get(<arg>).is_some()/is_none()` → the `contains_key` form.
    fn try_check_shape(&self, outer: &Expr) -> Option<SourceEdit> {
        let Expr::MethodCall(out) = outer else {
            return None;
        };
        if !out.attrs.is_empty() {
            return None;
        }
        let negated = match (out.method.to_string().as_str(), out.args.is_empty()) {
            ("is_some", true) => false,
            ("is_none", true) => true,
            _ => return None,
        };
        if out.turbofish.is_some() {
            return None;
        }
        let Expr::MethodCall(inner) = &*out.receiver else {
            return None;
        };
        if inner.method != "get" || inner.args.len() != 1 || inner.turbofish.is_some() {
            return None;
        }
        if !inner.attrs.is_empty() {
            return None;
        }
        let arg = inner.args.first()?;
        // Kept verbatim: the get-receiver and the single key argument.
        let kept = [byte_range(&*inner.receiver), byte_range(arg)];
        if drops_interior_comment(self.source, byte_range(outer), &kept) {
            return None;
        }
        let recv = src(self.source, &*inner.receiver)?;
        let arg_text = src(self.source, arg)?;
        let neg = if negated { "!" } else { "" };
        Some(edit(outer, format!("{neg}{recv}.contains_key({arg_text})")))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn get_is_none_becomes_negated_contains_key() {
        // The P51a production specimen shape (lint_level/tests.rs, cycle 71):
        // `driver.get(lint).is_none()` → `!driver.contains_key(lint)`.
        let input =
            "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get(&k).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(
            out.contains("!m.contains_key(&k)"),
            "is_none must negate contains_key: {out}"
        );
        assert!(!out.contains("get"), "get must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn get_is_some_becomes_contains_key() {
        let input =
            "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get(&k).is_some() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(
            out.contains("m.contains_key(&k)") && !out.contains('!'),
            "is_some keeps the polarity: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_survives_verbatim() {
        let input = "fn f(s: &S) -> bool { s.map.get(s.key()).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(
            out.contains("!s.map.contains_key(s.key())"),
            "receiver and arg bytes survive verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn turbofish_get_refused() {
        let input =
            "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get::<u8>(&k).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input);
        assert!(out.is_none(), "turbofish get refuses: {out:?}");
    }

    #[test]
    fn zero_or_multi_arg_get_refused() {
        let noarg = "fn f(m: M) -> bool { m.get().is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", noarg).is_none());
        let outer_args = "fn f(m: M, k: u8, j: u8) -> bool { m.get(&k, &j).is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", outer_args).is_none());
    }

    #[test]
    fn wrong_outer_method_refused() {
        let input =
            "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> Option<&u8> { m.get(&k).copied() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn interior_comment_refused() {
        let input =
            "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get( /* key */ &k).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input);
        assert!(
            out.is_none(),
            "comment that the splice would drop refuses: {out:?}"
        );
    }
}
