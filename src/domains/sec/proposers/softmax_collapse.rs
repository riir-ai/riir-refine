//! The `softmax-collapse` family (Issue 185, arXiv:2610.31642) — SUGGEST-ONLY
//! detectors for the f32 Softmax Collapse paste, plus the shared shape
//! helpers the mechanical sibling (`sigmoid_minus_one`) consumes.
//!
//! `grad = softmax − onehot` computes the target slot as `p_t − 1`, which
//! rounds to EXACTLY 0.0 in f32 once the logit margin passes the cliff
//! `24·ln2 ≈ 16.6`. Every non-target slot stays positive, so the row stops
//! summing to zero — under `AdamW` that is the paper's Slingshot instability.
//! `−ln p_t` reads 0 past the same cliff. The code is valid; clippy is
//! silent. riir-train Issue 645 fixed 7 sites; a census days later found
//! ~20 more across 6 repos, written three different ways.
//!
//! ## Why this lives in `sec` (Issue 185 domain decision)
//!
//! A numerics-CORRECTNESS defect in valid code is the `nan-unsafe-sort-
//! comparator` shape (a float defect, conservative gates, one mechanical
//! rewrite + suggest-only siblings), and that rule is already a `sec`
//! rule. `rust_perf`'s contract is cycles (wrong axis); `clippy_lints` heals
//! clippy's own findings (clippy cannot see this). A new domain would need
//! a `DomainId`, a router lane and a matcher overlay for three rules — the
//! substrate-first verdict is to consume `sec` and open one APPEND-ONLY
//! family (`SecFamily::Numerics`).
//!
//! ## The two detectors here (suggest-only — never an edit)
//!
//! | Rule | Shape | Why suggest-only |
//! |---|---|---|
//! | `softmax-onehot-collapse` | a softmax row `X` (a `softmax*` call result, or a row normalized by `/= sum` / `* inv_sum` / a `&mut X` scale-by-sum call) followed in the SAME fn by `X[t] -= 1.0`, `X[v] - if v == t { 1.0 } else { 0.0 }`, `x - f32::from(i == t)` over an element of `X`, or `X - onehot` | the repair restructures the pass (exclude the target from the LSE, write `−σ(m − z_t)` at the target) and the shapes vary — SIMD kernels, temperature, weighted one-hot, a per-repo primitive (`ce_collapse_free_into`) |
//! | `neg-ln-softmax-prob` | `-X[t].ln()`, `-(X[t].max(eps)).ln()`, `loss -= X[t].ln()` over a softmax row `X` | low severity: the loss READS 0 past the cliff (the gradient is the other rule); the repair is `softplus(m − z_t)` from the logits, which the row no longer carries |
//!
//! ## Gates (conservative — refuse more than fires)
//!
//! - **Row evidence first, in walk order**: a subtraction fires only on a
//!   row MARKED earlier in the same fn — never on a bare `x[t] -= 1.0`.
//!   `log_softmax`-named producers never mark (log space is immune), nor
//!   do `*loss*`/`*entropy*`/`*grad*`/`*backward*` producers (not a row).
//! - **Innermost fn**: a nested `fn` gets a fresh frame (closures share the
//!   enclosing fn's — the `.map(|(i, pi)| pi - f32::from(i == t))` form).
//! - **Tests are fixture habitat**: `#[test]` fns and `#[cfg(test)]` mods
//!   are skipped (the shipped repairs carry a "premise: the naive form
//!   collapses" assertion BY DESIGN). So are fns named `*collapse_free*`
//!   (the primitive itself).
//! - **Macro tokens are never parsed** (`assert!((p[t] - 1.0).abs() ..)`).
//!
//! Honest recall limits: a row normalized in a HELPER and passed in as a
//! parameter carries no evidence (cross-fn); a WEIGHTED one-hot
//! (`X[t] -= inv_l`, `X[t] -= w`) is out — only the literal `1.0`; a row
//! normalized by an un-sum-named divisor (`/= z`, `/= total`) is out.

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Expr, File, Pat, UnOp};

/// `softmax-onehot-collapse` (T2).
pub const RULE_SOFTMAX_ONEHOT: &str = "softmax-onehot-collapse";
/// `neg-ln-softmax-prob` (T3).
pub const RULE_NEG_LN: &str = "neg-ln-softmax-prob";

/// The T2 advice (single source, tested).
pub const ADVICE_ONEHOT: &str = "`p_t - 1` on a softmax row rounds to exactly 0.0 past the \
f32 margin cliff (~16.6) while every other p_i stays positive (Softmax Collapse, \
arXiv:2610.31642) — exclude the target from the LSE: m = logsumexp_{o!=t} z_o, \
grad[t] = -sigmoid(m - z_t), grad[i!=t] = sigmoid(m - z_t)*exp(z_i - m) (the \
`ce_collapse_free_into` form; Issue 185, suggest-only)";

