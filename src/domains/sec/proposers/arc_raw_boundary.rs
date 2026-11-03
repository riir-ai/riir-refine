//! arc-refcount-across-raw-boundary — SUGGEST-ONLY detector (Batch 325, the mozilla/uniffi-rs FFI-hygiene distill; Research 361 / Plan 327).
//!
//! A raw `Arc` pointer crossing an unsafe boundary — a waker-vtable data
//! pointer, an FFI handle — has exactly two custody regimes: BORROWED
//! (`Arc::increment_strong_count` + deref; the boundary keeps the count) or
//! CONSUMING (`Arc::from_raw` exactly once). The detector flags the three
//! mixed-regime shapes: (a) a `from_raw` result `mem::forget`-ed on a path
//! that also hands out another owner — double-own, ending as a double-free
//! or a permanent leak; (b) two consuming `from_raw` calls in one fn —
//! double-free; (c) an `increment_strong_count` stacked before a later
//! `from_raw` consume of the same pointer — over-count, permanent leak.
//! Cited class shape (never pasted): uniffi's `future.rs` waker vtable
//! borrows with `Arc::<Self>::increment_strong_count(ptr)` in the clone arm
//! (`:251`) and consumes with `Arc::<Self>::from_raw(ptr)` in the wake/drop
//! arm (`:288`); `handle.rs` `into_arc_borrowed` increments then `from_raw`s
//! — the two regimes meeting in one handle.
//!
//! ## Why suggest-only (never a fix)
//!
//! Which regime applies is the PROTOCOL's answer — whether the callee may
//! drop the pointer is written in the boundary contract (a waker's clone
//! borrows, its wake consumes), not derivable from the syntax. A mechanical
//! fix would have to invent the vtable's semantics, and a wrong one
//! double-frees faster than the bug it replaces. The rule flags the
//! QUESTION: NO `FIXABLE_SEC` membership, NO `collect_edits` arm — the
//! suggestion surfaces through retrieval and reviewer read.
//!
//! ## The bounded form
//!
//! ```text
//! // borrowed arm — hand out another count, deref; the boundary keeps custody
//! unsafe { Arc::increment_strong_count(ptr) };
//! let arc: &Arc<T> = unsafe { &*ptr };
//! // ... the pointer stays owned by the boundary — no from_raw, no forget
//!
//! // consume arm — take ownership exactly once; the Arc's own drop retires it
//! let arc: Arc<T> = unsafe { Arc::from_raw(ptr) }; // exactly once
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! A finding requires ALL of:
//!
//! - **Raw-pointer signal**: the fn has a `*const`/`*mut` parameter, or a
//!   `from_raw` argument is a bare path variable (`ptr`, not `ptr.cast()`).
//! - **Gate (a) — `into_raw` suppresses EVERYTHING**: any `Arc::into_raw` in
//!   the same fn makes it a custody-transfer site — the legitimate
//!   ownership round-trip — and the whole fn is skipped.
//! - **Gates (b)+(c) — comment/macro guard**: `#[cfg(test)]` mods are
//!   skipped (house discipline); macro tokens are never descended into.
//! - **Gate (d) — `ManuallyDrop::new` anywhere in the fn** suppresses
//!   `ForgetAfterFromRaw` and `DoubleFromRaw` for that fn — the custody
//!   marker is already the answer.
//! - **`ForgetAfterFromRaw`**: a from_raw-bound variable is later passed to
//!   `mem::forget` AND the fn carries `increment_strong_count` anywhere,
//!   OR the variable has ≥2 ownership events (`from_raw` binds of it +
//!   increments whose bare argument is it — the bind is one event, the
//!   forget is the trigger and is never counted). ONE bind + ONE forget
//!   with no other events is the round-trip/benign-forget shape — SILENT
//!   (the FP control).
//! - **`DoubleFromRaw`**: ≥2 `from_raw` call sites in one fn with no
//!   `ManuallyDrop` anywhere in it — the conservative text-level proxy;
//!   one finding per fn, reported on the second site.
//! - **`IncrementBeforeFromRaw`**: an `increment_strong_count` whose bare
//!   argument variable a LATER `from_raw` in the same fn also consumes as
//!   its bare argument (walk-order ordinal: increment < consume).
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Per-fn boundary only**: the correct uniffi split (increment in the
//!   clone arm, `from_raw` in the wake/drop arm) is a TWO-fn protocol —
//!   cross-fn custody is unconnected here and stays silent.
//! - **`Weak::from_raw` is out of scope** — `Arc` paths only.
//! - **Only `let` binds register custody variables**:
//!   `x = Arc::from_raw(..)` re-assignment and destructuring binds are
//!   untracked (silent).
//! - **Method-chain arguments carry no variable** (`ptr.cast()`) — the
//!   increment/consume pairing and the raw-signal arm skip them.
//! - **Closure bodies attribute to the enclosing fn** — a custody mix
//!   inside a closure reports on the outer fn's name (per-fn granularity).
//! - **Benign single consume stays silent**: one `from_raw`, consumed once,
//!   no forget, no increment — the common correct case never fires.
//! - **Mutually exclusive branches fire too**: the `DoubleFromRaw` proxy is
//!   text-level (one `from_raw` per `if`/`match` arm is the same two-site
//!   count) — review decides (suggest-only is the blast-radius control).
//!
//! ## Distinct-from
//!
//! - `rc-backedge-field-prefer-weak` owns the strong-`Rc` backedge CYCLE
//!   (a struct-shape rule); this owns raw-pointer custody AT an unsafe
//!   boundary — a use-site regime, not a field type.
//! - `ptr-deref-null-check-first` owns null-deref UB (dereference
//!   validity); this owns ownership/refcount custody — a null-checked
//!   pointer can still be double-consumed.
//!
//! Batch 325 / Plan 327 / Research 361 (mozilla/uniffi-rs @
//! ef9625b68dd51207f55f024a27649e58edb52b09, MPL-2.0 — cited as class
//! provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Block, Expr, ExprCall, File, FnArg, ImplItem, Item, Pat, Path, Signature, Stmt, Type};

/// One detected raw-`Arc` custody question at an unsafe boundary
/// (suggest-only; this module never produces an edit).
#[derive(Debug, Clone)]
pub struct ArcRawBoundaryFinding {
    /// The enclosing function's name (custody events are scoped per fn).
    pub fn_name: String,
    /// Line of the offending site: the `forget` call, the second
    /// `from_raw`, or the over-counted `from_raw` consume (0 when the
    /// span is out of range — the foreign-parse guard).
    pub line: usize,
    /// Which of the three buggy custody shapes fired.
    pub kind: ArcRawBuggyKind,
    /// The offending call's source text (empty past the foreign-parse
    /// guard — a foreign span is never sliced).
    pub site_text: String,
    /// The advice line (the two-regime custody law).
    pub advice: &'static str,
}

/// The three buggy custody shapes across a raw `Arc` boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcRawBuggyKind {
    /// A `from_raw` result `mem::forget`-ed on a path that also hands out
    /// another owner — double-own (double-free or permanent leak).
    ForgetAfterFromRaw,
    /// Two consuming `from_raw` calls in one fn — double-free.
    DoubleFromRaw,
    /// `increment_strong_count` stacked before a later `from_raw` consume
    /// of the same pointer — over-count (permanent leak).
    IncrementBeforeFromRaw,
}

