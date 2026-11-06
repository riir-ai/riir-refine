# Release notes — v0.4.7 (the corpus tree, sorted)

Owner call: the stats corpus block reads like `cargo tree` now.

---

corpus: kernel_opt 1061 (=) — rust_perf 302 (=) · sec 45 (=) · clippy_lints 112 (=) · dist 8 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.6 (user-visible)

- **The `cargo refine stats` corpus block is a tree:** `📚 corpus` with
  `├──` children per domain (emoji + colored count + a braille bar of the
  domain's share of the compiled corpus), sorted **biggest-first** (kernel
  1061 → perf 302 → clippy 112 → sec 45), the unnumbered tail
  (docker · dist · shader) as `└──`, and the leased delta under the tree.
- Tree glyphs dim on a terminal; plain when piped / `NO_COLOR` / `TERM=dumb`.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
