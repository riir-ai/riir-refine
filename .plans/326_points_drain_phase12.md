# Plan 326 — the points drain, Phases 1–2: wire + client pre-check + seat counters + service accounting

**Status:** Active — executes [Proposal 027](../.proposals/027_points_drain_fair_use.md) (AGREE r3, `fefce49f5`) Phases 1–2, which need NO ratified numbers (everything ships inert / report-only); Phase 3 (ratification + promotion) is the owner act and is NOT this plan. Design contract = Proposal 027 — this plan does not re-argue it; deviations land in 027 first.
**Claimed:** zed-m3-drain2, 1793594377, expires 2026-11-04T04:39:37+00:00 — **EXECUTED 2026-11-02: T1–T16 (+T13.5, T5b) all landed; Phases 1–2 COMPLETE, byte-inert until ratification.**
**Lease closed:** zed-m3-drain2, 1793594377, claimed 2026-11-02 — executed within the window (ledger `b3ad4d4`+`0f00664`, refine `da867a10b`+`943de6a5`, instinct `22ff162`+`41e9428`+`960d77f`, dapps `8c6a46d`).

## Task

Ship the drain SHAPE across the four repos, byte-inert until the joint owner ratification pins A/B/R/C/P:

- **riir-ledger** — additive `drain_state` DTO + `FORMAT_VERSION` bump.
- **riir-refine** — the `points_drain` feature: day-state, `allow_hosted()` UX pre-check, `StandReason::DrainExhausted`, the 4-header attestation composer (Appendix A amendment 1).
- **riir-instinct** — the seat's report-only day counters, attestation verify, replay-window bitmap (`N = 128`), tiered fail, enforcement hooks (config-gated OFF), aggregate export.
- **riir-dapps** — the aggregate ingest route (idempotent by `(account, day)`), `drain_state` on `/points/balance`, the debit-at-settle path (config R/C absent = inert), the reconciliation test.

## Phase 1 — wire + client pre-check (riir-ledger, riir-refine)

- [x] **T1** `riir-ledger`: `DrainState { allowance_remaining_today, billable_ceiling_today, rate_served }` DTO + `FORMAT_VERSION` bump; additive serde (absence decodes byte-identically on today's readers — the `pay` law); the leaf-law test stays green (`cargo check --no-default-features`, zero optional deps). **— LANDED 2026-11-02 `b3ad4d4` (+ T1b `0f00664`: `attest::attestation_signing_bytes` — the ONE signing-bytes home — + `UsageState`; every golden re-pinned in the same commit, privacy census 11→14, all 30 tests + clippy green; the push went through — the `gist-rs/riir-ledger` remote EXISTS, the Plan 325 T9 residue note was stale)**
- [x] **T2** `riir-refine`: feature `points_drain` (opt-in, implied by nothing, never in a default or release set); the machine-local day state beside `client_id.rs`'s store — `{ utc_day, hosted_calls_today, cached_balance: Option<u64> }` (0600, same store law). **— LANDED 2026-11-02 `da867a10b`** (`src/points_drain.rs`; the state carries `billable_calls_today` + the served-constants cache beyond the sketch — both required by T3's own algorithm, disclosed)
- [x] **T3** `riir-refine`: `allow_hosted()` in `escalation_dispatch.rs` — AFTER the `min_pool` gate passes, immediately before the transport call; two integer compares; `Worthy`/`RateLatched`/`BelowMinPool` consume NOTHING; constants are SERVED (`drain_state`), never compiled — absent served constants ⇒ the gate does nothing (byte-identical to today). **— LANDED `da867a10b`** (6 order-pin tests; fails open on every local fault — the seat enforces)
- [x] **T4** `riir-refine`: `StandReason::DrainExhausted` arm → `EscalatedFix::StoodByModelless` + the ONE disclosure line (contribute, not pay — the issue-075 soft-gate sentence shape; contribution vocabulary only, the T7 law). **— LANDED `da867a10b`** (`fixer_regate.rs::stand_reason_axes`, counted on its own axis)
- [x] **T5** `riir-refine`: the attestation header composer — `X-Points-Account` (BLAKE3(pubkey) hex via the substrate's own `account_id_for`, reused not re-derived) + `X-Points-Pubkey` + `X-Points-Sig` (over `account_id ‖ utc_day ‖ call_seq ‖ blake3(request_body)`); the ONE header-setter seam (the `kat_ua` law — no duplicate `.set`); per-process `call_seq` counter. **— LANDED `da867a10b` + T5b `943de6a5`** (T5b adds the fourth header `X-Points-Seq` — the instinct lane found the three-header wire unverifiable: Ed25519 has no message recovery; the round-trip test mirrors the seat's full flow: transport seq → payload re-derivation → sig verify → authoritative-seq extraction from the verified payload `[42..50]`)
- [x] **T6** `riir-refine` tests (G1/G4): counter-order pin (no allowance consumed on non-seat-call outcomes); drain-off + absent-constants byte-identical boots; exhausted-points local `fix` output byte-identical; day-rollover re-roll; counting-allocator canary on the pre-check path (own binary + required-features row in the SAME commit — the green-zero law); dispatch overhead < 1 ms/call (G2 sanity, the FULL wired path as worst case). **— LANDED `da867a10b`** (9 module + 6 dispatch tests; 2875 lib green at the wired posture)
- [x] **T7** charitable gate: `points_drain` joins the FEATURES line only when it ships in a posture the gate sweeps; the sweep's positive arms unchanged (no new user-facing payment vocabulary — the drain declines with the contribute sentence). **— LANDED `da867a10b`: PASS** (the full script incl. the leak scan; `points_drain` in no FEATURES line)

