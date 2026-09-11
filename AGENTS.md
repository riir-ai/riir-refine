# AGENTS.md — riir-clippy

The global `~/.agents/` rules apply; this file documents repo-local context.
History, batch records, sweep campaign records, score-bench history: `HISTORY.md`.

## Boundary contract — read `BOUNDARY.md` first

[`BOUNDARY.md`](BOUNDARY.md) is the authoritative per-repo contract (owns /
does-not-own / dep allowlist / drift ledger); on any conflict, BOUNDARY.md wins. Enforcement is not prose: `../riir-ai/scripts/ci_boundary_contract.sh` — run via the `boundary-guard` skill, never ad-hoc greps. Found a violation? File the issue FIRST (`.issues/NNN_boundary_*.md`), add the drift row, then fix; closing the issue removes the row in the same commit.

## Role

**Multi-domain modelless code healer** — a pluggable framework behind the `HealDomain` trait (Plan 001) that heals code via a drafter+pruner+corpus pattern. **Developer tooling repo** — NOT game, NOT training, NOT chain.

Retrieval domains (Plan 001 + Plans 018/030; `rustc_errors` is a 6th, driver-based rather than retrieval) — batch-by-batch corpus narratives live in `HISTORY.md`:

- **`clippy_lints`** (default-on) — heals `cargo clippy` warnings; 49 corpus lints across style/complexity/perf/pedantic/nursery (38 `--fix`, 8 `--verify`, 3 self_evolve-only).
- **`rust_perf`** (default-on) — heals Rust perf issues; 126 rules, promoted via the `rust_perf_bench` Pareto + alloc-count validator.
- **`kernel_opt`** (opt-in) — heals GPU kernel optimization; 607 rules.
- **`sec`** (opt-in) — security-pattern bounded fixes (15 rules; 6 fixable + 9 suggest-only).
- **`docker`** (default-on) — Dockerfile bounded-fix healing (31 rules, hadolint oracle).
- **`rustc_errors`** (default-on) — `cargo heal --fix-compile`, oracle-driven compile-error repair keyed by rustc error code; driver-shaped, inert until invoked.

The pipeline (shared across all domains):

1. **L0 — Rule distillation**: each rule → named entry + bounded fix-space.
2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec, zero-alloc.
3. **L2 — Validator**: `syn`-based syntax parse check (always-on, modelless); domain-specific verification is opt-in: clippy → `clippy_verify` (real `cargo clippy` re-run), rust_perf → `rust_perf_verify` (`cargo test` re-run), kernel_opt → `kernel_verify` (CubeCL compile, STUB).
4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration.
5. **L4 — LoRA fallback** (deferred/inert): adaptive learning, last resort — enforced in code (see `l4_lora`).

### Status (Plan 005 / retrieval / self-evolve)

- Plan 005 complete: `cargo heal --suggest / --fix / --fix-compile`; latent retrieval via riir-rag — fan-out across enabled domains, the router verdict is a per-span ANNOTATION, never a gate; Phase 3 CLOSED at measured G1 (`RerankMode::Structural` default).
- `fix_verify` = the compile-gated `--fix` mode (baseline check → apply → revert offending edits). `self_evolve` DEFAULT-ON (Bench 023); trajectories persist on the neuron-db Warm-tier store under `latent_retrieval` (`.heal/trajectories.kv`); opt-out `RIIR_HEAL_SELF_EVOLVE=0`.
- Domain GOATs on record: `rustc_errors` at N=212 (Bench 045), `docker` Phase 0+1 (Bench 056), `sec` Phase 0+1 (Bench 046).
- L4 fallback seam installed but NOT promoted — riir-train Plan 336 measured FAIL (0/60); `decline` is a correct answer.
- score-bench: run `cargo heal --score-bench` after every proposer/corpus change (append-only `.benchmarks/score_history.jsonl`; generations 2–7 records in `HISTORY.md`).

## CF ops surface + the `.env` contract (read before touching ops)

This repo hosts the **LLM ops plane**: the `riir-kat-housekeeping` worker (`cloudflare/housekeeping/`), the `riir-ops-dashboard` worker (`cloudflare/dashboard/`), and the nightly generator (`scripts/ops_nightly.sh`). Credentials live in **`.env` (gitignored — never commit, never print values)**; [`.env.example`](.env.example) is the documented template. AI sessions: treat `.env` as read-only local state — pipe values into `wrangler secret put` via stdin, never echo them.

**Deploy hazard (load-bearing):** wrangler honours `CF_API_TOKEN` as a deprecated alias of `CLOUDFLARE_API_TOKEN` — an analytics-scoped value in the deploy env silently breaks auth. Every `wrangler deploy` in this repo runs under `env -u CLOUDFLARE_API_TOKEN -u CF_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL …`. Keep that prefix; a bare `npx wrangler deploy` is the bug. (Full record — issue resolutions, dashboard-auth state, cost model: `HISTORY.md`.)

