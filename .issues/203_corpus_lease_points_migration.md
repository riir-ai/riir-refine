# Issue 203 — the pivot's corpus-lease transport never migrated off `kat_transport`: the shipped v0.4.0 `corpus`/`update` verbs dead-end (found by the live-mainnet user-journey e2e, 2026-11-04)

**Status:** open — filed 2026-11-04 (zed-m3-issues-fixall) from the first live-mainnet e2e of the shipped v0.4.0 binary (tag build `d2fbba094c91`, the release set) against the deployed gen2 worker. Blocks the "corpus freshness is a public good" half of the pivot's CLI table (issue 192: `corpus` = "corpus info + update offer", `update` = "corpus lease refresh (exists)").

## What the shipped binary actually does (measured, live)

- `cargo refine corpus` (the points verb, `run_points_corpus` at `src/bin/heal.rs:3079`) prints: *"update: this build has no corpus-lease transport — cargo build --release --features latent_retrieval,points,fix_verify,update_check --bin cargo-refine"* — naming the EXACT release set the binary already carries (the build stamp's own `release set: complete`). The suggestion is unsatisfiable by construction: the gate it names (`kat_transport`) is the KAT transport plane the pivot deliberately compiles OUT of every shipped build, so **every shipped v0.4.x binary prints this dead-end forever**.
- `cargo refine update` refuses naming `kat_transport` (`KatCommand::Update | Sync` arm, `heal.rs:13420-class`) — same root: the corpus-lease refresh wire (`kat_protocol` lease endpoints, the `KAT_SIGNING_V2_ONLY` signing posture) still lives behind `kat_transport`, and the points plane (`points_client`, the `/points/*` wire) never grew a lease-refresh route.

## The design gap (root cause)

Issue 192's CLI table moved the USER-FACING verbs to the points plane but the corpus-lease UPDATE WIRE stayed on the KAT transport: the lease endpoints are server-side live (the hourly crank publishes epochs — plan 061/060), the client half (`cargo refine update` → lease fetch → overlay verify) rides `kat_transport`'s ureq + signing stack. The pivot flipped the release set to `latent_retrieval,points,fix_verify,update_check` without migrating the lease-fetch leg onto the points transport (which is std-only HTTP — the lease READ half needs no signing at all; the ledger-id-signed POSTs are the charge plane, which is correctly frozen).

## The fix shape (the points-free lease read)

1. Migrate the lease REFRESH READ (fetch the current epoch/overlay, verify, apply — the client's existing `update`/`sync` read half) onto the points transport behind `points` (it is points-free by law — GAP: "corpus freshness is a public good"). The lease CHARGE plane (the signed debit, `charge_corpus_lease`) stays frozen with the KAT lane — the shipped posture charges nothing, which matches the pivot's "never billed" copy.
2. Fix `run_points_corpus`'s dead-end copy: the `#[cfg(feature = "kat_transport")]` forks flip to `#[cfg(feature = "points")]` (+ honest state: "corpus lease refresh: available via the points transport" once (1) lands, or the deferred-honest line until then).
3. The `KatCommand::Update | Sync` refusal arm re-aims: without the migration, `update` stays refused in shipped builds — with (1), `update` dispatches through the points transport and the refusal shrinks to the genuinely-absent charge plane.

## Sister findings from the same e2e (recorded here; owner acts, not code)

- `tuna.riir.ai` does not resolve — `info`/`rank`/`join`'s points plane names the DNS failure honestly (the plan-061 residue: the custom_domain row, owner act). The worker's `/points/*` routes have been live since plan 061's 11-02 deploy; only the host attach is missing.
- `join` fails at GitHub with 400 — Device Flow is still disabled on the wallet OAuth app (`Ov23liAe58wk2byrYeOv`; riir-refine issue 188 T0, probed `device_flow_disabled` 2026-11-04, owner's 1-minute dashboard act).

## Acceptance

- [ ] The lease-read migration behind `points` (+ the honest deferred line until it lands); the dead-end copy fixed to name real state, never an unsatisfiable rebuild.
- [ ] `cargo refine update`/`corpus` in the SHIPPED release set stop dead-ending (either working via (1), or the deferred-honest copy).
- [ ] Gates: the points-verb gates battery + the charitable sweep stays green (no KAT copy regressions — the sweep's bar-1 pattern must stay zero).
- [ ] Re-run the live e2e (the user-journey table of 2026-11-04) — `corpus`/`update` join the passing rows.
