# 174 — CLI tests that `login` adopt the box's real `~/.ssh/id_ed25519` (HOME not isolated)
**Status:** OPEN — census done, fix pattern proven in `build_stamp_cli` + `kat_refill_cli` (`7f076057`, this issue's filing commit)
**Claimed:** zed-m3-idle-174, 1793029400, expires 2026-10-28T09:30:00+00:00

## Finding
riir-auth `account_key` resolves `$HOME/.ssh/id_ed25519` (then `USERPROFILE`) and, when present,
**adopts it IN PLACE** as the account key (no copy; the scratch account record names the real path).
Every CLI test that runs `cargo refine login` with only `RIIR_AUTH_ACCOUNT_DIR` /
`RIIR_REFINE_KAT_ACCOUNT_DIR` isolated therefore signs with the developer's real SSH identity on a
box that has one, and with a freshly minted key on a box (or CI runner) that does not — the same
test exercises two different code paths depending on the box. Measured 2026-10-26 on amo:
`kat: account key adopted IN PLACE — /Users/…/.ssh/id_ed25519 IS the account key`.
Read-only (nothing is written to `~/.ssh`), but box-dependent and identity-leaking into scratch records.

## Census (tests setting `RIIR_AUTH_ACCOUNT_DIR` without `HOME`)
`cargo_convention_cli`, `kat_adopt_state_cli`, `kat_burn_stack`, `kat_join_cli`,
`kat_soft_gate_cli`, `sync_export_cli`, `unmine_consent_cli`, `topup_cli`.

## Tasks
- [ ] T1 — shared `tests/common/hermetic.rs`: one fn applying HOME/USERPROFILE → scratch AND pinning
  `CARGO_HOME`/`RUSTUP_HOME` to their real resolved values (a bare HOME override breaks any test whose
  binary spawns cargo/rustup — check `cargo_convention_cli` first). Each file's `heal` helper calls it.
- [ ] T2 — assert the login line reads `account key generated` (the `build_stamp_cli::tuna_logged_in` guard), so a regression re-adopting the SSH key reds.
- [ ] T3 — run each target at its own required-features + `--all-features`.
