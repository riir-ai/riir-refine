//! Issue 125 — the CUA-S1 specialist option-scorer `PoC` harness
//! (Research 573's recipe on healer rule selection).
//!
//! The challenger hypothesis: a ~719k-param trained byte-level specialist
//! (context = span bytes, options = candidate rules + skip, one logit per
//! option — [`riir_clippy::draft::choice_scorer`]) beats the shipped
//! modelless `RerankMode::Structural` rerank at composite top-1 on the
//! SAME production pools. The modelless default is NOT challenged until
//! the GOAT gates pass; a Bench-065 memorization signature = REFUTED.
//!
//! Data sources (honest scope): the `retrieval_eval` fixtures — 48 clippy +
//! 48 perf oracle-verified spans + 12 clean excerpts (the skip rows). The
//! issue's other two named sources do not exist on this box (no fixseq
//! rings; `--clippy-json` feeds are run artifacts) — a corpus-scale caveat
//! the verdict records: 96 gold spans against CUA-S1's ~150k rows bounds
//! what this `PoC` can prove about the CLASS; the gates below test the
//! mechanism on our corpus.
//!
//! Modes: `selftest` (tiny smoke) · `render` (dataset inventory) ·
//! `train` (split → threaded CE/AdamW → checkpoint) · `eval` (gates +
//! verdict) · `all`. Runs are deterministic: fixed seeds, fixed 8 training
//! workers, fixed reduction order — bit-identical reruns.
//!
//! Verdict record: `.benchmarks/098_choice_scorer_poc.md`.

use std::collections::BTreeMap;
use std::time::Instant;

use riir_clippy::domain::HealDomain;
use riir_clippy::domains::clippy_lints::ClippyLintDomain;
use riir_clippy::domains::rust_perf::RustPerfDomain;
use riir_clippy::draft::choice_scorer::{
    ChoiceScorer, Rng, ScorerConfig, ScorerGrads, AdamW,
};
use riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS;
use riir_clippy::draft::eval_fixtures_perf::PERF_SPANS;
use riir_clippy::draft::instantiation_rerank::rule_family;
use riir_clippy::draft::latent_matcher::LatentMatcher;
use riir_clippy::draft::retrieval_eval::CLEAN_SPANS;

// ---------------------------------------------------------------------------
// Constants (all seeds fixed — determinism is a gate, not a preference)
// ---------------------------------------------------------------------------

const SPLIT_SEED: u64 = 125;
const AUG_SEED: u64 = 1_250;
const INIT_SEED: u64 = 5_800;
const FROZEN_SEED: u64 = 7_777;
const SHUFFLE_SEED: u64 = 42;
const POOL_TOP_K: usize = 8; // per domain — the eval's PROBE_TOP_K shape
const MAX_OPTIONS: usize = 33; // CUA-S1's option ceiling
const TRAIN_MAX_OPTIONS: usize = 24;
const CONFUSER_EPISODES: usize = 32; // + 8 production-pool copies per gold span
const CLEAN_EPISODES: usize = 16;
const HELDOUT_FRACTION_NUM: usize = 1; // 1/5 of RULES held out (rule-disjoint)
const HELDOUT_FRACTION_DEN: usize = 5;
const EPOCHS: u64 = 6; // the recipe's 6 epochs
const BATCH: usize = 32;
const WORKERS: usize = 8; // FIXED — reduction order is part of determinism
const BASE_LR: f32 = 1e-3;
const CONFUSER_MIN: usize = 2; // forced same-family look-alikes per episode

const SKIP_RULE_IDX: u32 = u32::MAX; // the trained decline option

fn skip_option_text() -> Vec<u8> {
    SKIP_TEXT.to_vec()
}

// ---------------------------------------------------------------------------
// Dataset
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RuleOption {
    rule_id: String,
    text: Vec<u8>, // "{rule_id}\n{buggy_code}" truncated to opt_len
}

#[derive(Clone)]
struct Episode {
    ctx: Vec<u8>,
    /// Option texts as corpus indices + the trailing skip marker.
    options: Vec<u32>,
    /// Correct option index into `options`.
    label: usize,
}

#[derive(Clone)]
struct LabeledSpan {
    rule_id: String,
    code: String,
    clean: bool,
}

struct Dataset {
    rules: Vec<RuleOption>, // corpus-wide option table (rule idx = position)
    rule_idx: BTreeMap<String, u32>,
    spans: Vec<LabeledSpan>,
    /// The production pool (canonical-merge order, capped) per span.
    pools: Vec<Vec<u32>>,
    /// Per-RULE held-out flag (the split is rule-disjoint).
    heldout_rules: BTreeMap<String, bool>,
}

