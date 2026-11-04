# Plan 331 — Mozilla outbound fix-PR wetrun (uniffi-rs + sccache): scan → gate → heal → verify → PR-ready

**Renumbered:** 329 → 331 on 2026-11-03 (owner-directed collision fix; first-allocation loss to `329_engine_mozilla_methodology_port` — that file landed 3.5 h earlier, `ee0a6901` 11-02T13:08+07 vs this file's `531b83af` 11-02T16:41+07; bench 127 + issue 198 citations moved in the same commit). The claim line below is unchanged.

**Status:** Claimed: zed-m3-mozilla-wetrun, 1793602400, expires 2026-11-04T06:30Z — the push is the arbiter

Executing [Proposal 022](../.proposals/022_outbound_fix_pr_campaign.md) (outbound fix-PR campaign) as a **manual wetrun** against the Mozilla quartet the owner named ([Proposal 025](../.proposals/025_mozilla_human_scale_ai_incubator.md) evidence: real upstream PRs are the grant's "product-community fit" proof, the self-graded WEAK criterion). The `harvest` CLI feature (Proposal 022 T1.x) does NOT exist yet — this wetrun applies the pipeline's GATES BY HAND and defers the feature to its own later plan. Owner-directed 2026-11-02: "wetrun first" (the real sends, not more planning); forks land in the `riir-ai` GitHub org; gh auth = the owner completes the device-flow login, sends are owner-approved per the 022 account rule.

## Lane separation (the owner's mining question — answered)

**Mining is NOT a prerequisite.** The quartet distill verdicts already exist and are STAGED in the INBOUND lane: Research 361 (uniffi-rs FFI-hygiene → sec, YES B−) + Plan 327, Research 362 (sccache io/cache/process → rust_perf, YES B−) + Plan 328. Neither claims a live lease. This plan is the OUTBOUND lane: healer → fixes → upstream PRs. Per Proposal 022's fusion lineage the lanes only couple as: **heal misses found here → `.distill/001_mining_queue_snapshot.md` §"Post-mining intake"** (after the heals, not before). No new mining run is owed by this wetrun; Plans 327/328 proceed (or not) on their own queue order.

## Targets (curated per Proposal 022 Phase 1 criteria: permissive license, active, no CLA wall)

| repo | pin | license | MSRV/toolchain | activity | verdict |
|---|---|---|---|---|---|
| mozilla/uniffi-rs | `ef9625b6` | MPL-2.0 | toolchain pin **1.94.0** (clippy component) | 2026-10-28 | **Wetrun target** — active upstream, real fix space |
| mozilla/sccache | `71d107e0` | Apache-2.0 | `rust-version = 1.91.0`, edition 2024, CI clippy `-D warnings` (allow: type_complexity, new-without-default) | 2026-11-01 | **Wetrun target** — but note T-scc below |
| mozilla/rust-code-analysis | `37e5d83c` | MPL-2.0 | edition 2024 | **dormant since 2026-01** | Scan-only — a dormant repo must not burn the one-PR-per-repo-ever shot |
| mozilla/supply-chain | `e6648794` | — | — | — | **Skip** — data-only (0 .rs files, sibling distill confirmed) |

Forks: `github.com/riir-ai/<repo>` (org exists; creation at SEND time, gh authed, per fork). Branch naming `refine/<rule>` per Proposal 022. Rate ceiling: **≤3 PRs/day total; one PR per repo, ever; never re-PR after a close** (022 §"The proposed design").

## The sccache problem statement (T-scc — read before ANY fix work)

The sibling distill (Research 362) verified that sccache's `posix_spawn` fallback concern is **ALREADY FIXED** at the pin: `mock_command.rs:260–282` carries the `share_jobserver` opt-in flag with the mechanism comment, and the callers that need fd inheritance call it (`:707`). The "A-graded GOAT lead" is therefore **the upstream repo's own landed decision — NOT an open fix**. Any PR built on it would be a duplicate-of-their-own-code embarrassment and a campaign killer. Consequence: **sccache enters the wetrun ONLY if the scan finds genuinely open, flagged-by-their-own-clippy findings** — otherwise sccache's day-1 slot stays empty. Honest expectation recorded up front: sccache runs CI clippy `-D warnings` on every push, so the likely finding count is ZERO and the correct outcome is "nothing to send", which the campaign counts as a pass (the gate worked), not a failure.

## Verified open fix space (uniffi-rs — coordinator re-verified at the pin 2026-11-02)

1. **`waker_wake`/`waker_drop` via `waker_clone` flag-restore round-trip** — `uniffi_core/src/ffi/rustfuture/future.rs:245–292` re-verified verbatim this session: `waker_clone` uses `Arc::increment_strong_count` (borrow, pointer unchanged); `waker_wake`/`waker_drop` call `recreate_arc` = `Arc::from_raw` (consume) — so a second wake on the same waker runs `from_raw` against an already-reowned refcount. Mitigated today by every executor cloning the waker before each wake (a documented `RawWaker` contract). Fix shape: wake arms take the borrowed path (`&*ptr` like `waker_wake_by_ref`, which already reads the scheduler through the borrowed pointer). Semantics-neutral (wake is `&self`), deletes the latent double-consume hazard + per-wake refcount pair. Single-file, ~8 lines. **Lead candidate for PR #1** (one lint family = this one hazard; manual semantic fix, healer-cited as disclosure only).
2. Research 361's remaining candidates (ManuallyDrop out-param, nested catch-unwind, null-sentinel conjunction, callback-outside-lock) are **style/hardening-grade, not flagged-by-clippy defects** — cold PRs would read as opinion. EXCLUDED from the wetrun; they stay mined into the sec corpus by Plan 327 where they belong.

## Gates (from Proposal 022, applied by hand — binding)

- **G1 per-PR:** created-warnings == 0 (external bar ZERO; local 1.8% is not the bar); compile green at the TARGET's toolchain (uniffi: 1.94.0 via its rust-toolchain.toml — our 1.99 workspace pin must not leak in; verify `cargo +1.94.0 clippy` if the toolchain is installed, else document the gap); diff contains ONLY the healer-relevant spans (no reformat, no drive-by edits); uniffi's own test suite green on touched crates.
- **G2 campaign:** accept rate is the metric (merged ÷ (merged+closed), 90-day window) — measured by the outcome poller AFTER sends; day-1 continuation floor N/A (first sends).
- **G3 no-regression:** no riir-refine code changes in this wetrun (manual ops lane) → the release set is untouched by construction. Docs/records only.
- **Rate ceiling:** ≤3 PRs/day; the send queue file below is the owner-review artifact; every send is owner-approved (022 account rule).

## Bench discipline ("full bench before/after" — the owner's requirement)

- **BEFORE (T2):** the scan inventory per repo (findings by rule, posture, MSRV/edition gate results) — frozen into `.benchmarks/126` + the queue file. This is the wetrun's "before" state: what the healer found, gated.
- **AFTER (T5):** per candidate fix: created-warnings count (must print ZERO), compile status at their toolchain, diff-span proof (only intended hunks), test status. Frozen into the same bench record.
- **Campaign metric:** PR URLs + outcomes recorded in the bench record; the accept-rate stratum updates when Mozilla responds.

## Tasks

- [x] T1 — Clones at pins (`.raw/mozilla-heal/`, 2026-11-02: uniffi-rs ef9625b6, sccache 71d107e0, rca 37e5d83c, supply-chain e6648794) + pins recorded in this plan
- [x] T2 — BEFORE scan: per-repo `cargo clippy --message-format=json` at THEIR posture (sccache: their CI line `--all-targets -- -D warnings -A type_complexity -A new-without-default` @ 1.91-target; uniffi: workspace clippy @ 1.94.0) + rust-code-analysis inventory (scan-only) → `.benchmarks/127_mozilla_wetrun/scan_before/` — **RESULT: ALL TARGETS CLIPPY-CLEAN at their own posture** (uniffi 0/598 artifacts @ 1.94.0, sccache 0 @ their CI flags, mp4parse 0, rca dormant-skip, supply-chain data-only-skip). Pedantic sweeps recorded but excluded by the their-posture gate. See `.benchmarks/127_mozilla_wetrun/README.md`
- [x] T3 — Gate + pivot: mechanical fix set EMPTY everywhere (the 022 gate worked). Pivoted to the issue-driven lane: **mozilla/sccache#2975** (CACHEDIR.TAG, fresh, additive, zero-risk). sccache confirmed as the day-1 PR; uniffi day-1 = no PR (honest empty)
- [x] T4 — HEAL sccache#2975 on branch `refine/cachedir-tag` @ `b97bcc0`: 80 insertions in `src/lru_disk_cache/mod.rs` (tag writer + init write + scan-loop exclusion + 2 unit tests). G1 verified at THEIR MSRV 1.91.0: 598 lib tests green (+2 new), their exact CI clippy flags → 0 findings (created-warnings ZERO), fmt clean, single-file diff. Design constraints found in-source: tag must bypass the LRU (eviction would delete it) and `can_store` (tiny caches would delete it)
- [x] T5 — SEND **LANDED 2026-11-02 ~17:06Z**: owner completed the third device-flow window (`2525-AD3A`, ~4 min; scopes `gist, read:org, repo`) → fork https://github.com/riir-ai/sccache created → `refine/cachedir-tag` @ `b97bcc0` pushed → **PR https://github.com/mozilla/sccache/pull/2977 open** (1 file, +80/−0, refs #2975; API-verified). uniffi-rs: NO day-1 PR (clean tree; #3020 pile-on needs owner call)
- [-] T6 — Outcome poller + accept-rate stratum (weekly review ritual; `.benchmarks/127` updated on merge/close) — **OWNER VERDICT 2026-11-02 RECLASSIFIES #2977 as goodwill-only (see bench 127 ledger): it does NOT enter the accept-rate denominator; the poller now tracks it as a plain upstream contribution. The campaign's next send must be evidence-bearing: healer-found perf mechanism → microbenched AT the target repo → measured delta + box provenance in the PR, or not sent. POLL 2026-11-04: OPEN (head `903f0be` unchanged since the round-5 reply; no maintainer activity since).**
- [x] T6a — The corrected material hunt (owner-verdict follow-through): sccache **#2968** ("Avoid repeated read-timeout waits when a cache backend is unhealthy") is THE bench-measurable perf candidate — maintainer-requested, issue-author-measured (stalled builds 9.0s vs 0.77s healthy warm = 11.7×; 32 timeouts per build; waits accumulate down the dep graph AND recur in the same daemon), and the mechanism surface is small and real: `MissType::TimedOut` (compiler.rs:753) already exists as a distinct signal, `ServerStats.cache_timeouts` (server.rs:1817) already counts it, and nothing consumes it — a daemon-scoped backend backoff (e.g. after N consecutive timeouts, shed the remote-get attempt for a cooldown window; recovery = the timeout not firing or an explicit probe) is a ~50-line change at ONE seam. The issue author explicitly asked "would backend-scoped backoff be appropriate… I'd like to understand the preferred behavior before proposing an implementation". **T6a now binds the FOUR SEND GATES (Proposal 022 §"The four send gates", owner verdict 2026-11-02): G-PR1 GOAT bench at their repo (before/after, their toolchain, pinned harness, box provenance) → G-PR2 Claude verdict AGREE (no slop, gain claim sound) → G-PR3 SVG before/after chart in the PR → G-PR4 owner-fingerprint send (kat-secret exec; `gh-outbound-pr` token owner-sealed at first use). ALL FOUR or nothing sends.**
- [x] T7 — Misses → post-mining intake line in `.distill/001` (the lane coupling, AFTER the heals) — **DONE: P113 (the upstream-state-check template gap) + P114 (the walk×strict-parse sec candidate, surfaced by our own round-4 CI failure) both in §Post-mining intake (2026-11-02/03); the issue-198 acceptance row ticked 2026-11-04**
- [x] T8 — Close-out: HISTORY.md record + Proposal 022/025 evidence lines (PR URLs) + `.raw` clones removed per the ephemeral law — **EXECUTED 2026-11-04: HISTORY record; Proposal 022 Status + Proposal 025 self-grade evidence lines; `.raw/mozilla-heal` (5 clones + target dirs, 16 GB) removed — the sccache clone verified at `903f0be` == fork/refine/cachedir-tag == the PR head before removal; the other four clones clean. The PR poll (T6) stays the open thread**

## The send-queue file (owner-review artifact — `--harvest-send` semantics by hand)

| # | repo | branch | one-liner | status |
|---|---|---|---|---|
| 1 | mozilla/sccache | `refine/cachedir-tag` @ `b97bcc0` | CACHEDIR.TAG per the cache-directory tagging spec (fixes their #2975); 80 insertions, 1 file; MSRV 1.91.0 gates green (598 tests, 0 clippy findings at their CI flags, fmt clean) | READY — awaiting owner approve + gh auth |
| 2 | mozilla/uniffi-rs | — | NO day-1 PR: tree clippy-clean at 1.94.0; the engaged fix space (#3011 via PR #3020) is another contributor's CHANGES_REQUESTED PR — piling a competing PR needs an owner call | HELD |

## Send mechanics (post-login, one shot)

```sh
gh auth status  # owner completes device-flow login first
gh repo fork mozilla/sccache --org riir-ai --clone=false
cd .raw/mozilla-heal/sccache
git remote add fork git@github.com:riir-ai/sccache.git
git push -u fork refine/cachedir-tag
cd /Users/kat/git/riir-refine
gh pr create --repo mozilla/sccache --head riir-ai:refine/cachedir-tag \
  --title "Tag the disk cache directory with CACHEDIR.TAG" \
  --body-file .benchmarks/127_mozilla_wetrun/pr_body.md
```

PR body draft: `.benchmarks/127_mozilla_wetrun/pr_body.md` (022 template:
what/why/how-verified + Fixes #2975; no aggregate numbers; ⛔ NO
attribution/disclosure/“Generated by” block — owner verdict 2026-11-02:
AI-slop + self-promo, banned until the grant lands).

## Cautions

1. **The uniffi waker lead was WITHDRAWN during preparation** — re-derivation at the pin showed the `waker_wake`/`waker_drop` consume-path (`Arc::from_raw`) is the RawWaker contract working as designed (the vtable wake/drop calls take ownership of one strong count; `waker_wake_by_ref` is the borrowed arm). A "borrowed wake" fix would LEAK one count per `Waker::wake()`. The distill Research 361 framing described the contract, not a defect. Lesson recorded in `.benchmarks/126`: re-derive every semantic lead at the pin against the API contract before scheduling a PR; clippy-clean trees have already survived their own review.
2. **Do not let our workspace's 1.99 toolchain touch their tree** — all builds in the clones ran under the repo's own toolchain files (`rustup toolchain install 1.91.0` done for sccache); `CARGO_TARGET_DIR` inside each clone (sccache target-wetrun ≈ 2.7 GB, cleaned at T8).
3. rca stays scan-only (dormant); supply-chain skipped (data-only). One-PR-per-repo-ever is spent ONCE, on the strongest candidate.
4. The PR body is 022 §"The proposed design" verbatim structure: what/why/how-verified + disclosure + repro + "Happy to rework or close." — no logo until the marketing assets exist, no aggregate numbers ever.

## Related

- [Proposal 022](../.proposals/022_outbound_fix_pr_campaign.md) — the campaign design + gates this plan executes by hand
- [Proposal 025](../.proposals/025_mozilla_human_scale_ai_incubator.md) — the grant application these PRs evidence (W3 community proof, G3 evidence pack)
- [Research 361](../.research/361_uniffi_rs_ffi_hygiene_distill.md) / [Plan 327](327_uniffi_ffi_hygiene_batch.md) — INBOUND lane (mining), explicitly NOT a prerequisite for this wetrun
- [Research 362](../.research/362_sccache_distill.md) / [Plan 328](328_sccache_perf_batch.md) — INBOUND lane sibling; its A-lead is sccache's OWN landed fix (T-scc)
- [Plan 224](224_fsearch_outbound_pr_distill.md) — prior-art outbound-PR lane (fsearch), its T5–T8 gated on mainnet — unrelated target, same discipline
