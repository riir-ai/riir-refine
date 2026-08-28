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
//! learn:{lint_key}                          → JSON LearnRowDoc (037 E3)
//! corpus:{YYYYMMDD}:{domain}                → JSON CorpusCountRow (Issue 050 T2)
//! scorerun:{unix}:{sha|nosh}                → JSON ScoreRunRow    (Issue 050 T2)
//! ```
//!
//! Family scans are `scan_prefix(b"traj:")`; a trajectory's counters
//! update by read-modify-write under the full-row key (idempotent upsert).
//! The tier transitions are Issue 021's, durable: `success && verified`
//! certifies; 3 consecutive failures withdraw (absorbing — only a
//! verified success exits). The `learn:` rows are the per-rule EMA twin
//! (Issue 037 E3): every verdict also steps the rule's evolved direction
//! ([`ema_step`](crate::evolve::ema_step) — same rule as the in-process
//! layer), durable + observable via `--memory-stats`.
//!
//! The `elo`/`span_elo` row fields (Issue 039 T2) are written ONLY by the
//! batch pass [`TrajStore::recompute_elo`] (`cargo heal --recompute-elo`)
//! — ratings are a derived view over the verdict counts, never updated
//! online (the issue's non-goal; G4).
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
use crate::elo::EloTable;
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
    /// The rule's ELO rating (Issue 039 reservation; T2 computes — batch
    /// PL→Elo over the verdict history, rule-vs-shape matches). Base 1200
    /// = unrated (the honest pre-T2 state every row decodes with).
    #[serde(default = "default_elo")]
    pub elo: f32,
    /// The opponent (shape) side of the last match — span difficulty
    /// emerges as the opponent rating (039's match semantics rate BOTH
    /// sides). Same base-1200 default; denormalized per-row until T2
    /// decides whether spans get their own derivation.
    #[serde(default = "default_elo")]
    pub span_elo: f32,
    /// Component-attributed failure class (Issue 047, Recuris E/ρ/W/C
    /// ontology) — the LAST decision-time attribution recorded for this
    /// row's failures. `None` on success-only rows AND on pre-Issue-047
    /// rows (serde default keeps old rows decoding as unattributed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<crate::component::ComponentClass>,
}

/// The ELO base rating (unrated) — re-exported from the math module
/// (Issue 039 T2 moved it beside [`EloTable`]; the arena calculator + PL
/// conversion's shared convention — base 1200 / 400 per log10 unit;
/// promotion to `katgpt-core` filed as katgpt-rs Issue 686).
pub use crate::elo::default_elo;

/// One persisted per-rule EMA learn row (Issue 037 E3) — the durable twin
/// of [`LatentSkillEvolution`]'s in-memory directions. Key: `learn:{lint_key}`.
/// Every verdict event ([`TrajStore::record`]) feeds the rule's row through
/// [`ema_step`](crate::evolve::ema_step) — the same rule, same α/β, durable —
/// so "the EMA path consumes real verdicts" is answerable from the store
/// itself (per-rule counters + a nonzero `last_delta`).
///
/// Mirrors the evolve.rs initialization rule: `direction` stays empty until
/// the rule's first SUCCESS (a first fail never establishes a baseline), so
/// `updates > 0` ⟺ the direction is initialized ⟺ `last_delta > 0`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct LearnRowDoc {
    /// The rule key (the `traj:` row's `lint_key` — e.g. `uninlined_format_args`
    /// or `E0502:e0502-clone-out`).
    pub lint_key: String,
    /// The EMA-evolved direction; EMPTY until the rule's first success.
    #[serde(default)]
    pub direction: Vec<f32>,
    /// EMA steps that actually moved the vector (init + every post-init
    /// verdict) — the per-rule learn counter.
    #[serde(default)]
    pub updates: u32,
    /// Verdict outcomes consumed (raw counters — includes pre-init fails,
    /// which are direction no-ops but still verdicts).
    #[serde(default)]
    pub successes: u32,
    #[serde(default)]
    pub fails: u32,
    /// L2 magnitude of the last applied step — the "direction deltas ≠ 0"
    /// evidence (E3's gate text). 0.0 only while the row is pre-init.
    #[serde(default)]
    pub last_delta: f32,
    /// The heal-run id of the latest verdict (037-T2 attribution).
    #[serde(default)]
    pub last_run_id: String,
    #[serde(default)]
    pub last_seen_at: u64,
}

/// One durable corpus-size row (Issue 050 T2) — the Warm-tier twin of the
/// dashboard's `.benchmarks/corpus_history.jsonl` projection. Key:
/// `corpus:{YYYYMMDD}:{domain}`. One row per (day, domain); the per-day
/// FLAT shape the dashboard consumes is derived by
/// [`TrajStore::corpus_series_by_day`]. A typed doc row (not a Pod) — same
/// pattern as every row here (the `source_note` half is variable-length
/// text; the dep-light JSON-row rule from the module header applies).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct CorpusCountRow {
    /// Calendar day, `YYYY-MM-DD` (UTC — the same convention as
    /// [`today_local_date`]).
    pub day: String,
    /// The corpus domain (`kernel_opt` / `rust_perf` / `clippy_lints` / `sec`).
    pub domain: String,
    /// Marker occurrences counted by the source walk (`rule_id:` /
    /// `lint_name:`).
    pub count: u32,
    /// Whether the domain's feature was compiled into the binary that
    /// stamped the row (`cfg!(feature = ...)` — text counts and compiled
    /// liveness are different facts).
    pub compiled: bool,
    /// Where the row came from: `"source-walk"` (the `--corpus-stats`
    /// stamp) or `"import-jsonl"` (the one-time jsonl import).
    pub source_note: String,
}

/// One durable score-run row (Issue 050 T2) — the Warm-tier twin of a
/// `.benchmarks/score_history.jsonl` line. The first eleven fields are the
/// `score_bench::report::HistoryRecord` shape VERBATIM (schema-identity
/// jsonl ↔ store is pinned by a test); `recorded_unix` is the ordering key
/// derived from `ts` (0 when the timestamp does not parse). Key:
/// `scorerun:{recorded_unix}:{sha-or-"nosh"}` — upsert semantics, so a
/// re-import or a dual-write of the same run is idempotent by construction.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ScoreRunRow {
    /// ISO-8601 timestamp of the run (the `HistoryRecord` `ts` verbatim;
    /// schema-identical with the jsonl line by test).
    pub ts: String,
    /// Short git sha of the healer build (empty → key part `nosh`).
    pub sha: String,
    pub n_ran: usize,
    pub n_healable: usize,
    pub n_clean: usize,
    pub heal_rate: f32,
    pub created_rate: f32,
    pub parse_safety: f32,
    pub decline_correctness: f32,
    pub edits_total: usize,
    pub vocab_size: usize,
    /// Unix seconds of `ts` (the store-key ordering half; 0 = unparseable).
    #[serde(default)]
    pub recorded_unix: u64,
}

