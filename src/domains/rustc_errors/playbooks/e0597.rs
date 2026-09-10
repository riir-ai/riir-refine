//! The E0597 playbook family (Plan 019 Phase 2) — "borrowed value does not
//! live long enough", the uncontested territory the premise probe measured
//! (zero rustc suggestion spans; `cargo fix --broken-code` changes nothing).
//!
//! ## Oracle-grounded shape knowledge (probed rustc 1.93.0, 2026-08-21)
//!
//! Every anchor below comes from labeled spans the diagnostic itself carries
//! ([`LabeledSpan`]) — the playbooks consume the oracle's location knowledge,
//! they never guess locations:
//!
//! - label "type annotation requires that 'x' is borrowed for 'static" —
//!   the span covers the ANNOTATION text (`Vec<Case<'static>>`); template 4
//!   swaps the `'static` token for `'_` (the min-delta fix).
//! - label "coercion requires …" — same demand, but the span covers the
//!   coercion SITE (no `'static` token there); the annotation lives
//!   elsewhere, so template 4 stays silent and the closure templates
//!   arbitrate.
//! - label "value captured here" — the span covers the closure's `||`.
//! - label "binding 'x' declared here" — the span covers the binding ident.
//! - label "'x' dropped here while still borrowed" — the span covers the `}`
//!   of the block the value dies in (template 6's hoist target).
//!
//! A rustc that changes these label texts makes the family go SILENT (every
//! diagnostic a miss), never wrong — the floor gate re-measures on toolchain
//! bumps.
//!
//! ## The candidate ladder (ranked alternatives — the product claim)
//!
//! `cargo fix` applies its one suggestion blind; this family ENUMERATES:
//! the `'_` swap (2 bytes) → `move ` (5 bytes) → `{ let x = x.clone(); move
//! … }` → drop-the-box (statement) → `Rc::clone`/`Arc::clone` capture →
//! scope hoist. The driver applies best-first, rechecks, reverts no-progress
//! — the oracle arbitrates which rung compiles.
//!
//! ## Template inventory vs the plan
//!
//! - Template 1 (plain `move` closure) is DEMOTED: that shape is diagnosed
//!   E0373 and rustc's `move ` suggestion is `MachineApplicable` (`cargo fix`
//!   territory).
//! - `Box::leak` is EXCLUDED by design: compiles, tests pass, leaks. Pinned
//!   by [`tests::box_leak_is_never_emitted`].
//! - Template 3's full `&mut dyn FnMut()` runner restructure (the committed
//!   incident fix) is a multi-hunk rewrite outside a single-span candidate;
//!   the bounded single-boxed-let form ships here, and the `'_` swap
//!   (template 4) covers the incident's `Vec<Case<'static>>` shape at
//!   min-delta.

use std::ops::Range;

use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprClosure, ItemFn, Pat, PatType, Stmt};

use super::{anchor, binding_name, cand, CompileProposer, FixCandidate, Playbooks};
use crate::domains::rustc_errors::diag::RustcErrorDiag;

// ── shared syn helpers (closure + let extraction) ────────────────────

/// The full byte range of the closure starting at `closure_start` (syn
/// ground truth, not brace matching). `None` when the file does not parse
/// or no closure starts there (macro bodies are opaque to syn).
fn closure_range(source: &str, closure_start: usize) -> Option<Range<usize>> {
    let file = syn::parse_file(source).ok()?;
    let mut v = ClosureFinder {
        at: closure_start,
        found: None,
    };
    v.visit_file(&file);
    v.found
}

struct ClosureFinder {
    at: usize,
    found: Option<Range<usize>>,
}

impl<'ast> Visit<'ast> for ClosureFinder {
    fn visit_expr_closure(&mut self, c: &'ast ExprClosure) {
        if self.found.is_none() {
            let r = c.span().byte_range();
            if r.start == self.at {
                self.found = Some(r);
            }
        }
        syn::visit::visit_expr_closure(self, c);
    }
}

/// Owned snapshot of one `let` statement — the byte ranges + initializer
/// classification the closure templates need, extracted under the parse
/// (no AST lifetimes escape `locals`).
struct LocalInfo {
    /// The whole statement (`let … ;`).
    stmt: Range<usize>,
    /// The binding pattern WITHOUT a type ascription (`f`, not `f: Box<…>`).
    pat: Range<usize>,
    /// The ascription type (`let x: T`), when present.
    ty: Option<Range<usize>>,
    init: Option<InitInfo>,
}

/// What the initializer IS, for the two shapes that matter here.
struct InitInfo {
    is_box_new: bool,
    /// The single argument of that `Box::new(…)`, when it is one.
    box_arg: Option<Range<usize>>,
    /// `"std::rc::Rc::clone"` / `"std::sync::Arc::clone"` when the init is
    /// an `Rc::new` / `Arc::new` call.
    shared_kind: Option<&'static str>,
}

