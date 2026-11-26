//! PAV rollout engine — plan 356 A1's edit-prefix tree over the L4 seam.
//!
//! A *rollout* is a sequence of fix attempts on one span: each step hands the
//! CURRENT code state to an [`EditGenerator`] (the daemon, or a deterministic
//! mock), the answer passes the same L2 parse gate the heal pipeline's
//! invariant 3 applies (`RustAnalyzerPruner::validate(...).parses`), and the
//! parsed answer becomes the next state. A rollout ends in one of four ways:
//! [`Outcome::Clean`] (the [`TerminalJudge`] says the state is done),
//! [`Outcome::Decline`] (the generator returned nothing), [`Outcome::Parse`]
//! (the L2 gate rejected the answer), or [`Outcome::Budget`] (step budget).
//!
//! The engine interns every visited state into a prefix tree keyed by
//! BLAKE3(state), so **one rollout donates one Bernoulli sample to every
//! prefix it visits** — the shared-suffix trick that makes the paper's
//! (prefix, MC-Q) dataset affordable: `n_cov` root rollouts already give
//! every on-path prefix `n_cov` samples; the `n_mc` top-ups only fill
//! under-sampled prefixes. That is the "`n_cov` > `n_mc` at low budget" law
//! from Research 509, enforced structurally (top-ups stop at `n_mc`
//! samples).
//!
//! GPU note (plan 356 A1 CPU-side half): the real generator is
//! [`DaemonEditGenerator`] over the riir-train `bonsai_clippy_l4_eval
//! --serve` daemon (~seconds per call). Everything here is seam-shaped so the
//! campaign is one CLI run and the tests run in microseconds on a mock.

use std::collections::HashMap;

use serde::Serialize;

use crate::l4_fallback::L4Fallback as _;
use crate::pruner::RustAnalyzerPruner;

/// BLAKE3 of the state — the prefix-tree key (deterministic dedupe).
#[must_use]
pub fn state_hash(state: &str) -> [u8; 32] {
    blake3::hash(state.as_bytes()).into()
}

/// What one rollout ended as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The terminal judge accepted the final state — a SUCCESS sample for
    /// every prefix on the path.
    Clean,
    /// The generator declined (no answer).
    Decline,
    /// The L2 parse gate rejected the generator's answer.
    Parse,
    /// The step budget ran out.
    Budget,
}

impl Outcome {
    /// The Bernoulli sample this outcome contributes to visited prefixes.
    #[must_use]
    pub fn success(self) -> bool {
        matches!(self, Outcome::Clean)
    }

    /// Stable lowercase name for stats/JSONL.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Decline => "decline",
            Self::Parse => "parse",
            Self::Budget => "budget",
        }
    }
}

/// Everything a generator sees about the step it is asked to take.
///
/// `rollout_id` is derived deterministically from (config seed, span index,
/// rollout purpose, rollout index) so a seeded generator can vary its script
/// per rollout while the whole dataset build stays reproducible.
pub struct RolloutCtx<'a> {
    pub span_id: &'a str,
    /// Current code state (the daemon's prompt body, lint-agnostic per the
    /// training-corpus contract in `l4_daemon`).
    pub state: &'a str,
    /// Edits applied so far on this path (0 at the root).
    pub depth: u32,
    /// Deterministic per-rollout id.
    pub rollout_id: u64,
    /// Generator calls so far within this rollout (0-based).
    pub call_index: u32,
}

/// The edit source: one full-state fix proposal per call.
///
/// Mirrors [`crate::l4_fallback::L4Fallback::generate_fix`] semantics (the
/// answer is the next full state, gated downstream by the L2 parse check)
/// without requiring the trait object machinery — the curate driver wants a
/// concrete generator it can also drive from a mock.
pub trait EditGenerator {
    /// Propose the next code state for `ctx.state`, or `None` to decline.
    fn propose(&mut self, ctx: &RolloutCtx<'_>) -> Option<String>;
    fn name(&self) -> &'static str;
}

/// The terminal-success oracle for a code state.
///
/// [`VocabularyJudge`] (free, default during MC) approximates "clippy clean"
/// by scanning our own fixable-lint vocabulary — the same axis the score
/// bench's `heal_rate` measures. The real `cargo clippy` verdict is the
/// *ground-truth* pass ([`crate::pav_data`] grounding), reserved for the
/// curated subset because it costs a cargo invocation per state.
pub trait TerminalJudge {
    fn is_clean(&self, state: &str) -> bool;
    fn name(&self) -> &'static str;
}

/// Free judge: no proposer-firable vocabulary lint site remains.
///
/// Scans [`FIXABLE_LINTS`](crate::domains::clippy_lints::proposers::FIXABLE_LINTS)
/// via `propose_fix` — modelless, CPU-only, the bench vocabulary axis.
/// States with lints OUTSIDE the vocabulary (cast-precision etc.) can still
/// read as "clean" here; the grounding pass is what catches that, which is
/// exactly why the dataset keeps both columns.
pub struct VocabularyJudge;

impl TerminalJudge for VocabularyJudge {
    fn is_clean(&self, state: &str) -> bool {
        crate::domains::clippy_lints::proposers::FIXABLE_LINTS
            .iter()
            .all(|lint| crate::domains::clippy_lints::proposers::propose_fix(lint, state).is_none())
    }

