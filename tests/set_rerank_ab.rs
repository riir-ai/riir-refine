//! Issue 121 (katgpt-rs Plan 599 T4.2) — the counter-anchored set-rerank A/B.
//!
//! Arms over the SAME fixtures, judged by axes NONE of the arms read:
//!
//! - **oracle truth** — real `cargo clippy` labels for the composite
//!   multi-lint spans (the T3.5.5 uncontaminated discipline: the oracle
//!   knows nothing about our corpus, scores, or admission kernel);
//! - **collapse** — `detect_collapse`'s family-entropy verdict over the
//!   delivered slate (the issue's mandate: the detector stays the EVAL
//!   axis, never the guard).
//!
//! ```bash
//! cargo test --release --features set_rerank,clippy_lints,rust_perf,clippy_verify \
//!   --test set_rerank_ab -- --nocapture --test-threads=1
//! ```
//!
//! Deliberately NOT a committed regression floor (the uncontaminated_g1
//! doctrine): clippy's lint set moves between toolchains. It is the
//! measurement you run when you touch the set-rerank weights or the latent
//! choice; the verdict lands in the benchmark record (.benchmarks/096).

#![cfg(all(feature = "set_rerank", feature = "clippy_verify"))]

use std::collections::BTreeSet;
use std::time::Instant;

use riir_refine::domain::{HealDomain, HealEntry};
use riir_refine::draft::eval_fixtures_composite::COMPOSITE_SPANS;
use riir_refine::draft::set_rerank::{LatentChoice, SetRerankConfig, DEFAULT_SLATE_K};
use riir_refine::draft::{ClippyOracle, LatentMatcher, RerankMode, RERANK_POOL};

fn vocabulary() -> BTreeSet<String> {
    riir_refine::domains::clippy_lints::ClippyLintDomain::build_corpus()
        .into_iter()
        .map(|e| e.rule_id().to_string())
        .collect()
}

struct Truth {
    code: &'static str,
    /// Real clippy lints intersected with the corpus vocabulary.
    real: BTreeSet<String>,
}

fn gather_truth(oracle: &ClippyOracle) -> Vec<Truth> {
    let vocab = vocabulary();
    let mut out = Vec::with_capacity(COMPOSITE_SPANS.len());
    for (_tag, code) in COMPOSITE_SPANS {
        let v = oracle.lints_for(code);
        assert!(v.ran, "cargo clippy did not run: {:?}", v.error);
        if !v.compiles {
            continue;
        }
        let real = v.within_vocabulary(&vocab);
        if real.is_empty() {
            continue;
        }
        out.push(Truth { code, real });
    }
    out
}

/// What one arm returns per span — the pieces the eval axes need (owns its
/// ids; the A/B passes them across closures).
struct Slate {
    /// Rule ids, best first.
    ids: Vec<String>,
    /// The exact-Vendi certificate of the delivered slate.
    vendi: f32,
    /// The certificate's geometric collapse flag.
    cert_collapsed: bool,
    /// Whether the recovery ladder fired for this span.
    recovery_fired: bool,
}

#[derive(Default)]
struct ArmStats {
    n: usize,
    top1_any_true: usize,
    recall5_any_true: usize,
    /// Slates whose top-k is a single family while NO oracle label's family
    /// matches it — the multi-label collapse-with-oracle-outside.
    collapse_miss: usize,
    /// Slates whose top-k is a single family (the raw categorical signal).
    collapse_raw: usize,
    vendi_sum: f32,
    cert_collapsed: usize,
    recovery_fired: usize,
    ns_us: u128,
}

fn family(rule_id: &str) -> &str {
    riir_refine::draft::rule_family(rule_id)
}

fn pct(k: usize, d: usize) -> f32 {
    if d == 0 {
        0.0
    } else {
        k as f32 / d as f32 * 100.0
    }
}

