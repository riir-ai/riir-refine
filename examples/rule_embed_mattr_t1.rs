//! Issue 133 T1 — the Matryoshka-k randomized-budget supervision harness
//! (`mattr_k_poc`, implies `rule_embed_poc`).
//!
//! Re-runs the Bench 099 two-tower training with randomized-k supervision
//! vs the shipped fixed objective, over the 049_t01 real-repo span corpus
//! (946 labeled spans / 1095 span↔lint pairs under today's 100-lint clippy
//! vocabulary), and evaluates at k ∈ {1,3,5,10,20} including held-out
//! widths (training draws k from {1,2,3,5,8}; depths 10/20 are never drawn).
//!
//! Arms (`--arms` csv, default all):
//! - `control_shipped_fixed` — the SHIPPED pair-structured sigmoid-NCE
//!   objective (`rule_embed::train`) on the union data (shipped recipe's
//!   self-pairs + fixture train pairs + the span-corpus train pairs). The
//!   control.
//! - `spanwise_full` — the span-structured full-pool BCE at k = pool every
//!   step. DIAGNOSTIC: isolates the budget-randomization axis from the
//!   loss-shape axis (same shape as the randomized arms, no randomization).
//! - `uniform_k_1_2_3_5_8` — per-step uniform budget draw, keep-first
//!   supervision only (the issue's arm b).
//! - `uniform_all_widths` — same over {1,2,3,5,8,10,20}: the withheld-width
//!   A/B (depths 10/20 seen vs unseen at train time).
//! - `logboth_k_1_2_3_5_8` — P(k) ∝ 1/k draws + keep-first AND
//!   exclude-last masks (the issue's arm c; measured for the record, never
//!   wired into `active_set`).
//!
//! The rule-level blake3 withheld split carries over: withheld lints' pairs
//! are the OOD read (852 of 1095 pairs here — doc_markdown alone is 518 and
//! IS withheld, so the issue's doc_markdown CAP arm is vacuous at training
//! time under this protocol; the cap is applied to the EVAL population
//! instead, on every arm, as the concentration guard).
//!
//! Every B1 condition is honored structurally: the density histogram is
//! printed FIRST; deep-k reads are labeled corpus-limited-by-density; macro
//! (per-lint) metrics ride beside micro on every read.
//!
//! Usage:
//! ```text
//! cargo run --release --features mattr_k_poc --example rule_embed_mattr_t1 -- \
//!     [--arms a,b,...] [--out-dir /tmp/mattr_t1] [--quick]
//! ```
//! `--quick` trains 120 steps per arm — a plumbing smoke, never a result.
//!
//! Determinism: seeded SplitMix64 init, full-batch fixed-op-order kernels,
//! deterministic bisection — same inputs, same artifact. No GPU, no new
//! deps (`exact_mass_admit` rides the existing katgpt-core path dep).

#![allow(clippy::cast_precision_loss)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use riir_refine::domain::{HealDomain, HealEntry};
use riir_refine::draft::domain_router::DomainId;
use riir_refine::draft::latent_matcher::LatentMatcher;
use riir_refine::draft::retrieval_eval::{canonical_rank, labeled_eval_spans, run_split_eval};
use riir_refine::draft::rule_embed::{
    decode_verified, embedded_tables, fixture_is_withheld, train, TrainConfig, TrainData, Trained,
    TrainedTables,
};
use riir_refine::draft::rule_embed_mattr::{
    train_spanwise, KBudget, MattrConfig, SpanRow, SpanTrainData,
};
use riir_refine::draft::{RerankMode, RetrievalConfig};

// ---------------------------------------------------------------------------
// The pinned corpus shape (Issue 133's 2026-10-08 re-check — the corpus
// builder is a REPRODUCTION of that measure; drift here is loud, not silent).
// ---------------------------------------------------------------------------

const EXPECT_LABELED: usize = 946;
const EXPECT_SINGLE: usize = 816;
const EXPECT_MULTI: usize = 130;
const EXPECT_PAIRS: usize = 1095;
const EXPECT_LINTS: usize = 18;
const EXPECT_TRAIN_PAIRS: usize = 243;
const EXPECT_WITHHELD_PAIRS: usize = 852;

/// The B1 concentration guard: doc_markdown (518 pairs, 47%) is capped at
/// the next-largest lint's pair count (suboptimal_flops, 206) in the CAPPED
/// eval population. Training-side capping is vacuous under the rule-level
/// split (doc_markdown is withheld), so the cap lives on the eval side.
const DOC_LINT: &str = "doc_markdown";
const DOC_MARKDOWN_CAP: usize = 206;

const KS: [usize; 5] = [1, 3, 5, 10, 20];
/// ≥ 20 + margin; ranks below the shipped PROBE_TOP_K window (8) are the
/// same ordering, just truncated later — hits@1/3/5 are window-stable.
const FUSED_DEPTH: usize = 24;
const WEIGHTS: [f32; 7] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];
const GATE_WEIGHT: f32 = 16.0; // the standing gate's PINNED_WEIGHT

const BUDGET_SET: &[u32] = &[1, 2, 3, 5, 8];
const ALL_WIDTHS: &[u32] = &[1, 2, 3, 5, 8, 10, 20];