#[cfg(feature = "score_bench")]
impl From<&crate::score_bench::report::HistoryRecord> for ScoreRunRow {
    fn from(r: &crate::score_bench::report::HistoryRecord) -> Self {
        Self {
            ts: r.ts.clone(),
            sha: r.sha.clone(),
            n_ran: r.n_ran,
            n_healable: r.n_healable,
            n_clean: r.n_clean,
            heal_rate: r.heal_rate,
            created_rate: r.created_rate,
            parse_safety: r.parse_safety,
            decline_correctness: r.decline_correctness,
            edits_total: r.edits_total,
            vocab_size: r.vocab_size,
            recorded_unix: parse_ts_unix(&r.ts),
        }
    }
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

/// blake3 of `shape`, first 12 bytes hex (the row-key identity). Public since
/// Plan 027 (T0.3): the KAT sync payload's `shape_id` derives from THIS fn —
/// one derivation, so a payload row is addressable by the store's own row
/// identity (no second hash to drift).
#[must_use]
pub fn shape_hash(shape: &str) -> String {
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

/// Today's date as `YYYY-MM-DD` (the corpus-stamp day key, Issue 050 T2).
/// UTC-based — the crate's one date convention (deterministic across
/// machines); may trail a wall-clock-local `date.today()` by the
/// UTC-to-local offset within the first hours of a day.
#[must_use]
pub fn today_local_date() -> String {
    let day = i64::try_from(now_unix_secs() / 86_400).unwrap_or(0);
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Parse `YYYY-MM-DD[THH:MM:SS]` (the score-history `ts` shape) to unix
/// seconds — the `ScoreRunRow::recorded_unix` derivation. `0` on any
/// malformed input (the honest fallback; the row still stores, it just
/// sorts first).
// The single-char names are y/m/d/h/min/s — deliberate, mirroring the
// crate's `local_iso_timestamp` convention.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn parse_ts_unix(ts: &str) -> u64 {
    let b = ts.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return 0;
    }
    let y: i64 = ts.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    let m: u32 = ts.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0);
    let d: u32 = ts.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(0);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return 0;
    }
    let (h, mi, s): (i64, i64, i64) = if b.len() >= 19 && b[10] == b'T' {
        (
            ts.get(11..13).and_then(|v| v.parse().ok()).unwrap_or(0),
            ts.get(14..16).and_then(|v| v.parse().ok()).unwrap_or(0),
            ts.get(17..19).and_then(|v| v.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };
    let total = days_from_civil(y, m, d) * 86_400 + h * 3600 + mi * 60 + s;
    u64::try_from(total).unwrap_or(0)
}

/// Inverse of [`civil_from_days`] — Howard Hinnant's `days_from_civil`
/// (the score-run timestamp parser's date half; same bounded-range
/// argument as the forward direction).
#[must_use]
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (i64::from(m) + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
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
    /// (037-T2). Unattributed (Issue 047 component = `None`) — use
    /// [`record_classified`](Self::record_classified) for failures whose
    /// decision-time signal is known.
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
        self.record_classified(lint_key, shape, fix, success, verified, run_id, None)
    }

    /// [`record`](Self::record) with the Issue 047 component attribution:
    /// `Some(class)` stamps the row's failure class (the LAST decision-time
    /// attribution — successes leave a previous tag in place; the tag is
    /// failure metadata for the audit, not a state machine).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    #[allow(clippy::too_many_arguments)] // mirrors `record`'s call shape + the one attribution param
    pub fn record_classified(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        success: bool,
        verified: bool,
        run_id: &str,
        component: Option<crate::component::ComponentClass>,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self.load_or_new_row(&key, lint_key, shape, fix);
        // The text halves update with the latest sighting (the shape key
        // is stable, the fix may have been refined).
        row.fix = fix.to_string();
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        if let Some(class) = component {
            row.component = Some(class);
        }
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
        self.put_row(&key, &row)?;
        // The EMA half (Issue 037 E3): the same verdict also feeds the
        // rule's durable learn row — one event, two durable effects
        // (trajectory counters for retrieval, direction evolution for
        // learn observability).
        self.learn_step(lint_key, fix, success, run_id)
    }

    /// Upsert one APPLY event (038-T2: the `record_applied` equivalent —
    /// memory-only in envelope terms). Creates/refreshes the row's text
    /// halves + run attribution WITHOUT bumping any outcome counter: an
    /// apply is a sighting, not a verdict — the tier stays whatever it was
    /// (Heuristic on first sight) until a verify run records an outcome.
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn record_applied(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        run_id: &str,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self.load_or_new_row(&key, lint_key, shape, fix);
        row.fix = fix.to_string();
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        self.put_row(&key, &row)
    }

    /// Import one FULL row verbatim (038-T3: the one-time envelope → store
    /// migration). Unlike [`record`](Self::record)/[`record_applied`](Self::record_applied)
    /// this writes the caller's counters + tier as-is — the envelope's
    /// accumulated stats are the ground truth being migrated, not events to
    /// replay. Keyed by the row's own (`lint_key`, shape).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn import_row(&self, row: &TrajRowDoc) -> Result<(), String> {
        let key = Self::row_key(&row.lint_key, &row.shape);
        self.put_row(&key, row)
    }

    /// Load the row at `key`, or create a fresh Heuristic one for
    /// `(lint_key, shape, fix)` (the shared load-or-create of
    /// [`record`](Self::record)/[`record_applied`](Self::record_applied)).
    fn load_or_new_row(
        &self,
        key: &[u8],
        lint_key: &str,
        shape: &str,
        fix: &str,
    ) -> TrajRowDoc {
        self.kv
            .get_bytes(key)
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
                    elo: default_elo(),
                    span_elo: default_elo(),
                    component: None,
                }
            })
    }

    /// Encode + put one row + keep the BM25 axis live (the shared tail of
    /// every row-writing method).
    fn put_row(&self, key: &[u8], row: &TrajRowDoc) -> Result<(), String> {
        let bytes =
            serde_json::to_vec(row).map_err(|e| format!("encode row: {e}"))?;
        self.kv.put(key, &bytes).map_err(|e| format!("put row: {e}"))?;
        self.bm25
            .index(key, &format!("{} {}", row.shape, row.fix));
        Ok(())
    }

    /// Load-or-create the rule's learn row, apply one EMA step (the shared
    /// [`ema_step`](crate::evolve::ema_step) core — same α/β as the
    /// in-process layer), persist. Called ONLY from [`record`](Self::record):
    /// the EMA consumes verdicts, not sightings — `record_applied` never
    /// touches learn rows.
    ///
    /// Initialization rule (evolve.rs): the direction is established by the
    /// rule's first SUCCESS (a first fail is a direction no-op — it never
    /// establishes what a good fix looks like), with `last_delta` = the full
    /// initial vector's L2 norm (the honest "delta ≠ 0" at birth).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure (the learn row is skipped on read
    /// errors — an undecodable row restarts fresh rather than failing the
    /// verdict that feeds it).
    fn learn_step(&self, lint_key: &str, fix: &str, success: bool, run_id: &str) -> Result<(), String> {
        let key = format!("learn:{lint_key}").into_bytes();
        let mut row: LearnRowDoc = self
            .kv
            .get_bytes(&key)
            .and_then(|v| serde_json::from_slice::<LearnRowDoc>(&v).ok())
            .unwrap_or_else(|| LearnRowDoc {
                lint_key: lint_key.to_string(),
                direction: Vec::new(),
                updates: 0,
                successes: 0,
                fails: 0,
                last_delta: 0.0,
                last_run_id: String::new(),
                last_seen_at: 0,
            });
        let mut fix_dir = [0.0f32; EMBED_DIM];
        embed_into(lint_key, fix, &mut fix_dir);
        if row.direction.len() < EMBED_DIM {
            // Pre-init: only a success establishes the baseline.
            if success {
                row.last_delta = l2_norm(&fix_dir);
                row.direction = fix_dir.to_vec();
                row.updates = row.updates.saturating_add(1);
            }
        } else {
            let mut dir = [0.0f32; EMBED_DIM];
            for (i, x) in row.direction.iter().take(EMBED_DIM).enumerate() {
                dir[i] = *x;
            }
            row.last_delta =
                crate::evolve::ema_step(&mut dir, &fix_dir, success, &crate::evolve::EvolveConfig::default());
            row.direction = dir.to_vec();
            row.updates = row.updates.saturating_add(1);
        }
        if success {
            row.successes = row.successes.saturating_add(1);
        } else {
            row.fails = row.fails.saturating_add(1);
        }
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        let bytes =
            serde_json::to_vec(&row).map_err(|e| format!("encode learn row: {e}"))?;
        self.kv.put(&key, &bytes).map_err(|e| format!("put learn row: {e}"))
    }

    /// Every persisted EMA learn row (the `--memory-stats` learn section +
    /// tests), in key order. Learn rows live under `learn:` — invisible to
    /// [`len`](Self::len)/[`dump`](Self::dump)/[`retrieve`](Self::retrieve)
    /// by construction (they scan `traj:`).
    #[must_use]
    pub fn learn_rows(&self) -> Vec<LearnRowDoc> {
        self.kv
            .scan_prefix(b"learn:")
            .into_iter()
            .filter_map(|(_, v)| serde_json::from_slice::<LearnRowDoc>(&v).ok())
            .collect()
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

    /// The store-side twin of `LatentFixMemory::stats_report` — full
    /// feature parity for the `--memory-stats` kv branch (rows, distinct
    /// rules, tier breakdown, outcomes, top-8 histogram, loop state) plus
    /// the cadence line the envelope could never answer (F3: it carried no
    /// timestamps; store rows carry `last_seen_at` since 037-T2) and the
    /// EMA learn section (E3: per-rule learn counters + the decay line —
    /// persisted state only this backend can show; the envelope's EMA half
    /// is the in-process evolution layer, Bench 024 G7's evidence).
    ///
    /// Migrated rows (038-T3) carry `last_seen_at = 0` and are excluded
    /// from the day buckets by construction — the cadence line counts
    /// post-migration verdict activity only.
    #[must_use]
    pub fn stats_report(&self, bad_rows: usize) -> String {
        use std::collections::{BTreeSet, HashMap};
        use std::fmt::Write as _;

        let rows = self.dump();
        let (mut certified, mut heuristic, mut withdrawn, mut successes, mut fails, mut consec) =
            (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
        let mut rules: HashMap<&str, u32> = HashMap::new();
        let mut last_seen = 0u64;
        let mut days: BTreeSet<u64> = BTreeSet::new();
        for r in &rows {
            match EvidenceTier::from_u8(r.tier) {
                EvidenceTier::Certified => certified += 1,
                EvidenceTier::Withdrawn => withdrawn += 1,
                EvidenceTier::Heuristic => heuristic += 1,
            }
            successes += r.success_count;
            fails += r.fail_count;
            if r.consecutive_fails > 0 {
                consec += 1;
            }
            *rules.entry(r.lint_key.as_str()).or_insert(0) += 1;
            if r.last_seen_at > 0 {
                days.insert(r.last_seen_at / 86_400);
                if r.last_seen_at > last_seen {
                    last_seen = r.last_seen_at;
                }
            }
        }
        let mut report = String::with_capacity(512);
        let corrupt = if bad_rows > 0 {
            format!(" ({bad_rows} corrupt skipped)")
        } else {
            String::new()
        };
        let _ = writeln!(
            report,
            "kv store: {}{}\nrows: {} (distinct rules: {})\ntiers: Certified {certified} | Heuristic {heuristic} | Withdrawn {withdrawn}\noutcomes: success {successes} | fail {fails} | rows with consecutive fails {consec}",
            self.root.display(),
            corrupt,
            rows.len(),
            rules.len(),
        );
        let mut top: Vec<(&str, u32)> = rules.into_iter().collect();
        top.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        for (rule, count) in top.iter().take(8) {
            let _ = writeln!(report, "  {count:6}  {rule}");
        }
        if last_seen > 0 {
            let day = i64::try_from(last_seen / 86_400).unwrap_or(0);
            let (y, m, d) = civil_from_days(day);
            let _ = writeln!(
                report,
                "activity: last verdict {y:04}-{m:02}-{d:02} | distinct active days: {}",
                days.len()
            );
        } else {
            let _ = writeln!(
                report,
                "activity: no verdict timestamps yet (apply-only or migrated rows)"
            );
        }
        // ── Issue 037 E3: the EMA learn section (see write_learn_section).
        write_learn_section(&mut report, &self.learn_rows());
        let _ = writeln!(
            report,
            "{}",
            crate::memory::loop_state_line(!rows.is_empty(), successes, certified)
        );
        report
    }

    /// Issue 039 T2 — the batch rating pass: replay every trajectory row's
    /// verdict counts as rule-vs-shape matches from base 1200 (canonical
    /// order: rows in store key order, losses-then-wins within each row —
    /// see [`crate::elo`]) and write the FINAL ratings back denormalized on
    /// every row of each player. Deterministic + idempotent: a second pass
    /// re-derives bit-identical ratings and writes 0 rows.
    ///
    /// The ONLY rating writer (the issue's non-goal: no online ELO updates
    /// in the hot fix loop — [`record`](Self::record) never touches these
    /// fields; G4's heal-path promise).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure on a changed row.
    #[allow(clippy::float_cmp)] // bit-identity IS the idempotency gate (deterministic replay)
    pub fn recompute_elo(&self) -> Result<EloRecomputeSummary, String> {
        let rows = self.dump();
        let mut summary = EloRecomputeSummary {
            rows_seen: rows.len(),
            ..EloRecomputeSummary::default()
        };
        let mut table = EloTable::new();
        let mut rules = std::collections::BTreeSet::new();
        let mut spans = std::collections::BTreeSet::new();
        for row in &rows {
            if row.success_count.saturating_add(row.fail_count) == 0 {
                continue; // apply-only sighting: not a match
            }
            let span_id = shape_hash(&row.shape);
            rules.insert(row.lint_key.clone());
            spans.insert(span_id.clone());
            for _ in 0..row.fail_count {
                table.play(&row.lint_key, &span_id, false);
                summary.matches += 1;
            }
            for _ in 0..row.success_count {
                table.play(&row.lint_key, &span_id, true);
                summary.matches += 1;
            }
        }
        summary.rules_rated = rules.len();
        summary.spans_rated = spans.len();
        for row in &rows {
            let span_id = shape_hash(&row.shape);
            let (elo, span_elo) = (table.rating(&row.lint_key), table.rating(&span_id));
            if row.elo == elo && row.span_elo == span_elo {
                continue; // unchanged (or apply-only, still at base)
            }
            let mut updated = row.clone();
            updated.elo = elo;
            updated.span_elo = span_elo;
            let key = Self::row_key(&row.lint_key, &row.shape);
            self.put_row(&key, &updated)?;
            summary.rows_written += 1;
        }
        Ok(summary)
    }

    /// Issue 039 T4 — the store→table bridge T3 deferred to its caller:
    /// load the PERSISTED ratings into an [`EloTable`] without replaying
    /// any match (batch-on-load design — this returns the table the last
    /// [`recompute_elo`](Self::recompute_elo) pass produced, as stored;
    /// verdicts recorded since surface at the next recompute).
    ///
    /// Loading rule: recompute writes each player's FINAL rating on EVERY
    /// row, so any row carries it — but a row CREATED after the last pass
    /// still sits at [`default_elo`] (a sighting, not a rating event). The
    /// bridge therefore takes a player's non-base rating (last one in key
    /// order on a hand-edited tie) and leaves all-base players OUT of the
    /// table — [`EloTable::rating`] answers base for them. A rule whose
    /// true rating is exactly base degrades to unrated: conservative for a
    /// tie-break axis, documented.
    ///
    /// Both namespaces load (rule `lint_key`s AND span `shape_hash`es —
    /// span difficulty is the ruliology payoff). Tier is NOT filtered
    /// here: the persisted ratings include every match recompute replayed
    /// (Withdrawn rows' too); Issue 021's suggestion-trust exclusion is
    /// the consumer's tier axis, and the audit leaderboard's Withdrawn
    /// exclusion is a render filter in `self_evolve_audit.sh` §1b (the
    /// documented split).
    #[must_use]
    #[allow(clippy::float_cmp)] // base-1200.0 is exactly representable + serde round-trips it
    pub fn elo_table(&self) -> EloTable {
        let base = default_elo();
        let mut players: HashMap<String, f32> = HashMap::new();
        for row in self.dump() {
            if row.elo != base {
                players.insert(row.lint_key.clone(), row.elo);
            }
            if row.span_elo != base {
                players.insert(shape_hash(&row.shape), row.span_elo);
            }
        }
        EloTable::from_ratings(players)
    }

    // ── Issue 050 T2: the corpus/score series families ────────────────

    /// Full row key: `corpus:{YYYYMMDD}:{domain}` (the dashed day compacted;
    /// domain last so a prefix scan by day is a single `scan_prefix`).
    fn corpus_key(day: &str, domain: &str) -> Vec<u8> {
        format!("corpus:{}:{}", day.replace('-', ""), domain).into_bytes()
    }

    /// Full row key: `scorerun:{recorded_unix}:{sha-or-nosh}`.
    fn score_key(rec: &ScoreRunRow) -> Vec<u8> {
        let sha = if rec.sha.is_empty() { "nosh" } else { rec.sha.as_str() };
        format!("scorerun:{}:{}", rec.recorded_unix, sha).into_bytes()
    }

    /// Encode + put one row IFF the stored bytes differ (the import path's
    /// idempotence counter: `false` = unchanged, nothing written).
    fn put_row_if_changed(&self, key: &[u8], row: &impl serde::Serialize) -> Result<bool, String> {
        let bytes = serde_json::to_vec(row).map_err(|e| format!("encode row: {e}"))?;
        if self.kv.get_bytes(key).is_some_and(|old| old.as_ref() == bytes.as_slice()) {
            return Ok(false);
        }
        self.kv.put(key, &bytes).map_err(|e| format!("put row: {e}"))?;
        Ok(true)
    }

    /// Upsert today's (or any day's) corpus count for one domain — the
    /// `--corpus-stats` stamp. Idempotent overwrite by construction: the
    /// same inputs produce the same bytes at the same key.
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn upsert_corpus_count(
        &self,
        day: &str,
        domain: &str,
        count: u32,
        compiled: bool,
        source_note: &str,
    ) -> Result<(), String> {
        let row = CorpusCountRow {
            day: day.to_string(),
            domain: domain.to_string(),
            count,
            compiled,
            source_note: source_note.to_string(),
        };
        let bytes = serde_json::to_vec(&row).map_err(|e| format!("encode corpus row: {e}"))?;
        self.kv
            .put(&Self::corpus_key(day, domain), &bytes)
            .map_err(|e| format!("put corpus row: {e}"))
    }

    /// Every corpus row, sorted by (day, domain) — the raw series.
    ///
    /// # Errors
    ///
    /// Row decode failure (a corrupt row is a real error here — unlike the
    /// trajectory families, this series is small and should never rot;
    /// silence would hide a broken stamp).
    pub fn corpus_series(&self) -> Result<Vec<CorpusCountRow>, String> {
        let mut rows: Vec<CorpusCountRow> = Vec::new();
        for (_, v) in self.kv.scan_prefix(b"corpus:") {
            let row: CorpusCountRow =
                serde_json::from_slice(&v).map_err(|e| format!("decode corpus row: {e}"))?;
            rows.push(row);
        }
        rows.sort_by(|a, b| a.day.cmp(&b.day).then(a.domain.cmp(&b.domain)));
        Ok(rows)
    }

    /// The dashboard-facing shape: one FLAT `{"ts": day, "<domain>": n}`
    /// object per day, oldest-first. Domains with no row that day are
    /// OMITTED — never a synthesized 0 (an absent stamp is "not measured",
    /// not "measured zero"). Identical to the
    /// `.benchmarks/corpus_history.jsonl` row shape by contract.
    ///
    /// # Errors
    ///
    /// Row decode failure (see [`corpus_series`](Self::corpus_series)).
    pub fn corpus_series_by_day(&self) -> Result<Vec<serde_json::Value>, String> {
        // BTreeMaps on both levels — the emission order is deterministic
        // regardless of scan order.
        let rows = self.corpus_series()?;
        let mut by_day: std::collections::BTreeMap<&str, std::collections::BTreeMap<&str, u32>> =
            std::collections::BTreeMap::new();
        for row in &rows {
            by_day
                .entry(row.day.as_str())
                .or_default()
                .insert(row.domain.as_str(), row.count);
        }
        Ok(by_day
            .into_iter()
            .map(|(day, domains)| {
                let mut obj = serde_json::Map::new();
                obj.insert("ts".into(), serde_json::Value::String(day.to_string()));
                for (domain, count) in domains {
                    obj.insert(domain.to_string(), serde_json::json!(count));
                }
                serde_json::Value::Object(obj)
            })
            .collect())
    }

    /// Record one finished score-bench run (the dual-write tail of the
    /// jsonl append — Issue 050 T2). Upsert at
    /// `scorerun:{recorded_unix}:{sha|nosh}`: the same run re-recorded is
    /// idempotent, a genuinely new run gets a new key.
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn record_score_run(&self, rec: &ScoreRunRow) -> Result<(), String> {
        let bytes = serde_json::to_vec(rec).map_err(|e| format!("encode score row: {e}"))?;
        self.kv
            .put(&Self::score_key(rec), &bytes)
            .map_err(|e| format!("put score row: {e}"))
    }

    /// Every score-run row, sorted by (`recorded_unix`, key) — the
    /// store-side score history (the jsonl's queryable twin).
    ///
    /// # Errors
    ///
    /// Row decode failure (see [`corpus_series`](Self::corpus_series)).
    pub fn score_runs(&self) -> Result<Vec<ScoreRunRow>, String> {
        let mut rows: Vec<(Vec<u8>, ScoreRunRow)> = Vec::new();
        for (k, v) in self.kv.scan_prefix(b"scorerun:") {
            let row: ScoreRunRow =
                serde_json::from_slice(&v).map_err(|e| format!("decode score row: {e}"))?;
            rows.push((k, row));
        }
        rows.sort_by(|a, b| {
            a.1.recorded_unix
                .cmp(&b.1.recorded_unix)
                .then(a.0.cmp(&b.0))
        });
        Ok(rows.into_iter().map(|(_, r)| r).collect())
    }

    /// One-time (idempotent) import of the committed
    /// `.benchmarks/corpus_history.jsonl` projection: each flat
    /// `{"ts": day, "<domain>": n}` line becomes one [`CorpusCountRow`]
    /// per domain with `source_note = "import-jsonl"` and
    /// `compiled = false` (the flat shape carries no compiled fact — an
    /// imported row honestly records "not verified"). Unchanged rows are
    /// not rewritten, and an existing `source-walk` row is NEVER clobbered
    /// (the first-hand stamp outranks the second-hand projection — the
    /// store is the source of truth, so re-importing a stale jsonl cannot
    /// overwrite today's live stamp).
    ///
    /// Returns `(rows_seen, rows_written)`.
    ///
    /// # Errors
    ///
    /// File read / KV put / JSON encode failure. A malformed LINE is
    /// skipped (one stderr line), never fatal.
    pub fn import_corpus_jsonl(&self, path: &Path) -> Result<(usize, usize), String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let (mut seen, mut written) = (0usize, 0usize);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
                eprintln!(
                    "traj-store: skipping malformed corpus-history line in {}",
                    path.display()
                );
                continue;
            };
            let Some(obj) = row.as_object() else {
                continue;
            };
            let Some(day) = obj.get("ts").and_then(|v| v.as_str()) else {
                continue;
            };
            for (domain, val) in obj {
                if domain == "ts" {
                    continue;
                }
                let Some(count) = val.as_u64().and_then(|c| u32::try_from(c).ok()) else {
                    eprintln!(
                        "traj-store: skipping non-numeric corpus count ({domain}) in {}",
                        path.display()
                    );
                    continue;
                };
                seen += 1;
                let row = CorpusCountRow {
                    day: day.to_string(),
                    domain: domain.clone(),
                    count,
                    compiled: false,
                    source_note: "import-jsonl".to_string(),
                };
                let key = Self::corpus_key(day, domain);
                // Provenance precedence: a first-hand source-walk stamp
                // outranks the second-hand projection — yield, count as
                // seen only. Same-provenance re-imports compare-and-put.
                let yields = self
                    .kv
                    .get_bytes(&key)
                    .and_then(|old| serde_json::from_slice::<CorpusCountRow>(&old).ok())
                    .is_some_and(|old| old.source_note == "source-walk");
                if yields {
                    continue;
                }
                if self.put_row_if_changed(&key, &row)? {
                    written += 1;
                }
            }
        }
        Ok((seen, written))
    }

    /// One-time (idempotent) import of the committed
    /// `.benchmarks/score_history.jsonl`: each `HistoryRecord` line becomes
    /// a [`ScoreRunRow`] with `recorded_unix` derived from `ts` (0 on a
    /// malformed timestamp). Unchanged rows are not rewritten.
    ///
    /// Returns `(rows_seen, rows_written)`.
    ///
    /// # Errors
    ///
    /// File read / KV put / JSON encode failure. A malformed LINE is
    /// skipped (one stderr line), never fatal.
    pub fn import_score_jsonl(&self, path: &Path) -> Result<(usize, usize), String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let (mut seen, mut written) = (0usize, 0usize);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(mut row) = serde_json::from_str::<ScoreRunRow>(line) else {
                eprintln!(
                    "traj-store: skipping malformed score-history line in {}",
                    path.display()
                );
                continue;
            };
            seen += 1;
            // Derive the ordering key from the row's own ts — never trust a
            // stale stored value (and the jsonl shape carries none).
            row.recorded_unix = parse_ts_unix(&row.ts);
            if self.put_row_if_changed(&Self::score_key(&row), &row)? {
                written += 1;
            }
        }
        Ok((seen, written))
    }
}

