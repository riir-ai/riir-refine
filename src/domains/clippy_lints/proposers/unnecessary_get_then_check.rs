//! `clippy::unnecessary_get_then_check` — `map.get(k).is_some()` →
//! `map.contains_key(k)` and `map.get(k).is_none()` → `!map.contains_key(k)`
//! (P51a, 2026-10-14; complexity group per the pinned 1.98.1 driver — probe
//! re-verified against the driver's own suggestion shape, `MaybeIncorrect`).
//!
//! ## The rewrite (form picked by parse-time type evidence, issue 235 R1)
//!
//! The outer `is_some`/`is_none` (zero args, no turbofish) on a receiver
//! that is an inner `get` call (EXACTLY one argument, no turbofish). The
//! wrong-heal lesson (riir-rethink `decstat` casualty, 2026-11-11): a
//! `Vec::get(i).is_none()` bounds check is NOT a map membership check, and
//! the `contains_key` splice on it is an E0599 that shipped through a
//! gate-posture hole. The collector therefore classifies the receiver
//! BEFORE picking the output form:
//!
//! - **Map-like** (the receiver's declared type resolves in-file and its
//!   outermost path segment ends in `Map` — the `HashMap`/`BTreeMap`/
//!   `IndexMap` families and custom forwarders): `get(<args>).is_some()`
//!   is replaced by `contains_key(<args>)` — the receiver and the argument
//!   bytes survive VERBATIM. `is_none` adds a leading `!` on the receiver
//!   span (`!` binds tighter than every operator that can legally surround
//!   a `bool` operand, so no parens are ever needed).
//! - **Index-like** (`Vec`, `VecDeque`, slice refs, arrays — the type
//!   names the index family): `get(i)` is a BOUNDS CHECK, healed as the
//!   comparison — `arg >= recv.len()` for `is_none`, `arg < recv.len()`
//!   for `is_some` (equivalent by construction: the family's `get` returns
//!   `None` iff the index is past the end). The argument is parenthesized
//!   when it does not bind tighter than a comparison; a borrowed argument
//!   (`v.get(&k)`) cannot be a slice get, so the site declines. `String`/
//!   `&str` are deliberately NOT index-like — `str::get` is also a
//!   char-boundary check, so the comparison form is not equivalent there.
//! - **Unknown** (no parse-time evidence: untyped bindings, cross-file
//!   types, method-call and deep-field receivers, closure params): the
//!   site DECLINES — manual repair. The `--verify` compile gate remains
//!   the backstop only for the two provable classes; the decstat casualty
//!   shipped because the gate compiled a posture the gated file did not
//!   join, so a blind splice is no longer emitted at all.
//!
//! ## The type evidence (the bounded census, issue 235 R1)
//!
//! One map of in-file `struct` fields, one table of type aliases, and a
//! per-fn scope stack (fn params, typed `let` bindings, consts/statics at
//! every nesting level, the enclosing impl's `Self` struct). The receiver
//! must be a single-ident path, one field access off such a path or `self`
//! (unwrapping paren/deref/reference). Evidence is POSITIVE-ONLY: the
//! rewrite fires only where the type is PROVEN; exact-match names are
//! checked before the suffix (`Vec`/`VecDeque` index-like; `…Map` map-like).
//!
//! ## Why it is safe when it compiles (the VERIFY bar)
//!
//! Map form: both calls are `&self` methods with the SAME key parameter
//! class (`Q: Borrow<K>` for the std maps) — for `HashMap`/`BTreeMap`
//! (and anything forwarding to them) the rewrite is lookup-identical.
//! Index form: the comparison is equivalent to the `Option` check on the
//! whole index family. The residual E0599 class (a `Map`-suffixed type
//! without `contains_key`) stays compile-error-visible for the `--verify`
//! gate to revert.
//!
//! ## Documented divergences
//!
//! - **Narrower than the pre-235 shape-fire, by design**: untyped and
//!   cross-file receivers used to heal blind and lean on the compile gate;
//!   they now decline. A custom type named `…Map` without `contains_key`
//!   still heals and lands on the gate (the suffix policy's residual).
//! - A custom type whose `get` and `contains_key` DISAGREE semantically
//!   would compile both ways — the pathological class, disclosed here;
//!   every std shape is lookup-identical.
//! - `get` with a turbofish (`m.get::<K>(k)`) refuses — upstream's own
//!   splice would preserve it; the shape is vanishingly rare and the
//!   re-splice cost is not paid.
//! - Closure-typed params and `for`-loop patterns are not recorded into
//!   the scope stack (v1 census bounds) — receivers bound there decline.
//!
//! Interior-comment guard: the receiver and the single argument survive
//! verbatim; a comment anywhere else in the replaced span refuses.

