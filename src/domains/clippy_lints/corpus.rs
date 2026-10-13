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
/// Returns a BLAKE3-committed [`Corpus<ClippyLintDomain>`] — the seed
/// corpus. The entry count is DERIVED (it moves with every mining
/// batch): run `cargo refine --corpus-stats`, never read a typed figure
/// here — the hand-typed one rotted ("72 entries" against an
/// enumeration that had already reached Batch 164). The enumeration
/// below is the per-batch landing index: the 10 seed lints (style,
/// complexity, perf, nursery groups) + the 31
/// Issue 016 structural-expansion and post-mining lints (frequency-ranked
/// from kornel's 100K-crate clippy run + Li et al.'s 8.09M-warning
/// crates.io study ∩ clippy's `MachineApplicable` axis ∩ parse-only
/// provability — see `.benchmarks/021`; the Batch 89 pair from the Plan
/// 049 T0.1 real-code fired histogram, Bench 061) + the 2 Batch 90
/// fired-histogram pedantic rows + the 4 Batch 91 clippy-1.98 new-lint
/// rows (P9 intake, all four suggestion-only in real clippy) + the 5
/// Batch 230 clippy-1.99 new-lint rows (2026-10-13, the TOOLCHAIN-MOVE
/// RE-ARM — upstream stable 1.98.1→1.99.0 shipped 2026-09-28; the
/// Batch-91 precedent class; probe-verified against the isolated +1.99.0
/// toolchain, all fixed forms compile-verified at BOTH 1.99.0 and 1.98.1;
/// AHEAD-OF-PIN lints — see `lint_level.rs` `AHEAD_OF_PIN_1_99`) + the 2
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
///     + the 4 Batch 155 rows (2026-09-18, the frontier-lead mining pack —
///       `let_and_return` + `ptr_arg` + `redundant_slicing` +
///       `map_flatten`, Plan 146 / Research 182; each fixture is the
///       oracle-verified frontier-family (seed, broken) pair, and
///       `needless_borrow`'s receiver extension rides its existing entry).
///     + the 2 Batch 156 rows (2026-09-18, the frontier-lead mining pack II
///       — `io_other_error` + `wrong_self_convention`, Plan 147; the
///       re-armed frontier leads, each fixture the oracle-verified family
///       pair).
///     + the 2 Batch 157 rows (2026-09-18, the frontier-lead mining pack III
///       — `redundant_closure_for_method_calls` + `iter_skip_next`, Plan
///       148; the Batch 156 re-arm's leads, each fixture the
///       oracle-verified family pair).
///     + the 2 Batch 158 rows (2026-09-18, the frontier-lead mining pack IV
///       — `to_string_in_format_args` + `manual_range_contains`, Plan
///       149; the Batch 157 re-arm's leads, each fixture the
///       oracle-verified family pair; the perf-group pin rode the plan149
///       probe's default-groups arm).
///     + the 2 Batch 159 rows (2026-09-18, the frontier-lead mining pack V
///       — `single_char_add_str` + `manual_clamp`, Plan 150; the Batch 158
///       re-arm's leads, each fixture the oracle-verified family pair;
///       BOTH groups style/default-on, pinned by the plan150 probe's
///       default-groups arms).
///     + the 2 Batch 160 rows (2026-09-18, the frontier-lead mining pack VI
///       — `from_str_radix_10` + `assign_op_pattern`, Plan 151; the Batch
///       159 re-arm's leads, each fixture the oracle-verified family
///       pair; BOTH groups style/default-on, pinned by the plan151
///       probe's default-groups arms).
///     + the 2 Batch 161 rows (2026-09-18, the frontier-lead mining pack VII
///       — `unit_arg` + `string_lit_as_bytes`, Plan 152; the Batch 160
///       re-arm's leads, each fixture the oracle-verified family pair;
///       complexity/default-on + NURSERY groups pinned by the plan152
///       probe's default-groups arms).
///     + the 2 Batch 162 rows (2026-09-19, the frontier-lead mining pack
///       VIII — `bytes_count_to_len` + `bool_comparison`, Plan 153; the
///       Batch 161 re-arm's leads, both complexity/default-on, pinned by
///       the plan152 probe arms).
///     + the 2 Batch 163 rows (2026-09-19, the frontier-lead mining pack IX
///       — `manual_string_new` + `needless_late_init`, Plan 154; the
///       Batch 162 re-arm's recorded spares; pedantic + style groups
///       pinned by the plan154 probe's group arms).
///     + the 2 Batch 164 rows (2026-09-19, the frontier-lead mining pack X
///       — `match_like_matches_macro` + `excessive_precision`, Plan 155;
///       the coverage-report re-arm leads, both style/default-on, pinned
///       by the plan155 probe's no-flags arms).
///     + the 2 Batch 167 rows (2026-09-21, the coverage-report re-arm
///       pack XI — `unnecessary_cast` + `clone_on_copy`, Plan 158; the
///       fresh coverage-report derivation's leads, both
///       complexity/default-on, pinned by the plan158 probe arms).
///     + the 2 Batch 168 rows (2026-09-21, the coverage-report re-arm
///       pack XII — `empty_line_after_doc_comments` +
///       `new_without_default`, Plan 159; the suspicious group's FIRST
///       corpus row + the purely additive `impl Default`, both
///       default-on, the TENTH consecutive all-FIXABLE pack).
///     + the 2 Batch 169 rows (2026-09-21, the coverage-report pack
///       XIII — `match_ref_pats` + `into_iter_on_ref`, Plan 160; the
///       Batch-168 staged alternates, both style/default-on, pinned by
///       the plan160 probe — the deref-scrutinee `&`-strip + the
///       container-proven method rename; the ELEVENTH consecutive
///       all-FIXABLE pack).
///     + the 2 Batch 175 rows (2026-09-24, the coverage-report pack XV —
///       `len_without_is_empty` + `cast_abs_to_unsigned`, Plan 169; the
///       pack-XV OVERTURNS of two Plan-163 refusals on new evidence —
///       the landed `new_without_default` insertion precedent and the
///       `scalar_param_heads_over` provenance substrate; both atoms fire
///       ALONE (zero co-fires, the plan169 probe), style + suspicious
///       groups, the TWELFTH consecutive all-FIXABLE pack).
// Data table, not logic — length is the point. The entry count is
// derived (module doc), never typed here.
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
            lint_group: "complexity".to_string(),
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
            lint_group: "style".to_string(),
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
            lint_group: "style".to_string(),
            buggy_code: "fn sum_vec(v: &Vec<i32>) -> i32 {\n    let mut total = 0;\n    for i in 0..v.len() { total += v[i]; }\n    total\n}".to_string(),
            fixed_code: "fn sum_vec(v: &Vec<i32>) -> i32 {\n    v.iter().sum()\n}".to_string(),
        },
        // ── perf group ───────────────────────────────────────────────────
        ClippyFixEntry {
            lint_name: "useless_conversion".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn add_one(x: i32) -> i32 {\n    let y: i32 = x.into();\n    y + 1\n}".to_string(),
            fixed_code: "fn add_one(x: i32) -> i32 {\n    x + 1\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "unnecessary_mut_passed".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn length(s: &str) -> usize { s.len() }\nfn caller(mut s: String) -> usize { length(&mut s) }".to_string(),
            fixed_code: "fn length(s: &str) -> usize { s.len() }\nfn caller(s: String) -> usize { length(&s) }".to_string(),
        },
        // ── nursery / pedantic ───────────────────────────────────────────
        ClippyFixEntry {
            lint_name: "needless_borrow".to_string(),
            lint_group: "style".to_string(),
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
            lint_group: "pedantic".to_string(),
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
            lint_group: "pedantic".to_string(),
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
            // Batch 183 (P41 fix half): WIDENED to a two-fn file — the
            // original bare-receiver fn plus the method-chain fn-path
            // mapper fn (the Elo-1222 toughest-span class, the riir-refine
            // store's own top-1). PROBE (2026-09-28, box clippy 1.98.1,
            // temp crate removed after): upstream `map_unwrap_or` is
            // SILENT on the recorded store spelling
            // `strip_suffix(..).map(f).unwrap_or(a)` — it fires on the
            // `unwrap_or_else` variant only (the bare-receiver `unwrap_or`
            // form still fires, so the blocker is the method-call receiver
            // under `unwrap_or`) — the widened fn carries the FIRING form;
            // the similarity bag still template-matches the `unwrap_or`
            // store span (the `strip_suffix` + `map(str::to_string)`
            // tokens dominate). rustc-verified. Entry count unchanged; the
            // ATOM's code changed → freeze-table regen in this batch.
            buggy_code: "fn f(x: Option<u8>) -> u8 { x.map(|v| v + 1).unwrap_or(0) }\nfn g(text: &str, suffix: &str, fallback: &str) -> String { text.strip_suffix(suffix).map(str::to_string).unwrap_or_else(|| fallback.to_string()) }".to_string(),
            fixed_code: "fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }\nfn g(text: &str, suffix: &str, fallback: &str) -> String { text.strip_suffix(suffix).map_or_else(|| fallback.to_string(), str::to_string) }".to_string(),
        },
        // P15 post-mining intake (2026-09-14, the born-lint closure — the
        // healer's own map_unwrap_or `map_or(false, f)` fallback output is
        // re-flagged by real clippy as this lint). Probe evidence:
        // `.raw/mapor_probe`, clippy 1.98 — all terminal suggestions
        // MachineApplicable.
        ClippyFixEntry {
            lint_name: "unnecessary_map_or".to_string(),
            lint_group: "style".to_string(),
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
            lint_group: "complexity".to_string(),
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
            lint_group: "complexity".to_string(),
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
            lint_group: "pedantic".to_string(),
            buggy_code: "fn f(s: &str) -> bool { s.contains(\"x\") }".to_string(),
            fixed_code: "fn f(s: &str) -> bool { s.contains('x') }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_is_multiple_of".to_string(),
            lint_group: "complexity".to_string(),
            // Post-mining P7. VERIFY-tier: parse-only cannot see dividend
            // signedness and `is_multiple_of` is unsigned-only — the
            // `--verify` compile gate reverts any signed-dividend break.
            // Batch 183 (P40 fix half): WIDENED to a two-fn file — the
            // original literal-divisor fn plus the variable-divisor fn,
            // because the apply leg had no template for `n % d == 0`
            // (riir-refine store L=2, riir-ai L=4 recorded misses; upstream
            // FIRES all three recorded spellings — probe 2026-09-27,
            // verdict in the snapshot's P40 row). The proposer splices
            // variable divisors verbatim
            // (`non_literal_divisor_spliced_verbatim`). rustc-verified +
            // upstream-FIRES-verified 2026-09-28. Entry count unchanged
            // (the corpus is one entry per lint); the ATOM's code changed,
            // so the generation fixtures moved → freeze-table regen in
            // this batch.
            buggy_code: "fn f(n: u64) -> bool { n % 4 == 0 }\nfn g(n: u64, d: u64) -> bool { n % d == 0 }".to_string(),
            fixed_code: "fn f(n: u64) -> bool { n.is_multiple_of(4) }\nfn g(n: u64, d: u64) -> bool { n.is_multiple_of(d) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_range_patterns".to_string(),
            lint_group: "complexity".to_string(),
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
            lint_group: "nursery".to_string(),
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
            lint_group: "nursery".to_string(),
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
            // Batch 183 (P41 fix half): WIDENED to a two-fn file — the
            // original bare-Path scrutinee fn plus the unsafe-BLOCK
            // scrutinee fn (the live riir-chain `match unsafe {
            // read_array(..) }` shapes): the block's text ends in `}`, so
            // the rewrite must parenthesize (`let Some(v) = (unsafe {
            // … }) else` — the riir-wallet-worker escape, already the
            // proposer's own behavior; the module test pins it). The new
            // fn is AUTHORED for edition 2021: a LOCAL raw pointer in a
            // safe fn (an `unsafe fn` + explicit block trips
            // `unused_unsafe`; a pointer-ARG deref trips deny-by-default
            // `not_unsafe_ptr_arg_deref` — both P41 fixture lessons).
            // rustc-verified + upstream-FIRES-verified 2026-09-28. Entry
            // count unchanged; the ATOM's code changed → freeze-table
            // regen in this batch.
            buggy_code: "fn f(x: Option<u32>) -> u32 { let v = match x { Some(v) => v, None => return 0 }; v + 1 }\nfn g(data: &mut u32) -> u32 { let p = data as *mut u32; let v = match unsafe { p.as_ref() } { Some(v) => v, None => return 0 }; *v + 1 }".to_string(),
            fixed_code: "fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }\nfn g(data: &mut u32) -> u32 { let p = data as *mut u32; let Some(v) = (unsafe { p.as_ref() }) else { return 0 }; *v + 1 }".to_string(),
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
            buggy_code: "fn ids(bytes: &[u8]) -> Vec<u32> {\n    let mut out = Vec::new();\n    for chunk in bytes.chunks_exact(4) {\n        out.push(u32::from_le_bytes(chunk.try_into().unwrap()));\n    }\n    out\n}\nfn firsts(bytes: &[u8]) -> Vec<u8> {\n    bytes.chunks_exact(4).map(|p| p[0]).collect()\n}\nfn tails(payload: &[u8]) -> u32 {\n    let mut t = 0u32;\n    for c in payload[5..].chunks_exact(4) {\n        t += u32::from_le_bytes(c.try_into().unwrap());\n    }\n    t\n}".to_string(),
            // Batch 183 (P41 fix half): the third fn `tails` carries the
            // offset-SUBSCRIPT receiver (the recorded non-bare-receiver
            // class: katgpt-rs `payload[5..].chunks_exact(4)`, riir-train
            // `remap_bytes.chunks_exact(4)` multi-line; the riir-ai
            // `bytes.chunks_exact(shard_size)` non-const-size form stays
            // UNFIXABLE by design — the proposer's const-expr guard
            // refuses it). REPAIR in the same edit: the old fixed
            // `from_le_bytes(chunk)` was an E0308 — the for-loop iterates
            // `&[[u8; 4]]`, so `chunk` is `&[u8; 4]` and `from_le_bytes`
            // takes the array BY VALUE (`*chunk`). The pipeline never
            // compiled `fixed_code` (the freeze labels only the buggy
            // side; the standing gate is syn-parse), so the broken
            // template shipped — the apply leg would paste it and burn a
            // verify-revert. rustc-verified + upstream-FIRES-verified
            // 2026-09-28. Entry count unchanged; the ATOM's code changed
            // → freeze-table regen in this batch.
            fixed_code: "fn ids(bytes: &[u8]) -> Vec<u32> {\n    let mut out = Vec::new();\n    for chunk in bytes.as_chunks::<4>().0 {\n        out.push(u32::from_le_bytes(*chunk));\n    }\n    out\n}\nfn firsts(bytes: &[u8]) -> Vec<u8> {\n    bytes.as_chunks::<4>().0.iter().map(|p| p[0]).collect()\n}\nfn tails(payload: &[u8]) -> u32 {\n    let mut t = 0u32;\n    for c in payload[5..].as_chunks::<4>().0 {\n        t += u32::from_le_bytes(*c);\n    }\n    t\n}".to_string(),
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
            lint_group: "style".to_string(),
            buggy_code: "fn kept(xs: Vec<i32>) -> Vec<i32> {\n    xs.into_iter().filter_map(|x| (x > 0).then(|| x * 2)).collect()\n}".to_string(),
            fixed_code: "fn kept(xs: Vec<i32>) -> Vec<i32> {\n    xs.into_iter().filter(|&x| (x > 0)).map(|x| x * 2).collect()\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "vec_init_then_push".to_string(),
            lint_group: "perf".to_string(),
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
        // ── Batch 155 (2026-09-18, the frontier-lead mining pack — the
        // frontier miner's first 5 consumed leads, Plan 146 / Research 182.
        // Each fixture is the ORACLE-VERIFIED family (seed, broken) pair:
        // seed clean under all+pedantic+nursery, broken fires EXACTLY its
        // target lint — the fixture IS the probe. needless_borrow's
        // extension rides its existing seed entry.) ──
        ClippyFixEntry {
            lint_name: "let_and_return".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn shout(s: &str) -> String { let out = s.to_uppercase(); out }".to_string(),
            fixed_code: "fn shout(s: &str) -> String { s.to_uppercase() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "ptr_arg".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn total(v: &Vec<i32>) -> i32 { v.iter().sum() }".to_string(),
            fixed_code: "fn total(v: &[i32]) -> i32 { v.iter().sum() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "redundant_slicing".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: &str) -> char { first_char(&s[..]) }".to_string(),
            fixed_code: "fn first_char(s: &str) -> char { s.chars().next().unwrap_or(' ') }\nfn pick(s: &str) -> char { first_char(s) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "map_flatten".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().map(|x| x.split(' ')).flatten().collect() }".to_string(),
            fixed_code: "fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().flat_map(|x| x.split(' ')).collect() }".to_string(),
        },
        // ── Batch 156 (2026-09-18, the frontier-lead mining pack II — the
        // frontier miner's re-armed leads, Plan 147. Each fixture is the
        // ORACLE-VERIFIED family (seed, broken) pair from the Batch 155
        // re-arm: seed clean under all+pedantic+nursery, broken fires
        // EXACTLY its target lint.) ──
        ClippyFixEntry {
            lint_name: "io_other_error".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn mk(e: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, e) }".to_string(),
            fixed_code: "fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "wrong_self_convention".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(&self) -> String { format!(\"sig-{}\", self.code) }\n}".to_string(),
            fixed_code: "struct Sig { code: u32 }\nimpl Sig {\n    fn into_label(self) -> String { format!(\"sig-{}\", self.code) }\n}".to_string(),
        },
        // ── Batch 157 (2026-09-18, the frontier-lead mining pack III — the
        // Batch 156 re-arm's leads, Plan 148. Each fixture is the
        // ORACLE-VERIFIED family (seed, broken) pair from the Batch 156
        // re-arm probe: seed clean under all+pedantic+nursery, broken fires
        // EXACTLY its target lint.) ──
        ClippyFixEntry {
            lint_name: "redundant_closure_for_method_calls".to_string(),
            lint_group: "pedantic".to_string(),
            buggy_code: "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(|s| s.len()).collect() }".to_string(),
            fixed_code: "fn lens(v: &[String]) -> Vec<usize> { v.iter().map(String::len).collect() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "iter_skip_next".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn at(v: &[u8], n: usize) -> Option<&u8> { v.iter().skip(n).next() }".to_string(),
            fixed_code: "fn at(v: &[u8], n: usize) -> Option<&u8> { v.get(n) }".to_string(),
        },
        // ── Batch 158 (2026-09-18, the frontier-lead mining pack IV — the
        // Batch 157 re-arm's leads, Plan 149. Each fixture is the
        // ORACLE-VERIFIED family (seed, broken) pair from the Batch 157
        // re-arm probe: seed clean under all+pedantic+nursery, broken fires
        // EXACTLY its target lint. `to_string_in_format_args` is a PERF-
        // group (default-on) lint — the group pinned by the plan149
        // probe's default-groups arm.) ──
        ClippyFixEntry {
            lint_name: "to_string_in_format_args".to_string(),
            lint_group: "perf".to_string(),
            buggy_code: "fn show(x: u32) -> String { format!(\"n={}\", x.to_string()) }".to_string(),
            fixed_code: "fn show(x: u32) -> String { format!(\"n={}\", x) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_range_contains".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); x >= 3 && x <= 5 }".to_string(),
            fixed_code: "fn in_band(x: u8, hits: &mut Vec<u8>) -> bool { hits.push(x); (3..=5).contains(&x) }".to_string(),
        },
        // ── Batch 159 (2026-09-18, the frontier-lead mining pack V — the
        // Batch 158 re-arm's leads, Plan 150. Each fixture is the
        // ORACLE-VERIFIED family (seed, broken) pair from the Batch 158
        // re-arm probe: seed clean under all+pedantic+nursery, broken fires
        // EXACTLY its target lint. Both style-group (default-on) — groups
        // pinned by the plan150 probe's default-groups arms.) ──
        ClippyFixEntry {
            lint_name: "single_char_add_str".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn mark(s: &mut String) { s.push_str(\"!\") }".to_string(),
            fixed_code: "fn mark(s: &mut String) { s.push('!') }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_clamp".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.max(3).min(5) }".to_string(),
            fixed_code: "fn band(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); x.clamp(3, 5) }".to_string(),
        },
        // ── Batch 160 (2026-09-18, the frontier-lead mining pack VI — the
        // Batch 159 re-arm's leads, Plan 151. Each fixture is the
        // ORACLE-VERIFIED family (seed, broken) pair from the Batch 159
        // re-arm probe: seed clean under all+pedantic+nursery, broken fires
        // EXACTLY its target lint. Both style-group (default-on) — groups
        // pinned by the plan151 probe's default-groups arms.) ──
        ClippyFixEntry {
            lint_name: "from_str_radix_10".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(s, 10) }".to_string(),
            fixed_code: "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "assign_op_pattern".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y = y + 1; y }".to_string(),
            fixed_code: "fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y += 1; y }".to_string(),
        },
        // ── Batch 161 (2026-09-18, the frontier-lead mining pack VII — the
        // Batch 160 re-arm's leads, Plan 152. Each fixture is the
        // ORACLE-VERIFIED family (seed, broken) pair from the Batch 160
        // re-arm probe: seed clean under all+pedantic+nursery, broken fires
        // EXACTLY its target lint. `unit_arg` is complexity-group
        // (default-on) and `string_lit_as_bytes` is NURSERY (allow-by-
        // default) — BOTH groups pinned by the plan152 probe's
        // default-groups arms; the Batch 160 miner comment's "default-level
        // perf" for string_lit_as_bytes was wrong on both counts.) ──
        ClippyFixEntry {
            lint_name: "unit_arg".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { touch({ hits.push(1); }, hits) }".to_string(),
            fixed_code: "fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }\nfn go(hits: &mut Vec<u8>) { { hits.push(1); };\ntouch((), hits) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "string_lit_as_bytes".to_string(),
            lint_group: "nursery".to_string(),
            buggy_code: "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); \"abc\".as_bytes() }".to_string(),
            fixed_code: "fn raw(hits: &mut Vec<u8>) -> &'static [u8] { hits.push(0); b\"abc\" }".to_string(),
        },
        // ── Batch 162 (2026-09-19, the frontier-lead mining pack VIII —
        // the Batch 161 re-arm's leads, Plan 153. Each fixture is the
        // ORACLE-VERIFIED family pair from the Batch 159 re-arm probe
        // (scripts/plan152_frontier_probe.py): seed clean, broken fires
        // EXACTLY its target. Both complexity-group (default-on) — the
        // group pinned by the probe's arms.) ──
        ClippyFixEntry {
            lint_name: "bytes_count_to_len".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.bytes().count() }".to_string(),
            fixed_code: "fn n(s: &str, hits: &mut Vec<u8>) -> usize { hits.push(0); s.len() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "bool_comparison".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x == true }".to_string(),
            fixed_code: "fn flag(x: bool, hits: &mut Vec<bool>) -> bool { hits.push(x); x }".to_string(),
        },
        // ── Batch 163 (2026-09-19, the frontier-lead mining pack IX —
        // the Batch 162 re-arm's recorded spares, Plan 154. Each fixture is
        // the ORACLE-VERIFIED family pair from the plan154 probe
        // (scripts/plan154_frontier_probe.py): seed clean, broken fires
        // EXACTLY its target. manual_string_new is PEDANTIC-group
        // (allow-by-default; pinned by the probe's group arms — the bench
        // oracle passes -W clippy::pedantic); needless_late_init is
        // style/default-on. The nli fixture's twice-used-binding `x + x`
        // tail dodges our own let_and_return — the recorded plan152 spare
        // dodge.) ──
        ClippyFixEntry {
            lint_name: "manual_string_new".to_string(),
            lint_group: "pedantic".to_string(),
            buggy_code: "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::from(\"\") }".to_string(),
            fixed_code: "fn mk(hits: &mut Vec<u8>) -> String { hits.push(0); String::new() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_late_init".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x; x = 5u32; x + x }".to_string(),
            fixed_code: "fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x + x }".to_string(),
        },
        // ── Batch 164 (2026-09-19, the frontier-lead mining pack X —
        // the coverage-report re-arm, Plan 155. Each fixture is the
        // ORACLE-VERIFIED family pair from the plan155 probe
        // (scripts/plan155_frontier_probe.py): seed clean, broken fires
        // EXACTLY its target. Both style/default-on — the group pinned by
        // the probe's no-flags arms. The mlmm fixture's `+ 0` tail dodges
        // our own needless_return; the ep fixture is the probe seed.) ──
        ClippyFixEntry {
            lint_name: "match_like_matches_macro".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); match x { 0 => true, _ => false } }".to_string(),
            fixed_code: "fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); matches!(x, 0) }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "excessive_precision".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 2.141592653589793238462643f32 }".to_string(),
            fixed_code: "fn f(hits: &mut Vec<u8>) -> f32 { hits.push(0); 2.1415927f32 }".to_string(),
        },
        // ── Batch 167 (2026-09-21, the coverage-report re-arm pack XI —
        // Plan 158, the 2026-09-21 derivation leads, probe
        // scripts/plan158_frontier_probe.py). Each fixture is the
        // ORACLE-VERIFIED family pair from the probe: seed clean, broken
        // fires EXACTLY its target. Both complexity/default-on — the
        // group pinned by the probe's no-flags arms. The uc fixture is
        // the PARAM shape — the probe's annotated-let seed co-fired our
        // own let_and_return (tail-expression shape), so the pack ships
        // the shape the v1 proposer owns.) ──
        ClippyFixEntry {
            // u64 (not u32): the scalar dodge — a u32 fixture triple-crowds
            // the u32-heavy eval spans (the deref_addrof span lost its
            // top-3 to the raw u32 token count, measured in the plan158
            // A/B rank dump); the lint fires identically on any scalar.
            lint_name: "unnecessary_cast".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(hits: &mut Vec<u8>, p: u64) -> u64 { hits.push(0); p as u64 }".to_string(),
            fixed_code: "fn f(hits: &mut Vec<u8>, p: u64) -> u64 { hits.push(0); p }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "clone_on_copy".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn f(hits: &mut Vec<u8>, x: u64) -> u64 { hits.push(0); x.clone() }".to_string(),
            fixed_code: "fn f(hits: &mut Vec<u8>, x: u64) -> u64 { hits.push(0); x }".to_string(),
        },
        // ── Batch 168 (2026-09-21, the coverage-report re-arm pack XII —
        // Plan 159, the post-Batch-167 derivation leads, probe
        // scripts/plan159_pack12_probe.py). eld is suspicious/default-on —
        // the group's FIRST corpus row (the derivation's suspicious column
        // read 0 covered); the probed silent shape (doc + blank +
        // `#[derive]`) is the guard, mirrored. nwd is style/default-on —
        // the fix is purely additive (clippy's own "try adding this"
        // shape). The nwd fixture carries no `hits` param BY PREMISE:
        // `new` must be zero-arg for the lint to fire at all.) ──
        ClippyFixEntry {
            lint_name: "empty_line_after_doc_comments".to_string(),
            lint_group: "suspicious".to_string(),
            buggy_code: "/// Adds one.\n\nfn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); 1 }".to_string(),
            fixed_code: "/// Adds one.\nfn f(hits: &mut Vec<u8>) -> u8 { hits.push(0); 1 }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "new_without_default".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "pub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } }".to_string(),
            fixed_code: "pub struct W { pub x: u8 }\nimpl W { pub fn new() -> Self { W { x: 1 } } }\n\nimpl Default for W {\n    fn default() -> Self {\n        Self::new()\n    }\n}".to_string(),
        },
        // ── Batch 169 (2026-09-21, the coverage-report pack XIII — the
        // Batch-168 staged alternates, Plan 160, probe
        // scripts/plan160_pack13_probe.py). mrf fires ONLY its own lint
        // on the no-binding fixture (the binding shape co-fires
        // needless_borrowed_reference — probe — so the atom must be the
        // clean no-binding spelling, and the proposer REFUSES bindings
        // anyway: the binding-mode hazard is the staged scope). iir's
        // clean fixture is the bare `.next()` shape (the `.count()` tails
        // co-fire iter_count — probe — the gen-8 shaped-verdict class);
        // the receiver ident is signature-proven container-typed (the
        // param_heads_over precedent). ──
        ClippyFixEntry {
            lint_name: "match_ref_pats".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "pub enum Fruit { Apple, Pear }\nfn f(hits: &mut Vec<u8>, x: &Fruit) -> u64 { hits.push(0); match x { &Fruit::Apple => 1, &Fruit::Pear => 2 } }".to_string(),
            fixed_code: "pub enum Fruit { Apple, Pear }\nfn f(hits: &mut Vec<u8>, x: &Fruit) -> u64 { hits.push(0); match *x { Fruit::Apple => 1, Fruit::Pear => 2 } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "into_iter_on_ref".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn f(hits: &mut Vec<u8>, v: Vec<u64>) -> Option<u64> { hits.push(0); (&v).into_iter().next().copied() }".to_string(),
            fixed_code: "fn f(hits: &mut Vec<u8>, v: Vec<u64>) -> Option<u64> { hits.push(0); v.iter().next().copied() }".to_string(),
        },
        // ── Batch 171 (the coverage-report pack XIV, Plan 163): the
        // comment-byte tab spacing (probe: `///`/`//!`/`/** */`/code
        // samples all fire; regular `//` silent — mirrored) + the
        // shape-proven adapter rename (`x.clone()` on `&T` proves T:
        // Clone = cloned(); `*x` proves T: Copy = copied(); into_iter
        // chains refused — clippy's empty suggestion there is Copy-only). ──
        ClippyFixEntry {
            lint_name: "tabs_in_doc_comments".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "///\ttabbed doc\nfn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 1 }".to_string(),
            fixed_code: "///    tabbed doc\nfn f(hits: &mut Vec<u8>) -> u64 { hits.push(0); 1 }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "map_clone".to_string(),
            lint_group: "style".to_string(),
            // The DEREF shape is the corpus atom (the regen's catch: the
            // clone shape co-fires clone_on_copy on Copy elements AND
            // redundant_closure_for_method_calls BY NATURE — `|x| x.clone()`
            // IS a method-forwarding closure; the deref body fires exactly
            // map_clone, probe mc_deref_copy). The clone arm lives in the
            // module tests.
            buggy_code: "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Vec<u8> { hits.push(0); v.iter().map(|x| *x).collect() }".to_string(),
            fixed_code: "fn f(hits: &mut Vec<u8>, v: &Vec<u8>) -> Vec<u8> { hits.push(0); v.iter().copied().collect() }".to_string(),
        },
        // ── Batch 175 (2026-09-24, the coverage-report pack XV, Plan 169,
        // probe scripts/plan169_pack15_probe.py): both atoms fire ALONE
        // (zero co-fires). lwie = the probe's plain inherent-impl shape —
        // the additive is_empty insertion, the NWD precedent, the
        // Plan-163 fn-body-invention refusal OVERTURNED on the landed
        // gate-proven insertion substrate. cau = the probe's cau_i64
        // same-width shape — the signature-proven unsigned_abs rewrite,
        // the Plan-163 typeck refusal OVERTURNED on the Plan-149
        // param-heads substrate; narrow/wide regimes keep the cast and
        // live in the module tests. ──
        ClippyFixEntry {
            lint_name: "len_without_is_empty".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "pub struct LwA { items: Vec<u8> }\nimpl LwA {\n    pub fn len(&self) -> usize {\n        self.items.len()\n    }\n}".to_string(),
            fixed_code: "pub struct LwA { items: Vec<u8> }\nimpl LwA {\n    pub fn len(&self) -> usize {\n        self.items.len()\n    }\n    pub fn is_empty(&self) -> bool {\n        self.len() == 0\n    }\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "cast_abs_to_unsigned".to_string(),
            lint_group: "suspicious".to_string(),
            buggy_code: "pub fn cau_a(x: i64) -> u64 {\n    x.abs() as u64\n}".to_string(),
            fixed_code: "pub fn cau_a(x: i64) -> u64 {\n    x.unsigned_abs()\n}".to_string(),
        },
        // ── Batch 184 (2026-09-28, the coverage-report pack XVI, Plan 179,
        // probe scripts/plan179_pack16_probe.py): both atoms fire ALONE
        // (zero co-fires, measured) and both FIXED forms compile clean +
        // silent at probe time (the batch-183 latent-E0308 class closed in
        // the probe). powc = the as_ptr-local shape — the pointer-
        // provenance rewrite, the Plan-169 staged IOU; the negative arm
        // and the u64 operand are probe-measured upstream-silent (the
        // usize precondition is parity, not divergence). nbr = the let
        // &ref shape with an auto-deref usage (tolerates the one-level
        // binding move the deletion makes) — the plan158 RETIRE overturned
        // on the plan169 probe's measured firing; the match shape co-fires
        // match_ref_pats and is refused by construction. ──
        ClippyFixEntry {
            lint_name: "ptr_offset_with_cast".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "pub fn powc_a(b: &[u8], i: usize) -> u8 {\n    let p = b.as_ptr();\n    unsafe { *p.offset(i as isize) }\n}".to_string(),
            fixed_code: "pub fn powc_a(b: &[u8], i: usize) -> u8 {\n    let p = b.as_ptr();\n    unsafe { *p.add(i) }\n}".to_string(),
        },
        ClippyFixEntry {
            lint_name: "needless_borrowed_reference".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "pub fn nbr_b(a: u64) -> String {\n    let &ref y = &a;\n    y.to_string()\n}".to_string(),
            fixed_code: "pub fn nbr_b(a: u64) -> String {\n    let ref y = &a;\n    y.to_string()\n}".to_string(),
        },
        // ── Batch 185 (2026-09-28, the coverage-report pack XVII — MBL
        // solo, Plan 180, probe scripts/plan180_pack17_probe.py): the
        // subset-driven generic-bound consolidation. Upstream emits NO
        // replacement (both duplicate locations are primary spans —
        // probe-measured), so the fix is synthesized; the atom fires
        // ALONE (zero co-fires) and both fixed directions compile clean +
        // silent at probe time. fn-only surface = upstream parity (trait
        // impls / struct definitions probe-silent). MTA
        // (missing_transmute_annotations) was the LEAD-2 candidate and is
        // probe-REJECTED this batch — the negative matrix is permanent
        // record in the probe + plan. ──
        ClippyFixEntry {
            lint_name: "multiple_bound_locations".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "pub fn mbl_a<T: Default>(t: T) -> T\nwhere\n    T: Default,\n{\n    t\n}".to_string(),
            fixed_code: "pub fn mbl_a<T>(t: T) -> T\nwhere\n    T: Default,\n{\n    t\n}".to_string(),
        },
        // ── Batch 191 (2026-09-30, the coverage-report pack XVIII — the
        // mem_replace_with_default solo, Plan 188, probe
        // scripts/plan188_pack18_probe.py): `std::mem::replace(dst,
        // Default::default())` → `std::mem::take(dst)` — the STD
        // DEFINITION IDENTITY (`take` IS `replace(dest, T::default())`,
        // the std source). Both firing path spellings verified
        // (std-path + the import-verified `mem::` form), the turbofish /
        // non-default / take-already shapes probe-SILENT, and both fixed
        // forms compile clean + silent. The atom avoids the let-tail
        // shape (the probe's let_and_return co-fire — designed out).
        // Bare `replace(..)` FIRES upstream but refuses here (the
        // SAFER-than-clippy import-ambiguity divergence, probe-recorded);
        // `<_>::default()` fires upstream, refuses v1 (documented
        // narrower divergence). Style-group (default-on; the installed
        // driver is the authority — the lint_level gate's own derivation). ──
        ClippyFixEntry {
            lint_name: "mem_replace_with_default".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn mrw(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::replace(dst, Default::default())) }".to_string(),
            fixed_code: "fn mrw(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::take(dst)) }".to_string(),
        },
        // ── Batch 228 (2026-10-13, the coverage-report re-arm pack XIX —
        // Plan 230, probe scripts/plan230_pack19_probe.py): the
        // std/adapter DEFINITION-IDENTITY trio from the fresh 0.1.98
        // derivation's evidence tail. manual_repeat_n: the adapter is
        // DEFINED as the expansion (RepeatN yields value count times —
        // the same elements as repeat(value).take(count), cloned per
        // yield in both); the input's compilation proves T: Clone; MSRV
        // floor 1.82 carried as the proposer's context_note.
        // manual_ok_err: Option::ok/err are DEFINED as these matches
        // (the std source); two-arm identity matches only — the if-let
        // form fires upstream but refuses v1 (documented narrower
        // divergence). manual_next_back: Rev<I> inverts direction, so
        // `it.rev().next()` IS `it.next_back()`; the input's compilation
        // proves DoubleEndedIterator. All three: probe VERDICT PASS
        // (8 firing / 3 silent / 3 posture / 6 fixed forms compile clean
        // + silent, zero co-fires). ──
        ClippyFixEntry {
            lint_name: "manual_repeat_n".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn mrn(n: usize) -> Vec<u32> { std::iter::repeat(0u32).take(n).collect() }".to_string(),
            fixed_code: "fn mrn(n: usize) -> Vec<u32> { std::iter::repeat_n(0u32, n).collect() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_ok_err".to_string(),
            lint_group: "complexity".to_string(),
            buggy_code: "fn moe(r: Result<u32, String>) -> Option<u32> { match r { Ok(v) => Some(v), Err(_) => None } }".to_string(),
            fixed_code: "fn moe(r: Result<u32, String>) -> Option<u32> { r.ok() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_next_back".to_string(),
            lint_group: "style".to_string(),
            buggy_code: "fn mnb(vals: &[u32]) -> Option<u32> { vals.iter().rev().next().copied() }".to_string(),
            fixed_code: "fn mnb(vals: &[u32]) -> Option<u32> { vals.iter().next_back().copied() }".to_string(),
        },
        // ── Batch 230 (the 2026-10-13 clippy-1.99 new-lint wave, Plan 232 —
        // the TOOLCHAIN-MOVE RE-ARM: upstream stable 1.98.1→1.99.0 shipped
        // 2026-09-28; the Batch-91 precedent class. All five probe-verified
        // against the ISOLATED +1.99.0 toolchain this session; every fixed
        // form below COMPILE-VERIFIED at BOTH 1.99.0 and 1.98.1 (the
        // Batch-183 E0308 law: fixed_code must compile, not just parse).
        // The lints are AHEAD-OF-PIN (lint_level.rs AHEAD_OF_PIN_1_99): they
        // fire nowhere on the workspace's pinned 1.98.1 driver — the live
        // surface is public consumers on stable 1.99. Groups per the +1.99.0
        // driver's -W help (assert-is-empty/manual-bit-width = pedantic,
        // block-scrutinee/mismatched-bit-width-type = suspicious,
        // nonnull-unchecked-on-box-ptr = complexity) ──
        ClippyFixEntry {
            lint_name: "assert_is_empty".to_string(),
            lint_group: "pedantic".to_string(),
            // `assert_eq!(x, <empty>)` prints the asserted value on
            // failure — that diagnostics difference IS the lint's purpose;
            // pass-behavior identical (assert_eq! is assert!(*a == *b)).
            // The &Vec form rides `.as_slice()`; the owned-Vec cast form
            // `[T; 0]` is the proposer's signature-proven arm.
            buggy_code: "fn aie(v: &Vec<u32>, s: &str) { assert!(v.is_empty()); assert!(!s.is_empty()); }".to_string(),
            fixed_code: "fn aie(v: &Vec<u32>, s: &str) { assert_eq!(v.as_slice(), []); assert_ne!(s, \"\"); }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "manual_bit_width".to_string(),
            lint_group: "pedantic".to_string(),
            // `T::BITS - x.leading_zeros()` IS `x.bit_width()` for unsigned
            // (the std definition); signed rides cast_unsigned (the +1.99
            // upstream .fixed shape). bit_width() stabilized 1.97 — the
            // LINT ships 1.99 and clippy is MSRV-aware, so the lint only
            // fires where the fix compiles (chunks_exact precedent).
            buggy_code: "fn mbw(w: u32, x: i64) -> u32 { u32::BITS - w.leading_zeros() }".to_string(),
            fixed_code: "fn mbw(w: u32, x: i64) -> u32 { w.bit_width() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "mismatched_bit_width_type".to_string(),
            lint_group: "suspicious".to_string(),
            // The LHS names a DIFFERENT width than the receiver — a
            // probable bug; the fix follows the RECEIVER's kind (probed:
            // `u32::BITS - w.leading_zeros()` with w: u8 → `w.bit_width()`).
            buggy_code: "fn mbwt(w: u8) -> u32 { u32::BITS - w.leading_zeros() }".to_string(),
            fixed_code: "fn mbwt(w: u8) -> u32 { w.bit_width() }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "block_scrutinee".to_string(),
            lint_group: "suspicious".to_string(),
            // HELP-ONLY upstream (no .fixed) — the identity fix is ours:
            // un-brace the single-expression scrutinee; same expression,
            // same evaluation point. Multi-statement blocks and unsafe
            // wrappers refuse (upstream ignores unsafe scrutinees too).
            buggy_code: "fn bs(o: Option<u32>) -> u32 { match { o.unwrap() } { n => n + 1 } }".to_string(),
            fixed_code: "fn bs(o: Option<u32>) -> u32 { match o.unwrap() { n => n + 1 } }".to_string(),
        },
        ClippyFixEntry {
            lint_name: "nonnull_unchecked_on_box_ptr".to_string(),
            lint_group: "complexity".to_string(),
            // Same pointer value, leak-on-both-sides ownership (into_raw
            // leaks; Box::leak leaks) — safe-replaces-unsafe via
            // NonNull::from_mut(&mut) (a reference is provably non-null).
            // Only BOX_LEAK MSRV-gated upstream; from_mut compiles at
            // 1.98.1 (probe-verified).
            buggy_code: "fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { unsafe { std::ptr::NonNull::new_unchecked(Box::into_raw(one)) } }".to_string(),
            fixed_code: "fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }".to_string(),
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
    /// 1 P31 post-mining intake row (`manual_range_patterns`, 2026-09-17
    /// — the riir-chain `51b456f` MISS artifact), and the 6 Batch 150
    /// rows (`print_with_newline`, `write_with_newline`,
    /// `unreadable_literal`, `legacy_numeric_constants`,
    /// `partialeq_to_none`, `cast_lossless` — the staged default-level +
    /// pedantic probe pack, Bench 094 evidence), and the 2 Batch 151
    /// rows (`doc_lazy_continuation`, `doc_overindented_list_items` —
    /// the doc-line state-machine port, Research 177 #7), and the 4
    /// Batch 155 rows (`let_and_return`, `ptr_arg`, `redundant_slicing`,
    /// `map_flatten` — the frontier-lead mining pack, Plan 146), and the
    /// 2 Batch 156 rows (`io_other_error`, `wrong_self_convention` — the
    /// frontier-lead mining pack II, Plan 147), and the 2
    /// Batch 157 rows (`redundant_closure_for_method_calls`,
    /// `iter_skip_next` — the frontier-lead mining pack III, Plan 148),
    /// and the 2 Batch 158 rows (`to_string_in_format_args`,
    /// `manual_range_contains` — the frontier-lead mining pack IV,
    /// Plan 149), the 2 Batch 159 rows (`single_char_add_str`,
    /// `manual_clamp` — the frontier-lead mining pack V, Plan 150), the
    /// 2 Batch 160 rows (`from_str_radix_10`, `assign_op_pattern` — the
    /// frontier-lead mining pack VI, Plan 151), the 2 Batch 161 rows
    /// (`unit_arg`, `string_lit_as_bytes` — the frontier-lead mining pack
    /// VII, Plan 152), the 2 Batch 162 rows (`bytes_count_to_len`,
    /// `bool_comparison` — the frontier-lead mining pack VIII, Plan 153),
    /// the 2 Batch 163 rows (`manual_string_new`, `needless_late_init`,
    /// the frontier-lead mining pack IX, Plan 154), the 2 Batch 164 rows
    /// (`match_like_matches_macro`, `excessive_precision` — the
    /// frontier-lead mining pack X, the coverage-report re-arm, Plan
    /// 155), the 2 Batch 167 rows (`unnecessary_cast`, `clone_on_copy` —
    /// the coverage-report re-arm pack XI, the 2026-09-21 derivation
    /// leads, Plan 158), and the 2 Batch 168 rows
    /// (`empty_line_after_doc_comments`, `new_without_default` — the
    /// coverage-report re-arm pack XII, the post-Batch-167 derivation
    /// leads, Plan 159), and the 2 Batch 169 rows (`match_ref_pats`,
    /// `into_iter_on_ref` — the coverage-report pack XIII, Plan 160), and
    /// the 2 Batch 171 rows (`tabs_in_doc_comments`, `map_clone` — the
    /// coverage-report pack XIV, Plan 163), and the 2 Batch 175 rows
    /// (`len_without_is_empty`, `cast_abs_to_unsigned` — the
    /// coverage-report pack XV, the Plan-163 overturns, Plan 169) = 96,
    /// and the 2 Batch 184 rows (`ptr_offset_with_cast`,
    /// `needless_borrowed_reference` — the coverage-report pack XVI, the
    /// Plan-169 staged IOU pair, Plan 179) = 98.
    ///
    /// (`multiple_bound_locations` — the coverage-report pack XVII, MBL
    /// solo — Plan 180) = 99.
    ///
    /// (`match_ref_pats`, `into_iter_on_ref` — the coverage-report pack
    /// XIII, the Batch-168 staged alternates, Plan 160) = 92.
    ///
    /// (`len_without_is_empty`, `cast_abs_to_unsigned` — the
    /// coverage-report pack XV, the Plan-163 overturns on new evidence,
    /// Plan 169) = 96. Batch 183 (2026-09-28) added NO new entries — it
    /// WIDENED four existing entries into multi-fn files (the P40/P41
    /// receiver-scrutinee fix halves) and repaired one latent E0308 in a
    /// `fixed_code`, so the one-entry-per-lint schema and the 96 count
    /// hold. Batch 184 (the pack XVI, Plan 179) then added the 2 staged
    /// IOU rows = 98.
    ///
    /// (`missing_const_for_fn` — the histogram's #1 — was proposed then
    /// KILLED: a broad const-ability proposer cross-fires every fixture
    /// and the clean-span set; see `VERIFY_LINTS` + `.plans/050`.)
    #[test]
    fn corpus_size_is_82() {
        let corpus = build_seed_corpus();
        // Batch 228 (2026-10-13, the coverage-report re-arm pack XIX,
        // Plan 230) added the std/adapter definition-identity trio
        // (manual_repeat_n + manual_ok_err + manual_next_back) = 103.
        // Batch 230 (2026-10-13, the clippy-1.99 new-lint wave, Plan 232)
        // added the five toolchain-move re-arm rows (assert_is_empty +
        // manual_bit_width + mismatched_bit_width_type + block_scrutinee +
        // nonnull_unchecked_on_box_ptr) = 108.
        assert_eq!(corpus.entries.len(), 108);
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
