# Issue 256 — fix-trajectory early-read: census the "will-verify-fail" predictor before any wiring

**Status:** Open — consumer POC filed 2026-11-24 from `../katgpt-rs/.research/692_RET_Macrostate_Closure_Audit.md` (arXiv:2605.09294); rides the SHIPPED Plan-292 FutureBehaviorProbe substrate + 301 direction-bank pattern; UNBLOCKED 2026-11-25 (katgpt-rs 1244 T1+T2 LANDED — `c80cf19e7` + `8adb663a9`, Bench 994 G2 PASS). **Numbering note:** allocated as 255 concurrently with `.issues/255_fixer_classes45_used_binding_and_with_capacity.md` (the two-sessions-one-highwater class); renumbered 255→256 at the rebase — 255 keeps the first-pushed holder.
**Claimed:** zed-8895fce6@m3, 1795544400, expires 2026-11-27T18:00:00+00:00

## What

The fix lane's verify loops cost minutes on large workspaces. Fix attempts already persist trajectories WITH outcomes (`EvolveRecorder`, `.refine/trajectories.kv`, adoption_gate outcome rows) — a labeled dataset for the prediction-feature pattern (Research 267: detection ≠ prediction). RET's contribution here is the early-read schedule (read at ~10% trajectory depth, predict the turn-level outcome) plus the macrostate VALIDATION step. No behavior change at P0 — a census only.

- [ ] T0 (census, report-only): fit the shipped DoM/direction probe on EARLY fix-attempt state (first ~10% of the attempt's logged state) and measure held-out recall/precision of "will verify fail"; paired read with LB95 bars; no fixer behavior touched.
- [ ] T1 (macrostate validation): run `closure_audit` (katgpt-rs `.issues/1244`) over the chosen fix-trajectory summary — is the early state a closed macrostate of the attempt's dynamics? An `Undecidable` verdict kills the lane cheaply (the summary is insufficient; find a better one or stop).
- [ ] T2 (pre-registration): ONLY if the census clears its recall floor (pin: recall ≥ the false-abort-priced bar BEFORE reading — the regression class is aborting a fix that WOULD have verified; price the false-abort cost against the verify-loop minutes saved per the calibration-gate rules), write the T3 pre-registration.
- [ ] T3 (wiring, gated): decline/abort lane behind a feature flag with the GOAT battery + kill switch; `--verify-args` compatible; never in the release set until its own gates pass.

## Proof path

P0 census numbers ARE the gate: the lane lives or dies on held-out early-read recall at the priced false-abort bar, one frozen test read, echo-gate discipline on any synthetic augmentation (Research 692 §5 trigger (b), the C5 recipe v3 App. L.3). **In-repo prior analysis this fills: riir-refine Research 168 (Latent Programming Horizons → Probe-Gated Verification, arXiv:2607.05188) — its probe-gated-verification gap statement is this census made actionable.**

## Related

Research 692 §4 · Research 267 (detection-vs-prediction; the substrate) · Research 301 (direction-bank + cascade pattern) · katgpt-rs `.issues/1244` (closure audit) · `.refine/trajectories.kv` + adoption_gate outcome rows (the dataset).
