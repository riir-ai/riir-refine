//! `clippy::to_string_in_format_args` — `format!("n={}", x.to_string())` →
//! `format!("n={}", x)` on SIGNATURE-PROVEN scalar receivers
//! (Batch 158 / Plan 149 — the frontier-lead mining pack IV; family fixture
//! verified to fire exactly this lint under all+pedantic+nursery AND under
//! default groups — a `clippy::perf` lint, 2026-09-18).
//!
//! ## Why the strip is identity (the FIXABLE bar)
//!
//! `{x.to_string()}` ≡ `{x}` exactly when `x`'s `to_string` is the std
//! Display BLANKET (`impl<T: Display> ToString for T`) — then both legs
//! format the same Display output, and the rewrite only deletes a
//! temporary allocation (the point of the perf lint). Parse-only cannot
//! resolve the impl, so the receiver is proven by SIGNATURE DERIVATION
//! (the `slice_param_heads_over` pattern): a single-segment ident that is
//! a fn/impl-method param with an EXPLICIT scalar annotation — primitive
//! (`u8`/`i32`/`f64`/`bool`/`char`/…), `&str`, `String`, `&String` — and
//! never rebound in the body. For every such type the only in-scope
//! `to_string` IS the blanket: primitives/`str`/`String` have no inherent
//! `to_string`, and a second in-scope trait method named `to_string` for
//! the same receiver would make the ORIGINAL not compile (E0031
//! ambiguity) — so every compiling input is safe by construction.
//!
//! The LITERAL arm (`"lit".to_string()`) is construction-identical without
//! any derivation: a `str` receiver's `to_string` is always the blanket
//! (inherent impls on `str` live only in std).
//!
//! ## The placeholder guard
//!
//! The strip is only correct where the arg fills a BARE `{}` (implicit
//! Display). A spec placeholder changes the trait: `{:?}` formats Debug,
//! and `Debug for String` ≠ `Debug for u32` — so an arg consumed by a
//! `{:spec}` placeholder REFUSES. The template walk reuses
//! `uninlined_format_args`' family table + placeholder classification;
//! dynamic-spec refs (`{:1$}` / `{:.*}` — placeholders that consume extra
//! positionals) abort the whole macro, keeping the arg↔placeholder mapping
//! exact.
//!
//! ## Composition with `uninlined_format_args` (by design)
//!
//! Our `uninlined_format_args` proposer skips method-call args (real
//! clippy's own envelope — probe-verified); this rule fires exactly on
//! that skipped shape, and its plain-ident output is what uninlined
//! inlines on the NEXT fixpoint iteration:
//! `format!("n={}", x.to_string())` → `format!("n={}", x)` →
//! `format!("n={x}")`. One anti-pattern per proposer; the fixpoint
//! composes them (never a ping-pong: the inlined form has no args left
//! for either rule to touch).
//!
//! Guards: named/assign-shaped args abort the macro (the positional
//! mapping must be exact); attrs/turbofish/args on the `to_string` call
//! refuse; receivers that are paths-but-unproven (locals, fields,
//! multi-segment paths) refuse — the documented narrower-than-clippy
//! divergence (clippy type-resolves; parse-only derives from the
//! signature). Direct post-template family-macro children are skipped
//! (the `format_in_format_args` boundary, the uninlined precedent);
//! deeper nested family macros are visited standalone.

use std::collections::BTreeMap;

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, ImplItemFn, ItemFn, Macro};

use super::SourceEdit;
use super::helpers::{byte_range, scalar_param_heads_over};
use super::uninlined_format_args::{
    Placeholder, classify_placeholder, has_dynamic_spec_ref, template_and_post_template_args,
};

/// The scalar annotations whose Display-blanket `to_string` is structural
/// (`scalar_param_heads_over` yields `&str`/`&String` for the ref forms).
const BLANKET_SCALARS: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64", "bool", "char", "&str", "String", "&String",
];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeMap::new(),
        saved: BTreeMap::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Signature-proven scalar bindings of the enclosing fn (type text).
    proven: BTreeMap<String, String>,
    /// The set to restore on fn exit (one-deep save).
    saved: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = scalar_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = scalar_param_heads_over(f.sig.inputs.iter(), &f.block);
        self.saved = saved;
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = std::mem::take(&mut self.saved);
    }

    fn visit_expr_macro(&mut self, m: &'ast syn::ExprMacro) {
        self.try_strip(&m.mac);
        syn::visit::visit_expr_macro(self, m);
    }

    fn visit_stmt_macro(&mut self, m: &'ast syn::StmtMacro) {
        self.try_strip(&m.mac);
        syn::visit::visit_stmt_macro(self, m);
    }
}

