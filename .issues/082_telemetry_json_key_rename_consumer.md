# Issue 082 — `kat_wetrun` parses the `/telemetry` keys that riir-dapps just renamed

**Status:** RESOLVED 2026-09-09 (same day). Both spellings decode, `_micro`
wins, the struct fields no longer lie about their unit, and the human-facing
G-WET line renders KAT decimals. riir-dapps `.issues/033` is UNBLOCKED on this
side — dropping the legacy keys can no longer break `kat_wetrun`.
Verified: `--lib kat_wetrun` 12/12 (was 11 — the new precedence test),
`--test kat_wetrun_gwet` 11/11, clippy clean over lib + `cargo-heal` + tests
at `latent_retrieval,rust_perf,clippy_lints,kat_service,kat_transport`.

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

- [x] `parse_supply` reads `*_micro` first and falls back to `*_kat`, the
      pattern riir-dao's `dao-soak` uses (`u_micro(new, legacy)`) — it works on
      both sides of the riir-dapps deploy boundary, which matters because
      riir-dapps is fixed IN TREE but not deployed. `_micro` WINS where both
      are present (riir-dapps emits the pair with equal values, so there is no
      real conflict to arbitrate — preferring the honest name simply makes the
      legacy arm dead the moment the deploy lands). A row carrying NEITHER
      spelling is still a hard error, and the message now names both, so the
      reader can tell which side of the deploy the service is on
- [x] Struct fields renamed `mint_kat`/`burn_kat`/`pool_paid_kat`/
      `pool_available_kat` → `*_micro`. Byte-neutral, and the PROOF is that
      `to_bytes`'s fixed offsets (`0..8/8..16/16..24/24..32/32..40`),
      `ENCODED_LEN = 40`, and `the_supply_encoding_covers_every_field` are all
      untouched — only identifiers moved
- [x] The struct's own unit prose was the ambiguity that caused this: it said
      "every field is `i64` micro/KAT as the service serves it". Now it says
      micro-KAT with the 1_000_000 factor, and each field doc carries the unit
- [x] `src/bin/heal.rs` G-WET supply line: it printed the four raw micro
      integers under unit-free labels next to the word "supply" — the
      `.issues/081` display class in its milder form (it did not claim KAT,
      but it is the line a human reads when the gate reds). Now rendered via
      `kat_payreq_client::render_kat`, so `1400000` reads `1.400000 KAT`
- [x] The legacy arm is the one with no other coverage — the mock transport in
      `tests/kat_wetrun_gwet.rs` serves the NEW spelling (post-deploy
      reality), so the fallback is pinned by a dedicated unit test
      (`parse_supply_reads_either_spelling_and_prefers_micro`) rather than by
      the integration mock
- [x] riir-dapps `.issues/033` told: its "drop the `*_kat` keys" task no
      longer has a consumer interlock here

## Cross-refs

`.issues/081` (the burn-wire member of this family, live damage),
riir-dapps `.issues/033`, riir-dao `.issues/002`.
