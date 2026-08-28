# Plan 042 — KAT T2.4: score-bench promotion gate + EvidenceTier wiring

**Status:** COMPLETE — landed (all gates green; commit follows)

## Goal

Corpus-candidate rules arriving from the network (the service plane's
proven-mining channel, served later via the lease / a candidates segment)
must pass the frozen score-bench BEFORE entering the live corpus — and
their EvidenceTier must transition correctly:

- **Promote** iff the trial (base + candidates) is not-worse on
  `heal_rate` AND not-worse on `created_rate` (canonical INTEGER totals —
  same denominators by construction) + every candidate's `fixed_code`
  parses (fragment parse — the poisoning defense: broken/new-lint fixes
  never enter).
- **Reject** any regression or unsafe candidate (audit row; never enters).
- **Withdrawn (absorbing)**: a promoted rule whose post-hoc bench regresses
  → Withdrawn + removed from the overlay + can NEVER return via promotion
  (stricter than the trajectory tier's re-certification exit — network
  rules already had their quorum chance; §Mining #4's poisoning defense).

## Design

- `src/kat_promotion.rs` behind `all(kat_service, score_bench)` (both
  existing opt-in surfaces; zero new features, zero new deps).
- Candidate schema v1: NDJSON rows `{schema_version, rule_id, domain
  (clippy|perf), buggy_code, fixed_code, provenance}` — mirrors the lease
  `PatternEntry` triple (rule_id, buggy, fixed) + versioning + provenance.
- Trial mechanism: `score_bench::heal_fixpoint_with(source, &[(&str,&str)])`
  — a candidate TEXTUAL lane (verbatim substitution, the L1 verbatim-tier
  semantics) composed with the proposer fixpoint in one bounded loop;
  `run_bench_with(oracle, fixtures, candidates)` runs the frozen fixtures
  under the lane. `heal_fixpoint`/`run_bench` delegate with `&[]`
  (byte-identical default — existing tests pin it).
- Registry `.heal/kat_promotion.json`: promoted (Certified) entries +
  withdrawn content-hash/rule-id blacklists (absorbing). Audit
  `.heal/kat_promotion_audit.jsonl` (append-only, the score_history
  precedent). Corrupt registry = gate REFUSES (can't promote around a
  broken blacklist).
- Trial runs NEVER touch `.benchmarks/score_history.jsonl` (the gate
  computes + reports; only `--score-bench` appends history) — pinned by
  e2e test.
- Gate flow: schema/absorbing pre-filters → bench(base) → batch trial →
  pass ⇒ promote all / fail ⇒ per-candidate attribution. Post-hoc sweep:
  bench(base) vs bench(base + overlay lanes) → regressors withdrawn.
- Overlay compose (retrieval): promoted triples MERGE with the run overlay
  (leased-or-compiled-in per domain, dedup by rule_id) in `heal.rs` —
  skipped under `--corpus-epoch` pinning (exact reproduction) — via the
  pure `merge_triples`. Promotion never mutates the frozen base corpus
  (pinned by test on frozen corpus bytes).
- Network seam (documented, NOT built — service-side): the lease manifest
  can carry a candidates segment; a delivered segment is written to a
  local candidates file + this gate runs before the rules reach the
  overlay. The service plane (T1.5/T2.x) owns delivery.

## Tasks

- [x] T1 `score_bench`: `heal_fixpoint_with` + `run_bench_with` (delegating
      defaults; default path byte-identical — score_bench tests 12/0+1i
      unchanged)
- [x] T2 `src/kat_promotion.rs`: schema + parse/validate (strict v1 NDJSON;
      fragment-parse + dup-content + domain guards), registry
      (EvidenceTier reuse; absorbing Withdrawn on content-hash AND id
      blacklists; corrupt registry = gate refuses), integer-totals verdict
      (`not_worse` canonical, fail-closed on n_ran==0), gate (pre-filters:
      ParseUnsafe / WithdrawnAbsorbing / **ConflictingMatchKey** — the
      shadowing hole found live: same match key + different fixed ⇒ only
      the first fires in a batch, the rest would ride the pass; rejected
      wholesale pre-bench) + post-hoc sweep, audit + registry persistence,
      pure overlay merge helpers
- [x] T3 `src/lib.rs` module gate `all(kat_service, score_bench)`
- [x] T4 `src/bin/heal.rs`: `--candidates <path>` standalone mode (sweep →
      gate → persist → audit → report; honest exit codes) +
      promoted-triples merge into the run overlay via
      `run_overlay_with_promotion` (non-pinned; corrupt registry → warn +
      continue WITHOUT network rules) + reheat merge + usage/args/mode-guard
- [x] T5 `tests/kat_promotion_g2p.rs` (4 e2e tests, real oracle, one-fixture
      subset, ORACLE_LOCK serialization — the oracle work root is not
      concurrency-safe, found live) + 11 module unit tests (schema,
      absorbing, canonical comparison, merge, base-immutability, overlay
      pin semantics, lane determinism ×3, conflicting keys)
- [x] T6 CI Layer 5c pin (`--test kat_promotion_g2p`)
- [x] T7 Validation: default lib 712/0 · dual-feature lib 980/0 (kat_promotion
      11/0) · all-features lib 1421/0/13i · all-features check 0 errors ·
      clippy 0 new warnings in touched files (healer-first: 3 verified
      edits landed, 1 healer false-positive reverted — the redundant_closure
      that stripped a `&PathBuf→&Path`-adapting closure at heal.rs:2112,
      caught by all-features compile, invisible to the verify's feature set
      — the gate-coverage trap, documented below) · live CLI e2e smoke on
      the full 35-fixture bench (benign PROMOTE + poison REJECT
      created_regression with per-candidate attribution; registry + audit
      persisted; score_history untouched) · `--suggest` consumes the
      promoted rule as the top clippy suggestion through the merged overlay

## Found live (recorded for the next healer batch)

- **ConflictingMatchKey** — the batch-trial shadowing hole (design bug
  caught by the e2e test before it shipped): candidates sharing a match
  key with different fixeds make only the first fire; the rest ride the
  batch pass. Now a deterministic pre-bench rejection.
- **The oracle's shared work root is not concurrency-safe** — parallel
  gate tests raced cargo into degenerate equal-zero verdicts (clean
  sweep). Tests serialize via `ORACLE_LOCK`.
- **Healer divergence re-hit**: `redundant_closure` stripped a
  deref-coercing closure (E0631 under `rustc_errors`) — the class the
  corpus already documents; the verify feature set must cover the gated
  code or the compile gate proves nothing about it.

## Deferrals (honest)

- The network candidate channel itself (lease segment delivery) —
  service-side, T1.5/T2.x territory. This plan ships the consumer gate.
- Trainer-side quorum verifier — T2.2 territory.
- Per-domain kernel/sec candidates (the bench lanes are clippy+perf;
  kernel candidates need a kernel bench — loud schema error for now).
- `decline_correctness` is REPORTED by the gate but not gating (the
  pre-registered bar is heal + created + parse-safety; a candidate lane
  touching clean fixtures with clean text moves no axis).
