# LatentFixMemory — 4-axis retrieval over fix trajectories

Stores past fix attempts (`FixTrajectory`) and retrieves the top-K most
relevant ones for a given query. Borrows the multi-axis retrieval shape from
`riir-rag` (riir-ai), reimplemented locally.

## The retrieval axes

### 1. Latent (dot + sigmoid)

```text
score = sigmoid(dot(embed(query), trajectory.direction))
```

"Similar lint + code pattern" signal. Uses the BLAKE3-deterministic
embedder (`embed_into`) for the query direction, and each trajectory's
cached `direction` field (computed once at insert time).

### 2. BM25 (simplified)

```text
if query.lint_name_hash == trajectory.lint_name_hash:
    score += BM25_BOOST
```

"Same lint, different code" signal. Uses a pre-computed `lint_name_hash`
(BLAKE3-derived `u64`) for O(1) matching — avoids per-trajectory string
comparison in the retrieval hot path.

A full BM25 over code tokens would add complexity for marginal POC gain;
the latent axis already captures code similarity.

### 3. Trust — the evidential-tier prior (Issue 021)

```text
combined = 0.4·latent + 0.6·bm25 + 0.2·trust_prior(tier)
         where trust_prior: Certified 1.0 / Heuristic 0.5 / Withdrawn (excluded)
```

A bounded per-entry prior from the trajectory's [`EvidenceTier`] — HOW the
fix's outcome was established. Trust breaks ties + small similarity edges in
favor of Certified entries without overriding relevance: at the default
weights, a different-lint Certified entry (≤ 0.4 + 0.2) can never outrank a
same-lint Heuristic one (≥ 0.6 + 0.1). The prior is sigmoid-family bounded
per entry — never a softmax across entries (house rule: scores stay
independent, no cross-entry normalization).

**Withdrawn entries are never emitted as suggestions** — `retrieve_into`
skips them before scoring. They stay in memory + reachable via
`iter()`/`get()` for the adaptation loop's failure analysis.

This is the retelling-bias defense (arXiv:2608.11195, Research 049): without
the tier, an early syn-parse-only success is indistinguishable in memory
from a full-validator admission, and iterated retrieval launders the former
into the latter.

### 4. Graph (deferred)

Out of scope per Issue 004 — deferred until G5 passes with latent + BM25
only (it did). Will revisit if a concrete use case for graph-based
retrieval (e.g., "fixes that depend on other fixes") materializes.

## FixTrajectory

```rust
pub struct FixTrajectory {
    pub lint_name: String,         // e.g. "needless_return"
    pub lint_name_hash: u64,       // BLAKE3 hash — for O(1) BM25 matching
    pub buggy_code: String,        // the code that triggered the lint
    pub fixed_code: String,        // the fix that was applied
    pub direction: Direction,      // cached [f32; 8] embedding
    pub success_count: u32,        // times this fix led to success
    pub fail_count: u32,           // times this fix failed
    pub consecutive_fails: u32,    // streak driver for the Withdrawn transition (Issue 021)
    pub tier: EvidenceTier,        // evidential status (Issue 021)
}
```

The `direction` field is the latent embedding of `(lint_name, fixed_code)`
computed once at `new()` time and cached for O(1) retrieval scoring.

**Stats-bucket semantics:** `record_outcome` matches by `(lint_name,
fixed_code)` — trajectories sharing that pair are ONE bucket (the first
match receives the outcome). `buggy_code` is informational.

## Alloc-free read path

The retrieve path reuses a caller-provided scratch `Vec<usize>` for the
index sort, and writes results into a caller-provided slice. The only
allocation is the `Vec<FixTrajectory>` growth on insert (amortized).

## API

```rust
pub struct LatentFixMemory {
    trajectories: Vec<FixTrajectory>,
}

impl LatentFixMemory {
    pub fn insert(&mut self, trajectory: FixTrajectory);
    pub fn retrieve(&self, query: &LatentQuery, k: usize,
                    scratch: &mut Vec<usize>,
                    out: &mut [ScoredTrajectory]) -> usize;
    pub fn record_outcome(&mut self, lint_name: &str, fixed_code: &str,
                          success: bool, verified: bool);
    pub fn record_certified(&mut self, lint_name: &str, buggy_code: &str,
                            fixed_code: &str);
    pub fn success_rate_for(&self, lint_name: &str, fixed_code: &str) -> f32;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

## EvidenceTier (Issue 021)

```rust
#[repr(u8)]
pub enum EvidenceTier {
    Certified = 0,  // full-validator admission (the only promotion + only Withdrawn exit)
    Heuristic = 1,  // syn-parse only / unverified history (default)
    Withdrawn = 2,  // 3+ consecutive post-hoc failures; absorbing
}
```

Transition discipline (the riir-chain slashing-monotonicity pattern —
monotone under retelling, which is the property whose absence caused the
paper's session-44 incident):

- `(success && verified)` → **Certified**. Nothing else ever promotes.
- `consecutive_fails ≥ 3` → **Withdrawn**.
- Withdrawn is **absorbing**: outcome counts alone never leave it. 100
  unverified successes later, the tier stays Withdrawn (stats climb,
  the tier decides). One fresh validator pass re-certifies.

Certification admission (`record_certified`) is wired into the Stage 2
tuning loop (`tune_against_fixture`): every verifier-passed fix lands in
memory as Certified, alongside its verified-pool entry. `HealPipeline::
report_outcome` carries the same `verified` flag; `EvolveRecorder`'s CLI
path keeps `record_outcome` (unverified context — the current call sites
are compile-gate reverts, where failures withdraw regardless) plus
`record_outcome_verified` for future full-validator callers.

Pre-Issue-021 memory files decode as Heuristic (`#[serde(default)]`) — the
honest tier for unverified history.

