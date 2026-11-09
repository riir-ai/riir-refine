# Release notes — v0.5.0 (join says hello)

The contribution `join` flow gets its terminal manners, and the corpus
keeps growing.

---

corpus: kernel_opt 1076 (+15) — rust_perf 312 (+10) · sec 51 (+6) · clippy_lints 115 (+3) · docker 31 (=) · dist 8 (=) · shader_perf 8 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.8 (user-visible)

- **`cargo refine join` now talks while it waits**: a live stderr line
  ticks once a second during the GitHub device login (`⏳ waiting for you
  to authorize in the browser… (Ns elapsed · polling every 5s)`) instead
  of silence — the poll cadence is unchanged at the server-stated
  interval (polling faster trips `slow_down`, +5 s). Tty-gated; pipes
  stay clean.
- **🎉 on the join terminal lines**: the bound path's `joined:` line and
  the local-record fallback line, whose GitHub login renders bold+cyan
  (plain text under `NO_COLOR` / non-tty — greps on the plain login
  still match).
- **Corpus growth**: +15 kernel_opt, +10 rust_perf, +6 sec, +3
  clippy_lints rules mined since v0.4.8 (the metal-graph lifecycle,
  mixed-precision-attention, TileQuant, LocalSR measurement-honesty and
  safeagents/spillway egress-guard distills, among others).
- Internal: the device-flow poll loop gained a progress seam
  (`provider_login` / riir-kat forwards) — the `login --provider github`
  flow rides the same loop unchanged.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