impl Collector<'_> {
    fn try_strip(&mut self, mac: &Macro) {
        let Some((raw, args)) = template_and_post_template_args(mac) else {
            return;
        };
        // Every post-template arg must parse as a plain (non-assign)
        // expression — a named `k = v` arg (Assign) or anything unparseable
        // shifts the positional mapping, so the whole macro refuses.
        let mut exprs = Vec::with_capacity(args.len());
        for a in &args {
            let stream: proc_macro2::TokenStream = a.toks.iter().cloned().collect();
            let Ok(expr) = syn::parse2::<Expr>(stream) else {
                return;
            };
            if matches!(expr, Expr::Assign(_)) {
                return;
            }
            exprs.push(expr);
        }
        // Which positional index fills a BARE `{}` placeholder.
        let Some(empty_slots) = implicit_display_slots(&raw, exprs.len()) else {
            return;
        };
        for (k, expr) in exprs.iter().enumerate() {
            if !empty_slots.contains(&k) {
                continue;
            }
            if let Some(replacement) = self.strip_one(expr) {
                let range = byte_range(expr);
                self.edits.push(SourceEdit {
                    start: range.start,
                    end: range.end,
                    replacement,
                });
            }
        }
        // Nested family macros one level down (the format_args boundary:
        // a DIRECT post-template macro child is `format_in_format_args`
        // territory and is skipped; deeper macros inside non-macro args —
        // `String::from(format!(..))` — are standalone and ARE visited —
        // the uninlined descent rule, verbatim).
        for expr in &exprs {
            if matches!(expr, Expr::Macro(_)) {
                continue;
            }
            self.visit_expr(expr);
        }
    }

    /// The replacement text when `expr` is a strippable `to_string` call.
    fn strip_one(&self, expr: &Expr) -> Option<String> {
        let Expr::MethodCall(m) = expr else {
            return None;
        };
        if !is_plain_to_string(m) {
            return None;
        }
        match &*m.receiver {
            // Proven scalar param — the blanket derivation.
            Expr::Path(p) => {
                if p.qself.is_some() || p.path.leading_colon.is_some() {
                    return None;
                }
                if p.path.segments.len() != 1 || !p.path.segments[0].arguments.is_none() {
                    return None;
                }
                let ident = p.path.segments[0].ident.to_string();
                let ty = self.proven.get(&ident)?;
                if !BLANKET_SCALARS.contains(&ty.as_str()) {
                    return None;
                }
                Some(ident)
            }
            // String literal — construction-identical (the str blanket).
            // The receiver's ORIGINAL source text (escapes and raw forms
            // verbatim) is the replacement.
            Expr::Lit(l) if matches!(l.lit, syn::Lit::Str(_)) => {
                let r = byte_range(&m.receiver);
                self.source.get(r).map(str::to_string)
            }
            _ => None,
        }
    }
}

/// `x.to_string()` — plain method call, 0 args, no turbofish, no attrs.
fn is_plain_to_string(m: &ExprMethodCall) -> bool {
    m.method == "to_string" && m.args.is_empty() && m.turbofish.is_none() && m.attrs.is_empty()
}

