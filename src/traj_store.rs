//! The neuron-db trajectory store (Plan 019 T4.3) — the DURABLE backend
//! for `--fix-compile` self-evolution, behind `latent_retrieval`.
//!
//! ## What it is
//!
//! [`LocalKvStore`] at `<repo>/.refine/trajectories.kv/` (encrypted WAL,
//! passphrase = a generated seed at `<repo>/.refine/key` — the threat model
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
//! The tier transitions are Issue 021's, durable, now gated by Issue 126
//! lane (b)'s Lemma-4.2 likelihood-ratio guard ([`crate::tier_guard`]):
//! `success && verified` certifies WHEN the goal-side Elo model and the
//! row's own Laplace outcome history agree within ε (counters always
//! record — only the promotion is gated); 3 consecutive failures withdraw
//! (absorbing — a verified success exits, through the same guard). The `learn:` rows are the per-rule EMA twin
//! (Issue 037 E3): every verdict also steps the rule's evolved direction
//! ([`ema_step`](crate::evolve::ema_step) — same rule as the in-process
//! layer), durable + observable via `--memory-stats`.
//!
//! The `elo`/`span_elo` row fields (Issue 039 T2) are written ONLY by the
//! batch pass [`TrajStore::recompute_elo`] (`cargo refine --recompute-elo`)
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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use riir_neuron_db::bm25::Bm25Index;
use riir_neuron_db::local_kv::LocalKvStore;

use crate::elo::EloTable;
use crate::embedder::{EMBED_DIM, embed_into};
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
    /// Issue 105 (PC-ALM credit): the number of runs this key kept an
    /// attempt in — the journey-evidence denominator. `0` on pre-105 rows
    /// (the neutral no-data point: [`crate::journey::journey_factor`]
    /// reads exactly 1.0).
    #[serde(default)]
    pub journey_runs: u32,
    /// Issue 105: Σ over TAINTED runs (the run ended with a masking-stage
    /// error present) of that run's composite `(α·λ_T + ρ·r_T)·256`,
    /// fixed-point. `0` = no tainted evidence — the factor is exactly 1.0
    /// and the row scores bit-identically to pre-105.
    #[serde(default)]
    pub journey_taint_sum: u32,
    /// Component-attributed failure class (Issue 047, Recuris E/ρ/W/C
    /// ontology) — the LAST decision-time attribution recorded for this
    /// row's failures. `None` on success-only rows AND on pre-Issue-047
    /// rows (serde default keeps old rows decoding as unattributed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<crate::component::ComponentClass>,
    /// riir-dapps Issue 119 T2: the repo-relative path (forward slashes) of
    /// the file the CURRENT `shape`/`fix` snippet came from — refreshed by
    /// every SOURCED write ([`TrajStore::sourced`]); unsourced writes leave
    /// it as it was. `""` = unknown (rows written before the field existed
    /// decode with the serde default, and so do rows whose writer had no
    /// file — the envelope migration, the drain's re-records of a row that
    /// never had one).
    ///
    /// LOCAL-ONLY: it is the input the sync gate checks against the policy's
    /// `allow_paths` (an empty path FAILS CLOSED whenever `allow_paths` is
    /// non-empty — [`crate::kat_sync_payload`]'s `gate_row`), and it is
    /// never a payload field: the wire row stays path-free (the `--mine`
    /// promise — "never a file path").
    #[serde(default)]
    pub src_path: String,
    /// Issue 148 T-b (the suite-mint label): the REJECTED candidate set —
    /// the retrieval matcher's proposed rule ids for the span this row's
    /// fix was applied to, MINUS the applied rule — the contrastive
    /// negatives a refine-rule suite mint trains against. Sorted, deduped,
    /// capped at [`REJECTED_CAP`]; refreshed by every record that KNOWS the
    /// candidate set (`Some`), left untouched by records that do not
    /// (`None` — the `src_path` law). The join is chunk-neighborhood
    /// approximate: the matcher proposes per suggest-span, the edit lands
    /// at a line inside one. Vocabulary ids from the compiled corpus, never
    /// vocabulary ids from the compiled corpus, never
    /// user bytes — crosses the wire ungated like `domain` (v4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected: Vec<String>,
    /// Issue 156 — the `FlyBy` state tallies (T1): the no-tool (modelless)
    /// continuation attempts/successes and the tool-assisted (escalated,
    /// transport-served) attempts/successes for this STATE. An ORTHOGONAL
    /// evidence axis (the Issue-105 journey precedent): never touches the
    /// outcome counters, the consecutive-fail ladder, or the tier FSM — a
    /// modelless failure is evidence about the STATE (`FlyBy` keeps it
    /// precisely because no-tool fails), never against the rule.
    #[serde(default)]
    pub b0_attempts: u32,
    /// Issue 156: no-tool continuations that verified — ANY nonzero value
    /// rejects the state (execution-reachable, `FlyBy`'s V>0 law).
    #[serde(default)]
    pub b0_successes: u32,
    /// Issue 156: tool-assisted continuations actually served by the
    /// transport (a stood-by-modelless leg is no-tool, never counted here).
    #[serde(default)]
    pub bd_attempts: u32,
    /// Issue 156: tool-assisted continuations that verified — the rescue
    /// evidence. `b0_successes == 0 ∧ bd_successes >= 3` is the keep-set.
    #[serde(default)]
    pub bd_successes: u32,
    /// Issue 156 (T2): the SHALLOWEST sufficient depth at which a rescue
    /// was demonstrated — [`crate::rescue_mine::RescueDepth::as_u8`];
    /// `0` = none yet. Only successful sightings update it, only downward.
    #[serde(default)]
    pub rescue_depth: u8,
}

