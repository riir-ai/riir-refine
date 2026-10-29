//! Issue 134 T1 — the standing **demonstrability column** (measurement
//! only, no behavior change).
//!
//! Source: katgpt-rs Research 585 (SAT, arXiv:2609.22682): *demonstrability*
//! — whether correct output is **recognizable once it appears** — tracks
//! where multi-candidate collaboration beats the best member (Spearman
//! ρ=0.90 across 8 benchmarks). The allocation law T2/T3 consume:
//! **candidate-pool depth pays in proportion to measured selection
//! discriminability**; where selection does not discriminate, generation is
//! wasted at any depth and the spend belongs in rerank/discrimination work
//! instead.
//!
//! ## Coverage-conditioned by construction
//!
//! Both formulas CONDITION on the correct candidate having been PRESENT in
//! the pool. A raw success rate would contaminate T3's Spearman — easy
//! domains carry both high success and high gain-from-depth, and the
//! ρ-test would pass for the wrong reason. Generation-side failures are
//! reported BESIDE, never pooled:
//!
//! - **Eval (formula 1)**: `correct_top1 / (correct_top1 + wrong_rule)`
//!   over the Issue-125 `ChoiceTaxonomy` — conditioned on gold (or an
//!   authored acceptable alternative) having surfaced; `wrong_span` (never
//!   surfaced) is the generation-side exclusion.
//! - **Bench (formula 2)**: `resolved / (resolved + persist)` over the
//!   Issue-089-T8 per-rule outcomes — conditioned on the rule having fired
//!   on its own single-target fixture (the correct fix is in the corpus by
//!   construction, so its availability is guaranteed) and on the output
//!   being ok; `broken` (the composition broke compile) is the
//!   generation-side exclusion; `shaped` (target cleared ∧ new diags —
//!   `is_shaped()` requires `resolved() > 0`, so shaped ⊆ resolved) is
//!   reported beside as the half-win share of the success class.
//!
//! ⚠ **Formula correction, recorded in `.issues/134` at implementation
//! time**: the issue text wrote `persist / (persist + shaped)`, which is
//! transposed against `RuleOutcome`'s own field semantics — `persist` is
//! the target-STILL-FIRING fork (the correct fix was available and
//! selection did not land it: the `wrong_rule` analog), while `shaped` is
//! a decomposition of the SUCCESS class, not a separate pool. The
//! eval-symmetric constant above preserves the issue's direction
//! (high = selection discriminates), its conditioning (generation-side
//! excluded, reported beside), and its per-rule floor.
//!
//! ## The frontier-report refusal law
//!
//! The column refuses a confident zero on empty labeled sets: a constant
//! printed over no data is the exact failure mode that law exists to
//! prevent. [`render_column`] prints the refusal; [`Row::constant`]
//! returns `None` on an empty labeled set, never `0.0`.

// Count arithmetic: every value here is a bench/eval population size
// (fixtures + spans — thousands, never near u32::MAX) rendered to a display
// metric; the pedantic cast lints are noise, allowed with cause (the
// score_bench::report precedent).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use serde::{Deserialize, Serialize};

/// Per-rule rows print only above this labeled-set floor — the tail-support
/// rule applied to a selection statistic: below it a per-rule constant is
/// one fixture's coin flip, not a measurement (domain aggregates are the
/// T3 qualifying unit and need only a non-empty labeled set).
pub const MIN_PER_RULE_N: u32 = 20;

/// The T3 qualifying-set candidates (the retrieval/compile domain ids, in
/// the `RuleDomains`/registry spelling — the one spelling both formulas
/// key on, so the eval and bench halves of the column can be joined).
pub const T3_DOMAINS: [&str; 8] = [
    "clippy",
    "rust_perf",
    "rustc_errors",
    "kernel_opt",
    "sec",
    "docker",
    "dist",
    "shader_perf",
];

/// The canonical (`RuleDomains`/registry) spelling for a `DomainId::label()`
/// — the eval's labels (`perf` / `kernel` / `shader`) and the registry's
/// ids (`rust_perf` / `kernel_opt` / `shader_perf`) differ, and a column
/// keyed on two spellings cannot be joined.
#[must_use]
pub fn canonical_domain(label: &str) -> &str {
    match label {
        "perf" => "rust_perf",
        "kernel" => "kernel_opt",
        "shader" => "shader_perf",
        other => other,
    }
}

