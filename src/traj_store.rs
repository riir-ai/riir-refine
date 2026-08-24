//! The neuron-db trajectory store (Plan 019 T4.3) — the DURABLE backend
//! for `--fix-compile` self-evolution, behind `latent_retrieval`.
//!
//! ## What it is
//!
//! [`LocalKvStore`] at `<repo>/.heal/trajectories.kv/` (encrypted WAL,
//! passphrase = a generated seed at `<repo>/.heal/key` — the threat model
//! is integrity + durability, not secrecy), plus the two retrieval axes
//! the plan's shape-conditional ordering (T4.2) consumes:
//!
//! - **lexical** — [`Bm25Index`] over each trajectory's shape + fix text,
//!   rebuilt at open (papaya lock-free reads).
//! - **latent** — the SAME deterministic 8-D embedding the in-memory
//!   [`LatentFixMemory`](crate::memory::LatentFixMemory) computes at
//!   `FixTrajectory::new` (`crate::embedder::embed_into`), stored per row
//!   and queried by sigmoid-dot (the house rule — never softmax).
//!
//! ## Substrate-first corrections to the plan (recorded, not papered over)
//!
//! - The plan's T4.3 wording says "latent axis `DenseEmbedIndex`".
//!   Measured against the substrate: `DenseEmbedIndex` is the **768-D**
//!   Bonsai dense-rerank path (`DENSE_EMBED_DIM`, blocked on C13 per
//!   riir-rag's own notes), while the modelless embedders are **8-D** —
//!   the dims do not compose. The honest 8-D substrate is the sigmoid-dot
//!   over stored directions (exactly what `LatentFixMemory`'s retrieval
//!   does), made durable. Same embedding, same similarity math.
//! - The plan's wording says "rows via `put_pod`". `put_pod` requires the
//!   caller to implement `bytemuck::Pod`, which would make bytemuck a NEW
//!   direct dep for one fixed-layout struct. The dep-light rule (Issue
//!   018 G4) wins: rows are one JSON document each (numeric + text halves
//!   together — also removes the companion-row join), via plain
//!   [`LocalKvStore::put`]. Row counts are tens-per-family; the encode
//!   cost is not on any hot path.
//!
//! ## Row layout
//!
//! ```text
//! traj:{code}:{strategy_id}:{blake3_12(shape)} → JSON TrajRowDoc
//! ```
//!
//! Family scans are `scan_prefix(b"traj:")`; a trajectory's counters
//! update by read-modify-write under the full-row key (idempotent upsert).
//! The tier transitions are Issue 021's, durable: `success && verified`
//! certifies; 3 consecutive failures withdraw (absorbing — only a
//! verified success exits).
//!
//! ## 037-T2 verdict attribution (038's schema half)
//!
//! Every `record` stamps [`TrajRowDoc::last_run_id`] (the heal-run that
//! produced the verdict — one id per CLI invocation) and
//! [`TrajRowDoc::last_seen_at`] (unix seconds — 037-A1's memory-cadence
//! half + 038-T1's timestamp field, landed here so 038 keeps a single
//! read path: in-schema fields, no sidecar). Pre-T2 rows decode with
//! `""`/`0` defaults (serde).
//!
//! ## One active backend per build (the constraint)
//!
//! The default (non-`latent_retrieval`) build keeps the `memory.json`
//! envelope (Issue 018's G4 dep-light rule) — no dual-write drift. With
//! `latent_retrieval` on, the compile domain records HERE.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use riir_neuron_db::bm25::Bm25Index;
use riir_neuron_db::local_kv::LocalKvStore;

use crate::embedder::{embed_into, EMBED_DIM};
use crate::memory::EvidenceTier;

/// One durable trajectory row (JSON doc — numeric + text halves together).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct TrajRowDoc {
    /// The trajectory key half: `"{code}:{strategy_id}"`.
    pub lint_key: String,
    /// The trigger text the fix replaced (the `buggy` half).
    pub shape: String,
    /// The replacement landed (the `fixed` half).
    pub fix: String,
    /// `embed_into(lint_key, fix)` — the same 8-D direction the in-memory
    /// memory computes (one embedding substrate for rules + lived
    /// experience).
    pub direction: Vec<f32>,
    pub success_count: u32,
    pub fail_count: u32,
    /// Consecutive failures since the last success (drives Withdrawn).
    pub consecutive_fails: u32,
    /// Evidential tier (Issue 021 semantics, durable).
    pub tier: u8,
    /// The heal-run id that produced the latest verdict (037-T2 / 038-T1:
    /// per-run attribution; `""` on rows written before the field
    /// existed — serde default keeps old rows decoding).
    #[serde(default)]
    pub last_run_id: String,
    /// Unix seconds of the latest verdict sighting (037-A1's cadence half;
    /// `0` on pre-T2 rows).
    #[serde(default)]
    pub last_seen_at: u64,
}

