# Issue 038 — Unify self-evolve memory onto the neuron-db Warm-tier store (kill the JSON envelope)

**Repo:** riir-clippy
**Filed:** 2026-08-24 (owner prompt: "log should store in neuron-db local kv? no need to parse back and forth? ... latent first with dag and ast")
**Status:** OPEN — feeds [Issue 037](037_e2e_self_evolve_adoption_proof.md) E-gates; **supersedes 037-T3** (timestamps); should land before [Proposal 003](../.proposals/003_binary_only_distribution.md) executes (avoids migrating user data post-release); [Issue 039](039_elo_ruliology_seamless_loop.md) (ELO + evidence-grounded ruliology) builds its derived views on this store — its T1 reserves elo fields on the T1 row schema
**Branch:** develop

## The ask

`LatentFixMemory` persists as a whole-file JSON envelope
(`~/.cache/riir-clippy/memory.json`, `RIIR_HEAL_MEMORY` override): every CLI
run loads → parses → merges → serializes → rewrites the ENTIRE file
(`src/recorder.rs` L262–287, `src/bin/heal.rs`). Owner's call: store it in
neuron-db `LocalKvStore` instead — no parse back-and-forth, latent-first,
machine-readable-only is fine.

## Substrate-first grep verdict (the substrate ALREADY EXISTS — this is a consolidation, not new build)

