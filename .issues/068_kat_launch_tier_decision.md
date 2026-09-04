# Issue 068 — OWNER DECISION: the KAT launch tiers (settles 059 + 060, restructures 061)

Status: **DECIDED (2026-09-04) — this is a decision RECORD, not a work item.**
It resolves every open row in [Issue 059](059_proposal_007_remainder_ops_signer_and_counsel_gate.md)
and [Issue 060](060_proposal_008_remainder_new_plan_and_gated_levers.md) (both now
RESOLVED in place, kept as records because ~30 files across five repos cite them by
number), and re-tiers the open rows in
[Issue 061](061_proposal_011_product_loop_rollout.md), which stays the live product
tracker. Owner asked for a decision on 2026-09-04 after all three trackers had sat in
"no engineering unblock exists" for two days.
Branch: `develop` (per global rule — no feature branches)
Proposals: [007](../.proposals/007_kat_usdc_staking.md),
[008](../.proposals/008_adaptive_tokenomics_riir_dao.md),
[011](../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md),
posture home [005 caveat #7](../.proposals/005_kat_heal_service_network.md)

## The decision in one paragraph

Three trackers all read "blocked on an owner act, ops creds, or counsel", and read
together the blockage is **one mis-scoping, not three shortages**. The counsel gate and
the ops gate both belong to the **USDC custody rail**; the ops signer exists only to send
USDC refunds, and the counsel question is only sharp once real users' dollars or freely
transferable credits are involved. Nothing on the critical path needs either. The surface
that is actually next — a **testnet** closed loop between **operator-owned accounts**,
paid in non-transferable service credit, with no fiat anywhere — sits inside Proposal 005
caveat #7 **unchanged** and is gated by neither. So: split the rollout into three tiers,
authorize Tier 0 now, and stop holding engineering behind gates that belong to Tier 2.

## The tiers

| Tier | Surface | Gate | Verdict |
|---|---|---|---|
| **0** | devnet + **testnet** only; **operator-owned accounts only**; faucet/grant-funded KAT service credit; no fiat, no custody, no non-operator transfer | **none** — inside 005 caveat #7 unchanged (test credit, no independent counterparty, no value substituting for currency) | **AUTHORIZED NOW** |
| **1** | KAT rail open to **non-operator** accounts, and/or the operator loop on **mainnet** — a transferability change | ONE narrow counsel question (below) **+** 005 caveat #7's Phase-1 exit list: ToS w/ data-license grant, privacy policy, sanctions screening, tax handling, refund policy | **NOT PROPOSED** — needs a Tier-0 demand signal first |
| **2** | live **USDC custody** (staking deposits + outbound refunds), live Stripe | counsel (custody + incentives) **AND** measured demand | **NOT PROPOSED** |

**Tier 0 is deliberately narrower than Proposal 011's own framing.** 011 caveat #1 called
the initial surface "the closed-loop bar→foo operator network" without naming a network;
this decision pins Tier 0 to devnet/testnet so the posture question cannot even be asked
of it. A **mainnet** operator loop is Tier 1, not Tier 0 — the author of this record is
not counsel, and the honest move is to put the ambiguous case behind the gate rather than
to argue it out of one.

**The narrow Tier-1 counsel question, for when it is asked:** *"May a non-transferable,
non-redeemable prepaid service credit move between two independent account holders inside
our own closed loop without reclassifying the product?"* That is one question about one
rail. It is **not** the review Issue 059 was holding — that one bundled USDC custody,
staking incentives, and the transfer rail into a single engagement (007 caveat #1's
"shared envelope"). Bundling made the cheap question wait on the expensive one.

## Decisions

### D1 — Ship the KAT-stake arm; do not propose the USDC arm. (settles 059 T3.1)
007 was designed with two arms and a counsel gate to pick between them. **Pick the
KAT-stake arm now.** It ships under the *unchanged* 005 posture (no custody, no outbound
transfers, no new legal surface), it is **already live** (`staking_share_bp` 500 with
`yield_ceiling_bp` 22 armed in `EpochParams::DEFAULT`, ordering enforced structurally at
`riir-dapps/src/kat/ledger.rs:1350`), and choosing it costs nothing because it is the arm
that already runs. The USDC arm is not dropped — it is moved to Tier 2 and **not put in
front of counsel until there is demand for it**. Paying for a review of a rail we have
just decided not to operate is the wrong order.
**Re-open trigger:** a Tier-2 demand signal (a real inbound USDC deposit intent, or an
operator ask for fiat-in), at which point the USDC arm and the transfer rail go to counsel
*together* — the shared envelope 007 caveat #1 describes is correct, it was just opened
too early.

### D2 — Do not build the ops signer. (settles 059 T2.2)
The outbound USDC signer serves exactly one purpose: USDC unstake refunds, which exist
only in Tier 2. It is also the **highest-blast-radius component in the system** (a custody
key with send authority). Building it now means standing up key custody, an RPC
dependency, and a daily-drain driver for zero users, and guessing at the connection
pooling / alerting shape that the issue correctly says the ops answers determine.
**The two "missing" ops decisions are therefore not missing — they are premature.** The
right answer to "which RPC provider?" for a rail we are not running is "do not choose one
yet."
**Re-open trigger:** Tier 2 authorized (D1's trigger fires). Re-open as its own plan then,
with real load numbers instead of estimates.

### D3 — The copy review does not need counsel. (settles 059 T3.2)
T3.2 was riding T3.1 because the honest-disclosure wording depended on which arm shipped.
D1 picks the arm, and the KAT-stake arm has **no escrow window, no venting slices, and no
refund-to-source story to disclose** — there is no custody, so there is nothing to be
honest about beyond what already ships. What remains is mechanical and already partly
enforced: no `$` beside a KAT figure, service-credit labeling (005 caveat #15, test-pinned
since 007 T2.1). **Residual engineering, not a gate:** extend that pinned copy set to the
KAT-stake surface — the per-tranche 7-day lockup clock and the no-promised-rate line.
Acceptance: the existing copy-rule test covers the stake/unstake strings, canaried by
reintroducing a `$`-adjacent rendering. Small; anyone can pick it up.

### D4 — Advisory-only is 008's shipped end state. Do not commission the v2 plan. (settles 060, the NEW-plan row)
The G5 verdict is **FAIL (wash)** — verified at source this date, not read off a status
line: `riir-dao/.benchmarks/001_g5_simulator.md` reads *"FAIL — no metric where adaptive
strictly beats frozen (wash)"* with a regression pin asserting the verdict stays exactly
that. Bench 004 axis-E adds that no agent-reachable `decay_factor` keeps mining alive to a
1000-epoch horizon at launch scale under any cap. Commissioning a v2-candidates campaign
today buys a re-measurement whose only evidenced outcome class is another wash, and it
would need an owner **supply-ratio** decision made on no data.
**So advisory-only is not a waiting room — it is the answer.** The system landed, it is
bounded, it is audited (`dao-audit`), and a frozen constant performs as well as anything
adaptive we have tried. That is a recorded result, not a shortfall.
**Re-open trigger — let the data commission the plan, not the calendar:** the Tier-0 soak
(D7) shows a metric where a frozen constant is measurably leaving value on the table.
Absent that, 008 stays closed.

### D5 — Kill the dopamine levers. (settles 060 T3.1 + T3.2)
`FairRoll` jackpot tables and streak bonuses were gated on G5-class evidence that failed.
They are now **WON'T-DO for 008**, not deferred. Two reasons, both concrete:
- The guard layer already pins `REWARD_EV_EPSILON_BP = 0` — "EV ≤ base exactly", the
  strictest reading (`riir-dao/crates/riir-dao/src/guard.rs`, asserted in
  `tests/dapps_contract_pins.rs:177`). A jackpot table with **zero EV edge** is a
  presentation layer over a constant. It has no economic function by construction.
- It carries a real cost against D1: gambling-adjacent mechanics and copy, on a token we
  have just decided to keep non-transferable and closed-loop, add exposure to exactly the
  surface 005 caveat #15 says is "most likely to be screenshotted".
The guard-side caps stay (they bound anything future), the levers do not get wired.
**Re-open trigger:** only inside a future proposal that first shows a retention metric the
levers are supposed to move, measured, with the copy posture re-checked.

### D6 — The autonomy ramp stays foreclosed. (settles 060 T4.1/T4.2)
Already terminal-as-specified; recorded here so it is closed rather than perpetually
"open, gated". Re-opens only inside a future proposal, never inside 008.

### D7 — Insert a **devnet rung** before the testnet ceremony. (restructures 061 T2.2/T2.3, T4.4)
**This is the engineering unblock the three trackers said did not exist**, and it is the
highest-leverage row in the whole set.

061 T2.2 is an owner act because it authors real testnet key material and writes to nodes
as root (caveat #6: wrong root key = locked treasury; double mint = supply bug). All true
— *of testnet*. **Devnet is none of those things**: ephemeral keys, a local loopback
cluster, no production hosts. `examples/genesis-devnet.yaml` already exists and G-DEPLOY
already ran against three real `mock-chain` processes on loopback
(riir-deployer `96009cc`, Bench 003). The only thing standing between us and an executed
ceremony is that `genesis --execute` refuses **unconditionally**, naming T2.2 — a refusal
written for the testnet/mainnet hazard that also catches the case carrying none of it.

**Decision: allow `genesis --execute` for `--dest devnet` only**; testnet and mainnet stay
refused exactly as today. Run the four ceremony steps end to end on devnet, produce
`devnet.toml`, then run T4.4's shipped wet-run harness against it.

What this buys, in order of value:
1. **The owner's one-shot testnet ceremony stops being one-shot.** Caveat #6's failure
   modes are unrecoverable *because the ceremony has never been executed*. A devnet
   rehearsal turns the owner act from a design-and-execute job into a
   run-the-rehearsed-script job.
2. **It produces the live telemetry D8 needs**, which is the only thing 060 T2.3 was
   waiting for.
3. **It exercises T4.4's harness for real.** The wet-run's six steps, G-WET's
   bit-unchanged supply assertion, and the crank's treasury-debit path have been proven
   against a *scripted* service. Devnet runs them against an actual one.

**Guardrails — these are load-bearing, not garnish.** The devnet arm must not become the
override the mainnet refusal exists to prevent (011 caveat #2: an override env var is a
standing invitation):
- The permit is on the **destination name**, matched exactly, in the same shape T3.1
  already uses for `RIIR_DEPLOY_ALLOW_MAINNET` — *not* a truthy env var, and not a
  `--force` flag.
- `testnet` and `mainnet` keep the current unconditional refusal, pinned by a test that
  fails if either becomes executable.
- The devnet ceremony must refuse to run against any host not on loopback/private
  address space — fail-closed on an unclassifiable host, the rule
  `kat_wetrun.rs` already applies.
- `wetrun`'s mainnet refusal is **untouched**. It reads no environment and must keep
  reading none.
**Acceptance:** `devnet.toml` authored by the ceremony (not by hand); the four steps
reported; the wet-run's six steps green against the devnet cluster with G-WET's supply
counters bit-unchanged; the testnet/mainnet refusal tests still red when broken.
**Owner: nothing to do until the devnet report is clean.** Then the testnet ceremony
(T2.2/T2.3) runs from the rehearsed runbook.

### D8 — The advisory soak runs on devnet. (settles 060 T2.3)
060 T2.3 was blocked on live telemetry, which was blocked on the genesis ceremony, which
was an owner act. D7 breaks that chain: the devnet cluster emits real epochs. Run the
advisory-mode soak there. A devnet soak is not a substitute for a testnet one for
*launch* purposes, but it is a full substitute for the purpose 060 stated — proving the
advisory loop runs N epochs clean and the guard layer does not trip on real telemetry.
**Row moves to 061** and 060 closes.

### D9 — Create `gist-rs/cargo-heal`. **OWNER ACT — the only one left that blocks a shipped surface.**
Re-verified 2026-09-04: `gh repo view gist-rs/cargo-heal` → *"Could not resolve to a
Repository"*. The repo has never existed. **Both install one-liners are therefore
unrunnable, which means nobody can install `cargo-heal` at all** — so the entire
onboarding loop is unreachable no matter how the genesis ceremony goes. This is a smaller
act than the ceremony and it gates more.
Decision: **create it public, binaries only** — Proposal 003 stands unchanged (riir-clippy
source stays private; releases are the only public surface). The install-copy pin
(`tests/install_copy_parity.rs`) already fixes the canonical URL pair, so the repo has a
spec to satisfy the moment it exists.
Not executed here: creating a public repo under the owner's org is outward-facing and is
the owner's call to make, not an agent's inference from "make the best decision". The
command is one line and the owner is authenticated:
`gh repo create gist-rs/cargo-heal --public --description "cargo-heal — binary releases"`.
Say the word and it is done.

## Disposition of the deferred list (061)

Every row now carries a verdict instead of an open bracket:

| Row | Verdict |
|---|---|
| Chain-NFT invoice mint | **WON'T-DO** — a per-payment NFT fails 011's own rate test. Re-open only as Glacial **batch** anchoring, if invoice volume ever asks for it. |
| MemoShard v4 signing-message adoption | **DEFER** (unchanged) — rides riir-chain Proposal 008's decision; not ours. |
| LIVE Stripe + Solana-Pay rails | **Tier 2** (D1). Mock-tested; stays that way. |
| KAT rail beyond operator-owned accounts | **Tier 1** (D1). |
| `riir-wallet-wasm` invoice display | **DEFER** (unchanged) — CLI view ships first, browser wallet later. |
| Discord faucet bot | **WON'T-DO now** — link-first already made it pure automation with no bot needed. Re-open on volume. |
| Baz fixer tier | **Gate RE-SCOPED — (c) collapses.** Its three gates were (a) a fixer arm beats `decline` on the reachable set, (b) leak-posture sign-off, (c) reward economics with **G5-class evidence**. D4 closes 008 as terminal, so (c) as written is permanently unsatisfiable — and it was **mis-specified**: a fixer tier needs a reward *constant*, not an *adaptive* one, and 008's measured verdict is precisely that a frozen constant performs as well as anything adaptive. So (c) is discharged by picking a bounded constant inside the existing guard caps. **The gate is now (a) + (b).** Still behind clippy-tier onboarding per 011 §9 — the least-proven earning lane does not go in front of the newest users. |

## What is actually open after this record

1. **D7** — the devnet ceremony rung (riir-deployer). Engineering, unblocked, ready to plan.
2. **D8** — the advisory soak, once D7's cluster is up. Rides D7.
3. ~~**D3** — extend the pinned copy set to the KAT-stake surface. Small, unblocked.~~
   **DONE 2026-09-05 (this commit):** the gate predicate is now ONE shared fn
   (`kat_stake_client::caveat15_violation`) driven by the module tests AND the
   integration binary (the two hand-copied loops it replaces could drift), the
   required-disclosure checklist (`caveat15_missing_disclosure`: 7-day lockup
   clock, no-promised-rate line, service credit, rate-floats) pins the stake
   copy set, and the D3 acceptance canaries landed — a `$`-adjacent accrual
   render and a promised-rate (APY-outside-no-promise) render both FIRE the
   gate (a clean-only gate is a green zero). 21/21 module + 9/9 integration
   under `kat_transport`, clippy 0.
4. **D9** — create `gist-rs/cargo-heal`. **Owner act, one command, awaiting go-ahead.**
5. **T2.2/T2.3/T4.4 on testnet** — owner act, but only *after* D7's rehearsal reports clean.

Everything else in 059, 060 and 061 is now decided, closed, or explicitly tiered with a
named re-open trigger. **No row anywhere in the three trackers is "waiting" without a
trigger that says what would end the wait.**

## Honest caveats on this decision

1. **The author is not counsel.** Nothing here is a legal opinion. What is decided is
   *product scope*: which surfaces we operate. Tier 0 is chosen because it needs no new
   legal posture — not because a lawyer said the ambiguous cases are fine. The ambiguous
   case (a mainnet operator loop) is deliberately placed in Tier 1 rather than argued into
   Tier 0.
2. **D4 records a negative result as an end state.** If someone later reads "advisory-only"
   as unfinished work, they will re-run a campaign we already paid for. The re-open trigger
   is a *measurement*, and it is the only thing that should reopen it.
3. **D7 relaxes a refusal, and refusals are load-bearing here.** The guardrails in D7 are
   the decision, not a suggestion — a devnet arm that grows into a general override would
   undo 011 caveat #2 and T4.2's no-override property, which were both expensive to
   establish and are currently proven by test.
4. **D2 accepts a known cost.** Not building the signer means that if Tier 2 is ever
   authorized, the longest-lead item starts from zero. That is the correct trade at zero
   users, and it is recorded here so the lead time is not a surprise later.
5. **Tier 0 on devnet only is a rung, not a destination.** It proves the loop; it does not
   prove the product. The demand signal that opens Tier 1 has to come from somewhere, and
   Tier 0's operator loop is the only place it can — which is a real limitation of this
   sequencing, stated rather than hidden.
