# Mozilla Human-Scale AI Incubator — grant readiness (the private playbook)

> **Purpose:** the full grant-strategy playbook — program facts, per-repo
> eligibility matrices, verdicts, frictions, the evidence inventory, the
> consult email, and the timeline. Program digest (public-safe):
> [katgpt-rs Research 642](../../../katgpt-rs/.research/642_Mozilla_Human_Scale_AI_Grant.md).
> Working proposal: [`../../.proposals/025_mozilla_human_scale_ai_incubator.md`](../../.proposals/025_mozilla_human_scale_ai_incubator.md) (v4).
> Task issues: [`../../.issues/190_…`](../../.issues/190_mozilla_grant_readiness_gaps.md) + the v4 pivot (issue 192 — CLOSED + removed 2026-11-10, record in `HISTORY.md`; the durable model lives in [`../../.proposals/025_mozilla_human_scale_ai_incubator.md`](../../.proposals/025_mozilla_human_scale_ai_incubator.md) §"The points plane in full").
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
- **Decision stack — folded in as the decision layer.** Honest scoping: the application names the OPEN components the rulewright consumes (reflex decision envelope + instinct escalation grammar); the private rung, weights, and economy planes are out of scope by name.
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
| Public distribution surface | `riir-ai/cargo-refine` (releases + install.sh/ps1), Homebrew tap, Scoop bucket, **crates.io** (`cargo-refine` 0.5.1, live 2026-11-11 — `cargo install cargo-refine`) | ✅ public |
| Upstream acceptance | mozilla/sccache #2977 **MERGED** (2026-11-07, `907230be`, six review rounds); #3025 (backend read backoff) **open + full CI matrix green** (50 pass / 1 by-design skip), answering the issue author's ratified design with their own reproducer measured | ✅ public |
| Open data commitment | Contributed fix-pairs publish under MIT (consent screen pins it) | ✅ shipped |
| Installer downloads / repo stars / issues | GitHub releases API + repo pages | ⬜ collect (T3) |
| External users | — | ❌ none yet — pilot soft launch (T4) + no-submit gate (T10) |
| Named community in dialogue | — | ❌ the decisive gap; define + open dialogue (T4, W2 in the proposal) |

## 8. Pre-deadline checklist

- [x] Owner: openness amendment decided (T1) — **DECIDED YES 2026-10-31 (issue 192 decision 1; Proposal 025 G1); ticked 2026-11-05 (was stale vs the issue row)**
- [x] Owner: event posture confirmed (T2) — **CONFIRMED-BY-ACT 2026-11-03 (`aea8e270`: RELEASE_FEATURES flipped to the charitable posture, v0.4.0 tagged on main); delegated ratification recorded 2026-11-04; ticked 2026-11-05 (was stale vs the issue row)**
- [x] Evidence pack collected (T3) — landed 2026-10-31 by zed-m3-idle-ev190 → [`mozilla_evidence_pack.md`](mozilla_evidence_pack.md)
- [ ] Charitable-build sweep passes (T14) — **requires T14a + T14b landed by 2026-11-14; blocks pilot installers. State 2026-11-05: T14b LANDED 2026-10-31 (`a96a293f`); T14a-min LANDED (`e7cf72ea`), T14a's OPTIONAL half (riir-kat → optional dep) owner-gated; REGULAR bar LANDED (bench 123, `charitable_gate.sh`). Remaining: the T14a optional half + the strict-bar sweep run**
- [x] IPP form answer blocks prepared (§14) — **2026-11-05 (this doc): the live IPP form (mozillafoundation.tfaforms.net/f/humanscaleai) field-mapped; drafts are agent source material, owner re-voices before submission**
- [ ] Pilot soft launch published (T4), announcing the T11 public-repo-free policy
- [ ] Owner: entity / fiscal sponsor path known (T5) — **by 2026-11-07**
- [ ] Public-copy audit done (T6) — **probe 2026-11-05 (zed-m3 idle lane): the gen2 copy is NOT live — refine.riir.ai still serves the pre-pivot page (mining/KAT/pricing vocabulary, Pricing + Network-pulse sections, token-&-staking-risk links). The deploy stays owner-timed (vault leg) and MUST land before submission — a reviewer clicking the application's link sees the credit plane the application says is gone. Re-probe after the leg**
- [ ] Consult email v3 sent (~Nov 7) + info session attended (T7)
- [ ] T11 public-repo-free policy shipped (code + README) — **by 2026-11-17**
- [ ] No-submit gate check (T10) — **by 2026-11-17**
- [ ] Final prose owner-voiced, submitted before Nov 20 11:59 PM PST (T8)
- [-] T12/T13 pinning — Posture B only; off the submission path under Posture A unless the consult answer triggers the owner switch

