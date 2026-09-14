# Proposal 015 — Replay endorsement reward class (the Max earn class, C1→C5 economics)

Status: **draft — Phase 1 LIBRARY SLICE LANDED 2026-09-14 (riir-dapps `00b6583`: the lease/verdict/agreement/canary lane + the conservative split + G1-a..d/G3 gates green, inert at cut=0; the worker HTTP routes + the G2 µs figure remain — Plan 029 T1.4); Phase 2 = the `cargo heal replay` client; Phase 3 = the econ-sim arm + owner ratification of a non-zero `replay_cut_bp` (owner-gated)**
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: [Proposal 013](013_consumer_install_tier_spec.md) T3.1 (the tier that needs the class) × [Proposal 005](005_kat_heal_service_network.md) §Mining (the C1→C5 escalation this economizes) × [Proposal 010](010_kat_yield_ceiling_and_treasury_funded_free_tier.md) (the sub-split + distribution-neutral-at-the-parent discipline) × TrueBit's forced-error incentive ([arXiv:1806.11476](https://arxiv.org/abs/1806.11476), adapted — cited-only) × the SHIPPED `VerdictVector.commitment()` / `verify_no_verdict_split()` / `ProofVerifier` substrate (`riir-dapps/src/kat/mining.rs`)
Related: [Proposal 007](007_kat_usdc_staking.md) (the sub-split precedent `usdc_kat_sub_split_bp`), [Proposal 008](008_adaptive_tokenomics_riir_dao.md) (versioned-bounded-movable lever law), [the tier spec](../.docs/01_orientation/node_tiers.md), [`l4_fallback.md`](../.docs/10_self_evolve/l4_fallback.md) (the serving-lane decision P013 builds on)

## TL;DR

