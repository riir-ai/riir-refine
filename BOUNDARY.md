# riir-refine — boundary contract

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

**Issue 088 (2026-09-10):** the KAT service plane (protocol tier, client
plane, transports — 25 modules) moved to the sibling leaf crate
`../riir-kat`; root re-exports every moved module at its historical path
(`riir_clippy::kat_meter` etc. resolve bit-identically). Root KEEPS the
healer-coupled `kat_sync_payload` + `kat_pending_estimator` (traj_store)
and `kat_promotion` (score_bench/domain).

## Does not own

| Concern | Correct home |
|---|---|
| Game runtime / cognition | `../riir-ai` |
| Chain / storage / training | the respective `../riir-*` repos |

## May depend on

| Crate | Location | Condition |
|---|---|---|
| katgpt-core | `../katgpt-rs/crates/katgpt-core` | **the only dep in the default build** |
| katgpt-micro-belief | `../katgpt-rs/crates/katgpt-micro-belief` | opt-in `nonergodic_poc` only (Issue 093 T1 — the `NonergodicFilter` kernel, feature-forwarded `nonergodic_belief`). Already in the graph transitively via katgpt-core defaults (bom_sampling → micro_belief); this row is the direct path + feature gate. |
| katgpt-device-verify | `../katgpt-rs/crates/katgpt-device-verify` | **`[patch]` entry only — no code-level dep** (`grep katgpt_device_verify` over `*.rs`: 0 files). A patch section must cover EVERY crate resolved from the patched source, and `riir-neuron-db` resolves this one under `latent_retrieval`; without the row its git copy survives and seven katgpt crates compile twice (Issue 058). Same ride-along shape as `riir-mmorpg-examples`'s row. |
| riir-infer-core, riir-infer-gpu | `../riir-infer`, `../riir-infer/crates/riir-infer-gpu` | opt-in `ternary_inference` only (T2.3 retarget: the ternary lane's inference substrate is the public inference repo — zero riir-ai edges on this lane; loaders/tokenizers/kernels/drafters all resolve there) |
| riir-rag | `../riir-ai/crates/riir-rag` | opt-in `latent_retrieval` only |
| riir-neuron-db | `../riir-neuron-db` | `optional = true`, `default-features = false` (measured 2026-08-21) |
| riir-kat | `../riir-kat` | `default-features = false` (non-optional — root re-exports the member's free-tier trio ungated); `kat_service` enables `riir-kat/client`, `kat_transport` adds `riir-kat/kat_transport` (issue 088, 2026-09-10). riir-auth is no longer a ROOT dep — it enters the graph transitively through riir-kat's `account_key` (same sibling, resolution-load-bearing). |
| riir-llm | `../riir-llm` | `optional = true`, `default-features = false` (issue 124, 2026-09-18 — the shared LLM-transport leaf, the Issue-051/106 extraction). `groq_first` = `riir-llm/transport` (no direct ureq of its own); `cf_workers_ai` adds `riir-llm/transport` beside its own ureq (the GraphQL usage meter stays clippy-side — the leaf speaks chat-completions, not GraphQL). Wire URL/body/parse/classify/timeouts/bearer hygiene live in the leaf; the windowed `LlmRouter` policy, FreeTierStats/providers ledger contract, pricing (`avoided_usd`, `llm_spend`) stay HERE. |

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
resolved to `../riir-dapps`, client half now OWNED by `../riir-kat` per
issue 088, allowlist updated same commit).
