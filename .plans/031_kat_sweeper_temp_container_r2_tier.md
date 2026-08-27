# Plan 031 — KAT infra: own-repo sweeper + temp container + R2 persistence tier

**Status:** Phase A COMPLETE (A1–A4, 2026-08-27 — mechanism + first measured runs; the week-of-runs deliverable accumulates async). Phase B (temp container) + Phase C (R2) remain gated per the sequencing rationale — each opens on the previous phase's measurement.

Expands [Proposal 005](../.proposals/005_kat_heal_service_network.md) rollout tasks **T2.0 (container half) + T2.1 (correction gates + R2 mitigation) + T2.6 (sweeper lane)** — and the new §R2 as the persistence tier — into executable work. No KAT-rail dependency (rides the AI-call surface + the healer, not the sync surface), so this can proceed independently of Phase 1.

**Honest sequencing rationale (bootstrap-first, the proposal's own discipline):** the local M3 idle-loop already sweeps whenever an agent session is open. Phase A (local cron) validates the loop at $0 and measures the real window lengths; the container (Phase B) is only worth its $5/mo plan gate + image pipeline once the loop is proven valuable AND 24/7 coverage matters; R2 (Phase C) only pays once cold-disk wakes exist to serve. Each phase's go condition is the previous phase's measurement.

**Dependency:** Issue 046 (USD hard cap) — **LANDED 2026-08-27, commit `8fce5b2`** (resolved + removed per the noise rule; see A1). No unattended LLM loop ran before it.

## Phase A — local bootstrap (no CF container, $0)

- [x] A1 Dependency: Issue 046 green (the cap enforced in `CfClient::generate`) — **RESOLVED 2026-08-27, commit `8fce5b2`** (`src/llm_spend.rs` + the gate in `generate`; G1/G2/G3 green; issue file removed per the noise rule).
- [x] A2 **Local cron/launchd sweeper** (T2.6's local half) — **mechanism landed 2026-08-27, commit `e373291`** (+ parser/path fix `035e598`): `scripts/kat_sweeper/{kat_sweeper.toml, run_sweep.sh, com.gist.rs.kat-sweeper.plist}`. Config: 2 own repos (riir-clippy `src/bin` hourly, riir-dapps `src/dapps` daily) — path-scoped, never blind (the plan's `-p <crate>` translates to a path scope; the healer CLI takes paths). Runner: pull --rebase → review branch `heal/<repo>-<yyyymmdd>` → `cargo heal --fix --write --verify` scoped → commit kept fixes ON THE BRANCH (never develop, never pushed; dirty tree = SKIP, never stash) → the measured run row. launchd: hourly, DISABLED by default (`launchctl load -w` is the documented opt-in; the daily-drain variant documented inline). **First measured runs recorded below (A4 table). The one-week deliverable continues async** — the rows accumulate in `.heal/kat_sweeper_runs.jsonl` (local, v1 schema) as the cron runs; the committed record is this table.
- [x] A3 **Trainer-window local drain** (T2.0's local half) — **mechanism landed 2026-08-27, commit `e373291`**: `src/bin/kat_drain.rs` (required-features `cf_workers_ai` — implies the redaction gate). The Bench 049 harness productized: TrajStore miss backlog (`fail_count > 0`, Withdrawn excluded) → dedup + priority (`fail_count × gap(2.0 off-corpus / 1.0 in-corpus)`, never-healed first, `(lint, shape_hash)` tie-break) → modelless pre-pass (the L4 contract — only misses reach the LLM) → redaction gate (every span, own-repo origin; all-blocked = exit 4) → one-call smoke (caveat #18(d); failure aborts BEFORE burn, exit 5) → `CfClient` with the qwen3 gates (budget sized for reasoning — `--max-new` default 1024; `REASONING_OVERFLOW` = ONE escalated retry then hard per-span decline; cap-reached = stop-the-day — the USD cap + both schemas + `/no_think` already live inside `CfClient`, not duplicated) → strict-keep `score_case` verify → Certified trajectories + the `llm_cost_ledger.jsonl` row (arm `DRAIN`, drain-specific fields v1). NEVER-WRONG by construction: the drain never writes repo files — keeps are Certified trajectories, rejects burn neurons only. `--dry-run` prints the queue offline (CI-able); 12 unit tests pinned in `ci_feature_guard.sh` L5c. **The LLM window itself is async-gated** (CF creds are exported interactively, not in this shell; the local backlog is currently failure-free — 32 rows, 0 misses — so today's window is the healthy empty queue). The first week's ledger rows accumulate as the daily cron meets creds + a non-empty backlog.
- [x] A4 **Window measurement** (feeds Phase B sizing) — **landed 2026-08-27**: `/usr/bin/time -l` wraps both runners (wall + user+sys CPU + peak RSS bytes, macOS layout); per-run rows append to `.heal/kat_sweeper_runs.jsonl` (v1). First measured numbers (2026-08-27, M3 Max, quiet box):

  | run | wall | CPU (u+s) | peak RSS | findings | keeps | reverts |
  |---|---|---|---|---|---|---|
  | sweep #1 riir-clippy `src/bin` (cold: index build + cargo check baseline) | 22.36 s | (old parser — not recorded) | 406 MB | 7 | 7 | 0 |
  | sweep #2 same scope (steady state — the expected hourly shape) | 0.36 s | 0.35 s | 38 MB | 0 | 0 | 0 |
  | drain `--dry-run` (empty queue — the current healthy state; release build 66 s one-time, NOT in the window) | 0.73 s | 0.01 s | 8.2 MB | 0 queued | — | — |

  Honest notes: (a) the week-of-runs table continues async — these are day-1 rows; (b) the first sweep's CPU column was lost to the parser bug fixed in `035e598` (wall + RSS are correct); (c) **the T2.0 sizing read so far: the steady-state window is CHEAP (sub-second wall, <40 MB) — a `basic`-tier container would already fit the no-op + small-fix shapes; only cold-compile windows (the 22 s shape) argue for more, and Phase B's go/no-go waits on the week's distribution of those.** (d) The 7 kept fixes landed on develop after author review (`6e1a726`) — the review-branch → human-review → merge loop demonstrated end-to-end.

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
