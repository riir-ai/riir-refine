# Bench 152 — i190 T10 signal-receipt refresh (2026-11-14T21:0x UTC)

**Scope:** issue 190 T10 no-submit-gate receipts, refresh of the 2026-11-11 baseline (bench 151's neighbor window; this is the issue-190 record — bench number allocated here because the evidence pack row cites this read date).

**Method:** unauthenticated public APIs, one read each, no credentials touched:
`https://crates.io/api/v1/crates/cargo-refine` + GitHub REST `repos/riir-ai/{cargo-refine,riir-refine,homebrew-tap,scoop-bucket}` + `repos/riir-ai/riir-refine/issues?state=all`.

## Readings

| axis | 2026-11-11 baseline | 2026-11-14 read | delta |
|---|---|---|---|
| crates.io `cargo-refine` total downloads | 0 (at-publish baseline) | **23** (0.5.0 = 11, 0.5.1 = 12) | +23 |
| crates.io recent downloads (365d window) | — | 23 | — |
| dist repo `riir-ai/cargo-refine` stars | 1 | **2** | +1 |
| dist repo forks | 0 | 0 | 0 |
| main repo `riir-ai/riir-refine` stars | — (adjacent evidence was katgpt-rs) | 1 | — |
| homebrew-tap stars | 0 | 0 | 0 |
| scoop-bucket stars | 0 | 0 | 0 |
| external issues/PRs on riir-ai/riir-refine | 0 | **0** (0 total rows, zero PRs) | 0 |
| T4 pilot sign-ups | 0 | 0 (T4 publish act not executed — see below) | 0 |

## Honest reading against the T10(a) bar

The bar: "≥1 external user/contributor signal exists by Nov 17 (pilot sign-ups from T4, installer downloads with issues/stars, an external contributor PR)."

- **Downloads:** 23 crates.io downloads in ~4 days. The evidence pack's own caveat binds: crates.io counts include CI/mirror fetches; quote as "downloads", never "users". 23 downloads is a floor improvement over the 0-baseline but is NOT a named user.
- **Stars:** dist repo went 1★ → 2★ (+1 external star). A star is a real (weak) external signal, but it is not a user/contributor, and a 1★ delta can also be a drive-by.
- **Issues/PRs:** zero external issues or PRs on the main repo. No external contributor PR exists (the sccache PRs are OURS into Mozilla, not inbound).
- **T4 pilot:** not executed — publish act blocked on credentialed channels (r/rust + URLO accounts absent on this box; TWiR submission = an outbound send under the project identity, gated by the `pr`/`outbound-pr` doctrine's owner-fingerprint gate).

**Verdict at this read: the T10(a) floor is NOT yet honestly met** — no pilot sign-ups, no external issue/PR, and the download/star delta is too thin to name as "users". The submission copy's honest posture ("recruiting now", "we will report it openly either way") does not depend on the gate passing, but the T10 checkbox stays unchecked on these numbers.

## What would change the verdict before 2026-11-17

1. **Owner-voiced T4 publish** (r/rust + URLO posts, optionally a TWiR submission) — the staged copy is ready (`mozilla_grant_readiness.md` §5, verbatim); sign-ups land within hours on the forums.
2. A real install followed by an external issue/star cluster (mirror-noise caveats still bind).
3. Any inbound external PR to `riir-ai/riir-refine`.

Re-read owed at the gate: whoever runs the T10 check on 2026-11-17 re-runs these two API reads and appends the delta; the T8 submission decision (owner) then has the receipts either way.

Provenance: read 2026-11-14T21:05–21:10 UTC from amo; curl with identifying User-Agent; no auth; counters are point-in-time.
