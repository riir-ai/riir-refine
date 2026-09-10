//! The borrowck playbook family (Plan 019 Phase 3) — E0502, E0499 (the
//! uncontested territory the premise probe measured at 0% `cargo fix`
//! coverage) plus E0505 (the one contested opening: rustc's own
//! `MachineApplicable` `.clone()` rewrite diverges on aliasing — Bench 041).
//!
//! ## What is deliberately NOT here
//!
//! E0596, E0382, E0507, E0373 ship no template **by measurement**: on the
//! mutation corpus `cargo fix --broken-code` scores 100% matched-inverse on
//! all four (Bench 041). A template that cannot beat the floor does not
//! ship — the G-floor gate (T6.9), not authoring ambition, decides the
//! inventory.
//!
//! ## Oracle-grounded shape knowledge (probed rustc 1.97.0, 2026-08-21)
//!
//! Every anchor comes from labeled spans the diagnostic itself carries:
//!
//! - **E0502** — primary "mutable borrow occurs here" (the mutation expr),
//!   "immutable borrow occurs here" (covers the borrowed PLACE — `v` in
//!   `&v[0]`, not the `&`; coverage varies by shape, so the playbooks
//!   locate the enclosing `let` via syn and never splice on the label
//!   span itself), "immutable borrow later used here" (a use site).
//! - **E0499** — "first mutable borrow occurs here" + primary "second
//!   mutable borrow occurs here" (each covers the full `&mut …` borrow
//!   expr) + "first borrow later used here" (covers the first borrow's
//!   receiver at its last use).
//! - **E0505** — primary "move out of `x` occurs here" (covers the moved
//!   expr at the move site), "borrow of `x` occurs here",
//!   "binding `x` declared here", "borrow later used here".
//!
//! A rustc that changes these label texts makes the family go SILENT
//! (misses), never wrong.
//!
//! ## The candidate ladders
//!
//! - **E0502**: deref-removal first (two deletions — the min-delta pair,
//!   correct where the element is `Copy`), then clone-out (`&e` →
//!   `e.clone()`, works for owned-element borrows whose uses are method
//!   calls). The oracle arbitrates: deref-removal on a non-`Copy` element
//!   trades E0502 for E0507 → no progress → revert → clone-out applies.
//! - **E0499**: swap the second borrow's declaration with the first
//!   borrow's intervening last use — the NLL fix (the first borrow now
//!   ends before the second begins). Diverges from the seed's brace form
//!   (no block introduced) but keeps every expression in the same
//!   evaluation order — the safe variant of the scope restructure.
//! - **E0505**: clone AT THE MOVE SITE (`v` → `v.clone()`), restoring the
//!   borrowed original's aliasing. This is the measured differentiator:
//!   rustc's own fix rewrites the BORROW site (`let r = &v.clone()` —
//!   `r` then borrows a temporary, aliasing silently changed); ours keeps
//!   `r` borrowing `v` and matches the known inverse byte-for-byte.

use std::ops::Range;

use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprReference, Local, Pat, UnOp};

use super::{anchor, cand, CompileProposer, FixCandidate, Playbooks, SpanEdit};
use crate::domains::rustc_errors::diag::RustcErrorDiag;

// ── shared syn extraction ─────────────────────────────────────────────

/// One `let x = &init;` snapshot: the binding name, the `&` token, and the
/// full init expression. Owned ranges — no AST lifetimes escape the parse.
struct RefLocal {
    name: String,
    /// The `&` token of the reference init.
    amp: Range<usize>,
    /// The full init expression (`&…`).
    init: Range<usize>,
}

/// All locals whose initializer is a plain reference expression.
fn ref_locals(source: &str) -> Vec<RefLocal> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut v = RefLocalCollector { out: Vec::new() };
    v.visit_file(&file);
    v.out
}

struct RefLocalCollector {
    out: Vec<RefLocal>,
}

