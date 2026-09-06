//! Issue 072 T5/T6 — the M4 composite exactness ladder + the M5 generation-v2
//! fixture set (`.plans/092`; riir-train Research 450 §4.1 M4/M5).
//!
//! # M4 — amplification as an exactness schedule
//!
//! A k-composite fixture's gold IS the composition of the k atomic golds —
//! exact, no extra oracle. The atoms are NOT authored here: they come from
//! the clippy seed corpus itself (construction-first — the corpus entries'
//! `buggy_code` values are already probe-verified to fire their lint, and
//! `floor_corpus_integrity` compiles them). An atom qualifies when its
//! `buggy_code` parses standalone as a file (complete items only — composable
//! by wrapping in a module, which keeps item names collision-free and cannot
//! create cross-atom interference by construction).
//!
//! The ladder: k ∈ {1, 2, 3, 4}, deterministic stride-window selection over
//! the sorted lint set (no RNG — the corpus house rule), so the whole
//! generation is byte-stable and re-derivable from the corpus alone.
//!
//! # M5 — the generationed bench ratchet
//!
//! `generation_v2()` is the admitted fixture set: the specs, minus the ids
//! the freeze run EXCLUDED (interference composites whose real-clippy labels
//! disagreed with the union-of-atoms gold — construction proposes, the
//! oracle disposes), each carrying its oracle-verified frozen label. The
//! labels/exclusions land as data AFTER the freeze e2e
//! (`tests/generation_v2_freeze.rs`, `#[ignore]`d — spawns real clippy).
//!
//! The ratchet itself (saturation detector, the non-vacuity admission gate
//! and per-generation history series) lives in `report.rs` and the CLI; this
//! module only owns the generation.

use std::collections::{BTreeMap, BTreeSet};

use super::BenchFixture;

// ── the atom factory (M4, construction-first) ───────────────────────────

/// One atomic unit: a corpus span carrying exactly one vocabulary lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    /// The vocabulary lint the atom fires (the atom's whole gold).
    pub lint: String,
    /// The corpus `buggy_code` (parses standalone as a file).
    pub code: String,
}

/// The atom table: per vocabulary lint, the FIRST corpus entry (min
/// `buggy_code` per lint — the sorted-(lint, code) first) whose `buggy_code`
/// parses standalone.
///
/// Atoms that fail freeze verification are removed from [`FROZEN_LABELS`]'s
/// coverage instead of here — the factory stays a pure function of the
/// corpus, and the freeze record documents any exclusion.
#[must_use]
pub fn atoms() -> Vec<Atom> {
    let vocab = super::vocabulary();
    let mut best: BTreeMap<String, String> = BTreeMap::new();
    for e in crate::build_seed_corpus().entries {
        if !vocab.contains(e.lint_name.as_str()) || syn::parse_file(&e.buggy_code).is_err() {
            continue;
        }
        match best.get(&e.lint_name) {
            Some(existing) if *existing <= e.buggy_code => {}
            _ => {
                best.insert(e.lint_name.clone(), e.buggy_code.clone());
            }
        }
    }
    best.into_iter()
        .map(|(lint, code)| Atom { lint, code })
        .collect()
}

// ── the deterministic k-ladder (M4) ─────────────────────────────────────

/// The k values the ladder spans (k=1 is the atom-solve-rate baseline).
pub const LADDER_KS: [usize; 4] = [1, 2, 3, 4];

/// Composite counts per k (the k=1 rung is every atom). Sized so a full
/// freeze stays a few minutes of real clippy: n + 16 + 12 + 8 fixtures.
#[must_use]
pub fn count_for_k(k: usize, n_atoms: usize) -> usize {
    match k {
        1 => n_atoms,
        2 => 16.min(n_atoms / 2),
        3 => 12.min(n_atoms / 3),
        4 => 8.min(n_atoms / 4),
        _ => 0,
    }
}

/// The lints of composite `i` at width `k` — a stride window over the sorted
/// atom set (wrapping; distinct because `k <= n_atoms`).
#[must_use]
pub fn lints_for_composite(k: usize, i: usize, lints: &[String]) -> Vec<String> {
    let n = lints.len();
    (0..k).map(|j| lints[(i * k + j) % n].clone()).collect()
}

fn mod_wrap(index: usize, code: &str) -> String {
    format!("mod gen_{index} {{\n{code}\n}}\n")
}

