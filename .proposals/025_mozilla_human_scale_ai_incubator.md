# Proposal 025 — Mozilla Human-Scale AI Incubator application: cargo refine, the modelless code healer

Status: **draft** — eligibility research done ([katgpt-rs Research 642](../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md)); blocked on owner gates G1–G4 below + Mozilla consult; nothing submits before 2026-11-12
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned (grant submitter = owner)
Deadline: **2026-11-20 11:59 PM PST** · Info session: **2026-11-12 17:00 UTC** · Consult email: draft in [`../.docs/01_orientation/mozilla_grant_readiness.md`](../.docs/01_orientation/mozilla_grant_readiness.md)
Companion: [riir-rethink Proposal 005](../../riir-rethink/.proposals/005_mozilla_human_scale_ai_incubator.md) — the second candidate; which one (or both) submits is a consult outcome

## TL;DR

We propose to apply to Mozilla Foundation's Human-Scale AI Incubator with **cargo refine**: a local-first, modelless code healer for Rust. Its repair "model" is not a neural network but a distilled, inspectable corpus of repair rules (580+ rules, 6 domains) searched by a zero-allocation ternary SIMD drafter — **zero parameters in the hot path, below Mozilla's own 7B "small" reference point** — running offline on hardware people already own, with the developer's fix memory stored on their own disk and their code never leaving the machine without explicit, redacted, revocable consent. The grant funds three things: **(1)** opening the healer's core under a permissive licence, **(2)** a pilot with Rust developers who cannot pay cloud-AI subscriptions or assume reliable connectivity (students, indie maintainers, resource-constrained regions), and **(3)** publishing the consented repair-rule corpus + corpus-authoring tools so communities can build and govern their own. **This proposal is a working draft, not final application prose** (Mozilla's AI policy requires owner-voiced submissions — see §Compliance).

## Why this project fits Human-Scale AI

### Small — the zero-parameter hot path

- The L1 drafter is **modelless**: the corpus *is* the model. Distilled rules carry latent vectors; a ternary SIMD matvec scores candidate fixes with sub-millisecond, allocation-free CPU work. No LLM inference, no GPU, no frontier model, no account, no network at heal time.
- One static binary; installs from public installers (GitHub releases, Homebrew tap, Scoop bucket); works fully offline.
- Where any larger model exists in the product (hosted escalation, L4 fallback), it is **opt-in and out of the proposed charitable scope** — the funded build never requires it.
- Contrast with the "not looking for" list: we do not depend on a frontier or slimmed-frontier model (their explicit exclusion), we are not a generic chatbot, and the technology is not a concept — it ships and heals today across a 32-repo production workspace.

### Intentional — the developer owns their repair memory

- **Consent-first telemetry:** the contribution plane never transmits unless the developer opts in (`Unset` never pushes; `OptedOut` never transmits). The consent screen states the **MIT licence for published pairs** — contributed repair knowledge becomes open data by design.
- **Local-first memory:** fix trajectories live in `.refine/` on the developer's disk — inspectable, exportable, purgeable (`--purge`); self-evolution has a documented opt-out.
- **Redaction by default:** the contribution policy refuses repo-private context (`allow_redacted: false`, own-repo + standard source globs only).
- **Disclosed costs:** every credit/fee surface prints in the run footer; nothing is hidden in the heal path.

### Tailored — heals *your* codebase, built *with* a named community

- Corpora are per-domain; `--repo-posture` heals according to the target repository's own lint posture; trajectory memory learns which rules work in *this* codebase (Elo + Beta-posterior selection over the developer's own fix history). The tool adapts to the community's code, not the reverse.
- **Proposed served community:** Rust developers **without cloud-AI budgets** — students and learners, indie and FOSS maintainers, educators, and developers in regions where AI subscriptions, reliable low-latency connectivity, or recent hardware cannot be assumed. The tool is free, offline-capable, and sized for old machines by construction; this population is structurally excluded from today's cloud-AI coding tools, and the maintainers' own development posture (self-funded, consumer hardware, no datacenter) shares the constraint.

## The charitable purpose

Keep software development affordable, private, and accessible for developers excluded from cloud-AI economics, and leave the resulting knowledge in the commons:

1. **Open code.** Open the modelless core (corpus format + embedder + drafter + pruner + self-evolve loop) under MIT or Apache-2.0 in months 1–3 of the term, with public CI, benchmarks, and docs. *(Amends the Research 003 private-source posture for this repo — owner gate G1.)*
2. **Open data.** Publish the consented repair-rule corpus under MIT (already the licence of contributed pairs) with a corpus-authoring guide so communities can distill, own, and govern rule corpora for their own languages and codebases.
3. **Measured public benefit.** Published heal-rate and latency benchmarks; a public pilot cohort with results reported openly at the Mozilla Festival showcase.

