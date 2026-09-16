//! Seed corpus — common clippy lint examples for the clippy-healing domain.
//!
//! Each entry is a `(lint_name, lint_group, buggy_code, fixed_code)` tuple
//! representing a real clippy lint pattern. The corpus is BLAKE3-committed
//! via [`Corpus::new`](`crate::corpus::Corpus::new`).
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
/// Returns a BLAKE3-committed [`Corpus<ClippyLintDomain>`] with 64 entries:
/// the 10 seed lints (style, complexity, perf, nursery groups) + the 31
/// Issue 016 structural-expansion and post-mining lints (frequency-ranked
/// from kornel's 100K-crate clippy run + Li et al.'s 8.09M-warning
/// crates.io study ∩ clippy's `MachineApplicable` axis ∩ parse-only
/// provability — see `.benchmarks/021`; the Batch 89 pair from the Plan
/// 049 T0.1 real-code fired histogram, Bench 061) + the 2 Batch 90
/// fired-histogram pedantic rows + the 4 Batch 91 clippy-1.98 new-lint
/// rows (P9 intake, all four suggestion-only in real clippy) + the 2
/// P10 post-mining intake rows (2026-09-13, the 089-T3 usage artifact —
/// both probe-verified on-by-default in clippy 1.98, the pair the
/// mechanical fixer needed by hand on `src/bin/heal.rs`) + the 1 P11
/// post-mining intake row (2026-09-13, the Bench-088 usage artifact —
/// `format_collect`, the first two-span clippy rule: fold rewrite + the
/// anonymous `use std::fmt::Write as _;` insertion) + the 1 P12
/// post-mining intake row (2026-09-14, the 2026-09-09 nursery-gap row —
/// `needless_pass_by_ref_mut`, the first receiver+cascade rule:
/// `&mut self` → `&self` + the same-file `let mut` caller strip)
///     + the 1 P15 post-mining intake row (2026-09-14, the born-lint
///       closure — `unnecessary_map_or`, the healer's own `map_or(false, f)`
///       fallback output re-flagged by real clippy; fires only on receivers
///       PROVEN Option/Result by the shared `receiver_kind` substrate)
///     + the 1 Issue 120 intake row (2026-09-16, the pedantic doc-lint
///       family — `doc_markdown`, the every-landing manual backtick pass
///       this crate's own `#![warn(clippy::pedantic)]` root generates).
///     + the 1 P31 post-mining intake row (2026-09-17, the riir-chain
///       `51b456f` MISS artifact — `manual_range_patterns`, clippy's own
///       algorithm ported verbatim onto syn: range alternatives,
///       duplicates, negatives and the coverage walk included; the
///       boundary literals splice verbatim so the rewrite is
///       set-identical + inference-identical).
///     + the 2 Batch 151 rows (2026-09-17, the doc-line state-machine
///       port — `doc_lazy_continuation` + `doc_overindented_list_items`,
///       the staged follow-up from the Batch 150 probe pack, Research 177
///       #7; one proposer module houses both, the probe151 boundary
///       matrix in Plan 137).
// 64-entry data table, not logic — length is the point.
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
            // Both shapes (P11 receiver extension, 2026-09-13): the free-fn
            // arg strip + the whitelisted method-receiver strip.
            buggy_code: "fn take_ref(x: &i32) -> i32 { *x }\nfn caller(x: &i32) -> i32 { take_ref(&x) }\nfn recv(v: Vec<u8>) -> usize { let n = (&v).as_slice().len(); n }".to_string(),
            fixed_code: "fn take_ref(x: &i32) -> i32 { *x }\nfn caller(x: &i32) -> i32 { take_ref(x) }\nfn recv(v: Vec<u8>) -> usize { let n = v.as_slice().len(); n }".to_string(),
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
        // P15 post-mining intake (2026-09-14, the born-lint closure — the
        // healer's own map_unwrap_or `map_or(false, f)` fallback output is
        // re-flagged by real clippy as this lint). Probe evidence:
        // `.raw/mapor_probe`, clippy 1.98 — all terminal suggestions
        // MachineApplicable.
        ClippyFixEntry {
            lint_name: "unnecessary_map_or".to_string(),
            lint_group: "pedantic".to_string(),
            buggy_code: "fn f(x: Option<u8>) -> bool { x.map_or(false, |v| v > 1) }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> bool { x.is_some_and(|v| v > 1) }".to_string(),
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
        ClippyFixEntry {
            lint_name: "manual_is_multiple_of".to_string(),
            lint_group: "style".to_string(),
            // Post-mining P7. VERIFY-tier: parse-only cannot see dividend
            // signedness and `is_multiple_of` is unsigned-only — the
            // `--verify` compile gate reverts any signed-dividend break.
            buggy_code: "fn f(n: u64) -> bool { n % 4 == 0 }".to_string(),
            fixed_code: "fn f(n: u64) -> bool { n.is_multiple_of(4) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_range_patterns".to_string(),
            lint_group: "style".to_string(),
            // Post-mining P31 intake (2026-09-17, the riir-chain 51b456f
            // MISS artifact — the file carried `0 | 1 | 2` the suggest lane
            // could not rank). FIXABLE: clippy's own algorithm ported
            // verbatim; the boundary literals splice verbatim so the
            // rewrite is set-identical + inference-identical.
            buggy_code: "fn f(n: u8) -> u8 { match n { 0 | 1 | 2 => 9, _ => 0 } }".to_string(),
            fixed_code: "fn f(n: u8) -> u8 { match n { 0..=2 => 9, _ => 0 } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "use_self".to_string(),
            lint_group: "style".to_string(),
            // Batch 89 (the Plan 049 fired histogram: 130 real instances).
            // FIXABLE: inherent-impl path rewrite with the soundness guards
            // from the module docs (nested items, bounds, attrs, macros,
            // generic-arg identity, value-namespace ambiguity).
            buggy_code: "struct V { n: u32 }\nimpl V { fn new() -> V { V { n: 1 } } }".to_string(),
            fixed_code: "struct V { n: u32 }\nimpl V { fn new() -> Self { Self { n: 1 } } }"
                .to_string(),
        },
        ClippyFixEntry {
            lint_name: "suboptimal_flops".to_string(),
            lint_group: "perf".to_string(),
            // Batch 89 (206 real instances). VERIFY-tier: the receiver's
            // type is invisible to syn. The powf(2.0)→powi(2) shape carries
            // the Batch-53 powi-vs-powf measured-TIE annotation — this is a
            // clippy-pedantic idiom fix, not a release-speed claim.
            buggy_code: "fn unit_len(x: f64, y: f64) -> f64 { (x * x + y * y).powf(0.5) }"
                .to_string(),
            fixed_code: "fn unit_len(x: f64, y: f64) -> f64 { (x * x + y * y).sqrt() }"
                .to_string(),
        },
        ClippyFixEntry {
            lint_name: "items_after_statements".to_string(),
            lint_group: "pedantic".to_string(),
            // Batch 90 (the fired-histogram pedantic tail: 39 real
            // instances). FIXABLE: block items cannot capture block locals
            // and name resolution is order-independent within a block, so
            // the hoist is semantics-preserving; the proposer's comment
            // guards (leading gap + same-line trailing) refuse where the
            // hoist would displace a comment.
            buggy_code: "fn f() -> i32 { let x = 1; fn h(v: i32) -> i32 { v * 2 } h(x) }".to_string(),
            fixed_code: "fn f() -> i32 { fn h(v: i32) -> i32 { v * 2 } let x = 1; h(x) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_let_else".to_string(),
            lint_group: "pedantic".to_string(),
            // Batch 90 (~40 real instances). FIXABLE: the probe mapped real
            // clippy's silent shapes 1:1 onto parse-only guards (binding-use
            // fallbacks, non-identity arm-1 bodies, 3-arm/guard shapes,
            // non-diverging fallbacks); divergence is judged visibly
            // (return/break/continue/panic-family macro).
            buggy_code: "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => return 0 }; v + 1 }"
                .to_string(),
            fixed_code: "fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }"
                .to_string(),
        },
        // ── Batch 91 (the 2026-09-05 P9 clippy-1.98 new-lint batch) ──
        // Populations measured workspace-wide by the 1.98.1 re-gate sweep:
        // manual_checked_ops [dao 17 + katgpt-rs 7], byte_char_slices
        // [ndb 6 + katgpt-rs 2], manual_slice_fill [ndb 2],
        // chunks_exact_to_as_chunks [katgpt-rs 12 + ndb 2]. All four lints
        // are suggestion-only in clippy (MaybeIncorrect / Unspecified —
        // `--fix` applied 0/17 on the dao run), which is why the proposers
        // exist.
        ClippyFixEntry {
            lint_name: "byte_char_slices".to_string(),
            lint_group: "style".to_string(),
            // FIXABLE: bit-identical by construction (same bytes, same
            // `[u8; N]` type). The ndb wire-magic batch landed this exact
            // form (`FREEZE_MAGIC: [u8; 4] = *b"FRZE"`) with the
            // consensus-committed device-vector pins still passing.
            buggy_code: "pub const SHARD_MAGIC: [u8; 4] = [b'S', b'H', b'R', b'D'];".to_string(),
            fixed_code: "pub const SHARD_MAGIC: [u8; 4] = *b\"SHRD\";".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_slice_fill".to_string(),
            lint_group: "style".to_string(),
            // FIXABLE behind the pure-value gate: `fill` evaluates its value
            // ONCE while the loop evaluates it PER ITERATION — impure values
            // (calls) change evaluation count and stay silent. A path value
            // is safe because the original loop only compiles when the value
            // is `Copy`, which subsumes `fill`'s `Clone` bound. Iterable
            // gate: only the `&mut place` form (real clippy is silent on
            // `iter_mut()` too).
            buggy_code: "struct Terrain { data: Vec<f32> }\nfn reset(t: &mut Terrain) {\n    for v in &mut t.data {\n        *v = 0.5;\n    }\n}"
                .to_string(),
            fixed_code: "struct Terrain { data: Vec<f32> }\nfn reset(t: &mut Terrain) {\n    t.data.fill(0.5);\n}"
                .to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_checked_ops".to_string(),
            lint_group: "complexity".to_string(),
            // VERIFY: real clippy fires ONLY on unsigned divisions
            // (probe-verified: i32/i64/isize silent — for signed types
            // checked_div also catches MIN / -1, a behavior change) and is
            // MaybeIncorrect (its --fix applied 0/17 on the dao run). Our
            // proposer refuses provably-signed divisors, refuses
            // method-call divisors (double evaluation — clippy refuses them
            // too), and requires the division's divisor to be TEXTUALLY the
            // condition's divisor.
            buggy_code: "fn share(total: u64, n: u64) -> u64 {\n    if n == 0 { 0 } else { total / n }\n}"
                .to_string(),
            fixed_code: "fn share(total: u64, n: u64) -> u64 {\n    total.checked_div(n).unwrap_or(0)\n}"
                .to_string(),
        },
        ClippyFixEntry {
            lint_name: "chunks_exact_to_as_chunks".to_string(),
            lint_group: "style".to_string(),
            // Both shapes (P13 chain-root extension, 2026-09-14): the
            // direct for-loop + the iterator-chain root
            // (`.map/.all/.enumerate/...` composes; `.0.iter()` inserted —
            // clippy's own chain suggestion; the `_mut` twin takes
            // `.iter_mut()`, SAFER than clippy's literal E0594 suggestion).
            // VERIFY: the chunk changes `&[u8]` → `&[u8; N]` (a body
            // consuming it as `&[u8]` in a typed position is E0599 the
            // compile gate reverts) and a non-const Path size compiles as
            // `chunks_exact(n)` but not as `as_chunks::<n>()`. MSRV floor:
            // `as_chunks` stabilized in std 1.88 — the workspace floats
            // 1.98.1 and real clippy is MSRV-aware (it never fires on an
            // older declared rust-version), so the suggest path is
            // inherently safe; the module doc carries the note.
            buggy_code: "fn ids(bytes: &[u8]) -> Vec<u32> {\n    let mut out = Vec::new();\n    for chunk in bytes.chunks_exact(4) {\n        out.push(u32::from_le_bytes(chunk.try_into().unwrap()));\n    }\n    out\n}\nfn firsts(bytes: &[u8]) -> Vec<u8> {\n    bytes.chunks_exact(4).map(|p| p[0]).collect()\n}".to_string(),
            fixed_code: "fn ids(bytes: &[u8]) -> Vec<u32> {\n    let mut out = Vec::new();\n    for chunk in bytes.as_chunks::<4>().0 {\n        out.push(u32::from_le_bytes(chunk));\n    }\n    out\n}\nfn firsts(bytes: &[u8]) -> Vec<u8> {\n    bytes.as_chunks::<4>().0.iter().map(|p| p[0]).collect()\n}".to_string(),
        },
        // ── P10 post-mining intake (2026-09-13, the 089-T3 usage artifact) ──
        // Both probe-verified on-by-default in clippy 1.98 (probe crate,
        // 2026-09-13). filter_map_bool_then: MachineApplicable in real
        // clippy but the emitted `|&x|` binds only Copy items (String
        // items silent — probed) → VERIFY tier (E0507 is check-visible).
        // vec_init_then_push: help-only in real clippy (--fix applied 0/3
        // even on the plain shape — probed); the proposer's parse-visible
        // guards (later-mutable-use refusal, attributed/cfg'd push refusal)
        // narrow the MaybeIncorrect class → VERIFY tier.
        ClippyFixEntry {
            lint_name: "filter_map_bool_then".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn kept(xs: Vec<i32>) -> Vec<i32> {\n    xs.into_iter().filter_map(|x| (x > 0).then(|| x * 2)).collect()\n}".to_string(),
            fixed_code: "fn kept(xs: Vec<i32>) -> Vec<i32> {\n    xs.into_iter().filter(|&x| (x > 0)).map(|x| x * 2).collect()\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "vec_init_then_push".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn build() -> Vec<u8> {\n    let mut v = Vec::new();\n    v.push(1);\n    v.push(2);\n    v\n}".to_string(),
            fixed_code: "fn build() -> Vec<u8> {\n    let v = vec![1, 2];\n    v\n}".to_string(),
        },
        // ── P11 post-mining intake (2026-09-13, the Bench-088 usage artifact) ──
        ClippyFixEntry {
            lint_name: "format_collect".to_string(),
            lint_group: "pedantic".to_string(),
            // The first TWO-span clippy rule: the fold rewrite + the
            // anonymous `use std::fmt::Write as _;` insertion (skipped when
            // the trait is already in scope). fixed_code is byte-shaped
            // like the proposer's output (single-line fold).
            buggy_code: "fn render(xs: &[u32]) -> String {\n    xs.iter().map(|x| format!(\"{}\", x)).collect::<String>()\n}".to_string(),
            fixed_code: "use std::fmt::Write as _;\nfn render(xs: &[u32]) -> String {\n    xs.iter().fold(String::new(), |mut out, x| { let _ = write!(out, \"{}\", x); out })\n}".to_string(),
        },
        // ── P12 post-mining intake (2026-09-14, the 2026-09-09 nursery-gap row) ──
        ClippyFixEntry {
            lint_name: "needless_pass_by_ref_mut".to_string(),
            lint_group: "nursery".to_string(),
            // NURSERY / default-`allow` — fires only under an explicit -W
            // stream (probe-verified 2026-09-14, clippy 0.1.98: private +
            // pub(crate) methods fire, `pub` is silent, trait impls silent,
            // suggestion applicability Unspecified). The fixture carries
            // BOTH halves: the receiver fix + the same-file `let mut`
            // caller cascade.
            buggy_code: "struct Probe { hits: u32 }\nimpl Probe {\n    fn read(&mut self) -> u32 {\n        self.hits\n    }\n    fn run() -> u32 {\n        let mut p = Probe { hits: 1 };\n        p.read()\n    }\n}".to_string(),
            fixed_code: "struct Probe { hits: u32 }\nimpl Probe {\n    fn read(&self) -> u32 {\n        self.hits\n    }\n    fn run() -> u32 {\n        let p = Probe { hits: 1 };\n        p.read()\n    }\n}".to_string(),
        },
        // ── Issue 120 intake (2026-09-16, the pedantic doc-lint family) ──
        ClippyFixEntry {
            lint_name: "doc_markdown".to_string(),
            lint_group: "pedantic".to_string(),
            // The every-landing manual backtick pass (this crate compiles
            // `#![warn(clippy::pedantic)]`, so every new module generates
            // the family). Probe-verified fire/silent against real clippy
            // 0.1.98 at landing (the /tmp scratch-crate record in
            // HISTORY): buggy fires on both words, fixed is silent. The
            // fixture vocabulary is deliberately DOC-flavored (no generic
            // Rust tokens) — centroid-defensive for the retrieval router.
            buggy_code: "/// Wraps the doc_span word of an IdentText run.\nfn wrap() {}".to_string(),
            fixed_code: "/// Wraps the `doc_span` word of an `IdentText` run.\nfn wrap() {}".to_string(),
        },
        // ── Batch 150 (the staged default-level + pedantic probe pack;
        // each fixture's buggy_code FIRES its own proposer — the
        // all_buggy_codes_fire_their_lint lockstep — and carries the
        // probe-verified fire/silence shapes per the P11 pattern) ──
        ClippyFixEntry {
            lint_name: "print_with_newline".to_string(),
            lint_group: "style".to_string(),
            // Bench 094 default-level evidence (top-10k crates.io sample,
            // clippy 0.1.98). Probe: print!/eprint! fire on a literal
            // ending in the \n escape; println!/eprintln! never fire;
            // raw strings and escaped-backslash endings are silent.
            buggy_code: "fn ready() { print!(\"ready\\n\"); }".to_string(),
            fixed_code: "fn ready() { println!(\"ready\"); }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "write_with_newline".to_string(),
            lint_group: "style".to_string(),
            // The fmt::Write sibling. 1-crate concentration caveat from
            // the histogram (the 248-firing nominal top-1 is one codegen
            // crate = 39% of its warnings) recorded in the bench doc.
            // The fixture carries the trait import — the real fire shape
            // needs it to compile (the freeze oracle's compiles filter;
            // write! on &mut String is E0599 without it).
            buggy_code: "use std::fmt::Write as _;\nfn line(f: &mut String) { write!(f, \"line\\n\"); }".to_string(),
            fixed_code: "use std::fmt::Write as _;\nfn line(f: &mut String) { writeln!(f, \"line\"); }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unreadable_literal".to_string(),
            lint_group: "pedantic".to_string(),
            // The only parse-only-PROVABLE pedantic row in the staged
            // head: digit grouping is value-preserving by construction.
            // Digit-count threshold ≥6 (10000 silent, 999999 fires).
            buggy_code: "fn pages() -> u32 { let pages = 1234567; pages }".to_string(),
            fixed_code: "fn pages() -> u32 { let pages = 1_234_567; pages }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "legacy_numeric_constants".to_string(),
            lint_group: "style".to_string(),
            // Shadow-guarded FIXABLE: the rewrite is meaning-changing
            // under an in-scope item named like a primitive and compiles,
            // so the proposer refuses such files wholesale (real clippy
            // marks its own suggestion MaybeIncorrect for this reason).
            buggy_code: "fn cap() -> u32 { let cap = std::u32::MAX; cap }".to_string(),
            fixed_code: "fn cap() -> u32 { let cap = u32::MAX; cap }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "partialeq_to_none".to_string(),
            lint_group: "style".to_string(),
            // VERIFY: type-blind rewrite, E0599 on a custom
            // PartialEq<Option> receiver — check-visible, the compile
            // gate reverts (the redundant_closure demotion precedent).
            buggy_code: "fn check(x: Option<u32>) -> bool { x == None }".to_string(),
            fixed_code: "fn check(x: Option<u32>) -> bool { x.is_none() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "cast_lossless".to_string(),
            lint_group: "pedantic".to_string(),
            // VERIFY via the safe-let gate: the operand type is only
            // followed from an explicit annotation, and the rewrite
            // emits only on std's primitive From table. The corpus
            // fixture is the PARAM shape (the lockstep fixture must fire
            // the proposer; the annotated-let shape is unit-tested in
            // the module).
            buggy_code: "fn widen(x: u8) -> u64 { x as u64 }".to_string(),
            fixed_code: "fn widen(x: u8) -> u64 { u64::from(x) }".to_string(),
        },
        // ── Batch 151 (2026-09-17, the doc-line state-machine port — the
        // staged follow-up from the Batch 150 probe pack, Research 177 #7;
        // probe151 boundary matrix in Plan 137). FIXABLE per the
        // doc_markdown precedent: comment-local edits can never change
        // compilation. Fixtures are the quote shape (MachineApplicable in
        // real clippy) and the overindent shape (the dedent to the content
        // col); doc-flavored vocab for centroid defense.
        ClippyFixEntry {
            lint_name: "doc_lazy_continuation".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "/// > The quote paragraph.\n/// lazy continuation line.\nfn quoted() {}".to_string(),
            fixed_code: "/// > The quote paragraph.\n/// > lazy continuation line.\nfn quoted() {}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "doc_overindented_list_items".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "/// - the item text.\n///     an overindented cont.\nfn listed() {}".to_string(),
            fixed_code: "/// - the item text.\n///   an overindented cont.\nfn listed() {}".to_string(),
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
        assert!(
            groups.contains(&"complexity"),
            "must cover complexity group"
        );
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
                entry.lint_name, result.diagnostics, entry.fixed_code
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

    /// Corpus size pin: 10 seed, 31 expansion/post-mining, 2 Batch 89,
    /// 2 Batch 90, 4 Batch 91, the 2 P10 post-mining intake rows, the
    /// 1 P11 post-mining intake row (`format_collect`, 2026-09-13), the
    /// 1 P12 post-mining intake row (`needless_pass_by_ref_mut`,
    /// 2026-09-14), the 1 P15 post-mining intake row
    /// (`unnecessary_map_or`, 2026-09-14 — the born-lint closure), the
    /// 1 Issue 120 intake row (`doc_markdown`, 2026-09-16), the 1
    /// P31 post-mining intake row (`manual_range_patterns`, 2026-09-17
    /// — the riir-chain `51b456f` MISS artifact), and the 6 Batch 150
    /// rows (`print_with_newline`, `write_with_newline`,
    /// `unreadable_literal`, `legacy_numeric_constants`,
    /// `partialeq_to_none`, `cast_lossless` — the staged default-level +
    /// pedantic probe pack, Bench 094 evidence), and the 2 Batch 151
    /// rows (`doc_lazy_continuation`, `doc_overindented_list_items` —
    /// the doc-line state-machine port, Research 177 #7) = 64.
    /// (`missing_const_for_fn` — the histogram's #1 — was proposed then
    /// KILLED: a broad const-ability proposer cross-fires every fixture
    /// and the clean-span set; see `VERIFY_LINTS` + `.plans/050`.)
    #[test]
    fn corpus_size_is_64() {
        let corpus = build_seed_corpus();
        assert_eq!(corpus.entries.len(), 64);
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
