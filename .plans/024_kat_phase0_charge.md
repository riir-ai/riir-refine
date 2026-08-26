# Plan 024 — KAT Phase 0 T0.6: the charge model (per-unit burn) + charge consumer

Executes [Proposal 005](../.proposals/005_kat_heal_service_network.md) **Phase 0 T0.6** — the
charge-model decision + the consumer of meter events. Opened 2026-08-26 after [Plan 023](023_kat_phase0_meter.md)
landed the meter (the producer).

## The decision (T0.6 — recorded, with its tension)

**Per-unit burn, not the volume-tiered lease.** Session decision 2026-08-26:

- **Simplest auditable accounting:** charge = tokens × rate — one integer multiply; a
  tier table would add tier-boundary + lease-expiry semantics Phase 0 has no server to
  arbitrate.
- **Token-only, no `$`** (caveats #12/#15) — the charge line reports KAT only.
- **It IS the §KAT spec** ("Modelless healing — 1 KAT per code word token processed");
  caveat #16's volume-tiered-lease *recommendation* is the proposal author's suggestion,
  not the owner's call. The owner's call (2026-08-25) was that the modelless burn IS the
  sink; caveat #16 itself frames its recommendation as "preserving the owner's call".
- **The caveat #16 substitution argument bites at T1.3** (the Stripe SKU shape — where a
  per-unit price meets the free default build), not at Phase 0 (accounting-only, G9d:
  nothing mints or spends). The decision is explicitly **revisitable before T1.3**; the
  rate rides a versioned constant precisely so a revisit is a versioned delta, not a
  rewrite.

### Rate encoding (the letter/intent note)

The session decision said "encode the rate as a versioned constant beside `METER_VERSION`
in `src/kat_meter.rs` so rate changes are covered by `METER_CHANGELOG`". Implemented as
**`CHARGE_VERSION` + `CHARGE_CHANGELOG` siblings** (in `src/kat_meter.rs`, literally
beside `METER_VERSION`) rather than folding into `METER_VERSION`:

- A `METER_VERSION` bump discards existing meter files (unknown-version → fresh, Plan 023
  semantics — correct for *counting* changes, where old counts are incomparable). Folding
  a rate change into it would **destroy the token history** on every reprice — the exact
  opposite of the audit trail the decision wants.
- A rate change does not move the token count; it moves the KAT owed per token. Separate
  axis, separate version, same discipline: **either change is a reprice; neither may be
  silent.** The intent (versioned constant + changelog-covered reprices) is honored
  exactly; only the changelog's *name* differs.

## Design

1. **Rate:** `CHARGE_KAT_PER_MILLION_TOKENS: u64 = 1_000_000` (= exactly 1 KAT per token
   at v1). Per-million encoding so a future fractional rate (500_000 = 0.5 KAT/token)
   needs no type change; at v1 the division is exact for every input (u128 intermediate
   before the divide — `u64::MAX × 1e6` overflows u64).
