# Plan 128 — P015 Phase 3: the replay-endorsement econ-sim arm (gates any activation ≠ 0)

**Status:** IN PROGRESS — filed 2026-09-16 on owner GO (the P015 Phase-1
ledger + routes and Phase-2 client are LANDED and inert at `replay_cut_bp = 0`;
Phase 3 is the only remaining gate between them and a live devnet cut).

Upstream design: [Proposal 015](../.proposals/015_replay_endorsement_reward_class.md)
(§"G7-style econ arm"). Phase 1: dapps `kat` ledger plane + the
`/mining/replay/*` worker surface (G2 measured 58 µs/verdict release). Phase 2:
the `cargo heal replay` client (riir-kat `kat_replay_client` + the
clippy_verify-gated engine, Plan 127). This plan is the **simulator half**;
owner ratification of the devnet `replay_cut_bp` consumes its output table.

## Why a sim and not a number

The endorsement reward pays miners for REPLAYING other miners' contributed
rows through the real clippy oracle and counter-signing the verdict. Three
failure modes are priced in the proposal and none has a measured magnitude:

1. **Miner dilution** — every bp routed to replay endorsements is a bp not
   paid to first-party heals; the cut trades replay coverage for heal yield.
2. **Verifier break-even** — a replay costs real oracle seconds; if the
   endorsement rate underpays the oracle time, no rational node replays and
   the quorum silently starves (the prove leg stays SEAM-ONLY forever).
3. **Self-pairing** — a miner replaying its OWN submitted rows both ways
   (submit + endorse) mints without independent verification. The design's
   conservative-agreement rule narrows but does not eliminate the hole; the
   sim must PRICE it (work-share an adversary can capture before detection),
   not argue it away.

## Tasks

- [ ] T1 (P0) — Sim harness: deterministic replayer over a seeded corpus of
      synthetic miner populations (honest / lazy / self-pairing adversary /
      colluding pairs), driving the REAL `KatLedger` epoch-settle math
      (riir-dapps `kat` crate — consume, do not fork; the dao-sim precedent:
      G5 consumed the shipped `EpochParams`). Outputs per (cut, rate,
      adversary-share) cell: miner-dilution curve, replay-coverage %,
      endorsement take-home vs oracle-time cost, self-pairing capture.
- [ ] T2 (P0) — The sweep: `replay_cut_bp ∈ {25, 50, 100, 200}` × canary rate
      ∈ {2%, 5%, 10%} × adversary work-share ∈ {0, 10%, 30%, 50%}. Every cell
      records the three proposal failure modes. G-gate: a cell PASSES only if
      coverage ≥ 80% at break-even-or-better take-home AND self-pairing
      capture ≤ 2× an honest endorser's take-home.
- [ ] T3 (P1) — Break-even calibration: measure the REAL oracle cost of one
      replayed span (the clippy_verify lane, ms/span on this box + the 4090
      class) so "take-home ≥ oracle time" is measured, not assumed. Record in
      the bench doc; the sim consumes the number.
- [ ] T4 (P1) — Canary-catch economics: the design routes a canary rate
      through the lease; sweep the false-accept cost (a lazy endorser passing
      canaries by luck) vs canary rate — pick the smallest rate whose
      per-epoch lazy-catch probability ≥ 0.99.
- [ ] T5 (P0) — The verdict table + ratification memo: one page — the
      recommended `replay_cut_bp`, the canary rate, the priced self-pairing
      residual, and the monitoring counters the worker should expose so a
      live divergence from the sim is observable. This table is what the
      owner ratifies; mainnet stays 0 regardless (ceremony-gated).
- [ ] T6 (P2) — Bench doc: `.benchmarks/` record with the full cell table,
      G2-consistent harness notes, and the honest caveats (synthetic corpus
      shape, oracle-cost portability).

## G-gates (mirror P015 §econ arm)

- G1 (correctness): the sim's ledger math matches `KatLedger`'s settle
  bit-for-bit on a fixed seed (consume the real crate — no parallel money
  math).
- G2 (econ): the sweep produces ≥1 PASSING cell at each sweep axis's lowest
  adversarial setting, and NO cell passes at 50% adversary share (if one
  does, the cut design has a hole — stop and re-design, do not ratify).
- G3 (no-regression): existing `dao-sim` / ledger chaos suites untouched and
  green.
- G4 (alloc): harness is report-side; no ledger-plane changes (Phase-1
  surface unchanged).

## Non-goals

Mainnet activation (ceremony-gated, owner-only); changing the
conservative-agreement rule (sim prices it, P015 owns it); any
`replay_cut_bp` write — the genesis flip is the owner's ratification commit,
not this plan's.
