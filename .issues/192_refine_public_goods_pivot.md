# Issue 192 — Public-goods pivot: open-src Refine, TUNA points, the new CLI surface, Rethink = PoC (owner v4 directives)

**Date:** 2026-10-31
**Note:** allocated 192 after a same-day collision on 191 (191 = kernel_opt goat-gate scoping, first allocation wins; dual_allocation_gate + first-allocation rule; this file was never committed at 191).
**Status:** open — owner directives 2026-10-31; supersedes the KAT-era posture for the public product; feeds [Proposal 025 v4](../.proposals/025_mozilla_human_scale_ai_incubator.md) + [Plan 324](../.plans/324_mozilla_web_alignment.md) (websites)
**Owner decisions recorded:** Refine open-src (decided); moat moved to Rethink (open later after running); sites drop pricing (PoC phase); seal-*/riir-ai = the real moat (separate business/brand, PRs not accepted, nothing leaks back/forth); release = main+tag; KAT requirement ripped out; TUNA = contribution points; no more mainnet for refine (proposal 023 DEFERRED-FROZEN, issue 175 removed → HISTORY); seal-* mainnet/devnet on hold until localnet multi-authority resolves (chain issue 174).

## Decisions (owner, verbatim-intent)

1. **Refine must be open src** — resolved: G1 in issue 190 is DECIDED YES.
2. **Moat moved to Rethink** — Rethink openable later after running a while; websites remove pricing; Rethink = "ongoing PoC until we beat all decision lanes possible"; Refine = real-life coding use, closed beta, corpus stacking, other languages/domains planned.
3. **Release flow** — main + tag (gitflow-style): `develop` stays the working branch (global rule); `main` = release branch; releases = tags on main (release.yml already builds on `v*` tags — formalize, no new machinery).
4. **KAT requirement ripped out; TUNA = points** — contribution shows off as TUNA points (non-transferable, expiring reputation); the drain only applies to temp offline/private use as penalty/debt against misuse. No purchase. No KAT for users.
5. **New CLI surface** (see the table below) + a light ledger (`riir-ledger` or similar) — NOT full chain, NOT ssh-key custody: a subset.
6. **chaind positioning** — points tier = fast mint (operator-batch shape, no 2/3 quorum wait — this already exists as the TUNA operator batch); chaind promotion continues for seal-* only; no Refine chain consumer; Rethink may consume later.
7. **seal-* mainnet/devnet on hold** until localnet multi-authority (docker same-rig + cross-rig) resolves — filed as [riir-chain issue 174](../../riir-chain/.issues/174_seal_localnet_multi_authority_precondition.md).

## The new CLI surface (owner's table, gaps filled)

