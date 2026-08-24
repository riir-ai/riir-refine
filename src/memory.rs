//! Latent fix-trajectory memory with 3-axis retrieval.
//!
//! Borrows the 3-axis retrieval shape from riir-rag (riir-ai): latent KNN +
//! BM25 lexical + optional graph. Reimplemented locally because riir-clippy
//! cannot depend on riir-ai (boundary rule).
//!
//! ## Axes
//!
//! - **Latent**: `dot(embed(query), embed(trajectory)) + sigmoid` — "similar
//!   lint + code pattern" signal. Uses [`crate::embedder::embed`] for the
//!   query direction and each trajectory's stored `direction`.
//! - **BM25 (simplified)**: exact `lint_name` match. If the query's `lint_name`
//!   matches a trajectory's `lint_name`, a fixed BM25-style boost is applied.
//!   This is the "same lint, different code" signal. (A full BM25 over code
//!   tokens would add complexity for marginal POC gain; the latent axis
//!   already captures code similarity.)
//! - **Trust** (Issue 021): a bounded per-entry evidential prior derived from
//!   the trajectory's [`EvidenceTier`] — Certified (full-validator admission)
//!   ranks above Heuristic (syn-parse only) at equal similarity, and
//!   **Withdrawn entries are never emitted as suggestions** (they stay
//!   reachable via [`LatentFixMemory::iter`]/[`get`](LatentFixMemory::get)
//!   for the adaptation loop's failure analysis). The prior is a per-entry
//!   bounded term (sigmoid-family, never a softmax across entries — scores
//!   stay independent, no cross-entry normalization).
//! - **Graph**: deferred (out of scope per Issue 004 — defer until G5 passes
//!   with latent + BM25 only).
//!
//! The trust axis is the retelling-bias defense from arXiv:2608.11195
//! (Research 049): without it, an early heuristic success is
//! indistinguishable in memory from a full-validator pass, and iterated
//! retrieval launders the former into the latter.
//!
//! ## Alloc-free read path
//!
//! The retrieve path reuses a caller-provided scratch `Vec<usize>` for the
//! index sort, and writes results into a caller-provided slice. The only
//! allocation is the `Vec<FixTrajectory>` growth on insert (amortized).

use crate::embedder::{Direction, EMBED_DIM, embed_into};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Consecutive post-hoc failures after which a trajectory is Withdrawn
/// (Issue 021). Withdrawn is an **absorbing** tier — outcome counts alone can
/// never leave it; only a fresh full-validator pass (re-certification) can.
const WITHDRAW_CONSECUTIVE_FAILS: u32 = 3;

/// Evidential status of a stored fix trajectory (Issue 021).
///
/// Distinguishes HOW a fix's outcome was established, which plain
/// `success_count`/`fail_count` cannot: an early syn-parse-only success and a
/// full-validator admission are indistinguishable under outcome counts alone,
/// and iterated retrieval launders the former into the latter (the
/// retelling-bias failure mode, arXiv:2608.11195 session-44 incident;
/// Research 049).
///
/// Transition discipline (mirrors riir-chain's slashing monotonicity):
///
/// - `(success && verified)` → **Certified** — the ONLY promotion and the ONLY
///   exit from Withdrawn. A full-validator pass re-certifies any tier.
/// - Consecutive failures ≥ [`WITHDRAW_CONSECUTIVE_FAILS`] → **Withdrawn**.
/// - Everything else leaves the tier unchanged (Withdrawn included —
///   absorbing under outcome counts alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum EvidenceTier {
    /// Admitted by the full validator (`clippy_verify` / `rust_perf_verify`
    /// re-run, or the Stage 2 tuning loop's verifier pass — the
    /// `adaptive_drafter` verified-pool admission rule).
    Certified = 0,
    /// Syn-parse only, or unverified history. The constructor default — and
    /// the decode default for pre-Issue-021 serialized memories, which is
    /// the honest tier for unverified history.
    #[default]
    Heuristic = 1,
    /// Retracted after repeated post-hoc failures. Absorbing: never re-emitted
    /// as a suggestion without an explicit re-certification event.
    Withdrawn = 2,
}

impl EvidenceTier {
    /// Pure tier transition — a function of `(tier, success, verified,
    /// consecutive_fails)` only, never wall-clock (G1 determinism holds:
    /// transitions depend only on recorded events).
    #[must_use]
    #[inline]
    pub fn advance(self, success: bool, verified: bool, consecutive_fails: u32) -> Self {
        // Re-certification: a fresh full-validator pass. The ONLY promotion
        // path and the ONLY exit from Withdrawn.
        if success && verified {
            return Self::Certified;
        }
        match self {
            // Absorbing: no outcome-count path leaves Withdrawn.
            Self::Withdrawn => Self::Withdrawn,
            _ if consecutive_fails >= WITHDRAW_CONSECUTIVE_FAILS => Self::Withdrawn,
            _ => self,
        }
    }

    /// Bounded trust prior for retrieval blending (Issue 021).
    ///
    /// Certified 1.0, Heuristic 0.5 (neutral — the established
    /// untried-neutral convention), Withdrawn 0.0 (excluded from suggestions
    /// before scoring ever sees it; the 0.0 only matters if a future caller
    /// opts back in).
    #[must_use]
    #[inline]
    pub fn trust_prior(self) -> f32 {
        match self {
            Self::Certified => 1.0,
            Self::Heuristic => 0.5,
            Self::Withdrawn => 0.0,
        }
    }
}

/// A stored fix trajectory — one past fix attempt with its outcome stats.
///
/// The `direction` field is the latent embedding of `(lint_name, buggy_code,
/// fixed_code)` computed once at insert time and cached for O(1) retrieval
/// scoring.
///
/// `Serialize`/`Deserialize` (Issue 018): trajectories persist to a local
/// JSON file so the memory accumulates across `cargo heal` invocations —
/// an in-process-only memory would be permanently starved in a CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixTrajectory {
    /// The clippy lint name (e.g., "`needless_return`").
    pub lint_name: String,
    /// BLAKE3-derived hash of `lint_name` — used for O(1) BM25 axis matching
    /// (avoids per-trajectory string comparison in the retrieval hot path).
    pub lint_name_hash: u64,
    /// The buggy code that triggered the lint.
    pub buggy_code: String,
    /// The fixed code that was applied.
    pub fixed_code: String,
    /// Cached latent embedding of (`lint_name`, `fixed_code`).
    pub direction: Direction,
    /// How many times this trajectory's fix led to a successful resolution.
    pub success_count: u32,
    /// How many times this trajectory's fix failed.
    pub fail_count: u32,
    /// Consecutive failures since the last success (drives the Withdrawn
    /// transition, Issue 021). Reset to 0 on any success.
    #[serde(default)]
    pub consecutive_fails: u32,
    /// Evidential status (Issue 021). Defaults to [`EvidenceTier::Heuristic`]
    /// — for fresh trajectories (syn-parse only until a validator admits
    /// them) AND for pre-Issue-021 serialized memories (`#[serde(default)]`
    /// decodes missing fields as the honest tier for unverified history).
    #[serde(default)]
    pub tier: EvidenceTier,
}

