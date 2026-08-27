//! PAV data curation — plan 356 A1's CPU-side harness.
//!
//! Builds the **(prefix, MC-Q) dataset** that Phase A's edit-scorer trains
//! on (arXiv:2410.08146 / katgpt-rs Research 509): for every span, drive
//! n_cov coverage rollouts + n_mc top-up continuations through the
//! [`EditGenerator`] seam, estimate each visited prefix's success
//! probability from the shared-suffix samples, cut hopeless tails with
//! first-pit curation, and emit JSONL pairs plus a manifest.
//!
//! ## What is CPU and what is GPU
//!
//! Everything here is CPU except the generator: [`DaemonEditGenerator`]
//! (the riir-train `bonsai_clippy_l4_eval --serve` daemon) is the GPU half
//! — the ~30–40 h exclusive-4090 campaign budget in the plan is daemon
//! calls. Tests and dry-runs use [`FnGenerator`] and cost microseconds.
//!
//! ## The low-budget law, structurally
//!
//! Research 509: *n_cov > n_mc at low budget*. The prefix tree makes that
//! the default economy: one rollout donates one Bernoulli sample to EVERY
//! prefix it visits, so n_cov root rollouts leave every on-path prefix with
//! n_cov samples; the n_mc top-ups only fill under-sampled prefixes and stop
//! at n_mc. Defaults (n_cov 8 > n_mc 5) satisfy the law; the stats report
//! records the regime line when a config inverts it.
//!
//! ## First-pit curation
//!
//! On a failed rollout, the *pit* is the first prefix whose Q̂ drops below
//! `eps_pit` (Research 509 §"first-pit attribution": fault = first step
//! where Q̂ ≈ 0 on an incorrect sequence). Pairs strictly deeper than a
//! path's pit are dropped — training on a proven-hopeless tail teaches the
//! scorer nothing but pessimism. The pit prefix itself is KEPT: it is the
//! informative negative.
//!
//! The canonical kernel home is `katgpt_core::prover_selection::first_pit`
//! (katgpt-rs Issue 692 T2, in flight at harness-landing time); the local
//! twin below is signature-identical — swap on landing is a one-line import.

mod rollout;
mod spans;

pub use rollout::{
    state_hash, DaemonEditGenerator, EditGenerator, FnGenerator, Outcome, PrefixNode, RolloutCtx,
    SpanTree, TerminalJudge, VocabularyJudge,
};
pub use spans::{extra_spans, fixture_spans, store_spans, PavSpan, SpanSource};

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Derive a deterministic rollout id from the campaign seed.
///
/// Purpose tags keep coverage rollouts, top-ups, and any future phase from
/// colliding on the same id for the same span.
fn derive_seed(seed: u64, span_idx: usize, purpose: u8, idx: u64) -> u64 {
    let mut h = blake3::Hasher::new();
    h.update(&seed.to_le_bytes());
    h.update(&(span_idx as u64).to_le_bytes());
    h.update(&[purpose]);
    h.update(&idx.to_le_bytes());
    let out = h.finalize();
    u64::from_le_bytes(out.as_bytes()[..8].try_into().expect("8 bytes"))
}

/// First index where `q_seq[i] < eps`; `None` when the sequence never pits.
///
/// Twin of `katgpt_core::prover_selection::first_pit` (katgpt-rs Issue 692
/// T2) — swap to the substrate import when it lands; identical signature.
#[must_use]
pub fn first_pit(q_seq: &[f32], eps: f32) -> Option<usize> {
    q_seq.iter().position(|&q| q < eps)
}

/// `BoK` transform (plan 356 A2's closed form, `K=4`): the probability that
/// at least one of K independent continuations from a prefix with
/// per-rollout success Q succeeds. Emitted alongside Q̂ so the training
/// side can pick its label column without re-deriving it.
///
/// Note: `BoK` with `K=4` — “Bo4” in the plan's shorthand.
#[must_use]
pub fn bok4(q: f32) -> f32 {
    1.0 - (1.0 - q).powi(4)
}