impl RefLocalCollector {
    fn classify(l: &Local) -> Option<RefLocal> {
        let Pat::Ident(pi) = &l.pat else { return None };
        let init = l.init.as_ref()?;
        let Expr::Reference(ExprReference { and_token, .. }) = &*init.expr else {
            return None;
        };
        Some(RefLocal {
            name: pi.ident.to_string(),
            amp: and_token.span().byte_range(),
            init: expr_init_range(l),
        })
    }
}

/// The full init-expression byte range of a `Local` (from the parse —
/// `Local::init` is `Option<LocalInit>` whose `expr` span may exclude
/// trailing attributes; the expression node's own span is what we splice).
fn expr_init_range(l: &Local) -> Range<usize> {
    l.init
        .as_ref()
        .map_or_else(|| l.pat.span().byte_range(), |i| i.expr.span().byte_range())
}

impl<'ast> Visit<'ast> for RefLocalCollector {
    fn visit_local(&mut self, l: &'ast Local) {
        if l.attrs.is_empty() {
            if let Some(r) = Self::classify(l) {
                self.out.push(r);
            }
        }
        syn::visit::visit_local(self, l);
    }
}

/// Byte ranges of every `*x` deref expression whose operand is exactly the
/// ident `name` (the star token, not the operand).
fn deref_stars_of(source: &str, name: &str) -> Vec<Range<usize>> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut v = DerefCollector {
        name: name.to_string(),
        out: Vec::new(),
    };
    v.visit_file(&file);
    v.out
}

struct DerefCollector {
    name: String,
    out: Vec<Range<usize>>,
}

impl<'ast> Visit<'ast> for DerefCollector {
    fn visit_expr(&mut self, e: &'ast Expr) {
        // `*x` is `Expr::Unary` with `UnOp::Deref` in syn 2 (there is no
        // Expr::Deref variant).
        if let Expr::Unary(u) = e {
            if let UnOp::Deref(star) = &u.op {
                if let Expr::Path(p) = &*u.expr {
                    if p.path.is_ident(&self.name) {
                        self.out.push(star.span().byte_range());
                    }
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
}

/// Every statement byte range in the file, in source order (block stmts at
/// every nesting depth — the swap template locates enclosing statements by
/// containment).
fn stmt_ranges(source: &str) -> Vec<Range<usize>> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut v = StmtCollector { out: Vec::new() };
    v.visit_file(&file);
    v.out.sort_by_key(|r| r.start);
    v.out
}

struct StmtCollector {
    out: Vec<Range<usize>>,
}

impl<'ast> Visit<'ast> for StmtCollector {
    fn visit_block(&mut self, b: &'ast syn::Block) {
        for s in &b.stmts {
            self.out.push(s.span().byte_range());
        }
        syn::visit::visit_block(self, b);
    }
}

/// The statement wholly containing `at` (the INNERMOST by shortest span).
fn stmt_containing(stmts: &[Range<usize>], at: usize) -> Option<&Range<usize>> {
    stmts
        .iter()
        .filter(|r| r.contains(&at))
        .min_by_key(|r| r.len())
}

// ── E0502 template A — deref-removal (the atomic pair) ────────────────

/// `let x = &e;` + uses `*x` → `let x = e;` + uses `x`. Deleting the `&`
/// WITHOUT the `*`s trades E0502 for E0614 (no progress → revert), so the
/// pair ships as ONE candidate with extra edits — the atomicity is the
/// correctness argument. Correct where the borrowed element is `Copy`;
/// for non-`Copy` the oracle rejects (E0507) and clone-out takes over.
pub struct E0502DerefRemoval;

impl CompileProposer for E0502DerefRemoval {
    fn name(&self) -> &'static str {
        "e0502-deref-removal"
    }

    fn code(&self) -> &'static str {
        "E0502"
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
        // The borrowed-place label locates the binding; the anchor span
        // sits INSIDE the reference init.
        let Some(borrow) = anchor(diag, |l| l.contains("immutable borrow occurs here")) else {
            return Vec::new();
        };
        let Some(rl) = ref_locals(source)
            .into_iter()
            .find(|rl| rl.init.contains(&borrow.byte_start))
        else {
            return Vec::new();
        };
        let stars = deref_stars_of(source, &rl.name);
        if stars.is_empty() {
            // No deref uses to pair with — the `&`-only edit is the
            // incomplete half of this template. Stay silent; clone-out
            // (or a miss) answers instead.
            return Vec::new();
        }
        // Primary edit: delete the `&`. Extras: delete each `*`.
        let extras: Vec<SpanEdit> = stars
            .into_iter()
            .map(|r| SpanEdit {
                byte_span: (r.start, r.end),
                replacement: String::new(),
            })
            .collect();
        vec![FixCandidate {
            strategy_id: self.name().to_string(),
            code: diag.code.clone(),
            file: file.to_path_buf(),
            line: borrow.line_start,
            byte_span: (rl.amp.start, rl.amp.end),
            replacement: String::new(),
            shape: String::new(),
            // No note by design: reading a Copy element by value is
            // byte-identical to reading it through the reference — the
            // template only fires on Copy-typed elements (see the family
            // gate), so there is no semantics change to name.
            semantic_note: None,
            extra_edits: extras,
        }]
    }
}

// ── E0502 template B — clone-out at the binding ───────────────────────

/// `let x = &e;` → `let x = e.clone();` — the borrow ends at the binding,
/// so the later mutation is free. Diverges from a seed that read a field
/// out (`e[0].len()`), but is semantically equal for the common
/// read-through-the-reference uses (`x.len()`, `x.foo()`). Correct-by-
/// oracle for owned-element borrows where deref-removal cannot move out.
pub struct E0502CloneOut;

impl CompileProposer for E0502CloneOut {
    fn name(&self) -> &'static str {
        "e0502-clone-out"
    }