impl EvidenceTier {
    /// Stable `u8` for the row doc.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Certified => 0,
            Self::Heuristic => 1,
            Self::Withdrawn => 2,
        }
    }

    /// Inverse of [`Self::as_u8`] (unknown values → `Heuristic`, the
    /// honest tier for unverifiable history).
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Certified,
            2 => Self::Withdrawn,
            _ => Self::Heuristic,
        }
    }
}

/// One retrieved neighbor for shape-conditional ordering (T4.2's input).
#[derive(Clone, Debug, PartialEq)]
pub struct TrajNeighbor {
    pub lint_key: String,
    pub shape: String,
    pub fix: String,
    /// BM25 lexical score against the query shape.
    pub bm25: f32,
    /// `sigmoid(dot(query_dir, row_dir))` — the latent axis (house rule).
    pub latent: f32,
    pub success_count: u32,
    pub fail_count: u32,
    pub tier: EvidenceTier,
}

impl TrajNeighbor {
    /// The raw outcome rate T4.2 feeds `BetaPosterior` (relevance weighting
    /// happens at the consumer).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // trajectory counts are ~10^2
    pub fn outcome_rate(&self) -> f32 {
        let n = self.success_count + self.fail_count;
        if n == 0 {
            return 0.0;
        }
        self.success_count as f32 / n as f32
    }
}

/// The durable trajectory store.
pub struct TrajStore {
    kv: LocalKvStore,
    bm25: Bm25Index,
    /// The store root (`.heal/trajectories.kv`), for diagnostics.
    pub root: PathBuf,
}

/// blake3 of `shape`, first 12 bytes hex (the row-key identity).
fn shape_hash(shape: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(shape.as_bytes());
    let hex = h.finalize().to_hex().to_string();
    hex[..12].to_string()
}

/// Current unix time in seconds (the `last_seen_at` stamp).
fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Resolve or create the per-repo key seed at `<root>/.heal/key`. The
/// threat model is integrity + durability, not secrecy — the seed just
/// needs to be STABLE so the WAL stays readable across runs.
fn ensure_key_seed(heal_dir: &Path) -> std::io::Result<String> {
    let key_path = heal_dir.join("key");
    if key_path.exists() {
        let seed = std::fs::read_to_string(&key_path)?;
        return Ok(seed.trim().to_string());
    }
    // std-only randomness: time + an address + pid is plenty for an
    // integrity seed (not a secret — the file sits beside the store).
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0u128, |d| d.as_nanos());
    let a = std::process::id();
    let seed = format!("{t:x}{a:x}");
    std::fs::create_dir_all(heal_dir)?;
    std::fs::write(&key_path, &seed)?;
    Ok(seed)
}

impl TrajStore {
    /// Open (or create) the store under `<root>/.heal/trajectories.kv` and
    /// rebuild the BM25 axis from the rows.
    ///
    /// # Errors
    ///
    /// `LocalKvStore::open_dir` failures (WAL replay, unwritable dir) or
    /// the key-seed write. A corrupt row JSON does NOT fail the open — it
    /// is skipped + counted in the returned warning count (one bad row
    /// must not brick the store).
    pub fn open(root: &Path) -> Result<(Self, usize), String> {
        let heal = root.join(".heal");
        let store_dir = heal.join("trajectories.kv");
        let passphrase = ensure_key_seed(&heal).map_err(|e| format!("key seed: {e}"))?;
        let kv = LocalKvStore::open_dir(&store_dir, &passphrase)
            .map_err(|e| format!("open {}: {e}", store_dir.display()))?;
        let mut store = Self {
            kv,
            bm25: Bm25Index::new(),
            root: store_dir,
        };
        let bad = store.rebuild_bm25();
        Ok((store, bad))
    }