/// Campaign knobs. Defaults are the plan's low-budget regime.
#[derive(Debug, Clone)]
pub struct PavDataConfig {
    /// Coverage rollouts per span (each donates a sample to every prefix
    /// it visits).
    pub n_cov: usize,
    /// Minimum MC samples per emitted prefix (top-ups fill up to this).
    pub n_mc: usize,
    /// Step budget per rollout (edits from the rollout's start state).
    pub max_steps: u32,
    /// Cap on emitted (and top-up-sampled) prefixes per span.
    pub max_prefixes_per_span: usize,
    /// First-pit threshold: Q̂ below this on a failed path is the pit.
    pub eps_pit: f32,
    /// Deterministic seed base for rollout-id derivation.
    pub seed: u64,
    /// Run the real-clippy grounding pass on the curated terminal subset
    /// (slow: one cargo invocation per kept clean pair — default off, the
    /// campaign CLI opt-in).
    pub ground_truth: bool,
}

impl Default for PavDataConfig {
    fn default() -> Self {
        Self {
            n_cov: 8,
            n_mc: 5,
            max_steps: 6,
            max_prefixes_per_span: 64,
            eps_pit: 0.10,
            seed: 0,
            ground_truth: false,
        }
    }
}

impl PavDataConfig {
    /// The Research 509 regime line for this config's budget split.
    #[must_use]
    pub fn regime(&self) -> &'static str {
        if self.n_cov > self.n_mc {
            "low-budget (n_cov > n_mc — coverage-first)"
        } else {
            "high-budget (n_mc >= n_cov — Research 509 says coverage-first at low budget)"
        }
    }
}

/// One dataset row: a code state + its MC-Q label (+ Bo4 column).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PavPair {
    pub span_id: String,
    pub source: String,
    pub provenance: String,
    /// Edits applied before this state (0 = the original span).
    pub depth: u32,
    /// The code state the scorer sees at consult time.
    pub state: String,
    /// MC-Q estimate (successes / samples).
    pub q_hat: f32,
    pub n_success: u32,
    /// Samples backing the estimate (>= `n_mc` for emitted pairs).
    pub n_mc: u32,
    /// `1 - (1 - q_hat)^4` (A2's closed form).
    pub q_bo4: f32,
    /// First-pit curation verdict (false = dropped as a hopeless tail).
    pub kept: bool,
    /// Earliest pit depth among failed paths through this state.
    pub pit_depth: Option<u32>,
    /// The free judge's verdict on this state (vocabulary-clean?).
    pub proxy_clean: bool,
    /// Real-clippy grounding, when the grounding pass ran (`None` otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_clean: Option<bool>,
}

/// Campaign summary (manifest sidecar + console report).
#[derive(Debug, Clone, Default, Serialize)]
pub struct PavStats {
    pub spans_total: usize,
    /// Spans whose root already passed the judge (trivial q=1 root pair,
    /// zero generator calls).
    pub spans_clean_root: usize,
    pub rollouts_total: usize,
    pub outcomes: BTreeMap<String, usize>,
    pub pairs: usize,
    pub pairs_kept: usize,
    pub pairs_dropped_pit: usize,
    /// Kept pairs with `q_hat` < 0.5 / >= 0.5 (the A3 class-balance buckets).
    pub bucket_low: usize,
    pub bucket_high: usize,
    pub regime: &'static str,
}

impl PavStats {
    fn note_outcome(&mut self, out: Outcome) {
        *self.outcomes.entry(out.name().to_string()).or_insert(0) += 1;
        self.rollouts_total += 1;
    }
}

/// The whole campaign output for one `curate` run.
#[derive(Debug, Clone)]
pub struct PavDataset {
    pub pairs: Vec<PavPair>,
    pub stats: PavStats,
}

/// Internal per-node scratch: on-path bookkeeping for curation.
#[derive(Clone)]
struct NodeMark {
    /// This node sits on at least one CLEAN path.
    on_clean_path: bool,
    /// This node sits strictly beyond the pit of at least one FAILED path
    /// through it (the drop evidence).
    beyond_pit: bool,
    /// Earliest pit depth among failed paths through this node
    /// (diagnostics column).
    pit_depth: Option<u32>,
    /// The free judge's verdict (evaluated once at emit time).
    proxy_clean: bool,
}

/// Rollout-id purpose tags: keep coverage rollouts, top-ups, and any
/// future phase from colliding on the same id for the same span.
const PURPOSE_COV: u8 = 1;
const PURPOSE_TOPUP: u8 = 2;

