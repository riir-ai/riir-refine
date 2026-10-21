//! `clippy::map_unwrap_or` — `x.map(f).unwrap_or(d)` → `x.map_or(d, f)`, or
//! `is_some_and(<F>)` when the default is `false` and the closure provably
//! returns bool (real clippy 1.97's own suggestion for that shape — its
//! `map_or` output would be re-flagged by `unnecessary_map_or`, a style
//! lint, so the fused form is NOT the terminal state there).
//!
//! The `unwrap_or_else` arm (riir-train slice 8, 2026-08-18): real clippy
//! fires the SAME lint on `x.map(f).unwrap_or_else(g)` and suggests
//! `x.map_or_else(g, f)` — default closure FIRST, mapper SECOND. Both
//! evaluate both closures lazily, so the fused form preserves semantics
//! for ANY shapes. The bool-literal special cases do not apply (the
//! default is a closure, never a bool literal).
//!
//! `Option::map_or(default, f)` is the fused form of `map(f).unwrap_or(d)`:
//! both evaluate `d` eagerly and `f` lazily, so the rewrite preserves
//! semantics for ANY default/mapper shapes (no purity requirement).
//! Span-preserving (T6.4): the whole `unwrap_or`/`unwrap_or_else` call's
//! byte range is replaced by `<recv-of-map>.map_or(<D>, <F>)` (or
//! `map_or_else(<G>, <F>)`), each fragment being the ORIGINAL source text
//! of the matched node.
//!
//! Bool-default terminal states (clippy 1.93 probe, 2026-08-18 evening):
//! a provably-bool closure with `false`/`true` defaults emits the TERMINAL
//! form directly — `is_some_and(<f>)` / `is_none_or(<f>)`. The earlier
//! `true`-default SKIP was based on the two-step clippy chain
//! (`map_or(true, f)` → `is_none_or`), but emitting the terminal state
//! directly is one clean step (probe: both outputs lint-clean on 1.93;
//! `is_none_or` stable since 1.82). Closures whose bool-ness cannot be
//! proven syntactically (e.g. a bare path) keep the `map_or` form — a
//! parse-only limitation, not a wrong fix.
//!
//! Documented divergence (P9, 2026-08-25): a mapper whose TAIL casts to a
//! trait-object reference (`|f| f as &dyn Fn(..)`) is SKIPPED — real clippy
//! fires and suggests `map_or`, but the suggestion only compiles when the
//! default already has the trait-object type. `unwrap_or` accepts any
//! unsize-coercible default (`&fn(..)`, `&[T; N]`, fn items) because
//! `map(..)` fixed the Option parameter first; `map_or`'s U is inferred
//! from both args simultaneously and coercions cannot fire through it
//! (E0308, probe-verified on riir-ai ruliology/player.rs:463). The healer
//! stays silent there — SAFER than clippy's own autofix (the array-literal
//! guard class, second member).

use super::helpers::{bool_lit, byte_range, is_postfix_safe, macro_expr_args};
use super::receiver_kind::{classify_receiver, collect_fn_rets, ReceiverKind, ScopeKinds};
use super::{edit, src, SourceEdit};
use syn::visit::Visit;
use syn::{
    Expr, ExprMacro, ExprMethodCall, File, Local, Macro, Pat, Signature, Stmt, StmtMacro, Type,
};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
        scopes: Vec::new(),
        pinned: std::collections::HashSet::new(),
        fn_rets: collect_fn_rets(file),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// Enclosing-fn scope stack (param types + SAFE-LET classifications —
    /// the shared `receiver_kind` substrate since P15, 2026-09-14; the
    /// Issue 031 H1 enclosing-fn-stack pattern). Let-initializer
    /// classification means the born-lint class (`let best = ..find(..);
    /// best.map(f).unwrap_or(false)`) now reaches the terminal forms
    /// directly instead of emitting the `map_or` that `unnecessary_map_or`
    /// re-flags.
    scopes: Vec<ScopeKinds>,
    /// Byte ranges of `unwrap_or`/`unwrap_or_else` calls whose array-literal
    /// default sits in a COERCION-FORCED position (fn return tail with a
    /// declared `&[T]` return type, or a `let` initializer whose pattern
    /// annotation is `&[T]`). The P2 extension of the 2bf2a44 guard: in
    /// these positions the call site fixes `map_or`'s U from syntax we can
    /// read, so the unsize coercion fires and the rewrite compiles
    /// (probe-verified 2026-08-28 — see the guard in `try_map_or`).
    pinned: std::collections::HashSet<(usize, usize)>,
    /// Same-file fn name → Option/Result classification of its RETURN type
    /// (a fn whose return type is an `Option`/`Result` path, any arity) —
    /// built by the file-wide PRE-PASS (P15: forward references to later
    /// fns classify too; the v1 during-visit collection missed them).
    /// Foreign fns are absent → Unknown (conservative, unchanged).
    fn_rets: std::collections::HashMap<String, ReceiverKind>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.scopes.push(ScopeKinds::from_fn(
            &f.sig,
            &f.block,
            self.source,
            &self.fn_rets,
        ));
        self.note_fn_tail_pin(&f.sig, &f.block);
        syn::visit::visit_item_fn(self, f);
        self.scopes.pop();
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.scopes.push(ScopeKinds::from_fn(
            &f.sig,
            &f.block,
            self.source,
            &self.fn_rets,
        ));
        self.note_fn_tail_pin(&f.sig, &f.block);
        syn::visit::visit_impl_item_fn(self, f);
        self.scopes.pop();
    }

    /// Annotated-let initializer pin: `let q: &[T] = chain.unwrap_or(&[..]);`
    /// The annotation fixes U before the default is checked, so the
    /// unsize coercion fires (probe A3/A4, 2026-08-28). Direct initializer
    /// only — a chain nested inside a match/if inside the initializer keeps
    /// its pin (the annotation forces the whole expression's type by
    /// unification), but the emitter stays conservative here: direct shape
    /// only, the nested forms (`collider_pruner` class) stay guarded.
    fn visit_local(&mut self, l: &'ast Local) {
        if let (Pat::Type(pt), Some(init)) = (&l.pat, l.init.as_ref()) {
            if ty_is_slice_ref(&pt.ty) {
                if let Expr::MethodCall(m) = &*init.expr {
                    if is_unwrap_or_call(m) {
                        self.pinned.insert(two(byte_range(&*init.expr)));
                    }
                }
            }
        }
        syn::visit::visit_local(self, l);
    }

    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, m);
        let slice_pinned = self.pinned.contains(&two(byte_range(m)));
        if let Some(replacement) = try_map_or(
            self.source,
            m,
            self.innermost_scope(),
            Some(&self.fn_rets),
            slice_pinned,
        ) {
            self.edits.push(edit(m, replacement));
        }
    }

    /// Macro arguments are OPAQUE token streams to syn — the default
    /// visitor never descends into them, so `.map(f).unwrap_or_else(g)`
    /// inside a `writeln!`/`println!`/`format!` arg (the `bench_241` shape)
    /// is invisible to the method-call visit above. Real clippy matches
    /// there (post-expansion HIR); we parse the expression args of the
    /// known format-family macros + run the same matcher over them. The
    /// cloned tokens keep their file-relative spans, so edits collected
    /// on the parsed args slice the ORIGINAL source correctly.
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        syn::visit::visit_expr_macro(self, m);
        self.scan_macro_args(&m.mac);
    }

    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        syn::visit::visit_stmt_macro(self, m);
        self.scan_macro_args(&m.mac);
    }
}

