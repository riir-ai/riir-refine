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
- [x] T1.2f Dist flip EXECUTED 2026-10-01: owner renamed the GitHub dist repo (→ `gist-rs/cargo-refine`); dist repo swept (README/installers/guard — installers carry a cargo-heal asset-spelling fallback so pre-rename releases stay installable); **v0.2.0 cut manually** (the 5-target zigbuild matrix from the M3 per the free-tier manual-deploy rule; THIRD_PARTY_LICENSES via cargo-about; size-gate 5/5 + leak-scan 5/5 — the ceiling re-based 6.5→9.0 MiB from the FIRST all-five measurement, numbers recorded in the script header; release https://github.com/gist-rs/cargo-refine/releases/tag/v0.2.0); tap `0c003fb` (Formula/cargo-refine.rb, class CargoRefine — the yml's class bug fixed in the same pass) + scoop `57a043e` (bucket/cargo-refine.json, gnu zip) flipped with no aliases; dapps install copy swept `5f8d48b` + parity 3/3; the riir-llm/mmorpg-examples/viewbridge AGENTS sweep misses fixed. **Rider: the `.heal` → `.refine` state-dir amendment** (owner call, same day) — riir-kat `1d20c58` + riir-clippy code/disk/binary (24 repo dirs + home + `.refine-sync-policy` migrated on the M3; account key in `~/.config/riir-auth` untouched; live-verified: account c79c…5345, burn ledger reads, no `.heal` recreated).
- [ ] Revisit trigger (caveat 2): first verifiable external install → alias + compat pass owed.

## Gates

- [x] lib default 2398/0; clippy `-D warnings` clean at the full release feature set (covers riir-kat via path dep).
- [x] CLI battery: build_stamp 4/4, unmine 17/17, kat_soft_gate 9/9, kat_refill 5/5, install_copy_parity 3/3.
- [x] `repo_posture_cli` 3 arms RED — RESOLVED same-day follow-up session: the applying arms
      invoked `--fix` without `--no-verify`, so under `--all-features` (`fix_verify` compiled in)
      the compile gate refused the manifest-less hermetic fixture (`verify: FAILED — no Cargo.toml
      above src/lib.rs`) and reverted the edits. Fixed by `--no-verify` on the three applying arms
      (the test's own no-cargo-spawn hermeticity law) — 6/6 at BOTH postures now. En route: the
      `heal.rs` unreachable-`return` clippy red at the mid posture (required-features WITHOUT
      `kat_transport`) fixed, and the three owed `[[test]] required-features` rows landed
      (`repo_posture_cli` + `auto_oracle_features_cli` + `coverage_report_cli` — the T1.1e
      green-zero law; naming any at default features now refuses instead of `ok. 0 passed`).
- [x] riir-kat lib 236/236 after its sweep.