const DEFAULT_OUT_DIR: &str = "/tmp/mattr_t1";

// ---------------------------------------------------------------------------
// Arms
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    ShippedControl,
    SpanwiseFull,
    Uniform,
    UniformAllWidths,
    LogBoth,
}

impl Arm {
    const ALL: [Arm; 5] = [
        Arm::ShippedControl,
        Arm::SpanwiseFull,
        Arm::Uniform,
        Arm::UniformAllWidths,
        Arm::LogBoth,
    ];

    fn tag(self) -> &'static str {
        match self {
            Arm::ShippedControl => "control_shipped_fixed",
            Arm::SpanwiseFull => "spanwise_full",
            Arm::Uniform => "uniform_k_1_2_3_5_8",
            Arm::UniformAllWidths => "uniform_all_widths_1_2_3_5_8_10_20",
            Arm::LogBoth => "logboth_k_1_2_3_5_8",
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Arm::ShippedControl => "shipped pair-NCE (fixed objective)",
            Arm::SpanwiseFull => "spanwise full-BCE (k = pool, no randomization)",
            Arm::Uniform => "spanwise BCE, k ~ U{1,2,3,5,8}, keep-first",
            Arm::UniformAllWidths => "spanwise BCE, k ~ U{1,2,3,5,8,10,20}, keep-first",
            Arm::LogBoth => "spanwise BCE, k ~ P(∝1/k){1,2,3,5,8}, keep-first + exclude-last",
        }
    }

    fn from_tag(t: &str) -> Option<Arm> {
        Arm::ALL.into_iter().find(|a| a.tag() == t)
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy)]
struct KStats {
    pairs: u32,
    hits: [u32; KS.len()],
}

impl KStats {
    fn bump(&mut self, rank: Option<usize>) {
        self.pairs += 1;
        for (i, &k) in KS.iter().enumerate() {
            if rank.is_some_and(|r| r < k) {
                self.hits[i] += 1;
            }
        }
    }

    /// Recall at index i; NaN when the bucket has no pairs (never pooled).
    fn recall(&self, i: usize) -> f64 {
        if self.pairs == 0 {
            f64::NAN
        } else {
            f64::from(self.hits[i]) / f64::from(self.pairs)
        }
    }
}

/// Curves over the span corpus. Split 0 = train-split pairs (in-sample),
/// split 1 = withheld-split pairs (the OOD read). `capped_*` recompute the
/// same over the doc_markdown-capped population (B1 condition 2).
#[derive(Debug, Default)]
struct SplitCurves {
    micro: [KStats; 2],
    per_lint: HashMap<String, [KStats; 2]>,
    capped_micro: [KStats; 2],
    capped_per_lint: HashMap<String, [KStats; 2]>,
}

impl SplitCurves {
    /// Macro recall at index i over the split's lints (≥1 pair).
    fn macro_recall(&self, split: usize, i: usize, capped: bool) -> (f64, usize) {
        let map = if capped {
            &self.capped_per_lint
        } else {
            &self.per_lint
        };
        let mut sum = 0.0_f64;
        let mut n = 0_usize;
        for stats in map.values() {
            if stats[split].pairs > 0 {
                sum += stats[split].recall(i);
                n += 1;
            }
        }
        (
            if n == 0 {
                f64::NAN
            } else {
                sum / f64::from(n as u32)
            },
            n,
        )
    }
}

/// JSONL-safe float: -1.0 stands for n/a (NaN never serializes).
fn jf(x: f64) -> f64 {
    if x.is_finite() {
        x
    } else {
        -1.0
    }
}

struct GoldPair {
    lint: String,
    withheld: bool,
    capped_in: bool,
}

struct EvalSpan {
    code: String,
    golds: Vec<GoldPair>,
}

// ---------------------------------------------------------------------------
// Output sinks
// ---------------------------------------------------------------------------

struct Sinks {
    out_dir: PathBuf,
    jsonl_path: PathBuf,
    md: String,
}

impl Sinks {
    fn new(out_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&out_dir);
        Self {
            jsonl_path: out_dir.join("results.jsonl"),
            md: String::new(),
            out_dir,
        }
    }

    fn md(&mut self, line: &str) {
        println!("{line}");
        self.md.push_str(line);
        self.md.push('\n');
    }

    fn jsonl(&self, value: serde_json::Value) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.jsonl_path)
        {
            let _ = writeln!(f, "{value}");
        }
    }

    fn flush(self, run_tag: &str) -> (PathBuf, PathBuf) {
        let path = self.out_dir.join(format!("report_{run_tag}.md"));
        let _ = std::fs::write(&path, &self.md);
        (path, self.jsonl_path)
    }
}

