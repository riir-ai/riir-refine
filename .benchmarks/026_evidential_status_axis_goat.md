# Bench 026 — Issue 021: evidential-status axis for `LatentFixMemory`

**Date:** 2026-08-16.
**Issue:** `021` (resolved + removed).
**Research:** [049](../.research/049_Long_Horizon_AI_Research_Evidential_Status.md)
(arXiv:2608.11195 — the retelling-bias failure mode).
**Scope:** per-trajectory evidential tier (`EvidenceTier`: Certified /
Heuristic / Withdrawn), absorbing-Withdrawn transition discipline,
trust-weighted retrieval, and the full-validator certification admission —
the retelling-bias defense for the self-adaptive track. Schema + ranking
semantics only; no new learning mechanisms (per the issue's non-goals).

## What shipped

1. **`EvidenceTier`** (`src/memory.rs`) — `#[repr(u8)]` enum on
   `FixTrajectory` (one byte, zero-alloc):
   - `Certified = 0` — admitted by a full validator (the
     `adaptive_drafter` verified-pool admission rule: a real
     `clippy_verify` re-run proving the lint resolved, or the Stage 2
     tuning loop's verifier pass).
   - `Heuristic = 1` — syn-parse only / unverified history. Constructor
     default AND the serde decode default: pre-Issue-021 memory files
     (`#[serde(default)]` on `tier` + `consecutive_fails`) decode as
     Heuristic — the honest tier for unverified history. Back-compat test
     pins a hand-written old-format v1 envelope.
   - `Withdrawn = 2` — retracted after `WITHDRAW_CONSECUTIVE_FAILS = 3`
     consecutive post-hoc failures. **Absorbing** (the riir-chain
     slashing-monotonicity pattern): no sequence of outcome counts leaves
     it; only `(success && verified)` — a fresh full-validator pass —
     exits (re-certification).
2. **`consecutive_fails: u32`** on `FixTrajectory` — the streak driver,
   reset on any success (honest bookkeeping: `success_count` still climbs
   while Withdrawn holds — stats record, tier decides).
3. **Signature change** — `LatentFixMemory::record_outcome(lint, fixed,
   success, verified)` (the issue's required signature): the tier
   transition is a pure function of `(tier, success, verified,
   consecutive_fails)` — no wall-clock, G1 determinism holds.
   `HealPipeline::report_outcome` gains the same `verified` param.
   `EvolveRecorder::record_outcome` keeps its 4-arg CLI shape, delegating
   to a new `record_outcome_verified` with `verified: false` — the
   current `cargo heal` call sites are compile-gate reverts (failures),
   where the flag is behaviorally irrelevant (a failure withdraws
   regardless of verification). `src/bin/heal.rs` untouched.
4. **Certification admission** — `LatentFixMemory::record_certified(lint,
   buggy, fixed)`: insert-if-absent + `(success=true, verified=true)`.
   Wired into `tune_against_fixture` (both Stage 1 + Stage 2 paths): every
   verifier-passed fix now also lands in memory as a Certified trajectory
   (previously the tuning loop only fed the adaptive pool — memory never
   learned from certifications).
5. **Trust-weighted retrieval** — `RetrievalWeights.trust` (default 0.2):
   `combined = 0.4·sigmoid(dot) + 0.6·bm25 + 0.2·trust_prior` where the
   prior is Certified 1.0 / Heuristic 0.5 (the untried-neutral
   convention) / Withdrawn 0.0. Additive per-entry bounded term —
   sigmoid-family, never a softmax across entries (house rule). Design
   constraint, measured: at 0.2, a different-lint Certified entry
   (≤ 0.4+0.2) can never outrank a same-lint Heuristic one (≥ 0.6+0.1) —
   trust breaks ties + small edges, it must not override relevance.
6. **Withdrawn exclusion** — `retrieve_into` skips Withdrawn before
   scoring: a retracted fix is never emitted as a suggestion. Withdrawn
   entries stay in memory + reachable via `iter()`/`get()` for the
   adaptation loop's failure analysis.

## The honest A/B finding (T2)

The issue's original A/B shape — "a Certified entry and a Heuristic entry
with equal similarity" as two separate entries — is **not constructible
through the real API**, and the first test run proved it:
`record_outcome` matches by `(lint_name, fixed_code)`, so two trajectories
sharing that pair are ONE stats bucket — certifying "the second one"
actually certifies the first. Exactly-equal similarity between two
distinct-tier entries cannot arise from real record paths (the embedder
keys on `(lint, fixed)`; identical keys = identical embedding = same
bucket).

The falsifiable A/B was redesigned into two constructible halves:

- **Within-subject (equal similarity, exact magnitude):** the SAME entry
  queried before vs after certification — the only changed variable is the
  tier. Measured delta = **exactly 0.1** (0.5 prior gap × 0.2 weight;
  assertion tolerance 1e-5). If the trust axis were unwired, delta = 0 →
  FAIL.
- **Cross-entry (small-edge regime):** a Heuristic entry whose latent
  score is STRICTLY higher than the Certified one's, found by runtime
  search over 200 enumerated fixed codes (the hash-based embedder has no
  locality, so "slightly more similar" must be searched, not composed).
  With the edge < the trust gap (0.1 / 0.4 latent-weight = 0.25
  latent-score units), the Certified entry still ranks #1. Self-adapting
  across embedder changes; asserts the candidate exists so a silent
  no-op cannot pass.

Withdrawn exclusion: a memory whose only match is Withdrawn returns ZERO
suggestions; mixed memory surfaces only non-withdrawn entries; the
Withdrawn entry stays retrievable via `iter()`.

## GOAT

| Gate | Status | Evidence |
|---|---|---|
| T1 schema + transitions | ✅ PASS | 7 new memory tests: admission rules table, exhaustive absorbing check (every `(success, verified)` × streak regime from Withdrawn), end-to-end 100-unverified-successes-stay-Withdrawn + one-recertification-exits, JSON round-trip incl. tier + streak |
| T2 falsifiable A/B | ✅ PASS | within-subject delta exactly 0.1 + cross-entry small-edge flip + Withdrawn never suggested (3 tests; the first run FAILED on the stats-bucket subtlety and drove the redesign — the gate is load-bearing, not decorative) |
| T3 no-regression | ✅ PASS | 415 default / 429 `clippy_verify` lib tests (was 408); goat_gate_004 G1/G3/G4/G5 pass, G2 release-only pass (re-run); stage2_goat 5/5; full CI guard `ci_feature_guard.sh` all layers (668 combined-state tests, incl. the Issue 017 retrieval floors + Issue 020 annotation floor) |
| T4 alloc (review-audit) | ✅ PASS | the tier check is a `u8` compare + one f32 FMA per entry in the retrieval scan — zero allocation by construction; `retrieve_into_is_alloc_free_after_warmup` re-passes (capacity stable across 10 repeat calls); `heal_is_alloc_free_after_warmup` re-passes |
| T5 back-ref + removal | ✅ PASS | Research 049 §2.3 close-out note; issue file removed per noise-reduction rule |

## Honest scope notes

- **`record_reviewed` stays memory-only** (Heuristic) under `--verify` —
  a rustc pass proves non-breakage, not lint resolution (Bench 023's
  semantics; certification requires the full validator).
- **Scope-of-validity (the issue's stretch goal) deferred** — the u8
  fixture-family bitset would complicate the schema review for marginal
  POC gain; the issue marked it "defer if it complicates", and it does.
  The failure mode it guards (cross-family retrieval of a
  family-validated fix) remains open; re-open with a concrete
  cross-family failure instance.
- **Tuning-loop certification ordering** — `record_certified` is called
  before `adaptive.insert` (borrow then move — one allocation, not two).
  If memory already holds a Heuristic trajectory with the same
  `(lint, fixed)` but different buggy code, the certification lands on
  the existing bucket (first match) and the freshly-inserted duplicate
  triple stays Heuristic — retrieval then favors the Certified bucket.
  Correct under the stats-bucket semantics; documented here so the
  behavior is a decision, not a surprise.
- **Game-side provenance axis** (Research 049 §2.3's note on NPC memory
  consolidation) remains a separate substrate-first pass — out of scope
  here per the issue's non-goals.
