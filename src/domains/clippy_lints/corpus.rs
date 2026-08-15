//! Seed corpus — common clippy lint examples for the clippy-healing domain.
//!
//! Each entry is a `(lint_name, lint_group, buggy_code, fixed_code)` tuple
//! representing a real clippy lint pattern. The corpus is BLAKE3-committed
//! via [`Corpus::new`](crate::corpus::Corpus::new).
//!
//! ## Caveat
//!
//! These examples are **illustrative** — the L2 syntax pruner validates that
//! the fixed code parses as valid Rust, but does NOT verify the lint is
//! actually resolved in the fast path. For truthful lint resolution, enable
//! the `clippy_verify` feature and use `validate_with_clippy` (runs real
//! `cargo clippy`). The `buggy_code` and `fixed_code` represent the *pattern*
//! the tool learns from, not verified trigger/fix pairs in the fast path.

use crate::corpus::Corpus;
use crate::domains::clippy_lints::{ClippyFixEntry, ClippyLintDomain};

/// Build a seed corpus from common clippy lint examples.
///
/// Returns a BLAKE3-committed [`Corpus<ClippyLintDomain>`] with 40 entries:
/// the 10 seed lints (style, complexity, perf, nursery groups) + the 30
/// Issue 016 structural-expansion lints (frequency-ranked from kornel's
/// 100K-crate clippy run + Li et al.'s 8.09M-warning crates.io study ∩
/// clippy's `MachineApplicable` axis ∩ parse-only provability — see
/// `.benchmarks/021`).
// 40-entry data table, not logic — length is the point.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn build_seed_corpus() -> Corpus<ClippyLintDomain> {
    Corpus::new(vec![
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
        // ── Issue 016: structural expansion (30 lints) ────────────────────
        ClippyFixEntry {
            lint_name: "semicolon_if_nothing_returned".to_string(),
            lint_group: "pedantic".to_string(),
            // 41,601× in kornel's 100K-crate clippy run (v1.80–v1.83) — the
            // most frequent structural lint in that dataset's top-10.
            buggy_code: "fn greet() {}\nfn main() {\n    greet()\n}".to_string(),
            fixed_code: "fn greet() {}\nfn main() {\n    greet();\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "collapsible_if".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn work() {}\nfn f(a: bool, b: bool) { if a { if b { work(); } } }".to_string(),
            fixed_code: "fn work() {}\nfn f(a: bool, b: bool) { if a && b { work(); } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "collapsible_else_if".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn work() {}\nfn f(a: bool, b: bool) { if a { work(); } else { if b { work(); } } }".to_string(),
            fixed_code: "fn work() {}\nfn f(a: bool, b: bool) { if a { work(); } else if b { work(); } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "single_match".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn use_v(v: u8) {}\nfn f(x: Option<u8>) { match x { Some(v) => use_v(v), _ => () } }".to_string(),
            fixed_code: "fn use_v(v: u8) {}\nfn f(x: Option<u8>) { if let Some(v) = x { use_v(v) } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "single_match_else".to_string(),
            lint_group: "pedantic".to_string(),
            // Shape discipline (Issue 022): real clippy 1.97 only fires this
            // when the else arm is a STATEMENT BLOCK — a bare-default else
            // routes to manual_unwrap_or[_default] instead. Verified against
            // real clippy (the G6 demo path).
            buggy_code: "fn score(g: u8) -> u8 { g * 2 }\nfn f(kind: Option<u8>) -> u8 { match kind { Some(k) => score(k), _ => { let fallback = 2; score(fallback) } } }".to_string(),
            fixed_code: "fn score(g: u8) -> u8 { g * 2 }\nfn f(kind: Option<u8>) -> u8 { if let Some(k) = kind { score(k) } else { let fallback = 2; score(fallback) } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "redundant_pattern_matching".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(r: Result<u8, u8>) -> bool { if let Ok(_) = r { true } else { false } }".to_string(),
            fixed_code: "fn f(r: Result<u8, u8>) -> bool { if r.is_ok() { true } else { false } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "question_mark".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(x: Option<u8>) -> Option<u8> { if x.is_none() { return None; } Some(x.unwrap() + 1) }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> Option<u8> { x?; Some(x.unwrap() + 1) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "range_plus_one".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(n: usize) -> usize { let mut s = 0; for i in 0..n+1 { s += i; } s }".to_string(),
            fixed_code: "fn f(n: usize) -> usize { let mut s = 0; for i in 0..=n { s += i; } s }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "uninlined_format_args".to_string(),
            lint_group: "pedantic".to_string(),
            buggy_code: "fn f(name: &str) -> String { format!(\"{}\", name) }".to_string(),
            fixed_code: "fn f(name: &str) -> String { format!(\"{name}\") }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "identity_op".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(x: u32) -> u32 { x * 1 }".to_string(),
            fixed_code: "fn f(x: u32) -> u32 { x }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_bool".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(b: bool) -> bool { if b { true } else { false } }".to_string(),
            fixed_code: "fn f(b: bool) -> bool { b }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_bool_assign".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(c: bool) { let mut x = false; if c { x = true; } else { x = false; } }".to_string(),
            fixed_code: "fn f(c: bool) { let mut x = false; x = c; }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "match_bool".to_string(),
            lint_group: "pedantic".to_string(),
            buggy_code: "fn f(b: bool) -> u8 { match b { true => 1, false => 0 } }".to_string(),
            fixed_code: "fn f(b: bool) -> u8 { if b { 1 } else { 0 } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_match".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(x: Option<u8>) -> Option<u8> { match x { Some(v) => Some(v), None => None } }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> Option<u8> { x }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "redundant_static_lifetimes".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "static GREETING: &'static str = \"hi\";".to_string(),
            fixed_code: "static GREETING: &str = \"hi\";".to_string(),
        },
        ClippyFixEntry {
            lint_name: "double_parens".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(x: i32) -> i32 { ((x + 1)) }".to_string(),
            fixed_code: "fn f(x: i32) -> i32 { (x + 1) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unnecessary_lazy_evaluations".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(x: Option<u8>) -> u8 { x.unwrap_or_else(|| 5) }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> u8 { x.unwrap_or(5) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "map_unwrap_or".to_string(),
            lint_group: "pedantic".to_string(),
            buggy_code: "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "filter_next".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(xs: &[u8]) -> Option<&u8> { xs.iter().filter(|c| **c > 3).next() }".to_string(),
            fixed_code: "fn f(xs: &[u8]) -> Option<&u8> { xs.iter().find(|c| **c > 3) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "iter_nth_zero".to_string(),
            lint_group: "style".to_string(),
            // Shape discipline (Issue 022): `.iter().nth(0)` on a SLICE
            // receiver routes to clippy 1.97's more-specific `iter_nth` —
            // `iter_nth_zero` fires on non-slice iterators (IntoIter, ranges).
            // Verified against real clippy.
            buggy_code: "fn f(mut lane: std::vec::IntoIter<u8>) -> Option<u8> { lane.nth(0) }".to_string(),
            fixed_code: "fn f(mut lane: std::vec::IntoIter<u8>) -> Option<u8> { lane.next() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_unwrap_or".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(x: Option<u8>) -> u8 { match x { Some(v) => v, None => 7 } }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> u8 { x.unwrap_or(7) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unused_unit".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "struct S;\nimpl S {\n    fn m(&self) -> () {}\n}".to_string(),
            fixed_code: "struct S;\nimpl S {\n    fn m(&self) {}\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "useless_format".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f() -> String { format!(\"done\") }".to_string(),
            fixed_code: "fn f() -> String { \"done\".to_string() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "deref_addrof".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(x: u8) -> u8 { let y = *&x; y }".to_string(),
            fixed_code: "fn f(x: u8) -> u8 { let y = x; y }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "wildcard_in_or_patterns".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(n: u8) -> u8 { match n { 1 | _ => 9, } }".to_string(),
            fixed_code: "fn f(n: u8) -> u8 { match n { _ => 9, } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unnecessary_sort_by".to_string(),
            lint_group: "perf".to_string(),
            buggy_code: "fn f(mut v: Vec<u8>) { v.sort_by(|a, b| a.cmp(b)); }".to_string(),
            fixed_code: "fn f(mut v: Vec<u8>) { v.sort(); }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "iter_next_loop".to_string(),
            lint_group: "correctness".to_string(),
            // `mut` discipline (Issue 022): `.next()` needs `&mut self` — the
            // pre-fix entry didn't compile, so real clippy could never lint
            // it (parse-only proposers never noticed).
            buggy_code: "fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it.next() { s += x; } s }".to_string(),
            fixed_code: "fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "comparison_to_empty".to_string(),
            lint_group: "style".to_string(),
            // Shape discipline (Issue 022): the REAL comparison_to_empty is
            // `s == ""` → `s.is_empty()` (verified against real clippy). The
            // `.len() == 0` form is real clippy's `len_zero` — the proposer
            // still detects + heals BOTH shapes (same fix either way).
            buggy_code: "fn f(s: &str) -> bool { s == \"\" }".to_string(),
            fixed_code: "fn f(s: &str) -> bool { s.is_empty() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "while_let_on_iterator".to_string(),
            lint_group: "style".to_string(),
            // `mut` discipline (Issue 022): same as iter_next_loop — the
            // pre-fix entry didn't compile under real clippy.
            buggy_code: "fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; while let Some(x) = it.next() { s += x; } s }".to_string(),
            fixed_code: "fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "single_char_pattern".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(s: &str) -> bool { s.contains(\"x\") }".to_string(),
            fixed_code: "fn f(s: &str) -> bool { s.contains('x') }".to_string(),
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
        let thawed = Corpus::<ClippyLintDomain>::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert_eq!(thawed.entries.len(), corpus.entries.len());
        assert!(thawed.verify());
    }

    /// Corpus size pin: 10 seed + 30 Issue 016 expansion.
    #[test]
    fn corpus_size_is_40() {
        let corpus = build_seed_corpus();
        assert_eq!(corpus.entries.len(), 40);
    }

    /// Every corpus entry's proposer round-trips: the buggy fixture fires
    /// the lint AND the output re-parses (the L2 pruner contract).
    #[test]
    fn all_buggy_codes_fire_their_lint() {
        let corpus = build_seed_corpus();
        for entry in &corpus.entries {
            let out = crate::domains::clippy_lints::proposers::propose_fix(
                &entry.lint_name,
                &entry.buggy_code,
            )
            .unwrap_or_else(|| {
                panic!(
                    "proposer did not fire on corpus fixture for `{}`",
                    entry.lint_name
                )
            });
            assert!(
                syn::parse_file(&out).is_ok(),
                "output for `{}` does not re-parse:\n{}",
                entry.lint_name,
                out
            );
        }
    }

    /// Issue 016 acceptance: `--suggest` must retrieve each rule (the hybrid
    /// latent+BM25 retriever). Builds the per-domain [`RuleIndex`] from the
    /// corpus itself and queries each entry's own `buggy_code` — the entry
    /// must appear in the top-5 retrieved rules. Gated on `latent_retrieval`
    /// (the `RuleIndex` lives behind it).
    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn every_lint_retrievable_via_suggest() {
        let corpus = build_seed_corpus();
        let patterns: Vec<(String, String)> = corpus
            .entries
            .iter()
            .map(|e| (e.lint_name.clone(), e.buggy_code.clone()))
            .collect();
        let index = crate::draft::corpus_index::RuleIndex::build(patterns).expect("index builds");
        for entry in &corpus.entries {
            let hits = index.suggest(&entry.buggy_code, 5);
            assert!(
                hits.iter().any(|h| h.rule_id == entry.lint_name),
                "`{}` not retrieved in top-5 by its own buggy_code: {:?}",
                entry.lint_name,
                hits.iter().map(|h| h.rule_id.clone()).collect::<Vec<_>>()
            );
        }
    }
}