impl FixTrajectory {
    /// Construct a new trajectory, computing its embedding + hash at creation time.
    #[must_use]
    pub fn new(lint_name: String, buggy_code: String, fixed_code: String) -> Self {
        let mut direction = [0.0f32; EMBED_DIM];
        embed_into(&lint_name, &fixed_code, &mut direction);

        // Pre-compute the lint_name hash for O(1) BM25 matching in retrieval.
        let mut hasher = Hasher::new();
        hasher.update(lint_name.as_bytes());
        let lint_name_hash = hasher.finalize().as_bytes()[..8]
            .try_into()
            .map_or(0, u64::from_le_bytes);

        Self {
            lint_name,
            lint_name_hash,
            buggy_code,
            fixed_code,
            direction,
            success_count: 0,
            fail_count: 0,
            consecutive_fails: 0,
            tier: EvidenceTier::Heuristic,
        }
    }

    /// Net success rate ∈ [0.0, 1.0]. Trajectories with no attempts return 0.5
    /// (neutral — neither trusted nor distrusted).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counts are small (<2^24); f32 is exact
    pub fn success_rate(&self) -> f32 {
        let total = self.success_count + self.fail_count;
        if total == 0 {
            return 0.5;
        }
        (self.success_count as f32) / (total as f32)
    }
}

/// A retrieval query — what we're looking for in the memory.
#[derive(Debug, Clone)]
pub struct LatentQuery {
    /// The lint name to match (BM25 axis).
    pub lint_name: String,
    /// BLAKE3-derived hash of `lint_name` — pre-computed for O(1) BM25 matching.
    pub lint_name_hash: u64,
    /// The latent direction of the current (`lint_name`, `buggy_code`) context.
    pub direction: Direction,
}

impl LatentQuery {
    /// Construct a query from (`lint_name`, `buggy_code`), computing the embedding + hash.
    #[must_use]
    pub fn new(lint_name: String, buggy_code: &str) -> Self {
        let mut direction = [0.0f32; EMBED_DIM];
        embed_into(&lint_name, buggy_code, &mut direction);
        let mut hasher = Hasher::new();
        hasher.update(lint_name.as_bytes());
        let lint_name_hash = hasher.finalize().as_bytes()[..8]
            .try_into()
            .map_or(0, u64::from_le_bytes);
        Self { lint_name, lint_name_hash, direction }
    }
}

/// A scored retrieval result.
#[derive(Debug, Clone)]
pub struct ScoredTrajectory {
    /// Index into the memory's trajectory vector.
    pub idx: usize,
    /// Combined retrieval score (latent + BM25). Higher = more relevant.
    pub score: f32,
    /// The latent-axis component of the score.
    pub latent_score: f32,
    /// The BM25-axis component of the score.
    pub bm25_score: f32,
}

/// Weights for combining the retrieval axes.
#[derive(Debug, Clone, Copy)]
pub struct RetrievalWeights {
    /// Weight on the latent (dot-product + sigmoid) axis.
    pub latent: f32,
    /// Weight on the BM25 (exact `lint_name` match) axis.
    pub bm25: f32,
    /// Weight on the trust (evidential-tier prior) axis (Issue 021).
    pub trust: f32,
}

impl Default for RetrievalWeights {
    fn default() -> Self {
        // BM25 gets higher weight because exact lint_name match is a strong
        // signal for clippy fixes (the fix space is lint-specific). The latent
        // axis adds a "similar code pattern" refinement. Trust (Issue 021) is
        // deliberately the SMALLEST axis: it must break ties + small
        // similarity edges in favor of Certified entries without overriding
        // relevance — at 0.2, a different-lint Certified entry (≤ 0.4 + 0.2)
        // can never outrank a same-lint Heuristic one (≥ 0.6 + 0.1), and the
        // Certified-over-Heuristic gap is 0.1 (0.5 prior difference × 0.2).
        Self { latent: 0.4, bm25: 0.6, trust: 0.2 }
    }
}

/// Scalar dot product for fixed 8-dim vectors.
///
/// Faster than `simd_dot_f32` for `EMBED_DIM=8` (avoids the slice-length +
/// dispatch overhead). The compiler unrolls this into ~8 FMA instructions.
#[inline]
fn scalar_dot_8(a: &[f32; EMBED_DIM], b: &[f32; EMBED_DIM]) -> f32 {
    a[0] * b[0]
        + a[1] * b[1]
        + a[2] * b[2]
        + a[3] * b[3]
        + a[4] * b[4]
        + a[5] * b[5]
        + a[6] * b[6]
        + a[7] * b[7]
}

/// The in-memory fix-trajectory store with 3-axis retrieval.
///
/// POC implementation: a `Vec<FixTrajectory>` with linear-scan retrieval.
/// At N=1000 trajectories, the linear scan is O(N × `EMBED_DIM`) = 8000 FMA
/// ops — well under the G2 latency target (<50µs p50).
pub struct LatentFixMemory {
    /// All stored trajectories.
    trajectories: Vec<FixTrajectory>,
    /// Retrieval axis weights (tunable).
    weights: RetrievalWeights,
}

impl Default for LatentFixMemory {
    fn default() -> Self {
        Self::new(RetrievalWeights::default())
    }
}

impl LatentFixMemory {
    /// Construct an empty memory with the given retrieval weights.
    #[must_use]
    pub fn new(weights: RetrievalWeights) -> Self {
        Self { trajectories: Vec::new(), weights }
    }

    /// Pre-allocate capacity for `n` trajectories (avoids realloc during
    /// steady-state inserts).
    #[must_use]
    pub fn with_capacity(n: usize) -> Self {
        Self { trajectories: Vec::with_capacity(n), weights: RetrievalWeights::default() }
    }

