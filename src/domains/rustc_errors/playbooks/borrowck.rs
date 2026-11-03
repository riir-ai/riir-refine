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
//!   calls), then read-relocate (the mid-read reorder — the immutable-later
//!   shape only, anchored on the borrow + the conflicting mutable's last
//!   use). The oracle arbitrates: deref-removal on a non-`Copy` element
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
use crate::domains::rustc_errors::diag::{LabeledSpan, RustcErrorDiag};

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
            diag_digest: None,
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

// ── E0502 template C — read-relocate (the hoist-to-first-use) ─────────

/// `let head = &v[0]; …conflicting borrows…; *head` → hoist the read to
/// JUST BEFORE its first use, by value: delete the decl statement,
/// re-insert `let head = v[0];` immediately before the first statement that
/// references the binding, drop each `*` star. On the mid-read shape (F3:
/// the read sits inside the conflicting borrows' live region but its first
/// USE is after they all end) this is exactly rustc's own documented fix
/// shape — declare just before first use — and it clears the E0502 while the
/// E0499 remains for the swap to finish (KEEP 2→1, then swap KEEP 1→0).
///
/// Anchors: the diagnostic's OWN `immutable borrow occurs here` label locates
/// the ref-local; the first-use statement is found in the SOURCE (syn stmt
/// ranges + an ident-boundary scan — the same machinery as the star
/// collector). Min-delta ordering makes this the THIRD rung (deref-removal,
/// clone-out first), so it fires only where they revert or stay silent.
///
/// Guards, all load-bearing (the family discipline: a guard that cannot hold
/// locally makes the template SILENT — rustc drift degrades to misses, never
/// wrong edits):
///
/// - The first-use statement must come strictly AFTER the decl statement
///   (else there is nothing to hoist past).
/// - Decl and first-use statements single-line, comment-free (the E0499
///   template's splice guards).
/// - Correct where the element is `Copy`; for non-`Copy` the oracle rejects
///   the by-value re-insert (E0507/E0508 move-out) → revert → clone-out
///   remains that stratum's repair. A first use still INSIDE a conflicting
///   borrow region re-fires E0502 → revert (the same oracle arbitration
///   deref-removal already relies on).
pub struct E0502ReadRelocate;

impl CompileProposer for E0502ReadRelocate {
    fn name(&self) -> &'static str {
        "e0502-read-relocate"
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
        let stmts = stmt_ranges(source);
        let Some(decl) = stmt_containing(&stmts, rl.init.start) else {
            return Vec::new();
        };
        // The FIRST statement after the decl that references the binding —
        // the hoist target. Found by ident-boundary scan of the stmt text
        // (a plain `contains` would match `head2`).
        let Some(first_use) = stmts
            .iter()
            .filter(|s| s.start > decl.end)
            .find(|s| {
                source
                    .get(s.start..s.end)
                    .is_some_and(|t| contains_ident(t, &rl.name))
            })
            .cloned()
        else {
            return Vec::new();
        };
        let Some(decl_text) = source.get(decl.start..decl.end) else {
            return Vec::new();
        };
        let Some(use_text) = source.get(first_use.start..first_use.end) else {
            return Vec::new();
        };
        if decl_text.contains('\n')
            || use_text.contains('\n')
            || decl_text.contains("//")
            || use_text.contains("//")
            || decl_text.contains("/*")
            || use_text.contains("/*")
        {
            return Vec::new();
        }
        // The by-value place text: `&place` → `place` (the clone-out slice).
        let Some(place) = source.get(rl.amp.end..rl.init.end) else {
            return Vec::new();
        };
        // Full-line deletion of the decl (indent + stmt + newline); the
        // re-insert lands on its own line immediately before the first-use
        // statement, mirroring that line's indentation.
        let del_line_start = source[..decl.start].rfind('\n').map_or(0, |i| i + 1);
        let mut del_end = decl.end;
        if source[del_end..].starts_with('\n') {
            del_end += 1;
        }
        let use_line_start = source[..first_use.start].rfind('\n').map_or(0, |i| i + 1);
        let Some(use_indent) = source.get(use_line_start..first_use.start) else {
            return Vec::new();
        };
        // The use stmt must START ITS OWN LINE (indent-only prefix) — an
        // insertion into mid-line code is a splice we refuse.
        if !use_indent.trim().is_empty() {
            return Vec::new();
        }
        let insert = format!(
            "{indent}let {name} = {place};\n",
            indent = use_indent,
            name = rl.name
        );
        let mut extras: Vec<SpanEdit> = Vec::with_capacity(1 + 1);
        extras.push(SpanEdit {
            byte_span: (use_line_start, use_line_start),
            replacement: insert,
        });
        for s in deref_stars_of(source, &rl.name) {
            extras.push(SpanEdit {
                byte_span: (s.start, s.end),
                replacement: String::new(),
            });
        }
        vec![FixCandidate {
            strategy_id: self.name().to_string(),
            code: diag.code.clone(),
            file: file.to_path_buf(),
            line: borrow.line_start,
            byte_span: (del_line_start, del_end),
            replacement: String::new(),
            shape: String::new(),
            semantic_note: Some(
                "hoists the read to just before its first use — the value read \
                 differs if the statements crossed mutated the indexed place",
            ),
            diag_digest: None,
            extra_edits: extras,
        }]
    }
}

