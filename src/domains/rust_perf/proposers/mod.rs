//! Stage 2 proposal mechanism for the `rust_perf` domain — per-rule
//! `syn`-level AST rewrites (Plan 005 Phase 4 + Phase 6 T6.1).
//!
//! Mirrors [`crate::domains::clippy_lints::proposers`] in being a per-rule
//! codemod that matches the rule's anti-pattern in the caller's code — the
//! rewrite mechanism is **span-preserving source edits** (Phase 6 T6.1 +
//! T6.4): each proposer walks the parsed file immutably and emits
//! [`SourceEdit`]s (byte range + replacement text). `apply_edits` splices
//! them into the ORIGINAL source — every byte outside the matched spans
//! stays identical. No whole-file token re-emission, no reformat.
//!
//! The emitter substrate ([`crate::edit_emitter`]) is shared with the
//! `clippy_lints` proposers since T6.4 — both domains re-export it from
//! here for API stability.
//!
//! ## Phase 4 scope (T4.1-T4.7): the top-5 Pareto-dominant perf rules
//!
//! | Rule | Rewrite | Signature-visible change? |
//! |---|---|---|
//! | `vec-with-capacity` | `Vec::new()`/`vec![]` + push loop → `Vec::with_capacity(<cap>)` | no |
//! | `entry-vs-contains-insert` | `if !m.contains_key(&k) { m.insert(k, v); }` → `m.entry(k).or_insert(v);` | no |
//! | `static-str` | `-> String { "lit".to_string() }` → `-> &'static str { "lit" }` | **yes** (return type) |
//! | `bool-to-bitpack` | `fn(flags: &[bool]) { flags.iter().filter(..).count() }` → `fn(flags: &[u64]) { .. count_ones().sum() }` | **yes** (param type) |
//! | `fxhashmap` | `use std::collections::HashMap;` + body `HashMap` refs → `rustc_hash::FxHashMap` | no (self-gated: fires only when the file already imports `rustc_hash` — a splice cannot add the Cargo.toml dep) |
//!
//! ## Batch 48 follow-up (2026-08-17): +3 proposer-covered rules
//!
//! The three parse-clean `ParetoDominant` candidates whose proposers were
//! deferred pending bench verdicts (Bench 020/B48 — all three measured
//! `ParetoDominant`, so they are now `--fix` eligible):
//!
//! | Rule | Rewrite | Signature-visible change? |
//! |---|---|---|
//! | `hashset-insert-bool-return` | `if !s.contains(k) { s.insert(k); .. }` → `if s.insert(k) { .. }` (insert's bool return; the insert stmt is deleted) | no |
//! | `collect-append-to-extend` | `let mut staged = it.collect(); recv.append(&mut staged);` → `recv.extend(it);` (the staging `let` is deleted) | no |
//! | `vec-macro-zero-fill` | `let mut v = Vec::new(); v.resize(n, 0);` → `let mut v = vec![0; n];` (resize stmt deleted; `v` stays alive) | no |
//!
//! `filter-count-to-any` stays proposer-less BY DESIGN (`ParetoDominated`
//! 0.82-0.90x — never give a losing rewrite a `--fix` path).
//!
//! ## Batch 53 follow-up (2026-08-17): +2 proposer-covered rules
//!
//! The two Batch 53 `ParetoDominant` rules (Bench 038) land proposers:
//!
//! | Rule | Rewrite | Signature-visible change? |
//! |---|---|---|
//! | `distance-sq-no-sqrt` | `(x*x + y*y).sqrt() < r` → `(x*x + y*y) < r * r` (either operand order; sum-of-squares receiver + pure-path/literal threshold gates) | no |
//! | `hashmap-with-capacity` | `let mut m = HashMap::new();` + first-mention for-loop over a len-able iterable → `HashMap::with_capacity(iter.len())` | no |
//!
//! The other two Batch 53 rules stay proposer-less BY DESIGN, each with a
//! measured reason: `powi-vs-powf` is ALL-TIE 1.00x in release (LLVM
//! instcombine lowers `pow(x, 2.0)` to the same multiply as `powi(2)` — a
//! zero-gain rewrite gets no `--fix` path; the `filter-count-to-any`
//! rationale applied to a tie instead of a loss), and `soa-hot-field` is
//! architectural (a struct layout change, suggestion-only).
//!
//! ## Plan 049 T0.0c (2026-08-30): +1 proposer-covered rule
//!
//! The Bench 043 headroom decomposition showed the shrink rule's eval span
//! ranked IN-POOL with no edit to apply ("unfixable target"). The proposer
//! inserts `v.shrink_to_fit();` before a build fn's tail. It MUTUALLY
//! EXCLUDES with `vec-with-capacity` (which runs as a probe inside it):
//! where the capacity is derivable, `Vec::with_capacity(n)` is the strictly
//! better fix and shrink stays silent; shrink is the fallback for `while`/
//! `loop` push bodies, unknown-iterator for loops, and over-provisioned or
//! guessed `with_capacity` hints. The win is peak memory, not speed — the
//! corpus note stands. Two of the three unfixable targets REMAIN
//! proposer-less by architecture: `dyn-to-enum` (full devirtualization —
//! the `usize-index-to-u32` type-ripple class) and `box-str-immutable`
//! (Vec<String> → Vec<Box<str>> ripples into every push site and changes
//! mutability semantics); they are the honest residual for the trained-arm
//! denominator.
//!
//! ## Honest caveats (Proposal 002 caveats 3-4, restated for this domain)
//!
//! - **`--fix` only applies CONFIRMED visitor matches** (these proposers).
//!   Latent-space suggestions from `--suggest` never auto-apply — the
//!   proposer's detector is the gate, retrieval only reports/prioritizes.
//! - **`fxhashmap` self-gates on an existing `rustc_hash` import** in the
//!   target file (Gate 0 in the proposer): a span-preserving splice can
//!   add a `use` line but cannot add the `rustc-hash` dependency to the
//!   crate's `Cargo.toml`, so the rewrite only fires where the crate path
//!   already resolves. Enforced since the 2026-08-15 game-sdk G5 smoke
//!   caught an E0432 (`bevy_shared.rs`, crate without `rustc-hash`).
//!   Cold-start adoption (first file in a crate + the dep addition) stays
//!   manual.
//! - `static-str` + `bool-to-bitpack` change fn signatures — the rewrite is
//!   span-local (the one fn) but semantically ripples to callers. Since the
//!   2026-08-18 `action_hoare.rs` E0308 lesson BOTH carry a caller gate:
//!   they refuse when the fn ident occurs more than once in the file
//!   ([`count_ident_occurrences`]) — a signature a span-local edit cannot
//!   adapt is a compile break waiting to fire in every `--fix --write`
//!   sweep. `fxhashmap` remains the one body-local type-changing rewrite
//!   without a caller gate (type-flow exposure, not signature exposure;
//!   heavily self-gated on an existing `rustc_hash` import — compile-gate
//!   it if it ever fires wrong); `rust_perf_verify` (cargo test) validates
//!   semantics either way.
//! - Since T6.4 the `clippy_lints` proposers use the same edit-splice emitter
//!   (they were mutation-based + whole-file `ToTokens` re-emission before).