impl Collector<'_> {
    /// Return-tail pin: a fn whose signature declares `-> &[T]` and whose
    /// block's DIRECT tail is an `unwrap_or`/`unwrap_or_else` call — the
    /// return type forces U (the five live return-tail sites:
    /// `dual_gram_pca_bridge` ×2, `vocab_clusters`, `freeze_thaw`, `fidelity`).
    fn note_fn_tail_pin(&mut self, sig: &Signature, block: &syn::Block) {
        if !sig_ret_is_slice_ref(sig) {
            return;
        }
        if let Some(Stmt::Expr(Expr::MethodCall(m), None)) = block.stmts.last() {
            if is_unwrap_or_call(m) {
                self.pinned.insert(two(byte_range(m)));
            }
        }
    }

    fn scan_macro_args(&mut self, mac: &Macro) {
        let scope = self.innermost_scope().cloned();
        for expr in macro_expr_args(mac) {
            scan_expr_for_matches(
                self.source,
                &mut self.edits,
                &expr,
                scope.as_ref(),
                Some(&self.fn_rets),
            );
        }
    }

    fn innermost_scope(&self) -> Option<&ScopeKinds> {
        self.scopes.last()
    }
}

/// Walk one macro-arg expression for `map(..).unwrap_or(_or_else)(..)`
/// matches. The lifetimes tie the AST borrow to the source/edits borrows
/// (all share the caller's frame); spans are file-relative by token
/// round-trip, so `src`/`edit` operate on the WHOLE-file source.
fn scan_expr_for_matches(
    source: &str,
    edits: &mut Vec<SourceEdit>,
    expr: &Expr,
    scope: Option<&ScopeKinds>,
    fn_rets: Option<&std::collections::HashMap<String, ReceiverKind>>,
) {
    struct Sub<'a> {
        source: &'a str,
        edits: &'a mut Vec<SourceEdit>,
        scope: Option<&'a ScopeKinds>,
        fn_rets: Option<&'a std::collections::HashMap<String, ReceiverKind>>,
    }
    impl<'ast> Visit<'ast> for Sub<'ast> {
        fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
            syn::visit::visit_expr_method_call(self, m);
            if let Some(replacement) = try_map_or(self.source, m, self.scope, self.fn_rets, false) {
                self.edits.push(edit(m, replacement));
            }
        }
    }
    let mut sub = Sub {
        source,
        edits,
        scope,
        fn_rets,
    };
    sub.visit_expr(expr);
}

/// `-> &[T]` / `-> &mut [T]` (paren-wrapped accepted). The coercion-forcing
/// return type for the P2 tail pin.
fn sig_ret_is_slice_ref(sig: &Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    ty_is_slice_ref(ty)
}

fn ty_is_slice_ref(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => matches!(&*r.elem, Type::Slice(_)),
        Type::Paren(p) => ty_is_slice_ref(&p.elem),
        _ => false,
    }
}

fn is_unwrap_or_call(m: &ExprMethodCall) -> bool {
    matches!(
        m.method.to_string().as_str(),
        "unwrap_or" | "unwrap_or_else"
    )
}

/// `byte_range` as a hashable pair (Range<usize> is not Hash).
fn two(r: std::ops::Range<usize>) -> (usize, usize) {
    (r.start, r.end)
}