    /// Index every row into BM25. Returns the count of undecodable rows.
    fn rebuild_bm25(&mut self) -> usize {
        let mut bad = 0;
        for (k, v) in self.kv.scan_prefix(b"traj:") {
            match serde_json::from_slice::<TrajRowDoc>(&v) {
                Ok(t) => {
                    self.bm25
                        .index(&k, &format!("{} {}", t.shape, t.fix));
                }
                Err(_) => bad += 1,
            }
        }
        bad
    }

    /// Full row key: `traj:{code}:{strategy}:{blake3_12(shape)}`.
    fn row_key(lint_key: &str, shape: &str) -> Vec<u8> {
        format!("traj:{lint_key}:{}", shape_hash(shape)).into_bytes()
    }

    /// Upsert one outcome. Creates the row on first sight; increments the
    /// counters + transitions the tier on repeat (Issue 021 semantics,
    /// durable: `success && verified` certifies; 3 consecutive failures
    /// withdraw — absorbing, only a verified success exits). Stamps the
    /// verdict's run attribution: `run_id` + the current unix time
    /// (037-T2).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn record(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        success: bool,
        verified: bool,
        run_id: &str,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self
            .kv
            .get_bytes(&key)
            .and_then(|v| serde_json::from_slice::<TrajRowDoc>(&v).ok())
            .unwrap_or_else(|| {
                let mut dir = [0.0f32; EMBED_DIM];
                embed_into(lint_key, fix, &mut dir);
                TrajRowDoc {
                    lint_key: lint_key.to_string(),
                    shape: shape.to_string(),
                    fix: fix.to_string(),
                    direction: dir.to_vec(),
                    success_count: 0,
                    fail_count: 0,
                    consecutive_fails: 0,
                    tier: EvidenceTier::Heuristic.as_u8(),
                    last_run_id: String::new(),
                    last_seen_at: 0,
                }
            });
        // The text halves update with the latest sighting (the shape key
        // is stable, the fix may have been refined).
        row.fix = fix.to_string();
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        if success {
            row.success_count = row.success_count.saturating_add(1);
            row.consecutive_fails = 0;
            if verified {
                row.tier = EvidenceTier::Certified.as_u8();
            }
        } else {
            row.fail_count = row.fail_count.saturating_add(1);
            row.consecutive_fails = row.consecutive_fails.saturating_add(1);
            if row.consecutive_fails >= 3 {
                row.tier = EvidenceTier::Withdrawn.as_u8();
            }
        }
        let bytes =
            serde_json::to_vec(&row).map_err(|e| format!("encode row: {e}"))?;
        self.kv.put(&key, &bytes).map_err(|e| format!("put row: {e}"))?;
        // BM25 stays live for future queries in this process.
        self.bm25
            .index(&key, &format!("{} {}", row.shape, row.fix));
        Ok(())
    }

    /// Hybrid retrieval (T4.2's input): BM25 over shape+fix + sigmoid-dot
    /// over the stored 8-D directions, fused; `Withdrawn` rows EXCLUDED
    /// (the T7.4 rule); family-filtered by `lint_prefix` (e.g.
    /// `"E0502:"`); top-k by fused score.
    #[must_use]
    pub fn retrieve(&self, query_shape: &str, lint_prefix: &str, k: usize) -> Vec<TrajNeighbor> {
        if k == 0 {
            return Vec::new();
        }
        let mut qdir = [0.0f32; EMBED_DIM];
        // The query embeds the SHAPE (what the current error looks like);
        // rows embed (key, fix). Cross-axis by construction — the same
        // asymmetry the memory's retrieval carries.
        embed_into(lint_prefix, query_shape, &mut qdir);

        let bm25_hits: HashMap<Vec<u8>, f32> = self
            .bm25
            .search(query_shape, k.saturating_mul(4))
            .into_iter()
            .collect();

        let mut out: Vec<TrajNeighbor> = Vec::new();
        for (k_bytes, v) in self.kv.scan_prefix(b"traj:") {
            if !lint_prefix.is_empty()
                && !k_bytes
                    .strip_prefix(b"traj:")
                    .is_some_and(|id| id.starts_with(lint_prefix.as_bytes()))
            {
                continue; // family filter
            }
            let Ok(row) = serde_json::from_slice::<TrajRowDoc>(&v) else {
                continue;
            };
            let tier = EvidenceTier::from_u8(row.tier);
            if matches!(tier, EvidenceTier::Withdrawn) {
                continue; // T7.4: excluded from retrieval
            }
            let mut rdir = [0.0f32; EMBED_DIM];
            for (i, x) in row.direction.iter().take(EMBED_DIM).enumerate() {
                rdir[i] = *x;
            }
            let latent = sigmoid_dot_8(&qdir, &rdir);
            let bm25 = bm25_hits.get(&k_bytes).copied().unwrap_or(0.0);
            out.push(TrajNeighbor {
                lint_key: row.lint_key,
                shape: row.shape,
                fix: row.fix,
                bm25,
                latent,
                success_count: row.success_count,
                fail_count: row.fail_count,
                tier,
            });
        }
        // Fuse: normalize bm25 by the max hit (unbounded scale), add the
        // already-bounded latent.
        let max_bm25 = out.iter().map(|n| n.bm25).fold(0.0f32, f32::max);
        let norm = max_bm25.max(1e-9);
        out.sort_by(|a, b| {
            let fa = a.bm25 / norm + a.latent;
            let fb = b.bm25 / norm + b.latent;
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        });
        out.truncate(k);
        out
    }

    /// Every row (the `--dump-trajectories` backend), in key order.
    #[must_use]
    pub fn dump(&self) -> Vec<TrajRowDoc> {
        self.kv
            .scan_prefix(b"traj:")
            .into_iter()
            .filter_map(|(_, v)| serde_json::from_slice::<TrajRowDoc>(&v).ok())
            .collect()
    }

    /// Row count (diagnostics + tests).
    #[must_use]
    pub fn len(&self) -> usize {
        self.kv.scan_prefix(b"traj:").len()
    }

    /// Is the store empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// sigmoid(dot(a, b)) — the house similarity (never softmax).