use syn::{Expr, File, Stmt};

pub mod bool_to_bitpack;
pub mod collect_append_to_extend;
pub mod distance_sq_no_sqrt;
pub mod entry_vs_contains;
pub mod fxhashmap;
pub mod hashmap_with_capacity;
pub mod hashset_insert_bool_return;
pub mod shrink_to_fit_after_build;
pub mod static_str;
pub mod vec_macro_zero_fill;
pub mod vec_with_capacity;

// Shared emitter substrate (T6.4): re-exported so the pre-T6.4 import path
// `rust_perf::proposers::{apply_edits, collect_edits, SourceEdit, ...}`
// keeps compiling. The helpers stay `pub(crate)`-visible via the crate root.
pub use crate::edit_emitter::{
    RuleEdit, SourceEdit, apply_edits, delete_stmt_edit, edit, for_each_fn, line_indent,
    span_range, src, swallow_following_space,
};

/// Count word-boundary occurrences of `ident` in `source` (Rust ident
/// charset on both sides; `fn name` self-references count).
///
/// Caller gate for the signature-changing proposers (`static-str`,
/// `bool-to-bitpack`): a fn whose ident appears MORE THAN ONCE in the file
/// (call site, fn-as-value, macro arg, even a comment) is referenced from
/// somewhere a span-local edit cannot reach — rewriting its signature would
/// break the caller (the 2026-08-18 `action_hoare.rs` E0308 lesson: the heal
/// changed `count_satisfied(&[bool])` to `&[u64]` while its live caller still
/// passed `&[bool]`). Refusing is the only mechanically-safe direction.
pub(crate) fn count_ident_occurrences(source: &str, ident: &str) -> usize {
    let bytes = source.as_bytes();
    let mut count = 0;
    let mut from = 0;
    while let Some(rel) = source[from..].find(ident) {
        let at = from + rel;
        let end = at + ident.len();
        let before_ok = at == 0 || !is_ident_byte(bytes[at - 1]);
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            count += 1;
        }
        from = end;
    }
    count
}