    fn code(&self) -> &'static str {
        "E0502"
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
        let Some(borrow) = anchor(diag, |l| l.contains("immutable borrow occurs here")) else {
            return Vec::new();
        };
        let Some(rl) = ref_locals(source)
            .into_iter()
            .find(|rl| rl.init.contains(&borrow.byte_start))
        else {
            return Vec::new();
        };
        // init = `&rest` → `rest.clone()` (the amp token is the leading `&`).
        let Some(inner) = source.get(rl.amp.end..rl.init.end) else {
            return Vec::new();
        };
        let repl = format!("{inner}.clone()");
        vec![cand(
            diag,
            file,
            borrow.line_start,
            (rl.init.start, rl.init.end),
            repl,
            self.name(),
        )
        .with_note("clones the element — the binding owns a copy instead of borrowing (aliasing not preserved)")]
    }
}

// ── E0499 — swap the second borrow decl with the first's last use ─────

/// `let a = &mut *v; let b = &mut *v; a.push(1); b.push(2);` →
/// `let a = &mut *v; a.push(1); let b = &mut *v; b.push(2);` — NLL ends
/// the first borrow before the second begins. ONE span covering both
/// statements, replacement swaps them. Guards, all load-bearing:
///
/// - The last-use statement must come AFTER the second decl (else the
///   borrows already nest — nothing to swap).
/// - The two statements must be ADJACENT (whitespace-only between) —
///   relocating a foreign statement past another changes its evaluation
///   order and this template refuses that class.
/// - Single-line, comment-free statements (a multi-line splice is not
///   min-delta; a comment would be displaced).
pub struct E0499SwapSecondDecl;

