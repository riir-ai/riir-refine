//! Plan 005 T3.5 option 3 — the 768-D Bonsai dense-rerank measurement.
//!
//! Gated behind `bonsai_retrieval` (never default): loads the ~6.7 GB
//! Ternary-Bonsai-27B GGUF and runs a CPU ternary forward pass per embedded
//! text. Run explicitly:
//!
//! ```bash
//! cargo test --release --features bonsai_retrieval --test bonsai_retrieval_probe \
//!   -- --ignored --nocapture --test-threads=1
//! ```
//!
//! ## Why both tests are `#[ignore]`
//!
//! `--all-features` enables `bonsai_retrieval`, so a plain
//! `cargo test --all-features --tests` sweep used to pull this binary in and
//! run BOTH tests -- in DEBUG, and in PARALLEL (two concurrent 6.7 GB model
//! loads; measured 15.5 GB RSS, 199% CPU, still going at 15 min). The
//! documented ~50 min is a RELEASE figure for ONE test, so a debug sweep is
//! effectively unbounded. That is why the repo's all-binaries run had never
//! once completed -- not a flake or a crash, just this probe.
//!
//! These are measurements, not gates: nothing regresses if a sweep skips
//! them, and the `--test-threads=1` above is load-bearing (the parallel
//! double-load is what made the memory pressure dangerous on a box shared
//! with sibling agents). `#[ignore]` keeps the sweep bounded while the
//! explicit invocation above still runs them in full.
//!
//! ## Why this is scope-limited, and how
//!
//! [`probe_cost`] measured the CPU forward pass at **16.6 s (short) / 48.1 s
//! (long) per embed**, with a 166 s model load. The full eval issues ~1018
//! embeds per config (index chunks + multi-scale line queries across 96
//! spans), which projects to **~13.6 hours per config** — ~41 hours for a
//! 3-weight sweep. That is not a benchmark anyone can run.
//!
//! [`dense_rerank_precision`] therefore measures the largest MATCHED subset
//! that is feasible, and states the scope rather than quietly shrinking it:
//!
//! - **clippy domain only** — the domain whose G1 actually fails (perf is
//!   already 97.9%).
//! - **whole-span scale only**, on BOTH arms — the modelless floor is
//!   re-measured in the same whole-span mode, so the comparison is
//!   apples-to-apples rather than dense-vs-multi-scale.
//! - **dense vectors cached once**, so sweeping the fusion weight is free
//!   (weights change ranking, not embeddings).
//!
//! Cost: 40 pattern embeds + 48 span embeds ≈ 50 min + load, once.

#![cfg(feature = "bonsai_retrieval")]

use std::collections::HashMap;
use std::time::Instant;

use riir_refine::draft::{IndexTokenizer, RetrievalConfig, RuleIndex};

/// Weight sweep for the dense term. The modelless sweep showed the 8-D latent
/// term is inert at any weight, so the dense term is given real authority —
/// weights well above 1.0 — or it could not be tested at all.
const DENSE_WEIGHTS: &[f32] = &[0.5, 1.0, 5.0, 20.0, 100.0];

fn gguf_present() -> bool {
    let path = riir_rag_bonsai::bonsai_embedder::default_gguf_path();
    if !path.exists() {
        eprintln!("SKIP: GGUF not present at {}", path.display());
        return false;
    }
    true
}

#[test]
#[ignore = "loads a 6.7 GB GGUF and runs a CPU ternary forward pass per embed; \
           ~50 min in RELEASE (far worse in debug). A measurement probe, not a \
           gate -- run it explicitly with the invocation in the module docs."]
fn probe_cost() {
    use riir_rag::DenseEmbedder;
    use riir_rag_bonsai::{BonsaiEmbedStrategy, BonsaiEmbedder};
    if !gguf_present() {
        return;
    }
    let path = riir_rag_bonsai::bonsai_embedder::default_gguf_path();

    let t0 = Instant::now();
    let embedder = BonsaiEmbedder::load(&path, BonsaiEmbedStrategy::MeanPool).expect("bonsai load");
    let load_s = t0.elapsed().as_secs_f64();

    let short = "fn f(x: u32) -> u32 { x * 1 }";
    let long = r"pub fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {
        for u in users {
            if u.id == id { return Some(u); }
        }
        None
    }";

    let (mut short_s, mut long_s) = (0.0, 0.0);
    for _ in 0..3 {
        let t = Instant::now();
        let _ = embedder.embed_dense(short);
        short_s += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let _ = embedder.embed_dense(long);
        long_s += t.elapsed().as_secs_f64();
    }
    println!("\n=== T3.5 option 3 cost probe ===");
    println!("  GGUF load           : {load_s:>8.2} s");
    println!("  embed_dense (short) : {:>8.1} ms", short_s / 3.0 * 1000.0);
    println!("  embed_dense (long)  : {:>8.1} ms", long_s / 3.0 * 1000.0);
    let embeds_per_eval = 1018.0;
    println!(
        "  full eval ~{embeds_per_eval:.0} embeds -> {:.1} h/config (why this file is scope-limited)",
        embeds_per_eval * (long_s / 3.0) / 3600.0
    );
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    match denom > 0.0 {
        true => dot / denom,
        false => 0.0,
    }
}