impl ArcRawBuggyKind {
    /// The one-line review question this kind asks.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::ForgetAfterFromRaw => {
                "forget-after-from_raw: a borrowed Arc raw pointer was consumed as \
                 owned and forgotten — double-own; the borrowed arm increments and \
                 derefs, it never from_raw+forgets"
            }
            Self::DoubleFromRaw => {
                "two from_raw consumes in one fn: the same custody taken twice — \
                 double-free; from_raw must be exactly once"
            }
            Self::IncrementBeforeFromRaw => {
                "increment_strong_count stacked before a from_raw consume of the \
                 same pointer — over-count; the consume arm needs no increment"
            }
        }
    }
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a raw Arc pointer crossing an unsafe boundary has exactly two \
custody regimes: BORROWED (`Arc::increment_strong_count(ptr)` + deref — the boundary \
keeps the count) or CONSUMING (`Arc::from_raw` exactly once). Mixing them (from_raw + \
forget, a second from_raw, an increment stacked on a from_raw consume) double-frees or \
leaks the count — pick the regime the boundary protocol names and write only that arm \
(Batch 325, suggest-only)";

/// Detect raw-`Arc` custody bugs across unsafe boundaries (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ArcRawBoundaryFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        walk_item(source, item, &mut findings);
    }
    findings
}

