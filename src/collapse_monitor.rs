//! Issue 042 — self-collapse monitor for the fix-trajectory retrieval loop
//! (riir-ai [R346](../riir-ai/.research/346_rag_collapse_self_authored_retrieval.md),
//! arXiv:2608.22118 "RAG Collapse").
//!
//! The paper's transferable finding: a retriever whose corpus contains
//! content **the system itself authored** over-fetches that content — a
//! memory trajectory IS the answer to a previous similar query, hence
//! maximally similar to future queries; the similarity score that drives
//! retrieval is the same property that drives collapse (79.6% of 1,528
//! simulations; self-authored content cited at 38.9% vs 7.4%; the gap
//! survives quality controls at +26pp).
//!
//! Our instance of the loop shape is [`HealPipeline::heal`] Step 1
//! (`memory.retrieve_into`) + Step 3 (retrieved trajectories become
//! `HealSource::Memory` candidates) + `record_applied` writing every applied
//! fix back. This module instruments the REAL loop — the same
//! [`LatentQuery`] construction, the same [`LatentFixMemory::retrieve_into`]
//! call, the same dedup rule, and [`HealPipeline::heal`] itself for the
//! selection verdict — never a parallel reimplementation.
//!
//! ## What is measured (the collapse signature)
//!
//! For a FIXED fixture span set under varying store conditions (fresh /
//! mid-age prefix / production store):
//!
//! - **Pool memory-fraction** — the Memory share of the Step-3 candidate
//!   pool (drafter candidate + up to `TOP_K` memory trajectories, after
//!   heal's exact-code dedup). The paper's "self-authored share of the
//!   retrieval pool".
//! - **Memory win-rate** — the share of served fixes whose selected
//!   candidate is `HealSource::Memory`. The paper's "citation rate" analog.
//! - **Top-k rule-entropy** — normalized Shannon entropy over the `lint_name`
//!   distribution of the retrieved top-k (per-span mean + pooled over all
//!   retrieved trajectories). The paper's pool-diversity axis.
//! - **Same-rule retrieval rate** — the share of retrieved trajectories
//!   matching the query's rule (the on-topic fraction; low same-rule +
//!   low entropy = crowding by unrelated self-content).
//!
//! ## Two structural findings (measured at Issue-042 land time, Bench 048)
//!
//! 1. **The dry `--suggest` path cannot collapse** — `LatentMatcher`
//!    retrieves from the corpus `RuleIndex` only; the trajectory store is
//!    never consulted. Collapse risk on suggest is structurally zero.
//! 2. **The CLI `--fix` path records but never reads back** —
//!    `EvolveRecorder`/`record_traj` write trajectories; no CLI retrieval
//!    consults them. The memory-reading loop is the API surface
//!    ([`HealPipeline::heal`]), so the collapse risk is LIVE at the API
//!    level and latent at the CLI level. This monitor is the tripwire for
//!    both.

use crate::domain::HealEntry;
use crate::drafter::TernaryDrafter;
use crate::memory::{FixTrajectory, LatentFixMemory, LatentQuery};
use crate::pruner::RustAnalyzerPruner;
use crate::self_evolve::{HealPipeline, HealSource};

/// The retrieval depth of the real loop — `heal()` Step 1 hardcodes `k = 5`
/// (`memory.retrieve_into(&query, 5, …)`). The monitor matches it so the
/// measurement IS the loop's behavior, not an approximation of it.
pub const TOP_K: usize = 5;

/// Normalized Shannon entropy over a count vector: `H(p) / log2(n)` where
/// `n` is the number of ITEMS (not distinct categories) — a single-item
/// pool is minimally diverse (0.0), a uniformly-spread pool over `n` items
/// is 1.0. Returns 0.0 for empty/single pools.
///
/// Normalization by item count (not distinct count) makes spans with
/// different `k' ≤ k` retrieved counts comparable: a 5-item pool of 5
/// distinct rules (H=log2 5, norm=log2 5) scores 1.0, and a 5-item pool of
/// one rule scores 0.0, on the same scale.
#[must_use]
#[allow(clippy::cast_precision_loss)] // counts are small (<2²⁴); f32 is exact
pub fn normalized_entropy(counts: &[usize]) -> f32 {
    let n: usize = counts.iter().sum();
    if n < 2 {
        return 0.0;
    }
    let nf = n as f32;
    let mut h = 0.0f32;
    for &c in counts {
        if c == 0 {
            continue;
        }
        let p = c as f32 / nf;
        h -= p * p.log2();
    }
    h / nf.log2()
}

