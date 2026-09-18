# Issue 123 — TUNA trial-credit client surfaces: ack v4, the 075 soft gate's second plane, footer + `--info` readouts

**Status:** OPEN — client half LANDED 2026-09-18 (ack v4 + gate + footer + `--info`, all additive with byte-identical degradation pinned by tests); **server wire half LANDED 2026-09-18** (riir-dapps `90cfb0e` — Issue 097, resolved + removed under the noise-reduction rule; the record lives in the dapps HISTORY "Proposal 006 LANDED" section + [Proposal 006](../riir-dapps/.proposals/006_tuna_trial_credit_and_token_conditions.md) B2): `/burn` + `/history` carry the zipped `tuna_balance_micro`/`tuna_expires_unix` pair (spendable-only) + the `charged_tuna_micro`/`charged_kat_micro` split; the worker manifest now pins `kat_tuna`); **THE CLAIM FLIP LANDED 2026-09-18** (riir-dapps `c6d2e9c` + the coherence follow-up `789d18f`: `/account/claim` draws TUNA — D1 executed; `/tuna/genesis` ops route; `/balance` + `/history` `granted` report the unified claim predicate `grant_claimed`, so `ensure_grant_flow` + the welcome gate converge for trial accounts; **devnet has the reservoir — genesis run 2026-09-18T05:07:40Z, live claim/replay/balance/stats verified** `5e32d72`; apex + mainnet.ai code-only, claims 503 fail-closed until the owner runs `/tuna/genesis` there). **DEVNET IS LIT**: the fleet's machines point at devnet, so `/burn` acks now carry TUNA truth — the footer TUNA line + the gate's second plane render for any account holding trial credit, byte-identical everywhere else. Remaining here, all CLIENT-side: the **plane-split run line** (needs `charged_*` on the parsed response struct in riir-kat + the footer render here), **post-D1 grant-bar honesty** (live on devnet now — new users hold TUNA, no KAT grant; the footer's `grant` bar still projects the 100 KAT tier for everyone; the TUNA line renders beside it so both tanks are visible; the honest fix needs a claim-plane signal server→client), **TUNA bar scale** (the claim now uses the same `cfg.free_grant` the bar projects — accurate for fresh claims; multi-tranche accounts via `tuna_credit` can exceed it), and **light-up on apex/mainnet** (owner deploy + one genesis curl each). Until a server sends the fields, every consumer reads `None` and the behavior is unchanged.

## Why

Proposal 006 (riir-dapps, ACCEPTED 2026-09-18) ships TUNA — the trial
credit: heals only, non-transferable, expires. The owner asked how the
CLI surfaces it, and the review found one real bug in the plan of record:

**The 075 soft gate would refuse a trial user who is NOT depleted.** The
gate's predicate is KAT-only (`last_balance_micro ≤ 0` + never
contributed). Post-B3, `apply_burn` draws TUNA first — a trial user
holding 47 TUNA with a 0 KAT balance can still heal, but the gate reads
the KAT leg alone and refuses. That is the Issue-094 shape (a property
asserted where nobody checks it) caught before the plane exists.

## Landed (client half, additive, forward-compatible)

1. **Burn ack v4** (`riir-kat/src/kat_protocol_burn.rs` +
   `kat_burn_client.rs`): `BurnReportAck` + `KatBurnAck` gain
   `tuna_balance_micro: Option<i64>` and `tuna_expires_unix: Option<u64>`,
   parsed/written DEFENSIVELY (absent or wrong-typed → `None` — a
   pre-TUNA server keeps parsing). `BURN_ACK_VERSION` 3 → 4; the loader's
   upgrade arm widened to EVERY older version (all migrations since v1→v2
   were additive-field only; upgrading in place keeps the burn total and
   the gate's synced balance where the old reset dropped both). Pinned by
   the v4-parse arm + the real-v1-file-upgrades-silently arm.
2. **Soft gate second plane** (`riir-kat/src/kat_soft_gate.rs`):
   `SoftGateInput` gains `tuna_balance_micro`; a server-reported positive
   TUNA balance LIFTS the refusal. `None` is "no trial-credit truth" —
   never an invented zero — so pre-v4 acks keep the KAT-only posture byte
   for byte. The refusal prose names the exhausted trial credit when the
   server said so (`Some ≤ 0` is a fact, not an absence).
3. **Footer TUNA line** (`src/kat_footer.rs`): `TunaLine { balance,
   grant scale, expires, now }` renders a 4th line between grant and sync
   — `tuna  ⣿…  47.2 of 100 TUNA left · expires 2026-10-18 (12d) · trial
   credit — heals only` — only when the ack carries TUNA truth; `None`
   renders nothing (the 3-line footer, byte-identical, pinned by the
   existing shape tests). Shared date helper `ymd_from_unix`
   (civil-from-days, no chrono dep).
4. **`--info`** (`src/bin/kat_info/mod.rs`): the wallet view gains the
   `tuna:` position line (from `/history`, additive parse) and the
   soft-gate ALLOW line names the trial credit. Both degrade to nothing
   on a pre-TUNA deployment.

## Still open (needs the server half — riir-dapps Issue 097 / Stage B)

- [ ] **Light-up**: a post-B server must SEND `tuna_balance_micro` +
      `tuna_expires_unix` on `/burn` and `/history` (and `/sync`'s
      ingest-reported balance if it echoes one). Everything here is
      wired; the fields are simply `None` until then.
- [ ] **Plane-split run line**: the footer's run line (`8,791 tok →
      0.0088 KAT`) cannot say WHICH plane funded the charge — the client
      only knows its own token count. Needs the burn response to report
      the split (`charged_tuna_micro` / `charged_kat_micro`); that split
      is also the mid-run TUNA→KAT handoff moment the owner asked about
      (Proposal 006 Q1) and it happens in the SERVER's `apply_burn`
      (TUNA-first, oldest-expiry-first), invisible to the client until
      the response names it.
- [ ] **Post-D1 grant-bar honesty**: after the claim flip (D1), new users
      hold TUNA at claim and NO KAT grant — yet the footer's `grant` bar
      projects the 100 KAT free tier for everyone. The server must say
      which plane a user's grant was (or the ack must carry a claim-kind)
      before that bar is honest for trial users. Interim: the TUNA line
      renders beside it, so the trial user sees both tanks.
- [ ] **TUNA bar scale**: the footer gauges TUNA against
      `FREE_TIER_GRANT_MICRO_KAT` (the caller's projection). When B2
      fixes the real claim size, re-aim (ideally: the server reports it).

## Verdict on the owner's Q1/Q2 (recorded)

- Q1 `--fix`: yes — plane-aware tail, landed as far as the client can
  reach without server truth (balance + expiry now; run-split open).
- Q2 `--info`: yes — wallet TUNA line + gate view, landed; the two-door
  conversion counters (healed vs expired) ride B4's sweep and should
  surface in the same `/history` read when they exist.

## Refs

- riir-dapps Proposal 006 (`.proposals/006_tuna_trial_credit_and_token_conditions.md`)
- riir-dapps Issue 097 — the server wire half
- riir-dapps Issue 075 / 080 — the gate + its server-truth rule (the v3 precedent this v4 follows)
