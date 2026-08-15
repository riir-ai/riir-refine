//! `clippy::wildcard_in_or_patterns` — `1 | _ => 9` → `_ => 9`
//! (Issue 016).
//!
//! Pattern: an or-pattern with more than one case where any case is the
//! wildcard — `_` subsumes every other case, so the whole or-pattern is
//! replaced by `_`.

use super::{edit, SourceEdit};
use syn::visit::Visit;
use syn::{File, Pat, PatOr};

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
    fn visit_pat_or(&mut self, o: &'ast PatOr) {
        syn::visit::visit_pat_or(self, o);
        if o.cases.len() > 1 && o.cases.iter().any(|c| matches!(c, Pat::Wild(_))) {
            self.edits.push(edit(o, "_".to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn wildcard_case_subsumes_the_or() {
        let out = propose_fix(
            "wildcard_in_or_patterns",
            "fn f(n: u8) -> u8 { match n { 1 | _ => 9, } }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(n: u8) -> u8 { match n { _ => 9, } }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn wildcard_in_later_position_subsumes() {
        let out = propose_fix(
            "wildcard_in_or_patterns",
            "fn f(n: u8) -> u8 { match n { 1 | 2 | _ => 9, } }",
        )
        .expect("pattern matched");
        assert_eq!(out, "fn f(n: u8) -> u8 { match n { _ => 9, } }");
        assert!(syn::parse_file(&out).is_ok(), "{out}");
    }

    #[test]
    fn concrete_or_unchanged() {
        let out = propose_fix(
            "wildcard_in_or_patterns",
            "fn f(n: u8) -> u8 { match n { 1 | 2 => 9, _ => 0, } }",
        );
        assert!(out.is_none(), "no wildcard inside the or-pattern: {out:?}");
    }

    #[test]
    fn lone_wildcard_arm_unchanged() {
        let out = propose_fix("wildcard_in_or_patterns", "fn f(n: u8) -> u8 { match n { _ => 9, } }");
        assert!(out.is_none(), "single-case `or` is not an or-pattern: {out:?}");
    }
}
