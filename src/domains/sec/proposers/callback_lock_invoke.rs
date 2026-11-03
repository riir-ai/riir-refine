//! ffi-callback-invoke-outside-lock — SUGGEST-ONLY detector (Batch 325, the mozilla/uniffi-rs FFI-hygiene distill; Research 361 / Plan 327).
//!
//! A callback/waker stored behind a mutex that is INVOKED while the lock
//! guard is still live. If the callback (or the thread it runs on) re-enters
//! this lock — or already holds a foreign runtime lock: a Python GIL, a JS
//! event loop — the internal Mutex and the foreign lock wait on each other
//! and the thread deadlocks (the cross-FFI ABBA class; boundary liveness,
//! the `Ffi` family). The uniffi scheduler's own law is extract-under-lock,
//! invoke-after-release: `store()` lifts `to_invoke` out of the
//! `state.lock()` scope and runs `cb.invoke(poll)` only after the guard has
//! dropped (`oneshot.rs`'s wake site is the same shape — class provenance,
//! cited never pasted).
//!
//! ## Why suggest-only (never a fix)
//!
//! Whether the callback can re-enter this lock — directly, through the
//! thread it runs on, or through a foreign runtime lock — is a protocol
//! property of the callback contract, never mechanical. NO `FIXABLE_SEC`
//! membership, NO `collect_edits` arm — the suggestion surfaces through
//! retrieval and review.
//!
//! ## The bounded form
//!
//! ```text
//! // extract under the lock — the guard is a temporary, dropped at the `;`
//! let to_invoke = state.lock().unwrap().cb.take();
//! if let Some(cb) = to_invoke {
//!     cb.invoke(x); // invoke AFTER the guard has dropped
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Guard-live-at-invoke shape**: a `let g = x.lock()` /
//!   `.lock().unwrap()` / `.lock().expect(..)` binding whose variable is
//!   still in scope at a call site naming the invocation vocabulary — the
//!   deadlock QUESTION, conservatively proxied by the callee text.
//! - **Drop/scope-end suppression (the FP control)**: a `drop(g)` /
//!   `drop(&g)` statement or the end of the enclosing block BEFORE the
//!   invoke site means the extract-then-invoke shape is already present —
//!   silent. A drop folded inside a larger expression is not recognized
//!   (only the statement form is the signature).
//! - **Inner-scope suppression**: a guard bound inside a nested block with
//!   the invoke OUTSIDE that scope is out of reach by construction — silent.
//! - **Vocabulary gate**: the call site's callee or callee-receiver must
//!   name a whole `_`/digit segment of invoke/wake/call/notify/complete/
//!   resume/fire — `cb.invoke(x)`, `waker.wake_by_ref()`,
//!   `fire_callback(poll)`; `callbacks.run(x)` stays silent.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into; fns with no lock call stay
//!   silent (no lock = no rule).
//! - **One finding per (fn, guard, invoke) triple** — the first qualifying
//!   invoke site only.
//!
//! ## Honest recall limits (stated in-rule)
//!
//! - **Cross-fn invocation is unconnected**: a `cb.invoke(..)` delegated to
//!   a helper never fires.
//! - **`std::sync::Mutex` only at this pin**: the `.lock()` spelling is the
//!   tracked shape; `RwLock::read`/`write` and the spin-style guards are out
//!   of scope (`parking_lot`'s `.lock()` shares the spelling and would fire
//!   too — review decides, suggest-only is the blast-radius control).
//! - **A callback proven pure-internal stays fire-adjacent**: the rule
//!   flags the QUESTION, review decides.
//! - **Conditional guard release is OUT**: an early-return inside the lock
//!   scope with the invoke after it does not fire — only the
//!   straightforward guard-live-at-invoke shape does. Closure bodies run
//!   with a FRESH scope (outer guards not assumed live there — the
//!   deferred-execution face stays silent).
//!
//! ## Distinct-from
//!
//! - **`host-lock-across-reentrant-blocking-call`** (the corpus's `entries_08`
//!   cubecl #1678 rule) owns the SAME-LOCK self-re-entry failure mode: a
//!   blocking foreign call made while holding the host lock THAT THE CALL
//!   RE-ENTERS — one lock on both sides of the wait. THIS rule is strictly
//!   CROSS-FFI ABBA: a language/runtime boundary where the invoked callback
//!   may run on, or block on, the foreign side's lock — the internal Mutex
//!   vs a foreign GIL/event-loop, two locks with two owners, neither
//!   recoverable from the Rust side. Same extract-under-lock /
//!   invoke-after-release SHAPE, different failure regime — the corpus
//!   admits same-shape-different-failure-mode rules (the tokio-guard
//!   precedent), so flipping this rule's demotion never collides with that
//!   one.
//! - **`drop-impl-no-panic`**: panic SAFETY at the Drop boundary, not lock
//!   liveness — unrelated surfaces.
//!
//! Batch 325 / Plan 327 / Research 361 (mozilla/uniffi-rs @
//! ef9625b68dd51207f55f024a27649e58edb52b09, MPL-2.0 — cited as class
//! provenance; the fixture is an original synthesis).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Expr, File, Item, Pat, Stmt};