## 9. Sustainability postures + license strategy (v3 2026-10-31, owner direction)

**POSTURE A — EVENT POSTURE (PRIMARY; CONTINGENT).** All KAT/credit functionality feature-gated OFF in the public artifact — **contingent on T14a + T14b landing (by 2026-11-14) and the T14 sweep passing**: public installers ship the charitable feature set (no `kat_service`/`kat_transport`) only then; internal dev builds unchanged (enabling `kat_service` restores the full flows). **Known blockers the sweep depends on:** the inert `riir-kat` accounting-types crate remains linked until T14a (non-optional dep, feature-free tier: mining_consent/service_config/soft_gate/units/service_urls compile ungated), and the mining/join/consent CLI surface is ungated until T14b (`--mine`/`--unmine`, the "how to earn KAT" usage lines, the join/config service-URL surface, `run_mine_intro`/`ensure_join_env_recorded`). Until both land and T14 passes: no charitable installer ships, and Posture A's sentences are targets, not claims — the T10 no-submit gate holds regardless. The application carries **zero credit/ledger/tokenomics claims** — nothing to disclose, nothing to pin, T13 does not apply to the funded scope. Sustainability = open source:

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
| **Nov 14** | **T14a + T14b code deadline** (optional `riir-kat` + CLI mining/join/consent gating), then T14 sweep |
| Nov 14–17 | Sweep passes → pilot installers; external-pilot soft launch announcing the public-repo-free policy (T11); capture download/issue evidence |
| **Nov 17** | **No-submit gate check (T10)** — decide submit / hold on the evidence |
| Nov 17–19 | Final application prose owner-voiced (only if the gate passes); owner review; submit |
| **Nov 20, 11:59 PM PST** | **Deadline** |

## 11. Consult email draft v2 (send by ~Nov 7; owner sends from owner address)

> **To:** incubator@mozillafoundation.org
> **Subject:** Human-Scale AI Incubator — eligibility questions before we apply
>
> Dear Incubator team,
>
> We are preparing an application for the Human-Scale AI Incubator (deadline Nov 20) and would be grateful for steerage on three eligibility points. (We have consolidated to a single proposal: a local-first code-fixing tool — a modelless core paired with an optional ≈0.4B-parameter encoder — that starts with Rust and expands to further languages, Python, JavaScript and Go included, by community demand, embedding our open decision-serving components for its escalation decisions.)
>
> 1. **Open-source sustainability + contribution points.** Our tool is free for open-source repositories; the repair corpus is MIT open data today, and the healing core and the fleet/corpus-production tooling will be MIT — the core source opens during months 1–3 of the grant term. We sustain it as a public good: contributors earn non-transferable, expiring reputation points (no payment, no token, no speculation — the point balance only meters hosted lanes as fair use); a public leaderboard ranks joined contributors; revenue later comes from enterprise corpus licensing, hosted/support services, and additional language domains. One plain disclosure: we are removing the old credit/token lane from the public builds entirely before any public release, and it is entirely outside the funded scope. Is this sustainability model acceptable within a funded project?
> 2. **Charitable purpose as a self-funded team.** Neither project has revenue or clients yet; the team is self-funded on consumer hardware. Is applying via a fiscal sponsor acceptable, and what documentation do you require?
> 3. **"Work in the open."** The tool currently distributes via public installers while its source is private. We are prepared to open-source the core during the grant term (months 1–3) under a permissive licence and to publish our consent-gated repair-rule corpus as open data. Is a committed, dated open-sourcing plan acceptable evidence of working in the open, or is open source required at application time?
>
> We plan to attend the November 12 info session. Thank you for your time — and for funding this alternative trajectory.
>
> — The cargo refine & riir.ai team · grants@riir.ai
>
> *(From/Reply-To: grants@riir.ai — the domain mail is catch-all (Cloudflare Email Routing), so this address receives every reply; agents may send from it per owner decision 2026-11-01.)*