/// Ident-boundary containment: `text` mentions the identifier `name` as a
/// standalone token (not as a substring of a longer identifier).
fn contains_ident(text: &str, name: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut word = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if is_ident(c) {
            word.push(c);
        } else {
            if word == name {
                return true;
            }
            word.clear();
        }
    }
    false
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

// ── E0505 — hoist the borrow's use above the move (the non-Clone rung) ──

/// `let r = &t; let w = t; let n = r.v;` → swap the move statement with the
/// borrow's first-use statement (they must be adjacent) — NLL ends the
/// borrow before the move, no trait requirement. The SECOND rung of the
/// E0505 ladder: [`E0505CloneAtMove`] keeps on `Clone` types but reverts on
/// non-`Clone` ones (E0599 at equal count — no strict lexicographic
/// decrease), and this strategy recovers the site the clone rung cannot.
/// Anchors on rustc's own `borrow later used here` label, never on a
/// guessed binding name.
pub struct E0505UseBeforeMove;

impl CompileProposer for E0505UseBeforeMove {
    fn name(&self) -> &'static str {
        "e0505-use-before-move"
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
        let Some(borrow_use) = anchor(diag, |l| l.contains("borrow later used here")) else {
            return Vec::new();
        };
        swap_move_with_use(diag, file, source, moved, borrow_use, self.name())
    }
}

// ── E0382 — the move ladder (clone rung + use-before-move rung) ────────

/// `let w = t; let n = t.v;` → `let w = t.clone();` — insert `.clone()` at
/// the indicted `value moved here` place. The FIRST rung of the E0382
/// ladder: keeps on `Clone` types, reverts on non-`Clone` (E0599 at equal
/// count) so the [`E0382UseBeforeMove`] rung takes over — the same ladder
/// shape [`E0505CloneAtMove`] established one code over.
pub struct E0382CloneAtMove;

impl CompileProposer for E0382CloneAtMove {
    fn name(&self) -> &'static str {
        "e0382-clone-at-move"
    }

    fn code(&self) -> &'static str {
        "E0382"
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
        let Some(moved) = anchor(diag, |l| l.contains("value moved here")) else {
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
        .with_note("clones at the move — the destination owns a copy; the source stays live")]
    }
}

/// `let w = t; let n = t.v;` → swap the two statements (they must be
/// adjacent) — the use lands before the move. The SECOND rung of the E0382
/// ladder and the only one available on non-`Clone` types.
pub struct E0382UseBeforeMove;

impl CompileProposer for E0382UseBeforeMove {
    fn name(&self) -> &'static str {
        "e0382-use-before-move"
    }

    fn code(&self) -> &'static str {
        "E0382"
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
        let Some(moved) = anchor(diag, |l| l.contains("value moved here")) else {
            return Vec::new();
        };
        let Some(usex) = anchor(diag, |l| l.contains("value used here after move")) else {
            return Vec::new();
        };
        swap_move_with_use(diag, file, source, moved, usex, self.name())
    }
}

