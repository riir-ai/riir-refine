# Plan 107 — Consumer-facing public surface (dist README economics + mining join + leaderboard)

**Status:** drafted 2026-09-07 — T1 decision gate open (owner); T2–T5 queued behind it; the release e2e itself runs under Plan 105 T3.2 independently.

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

- [ ] T1 (DECISION GATE, owner) — ship `kat_service` in the release binary?
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
- [ ] T2 — dist README consumer sections (edited in `gist-rs/cargo-heal`;
      the zero-`.rs` CI guard there is unaffected by `.md` edits):
  - [ ] T2.1 Feature table with honest per-domain status labels — pin every
        count from the corpus at write time (AGENTS.md's own role section
        disagrees with itself: rust_perf "112 rules" vs the feature list's
        "105 rules" — count, don't quote): clippy_lints (default, GOAT),
        rust_perf (default, Pareto-promoted), kernel_opt (opt-in, NOT
        GOAT-gated), docker (default, GOAT Ph0+1), rustc_errors (default,
        GOAT N=212), sec (opt-in). Plus a "what it is NOT" list: no trained
        weights, no network calls at heal time, no LLM fixer.
  - [ ] T2.2 TL;DR (install + use) already leads the README — keep it on
        top; extend Use with the `kat` quickstart, gated on T1 = GO:
        `cargo heal kat login` → `kat sync` → `kat account` (+ web wallet
        link). Gated OFF: one roadmap line instead.
  - [ ] T2.3 "KAT & the heal network" economics section + the mermaid
        flywheel (draft at T4): the 1-KAT-per-code-token burn, the free
        grant, decayed first-come mining rewards, redaction-gated sync,
        DAO-bounded parameters + never-lever paragraph, devnet status, the
        no-USD-promise sentence (caveat #7).
  - [ ] T2.4 "See your contribution" — leaderboard link (T3) + `kat account`
        output explainer + the web wallet URL (kat-service `/wallet`).
  - [ ] T2.5 Privacy & data posture: local-first (meter, charge ledger, and
        self-evolve trajectories live under `.heal/` on the machine);
        `kat sync` is the ONLY egress and it sends redacted,
        content-addressed spans under the sync data-license, opt-in;
        kill-switch `RIIR_HEAL_KAT_METER=0` disables meter + charge.
  - [ ] T2.6 FAQ / troubleshooting: the documented divergence classes (the
        healer deliberately declines some fixes — link the cargo-heal skill
        table's public-safe subset), update + uninstall per install channel,
        platform matrix (the 5 release triples), where release notes live,
        support channel (dist repo issues).
- [ ] T3 (riir-dapps — file the `.issues/NNN` there at execution) —
      `GET /leaderboard` on kat-service: epoch-scoped rank projected from
      ledger state (mining credit + grants), pseudonymous account ids, cheap
      cache headers, no PII, no bearer; optional per-epoch static JSON
      snapshot mirrored into the dist repo for zero-JS embeds; linked from
      `/wallet` and the `/` service-info page. The ops dashboard stays admin.
- [ ] T4 — mermaid draft for T2.3 (GitHub renders mermaid in READMEs):

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
- [ ] T5 — record + sync: bench/doc row for the consumer-surface verdict
      (final README link, leaderboard link, T1 outcome), doc-sync touch,
      HISTORY row.

## Non-goals

- No LLM-fixer marketing (Proposal 005 caveat #2 stands until the
  leak-posture sign-off).
- No new public repo or marketing site — the consumer web is the dist README
  + kat-service public routes; the ops dashboard stays admin.
- No corpus/method disclosure (Proposal 003 capabilities-not-methods).
- No USD pricing or value promises (caveats #7 / #12 / #15).