## 12. Links

- Overview: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/>
- Call for proposals: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/call-for-proposals/>
- Anchor projects: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/cohort/>
- FAQs: <https://www.mozillafoundation.org/en/what-we-do/grantmaking/incubator/human-scale-ai/call-for-proposals/faqs/>

## 13. Canonical copy block (Plan 324 T1.4 — written ONCE, mirrored everywhere)

**The one source for every front** (reflex.riir.ai, refine.riir.ai, rethink.riir.ai, riir.ai).
Mirrors copy from here verbatim; never re-word per site. Laws that bind every sentence:

- **Vocabulary law (issue 192 GAP 10):** "contribute / contribution / contributor / contribution
  points" ONLY. Never "mining / mine / miner" anywhere public.
- **Roadmap law (Plan 324 G-B):** the points plane is described as **roadmap, never live** —
  no leaderboard/beta/join copy as a shipped feature until W0 ships.
- **Truth law (Plan 324):** every claim on-page must be true on the date it ships. Numbers come
  from the evidence pack (`mozilla_evidence_pack.md`) with provenance, never hand-typed.
- **Scope law (issue 192 GAP 9):** the riir family is open; the game business is a separate
  brand — never mentioned, never in scope, on any front.

### 13.1 Positioning one-liners (family bar, T1.3's source)

| Front | One-liner |
|---|---|
| riir.ai | The riir.ai family — local-first, modelless AI tools that run on the machine you own. |
| reflex | The modelless decision engine — zero-parameter, sub-millisecond decisions, and public arenas where anyone can watch it compete. |
| refine | The local-first rulewright for Rust — fixes compiler and lint findings in-loop, offline, free for open-source repositories. |
| rethink | The open research lane — generic decision benchmarks, beaten in the open, one lane at a time. |

### 13.2 The three-principles answers (each verifiable on-page in one click)

**Small — adaptive by design, capable at every size.** The stack sizes itself to its deployment.
The always-on floor is the modelless engine: zero weights, offline, sub-millisecond, fully
private — it answers the common case on any machine. Where a decision needs more, a
≈0.4B-parameter encoder arm escalates, and the rungs are measured, not aspirational:

- **Modelless floor** — zero weights, always on (the privacy story, not the size headline).
- **One int8 (Q8) checkpoint, deployed** — 484 MB measured end to end (the published
  reflex.riir.ai breakdown: a 7.9 MB GPU-host serve binary + 20.8 MB dataset seats +
  the 447.7 MB Q8 checkpoint + three ≈8 MB locked per-suite heads).
- **Board subset → full board** — load only the suites you serve; the full served board
  (english + typed) is ≈0.9 GB at Q8 (2 × 447.7 MB; the ≈1.7 GB F16 posture is the reference
  build; Q8 is the adopted serving tier at 0.956–1.000 retention).
- **Hosted think leg** — the server-adaptive seam: a light box keeps only the modelless
  material local and escalates the encoder to a hosted machine — ≈0 GB of local encoder.

The whole ladder tops out ≈17× below Mozilla's own small-model calibration (7B), parameters
to parameters — and it never shrinks below the quality floor: lighter quantization is a
recorded negative and is never offered.

**Intentional — consent-first, inspectable, yours.** Local healing never touches the network.
Sharing data is opt-in, one-time consented, and secret-blind by policy: the redaction layer
never lets credentials or secrets leave the machine, and the default is anonymous. You can
`leave` at any time — joined or not, the tool is identical locally. The repair corpus is MIT
open data forever; the bench history is public; every rule in the corpus is reviewed, not
scraped. And the mix is honest about itself: the modelless floor does not win every suite, so
the ladder escalates instead of pretending — the published bench tables show which arm serves
each suite and why.

**Tailored — adaptive, flexible, self-evolving with its community.** The rulewright is
domain-pluggable by architecture: the alpha targets Rust — the most requested, hardest-linted
community — and the same framework already ships non-Rust domains (Dockerfiles, WGSL shaders),
with Python, JavaScript/TypeScript, Go and more on the roadmap, added when the community asks.
The dialogue is structural, not consultative: every contributed repair joins a reviewed,
MIT-licensed corpus; the public bench history scores every batch (200+ runs published); and
healing improves by corpus, never by retraining — the tool literally gets better at what its
community fixes most. Free for open-source repositories, one command to install, zero ongoing
cost: built for developers without cloud-AI budgets, anywhere.

