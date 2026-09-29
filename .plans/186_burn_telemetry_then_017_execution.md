# Plan 186 — Burn-telemetry fix first, trainer/fixer staking answers, then Proposal 017 execution

**Status:** AGREE — Claude consult rounds 1–2 (REVISE, AGREE + nits applied); OWNER GO GATEs pending (2026-09-29)

Refs: `../riir-dapps/.issues/107` (epoch burn rollup — the fix-first bug) · `../riir-dapps/.issues/108` (trainer reward path + two-role POC) · `.proposals/017_buffered_reward_restake.md` (AGREE, r3) · `../riir-dapps/.plans/036` (the P1 deferral note) · Plan 181 A1-side (P015 dated hold) · 017 caveat 10 (live-data scale reality).

- **Sequencing verdict:** the burn-telemetry fix lands BEFORE the 017 owner gate. Everything the gate reads — the soak counters, G7-S, the dao guard, the emission-cap transition — reads epoch burn, and epoch burn reads a fake 0 until Issue 107 closes. A soak run during the gap would measure a fake zero and poison the default-ON decision. (Claude round 1 concurred on ordering; six defects on the fix shape and gate teeth — applied, see Issues 107/108 for the corrected fix shape.)

## Phase A — fix the instrument (riir-dapps, Issue 107)