fn score_arm(name: &str, truth: &[Truth], mut slate_of: impl FnMut(&str) -> Slate) -> ArmStats {
    let mut st = ArmStats::default();
    for t in truth {
        let t0 = Instant::now();
        let slate = slate_of(t.code);
        st.ns_us += t0.elapsed().as_micros();
        st.n += 1;
        if let Some(top) = slate.ids.first() {
            if t.real.contains(top.as_str()) {
                st.top1_any_true += 1;
            }
        }
        if slate
            .ids
            .iter()
            .take(DEFAULT_SLATE_K)
            .any(|id| t.real.contains(id.as_str()))
        {
            st.recall5_any_true += 1;
        }
        let take = DEFAULT_SLATE_K.min(slate.ids.len());
        let top_k: Vec<&str> = slate.ids[..take].iter().map(String::as_str).collect();
        if !top_k.is_empty() {
            let f0 = family(top_k[0]);
            if top_k.iter().all(|id| family(id) == f0) {
                st.collapse_raw += 1;
                let oracle_inside = t.real.iter().any(|label| family(label) == f0);
                if !oracle_inside {
                    st.collapse_miss += 1;
                }
            }
        }
        st.vendi_sum += slate.vendi;
        st.cert_collapsed += usize::from(slate.cert_collapsed);
        st.recovery_fired += usize::from(slate.recovery_fired);
    }
    let mean_vendi = if st.n == 0 {
        0.0
    } else {
        st.vendi_sum / st.n as f32
    };
    let us_per_span = if st.n == 0 {
        0.0
    } else {
        st.ns_us as f32 / st.n as f32
    };
    println!(
        "  {name:<26} n={:<3} top1 {:>5.1}% recall@5 {:>5.1}% | collapse raw {:<2} miss {:<2} | vendi {:.3} cert-coll {} recov {} | {:>6.1} us/span",
        st.n,
        pct(st.top1_any_true, st.n),
        pct(st.recall5_any_true, st.n),
        st.collapse_raw,
        st.collapse_miss,
        mean_vendi,
        st.cert_collapsed,
        st.recovery_fired,
        us_per_span,
    );
    st
}

#[test]
fn set_rerank_ab_composite() {
    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("SKIP: cargo not available");
        return;
    }
    let truth = gather_truth(&oracle);
    println!(
        "\n== Issue 121 set-rerank A/B — composite spans ({} scorable) ==",
        truth.len()
    );
    if truth.len() < 5 {
        // Not a floor — a refuse-to-guess guard (a verdict over a starving
        // rig is noise wearing a percentage).
        eprintln!(
            "SKIP: only {} scorable spans on this toolchain",
            truth.len()
        );
        return;
    }

    // Arm 1a — BASELINE: the shipped merged order (all_rules' key: score
    // desc, rule_id asc — the CURRENT merge, which discards the
    // instantiates flag), with the certificate read post-hoc.
    let mut m = LatentMatcher::from_enabled_domains().expect("build matcher");
    let baseline = score_arm("baseline (merged order)", &truth, |code| {
        let (_, by_domain) = m.suggest_span(code, RERANK_POOL);
        let mut all: Vec<(String, f32)> = by_domain
            .iter()
            .flat_map(|(_, rules)| rules.iter().map(|r| (r.rule_id.clone(), r.score)))
            .collect();
        all.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let ids: Vec<String> = all
            .iter()
            .take(DEFAULT_SLATE_K)
            .map(|(id, _)| id.clone())
            .collect();
        let cert = m.certify_slate_ids(&ids.iter().map(String::as_str).collect::<Vec<_>>());
        Slate {
            ids,
            vendi: cert.vendi,
            cert_collapsed: cert.collapsed,
            recovery_fired: false,
        }
    });

    // Arm 1b — CANONICAL MERGE CONTROL: the per-domain rerank's own key
    // (instantiates desc, score desc, rule_id asc) applied ACROSS domains —
    // the defend-wrong control for the set-rerank's structural bonus: if
    // this arm matches the set-rerank, the win is the merge-key fix, not
    // the counter-anchor.
    let canonical = score_arm("canonical merge (struct key)", &truth, |code| {
        let (_, by_domain) = m.suggest_span(code, RERANK_POOL);
        let mut all: Vec<(String, f32, bool)> = by_domain
            .iter()
            .flat_map(|(_, rules)| {
                rules
                    .iter()
                    .map(|r| (r.rule_id.clone(), r.score, r.instantiates))
            })
            .collect();
        all.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| b.1.total_cmp(&a.1))
                .then_with(|| a.0.cmp(&b.0))
        });
        let ids: Vec<String> = all
            .iter()
            .take(DEFAULT_SLATE_K)
            .map(|(id, _, _)| id.clone())
            .collect();
        let cert = m.certify_slate_ids(&ids.iter().map(String::as_str).collect::<Vec<_>>());
        Slate {
            ids,
            vendi: cert.vendi,
            cert_collapsed: cert.collapsed,
            recovery_fired: false,
        }
    });

    // Arms 2/3 — set rerank, both latent spaces. A matcher per arm keeps the
    // G4 scratch honest (no cross-arm reuse).
    let full_cfg = SetRerankConfig {
        latent: LatentChoice::Full8,
        ..SetRerankConfig::default()
    };
    let mut m2 = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_set_config(full_cfg);
    let full = score_arm("set-rerank Full8", &truth, |code| {
        let slate = m2.suggest_set_slate(code, DEFAULT_SLATE_K);
        Slate {
            ids: slate.admitted.iter().map(|r| r.rule_id.clone()).collect(),
            vendi: slate.certificate.vendi,
            cert_collapsed: slate.certificate.collapsed,
            recovery_fired: slate.recovery_fired,
        }
    });

    let struct_cfg = SetRerankConfig {
        latent: LatentChoice::Structural4,
        ..SetRerankConfig::default()
    };
    let mut m3 = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_set_config(struct_cfg);
    let structural = score_arm("set-rerank Structural4", &truth, |code| {
        let slate = m3.suggest_set_slate(code, DEFAULT_SLATE_K);
        Slate {
            ids: slate.admitted.iter().map(|r| r.rule_id.clone()).collect(),
            vendi: slate.certificate.vendi,
            cert_collapsed: slate.certificate.collapsed,
            recovery_fired: slate.recovery_fired,
        }
    });

    // The verdict lines the benchmark record cites. HONEST framing: an arm
    // that lowers collapse_miss while holding recall is the win shape; an
    // arm that lowers recall is a loss REGARDLESS of its vendi. The
    // canonical-merge column isolates the structural-key confound: credit
    // the counter-anchor only for what it beats THAT arm by.
    println!(
        "\n  verdict axes: collapse_miss {} -> {} (Full8) / {} (Struct4) | recall@5 {:.1}% -> {:.1}% / {:.1}% | canonical-merge control recall@5 {:.1}%",
        baseline.collapse_miss,
        full.collapse_miss,
        structural.collapse_miss,
        pct(baseline.recall5_any_true, baseline.n),
        pct(full.recall5_any_true, full.n),
        pct(structural.recall5_any_true, structural.n),
        pct(canonical.recall5_any_true, canonical.n),
    );

    // Structural pin (the one honest floor in this file): the set-rerank
    // must not SINK recall@5 below the baseline on this rig — an arm that
    // evicts true rules to buy diversity loses under the issue's own
    // ordering (collapse drops AND oracle-recall rises). Allow the 1-span
    // tie-break noise floor.
    assert!(
        full.recall5_any_true + 1 >= baseline.recall5_any_true,
        "Full8 arm lost recall@5: {} -> {}",
        baseline.recall5_any_true,
        full.recall5_any_true
    );
    assert!(
        structural.recall5_any_true + 1 >= baseline.recall5_any_true,
        "Structural4 arm lost recall@5: {} -> {}",
        baseline.recall5_any_true,
        structural.recall5_any_true
    );
}

