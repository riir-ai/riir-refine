# Issue 148 — the traj-mining contribution claim: name the lane as the (future) refine-rule-suite training source + add the missing label fields

**Status:** OPEN — moved from [Issue 147](147_decision_cpi_t0_client_and_escalation_poc.md) at the T0′ closure (2026-10-05, the pre-registered NO-GO disposition: the tier closed, the contribution lane does not). **Standalone value: the corpus feeds self_evolve/Elo + mining regardless of the (now-closed) Decision-CPI tier.**

## What (Proposal 021 §3 carrier-corrected at verdict r2)

The refine-rule SUITE's training corpus — the artifact that would eventually
un-refute the decision-scorer class (Bench 098's negative prior) and any
future re-open of the escalation tier — is the **EXISTING traj mining lane**,
not a new lane and not decstat:

- `src/kat_sync_payload.rs`: gate-passed TrajStore rows —
  `shape → fix → lint_key`, `RedactionGate`-checked, BLAKE3-committed,
  account-key-signed, mining-consented. A code-shape→fix→lint corpus is
  exactly what a suite mint consumes.
- **decstat/fixstat CANNOT carry contexts** ("Stats only, never questions,
  never answers, never option text" — their redaction law + golden bytes).
- **YIELDS to P019 Move 1 on the lane's code** (the one-twist law: refine
  mines heal spans; rethink mines decstat receipts).

## T0′'s measured corpus state (the re-open bar, recorded 2026-10-05)

- Fleet (devnet `/mining/adoption`, live): rows_contributed **7,513**
  (SHIKUWA 7,210 / m3 301+2), rows_credited **533**,
  rejected_or_quarantined **6,980** (92.9% non-credited; duplicate-vs-quarantine
  not publicly decomposable — distinct usable server-side contexts bounded
  `[533, 7,513]`).
- m3 distinct pushed (lint, shape) contexts: **301** (`kat_sync_state.json`
  `synced_rows`; only riir-refine carries a sync state — the other local
  stores were never pushed).
- **The lane is CONVERGED, not idle**: m3's sync state was rewritten
  2026-10-05 04:31 (the auto-sync tail after a live `--fix` run) with
  rows_contributed unchanged at 301 — "duplicate shapes earn nothing".
  Last fleet contributions: 2026-09-16 (SHIKUWA) / 2026-09-18 (m3).
- Against the bars: **533 clean = 19× off the 10⁴ un-refute target**;
  7,513 loose = 75% of target. Growth requires new machines/repos/rules,
  not more re-healing.

## Tasks

- [ ] **T-a The claim**: record in the lane's module docs
      (`src/kat_sync_payload.rs`) that its gate-passed rows are the
      designated training source for any future refine-rule suite mint —
      one paragraph, no code. (This is the T0.8 "NAME the lane" half;
      zero-behavior-change.)
- [ ] **T-b The label fields**: add the fields a suite mint needs and the
      lane lacks — per-span **the rejected candidate set** (the top-k
      rules the matcher proposed that were NOT applied), recorded at
      heal time where the candidate set is already in hand. Schema note:
      the sync payload's rows are content-addressed + idempotent — a new
      field is a payload-version decision (see `PAYLOAD_CHANGELOG`), not
      a silent append; coordinate with the receiver's `MIN_SYNC_PAYLOAD_VERSION`
      posture. YIELDS to P019 Move 1 if that move lands first.
- [ ] **T-c The growth lever (analysis, then maybe code)**: the corpus is
      converged on m3 — growth needs NEW contexts. Cheapest levers, in
      order: (1) the never-pushed local stores (riir-game-sdk 12M,
      riir-ai 6.2M, riir-dapps 708K, katgpt-rs 724K, … — Aug-era, consent
      + policy per repo required); (2) new corpus batches (each batch's
      new rules carry new spans); (3) fleet growth. Record which levers
      are owner-consent-gated (per-repo `.refine-sync-policy` + mining
      consent are already the mechanism — nothing new to build).

## Non-goals

- No escalation client, no tier re-open (147 is CLOSED; the re-open
  condition — corpus at the 10³–10⁴ clean-context class — is recorded
  there with the headroom baseline: subset 16/141 @ θ=0.05, top-1 0.750
  vs overall 0.894, +2.8pp ceiling).
- No decstat/fixstat changes (stats-only by their redaction law).

Refs: Proposal 021 §3 + §8 (T0.8) · Issue 147 (the T0′ record) · Bench 098 /
Issue 125 (the negative prior) · Research 206 (the flywheel) · P019 Move 1
(the lane-code yield).