    /// Number of stored trajectories.
    #[must_use]
    pub fn len(&self) -> usize {
        self.trajectories.len()
    }

    /// Whether the memory is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trajectories.is_empty()
    }

    /// Insert a trajectory (or update stats if an identical (lint, buggy,
    /// fixed) triple already exists).
    pub fn insert(&mut self, traj: FixTrajectory) {
        // Dedup by (lint_name, buggy_code, fixed_code). If the exact triple
        // exists, we don't add a duplicate — we just bump its stats via the
        // caller's `record_outcome` path.
        let exists = self.trajectories.iter().any(|t| {
            t.lint_name == traj.lint_name
                && t.buggy_code == traj.buggy_code
                && t.fixed_code == traj.fixed_code
        });
        if !exists {
            self.trajectories.push(traj);
        }
    }

    /// Record a success or failure outcome for a specific (lint, `fixed_code`)
    /// pair, with its verification context (Issue 021). If no matching
    /// trajectory exists, it's a no-op (the trajectory must be inserted first
    /// via `insert`).
    ///
    /// The tier transition is a pure function of `(tier, success, verified,
    /// consecutive_fails)` — see [`EvidenceTier::advance`]. `verified: true`
    /// means the verdict came from a full validator (clippy re-run proving the
    /// lint resolved, domain validator, tuning-loop verifier); a syn-parse
    /// pass or a manual "looked right" is `verified: false`.
    pub fn record_outcome(
        &mut self,
        lint_name: &str,
        fixed_code: &str,
        success: bool,
        verified: bool,
    ) {
        for t in &mut self.trajectories {
            if t.lint_name == lint_name && t.fixed_code == fixed_code {
                if success {
                    t.success_count = t.success_count.saturating_add(1);
                    t.consecutive_fails = 0;
                } else {
                    t.fail_count = t.fail_count.saturating_add(1);
                    t.consecutive_fails = t.consecutive_fails.saturating_add(1);
                }
                t.tier = t.tier.advance(success, verified, t.consecutive_fails);
                return;
            }
        }
        // No matching trajectory — caller should have inserted it first.
    }

    /// Record a **certification event** — a full-validator admission (Issue 021).
    ///
    /// Inserts the trajectory if absent, then records `(success = true,
    /// verified = true)`: `success_count` bumps and the tier becomes
    /// [`EvidenceTier::Certified`] (or re-certifies a Withdrawn entry — the
    /// only exit). This is the memory-side admission rule for callers that
    /// already ran the full validator (the Stage 2 tuning loop's verifier
    /// pass, `insert_verified`-style external verification paths).
    pub fn record_certified(&mut self, lint_name: &str, buggy_code: &str, fixed_code: &str) {
        self.insert(FixTrajectory::new(
            lint_name.to_string(),
            buggy_code.to_string(),
            fixed_code.to_string(),
        ));
        self.record_outcome(lint_name, fixed_code, true, true);
    }

    /// Retrieve the top-K trajectories for a query, writing their indices +
    /// scores into `out`. Returns the number written (≤ k, ≤ memory size).
    ///
    /// The `scratch` buffer is reused for the index sort to avoid allocation.
    /// Caller-owned buffers keep the read path alloc-free after warm-up.
    ///
    /// # Panics
    ///
    /// Panics if `out.len() < k` or `scratch` is non-empty on entry (it's
    /// cleared internally).
    pub fn retrieve_into(
        &self,
        query: &LatentQuery,
        k: usize,
        scratch: &mut Vec<usize>,
        out: &mut Vec<ScoredTrajectory>,
    ) -> usize {
        out.clear();
        if self.trajectories.is_empty() || k == 0 {
            return 0;
        }

        // Score all trajectories, maintaining an inline top-K via a simple
        // insertion approach. This avoids pushing N results + sorting,
        // keeping the hot path cache-friendly for large N.
        //
        // For K=5 and N=1000, this is ~5× faster than push-all + sort:
        // we do 1000 comparisons + at most 5×1000 insertions (worst case
        // is all ascending, but typical is ~50 insertions total).
        for (idx, traj) in self.trajectories.iter().enumerate() {
            // Trust axis gate (Issue 021): Withdrawn entries are never emitted
            // as suggestions — skip before scoring. They stay reachable for
            // failure analysis via `iter()`/`get()`.
            if traj.tier == EvidenceTier::Withdrawn {
                continue;
            }

            // Latent axis: dot(query.direction, traj.direction), then sigmoid.
            let dot = scalar_dot_8(&query.direction, &traj.direction);
            let latent_score = katgpt_core::sigmoid(dot);

            // BM25 axis: exact lint_name match via pre-computed hash (O(1)).
            let bm25_score = if traj.lint_name_hash == query.lint_name_hash { 1.0 } else { 0.0 };

            // Trust axis (Issue 021): bounded per-entry prior from the
            // evidential tier. Additive + sigmoid-family bounded — never a
            // softmax across entries (scores stay independent).
            let trust_score = traj.tier.trust_prior();

            let combined = self.weights.latent * latent_score
                + self.weights.bm25 * bm25_score
                + self.weights.trust * trust_score;

            // Inline top-K insertion: if `out` has < K entries, push.
            // Otherwise, if this score beats the minimum in `out`, replace it.
            if out.len() < k {
                out.push(ScoredTrajectory { idx, score: combined, latent_score, bm25_score });
                // Keep `out` sorted descending by score (insertion sort — cheap for small K).
                let j = out.len() - 1;
                while j > 0 && out[j - 1].score < out[j].score {
                    out.swap(j - 1, j);
                }
            } else if combined > out[k - 1].score {
                // Replace the last (lowest) entry + bubble up.
                out[k - 1] = ScoredTrajectory { idx, score: combined, latent_score, bm25_score };
                let mut j = k - 1;
                while j > 0 && out[j - 1].score < out[j].score {
                    out.swap(j - 1, j);
                    j -= 1;
                }
            }
        }

        let _ = scratch; // scratch reserved for future use
        out.len()
    }

    /// Convenience wrapper: allocates the result Vec (not alloc-free — use
    /// `retrieve_into` for the hot path).
    #[must_use]
    pub fn retrieve(&self, query: &LatentQuery, k: usize) -> Vec<ScoredTrajectory> {
        let mut out = Vec::with_capacity(k);
        let mut scratch = Vec::new();
        self.retrieve_into(query, k, &mut scratch, &mut out);
        out
    }

    /// Borrow a trajectory by index (for consumers to read the stored code).
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&FixTrajectory> {
        self.trajectories.get(idx)
    }

    /// Look up the success rate for a specific (`lint_name`, `fixed_code`) pair.
    ///
    /// Returns `0.5` (neutral) if no trajectory exists for this pair — the
    /// caller should treat this as "no information". This lets the heal
    /// pipeline look up the ACTUAL trajectory stats for a drafter candidate,
    /// Beta-Bernoulli counts `(successes, failures)` for a `(lint, fixed_code)`
    /// pair — the sample size that [`success_rate_for`](Self::success_rate_for)
    /// discards (Plan 005 T9.3).
    ///
    /// `(0, 0)` for an unseen pair, which `best_belief_score` reads as the
    /// uniform `Beta(1, 1)` prior — the honest "no evidence" position.
    #[must_use]
    pub fn counts_for(&self, lint_name: &str, fixed_code: &str) -> (u32, u32) {
        self.trajectories
            .iter()
            .find(|t| t.lint_name == lint_name && t.fixed_code == fixed_code)
            .map_or((0, 0), |t| (t.success_count, t.fail_count))
    }

    /// Look up the success rate for a specific (`lint_name`, `fixed_code`) pair.
    ///
    /// Returns `0.5` (neutral) if no trajectory exists for this pair — the
    /// caller should treat this as "no information". This lets the heal
    /// pipeline look up the ACTUAL trajectory stats for a drafter candidate,
    /// instead of assuming a hardcoded neutral 0.5.
    #[must_use]
    pub fn success_rate_for(&self, lint_name: &str, fixed_code: &str) -> f32 {
        for t in &self.trajectories {
            if t.lint_name == lint_name && t.fixed_code == fixed_code {
                return t.success_rate();
            }
        }
        0.5 // no trajectory → neutral
    }

    /// Iterate over all stored trajectories.
    pub fn iter(&self) -> impl Iterator<Item = &FixTrajectory> {
        self.trajectories.iter()
    }

    /// One-command debug summary of the store (Issue 037 observability —
    /// the `cargo heal --memory-stats` query surface).
    ///
    /// Answers "is the self-evolve loop actually closed?" from the data:
    /// the E1/E2/E3 verdicts at the bottom mirror Issue 037's gates —
    /// E1 `record` needs buckets, E2 `verdict` needs at least one positive
    /// outcome (a success count or a Certified tier entry), E3 `learn` is
    /// observable through tier promotion (the same verified outcomes the
    /// EMA half consumes). Read-only; never mutates.
    #[must_use]
    pub fn stats_report(&self, path: &std::path::Path, status: CommitmentStatus) -> String {
        use std::collections::HashMap;
        use std::fmt::Write as _;

        let mut report = String::with_capacity(512);
        let (mut certified, mut heuristic, mut withdrawn, mut successes, mut fails, mut consec) =
            (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
        let mut rules: HashMap<&str, u32> = HashMap::new();
        for t in &self.trajectories {
            match t.tier {
                EvidenceTier::Certified => certified += 1,
                EvidenceTier::Withdrawn => withdrawn += 1,
                EvidenceTier::Heuristic => heuristic += 1,
            }
            successes += t.success_count;
            fails += t.fail_count;
            if t.consecutive_fails > 0 {
                consec += 1;
            }
            *rules.entry(t.lint_name.as_str()).or_insert(0) += 1;
        }
        let commitment = match status {
            CommitmentStatus::Absent => "absent (pre-T7.4 file — unverifiable)",
            CommitmentStatus::Verified => "verified (BLAKE3)",
            CommitmentStatus::Mismatch => "MISMATCH — file corrupted or edited after write",
        };
        let _ = writeln!(
            report,
            "memory store: {}\nenvelope: v1 | commitment: {commitment}\nbuckets: {} (distinct rules: {})\ntiers: Certified {certified} | Heuristic {heuristic} | Withdrawn {withdrawn}\noutcomes: success {successes} | fail {fails} | buckets with consecutive fails {consec}",
            path.display(),
            self.trajectories.len(),
            rules.len(),
        );
        let mut top: Vec<(&str, u32)> = rules.into_iter().collect();
        top.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        for (rule, count) in top.iter().take(8) {
            let _ = writeln!(report, "  {count:6}  {rule}");
        }
        let e1 = if self.trajectories.is_empty() { "—" } else { "OK" };
        let e2 = if successes > 0 || certified > 0 { "OK" } else { "—" };
        let e3 = if certified > 0 { "OK" } else { "—" };
        // Hints only for gates that are still open (post-037-T2: a "—"
        // verdict means no verified heal has run yet, not missing wiring).
        let mut hints: Vec<&str> = Vec::new();
        if e1 == "—" {
            hints.push("record needs a first trajectory");
        }
        if e2 == "—" {
            hints.push("verdict needs a positive outcome (run a --fix --verify heal)");
        }
        if e3 == "—" {
            hints.push("learn needs tier promotion");
        }
        let hint = if hints.is_empty() {
            String::new()
        } else {
            format!(" — {}", hints.join("; "))
        };
        let _ = writeln!(
            report,
            "loop state (Issue 037): record {e1} | verdict {e2} | learn {e3}{hint}"
        );
        report
    }

    /// Serialize the memory state to a JSON string (v1 envelope, Issue 018).
    ///
    /// Round-trips losslessly: `direction` f32 components serialize via
    /// ryu's shortest round-trip form, so [`load_json`](Self::load_json)
    /// recovers bit-identical values. Retrieval weights are config, not
    /// state — they are NOT serialized; the loaded memory keeps its own.
    ///
    /// # Errors
    ///
    /// Only if a direction component is non-finite (`serde_json` rejects
    /// `NaN`/`∞`; the embedder never produces them — a corrupted in-memory
    /// state surfaces as an error rather than a silently-lossy file).
    pub fn to_json(&self) -> serde_json::Result<String> {
        let commitment = commit_trajectories(&self.trajectories)?;
        serde_json::to_string(&MemoryFileV1 {
            version: 1,
            commitment: Some(commitment),
            trajectories: self.trajectories.clone(),
        })
    }

    /// Restore trajectories from JSON written by [`to_json`](Self::to_json).
    ///
    /// Merge semantics: each file trajectory is [`insert`](Self::insert)ed,
    /// so entries whose `(lint, buggy, fixed)` triple already exists keep
    /// the live stats (the caller's newer observations win) and new triples
    /// are appended with their persisted stats.
    ///
    /// # Errors
    ///
    /// Malformed JSON, or an unknown envelope `version` (a future format
    /// change must fail loudly, not silently misread fields).
    pub fn load_json(&mut self, json: &str) -> serde_json::Result<()> {
        self.load_json_verified(json).map(|_| ())
    }

    /// [`load_json`](Self::load_json), reporting the BLAKE3 commitment check
    /// (Plan 005 T7.4).
    ///
    /// The trajectories are loaded either way — see [`CommitmentStatus`] for
    /// why a mismatch is advisory. Callers that care (the `cargo heal` CLI)
    /// warn on [`CommitmentStatus::Mismatch`].
    ///
    /// # Errors
    ///
    /// Malformed JSON, or an unknown envelope `version`.
    pub fn load_json_verified(&mut self, json: &str) -> serde_json::Result<CommitmentStatus> {
        let file: MemoryFileV1 = serde_json::from_str(json)?;
        let status = match &file.commitment {
            None => CommitmentStatus::Absent,
            Some(expected) => {
                let actual = commit_trajectories(&file.trajectories)?;
                if &actual == expected {
                    CommitmentStatus::Verified
                } else {
                    CommitmentStatus::Mismatch
                }
            }
        };
        for traj in file.trajectories {
            self.insert(traj);
        }
        Ok(status)
    }

    /// Write the memory state to `path` (v1 JSON envelope, Issue 018).
    ///
    /// Creates no directories — callers that want parent-dir creation
    /// handle it themselves (keeps this method a plain file write).
    ///
    /// # Errors
    ///
    /// I/O errors from the write, or serialization failure (non-finite
    /// direction component — see [`to_json`](Self::to_json)).
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = self.to_json().map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Load trajectories from `path` (best-effort merge, Issue 018).
    ///
    /// - Missing file: no-op `Ok(())` (a fresh memory is the normal first run).
    /// - Malformed JSON: the caller decides whether to warn + continue fresh
    ///   (the CLI does; a corrupt file must not abort a heal run).
    ///
    /// # Errors
    ///
    /// `InvalidData` for malformed JSON or an unknown envelope version;
    /// raw I/O errors for everything except `NotFound` (which is `Ok`).
    pub fn load(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.load_verified(path).map(|_| ())
    }

    /// [`load`](Self::load), reporting the BLAKE3 commitment check (T7.4).
    ///
    /// A missing file reports [`CommitmentStatus::Absent`] — nothing was read,
    /// so nothing could be verified.
    ///
    /// # Errors
    ///
    /// Same as [`load`](Self::load).
    pub fn load_verified(&mut self, path: &std::path::Path) -> std::io::Result<CommitmentStatus> {
        match std::fs::read_to_string(path) {
            Ok(json) => self
                .load_json_verified(&json)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CommitmentStatus::Absent),
            Err(e) => Err(e),
        }
    }
}