/// `x.map(f).unwrap_or(d)` → `x.map_or(d, f)` / `is_some_and(f)`;
/// `x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)`.
fn try_map_or(
    source: &str,
    m: &ExprMethodCall,
    scope: Option<&ScopeKinds>,
    fn_rets: Option<&std::collections::HashMap<String, ReceiverKind>>,
    slice_pinned: bool,
) -> Option<String> {
    let else_form = if m.method == "unwrap_or" {
        false
    } else if m.method == "unwrap_or_else" {
        true
    } else {
        return None;
    };
    if !m.attrs.is_empty() || m.turbofish.is_some() {
        return None;
    }
    if m.args.len() != 1 {
        return None;
    }
    let Expr::MethodCall(map) = &*m.receiver else {
        return None; // receiver isn't a method call
    };
    if !map.attrs.is_empty() || map.turbofish.is_some() || map.method != "map" {
        return None;
    }
    if map.args.len() != 1 {
        return None;
    }
    let recv = src(source, &*map.receiver)?;
    let default = src(source, &m.args[0])?;
    let mapper = src(source, &map.args[0])?;
    // Array-literal defaults (`&[...]`, incl. `&[]`) must stay on the
    // `map(f).unwrap_or(&[...])` form: `map_or`'s U unification needs the
    // CALL SITE to force the type for the unsize coercion `&[T; N] ->
    // &[T]`. In inference-transparent positions (plain `let`, generic
    // args) rustc fixes U from the array literal and the closure's slice
    // return E0308s — probe-verified 2026-08-18: `map_or(&[], |m|
    // m.as_slice())` AND `map_or(&[1.0], ...)` both fail in `let`
    // position while both compile context-forced; the original always
    // compiles.
    //
    // P2 extension (2026-08-28): the call site CAN be read when the
    // position itself carries the forcing type — a fn return tail whose
    // signature declares `&[T]`, or a let initializer whose pattern
    // annotation is `&[T]` (`slice_pinned`, collected by the pre-pass
    // on the Collector). In those positions the rewrite compiles
    // (probe A2/A4) and real clippy `-W clippy::map_unwrap_or` fires on
    // the original shape (probe A1/A3), so the silence was pure guard
    // over-conservatism. Everything else stays skipped — including the
    // plain-let control, where clippy 1.97's own MachineApplicable
    // suggestion emits the E0308 form (we stay SAFER than clippy).
    if is_array_lit_default(&m.args[0], else_form) && !slice_pinned {
        return None;
    }
    // Trait-object-cast mappers (the P9 class, probe-verified 2026-08-25
    // on riir-ai ruliology/player.rs:463 — E0308 "expected `&fn(..) ->
    // f64`, found `&dyn Fn(..) -> f64`"): a mapper whose TAIL casts to a
    // trait-object reference (`|f| f as &dyn Fn(..)`) makes U a trait
    // object, and the default reaches it only through an UNSIZE coercion
    // (`&fn(..) -> &dyn Fn(..)`). `unwrap_or` performs that coercion —
    // `map(..)` has already fixed the Option's parameter, so the default's
    // expected type is known before it is checked. `map_or`'s U is a free
    // inference variable fed by BOTH args; a coercion cannot fire through
    // it, so the ascribed `&fn(..)` default wins U and the closure's
    // `&dyn Fn(..)` return is a hard E0308. The parse-only tail check is
    // precise (a dyn cast NOT in tail position doesn't type the closure's
    // return); the one safe default — an identical `as &dyn ..` cast — is
    // unprovable short of text equality, so skip the shape entirely
    // (joins the we-stay-silent class beside the array-literal guard
    // above; sibling of the 2bf2a44 E0308 probe).
    if let Expr::Closure(c) = &map.args[0] {
        if tail_casts_to_trait_object(&c.body) {
            return None;
        }
    }
    let recv = if is_postfix_safe(&map.receiver) {
        recv
    } else {
        format!("({recv})")
    };
    if else_form {
        // `x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)` — real
        // clippy's own suggestion for the else arm; default closure
        // FIRST, mapper SECOND. No bool-literal special cases (the
        // default is always a closure here).
        return Some(format!("{recv}.map_or_else({default}, {mapper})"));
    }
    let bool_default = bool_lit(&m.args[0]);
    let mapper_is_bool = closure_body_is_bool(&map.args[0]);
    if let Some(default) = bool_default {
        if mapper_is_bool {
            // Terminal forms fire only on PROVABLY-typed receivers:
            // `is_some_and`/`is_none_or` are Option-only, `is_ok_and` is
            // Result-only — the wrong pick is a COMPILE error (E0599, the
            // bench_440 env::var catch, 2026-08-18). Unknown receivers
            // fall back to the always-compiling `map_or` (one step short
            // of real clippy's terminal suggestion — the documented
            // SAFER-than-clippy divergence, same class as the type-blind
            // lint demotions).
            match classify_receiver(&recv, scope, fn_rets) {
                ReceiverKind::OptionLike => {
                    if default {
                        // clippy chain: `map_or(true, f)` → `is_none_or`
                        // (probe-verified lint-clean on 1.93).
                        return Some(format!("{recv}.is_none_or({mapper})"));
                    }
                    // Real clippy 1.93's own suggestion for this shape:
                    // the `map_or` output would be re-flagged by
                    // `unnecessary_map_or` (a default-group lint).
                    return Some(format!("{recv}.is_some_and({mapper})"));
                }
                ReceiverKind::ResultLike => {
                    if default {
                        // Real clippy is SILENT on Result + true default
                        // (probe 2026-08-18: no `is_err_or` suggestion —
                        // not stable); `map_or(true, f)` IS the terminal
                        // state for this receiver kind.
                        return Some(format!("{recv}.map_or(true, {mapper})"));
                    }
                    // Real clippy suggests `is_ok_and` directly for
                    // `Result.map(f).unwrap_or(false)` (probe-verified:
                    // skips the map_or intermediate entirely).
                    return Some(format!("{recv}.is_ok_and({mapper})"));
                }
                ReceiverKind::Unknown => {
                    // Cannot prove Option vs Result — emit the form that
                    // compiles on BOTH. May leave an `unnecessary_map_or`
                    // warning for Option receivers, but never breaks the
                    // build (compile-safety over terminal completeness).
                    return Some(format!("{recv}.map_or({default}, {mapper})"));
                }
            }
        }
    }
    Some(format!("{recv}.map_or({default}, {mapper})"))
}

