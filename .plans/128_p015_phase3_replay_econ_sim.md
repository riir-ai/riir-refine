# Plan 128 — P015 Phase 3: the replay-endorsement econ-sim arm (gates any activation ≠ 0)

**Status:** T1/T2/T4/T5/T6 COMPLETE (2026-09-15, dapps `47799dc` —
[Bench 013](../../riir-dapps/.benchmarks/013_replay_econ_sim.md); the sim
lives in riir-dapps `tests/kat_econ_sim.rs` §`replay_econ` beside the real
ledger math it consumes). T3 measured (0.17 s/item cold, M3). The verdict:
**CONDITIONAL GO with two measured constraints** — the identity threshold
(cuts < 2 223 bp are structurally inert; the sweep grid was RE-BOUND by
this finding) and the canary ladder (per-lease filtering weak; 10% is the
only arm passing the zero-work tolerance at every share). G2's second
clause FIRED and is re-worded to the ≤5% zero-work-capture tolerance —
owner ratifies the re-wording WITH the cut. Remaining: T3.2 (the owner's
ratification itself) + T3.3.

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

- [x] T1 (P0) — Sim harness LANDED (`replay_econ` module; the split
      mirror PINNED against `record_proven_reward_with_replay` on a real
      in-memory ledger — the pin caught the harness never setting the
      config's cut, exactly the parallel-money-math hazard G1 names).
- [x] T2 (P0) — The sweep RAN, grid RE-BOUND by measurement: the
      proposal's cut band {25..200} bp priced to ZERO (the floor-integer
      identity threshold — see F1), so the live grid is
      {2 223, 3 333, 5 000, 6 667} bp × canary {2%, 5%, 10%} × adversary
      {0, 10%, 30%, 50%}. 40/48 cells pass; the 8 failures are exactly the
      2%-canary × ≥30%-share cells (zero-work capture > 5%).
- [x] T3 (P1) — Break-even measured: ~0.17 s/item cold / 0.08 s warm
      (cargo+rustc startup dominated); a 16-item lease ≈ 2.7 s. At the
      live devnet pool (32 230 µKAT/weight) the per-item payout is a
      ~$3e-6 subsidy — the lane is a contribution, not a living (F6).
- [x] T4 (P1) — Canary ladder measured: per-lease catch at 16 items is
      27.6%/56.0%/81.5% at 2/5/10% — the epoch-level zeroing is the real
      defense; 99% epoch catch needs 15/6/3 leases. The 10% arm is the
      recommendation (survival 0.5%).
- [x] T5 (P0) — The memo is Bench 013 §T5: **recommended v1 posture =
      keep `replay_cut_bp = 0`** until the pool backs a meaningful
      per-weight payout AND the 10% canary is set; if armed: cut ≥ 2 223 bp,
      canary 10%, accept the ≤2× self-pairing residual (lease-disjointness
      is the designed fix if not), devnet only. Monitoring counters named
      (replay_rows_confirmed / verifier_weight_total / canary_catches /
      zero_events).
- [x] T6 (P2) — dapps `.benchmarks/013_replay_econ_sim.md` (the full
      table, the G2 re-wording record, the caveats).

## G-gates (mirror P015 §econ arm)

- G1 (correctness): the sim's ledger math matches `KatLedger`'s settle
  bit-for-bit on a fixed seed (consume the real crate — no parallel money
  math).
- G2 (econ): ≥1 passing cell at the lowest adversarial setting — HELD.
  The second clause ("no cell passes at 50% adversary share") FIRED AS
  WRITTEN and is re-worded by the measurement to the ≤5% zero-work-capture
  tolerance: share scales LINEARLY (self-pairing ≤ 2× by the split's own
  conservation; collusion survival is share-independent), so no cliff
  exists at 50% — the original clause is unsatisfiable by any cut that
  rewards honest work. The re-worded gate DISCRIMINATES (40/48 pass; the
  failures are the 2%-canary ≥30%-share cells). Owner ratifies the
  re-wording with the cut (Bench 013 §G2).
- G3 (no-regression): existing `dao-sim` / ledger chaos suites untouched and
  green.
- G4 (alloc): harness is report-side; no ledger-plane changes (Phase-1
  surface unchanged).

## Non-goals

Mainnet activation (ceremony-gated, owner-only); changing the
conservative-agreement rule (sim prices it, P015 owns it); any
`replay_cut_bp` write — the genesis flip is the owner's ratification commit,
not this plan's.