#[derive(Debug)]
struct FnScan {
    name: String,
    from_raw: Vec<FromRawSite>,
    increments: Vec<IncrementSite>,
    forgets: Vec<ForgetRequest>,
    has_into_raw: bool,
    has_manuallydrop: bool,
    has_ptr_param: bool,
    ordinal: usize,
    pending_bind: Option<String>,
}

#[derive(Debug)]
struct FromRawSite {
    line: usize,
    site_text: String,
    arg_var: Option<String>,
    bind_var: Option<String>,
    ordinal: usize,
}

#[derive(Debug)]
struct IncrementSite {
    arg_var: Option<String>,
    ordinal: usize,
}

#[derive(Debug)]
struct ForgetRequest {
    line: usize,
    site_text: String,
    var: Option<String>,
}

impl FnScan {
    fn new(name: String, has_ptr_param: bool) -> Self {
        Self {
            name,
            from_raw: Vec::new(),
            increments: Vec::new(),
            forgets: Vec::new(),
            has_into_raw: false,
            has_manuallydrop: false,
            has_ptr_param,
            ordinal: 0,
            pending_bind: None,
        }
    }

    /// Record a path-call event when it is one of the custody ops. Macro
    /// tokens never reach here (gate c).
    fn note_call(&mut self, source: &str, call: &ExprCall) {
        let Some(path) = call_path(call) else {
            return;
        };
        let Some(last) = path.segments.last() else {
            return;
        };
        match last.ident.to_string().as_str() {
            "from_raw" if seg_any(path, "Arc") => {
                self.ordinal += 1;
                let (line, site_text) = call_site(source, call);
                self.from_raw.push(FromRawSite {
                    line,
                    site_text,
                    arg_var: call.args.first().and_then(path_var_name),
                    bind_var: self.pending_bind.take(),
                    ordinal: self.ordinal,
                });
            }
            "increment_strong_count" if seg_any(path, "Arc") => {
                self.ordinal += 1;
                self.increments.push(IncrementSite {
                    arg_var: call.args.first().and_then(path_var_name),
                    ordinal: self.ordinal,
                });
            }
            "new" if seg_any(path, "ManuallyDrop") => self.has_manuallydrop = true,
            "forget" if seg_any(path, "mem") => {
                let (line, site_text) = call_site(source, call);
                self.forgets.push(ForgetRequest {
                    line,
                    site_text,
                    var: call.args.first().and_then(path_var_name),
                });
            }
            "into_raw" if seg_any(path, "Arc") => self.has_into_raw = true,
            _ => {}
        }
    }

