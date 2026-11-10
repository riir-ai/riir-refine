//! `clippy::needless_borrowed_reference` — `let &ref y = e;` →
//! `let ref y = e;` (the leading `&` deleted) (Batch 184 / Plan 179 —
//! the coverage-report pack XVI; the plan158 RETIRE overturned on
//! plan160/plan169's measured firing shapes. Probe:
//! `scripts/plan179_pack16_probe.py`).
//!
//! ## Why the deletion is construction-identity (the FIXABLE bar)
//!
//! Upstream's own suggestion is a one-span deletion of the leading `&`
//! (measured `MachineApplicable`). The binding moves one reference level
//! out (`&u64` → `&&u64`), which is exactly what upstream's fix does —
//! usage sites that auto-deref (method calls) are unaffected, and the
//! `--fix` compile gate reverts any site where a use needed the exact
//! binding type. That is the shared fate of every deletion-shaped fix
//! in this corpus (`match_ref_pats` rewrites its scrutinee the same way).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - **LET-bindings only** — the match-arm shape (`&Some(ref x) =>`)
//!   CO-FIRES `match_ref_pats` (probe-measured, corpus.rs records it),
//!   and that landed rule owns the shape; emitting a second edit set on
//!   the same span would double-fix one diagnostic. Refused by
//!   construction (we walk `Stmt::Local` only).
//! - The pattern must be EXACTLY `&ref IDENT`: a `Pat::Reference` with
//!   no `mut` on the `&` (the `&mut` spellings measured SILENT — the
//!   guessed shape does not even compile), an inner `Pat::Ident`
//!   carrying `ref` but not `ref mut`, no subpattern, no or-patterns.
//! - `let-else` spellings refuse (`LocalInit.diverge`) — editing the
//!   pattern would birth an IRREFUTABLE `let ref y = .. else` (broken
//!   code; the else survives outside the edit span).
//! - Fixpoint: after the deletion the pattern carries no `&`-destructure
//!   (probe: the fixed form is SILENT and compiles clean).

use syn::visit::Visit;
use syn::{File, Pat, Stmt};

use super::SourceEdit;
use super::helpers::byte_range;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector { edits: Vec::new() };
    v.visit_file(file);
    v.edits
}

struct Collector {
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_stmt(&mut self, s: &'ast Stmt) {
        syn::visit::visit_stmt(self, s);
        if let Some(e) = Self::try_rewrite(s) {
            self.edits.push(e);
        }
    }
}

impl Collector {
    fn try_rewrite(s: &Stmt) -> Option<SourceEdit> {
        let Stmt::Local(local) = s else { return None };
        // The let-else spelling refuses: editing the pattern would birth
        // an IRREFUTABLE `let ref y = .. else` (broken code — the else
        // survives outside the edit span). syn 2 carries the diverging
        // arm on LocalInit.diverge.
        if local.init.as_ref().is_some_and(|i| i.diverge.is_some()) {
            return None;
        }
        let Pat::Reference(r) = &local.pat else {
            return None;
        };
        if r.mutability.is_some() {
            return None;
        }
        let Pat::Ident(pi) = &*r.pat else { return None };
        if pi.by_ref.is_none() || pi.mutability.is_some() || pi.subpat.is_some() {
            return None;
        }
        // Delete exactly the leading `&` token.
        let amp = byte_range(&r.and_token);
        Some(SourceEdit {
            start: amp.start,
            end: amp.end,
            replacement: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn seed_is_clean() {
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(a: u64) -> String { let ref y = &a; y.to_string() }",
            )
            .is_none()
        );
    }

    #[test]
    fn let_shape_deletes_the_amp() {
        let out = propose_fix(
            "needless_borrowed_reference",
            "pub fn f(a: u64) -> String {\n    let &ref y = &a;\n    y.to_string()\n}",
        )
        .expect("matched");
        assert!(out.contains("let ref y = &a;"), "{out}");
        assert!(!out.contains("let &ref"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn match_shape_refused_match_ref_pats_owns_it() {
        // The co-fire class: match_ref_pats (landed, Batch 169) owns the
        // match-arm shape — this rule emits nothing there.
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub enum E { A, B }\npub fn f(r: &E) -> u8 { match r { &E::A => 1, &E::B => 2 } }",
            )
            .is_none()
        );
    }

    #[test]
    fn nested_block_locals_fire() {
        let out = propose_fix(
            "needless_borrowed_reference",
            "pub fn f(a: u64) -> u64 {\n    {\n        let &ref y = &a;\n        *y\n    }\n}",
        )
        .expect("matched");
        assert!(out.contains("let ref y = &a;"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn ref_mut_inner_refuses() {
        // The mut inner binding is the unmeasured spelling — v1 stays
        // non-mut (narrower than clippy, documented).
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(mut a: u64) { let &mut ref mut y = &mut a; *y += 1; }",
            )
            .is_none()
        );
    }

    #[test]
    fn outer_mut_refuses() {
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(mut a: u64) { let &mut ref y = &mut a; let _ = y; }",
            )
            .is_none()
        );
    }

    #[test]
    fn plain_ref_binding_refuses() {
        // `ref y` WITHOUT the `&`-destructure is the owned-scrutinee
        // shape — upstream SILENT (probe nbr_ref_bind_owned).
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(a: u64) { let ref y = &a; let _ = y; }",
            )
            .is_none()
        );
    }

    #[test]
    fn non_ref_pattern_refuses() {
        // `&y` (move-out destructure) is a different shape entirely.
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(a: u64) -> u64 { let &y = &a; y }",
            )
            .is_none()
        );
    }

    #[test]
    fn subpattern_refuses() {
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(a: u64) { let &ref y @ _ = &a; let _ = y; }",
            )
            .is_none()
        );
    }

    #[test]
    fn let_else_refuses() {
        // Editing the pattern would birth an IRREFUTABLE let-else
        // (broken code — the else survives outside the edit span), so
        // the LocalInit.diverge guard refuses.
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(a: u64) -> u64 { let &ref y = &a else { return 0 }; *y }",
            )
            .is_none()
        );
    }

    #[test]
    fn tuple_pattern_refuses() {
        assert!(
            propose_fix(
                "needless_borrowed_reference",
                "pub fn f(a: u64, b: u64) -> u64 { let (&ref y, _) = (&a, &b); *y }",
            )
            .is_none()
        );
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let src = "pub fn f(a: u64) -> String {\n    let &ref y = &a;\n    y.to_string()\n}";
        let first = propose_fix("needless_borrowed_reference", src).expect("matched");
        assert!(propose_fix("needless_borrowed_reference", &first).is_none());
    }
}