/// True for `&[...]` / `&mut [...]` defaults (recursing through parens).
/// Casts are safe — `as` ascribes the target type, resolving the coercion
/// locally.
fn is_array_lit_ref(e: &Expr) -> bool {
    match e {
        Expr::Reference(r) => matches!(&*r.expr, Expr::Array(_)),
        Expr::Paren(p) => is_array_lit_ref(&p.expr),
        _ => false,
    }
}

/// Array-literal default check across both rewrite forms: direct for
/// `unwrap_or` (`&[...]`), inside the default closure's body for
/// `unwrap_or_else` (`|| &[...]`).
fn is_array_lit_default(arg: &Expr, else_form: bool) -> bool {
    if else_form {
        match arg {
            Expr::Closure(c) => is_array_lit_ref(&c.body),
            _ => false,
        }
    } else {
        is_array_lit_ref(arg)
    }
}

/// True when the expression's TAIL (its value — the closure's return type
/// when called on a closure body) is a cast to a trait-object type or a
/// (smart-)pointer/reference TO one: `f as &dyn Fn(..)`, `{ ..; f as *const dyn T }`.
/// Parenthesized tails and block tails (last stmt, no semi) recurse.
fn tail_casts_to_trait_object(body: &Expr) -> bool {
    match body {
        Expr::Paren(p) => tail_casts_to_trait_object(&p.expr),
        Expr::Block(b) => b.block.stmts.last().is_some_and(|s| match s {
            Stmt::Expr(e, _) => tail_casts_to_trait_object(e),
            _ => false,
        }),
        Expr::Cast(c) => ty_refs_trait_object(&c.ty),
        _ => false,
    }
}

/// Cast target is a bare trait object, or a reference/pointer to one.
fn ty_refs_trait_object(ty: &Type) -> bool {
    match ty {
        Type::TraitObject(_) => true,
        Type::Reference(r) => matches!(&*r.elem, Type::TraitObject(_)),
        Type::Ptr(p) => matches!(&*p.elem, Type::TraitObject(_)),
        Type::Paren(p) => ty_refs_trait_object(&p.elem),
        _ => false,
    }
}

/// Syntactically PROVE the expression is bool-typed. Conservative: only
/// shapes that are bool whenever the code compiles (comparisons, `!`,
/// `&&`/`||`, bool literals) plus a small allowlist of bool-returning method
/// names. Unknown shapes return false (the proposer then fires `map_or`,
/// which is real clippy's suggestion for everything it cannot see through).
fn expr_is_bool(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(&l.lit, syn::Lit::Bool(_)),
        Expr::Paren(p) => expr_is_bool(&p.expr),
        Expr::Unary(u) => matches!(u.op, syn::UnOp::Not(_)),
        Expr::Binary(b) => matches!(
            b.op,
            syn::BinOp::And(_)
                | syn::BinOp::Or(_)
                | syn::BinOp::Eq(_)
                | syn::BinOp::Ne(_)
                | syn::BinOp::Lt(_)
                | syn::BinOp::Le(_)
                | syn::BinOp::Gt(_)
                | syn::BinOp::Ge(_)
        ),
        Expr::MethodCall(m) => is_bool_method(&m.method),
        // `matches!(e, p)` evaluates to a bool by definition (post-expansion
        // it's an `is`-match expression) — found as live residue in katgpt
        // percepta's `.map(|d| matches!(d.kind, ..)).unwrap_or(true)`.
        Expr::Macro(mac) => mac.mac.path.is_ident("matches"),
        _ => false,
    }
}

fn is_bool_method(name: &syn::Ident) -> bool {
    let n = name.to_string();
    n.starts_with("is_")
        || matches!(
            n.as_str(),
            "contains"
                | "contains_key"
                | "starts_with"
                | "ends_with"
                | "any"
                | "all"
                | "eq"
                | "ne"
                | "lt"
                | "le"
                | "gt"
                | "ge"
        )
}

