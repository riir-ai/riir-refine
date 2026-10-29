//! `clippy::assert_is_empty` — `assert!(s.is_empty())` → `assert_eq!(s, "")`
//! (negated → `assert_ne!`, `debug_assert!` twins map to the `_eq`/`_ne`
//! debug forms) on a proven receiver (Batch 230 / Plan 232 — NEW
//! in clippy 1.99 / PR #17149, pedantic group; probe-verified 2026-10-13
//! against clippy 1.99.0 (isolated toolchain); the fixed forms compile at
//! 1.98.1 too).
//!
//! ## Why the rewrite is pass-behavior identical (the FIXABLE bar)
//!
//! `assert_eq!(a, b)` is defined by std as exactly `assert!(*a == *b)`
//! (modulo the by-reference capture dance) — so the PASS path of the
//! rewrite is identical: same condition, same operands, same evaluation.
//! The PANIC MESSAGE differs — and that difference IS the lint's stated
//! purpose: on failure `assert_eq!` prints the asserted values
//! (`left: "x"`, `right: ""`) while `assert!` prints only the literal
//! message; printing the asserted value on failure is the improvement the
//! lint exists to make, not a behavior regression.
//!
//! ## Provenance (the P98 extension, 2026-10-29 — the 93-site wave class)
//!
//! The type-blindness leg: the receiver's family is proven from
//! FILE-LOCAL construction facts — no type checker here. The original law
//! proved params from the fn signature alone; the pin-bump wave (commit
//! ff91fb5f) proved the demand was 93/93 locals and method/field
//! receivers (`set.caps().is_empty()`, `s.lints.is_empty()`), every one
//! of which the signature-only gate refused. Four construction-fact legs
//! now feed one proven-binding map:
//!
//! 1. ANNOTATED LETS — `let v: Vec<String> = …;` (the annotation is the
//!    construction fact, exactly like a param's).
//! 2. SAME-FILE DEFINITIONS — `let v = g();` resolves `g` through the
//!    file's own fn table; `s.caps()` / `s.lints` resolve through the
//!    file's inherent-impl and struct tables; `NAMES` (a const/static)
//!    resolves through its declared type. A type the file does not
//!    define is absent, and absence refuses — cross-file types are never
//!    guessed.
//! 3. STD CONSTRUCTORS — `String::new|from|with_capacity` (the Str
//!    family) and the turbofish `Vec::<T>::new()` (`VecOwned`). Bare
//!    `Vec::new()` refuses: the element is unknown, the exact
//!    under-inference clippy's own `MaybeIncorrect` suggestion trips on.
//! 4. PROPAGATION — `let w = v;` copies the proven fact one hop (types
//!    do not change on a move or a copy).
//!
//! Receiver shapes: a bare binding (or const/static) ident, a ONE-HOP
//! field, or a ONE-HOP inherent method call — deeper chains refuse.
//!
//! Soundness legs: a local is a fact only when its name is bound EXACTLY
//! once in the fn (no shadow, no re-`=` — the census leg, closure bodies
//! counted) and the resolution chain carries no generics (a return type
//! mentioning a generic id is not a fact — there is no substitution
//! machinery here); `Self` in an inherent impl's return substitutes to
//! the self type's name; two same-named definitions with different types
//! poison the name (`Ambiguous`); trait methods (`collect`, `trim`,
//! `clone`…) are in no inherent table and refuse — their return types
//! are unknowable file-locally. Every refusal is the honest answer, and
//! the compile gate behind `--fix --verify` is the backstop.
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - The macro path is EXACTLY `assert` or `debug_assert` — a plain
//!   single segment, no generic args (`assert_eq` is NOT our input; it is
//!   the OUTPUT; multi-segment/qualified spellings refuse).
//! - EXACTLY ONE macro argument — a custom message (`assert!(v, "msg")`,
//!   2+ args) never fires, matching the verified upstream silence.
//! - The single argument is `x.is_empty()` or `!x.is_empty()`: method
//!   exactly `is_empty`, ZERO args, no turbofish, receiver a PROVEN
//!   shape (see Provenance). Method chains beyond one resolvable hop,
//!   fields through unproven bases, indexes, calls, and macro-expanded
//!   receivers (`m!().is_empty()`) refuse.
//! - Proven families and their fixed forms: `&str` / `String` /
//!   `&String` → `assert_eq!(s, "")`; `&[T]` → `assert_eq!(s, [])`;
//!   `&Vec<T>` → `assert_eq!(s.as_slice(), [])`; owned `Vec<T>` →
//!   `assert_eq!(v, [] as [T; 0])` (`T` spelled via canonical token text).
//!   Classification is STRUCTURAL on the syn type node — the conservative
//!   reading of "exact source spelling": `&mut` anything, `&[T; N]`
//!   arrays (a `[T; N] == [T; 0]` compare would not compile),
//!   multi-segment paths (`std::vec::Vec<T>`), qualified paths, `HashMap`
//!   and every other type text → REFUSE.
//! - Negation flips `_eq` → `_ne`; the `debug_` prefix carries.
//! - A comment inside the replaced macro span refuses (only the
//!   receiver's own bytes survive verbatim in the splice).
//! - Macros nested inside OTHER macros' token streams are invisible to
//!   syn's visitor (opaque tokens, no descent) — conservative silence.
//!
//! House note: the arg extraction is `all_expr_args`, NOT
//! `macro_expr_args` — the assert CONDITION is the PRE-template operand
//! (family position `Second`), which `post_template_expr_args` skips, so
//! `macro_expr_args` returns an empty vec for every bare `assert!(cond)`
//! (dead for this lint's shape). `all_expr_args` includes the pre-template
//! operands — the same lane `match_bool` uses for `assert!(match ..)`.

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::visit::Visit;
use syn::{Expr, ExprMacro, File, ImplItemFn, ItemFn, Macro, StmtMacro};