    /// Apply the fire rules — every suppression gate first (refuse more
    /// than fires).
    fn finish(&self, findings: &mut Vec<ArcRawBoundaryFinding>) {
        // Gate (a): an into_raw in the same fn is the legitimate
        // ownership round-trip — the whole fn is a custody-transfer site.
        if self.has_into_raw {
            return;
        }
        // The raw-pointer signal: a raw ptr param, or a from_raw whose
        // argument is a bare path variable.
        let raw_signal = self.has_ptr_param || self.from_raw.iter().any(|s| s.arg_var.is_some());
        if !raw_signal {
            return;
        }
        let has_increment = !self.increments.is_empty();

        // Rule 1 — ForgetAfterFromRaw (gate d suppresses): a from_raw-bound
        // variable reaches mem::forget.
        if !self.has_manuallydrop {
            for var in self.bound_vars() {
                let Some(forget) = self.forgets.iter().find(|f| f.var.as_deref() == Some(var))
                else {
                    continue;
                };
                // Ownership events EXCLUDING the trigger forget: the bind
                // is one, an increment whose bare argument is the var is
                // another, a second bind is another. One bind + one forget
                // alone is the round-trip/benign-forget shape — silent
                // (the FP control).
                let binds = self
                    .from_raw
                    .iter()
                    .filter(|s| s.bind_var.as_deref() == Some(var))
                    .count();
                let increments_on_var = self
                    .increments
                    .iter()
                    .filter(|i| i.arg_var.as_deref() == Some(var))
                    .count();
                if binds + increments_on_var >= 2 || has_increment {
                    findings.push(self.finding(
                        forget.line,
                        forget.site_text.clone(),
                        ArcRawBuggyKind::ForgetAfterFromRaw,
                    ));
                }
            }
        }

        // Rule 2 — DoubleFromRaw (gate d suppresses): the conservative
        // text-level proxy fires once per fn, on the second site.
        if !self.has_manuallydrop && self.from_raw.len() >= 2 {
            let second = &self.from_raw[1];
            findings.push(self.finding(
                second.line,
                second.site_text.clone(),
                ArcRawBuggyKind::DoubleFromRaw,
            ));
        }

        // Rule 3 — IncrementBeforeFromRaw: the increment's bare argument
        // variable is later consumed by from_raw as its bare argument. NOT
        // suppressed by gate d — a custody marker does not uncount.
        for inc in &self.increments {
            let Some(var) = inc.arg_var.as_deref() else {
                continue;
            };
            for site in &self.from_raw {
                if site.arg_var.as_deref() == Some(var) && site.ordinal > inc.ordinal {
                    findings.push(self.finding(
                        site.line,
                        site.site_text.clone(),
                        ArcRawBuggyKind::IncrementBeforeFromRaw,
                    ));
                }
            }
        }
    }

    /// The distinct from_raw-bound variables, in first-bind order.
    fn bound_vars(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for site in &self.from_raw {
            if let Some(v) = site.bind_var.as_deref() {
                if !seen.contains(&v) {
                    seen.push(v);
                }
            }
        }
        seen
    }

    fn finding(
        &self,
        line: usize,
        site_text: String,
        kind: ArcRawBuggyKind,
    ) -> ArcRawBoundaryFinding {
        ArcRawBoundaryFinding {
            fn_name: self.name.clone(),
            line,
            kind,
            site_text,
            advice: ADVICE,
        }
    }
}

fn walk_item(source: &str, item: &Item, findings: &mut Vec<ArcRawBoundaryFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.sig, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.sig, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (gate b, house discipline): #[cfg(test)] mods
            // are fixture habitat, not production custody code.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    walk_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

/// One fn = one custody scan; a nested fn item gets its own scan.
fn scan_fn(
    source: &str,
    sig: &Signature,
    block: &Block,
    findings: &mut Vec<ArcRawBoundaryFinding>,
) {
    let mut scan = FnScan::new(sig.ident.to_string(), sig_has_ptr_param(sig));
    walk_stmts(source, &block.stmts, &mut scan, findings);
    scan.finish(findings);
}

fn walk_stmts(
    source: &str,
    stmts: &[Stmt],
    scan: &mut FnScan,
    findings: &mut Vec<ArcRawBoundaryFinding>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    if init_from_raw(&init.expr).is_some() {
                        // The from_raw inside the init is the immediate
                        // next site recorded; attach the bind name to it.
                        scan.pending_bind = pat_ident_name(&l.pat);
                    }
                    walk_expr(source, &init.expr, scan, findings);
                    if let Some((_, diverge)) = &init.diverge {
                        walk_expr(source, diverge, scan, findings);
                    }
                }
                scan.pending_bind = None;
            }
            Stmt::Item(item) => walk_item(source, item, findings),
            Stmt::Expr(e, _) => walk_expr(source, e, scan, findings),
            Stmt::Macro(_) => {} // gate c: macro tokens are never descended into
        }
    }
}