/// The shared adjacent-statement swap behind both use-before-move rungs —
/// the [`E0499SwapSecondDecl`] discipline applied to moves: the use must
/// come AFTER the move, the two statements must be ADJACENT
/// (whitespace-only gap), single-line and comment-free, and the use must
/// not reference the move's DESTINATION binding (that would be a use
/// before its new declaration site after the swap).
fn swap_move_with_use(
    diag: &RustcErrorDiag,
    file: &std::path::Path,
    source: &str,
    moved: &LabeledSpan,
    usex: &LabeledSpan,
    strategy: &'static str,
) -> Vec<FixCandidate> {
    let stmts = stmt_ranges(source);
    let Some(m) = stmt_containing(&stmts, moved.byte_start) else {
        return Vec::new();
    };
    let Some(u) = stmt_containing(&stmts, usex.byte_start) else {
        return Vec::new();
    };
    if u.start < m.end {
        return Vec::new(); // the use is not after the move — nothing to swap
    }
    let Some(gap) = source.get(m.end..u.start) else {
        return Vec::new();
    };
    if !gap.trim().is_empty() {
        return Vec::new(); // foreign statements between — refuse
    }
    let Some(m_text) = source.get(m.start..m.end) else {
        return Vec::new();
    };
    let Some(u_text) = source.get(u.start..u.end) else {
        return Vec::new();
    };
    if m_text.contains('\n')
        || u_text.contains('\n')
        || m_text.contains("//")
        || u_text.contains("//")
        || m_text.contains("/*")
        || u_text.contains("/*")
    {
        return Vec::new();
    }
    // The destination binding: `let NAME = place;` — the use must not
    // mention NAME (post-swap that would read the binding before its
    // declaration).
    let Some(dest) = m_text
        .strip_prefix("let ")
        .and_then(|rest| rest.split_whitespace().next())
    else {
        return Vec::new();
    };
    if contains_ident(u_text, dest) {
        return Vec::new();
    }
    let repl = format!("{u_text}{gap}{m_text}");
    vec![cand(
        diag,
        file,
        moved.line_start,
        (m.start, u.end),
        repl,
        strategy,
    )
    .with_note(
        "swaps the move with the use — evaluation order of the two statements \
         exchanges; legal only when neither reads the other's result",
    )]
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
        .with(Box::new(E0502ReadRelocate))
        .with(Box::new(E0499SwapSecondDecl))
        .with(Box::new(E0505CloneAtMove))
        .with(Box::new(E0505UseBeforeMove))
        .with(Box::new(E0382CloneAtMove))
        .with(Box::new(E0382UseBeforeMove))
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
        assert_eq!(propose_all(&E0502DerefRemoval, src, &diag), []);
    }

    #[test]
    fn deref_removal_ignores_other_codes_and_missing_anchor() {
        let wrong = e0502_diag("E0499", E0502_COPY_SRC);
        assert_eq!(propose_all(&E0502DerefRemoval, E0502_COPY_SRC, &wrong), []);
        let no_anchor = diag_for("E0502", E0502_COPY_SRC, "v.push(1)", &[]);
        assert_eq!(
            propose_all(&E0502DerefRemoval, E0502_COPY_SRC, &no_anchor),
            []
        );
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
        assert_eq!(propose_all(&E0502CloneOut, src, &diag), []);
    }

    // ── E0502 read-relocate (Issue 101 — hoist to first use) ──────────

    /// The COLD F3 state (the measured firing surface): the read sits inside
    /// the conflicting borrows' live region; its first USE (the tail) is
    /// after they all end. The hoist clears the E0502 (KEEP 2→1); the E0499
    /// remains for the swap (KEEP 1→0) — clean in 4 attempts.
    const RELOCATE_COLD: &str = "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let a = &mut *v;\n    let head = &v[0];\n    let b = &mut *v;\n    a.push(4i32);\n    b.push(5i32);\n    *head + v[0]\n}\n";

    const RELOCATE_COLD_HOISTED: &str = "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let a = &mut *v;\n    let b = &mut *v;\n    a.push(4i32);\n    b.push(5i32);\n    let head = v[0];\n    head + v[0]\n}\n";

    fn relocate_diag(code: &str, src: &str) -> RustcErrorDiag {
        diag_for(
            code,
            src,
            "&v[0]",
            &[
                ("mutable borrow occurs here", "&mut *v"),
                ("mutable borrow later used here", "a.push(4i32)"),
                ("immutable borrow occurs here", "&v[0]"),
            ],
        )
    }

    #[test]
    fn read_relocate_hoists_the_decl_to_its_first_use() {
        let diag = relocate_diag("E0502", RELOCATE_COLD);
        let out = propose_all(&E0502ReadRelocate, RELOCATE_COLD, &diag);
        assert_eq!(out.len(), 1, "must fire on the cold mid-read shape");
        let c = &out[0];
        assert_eq!(c.strategy_id, "e0502-read-relocate");
        // Full-line delete + by-value insert before the first use + star
        // deletion = 3 edits, atomic.
        assert_eq!(c.extra_edits.len(), 2, "insert + one star");
        assert!(c.semantic_note.is_some(), "relocation is semantics-named");
        assert_eq!(apply(RELOCATE_COLD, c), RELOCATE_COLD_HOISTED);
    }

    /// The POST-SWAP state fires too — the hoist lands before the tail,
    /// after every borrow region (diverged-from-seed but compiling; the
    /// oracle + G1 arbitrate).
    #[test]
    fn read_relocate_fires_on_the_post_swap_state() {
        let post_swap = "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let a = &mut *v;\n    let head = &v[0];\n    a.push(4i32);\n    let b = &mut *v;\n    b.push(5i32);\n    *head + v[0]\n}\n";
        let diag = relocate_diag("E0502", post_swap);
        let out = propose_all(&E0502ReadRelocate, post_swap, &diag);
        assert_eq!(out.len(), 1);
        assert_eq!(
            apply(post_swap, &out[0]),
            "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let a = &mut *v;\n    a.push(4i32);\n    let b = &mut *v;\n    b.push(5i32);\n    let head = v[0];\n    head + v[0]\n}\n"
        );
    }

    #[test]
    fn read_relocate_is_silent_without_a_later_use_or_anchor() {
        // No use of the binding after the decl — nothing to hoist past.
        let no_use = "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let a = &mut *v;\n    let head = &v[0];\n    a.push(4i32);\n    v[0]\n}\n";
        let diag = relocate_diag("E0502", no_use);
        assert_eq!(propose_all(&E0502ReadRelocate, no_use, &diag), []);
        // Only a LONGER-IDENT mention later (`head2`) — the ident-boundary
        // scan must not treat it as a use of `head`.
        let longer_ident = "pub fn probe(v: &mut Vec<i32>) -> i32 {\n    let a = &mut *v;\n    let head = &v[0];\n    let head2 = v[1];\n    a.push(4i32);\n    head2 + v[0]\n}\n";
        let d2 = relocate_diag("E0502", longer_ident);
        assert_eq!(propose_all(&E0502ReadRelocate, longer_ident, &d2), []);
        // Wrong code.
        let d3 = relocate_diag("E0499", RELOCATE_COLD);
        assert_eq!(propose_all(&E0502ReadRelocate, RELOCATE_COLD, &d3), []);
        // No `immutable borrow occurs here` anchor (the E0505 shape).
        let d4 = diag_for("E0502", RELOCATE_COLD, "&v[0]", &[]);
        assert_eq!(propose_all(&E0502ReadRelocate, RELOCATE_COLD, &d4), []);
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
        assert_eq!(propose_all(&E0499SwapSecondDecl, src, &diag), []);
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
        assert_eq!(propose_all(&E0499SwapSecondDecl, src, &diag), []);
    }

    // ── E0505 clone-at-move ───────────────────────────────────────────

    /// The corpus mutant `e0505_drop_clone_under_borrow` — the case where
    /// `cargo fix` diverges (rewrites the borrow site) and this template
    /// restores the seed.
    const E0505_SRC: &str = "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v;\n    r.len() + w.len()\n}\n";

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
        assert_eq!(propose_all(&E0505CloneAtMove, src, &d), []);
    }

    // ── the no-leak pin (the family-level exclusion) ──────────────────

    #[test]
    fn family_never_emits_leak_or_reference_extension() {
        // No replacement in this family may extend a lifetime by leaking
        // (`Box::leak`/`mem::forget`) — the wrong-but-compiling class the
        // plan excludes by design.
        let srcs = [E0502_COPY_SRC, RELOCATE_COLD, E0499_SRC, E0505_SRC];
        let diags = [
            e0502_diag("E0502", E0502_COPY_SRC),
            relocate_diag("E0502", RELOCATE_COLD),
            e0499_diag(),
            diag_for("E0505", E0505_SRC, "let w = v;", &[]),
        ];
        let pb = family();
        assert_eq!(pb.len(), 8);
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

    // ── E0505 use-before-move (the non-Clone rung) ───────────────────

    /// The Plan-432 non-`Clone` saddle shape: the borrow's use is its own
    /// statement AFTER the move. The swap restores the seed exactly.
    const E0505_NC_SRC: &str = "pub fn probe(t: Tkn) -> u64 {\n    let r = &t;\n    let w = t;\n    let n = r.v;\n    n + w.v\n}\n";

    fn e0505_nc_diag() -> RustcErrorDiag {
        let mut d = diag_for(
            "E0505",
            E0505_NC_SRC,
            "let w = t;",
            &[
                ("move out of `t` occurs here", "w = t;\n"),
                ("borrow of `t` occurs here", "&t"),
                ("borrow later used here", "r.v"),
            ],
        );
        // The move label covers just the moved `t` (the operand, probed).
        let t_at = E0505_NC_SRC.find("w = t;").unwrap() + "w = ".len();
        for ls in &mut d.labeled_spans {
            if ls.label.contains("move out of") {
                ls.byte_start = t_at;
                ls.byte_end = t_at + 1;
            }
        }
        d
    }

    #[test]
    fn e0505_use_before_move_swaps_adjacent_stmts() {
        let d = e0505_nc_diag();
        let out = propose_all(&E0505UseBeforeMove, E0505_NC_SRC, &d);
        assert_eq!(out.len(), 1, "{out:?}");
        let fixed = apply(E0505_NC_SRC, &out[0]);
        assert_eq!(
            fixed,
            "pub fn probe(t: Tkn) -> u64 {\n    let r = &t;\n    let n = r.v;\n    let w = t;\n    n + w.v\n}\n",
            "byte-identical to the corpus seed — the saddle's resolving rung"
        );
        assert_eq!(out[0].strategy_id, "e0505-use-before-move");
    }

    #[test]
    fn e0505_use_before_move_refuses_inline_tail_use() {
        // The committed `E0505_SRC` shape: the borrow's use is inline in the
        // tail and mentions the move destination — the swap would read `w`
        // before its declaration. Refuse.
        let d = diag_for(
            "E0505",
            E0505_SRC,
            "let w = v;",
            &[
                ("move out of `v` occurs here", "v;\n"),
                ("borrow of `v` occurs here", "&v"),
                ("borrow later used here", "r.len()"),
            ],
        );
        assert_eq!(propose_all(&E0505UseBeforeMove, E0505_SRC, &d), []);
    }

    #[test]
    fn e0505_use_before_move_refuses_foreign_stmt_between() {
        let src = "pub fn probe(t: Tkn) -> u64 {\n    let r = &t;\n    let w = t;\n    let k = 5u64;\n    let n = r.v;\n    n + w.v + k\n}\n";
        let d = diag_for(
            "E0505",
            src,
            "let w = t;",
            &[
                ("move out of `t` occurs here", "w = t;\n"),
                ("borrow of `t` occurs here", "&t"),
                ("borrow later used here", "r.v"),
            ],
        );
        assert_eq!(propose_all(&E0505UseBeforeMove, src, &d), []);
    }

    // ── E0382 — the move ladder ───────────────────────────────────────

    const E0382_NC_SRC: &str =
        "pub fn probe(t: Tkn) -> u64 {\n    let w = t;\n    let n = t.v;\n    n + w.v\n}\n";

    fn e0382_nc_diag() -> RustcErrorDiag {
        let mut d = diag_for(
            "E0382",
            E0382_NC_SRC,
            "let n = t.v;",
            &[
                ("value moved here", "w = t;\n"),
                ("value used here after move", "t.v"),
            ],
        );
        // The moved-here label covers just the operand `t` (probed).
        let t_at = E0382_NC_SRC.find("w = t;").unwrap() + "w = ".len();
        for ls in &mut d.labeled_spans {
            if ls.label.contains("value moved here") {
                ls.byte_start = t_at;
                ls.byte_end = t_at + 1;
            }
        }
        d
    }

    #[test]
    fn e0382_clone_at_move_inserts_after_the_moved_place() {
        let d = e0382_nc_diag();
        let out = propose_all(&E0382CloneAtMove, E0382_NC_SRC, &d);
        assert_eq!(out.len(), 1, "{out:?}");
        let fixed = apply(E0382_NC_SRC, &out[0]);
        assert_eq!(
            fixed,
            "pub fn probe(t: Tkn) -> u64 {\n    let w = t.clone();\n    let n = t.v;\n    n + w.v\n}\n",
            "clone inserted at the move — the first rung (E0599 on non-Clone)"
        );
    }

    #[test]
    fn e0382_use_before_move_swaps_adjacent_stmts() {
        let d = e0382_nc_diag();
        let out = propose_all(&E0382UseBeforeMove, E0382_NC_SRC, &d);
        assert_eq!(out.len(), 1, "{out:?}");
        let fixed = apply(E0382_NC_SRC, &out[0]);
        assert_eq!(
            fixed,
            "pub fn probe(t: Tkn) -> u64 {\n    let n = t.v;\n    let w = t;\n    n + w.v\n}\n",
            "byte-identical to the corpus seed"
        );
    }

    #[test]
    fn e0382_use_before_move_refuses_foreign_stmt_between() {
        let src = "pub fn probe(t: Tkn) -> u64 {\n    let w = t;\n    let k = 5u64;\n    let n = t.v;\n    n + w.v + k\n}\n";
        let d = diag_for(
            "E0382",
            src,
            "let n = t.v;",
            &[
                ("value moved here", "w = t;\n"),
                ("value used here after move", "t.v"),
            ],
        );
        assert_eq!(propose_all(&E0382UseBeforeMove, src, &d), []);
    }
}