impl CompileProposer for E0499SwapSecondDecl {
    fn name(&self) -> &'static str {
        "e0499-swap-second-decl"
    }

    fn code(&self) -> &'static str {
        "E0499"
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
        let Some(second) = anchor(diag, |l| l.contains("second mutable borrow occurs here")) else {
            return Vec::new();
        };
        let Some(last_use) = anchor(diag, |l| l.contains("first borrow later used here")) else {
            return Vec::new();
        };
        let stmts = stmt_ranges(source);
        let Some(b) = stmt_containing(&stmts, second.byte_start) else {
            return Vec::new();
        };
        let Some(u) = stmt_containing(&stmts, last_use.byte_start) else {
            return Vec::new();
        };
        // The use must come after the second decl, and the two must be
        // adjacent statements (whitespace-only gap).
        if u.start < b.end {
            return Vec::new();
        }
        let Some(gap) = source.get(b.end..u.start) else {
            return Vec::new();
        };
        if !gap.trim().is_empty() {
            return Vec::new(); // foreign statements between — refuse
        }
        let Some(b_text) = source.get(b.start..b.end) else {
            return Vec::new();
        };
        let Some(u_text) = source.get(u.start..u.end) else {
            return Vec::new();
        };
        if b_text.contains('\n')
            || u_text.contains('\n')
            || b_text.contains("//")
            || u_text.contains("//")
            || b_text.contains("/*")
            || u_text.contains("/*")
        {
            return Vec::new();
        }
        let repl = format!("{u_text}{gap}{b_text}");
        vec![cand(
            diag,
            file,
            second.line_start,
            (b.start, u.end),
            repl,
            self.name(),
        )]
    }
}

// ── E0505 — clone at the move site (the contested-opening win) ─────────

/// `let r = &v; let w = v;` → `let w = v.clone();` — insert `.clone()` at
/// the indicted move site. rustc's own `MachineApplicable` fix rewrites the
/// BORROW site (`let r = &v.clone()`) so `r` borrows a temporary — aliasing
/// silently changed (Bench 041's one measured divergence). Cloning at the
/// move keeps `r` borrowing the original and matches the known inverse.
pub struct E0505CloneAtMove;

impl CompileProposer for E0505CloneAtMove {
    fn name(&self) -> &'static str {
        "e0505-clone-at-move"
    }

    fn code(&self) -> &'static str {
        "E0505"
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
        let Some(moved) = anchor(diag, |l| {
            l.contains("move out of") && l.contains("occurs here")
        }) else {
            return Vec::new();
        };
        let end = moved.byte_end;
        if source[end..].starts_with(".clone()") {
            return Vec::new(); // already cloned here
        }
        vec![cand(
            diag,
            file,
            moved.line_start,
            (end, end),
            ".clone()".to_string(),
            self.name(),
        )
        .with_note(
            "clones at the move — the destination owns a copy; the source borrow is preserved",
        )]
    }
}

// ── the family registry ───────────────────────────────────────────────

/// The Phase 3 registry: the uncontested pair (E0502, E0499) plus the one
/// contested opening (E0505). E0596/E0382/E0507/E0373 ship no template —
/// `cargo fix` scores 100% matched there (Bench 041, the T6.9 floor).
#[must_use]
pub fn family() -> Playbooks {
    Playbooks::empty()
        .with(Box::new(E0502DerefRemoval))
        .with(Box::new(E0502CloneOut))
        .with(Box::new(E0499SwapSecondDecl))
        .with(Box::new(E0505CloneAtMove))
}