fn walk_expr(source: &str, e: &Expr, scan: &mut FnScan, findings: &mut Vec<ArcRawBoundaryFinding>) {
    match e {
        Expr::Call(c) => {
            scan.note_call(source, c);
            walk_expr(source, &c.func, scan, findings);
            for a in &c.args {
                walk_expr(source, a, scan, findings);
            }
        }
        Expr::MethodCall(m) => {
            walk_expr(source, &m.receiver, scan, findings);
            for a in &m.args {
                walk_expr(source, a, scan, findings);
            }
        }
        Expr::If(i) => {
            walk_expr(source, &i.cond, scan, findings);
            walk_stmts(source, &i.then_branch.stmts, scan, findings);
            if let Some((_, else_e)) = &i.else_branch {
                walk_expr(source, else_e, scan, findings);
            }
        }
        Expr::Match(m) => {
            walk_expr(source, &m.expr, scan, findings);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    walk_expr(source, guard, scan, findings);
                }
                walk_expr(source, &arm.body, scan, findings);
            }
        }
        Expr::Unsafe(u) => walk_stmts(source, &u.block.stmts, scan, findings),
        Expr::Block(b) => walk_stmts(source, &b.block.stmts, scan, findings),
        Expr::Async(a) => walk_stmts(source, &a.block.stmts, scan, findings),
        Expr::TryBlock(t) => walk_stmts(source, &t.block.stmts, scan, findings),
        Expr::Loop(l) => walk_stmts(source, &l.body.stmts, scan, findings),
        Expr::While(w) => {
            walk_expr(source, &w.cond, scan, findings);
            walk_stmts(source, &w.body.stmts, scan, findings);
        }
        Expr::ForLoop(fl) => {
            walk_expr(source, &fl.expr, scan, findings);
            walk_stmts(source, &fl.body.stmts, scan, findings);
        }
        Expr::Closure(cl) => walk_expr(source, &cl.body, scan, findings),
        other => walk_expr_leaf(source, other, scan, findings),
    }
}

/// The leaf/transparent expressions: recurse into subexpressions with the
/// scan state unchanged (no custody event of their own). `Expr::Macro` is
/// a deliberate no-op here (gate c: macro tokens are never descended into)
/// and shares the wildcard's empty body — the split keeps each arm's job
/// explicit without tripping the same-arms class.
fn walk_expr_leaf(
    source: &str,
    e: &Expr,
    scan: &mut FnScan,
    findings: &mut Vec<ArcRawBoundaryFinding>,
) {
    match e {
        Expr::Paren(p) => walk_expr(source, &p.expr, scan, findings),
        Expr::Group(g) => walk_expr(source, &g.expr, scan, findings),
        Expr::Reference(r) => walk_expr(source, &r.expr, scan, findings),
        Expr::Unary(u) => walk_expr(source, &u.expr, scan, findings),
        Expr::Binary(b) => {
            walk_expr(source, &b.left, scan, findings);
            walk_expr(source, &b.right, scan, findings);
        }
        Expr::Assign(a) => {
            walk_expr(source, &a.left, scan, findings);
            walk_expr(source, &a.right, scan, findings);
        }
        Expr::Field(f) => walk_expr(source, &f.base, scan, findings),
        Expr::Index(ix) => {
            walk_expr(source, &ix.expr, scan, findings);
            walk_expr(source, &ix.index, scan, findings);
        }
        Expr::Tuple(t) => {
            for e in &t.elems {
                walk_expr(source, e, scan, findings);
            }
        }
        Expr::Array(a) => {
            for e in &a.elems {
                walk_expr(source, e, scan, findings);
            }
        }
        Expr::Repeat(r) => {
            walk_expr(source, &r.expr, scan, findings);
            walk_expr(source, &r.len, scan, findings);
        }
        Expr::Struct(s) => {
            for f in &s.fields {
                walk_expr(source, &f.expr, scan, findings);
            }
            if let Some(rest) = &s.rest {
                walk_expr(source, rest, scan, findings);
            }
        }
        Expr::Cast(c) => walk_expr(source, &c.expr, scan, findings),
        Expr::Let(l) => walk_expr(source, &l.expr, scan, findings),
        Expr::Range(r) => {
            if let Some(start) = &r.start {
                walk_expr(source, start, scan, findings);
            }
            if let Some(end) = &r.end {
                walk_expr(source, end, scan, findings);
            }
        }
        Expr::RawAddr(ra) => walk_expr(source, &ra.expr, scan, findings),
        Expr::Await(a) => walk_expr(source, &a.base, scan, findings),
        Expr::Try(t) => walk_expr(source, &t.expr, scan, findings),
        Expr::Return(r) => {
            if let Some(e) = &r.expr {
                walk_expr(source, e, scan, findings);
            }
        }
        Expr::Break(b) => {
            if let Some(e) = &b.expr {
                walk_expr(source, e, scan, findings);
            }
        }
        Expr::Yield(y) => {
            if let Some(e) = &y.expr {
                walk_expr(source, e, scan, findings);
            }
        }
        // Expr::Macro: gate c. Lit / Path / Verbatim / Rest: leaves.
        _ => {}
    }
}

