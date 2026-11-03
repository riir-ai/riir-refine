# Plan 334 — P015 Phase 3: replay settlement econ-sim arm + ratification runway (the Max tier's earn story)

**Status:** CLOSED 2026-11-03 (second session) — **DUPLICATE of riir-clippy [Plan 128](128_p015_phase3_replay_econ_sim.md)**, which is COMPLETE and its ratification CONSUMED (2026-09-15, owner delegation: `replay_cut_bp` STAYS 0). No task here was executed; none is owed. Premise defect recorded below; disposition: CLOSED-DUPLICATE.

## Premise correction (2026-11-03, second session — the plan as allocated was wrong)

This plan was allocated on the premise that P015 Phase 3's econ-sim arm was open. **It is not.** riir-clippy [Plan 128](128_p015_phase3_replay_econ_sim.md) ran the full sim (riir-dapps `tests/kat_econ_sim.rs` §`replay_econ`, G1-pinned against the real `record_proven_reward_with_replay` ledger math — no parallel money math), recorded [riir-dapps Bench 013](../../riir-dapps/.benchmarks/013_replay_econ_sim.md) (T1/T2/T4/T5/T6 + Addendum 1 the canary-bound conflict + Addendum 2 the plan-045 C4 re-run byte-identical + **verdict AGREE round 2 closing**), and the ratification ALREADY HAPPENED — 2026-09-15 under owner delegation ("make best decision for prod grade"): **`replay_cut_bp` STAYS 0** — the recording lane is live and free; paying at the current pool (~$3e-6/item, later decay-bound to ~0 µKAT/weight) buys no economic behavior.

Map of the duplication: this plan's T1 = Plan 128 T1/T2 (the sweep ran, 48 cells, 40 pass, the 2%-canary ≥30%-share failures); this plan's T1b = Plan 128 T5 (= Bench 013 §T5, whose recommendation is KEEP 0); this plan's T2 was already consumed (the keep-0 ratification IS the T3.2 outcome); this plan's T3's flips are "NOT OWED at cut 0" per Plan 128's close. The grounding failure: allocation grepped P015 + `replay_lane.md` but NOT the parent Plan 128 or riir-dapps `.benchmarks/` — the numbering gates catch number collisions, not semantic duplicates. The verdict round that approved opening this plan was given the same incomplete framing.

### What actually stands (the real Phase-3 runway — all recorded, none of it this plan's work)

- **The arming interlock ships in code**: `REPLAY_CANARY_RATE_BOUND_PERMYRIAD` (50‰) + `REPLAY_CUT_ARMING_MIN_BOUND_PERMYRIAD` + the compile-time one-act `const` pin + `arming_interlock_check` (`riir-dapps/src/kat/replay.rs`; Bench 013 Addendum 1's bound+rate coupling, mechanized — arming under the bound is refused at both compile time and config validation).
- **The arming preconditions** (pool scale ~10⁵×, 10% canary, the §T5 monitoring counters, cut ≥ 2 223 bp) live in Plan 128's close + Bench 013 F6 + Addendum 2 (devnet per-weight decay-bound to ~0 µKAT/weight — arming today pays verifiers ~nothing). dapps `.issues/091` (Plan 128's cited home for them) is closed+removed per the noise rule — record in git history.
- **The Mozilla W3 pilot's earn story is carried by vouchers/service capacity** (Proposal 025 W3 verbatim: "pilot users never pay and never need to contribute" — funded as service capacity + expiring pilot vouchers), NOT by replay settlement. Bench 013's keep-0 verdict stands; the tier-spec Max row's "designed — does not settle" wording stays TRUE.

---

## The original (pre-correction) body — kept as the record of what was allocated

**Status (superseded):** ACTIVE 2026-11-03 — opened on the Plan 333 OwnerGate approval (Claude verdict round 1: "APPROVE opening it as its own plan now"); Phase A (T1–T1b) agent-executable; Phase B owner-gated; spec of record: [Proposal 015](../.proposals/015_replay_endorsement_reward_class.md) Phase 3 (DATED HOLD 2026-09-28 at `replay_cut_bp = 0` — basis riir-dapps Bench 013: a cut < 2,223 bp pays verifiers nothing; arming = one devnet act at cut ≥ 2,223 bp)

## Why now