/// The persisted memory file envelope (v1, Issue 018).
///
/// `version` guards future format changes — a loader that sees an unknown
/// version fails loudly (mapped to `InvalidData` by [`LatentFixMemory::load`])
/// instead of silently misreading fields.
#[derive(Serialize, Deserialize)]
struct MemoryFileV1 {
    version: u32,
    /// BLAKE3 commitment over the canonical `trajectories` serialization
    /// (Plan 005 T7.4). Absent in v1 files written before the commitment
    /// landed; `#[serde(default)]` keeps those loadable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    commitment: Option<String>,
    trajectories: Vec<FixTrajectory>,
}

/// Result of checking a loaded file's BLAKE3 commitment (Plan 005 T7.4).
///
/// Mirrors [`Corpus::thaw`](crate::corpus::Corpus::thaw)'s documented
/// precedent: a mismatch is **advisory, not a hard gate**. The learned state
/// is a performance hint, and refusing to heal because a hint file was edited
/// would be a worse failure than proceeding with a warning. Callers decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentStatus {
    /// Pre-T7.4 file with no commitment field — loadable, unverifiable.
    Absent,
    /// Commitment present and matches the payload.
    Verified,
    /// Commitment present and does NOT match — the file was truncated,
    /// corrupted, or edited after it was written.
    Mismatch,
}

