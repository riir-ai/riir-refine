//! Issue 097 — linking↔retrieval-confusion PoC (defend-wrong).
//!
//! Hypothesis (the Ren & Lim CIFAR-10 analogue translated to the healer
//! surface): two lint rules whose span-embedding clouds (8-D
//! `ModellessEmbedder`, joint PCA-3D) are topologically LINKED are the rule
//! pairs the retriever confuses. The paper measured linking ↔ classifier
//! confusion at Spearman r ≈ 0.48 on CIFAR-10; this harness measures the
//! healer-surface version on the oracle-labeled retrieval fixtures
//! (`CLIPPY_SPANS` + `PERF_SPANS` — the same span canon `retrieval_eval`
//! measures; `labeled_eval_spans()` itself is `pub(crate)`).
//!
//! Design (pre-registered in `.issues/097`):
//! - **Clouds**: per labeled fixture span, the base span + M salted
//!   shape-preserving rename variants (`token` → `token_v{salt}`, keywords
//!   untouched) → one cloud per rule. CAVEAT carried into the verdict: these
//!   are RENAME ORBITS (1 exemplar/rule at fixture scale), not the paper's
//!   multi-exemplar class manifolds.
//! - **Linking**: `katgpt_core::linking_fold::detect_linking` per
//!   within-domain rule pair (joint PCA-3D internal, d = LATENT_DIM).
//! - **Ground truth, two axes**: (a) base-span confusion (label vs top-1 —
//!   the fixture canon, sparse); (b) variant-level confusion rate (each
//!   variant's top-1 is evidence — denser, includes the base span).
//! - **Stats**: Fisher exact (one-tailed enrichment) on the linked×confused
//!   2×2; Spearman ρ(|link| ordinal, pair confusion rate) over all pairs;
//!   enrichment ratio (confusion mass on linked vs unlinked pairs).
//! - **Verdict**: GO = enrichment with p < 0.05 on ≥1 domain, coherent on
//!   both axes; NO-GO = no enrichment (recorded negative, orbit-manifold
//!   caveat); UNDERPOWERED = too few confusion events to test at all.
//!
//! Measurement-only: no heal-path, no router, no corpus writes. Offline
//! instrumentation per the `gw_corpus` precedent.
//!
//! ```text
//! cargo run --example linking_confusion_poc --features linking_confusion_poc --release
//! ```

use std::collections::BTreeMap;

use katgpt_core::linking_fold::{detect_linking, LinkingDetectorConfig};
use riir_refine::draft::domain_router::DomainId;
use riir_refine::draft::eval_fixtures_clippy::CLIPPY_SPANS;
use riir_refine::draft::eval_fixtures_perf::PERF_SPANS;
use riir_refine::draft::latent_matcher::LatentMatcher;
use riir_refine::draft::retrieval_eval::canonical_rank;
use riir_refine::draft::{RetrievalConfig, RerankMode};
use riir_rag::{Embedder, ModellessEmbedder};

/// Variants per rule cloud (salt 1..=M + the base span = M+1 points).
/// Must keep n ≥ k_neighbors + 1 (k=8 default → ≥9); 33 comfortably clears.
const M_VARIANTS: usize = 32;

/// Probe depth for the confusion ground truth — mirrors retrieval_eval's
/// PROBE_TOP_K (private there; duplicated here deliberately, one line).
const TOP_K: usize = 8;

/// Minimum cloud size to join the pair sweep (defensive — every fixture
/// rule clears this at M_VARIANTS=32).
const MIN_CLOUD: usize = 9;

// ─────────────────────────────────────────────────────────────────────────────
// Salted shape-preserving rename: `token` → `token_v{salt}`
// ─────────────────────────────────────────────────────────────────────────────

/// Rust keywords + the common type/literal names in the fixture pseudocode.
/// NEVER suffixed — they carry the structure the embedder and the retriever
/// both read; only value-identifier names move (the mutate_shape premise,
/// salted for multiplicity).
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "yield",
    "bool", "char", "str", "String", "Vec", "Option", "Some", "None", "Ok", "Err", "Result",
    "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize", "f32", "f64",
];

