# Release notes — v0.4.5 (the stats panel + the honest split)

Cut 2026-11-06 on the owner's UX call + the Claude verdict (REVISE, addressed):
"`~90% unfixed` is ambiguous — say what is fixable-but-unfixed and what is
unfixable" — plus `cargo refine stats` as a standing panel.

---

corpus: kernel_opt 1061 (=) — rust_perf 302 (=) · sec 45 (=) · clippy_lints 112 (=) · dist 8 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.4 (user-visible)

- **The fix-run residual is now a three-way split (shares of the total,
  sums to 100):** `🛠️ P% fixed · U% unfixed (fixable — the fixer's gap) ·
  X% unfixable now (no compiled fix — the trainer's queue)`. The old
  "~N% unfixed" mixed two different problems; now the fixer's gap and the
  trainer's queue are separately visible — you lower the first with more
  fixes, the second with trained rules.
- **`cargo refine stats` — the self-evolve loop in one panel:**
  🛠️ the fixer success rate (this machine's campaign ledger, sample size
  always shown: `10% fixer success rate (52/511)`), 👩‍🔬 the trainer's
  output (rules leased, delta vs the previous epoch, epochs installed),
  🧬 corpus growth (compiled counts + the leased delta), 📡 the network
  aggregate (contributors · contributions). Lines whose data is absent say
  so — no fake numbers.
- **The update alert is colored**: the NEW version renders green, the
  installed (old) yellow — on a terminal; piped/`NO_COLOR`/`TERM=dumb`
  output stays plain and the version tokens stay grep-able either way.
- **The bare-run header lost the help hint** — `cargo-refine 0.4.4 · corpus
  epoch 2971` and nothing else (`help` and `--help` remain the ways to ask).
- **Every user-facing `cargo refine --fix` string now says
  `cargo refine fix`** — the verb form everywhere (the flag spelling still
  works; the help says so).
- **The corpus line tells the truth about domains**: `corpus: leased rules
  riding this run (epoch N): …` plus a compiled-domains line that names
  kernel as pregate-gated (cost — the detector decides per file). All
  compiled domains were ALWAYS active in the scan; the old line listed only
  the LEASED overlay's domains and read wrong.

## Honest notes

- The network-side aggregates (a network fixer success rate, trained-% and
  accuracy of the finding surface, per-day corpus growth) need server
  routes — filed as riir-dapps issue 160; the stats lines cite it until
  then. The leased corpus carrying non-clippy domains is riir-dapps issue
  161.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