/// Direct nested statement blocks of one statement (Plan 097 T3/T4 — the
/// capacity proposers' nested-block descent). `Item` statements (nested
/// fns — visited separately via `for_each_fn`) and macro statements return
/// empty; closure bodies are not block statements here.
pub(crate) fn stmt_nested_blocks(stmt: &Stmt) -> Vec<&syn::Block> {
    match stmt {
        Stmt::Local(local) => match &local.init {
            Some(init) => expr_nested_blocks(&init.expr),
            None => Vec::new(),
        },
        Stmt::Expr(e, _) => expr_nested_blocks(e),
        Stmt::Item(_) | Stmt::Macro(_) => Vec::new(),
    }
}

pub(crate) fn expr_nested_blocks(e: &Expr) -> Vec<&syn::Block> {
    let mut out = Vec::new();
    collect_expr_blocks(e, &mut out);
    out
}

fn collect_expr_blocks<'a>(e: &'a Expr, out: &mut Vec<&'a syn::Block>) {
    match e {
        Expr::Block(b) => out.push(&b.block),
        Expr::Unsafe(u) => out.push(&u.block),
        Expr::If(i) => {
            out.push(&i.then_branch);
            if let Some((_, else_e)) = &i.else_branch {
                collect_expr_blocks(else_e, out);
            }
        }
        Expr::Loop(l) => out.push(&l.body),
        Expr::While(w) => out.push(&w.body),
        Expr::ForLoop(fl) => out.push(&fl.body),
        Expr::Match(m) => {
            for arm in &m.arms {
                collect_expr_blocks(&arm.body, out);
            }
        }
        _ => {}
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Rule ids with compiled-in proposers (`--fix` eligible). A visitor match
/// is the confirmation — retrieval scores only report/prioritize.
pub const FIXABLE_RULES: &[&str] = &[
    "vec-with-capacity",
    "entry-vs-contains-insert",
    "static-str",
    "bool-to-bitpack",
    "fxhashmap",
    "hashset-insert-bool-return",
    "collect-append-to-extend",
    "vec-macro-zero-fill",
    "distance-sq-no-sqrt",
    "hashmap-with-capacity",
    "shrink-to-fit-after-build",
];

/// `FIXABLE_RULES` whose corpus entry carries NO bench spec — the win was
/// never benchmarked (`NotBenchable`). `bench_spec` itself only exists
/// under `rust_perf_bench`, so this list is the default-features-visible
/// half of a derived pin: `corpus::tests::not_benchable_fixable_pin`
/// asserts it equals `FIXABLE_RULES` ∩ no-bench-spec, both directions.
/// Under `--repo-posture` the fix lane declines these (post-mining intake
/// P39, 2026-09-25): the mode's contract is "only what a gate verified",
/// and an un-benchmarked fixable was verified by nobody — the target's
/// clippy cannot speak to our rules, so the conservative mode defaults to
/// declining them rather than exempting them silently. `--apply-
/// notbenchable` is the explicit override. `shrink-to-fit-after-build` is
/// the founding specimen (the 09-25 riir-dapps request-scoped-Vec landing;
/// its corpus entry's own words name LONG-LIVED Vecs as the win regime).
pub const NOT_BENCHABLE_FIXABLE: &[&str] = &["shrink-to-fit-after-build"];

/// Proposer option flags (Issue 015 verify-mode).
///
/// `--fix` defaults to the conservative parse-only gates; `--fix
/// --verify` (rustc backing the revert) passes `untyped_paths = true` to
/// attempt plain-path capacity bounds the usize gate cannot prove.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProposerOpts {
    /// Attempt plain-path capacity bounds whose type syn cannot prove
    /// (consts, associated consts, locals). rustc's E0308 backs the
    /// auto-revert in verify-mode. The scope gate (no later-let
    /// references) stays ON regardless.
    pub untyped_paths: bool,
}

/// Propose a fix for `(rule_id, buggy_code)` by applying the rule's
/// span-preserving edits.
///
/// Returns `Some(fixed_code)` if a pattern matched and was rewritten,
/// `None` otherwise. The output preserves the input's formatting
/// byte-identically outside the matched spans. NOT verified here — the
/// caller runs the L2 pruner (parse check) and, behind `rust_perf_verify`,
/// `cargo test`.
///
/// # Errors
///
/// Returns `None` if the code doesn't parse as a `syn::File` or the rule
/// has no compiled-in proposer (the caller should fall back to `--suggest`
/// output for that rule).
#[must_use]
pub fn propose_fix(rule_id: &str, buggy_code: &str) -> Option<String> {
    let file: File = syn::parse_file(buggy_code).ok()?;
    let edits = collect_edits(rule_id, buggy_code, &file);
    if edits.is_empty() {
        return None;
    }
    let out = apply_edits(buggy_code, edits.iter().map(|e| e.edit.clone()).collect());
    Some(out)
}

