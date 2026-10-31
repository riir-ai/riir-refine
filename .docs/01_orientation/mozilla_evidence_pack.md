# Mozilla evidence pack — cargo refine (issue 190 T3)

**Date:** 2026-10-31 · **Collector:** zed-m3-idle-ev190 (agent) · **Feeds:** issue [190](../../.issues/190_mozilla_grant_readiness_gaps.md) T3 → Proposal 025 application packet.
**Provenance:** GitHub REST API, unauthenticated read, 2026-10-31 ~15:1x UTC (star/fork/download counters are point-in-time). Internal figures read from tracked files at the cited paths. No telemetry exists anywhere in the product — every number below is a durable public counter or a tracked record.

## 1. Public distribution surface

| surface | URL | stars | forks | open issues | since |
|---|---|---|---|---|---|
| cargo-refine (releases + installers) | github.com/gist-rs/cargo-refine | 1 | 0 | 0 | 2026-09-07 |
| homebrew-tap | github.com/gist-rs/homebrew-tap | 0 | 0 | 0 | 2026-09-07 |
| scoop-bucket | github.com/gist-rs/scoop-bucket | 0 | 0 | 0 | 2026-09-07 |
| katgpt-rs (public upstream primitives) | github.com/katopz/katgpt-rs | 140 | 27 | 0 | 2026-05-04 |

### Release download counts (releases API, cumulative per asset)

13 releases from v0.1.0 (2026-09-07) to v0.3.5 (2026-10-30); **201 total asset downloads**.
Every release ships 5 platform archives (aarch64/x86_64 macOS, x86_64 Windows, aarch64/x86_64 Linux musl) + SHA256SUMS + licences.

| release | date | downloads | | release | date | downloads |
|---|---|---|---|---|---|---|
| v0.1.0 | 09-07 | 12 | | v0.3.0 | 10-11 | 15 |
| v0.1.1 | 09-07 | 19 | | v0.3.1 | 10-12 | 14 |
| v0.1.2 | 09-08 | 11 | | v0.3.2 | 10-17 | 6 |
| v0.1.3 | 09-08 | 15 | | v0.3.3 | 10-17 | 15 |
| v0.1.4 | 09-09 | 21 | | v0.3.4 | 10-29 | 14 |
| v0.2.0 | 10-01 | 20 | | v0.3.5 | 10-30 | 12 |
| v0.2.1 | 10-05 | 27 | | | | |

**Undercount note (honest):** brew/scoop installs bypass release assets (no telemetry anywhere), so 201 is a floor on installs. Windows v0.1.4 (6) + v0.2.1/v0.3.x archives show the multi-OS spread is real, not one platform.

## 2. Internal usage evidence (tracked records)

- **Heal-rate ledger**: `.benchmarks/score_history.jsonl` — **209 recorded score-bench runs** since the score-bench landed; latest entry (2026-10-29): **heal_rate 96.4%** of clippy findings auto-fixed, **parse_safety 1.0** (every write re-parses), **decline_correctness 1.0** (out-of-capability edits declined, never guessed), created-warning rate 1.8%.
- **Corpus standing counts** (AGENTS.md §Role, 2026-10-31): clippy 112 lints · rust_perf 289 rules · kernel_opt 1,051 rules · sec 31 · docker 31 · shader_perf 8 · dist 7 — every rule standalone-compiled AND executed before entry.
- **Batch cadence**: rust_perf at Batch 300, kernel_opt at Batch 323 (both 2026-10-31) — a rule-mining batch lands on a sustained near-daily cadence (batch history: `HISTORY.md` ledger).
- **Adoption verdict**: production default dev-loop tool since 2026-08-24 (AGENTS.md "Production adoption — GO", live trial); mechanical clippy findings across the workspace route to the healer FIRST, manual second — the convention is written into every consuming repo's AGENTS.md.
- **Deployment scale**: the default lint-healing tool across a **32-repo production workspace** (game runtime, chain, storage, training-methods, serving) — the consumers are private; the scale claim is attested by the workspace's tracked boundary contract (`katgpt-rs/AGENTS.md` repo roster).

## 3. Deployment story + links

- One-command install on 3 OS families: `install.sh` / `install.ps1` + verified binary self-update (`cargo refine upgrade`, SHA256SUMS-checked) — github.com/gist-rs/cargo-refine.
- Package-manager lanes: `brew tap gist-rs/tap` (github.com/gist-rs/homebrew-tap), scoop bucket (github.com/gist-rs/scoop-bucket).
- The product is **local-first and offline in its heal path**: modelless drafter + corpus, no code egress without explicit consent, no account required to run (KAT contribution plane is opt-in).
- Honest gap: **no public consumer repos** — the workspace consumers are private, so external verification today is the install surface + counters, not consumer code in the open.

## 4. Reading for the T10 no-submit gate (2026-11-17)

External user/contributor signal available TODAY: 201 release downloads across 5 OS targets + 1★ on the dist repo + the upstream katgpt-rs community surface (**140★ / 27 forks**, actively pushed daily) as adjacent-but-distinct evidence. This is a real but thin signal — T4 (pilot recruiting, owner-voiced) is the lever that converts "downloads" into "users we can name". The packet should quote the floor honestly and lean on the 209-run heal-rate ledger + the 32-repo internal deployment as the depth dimension.
