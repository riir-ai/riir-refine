# Mozilla Human-Scale AI Incubator — grant readiness (the private playbook)

> **Purpose:** the full grant-strategy playbook — program facts, per-repo
> eligibility matrices, verdicts, frictions, the evidence inventory, the
> consult email, and the timeline. Program digest (public-safe):
> [katgpt-rs Research 642](../../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md).
> Working proposal: [`../../.proposals/025_mozilla_human_scale_ai_incubator.md`](../../.proposals/025_mozilla_human_scale_ai_incubator.md).
> Task issue: [`../../.issues/190_mozilla_grant_readiness_gaps.md`](../../.issues/190_mozilla_grant_readiness_gaps.md).
> **This file is grant strategy in a private repo — do not mirror, quote, or
> link it from any public surface.**

## 1. Program facts (verified 2026-10-31)

| Item | Fact |
|---|---|
| Program | Mozilla Foundation Incubator — **Human-Scale AI** cohort |
| Award | $300K + 12 months from April 2027; up to $200K + 2 more years on progress → **$500K/3yr total, non-dilutive** |
| Cohort | 7 projects + 3 anchors (Tiles, Family Intelligence, DAIR) |
| Deadline | **2026-11-20, 11:59 PM PST (UTC-8)** = Nov 21 07:59 UTC — no extensions |
| Info session | **2026-11-12, 17:00 UTC** (registration "coming soon" on the CFP page) |
| Decision | Late Feb 2027; no individual updates |
| Contact | `incubator@mozillafoundation.org` |
| Eligibility | Must receive funds from a US 501(c)(3); charitable purpose required; for-profit OK with charitability case; **fiscal sponsor recommended** |
| Required principles | **Small · Intentional · Tailored** — all three; Tailored = named community in dialogue |
| Evaluation | Human-Scale expression · technical viability · problem-community clarity · product-community fit · impact/values · sustainability |
| AI policy | Submitted content must be the owner's original voice |

## 2. Eligibility verdict (decided 2026-10-31)

**ONE proposal: cargo refine. The decision-serving family folds in as the decision layer (§3 of Proposal 025). Rethink's application (005) PARKED; its community work continues as product follow-up.**

- **cargo refine — THE application.** Strongest Small+Intentional story in the workspace; openness and credit-plane frictions are decidable before the deadline; community evidence is the real risk, gated by the no-submit check (190 T10). One-line reason: zero-parameter hot path + consent-first data flows + per-repo tailored healing + free-for-the-commons policy is a near-textbook Human-Scale AI expression, *if* the openness amendment lands.
- **Decision stack — folded in as the decision layer.** Honest scoping: the application names the OPEN components the healer consumes (reflex decision envelope + instinct escalation grammar); the private rung, weights, and economy planes are out of scope by name.
- **Not eligible as-is** (historical, pre-adjustments): private-forever + commercial posture — the frictions below exist to fix exactly that.

## 3. Eligibility matrix — cargo refine (§ marks the proposal sections)

| Principle | Evidence we already ship | Gap |
|---|---|---|
| **Small** | The L1 drafter is **modelless**: 580+ distilled repair rules across 6 domains are the model — ternary SIMD matvec over latent rule vectors, zero-alloc, sub-ms, CPU-only. No LLM, no GPU, no API, works offline, one static binary. **Below Mozilla's own 7B reference floor: zero parameters in the hot path.** Larger-model lanes (hosted escalation, L4) are opt-in and out of proposed scope. | None material. |
| **Intentional** | Consent-first telemetry (Unset never pushes; OptedOut never transmits); fix-trajectory memory in `.refine/` on the dev's disk (inspect/export/`--purge`); consent screen pins the **MIT licence for published pairs**; redaction refuses repo-private context (`allow_redacted: false`); every cost surface disclosed in the run footer. | The credit plane (KAT) sits on the same surface — friction F2. |
| **Tailored** | Per-domain corpora; `--repo-posture` heals by the target repo's own lint posture; trajectory memory learns which rules work in *this* codebase (Elo + Beta-posterior over the dev's own fix history). Proposed community: **Rust developers without cloud-AI budgets** — students, indie maintainers, educators, resource-constrained regions. | **No external community yet** — pilot must be designed (issue 190 T4). |

