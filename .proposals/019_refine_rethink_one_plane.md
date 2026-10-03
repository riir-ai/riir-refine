# Proposal 019 — one product family, one plane: refine × rethink DRY + the shared dapp + the DB-style quorum ladder

**Status:** PROPOSED — rev 2 + round-3 amendments; **Claude verdict AGREE (round 2, closed round 3** — the leaf-contract splits, the trait-not-module oracle law, the mirror deletion, the build-stamp renderer, the dapps quorum-core extraction with per-lane canary polarity, the assigned-N/close-rule/audit-before-slash/sybil/even-N/legacy quorum rules, and the pinned-unbuilt CLI list all reviewer-checked against the code) — owner directive 2026-10-03, two parts: (1) "riir-refine and riir-rethink should be nearly identical and aligned as possible bc it's nearly same thing just twist only how to mine… feel free to propose to refactor both for alignment, breaking is fine"; (2) "do 2/3, 5/9, … like db — riir-refine and riir-rethink should DRY code and share dapp too" (read: verification quorums scale like replicated-DB majority quorums — 2-of-3, 5-of-9 — and the two products share ONE service plane).
Branch: `develop` (per global rule)
Owner: unassigned (directive recorded; the breaking-change allowance is explicit)
Fusion of: dapps Issue 082 (the wire compiles ONCE — the server-half law this extends to the client half) × the riir-kat leaf spin-out (Issue 088) × riir-dapps Plan 043 (the lane-generic ingest: fix + dec + healqual from one core) × [Proposal 015](015_replay_endorsement_reward_class.md) (the K=2 replay-verifier shape the ladder generalizes) × [riir-rethink Proposal 003](../../riir-rethink/.proposals/003_rethink_bin_stake_tier_leased_artifacts.md) (the sibling product this aligns with) × riir-refine Proposals 003/005/013 (the dist pattern, the economy, the tiers)
Related: `reflex` stays OUT of this family (the free floor, unmetered) except as an optional late adopter of the shared build-stamp renderer; riir-kat gains wire + traits (never product code); riir-dapps gains the quorum core + ladder

## TL;DR

`cargo refine` (heals code) and `rethink` (serves decisions) are ONE product family with ONE twist — **what is mined** (heal spans vs decstat receipts) — so they share everything except the twist. The audit says we are already substantially aligned by design: the wire composers + most client modules live in riir-kat, and the dapps ingest is lane-generic. This proposal finishes it: (1) the remaining refine-local client surface splits — wire halves to riir-kat, healer-coupled projections stay in refine; the two contribution lanes become ONE `MiningLane`/`VerifyOracle` trait pair (traits in kat, impls in the products); the dapps sync-payload MIRROR is deleted via `kat_protocol_sync` (Issue 082 completed, not re-pinned); (2) the CLI surface becomes a written parity contract with a pinned not-yet-built list; (3) one reusable release workflow feeds two source-free dist repos; (4) ONE dapps plane — one ledger, one stake position read by both products' tiers, one epoch settle — and its verification quorums become a **round**, not a race: assigned-N majority voting (2/3, 3/5, 5/9), a close-of-round rule, a dissent/slash ledger, stake-floor sybil resistance, per-lane quorum classes.

## The owner law (recorded)

One family, one plane, the twist is the mine. Any code, wire, CLI verb, gate, or pipeline that exists in both products is a bug until it exists once — in riir-kat (wire + traits), riir-dapps (server), or a reusable workflow (release) — with per-lane parameters where the products genuinely differ. Breaking changes to reach it are pre-authorized.

## The alignment audit (grep-verified 2026-10-03; rev-2 corrected)