/// Deterministic shape-preserving rename with a variant salt: every
/// non-keyword identifier token gets `_v{salt}` appended. Same token → same
/// suffix (consistent within a variant), distinct salts → distinct embedding
/// points, relative token structure preserved across variants.
fn salted_variant(text: &str, salt: usize) -> String {
    let suffix = format!("_v{salt}");
    let mut out = String::with_capacity(text.len() + 64);
    let mut token = String::new();
    let flush = |out: &mut String, token: &mut String| {
        if !token.is_empty() {
            out.push_str(token);
            if !KEYWORDS.contains(&token.as_str()) {
                out.push_str(&suffix);
            }
            token.clear();
        }
    };
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            flush(&mut out, &mut token);
            out.push(ch);
        }
    }
    flush(&mut out, &mut token);
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Stats helpers (the local-helper convention — Spearman exists only as
// private per-module helpers workspace-wide; substrate check in the issue)
// ─────────────────────────────────────────────────────────────────────────────

/// Spearman rank correlation with average ranks for ties (the gw_alignment
/// tests.rs helper shape).
fn ranks_of(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&i, &j| v[i].total_cmp(&v[j]));
    let mut r = vec![0.0_f64; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            r[idx[k]] = avg;
        }
        i = j + 1;
    }
    r
}

fn spearman(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    if n < 2 {
        return f64::NAN;
    }
    let ra = ranks_of(x);
    let rb = ranks_of(y);
    let mut d2 = 0.0;
    for i in 0..n {
        let d = ra[i] - rb[i];
        d2 += d * d;
    }
    1.0 - (6.0 * d2) / (n as f64 * (n as f64 - 1.0) * (n as f64 + 1.0))
}

/// ln(n!) by summation (n ≤ a few thousand here — exact enough in f64).
fn ln_fact(n: usize) -> f64 {
    let mut s = 0.0;
    for k in 2..=n {
        s += (k as f64).ln();
    }
    s
}