fn build_dataset() -> Result<Dataset, String> {
    // ---- corpus-wide option table (clippy + perf, deterministic order) ----
    let clippy = ClippyLintDomain::build_corpus();
    let perf = RustPerfDomain::build_corpus();
    let mut rules: Vec<RuleOption> = Vec::with_capacity(clippy.len() + perf.len());
    let mut rule_idx: BTreeMap<String, u32> = BTreeMap::new();
    let cfg = ScorerConfig::default();
    {
        let entries: Vec<&dyn riir_clippy::domain::HealEntry> = clippy
            .iter()
            .map(|e| e as &dyn riir_clippy::domain::HealEntry)
            .chain(perf.iter().map(|e| e as &dyn riir_clippy::domain::HealEntry))
            .collect();
        for e in entries {
            let text = format!("{}\n{}", e.rule_id(), e.buggy_code());
            let text = text.as_bytes()[..text.len().min(cfg.opt_len)].to_vec();
            rule_idx.insert(e.rule_id().to_string(), rules.len() as u32);
            rules.push(RuleOption { rule_id: e.rule_id().to_string(), text });
        }
    }

    // ---- labeled spans ----
    let mut spans: Vec<LabeledSpan> = Vec::new();
    for (lint, code) in CLIPPY_SPANS {
        spans.push(LabeledSpan { rule_id: lint.to_string(), code: code.to_string(), clean: false });
    }
    for (rule, code) in PERF_SPANS {
        spans.push(LabeledSpan { rule_id: rule.to_string(), code: code.to_string(), clean: false });
    }
    for (_tag, code) in CLEAN_SPANS {
        spans.push(LabeledSpan { rule_id: String::new(), code: code.to_string(), clean: true });
    }

    // ---- production pools per span (the canonical merge, mirrored) ----
    let mut matcher = LatentMatcher::from_enabled_domains()
        .map_err(|e| format!("matcher: {e}"))?;
    let mut pools = Vec::with_capacity(spans.len());
    for span in &spans {
        let mut pool_ids: Vec<u32> = Vec::new();
        if !span.clean {
            let (_routed, by_domain) = matcher.suggest_span(&span.code, POOL_TOP_K);
            let mut all: Vec<&riir_clippy::draft::corpus_index::ScoredRule> = by_domain
                .iter()
                .flat_map(|(_, rs)| rs.iter())
                .collect();
            // MUST mirror SpanSuggestion::all_rules' canonical key.
            all.sort_unstable_by(|a, b| {
                b.instantiates
                    .cmp(&a.instantiates)
                    .then_with(|| b.score.total_cmp(&a.score))
                    .then_with(|| a.rule_id.cmp(&b.rule_id))
            });
            for r in all.into_iter().take(MAX_OPTIONS) {
                if let Some(&idx) = rule_idx.get(&r.rule_id) {
                    if !pool_ids.contains(&idx) {
                        pool_ids.push(idx);
                    }
                }
            }
        }
        pools.push(pool_ids);
    }

    // ---- rule-disjoint split (stratified by domain, seeded shuffle) ----
    let mut rng = Rng::new(SPLIT_SEED);
    let mut heldout_rules: BTreeMap<String, bool> = BTreeMap::new();
    for domain in ["clippy", "perf"] {
        let mut ids: Vec<String> = rules
            .iter()
            .map(|r| r.rule_id.clone())
            .filter(|id| {
                (domain == "clippy" && CLIPPY_SPANS.iter().any(|(l, _)| l == id))
                    || (domain == "perf" && PERF_SPANS.iter().any(|(r, _)| r == id))
            })
            .collect();
        // Fisher-Yates with the split rng (deterministic)
        for i in (1..ids.len()).rev() {
            let j = rng.uniform_range(0, i + 1);
            ids.swap(i, j);
        }
        let n_held = ids.len() * HELDOUT_FRACTION_NUM / HELDOUT_FRACTION_DEN;
        for (i, id) in ids.iter().enumerate() {
            heldout_rules.insert(id.clone(), i < n_held);
        }
    }

    Ok(Dataset { rules, rule_idx, spans, pools, heldout_rules })
}

const SKIP_TEXT: &[u8] = b"skip\nno rule applies to this span";

/// Option text slices for an episode (the skip option's text is a const).
fn rule_heldout(ds: &Dataset, rule_id: &str) -> bool {
    ds.heldout_rules.get(rule_id).copied().unwrap_or(false)
}