### 13.3 The six-criteria one-liners

| Criterion | On-page answer |
|---|---|
| Human-Scale AI expression | The mix is the intelligence: a zero-weight modelless floor answers when it can; a ≈0.4B encoder escalates only where it can't — ≈17× below the funder's own small-model bar, sized from a zero-weight floor to a 484 MB full encoder-arm deployment (published measurement) to a hosted think leg. |
| Technical viability | Shipped and used: 13 releases, 200+ installer downloads, 5 OS targets, 200+ scored benchmark runs at a 96%+ heal rate, deployed as the default dev-loop tool across a 32-repo production workspace (evidence pack). |
| Problem-community clarity | Software teams lose hours every week to compiler, linter and review friction; the rulewright fixes the mechanical share of it in-loop, in seconds, for free — alpha targets Rust, the domain set expands by community demand. |
| Product-community fit | One command, offline, no account; free for open-source repositories forever; pluggable domains (Rust alpha, Docker and shader domains shipped, Python/JS/Go roadmap); the people it serves build it — their contributions become the corpus. |
| Impact and values | Community corpus MIT-licensed and live today, with a committed, dated open-core plan (the core source opens during the grant term); consent-first, no token, no payment, no speculation; works without cloud AI; keeps small hardware useful. |
| Sustainability potential | Free for OSS forever; contribution points — non-transferable, expiring reputation, never payment — meter hosted lanes as fair use; later revenue from enterprise corpus licensing, hosted/support services, additional language domains (Python/Go planned), and the hosted decision-serving API (Rethink opens as funded work — the moat is the service, not code restriction; owner amendment 2026-11-11). |

### 13.4 The corpus / points / governance story (the sustainability paragraph, one form)

> cargo refine's repair corpus is MIT open data — live today, forever. The core itself ships
> as checksum-verified binaries today, with a committed, dated plan to open the source during
> the grant term (months 1–3). Anyone can
> contribute repairs; contributions earn contribution points — non-transferable, expiring
> reputation that meters hosted lanes as fair use. There is no payment and no token: points can
> never be bought, sold, or traded. The public leaderboard (roadmap) ranks contributors by
> reputation; the corpus they build stays open for everyone, including competitors. Governance
> is the licence plus the public record: reviewed corpus, published bench history, and a
> dated open-core commitment.
> New domains — Python, JavaScript, Go and beyond — are roadmap, added when the community
> asks; every contribution is reviewed, bench-scored in public, and improves healing for
> everyone without retraining anything.

**(Roadmap framing when the points plane is named at all):** "coming next: join the
contribution network, earn contribution points, appear on the public leaderboard" — future
sense only, no sign-up UI, no beta list until W0 ships.

### 13.5 What is deliberately absent from every front

- No pricing, no packs, no tiers, no store, no checkout — the storefront copy is gone.
- No KAT, no wallet, no mainnet — the credit plane is out of the story entirely.
- No game business, no seal, no separate-brand content.
- No mining vocabulary ("mine/mining/miner" — see the vocabulary law above).
- No live-feature claims for the points plane (roadmap law).
- No unqualified "as small as you like" size claims — the ladder names its measured rungs;
  lighter quantization below Q8 is a recorded quality negative and is never offered.
- No user counts, testimonials, or metrics that the evidence pack cannot pin.

## 14. IPP form answer blocks (agent source material — NEVER paste-ready; form mapped live 2026-11-05)

The live form (`mozillafoundation.tfaforms.net/f/humanscaleai`, fetched 2026-11-05): save-and-resume
supported; **submitted forms are final**; an eligibility hard-stop fires on the applicant country
before the project section; `Countries/Regions served` is asked right after the fiscal-sponsor block.
Field order: Applicant Info & Eligibility → Acknowledgements → **Project Overview (the selects)** →
**Project Details (7 long answers)** → **Impact and Openness (5 long answers)** → links + video →
AI-disclosure → sharing/newsletter/terms.