use std::collections::HashMap;

use syn::visit::Visit;
use syn::{
    BinOp, Expr, Fields, File, FnArg, GenericArgument, Member, Pat, PathArguments, Type, UnOp,
};

use super::helpers::{byte_range, drops_interior_comment};
use super::{SourceEdit, edit, src};

/// Issue 235 R1: what the parse-time evidence says about the receiver's
/// type — the classification that picks the rewrite's output form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecvKind {
    /// Outermost type is a map-like path (`…Map`) — `contains_key` exists.
    MapLike,
    /// Outermost type names the index family (`Vec`, `VecDeque`, slice,
    /// array) — `.get(i)` is a bounds check, healed as the comparison form.
    IndexLike,
    /// No parse-time evidence — the site declines (manual repair).
    Unknown,
}

/// One scope frame: the fn's (or module's) typed bindings, plus the
/// enclosing impl's `Self` struct name when the frame is a method's.
struct Scope<'ast> {
    bindings: HashMap<String, &'ast Type>,
    self_struct: Option<String>,
}

impl Scope<'_> {
    fn new(self_struct: Option<String>) -> Self {
        Scope {
            bindings: HashMap::new(),
            self_struct,
        }
    }
}

struct Collector<'a, 'ast> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Scope stack; `[0]` is the file base (consts/statics), each fn pushes.
    scopes: Vec<Scope<'ast>>,
    /// In-file struct name → its named fields (name, type).
    structs: HashMap<String, Vec<(String, &'ast Type)>>,
    /// In-file type aliases (`type X = Y;`), resolved one hop.
    aliases: HashMap<String, &'ast Type>,
    /// The impl block currently being visited (`Self`'s struct name).
    current_self: Option<String>,
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        scopes: vec![Scope::new(None)],
        structs: HashMap::new(),
        aliases: HashMap::new(),
        current_self: None,
    };
    v.visit_file(file);
    v.edits
}