use super::helpers::{body_rebinds, byte_range, drops_interior_comment};
use super::uninlined_format_args::all_expr_args;
use super::{edit, src, SourceEdit};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let tables = FileTables::build(file);
    let mut v = Collector {
        source,
        edits: Vec::new(),
        proven: BTreeMap::new(),
        tables: &tables,
    };
    v.visit_file(file);
    v.edits
}

/// The signature-proven family of a type — decides the right-hand
/// side of the rewrite and whether the receiver is re-spelled.
#[derive(Clone)]
enum ParamType {
    /// `&str` / `String` / `&String` — compare against `""`.
    Str,
    /// `&[T]` — compare against `[]`.
    Slice,
    /// `&Vec<T>` — re-spell the receiver `.as_slice()`, compare against
    /// `[]` (the probe-verified fixed form for the shared-Vec family).
    VecRef,
    /// Owned `Vec<T>` (element text spelled) — compare against
    /// `[] as [T; 0]` (the cast names `T`, the one shape where a bare
    /// `[]` under-infers).
    VecOwned(String),
}

impl ParamType {
    /// The replacement macro invocation for the whole assert span —
    /// `name!(lhs, rhs)` with the variant/prefix carried from the input.
    fn rewrite(&self, debug: bool, negated: bool, recv: &str) -> String {
        let name = match (debug, negated) {
            (false, false) => "assert_eq",
            (false, true) => "assert_ne",
            (true, false) => "debug_assert_eq",
            (true, true) => "debug_assert_ne",
        };
        let lhs = match self {
            Self::VecRef => format!("{recv}.as_slice()"),
            _ => recv.to_string(),
        };
        let rhs = match self {
            Self::Str => "\"\"".to_string(),
            Self::Slice | Self::VecRef => "[]".to_string(),
            Self::VecOwned(elem) => format!("[] as [{elem}; 0]"),
        };
        format!("{name}!({lhs}, {rhs})")
    }
}

/// Everything proven about one binding: a direct rewrite family when the
/// type classifies, and/or the bare type name the field/method tables
/// resolve through. A default (empty) `Proven` refuses every lookup.
#[derive(Clone, Default)]
struct Proven {
    family: Option<ParamType>,
    type_name: Option<String>,
}

/// A name-resolution outcome across the file's own definitions.
enum Table<V> {
    /// One definition (or several agreeing byte-for-byte).
    Known(V),
    /// Two same-named definitions with different types — no honest fact.
    Ambiguous,
}

impl<V> Table<V> {
    fn known(&self) -> Option<&V> {
        match self {
            Table::Known(v) => Some(v),
            Table::Ambiguous => None,
        }
    }
}

/// File-local name tables for the construction-fact legs: free fns,
/// inherent-impl methods per self type, struct fields per type, and
/// consts/statics by name. Built once per `collect_edits` from the parsed
/// file; a type the file does not define is simply absent, and absence
/// refuses.
#[derive(Default)]
struct FileTables {
    fns: BTreeMap<String, Table<syn::Type>>,
    methods: BTreeMap<String, BTreeMap<String, Table<syn::Type>>>,
    structs: BTreeMap<String, Table<BTreeMap<String, syn::Type>>>,
    consts: BTreeMap<String, Table<syn::Type>>,
}

impl FileTables {
    fn build(file: &File) -> Self {
        let mut b = TableBuilder {
            tables: Self::default(),
        };
        b.visit_file(file);
        b.tables
    }

    fn get_fn(&self, name: &str) -> Option<&syn::Type> {
        self.fns.get(name)?.known()
    }

    fn get_method(&self, ty: &str, m: &str) -> Option<&syn::Type> {
        self.methods.get(ty)?.get(m)?.known()
    }

    fn get_field(&self, ty: &str, f: &str) -> Option<&syn::Type> {
        self.structs.get(ty)?.known()?.get(f)
    }

    fn get_const(&self, name: &str) -> Option<&syn::Type> {
        self.consts.get(name)?.known()
    }
}

/// Whether two tabled types agree byte-for-byte (identical re-definitions
/// keep the name; any disagreement poisons it).
fn type_same(a: &syn::Type, b: &syn::Type) -> bool {
    a.to_token_stream().to_string() == b.to_token_stream().to_string()
}

/// Registers a name, poisoning on disagreement: the first definition
/// wins as `Known`; a second that agrees byte-for-byte keeps it; a
/// second that disagrees marks the name `Ambiguous` (refused forever).
fn insert<V: Clone>(
    map: &mut BTreeMap<String, Table<V>>,
    name: &str,
    value: V,
    same: impl Fn(&V, &V) -> bool,
) {
    let bump = match map.get(name) {
        None => Some(Table::Known(value.clone())),
        Some(Table::Known(prev)) if same(prev, &value) => None,
        Some(_) => Some(Table::Ambiguous),
    };
    if let Some(t) = bump {
        map.insert(name.to_string(), t);
    }
}

/// One pass over the file building the resolution tables. Generic
/// definitions are excluded (a return mentioning a generic id is not a
/// concrete fact — there is no substitution machinery); trait impls are
/// excluded (they are not inherent facts); `Self` in an inherent impl's
/// return substitutes to the self type's name.
struct TableBuilder {
    tables: FileTables,
}

