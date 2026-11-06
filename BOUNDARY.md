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

**Multi-domain modelless rulewright** — developer tooling behind the
`HealDomain` trait (clippy_lints, rust_perf, kernel_opt). Borrows *patterns*
from game crates but has zero game-domain coupling.

**Issue 088 (2026-09-10):** the KAT service plane (protocol tier, client
plane, transports — 25 modules) moved to the sibling leaf crate
`../riir-kat`; root re-exports every moved module at its historical path
(`riir_refine::kat_meter` etc. resolve bit-identically). Root KEEPS the
rulewright-coupled `kat_sync_payload` + `kat_pending_estimator` (traj_store)
and `kat_promotion` (score_bench/domain).

**Issue 200 (2026-11-05):** the node-composition family — TWO inner
workspace members under the root-package workspace (`default-members =
["."]`, so every bare cargo command and every pre-existing gate stays
root-scoped):

- `crates/riir-refine-fixer` — the FIXER TIER: the migrated `fixer_regate`
  bin + the charter for the fail-closed OSS-class check (the owner trust
  rule: the trust boundary follows CODE OWNERSHIP, not node class —
  "open-source class" is checked client-side BEFORE any network connection,
  fail-closed; anything unconfirmable is PRIVATE → self-host route).
- `crates/riir-refine-trainer` — the TRAINER TIER: the migrated `kat_drain`
  bin; feature-superset of the fixer. The intra-repo path edge to the fixer
  is allowlisted but carried only once the trainer consumes a fixer-lib item
  (an unused edge trips cargo 1.101 `cargo::unused_dependencies`; dropped
  2026-11-06, issue 211). NEVER-WRONG by
  construction: the drain never writes a repo file.

Tier law: tier manifests forward the ROOT'S EXISTING optional seams
(`cf_workers_ai`, `groq_first`) and add NO new sibling edge to the graph —
the shared Cargo.lock resolves every member's path deps, so each NEW tier
sibling edge (e.g. the TUNA-client class, when the operator arms it) gets
an allowlist row HERE before it lands (issue 200 verdict round 4). Tier
bins are NATIVE-ONLY — never join a wasm32 combo (the `hero_planner`
posture). `riir-llm` (hosted options) and `riir-infer` serving (opt-in
`ternary_inference`) stay root-allowlisted rows, reachable by the tiers
through the core's features.

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
| katgpt-percepta | `../katgpt-rs/crates/katgpt-percepta` | opt-in `wasm_validator` only (Plan 199 T3.2 — the WASM validator lane's MVP decoder + lowering pipeline at `percepta_wasm`; `good_lp`/`percepta_compile` stay off). Same katgpt-rs checkout as katgpt-core — no new CI sibling. |
| katgpt-device-verify | `../katgpt-rs/crates/katgpt-device-verify` | **`[patch]` entry only — no code-level dep** (`grep katgpt_device_verify` over `*.rs`: 0 files). A patch section must cover EVERY crate resolved from the patched source, and `riir-neuron-db` resolves this one under `latent_retrieval`; without the row its git copy survives and seven katgpt crates compile twice (Issue 058). Same ride-along shape as `riir-mmorpg-examples`'s row. |
| riir-infer-core, riir-infer-gpu | `../riir-infer`, `../riir-infer/crates/riir-infer-gpu` | opt-in `ternary_inference` only (T2.3 retarget: the ternary lane's inference substrate is the public inference repo — zero riir-ai edges on this lane; loaders/tokenizers/kernels/drafters all resolve there) |
| riir-rag | `../riir-neuron-db/crates/riir-rag` | opt-in `latent_retrieval` only (crate carved there from riir-ai, riir-ai Issue 1038, 2026-10-07) |
| riir-rag-bonsai | `../riir-ai/crates/riir-rag-bonsai` | opt-in `bonsai_retrieval` only — the Bonsai-27B dense-embedder arm (carved out of riir-rag by the same issue; it forwards `riir-rag/dense_embed` so the dense channel rides along) |
| riir-neuron-db | `../riir-neuron-db` | `optional = true`, `default-features = false` (measured 2026-08-21) |
| riir-kat | `../riir-kat` | `default-features = false` (non-optional — root re-exports the member's free-tier trio ungated); `kat_service` enables `riir-kat/client`, `kat_transport` adds `riir-kat/kat_transport` (issue 088, 2026-09-10). riir-auth is no longer a ROOT dep — it enters the graph transitively through riir-kat's `account_key` (same sibling, resolution-load-bearing). |
| riir-ledger | `../riir-ledger` | the `points` feature only (riir-refine Proposal 026 P3 / Plan 325 T3, 2026-11-02): the points wire leaf consumed by `src/points_client.rs` — protocol tier (`default-features = false` is its only shape: the crate has NO features at all, serde + serde_json only), zero sibling deps. The wire is the contract; the client policy (consent, offline gate, cache, rendering) stays HERE |
| riir-llm | `../riir-llm` | `optional = true`, `default-features = false` (issue 124, 2026-09-18 — the shared LLM-transport leaf, the Issue-051/106 extraction). `groq_first` = `riir-llm/transport` (no direct ureq of its own); `cf_workers_ai` adds `riir-llm/transport` beside its own ureq (the GraphQL usage meter stays clippy-side — the leaf speaks chat-completions, not GraphQL). Wire URL/body/parse/classify/timeouts/bearer hygiene live in the leaf; the windowed `LlmRouter` policy, FreeTierStats/providers ledger contract, pricing (`avoided_usd`, `llm_spend`) stay HERE. |
| `reflexer-vessel` | `../riir-reflexer/crates/reflexer-vessel` | opt-in `vessel_artifacts` only (Plan 202 R4, 2026-10-08 — the vessel verify path, ONE artifact protocol across the decision stack). `default-features = false` + `vessel_public_read` ONLY — the hosted reader never compiles into this publicly-distributed binary (the moat: hosted vessels refuse at peek, structurally). Verify-only: refine NEVER mints (A8 — minting is riir-train's offline lane) and NEVER embeds vessel bytes (A10 — the artifact-dir leak test pins it). Public crate, deps blake3 + ed25519-dalek only; same katgpt-free leaf law as this repo's default build. |

## Standing invariants

- **Source secrecy**: this repo's source never becomes public. Distribution is
  prebuilt binaries only, via the 3 tiny source-free public repos
  (Proposal 003). Never `cargo publish`; never ship dev builds or
  `ternary_inference` in release artifacts.

## Inherited boundaries (links)

- Dep direction: `../riir-ai/BOUNDARY.md`

## Drift ledger (target vs actual)

**None.** Row D1 (the reflexer-vessel allowlist cell shape, Issue 154, found
by the 170th boundary run 2026-10-08) is closed — ⚠ the first fix commit
(`6135b0f3`) edited only THIS prose and never backticked the cell (its
message claimed otherwise — caught by the 172nd scoped run, Issue 155); the
cell backtick itself landed with the Issue-155 fix commit the same morning.

Otherwise clean (surfaces clean at the 2026-08-19 guard run; the
2026-08-25 Proposal-005 service-plane row closed with Issue 041 — home
resolved to `../riir-dapps`, client half now OWNED by `../riir-kat` per
issue 088, allowlist updated same commit).