**Frictions:**

- **F1 — Openness.** Research 003 law: "anything riir-* is internal… prebuilt binaries only." Mozilla expects work in the open + open code left behind. Dist surface (installer repo, Homebrew tap, Scoop bucket) is already public; the consented corpus is already MIT data. **Proposed amendment: open the modelless core (corpus format + embedder + drafter + pruner + self-evolve) under MIT/Apache-2.0, months 1–3 of the term.** Owner decision — issue 190 T1. (This amendment language lives here and in issue 190 — private repo only.)
- **F2 — Credit-plane posture.** A token-mediated pay plane on a devnet ledger reads commercial/crypto against the charitable framing. **v2 resolution (owner direction): the credit system is reframed as mechanism design — free for open-source repos, contribution-earned credits or cost-recovery payment, operational bonds, expiring pilot vouchers; pay lanes stay out of grant materials (code untouched); vocabulary discipline applies.** Owner decision — issue 190 T2; the balanced story is §9 below; verify items in 190 T12.
- **F3 — Product-community evidence.** Nothing external yet. Mitigations: installer download counts (releases API), dist-repo issues/stars, honest internal-deployment-at-scale story. Pre-application pilot: recruit 10–20 external users via Rust community channels. Issue 190 T3/T4 + the **no-submit gate (T10)**.
- **F4 — Receiving entity.** gist-rs registration status + fiscal-sponsor path. Issue 190 T5.

## 4. Eligibility matrix — decision stack (rethink's proposal; full playbook in `../../riir-rethink/.docs/09_human_scale_ai/`)

| Principle | Evidence | Gap |
|---|---|---|
| **Small** | Public engine serves with **zero neural weights** (hashed-feature embedding → corpus routing → drafter scoring → sigmoid calibration), p99 ≤ 1 ms/decision on laptop CPU; encoder tier sub-1B-class; one static binary. | None material. |
| **Intentional** | **Abstention-first** (declines uncalibrated instead of hallucinating); BLAKE3 **decision receipts** (input + canonical decision + engine build); consent-gated stats; open operator-configured escalation grammar (`EscalateSpec`). | Serving weights stay private — acceptable (weights never ship anyway) but stated plainly. |
| **Tailored** | Deployment-ready for offline/local decision needs (education/community apps; small-org triage). | **Weakest axis across both proposals**: served suites today are benchmarks, not a named community. Year-1 co-design is the core of the ask (rethink issue 048 T1). |

## 5. Anchor-project differentiation (prior-art check — searches ran 2026-10-31)

- **Tiles** (education AI commons): quantized open-weight models via llama.cpp + P2P inference (Iroh) + DIDs. *We differ:* no weights at all in the hot path (modelless drafter — orders of magnitude below any quantized model); complementary, not competing.
- **Family Intelligence** (private shared memory protocol): encrypted local-first household memory. *We differ:* repair-memory / decision-memory rather than household-memory; same ethic, different domain.
- **DAIR** (shared renewable compute): infrastructure. *We differ:* we reduce the compute *need* rather than share the compute *supply*.

External-landscape searches (2026-10-31): open local coding tools (Continue, localpilot, DeepCoder-14B) all ride 7B–14B LLMs — **no shipped modelless/corpus-is-the-model healer found**; abstention/selective prediction is rich academic literature (El-Yaniv 2010 → NeurIPS 2025) but **no deployed abstention-first serving stack for communities found**. Supports the novelty criterion without overclaiming (landscape searches, not systematic reviews).

## 6. Cross-cutting consult questions (the email below asks exactly these, v3)

1. **Open-core sustainability.** MIT modelless core + MIT fleet/corpus-production tooling + MIT community corpus; revenue via enterprise licensing of our own GPU-produced corpus and hosted/support services; no credit/ledger/payment functionality is present in the public build (a separate commercial lane exists — disclosed in one sentence — and is out of the funded scope entirely). Acceptable within a funded project?
2. **Entity/fiscal sponsor.** Self-funded team, no clients — is fiscal sponsorship acceptable, and what documentation is required?
3. **Openness timing.** Is a committed, dated open-sourcing plan (core open by month 3) acceptable evidence of "work in the open", or is open source required at application time?

