# Bench 159 — i190 T10 gate-day read (2026-11-17T02:3xZ)

**Scope:** the issue 190 T10 no-submit gate check, owed 2026-11-17 (bench 152's hand-off
row: "whoever runs the T10 check on 2026-11-17 re-runs these two API reads and appends
the delta"). Claimed + executed by zed-c0c35a68@amo same session (the prior lane's
15:00Z wake was verified NOT armed — `agent-mail scheduled` shows no pending delivery on
any candidate inbox — so the gate read would have been orphaned).

**Method:** identical to bench 152 — unauthenticated public APIs, no credentials:
`https://crates.io/api/v1/crates/cargo-refine` (+ `/versions`) + GitHub REST
`repos/riir-ai/{cargo-refine,homebrew-tap,scoop-bucket}` + all-state issues and PRs
queries + `releases?per_page=100` asset totals.

## Readings (vs the 11-14 refresh, bench 152)

| axis | 2026-11-14 (bench 152) | 2026-11-17 gate read | delta |
|---|---|---|---|
| crates.io total downloads | 23 (0.5.0=11 / 0.5.1=12) | **25** (0.5.0=12 / 0.5.1=13) | +2 |
| crates.io recent downloads (window) | 23 | 25 | +2 |
| dist repo `riir-ai/cargo-refine` stars | 2 | **2** | 0 |
| dist repo forks / watchers | 0 / 0 | 0 / 0 | 0 |
| dist repo issues (all states) | 0 | 0 | 0 |
| dist repo PRs (all states) | 0 | 0 | 0 |
| release asset downloads (23 releases, lifetime total) | — | **475** | — (first lifetime total in this family; T3's 10-31 pack cited 201 across 13 releases) |
| homebrew-tap / scoop-bucket stars | 0 / 0 | 0 / 0 | 0 |
| T4 pilot sign-ups | 0 | 0 — the publish act STILL has not happened; the T4 window note ("a send any time before 2026-11-17 still lands inside the T10 window") expires today unexecuted | 0 |

## Honest reading against the T10 bars

- **T10(a) — external user/contributor signal:** NOT met. The +2 crates.io downloads in
  3 days carry the standing mirror/CI-fetch caveat (quote as "downloads", never
  "users" — at 25 total, our own box installs are a plausible share of the delta); no
  star movement, zero external issues/PRs anywhere in the dist family, no pilot
  sign-ups (the owner-routed T4 send never executed).
- **T10(b) — consult answer:** NOT met and now moot-shaped: the T7 consult email was
  DEFERRED per owner and never sent (the Cloudflare Email Service unblock research
  stands, but its two owner-side prerequisites — domain onboarding + an
  Email-Sending:Edit token — were never completed). No Mozilla answer exists to cite.

**Gate verdict at 2026-11-17: BOTH bars unmet → the pre-registered no-submit-by-default
rule FIRES: HOLD the submission.** "If neither: hold the submission, keep the packet,
re-target the next cohort." The HOLD is the issue's own pre-registered rule, not a new
owner gate — but the SUBMIT-ANYWAY override is inherently the owner's call (T8 is
owner-voiced and owner-entered; the deadline is 2026-11-20 11:59 PM PST, ~3.5 days out).
The receipts above are the complete decision input either way. Also carried into the
same decision: T6's standing ⛔ pre-submission gate (gen2 deploy not live, probed
2026-11-05) remains red.

## What would still change the verdict before the deadline (all owner acts)

1. **T4 send** (the staged §5 copy, owner-voiced) + sign-ups landing before 2026-11-20 —
   forums deliver within hours; a send today would still give ~3 days of signal, but the
   T10 window itself (by Nov 17) is closed at this read.
2. **T7 send** via the researched Cloudflare Email Service path (owner completes the
   domain onboarding + token) — an answer before the 20th is unlikely but nonzero.
3. Any inbound external PR/issue cluster (nothing pending).

Re-read cadence after this: none scheduled — the next meaningful read is the owner's
T8 decision moment (this bench + bench 152 are the receipts).