/// One [`TrajStore::recompute_elo`] pass summary (the `--recompute-elo`
/// print + tests).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EloRecomputeSummary {
    /// Rows scanned.
    pub rows_seen: usize,
    /// Rows whose stored ratings changed (an idempotent second pass: 0).
    pub rows_written: usize,
    /// Distinct rules with ≥1 outcome.
    pub rules_rated: usize,
    /// Distinct spans with ≥1 outcome.
    pub spans_rated: usize,
    /// Total matches replayed (Σ success+fail counts).
    pub matches: usize,
}

/// The `--memory-stats` EMA learn section (Issue 037 E3) — per-rule learn
/// counters + the decay line. The durable evidence the gate asks for:
/// rules with verdicts carry `updates > 0` and a nonzero last |Δ| (the
/// direction MOVED — "direction deltas ≠ 0 for buckets with outcomes").
fn write_learn_section(report: &mut String, learns: &[LearnRowDoc]) {
    use std::fmt::Write as _;
    if learns.is_empty() {
        let _ = writeln!(
            report,
            "learn (EMA): no evolved rules yet — verdict runs (--fix --verify) land per-rule learn rows"
        );
        return;
    }
    let total_updates: u32 = learns.iter().map(|l| l.updates).sum();
    let _ = writeln!(
        report,
        "learn (EMA): {} evolved rule(s) | {total_updates} update(s) consumed",
        learns.len()
    );
    let mut top_l: Vec<&LearnRowDoc> = learns.iter().collect();
    top_l.sort_unstable_by(|a, b| b.updates.cmp(&a.updates).then(a.lint_key.cmp(&b.lint_key)));
    for l in top_l.into_iter().take(8) {
        let _ = writeln!(
            report,
            "  {:6}  last |Δ| {:.4}  s{} f{}  {}",
            l.updates, l.last_delta, l.successes, l.fails, l.lint_key
        );
    }
    let cfg = crate::evolve::EvolveConfig::default();
    let updated = learns.iter().filter(|l| l.updates > 0).count();
    #[allow(clippy::cast_precision_loss)] // rule count — tens at most
    let mean_delta = if updated > 0 {
        learns.iter().map(|l| l.last_delta).sum::<f32>() / updated as f32
    } else {
        0.0
    };
    let _ = writeln!(
        report,
        "decay: α {:.3} toward / β {:.3} away | mean last |Δ| {:.4} across {updated} updated rule(s)",
        cfg.alpha, cfg.beta, mean_delta
    );
}