## Phase 2 — seat counters + service accounting (riir-instinct, riir-dapps)

- [x] **T8** `riir-instinct`: seat-side day counters keyed `(account, utc_day)` + `(source_hash, utc_day)` — REPORT-ONLY (counts, no enforcement, no debits) until ratified config exists; the per-source key = salted daily-rotating /24 (IPv4) or /64 (IPv6) prefix hash, kept for the day only, never persisted past it; the privacy disclosure line lands beside the T16.1 client-UUID disclosure (same doc, plain words). **— LANDED 2026-11-02 `22ff162`** (deviation: std `Mutex`+`try_lock`, not papaya — moat-only per the 2026-10-03 carve; the `try_lock` miss branch IS the tiered-fail path; the disclosure landed in instinct README + the canonical §6 row in refine's `points_privacy_id_design.md`)
- [x] **T9** `riir-instinct`: attestation verify — `blake3(pubkey) == account_id`, then sig verify over the 4-part message; verification needs NO registry; account EXISTENCE (joined? balance?) is a separate lookup failing gracefully to the unjoined posture; failure classes (wrong sig / below-window seq / missing header) → unjoined posture, never an error. **— LANDED `22ff162`** (deviation: `ed25519-dalek` narrow optional-lib dep + BOUNDARY.md amendment, dated — the "reuse via the vessel reader" premise was stale post-carve; owner-review item)
- [x] **T10** `riir-instinct`: the replay-window bitmap — FIXED `N = 128` per `(account, day)` (IPsec shape): unseen-and-in-window accepts (out-of-order calls from concurrent runs ACCEPT — the test pins it), below-window rejects; wrap = 128 distinct seqs. **— LANDED `22ff162`** (the gate caught a real first-run bug: the slide must LEFT-shift — the first version right-shifted, silently wiping history and re-admitting replays; fixed in the landing)
- [x] **T11** `riir-instinct`: enforcement hooks — joined personal allowance `A_served` + billable tier (`B` calls at balance ≥ `R`) + per-source budget `P`; wired but config-gated OFF (absent/malformed ratified config = report-only); the tiered fail: counter-store outage ⇒ per-source traffic fails OPEN, the billable tier fails CLOSED (the anonymous posture serves). **— LANDED `22ff162`** (`INSTINCT_DRAIN_ENFORCE` + A/B/R/P; the joined posture rides the plane's usage-state read behind a TTL cache — failed read ⇒ unjoined, never cached negative; 429 `quota_exhausted` in contribution vocabulary; the tiered fail pinned by a store-poisoning test)
- [x] **T12** `riir-instinct`: the daily aggregate export — `(account, day, free_calls, billable_calls)` rows to the points plane, authenticated service route, idempotent by `(account, day)`; HTTP data crossing, ZERO cargo deps in either direction (the one-way rule; run `../riir-ai/scripts/ci_boundary_contract.sh` via the boundary-guard skill after the instinct touch). **— LANDED `22ff162`** (background 60 s tick at UTC rollover, never the /decide path; std-only HTTP, http-only documented; without the plane → the local daily JSONL dump — the P-sizing data; boundary contract clean: 32 repos, 481 edges; `ext_seat_rerank_gates` 8/8 + `serve_gates` 23/23 at BOTH feature states)
- [x] **T13** `riir-dapps`: the aggregate ingest route (the points row family — additive rows; the Plan 061 shapes); idempotent replay = no-op (watermark posture test); integer math only. **— LANDED 2026-11-02 `8c6a46d`** (`POST /points/usage`, bearer `KAT_USAGE_TOKEN` declared in wrangler.toml — provisioning per env = owner act; first-write-wins on the `points:usage:` rows; all-or-nothing validation; `{rows, processed, duplicates}` ack)
- [x] **T14** `riir-dapps`: `drain_state` on `/points/balance` (additive serde; the served constants come from deployment config — absent = the fields report the inert posture); refusals keep the `ErrorEnvelope` shape. **— LANDED `8c6a46d`** (`KAT_POINTS_DRAIN_{ALLOWANCE,BILLABLE_CEILING,RATE}`, all-or-nothing arming, null when unconfigured; the served values are the CONFIG CEILINGS — upper bounds for the UX-only pre-check, honest comment in the route; per-account remainders arrive when seat exports land)
- [x] **T15** `riir-dapps`: debit-at-settle — billable day ⇒ debit `R` from balance, FLOORED AT 0 (never negative — property test); config R/C absent = no debits; the accounting determinism test (same rows → bit-identical debits) + the settle reconciliation (seat counts vs plane debits) in the `dapps_contract_pins` shape. **— LANDED `8c6a46d`** (each UNBILLED billable `(account, day)` debits `min(R, balance)`; one `points:debit:` watermark row per billed day — never a double debit, re-tunes never re-bill; resumable; CrankReport v10 +3 fields lenient decode; **conservation RESOLVED: the debit is a BURN — balance → a `burned` door, Σ over the debit rows; `holds()` widened to `minted == balance + burned`, zero debits read the exact prior identity, all conservation tests green**)
- [x] **T16** gates: `points`-plane suites at both feature states in each touched repo; the dapps deploy-ledger audit only if a worker deploy happens (none planned — the ingest route rides the kat-service worker's next scheduled deploy, owner-timed). **— LANDED `8c6a46d`**: worker 619 lib (`kat_token_registry`) / 616 default, clippy clean both postures, wasm32 check green; root 205/203 lib + kat_crank 15; the v1→v2 wire pins updated in the same commit (test.sh 6 assertions + the contract fixture). Residue: the workerd e2e (`test.sh`) NOT EXECUTED — its 6 pins are updated for the next run (owner-timed, alongside Phase 3)

## Phase 3 — NOT this plan (the owner act)

The joint ratification (earn table + H + A/B/R/C/P — `P` sized from T8's report-only per-source counts, clearing the shared-NAT edge), config deploy, `points_drain` promotion (BEFORE any `hosted_expert` arming — the ordering law), the fronts un-roadmap-framed (Plan 324 G-B).

## Gates (from Proposal 027's GOAT — the plan's acceptance bar)

G1 property tests per T6/T9/T10/T11/T13/T15; G2 dispatch-overhead bench (T6); G3 flag-off byte-identical everywhere (a tested posture, never assumed); G4 alloc-free pre-check (T6); accounting determinism (T15). `cargo refine --score-bench` if any proposer-adjacent file is touched.

## Notes

- Lease per the claim protocol before starting any task; one repo's tasks per session ideally (disjoint write sets: T1–T7 refine+ledger, T8–T12 instinct, T13–T16 dapps — parallelizable across agents with the boundary-guard run on the instinct edge).
- riir-ledger push residue: T1 lands locally; the push fires when the owner creates the `gist-rs/riir-ledger` remote (same residue as Plan 325 T9).
- The seat touch (T8–T12) is the FIRST hosted-seat meter change — the instinct serve gates (`tests/ext_seat_rerank_gates.rs`) must stay green; the echo contract is untouched (headers only).

## Appendix A — the landed wire contract (T1 + T1b, riir-ledger `b3ad4d4` + `0f00664`)

Landed 2026-11-02; every task below consumes THIS, never a local re-derivation:

- **`FORMAT_VERSION = 2`** (the v1→2 bump IS the drain vocabulary).
- **`BalanceResponse.drain_state: Option<DrainState>`** (`serde(default)`):
  `{ allowance_remaining_today, billable_ceiling_today, rate_served }` — served
  constants; `None` = inert pre-check. A v1-era server JSON decodes `None`
  (golden-pinned).
- **`DrainState`** — the three served constants (T14 fills them from config).
- **`DrainUsageRow { account_id_hex, utc_day, free_calls, billable_calls }`** +
  **`DrainUsageExport { format_version, rows }`** — the seat→plane daily
  aggregate; idempotent by `(account_id_hex, utc_day)`; `utc_day` = `YYYY-MM-DD`
  UTC.
- **`UsageState { format_version, account_id_hex, joined, balance }`** — the
  plane's bearer-authed service answer the seat TTL-caches (the NEW T13 surface:
  `GET /points/usage-state?account=<64hex>`; the dapps agent names the route per
  the worker's routing conventions — the CONTRACT is: service bearer, account →
  joined+balance, `UsageState` body).
- **`attest::attestation_signing_bytes(account_id, utc_day, call_seq, body_hash)`**
  — the ONE signing-bytes home: raw 32-byte account id ‖ ASCII `YYYY-MM-DD` ‖
  u64 call_seq BIG-endian ‖ 32-byte BLAKE3(body), NO separators, 82 bytes total.
  `body_hash` is caller-computed (leaf law: no hash dep). Headers:
  `X-Points-Account` (64 hex) / `X-Points-Pubkey` (ed25519 pubkey, 64 hex) /
  `X-Points-Sig` (detached Ed25519 over those exact bytes). The seat checks
  `blake3(pubkey) == account_id` then verifies; ANY attestation failure → the
  unjoined posture, never an error.
- **Route pair for T12/T13:** the seat POSTs `DrainUsageExport` to the plane's
  usage-ingest route (T13; bearer service token — the service-secret pattern the
  kat-service already uses) and reads `UsageState` (above). Until the plane URL
  + token are configured, the seat keeps counters LOCAL (a daily JSON-lines dump
  beside its state) — that dump is the `P` sizing data.
- **Config seams (all ABSENT pre-ratification):** the plane carries the ratified
  `A/B/R/C/P` (config; absent → `drain_state: None`, no debits); the seat carries
  its enforcement copy (env/config; absent → report-only counting only).