**Compliance (binds every block below):** Mozilla's AI policy allows prep assistance but submitted
content must be the applicant's own work and voice — the owner re-voices each answer before entry;
this section is raw material, in one voice, for that pass. Vocabulary law (§13) applies: contribute /
contribution points only; points are roadmap; every number traces to the evidence pack and is
re-verified on submission day.

### 14.1 Select-field recommendations

| Field | Recommended | Note |
|---|---|---|
| Applicant Type | **Fiscal sponsorship** if a sponsor is signed by submission; else **Individual** | T5 is deferred to the shortlist stage — an individual may lawfully receive 501(c)(3) funds; owner decision, MUST be settled before the country gate fires |
| Cohort availability ack | Yes | 12 months from April 2027 |
| Human-Scale AI principles | **Small + Intentional + Tailored** (all three) | §13.2 maps each to a one-click on-page answer |
| Project Stage | **Deployed with early users (less than 100 users)** IF ≥1 external user can be evidenced by submission day; else **Working prototype** | The T10 gate (Nov 17) governs this exact call — the stage answer must match the evidence ON submission day, not the aspiration |
| Early signals | **Users**; add **Contributors** only if an external contribution lands by submission | The 32-repo deployment is one team — honest reading is Users-only today; the sccache PR is us contributing upstream, not external contribution to us |
| Approx users | **Less than 100** | Honest; download counts ≠ users |
| Primary outputs | **Technology platform or tool + Dataset**; optionally Educational resources | The MIT corpus is the dataset claim |
| Accessibility | **"We have considered it, but it is not yet a focus"** | CLI-first product; do not overclaim — all answers are considered equally |
| Frontier-model API dependence | **No** | The hot path calls no model; no GPT/Claude/Gemini/Llama family anywhere core |
| Largest parameter count | **10 billion or fewer** | The ≈0.4B escalation encoder is the largest model named in the application; "does not use AI models directly" is defensible for the healer core alone but reads inconsistent once the decision arm is named — keep the application text and this select coherent |
| Charitable purpose | **Yes** | Public benefit described below |
| Work in the open | **Open sourcing code or other assets · Openly sharing datasets · Publishing educational materials · Public workshops or presentations** | Research papers: unchecked (not on the plan — never claim) |
| How did you hear | Owner picks (Foundation Website / Web Search / etc.) | Explicitly not review criteria |
| Share with other funders | **Yes** | Optional, no effect |
| Countries/Regions served | Owner picks | Global software community; the team's own location is the org-address country — that field hits the eligibility gate, not this one |

### 14.2 Short-form answers

**Proposed Project Title**
> cargo refine — the modelless code healer: free, offline fixing for every Rust developer

**Project Summary**
> cargo refine is a local-first developer tool that fixes compiler warnings, lint findings, performance traps, GPU-kernel and shader hot spots, and security anti-patterns in Rust code in-loop — on the developer's own machine, offline, with no AI
> service call. Its core is modelless: a reviewed, MIT-licensed corpus of repair patterns plus
> deterministic drafters and a compile gate that keeps only fixes that still build. Where a case
> needs more, a ≈0.4B-parameter escalation arm answers — roughly 17× smaller than the small-model
> bar this call itself sets. It is free forever for open-source repositories, installable in one
> command on five OS targets and crates.io, and it improves by community contribution, never by retraining.

**Public benefit (beyond the organization)**
> Every Rust developer without a cloud-AI budget — students, FOSS maintainers, regional and
> resource-constrained dev communities — gets lint/perf/security fixing that today requires paid cloud
> copilots or local 7B+ models they cannot run. The benefit is durable commons, not a free trial:
> the repair corpus is MIT open data forever, the core source opens under MIT/Apache-2.0 in months
> 1–3 of the term, and free-for-open-source-repositories is shipped product policy, not a promise —
> with authoring tools that make the framework language-general: Rust proves the pattern; Python,
> Go, and JavaScript are the named next corpora.

### 14.3 Project Details — the seven long answers