/// One arm/posture's curves → JSONL + the human table.
fn emit_curves(sinks: &mut Sinks, arm: &str, posture: &str, curves: &SplitCurves) {
    let split_names = ["train", "withheld"];
    for (s, split) in split_names.iter().enumerate() {
        for (i, &k) in KS.iter().enumerate() {
            let m = &curves.micro[s];
            sinks.jsonl(serde_json::json!({
                "arm": arm, "posture": posture, "split": split, "lint": "MICRO",
                "k": k, "pairs": m.pairs, "hits": m.hits[i], "recall": jf(m.recall(i)),
            }));
            let (mac, lints) = curves.macro_recall(s, i, false);
            sinks.jsonl(serde_json::json!({
                "arm": arm, "posture": posture, "split": split, "lint": "MACRO",
                "k": k, "lints": lints, "recall": jf(mac),
            }));
            let mc = &curves.capped_micro[s];
            sinks.jsonl(serde_json::json!({
                "arm": arm, "posture": posture, "split": format!("capped_{split}"), "lint": "MICRO",
                "k": k, "pairs": mc.pairs, "hits": mc.hits[i], "recall": jf(mc.recall(i)),
            }));
            let (mac_c, lints_c) = curves.macro_recall(s, i, true);
            sinks.jsonl(serde_json::json!({
                "arm": arm, "posture": posture, "split": format!("capped_{split}"), "lint": "MACRO",
                "k": k, "lints": lints_c, "recall": jf(mac_c),
            }));
        }
        // Per-lint rows (the B1 macro read's atoms — every lint, every k,
        // both splits; the top-4 concentration read slices these).
        let mut lints: Vec<&String> = curves.per_lint.keys().collect();
        lints.sort();
        for lint in lints {
            let st = &curves.per_lint[lint];
            for (i, &k) in KS.iter().enumerate() {
                sinks.jsonl(serde_json::json!({
                    "arm": arm, "posture": posture, "split": split, "lint": lint,
                    "k": k, "pairs": st[s].pairs, "hits": st[s].hits[i],
                    "recall": jf(st[s].recall(i)),
                }));
            }
        }
    }
    for (s, split) in split_names.iter().enumerate() {
        let m = &curves.micro[s];
        let cells: String = KS
            .iter()
            .enumerate()
            .map(|(i, k)| format!("k={k} {:5.1}%", 100.0 * m.recall(i)))
            .collect::<Vec<_>>()
            .join("  ");
        let (mac0, _) = curves.macro_recall(s, 0, false);
        let (mac4, _) = curves.macro_recall(s, 4, false);
        sinks.md(&format!(
            "  {arm:<30} {posture:<14} {split:<9} pairs {:5} | {cells} | macro@1 {:5.1}% macro@20 {:5.1}%",
            m.pairs,
            100.0 * mac0,
            100.0 * mac4,
        ));
        let mc = &curves.capped_micro[s];
        let cells_c: String = KS
            .iter()
            .enumerate()
            .map(|(i, k)| format!("k={k} {:5.1}%", 100.0 * mc.recall(i)))
            .collect::<Vec<_>>()
            .join("  ");
        let (macc0, _) = curves.macro_recall(s, 0, true);
        let (macc4, _) = curves.macro_recall(s, 4, true);
        sinks.md(&format!(
            "  {arm:<30} {posture:<14} cap-{split:<7} pairs {:5} | {cells_c} | macro@1 {:5.1}% macro@20 {:5.1}%",
            mc.pairs,
            100.0 * macc0,
            100.0 * macc4,
        ));
    }
}

// ---------------------------------------------------------------------------
// Rule book + corpus
// ---------------------------------------------------------------------------

struct Book {
    /// `(rule_id, buggy\nfixed)` — every enabled-domain rule.
    rule_texts: Vec<(String, String)>,
    index: HashMap<String, usize>,
    /// Sorted clippy rule ids — the spans.jsonl ranking pool.
    clippy_pool: Vec<String>,
}

fn clippy_triples() -> Vec<(String, String, String)> {
    #[cfg(feature = "clippy_lints")]
    return riir_refine::domains::clippy_lints::ClippyLintDomain::build_corpus()
        .into_iter()
        .map(|e| {
            (
                e.rule_id().to_string(),
                e.buggy_code().to_string(),
                e.fixed_code().to_string(),
            )
        })
        .collect();
    #[cfg(not(feature = "clippy_lints"))]
    Vec::new()
}

fn aux_triples() -> Vec<(String, String, String)> {
    let mut triples = Vec::new();
    #[cfg(feature = "rust_perf")]
    triples.extend(
        riir_refine::domains::rust_perf::RustPerfDomain::build_corpus()
            .into_iter()
            .map(|e| {
                (
                    e.rule_id().to_string(),
                    e.buggy_code().to_string(),
                    e.fixed_code().to_string(),
                )
            }),
    );
    #[cfg(feature = "docker")]
    triples.extend(
        riir_refine::domains::docker::DockerDomain::build_corpus()
            .into_iter()
            .map(|e| {
                (
                    e.rule_id().to_string(),
                    e.buggy_code().to_string(),
                    e.fixed_code().to_string(),
                )
            }),
    );
    #[cfg(feature = "dist")]
    triples.extend(
        riir_refine::domains::dist::DistDomain::build_corpus()
            .into_iter()
            .map(|e| {
                (
                    e.rule_id().to_string(),
                    e.buggy_code().to_string(),
                    e.fixed_code().to_string(),
                )
            }),
    );
    triples
}

