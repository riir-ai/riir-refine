# Issue 115 — Auto-oracle feature blindness (Issue 113 follow-up: default-features spawn misses gated diagnostics)
**Status:** OPEN — P1, unblocked. Filed 2026-09-16 from the room-to-improve triage (session record: HISTORY.md same-day entry).

## Why

The Issue-113 auto-oracle spawns
`cargo clippy --workspace --no-deps --message-format=json --quiet` at **DEFAULT
features** in the detected project root (`src/clippy_oracle.rs`). Diagnostics
that only fire behind a non-default feature are therefore **invisible to the
oracle-anchored suggest lane** — spans there fall back to `[heuristic]`
ranking, the exact failure mode the oracle was built to remove.

This is a documented blindness class in this repo's own guard-landing
protocol: `scripts/consumer_residue_sweep.sh`'s screen runs
`--all-features --all-targets` precisely because *"Not default features —
measured residue sat behind non-default ones."* The auto-oracle reintroduces
the class one lane over.

Precedent for the miss shape: the P15 Unknown-receiver residue lived behind
the oracle gap until `--clippy-json` fed it manually; feature-gated lints are
the same class with a different gate.

## Shape of the fix

- An env knob (`RIIR_HEAL_AUTO_ORACLE_FEATURES=<csv>`, empty = today's
  default-features posture) re-aiming the spawn with `--features <set>`.
- A loud status line whenever the knob is active (cost honesty: a
  feature-superset clippy run is materially slower — the sweep screen pays
  the same price by design).
- `RIIR_HEAL_AUTO_ORACLE=0` keeps winning outright (never-spawn posture
  unchanged); a manual `--clippy-json` feed still wins over both.

## Honest caveats

- No `--all-features` default flip: cost + the feature-combo explosion in
  sibling repos (riir-engine ~50 features) make the superset run a deliberate
  opt-in, not a posture change.
- The oracle can only see features that exist in the TARGET project's
  manifests; unknown feature names fail the clippy spawn — surface that
  failure loudly (the existing loud-heuristic-fallback path), never silently.

## Tasks
- [ ] T1 — `RIIR_HEAL_AUTO_ORACLE_FEATURES` env: parse, validate non-empty,
      plumb into the spawn args; loud line when active.
- [ ] T2 — Failure posture: feature set that fails the spawn degrades to the
      loud heuristic-only fallback (existing path), one line naming the knob.
- [ ] T3 — Fixture test: synthetic cargo project with a feature-gated
      diagnostic; assert `[oracle]` anchoring with the knob, `[heuristic]`
      without (the P15 specimen shape).
- [ ] T4 — SKILL.md / `--help` doc row: when to pay the superset cost.

## Related
- Issue 113 (auto-oracle landing), Issue 091 T1.5 (fixseq ring), the P15
  oracle-edits lane (`clippy_feed::oracle_edits`).
- `scripts/consumer_residue_sweep.sh` header (the documented class).
