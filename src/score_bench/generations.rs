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

/// One rung of a generation ladder, fully determined by its atom table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenSpec {
    pub id: String,
    pub k: usize,
    /// The union-of-atoms gold (expected, NOT yet oracle-verified).
    pub expect_lints: Vec<String>,
    pub code: String,
}

/// The shared ladder builder behind [`specs`] (v2) and [`specs_v3`] (gen 3):
/// atoms (k=1) + stride-window composites (k=2..4) over a sorted atom-id
/// list, ids `{prefix}-k{k}-{ids joined +}`. Byte-stable — pinned by test.
///
/// # Panics
///
/// Only if the atom table loses an id between the list build and the lookup
/// (cannot happen — both derive from the same atom list; the `expect`
/// documents the invariant instead of silently skipping).
#[must_use]
fn specs_over_atoms(prefix: &str, atom_list: &[V3Atom]) -> Vec<GenSpec> {
    let ids: Vec<String> = atom_list.iter().map(|a| a.id.clone()).collect();
    let by_id: BTreeMap<&str, &V3Atom> =
        atom_list.iter().map(|a| (a.id.as_str(), a)).collect();
    let n = ids.len();
    let mut out = Vec::new();
    for &k in &LADDER_KS {
        let count = count_for_k(k, n);
        for i in 0..count {
            let picks = lints_for_composite(k, i, &ids);
            let id = format!("{prefix}-k{k}-{}", picks.join("+"));
            let mut code = String::new();
            for (index, atom_id) in picks.iter().enumerate() {
                let atom = by_id.get(atom_id.as_str()).expect("atom id drawn from the atom table");
                code.push_str(&mod_wrap(index, &atom.code));
            }
            let expect_lints: BTreeSet<String> = picks
                .iter()
                .flat_map(|a| by_id[a.as_str()].expect.iter().cloned())
                .collect();
            out.push(GenSpec {
                id,
                k,
                expect_lints: expect_lints.into_iter().collect(),
                code,
            });
        }
    }
    out
}

