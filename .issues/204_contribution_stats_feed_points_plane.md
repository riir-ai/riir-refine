# Issue 204 — the contribution-stats feed: a joined user's fix/unfixed data auto-sends to the node (the owner's UX law: "after join it should auto send unfixed data")

**Status:** open — filed 2026-11-04 (zed-m3-issues-fixall) from the CLI-UX overhaul session (the owner's fix-summary spec: `🛠️ N% unfixed — do \`cargo refine join\` for self-evolve` — the CTA is only honest if joining actually ARM the auto-send).

## What exists, what's missing

- **The push tail exists but is KAT-wire only**: the `--fix` run-end stats push (Proposal 004 Half 1.5, heal.rs ~18249) is gated `kat_service + kat_transport` and dials `command_service_url()` (the KAT hosts) — the SHIPPED charitable build has neither feature, so it can never push. The `kat_stats_consent` plane (the third consent) + the `FixStatRow` wire (machine label, toolchain, files_touched, rule counts, domain fold) are all built and golden-tested — only the transport is on the wrong plane.
- **The misses half has NO wire at all**: the `--misses` queues (`src/misses*.rs`-adjacent tallies + the rescue tallies) are local-only. "Unfixed data" = these rows (rule id, span shape, decline reason — secret-blind per the sync policy).
- **The points plane has no contribution endpoint**: `/points/*` ships join/leave/rank/info/balance/anchor only (plan 061). A stats/miss feed needs a worker route + a wire DTO.

## The shape (the points-plane contribution feed)

1. **Worker** (riir-dapps kat-service): `POST /points/contribute` — the same account-key-signed envelope family as join; payload = the FixStatRow shape + the miss rows (rule id, shape hash, decline class — never code, never spans-with-content; the sync policy's `allow_redacted: false` law is the privacy floor). consent-gated server-side too (the row carries the client UUID; joined identity only — anonymous sends are the `--stats` plane's business, not this one).
2. **Client** (riir-refine points_client): `contribute_stats()` + `contribute_misses()` over `UreqPointsTransport`; the run-end tail re-aims from the KAT hosts to the points transport behind `points` (the kat_transport tail stays for the operator lane). Auto-send = the fix run-end, when joined (the binding exists) AND the consent state allows — the owner's flow makes join IMPLY the stats consent ask (the consent screen's join-or-anonymous question already exists; joining answers it).
3. **The UX tie-in**: the fix summary's `🛠️ N% unfixed` line reads `cargo refine join` only when NOT joined; when joined + consented it prints `📡 shared N finding(s) with the network` (the push tail's own line, reworded to the contribution vocabulary — GAP 10).

## The auto-send law (owner directive verbatim)

"after join it should auto send unfixed data to the node" — join flips the default: the first-run consent question's JOIN answer turns the stats+miss push ON for subsequent fix runs (the consent text already says what is shared: spans/results, secret-blind, `allow_redacted: false`). Leave turns it off (zero data sent — the leave law).

## Acceptance

- [ ] The worker route + the wire DTO (riir-dapps lane; the points wire's format_version discipline).
- [ ] The client transport + the run-end tail re-aim behind `points` (the charitable build CAN push; the kat_transport tail stays the operator lane).
- [ ] The join-implies-consent default + the leave-off law, tested both directions.
- [ ] The fix summary's joined/consented print (`📡 shared …`), the un-joined CTA unchanged.
- [ ] Gates: the charitable sweep (no KAT copy — the contribution vocabulary only), the points-verb gates, the sync-policy privacy floor test.
