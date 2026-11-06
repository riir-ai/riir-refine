//! Issue 119 T3 — arms for the coverage-debt sweep.
//!
//! Every arm below perturbs something OUTSIDE the assertion it makes, so a
//! green row is evidence the rule fired rather than evidence the fixture
//! was shaped to agree with it (the `arm_reach_audit` discipline).

use std::fmt::Write as _;

use super::*;

/// A minimal `-W help` transcript in the real format — column-aligned,
/// both sections, the rustc namespace present so the section split is
/// actually exercised rather than assumed.
fn help_fixture(n_filler: usize) -> String {
    let mut s = String::from(
        "\nAvailable lint options:\n    -W <foo>           Warn about <foo>\n\n\
         Lint checks provided by rustc:\n\n\
         \x20   name  default  meaning\n\
         \x20   ----  -------  -------\n\
         \x20   dead-code  warn    detects unused code\n\
         \x20   unused-mut  warn    detects unneeded mut\n\n\
         Lint groups provided by rustc:\n\n\
         \x20   name  sub-lints\n\
         \x20   unused  unused-mut, dead-code\n\n\
         Lint checks loaded by this crate:\n\n\
         \x20   name  default  meaning\n\
         \x20   ----  -------  -------\n\
         \x20   clippy::needless-return  warn     unneeded return statement\n\
         \x20   clippy::must-use-candidate  allow    fn could be must_use\n\
         \x20   clippy::approx-constant  deny     approximate constant\n\
         \x20   clippy::absolute-paths  allow    usage of an item without a use\n",
    );
    for i in 0..n_filler {
        let _ = writeln!(
            s,
            "                        clippy::filler-lint-{i:04}  warn     filler"
        );
    }
    s.push_str(
        "\nLint groups loaded by this crate:\n\n\
         \x20   name  sub-lints\n\
         \x20   ----  ---------\n",
    );
    // `all` is the UNION and must contribute no group membership.
    s.push_str("                           clippy::all  clippy::needless-return, clippy::approx-constant\n");
    s.push_str("                         clippy::style  clippy::needless-return\n");
    s.push_str("                      clippy::pedantic  clippy::must-use-candidate\n");
    s.push_str("                   clippy::correctness  clippy::approx-constant\n");
    let mut restriction =
        String::from("                   clippy::restriction  clippy::absolute-paths");
    for i in 0..n_filler {
        let _ = write!(restriction, ", clippy::filler-lint-{i:04}");
    }
    restriction.push('\n');
    s.push_str(&restriction);
    s
}

/// Enough filler to clear `MIN_DERIVED_LINTS` without the arms caring.
fn full_surface() -> LintSurface {
    parse_lint_help(&help_fixture(MIN_DERIVED_LINTS))
}

fn corpus_rules() -> Vec<String> {
    let mut v: Vec<String> = (0..MIN_CORPUS_RULES)
        .map(|i| format!("corpus_only_rule_{i:03}"))
        .collect();
    v.push("needless_return".into());
    v
}

fn fix_set() -> Vec<String> {
    let mut v: Vec<String> = (0..MIN_FIX_SET)
        .map(|i| format!("fix_only_{i:03}"))
        .collect();
    // In the fix lane, absent from the corpus — the Issue-116 seam.
    v.push("approx_constant".into());
    v
}

/// Rows for the default-eco axis when a test does not exercise it — a
/// named const so the lifetime outlives the helper's body.
const NO_DEFAULT_ROWS: &[(String, u32)] = &[];

/// Empty body-scan inputs — the default every arm that does not exercise
/// the Issue-167 tier rides.
const NO_BLOBS: &[String] = &[];

fn inputs<'a>(
    surface: LintSurface,
    corpus: &'a [String],
    fixes: &'a [String],
    others: &'a BTreeMap<String, Vec<String>>,
    hist: &'a [(String, u32)],
) -> CoverageInputs<'a> {
    CoverageInputs {
        toolchain: "clippy 0.1.98 (deadbeef 2026-09-01)",
        surface,
        corpus_rules: corpus,
        fix_set: fixes,
        other_domains: others,
        histogram: hist,
        histogram_provenance: Some(HistogramProvenance {
            source: "fixture".into(),
            captured_utc: "2026-08-30".into(),
            toolchain: "cargo 1.97.0".into(),
            dataset_blake3: "eb4aaed8".into(),
            repos: 6,
            rows: 2,
        }),
        default_histogram: NO_DEFAULT_ROWS,
        default_histogram_provenance: None,
        corpus_blobs: NO_BLOBS,
        corpus_docs_scanned: false,
        ahead_of_pin: &[],
    }
}

