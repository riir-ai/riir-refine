# Proposal 025 — Mozilla Human-Scale AI Incubator application: cargo refine, the modelless code healer

Status: **draft v3** — one-proposal decision (2026-10-31): refine is THE application; the decision-serving family folds in as the decision layer (§3); [Proposal 005](../../riir-rethink/.proposals/005_mozilla_human_scale_ai_incubator.md) PARKED. v3: **Event posture primary** — credit plane feature-gated OFF in the public artifact; sustainability = open-core + dual-licensed fleet tooling (§4). Still blocked on owner gates G1–G4 + Mozilla consult; nothing submits before 2026-11-12
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned (grant submitter = owner)
Deadline: **2026-11-20 11:59 PM PST** · Info session: **2026-11-12 17:00 UTC** · Consult email: draft in [`../.docs/01_orientation/mozilla_grant_readiness.md`](../.docs/01_orientation/mozilla_grant_readiness.md) (v2 — credit-plane question added)

## TL;DR

We propose to apply to Mozilla Foundation's Human-Scale AI Incubator with **cargo refine**: a local-first, modelless code healer for Rust. Its repair "model" is not a neural network but a distilled, inspectable corpus of repair rules (580+ rules, 6 domains) searched by a zero-allocation ternary SIMD drafter — **zero parameters in the hot path, below Mozilla's own 7B "small" reference point** — running offline on hardware people already own, with the developer's fix memory stored on their own disk and their code never leaving the machine without explicit, redacted, revocable consent. Two design components complete the picture: **(1) the decision layer** — the healer's escalation decisions (attempt local fix / escalate / abstain) run on the open decision-serving stack's operator-configured grammar and audited decision envelope (§3); and **(2) an open-source sustainability model** (§4): everything is MIT — the modelless core, the fleet/corpus-production tooling, and the community-contributed corpus — while revenue comes from enterprise licensing of our own GPU-produced corpus and hosted/support services, and **the credit/ledger plane is compiled out of the public artifact entirely** (Posture A). The grant funds: **(1)** opening the healer's core, **(2)** a pilot with Rust developers who cannot pay cloud-AI subscriptions or assume reliable connectivity, and **(3)** publishing the consented repair-rule corpus + corpus-authoring tools so communities can build and govern their own. **This proposal is a working draft, not final application prose** (Mozilla's AI policy requires owner-voiced submissions — see §Compliance).

## Why this project fits Human-Scale AI

### Small — the zero-parameter hot path

- The L1 drafter is **modelless**: the corpus *is* the model. Distilled rules carry latent vectors; a ternary SIMD matvec scores candidate fixes with sub-millisecond, allocation-free CPU work. No LLM inference, no GPU, no frontier model, no account, no network at heal time.
- One static binary; installs from public installers (GitHub releases, Homebrew tap, Scoop bucket); works fully offline — including credit accounting, which accumulates locally and settles opportunistically when connectivity returns (offline-first by design, for intermittent-connectivity regions).
- Where any larger model exists in the product (hosted escalation tier), it is **opt-in and out of the proposed charitable scope** — the funded build never requires it.
- Contrast with the "not looking for" list: no frontier-model dependence (their explicit exclusion), not a generic chatbot, working and deployed today across a 32-repo production workspace.

### Intentional — the developer owns their repair memory

- **Consent-first telemetry:** the contribution plane never transmits unless the developer opts in (`Unset` never pushes; `OptedOut` never transmits). The consent screen states the **MIT licence for published pairs** — contributed repair knowledge becomes open data by design.
- **Local-first memory:** fix trajectories live in `.refine/` on the developer's disk — inspectable, exportable, purgeable (`--purge`); self-evolution has a documented opt-out.
- **Redaction by default:** the contribution policy refuses repo-private context (`allow_redacted: false`, own-repo + standard source globs only).
- **Disclosed costs:** every credit/fee surface prints in the run footer; nothing is hidden in the heal path.

### Tailored — heals *your* codebase, built *with* a named community

- Corpora are per-domain; `--repo-posture` heals according to the target repository's own lint posture; trajectory memory learns which rules work in *this* codebase (Elo + Beta-posterior selection over the developer's own fix history). The tool adapts to the community's code, not the reverse.
- **Proposed served community:** Rust developers **without cloud-AI budgets** — students and learners, indie and FOSS maintainers, educators, and developers in regions where AI subscriptions, reliable low-latency connectivity, or recent hardware cannot be assumed. Free for open-source repositories by policy; free-tier grant + contribution earn-path for everyone else; the maintainers' own development posture (self-funded, consumer hardware, no datacenter) shares the constraint.

