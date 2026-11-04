# Release notes — v0.4.1 (mainnet-only, enforced in the binary)

Cut 2026-11-04 on the owner's mainnet-only directive ("do it now" — the
fifth raise). v0.4.0 was default-mainnet; **v0.4.1 is net-PINNED**: the
devnet network is compiled out of the shipped binary entirely.

---

corpus: kernel_opt 1051 (=) — rust_perf 294 (=) · sec 38 (=) · clippy_lints 112 (=) · dist 7 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.0 (user-visible)

- **`net: mainnet` — the compile-time pin.** `--version` now prints the
  pinned posture. There is no devnet dial in this binary: the devnet host
  set is compiled to nothing (the foreign-hosts compile-to-nothing
  discipline), `--net devnet` refuses, and any config/env override pointing
  the points plane off the mainnet host family (`mainnet.riir.ai`,
  `tuna.riir.ai`, `riir.ai` + gen-1 aliases) refuses naming the pin.
  Verified at the byte level: the shipped binary carries ZERO devnet host
  strings.
- **The points plane rides the pin too.** `RIIR_REFINE_POINTS_SERVICE_URL`
    cannot redirect a pinned build at another network — the override
  resolves through the same compile-time host check.
- **The first-run stamp bug fixed.** v0.4.0 binaries printed
  `release set: STALE — missing net-mainnet` in some postures (cargo's
  env mangling turned the feature's `-` into `_`); the stamp now carries
  the manifest spelling and every posture reads `complete`.
- Everything else is v0.4.0's surface unchanged: free local healing, the
  verb-first CLI, join/leave/rank/info/corpus on the contribution points
  plane (live at tuna.riir.ai), consent-first, offline mode.

## Honest notes

- The corpus-lease refresh (`cargo refine update`) still names its missing
  transport — the lease-read migration onto the points transport is filed
  as riir-refine issue 203 and lands in a later cut.
- The mainnet host literals in the binary are the pin's own diagnostics
  (what it allows + what a refusal names) — disclosed and capped by the
  charitable sweep, which now hard-zeroes DEVNET hosts specifically.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
