# Proposal 013 — Consumer-facing install tier spec (Ultra / Max / Pro / Lite)

Status: **shipped Phase 1 (spec doc + corrections ledger, `c246e61f`, 2026-09-12); Phase 2 COMPLETE 2026-09-15 (T2.1 the ai.gist.rs chooser — moved 2026-09-15 to the FRONT page as the tiers matrix + role-tabbed Get started, riir-dapps `0553a7f`, leaderboard stats-only, live on all three envs; T2.2 the public README tier section, cargo-heal `62a435f`); Phase 3 replay reward class designed — [Proposal 015](015_replay_endorsement_reward_class.md)**, Phase-1 library slice landed (riir-dapps `00b6583`); Phase 4 game-launch)
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 005 §"Node-tier additions" × riir-ai `node_tiers.md` (the 4-system topology map) × `l4_fallback.md` §5 (the serving-lane decision) × Issue 051 `LlmRouter` (the groq-first API lane)
Related: [Proposal 003](003_binary_only_distribution.md) (the install surface), [Proposal 011](011_kat_pay_request_qr_genesis_and_onboarding.md) (onboarding), [Bench 049](../.benchmarks/049_fixer_regate_poc.md), [the spec doc](../.docs/01_orientation/node_tiers.md)

## TL;DR

Package the heal network's existing roles/hats/lanes as **four consumer-facing
install tiers — Ultra / Max / Pro / Lite** — speced in ONE canonical doc
([`.docs/01_orientation/node_tiers.md`](../.docs/01_orientation/node_tiers.md))
so a dev, consumer, or investor can answer "what can I run on my rig, and what
does it get me?" without assembling five documents. The owner's 4-tier shape
is adopted; four of its load-bearing details are corrected against measured
verdicts (local >27B LLM, the 80/20 split, <8B local models, "unlimited"
anything). A tier is an **install preset over five existing axes** (chain
identity × runtime mode × LLM lane × hardware floor × earn class) — **no new
chain tiers** (the Proposal 005 discipline). This is our packaging of our own
measured machinery, not a distillation of external prior art; the closest
external pattern is Storj/Akash-style operator recruitment pages (hardware
floor + honest earnings), cited-only.

## The problem this solves