/// The T3 advice (single source, tested).
pub const ADVICE_NEG_LN: &str = "`-ln p_t` over a softmax row reads 0 past the f32 margin \
cliff (~16.6) — compute the loss from the logits: softplus(m - z_t) with \
m = logsumexp_{o!=t} z_o (the `ce_collapse_free_into` loss; Issue 185, suggest-only, \
low severity)";

/// One detected collapse site (T2 or T3).
#[derive(Debug, Clone)]
pub struct CollapseFinding {
    /// [`RULE_SOFTMAX_ONEHOT`] or [`RULE_NEG_LN`].
    pub rule_id: &'static str,
    /// Byte range of the offending expression.
    pub span: Range<usize>,
    /// Its source text.
    pub text: String,
    /// The advice line.
    pub advice: &'static str,
}

/// Detect the two suggest-only shapes (no edits are produced, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<CollapseFinding> {
    let mut scan = Scan {
        source,
        findings: Vec::new(),
        frames: Vec::new(),
    };
    scan.visit_file(file);
    scan.findings
}

// ── Shared shape helpers (also consumed by `sigmoid_minus_one`) ──────────

/// The value of a float literal (`1.0`, `1.`, `1.0_f32`, `1f32`, `0.0`).
/// Unsuffixed integers are NOT floats (`x - 1` on a float does not compile).
pub(crate) fn float_lit(e: &Expr) -> Option<f64> {
    let Expr::Lit(l) = e else { return None };
    match &l.lit {
        syn::Lit::Float(f) => f.base10_parse::<f64>().ok(),
        syn::Lit::Int(i) if matches!(i.suffix(), "f32" | "f64") => i.base10_parse::<f64>().ok(),
        _ => None,
    }
}

pub(crate) fn is_float_one(e: &Expr) -> bool {
    float_lit(e) == Some(1.0)
}

/// The one-hot scalar at the target: `1.0`, or the MEAN-scaled `1.0 / n`
/// (`dl[t] -= 1.0 / l as f32` after `dl[v] = p[v] / l as f32`).
fn is_unit_onehot(e: &Expr) -> bool {
    match peel_paren(e) {
        Expr::Binary(b) if matches!(b.op, BinOp::Div(_)) => is_float_one(peel_paren(&b.left)),
        other => is_float_one(other),
    }
}

fn is_float_zero(e: &Expr) -> bool {
    float_lit(e) == Some(0.0)
}

/// `(e)` → `e`, any depth.
pub(crate) fn peel_paren(mut e: &Expr) -> &Expr {
    while let Expr::Paren(p) = e {
        e = &p.expr;
    }
    e
}

fn single_block_expr(b: &syn::Block) -> Option<&Expr> {
    match b.stmts.as_slice() {
        [syn::Stmt::Expr(e, None)] => Some(e),
        _ => None,
    }
}

/// An equality test (`a == b` / `a != b`) — the one-hot selector.
fn is_selector(e: &Expr) -> bool {
    matches!(peel_paren(e), Expr::Binary(b) if matches!(b.op, BinOp::Eq(_) | BinOp::Ne(_)))
}

/// A 0/1 one-hot scalar: `if c { 1.0 } else { 0.0 }` (or the swapped
/// arms) and `f32::from(c)` / `f64::from(c)`, with `c` an `==`/`!=` test.
pub(crate) struct OneHot<'a> {
    /// The selector expression, verbatim.
    pub cond: &'a Expr,
    /// The scalar is 1.0 when `cond` holds (false: the swapped arms).
    pub one_when_true: bool,
}

pub(crate) fn onehot_shape(e: &Expr) -> Option<OneHot<'_>> {
    match peel_paren(e) {
        Expr::If(ifx) => {
            let (_, else_e) = ifx.else_branch.as_ref()?;
            let Expr::Block(eb) = else_e.as_ref() else {
                return None; // `else if` chains are not a one-hot
            };
            let then_v = single_block_expr(&ifx.then_branch)?;
            let else_v = single_block_expr(&eb.block)?;
            if !is_selector(&ifx.cond) {
                return None;
            }
            let one_when_true = match (is_float_one(then_v), is_float_zero(else_v)) {
                (true, true) => true,
                _ if is_float_zero(then_v) && is_float_one(else_v) => false,
                _ => return None,
            };
            Some(OneHot {
                cond: &ifx.cond,
                one_when_true,
            })
        }
        Expr::Call(c) => {
            let Expr::Path(p) = c.func.as_ref() else {
                return None;
            };
            let segs: Vec<String> = p
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            let is_from = matches!(
                segs.as_slice(),
                [ty, f] if (ty == "f32" || ty == "f64") && f == "from"
            );
            if !is_from || c.args.len() != 1 {
                return None;
            }
            let arg = c.args.first()?;
            is_selector(arg).then_some(OneHot {
                cond: arg,
                one_when_true: true,
            })
        }
        _ => None,
    }
}