/// Run the curation campaign over `spans`.
///
/// Deterministic given `cfg.seed` and a deterministic generator; the real
/// daemon is not deterministic (nor is it expected to be — the SAMPLING
/// schedule is what the seed pins).
#[must_use]
pub fn curate(
    spans: &[PavSpan],
    generator: &mut dyn EditGenerator,
    judge: &dyn TerminalJudge,
    cfg: &PavDataConfig,
) -> PavDataset {
    let pruner = crate::pruner::RustAnalyzerPruner::new();
    let mut stats = PavStats {
        regime: cfg.regime(),
        ..PavStats::default()
    };
    stats.outcomes.insert(Outcome::Clean.name().to_string(), 0);
    stats
        .outcomes
        .insert(Outcome::Decline.name().to_string(), 0);
    stats.outcomes.insert(Outcome::Parse.name().to_string(), 0);
    stats.outcomes.insert(Outcome::Budget.name().to_string(), 0);
    let mut pairs = Vec::new();

    for (span_idx, span) in spans.iter().enumerate() {
        stats.spans_total += 1;

        // Clean root: nothing to fix — emit the trivial anchor pair and
        // spend zero generator calls on it.
        if judge.is_clean(&span.code) {
            stats.spans_clean_root += 1;
            pairs.push(PavPair {
                span_id: span.id.clone(),
                source: span.source.name().to_string(),
                provenance: span.provenance.clone(),
                depth: 0,
                state: span.code.clone(),
                q_hat: 1.0,
                n_success: 0,
                n_mc: 0,
                q_bo4: bok4(1.0),
                kept: true,
                pit_depth: None,
                proxy_clean: true,
                verified_clean: None,
            });
            continue;
        }

        let mut tree = SpanTree::new(&span.code);

        // Phase 1 — coverage rollouts from the root.
        for i in 0..cfg.n_cov {
            let rid = derive_seed(cfg.seed, span_idx, PURPOSE_COV, i as u64);
            let out = tree.rollout(
                tree.root(),
                generator,
                judge,
                &pruner,
                cfg.max_steps,
                rid,
                &span.id,
            );
            stats.note_outcome(out);
        }

        // Phase 2 — top-ups until every emitted prefix has `n_mc` samples.
        // Insertion order = discovery order; the cap bounds both top-up and
        // emission surface per span.
        let cap = cfg.max_prefixes_per_span.min(tree.nodes.len());
        let mut topup_idx = 0u64;
        for node in 0..cap {
            while tree.nodes[node].samples.len() < cfg.n_mc {
                let rid = derive_seed(cfg.seed, span_idx, PURPOSE_TOPUP, topup_idx);
                topup_idx += 1;
                let out = tree.rollout(
                    node,
                    generator,
                    judge,
                    &pruner,
                    cfg.max_steps,
                    rid,
                    &span.id,
                );
                stats.note_outcome(out);
            }
        }

        // Phase 3 — curation marks + pair emission.
        let marks = curation_marks(&tree, cfg.eps_pit, judge);
        emit_span_pairs(span, &tree, &marks, cap, &mut pairs, &mut stats);
    }

    PavDataset { pairs, stats }
}

/// Curation bookkeeping: which nodes sit on clean paths, and which lie
/// strictly beyond a failed path's pit.
fn curation_marks(tree: &SpanTree, eps_pit: f32, judge: &dyn TerminalJudge) -> Vec<NodeMark> {
    let mut marks = vec![
        NodeMark {
            on_clean_path: false,
            beyond_pit: false,
            pit_depth: None,
            proxy_clean: false
        };
        tree.nodes.len()
    ];
    for (path, &out) in tree.paths.iter().zip(tree.outcomes.iter()) {
        if out == Outcome::Clean {
            for &n in path {
                marks[n].on_clean_path = true;
            }
            continue;
        }
        // Failed path: q_seq along the path (root first); a pit at index p
        // drops every strictly-later position on THIS path. Paths with no
        // pit keep every visited node (nothing proven hopeless on this
        // path).
        let q_seq: Vec<f32> = path.iter().map(|&n| tree.nodes[n].q_hat()).collect();
        let Some(p) = first_pit(&q_seq, eps_pit) else {
            continue;
        };
        let pit_node_depth = tree.nodes[path[p]].depth;
        for (i, &n) in path.iter().enumerate() {
            if i > p {
                marks[n].beyond_pit = true;
            }
            let cur = marks[n].pit_depth;
            marks[n].pit_depth = Some(cur.map_or(pit_node_depth, |e: u32| e.min(pit_node_depth)));
        }
    }
    for (n, mark) in marks.iter_mut().enumerate() {
        mark.proxy_clean = judge.is_clean(&tree.nodes[n].state);
    }
    marks
}

