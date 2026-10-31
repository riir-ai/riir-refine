# Proposal 025 — Mozilla Human-Scale AI Incubator application: cargo refine, the modelless code healer

Status: **draft v2** — ONE-PROPOSAL decision recorded 2026-10-31 (owner): refine is THE application; the decision-serving family folds in as the decision layer (§3); [Proposal 005](../../riir-rethink/.proposals/005_mozilla_human_scale_ai_incubator.md) PARKED. Still blocked on owner gates G1–G4 + Mozilla consult; nothing submits before 2026-11-12
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned (grant submitter = owner)
Deadline: **2026-11-20 11:59 PM PST** · Info session: **2026-11-12 17:00 UTC** · Consult email: draft in [`../.docs/01_orientation/mozilla_grant_readiness.md`](../.docs/01_orientation/mozilla_grant_readiness.md) (v2 — credit-plane question added)

## TL;DR

We propose to apply to Mozilla Foundation's Human-Scale AI Incubator with **cargo refine**: a local-first, modelless code healer for Rust. Its repair "model" is not a neural network but a distilled, inspectable corpus of repair rules (580+ rules, 6 domains) searched by a zero-allocation ternary SIMD drafter — **zero parameters in the hot path, below Mozilla's own 7B "small" reference point** — running offline on hardware people already own, with the developer's fix memory stored on their own disk and their code never leaving the machine without explicit, redacted, revocable consent. Two design components complete the picture: **(1) the decision layer** — the healer's escalation decisions (attempt local fix / escalate / abstain) run on the open decision-serving stack's operator-configured grammar and audited decision envelope (§3), and **(2) the contribution-credit plane** — the sustainability mechanism that keeps the free tier alive: free for open-source repositories, contribution-earned credits or cost-recovery payment for private use, operational bonds for service integrity, no speculation (§4). The grant funds: **(1)** opening the healer's core under a permissive licence, **(2)** a pilot with Rust developers who cannot pay cloud-AI subscriptions or assume reliable connectivity (funded as service capacity + expiring pilot vouchers), and **(3)** publishing the consented repair-rule corpus + corpus-authoring tools so communities can build and govern their own. **This proposal is a working draft, not final application prose** (Mozilla's AI policy requires owner-voiced submissions — see §Compliance).

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

## The contribution-credit plane — sustainability without profit

The healer is a shared compute service at scale (local fixes cost nothing; the hosted tier and verification runs cost real compute on contributor rigs). The sustainability mechanism is **mechanism design, not profit**:

| Design element | What it is | What it is NOT |
|---|---|---|
| **Free for open-source repositories** | Product policy (shipping pre-application): heals on public/open-source repos cost nothing, forever — the grant makes it durable, not possible | Not a limited promo |
| **Contribution-earned credits** | The primary earn path: contribute redacted fix-pairs under an open (MIT) licence → earn service credit. The commons grows as it is used | Not pay-to-play; an individual never must pay money |
| **Cost-recovery payment** | The alternative path for organizations that prefer money to contribution: priced to recover running cost | Not a profit line — the entity is self-funded, no investors, no return |
| **Operational bonds** | Service operators (fixer/trainer rigs) post a bond covering penalties for bad service (wrong fixes, oracle cheating, downtime) — integrity by accountability. In the funded scope, bonds carry **no reward attached** (pin a no-reward bond variant or drop the claim — T12) | Not investment: within the funded scope, no yield, no appreciation, no trading |
| **Expiring pilot vouchers** | Onboarding credit for cohorts (pilots, students) — expiring, non-transferable | Not speculative supply |
| **Offline-first accounting** | Work and credit accumulate locally; settle opportunistically when online — built for intermittent connectivity | Not a closed system; the ledger is a coordination record |
| **No-fee coordination ledger** | The credit ledger records; it does not tax (chain-fee posture to be verified and pinned before any external claim) | Not a revenue source |
| **Scope boundary — what the funded scope EXCLUDES** | Our codebase today also ships a mainnet settlement lane, staking-reward lanes, a currency swap, and payment rails. **None of these are part of the funded project**: the charitable scope uses only non-transferable credits and expiring vouchers, and excludes the payment, staking-reward, and exchange lanes entirely | Not hidden — disclosed to the finder up front (consult email Q1) and enforced as the grant's scope boundary |

**Why this is legible to reviewers:** sybil resistance and cost recovery for shared compute are standard distributed-systems practice — the same genus as mutual-credit / time-bank systems and storage-provider collateral, and the p2p patterns in Mozilla's own anchor projects (Iroh-based inference sharing, shared renewable compute). The economy exists **to keep the free tier alive**, not to extract: contribute-or-cost-recovery means the resource-constrained developer is never excluded, and the open-source commons is subsidized by design.

**Vocabulary discipline (binding for all grant materials):** never "token", "airdrop", "mint", "staking", "crypto" — say "contribution credits", "expiring pilot vouchers", "operational bonds", "coordination ledger". The mechanism is real; the words carry the posture.

## The charitable purpose

Keep software development affordable, private, and accessible for developers excluded from cloud-AI economics, and leave the resulting knowledge in the commons:

1. **Open code.** Open the modelless core (corpus format + embedder + drafter + pruner + self-evolve loop) under MIT or Apache-2.0 in months 1–3 of the term, with public CI, benchmarks, and docs. *(Amends the Research 003 private-source posture for this repo — owner gate G1.)*
2. **Open data.** Publish the consented repair-rule corpus under MIT (already the licence of contributed pairs) with a corpus-authoring guide so communities can distill, own, and govern rule corpora for their own languages and codebases.
3. **Free for the commons.** Ship and document the public-repository-free policy as a durable product commitment.
4. **Measured public benefit.** Published heal-rate and latency benchmarks; a public pilot cohort with results reported openly at the Mozilla Festival showcase.

## Year-1 workplan (what the $300K funds)

| # | Work | Quarter | Output |
|---|---|---|---|
| W1 | **Open-source the core**: extract the modelless pipeline into a public repo (MIT/Apache-2.0), public CI + benchmarks, contribution guide | Q1 | Public repo + docs; dated open-sourcing commitment in the application |
| W2 | **Public-repo-free policy shipped**: the free-for-open-source gate implemented, documented, and announced as product policy | Q1 | The charitable commitment is code, not prose |
| W3 | **Community pilot**: 30–50 Rust developers from resource-constrained cohorts (student groups, FOSS maintainers, regional dev communities). Funded as **service capacity + expiring pilot vouchers** — pilot users never pay and never need to contribute | Q1–Q2 | Pilot report: heal rates, retention, offline usage, quotes |
| W4 | **Corpus tools + governance**: publish corpus-authoring tooling + a governance README; onboard 1–2 external corpus contributors | Q2 | Open corpus + governance framework |
| W5 | **Second language domain** via community contribution (proves the framework generalizes and creates a contributor surface) | Q3 | One community-built domain corpus |
| W6 | **Sustainability**: the credit plane carries the free tier after the grant (contribution earn-path + cost recovery + bonds); durable legal home (fiscal sponsor → foundation path); Mozilla and peer funders engaged as **ongoing sponsors of the free tier** (sponsor-lane posture for future cohorts of pilot vouchers / capacity) | Q4 | Sustainability plan per Mozilla's follow-on criteria |

Budget shape (indicative): ~60% people (2–3 contributors), ~15% pilot + community (service capacity, voucher subsidies, travel to one Mozilla Festival), ~15% infrastructure + audit, ~10% contingency. Grant money funds capacity and pilot vouchers — never speculative credit supply. No trading, no profit line in the funded scope.

## Self-grade against Mozilla's six criteria (honest)

| Criterion | Grade | Evidence / gap |
|---|---|---|
| Human-Scale AI expression | **Strong** | All three principles mapped above; the zero-parameter hot path is the headline. |
| Technical viability | **Strong** | Working, deployed, measured (GOAT-gated benches; production use across our own 32-repo workspace since 2026-08); team of experienced Rust engineers with capacity. Novelty: landscape search (2026-10-31, playbook §5) found no shipped modelless healer — local coding tools all ride 7B–14B LLMs. |
| Problem-community clarity | **Medium** | Community defined (resource-constrained Rust devs) but not yet in dialogue — pilot W3 + pre-application outreach are the fix (issue 190 T4). |
| Product-community fit | **Weak — the decisive risk** | No external users yet. Mitigations: installer download counts, dist-repo issues/stars, internal-deployment-at-scale story, soft-launch pilot recruitment; the no-submit gate (190 T10) keeps submission honest. |
| Impact and values | **Strong** | Open code + open data + free-for-the-commons policy + privacy-by-default + measured benefit. |
| Sustainability potential | **Medium-strong** | The credit plane is a concrete, mechanism-design answer to "how does the free tier endure" (their criterion 6); durable home via W6; team open to models. |

## Owner gates (must resolve before submission)

- **G1 — Openness amendment** (decide by 2026-11-07). Approve opening the modelless core under MIT/Apache-2.0 with a dated commitment (amends Research 003 for riir-refine; KAT/commercial surfaces stay private). Issue 190 T1.
- **G2 — Credit-plane posture** (decide by 2026-11-07, supersedes the v1 "KAT freeze" gate). (a) Ship public-repo-free as product policy pre-application; (b) grant narrative uses the §4 vocabulary discipline; (c) pay/lease/purchase lanes stay out of grant materials (code untouched); (d) pilot funding mode: service capacity + expiring vouchers; (e) the §4 scope-boundary row is honored — mainnet settlement, staking-reward, swap, and payment lanes are excluded from the funded scope and DISCLOSED (not concealed) in the consult email. Issue 190 T2 + T11/T12.
- **G3 — Evidence pack** (executable now). Installer downloads, dist-repo signals, internal-usage stats into the application. Issue 190 T3.
- **G4 — Entity + submission** (decide by 2026-11-07; prose Nov 17–19 only if the no-submit gate passes). Fiscal sponsor resolved; final prose owner-voiced; single proposal submitted. Issue 190 T5/T7/T8/T10.

## Compliance notes

- **Mozilla AI-use policy:** this draft is internal preparation material. Submitted answers must be written by the owner in their own voice; agent text is source material, never paste-ready prose.
- **Vocabulary discipline:** §4's binding word list applies to every grant-facing document.
- **Verify-before-claim (blocking external use):** chain-fee posture ("no fees" must be pinned against riir-chain's shipped economics before it appears anywhere); fixer/trainer bond + penalty parameters pinned from riir-refine/riir-chain docs; pilot-voucher allocation mechanics confirmed as a lawful operator batch. Issue 190 T12.
- **No fabrication:** the pilot (W3) and partner names are TODOs, not achievements; the application says "recruiting now" honestly.

## Related

- [katgpt-rs Research 642](../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md) — public program digest; full strategy in the private readiness playbook
- [refine issue 190](../.issues/190_mozilla_grant_readiness_gaps.md) — the readiness task list (v2: T11 public-repo-free policy, T12 credit-plane verify)
- [refine .docs/01_orientation/mozilla_grant_readiness.md](../.docs/01_orientation/mozilla_grant_readiness.md) — the private playbook (v2: §9 the credit-plane story, §11 the consult email)
- [riir-rethink Proposal 005](../../riir-rethink/.proposals/005_mozilla_human_scale_ai_incubator.md) — PARKED (superseded by the one-proposal decision); its community-candidate work lives on as product follow-up
- [Research 003](../../katgpt-rs/.research/003_Commercial_Open_Source_Strategy_Verdict.md) — the openness law this application proposes to amend for the charitable core
