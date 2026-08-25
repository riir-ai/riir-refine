# riir-clippy — boundary contract

> The single source of truth for what may live in and depend on this repo.
> Audited by the `boundary-guard` skill + two scripts:
> `../riir-ai/scripts/ci_boundary_contract.sh` (workspace — is the dep graph
> what this file says, and is this file still honest?) and
> `../riir-mmorpg-examples/scripts/ci_boundary_guard.sh` (per-repo — is the
> CODE in the right repo?). Cross-repo rules LINK to their one
> canonical home — never copied. Rollout record:
> `../riir-ai/.docs/01_orientation/737_boundary_contract_rollout_record.md`.

## Owns

**Multi-domain modelless code healer** — developer tooling behind the
`HealDomain` trait (clippy_lints, rust_perf, kernel_opt). Borrows *patterns*
from game crates but has zero game-domain coupling.

## Does not own

| Concern | Correct home |
|---|---|
| Game runtime / cognition | `../riir-ai` |
| Chain / storage / training | the respective `../riir-*` repos |

## May depend on

| Crate | Location | Condition |
|---|---|---|
| katgpt-core | `../katgpt-rs/crates/katgpt-core` | **the only dep in the default build** |
| riir-engine, riir-gpu | `../riir-ai/crates/*` | opt-in `ternary_inference` only |
| riir-rag | `../riir-ai/crates/riir-rag` | opt-in `latent_retrieval` only |
| riir-neuron-db | `../riir-neuron-db` | `optional = true`, `default-features = false` (measured 2026-08-21) |
## Standing invariants

- **Source secrecy**: this repo's source never becomes public. Distribution is
  prebuilt binaries only, via the 3 tiny source-free public repos
  (Proposal 003). Never `cargo publish`; never ship dev builds or
  `ternary_inference` in release artifacts.

## Inherited boundaries (links)

- Dep direction: `../riir-ai/BOUNDARY.md`

## Drift ledger (target vs actual)

**None.** Otherwise clean (surfaces clean at the 2026-08-19 guard run; the
2026-08-25 Proposal-005 service-plane row closed with Issue 041 — home
resolved to `../riir-dapps`, client half only, allowlist unchanged).