/// One detected invoke-under-live-guard site.
#[derive(Debug, Clone)]
pub struct CallbackLockFinding {
    /// The enclosing function's name.
    pub fn_name: String,
    /// 1-based source line of the invoke call.
    pub line: usize,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a callback/waker is INVOKED while a mutex guard is \
still live — if the callback re-enters this lock, or runs on a thread that \
holds a foreign runtime lock (a Python GIL, a JS event loop), the two locks \
wait on each other and the thread deadlocks (the cross-FFI ABBA class). \
Extract under the lock, invoke after release: `let to_invoke = \
state.lock().unwrap().cb.take(); if let Some(cb) = to_invoke { cb.invoke(x); }` \
(Batch 325, suggest-only)";

/// The callback/waker invocation vocabulary — whole `_`/digit segments of
/// the callee or callee-receiver idents (a conservative proxy, stated
/// in-rule).
const INVOKE_VOCAB: &[&str] = &[
    "invoke", "wake", "call", "notify", "complete", "resume", "fire",
];

/// Detect callback invocations under a live mutex guard (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<CallbackLockFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<CallbackLockFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.sig.ident.to_string(), &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.sig.ident.to_string(), &m.block, findings);
                }
            }
        }
        Item::Trait(tr) => {
            for ti in &tr.items {
                if let syn::TraitItem::Fn(m) = ti {
                    if let Some(block) = &m.default {
                        scan_fn(source, &m.sig.ident.to_string(), block, findings);
                    }
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production callback paths.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

fn scan_fn(source: &str, name: &str, block: &syn::Block, findings: &mut Vec<CallbackLockFinding>) {
    // Loose early exit (gate e): no lock spelling in the body, nothing to
    // track. The structural init-chain check is the real gate.
    let body_ts = block.to_token_stream().to_string();
    if !body_ts.contains("lock") {
        return;
    }
    let mut walker = Walker {
        source,
        fn_name: name,
        fired_guards: Vec::new(),
        fired_lines: Vec::new(),
        findings,
    };
    let mut live = Vec::new();
    walker.scan_stmts(&block.stmts, &mut live);
}

/// The per-function walk state: the line authority + the dedupe sets behind
/// the first-qualifying-site fire rule.
struct Walker<'a> {
    source: &'a str,
    fn_name: &'a str,
    /// Guards that already produced their first finding (per-guard dedupe).
    fired_guards: Vec<String>,
    /// Invoke lines already reported (per-site dedupe).
    fired_lines: Vec<usize>,
    findings: &'a mut Vec<CallbackLockFinding>,
}

impl Walker<'_> {
    fn scan_stmts(&mut self, stmts: &[Stmt], live: &mut Vec<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Local(l) => {
                    if let Some(init) = &l.init {
                        // The initializer runs BEFORE the binding exists: an
                        // invoke inside it sees only the pre-binding guards.
                        self.scan_expr(&init.expr, live);
                        if let Some((_, else_e)) = &init.diverge {
                            self.scan_expr(else_e, live);
                        }
                    }
                    // `let g = x.lock()` / `.lock().unwrap()` /
                    // `.lock().expect(..)` — the bound value IS the guard.
                    if let Pat::Ident(id) = &l.pat {
                        if l.init.as_ref().is_some_and(is_guard_init) {
                            live.push(id.ident.to_string());
                        }
                    }
                }
                Stmt::Item(item) => {
                    // A nested fn is its own scope: walk it fresh.
                    collect_in_item(self.source, item, &mut *self.findings);
                }
                Stmt::Expr(e, _) => {
                    // Gate (a): a `drop(g)` / `drop(&g)` statement releases
                    // the guard — everything after it in this scope is
                    // silent.
                    if let Some(guard) = drop_of_guard(e) {
                        live.retain(|n| *n != guard);
                    }
                    self.scan_expr(e, live);
                }
                Stmt::Macro(_) => {} // gate (d): never descend into macro invocations
            }
        }
    }

    fn scan_expr(&mut self, e: &Expr, live: &[String]) {
        match e {
            Expr::Call(c) => {
                if path_has_vocab(&c.func) {
                    self.note_invoke(e, live);
                }
                self.scan_expr(&c.func, live);
                for a in &c.args {
                    self.scan_expr(a, live);
                }
            }
            Expr::MethodCall(m) => {
                if call_site_has_vocab(m) {
                    self.note_invoke(e, live);
                }
                self.scan_expr(&m.receiver, live);
                for a in &m.args {
                    self.scan_expr(a, live);
                }
            }
            Expr::If(i) => {
                self.scan_expr(&i.cond, live);
                let mut inner = live.to_vec();
                self.scan_stmts(&i.then_branch.stmts, &mut inner);
                if let Some((_, else_e)) = &i.else_branch {
                    self.scan_expr(else_e, live);
                }
            }
            Expr::Match(m) => {
                self.scan_expr(&m.expr, live);
                for arm in &m.arms {
                    if let Some((_, guard)) = &arm.guard {
                        self.scan_expr(guard, live);
                    }
                    self.scan_expr(&arm.body, live);
                }
            }
            Expr::Block(b) => {
                let mut inner = live.to_vec();
                self.scan_stmts(&b.block.stmts, &mut inner);
            }
            Expr::Unsafe(u) => {
                let mut inner = live.to_vec();
                self.scan_stmts(&u.block.stmts, &mut inner);
            }
            Expr::Loop(l) => {
                let mut inner = live.to_vec();
                self.scan_stmts(&l.body.stmts, &mut inner);
            }
            Expr::While(w) => {
                self.scan_expr(&w.cond, live);
                let mut inner = live.to_vec();
                self.scan_stmts(&w.body.stmts, &mut inner);
            }
            Expr::ForLoop(fl) => {
                self.scan_expr(&fl.expr, live);
                let mut inner = live.to_vec();
                self.scan_stmts(&fl.body.stmts, &mut inner);
            }
            Expr::Async(a) => {
                let mut inner = live.to_vec();
                self.scan_stmts(&a.block.stmts, &mut inner);
            }
            Expr::Closure(cl) => {
                // A closure runs LATER: outer guards are not assumed live
                // inside it (stated recall limit). Its OWN lock-then-invoke
                // shape is still walked — from a fresh scope.
                let fresh = Vec::new();
                self.scan_expr(&cl.body, &fresh);
            }
            // Expr::Macro (gate d: never descend into macro invocations) and
            // the other leaves fall through `scan_expr_leaf` / the wildcard.
            Expr::Binary(b) => {
                self.scan_expr(&b.left, live);
                self.scan_expr(&b.right, live);
            }
            other => self.scan_expr_leaf(other, live),
        }
    }

    /// The leaf/transparent expressions: recurse into subexpressions with
    /// the live set unchanged (no new scope, no fire rule of their own).
    fn scan_expr_leaf(&mut self, e: &Expr, live: &[String]) {
        match e {
            Expr::Paren(p) => self.scan_expr(&p.expr, live),
            Expr::Group(g) => self.scan_expr(&g.expr, live),
            Expr::Reference(r) => self.scan_expr(&r.expr, live),
            Expr::Unary(u) => self.scan_expr(&u.expr, live),
            Expr::Cast(c) => self.scan_expr(&c.expr, live),
            Expr::Field(f) => self.scan_expr(&f.base, live),
            Expr::Index(ix) => {
                self.scan_expr(&ix.expr, live);
                self.scan_expr(&ix.index, live);
            }
            Expr::Tuple(t) => {
                for el in &t.elems {
                    self.scan_expr(el, live);
                }
            }
            Expr::Array(a) => {
                for el in &a.elems {
                    self.scan_expr(el, live);
                }
            }
            Expr::Assign(a) => {
                self.scan_expr(&a.left, live);
                self.scan_expr(&a.right, live);
            }
            Expr::Range(r) => {
                if let Some(start) = &r.start {
                    self.scan_expr(start, live);
                }
                if let Some(end) = &r.end {
                    self.scan_expr(end, live);
                }
            }
            Expr::Struct(s) => {
                for f in &s.fields {
                    self.scan_expr(&f.expr, live);
                }
                if let Some(rest) = &s.rest {
                    self.scan_expr(rest, live);
                }
            }
            Expr::Return(r) => {
                if let Some(rv) = &r.expr {
                    self.scan_expr(rv, live);
                }
            }
            Expr::Try(t) => self.scan_expr(&t.expr, live),
            Expr::Await(a) => self.scan_expr(&a.base, live),
            Expr::Yield(y) => {
                if let Some(yv) = &y.expr {
                    self.scan_expr(yv, live);
                }
            }
            Expr::Break(b) => {
                if let Some(bv) = &b.expr {
                    self.scan_expr(bv, live);
                }
            }
            Expr::Let(l) => self.scan_expr(&l.expr, live),
            _ => {}
        }
    }

    /// The fire rule: the FIRST qualifying invoke site per (fn, guard), one
    /// finding per site — exactly one (fn, guard, invoke) triple each.
    fn note_invoke(&mut self, e: &Expr, live: &[String]) {
        if live.is_empty() {
            return;
        }
        let invoke_line = line_of(self.source, e.span().byte_range().start);
        if self.fired_lines.contains(&invoke_line) {
            return;
        }
        let fresh: Vec<String> = live
            .iter()
            .filter(|g| !self.fired_guards.contains(g))
            .cloned()
            .collect();
        if fresh.is_empty() {
            return;
        }
        self.fired_lines.push(invoke_line);
        self.fired_guards.extend(fresh);
        self.findings.push(CallbackLockFinding {
            fn_name: self.fn_name.to_string(),
            line: invoke_line,
            advice: ADVICE,
        });
    }
}