/// The same inputs with a default-eco histogram joined — the Bench-094 half
/// of the frequency axis. Provenance is a fixture: same shape as the real
/// manifest supplies, so the render arms are exercised identically.
fn inputs_with_default<'a>(
    surface: LintSurface,
    corpus: &'a [String],
    fixes: &'a [String],
    others: &'a BTreeMap<String, Vec<String>>,
    hist: &'a [(String, u32)],
    default_hist: &'a [(String, u32)],
) -> CoverageInputs<'a> {
    let mut ins = inputs(surface, corpus, fixes, others, hist);
    ins.default_histogram = default_hist;
    ins.default_histogram_provenance = Some(HistogramProvenance {
        source: "Bench 094 default-level fired histogram (fixture)".into(),
        captured_utc: "2026-09-17".into(),
        toolchain: "clippy 0.1.98 (deadbeef 2026-09-01)".into(),
        dataset_blake3: "2740b5e6".into(),
        repos: 184,
        rows: default_hist.len(),
    });
    ins
}

// ── the parser ───────────────────────────────────────────────────────────

#[test]
fn parse_reads_only_the_crate_sections() {
    let s = parse_lint_help(&help_fixture(0));
    // 4 clippy lints; the two rustc lints are a different namespace.
    assert_eq!(s.lints.len(), 4, "rustc-namespace rows must not be counted");
    assert!(s.lints.iter().all(|l| !l.name.contains("dead_code")));
}

#[test]
fn parse_normalizes_kebab_to_snake() {
    let s = parse_lint_help(&help_fixture(0));
    assert!(s.lints.iter().any(|l| l.name == "needless_return"));
    assert!(
        !s.lints.iter().any(|l| l.name.contains('-')),
        "a kebab name leaks straight into a 100%-debt diff"
    );
}

#[test]
fn parse_reads_default_levels() {
    let s = parse_lint_help(&help_fixture(0));
    let by = |n: &str| s.lints.iter().find(|l| l.name == n).map(|l| l.level);
    assert_eq!(by("needless_return"), Some(DefaultLevel::Warn));
    assert_eq!(by("must_use_candidate"), Some(DefaultLevel::Allow));
    assert_eq!(by("approx_constant"), Some(DefaultLevel::Deny));
    assert_eq!(s.level_unparsed, 0);
}

#[test]
fn union_group_all_is_never_a_lints_own_group() {
    let s = parse_lint_help(&help_fixture(0));
    assert!(
        !s.groups.contains_key(UNION_GROUP),
        "clippy::all is the union — counting it double-counts every member"
    );
    let g = |n: &str| {
        s.lints
            .iter()
            .find(|l| l.name == n)
            .map(|l| l.group.clone())
            .unwrap_or_default()
    };
    // Both are IN clippy::all; each must report its own group.
    assert_eq!(g("needless_return"), "style");
    assert_eq!(g("approx_constant"), "correctness");
    assert_eq!(s.ambiguous_group, 0);
    assert_eq!(s.ungrouped, 0);
}

#[test]
fn a_lint_in_two_non_union_groups_is_ambiguous_not_guessed() {
    let mut text = help_fixture(0);
    // Put a style lint into `perf` as well.
    text.push_str("                          clippy::perf  clippy::needless-return\n");
    let s = parse_lint_help(&text);
    assert_eq!(s.ambiguous_group, 1);
    let row = s
        .lints
        .iter()
        .find(|l| l.name == "needless_return")
        .unwrap();
    assert_eq!(
        row.group, "(ambiguous)",
        "a two-group lint must not be silently bucketed"
    );
}

#[test]
fn a_lint_in_no_group_is_ungrouped_not_bucketed() {
    let text = help_fixture(0).replace(
        "                      clippy::pedantic  clippy::must-use-candidate\n",
        "",
    );
    let s = parse_lint_help(&text);
    assert_eq!(s.ungrouped, 1);
    let row = s
        .lints
        .iter()
        .find(|l| l.name == "must_use_candidate")
        .unwrap();
    assert_eq!(row.group, "(ungrouped)");
}

// ── the floors: a green zero over a blind walk is the failure mode ──────