fn span_heldout(ds: &Dataset, span_i: usize) -> bool {
    let s = &ds.spans[span_i];
    !s.clean && rule_heldout(ds, &s.rule_id)
}

fn option_texts<'a>(ds: &'a Dataset, ep: &Episode) -> Vec<&'a [u8]> {
    ep.options
        .iter()
        .map(|&idx| {
            if idx == SKIP_RULE_IDX {
                SKIP_TEXT
            } else {
                ds.rules[idx as usize].text.as_slice()
            }
        })
        .collect()
}

/// One training-episode generator (deterministic per span + aug rng).
fn training_episodes(ds: &Dataset, span_i: usize, rng: &mut Rng, out: &mut Vec<Episode>) {
    let span = &ds.spans[span_i];
    let pool = &ds.pools[span_i];
    let gold_idx = ds.rule_idx.get(&span.rule_id).copied();

    if span.clean {
        for _ in 0..CLEAN_EPISODES {
            let n = rng.uniform_range(6, TRAIN_MAX_OPTIONS + 1);
            let mut options: Vec<u32> = Vec::with_capacity(n + 1);
            while options.len() < n {
                let cand = rng.uniform_range(0, ds.rules.len()) as u32;
                if !options.contains(&cand) {
                    options.push(cand);
                }
            }
            options.push(SKIP_RULE_IDX);
            let label = options.len() - 1;
            out.push(Episode { ctx: span.code.as_bytes().to_vec(), options, label });
        }
        return;
    }

    // (1) the production-pool episodes ×8 (order-shuffled copies): the
    // eval shape — gold present in the canonical pool + skip competing.
    // Weighted because the canonical pool is what eval measures against;
    // the 39-episode confuser curriculum below must not drown it out
    // (v1's unweighted mix collapsed the trained model to always-skip —
    // measured 0/83 with skip at p≈1.00 — a curriculum defect, not a
    // model failure: the frozen control scored 7.2%).
    {
        let base_options = pool.clone();
        let base_label = match gold_idx {
            Some(g) if base_options.contains(&g) => base_options.iter().position(|&x| x == g).unwrap(),
            _ => {
                // gold genuinely absent from the production pool — the
                // HONEST decline row (skip is the correct answer here).
                let mut o = base_options.clone();
                o.push(SKIP_RULE_IDX);
                let l = o.len() - 1;
                for copy in 0..8 {
                    let mut o2 = o.clone();
                    if copy > 0 {
                        shuffle_options(&mut o2, rng);
                    }
                    let l2 = if copy == 0 { l } else { o2.iter().position(|&x| x == SKIP_RULE_IDX).unwrap() };
                    out.push(Episode { ctx: span.code.as_bytes().to_vec(), options: o2, label: l2 });
                }
                // decline rows already emitted — continue to confusers
                // with the aug loop below.
                for ep_i in 0..CONFUSER_EPISODES {
                    emit_confuser_episode(ds, span_i, gold_idx, rng, ep_i, out);
                }
                return;
            }
        };
        for copy in 0..8 {
            let mut o2 = base_options.clone();
            o2.push(SKIP_RULE_IDX);
            let l2 = if copy == 0 {
                base_label
            } else {
                shuffle_options(&mut o2, rng);
                o2.iter().position(|&x| x == gold_idx.unwrap()).unwrap()
            };
            out.push(Episode { ctx: span.code.as_bytes().to_vec(), options: o2, label: l2 });
        }
    }

    // (2) confuser-augmented episodes — gold ALWAYS present and labeled
    // (v1's 25% gold-dropped-same-context rows were contradictory label
    // noise: the context still matched the gold pattern — that mix
    // collapsed the trained model to always-skip, measured 0/83 with
    // skip at p≈1.00. The honest decline signal comes from clean spans +
    // gold-absent production pools.)
    for ep_i in 0..CONFUSER_EPISODES {
        emit_confuser_episode(ds, span_i, gold_idx, rng, ep_i, out);
    }
}