struct LocalCollector {
    out: Vec<LocalInfo>,
}

impl<'ast> Visit<'ast> for LocalCollector {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if l.attrs.is_empty() {
            let (pat, ty) = match &l.pat {
                // `let x: T = …` — the ascription lives in the PATTERN
                // (syn 2's `Local` has no `ty` field).
                Pat::Type(PatType { pat, ty, .. }) => {
                    (pat.span().byte_range(), Some(ty.span().byte_range()))
                }
                _ => (l.pat.span().byte_range(), None),
            };
            let init = l.init.as_ref().map(|i| {
                let mut info = InitInfo {
                    is_box_new: false,
                    box_arg: None,
                    shared_kind: None,
                };
                if let Expr::Call(ExprCall { func, args, .. }) = &*i.expr {
                    info.is_box_new = is_box_new(func) && args.len() == 1;
                    if info.is_box_new {
                        info.box_arg = Some(args[0].span().byte_range());
                    }
                    info.shared_kind = shared_ctor_kind(func);
                }
                info
            });
            self.out.push(LocalInfo {
                stmt: l.span().byte_range(),
                pat,
                ty,
                init,
            });
        }
        syn::visit::visit_local(self, l);
    }
}

/// The `let` snapshots of a file, in source order (owned ranges — no AST
/// lifetimes escape the parse). Attr-carrying locals are skipped: a
/// whole-stmt splice would drop the attrs.
fn locals(source: &str) -> Vec<LocalInfo> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut v = LocalCollector { out: Vec::new() };
    v.visit_file(&file);
    v.out
}

// ── Template 4 — explicit object-lifetime coercion (`'static` → `'_`) ──

/// Swap the `'static` tokens inside the annotation the diagnostic itself
/// indicts. The min-delta candidate for the founding incident's
/// `Vec<Case<'static>>` family: one token, semantics-local, keeps every
/// borrow alive exactly as long as the block needs.
pub struct AnonLifetime;

impl CompileProposer for AnonLifetime {
    fn name(&self) -> &'static str {
        "e0597-anon-lifetime"
    }

    fn code(&self) -> &'static str {
        "E0597"
    }

    fn propose(
        &self,
        diag: &RustcErrorDiag,
        file: &std::path::Path,
        source: &str,
    ) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        // Only the ANNOTATION indictment carries a swap-able token. The
        // "coercion requires" variant points at the coercion site — silent.
        // (`'static` with the leading apostrophe — the trailing delimiter is
        // a backtick in rustc's rendering, not an apostrophe.)
        let Some(ls) = anchor(diag, |l| {
            l.contains("type annotation requires") && l.contains("'static")
        }) else {
            return Vec::new();
        };
        let Some(seg) = source.get(ls.byte_start..ls.byte_end) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(at) = seg[from..].find("'static") {
            let abs = ls.byte_start + from + at;
            out.push(cand(
                diag,
                file,
                ls.line_start,
                (abs, abs + "'static".len()),
                "'_".to_string(),
                self.name(),
            )
            .with_note(
                "loosens the lifetime annotation ('static -> '_) — an API-contract change, not a purely local one",
            ));
            from += at + "'static".len();
        }
        out.shrink_to_fit();
        out
    }
}

// ── Template 2 — closure capture: `move`, then `move`+`.clone()` ──────

/// Prepend `move ` to the closure the diagnostic indicts — transfers
/// ownership of the capture into the closure, which then satisfies `'static`
/// IF the binding is not used afterwards (E0382 otherwise → the driver
/// reverts and tries [`MoveCloneCapture`]). The oracle arbitrates.
pub struct MoveCapture;

impl CompileProposer for MoveCapture {
    fn name(&self) -> &'static str {
        "e0597-move"
    }

    fn code(&self) -> &'static str {
        "E0597"
    }

    fn propose(
        &self,
        diag: &RustcErrorDiag,
        file: &std::path::Path,
        source: &str,
    ) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        let Some(cap) = anchor(diag, |l| l == "value captured here") else {
            return Vec::new();
        };
        if !source[cap.byte_start..].starts_with("||") {
            return Vec::new(); // unexpected anchor shape (already `move`?)
        }
        if source[..cap.byte_start].trim_end().ends_with("move") {
            return Vec::new(); // already a move closure — a second `move` parses nowhere
        }
        vec![cand(
            diag,
            file,
            cap.line_start,
            (cap.byte_start, cap.byte_start),
            "move ".to_string(),
            self.name(),
        )
        .with_note("moves the capture into the closure — ownership leaves the enclosing scope")]
    }
}

/// `{ let x = x.clone(); move || … }` — the move capture PLUS a clone so the
/// original binding stays usable (the E0382-avoiding rung). Clone-ness is
/// NOT knowable parse-only: the candidate is emitted blind and cargo check
/// arbitrates (revert is one iteration — the plan's oracle-arbitration
/// rule).
pub struct MoveCloneCapture;

