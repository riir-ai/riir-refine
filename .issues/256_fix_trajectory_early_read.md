# Issue 256 — fix-trajectory early-read: census the "will-verify-fail" predictor before any wiring

**Status:** Open — consumer POC filed 2026-11-24 from `../katgpt-rs/.research/692_RET_Macrostate_Closure_Audit.md` (arXiv:2605.09294); rides the SHIPPED Plan-292 FutureBehaviorProbe substrate + 301 direction-bank pattern; UNBLOCKED 2026-11-25 (katgpt-rs 1244 T1+T2 LANDED — `c80cf19e7` + `8adb663a9`, Bench 994 G2 PASS). **Numbering note:** allocated as 255 concurrently with `.issues/255_fixer_classes45_used_binding_and_with_capacity.md` (the two-sessions-one-highwater class); renumbered 255→256 at the rebase — 255 keeps the first-pushed holder.
**Lease closed:** zed-8895fce6@m3, 1795544400, was expires 2026-11-27T18:00:00+00:00 — lapsed unlanded past T0 (the instrument + census record landed 11-25; T0c stayed open; m3 dark all day, no renewal)
**Lease closed:** zed-600a1d37@4090 (claim record: zed-600a1d37@4090, epoch 1796289700, was expires 2026-11-27T23:00:00+00:00) — the claimed scope (T0c(b) capture + census re-run + record 168) LANDED same session; fold per the landed-batch-lease class.

## What

The fix lane's verify loops cost minutes on large workspaces. Fix attempts already persist trajectories WITH outcomes (`EvolveRecorder`, `.refine/trajectories.kv`, adoption_gate outcome rows) — a labeled dataset for the prediction-feature pattern (Research 267: detection ≠ prediction). RET's contribution here is the early-read schedule (read at ~10% trajectory depth, predict the turn-level outcome) plus the macrostate VALIDATION step. No behavior change at P0 — a census only.

- [x] T0 (census, report-only): EXECUTED 2026-11-25 — verdict NO VERDICT, typed. The per-attempt ring (the state sequence T0 needs) has zero dumps on this box (24 stores scanned); the aggregate stores carry 4 fails in 1019 attempts (0.4% negative support). Instrument landed (`examples/fix_trajectory_census` + `src/fix_trajectory_census.rs` + 7 gates) + the census record (`.benchmarks/165`); re-runs itself when the capture (T0c) lands.
- [ ] T1 (macrostate validation): ARMED but unserved — the audit arm (41-D state summary vs 32-D embed microstate, `katgpt_core::closure_audit`) ships in the census instrument behind `fix_trajectory_census`; it refused `NoWindows` on today's stores for the same reason T0 did (no rings). Re-runs with T0 on the captured corpus.
- [ ] T2 (pre-registration): ONLY if the census clears its recall floor (pin: recall ≥ the false-abort-priced bar BEFORE reading — the regression class is aborting a fix that WOULD have verified; price the false-abort cost against the verify-loop minutes saved per the calibration-gate rules), write the T3 pre-registration.
- [ ] T3 (wiring, gated): decline/abort lane behind a feature flag with the GOAT battery + kill switch; `--verify-args` compatible; never in the release set until its own gates pass.
- [x] **T0c (capture, follow-up): BOTH LEGS EXECUTED — bench 168 (composed mints, in-process driver: 172 rings/574 attempts, ends-bad 3.0%) + bench 169 (the production CLI on real workspace code: reflexer-vessel copies × 30 hand-authored real-code-shape breakages — 19 rings/24 attempts, 0 ends-bad, typed SINGLE CLASS standalone; pooled AUC 0.994/recall 1.000 over 5 positives). 169's structural finding: the negative class is UNREACHABLE by deliberate capture — the driver's candidate set is playbook-bounded (rustc suggestions feed the digest, never a candidate), so uncovered real breakage takes the MISS path (38 miss files this capture) with no ring; the ladder-exhaustion endings the census's positive class needs arise only in ORGANIC rings.** T2 stays GATED; re-open = organic accumulation (zero-cost, auto-dumping) or playbook-growth landings that exhaust ladders on real shapes (the miss queue's own authoring loop is the intake).
**Lease closed:** zed-600a1d37@4090 (claim record: zed-600a1d37@4090, epoch 1796289700, was expires 2026-11-27T23:00:00+00:00) — the claimed scope (T0c(b) capture + census re-run + record 168) LANDED same session; fold per the landed-batch-lease class.

## Proof path

P0 census numbers ARE the gate: the lane lives or dies on held-out early-read recall at the priced false-abort bar, one frozen test read, echo-gate discipline on any synthetic augmentation (Research 692 §5 trigger (b), the C5 recipe v3 App. L.3). **In-repo prior analysis this fills: riir-refine Research 168 (Latent Programming Horizons → Probe-Gated Verification, arXiv:2607.05188) — its probe-gated-verification gap statement is this census made actionable.**

## Related

Research 692 §4 · Research 267 (detection-vs-prediction; the substrate) · Research 301 (direction-bank + cascade pattern) · katgpt-rs `.issues/1244` (closure audit) · `.refine/trajectories.kv` + adoption_gate outcome rows (the dataset).