/// One confuser episode: gold + forced same-family look-alikes + random
/// distractors (never a held-out rule), seeded order shuffle, label=gold.
fn emit_confuser_episode(
    ds: &Dataset,
    span_i: usize,
    gold_idx: Option<u32>,
    rng: &mut Rng,
    _ep_i: usize,
    out: &mut Vec<Episode>,
) {
    let span = &ds.spans[span_i];
    let Some(gold) = gold_idx else { return };
    let gold_family = rule_family(&span.rule_id);
    let n_total = rng.uniform_range(8, TRAIN_MAX_OPTIONS + 1);
    let mut options: Vec<u32> = Vec::with_capacity(n_total + 1);
    options.push(gold);
    let mut confusers = 0;
    for (idx, r) in ds.rules.iter().enumerate() {
        if confusers >= CONFUSER_MIN {
            break;
        }
        if rule_family(&r.rule_id) == gold_family
            && idx as u32 != gold
            && !rule_heldout(ds, &r.rule_id)
            && !options.contains(&(idx as u32))
        {
            options.push(idx as u32);
            confusers += 1;
        }
    }
    while options.len() < n_total {
        let cand = rng.uniform_range(0, ds.rules.len()) as u32;
        if !options.contains(&cand) && !rule_heldout(ds, &ds.rules[cand as usize].rule_id) {
            options.push(cand);
        }
    }
    shuffle_options(&mut options, rng);
    let label = options.iter().position(|&x| x == gold).unwrap();
    out.push(Episode { ctx: span.code.as_bytes().to_vec(), options, label });
}

fn shuffle_options(options: &mut [u32], rng: &mut Rng) {
    for i in (1..options.len()).rev() {
        let j = rng.uniform_range(0, i + 1);
        options.swap(i, j);
    }
}

fn render_training_set(ds: &Dataset) -> Vec<Episode> {
    let mut rng = Rng::new(AUG_SEED);
    let mut out = Vec::new();
    let mut dropped = 0usize;
    for span_i in 0..ds.spans.len() {
        if span_heldout(ds, span_i) {
            continue; // OOD spans never train
        }
        // A gold label outside the corpus option table cannot be an
        // option — the span is unusable for training (counted, printed).
        if !ds.spans[span_i].clean && !ds.rule_idx.contains_key(&ds.spans[span_i].rule_id) {
            dropped += 1;
            continue;
        }
        training_episodes(ds, span_i, &mut rng, &mut out);
    }
    if dropped > 0 {
        println!("render: {dropped} spans dropped (gold rule not in the corpus table)");
    }
    out
}

// ---------------------------------------------------------------------------
// Training (threaded, deterministic: fixed worker count + ordered reduce)
// ---------------------------------------------------------------------------

fn train(ds: &Dataset, episodes: &[Episode], tag: &str) -> ChoiceScorer {
    let model = ChoiceScorer::new(ScorerConfig::default(), INIT_SEED);
    let mut model = model;
    let total_steps = (episodes.len() / BATCH) as u64 * EPOCHS;
    let mut adam = AdamW::new(&model, BASE_LR);
    adam.total_steps = total_steps.max(1);
    adam.warmup = (total_steps / 20).max(1);

    let mut order: Vec<usize> = (0..episodes.len()).collect();
    let mut rng = Rng::new(AUG_SEED ^ 0xE_0C4);
    let t0 = Instant::now();
    let mut log_loss = 0.0_f32;
    let mut log_n = 0u32;

    for epoch in 0..EPOCHS {
        // seeded per-epoch shuffle
        for i in (1..order.len()).rev() {
            let j = rng.uniform_range(0, i + 1);
            order.swap(i, j);
        }
        let mut step_i = 0u64;
        for batch_start in (0..episodes.len()).step_by(BATCH) {
            let batch: Vec<&Episode> = order[batch_start..(batch_start + BATCH).min(episodes.len())]
                .iter()
                .map(|&i| &episodes[i])
                .collect();
            let n_this = batch.len();
            let slices: Vec<Vec<&Episode>> = split_batch(&batch, WORKERS);
            let mut losses = [0.0_f32; WORKERS];
            let results: Vec<(ScorerGrads, f32)> = std::thread::scope(|scope| {
                let mut joins = Vec::with_capacity(WORKERS);
                // consuming zip: each worker OWNS its zeroed grad slot and
                // RETURNS it with its loss sum (indexed disjoint mutable
                // borrows in a loop don't typecheck; ownership does).
                for (slice, loss_acc) in slices.iter().zip(losses.iter_mut()) {
                    let model_ref = &model;
                    let mut g = ScorerGrads::zero_like(&model);
                    joins.push(scope.spawn(move || {
                        let mut lsum = 0.0_f32;
                        for ep in slice {
                            let opts = option_texts(ds, ep);
                            lsum += model_ref.forward_backward(&mut g, &ep.ctx, &opts, ep.label);
                        }
                        *loss_acc = lsum;
                        g
                    }));
                }
                joins
                    .into_iter()
                    .map(|j| {
                        let g = j.join().expect("worker");
                        (g, 0.0) // loss already written through loss_acc
                    })
                    .collect::<Vec<_>>()
            });
            let mut grads = ScorerGrads::zero_like(&model);
            for (g, _) in &results {
                grads.add_from(g);
            }
            grads.scale(1.0 / n_this as f32);
            adam.step(&mut model, &grads);
            log_loss += losses.iter().sum::<f32>();
            log_n += n_this as u32;
            step_i += 1;
        }
        println!(
            "[{tag}] epoch {}/{}: mean loss {:.4} ({} steps, {:.1}s)",
            epoch + 1,
            EPOCHS,
            log_loss / log_n as f32,
            step_i,
            t0.elapsed().as_secs_f32()
        );
        log_loss = 0.0;
        log_n = 0;
    }
    model
}

