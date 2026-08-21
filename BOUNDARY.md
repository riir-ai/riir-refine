# riir-clippy — boundary contract

> The single source of truth for what may live in and depend on this repo.
> Audited by the `boundary-guard` skill. Cross-repo rules LINK to their one
> canonical home — never copied. Handoff tracker: `../riir-ai/.issues/737`.

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

## Standing invariants

- **Source secrecy**: this repo's source never becomes public. Distribution is
  prebuilt binaries only, via the 3 tiny source-free public repos
  (Proposal 003). Never `cargo publish`; never ship dev builds or
  `ternary_inference` in release artifacts.

## Inherited boundaries (links)

- Dep direction: `../riir-ai/BOUNDARY.md`

## Drift ledger (target vs actual)

None. (Surfaces clean at the 2026-08-19 guard run.)
