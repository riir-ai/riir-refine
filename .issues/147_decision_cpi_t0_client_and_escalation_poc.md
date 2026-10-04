# Issue 147 — Decision-CPI T0: headroom kill first, then the suite-agnostic client + guards + the contribution lane (Proposal 021's POC)

**Status:** OPEN (T0′+T0+T0.8; Proposal 021 rev 3's POC — the client half T0.1–T0.7 is GATED on T0′'s go; no dapps dependency, no suite dependency for the client half; the escalation USE is corpus-gated; alignment re-call 2026-10-04: T0′ clean-baseline + T0.8's NO-GO disposition)

## What

The healer's decision-CPI tier, staged per Proposal 021 rev 3: **T0′ measures the headroom + corpus first (the cheap dual kill)**; if go, T0.8 names the traj mining lane as the suite's training source (the carrier that un-refutes the scorer class) ‖ T0 builds the suite-agnostic envelope client (serves ANY serve-edge node — localhost instinct, the arena suites — not just the future refine-rule suite). Full design + the Bench-098 negative prior + the never-per-span reconciliation: [`.proposals/021`](../.proposals/021_decision_cpi_escalation_tier.md).

## Tasks

- [ ] **T0′ THE HEADROOM + CORPUS KILL (FIRST — hours, before any client code)**: replay the retrieval eval fixtures; count spans below a pre-registered `margin_weight` threshold (`draft::margin_weight` — the existing substrate); measure structural top-1 on that subset; AND count today's gate-passing mined traj rows against Bench 098's 101 gold contexts and the 10⁴ target. **Clean baseline (alignment re-call): runs with NO `--hotpath-json` feed** (P020's hot-span anchoring must not contaminate the baseline) **at a pinned refine SHA**. **CLOSE the tier with the record if the subset is empty, structurally saturated, OR the corpus is far off and not growing — T0.8 then moves to its own issue** (its corpus value is standalone: it feeds self_evolve/Elo + mining regardless; the tier closes, the contribution lane does not). Record: threshold, subset size, structural top-1, corpus count, the go/no-go.
- [ ] **T0.1 The seam** (`decision_cpi` feature, default-off): `DecisionEscalation` at the margin_weight low-margin branch — input: span text + the candidate set + context; output: choice/abstain + receipt. **The wire = the SERVE-EDGE ENVELOPE** (`DecideReq {suite, state, options|questions}` — what the edge parses; NOT `katgpt-core::decision_wire`, which is opt-in and multi-question-path only). Envelope pinned by contract test.
- [ ] **T0.2 The thin client**: std-only (~120 lines, the riir-ai `decision_client` pattern: hand-built envelope, per-call L3 deadline, 250ms connect cap); `RIIR_REFINE_DECISION_URL` (unset = never called); client-kept receipts under `.refine/` (gitignored) — **as AUDIT MIRROR only** (content hashes are recomputable by any same-release caller; they meter nothing). P019 Move-1 riir-kat promotion recorded as the follow-up home.
- [ ] **T0.3 The guards**: `RIIR_REFINE_CPI_BUDGET=N` (default 0); circuit breaker (3 consecutive failures → disable + loud); run-summary disclosure (calls, fallbacks, receipts, "unpriced — dapps ingest pending").
- [ ] **T0.4 Consent**: a SEPARATE three-tier record for remote span egress (default Unset = remote refuses; localhost exempt); NOT piggybacked on mining consent; the redaction trade in the module doc.
- [ ] **T0.5 Contract tests** (the `kat_soft_gate_cli.rs` hermetic pattern): mock serve pinning the ENVELOPE; failure paths (timeout/5xx/malformed → fallback + loud; budget-exhausted → disable + remedy); default-off verified (byte-identical run, zero calls).
- [ ] **T0.6 The plumbing eval**: scripted mock node (deterministic choices) proving escalation + fallback + receipt path end-to-end on the retrieval fixtures.
- [ ] **T0.7 The metering design note**: SERVER-signed counts keyed to authenticated callers (the source of truth), cumulative epoch-window reports (burn-watermark shape), client receipts as the dispute mirror — documented in the module; implementation is dapps + edge-auth (below).
- [ ] **T0.8 THE CONTRIBUTION CLAIM (the training source — carrier-corrected at verdict r2)**: NAME the EXISTING traj mining lane (`src/kat_sync_payload.rs`: gate-passed TrajStore rows — shape → fix → lint_key, RedactionGate-checked, BLAKE3-committed, account-key-signed, mining-consented) as the refine-rule suite's training source, + add the label fields it lacks (e.g. the rejected candidate set per span). **decstat/fixstat CANNOT carry contexts** ("Stats only, never questions, never answers, never option text" — their redaction law + golden bytes). YIELDS to P019 Move 1 on the lane's code. Standalone value: the corpus feeds self_evolve/Elo + mining regardless of the tier's fate.

## Explicitly NOT in T0 (the corrected dependency list)

- **T0.1–T0.7 (the client half) is GATED on T0′'s go** — an idle client serving only the arena suites during an adoption-dependent stretch is not worth building unless the headroom + corpus readings say the lane lives; the gate is hours, the client is days. (If T0′ reads go, the client's suite-agnostic service — localhost instinct nodes, the arena suites — is the stated justification for T0-timing rather than suite-timing.)

- **The refine-rule SUITE** (T1 — riir-train mint + HOSTED-ONLY vessel + arsenal row; `/decide` refuses unknown suites and the encoder lane refuses non-canonical question shapes — both by design; the suite is corpus-gated by T0.8).
- **Caller AUTH on the instinct/rethink edge** (the edge has NO token surface today — `serve_edge.rs:762`; riir-kat `account_key` is the substrate; an instinct-side code task, prerequisite to any KAT movement).
- **The G1 real-node gate** (T2 — suite precondition; paired LB95 > 0 with pre-registered min n; OOD non-regression; cost-adjusted gain; the floor that measured 88.0%/100% against Bench 098's challenger).
- **dapps ingest/pricing** (T3) · **the riir-kat client promotion** (rides P019) · **further escalation points** (frontier triage, composing with Issue 146/P020).

## Law check (the one-line records)

CPI not lib: rethink's arsenal marks the encoder+head vessels `hosted_only`; a distributed binary must not link them. Never-per-span (R200): reconciled — off-hot-path (the below-threshold slice only), default-off, deadline-bounded, budget-bounded, suite-canonical questions; T0′ kills the lane if the slice is empty/saturated. Negative prior: Bench 098 (27.7 vs 88.0 / 5.6 vs 100 OOD) — the tier exists ONLY as the sequenced bet on the corpus flywheel (carrier: the TRAJ mining lane, NOT decstat — stats-only by law) inverting it.

Refs: Proposal 021 · P019 (one-plane) · P020 (the sibling lane — T3 composition) · Bench 098 / Issue 125 (the prior) · Research 200 (never-per-span) · Research 206 (the flywheel) · instinct `serve_edge` (the server + the no-token fact) · riir-ai decision_client (the pattern) · Issue 146 (the composition target).
