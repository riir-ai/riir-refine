# Node Tiers — the install spec (one place to compare what you can run)

> **Purpose:** the consumer/dev/investor-facing tier spec for the KAT heal
> network — which install fits which rig, what each tier does, what it costs,
> what it can earn, and what is actually runnable **today** vs designed vs
> blocked. The canonical argument + corrections live in
> [Proposal 013](../../.proposals/013_consumer_install_tier_spec.md); this doc
> is the spec itself. The internal topology map (roles × chain tiers × hats)
> lives in [riir-ai `node_tiers.md`](../../../riir-ai/.docs/01_orientation/node_tiers.md)
> — this doc is its consumer-facing packaging layer, per
> [Proposal 005 §"Node-tier additions"](../../.proposals/005_kat_heal_service_network.md).
>
> **Why this exists (owner ask, 2026-09-12):** a dev or investor asking "what
> can I run on MY rig, and what does it get me?" had to assemble the answer
> from Proposal 005, the l4_fallback serving-lane decision, three benches, and
> two sibling-repo docs. ai.gist.rs needs a chooser; the game launch will need
> the same artifact shape. This is that one place.

## The rule: a tier is an install preset, not a new chain tier

**No new chain tiers** (the Proposal 005 discipline). A tier is a curated
bundle over five existing axes:

```
tier = (chain identity × runtime mode × LLM lane × hardware floor × earn class)
```

This mirrors the game-stack's 3-axis disambiguation (runtime role / chain
tier / runtime hat — see riir-ai `node_tiers.md` §TL;DR): the axes are what
the machinery knows; the tiers are what a human picks.

## The tier table

| | **Lite** | **Pro** | **Max** | **Ultra** |
|---|---|---|---|---|
| **What you run** | the `cargo heal` binary, anonymous | `cargo heal login` (account + miner mode) | + `kat_drain` contributor + quorum-replay participant | + full trainer window host (own Container/VPS cron) + vessel corpus host |
| **Chain identity** | none (no account) | `Player(1)`, stake 0, vote 0 | replay endorsement class *(designed — [Proposal 015](../../.proposals/015_replay_endorsement_reward_class.md); NOT YET SETTLEABLE)* | `Trainer(2)`, stake 1K |
| **LLM lane** | none — modelless only | API pay-per-fix *(gated — leak sign-off)* | API: groq free leg → CF cap-guarded | API-first (groq → CF); local Bonsai **optional + UNARMED** (reopen bar) |
| **Hardware floor** | anything that runs the binary (native desktop + musl Linux) | any desktop | CPU box that fits rustc compile windows (~2–4 vCPU / 4–8 GB; **no GPU**) | GPU rig OR container-capable VPS (`standard-1`-class; R2 cache tier) |
| **Costs** | nothing — free forever | KAT burn (1 KAT / M code word tokens; 100 KAT login grant) | API costs ride the groq free leg / CF USD cap | stake + GPU/container cost (v1: fleet-funded from USD treasury, G7) |
| **Earns** | nothing (outside the economy) | mining: synced data → quorum-proven → KAT | replay/verify rewards *(class NOT YET DESIGNED)* + mining | trainer endorsements + epoch pool share |
| **Game-tier analog** | Spectator / Satellite | Player | Curator(3) — verify work, CPU-only | Authority — the load-bearing node |
| **Status today** | ✅ **shipped** | ✅ core shipped (login/meter/sync); fixer half **BLOCKED** (leak) | 🟡 drain bin shipped (`kat_drain`); replay class **designed ([P015](../../.proposals/015_replay_endorsement_reward_class.md)) — settlement pending its Phase 3** | 🟡 designed (P005); launch = **OUR replicas only** (decentralization phase-gated) |

## Tier detail

### Lite — the anonymous binary

Today's bare `cargo heal`. Modelless healing (L0–L3), offline, no account,
frozen corpus, free forever — the honesty dividend. Outside the economy: no
KAT, no earnings, no hot-push. The planned witness upgrade (client-side
verification of chain-anchored ledger roots — a light-client read path, vote
weight 0) is the Satellite pattern; it adds verification, **not consensus**.

### Pro — account + miner (+ fixer when unblocked)

`cargo heal login` + sync-as-mining: heal locally → sync redacted trajectory
batches → quorum-proven novelty mints KAT from the epoch pool. Burns KAT per
code word token; the 100 KAT login grant ≈ 100M tokens of buying power.
The **fixer half** (consumer pays KAT → live span → fixed via API at LLM-token
rates) stays **BLOCKED on the leak-posture sign-off** — its quality gate
already lifted (Bench 049: verify-filtered acceptance 5–6/9 vs decline 0/9).
"Fix nolimit" is false at every tier: the meter, rate limits, and the USD-cap
discipline bind everything.

