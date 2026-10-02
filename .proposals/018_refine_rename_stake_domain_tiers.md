# Proposal 018 — `riir-refine`: the rename (clippy → refine), the stake→domain tier axis, and the fleet umbrella alignment

**Status:** Phase 1 EXECUTING — T1.2+T1.3 landed 2026-10-01 (binary `cargo refine` + the surface sweep); **T1.2f dist flip EXECUTED 2026-10-01** (owner renamed the GitHub dist repo → `gist-rs/cargo-refine`; dist repo swept; **v0.2.0 RELEASED** — 5 targets built locally via zigbuild per the manual-deploy rule, the size-gate ceiling re-based on the first all-five measurement 6.58–8.20 MiB → 9.0 MiB with the numbers recorded, leak-scan 5/5 clean; tap `Formula/cargo-refine.rb` + scoop `bucket/cargo-refine.json` flipped, no aliases; dapps install copy swept, parity 3/3; installers keep pre-rename releases installable via the cargo-heal asset-spelling fallback) + **the owner amendment: user state `.heal` → `.refine`** (2026-10-01, the 2-machine no-alias call: code + disk migrated on the M3 — 24 repos + home + `.refine-sync-policy`; the account key lives in `~/.config/riir-auth`, never at risk; env vars `RIIR_HEAL_*` + wire domains + internal heal vocabulary stay by design). T1.1 repo rename remains. Filed 2026-09-30 on owner direction (same-session calls: the rename ordered with `Refine` picked over `Repair` — "sometimes it's not repair but improve"; NO compatibility aliases — the fleet is 2 machines; `mainnet.ai.gist.rs` to be turned OFF; **Reflex stays free, no KAT plane**). Companion landing plan: [riir-dapps Plan 042](../../riir-dapps/.plans/042_fleet_domains_umbrella.md). Sibling design: [riir-ai Proposal 051](../../riir-ai/.proposals/051_rethink_all_tier_adaptive_serving_family.md) (rethink's identical tier axis + the R-family).
Branch: `develop` (per global rule — no feature branches)
Owner: katopz (name + no-alias + reflex-free calls, 2026-09-30)
Fusion of: [Proposal 013](013_consumer_install_tier_spec.md) (the tier presets this extends) × [Proposal 003](003_binary_only_distribution.md) (the dist surface) × [Proposal 007](007_kat_usdc_staking.md) (the live stake substrate) × riir-ai Proposal 051 (the same sixth axis, designed for rethink first)
Related: [Proposal 015](015_replay_endorsement_reward_class.md) (the earn side of the ladder), [`node_tiers.md`](../.docs/01_orientation/node_tiers.md) (the canonical tier spec), Batch 193 `621b75c0` (the multi-domain mining-live evidence, 2026-09-30)

## TL;DR

Rename `riir-clippy` → **`riir-refine`** (binary `cargo heal` → `cargo refine`), completing the re-family — **Reflex (free) / Rethink / Refine** — and extend the consumer tier table with the SAME sixth axis rethink adopts: **stake posture → served-domain set** (more stake = higher tier = more of the 7 live rule domains unlocked, for healing AND mining — "mine where you're staked"). The rename is brand-surface only: internal `heal` vocabulary (`HealDomain`, `heal.rs`, `.heal/` state dirs), the wire/domain separators (`kat:healqual:v1:` — immutable once shipped), and the account state NEVER change (move-don't-rewrite). No aliases ship (owner call: the fleet is the M3 + 4090; the public dist repos exist but carry no user base to strand — the revisit trigger is the first external install). This is our own packaging of shipped machinery plus one designed axis, honestly labeled per the P013 laws.

## The problem this solves

1. **The name predates the family.** `riir-clippy` names the original clippy-lint healer; the product is now SEVEN domains (clippy_lints 100 / rust_perf 165 / kernel_opt 703 / sec / docker / dist / shader_perf), and the fleet is branding as the re-family (Reflex, Rethink). "clippy" undersells exactly the way "repair" would have.
2. **The tier table has no domain axis.** P013's presets (Lite/Pro/Max/Ultra over five axes) predate multi-domain mining being LIVE (Batch 193 today). Domains are minting assets with measured value; nothing lets a user's commitment (stake) buy deeper domain access — the axis rethink's 051 already designs.
3. **The umbrella needs its third pillar named.** ai.gist.rs becomes "Reflex. Rethink. Refine." — this proposal supplies the third name and its tier ladder; dapps Plan 042 executes the domains/sites.

## The naming law

| Surface | From | To |
|---|---|---|
| Repo | `riir-clippy` | **`riir-refine`** (the 22-file registration: repo_set + pins + sweeps + skill symlinks — the fleet's LARGEST; own commit family) |
| Binary | `cargo heal` | **`cargo refine`** — NO alias binary (owner call 2026-09-30; revisit at the first external install) |
| Public dist repo | `gist-rs/cargo-heal` | `gist-rs/refine` (tap formula + scoop manifest renamed in the same pass; no dual-name transition needed at a 2-machine fleet) |
| Internal vocabulary | `heal.rs`, `HealDomain`, `heal()` verbs, `cargo heal --fix` semantics | **UNCHANGED** — move-don't-rewrite; the healer's own verbs stay (`.heal/` state dirs left this tier — migrated with user state, below) |
| Wire constants | `kat:healqual:v1:`, `kat:corpusdays:v1:`, fixstats/decstat domain separators | **UNCHANGED, IMMUTABLE** — codes are stable forever once shipped; display strings change freely |
| User state | `~/.heal` account dir, outbox, fixseq rings; per-repo `.heal/` state dirs | **MIGRATED `→ .refine`** — owner amendment 2026-10-01 (the 2-machine no-alias call; code constants + disk moved together, 24 repos + home) |
| Cargo feature flags | `kernel_heal`, `kernel_verify`, `clippy_lints`, `rust_perf`, … (+ every `required-features` name) | **UNCHANGED** — internal tier, same standing as `RIIR_HEAL_*`; users never type them (the domain id is `--domain kernel_opt`). Verdict 2026-10-02: renaming the flag alone leaves `HealDomain`/`heal()` mixed and worsens `refine` grep noise (caveat 4); ~22 files of churn, zero behavior gain |

**Why Refine (the pick history, disclosed):** `Repair` was the earlier candidate — passed over because the product's value is fix + improve + optimize, and only ONE domain (rustc_errors) is literally repair; rust_perf improves, kernel_opt optimizes, clippy_lints refines style, sec hardens. `Reforge` was rejected pre-grep (MMO item-stat vocabulary; Seal IP lives in this workspace — a future sage-class collision). **Collisions, honestly:** `refine` is prose-noisy (ODE-refined, trajectory-refined — noise, never vocabulary); two aligned identifiers exist (`partition_refine` in katgpt-core bisimulation — state-minimization, a different concept, one disambiguation row; the `trd_refined_draft` feature); external: **refine.dev** (React admin framework, unrelated domain — nominative care in public copy, no adjacency). `cargo refine` occupies a free cargo-subcommand slot (`cargo fix` is taken; refine is not).

## The stake→domain tier axis (the sixth axis, same design as 051)

P013's law: "a tier is an install preset over EXISTING axes, no new chain tiers." This adds ONE axis — **the served-domain set** — selected by stake posture (live substrate: Proposal 007 staking, `/stake/info`):

| refine tier | stake posture | domains unlocked (heal + mine) |
|---|---|---|
| Trial | none | the basic domain(s), TUNA-budgeted |
| Pro | account (stake 0) | basic domains, KAT burn per M tokens |
| Max | staked | + mid domains (e.g. rust_perf, sec) |
| Ultra | stake 1K-class | all 7 domains + trainer/vessel surfaces |

- **Gating point:** the corpus lease/sync + mining admission (server-side) — the binary ships all domain features; the RULE CORPORA and the mining lanes are what gate. **Mine where you're staked** — the same posture gates both directions of value.
- **The soft gate gains a third door:** contribute / pay / **stake** (the 075 gate's refuel contract gains the stake row).
- Every constant owner-gated (never-lever class); capabilities-only copy until the gating settles (the P013 "designed — does not settle yet" law); tier vocabulary stays out of code identifiers and is re-grepped at plan time (P013 caveat 5).

## Reflex stays free (owner call, recorded)

**Reflex has no KAT plane — free forever, local and arena.** 014 Phase 4's reflex-hosted burn lane is SUPERSEDED: the hosted/metered plane belongs to Rethink (051 §distribution) and Refine. The umbrella's free pillar is reflex; the paid pillars are refine + rethink; one account, one balance, TUNA-first across both paid products.

## Honest caveats — READ BEFORE IMPLEMENTING

1. **The rename is the fleet's largest 22-file registration.** clippy is pinned in more sweep/floor/alias tables than any repo, plus `~/.agents/skills/` symlinks (the cargo-heal skill + the riir-clippy idle-loop skill), the `.distill/001` resume state, and the release pipeline. Own commit family; registration gate green before push. Done partially it reds nineteen sweeps (the riir-llm precedent).
2. **No-alias is a 2-machine call, recorded with its revisit trigger.** The dist repos are PUBLIC (installable by anyone); the moment a real external install exists, a silent DEFAULT/name break strands it. The trigger: first verifiable external user → alias + compat pass owed.
3. **The tier axis is designed, not shipped.** No domain-unlock advertising until the gating settles; the constants are owner acts; the P013 corrections ledger binds (no "unlimited anything", no percentage dials).
4. **The name's collision tax is prose-heavy.** `refine` greps will surface refinement prose everywhere; the two identifier collisions (partition_refine, trd_refined_draft) need their disambiguation rows; refine.dev needs nominative care. Acceptable — the rethink precedent — but real.
5. **`mainnet.ai.gist.rs` teardown is env-args-load-bearing** (`wrangler delete --env mainnet`, NEVER bare — bare = the apex = the real mainnet; the dapps AGENTS hazard). Plan 042 T4 carries the exact command.

## Fusion lineage

P013's tier presets × P007's live staking × 051's sixth axis (designed for rethink, mirrored here) → the stake→domain ladder; the name completes the Reflex/Rethink/Refine family from the same owner session that produced 051.

## GOAT gate

- **G1:** the rename is behavior-neutral — every gate (test_gate, full guard, score-bench floors) green with zero count movement; old bench/issue numbering intact.
- **G2/G4:** N/A (no hot-path change).
- **G3:** no doc contradicts — AGENTS/HISTORY/skill docs swept in the rename commit family; the `node_tiers.md` one-place rule holds (mirrors name their source).
- **Tier axis G:** the gating settles on devnet first (chaos arm: a staked account's domain set flips exactly at the posture boundary); conservation untouched (no new emission); capabilities-only copy until then.

## Phased rollout (sketch — the landing plan expands)

### Phase 1 — the rename (mechanical, behavior-neutral)
- [ ] T1.1 repo rename + the 22-file registration (repo_set, pins, sweeps, alias tables, skill symlinks, `.distill` paths)
- [ ] T1.2 binary `cargo refine` + release surface (`gist-rs/refine`, tap, scoop — renamed, no dual-name transition)
- [ ] T1.3 docs sweep: AGENTS/HISTORY/README/skill docs; internal `heal` vocabulary explicitly NOT renamed (the move-don't-rewrite note in the diff message)

### Phase 2 — the stake→domain axis
- [ ] T2.1 dapps: the tier rows (stake posture → served-domain set) + the soft-gate third door (contribute/pay/stake)
- [ ] T2.2 the corpus-lease/mining gating (server-side; devnet-first; owner constants)
- [ ] T2.3 `node_tiers.md` gains the sixth axis (the one-place rule) + the mirrors (ai.gist.rs umbrella, refine.gist.rs)

### Phase 3 — the fleet domains (executed by dapps Plan 042)
- [ ] T3.1 cross-ref: umbrella at the apex, refine.gist.rs marketing, devnet console, mainnet.ai.gist.rs OFF, reflex-site lane renames, rethink.gist.rs — see [Plan 042](../../riir-dapps/.plans/042_fleet_domains_umbrella.md)

## Risks

1. **Migration risk (highest):** the 22-file registration done partially → cross-repo sweep reds. Mitigation: one commit family, gates green before push.
2. **Strand risk:** the no-alias break at a 2-machine fleet is safe TODAY; the public dist makes it unsafe TOMORROW — the revisit trigger is mandatory (caveat 2).
3. **Naming risk:** refine's prose noise in greps + the two identifier rows (caveat 4).
4. **Economics risk:** the tier axis advertised before it settles (the P013 law) — capabilities-only copy enforced in review.

## Out of scope

- No internal vocabulary rename (heal stays heal in code/wire; the user state + per-repo state dirs migrated per the amendment above).
- No new chain tier (P013 law); no dao lever (constants owner-gated, never-lever class).
- No change to reflex/reflexer/rethink surfaces beyond Plan 042's cross-refs.
- No mainnet arming of anything.

## References

- Internal: P013 + `node_tiers.md` (the tier substrate), P003 (the dist pattern), P007/P017 (live staking), P015 (the earn ladder), riir-ai 051 (the sibling sixth-axis design), dapps Plan 042 (the domains/sites execution), Batch 193 `621b75c0` (mining-live).
- External: refine.dev (the name collision, disclosed — unrelated domain); no distillation source (this is our own packaging + one designed axis).

## TL;DR (closer)

**Ship the rename and the axis, phased:** Phase 1 renames (behavior-neutral, the fleet's biggest registration, no aliases per the owner call with its recorded revisit trigger); Phase 2 lands the stake→domain ladder on devnet with the P013 copy laws; Phase 3 is Plan 042's domains pass. Open the Phase-1 plan when the owner waves it through.
