# 166 — `cargo refine tuna` blames the deployment for an account-scoped fact

**Status:** FIXED in-session (2026-10-17) — fix commit lands with this file; removed on close with the HISTORY.md note.

## Finding (same session as riir-dapps Issue 128 — the grant-badge family)

On the M3 (fleet account `c79c…5345`, mainnet), `cargo refine tuna` printed:

```
tuna: the service does not carry the trial-credit plane (pre-TUNA deployment)
```

while the SAME deployment serves the 4090's account `100.000000 TUNA — expires in 27 d`.
The read was fine (`fetch_service_history` succeeded — the `kat` verb printed the correct
109 KAT from the same body): mainnet's TUNA plane is live (`/tuna/stats`
`genesis_done:1`). The `/history` wire omits the zipped `tuna_balance_micro` +
`tuna_expires_unix` pair when the account holds NO SPENDABLE trial credit — which is the
fleet account's true state on mainnet (it is the pre-flip KAT-grandfathered account: the
free tier was consumed as 100 KAT in the pre-2026-09-18 era; TUNA draws nothing, and its
devnet grace TUNA is a different net). `None` means "this account has no spendable trial
credit", and the account response's `granted` (the unified claim predicate) says WHY.

## Fix

`run_kat_tuna`'s None-arm splits on `h.granted` via the pure `tuna_none_line` helper:

- consumed → `0.000000 TUNA — the free tier is consumed on this account (a KAT-plane grant, or TUNA spent/expired; it is granted once per account)`
- not claimed → `not claimed yet — `cargo refine account` (or any --mine sync) claims the free tier automatically`

The "pre-TUNA deployment" wording is gone — it was unreachable-as-claimed (a genuinely
pre-TUNA worker is indistinguishable from a consumed account on this read) and was the
wrong attribution for every real hit.

## Test

`src/bin/heal.rs::tuna_none_line_tests::consumed_and_unclaimed_get_distinct_honest_lines`
