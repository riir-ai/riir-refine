//! `clippy::match_ref_pats` — `match f { &Fruit::Apple => 1, &Fruit::Pear
//! => 2 }` → `match *f { Fruit::Apple => 1, Fruit::Pear => 2 }` (Batch 169 /
//! Plan 160 — the coverage-report pack XIII, the Batch-168 staged alternate;
//! envelope verified under the plain oracle 2026-09-21,
//! `scripts/plan160_pack13_probe.py`).
//!
//! ## Why the rewrite is construction-identity (the FIXABLE bar)
//!
//! On any COMPILING input matching the guards, the scrutinee is a reference
//! (patterns prefixed `&` only typecheck against one) and every arm pattern
//! is BINDING-FREE — so `match *f { P => .. }` selects arms on the same
//! dereferenced place with zero moves (no binding can move out of it) and
//! the arm bodies are untouched. Byte-for-byte clippy's own
//! `MachineApplicable` suggestion shape ("instead of prefixing all patterns
//! with `&`, you can dereference the expression" — probe-measured:
//! `*f` + `Fruit::Apple` + `Fruit::Pear` replacements).
//!
//! ## Guards (the bounded fix-space — narrower than clippy, by design)
//!
//! - EVERY arm's top-level pattern must be a `&`-pattern — the probe's
//!   measured upstream silences, mirrored: a bare `_` arm, a mixed arm
//!   (`&A => .., B => ..`), and an or-pattern arm (`&A | &B => ..`) are all
//!   SILENT in real clippy and refuse here too. `&_` IS a `&`-pattern
//!   (probe: fires; the suggestion rewrites it to `_`).
//! - Arm guards ride verbatim (probe: `&A if k > 1 => ..` fires; only the
//!   pattern bytes move).
//! - The `&` sub-pattern must be BINDING-FREE (paths, wildcards, literals,
//!   paren/tuple/tuple-struct/struct/slice/or composites of those — a
//!   struct-pattern FIELD SHORTHAND `Point { x }` is a binding and refuses).
//!   clippy fires on the binding shape too (`&Some(ref x)` — probe), but
//!   the rewrite changes binding modes there — the documented
//!   narrower-than-clippy divergence (the staged scope, Plan 159).
//! - The sub-pattern must not be another `&` (a `&&A` arm would leave the
//!   output still all-`&`-armed — our own fixpoint would re-fire).
//! - The scrutinee must be a plain single-segment ident path — `*expr` is
//!   emitted verbatim before it; calls/fields/literals/parens refuse (the
//!   later-widening class, the `unnecessary_cast` precedent).
//! - Match/arm attributes are untouched (the edits move pattern bytes and
//!   insert one `*`; nothing is spliced away).
//!
//! Distinct from `redundant_pattern_matching` (Option/Option-Result
//! `if let Ok(_)`-to-method — a different lint and seam) and from
//! `needless_borrow` (call-site `&` removal — a different lint).

use syn::visit::Visit;
use syn::{Expr, ExprMatch, File, Pat};

use super::helpers::byte_range;
use super::SourceEdit;

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let mut scan = Scan { edits: Vec::new() };
    scan.visit_file(file);
    scan.edits
}

struct Scan {
    edits: Vec<SourceEdit>,
}

/// The sub-pattern carries no binding of any kind — the binding-mode
/// hazard guard. Field SHORTHAND (`Point { x }`) binds and refuses.
fn binding_free(p: &Pat) -> bool {
    match p {
        Pat::Wild(_) | Pat::Rest(_) | Pat::Lit(_) | Pat::Path(_) => true,
        Pat::Paren(i) => binding_free(&i.pat),
        Pat::Or(o) => o.cases.iter().all(binding_free),
        Pat::Tuple(t) => t.elems.iter().all(binding_free),
        Pat::TupleStruct(t) => t.elems.iter().all(binding_free),
        Pat::Slice(s) => s.elems.iter().all(binding_free),
        Pat::Struct(s) => {
            // a rest pattern (`..`) inside is fine — it binds nothing;
            // FIELD SHORTHAND (`Point { x }`) desugars to `x: x` — an
            // Ident pat — and `binding_free` refuses it
            s.fields.iter().all(|f| binding_free(&f.pat))
        }
        // every binding-bearing shape (Ident/Range/Reference/Type/Macro/
        // Const/Verbatim) plus syn's #[non_exhaustive] tail — the unknown
        // refuses (the conservative direction for a binding check)
        _ => false,
    }
}