/// One rung of the ladder, fully determined by the atom table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gen2Spec {
    pub id: String,
    pub k: usize,
    /// The union-of-atoms gold (expected, NOT yet oracle-verified).
    pub expect_lints: Vec<String>,
    pub code: String,
}

/// The deterministic generation: atoms (k=1) + stride-window composites
/// (k=2..4). Byte-stable — pinned by test.
///
/// # Panics
///
/// Only if the atom table loses a lint between the map build and the lookup
/// (cannot happen — both derive from the same `atoms()` call; the `expect`
/// documents the invariant instead of silently skipping).
#[must_use]
pub fn specs() -> Vec<Gen2Spec> {
    let atom_list = atoms();
    let lints: Vec<String> = atom_list.iter().map(|a| a.lint.clone()).collect();
    let by_lint: BTreeMap<&str, &Atom> =
        atom_list.iter().map(|a| (a.lint.as_str(), a)).collect();
    let n = lints.len();
    let mut out = Vec::new();
    for &k in &LADDER_KS {
        let count = count_for_k(k, n);
        for i in 0..count {
            let picks = lints_for_composite(k, i, &lints);
            let id = format!("gen2-k{k}-{}", picks.join("+"));
            let mut code = String::new();
            for (index, lint) in picks.iter().enumerate() {
                let atom = by_lint.get(lint.as_str()).expect("lint drawn from the atom table");
                code.push_str(&mod_wrap(index, &atom.code));
            }
            let expect_lints: BTreeSet<String> = picks.into_iter().collect();
            out.push(Gen2Spec {
                id,
                k,
                expect_lints: expect_lints.into_iter().collect(),
                code,
            });
        }
    }
    out
}