impl<'ast> Visit<'ast> for TableBuilder {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        if let syn::ReturnType::Type(_, ty) = &f.sig.output {
            if !mentions_param(&f.sig.generics, ty) {
                insert(
                    &mut self.tables.fns,
                    &f.sig.ident.to_string(),
                    (**ty).clone(),
                    type_same,
                );
            }
        }
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        if let syn::Fields::Named(named) = &s.fields {
            let mut fields = BTreeMap::new();
            for field in &named.named {
                let Some(ident) = &field.ident else { continue };
                if mentions_param(&s.generics, &field.ty) {
                    continue;
                }
                fields.insert(ident.to_string(), field.ty.clone());
            }
            if !fields.is_empty() {
                // A second definition of the same struct name is a genuine
                // ambiguity (mod-scoped names collide at this table's
                // granularity) — any collision poisons the name.
                insert(
                    &mut self.tables.structs,
                    &s.ident.to_string(),
                    fields,
                    |_, _| false,
                );
            }
        }
        syn::visit::visit_item_struct(self, s);
    }

    fn visit_item_impl(&mut self, imp: &'ast syn::ItemImpl) {
        if imp.trait_.is_some() || !imp.generics.params.is_empty() {
            // Trait impls are not inherent facts; a generic impl's methods
            // would not resolve for a concrete instance name.
            return;
        }
        let syn::Type::Path(tp) = &*imp.self_ty else {
            return;
        };
        let Some(seg) = sole_segment(tp) else {
            return;
        };
        if !seg.arguments.is_none() {
            return;
        }
        let self_name = seg.ident.to_string();
        let entry = self.tables.methods.entry(self_name.clone()).or_default();
        for item in &imp.items {
            let syn::ImplItem::Fn(m) = item else { continue };
            let syn::ReturnType::Type(_, ty) = &m.sig.output else {
                continue;
            };
            if mentions_param(&m.sig.generics, ty) {
                continue;
            }
            let resolved = substitute_self(ty, &self_name);
            insert(entry, &m.sig.ident.to_string(), resolved, type_same);
        }
        // Descend: free fns nested in method bodies are file-local facts too.
        syn::visit::visit_item_impl(self, imp);
    }

    fn visit_item_const(&mut self, c: &'ast syn::ItemConst) {
        if !mentions_param(&c.generics, &c.ty) {
            insert(
                &mut self.tables.consts,
                &c.ident.to_string(),
                (*c.ty).clone(),
                type_same,
            );
        }
        syn::visit::visit_item_const(self, c);
    }

    fn visit_item_static(&mut self, s: &'ast syn::ItemStatic) {
        insert(
            &mut self.tables.consts,
            &s.ident.to_string(),
            (*s.ty).clone(),
            type_same,
        );
        syn::visit::visit_item_static(self, s);
    }
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Proven bindings of the enclosing fn — params plus the stable
    /// locals' file-local construction facts.
    proven: BTreeMap<String, Proven>,
    tables: &'a FileTables,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        // Save/restore via a plain local — correct under arbitrary
        // nesting (an inner fn's bindings replace the outer map for the
        // inner body only).
        let saved = std::mem::take(&mut self.proven);
        self.proven = proven_bindings(f.sig.inputs.iter(), &f.block, self.tables);
        syn::visit::visit_item_fn(self, f);
        self.proven = saved;
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let saved = std::mem::take(&mut self.proven);
        self.proven = proven_bindings(f.sig.inputs.iter(), &f.block, self.tables);
        syn::visit::visit_impl_item_fn(self, f);
        self.proven = saved;
    }

    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro(&m.mac);
    }

    /// Statement-position macros (`assert!(…);`) parse as `Stmt::Macro`,
    /// not `Expr::Macro` — both shapes are visited (the `match_bool`
    /// precedent).
    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro(&m.mac);
    }
}

impl Collector<'_> {
    fn scan_macro(&mut self, mac: &Macro) {
        if let Some(replacement) = self.try_rewrite(mac) {
            self.edits.push(edit(mac, replacement));
        }
    }

    /// The whole-macro rewrite when every guard passes — `None` refuses.
    fn try_rewrite(&self, mac: &Macro) -> Option<String> {
        let debug = plain_assert_variant(mac)?;
        let args = all_expr_args(mac)?;
        if args.len() != 1 {
            return None; // custom message / format args — upstream silence
        }
        let toks: TokenStream = args[0].toks.iter().cloned().collect();
        let expr = syn::parse2::<Expr>(toks).ok()?;
        let (negated, call) = match &expr {
            Expr::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => (true, &*u.expr),
            other => (false, other),
        };
        let Expr::MethodCall(m) = call else {
            return None;
        };
        if m.method != "is_empty" || !m.args.is_empty() || m.turbofish.is_some() {
            return None;
        }
        let family = self.resolve_family(&m.receiver)?;
        // The whole macro span is re-spelled; only the receiver's own
        // bytes survive verbatim — a comment anywhere else would drop.
        if drops_interior_comment(self.source, byte_range(mac), &[byte_range(&*m.receiver)]) {
            return None;
        }
        let recv = src(self.source, &*m.receiver)?;
        Some(family.rewrite(debug, negated, &recv))
    }

    /// The family of the asserted value, proven from the receiver shape:
    /// a bare binding (or const/static) path, a ONE-HOP field (`s.lints`),
    /// or a ONE-HOP inherent method call (`set.caps()`) — the base must be
    /// a binding whose type name is a construction fact. Deeper chains,
    /// trait methods (absent from the inherent table), macro receivers,
    /// and every type the file does not define refuse.
    fn resolve_family(&self, expr: &Expr) -> Option<ParamType> {
        match expr {
            Expr::Path(_) => {
                let name = bare_ident(expr)?;
                if let Some(p) = self.proven.get(name.as_str()) {
                    return p.family.clone();
                }
                let ty = self.tables.get_const(name.as_str())?;
                classify_family(ty)
            }
            Expr::Field(f) => {
                let syn::Member::Named(member) = &f.member else {
                    return None;
                };
                let base = bare_ident(&f.base)?;
                let tn = self.proven.get(base.as_str())?.type_name.as_ref()?;
                let ty = self.tables.get_field(tn, &member.to_string())?;
                classify_family(ty)
            }
            Expr::MethodCall(m) => {
                if m.turbofish.is_some() {
                    return None;
                }
                let base = bare_ident(&m.receiver)?;
                let tn = self.proven.get(base.as_str())?.type_name.as_ref()?;
                let ty = self.tables.get_method(tn, &m.method.to_string())?;
                classify_family(ty)
            }
            _ => None,
        }
    }
}