**1. What technology are you building and how does it address this cohort's theme?**
> cargo refine is a command-line tool that fixes Rust code the way a careful reviewer would: it
> reads the compiler's and Clippy's findings, proposes the repair, re-builds the project, and keeps
> the fix only if the code still compiles. The intelligence is a mix, not one model: a zero-weight
> modelless engine — a reviewed corpus of repair patterns, deterministic drafters, and a
> similarity-ranked retrieval — answers the common case instantly and privately on any machine;
> where a case genuinely needs more, a small (≈0.4B-parameter) escalation arm is consulted. This
> addresses the cohort's theme directly: it is Small (the hot path uses no model at all, and the
> largest component is ~17× below the call's own 7B calibration), Intentional (local by default,
> opt-in sharing, secret-blind redaction, a `leave` command, MIT open data), and Tailored (built
> with and for developers excluded from cloud-AI economics — free for open-source repositories,
> one-command install, works offline on a laptop).

**2. Small — models, hardware, infrastructure, and why they fit**
> The healer ships as a single binary with no model files on the hot path: its knowledge is a
> curated corpus (1,642 pattern classes in 13.4 MB of rule text across six domains — measured on
> the shipped buckets, 2026-11-10) plus the developer's own verified-fix history,
> retrieved and applied deterministically. The escalation arm is one ≈0.4B encoder; its deployed
> serving posture is measured at 484 MB end to end (Q8 checkpoint + seats), and the heavier
> quantizations below that are a recorded quality negative we refuse to offer. The full decision
> ladder tops out ~17× below the 7B small-model calibration in this call. Hardware: any laptop —
> no GPU, no cloud dependency; the tool works with the network off. This is the right fit because
> code repair is pattern-classification over a bounded, reviewed vocabulary — scaling laws for
> generative models do not apply to a corpus a community curates, and every removed megabyte is a
> developer who can actually run it.

**3. Intentional — data understanding, control, community agency**
> Default is local and silent: healing runs offline and uploads nothing — verifiable by running
> the tool with the network off. Sharing is opt-in, consented once, reversible (`leave` restores
> the logged-out tool with no loss), and secret-blind by construction: every shared snippet passes
> a secret scanner first, and any finding drops the row — the redaction never sees credentials as
> shareable data. What sharing produces is a public MIT-licensed repair corpus and a public,
> published benchmark history (200+ scored runs) — the community can audit exactly what the tool
> learned and how well it works. Governance on the roadmap is contribution-based: contributors
> earn contribution points — non-transferable, expiring reputation that meters hosted lanes as
> fair use, never payment, never a token — and the corpus they build stays open for everyone,
> including competitors. Governance is the licence plus the public record: reviewed corpus,
> published benches, dated open-core commitment.

**4. Tailored — how people, problem, context shaped the technology**
> The alpha community is Rust developers — the most lint-saturated, tooling-demanding language
> community — chosen because their pain is measurable (compiler + Clippy diagnostics) and their
> tooling culture demands correctness: hence the compile gate, which discards any fix that does
> not build, and the divergence classes the tool openly refuses to touch. The context is
> resource constraint: no cloud budget, sometimes no reliable network — hence offline-first, a
> single static binary, five OS targets, and free-for-open-source-repositories as shipped policy.
> The dialogue is structural: every contributed repair joins the reviewed MIT corpus, the public
> bench history scores every change, and new domains (Dockerfiles, GPU kernels, and WGSL shaders shipped today;
> Python, JavaScript, Go on the roadmap) are added when the community asks — healing improves by
> what the community contributes, never by retraining.

**5. Traction — early product-community fit evidence**
> Working, shipped, measured: 13 public releases with checksum-verified installers and 200+
> downloads across 5 OS targets (macOS Apple Silicon/Intel, Linux x86_64/aarch64 static, Windows), now on crates.io too;
> a public benchmark history of 200+ scored runs at a 96%+ heal rate; deployed daily as the
> default dev-loop tool across a 32-repo production workspace since August 2026 — the tool fixes
> real code every working day, not in demos. The corpus grows about a hundred rules a week (562
> when tracking began in late August; 1,602 across seven domains today), every source pinned,
> deltas published in each release's notes. First external upstream engagement is MERGED:
> a fix PR to Mozilla's own sccache (issue #2975) landed after six maintainer review rounds
> (merge commit `907230be`, November 2026), and a second sccache PR — backend read backoff on a
> maintainer-engaged issue whose author measured our prototype with their own reproducer — is open
> with its full CI matrix green. The pilot cohort (30–50 Rust developers from student groups, FOSS maintenance
> and regional communities) is being recruited now — the application window is when that evidence
> lands, and we will report it openly either way.