Proposal 005 §"Node-tier additions" placed the KAT rows into the INTERNAL
topology map (`Healer client`, `heal-proof hat`, the worker surfaces) — for
machinery owners. But there is no **consumer-facing artifact**: an investor
with a 4090 rig, a dev with a laptop, and a CI box have no single table to
compare installs against. ai.gist.rs (the service's public front) has no
chooser; `gist-rs/cargo-heal` README has no tier section; the game launch
will need the identical artifact for its node choice. Worse, without a
corrections ledger, the natural tier spec an eager session would write
(local big LLMs! percentage splits! unlimited fixing!) re-litigates measured
negatives (Benches 039/463/465/467/049) every time.

## The proposed design

A tier = **install preset** over five axes the machinery already knows:

```
tier = (chain identity × runtime mode × LLM lane × hardware floor × earn class)
```

| | Lite | Pro | Max | Ultra |
|---|---|---|---|---|
| Runs | anonymous binary | `login` + miner | + `kat_drain` + replay participant | + trainer window host + vessels |
| Identity | none | `Player(1)` | replay class *(new, undesigned)* | `Trainer(2)` stake 1K |
| LLM lane | none | API *(leak-gated)* | groq → CF capped | API-first; local optional/unarmed |
| Hardware | any | desktop | CPU compile box, no GPU | GPU rig / container VPS |
| Earns | nothing | mining | replay *(undesigned)* + mining | endorsements + pool |
| Status | ✅ shipped | core ✅ / fixer BLOCKED | drain 🟡 / replay 🔴 | designed; ours-only at launch |

Topology alignment with the game stack (the owner's "nearly same topology as
game tier"): Ultra↔Authority (load-bearing, container), Max↔Curator(3)
(verify work, CPU-only, staked), Pro↔Player (desktop client), Lite↔
Spectator/Satellite (observer, vote 0). The 3-axis disambiguation pattern
(role/tier/hat) is reused verbatim as the spec's backbone.

### The corrections (the owner's brief → corrected)

1. **Ultra's "own llm > 27B local"** → **API-first; local optional + UNARMED.**
   Local Bonsai-27B: quality FAIL (WIN 5/REG 4, 2 silently wrong — Bench 039),
   ~191 s/fix (Bench 463), RLVR G1 0/60 twice (465/467). The serving-lane
   decision demoted it with a reopen bar (`l4_fallback.md` §5–6). An Ultra
   tier that markets local-LLM fixing contradicts the record; the container
   half survives as designed (deploy-once/cron-start/sleep-after, P005).
2. **Max's "fix 80%, consensus 20%"** → the split is **already the pipeline**:
   96.2% modelless heal rate; the L4 reachability contract routes ONLY misses
   (9/29 addressable) to any LLM. Consensus is deterministic-replay agreement,
   never a workload share. Max's real shape: **replay participant without GPU**
   (replay is compile-bound CPU work) + `kat_drain` API contributor.
3. **Max's "local ternary or llm < 8B"** → ternary IS the modelless L1 drafter
   (present in every tier, not an LLM); **no <8B model was ever benched** — the
   one measured mid-size model (qwen3.8-27b) failed economics. Entry condition
   for any small-local-model tier: a Bench-049-style gate.
4. **Pro's "fix nolimit via api like we have on cloudflare"** → no live fixer
   exists on CF; the Worker is the service plane, the Container is the trainer
   window. Pro = account+miner **+ fixer-consumer when the leak sign-off
   lands** (quality gate already lifted). "Unlimited" anything is false: meter
   + USD cap + never-wrong verify bind every tier.
5. **Lite's "consensus 100% confirm or witness"** → lite can **witness/verify**
   (client-side root verification, vote 0 — the Satellite pattern); it can
   never confirm consensus (staked, all-or-nothing per record).

Full table + per-tier detail: [the spec doc](../.docs/01_orientation/node_tiers.md).

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Trainer decentralization is v1 fiction (P005 caveat #10).** At launch,
   Ultra/Max-class work runs on OUR replicas. The tier page must carry an
   honest installable-today column; "run an Ultra node" marketing before the
   vessel/stake/endorsement phase gate is a lie we must not ship.
2. **Max's replay reward class does not exist.** Replay participation needs a
   new endorsement subclass (Curator-adjacent) + ledger support + a G7 sim
   extension. Until it can SETTLE, the page must not advertise replay
   earnings — the "do not market LLM fixing" rule, generalized.
3. **The leak-posture sign-off gates every consumer-src lane.** Pro's fixer
   half and any tier touching live consumer code stay unadvertised until the
   owner's sign-off (P005 caveat #2b). Quality (Bench 049) is NOT the gate.
4. **Prior-art honesty:** the packaging pattern (hardware-floor recruitment
   table + honest earnings) is Storj/Akash-shaped (cited-only); nothing here
   is claimed as externally novel. Our contribution is the corrections ledger
   binding the tier table to measured verdicts.
5. **Naming:** `ultra/max/pro/lite` have zero hits as tier vocabulary
   (grep 2026-09-12) — but `Ultra` collides with Apple-silicon prose ("M3
   Ultra") in the kernel_opt corpus. Keep tier names OUT of code identifiers
   (marketing/config surface only); re-run the zero-hit grep at plan time per
   the KAT naming discipline (P005 caveat #13).
6. **Doc drift is the standing risk.** Status cells (shipped/designed/blocked)
   must carry dates and live in the spec's status column; the doc-sync sweep
   covers it. A tier table with a stale "BLOCKED" is a leak-shaped incident.

## Fusion lineage

1. **Proposal 005 §"Node-tier additions"** — the role/hat rows being packaged.
2. **riir-ai `node_tiers.md`** — the 4-system disambiguation (role/tier/hat/
   platform) reused as the preset's axes; the Satellite placement pattern.
3. **`l4_fallback.md` §5 + Issue 051 `LlmRouter`** — the serving-lane decision
   and the groq-first API lane that make API-first the honest default.

## GOAT gate

Docs artifact: **G1** = every tier row maps 1:1 to a shipped or designed
mechanism with a citation (the spec's mapping table IS the audit; a row with
no citation fails). **G3** = no existing doc contradicts (cross-refs added
both directions — this proposal ↔ spec doc ↔ P005 ↔ riir-ai node_tiers.md).
G2/G4 N/A. The NEW mechanisms (replay reward class, CLI tier presets, the
ai.gist.rs chooser) each carry their own gate **before the public page
advertises them** — same law as "do not market LLM fixing".

## What ships now (riir-clippy) vs deferred

### Ships now — docs only
- [`.docs/01_orientation/node_tiers.md`](../.docs/01_orientation/node_tiers.md) — the canonical spec (this proposal's companion).
- This proposal + cross-references + the corrections ledger.

### Deferred
- **ai.gist.rs chooser page** (riir-dapps plan): capabilities-only copy, rig →
  tier → install command; blocked on the leak-gate + copy review.
- **`gist-rs/cargo-heal` README tier section** — same table, source-free (P003).
- **CLI tier presets** (`cargo heal login --tier max` as config sugar) —
  demand-gated; no behavior change implied.
- **Replay endorsement reward class** — the Max economics; needs design + G7.
- **Game-launch node chooser** — same artifact shape at game launch; separate.

### Explicitly NOT shipped
- No new chain tiers, no local-LLM fixing claims, no <8B model tier, no
  "unlimited" anything, no earnings promises beyond settleable classes.

## Phased rollout (sketch)

### Phase 1 — the spec (THIS)
- [x] T1.1 the canonical tier spec doc (`.docs/01_orientation/node_tiers.md`)
- [x] T1.2 the corrections ledger (in-doc) + this proposal
- [x] T1.3 index line in `.docs/01_orientation/README.md`

### Phase 2 — the consumer mirror (gated)
- [x] T2.1 ai.gist.rs chooser page (dapps `9694677`+`373b754`, live on devnet `c3c792b1` 2026-09-14 — the owner's ask unblocked it; capabilities-only copy honored; moved to the FRONT page 2026-09-15 as the tiers × roles matrix above a role-tabbed Get started, riir-dapps `0553a7f` — the leaderboard is stats-only now, live on all three envs)
- [x] T2.2 gist-rs/cargo-heal README tier section (source-free; cargo-heal `62a435f` 2026-09-15 — the roles × tiers matrix, capabilities-only; also fixed the README's per-token burn unit + next-release mining staleness the section would have contradicted)

### Phase 3 — mechanisms the table honestly lacks
- [ ] T3.1 replay endorsement reward class (design + ledger + G7 sim) — **design filed: [Proposal 015](015_replay_endorsement_reward_class.md)** (2026-09-14); ledger + sim = P015 Phases 1/3
- [ ] T3.2 CLI tier presets (config sugar only)

### Phase 4 — game launch (out of scope here)
- [ ] T4.1 the same chooser pattern for game node tiers

## Risks

1. **Marketing-ahead-of-mechanism** (the Max earnings class) — mitigated by
   the honest status column + "cannot settle ⇒ cannot advertise" law.
2. **Leak regression** — any consumer-src lane advertised early re-opens the
   P005 caveat #2 posture; the sign-off is a hard gate.
3. **Decentralization overclaim** — Ultra-before-phase-gate contradicts
   caveat #10; the page says "ours" until the machinery matures.
4. **Naming collision** (`Ultra` vs M3 Ultra prose) — marketing-surface-only
   names; never code identifiers.

## Out of scope

- Chain-tier changes (none — Trainer(2)/Player(1) stand as-is).
- Any LLM quality work (the lanes consume existing verdicts; reopen bars own
   the local lane).
- The game's own node tiers (noted as pattern reuse only).

## References

### In-house (the load-bearing evidence)
1. [Proposal 005](005_kat_heal_service_network.md) — §"Node-tier additions", §"The daily trainer window", caveats #2/#10/#13.
2. [riir-ai `node_tiers.md`](../../riir-ai/.docs/01_orientation/node_tiers.md) — Systems 1–4 + the KAT/Satellite placement rows.
3. [`l4_fallback.md`](../.docs/10_self_evolve/l4_fallback.md) — §5 serving-lane decision, §6 reopen bar.
4. [Bench 049](../.benchmarks/049_fixer_regate_poc.md) — the fixer quality re-gate (CF arms win; qwen3.8-27b unusable).
5. Benches 039 / 463 / 465+467 (`../.benchmarks/`; riir-train) — the local L4 demotion evidence.
6. Issue 051 — `LlmRouter` groq-first lane (`src/groq/router.rs`, `src/bin/kat_drain.rs`).

### External (cited-only)
7. Storj node-operator recruitment (hardware-floor table + honest earnings
   shape) — medium.com/@storjproject; Akash provider attributes
   (akash.network/docs) — attribute-based self-description vs curated
   presets: we ship curated presets OVER attribute axes.

## TL;DR

GO — the 4-tier packaging ships as the canonical spec doc now (Phase 1); the
consumer mirror waits on its gates; the corrections ledger keeps the table
honest against the measured record.
