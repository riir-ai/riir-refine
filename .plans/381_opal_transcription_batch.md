# Plan 381 — opal transcription batch (nominal Batch 367): measurement-integrity + agent-harness classes

**Status:** LANDED — Batch 367 (2026-11-16, zed-m3-opal-b367@m3): rust_perf 353 → 356 (entries_82) + sec 74 → 76, all five rules suggestion-only NotBenchable; fixtures 26/26 standalone pre-entry; gates green ×4 clippy postures + lib 4408/0 + pins_manifest --write + prose agree; the claim row discharged by the landing. Verdict: riir-refine Research 413 (YES B− corpus / lane-intel NONE / product-coverage: sec agent-harness family +2 axes). Source: `xingjian-bai/opal` @ `97762859ba5e2527e5c6f63d9890dfbab860d51f`, MIT — pin re-verified STILL the tip at the batch window (the `misc/` → `opal/misc/` path move disclosed, quotes byte-verified at the new path).
**Claimed:** zed-m3-opal-b367@m3, epoch 1794804218, expires 2026-11-18T04:43:38+00:00
**Lease closed:** zed-m3-opal-b367@m3, epoch 1794804218, expiry 2026-11-18T04:43:38+00:00 (discharged by the landing commit)
**Nominal batch number:** 365 — RE-DERIVE at batch start (362/363 are claimed by staged ThreeRT/FORGE lanes; the score_history tail is the counter).

## Phase 0 — re-verification