#[test]
fn derivation_floor_refuses_a_format_regression() {
    let s = parse_lint_help(&help_fixture(0));
    assert!(
        s.blind.is_some(),
        "4 lints is below MIN_DERIVED_LINTS — a `-W help` format change must \
         RED, not report a flattering small surface"
    );
    // ...and the same fixture with real volume does not.
    assert!(full_surface().blind.is_none());
}

#[test]
fn a_missing_section_is_blind_not_empty() {
    let s = parse_lint_help("Available lint options:\n  -W <foo>  Warn\n");
    assert!(s.blind.is_some());
    assert_eq!(s.lints, []);
}

#[test]
fn corpus_floor_refuses_an_uncompiled_domain() {
    let others = BTreeMap::new();
    let fixes = fix_set();
    let thin = vec!["needless_return".to_string()];
    let r = coverage(&inputs(full_surface(), &thin, &fixes, &others, &[]));
    assert!(
        r.blind.is_some(),
        "an absent clippy_lints corpus makes the debt read as TOTAL — refuse"
    );
    assert!(
        r.uncovered.is_empty(),
        "no rows may be reported past a refusal"
    );
}

#[test]
fn fix_set_floor_refuses_an_uncompiled_lane() {
    let others = BTreeMap::new();
    let corpus = corpus_rules();
    let thin = vec!["approx_constant".to_string()];
    let r = coverage(&inputs(full_surface(), &corpus, &thin, &others, &[]));
    assert!(r.blind.is_some());
}

#[test]
fn a_blind_derivation_propagates_into_the_report() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(
        parse_lint_help(&help_fixture(0)),
        &corpus,
        &fixes,
        &others,
        &[],
    ));
    assert!(r.blind.is_some());
    assert!(r.render().contains("⛔ BLIND"));
    assert!(!r.render().contains("the coverage debt"));
}

// ── the three coverage axes, never pooled ───────────────────────────────

#[test]
fn corpus_and_fix_only_are_separate_buckets() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    assert!(r.blind.is_none());
    // needless_return: corpus (and not in the fix fixture) → covered_corpus
    assert_eq!(r.covered_corpus, 1);
    // approx_constant: fix lane only → its own bucket, NOT covered_corpus
    assert_eq!(
        r.covered_fix_only, 1,
        "a fix-set-only lint is a corpus gap wearing a green"
    );
    assert!(r.uncovered.iter().all(|u| u.lint != "approx_constant"));
    assert!(r.uncovered.iter().any(|u| u.lint == "must_use_candidate"));
}

#[test]
fn the_firing_split_separates_restriction_backlog_from_real_debt() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    // must_use_candidate (allow) + absolute_paths (allow) never fire;
    // every filler is `warn` and does.
    assert_eq!(r.uncovered_firing(), MIN_DERIVED_LINTS);
    assert_eq!(r.uncovered.len(), MIN_DERIVED_LINTS + 2);
}

#[test]
fn a_cross_domain_name_collision_is_a_finding_not_silence() {
    let mut others = BTreeMap::new();
    others.insert(
        "rust_perf".to_string(),
        vec!["must-use-candidate".to_string()],
    );
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(
        full_surface(),
        &corpus,
        &fixes,
        &others,
        &others_hist(),
    ));
    assert_eq!(r.collides_other_domain.len(), 1);
    assert_eq!(r.collides_other_domain[0].1, "rust_perf");
    assert!(r.render().contains("two domains claiming one name"));
}

fn others_hist() -> Vec<(String, u32)> {
    vec![]
}

#[test]
fn the_clean_dedup_case_says_so_explicitly() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    assert_eq!(r.collides_other_domain, []);
    assert!(r.render().contains("0 uncovered lint name collides"));
}

#[test]
fn a_corpus_rule_the_driver_no_longer_ships_is_reported() {
    let others = BTreeMap::new();
    let mut corpus = corpus_rules();
    corpus.push("renamed_away_lint".into());
    let fixes = fix_set();
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    assert!(r
        .corpus_rules_not_in_surface
        .contains(&"renamed_away_lint".to_string()));
    assert!(r.render().contains("renamed/removed"));
}