/// Emit the (prefix, MC-Q) pairs for one span's tree (bounded by `cap`),
/// updating bucket + kept/dropped accounting.
fn emit_span_pairs(
    span: &PavSpan,
    tree: &SpanTree,
    marks: &[NodeMark],
    cap: usize,
    pairs: &mut Vec<PavPair>,
    stats: &mut PavStats,
) {
    for (n, mark) in marks.iter().enumerate().take(cap) {
        let node = &tree.nodes[n];
        let q = node.q_hat();
        let kept = !mark.beyond_pit || mark.on_clean_path;
        let pit_depth = if mark.on_clean_path {
            None
        } else {
            mark.pit_depth
        };
        if kept {
            if q < 0.5 {
                stats.bucket_low += 1;
            } else {
                stats.bucket_high += 1;
            }
            stats.pairs_kept += 1;
        } else {
            stats.pairs_dropped_pit += 1;
        }
        stats.pairs += 1;
        pairs.push(PavPair {
            span_id: span.id.clone(),
            source: span.source.name().to_string(),
            provenance: span.provenance.clone(),
            depth: node.depth,
            state: node.state.clone(),
            q_hat: q,
            n_success: u32::try_from(node.samples.iter().filter(|b| **b).count())
                .unwrap_or(u32::MAX),
            n_mc: u32::try_from(node.samples.len()).unwrap_or(u32::MAX),
            q_bo4: bok4(q),
            kept,
            pit_depth,
            proxy_clean: mark.proxy_clean,
            verified_clean: None,
        });
    }
}

/// Grounding pass: real `cargo clippy` on the curated terminal subset.
///
/// Only kept + proxy-clean pairs are checked (the "cargo-check only the
/// curated final subset" half of A1): each costs one cargo invocation, so
/// it is the campaign CLI's opt-in flag, never the default. Returns the
/// number of grounded pairs; pairs whose oracle verdict is unavailable stay
/// `verified_clean: None` (never silently `false`).
///
/// # Errors
/// I/O creating the work root.
pub fn ground_truth(pairs: &mut [PavPair]) -> Result<usize, String> {
    let oracle = crate::draft::clippy_oracle::ClippyOracle::new()
        .with_work_root(std::env::temp_dir().join("riir_clippy_pav_ground"));
    if !oracle.is_available() {
        return Ok(0);
    }
    let vocab = crate::score_bench::vocabulary();
    let mut grounded = 0usize;
    for pair in pairs.iter_mut() {
        if !pair.kept || !pair.proxy_clean {
            continue;
        }
        let verdict = oracle.lints_for(&crate::score_bench::wrap(&pair.state));
        if !verdict.ran || !verdict.compiles {
            continue;
        }
        pair.verified_clean = Some(verdict.within_vocabulary(&vocab).is_empty());
        grounded += 1;
    }
    Ok(grounded)
}

/// Manifest sidecar: config + stats snapshot (written next to the JSONL).
#[derive(Debug, Serialize)]
pub struct PavManifest {
    pub generator: String,
    pub judge: String,
    pub n_cov: usize,
    pub n_mc: usize,
    pub max_steps: u32,
    pub eps_pit: f32,
    pub seed: u64,
    pub stats: PavStats,
}

