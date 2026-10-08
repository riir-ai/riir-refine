# Issue 152 — sync_export_cli red on develop since 2026-10-03 (the 21f76593 empty-path fail-closed broke the seeded legacy rows)

**Status:** OPEN — found 2026-10-08 by the branding-retirement pass (pre-edit HEAD baseline reproduced the failure in a detached worktree; NOT caused by that pass, whose diff touches no row-gating code).

## The failure

`cargo test --features kat_transport,latent_retrieval,rust_perf,clippy_lints,kat_service --test sync_export_cli` → 2 of 4 FAIL (verified at HEAD `80808901` AND with the branding edits — identical failures):

- `export_writes_a_verifiable_batch_without_an_account` — `assert_eq!(header.rows_included, 1)` got 0
- `export_to_stdout_is_pure_ndjson` — stderr lacks `export: 1 row(s) included`

## Root cause

The tests (43c104d5, 2026-10-03, riir-dapps Issue 112 T4) seed the store via
`TrajStore::record(rule, buggy, fixed, parses, resolved, source_tag)` — which writes **no `src_path`**.
`21f76593` (2026-10-03, riir-dapps Issue 119 T2 — "mined rows honour the sync policy's allow_paths …
**empty path fails closed**") landed after the test file and blocks exactly those rows at the gate →
0 rows included. `3e8f87fb` softened only the blocked-count MESSAGE for legacy rows, not the gating.
`d89fea5e` (T12b) ran g4 + fixture-freeze + lib for its gates — this suite was not in the set, so the
red sat unnoticed.

## The fix (owner of the sync lane; mechanical)

Seed the rows with a real `src_path` under the scratch dir (the API that carries it —
`TrajRowDoc`'s src_path field / a `record` overload), so the row passes `gate_row`'s allow-path
check; keep one blocked-row variant asserting the fail-closed path stays loud (that behavior is
correct and worth pinning). Update the two `rows_included` assertions accordingly.

## Notes

- The suite's hermetic env_remove of `RIIR_HEAL_SYNC_POLICY` (removed 2026-10-08 by the branding
  pass, along with the compat shim) is unrelated to this red — ambient env verified clean, and the
  baseline at pre-edit HEAD failed identically.
- When the riir-kat `RIIR_HEAL_*` dual-read fallbacks retire (issue 004 there), the drill's and the
  CLI tests' legacy-name sanitation sites can shed their remaining legacy knobs in the same pass.