/// Issue 168: a key on the inputs' ahead-of-pin registry is a known-good
/// posture — it partitions into `corpus_rules_ahead_of_pin`, never the
/// renamed/removed drift, and the render discloses it on its own line.
#[test]
fn an_ahead_of_pin_key_is_disclosed_not_drift_flagged() {
    let others = BTreeMap::new();
    let mut corpus = corpus_rules();
    corpus.push("manual_bit_width".into());
    corpus.push("renamed_away_lint".into());
    let fixes = fix_set();
    let mut ins = inputs(full_surface(), &corpus, &fixes, &others, &[]);
    ins.ahead_of_pin = &["manual_bit_width"];
    let r = coverage(&ins);
    assert_eq!(
        r.corpus_rules_ahead_of_pin,
        vec!["manual_bit_width".to_string()],
        "the registered key discloses in its own bucket"
    );
    assert!(
        r.corpus_rules_not_in_surface
            .contains(&"renamed_away_lint".to_string()),
        "an unregistered absent key stays the renamed/removed drift"
    );
    assert!(
        !r.corpus_rules_not_in_surface
            .contains(&"manual_bit_width".to_string()),
        "the registered key must not double-report as drift"
    );
    let out = r.render();
    assert!(out.contains("AHEAD-OF-PIN"));
    assert!(out.contains("manual_bit_width"));
    assert!(!out.contains("(renamed/removed): manual_bit_width"));
}

// ── T3: the frequency join ──────────────────────────────────────────────

#[test]
fn frequency_is_joined_and_ranked_never_estimated() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![
        ("must_use_candidate".to_string(), 1011),
        ("absolute_paths".to_string(), 7),
    ];
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist));
    let ranked = r.freq_ranked();
    assert_eq!(ranked.len(), 2, "only the joined rows carry evidence");
    assert_eq!(ranked[0].lint, "must_use_candidate");
    assert_eq!(ranked[0].freq, FreqEvidence::Fired(1011));
    assert_eq!(ranked[1].freq, FreqEvidence::Fired(7));
    // Everything else is UNCITED — not zero.
    let filler = r
        .uncovered
        .iter()
        .find(|u| u.lint.starts_with("filler_lint_"))
        .expect("filler rows are uncovered");
    assert_eq!(
        filler.freq,
        FreqEvidence::Uncited,
        "'not observed in six repos on one day' is not 'does not fire'"
    );
}

// ── T3: the dual-evidence tier ──────────────────────────────────────────

/// A `-W help` fixture that also ships the three study-cited lints, so the
/// study join has something real to land on.
fn help_with_study(n_filler: usize) -> String {
    let mut text = help_fixture(n_filler);
    let rows = "                    clippy::missing-errors-doc  allow    missing errors section\n\
                \x20                   clippy::missing-panics-doc  allow    missing panics section\n\
                \x20      clippy::semicolon-if-nothing-returned  allow    missing trailing semicolon\n";
    text = text.replace(
        "\nLint groups loaded by this crate:",
        &format!("{rows}\nLint groups loaded by this crate:"),
    );
    text.push_str(
        "                      clippy::pedantic  clippy::missing-errors-doc, \
         clippy::missing-panics-doc, clippy::semicolon-if-nothing-returned\n",
    );
    text
}

#[test]
fn both_sources_produce_the_dual_evidence_tier() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    // missing_errors_doc is in BOTH the local histogram and kornel's top-10.
    let hist = vec![
        ("missing_errors_doc".to_string(), 90),
        ("must_use_candidate".to_string(), 1011),
    ];
    let surface = parse_lint_help(&help_with_study(MIN_DERIVED_LINTS));
    assert!(surface.blind.is_none());
    let r = coverage(&inputs(surface, &corpus, &fixes, &others, &hist));

    let dual = r.dual_evidence();
    assert_eq!(dual.len(), 1, "exactly one lint carries both sources");
    assert_eq!(dual[0].lint, "missing_errors_doc");
    assert_eq!(
        dual[0].freq,
        FreqEvidence::Both {
            fired: 90,
            study: 56_975
        }
    );

    // Tier DOMINATES the count: 90 firings with two sources outranks 1011
    // with one. Pooling the sources would invert this.
    let ranked = r.freq_ranked();
    assert_eq!(ranked[0].lint, "missing_errors_doc");
    assert_eq!(ranked[1].lint, "must_use_candidate");
}