1. **The dual-backend precedent is in-repo.** The `rustc_errors` domain
   (`--fix-compile`) ALREADY records to a neuron-db store
   (`.heal/trajectories.kv`) under `latent_retrieval` builds —
   `heal.rs` L583–593 (`.heal/trajectories.kv` resolution +
   `--dump-trajectories`), L665–688 (durable store, shape-conditional
   ordering), L739–743 ("ONE active backend per build: the neuron-db store
   under `latent_retrieval`, memory.json otherwise — never both"). This
   issue extends that backend to the clippy/rust_perf fix memory — same
   store, same rule.
2. **The architectural twin exists in riir-agents.** `SharedContextLog::
   with_kv(LocalKvStore)` (riir-ai `shared_context.rs` L346–375) — one Pod
   per key, encrypted Warm tier, BLAKE3 commitment verified on round-trip
   (`warm_tier_write_through_round_trips` test). `TaskStore` is the same
   pattern. Copy it.
3. **Boundary-clean.** `BOUNDARY.md` already allows
   `riir-neuron-db = { optional = true, default-features = false }` —
   no contract change; `latent_retrieval` already pulls it transitively.

## Design verdict (owner's three axes, answered)

| Axis | Verdict | Reason |
|---|---|---|
| **LocalKvStore backend** | **GO** | Kills whole-file parse/serialize/rewrite; per-bucket Pod rows (`key = hash(lint_name, fixed_code)`) get incremental puts via `CommitBatch` (1 fsync per run, Plan 323 B3); WAL encryption at rest; timestamps land in the Pod row for free (037-A1's memory half). |
| **Latent-first** | **YES on the index side, NO on exact-match storage** | The `direction` field already rides each bucket; retrieval-side latent index already exists (`NeuronDbRetriever` / `DenseEmbedIndex`, `draft/corpus_index.rs`). But `record_outcome` matches by EXACT `(lint_name, fixed_code)` — buckets must stay raw Pod bytes. Human readability moves to `--dump-trajectories` (already shipped for the rustc_errors backend) + the audit script. |
| **DAG / AST** | **DEFER with a named consumer** | AST span storage: `AstChunker` exists but no trajectory consumer needs node-level storage today. DAG (fix-attempt chains): the named consumer is Proposal 002's deep-fusion E (SweTrajectoryFreezer) — file when E3-learn demands chains, not before. |

**Concurrency bonus (measured pain):** today two concurrent agent sessions
last-writer-wins the WHOLE file (merge-on-load only helps sequential runs —
8,236 buckets lost on a race). Per-key puts shrink the loss surface to the
same-bucket stats race.

## Tasks

> **037-T2 pre-landed the schema half of T1/T2 (2026-08-24):**
> `TrajRowDoc` now carries `last_run_id` + `last_seen_at` (serde-defaulted,
> pre-T2 rows decode), `record()` takes the run id, and the `--fix
> --verify` verdict events already land in `.heal/trajectories.kv`
> (transitional dual-record with memory.json — T3 below retires the
> envelope half). T1's `FixTrajectoryRow` design should build on these
> fields rather than re-deriving them.

- [x] **T1 — row schema** (decided: NO separate Pod split — the row IS
      `TrajRowDoc`): the text-carrying per-key JSON doc row the store
      already ships (037-T2 pre-landed `last_run_id` + `last_seen_at` on
      it). The issue's own decision rule — "what does record_outcome's
      exact-match actually read" — answers `fixed_code` bytes, so the text
      must live in the row; a separate Pod-index/text-sidecar split would
      duplicate the row family with zero consumers (per-row serde ≠ the
      whole-file parse the issue kills; the admit path is per-run, not
      per-tick). 039-T1's elo fields land here later as serde-defaulted
      additions.
- [x] **T2 — backend swap in `recorder.rs`**: landed in `src/bin/heal.rs` (the
      CLI is the only envelope consumer — the recorder itself is
      backend-agnostic). Under `latent_retrieval` the repo-local store is
      opened UP FRONT (`.git`-walk root resolution: `RIIR_HEAL_TRAJ_ROOT` →
      first path's repo root → cwd) and is the ONE backend: apply events →
      `TrajStore::record_applied` (sighting: texts + run stamp, no counters);
      verify verdicts → `record` (as 037-T2 shipped). The envelope is
      neither loaded nor saved when the store is active (in-session recorder
      calls remain — they feed EMA; only persistence swaps). **Repo-local**
      picked per the issue (037-A2 per-repo evidence).
- [x] **T3 — one-backend rule**: store under `latent_retrieval`; envelope
      otherwise (light builds — by design, one stderr line only on the
      store-FAILED fallback). **Migration** fires on the first PERSISTING
      run (`--write || --verify`): envelope present + store empty → every
      bucket imported verbatim (`import_row`) → envelope renamed
      `memory.json.migrated` (the once-only fence; idempotence pinned by
      e2e). Unreadable envelope → skip + warn, never fatal.
- [x] **T4 — audit + dump**: `self_evolve_audit.sh` §1 is store-first (per-repo
      `.heal/trajectories.kv` scan via the binary's backend-agnostic
      `--dump-trajectories`, tiers/outcomes per repo — doubles as the
      037-A2 per-repo evidence line) with the envelope parse as fallback
      + a `.migrated` tombstone note. `--memory-stats` reports the
      migrated-backend state.
- [x] **T5 — GOAT**: G1 round-trip + once-only migration + store-only
      persistence pinned by `tests/store_backend_e2e.rs` (2 e2e tests,
      real binary + real cargo/clippy oracle: heal → Certified rows →
      envelope migrated + never rewritten → migration never re-fires;
      verified stats-verbatim imports). G2 no whole-file parse: by
      construction (envelope untouched when the store is active — asserted
      `!envelope.exists()` post-run). G3 lib tests green in BOTH feature
      states (default 660/0; full-feature clean, clippy 0 warnings both).
      G4 admit-path alloc: per-event bounded (per-row serde + put — same
      class the rustc_errors domain already GOATs). G8 the 037-E2 flow
      end-to-end on the new backend: **the first e2e test's oracle line —
      "3 confirmed resolved (Certified)" — IS the gate passing.**
- [x] **T6 — 037 cross-link**: 037-T3's superseded note stands (timestamps
      land in `last_seen_at`); 037-E2's substance is now provable on the
      store backend via `--memory-stats` / the audit script §1.

## Non-goals

- No DAG/AST storage (deferred, consumer-gated above).
- No remote/shared store (single-developer local Warm tier only).
- No public-binary behavior change (Proposal 003 still blocked by 037).
