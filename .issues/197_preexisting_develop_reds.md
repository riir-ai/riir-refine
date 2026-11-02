# Issue 197 — pre-existing develop reds: docker_t5_control test failure + 5 --all-features clippy findings

**Status:** RESOLVED 2026-11-02 — both reds fixed, reproducers re-run green (see the remedies below); the file folds to HISTORY.md next commit (the noise rule).
**Lease closed:** zed-780f7da3@m3, 1793602209, expires 2026-11-04T06:50:09Z (closed 2026-11-02, same day)

## What fails

1. **`tests/docker_t5_control::t5_control_scanner_structure_on_in_tree_files`** —
   fails in every posture on clean develop.
2. **`cargo clippy --all-targets --all-features -- -D warnings`** — 5 findings:
   `src/bin/heal.rs` + `src/score_bench/regime.rs`.

## Reproduce

```sh
cargo test --test docker_t5_control 2>&1 | tail
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -c '^error'
```

## Notes

- Found while gating plan 326 T2–T7 (the points_drain client half, `da867a10b`);
  the lane ran its gates at NAMED postures (default / points / points_drain /
  combined / no-default) — all green — the reds live only in the
  docker-control test and the --all-features sweep, neither touched by 326.
- The all-features findings are in files plan 326 never touched; fixing them is
  a standalone heal lane (healer-first per the lint law).

## Remedies (2026-11-02, zed-780f7da3@m3)

1. **T5 pin** — the pin reads riir-chain's `deploy/Dockerfile` at origin/develop,
   which `de1b3d9f` (2026-10-29, the 963-T3 toolchain-coherence lane) bumped
   `rust:1.98.1-alpine` → `rust:1.99.0-alpine` DELIBERATELY (its own dated bump row).
   Remedy = the file's own precedent: pin moved with a dated comment citing the
   bumping commit; stages 2 + stanzas 18 re-verified unchanged, only the tag moved.
2. **all-features clippy** — 3 findings (the issue's "5" counted the two
   `could not compile` summary lines the failing crates appended):
   - `src/bin/heal.rs` `empty_line_after_doc_comments`: the Issue-153 run-start doc
     comment had been ORPHANED by the Plan 325 points-section insert (`7b8aebe51`
     sat it above `points_account_signing` instead of its true target
     `run_update_surface_hooks`) — fixed by moving the banner + doc comment back
     onto the function (root cause, not lint-silence; the doc now documents the
     function it names).
   - `src/score_bench/regime.rs` `doc_markdown`: backticked `tests/step_noise_gates.rs`.
   - `src/score_bench/regime.rs` `cast_precision_loss`: house-pattern `#[allow]`
     with the loop-index-fixture reason (t ≤ 47; pedantic is repo-enforced via
     `lib.rs` `warn(clippy::pedantic)`; healer dry-run offered nothing — manual
     per the lint law).
   Healer-first honored: `cargo refine` dry-run over both files before any manual edit.

## Verified (final diff)

- `cargo test --test docker_t5_control` 10/10 · `clippy --all-targets --all-features
  -D warnings` clean · default + `elo_k_eff` clippy postures clean · points wired
  posture (`points,points_drain,latent_retrieval`) 3018 passed · regime 7/7 at
  `score_bench,elo_k_eff`.
- Pre-existing rustfmt drift surfaced by a per-file fmt pass (the Plan 325 points
  fns in heal.rs + one import sort) was REVERTED before commit — not this lane's
diff to absorb.