/// The proven bindings of one fn: params first (the signature law — a
/// plain never-rebound `Pat::Ident` param whose annotation classifies;
/// the shared `body_rebinds` soundness leg), then every stable `let`
/// local's file-local construction fact. The map is complete before the
/// body is visited: each name is bound at most once (the census leg
/// refuses shadows), so traversal order cannot change any verdict.
fn proven_bindings<'i>(
    inputs: impl Iterator<Item = &'i syn::FnArg>,
    block: &syn::Block,
    tables: &FileTables,
) -> BTreeMap<String, Proven> {
    let mut out = BTreeMap::new();
    for arg in inputs {
        let syn::FnArg::Typed(p) = arg else { continue };
        let syn::Pat::Ident(pi) = &*p.pat else {
            continue;
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            continue;
        }
        if body_rebinds(block, &pi.ident.to_string()) {
            continue;
        }
        out.insert(
            pi.ident.to_string(),
            Proven {
                family: classify_family(&p.ty),
                type_name: named_type_name(&p.ty),
            },
        );
    }
    let census = binding_census(block);
    let mut scan = LocalScanner {
        out: &mut out,
        tables,
        census: &census,
    };
    scan.visit_block(block);
    out
}

/// Walks the fn body's `let` statements in source order, inserting each
/// stable local's proven fact (or poisoning an existing entry for the
/// name).
struct LocalScanner<'a> {
    out: &'a mut BTreeMap<String, Proven>,
    tables: &'a FileTables,
    census: &'a BTreeMap<String, (usize, bool)>,
}

impl<'ast> Visit<'ast> for LocalScanner<'_> {
    /// Nested fn bodies are their own scopes — their locals are never
    /// this fn's facts. Closures are NOT skipped: a closure rebinding the
    /// name breaks the one-binding census for the outer binding
    /// (conservative).
    fn visit_item_fn(&mut self, _f: &'ast ItemFn) {}

    fn visit_local(&mut self, l: &'ast syn::Local) {
        // syn 2 folds the annotation into the pattern: `let v: T = …`
        // parses as Pat::Type { pat: Ident, ty }.
        let (pi, ty) = match &l.pat {
            syn::Pat::Ident(pi) => (pi, None),
            syn::Pat::Type(pt) => match &*pt.pat {
                syn::Pat::Ident(pi) => (pi, Some(&*pt.ty)),
                _ => return,
            },
            _ => return,
        };
        if pi.by_ref.is_some() || pi.subpat.is_some() {
            return;
        }
        let name = pi.ident.to_string();
        let fact = match ty {
            Some(t) => Proven {
                family: classify_family(t),
                type_name: named_type_name(t),
            },
            None => match &l.init {
                Some(init) => init_fact(&init.expr, self.tables, self.out),
                None => Proven::default(),
            },
        };
        // The construction-fact leg: exactly ONE binding pattern for the
        // name (its own) and zero assignments — a shadow or a re-`= `
        // poisons the name (an empty `Proven` refuses every lookup).
        let entry = match self.census.get(&name) {
            Some((1, false)) => fact,
            _ => Proven::default(),
        };
        self.out.insert(name, entry);
    }
}

fn init_fact(expr: &Expr, tables: &FileTables, proven: &BTreeMap<String, Proven>) -> Proven {
    match expr {
        Expr::Call(c) => call_fact(&c.func, tables),
        Expr::Struct(s) => Proven {
            family: None,
            type_name: sole_ident_path(&s.path),
        },
        Expr::Path(p) => {
            let Some(name) = sole_ident_path(&p.path) else {
                return Proven::default();
            };
            // `let w = v;` — the binding-to-binding propagation (one hop;
            // types do not change on a move or a copy). Read from the map
            // BEFORE inserting: `let x = x;` copies the shadowed binding's
            // fact, then poisons the name.
            if let Some(prev) = proven.get(name.as_str()) {
                return prev.clone();
            }
            // A bare unit-struct path names its type — a fact for
            // field/method resolution.
            if matches!(tables.structs.get(name.as_str()), Some(Table::Known(_))) {
                return Proven {
                    family: None,
                    type_name: Some(name),
                };
            }
            Proven::default()
        }
        _ => Proven::default(),
    }
}

