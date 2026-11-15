# Plan 381 — opal transcription batch (nominal Batch 367): measurement-integrity + agent-harness classes

**Status:** STAGED — plan-first per the distill law; the cargo half awaits a batch window (the claim row lands when it starts, the push is the arbiter). Verdict: riir-refine Research 413 (YES B− corpus / lane-intel NONE / product-coverage: sec agent-harness family +2 axes). Source: `xingjian-bai/opal` @ `97762859ba5e2527e5c6f63d9890dfbab860d51f`, MIT, v0.3.10 alpha — re-clone + re-pin at T0.
**Nominal batch number:** 365 — RE-DERIVE at batch start (362/363 are claimed by staged ThreeRT/FORGE lanes; the score_history tail is the counter).

## Phase 0 — re-verification

- [ ] T0.1 re-clone `xingjian-bai/opal` shallow into `.raw/opal`, pin the new tip sha + license in the batch record; delete the clone at close.
- [ ] T0.2 re-verify every Research 413 quote at the new pin (integrity.py docstring + constants; boundary.py header + categories + `check()` never-raises; wrapper.py `_apply` three-way-merge comment; recovery.py `_take_snapshot` size warning + README tiered-exactness lines; waiting.py `tick` paused-exclusion + the "Why" docstring). A moved quote re-grades its rule before any corpus edit.
- [ ] T0.3 dedup re-run at 3+ vocabularies over rust_perf + sec corpus dirs (the five pattern sets from Research 413 §Dedup) — dedup-at-mining binds.
- [x] T0.4 adversarial panel run 2026-11-15 (sessions 77b9d9ef no-GD / a9cf61cf model-based); outcomes recorded in Research 413 §Adversarial panel; the model-based advocate's static-distillate finding FILED as riir-train Issue 692 (fusion idea, proof path named).

## Phase 1 — fixtures (original Rust, four-arm discipline per law: buggy/fixed/all-safe/tampered)

- [ ] T1.1 R1 `eval-window-state-restore-roundtrip-detection`: a measurement-harness fixture — ring of strided f64 samples over a guarded state vector (seeded offset, stride coprime with the logical shape), an eval window at a NON-announced step, the detector algebra (threshold × median-typical + floor-of-norm; SEEN 0.5 / BACK 0.25; isolation both sides). Arms: cheater scale-and-restore → round trip FLAGGED; persistent weight-swap → informational jump, NOT flagged; legitimate high-lr spike → NOT flagged (not isolated); sustained oscillation → NOT flagged; PLUS the panel falsifier arms: out-and-back slower than the ring window → template never completes (not-detected, documented); move smeared across sub-threshold updates → undetected (documented); warm-restart legit jump → no false flag. Execute standalone BEFORE entry.
- [ ] T1.2 R2 `agent-code-static-boundary-diagnostics-never-gate`: a scanner fixture over a proposed-source string — category taxonomy (io / dynamic-eval-analog / introspection / protected-type-mutation), alias resolution, diagnostics-never-gate posture (the scan result is recorded, acceptance is the caller's). Arms: io-touching proposal flagged, benign proposal clean, malformed source → empty diagnostics + never panics, deliberate-looking-but-benign use → flagged (a flag can be benign — the observation is recorded either way); the vocabulary-bound regime noted (assembled/dynamic-construction evades — the category vocabulary, not the walker, is the bound). Execute standalone.
- [ ] T1.3 R3 `stale-proposal-merges-against-shown-base`: apply fixture — proposal computed at base B applied onto current C after a revert of B±; naive wholesale apply resurrects reverted content (buggy arm), delta-vs-shown-base apply does not (fixed arm); the panel kill arm: a full-file rewrite degenerates the merge to wholesale write and carries the stale view — named as the regime condition (protection holds where edits are local). Execute standalone.
- [ ] T1.4 R4 `capacity-gated-exact-undo-loud-degrade`: swap-with-rollback fixture — snapshot byte estimate vs budget; under budget: exact restore verified byte-identical; over budget: skip + LOUD named event + degrade to report-not-undo; the retained-fallback trial-window shape in the notes arm; regime condition carried: the snapshot fully captures the mutable state (a first-step failure touching state outside the snapshot — RNG streams, cursors — is outside the guarantee). Execute standalone.
- [ ] T1.5 R5 face-or-drop decision (the batch's one authoring swing): if kept, a rate-estimate fixture excluding self-caused pauses (median-of-5) + the falsifier arms (bimodal latency hides the tail; accepted rewrites stale the estimate); if dropped, the drop reason recorded in the entries header with the self-caused-vs-external axis named (kin: wall-clock-ungated, sustained-load-drift).

## Phase 2 — corpus entries

- [ ] T2.1 rust_perf `measurement` entries (R1, R4, R5-if-kept) with source verbatim, regime conditions, honest not-detected/evasion lists carried IN-rule, kinship adjudications both directions (R1 vs spread-unstable + wall-clock-ungated + box-state; R4 vs B274 admission polarity; R5 vs wall-clock family).
- [ ] T2.2 sec InputValidation entries (R2, R3) with the agent-harness family adjudications in-rule (R2 vs untrusted-content-into-prompt code-axis + spillway anti-authority posture; R3 vs externally-authored-edits-by-id + the escalation-chain stale-answer instance).
- [ ] T2.3 `pins_manifest --write` in the same landing commit; count pins one-landing (AGENTS + README + .docs heads + health views + the narrative heads).

## Phase 3 — gates + landing

- [ ] T3.1 fixtures standalone-compiled AND executed pre-entry (Phase 1 receipts in the batch record).
- [ ] T3.2 gates at an isolated `CARGO_TARGET_DIR`: default lib, full-feature lib, clippy `-D warnings` ×4 postures, fmt gate, retrieval floors (no re-pin expected — rust_perf + sec retrieval lanes unchanged in shape).
- [ ] T3.3 freeze/score-bench owed ONLY if clippy_lints corpus moved (it does not — assert, don't run).
- [ ] T3.4 research note + queue snapshot + this plan checked off in the landing commit; binary rebuild check owed (rust_perf + sec ARE in the shipped release set — follow the dc51dadb binary-lane path).
- [ ] T3.5 commit + push on develop; `rm -rf .raw/opal` in the same close; df line in the summary.

## Proof path (the ungoat law — named before any corpus edit)

Falsification arm: every fixture's tampered arm must RED (the detector/scanner/apply/undo must distinguish its classes); pre-registered gates: fixtures pass standalone, retrieval floors hold, counts move by exactly the landed rule count. No benchable GOAT is claimed (metric-mismatch class — Research 413 §GOAT screen); every entry lands first-class suggestion-only `NotBenchable` with `bench_spec: None`.