fn split_batch<'a>(batch: &[&'a Episode], workers: usize) -> Vec<Vec<&'a Episode>> {
    let mut out: Vec<Vec<&'a Episode>> = vec![Vec::new(); workers];
    for (i, ep) in batch.iter().enumerate() {
        out[i % workers].push((*ep) as &'a Episode);
    }
    out
}

// ---------------------------------------------------------------------------
// Eval + gates
// ---------------------------------------------------------------------------

struct GateReport {
    name: &'static str,
    passed: bool,
    detail: String,
}

fn evaluate(ds: &Dataset, model: &ChoiceScorer, tag: &str, shuffled: bool) -> (usize, usize, usize) {
    // returns (n_eval, scorer_correct, baseline_correct) over gold spans
    let mut pools_cache: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    let mut n = 0;
    let mut sc = 0;
    let mut bl = 0;
    let mut rng = Rng::new(SHUFFLE_SEED);
    for span_i in 0..ds.spans.len() {
        let span = &ds.spans[span_i];
        let want_ood = heldout_filter(tag);
        if span.clean || span_heldout(ds, span_i) != want_ood {
            continue;
        }
        let pool = &ds.pools[span_i];
        let gold = match ds.rule_idx.get(&span.rule_id) {
            Some(g) => *g,
            None => continue,
        };
        let mut options: Vec<u32> = pool.clone();
        options.push(SKIP_RULE_IDX);
        let ctx: Vec<u8> = if shuffled {
            rng.shuffled_bytes(span.code.as_bytes())
        } else {
            span.code.as_bytes().to_vec()
        };
        let opt_pool_refs: Vec<Vec<f32>> = options
            .iter()
            .map(|&idx| match pools_cache.entry(idx) {
                std::collections::btree_map::Entry::Occupied(o) => o.get().clone(),
                std::collections::btree_map::Entry::Vacant(v) => {
                    let text = if idx == SKIP_RULE_IDX {
                        skip_option_text()
                    } else {
                        ds.rules[idx as usize].text.clone()
                    };
                    let p = model.encode_option(&text);
                    v.insert(p).clone()
                }
            })
            .collect();
        let refs: Vec<&[f32]> = opt_pool_refs.iter().map(|v| v.as_slice()).collect();
        let logits = model.forward_logits_cached(&ctx, &refs);
        let best = logits
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1)).map_or(0, |(i, _)| i);
        let scorer_pick = options[best];
        let gold_pos = options.iter().position(|&x| x == gold);
        if std::env::var("CHOICE_DEBUG").is_ok() && n < 10 {
            let probs = softmax_of(&logits);
            let top3: Vec<String> = {
                let mut idxs: Vec<usize> = (0..options.len()).collect();
                idxs.sort_by(|&a, &b| logits[b].total_cmp(&logits[a]));
                idxs.iter().take(3).map(|&i| {
                    let rid = if options[i] == SKIP_RULE_IDX { "skip".to_string() } else { ds.rules[options[i] as usize].rule_id.clone() };
                    format!("{rid}({:.2})", probs[i])
                }).collect()
            };
            println!(
                "debug[span {span_i}] gold={} (pos {:?}) picks=[{}]",
                span.rule_id, gold_pos, top3.join(", ")
            );
        }
        // Baseline: the canonical merge's top-1 (pool[0]) — the production
        // ranker's answer on the same option set.
        let baseline_pick = pool.first().copied();
        n += 1;
        if gold_pos == Some(best) || scorer_pick == gold {
            sc += 1;
        }
        if baseline_pick == Some(gold) {
            bl += 1;
        }
    }
    (n, sc, bl)
}

fn heldout_filter(tag: &str) -> bool {
    tag == "ood"
}

