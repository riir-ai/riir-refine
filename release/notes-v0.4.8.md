# Release notes — v0.4.8 (the corpus tree's also-compiled trio)

The stats corpus tree numbers every compiled domain now.

---

corpus: kernel_opt 1061 (=) — rust_perf 302 (=) · sec 45 (=) · clippy_lints 112 (=) · docker 31 (=) · dist 8 (=) · shader_perf 8 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.7 (user-visible)

- **docker · dist · shader join the 📚 tree as full bars** (🐳 31 · 📦 8 ·
  🎨 8, biggest-first with the four nested domains; `└──` closes on the
  last bar). The `also compiled:` tail now names only `rustc_errors`
  (oracle-driven, uncounted — no corpus rows to count).
- The counts come from the pins manifest's new flat trio
  (`docker_rules` / `dist_rules` / `shader_perf_rules`) — derived from the
  compiled corpora by the same dump gate, never hand-typed; the gate's
  prose half now pins the AGENTS.md/README live rows for the trio too.
- `cargo refine --info`'s corpus line prints the trio's real numbers
  where it printed `-` before.
- Internal: --all-features clippy residue repaired (map_or,
  `usize::cast_signed`, a doc backtick).

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
