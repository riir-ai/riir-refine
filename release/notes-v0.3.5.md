# Release notes — v0.3.5 (the TOTP heartbeat hotfix)

Cut 2026-10-30, hotfix on v0.3.4 (published 2026-10-29). One fix; the corpus
moved two kernel rules since the v0.3.4 build.

---

corpus: kernel_opt 1022 (+2 since the v0.3.4 build) — rust_perf 278 (=) · sec 31 (=) · clippy_lints 112 (=) · dist 7 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.3.4 (user-visible)

- **TOTP-armed accounts: consent heartbeats work.** v0.3.4 shipped the consent
  heartbeat with its `X-Kat-Totp` header twice — `post_consent` set the
  header explicitly while the shared UA/env attach layer set the same header
  on the same POST. The worker runtime comma-joins repeats, so the edge read
  an 18-character code and refused EVERY armed account's heartbeat with 403
  `totp_required` — contribution consent stalled until upgrade. Found on the
  four-rig gen-2 contribution lane 2026-10-30. The fix makes the attach
  layer the one setter, with regression pins (a consent-POST pin + a
  source-shape test refusing a second explicit header set).
- Everything else is v0.3.4 unchanged — the gen-2 mainnet client; see the
  v0.3.4 release notes.

## Honest exposure bound (the T15 note)

The double-header is the only defect this cut fixes; v0.3.4 shipped it, and
no earlier release could reach gen-2 at all (they cannot sign v2), so no
other release ever carried it. Who is affected: a v0.3.4 user who arms TOTP
and contributes — heartbeats fail until upgrade; nothing is lost, the rows
stay local and push after the upgrade. No other prior-release defect is
newly disclosed by this cut.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