/// The per-row cap on [`TrajRowDoc::rejected`] — the label is the
/// neighborhood signal, not the whole index; 16 covers every top-k the
/// CLI ships without letting a wide fan-out bloat the row doc.
pub const REJECTED_CAP: usize = 16;

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
    /// `lint_name:`), or the compiled corpus size when [`Self::source_note`]
    /// is `"runtime-corpus"`.
    pub count: u32,
    /// Whether the domain's feature was compiled into the binary that
    /// stamped the row (`cfg!(feature = ...)` — text counts and compiled
    /// liveness are different facts).
    pub compiled: bool,
    /// Where the row came from: `"source-walk"` (the `--corpus-stats`
    /// needle fallback), `"runtime-corpus"` (the compiled builder count —
    /// Plan 059 B1, the 009-F2 fix), or `"import-jsonl"` (the one-time
    /// jsonl import).
    pub source_note: String,
}

/// The first-hand corpus provenances — a live `--corpus-stats` stamp
/// (either counting mode). The jsonl import yields to these; only a
/// second-hand `import-jsonl` row may be superseded by a re-import.
fn is_first_hand_corpus_source(note: &str) -> bool {
    matches!(note, "source-walk" | "runtime-corpus")
}

/// One durable score-run row (Issue 050 T2) — the Warm-tier twin of a
/// `.benchmarks/score_history.jsonl` line. The first eleven fields are the
/// `score_bench::report::HistoryRecord` shape VERBATIM (schema-identity
/// jsonl ↔ store is pinned by a test); `recorded_unix` is the ordering key
/// derived from `ts` (0 when the timestamp does not parse). The deliberate
/// exceptions (instrumentation, not scored axes — the additive law):
/// `HistoryRecord`'s `ood` OOD-axis (issue 077 T4), `strata` (issue 089 T6),
/// `rule_outcomes` (issue 089 T8), `cross_ooo` (issue 098 T3/T4) and
/// `demonstrability` (issue 134 T1) are dropped here — the jsonl stays the
/// committed projection (pinned by the same test). The plan 329 P4-a
/// toolchain provenance pair (`rustc_version` / `clippy_driver_version`)
/// RIDES the row since issue 202 (option (a): stamp fields only, the KEY
/// unchanged — `scorerun:{recorded_unix}:{sha-or-"nosh"}` keeps its shape;
/// new rows just carry longer payloads). The pooling law is P4-a's report
/// G1 mirrored store-side: a row whose toolchain is unknown (`None` via the
/// serde default — every row written before issue 202) NEVER pools with a
/// pinned row, and a clippy-only difference is a different bucket too
/// ([`score_runs_share_toolchain`] is the bucket rule). Upsert semantics,
/// so a re-import or a dual-write of the same run is idempotent by
/// construction.
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
    /// The bench generation (Plan 092 M5) — mirrors the `HistoryRecord`
    /// field; `0` is the LEGACY UNPINNED class (plan 329 P4-a): rows
    /// written before the field existed default to it (serde) and never
    /// pool with a pinned generation in any comparison.
    #[serde(default = "default_score_generation")]
    pub generation: u8,
    /// Unix seconds of `ts` (the store-key ordering half; 0 = unparseable).
    #[serde(default)]
    pub recorded_unix: u64,
    /// Plan 329 P4-a / issue 202 — the toolchain that produced this row's
    /// rates, mirrored from `HistoryRecord` verbatim (Option semantics
    /// included: `None` = unknown-toolchain, the serde default for every
    /// row written before the field existed). A rate without its toolchain
    /// is not a measurement — the sccache cache-key completeness doctrine.
    #[serde(default)]
    pub rustc_version: Option<String>,
    /// Plan 329 P4-a / issue 202 — the clippy-driver version when the
    /// oracle lane ran (`None` = the lane did not run, or the row predates
    /// the stamp). Part of the pooling pair: a clippy-only difference is a
    /// different comparison bucket.
    #[serde(default)]
    pub clippy_driver_version: Option<String>,
}

/// The serde default for pre-generation score rows (Plan 092 T4): `0` —
/// the LEGACY UNPINNED class (plan 329 P4-a), the same doctrine as the
/// jsonl's `score_bench::report::default_generation`. `ScoreRunRow::from`
/// copies `r.generation` verbatim (u8→u8), so the store carries `0` for
/// legacy rows too — consistent across both media.
#[must_use]
pub fn default_score_generation() -> u8 {
    0
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
            generation: r.generation,
            recorded_unix: parse_ts_unix(&r.ts),
            rustc_version: r.rustc_version.clone(),
            clippy_driver_version: r.clippy_driver_version.clone(),
        }
    }
}

/// Plan 329 P4-a's report G1, mirrored store-side (issue 202): two
/// score-run rows share a comparison bucket ONLY when both toolchain halves
/// match exactly (`None == None` pools with `None`). An unknown-toolchain
/// row never pools with a pinned row, and a clippy-only difference is a
/// different bucket too — the oracle lane may have run on one row and not
/// the other, so the rates are not comparable. The jsonl-side twin is
/// `score_bench::report::detect_saturation`'s anchor split; the store-side
/// comparison sites (`score_runs` consumers) must bucket through this
/// predicate, never compare raw rates across it.
#[must_use]
pub fn score_runs_share_toolchain(a: &ScoreRunRow, b: &ScoreRunRow) -> bool {
    a.rustc_version == b.rustc_version && a.clippy_driver_version == b.clippy_driver_version
}