/// Write the dataset as JSONL + a `.manifest.json` sidecar.
///
/// # Errors
/// I/O (create/write failures carry the path).
pub fn write_jsonl(
    path: &Path,
    dataset: &PavDataset,
    manifest: &PavManifest,
) -> Result<(), String> {
    {
        let mut f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for pair in &dataset.pairs {
            let line = serde_json::to_string(pair)
                .map_err(|e| format!("{}: encode {e}", path.display()))?;
            writeln!(f, "{line}").map_err(|e| format!("{}: {e}", path.display()))?;
        }
        f.flush().map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mpath = path.with_extension("manifest.json");
    let m = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("{}: encode {e}", mpath.display()))?;
    std::fs::write(&mpath, m).map_err(|e| format!("{}: {e}", mpath.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Marker judge + marker generator over tiny parseable states.
    struct MarkerJudge;
    impl TerminalJudge for MarkerJudge {
        fn is_clean(&self, state: &str) -> bool {
            state.contains("CLEAN")
        }
        fn name(&self) -> &'static str {
            "marker"
        }
    }

    #[test]
    fn first_pit_finds_first_crossing_only() {
        let seq = [0.9, 0.8, 0.05, 0.9, 0.01];
        assert_eq!(first_pit(&seq, 0.1), Some(2), "the FIRST crossing wins");
        assert_eq!(first_pit(&[0.9, 0.8], 0.1), None);
        assert_eq!(first_pit(&[], 0.1), None);
        // eps boundary: strictly below fires, equal does not.
        assert_eq!(first_pit(&[0.1], 0.1), None);
        assert_eq!(first_pit(&[0.0999, 0.9], 0.1), Some(0));
    }

    #[test]
    fn bok4_matches_the_closed_form() {
        assert!((bok4(0.0)).abs() < 1e-9);
        assert!((bok4(1.0) - 1.0).abs() < 1e-9);
        assert!((bok4(0.5) - 0.9375).abs() < 1e-9, "1 - 0.0625");
    }

    fn span(code: &str, id: &str) -> PavSpan {
        PavSpan {
            id: id.to_string(),
            provenance: "test".to_string(),
            code: code.to_string(),
            source: SpanSource::Extra,
        }
    }

    /// Deterministic generator: rollout r>0 fixes in one step, rollout r=0
    /// never does (parses forever, marker never lands).
    fn mixed_gen() -> FnGenerator<impl FnMut(&RolloutCtx<'_>) -> Option<String>> {
        FnGenerator::new("mixed", |ctx| {
            if ctx.rollout_id % 4 == 0 {
                Some(format!("fn x() {{ let _ = {}u32; }}\n", ctx.call_index + 1))
            } else {
                Some("// CLEAN\nfn x() {}\n".to_string())
            }
        })
    }

    #[test]
    fn curate_end_to_end_markerclean_statistics() {
        let spans = vec![span("fn x() {}\n", "s1")];
        let mut generator = mixed_gen();
        let cfg = PavDataConfig {
            n_cov: 8,
            n_mc: 3,
            max_steps: 4,
            ..PavDataConfig::default()
        };
        let ds = curate(&spans, &mut generator, &MarkerJudge, &cfg);

        // Root pair + the clean-step pair exist; every pair carries its
        // samples and the Bo4 column.
        assert!(ds.pairs.iter().any(|p| p.depth == 0));
        assert!(ds.pairs.iter().any(|p| p.state.contains("CLEAN")));
        assert!(ds
            .pairs
            .iter()
            .all(|p| (p.q_bo4 - bok4(p.q_hat)).abs() < 1e-6));
        // Root got a sample from every coverage rollout (all paths start
        // there) — no top-up needed for it.
        let root = ds.pairs.iter().find(|p| p.depth == 0).unwrap();
        assert_eq!(root.n_mc, 8, "n_cov samples arrive at the root for free");
        // Rollout accounting matches the config's coverage count + any
        // top-ups (>= n_cov).
        assert!(ds.stats.rollouts_total >= 8);
        assert!(ds.stats.pairs >= 2);
    }

    #[test]
    fn clean_root_shortcircuits_with_zero_generator_calls() {
        let spans = vec![span("// CLEAN\nfn x() {}\n", "clean1")];
        let mut generator = FnGenerator::new("never", |_c| {
            panic!("clean root must not consult the generator")
        });
        let cfg = PavDataConfig::default();
        let ds = curate(&spans, &mut generator, &MarkerJudge, &cfg);
        assert_eq!(ds.stats.spans_clean_root, 1);
        assert_eq!(ds.stats.rollouts_total, 0);
        assert_eq!(ds.pairs.len(), 1);
        let p = &ds.pairs[0];
        // bit-identity intent (clippy::float_cmp): the rollout quality is
        // exactly 1.0 by construction — compare bits, not tolerances.
        assert_eq!(p.q_hat.to_bits(), 1.0f32.to_bits());
        assert!(p.kept);
        assert!(p.proxy_clean);
    }

    #[test]
    fn first_pit_drops_the_hopeless_tail_and_keeps_the_pit() {
        // Rollout script: every rollout walks two parseable steps and NEVER
        // cleans — all prefixes fail, q_hat = 0 everywhere, eps_pit = 0.5
        // ⇒ pit at the ROOT (q=0 < eps). Everything deeper than the root is
        // dropped; the root itself is kept (the informative negative).
        let spans = vec![span("fn x() {}\n", "hopeless")];
        let mut generator = FnGenerator::new("never-clean", |ctx| {
            Some(format!("fn x() {{ let _ = {}u32; }}\n", ctx.call_index + 1))
        });
        let cfg = PavDataConfig {
            n_cov: 4,
            n_mc: 2,
            max_steps: 2,
            eps_pit: 0.5,
            ..PavDataConfig::default()
        };
        let ds = curate(&spans, &mut generator, &MarkerJudge, &cfg);
        assert_eq!(
            ds.stats.pairs_dropped_pit,
            ds.stats.pairs - 1,
            "only the root survives"
        );
        let root = ds.pairs.iter().find(|p| p.depth == 0).unwrap();
        assert!(root.kept);
        assert_eq!(root.pit_depth, Some(0));
        assert!(ds.pairs.iter().filter(|p| p.depth > 0).all(|p| !p.kept));
    }

    #[test]
    fn curate_is_deterministic_under_seed_and_generator() {
        let spans = vec![span("fn x() {}\n", "s1"), span("fn y() {}\n", "s2")];
        let cfg = PavDataConfig {
            n_cov: 6,
            n_mc: 2,
            max_steps: 3,
            seed: 42,
            ..Default::default()
        };
        let ds1 = curate(&spans, &mut mixed_gen(), &MarkerJudge, &cfg);
        let ds2 = curate(&spans, &mut mixed_gen(), &MarkerJudge, &cfg);
        assert_eq!(ds1.pairs.len(), ds2.pairs.len());
        for (a, b) in ds1.pairs.iter().zip(ds2.pairs.iter()) {
            assert_eq!(a.state, b.state);
            // bit-identity intent (clippy::float_cmp): seed-stability means
            // bit-identical sampling — compare bits, not tolerances.
            assert_eq!(
                a.q_hat.to_bits(), b.q_hat.to_bits(),
                "sampling schedule must be seed-stable"
            );
            assert_eq!(a.kept, b.kept);
        }
    }

    #[test]
    fn jsonl_writer_round_trips() {
        let dir = std::env::temp_dir().join(format!("pav_w_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("ds.jsonl");
        let spans = vec![span("fn x() {}\n", "s1")];
        let cfg = PavDataConfig {
            n_cov: 4,
            n_mc: 2,
            max_steps: 3,
            ..Default::default()
        };
        let ds = curate(&spans, &mut mixed_gen(), &MarkerJudge, &cfg);
        let manifest = PavManifest {
            generator: "mixed".into(),
            judge: "marker".into(),
            n_cov: cfg.n_cov,
            n_mc: cfg.n_mc,
            max_steps: cfg.max_steps,
            eps_pit: cfg.eps_pit,
            seed: cfg.seed,
            stats: ds.stats.clone(),
        };
        write_jsonl(&out, &ds, &manifest).unwrap();
        let raw = std::fs::read_to_string(&out).unwrap();
        let rows: Vec<PavPair> = raw
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows.len(), ds.pairs.len());
        assert_eq!(rows[0].state, ds.pairs[0].state);
        assert!(out.with_extension("manifest.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn regime_line_tracks_the_budget_law() {
        assert!(PavDataConfig::default().regime().contains("low-budget"));
        let inverted = PavDataConfig {
            n_mc: 9,
            n_cov: 2,
            ..PavDataConfig::default()
        };
        assert!(inverted.regime().contains("high-budget"));
    }

    #[test]
    fn derive_seed_is_purpose_and_index_stable() {
        let a = derive_seed(7, 3, 1, 0);
        let b = derive_seed(7, 3, 1, 0);
        assert_eq!(a, b);
        assert_ne!(a, derive_seed(7, 3, 2, 0), "purpose separates id spaces");
        assert_ne!(a, derive_seed(7, 4, 1, 0), "span index separates");
        assert_ne!(a, derive_seed(8, 3, 1, 0), "seed separates");
    }
}
