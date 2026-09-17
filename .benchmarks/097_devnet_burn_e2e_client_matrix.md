# Bench 097 — Plan 141: the devnet burn-process e2e, M3 + RTX4090 as the client fleet

**Status:** RECORD — executed live 2026-09-18 (M3, devnet `https://devnet.ai.gist.rs`; 4090/SHIKUWA leg over Tailscale `100.85.179.44`).

## 0. What ran

Plan 141's client burn-path proof, end to end, against live devnet, with
server-observable assertions only (G1): metering, push, watermark, replay,
clamp, depletion, soft-gate refusal, refuel arms, conservation, hourly
settle, repeatability. The rerunnable instrument is
[`scripts/burn_e2e.sh`](../scripts/burn_e2e.sh) — **9/9 hard assertions
green twice consecutively** (T7).

## 1. P2 — both machines at the release set

| machine | build | `cargo heal --version` |
|---|---|---|
| M3 (macOS arm64) | HEAD `bdc2961a` + `--features latent_retrieval,kat_service,kat_transport,fix_verify`, 1m39s | `release set: complete (latent_retrieval,kat_service,kat_transport,fix_verify)` |
| 4090 / SHIKUWA (Windows) | `16948eab` (1 behind origin; the delta commit `6c7e086c` touches zero kat/burn surfaces — verified via `git diff --name-only`), 11m48s via WMI-detached background build | `release set: complete (latent_retrieval,kat_service,kat_transport,fix_verify)` |

Both machines `config get`: `service url: https://devnet.ai.gist.rs · source: config`.

**4090 note (sibling agents):** its checkout carried another agent's
Batch-152 WIP (6 dirty files, exactly the paths `6c7e086c` landed
upstream). Per the never-stash-sibling-work rule the tree was left dirty;
the build ran on it (it compiles clean) and NO merge was performed. Also
measured: hung `git fetch` processes were killed by PID only after
confirming, via `Get-CimInstance Win32_Process` command lines, that the
victims were this session's own riir-clippy fetches — a sibling agent's
seal-game-editor processes ran untouched alongside.

## 2. The test fleet (throwaway accounts — G2)

| account | role | end state (server-read) |
|---|---|---|
| `c79c…5345` | M3 FLEET — **never touched** | balance 122.270866 KAT, contributed=true, unchanged |
| `7161…bd29` | 4090 FLEET — **never touched** | balance 116.128904 KAT, contributed=true, unchanged |
| `2e95…1069` | acct1: grant + depletion fuel | granted → burned to **0** over 90 metered scans |
| `518a…8176` | acct2: clamp / gate refusal / refuel arm B | no grant → clamped → REFUSED → contributed → gate lifted |
| `e051…785d` (M3) + per-cycle A/B/C keys | negative arms | warn-only never-synced; fresh per runner cycle |

The runner arms a **fleet guard**: it hard-refuses (exit 2) if a computed
test-account id ever prefixes a fleet id.

## 3. Per-track results (server-read evidence)

### T1 — metering (repo-local charge ledgers)
- Scratch scan: `this run 1,180,318 tok → 1.18 KAT` (1 KAT / 1M tokens, meter v1 · charge v1); 282 files / 2217 spans / 34.5 s wall.
- Repo-local artifacts: `.heal/kat_meter.json` (cumulative_tokens 1180318, sessions 1), `.heal/kat_charge.json` (charged_kat 1180318), footer + grant bar coherent.
- **GREEN** (runner T1 asserts tokens > 1e6 + footer shape, per cycle).

### T2 — push + watermark + replay
- Auto burn-sync tail after every charge-settling run: `burn sync: 1 KAT reported (server balance 99.000000 KAT)`; `/history` shows `balance_micro 99000000`, `burn.burn_kat_total 1`, event `seq 0 · total_kat 1 · applied_micro 1000000`.
- Watermark contiguity: acct1's 95 events span **seq 0..94 contiguous**, `total_kat` monotone non-decreasing end-to-end (verified programmatically over the full event list).
- Replay: re-reporting the same cumulative → the client-side drain answers `wire_total <= ack → Skipped` (no dial; the 999_999 sub-KAT floor) and the server answers 200 no-op (`events_total` unchanged, zero new debit) — acct1 `burn_total` stayed 108 → then 110 → 112 → 114 only when NEW cumulative was reported; the same-total re-push moved nothing.
- **GREEN.**

