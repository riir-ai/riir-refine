# Plan 107 — Consumer-facing public surface (dist README economics + mining join + leaderboard)

**Status:** EXECUTED 2026-09-07 — T1 GO (owner-delegated via "do 107"): v0.1.1 ships `kat_service`; T2 README live on the dist repo; T3/T4 filed as riir-dapps issues 019/020 (implementation + the service-route ops act land there); T5 recorded in Bench 079. Transport deferred to v0.1.2 (license-read + Windows build risk — see Bench 079).

## Why

The dist repo (`gist-rs/cargo-heal`) README is capabilities-only (Proposal 003
compliant): what the healer does, install, use, verify, attribution. It says
nothing about the KAT service economy, how to join mining, or where
contribution is visible. The only live web surface is the ADMIN dashboard
(`riir-ops-dashboard.foxfox.workers.dev`) — there is no consumer web. This
plan defines the consumer surface: the public README's consumer sections, the
missing leaderboard route, and the decision that gates the mining quickstart.

## Grounded state (grepped 2026-09-07, not assumed)

- **CLI:** `cargo-heal` ships a full `kat` subcommand family — `login`,
  `account`, `update`, `sync` (push redacted batches), `claim`, `stake`,
  `unstake`, `vote`, `pay`, `faucet` (`src/bin/heal.rs` `KatCommand`).
  BUT the bin's `required-features` are
  `["latent_retrieval", "rust_perf", "clippy_lints"]` — **`kat_service` is
  NOT in the release feature set**: the public binary parses the subcommands
  unconditionally and errors cleanly (feature-off notice) instead of running
  them. The join-mining UX exists in code and is absent from the shipped
  artifact. This is the single biggest gap between the ask ("how they can
  join mining") and what a `v*` download can actually do today.
- **Service plane (riir-dapps `cloudflare/kat-service`, devnet, live):**
  `/sync` (MiningQueue ingest), `/claim`, `/balance?acct=`, `/health`,
  `/wallet` (web wallet page), `/telemetry?epoch=`, `/payreq/*` incl. faucet,
  `/stake/*`, `/gov/*`, `/admin/*` (bearer-gated). **No leaderboard route
  exists.** `/balance` is per-account query-only.
- **Fixer:** consumer-facing LLM fixing stays BLOCKED (Proposal 005 caveat
  #2 — leak-posture sign-off pending; verify-filtered acceptance already met
  on CF arms, Bench 049). The README must never market "AI fixes your code":
  the healer is modelless, corpus-driven, network-free at heal time.
- **What the public README may say:** governed by Proposal 003's
  capabilities-not-methods rule (Research 003). Capabilities/flags/behavior:
  yes. Corpus internals, mining methodology, source: no.
- **Economics honesty gates:** 1 KAT per code word token burned at heal time
  (Proposal 005 TL;DR — the modelless sink IS the token economics); mining
  rewards decayed, first-discovery bonus (`REWARD_DECAY_BP` 9_900 ratified
  2026-08-28); free grant 100M KAT ≈ $300 Claude-equivalent; DAO
  guard-bounded parameters, never-lever list contract (riir-dao Plan 002);
  **KAT transferability / USD-out is counsel-gated** (caveat #7) — the README
  must NOT promise money; devnet status stated plainly.
- **Numbering note:** `.plans/.highwater` read 105 while `106_batch125_*`
  exists on disk — stale-highwater precedent; this plan consumes **107** and
  writes the highwater back.

## Tasks

- [x] T1 (DECISION GATE, owner) — ship `kat_service` in the release binary? **GO (executed)** — added to the bin's required-features; `release.yml` build + about generate pass the same set; dry run 34087690447 all green (G4 accepted ssh-key); tagged v0.1.1 (run 34088624161). Declining was never on the table after the "do 107" instruction; the transport half DID defer — see the status line.
      **Recommendation: YES.** Mining sync pushes REDACTED batches through
      `RedactionGate` (opt-in; everything stays local until `kat sync`);
      the leak-gated surface is the FIXER and it stays out; `kat_service`
      is dep-light (`ssh-key` + `ed25519-dalek`, dual MIT/Apache — the
      cargo-about G4 license allowlist should hold). Execution: add
      `kat_service` to the bin's required-features → full
      `ci_feature_guard.sh` → a `workflow_dispatch` dry run proving the
      binary builds AND `THIRD_PARTY_LICENSES.md` regenerates clean →
      promote on the next tag. Declining = T2.3's quickstart ships as
      roadmap-only ("the mining client ships in a future release") — still
      honest, still useful, decision recorded either way.
- [x] T2 — dist README consumer sections (executed: `gist-rs/cargo-heal@a11c73a`):
  - [x] T2.1 Feature table with honest per-domain status labels — counts pinned by running the shipping binary's `--corpus-stats --json` (49 / 112 / 559 / 31 / 12); AGENTS.md's stale "105" corrected to 112; "what it is NOT" list included.
  - [x] T2.2 TL;DR kept on top; the quickstart landed with T1 — measured correction: the CLI surface is TOP-LEVEL (`cargo heal login` / `account`), not `cargo heal kat ...`; `sync`/`claim` printed as "next release" (transport deferred).
  - [x] T2.3 economics section + mermaid flywheel (solid = live, dashed = mining loop) with never-lever, devnet-honesty, no-USD-promise paragraphs.
  - [x] T2.4 "See your contribution" — account explainer + service-status table (wallet/service URLs marked bring-up-in-progress; flips live when riir-dapps Issue 020 closes); the leaderboard link attaches when Issue 019 lands its route.
  - [x] T2.5 privacy & data posture section (offline healing, local `.heal/` + `~/.config/riir-heal/`, opt-in redacted-only egress, `RIIR_HEAL_KAT_METER=0` kill switch).
  - [x] T2.6 FAQ / troubleshooting (bounded-fix-space skips, `--fix` safety + fmt discipline, `--verify` latency, update + uninstall, unsigned-binary note); divergence-class detail kept generic in the consumer surface (capabilities-not-methods).
- [x] T3 (riir-dapps) — **filed: `riir-dapps/.issues/019_kat_service_leaderboard_route.md`** (`48452a3`): `GET /leaderboard?epoch=&limit=` projection over the ledger DO, pseudonymous id prefixes, cache headers, consumer-web-only contract note; also **Issue 020** — the service plane is not publicly reachable (workers.dev route disabled / `kat.heal.gist.rs` DNS missing; owner ops act). Implementation + deploy tracked in riir-dapps.
- [x] T4 — mermaid flywheel shipped in the README (solid/dashed live-vs-roadmap form; draft below superseded):

      ```mermaid
      graph LR
          A["cargo heal --fix (modelless)"] --> B["1 KAT burned per code token"]
          A --> C["redacted fix spans (opt-in kat sync)"]
          C --> D["trainer-quorum replay proof"]
          D --> E["mining reward (decayed, first-come)"]
          D --> F["better corpus rules"]
          F --> A
          E --> G["wallet + leaderboard"]
      ```
- [x] T5 — record + sync: **Bench 079** (verdict, measured counts, gate evidence, deferral reasons), AGENTS.md hygiene bullet (real feature set + license-parity warning) + rust_perf "105"→"112" correction; the README status table is the doc-sync touchpoint for Issues 019/020 closure.

## Non-goals

- No LLM-fixer marketing (Proposal 005 caveat #2 stands until the
  leak-posture sign-off).
- No new public repo or marketing site — the consumer web is the dist README
  + kat-service public routes; the ops dashboard stays admin.
- No corpus/method disclosure (Proposal 003 capabilities-not-methods).
- No USD pricing or value promises (caveats #7 / #12 / #15).