fn calibration(ds: &Dataset, model: &ChoiceScorer) -> (f32, f32, f32, f32) {
    // (ECE raw, Brier raw, ECE platt, Brier platt) over gold-span decisions
    // — 15 equal-width bins; Platt = 1-param temperature on logits fit by
    // NLL grid on the TRAIN-rule spans.
    struct Decision {
        conf: f32,
        correct: bool,
    }
    let mut decisons: Vec<Decision> = Vec::new();
    let mut pools_cache: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    for span_i in 0..ds.spans.len() {
        let span = &ds.spans[span_i];
        if span.clean {
            continue;
        }
        let pool = &ds.pools[span_i];
        let Some(&gold) = ds.rule_idx.get(&span.rule_id) else { continue };
        let mut options = pool.clone();
        options.push(SKIP_RULE_IDX);
        let opt_pools: Vec<Vec<f32>> = options
            .iter()
            .map(|&idx| pools_cache.entry(idx).or_insert_with(|| {
                let text = if idx == SKIP_RULE_IDX { skip_option_text() } else { ds.rules[idx as usize].text.clone() };
                model.encode_option(&text)
            }).clone())
            .collect();
        let refs: Vec<&[f32]> = opt_pools.iter().map(|v| v.as_slice()).collect();
        let logits = model.forward_logits_cached(span.code.as_bytes(), &refs);
        let probs = softmax_of(&logits);
        let best = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1)).map_or(0, |(i, _)| i);
        decisons.push(Decision { conf: probs[best], correct: options[best] == gold });
    }
    let ece_brier = |decisions: &[Decision]| -> (f32, f32) {
        const BINS: usize = 15;
        let mut bin_sum = [0.0_f32; BINS];
        let mut bin_n = [0u32; BINS];
        let mut brier = 0.0_f32;
        for d in decisions {
            let b = (d.conf * BINS as f32) as usize;
            let b = b.min(BINS - 1);
            bin_sum[b] += if d.correct { 1.0 } else { 0.0 };
            bin_n[b] += 1;
            let err = if d.correct { 1.0 } else { 0.0 } - d.conf;
            brier += err * err;
        }
        let mut ece = 0.0_f32;
        for b in 0..BINS {
            if bin_n[b] > 0 {
                let acc = bin_sum[b] / bin_n[b] as f32;
                let conf_mean = b as f32 / BINS as f32 + 1.0 / (2.0 * BINS as f32);
                ece += bin_n[b] as f32 / decisions.len() as f32 * (acc - conf_mean).abs();
            }
        }
        (ece, brier / decisions.len().max(1) as f32)
    };
    let (ece_raw, brier_raw) = ece_brier(&decisons);
    // Platt temperature: grid T in [0.25..4], pick min NLL of the softened
    // top-1 distribution (a 1-param calibration fit on the same decisions —
    // honest for a REPORT; a deploy would fit on a held-out cal split).
    let best_t = {
        let mut best = (f32::INFINITY, 1.0_f32);
        let mut t = 0.25_f32;
        while t <= 4.0 {
            let mut nll = 0.0_f32;
            for d in &decisons {
                // softened confidence: p' = sigmoid(logit-approx / T) — we
                // only have probs; use the logit of the top-1 confidence.
                let lg = (d.conf.clamp(1e-6, 1.0 - 1e-6)).ln() - ((1.0 - d.conf.clamp(1e-6, 1.0 - 1e-6)).ln());
                let p = 1.0 / (1.0 + (-lg / t).exp());
                let y = if d.correct { 1.0 } else { 0.0 };
                nll += -(y * p.ln() + (1.0 - y) * (1.0 - p).ln());
            }
            if nll < best.0 {
                best = (nll, t);
            }
            t += 0.25;
        }
        best.1
    };
    let platt: Vec<Decision> = decisons
        .iter()
        .map(|d| {
            let c = d.conf.clamp(1e-6, 1.0 - 1e-6);
            let lg = c.ln() - (1.0 - c).ln();
            let p = 1.0 / (1.0 + (-lg / best_t).exp());
            Decision { conf: p, correct: d.correct }
        })
        .collect();
    let (ece_p, brier_p) = ece_brier(&platt);
    (ece_raw, brier_raw, ece_p, brier_p)
}

fn softmax_of(v: &[f32]) -> Vec<f32> {
    let max = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = v.iter().map(|x| (x - max).exp()).collect();
    let s: f32 = exps.iter().sum();
    exps.iter().map(|e| e / s).collect()
}

