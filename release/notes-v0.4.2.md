# Release notes — v0.4.2 (the output you asked for)

Cut 2026-11-04 on the owner's UX law: the CLI speaks in short, precise,
call-to-action lines — separated, emoji-led, offering the next command.

---

corpus: kernel_opt 1051 (=) — rust_perf 294 (=) · sec 38 (=) · clippy_lints 112 (=) · dist 7 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.1 (user-visible)

- **One-line update alert.** `⬆️ update available: 0.4.1 → 0.4.2 — run
  \`cargo refine upgrade\`` — no release title, no URL block.
- **No billing vocabulary anywhere.** The `repo: open-source … never bills`
  line is gone (the run is free, full stop — saying "never bills" was still
  saying it).
- **The four-line header:** the version (+ the corpus epoch when one is
  installed), the update alert when one is due, `Refining…` before the
  quiet part, and the help hint.
- **The summary is two emoji lines:**
  `🔍 Found 21,086 clippy | 21,084 perf | 6 sec` — thousands-separated,
  biggest first.
  `🛠️ ~94% fixable via \`cargo refine fix\``
- **The fix run:** `🔍 Fixed N edit(s) across F file(s)` +
  `🛠️ ~6% unfixed — \`cargo refine join\` for self-evolve`.
- **Secrets in one line:** `🔒 3 secret-shaped literal(s) in 3 files —
  suggest-only, never auto-fixed; details: \`cargo refine scan\``.
- **The per-span preview is capped at 25** on tree-wide runs (the count
  stays exact; the tail line names the overflow).

## Honest notes

- `corpus` / `update` still name their missing transport in the shipped
  build (riir-refine issue 203 — the lease-read migration onto the points
  transport).
- The joined-user auto-send of unfixed data needs the worker's
  contribution endpoint — filed as riir-refine issue 204 (the client
  consent machinery exists; the wire is the missing half).

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