#[test]
#[ignore = "loads a 6.7 GB GGUF and runs a CPU ternary forward pass per embed; \
           ~50 min in RELEASE (far worse in debug). A measurement probe, not a \
           gate -- run it explicitly with the invocation in the module docs."]
fn dense_rerank_precision() {
    use riir_rag::DenseEmbedder;
    use riir_rag_bonsai::{BonsaiEmbedStrategy, BonsaiEmbedder};
    if !gguf_present() {
        return;
    }

    // ── The matched modelless arm: clippy corpus, whole-span scale. ──
    use riir_refine::domain::{HealDomain, HealEntry};
    let corpus: Vec<(String, String)> =
        riir_refine::domains::clippy_lints::ClippyLintDomain::build_corpus()
            .into_iter()
            .map(|e| (e.rule_id().to_string(), e.buggy_code().to_string()))
            .collect();
    let rule_texts: HashMap<String, String> = corpus.iter().cloned().collect();
    let n_rules = corpus.len();

    let index = RuleIndex::build_with(
        corpus,
        RetrievalConfig {
            tokenizer: IndexTokenizer::OpAwareBigram,
            latent_weight: RetrievalConfig::DEFAULT_LATENT_WEIGHT,
            dense_weight: 0.0,
            two_stage_oversample: RetrievalConfig::DEFAULT_TWO_STAGE_OVERSAMPLE,
        },
    )
    .expect("build clippy index");

    let spans = riir_refine::draft::eval_fixtures_clippy::CLIPPY_SPANS;
    println!("\n=== T3.5 option 3 — 768-D dense rerank vs the matched modelless floor ===");
    println!(
        "  scope: clippy only, whole-span scale on BOTH arms, {n_rules} rules, {} spans",
        spans.len()
    );

    let path = riir_rag_bonsai::bonsai_embedder::default_gguf_path();
    let t0 = Instant::now();
    let embedder = BonsaiEmbedder::load(&path, BonsaiEmbedStrategy::MeanPool).expect("bonsai load");
    println!(
        "  model loaded in {:.1} s; embedding {} patterns + {} spans (cached once)...",
        t0.elapsed().as_secs_f64(),
        n_rules,
        spans.len()
    );

    // Cache every dense vector ONCE — the weight sweep then costs nothing.
    let t0 = Instant::now();
    let rule_dense: HashMap<&str, Vec<f32>> = rule_texts
        .iter()
        .map(|(id, text)| (id.as_str(), embedder.embed_dense(text)))
        .collect();
    println!(
        "  {} pattern embeds in {:.1} min",
        rule_dense.len(),
        t0.elapsed().as_secs_f64() / 60.0
    );

    let t0 = Instant::now();
    let span_dense: Vec<Vec<f32>> = spans
        .iter()
        .map(|(_, code)| embedder.embed_dense(code))
        .collect();
    println!(
        "  {} span embeds in {:.1} min",
        span_dense.len(),
        t0.elapsed().as_secs_f64() / 60.0
    );

    // Modelless whole-span scores for every rule, per span (top_k = all).
    let modelless: Vec<Vec<(String, f32)>> = spans
        .iter()
        .map(|(_, code)| {
            index
                .suggest(code, n_rules)
                .into_iter()
                .map(|r| (r.rule_id, r.score))
                .collect()
        })
        .collect();

    // Rank with a given dense weight, using the cached vectors.
    let top1_at = |dense_weight: f32| -> f32 {
        let mut hits = 0usize;
        for (i, (label, _)) in spans.iter().enumerate() {
            let mut scored: Vec<(&str, f32)> = modelless[i]
                .iter()
                .map(|(id, s)| {
                    let d = rule_dense
                        .get(id.as_str())
                        .map_or(0.0, |rv| cosine(&span_dense[i], rv));
                    (id.as_str(), s + dense_weight * d)
                })
                .collect();
            // Deterministic tie-break by rule_id, matching the eval harness.
            scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            if scored.first().is_some_and(|(id, _)| *id == *label) {
                hits += 1;
            }
        }
        hits as f32 / spans.len() as f32
    };

    let floor = top1_at(0.0);
    println!(
        "\n  matched modelless floor (dense_w = 0): clippy@1 {:.1}%",
        floor * 100.0
    );
    println!("{:>10}  {:>10}  {:>10}", "dense_w", "clippy@1", "vs floor");
    let mut best = floor;
    for &w in DENSE_WEIGHTS {
        let acc = top1_at(w);
        best = best.max(acc);
        println!(
            "{w:>10.1}  {:>9.1}%  {:>+9.1}pp",
            acc * 100.0,
            (acc - floor) * 100.0
        );
    }
    println!(
        "\n  best clippy@1 across the dense sweep: {:.1}% (floor {:.1}%, T3.5 G1 bar 80%)",
        best * 100.0,
        floor * 100.0
    );
    println!(
        "  verdict: dense rerank {} the matched modelless floor",
        match best > floor {
            true => "BEATS",
            false => "does NOT beat",
        }
    );
    // No hard assert: this test IS the measurement that decides whether the
    // 6.7 GB dependency earns its place. The verdict lives in the bench note;
    // asserting today's number would only freeze it.
}