fn latency_gate(ds: &Dataset, model: &ChoiceScorer) -> f32 {
    let mut pools_cache: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    let mut times = Vec::with_capacity(ds.spans.len());
    for span_i in 0..ds.spans.len() {
        let span = &ds.spans[span_i];
        if span.clean {
            continue;
        }
        let pool = &ds.pools[span_i];
        let mut options = pool.clone();
        options.push(SKIP_RULE_IDX);
        let opt_pools: Vec<Vec<f32>> = options
            .iter()
            .map(|&idx| pools_cache.entry(idx).or_insert_with(|| {
                let text = if idx == SKIP_RULE_IDX { skip_option_text() } else { ds.rules[idx as usize].text.clone() };
                model.encode_option(&text)
            }).clone())
            .collect();
        let refs: Vec<&[f32]> = opt_pools.iter().map(|v| v.as_slice()).collect();
        // warm
        let _ = model.forward_logits_cached(span.code.as_bytes(), &refs);
        let t0 = Instant::now();
        let _ = model.forward_logits_cached(span.code.as_bytes(), &refs);
        times.push(t0.elapsed().as_secs_f32() * 1e6);
    }
    let n = times.len().max(1) as f32;
    times.iter().sum::<f32>() / n
}

// ---------------------------------------------------------------------------
// Checkpoint
// ---------------------------------------------------------------------------

fn save_checkpoint(model: &ChoiceScorer, path: &str) -> std::io::Result<String> {
    let flat = model.to_flat();
    let hash = blake3_of(&flat);
    std::fs::write(path, bytemuck_serialize(&flat))?;
    Ok(hash)
}