/// One-tailed Fisher exact P(right-tail) for the 2×2
/// ```text
///            confused   not
/// linked        a        b
/// unlinked      c        d
/// ```
fn fisher_right_tail(a: usize, b: usize, c: usize, d: usize) -> f64 {
    let row1 = a + b;
    let col1 = a + c;
    let n = a + b + c + d;
    let denom =
        ln_fact(row1) + ln_fact(n - row1) + ln_fact(col1) + ln_fact(n - col1) - ln_fact(n);
    let log_p_at = |x: usize| -> f64 {
        ln_fact(x) + ln_fact(row1 - x) + ln_fact(col1 - x) + ln_fact(n - row1 - col1 + x) - denom
    };
    let hi = row1.min(col1);
    let mut total = 0.0;
    let p_obs = log_p_at(a);
    for x in a..=hi {
        let lp = log_p_at(x);
        if lp <= p_obs + 1e-9 {
            total += lp.exp();
        }
    }
    total.min(1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// The PoC
// ─────────────────────────────────────────────────────────────────────────────

struct RuleCloud {
    /// Flat [n, d] embedding points (base first, then salted variants).
    points: Vec<f32>,
    n: usize,
}

struct PairResult {
    a: String,
    b: String,
    link: i32,
    /// fraction of a∪b (variants + base) whose top-1 is the OTHER rule.
    confusion_rate: f64,
    /// base-span confusion either direction.
    base_confused: bool,
    /// confusion events (variant top-1 ≠ own rule) on either side.
    events: usize,
}

fn main() {
    let embedder = ModellessEmbedder;
    let dim = riir_rag::query::LATENT_DIM;

    // The span canon: the two pub fixture consts (the kernel probes are
    // kernel_opt-gated and left out of this PoC by design).
    let canon: Vec<(DomainId, &str, &str)> = CLIPPY_SPANS
        .iter()
        .map(|(l, c)| (DomainId::Clippy, *l, *c))
        .chain(PERF_SPANS.iter().map(|(l, c)| (DomainId::Perf, *l, *c)))
        .collect();
    println!(
        "linking_confusion_poc (Issue 097): {} labeled spans, dim={dim}, M={M_VARIANTS}",
        canon.len()
    );

    // ── Matcher exactly as run_eval_inner builds it ────────────────────────
    let mut matcher = LatentMatcher::from_enabled_domains_with(RetrievalConfig::default())
        .expect("build matcher")
        .with_rerank_mode(RerankMode::default());

    // ── Group spans by (domain, rule) → clouds ────────────────────────────
    let mut grouped: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (domain, label, code) in &canon {
        if !matcher.has_domain(*domain) {
            continue;
        }
        grouped
            .entry((domain.label().to_string(), (*label).to_string()))
            .or_default()
            .push((*code).to_string());
    }

    let mut clouds: BTreeMap<(String, String), RuleCloud> = BTreeMap::new();
    for (key, codes) in &grouped {
        let mut points = Vec::with_capacity((M_VARIANTS + 1) * codes.len() * dim);
        for c in codes {
            points.extend_from_slice(&embedder.embed(c)[..]);
        }
        for salt in 1..=M_VARIANTS {
            for c in codes {
                let t = salted_variant(c, salt);
                points.extend_from_slice(&embedder.embed(&t)[..]);
            }
        }
        let n = points.len() / dim;
        clouds.insert(
            key.clone(),
            RuleCloud { points, n },
        );
    }
    println!(
        "clouds: {} rules (min n = {})",
        clouds.len(),
        clouds.values().map(|c| c.n).min().unwrap_or(0)
    );

    // ── Variant-level retrieval: confusion events per rule pair ───────────
    let mut conf_events: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    let mut own_queries: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut base_top1: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut total_queries = 0_usize;
    let mut total_correct = 0_usize;

    for (key @ (dom_s, label), codes) in &grouped {
        let domain = domain_of(dom_s);
        let cloud_n = clouds[key].n;
        if cloud_n < MIN_CLOUD {
            continue;
        }
        // base spans: the strict canon axis
        for c in codes {
            let (_routed, by_domain) = matcher.suggest_span(c, TOP_K);
            let top1 = top1_of(&by_domain, domain);
            if let Some(t) = &top1 {
                if t != label {
                    *conf_events
                        .entry((dom_s.clone(), label.clone(), t.clone()))
                        .or_default() += 1;
                } else {
                    total_correct += 1;
                }
                total_queries += 1;
            }
            base_top1.insert(key.clone(), top1.unwrap_or_default());
        }
        *own_queries.entry(key.clone()).or_default() += codes.len();
        // salted variants
        for salt in 1..=M_VARIANTS {
            for c in codes {
                let t = salted_variant(c, salt);
                let (_routed, by_domain) = matcher.suggest_span(&t, TOP_K);
                if let Some(top) = top1_of(&by_domain, domain) {
                    if &top != label {
                        *conf_events
                            .entry((dom_s.clone(), label.clone(), top.clone()))
                            .or_default() += 1;
                    } else {
                        total_correct += 1;
                    }
                    total_queries += 1;
                }
                *own_queries.entry(key.clone()).or_default() += 1;
            }
        }
    }
    println!(
        "variant retrieval: {} queries, top1-self rate {:.1}%",
        total_queries,
        100.0 * total_correct as f64 / total_queries.max(1) as f64
    );

    // ── Pair sweep: linking per within-domain rule pair ───────────────────
    let detector = LinkingDetectorConfig::default();
    let domains: Vec<String> = grouped
        .keys()
        .map(|(d, _)| d.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut results: Vec<(String, PairResult)> = Vec::new();
    let mut sweep_pairs = 0_usize;
    let mut sweep_ms = 0_u128;
    for dom_s in &domains {
        let mut rules: Vec<String> = clouds
            .iter()
            .filter(|((d, _), c)| d == dom_s && c.n >= MIN_CLOUD)
            .map(|((_, l), _)| l.clone())
            .collect();
        rules.sort();
        for i in 0..rules.len() {
            for j in (i + 1)..rules.len() {
                let (a, b) = (rules[i].as_str(), rules[j].as_str());
                let ka = (dom_s.clone(), a.to_string());
                let kb = (dom_s.clone(), b.to_string());
                let ca = &clouds[&ka];
                let cb = &clouds[&kb];
                let t0 = std::time::Instant::now();
                let verdict = detect_linking(&ca.points, &cb.points, dim, &detector);
                sweep_ms += t0.elapsed().as_millis();
                sweep_pairs += 1;
                let qa = own_queries[&ka].max(1);
                let qb = own_queries[&kb].max(1);
                let ab = conf_events
                    .get(&(dom_s.clone(), a.to_string(), b.to_string()))
                    .copied()
                    .unwrap_or(0);
                let ba = conf_events
                    .get(&(dom_s.clone(), b.to_string(), a.to_string()))
                    .copied()
                    .unwrap_or(0);
                let base_confused = base_top1.get(&ka).map(String::as_str) == Some(b)
                    || base_top1.get(&kb).map(String::as_str) == Some(a);
                results.push((
                    dom_s.clone(),
                    PairResult {
                        a: a.to_string(),
                        b: b.to_string(),
                        link: verdict.link,
                        confusion_rate: (ab + ba) as f64 / (qa + qb) as f64,
                        base_confused,
                        events: ab + ba,
                    },
                ));
            }
        }
    }
    println!(
        "pair sweep: {} pairs in {} ms (avg {:.2} ms/pair)",
        sweep_pairs,
        sweep_ms,
        sweep_ms as f64 / sweep_pairs.max(1) as f64
    );

    // ── Per-domain stats + pooled headline ────────────────────────────────
    for dom_s in &domains {
        let set: Vec<&PairResult> = results
            .iter()
            .filter(|(d, _)| d == dom_s)
            .map(|(_, r)| r)
            .collect();
        analyze(dom_s, &set);
    }
    let pooled: Vec<&PairResult> = results.iter().map(|(_, r)| r).collect();
    analyze("POOLED", &pooled);

    println!(
        "\nverdict rule (pre-registered): GO = linked-pair enrichment p<0.05 on ≥1 domain,\n\
         coherent on both axes (variant-rate + base-canon); NO-GO = no enrichment;\n\
         UNDERPOWERED = fewer than ~8 confused pairs total."
    );
}

fn domain_of(label: &str) -> DomainId {
    match label {
        "clippy" => DomainId::Clippy,
        "perf" => DomainId::Perf,
        "kernel" => DomainId::Kernel,
        "sec" => DomainId::Sec,
        "docker" => DomainId::Docker,
        _ => DomainId::Dist,
    }
}

fn top1_of(
    by_domain: &[(DomainId, Vec<riir_refine::draft::ScoredRule>)],
    want: DomainId,
) -> Option<String> {
    by_domain
        .iter()
        .find(|(d, _)| *d == want)
        .and_then(|(_, rules)| canonical_rank(rules).first().map(|r| r.rule_id.clone()))
}

fn analyze(name: &str, set: &[&PairResult]) {
    if set.is_empty() {
        println!("\n── {name}: no pairs");
        return;
    }
    let n = set.len();
    let linked: Vec<&&PairResult> = set.iter().filter(|r| r.link != 0).collect();
    let confused: Vec<&&PairResult> = set.iter().filter(|r| r.events > 0).collect();
    let base_confused: Vec<&&PairResult> = set.iter().filter(|r| r.base_confused).collect();
    let a = set.iter().filter(|r| r.link != 0 && r.events > 0).count();
    let b = linked.len() - a;
    let c = confused.len() - a;
    let d = n - a - b - c;

    let conf_mass_linked: f64 = linked.iter().map(|r| r.confusion_rate).sum();
    let conf_mass_unlinked: f64 = set
        .iter()
        .filter(|r| r.link == 0)
        .map(|r| r.confusion_rate)
        .sum();
    let enrich = if conf_mass_unlinked > 0.0 && !linked.is_empty() {
        (conf_mass_linked / linked.len() as f64)
            / (conf_mass_unlinked / (n - linked.len()) as f64)
    } else {
        f64::NAN
    };
    let p = fisher_right_tail(a, b, c, d);

    let xs: Vec<f64> = set.iter().map(|r| r.link.abs() as f64).collect();
    let ys: Vec<f64> = set.iter().map(|r| r.confusion_rate).collect();
    let rho = spearman(&xs, &ys);
    let base_linked = base_confused.iter().filter(|r| r.link != 0).count();

    println!("\n── {name}: {n} pairs ──────────────────────────────");
    println!(
        "  linked: {} ({:.1}%)   confused (any variant event): {}   base-canon confused: {}",
        linked.len(),
        100.0 * linked.len() as f64 / n as f64,
        confused.len(),
        base_confused.len()
    );
    println!("  2×2 linked×confused: a={a} b={b} c={c} d={d}   Fisher right-tail p={p:.4}");
    println!(
        "  mean confusion rate: linked {:.4} vs unlinked {:.4}  → enrichment ×{:.2}",
        conf_mass_linked / linked.len().max(1) as f64,
        conf_mass_unlinked / (n - linked.len()).max(1) as f64,
        enrich
    );
    println!(
        "  Spearman ρ(|link|, confusion rate) = {rho:.3}   (paper's CIFAR-10 bar class: ≈0.48)"
    );
    println!(
        "  base-canon axis: {}/{} base-confused pairs linked (overall linked rate {:.1}%)",
        base_linked,
        base_confused.len(),
        100.0 * linked.len() as f64 / n as f64
    );
    if let (Some(maxl), Some(maxr)) = (
        set.iter().max_by(|x, y| x.link.abs().cmp(&y.link.abs())),
        set.iter()
            .max_by(|x, y| x.confusion_rate.total_cmp(&y.confusion_rate)),
    ) {
        println!(
            "  strongest link: {} ↔ {} |link|={}   most-confused: {} ↔ {} rate {:.3}",
            maxl.a, maxl.b, maxl.link, maxr.a, maxr.b, maxr.confusion_rate
        );
    }
}