## The decision layer (the open decision-serving component)

The healer does not just apply fixes — it constantly makes **calibrated decisions**: attempt a local modelless fix, escalate the hard case to the hosted tier, or abstain and tell the developer why. That decision machinery runs on the **open decision-serving stack** (the public `riir-reflex` + `riir-instinct` components of our workspace):

- the **operator-configured escalation grammar** (one public definition, manifest-driven — the community/operator decides when their deployment escalates, never a hidden heuristic);
- the **audited decision envelope** (versioned decision contract with typed refusals — every escalation names its tier and reason);
- the abstention-first calibration pattern (decline when uncalibrated, never guess silently).

The private serving-research lane that matured these patterns stays out of charitable scope entirely: no private code, weights, or enforcement planes are funded, opened, or named as deliverables. For the application this is one honest sentence: *the healer embeds our open decision-serving components for its escalation decisions* — and it differentiates the tool: an AI developer tool that knows when it doesn't know.

## The credit plane and the sustainability postures (v3 2026-10-31, owner direction)

**Posture A — EVENT POSTURE (PRIMARY).** All KAT/credit surfaces are **feature-gated and compiled out of the public artifact** before the application: public installers ship the charitable feature set (no `kat_service`/`kat_transport`; internal dev builds unchanged). The application makes **no credit, ledger, or tokenomics claims at all** — so there is nothing to disclose about the sibling commercial lanes, and the payee/staking/transferability questions (T13) do not apply to the funded scope. Sustainability is the **open-core** story below.

**Open-core sustainability (the criterion-6 answer):**

| Layer | License | Moat status |
|---|---|---|
| Modelless core (corpus format + embedder + drafter + pruner + self-evolve) | **MIT/Apache-2.0** — genuine OSI open source, opened months 1–3 | None — deliberately open, it is the commons |
| Community-contributed fix-pairs (data) | **MIT forever** (the consent screen pins it) | Commons — never sellable, by design |
| Fixer/trainer fleet + corpus-production tooling | **MIT — same as the core** (genuinely open source, no restrictions) | Not the moat — the moat is data + operation, not code |
| Our own GPU-mined corpus (from our own repos/rigs) | Proprietary; enterprise-licensable | THE commercial asset — data, not code |

**Why opening fixer/trainer loses nothing — the moat is not code:** (a) the **corpus** the GPU rigs produce is proprietary data — it ships in no repo and no binary; (b) the **GPU operation itself** is a real, ongoing cost — the barrier to reproducing the corpus is compute, not secrets; (c) revenue comes from **enterprise corpus licensing + hosted/support services** — enterprises pay for operation and data, never for the right to run code. A code-restriction boundary was considered and rejected as the default: AGPL-3.0 §13 binds only those offering a *modified* fleet as a hosted service to others — a company producing corpus on its own machines owes nothing, so the license is not the revenue line we might have implied, and selling commercial licenses would additionally require a CLA on every outside contribution. The dual-license alternate is recorded in the playbook (with the CLA task and the corrected AGPL description) as an owner-gated option, not the plan.

**If the grant fails:** the commercial lane continues on its own — enterprise licensing of our own mined corpus + paid commercial licenses for for-profit fleet operation — while the community artifacts (MIT core + MIT corpus + AGPL fleet) stay open exactly as the application promised. The plan is the same either way; only who pays the bills changes.

---