### Max — the no-GPU contributor

Two halves:
1. **`kat_drain` contributor** — 🟡 shipped (Issue 051): the groq-first
   `LlmRouter` drain over the unproven queue, redaction-gated, USD-capped.
   Anyone with API keys can run windows; free-tier absorption measured at
   $0.0000 metered.
2. **Quorum-replay participant** — 🟡 **designed 2026-09-14 ([Proposal 015](../../.proposals/015_replay_endorsement_reward_class.md))**: leased third-party replays (`fix_verify` + clippy oracle on the verifier's own rig), K=2 verdict-commitment agreement (`verify_no_verdict_split` reuse), a conservative per-row reward split (`replay_cut_bp`, genesis 0 = inert), and a TrueBit-adapted canary discipline against rubber-stamping — the C1→C5 escalation P005 §Mining named, with economics. **Still cannot settle** until P015 Phase 3 (econ-sim arm + owner ratification + a live devnet replay-settled reward): **do not advertise replay earnings until it can settle** stands.

### Ultra — the trainer node

The P005 trainer plane: daily trainer window host (deploy-once CF Container,
cron-start, sleep-after — billing = run window only), vessel discipline
(corpus packs as `NeuronVessel`s; fuel-gated WASM for third-party-derived
code), quorum participant, chain `Trainer(2)` stake 1K. LLM lane is
**API-first** (groq free leg → CF fallback, cap-guarded). Local Bonsai-27B is
**optional and UNARMED**: measured quality-FAIL (Bench 039: WIN 5 / REGRESSION
4, 2 silently wrong), latency-FAIL (~191 s/fix, Bench 463), RLVR G1 0/60 twice
(Benches 465/467) — it reopens only per the
[`l4_fallback.md` §reopen bar](../10_self_evolve/l4_fallback.md). **v1
decentralization honesty (P005 caveat #10):** at launch the trainers are OUR
replicas; "Ultra" as a third-party-runnable install is phase-gated behind the
vessel/stake/endorsement machinery maturing. Say "our trainers" until then.

## The corrections ledger (what NOT to spec — measured)

| Rejected spec | Why | Evidence |
|---|---|---|
| "own LLM > 27B local" as a tier centerpiece | Measured quality + latency FAIL; demoted with a reopen bar | Benches 039 / 463 / 465 / 467; `l4_fallback.md` §5 |
| "local ternary or LLM < 8B" as a Max lane | Ternary = the modelless L1 drafter (every tier has it, it is not an LLM); **no <8B model was ever benched** — the one mid-size model measured (qwen3.8-27b) failed economics (thinking tokens) | Bench 049 |
| "fix 80% / consensus 20%" as a tier dial | The 80/20 already ships as the **modelless-first pipeline**: 96.2% modelless heal rate; the L4 reachability contract routes ONLY misses (9/29 addressable) to any LLM. Consensus is agreement (deterministic replay), never a workload share | score-bench; Bench 037; `l4_fallback.md` §2 |
| "consensus nolimit / lite consensus 100%" | Consensus participation is staked + all-or-nothing per record; a lite node **witnesses/verifies roots** (vote 0), it never confirms | riir-ai `node_tiers.md` System 2/3 |
| "fix nolimit" | Meter + USD cap + rate limits bind every tier; the never-wrong verify contract holds everywhere | P005 window gates; `llm_spend.rs` |

## Consumer mirror (ai.gist.rs) — deferred

The chooser page on ai.gist.rs (rig → tier → install command) is
**capabilities-only** copy: hardware floors, what runs where, honest status.
No earnings promises beyond settleable classes; no local-LLM claims; the
leak-gated lanes stay unadvertised until their sign-offs land. Ships with the
Proposal-013 Phase 2 dapps plan. The public dist README
(`gist-rs/cargo-heal`) carries the same table, source-free per Proposal 003.

**Game-launch reuse:** when the game ships its node choice, this same
artifact shape (preset table over role/identity/hardware axes + honest status
column) is the pattern — noted, out of scope here.

## See also

- [Proposal 013](../../.proposals/013_consumer_install_tier_spec.md) — the argument, corrections, caveats
- [Proposal 015](../../.proposals/015_replay_endorsement_reward_class.md) — the Max replay earn class (designed; settlement pending Phase 3)
- [Proposal 005 §"Node-tier additions"](../../.proposals/005_kat_heal_service_network.md) — the role/hat placement rows this packages
- [riir-ai `node_tiers.md`](../../../riir-ai/.docs/01_orientation/node_tiers.md) — the internal 4-system topology map
- [`../10_self_evolve/l4_fallback.md`](../10_self_evolve/l4_fallback.md) — the LLM serving-lane decision + reopen bar
- [Bench 049](../../.benchmarks/049_fixer_regate_poc.md) — the fixer quality re-gate