/// The collapse-signature metrics for one (store condition × fixture set)
/// run. All ratios are in `[0, 1]`.
#[derive(Debug, Clone, PartialEq)]
pub struct CollapseMetrics {
    /// Fixture spans measured.
    pub spans: usize,
    /// Trajectories in the measured store.
    pub store_size: usize,
    /// Spans where the L1 drafter produced a candidate (Step 2).
    pub drafter_spans: usize,
    /// Spans where Step 1 retrieved ≥ 1 trajectory.
    pub spans_with_retrieval: usize,
    /// Memory candidates that entered the Step-3 pool (post-dedup), summed.
    pub memory_candidates: usize,
    /// Total Step-3 pool candidates (drafter + memory, post-dedup), summed.
    pub pool_candidates: usize,
    /// Memory share of the pooled candidate count.
    pub pool_memory_fraction: f32,
    /// Mean per-span memory fraction (spans with an empty pool excluded).
    pub per_span_memory_fraction: f32,
    /// Served fixes whose selected candidate came from Memory.
    pub memory_wins: usize,
    /// Served fixes whose selected candidate came from the Drafter.
    pub drafter_wins: usize,
    /// Spans where `heal()` returned `None` (declined).
    pub declined: usize,
    /// Memory share of SERVED fixes (the paper's citation-rate analog).
    pub memory_win_rate: f32,
    /// Mean per-span normalized entropy of the retrieved top-k's rule
    /// distribution (spans with ≥ 1 retrieval).
    pub per_span_topk_entropy: f32,
    /// Normalized entropy of the rule distribution pooled over ALL
    /// retrieved trajectories across spans.
    pub pooled_rule_entropy: f32,
    /// Share of retrieved trajectories whose rule matches the query's rule.
    pub same_rule_retrieval_rate: f32,
    /// Distinct rules among all retrieved trajectories.
    pub distinct_rules_retrieved: usize,
}