**Posture B — CREDIT POSTURE (ALTERNATE; only if Mozilla's consult answer invites a credit mechanism).** Kept for completeness — the previous design. A metering plane for the hosted tier: free for open-source repositories; contribution-earned credits or post-grant cost-recovery payment (a post-grant product lane, outside the funded scope); operational bonds with **no reward attached in the funded scope**; expiring pilot vouchers; offline-first accounting. **Scope boundary — what the funded scope EXCLUDES:** our codebase today also ships a mainnet settlement lane (kat_solana), staking-reward lanes, a KAT↔SEAL swap, and payment rails (KAT_PAYEE currently = owner's personal wallet) — none of these are the funded project; this posture requires them to be **disclosed to the fundler up front** (one plain sentence in the consult email), never concealed. All claims in this posture are gated by verify-before-claim pinning (T12/T13) with a drop-not-hedge rule.

**Decision rule:** Posture A ships unless Mozilla's consult answer explicitly welcomes the credit mechanism; switching to Posture B is an owner gate after the answer, never a default.

## The charitable purpose

Keep software development affordable, private, and accessible for developers excluded from cloud-AI economics, and leave the resulting knowledge in the commons:

1. **Open code.** Open the modelless core (corpus format + embedder + drafter + pruner + self-evolve loop) under MIT or Apache-2.0 in months 1–3 of the term, with public CI, benchmarks, and docs. *(Amends the Research 003 private-source posture for this repo — owner gate G1.)*
2. **Open data.** Publish the consented repair-rule corpus under MIT (already the licence of contributed pairs) with a corpus-authoring guide so communities can distill, own, and govern rule corpora for their own languages and codebases.
3. **Free for the commons.** Ship and document the public-repository-free policy as a durable product commitment.
4. **Open fleet tooling.** Open the fixer/trainer service + corpus-production tooling under the same MIT license as the core — one license story, no restrictions; revenue is corpus + services, not code.
5. **Measured public benefit.** Published heal-rate and latency benchmarks; a public pilot cohort with results reported openly at the Mozilla Festival showcase.

## Year-1 workplan (what the $300K funds)

| # | Work | Quarter | Output |
|---|---|---|---|
| W1 | **Open-source the core**: extract the modelless pipeline into a public repo (MIT/Apache-2.0), public CI + benchmarks, contribution guide | Q1 | Public repo + docs; dated open-sourcing commitment in the application |
| W1b | **Open the fleet tooling (MIT)**: fixer/trainer service + corpus-production tooling published under the same permissive license as the core; license-and-boundaries table in the public README (the corpus split: community-contributed = MIT forever; own-GPU-mined = proprietary) | Q2 | One clean license story — the business is corpus + services, not code restriction |
| W2 | **Public-repo-free policy shipped**: the free-for-open-source gate implemented, documented, and announced as product policy | Q1 | The charitable commitment is code, not prose |
| W3 | **Community pilot**: 30–50 Rust developers from resource-constrained cohorts (student groups, FOSS maintainers, regional dev communities). Funded as **service capacity + expiring pilot vouchers** — pilot users never pay and never need to contribute | Q1–Q2 | Pilot report: heal rates, retention, offline usage, quotes |
| W4 | **Corpus tools + governance**: publish corpus-authoring tooling + a governance README; onboard 1–2 external corpus contributors | Q2 | Open corpus + governance framework |
| W5 | **Second language domain** via community contribution (proves the framework generalizes and creates a contributor surface) | Q3 | One community-built domain corpus |
| W6 | **Sustainability (open-core)**: charitable feature set shipped as the public artifact (credit plane compiled out — T14); license boundaries published (MIT core / MIT community data / dual-licensed fleet tooling / proprietary own-mined corpus); durable legal home (fiscal sponsor → foundation path); if the grant fails, the enterprise corpus + fleet-license lane funds continued work with every community promise intact | Q4 | Sustainability plan per Mozilla's follow-on criteria |

Budget shape (indicative): ~60% people (2–3 contributors), ~15% pilot + community (service capacity, voucher subsidies, travel to one Mozilla Festival), ~15% infrastructure + audit, ~10% contingency. Grant money funds capacity and pilot vouchers — never speculative credit supply. No trading, no profit line in the funded scope.

## Self-grade against Mozilla's six criteria (honest)

