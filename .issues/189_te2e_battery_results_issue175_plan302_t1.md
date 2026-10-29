# Issue 189 — T-E2E battery for Issue 175 / plan 302 T1: dao fixtures + refine Proposal-023 cells — results and the remaining halves

**Status:** OPEN — the battery ran green on every cell this session could reach; the remaining halves are (a) the plan-302 checkbox tick + T3b owner-call verdict, (b) E-5's live re-file attempt (box-scoped), (c) the owner-gated `kat_migrate` promote.
**Lease released:** zed-amo-175-te2e, 1793265834, expiry 2026-10-31T09:23:54Z — the measurement this issue records is DONE; released 2026-10-29 same-session (the remaining tails below belong to the next lane).

Executes the open plan-302 T1 checkbox ("T7 devnet rehearsal (agent)") from
[`../riir-refine/.plans/302_issue175_migration_then_secret_reset.md`](../.plans/302_issue175_migration_then_secret_reset.md)
against issue 175. Plan 302 is FROZEN at T1 by owner D1 (mainnet re-genesis,
riir-dapps Proposal 010) — T-MIGRATE is moot; this battery proves the D5
devnet-proven safe path. Measured 2026-10-29 on amo (macOS 27.0.1, M5 Ultra,
binary `5fdf37b78fab` = the full release set).

## Measured — the dao fixture half (ALL GREEN)

| suite | posture | result |
|---|---|---|
| `cloudflare/kat-service` `migrate_contract` (E1–E12 + rule_c + i143 + i146 + totp + tick/census lanes) | `--features kat_migrate` | **63/63 PASS** (the plan-302 "17 tests" figure is stale — cells 143/146/147 + rule_c grew it) |
| `riir-dapps` `kat_proof_spec_match::migrate_sweep_spec` (the 3 spec-match arms) | `--features kat_tuna,kat_migrate,kat_statehash` | **3/3 PASS** (the statehash arm silently needs `kat_statehash` — at `kat_tuna,kat_migrate` it filters out to 2/3, the documented green-zero shape) |
| `riir-dapps` lib migrate + migrate_sweep store arms (lifecycle, family sums, tombstone chokepoint, i143/i145 release arms) | `--features kat_migrate` | **50/50 PASS** |

This carries refine Proposal-023 **E-1** (no-objection release: `migrate_tally_e1_owner_endorsement_releases_via_lazy_status` + the tick lanes; tombstone refusal: `migrate_e6_tombstone_kills_every_route_family`, `a_dead_claim_account_is_unfilable_after_a_release`) and **E-2** (objection-cancel: `migrate_tally_e2_zero_votes_cancel_account_intact`, `rc_zero_votes_and_draws_cancel`; claimant-only cancel / thief-shape: `migrate_e10_cancel_is_claimant_only`), plus T1-live's two REAL devnet cron releases (filings #1/#2, exact conservation — recorded in the plan).

## Measured — refine-local E-3 (live, both halves, GREEN)

