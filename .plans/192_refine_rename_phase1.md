# Plan 192 — The refine rename, Phase 1 (binary + surface sweep)

**Status:** IN PROGRESS — T1.2 + T1.3 LANDED 2026-10-01 (owner ask: "see `cargo heal` a lot, it should changed to `cargo refine`? grep more"); T1.1 (repo rename) + the dist flip remain OWNER-SIDE.
Parent: [Proposal 018](../.proposals/018_refine_rename_stake_domain_tiers.md) (owner calls 2026-09-30: Refine picked, NO alias, Reflex stays free).

## What landed (2026-10-01)

- [x] T1.2a `[[bin]] name = "cargo-heal"` → `"cargo-refine"` (path `src/bin/heal.rs` UNCHANGED — internal vocabulary, move-don't-rewrite).
- [x] T1.2b Every CLI/display string: help, remedies, gate refusals, `--version` prefix (`cargo-refine <ver>`), build-stamp rebuild hints (`--bin cargo-refine`) + their test pins (`CARGO_BIN_EXE_cargo-refine`).
- [x] T1.2c The **riir-kat leaf sweep** (the missed surface the failing `kat_refill_cli` arm exposed): 18 files / 71 replacements — the client modules' user-facing copy (`cargo refine account/pay/faucet/stake/...`) are display strings of THIS product; wire composers + `kat:*` separators untouched.
- [x] T1.2d Global binary rebuilt at the release set (`default,latent_retrieval,kat_service,kat_transport,fix_verify`), symlink `~/.cargo/bin/cargo-refine` replaces `cargo-heal` (REMOVED — no alias, the owner call; stragglers fail loudly), `cargo refine --version` stamps complete, dry-run smoke green (lease + auto-oracle + anchored findings).
- [x] T1.2e release.yml packages the renamed binary forward (next tag ships `cargo-refine` archives + formula `bin.install` + scoop `bucket/cargo-refine.json`); `DIST_REPO: gist-rs/cargo-heal` KEPT (live repo; redirects after the owner flip).
- [x] T1.3a Living-docs sweep, riir-clippy: README/AGENTS/.docs book/.distill resume state/skills (3) — command examples now `cargo refine`; `.docs/07_examples/cargo_heal.md` → `cargo_refine.md`; skill dir `.agents/skills/cargo-heal` → `cargo-refine`.
- [x] T1.3b Sibling sweep (living docs + the one functional gate): katgpt-rs, riir-ai, riir-train, riir-neuron-db, riir-game-sdk, riir-chain (dockerfile_heal_gate `HEAL_BIN=cargo-refine` + workflow), riir-deployer, seal-remake, riir-reflex — AGENTS "Lint healing" sections + prose. riir-kat (above). riir-dapps: served copy (front page, heal_skill route `/skills/cargo-refine/SKILL.md`, wallet/mining/refusal strings) + contract pins.
- [x] T1.3c User-global config: `~/.agents/AGENTS.md` lint-healing rule + `~/.agents/skills/` symlink.
- [x] T1.3d 4090 box: pull + rebuild + `cargo-refine.exe` copy + remove `cargo-heal.exe`.

## Kept ON PURPOSE (grep noise, not stragglers)

- `gist-rs/cargo-heal` URLs/repo refs + `brew/scoop install cargo-heal` lines — coupled to the LIVE dist repo (works today, redirects after the flip).
- HISTORY.md / .benchmarks / .research / .proposals / .plans archives — dated records.
- Sibling provenance comments ("the cargo-heal Issue-117 lesson") — historical attribution.
- Internal vocabulary: `heal.rs`, `HealDomain`, `.heal/`, `RIIR_HEAL_*`, `~/.heal`, `kat:healqual:v1:`, `heal_skill.rs` module/route fn names.

## Owner-side remainders (cannot be done from an agent session)

- [ ] T1.1 GitHub remote rename `gist-rs/riir-clippy` → `gist-rs/riir-refine` + local dir renames (BOTH boxes) + the 22-file registration (repo_set, sweep pins, alias tables) — the fleet's largest registration; own commit family.
- [ ] T1.2f Dist flip: rename `gist-rs/cargo-heal` → `gist-rs/refine` (+ tap formula file + scoop manifest), cut the first `cargo-refine` release, then update the kept URLs/formula lines in copy.
- [ ] Revisit trigger (caveat 2): first verifiable external install → alias + compat pass owed.

## Gates

- [x] lib default 2398/0; clippy `-D warnings` clean at the full release feature set (covers riir-kat via path dep).
- [x] CLI battery: build_stamp 4/4, unmine 17/17, kat_soft_gate 9/9, kat_refill 5/5, install_copy_parity 3/3.
- [ ] ⚠ `repo_posture_cli` 3 arms RED — **pre-existing at HEAD** (verified by stash; not this rename). Filed for the owning session.
- [x] riir-kat lib 236/236 after its sweep.