// ── the freeze record (M5 — landed by the freeze e2e, 2026-09-06) ──────
//
// Freeze verdict (`tests/generation_v2_freeze.rs --ignored`, release,
// 32 s): 38 atoms (2 bad — `needless_match` also fires `manual_map`,
// `uninlined_format_args` also fires `useless_format`, so their k=1 specs
// and every composite containing them are excluded; 8 exclusions total,
// zero independent interference), 58 verified composites, union identity
// 58/58 against real clippy. Solve rates: 100.0% at EVERY k (36/36, 28/28,
// 30/30, 24/24) — the monotone-decay gate passes vacuously (flat). This is
// pre-registered branch (B): the k-ladder over the corpus's own shapes
// cannot be harder than the proposers trained on that corpus, so the M5
// non-vacuity admission gate refuses generation 2 (v2 100% > v1 96.2% =
// not harder). The record stays landed: the drift gate lives, and the
// difficulty-axis reopen (persist-shape stratification / real-repo
// interference spans) re-runs this exact freeze.
pub static FROZEN_LABELS: &[(&str, &[&str])] = &[
    ("gen2-k1-byte_char_slices", &["byte_char_slices"]),
    ("gen2-k1-collapsible_else_if", &["collapsible_else_if"]),
    ("gen2-k1-collapsible_if", &["collapsible_if"]),
    ("gen2-k1-deref_addrof", &["deref_addrof"]),
    ("gen2-k1-double_parens", &["double_parens"]),
    ("gen2-k1-filter_next", &["filter_next"]),
    ("gen2-k1-identity_op", &["identity_op"]),
    ("gen2-k1-items_after_statements", &["items_after_statements"]),
    ("gen2-k1-iter_next_loop", &["iter_next_loop"]),
    ("gen2-k1-iter_nth_zero", &["iter_nth_zero"]),
    ("gen2-k1-manual_let_else", &["manual_let_else"]),
    ("gen2-k1-manual_map", &["manual_map"]),
    ("gen2-k1-manual_slice_fill", &["manual_slice_fill"]),
    ("gen2-k1-manual_unwrap_or", &["manual_unwrap_or"]),
    ("gen2-k1-map_unwrap_or", &["map_unwrap_or"]),
    ("gen2-k1-match_bool", &["match_bool"]),
    ("gen2-k1-needless_bool", &["needless_bool"]),
    ("gen2-k1-needless_bool_assign", &["needless_bool_assign"]),
    ("gen2-k1-needless_lifetimes", &["needless_lifetimes"]),
    ("gen2-k1-needless_range_loop", &["needless_range_loop"]),
    ("gen2-k1-needless_return", &["needless_return"]),
    ("gen2-k1-question_mark", &["question_mark"]),
    ("gen2-k1-range_plus_one", &["range_plus_one"]),
    ("gen2-k1-redundant_field_names", &["redundant_field_names"]),
    ("gen2-k1-redundant_pattern_matching", &["redundant_pattern_matching"]),
    ("gen2-k1-redundant_static_lifetimes", &["redundant_static_lifetimes"]),
    ("gen2-k1-semicolon_if_nothing_returned", &["semicolon_if_nothing_returned"]),
    ("gen2-k1-single_match", &["single_match"]),
    ("gen2-k1-single_match_else", &["single_match_else"]),
    ("gen2-k1-unnecessary_lazy_evaluations", &["unnecessary_lazy_evaluations"]),
    ("gen2-k1-unnecessary_sort_by", &["unnecessary_sort_by"]),
    ("gen2-k1-unnecessary_unwrap", &["unnecessary_unwrap"]),
    ("gen2-k1-unused_unit", &["unused_unit"]),
    ("gen2-k1-use_self", &["use_self"]),
    ("gen2-k1-useless_format", &["useless_format"]),
    ("gen2-k1-wildcard_in_or_patterns", &["wildcard_in_or_patterns"]),
    ("gen2-k2-byte_char_slices+collapsible_else_if", &["byte_char_slices", "collapsible_else_if"]),
    ("gen2-k2-collapsible_if+deref_addrof", &["collapsible_if", "deref_addrof"]),
    ("gen2-k2-double_parens+filter_next", &["double_parens", "filter_next"]),
    ("gen2-k2-identity_op+items_after_statements", &["identity_op", "items_after_statements"]),
    ("gen2-k2-iter_next_loop+iter_nth_zero", &["iter_next_loop", "iter_nth_zero"]),
    ("gen2-k2-manual_let_else+manual_map", &["manual_let_else", "manual_map"]),
    ("gen2-k2-manual_slice_fill+manual_unwrap_or", &["manual_slice_fill", "manual_unwrap_or"]),
    ("gen2-k2-map_unwrap_or+match_bool", &["map_unwrap_or", "match_bool"]),
    ("gen2-k2-needless_bool+needless_bool_assign", &["needless_bool", "needless_bool_assign"]),
    ("gen2-k2-needless_range_loop+needless_return", &["needless_range_loop", "needless_return"]),
    ("gen2-k2-question_mark+range_plus_one", &["question_mark", "range_plus_one"]),
    ("gen2-k2-redundant_field_names+redundant_pattern_matching", &["redundant_field_names", "redundant_pattern_matching"]),
    ("gen2-k2-redundant_static_lifetimes+semicolon_if_nothing_returned", &["redundant_static_lifetimes", "semicolon_if_nothing_returned"]),
    ("gen2-k2-single_match+single_match_else", &["single_match", "single_match_else"]),
    ("gen2-k3-byte_char_slices+collapsible_else_if+collapsible_if", &["byte_char_slices", "collapsible_else_if", "collapsible_if"]),
    ("gen2-k3-deref_addrof+double_parens+filter_next", &["deref_addrof", "double_parens", "filter_next"]),
    ("gen2-k3-identity_op+items_after_statements+iter_next_loop", &["identity_op", "items_after_statements", "iter_next_loop"]),
    ("gen2-k3-iter_nth_zero+manual_let_else+manual_map", &["iter_nth_zero", "manual_let_else", "manual_map"]),
    ("gen2-k3-manual_slice_fill+manual_unwrap_or+map_unwrap_or", &["manual_slice_fill", "manual_unwrap_or", "map_unwrap_or"]),
    ("gen2-k3-match_bool+needless_bool+needless_bool_assign", &["match_bool", "needless_bool", "needless_bool_assign"]),
    ("gen2-k3-needless_return+question_mark+range_plus_one", &["needless_return", "question_mark", "range_plus_one"]),
    ("gen2-k3-redundant_field_names+redundant_pattern_matching+redundant_static_lifetimes", &["redundant_field_names", "redundant_pattern_matching", "redundant_static_lifetimes"]),
    ("gen2-k3-semicolon_if_nothing_returned+single_match+single_match_else", &["semicolon_if_nothing_returned", "single_match", "single_match_else"]),
    ("gen2-k3-unnecessary_unwrap+unused_unit+use_self", &["unnecessary_unwrap", "unused_unit", "use_self"]),
    ("gen2-k4-byte_char_slices+collapsible_else_if+collapsible_if+deref_addrof", &["byte_char_slices", "collapsible_else_if", "collapsible_if", "deref_addrof"]),
    ("gen2-k4-double_parens+filter_next+identity_op+items_after_statements", &["double_parens", "filter_next", "identity_op", "items_after_statements"]),
    ("gen2-k4-iter_next_loop+iter_nth_zero+manual_let_else+manual_map", &["iter_next_loop", "iter_nth_zero", "manual_let_else", "manual_map"]),
    ("gen2-k4-manual_slice_fill+manual_unwrap_or+map_unwrap_or+match_bool", &["manual_slice_fill", "manual_unwrap_or", "map_unwrap_or", "match_bool"]),
    ("gen2-k4-needless_range_loop+needless_return+question_mark+range_plus_one", &["needless_range_loop", "needless_return", "question_mark", "range_plus_one"]),
    ("gen2-k4-redundant_field_names+redundant_pattern_matching+redundant_static_lifetimes+semicolon_if_nothing_returned", &["redundant_field_names", "redundant_pattern_matching", "redundant_static_lifetimes", "semicolon_if_nothing_returned"]),
];