/// Whether a let-INIT binds THE GUARD ITSELF: a chain whose outermost real
/// call is `lock`, with at most an `unwrap`/`expect` tail. Any other tail
/// (`take()`, `clone()`, a field projection) means the bound value is not
/// the guard — the extract-then-invoke shape stays silent.
fn is_guard_init(init: &syn::LocalInit) -> bool {
    guard_chain(&init.expr)
}

fn guard_chain(e: &Expr) -> bool {
    let Expr::MethodCall(m) = e else {
        return false;
    };
    match m.method.to_string().as_str() {
        "lock" => m.args.is_empty(),
        "unwrap" | "expect" => guard_chain(&m.receiver),
        _ => false,
    }
}

/// `drop(g)` / `drop(&g)` / `drop(&mut g)` as a statement — the guard-var
/// name (gate (a)'s structural form; only the statement shape is honored).
fn drop_of_guard(e: &Expr) -> Option<String> {
    let Expr::Call(c) = e else {
        return None;
    };
    let Expr::Path(p) = &*c.func else {
        return None;
    };
    if !p.path.segments.iter().any(|s| s.ident == "drop") || c.args.len() != 1 {
        return None;
    }
    let mut arg = &c.args[0];
    if let Expr::Reference(r) = arg {
        arg = r.expr.as_ref();
    }
    if let Expr::Paren(pp) = arg {
        arg = pp.expr.as_ref();
    }
    if let Expr::Path(p) = arg {
        return p.path.segments.last().map(|s| s.ident.to_string());
    }
    None
}