| Verb | Behavior | Notes/gaps flagged |
|---|---|---|
| `cargo refine` | dry-run scan (unchanged) | — |
| `cargo refine fix` | **free, no join needed**; first-run consent ONCE for secret-blind data send (spans/results, `allow_redacted: false` policy); asks join-or-anonymous | Local healing never needs consent or network — consent gates the SEND only. Keep a fully-offline mode (`RIIR_REFINE_OFFLINE=1`) documented for air-gapped users |
| `cargo refine join` | join contribution: policy accept + GitHub login | Reuses shipped substrate: `login --provider github` (device flow) + the mining-consent screen + `.refine-sync-policy` redaction. No new auth code |
| `cargo refine left` | leave contribution: log off, zero data sent | **Naming flag:** `left` reads as past tense; ship `leave` with `left` as hidden alias (owner's word kept as alias) |
| `cargo refine rank` | top-ten leaderboard + own rank | **Sybil flag:** anonymous/derived-ID users must NOT be rank-eligible (rotating identity farms the board) — rank requires GitHub-joined identity; anonymous = local-only stats |
| `cargo refine info` | top fixed repos by domain (`--perf`, `--clippy`, …) | Aggregated, redaction-policy-scoped; no repo content, counts + domains only |
| `cargo refine corpus` | corpus info + update offer | The corpus-lease/update surface minus KAT — points-free (corpus freshness is a public good) |
| `cargo refine update` | corpus lease refresh (exists) | Drops the KAT charge line in the public build |
| `cargo refine upgrade` | binary self-update (exists) | unchanged |
| `cargo refine version` | NEW subcommand (alias of `--version`) | keep `--version` working |

## Gaps the owner asked me to fill (the misses)

- **GAP 1 — the derived unique ID is a fingerprinting pattern (the big one).** Hashed NIC MAC + hostname is trivially reversible (small MAC space → rainbow tables; hostnames → dictionary) and stable across networks = a persistent cross-context tracker. For a Mozilla "Intentional"-principle application this is the single most attackable design choice. **Fix (recommended):** a locally generated random client UUID (resettable by the user), stored in the machine-local config; the salted hardware hash may exist ONLY as an optional anti-abuse signal, salted with a server-side per-join secret (never unsalted), disclosed in the consent text in plain words ("a random ID we generate; we never read your hardware"). Decide: proposal-level (T16.1 below).
- **GAP 2 — T14 sweep re-scope.** The landed sweep pattern (`mine|KAT|earn|payout|epoch|settle|lease|credit|join|service`) now collides with PRODUCT surfaces (`join`, `rank`, `corpus`, service endpoints). Re-scope the bar to payment/token terms only (`KAT|payout|epoch settle|lease|pack|USDC|wallet|topup`) and add positive assertions (join/points/corpus surfaces PRESENT and consent-gated). Amend T14's note; the charitable gate keeps its hard-zero KAT classes.
- **GAP 3 — Posture wording.** "No credit/ledger/payment functionality" becomes **"no payment, no token, no speculation — contribution points are non-transferable, expiring reputation"** (proposal 025 v4). The points plane ships as grant work (Q1), so the pre-application artifact stays free-fix-only; the application discloses the points DESIGN, not a shipped system.
- **GAP 4 — "drain as penalty/debt" ambiguity — tightened per verdict r1:** local `fix` is ALWAYS free and never blocked; points never go negative-locked. **v1 public copy leaves the drain OUT entirely** (copy gate in Plan 324); the numeric fair-use policy (drain rate, balance cap, refill paths) is defined in T16.2's proposal before any drain ships — until then the only public sentence is "points meter hosted lanes as fair use".
- **GAP 4b — expiry vs reputation (verdict r1):** expiry must NOT delete history — recommended resolution (owner-gated in T16.2): the **balance** (rank-eligible weight) decays for leaderboard ranking only; **contribution history and corpus contributions are permanent** (the corpus is MIT data forever). Expiry-that-erases reads as an engagement trap — refused.
- **GAP 5 — GitHub org/app for join.** The device-flow client id exists (`Ov23liAe58wk2byrYeOv`); decide the app ownership + scopes for the public product (T16.3).
- **GAP 6 — `riir-ledger` (owner asked).** Yes — but v1 = **points service + optional anchoring**, not a chain: balances in the riir-dapps service DB (the TUNA plane already has fast operator mint, expiry, non-transferability — exactly points semantics), Merkle-anchored epochs via the existing `anchor_root` substrate for tamper-evidence, chaind deferred (seal-* only). A full chain/ssh-custody revisit goes to a proposal only if anchoring proves insufficient. Named decision recorded in T16.4; the name `riir-ledger` is fine for the wire/leaf crate.
- **GAP 7 — existing-account migration.** Live mainnet/devnet KAT accounts: freeze marketing (done by posture), keep internal devnet flows, and write the one-paragraph "what happens to existing balances" note into the website FAQ (honesty beats silence).
- **GAP 8 — sites** = [Plan 324](../.plans/324_mozilla_web_alignment.md) (item 1 of the pivot).
- **GAP 9 — seal moat reading — DECIDED 2026-10-31 (owner, emphatic): seal-*/riir-ai MUST STAY PRIVATE. Reading (b) is dead.** The owner's words settle it: the "open it" reading was a misparse of item 4's phrasing ("we wont accept PR for now so it should be fine for open it" = *it is safe to open REFINE/RETHINK because seal accepts no PRs and nothing leaks* — never "open seal"). Recording firmly: **seal-*/riir-ai are the private gated moat, the real business, a separate brand — they never appear in public copy, never enter a grant scope, and their source stays closed; opening them is not a proposal unless the OWNER initiates one.** Plan 324 **G-A is RESOLVED** — Phase 1 (site copy) is UNBLOCKED, with the standing constraint: zero game-business content on any front.

- **GAP 10 — vocabulary law (owner, 2026-10-31): the word "mining" is BANNED from public copy and user-facing docs** — it carries crypto-extraction connotations and causes negative feeling. Public vocabulary: **contribute / contribution / contributor / contribution points**. Scope: all four website fronts, README/landing copy, consent screens, CLI help text, the application. NOT in scope: internal code identifiers (`kat_mining_consent`, `run_mine_intro` — renamed only when touched), the T14 sweep's detector patterns (they exist to catch the OLD KAT surface, including the word "mine"), and internal repo docs describing that code.
- **GAP 11 — crates.io publication (owner, 2026-10-31): when the extraction opens, it publishes to crates.io directly** — distribution upgrades from installers-only to normal `cargo add`/`cargo install` consumption. This supersedes Research 003's no-cargo-publish law FOR THE EXTRACTION REPO ONLY (private crates keep `publish = false`). Consequences folded into T16.5: crate-name decisions (availability + naming convention, owner), publish pipeline (CI step + crates.io token secret — owner-gated secret setup), docs.rs-ready metadata (license files, description, repository links), and crates.io download counts join the evidence pack (Plan 324 T2.3).

## Tasks

- [ ] **T16.1 — Privacy/ID design note (BLOCKS the points implementation).** One page: client UUID + optional salted anti-abuse signal + consent text + retention/derivation table. Feeds the application's Intentional answers.
- [ ] **T16.2 — Points-plane proposal (`riir-ledger` wire + earn/drain schedule + leaderboard rules).** Proposal doc in `.proposals/` (next number); reuse: mining consent, GitHub device flow, sync-policy redaction, TUNA operator mint, `anchor_root` anchoring.
- [ ] **T16.3 — GitHub app ownership/scopes decision (owner).**
- [ ] **T16.4 — `riir-ledger` naming + anchoring-tier decision (owner; GAP 6 recommendation = service DB + anchor_root, chaind deferred).**
- [ ] **T16.5 — CLI surface implementation plan** (verbs table above → phases; feature-gate map for charitable vs internal builds; sweep re-scope from GAP 2 lands here as a task row; **crates.io publication per GAP 11** — crate names (owner), publish pipeline + token secret (owner-gated), docs.rs metadata; **vocabulary law per GAP 10** — no "mining" in any help text/consent screen/README).
- [ ] **T16.6 — Websites** → execute [Plan 324](../.plans/324_mozilla_web_alignment.md).
- [ ] **T16.7 — Mainnet-freeze residue** (done in this commit): proposal 023 DEFERRED-FROZEN; issue 175 removed → HISTORY; issue 188 un-freeze note; the KAT billing/orientation docs get a "frozen lane" banner on next touch (not blocking).
- [ ] **T16.8 — seal hold pointer** → [riir-chain issue 174](../../riir-chain/.issues/174_seal_localnet_multi_authority_precondition.md).
