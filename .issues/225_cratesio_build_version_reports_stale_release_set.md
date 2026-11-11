# Issue 225 — the crates.io build's `--version` calls itself STALE and prescribes a rebuild it cannot do

**Status:** open — filed 2026-11-11 from the cargo-refine 0.5.0 crates.io install smoke (issue 221 P3).

## What happens

`cargo install cargo-refine` (crates.io 0.5.0, verified on amo 2026-11-11) then `cargo-refine --version` prints:

```
build: unknown
release set: STALE — missing latent_retrieval,points,net-mainnet (the network surfaces — sync/update/--refill/burn-push — are compiled out of this binary)
rebuild: cargo build --release --features latent_retrieval,points,net-mainnet,fix_verify,update_check,kernel_pregate,shader_perf --bin cargo-refine
```

Both lines are wrong FOR THIS LANE: the crates.io crate is the intended code-fixing posture (issue 221 lane design — retrieval/points/net are binaries-only), and the prescribed features do not exist in the published manifest (the surgery removed them), so following the advice fails. `build: unknown` — no commit stamp in a registry build.

## Fix

- [ ] The publish surgery (`scripts/cratesio_publish.sh`) sets a build-time lane marker (e.g. a `cratesio` cfg/env read by the stamp) so `--version` prints `release set: crates.io (code-fixing; join/leaderboard/latent retrieval ship in the prebuilt binaries — brew or install.sh)` instead of STALE + a rebuild line.
- [ ] Stamp the published version's tag/commit (the surgery knows `--ref`) instead of `build: unknown`.
- [ ] Test: the stamp's lane branch, both postures.
- [ ] Ships in the next crates.io release (0.5.1); issue 221 P4's alert/upgrade messaging rides the same lane marker.