## LatentQuery

```rust
pub struct LatentQuery {
    pub lint_name: String,
    pub lint_name_hash: u64,   // pre-computed for O(1) BM25
    pub direction: Direction,  // pre-computed embed(lint_name, buggy_code)
}
```

The query pre-computes its embedding + hash at construction time so the
retrieval hot path is pure arithmetic (no hashing in the loop).

## `success_rate_for` (Issue 008)

```rust
pub fn success_rate_for(&self, lint_name: &str, fixed_code: &str) -> f32
```

Looks up the actual success rate for a specific `(lint_name, fixed_code)`
pair. Returns `0.5` (neutral) if no trajectory exists for this pair.

This was added in Issue 008 to fix a G7 mechanism bug: the heal pipeline
previously hardcoded `success_rate = 0.5` for drafter candidates, which
masked fail feedback — a repeatedly-failed drafter pick kept its neutral
0.5, tying with the correct candidate (also 0.5) and winning ties. Now
the pipeline looks up the actual trajectory stats, so after a fail the
drafter candidate's rate drops to 0.0 and the correct candidate wins.

Linear scan over all trajectories — acceptable for POC scale, worth
profiling at production volume.

## Persistence — two backends, one active (Issue 018 → Issue 038)

`LatentFixMemory` itself is backend-agnostic; persistence is chosen by
feature build (**one active backend per run — never both**, Issue 038):

| Build | Backend | Location | Why |
|---|---|---|---|
| `latent_retrieval` | **neuron-db store (`TrajStore`)** — the ONE backend | repo-local `.refine/trajectories.kv` (encrypted WAL; root = `RIIR_HEAL_TRAJ_ROOT` → first path's repo root → cwd) | per-key Pod rows, incremental puts via `CommitBatch`, WAL encryption at rest, run-attributed timestamps |
| default (light) | JSON envelope (Issue 018) | `~/.cache/riir-clippy/memory.json` (`RIIR_HEAL_MEMORY` override) | zero heavy deps for light builds |

### The store backend (Issue 038 — commit `e4ce8ba`)

[`src/traj_store.rs`](../../src/traj_store.rs): `TrajStore::open` → per-key
rows keyed `(lint, shape)`; `TrajRowDoc` carries the text triple + counters +
tier + `last_run_id` + `last_seen_at` (unix secs — 037-A1's cadence line
reads it). `record_applied` = sighting (texts + run stamp, no counters);
`record` = outcome verdict (the 037-T2 matrix); `import_row` = verbatim
migration. **One-time migration** on the first persisting run
(`--write || --verify`): envelope present + store empty → every bucket
imported verbatim → envelope renamed `memory.json.migrated` (the once-only
fence; idempotence pinned by `tests/store_backend_e2e.rs`). Per-key puts
shrink the concurrent-session loss surface from whole-file last-writer-wins
to the same-bucket stats race.

**Durable EMA learn rows (Issue 037 E3, commit `efa254c`):** `TrajStore::record`
also feeds `learn:{lint_key}` rows (`LearnRowDoc`) through the same
`evolve::ema_step` core the in-process layer uses — cross-process durable
learning, reported by `--memory-stats` (per-rule counters + the decay line).
Learn rows are invisible to the trajectory surface.

Observability: `--memory-stats` prints envelope-parity detail PLUS the
cadence line the envelope could never answer (`activity: last verdict
… | distinct active days N`); `scripts/self_evolve_audit.sh` §1 is
store-first (per-repo row summaries via `--dump-trajectories`) with the
envelope parse as fallback.

### The JSON envelope (Issue 018 — light-build fallback)

```rust
pub fn to_json(&self) -> serde_json::Result<String>
pub fn load_json(&mut self, json: &str) -> serde_json::Result<()>
pub fn save(&self, path: &Path) -> std::io::Result<()>
pub fn load(&mut self, path: &Path) -> std::io::Result<()>
```

The v1 JSON envelope (`{"version":1,"trajectories":[…]}`) round-trips
losslessly:

- `direction` f32 components serialize via ryu's shortest round-trip form
  → bit-identical recovery.
- **Merge-on-load**: each file trajectory is `insert`ed — identical
  `(lint, buggy, fixed)` triples keep the LIVE stats (newer observations
  win); new triples append with their persisted stats.
- `NotFound` = fresh memory `Ok(())` (first run); malformed JSON or an
  unknown `version` = `InvalidData` (never aborts a heal run — the CLI
  warns + continues fresh).
- Retrieval weights are config, not state — never serialized.

The `cargo refine` bin resolves the path: `RIIR_HEAL_MEMORY` override, else
`$XDG_CACHE_HOME`/`~/.cache`/`%LOCALAPPDATA%`/temp → `riir-clippy/memory.json`.

## See also

- [`heal_pipeline.md`](heal_pipeline.md) — how the memory is composed
- [`evolve_recorder.md`](evolve_recorder.md) — the oracle-split consumer (Issue 018)
- [`../03_drafter/embedder.md`](../03_drafter/embedder.md) — the BLAKE3 embedder
- [Bench 026](../../.benchmarks/026_evidential_status_axis_goat.md) — the Issue 021 GOAT (the honest A/B finding: equal-similarity distinct-tier pairs are not constructible; the within-subject delta is exactly the 0.1 trust gap)