### T2c — the clamp (the server contract, live)
- acct2 (NO grant, balance 0) reported a 1-KAT cumulative burn: event `seq 0 · total_kat 1 · applied_micro 0` — **debited exactly 0, never negative**, watermark advanced to 1, balance stayed 0.
- **GREEN** (runner asserts applied_micro == 0 with burn_total > 0 on a zero-balance account).

### T3 — depletion + soft gate (the headline)
- **Depletion:** 90 repeated metered scans on acct1 (each ≈1.18 KAT, ≈35 s) drove the 100 KAT grant to **exactly 0**; the burn push reported whole-KAT floors (1 → 107 KAT) with sub-KAT remainders riding forward locally, exactly the `wire_kat_total` contract. Final: 108.59 KAT metered cumulative, 108 KAT reported, server balance **0.000000**, grant bar empty.
- **The refusal:** acct2 (depleted + never-contributed) at run start:
  ```
  cargo heal: REFUSED — the account's last synced KAT balance is 0.000000 KAT
  (depleted) and this machine has not yet contributed (Issue 075 soft gate).
  Refuel with exactly one of:
    * contribute:  cargo heal --mine … → cargo heal sync …
    * top up:      https://devnet.ai.gist.rs/pay?acct=518a…8b23b3d8
  ```
  exit code **1**. The plan's "three refuel paths" wording predates D4: the
  live refusal names exactly the TWO real paths (contribute / pay) and the
  D4-retired `--refill` is absent from the user remedy — recorded as the
  designed post-D4 shape.