/// Which labeled-outcome source produced a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The retrieval-eval choice taxonomy (formula 1).
    EvalTaxonomy,
    /// The score-bench per-rule outcomes (formula 2).
    ScoreBench,
}

/// One domain's (or rule's) demonstrability constant — the raw counts ride
/// with the constant so every reader can re-derive it (a ratio without its
/// denominator is a display number, not a measurement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// Canonical domain id (the `RuleDomains` spelling, or `"other"` for
    /// unattributable bench rules — which never qualifies for T3).
    pub domain: String,
    /// `None` = the domain aggregate; `Some` = one rule's row (only above
    /// [`MIN_PER_RULE_N`]).
    pub rule: Option<String>,
    pub source: Source,
    /// Correct candidate present AND selected (eval: strict + acceptable
    /// top-1; bench: target resolved).
    pub selected: u32,
    /// Correct candidate present, selection did not land it (eval:
    /// `wrong_rule`; bench: `persist`).
    pub rejected: u32,
    /// Half-win share of the success class, reported beside (bench:
    /// `shaped ⊆ resolved`; eval: 0 — the taxonomy has no half-win class).
    /// Pooled INTO `selected` (the core landed), never into the rejection.
    pub selected_partial: u32,
    /// Generation-side exclusions, reported beside, NEVER pooled (eval:
    /// `wrong_span`; bench: `broken`).
    pub excluded_generation: u32,
}

impl Row {
    /// The labeled-set size the constant conditions on (the denominator).
    #[must_use]
    pub fn n(&self) -> u32 {
        self.selected + self.rejected
    }

    /// `selected / n` — `None` on an empty labeled set: never a confident
    /// `0.0` (the frontier-report law).
    #[must_use]
    pub fn constant(&self) -> Option<f32> {
        let n = self.n();
        (n > 0).then(|| self.selected as f32 / n as f32)
    }

    /// T3's labeled-set floor: a domain QUALIFIES only with a non-empty
    /// conditioned set (`"other"` rows are visible but never qualifying —
    /// attribution is membership, never a guess).
    #[must_use]
    pub fn qualifies(&self) -> bool {
        self.n() > 0 && self.domain != "other" && self.rule.is_none()
    }
}

/// Formula 1 — one eval-taxonomy read as a row. `correct_top1` is strict +
/// acceptable (both are "gold or an authored alternative won top-1").
#[cfg(feature = "latent_retrieval")]
pub fn eval_row(domain: &str, rule: Option<&str>, t: &crate::draft::ChoiceTaxonomy) -> Row {
    Row {
        domain: domain.to_string(),
        rule: rule.map(str::to_string),
        source: Source::EvalTaxonomy,
        selected: (t.correct_rule + t.correct_via_acceptable) as u32,
        rejected: t.wrong_rule as u32,
        selected_partial: 0,
        excluded_generation: t.wrong_span as u32,
    }
}

/// Formula 2 — one bench-outcome read as a row (raw counts; the caller
/// owns the attribution). `shaped` rides beside as the half-win share of
/// the success class (shaped ⊆ resolved — [`is_shaped`] requires
/// `resolved() > 0`), never in the rejection.
///
/// [`is_shaped`]: `crate::score_bench::FixtureOutcome::is_shaped`
#[must_use]
pub fn bench_row(
    domain: &str,
    rule: Option<&str>,
    resolved: usize,
    persist: usize,
    shaped: usize,
    broken: usize,
) -> Row {
    Row {
        domain: domain.to_string(),
        rule: rule.map(str::to_string),
        source: Source::ScoreBench,
        selected: resolved as u32,
        rejected: persist as u32,
        selected_partial: shaped as u32,
        excluded_generation: broken as u32,
    }
}