## 7. Evidence inventory (what we can honestly show today)

| Signal | Where | Status |
|---|---|---|
| Working, measured technology | GOAT benches, `.benchmarks/08_benchmarks/` book, score-bench history | ✅ shipped |
| Production deployment (internal) | Default dev-loop tool across the workspace's 32 repos since 2026-08 | ✅ honest, but internal |
| Public distribution surface | `gist-rs/cargo-refine` (releases + install.sh/ps1), Homebrew tap, Scoop bucket | ✅ public |
| Open data commitment | Contributed fix-pairs publish under MIT (consent screen pins it) | ✅ shipped |
| Installer downloads / repo stars / issues | GitHub releases API + repo pages | ⬜ collect (T3) |
| External users | — | ❌ none yet — pilot soft launch (T4) + no-submit gate (T10) |
| Named community in dialogue | — | ❌ the decisive gap; define + open dialogue (T4, W2 in the proposal) |

## 8. Pre-deadline checklist

- [ ] Owner: openness amendment decided (T1) — **by 2026-11-07**
- [ ] Owner: event posture confirmed (T2) — **by 2026-11-07**
- [x] Evidence pack collected (T3) — landed 2026-10-31 by zed-m3-idle-ev190 → [`mozilla_evidence_pack.md`](mozilla_evidence_pack.md)
- [ ] Charitable-build sweep passes (T14) — **before pilot installers ship**
- [ ] Pilot soft launch published (T4), announcing the T11 public-repo-free policy
- [ ] Owner: entity / fiscal sponsor path known (T5) — **by 2026-11-07**
- [ ] Public-copy audit done (T6)
- [ ] Consult email v3 sent (~Nov 7) + info session attended (T7)
- [ ] T11 public-repo-free policy shipped (code + README) — **by 2026-11-17**
- [ ] No-submit gate check (T10) — **by 2026-11-17**
- [ ] Final prose owner-voiced, submitted before Nov 20 11:59 PM PST (T8)
- [-] T12/T13 pinning — Posture B only; off the submission path under Posture A unless the consult answer triggers the owner switch

## 9. Sustainability postures + license strategy (v3 2026-10-31, owner direction)

**POSTURE A — EVENT POSTURE (PRIMARY).** All KAT/credit functionality feature-gated OFF in the public artifact before the application: public installers ship the charitable feature set (no `kat_service`/`kat_transport`); internal dev builds unchanged; the heal path carries no credit/ledger/payment functionality in any command. **Precision (bound by T14):** the inert `riir-kat` accounting-types crate remains linked (non-optional dep, feature-free tier: mining_consent/service_config/soft_gate/units/service_urls compile ungated) — claims use the precise form ("no credit/ledger/payment functionality"), never "fully compiled out"; **T14a** (make `riir-kat` optional behind the features) is the dated engineering upgrade — until it lands and T14 re-passes, the strict zero-graph/zero-strings bar does NOT apply. The application carries **zero credit/ledger/tokenomics claims** — nothing to disclose, nothing to pin, T13 does not apply to the funded scope. Sustainability = open source:

| Layer | License | Moat status |
|---|---|---|
| Modelless core (corpus format + embedder + drafter + pruner + self-evolve) | MIT/Apache-2.0 (OSI) | Deliberately open — the commons |
| Community-contributed fix-pairs (data) | MIT forever (consent screen pins it) | Commons — never sellable |
| Fixer/trainer fleet + corpus-production tooling | MIT (same as the core) | Not the moat — the moat is data + operation, not code |
| Our own GPU-mined corpus (own repos/rigs) | Proprietary; enterprise-licensable | The commercial asset if the grant fails |

**License strategy (corrected per verdict r1):** fleet tooling ships **MIT** — one clean license story. The AGPL/commercial dual-license alternate is recorded but NOT the plan: AGPL-3.0 §13 binds only those offering a *modified* fleet as a hosted service to others — a company producing corpus on its own machines owes nothing, so "share-alike or buy a license" was a false moat claim, and selling commercial licenses would additionally require a CLA on every outside contribution (contributor friction). A non-profit-only license remains refused — it is not open source; if ever used, ALL materials must say "source-available", never "open source". **The real moat:** the GPU-mined corpus (proprietary data), the GPU operation cost (the reproduction barrier), hosted/support services, and the community first-mover position.

