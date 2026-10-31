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

## 2. Eligibility verdict (decide here, act in issue 190)

**Both candidates conditionally eligible — prepare both, consult Mozilla, decide scope after their answer.**

- **cargo refine — PRIMARY.** Strongest Small+Intentional story in the workspace; openness and token frictions are decidable before the deadline; community evidence is the real risk and partially mitigable in the time left. One-line reason: zero-parameter hot path + consent-first data flows + per-repo tailored healing is a near-textbook Human-Scale AI expression, *if* the openness amendment lands.
- **Decision stack (reflex + instinct) — SECONDARY.** Public code + abstention-first honesty is a strong Intentional story; the missing named community is the decisive gap and cannot be fully closed before Nov 20. One-line reason: strongest existing open-source surface, weakest community narrative.
- **Not eligible as-is:** either under its current private-forever + commercial posture — the frictions below exist to fix exactly that.

## 3. Eligibility matrix — cargo refine (§ marks the proposal sections)

| Principle | Evidence we already ship | Gap |
|---|---|---|
| **Small** | The L1 drafter is **modelless**: 580+ distilled repair rules across 6 domains are the model — ternary SIMD matvec over latent rule vectors, zero-alloc, sub-ms, CPU-only. No LLM, no GPU, no API, works offline, one static binary. **Below Mozilla's own 7B reference floor: zero parameters in the hot path.** Larger-model lanes (hosted escalation, L4) are opt-in and out of proposed scope. | None material. |
| **Intentional** | Consent-first telemetry (Unset never pushes; OptedOut never transmits); fix-trajectory memory in `.refine/` on the dev's disk (inspect/export/`--purge`); consent screen pins the **MIT licence for published pairs**; redaction refuses repo-private context (`allow_redacted: false`); every cost surface disclosed in the run footer. | The credit plane (KAT) sits on the same surface — friction F2. |
| **Tailored** | Per-domain corpora; `--repo-posture` heals by the target repo's own lint posture; trajectory memory learns which rules work in *this* codebase (Elo + Beta-posterior over the dev's own fix history). Proposed community: **Rust developers without cloud-AI budgets** — students, indie maintainers, educators, resource-constrained regions. | **No external community yet** — pilot must be designed (issue 190 T4). |

**Frictions:**

- **F1 — Openness.** Research 003 law: "anything riir-* is internal… prebuilt binaries only." Mozilla expects work in the open + open code left behind. Dist surface (installer repo, Homebrew tap, Scoop bucket) is already public; the consented corpus is already MIT data. **Proposed amendment: open the modelless core (corpus format + embedder + drafter + pruner + self-evolve) under MIT/Apache-2.0, months 1–3 of the term.** Owner decision — issue 190 T1. (This amendment language lives here and in issue 190 — private repo only.)
- **F2 — KAT/token posture.** A token-mediated pay plane on a devnet ledger reads commercial/crypto against the charitable framing. **Freeze all pay/lease/purchase lanes out of the funded scope** (code stays; no tokens marketed; no soft-gate refusals in the charitable build). Owner decision — issue 190 T2.
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

## 6. Cross-cutting consult questions (the email below asks exactly these)

1. **Two proposals, one team.** The FAQ's "apply to both" answer covers Fellowship-vs-Incubator, not two Incubator proposals from one organization — ask.
2. **Entity/fiscal sponsor.** Self-funded team, no clients — is fiscal sponsorship acceptable, and what documentation is required?
3. **Openness timing.** Is a committed, dated open-sourcing plan (core open by month 3) acceptable evidence of "work in the open", or is open source required at application time?
4. **Token-adjacent history.** One project carries an opt-in contribution-credit plane on a devnet ledger (nothing sold, nothing marketed). Must the charitable scope be fully crypto-free, or is a frozen/out-of-scope posture acceptable?

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
- [ ] Owner: KAT freeze mechanics decided (T2) — **by 2026-11-07**
- [ ] Evidence pack collected (T3)
- [ ] Pilot soft launch published (T4)
- [ ] Owner: entity / fiscal sponsor path known (T5) — **by 2026-11-07**
- [ ] Public-copy audit done (T6)
- [ ] Consult email sent (~Nov 7) + info session attended (T7)
- [ ] No-submit gate check (T10) — **by 2026-11-17**
- [ ] Final prose owner-voiced, submitted before Nov 20 11:59 PM PST (T8)

## 9. Timeline to deadline (from 2026-10-31)

| Date | Action |
|---|---|
| Nov 3–7 | Owner decisions on 190 T1/T2 (+ rethink 048 T1/T2/T4); entity/fiscal-sponsor facts gathered |
| Nov 7 | Send consult email to `incubator@mozillafoundation.org` (answers reshape scope before final prose) |
| Nov 12 | Attend info session (17:00 UTC); register when posted |
| Nov 12–17 | External-pilot soft launch: call for pilot users in Rust community channels; capture download/issue evidence pack |
| **Nov 17** | **No-submit gate check (T10)** — decide submit / hold on the evidence |
| Nov 17–19 | Final application prose owner-voiced (only if the gate passes); owner review; submit both (or one, per consult answer) |
| **Nov 20, 11:59 PM PST** | **Deadline** |

## 10. Consult email draft (send by ~Nov 7; owner sends from owner address)

> **To:** incubator@mozillafoundation.org
> **Subject:** Human-Scale AI Incubator — eligibility questions before we apply
>
> Dear Incubator team,
>
> We are preparing applications for the Human-Scale AI Incubator (deadline Nov 20) and would be grateful for steerage on four eligibility points:
>
> 1. **Two projects, one team.** We build two working, related technologies: (a) a local-first, modelless code-healing tool for Rust — its repair "model" is a distilled, inspectable corpus of rules searched with zero neural parameters, running offline on hardware people already own, with the developer's fix memory on their own disk and nothing leaving the machine without explicit, redacted consent; and (b) an abstention-first decision-serving stack (open-source engine that classifies and routes with calibrated confidence, refuses to answer when unsure, and runs sub-millisecond on a laptop CPU with no large-model dependency). May one team submit two separate proposals to this cohort, or should we select one? We are happy to consolidate if two applications from one team would disadvantage either.
> 2. **Charitable purpose as a self-funded team.** Neither project has revenue or clients yet; the team is self-funded on consumer hardware. Is applying via a fiscal sponsor acceptable, and what documentation do you require?
> 3. **"Work in the open."** Project (a) currently distributes via public installers while its source is private. We are prepared to open-source the core during the grant term (months 1–3) under a permissive licence and to publish our consent-gated repair-rule corpus as open data. Is a committed, dated open-sourcing plan acceptable evidence of working in the open, or is open source required at application time?
> 4. **Token-adjacent history.** One project carries an opt-in contribution-credit plane currently on a test-network ledger (nothing sold, nothing marketed). Will the funded scope need to be fully crypto-free, or is a frozen/out-of-scope posture acceptable?
>
> We plan to attend the November 12 info session. Thank you for your time — and for funding this alternative trajectory.
>
> [Owner name + contact]

## 11. Links

- Overview: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/>
- Call for proposals: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/call-for-proposals/>
- Anchor projects: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/cohort/>
- FAQs: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/call-for-proposals/faqs/>