impl CompileProposer for MoveCloneCapture {
    fn name(&self) -> &'static str {
        "e0597-move-clone"
    }

    fn code(&self) -> &'static str {
        "E0597"
    }

    fn propose(
        &self,
        diag: &RustcErrorDiag,
        file: &std::path::Path,
        source: &str,
    ) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        let Some(cap) = anchor(diag, |l| l == "value captured here") else {
            return Vec::new();
        };
        let Some(name) =
            anchor(diag, |l| l.starts_with("binding `")).and_then(|ls| binding_name(&ls.label))
        else {
            return Vec::new();
        };
        if !source[cap.byte_start..].starts_with("||") {
            return Vec::new();
        }
        let Some(range) = closure_range(source, cap.byte_start) else {
            return Vec::new();
        };
        let Some(text) = source.get(range.start..range.end) else {
            return Vec::new();
        };
        let repl = format!("{{ let {name} = {name}.clone(); move {text} }}");
        vec![cand(
            diag,
            file,
            cap.line_start,
            (range.start, range.end),
            repl,
            self.name(),
        )
        .with_note("clones the captured binding — the closure owns a copy; mutations no longer alias the original")]
    }
}

// ── Template 3 — drop the Box (bounded single-let form) ───────────────

/// `let f: Box<dyn …> = Box::new(c);` → `let f = c;` — removing the
/// `Box<dyn>` (whose object-lifetime default the annotation pins) lets the
/// closure keep its local borrows. Fires only when the let carries BOTH the
/// `Box<…>` annotation AND a `Box::new(closure)` initializer whose closure
/// the diagnostic indicts. The full `&mut dyn FnMut()` runner restructure
/// (the committed incident fix) is a multi-hunk rewrite — out of scope for
/// a single-span candidate; the `'_` swap covers that family at min-delta.
pub struct DropTheBox;

impl CompileProposer for DropTheBox {
    fn name(&self) -> &'static str {
        "e0597-drop-the-box"
    }

    fn code(&self) -> &'static str {
        "E0597"
    }

    fn propose(
        &self,
        diag: &RustcErrorDiag,
        file: &std::path::Path,
        source: &str,
    ) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        let Some(cap) = anchor(diag, |l| l == "value captured here") else {
            return Vec::new();
        };
        for li in locals(source) {
            if !li.stmt.contains(&cap.byte_start) {
                continue;
            }
            let Some(ty) = li.ty.as_ref() else { continue };
            if !source.get(ty.clone()).is_some_and(|t| t.contains("Box<")) {
                continue;
            }
            let Some(init) = li.init.as_ref() else {
                continue;
            };
            if !init.is_box_new {
                continue;
            }
            let Some(pat_text) = source.get(li.pat.clone()) else {
                continue;
            };
            let Some(inner) = init.box_arg.as_ref().and_then(|r| source.get(r.clone())) else {
                continue;
            };
            let repl = format!("let {pat_text} = {inner};");
            return vec![cand(
                diag,
                file,
                cap.line_start,
                (li.stmt.start, li.stmt.end),
                repl,
                self.name(),
            )
            .with_note("unboxes the binding — the value is no longer heap-allocated; drop timing may change")];
        }
        Vec::new()
    }
}

/// `Box::new` / `std::boxed::Box::new` — any path whose last two segments
/// are `Box` then `new`.
fn is_box_new(func: &Expr) -> bool {
    let Expr::Path(p) = func else { return false };
    let n = p.path.segments.len();
    let is = |i: usize, name: &str| p.path.segments.get(i).is_some_and(|s| s.ident == name);
    n >= 2 && is(n - 1, "new") && is(n - 2, "Box")
}

// ── Template 5 — Rc/Arc shared ownership ──────────────────────────────

/// `{ let x = Rc::clone(&x); move || … }` when the binding was ALREADY
/// created shared (`Rc::new`/`Arc::new`) — the closure then owns an
/// reference-counted clone and satisfies `'static` while the original stays
/// usable. Non-shared bindings stay silent (the two-hunk wrap-the-init form
/// is deliberately not attempted: it rewrites a distant statement and its
/// guards are not provable parse-only).
pub struct SharedOwnership;

