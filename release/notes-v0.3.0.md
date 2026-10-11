# Release notes draft — v0.3.0 (the Issue-003 T15 patched release)

Drafted 2026-10-11 ahead of the cut (riir-refine `.plans/223_refine_mainnet_release_checklist.md`).
The `scripts/manual_release.sh` publish step REQUIRES this file, its `corpus:` line
(becomes the release TITLE — the update alert quotes it), and fails without either.
Re-verify the corpus counts at cut time against `cargo refine --info` (the CORPUS
section) — the numbers below were measured at draft time from the AGENTS domain
table (HEAD `7caa23c5`: v0.2.1 shipped at rust_perf 179 / kernel_opt 777 / dist 6).

---

corpus: rust_perf +19 (179→198), kernel_opt +18 (777→795), dist +1 (6→7)

The per-network client release (riir-kat Issue 003 T15).

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.2.1 (user-visible)

- **Per-network client state** — every money/sync state file now lives under
  `.refine/net/<host>/` keyed on the CANONICAL service host. Mixed-era flat
  files from before this release are refused (loud, with the remedy) before any
  money decision: run `cargo refine config adopt-state <devnet|mainnet> [--discard]`
  (oracle-confirmed adoption; `--discard` quarantines with `.bak` twins) and
  `cargo refine config get` prints a per-file diagnosis.
- **`--net <devnet|mainnet>`** one-invocation net selection; `cargo refine --version`
  now prints the compiled net pin (`net: multi (default mainnet)` on the shipped bin).
- **Net-pinned builds** (`net-mainnet` / `net-devnet` features + optional `allow-local`)
  for rigs that must never dial the other network; pinned builds refuse foreign hosts
  naming the rebuild.
- **Every KAT dial sends a User-Agent** (`<consumer>; riir-kat/<version>; net/<slot>`)
  so the service can count client postures.
- **Attested-intent refusal**: money writes verify the server's self-declared env
  label + ledger instance id before dialling — a devnet/mainnet mixup refuses locally,
  fail-closed, both directions.
- All the healer/corpus work under the `corpus:` line above.

## Honest exposure bound (the T15 note — required reading)

Every binary shipped before this one carries the bidirectional service-host
failover bug (fixed in source 2026-10-07, `6d7567a` in riir-kat): during a
host-level outage window, a devnet-era lease/payreq money write could land on
the OTHER network's ledger. Exposure = devnet host-level-outage windows ×
old-client population, and it ENDS DEFINITIVELY at the v2-only cutover, when
the fresh mainnet ledger stops accepting pre-v2 signatures entirely (HTTP 426
`client too old` — the refusal names this upgrade). Old rigs that never upgrade
are locked out loudly at that point, never silently damaged. Per-network state
dirs + the attested-intent refusal (both in THIS release) close the same class
client-side.

## Upgrade channels

`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest MUST be bumped in the same window as this release, or the first two
arms are dead and only the releases URL works (launch-checklist rows R4/R5).