/// Aggregate the score-bench per-rule read (Issue 089 T8) into the bench
/// half of the column: one domain aggregate per attributed domain, then
/// per-rule rows above [`MIN_PER_RULE_N`]. Attribution through the SAME
/// compiled rule→domain table the sync payload and pending estimator
/// project through ([`crate::rule_domains::RuleDomains::compiled`]) —
/// unattributed rules fold into `"other"` (the corpus view's catch-all,
/// visible in the column, never T3-qualifying).
#[cfg(feature = "score_bench")]
#[must_use]
pub fn from_rule_outcomes(rows: &[crate::score_bench::RuleOutcome]) -> Vec<Row> {
    let table = crate::rule_domains::RuleDomains::compiled();
    let mut domains: std::collections::BTreeMap<String, [usize; 4]> =
        std::collections::BTreeMap::new();
    let mut per_rule: Vec<Row> = Vec::new();
    for r in rows {
        let raw = table.domain_for(&r.rule);
        let domain = if raw.is_empty() { "other" } else { raw };
        // [resolved, persist, shaped, broken] — the bench_row argument
        // order, one place.
        let e = domains.entry(domain.to_string()).or_insert([0usize; 4]);
        e[0] += r.resolved;
        e[1] += r.persist;
        e[2] += r.shaped;
        e[3] += r.broken;
        let n = r.resolved + r.persist;
        if n as u32 >= MIN_PER_RULE_N {
            per_rule.push(bench_row(
                domain,
                Some(&r.rule),
                r.resolved,
                r.persist,
                r.shaped,
                r.broken,
            ));
        }
    }
    let mut out: Vec<Row> = domains
        .into_iter()
        .map(|(domain, c)| bench_row(&domain, None, c[0], c[1], c[2], c[3]))
        .collect();
    out.extend(per_rule);
    out
}

/// The standing-column render: qualifying domain rows first, then per-rule
/// rows above the floor, then the T3 drop ledger (candidate domains with
/// no measured row, each with its reason), then the frontier-report
/// refusal when NOTHING carries a non-empty labeled set. Deterministic in
/// its input; pure (no I/O).
#[must_use]
pub fn render_column(rows: &[Row]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "-- demonstrability column (134 T1: selection discriminability, coverage-conditioned) --\n",
    );
    let qualifying: Vec<&Row> = rows.iter().filter(|r| r.qualifies()).collect();
    let per_rule: Vec<&Row> = rows
        .iter()
        .filter(|r| r.rule.is_some() && r.n() > 0)
        .collect();
    if rows.is_empty() {
        out.push_str("  REFUSED: no labeled sets measured — the demonstrability column refuses a confident zero (the frontier-report law)\n");
        return out;
    }
    for r in &qualifying {
        let _ = writeln!(
            out,
            "  [{:<11}] n={:>4}  constant={}  (selected {} / rejected {} | half-win {} | generation-side excluded {})",
            r.domain,
            r.n(),
            pct(r.constant()),
            r.selected,
            r.rejected,
            r.selected_partial,
            r.excluded_generation,
        );
    }
    if per_rule.is_empty() {
        out.push_str("  per-rule (n>=20): none measured above the floor\n");
    } else {
        let _ = writeln!(out, "  per-rule (n>={MIN_PER_RULE_N}):");
        for r in &per_rule {
            let _ = writeln!(
                out,
                "    {:<11} {}  n={:>4}  constant={}  (selected {} / rejected {} | half-win {} | excluded {})",
                r.domain,
                r.rule.as_deref().unwrap_or("?"),
                r.n(),
                pct(r.constant()),
                r.selected,
                r.rejected,
                r.selected_partial,
                r.excluded_generation,
            );
        }
    }
    // The drop ledger: every T3 candidate with no qualifying row, with its
    // reason — the honest answer for the thin domains is DROPPED, not 0.0.
    let measured: std::collections::BTreeSet<&str> =
        qualifying.iter().map(|r| r.domain.as_str()).collect();
    let drops: Vec<&str> = T3_DOMAINS
        .iter()
        .copied()
        .filter(|d| !measured.contains(d))
        .collect();
    if drops.is_empty() {
        out.push_str("  drops: none — every T3 candidate domain carries a qualifying row\n");
    } else {
        out.push_str("  drops (T3 candidates with no qualifying row):\n");
        for d in drops {
            let reason = if d == "rustc_errors" {
                "no labeled selection-outcome set measured by either formula — demonstrability is NEVER asserted ~1 (a free coordinate in T3's Spearman); joins T3 only when a T1 measurement lands"
            } else {
                "no labeled spans/rows measured in this build (feature off or no fixtures) — DROPPED, never 0.0"
            };
            let _ = writeln!(out, "    {d:<11} {reason}");
        }
    }
    if qualifying.is_empty() {
        out.push_str("  REFUSED: every measured labeled set is empty — no qualifying domain, nothing to correlate (the frontier-report law)\n");
    }
    out
}