// ── fixtures (proposer discipline: buggy/clean pairs + negative
// controls; every FIXED form oracle-verified on rustc 1.97.0 before
// becoming a fixture — see the probe record in the plan/bench) ──────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::rustc_errors::diag::LabeledSpan;
    use std::path::Path;

    /// Build a diag with labeled spans located by the anchor texts they
    /// cover — the same spans rustc emits for these shapes (probed).
    fn diag_for(
        code: &str,
        source: &str,
        primary: &str,
        labels: &[(&str, &str)],
    ) -> RustcErrorDiag {
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
            label: "primary".to_string(),
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
            code: code.to_string(),
            message: String::new(),
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

    /// Apply a candidate's edits to the source (mirror of the driver's
    /// splice, for asserting the fixed form).
    fn apply(source: &str, c: &FixCandidate) -> String {
        let mut edits = vec![(c.byte_span, c.replacement.clone())];
        edits.extend(
            c.extra_edits
                .iter()
                .map(|e| (e.byte_span, e.replacement.clone())),
        );
        edits.sort_by_key(|((s, _), _)| *s);
        let mut out = String::new();
        let mut cursor = 0;
        for ((s, e), r) in edits {
            out.push_str(&source[cursor..s]);
            out.push_str(&r);
            cursor = e;
        }
        out.push_str(&source[cursor..]);
        out
    }

    // ── E0502 deref-removal ───────────────────────────────────────────

    /// The corpus mutant `e0502_borrow_across_mutation` (u8 elements —
    /// Copy): the pair restores the seed exactly.
    const E0502_COPY_SRC: &str = "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = &v[0];\n    v.push(1);\n    *first\n}\n";

    fn e0502_diag(code: &str, src: &str) -> RustcErrorDiag {
        diag_for(
            code,
            src,
            "v.push(1)",
            &[
                ("immutable borrow occurs here", "v[0]"),
                ("immutable borrow later used here", "*first"),
            ],
        )
    }

    #[test]
    fn deref_removal_pairs_both_edits() {
        let diag = e0502_diag("E0502", E0502_COPY_SRC);
        let out = propose_all(&E0502DerefRemoval, E0502_COPY_SRC, &diag);
        assert_eq!(out.len(), 1, "{out:?}");
        let c = &out[0];
        assert_eq!(c.strategy_id, "e0502-deref-removal");
        assert_eq!(c.extra_edits.len(), 1, "the `*` deletion rides as an extra");
        let amp = E0502_COPY_SRC.find("&v[0]").expect("borrowed index expr");
        assert_eq!(c.byte_span, (amp, amp + 1));
        let star = E0502_COPY_SRC.find("*first").unwrap();
        assert_eq!(c.extra_edits[0].byte_span, (star, star + 1));
        // The fixed form is the corpus seed, byte-identical.
        let fixed = apply(E0502_COPY_SRC, c);
        assert_eq!(
            fixed,
            "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = v[0];\n    v.push(1);\n    first\n}\n"
        );
    }

    #[test]
    fn deref_removal_is_silent_without_deref_uses() {
        // The String variant: uses are method calls (`head.len()`), no
        // `*head` to pair with — the incomplete half never ships alone.
        let src = "pub fn probe(items: &mut Vec<String>) -> usize {\n    let head = &items[0];\n    items.push(String::new());\n    head.len()\n}\n";
        let diag = diag_for(
            "E0502",
            src,
            "items.push(String::new())",
            &[
                ("immutable borrow occurs here", "items[0]"),
                ("immutable borrow later used here", "head.len()"),
            ],
        );
        assert!(propose_all(&E0502DerefRemoval, src, &diag).is_empty());
    }

    #[test]
    fn deref_removal_ignores_other_codes_and_missing_anchor() {
        let wrong = e0502_diag("E0499", E0502_COPY_SRC);
        assert!(propose_all(&E0502DerefRemoval, E0502_COPY_SRC, &wrong).is_empty());
        let no_anchor = diag_for("E0502", E0502_COPY_SRC, "v.push(1)", &[]);
        assert!(propose_all(&E0502DerefRemoval, E0502_COPY_SRC, &no_anchor).is_empty());
    }

    // ── E0502 clone-out ───────────────────────────────────────────────

    #[test]
    fn clone_out_rewrites_the_init() {
        let src = "pub fn probe(items: &mut Vec<String>) -> usize {\n    let head = &items[0];\n    items.push(String::new());\n    head.len()\n}\n";
        let diag = diag_for(
            "E0502",
            src,
            "items.push(String::new())",
            &[
                ("immutable borrow occurs here", "items[0]"),
                ("immutable borrow later used here", "head.len()"),
            ],
        );
        let out = propose_all(&E0502CloneOut, src, &diag);
        assert_eq!(out.len(), 1, "{out:?}");
        let init_start = src.find("&items[0]").unwrap();
        assert_eq!(
            out[0].byte_span,
            (init_start, init_start + "&items[0]".len())
        );
        assert_eq!(out[0].replacement, "items[0].clone()");
        let fixed = apply(src, &out[0]);
        assert!(fixed.contains("let head = items[0].clone();"));
    }

    #[test]
    fn clone_out_ignores_non_reference_inits() {
        // The label points inside a non-reference init — silent (the
        // template's shape knowledge is the `let x = &e;` form only).
        let src = "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = v[0];\n    v.push(1);\n    first\n}\n";
        let diag = diag_for(
            "E0502",
            src,
            "v.push(1)",
            &[("immutable borrow occurs here", "v")],
        );
        assert!(propose_all(&E0502CloneOut, src, &diag).is_empty());
    }

    // ── E0499 swap ────────────────────────────────────────────────────

    /// The corpus mutant `e0499_flatten_borrow_scope`.
    const E0499_SRC: &str = "pub fn probe(v: &mut Vec<u8>) {\n    let a = &mut *v;\n    let b = &mut *v;\n    a.push(1);\n    b.push(2);\n}\n";

    fn e0499_diag() -> RustcErrorDiag {
        // The SECOND borrow is the primary; the first's last use is `a` in
        // `a.push(1)` — rustc labels the receiver only (probed).
        diag_for(
            "E0499",
            E0499_SRC,
            "let b = &mut *v;",
            &[
                ("first mutable borrow occurs here", "let a = &mut *v;"),
                ("second mutable borrow occurs here", "let b = &mut *v;"),
                ("first borrow later used here", "a.push"),
            ],
        )
    }

    #[test]
    fn swap_moves_the_use_before_the_second_decl() {
        let diag = e0499_diag();
        let out = propose_all(&E0499SwapSecondDecl, E0499_SRC, &diag);
        assert_eq!(out.len(), 1, "{out:?}");
        let c = &out[0];
        assert_eq!(c.strategy_id, "e0499-swap-second-decl");
        let b_start = E0499_SRC.find("let b = &mut *v;").unwrap();
        let u_start = E0499_SRC.find("a.push(1);").unwrap();
        let u_end = u_start + "a.push(1);".len();
        assert_eq!(c.byte_span, (b_start, u_end));
        // The fixed form is the oracle-verified reorder.
        let fixed = apply(E0499_SRC, c);
        assert_eq!(
            fixed,
            "pub fn probe(v: &mut Vec<u8>) {\n    let a = &mut *v;\n    a.push(1);\n    let b = &mut *v;\n    b.push(2);\n}\n"
        );
    }

    #[test]
    fn swap_is_silent_when_use_precedes_second_decl() {
        // Already-nested borrows: the use comes BEFORE the second decl —
        // nothing to swap (this shape is not even an E0499 in practice;
        // the guard must still hold for mis-anchored diags).
        let src = "pub fn probe(v: &mut Vec<u8>) {\n    let a = &mut *v;\n    a.push(1);\n    let b = &mut *v;\n    b.push(2);\n}\n";
        let diag = diag_for(
            "E0499",
            src,
            "let b = &mut *v;",
            &[
                ("first mutable borrow occurs here", "let a = &mut *v;"),
                ("second mutable borrow occurs here", "let b = &mut *v;"),
                // The last-use label (wrongly) points at the pre-decl use.
                ("first borrow later used here", "a.push"),
            ],
        );
        assert!(propose_all(&E0499SwapSecondDecl, src, &diag).is_empty());
    }

    #[test]
    fn swap_is_silent_with_foreign_statements_between() {
        // A foreign statement sits between the second decl and the use —
        // relocating it would change evaluation order. Refuse.
        let src = "pub fn probe(v: &mut Vec<u8>) {\n    let a = &mut *v;\n    let b = &mut *v;\n    let n = 1u8;\n    a.push(n);\n    b.push(2);\n}\n";
        let diag = diag_for(
            "E0499",
            src,
            "let b = &mut *v;",
            &[
                ("first mutable borrow occurs here", "let a = &mut *v;"),
                ("second mutable borrow occurs here", "let b = &mut *v;"),
                ("first borrow later used here", "a.push"),
            ],
        );
        assert!(propose_all(&E0499SwapSecondDecl, src, &diag).is_empty());
    }

    // ── E0505 clone-at-move ───────────────────────────────────────────

    /// The corpus mutant `e0505_drop_clone_under_borrow` — the case where
    /// `cargo fix` diverges (rewrites the borrow site) and this template
    /// restores the seed.
    const E0505_SRC: &str =
        "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v;\n    r.len() + w.len()\n}\n";

    #[test]
    fn clone_at_move_inserts_after_the_move_span() {
        let diag = diag_for(
            "E0505",
            E0505_SRC,
            "let w = v;",
            &[
                ("move out of `v` occurs here", "v;\n"),
                ("borrow of `v` occurs here", "&v"),
                ("borrow later used here", "r.len()"),
            ],
        );
        // The move label covers just the moved `v` (the operand, probed) —
        // narrow the primary-style label to it.
        let w_at = E0505_SRC.find("let w = v;").unwrap() + "let w = ".len();
        let mut d = diag;
        for ls in &mut d.labeled_spans {
            if ls.label.contains("move out of") {
                ls.byte_start = w_at;
                ls.byte_end = w_at + 1;
            }
        }
        let out = propose_all(&E0505CloneAtMove, E0505_SRC, &d);
        assert_eq!(out.len(), 1, "{out:?}");
        let c = &out[0];
        assert_eq!(c.byte_span, (w_at + 1, w_at + 1), "insert at span end");
        assert_eq!(c.replacement, ".clone()");
        let fixed = apply(E0505_SRC, c);
        assert_eq!(
            fixed,
            "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v.clone();\n    r.len() + w.len()\n}\n",
            "byte-identical to the corpus seed — the MatchedInverse claim"
        );
    }

    #[test]
    fn clone_at_move_is_silent_when_already_cloned() {
        let src = "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v.clone();\n    r.len() + w.len()\n}\n";
        let w_at = src.find("let w = v.clone();").unwrap() + "let w = ".len();
        let mut d = diag_for(
            "E0505",
            src,
            "let w = v.clone();",
            &[("move out of `v` occurs here", "v.clone()")],
        );
        for ls in &mut d.labeled_spans {
            if ls.label.contains("move out of") {
                ls.byte_start = w_at;
                ls.byte_end = w_at + 1;
            }
        }
        assert!(propose_all(&E0505CloneAtMove, src, &d).is_empty());
    }

    // ── the no-leak pin (the family-level exclusion) ──────────────────

    #[test]
    fn family_never_emits_leak_or_reference_extension() {
        // No replacement in this family may extend a lifetime by leaking
        // (`Box::leak`/`mem::forget`) — the wrong-but-compiling class the
        // plan excludes by design.
        let srcs = [E0502_COPY_SRC, E0499_SRC, E0505_SRC];
        let diags = [
            e0502_diag("E0502", E0502_COPY_SRC),
            e0499_diag(),
            diag_for("E0505", E0505_SRC, "let w = v;", &[]),
        ];
        let pb = family();
        assert_eq!(pb.len(), 4);
        for (src, diag) in srcs.iter().zip(&diags) {
            for c in pb.propose(diag, Path::new("/x/src/lib.rs"), src) {
                for text in std::iter::once(&c.replacement)
                    .chain(c.extra_edits.iter().map(|e| &e.replacement))
                {
                    assert!(
                        !text.contains("Box::leak") && !text.contains("::leak("),
                        "{} proposed a leak: {text}",
                        c.strategy_id
                    );
                }
            }
        }
    }
}