**6. Challenges and biggest blockers right now**
> The biggest blocker is trust: the tool works and is measured, but no external community uses it
> yet, and trust in a repair tool must be earned one merge at a time. Four, honestly, after that:
> (a) Adoption signal: converting installer downloads into an active external user and contributor
> base — the pilot cohort is the instrument, and it is the same evidence this call weights (early
> product-community fit). (a2) The technique itself is new and sits between rules-based tools and
> large language models — distilled repair rules, a small fixed drafting step, deterministic
> verification gates, no generative model anywhere — so the idea draws skepticism from both camps;
> explaining that middle ground is real work, and the compile-gated, published results are the
> answer. (b) Receiving-entity paperwork: a self-funded team needs a fiscal-sponsor path to
> lawfully receive US 501(c)(3) grant funds; the research is done, the decision lands with the
> shortlist stage. (c) Executing the dated open-core extraction without slowing the product:
> opening the core in months 1–3 means restructuring a private workspace into a public repository
> with public CI and a contribution guide — real engineering capacity the grant directly funds.
> And this call is uniquely placed on the trust blocker: a Mozilla endorsement plus the cohort
> platform is how an unknown tool meets its first real community — the exact compounding the grant
> exists to buy.

**7. Team**
> A self-funded team of experienced Rust systems engineers who build and operate the entire stack
> themselves — the healing engine and its rule corpus, the benchmark infrastructure, the serving
> tier, and the public sites — with production Rust across a 32-repository workspace where the
> tool runs as the default dev-loop instrument every day. The engine is built on katgpt-rs, the
> team's MIT-licensed public research substrate distilling recent AI papers into proven Rust
> primitives (in-tree formal proofs, production game use). The dogfooding is the team's most
> relevant experience: the tool is not a research prototype; it is infrastructure the team
> depends on. [Owner: add names, roles, prior work links.]

### 14.4 Impact and Openness — the five long answers

**REFRESHED 2026-11-11 — the finalized, count-verified versions (126/115/125/116/170 words) live in [`../../.proposals/025_mozilla_human_scale_ai_incubator.md`](../../.proposals/025_mozilla_human_scale_ai_incubator.md) §"Impact and Openness" (after the Team answer) — those are canonical now; the older drafts below are the pre-refresh source.** New facts woven in: crates.io live (0.5.1), the corpus growth rate (~100 rules/week), the measured corpus size (1,642 pattern classes / 13.4 MB), upstream merge acceptances as a public counter, and the language vision (Python/Go/JS) in the 36-month row.

**8. Success in 12 and 36 months + measurement** (superseded — see the proposal)
> 12 months: the modelless core open-sourced under MIT/Apache-2.0 with public CI and published
> crates (downloads as the public counter); the pilot cohort complete with an openly published
> report (heal rates, retention, offline usage, participant quotes); one community-built language
> domain landed; 1–2 external corpus contributors onboarded. 36 months: a multi-language repair
> commons governed publicly (reviewed corpus, published benches, contribution governance live);
> the tool a default in resource-constrained development environments; sustainability running on
> enterprise corpus licensing and hosted/support services so the commons never needs charity to
> stay open. Measurement is public counters only: crates/downloads, the pilot report's own
> numbers, contributor count, and the published benchmark history — nothing self-attested.

**9. Sustainability needs + barriers**
> Needs: a durable legal home (fiscal sponsor now, foundation path later); the pilot-to-community
> conversion that turns downloads into contributors; and revenue lanes that do not tax the
> commons — enterprise licensing of our own GPU-mined corpora and hosted/support services, while
> community-contributed data stays MIT forever and the core stays open. Barriers: capacity (the
> team is self-funded — the grant is the runway that converts a working tool into an open
> program); the corpus-split boundary (community-contributed MIT vs own-mined proprietary) must
> stay trustworthy and auditable; and keeping the contribution-points reputation system strictly
> non-speculative while still genuinely motivating.

