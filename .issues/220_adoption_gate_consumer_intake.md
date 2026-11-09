# Issue 220 — T1 consumer intake: adoption/retention regression gate (riir-refine side)

**Status:** T1 WIRING LANDED 2026-11-10 (amo idle lane; feature `adoption_gate`, G1+G2 green, the landing commit is the parent of this edit) — the issue stays OPEN for G3 (the REAL-store GOAT cell, katgpt-rs plan 633 T3, which this consumer opens: the store is named — the JSONL window + the latent store's trajectory rows). Intake issue for Plan 633 T1 (katgpt-rs). The gate primitive + fixture are LANDED katgpt-rs-side; this issue is the riir-refine consumer wiring, filed per katgpt-rs `.issues/987_cactus_neurips2026_follow_ups.md` priority-1 (AGCL form, Research 657 §A5) and katgpt-rs `.plans/633_adoption_retention_regression_gate.md`.
**Lease closed:** amo-idle-220-i1191cont, 1794259134, was expires 2026-11-11T21:18:54+00:00 — the T1 wiring landed in the window (claim `c62b85de`); G3 stays unclaimed (no lease is not a license — claim before the cell).

## Landing record (2026-11-10, the T1 wiring)

- **(3) Feature forward:** `adoption_gate = ["katgpt-core/adoption_gate"]` (the `elo_k_eff`/`step_noise` forward shape — same name, forwarded, never forked); `src/adoption_gate.rs` (cfg-gated) is the seam: `outcome_digest(tag, payload)` (blake3 over `tag || 0x00 || payload`), `AdoptionWindow` (the JSONL sidecar `.refine/adoption_outcomes.jsonl` — one row per final fix verdict, hex digests, seat rows carry `build`; replayed in file order into the producer gate, idempotent by ingest dedup; tail compaction past 4096 rows), `GateVerdict {Proceed, Refuse, Abstain}`.
- **(1) Active-set gate:** `--active-set-demote` scores the challenger = the window MINUS the rule's rows (what the demoted set still produces) via the producer's `decide`; `Keep` refuses exit 1, the marker file untouched; `--active-set-readmit` refuses iff the rule's own history repeats a known-rejected output (`repeated > 0` — the composite decision is uninformative on the readmit direction, documented in-module). An empty/foreign window ABSTAINS and proceeds — gate on evidence, never on absence (the operator-command divergence from the producer's decline-first law, which stays binding for any future AUTOMATED demotion path).
- **(2) Seat head-swap watch:** seat rows carry the seat receipt's build fingerprint (threaded serve-site → verdict bookkeeping via a per-run path map); a build whose historical seat rows repeat known-rejected outputs stands the seat down FOR THE SESSION (`EscalationDispatch::stand_down`, in-memory like the rate latch — the manifest is never written, nothing auto-demotes) and demotes the current pick to stood-by. First-sighting builds are `NoEvidence` → serve (the serve-boundary compile check still gates every pick).
- **Recorder:** the fix lane's three verdict sites (kept / per-line revert / whole-file revert — the Issue-018 semantics) append outcome rows best-effort (warn-once); kept edits under a non-clean verify (P104 UNVERIFIED-KEPT) record NOTHING (no evidence either way).
- **G1:** `tests/adoption_gate_gates.rs` 6/6 — the producer fixture's ingest sequence replayed through the seam with exact f64 pins (retained `8/12`, the scorer IS the producer's `decide` so the expression tree cannot drift; the one unrepresentable producer-candidate element — an out-of-window novel output — disclosed in the test doc); demote-refuse/abstain, readmit repeat-refuse/clean-proceed/no-rows-abstain, seat swap stand-down, never-double-file + first-verdict-wins.
- **G2:** feature-off byte-identical by construction (the module + all wiring cfg out; default clippy `-D warnings` green, the default lib suite 3775→3778 unchanged posture, no new default-posture behavior).
- En-route gate repairs (pre-existing, mechanical, provably at HEAD — the detached-HEAD worktree re-run proved all three standing failures byte-identical): `kat_redact.rs` assert_is_empty → `assert_ne!(findings, [] as [Finding; 0])`; `kat_state_stamp.rs` single-char patterns ×2; the batch-345 corpus test's too_many_lines split + a `LocalSR` doc backtick. **Still standing (NOT mine, the issue-192/221 lanes' residue, verified failing at HEAD):** `tests/heal_usage_flag_parity` (the usage text still advertises the removed `--mine/--unmine/--purge/--purge-server/--flag`), `tests/micro_literal_gate` (`kat_units.rs:376`'s anonymous 1_000_000 lacks its allowlist row), the two `kat_units.rs` doctests (`crate::kat_units` unresolved in doctest scope after the vendoring). The 221 lease (zed-352d06e6-m3-221, live) owns that area.

## What exists (producer side, landed `cf65c58a7` 2026-11-08)

`katgpt-core::adoption_gate` behind the opt-in feature `adoption_gate` (zero new deps, modelless):

- `OutcomeDigest` — blake3 `[u8; 32]` newtype over an accepted outcome or a produced output.
- `DedupRing` — fixed 32-slot FIFO membership set (1064 B, layout-pinned); the local mirror of riir-neuron-db's `RingRow<T,32>`.
- `Decontaminator` — never-double-file stride-5 splitter (held-out audit sink vs gate window).
- `score_adoption` / `AdoptionGate::decide` — score `= (1−α)·retained + α·(1−repeated)`, f64 sequential folds, ties→challenger, `NoEvidence` → keep incumbent (decline-first), always-from-base purity pinned bit-for-bit.
- 9 fixture tests green; clippy `-D warnings` green at both feature postures.

## What this repo consumes (the T1 wiring — LANDED 2026-11-10)

- [x] **T1-a Active-set changes** (`--active-set-demote` / `--active-set-readmit`, Issue 102 machinery): a demotion that later turns out wrong (the demoted rule's lints re-fire and again get fixed by hand or by another rule) is exactly the AGCL regression class. `AdoptionGate::decide` wired over the outcome stream = (rule, fix verdict) rows recorded by the fix lane's verdict bookkeeping into the window sidecar; on demote/readmit the NEW active set is scored against the previous window's accepted outcomes (fixes that compiled+verified) and rejected outputs (regressions re-fixed). Design deltas, disclosed: the window is the repo-local JSONL sidecar (`.refine/adoption_outcomes.jsonl`) rather than the latent store — the misses/corpus-gap queues aggregate per-lint (the traj store UPSERTs), not an ordered event stream; the store-side window is the G3 cell's decision.
- [x] **T1-b `seat_rerank` head swaps** (Plan 202 R3a, ARMED on the dev box): the swap = the seat receipt's `build` fingerprint change (the receipt arrives on every seat answer — no instinct-side lockstep needed); the gate = non-repetition of previously-rejected outputs by the new build's rows (retention's pre-hoc half is unobservable for a first-sighting head — NoEvidence → serve, documented); the session-scoped `stand_down` is the demote mechanism (in-memory, manifest never written). The α/stride/window-on-the-manifest-row config is DEFERRED to the G3 posture (T1 ships the producer defaults: α 0.5, stride 5, ring window 32).
- [x] **T1-c Consumer-side feature**: the katgpt-core feature forward `adoption_gate` re-export (the `elo_k_eff`/`step_noise` forward precedent — forward, never fork; goat-audit watches this edge). Naming per house law: the consumer feature stays `adoption_gate` (same name, forwarded).

## Bars (pre-registered now, before any wiring)

- G1 (correctness): a seeded synthetic outcome stream replays through the consumer seam bit-identically to the producer fixture verdicts (the purity pin carries over — same inputs, same verdict).
- G2 (no regression): gate-OFF byte-identity — with the consumer feature off, active-set demote/readmit + seat_rerank behave exactly as today (the `elo_k_eff` feature-off byte-identical posture).
- G3 (the GOAT, T3 in plan 633): gate-on vs gate-off on the REAL store — fix trajectories + oracle verdicts; α under judge noise measured (compile-pass/oracle verdicts are noisy judges — the AGCL simulated-perfect-user gap, inherited honestly); may fail honestly — a verdict, not a bug.
- ⛔ No promotion talk before G3; `decline` (keep incumbent) is the default answer forever.

## Refs

- katgpt-rs `.plans/633_adoption_retention_regression_gate.md` (T1 = this issue) · katgpt-rs `.issues/987` priority-1 row · Research 657 §A5 (AGCL)
- Producer precedent: riir-refine 632 `elo_k_eff` (consumer of katgpt-core `step_noise`, `280f15766`)
- The never-double-file decontamination answers the never-double-file decontamination risk the active-set ledger never had a guard for.