/// The fact carried by a call-shaped initializer: the hardcoded std
/// constructor carve-outs (the same families the signature law names),
/// else the file's own fn / inherent-method tables.
fn call_fact(func: &Expr, tables: &FileTables) -> Proven {
    let Expr::Path(p) = func else {
        return Proven::default();
    };
    if p.qself.is_some() || p.path.leading_colon.is_some() {
        return Proven::default();
    }
    let segs: Vec<&syn::PathSegment> = p.path.segments.iter().collect();
    match segs.len() {
        1 => {
            // A free fn call — a turbofish here is generic flow, refused.
            if !segs[0].arguments.is_none() {
                return Proven::default();
            }
            let name = segs[0].ident.to_string();
            tables.get_fn(&name).map(type_fact).unwrap_or_default()
        }
        2 => {
            let head = segs[0].ident.to_string();
            let tail = segs[1].ident.to_string();
            if !segs[1].arguments.is_none() {
                return Proven::default(); // `Type::f::<T>()` — generic flow
            }
            if head == "String" && matches!(tail.as_str(), "new" | "from" | "with_capacity") {
                return Proven {
                    family: Some(ParamType::Str),
                    type_name: None,
                };
            }
            if head == "Vec" && tail == "new" {
                // `Vec::<T>::new()` spells T — the one sound turbofish.
                if let Some(elem) = sole_type_argument(segs[0]) {
                    if !contains_infer(elem) {
                        return Proven {
                            family: Some(ParamType::VecOwned(type_text(elem))),
                            type_name: None,
                        };
                    }
                }
                return Proven::default(); // `Vec::new()` — element unknown
            }
            // `Type::assoc_fn` — resolved through the inherent-method table.
            tables
                .get_method(&head, &tail)
                .map(type_fact)
                .unwrap_or_default()
        }
        _ => Proven::default(),
    }
}

fn type_fact(ty: &syn::Type) -> Proven {
    Proven {
        family: classify_family(ty),
        type_name: named_type_name(ty),
    }
}

/// Per-name binding facts across one fn body: how many times the name is
/// bound by a pattern (its own `let`, a shadow, an `if let`…) and
/// whether any assignment targets it. Nested fn bodies are other scopes
/// (skipped); closure bodies are NOT (a closure rebinding the name
/// poisons it).
fn binding_census(block: &syn::Block) -> BTreeMap<String, (usize, bool)> {
    struct Census {
        m: BTreeMap<String, (usize, bool)>,
    }
    impl<'ast> Visit<'ast> for Census {
        fn visit_item_fn(&mut self, _f: &'ast ItemFn) {}
        fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
            self.m.entry(p.ident.to_string()).or_insert((0, false)).0 += 1;
            syn::visit::visit_pat_ident(self, p);
        }
        fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
            if let Some(id) = expr_path_ident(&a.left) {
                self.m.entry(id).or_insert((0, false)).1 = true;
            }
            syn::visit::visit_expr_assign(self, a);
        }
        fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
            use syn::BinOp;
            if matches!(
                b.op,
                BinOp::AddAssign(_)
                    | BinOp::SubAssign(_)
                    | BinOp::MulAssign(_)
                    | BinOp::DivAssign(_)
                    | BinOp::RemAssign(_)
                    | BinOp::BitXorAssign(_)
                    | BinOp::BitAndAssign(_)
                    | BinOp::BitOrAssign(_)
                    | BinOp::ShlAssign(_)
                    | BinOp::ShrAssign(_)
            ) {
                if let Some(id) = expr_path_ident(&b.left) {
                    self.m.entry(id).or_insert((0, false)).1 = true;
                }
            }
            syn::visit::visit_expr_binary(self, b);
        }
    }
    let mut c = Census { m: BTreeMap::new() };
    c.visit_block(block);
    c.m
}