fn sigmoid_dot_8(a: &[f32; EMBED_DIM], b: &[f32; EMBED_DIM]) -> f32 {
    let mut dot = 0.0f32;
    for i in 0..EMBED_DIM {
        dot += a[i] * b[i];
    }
    katgpt_core::sigmoid(dot)
}

// ── T4.2: the shape-conditional candidate ordering ───────────────────

/// The unseen-strategy belief quantile (also the `Beta(1,1)` baseline:
/// `best_belief_score(0, 0, 0.32)` = 0.32 — the plan's "unseen-strategy
/// prior 0.32").
const BELIEF_EPSILON: f32 = 0.32;

/// Shape-conditional score for one `(code, strategy, shape)` triple: the
/// ε-quantile of `Beta(1 + S, 1 + F)` where `S`/`F` are RELEVANCE-WEIGHTED
/// pseudo-counts over the retrieved same-strategy neighbors — flat
/// `(code, strategy)` counters cannot distinguish E0597-boxed-closure
/// from E0597-returns-ref; retrieved neighbors can (the plan's T4.2
/// claim — benefit measured at G6, T4.2's honest-negative clause stands).
#[must_use]
pub fn shape_conditional_score(store: &TrajStore, code: &str, strategy_id: &str, shape: &str) -> f32 {
    let neighbors = store.retrieve(shape, &format!("{code}:"), 8);
    let mut s = 0.0f32;
    let mut f = 0.0f32;
    let mut w_sum = 0.0f32;
    for n in &neighbors {
        if !n.lint_key.ends_with(&format!(":{strategy_id}")) {
            continue;
        }
        // Relevance weight: the fused score normalized by the best hit —
        // nearer shapes carry more of the outcome evidence.
        let w = n.bm25 + n.latent + 1e-6;
        w_sum += w;
        #[allow(clippy::cast_precision_loss)] // trajectory counts are ~10^2
        let (ns, nf) = (n.success_count as f32, n.fail_count as f32);
        s += w * ns;
        f += w * nf;
    }
    if w_sum == 0.0 {
        // Unseen at this shape: the Beta(1,1) quantile — neither reward
        // nor penalty (min-delta ordering decides among unseen peers).
        return katgpt_core::best_belief_score(0, 0, BELIEF_EPSILON);
    }
    // The substrate quantile takes integer counts; round (never
    // truncate — 2.7 weighted successes are closer to 3 than 2).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (si, fi) = (s.round() as u32, f.round() as u32);
    katgpt_core::best_belief_score(si, fi, BELIEF_EPSILON)
}

