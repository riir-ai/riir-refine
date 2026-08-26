# Plan 031 — KAT infra: own-repo sweeper + temp container + R2 persistence tier

Expands [Proposal 005](../.proposals/005_kat_heal_service_network.md) rollout tasks **T2.0 (container half) + T2.1 (correction gates + R2 mitigation) + T2.6 (sweeper lane)** — and the new §R2 as the persistence tier — into executable work. No KAT-rail dependency (rides the AI-call surface + the healer, not the sync surface), so this can proceed independently of Phase 1.

**Honest sequencing rationale (bootstrap-first, the proposal's own discipline):** the local M3 idle-loop already sweeps whenever an agent session is open. Phase A (local cron) validates the loop at $0 and measures the real window lengths; the container (Phase B) is only worth its $5/mo plan gate + image pipeline once the loop is proven valuable AND 24/7 coverage matters; R2 (Phase C) only pays once cold-disk wakes exist to serve. Each phase's go condition is the previous phase's measurement.

**Dependency:** Issue 046 (USD hard cap) — **LANDED 2026-08-27, commit `8fce5b2`** (resolved + removed per the noise rule; see A1). No unattended LLM loop ran before it.

## Phase A — local bootstrap (no CF container, $0)

- [x] A1 Dependency: Issue 046 green (the cap enforced in `CfClient::generate`) — **RESOLVED 2026-08-27, commit `8fce5b2`** (`src/llm_spend.rs` + the gate in `generate`; G1/G2/G3 green; issue file removed per the noise rule).
- [ ] A2 **Local cron/launchd sweeper** (T2.6's local half): scheduled `cargo heal --fix --verify` runs over a TARGETED own-repo list (`-p <crate>` scope, span-class-filtered — never blind full sweeps); findings land on **review branches** (scoped per-repo deploy key, never the owner key, never direct `develop`); the never-wrong gates ride `--verify` as today. Deliverable: the launchd plist (or cron entry) + the targeting config + one week of measured runs (window length, findings/keeps/reverts per repo).
- [ ] A3 **Trainer-window local drain** (T2.0's local half): productize the Bench 049 harness into the drain loop — pull the local miss backlog (TrajStore) → dedup/priority → modelless pre-pass → LLM (capped, both neuron + USD bounds) → `fix_verify` + oracle strict-keep → commit trajectories + the transparency ledger row (neurons, model id, attempts/keeps/declines, est-USD vs cap). Runs daily via the same cron surface. Deliverable: the loop binary + the first week's ledger rows.
- [ ] A4 **Window measurement** (feeds Phase B sizing): per-run `cpuTimeSec` vs wall + peak RSS recorded; the T2.0 sizing rule (CPU work-constant, mem/disk per-wall-second) applied to pick the instance tier from DATA.

## Phase B — the temp container (deploy-once, cron-start, sleep-billed)

- [ ] B1 **The image**: Dockerfile carrying the prebuilt `cargo-heal` + the pinned toolchain layer (the A3 loop + `--fix --verify` needs rustc). **Dogfood gate: the image's Dockerfile is a `docker`-domain heal target** (Plan 030 T1.6 adoption candidate — the container eats its own cooking from day one).
- [ ] B2 **The Container class**: `onActivityExpired()` override holding through a drain (or the documented decision to die-and-resume); **commit-before-sleep** enforced structurally (git push + ledger/trajectory commit to R2/store BEFORE the sleep transition — local disk dies with the instance); SIGTERM graceful drain within the stop grace.
- [ ] B3 **Cranker extension**: the live `cloudflare/housekeeping` Worker gains the Cron Container pattern (`scheduled()` → `getContainer().start()`) + the **poll heartbeat** (each poll resets `sleepAfter`; doubles as liveness monitoring feeding the Issue 044 dashboard).
- [ ] B4 **Deploy + measured window**: real runs on the A4-chosen tier; cost observability via Issue 045's graphs; **cost gate**: the sweeper window's monthly overage stays ≤ the ~$4–14/mo band at the included-allowance low end; if a window blows the band, re-size UP (the work-constant rule) before considering cadence cuts.
- [ ] B5 **Secrets posture**: scoped per-repo deploy keys (write-access to target branches only) + the CF AI/OAuth creds; a woken container stays Worker-binding-reachable only (no public route).

## Phase C — R2 persistence tier (GATED on B4's cold-window measurement)

Opens only if B4 shows cold-compile wall-time dominating the window cost (the expected case for any `-p` scope with deps).

- [ ] C1 **Bucket layout + creds**: per-purpose buckets (`kat-cache-sccache`, `kat-artifacts`), least-privilege R2 tokens; lifecycle policy reaping stale cache classes (Standard storage — Infrequent Access is wrong for a hot cache: retrieval fees + 30-day minimums).
- [ ] C2 **sccache over R2** (preferred form): `SCCACHE_ENDPOINT` → the R2 S3 API in the image env; **measurement gate**: cold vs warm window wall-time on the same repo scope — the warm win must exceed the wake-time cache-pull overhead (expected: minutes saved vs seconds-to-a-minute pulled; R2↔Container bandwidth is free both directions on-CF).
- [ ] C3 **Registry-bundle bootstrap tier** (only if sccache fights back): tarball pull of cargo registry + git db on wake, delta-push before sleep; safe for the content-addressed registry, NOT for `target/` (absolute paths).
- [ ] C4 **Worker + R2 artifact serving**: built binaries + cache bundles served via the Worker → R2 `get()` path (egress-free by construction); extends to **NeuronVessel packs** only when Phase 3 opens. Cache objects are regenerable — git stays the source of truth.

## Close condition (honest)

If A2's week of local-cron runs shows the idle-loop already covers the sweeper's value (agents open most hours, findings trivial), Phase B defers with the measurement recorded — the local loop IS the sanctioned bootstrap shape per Proposal 005. If B4's windows sit inside the included allowances with cold compiles (small `-p` scopes), Phase C defers — nothing to cache. Record either verdict in this file.

## Ref

Proposal 005 §The daily trainer window + §The own-repo hourly sweeper lane + §R2 as the persistence tier · Issue 046 (cap, dependency) · Issue 045 (cost observability, live) · Issue 044 (dashboard) · [Bench 049](../.benchmarks/049_fixer_regate_poc.md) (the drain loop's provenance) · [Plan 030](030_docker_domain_phase0_batch1.md) (the B1 dogfood gate).
