# Release notes — v0.3.3 (the windows self-upgrade fix)

Cut 2026-10-17, hotfix on v0.3.2 (same day). Corpus counts unchanged since
v0.3.2 (no batch landed between the cuts).

---

corpus: no count delta since v0.3.2 (clippy_lints 112, rust_perf 230, kernel_opt 848, dist 7)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.3.2 (user-visible)

- **Windows dev-box exes can `cargo refine upgrade` again** — a windows exe
  built with the msvc toolchain (the dev-box `cargo build` + copy convention)
  answered "no release asset for this platform" because the check matched the
  compile-time ABI spelling exactly and the dist ships one windows asset (the
  gnu zip). The msvc arm now maps to that SAME asset — runtime-self-contained,
  SHA256SUMS-verified like any other — and the upgrade report prints the ABI
  crossing ("this exe was built for the msvc toolchain — installing the
  windows-gnu release asset"). Normal users (install.ps1 / scoop / a prior
  gnu install) were never affected; macOS/Linux were never affected.

## Honest exposure bound (the T15 note)

v0.3.3 changes the windows self-upgrade mapping only; no prior release
carries a newly disclosed defect from this cut. Every other v0.3.2 surface
is unchanged — see the v0.3.2 notes for the release's full content.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