/// Order a driver iteration's candidates shape-conditionally: score
/// DESC (the T4.2 neighbor-weighted `BetaPosterior`), then the static
/// min-delta keys as tiebreak (file → line → total length → id — seen
/// strategies outrank unseen; unseen peers fall back to min-delta).
/// This is the closure the CLI installs via
/// [`CompileDriver::with_ordering`](crate::domains::rustc_errors::driver::CompileDriver::with_ordering).
/// Needs BOTH `latent_retrieval` (the store) and `rustc_errors` (the
/// candidate type).
#[cfg(feature = "rustc_errors")]
pub fn shape_conditional_order(
    cands: &mut [crate::domains::rustc_errors::playbooks::FixCandidate],
    store: &TrajStore,
) {
    let belief = |cand: &crate::domains::rustc_errors::playbooks::FixCandidate| {
        shape_conditional_score(store, &cand.code, &cand.strategy_id, &cand.shape)
    };
    cands.sort_by(|a, b| {
        belief(b)
            .partial_cmp(&belief(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
            .then(a.total_len().cmp(&b.total_len()))
            .then(a.strategy_id.cmp(&b.strategy_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "riir_traj_store_{}_{}_{}",
            std::process::id(),
            name,
            line!()
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn tier_u8_roundtrip() {
        for t in [
            EvidenceTier::Certified,
            EvidenceTier::Heuristic,
            EvidenceTier::Withdrawn,
        ] {
            assert_eq!(EvidenceTier::from_u8(t.as_u8()), t);
        }
        assert_eq!(EvidenceTier::from_u8(9), EvidenceTier::Heuristic);
    }

    #[test]
    fn record_then_reopen_then_retrieve() {
        let root = tmp_root("roundtrip");
        {
            let (store, bad) = TrajStore::open(&root).unwrap();
            assert_eq!(bad, 0);
            store
                .record("E0502:e0502-deref-removal", "&v[0]", "", true, true, "run-a")
                .unwrap();
            store
                .record(
                    "E0502:e0502-clone-out",
                    "&items[0]",
                    "items[0].clone()",
                    false,
                    false,
                    "run-a",
                )
                .unwrap();
            assert_eq!(store.len(), 2);
        }
        // Reopen: WAL replay carries the rows + the BM25 rebuild works.
        {
            let (store, bad) = TrajStore::open(&root).unwrap();
            assert_eq!(bad, 0, "no corrupt rows");
            assert_eq!(store.len(), 2, "durable across close/reopen");
            let hits = store.retrieve("&v[0]", "E0502:", 5);
            assert_eq!(hits.len(), 2);
            let top = &hits[0];
            assert_eq!(
                top.lint_key,
                "E0502:e0502-deref-removal",
                "exact-shape neighbor ranks first: {hits:?}"
            );
            assert_eq!(top.success_count, 1);
            assert!((top.outcome_rate() - 1.0).abs() < 1e-6);
            // Family filter excludes other codes.
            assert!(store.retrieve("&v[0]", "E0597:", 5).is_empty());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn three_consecutive_failures_withdraw_and_are_excluded() {
        let root = tmp_root("withdrawn");
        let (store, _) = TrajStore::open(&root).unwrap();
        for _ in 0..3 {
            store
                .record("E0499:x", "&mut *v", "swap", false, false, "run-f")
                .unwrap();
        }
        assert!(store.retrieve("&mut *v", "E0499:", 5).is_empty());
        assert_eq!(store.dump().len(), 1, "still dumped (failure analysis)");
        let row = &store.dump()[0];
        assert_eq!(row.tier, EvidenceTier::Withdrawn.as_u8());
        // A verified success exits Withdrawn (absorbing ONLY until one).
        store
            .record("E0499:x", "&mut *v", "swap", true, true, "run-g")
            .unwrap();
        let row = &store.dump()[0];
        assert_eq!(row.tier, EvidenceTier::Certified.as_u8());
        assert_eq!(row.consecutive_fails, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dump_carries_the_full_doc() {
        let root = tmp_root("dump");
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("E0505:e0505-clone-at-move", "v", ".clone()", true, true, "run-d")
            .unwrap();
        let dumped = store.dump();
        assert_eq!(dumped.len(), 1);
        assert_eq!(dumped[0].lint_key, "E0505:e0505-clone-at-move");
        assert_eq!(dumped[0].fix, ".clone()");
        assert_eq!(dumped[0].success_count, 1);
        assert_eq!(dumped[0].tier, EvidenceTier::Certified.as_u8());
        assert_eq!(dumped[0].direction.len(), EMBED_DIM);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn record_stamps_run_id_and_seen_at() {
        let root = tmp_root("runid");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            store
                .record("E0502:s", "&v[0]", "v[0]", true, true, "run-abc")
                .unwrap();
            let row = &store.dump()[0];
            assert_eq!(row.last_run_id, "run-abc");
            assert!(row.last_seen_at > 0, "timestamp stamped");
        }
        // Durable across reopen + updated by the next sighting.
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let before = store.dump()[0].last_seen_at;
            store
                .record("E0502:s", "&v[0]", "v[0]", true, true, "run-zzz")
                .unwrap();
            let row = &store.dump()[0];
            assert_eq!(row.last_run_id, "run-zzz", "latest sighting wins");
            assert!(row.last_seen_at >= before);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pre_t2_rows_without_verdict_fields_decode() {
        // A row written before 037-T2 (no last_run_id / last_seen_at) must
        // decode with the serde defaults — the schema is append-only.
        let doc: TrajRowDoc = serde_json::from_str(
            r#"{"lint_key":"E0502:x","shape":"&v","fix":"v","direction":[0.0,0.0],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":0}"#,
        )
        .expect("pre-T2 row decodes");
        assert_eq!(doc.last_run_id, "");
        assert_eq!(doc.last_seen_at, 0);
        assert_eq!(doc.success_count, 1);
    }

    #[cfg(feature = "rustc_errors")]
    #[test]
    fn shape_conditional_ordering_prefers_the_winning_neighbor() {
        use crate::domains::rustc_errors::playbooks::FixCandidate;
        let root = tmp_root("ordering");
        let (store, _) = TrajStore::open(&root).unwrap();
        // History: deref-removal SUCCEEDED on this exact shape 5×;
        // clone-out FAILED on it 5× (a synthetic reversed ladder — the
        // corpus ladder is the other way round, which is the point: the
        // ORDER follows lived outcomes, not the static preference).
        for _ in 0..5 {
            store
                .record("E0502:e0502-deref-removal", "&v[0]", "", true, true, "run-o")
                .unwrap();
            store
                .record("E0502:e0502-clone-out", "&v[0]", "v[0].clone()", false, false, "run-o")
                .unwrap();
        }
        let mk = |strat: &str, repl: &str, len_hint: usize| FixCandidate {
            strategy_id: strat.to_string(),
            code: "E0502".to_string(),
            file: std::path::PathBuf::from("a.rs"),
            line: 2,
            byte_span: (0, len_hint),
            replacement: repl.to_string(),
            shape: "&v[0]".to_string(),
            semantic_note: None,
            extra_edits: Vec::new(),
        };
        // Static min-delta would ALSO pick deref-removal here (shorter
        // total length) — so the flip arm below is the load-bearing
        // assertion: with the history REVERSED, the order flips, which
        // static ordering can never do.
        let mut cands = vec![
            mk("e0502-clone-out", "v[0].clone()", 6),
            mk("e0502-deref-removal", "", 1),
        ];
        shape_conditional_order(&mut cands, &store);
        assert_eq!(
            cands[0].strategy_id, "e0502-deref-removal",
            "the 5×-successful neighbor outranks the 5×-failed one"
        );
        // And the override is REAL: flip the history, the order flips.
        let flipped_root = tmp_root("ordering_flip");
        let (flipped, _) = TrajStore::open(&flipped_root).unwrap();
        for _ in 0..5 {
            flipped
                .record("E0502:e0502-deref-removal", "&v[0]", "", false, false, "run-f")
                .unwrap();
            flipped
                .record("E0502:e0502-clone-out", "&v[0]", "v[0].clone()", true, true, "run-f")
                .unwrap();
        }
        let mut flipped_cands = vec![
            mk("e0502-clone-out", "v[0].clone()", 6),
            mk("e0502-deref-removal", "", 1),
        ];
        shape_conditional_order(&mut flipped_cands, &flipped);
        assert_eq!(
            flipped_cands[0].strategy_id, "e0502-clone-out",
            "flipped history flips the order — the ordering follows outcomes"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&flipped_root);
    }
}
