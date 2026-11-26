//! `clippy::explicit_auto_deref` — an explicit deref that the ambient
//! auto-deref/deref-coercion would perform anyway: `f(&*e)` → `f(&e)` and
//! `f(&**e)` → `f(e)` (P51a, 2026-10-14; complexity group per the pinned
//! 1.98.1 driver; probe-verified against the driver's own suggestions —
//! BOTH shapes `MachineApplicable`, the deref-COUNT selects the rewrite:
//! one deref collapses to `&e`, two to `e`).
//!
//! ## The deref-count law
//!
//! The two production specimens (cycle 71, both healed by the driver's
//! suggestions): `bare_ident(&*m.receiver)` → `&m.receiver` (the receiver
//! is an owned `Box<Expr>` — one deref, keep the `&`) and
//! `self.visit_expr(&**else_e)` → `else_e` (`else_e` is a `&Box<Expr>`
//! match-ergonomics binding — two derefs, drop the whole `&**`). The
//! driver has type info to know WHICH collapse is right; this proposer
//! takes the deterministic syntactic law — count the derefs under the
//! reference and splice accordingly:
//!
//! - `&*e`  → `&e`   (one `*` deleted; `e`'s bytes verbatim)
//! - `&**e` → `e`    (the `&**` prefix deleted; `e`'s bytes verbatim)
//!
//! Any deeper chain (`&***e` — three derefs exist for `&&Box<T>`-class
//! operands) refuses: the collapse target stops being derivable from the
//! count alone at the shapes that did not fire upstream.
//!
//! ## Why it is safe when it compiles (the VERIFY bar)
//!
//! For a CONCRETE reference-taking callee the rewrite is a no-op
//! adjustment: the original `f(&*e)` compiling proves the callee's param
//! accepts `&Target(e::Deref)`; the rewrite passes `&e` (or `e`) and the
//! arg-position deref coercion walks the SAME one-step (or two-step)
//! Deref chain — same pointer, same borrow, no move of `e` itself (`&e`
//! borrows; `&**e`→`e` where the two-deref shape proves `e` is a
//! reference place — references are `Copy`).
//!
//! ## Documented divergences
//!
//! - **The generic-callee class (SILENT)**: `fn g<T>(x: T)` binds `T =
//!   &Target` for the original and `T = &e`-type for the rewrite — both
//!   compile, the callee receives a different type. Upstream's typeck
//!   never suggests there; a parse-only proposer cannot see the callee.
//!   Disclosed: the `--verify` compile gate catches every concrete-callee
//!   mis-fire (E0308) and this silent class is the residual risk, bounded
//!   by how rare `&*`/`&**` args to GENERIC callees are (the specimens
//!   were concrete-typed helpers).
//! - **Let-init positions refuse**: `let x: &u8 = &*b;` — the annotation
//!   makes the coercion site explicit and the rewrite would change the
//!   inferred binding type on UN-annotated lets (`let x = &*b` binds
//!   `&u8`, `let x = &b` binds `&Box<u8>` — silently different). Only the
//!   argument-position shape (call, method-call, and macro-expression
//!   args) fires — positions where a callee (and its coercion site)
//!   exists.
//! - Receiver position (`(*e).method()` — no `&`) is a DIFFERENT upstream
//!   shape and stays silent (v1 scope).
//! - `&mut *e` and any `&mut` form refuse (reborrow semantics differ).
//! - Attributes on the reference or a comment under the splice refuse
//!   (only `e`'s bytes survive).
//!
//! Interior-comment guard: the operand's bytes are the only survivors.
//!
//! **Owned-binding guard (i258):** `&*x → &x` is type-identity ONLY when
//! `x` is an OWNED value — when `x` is already a reference (`&mut T`),
//! `&x` re-references (`&&mut T`) while `&*x` reborrows (`&T`), and for
//! kernel-launch arg lists (`cudarc::LaunchArgs`) the difference is a
//! compile break on the non-macOS lane the mac gate never sees (measured:
//! `riir-train-gpu` E0277 ×6, fixed `740cbebc`). The collapse therefore
//! fires only on operands whose type is *syntactically owned or not
//! reference-shaped*: a field/index projection (whose base a fn param of
//! unknown type could still make ref-shaped — so bare-path operands
//! refuse, and projections through a BARE-path base refuse too), or a
//! bare path PROVEN owned by an owned `let` init (non-reference init) in
//! the same fn body. Fn params and `let g = &…` bindings are
//! reference-shaped by construction → refuse (refuse-on-uncertainty).