## Year-1 workplan (what the $300K funds)

| # | Work | Quarter | Output |
|---|---|---|---|
| W1 | **Open-source the core**: extract the modelless pipeline into a public repo (MIT/Apache-2.0), public CI + benchmarks, contribution guide | Q1 | Public repo + docs; dated open-sourcing commitment in the application |
| W2 | **Community pilot**: 30–50 Rust developers from resource-constrained cohorts (student groups, FOSS maintainers, regional dev communities — recruitment via Rust community channels; specific partners named before submission where possible) | Q1–Q2 | Pilot report: heal rates, retention, offline usage, quotes |
| W3 | **Corpus tools + governance**: publish corpus-authoring tooling + a governance README; onboard 1–2 external corpus contributors | Q2 | Open corpus + governance framework |
| W4 | **Second language domain** via community contribution (proves the framework generalizes and creates a contributor surface) | Q3 | One community-built domain corpus |
| W5 | **Sustainability**: establish the durable home (fiscal sponsor → or open-collective / foundation path); decide the open-core boundary for any future hosted tier (kept outside charitable scope) | Q4 | Sustainability plan per Mozilla's follow-on criteria |

Budget shape (indicative): ~60% people (2–3 contributors), ~15% pilot + community (stipends, travel to one Mozilla Festival), ~15% infrastructure + audit, ~10% contingency. No tokens, no crypto, no paid API dependencies in the funded scope.

## Self-grade against Mozilla's six criteria (honest)

| Criterion | Grade | Evidence / gap |
|---|---|---|
| Human-Scale AI expression | **Strong** | All three principles mapped above; the zero-parameter hot path is the headline. |
| Technical viability | **Strong** | Working, deployed, measured (GOAT-gated benches; production use across our own 32-repo workspace since 2026-08); team of experienced Rust engineers with capacity. Novelty: landscape search (2026-10-31, recorded in the readiness playbook §5) found no shipped modelless healer — local coding tools all ride 7B–14B LLMs. |
| Problem-community clarity | **Medium** | Community defined (resource-constrained Rust devs) but not yet in dialogue — pilot W2 + pre-application outreach are the fix (issue 190 T4). |
| Product-community fit | **Weak — the decisive risk** | No external users yet. Mitigations to gather before submission: public installer download counts, dist-repo issues/stars, honest internal-deployment scale, soft-launch pilot recruitment. (Owner gate G3.) |
| Impact and values | **Strong** | Open code + open data + privacy-by-default + measured benefit; explicitly builds public-good developer infrastructure. |
| Sustainability potential | **Medium** | No revenue today (helpful for charitability, weak for sustainability); W5 + open-core boundary is the plan; team is open to different models (their criterion). |

## Owner gates (must resolve before submission)

- **G1 — Openness amendment.** Approve opening the modelless core under MIT/Apache-2.0 with a dated commitment (amends Research 003 for riir-refine; the KAT/commercial surfaces stay private if desired — the charitable scope is the modelless core). Filed: issue 190 T1.
- **G2 — KAT freeze for the funded scope.** All pay/lease/purchase lanes and the soft-gate refusal stay out of the charitable build; no token marketing in grant materials. Filed: issue 190 T2.
- **G3 — Evidence pack.** Collect installer downloads, dist-repo signals, internal-usage stats (batch/heal counts from our own benches) into the application. Filed: issue 190 T3.
- **G4 — Entity + submission.** Receiving entity / fiscal sponsor resolved; final prose owner-voiced; submits one or both proposals per the Mozilla consult. Filed: issue 190 T5/T7/T8 + rethink issue 048 T6 (the two-application question).

## Compliance notes

- **Mozilla AI-use policy:** this draft is internal preparation material. Submitted answers must be written by the owner in their own voice; agent text is source material, never paste-ready prose.
- **Public-safe:** nothing here names credentials, private internals beyond what the application itself would state, or third-party promises we cannot keep.
- **No fabrication:** the pilot (W2) and partner names are TODOs, not achievements; the application will say "recruiting now" honestly, per the FAQ's "early evidence" bar.

## Related

- [katgpt-rs Research 642](../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md) — public program digest; full eligibility matrices + strategy in the private readiness playbook
- [refine issue 190](../.issues/190_mozilla_grant_readiness_gaps.md) — the readiness task list
- [refine .docs/01_orientation/mozilla_grant_readiness.md](../.docs/01_orientation/mozilla_grant_readiness.md) — the private playbook: matrices, verdicts, frictions, evidence inventory, consult-email draft, timeline
- [Research 003](../../katgpt-rs/.research/003_Commercial_Open_Source_Strategy_Verdict.md) — the openness law this application proposes to amend for the charitable core