/// `#[test]` / `#[tokio::test]` / `#[bench]` / `#[cfg(test)]` on a fn.
pub(crate) fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let last = a.path().segments.last().map(|s| s.ident.to_string());
        matches!(last.as_deref(), Some("test" | "bench")) || is_cfg_test(a)
    })
}

/// `#[cfg(test)]` (and `all(test, ..)`) — never `cfg(not(test))`.
pub(crate) fn is_cfg_test(a: &syn::Attribute) -> bool {
    if !a.path().is_ident("cfg") {
        return false;
    }
    let ts = a.meta.to_token_stream().to_string();
    ts.contains("test") && !ts.contains("not")
}

pub(crate) fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs.iter().any(is_cfg_test)
}

/// The primitive's own name (`ce_collapse_free`, `ce_collapse_free_into`).
pub(crate) fn is_primitive_fn(name: &str) -> bool {
    name.contains("collapse_free")
}

// ── Row-key bookkeeping ──────────────────────────────────────────────────

/// The storage key an expression reads/writes: `&mut bctx.d_logits[..v]`
/// → `bctx.d_logits`, `probs.iter().enumerate()` → `probs`,
/// `d_logits[t][j]` → `d_logits`, `*g` → `g`.
fn root_key(e: &Expr) -> Option<String> {
    let mut e = e;
    loop {
        e = match e {
            Expr::Paren(p) => &p.expr,
            Expr::Reference(r) => &r.expr,
            Expr::Index(i) => &i.expr,
            Expr::MethodCall(m) => &m.receiver,
            Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)) => &u.expr,
            Expr::Path(p) => {
                return p.path.get_ident().map(ToString::to_string);
            }
            Expr::Field(f) => {
                let t: String = f
                    .to_token_stream()
                    .to_string()
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                return Some(t);
            }
            _ => return None,
        };
    }
}

fn pat_idents(p: &Pat, out: &mut Vec<String>) {
    match p {
        Pat::Ident(i) => out.push(i.ident.to_string()),
        Pat::Tuple(t) => t.elems.iter().for_each(|e| pat_idents(e, out)),
        Pat::TupleStruct(t) => t.elems.iter().for_each(|e| pat_idents(e, out)),
        Pat::Reference(r) => pat_idents(&r.pat, out),
        Pat::Type(t) => pat_idents(&t.pat, out),
        Pat::Paren(p) => pat_idents(&p.pat, out),
        _ => {}
    }
}

fn callee_name(e: &Expr) -> Option<String> {
    match e {
        Expr::Call(c) => match c.func.as_ref() {
            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        },
        Expr::MethodCall(m) => Some(m.method.to_string()),
        _ => None,
    }
}

/// A producer of a PROBABILITY row (never a log-prob, loss or gradient).
fn is_softmax_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("softmax")
        && ![
            "log", "loss", "entropy", "xent", "cross", "grad", "backward",
        ]
        .iter()
        .any(|w| n.contains(w))
}

/// A normalizer ident (`sum`, `sum_exp`, `inv_sum`, `denom`, `partition`).
fn is_norm_ident(e: &Expr) -> bool {
    let Expr::Path(p) = peel_paren(e) else {
        return false;
    };
    p.path.get_ident().is_some_and(|i| {
        let n = i.to_string().to_ascii_lowercase();
        ["sum", "denom", "partition"].iter().any(|w| n.contains(w))
    })
}

/// An inverse normalizer (`inv_sum`, `inv_denom`) — the `*`-form.
fn is_inv_norm_ident(e: &Expr) -> bool {
    is_norm_ident(e)
        && matches!(peel_paren(e), Expr::Path(p) if p.path.get_ident().is_some_and(|i| i.to_string().to_ascii_lowercase().contains("inv")))
}

/// Does `e` contain (anywhere, outside macros) a `softmax*` producer call?
fn mentions_softmax(e: &Expr) -> bool {
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr(&mut self, e: &'ast Expr) {
            if callee_name(e).is_some_and(|n| is_softmax_name(&n)) {
                self.0 = true;
            }
            syn::visit::visit_expr(self, e);
        }
    }
    let mut v = V(false);
    v.visit_expr(e);
    v.0
}

/// Does `e` contain a division by a normalizer (`.map(|x| x / sum)`)?
fn mentions_div_by_norm(e: &Expr) -> bool {
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
            if matches!(b.op, BinOp::Div(_)) && is_norm_ident(&b.right)
                || matches!(b.op, BinOp::Mul(_))
                    && (is_inv_norm_ident(&b.right) || is_inv_norm_ident(&b.left))
            {
                self.0 = true;
            }
            syn::visit::visit_expr_binary(self, b);
        }
    }
    let mut v = V(false);
    v.visit_expr(e);
    v.0
}

// ── The scan ─────────────────────────────────────────────────────────────