**Why opening fixer/trainer loses nothing:** the moat is data + operation + services — not code. Opening invites contributors; the corpus split (community MIT forever / own-mined proprietary) and the service relationships keep the business.

**If the grant fails:** enterprise licensing of our own mined corpus + hosted/support services fund continued work; every community promise (MIT core + MIT fleet + MIT data) stays intact. Same plan either way; only who pays changes.

**POSTURE B — CREDIT POSTURE (ALTERNATE; owner gate after a positive consult answer only).** The v2 metering-plane design: free for OSS repos; contribution-earned credits or post-grant cost-recovery payment; operational bonds with no reward attached in the funded scope; expiring pilot vouchers; offline-first accounting. **Scope boundary (disclosed, never concealed):** the codebase ships a mainnet settlement lane (kat_solana), staking-reward lanes (kat_staking pays a USDC share, `usdc_kat_sub_split_bp` 70%), a KAT↔SEAL swap, and payment rails (KAT_PAYEE_B58 = owner's personal wallet, riir-dapps HISTORY 2026-10-13) — none in the funded scope; the consult email carries the one-sentence disclosure. All Posture-B claims gated by T12/T13 pinning (drop-not-hedge).

**Decision rule:** Posture A ships unless the consult answer explicitly welcomes the credit mechanism; the switch is an owner gate, never a default.

## 10. Timeline to deadline (from 2026-10-31)

| Date | Action |
|---|---|
| Nov 3–7 | Owner decisions on 190 T1/T2/T5 (+ rethink 048 T4); entity/fiscal-sponsor facts gathered |
| Nov 7 | Send consult email v3 to `incubator@mozillafoundation.org` (answers reshape the application before final prose) |
| Nov 12 | Attend info session (17:00 UTC); register when posted |
| Nov 12–17 | Charitable-build sweep (T14) passes → pilot installers; external-pilot soft launch announcing the public-repo-free policy (T11); capture download/issue evidence |
| **Nov 17** | **No-submit gate check (T10)** — decide submit / hold on the evidence |
| Nov 17–19 | Final application prose owner-voiced (only if the gate passes); owner review; submit |
| **Nov 20, 11:59 PM PST** | **Deadline** |

## 11. Consult email draft v2 (send by ~Nov 7; owner sends from owner address)

> **To:** incubator@mozillafoundation.org
> **Subject:** Human-Scale AI Incubator — eligibility questions before we apply
>
> Dear Incubator team,
>
> We are preparing an application for the Human-Scale AI Incubator (deadline Nov 20) and would be grateful for steerage on three eligibility points. (We have consolidated to a single proposal: a local-first, modelless code-healing tool for Rust, which embeds our open decision-serving components for its escalation decisions.)
>
> 1. **Open-core sustainability.** Our tool is free for open-source repositories. We sustain it as a fully open-source project: the healing core AND the fleet/corpus-production tooling are MIT, and the community-contributed repair corpus stays open (MIT) forever. Revenue comes from enterprise licensing of the repair corpus our own GPU infrastructure produces, and from hosted/support services. One plain disclosure: a separate credit/ledger commercial lane exists in our workspace — no part of it is present in the public build, and it is entirely outside the funded scope. Is this sustainability model acceptable within a funded project?
> 2. **Charitable purpose as a self-funded team.** Neither project has revenue or clients yet; the team is self-funded on consumer hardware. Is applying via a fiscal sponsor acceptable, and what documentation do you require?
> 3. **"Work in the open."** The tool currently distributes via public installers while its source is private. We are prepared to open-source the core during the grant term (months 1–3) under a permissive licence and to publish our consent-gated repair-rule corpus as open data. Is a committed, dated open-sourcing plan acceptable evidence of working in the open, or is open source required at application time?
>
> We plan to attend the November 12 info session. Thank you for your time — and for funding this alternative trajectory.
>
> [Owner name + contact]

## 11. Links

- Overview: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/>
- Call for proposals: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/call-for-proposals/>
- Anchor projects: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/cohort/>
- FAQs: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/call-for-proposals/faqs/>