/// Measure the collapse signature of the real retrieval loop over a fixed
/// fixture span set, against one memory (store condition).
///
/// Instruments, in order — all real API, same call shapes `heal()` uses:
///
/// 1. `LatentQuery::new(rule, buggy)` — Step 1's exact query construction.
/// 2. `memory.retrieve_into(&q, TOP_K, …)` — Step 1's exact retrieval call.
/// 3. `drafter.draft_fix(rule, buggy, corpus)` — Step 2's exact draft call
///    (an independent fallback drafter instance, same config as the
///    pipeline's).
/// 4. heal's Step-3 dedup rule (skip memory candidates whose code equals
///    the drafter's) — mirrored one-line, exact-same comparison.
/// 5. `HealPipeline::heal(rule, buggy, corpus)` with the measured memory
///    installed — the real selection verdict (`HealResult.source`).
///
/// Deterministic: no RNG, no wall-clock, insertion-order iteration. Pinned
/// by the committed double-run gate (`collapse_monitor_is_deterministic`).
#[must_use]
#[allow(clippy::too_many_lines)] // mirrors heal()'s own Steps 1-5 one-for-one; splitting it would separate the loop from the thing measured
#[allow(clippy::cast_precision_loss)] // counts ≤ 8241 rows / 48 spans — exact in f32 by orders of magnitude
pub fn measure<E: HealEntry>(
    memory: &LatentFixMemory,
    spans: &[(&str, &str)],
    corpus: &[E],
) -> CollapseMetrics {
    // Install the measured memory into a real pipeline (same content, same
    // order — retrieval iterates the Vec in insertion order).
    let owned: Vec<FixTrajectory> = memory.iter().cloned().collect();
    let mut pipeline = HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new());
    *pipeline.memory_mut() = LatentFixMemory::from_trajectories(owned);

    let mut drafter = TernaryDrafter::fallback();
    let mut scratch: Vec<usize> = Vec::new();
    let mut retrieved: Vec<crate::memory::ScoredTrajectory> = Vec::with_capacity(TOP_K);

    let mut drafter_spans = 0usize;
    let mut spans_with_retrieval = 0usize;
    let mut memory_candidates = 0usize;
    let mut pool_candidates = 0usize;
    let mut per_span_frac_sum = 0.0f32;
    let mut per_span_frac_n = 0usize;
    let mut memory_wins = 0usize;
    let mut drafter_wins = 0usize;
    let mut declined = 0usize;
    let mut per_span_h_sum = 0.0f32;
    let mut per_span_h_n = 0usize;
    let mut same_rule = 0usize;
    let mut retrieved_total = 0usize;
    let mut pooled_rule_counts: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();

    for (rule, buggy) in spans {
        // Step 1 (real call shape): retrieve top-k for this query.
        let q = LatentQuery::new((*rule).to_string(), buggy);
        memory.retrieve_into(&q, TOP_K, &mut scratch, &mut retrieved);
        if !retrieved.is_empty() {
            spans_with_retrieval += 1;
        }

        // Step 2 (real call): does the drafter produce a candidate?
        let draft_code = drafter.draft_fix(rule, buggy, corpus).map(|d| d.code);
        if draft_code.is_some() {
            drafter_spans += 1;
        }

        // Step 3 (mirrored dedup rule): pool = drafter + non-duplicate memory.
        let mut mem_k = 0usize;
        for scored in &retrieved {
            if let Some(traj) = memory.get(scored.idx) {
                if draft_code.as_ref().is_some_and(|d| *d == traj.fixed_code) {
                    continue; // heal's exact-code dedup
                }
                mem_k += 1;
                *pooled_rule_counts.entry(traj.lint_name.as_str()).or_insert(0) += 1;
                if traj.lint_name == *rule {
                    same_rule += 1;
                }
            }
        }
        retrieved_total += retrieved.len();
        memory_candidates += mem_k;
        let pool_k = mem_k + usize::from(draft_code.is_some());
        pool_candidates += pool_k;
        if pool_k > 0 {
            per_span_frac_sum += mem_k as f32 / pool_k as f32;
            per_span_frac_n += 1;
        }

        // Per-span top-k rule entropy (over ALL retrieved, pre-dedup — the
        // retrieval pool is what the retriever surfaces).
        if !retrieved.is_empty() {
            let counts: Vec<usize> = retrieved_rule_counts(memory, &retrieved).values().copied().collect();
            per_span_h_sum += normalized_entropy(&counts);
            per_span_h_n += 1;
        }

        // Step 4+5 (real selection): heal() picks; the source is the verdict.
        match pipeline.heal(rule, buggy, corpus) {
            Some(r) => match r.source {
                HealSource::Memory => memory_wins += 1,
                HealSource::Drafter | HealSource::Stage2Adaptive => drafter_wins += 1,
                #[cfg(feature = "l4_lora")]
                HealSource::L4Lora => drafter_wins += 1, // not installed; unreachable
            },
            None => declined += 1,
        }
    }

    let served = memory_wins + drafter_wins;
    let pooled_counts: Vec<usize> = pooled_rule_counts.values().copied().collect();
    CollapseMetrics {
        spans: spans.len(),
        store_size: memory.len(),
        drafter_spans,
        spans_with_retrieval,
        memory_candidates,
        pool_candidates,
        pool_memory_fraction: ratio(memory_candidates, pool_candidates),
        per_span_memory_fraction: if per_span_frac_n == 0 {
            0.0
        } else {
            per_span_frac_sum / per_span_frac_n as f32
        },
        memory_wins,
        drafter_wins,
        declined,
        memory_win_rate: ratio(memory_wins, served),
        per_span_topk_entropy: if per_span_h_n == 0 {
            0.0
        } else {
            per_span_h_sum / per_span_h_n as f32
        },
        pooled_rule_entropy: normalized_entropy(&pooled_counts),
        same_rule_retrieval_rate: ratio(same_rule, retrieved_total),
        distinct_rules_retrieved: pooled_rule_counts.len(),
    }
}

