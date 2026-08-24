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

- [ ] **T1 — Pod row schema** `FixTrajectoryRow` (`#[repr(C)]`, bytemuck Pod):
      the bucket fields (lint_name_hash, buggy/fixed BLAKE3 commitments,
      direction `[f32; 8]`, success/fail counts, consecutive_fails, tier,
      **last_seen_at u64**) — variable-length strings stay out of the Pod
      (hashes only; full text recoverable from the corpus/fixtures) OR a
      sidecar row family — decide by what `record_outcome`'s exact-match
      actually reads (it needs `fixed_code` bytes → sidecar text row per
      bucket, Pod-index row for stats).
- [ ] **T2 — backend swap in `recorder.rs`**: `LocalKvStore::open_dir` at
      `.heal/trajectories.kv` (repo-local, matching the rustc_errors
      convention) OR keep the cache-dir path — decide: repo-local wins for
      multi-repo adoption audit (037-A2 reads `.heal/` presence per repo);
      cache-dir wins for cross-repo skill stats. **Pick repo-local** (the
      adoption gate wants per-repo evidence).
- [ ] **T3 — one-backend rule**: extend the existing "ONE active backend per
      build" rule to the fix memory (neuron store under `latent_retrieval`,
      JSON envelope otherwise — deprecation note in the envelope path);
      migration on first open (envelope present + store absent → import
      once, rename envelope to `.migrated`).
- [ ] **T4 — audit + dump**: `self_evolve_audit.sh` section 1 reads
      `cargo heal --dump-trajectories` (backend-agnostic) instead of parsing
      memory.json; keep JSON parsing as fallback.
- [ ] **T5 — GOAT**: G1 round-trip (record → reopen → identical retrieval
      ranking incl. trust weighting), G2 no parse on hot path, G3 lib tests
      both feature states, G4 alloc check on the admit path, G8 the
      037-E2 flow end-to-end on the new backend (T2-of-037 wired outcomes
      landing as Certified rows).
- [ ] **T6 — 037 cross-link**: mark 037-T3 superseded (timestamps land here);
      037-T2 (wire outcomes) is orthogonal and stays in 037.

## Non-goals

- No DAG/AST storage (deferred, consumer-gated above).
- No remote/shared store (single-developer local Warm tier only).
- No public-binary behavior change (Proposal 003 still blocked by 037).