- [ ] A1: epoch burn rollup — **two sources** (`EpochRow.burn_total + Σ buckets` over wall-hour buckets whose hour index falls in the epoch range; `EpochRow.burn_total` untouched — it is the staking distribution's base); populate the committed epoch telemetry `burn_micro` at write-time + epoch-page `burn_epoch_micro`; golden vector updated (schema change).
- [ ] A2: attribution rule — a bucket belongs to the epoch containing `bucket × 3600` (hour INDEX, not seconds); stalled-crank straddle is attributed whole; test-pinned both sides of a boundary + a stall-across-boundary fixture + the fast-settle vs epoch-settle ordering within the boundary hour.
- [ ] A2b: `mint_micro` rolls up in the SAME change (same zeroed source; the emission-cap decision reads mint) — sum each bucket's pool/staking/treasury split (buckets settle under `EpochParams::DEFAULT`).
- [ ] A2c: pre-fix epochs read UNMEASURED, not zero — new `live_mask` bit `LIVE_BURN_ROLLUP` is SET ON EVERY ROW where the rollup is real (readers check the bit per row; hash-chained pre-fix rows can never be rewritten and simply carry it unset); dao guard / G16 / G7-S / explorer honor the bit.
- [ ] A2d: explorer read-time rollup for PAST epochs (optional, transparency gain, no chain rewrite) — the buckets for 2956–2959 exist; the epoch page may compute a read-time rollup for them labeled UNMEASURED-in-committed-row (the committed hash-chained zeros stand; the label carries the truth).
- [ ] A3: reconcile — EXACT identity, zero tolerance: `Σ watermarks == Σ EpochRow.burn_total + Σ bucket.burn_total + Meta.burn_pending`; plus Σ `burn_micro` over closed epochs (from the fix epoch) + open-epoch buckets == lifetime burn; added as a `tokenomics_reconcile.sh` source.
- [ ] A3b: first-run reds are FINDINGS, not noise — if the identity fails because some path advances a watermark without adding to `burn_pending` (e.g. forgiven or free-tier burn), the red is adjudicated as a finding and fixed at the source; NO tolerance is added to make it pass.
- [ ] A4: deploy devnet SHADOW-FIRST — new values rendered beside the old, decisions still on the old, until the before/after diff of what the dao reads is recorded and G16/emission guards are confirmed advisory on devnet; then flip; live-verify epoch ≥2960 renders real burn/mint; 017's burn-reader liveness counter green. The flip itself is a RECORDED DECISION (who flipped, on which diff) in this plan's log — it is a live guard input changing, never an unlogged side effect.

## Phase B — record the role answers (no payout code; Issue 108)

- [ ] B1: fixer — NO new machinery (recorded in Issue 108): a fixer's KAT already arrives as mining credits from `--fix`-driven batches through the single `pay_due_rewards` leg; 017's account-level restake/buffer applies to it unchanged.
- [ ] B2: replay/verify trainer — payout class exists (P015 endorsement), disarmed by a dated hold (`replay_cut_bp = 0`); arming preconditions on record (pool scale ~10⁶×, canary bound ~20× short). **OWNER GATE A: ratify cut + canary bound as ONE act, or keep disarmed.** Prerequisites that land BEFORE the ratification: (1) the install-time interlock (refuse `replay_cut_bp > 0` while the bound is below the recorded minimum) so Gate A cannot be done as two; (2) the bond / one-owner-K-accounts answer for the K=2 quorum (the B3 bond line applies to this lane too — named here so Gate A cannot skip it).
- [ ] B3: distill trainer — no wire; oracle constraint recorded (self-reported training outcomes = free mint, the decstat lesson); three candidate designs priced in Issue 108, with the **slashable-bond requirement** (spot-check or quorum deters only when getting caught costs more than cheating pays — a bond separate from 017's restake, which must stay a yield posture, never a penalty surface) and the one-owner-K-accounts question (also owed for the replay lane's K=2 before Gate A). Design-1 numerics: GPU recomputation needs a tolerance band or a CPU-reference oracle. **OWNER GATE B: pick a design (or none) before any wire is built.**
- [ ] B4a (NOW, before Gate 1 — verifies B1 instead of assuming it): the fixer + mining two-role checks on the m3 account — both lanes' credits on ONE account, ONE payout leg, ONE restake posture slot, coherent receipts + stake head, explorer renders both without double-count.
- [ ] B4b (after any trainer role arms): the trainer-lane rows join the same POC (the owner's m3-SSH-wallet two-role test).

## ⛔ OWNER GO GATE 1 — approve Proposal 017 implementation

Condition (ALL named): Phase A green AND deployed past shadow (A1–A4, the A3 identity green on devnet) + Phase B recorded (B1 verified by B4a, B2/B3 with their gates pending owner) + the 017 caveat-10 counters now measurable. Nothing in 017's Phase 0–3 starts before this gate.

## Phase C — Proposal 017 implementation (after Gate 1; phases per 017)

- [ ] C1 (017 Phase 0): `FLOOR_LIQUID_MICRO` calibration from the charge ledger — anchored by real burn data (fleet 171 KAT / m3 21 KAT lifetime as the sanity band). ⚠ the band is SMALL — the ~10⁶× pool-scale caveat (Bench 013 F6) applies to it too: treat the band as an order-of-magnitude prior, never as a calibrated rate.
- [ ] C2 (017 Phase 1): ledger — posture side row + buffered split + ONE receipt-split function (`rows_resolved += n` once; `remainder > 0 ⇒ Finalized`) + `kat:stakeopseq:` + the `vest_ticks ≤ lockup_ticks` install refusal + golden/rollback/adversarial arms (017 GOAT G1/G8-S lists are the task list).
- [ ] C3 (017 Phase 2): worker — `/stake/restake` + `/stake/redeem` (op-seq, per-route prefixes) + contract pins + `/stake/info` op-seq field.
- [ ] C4 (017 Phase 3): CLI — `stake --restake off|buffered` + `stake --redeem` + `--info` render (resolved posture, buffer, projected split, op seq) + wire rows (riir-kat).
- [ ] C5: deploy devnet; soak begins — duration ≥ 2 × `LOCKUP_TICKS`; counters per 017 Ships-now incl. emission-cap headroom + burn-reader liveness + posture counts + forgiven-burn volume + dormant-restaked share.
- [ ] C5b: default-posture teeth — a test pins the shipped default posture to `off` (a C2/C4 change that flips the default reds until Gate 2 lands the flip commit), and the first-payout nudge is a **flag** (`devnet-only` enforced, not a sentence) with a test.

## ⛔ OWNER GO GATE 2 — default-ON flip

Condition (all three, per 017 Deferred): soak green over ≥ 2 lockups ∧ the dormancy decision for restaked abandoned accounts (017 caveat 3) ∧ counsel sign-off (017 caveat 4 — default yield breaks the "default experience promises nothing" hygiene; the nudge flag flips in the same commit as the posture default).

## Verification

Phase A: explorer epoch page shows real burn; reconcile gate green; dao telemetry nonzero. Phase B: records only (issues 107/108 are the record). Phase C: 017's own GOAT gates (G1/G8-S goldens, G7-S simulator, G3 no-regression, G2 perf, G4 typing) + devnet soak readout. Docs-only tasks (B1–B3) carry no cargo.

## Honest scope notes

- This plan does NOT arm any payout (Gate A/B are owner acts recorded in Issue 108) and does NOT implement 017 before Gate 1 — the plan exists so the fix-first ordering survives session handoffs.
- The trainer POC B4b may land after Gate 1; it is not a Gate-1 precondition (B4a IS one — it verifies B1). The default-ON decision (Gate 2) should see B4b's result if a trainer role armed by then.