/// Ratio helper: `a / b`, 0.0 for an empty denominator (a monitor that
/// reports `NaN` on fresh stores would break the audit script's greps).
#[allow(clippy::cast_precision_loss)] // small counts; exact in f32
fn ratio(a: usize, b: usize) -> f32 {
    if b == 0 { 0.0 } else { a as f32 / b as f32 }
}

/// Rule-id histogram of one span's retrieved top-k (shared by the per-span
/// entropy + the pooled distribution).
fn retrieved_rule_counts<'a>(
    memory: &'a LatentFixMemory,
    retrieved: &[crate::memory::ScoredTrajectory],
) -> std::collections::BTreeMap<&'a str, usize> {
    let mut per_rule = std::collections::BTreeMap::new();
    for scored in retrieved {
        if let Some(traj) = memory.get(scored.idx) {
            *per_rule.entry(traj.lint_name.as_str()).or_insert(0) += 1;
        }
    }
    per_rule
}

/// Shared tail of the `--collapse-report` bin mode + the audit script's
/// §1c: measure, double-run for the determinism check, print one line with
/// the verdict. Returns the first run's metrics.
pub fn collapse_report_common<E: HealEntry>(
    label: &str,
    memory: &LatentFixMemory,
    spans: &[(&str, &str)],
    corpus: &[E],
) -> CollapseMetrics {
    let a = measure(memory, spans, corpus);
    let b = measure(memory, spans, corpus);
    let det = if a.bit_signature() == b.bit_signature() {
        "deterministic"
    } else {
        "NON-DETERMINISTIC"
    };
    println!("{} [{det}]", a.report(label));
    a
}

impl CollapseMetrics {
    /// One-line machine-comparable signature (full f32 precision) — the
    /// determinism double-run gate compares these strings bit-for-bit.
    #[must_use]
    pub fn bit_signature(&self) -> String {
        format!(
            "spans={} rows={} draft={} retr={} memc={} pool={} pmf={:.6e} psmf={:.6e} \
             mw={} dw={} dec={} mwr={:.6e} h_span={:.6e} h_pool={:.6e} same={:.6e} dist={}",
            self.spans,
            self.store_size,
            self.drafter_spans,
            self.spans_with_retrieval,
            self.memory_candidates,
            self.pool_candidates,
            self.pool_memory_fraction,
            self.per_span_memory_fraction,
            self.memory_wins,
            self.drafter_wins,
            self.declined,
            self.memory_win_rate,
            self.per_span_topk_entropy,
            self.pooled_rule_entropy,
            self.same_rule_retrieval_rate,
            self.distinct_rules_retrieved
        )
    }

    /// Human-readable block (the audit script + bench table line).
    #[must_use]
    pub fn report(&self, label: &str) -> String {
        format!(
            "{label}: rows={} spans={} | pool mem={}/{} ({:.1}%) per-span {:.1}% | \
             selection mem={} drafter={} declined={} (mem win {:.1}%) | \
             top-k H={:.3} pooled H={:.3} same-rule {:.1}% distinct={}",
            self.store_size,
            self.spans,
            self.memory_candidates,
            self.pool_candidates,
            self.pool_memory_fraction * 100.0,
            self.per_span_memory_fraction * 100.0,
            self.memory_wins,
            self.drafter_wins,
            self.declined,
            self.memory_win_rate * 100.0,
            self.per_span_topk_entropy,
            self.pooled_rule_entropy,
            self.same_rule_retrieval_rate * 100.0,
            self.distinct_rules_retrieved
        )
    }
}

// ── Store bridges (the durable backends → LatentFixMemory) ─────────────────