/// Spec ids the freeze run EXCLUDED (real-clippy labels ≠ union gold): the
/// 2 bad atoms plus the 6 composites whose membership in them cascades.
/// Zero independent interference was measured.
pub static EXCLUDED_IDS: &[&str] = &[
    "gen2-k1-needless_match",
    "gen2-k1-uninlined_format_args",
    "gen2-k2-needless_lifetimes+needless_match",
    "gen2-k2-uninlined_format_args+unnecessary_lazy_evaluations",
    "gen2-k3-needless_lifetimes+needless_match+needless_range_loop",
    "gen2-k3-uninlined_format_args+unnecessary_lazy_evaluations+unnecessary_sort_by",
    "gen2-k4-needless_bool+needless_bool_assign+needless_lifetimes+needless_match",
    "gen2-k4-single_match+single_match_else+uninlined_format_args+unnecessary_lazy_evaluations",
];

/// Freeze readiness: Ok(admitted count) when every spec is labeled or
/// excluded, Err(reason) while the freeze e2e has not landed its record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels (or which
/// exclusion rows no longer match the corpus-derived specs).
pub fn v2_freeze_status() -> Result<usize, String> {
    let all = specs();
    let excluded: BTreeSet<&str> = EXCLUDED_IDS.iter().copied().collect();
    let labeled: BTreeSet<&str> = FROZEN_LABELS.iter().map(|(id, _)| *id).collect();
    let missing: Vec<&str> = all
        .iter()
        .filter(|s| !excluded.contains(s.id.as_str()) && !labeled.contains(s.id.as_str()))
        .map(|s| s.id.as_str())
        .collect();
    let orphan_excluded: Vec<&str> = excluded
        .iter()
        .filter(|id| !all.iter().any(|s| &s.id == *id))
        .copied()
        .collect();
    if !orphan_excluded.is_empty() {
        return Err(format!(
            "EXCLUDED_IDS names ids the spec builder no longer produces (the corpus moved): {orphan_excluded:?} — re-run the freeze e2e"
        ));
    }
    if missing.is_empty() {
        Ok(labeled.len())
    } else {
        Err(format!(
            "generation v2 is not frozen: {} spec(s) lack oracle labels — run \
             `cargo test --features score_bench --test generation_v2_freeze -- --ignored \
             --test-threads=1` and land FROZEN_LABELS/EXCLUDED_IDS",
            missing.len()
        ))
    }
}

/// The admitted generation-v2 fixture set (the M5 ratchet's bench input).
///
/// Leaks the fixture strings once per call by design — `BenchFixture` is
/// `&'static`-shaped and this runs once per `--generation 2` invocation.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v2_freeze_status`] first
/// and print its error instead.
#[must_use]
pub fn generation_v2() -> Vec<BenchFixture> {
    let excluded: BTreeSet<&str> = EXCLUDED_IDS.iter().copied().collect();
    let labels: BTreeMap<&str, &[&str]> = FROZEN_LABELS.iter().map(|(id, l)| (*id, *l)).collect();
    specs()
        .into_iter()
        .filter(|s| !excluded.contains(s.id.as_str()))
        .map(|s| {
            let expect: &[&str] = labels
                .get(s.id.as_str())
                .copied()
                .unwrap_or_else(|| panic!("spec {} frozen but unlabeled — v2_freeze_status hole", s.id));
            BenchFixture {
                id: Box::leak(s.id.clone().into_boxed_str()),
                provenance: Box::leak(
                    format!("construction:gen2:k{}:{}", s.k, s.expect_lints.join("+"))
                        .into_boxed_str(),
                ),
                code: Box::leak(s.code.into_boxed_str()),
                expect_lints: Box::leak(expect.to_vec().into_boxed_slice()),
                clean: false,
            }
        })
        .collect()
}

