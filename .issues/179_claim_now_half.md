# Claim: riir-refine issue 179 NOW half (the kat_account provider-identity adoption, local-only slice)

**Status:** COMPLETE — NOW half landed 2026-10-27 (`95bc8c18`: the
render + the byte-preserve tests; the survey below). The AFTER half
(provider_login wiring, rebind surface, TOTP client) stays blocked on
the dapps 139/140 route hosts — separate claims, this lease is done.
Scope was: issue 179's NOW half ONLY — (1) the binding-row render on
refine's two account surfaces (`kat account` + `--info`), (2) the
byte-preserve fixture test, (3) the report-only signing-session survey
(recorded in the issue). Issue 175's operational lease (T-PREFLIGHT;
"no code changes in this lease") was untouched — the claim was code in
`src/bin/` + `tests/`, zero wire ops.

**Lease closed:** amo-auth-followup — the NOW half landed 2026-10-27 as
`95bc8c18` (see the Status line above); the claim line reshaped to this
non-claim form the same day, when the AFTER half's item 6 claimed its own
line (`amo-idle-totp`, the TOTP client half).

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
  (bin-local, `kat_info/provider_bindings.rs` — the substrate render
  lives in riir-auth/riir-kat, outside this repo's write scope) + one
  call site each in `run_kat_account` / `run_kat_info`. Zero lines when
  `providers` is empty (byte-identical output — the additive-law test
  pins it).
- [x] Byte-preserve test: a record fixture WITH provider rows round-trips
  the load→serialize→parse path unchanged (the feature-off-rewrite
  hazard asserted for the refine consumer); the render test pins the
  column shape + the empty-record byte-identity. LANDED `95bc8c18`:
  `tests/kat_account_g9.rs` g9f (2 tests; the renderer file included by
  PATH so the pinned suite tests the exact source the bins compile).
- [x] NOW-3 — the signing-session survey (report-only; arming is
  owner-gated, recorded here per the issue's split):

  | refine op | class | session-gate fit | note |
  |---|---|---|---|
  | `account migrate --github` (dual-sign filing) | WALLET (a key-identity act on the ledger) | STRONGEST — the exact `OpClass::Wallet` the T2 session refuses to seed | issue 175's T-MIGRATE; would require the operator to re-biometric every filing attempt within a 15-min window |
  | `--fix` compile-gated writes | local code mutation | NO — never a seed door; the fixer runs unattended (cron/idle) and a TTL gate would break every schedule-bearing lane | disclosed, deliberately out of scope |
  | `--mine`/sync push tails | network write, non-money | WEAK — the burn ack is accounting, not authority | a session here adds friction without an authority win |
  | `stake`/`pay`/`buy`/`topup` money verbs | WALLET-class on the wire | STRONG — the seed door exists precisely for these | but they are interactive verbs today; the session's ABSOLUTE TTL means a fresh biometric per 15-min window |

  Verdict: the only ops the session class fits WITHOUT breaking the
  unattended lanes are the interactive wallet verbs (`migrate` filing
  included). Arming any of them is an owner call (a behavior change to
  every schedule-bearing workflow is NOT implied); nothing here arms.
- [x] Session survey (NOW-3): report-only, recorded in the issue (no
  code — arming is owner-gated).