/// Load the memory from a store dump ([`crate::traj_store::TrajStore::dump`])
/// — the exact inverse of the 038-T3 migration's field mapping
/// (`lint_key`→`lint_name`, `shape`→`buggy_code`, `fix`→`fixed_code`),
/// preserving each row's stored `direction` and counters bit-for-bit.
///
/// Read-only by construction: the caller dumps the rows (or reads the
/// envelope) and this builds a fresh in-memory store; nothing here writes.
#[cfg(feature = "latent_retrieval")]
#[must_use]
pub fn memory_from_store_rows(rows: &[crate::traj_store::TrajRowDoc]) -> LatentFixMemory {
    use crate::memory::EvidenceTier;

    let mut trajs = Vec::with_capacity(rows.len());
    for row in rows {
        // `FixTrajectory::new` computes the lint hash + a fresh direction;
        // the stored direction wins (bit-faithful to the store).
        let mut t = FixTrajectory::new(
            row.lint_key.clone(),
            row.shape.clone(),
            row.fix.clone(),
        );
        for (i, x) in row.direction.iter().take(t.direction.len()).enumerate() {
            t.direction[i] = *x;
        }
        t.success_count = row.success_count;
        t.fail_count = row.fail_count;
        t.consecutive_fails = row.consecutive_fails;
        t.tier = EvidenceTier::from_u8(row.tier);
        trajs.push(t);
    }
    LatentFixMemory::from_trajectories(trajs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_uniform_is_one() {
        // Max-diversity pools (n items, n distinct) normalize to 1.0.
        assert!((normalized_entropy(&[1, 1]) - 1.0).abs() < 1e-6);
        assert!((normalized_entropy(&[1, 1, 1, 1, 1]) - 1.0).abs() < 1e-6);
        // A 2-of-3 split over 5 items: H = 0.9710 bits, normalized by
        // log2(5) = 2.3219 → 0.4182 (2 categories can never reach 1.0 at n=5).
        assert!((normalized_entropy(&[2, 3]) - 0.4183).abs() < 1e-3);
    }

    #[test]
    fn entropy_degenerate_is_zero() {
        // Exact-zero by early return — bit comparison, not tolerance.
        assert_eq!(normalized_entropy(&[]).to_bits(), 0.0f32.to_bits());
        assert_eq!(normalized_entropy(&[7]).to_bits(), 0.0f32.to_bits());
        assert_eq!(normalized_entropy(&[0, 9, 0]).to_bits(), 0.0f32.to_bits());
    }

    #[cfg(all(test, feature = "latent_retrieval"))]
    mod loop_tests {
        use super::super::*;
        use crate::domains::clippy_lints::ClippyFixEntry;
        use crate::draft::eval_fixtures_clippy::CLIPPY_SPANS;
        use crate::memory::LatentFixMemory;

        fn corpus() -> Vec<ClippyFixEntry> {
            crate::build_seed_corpus().entries
        }

        /// The synthetic maximal-self-similarity store: for every fixture
        /// span, the store holds a trajectory for the SAME rule whose
        /// `buggy_code` is the span itself (the healer's own past fix for
        /// exactly this shape — the paper's single-self-reference regime).
        fn synthetic_self_store() -> LatentFixMemory {
            let corpus = corpus();
            let mut mem = LatentFixMemory::default();
            for (rule, buggy) in CLIPPY_SPANS {
                let fixed = corpus
                    .iter()
                    .find(|e| e.lint_name == *rule)
                    .map_or_else(|| format!("/* fixed:{rule} */"), |e| e.fixed_code.clone());
                mem.insert(FixTrajectory::new((*rule).to_string(), buggy.to_string(), fixed));
            }
            mem
        }

        #[test]
        fn fresh_store_has_zero_memory_share() {
            let m = measure(&LatentFixMemory::default(), CLIPPY_SPANS, &corpus());
            assert_eq!(m.spans, CLIPPY_SPANS.len());
            assert_eq!(m.store_size, 0);
            assert_eq!(m.memory_candidates, 0);
            assert_eq!(m.pool_memory_fraction.to_bits(), 0.0f32.to_bits());
            assert_eq!(m.memory_wins, 0);
            assert_eq!(m.drafter_wins + m.declined, m.spans);
        }

        #[test]
        fn synthetic_self_store_elevates_memory_share() {
            // The instrumentation must SEE the paper's signature in the
            // maximal regime: with the store holding a same-rule trajectory
            // for every fixture span, BM25 (0.6) guarantees retrieval and
            // the pool must be memory-dominated vs the fresh baseline.
            let fresh = measure(&LatentFixMemory::default(), CLIPPY_SPANS, &corpus());
            let selfy = measure(&synthetic_self_store(), CLIPPY_SPANS, &corpus());
            assert!(
                selfy.pool_memory_fraction > fresh.pool_memory_fraction,
                "synthetic self-store must elevate pool memory-fraction: {} > {}",
                selfy.pool_memory_fraction,
                fresh.pool_memory_fraction
            );
            // The store holds ~1 same-rule trajectory per span, so BM25 (0.6
            // exact-match boost) must retrieve it in every span's top-k —
            // but at most ~1/5 of the top-k can be same-rule (the other 4
            // slots fill by latent similarity with other rules' entries).
            assert_eq!(selfy.spans_with_retrieval, selfy.spans);
            assert!(selfy.same_rule_retrieval_rate > 0.0);
        }

        #[test]
        fn collapse_monitor_is_deterministic() {
            let store = synthetic_self_store();
            let a = measure(&store, CLIPPY_SPANS, &corpus());
            let b = measure(&store, CLIPPY_SPANS, &corpus());
            assert_eq!(a.bit_signature(), b.bit_signature());
        }

        /// Dose-response sanity: a prefix of the synthetic store must not
        /// exceed the full store's memory-candidate count (monotone in
        /// store size for same-rule content).
        #[test]
        fn prefix_store_is_between_fresh_and_full() {
            let clippy_corpus = corpus();
            let full = synthetic_self_store();
            let owned: Vec<FixTrajectory> = full.iter().cloned().collect();
            let mid = LatentFixMemory::from_trajectories(owned[..owned.len() / 2].to_vec());
            let fresh = measure(&LatentFixMemory::default(), CLIPPY_SPANS, &clippy_corpus);
            let mid_m = measure(&mid, CLIPPY_SPANS, &clippy_corpus);
            let full_m = measure(&full, CLIPPY_SPANS, &clippy_corpus);
            assert!(mid_m.memory_candidates >= fresh.memory_candidates);
            assert!(mid_m.memory_candidates <= full_m.memory_candidates);
        }

        /// Exact corpus-span fixture set — the regime where the L1 drafter
        /// DOES fire (its own template shares identifiers with its own
        /// buggy shape by construction). This is the only regime where the
        /// paper's self-vs-external POOL COMPETITION exists (variant spans
        /// → the identity guard declines the corpus template → the pool is
        /// memory-only → memory-fraction is 100% structurally, a vacuum not
        /// a crowding-out).
        #[must_use]
        fn exact_corpus_spans(corpus: &[ClippyFixEntry]) -> Vec<(&str, &str)> {
            corpus.iter().map(|e| (e.lint_name.as_str(), e.buggy_code.as_str())).collect()
        }

        /// Production-store measurement (env-gated — the stores live on the
        /// dev box, not CI). Skips cleanly when absent; when present, runs
        /// the full dose-response over the envelope + per-repo stores and
        /// asserts determinism only (no hard floors — this is the monitor,
        /// not a gate on the stores' content).
        ///
        /// Env:
        /// - `RIIR_COLLAPSE_ENVELOPE` — path to a memory.json[.migrated] copy
        /// - `RIIR_COLLAPSE_STORE_ROOT` — dir of `<repo>/.heal/…` COPIES
        ///   (never the live stores — copy first).
        #[test]
        fn collapse_monitor_production_measurement() {
            let clippy_corpus = corpus();
            let envelope = std::env::var("RIIR_COLLAPSE_ENVELOPE").ok();
            let store_root = std::env::var("RIIR_COLLAPSE_STORE_ROOT").ok();
            if envelope.is_none() && store_root.is_none() {
                eprintln!(
                    "skip: production measurement needs RIIR_COLLAPSE_ENVELOPE and/or \
                     RIIR_COLLAPSE_STORE_ROOT (copies of the stores, never the live paths)"
                );
                return;
            }

            // (b) fresh baseline.
            let fresh = measure(&LatentFixMemory::default(), CLIPPY_SPANS, &clippy_corpus);
            println!("{}", fresh.report("fresh(empty)"));

            // (a) production envelope + dose-response prefixes.
            if let Some(path) = envelope {
                let mut mem = LatentFixMemory::default();
                mem.load(std::path::Path::new(&path))
                    .expect("envelope must load (copy path)");
                let owned: Vec<FixTrajectory> = mem.iter().cloned().collect();
                for n in [100usize, 1_000, 4_000, owned.len()] {
                    let n = n.min(owned.len());
                    let m = measure(
                        &LatentFixMemory::from_trajectories(owned[..n].to_vec()),
                        CLIPPY_SPANS,
                        &clippy_corpus,
                    );
                    println!("{}", m.report(&format!("envelope[:{n}]")));
                }
                // Determinism double-run on the full condition.
                let a = measure(
                    &LatentFixMemory::from_trajectories(owned.clone()),
                    CLIPPY_SPANS,
                    &clippy_corpus,
                );
                let b = measure(
                    &LatentFixMemory::from_trajectories(owned),
                    CLIPPY_SPANS,
                    &clippy_corpus,
                );
                assert_eq!(a.bit_signature(), b.bit_signature());

                // Dominant-rule regime: the spans for the store's single
                // most-concentrated rule — the paper's collapse regime if
                // it exists anywhere (7,286 same-rule self-trajectories).
                let fmt_spans: Vec<(&str, &str)> = CLIPPY_SPANS
                    .iter()
                    .copied()
                    .filter(|(r, _)| *r == "uninlined_format_args")
                    .collect();
                if !fmt_spans.is_empty() {
                    let m = measure(
                        &LatentFixMemory::from_trajectories(mem.iter().cloned().collect()),
                        &fmt_spans,
                        &clippy_corpus,
                    );
                    println!("{}", m.report("envelope[fmt-args-only]"));
                }

                // Exact-corpus regime: the drafter fires — the pool carries
                // BOTH sources, so memory win-rate measures whether
                // self-authored content outranks the external template.
                let exact = exact_corpus_spans(&clippy_corpus);
                let m = measure(
                    &LatentFixMemory::from_trajectories(mem.iter().cloned().collect()),
                    &exact,
                    &clippy_corpus,
                );
                println!("{}", m.report("envelope[exact-corpus-spans]"));
                let fresh_exact = measure(&LatentFixMemory::default(), &exact, &clippy_corpus);
                println!("{}", fresh_exact.report("fresh[exact-corpus-spans]"));
            }

            // Per-repo store copies.
            if let Some(root) = store_root {
                for repo in
                    ["riir-ai", "riir-chain", "riir-clippy", "riir-dapps", "riir-game-sdk", "riir-mmorpg-examples"]
                {
                    let dir = std::path::Path::new(&root).join(repo).join(".heal");
                    if !dir.join("trajectories.kv").exists() {
                        continue;
                    }
                    match crate::traj_store::TrajStore::open(
                        &std::path::Path::new(&root).join(repo),
                    ) {
                        Ok((store, bad)) => {
                            assert_eq!(bad, 0, "{repo}: copy must decode cleanly");
                            let mem = memory_from_store_rows(&store.dump());
                            let m = measure(&mem, CLIPPY_SPANS, &clippy_corpus);
                            println!("{}", m.report(&format!("store:{repo}")));
                        }
                        Err(e) => panic!("{repo}: store copy must open: {e}"),
                    }
                }
            }
        }
    }
}