#[test]
fn a_study_only_lint_is_cited_and_ranks_below_local_evidence() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![("must_use_candidate".to_string(), 3)];
    let surface = parse_lint_help(&help_with_study(MIN_DERIVED_LINTS));
    let r = coverage(&inputs(surface, &corpus, &fixes, &others, &hist));
    let ranked = r.freq_ranked();
    // 3 local firings still outrank 56,975 study firings — different tiers.
    assert_eq!(ranked[0].lint, "must_use_candidate");
    assert_eq!(ranked[0].freq, FreqEvidence::Fired(3));
    let study: Vec<&str> = ranked
        .iter()
        .filter(|r| matches!(r.freq, FreqEvidence::Study(_)))
        .map(|r| r.lint.as_str())
        .collect();
    assert_eq!(
        study,
        vec![
            "missing_errors_doc",
            "missing_panics_doc",
            "semicolon_if_nothing_returned"
        ],
        "study rows are cited, ordered by their published count"
    );
    assert_eq!(
        r.dual_evidence(),
        Vec::<&crate::corpus_coverage::CoverageRow>::new()
    );
}

#[test]
fn the_two_source_counts_are_never_summed() {
    let e = FreqEvidence::Both {
        fired: 90,
        study: 56_975,
    };
    assert_eq!(
        e.rank(),
        90,
        "the LOCAL count ranks; summing two different populations invents a third"
    );
    assert_eq!(e.label(), "local+study");
    assert!(e.is_cited());
    assert!(!FreqEvidence::Uncited.is_cited());
}

#[test]
fn a_missing_source_is_unobserved_not_zero() {
    assert_eq!(join_evidence(None, None, None), FreqEvidence::Uncited);
    assert_eq!(join_evidence(Some(5), None, None), FreqEvidence::Fired(5));
    assert_eq!(join_evidence(None, None, Some(5)), FreqEvidence::Study(5));
    assert_eq!(
        join_evidence(None, Some(7), None),
        FreqEvidence::FiredDefault(7)
    );
    // An unobserved source must NOT rank below an observed rare lint by
    // being defaulted to 0 — it leaves the tier instead.
    assert!(
        join_evidence(Some(1), None, None).tier() < join_evidence(None, None, Some(99_999)).tier()
    );
}

#[test]
fn the_three_source_precedence_is_documented_not_accidental() {
    // 1. fired + study is maximum breadth and stays Both regardless of the
    //    default-eco count (which remains in its own manifest).
    assert_eq!(
        join_evidence(Some(3), Some(9), Some(50_000)),
        FreqEvidence::Both {
            fired: 3,
            study: 50_000
        }
    );
    // 2. a default-eco count shadows a lone local count — same-level
    //    evidence is the axis this report's charter needs.
    assert_eq!(
        join_evidence(Some(40), Some(8), None),
        FreqEvidence::FiredDefault(8)
    );
    // ...and shadows a lone published count for the same reason.
    assert_eq!(
        join_evidence(None, Some(8), Some(99_999)),
        FreqEvidence::FiredDefault(8)
    );
    // 3. the tiers order default-eco between Both and Fired.
    let both = FreqEvidence::Both { fired: 1, study: 1 };
    let def = FreqEvidence::FiredDefault(1);
    let local = FreqEvidence::Fired(1);
    let study = FreqEvidence::Study(1);
    assert!(both.tier() < def.tier());
    assert!(def.tier() < local.tier());
    assert!(local.tier() < study.tier());
    assert_eq!(def.label(), "default-eco");
    // rank() mirrors the manifest count (the 8 above was the shadowing
    // example's count; this binding carries 1).
    assert_eq!(def.rank(), 1);
    assert!(def.is_cited());
}

#[test]
fn the_study_table_stays_inside_the_clippy_namespace() {
    // kornel's top-10 is 7 rustc lints + 3 clippy ones; attributing a rustc
    // lint's frequency to a clippy corpus gap is the transcription error
    // this pin exists to catch.
    assert_eq!(STUDY_KORNEL_2024.len(), 3);
    for (name, count) in STUDY_KORNEL_2024 {
        assert!(!name.contains("::"), "names are stored normalized");
        assert_eq!(*name, normalize(name), "{name} is not in normal form");
        assert!(*count > 0);
    }
    for banned in ["unused_imports", "missing_docs", "unused_qualifications"] {
        assert!(
            !STUDY_KORNEL_2024.iter().any(|(n, _)| *n == banned),
            "{banned} is a rustc lint, outside this report's namespace"
        );
    }
}