/// The `Arc::from_raw` call a `let` init resolves to through the usual
/// wrappers (`unsafe { .. }`, parens, a single-expression block).
fn init_from_raw(init: &Expr) -> Option<&ExprCall> {
    match init {
        Expr::Call(c) if is_arc_from_raw(c) => Some(c),
        Expr::Unsafe(u) => block_last_expr(&u.block).and_then(init_from_raw),
        Expr::Paren(p) => init_from_raw(&p.expr),
        Expr::Block(b) => block_last_expr(&b.block).and_then(init_from_raw),
        _ => None,
    }
}

fn block_last_expr(block: &Block) -> Option<&Expr> {
    match block.stmts.last()? {
        Stmt::Expr(e, None) => Some(e),
        _ => None,
    }
}

fn is_arc_from_raw(call: &ExprCall) -> bool {
    let Some(path) = call_path(call) else {
        return false;
    };
    let Some(last) = path.segments.last() else {
        return false;
    };
    last.ident == "from_raw" && seg_any(path, "Arc")
}

fn call_path(call: &ExprCall) -> Option<&Path> {
    let Expr::Path(p) = call.func.as_ref() else {
        return None;
    };
    if p.qself.is_some() {
        return None;
    }
    Some(&p.path)
}

fn seg_any(path: &Path, name: &str) -> bool {
    path.segments.iter().any(|s| s.ident == name)
}

/// A bare, single-segment, unqualified path — a plain variable name.
fn path_var_name(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else {
        return None;
    };
    if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    Some(p.path.segments[0].ident.to_string())
}

fn pat_ident_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pi) => Some(pi.ident.to_string()),
        Pat::Type(pt) => pat_ident_name(&pt.pat),
        _ => None,
    }
}

fn sig_has_ptr_param(sig: &Signature) -> bool {
    sig.inputs.iter().any(|arg| match arg {
        FnArg::Typed(pt) => type_has_ptr(&pt.ty),
        FnArg::Receiver(_) => false,
    })
}