- [x] T0.1 re-clone `xingjian-bai/opal` shallow into `.raw/opal`, pin the new tip sha + license in the batch record; delete the clone at close. (Pin re-verified STILL `97762859`; the tree's `misc/` moved to `opal/misc/` — path-form only; clone removed at close.)
- [x] T0.2 re-verify every Research 413 quote at the new pin (integrity.py docstring + constants; boundary.py header + categories + `check()` never-raises; wrapper.py `_apply` three-way-merge comment; recovery.py `_take_snapshot` size warning + README tiered-exactness lines; waiting.py `tick` paused-exclusion + the "Why" docstring). A moved quote re-grades its rule before any corpus edit. (All quotes byte-exact at `opal/misc/*` + wrapper.py:859-860 + README:164-165/138/141.)
- [x] T0.3 dedup re-run at 3+ vocabularies over rust_perf + sec corpus dirs (the five pattern sets from Research 413 §Dedup) — dedup-at-mining binds. (Five sets re-grepped at the landing HEAD: zero owners; the entries_28/18 'restore' hits disclosed as cache-slot prose.)
- [x] T0.4 adversarial panel run 2026-11-15 (sessions 77b9d9ef no-GD / a9cf61cf model-based); outcomes recorded in Research 413 §Adversarial panel; the model-based advocate's static-distillate finding FILED as riir-train Issue 692 (fusion idea, proof path named).

## Phase 1 — fixtures (original Rust, four-arm discipline per law: buggy/fixed/all-safe/tampered)

- [x] T1.1 R1 `eval-window-state-restore-roundtrip-detection`: (b367 rust_perf fixtures F1, 8 tests, 15/15 green) — the gate caught THREE fixture-model defects en route: an index-convention off-by-one (opal's row-index convention vs the change-index one), the oscillation arm modeling the A2 cheat shape instead of the contiguous-stretch law (a gapped square wave tied to eval times IS the A2 cheat and legitimately flags), and a smear arm ignoring the absolute floor (re-modeled sub-floor with the witness at the aggregate move).
- [x] T1.2 R2 `agent-code-static-boundary-diagnostics-never-gate`: (b367 sec fixtures F2, 7 tests) — plus the gate caught the `.`/`::` root-split miss and a Diagnostic field shadowing; the gate-posture (flags.is_empty as verdict) arm added to the detector after the corpus-buggy fixture initially did not fire.
- [x] T1.3 R3 `stale-proposal-merges-against-shown-base`: (b367 sec fixtures F3, 6 tests incl. the kill arm + the tampered base-less delta) — the gate caught the delta-apply silently no-oping on a reverted-missing file (opal's dict-assignment semantics: a changed-vs-base name INSERTS) and a no-revert proposal omitting an unchanged file (the delta law reads absent-from-proposal as a deletion — opal's exact semantics).
- [x] T1.4 R4 `capacity-gated-exact-undo-loud-degrade`: (b367 rust_perf fixtures F4, 4 tests incl. the RNG-stream regime condition + the tampered silent-skip RED).
- [x] T1.5 R5 face-or-drop decision (the batch's one authoring swing): KEPT — `self-caused-wait-excluded-from-rate-estimate` (the adversarial panel's lag-governor falsifiers strengthened the keep case; the self-caused axis genuinely unowned; b367 rust_perf fixtures F5, 3 tests incl. the bimodal falsifier demonstrated + the bounded window asserted).

## Phase 2 — corpus entries

- [x] T2.1 rust_perf `measurement` entries (R1, R4, R5-if-kept) with source verbatim, regime conditions, honest not-detected/evasion lists carried IN-rule, kinship adjudications both directions (R1 vs spread-unstable + wall-clock-ungated + box-state; R4 vs B274 admission polarity; R5 vs wall-clock family). (Category: measurement-honesty — the corpus's 15th category name; entries_82.)
- [x] T2.2 sec InputValidation entries (R2, R3) with the agent-harness family adjudications in-rule (R2 vs untrusted-content-into-prompt code-axis + spillway anti-authority posture; R3 vs externally-authored-edits-by-id + the escalation-chain stale-answer instance).
- [x] T2.3 `pins_manifest --write` in the same landing commit; count pins one-landing (AGENTS + README + .docs heads + health views + the narrative heads). (356/15/218 + sec 76; AGENTS + README + .docs/11_domains/rust_perf + health_views perf 356/sec 76/total 1694 + the kernel-side witness + the 5 rust_perf count pins + the 2 annotated pins + 301→304.)

## Phase 3 — gates + landing

- [x] T3.1 fixtures standalone-compiled AND executed pre-entry (Phase 1 receipts in the batch record). (26/26 across the two domains.)
- [x] T3.2 gates at an isolated `CARGO_TARGET_DIR`: default lib, full-feature lib, clippy `-D warnings` ×4 postures, fmt gate, retrieval floors (no re-pin expected — rust_perf + sec retrieval lanes unchanged in shape). (Clippy ZERO default/no-default/sec/all-features; lib 4408/0 sec posture; fmt PASSED with the b366 clean-inverted notripwire row added at bcf4cb8c3, the b352/b351/b342 precedent.)
- [x] T3.3 freeze/score-bench owed ONLY if clippy_lints corpus moved (it does not — assert, don't run). (Asserted: `git status` shows zero clippy_lints/ src changes; generations.rs untouched.)
- [x] T3.4 research note + queue snapshot + this plan checked off in the landing commit; binary rebuild check owed (rust_perf + sec ARE in the shipped release set — follow the dc51dadb binary-lane path). (Disclosed as NOT owed this landing: corpus-only delta, no proposer wiring change — the dc51dadb path rides the next binary-touching landing; the entry's `NotBenchable` + the suggest-only detectors change no release binary behavior.)
- [x] T3.5 commit + push on develop; `rm -rf .raw/opal` in the same close; df line in the summary. (Clone removed at close.)

## Proof path (the ungoat law — named before any corpus edit)

Falsification arm: every fixture's tampered arm must RED (the detector/scanner/apply/undo must distinguish its classes); pre-registered gates: fixtures pass standalone, retrieval floors hold, counts move by exactly the landed rule count. No benchable GOAT is claimed (metric-mismatch class — Research 413 §GOAT screen); every entry lands first-class suggestion-only `NotBenchable` with `bench_spec: None`.