    fn name(&self) -> &'static str {
        "vocabulary"
    }
}

/// The real generator: the riir-train fix daemon over `DaemonL4`.
pub struct DaemonEditGenerator {
    daemon: crate::l4_daemon::DaemonL4,
}

impl DaemonEditGenerator {
    /// Spawn the daemon (see [`DaemonL4::spawn`] for the failure contract —
    /// a 27B load failure must surface here, not as a mid-campaign decline).
    ///
    /// # Errors
    /// Whatever [`DaemonL4::spawn`] reports (spawn failure, pipe capture,
    /// exit-during-load).
    pub fn spawn(
        binary: &str,
        gguf: &str,
        lora: &str,
        max_new: usize,
    ) -> Result<Self, std::io::Error> {
        Self::spawn_with_args(binary, gguf, lora, max_new, &[])
    }

    /// [`Self::spawn`] plus extra daemon flags (issue 695 step 1: the
    /// campaign passes `--temperature 0.7 --serve-seed S` through here;
    /// whitespace-split by the caller's CLI flag).
    ///
    /// # Errors
    /// Whatever [`DaemonL4::spawn_with_args`] reports.
    pub fn spawn_with_args(
        binary: &str,
        gguf: &str,
        lora: &str,
        max_new: usize,
        extra_args: &[String],
    ) -> Result<Self, std::io::Error> {
        Ok(Self {
            daemon: crate::l4_daemon::DaemonL4::spawn_with_args(
                binary, gguf, lora, max_new, extra_args,
            )?,
        })
    }
}

impl EditGenerator for DaemonEditGenerator {
    fn propose(&mut self, ctx: &RolloutCtx<'_>) -> Option<String> {
        // rule_id is logging-only in the daemon prompt (the trained
        // instruction is lint-agnostic); "" keeps the prompt exactly the
        // trained shape.
        self.daemon.generate_fix("", ctx.state)
    }

    fn name(&self) -> &'static str {
        "daemon"
    }
}

