//! Seed corpus — common clippy lint examples for the clippy-healing pipeline.
//!
//! Each entry is a `(lint_name, lint_group, buggy_code, fixed_code)` tuple
//! representing a real clippy lint pattern. The corpus is BLAKE3-committed
//! via [`ClippyCorpus::new`].
//!
//! ## Caveat
//!
//! These examples are **illustrative** — the L2 syntax pruner validates that
//! the fixed code parses as valid Rust, but does NOT verify the lint is
//! actually resolved in the fast path. For truthful lint resolution, enable
//! the `clippy_verify` feature and use `validate_with_clippy` (runs real
//! `cargo clippy`). The `buggy_code` and `fixed_code` represent the *pattern*
//! the tool learns from, not verified trigger/fix pairs in the fast path.

use crate::corpus::{ClippyCorpus, ClippyFixEntry};

/// Build a seed corpus from common clippy lint examples.
///
/// Returns a BLAKE3-committed [`ClippyCorpus`] with ~10 entries covering
/// the most frequent clippy lints (style, complexity, perf groups).
#[must_use]
pub fn build_seed_corpus() -> ClippyCorpus {
    ClippyCorpus::new(vec![
        // ── style group ──────────────────────────────────────────────────
        ClippyFixEntry {
            lint_name: "needless_return".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn foo() -> i32 { return 42; }".to_string(),
            fixed_code: "fn foo() -> i32 { 42 }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "redundant_closure".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn double(x: i32) -> i32 { x * 2 }\nfn apply(f: impl Fn(i32) -> i32, x: i32) -> i32 { f(x) }\nfn caller(x: i32) -> i32 { apply(|v| double(v), x) }".to_string(),
            fixed_code: "fn double(x: i32) -> i32 { x * 2 }\nfn apply(f: impl Fn(i32) -> i32, x: i32) -> i32 { f(x) }\nfn caller(x: i32) -> i32 { apply(double, x) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_lifetimes".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn first<'a>(s: &'a String) -> &'a str { s.as_str() }".to_string(),
            fixed_code: "fn first(s: &String) -> &str { s.as_str() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "redundant_field_names".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "struct Point { x: i32, y: i32 }\nfn make(x: i32, y: i32) -> Point {\n    Point { x: x, y: y }\n}".to_string(),
            fixed_code: "struct Point { x: i32, y: i32 }\nfn make(x: i32, y: i32) -> Point {\n    Point { x, y }\n}".to_string(),
        },
        // ── complexity group ─────────────────────────────────────────────
        ClippyFixEntry {
            lint_name: "manual_map".to_string(),
            lint_group: "complexity".to_string(),
            // NOTE: uses `match ... { Some => Some, None => None }` which is the
            // canonical manual_map pattern. (The old `if x.is_some()` version
            // triggered `unnecessary_unwrap` instead — caught by G6 in Issue 008.)
            buggy_code: "fn opt_double(x: Option<i32>) -> Option<i32> {\n    match x {\n        Some(v) => Some(v * 2),\n        None => None,\n    }\n}".to_string(),
            fixed_code: "fn opt_double(x: Option<i32>) -> Option<i32> {\n    x.map(|v| v * 2)\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unnecessary_unwrap".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn process(opt: Option<i32>) -> i32 {\n    if opt.is_some() { opt.unwrap() } else { 0 }\n}".to_string(),
            fixed_code: "fn process(opt: Option<i32>) -> i32 {\n    opt.unwrap_or(0)\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_range_loop".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn sum_vec(v: &Vec<i32>) -> i32 {\n    let mut total = 0;\n    for i in 0..v.len() { total += v[i]; }\n    total\n}".to_string(),
            fixed_code: "fn sum_vec(v: &Vec<i32>) -> i32 {\n    v.iter().sum()\n}".to_string(),
        },
        // ── perf group ───────────────────────────────────────────────────
        ClippyFixEntry {
            lint_name: "useless_conversion".to_string(),
            lint_group: "perf".to_string(),
            buggy_code: "fn add_one(x: i32) -> i32 {\n    let y: i32 = x.into();\n    y + 1\n}".to_string(),
            fixed_code: "fn add_one(x: i32) -> i32 {\n    x + 1\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unnecessary_mut_passed".to_string(),
            lint_group: "perf".to_string(),
            buggy_code: "fn length(s: &str) -> usize { s.len() }\nfn caller(mut s: String) -> usize { length(&mut s) }".to_string(),
            fixed_code: "fn length(s: &str) -> usize { s.len() }\nfn caller(s: String) -> usize { length(&s) }".to_string(),
        },
        // ── nursery / pedantic ───────────────────────────────────────────
        ClippyFixEntry {
            lint_name: "needless_borrow".to_string(),
            lint_group: "nursery".to_string(),
            buggy_code: "fn take_ref(x: &i32) -> i32 { *x }\nfn caller(x: &i32) -> i32 { take_ref(&x) }".to_string(),
            fixed_code: "fn take_ref(x: &i32) -> i32 { *x }\nfn caller(x: &i32) -> i32 { take_ref(x) }".to_string(),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pruner::RustAnalyzerPruner;

    #[test]
    fn seed_corpus_is_nonempty() {
        let corpus = build_seed_corpus();
        assert!(!corpus.entries.is_empty(), "seed corpus must have entries");
        assert!(corpus.verify(), "seed corpus commitment must verify");
    }

    #[test]
    fn seed_corpus_has_unique_lint_names() {
        let corpus = build_seed_corpus();
        let names = corpus.lint_names();
        // Each lint should appear exactly once in the unique names list
        assert_eq!(
            names.len(),
            corpus.entries.len(),
            "each entry should have a unique lint_name"
        );
    }

    #[test]
    fn seed_corpus_covers_common_groups() {
        let corpus = build_seed_corpus();
        let groups: Vec<&str> = corpus
            .entries
            .iter()
            .map(|e| e.lint_group.as_str())
            .collect();
        // Should cover at least style, complexity, perf
        assert!(groups.contains(&"style"), "must cover style group");
        assert!(groups.contains(&"complexity"), "must cover complexity group");
        assert!(groups.contains(&"perf"), "must cover perf group");
    }

    #[test]
    fn all_fixed_code_parses() {
        // G1 for the seed corpus: every fixed_code entry must parse as valid Rust.
        // This is the load-bearing check — the L2 syntax pruner must accept every fix.
        let corpus = build_seed_corpus();
        let pruner = RustAnalyzerPruner::new();

        for entry in &corpus.entries {
            let result = pruner.validate(&entry.fixed_code, &entry.lint_name);
            assert!(
                result.parses,
                "fixed_code for lint '{}' must parse as valid Rust. Diagnostics: {:?}. Code: {}",
                entry.lint_name,
                result.diagnostics,
                entry.fixed_code
            );
        }
    }

    #[test]
    fn seed_corpus_freeze_thaw_roundtrip() {
        let corpus = build_seed_corpus();
        let frozen = corpus.freeze().unwrap();
        let thawed = ClippyCorpus::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert_eq!(thawed.entries.len(), corpus.entries.len());
        assert!(thawed.verify());
    }
}