/// A free-fn/path call whose path mentions the invocation vocabulary:
/// `invoke(x)`, `fire_callback(poll)`, `Waker::wake(tx)`.
fn path_has_vocab(func: &Expr) -> bool {
    let Expr::Path(p) = func else {
        return false;
    };
    p.path
        .segments
        .iter()
        .any(|s| segments_have_vocab(&s.ident.to_string()))
}

/// A method call whose method OR receiver-base mentions the vocabulary:
/// `cb.invoke(x)`, `waker.wake_by_ref()`.
fn call_site_has_vocab(m: &syn::ExprMethodCall) -> bool {
    segments_have_vocab(&m.method.to_string()) || receiver_base_vocab(&m.receiver)
}

fn receiver_base_vocab(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p
            .path
            .segments
            .iter()
            .any(|s| segments_have_vocab(&s.ident.to_string())),
        Expr::MethodCall(m) => receiver_base_vocab(&m.receiver),
        Expr::Field(f) => receiver_base_vocab(&f.base),
        Expr::Paren(p) => receiver_base_vocab(&p.expr),
        Expr::Reference(r) => receiver_base_vocab(&r.expr),
        Expr::Group(g) => receiver_base_vocab(&g.expr),
        _ => false,
    }
}

/// Whole `_`/digit-segment vocabulary match on the lowered ident text:
/// `invoke_cb`, `wake_by_ref`, `notify_all`, `fire_callback` fire;
/// `recalculate` does not (segment, not substring).
fn segments_have_vocab(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered
        .split(|c: char| c == '_' || c.is_ascii_digit())
        .any(|seg| INVOKE_VOCAB.contains(&seg))
}