/// sigmoid(dot(a, b)) — the house similarity (never softmax).
fn sigmoid_dot_8(a: &[f32; EMBED_DIM], b: &[f32; EMBED_DIM]) -> f32 {
    let mut dot = 0.0f32;
    for i in 0..EMBED_DIM {
        dot += a[i] * b[i];
    }
    katgpt_core::sigmoid(dot)
}

/// L2 norm of a fixed-dim vector (the learn row's `last_delta` at birth).
fn l2_norm(v: &[f32; EMBED_DIM]) -> f32 {
    let mut sq = 0.0f32;
    for &x in v {
        sq += x * x;
    }
    sq.sqrt()
}

/// days-since-epoch → (year, month, day) — Howard Hinnant's
/// `civil_from_days` (std-only date formatting for the stats report's
/// activity line; no chrono dep for one formatted date). All-i64 core:
/// every intermediate is non-negative by the era adjustment, so integer
/// division is floor division and the two `u32` conversions are infallible
/// by the algorithm's documented ranges.
#[must_use]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let d = u32::try_from(d).unwrap_or(1);
    let m = u32::try_from(m).unwrap_or(1);
    (y + i64::from(m <= 2), m, d)
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

    // ── Issue 047: component-attributed failure rows ──────────────────

    #[test]
    fn record_classified_stamps_and_persists_the_component() {
        let root = tmp_root("component");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            store
                .record_classified(
                    "E0502:e0502-clone-out",
                    "&items[0]",
                    "items[0].clone()",
                    false,
                    false,
                    "run-c",
                    Some(crate::component::ComponentClass::Drafter),
                )
                .unwrap();
            let row = &store.dump()[0];
            assert_eq!(row.component, Some(crate::component::ComponentClass::Drafter));
            assert_eq!(row.fail_count, 1);

            // A later unattributed SUCCESS leaves the last failure
            // attribution in place (failure metadata, not a state machine).
            store
                .record("E0502:e0502-clone-out", "&items[0]", "items[0].clone()", true, true, "run-s")
                .unwrap();
            let row = &store.dump()[0];
            assert_eq!(row.component, Some(crate::component::ComponentClass::Drafter));
        }
        // Durable across reopen (the audit reads it via --dump-trajectories).
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let row = &store.dump()[0];
            assert_eq!(row.component, Some(crate::component::ComponentClass::Drafter));
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pre_047_row_json_decodes_component_as_none() {
        // Back-compat: rows serialized before Issue 047 carry no `component`
        // field — they must decode as unattributed, not fail.
        let legacy = r#"{"lint_key":"E0502:x","shape":"&v[0]","fix":"v[0]","direction":[0.0],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":1,"last_run_id":"r","last_seen_at":1,"elo":1200.0,"span_elo":1200.0}"#;
        let row: TrajRowDoc = serde_json::from_str(legacy).expect("legacy row decodes");
        assert_eq!(row.component, None, "pre-047 row must decode unattributed");
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

    #[test]
    #[allow(clippy::float_cmp)] // exact base value — serde default, never computed
    fn pre_039_rows_without_elo_fields_decode_at_base() {
        // Issue 039 T1 reservation: rows written before the elo fields exist
        // decode at base 1200 — the honest unrated state every pre-039 row
        // carries (T2 recomputes from verdict history on load).
        let doc: TrajRowDoc = serde_json::from_str(
            r#"{"lint_key":"E0502:x","shape":"&v","fix":"v","direction":[0.0,0.0],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":0,"last_run_id":"run-1","last_seen_at":100}"#,
        )
        .expect("pre-039 row decodes");
        assert_eq!(doc.elo, default_elo());
        assert_eq!(doc.span_elo, default_elo());
        assert_eq!(doc.last_seen_at, 100);
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

    /// The kv branch of `--memory-stats` must carry the SAME detail as the
    /// envelope branch (rows, distinct rules, tiers, outcomes, histogram,
    /// loop state) — the observability regression: the store path shipped
    /// 038 as a 4-counter one-liner and the histogram/tier/loop-state
    /// detail was envelope-only (surfaced live on adoption day).
    #[test]
    fn stats_report_carries_envelope_parity() {
        let root = tmp_root("stats");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let empty = store.stats_report(0);
            assert!(empty.contains("rows: 0 (distinct rules: 0)"));
            assert!(empty.contains("record — | verdict — | learn —"));

            store
                .record("uninlined_format_args", "format!(\"{}\", x)", "format!(\"{x}\")", true, true, "run-1")
                .unwrap();
            store
                .record_applied("match_bool", "match c { true => a, _ => b }", "if c { a } else { b }", "run-1")
                .unwrap();
        }
        {
            let (store, bad) = TrajStore::open(&root).unwrap();
            assert_eq!(bad, 0);
            let report = store.stats_report(0);
            assert!(report.contains("rows: 2 (distinct rules: 2)"));
            assert!(report.contains("tiers: Certified 1 | Heuristic 1 | Withdrawn 0"));
            assert!(report.contains("outcomes: success 1 | fail 0 | rows with consecutive fails 0"));
            assert!(report.contains("     1  match_bool"));
            assert!(report.contains("     1  uninlined_format_args"));
            assert!(report.contains("activity: last verdict "));
            assert!(report.contains("distinct active days: 1"));
            assert!(
                report.contains("loop state (Issue 037): record OK | verdict OK | learn OK"),
                "certified row closes all three gates: {report}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Migrated rows carry `last_seen_at = 0` — the activity line must say
    /// so instead of fabricating a 1970 date.
    #[test]
    fn stats_report_migration_rows_report_no_activity() {
        let root = tmp_root("stats-migrated");
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .import_row(&TrajRowDoc {
                lint_key: "uninlined_format_args".to_string(),
                shape: "format!(\"{}\", x)".to_string(),
                fix: "format!(\"{x}\")".to_string(),
                direction: vec![0.0; EMBED_DIM],
                success_count: 3,
                fail_count: 0,
                consecutive_fails: 0,
                tier: EvidenceTier::Heuristic.as_u8(),
                last_run_id: "migrated".to_string(),
                last_seen_at: 0,
                elo: default_elo(),
                span_elo: default_elo(),
                component: None,
            })
            .unwrap();
        let report = store.stats_report(0);
        assert!(report.contains("rows: 1 (distinct rules: 1)"));
        assert!(report.contains("outcomes: success 3"));
        assert!(
            report.contains("activity: no verdict timestamps yet (apply-only or migrated rows)"),
            "{report}"
        );
        // Successes from migrated history still close E2 (same semantics as
        // the envelope report — a positive outcome is a positive outcome).
        assert!(report.contains("record OK | verdict OK | learn —"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn civil_from_days_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29), "leap day");
        assert_eq!(civil_from_days(11_323), (2001, 1, 1), "year after a leap");
        assert_eq!(civil_from_days(20_689), (2026, 8, 24), "adoption day");
    }

    // ── Issue 037 E3: the durable EMA learn rows ─────────────────────────

    /// The E3 mechanism: every verdict steps the rule's persisted direction
    /// (nonzero `last_delta` — "direction deltas ≠ 0 for buckets with
    /// outcomes"), init-on-first-success, fail-before-init is a direction
    /// no-op (the evolve.rs rule), and applies never touch learn rows.
    #[test]
    fn record_feeds_the_per_rule_ema_learn_rows() {
        let root = tmp_root("learn");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            // First verdict is a SUCCESS → initializes the direction.
            store
                .record("uninlined_format_args", "format!(\"{}\", x)", "format!(\"{x}\")", true, true, "run-1")
                .unwrap();
            let learns = store.learn_rows();
            assert_eq!(learns.len(), 1, "one learn row per rule");
            let l = &learns[0];
            assert_eq!(l.lint_key, "uninlined_format_args");
            assert_eq!(l.updates, 1, "init counts as the first update");
            assert!(l.last_delta > 0.0, "init delta = the born vector's norm");
            assert_eq!(l.direction.len(), EMBED_DIM);
            assert_eq!(l.successes, 1);
            assert_eq!(l.fails, 0);
            assert_eq!(l.last_run_id, "run-1");

            // A second, different fix → the direction MOVES (Δ ≠ 0).
            let init_dir = l.direction.clone();
            store
                .record("uninlined_format_args", "println!(\"{}\", y)", "println!(\"{y}\")", true, true, "run-2")
                .unwrap();
            let l = &store.learn_rows()[0];
            assert_eq!(l.updates, 2);
            assert_eq!(l.successes, 2);
            assert_ne!(l.direction, init_dir, "EMA moved the persisted direction");
            assert!(l.last_delta > 0.0);

            // A post-init FAIL moves the direction away — still an update.
            store
                .record("uninlined_format_args", "write!(f, \"{}\", z)", "write!(f, \"{z}\")", false, false, "run-3")
                .unwrap();
            let l = &store.learn_rows()[0];
            assert_eq!(l.updates, 3);
            assert_eq!(l.fails, 1);
        }
        // Durable across reopen (the WAL carries learn rows too).
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let l = &store.learn_rows()[0];
            assert_eq!(l.updates, 3, "learn state survives close/reopen");
            assert_eq!(l.last_run_id, "run-3");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // exact 0.0 — set, never computed
    fn first_fail_is_a_learn_direction_noop() {
        let root = tmp_root("learn-fail-first");
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("match_bool", "match c { true => a, _ => b }", "if c { a } else { b }", false, false, "run-f")
            .unwrap();
        let l = &store.learn_rows()[0];
        assert_eq!(l.fails, 1, "the verdict is still counted");
        assert_eq!(l.updates, 0, "but never establishes a direction");
        assert_eq!(l.last_delta, 0.0);
        assert!(l.direction.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn record_applied_does_not_touch_learn_rows() {
        // The EMA consumes verdicts, not sightings — an apply-only run
        // leaves the learn section empty.
        let root = tmp_root("learn-apply-only");
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record_applied("match_bool", "match c { true => a, _ => b }", "if c { a } else { b }", "run-a")
            .unwrap();
        assert!(store.learn_rows().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn learn_rows_do_not_pollute_the_trajectory_surface() {
        let root = tmp_root("learn-isolated");
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("E0502:e0502-deref-removal", "&v[0]", "", true, true, "run-1")
            .unwrap();
        assert_eq!(store.len(), 1, "learn rows are not trajectory rows");
        assert_eq!(store.dump().len(), 1);
        assert_eq!(store.learn_rows().len(), 1);
        assert_eq!(store.retrieve("&v[0]", "E0502:", 5).len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The `--memory-stats` learn section: per-rule counters + the decay
    /// line, with a graceful empty state (Issue 037 E3's observability).
    #[test]
    fn stats_report_prints_the_learn_section_and_decay_line() {
        let root = tmp_root("learn-stats");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let empty = store.stats_report(0);
            assert!(
                empty.contains("learn (EMA): no evolved rules yet"),
                "empty learn section: {empty}"
            );

            store
                .record("uninlined_format_args", "format!(\"{}\", x)", "format!(\"{x}\")", true, true, "run-1")
                .unwrap();
            store
                .record("match_bool", "match c { true => a, _ => b }", "if c { a } else { b }", true, true, "run-1")
                .unwrap();
        }
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let report = store.stats_report(0);
            assert!(
                report.contains("learn (EMA): 2 evolved rule(s) | 2 update(s) consumed"),
                "{report}"
            );
            assert!(report.contains("last |Δ| "), "per-rule delta column: {report}");
            assert!(report.contains("uninlined_format_args"));
            assert!(report.contains("match_bool"));
            assert!(
                report.contains("decay: α 0.100 toward / β 0.050 away | mean last |Δ| "),
                "decay line: {report}"
            );
            assert!(report.contains("across 2 updated rule(s)"));
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    // ── Issue 039 T2: batch ELO over the store ──────────────────────────────

    #[test]
    #[allow(clippy::float_cmp)] // base equality + shared-span bit-identity ARE the assertions
    fn recompute_elo_backfills_and_is_idempotent() {
        let root = tmp_root("elo-backfill");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            store.record("rule_a", "shape 1", "fix", true, true, "run-1").unwrap();
            for _ in 0..3 {
                store.record("rule_a", "shape 1", "fix", false, true, "run-1").unwrap();
            }
            store.record("rule_b", "shape 1", "fix2", true, true, "run-1").unwrap();
            // Apply-only sighting: no outcomes, stays unrated.
            store.record_applied("rule_c", "shape 3", "fix3", "run-1").unwrap();

            let s1 = store.recompute_elo().unwrap();
            assert_eq!(s1.rows_seen, 3);
            assert_eq!(s1.matches, 5);
            assert_eq!(s1.rules_rated, 2);
            assert_eq!(s1.spans_rated, 1);
            assert_eq!(s1.rows_written, 2, "only the two outcome rows");

            let dump = store.dump();
            let a = dump.iter().find(|r| r.lint_key == "rule_a").unwrap();
            let b = dump.iter().find(|r| r.lint_key == "rule_b").unwrap();
            let c = dump.iter().find(|r| r.lint_key == "rule_c").unwrap();
            // rule_b (1W 0L) above base; rule_a (1W 3L) below; both share
            // the span's FINAL rating (denormalized snapshot semantics).
            assert!(b.elo > default_elo(), "1W/0L rule rated up: {}", b.elo);
            assert!(a.elo < default_elo(), "1W/3L rule rated down: {}", a.elo);
            assert_eq!(a.span_elo, b.span_elo, "shared span carries one rating");
            // The span's record: 3 wins (rule_a's failures) vs 2 losses
            // (rule_a + rule_b successes) — a net-winning span rates UP.
            assert!(a.span_elo > default_elo(), "3W/2L span rated up: {}", a.span_elo);
            // Apply-only row stays at base (no matches).
            assert_eq!(c.elo, default_elo());
            assert_eq!(c.span_elo, default_elo());

            // Idempotent: bit-identical re-derivation writes nothing.
            let before = store.dump();
            let s2 = store.recompute_elo().unwrap();
            assert_eq!(s2.rows_written, 0);
            assert_eq!(store.dump(), before);
        }
        // Durable: ratings survive close/reopen (the backfill's point).
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            let dump = store.dump();
            let a = dump.iter().find(|r| r.lint_key == "rule_a").unwrap();
            assert!(a.elo < default_elo(), "rating persisted: {}", a.elo);
            let s = store.recompute_elo().unwrap();
            assert_eq!(s.rows_written, 0, "still idempotent after reopen");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // denormalized final-rating bit-identity IS the assertion
    fn shared_rule_rows_carry_the_final_rating() {
        // One rule over two shapes: EVERY row of the rule reports the
        // rule's FINAL rating (the consumer contract — T3/T4 read any row).
        let root = tmp_root("elo-final");
        let (store, _) = TrajStore::open(&root).unwrap();
        store.record("rule_r", "shape x", "f", true, true, "run-1").unwrap();
        store.record("rule_r", "shape x", "f", true, true, "run-1").unwrap();
        store.record("rule_r", "shape y", "f", false, true, "run-1").unwrap();
        store.recompute_elo().unwrap();
        let dump = store.dump();
        let x = dump.iter().find(|r| r.shape == "shape x").unwrap();
        let y = dump.iter().find(|r| r.shape == "shape y").unwrap();
        assert_eq!(x.elo, y.elo, "denormalized FINAL rating on every row");
        assert_ne!(x.span_elo, y.span_elo, "spans rate separately");
        // x lost to the rule twice (0W/2L → down); y beat it once (1W/0L
        // → up) — a span's wins come from the rule's failures.
        assert!(x.span_elo < default_elo(), "twice-beaten span rated down");
        assert!(y.span_elo > default_elo(), "span that beat the rule rated up");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // unrated == base IS the pre-backfill pin
    fn pre_039_row_decodes_unrated_then_backfill_rates_it() {
        // Extends the decode pin above: the SAME pre-039 JSON (no elo
        // fields), once in the store, gets ratings WRITTEN by the T2
        // backfill — decode-unrated is the input state, rated the
        // post-backfill state.
        let root = tmp_root("elo-pre039");
        let (store, _) = TrajStore::open(&root).unwrap();
        let pre039 = r#"{"lint_key":"match_bool","shape":"match c {\n    true => a,\n}","fix":"if c { a }","direction":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":2,"fail_count":0,"consecutive_fails":0,"tier":0,"last_run_id":"run-1","last_seen_at":100}"#;
        let shape = "match c {\n    true => a,\n}";
        let key = TrajStore::row_key("match_bool", shape);
        store.kv.put(&key, pre039.as_bytes()).unwrap();
        // Decodes unrated (the pin above) ...
        let row = &store.dump()[0];
        assert_eq!(row.elo, default_elo());
        // ... then the backfill rates it.
        let s = store.recompute_elo().unwrap();
        assert_eq!(s.rows_written, 1);
        let row = &store.dump()[0];
        assert_eq!(row.success_count, 2);
        assert!(row.elo > default_elo(), "2W/0L rated up: {}", row.elo);
        assert!(row.span_elo < default_elo());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // unrated == base IS the batch-only pin
    fn record_never_touches_elo_batch_only() {
        // The non-goal pin: no online ELO updates in the hot fix loop —
        // record leaves the row unrated; ONLY the batch pass writes
        // ratings (G4's heal-path promise).
        let root = tmp_root("elo-batch-only");
        let (store, _) = TrajStore::open(&root).unwrap();
        store.record("match_bool", "match c { true => a, _ => b }", "if c { a } else { b }", true, true, "run-a").unwrap();
        let row = &store.dump()[0];
        assert_eq!(row.elo, default_elo(), "record must not write ratings");
        assert_eq!(row.span_elo, default_elo());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // bridge fidelity IS bit-identity to the persisted rating
    fn elo_table_loads_persisted_ratings_verbatim() {
        // Issue 039 T4 — the bridge contract: after a recompute pass (and a
        // reopen), the store's table answers every row's PERSISTED rating
        // for BOTH namespaces (rule lint_key + span shape_hash). No replay:
        // the load is exactly what recompute wrote.
        let root = tmp_root("elo-table-load");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            for _ in 0..3 {
                store.record("rule_a", "shape 1", "fix", true, true, "run-1").unwrap();
            }
            store.record("rule_a", "shape 2", "fix", false, true, "run-1").unwrap();
            store.record("rule_b", "shape 1", "fix2", true, true, "run-1").unwrap();
            store.recompute_elo().unwrap();
        }
        let (store, _) = TrajStore::open(&root).unwrap();
        let table = store.elo_table();
        for row in store.dump() {
            assert_eq!(
                table.rating(&row.lint_key),
                row.elo,
                "rule rating must match the persisted row value for {}",
                row.lint_key
            );
            assert_eq!(
                table.rating(&shape_hash(&row.shape)),
                row.span_elo,
                "span rating must match the persisted row value"
            );
        }
        assert!(table.contains("rule_a"));
        assert!(table.contains("rule_b"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // the mixed-value edge IS a rating comparison
    fn elo_table_prefers_rated_row_over_post_recompute_sighting() {
        // The load-bearing edge: a row CREATED after the last recompute
        // sits at base (a sighting, not a rating event) while the rule's
        // older rows carry the rated value — the bridge must answer the
        // RATED value, not the stale base.
        let root = tmp_root("elo-table-edge");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            for _ in 0..4 {
                store.record("rule_r", "shape x", "f", true, true, "run-1").unwrap();
            }
            store.recompute_elo().unwrap();
            // Post-recompute sighting on a NEW shape: fresh row at base.
            store.record_applied("rule_r", "shape new", "f", "run-2").unwrap();
        }
        let (store, _) = TrajStore::open(&root).unwrap();
        let dump = store.dump();
        let rated = dump.iter().find(|r| r.shape == "shape x").unwrap().elo;
        let fresh = dump.iter().find(|r| r.shape == "shape new").unwrap().elo;
        assert!(rated > default_elo());
        assert_eq!(fresh, default_elo(), "fresh sighting sits at base");
        assert_eq!(store.elo_table().rating("rule_r"), rated);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[allow(clippy::float_cmp)] // unrated == base IS the absent-player contract
    fn elo_table_leaves_unrated_absent_and_empty_store_empty() {
        // Review-only sightings are draws — never matches — so an
        // apply-only rule stays OUT of the table (rating() answers base),
        // and a store with no rows bridges to an empty table.
        let root = tmp_root("elo-table-unrated");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            store.record_applied("review_only", "shape 3", "fix3", "run-1").unwrap();
            let table = store.elo_table();
            assert!(table.is_empty());
            assert!(!table.contains("review_only"));
            assert_eq!(table.rating("review_only"), default_elo());
        }
        let root2 = tmp_root("elo-table-empty");
        let (store, _) = TrajStore::open(&root2).unwrap();
        assert!(store.elo_table().is_empty());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&root2);
    }

    // ── Issue 050 T2: the corpus + score series families ─────────────

    #[test]
    fn corpus_roundtrip_and_idempotent_upsert() {
        let root = tmp_root("corpus-roundtrip");
        {
            let (store, _) = TrajStore::open(&root).unwrap();
            store
                .upsert_corpus_count("2026-08-28", "kernel_opt", 393, true, "source-walk")
                .unwrap();
            store
                .upsert_corpus_count("2026-08-28", "rust_perf", 88, true, "source-walk")
                .unwrap();
            store
                .upsert_corpus_count("2026-08-27", "sec", 6, false, "import-jsonl")
                .unwrap();
            let series = store.corpus_series().unwrap();
            assert_eq!(series.len(), 3);
            let json_before = serde_json::to_string(&series).unwrap();
            // Same inputs again → same bytes at the same keys (the
            // idempotent-overwrite contract).
            store
                .upsert_corpus_count("2026-08-28", "kernel_opt", 393, true, "source-walk")
                .unwrap();
            let json_after = serde_json::to_string(&store.corpus_series().unwrap()).unwrap();
            assert_eq!(json_before, json_after, "second upsert changed nothing");
        }
        // Reopen: the series is durable + (day, domain)-ordered.
        let (store, _) = TrajStore::open(&root).unwrap();
        let series = store.corpus_series().unwrap();
        assert_eq!(series.len(), 3);
        assert_eq!((series[0].day.as_str(), series[0].domain.as_str()), ("2026-08-27", "sec"));
        assert_eq!(
            (series[1].day.as_str(), series[1].domain.as_str()),
            ("2026-08-28", "kernel_opt")
        );
        assert_eq!(
            (series[2].day.as_str(), series[2].domain.as_str()),
            ("2026-08-28", "rust_perf")
        );
        assert_eq!(series[1].count, 393);
        assert!(series[1].compiled);
        assert_eq!(series[0].source_note, "import-jsonl");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn corpus_by_day_merges_and_omits_absent_domains() {
        let root = tmp_root("corpus-by-day");
        let (store, _) = TrajStore::open(&root).unwrap();
        // Day 1: only kernel_opt measured.
        store
            .upsert_corpus_count("2026-08-27", "kernel_opt", 380, true, "source-walk")
            .unwrap();
        // Day 2: kernel_opt + rust_perf (sec absent — must NOT appear, and
        // must NOT be synthesized as 0).
        store
            .upsert_corpus_count("2026-08-28", "kernel_opt", 393, true, "source-walk")
            .unwrap();
        store
            .upsert_corpus_count("2026-08-28", "rust_perf", 88, true, "source-walk")
            .unwrap();
        let by_day = store.corpus_series_by_day().unwrap();
        assert_eq!(by_day.len(), 2, "one flat row per day");
        assert_eq!(
            by_day[0],
            serde_json::json!({"ts": "2026-08-27", "kernel_opt": 380}),
            "absent domains omitted — never a 0"
        );
        assert_eq!(
            by_day[1],
            serde_json::json!({"ts": "2026-08-28", "kernel_opt": 393, "rust_perf": 88})
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn score_run_roundtrip_and_ordering() {
        let root = tmp_root("score-run");
        let mk = |ts: &str, sha: &str, unix: u64, rate: f32| ScoreRunRow {
            ts: ts.to_string(),
            sha: sha.to_string(),
            n_ran: 34,
            n_healable: 28,
            n_clean: 6,
            heal_rate: rate,
            created_rate: 0.01,
            parse_safety: 1.0,
            decline_correctness: 1.0,
            edits_total: 60,
            vocab_size: 33,
            recorded_unix: unix,
        };
        let (store, _) = TrajStore::open(&root).unwrap();
        // Insert out of order; score_runs must return (recorded_unix, key)
        // order. The empty-sha row pins the `nosh` key part.
        store.record_score_run(&mk("2026-08-25T04:05:18", "7399ffc", 1_756_087_518, 0.96)).unwrap();
        store.record_score_run(&mk("2026-08-19T04:49:29", "", 1_755_571_769, 0.84)).unwrap();
        store.record_score_run(&mk("2026-08-28T02:09:06", "9f08442", 1_756_337_346, 0.98)).unwrap();
        let runs = store.score_runs().unwrap();
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].sha, "", "empty sha stored verbatim (key part is nosh)");
        assert!(runs[0].recorded_unix < runs[1].recorded_unix);
        assert!(runs[1].recorded_unix < runs[2].recorded_unix);
        assert!((runs[2].heal_rate - 0.98).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn jsonl_imports_are_idempotent_and_skip_malformed_lines() {
        let root = tmp_root("jsonl-import");
        let dir = root.join("benchmarks");
        std::fs::create_dir_all(&dir).unwrap();
        let corpus_jsonl = dir.join("corpus_history.jsonl");
        std::fs::write(
            &corpus_jsonl,
            concat!(
                "{\"ts\":\"2026-08-27\",\"kernel_opt\":380,\"sec\":6}\n",
                "{\"ts\":\"2026-08-28\",\"kernel_opt\":393,\"rust_perf\":88,\"clippy_lints\":40,\"sec\":6}\n",
                "NOT JSON AT ALL\n",
            ),
        )
        .unwrap();
        let score_jsonl = dir.join("score_history.jsonl");
        std::fs::write(
            &score_jsonl,
            concat!(
                "{\"ts\":\"2026-08-19T04:49:29\",\"sha\":\"72bac03\",\"n_ran\":34,\"n_healable\":28,\"n_clean\":6,\"heal_rate\":0.84615386,\"created_rate\":0.057692308,\"parse_safety\":0.9285714,\"decline_correctness\":1.0,\"edits_total\":60,\"vocab_size\":33}\n",
                "GARBAGE\n",
            ),
        )
        .unwrap();
        let (store, _) = TrajStore::open(&root).unwrap();
        // 2 + 4 domain rows seen (the malformed line skipped, not fatal).
        let (seen, written) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
        assert_eq!((seen, written), (6, 6));
        let (seen_s, written_s) = store.import_score_jsonl(&score_jsonl).unwrap();
        assert_eq!((seen_s, written_s), (1, 1));
        // Series reflects the import with the import-jsonl provenance.
        let series = store.corpus_series().unwrap();
        assert_eq!(series.len(), 6);
        assert!(series.iter().all(|r| r.source_note == "import-jsonl"));
        assert_eq!(
            store.score_runs().unwrap()[0].recorded_unix,
            parse_ts_unix("2026-08-19T04:49:29")
        );
        // Second import: upserts change nothing → 0 written (idempotence).
        let (seen2, written2) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
        assert_eq!((seen2, written2), (6, 0));
        let (seen3, written3) = store.import_score_jsonl(&score_jsonl).unwrap();
        assert_eq!((seen3, written3), (1, 0));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn corpus_import_yields_to_a_first_hand_source_walk_row() {
        // Issue 050 T2 provenance precedence: the store is the source of
        // truth, so a (day, domain) key already holding a `source-walk`
        // stamp is never clobbered by the second-hand jsonl projection —
        // even when the projection carries a DIFFERENT (stale) count.
        let root = tmp_root("corpus-import-precedence");
        let dir = root.join("benchmarks");
        std::fs::create_dir_all(&dir).unwrap();
        let corpus_jsonl = dir.join("corpus_history.jsonl");
        std::fs::write(
            &corpus_jsonl,
            "{\"ts\":\"2026-08-28\",\"kernel_opt\":393}\n",
        )
        .unwrap();
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .upsert_corpus_count("2026-08-28", "kernel_opt", 396, true, "source-walk")
            .unwrap();
        // Import sees the row but writes nothing — the stamp survives.
        let (seen, written) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
        assert_eq!((seen, written), (1, 0));
        let series = store.corpus_series().unwrap();
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].count, 396, "source-walk stamp survived the import");
        assert_eq!(series[0].source_note, "source-walk");
        // An import-jsonl row (no stamp) is still idempotently refreshed.
        store
            .upsert_corpus_count("2026-08-27", "sec", 6, false, "import-jsonl")
            .unwrap();
        std::fs::write(
            &corpus_jsonl,
            concat!(
                "{\"ts\":\"2026-08-28\",\"kernel_opt\":393}\n",
                "{\"ts\":\"2026-08-27\",\"sec\":6}\n",
            ),
        )
        .unwrap();
        let (seen2, written2) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
        assert_eq!((seen2, written2), (2, 0), "same-bytes re-import writes nothing");
        assert_eq!(store.corpus_series().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_ts_unix_and_today_roundtrip() {
        // Date-only and full-timestamp shapes both parse.
        assert_eq!(parse_ts_unix("2026-08-19"), parse_ts_unix("2026-08-19T00:00:00"));
        assert_eq!(parse_ts_unix("2026-08-19T04:49:29"), 1_787_114_969);
        assert_eq!(parse_ts_unix("garbage"), 0);
        assert_eq!(parse_ts_unix("2026-13-99"), 0, "out-of-range fields refuse");
        // days_from_civil ∘ civil_from_days is the identity on sample dates.
        for day in [0i64, 1, 100, 19_000, 20_663] {
            let (y, m, d) = civil_from_days(day);
            assert_eq!(days_from_civil(y, m, d), day);
        }
        // today_local_date is the date part of a fresh civil computation
        // (shape check — the value moves with the clock).
        let today = today_local_date();
        assert_eq!(today.len(), 10);
        assert_eq!(today.as_bytes()[4], b'-');
        assert_eq!(today.as_bytes()[7], b'-');
        assert_eq!(parse_ts_unix(&today), parse_ts_unix(&format!("{today}T00:00:00")));
    }

    #[cfg(feature = "score_bench")]
    #[test]
    fn score_run_row_is_schema_identical_to_history_record() {
        // The jsonl ↔ store schema-identity contract (Issue 050 T2): a
        // HistoryRecord round-trips through ScoreRunRow with every field
        // preserved — only `recorded_unix` is added.
        let rec = crate::score_bench::report::HistoryRecord {
            ts: "2026-08-19T04:49:29".to_string(),
            sha: "72bac03".to_string(),
            n_ran: 34,
            n_healable: 28,
            n_clean: 6,
            heal_rate: 0.846_153_86,
            created_rate: 0.057_692_308,
            parse_safety: 0.928_571_4,
            decline_correctness: 1.0,
            edits_total: 60,
            vocab_size: 33,
        };
        let row = ScoreRunRow::from(&rec);
        assert_eq!(row.ts, rec.ts);
        assert_eq!(row.sha, rec.sha);
        assert_eq!(row.n_ran, rec.n_ran);
        assert_eq!(row.n_healable, rec.n_healable);
        assert_eq!(row.n_clean, rec.n_clean);
        // f32 fields: the contract is BIT-IDENTITY through the row round-trip
        // (not approximate equality) — float_cmp is the gate here.
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(row.heal_rate, rec.heal_rate);
            assert_eq!(row.created_rate, rec.created_rate);
            assert_eq!(row.parse_safety, rec.parse_safety);
            assert_eq!(row.decline_correctness, rec.decline_correctness);
        }
        assert_eq!(row.edits_total, rec.edits_total);
        assert_eq!(row.vocab_size, rec.vocab_size);
        assert_eq!(row.recorded_unix, parse_ts_unix(&rec.ts));
        // And the serialized jsonl line of the record is a PREFIX-shape of
        // the row: deserializing the record's JSON into a ScoreRunRow keeps
        // recorded_unix at its serde default (0) — the import path.
        let line = serde_json::to_string(&rec).unwrap();
        let from_jsonl: ScoreRunRow = serde_json::from_str(&line).unwrap();
        assert_eq!(from_jsonl.recorded_unix, 0, "jsonl shape carries no unix stamp");
        assert_eq!(from_jsonl.ts, rec.ts);
    }
}