fn type_has_ptr(ty: &Type) -> bool {
    match ty {
        Type::Ptr(_) => true,
        Type::Paren(t) => type_has_ptr(&t.elem),
        Type::Group(g) => type_has_ptr(&g.elem),
        Type::Reference(r) => type_has_ptr(&r.elem),
        Type::Array(a) => type_has_ptr(&a.elem),
        Type::Slice(s) => type_has_ptr(&s.elem),
        Type::Tuple(t) => t.elems.iter().any(type_has_ptr),
        _ => false,
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// `(line, site text)` of the call; `(0, "")` past the foreign-parse guard
/// (the span does not belong to `source` — never slice it).
fn call_site(source: &str, call: &ExprCall) -> (usize, String) {
    let span = call.span().byte_range();
    if span.end > source.len() {
        return (0, String::new());
    }
    (
        line_of_offset(source, span.start),
        source[span.clone()].to_string(),
    )
}

/// 1-based line of a byte offset (the offset must be ≤ `source.len()`).
fn line_of_offset(source: &str, offset: usize) -> usize {
    1 + source[..offset].matches('\n').count()
}

#[cfg(test)]
mod tests {
    use super::{detect, ArcRawBoundaryFinding, ArcRawBuggyKind};

    fn findings(src: &str) -> Vec<ArcRawBoundaryFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn buggy_waker_forget_fires() {
        let src = r"
struct InstinctWaker;

unsafe fn clone_waker(ptr: *const InstinctWaker) {
    Arc::increment_strong_count(ptr);
    let waker = unsafe { Arc::<InstinctWaker>::from_raw(ptr) };
    std::mem::forget(waker);
}
";
        let f = findings(src);
        let forget = f
            .iter()
            .find(|fd| fd.kind == ArcRawBuggyKind::ForgetAfterFromRaw)
            .expect("the waker-shaped forget-after-borrow fires");
        assert_eq!(forget.fn_name, "clone_waker");
        assert!(
            forget.site_text.contains("forget(waker)"),
            "{}",
            forget.site_text
        );
        // The fixture collapses BOTH waker arms into one fn, so the
        // increment-then-consume shape (c) is genuinely present too — rule
        // 3 fires beside rule 1. DoubleFromRaw must not (one from_raw site).
        assert!(f
            .iter()
            .any(|fd| fd.kind == ArcRawBuggyKind::IncrementBeforeFromRaw));
        assert!(!f.iter().any(|fd| fd.kind == ArcRawBuggyKind::DoubleFromRaw));
    }

    #[test]
    fn buggy_double_from_raw_fires() {
        let src = r"
unsafe fn double_consume(ptr: *const VaultShard) {
    let a = unsafe { Arc::from_raw(ptr) };
    let b = unsafe { Arc::from_raw(ptr) };
    drop((a, b));
}
";
        let f = findings(src);
        assert_eq!(
            f.len(),
            1,
            "the two-site proxy fires once, on the second site"
        );
        assert_eq!(f[0].kind, ArcRawBuggyKind::DoubleFromRaw);
        assert_eq!(f[0].fn_name, "double_consume");
        assert!(f[0].site_text.contains("from_raw"));
    }

    #[test]
    fn buggy_increment_before_consume_fires() {
        let src = r"
unsafe fn overcount_take(ptr: *const VaultShard) {
    Arc::increment_strong_count(ptr);
    let shard = unsafe { Arc::from_raw(ptr) };
    drop(shard);
}
";
        let f = findings(src);
        assert_eq!(
            f.len(),
            1,
            "no forget, single consume — only the over-count fires"
        );
        assert_eq!(f[0].kind, ArcRawBuggyKind::IncrementBeforeFromRaw);
        assert_eq!(f[0].fn_name, "overcount_take");
    }

    #[test]
    fn legitimate_into_raw_roundtrip_silent() {
        let src = r"
fn custody_transfer(shard: Arc<VaultShard>) -> usize {
    let ptr = Arc::into_raw(shard);
    let back = unsafe { Arc::from_raw(ptr) };
    Arc::into_raw(back) as usize
}
";
        assert!(
            findings(src).is_empty(),
            "gate (a): the into_raw round-trip suppresses the whole fn"
        );
    }

    #[test]
    fn legitimate_single_consume_silent() {
        let src = r"
unsafe fn consume_once(ptr: *const VaultShard) {
    let shard = unsafe { Arc::from_raw(ptr) };
    drop(shard);
}
";
        assert!(
            findings(src).is_empty(),
            "the benign single consume never fires"
        );
    }

    #[test]
    fn manuallydrop_custody_silent() {
        let src = r"
unsafe fn held_shard(ptr: *const VaultShard) {
    let held = ManuallyDrop::new(unsafe { Arc::from_raw(ptr) });
    Arc::increment_strong_count(ptr);
    let _ = &*held;
}
";
        assert!(findings(src).is_empty(), "the custody marker suppresses");
    }

    #[test]
    fn test_mod_skipped() {
        let src = r"
#[cfg(test)]
mod custody_tests {
    unsafe fn clone_waker(ptr: *const InstinctWaker) {
        Arc::increment_strong_count(ptr);
        let waker = unsafe { Arc::from_raw(ptr) };
        std::mem::forget(waker);
    }
}
";
        assert!(findings(src).is_empty(), "fixture habitat stays silent");
    }

    #[test]
    fn fixed_both_arms_stay_silent() {
        let src = r"
struct InstinctWaker;

unsafe fn clone_waker(ptr: *const InstinctWaker) {
    Arc::increment_strong_count(ptr);
    let borrowed: &InstinctWaker = &*ptr;
    let _ = borrowed;
}

unsafe fn consume_waker(ptr: *mut InstinctWaker) {
    let owned: Arc<InstinctWaker> = Arc::from_raw(ptr);
    drop(owned);
}
";
        assert!(
            findings(src).is_empty(),
            "both fixed arms (borrowed increment+deref, single from_raw) stay silent"
        );
    }
}