impl CompileProposer for SharedOwnership {
    fn name(&self) -> &'static str {
        "e0597-shared-ownership"
    }

    fn code(&self) -> &'static str {
        "E0597"
    }

    fn propose(
        &self,
        diag: &RustcErrorDiag,
        file: &std::path::Path,
        source: &str,
    ) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        let Some(cap) = anchor(diag, |l| l == "value captured here") else {
            return Vec::new();
        };
        let Some(bind) = anchor(diag, |l| l.starts_with("binding `")) else {
            return Vec::new();
        };
        let Some(name) = binding_name(&bind.label) else {
            return Vec::new();
        };
        if !source[cap.byte_start..].starts_with("||") {
            return Vec::new();
        }
        // The binding's initializer must be Rc::new / Arc::new.
        let mut shared: Option<&'static str> = None;
        for li in locals(source) {
            if !li.pat.contains(&bind.byte_start) {
                continue;
            }
            if let Some(init) = &li.init {
                shared = init.shared_kind;
            }
            break;
        }
        let Some(kind) = shared else {
            return Vec::new();
        };
        let Some(range) = closure_range(source, cap.byte_start) else {
            return Vec::new();
        };
        let Some(text) = source.get(range.start..range.end) else {
            return Vec::new();
        };
        let repl = format!("{{ let {name} = {kind}(&{name}); move {text} }}");
        vec![cand(
            diag,
            file,
            cap.line_start,
            (range.start, range.end),
            repl,
            self.name(),
        )
        .with_note("switches to shared ownership (Rc/Arc) — access is now shared, not exclusive")]
    }
}

/// `std::rc::Rc::new` → `"std::rc::Rc::clone"`, `std::sync::Arc::new` →
/// `"std::sync::Arc::clone"` (fully-qualified — no import needed).
fn shared_ctor_kind(func: &Expr) -> Option<&'static str> {
    let Expr::Path(p) = func else { return None };
    let segs = &p.path.segments;
    let n = segs.len();
    let is = |i: usize, name: &str| segs.get(i).is_some_and(|s| s.ident == name);
    if (n >= 2 && is(n - 1, "new") && is(n - 2, "Rc"))
        || (n >= 3 && is(n - 1, "new") && is(n - 2, "Rc") && is(n - 3, "rc"))
    {
        return Some("std::rc::Rc::clone");
    }
    if (n >= 2 && is(n - 1, "new") && is(n - 2, "Arc"))
        || (n >= 3 && is(n - 1, "new") && is(n - 2, "Arc") && is(n - 3, "sync"))
    {
        return Some("std::sync::Arc::clone");
    }
    None
}

// ── Template 6 — scope hoist (NLL: move the declaration out) ──────────

/// Hoist the indicted binding's `let` OUT of the block it dies in — the
/// classic NLL fix for the narrow-scope escape (`let r; { let x = …; r =
/// &x; } *r`). Guards, all load-bearing:
///
/// - The hoisted `let` must be the block's FIRST statement — any other
///   position changes the observable drop order of the statements it jumps.
/// - The dropped-`}` block must not be a function body (there is nowhere to
///   hoist to).
/// - Single-line statements only (multi-line re-indent is not min-delta).
/// - No comments in the spliced region (they would be displaced).
pub struct ScopeHoist;

impl ScopeHoist {
    /// The whitespace prefix of the line containing `at`.
    fn line_indent(source: &str, at: usize) -> String {
        let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
        source[line_start..at]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect()
    }
}

impl CompileProposer for ScopeHoist {
    fn name(&self) -> &'static str {
        "e0597-scope-hoist"
    }

    fn code(&self) -> &'static str {
        "E0597"
    }

    fn propose(
        &self,
        diag: &RustcErrorDiag,
        file: &std::path::Path,
        source: &str,
    ) -> Vec<FixCandidate> {
        if diag.code != self.code() {
            return Vec::new();
        }
        let Some(dropped) = anchor(diag, |l| l.contains("dropped here while still borrowed"))
        else {
            return Vec::new();
        };
        let Some(bind) = anchor(diag, |l| l.starts_with("binding `")) else {
            return Vec::new();
        };
        let Some(name) = binding_name(&bind.label) else {
            return Vec::new();
        };
        // Find the block whose closing brace IS the dropped span.
        let Ok(parsed) = syn::parse_file(source) else {
            return Vec::new();
        };
        let mut v = BlockScan {
            fn_body_closes: Vec::new(),
            blocks: Vec::new(),
        };
        v.visit_file(&parsed);
        let Some(b) = v
            .blocks
            .iter()
            .find(|b| b.close == dropped.byte_start && !v.fn_body_closes.contains(&b.close))
        else {
            return Vec::new();
        };
        // The binding's `let` must be the block's first statement.
        let Some(Stmt::Local(local)) = b.first_stmt.as_ref() else {
            return Vec::new();
        };
        let pat_range = local.pat.span().byte_range();
        let Some(pat_text) = source.get(pat_range.clone()) else {
            return Vec::new();
        };
        // The pattern text must actually carry the indicted binding's name
        // (guards a label→statement mismatch).
        if !pat_range.contains(&bind.byte_start) || !pat_text.contains(name) {
            return Vec::new();
        }
        let stmt_range = local.span().byte_range();
        let Some(stmt_text) = source.get(stmt_range.clone()) else {
            return Vec::new();
        };
        if stmt_text.contains('\n') || stmt_text.contains("//") || stmt_text.contains("/*") {
            return Vec::new();
        }
        // The spliced region (block open → stmt end) must not carry comments.
        let Some(region) = source.get(b.open..stmt_range.end) else {
            return Vec::new();
        };
        if region.contains("//") || region.contains("/*") {
            return Vec::new();
        }
        let indent = Self::line_indent(source, b.open);
        let stmt_trimmed = stmt_text.trim_start();
        // The kept text before the splice point already ends with the block
        // line's indentation — the replacement starts at the statement
        // itself, and only the re-opened block line carries the indent.
        let repl = format!("{stmt_trimmed}\n{indent}{{");
        vec![cand(
            diag,
            file,
            bind.line_start,
            (b.open, stmt_range.end),
            repl,
            self.name(),
        )
        .with_note("hoists the declaration out of the block — drop order changes")]
    }
}