use syn::visit::Visit;
use syn::{Expr, File, Pat};

use super::helpers::{byte_range, drops_interior_comment};
use super::{SourceEdit, src};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let owned = owned_let_bindings(file);
    let mut v = Collector {
        source,
        owned: &owned,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    /// Names bound by an owned `let` (non-reference init) somewhere in the
    /// file. Deliberately FILE-scoped, not per-fn: a name proven owned in
    /// ANY fn cannot be a fn parameter in the fn we are editing (a param
    /// and a `let` of the same name in one fn is a shadow the rewrite
    /// cannot reach parse-only — the refusal there is the safe arm).
    owned: &'a std::collections::HashSet<String>,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'_> {
    /// Only ARGUMENT positions: the parent visit hands us every call /
    /// method-call; we inspect each argument for the reference-deref shape.
    /// (A top-down `visit_expr` on the argument itself cannot know it sits
    /// in argument position — the eligibility is the PARENT's property.)
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        for a in &c.args {
            self.try_argument(a);
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        for a in &m.args {
            self.try_argument(a);
        }
        // The receiver position is the DIFFERENT upstream shape — silent.
        syn::visit::visit_expr_method_call(self, m);
    }
}

impl Collector<'_> {
    fn try_argument(&mut self, arg: &Expr) {
        let Some(se) = self.try_collapse(arg) else {
            return;
        };
        self.edits.push(se);
    }

    /// `&*e` → `&e` / `&**e` → `e` on one argument expression.
    fn try_collapse(&self, arg: &Expr) -> Option<SourceEdit> {
        let Expr::Reference(r) = arg else {
            return None;
        };
        if r.mutability.is_some() || !r.attrs.is_empty() {
            return None;
        }
        let collapse = collapse_kind(&r.expr)?;
        // Peel EXACTLY `collapse` derefs to the surviving operand.
        let mut operand: &Expr = &r.expr;
        for _ in 0..collapse {
            let Expr::Unary(u) = operand else {
                return None;
            };
            operand = &*u.expr;
        }
        // i258 owned-binding guard: for kind 1 the collapse KEEPS a `&`, so
        // the surviving operand must not itself be reference-shaped — else
        // `&*x → &x` re-references (`&&mut T`, the cudarc LaunchArgs E0277
        // class). Bare paths refuse unless proven owned; projections
        // through unknown bases refuse on the uncertainty. Kind 2 drops the
        // whole `&**` so the operand survives bare — no re-reference.
        if collapse == 1 && !operand_owned_shaped(operand, self.owned) {
            return None;
        }
        let kept = [byte_range(operand)];
        if drops_interior_comment(self.source, byte_range(arg), &kept) {
            return None;
        }
        let text = src(self.source, operand)?;
        let replacement = match collapse {
            1 => format!("&{text}"),
            _ => text,
        };
        Some(SourceEdit {
            start: byte_range(arg).start,
            end: byte_range(arg).end,
            replacement,
        })
    }
}

/// Whether `operand` is syntactically safe to sit under the kept `&`:
/// owned by construction, or a bare path PROVEN owned by an owned `let`
/// init. Anything else (bare fn-param paths, projections through bare
/// paths) is reference-shaped-or-unknown → refuse.
fn operand_owned_shaped(operand: &Expr, owned: &std::collections::HashSet<String>) -> bool {
    match operand {
        Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
            owned.contains(&p.path.segments[0].ident.to_string())
        }
        // Tuple/paren/group/call constructions are owned values (literals
        // are Expr::Lit in syn). EVERYTHING else refuses: bare multi-segment
        // paths, `&*x.field` / `&*x[i]` / `&*x.f()` projections of an
        // unknown base (the base may be `&T`/`&mut T`, so the kept `&`
        // would re-project/re-reference), macros, casts — the uncertainty
        // arm is the guard's whole point (refuse-on-uncertainty, i258).
        Expr::Paren(_) | Expr::Group(_) | Expr::Tuple(_) | Expr::Call(_) | Expr::Lit(_) => true,
        _ => false,
    }
}

