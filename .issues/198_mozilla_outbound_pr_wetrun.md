# Issue 198 — Mozilla outbound fix-PR wetrun (Proposal 022 Phase 1, quartet)

**Status:** OPEN — scan+heal complete, send owner-gated (Plan 329 executing)

Owner directive 2026-11-02: run the outbound fix-PR campaign (Proposal 022) as a wetrun against the Mozilla quartet (uniffi-rs, sccache, rust-code-analysis, supply-chain), forks under the `riir-ai` GitHub org, as Proposal 025 grant evidence (real upstream PRs = the product-community-fit proof). Full plan: `.plans/329_mozilla_outbound_pr_wetrun.md`; bench record: `.benchmarks/127_mozilla_wetrun/README.md`.

## Findings so far

1. **All scanned targets are clippy-clean at their own posture** (uniffi-rs @ pinned 1.94.0: 0 warnings/598 artifacts; sccache @ MSRV 1.91.0 with their exact CI flags: 0; mp4parse-rust: 0; rust-code-analysis: dormant since 2026-01, scan-only; supply-chain: 0 .rs files, skip). The 022 mechanical gate correctly yields ZERO clippy-lane PRs — recorded as a gate pass, not a failure.
2. **sccache's distill A-lead is the repo's own landed fix** — `pre_exec`/`posix_spawn` fallback already mitigated at the pin (`mock_command.rs:269–282`, `share_jobserver`, call site `:707`). Verified in-source before any PR work; nothing was sent on it.
3. **Day-1 PR prepared (heal complete, G1 green at their MSRV):** sccache#2975 CACHEDIR.TAG — branch `refine/cachedir-tag` @ `b97bcc0`, 80 insertions/1 file, 598 lib tests green (+2 new), their CI clippy flags 0 findings, fmt clean. Send is owner-gated (gh device-flow login → fork `riir-ai/sccache` → push → PR).
4. **uniffi-rs day-1 = no PR (honest empty):** tree clean; the engaged fix space (issue #3011) is another contributor's PR #3020 sitting CHANGES_REQUESTED for 3 weeks — a competing PR needs an owner call. Our own issue #3013 (Swift copyBytes) has zero upstream engagement; a PR there is speculative.
5. **Distill feedback:** add an upstream-state check (`git log -S`/blame at the pin) to the mining-candidate template — Research 362's A-lead was graded as open fix space without checking upstream state. → `.distill/001` post-mining intake.

## Acceptance

- [ ] T5 send executed (fork + push + PR URL recorded in bench 127's ledger)
- [ ] Outcome poller: PR merged/closed recorded; accept-rate ledger row updated
- [ ] T7 post-mining intake line landed in `.distill/001_mining_queue_snapshot.md`
- [ ] T8 close-out: HISTORY record + Proposal 022/025 evidence lines + `.raw` clones removed