/// The final path-segment ident of a type, through one reference.
fn path_tail_ident(ty: &Type) -> Option<String> {
    match ty {
        Type::Reference(r) => path_tail_ident(&r.elem),
        Type::Path(p) if p.qself.is_none() => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// The single generic type argument of `Box<T>`/`Rc<T>`/`Arc<T>` — the
/// smart-pointer unwrap. Multi-arg wrappers (`Cow<'a, T>`) decline.
fn single_type_arg(args: &PathArguments) -> Option<&Type> {
    let syn::PathArguments::AngleBracketed(ab) = args else {
        return None;
    };
    if ab.args.len() != 1 {
        return None;
    }
    match ab.args.first()? {
        GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}

impl<'ast> Collector<'_, 'ast> {
    fn push_fn_scope(&mut self, sig: &'ast syn::Signature, self_struct: Option<String>) {
        let mut bindings = HashMap::with_capacity(sig.inputs.len());
        for input in &sig.inputs {
            if let FnArg::Typed(pt) = input {
                if let Pat::Ident(pi) = &*pt.pat {
                    if pi.subpat.is_none() {
                        bindings.insert(pi.ident.to_string(), pt.ty.as_ref());
                    }
                }
            }
            // FnArg::Receiver: `self` resolves through `self_struct`.
        }
        self.scopes.push(Scope {
            bindings,
            self_struct,
        });
    }

    fn lookup_binding(&self, name: &str) -> Option<&'ast Type> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.bindings.get(name))
            .copied()
    }

    fn lookup_self_struct(&self) -> Option<&String> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.self_struct.as_ref())
    }

    fn classify_type(&self, ty: &Type) -> RecvKind {
        match ty {
            Type::Reference(r) => self.classify_type(&r.elem),
            Type::Slice(_) | Type::Array(_) => RecvKind::IndexLike,
            Type::Path(p) if p.qself.is_none() => {
                let Some(seg) = p.path.segments.last() else {
                    return RecvKind::Unknown;
                };
                let ident = seg.ident.to_string();
                // Exact-match names before the suffix policy.
                if matches!(ident.as_str(), "Vec" | "VecDeque") {
                    return RecvKind::IndexLike;
                }
                if ident.ends_with("Map") {
                    return RecvKind::MapLike;
                }
                if matches!(ident.as_str(), "Box" | "Rc" | "Arc") {
                    return match single_type_arg(&seg.arguments) {
                        Some(inner) => self.classify_type(inner),
                        None => RecvKind::Unknown,
                    };
                }
                if let Some(target) = self.aliases.get(ident.as_str()) {
                    return self.classify_type(target);
                }
                RecvKind::Unknown
            }
            _ => RecvKind::Unknown,
        }
    }

    fn field_kind(&self, struct_name: &str, field: &syn::Ident) -> RecvKind {
        let Some(rows) = self.structs.get(struct_name) else {
            return RecvKind::Unknown;
        };
        let Some((_, ty)) = rows.iter().find(|(n, _)| n == &field.to_string()) else {
            return RecvKind::Unknown;
        };
        self.classify_type(ty)
    }

    /// The receiver's classification from the bounded census. Only the
    /// shapes the evidence can actually reach classify; everything else is
    /// Unknown (decline).
    fn classify_receiver(&self, e: &Expr) -> RecvKind {
        match e {
            Expr::Paren(p) => self.classify_receiver(&p.expr),
            Expr::Reference(r) => self.classify_receiver(&r.expr),
            Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)) => self.classify_receiver(&u.expr),
            Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
                match self.lookup_binding(&p.path.segments[0].ident.to_string()) {
                    Some(ty) => self.classify_type(ty),
                    None => RecvKind::Unknown,
                }
            }
            Expr::Field(f) => {
                let Member::Named(field) = &f.member else {
                    return RecvKind::Unknown;
                };
                // One level off a named binding or `self`; deeper chains
                // (method-call bases, `a.b.c`) have no reachable evidence.
                match &*f.base {
                    Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
                        let name = p.path.segments[0].ident.to_string();
                        if name == "self" {
                            match self.lookup_self_struct() {
                                Some(s) => self.field_kind(s, field),
                                None => RecvKind::Unknown,
                            }
                        } else {
                            match self.lookup_binding(&name) {
                                Some(ty) => match path_tail_ident(ty) {
                                    Some(struct_name) => {
                                        let struct_name =
                                            match self.aliases.get(struct_name.as_str()) {
                                                Some(target) => {
                                                    path_tail_ident(target).unwrap_or(struct_name)
                                                }
                                                None => struct_name,
                                            };
                                        self.field_kind(&struct_name, field)
                                    }
                                    None => RecvKind::Unknown,
                                },
                                None => RecvKind::Unknown,
                            }
                        }
                    }
                    _ => RecvKind::Unknown,
                }
            }
            _ => RecvKind::Unknown,
        }
    }

    /// Is `e` safe as the LEFT operand of a bare comparison
    /// (`e >= recv.len()`)? Arithmetic/bit/shift operators, casts and the
    /// postfix forms all bind tighter than comparisons; logic operators,
    /// control-flow forms and macros do not — parenthesize them.
    fn needs_comparison_operand_parens(e: &Expr) -> bool {
        match e {
            Expr::Binary(b) => !matches!(
                b.op,
                BinOp::Add(_)
                    | BinOp::Sub(_)
                    | BinOp::Mul(_)
                    | BinOp::Div(_)
                    | BinOp::Rem(_)
                    | BinOp::BitAnd(_)
                    | BinOp::BitOr(_)
                    | BinOp::BitXor(_)
                    | BinOp::Shl(_)
                    | BinOp::Shr(_)
            ),
            Expr::Path(_)
            | Expr::Lit(_)
            | Expr::Paren(_)
            | Expr::Cast(_)
            | Expr::Index(_)
            | Expr::Field(_)
            | Expr::MethodCall(_)
            | Expr::Unary(_) => false,
            _ => true,
        }
    }

    /// `<recv>.get(<arg>).is_some()/is_none()` → the classified form.
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
        let kept = [byte_range(&inner.receiver), byte_range(arg)];
        if drops_interior_comment(self.source, byte_range(outer), &kept) {
            return None;
        }
        let recv = src(self.source, &inner.receiver)?;
        let arg_text = src(self.source, arg)?;
        let replacement = match self.classify_receiver(&inner.receiver) {
            RecvKind::MapLike => {
                let neg = if negated { "!" } else { "" };
                format!("{neg}{recv}.contains_key({arg_text})")
            }
            RecvKind::IndexLike => {
                // Slices index by value; a borrowed arg cannot be a slice
                // get — evidence and call disagree, so decline.
                if matches!(arg, Expr::Reference(_)) {
                    return None;
                }
                let arg_expr = if Self::needs_comparison_operand_parens(arg) {
                    format!("({arg_text})")
                } else {
                    arg_text
                };
                if negated {
                    format!("{arg_expr} >= {recv}.len()")
                } else {
                    format!("{arg_expr} < {recv}.len()")
                }
            }
            // Issue 235 R1: no parse-time type evidence — the site is left
            // for manual repair rather than guessed.
            RecvKind::Unknown => return None,
        };
        Some(edit(outer, replacement))
    }
}