| Criterion | Grade | Evidence / gap |
|---|---|---|
| Human-Scale AI expression | **Strong** | All three principles mapped above; the zero-parameter hot path is the headline. |
| Technical viability | **Strong** | Working, deployed, measured (GOAT-gated benches; production use across our own 32-repo workspace since 2026-08); team of experienced Rust engineers with capacity. Novelty: landscape search (2026-10-31, playbook §5) found no shipped modelless healer — local coding tools all ride 7B–14B LLMs. |
| Problem-community clarity | **Medium** | Community defined (resource-constrained Rust devs) but not yet in dialogue — pilot W3 + pre-application outreach are the fix (issue 190 T4). |
| Product-community fit | **Weak — the decisive risk** | No external users yet. Mitigations: installer download counts, dist-repo issues/stars, internal-deployment-at-scale story, soft-launch pilot recruitment; the no-submit gate (190 T10) keeps submission honest. |
| Impact and values | **Strong** | Open code + open data + free-for-the-commons policy + privacy-by-default + measured benefit. |
| Sustainability potential | **Medium-strong** | Open-core end to end (MIT core + MIT fleet tooling) with revenue from enterprise corpus licensing + hosted/support services — a standard, checkable answer to "how does this endure" (their criterion 6); durable home via W6; team open to models. Re-grade if the model shifts. |

## Owner gates (must resolve before submission)

- **G1 — Openness amendment** (decide by 2026-11-07). Approve opening the modelless core under MIT/Apache-2.0 with a dated commitment (amends Research 003 for riir-refine; KAT/commercial surfaces stay private). Issue 190 T1.
- **G2 — Event posture (decide by 2026-11-07).** (a) Confirm Posture A: credit plane feature-gated OFF in the public artifact (charitable feature set: no `kat_service`/`kat_transport`), internal dev builds unchanged; (b) charitable-build sweep passes (T14) before installers ship; (c) grant narrative carries NO credit/ledger claims (Posture B lives only as the alternate, switched in by an explicit owner gate if the consult answer invites it); (d) license boundaries published as in the open-core table (MIT core + MIT fleet + proprietary own-mined corpus); the dual-license fleet alternate is owner-gated, never the default. Issue 190 T2 + T14.
- **G3 — Evidence pack** (executable now). Installer downloads, dist-repo signals, internal-usage stats into the application. Issue 190 T3.
- **G4 — Entity + submission** (decide by 2026-11-07; prose Nov 17–19 only if the no-submit gate passes). Fiscal sponsor resolved; final prose owner-voiced; single proposal submitted. Issue 190 T5/T7/T8/T10.

## Compliance notes

- **Mozilla AI-use policy:** this draft is internal preparation material. Submitted answers must be written by the owner in their own voice; agent text is source material, never paste-ready prose.
- **Vocabulary discipline:** §4's binding word list applies to every grant-facing document.
- **Verify-before-claim (Posture B only):** if — and only if — the consult answer switches the application to the credit posture: chain-fee posture, no-reward bond variant, voucher operator-batch mechanics, credit transferability, and payee entity all pin from source with commit refs (T12/T13); anything unpinnable by Nov 17 is dropped, not hedged. Under Posture A none of these claims appear anywhere.
- **No fabrication:** the pilot (W3) and partner names are TODOs, not achievements; the application says "recruiting now" honestly.

## Related

- [katgpt-rs Research 642](../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md) — public program digest; full strategy in the private readiness playbook
- [refine issue 190](../.issues/190_mozilla_grant_readiness_gaps.md) — the readiness task list (v2: T11 public-repo-free policy, T12 verify-before-claim, T13 credit-scope integrity)
- [refine .docs/01_orientation/mozilla_grant_readiness.md](../.docs/01_orientation/mozilla_grant_readiness.md) — the private playbook (v3: §9 the two postures + license strategy, §11 the consult email)
- [riir-rethink Proposal 005](../../riir-rethink/.proposals/005_mozilla_human_scale_ai_incubator.md) — PARKED (superseded by the one-proposal decision); its community-candidate work lives on as product follow-up
- [Research 003](../../katgpt-rs/.research/003_Commercial_Open_Source_Strategy_Verdict.md) — the openness law this application proposes to amend for the charitable core