/// Collect one rule's edits against the original source. Empty = no match.
#[must_use]
pub fn collect_edits(rule_id: &str, source: &str, file: &File) -> Vec<RuleEdit> {
    collect_edits_opts(rule_id, source, file, ProposerOpts::default())
}

/// [`collect_edits`] with explicit proposer options (Issue 015: the CLI's
/// `--verify` path loosens the vec-with-capacity usize gate — see
/// [`ProposerOpts::untyped_paths`]).
#[must_use]
pub fn collect_edits_opts(
    rule_id: &str,
    source: &str,
    file: &File,
    opts: ProposerOpts,
) -> Vec<RuleEdit> {
    let edits = match rule_id {
        "vec-with-capacity" => vec_with_capacity::collect_edits_opts(source, file, opts),
        "entry-vs-contains-insert" => entry_vs_contains::collect_edits(source, file),
        "static-str" => static_str::collect_edits(source, file),
        "bool-to-bitpack" => bool_to_bitpack::collect_edits(source, file),
        "fxhashmap" => fxhashmap::collect_edits(source, file),
        "hashset-insert-bool-return" => hashset_insert_bool_return::collect_edits(source, file),
        "collect-append-to-extend" => collect_append_to_extend::collect_edits(source, file),
        "vec-macro-zero-fill" => vec_macro_zero_fill::collect_edits(source, file),
        "distance-sq-no-sqrt" => distance_sq_no_sqrt::collect_edits(source, file),
        "hashmap-with-capacity" => hashmap_with_capacity::collect_edits(source, file),
        "shrink-to-fit-after-build" => {
            shrink_to_fit_after_build::collect_edits_opts(source, file, opts)
        }
        _ => Vec::new(),
    };
    edits
        .into_iter()
        .map(|edit| RuleEdit {
            rule_id: rule_id.to_string(),
            edit,
        })
        .collect()
}