fn build_book() -> Book {
    let mut triples = clippy_triples();
    let clippy_n = triples.len();
    triples.extend(aux_triples());
    let mut rule_texts: Vec<(String, String)> = Vec::with_capacity(triples.len());
    let mut index = HashMap::with_capacity(triples.len());
    for (id, buggy, fixed) in triples {
        let text = format!("{buggy}\n{fixed}");
        if index.insert(id.clone(), rule_texts.len()).is_some() {
            panic!("duplicate rule id {id:?} across domains — the book must be unique");
        }
        rule_texts.push((id, text));
    }
    let mut clippy_pool: Vec<String> = rule_texts[..clippy_n]
        .iter()
        .map(|(id, _)| id.clone())
        .collect();
    clippy_pool.sort();
    Book {
        rule_texts,
        index,
        clippy_pool,
    }
}

/// The spans.jsonl corpus builder — a REPRODUCTION of Issue 133's re-check
/// intersection (oracle lint arrays ∩ today's clippy vocabulary). Returns
/// the eval spans + the per-lint pair histogram in file order.
fn load_span_corpus(vocab: &HashSet<String>) -> (Vec<EvalSpan>, Vec<(String, usize)>) {
    let path = std::env::var("MATTR_SPANS_PATH").unwrap_or_else(|_| {
        format!(
            "{}/.benchmarks/data/049_t01/spans.jsonl",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {path} failed: {e} — the supervision corpus is required"));
    let mut spans = Vec::new();
    let mut pair_counts: HashMap<String, usize> = HashMap::new();
    let mut doc_md_seen = 0_usize;
    let mut labeled = 0_usize;
    let mut single = 0_usize;
    let mut multi = 0_usize;
    let mut pairs = 0_usize;
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("spans.jsonl row unparsable: {e}"));
        let code = row["code"].as_str().unwrap_or_default().to_string();
        let lints = row["lints"].as_array().expect("lints array");
        let mut golds: Vec<String> = lints
            .iter()
            .filter_map(|v| v.as_str())
            .filter(|l| vocab.contains(*l))
            .map(String::from)
            .collect();
        golds.sort();
        golds.dedup();
        if golds.is_empty() {
            continue;
        }
        labeled += 1;
        if golds.len() == 1 {
            single += 1;
        } else {
            multi += 1;
        }
        let mut gps = Vec::with_capacity(golds.len());
        for lint in golds {
            *pair_counts.entry(lint.clone()).or_default() += 1;
            pairs += 1;
            let capped_in = if lint == DOC_LINT {
                let keep = doc_md_seen < DOC_MARKDOWN_CAP;
                doc_md_seen += 1;
                keep
            } else {
                true
            };
            gps.push(GoldPair {
                withheld: fixture_is_withheld(&lint),
                capped_in,
                lint,
            });
        }
        spans.push(EvalSpan { code, golds: gps });
    }

    // Loud reproduction checks — the issue's re-check numbers ARE the spec.
    let check = |cond: bool, what: &str, measured: String| {
        assert!(
            cond,
            "corpus drift vs the Issue-133 re-check: {what} = {measured}"
        );
    };
    check(
        labeled == EXPECT_LABELED,
        "labeled spans",
        format!("{labeled} (want {EXPECT_LABELED})"),
    );
    check(
        single == EXPECT_SINGLE,
        "single-label",
        format!("{single} (want {EXPECT_SINGLE})"),
    );
    check(
        multi == EXPECT_MULTI,
        "multi-label",
        format!("{multi} (want {EXPECT_MULTI})"),
    );
    check(
        pairs == EXPECT_PAIRS,
        "pairs",
        format!("{pairs} (want {EXPECT_PAIRS})"),
    );
    check(
        pair_counts.len() == EXPECT_LINTS,
        "distinct lints",
        format!("{} (want {EXPECT_LINTS})", pair_counts.len()),
    );
    let train_pairs: usize = pair_counts
        .iter()
        .filter(|(l, _)| !fixture_is_withheld(l))
        .map(|(_, c)| *c)
        .sum();
    check(
        train_pairs == EXPECT_TRAIN_PAIRS,
        "train-split pairs",
        format!("{train_pairs} (want {EXPECT_TRAIN_PAIRS})"),
    );
    check(
        pairs - train_pairs == EXPECT_WITHHELD_PAIRS,
        "withheld pairs",
        format!("{} (want {EXPECT_WITHHELD_PAIRS})", pairs - train_pairs),
    );

    let mut by_count: Vec<(String, usize)> = pair_counts.into_iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    (spans, by_count)
}

fn print_histogram(sinks: &mut Sinks, by_count: &[(String, usize)]) {
    sinks.md("## 0. Label-density histogram (B1 condition 1 — recorded FIRST)");
    sinks.md(&format!(
        "in-vocab: {EXPECT_LABELED} spans / {EXPECT_PAIRS} pairs — {EXPECT_SINGLE} single + \
         {EXPECT_MULTI} multi = 1.16 labels/span; rule-level blake3 split: train \
         {EXPECT_TRAIN_PAIRS} pairs vs withheld {EXPECT_WITHHELD_PAIRS} pairs"
    ));
    let total: usize = by_count.iter().map(|(_, c)| *c).sum();
    let mut cum = 0_usize;
    for (lint, c) in by_count {
        cum += c;
        sinks.md(&format!(
            "  {lint:<36} {c:4} pairs ({:4.1}% cum {:5.1}%)  split: {}",
            100.0 * f64::from(*c as u32) / f64::from(total as u32),
            100.0 * f64::from(cum as u32) / f64::from(total as u32),
            if fixture_is_withheld(lint) {
                "WITHHELD"
            } else {
                "train"
            },
        ));
    }
    let top4: usize = by_count.iter().take(4).map(|(_, c)| *c).sum();
    sinks.md(&format!(
        "  top-4 = {:.0}% of all pairs. doc_markdown is WITHHELD → its training-side cap is vacuous; \
         the B1 concentration guard runs as the CAPPED eval population (doc_markdown → \
         {DOC_MARKDOWN_CAP} pairs) on every arm.",
        100.0 * f64::from(top4 as u32) / f64::from(total as u32),
    ));
    sinks.md(
        "  Interpretation rule (pinned): a NEGATIVE at deep k (10, 20) reads \
         \"this corpus cannot test Matryoshka-k\" (corpus-limited, 1.16 labels/span), NEVER \
         \"Matryoshka-k loses\"; only shallow-k (top-1/top-3) vs the shipped fixed objective is \
         a mechanism verdict on this corpus.",
    );
}

// ---------------------------------------------------------------------------
// Ranking postures
// ---------------------------------------------------------------------------

fn ranked_fused(matcher: &mut LatentMatcher, code: &str) -> Vec<String> {
    let (_, by_domain) = matcher.suggest_span(code, FUSED_DEPTH);
    by_domain
        .iter()
        .find(|(d, _)| *d == DomainId::Clippy)
        .map(|(_, rules)| {
            canonical_rank(rules)
                .into_iter()
                .map(|r| r.rule_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn ranked_tower(tables: &TrainedTables, code: &str, pool: &[String]) -> Vec<String> {
    let phi = tables.project_span(code);
    let mut scored: Vec<(f32, &String)> = pool
        .iter()
        .filter_map(|id| tables.score_rule(&phi, id).map(|s| (s, id)))
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    scored.into_iter().map(|(_, id)| id.clone()).collect()
}

fn run_curves(spans: &[EvalSpan], mut ranked: impl FnMut(&str) -> Vec<String>) -> SplitCurves {
    let mut c = SplitCurves::default();
    for span in spans {
        let list = ranked(&span.code);
        for g in &span.golds {
            let rank = list.iter().position(|id| id == &g.lint);
            let s = usize::from(g.withheld);
            c.micro[s].bump(rank);
            c.per_lint.entry(g.lint.clone()).or_default()[s].bump(rank);
            if g.capped_in {
                c.capped_micro[s].bump(rank);
                c.capped_per_lint.entry(g.lint.clone()).or_default()[s].bump(rank);
            }
        }
    }
    c
}

// ---------------------------------------------------------------------------
// The fixture rig (Bench 099 standing gates)
// ---------------------------------------------------------------------------

struct RigRow {
    w: f32,
    baseline_train: usize,
    n_train: usize,
    baseline_withheld: usize,
    n_withheld: usize,
    trained_train: usize,
    trained_withheld: usize,
    regressions: usize,
    clean_b: usize,
    clean_t: usize,
    /// G1 anatomy: labels the baseline held at top-1 that this posture lost.
    regressed: Vec<String>,
}

fn rig_sweep(tables: &Arc<TrainedTables>) -> Vec<RigRow> {
    WEIGHTS
        .iter()
        .map(|&w| {
            let rep = run_split_eval(tables, w);
            let regressed = rep
                .miss_detail
                .iter()
                .filter(|(_, _, b, t)| b.is_some_and(|r| r == 0) && !t.is_some_and(|r| r == 0))
                .map(|(label, _, _, _)| label.clone())
                .collect();
            RigRow {
                w,
                baseline_train: rep.baseline_train.hits_at_1,
                n_train: rep.baseline_train.labeled,
                baseline_withheld: rep.baseline_withheld.hits_at_1,
                n_withheld: rep.baseline_withheld.labeled,
                trained_train: rep.trained_train.hits_at_1,
                trained_withheld: rep.trained_withheld.hits_at_1,
                regressions: rep.regressions_vs_baseline,
                clean_b: rep.clean_top1_fixable_baseline,
                clean_t: rep.clean_top1_fixable_trained,
                regressed,
            }
        })
        .collect()
}

/// argmax on the fixture TRAIN split, ties → smaller weight (the shipped
/// example's convention; rows are in ascending weight order).
fn pin_weight(rows: &[RigRow]) -> f32 {
    let mut best = 0;
    for (i, r) in rows.iter().enumerate() {
        if r.trained_train > rows[best].trained_train {
            best = i;
        }
    }
    rows[best].w
}

fn emit_rig(sinks: &mut Sinks, arm: &str, rows: &[RigRow], pinned: f32) {
    sinks.md(&format!("  fixture rig sweep ({arm}):"));
    for r in rows {
        sinks.md(&format!(
            "    w={:<5} train {}/{}  withheld {}/{}  regressions {}  clean {}/{}{}",
            r.w,
            r.trained_train,
            r.n_train,
            r.trained_withheld,
            r.n_withheld,
            r.regressions,
            r.clean_b,
            r.clean_t,
            if r.w == pinned { "  <- pinned" } else { "" },
        ));
        sinks.jsonl(serde_json::json!({
            "arm": arm, "section": "rig", "w": r.w,
            "baseline_train": r.baseline_train, "n_train": r.n_train,
            "baseline_withheld": r.baseline_withheld, "n_withheld": r.n_withheld,
            "trained_train": r.trained_train, "trained_withheld": r.trained_withheld,
            "regressions": r.regressions, "clean_b": r.clean_b, "clean_t": r.clean_t,
            "regressed_labels": r.regressed,
            "pinned": r.w == pinned,
        }));
    }
}

// ---------------------------------------------------------------------------
// Arm data + training
// ---------------------------------------------------------------------------

struct ArmData {
    pairs_data: TrainData,
    spanwise_data: SpanTrainData,
    n_span_spans: usize,
    n_span_pairs_train: usize,
    n_fixture_pairs: usize,
    n_self_pairs: usize,
}

fn build_arm_data(book: &Book, eval_spans: &[EvalSpan]) -> ArmData {
    // (a) the shipped recipe's self-pairs (non-withheld rules' buggy text).
    let mut self_pairs: Vec<(String, String)> = Vec::new();
    let mut self_span_rows: Vec<SpanRow> = Vec::new();
    for (i, (id, text)) in book.rule_texts.iter().enumerate() {
        if fixture_is_withheld(id) {
            continue;
        }
        let buggy = text.lines().next().unwrap_or_default().to_string();
        self_pairs.push((buggy.clone(), id.clone()));
        self_span_rows.push(SpanRow {
            text: buggy,
            golds: vec![i],
            forbidden: vec![],
        });
    }
    // (b) the fixture train-split pairs — kept in EVERY arm so the
    // fixture-rig comparison vs the shipped asset is apples-to-apples.
    let mut fixture_pairs: Vec<(String, String)> = Vec::new();
    let mut fixture_span_rows: Vec<SpanRow> = Vec::new();
    for span in labeled_eval_spans() {
        if fixture_is_withheld(&span.label) {
            continue;
        }
        let Some(&idx) = book.index.get(&span.label) else {
            panic!("fixture label {:?} missing from the rule book", span.label);
        };
        fixture_pairs.push((span.code.clone(), span.label.clone()));
        fixture_span_rows.push(SpanRow {
            text: span.code,
            golds: vec![idx],
            forbidden: vec![],
        });
    }
    // (c) the spans.jsonl train-split supervision (the T1 addition); the
    // withheld golds ride along as `forbidden` (excluded in BOTH directions).
    let mut span_pairs: Vec<(String, String)> = Vec::new();
    let mut span_rows: Vec<SpanRow> = Vec::new();
    for es in eval_spans {
        let golds: Vec<usize> = es
            .golds
            .iter()
            .filter(|g| !g.withheld)
            .filter_map(|g| book.index.get(&g.lint).copied())
            .collect();
        let forbidden: Vec<usize> = es
            .golds
            .iter()
            .filter(|g| g.withheld)
            .filter_map(|g| book.index.get(&g.lint).copied())
            .collect();
        if golds.is_empty() {
            continue;
        }
        for g in &golds {
            span_pairs.push((es.code.clone(), book.rule_texts[*g].0.clone()));
        }
        span_rows.push(SpanRow {
            text: es.code.clone(),
            golds,
            forbidden,
        });
    }
    let n_span_pairs_train = span_pairs.len();
    let n_span_spans = span_rows.len();
    let n_fixture_pairs = fixture_pairs.len();
    let n_self_pairs = self_pairs.len();

    let mut pairs = fixture_pairs;
    pairs.extend(span_pairs);
    pairs.extend(self_pairs);
    let mut spans = fixture_span_rows;
    spans.extend(span_rows);
    spans.extend(self_span_rows);

    ArmData {
        pairs_data: TrainData {
            pairs,
            rule_texts: book.rule_texts.clone(),
        },
        spanwise_data: SpanTrainData {
            spans,
            rule_texts: book.rule_texts.clone(),
        },
        n_span_spans,
        n_span_pairs_train,
        n_fixture_pairs,
        n_self_pairs,
    }
}

fn train_arm(arm: Arm, data: &ArmData, steps: usize) -> Result<Trained, String> {
    match arm {
        Arm::ShippedControl => {
            let cfg = TrainConfig {
                steps,
                ..TrainConfig::default()
            };
            train(&data.pairs_data, &cfg)
        }
        Arm::SpanwiseFull | Arm::Uniform | Arm::UniformAllWidths | Arm::LogBoth => {
            let budget = match arm {
                Arm::SpanwiseFull => KBudget::Full,
                Arm::Uniform => KBudget::Uniform(BUDGET_SET),
                Arm::UniformAllWidths => KBudget::Uniform(ALL_WIDTHS),
                _ => KBudget::LogBoth(BUDGET_SET),
            };
            let cfg = MattrConfig {
                base: TrainConfig {
                    steps,
                    ..TrainConfig::default()
                },
                budget,
                ..MattrConfig::default()
            };
            train_spanwise(&data.spanwise_data, &cfg)
        }
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut arms: Vec<Arm> = Arm::ALL.to_vec();
    let mut out_dir = PathBuf::from(DEFAULT_OUT_DIR);
    let mut steps = TrainConfig::default().steps;
    let mut eval_only = false;
    let mut args_iter = args.iter();
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--quick" => steps = 120,
            "--eval-only" => eval_only = true,
            "--arms" => {
                let list = args_iter.next().expect("--arms needs a csv list");
                arms = list
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|t| Arm::from_tag(t).unwrap_or_else(|| panic!("unknown arm {t:?}")))
                    .collect();
                assert!(!arms.is_empty(), "--arms selected nothing");
            }
            "--out-dir" => {
                out_dir = PathBuf::from(args_iter.next().expect("--out-dir needs a path"));
            }
            other => panic!("unknown flag {other}"),
        }
    }
    let quick = steps != TrainConfig::default().steps;
    let run_tag = arms.iter().map(|a| a.tag()).collect::<Vec<_>>().join("+");
    let run_tag = if quick {
        format!("quick_{run_tag}")
    } else {
        run_tag
    };
    let mut sinks = Sinks::new(out_dir);

    sinks.md(&format!(
        "# Issue 133 T1 — Matryoshka-k harness | arms: {run_tag} | steps {steps}{}",
        if quick { " (--quick smoke)" } else { "" },
    ));

    // ---- corpus + density histogram (FIRST, per B1) ------------------------
    let t0 = Instant::now();
    let book = build_book();
    let vocab: HashSet<String> = book.clippy_pool.iter().cloned().collect();
    let (eval_spans, by_count) = load_span_corpus(&vocab);
    print_histogram(&mut sinks, &by_count);
    sinks.md(&format!(
        "  rule book: {} rules (clippy pool {}) | corpus load {:.1}s",
        book.rule_texts.len(),
        book.clippy_pool.len(),
        t0.elapsed().as_secs_f32(),
    ));

    // ---- sanity: the shipped asset reproduces the standing pins ------------
    sinks.md("\n## 1. Harness sanity — the shipped frozen asset vs the standing pins");
    let t1 = Instant::now();
    let shipped = Arc::new(embedded_tables().clone());
    let rows = rig_sweep(&shipped);
    let pinned = pin_weight(&rows);
    emit_rig(&mut sinks, "shipped_asset", &rows, pinned);
    let g = rows
        .iter()
        .find(|r| r.w == GATE_WEIGHT)
        .expect("gate weight row");
    sinks.md(&format!(
        "  at the gate's pinned w={GATE_WEIGHT}: baseline {}/{} train + {}/{} withheld | \
         trained {}/{} train + {}/{} withheld | regressions {} | clean {}/{} \
         (floors: baseline >= 90/34, trained >= 95/34, clean <= 31, G1 = 0)",
        g.baseline_train,
        g.n_train,
        g.baseline_withheld,
        g.n_withheld,
        g.trained_train,
        g.n_train,
        g.trained_withheld,
        g.n_withheld,
        g.regressions,
        g.clean_b,
        g.clean_t,
    ));
    let shipped_total_at_gate = g.trained_train + g.trained_withheld;
    sinks.jsonl(serde_json::json!({
        "arm": "shipped_asset", "section": "sanity", "w_gate": GATE_WEIGHT,
        "trained_train": g.trained_train, "trained_withheld": g.trained_withheld,
        "baseline_train": g.baseline_train, "baseline_withheld": g.baseline_withheld,
        "regressions": g.regressions, "clean_b": g.clean_b, "clean_t": g.clean_t,
    }));
    sinks.md(&format!("  sanity done {:.1}s", t1.elapsed().as_secs_f32()));

    // ---- baseline fused curves over the span corpus ------------------------
    sinks.md("\n## 2. The shipped fused scorer (baseline posture) over the span corpus");
    let t2 = Instant::now();
    let mut baseline_matcher = LatentMatcher::from_enabled_domains_with(RetrievalConfig::default())
        .expect("build matcher")
        .with_rerank_mode(RerankMode::default());
    let baseline_curves = run_curves(&eval_spans, |code| {
        ranked_fused(&mut baseline_matcher, code)
    });
    emit_curves(&mut sinks, "-", "fused_baseline", &baseline_curves);
    sinks.jsonl(serde_json::json!({
        "arm": "-", "section": "done_posture", "posture": "fused_baseline",
        "secs": t2.elapsed().as_secs_f32(),
    }));

    // ---- arms ---------------------------------------------------------------
    sinks.md("\n## 3. Arms");
    let data = if eval_only {
        None
    } else {
        Some(build_arm_data(&book, &eval_spans))
    };
    if let Some(d) = &data {
        sinks.md(&format!(
            "  data: {} fixture pairs + {} span-corpus pairs ({} spans) + {} self-pairs = {} pair \
             rows (spanwise: {} span rows)",
            d.n_fixture_pairs,
            d.n_span_pairs_train,
            d.n_span_spans,
            d.n_self_pairs,
            d.pairs_data.pairs.len(),
            d.spanwise_data.spans.len(),
        ));
    }
    for arm in arms {
        sinks.md(&format!("\n### arm {} — {}\n", arm.tag(), arm.kind()));
        let t3 = Instant::now();
        let artifact_path = sinks.out_dir.join(format!("{}.bin", arm.tag()));
        let tr = if eval_only {
            // Load the trained artifact from disk — the eval passes re-run
            // over the SAME bytes (bit-honest re-measure, no retrain).
            let bytes = std::fs::read(&artifact_path)
                .unwrap_or_else(|e| panic!("read {}: {e}", artifact_path.display()));
            Trained {
                tables: decode_verified(&bytes)
                    .unwrap_or_else(|e| panic!("decode {}: {e}", artifact_path.display())),
                bytes,
                final_loss: f32::NAN,
                n_pairs: 0,
                n_rules: 0,
                steps: 0,
            }
        } else {
            let tr =
                train_arm(arm, data.as_ref().expect("train data"), steps).unwrap_or_else(|e| {
                    eprintln!("train failed for {}: {e}", arm.tag());
                    std::process::exit(1);
                });
            std::fs::write(&artifact_path, &tr.bytes)
                .unwrap_or_else(|e| panic!("write artifact failed: {e}"));
            tr
        };
        let digest: String = format!("{}", blake3::hash(&tr.bytes).to_hex())[..16].to_string();
        if eval_only {
            sinks.md(&format!(
                "  eval-only: loaded artifact {} bytes blake3 {digest} from {} ({:.1}s)",
                tr.bytes.len(),
                artifact_path.display(),
                t3.elapsed().as_secs_f32(),
            ));
        } else {
            sinks.md(&format!(
                "  trained: {} steps, final loss {:.4}, {} pairs, {} rules, artifact {} bytes \
                 blake3 {digest} -> {} ({:.1}s)",
                tr.steps,
                tr.final_loss,
                tr.n_pairs,
                tr.n_rules,
                tr.bytes.len(),
                artifact_path.display(),
                t3.elapsed().as_secs_f32(),
            ));
        }
        sinks.jsonl(serde_json::json!({
            "arm": arm.tag(), "section": "meta", "steps": tr.steps,
            "final_loss": jf(f64::from(tr.final_loss)), "n_pairs": tr.n_pairs, "n_rules": tr.n_rules,
            "bytes": tr.bytes.len(), "blake3_16": digest,
            "secs": t3.elapsed().as_secs_f32(), "eval_only": eval_only,
        }));

        // fixture rig + gates
        let tables = Arc::new(tr.tables.clone());
        let rows = rig_sweep(&tables);
        let w = pin_weight(&rows);
        emit_rig(&mut sinks, arm.tag(), &rows, w);
        let r = rows.iter().find(|r| r.w == w).expect("pinned row present");
        let g1 = r.regressions == 0;
        let total = r.trained_train + r.trained_withheld;
        let b_total = r.baseline_train + r.baseline_withheld;
        let g2 = total > b_total;
        let g2_ood = r.trained_withheld >= r.baseline_withheld;
        let g3 = r.clean_t <= r.clean_b;
        let beats_shipped = total > shipped_total_at_gate;
        sinks.md(&format!(
            "  gates @ pinned w={w}: G1(no-regress) {} | G2 full-rig {total} vs baseline \
             {b_total} {} | G2-OOD withheld {}/{} vs baseline {}/{} {} | G3 clean {}/{} {} | \
             vs shipped asset at w=16 ({shipped_total_at_gate}): beats_shipped {beats_shipped}",
            verdict(g1),
            verdict(g2),
            r.trained_withheld,
            r.n_withheld,
            r.baseline_withheld,
            r.n_withheld,
            verdict(g2_ood),
            r.clean_t,
            r.clean_b,
            verdict(g3),
        ));
        if !r.regressed.is_empty() {
            sinks.md(&format!("  G1 regressed spans: {:?}", r.regressed));
        }
        sinks.jsonl(serde_json::json!({
            "arm": arm.tag(), "section": "gates", "pinned_w": w,
            "g1": g1, "g2": g2, "g2_ood": g2_ood, "g3": g3,
            "trained_total": total, "baseline_total": b_total,
            "shipped_total_at_gate": shipped_total_at_gate,
            "beats_shipped": beats_shipped,
        }));

        // span-corpus curves: tower-only + fused-at-pinned-w
        let t4 = Instant::now();
        let tower = run_curves(&eval_spans, |code| {
            ranked_tower(&tr.tables, code, &book.clippy_pool)
        });
        emit_curves(&mut sinks, arm.tag(), "tower_only", &tower);
        let mut trained_matcher =
            LatentMatcher::from_enabled_domains_with(RetrievalConfig::default())
                .expect("build matcher")
                .with_rerank_mode(RerankMode::default())
                .with_rule_embed(&tables, w);
        let fused = run_curves(&eval_spans, |code| ranked_fused(&mut trained_matcher, code));
        emit_curves(&mut sinks, arm.tag(), "fused_trained", &fused);
        sinks.jsonl(serde_json::json!({
            "arm": arm.tag(), "section": "done_posture", "posture": "curves",
            "secs": t4.elapsed().as_secs_f32(),
        }));
    }

    let jsonl_path = sinks.jsonl_path.clone();
    let (report_path, _) = sinks.flush(&run_tag);
    println!(
        "\nreport: {}\njsonl:  {}",
        report_path.display(),
        jsonl_path.display()
    );
}

fn verdict(ok: bool) -> &'static str {
    if ok {
        "PASS"
    } else {
        "FAIL"
    }
}