Give the Max tier (the no-GPU contributor) a settleable earn class by paying
third-party **replay verifiers** a bounded cut of each confirmed mining row's
reward. The mechanism is the designed-but-degenerate C1→C5 escalation path
P005 already named: today's proof is `C1BootstrapVerifier` — a documented
**mock** that structurally ratifies the submitter's claim and never actually
replays. This proposal replaces the mock's monopoly with **real replays run by
leased participants** (`fix_verify` + clippy oracle on their own rig — the
substrate every `cargo heal` binary already ships), confirms rows on
**replica agreement over verdict commitments** (the exact bytes
`VerdictVector.commitment()` documents as "what replica agreement is checked
on"), and splits each confirmed row's reward **conservatively**: miner
`w·(1−cut)`, agreeing verifiers share `w·cut` — total weight unchanged, zero
new emission math, the entire `RewardRow → unpaid_rewards → settle_epoch →
pay_due_rewards` path reused verbatim. Rubber-stamping is priced by a
**canary discipline** adapted from TrueBit's forced errors: the lease mixes
secret known-bad rows; catching one is a jackpot multiplier, stamping one
zeroes the epoch. Genesis ships `replay_cut_bp = 0` — structurally inert
until the Phase-3 sim + owner ratification arm it. This is our own machinery
being economized; TrueBit is cited prior art for the verifier-payment problem,
not a distillation source.

## The problem this solves

1. **The tier table's honest hole.** P013's Max row reads *"replay class
   (new, undesigned)"* with the law *"do not advertise replay earnings until
   it can settle"* (P013 caveat #2). Max is the only tier whose earn class
   names a mechanism that does not exist.
2. **The proof path is a mock.** `riir-dapps/src/kat/mining.rs` ships the
   full quorum shape — `QuorumConfig` (with `c1()` documented as the
   *"C1 LocalOnly bootstrap → C5 DistributedConsensus"* path), the
   `ProofVerifier` trait, `prove()`, `verify_no_verdict_split()`,
   `ProofVerdict::{Proven,Failed,Quarantined,VersionSkew}` — but the only
   shipped verifier is `C1BootstrapVerifier` (kat-service `prove.rs`), which
   *"ratifies the submitter's claim when the proof object is structurally
   sound"*. No compile gate, no oracle, one replica. "Quorum-proven novelty"
   is honest vocabulary over a degenerate implementation.
3. **Nobody can carry the replay load.** Real service-side replay means the
   worker executing untrusted fix text against cargo projects — a container
   lane (cost, complexity, the P005 leak-posture surface). Meanwhile CPU rigs
   that WANT the work (the Max constituency) have no lane to earn. The work is
   compile-bound CPU — exactly what Max has and the worker lacks.

## The proposed design

### What already ships (consume, do not rebuild)

| Substrate | Where | Role here |
|---|---|---|
| `VerdictVector { compile_ok, oracle_resolved, heal_rate_milli, created_rate_milli }` + `commitment()` | `mining.rs` | The replay output + the agreement token (BLAKE3 over 18 canonical bytes) |
| `ProofVerifier` trait + `prove()` + `verify_no_verdict_split()` | `mining.rs` | The quorum seam; a `LeaseVerifier` adapter implements it over submitted verdicts |
| `ProofWorkItem { shape, fix, lint_key, …, submitter }` | `mining.rs` | The lease payload — replay inputs already the wire shape |
| `QuorumConfig::{corpus_version, model_version}` + `ProofVerdict::VersionSkew` | `mining.rs` | Version pinning BEFORE replay runs (G6 discipline — extended with a toolchain fingerprint, caveat 2) |
| `claim_weight(novelty, first)` → `RewardRow → settle_epoch → pay_due_rewards` | `rewards.rs`, `ledger.rs` | The settle path, reused verbatim for verifier credits |
| `usdc_kat_sub_split_bp` | `compose.rs` (P010) | The sub-split-of-a-parent precedent `replay_cut_bp` follows |

### The lease protocol (three load-bearing wire differences)

A verifier account requests a lease; the service assigns rows
**server-side** (a seeded shuffle, not client-chosen — the self-pairing
defense). Each leased item carries `{span_hash, fix_hash, shape, fix,
corpus_version, model_version, toolchain_fingerprint}` with THREE properties
the internal `/internal/plan` does NOT have:

1. **The claimed verdict vector is STRIPPED.** `C1BootstrapVerifier` consumes
   `obj.verdict_vector` (the submitter's claim); a third-party lease must not
   carry it — otherwise rubber-stamping is a copy-paste. The verifier
   COMPUTES the vector: apply `fix` to `shape` in a scratch cargo project,
   run the `fix_verify` compile gate + clippy oracle (both already in every
   healer binary).
2. **Submitter exclusion.** The service cannot lease a row to the account
   that submitted it (`ProofWorkItem.submitter` is on the row; the
   `submitter_for(key)` derivation is already the identity rule). Structural,
   not heuristic.
3. **Canary mixing.** At a secret rate ≤ 5% (the BOUND is public, the rate
   and positions are not), the lease includes a known-bad row — a fix whose
   true replay FAILS (`compile_ok = false` or the oracle does not resolve).
   A canary is indistinguishable from a real row without doing the work.

### Confirmation + the reward split (conservation by construction)

- A row is **replay-confirmed** when **K = 2** verdicts from distinct
  accounts (neither the submitter) agree on `VerdictVector.commitment()` —
  `verify_no_verdict_split` reused as the agreement check — AND the agreed
  vector passes the hard checks (the existing `Proven` arm; agreement on a
  failing vector is `Failed`, no credit — unchanged semantics).
- On credit, the row's weight `w = claim_weight(novelty, first)` splits:

```
miner    : RewardRow weight  w · (10_000 − replay_cut_bp) / 10_000
verifiers: one RewardRow each, weights summing to  w · replay_cut_bp / 10_000
                                                          (shared by the K agreeing)
TOTAL    : w   (unchanged — every other row's payout share is untouched)
```

  The verifier RewardRows ride the SAME `record`-keyed, epoch-frozen,
  exactly-once discipline (record id = `record_id(span_hash) ⊕ verifier
  account` — a distinct row key, the idempotency watermark pattern). `pay_due_rewards`
  is not modified: it already pays pro-rata over `weight_sum`, and the split
  keeps `weight_sum` invariant.
- **Canary economics** (TrueBit's forced error, adapted): a verifier that
  correctly FAILS a canary earns a jackpot multiplier on that item (e.g.
  ×`K`, exact constant sim-decided); one that stamps a canary as passing is
  **zeroed for the epoch** (all its replay RewardRows for that epoch drop to
  weight 0 — a weight slash, never a balance confiscation; Max has no
  principal at stake, its stake IS its accrued accuracy).

### Where the cut lives (minimal lever surface)

`replay_cut_bp` is a **`KatConfig` deployment-frozen constant, genesis `0`**
— the exact `reward_decay_bp` pre-v2 precedent. At 0 the split is the
identity (miner gets `w`, verifiers get nothing) and the lease routes can
refuse cheaply; the feature is structurally inert. Promoting it to a
governance lever (EpochParams `:v3` widening + a `LeverBound` row in
`governance.rs`) is DEFERRED until the Phase-3 sim proves the number — the
P008 versioned-bounded-movable law, entered through its re-open path.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **v1 trust is statistical, not Byzantine.** K=2 agreement + canaries +
   submitter exclusion bound rubber-stamping and lazy collusion, but an
   operator running two accounts does one unit of work for the full verifier
   cut (vs half each for an honest pair). Bounded by `replay_cut_bp` (≤ a few
   hundred bp of pool-class weight); server-assigned leases make same-row
   co-lease rare but not impossible. If Phase-3 sims price this hole above
   tolerance, the fix is lease-disjointness or K-randomization — designed
   hooks, not shipped v1.
2. **Replay determinism is toolchain-scoped.** `compile_ok`/`oracle_resolved`
   depend on rustc/clippy versions; verdicts from different toolchains are
   not comparable and will read as splits. The lease carries a
   `toolchain_fingerprint` (rustc `-vV` hash); mismatch is `VersionSkew`
   (existing semantics extended), never a slash. CAVEAT: the fingerprint
   pins the toolchain BUILD, not bit-identical behavior across platforms —
   cross-platform disagreement is an expected `Quarantined` mode until
   measured otherwise. G1 must include a same-toolchain-only agreement pin.
3. **The canary rate is secrecy-bounded, not secret-forever.** A patient
   adversary with sacrificial accounts can estimate the rate. The defense is
   that the canary only catches work-skipping — an adversary that DOES the
   work is indistinguishable from honest and earns honestly; the attack
   surface is exactly the work-avoidance the canary prices. Rate/BOUND are
   sim-decided in Phase 3, not baked here.
4. **Execution risk moves to volunteers — deliberately.** Leased `fix` text
   is untrusted code; each verifier runs it on their own rig inside the
   healer's existing scratch-project discipline. The service NEVER executes
   untrusted text in ANY phase of this proposal (the C1 mock does not
   execute; the C5 shape outsources execution). A future service-side audit
   replay lane needs the container story — deferred, named reopen.
5. **Do not advertise before it settles** (the P013 law, restated): the tier
   spec's Max row flips to "shipped" only after Phase 3 lands a live
   replay-settled reward on devnet. Design ≠ mechanism; a green G1 suite is
   not a payout.
6. **The `VerdictVector` rates are advisory fields.** `heal_rate_milli` /
   `created_rate_milli` ride the commitment but the hard checks are
   `compile ∧ oracle` (`passes()`); do not let the Phase-2 wire grow
   rate-gated rewards without their own gate (the score-bench freeze owns
   those numbers).

## Fusion lineage

1. **P005 §Mining C1→C5** — the escalation slot this fills; `QuorumConfig::c1`'s
   own doc names the destination.
2. **P010's sub-split + distribution-neutral discipline** — `replay_cut_bp`
   follows `usdc_kat_sub_split_bp`: carve from the parent slice, keep the
   burn partition untouched, conserve total weight.
3. **P013's corrections ledger** — the tier table that made this class
   explicit and law-bound ("cannot settle ⇒ cannot advertise").
4. **TrueBit's forced errors (1806.11476)** — the verifier-must-verify
   pricing problem and the inject-faults answer; our canary-zeroing is the
   no-slashing adaptation for a stakeless verifier class.

## GOAT gate

- **G1 (correctness, ledger)**: (a) conservation — for every confirmed row,
  `Σ weights(miner + verifiers) == claim_weight(novelty, first)`, pinned by
  test at cut ∈ {0, 100, 300, 1000} bp; (b) submitter exclusion refuses at
  lease; (c) canary stamp zeroes the epoch's replay rows, canary catch pays
  the jackpot; (d) `replay_cut_bp = 0` ⇒ byte-identical reward rows vs HEAD
  (the flag-off identity).
- **G2 (perf)**: verdict intake route µs-class per row (BLAKE3 + row writes,
  no compile anywhere near the worker); settle epoch time unchanged (same
  row count discipline).
- **G3 (no-regression)**: the entire existing mining/settle test corpus green
  untouched; `/sync` + prove-leg routes byte-identical.
- **G4 (alloc)**: new ledger ops follow the fixed-cap row discipline
  (`RewardRow` is fixed-size; no per-row heap growth beyond it).
- **G7-style econ arm (Phase 3, gates ANY activation ≠ 0)**: sweep
  `replay_cut_bp` × canary rate × adversary work-share — outputs: miner
  dilution curve, verifier break-even, canary catch-rate economics, the
  self-pairing hole priced. Owner ratification writes the devnet genesis
  value; mainnet keeps 0 until its own ceremony.

## What ships now (riir-dapps + riir-clippy) vs deferred

### Ships — Phase 1 (riir-dapps, inert at cut = 0)
- `replay_cut_bp: u32` on `KatConfig` (frozen, genesis 0).
- Lease assignment + verdict intake + agreement tracking + the conservative
  split + canary row type (server-side secret) + the `LeaseVerifier`
  `ProofVerifier` adapter — `mining.rs`/`ledger.rs` + account-authed routes
  (the `X-Kat-Account` session pattern, NOT the bearer-gated internal
  planes).
- The G1 contract tests (a)–(d).

### Ships — Phase 2 (riir-clippy)
- `cargo heal replay` — login-gated lease → local `fix_verify` + clippy
  oracle replay (scratch project) → verdict submit; toolchain fingerprint
  check; kill switch `RIIR_HEAL_REPLAY=0`.

### Deferred
- **Phase 3** — the econ-sim arm + owner ratification + devnet activation +
  the tier-spec status flip (Max row: designed → shipped) + P013 T3.1 close.
- **Service-side audit replay container lane** (Byzantine hardening; reopen
  with the P005 container story).
- **`replay_cut_bp` lever-ification** (EpochParams `:v3` + `LeverBound`) —
  only if governance ever moves it.
- **K-randomization / lease-disjointness** — designed hook, shipped only if
  the sim prices the self-pairing hole above tolerance.

### Explicitly NOT shipped
- No new chain tier, no stake requirement for verifiers (the class is
  deliberately stakeless — the work is the Sybil cost), no slashing of
  balances, no LLM anywhere in the replay path, no earnings language in any
  public surface before Phase 3 settles live.

## Phased rollout (sketch — a dapps plan owns Phase 1 tasks)

### Phase 1 — dapps ledger surface (inert)
- [ ] T1.1 `replay_cut_bp` on `KatConfig` (0) + config test
- [ ] T1.2 lease assignment (server-seeded shuffle, submitter exclusion) +
      `ProofWorkItem`-derived lease wire (claimed vector stripped)
- [ ] T1.3 canary row minting (secret rate ≤ public bound) + canary ledger
      class
- [ ] T1.4 verdict intake + agreement (`verify_no_verdict_split` reuse) +
      the conservative split + verifier `RewardRow`s (keyed idempotent)
- [ ] T1.5 `LeaseVerifier` adapter; C1 mock stays the cut=0 path
- [ ] T1.6 G1 (a)–(d) contract tests; G3 corpus green

### Phase 2 — the client (riir-clippy)
- [ ] T2.1 `cargo heal replay` subcommand (lease → scratch replay → verdict)
- [ ] T2.2 toolchain fingerprint + VersionSkew handling
- [ ] T2.3 devnet e2e: one leased batch, agreement, reward rows visible
- [ ] T2.4 docs: `.docs/` replay lane page + tier-spec status note

### Phase 3 — economics + activation (owner-gated value write)
- [ ] T3.1 econ-sim arm (cut × canary × adversary sweep)
- [ ] T3.2 owner ratification → devnet `replay_cut_bp` ≠ 0
- [ ] T3.3 tier-spec Max row flip + P013 T3.1 close (same commit)

## Risks

1. **Collusion/self-pairing** (caveat 1) — bounded by the cut; priced by the
   sim before activation; lease-disjointness is the designed fix.
2. **Toolchain skew reading as splits** (caveat 2) — fingerprint gate +
   `Quarantined`-not-slash semantics; measure cross-platform agreement in
   Phase 2 before K=2 goes live.
3. **Canary secrecy erosion** (caveat 3) — the bound is the contract; rate
   rotation per epoch is the cheap refresh.
4. **Spec-doc drift** — the Max row must stay "cannot settle" until Phase 3;
   the doc-sync sweep covers `node_tiers.md` (its standing risk row).
5. **Complexity creep toward a verifier marketplace** — out of scope by
   construction (no priors, no reputation NFTs, no stake); the weight IS the
   reputation.

## Out of scope

- The Pro fixer half (leak sign-off — separate gate, P013 Phase 2).
- T3.2 CLI tier presets (demand-gated; this adds `replay` as a MODE, not a
  preset).
- Any chain-side change (`Trainer(2)`/`Player(1)` untouched; verifiers sit at
  `Player(1)`, vote 0 — a ledger-account class, not a chain tier).
- dao involvement beyond reading the Phase-3 sim record (economics stays
  committed DATA until lever-ification is proposed).

## References

### In-house (grep-verified 2026-09-14)
1. `riir-dapps/src/kat/mining.rs` — `VerdictVector` (+`passes`, `canonical_bytes`,
   `commitment`), `ProofObject`, `QuorumConfig::c1`, `ProofVerifier`,
   `prove()`, `verify_no_verdict_split()`, `ProofVerdict`, `ProofWorkItem`,
   `ProofPlan`, `QueueRow`, `RowState`.
2. `riir-dapps/src/kat/rewards.rs` — `claim_weight` (3/1, ×3 first) + the
   ranking test.
3. `riir-dapps/src/kat/ledger.rs` — `record_proven_reward` → `RewardRow`;
   `settle_epoch`; `pay_due_rewards` (pool_room-bound).
4. `riir-dapps/src/kat/compose.rs` — `EpochParams` (v2 28 B row, armed
   6500/500/2000/1000), `usdc_kat_sub_split_bp` (the sub-split precedent).
5. `riir-dapps/src/kat/governance.rs` — `LeverBound` / `bounds_violation`
   (the deferred lever-ification path).
6. `riir-dapps/cloudflare/kat-service/src/prove.rs` — `C1BootstrapVerifier`
   (the documented mock), `mining_plan_route` / `mining_commit_route` (the
   internal plane API this proposal does NOT reuse for third parties).
7. `riir-dapps/tests/kat_econ_sim.rs` — the sim home the Phase-3 arm extends.

### External (cited-only)
8. [arXiv:1806.11476](https://arxiv.org/abs/1806.11476) — *A Predictable
   Incentive Mechanism for TrueBit* (Teutsch, Koch) — forced errors to
   incentivize verification; sybil discussion. The canary-jackpot is our
   adaptation; the on-chain bond/slashing design is NOT adopted (stakeless
   verifiers).
9. Storj/Akash operator-recruitment shapes — carried from P013's references
   (hardware-floor honesty, not mechanism).
10. EigenLayer AVS operator economics (restaked verification with slashing) —
    the contrast class: we deliberately do NOT stake-secure v1.

## TL;DR

GO — file this as the T3.1 design; open the riir-dapps Phase-1 plan when
construction is greenlit. Genesis `replay_cut_bp = 0` keeps everything inert
until the sim + owner ratification arm it; the tier table may say "designed"
the day Phase 1 lands, and "shipped" only after a live replay-settled reward
on devnet.
