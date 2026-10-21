//! Issue 098 T1+T2 — rule-corpus Cross-OOO axis driver (the measurement).
//!
//! Runs the absorption/splitting audit over the pooled real oracle-labeled
//! clippy spans (see `riir_refine::score_bench::cross_ooo`), plus the
//! per-domain population scan: how many rules in each domain's corpus even
//! HAVE ≥2 spans (the K ≥ 2 precondition for Cross-OOO). Verdict per Issue
//! 098 T3: pairs under 0.6 ⇒ absorption candidates (wiring); none ⇒ the
//! axis is hygiene-only (recorded negative).
//!
//! Offline instrumentation only — no heal path, no corpus writes, no
//! network. Run:
//!
//! ```text
//! cargo run --example cross_ooo_axis --features "score_bench kernel_opt sec docker" --release
//! ```

use std::collections::BTreeMap;

use katgpt_core::ooo_audit::OooAuditConfig;
use riir_refine::domain::HealEntry;
use riir_refine::score_bench::cross_ooo::{audit_clippy_corpus, render_report};

/// One domain's corpus population verdict (the K ≥ 2 precondition scan).
///
/// The per-domain blocks below are deliberately concrete (each corpus has a
/// different `D`): `rule_id()` is the one generic accessor they share.
struct Population {
    domain: &'static str,
    entries: usize,
    rules: usize,
    auditable: usize,
    max_spans: usize,
}

/// Fold one corpus's entries into the scan (the shared body of the blocks).
fn fold<'a, E: HealEntry + 'a>(
    domain: &'static str,
    entries: impl IntoIterator<Item = &'a E>,
    count: usize,
) -> Population {
    let mut per_rule: BTreeMap<&str, usize> = BTreeMap::new();
    for e in entries {
        *per_rule.entry(e.rule_id()).or_insert(0) += 1;
    }
    Population {
        domain,
        entries: count,
        rules: per_rule.len(),
        auditable: per_rule.values().filter(|c| **c >= 2).count(),
        max_spans: per_rule.values().copied().max().unwrap_or(0),
    }
}

fn main() {
    // ── population scan (per domain: the K ≥ 2 precondition) ──────────────
    let mut populations: Vec<Population> = Vec::new();
    {
        let corpus = riir_refine::build_seed_corpus();
        populations.push(fold(
            "clippy_lints (corpus)",
            &corpus.entries,
            corpus.entries.len(),
        ));
    }
    {
        let corpus = riir_refine::build_perf_seed_corpus();
        populations.push(fold(
            "rust_perf (corpus)",
            &corpus.entries,
            corpus.entries.len(),
        ));
    }
    #[cfg(feature = "kernel_opt")]
    {
        let corpus = riir_refine::build_kernel_seed_corpus();
        populations.push(fold(
            "kernel_opt (corpus)",
            &corpus.entries,
            corpus.entries.len(),
        ));
    }
    #[cfg(feature = "sec")]
    {
        let corpus = riir_refine::build_sec_seed_corpus();
        populations.push(fold("sec (corpus)", &corpus.entries, corpus.entries.len()));
    }
    #[cfg(feature = "docker")]
    {
        let corpus = riir_refine::build_docker_seed_corpus();
        populations.push(fold(
            "docker (corpus)",
            &corpus.entries,
            corpus.entries.len(),
        ));
    }

    println!("── per-domain corpus population (the K>=2 precondition) ──");
    println!(
        "{:<26}{:>9}{:>8}{:>11}{:>11}",
        "domain", "entries", "rules", "auditable", "max/rules"
    );
    for p in &populations {
        println!(
            "{:<26}{:>9}{:>8}{:>11}{:>11}",
            p.domain, p.entries, p.rules, p.auditable, p.max_spans
        );
    }
    println!(
        "(auditable = rules with >=2 corpus spans; the fixture-pool clippy \
         audit below adds the oracle-labeled fixture families on top)"
    );

    // ── the clippy fixture-pool audit (T1+T2) ─────────────────────────────
    let cfg = OooAuditConfig::default();
    let report = audit_clippy_corpus(&cfg);
    println!();
    println!("{}", render_report(&report));

    // ── verdict banner (T3) ────────────────────────────────────────────────
    if report.has_redundant_pairs() {
        println!(
            "VERDICT (T3): {} absorption candidate(s) at cross < {REDUNDANT} — wiring axis onward.",
            report.redundant_pairs.len(),
            REDUNDANT = riir_refine::score_bench::cross_ooo::REDUNDANT_PAIR_CROSS
        );
    } else {
        println!(
            "VERDICT (T3): no pair under {REDUNDANT} — the axis is hygiene-only at this pool \
             (recorded negative per the Issue 759 defend-wrong precedent).",
            REDUNDANT = riir_refine::score_bench::cross_ooo::REDUNDANT_PAIR_CROSS
        );
    }
}