## The `riir-clippy` skill (auto-continue idle loop)

`.agents/skills/riir-clippy/SKILL.md` (symlinked into `~/.agents/skills/`; canonical home is THIS repo) encodes the idle-time workflow: fix open issues → finish in-flight plans → clippy sweeps (heal-driven) → resolved-issue hygiene → batch mining + heal dry-run → boundary/substrate checks → benchmark sweeps → online unblock/perf search → paper research → doc-sync. One unit per cycle, commit + push each unit. Cross-session resume state: `.distill/001_mining_queue_snapshot.md` (updated in the same commit as each batch). Triggers: "continue", "auto continue", "idle", "nothing to do", "next batch", "Batch 32+". Auto_prompt handoffs are gated by the handoff summary's (3) What remains — "Nothing" falls through to the loop, listed work resumes first.

## Production adoption — `cargo heal` is the dev-time default

**Verdict: GO** (live trial, 2026-08-24). The healer is the DEFAULT tool for mechanical clippy findings across the workspace, ahead of manual fixes and `cargo clippy --fix`.

- **Global binary**: `~/.cargo/bin/cargo-heal` → symlink to `target/release/cargo-heal` (built `--features fix_verify,clippy_verify` — rebuild after healer source changes; the symlink keeps resolving).
- **Dev-time skill**: `cargo-heal` (`.agents/skills/cargo-heal/`, symlinked globally) — usage, divergence table, discipline, post-mining logging.
- **Post-mining**: improvement candidates mined from the healer's OWN usage artifacts; intake lives in `.distill/001_mining_queue_snapshot.md` §"Post-mining intake".

## Guard-landing protocol — a guard is not landed until the residue is counted

Landing a proposer guard stops the healer **emitting** a broken class. It does nothing about what the un-guarded healer already emitted and that already landed in consumer repos. **The rule:** on landing a guard for class C, run the sweep and paste its table into the landing note. Fix or file — never silently leave it unmeasured.

```bash
scripts/consumer_residue_sweep.sh <lint> [<lint> ...]        # the screen
scripts/consumer_residue_sweep.sh --grep '<regex>' <lint>    # narrow first
```

Target set: `scripts/swept_repos.txt` (derived from the BOUNDARY.md workspace, git worktrees excluded — the file documents how to re-derive it). Exits non-zero when residue exists. **The screen is the claim**: `cargo clippy --workspace --all-features --all-targets --keep-going`. Not `cargo check` — the guarded classes are accepted by check's typeck and rejected by clippy's, so the healer's own `--verify` compile gate is blind to them by construction. Not default features — measured residue sat behind non-default ones. The script `cargo clean -p`s each workspace member inside a sweep-local `CARGO_TARGET_DIR`, so a cached build cannot print a quiet zero (and it prints a per-repo diagnostic total as a liveness witness). (The measured story + the JSON-shape traps: `HISTORY.md`.)

## Source secrecy & binary-only distribution (non-negotiable)

**Per [Research 003](../riir-ai/.research/003_Commercial_Open_Source_Strategy_Verdict.md): "anything `riir-*` is internal. No exceptions."** This repo's source never becomes public. The product surface (`cargo heal`) ships as **prebuilt binaries only**, via GitHub Releases on a small PUBLIC distribution repo — NOT on this repo (release assets on a private repo are not publicly downloadable). Full design: `Proposal 003`. Hard rules for any future session:

1. **Never publish source.** No `cargo publish` (already blocked: `publish = false` in `Cargo.toml`), no public mirrors, no source tarballs in release assets, no "open-source it for adoption" — that path is reserved for `katgpt-rs` (the public funnel) alone.
2. **Public surface = 3 tiny source-free repos** (created 2026-09-07, Plan 105): `gist-rs/cargo-heal` (releases + `install.sh` + `install.ps1` + capabilities-only README), `gist-rs/homebrew-tap` (Formula), `gist-rs/scoop-bucket` (manifests). **Zero `.rs` files allowed in any of them** (CI guard seeded in each; the no-source guard runs on their pushes). Do not create other public distribution surfaces without a proposal.
3. **Release binary hygiene**: build with `default + latent_retrieval + kat_service + kat_transport + fix_verify` features ONLY (the release feature set — Plan 107 T1 added `kat_service`, v0.1.2 added `kat_transport`, v0.1.4 added `fix_verify` 2026-09-09; the licenses + build jobs in `release.yml` MUST pass the SAME feature set or `THIRD_PARTY_LICENSES.md` silently misses crates); `strip = true` in the release profile; ship `SHA256SUMS` + `THIRD_PARTY_LICENSES.md` in every archive (MIT/Apache binary-distribution obligation); never ship dev/debug builds or `ternary_inference`.
4. **Windows users** get `install.ps1` + the scoop bucket; winget only after Authenticode signing (deferred). macOS/Linux get `install.sh` + the brew tap.