#[test]
fn the_report_prints_both_evidence_sources_and_the_dual_count() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![("missing_errors_doc".to_string(), 90)];
    let surface = parse_lint_help(&help_with_study(MIN_DERIVED_LINTS));
    let out = coverage(&inputs(surface, &corpus, &fixes, &others, &hist)).render();
    assert!(out.contains("local:"), "the local source must be named");
    assert!(out.contains("study:"), "the study source must be named");
    assert!(out.contains("kornel"), "the study citation must be printed");
    assert!(out.contains("1 row(s) carry BOTH sources"));
    assert!(out.contains("local+study"));
    assert!(out.contains("uncited, NOT zero"));
}

#[test]
fn a_covered_lint_never_enters_the_ranked_head() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    // needless_return IS in the corpus; a hot histogram row must not
    // resurrect it as debt.
    let hist = vec![("needless_return".to_string(), 9999)];
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist));
    assert_eq!(
        r.freq_ranked(),
        Vec::<&crate::corpus_coverage::CoverageRow>::new()
    );
}

#[test]
fn a_stale_histogram_row_is_counted_not_dropped() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![("a_lint_clippy_dropped".to_string(), 42)];
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist));
    assert_eq!(r.histogram_rows_stale, 1);
    assert!(r.render().contains("no longer ships"));
}

#[test]
fn a_cross_toolchain_histogram_warns_on_its_own_line() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![("must_use_candidate".to_string(), 1011)];
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist));
    assert!(
        r.render().contains("DIFFERENT toolchain"),
        "the fixture captures under 1.97 and sweeps 1.98 — the staleness must print"
    );
}

#[test]
fn a_same_toolchain_histogram_does_not_warn() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![("must_use_candidate".to_string(), 1011)];
    let mut ins = inputs(full_surface(), &corpus, &fixes, &others, &hist);
    ins.histogram_provenance = Some(HistogramProvenance {
        toolchain: "0.1.98".into(),
        ..ins.histogram_provenance.clone().unwrap()
    });
    let r = coverage(&ins);
    assert!(!r.render().contains("DIFFERENT toolchain"));
}

#[test]
fn no_histogram_says_none_joined_rather_than_printing_an_empty_table() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let mut ins = inputs(full_surface(), &corpus, &fixes, &others, &[]);
    ins.histogram_provenance = None;
    let r = coverage(&ins);
    let out = r.render();
    assert!(out.contains("local: NONE joined"));
    // The STUDY source is independent of the histogram and must still print.
    assert!(out.contains("study: kornel"));
}

// ── the group table + the undecidable axis ──────────────────────────────

#[test]
fn group_debt_is_ordered_by_debt_and_sums_to_the_surface() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    let total: usize = r.group_debt.iter().map(|g| g.total).sum();
    assert_eq!(
        total, r.lints_derived,
        "the group table must be a partition"
    );
    let unc: usize = r.group_debt.iter().map(|g| g.uncovered).sum();
    assert_eq!(unc, r.uncovered.len());
    let cov: usize = r.group_debt.iter().map(|g| g.covered).sum();
    assert_eq!(cov, r.covered_corpus + r.covered_fix_only);
    for w in r.group_debt.windows(2) {
        assert!(w[0].uncovered >= w[1].uncovered, "hottest group first");
    }
}

#[test]
fn applicability_is_declared_underivable_rather_than_printed() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    let out = r.render();
    assert!(out.contains("applicability"));
    assert!(out.contains("EMITTED diagnostic"));
}

#[test]
fn normalize_is_idempotent_and_prefix_free() {
    assert_eq!(normalize("clippy::needless-return"), "needless_return");
    assert_eq!(normalize("needless_return"), "needless_return");
    assert_eq!(
        normalize(normalize("clippy::USE-Self").as_str()),
        "use_self"
    );
}

#[test]
fn fires_by_default_matches_the_level_semantics() {
    assert!(!DefaultLevel::Allow.fires_by_default());
    assert!(DefaultLevel::Warn.fires_by_default());
    assert!(DefaultLevel::Deny.fires_by_default());
    assert_eq!(DefaultLevel::parse("forbid"), Some(DefaultLevel::Deny));
    assert_eq!(DefaultLevel::parse("mumble"), None);
}