/// A plain single-segment ident path scrutinee (`f`, `self`).
fn ident_scrutinee(e: &Expr) -> bool {
    let Expr::Path(p) = e else { return false };
    p.qself.is_none()
        && p.path.leading_colon.is_none()
        && p.path.segments.len() == 1
        && p.path.segments[0].arguments.is_none()
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        syn::visit::visit_expr_match(self, m);
        // every arm: top-level `&`-pattern, binding-free, non-nested-ref
        let mut ok = ident_scrutinee(&m.expr);
        if ok {
            for arm in &m.arms {
                let Pat::Reference(r) = &arm.pat else {
                    ok = false;
                    break;
                };
                if matches!(&*r.pat, Pat::Reference(_)) || !binding_free(&r.pat) {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            return;
        }
        // edit 1: `match *f` — insert `*` before the scrutinee
        let s = byte_range(&m.expr);
        self.edits.push(SourceEdit {
            start: s.start,
            end: s.start,
            replacement: "*".to_string(),
        });
        // edit 2..: strip each arm's leading `&` (plus any whitespace
        // between it and the sub-pattern)
        for arm in &m.arms {
            let Pat::Reference(r) = &arm.pat else {
                continue;
            };
            let pat_start = byte_range(&arm.pat).start;
            let inner_start = byte_range(&r.pat).start;
            if inner_start > pat_start {
                self.edits.push(SourceEdit {
                    start: pat_start,
                    end: inner_start,
                    replacement: String::new(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const SEED: &str = "pub enum Fruit { Apple, Pear }\npub fn f(x: &Fruit) -> u64 {\n    match x {\n        &Fruit::Apple => 1,\n        &Fruit::Pear => 2,\n    }\n}";

    #[test]
    fn seed_fires_and_rewrites() {
        let out = propose_fix("match_ref_pats", SEED).expect("matched");
        assert!(out.contains("match *x"), "scrutinee deref missing: {out}");
        assert!(
            !out.contains("&Fruit::Apple") && !out.contains("&Fruit::Pear"),
            "leading & survived: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn binding_arm_refuses() {
        // clippy fires here (probe-measured) — we refuse: the rewrite
        // changes binding modes (the documented divergence).
        let src = "pub fn f(r: &Option<u64>) -> u64 {\n    match r {\n        &Some(ref x) => *x,\n        &None => 0,\n    }\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn field_shorthand_is_a_binding() {
        let src = "pub struct P { x: u64 }\npub fn f(p: &P) -> u64 {\n    match p {\n        &P { x } => x,\n    }\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn binding_free_struct_pat_fires() {
        let src = "pub struct P { x: u64 }\npub fn f(p: &P) -> u64 {\n    match p {\n        &P { x: 1 } => 1,\n        &P { x: 2 } => 2,\n    }\n}";
        let out = propose_fix("match_ref_pats", src).expect("matched");
        assert!(out.contains("match *p"));
        assert!(out.contains("P { x: 1 }"));
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn wildcard_arm_refuses() {
        // upstream-silent (probe): a bare `_` arm
        let src = "pub enum Fruit { Apple, Pear }\npub fn f(x: &Fruit) -> u64 {\n    match x {\n        &Fruit::Apple => 1,\n        _ => 2,\n    }\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn ref_wildcard_arm_fires() {
        // `&_` IS a &-pattern (probe: fires; suggestion `&_` -> `_`)
        let src = "pub enum Fruit { Apple, Pear }\npub fn f(x: &Fruit) -> u64 {\n    match x {\n        &Fruit::Apple => 1,\n        &_ => 2,\n    }\n}";
        let out = propose_fix("match_ref_pats", src).expect("matched");
        assert!(out.contains("match *x"));
        assert!(out.contains("_ => 2"));
        assert!(!out.contains("&_"), "leading & on wildcard survived: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mixed_arm_refuses() {
        // upstream-silent (probe): one arm without the leading `&`
        let src = "pub enum Fruit { Apple, Pear }\npub fn f(x: &Fruit) -> u64 {\n    match x {\n        &Fruit::Apple => 1,\n        Fruit::Pear => 2,\n    }\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn or_pattern_arm_refuses() {
        // upstream-silent (probe): the arm's top level is an Or, not a &
        let src = "pub enum Fruit { Apple, Pear }\npub fn f(x: &Fruit) -> u64 {\n    match x {\n        &Fruit::Apple | &Fruit::Pear => 1,\n    }\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn nested_ref_arm_refuses() {
        // stripping one `&` would leave the output still all-`&`-armed —
        // our own fixpoint would re-fire (the ping guard)
        let src = "pub enum Fruit { Apple, Pear }\npub fn f(x: &&Fruit) -> u64 {\n    match x {\n        &&Fruit::Apple => 1,\n        &&Fruit::Pear => 2,\n    }\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn guard_rides_verbatim() {
        let src = "pub enum Fruit { Apple, Pear }\npub fn f(x: &Fruit, k: u64) -> u64 {\n    match x {\n        &Fruit::Apple if k > 1 => 1,\n        &Fruit::Pear => 2,\n    }\n}";
        let out = propose_fix("match_ref_pats", src).expect("matched");
        assert!(
            out.contains("Fruit::Apple if k > 1"),
            "guard damaged: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_ident_scrutinee_refuses() {
        let src = "pub enum Fruit { Apple, Pear }\npub fn f() -> u64 {\n    match make() {\n        &Fruit::Apple => 1,\n        &Fruit::Pear => 2,\n    }\n}\nfn make() -> &'static Fruit {\n    panic!()\n}";
        assert!(propose_fix("match_ref_pats", src).is_none());
    }

    #[test]
    fn fix_is_fixpoint_stable() {
        let first = propose_fix("match_ref_pats", SEED).expect("matched");
        assert!(propose_fix("match_ref_pats", &first).is_none());
    }
}
