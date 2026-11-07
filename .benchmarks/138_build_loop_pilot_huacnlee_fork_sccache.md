# Bench 138 — build-loop pilot: huacnlee/cargo fork + sccache on the fixer/dev-loop posture (m3)

**Status:** RECORD — measured, verdicts recorded; adoption = fleet-scale owner call (riir-refine issue 211)

## Provenance

`PROVENANCE: power=AC Power load=4.10 swap=612.44M canary=skipped powermode=2(high)`
(`riir-reflex/scripts/bench_preflight.sh`, 2026-11-06 ~16:0x local, m3 QUIET per
`agent-mail rig status` — no jobs, load 4.1/16, AC, High Power mode). Single-run
cells (not interleaved pairs) — the deltas quoted are 5-7×, far above the ±5-22%
load-drift band, so the verdicts survive the single-run posture; the CLOSE cells
(A vs E, I vs F) are within drift band and are marked accordingly.

Postures: system cargo 1.99.0 (rustup shim, pinned `RUSTUP_TOOLCHAIN=1.99.0-aarch64-apple-darwin`
in BOTH postures so the fork — invoked directly, which bypasses rust-toolchain.toml
resolution — drives the identical rustc) vs huacnlee/cargo fork v0.102.0-9
(sha256-verified release binary, side-by-side at `~/.cargo/bin/cargo-hc`, `CARGO_FORK_AUTO_UPDATE=0`,
NEVER replacing system cargo) vs sccache 0.19.0 (brew, `RUSTC_WRAPPER`). Gate
command = the fixer/oracle posture: `cargo clippy --workspace --no-deps --all-targets --quiet`
(small workspaces; `--all-targets` dropped for the bevy-scale cells). Fork state
fresh (`~/.cargo/targets` absent pre-bench); sccache stats zeroed pre-bench.

## Cells (wall, `real` s; disk = `du -sk`)

| cell | posture | workspace | wall | disk | note |
|---|---|---|---|---|---|
| A | sys cold | riir-refine | **13.57** | 374 MB | fresh target t1b |
| B | sys 2nd identical checkout | riir-refine wt | **25.68** | 376 MB | full re-pay |
| C | fork cold | riir-refine | **24.06** | 451 MB | first build; fork tax 1.8× vs A |
| D | fork 2nd checkout | riir-refine wt | **19.27** | +388 MB | 0.75× B — deps shared, workspace units NOT (build.rs carve-out: riir-refine has build.rs) |
| E | sys+sccache cold | riir-refine | **15.50** | 374 MB | +14% populate tax vs A (within/near drift — marked) |
| F | sys+sccache+INCR=0, riir-kat | cross-project | **10.89** | 273 MB | vs plain I 10.40 s — NO wall gain |
| I | sys cold | riir-kat | **10.40** | 273 MB | baseline for F |
| S1 | sys cold | seal-game-editor (bevy) | **94.09** | 1.29 GB | 539 s CPU |
| S2 | sys 2nd worktree | sge wt | **88.81** | 1.27 GB | full re-pay at bevy scale |
| S3 | fork cold | seal-game-editor | **186.96** | →3.17 GB | first-build TAX 2.0× wall / 3.3× CPU vs S1 (anomaly, see below) |
| S4 | fork 2nd worktree | sge wt | **12.77** | +425 MB | **0.14× S2 — the fork's amortization, REAL at bevy scale** |
| S5 | fork warm | seal-game-editor | **4.93** | — | steady-state warm gate |

## Pre-registered bars → verdicts

1. **BAR-BIG-1: S4 ≤ 0.5 × S2** → **PASS** (12.77 vs 88.81 = **0.14×**, 6.95× faster). The fork's
   worktree amortization works where the dep tree dominates (bevy): the second
   worktree costs 425 MB instead of 1.27 GB and 13 s instead of 89 s.
2. **BAR-1 (small-scale): D ≤ 0.35 × B** → **FAIL** (0.75×). Small workspaces whose crates carry
   build.rs share only the dep slice; the workspace units re-carve per checkout.
3. **BAR-BIG-2: fork disk ≤ 0.7 × sys disk at N=2** → **FAIL at N=2** (3.59 GB fork-state vs
   2.56 GB sys, both including the first build). The crossover is arithmetic, not
   measured: sys grows linearly (+~89 s / +1.27 GB per worktree), fork pays ~13 s /
   +~0.43 GB per SUBSEQUENT worktree after one 187 s / 3.17 GB first build →
   **fork wins from the 3rd worktree per repo onward** and the gap widens linearly.