2. **Free tier + warn (constants, projection-only):** `FREE_TIER_GRANT_KAT = 100_000_000`
   (§KAT; minted server-side at `login`, T0.1+) and `KAT_WARN_REMAINING = 10_000_000`
   (§KAT out-of-KAT warn threshold; the QR half is T1.6). The local balance is a
   **projection**: `grant.saturating_sub(charged_kat)` — the server ledger is the
   authority (caveat #9); healing never blocks (G3, "warn-and-pay, never brick").
3. **Charge state** (`<root>/.heal/kat_charge.json`, same root family as the meter):
   `{ charge_version, charged_kat, meter_watermark_tokens }`. The watermark is the
   meter-cumulative already charged; **settle charges only the delta at the then-current
   rate** — a future rate change applies to NEW tokens only, the frozen `charged_kat`
   below the watermark is never silently re-derived.
4. **Load semantics** (never aborts a heal — the meter/recorder precedents):
   - missing / corrupt → **recovered**: watermark = the meter cumulative at open,
     `charged_kat = kat_for_tokens(watermark)` — the meter file is the token authority,
     so a lost charge file re-derives from it (approximated whole at the current rate;
     only reachable by file loss, never by a version bump).
   - version mismatch → **salvage**: keep `charged_kat` + watermark (valid history under
     the old model), restamp `charge_version`, warn loudly. No double-charge, no re-derivation.
   - meter regression (fresh meter below the watermark — corrupt-meter recovery):
     delta saturates to 0; no charge until the meter re-passes the watermark. Conservative,
     documented.
5. **CLI:** charge opens right after the meter (pre-recording — that is what makes
   recovery-at-open correct), settles at exit after `meter.save()`, prints one charge
   summary line + the ≤10M warn. `RIIR_HEAL_KAT_CHARGE_FILE` mirrors the meter override
   (tests/UX). **One kill-switch:** the charge exists only when the meter does
   (`RIIR_HEAL_KAT_METER=0` disables both — per the session decision, no second switch).

## Tasks

- [x] T1 `src/kat_meter.rs`: charge constants (`CHARGE_VERSION`, `CHARGE_CHANGELOG`,
  `CHARGE_KAT_PER_MILLION_TOKENS`, `FREE_TIER_GRANT_KAT`, `KAT_WARN_REMAINING`) +
  `kat_for_tokens` + `ChargeState` + `KatCharge` (open/open_at/settle/save/summary/
  balance projection/low-balance) + unit tests
- [x] T2 `src/lib.rs`: extend the `kat_service` re-export with the charge surface
- [x] T3 `src/bin/heal.rs`: `open_kat_charge` (env override + shared kill-switch) + open
  after the meter + settle/save/summary/warn at exit
- [x] T4 extend `tests/kat_meter_g9.rs`: exactness at v1 rate (incl. `u64::MAX`),
  delta-only settle (no double-charge), persistence round-trip, version-mismatch
  salvage, corrupt-file recovery-from-meter, balance saturation, charge changelog
  coverage + file stamp, summary carries no `$`
- [x] T5 validation: `cargo test/clippy --features kat_service` (lib + G9 file) green;
  default `--lib` baseline held (677); `--all-features` check green; bin e2e on M3
  (both lines, accumulation, kill-switch silence)
- [x] T6 `cargo heal --fix --write --verify` over the touched files; fix any mechanical
   findings (manual for divergence classes)
- [x] T7 `.plans/.highwater` 23 → 24

## Deferred `[-]`

- [-] T1.6 UX (QR + payment URL + persistent out-of-KAT banner): needs `qrcode` dep +
  the payment page — own task, gated on Phase 1.
- [-] Server-side reconciliation (T0.3 sync payload): the consumer of this ledger — the
  rate + watermark semantics here are designed so sync can replay them exactly.

## Validation record

(2026-08-26, M3)

- `cargo clippy --features kat_service --all-targets` — **0 warnings**
- `cargo test --features kat_service --lib` — **696/0** (Plan 023's 686 + 10 charge unit
  tests: changelog coverage, v1-rate exactness incl. `u64::MAX`, fractional-rate floor
  semantics, delta-only settle, meter-regression no-refund, persistence round-trip,
  version-mismatch salvage, corrupt/missing recovery-from-meter, balance saturation +
  warn threshold, no-`$` summary)
- `cargo test --features kat_service --test kat_meter_g9` — **11/11** (+4: charge version
  stamp on file, the bin-order composition cycle ×3, determinism across 25 independent
  settles + persisted reopen, independent u128 mul-div mirror)
- `cargo test --lib` (default) — **677/0**, baseline held (G3: charge code not compiled)
- `cargo test --features kat_service --test kat_meter_g2_alloc` — **1/1** held
- `cargo check --all-features --lib` — green
- Bin e2e (M3, `--features latent_retrieval,rust_perf,clippy_lints,kat_service`):
  run 1 → `kat: 122 …` + `kat-charge: 122 KAT burned this run (charge v1, 1000000 KAT
  per M tokens, cumulative 122 KAT — 99999878 of the 100000000 KAT grant projection
  remains; …)`; run 2 → accumulates (244/244, ledger files carry both version stamps +
  watermark == cumulative); `RIIR_HEAL_KAT_METER=0` → zero kat lines; fabricated
  95M-charged ledger → low-balance warn fires (4,999,878 ≤ 10M) with the never-brick
  message; fabricated charge-version-7 file → salvage warning + frozen charged_kat
  kept + only new tokens charged (12,345 + 122 = 12,467), version restamped
- `cargo heal --fix --write --verify` over the touched files: 8 mechanical edits landed
  compile-gated in `src/bin/heal.rs` (6× uninlined_format_args + map_unwrap_or +
  redundant_closure — pre-existing sites; 8 confirmed resolved / Certified), 0 applicable
  in the new kat files (written clean); the healer's `map_or` span-preserving edit
  re-indented manually (fmt skips the file for pre-existing drift)

## Honest notes

- **Tooling incident (recovered):** a `cargo fmt -- <files>` invocation formatted the
  ENTIRE workspace (~180 files) — cargo fmt does not scope to trailing file args. Full
  recovery via `git checkout -- . ':(exclude)…'` + re-application of the scoped edits;
  zero sibling WIP lost (verified: the sibling's files appeared only AFTER the recovery
  git status). Lesson: never `cargo fmt` with file args in this repo — the native
  healer fmt (`--fix --write`) is the sanctioned path, and it skips drift files by
  design. A second hazard caught: a filtered-output healer run still EXECUTES — the
  first `run #2` invocation applied the 8 edits invisibly behind a bad grep filter.
- **Concurrent sibling:** `src/bin/fixer_regate.rs` + `src/cf_workers_ai.rs` carry a
  sibling's live WIP (neuron cost-tracking, untracked `.benchmarks/llm_cost_ledger.jsonl`)
  and are EXCLUDED from this commit; HEAD also moved mid-session (25e36d9, docs-only).
- The `kat-charge` summary prints the raw rate constant (`1000000 KAT per M tokens`)
  rather than a derived `1 KAT/token` — deliberate: the constant IS the versioned
  contract; deriving a pretty form invites drift between display and charge.