## Why this is a separate repo (not in riir-ai)

Borrows the **drafter+pruner+corpus pattern** from `riir-games-quest/quest_grammar` but has **zero game-domain coupling**: no dep on `riir-games*` / `riir-game-sdk` / any game crate from ANY feature combination; no `riir-chain` dep (`riir-neuron-db` only transitive, via `riir-rag` under `latent_retrieval`); default build's only public dep is `katgpt-core` (ConstraintPruner trait + ternary SIMD matvec + `pick_domain`, Plan 558); two opt-in feature arms consume non-game sibling substrates (`ternary_inference` → `riir-engine` + `riir-gpu`, `latent_retrieval` → `riir-rag`); the `TernaryDraftModel` pattern is reimplemented locally (~200 LOC). riir-ai is for game runtime + engine, not developer tools.

### Plan 001 multi-domain architecture (2026-08-13)

Plan 001 generalized the framework from a clippy-only healer into a pluggable multi-domain framework behind the [`HealDomain`](src/domain.rs) trait — every domain shares the L0→L4 pipeline and owns its corpus + validator (Proposal 012's GPU kernel optimizer was absorbed as `kernel_opt`, Phase 3; the trait makes future extraction a move, not a rewrite). `cargo heal --domain auto` fans out across all feature-enabled domains with the router verdict as a per-span annotation, NOT a gate; each domain is still feature-selected and a feature-off domain is a hard error. Kernel proposers remain Phase 5 (deferred — concept-grade corpus). (Full record: `HISTORY.md`.)

### `kernel_opt`-specific layers (Phase 3)

1. `KernelExpertRouter` (`src/domains/kernel_opt/router.rs`) — modelless argmax over `katgpt_core::variable_rank_domain_expert::pick_domain<15, 8>` (8-dim kernel-signature activity vector, 15 per-category centroids); `build_activity()` is the consumer-facing constructor; zero allocation.
2. `KernelThermalRouter` (`src/domains/kernel_opt/thermal.rs`) — the Plasma→Hot→Warm→Cold→Freeze escalation (`#[repr(u8)] ThermalTier`); the scaffold returns Plasma (modelless-first mandate); Warm/Cold/Freeze require 4090 CUDA + LoRA (deferred to riir-train).

## Sibling-Repo Layout

```
/git/riir-clippy          ← this repo (clippy-healing pipeline)
/git/katgpt-rs            ← katgpt-core (NON-OPTIONAL: ConstraintPruner trait + ternary matvec)
/git/riir-ai              ← OPTIONAL dep (feature-gated arms only: ternary_inference →
                          riir-engine + riir-gpu, Bench 010; latent_retrieval → riir-rag,
                          Plan 005. Default build: NOT a dep)
/git/riir-train           ← OPTIONAL dep (only for L4 LoRA fallback, deferred)
/git/riir-chain           ← NOT a dep
/git/riir-neuron-db       ← transitive only (via riir-rag under latent_retrieval)
/git/riir-auth            ← OPTIONAL dep (account-key login substrate; kat_service →
                          riir-auth/account_key only, NOT github_login — Issue 081,
                          2026-09-09; src/kat_account.rs is the kat-facing facade)
```

Path deps in `Cargo.toml` assume the standard `/git/` sibling layout.

## Build Commands

```bash
# Default (clippy_lints domain + modelless path)
cargo check
cargo clippy --all-targets
cargo test --lib

# rust_perf domain (Plan 001 Phase 2)
cargo check --features rust_perf
cargo run --example perf_heal_demo --features rust_perf

# kernel_opt domain (Plan 001 Phase 3, 4090 DEFERRED)
cargo check --features kernel_heal
cargo run --example kernel_heal_demo --features kernel_heal

# Plan 005 latent retrieval + the cargo-heal CLI (opt-in; pulls riir-rag)
cargo check --features latent_retrieval
cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/

# With ruliology candidate search (L3)
cargo check --features ruliology_search

# All features (the pre-existing ternary_inference/riir-gpu failure was fixed
# in riir-ai commit 4bfe3b30 — relaxed attn_sub_on cfg gate to cubecl_runtime)
cargo check --all-features

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# The standalone-dep gap pin (no cargo, no siblings needed — the bare-clone CI tier)
./scripts/standalone_dep_gate.sh

# Re-run GOAT benches (see .benchmarks/001_goat_consolidated.md for measured results)
cargo bench --bench bench_001_l1_matvec
cargo bench --bench bench_002_l2_pruner_syn
cargo run --example ruliology_search_demo --features ruliology_search --release
```

### CI enforcement (2026-09-02, katgpt-rs Issue 701 R2 self-apply)

`.github/workflows/rust.yml` runs `scripts/ci_feature_guard.sh` — the same guard a developer runs, not a CI-only subset — plus a `standalone` job pinning the escaping-path-dep set (`scripts/standalone_dep_gate.sh`: 10 entries across 4 siblings since Issue 081 added riir-auth; private siblings via `SIBLING_REPOS_TOKEN`, hard-erroring when absent). Triggers are deliberately narrow (the workspace spending-limit convention): pushes to `main` and `workflow_dispatch` — the weekly `develop` rot-check cron was SUSPENDED 2026-09-09 (Actions spending limit; schedules evaluate against the default branch `develop`), commented in-file. Separately, `.github/workflows/release.yml` (Plan 105, Proposal 003) runs on `v*` tags (publish to the public dist repo + tap/bucket PRs) and `workflow_dispatch` (dry run); it needs `SIBLING_REPOS_TOKEN` (as rust.yml) plus `RELEASE_PAT` (the three public dist repos only — see Plan 105 T1.3). Toolchain installs from `rust-toolchain.toml` (never @stable); the Tier-2 `docker build --check` agreement degrades deterministically in CI via a dead `DOCKER_HOST`; the layer-5c wedge budget is 7200 s for the 4-core runner; first scheduled runs are the cost measurement (~300 min budget). (Full runner notes: `HISTORY.md`.)

## Feature Flag Discipline

- `default` = `["clippy_lints"]`: clippy domain + L0 corpus + L1 ternary drafter + L2 native pruner.
- `clippy_lints` (default-on): the clippy lint healing domain.
- `clippy_verify` (opt-in): lint-resolution verification via real `cargo clippy` re-run (slow, needs cargo+clippy on PATH). Implies `clippy_lints`.
- `rust_perf` (default-on): rust performance healing domain (126 rules measured by `--corpus-stats` 2026-09-10; the feature list's older "105"/"112"/"114"/"117"/"120"/"121"/"124" was stale — Pareto + alloc-count promoted).
- `rust_perf_verify` (opt-in): `cargo test` re-run for perf-rule verification. Implies `rust_perf`.
- `rust_perf_bench` (opt-in): the promotion gate — generated microbenchmarks with a deterministic alloc-count axis; GOAT PASS. Implies `rust_perf`.
- `kernel_heal` (opt-in): GPU kernel optimization domain (607 rules; 15 category experts + thermal router). NOT promoted to default — its GOAT gate (real corpus + full numeric validator) is pending.
- `kernel_verify` (opt-in): CubeCL compile check (STUB — delegates to the shared `RustAnalyzerPruner`). Implies `kernel_heal`.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search (pattern reuse, no dep; once per invocation, not per-tick). Implies `clippy_lints`.
- `latent_retrieval` (opt-in): real-file span-level latent retrieval via riir-rag; powers the `cargo-heal` bin (`--suggest` / `--fix` / `--domain`); gates `dep:riir-rag` (heavy tree — the default build stays light).
- `rustc_errors` (DEFAULT-ON since 2026-08-29): the `--fix-compile` driver domain; inert until invoked (compile-time cost only); dep-tree unchanged (`compile_oracle = []` — std + serde only).
- `docker` (DEFAULT-ON since 2026-08-28): Dockerfile healing domain; hadolint oracle (`docker_verify`); never a registry network call; the tier-2 `docker build --check` oracle rides `docker_buildcheck` (implies `docker_verify`) and never runs in the heal path.
- `sec` (opt-in): security-pattern healing domain (15 rules — 6 fixable + 9 suggest-only, incl. the Batch-131 InputValidation pair + the Batch-140 advisory-db `unbounded-read-into-vec`; the clippy boundary gate bans clippy auto-fixes in its territory).
- `self_evolve` (DEFAULT-ON since 2026-08-16): latent self-evolution — `LatentFixMemory` + `LatentSkillEvolution` + `EvolveRecorder`; the memory half runs wherever fixes are applied, the EMA learning half no-ops (one log line) without the `clippy_verify` oracle; opt-out `RIIR_HEAL_SELF_EVOLVE=0`.
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc).
- `l4_lora` (opt-in): the L4 generative fallback SEAM ONLY — trait + pipeline wiring, inert unless a fallback is installed via `HealPipeline::with_l4_fallback`. NOT promoted: `decline` is the default; the 3-condition reachability contract (no candidate / non-parsing / out-of-corpus-relevance) is enforced by `tests/g4_l4_composition.rs`.
- `clippy_lora` (opt-in): DEAD PLACEHOLDER — superseded by `l4_lora`; kept only to avoid breaking external `--features clippy_lora` invocations; do not add code behind it.
- `contrastive_scope` (opt-in): the input-scope gate at the `heal()` seam (`ScopeModel` over katgpt-core's `contrastive_scope`); declines out-of-scope input, bit-identical in-distribution.
- `ternary_inference` (opt-in): katgpt ternary generation on clippy prompts (inference arm; pulls `riir-engine` + `riir-gpu`).
- `dist` (opt-in, Issue 085): the `dist` domain — shipped-artifact build-config advice over name-matched `Cargo.toml` manifests (release-profile size knobs for bin crates + the workspace-member profile trap). SUGGEST-ONLY Phase 1; measured provenance 11.9 MB → 4.5 MB (-61%) on this tool itself. Second no-`syn` scanner (zero new deps); the `--domain dist` lane is suggest-citations only, never a manifest write.

Doctrine: the modelless path (L0–L3) is PRIMARY; LoRA (L4) is the conservative last resort and that is enforced in code, not by convention. **A syntactic check (`parses`) is not a relevance check** (the Issue 030 lesson) — any stage deciding "did the modelless path succeed?" must test whether the answer pertains to the input. Annotated rules carry `context_notes`, propagated end-to-end via `HealEntry::context_notes()`; the drafter surfaces the annotation, the consumer decides whether their context matches.

## CLI UX contract — the just-works surface (owner mandate 2026-09-08)

- **Bare `cargo heal` IS the dry run**: no mode, no paths → suggest review over `.` (or the given paths), zero edits, one banner line.
- **`cargo heal --fix` IS a real fix**: writes in place (`--write` implied) and compile-gates on `fix_verify` builds (opt out `--no-verify`). No paths → scans `.`.
- **`cargo heal --mine`** = the contribution onboarding (what mining is, the policy warning, ai.gist.rs, the login → heal → sync → account loop) **and the opt-IN consent write** (owner mandate 2026-09-08). **Auto-login (owner mandate 2026-09-10):** a missing account runs the zero-config login chain INLINE (`ensure_logged_in` — local key ops only, no network, no prompt; outcome printed via the shared `print_login_outcome` so `--mine` and `cargo heal login` describe events identically). `cargo heal login` stays the explicit/repair path (`--import-key`). **Auto-sync (same mandate):** the join completes in one command — `--mine` runs the FULL `cargo heal sync` (push + grant claim + burn drain + lease pull) at its end, behind the transport+latent_retrieval gates and a logged-in guard (sync exits on a missing account, which must never fail a completed --mine); re-run `--mine` or use `cargo heal sync` to push newer batches. **Auto-sync on every run (same mandate):** once consent is OptedIn, the bare dry run and `--fix` tails drain the mining batch themselves (`push_mining_batch(true)` in main's tail, BEFORE the burn drain — the 079 push-before-drain order; one line, ~5 s, never an exit-code change). `Unset` never auto-pushes — nothing silently leaves a machine that never joined (running `sync` IS the consent) — and `OptedOut` never pushes. The push half is extracted as `push_mining_batch(quiet, url, transport)` shared by `sync` (verbose) and the tails (quiet). **`--unmine` deliberately stays NOT a logout** — the account dir, key, balance and rewards are untouched (fully reversible); identity destruction would orphan the server-side balance forever, so `--unmine --purge` is the explicit escape hatch.
- **`cargo heal --unmine`** = the opt-OUT counterpart. Records machine-global consent = opted out (`mining_consent.json` in the account dir, `src/kat_mining_consent.rs` — DELIBERATELY UNGATED so it works in every feature build); `cargo heal sync` then refuses its PUSH half before building or signing anything, and still runs the lease PULL half (a read is not a contribution). Healing is UNCHANGED and was always local; the queued `.heal/outbox/` path is NAMED and never deleted. Idempotent, exit 0; `--mine --unmine` together is exit 2. **The three tiers are `Unset` / `OptedIn` / `OptedOut`, and `Unset` behaves exactly as the pre-consent build did — running `sync` IS the consent.** Never make the absent state a refusal; that breaks every existing user.
- **Auto burn-sync after every `--fix` run (logged-in, Issue 076 client half)**: the cumulative burn is reported to the service after the run finishes (`src/kat_burn_client.rs`) — account billing, NOT mining consent (`--unmine` does not stop it); ~5 s budget, one status line, never blocks or changes the exit code. A failed push **stacks offline** (git analogy: the `.heal/kat_burn_ack.json` watermark advances only on a 200-accepted response) and the next run or `sync` reports the FULL cumulative total (the server's monotone watermark makes replays no-ops); explicit `sync` drains the stack too (before the consent-gated batch push). Like every network surface here it rides `kat_transport` (ureq) — the push SHIPS: the release set has carried `kat_transport` since `v0.1.2` (`56bd9bf`, build + license jobs on the same set); only a bare `kat_service`-only dev build compiles it (and `sync`, and the consent heartbeat) to nothing.
- **The soft gate — depleted accounts refuse (Issue 075, owner decision 2026-09-08)**: a logged-in account whose LAST SYNCED server balance (the burn ack's `last_balance_micro`, never the local projection) is ≤ 0 and that has never contributed-and-synced gets scanning runs REFUSED at start (exit 1, all three refuel paths named — `--refill`, `--mine` + sync, or the payment page). The sticky grace is SERVER TRUTH ONLY (Issue 080, dated removal landed 2026-09-10 with fleet v3 live): the burn-ack v3 `contributed` field (written only from the authenticated burn-push response, beside `last_balance_micro`) is the sole input — the gate predicate is `ack.contributed == Some(true)`; `Some(false)` refuses and `None` (a pre-v3 ack/server) is simply NOT contributed (the balance leg then decides). The forgeable `contributed_once.json` gate input is FULLY RETIRED: the `kat_contribution` module is deleted, nothing reads or writes the file, and any copy a pre-removal install left on disk has zero effect. (History: the transitional truth-wins form `ack.contributed.unwrap_or_else(marker)` closed the trivial `Some(true) || marker` forge while the fleet was pre-v3; the removal closed the marker's residual `None` fallback.) Contributed accounts are never re-gated while depleted because mining payouts land at the 7-day epoch settle. Logged-out / offline / never-synced stays warn-only (no server authority → no honest gate). Mode-wide — every scanning run burns fuel (the bare dry run settles charge; measured), so gating only `--write` would be a sieve. **The refusal first OFFERS the contribute-now path (Issue 079)**: one y/N question on a TTY, or `RIIR_HEAL_KAT_PROMPT_YES=1` for agents/CI — yes runs `sync` inline; one accepted push lifts the gate for good; an explicit `--unmine` opt-out is never re-asked. Kill switches: `RIIR_HEAL_KAT_GATE=0`, or the shared `RIIR_HEAL_KAT_METER=0` billing plane. Predicate: `src/kat_soft_gate.rs`; CLI contract test: `tests/kat_soft_gate_cli.rs`.
- **`cargo heal --refill` — the dev/test faucet refill (Plan 117, 2026-09-09)**: the refuel path the soft gate names first. Composes shipped surfaces — the golden-pinned faucet POST (`faucet --request`'s wire; the DEPLOYMENT's `faucet_daily_cap_kat` is the faucet's gate, the client adds no host gate, the four ledger refusals surface verbatim), the wetrun's bearer crank POST (fires ONLY when `RIIR_HEAL_KAT_CRANK_TOKEN` is set — **the bearer IS the authorization**, prod never distributes one; fail-soft: a refused crank never fails the refill), and the public balance read — then writes the confirmed server balance into the burn ack (`record_server_balance`, server truth wins whatever the value, issue 080's lesson) so the gate lifts WITHOUT a new burn push. `--refill-wait <s>` bounds the settle poll (default 0 = one after-read). Exit 0 IFF the final synced balance is positive (explicit command — the burn push's never-blocks contract does not apply). Module: `src/kat_refill.rs` (transport COMPOSES `HttpPayreqTransport` + `HttpWetrunTransport`, zero HTTP re-derived); CLI contract test: `tests/kat_refill_cli.rs` (the `kat_burn_stack` mock-server shape). **Blocker CLEARED + SMOKE PASS 2026-09-10 — the owed item is DISCHARGED.** Server half first (idle-session verification): riir-dapps `dddbd03` resolved Issue 025 at the SOURCE (genesis re-scale, grant 100→100M KAT, ratios preserved) and funded the devnet treasury above the new 1e17 floor; Issue 035 resolved the settlement drain (worker `3dd1de63` live-verified at 100%, `bec3055`→`1caa0f6`). Client half (the 00:21 binary had been rebuilt WITHOUT the release feature set — `--refill` refused with the designed `exit 2`, and the same staleness silently disabled `sync`/burn-push/auto-sync while `--help` still ADVERTISED the flags): the global binary was rebuilt from HEAD `f9d6a89b1` with the release set (`default + latent_retrieval + kat_service + kat_transport + fix_verify`), built in an ISOLATED WORKTREE (`git worktree add`, clean HEAD — the batch-142 corpus WIP in the live checkout is NOT baked in) with `CARGO_TARGET_DIR=/tmp` (no lock contention with sibling agents), then copied over the symlink target; live smoke `--refill --refill-wait 45` (no crank token = faucet-only leg) **exit 0**: faucet request `b1bbd5a95c1cf1e3f88d3653bda8e5b9` opened for 10 KAT (daily-cap pin honored), crank skipped fail-soft, **kat-fuel 99,999,996 KAT (server truth, account `c79c93b8…`) written to the burn ack — the soft gate's precondition is lifted**. Standing caveat (the 00:21 class): any future bare `cargo build --release` of this crate OVERWRITES the symlink target with a default-features binary and silently re-stales `sync`/`refill`/burn-push — always rebuild with the release set (the release.yml feature list).
- **`cargo heal --version`** (also `-V`) prints the version and exits — any build, ungated.
- **`cargo heal --config-audit [PATHS…]`** (Plan 124, default `.`) — the config-hygiene REPORT (post-mining intake classes A+B): Default-impl fields defaulting to a const nobody field-reads while the const IS read elsewhere (class A — the knob that cannot be turned, invisible to clippy; probe-verified 1.98 zero-findings on the shape), and fields only ever asserted against their own Default value (class B — fake coverage; behavioral asserts against computed expectations correctly count as reads). Pure syn scan, zero new deps, ungated, suggest-only, ALWAYS exit 0 (report, not a gate); the detector sees only the scanned set (sibling/upstream consumers under-reported — stated in the footer). Validated: 511 real files / 151 candidates → 1 verified true positive (riir-chain `GuardVerdict.redirect`), 0 false positives; 15 fixture tests.
- **`cargo heal --frontier-report`** (Issue 089 T1+T2, 2026-09-11) — the learnability-frontier band REPORT, report-first by design law (R135/Bench 047: at real pool sparsity selection sophistication measured ≤0.04pp — no admission gate wires until the report proves band-mass is predictive). Per corpus rule: `r / f / n / p̂ / Beta-LCB₅` over the trajectory store + score history (LCB via `katgpt_core::best_belief_score` at the frontier ε), band-mass fraction in the header (band = `0 < p̂ ≤ 0.5`), saturation demotion CANDIDATES report-only (`p̂=1 ∧ n≥50`, the conservative proxy — per-attempt order is unreconstructible from cumulative counters; never deletes). Read-only, ungated (`src/frontier_report.rs`, pure stats); refuses exit 1 on no-store AND on population 0 — never a green zero. First measurements: riir-ai 0.0% band-mass (21 rules), this repo 5.6% (1 of 18) — T4's gate would have nothing to admit, which is why none is wired. Honest blocker: historical band-mass snapshots are unreconstructible (cumulative counters), so T4's GO evidence needs ≥3 FUTURE generations of snapshots.
- **`cargo heal --yes`** = non-interactive consent: every y/N prompt answers yes (the flag form of `RIIR_HEAL_YES=1`; the pre-existing `RIIR_HEAL_KAT_PROMPT_YES` still works for the 079 gate specifically). Agents/CI never need a TTY. It subsumes the old vote-only `--yes` (one flag, both meanings).
- **`cargo heal --mine` writes the default `.heal-sync-policy`** (v3: own-repo + standard source globs, `allow_redacted: false`) at the consent moment when the repo has none — deny-by-default stays (an existing policy is NEVER overwritten), but a fresh machine no longer hits `sync: 0 included, N blocked` with no remedy. When a sync IS blocked by `NoPolicy`, the sync output now names the exact file and the `--mine` remedy.
- **Service-host fallback chain (2026-09-09, repointed 2026-09-10 / Plan 023 S2)**: the three HTTP clients (lease/payreq/wetrun-read) retry a HOST-level failure (DNS, connection refused — `is_host_level_error`) once against the paired known host (`FALLBACK_SERVICE_URL` = the devnet subdomain ↔ `DEFAULT_SERVICE_URL`; a custom override URL is respected literally) and stick to it for the process lifetime. Timeouts/TLS/statuses are never failed over (a timed-out push may have landed). Born from v0.1.2/v0.1.3 shipping `heal.gist.rs` as default with no public DNS record; the retired dead host is GONE from the binary and the pair direction is default→devnet ONLY — a DEV client can never fail over onto mainnet (load-bearing since the flip: the default IS mainnet now), and a configured/env URL has no pair. Pinned by `fallback_pair_is_the_devnet_subdomain`.
- **`cargo heal config set --url <alias|url>` / `config get` (riir-dapps Plan 023 S3, 2026-09-10)**: the persisted service-url surface — the Solana-CLI model (`DEFAULT_SERVICE_URL` never moves; devnet is opt-in). Pinned alias map EXACTLY devnet/testnet/mainnet (case-insensitive full words; NO `localhost` alias — loopback is raw-URL-only and `http://` is loopback-only, g1); `config set` is LOUD (resolved URL + pinned-host tell or not-a-pinned-host warning with the local-artifact QR caveat + env-override trap, g2/g5); `config get` prints the ACTIVE url AND its source (env/config/default, g3); record = `service_url.json` in the account dir, 0600 (invalid/corrupt reads as not-set WITH a warning — never a silent money-URL fallback). Resolution: `RIIR_HEAL_KAT_SERVICE_URL` > config > default; it re-points the EXPLICIT money commands via `service_url_for_command()` (burn/mine/sync/refill/claims/pay/stake/vote/wetrun + the payment pages/QRs), while the ambient run-start lease phase keeps env-only semantics (a persisted config never silently onlines a heal run). Ungated module `kat_service_config.rs` (the consent-record pattern) — `config` works in every feature build. Fleet state 2026-09-10, post-swap: M3 + 4090 both `config set --url devnet` (devnet.ai.gist.rs hosts the moved fleet ledger) with devnet round-trips green.
- **Service environments (riir-dapps Plan 021 + Plans 023/024, FLIPPED 2026-09-10)**: the apex `https://ai.gist.rs` stays `DEFAULT_SERVICE_URL` forever — and since the 2026-09-10 domain swap it IS **MAINNET** (fresh ledger; the const never moved, the QR goldens never tripped); `devnet.ai.gist.rs` is its own worker (`[env.devnet]`, `riir-kat-service-devnet`) and hosts the fleet's MOVED ledger (85/2/24 keys BLAKE3-identical via the Plan 024 export/import move) — the devnet fleet's configured target (`config set --url devnet`); `testnet.ai.gist.rs` / `mainnet.ai.gist.rs` exist as separate worker deployments and are NEVER client defaults (a fresh install must never silently talk to mainnet — now load-bearing, since the default IS mainnet). The QR goldens pin the apex URL — touching either the default or the goldens is the issue-078 class of silent breakage. One host, one worker, one env (the devnet ALIAS fronting retired, `routes.rs` `c862f61`).
- **Sync prints the pipeline**: the server ingest report (rows_new/refreshed/terminal_skipped/recorded) plus one explainer line — rows enter the proof queue, credit at the proof pass, pay at the epoch settle, duplicate shapes earn nothing.
- `--write` still parses (no-op with `--fix`); `--suggest` stays the explicit review command; `--dry-run` belongs to `wetrun` only.
- ⚠ Docs predating this used `--fix` as the dry run — those flows WRITE now. Review = the bare form or `--suggest`.
- **Release feature set (v0.1.4+): `default + latent_retrieval + kat_service + kat_transport + fix_verify`** — `fix_verify` ships the documented `--fix` compile gate (and makes synced rows verified/Certified instead of Heuristic); it adds no new external crates (compile_oracle = std + serde only). release.yml builds and license-generates with exactly this set.
- **Release build profile (2026-09-09): `--profile dist`** (Cargo.toml `[profile.dist]`: lto fat + codegen-units 1 + opt-level "s" + panic abort + strip) — measured **11.0 MB → 4.3 MB (-61%)** on the arm64 shipping set (cargo bloat: 70% of .text is own-code corpus tables, so codegen knobs win, not dep removal). NOT `[profile.release]` on purpose — bench inherits release and the GOAT criterion benches must not eat opt-level "s". The LOCAL dev global binary stays on plain `--release` for drafter speed. Guarded by `scripts/binary_size_gate.sh` (a 6.5 MiB CEILING, fires upward only, provisional until all five matrix targets are measured; runs in the release package job before the leak scan; `--self-test` proves the detector).

## Documentation Shape

11-folder numbered `.docs/` book (sibling format): `01_orientation`, `02_corpus`, `03_drafter`, `04_pruner`, `05_clippy_verify`, `06_ruliology`, `07_examples`, `08_benchmarks`, `09_lessons`, `10_self_evolve`, `11_domains`. The top-level `.docs/README.md` is the index; each folder has a `README.md` index table. Files inside numbered folders have **no number prefix** (bare slugs — e.g. `crate_role.md`); add a doc by dropping `slug.md` in the right folder + one line in that folder's index table. `README.md` (repo root) is the build surface; `AGENTS.md` is the authoritative repo-local context.

## Numbering Discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and never reused**. Read `.issues/.highwater`, use `value + 1`, write back — same for `.plans/`, `.docs/`, `.benchmarks/`, `.research/`, and `.distill/` (the distill-program rolling resume state — edit-in-place per batch, NOT one file per batch). **One counter per directory, enforced:** every author reads the directory's `.highwater` and writes `value + 1` back in the same commit; a renumber consumes its target exactly like a fresh allocation. Covered by katgpt-rs `scripts/numbering_drift_sweep.py` (this repo is sweep-covered, not per-push-gated); the standing fix for a collision is renumber + citation rewrite, never a silent reuse.

Research-note shape: `.research/` notes are born-complete distillation records, not task trackers (no `- [ ]` checkboxes — pending work lives in `.issues/`, the SKILL.md mining queue, or the feeding plan). Every new note carries a `**Status:**` line — `RECORD` (batch landed, closed) · `DISTILLED — pending owner decision` · `NEGATIVE` (measured refutation); a later-filed delta gets a one-line status update pointing at the issue/commit.

## Branch

`develop` is the working branch. Commit directly on `develop` per the global rule.