The Mozilla pilot (Proposal 025 W3, Q1–Q2: 30–50 external devs, "pilot users never pay and never need to contribute") makes Max tier's replay lane the only external earn path that can settle in Q1 — and it cannot settle: the lane is client-shipped + server-shipped but **two-key inert** (genesis 0 on the config AND no deployment env). Phase 3 is the econ proof + one devnet act. Plan 333's [Phase C table](333_p005_fixer_leak_signoff_and_all_tier_prep.md) names this the largest "designed-but-does-not-settle" gap in the tier table.

## Grounded state (read this session, not assumed)

- **Phase 1 COMPLETE (riir-dapps, inert at cut = 0):** `replay_cut_bp: u32` frozen genesis 0 on `KatConfig`; server-seeded lease assignment (submitter exclusion); canary row minting (secret rate ≤ public bound); verdict intake + K=2 agreement (`verify_no_verdict_split` reuse) + the conservative split + keyed-idempotent verifier `RewardRow`s; the account-authed `/mining/replay/*` surface (Plan 029 T1.4) with the wire canonical in riir-kat `kat_protocol_replay`; **G2 measured 58 µs/verdict release**.
- **Phase 2 COMPLETE (client, Plan 127):** the replay subcommand (historical spelling `cargo heal replay`, now `cargo refine replay`) — step-0 canary hash check, the real clippy oracle on the verifier's own rig (needs `--features kat_transport,clippy_verify` — the release set carries no oracle), the pinned single-item rate rule, toolchain-fingerprint skew refusal (`Quarantined`-not-slash semantics), kill switch. T2.3's armed-lane leg deliberately deferred: **reward rows are never written at cut=0**, so "reward rows visible" is a Phase-3 question by construction.
- **Naming collision note:** P015's Phase-3 task ids (T3.1–T3.3) share the "T3.2" string with Proposal 005's leak sign-off gate — in this plan they are always written "P015 T3.x".

## Tasks

### Phase A — the econ proof (agent-executable)

- [ ] **T1 — the econ-sim arm (P015 T3.1)**: sweep `replay_cut_bp` × canary rate × adversary work-share; outputs pinned by P015's G7-style row: **miner dilution curve, verifier break-even, canary catch-rate economics, the self-pairing hole priced**. The Bench-013 2,223 bp break-even is a CONSTRAINT on the grid (the floor), not an output to re-derive. Home: the deterministic adversarial sim posture (the riir-dapps G7 / riir-dao G5 precedents) against the dapps `KatConfig` row; record = a `.benchmarks/` sweep doc. GOAT bar: same seed → bit-identical outputs, and the sweep's break-even row reproduces Bench 013's 2,223 bp at its own posture before any recommendation is read off the grid.
- [ ] **T1b — the ratification memo** (one page, for T2): the recommended cut + canary rate from the sweep; the self-pairing residual and the lease-disjointness/K-randomization trigger check (P015 ships those ONLY if the sim prices the hole above tolerance — the memo states which side of tolerance it landed on); the exact form of the devnet act.

### Phase B — activation (owner; `- [-]` until ratified)

- [-] **T2 — owner ratification → devnet `replay_cut_bp` ≠ 0** (P015 T3.2): the one devnet act at the ratified cut. Mainnet keeps 0 until its own ceremony.
- [-] **T3 — the same-commit flips (P015 T3.3)**: tier-spec Max row designed → shipped; P013 T3.1 close; the "do not advertise replay earnings until it can settle" law lifts in `node_tiers.md` (both surfaces) + the consumer mirror.

### Phase C — post-arming evidence (agent)

- [ ] **T4 — the deferred P015 T2.3 leg, now executable**: one leased devnet batch e2e — lease → scratch replay → verdict → agreement → **reward rows visible**; bench record.
- [ ] **T5 — doc-sync**: `replay_lane.md` + both `node_tiers.md` surfaces + the Mozilla evidence pack gain the settled number (the G3 evidence-pack row for Proposal 025).

## Non-goals

- No mainnet arming (own ceremony, owner).
- No verifier-marketplace creep — P015's explicit NOT-shipped list holds (no priors, no reputation NFTs, no stake; the work is the Sybil cost).
- No K-randomization / lease-disjointness ship unless the sim prices the self-pairing hole above tolerance.

## Related

[Proposal 015](../.proposals/015_replay_endorsement_reward_class.md) (the spec) · [Plan 127](127_p015_phase2_replay_client.md) (Phase 2 client) · riir-dapps Bench 013 (the 2,223 bp basis) · [Plan 333](333_p005_fixer_leak_signoff_and_all_tier_prep.md) T9 (the approval) · Proposal 025 W3 (the pilot this serves) · `../.docs/01_orientation/replay_lane.md`
