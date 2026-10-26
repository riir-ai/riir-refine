# Claim: riir-refine issue 179 NOW half (the kat_account provider-identity adoption, local-only slice)

**Status:** IN PROGRESS — claimed 2026-10-27 (amo session, this thread).
Scope: issue 179's NOW half ONLY — (1) the binding-row render on refine's
two account surfaces (`kat account` + `--info`), (2) the byte-preserve
fixture test, (3) the report-only signing-session survey (recorded in the
issue). The AFTER half (provider_login wiring, rebind surface, TOTP
client) stays blocked on the dapps 139/140 route hosts — NOT this claim.
Issue 175's operational lease (T-PREFLIGHT; "no code changes in this
lease") is untouched — this claim is code in `src/bin/` + `tests/`, zero
wire ops.

**Claimed:** amo-auth-followup, 1795800000, expires 2026-10-28T00:00:00Z

## Execution notes (the session record — updated as work lands)

- [x] Recon: the render surfaces are refine-side (the substrate's
  `summary_lines` lives in riir-auth/riir-kat — outside this repo's write
  scope; the bins render the identity block themselves, so the bindings
  render belongs in the bins, through the re-exported `account_line`).
- [x] Collision scan: the 178 Phase A session owns `src/blueprint.rs` +
  `src/lib.rs` (their uncommitted lib.rs hunk adds `pub mod blueprint;`).
  This claim's write set is DISJOINT: `src/bin/heal.rs`,
  `src/bin/kat_info/mod.rs`, `tests/kat_account_g9.rs` (+ this issue
  file). No lib.rs edit needed — both bins already reach
  `account_line`/`AccountState` through `riir_refine::kat_account*`.
- [x] Render helper: one `provider_binding_lines(&AccountState) -> Vec<String>`
  in the heal bin + one call site each in `run_kat_account` /
  `run_kat_info`. Zero columns when `providers` is empty (byte-identical
  output — the additive-law test pins it).
- [x] Byte-preserve test: a record fixture WITH provider rows round-trips
  the load→serialize→parse path unchanged (the feature-off-rewrite
  hazard asserted for the refine consumer); the render test pins the
  column shape + the empty-record byte-identity.
- [x] Session survey (NOW-3): report-only, recorded in the issue (no
  code — arming is owner-gated).