/// One construction-labeled preference pair (Issue 072 T1) — the durable
/// form of a [`crate::frontier_pairs::ConstructionPair`] at rest. SOFT
/// JUDGE ONLY: these rows feed rating ([`TrajStore::rate_pairs`]) and
/// coverage measurement — never [`EvidenceTier`], never the outcome
/// counters, never retrieval. The `pair:` key namespace is disjoint from
/// trajectories by construction.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct PairRowDoc {
    /// The planted truth's error code.
    pub code: String,
    /// The partner's error code (cross-code by construction — the fence
    /// asserts it, the A/B asserts it again at ingest).
    pub bad_code: String,
    /// The generator case the pair anchors (`gen_...`, the key half).
    pub case_name: String,
    /// Winner strategy (reproduces the planted truth).
    pub good_strategy: String,
    /// Loser strategy (does not resolve this case).
    pub bad_strategy: String,
    /// Winner fix text (the case's seed — self-contained evidence).
    pub good_fix: String,
    /// Loser fix text (the partner's seed).
    pub bad_fix: String,
    /// The anchored span's shape hash (joins pair evidence to spans).
    pub span_shape_hash: String,
    /// Always `"construction"` — the non-circularity marker; a row
    /// carrying any other provenance must never feed a rating.
    pub provenance: String,
    // NO ingest timestamp by design: the row bytes must be construction-
    // deterministic so re-import is idempotent (a wall-clock field broke
    // byte-comparison across a second boundary — caught by the full-suite
    // run; store/git history covers dating).
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
    /// Issue 105: the row's journey evidence, carried for the consumer's
    /// [`crate::journey::journey_factor`] discount on success credit.
    pub journey_runs: u32,
    pub journey_taint_sum: u32,
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
    /// The store root (`.refine/trajectories.kv`), for diagnostics.
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

/// Resolve or create the per-repo key seed at `<root>/.refine/key`. The
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

/// Overwrite the row's source path when the write is SOURCED (riir-dapps
/// Issue 119 T2); an unsourced write (`None`) keeps whatever path the row
/// already carries.
fn stamp_src(row: &mut TrajRowDoc, src: Option<&str>) {
    if let Some(p) = src {
        p.clone_into(&mut row.src_path);
    }
}

/// Stamp the Issue-148 T-b label: `Some(rejected)` refreshes the row's
/// candidate set (clamped to [`REJECTED_CAP`]); `None` leaves it — a record
/// without the candidate set in hand never erases one that had it.
fn stamp_rejected(row: &mut TrajRowDoc, rejected: Option<&[String]>) {
    if let Some(rs) = rejected {
        let mut v: Vec<String> = rs.to_vec();
        v.sort();
        v.dedup();
        v.truncate(REJECTED_CAP);
        row.rejected = v;
    }
}

/// `file` relative to `root`, forward slashes — both sides resolved to
/// absolute + canonical form first (a relative `file` resolves against the
/// cwd; a file that no longer exists resolves through its canonical parent,
/// so a deleted file still normalizes). `""` when `file` is not under
/// `root`.
#[must_use]
pub fn repo_relative_path(root: &Path, file: &Path) -> String {
    fn resolve(p: &Path) -> Option<PathBuf> {
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(p)
        };
        if let Ok(c) = abs.canonicalize() {
            return Some(c);
        }
        let parent = abs.parent().and_then(|d| d.canonicalize().ok());
        Some(match (parent, abs.file_name()) {
            (Some(d), Some(name)) => d.join(name),
            _ => abs,
        })
    }
    let (Some(root), Some(file)) = (resolve(root), resolve(file)) else {
        return String::new();
    };
    let Ok(rel) = file.strip_prefix(&root) else {
        return String::new();
    };
    let parts: Vec<String> = rel
        .components()
        .map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Option<_>>()
        .unwrap_or_default();
    parts.join("/")
}

/// A [`TrajStore`] write view bound to one snippet source file
/// ([`TrajStore::sourced`], riir-dapps Issue 119 T2). Same methods + call
/// shapes as the store's own `record*`; each also refreshes the row's
/// [`TrajRowDoc::src_path`].
#[derive(Clone, Copy)]
pub struct SourcedTraj<'a> {
    store: &'a TrajStore,
    src_path: &'a str,
}

impl SourcedTraj<'_> {
    /// [`TrajStore::record`], sourced.
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
        self.record_classified(lint_key, shape, fix, success, verified, run_id, None, None)
    }

    /// [`TrajStore::record_classified`], sourced.
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    #[allow(clippy::too_many_arguments)] // mirrors TrajStore::record_classified
    pub fn record_classified(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        success: bool,
        verified: bool,
        run_id: &str,
        component: Option<crate::component::ComponentClass>,
        rejected: Option<&[String]>,
    ) -> Result<(), String> {
        self.store.record_classified_src(
            lint_key,
            shape,
            fix,
            success,
            verified,
            run_id,
            component,
            Some(self.src_path),
            rejected,
        )
    }

    /// [`TrajStore::record_journey`], sourced.
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn record_journey(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        run_id: &str,
        taint_composite_256: Option<u32>,
    ) -> Result<(), String> {
        self.store.record_journey_src(
            lint_key,
            shape,
            fix,
            run_id,
            taint_composite_256,
            Some(self.src_path),
        )
    }

    /// [`TrajStore::record_applied`], sourced.
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
        rejected: Option<&[String]>,
    ) -> Result<(), String> {
        self.store
            .record_applied_src(lint_key, shape, fix, run_id, Some(self.src_path), rejected)
    }
}

impl TrajStore {
    /// Open (or create) the store under `<root>/.refine/trajectories.kv` and
    /// rebuild the BM25 axis from the rows.
    ///
    /// # Errors
    ///
    /// `LocalKvStore::open_dir` failures (WAL replay, unwritable dir) or
    /// the key-seed write. A corrupt row JSON does NOT fail the open — it
    /// is skipped + counted in the returned warning count (one bad row
    /// must not brick the store).
    pub fn open(root: &Path) -> Result<(Self, usize), String> {
        let heal = root.join(".refine");
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
                    self.bm25.index(&k, &format!("{} {}", t.shape, t.fix));
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
    /// durable, gated by Issue 126 lane (b)'s Lemma-4.2 guard:
    /// `success && verified` certifies when
    /// [`certify_allowed`](crate::tier_guard::certify_allowed) holds — the
    /// fresh-row cold start is behavior-identical; 3 consecutive failures
    /// withdraw — absorbing, a verified success exits through the same
    /// guard). Stamps the
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
        self.record_classified(lint_key, shape, fix, success, verified, run_id, None, None)
    }