/// Names bound by `let` with a NON-reference init anywhere in the file.
/// `let g = &…` / `let g = &mut …` bindings are reference-shaped and are
/// deliberately excluded — `&*g → &g` on them would re-reference.
fn owned_let_bindings(file: &File) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    let mut v = OwnedLetScanner { set: &mut set };
    v.visit_file(file);
    set
}

struct OwnedLetScanner<'a> {
    set: &'a mut std::collections::HashSet<String>,
}

impl<'ast> Visit<'ast> for OwnedLetScanner<'_> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let Some(init) = &l.init {
            if !init_is_reference(&init.expr) {
                for id in pat_idents(&l.pat) {
                    self.set.insert(id);
                }
            }
        }
        syn::visit::visit_local(self, l);
    }
}

/// The init is a borrow expression (`&e` / `&mut e`) — the binding is
/// reference-shaped.
fn init_is_reference(e: &Expr) -> bool {
    matches!(e, Expr::Reference(_))
}

/// Every identifier bound by a pattern (tuple patterns included; ref
/// patterns keep the inner idents — the binding is a place, ownership
/// follows the INIT which we already checked).
fn pat_idents(p: &Pat) -> Vec<String> {
    let mut out = Vec::new();
    let mut v = PatIdentScanner { out: &mut out };
    v.visit_pat(p);
    out
}

struct PatIdentScanner<'a> {
    out: &'a mut Vec<String>,
}

impl<'ast> Visit<'ast> for PatIdentScanner<'_> {
    fn visit_pat_ident(&mut self, pi: &'ast syn::PatIdent) {
        self.out.push(pi.ident.to_string());
        syn::visit::visit_pat_ident(self, pi);
    }
}