/// One-line semantic-shift annotations for the signature-visible fixables
/// (Issue 074 T2) — printed by the `--fix` review as
/// `⚠ semantic change: {note}`. The other eight `FIXABLE_RULES` are
/// semantics-preserving by construction and stay un-annotated (an
/// annotation that fires on most fixes is warning fatigue).
#[must_use]
pub fn context_note(rule_id: &str) -> Option<&'static str> {
    match rule_id {
        // Return type String -> &'static str: callers expecting an owned
        // String must now clone/allocate at the call site (the module
        // caveat, "Signature-visible change").
        "static-str" => Some(
            "signature-visible — returns &'static str (borrowed literal), not an owned String; callers expecting String need .to_string()",
        ),
        // &[bool] param -> packed &[u64]: callers must pass packed bits
        // (pack_bools at the boundary); fires only when the fn has no
        // live in-file caller (the caller gate).
        "bool-to-bitpack" => Some(
            "signature-visible — the &[bool] param becomes packed &[u64]; callers must pack at the boundary (fires only with no live in-file caller)",
        ),
        // NotBenchable + regime-fragile (the riir-dapps worker landing,
        // 2026-09-25: six request-scoped render Vecs got the shrink through
        // the posture-EXEMPT rust_perf lane with zero warning). The win
        // regime is LONG-LIVED Vecs (the corpus entry's own words); on
        // transient/request-scoped Vecs the realloc costs the build path
        // ~µs and the Vec drops before the freed capacity matters.
        "shrink-to-fit-after-build" => Some(
            "regime-fragile (NotBenchable) — the win is PEAK MEMORY on LONG-LIVED Vecs; on request-scoped/transient Vecs the realloc costs the build path and the Vec drops before the freed capacity matters (apply where the Vec outlives the build's caller)",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Squash all whitespace so assertions survive token spacing in
    /// generated fragments.
    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<String>()
    }

    #[test]
    fn unknown_rule_returns_none() {
        assert_eq!(propose_fix("not_a_real_rule", "fn foo() {}"), None);
    }

    /// The fix-review annotation layer (Issue 074 T2 + the riir-dapps worker
    /// landing 2026-09-25): the `NotBenchable` regime-fragile rule MUST carry
    /// its caveat — the posture-exempt `rust_perf` lane applies it into foreign
    /// repos, so the review print is the only warning the consumer gets.
    #[test]
    fn shrink_to_fit_carries_the_regime_caveat() {
        let note = context_note("shrink-to-fit-after-build")
            .expect("the regime-fragile NotBenchable rule must be annotated");
        assert!(note.contains("LONG-LIVED"), "{note}");
        assert!(note.contains("NotBenchable"), "{note}");
        // The two shipped signature-visible annotations stay.
        assert!(context_note("static-str").is_some());
        assert!(context_note("bool-to-bitpack").is_some());
        // And an unlisted fixable stays silent (warning-fatigue guard).
        assert!(context_note("vec-with-capacity").is_none());
    }

    #[test]
    fn unparseable_code_returns_none() {
        assert_eq!(
            propose_fix("vec-with-capacity", "this is not rust {{{"),
            None
        );
    }

    /// The load-bearing invariant of the span-preserving emitter: everything
    /// outside the matched span is byte-identical — original whitespace,
    /// line breaks, and untouched statements survive verbatim.
    #[test]
    fn output_preserves_untouched_bytes() {
        let src = "// header comment\nfn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
                   // fill it\n    for i in 0..n { v.push(i as u32); }\n    let tail = 7;\n    v\n}\n";
        let out = propose_fix("vec-with-capacity", src).expect("pattern matched");
        assert!(out.contains("// header comment\n"), "{out}");
        assert!(
            out.contains("// fill it\n    for i in 0..n { v.push(i as u32); }"),
            "{out}"
        );
        assert!(out.contains("let tail = 7;"), "{out}");
        assert!(out.contains("let mut v = Vec::with_capacity(n);"), "{out}");
        // Exactly one line differs in structure: the init line.
        let orig_lines: Vec<&str> = src.lines().collect();
        let out_lines: Vec<&str> = out.lines().collect();
        assert_eq!(
            orig_lines.len(),
            out_lines.len(),
            "line count changed: {out}"
        );
        let diffs = orig_lines
            .iter()
            .zip(&out_lines)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(diffs, 1, "expected exactly one changed line: {out}");
    }

    /// End-to-end: each proposer-covered corpus fixture fires its proposer,
    /// the output carries the fixed-form marker, AND the output parses (L2
    /// pruner — the same validation `RustPerfDomain::validate` runs).
    #[test]
    fn corpus_fixtures_fire_and_parse() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "vec-with-capacity",
                "fn build(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    \
                 for i in 0..n { v.push(i as u32); }\n    v\n}",
                "with_capacity(n)",
            ),
            (
                "entry-vs-contains-insert",
                "use std::collections::HashMap;\nfn insert_if_absent(map: &mut \
                 HashMap<u32, u32>, k: u32, v: u32) {\n    if !map.contains_key(&k) {\n        \
                 map.insert(k, v);\n    }\n}",
                "entry(k).or_insert(v)",
            ),
            (
                "static-str",
                "fn label() -> String { \"ready\".to_string() }",
                "&'static str",
            ),
            (
                "bool-to-bitpack",
                "fn count_true(flags: &[bool]) -> usize {\n    \
                 flags.iter().filter(|&&b| b).count()\n}",
                "count_ones() as usize",
            ),
            (
                "fxhashmap",
                "use rustc_hash::FxHashSet;\nuse std::collections::HashMap;\nfn lookup(map: &HashMap<u32, u32>, k: u32) \
                 -> Option<u32> {\n    map.get(&k).copied()\n}",
                "FxHashMap",
            ),
            (
                "hashset-insert-bool-return",
                "use std::collections::HashSet;\nfn register_seen(seen: &mut \
                 HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    \
                 for &x in items {\n        if !seen.contains(&x) {\n            \
                 seen.insert(x);\n            inserted += 1;\n        }\n    }\n    inserted\n}",
                "if seen.insert(x) {",
            ),
            (
                "collect-append-to-extend",
                "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in \
                 chunks {\n        let mut staged: Vec<u32> = c.iter().copied().collect();\n        \
                 target.append(&mut staged);\n    }\n}",
                "target.extend(c.iter().copied())",
            ),
            (
                "vec-macro-zero-fill",
                "fn zero_buf(n: usize) -> Vec<u8> {\n    let mut v = Vec::new();\n    \
                 v.resize(n, 0);\n    v\n}",
                "vec![0; n]",
            ),
        ];
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        for (rule, buggy, marker) in cases {
            let out = propose_fix(rule, buggy)
                .unwrap_or_else(|| panic!("proposer did not fire for corpus fixture `{rule}`"));
            assert!(
                squash(&out).contains(&squash(marker)),
                "`{rule}` output missing marker `{marker}`: {out}"
            );
            let verdict = pruner.validate(&out, rule);
            assert!(verdict.parses, "`{rule}` output does not parse: {out}");
        }
    }
}