/// Which positional-arg indices (0-based, post-template order) fill a BARE
/// `{}` placeholder. `None` aborts the macro: a dynamic-spec ref
/// (`{:1$}`/`{:.*}` — extra positional consumption), an explicit index /
/// malformed placeholder, or more placeholders than args.
fn implicit_display_slots(raw: &str, n_args: usize) -> Option<Vec<usize>> {
    let chars: Vec<char> = raw.chars().collect();
    let mut slots = Vec::new();
    let mut next_arg = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1; // skip the escape introducer…
                match chars.get(i) {
                    // `\u{…}` — the braces belong to the escape, not the
                    // template grammar (the build_inlined rule).
                    Some('u') => {
                        i += 1;
                        if chars.get(i) == Some(&'{') {
                            i += 1;
                            while i < chars.len() && chars[i] != '}' {
                                i += 1;
                            }
                            if i >= chars.len() {
                                return None; // malformed — not a compiling template
                            }
                            i += 1; // past the `}`
                        }
                    }
                    Some(_) => i += 1, // any other escape payload — 1 char
                    None => return None,
                }
            }
            '{' if chars.get(i + 1) == Some(&'{') => i += 2,
            '}' if chars.get(i + 1) == Some(&'}') => i += 2,
            '{' => {
                let mut content = String::new();
                i += 1;
                loop {
                    match chars.get(i) {
                        Some('}') => {
                            i += 1;
                            break;
                        }
                        Some(&c) => {
                            content.push(c);
                            i += 1;
                        }
                        None => return None, // unterminated — not a compiling template
                    }
                }
                match classify_placeholder(&content) {
                    Placeholder::Empty => {
                        if next_arg >= n_args {
                            return None; // more placeholders than args — defensive
                        }
                        slots.push(next_arg);
                        next_arg += 1;
                    }
                    Placeholder::Spec(spec) => {
                        if has_dynamic_spec_ref(spec) {
                            return None; // the spec consumes extra positionals — mapping unsafe
                        }
                        if next_arg >= n_args {
                            return None;
                        }
                        next_arg += 1;
                    }
                    Placeholder::AlreadyNamed => {}
                    Placeholder::NotInlinable => return None,
                }
            }
            '}' => return None, // lone `}` — invalid in a format string
            _ => i += 1,
        }
    }
    Some(slots)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn proven_param_stripped() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { format!(\"n={}\", x.to_string()) }",
        )
        .expect("matched");
        assert!(out.contains("format!(\"n={}\", x)"), "not stripped: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn proven_str_param_stripped() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn greet(name: &str) -> String { format!(\"hi {}\", name.to_string()) }",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"hi {}\", name)"),
            "not stripped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn proven_string_param_stripped() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn greet(name: String) -> String { format!(\"hi {}\", name.to_string()) }",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"hi {}\", name)"),
            "not stripped: {out}"
        );
    }

    #[test]
    fn string_literal_receiver_stripped() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show() -> String { format!(\"n={}\", \"lit\".to_string()) }",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"n={}\", \"lit\")"),
            "literal not stripped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn second_placeholder_position_mapped() {
        // The FIRST arg fills the spec slot (refused), the SECOND fills the
        // bare `{}` — only the second strips.
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(a: u32, b: u32) -> String { format!(\"{:?} {}\", a.to_string(), b.to_string()) }",
        )
        .expect("matched");
        assert!(
            out.contains("a.to_string(), b)"),
            "only b must strip (a fills {{:?}}): {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn spec_placeholder_refused() {
        // The single arg fills a `{:?}` — Debug of String ≠ Debug of u32.
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { format!(\"{:?}\", x.to_string()) }",
        );
        assert!(out.is_none(), "spec placeholder must refuse: {out:?}");
    }

    #[test]
    fn unproven_receiver_refused() {
        // Local binding (no signature derivation) — clippy would fire
        // (type-aware), we refuse: the documented narrower divergence.
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show() -> String { let x = 1u32; format!(\"n={}\", x.to_string()) }",
        );
        assert!(out.is_none(), "unproven receiver must refuse: {out:?}");
    }

    #[test]
    fn non_scalar_param_refused() {
        // Param is a custom-ish type not in the provable set.
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: Vec<u8>) -> String { format!(\"n={}\", x.to_string()) }",
        );
        assert!(out.is_none(), "non-scalar param must refuse: {out:?}");
    }

    #[test]
    fn field_receiver_refused() {
        let out = propose_fix(
            "to_string_in_format_args",
            "struct S { n: u32 }\nfn show(s: S) -> String { format!(\"n={}\", s.n.to_string()) }",
        );
        assert!(out.is_none(), "field receiver must refuse: {out:?}");
    }

    #[test]
    fn turbofished_call_refused() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { format!(\"n={}\", x.to_string::<()>()) }",
        );
        assert!(out.is_none(), "turbofish must refuse: {out:?}");
    }

    #[test]
    fn named_arg_aborts_the_macro() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { format!(\"{a}\", a = x, x = x.to_string()) }",
        );
        assert!(out.is_none(), "named args must abort: {out:?}");
    }

    #[test]
    fn println_and_write_families_fire() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: i64) { println!(\"v={}\", x.to_string()); }",
        )
        .expect("matched");
        assert!(
            out.contains("println!(\"v={}\", x)"),
            "println not stripped: {out}"
        );
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(f: &mut std::fmt::Formatter, x: u8) -> std::fmt::Result { writeln!(f, \"v={}\", x.to_string()) }",
        )
        .expect("matched");
        assert!(
            out.contains("writeln!(f, \"v={}\", x)"),
            "writeln not stripped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_derivation() {
        let out = propose_fix(
            "to_string_in_format_args",
            "struct P;\nimpl P {\n    fn show(&self, x: u16) -> String { format!(\"n={}\", x.to_string()) }\n}",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"n={}\", x)"),
            "impl-method param not stripped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rebound_param_refused() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { let x = 1u32; format!(\"n={}\", x.to_string()) }",
        );
        assert!(out.is_none(), "rebound param must refuse: {out:?}");
    }

    #[test]
    fn escaped_braces_in_template_walked() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { format!(\"{{}} {}\", x.to_string()) }",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"{{}} {}\", x)"),
            "escaped braces broke the walk: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn dynamic_spec_ref_aborts() {
        // `{:1$}` — the spec itself consumes a positional arg.
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32, w: usize) -> String { format!(\"{:1$}\", x.to_string(), w) }",
        );
        assert!(out.is_none(), "dynamic spec ref must abort: {out:?}");
    }

    #[test]
    fn nested_deeper_format_visited() {
        // A family macro nested inside a CALL arg (not a direct arg) is a
        // standalone format! — visited (the uninlined descent rule).
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> String { String::from(format!(\"n={}\", x.to_string())) }",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"n={}\", x)"),
            "nested format not visited: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_format_macro_untouched() {
        let out = propose_fix(
            "to_string_in_format_args",
            "fn show(x: u32) -> Vec<u8> { vec![x.to_string().len() as u8] }",
        );
        assert!(out.is_none(), "non-format context must not fire: {out:?}");
    }
}
