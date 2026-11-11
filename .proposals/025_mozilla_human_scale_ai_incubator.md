# Proposal 025 — Mozilla Human-Scale AI Incubator application: cargo refine, the modelless code healer

Status: **draft v4** — owner pivot 2026-10-31 (issue 192): **Refine open-src DECIDED (G1 resolved)**; posture becomes **public-goods points** (TUNA contribution points replace KAT/payment — no purchase, no token value; drain only as fair-use balance on hosted lanes); **Rethink = PoC research sibling** (sites drop pricing; commercial only post-trust); Refine = closed-beta for the contribution network + corpus stacking + Python/Go domains planned; **no more mainnet for refine** (proposal 023 DEFERRED-FROZEN, issue 175 removed → HISTORY). Websites: [Plan 324](../.plans/324_mozilla_web_alignment.md). Still gated on the consult + the T14/T14a/T14b artifact gates; nothing submits before 2026-11-12 · 2026-11-06: draft answers filed (§Draft application answers) · **2026-11-07: `TUNA_WEIGHTS_V1` armed** (accrual weights + weekly epoch, §points plane; node-misbehavior ladder in Plan 335 P2.0(c)) · **2026-11-07: `TUNA_ANTISPAM_V1` armed** (rate limit + identity floor + refundable 1 TUNA deposit + node bond; `claim` re-enabled as the 100 TUNA starter allowance under the two-source rule)
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned (grant submitter = owner)
Deadline: **2026-11-20 11:59 PM PST** · Info session: **2026-11-12 17:00 UTC** · Consult email: draft in [`../.docs/01_orientation/mozilla_grant_readiness.md`](../.docs/01_orientation/mozilla_grant_readiness.md) (v2 — credit-plane question added)

## TL;DR