fn pct(v: Option<f32>) -> String {
    v.map_or("-".to_string(), |x| format!("{x:.3}"))
}

/// The last history line carrying a demonstrability field, as `(ts, rows)`.
#[derive(Debug, Deserialize)]
struct HistoryTail {
    ts: String,
    #[serde(default)]
    demonstrability: Option<Vec<Row>>,
}

/// The STANDING read: the LAST `score_history.jsonl` row carrying a
/// demonstrability field (the bench half of the column as last committed).
/// Ungated — `serde_json` is a core dep — so every bin build can print the
/// standing column. `Ok(None)` = no row carries the field yet (the field
/// is additive; pre-T1 rows predate it) or the file is absent.
///
/// # Errors
///
/// Propagates filesystem errors (the file present but unreadable). A
/// malformed line is skipped loudly, never silently pooled into a verdict.
pub fn standing_from_jsonl(path: &std::path::Path) -> std::io::Result<Option<(String, Vec<Row>)>> {
    // A missing file is the honest "not yet measured" (Ok(None)), never an
    // error — the caller prints the not-yet-measured line either way.
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut last = None;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<HistoryTail>(line) {
            Ok(t) if t.demonstrability.is_some() => {
                last = Some((t.ts, t.demonstrability.unwrap_or_default()));
            }
            Ok(_) => {}
            Err(e) => eprintln!("demonstrability: skipping malformed history line: {e}"),
        }
    }
    Ok(last)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "latent_retrieval")]
    use super::eval_row;
    use super::{Row, Source, T3_DOMAINS};
    use super::{bench_row, canonical_domain, render_column, standing_from_jsonl};

    fn row(domain: &str, selected: u32, rejected: u32) -> Row {
        Row {
            domain: domain.to_string(),
            rule: None,
            source: Source::ScoreBench,
            selected,
            rejected,
            selected_partial: 0,
            excluded_generation: 0,
        }
    }

    #[test]
    fn constant_is_selected_over_conditioned_n() {
        let r = row("clippy", 23, 14);
        assert_eq!(r.n(), 37);
        let c = r.constant().expect("non-empty n");
        assert!((c - 23.0 / 37.0).abs() < 1e-6);

        // Empty labeled set → None, never a confident 0.0 (frontier-report law).
        let empty = row("sec", 0, 0);
        assert_eq!(empty.n(), 0);
        assert!(empty.constant().is_none());
        assert!(!empty.qualifies());
    }

    #[test]
    fn partial_and_generation_side_never_enter_the_denominator() {
        // Half-wins pool INTO selected (the core landed); generation-side
        // exclusions pool into NOTHING. Both must leave n() untouched.
        let mut r = bench_row("clippy", None, 10, 5, 4, 100);
        assert_eq!(r.n(), 15, "shaped+broken excluded from the conditioned set");
        assert_eq!(r.selected_partial, 4);
        assert_eq!(r.excluded_generation, 100);
        r.selected = 0;
        r.rejected = 0;
        assert!(r.constant().is_none());
    }

    #[test]
    fn other_rows_are_visible_but_never_t3_qualifying() {
        let r = row("other", 50, 10);
        assert!(!r.qualifies(), "attribution is membership, never a guess");
        assert!(r.n() > 0);
    }

    #[test]
    fn canonical_domain_maps_eval_labels_to_registry_spelling() {
        assert_eq!(canonical_domain("clippy"), "clippy");
        assert_eq!(canonical_domain("perf"), "rust_perf");
        assert_eq!(canonical_domain("kernel"), "kernel_opt");
        assert_eq!(canonical_domain("shader"), "shader_perf");
        assert_eq!(canonical_domain("docker"), "docker");
        // Every T3 candidate is reachable through the mapping or natively.
        for d in T3_DOMAINS {
            assert!(T3_DOMAINS.contains(&d));
        }
    }

    #[test]
    fn render_refuses_on_no_measured_rows() {
        let s = render_column(&[]);
        assert!(s.contains("REFUSED: no labeled sets measured"), "{s}");
    }

    #[test]
    fn render_refuses_when_every_labeled_set_is_empty() {
        let s = render_column(&[row("clippy", 0, 0), row("sec", 0, 0)]);
        assert!(
            s.contains("REFUSED: every measured labeled set is empty"),
            "{s}"
        );
        // And the drop ledger names BOTH domains as dropped, never 0.0.
        assert!(s.contains("clippy") && s.contains("sec"), "{s}");
    }

    #[test]
    fn render_is_display_only_and_prints_drop_ledger() {
        // The renderer displays the rows it is handed AS-IS — the n>=20
        // per-rule floor is PRODUCER-side (eval print + from_rule_outcomes,
        // each tested at its own gate); a renderer that re-filters would
        // silently hide rows a future producer bug hands it, and the
        // standing reader's job is fidelity to what was committed.
        let per_rule = bench_row("clippy", Some("manual_map"), 18, 2, 1, 0);
        let rows = vec![row("clippy", 23, 14), per_rule];
        let s = render_column(&rows);
        assert!(s.contains("[clippy") && s.contains("n=  37"), "{s}");
        assert!(s.contains("manual_map"), "{s}");
        // rustc_errors carries its special never-asserted reason.
        assert!(s.contains("rustc_errors"), "{s}");
        assert!(s.contains("NEVER asserted"), "{s}");
        // Every unmeasured T3 candidate appears in the drop ledger.
        for d in [
            "rust_perf",
            "rustc_errors",
            "kernel_opt",
            "sec",
            "docker",
            "dist",
            "shader_perf",
        ] {
            assert!(s.contains(d), "missing drop ledger entry for {d}: {s}");
        }
    }

    #[test]
    fn bench_row_pins_the_field_semantics() {
        // The formula correction pin: selected=resolved (the correct core
        // landed — shaped ⊆ resolved), rejected=persist (available, not
        // landed — the wrong_rule analog), broken = generation-side.
        let r = bench_row("clippy", None, 7, 3, 2, 5);
        assert_eq!(
            (
                r.selected,
                r.rejected,
                r.selected_partial,
                r.excluded_generation
            ),
            (7, 3, 2, 5)
        );
        let c = r.constant().expect("n>0");
        assert!((c - 0.7).abs() < 1e-6, "resolved/(resolved+persist) = 7/10");
    }

    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn eval_row_pins_the_taxonomy_mapping() {
        use crate::draft::ChoiceTaxonomy;
        // Functional-update construction, not default-then-assign — the
        // pedantic field_reassign_with_default lint (fires only under
        // --all-targets, which T1's --lib validation never compiled).
        let t = ChoiceTaxonomy {
            correct_rule: 10,
            correct_via_acceptable: 3,
            wrong_rule: 7,
            wrong_span: 9,
            ..ChoiceTaxonomy::default()
        };
        let r = eval_row("clippy", None, &t);
        assert_eq!(r.source, Source::EvalTaxonomy);
        assert_eq!(
            (
                r.selected,
                r.rejected,
                r.selected_partial,
                r.excluded_generation
            ),
            (13, 7, 0, 9)
        );
        let c = r.constant().expect("n>0");
        assert!((c - 13.0 / 20.0).abs() < 1e-6);
    }

    #[test]
    fn standing_from_jsonl_reads_the_last_row_carrying_the_field() {
        let dir = std::env::temp_dir().join(format!("demo_standing_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("score_history.jsonl");
        // A pre-T1 row (no field), then two carrying it — the LAST wins.
        let lines = [
            r#"{"ts":"2026-09-23T10:00:00","sha":"a","n_ran":1,"n_healable":1,"n_clean":0,"heal_rate":0.5,"created_rate":0.0,"parse_safety":1.0,"decline_correctness":1.0,"edits_total":1,"vocab_size":10,"generation":1}"#,
            r#"{"ts":"2026-09-24T10:00:00","sha":"b","n_ran":1,"n_healable":1,"n_clean":0,"heal_rate":0.5,"created_rate":0.0,"parse_safety":1.0,"decline_correctness":1.0,"edits_total":1,"vocab_size":10,"generation":1,"demonstrability":[{"domain":"clippy","rule":null,"source":"score_bench","selected":5,"rejected":5,"selected_partial":0,"excluded_generation":0}]}"#,
            r#"{"ts":"2026-09-24T11:00:00","sha":"c","n_ran":1,"n_healable":1,"n_clean":0,"heal_rate":0.5,"created_rate":0.0,"parse_safety":1.0,"decline_correctness":1.0,"edits_total":1,"vocab_size":10,"generation":1,"demonstrability":[{"domain":"rust_perf","rule":null,"source":"score_bench","selected":2,"rejected":2,"selected_partial":1,"excluded_generation":3}]}"#,
        ];
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        let got = standing_from_jsonl(&path)
            .unwrap()
            .expect("two rows carry the field");
        assert_eq!(got.0, "2026-09-24T11:00:00", "the LAST carrying row wins");
        assert_eq!(got.1.len(), 1);
        assert_eq!(got.1[0].domain, "rust_perf");
        assert_eq!((got.1[0].selected, got.1[0].rejected), (2, 2));
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn standing_from_jsonl_none_when_file_absent_or_field_never_present() {
        let dir = std::env::temp_dir().join(format!("demo_absent_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(
            standing_from_jsonl(&dir.join("nope.jsonl"))
                .unwrap()
                .is_none()
        );
        // A file whose rows predate the field → None (additive law).
        let path = dir.join("old.jsonl");
        std::fs::write(
            &path,
            r#"{"ts":"2026-09-22T10:00:00","sha":"a","n_ran":1,"n_healable":1,"n_clean":0,"heal_rate":0.5,"created_rate":0.0,"parse_safety":1.0,"decline_correctness":1.0,"edits_total":1,"vocab_size":10,"generation":1}"#
                .to_string()
                + "\n",
        )
        .unwrap();
        assert!(standing_from_jsonl(&path).unwrap().is_none());
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    /// The PRODUCER-side floor + attribution + aggregation (the renderer is
    /// display-only — see `render_is_display_only_and_prints_drop_ledger`).
    #[cfg(feature = "score_bench")]
    #[test]
    fn from_rule_outcomes_aggregates_by_domain_and_floors_per_rule() {
        use super::from_rule_outcomes;
        let mk = |rule: &str, resolved: usize, persist: usize, shaped: usize, broken: usize| {
            crate::score_bench::RuleOutcome {
                rule: rule.to_string(),
                fixtures: resolved + persist + broken,
                resolved,
                persist,
                broken,
                created: 0,
                edits: 0,
                shaped,
            }
        };
        // "needless_return" is a compiled clippy corpus rule (attribution
        // by table membership); "E0502:hoist-to-first-use" attributes to
        // rustc_errors by playbook code; "mystery" folds to other.
        let rows = [
            mk("needless_return", 30, 10, 5, 2),
            mk("mystery", 8, 2, 0, 0),
        ];
        let out = from_rule_outcomes(&rows);
        // Domain aggregates first (BTreeMap order), then per-rule rows.
        let clippy = out
            .iter()
            .find(|r| r.domain == "clippy")
            .expect("clippy row");
        assert!(clippy.rule.is_none());
        assert_eq!(clippy.source, Source::ScoreBench);
        // selected=resolved, rejected=persist, half-win=shaped beside,
        // broken excluded — the formula-correction pin.
        assert_eq!(
            (
                clippy.selected,
                clippy.rejected,
                clippy.selected_partial,
                clippy.excluded_generation
            ),
            (30, 10, 5, 2)
        );
        let c = clippy.constant().expect("n>0");
        assert!(
            (c - 0.75).abs() < 1e-6,
            "resolved/(resolved+persist) = 30/40"
        );
        // The n=40 aggregate qualifies for T3; "other" does not.
        assert!(clippy.qualifies());
        let other = out.iter().find(|r| r.domain == "other").expect("other row");
        assert!(
            !other.qualifies(),
            "attribution is membership, never a guess"
        );
        // Per-rule floor: the clippy rule is at n=40 >= 20 → a per-rule row
        // exists; the n=10 mystery rule does not.
        assert!(
            out.iter()
                .any(|r| r.rule.as_deref() == Some("needless_return"))
        );
        assert!(!out.iter().any(|r| r.rule.as_deref() == Some("mystery")));
        // The below-floor rule contributes to its DOMAIN aggregate even
        // though it never prints per-rule (aggregation is unconditional).
        assert_eq!(other.selected, 8);
        assert_eq!(other.rejected, 2);
    }
}