#[derive(Default)]
struct Frame {
    /// Keys proven to hold a softmax probability row (or scalar).
    rows: Vec<String>,
    /// Pattern ident → the key it iterates over (closure / `for` params).
    elem_of: Vec<(String, String)>,
}

struct Scan<'a> {
    source: &'a str,
    findings: Vec<CollapseFinding>,
    frames: Vec<Frame>,
}

impl Scan<'_> {
    fn frame(&mut self) -> Option<&mut Frame> {
        self.frames.last_mut()
    }

    /// Resolve an expression to its row key, following element aliases.
    fn resolve(&self, e: &Expr) -> Option<String> {
        let key = root_key(e)?;
        let f = self.frames.last()?;
        Some(
            f.elem_of
                .iter()
                .rev()
                .find(|(id, _)| *id == key)
                .map_or(key, |(_, k)| k.clone()),
        )
    }

    fn is_row(&self, e: &Expr) -> bool {
        let Some(f) = self.frames.last() else {
            return false;
        };
        self.resolve(e).is_some_and(|k| f.rows.contains(&k))
    }

    fn mark(&mut self, e: &Expr) {
        if let Some(k) = self.resolve(e) {
            if let Some(f) = self.frame() {
                if !f.rows.contains(&k) {
                    f.rows.push(k);
                }
            }
        }
    }

    fn bind_elems(&mut self, pat: &Pat, over: &Expr) {
        let Some(root) = root_key(over) else { return };
        // Resolve through an existing alias (nested iteration of an element).
        let root = self.resolve(over).unwrap_or(root);
        let mut ids = Vec::new();
        pat_idents(pat, &mut ids);
        if let Some(f) = self.frame() {
            for id in ids {
                f.elem_of.push((id, root.clone()));
            }
        }
    }

    fn push(&mut self, rule_id: &'static str, node: &Expr, advice: &'static str) {
        let span = node.span().byte_range();
        let Some(text) = self.source.get(span.clone()) else {
            return;
        };
        if self
            .findings
            .iter()
            .any(|f| f.span == span && f.rule_id == rule_id)
        {
            return;
        }
        self.findings.push(CollapseFinding {
            rule_id,
            text: text.to_string(),
            span,
            advice,
        });
    }

    fn with_frame(&mut self, name: &str, attrs: &[syn::Attribute], body: &syn::Block) {
        if is_test_fn(attrs) || is_primitive_fn(name) {
            return;
        }
        self.frames.push(Frame::default());
        self.visit_block(body);
        self.frames.pop();
    }

    /// The T3 operand: `X[t]`, `X[t].max(eps)`, `(X[t] + eps)`, `p` (an
    /// element alias or a marked scalar) — after peeling the clamps.
    fn is_prob_operand(&self, e: &Expr) -> bool {
        let mut e = peel_paren(e);
        loop {
            e = match e {
                Expr::MethodCall(m)
                    if matches!(m.method.to_string().as_str(), "max" | "min" | "clamp") =>
                {
                    peel_paren(&m.receiver)
                }
                Expr::Binary(b)
                    if matches!(b.op, BinOp::Add(_))
                        && float_lit(peel_paren(&b.right)).is_some() =>
                {
                    peel_paren(&b.left)
                }
                _ => break,
            };
        }
        match e {
            Expr::Index(_) | Expr::Path(_) => self.is_row(e),
            Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)) => self.is_row(e),
            _ => false,
        }
    }

    /// `R.ln()` with `R` a probability operand.
    fn is_ln_of_prob(&self, e: &Expr) -> bool {
        matches!(peel_paren(e), Expr::MethodCall(m)
            if m.method == "ln" && m.args.is_empty() && self.is_prob_operand(&m.receiver))
    }

    /// The left operand of a one-hot subtraction: an element of a row.
    fn is_row_element(&self, e: &Expr) -> bool {
        match peel_paren(e) {
            inner @ (Expr::Index(_) | Expr::Path(_)) => self.is_row(inner),
            inner @ Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)) => self.is_row(inner),
            _ => false,
        }
    }

    fn check_binary(&mut self, b: &syn::ExprBinary, node: &Expr) {
        match b.op {
            // ── marking: `X[i] /= sum`, `X[i] *= inv_sum`, `*p /= sum` ──
            BinOp::DivAssign(_) if is_norm_ident(&b.right) => self.mark(&b.left),
            BinOp::MulAssign(_) if is_inv_norm_ident(&b.right) => self.mark(&b.left),
            // ── T2 A: `X[t] -= 1.0` / T3: `loss -= X[t].ln()` ──
            BinOp::SubAssign(_) => {
                let lhs = peel_paren(&b.left);
                let lhs_is_slot = matches!(lhs, Expr::Index(_))
                    || matches!(lhs, Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)));
                if lhs_is_slot && is_unit_onehot(&b.right) && self.is_row(lhs) {
                    self.push(RULE_SOFTMAX_ONEHOT, node, ADVICE_ONEHOT);
                } else if self.is_ln_of_prob(&b.right) {
                    self.push(RULE_NEG_LN, node, ADVICE_NEG_LN);
                }
            }
            // ── T2 B/C: `p - onehot(v == t)` / `p - onehot_row` ──
            BinOp::Sub(_) => {
                let onehot_rhs = onehot_shape(&b.right).is_some()
                    || matches!(peel_paren(&b.right), Expr::Path(_) | Expr::Index(_))
                        && root_key(&b.right).is_some_and(|k| {
                            let n = k.to_ascii_lowercase();
                            n.contains("onehot") || n.contains("one_hot")
                        });
                if onehot_rhs && self.is_row_element(&b.left) {
                    self.push(RULE_SOFTMAX_ONEHOT, node, ADVICE_ONEHOT);
                }
            }
            _ => {}
        }
    }

    /// Marking by call: `softmax_into(&x, &mut row)`, `row.softmax_inplace()`,
    /// `simd_scale_inplace(&mut row, inv_sum)`.
    fn check_call_marks(&mut self, name: &str, args: &[&Expr], receiver: Option<&Expr>) {
        let n = name.to_ascii_lowercase();
        let softmax = is_softmax_name(name);
        let scales = ["scale", "div", "mul", "normaliz"]
            .iter()
            .any(|w| n.contains(w))
            && args.iter().any(|a| is_norm_ident(a));
        if !(softmax || scales) {
            return;
        }
        let muts: Vec<&Expr> = args
            .iter()
            .filter(|a| matches!(a, Expr::Reference(r) if r.mutability.is_some()))
            .copied()
            .collect();
        for m in muts {
            self.mark(m);
        }
        if softmax && (n.contains("inplace") || n.contains("in_place")) {
            if let Some(r) = receiver {
                self.mark(r);
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.with_frame(&f.sig.ident.to_string(), &f.attrs, &f.block);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.with_frame(&f.sig.ident.to_string(), &f.attrs, &f.block);
    }

    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if let Some(b) = &f.default {
            self.with_frame(&f.sig.ident.to_string(), &f.attrs, b);
        }
    }

    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        if is_test_mod(m) {
            return;
        }
        syn::visit::visit_item_mod(self, m);
    }

    fn visit_local(&mut self, l: &'ast syn::Local) {
        syn::visit::visit_local(self, l);
        let Some(init) = &l.init else { return };
        let e = peel_paren(&init.expr);
        // Alias: `let mut d = probs;` / `probs.clone()` / `probs.to_vec()`.
        let alias_src = match e {
            Expr::MethodCall(m)
                if matches!(
                    m.method.to_string().as_str(),
                    "clone" | "to_vec" | "to_owned"
                ) && m.args.is_empty() =>
            {
                Some(m.receiver.as_ref())
            }
            Expr::Path(_) | Expr::Reference(_) => Some(e),
            _ => None,
        };
        let produces = mentions_softmax(e)
            || matches!(e, Expr::MethodCall(_)) && mentions_div_by_norm(e)
            || alias_src.is_some_and(|a| {
                matches!(peel_paren(a), Expr::Path(_) | Expr::Reference(_)) && self.is_row(a)
            });
        if !produces || self.frames.is_empty() {
            return;
        }
        let mut ids = Vec::new();
        pat_idents(&l.pat, &mut ids);
        if let Some(f) = self.frame() {
            for id in ids {
                if !f.rows.contains(&id) {
                    f.rows.push(id);
                }
            }
        }
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        if self.frames.is_empty() {
            // Outside any fn body (consts, statics): nothing to track.
            return syn::visit::visit_expr(self, e);
        }
        match e {
            Expr::Binary(b) => self.check_binary(b, e),
            Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)) && self.is_ln_of_prob(&u.expr) => {
                self.push(RULE_NEG_LN, e, ADVICE_NEG_LN);
            }
            Expr::Assign(a) => {
                // `X[i] = e / sum` / `X[i] = e * inv_sum` marks X; so does a
                // copy of a row element, optionally mean-scaled
                // (`dl[i * v + j] = p[j] / l as f32`).
                let rhs = peel_paren(&a.right);
                let copies = match rhs {
                    Expr::Binary(rb) if matches!(rb.op, BinOp::Div(_) | BinOp::Mul(_)) => {
                        self.is_row_element(&rb.left)
                    }
                    other => self.is_row_element(other),
                };
                let normalizes = matches!(rhs, Expr::Binary(rb)
                    if matches!(rb.op, BinOp::Div(_)) && is_norm_ident(&rb.right)
                        || matches!(rb.op, BinOp::Mul(_))
                            && (is_inv_norm_ident(&rb.right) || is_inv_norm_ident(&rb.left)));
                if (normalizes || copies) && matches!(peel_paren(&a.left), Expr::Index(_)) {
                    self.mark(&a.left);
                }
            }
            Expr::Call(c) => {
                if let Some(name) = callee_name(e) {
                    let args: Vec<&Expr> = c.args.iter().collect();
                    self.check_call_marks(&name, &args, None);
                }
            }
            Expr::MethodCall(m) => {
                let args: Vec<&Expr> = m.args.iter().collect();
                self.check_call_marks(&m.method.to_string(), &args, Some(&m.receiver));
                // Closure params iterate the receiver chain's root row.
                for a in &m.args {
                    if let Expr::Closure(c) = a {
                        for p in &c.inputs {
                            self.bind_elems(p, &m.receiver);
                        }
                    }
                }
            }
            Expr::ForLoop(f) => self.bind_elems(&f.pat, &f.expr),
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(src: &str) -> Vec<CollapseFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    fn ids(src: &str) -> Vec<&'static str> {
        found(src).into_iter().map(|f| f.rule_id).collect()
    }

    // ── BEFORE spellings (each from a shipped repair) ──

    /// riir-rethink 2850291 (`ce_dlogits`): `pi - f32::from(i == gold)` over
    /// a closure element of a `softmax32` row.
    #[test]
    fn rethink_closure_f32_from_fires() {
        let src = "pub fn ce_dlogits(logits: &[f32], gold: usize) -> Vec<f32> {\n    let p = softmax32(logits);\n    p.iter()\n        .enumerate()\n        .map(|(i, pi)| pi - f32::from(i == gold))\n        .collect()\n}\n";
        assert_eq!(ids(src), vec![RULE_SOFTMAX_ONEHOT]);
    }

    /// katgpt-rs 4ba82db2f (mini-dLLM backward): the SIMD row scaled by
    /// `inv_sum`, then `d_logits[target] -= 1.0` on a FIELD key.
    #[test]
    fn katgpt_simd_scale_then_minus_one_fires() {
        let src = "fn backward(bctx: &mut Ctx, logits_p: &[f32], target: usize, vocab: usize) {\n    let max_l = simd_max_f32(logits_p);\n    bctx.d_logits[..vocab].copy_from_slice(logits_p);\n    simd_add_scalar_inplace(&mut bctx.d_logits[..vocab], -max_l);\n    simd_exp_inplace(&mut bctx.d_logits[..vocab]);\n    let sum_exp = simd_sum_f32(&bctx.d_logits[..vocab]);\n    let inv_sum = 1.0 / sum_exp;\n    katgpt_core::simd::simd_scale_inplace(&mut bctx.d_logits[..vocab], inv_sum);\n    bctx.d_logits[target] -= 1.0;\n}\n";
        assert_eq!(ids(src), vec![RULE_SOFTMAX_ONEHOT]);
    }

    /// riir-ai 8981315af (`set_diffusion_poc`): `/= sum_exp` loop, then the
    /// split one-hot AND the clamped `-=` ln loss.
    #[test]
    fn riir_ai_split_onehot_and_clamped_ln_fire() {
        let src = "fn train(logits: &[f32], probs: &mut [f32], target: usize, n: usize) -> f32 {\n    let mut total_loss = 0.0f32;\n    let mut sum_exp = 0.0f32;\n    for v in 0..n {\n        probs[v] = logits[v].exp();\n        sum_exp += probs[v];\n    }\n    for v in 0..n {\n        probs[v] /= sum_exp;\n    }\n    total_loss -= (probs[target].max(1e-7)).ln();\n    for v in 0..n {\n        let d_logit = probs[v] - if v == target { 1.0 } else { 0.0 };\n        let _ = d_logit;\n    }\n    total_loss\n}\n";
        let got = ids(src);
        assert_eq!(got, vec![RULE_NEG_LN, RULE_SOFTMAX_ONEHOT], "{got:?}");
    }

    /// riir-ai 8981315af (`AttentionModel`): the unparenthesized clamp.
    #[test]
    fn riir_ai_unparenthesized_clamp_fires() {
        let src = "fn t(logits: &[f32], probs: &mut [f32], target: usize) -> f32 {\n    let mut total_loss = 0.0;\n    let mut sum_exp = 0.0f32;\n    for v in 0..logits.len() { probs[v] = logits[v].exp(); sum_exp += probs[v]; }\n    for v in 0..logits.len() { probs[v] /= sum_exp; }\n    total_loss -= probs[target].max(1e-7).ln();\n    total_loss\n}\n";
        assert_eq!(ids(src), vec![RULE_NEG_LN]);
    }

    /// riir-refine 1b426b53 (`choice_scorer`): `-probs[label].ln()` then the
    /// aliased `dlogits[label] -= 1.0`; and the `loss()` neg-ln form.
    #[test]
    fn refine_choice_scorer_alias_and_neg_ln_fire() {
        let src = "impl S {\n    fn fb(&self, logits: &[f32], label: usize) -> f32 {\n        let probs = softmax(logits);\n        let loss = -probs[label].ln();\n        let mut dlogits = probs;\n        dlogits[label] -= 1.0;\n        loss\n    }\n    pub fn loss(&self, logits: &[f32], label: usize) -> f32 {\n        let p = softmax(logits);\n        -p[label].ln()\n    }\n}\n";
        let got = ids(src);
        assert_eq!(
            got,
            vec![RULE_NEG_LN, RULE_SOFTMAX_ONEHOT, RULE_NEG_LN],
            "{got:?}"
        );
    }

    #[test]
    fn softmax_into_out_param_and_onehot_row_fire() {
        let src = "fn g(z: &[f32], onehot: &[f32], out: &mut [f32]) {\n    let mut p = vec![0.0; z.len()];\n    softmax_into(z, &mut p);\n    for i in 0..z.len() {\n        out[i] = p[i] - onehot[i];\n    }\n}\nfn h(z: &[f32], onehot: f32) -> f32 {\n    let p = softmax(z);\n    p[0] - onehot\n}\n";
        assert_eq!(ids(src), vec![RULE_SOFTMAX_ONEHOT, RULE_SOFTMAX_ONEHOT]);
    }

    #[test]
    fn map_div_by_sum_row_and_for_deref_fire() {
        let src = "fn g(e: &[f32], t: usize) -> Vec<f32> {\n    let sum: f32 = e.iter().sum();\n    let mut p: Vec<f32> = e.iter().map(|x| x / sum).collect();\n    for (i, g) in p.iter_mut().enumerate() {\n        if i == t { *g -= 1.0; }\n    }\n    p\n}\n";
        assert_eq!(ids(src), vec![RULE_SOFTMAX_ONEHOT]);
    }

    /// riir-ai `binary_kernel_poc` (found by the T4 sweep): the row is
    /// COPIED mean-scaled into a second buffer, then the scaled one-hot.
    #[test]
    fn mean_scaled_copy_then_scaled_onehot_fires() {
        let src = "fn lg(logits: &[f32], clean: &[usize], vocab: usize) -> f32 {\n    let l = clean.len();\n    let mut probs = vec![0.0f32; vocab];\n    let mut dlogits = vec![0.0f32; l * vocab];\n    let mut loss = 0.0f32;\n    for i in 0..l {\n        softmax_row(logits, i, vocab, &mut probs);\n        loss -= probs[clean[i]].max(1e-12).ln();\n        for v in 0..vocab {\n            dlogits[i * vocab + v] = probs[v] / l as f32;\n        }\n        dlogits[i * vocab + clean[i]] -= 1.0 / l as f32;\n    }\n    loss\n}\n";
        assert_eq!(ids(src), vec![RULE_NEG_LN, RULE_SOFTMAX_ONEHOT]);
    }

    #[test]
    fn copy_into_a_scalar_or_non_unit_scaled_onehot_stays_silent() {
        // A scalar copy is not a row; `2.0 / l` is not a one-hot.
        let src = "fn g(z: &[f32], t: usize, l: f32, out: &mut [f32]) {\n    let p = softmax(z);\n    let mut a = 0.0;\n    a = p[t];\n    out[t] = p[t] / l;\n    out[t] -= 2.0 / l;\n    let _ = a;\n}\n";
        assert!(found(src).is_empty(), "{:?}", found(src));
    }

    // ── AFTER spellings (the shipped repairs) + the primitives stay silent ──

    #[test]
    fn primitive_ce_collapse_free_into_is_silent() {
        let src = "fn ce_collapse_free_into(logits: &[f32], target: usize, grad: &mut [f32]) -> f32 {\n    let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let mut sum_excl = 0.0f32;\n    for (v, (g, &z)) in grad.iter_mut().zip(logits).enumerate() {\n        *g = if v == target { 0.0 } else { (z - max_logit).exp() };\n        sum_excl += *g;\n    }\n    let gap = sum_excl.ln() + max_logit - logits[target];\n    let s = 1.0 / (1.0 + (-gap).exp());\n    let scale = s / sum_excl;\n    for g in grad.iter_mut() { *g *= scale; }\n    grad[target] = -s;\n    gap.max(0.0) + (-gap.abs()).exp().ln_1p()\n}\n";
        assert!(found(src).is_empty());
        // Even renamed, the body carries no collapse shape.
        let renamed = src.replace("ce_collapse_free_into", "excluded_lse_grad");
        assert!(found(&renamed).is_empty());
    }

    #[test]
    fn fixed_callers_are_silent() {
        let src = "impl S {\n    fn fb(&self, logits: &[f32], label: usize) -> f32 {\n        let (loss, dlogits) = ce_collapse_free(logits, label);\n        let _ = dlogits;\n        loss\n    }\n}\nfn train(logits: &[f32], probs: &mut [f32], target: usize) -> f32 {\n    let mut total_loss = 0.0;\n    total_loss += ce_collapse_free_into(logits, target, probs);\n    for v in 0..probs.len() {\n        let d_logit = probs[v];\n        let _ = d_logit;\n    }\n    total_loss\n}\nfn backward(bctx: &mut Ctx, logits_p: &[f32], target: usize, vocab: usize, max_l: f32) {\n    simd_exp_inplace(&mut bctx.d_logits[..vocab]);\n    bctx.d_logits[target] = 0.0;\n    let sum_excl = simd_sum_f32(&bctx.d_logits[..vocab]);\n    let s = ce_one_minus_p_target(sum_excl, max_l, logits_p[target]);\n    let scale = if sum_excl > 0.0 { s / sum_excl } else { 0.0 };\n    simd_scale_inplace(&mut bctx.d_logits[..vocab], scale);\n    bctx.d_logits[target] = -s;\n}\n";
        assert!(found(src).is_empty(), "{:?}", found(src));
    }

    #[test]
    fn premise_test_fns_and_cfg_test_mods_are_skipped() {
        let src = "#[test]\nfn premise() {\n    let p = softmax(&[20.0, 0.0]);\n    let g = p[0] - f32::from(0 == 0);\n    let _ = -p[0].ln();\n    let _ = g;\n}\n#[cfg(test)]\nmod tests {\n    fn helper() { let mut p = softmax(&[1.0]); p[0] -= 1.0; }\n}\n";
        assert!(found(src).is_empty());
    }

    #[test]
    fn unmarked_rows_and_log_space_are_silent() {
        let src = "fn a(x: &mut [f32], t: usize) { x[t] -= 1.0; }\nfn b(z: &[f32], t: usize) -> f32 {\n    let lp = log_softmax(z);\n    let g = lp[0] - if 0 == t { 1.0 } else { 0.0 };\n    g - lp[t]\n}\nfn c(z: &[f32], t: usize) -> f32 {\n    let l = softmax_cross_entropy(z, t);\n    -l.ln()\n}\nfn d(p: &[f32], t: usize) -> f32 { -p[t].ln() }\n";
        assert!(found(src).is_empty(), "{:?}", found(src));
    }

    #[test]
    fn weighted_onehot_and_non_one_literal_are_out_of_scope() {
        // Recall limit, pinned: `-= inv_l` (the kimi weighted form) and
        // `-= 0.5` never fire.
        let src = "fn k(z: &[f32], t: usize, inv_l: f32) -> Vec<f32> {\n    let mut p = softmax(z);\n    p[t] -= inv_l;\n    p[t] -= 0.5;\n    p\n}\n";
        assert!(found(src).is_empty());
    }

    #[test]
    fn nested_fn_gets_a_fresh_frame() {
        let src = "fn outer(z: &[f32]) {\n    let p = softmax(z);\n    fn inner(p: &mut [f32]) { p[0] -= 1.0; }\n    let _ = p;\n}\n";
        assert!(
            found(src).is_empty(),
            "the outer row does not license the inner fn"
        );
    }

    #[test]
    fn mark_must_precede_the_site() {
        let src = "fn g(x: &mut [f32], t: usize, sum: f32) {\n    x[t] -= 1.0;\n    for v in 0..x.len() { x[v] /= sum; }\n}\n";
        assert!(found(src).is_empty());
    }

    #[test]
    fn macro_tokens_are_never_parsed() {
        let src = "fn g(z: &[f32]) {\n    let p = softmax(z);\n    assert!((p[0] - 1.0).abs() < 1e-6);\n    debug_assert!(-p[0].ln() >= 0.0);\n}\n";
        assert!(found(src).is_empty());
    }

    #[test]
    fn corpus_fixtures_fire_buggy_and_stay_silent_fixed() {
        use crate::domain::HealDomain;
        use crate::domains::sec::SecDomain;
        let corpus = SecDomain::build_corpus();
        for rule in [RULE_SOFTMAX_ONEHOT, RULE_NEG_LN] {
            let r = corpus
                .iter()
                .find(|r| r.rule_id == rule)
                .expect("rule present");
            let hits: Vec<_> = found(&r.buggy_code)
                .into_iter()
                .map(|f| f.rule_id)
                .collect();
            assert_eq!(hits, vec![rule], "{rule} buggy fixture");
            assert!(
                found(&r.fixed_code).is_empty(),
                "{rule} fixed fixture must be silent"
            );
        }
    }

    #[test]
    fn advice_names_the_repair() {
        assert!(ADVICE_ONEHOT.contains("sigmoid(m - z_t)"));
        assert!(ADVICE_NEG_LN.contains("softplus"));
        let f = found("fn g(z: &[f32]) -> f32 { let p = softmax(z); -p[0].ln() }\n");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].text, "-p[0].ln()");
        assert_eq!(f[0].advice, ADVICE_NEG_LN);
    }
}
