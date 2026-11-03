# Plan 333 — Proposal 005 fixer unblock: the leak-posture sign-off package + all-tier prep for the Mozilla window

**Status:** ACTIVE 2026-11-03 — Phase A LANDED (T1, T1c, T2, T3, T5 done; T1b owner-gated; T4 = the opt-in-dark consent wiring, open); the memo is SIGNATURE-READY — OwnerGate target ≤ 2026-11-14; Phase B owner-gated (the signature IS the gate); spawned by the owner ask "how to unblock Proposal 005? we need to join Mozilla soon, prepare all tiers" — Proposal 025 window: G4 decide 2026-11-07, prose 2026-11-17–19, issue 190 deadline 2026-11-20

**Verdict:** Claude AGREE 2026-11-03 (round 1, delegated-owner flow) with four fold-ins (landed in this revision) and the OwnerGate decisions: **CF-only fixer launch approved**; Groq deferred until T1b closes; **T3 memo target 2026-11-14** (ahead of the 11-17 prose window); T6/T7/T8 take effect on signature only, no partial launch; T4 consent wiring may land dark; **T9 approved to open now**; T10 DEFERRED.

## TL;DR — how Proposal 005 actually unblocks

The fixer's two named blockers (caveat #2) are no longer both open. **Quality is LIFTED** (Bench 049: verify-filtered strict-keep 5/9 A / 6/9 B vs decline 0/9; pre-pass dominates; never-wrong held — `l4_fallback.md` §5 records the lane decision). **Leak is a signature, not a build**: the technical posture is already shipped — `kat_redact` fail-closed scanner (REDACT_VERSION 5; Bench 052 T3.2 external-origin suite + Bench 053; CI-gated via Bench 055), the consent seam `src/code_egress.rs` (`RIIR_REFINE_CODE_EGRESS=snippet`; unset/junk refuses PRE-socket), no-retention metadata on the hosted wire (Plan 202 R2), the USD circuit breaker (Issue 046, `llm_spend.rs`), the strict-keep verify contract. What remained was "vendor no-training verified in writing + the model-provider license half + the owner's sign-off" — and the vendor half is now half-done: **CF Workers AI is VERIFIED in writing** (T1, this session); **Groq is NOT** (and the vendor set drifted — see the finding). So the unblock is: finish vendor verification (Groq) + license rows → one-page memo → owner signs → ENABLE flip + deployer manifest + copy lifts. Phase B is the W1b prerequisite (Proposal 025 opens the fixer/trainer service under MIT in Q2 — the service must exist by then); it is NOT on the submission critical path (the application discloses the points plane as design; W0/Q1).

## The vendor-drift finding (T1, verified by fetch 2026-11-03)

Issue 051 (2026-08-31) made the daily trainer window **groq-first** after caveat #2 was written against "single vetted vendor (CF)". Verified today:

- **CF Workers AI — VERIFIED.** [Data usage](https://developers.cloudflare.com/workers-ai/platform/privacy/) (updated 2026-10-16): "**No training:** Cloudflare does not use Customer Content processed by Workers AI to train AI models." + "**Zero data retention**" for standard non-stateful inference; operational metadata is not Customer Content; exceptions named (stateful features — R2/KV/DO/Vectorize, logging/AI-Gateway caching, batch/async). Our posture (direct REST inference from a Worker, non-stateful) sits in the retained-free class. Third-party models carry their own licenses → the T2 half. Full quotes: [`../.docs/01_orientation/fixer_leak_posture_vendors.md`](../.docs/01_orientation/fixer_leak_posture_vendors.md).
- **Groq — NOT verified at the public tier.** The public Privacy Policy (eff. 2025-11-12) **excludes** Customer Data: "Our processing of Customer Data in connection with a customer's use of our Cloud Services is governed by our Groq Services Agreement and Data Processing Addendum." Terms of Use: "the Groq Services Agreement governs." No public no-training commitment for API data; free-vs-paid tier handling unknown from public pages. Two consequences: (a) the memo cannot cite Groq — **first enablement pins CF only** (the measured Bench-049 winner posture anyway); (b) DRIFT NOTE for the standing trainer window: its bootstrap posture was written as "own-code-to-CF is the already-vetted posture," but the LlmRouter now serves the free leg GROQ-first — the window's own vendor vetting must complete (lower bar: own-repo redacted data, not consumer code — but if a free tier trains on inputs, our own corpus spans leak into someone else's model: business-IP, not just consumer-privacy).