/// 1-based source line of a byte offset — the `source` parameter is the
/// line authority (span byte offsets are trusted per the house emitter).
fn line_of(source: &str, offset: usize) -> usize {
    let end = offset.min(source.len());
    1 + source[..end].matches('\n').count()
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::CallbackLockFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn buggy_invoke_under_lock_fires() {
        let src = r"use std::sync::Mutex;
trait Cb { fn invoke(&self, poll: u32); }
struct State { cb: Option<Box<dyn Cb>> }
fn poll(state: &Mutex<State>, poll_num: u32) {
    let g = state.lock().unwrap();
    if let Some(cb) = g.cb.as_ref() {
        cb.invoke(poll_num);
    }
}
";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the invoke-under-guard shape fires");
        assert_eq!(f[0].fn_name, "poll");
        assert_eq!(f[0].line, 7, "the finding names the invoke site");
    }

    #[test]
    fn extract_then_invoke_silent() {
        let src = r"use std::sync::Mutex;
trait Cb { fn invoke(&self, poll: u32); }
struct State { cb: Option<Box<dyn Cb>> }
fn poll(state: &Mutex<State>, poll_num: u32) {
    let cb = state.lock().unwrap().cb.take();
    if let Some(cb) = cb {
        cb.invoke(poll_num);
    }
}
";
        assert!(
            findings(src).is_empty(),
            "the guard is a temporary in the let-init — nothing live to hold"
        );
    }

    #[test]
    fn explicit_drop_guard_silent() {
        let src = r"use std::sync::Mutex;
trait Cb { fn invoke(&self, poll: u32); }
struct State { cb: Option<Box<dyn Cb>> }
fn poll(state: &Mutex<State>, poll_num: u32) {
    let g = state.lock().unwrap();
    let cb = g.cb.clone();
    drop(g);
    cb.invoke(poll_num);
}
";
        assert!(
            findings(src).is_empty(),
            "the guard is released before the invoke (gate a)"
        );
    }

    #[test]
    fn inner_scope_separate_silent() {
        let src = r"use std::sync::Mutex;
trait Cb { fn invoke(&self, poll: u32); }
struct State { cb: Option<Box<dyn Cb>> }
fn poll(state: &Mutex<State>, poll_num: u32) {
    let cb = {
        let g = state.lock().unwrap();
        g.cb.clone()
    };
    cb.invoke(poll_num);
}
";
        assert!(
            findings(src).is_empty(),
            "the guard never leaves its inner block (gate b)"
        );
    }

    #[test]
    fn no_lock_silent() {
        let src = r"trait Cb { fn invoke(&self, x: u32); }
fn run(cb: &dyn Cb, x: u32) {
    cb.invoke(x);
}
";
        assert!(findings(src).is_empty(), "no lock call, no rule (gate e)");
    }

    #[test]
    fn test_mod_skipped() {
        let src = r"use std::sync::Mutex;
#[cfg(test)]
mod tests {
    fn poll(state: &Mutex<u8>) {
        let g = state.lock().unwrap();
        g.invoke_all();
    }
}
";
        assert!(findings(src).is_empty(), "fixture habitat stays silent");
    }

    #[test]
    fn wake_vocabulary_and_expect_tail_fire() {
        let src = r#"use std::sync::Mutex;
use std::task::Waker;
fn nudge(state: &Mutex<Waker>, waker: &Waker) {
    let g = state.lock().expect("poisoned");
    waker.wake_by_ref();
}
"#;
        let f = findings(src);
        assert_eq!(f.len(), 1, "the expect-tail guard + wake vocabulary fires");
        assert_eq!(f[0].fn_name, "nudge");
    }

    /// The corpus fixtures (returned with the rule entry) fire/stay-silent
    /// here so the corpus entry and the detector can never drift apart.
    const CORPUS_BUGGY: &str = r"use std::sync::Mutex;