/// The mapper arg must be a closure whose body provably returns bool.
fn closure_body_is_bool(arg: &Expr) -> bool {
    let Expr::Closure(c) = arg else {
        return false;
    };
    expr_is_bool(&c.body)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn fuses_map_unwrap_or() {
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn chain_receiver_preserved() {
        let input = "fn f(v: Vec<u8>) -> u8 { v.first().map(|x| *x + 1).unwrap_or(9) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("v.first().map_or(9, |x| *x + 1)"),
            "chain receiver text not preserved: {out}"
        );
    }

    #[test]
    fn non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn other_method_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.filter(|v| *v > 0).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "filter receiver must not fire: {out:?}");
    }

    // Bool-default terminal states (Issue 027 probe class + 2026-08-18
    // evening re-probe on clippy 1.93; live residue in katgpt percepta):
    // - `.map(bool-f).unwrap_or(false)` → `is_some_and(f)` (the `map_or`
    //   output is re-flagged by the DEFAULT-group `unnecessary_map_or`);
    // - `.map(bool-f).unwrap_or(true)` → `is_none_or(f)` — the terminal
    //   form of clippy's two-step chain, emitted directly (the old skip
    //   left the original warned shape in place).
    // Array-literal defaults stay on the unwrap_or form (E0308 class,
    // probe-verified 2026-08-18 — see the guard comment in `try_map_or`).
    // The sibling katgpt-rs heal WIP broke on exactly this shape at
    // katgpt-forward/src/step.rs:196 (`map_or(&[], |m| m.as_slice())`).
    #[test]
    fn empty_array_lit_default_skipped() {
        let input =
            "fn f(m: Vec<Vec<f32>>) -> usize { m.get(1).map(|s| s.as_slice()).unwrap_or(&[]).len() }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "empty `&[]` default must not fire: {out:?}");
    }

    #[test]
    fn nonempty_array_lit_default_skipped() {
        let input =
            "fn f(m: Vec<Vec<f32>>) -> usize { m.first().map(|s| s.as_slice()).unwrap_or(&[1.0]).len() }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "non-empty `&[1.0]` default must not fire: {out:?}"
        );
    }

    #[test]
    fn or_else_array_lit_body_skipped() {
        let input =
            "fn f(m: Vec<Vec<f32>>) -> usize { m.get(1).map(|s| s.as_slice()).unwrap_or_else(|| &[]).len() }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "`|| &[]` default-closure body must not fire: {out:?}"
        );
    }

    // P2 extension (2026-08-28): array-literal defaults FIRE when the
    // position carries the coercion-forcing type — a fn return tail whose
    // signature declares `&[T]`, or an annotated-let initializer `let q:
    // &[T] = ...`. Probe evidence 2026-08-28: real clippy 1.97 `-W
    // clippy::map_unwrap_or` fires on both shapes, the rewritten `map_or`
    // compiles (A2/A4), and no default-group warning is created. The five
    // live sites: riir-engine dual_gram_pca_bridge ×2, riir-games-shared
    // vocab_clusters, katgpt-core freeze_thaw + breakeven/fidelity.
    #[test]
    fn return_tail_slice_pinned_fires() {
        let input = concat!(
            "pub fn eigenvalues(result: &Option<Vec<f32>>) -> &[f32] {\n",
            "    result\n",
            "        .as_ref()\n",
            "        .map(|r| r.as_slice())\n",
            "        .unwrap_or(&[])\n",
            "}\n"
        );
        let out = propose_fix("map_unwrap_or", input).expect("pinned tail must fire");
        assert!(
            out.contains(".as_ref().map_or(&[], |r| r.as_slice())"),
            "pinned return-tail rewrite not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn annotated_let_slice_pinned_fires() {
        let input =
            "fn f(m: std::collections::HashMap<usize, Vec<f32>>, k: usize) -> usize { let q: &[f32] = m.get(&k).map(|v| v.as_slice()).unwrap_or(&[]); q.len() }";
        let out = propose_fix("map_unwrap_or", input).expect("pinned let must fire");
        assert!(
            out.contains("let q: &[f32] = m.get(&k).map_or(&[], |v| v.as_slice());"),
            "pinned annotated-let rewrite not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn pinned_tail_requires_direct_tail() {
        // A `-> &[f32]` fn whose tail is `Some(chain.unwrap_or(&[]))` —
        // the pin is the DIRECT tail only; the Some(...) wrapper keeps the
        // position unforced (conservative).
        let input = concat!(
            "fn f<'a>(m: &'a Option<Vec<f32>>) -> Option<&'a [f32]> {\n",
            "    Some(m.as_ref().map(|r| r.as_slice()).unwrap_or(&[]))\n",
            "}"
        );
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "Some-wrapped tail must stay guarded: {out:?}"
        );
    }

    #[test]
    fn nested_in_annotated_let_still_skipped() {
        // The katgpt-band collider_pruner shape: the unwrap_or map chain
        // sits inside a match arm inside the annotated-let initializer —
        // NOT the direct init. Conservative: stays guarded (the pin is
        // direct-shape only).
        let input = concat!(
            "fn f<'a>(rows: &'a [Vec<f32>], idx: Option<usize>, z: &mut [&'a [f32]; 2]) {\n",
            "    let cols: &[&[f32]] = if idx.is_some() {\n",
            "        match idx { Some(i) => rows.get(i).map(|r| r.as_slice()).unwrap_or(&[]), None => &[] }\n",
            "    } else { &[] };\n",
            "    z[0] = cols;\n",
            "}"
        );
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "match-arm-nested chain must stay guarded: {out:?}"
        );
    }

    #[test]
    fn pinned_tail_inside_fn_body_stmt_still_skipped() {
        // A `-> &[f32]` fn whose unwrap_or chain sits in a NON-tail
        // statement (semicolon-terminated) is not forced by the return type
        // (the coercion target there is the let's inference variable).
        let input = concat!(
            "fn f(m: Vec<Vec<f32>>) -> &[f32] {\n",
            "    let q = m.first().map(|s| s.as_slice()).unwrap_or(&[]);\n",
            "    q\n",
            "}"
        );
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "plain-let inside a slice-returning fn must stay guarded: {out:?}"
        );
    }

    // Trait-object-cast mapper guard (the P9 class, 2026-08-25): the mapper's
    // tail casting to `&dyn ..` makes map_or's U a trait object, and the
    // default reaches it only via an unsize coercion that CANNOT fire through
    // map_or's free inference variable — the exact E0308 measured on
    // riir-ai ruliology/player.rs:463 (`expected &fn(..) -> f64, found
    // &dyn Fn(..) -> f64`). `unwrap_or` compiles the same shape because
    // `map(..)` fixes the Option parameter first. See the guard comment in
    // `try_map_or`.
    #[test]
    fn trait_object_cast_mapper_skipped() {
        // The player.rs production shape: default = `&(fn as fn-ptr)`,
        // mapper = `|f| f as &dyn Fn(..)`.
        let input = "fn f(p: Option<Box<dyn Fn(u8) -> u8>>, g: fn(u8) -> u8) -> u8 { p.as_ref().map(|f| f as &dyn Fn(u8) -> u8).unwrap_or(&(g as fn(u8) -> u8))(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "trait-object-cast mapper must not fire: {out:?}"
        );
    }

    #[test]
    fn trait_object_cast_block_tail_skipped() {
        let input = "fn f(p: Option<Box<dyn Fn(u8) -> u8>>, g: fn(u8) -> u8) -> u8 { p.as_ref().map(|f| { let _ = 1; f as &dyn Fn(u8) -> u8 }).unwrap_or(&(g as fn(u8) -> u8))(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "block-tail dyn cast must not fire: {out:?}");
    }

    #[test]
    fn or_else_trait_object_cast_mapper_skipped() {
        let input = "fn f(p: Option<Box<dyn Fn(u8) -> u8>>, g: fn(u8) -> u8) -> u8 { p.as_ref().map(|f| f as &dyn Fn(u8) -> u8).unwrap_or_else(|| &(g as fn(u8) -> u8))(0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(
            out.is_none(),
            "`map_or_else` shares U's inference — must not fire: {out:?}"
        );
    }

    #[test]
    fn concrete_cast_mapper_still_fires() {
        // Precision pin: casts to CONCRETE types are unaffected — both args
        // land on the same type, no coercion involved.
        let input = "fn f(x: Option<u8>) -> u64 { x.map(|v| v as u64).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v as u64)"),
            "concrete-cast mapper must still fuse: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_tail_dyn_cast_still_fires() {
        // Precision pin: a dyn cast NOT in tail position doesn't type the
        // closure's return — the rewrite is safe there.
        let input =
            "fn f(x: Option<u8>) -> i32 { x.map(|f| { let _ = f as &dyn Fn(); 1 }).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("map_or(0,"),
            "non-tail dyn cast must still fuse: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bool_closure_false_default_becomes_is_some_and() {
        let input =
            "fn f(x: Option<(bool, bool)>) -> bool { x.map(|s| !s.0 && !s.1).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|s| !s.0 && !s.1)"),
            "is_some_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn bool_closure_true_default_becomes_is_none_or() {
        let input =
            "fn f(x: Option<(bool, bool)>) -> bool { x.map(|s| s.0 || s.1).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_none_or(|s| s.0 || s.1)"),
            "is_none_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn contains_key_ref_pattern_closure_becomes_is_some_and() {
        // Live shape from katgpt percepta scheduler.rs:550 — the ref-pattern
        // closure `|&op| k.contains_key(&op)` over `Option<&T>`.
        let input = "fn f(x: Option<u32>, k: std::collections::HashMap<u32, u32>) -> bool { x.map(|&op| k.contains_key(&op)).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|&op| k.contains_key(&op))"),
            "is_some_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn matches_macro_closure_true_default_becomes_is_none_or() {
        // Live shape from katgpt percepta graph/types/mod.rs:1170 —
        // `.map(|d| matches!(d.kind, DimensionKind::Input)).unwrap_or(true)`.
        let input = "enum K { Input } fn f(x: Option<u32>) -> bool { x.map(|d| matches!(d, 1..=3)).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_none_or(|d| matches!(d, 1..=3))"),
            "is_none_or not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn comparison_closure_false_default_becomes_is_some_and() {
        let input = "fn f(x: Option<u8>) -> bool { x.map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|v| v > 3)"),
            "is_some_and not produced: {out}"
        );
    }

    // Turbofish receiver forms (2026-08-28, live residue in
    // riir-mmorpg-examples projector.rs:287 + game/tests.rs:1582):
    // `store.get::<ThinkTimer>(id)` — the plain `.get(` allowlist text never
    // matches `.get::<`, so the receiver classified Unknown and the terminal
    // was withheld, leaving `map_or` residue clippy flags as unnecessary.
    #[test]
    fn turbofish_get_receiver_becomes_is_some_and() {
        let input = "struct T(u8); fn f(store: &mut dyn Store, id: u32) -> bool { store.get::<T>(id).map(|t| t.0 > 0).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("store.get::<T>(id).is_some_and(|t| t.0 > 0)"),
            "turbofish .get terminal not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    // `.lock(` receivers (std Mutex/RwLock → LockResult = Result): 6
    // production sites in the same sweep (karma/party mirrors —
    // `.lock().map(bool-f).unwrap_or(false)` → clippy's own `is_ok_and`).
    #[test]
    fn lock_receiver_becomes_is_ok_and() {
        let input = "fn f(m: std::sync::Mutex<M>, local: u64) -> bool { m.lock().map(|g| g.is_partied(local)).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("m.lock().is_ok_and(|g| g.is_partied(local))"),
            "lock receiver terminal not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    // Precision pin: the turbofish variant must NOT flip a LATER known
    // shape's last-match-wins — `.get::<T>(k).ok()` still classifies by the
    // outermost `.ok()` (both OptionLike here; the pin guards ordering).
    #[test]
    fn turbofish_does_not_break_last_match_wins() {
        let input = "fn f(x: R) -> bool { x.get::<u8>(1).ok().map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.get::<u8>(1).ok().is_some_and(|v| v > 3)"),
            "ordering broken: {out}"
        );
    }

    // Result-receiver terminal states (bench_440 catch, 2026-08-18: the
    // heal emitted `is_some_and` on a `std::env::var` Result receiver —
    // E0599 compile break). Probe-verified on clippy 1.97: Result
    // receivers get `is_ok_and` directly; Result + true default is
    // clippy-SILENT (no stable `is_err_or`), so `map_or(true, f)` IS the
    // terminal state there. Unknown receivers (no param type, no known
    // shape) fall back to the always-compiling `map_or`.
    #[test]
    fn env_var_receiver_becomes_is_ok_and() {
        // Live shape from katgpt-core bench_440_lllg_paper_repro.rs:456.
        let input = "fn f() -> bool { std::env::var(\"LLLG_MULTISTEP\").map(|v| v == \"1\" || v.eq_ignore_ascii_case(\"true\")).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("std::env::var(\"LLLG_MULTISTEP\").is_ok_and(|v|"),
            "is_ok_and not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn parse_receiver_becomes_is_ok_and() {
        let input = "fn f() -> bool { \"5\".parse::<i32>().map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("\"5\".parse::<i32>().is_ok_and(|v| v > 3)"),
            "is_ok_and not produced: {out}"
        );
    }

    #[test]
    fn result_param_true_default_stays_map_or() {
        // Real clippy is SILENT on Result + true default (probe: no
        // `is_err_or` suggestion — not stable); `map_or(true, f)` is the
        // terminal state. Emitting `is_none_or` would be an E0599.
        let input = "fn f(x: Result<u8, u8>) -> bool { x.map(|v| v > 3).unwrap_or(true) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(true, |v| v > 3)"),
            "map_or(true, ..) not produced: {out}"
        );
    }

    #[test]
    fn result_param_false_default_becomes_is_ok_and() {
        let input = "fn f(x: Result<u8, u8>) -> bool { x.map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_ok_and(|v| v > 3)"),
            "is_ok_and not produced: {out}"
        );
    }

    #[test]
    fn unknown_receiver_falls_back_to_map_or() {
        // A receiver the classifier cannot prove Option vs Result (custom
        // fn call, no param type): the always-compiling `map_or` — never a
        // terminal form that could be an E0599 on the wrong receiver kind.
        let input = "fn f(k: u32) -> bool { lookup(k).map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("lookup(k).map_or(false, |v| v > 3)"),
            "map_or fallback not produced: {out}"
        );
        assert!(
            !out.contains("is_some_and"),
            "terminal form on unknown receiver must not be produced: {out}"
        );
    }

    #[test]
    fn same_file_fn_receiver_becomes_is_some_and() {
        // The build.rs:33 shape (recurred 2026-09-04; the riir-examples 08-19
        // miss class): a BARE-CALL receiver whose callee is a same-file fn
        // with an `-> Option<T>` return — the fn-return map proves the
        // receiver Option, so the terminal form replaces the `map_or` that
        // `unnecessary_map_or` re-flags.
        let input = concat!(
            "fn git_out(args: &[&str]) -> Option<String> { Some(String::new()) }\n",
            "fn build() -> bool { git_out(&[\"status\"]).map(|s| !s.is_empty()).unwrap_or(false) }",
        );
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("git_out(&[\"status\"]).is_some_and(|s| !s.is_empty())"),
            "is_some_and on same-file fn receiver not produced: {out}"
        );
    }

    #[test]
    fn same_file_result_fn_receiver_becomes_is_ok_and() {
        let input = concat!(
            "fn load(p: &str) -> Result<u8, u8> { Ok(1) }\n",
            "fn f(p: &str) -> bool { load(p).map(|v| v > 3).unwrap_or(false) }",
        );
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("load(p).is_ok_and(|v| v > 3)"),
            "is_ok_and on same-file Result fn receiver not produced: {out}"
        );
    }

    #[test]
    fn foreign_call_receiver_still_falls_back_to_map_or() {
        // The callee is NOT defined in the scanned file (no map entry) —
        // Unknown, conservative fallback unchanged (the pre-extension
        // behavior is the regression floor here).
        let input = "fn f(k: u32) -> bool { foreign_lookup(k).map(|v| v > 3).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("foreign_lookup(k).map_or(false, |v| v > 3)"),
            "map_or fallback not produced: {out}"
        );
        assert!(!out.contains("is_some_and"), "terminal leaked: {out}");
    }

    #[test]
    fn qualified_same_file_fn_receiver_becomes_is_some_and() {
        // `path::callee(..)` — the final `::` segment is the lookup key.
        let input = concat!(
            "mod git { pub fn out(args: &[&str]) -> Option<String> { None } }\n",
            "fn f() -> bool { git::out(&[\"s\"]).map(|s| !s.is_empty()).unwrap_or(false) }",
        );
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("git::out(&[\"s\"]).is_some_and(|s| !s.is_empty())"),
            "is_some_and on qualified fn receiver not produced: {out}"
        );
    }

    #[test]
    fn bool_method_closure_false_default_becomes_is_some_and() {
        let input =
            "fn f(x: Option<String>) -> bool { x.map(|s| s.contains('a')).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.is_some_and(|s| s.contains('a'))"),
            "is_some_and not produced: {out}"
        );
    }

    #[test]
    fn unprovable_closure_false_default_keeps_map_or() {
        // A bare-path body's bool-ness is invisible parse-only → the
        // conservative `map_or` form (real clippy's suggestion for what it
        // can see through type-independently).
        let input = "fn f(x: Option<bool>, b: bool) -> bool { x.map(|v| v).unwrap_or(false) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(false, |v| v)"),
            "map_or not produced: {out}"
        );
    }

    #[test]
    fn non_bool_default_with_bool_shaped_body_still_fuses() {
        // `0` is not a bool literal → the fused map_or form regardless of
        // the closure body's shape (type-mismatched bool body is impossible).
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or(0, |v| v + 1)"),
            "map_or not produced: {out}"
        );
    }

    // `unwrap_or_else` arm (riir-train slice 8): same lint, suggestion
    // `map_or_else(<default-closure>, <mapper>)` — arg ORDER is the
    // load-bearing fact (default first).
    #[test]
    fn fuses_map_unwrap_or_else_default_first() {
        let input = "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or_else(|| 0) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or_else(|| 0, |v| v + 1)"),
            "map_or_else not produced with default FIRST: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_else_chained_receiver_preserved() {
        // The bench_241 shape: a field-chain receiver under the map.
        let input = "fn f(s: S) -> String { s.opt.step.map(|v| v.to_string()).unwrap_or_else(|| \"none\".into()) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.opt.step.map_or_else(|| \"none\".into(), |v| v.to_string())"),
            "chained receiver text not preserved: {out}"
        );
    }

    #[test]
    fn or_else_non_map_receiver_unchanged() {
        let input = "fn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| 0) }";
        let out = propose_fix("map_unwrap_or", input);
        assert!(out.is_none(), "non-map receiver must not fire: {out:?}");
    }

    #[test]
    fn or_else_non_closure_default_still_fuses() {
        // A function-path default (`unwrap_or_else(Default::default)` shape)
        // is still the fused form — map_or_else takes any callable.
        let input =
            "fn f(x: Option<Vec<u8>>) -> Vec<u8> { x.map(|v| v.clone()).unwrap_or_else(Vec::new) }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("x.map_or_else(Vec::new, |v| v.clone())"),
            "map_or_else not produced for path default: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    // Macro-argument descent (riir-train slice 8, bench_241 shape): the
    // `.map(f).unwrap_or_else(g)` sites live inside `writeln!` args —
    // opaque tokens to syn's visitor, visible only via the format-family
    // macro scan.
    #[test]
    fn or_else_inside_writeln_args_fires() {
        let input = "fn f(o: impl std::fmt::Write, s: S) { \
            writeln!(o, \"| {} |\", s.step.map(|v| v.to_string()).unwrap_or_else(|| \"never\".into())).unwrap(); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.step.map_or_else(|| \"never\".into(), |v| v.to_string())"),
            "macro-arg match not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn or_else_multiline_writeln_chain_fires() {
        // The exact bench_241 shape: a multi-line field-chain receiver
        // inside a multi-line writeln! statement.
        let input = "fn f(w: W, s: S) { \
            writeln!( \
                w, \
                \"| Convergence | {} |\", \
                s.with.convergence_step \
                    .map(|st| format!(\"step {st}\")) \
                    .unwrap_or_else(|| \"never\".into()) \
            ).unwrap(); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("map_or_else(|| \"never\".into(), |st| format!(\"step {st}\"))"),
            "multiline macro-arg match not produced: {out}"
        );
        // The receiver text survives verbatim (newlines included).
        assert!(
            out.contains("s.with.convergence_step"),
            "receiver text not preserved: {out}"
        );
    }

    #[test]
    fn plain_map_unwrap_or_inside_println_fires() {
        let input = "fn f(s: S) { println!(\"{}\", s.opt.map(|v| v + 1).unwrap_or(0)); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.opt.map_or(0, |v| v + 1)"),
            "macro-arg map_or not produced: {out}"
        );
    }

    #[test]
    fn no_false_fire_on_clean_macro_args() {
        // A clean arg (no map/unwrap chain) + a named arg that doesn't
        // parse as an expression — neither fires, and the named arg does
        // not abort scanning of later args.
        let input = "fn f(s: S) { println!(\"{a} {b}\", a = 1, b = s.v.map(|x| x).unwrap_or(2)); }";
        let out = propose_fix("map_unwrap_or", input).expect("matched");
        assert!(
            out.contains("s.v.map_or(2, |x| x)"),
            "arg after a named arg not scanned: {out}"
        );
    }
}