## Phase A — the sign-off package (agent-executable)

- [x] **T1 — vendor verification doc** landed: [`../.docs/01_orientation/fixer_leak_posture_vendors.md`](../.docs/01_orientation/fixer_leak_posture_vendors.md) (CF quotes verbatim with URL + dates; Groq section explicitly OPEN)
- [ ] **T1b — Groq Services Agreement + DPA verification** (owner account or written inquiry to Groq; pin free-vs-paid tier handling). Gate for ANY Groq arm in the fixer lane and closes the trainer window's drift note. OwnerGate decision: CF-only launch approved; Groq deferred until this closes.
- [x] **T1c — Groq exposure check (pre-T1b)**: LANDED 2026-11-03 — the exposure HAS HAPPENED for the own-repo drain class: `.benchmarks/llm_cost_ledger.jsonl` run `drain-18d01cf6ab6e21c8-99237` shows `served_by: "groq"` absorbing 2/2 drain calls (323+9 tok). Data class = the unproven queue (own-repo redacted, sync-ToS-licensed — NOT consumer code; no consumer live code has ever left a machine). Recorded in the vendor doc's Groq section; T1b is the closure for both classes.
- [x] **T2 — model license half**: LANDED 2026-11-03 — llama-3.3-70b row CLOSED (Community License + AUP fetched + quoted: attribution §1.b.i binds distribution/containing, not hosted serving; naming clause binds output-trained distributed models; "Built with Llama" recommended as insurance; AUP classes fine for dev tooling; EU multimodal restriction N/A). qwen3.8-27b row stays OPEN with the named step (CF model-catalog license field) — not load-bearing for the CF-llama first enablement. Rows in `fixer_leak_posture_vendors.md`.
- [x] **T3 — the sign-off memo**: DRAFTED + SIGNATURE-READY 2026-11-03 — [`../.docs/01_orientation/fixer_leak_posture_signoff_memo.md`](../.docs/01_orientation/fixer_leak_posture_signoff_memo.md): the decision, the consent gate, the five numbered exclusions (incl. the no-payload-logging rule + pre-T6 checks, fold-in #1), the verified vendor + license posture, the pinned consumer disclosure/prompt text (fold-in #2), kill switches, the Phase-B flip list, the not-changed list, and the T1c exposure note. Awaiting the owner signature (OwnerGate target ≤ 2026-11-14).
- [ ] **T4 — per-repo consent UX (opt-in-dark)**: the hosted fix path asks once per repo (the mining-consent pattern), persists the choice, prints the exact refusal vocabulary when unset; the prompt text carries the consumer-facing disclosure (T3 fold-in #2) and is pinned word for word by the memo. Plan 049 T0.0d guard (a) verbatim: the wiring may land ahead of the sign-off; the ENABLE flip does not.
- [x] **T5 — the unblock runbook pointer**: riir-deployer `node_tiers.md` fixer row now names this plan + the T1 doc (status stays BLOCKED until signature — the row points, it does not promise).

## Phase B — on signature (owner act; `- [-]` until signed)

- [-] **T6 — ENABLE flip**: the fixer serving lane per `l4_fallback.md` §5 item 2's own gate (the arm that already passed: modelless pre-pass → CF llama-3.3-70b → `fix_verify` strict-keep), CF provider only at first.
- [-] **T7 — deployer manifest rows**: the fixer node (cf-worker + model binding) + un-`[-]` the Trainer-window row's leak gate (the proposal's "plus a leak-posture sign-off either way" covers both).
- [-] **T8 — copy lifts**: Proposal 005 caveat #2 amended; `node_tiers.md` Pro row "fixer half BLOCKED" → live posture; Plan 107's no-LLM-fixer-marketing non-goal lifts; dist README gains the fixer section (source-free, Proposal 003) **including the consumer-facing disclosure notice (T3 fold-in #2)**.

## Phase C — all-tier prep vs the Mozilla window (the owner's "prepare all tiers")

Submission does NOT wait on the fixer (Posture C is design-disclosed; W0/Q1 implements it; W1b/Q2 opens the fleet tooling — this plan's Phase B is what W1b opens). Per tier, honestly:

| tier | state today | prep item | window |
|---|---|---|---|
| **Lite** | ✅ shipped | none — evidence pack feeds G3 (issue 190 T3, LANDED 2026-10-31) | submission-ready |
| **Pro** | ✅ core shipped; fixer half BLOCKED | this plan (A → B) | memo by mid-Nov; flip in Q1 with W0 |
| **Max** | 🟡 replay client shipped (Plan 127); lane **RATIFIED-INERT** (keep `replay_cut_bp = 0` — Bench 013: the sweep ran, verdict AGREE, paying at the current pool buys nothing) | pilot posture: W3's vouchers/service capacity carry the earn-free story; replay settles only after the owner funds pool scale + moves the canary bound (the one-act interlock refuses partial arming) | with the owner's pool-scale act, not Q1-by-default |
| **Ultra** | 🟡 designed; window posture landed (groq-first router, USD cap, window gates) | T1b's Groq verification + first standing window on the real drain; vessel corpus host later | Q1–Q2 |
| **decstat** | two-key inert (owner arms) | untouched — not on this plan's path | owner |

- [x] **T9 — Max-tier settlement prep: APPROVED to open (OwnerGate, verdict round 1) — then CORRECTED same day (second session).** The "open as its own plan" act produced [Plan 334](334_p015_phase3_replay_settlement_econ_sim.md), which on execution-grounding turned out to be a **DUPLICATE of riir-clippy [Plan 128](128_p015_phase3_replay_econ_sim.md)** — the sim RAN (riir-dapps Bench 013, 48 cells, G1-pinned to the real ledger math, verdict AGREE round 2), and the ratification was ALREADY CONSUMED 2026-09-15 under owner delegation: **`replay_cut_bp` STAYS 0** (the recording lane is live and free; paying at the current pool buys no behavior). Plan 334 CLOSED-DUPLICATE with the full premise correction. The honest pilot posture: **W3's vouchers/service capacity carry the earn-free story** (Proposal 025 W3 verbatim); replay settles only via the recorded arming preconditions (owner ops: pool scale + the 10% canary one-act interlock — both mechanized/refused in `riir-dapps/src/kat/replay.rs`).
- [-] **T10 — Ultra window standing lane**: owner-gated ops cadence (free-tier allocation; T1b's Groq verification is the free leg's gate).

## Non-goals

- No code ENABLE before signature (Plan 049 T0.0d guard (a)).
- No marketing lift before signature (Plan 107 non-goal stands).
- The `kat_redact` STRICT-bar decision (issue 190 T14a: gate behind `kat_service` vs inline vs vendor a shared leaf) is a separate owner call — adjacent posture, different gate, not here.

## Related

Proposal 005 caveat #2 + §"Node-tier additions" · `l4_fallback.md` §5/§6 · Benches 049/052/053/055 · issue 190 (submission 2026-11-20) · issue 199 (the tier-name rename this plan's vocabulary follows) · Proposal 025 (W1b is Phase B's consumer) · Plan 202 R2/R3a (no-retention + consent substrate) · Plan 049 T0.0d (the opt-in-dark precedent)