    /// [`record`](Self::record) with the Issue 047 component attribution:
    /// `Some(class)` stamps the row's failure class (the LAST decision-time
    /// attribution — successes leave a previous tag in place; the tag is
    /// failure metadata for the audit, not a state machine).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    #[allow(clippy::too_many_arguments)] // mirrors `record`'s call shape + the attribution + label params
    pub fn record_classified(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        success: bool,
        verified: bool,
        run_id: &str,
        component: Option<crate::component::ComponentClass>,
        rejected: Option<&[String]>,
    ) -> Result<(), String> {
        self.record_classified_src(
            lint_key, shape, fix, success, verified, run_id, component, None, rejected,
        )
    }

    /// The one verdict-write body behind [`record_classified`](Self::record_classified)
    /// and [`SourcedTraj::record_classified`]: `src = Some(path)` refreshes
    /// the row's [`TrajRowDoc::src_path`]; `None` leaves it untouched.
    #[allow(clippy::too_many_arguments)] // record_classified's shape + the source path + the label
    fn record_classified_src(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        success: bool,
        verified: bool,
        run_id: &str,
        component: Option<crate::component::ComponentClass>,
        src: Option<&str>,
        rejected: Option<&[String]>,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self.load_or_new_row(&key, lint_key, shape, fix);
        // The text halves update with the latest sighting (the shape key
        // is stable, the fix may have been refined).
        row.fix = fix.to_string();
        stamp_src(&mut row, src);
        stamp_rejected(&mut row, rejected);
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        if let Some(class) = component {
            row.component = Some(class);
        }
        if success {
            row.success_count = row.success_count.saturating_add(1);
            row.consecutive_fails = 0;
            // Issue 126 lane (b) — the Lemma-4.2 likelihood-ratio guard: the
            // keep (reached goal) relabels to goal attainment (commanded
            // goal) only when the goal-side Elo model and the row's own
            // Laplace outcome history agree within ε. Counters always
            // record; only the tier promotion is gated. Withdrawn recovery
            // passes the same evidence bar (one uniform FSM).
            if verified
                && crate::tier_guard::certify_allowed(
                    row.elo,
                    row.span_elo,
                    row.success_count,
                    row.fail_count,
                )
            {
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

    /// Issue 105 — record one run's journey credit for a key that kept an
    /// attempt in that run. `taint_composite_256` is `Some` only for
    /// TAINTED runs (the run ended with a masking-stage error present —
    /// `JourneyCredit::tainted()`); clean runs pass `None` and bump the
    /// journey count alone. Never touches the outcome counters, tiers, or
    /// the keep rule — the credit is an ORTHOGONAL evidence axis consumed
    /// at selection time via [`crate::journey::journey_factor`] (the
    /// `shape_conditional_score` discount).
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure.
    pub fn record_journey(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        run_id: &str,
        taint_composite_256: Option<u32>,
    ) -> Result<(), String> {
        self.record_journey_src(lint_key, shape, fix, run_id, taint_composite_256, None)
    }

    /// The journey-write body (see [`record_journey`](Self::record_journey));
    /// `src` as in [`record_classified_src`](Self::record_classified_src).
    fn record_journey_src(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        run_id: &str,
        taint_composite_256: Option<u32>,
        src: Option<&str>,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self.load_or_new_row(&key, lint_key, shape, fix);
        stamp_src(&mut row, src);
        row.journey_runs = row.journey_runs.saturating_add(1);
        if let Some(c) = taint_composite_256 {
            row.journey_taint_sum = row.journey_taint_sum.saturating_add(c);
        }
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        self.put_row(&key, &row)
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
        rejected: Option<&[String]>,
    ) -> Result<(), String> {
        self.record_applied_src(lint_key, shape, fix, run_id, None, rejected)
    }

    /// The apply-write body (see [`record_applied`](Self::record_applied));
    /// `src` as in [`record_classified_src`](Self::record_classified_src).
    fn record_applied_src(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        run_id: &str,
        src: Option<&str>,
        rejected: Option<&[String]>,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self.load_or_new_row(&key, lint_key, shape, fix);
        row.fix = fix.to_string();
        stamp_src(&mut row, src);
        stamp_rejected(&mut row, rejected);
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        self.put_row(&key, &row)
    }

    /// Issue 156 (T1+T2) — record one arm-H paired observation for the state
    /// `(lint_key, shape)`: the modelless (no-tool) continuation outcome and,
    /// when the escalation chain actually served a transport answer, the
    /// tool-assisted (escalated) outcome at its depth.
    ///
    /// The tallies are an ORTHOGONAL evidence axis on the row (the Issue-105
    /// journey precedent): they never touch the outcome counters, the
    /// consecutive-fail ladder, or the tier FSM — a modelless failure is
    /// evidence about the STATE (`FlyBy` keeps it precisely because no-tool
    /// fails), never against the rule. The escalated verdict, when it
    /// VERIFIES, additionally rides the EXISTING evidence vocabulary
    /// ([`Self::record_classified`] success+verified — "no-fix fails, fix
    /// verifies"), so the rescue exemplar is retrievable as a Certified-tier
    /// trajectory when the Issue-126 tier guard agrees; an escalated
    /// non-verification tallies only (a failed rescue attempt is not evidence
    /// against the rule either).
    ///
    /// AUGMENT-ONLY law: this predicate narrows the candidate set entering
    /// the network-oracle agreement VETO — it never replaces the veto.
    ///
    /// # Errors
    ///
    /// KV put or JSON encode failure (either write half).
    pub fn record_rescue_pair(
        &self,
        lint_key: &str,
        shape: &str,
        fix: &str,
        modelless_success: bool,
        escalated: Option<(bool, crate::rescue_mine::RescueDepth)>,
        run_id: &str,
    ) -> Result<(), String> {
        let key = Self::row_key(lint_key, shape);
        let mut row = self.load_or_new_row(&key, lint_key, shape, fix);
        row.b0_attempts = row.b0_attempts.saturating_add(1);
        row.b0_successes = row
            .b0_successes
            .saturating_add(u32::from(modelless_success));
        if let Some((success, depth)) = escalated {
            row.bd_attempts = row.bd_attempts.saturating_add(1);
            row.bd_successes = row.bd_successes.saturating_add(u32::from(success));
            if success && (row.rescue_depth == 0 || depth.as_u8() < row.rescue_depth) {
                row.rescue_depth = depth.as_u8();
            }
        }
        row.last_run_id = run_id.to_string();
        row.last_seen_at = now_unix_secs();
        self.put_row(&key, &row)?;
        // The exemplar half (T2): a verified rescue rides the existing
        // evidence vocabulary; the tier guard governs certification.
        if matches!(escalated, Some((true, _))) {
            self.record_classified(lint_key, shape, fix, true, true, run_id, None, None)?;
        }
        Ok(())
    }

    /// The accumulated rescue evidence for one state (read-only): the `FlyBy`
    /// tallies + the row's evidence tier. Zeros + `Heuristic` when the state
    /// has no row or the row is undecodable.
    #[must_use]
    pub fn rescue_state(
        &self,
        lint_key: &str,
        shape: &str,
    ) -> (crate::rescue_mine::RescueTally, EvidenceTier) {
        let empty = crate::rescue_mine::RescueTally::default();
        let key = Self::row_key(lint_key, shape);
        let Some(bytes) = self.kv.get_bytes(&key) else {
            return (empty, EvidenceTier::Heuristic);
        };
        let Ok(row) = serde_json::from_slice::<TrajRowDoc>(&bytes) else {
            return (empty, EvidenceTier::Heuristic);
        };
        (
            crate::rescue_mine::RescueTally {
                b0_attempts: row.b0_attempts,
                b0_successes: row.b0_successes,
                bd_attempts: row.bd_attempts,
                bd_successes: row.bd_successes,
                depth: row.rescue_depth,
            },
            EvidenceTier::from_u8(row.tier),
        )
    }

    /// riir-dapps Issue 119 T2: a write view that stamps every row it
    /// touches with `src_path` — the repo-relative path of the file the
    /// snippet came from (normalize with [`src_path_of`](Self::src_path_of)).
    /// The production heal write sites go through this; the plain `record*`
    /// methods stay for callers with no file (tests, the drain's re-records,
    /// proof-of-concept examples) and leave a row's path as it was.
    #[must_use]
    pub fn sourced<'a>(&'a self, src_path: &'a str) -> SourcedTraj<'a> {
        SourcedTraj {
            store: self,
            src_path,
        }
    }

    /// The repo root this store belongs to (`<root>` of
    /// `<root>/.refine/trajectories.kv`).
    #[must_use]
    pub fn repo_root(&self) -> Option<&Path> {
        self.root.parent()?.parent()
    }

    /// `file` as a repo-relative, forward-slash path under
    /// [`repo_root`](Self::repo_root) — the [`TrajRowDoc::src_path`] form
    /// the sync policy's `allow_paths` globs are written against. `""` when
    /// the file is not under the repo root (or either side cannot be
    /// resolved): the unknown path, which the sync gate fails closed on.
    #[must_use]
    pub fn src_path_of(&self, file: &Path) -> String {
        self.repo_root()
            .map_or_else(String::new, |root| repo_relative_path(root, file))
    }

    /// The stored consecutive-fail streak for the exact (`lint_key`, `shape`,
    /// `fix`) pair — the P12 re-proposal-suppression signal. `0` means no
    /// row, no recorded failure, or the row's latest `fix` text differs (the
    /// proposer's output changed since the failures — the pair is new, so a
    /// retry is not a deterministic replay of a known revert). Read-only:
    /// querying never creates a row.
    #[must_use]
    pub fn pair_consecutive_fails(&self, lint_key: &str, shape: &str, fix: &str) -> u32 {
        let key = Self::row_key(lint_key, shape);
        let Some(bytes) = self.kv.get_bytes(&key) else {
            return 0;
        };
        let Ok(row) = serde_json::from_slice::<TrajRowDoc>(&bytes) else {
            return 0;
        };
        if row.fix == fix {
            row.consecutive_fails
        } else {
            0
        }
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
    fn load_or_new_row(&self, key: &[u8], lint_key: &str, shape: &str, fix: &str) -> TrajRowDoc {
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
                    journey_runs: 0,
                    journey_taint_sum: 0,
                    src_path: String::new(),
                    rejected: Vec::new(),
                    b0_attempts: 0,
                    b0_successes: 0,
                    bd_attempts: 0,
                    bd_successes: 0,
                    rescue_depth: 0,
                }
            })
    }

    /// Encode + put one row + keep the BM25 axis live (the shared tail of
    /// every row-writing method).
    fn put_row(&self, key: &[u8], row: &TrajRowDoc) -> Result<(), String> {
        let bytes = serde_json::to_vec(row).map_err(|e| format!("encode row: {e}"))?;
        self.kv
            .put(key, &bytes)
            .map_err(|e| format!("put row: {e}"))?;
        self.bm25.index(key, &format!("{} {}", row.shape, row.fix));
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
    fn learn_step(
        &self,
        lint_key: &str,
        fix: &str,
        success: bool,
        run_id: &str,
    ) -> Result<(), String> {
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
            row.last_delta = crate::evolve::ema_step(
                &mut dir,
                &fix_dir,
                success,
                &crate::evolve::EvolveConfig::default(),
            );
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
        let bytes = serde_json::to_vec(&row).map_err(|e| format!("encode learn row: {e}"))?;
        self.kv
            .put(&key, &bytes)
            .map_err(|e| format!("put learn row: {e}"))
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

    /// Issue 105 (T4 instrumentation): every trajectory row that carries
    /// journey evidence, as `(lint_key, journey_runs, journey_taint_sum)`
    /// — the `--frontier-report` integral axis feeds on this (report-only,
    /// never scored).
    #[must_use]
    pub fn journey_rows(&self) -> Vec<(String, u32, u32)> {
        self.kv
            .scan_prefix(b"traj:")
            .into_iter()
            .filter_map(|(_, v)| serde_json::from_slice::<TrajRowDoc>(&v).ok())
            .filter(|row| row.journey_runs > 0)
            .map(|row| (row.lint_key, row.journey_runs, row.journey_taint_sum))
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
                journey_runs: row.journey_runs,
                journey_taint_sum: row.journey_taint_sum,
            });
        }
        // Fuse: normalize bm25 by the max hit (unbounded scale), add the
        // already-bounded latent.
        let max_bm25 = out.iter().map(|n| n.bm25).fold(0.0f32, f32::max);
        let norm = max_bm25.max(1e-9);
        out.sort_by(|a, b| {
            let fa = a.bm25 / norm + a.latent;
            let fb = b.bm25 / norm + b.latent;
            fb.total_cmp(&fa)
        });
        out.truncate(k);
        out.shrink_to_fit();
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

    /// Issue 067 T1 — Certified same-rule trajectory density: the number of
    /// DISTINCT Certified rows per `lint_key`, one scan at call time. This
    /// is the reopen-trigger instrument: the issue's T2 (same-rule exemplar
    /// channels, condition-first placement) stays gated until a queried
    /// lint holds `>= 3` Certified same-rule trajectories. `Withdrawn` and
    /// `Heuristic` rows do not count (Heuristic is the unverified first
    /// sight; Withdrawn is the absorbing failure state).
    #[must_use]
    pub fn certified_density(&self) -> BTreeMap<String, usize> {
        let mut out: BTreeMap<String, usize> = BTreeMap::new();
        for (_, v) in self.kv.scan_prefix(b"traj:") {
            let Ok(row) = serde_json::from_slice::<TrajRowDoc>(&v) else {
                continue;
            };
            if EvidenceTier::from_u8(row.tier) == EvidenceTier::Certified {
                *out.entry(row.lint_key).or_insert(0) += 1;
            }
        }
        out
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
        // ── Issue 072 T7: the standing dispersion column — per-rule
        // outcome entropy (success/fail, the store's two classes; the
        // driver's 3-class version lives in `dispersion.rs`). A report
        // over observed rules where EVERY one reads zero entropy is the
        // vacuous flag: selection tuning over a no-dispersion loop is the
        // T39 finding, surfaced as standing telemetry instead of a one-off.
        write_dispersion_section(&mut report, &rows);
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
    /// Issue 190 (`elo_k_eff` feature): each match is scheduled by the
    /// rule's cumulative evidence — `elo_k_eff(ELO_K, n, ELO_K_EFF_FLOOR)`
    /// dampens a young rule's first outcomes (thin evidence ⇒ small
    /// movement); from the floor on the match moves at exactly [`ELO_K`].
    /// The feature-off build schedules every match at [`ELO_K`] verbatim
    /// (the same primitive, the same args — byte-identical rows). Zero-sum
    /// per match: the span opponent shares the rule's scheduled K.
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
        use crate::elo::{ELO_K, ELO_K_EFF_FLOOR, elo_k_eff};
        let rows = self.dump();
        let mut summary = EloRecomputeSummary {
            rows_seen: rows.len(),
            ..EloRecomputeSummary::default()
        };
        let mut table = EloTable::new();
        let mut rules = std::collections::BTreeSet::new();
        let mut spans = std::collections::BTreeSet::new();
        // Issue 190 — per-rule evidence counts, in canonical replay order.
        let mut evidence: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for row in &rows {
            if row.success_count.saturating_add(row.fail_count) == 0 {
                continue; // apply-only sighting: not a match
            }
            let span_id = shape_hash(&row.shape);
            rules.insert(row.lint_key.clone());
            spans.insert(span_id.clone());
            for _ in 0..row.fail_count {
                let n = evidence.entry(row.lint_key.clone()).or_default();
                *n += 1;
                let k = if cfg!(feature = "elo_k_eff") {
                    elo_k_eff(ELO_K, *n, ELO_K_EFF_FLOOR)
                } else {
                    ELO_K
                };
                table.play_with_k(&row.lint_key, &span_id, false, k);
                summary.matches += 1;
            }
            for _ in 0..row.success_count {
                let n = evidence.entry(row.lint_key.clone()).or_default();
                *n += 1;
                let k = if cfg!(feature = "elo_k_eff") {
                    elo_k_eff(ELO_K, *n, ELO_K_EFF_FLOOR)
                } else {
                    ELO_K
                };
                table.play_with_k(&row.lint_key, &span_id, true, k);
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

    /// Issue 232 T1 — the consumed-window digest: BLAKE3 over the canonical
    /// row encoding (`lint_key \u{1f} shape \u{1f} success \u{1f} fail` per
    /// row, store key order) — the window identity the ledger keys on. Any
    /// verdict mutation (a new outcome recorded, counters moved) changes
    /// the digest; an unchanged store re-derives it byte-identically.
    #[cfg(feature = "consumed_window_ledger")]
    #[must_use]
    pub fn consumed_window_digest(&self) -> String {
        use blake3::Hasher;
        let mut h = Hasher::new();
        for row in self.dump() {
            h.update(row.lint_key.as_bytes());
            h.update(&[0x1f]);
            h.update(row.shape.as_bytes());
            h.update(&[0x1f]);
            h.update(&row.success_count.to_le_bytes());
            h.update(&[0x1f]);
            h.update(&row.fail_count.to_le_bytes());
            h.update(&[0x0a]);
        }
        h.finalize().to_hex().to_string()
    }

    /// Issue 232 T1 — [`Self::recompute_elo`] at the consumed-window
    /// ledger's posture: consult + record the window consumption BEFORE the
    /// replay (the reuse count at read time), then replay verbatim — the
    /// summary is bit-identical to the ungated call (the ledger reports,
    /// never refuses; the replay's verdict on a reused window is
    /// UNDECIDABLE, not blocked).
    ///
    /// `selection` names the reading selection (provenance in the ledger
    /// and its report line); `epoch` stamps the consumption (the fixture
    /// passes pinned epochs; the CLI passes now). Returns the window
    /// digest too — the identity the report line names.
    ///
    /// # Errors
    ///
    /// Same as [`Self::recompute_elo`].
    #[cfg(feature = "consumed_window_ledger")]
    pub fn recompute_elo_with_ledger(
        &self,
        ledger: &mut crate::consumed_window::WindowLedger,
        selection: &str,
        epoch: u64,
    ) -> Result<
        (
            EloRecomputeSummary,
            crate::consumed_window::WindowRead,
            String,
        ),
        String,
    > {
        let digest = self.consumed_window_digest();
        let read = ledger.consult(&digest, selection, epoch);
        let summary = self.recompute_elo()?;
        Ok((summary, read, digest))
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
        let sha = if rec.sha.is_empty() {
            "nosh"
        } else {
            rec.sha.as_str()
        };
        format!("scorerun:{}:{}", rec.recorded_unix, sha).into_bytes()
    }

    /// Encode + put one row IFF the stored bytes differ (the import path's
    /// idempotence counter: `false` = unchanged, nothing written).
    fn put_row_if_changed(&self, key: &[u8], row: &impl serde::Serialize) -> Result<bool, String> {
        let bytes = serde_json::to_vec(row).map_err(|e| format!("encode row: {e}"))?;
        if self
            .kv
            .get_bytes(key)
            .is_some_and(|old| old.as_ref() == bytes.as_slice())
        {
            return Ok(false);
        }
        self.kv
            .put(key, &bytes)
            .map_err(|e| format!("put row: {e}"))?;
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
    /// not rewritten, and an existing FIRST-HAND stamp (`source-walk` or
    /// `runtime-corpus`) is NEVER clobbered — the live stamp outranks the
    /// second-hand projection (the store is the source of truth, so
    /// re-importing a stale jsonl cannot overwrite today's live count).
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
                // Provenance precedence: a first-hand live stamp (source-walk
                // needle OR the runtime corpus builder — Plan 059 B2) outranks
                // the second-hand projection — yield, count as seen only.
                // Same-provenance re-imports compare-and-put.
                let yields = self
                    .kv
                    .get_bytes(&key)
                    .and_then(|old| serde_json::from_slice::<CorpusCountRow>(&old).ok())
                    .is_some_and(|old| is_first_hand_corpus_source(&old.source_note));
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

    // ── Issue 072 T1/T3: the construction-pair family ─────────────────

    /// Full row key for one construction pair: `pair:{code}:{case}` — the
    /// case name is unique per generator case, so the key is 1:1 with the
    /// pair corpus and a re-import is idempotent by construction.
    fn pair_key(code: &str, case: &str) -> Vec<u8> {
        format!("pair:{code}:{case}").into_bytes()
    }

    /// Ingest construction pairs (Issue 072 T1). SOFT-JUDGE-ONLY by
    /// contract: pair rows never carry or touch [`EvidenceTier`], never
    /// bump success/fail counters, and are invisible to `dump()`/retrieval
    /// (a separate prefix namespace). Idempotent — the same pair corpus
    /// writes nothing on a second import.
    ///
    /// Returns `(rows_seen, rows_written)`.
    ///
    /// # Errors
    ///
    /// KV put / JSON encode failure.
    pub fn import_pairs(
        &self,
        pairs: &[crate::frontier_pairs::ConstructionPair],
    ) -> Result<(usize, usize), String> {
        let (mut seen, mut written) = (0usize, 0usize);
        for p in pairs {
            seen += 1;
            let row = PairRowDoc {
                code: p.code.to_string(),
                bad_code: p.bad_code.to_string(),
                case_name: p.case_name.clone(),
                good_strategy: p.good_strategy.clone(),
                bad_strategy: p.bad_strategy.clone(),
                good_fix: p.good_fix.clone(),
                bad_fix: p.bad_fix.clone(),
                span_shape_hash: shape_hash(&p.span_shape),
                provenance: p.provenance.to_string(),
            };
            if self.put_row_if_changed(&Self::pair_key(&row.code, &row.case_name), &row)? {
                written += 1;
            }
        }
        Ok((seen, written))
    }

    /// All stored pair rows (key order — `BTreeMap` scan, deterministic).
    ///
    /// # Errors
    ///
    /// JSON decode failure on a stored row (a corrupt row is named and
    /// skipped, never fatal — the import-path convention).
    pub fn pair_rows(&self) -> Result<Vec<PairRowDoc>, String> {
        let mut out = Vec::new();
        for (key, bytes) in self.kv.scan_prefix(b"pair:") {
            match serde_json::from_slice::<PairRowDoc>(&bytes) {
                Ok(row) => out.push(row),
                Err(e) => eprintln!(
                    "traj-store: skipping corrupt pair row {}: {e}",
                    String::from_utf8_lossy(&key)
                ),
            }
        }
        Ok(out)
    }

    /// Issue 072 T3 — the pair-fed rating pass: a Bradley–Terry closed-form
    /// fit over every stored construction pair, mapped onto the standard
    /// Elo scale. ALONGSIDE, never replacing, the outcome-fed
    /// [`Self::recompute_elo`]: this touches NO trajectory row, bumps NO
    /// counter, and answers from the `pair:` namespace only.
    ///
    /// Issue 089 T5 — the non-circularity fence, ENFORCED: only rows
    /// carrying [`crate::frontier_pairs::PROVENANCE`] ("construction")
    /// feed the fit. The row contract has said "any other provenance must
    /// never feed a rating" since 072; it was vacuous while every row was
    /// construction, and is load-bearing now that the organic vocabulary
    /// exists — an organic row is curriculum (weight 1.0 in yield
    /// accounting) precisely because it is OBSERVED, and observed wins are
    /// never rating evidence (the healer must not grade its own behavior).
    /// Excluded rows are counted loud, never silently dropped.
    ///
    /// Returns `(fit, pairs_rated)`; the fit maps strategy id → Elo-scale
    /// rating (the same base/scale the outcome table uses, so the two
    /// tables are directly comparable and combinable by the consumer).
    ///
    /// # Errors
    ///
    /// Propagates [`Self::pair_rows`].
    pub fn rate_pairs(
        &self,
        iters: usize,
    ) -> Result<(std::collections::BTreeMap<String, f32>, usize), String> {
        let rows = self.pair_rows()?;
        let fenced: Vec<&PairRowDoc> = rows
            .iter()
            .filter(|r| r.provenance == crate::frontier_pairs::PROVENANCE)
            .collect();
        if fenced.len() < rows.len() {
            eprintln!(
                "traj-store: rate_pairs excluded {} pair row(s) with non-construction provenance (the non-circularity fence, Issue 089 T5)",
                rows.len() - fenced.len()
            );
        }
        let obs: Vec<crate::elo::PairObservation> = fenced
            .iter()
            .map(|r| crate::elo::PairObservation {
                winner: r.good_strategy.clone(),
                loser: r.bad_strategy.clone(),
            })
            .collect();
        let n = obs.len();
        Ok((crate::elo::bradley_terry_elo(&obs, iters), n))
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

/// The standing dispersion column (Issue 072 T7) — per-rule outcome
/// entropy (success/fail, the store's two classes; the driver's 3-class
/// version lives in `dispersion.rs`). A report over observed rules where
/// EVERY one reads zero entropy is the vacuous flag: selection tuning over
/// a no-dispersion loop is the T39 finding, surfaced as standing telemetry
/// instead of a one-off.
fn write_dispersion_section(report: &mut String, rows: &[TrajRowDoc]) {
    use std::fmt::Write as _;
    let mut by_rule: std::collections::BTreeMap<&str, (u32, u32)> =
        std::collections::BTreeMap::new();
    for r in rows {
        let e = by_rule.entry(r.lint_key.as_str()).or_insert((0, 0));
        e.0 += r.success_count;
        e.1 += r.fail_count;
    }
    let mut ents: Vec<(String, f64)> = by_rule
        .iter()
        .map(|(rule, (s, f))| {
            let outcomes = crate::dispersion::from_counts(*s as usize, *f as usize, 0);
            (
                (*rule).to_string(),
                crate::dispersion::outcome_entropy(&outcomes),
            )
        })
        .collect();
    ents.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let vacuous = !ents.is_empty() && ents.iter().all(|(_, h)| *h <= f64::EPSILON);
    let _ = writeln!(
        report,
        "dispersion: {} rules observed | vacuous window: {} | top: {}",
        ents.len(),
        if vacuous {
            "YES (no rule shows outcome dispersion — selection tuning is blind)"
        } else {
            "no"
        },
        ents.iter()
            .take(3)
            .map(|(r, h)| format!("{r}={h:.2}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
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
pub fn shape_conditional_score(
    store: &TrajStore,
    code: &str,
    strategy_id: &str,
    shape: &str,
) -> f32 {
    let neighbors = store.retrieve(shape, &format!("{code}:"), 8);
    let mut s = 0.0f32;
    let mut f = 0.0f32;
    let mut w_sum = 0.0f32;
    for n in &neighbors {
        if !n.lint_key.ends_with(&format!(":{strategy_id}")) {
            continue;
        }
        // Relevance weight: nearer shapes carry more of the outcome
        // evidence. ⚠ `n.bm25` is the RAW BM25 score — `retrieve`
        // normalizes by the max hit only for its sort key and returns the
        // raw value — so the pseudo-count TOTAL scales with BM25 magnitude,
        // not with observations (Issue 139 T1). Left as-is on measurement:
        // 0 compile-lane rows across 8 live stores, and Bench 045 found no
        // ordering headroom for any policy. Re-gate before changing it.
        let w = n.bm25 + n.latent + 1e-6;
        w_sum += w;
        #[allow(clippy::cast_precision_loss)] // trajectory counts are ~10^2
        let (ns, nf) = (n.success_count as f32, n.fail_count as f32);
        // Issue 105: successes accumulated inside tainted journeys
        // carry reduced credit (the strand-keep repair); failures count
        // in full — taint reprices success evidence only.
        let jf = crate::journey::journey_factor(n.journey_runs, n.journey_taint_sum);
        s += w * ns * jf;
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
            .total_cmp(&belief(a))
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
            .then(a.total_len().cmp(&b.total_len()))
            .then(a.strategy_id.cmp(&b.strategy_id))
    });
}

#[cfg(test)]
#[path = "traj_store_tests.rs"]
mod tests;