trait Invoke { fn invoke(&self, poll: u32); }
struct Scheduler { cb: Mutex<Option<Box<dyn Invoke>>> }
impl Scheduler {
    fn poll_once(&self, poll: u32) {
        let g = self.cb.lock().unwrap();
        if let Some(cb) = g.as_ref() {
            cb.invoke(poll); // guard live: the callback may re-enter — ABBA deadlock
        }
    }
}
";

    const CORPUS_FIXED: &str = r"use std::sync::Mutex;
trait Invoke { fn invoke(&self, poll: u32); }
struct Scheduler { cb: Mutex<Option<Box<dyn Invoke>>> }
impl Scheduler {
    fn poll_once(&self, poll: u32) {
        let to_invoke = self.cb.lock().unwrap().take();
        if let Some(cb) = to_invoke {
            cb.invoke(poll); // extract-under-lock, invoke-after-release
        }
    }
}
";

    #[test]
    fn corpus_fixtures_fire_and_stay_silent() {
        let buggy = findings(CORPUS_BUGGY);
        assert_eq!(buggy.len(), 1, "the buggy fixture fires once");
        assert_eq!(buggy[0].fn_name, "poll_once");
        let fixed = syn::parse_file(CORPUS_FIXED).expect("fixed parses");
        assert!(
            detect(CORPUS_FIXED, &fixed).is_empty(),
            "the fixed fixture stays silent"
        );
    }
}
