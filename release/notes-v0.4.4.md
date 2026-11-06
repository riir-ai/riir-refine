# Release notes — v0.4.4 (the % lines: fixer fix-rate, trainer smartness)

Cut 2026-11-06 on the owner's asks: "how % unfixed that fixer fixed" and
"how % trainer train" — the two health verbs now answer with numbers.

---

corpus: kernel_opt 1061 (=) — rust_perf 302 (=) · sec 45 (=) · clippy_lints 112 (=) · dist 8 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.3 (user-visible)

- **`cargo refine fixer` now carries the fix-rate line:**
  `fix rate: 10% of the clippy fix-set citations landed (52/511) · ~90% unfixed`
  — lifetime over this machine's fix-campaign history, plus each campaign's
  own % on the "last:" line. The denominator is the run's own fixable-clippy
  floor (the compiled fix set), the honest axis: suggest-only domains
  (kernel/sec restructurings) are citations, never counted as fixable
  failures. A machine with no history prints no fake 0%.
- **`cargo refine trainer` now shows the trainer's OUTPUT as smartness:**
  the leased-corpus section names the installed epoch, its rule census
  (`corpus: 74 rules — clippy 74`), and the upgrade trail
  (`upgrades 2965→2971 (5 installed) — each epoch is a training round`).
  The trainer gets measurably smarter as epochs walk and rules accumulate;
  the loop line states the pipeline: unfixed → misses queues → mined rules
  → the next epoch → the fixer cites them.

## Honest notes

- The shipped binary's learning half stays memory-only BY DESIGN (the
  clippy_verify oracle is a dev-posture feature): the local trainer records
  and ranks memory, and the NETWORK trainer (the epoch pipeline) is what
  ships new rules. The % of unfixed that training can close is measured by
  the coverage lane (`cargo refine --coverage-report`), not asserted here.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