We propose to apply to Mozilla Foundation's Human-Scale AI Incubator with **cargo refine**: a local-first, modelless code healer for Rust. Its repair "model" is not a neural network but a distilled, inspectable corpus of repair rules (1,602 rules, 7 domains) searched by a zero-allocation ternary SIMD drafter — **zero parameters in the hot path, below Mozilla's own 7B "small" reference point** — running offline on hardware people already own, with the developer's fix memory stored on their own disk and their code never leaving the machine without explicit, redacted, revocable consent. Two design components complete the picture: **(1) the decision layer** — the healer's escalation decisions (attempt local fix / escalate / abstain) run on the open decision-serving stack's operator-configured grammar and audited decision envelope (§3); and **(2) an open-source sustainability model** (§4): everything is MIT — the modelless core, the fleet/corpus-production tooling, and the community-contributed corpus — while revenue comes from enterprise licensing of our own GPU-produced corpus and hosted/support services, and **no credit/ledger/payment functionality is present in the public build** (Posture A). The grant funds: **(1)** opening the healer's core, **(2)** a pilot with Rust developers who cannot pay cloud-AI subscriptions or assume reliable connectivity, and **(3)** publishing the consented repair-rule corpus + corpus-authoring tools so communities can build and govern their own. **This proposal is a working draft, not final application prose** (Mozilla's AI policy requires owner-voiced submissions — see §Compliance).

## Why this project fits Human-Scale AI

### Small — the zero-parameter hot path

- The L1 drafter is **modelless**: the corpus *is* the model. Distilled rules carry latent vectors; a ternary SIMD matvec scores candidate fixes with sub-millisecond, allocation-free CPU work. No LLM inference, no GPU, no frontier model, no account, no network at heal time.
- One static binary; installs from public installers (GitHub releases, Homebrew tap, Scoop bucket, crates.io); works fully offline — including credit accounting, which accumulates locally and settles opportunistically when connectivity returns (offline-first by design, for intermittent-connectivity regions).
- Where any larger model exists in the product (hosted escalation tier), it is **opt-in and out of the proposed charitable scope** — the funded build never requires it.
- Contrast with the "not looking for" list: no frontier-model dependence (their explicit exclusion), not a generic chatbot, working and deployed today across a 32-repo production workspace.

### Intentional — the developer owns their repair memory

- **Consent-first telemetry:** the contribution plane never transmits unless the developer opts in (`Unset` never pushes; `OptedOut` never transmits). The consent screen will state the **MIT licence for published pairs** (funded build — the `--mine` surface that carried it was removed `5e61bb59`, issue 192 GAP 10; port to the `join` consent copy, see §Draft application answers fact-pins) — contributed repair knowledge becomes open data by design.
- **Local-first memory:** fix trajectories live in `.refine/` on the developer's disk — inspectable, exportable, purgeable (funded build; the `--purge` flag retired 2026-11-06, `5e61bb59`, issue 192 GAP 10); self-evolution has a documented opt-out.
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

**Posture A — EVENT ARTIFACT (ships first; CONTINGENT).** All KAT/credit/payment functionality is **feature-gated OFF in the public artifact** — installers ship the charitable feature set (no `kat_service`/`kat_transport`) **only after T14a + T14b land and the T14 sweep passes** (T14b landed `a96a293f`; T14a min half `e7cf72ea`; strict half owner-gated). Internal dev builds unchanged (builds enabling `kat_service` keep the full flows). The application makes **no payment/token claims** — and under the v4 pivot the points plane (below) ships as grant work, disclosed as design. Until the sweep passes, no charitable installer ships and Posture A's sentences are targets, not claims.

**Posture C — PUBLIC-GOODS POINTS (v4 end state; ships as W0/Q1 grant work — issue 192).** Contribution earns **TUNA points**: non-transferable, expiring reputation — never purchase, never token value, never speculation; **the KAT requirement is ripped out**. Points draw down on hosted/expert lanes as a fair-use balance (never blocking local `fix`, never negative-locked). Leaderboard rank requires GitHub-joined identity (anti-Sybil — anonymous users get local stats only). Ledger v1 = points service + Merkle epoch anchoring (`anchor_root` substrate); **chaind deferred to seal-***; full chain only if anchoring proves insufficient. **Privacy law (binding):** a locally generated random client UUID (user-resettable) identifies installs; unsalted hardware/hostname hashing is REFUSED (reversible fingerprinting — issue 192 GAP 1); any anti-abuse hardware signal is server-salted and disclosed in plain words. The new CLI surface (`join`/`leave`/`rank`/`info`/`corpus`) ships with the plane (issue 192's table); the T14 sweep re-scopes to payment/token terms when it lands (GAP 2).

The application makes **no payment/token claims** — the points model is disclosed as reputation design (the Intentional principle made concrete), never as value. Payee/staking/transferability questions (T13) stay dead: points are non-transferable and expiring by construction. Sustainability is the **open-source** story below, with Rethink as the commercial research sibling post-trust (pricing removed from all sites — [Plan 324](../.plans/324_mozilla_web_alignment.md)).

### The points plane in full (Posture C mechanics — the durable record of the v4 model)

*This section is the canonical description of how the points plane works. Issue 192 is the
task tracker and will be removed on completion; this proposal is where the model lives.*

**Units.** One user-facing unit: **TUNA contribution points** — non-transferable, expiring,
no cash value, never purchasable. The substrate already exists and runs: the riir-dapps TUNA
plane ships fast operator mint (no quorum wait), expiry, and non-transferability — exactly
points semantics (riir-dapps [Proposal 006](../../riir-dapps/.proposals/006_tuna_trial_credit_and_token_conditions.md)).
**KAT is ripped out of the user story** — "no purchase, no KAT for users" — and the KAT lanes
are FROZEN, not deleted: proposal 023 DEFERRED-FROZEN, KAT billing/orientation docs get a
"frozen lane" banner on next touch, no Refine chain consumer, chaind promotion continues for
seal-* only. The points tier mints at the TUNA operator batch's fast shape — no 2/3 quorum wait.

**The user flow (the new CLI surface — verb-first; reconciled against the
shipped binary 2026-11-03, owner v5):**

| Verb | Behavior |
|---|---|
| `cargo refine` | dry run — review what would be fixed, zero edits; no account, no network |
| `cargo refine fix` | free, local, no join needed; compile-gated (a fix that breaks the build reverts); first-run consent ONCE for the secret-blind send (spans/results, `allow_redacted: false`); asks join-or-anonymous; fully-offline mode (`RIIR_REFINE_OFFLINE=1`) documented for air-gapped users. The `--fix` flag stays as the alias |
| `cargo refine scan` | the secret-blind scan — report secret-shaped content (kind + path:line:col + blinded context) in code, comments and string literals, over the canonical gitleaks-consolidated scanner; `scan --blind <file>` renders safe-to-share redacted text. Report-only, never auto-fixed, values never shown. (`--secret-scan` stays as the flag alias; owner pick 2026-11-03 — `scan`, not `lint`, because `lint` reads as the clippy-fixing domain and the bare dry run already owns the fix-scan meaning) |
| `cargo refine join` | contribution policy accept + GitHub login — the EXISTING device-flow OAuth app, minimal `read:user` scope (proves identity, never write; the client id is public by design and compiles as the default in the public posture) |
| `cargo refine left` / `leave` | leave contribution: log off, zero data sent (`leave` ships; `left` kept as hidden alias) |
| `cargo refine rank` | top-ten leaderboard + own rank — **rank requires GitHub-joined identity** (anti-Sybil: a rotating derived ID could farm the board; anonymous users get local-only stats) |
| `cargo refine info` | top fixed repos by domain — aggregated, redaction-policy-scoped, counts + domains only, never repo content |
| `cargo refine corpus` | corpus info + the freshness offer (points-free — corpus freshness is a public good; never a charge line in any posture) |
| `cargo refine upgrade` | verified binary self-update — SHA256SUMS-checked fetch + backup-swap, refuses dev builds |
| `cargo refine version` | version + the compiled feature set + the build stamp (alias of `--version`) |
| `cargo refine --stats [on\|off]` | share rule-and-count stats (opt-in, never your code or paths) — a THIRD consent state, separate from contribution |
| `--suggest` · `--fix-compile` · `--domain` · `--json` · `--help-all` | power + maintainer flags (the full inventory in `--help-all`) — never required for the user flow |

**Compiled OUT of the public build (the frozen KAT lane):** `login` · `account`
(keys/custody/migrate/totp/history) · `--mine`/`--unmine` (replaced by
`join`/`leave`) · `sync` · `claim` (RE-ENABLED in Posture C as the 100 TUNA starter allowance — `TUNA_ANTISPAM_V1`; the KAT-era claim stays out) · `stake`/`unstake` · `vote` · `pay` ·
`topup`/`buy` · `kat`/`tuna` · `get`/`send tuna` · `faucet` · `wetrun` ·
`replay` · `corpus-pr` · `config adopt-state` · `--refill` · `update` (the
corpus-lease refresh — the public `corpus` verb is its points-free face).
Refine is MAINNET-ONLY: there is no devnet lane and nobody — including us —
uses one (owner directive 2026-11-03). The mainnet/devnet WORD belongs to
chain (seal-*) and Rethink, not Refine. The chaind option stays open behind
the grant: the USDC-pour rail is already tested (the pay lane), and if
Mozilla passes, the frozen KAT/chaind plan (Proposal 023 DEFERRED-FROZEN)
is the re-promotion path.

**The three binding laws (verdict r1, 2026-10-31):**

1. **Drain.** Local `fix` is ALWAYS free and never blocked; points never go negative-locked.
   v1 public copy leaves the drain OUT entirely; the numeric fair-use policy (drain rate,
   balance cap, refill paths) is defined in the points-plane proposal (T16.2) BEFORE any
   drain ships — until then the only public sentence is "points meter hosted lanes as fair use".
2. **Expiry = rank-decay, never erasure.** The *balance* (rank-eligible weight) decays for
   leaderboard ranking only; contribution history and corpus contributions are PERMANENT
   (the corpus is MIT data forever). Expiry-that-erases was refused as an engagement trap.
3. **Vocabulary.** "mining / mine / miner" is banned from all public copy and user-facing
   docs — contribute / contribution / contributor / contribution points only (internal code
   identifiers rename only when touched; the T14 sweep's detector patterns keep the old
   words by design — they exist to catch the retired surface).

**Accrual weights — `TUNA_WEIGHTS_V1` (owner-armed 2026-11-07; the weight table is an owner lever, never an agent one).** Accrual only — the drain stays out of v1 (law 1). Points are awarded for CHECKED OUTCOMES, never for compute, uptime, or attempts:

| Contribution (GitHub-joined identity) | Points | Proof read by the operator batch |
|---|---|---|
| Contributed fix record accepted into corpus intake (compile-gated, secret-scanned, OSS-class target) | 1 | the fix's verify receipt + the intake row |
| Reported miss that a landed rule later heals | 2, at heal time | the content-addressed miss row + the healing rule's commit |
| Fix merged upstream into a public repo | 5, at most ONE per target repo per identity per epoch | the merge on the public repo; opened-but-unmerged PRs accrue nothing |
| Rule landed in the community corpus (tests + gates passed) | 25 | the corpus commit |
| Node uptime, compute served, fix attempts | **0, by design** | — |

Anti-farming: ≤ 200 base points (row 1) per identity per epoch; the same rule in the same repo counts once per epoch; a maintainer can opt a repo out of the upstream-merge row (the drive-by-PR harm to FOSS maintainers is the failure this row must never fund). Rank weight = Σ points × 2^(−age / 90 d) — the law-2 rank decay; the history it decays over is permanent. **Cadence:** weekly epoch, settle Monday 00:00 UTC — one batch does accrual + rank recompute + anchor (the Cost bar's once-per-epoch rule); row-1 points from epoch N settle at N+1 (a one-week review window). Weights are versioned and STORED AT ISSUE — a `V2` never re-prices points already accrued (the Proposal 006 D4a law). Public copy says "accrue", never "earn"/"reward" (law 3). Serving-cluster nodes post a **node bond** of their own accrued points (a lock, never a transfer; no yield ever; never KAT; a proven fault BURNS it into the existing expiry sink as a third counted door; self-host needs no bond) — the full misbehavior ladder lives in [Plan 335](../.plans/335_local_cluster_fixer_lane_qwen_local_gate_and_cli_runway.md) P2.0(c).

**Anti-spam — `TUNA_ANTISPAM_V1` (owner-armed 2026-11-07; a deposit, never a fee — the T16.2 fair-use policy row it belongs to).** Protects honest contributors from spam and botnets without taxing them. Local `fix`, the dry run, `scan` and `corpus` NEVER touch points (law 1); only SENDING something to the service can require a deposit. Four layers, cheapest first:

1. **Edge rate limit** per identity, checked before any compute or row write (the Cost bar — spam's real target is our Cloudflare budget, not rank).
2. **Identity floor:** GitHub-joined, account ≥ 30 days old with public activity; the claim's existing per-IP (5/h) and per-ASN (100/h) limits + optional Turnstile.
3. **Refundable submission deposit:** each fix record or miss report locks **1 TUNA**, REFUNDED on acceptance, BURNED on junk (fails the compile gate, duplicate, irreproducible miss). Honest contributors pay nothing net; spammers burn their own allowance.
4. **Node bond** (50 TUNA, serving cluster only — Plan 335 P2.0(c)).

**The starter allowance:** `claim` is RE-ENABLED in Posture C as a one-time **100 TUNA starter allowance** (the riir-dapps Proposal 006 B2 grant, `FREE_GRANT_MICRO_KAT` = 10⁸ µ: once per account LIFE, 30-day expiry, unspent allowance burns to the sink, 10 000-claim reservoir with its low-water alarm — a drained reservoir stops grants, refills are a dated operator act). It funds deposits so a newcomer with no contribution history can start immediately (100 honest attempts on day one). **Two-source rule:** every TUNA tranche carries its source — `TrialGrant` (the allowance) or `Contribution` (accrued from accepted work). Allowance TUNA pays deposits (and the later fair-use drain) but NEVER counts for rank and NEVER funds a node bond; only Contribution TUNA does both — so a fresh bot account starts at rank 0 and cannot post a bond. Public copy: "starter allowance" and "anti-spam deposit" — never "gas", "fee", "tokenomics", or any value word.

**The ledger.** `riir-ledger` v1 = points service + optional anchoring, NOT a chain:
balances in the riir-dapps service DB; Merkle-anchored epochs via the existing `anchor_root`
substrate for tamper-evidence; chaind deferred to seal-*; a full chain/ssh-custody revisit
needs a new proposal only if anchoring proves insufficient.

**Existing accounts.** Live KAT accounts: marketing frozen (the posture), internal devnet
flows keep working, and the website FAQ carries the one-paragraph "what happens to existing
balances" note — honesty beats silence.

### Perf/sec posture (design bars for W0's GOAT gate — targets, not measurements)

**Substrate reuse — no parallel system.** The dapps `kat_ledger` arm already IS the
durability-tier store (`local_kv::{LocalKvStore, WalBackend}` + `MerkleTree`), and the
per-batch `kat:statehash:v1` LtHash fold under `kat_tuna` (Proposal 006 B1) is the existing
per-batch tamper trail, with the shipped invariant `Σ balances + healed + expired ==
TUNA_RESERVOIR + topups` pinned per-op (live today: `conservation_holds: true`). riir-ledger
v1 adds rows + a rank table + an epoch anchor on THIS substrate — new storage is refused
unless T16.2 proves one necessary.

| Path | Bar |
|---|---|
| Earn | batched per-machine syncs in the shipped burn-sync posture (README: ~5 s budget, never blocks the run, monotone-watermark idempotent replay); even 10K contributors syncing hourly ≈ 3 QPS average — trivial for the service plane; mints ride the existing TUNA operator batch at epoch settle |
| Drain | **NOT a v1 path** (binding law 1 keeps the drain out of v1 copy and code). The bar below applies WHEN T16.2 defines the drain: a cached-balance O(1) pre-check adding ≤ 1 ms p99, or fully post-hoc accounting — never a synchronous remote-permission check on the decision path (the hosted decision is µs–ms class; the public bench board records the hosted cells at ~7.73 ms p50) |
| Rank | GitHub-joined identities only (anonymous = local-only stats, the anti-Sybil rule). The rank table is computed ONCE per epoch — expiry-as-rank-decay is the natural recompute tick — and served from cache; O(1) queries; never a per-query scan. The rank weight is a DERIVED, decaying quantity over PERMANENT contribution history; the ledger's shipped tranche-expiry mechanics (the bounded sweep burning expired tranches to `expired_micro`, Proposal 006 B4) are unchanged substrate behavior — T16.2 pins how the derived rank weight relates to the ledger balance |
| Anchor | the per-batch LtHash fold (shipped, O(1)-class per mutation) + per-epoch `anchor_root` / `chunked_root` (Bench 488). ⛔ `chunked_root` REQUIRED: plain `MerkleTree` hard-asserts at `MAX_LEAVES = 2²⁰` (~1.05M) leaves, so a million-mutation epoch sits at the panic edge by construction. Roots publish via the public status/explorer JSON (the hourly pass machinery exists). **OFF-CHAIN by decision 6** — no Refine chain consumer; a chain-anchored variant is an explicit decision-6 amendment (owner gate), never a default. No Cold-tier mirror in v1: the statehash trail + public roots + per-op conservation are the tamper-evidence story, and the dapps BOUNDARY admits `local_kv` + `MerkleTree` from neuron-db only — a points mirror would need a new BOUNDARY row (declined for v1) |
| Cost | the plane runs on the kat-service worker — Cloudflare read/write is a standing billing hazard. Bars: earn-sync writes batch per machine; rank recompute once per epoch; stats/rank reads served from cache; no per-query row scans; T16.2 names the per-epoch read/write budget |
| Privacy | published anchors carry ROOTS only; leaf keys salted; no client UUIDs, no GitHub IDs in any published anchor or stat (the client-UUID privacy law + T16.1 gate the implementation) |

**Boundary.** Contribution points fail the three-test rule's value test and rate test by
construction (non-transferable, expiring, written at service rates) — a service-plane
product, never a chain program. The money lane stays Glacial/quorum on riir-chain for the
game economy's settlement (outcome → dapps composer → escrow → chain), outside the funded
scope; the two lanes never share a write path.

**Open-core sustainability (the criterion-6 answer):**

| Layer | License | Moat status |
|---|---|---|
| Modelless core (corpus format + embedder + drafter + pruner + self-evolve) | **MIT/Apache-2.0** — genuine OSI open source, opened months 1–3 | None — deliberately open, it is the commons |
| Community-contributed fix-pairs (data) | **MIT forever** (the consent screen pins it) | Commons — never sellable, by design |
| Fixer/trainer fleet + corpus-production tooling | **MIT — same as the core** (genuinely open source, no restrictions) | Not the moat — the moat is data + operation, not code |
| Our own GPU-mined corpus (from our own repos/rigs) | Proprietary; enterprise-licensable | THE commercial asset — data, not code |

**Why opening fixer/trainer loses nothing — the moat is not code:** (a) the **corpus** the GPU rigs produce is proprietary data — it ships in no repo and no binary; (b) the **GPU operation itself** is a real, ongoing cost — the barrier to reproducing the corpus is compute, not secrets; (c) revenue comes from **enterprise corpus licensing + hosted/support services** — enterprises pay for operation and data, never for the right to run code. A code-restriction boundary was considered and rejected as the default: AGPL-3.0 §13 binds only those offering a *modified* fleet as a hosted service to others — a company producing corpus on its own machines owes nothing, so the license is not the revenue line we might have implied, and selling commercial licenses would additionally require a CLA on every outside contribution. The dual-license alternate is recorded in the playbook (with the CLA task and the corrected AGPL description) as an owner-gated option, not the plan.

**If the grant fails:** the commercial lane continues on its own — enterprise licensing of our own mined corpus + hosted/support services — while the community artifacts (MIT core + MIT corpus + MIT fleet) stay open exactly as the application promised. The plan is the same either way; only who pays the bills changes.

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
| W0 | **Pivot implementation** — model: §"The points plane in full" above (the durable record); task tracker: issue 192 while it lives. Points plane (`riir-ledger` v1 — service + anchoring), the new CLI surface (`join`/`leave`/`rank`/`info`/`corpus` + first-run consent + client-UUID privacy law), T14 sweep re-scope, websites ([Plan 324](../.plans/324_mozilla_web_alignment.md)) | Q1 | The public-goods story is running code, not prose |
| W1 | **Open-source the core**: extract the modelless pipeline into a public repo (MIT/Apache-2.0), public CI + benchmarks, contribution guide — DECIDED (G1) — **and publish to crates.io** (issue 192 GAP 11: crate names + publish pipeline + token secret are owner-gated; supersedes the no-publish law for the extraction only) | Q1 | Public repo + published crates + docs; T15 pre-open gate |
| W1b | **Open the fleet tooling (MIT)**: fixer/trainer service + corpus-production tooling published under the same permissive license as the core; license-and-boundaries table in the public README (the corpus split: community-contributed = MIT forever; own-GPU-mined = proprietary) | Q2 | One clean license story — the business is corpus + services, not code restriction |
| W2 | **Public-repo-free policy shipped**: the free-for-open-source gate implemented, documented, and announced as product policy | Q1 | The charitable commitment is code, not prose |
| W3 | **Community pilot**: 30–50 Rust developers from resource-constrained cohorts (student groups, FOSS maintainers, regional dev communities). Funded as **service capacity + expiring pilot vouchers** — pilot users never pay and never need to contribute | Q1–Q2 | Pilot report: heal rates, retention, offline usage, quotes |
| W4 | **Corpus tools + governance**: publish corpus-authoring tooling + a governance README; onboard 1–2 external corpus contributors | Q2 | Open corpus + governance framework |
| W5 | **Second language domain** via community contribution (proves the framework generalizes and creates a contributor surface) | Q3 | One community-built domain corpus |
| W6 | **Sustainability (open source)**: charitable feature set shipped as the public artifact (no credit/ledger/payment functionality — T14, precision-bound); license boundaries published (MIT core / MIT fleet tooling / MIT community data / proprietary own-mined corpus); durable legal home (fiscal sponsor → foundation path); if the grant fails, the enterprise corpus + hosted/support services lane funds continued work with every community promise intact | Q4 | Sustainability plan per Mozilla's follow-on criteria |

Budget shape (indicative): ~60% people (2–3 contributors), ~15% pilot + community (service capacity, voucher subsidies, travel to one Mozilla Festival), ~15% infrastructure + audit, ~10% contingency. Grant money funds capacity and pilot vouchers — never speculative credit supply. No trading, no profit line in the funded scope.

## Self-grade against Mozilla's six criteria (honest)

| Criterion | Grade | Evidence / gap |
|---|---|---|
| Human-Scale AI expression | **Strong** | All three principles mapped above; the zero-parameter hot path is the headline. |
| Technical viability | **Strong** | Working, deployed, measured (GOAT-gated benches; production use across our own 32-repo workspace since 2026-08); team of experienced Rust engineers with capacity. Novelty: landscape search (2026-10-31, playbook §5) found no shipped modelless healer — local coding tools all ride 7B–14B LLMs. |
| Problem-community clarity | **Medium** | Community defined (resource-constrained Rust devs) but not yet in dialogue — pilot W3 + pre-application outreach are the fix (issue 190 T4). |
| Product-community fit | **Weak — the decisive risk** | No external users yet. Mitigations: installer download counts, dist-repo issues/stars, internal-deployment-at-scale story, soft-launch pilot recruitment; the no-submit gate (190 T10) keeps submission honest. First external upstream contribution **MERGED** (mozilla/sccache#2977 — fixes their #2975, six review rounds 2026-11-02..07, merge commit `907230be`) — an accept, not just a goodwill signal; a **second sccache PR open and green** (#3025, backend read backoff, sent 2026-11-11: full CI matrix 50 pass / 1 by-design skip / 0 fail, answering the issue author's ratified per-level design with their own reproducer measured) — two contributions, still not a user. |
| Impact and values | **Strong** | Open code + open data + free-for-the-commons policy + privacy-by-default + measured benefit. |
| Sustainability potential | **Medium-strong** | Open-core end to end (MIT core + MIT fleet tooling) with revenue from enterprise corpus licensing + hosted/support services — a standard, checkable answer to "how does this endure" (their criterion 6); durable home via W6; team open to models. Re-grade if the model shifts. |

## Draft application answers (source material — owner-voiced before submission; per §Compliance, agent text is source, never paste-ready prose)

**Q: "Briefly describe the public benefit of this project and who will benefit from this work beyond your organization." (149 words)**

Public benefit. cargo refine keeps software development affordable, private, and accessible for developers the cloud-AI economy leaves out. Its repair engine is a small, inspectable corpus of open repair rules: zero parameters, sub-millisecond on ordinary CPUs, running offline on hardware people already own; no account, no subscription, no GPU. Code never leaves the developer’s machine without explicit, redacted, revocable consent.

Who benefits beyond us. Rust developers without cloud-AI budgets: students and learners, indie and FOSS maintainers, educators, and developers in regions where subscriptions, reliable connectivity, or recent hardware cannot be assumed. Open-source repositories are free by policy. The grant opens the modelless core, fleet tooling, and the community-contributed repair corpus under MIT with authoring tools so communities build corpora for other languages, plus Rethink, a research-stage escalation tier: small encoders that abstain when uncertain. Benefit is measured: published benchmarks and a public pilot cohort reported at Mozilla Festival.

**Short summary (50 words)**

cargo refine fixes what clippy's autofix can't: compile errors, Dockerfiles, performance issues no lint catches. An offline corpus is the model; code never leaves your machine. Opt in and secret-scanned fix records teach a shared corpus; every machine's Refine gets better. Rethink adds research-stage encoders that abstain when uncertain.

**Q: Project Details — what technology + how it addresses the cohort theme (199/200 words)**

cargo refine is a repair tool for Rust code. It finds and fixes what compilers and linters report: style warnings the standard autofix cannot handle, compile errors, Dockerfile problems, performance traps, and security anti-patterns. Its repair engine is not a large language model. It is a library of 1,602 inspectable repair rules across seven domains, distilled from real fixes with every source pinned, searched on ordinary CPUs in under a millisecond. Nothing is generated; every repair is a rule a developer can read, question, or refuse. The tool runs offline as one static binary on hardware people already own, and code never leaves the machine without explicit, redacted, revocable consent. Developers who opt in contribute secret-scanned fix records to a shared MIT-licensed corpus, so every installation gets better without anyone surrendering their code. When a problem is too hard, the system abstains or escalates through a calibrated decision layer, including Rethink, a research-stage escalation tier the grant would open: small encoders that decline when uncertain rather than guess. This addresses the cohort's theme: the AI is small (zero parameters in the hot path), intentional (the developer owns their repair memory), and tailored (rules ranked by their codebase's fix history).

**Q: Small — models, hardware, infrastructure (96/100 words)**

The repair engine uses no model in the hot path: the corpus is the model, scored by a zero-allocation vector search on whatever CPU the developer has. No GPU, no datacenter, no account, no network at fix time. One static binary installs from public installers and works fully offline, including contribution accounting, which accumulates locally and syncs when connectivity returns. Rethink, a research-stage escalation tier the funded build would open, uses encoders of roughly 400 million parameters, far below this cohort's 7-billion reference point. Hosted infrastructure is minimal: a small hosted service that records opt-in contributions.

**Q: Intentional — data control + community agency (100/100 words)**

Data collection is opt-in by construction: without explicit consent nothing leaves the machine, and opting out stops transmission; the consent screen will state that contributed records become MIT-licensed data. Local fix memory lives in one folder on the developer's disk, inspectable and exportable, with purge and a resettable installation identifier part of the funded build, where hardware fingerprinting is refused by design. Contributed records are secret-scanned and stripped of repo-private context before they leave. Community agency is structural: the shared corpus and every tool that builds it are MIT-licensed, so communities can distill, audit, and govern corpora for their languages.

**Q: Tailored — how people/problem/context shaped the technology (99/100 words)**

The tool adapts to the codebase, not the reverse. Rules are grouped per domain, and a repository-posture mode applies only the fixes the target project's own lint configuration asks for. Each installation learns which rules work in its codebase, ranking them from the developer's own fix history, so repeated advice matches what helps there. Corpora are authored, not scraped: communities can distill rules for their languages, and a governance guide will ship with the corpus tooling. The design serves developers without cloud-AI budgets, so it must work offline, on old hardware, with no account: that constraint shaped every layer.

**Q: Traction + early product-community fit (149/150 words)**

The tool has run in daily production across a 32-repository Rust workspace since August 2026, where it is the default lint-fixing tool; every applied fix is compile-gated (kept only if the code still compiles) and logged. The corpus grew from one clippy-fixing domain in August to 1,602 rules across seven domains today, mined in auditable batches with every source pinned. Installers shipped in September 2026 (GitHub releases, Homebrew, Scoop); crates.io followed in November. As product-community signal, we scanned two Mozilla codebases, uniffi-rs and sccache, at their own lint postures: one sccache fix is merged (six review rounds), and a second, on an issue whose author tested our prototype with their own reproducer, is open with its full CI matrix green. The remaining gap is honest: no external users yet. The grant's pilot of 30 to 50 resource-constrained Rust developers, now recruiting, and published download metrics test product-community fit openly.

**Q: Challenges + biggest blockers (148/150 words)**

The biggest blocker is product-community fit: the tool works and is measured, but no external community uses it yet, and trust in a repair tool must be earned one merge at a time. The pilot cohort is recruiting now, and recruitment itself is a challenge: reaching student groups, FOSS maintainers, and regional communities takes time and presence. Second, the market frame works against us: developers expect code AI to mean a large language model, so a zero-parameter corpus engine requires explanation, and skepticism is fair until benchmarks are public. Third, executing the openness pivot cleanly: open-sourcing the core, the fleet tooling, and the corpus under MIT while keeping our own GPU-mined corpus separate demands careful license boundaries (commercial use deferred until community trust) and public CI from month one. Finally, capacity: the project is currently built by one engineer; the grant funds the transition to a small team.

**Q: Team + relevant experience (137/150 words)**

The project is built and maintained today by one principal Rust engineer, who designed and shipped the entire 32-repository stack it runs in: the repair engine, the rule-corpus production pipeline, the decision-serving components, and the training rigs behind them. That work is the experience: a production tool with compile-gated fixes, a corpus that grew from one domain to seven in three months, and an upstream fix to Mozilla's sccache merged after six maintainer review rounds. Development runs on a disciplined agent-assisted workflow, with every change gated by compile checks, benchmarks, and recorded reviews; the founder will bring that same discipline to onboarding. The grant funds the planned growth to two or three contributors: a corpus engineer for the authoring tooling and governance work, and community-facing capacity for the pilot cohort, with the founder remaining the technical lead.

⚠ All filed draft texts name Rethink, hedged as a research-stage tier the funded build would open; that inclusion is contingent on the openness amendment (a new owner gate + workplan row, mirroring G1's Research 003 amendment for Refine) landing first. If the amendment does not land, strip every Rethink clause from every text before submission.

Fact pins for these drafts (verdict r1 fixes applied):
- Rule count **1,602** = `.benchmarks/pins_manifest.json` fields summed (clippy_lints.rules 120 + dist_rules 8 + docker_rules 31 + kernel_opt.rules 1076 + rust_perf.rules 314 + sec.rules 51 + shader_perf_rules 8), re-derived 2026-11-11 (was 1,580 at filing 2026-11-06 — Batches 346–348 moved clippy 112→120, rust_perf 302→314, kernel_opt 1073→1076, sec 46→51); TL;DR + Project Details + Traction refreshed in the same commit. ⚠ Re-derive at submission day — the corpus moves weekly.
- Installers: GitHub releases + Homebrew tap + Scoop bucket shipped September 2026; **crates.io `cargo-refine` 0.5.1 LIVE 2026-11-11** (issue 221's publish lane — verified against the registry API the same day; `cargo install cargo-refine` is a real path the Small/installer claims can cite).
- Latency claim scoped to SEARCH only: README pins (drafter p50 ~2.3 µs; search ~400 µs per 10 candidates) — never quote sub-ms on the repair path (the compile gate is seconds).
- Intentional-text split, shipped vs funded-build: MIT disclosure FUTURE TENSE at filing (2026-11-06) — the consent screen that carried it (`--mine` surface, riir-refine `c4fc0a4c`, tracked by riir-dapps Issue 119 T16b) was REMOVED in `5e61bb59` (issue 192 GAP 10); the MIT sentence must be ported to the `join`/first-run consent copy with a pinning test before present tense can be claimed. Inspectable/exportable local memory SHIPPED (`.refine/` + the export path); purge command, resettable client UUID, hardware-fingerprint refusal are FUNDED-BUILD design (issue 192 privacy law), stated as "part of the funded build" in the text.
- sccache PR #2977: **MERGED** — verified via GitHub API 2026-11-07: merged by sylvestre at 12:31:11Z, merge commit `907230be`, six review rounds (Bench 127 ledger). Traction + Team + self-grade updated in the same commit per this pin's standing instruction.
- sccache PR **#3025** (backend read backoff, the #2968 lane): **SENT 2026-11-11, OPEN, mergeable-clean, full CI matrix GREEN (50 pass / 1 by-design skip / 0 fail)** — verified via GitHub API 2026-11-11. The issue author (mitander) ratified the per-level design on-thread 2026-11-10 and their parallel reproducer is measured in the PR body; first responder was the Codecov bot (telemetry — 88.62% patch coverage, the project delta a fork-upload artifact; never replied to). Traction Q + self-grade updated per this pin. Second contribution, still not a user — the self-grade keeps "Weak".
- Rethink: every text hedges "research-stage … would open"; strip-all rule stands if the openness amendment does not land.
- Word counts in headings assume hyphenated compounds = one word and no standalone em-dashes (the MS-Word counting trap).

## Owner gates (must resolve before submission)

- **G1 — Openness amendment — DECIDED 2026-10-31 (owner, issue 192): YES, open-src.** Open the modelless core under MIT/Apache-2.0 with a dated commitment (amends Research 003 for riir-refine; KAT/commercial surfaces stay private). Execution: W1 + the T15 pre-open gate.
- **G2 — Event artifact + points posture (was "event posture").** (a) Confirm Posture A mechanics unchanged: installers gated on T14a + T14b + T14 sweep; (b) internal dev builds unchanged — enabling `kat_service` restores the full flows; (c) the points plane (Posture C) is DESIGN-disclosed in the application, implemented as W0/Q1 — issue 192; (d) license boundaries published as in the open-source table (MIT core + MIT fleet + proprietary own-mined corpus); the dual-license fleet alternate stays owner-gated. Issue 190 T2 + issue 192.
- **G3 — Evidence pack** (executable now). Installer downloads, dist-repo signals, internal-usage stats into the application. Issue 190 T3.
- **G4 — Entity + submission** (decide by 2026-11-07; prose Nov 17–19 only if the no-submit gate passes). **Runway half DECIDED 2026-11-07 (delegated-owner verdict PROCEED, recorded at the decide-by date — reversible; holding would lose the Nov 20 IPP window):** IPP prep continues toward the Nov 20 registration. **Still owner-only by rule, not by gate:** the submitted prose must be owner-voiced (Mozilla AI-use policy, T8), the receiving entity needs owner residency/tax facts (T5, deferred to shortlist), the consult email has no agent outbound path (T7), and the submit/no-submit call stays issue 190 T10 on 2026-11-17. Fiscal sponsor resolved; final prose owner-voiced; single proposal submitted. Issue 190 T5/T7/T8/T10.

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
