# Issue 197 — pre-existing develop reds: docker_t5_control test failure + 5 --all-features clippy findings

**Status:** OPEN — pre-existing on clean develop, NOT introduced by plan 326 (verified by the T2–T7 lane via stash on clean develop, 2026-11-02); filed so the reds are tracked, not absorbed into a later lane's diff.
**Claimed:** zed-780f7da3@m3, 1793602209, expires 2026-11-04T06:50:09Z

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
