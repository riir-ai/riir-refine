//! `clippy::io_other_error` — `io::Error::new(io::ErrorKind::Other, e)` →
//! `io::Error::other(e)` (Batch 156 / Plan 147 — the frontier-lead mining
//! pack II; family fixture verified to fire exactly this lint under
//! all+pedantic+nursery, 2026-09-18).
//!
//! Pattern: a call whose callee path ends `…Error::new` with exactly two
//! args, the first a path ending `…ErrorKind::Other`. The callee is rewritten
//! to `…Error::other` (prefix verbatim) and the kind argument is deleted.
//!
//! **FIXABLE tier** (inference-identical by construction): std's
//! `Error::new<E: Into<Box<dyn Error + Send + Sync>>>(kind, e)` and
//! `Error::other<E: Into<Box<dyn Error + Send + Sync>>>(e)` carry the
//! IDENTICAL bound on the payload — every input that compiled through `new`
//! with the literal `ErrorKind::Other` compiles through `other` with the same
//! inference. MSRV floor: `Error::other` stabilized in std 1.74 (the
//! workspace floats 1.98.1, the `chunks_exact_to_as_chunks` precedent for
//! documenting the floor rather than gating on it).
//!
//! Guards (parse-only provable):
//! - the callee path's last two segments are exactly `Error::new` with NO
//!   generic args on any segment (a turbofish `new::<T>` is a different
//!   call-site contract — refused); no qself, no leading `::`;
//! - the kind arg is a PATH (paren layers unwrapped) whose last two segments
//!   are exactly `ErrorKind::Other` — bare `Other` (from a glob
//!   `use ErrorKind::*`) is refused (unprovable provenance, the documented
//!   divergence); consts/let bindings holding the kind refuse with it;
//! - no comment may live in the deleted kind range (the
//!   `drops_interior_comment` contract);
//! - **shadow guard** (the `legacy_numeric_constants` precedent): the whole
//!   file refuses when any item (mod/struct/enum/union/trait/type/fn/const/
//!   static) is named `Error`, `ErrorKind`, or `io`, or when a NON-std use
//!   tree mentions any of those names — path resolution prefers in-scope
//!   items, and the rewrite compiles with a different meaning under
//!   shadowing. Plain `use std::io::…` re-openings (incl. grouped trees and
//!   `use std::io;` itself) are exempt — same module, semantics preserved.

use syn::visit::Visit;
use syn::{Expr, File};

use super::helpers::{byte_range, drops_interior_comment, file_shadows};
use super::SourceEdit;

/// Names whose file-level shadowing makes the rewrite meaning-changing.
const SHADOW_NAMES: &[&str] = &["Error", "ErrorKind", "io"];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    if file_shadows(file, SHADOW_NAMES) {
        return Vec::new(); // conservatively empty — the guard refused the file
    }
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        let Expr::Path(func) = &*call.func else {
            return;
        };
        let segs = &func.path.segments;
        // `…Error::new`, plain paths only.
        if func.qself.is_some()
            || func.path.leading_colon.is_some()
            || segs.len() < 2
            || segs[segs.len() - 1].ident != "new"
            || segs[segs.len() - 2].ident != "Error"
            || segs.iter().any(|s| !s.arguments.is_none())
        {
            return;
        }
        if call.args.len() != 2 {
            return;
        }
        let kind = unwrap_parens(&call.args[0]);
        if !is_error_kind_other(kind) {
            return;
        }
        // Prefix text: everything before the `new` segment, minus the
        // trailing `::` separators — spliced verbatim so `std::io::Error`,
        // `io::Error`, and `Error` imports all survive with their own shape.
        let path_range = byte_range(func);
        let new_start = byte_range(segs.last().expect("len checked")).start;
        let Some(raw_prefix) = self.source.get(path_range.start..new_start) else {
            return;
        };
        let prefix = raw_prefix.trim_end_matches(':');
        if prefix.is_empty() {
            return; // defensive: `::new` with no receiver is not this rule
        }
        // The deleted range [kind.start .. arg1.start] must not carry a
        // comment (it is removed with the argument).
        let a0 = byte_range(&call.args[0]);
        let a1 = byte_range(&call.args[1]);
        if drops_interior_comment(self.source, a0.start..a1.start, &[]) {
            return;
        }
        self.edits.push(SourceEdit {
            start: path_range.start,
            end: path_range.end,
            replacement: format!("{prefix}::other"),
        });
        self.edits.push(SourceEdit {
            start: a0.start,
            end: a1.start,
            replacement: String::new(),
        });
    }
}

