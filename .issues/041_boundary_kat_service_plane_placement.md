# Issue 041 — boundary: the KAT service plane (CF Worker + `riir-chain-sdk`) has no home in riir-clippy

Status: **OPEN** — blocks [Proposal 005](../.proposals/005_kat_heal_service_network.md) Phase 0/1.
Filed: 2026-08-25 (proposal review — filed BEFORE any code, per the
`BOUNDARY.md` rule "file the issue FIRST, add the drift row, then fix").
Repo: `riir-clippy` (contract owner) · sibling candidate: `../riir-dapps`

## The violation Proposal 005 would introduce

Proposal 005 §"Ships now" places the service plane — **CF Worker (ingest +
ledger DO + payment routes)** — as a **riir-clippy workspace member**, and
composes payment-worker mints **via `riir-chain-sdk`**.

Both collide with this repo's own contract:

1. **`BOUNDARY.md` §"Does not own"** already routes *"Chain / storage /
   training"* to *"the respective `../riir-*` repos"*. A chain-SDK dep and a
   mint path are chain concerns by that row's plain text.
2. **`BOUNDARY.md` §"May depend on"** allowlists exactly four crates —
   `katgpt-core` (default), `riir-engine`/`riir-gpu` (`ternary_inference`),
   `riir-rag` (`latent_retrieval`), `riir-neuron-db` (optional). **`riir-chain`
   / `riir-chain-sdk` is not on it.** `../riir-ai/scripts/ci_boundary_contract.sh`
   fails on an undeclared cross-repo dep.
3. **§"Owns"** scopes this repo to *"Multi-domain modelless code healer —
   developer tooling behind the `HealDomain` trait"*. Stripe checkout, a
   Durable-Object ledger, and a wasm32 Worker are not developer tooling behind
   `HealDomain`.

Proposal 005's own Risk 7 names the coupling but mitigates it with *"worker is
a separate workspace member"* + *"boundary-checked at plan time"*. That does
not clear the gate: `ci_boundary_contract.sh` measures the **repo's** dep
graph, not a per-member closure, and the rule is issue-first, not plan-time.

## Options

| Option | Verdict |
|---|---|
| **A — service plane → `../riir-dapps`** (RECOMMENDED) | riir-dapps already owns *"game outcome → generic chain settlement"*, the exact-once settlement ring, and the tiered-durability model (`DoWal`/`R2Wal`) whose chaos/soak numbers Proposal 005's **G1 already reuses** (341 obligations G1 / 559 G8). Generalizing its settlement layer from "game outcome" to "metered-service outcome" is a smaller conceptual move than making a lint-fixer repo a payments processor. riir-clippy keeps the client half only (`kat_service`: key, meter, redaction gate, sync payload, lease consumer) — all of which ARE healer concerns. |
| **B — new repo `riir-kat`** | Clean but adds a 16th repo + a new BOUNDARY contract + CI row for one Worker. Prefer A unless riir-dapps' own boundary rejects the non-game framing. |
| **C — keep in riir-clippy, amend the contract** | Requires deleting/rewriting the "Does not own: Chain" row and widening §Owns to include payments. Rejected: it dissolves the row that keeps this repo shippable as a source-secret binary (§Standing invariants) and drags Stripe/DO secrets into the healer's release surface. |

**Note on the shared row format:** the proposal justified in-repo placement as
"keeps the `TrajStore` row format in-repo — DRY". That is satisfiable without
the dep: the sync payload schema is a **versioned serialization contract**
(riir-clippy owns it, Phase 0 T0.3); the Worker consumes it as data. A schema
is not a reason to co-locate a payments plane.

## Tasks

- [ ] T1 Decide A / B / C (owner) — A recommended.
- [ ] T2 If A: confirm against `../riir-dapps/BOUNDARY.md` that a
      metered-service settlement plane fits its §Owns, or widen it there with
      its own drift row.
- [ ] T3 Update Proposal 005 §"Ships now" + Phase 1 to the chosen home;
      riir-clippy's ships-now list keeps the **client half only**.
- [ ] T4 Confirm the client half needs **no** new cross-repo dep (key =
      existing SSH-Ed25519 loaders; meter = raw span bytes per Proposal 005
      §KAT; HTTP client is a third-party dep, not a riir dep) — then this
      issue closes with **no allowlist change at all**.
- [ ] T5 Run `boundary-guard` on the chosen layout; remove the drift row from
      `BOUNDARY.md` in the closing commit.

## Closing condition

Proposal 005's service plane has a declared home whose `BOUNDARY.md` §Owns
covers it, `ci_boundary_contract.sh` is green, and riir-clippy's allowlist is
unchanged (option A/B) or explicitly widened with rationale (option C).