/// The ladder k of a fixture id (`gen2-k3-...` → 3); `None` for non-gen2 ids.
#[must_use]
pub fn k_of(id: &str) -> Option<usize> {
    let rest = id.strip_prefix("gen2-k")?;
    let k: String = rest.chars().take_while(char::is_ascii_digit).collect();
    k.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom_lints() -> BTreeSet<String> {
        atoms().into_iter().map(|a| a.lint).collect()
    }

    #[test]
    fn atom_table_is_the_parse_standalone_vocabulary_slice() {
        let lints = atom_lints();
        let vocab = super::super::vocabulary();
        // The atom table is the parse-standalone slice of the corpus: every
        // atom is a vocabulary lint, and the table is non-trivial. The freeze
        // e2e verifies each atom individually — this gate pins the factory's
        // invariants, not a count the corpus may legitimately change.
        assert!(!lints.is_empty(), "atom table empty — factory broken");
        for a in atoms() {
            assert!(vocab.contains(a.lint.as_str()), "atom {} not in vocabulary", a.lint);
            assert!(
                syn::parse_file(&a.code).is_ok(),
                "atom {} does not parse standalone",
                a.lint
            );
            assert!(
                !a.code.contains("mod gen_"),
                "atom {} carries the composite wrapper",
                a.lint
            );
        }
    }

    #[test]
    fn specs_are_deterministic_and_ladder_shaped() {
        let s1 = specs();
        let s2 = specs();
        assert_eq!(s1, s2, "specs must be byte-stable");
        let n = atoms().len();
        let k1 = s1.iter().filter(|s| s.k == 1).count();
        assert_eq!(k1, count_for_k(1, n));
        for &k in &LADDER_KS[1..] {
            for s in s1.iter().filter(|s| s.k == k) {
                assert_eq!(s.expect_lints.len(), k, "k={k} spec {} gold size", s.id);
                assert_eq!(
                    s.id,
                    format!("gen2-k{k}-{}", s.expect_lints.join("+")),
                    "id encodes k + gold"
                );
            }
        }
    }

    #[test]
    fn composite_code_wraps_every_atom_exactly_once() {
        let by_lint: BTreeMap<String, String> =
            atoms().into_iter().map(|a| (a.lint, a.code)).collect();
        for s in specs().iter().filter(|s| s.k >= 2) {
            for lint in &s.expect_lints {
                let atom_code = &by_lint[lint];
                assert!(
                    s.code.contains(atom_code.as_str()),
                    "composite {} lost atom {lint}",
                    s.id
                );
            }
            let mods = s.code.matches("mod gen_").count();
            assert_eq!(mods, s.k, "composite {} wraps k atoms", s.id);
        }
    }

    #[test]
    fn ids_are_unique_across_the_generation() {
        let s = specs();
        let ids: BTreeSet<&str> = s.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids.len(), s.len(), "duplicate generation ids");
    }

    #[test]
    fn k_of_parses_gen2_ids_only() {
        assert_eq!(k_of("gen2-k3-a+b+c"), Some(3));
        assert_eq!(k_of("gen2-k1-identity_op"), Some(1));
        assert_eq!(k_of("composite-return_plus_identity_op"), None);
        assert_eq!(k_of("gen2-"), None);
    }

    #[test]
    fn freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            // Pre-freeze: every spec is missing → Err (a silently-empty
            // generation must never reach the bench).
            assert!(
                v2_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            // Post-freeze: the always-on coverage gate — labels ∪ excluded
            // covers every spec, and each label equals the spec's union gold.
            for s in specs() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS
                        .iter()
                        .find(|(id, _)| *id == s.id)
                        .map_or(&[] as &[&str], |(_, l)| *l);
                    let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
                    let got: BTreeSet<&str> = frozen.iter().copied().collect();
                    assert_eq!(want, got, "frozen label drift on {}", s.id);
                }
            }
        }
    }
}