| Layer | Already one home | Residue (this proposal's work) |
|---|---|---|
| Wire composers | riir-kat `kat_protocol_*` (burn, payreq, consent, decstat±receipt±verdict, fixstat, lease, replay, siwr, stake, shop, seal, vote, wetrun, account) | the sync-payload shape (dapps still carries a DELIBERATE mirror, `kat/mining.rs:100-104`, only because the canonical copy is healer-coupled — fixed by Move 1) |
| Client modules | riir-kat: `kat_account(+client)`, `kat_burn_client`, `kat_refill`, `kat_soft_gate`, `kat_meter`, `kat_redact`, `kat_mining_consent`, `kat_service_config/urls`, the stake/payreq/vote/shop/seal/siwr/wetrun/lease/replay/decstat/fixstat/adoption clients | refine-local, **two clean movers**: `kat_footer` (no crate imports), `kat_stats_consent` (depends only on kat's `kat_mining_consent`); refine-local, **four healer-coupled — split, not moved** (their healer imports measured): `kat_promotion` (imports `domain`, `domains`, `draft`, `memory`, `score_bench`), `kat_sync_payload` (`traj_store`, `rule_domains`), `kat_pending_estimator` (`traj_store`), `kat_replay` (`draft`) — the wire shape/DTOs go to kat, the projection from healer data stays in refine (riir-kat's BOUNDARY.md forbids the whole-module moves; moving them whole would make the leaf depend on riir-refine) |
| Contribution lane (server) | dapps lane-generic INGEST (`ingest_stat_run`: fix + dec + healqual from one core) | the client push/verify stacks are two parallel shapes; dapps VERIFICATION is shared-shape-not-shared-code — `DECSTAT_QUORUM_K` (`kat/decstat.rs`) and `REPLAY_QUORUM_K` (`kat/replay.rs`) are two constants + two tally loops (Move 4.0) |
| Economy | ONE `KatLedger`, ONE epoch settle, TUNA-first burn, staking tranches (007/017), soft gate | tiers not formalized as cross-product (no single stake read + no cross-product slash propagation) |
| Release/dist | the pattern exists, twice-divergent: refine's `release.yml`/installers/leak-scan + reflex's port — the two `build_stamp.rs` copies differ by ~404 diff lines (a UNIFICATION, not deleting identical twins) | no reusable workflow; no parity gate; rethink's pipeline unborn |

## Move 1 — the lane-generic contribution client (traits + wire in kat; projections + oracles in the products)

riir-kat gains the SURFACES only:

```
// riir-kat: traits + wire DTOs. NO product imports — the leaf stays a leaf.
trait MiningLane {
  fn lane_id(&self) -> &'static str;            // "heal" | "dec" (the dapps lane key)
  fn outbox_row(&self) -> OutboxRow;            // lane-generic machinery, lane-keyed
  fn push/lease_pull(...)                       // identical for both lanes
  fn oracle(&self) -> &dyn VerifyOracle;
}
trait VerifyOracle { fn verify(&self, lease: &LeaseItem) -> Verdict }   // THE TWIST LIVES HERE AND ONLY HERE

// riir-refine: HealRecompileOracle (rebuild the span's module + the clippy oracle; imports draft/compile_oracle — product-side, correct)
// riir-rethink: DecReplayOracle  (re-run the receipt's input on the same release, compare decision hashes; imports riir_reflex::harness::suites + riir_instinct::server — product-side, correct)
```

- The refine moves: `kat_footer` + `kat_stats_consent` move WHOLE. `kat_sync_payload`, `kat_replay`, `kat_promotion`, `kat_pending_estimator` SPLIT — wire/DTO halves to kat; the healer-data projections stay (e.g. a new `kat_protocol_sync` in kat carries the sync-payload shape; refine keeps the code that builds it from `traj_store`/`rule_domains`).
- **The dapps sync-payload mirror is DELETED, not re-pinned** — with the shape in `kat_protocol_sync`, dapps imports it the Issue-082 way (`kat/mining.rs:100-104` records the mirror as deliberate ONLY because the canonical copy was healer-coupled; the coupling is what this split removes). No hand mirror survives to drift. The atomic-family risk stands (caveat 1).
- Consent stays THREE-TIER (`Unset`/`OptedIn`/`OptedOut`), per-product env names, one record shape, one default-dir resolution.

## Move 2 — the CLI parity contract (written, gated, honest about what does not exist yet)

One table, both bins, asserted by `kat_cli_parity_gate` in each repo: shared verbs `login`, `account`, `sync`, `stake`, `pay`, `faucet`, `wetrun`, `refill`, `--info`, `--version`, `--yes`; identical refusal shapes (every refusal names the remedy + exit code), the kill-switch spelling law, the auto-login chain, the auto-sync tails, the just-works defaults. Product verbs stay per-product (`refine: --fix/--suggest/--fix-compile/--mine/--unmine…` vs `rethink: serve/add/arsenal…`; `activate` is a rethink PRODUCT verb — the Plan 007 G3 entitlement lane; refine has no entitlement plane, so it is pinned rethink-only, not shared — otherwise refine's own gate is red on day one).

**Pinned not-yet-built (rethink)** — the gate carries this list, entries carry a reason, and the list may only ever SHRINK: `login`, `account`, `sync`, `stake`, `pay`, `faucet`, `wetrun`, `refill`, `activate` (rethink-only), `--info`, `--version`, `--yes` — all "not yet built; Plan 007 G3 + this contract"; today rethink ships only `serve`. (Refine implements the shared set today.) A parity gate without this list is red on day one or vacuously green — the list IS the gate's honesty.

## Move 3 — one release pipeline, two dist repos

- `build_stamp` becomes a **pure renderer in riir-kat** — `render(features, version, product_name) -> String` — plus a shared `build.rs` HELPER shipped as a build-dependency. The products keep their own `env!("…COMPILED_FEATURES")` / `CARGO_PKG_VERSION` reads and call the renderer. (Moving the module whole would silently stamp riir-kat's features and version — a wrong stamp with no error; the env! reads are load-bearing product facts.) The two existing copies (refine, reflex; ~404 lines apart) unify on the renderer; rethink adopts at birth; breaking: refine's `release.yml` inlines shrink to `uses:`.
- A REUSABLE workflow (the katgpt-rs `sibling_docs_drift.yml` precedent): `kat-release.yml` — dist profile, `--remap-path-prefix`, leak scan, `SHA256SUMS`, `THIRD_PARTY_LICENSES`, installers, the no-source CI guard, the stale-binary stamp check — consumed by `gist-rs/cargo-refine` and `gist-rs/rethink`.

## Move 4 — one dapp, and the verification ROUND (not a race)

The shared plane: ONE dapps worker is the economy backend for the family — ONE ledger, ONE stake position per account (a stake qualifying for a rethink tier qualifies for the refine tier table and back; the tier TABLES stay per-product, the stake they read is one; a stake SLASHED in one lane drops the tier in BOTH products — else cross-product arbitrage), ONE epoch settle paying every lane's verified rows.

### 4.0 — extract ONE quorum-round core in dapps first

`kat/decstat.rs` and `kat/replay.rs` each carry their own `*_QUORUM_K = 2` and their own tally loop. The ladder is built ONCE, on one extracted round core (assignment, tally, close, dissent recording), consumed by both lanes — never twice. **Canary polarity is a per-lane parameter** (decstat's canary polarity is INVERTED relative to the replay lane — `decstat.rs:26`), and the D0 parity gate covers canary verdicts as well as confirm/refute — else "byte-identical for existing shapes" passes while one lane's honesty stamp flips.

### 4.1 — the race that exists today, and the round that fixes it

Today decstat confirms on the FIRST 2 `Verified` and — as an independent threshold — refutes on 2 `Mismatch`. That is a race, not a vote: past 3 verifiers both thresholds can be crossed and whichever arrives first wins. The round fixes it with three rules:

1. **Assigned-N**: the N verifier accounts are FIXED AT LEASE TIME (randomized draw from the stake-qualified pool) — not "whoever shows up".
2. **Close-of-round**: the round closes when all N have reported OR the round deadline passes; the verdict rule is stated once — **K of assigned N, else re-lease** (never a majority of whoever reported; a round that cannot reach K re-leases, which subsumes any separate minimum-report threshold).
3. **Dissent ledger + audit-before-slash**: both oracles are deterministic — any mismatch is evidence that someone lied. A split verdict first records dissent rows and triggers an AUDIT (a re-round at a higher class, or resolution against canary known-answer evidence); slashing happens only after the audit settles who lied. The minority count alone is NEVER enough — else a colluder majority slashes honest whistleblowers and gains a griefing weapon.

### 4.2 — the ladder ("2/3, 5/9 … like db")

| Class | Assigned N | Quorum K = ⌊N/2⌋+1 | Forging needs | Censoring needs |
|---|---|---|---|---|
| K2-bootstrap (LEGACY — the settled rows of the K=2 era, grandfathered; not a class new rounds may take) | pool | 2 | — | — |
| 3 | 3 | **2** | 2 colluders of 3 | 2 |
| 5 | 5 | **3** | 3 of 5 | 3 |
| 9 | 9 | **5** | 5 of 9 | 5 |

- **Why majority-of-N (not BFT 2f+1-of-3f+1)**: the owner's "2/3, 5/9" is exactly `⌊N/2⌋+1`; BFT supermajorities give 3/4, 5/7, 7/10 and never 5/9. With one trusted tallier (the dapps ledger) and no peer-to-peer agreement, majority voting is the correct strength: a confirm majority and a refute majority cannot BOTH form (⌊N/2⌋+1 + ⌊N/2⌋+1 > N — verdicts are mutually exclusive); forging a row needs K colluders among the assigned N; censoring needs N−K+1.
- **Sybil resistance comes from stake, not K**: each assigned verifier carries a stake floor (a `KatConfig` constant per lane); the draw is randomized over the stake-qualified pool — colluder probability is a function of the colluder SHARE of staked identity, and K sets how much of the draw they must win.
- **Even/odd pools**: classes are the canonical sizes (3, 5, 9); a pool of N rounds DOWN to the largest class ≤ N (a 7-verifier pool runs as class 5 + 2 standby). No formula applies off the table.
- **Per-lane classes are independent** (cost scales with K and the oracles differ by 3–6 orders): the heal lane (recompile = seconds/verifier) may stay at class 3 while the dec lane (replay = µs–ms) climbs to 9. The floors are measured PER CLASS PER LANE before any arming (the 015 hold law: a cut below the measured verifier floor pays nothing); two-key arming (`*_cut_bp` + `*_reward_micro`) unchanged; devnet-first.

## What stays per-product (the twist, protected)

The oracle cores, the corpora (rules vs arsenal/specialists), the meters (code-word tokens vs decisions), the domain verbs, the per-lane verifier floors and class choices, the manifests. These NEVER merge — merging them would erase the products.

## Breaking changes (pre-authorized, listed — rev 2 exact)

1. The four healer-coupled modules SPLIT (wire halves to kat — incl. the new `kat_protocol_sync`; projections stay in refine); only `kat_footer` + `kat_stats_consent` move whole.
2. The dapps sync-payload mirror is DELETED (dapps imports `kat_protocol_sync` the Issue-082 way) — one atomic family: refine + kat + dapps in one push window.
3. rethink's `decstat*` client modules collapse into the `dec` lane impl + `DecReplayOracle` (pre-1.0 private repo; no external consumers).
4. refine's `release.yml` de-duplicates into the reusable `kat-release.yml`; both `build_stamp.rs` copies unify on the kat renderer.
5. The verification wire carries the round's assigned-N + class (additive; old rows read the grandfathered `K2-bootstrap`).
6. The decstat/replay tally loops are REPLACED by the one round core (the race fix — behavior change at N>2 only; N≤2 pools unchanged).

## Phases

- **Phase A** — the moves + splits: clean movers to kat; the four wire/DTO splits; `MiningLane`/`VerifyOracle` traits in kat; both products' lane impls; the dapps mirror deletion family; the riir-kat `BOUNDARY.md` "Does not own" rows edited to match the split ("projection stays, wire half moves").
- **Phase B** — the parity contract md + `kat_cli_parity_gate` in both repos (shared-verb table + the pinned not-yet-built list with reasons; the list only shrinks).
- **Phase C** — the build-stamp renderer + build.rs helper in kat; the reusable `kat-release.yml`; both dist repos consume it.
- **Phase D0** — the dapps quorum-round core extraction (one tally, one assignment, one close rule, the dissent ledger — both lanes consume it at K=2 parity first, byte-identical verdicts for existing shapes).
- **Phase D** — the ladder: class table + stake floors + randomized assignment + close rule, per-lane class arming (owner-gated per class, per lane, after per-class floor measurement on devnet).
- **Phase E** — the cross-product tier table read + the cross-product slash propagation (one stake position; slash in one lane drops both products' tiers).

## Gates

- All existing refine gates green through the moves (lib counts re-pinned; the devnet e2e `ALL E2E CHECKS PASSED`).
- The dapps contract tests (fix + dec lane separation, conservation, canary) green at every phase; the mirror deletion lands as ONE family (refine + kat + dapps in one push window) or not at all.
- D0's parity gate: for existing N≤2 shapes, the round core reproduces today's verdicts byte-identically — confirm/refute AND canary verdicts (per-lane polarity parameterized).
- The ladder: mutual-exclusivity tests (a confirm and a refute quorum cannot both form at every class), close-rule tests (deadline + all-reported), dissent-recording tests (mismatches recorded, escalation fires), conservation-by-construction extended to K=3 and K=5 pools.
- The parity gate reds on any unilateral verb/refusal divergence OR any growth of the not-yet-built list.

## Honest caveats

1. The mirror deletion is the riskiest step — dapps pins refine's sync payload shape by golden test; the family MUST land atomically or devnet ingest breaks (the Issue-103 deploy-order class).
2. Majority quorums raise per-row verification burn ~linearly in K, and the two oracles differ by 3–6 orders of magnitude — classes arm per lane per class, on measured floors, never inherited.
3. One stake position across products is a feature (liquidity of commitment) and a risk (one tier-qualifying stake unlocks both products' gates AND one slash drops both) — per-product tier constants stay owner-gated so the unlock sets differ deliberately; the slash propagation is part of Phase E, not an afterthought.
4. `reflex` adoption of shared pieces is optional and late — do not block this family on it.
5. Nothing here changes A10/HOSTED-ONLY or rethink 003's Option A posture; the ladder governs verification lanes only.
6. The tallier is trusted (the dapps ledger) — the ladder hardens against colluding VERIFIERS, not against the operator; operator trust is the 051 governance-tranche question, deliberately out of scope here.

## TL;DR (closer)

Verdict: FINISH THE ALIGNMENT — one client plane (kat: wire + traits), one contract (parity-gated CLI with an honest not-yet-built list), one pipeline (reusable release), one service plane (dapps) whose verification becomes a real ROUND (assigned-N majority, close rule, dissent ledger, stake floors — 2/3, 3/5, 5/9). The twist — the mine — stays exactly where it belongs: in the product-repo oracle impls.