#[test]
fn the_firing_evidence_cross_tab_is_reported_and_its_disjoint_case_is_loud() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    // must_use_candidate is `allow`; every filler is `warn`. So the
    // evidenced set and the firing set are DISJOINT — the real shape.
    let hist = vec![("must_use_candidate".to_string(), 1011)];
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist));
    assert!(r.uncovered_firing() > 0);
    assert_eq!(r.firing_with_evidence(), 0);
    let out = r.render();
    assert!(out.contains("carry evidence"));
    assert!(
        out.contains("⚠ DISJOINT"),
        "a disjoint cross-tab must be loud"
    );

    // ...and the warning does NOT fire once a firing lint carries evidence.
    let hist2 = vec![("filler_lint_0000".to_string(), 12)];
    let r2 = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist2));
    assert_eq!(r2.firing_with_evidence(), 1);
    assert!(!r2.render().contains("⚠ DISJOINT"));
}

// ── T4 successor: the default-eco axis (Bench 094) ─────────────────────

/// The exact shape the Issue-119 first run said was missing for the ENTIRE
/// default-firing set: a default-firing uncovered lint whose only evidence
/// comes from the default-level ecosystem histogram. The cross-tab's
/// breakdown line must name the source, and the provenance must print.
#[test]
fn a_default_eco_row_closes_the_default_firing_evidence_gap() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let def = vec![("filler_lint_0000".to_string(), 8)];
    let r = coverage(&inputs_with_default(
        full_surface(),
        &corpus,
        &fixes,
        &others,
        &[],
        &def,
    ));
    assert_eq!(r.firing_with_evidence(), 1);
    assert_eq!(r.firing_with_default_evidence(), 1);
    let row = r
        .uncovered
        .iter()
        .find(|r| r.lint == "filler_lint_0000")
        .expect("filler is uncovered");
    assert_eq!(row.freq, FreqEvidence::FiredDefault(8));
    let out = r.render();
    assert!(
        out.contains("default-eco: Bench 094"),
        "the source must print"
    );
    assert!(out.contains("via the default-eco histogram"));
    assert!(!out.contains("⚠ DISJOINT"));
}

/// The disjoint warning STILL fires when the default-eco histogram is
/// absent — the gap can regress, and the instrument must stay loud.
#[test]
fn a_missing_default_eco_manifest_prints_none_joined_and_disjoint_stays_live() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let hist = vec![("must_use_candidate".to_string(), 1011)];
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &hist));
    let out = r.render();
    assert!(out.contains("default-eco: NONE joined"));
    assert!(out.contains("⚠ DISJOINT"));
    assert_eq!(r.firing_with_default_evidence(), 0);
}

/// Stale rows in the default-eco histogram count separately — a lint the
/// driver no longer ships is drift, not evidence.
#[test]
fn default_eco_stale_rows_are_counted_not_silently_dropped() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let def = vec![
        ("filler_lint_0000".to_string(), 8),
        ("removed_lint_from_2023".to_string(), 500),
    ];
    let r = coverage(&inputs_with_default(
        full_surface(),
        &corpus,
        &fixes,
        &others,
        &[],
        &def,
    ));
    assert_eq!(r.default_histogram_rows_stale, 1);
    assert_eq!(r.default_histogram_provenance.as_ref().unwrap().rows, 2);
}

// ── Issue 167: the pattern-mentioned tier ───────────────────────────────

/// The measured precedent, as a fixture: `len_zero` UNKEYED in the corpus,
/// mentioned only in a doc-comment blob. It must land in
/// `pattern_mentioned` while the buckets' counts stay identical to a run
/// without the body scan — the disclosure, never a fold.
/// The fixture with `len_zero` added: the lint row goes into the CHECKS
/// section (before the groups header), the group membership rides the
/// existing style row — the same insertion shape `help_with_study` uses.
fn help_with_len_zero(n_filler: usize) -> String {
    help_fixture(n_filler)
        .replace(
            "\nLint groups loaded by this crate:",
            "         \x20   clippy::len-zero  warn     checks for comparisons to zero\
             \n\nLint groups loaded by this crate:",
        )
        .replace(
            "                      clippy::style  clippy::needless-return\n",
            "                      clippy::style  clippy::needless-return, clippy::len-zero\n",
        )
}