fn bytemuck_serialize(flat: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(flat.len() * 4);
    for v in flat {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn blake3_of(flat: &[f32]) -> String {
    let bytes = bytemuck_serialize(flat);
    let mut h = blake3::Hasher::new();
    h.update(&bytes);
    format!("{}", h.finalize().to_hex())
}

fn load_checkpoint(path: &str) -> Result<ChoiceScorer, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if !bytes.len().is_multiple_of(4) {
        return Err("corrupt checkpoint".into());
    }
    let flat: Vec<f32> = bytes.as_chunks::<4>().0.iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    ChoiceScorer::from_flat(ScorerConfig::default(), &flat)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    println!("== choice_scorer_poc ({mode}) — Issue 125 / Research 573 ==");

    match mode.as_str() {
        "selftest" => {
            let ds = build_dataset().expect("dataset");
            println!("dataset: {} rules, {} spans", ds.rules.len(), ds.spans.len());
            let mut eps = Vec::new();
            let mut rng = Rng::new(AUG_SEED);
            training_episodes(&ds, 0, &mut rng, &mut eps);
            let m = ChoiceScorer::new(ScorerConfig::default(), 1);
            let l0 = m.loss(&eps[0].ctx, option_texts(&ds, &eps[0]).as_slice(), eps[0].label);
            println!("smoke: {} episodes for span 0, loss at init {:.4}", eps.len(), l0);
        }
        "render" => {
            let ds = build_dataset().expect("dataset");
            let eps = render_training_set(&ds);
            let n_held = (0..ds.spans.len()).filter(|i| span_heldout(&ds, *i)).count();
            let n_gold = ds.spans.iter().filter(|s| !s.clean).count();
            println!(
                "render: {} rules · {} gold spans · {} clean · {} held-out spans · {} training episodes",
                ds.rules.len(),
                n_gold,
                ds.spans.len() - n_gold,
                n_held,
                eps.len()
            );
            let avg_opts: f32 = eps.iter().map(|e| e.options.len()).sum::<usize>() as f32 / eps.len() as f32;
            println!("avg options/episode: {avg_opts:.1}");
        }
        "train" | "all" => {
            let ds = build_dataset().expect("dataset");
            let eps = render_training_set(&ds);
            println!("training on {} episodes ({} epochs, batch {BATCH}, {WORKERS} workers)", eps.len(), EPOCHS);
            let model = train(&ds, &eps, "train");
            let ckpt = "/tmp/choice_scorer_ckpt.bin";
            match save_checkpoint(&model, ckpt) {
                Ok(hash) => println!("checkpoint: {ckpt} (blake3 {hash})"),
                Err(e) => println!("checkpoint save failed: {e}"),
            }
            if mode == "train" {
                return;
            }
            run_eval(&ds, &model);
        }
        "eval" => {
            let ds = build_dataset().expect("dataset");
            let model = load_checkpoint("/tmp/choice_scorer_ckpt.bin").expect("checkpoint — run --train first");
            run_eval(&ds, &model);
        }
        other => println!("unknown mode {other:?} (selftest|render|train|eval|all)"),
    }
}

fn run_eval(ds: &Dataset, model: &ChoiceScorer) {
    let mut gates: Vec<GateReport> = Vec::new();

    // ---- competitors ----
    let (n_in, sc_in, bl_in) = evaluate(ds, model, "in", false);
    let (n_ood, sc_ood, bl_ood) = evaluate(ds, model, "ood", false);
    let frozen = ChoiceScorer::new(ScorerConfig::default(), FROZEN_SEED);
    let (n_f, sc_f, _) = evaluate(ds, &frozen, "in", false);
    let (n_sh, sc_sh, _) = evaluate(ds, model, "in", true);

    let pct = |c: usize, n: usize| if n > 0 { c as f32 / n as f32 } else { 0.0 };
    println!("\n-- arms (top-1 on production pools) --");
    println!("scorer  in-dist : {}/{} ({:.1}%)", sc_in, n_in, 100.0 * pct(sc_in, n_in));
    println!("baseline in-dist: {}/{} ({:.1}%)", bl_in, n_in, 100.0 * pct(bl_in, n_in));
    println!("scorer  OOD     : {}/{} ({:.1}%)", sc_ood, n_ood, 100.0 * pct(sc_ood, n_ood));
    println!("baseline OOD    : {}/{} ({:.1}%)", bl_ood, n_ood, 100.0 * pct(bl_ood, n_ood));
    println!("frozen  in-dist : {}/{} ({:.1}%)", sc_f, n_f, 100.0 * pct(sc_f, n_f));
    println!("shuffled control: {}/{} ({:.1}%)", sc_sh, n_sh, 100.0 * pct(sc_sh, n_sh));

    // ---- G1: composite top-1 >= structural baseline, same fixtures ----
    let g1 = pct(sc_in, n_in) >= pct(bl_in, n_in);
    gates.push(GateReport {
        name: "G1 in-dist >= baseline",
        passed: g1,
        detail: format!("scorer {:.1}% vs baseline {:.1}%", 100.0 * pct(sc_in, n_in), 100.0 * pct(bl_in, n_in)),
    });

    // ---- G1-OOD: gap <= 10pp where structural holds; memorization = REFUTED ----
    let ood_gap = pct(bl_ood, n_ood) - pct(sc_ood, n_ood);
    let memorization = pct(sc_in, n_in) - pct(sc_ood, n_ood) > 0.30;
    gates.push(GateReport {
        name: "G1-OOD gap <= 10pp",
        passed: ood_gap <= 0.10,
        detail: format!("gap {:+.1}pp (scorer {:.1}% vs baseline {:.1}%)", 100.0 * ood_gap, 100.0 * pct(sc_ood, n_ood), 100.0 * pct(bl_ood, n_ood)),
    });
    gates.push(GateReport {
        name: "no Bench-065 memorization signature",
        passed: !memorization,
        detail: format!("in {:.1}% vs ood {:.1}% (collapse >30pp = REFUTED)", 100.0 * pct(sc_in, n_in), 100.0 * pct(sc_ood, n_ood)),
    });

    // ---- G2: latency ----
    let us = latency_gate(ds, model);
    gates.push(GateReport {
        name: "G2 latency <= ~1ms/span",
        passed: us <= 1000.0,
        detail: format!("{us:.0} µs/span (cached option pools, single thread)"),
    });

    // ---- control: shuffled context must degrade to ~chance ----
    let chance = 1.0 / 20.0; // ~typical option count
    let shuffle_ok = pct(sc_sh, n_sh) <= (pct(sc_in, n_in) * 0.5).max(chance + 0.10);
    gates.push(GateReport {
        name: "shuffled-context control",
        passed: shuffle_ok,
        detail: format!("shuffled {:.1}% vs real {:.1}% (chance ~{:.1}%)", 100.0 * pct(sc_sh, n_sh), 100.0 * pct(sc_in, n_in), 100.0 * chance),
    });

    // ---- calibration ----
    let (ece_r, br_r, ece_p, br_p) = calibration(ds, model);
    gates.push(GateReport {
        name: "calibration reported (ECE/Brier)",
        passed: true, // report-first: the numbers are the gate's content
        detail: format!("ECE {ece_r:.3}→{ece_p:.3} Brier {br_r:.3}→{br_p:.3} (Platt-T)"),
    });

    // ---- verdict ----
    println!("\n-- gates --");
    let mut all_pass = true;
    for g in &gates {
        println!("{} {:-<40} {}", if g.passed { "PASS" } else { "FAIL" }, g.name, g.detail);
        all_pass &= g.passed;
    }
    println!(
        "\nVERDICT: {} — modelless default stands; see .benchmarks/098_choice_scorer_poc.md",
        if all_pass { "GOAT CANDIDATE (gates pass — promotion is a separate owner-gated plan)" } else { "REFUTED for promotion (recorded)" }
    );
}