/// Closure-backed generator for tests and dry-run scripts.
pub struct FnGenerator<F>
where
    F: FnMut(&RolloutCtx<'_>) -> Option<String>,
{
    f: F,
    name: &'static str,
    /// Total `propose()` calls served (campaign-cost accounting + tests).
    pub calls: u32,
}

impl<F> FnGenerator<F>
where
    F: FnMut(&RolloutCtx<'_>) -> Option<String>,
{
    pub fn new(name: &'static str, f: F) -> Self {
        Self { f, name, calls: 0 }
    }
}

impl<F> EditGenerator for FnGenerator<F>
where
    F: FnMut(&RolloutCtx<'_>) -> Option<String>,
{
    fn propose(&mut self, ctx: &RolloutCtx<'_>) -> Option<String> {
        self.calls += 1;
        (self.f)(ctx)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

/// One interned prefix: a code state at a depth, plus its MC samples.
#[derive(Debug, Clone)]
pub struct PrefixNode {
    pub state: String,
    pub depth: u32,
    /// One Bernoulli sample per rollout (coverage or top-up) that passed
    /// through this node: did continuing from here reach `Outcome::Clean`?
    pub samples: Vec<bool>,
}

impl PrefixNode {
    /// MC-Q estimate from the samples so far (0.0 when unsampled).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // sample counts ≤ ~10² per prefix, exact in f32
    pub fn q_hat(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let n = self.samples.len() as f32;
        let s = self.samples.iter().filter(|b| **b).count() as f32;
        s / n
    }
}

/// One L2 parse-gate rejection, recorded for diagnosis (issue 695's
/// parse-robustness rung: the chunk-1 campaign's 246/248 `parse` outcomes
/// carried NO reason — the gate's syn diagnostics were discarded at the
/// call site, so the failure cause was unanswerable from the run record).
/// Bounded per tree: the first [`SpanTree::PARSE_FAILURE_CAP`] rejections
/// are kept verbatim, later ones count only.
#[derive(Debug, Clone, Serialize)]
pub struct ParseFailure {
    pub span_id: String,
    pub depth: u32,
    pub rollout_id: u64,
    /// The rejected edit's byte length (truncation vs drift reads here).
    pub edit_bytes: usize,
    /// First line of the rejected edit, single-line, ≤ 160 bytes.
    pub edit_head: String,
    /// The L2 gate's own diagnostic (the syn error text).
    pub diagnostic: String,
}

/// The per-span prefix tree: interned states + the rollouts that walked them.
#[derive(Debug, Default)]
pub struct SpanTree {
    /// Node 0 is always the root (the original span).
    pub nodes: Vec<PrefixNode>,
    by_hash: HashMap<[u8; 32], usize>,
    /// One path (node indices, start..terminal inclusive) per rollout.
    pub paths: Vec<Vec<usize>>,
    pub outcomes: Vec<Outcome>,
    /// L2 parse-gate rejections with their causes (bounded; see the cap).
    pub parse_failures: Vec<ParseFailure>,
}

impl SpanTree {
    /// Max [`ParseFailure`] records kept per tree; the rest count in the
    /// stats' `parse_failures_total` without their payloads.
    pub const PARSE_FAILURE_CAP: usize = 64;
}

impl SpanTree {
    /// New tree for `root_state`; node 0 is the root, unsampled.
    #[must_use]
    pub fn new(root_state: &str) -> Self {
        Self {
            nodes: vec![PrefixNode {
                state: root_state.to_string(),
                depth: 0,
                samples: Vec::new(),
            }],
            by_hash: HashMap::from([(state_hash(root_state), 0)]),
            paths: Vec::new(),
            outcomes: Vec::new(),
            parse_failures: Vec::new(),
        }
    }

    #[must_use]
    pub fn root(&self) -> usize {
        0
    }

    /// Intern (or find) the node for `state` at `depth`. Dedupes by state
    /// hash — two rollouts reaching the same code share one prefix.
    pub fn intern(&mut self, state: &str, depth: u32) -> usize {
        let h = state_hash(state);
        if let Some(&i) = self.by_hash.get(&h) {
            return i;
        }
        let i = self.nodes.len();
        self.nodes.push(PrefixNode {
            state: state.to_string(),
            depth,
            samples: Vec::new(),
        });
        self.by_hash.insert(h, i);
        i
    }

    /// Donate one rollout's outcome to every prefix on its path, recording
    /// the path. `path` includes the start node; the terminal node is
    /// whatever state the rollout ended on.
    pub fn record_path(&mut self, path: Vec<usize>, outcome: Outcome) {
        let success = outcome.success();
        for &n in &path {
            self.nodes[n].samples.push(success);
        }
        self.paths.push(path);
        self.outcomes.push(outcome);
    }

    /// Run ONE rollout from `start_node` and record it.
    ///
    /// Each step: propose → L2 parse gate → intern → terminal check →
    /// budget. Returns the outcome. `params.rollout_id` reaches the
    /// generator via [`RolloutCtx`] so seeded generators can vary per
    /// rollout.
    #[allow(clippy::too_many_arguments)] // 7 after the params bundle; the walkthrough args are all load-bearing
    pub fn rollout<G: EditGenerator + ?Sized, J: TerminalJudge + ?Sized>(
        &mut self,
        start_node: usize,
        generator: &mut G,
        judge: &J,
        pruner: &RustAnalyzerPruner,
        max_steps: u32,
        rollout_id: u64,
        span_id: &str,
    ) -> Outcome {
        let mut path = vec![start_node];
        let mut state = self.nodes[start_node].state.clone();
        let start_depth = self.nodes[start_node].depth;
        let mut outcome = Outcome::Budget;
        // `call` counts SERVED proposes only (conditional increment), so
        // `enumerate` cannot express it — hence the loop-level allow.
        let mut call = 0u32;
        #[allow(clippy::explicit_counter_loop)]
        for step in 0..max_steps {
            let ctx = RolloutCtx {
                span_id,
                state: &state,
                depth: start_depth + step,
                rollout_id,
                call_index: call,
            };
            let Some(edit) = generator.propose(&ctx) else {
                outcome = Outcome::Decline;
                break;
            };
            call += 1;
            let verdict = pruner.validate(&edit, "");
            if !verdict.parses {
                if self.parse_failures.len() < Self::PARSE_FAILURE_CAP {
                    let mut head = edit.lines().next().unwrap_or("").to_string();
                    if head.len() > 160 {
                        // Byte 160 may land mid-codepoint on multi-byte edits — the
                        // recorder must never panic on the inputs it exists to capture
                        // (the 4090 diag arm crashed here: rollout.rs:380, T=0.7 answers).
                        let mut cut = 160;
                        while cut > 0 && !head.is_char_boundary(cut) {
                            cut -= 1;
                        }
                        head.truncate(cut);
                    }
                    self.parse_failures.push(ParseFailure {
                        span_id: span_id.to_string(),
                        depth: start_depth + step,
                        rollout_id,
                        edit_bytes: edit.len(),
                        edit_head: head,
                        diagnostic: verdict.diagnostics.first().cloned().unwrap_or_default(),
                    });
                }
                outcome = Outcome::Parse;
                break;
            }
            let node = self.intern(&edit, start_depth + step + 1);
            path.push(node);
            state = edit;
            if judge.is_clean(&state) {
                outcome = Outcome::Clean;
                break;
            }
        }
        self.record_path(path, outcome);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Judge that accepts exactly the state containing the CLEAN marker.
    struct MarkerJudge;
    impl TerminalJudge for MarkerJudge {
        fn is_clean(&self, state: &str) -> bool {
            state.contains("CLEAN")
        }
        fn name(&self) -> &'static str {
            "marker"
        }
    }

    fn pruner() -> RustAnalyzerPruner {
        RustAnalyzerPruner::new()
    }

    #[test]
    fn rollout_success_donates_samples_to_every_prefix() {
        let mut tree = SpanTree::new("// start\nfn a() {}\n");
        let mut generator = FnGenerator::new("t", |ctx| match ctx.call_index {
            0 => Some("// one\nfn a() {}\n".to_string()),
            _ => Some("// CLEAN\nfn a() {}\n".to_string()),
        });
        let out = tree.rollout(0, &mut generator, &MarkerJudge, &pruner(), 6, 1, "s");
        assert_eq!(out, Outcome::Clean);
        // Path = root -> step1 -> step2(clean terminal).
        assert_eq!(tree.paths[0].len(), 3);
        assert_eq!(tree.nodes.len(), 3);
        // Every prefix on the path got exactly one SUCCESS sample.
        for n in &tree.nodes {
            assert_eq!(n.samples, vec![true]);
            assert!((n.q_hat() - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn unparseable_answer_is_a_parse_outcome_and_keeps_prior_prefixes() {
        let mut tree = SpanTree::new("fn a() {}\n");
        let mut generator = FnGenerator::new("t", |_ctx| Some("fn broken( {{{".to_string()));
        let out = tree.rollout(0, &mut generator, &MarkerJudge, &pruner(), 3, 7, "s");
        assert_eq!(out, Outcome::Parse);
        // Only the root was visited; it carries one failure sample.
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].samples, vec![false]);
        assert!((tree.nodes[0].q_hat() - 0.0).abs() < 1e-9);
        // The parse-robustness record (issue 695): the rejection carries its
        // cause, bounded by the cap.
        assert_eq!(tree.parse_failures.len(), 1);
        let pf = &tree.parse_failures[0];
        assert_eq!(pf.span_id, "s");
        assert_eq!(pf.rollout_id, 7);
        assert_eq!(pf.depth, 0);
        assert_eq!(pf.edit_bytes, "fn broken( {{{".len());
        assert!(
            !pf.diagnostic.is_empty(),
            "the syn diagnostic must ride the record"
        );
    }

    #[test]
    fn multi_byte_head_truncation_never_panics_and_snaps_to_a_char_boundary() {
        // The recorder's edit_head cut lands mid-codepoint when byte 160 sits
        // inside a multi-byte char (the 4090 diag arm crashed at rollout.rs:380
        // on real T=0.7 answers). 81 U+00E9 chars = 162 bytes; byte 160 is the
        // FIRST byte of the 81st char — the exact panic shape.
        let edit = format!("fn broken(){{ {} }}", "\u{00e9}".repeat(81));
        assert_eq!(edit.len(), 177);
        assert!(edit.len() > 160);
        assert!(
            !edit.is_char_boundary(160),
            "fixture must straddle byte 160"
        );
        let mut tree = SpanTree::new("fn a() {}\n");
        let mut generator = FnGenerator::new("t", move |_ctx| Some(format!("{edit} {{{{")));
        let out = tree.rollout(0, &mut generator, &MarkerJudge, &pruner(), 3, 7, "s");
        assert_eq!(out, Outcome::Parse);
        assert_eq!(tree.parse_failures.len(), 1);
        let head = &tree.parse_failures[0].edit_head;
        assert!(head.len() <= 160, "head must respect the byte cap");
        assert!(
            head.is_char_boundary(head.len()),
            "head must end on a char boundary"
        );
        // The snapped cut keeps the maximum possible prefix: 160 bytes → 159
        // (one é byte dropped).
        assert_eq!(head.len(), 159);
        assert!(head.chars().all(|c| c == 'é'
            || !c.is_ascii()
            || c.is_ascii_alphanumeric()
            || " (){}_".contains(c)));
    }

    #[test]
    fn decline_and_budget_are_distinct_failures() {
        let mut t1 = SpanTree::new("fn a() {}\n");
        let mut g1 = FnGenerator::new("t", |_c| None);
        assert_eq!(
            t1.rollout(0, &mut g1, &MarkerJudge, &pruner(), 3, 1, "s"),
            Outcome::Decline
        );

        // Never clean, always parseable, budget exhausts.
        let mut t2 = SpanTree::new("fn a() {}\n");
        let mut last = 0u32;
        let mut g2 = FnGenerator::new("t", move |c| {
            last = last.max(c.call_index) + 1;
            Some(format!("fn a() {{ let _ = {last}; }}\n"))
        });
        assert_eq!(
            t2.rollout(0, &mut g2, &MarkerJudge, &pruner(), 3, 2, "s"),
            Outcome::Budget
        );
        // root + 3 interned steps.
        assert_eq!(t2.nodes.len(), 4);
        assert_eq!(t2.outcomes, vec![Outcome::Budget]);
        assert_eq!(t1.outcomes, vec![Outcome::Decline]);
    }

    #[test]
    fn shared_suffixes_dedupe_into_one_prefix() {
        let mut tree = SpanTree::new("fn a() {}\n");
        let mut generator = FnGenerator::new("t", |_c| Some("// CLEAN\nfn a() {}\n".to_string()));
        tree.rollout(0, &mut generator, &MarkerJudge, &pruner(), 2, 1, "s");
        tree.rollout(0, &mut generator, &MarkerJudge, &pruner(), 2, 2, "s");
        // Both rollouts converged on the identical state: 2 nodes total,
        // both with 2 samples.
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.nodes[1].samples.len(), 2);
        assert!((tree.nodes[1].q_hat() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn continuation_from_mid_tree_node_topups_only_that_subtree() {
        let mut tree = SpanTree::new("fn a() {}\n");
        // Step 1 shared, step 2 clean.
        let mut generator = FnGenerator::new("t", |c| match c.call_index {
            0 => Some("// mid\nfn a() {}\n".to_string()),
            _ => Some("// CLEAN\nfn a() {}\n".to_string()),
        });
        tree.rollout(0, &mut generator, &MarkerJudge, &pruner(), 4, 1, "s");
        assert_eq!(tree.nodes.len(), 3);
        // A top-up continuation from node 1 walks only its own suffix; its
        // record_path donates along the walked path (starting at node 1), so
        // the ROOT keeps its single original sample.
        let mut gen2 = FnGenerator::new("t2", |_c| Some("// CLEAN\nfn a() {}\n".to_string()));
        tree.rollout(1, &mut gen2, &MarkerJudge, &pruner(), 4, 9, "s");
        assert_eq!(tree.nodes[0].samples.len(), 1);
        assert_eq!(tree.nodes[1].samples.len(), 2);
        assert_eq!(tree.nodes[2].samples.len(), 2);
    }

    #[test]
    fn vocabulary_judge_matches_a_real_fixable_site() {
        let dirty = "fn f() -> i32 {\n    return 7;\n}\n";
        let clean = "fn f() -> i32 {\n    7\n}\n";
        assert!(
            !VocabularyJudge.is_clean(dirty),
            "needless_return site must read dirty"
        );
        assert!(
            VocabularyJudge.is_clean(clean),
            "healed span must read clean"
        );
    }
}