#[test]
fn a_doc_only_mention_lands_in_pattern_mentioned_never_in_covered() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let text = help_with_len_zero(MIN_DERIVED_LINTS);
    let surface = parse_lint_help(&text);
    assert!(surface.blind.is_none());
    assert!(surface.lints.iter().any(|l| l.name == "len_zero"));
    let blobs = vec!["heals clippy's len_zero pattern (`.len() == 0` -> `is_empty()`)".to_string()];
    let mut ins = inputs(surface, &corpus, &fixes, &others, &[]);
    ins.corpus_blobs = &blobs;
    ins.corpus_docs_scanned = true;
    let r = coverage(&ins);
    let base = coverage(&inputs(
        parse_lint_help(&text),
        &corpus,
        &fixes,
        &others,
        &[],
    ));
    // The buckets are UNCHANGED by the scan (membership both directions):
    assert_eq!(r.covered_corpus, base.covered_corpus);
    assert_eq!(r.covered_fix_only, base.covered_fix_only);
    assert_eq!(r.uncovered.len(), base.uncovered.len());
    assert_eq!(r.group_debt, base.group_debt);
    // len_zero is uncovered AND pattern-mentioned — disclosed, never folded:
    assert!(r.uncovered.iter().any(|u| u.lint == "len_zero"));
    assert_eq!(r.pattern_mentioned, vec!["len_zero".to_string()]);
    let out = r.render();
    assert!(out.contains("pattern-mentioned in rule bodies"));
    assert!(out.contains("LOWER BOUND"));
}

/// `manual_len_zero` naming `len_zero` as a substring is NOT coverage of
/// `len_zero` — the scan is word-boundary by construction, and this arm
/// pins the construction against a substring-check regression.
#[test]
fn a_longer_identifier_mention_is_not_a_mention() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let text = help_with_len_zero(MIN_DERIVED_LINTS);
    let surface = parse_lint_help(&text);
    let blobs = vec!["see also manual_len_zero and its friends".to_string()];
    let mut ins = inputs(surface, &corpus, &fixes, &others, &[]);
    ins.corpus_blobs = &blobs;
    let r = coverage(&ins);
    assert_eq!(r.pattern_mentioned, Vec::<String>::new());
}

/// An empty blob set is the scan's honest zero — identical buckets, and
/// the seams+limits disclosure ALWAYS prints; the docs-unavailable line
/// rides only when the scan could not read the doc half.
#[test]
fn no_blobs_yield_no_mentions_and_the_lower_bound_still_discloses() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let r = coverage(&inputs(full_surface(), &corpus, &fixes, &others, &[]));
    assert_eq!(r.pattern_mentioned, Vec::<String>::new());
    assert!(!r.corpus_docs_scanned);
    let out = r.render();
    assert!(out.contains("LOWER BOUND"));
    assert!(
        out.contains("pattern-literals-only"),
        "an absent docs half must be loud, never a silent zero"
    );
}

/// A body scan that DID read the doc half must not print the
/// pattern-literals-only warning — the disclosure fires one way only.
#[test]
fn a_docs_scanned_run_prints_no_availability_warning() {
    let others = BTreeMap::new();
    let (corpus, fixes) = (corpus_rules(), fix_set());
    let mut ins = inputs(full_surface(), &corpus, &fixes, &others, &[]);
    ins.corpus_docs_scanned = true;
    let r = coverage(&ins);
    assert!(!r.render().contains("pattern-literals-only"));
}

/// The single-pass scanner itself: word boundaries, digits, trailing
/// punctuation, no cross-boundary bleed in either direction, sorted out.
#[test]
fn mentioned_names_is_word_boundary_exact() {
    let candidates: BTreeSet<String> = ["len_zero", "len_zero_2", "approx_constant"]
        .into_iter()
        .map(String::from)
        .collect();
    let found = mentioned_names(
        "len_zero, manual_len_zero xlen_zero len_zero_2 approx_constant::foo len_zero\n",
        &candidates,
    );
    assert_eq!(found, vec!["approx_constant", "len_zero", "len_zero_2"]);
    assert_eq!(mentioned_names("", &candidates), Vec::<String>::new());
}

/// The live corpus's own precedent, pinned at the source: the
/// `comparison_to_empty` proposer names `len_zero` ONLY in its doc
/// comments — the exact shape the name-keyed buckets go blind on. If this
/// source ever stops carrying the mention, the fixture loses its
/// real-world anchor and this test says so.
#[test]
fn the_len_zero_precedent_source_is_scannable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/domains/clippy_lints/proposers/comparison_to_empty.rs");
    let text = std::fs::read_to_string(&path)
        .expect("the precedent proposer source must be readable beside the crate");
    let cands: BTreeSet<String> = ["len_zero".to_string()].into_iter().collect();
    assert_eq!(mentioned_names(&text, &cands), vec!["len_zero".to_string()]);
}