Isolated rig (no real account touched): fresh devnet login
`54862daf…fcfc8` under `RIIR_AUTH_ACCOUNT_DIR=/tmp/t7_e3_home/riir-auth`,
scan root isolated via `RIIR_REFINE_TRAJ_ROOT=/tmp/t7_e3_scratch` (the
resolve_heal_root env seam — the only way to keep this repo's shared
`.refine/` and the sibling session's pending records out of the cell).

- One real `--fix` run on a scratch crate (needless_return) generated a live
  devnet burn: charge book written, `sync ✓ current` (`5486…694c` on the
  devnet leaderboard), corpus lease epoch 2969 charged 0.001 KAT.
- The gate threshold: `charged_kat` is micro-units; the gate is
  `wire_kat_total(charged) − ack.burn_kat_total > 0` with
  `MIGRATE_MAX_UNSYNCED = 0` — i.e. it fires at ≥ 1 wire KAT pending. A
  sub-1-KAT book reads READY (`[STALE]` surface, no blocker) — **a real
  day of healing (~155k tokens = 155 charge units) never crosses it**; only
  legacy month-scale books do. E-3 is a fault-injection cell by its own
  wording ("a **deliberately** stale burn ack"), so the scratch book was
  sized to 1,234,567 µKAT (1 wire KAT + change) to arm the gate.
- **Refusal half GREEN**: census flipped `verdict: STALE — the filing
  REFUSES until: - the burn book holds 1 µKAT the service never
  acknowledged — run \`cargo refine sync\` (the auto burn-sync drains it)
  UNDER THE OLD ACCOUNT first` — naming the drain command exactly per
  Proposal 023.
- **Drain half GREEN**: `cargo refine sync --net devnet` → `burn sync: 1
  KAT reported (server balance 0.000000 KAT)`; ack v6 written (ledger_id
  `e1a835f9…` pinned, `charged_at_ack_micro` 1234567). Census re-read →
  `verdict: READY — the filing may proceed`.

Cosmetic finding (not a blocker): the census/sync render prints wire-KAT
values with µKAT labels ("holds **1 µKAT**", "booked **1 µKAT**") — the
arithmetic is right (wire-KAT units), the unit label is the Issue-094 slip
class (`kat_units::render_units` should render these two sites). Filed as
the follow-up task below.

## Measured — E-4 watermark reset (GREEN by three legs)

1. Conservation (T1-live, recorded): devnet sweep moved 174,866,068 µKAT
   exactly, old→new, zero loss.
2. Family law (green arms above): watermarks/burn-acks are per-account
   KEPT-and-reset rows — `zero_move_preserves_the_family_sum_and_zeroes_the_old_row`
   + `the_release_sweep_preserves_every_family_sum` + the census-table
   no-gaps arm pin "no old watermark replayed onto the new account" in code.
3. Live born-clean read (this cell): the fresh account's sync state started
   at 0 synced rows / no watermark, and its ack bound only its own ledger
   tail after the drain — the new-account-side watermark-reset property,
   observed live.

## E-5 released-id re-file refusal — fixture GREEN, live attempt BOX-SCOPED

- Store law green: `a_dead_claim_account_is_unfilable_after_a_release`,
  `migrate_tally_cancelled_id_refiles_as_replay`,
  `a_second_release_refuses_and_a_replay_of_the_cleanup_is_noop`.
- T1-live already proved wire-side post-release refusals (409
  `github_bound`, 409 `claim_not_fresh` cells) on the released identities.
- The fresh live attempt (re-file FROM the released `c79c93b8`) cannot run
  from amo: the c79c93b8 key lives at `/Users/kat/deprecated/old-ssh/`
  on the M3 (owner-held; the /tmp copies were shredded per plan-302 T2/T4
  hygiene). Remaining tail: run the same one-command attempt on the M3 and
  record which refusal code fires (`claim_not_fresh` vs `login_unproven`
  ordering vs the dead-account refusal — issue 144's proof gate may run
  first).

## The T3b owner call (SURFACED, awaiting the owner)

The mining plane's pending rows (MiningDo queue: unproven contribution
rows) are TTL'd ephemera on the OLD account — the sweep moves LEDGER
families only (one-batch re-key, per-family conservation) and does not
drain or gate on pending mining rows. Two postures:

- **(a) Accept the ephemera** (the current code's shape): pending rows TTL
  out on the dead account; the contribution FACT for already-accepted
  batches is safe (ledger-side); only unproven pending credit is forfeit.
- **(b) Gate at T7**: the census grows a mining-plane surface refusing the
  filing while pending rows exist, naming `cargo refine --mine` (drain
  first) — symmetric with the burn-delta gate.

Plan 302's data-survival table row: "MiningDo pending rows are TTL'd
ephemera — the T3b owner call, surface at T7". This issue is that
surfacing. Note: at re-genesis (D1) the whole plane is replaced, making
(a) the de-facto outcome unless the owner wants the gate for the D5 safe
path's own discipline.

## Owner-gated (parked here by design)

- GOAT verdict → `kat_migrate` promote to the apex/mainnet worker (the
  setup.sh `--mainnet` zero-pending gate runs first; deploy at ≥ `5a94f76`).
- The T3b ephemera-vs-gate call above.

## Remaining tasks

- [ ] Tick plan-302's open T1 checkbox with this issue as the record (the
      battery ran; the drill pair `c79c93b8 → 1bb4b7b5` is moot — the
      1bb4b7b5 copies were shredded and T1-live drilled real pairs #1/#2
      to release instead).
- [ ] E-5 live tail on the M3: the released-id re-file attempt, record the
      firing refusal code.
- [ ] Follow-up (display, riir-refine): the two census/sync render sites
      print wire-KAT with µKAT labels — route them through
      `kat_units::render_units`.
- [ ] Owner: the T3b call + the promote verdict.
