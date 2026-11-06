# Release notes — v0.4.6 (the stats panel, polished)

Cut 2026-11-06 on the owner's UI call: the panel should scan like a gauge,
not read like a sentence.

---

corpus: kernel_opt 1061 (=) — rust_perf 302 (=) · sec 45 (=) · clippy_lints 112 (=) · dist 8 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.5 (user-visible)

- **`cargo refine stats` restructured**: short lines; the trainer line is
  `🔬 74 rules · 5 epochs · epoch 2971`; the corpus block is one row per
  domain — 📎 clippy · ⚡ perf · 🧬 kernel · 🛡️ sec — each with its count
  and a braille bar of its share of the compiled corpus (the same gauge
  the footer uses); the leased delta renders as `leased +N vs prev epoch`.
- **Numbers are colored** on a terminal (green success %, cyan counts, dim
  labels); piped / `NO_COLOR` / `TERM=dumb` output stays plain and every
  version/count token stays grep-able.
- The ZWJ emoji (👩‍🔬) is gone — it rendered badly on some terminals; 🔬
  everywhere.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