4. sccache cross-project (F ≤ 0.7 × I) → **FAIL / NO GAIN** (10.89 vs 10.40, within drift).
   Stats: 0 cache hits; the incremental=0 half WORKED (no new `incremental`
   non-cacheable entries; CPU dropped 28→9 s user) but the shared-dep units keyed
   differently across the two lockfiles (features/cfg differ per workspace), and
   these workspaces are bins + proc-macros + path deps — thin cacheable surface.

## Verdicts

- **Fork (huacnlee/cargo):** the mechanism delivers as documented at dep-heavy
  scale — second-worktree cost collapses (0.14× wall, 0.33× disk at bevy scale).
  ADOPTION SHAPE: worth it only where a repo accumulates ≥3 concurrent
  worktrees/checkout pairs (the git-workflow ticket lanes on amo/bmo qualify;
  m3's current concurrency is 1-2). First-build tax is heavy (2× wall / 3.3× CPU
  on the bevy tree — anomaly below) and the fork IGNORES `CARGO_TARGET_DIR`
  (the isolation law's replacement-protection test — the shared-target
  gate-hazard reproduction — is STILL OWED before any default adoption).
- **sccache:** not the lever for the fixer posture on this fleet's small
  workspaces — 0 hits cross-project, +14% cold tax. The `CARGO_INCREMENTAL=0`
  half is a free 3× CPU cut on gates and costs nothing — worth adopting as gate
  posture separately (a build-hygiene note, not a tool install).
- **blaze:** NOT tested (own dated-nightly toolchain + daemon install; separate
  pilot — issue 211 keeps it as the open dev-loop rung).

## Anomalies / disclosed unknowns

- **S3's 2.0× wall / 3.3× CPU first-build tax is unexplained** — candidates:
  content-key hashing over every unit's inputs, fork dev-default debuginfo
  interacting with the workspace's own `[profile.dev.package."*"] opt-level=3`,
  or build-script re-execution. Upstream-worthy question; do NOT quote the fork's
  first-build cost without reproducing it.
  **Repro queued (Issue 211 closed 2026-11-07, the row moved here):** isolated
  A/B as Exclusive amo job `200b5b44` (cargo-hc at `amo:~/bin/cargo-hc`
  sha-verified, seal-game-editor at amo develop head, plain arm on a scratch
  `CARGO_TARGET_DIR`, fork unit reset per rep, 2 interleaved reps). File
  upstream ONLY if reproduced; the posting act is owner-reserved
  (outward-facing). The fork itself is NOT adopted (delegated-owner verdict
  4785a7a0: DECLINE; `cargo-hc` uninstalled from m3) — the repro is
  curiosity-grade, it gates nothing.
- Main-clone cells (A, C, S1, S3, S5) measured the working tree AS FOUND — sge's
  main clone carried a sibling ticket lane's WIP (branch `fix/sge-builds-the-world-relay`);
  worktree cells measured the clean commit. Compile surfaces were docs-identical
  (the delta commits are docs-only), so cross-posture ratios stand.
- Fork side effects on m3, KEPT + disclosed: `~/.cargo/bin/cargo-hc` (side-by-side
  binary, system cargo untouched), `~/.cargo/targets/` 3.59 GB (fork-GC-bounded:
  per-project ≤ 2× largest build of the last day, shared-deps ≤ 50 GiB / 7-day
  TTL; `rm -rf ~/.cargo/targets` resets; uninstall = delete the binary).
  sccache cache 48 MB at `~/Library/Caches/Mozilla.sccache`; server stopped after
  the bench.
- Fork surfaced upstream-1.101 manifest lints our repos trip (`cargo::unused_dependencies`
  on riir-refine's `riir-refine-fixer` dep; `package.homepage` redundant with
  `repository` on riir-neuron-db) — free hygiene signal, filed nothing yet;
  candidates for the next docs/hygiene sweep.
- No Cargo.lock churn in any repo (verified post-bench). Bench worktrees
  (`riir-refine-bench-wt`, `sge-bench-wt`) and `/tmp/hc-bench` scratch removed at close.

## Relation to the fixer tier

The fixer's compile legs inherit the caller's cargo — so the fork's win arrives
WITHOUT riir-refine code changes wherever it's adopted per-box; the
`CARGO_TARGET_DIR` conflict means a box adopting the fork must either retire the
per-plan isolation rule or accept the fork's locking as the replacement (test
owed). The oracle-results cache (`08c5b6ee7`) stays complementary: it removes
REPEAT-session clippy cost; the fork removes dep-tree cost per checkout. Full
adoption framework + the owner-gated decision: riir-refine issue 211.