- **Arm A:** `--refill` on devnet exits 1 — client posture `DevnetOps`
  (the tool runs) but the deployed devnet worker carries **no faucet route
  at all** (the `kat_faucet` feature is off in the deployed artifact: the
  POST falls through to the generic payreq matcher — "payreq: request id
  must be 32 hex chars"). Raw probe confirmed both hosts answer
  identically; no money path exists. Recorded honestly: the refusal is the
  D4 feature-set shape, not a cap refusal.
- **Arm B (the lift, measured live in full):** kill-switched fixture fix
  (real applied edits → 2 REAL included rows) → `--mine` + sync → batch
  accepted → server `contributed: true` AND the once-per-key grant claim
  (balance 100 KAT) → a fresh metered delta pushes the burn drain, whose
  v3 response carries `contributed: Some(true)` into the ack → the NEXT
  UNGATED run **exits 0**.
- **Negative:** never-synced account C runs **warn-only, exit 0** (no
  server truth → no gate — the designed leniency, verified).
- **GREEN** (all four arms; runner T3 asserts rc + remedy names per cycle).

### T4 — mining rebate (partial, honestly)
- Observed live: `pool_paid_micro 22,399,770` — **the 22.39977 KAT =
  70% × 32 KAT all-time-burned invariant, still standing** on the live
  server (Bench 001's number, re-read 2026-09-18).
- Epoch 2959 open during the session; the 4090 fleet's epoch-2958 receipt
  (`Minted 8.128904 KAT`) and epoch-2959 receipt (`Allocated`, 5 rows) show
  the allocate → mint pipeline mid-flight. The full epoch reconciliation
  (hold burns steady for a full epoch; reconcile minted = 70% × burned for
  the settled epochs) needs the epoch boundary — **deferred, cadence-bound**.
- The test accounts contributed real rows (acct1: 2; acct2: 2) now pending
  proof — their payout lands at the epoch settle, outside this session.

### T5 — conservation, live
- Per-account server-read invariant `balance_micro + Σ applied_micro ==
  grant`:
  - acct1: 0 + 100,000,000 == 100,000,000 ✓
  - acct2: 0 + 0 == 0 (never granted; the clamp held) ✓
  - runner T5 asserts both per cycle. **GREEN.**
- **Fleet-account observation (for riir-dapps):** the M3 fleet account
  reads balance 122.270866 KAT = grant 100 + receipt 14.270866 + **an
  unexplained exactly-8.000000 KAT** (receipts_total_matching=1; no second
  receipt at any limit). Recorded as an open question for the dapps side —
  NOT a conservation failure (the per-account event invariant held
  everywhere it was measurable; the fleet account's burn events predate the
  event lane, so its debit history is watermark-only).

### T6 — concurrency + hourly settle
- Parallel-fleet: both machines pushed burns against shared devnet in the
  same window (4090's own `92a6…82d9` test account + M3's accounts);
  per-account event seq stayed **contiguous 0..N** on every account;
  no lost updates, no cross-account interference. The runner's 4090 leg
  discovers the box (LAN then Tailscale), verifies its release-set binary,
  and drives its own cycle.
- **Hourly settle observed:** `/health` poll over 25 min — `burn_bucket`
  advanced 497133 → 497134 (the hourly KAT_FAST_SETTLE leg) and
  `pool_available_micro` jumped **67,200,000 → 98,000,000** (the bucket's
  settle landing accrued burn into the pool). Anchor seq advanced 215 → 216.
  **GREEN** for the within-session observable; the full-epoch cadence is
  the T4 remainder.

### T7 — repeatability
- `scripts/burn_e2e.sh` run twice consecutively: **9/9 PASS both times**
  (exit 0, stable output shape; fresh throwaway keys per cycle, replays
  no-ops by design). **GREEN.**

## 4. Findings worth keeping (beyond the green)

1. **The grant-claim sync arms the contributed grace** (`kat:contrib` is
   write-once per accepted batch, and the T079 claim rides sync) — so a
   granted + synced devnet account can never hit the 075 refusal while its
   ack is fresh. The refusal headline is reachable only for an account
   that pushes burns WITHOUT ever completing an accepted mining batch.
   Both shapes were exercised; the design consequence is recorded for
   Issue 094 follow-up (dapps-side).
2. **The v3 grace flag is ack-carried**: the ack's `contributed` updates
   only from a burn-push response, and the drain skips while
   `wire_total <= ack`. A depleted, zero-delta account that contributes
   learns contributed=true server-side but its LOCAL ack stays `false`
   until one more metered delta burns — the runner encodes the measured
   lift sequence (fix rows → sync → delta → sync → ungated rerun).
3. **The deployed devnet worker has no faucet route** (post-D4 feature
   set): `--refill` fails with the generic payreq matcher error, which the
   client surfaces verbatim. The message is honest but names no path —
   a minor UX follow-up candidate for riir-clippy (surface the D4 two-path
   remedy when the server 400s the faucet leg).
4. **Sub-KAT burns never cross the wire** — by design (`wire_kat_total`)
   — and the depletion proved the floor's working: 0.18–0.999 KAT remainders
   accumulate locally until a whole KAT reports. The server-side clamp then
   applies per REPORT, not per token.
5. **`/leaderboard` on devnet renders HTML only** (the deployment sets
   `KAT_WWW_HOST = devnet.ai.gist.rs`, so the host-split always renders the
   page). Machine-JSON consumers on devnet should read `/history` + `/health`
   instead — recorded for the runner (it does).

## 5. Deferred (exact reasons)

- **T4 full-epoch reconciliation** — needs the epoch-2959 boundary settle
  (cadence-bound, > session window). The 70%-invariant echo and the
  Allocated→Minted pipeline states were observed; the reconcile arithmetic
  re-runs at the boundary.
- **D4a vest-cycle datum (depletion → spendable gap)** — needs
  settle + `KAT_MINT_VEST_DAYS=1` on a contributed test account, i.e. the
  same boundary; the runner's test accounts hold pending claims that will
  make this measurable at the next settle.
- **Ops reset of a depleted test account** — no per-account ops credit
  surface exists (`/admin/treasury/fund` seeds the TREASURY only; the
  faucet route is compiled out of the deployed devnet worker). Per the
  plan's own rule the throwaway accounts stay depleted rather than
  improvising a mint; fresh keys per runner cycle are the reset.

## 6. Run pointers

- Runner: `scripts/burn_e2e.sh` (G1–G4 encoded; fleet guard; devnet-only).
- Session forensics: `/tmp/kat141/` (deplete.log — the 90-run depletion
  loop; per-arm logs), `/tmp/kat-burn-e2e/` (the runner's last cycle).
- Server reads quoted above are reproducible via
  `GET /history?acct=<id>&events_limit=…` and `GET /health` on devnet.