**10. What the $300,000 grant unlocks**
> It converts a working, self-funded prototype into an open, governed, community-owned program in
> the first year: ~60% funds 2–3 contributors (the open-core extraction, the pilot, the second
> language domain); ~15% funds the pilot itself (service capacity and expiring pilot vouchers —
> pilot users never pay and never need to contribute); ~15% infrastructure and audit (public CI,
> security review of the redaction and consent planes); ~10% contingency. Concretely: months 1–3
> the core is public; by month 6 the pilot cohort's report is published; by month 12 a language
> domain built by someone outside the team ships. Without it the same roadmap runs — slower,
> and private for longer; the grant buys openness sooner, which is the point.

**11. Working in the open — code, learnings, data**
> Code: the modelless core (corpus format, retrieval, drafters, compile gate, self-evolve loop)
> and the fleet/corpus-production tooling open under MIT/Apache-2.0 — one license story — in
> months 1–3, with public CI, docs and a contribution guide; crates published to crates.io.
> Data: the consented repair corpus is MIT open data today and stays MIT forever, with a published
> corpus-authoring guide so other communities can distill and govern corpora for their own
> languages and codebases. Learnings: the benchmark history is public (200+ scored runs, appended
> per change — negative results included); the pilot report publishes openly; we present at
> Mozilla Festival. Educational: agent-facing and human docs ship with the installers today.

**12. Novel approaches the work introduces**
> (a) Modelless-by-architecture repair: a reviewed corpus plus deterministic drafters plus a
> compile gate replaces the field's assumption that code fixing needs an LLM on the hot path —
> our October 2026 landscape search found no shipped modelless healer; local coding tools ride
> 7B–14B models. (b) Improvement by corpus, not retraining: the tool's capability grows from
> community-contributed, reviewed pairs with a fixed evaluator — bounded self-refinement that is
> measured and published, convergent and evaluable, with negative results kept. (c) A measured
> size ladder instead of one scaled-down model: zero-weight floor → ≈0.4B escalation arm at a
> measured 484 MB deployed → a hosted think leg — each rung admitted only when it beats the one
> below on the public bench. (d) Consent-first data plane as a product primitive: secret-blind
> redaction and one-command reversible sharing built into the tool, not a policy page.

### 14.5 Links + video (owner acts)

- **Repository/demo/website:** `refine.riir.ai` + `github.com/riir-ai/cargo-refine` (releases) + `crates.io/crates/cargo-refine` (0.5.1 live 2026-11-11) +
  the public bench tables. ⛔ **Gate:** the gen2 de-tokenized deploy MUST be live before submission
  (probe 2026-11-05: still the pre-pivot copy — see §8 T6 row and issue 190 T6).
- **Two-minute video (strongly encouraged):** owner act. Suggested script shape: 30s the problem
  (lint/perf friction), 40s live demo (dry run → --fix → compile gate → network off), 30s the
  corpus/community story, 20s the team and why it matters. No polish needed; reviewers want the
  technology working and the team's voice.

### 14.6 AI-disclosure + honest-tension register

**AI disclosure (the form asks; answer honestly):** recommended: "Yes — AI tools were used for
research preparation and drafting assistance; every submitted answer was written and voiced by the
applicant." This is exactly Mozilla's allowed use; the owner's re-voicing pass (T8) is what makes
it true — do not submit before that pass.

**Tension register (each must be TRUE on submission day):**

1. **Stage + signals ↔ the T10 gate (Nov 17).** The form's stage answer must match the evidence
   that exists that day. Zero pilot sign-ups + no external-user evidence by Nov 17 = the playbook's
   own no-submit gate holds; do not submit a "deployed with early users" claim without a user.
2. **The live site.** refine.riir.ai serves the pre-pivot copy until the gen2 deploy lands
   (owner-timed). A reviewer clicking the application's link must NOT see the credit plane the
   application says is gone.
3. **Points plane = roadmap.** The Intentional/governance answers must not imply the leaderboard
   or contribution points exist today — §13.4's roadmap framing only.
4. **Numbers.** Every figure in these drafts traces to the evidence pack; re-verify download and
   release counts on submission day (they grow — use the fresh ones).
5. **No fabrication.** The pilot is recruiting, not run; partner names do not exist. The sccache
   record is one MERGED PR (#2977, `907230be`) plus one OPEN green PR (#3025) — say exactly that;
   #3025 is an open PR, not an accept, and not a user. Coverage-bot reports (Codecov) are
   telemetry, never cited as community engagement. Say exactly that.