/// The generation-2 spec ladder (corpus atoms keyed by lint — byte-identical
/// to the pre-generalization builder).
#[must_use]
pub fn specs() -> Vec<GenSpec> {
    let atom_list: Vec<V3Atom> = atoms()
        .into_iter()
        .map(|a| V3Atom { id: a.lint.clone(), expect: vec![a.lint], code: a.code })
        .collect();
    specs_over_atoms("gen2", &atom_list)
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

/// Freeze readiness for one generation: Ok(admitted count) when every spec
/// is labeled or excluded, Err(reason) while the generation's freeze e2e has
/// not landed its record.
fn freeze_status_for(
    all: &[GenSpec],
    gen: u8,
    test_name: &str,
    labels: &[(&'static str, &'static [&'static str])],
    excluded: &[&'static str],
) -> Result<usize, String> {
    let excluded: BTreeSet<&str> = excluded.iter().copied().collect();
    let labeled: BTreeSet<&str> = labels.iter().map(|(id, _)| *id).collect();
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
            "generation v{gen} is not frozen: {} spec(s) lack oracle labels — run \
             `cargo test --features score_bench --test {test_name} -- --ignored \
             --test-threads=1` and land the freeze record",
            missing.len()
        ))
    }
}

/// The admitted fixture set for one generation (the M5 ratchet's bench
/// input). Leaks the fixture strings once per call by design —
/// `BenchFixture` is `&'static`-shaped and this runs once per
/// `--generation <N>` invocation.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check the generation's
/// `*_freeze_status` first and print its error instead.
fn generation_fixtures(
    all: Vec<GenSpec>,
    gen: u8,
    labels: &[(&'static str, &'static [&'static str])],
    excluded: &[&'static str],
) -> Vec<BenchFixture> {
    let excluded: BTreeSet<&str> = excluded.iter().copied().collect();
    let label_map: BTreeMap<&str, &[&str]> = labels.iter().map(|(id, l)| (*id, *l)).collect();
    all.into_iter()
        .filter(|s| !excluded.contains(s.id.as_str()))
        .map(|s| {
            let expect: &[&str] = label_map
                .get(s.id.as_str())
                .copied()
                .unwrap_or_else(|| panic!("spec {} frozen but unlabeled — freeze_status hole", s.id));
            BenchFixture {
                id: Box::leak(s.id.clone().into_boxed_str()),
                provenance: Box::leak(
                    format!("construction:gen{gen}:k{}:{}", s.k, s.expect_lints.join("+"))
                        .into_boxed_str(),
                ),
                code: Box::leak(s.code.into_boxed_str()),
                expect_lints: Box::leak(expect.to_vec().into_boxed_slice()),
                clean: false,
            }
        })
        .collect()
}

/// Freeze readiness: Ok(admitted count) when every spec is labeled or
/// excluded, Err(reason) while the freeze e2e has not landed its record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels (or which
/// exclusion rows no longer match the corpus-derived specs).
pub fn v2_freeze_status() -> Result<usize, String> {
    freeze_status_for(&specs(), 2, "generation_v2_freeze", FROZEN_LABELS, EXCLUDED_IDS)
}

/// The admitted generation-v2 fixture set (the M5 ratchet's bench input).
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v2_freeze_status`] first
/// and print its error instead.
#[must_use]
pub fn generation_v2() -> Vec<BenchFixture> {
    generation_fixtures(specs(), 2, FROZEN_LABELS, EXCLUDED_IDS)
}

/// The ladder k of a generation fixture id (`gen2-k3-…` → 3,
/// `gen3-k2-…` → 2); `None` for non-generation ids.
#[must_use]
pub fn k_of(id: &str) -> Option<usize> {
    let rest = id.strip_prefix("gen")?;
    let (_gen, rest) = rest.split_once('-')?;
    let rest = rest.strip_prefix('k')?;
    let k: String = rest.chars().take_while(char::is_ascii_digit).collect();
    k.parse().ok()
}

// ── generation 3 — persist-shape stratification (Bench 069 reopen #1) ──
//
// The corpus-atom ladder (generation 2) measured 100% at every k: shapes
// the corpus teaches are shapes the proposers solve. Generation 3
// stratifies the atom table with AUTHORED persist-shape atoms — real-code
// shapes where a vocabulary lint FIRES and the heal lane DELIBERATELY stays
// silent, each a documented parse-only divergence class with the record
// that measured the silence. These give the bench measurement headroom the
// saturated corpus ladder lacks: when a proposer extension ever lands
// (macro-arg descent past a nested macro, the named-arg rename rewrite, a
// further array-literal relaxation), the generation's rate MOVES and the
// ratchet sees it.
//
// Pre-registered at authoring (`.benchmarks/070`), BEFORE the freeze run:
// the v2 M4 k-decay gate does NOT carry to gen 3 — its difficulty axis is
// per-atom strata, not k-composition, and the prefix-window geometry makes
// strict monotonicity a sampling artifact. The binding checks are the
// stratification gate (every persist atom resolves 0) and the M5 admission
// gate (aggregate rate strictly below v1's latest).
//
// The stratification gate already caught two stale atoms on the FIRST
// freeze run (the loop working as designed):
// - a named-arg-with-rename shape (`println!("{iters}", iters = ITERS)`)
//   RESOLVED — the P3 rename rewrite closed the class after the bench_337
//   record was written; the atom was replaced.
// - a second-level macro-nesting shape now fires clippy 1.98's NEW
//   `format_in_format_args` instead of `uninlined_format_args` — the class
//   moved off-vocabulary (labels move between toolchains, the oracle's
//   own caveat); the atom was dropped.

/// One authored persist-shape atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistAtom {
    /// Stable id (namespaced `persist_*` — disjoint from corpus lint ids).
    pub id: &'static str,
    /// The divergence class + the record that measured the silence.
    pub note: &'static str,
    /// The shape (parses standalone as a file; compiles in the stub crate).
    pub code: &'static str,
    /// The vocabulary lints the freeze oracle must confirm (the atom's gold).
    pub expect: &'static [&'static str],
}

pub static PERSIST_ATOMS: &[PersistAtom] = &[
    PersistAtom {
        id: "persist_array_default",
        note: "array-literal map_or guard: real clippy's own MachineApplicable rewrite E0308s in a plain-let position (map_or's U unifies from the array literal first), so the proposer refuses — the slice_pinned relaxation (P2, 2026-08-28) fires only when a `&[T]` annotation forces the type, which this plain let deliberately lacks (measured at forward step.rs:196)",
        code: r"fn persist_array_default(o: Option<&str>) -> Vec<u8> {
    let bytes = o.map(|s| s.as_bytes()).unwrap_or(&[]);
    bytes.to_vec()
}",
        expect: &["map_unwrap_or"],
    },
    PersistAtom {
        id: "persist_comment_arm",
        note: "comment-guarded match rewrite: real clippy fires single_match_else on the corpus's own Some/block-fallback shape, but the interior comment would be displaced by the if-let rewrite and the drops_interior_comment guard refuses — the healer is SAFER than clippy there (guards from Issue 027/029; measured at transposition.rs:67)",
        code: r"fn score2(g: u8) -> u8 {
    g * 2
}

fn persist_comment_arm(kind: Option<u8>) -> u8 {
    match kind {
        // The fallback block is the documented shape.
        Some(k) => score2(k),
        _ => {
            let fallback = 2;
            score2(fallback)
        }
    }
}",
        expect: &["single_match_else"],
    },
    PersistAtom {
        id: "persist_comment_hoist",
        note: "comment-guarded item hoist: real clippy fires items_after_statements on the fn-after-statement shape, but hoisting the item above the statement would displace the interior comment and the leading-gap guard refuses (the Batch-90 comment guards; corpus note cites the class)",
        code: r"fn persist_comment_hoist() -> i32 {
    let x = 1;
    // The helper stays below this note.
    fn h(v: i32) -> i32 {
        v * 2
    }
    h(x)
}",
        expect: &["items_after_statements"],
    },
    PersistAtom {
        id: "persist_useless_format",
        note: "the type-boundary terminal: real clippy fires useless_format on the String-typed named-capture form, but the terminal fix (clone vs to_string) needs the arg's TYPE — the single-positional proposer form deliberately does not fire, and firing would trade the persist for a created implicit_clone (measured on the composite-range_loop_plus_format_args fixture, run #3 triage)",
        code: r#"fn persist_useless_format() {
    let elem: String = "payload".to_string();
    let s: String = format!("{elem}");
    println!("{s}");
}"#,
        expect: &["useless_format"],
    },
];

/// One generation-3 atom: a corpus-derived solvable atom (id = its lint) or
/// an authored persist-shape atom (id = `persist_*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V3Atom {
    pub id: String,
    /// The atom's gold (union member of every composite containing it).
    pub expect: Vec<String>,
    pub code: String,
}

/// The generation-3 atom table: the five persist atoms FIRST (the stride
/// windows cover the list head at k≥2, so declaring the stratification up
/// front puts a persist shape in every composite rung), then the corpus
/// atoms in their sorted-lint order. Order is load-bearing and pinned.
#[must_use]
pub fn v3_atoms() -> Vec<V3Atom> {
    let mut out: Vec<V3Atom> = Vec::with_capacity(PERSIST_ATOMS.len() + 40);
    out.extend(PERSIST_ATOMS.iter().map(|p| V3Atom {
        id: p.id.to_string(),
        expect: p.expect.iter().map(ToString::to_string).collect(),
        code: p.code.to_string(),
    }));
    out.extend(atoms().into_iter().map(|a| V3Atom {
        id: a.lint.clone(),
        expect: vec![a.lint],
        code: a.code,
    }));
    out
}

/// The generation-3 spec ladder: persist-stratified atoms (k=1) + the same
/// stride-window composites (k=2..4). Byte-stable — pinned by test.
#[must_use]
pub fn specs_v3() -> Vec<GenSpec> {
    let atom_list = v3_atoms();
    specs_over_atoms("gen3", &atom_list)
}

/// Spec ids the generation-3 freeze run EXCLUDED (real-clippy labels ≠ the
/// union-of-atom gold) — landed by the freeze e2e.
// Landed by tests/generation_v3_freeze.rs --ignored (2026-09-06): the 2 known
// corpus double-fire atoms (needless_match, uninlined_format_args) + their 4
// cascade composites. Zero independent interference measured.
pub static EXCLUDED_IDS_V3: &[&str] = &[
    "gen3-k1-needless_match",
    "gen3-k1-uninlined_format_args",
    "gen3-k2-needless_lifetimes+needless_match",
    "gen3-k3-needless_bool_assign+needless_lifetimes+needless_match",
    "gen3-k3-single_match_else+uninlined_format_args+unnecessary_lazy_evaluations",
    "gen3-k4-needless_bool+needless_bool_assign+needless_lifetimes+needless_match",
];

/// The generation-3 frozen labels (spec id → vocabulary lints real clippy
/// fired at freeze time) — landed by the freeze e2e.
// Landed by tests/generation_v3_freeze.rs --ignored (2026-09-06): 72 verified
// specs (38 corpus atoms + 4 persist atoms, and all k>=2 composites),
// oracle labels == union-of-atom gold 72/72 against real clippy.
pub static FROZEN_LABELS_V3: &[(&str, &[&str])] = &[
    ("gen3-k1-byte_char_slices", &["byte_char_slices"]),  // k=1
    ("gen3-k1-collapsible_else_if", &["collapsible_else_if"]),  // k=1
    ("gen3-k1-collapsible_if", &["collapsible_if"]),  // k=1
    ("gen3-k1-deref_addrof", &["deref_addrof"]),  // k=1
    ("gen3-k1-double_parens", &["double_parens"]),  // k=1
    ("gen3-k1-filter_next", &["filter_next"]),  // k=1
    ("gen3-k1-identity_op", &["identity_op"]),  // k=1
    ("gen3-k1-items_after_statements", &["items_after_statements"]),  // k=1
    ("gen3-k1-iter_next_loop", &["iter_next_loop"]),  // k=1
    ("gen3-k1-iter_nth_zero", &["iter_nth_zero"]),  // k=1
    ("gen3-k1-manual_let_else", &["manual_let_else"]),  // k=1
    ("gen3-k1-manual_map", &["manual_map"]),  // k=1
    ("gen3-k1-manual_slice_fill", &["manual_slice_fill"]),  // k=1
    ("gen3-k1-manual_unwrap_or", &["manual_unwrap_or"]),  // k=1
    ("gen3-k1-map_unwrap_or", &["map_unwrap_or"]),  // k=1
    ("gen3-k1-match_bool", &["match_bool"]),  // k=1
    ("gen3-k1-needless_bool", &["needless_bool"]),  // k=1
    ("gen3-k1-needless_bool_assign", &["needless_bool_assign"]),  // k=1
    ("gen3-k1-needless_lifetimes", &["needless_lifetimes"]),  // k=1
    ("gen3-k1-needless_range_loop", &["needless_range_loop"]),  // k=1
    ("gen3-k1-needless_return", &["needless_return"]),  // k=1
    ("gen3-k1-persist_array_default", &["map_unwrap_or"]),  // k=1
    ("gen3-k1-persist_comment_arm", &["single_match_else"]),  // k=1
    ("gen3-k1-persist_comment_hoist", &["items_after_statements"]),  // k=1
    ("gen3-k1-persist_useless_format", &["useless_format"]),  // k=1
    ("gen3-k1-question_mark", &["question_mark"]),  // k=1
    ("gen3-k1-range_plus_one", &["range_plus_one"]),  // k=1
    ("gen3-k1-redundant_field_names", &["redundant_field_names"]),  // k=1
    ("gen3-k1-redundant_pattern_matching", &["redundant_pattern_matching"]),  // k=1
    ("gen3-k1-redundant_static_lifetimes", &["redundant_static_lifetimes"]),  // k=1
    ("gen3-k1-semicolon_if_nothing_returned", &["semicolon_if_nothing_returned"]),  // k=1
    ("gen3-k1-single_match", &["single_match"]),  // k=1
    ("gen3-k1-single_match_else", &["single_match_else"]),  // k=1
    ("gen3-k1-unnecessary_lazy_evaluations", &["unnecessary_lazy_evaluations"]),  // k=1
    ("gen3-k1-unnecessary_sort_by", &["unnecessary_sort_by"]),  // k=1
    ("gen3-k1-unnecessary_unwrap", &["unnecessary_unwrap"]),  // k=1
    ("gen3-k1-unused_unit", &["unused_unit"]),  // k=1
    ("gen3-k1-use_self", &["use_self"]),  // k=1
    ("gen3-k1-useless_format", &["useless_format"]),  // k=1
    ("gen3-k1-wildcard_in_or_patterns", &["wildcard_in_or_patterns"]),  // k=1
    ("gen3-k2-byte_char_slices+collapsible_else_if", &["byte_char_slices", "collapsible_else_if"]),  // k=2
    ("gen3-k2-collapsible_if+deref_addrof", &["collapsible_if", "deref_addrof"]),  // k=2
    ("gen3-k2-double_parens+filter_next", &["double_parens", "filter_next"]),  // k=2
    ("gen3-k2-identity_op+items_after_statements", &["identity_op", "items_after_statements"]),  // k=2
    ("gen3-k2-iter_next_loop+iter_nth_zero", &["iter_next_loop", "iter_nth_zero"]),  // k=2
    ("gen3-k2-manual_let_else+manual_map", &["manual_let_else", "manual_map"]),  // k=2
    ("gen3-k2-manual_slice_fill+manual_unwrap_or", &["manual_slice_fill", "manual_unwrap_or"]),  // k=2
    ("gen3-k2-map_unwrap_or+match_bool", &["map_unwrap_or", "match_bool"]),  // k=2
    ("gen3-k2-needless_bool+needless_bool_assign", &["needless_bool", "needless_bool_assign"]),  // k=2
    ("gen3-k2-needless_range_loop+needless_return", &["needless_range_loop", "needless_return"]),  // k=2
    ("gen3-k2-persist_array_default+persist_comment_arm", &["map_unwrap_or", "single_match_else"]),  // k=2
    ("gen3-k2-persist_comment_hoist+persist_useless_format", &["items_after_statements", "useless_format"]),  // k=2
    ("gen3-k2-question_mark+range_plus_one", &["question_mark", "range_plus_one"]),  // k=2
    ("gen3-k2-redundant_field_names+redundant_pattern_matching", &["redundant_field_names", "redundant_pattern_matching"]),  // k=2
    ("gen3-k2-redundant_static_lifetimes+semicolon_if_nothing_returned", &["redundant_static_lifetimes", "semicolon_if_nothing_returned"]),  // k=2
    ("gen3-k3-collapsible_if+deref_addrof+double_parens", &["collapsible_if", "deref_addrof", "double_parens"]),  // k=3
    ("gen3-k3-filter_next+identity_op+items_after_statements", &["filter_next", "identity_op", "items_after_statements"]),  // k=3
    ("gen3-k3-iter_next_loop+iter_nth_zero+manual_let_else", &["iter_next_loop", "iter_nth_zero", "manual_let_else"]),  // k=3
    ("gen3-k3-manual_map+manual_slice_fill+manual_unwrap_or", &["manual_map", "manual_slice_fill", "manual_unwrap_or"]),  // k=3
    ("gen3-k3-map_unwrap_or+match_bool+needless_bool", &["map_unwrap_or", "match_bool", "needless_bool"]),  // k=3
    ("gen3-k3-needless_range_loop+needless_return+question_mark", &["needless_range_loop", "needless_return", "question_mark"]),  // k=3
    ("gen3-k3-persist_array_default+persist_comment_arm+persist_comment_hoist", &["items_after_statements", "map_unwrap_or", "single_match_else"]),  // k=3
    ("gen3-k3-persist_useless_format+byte_char_slices+collapsible_else_if", &["byte_char_slices", "collapsible_else_if", "useless_format"]),  // k=3
    ("gen3-k3-range_plus_one+redundant_field_names+redundant_pattern_matching", &["range_plus_one", "redundant_field_names", "redundant_pattern_matching"]),  // k=3
    ("gen3-k3-redundant_static_lifetimes+semicolon_if_nothing_returned+single_match", &["redundant_static_lifetimes", "semicolon_if_nothing_returned", "single_match"]),  // k=3
    ("gen3-k4-byte_char_slices+collapsible_else_if+collapsible_if+deref_addrof", &["byte_char_slices", "collapsible_else_if", "collapsible_if", "deref_addrof"]),  // k=4
    ("gen3-k4-double_parens+filter_next+identity_op+items_after_statements", &["double_parens", "filter_next", "identity_op", "items_after_statements"]),  // k=4
    ("gen3-k4-iter_next_loop+iter_nth_zero+manual_let_else+manual_map", &["iter_next_loop", "iter_nth_zero", "manual_let_else", "manual_map"]),  // k=4
    ("gen3-k4-manual_slice_fill+manual_unwrap_or+map_unwrap_or+match_bool", &["manual_slice_fill", "manual_unwrap_or", "map_unwrap_or", "match_bool"]),  // k=4
    ("gen3-k4-needless_range_loop+needless_return+question_mark+range_plus_one", &["needless_range_loop", "needless_return", "question_mark", "range_plus_one"]),  // k=4
    ("gen3-k4-persist_array_default+persist_comment_arm+persist_comment_hoist+persist_useless_format", &["items_after_statements", "map_unwrap_or", "single_match_else", "useless_format"]),  // k=4
    ("gen3-k4-redundant_field_names+redundant_pattern_matching+redundant_static_lifetimes+semicolon_if_nothing_returned", &["redundant_field_names", "redundant_pattern_matching", "redundant_static_lifetimes", "semicolon_if_nothing_returned"]),  // k=4
];

/// Freeze readiness for generation 3 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
pub fn v3_freeze_status() -> Result<usize, String> {
    freeze_status_for(&specs_v3(), 3, "generation_v3_freeze", FROZEN_LABELS_V3, EXCLUDED_IDS_V3)
}

/// The admitted generation-v3 fixture set.
///
/// # Panics
///
/// Only on a freeze-record hole — callers check [`v3_freeze_status`] first.
#[must_use]
pub fn generation_v3() -> Vec<BenchFixture> {
    generation_fixtures(specs_v3(), 3, FROZEN_LABELS_V3, EXCLUDED_IDS_V3)
}

// ── Generation 4: the rustc-errors-side ladder (Plan 093, Bench 069 reopen
//    #4) ──────────────────────────────────────────────────────────────────
// cfg(`rustc_errors`): `score_bench` does not imply the domain — the atom
// factory reads `corpus_gen` through it, and the whole section compiles to
// nothing without the feature (a `--features score_bench` build stays
// green; the CLI arm errors cleanly instead).

/// Deterministic atom-slice target out of `generate_corpus()` (n ≥ 200 →
/// stride 5 → ~40 atoms, family-proportional because the corpus is
/// family-ordered). Sized so the ladder matches the clippy generations
/// (40 + 16 + 12 + 8 = 76 specs) while each spec costs a real `cargo check`.
#[cfg(feature = "rustc_errors")]
pub const GEN4_ATOM_TARGET: usize = 40;

/// The generation-4 atom table: a stride slice of the generated compile-error
/// corpus (`broken` half — the driver's repair subject), round-robin
/// interleaved by expected code so the stride windows compose MIXED-code
/// composites (raw corpus order is family-clustered; name order would be
/// too). Deterministic — no RNG, no environment reads.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn v4_atoms() -> Vec<V3Atom> {
    use crate::domains::rustc_errors::corpus_gen::GeneratedCase;

    let corpus = crate::domains::rustc_errors::corpus_gen::generate_corpus();
    let stride = (corpus.len() / GEN4_ATOM_TARGET).max(1);
    let sliced: Vec<&GeneratedCase> =
        corpus.iter().step_by(stride).take(GEN4_ATOM_TARGET).collect();

    // Round-robin interleave by expected code: one atom per code group per
    // pass, groups in sorted-code order, within-group order preserved —
    // adjacent atoms differ in code until groups exhaust at the tail.
    let mut groups: BTreeMap<&str, Vec<&GeneratedCase>> = BTreeMap::new();
    for case in &sliced {
        groups.entry(case.expect_code).or_default().push(case);
    }
    let keys: Vec<&str> = groups.keys().copied().collect();
    let mut cursors: BTreeMap<&str, usize> = keys.iter().map(|k| (*k, 0)).collect();
    let mut order: Vec<&GeneratedCase> = Vec::with_capacity(sliced.len());
    while order.len() < sliced.len() {
        let mut progressed = false;
        for key in &keys {
            let cur = cursors[key];
            if cur < groups[key].len() {
                order.push(groups[key][cur]);
                cursors.insert(key, cur + 1);
                progressed = true;
            }
        }
        if !progressed {
            break; // unreachable while order.len() < sliced.len()
        }
    }

    order
        .into_iter()
        .map(|c| V3Atom {
            id: c.name.clone(),
            expect: vec![c.expect_code.to_string()],
            code: c.broken.clone(),
        })
        .collect()
}

/// The generation-4 spec ladder: corpus-strided atoms (k=1) + the same
/// stride-window composites (k=2..4) over the code-interleaved atom list.
/// Byte-stable — pinned by test.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn specs_v4() -> Vec<GenSpec> {
    let atom_list = v4_atoms();
    specs_over_atoms("gen4", &atom_list)
}

/// Member i's expected error code for a generation-4 spec id, in the
/// composite's `mod gen_{i}` order (with duplicates — the per-member
/// multiset, not the distinct union in [`GenSpec::expect_lints`]). The
/// measurement runner attributes final-check diagnostics to member regions
/// through this ordering.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn v4_member_codes(spec_id: &str) -> Option<Vec<String>> {
    let rest = spec_id.strip_prefix("gen4-k")?;
    let (k, names) = rest.split_once('-')?;
    let k: usize = k.parse().ok()?;
    let names: Vec<&str> = names.split('+').collect();
    if names.len() != k {
        return None;
    }
    let atoms = v4_atoms();
    let by_name: BTreeMap<&str, &V3Atom> = atoms.iter().map(|a| (a.id.as_str(), a)).collect();
    names
        .iter()
        .map(|n| by_name.get(n).map(|a| a.expect[0].clone()))
        .collect()
}

/// Spec ids the generation-4 freeze run EXCLUDED (composed codes ≠ the union
/// of member codes on a real `cargo check` — the composition-interference
/// class) — landed by the freeze e2e. Empty pre-freeze (the dual-mode gate
/// then asserts the un-frozen Err).
pub static EXCLUDED_IDS_V4: &[&str] = &[];

/// The generation-4 frozen labels (spec id → the DISTINCT member error-code
/// set real rustc produced on the composed broken file at freeze time) —
/// landed by the freeze e2e. Empty pre-freeze.
pub static FROZEN_LABELS_V4: &[(&str, &[&str])] = &[];

/// Freeze readiness for generation 4 — Err until the freeze e2e lands the
/// record.
///
/// # Errors
///
/// The un-frozen state: how many specs lack oracle labels.
#[cfg(feature = "rustc_errors")]
pub fn v4_freeze_status() -> Result<usize, String> {
    freeze_status_for(&specs_v4(), 4, "generation_v4_freeze", FROZEN_LABELS_V4, EXCLUDED_IDS_V4)
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
    fn freeze_status_holes_are_loud_v3() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V3.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V3.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            // Pre-freeze: every spec is missing → Err (a silently-empty
            // generation must never reach the bench).
            assert!(
                v3_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            // Post-freeze: the always-on coverage gate — labels ∪ excluded
            // covers every spec, and each label equals the spec's union gold.
            for s in specs_v3() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V3
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

    #[test]
    fn k_of_parses_generation_ids() {
        assert_eq!(k_of("gen2-k3-a+b+c"), Some(3));
        assert_eq!(k_of("gen2-k1-identity_op"), Some(1));
        assert_eq!(k_of("gen3-k2-persist_array_default+map_unwrap_or"), Some(2));
        assert_eq!(k_of("composite-return_plus_identity_op"), None);
        assert_eq!(k_of("gen2-"), None);
        assert_eq!(k_of("gen-"), None);
    }

    #[test]
    fn persist_atom_table_parses_and_is_namespaced() {
        let vocab = super::super::vocabulary();
        assert!(!PERSIST_ATOMS.is_empty(), "persist table empty");
        for p in PERSIST_ATOMS {
            assert!(p.id.starts_with("persist_"), "persist id {} not namespaced", p.id);
            assert!(!p.expect.is_empty(), "persist atom {} expects nothing", p.id);
            for l in p.expect {
                assert!(vocab.contains(*l), "persist atom {} expects non-vocabulary lint {l}", p.id);
            }
            assert!(
                syn::parse_file(p.code).is_ok(),
                "persist atom {} does not parse standalone",
                p.id
            );
            assert!(!p.code.contains("mod gen_"), "persist atom {} carries the wrapper", p.id);
        }
    }

    #[test]
    fn v3_specs_are_deterministic_and_ladder_shaped() {
        let s1 = specs_v3();
        let s2 = specs_v3();
        assert_eq!(s1, s2, "gen3 specs must be byte-stable");
        let n = v3_atoms().len();
        assert!(n > PERSIST_ATOMS.len(), "gen3 atom table lost its corpus half");
        let k1 = s1.iter().filter(|s| s.k == 1).count();
        assert_eq!(k1, count_for_k(1, n));
        // The stratification contract: the persist atoms sit at the list
        // head, so every composite rung samples them.
        for &k in &LADDER_KS[1..] {
            let covered = count_for_k(k, n) * k;
            assert!(
                covered >= PERSIST_ATOMS.len(),
                "k={k} windows ({covered} slots) cannot reach all {} persist atoms",
                PERSIST_ATOMS.len()
            );
            for s in s1.iter().filter(|s| s.k == k) {
                let mods = s.code.matches("mod gen_").count();
                assert_eq!(mods, s.k, "gen3 composite {} wraps k atoms", s.id);
                assert!(
                    s.id.starts_with(&format!("gen3-k{k}-")),
                    "gen3 id encodes the generation + k: {}",
                    s.id
                );
            }
        }
        // Every persist atom appears in at least one k=2 composite.
        let k2_code: String = s1.iter().filter(|s| s.k == 2).map(|s| s.code.as_str()).collect();
        for p in PERSIST_ATOMS {
            assert!(k2_code.contains(p.code), "persist atom {} absent from every k=2 composite", p.id);
        }
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

    // ── generation 4 (rustc_errors-gated: the atom factory reads corpus_gen)

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_atom_table_slices_the_corpus_deterministically() {
        use crate::domains::rustc_errors::corpus_gen::{generate_corpus, territory_of, Territory};

        let a1 = v4_atoms();
        let a2 = v4_atoms();
        assert_eq!(a1, a2, "gen4 atom table must be byte-stable");
        let corpus = generate_corpus();
        let by_name: BTreeMap<&str, &crate::domains::rustc_errors::corpus_gen::GeneratedCase> =
            corpus.iter().map(|c| (c.name.as_str(), c)).collect();
        assert!(!a1.is_empty(), "gen4 atom table empty");
        assert!(a1.len() <= GEN4_ATOM_TARGET, "gen4 slice overshot its target");
        for a in &a1 {
            let case = by_name.get(a.id.as_str()).unwrap_or_else(|| panic!("gen4 atom {} is not a corpus case", a.id));
            assert_eq!(a.code, case.broken, "gen4 atom {} must carry the broken half", a.id);
            assert_eq!(a.expect, vec![case.expect_code.to_string()]);
            assert!(!a.code.contains("mod gen_"), "gen4 atom {} carries the wrapper", a.id);
        }
        // The stride slice must span both territories — the bench reports
        // them separately, and a single-territory ladder would blend by
        // omission.
        let uncontested = a1
            .iter()
            .any(|a| matches!(territory_of(&a.expect[0]), Territory::Uncontested));
        let contested = a1
            .iter()
            .any(|a| matches!(territory_of(&a.expect[0]), Territory::Contested));
        assert!(uncontested && contested, "gen4 slice lost a territory");
    }

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_specs_are_deterministic_and_ladder_shaped() {
        use crate::domains::rustc_errors::corpus_gen::territory_of;

        let s1 = specs_v4();
        let s2 = specs_v4();
        assert_eq!(s1, s2, "gen4 specs must be byte-stable");
        let ids: BTreeSet<&str> = s1.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids.len(), s1.len(), "duplicate gen4 generation ids");
        let n = v4_atoms().len();
        let k1 = s1.iter().filter(|s| s.k == 1).count();
        assert_eq!(k1, count_for_k(1, n));
        for &k in &LADDER_KS[1..] {
            for s in s1.iter().filter(|s| s.k == k) {
                let mods = s.code.matches("mod gen_").count();
                assert_eq!(mods, s.k, "gen4 composite {} wraps k atoms", s.id);
                assert!(
                    s.id.starts_with(&format!("gen4-k{k}-")),
                    "gen4 id encodes the generation + k: {}",
                    s.id
                );
                // expect_lints is the DISTINCT code set (≤ k — same-code
                // atoms can co-window); the per-member multiset is k.
                assert!(
                    !s.expect_lints.is_empty() && s.expect_lints.len() <= k,
                    "gen4 composite {} union size vs k",
                    s.id
                );
            }
            // Territory mix at every composite rung — both territories must
            // be reachable, or the per-territory report would be vacuous.
            let codes: BTreeSet<&str> = s1
                .iter()
                .filter(|s| s.k == k)
                .flat_map(|s| s.expect_lints.iter().map(String::as_str))
                .collect();
            let has_uncontested = codes.iter().any(|c| matches!(territory_of(c), crate::domains::rustc_errors::corpus_gen::Territory::Uncontested));
            let has_contested = codes.iter().any(|c| matches!(territory_of(c), crate::domains::rustc_errors::corpus_gen::Territory::Contested));
            assert!(has_uncontested && has_contested, "k={k} rung lost a territory");
        }
        // Every distinct code appears in at least one k≥2 composite.
        let composite_codes: BTreeSet<&str> = s1
            .iter()
            .filter(|s| s.k >= 2)
            .flat_map(|s| s.expect_lints.iter().map(String::as_str))
            .collect();
        for a in v4_atoms() {
            let code = a.expect[0].clone();
            assert!(
                composite_codes.contains(code.as_str()),
                "code {code} absent from every gen4 composite"
            );
        }
    }

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_member_codes_resolves_the_per_member_multiset() {
        let specs = specs_v4();
        for s in &specs {
            let codes = v4_member_codes(&s.id)
                .unwrap_or_else(|| panic!("member codes unresolvable for {}", s.id));
            assert_eq!(codes.len(), s.k, "member multiset size vs k: {}", s.id);
            let distinct: BTreeSet<&str> = codes.iter().map(String::as_str).collect();
            let want: BTreeSet<&str> = s.expect_lints.iter().map(String::as_str).collect();
            assert_eq!(distinct, want, "member distinct set vs union gold: {}", s.id);
        }
        assert_eq!(v4_member_codes("gen4-"), None);
        assert_eq!(v4_member_codes("gen2-k1-identity_op"), None);
        assert_eq!(v4_member_codes("gen4-k2-nonexistent+also_missing"), None);
    }

    #[test]
    #[cfg(feature = "rustc_errors")]
    fn v4_freeze_status_holes_are_loud() {
        let excluded: BTreeSet<&str> = EXCLUDED_IDS_V4.iter().copied().collect();
        let labeled: BTreeSet<&str> = FROZEN_LABELS_V4.iter().map(|(id, _)| *id).collect();
        if excluded.is_empty() && labeled.is_empty() {
            assert!(
                v4_freeze_status().is_err(),
                "pre-freeze must not claim readiness"
            );
        } else {
            for s in specs_v4() {
                assert!(
                    excluded.contains(s.id.as_str()) || labeled.contains(s.id.as_str()),
                    "spec {} is neither labeled nor excluded",
                    s.id
                );
                if labeled.contains(s.id.as_str()) {
                    let frozen = FROZEN_LABELS_V4
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

    #[test]
    fn ids_are_unique_across_the_generation() {
        for (name, list) in [("gen2", specs()), ("gen3", specs_v3())] {
            let ids: BTreeSet<&str> = list.iter().map(|x| x.id.as_str()).collect();
            assert_eq!(ids.len(), list.len(), "duplicate {name} generation ids");
        }
    }
}