impl<'ast> Visit<'ast> for Collector<'_, 'ast> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        syn::visit::visit_expr(self, e);
        if let Some(se) = self.try_check_shape(e) {
            self.edits.push(se);
        }
    }

    fn visit_item_const(&mut self, c: &'ast syn::ItemConst) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(c.ident.to_string(), &c.ty);
        }
        syn::visit::visit_item_const(self, c);
    }

    fn visit_item_static(&mut self, s: &'ast syn::ItemStatic) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(s.ident.to_string(), &s.ty);
        }
        syn::visit::visit_item_static(self, s);
    }

    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        if let Fields::Named(fields) = &s.fields {
            let rows = fields
                .named
                .iter()
                .filter_map(|f| {
                    let n = f.ident.as_ref()?.to_string();
                    Some((n, &f.ty))
                })
                .collect();
            self.structs.insert(s.ident.to_string(), rows);
        }
        syn::visit::visit_item_struct(self, s);
    }

    fn visit_item_type(&mut self, t: &'ast syn::ItemType) {
        self.aliases.insert(t.ident.to_string(), &t.ty);
        syn::visit::visit_item_type(self, t);
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let saved = self.current_self.take();
        self.current_self = path_tail_ident(&i.self_ty);
        syn::visit::visit_item_impl(self, i);
        self.current_self = saved;
    }

    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.push_fn_scope(&f.sig, None);
        syn::visit::visit_item_fn(self, f);
        self.scopes.pop();
    }

    fn visit_impl_item_fn(&mut self, m: &'ast syn::ImplItemFn) {
        let self_struct = self.current_self.clone();
        self.push_fn_scope(&m.sig, self_struct);
        syn::visit::visit_impl_item_fn(self, m);
        self.scopes.pop();
    }

    fn visit_trait_item_fn(&mut self, m: &'ast syn::TraitItemFn) {
        self.push_fn_scope(&m.sig, None);
        syn::visit::visit_trait_item_fn(self, m);
        self.scopes.pop();
    }

    fn visit_local(&mut self, l: &'ast syn::Local) {
        // `let x: T = …;` — the annotation lives in the PatType.
        if let Pat::Type(pt) = &l.pat {
            if let Pat::Ident(pi) = &*pt.pat {
                if pi.subpat.is_none() {
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.bindings.insert(pi.ident.to_string(), pt.ty.as_ref());
                    }
                }
            }
        }
        syn::visit::visit_local(self, l);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn get_is_none_becomes_negated_contains_key() {
        // The typed-map shape (a typed fn param — resolvable evidence).
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
    fn field_receiver_in_file_struct_still_heals() {
        // Issue 235: the field form heals ONLY when the struct is in-file
        // (`s: &S` with S unknown declined after the incident).
        let input = "struct S { map: std::collections::HashMap<u8, u8> }\n\
                     fn f(s: &S, k: u8) -> bool { s.map.get(&k).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(
            out.contains("!s.map.contains_key(&k)"),
            "receiver and arg bytes survive verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn self_field_receiver_heals() {
        let input = "struct T { m: std::collections::HashMap<u8, u8> }\n\
                     impl T { fn f(&self, k: u8) -> bool { self.m.get(&k).is_some() } }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(
            out.contains("self.m.contains_key(&k)") && !out.contains('!'),
            "self's impl struct resolves the field type: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn typed_local_heals() {
        let input = "fn f(k: u8) -> bool { let m: std::collections::HashMap<u8, u8> = Default::default(); m.get(&k).is_some() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(out.contains("m.contains_key(&k)"), "{out}");
    }

    #[test]
    fn untyped_let_declines() {
        let input = "fn f(k: u8) -> bool { let m = make_map(); m.get(&k).is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn unknown_param_type_declines() {
        // The incident class: no in-file evidence — the contains_key splice
        // would be a guess; the site is left for manual repair (issue 235).
        let input = "fn f(m: &M, k: u8) -> bool { m.get(&k).is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn method_call_receiver_declines() {
        let input = "fn f() -> bool { build().get(&1).is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn hashset_receiver_declines() {
        let input =
            "fn f(s: &std::collections::HashSet<u8>, k: u8) -> bool { s.get(&k).is_some() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn string_receiver_declines() {
        // str::get is also a char-boundary check — the comparison form is
        // NOT equivalent there; the honest answer declines.
        let input = "fn f(s: &str, i: usize) -> bool { s.get(i).is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn vec_get_is_none_becomes_bounds_check() {
        // THE issue-235 wrong-heal shape: Vec<String> + get(usize).is_none()
        // must NEVER produce contains_key (the E0599 that shipped once
        // through a gate-posture hole).
        let input = "fn f(v: &Vec<String>, i: usize) -> bool { v.get(i).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(out.contains("i >= v.len()"), "bounds-check form: {out}");
        assert!(!out.contains("contains_key"), "no contains_key: {out}");
        assert!(!out.contains("get"), "get must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn vec_get_is_some_becomes_bounds_check() {
        let input = "fn f(v: Vec<u8>, i: usize) -> bool { v.get(i).is_some() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(out.contains("i < v.len()"), "{out}");
        assert!(!out.contains("contains_key"), "{out}");
        assert!(!out.contains('!'), "is_some stays positive: {out}");
    }

    #[test]
    fn slice_ref_get_is_none_becomes_bounds_check() {
        let input = "fn f(v: &[u8], i: usize) -> bool { v.get(i).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(out.contains("i >= v.len()"), "{out}");
    }

    #[test]
    fn vecdeque_get_becomes_bounds_check() {
        let input =
            "fn f(v: &std::collections::VecDeque<u8>, i: usize) -> bool { v.get(i).is_some() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(out.contains("i < v.len()"), "{out}");
    }

    #[test]
    fn complex_index_arg_gets_parens() {
        // An if-expression does not bind tighter than a comparison — the
        // splice parenthesizes the argument.
        let input =
            "fn f(v: &Vec<u8>, i: usize) -> bool { v.get(if i > 3 { 9 } else { 1 }).is_some() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(
            out.contains("(if i > 3 { 9 } else { 1 }) < v.len()"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn borrowed_arg_index_site_declines() {
        // A borrowed arg cannot be a slice get — the evidence and the call
        // disagree, so the site declines rather than guessing.
        let input = "fn f(v: &Vec<u8>, k: u8) -> bool { v.get(&k).is_none() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn custom_map_suffixed_type_heals() {
        // The suffix policy: `…Map` names are trusted as map-like (a type
        // without contains_key is an E0599 the --verify gate reverts —
        // the disclosed residual).
        let input = "struct CacheMap { inner: u8 }\n\
                     fn f(m: &CacheMap, k: u8) -> bool { m.get(&k).is_some() }";
        let out = propose_fix("unnecessary_get_then_check", input).expect("matched");
        assert!(out.contains("m.contains_key(&k)"), "{out}");
    }

    #[test]
    fn turbofish_get_refused() {
        let input = "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get::<u8>(&k).is_none() }";
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
        let input = "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> Option<&u8> { m.get(&k).copied() }";
        assert!(propose_fix("unnecessary_get_then_check", input).is_none());
    }

    #[test]
    fn interior_comment_refused() {
        let input = "fn f(m: &std::collections::HashMap<u8, u8>, k: u8) -> bool { m.get( /* key */ &k).is_none() }";
        let out = propose_fix("unnecessary_get_then_check", input);
        assert!(
            out.is_none(),
            "comment that the splice would drop refuses: {out:?}"
        );
    }
}