/// How many derefs sit under the reference — 1 or 2 (deeper refuses).
fn collapse_kind(inner: &Expr) -> Option<u8> {
    let Expr::Unary(u1) = inner else {
        return None;
    };
    if !matches!(u1.op, syn::UnOp::Deref(_)) {
        return None;
    }
    if let Expr::Unary(u2) = &*u1.expr {
        if matches!(u2.op, syn::UnOp::Deref(_)) {
            // Three derefs refuse.
            if matches!(&*u2.expr, Expr::Unary(u3) if matches!(u3.op, syn::UnOp::Deref(_))) {
                return None;
            }
            return Some(2);
        }
    }
    Some(1)
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn single_deref_on_owned_let_fires_keeps_ref() {
        // The kind-1 collapse keeps its `&` — on an OWNED binding. (The
        // pre-i258 field-projection specimen `&*m.receiver` now refuses:
        // the operand's binding form is unprovable parse-only.)
        let input = "fn g() -> Option<u8> { let rx = make_rx(); bare_ident(&*rx) } fn bare_ident(e: &Expr) -> Option<u8> { None }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(
            out.contains("bare_ident(&rx)"),
            "one deref keeps the &: {out}"
        );
        assert!(!out.contains("&*"), "the deref is gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_projection_operand_now_refuses() {
        // i258: the OLD firing specimen is the refuse arm under the guard.
        let input = "fn g(m: &M) -> Option<u8> { bare_ident(&*m.receiver) } fn bare_ident(e: &Expr) -> Option<u8> { None }";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "unproven projection refuses: {out:?}");
    }

    #[test]
    fn double_deref_collapses_to_operand() {
        // Specimen 2 (block_scrutinee.rs, cycle 71): `&**else_e` — the
        // driver's suggestion was `else_e`. Kind 2 drops the WHOLE `&**`,
        // so the operand survives bare and the owned-binding guard does
        // not bind (no re-reference is possible).
        let input = "fn f(s: &S) { let Some(e) = &s.opt else { return }; visit(&**e) }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(
            out.contains("visit(e)"),
            "two derefs drop the whole &**: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn method_call_argument_fires() {
        let input = "fn f(m: &M) { self.visit_expr(&**m.else_e) }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(out.contains("self.visit_expr(m.else_e)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn let_init_position_refused() {
        // The inference-position divergence: `let x = &*b` binds &u8;
        // `let x = &b` would bind &Box<u8> — silently different.
        let input = "fn f(b: Box<u8>) { let x = &*b; takes(x) } fn takes(e: &u8) {}";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "let-init positions refuse: {out:?}");
    }

    #[test]
    fn mut_reference_refused() {
        let input = "fn f(b: &mut Box<u8>) { takes(&mut *b) } fn takes(e: &mut u8) {}";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(
            out.is_none(),
            "&mut forms refuse (reborrow semantics): {out:?}"
        );
    }

    #[test]
    fn three_deref_chain_refused() {
        let input = "fn f(r: &&Box<u8>) { takes(&***r) } fn takes(e: &u8) {}";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(
            out.is_none(),
            "chains deeper than two derefs refuse: {out:?}"
        );
    }

    #[test]
    fn receiver_position_refused() {
        let input = "fn f(b: Box<Vec<u8>>) { let n = (*b).len(); let _ = n; }";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(out.is_none(), "the receiver shape is v1-silent: {out:?}");
    }

    // ── i258: the owned-binding guard — the cudarc LaunchArgs break class ──

    #[test]
    fn ref_typed_fn_param_operand_refused() {
        // THE measured break: `grad_q: &mut CudaSlice<f32>` is a fn param;
        // `&*grad_q` reborrows (valid), `&grad_q` is `&&mut T` → E0277
        // (riir-train `740cbebc`). The old rewrite produced exactly this.
        let input = "fn f(grad_q: &mut [f32]) { launch.arg(&*grad_q); }";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(
            out.is_none(),
            "ref-typed fn-param operand must refuse: {out:?}"
        );
    }

    #[test]
    fn ref_shaped_let_binding_operand_refused() {
        // `let g = &pool;` binds `&T` — `&*g → &g` re-references.
        let input = "fn f(pool: [f32; 4]) -> i32 { let g = &pool; launch.arg(&*g); 0 }";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(
            out.is_none(),
            "reference-init let binding must refuse: {out:?}"
        );
    }

    #[test]
    fn owned_let_binding_operand_still_fires() {
        // Proven owned: `let owned = make();` (non-reference init) — the
        // rewrite is type-identity and stays sound.
        let input = "fn f() -> i32 { let owned = make(); launch.arg(&*owned); 0 }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(out.contains("launch.arg(&owned)"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn owned_literal_and_call_operands_still_fire() {
        let input = "fn f(x: u8) -> i32 { launch.arg(&*7u8); launch.arg(&*make()); 0 }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(out.contains("launch.arg(&7u8)"), "{out}");
        assert!(out.contains("launch.arg(&make())"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn owned_binding_guard_census_mixed_arm() {
        // The census arm: owned-let + literal + call fire; the field
        // projection in the SAME body refuses (per-site, not per-file).
        let input = "fn f(x: u8) -> i32 { let owned = make(); launch.arg(&*owned); launch.arg(&*7u8); launch.arg(&*make()); launch.arg(&*fielded.a); 0 }";
        let out = propose_fix("explicit_auto_deref", input).expect("matched");
        assert!(out.contains("launch.arg(&owned)"), "{out}");
        assert!(out.contains("launch.arg(&7u8)"), "{out}");
        assert!(out.contains("launch.arg(&make())"), "{out}");
        assert!(
            !out.contains("launch.arg(&fielded.a)"),
            "field projection must refuse: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn field_projection_operands_refuse_on_uncertainty() {
        // The pre-guard specimens fired on `&*m.receiver` (field of a param
        // whose type is unknown parse-only). The safe posture narrows the
        // rule: projections through unknown bases refuse — the specimen
        // class was lucky, not sound.
        let input = "fn g(m: &M) -> Option<u8> { bare_ident(&*m.receiver) } fn bare_ident(e: &Expr) -> Option<u8> { None }";
        let out = propose_fix("explicit_auto_deref", input);
        assert!(
            out.is_none(),
            "field projection through an unproven base must refuse: {out:?}"
        );
    }

    #[test]
    fn ref_typed_operand_break_is_the_triple_arm_shape() {
        // The four-arm discipline: the REAL cudarg break shape compiles
        // only in the refused direction. The fixed spelling (what the
        // lane landed, 740cbebc) keeps the reborrow byte-identical.
        let input = "fn f(grad_q: &mut [f32]) { launch.arg(&*grad_q); }";
        assert!(propose_fix("explicit_auto_deref", input).is_none());
        syn::parse_file(input).expect("the fixed spelling parses");
    }
}