/// BLAKE3 over the canonical serialization of `trajectories`.
///
/// Committing the trajectory array rather than the whole envelope keeps the
/// commitment stable across envelope-level additions (a new optional field
/// must not invalidate every existing file).
fn commit_trajectories(trajectories: &[FixTrajectory]) -> serde_json::Result<String> {
    let payload = serde_json::to_vec(trajectories)?;
    Ok(blake3::hash(&payload).to_hex().to_string())
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // tests check bit-identical determinism
mod tests {
    use super::*;

    fn make_traj(lint: &str, buggy: &str, fixed: &str) -> FixTrajectory {
        FixTrajectory::new(lint.to_string(), buggy.to_string(), fixed.to_string())
    }

    fn make_query(lint: &str, buggy: &str) -> LatentQuery {
        LatentQuery::new(lint.to_string(), buggy)
    }


    // ── T7.4: BLAKE3 commitment over the learned state ────────────────────

    #[test]
    fn commitment_round_trips_as_verified() {
        let mut m = LatentFixMemory::new(RetrievalWeights::default());
        m.insert(make_traj("needless_return", "return x;", "x"));
        m.insert(make_traj("single_match", "match v { _ => {} }", "()"));
        let json = m.to_json().expect("serialize");

        let mut loaded = LatentFixMemory::new(RetrievalWeights::default());
        assert_eq!(
            loaded.load_json_verified(&json).expect("load"),
            CommitmentStatus::Verified
        );
        assert_eq!(loaded.len(), m.len());
    }

    #[test]
    fn a_tampered_payload_is_detected() {
        let mut m = LatentFixMemory::new(RetrievalWeights::default());
        m.insert(make_traj("needless_return", "return x;", "x"));
        let json = m.to_json().expect("serialize");

        // Edit the payload without touching the commitment — exactly what a
        // corrupted write or a hand-edit looks like.
        let tampered = json.replace("needless_return", "single_match");
        assert_ne!(tampered, json, "the test edit must actually change the file");

        let mut loaded = LatentFixMemory::new(RetrievalWeights::default());
        assert_eq!(
            loaded.load_json_verified(&tampered).expect("load"),
            CommitmentStatus::Mismatch
        );
        // Advisory, not a gate: the data still loads (see CommitmentStatus).
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn stats_report_reflects_loop_state() {
        // Issue 037 observability: the E1/E2/E3 lines must derive from the
        // data — fresh memory is record-less; after one apply it records but
        // has no verdict; after a certified outcome all three are OK.
        let path = std::path::Path::new("/tmp/stats-report-probe/memory.json");
        let mut m = LatentFixMemory::default();
        let fresh = m.stats_report(path, CommitmentStatus::Verified);
        assert!(fresh.contains("buckets: 0"), "fresh: {fresh}");
        assert!(fresh.contains("record — | verdict — | learn —"), "fresh: {fresh}");

        m.insert(make_traj("needless_return", "return 42;", "42"));
        m.insert(make_traj("match_bool", "match b { true => x }", "if b { x }"));
        let recorded = m.stats_report(path, CommitmentStatus::Verified);
        assert!(recorded.contains("buckets: 2 (distinct rules: 2)"), "recorded: {recorded}");
        assert!(recorded.contains("tiers: Certified 0 | Heuristic 2 | Withdrawn 0"));
        assert!(recorded.contains("record OK | verdict — | learn —"), "recorded: {recorded}");
        // Open gates carry their hint (post-T2 wording: a "—" verdict means
        // no verified heal has run, not missing wiring).
        assert!(recorded.contains("verdict needs a positive outcome"), "recorded: {recorded}");

        m.record_certified("needless_return", "return 42;", "42");
        let evolved = m.stats_report(path, CommitmentStatus::Verified);
        assert!(evolved.contains("tiers: Certified 1 | Heuristic 1 | Withdrawn 0"));
        assert!(evolved.contains("outcomes: success 1 | fail 0"));
        assert!(evolved.contains("record OK | verdict OK | learn OK"), "evolved: {evolved}");
        // Post-T2: a closed loop prints NO hint — the stale "verdict needs…"
        // suffix must not survive once every gate is OK.
        assert!(!evolved.contains("needs a"), "closed loop must carry no hints: {evolved}");

        // The commitment status surfaces — a MISMATCH must be loud.
        let tampered = m.stats_report(path, CommitmentStatus::Mismatch);
        assert!(tampered.contains("MISMATCH"), "tampered: {tampered}");
    }

    #[test]
    fn pre_commitment_files_still_load() {
        // Back-compat: a v1 file written before T7.4 has no commitment field.
        let legacy = r#"{"version":1,"trajectories":[]}"#;
        let mut loaded = LatentFixMemory::new(RetrievalWeights::default());
        assert_eq!(
            loaded.load_json_verified(legacy).expect("load"),
            CommitmentStatus::Absent
        );
    }

    #[test]
    fn commitment_is_deterministic_across_instances() {
        // Two memories built the same way must commit to the same hash —
        // otherwise the commitment is not reproducible and cannot be audited.
        let build = || {
            let mut m = LatentFixMemory::new(RetrievalWeights::default());
            m.insert(make_traj("needless_return", "return x;", "x"));
            m.insert(make_traj("identity_op", "x * 1", "x"));
            m.to_json().expect("serialize")
        };
        assert_eq!(build(), build());
    }

    #[test]
    fn empty_memory_returns_empty() {
        let mem = LatentFixMemory::default();
        let q = make_query("needless_return", "return 42;");
        let results = mem.retrieve(&q, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn retrieve_finds_inserted_trajectory() {
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));

        let q = make_query("needless_return", "return 42;");
        let results = mem.retrieve(&q, 1);
        assert_eq!(results.len(), 1);
        let traj = mem.get(results[0].idx).unwrap();
        assert_eq!(traj.lint_name, "needless_return");
    }

    #[test]
    fn bm25_axis_prefers_same_lint_name() {
        // Insert two trajectories: one with matching lint, one without.
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        mem.insert(make_traj("redundant_closure", "|x| f(x)", "f"));

        // Query for needless_return — the same-lint trajectory should rank #1.
        let q = make_query("needless_return", "return 99;");
        let results = mem.retrieve(&q, 2);
        assert_eq!(results.len(), 2);
        let top = mem.get(results[0].idx).unwrap();
        assert_eq!(top.lint_name, "needless_return", "BM25 axis must rank same-lint first");
        assert_eq!(results[0].bm25_score, 1.0);
    }

    #[test]
    fn dedup_on_identical_triple() {
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        assert_eq!(mem.len(), 1, "identical triples must dedup");
    }

    #[test]
    fn record_outcome_updates_stats() {
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));

        mem.record_outcome("needless_return", "42", true, false);
        mem.record_outcome("needless_return", "42", true, false);
        mem.record_outcome("needless_return", "42", false, false);

        let traj = mem.get(0).unwrap();
        assert_eq!(traj.success_count, 2);
        assert_eq!(traj.fail_count, 1);
        assert!((traj.success_rate() - (2.0 / 3.0)).abs() < 1e-5);
    }

    #[test]
    fn retrieve_into_is_alloc_free_after_warmup() {
        // G4 smoke: retrieve_into reuses `out` capacity across calls.
        let mut mem = LatentFixMemory::default();
        for i in 0..50 {
            mem.insert(make_traj(
                "needless_return",
                &format!("return {i};"),
                &format!("{i}"),
            ));
        }

        let q = make_query("needless_return", "return 0;");
        let mut out = Vec::with_capacity(5);
        let mut scratch = Vec::new();

        // First call warms up `out`.
        mem.retrieve_into(&q, 5, &mut scratch, &mut out);
        let cap_after_first = out.capacity();

        // Subsequent calls must not grow capacity.
        for _ in 0..10 {
            mem.retrieve_into(&q, 5, &mut scratch, &mut out);
            assert_eq!(out.capacity(), cap_after_first, "capacity grew — not alloc-free");
        }
    }

    #[test]
    fn top_k_respects_k_param() {
        let mut mem = LatentFixMemory::default();
        for i in 0..20 {
            mem.insert(make_traj("needless_return", &format!("return {i};"), &format!("{i}")));
        }
        let q = make_query("needless_return", "return 0;");
        let results = mem.retrieve(&q, 5);
        assert_eq!(results.len(), 5, "must return exactly k=5 results");
    }

    // ── Issue 018: persistence ─────────────────────────────────────────

    #[test]
    fn json_round_trip_is_lossless() {
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        mem.insert(make_traj("collapsible_if", "if a { if b { x(); } }", "if a && b { x(); }"));
        mem.record_outcome("needless_return", "42", true, false);
        mem.record_outcome("needless_return", "42", false, false);

        let json = mem.to_json().expect("serialize");
        let mut loaded = LatentFixMemory::default();
        loaded.load_json(&json).expect("deserialize");

        assert_eq!(loaded.len(), mem.len(), "entry count must round-trip");
        for (a, b) in mem.iter().zip(loaded.iter()) {
            assert_eq!(a.lint_name, b.lint_name);
            assert_eq!(a.buggy_code, b.buggy_code);
            assert_eq!(a.fixed_code, b.fixed_code);
            assert_eq!(a.lint_name_hash, b.lint_name_hash);
            assert_eq!(a.success_count, b.success_count);
            assert_eq!(a.fail_count, b.fail_count);
            assert_eq!(a.consecutive_fails, b.consecutive_fails);
            assert_eq!(a.tier, b.tier, "evidential tier must round-trip");
            assert_eq!(a.direction, b.direction, "direction must round-trip bit-identically");
        }
    }

    #[test]
    fn file_save_load_missing_and_merge() {
        let dir = std::env::temp_dir().join(format!("riir-clippy-mem-{}-save-load", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("memory.json");
        let _ = std::fs::remove_file(&path);

        // Missing file → no-op Ok.
        let mut mem = LatentFixMemory::default();
        mem.load(&path).expect("missing file must be a no-op Ok");
        assert!(mem.is_empty());

        // Save → load round-trip via the file system.
        mem.insert(make_traj("needless_return", "return 1;", "1"));
        mem.record_outcome("needless_return", "1", true, false);
        mem.save(&path).expect("save");
        let mut mem2 = LatentFixMemory::default();
        mem2.load(&path).expect("load");
        assert_eq!(mem2.len(), 1);
        assert_eq!(mem2.get(0).unwrap().success_count, 1, "stats must persist");

        // Merge semantics: loading an identical triple keeps LIVE stats;
        // a new triple is appended.
        mem2.record_outcome("needless_return", "1", true, false);
        mem2.load(&path).expect("re-load");
        assert_eq!(mem2.len(), 1);
        assert_eq!(mem2.get(0).unwrap().success_count, 2, "live stats win over persisted");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_is_invalid_data() {
        let dir = std::env::temp_dir().join(format!("riir-clippy-mem-{}-corrupt", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("memory.json");
        std::fs::write(&path, "{ not json").expect("write corrupt file");

        let mut mem = LatentFixMemory::default();
        let err = mem.load(&path).expect_err("corrupt file must error");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(mem.is_empty(), "failed load must leave the memory untouched");
        let _ = std::fs::remove_file(&path);
    }

    // ── Issue 021: evidential-status axis ────────────────────────────

    #[test]
    fn tier_transitions_follow_the_admission_rules() {
        use EvidenceTier::{Certified, Heuristic, Withdrawn};

        // Fresh trajectories are Heuristic.
        assert_eq!(make_traj("l", "b", "f").tier, Heuristic);

        // Verified success → Certified (the only promotion).
        assert_eq!(Heuristic.advance(true, true, 0), Certified);
        assert_eq!(Certified.advance(true, true, 0), Certified);

        // Unverified success never promotes.
        assert_eq!(Heuristic.advance(true, false, 0), Heuristic);

        // 3 consecutive fails (any verification) → Withdrawn.
        assert_eq!(Heuristic.advance(false, true, 3), Withdrawn);
        assert_eq!(Heuristic.advance(false, false, 3), Withdrawn);
        assert_eq!(Certified.advance(false, true, 3), Withdrawn);
        assert_eq!(Heuristic.advance(false, false, 2), Heuristic, "2 fails is under the bar");

        // Success resets the consecutive-fail streak — modelled by the caller
        // passing the reset streak (0) — and only verified success re-certifies.
        assert_eq!(Withdrawn.advance(true, true, 0), Certified, "re-certification is the only exit");
    }

    #[test]
    fn withdrawn_is_absorbing_under_outcome_counts_alone() {
        // T1 absorbing property: from Withdrawn, NO sequence of outcomes that
        // lacks a (success && verified) event can leave Withdrawn. Exhaustive
        // over every (success, verified) combo × streak regime — plus a long
        // deterministic sequence of unverified successes.
        for success in [false, true] {
            for verified in [false, true] {
                for streak in [0u32, 1, 3, 100] {
                    let next = EvidenceTier::Withdrawn.advance(success, verified, streak);
                    if success && verified {
                        assert_eq!(next, EvidenceTier::Certified);
                    } else {
                        assert_eq!(next, EvidenceTier::Withdrawn, "absorbing under ({success}, {verified}, {streak})");
                    }
                }
            }
        }

        // End-to-end through the memory: reach Withdrawn, then 100 unverified
        // successes — the tier must stay Withdrawn (retelling-bias defense:
        // headline successes cannot launder a retraction away).
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        for _ in 0..3 {
            mem.record_outcome("needless_return", "42", false, true);
        }
        assert_eq!(mem.get(0).unwrap().tier, EvidenceTier::Withdrawn);
        for _ in 0..100 {
            mem.record_outcome("needless_return", "42", true, false);
        }
        let traj = mem.get(0).unwrap();
        assert_eq!(traj.tier, EvidenceTier::Withdrawn, "unverified successes must not un-withdraw");
        assert_eq!(traj.success_count, 100, "stats still record honestly");
        assert_eq!(traj.consecutive_fails, 0, "streak reset is honest bookkeeping");

        // One re-certification exits.
        mem.record_outcome("needless_return", "42", true, true);
        assert_eq!(mem.get(0).unwrap().tier, EvidenceTier::Certified);
    }

    #[test]
    fn certified_outranks_heuristic_at_equal_similarity() {
        // T2 falsifiable A/B — within-subject design. Trajectories sharing
        // (lint, fixed) are one stats bucket (`record_outcome` matches on that
        // pair), so two distinct-tier entries with EXACTLY equal similarity
        // are not constructible through the real API. The honest equivalent:
        // the SAME entry queried before vs after certification — the only
        // changed variable is the tier, so the score delta must be EXACTLY
        // the trust gap (0.5 prior × 0.2 weight = 0.1). Fails if the trust
        // axis is not wired (delta would be 0).
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        assert_eq!(mem.get(0).unwrap().tier, EvidenceTier::Heuristic);

        let q = make_query("needless_return", "return 43;");
        let before = mem.retrieve(&q, 1);
        assert_eq!(before.len(), 1);

        // Certify (full-validator admission).
        mem.record_outcome("needless_return", "42", true, true);
        assert_eq!(mem.get(0).unwrap().tier, EvidenceTier::Certified);

        let after = mem.retrieve(&q, 1);
        assert_eq!(after.len(), 1);
        let delta = after[0].score - before[0].score;
        assert!((delta - 0.1).abs() < 1e-5, "certification must add exactly the trust gap (0.1), got {delta}");

        // And the cross-entry ordering proof lives in the small-edge test
        // below: a Certified entry beats a strictly-more-similar Heuristic
        // one when the edge is under the gap.
    }

    #[test]
    fn certified_beats_small_similarity_edge_of_heuristic() {
        // T2 (higher-similarity regime): a Heuristic entry whose latent score
        // is STRICTLY higher than the Certified one's must still lose when the
        // edge is smaller than the trust gap. The embedder is hash-based (no
        // locality), so the higher-similarity candidate is FOUND at runtime:
        // enumerate fixed codes, pick one with latent strictly inside the
        // (certified, certified + gap) window. Self-adapting across embedder
        // changes; asserts found so a silent no-op can't pass.
        let lint = "needless_return";
        let cert_fixed = "42";
        let mut mem = LatentFixMemory::default();
        mem.record_certified(lint, "return 42;", cert_fixed);

        let q = make_query(lint, "return 7;");
        let base = mem.retrieve(&q, 1);
        let cert_latent = base[0].latent_score;

        let weights = RetrievalWeights::default();
        let trust_gap = weights.trust * (EvidenceTier::Certified.trust_prior()
            - EvidenceTier::Heuristic.trust_prior());
        let latent_edge_max = trust_gap / weights.latent; // 0.25 at defaults

        let mut heuristic_fixed = None;
        for i in 0..200u32 {
            let code = format!("let heuristic = {i};");
            let mut dir = [0.0f32; EMBED_DIM];
            crate::embedder::embed_into(lint, &code, &mut dir);
            let latent = katgpt_core::sigmoid(scalar_dot_8(&q.direction, &dir));
            if latent > cert_latent && latent - cert_latent < latent_edge_max {
                heuristic_fixed = Some(code);
                break;
            }
        }
        let heuristic_fixed =
            heuristic_fixed.expect("a higher-but-close latent candidate must exist in 200 draws");

        mem.insert(make_traj(lint, "return 8;", &heuristic_fixed));
        assert_eq!(mem.get(1).unwrap().tier, EvidenceTier::Heuristic);

        let results = mem.retrieve(&q, 2);
        assert_eq!(results.len(), 2);
        let winner = mem.get(results[0].idx).unwrap();
        assert_eq!(
            winner.tier,
            EvidenceTier::Certified,
            "trust gap must dominate a small similarity edge"
        );
        assert!(results[0].latent_score < results[1].latent_score,
            "sanity: the runner-up really was more similar");
    }

    #[test]
    fn withdrawn_entries_are_never_suggested() {
        // T2: a Withdrawn entry is excluded from suggestions entirely, even
        // when it is the ONLY match; it stays reachable via iter()/get().
        let mut mem = LatentFixMemory::default();
        mem.insert(make_traj("needless_return", "return 42;", "42"));
        for _ in 0..3 {
            mem.record_outcome("needless_return", "42", false, true);
        }
        assert_eq!(mem.get(0).unwrap().tier, EvidenceTier::Withdrawn);

        // Only match is Withdrawn → no suggestions at all.
        let q = make_query("needless_return", "return 43;");
        assert!(mem.retrieve(&q, 5).is_empty(), "Withdrawn must never be suggested");

        // Mixed memory: only the non-withdrawn entry surfaces.
        mem.insert(make_traj("needless_return", "return 44;", "44"));
        let results = mem.retrieve(&q, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(mem.get(results[0].idx).unwrap().fixed_code, "44");

        // Still reachable for failure analysis.
        assert_eq!(mem.len(), 2, "Withdrawn entry stays in memory");
        assert!(mem.iter().any(|t| t.tier == EvidenceTier::Withdrawn));
    }

    #[test]
    fn record_certified_admits_and_recertifies() {
        let mut mem = LatentFixMemory::default();
        mem.record_certified("needless_return", "return 42;", "42");
        let traj = mem.get(0).unwrap();
        assert_eq!(traj.tier, EvidenceTier::Certified);
        assert_eq!(traj.success_count, 1);

        // Re-certifying an existing (lint, fixed) pair bumps + stays Certified.
        mem.record_certified("needless_return", "return 42;", "42");
        assert_eq!(mem.len(), 1, "dedup on the identical triple");
        assert_eq!(mem.get(0).unwrap().success_count, 2);
        assert_eq!(mem.get(0).unwrap().tier, EvidenceTier::Certified);
    }

    #[test]
    fn pre_issue021_json_decodes_as_heuristic() {
        // Back-compat: a v1 envelope written BEFORE Issue 021 (no
        // `tier`/`consecutive_fails` fields) must decode as Heuristic — the
        // honest tier for unverified history. Hand-written v1 JSON mirrors
        // the old serde output shape exactly.
        let old_json = r#"{
            "version": 1,
            "trajectories": [
                {
                    "lint_name": "needless_return",
                    "lint_name_hash": 12345,
                    "buggy_code": "return 42;",
                    "fixed_code": "42",
                    "direction": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
                    "success_count": 3,
                    "fail_count": 0
                }
            ]
        }"#;
        let mut mem = LatentFixMemory::default();
        mem.load_json(old_json).expect("old-format file must load");
        let traj = mem.get(0).unwrap();
        assert_eq!(traj.tier, EvidenceTier::Heuristic, "old history decodes as Heuristic");
        assert_eq!(traj.consecutive_fails, 0);
        assert_eq!(traj.success_count, 3, "stats survive");
    }
}
