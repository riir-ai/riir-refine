# Issue 082 — `kat_wetrun` parses the `/telemetry` keys that riir-dapps just renamed

**Status:** OPEN — filed 2026-09-09. **Not broken today**: riir-dapps emits the
legacy keys alongside the new ones for one deprecation release. This issue is
the interlock that must be closed BEFORE that release drops them.

## The finding

`src/kat_wetrun.rs:283,286` reads the telemetry row out of the worker's
`GET /telemetry` JSON by key name:

```rust
mint_kat: n("mint_kat")?,
pool_available_kat: n("pool_available_kat")?,
```

Those four amount fields always carried **micro** under a KAT name (1 KAT =
1,000,000 micro) — the same defect family as `.issues/081`, where an
unconverted micro book value in a whole-KAT wire field clamp-wiped a 100 KAT
grant. riir-dapps renamed them to `*_micro` on 2026-09-09 (riir-dapps
`.issues/033`, commit e114659) and riir-dao followed (riir-dao `.issues/002`,
commit a4d8eb8).

`n(...)` uses `?`, so a missing key is a hard error — the moment the legacy
keys are dropped, `kat_wetrun` stops decoding the row.

## Why it is not urgent and must not be forgotten

The worker publishes BOTH spellings for one release, so nothing breaks now.
That is also exactly how a consumer gets forgotten: the window closes silently.

## Fix

- [ ] `kat_wetrun.rs`: read `*_micro` first and fall back to `*_kat`, the
      pattern riir-dao's `dao-soak` uses (`u_micro(new, legacy)`) — it works on
      both sides of the riir-dapps deploy boundary, which matters because
      riir-dapps is fixed IN TREE but not deployed
- [ ] Rename the local struct fields `mint_kat`/`pool_available_kat` →
      `*_micro` for the same honesty reason (they are micro; the byte codec at
      `kat_wetrun.rs:227-230` is fixed-offset, so the rename is byte-neutral —
      keep the golden/round-trip test unchanged as the proof)
- [ ] `src/bin/heal.rs:2056-2063` renders these values — check whether it
      renders them as KAT anywhere (the `.issues/081` display class)
- [ ] Tell riir-dapps `.issues/033` when done — its "drop the `*_kat` keys"
      task is gated on this one

## Cross-refs

`.issues/081` (the burn-wire member of this family, live damage),
riir-dapps `.issues/033`, riir-dao `.issues/002`.
