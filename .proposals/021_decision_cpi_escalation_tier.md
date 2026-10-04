# Proposal 021 — the Decision-CPI escalation tier: refine consumes rethink's served decisions (service CPI over lib, KAT-metered, local-floor-gated, FLYWHEEL-SEQUENCED)

**Status:** PROPOSED — rev 3; **Claude verdict AGREE 2026-10-04 (rounds 1–3; round 1: the serve-edge envelope facts, the Bench-098 negative prior, the never-per-span reconciliation, the contribution-before-consumption inversion, server-signed metering, the suite + auth dependencies, the T0′ kill, the G1 power spec; round 2: the traj-lane carrier correction — decstat is stats-only by its own redaction law, the labeled contexts ride `kat_sync_payload`'s gate-passed TrajStore rows; round 3: the mechanical record repairs)**
Branch: `develop`
Owner directive 2026-10-04 ("we even can use rethink node to serve decision for us, it's family anyhow and even use same chain and KAT, either use as lib share core logic or dapp CPI style any is cool for me, SOLID DRY distributed should take to the account here including tokenomics for call CPI and cost")
Fusion of: [Proposal 019](019_refine_rethink_one_plane.md) (the one-plane family law — the CONSUMPTION lane beside P019's contribution lanes; beside it sits the sibling [Proposal 020](020_profile_guided_healing.md), the profile-guided-healing design home — the threads compose: hot-span anchoring feeds the escalation's candidate set) × the instinct/rethink shared `serve_edge` (the server that answers `/decide`) × riir-ai `decision_client` (the envelope pattern to promote) × **Bench 098 / Issue 125 (the negative prior this proposal must invert)** × **Research 200 (the never-per-span law)** × **Research 206 (the arena-IS-the-mining-rig route — the flywheel that un-refutes)** × the KAT economy (burn watermark + epoch settle + decstat)

## TL;DR

The healer gets an **escalation tier**: at low-margin retrieval spans (`draft::margin_weight` — the existing substrate), it asks a family decision node over the **serve-edge envelope** (`DecideReq {suite, state, options|questions}` — what the edge actually parses; `katgpt-core::decision_wire` is opt-in and NOT the live wire), under deadline + budget + circuit breaker, **always falling back to the local structural rerank**. **CPI over lib** — decided by the A10 moat law (rethink's arsenal marks the encoder + head vessels `class = "hosted_only"`; linking them into the distributed cargo-refine binary is the act the class forbids), GPU locality, DRY, and economics. **But the recorded prior is NEGATIVE in exactly this domain**: Bench 098's choice_scorer lost 27.7%-vs-88.0% (5.6%-vs-100% OOD) at 8.9ms/span, starved on 101 gold contexts — Research 200's "never per-span" law is its standing proof. The inversion route is recorded too (Research 206): the consent-gated corpus grows the 10⁴–10⁵ labeled contexts that un-refute the class — **carrier-corrected at verdict r2: the contexts ride the TRAJ MINING LANE (gate-passed TrajStore rows — kat_sync_payload), NOT decstat — decstat is stats-only by its own redaction law**. **Therefore the sequencing is contribution BEFORE consumption**: T0′ (headroom + corpus kill) → **if go**: the traj-lane contribution claim (T0.8) ‖ the client (T0.1–T0.7, gated) → the refine-rule SUITE (a new suite: train mint + vessel + arsenal row — `/decide` refuses unknown suites and the encoder lane serves only canonical question shapes) → only then the escalation armed and G1 measured against the floor that already killed one challenger.

## 1. The verdict: CPI, not lib (corrected facts)

1. **A10 / the moat law (decisive, verified).** rethink's `arsenal.toml` marks the encoder and head vessels `class = "hosted_only"`; cargo-refine is a distributed binary; lib-linking ships those weights to uncontrolled hardware — the act the class forbids. The deciding computation cannot ship; the seam is a service boundary.
2. **GPU locality.** The encoder lane is GPU-host-only (Issue 016's posture; ms-class forwards); the healer runs on laptops and CI.
3. **DRY.** The server exists — rethink serves through the SHARED open edge (`riir_instinct::serve_edge::run`). A lib integration would duplicate serving logic the family ships once.
4. **Economics.** A lib call has no meterable seam. The tokenomics requirement selects the service form.
5. **Distributed correctness.** Independent failure (fallback-local), scaling, audit.

**The legal lib share**: the CLIENT substrate only (the calling logic), promoted to riir-kat per P019's Move-1 law; v1 local, mechanical move later.

**The wire (corrected)**: the live edge parses its own envelope — `DecideReq {suite, state, options|questions}` — and the riir-ai client builds/parses that envelope by hand (`decision_client.rs:211, 252`). `katgpt-core::decision_wire` is an opt-in katgpt-core feature refine does not enable today; it covers the multi-question path only. DIP/ISP as designed: the healer depends on the SERVE-EDGE ENVELOPE (pinned by contract test), never on rethink/instinct types; LSP holds over the envelope — any serve-edge server substitutes (reflex/instinct/rethink/localhost/mock).

**"Same chain and KAT"**: batched settlement riding the existing epoch settle, never per-call chain writes (the three-test/Glacial law).

## 2. The escalation seam (WHERE the healer asks — and the never-per-span reconciliation)

**The signal (the existing substrate, named)**: `draft::margin_weight` — `span_margin(matcher, code)` / `contested_margin(ranked)` / `span_min_margin`. The escalation set = spans below a pre-registered margin threshold. There is no "fused-gate abstain branch" in the retrieval path — margin_weight IS the low-confidence signal.

**The never-per-span reconciliation (argued, per Research 200's law)**: the law's proof is that a learned scorer ON THE HOT PATH (per-span, always-on) lost catastrophically and cost 8.9ms/span. This proposal's tier differs on every axis the law prices: **off the hot path** (only the below-threshold slice — measured first by T0′, expected small), **default-off** (feature inert + budget default 0), **deadline-bounded** (250ms connect cap, fallback always), **bounded per run** (budget), **and it asks a different question** (a suite-trained head over the family's canonical question — not a generic scorer over raw spans). Defensible — but only as designed: if T0′ measures the escalated subset as empty or structurally-saturated, the tier is closed with the record, per the law.

**The suite precondition (the fact that re-sequences everything)**: `/decide` looks up a REGISTERED suite (`serve_edge.rs` — unknown suites get `404 unknown_suite`), and the encoder lane serves only "the suite's CANONICAL question under its CANONICAL option order" — anything that "reorders, widens, or rewrites" refuses loud (`riir-rethink/src/encoder_serve.rs:28-35`). A per-span top-k candidate-rule list is exactly that rewrite class. **The refine-rule suite does not exist**: it needs a labeled corpus (the flywheel), a riir-train mint, a vessel, an arsenal row. The nearest existing suite (`code_fixtures`) is a different question and itself unsold (−6.25 under the paw bar).

## 3. Tokenomics (corrected metering)

**Metering source of truth = the SERVER's count, keyed to an AUTHENTICATED caller, SIGNED by the node.** The serve edge's receipts are content hashes (build fp + BLAKE3(input) + BLAKE3(decision) + lane) — anyone running the same release can recompute them (`serve_edge.rs:11-13`), so client-kept receipts prove nothing to a payee: they are an **audit/dispute mirror**, not the meter. Caller auth is CODE, not ops — the edge records "this server has no token surface" (`serve_edge.rs:762`); riir-kat's `account_key` is the substrate to reuse (an instinct/rethink dependency, §6).

**Settlement**: cumulative signed usage report per epoch window (the burn-watermark shape: monotone, replay-no-op), cross-checkable against server-side counts; rides dapps' epoch settle; no new chain program.

**Payer/corpus models — RESEQUENCED (the carrier corrected at verdict round 2)**:
1. **The contribution lane FIRST (the flywheel — carrier: the EXISTING traj mining lane, NOT decstat)**: the refine-rule suite's training corpus is the GATE-PASSED `TrajStore` rows the healer ALREADY ships (`src/kat_sync_payload.rs`: shape → fix → lint_key, `RedactionGate`-checked, BLAKE3-committed, account-key-signed, mining-consented) — a code-shape→fix→lint corpus is exactly what a suite mint consumes. **decstat/fixstat CANNOT carry it** (their own law: "Stats only, never questions, never answers, never option text" — counter rows cannot train; putting span text in would break the redaction law AND the golden bytes). T0.8 therefore NAMES the existing lane as the training source + adds whatever labels it lacks (e.g. the rejected candidate set per span) — and YIELDS to P019 Move 1 on the lane's code (the one-twist law: refine mines heal spans; rethink mines decstat receipts). Research 206's "decstat grows labeled contexts" phrasing inherits this correction (addendum filed there).
2. **Caller-burn** — µ-KAT/decision against balance/grant (the soft-gate analogy) once the tier serves.
3. **Subscription** — later, with the hosted tier.

**Pricing** = a dapps-side governed constant (never-lever). **Client cost guards**: `RIIR_REFINE_CPI_BUDGET=N` (default 0 = armed-but-idle), 250ms connect cap, circuit breaker (3 fails → disable + loud), run-summary disclosure.

## 4. Privacy (unchanged from rev 1)

The encoder needs span TEXT. Default OFF (`RIIR_REFINE_DECISION_URL` unset = never called); localhost-first (self-hosted node = no egress, lighter consent); remote egress = a SEPARATE three-tier consent (not piggybacked on mining consent); the redaction trade stated (full redaction incompatible with encoder value). The CONTRIBUTION lane inherits the mining-consent posture (trajectory-class egress, the sync-policy analogy) — its rows carry span+verdict, consent-gated, redaction-aware.

## 5. SOLID / DRY / distributed (corrected mapping)

| Principle | The decision |
|---|---|
| SRP | refine owns WHEN; the node owns WHAT; dapps owns SETTLE; riir-train owns the SUITE mint. |
| OCP | New escalation points reuse the envelope + client unchanged. |
| LSP | Any serve-edge server substitutes (reflex/instinct/rethink/localhost/mock) — stated against the ENVELOPE, not decision_wire. |
| ISP | The narrow envelope types only; no serving internals cross the seam. |
| DIP | The healer depends on the pinned envelope (contract-tested), never rethink/instinct types. |
| DRY | Envelope (the edge, exists) · server (shared edge, exists) · client pattern (riir-ai, promote per P019) · metering (burn-report shape) · consent (three-tier) · escalation signal (`margin_weight`, exists) · outcome rows (`decstat`, exists) — ONE new artifact: the seam + its gate; ONE new trained artifact: the suite (the flywheel's product). |
| Distributed | fallback-local availability · deadline latency · breaker isolation · server-signed audit · consent data boundary · budget cost. |

## 6. Dependencies & follow-ups (the corrected list)

- **The refine-rule SUITE** (the critical path): labeled corpus (the contribution lane) → riir-train mint → HOSTED-ONLY vessel → arsenal row → the encoder lane can serve the canonical refine question. A rethink/train-side work item named here, executed there.
- **Caller AUTH on the instinct/rethink edge**: the no-token-surface gap; riir-kat `account_key` substrate; an instinct-side code task.
- **dapps**: the usage-report ingest + the price constant + the subsidy rule (their proposal process).
- **riir-kat**: the client promotion (rides P019 Move-1 sequencing).

## 7. Gates

- **G0 (correctness)**: every failure path collapses to local; hermetic contract tests pin the ENVELOPE against a mock node; default-off verified (no env = zero calls, byte-identical run).
- **T0′ (the headroom + corpus kill — FIRST, before any client)**: replay the retrieval eval fixtures, count spans below the pre-registered margin threshold, measure structural top-1 on that subset; AND count today's gate-passing mined traj rows against Bench 098's 101 gold contexts and the 10⁴ target. **If the subset is empty, structurally saturated, OR the corpus is far off target and not growing, the tier CLOSES with the record** (the never-per-span law holds; nothing was built). Both kill conditions in one hours-long task.
- **G1 (the floor gate — specified to be runnable)**: PRECONDITION the suite exists. Power honesty: the escalated subset may be single-digit of ~112 labeled fixture spans → a paired LB95 > 0 with a PRE-REGISTERED minimum n (the arsenal's own certification standard — `stats::PairedDiff::lb95`); OOD non-regression required; **cost-adjusted gain** (accuracy delta per µ-KAT + latency) reported beside the raw delta. The floor is the local structural rerank — the same floor that measured 88.0% in-dist / 100% OOD against Bench 098's challenger.
- **G2 (cost)**: budget holds; disclosure honest; server-signed counts reconcile.
- **G3 (privacy)**: default-off by test; remote egress refuses without the separate consent.

## 8. Phases (re-sequenced: contribution before consumption)

- **T0′ = the headroom kill** (hours; closes the lane early if negative — Issue 147's first task).
- **T0 = Issue 147** (GATED on T0′'s go — the client is suite-agnostic and serves any serve-edge node today incl. localhost instinct + the arena suites, which is the stated justification for building it at T0 rather than beside the suite; if the go is marginal, defer): seam at margin_weight + envelope client + guards + separate consent + hermetic contracts + metering design (server-signed). No dapps/suite dependency.
- **T0.8 = the contribution claim**: NAME the existing traj mining lane as the suite's training source + add the missing label fields (the rejected candidate set) — yielding to P019 Move 1 on the lane's code (standalone value: the corpus feeds self_evolve/Elo + mining regardless of the tier's fate).
- **T1 = the suite** (train mint + vessel + arsenal row — riir-train/rethink side, corpus-gated).
- **T2 = the G1 measurement** (the escalation armed against the real node; the go/no-go).
- **T3 = the dapps usage-ingest + client promotion + the second escalation point** (frontier triage composing with Issue 146/P020's hot-span lane).

## 9. Honest caveats (the prior record cited, headline)

1. **THE NEGATIVE PRIOR IS THE HEADLINE.** Bench 098 / Issue 125 ran this exact experiment in this exact domain: the learned option-scorer for rule choice measured **27.7% vs the structural rerank's 88.0% in-distribution, 5.6% vs 100% OOD, at 8.9ms/span**, starving on 101 gold contexts. Research 200's standing law ("never per-span — the refutation is the proof the hot path stays modelless") is the recorded consequence. This proposal is a SEQUENCED BET on the recorded inversion route (Research 206's corpus flywheel), not a fresh optimism: the corpus must reach the 10⁴–10⁵ labeled-context class before the class un-refutes, and T0′/G1 are designed to kill the lane cheaply at every stage if it does not.
2. **The suite does not exist** and is the critical path (corpus → mint → vessel → arsenal row); `/decide` refuses unknown suites by design and the encoder lane refuses non-canonical question shapes by design — both correct laws that price this proposal honestly.
3. **Auth is missing on the edge** (no token surface — code, not ops) and receipts cannot meter (content hashes) — both fixed in §3/§6 before any KAT moves.
4. **Pricing needs dapps**; T0/T0.8 are fully useful without it.
5. **Privacy may cap adoption** (span egress; localhost mitigates; consent stays unbundled from pricing).
6. **Node ops** is an owner commitment; localhost-first works without it.
7. **P019/P020 coordination**: the client promotion yields to P019; the T3 composition rides P020's lane.