/// One block: byte offsets of `{` and `}` plus its first statement (borrowed).
struct BlockInfo<'a> {
    open: usize,
    close: usize,
    first_stmt: Option<&'a Stmt>,
}

struct BlockScan<'a> {
    fn_body_closes: Vec<usize>,
    blocks: Vec<BlockInfo<'a>>,
}

impl<'ast> Visit<'ast> for BlockScan<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        self.fn_body_closes
            .push(f.block.brace_token.span.close().byte_range().start);
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_block(&mut self, b: &'ast syn::Block) {
        self.blocks.push(BlockInfo {
            open: b.brace_token.span.open().byte_range().start,
            close: b.brace_token.span.close().byte_range().start,
            first_stmt: b.stmts.first(),
        });
        syn::visit::visit_block(self, b);
    }
}

// ── the family registry ───────────────────────────────────────────────

/// The full Phase 2 registry — templates 2–6 (template 1 demoted to
/// `cargo fix`'s E0373 territory; `Box::leak` excluded by design, pinned by
/// test).
#[must_use]
pub fn family() -> Playbooks {
    Playbooks::empty()
        .with(Box::new(AnonLifetime))
        .with(Box::new(MoveCapture))
        .with(Box::new(MoveCloneCapture))
        .with(Box::new(DropTheBox))
        .with(Box::new(SharedOwnership))
        .with(Box::new(ScopeHoist))
}