/// Structural classification of a provable type into its rewrite
/// family — the conservative posture of `scalar_head_of`/`elem_head_of`:
/// every unusual shape (mut refs, arrays, multi-segment/qualified paths,
/// non-container types, `_` elements) refuses.
fn classify_family(ty: &syn::Type) -> Option<ParamType> {
    match ty {
        syn::Type::Reference(r) => {
            if r.mutability.is_some() {
                return None;
            }
            match &*r.elem {
                syn::Type::Slice(_) => Some(ParamType::Slice),
                syn::Type::Path(p) => {
                    let seg = sole_segment(p)?;
                    match seg.ident.to_string().as_str() {
                        "str" | "String" => Some(ParamType::Str),
                        "Vec" => {
                            // Shape check only — exactly one type argument
                            // must be present; the T text is not needed.
                            sole_type_argument(seg)?;
                            Some(ParamType::VecRef)
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        syn::Type::Path(p) => {
            let seg = sole_segment(p)?;
            match seg.ident.to_string().as_str() {
                "String" => Some(ParamType::Str),
                "Vec" => {
                    let elem = sole_type_argument(seg)?;
                    if contains_infer(elem) {
                        return None;
                    }
                    Some(ParamType::VecOwned(type_text(elem)))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether a type contains a `_` inference placeholder — `Vec<_>` has no
/// spellable element, and the as-cast form would not compile.
fn contains_infer(ty: &syn::Type) -> bool {
    struct InferHit(bool);
    impl<'ast> Visit<'ast> for InferHit {
        fn visit_type_infer(&mut self, _: &'ast syn::TypeInfer) {
            self.0 = true;
        }
    }
    let mut h = InferHit(false);
    h.visit_type(ty);
    h.0
}

/// The sole path segment of a plain path type — no qself, no leading
/// `::`, exactly one segment (`std::vec::Vec<T>` and qualified spellings
/// refuse).
fn sole_segment(p: &syn::TypePath) -> Option<&syn::PathSegment> {
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    p.path.segments.first()
}

/// The single type argument of a generic segment (`Vec<T>` → `T`) —
/// exactly one angle-bracketed argument and it is a type; `Vec<>`,
/// `Vec<T, U>` and lifetime-only shapes refuse.
fn sole_type_argument(seg: &syn::PathSegment) -> Option<&syn::Type> {
    let syn::PathArguments::AngleBracketed(ab) = &seg.arguments else {
        return None;
    };
    if ab.args.len() != 1 {
        return None;
    }
    match ab.args.first()? {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}

/// Canonical token text of a type (the owned-`Vec` cast's element
/// spelling) — a bare ident prints clean; a nested generic prints with
/// token spacing, which is still-valid Rust at the cast position.
fn type_text(t: &syn::Type) -> String {
    t.to_token_stream().to_string()
}

/// The bare single-word name of a type, for field/method resolution:
/// `CoverageReport`, through one non-mut reference (`&CoverageReport` —
/// auto-deref/auto-ref make the type's fields and inherent methods
/// reachable). Generic instances (`Vec<String>`) and every other shape
/// refuse.
fn named_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(p) => {
            let seg = sole_segment(p)?;
            if !seg.arguments.is_none() {
                return None;
            }
            Some(seg.ident.to_string())
        }
        syn::Type::Reference(r) if r.mutability.is_none() => named_type_name(&r.elem),
        _ => None,
    }
}

/// Whether a type mentions any generic parameter (type or lifetime) of
/// its definition — such a return is not a concrete fact, and no
/// substitution machinery exists here. Lifetime ids are checked because
/// they leak into the emitted cast text (`[] as [&'a str; 0]`).
fn mentions_param(generics: &syn::Generics, ty: &syn::Type) -> bool {
    let ids: Vec<String> = generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(tp) => Some(tp.ident.to_string()),
            syn::GenericParam::Lifetime(lt) => Some(lt.lifetime.ident.to_string()),
            syn::GenericParam::Const(_) => None,
        })
        .collect();
    if ids.is_empty() {
        return false;
    }
    ty.to_token_stream().into_iter().any(|t| {
        matches!(&t, proc_macro2::TokenTree::Ident(i) if ids.iter().any(|id| id == &i.to_string()))
    })
}

/// Rewrite every `Self` token in a return type to the impl's self-type
/// name (token-level — no substring hazards). A parse failure keeps the
/// original, which classification will then refuse.
fn substitute_self(ty: &syn::Type, self_name: &str) -> syn::Type {
    let ts: TokenStream = ty
        .to_token_stream()
        .into_iter()
        .map(|t| match t {
            proc_macro2::TokenTree::Ident(ref i) if i == "Self" => {
                proc_macro2::TokenTree::Ident(proc_macro2::Ident::new(self_name, i.span()))
            }
            other => other,
        })
        .collect();
    syn::parse2(ts).unwrap_or_else(|_| ty.clone())
}

/// The single segment name of a plain path (no leading `::`, no generic
/// args) — the caller owns the qself check where its type carries one.
fn sole_ident_path(p: &syn::Path) -> Option<String> {
    if p.leading_colon.is_some() || p.segments.len() != 1 {
        return None;
    }
    let seg = p.segments.first()?;
    if !seg.arguments.is_none() {
        return None;
    }
    Some(seg.ident.to_string())
}

/// The bare path ident of a receiver expression — `None` for every other
/// shape (chain, field, index, call, macro, multi-segment path).
fn bare_ident(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() {
        return None;
    }
    sole_ident_path(&p.path)
}

/// The bare ident of a plain path expression — assignment/op-assign
/// targets.
fn expr_path_ident(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else { return None };
    if p.qself.is_some() {
        return None;
    }
    sole_ident_path(&p.path)
}

/// `Some(debug)` when the macro path is exactly `assert` / `debug_assert`
/// (a plain single segment, no generic args); `None` for every other
/// macro — `assert_eq` inputs are not ours, multi-segment spellings
/// refuse.
fn plain_assert_variant(mac: &Macro) -> Option<bool> {
    let p = &mac.path;
    if p.leading_colon.is_some() || p.segments.len() != 1 {
        return None;
    }
    let seg = p.segments.first()?;
    if !seg.arguments.is_none() {
        return None;
    }
    match seg.ident.to_string().as_str() {
        "assert" => Some(false),
        "debug_assert" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const RULE: &str = "assert_is_empty";

    #[test]
    fn seed_is_clean() {
        // Clean code — the `if` condition is NOT an assert context
        // (upstream silence for non-assert contexts), and no macro fires.
        assert!(propose_fix(
            RULE,
            "fn f(hits: &mut Vec<u8>, v: Vec<u32>) -> bool { hits.push(0); if v.is_empty() { return true; } false }",
        )
        .is_none());
    }

    #[test]
    fn str_family_fires() {
        // `&str`, `String` and `&String` all take the `""` form.
        let out = propose_fix(
            RULE,
            "fn f(a: &str, b: String, c: &String) { assert!(a.is_empty()); assert!(b.is_empty()); assert!(c.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(a, \"\")"),
            "a not rewritten: {out}"
        );
        assert!(
            out.contains("assert_eq!(b, \"\")"),
            "b not rewritten: {out}"
        );
        assert!(
            out.contains("assert_eq!(c, \"\")"),
            "c not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn slice_param_fires() {
        let out = propose_fix(RULE, "fn f(s: &[u32]) { assert!(s.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_eq!(s, [])"),
            "slice not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn vec_ref_param_fires() {
        // `&Vec<T>` re-spells the slice view — the probe-verified form.
        let out =
            propose_fix(RULE, "fn f(v: &Vec<u32>) { assert!(v.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_eq!(v.as_slice(), [])"),
            "vec-ref not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn owned_vec_param_fires() {
        // Owned `Vec<T>` spells the element type in the cast.
        let out =
            propose_fix(RULE, "fn f(v: Vec<u32>) { assert!(v.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_eq!(v, [] as [u32; 0])"),
            "owned vec not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negated_fires_assert_ne() {
        let out = propose_fix(RULE, "fn f(s: &str) { assert!(!s.is_empty()); }").expect("matched");
        assert!(
            out.contains("assert_ne!(s, \"\")"),
            "negated not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn debug_assert_variants_fire() {
        // The `debug_` prefix carries across both polarities.
        let out = propose_fix(
            RULE,
            "fn f(s: &str) { debug_assert!(s.is_empty()); debug_assert!(!s.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("debug_assert_eq!(s, \"\")"),
            "debug assert not rewritten: {out}"
        );
        assert!(
            out.contains("debug_assert_ne!(s, \"\")"),
            "debug negated not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_bigger_fn_fires() {
        let out = propose_fix(
            RULE,
            "fn f(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(1); if s.is_empty() { assert!(s.is_empty()); } hits.len() }",
        )
        .expect("matched");
        assert!(out.contains("hits.push(1);"), "context lost: {out}");
        assert!(
            out.contains("assert_eq!(s, \"\")"),
            "nested assert not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn impl_method_param_fires() {
        let out = propose_fix(
            RULE,
            "struct S; impl S { fn f(&self, s: &str) { assert!(s.is_empty()); } }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s, \"\")"),
            "impl-fn assert not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn custom_message_refuses() {
        // Upstream silence: 2+ macro args carry a message — never ours.
        assert!(propose_fix(
            RULE,
            "fn f(v: Vec<u32>) { assert!(v.is_empty(), \"not empty: {v:?}\"); }",
        )
        .is_none());
    }

    #[test]
    fn method_chain_receiver_refuses() {
        // `s.trim().is_empty()` — `trim` is in no inherent table of this
        // file (a primitive's trait method — its return is unknowable).
        assert!(propose_fix(RULE, "fn f(s: &str) { assert!(s.trim().is_empty()); }",).is_none());
    }

    #[test]
    fn let_annotated_vec_fires() {
        // P98: an annotated local is a construction fact, exactly like a
        // param — the pin-bump wave (ff91fb5f) proved the demand (93 real
        // sites were locals, not params; this shape was the refusal that
        // caused the 93/93 proposer miss).
        let out = propose_fix(
            RULE,
            "fn f() { let v: Vec<u32> = Vec::new(); assert!(v.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(v, [] as [u32; 0])"),
            "annotated local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_call_free_fn_fires() {
        // `let v = g();` resolves through the file's own fn table.
        let out = propose_fix(
            RULE,
            "fn g() -> Vec<String> { todo!() } fn f() { let v = g(); assert!(v.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(v, [] as [String; 0])"),
            "fn-return local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_string_new_fires() {
        // The hardcoded std constructor: `String::new()` is the Str family.
        let out = propose_fix(
            RULE,
            "fn f() { let s = String::new(); assert!(s.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s, \"\")"),
            "String::new local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_string_from_fires() {
        // `String::from(..)` — the same hardcoded Str family.
        let out = propose_fix(
            RULE,
            "fn f() { let s = String::from(\"x\"); assert!(s.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s, \"\")"),
            "String::from local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_turbofish_vec_new_fires() {
        // `Vec::<T>::new()` spells T — the one sound turbofish.
        let out = propose_fix(
            RULE,
            "fn f() { let v = Vec::<u32>::new(); assert!(v.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(v, [] as [u32; 0])"),
            "turbofish local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_constructor_assoc_fires() {
        // The wave's dominant shape: `let set = ActiveSet::empty();` then
        // `set.caps().is_empty()` — the assoc call resolves the binding's
        // type name, the method call resolves the asserted type.
        let out = propose_fix(
            RULE,
            "struct S; impl S { fn new() -> S { todo!() } fn caps(&self) -> Vec<(String, u32)> { todo!() } } \
             fn f() { let s = S::new(); assert!(s.caps().is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s.caps(), [] as ["),
            "method receiver not rewritten: {out}"
        );
        assert!(out.contains("; 0])"), "cast missing: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn self_return_substitutes_fires() {
        // `-> Self` in an inherent impl substitutes to the self type name.
        let out = propose_fix(
            RULE,
            "struct S; impl S { fn make() -> Self { todo!() } fn caps(&self) -> Vec<String> { todo!() } } \
             fn f() { let s = S::make(); assert!(s.caps().is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s.caps(), [] as [String; 0])"),
            "Self substitution failed: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_receiver_param_fires() {
        // A param's named type resolves the method's return through the
        // file's inherent table.
        let out = propose_fix(
            RULE,
            "struct S; impl S { fn caps(&self) -> Vec<(String, u32)> { todo!() } } \
             fn f(s: S) { assert!(s.caps().is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s.caps(), [] as ["),
            "param method receiver not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_receiver_fires() {
        // `r.lints` — a one-hop field through the file's struct table.
        let out = propose_fix(
            RULE,
            "struct R { lints: Vec<String> } fn f(r: R) { assert!(r.lints.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(r.lints, [] as [String; 0])"),
            "field receiver not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn struct_literal_let_fires() {
        // `let r = R { … };` — the struct literal names the binding's type.
        let out = propose_fix(
            RULE,
            "struct R { lints: Vec<String> } fn f() { let r = R { lints: Vec::new() }; assert!(r.lints.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(r.lints, [] as [String; 0])"),
            "struct-literal local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn vecref_method_receiver_fires() {
        // A method returning `&Vec<T>` takes the re-spelled slice view.
        let out = propose_fix(
            RULE,
            "struct S; impl S { fn pairs(&self) -> &Vec<u32> { todo!() } } \
             fn f(s: S) { assert!(s.pairs().is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(s.pairs().as_slice(), [])"),
            "vec-ref method receiver not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn propagation_let_fires() {
        // `let w = v;` copies the proven fact one hop.
        let out = propose_fix(
            RULE,
            "fn g() -> Vec<String> { todo!() } fn f() { let v = g(); let w = v; assert!(w.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(w, [] as [String; 0])"),
            "propagated binding not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn negated_local_fires() {
        // Negation flips `_eq` to `_ne` on the new legs exactly as on
        // params.
        let out = propose_fix(
            RULE,
            "fn f() { let v: Vec<u32> = Vec::new(); assert!(!v.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_ne!(v, [] as [u32; 0])"),
            "negated local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mut_binding_method_calls_still_fire() {
        // `v.reserve(10)` is a method call, NOT a rebind — the census
        // stays at one binding and the assert still fires.
        let out = propose_fix(
            RULE,
            "fn f() { let mut v: Vec<u32> = Vec::new(); v.reserve(10); assert!(v.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(v, [] as [u32; 0])"),
            "mut local not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn const_item_receiver_fires() {
        // A const resolves through its declared type.
        let out = propose_fix(
            RULE,
            "const NAMES: Vec<String> = Vec::new(); fn f() { assert!(NAMES.is_empty()); }",
        )
        .expect("matched");
        assert!(
            out.contains("assert_eq!(NAMES, [] as [String; 0])"),
            "const receiver not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn shadowed_local_refuses() {
        // Two bindings of one name — no honest fact about either.
        assert!(propose_fix(
            RULE,
            "fn f() { let v: Vec<u32> = Vec::new(); let v: Vec<String> = Vec::new(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn reassigned_local_refuses() {
        // A re-`=` breaks the construction fact.
        assert!(propose_fix(
            RULE,
            "fn f() { let mut v: Vec<u32> = Vec::new(); v = Vec::new(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn infer_element_refuses() {
        // `Vec<_>` has no spellable element — the cast would not compile.
        assert!(propose_fix(
            RULE,
            "fn f() { let v: Vec<_> = Vec::new(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn vec_new_bare_refuses() {
        // `Vec::new()` without the turbofish: the element is unknown — the
        // exact under-inference clippy's own MaybeIncorrect form trips on.
        assert!(propose_fix(
            RULE,
            "fn f() { let v = Vec::new(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn generic_free_fn_refuses() {
        // A return type mentioning the fn's generic is not a concrete
        // fact.
        assert!(propose_fix(
            RULE,
            "fn g<T>() -> Vec<T> { todo!() } fn f() { let v = g(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn generic_impl_refuses() {
        // A generic impl's methods are not inherent facts for a name
        // lookup (and `S<u32>` is not a bare named type).
        assert!(propose_fix(
            RULE,
            "struct S<T> { x: Vec<u32> } impl<T> S<T> { fn caps(&self) -> Vec<u32> { todo!() } } \
             fn f(s: S<u32>) { assert!(s.caps().is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn cross_file_method_refuses() {
        // No inherent impl in the file — the return type is unknowable
        // (cross-file defs are never guessed).
        assert!(propose_fix(
            RULE,
            "struct S; fn f(s: S) { assert!(s.caps().is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn trait_method_receiver_refuses() {
        // `String::clone` is not in any inherent table of this file.
        assert!(propose_fix(RULE, "fn f(s: String) { assert!(s.clone().is_empty()); }",).is_none());
    }

    #[test]
    fn two_hop_chain_refuses() {
        // Exactly one hop is proven; `s.a().a()` is a chain the receiver
        // law refuses (the outer method's receiver is not a bare binding).
        assert!(propose_fix(
            RULE,
            "struct S; impl S { fn a(&self) -> S { todo!() } } fn f(s: S) { assert!(s.a().a().is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn ambiguous_names_refuses() {
        // Two same-named fns with different returns poison the name.
        assert!(propose_fix(
            RULE,
            "mod a { fn g() -> Vec<u32> { todo!() } } mod b { fn g() -> Vec<String> { todo!() } } \
             fn f() { let v = g(); assert!(v.is_empty()); }",
        )
        .is_none());
    }

    #[test]
    fn mut_vec_ref_refuses() {
        // `&mut Vec<u32>` — the mut reference refuses (conservative).
        assert!(propose_fix(RULE, "fn f(v: &mut Vec<u32>) { assert!(v.is_empty()); }",).is_none());
    }

    #[test]
    fn macro_expanded_receiver_refuses() {
        // A macro-expansion receiver is not a bare proven ident.
        assert!(propose_fix(RULE, "fn f() { assert!(mk!().is_empty()); }",).is_none());
    }

    #[test]
    fn comment_in_span_refuses() {
        assert!(
            propose_fix(RULE, "fn f(s: &str) { assert!(s /* keep */ .is_empty()); }",).is_none()
        );
    }

    #[test]
    fn hashmap_param_refuses() {
        // `&HashMap<K, V>` is not in the proven family map.
        assert!(propose_fix(
            RULE,
            "use std::collections::HashMap; fn f(m: &HashMap<u32, u32>) { assert!(m.is_empty()); }",
        )
        .is_none());
    }
}