#[test]
fn set_rerank_ood_withheld_pair() {
    use riir_refine::draft::run_withheld_pair;
    use riir_refine::draft::run_withheld_pair_set_admission;

    let pop = riir_refine::draft::withheld_pair_ood::clippy_population();
    println!("\n== Issue 121 OOD rig — clippy_population, pure retrieval arms ==");

    let base = run_withheld_pair(
        pop.clone(),
        None,
        RerankMode::Off,
        "shipped (pure retrieval)",
    );
    println!(
        "  shipped                 ID top-5 {:>5.1}%  OOD top-5 {:>5.1}%  NOROLE top-5 {:.1}%",
        base.id_topk(5),
        base.ood_topk(5),
        base.norole_topk(5)
    );

    // The alpha ablation (R4T's own ablation law): the pure-retrieval rig
    // carries NO structural evidence, so quality is min-max score alone and
    // the alignment anchor operates in the hash-noise space — dropping
    // alpha isolates whether the regression is the alignment anchor or the
    // diversity anchor. Recorded whatever the outcome.
    for (label, latent, alpha) in [
        ("set Full8  a=0.6", LatentChoice::Full8, 0.6),
        ("set Struct4 a=0.6", LatentChoice::Structural4, 0.6),
        ("set Full8  a=0.2", LatentChoice::Full8, 0.2),
        ("set Full8  a=0.0", LatentChoice::Full8, 0.0),
    ] {
        let cfg = SetRerankConfig {
            latent,
            admission: katgpt_core::set_admission::SetAdmissionConfig {
                alpha_align: alpha,
                ..katgpt_core::set_admission::SetAdmissionConfig::default()
            },
            ..SetRerankConfig::default()
        };
        let set = run_withheld_pair_set_admission(pop.clone(), cfg, DEFAULT_SLATE_K, "set arm");
        println!(
            "  {label:<24} ID top-5 {:>5.1}%  OOD top-5 {:>5.1}%  NOROLE top-5 {:.1}%",
            set.id_topk(5),
            set.ood_topk(5),
            set.norole_topk(5)
        );
        // NOROLE must stay 0 under every arm — the admission cannot invent
        // the absent role (every candidate grounds to a corpus rule and the
        // role is gone from the corpus).
        assert_eq!(
            set.norole_topk(1),
            0.0,
            "set admission leaked the withheld ROLE (norole canary, {label})"
        );
    }
}