/// Peel paren layers for the kind check (the arg is deleted, not moved, so
/// only the DETECTION needs the unwrap).
fn unwrap_parens(mut e: &Expr) -> &Expr {
    while let Expr::Paren(p) = e {
        e = &p.expr;
    }
    e
}

/// Last two path segments exactly `ErrorKind::Other`, plain path.
fn is_error_kind_other(e: &Expr) -> bool {
    let Expr::Path(p) = e else {
        return false;
    };
    let segs = &p.path.segments;
    p.qself.is_none()
        && p.path.leading_colon.is_none()
        && segs.len() >= 2
        && segs[segs.len() - 1].ident == "Other"
        && segs[segs.len() - 2].ident == "ErrorKind"
        && segs.iter().all(|s| s.arguments.is_none())
}
// (The shadow guard lives at `helpers::file_shadows` — factored at the
// Batch 157 second consumer, same semantics this module shipped in
// Batch 156.)

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn rewrites_fully_qualified_form() {
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e) }",
        )
        .expect("matched");
        assert!(
            out.contains("std::io::Error::other(e)"),
            "not rewritten: {out}"
        );
        assert!(!out.contains("new"), "kind arg/callee must be gone: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_imported_short_form() {
        let out = propose_fix(
            "io_other_error",
            "use std::io::{Error, ErrorKind};\nfn mk(e: String) -> Error { Error::new(ErrorKind::Other, e) }",
        )
        .expect("matched");
        assert!(out.contains("Error::other(e)"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn rewrites_io_prefixed_form() {
        let out = propose_fix(
            "io_other_error",
            "use std::io;\nfn mk(e: String) -> io::Error { io::Error::new(io::ErrorKind::Other, e) }",
        )
        .expect("matched");
        assert!(out.contains("io::Error::other(e)"), "not rewritten: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn second_arg_preserved_verbatim() {
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(\n    std::io::ErrorKind::Other,\n    format!(\"boom: {e}\"),\n) }",
        )
        .expect("matched");
        assert!(
            out.contains("format!(\"boom: {e}\")"),
            "payload text must ride verbatim: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn other_error_kinds_refused() {
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::NotFound, e) }",
        );
        assert!(out.is_none(), "non-Other kind must refuse: {out:?}");
    }

    #[test]
    fn bare_other_glob_form_refused() {
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(Other, e) }",
        );
        assert!(
            out.is_none(),
            "bare Other (glob provenance) must refuse: {out:?}"
        );
    }

    #[test]
    fn wrong_arity_refused() {
        let one = propose_fix(
            "io_other_error",
            "fn mk() -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other) }",
        );
        let three = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e, 1) }",
        );
        assert!(
            one.is_none() && three.is_none(),
            "arity must refuse: {one:?} {three:?}"
        );
    }

    #[test]
    fn turbofish_refused() {
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new::<String>(std::io::ErrorKind::Other, e) }",
        );
        assert!(out.is_none(), "turbofish must refuse: {out:?}");
    }

    #[test]
    fn comment_in_kind_range_refused() {
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(\n    std::io::ErrorKind::Other,\n    // the payload\n    e) }",
        );
        assert!(
            out.is_none(),
            "comment in the deleted range must refuse: {out:?}"
        );
    }

    #[test]
    fn comment_before_kind_survives() {
        // A comment BEFORE the first arg is outside the deleted range — the
        // rewrite is legal and the comment rides verbatim.
        let out = propose_fix(
            "io_other_error",
            "fn mk(e: String) -> std::io::Error { std::io::Error::new(\n    // the kind we always use\n    std::io::ErrorKind::Other, e) }",
        )
        .expect("matched");
        assert!(
            out.contains("// the kind we always use"),
            "comment must survive: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn local_mod_io_refuses_whole_file() {
        let out = propose_fix(
            "io_other_error",
            "mod io { pub fn x() {} }\nfn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e) }",
        );
        assert!(out.is_none(), "mod io must trip the shadow guard: {out:?}");
    }

    #[test]
    fn aliased_import_refuses_whole_file() {
        let out = propose_fix(
            "io_other_error",
            "use mycrate::Error;\nfn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e) }",
        );
        assert!(
            out.is_none(),
            "non-std Error import must trip the guard: {out:?}"
        );
    }
}