// ── fixtures (the proposer discipline: buggy/clean pairs + negative
// controls; every FIXED form below was oracle-verified to compile before
// becoming a fixture — see the plan's probe record) ────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::rustc_errors::diag::LabeledSpan;
    use std::path::Path;

    /// Build an E0597 diag for `source` with anchors located by finding the
    /// anchor texts — the same spans rustc emits for these shapes (probed).
    fn e0597_diag(source: &str, primary: &str, labels: &[(&str, &str)]) -> RustcErrorDiag {
        let at = |needle: &str| {
            source
                .find(needle)
                .unwrap_or_else(|| panic!("fixture lacks {needle}"))
        };
        let mut labeled_spans = vec![LabeledSpan {
            file: "src/lib.rs".to_string(),
            line_start: 1,
            is_primary: true,
            byte_start: at(primary),
            byte_end: at(primary) + primary.len(),
            label: "borrowed value does not live long enough".to_string(),
        }];
        for (label, covers) in labels {
            labeled_spans.push(LabeledSpan {
                file: "src/lib.rs".to_string(),
                line_start: 1,
                is_primary: false,
                byte_start: at(covers),
                byte_end: at(covers) + covers.len(),
                label: label.to_string(),
            });
        }
        RustcErrorDiag {
            code: "E0597".to_string(),
            message: "`x` does not live long enough".to_string(),
            primary: Some(("src/lib.rs".to_string(), 1)),
            suggestions: Vec::new(),
            labeled_spans,
            rendered: None,
        }
    }

    fn propose_all(
        p: &dyn CompileProposer,
        source: &str,
        diag: &RustcErrorDiag,
    ) -> Vec<FixCandidate> {
        p.propose(diag, Path::new("/x/src/lib.rs"), source)
    }

    // ── T4 AnonLifetime ────────────────────────────────────────────────

    const INCIDENT_SRC: &str = "pub fn run() -> usize {\n    let n = 5usize;\n    let cases: Vec<Case<'static>> = vec![Case { f: Box::new(|| n) }];\n    cases.iter().map(|c| (c.f)()).sum()\n}\n";

    #[test]
    fn anon_lifetime_swaps_the_indicted_static() {
        let diag = e0597_diag(
            INCIDENT_SRC,
            "|| n",
            &[
                ("binding `n` declared here", "let n = 5usize"),
                ("value captured here", "||"),
                (
                    "type annotation requires that `n` is borrowed for `'static`",
                    "Vec<Case<'static>>",
                ),
            ],
        );
        let out = propose_all(&AnonLifetime, INCIDENT_SRC, &diag);
        assert_eq!(out.len(), 1, "one 'static token → one candidate");
        let at = INCIDENT_SRC.find("'static").unwrap();
        assert_eq!(out[0].byte_span, (at, at + "'static".len()));
        assert_eq!(out[0].replacement, "'_");
        assert_eq!(out[0].strategy_id, "e0597-anon-lifetime");
        // The spliced result is the oracle-verified fixed form.
        let spliced = format!(
            "{}{}{}",
            &INCIDENT_SRC[..at],
            "'_",
            &INCIDENT_SRC[at + "'static".len()..]
        );
        assert!(spliced.contains("Vec<Case<'_>>"));
    }

    #[test]
    fn anon_lifetime_is_silent_on_coercion_variant() {
        // "coercion requires" points at the coercion SITE — no token to swap.
        let src = "pub fn r() -> Reg {\n    let name = String::from(\"x\");\n    Reg { f: Box::new(|| name.len()) }\n}\n";
        let diag = e0597_diag(
            src,
            "name.len()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "||"),
                (
                    "coercion requires that `name` is borrowed for `'static`",
                    "Box::new(|| name.len())",
                ),
            ],
        );
        assert!(propose_all(&AnonLifetime, src, &diag).is_empty());
    }

    #[test]
    fn anon_lifetime_ignores_other_codes_and_missing_anchor() {
        let mut diag = e0597_diag(INCIDENT_SRC, "|| n", &[]);
        diag.code = "E0499".to_string();
        assert!(propose_all(&AnonLifetime, INCIDENT_SRC, &diag).is_empty());
        // E0597 but no annotation label → silent (label paths only).
        let no_ann = e0597_diag(INCIDENT_SRC, "|| n", &[("value captured here", "||")]);
        assert!(propose_all(&AnonLifetime, INCIDENT_SRC, &no_ann).is_empty());
    }

    // ── T2 MoveCapture / MoveCloneCapture ──────────────────────────────

    const CAPTURE_SRC: &str = "pub fn r() -> usize {\n    let name = String::from(\"x\");\n    let f: Box<dyn Fn() -> usize + 'static> = Box::new(|| name.len());\n    f()\n}\n";

    #[test]
    fn move_prefixes_the_indicted_closure() {
        let diag = e0597_diag(
            CAPTURE_SRC,
            "name.len()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "||"),
            ],
        );
        let out = propose_all(&MoveCapture, CAPTURE_SRC, &diag);
        assert_eq!(out.len(), 1);
        let at = CAPTURE_SRC.find("||").unwrap();
        assert_eq!(out[0].byte_span, (at, at));
        assert_eq!(out[0].replacement, "move ");
    }

    #[test]
    fn move_is_silent_when_closure_already_moves() {
        let src = "pub fn r() -> usize {\n    let name = String::from(\"x\");\n    let f: Box<dyn Fn() -> usize + 'static> = Box::new(move || name.clone().len());\n    f()\n}\n";
        let diag = e0597_diag(
            src,
            "name.clone()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "|| name.clone()"),
            ],
        );
        // The capture anchor must sit at a bare `||`; `move ||` closures
        // point the anchor at `||` too but the preceding text is `move`.
        let at = src.find("||").unwrap();
        let mut d = diag;
        for ls in &mut d.labeled_spans {
            if ls.label == "value captured here" {
                ls.byte_start = at;
                ls.byte_end = at + 2;
            }
        }
        assert!(propose_all(&MoveCapture, src, &d).is_empty());
    }

    #[test]
    fn move_clone_builds_the_shadow_block() {
        let diag = e0597_diag(
            CAPTURE_SRC,
            "name.len()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "||"),
            ],
        );
        let out = propose_all(&MoveCloneCapture, CAPTURE_SRC, &diag);
        assert_eq!(out.len(), 1);
        let at = CAPTURE_SRC.find("||").unwrap();
        assert_eq!(out[0].byte_span, (at, at + "|| name.len()".len()));
        assert_eq!(
            out[0].replacement,
            "{ let name = name.clone(); move || name.len() }"
        );
    }

    #[test]
    fn move_clone_needs_both_anchors() {
        let only_cap = e0597_diag(CAPTURE_SRC, "name.len()", &[("value captured here", "||")]);
        assert!(propose_all(&MoveCloneCapture, CAPTURE_SRC, &only_cap).is_empty());
    }

    // ── T3 DropTheBox ──────────────────────────────────────────────────

    #[test]
    fn drop_the_box_rewrites_the_single_let() {
        let diag = e0597_diag(
            CAPTURE_SRC,
            "name.len()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "||"),
            ],
        );
        let out = propose_all(&DropTheBox, CAPTURE_SRC, &diag);
        assert_eq!(out.len(), 1);
        let start = CAPTURE_SRC.find("let f:").unwrap();
        let end = CAPTURE_SRC[start..]
            .find(';')
            .map(|i| start + i + 1)
            .unwrap();
        assert_eq!(out[0].byte_span, (start, end));
        assert_eq!(out[0].replacement, "let f = || name.len();");
    }

    #[test]
    fn drop_the_box_silent_without_box_annotation() {
        // Box::new inside a struct-literal init — no `Box<` let-annotation
        // to drop (the Holder incident shape routes to T2).
        let src = "pub struct H { pub f: Box<dyn Fn() -> usize + 'static> }\npub fn r() -> usize {\n    let name = String::from(\"x\");\n    let h = H { f: Box::new(|| name.len()) };\n    (h.f)()\n}\n";
        let diag = e0597_diag(
            src,
            "name.len()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "||"),
            ],
        );
        assert!(propose_all(&DropTheBox, src, &diag).is_empty());
    }

    // ── T5 SharedOwnership ─────────────────────────────────────────────

    const RC_SRC: &str = "pub fn r() -> usize {\n    let name = std::rc::Rc::new(String::from(\"x\"));\n    let f: Box<dyn Fn() -> usize + 'static> = Box::new(|| name.len());\n    f()\n}\n";

    fn rc_diag() -> RustcErrorDiag {
        e0597_diag(
            RC_SRC,
            "name.len()",
            &[
                ("binding `name` declared here", "name = std"),
                ("value captured here", "||"),
            ],
        )
    }

    #[test]
    fn shared_ownership_clones_the_rc_binding() {
        let diag = rc_diag();
        let out = propose_all(&SharedOwnership, RC_SRC, &diag);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].replacement,
            "{ let name = std::rc::Rc::clone(&name); move || name.len() }"
        );
    }

    #[test]
    fn shared_ownership_silent_on_plain_bindings() {
        let diag = e0597_diag(
            CAPTURE_SRC,
            "name.len()",
            &[
                ("binding `name` declared here", "name = String"),
                ("value captured here", "||"),
            ],
        );
        assert!(propose_all(&SharedOwnership, CAPTURE_SRC, &diag).is_empty());
    }

    #[test]
    fn shared_ownership_arc_variant() {
        let src = "pub fn r() -> usize {\n    let name = std::sync::Arc::new(String::from(\"x\"));\n    let f: Box<dyn Fn() -> usize + 'static> = Box::new(|| name.len());\n    f()\n}\n";
        let diag = e0597_diag(
            src,
            "name.len()",
            &[
                ("binding `name` declared here", "name = std"),
                ("value captured here", "||"),
            ],
        );
        let out = propose_all(&SharedOwnership, src, &diag);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].replacement,
            "{ let name = std::sync::Arc::clone(&name); move || name.len() }"
        );
    }

    // ── T6 ScopeHoist ──────────────────────────────────────────────────

    const SCOPE_SRC: &str = "pub fn probe() -> i32 {\n    let r;\n    {\n        let x = 5i32;\n        r = &x;\n    }\n    *r\n}\n";

    fn scope_diag() -> RustcErrorDiag {
        e0597_diag(
            SCOPE_SRC,
            "&x",
            &[
                ("binding `x` declared here", "x = 5i32"),
                ("`x` dropped here while still borrowed", "    }\n    *r"),
                ("borrow later used here", "*r"),
            ],
        )
    }

    /// The dropped span covers just the `}` — the fixture helper locates it
    /// by line; narrow it to the brace byte.
    fn narrowed_scope_diag() -> RustcErrorDiag {
        let mut d = scope_diag();
        // The `}` is the byte right before the `\n    *r` tail.
        let at = SCOPE_SRC.find("\n    *r").unwrap() - 1;
        for ls in &mut d.labeled_spans {
            if ls.label.contains("dropped here") {
                ls.byte_start = at;
                ls.byte_end = at + 1;
            }
        }
        d
    }

    #[test]
    fn scope_hoist_moves_first_stmt_out() {
        let diag = narrowed_scope_diag();
        let out = propose_all(&ScopeHoist, SCOPE_SRC, &diag);
        assert_eq!(out.len(), 1, "{out:?}");
        // The splice starts AT the block's `{` (the line indentation stays
        // in the kept prefix) and ends after the hoisted statement.
        let open = SCOPE_SRC.find("    {").unwrap() + 4;
        let stmt_start = SCOPE_SRC.find("let x = 5i32;").unwrap();
        let stmt_end = stmt_start + "let x = 5i32;".len();
        assert_eq!(out[0].byte_span, (open, stmt_end));
        assert_eq!(out[0].replacement, "let x = 5i32;\n    {");
        // Spliced result is the oracle-verified fixed form.
        let spliced = format!(
            "{}{}{}",
            &SCOPE_SRC[..open],
            "let x = 5i32;\n    {",
            &SCOPE_SRC[stmt_end..]
        );
        let expected = "pub fn probe() -> i32 {\n    let r;\n    let x = 5i32;\n    {\n        r = &x;\n    }\n    *r\n}\n";
        assert_eq!(spliced, expected);
    }

    #[test]
    fn scope_hoist_silent_when_not_first_stmt() {
        let src = "pub fn probe() -> i32 {\n    let r;\n    {\n        let y = 1i32;\n        let x = 5i32;\n        r = &x;\n    }\n    *r\n}\n";
        let mut d = narrowed_scope_diag();
        let at = src.find("x = 5i32").unwrap() - 4; // the binding ident `x`
        for ls in &mut d.labeled_spans {
            if ls.label.starts_with("binding `") {
                ls.byte_start = at;
                ls.byte_end = at + 1;
            }
        }
        assert!(propose_all(&ScopeHoist, src, &d).is_empty());
    }

    #[test]
    fn scope_hoist_silent_when_dropped_is_fn_body() {
        // The dropped `}` is the fn's own brace — nowhere to hoist to.
        let src = "pub fn probe(x: &i32) -> i32 {\n    let r = x;\n    *r\n}\n";
        let at = src.find("\n}\n").unwrap() + 1;
        let d = e0597_diag(
            src,
            "x",
            &[
                ("binding `x` declared here", "x: &i32"),
                ("`x` dropped here while still borrowed", "}\n"),
            ],
        );
        let mut d = d;
        for ls in &mut d.labeled_spans {
            if ls.label.contains("dropped here") {
                ls.byte_start = at;
                ls.byte_end = at + 1;
            }
        }
        assert!(propose_all(&ScopeHoist, src, &d).is_empty());
    }

    #[test]
    fn scope_hoist_comment_guard() {
        let src = "pub fn probe() -> i32 {\n    let r;\n    {\n        // keep me in the block\n        let x = 5i32;\n        r = &x;\n    }\n    *r\n}\n";
        let mut d = narrowed_scope_diag();
        for ls in &mut d.labeled_spans {
            if ls.label.starts_with("binding `") {
                let at = src.find("x = 5i32").unwrap() - 4;
                ls.byte_start = at;
                ls.byte_end = at + 1;
            }
            if ls.label.contains("dropped here") {
                let at = src.find("\n    }\n").unwrap() + 1;
                ls.byte_start = at;
                ls.byte_end = at + 1;
            }
        }
        assert!(propose_all(&ScopeHoist, src, &d).is_empty());
    }

    // ── the exclusion pin ──────────────────────────────────────────────

    /// `Box::leak` compiles, tests pass, leaks — the canonical
    /// wrong-but-compiling fix. No playbook in this family may EVER emit it.
    #[test]
    fn box_leak_is_never_emitted() {
        let all_srcs = [INCIDENT_SRC, CAPTURE_SRC, RC_SRC, SCOPE_SRC];
        let diags = [
            e0597_diag(
                INCIDENT_SRC,
                "|| n",
                &[
                    ("binding `n` declared here", "let n = 5usize"),
                    ("value captured here", "||"),
                    (
                        "type annotation requires that `n` is borrowed for `'static`",
                        "Vec<Case<'static>>",
                    ),
                ],
            ),
            e0597_diag(
                CAPTURE_SRC,
                "name.len()",
                &[
                    ("binding `name` declared here", "name = String"),
                    ("value captured here", "||"),
                ],
            ),
            rc_diag(),
            narrowed_scope_diag(),
        ];
        let pb = family();
        assert_eq!(pb.len(), 6);
        for (src, diag) in all_srcs.iter().zip(&diags) {
            for c in pb.propose(diag, Path::new("/x/src/lib.rs"), src) {
                assert!(
                    !c.replacement.contains("Box::leak") && !c.replacement.contains("::leak("),
                    "{} proposed a leak: {}",
                    c.strategy_id,
                    c.replacement
                );
            }
        }
    }

    /// The ladder itself: for the incident shape the `'_` swap is the
    /// min-delta candidate (driver ordering: file → line → code →
    /// replacement length), which is the product claim over `cargo fix`'s
    /// single blind suggestion.
    #[test]
    fn incident_ladder_orders_anon_lifetime_first() {
        let diag = e0597_diag(
            INCIDENT_SRC,
            "|| n",
            &[
                ("binding `n` declared here", "let n = 5usize"),
                ("value captured here", "||"),
                (
                    "type annotation requires that `n` is borrowed for `'static`",
                    "Vec<Case<'static>>",
                ),
            ],
        );
        let mut cands = family().propose(&diag, Path::new("/x/src/lib.rs"), INCIDENT_SRC);
        assert!(
            cands.len() >= 2,
            "the incident shape has a ladder: {cands:?}"
        );
        cands.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.line.cmp(&b.line))
                .then(a.code.cmp(&b.code))
                .then(a.replacement.len().cmp(&b.replacement.len()))
                .then(a.strategy_id.cmp(&b.strategy_id))
        });
        assert_eq!(cands[0].strategy_id, "e0597-anon-lifetime");
        assert_eq!(cands[0].replacement, "'_");
    }
}
