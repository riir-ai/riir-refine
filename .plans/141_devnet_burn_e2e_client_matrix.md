# Plan 141 — Devnet burn-process e2e: M3 + RTX4090 as the full client fleet

**Status:** EXECUTED 2026-09-18 — T1/T2/T3/T5/T6/T7 GREEN live on devnet (Bench 097: `.benchmarks/097_devnet_burn_e2e_client_matrix.md`; runner `scripts/burn_e2e.sh`, 9/9 twice). T4 partial (epoch-bound: full reconcile + D4a vest datum deferred to the epoch-2959 settle). P1 preconditions landed upstream (riir-dapps Issue 094 + Plan 035 wave-2).

## 0. Goal + why now

Prove the CLIENT burn path end-to-end against live devnet with both real
machines — metering, push, watermark, depletion, soft-gate refusal, refuel,
mining rebate, conservation — producing a rerunnable script + a Bench record.

**Why this is only possible after Issue 094's fix:** with the 100M KAT grant
the depletion/gate/faucet paths are unreachable (~91,000 years at the
heaviest observed burn). At grant = 100 KAT they become reachable in days —
the fix converts the devnet fleet into the burn-loop test rig.

Fleet (devnet, 2026-09-17): m3 (`c79c…5345`, 228 pushed / 178 credited) and
SHIKUWA = RTX4090 (`7161…bd29`, mining). A third throwaway **test account**
is created for depletion tests — never the fleet accounts.

## 1. Preconditions

- [x] P1: riir-dapps Issue 094 fix tasks landed (grant 10⁸ µ; devnet ÷10⁶
      rescale — D3; faucet/drip user tier RETIRED per D4, devnet ops-only
      credit remains for this rig; front-page copy from constant; devnet
      worker runs `KAT_MINT_VEST_DAYS=1` — D4a). Verified live: grant claim =
      100.000000 KAT; the deployed devnet worker carries NO faucet route
      (`kat_faucet` compiled out).
- [x] P2: both machines rebuilt with the RELEASE feature set
      `default + latent_retrieval + kat_service + kat_transport + fix_verify`
      — the 00:21 stale-binary class; verify via `cargo heal --version`
      (build_stamp prints the compiled feature set + STALE warning). M3 at
      `bdc2961a` (1m39s); 4090 at `16948eab` (11m48s — the 1-commit delta
      `6c7e086c` touches zero kat/burn surfaces, verified).
- [x] P3: both machines pinned to devnet (`cargo heal config set --url
      devnet` → both read `source: config`); test accounts created + funded
      (grant claim); the runner's fleet guard hard-refuses fleet ids.
- [x] P4: 4090 reachable from M3 (LAN `192.168.1.33` timed out all session;
      Tailscale `100.85.179.44` green; PowerShell quoting solved via
      `-EncodedCommand`).

## 2. Tracks

### T1 — Metering coverage (repo-local charge ledgers)
- [x] Scratch repo `cargo heal --fix` on each machine: charge ledger row
      created; µ accumulation; footer numbers sane
      (`run_tokens`, `run_micro_kat`, `cum_micro_kat`, `grant_micro_kat`,
      `rate_micro_per_mtok`). Measured: 1,180,318 tok → 1.18 KAT per full
      scan of the 282-file corpus; meter/charge/footer all agree.
- [x] Multi-repo independence: ledgers are repo-local, the account is
      machine-global — verified (scratch books start at 0 per repo; the
      server watermark is one per account; the fleet's riir-clippy book
      (5.79 KAT) is separate from every scratch book).

### T2 — Push + watermark (the wire half)
- [x] Auto burn-sync tail fires after every `--fix` run; server
      `/history?acct=` shows the burn event rows (per-account seq contiguous:
      acct1 seq 0..94 contiguous over 95 events).
- [x] Replay: re-reporting the same cumulative = 200 no-op, zero debit
      (client-side `Skipped` on `wire_total <= ack`; server `events_total`
      unchanged; balance unchanged).
- [x] Clamp (from the TEST account only): a report exceeding balance debits
      to exactly 0, never negative — acct2 event `seq 0 · total 1 KAT ·
      applied 0 µ`.

### T3 — Depletion + soft gate (the headline)
- [x] Test account burns to 0 (repeated `--fix` over a large corpus; no
      fleet accounts touched): 90 metered scans, grant 100 → 0.000000 KAT
      server-read; whole-KAT report floors with sub-KAT remainders riding
      forward (the `wire_kat_total` contract observed end-to-end).
- [x] Next scanning run REFUSES: exit 1, the refuel paths named — the
      post-D4 shape names exactly the TWO real paths (contribute / pay);
      the plan's `--refill` third path is retired (D4) and absent from the
      user remedy (recorded in Bench 097 §3/T3).
- [x] Refuel arm A (post-D4): `--refill` refuses loud (rc 1) — the deployed
      devnet worker has no faucet route at all (`kat_faucet` off; the POST
      falls through to the generic payreq matcher), client surfaces it
      verbatim. The RIG used NO ops reset (none exists per-account —
      `/admin/treasury/fund` seeds the treasury only); fresh keys per
      runner cycle are the reset, recorded honestly.
- [x] Refuel arm B (the primary user path): `--mine` + sync contributes;
      the contribute-now flow exercised with `RIIR_HEAL_KAT_PROMPT_YES=1`;
      the sticky contributed-grace lifts the gate (full measured sequence:
      real fixture rows → accepted batch → server contributed=true → burn
      drain carries the v3 flag into the ack → ungated rerun rc 0).
- [-] D4a cycle datum: the depletion→spendable gap needs the epoch-2959
      settle + vest (cadence-bound, > session window) — deferred to the
      boundary; test accounts hold pending claims that make it measurable
      then. Bench 097 §5.
- [x] Negative arm: never-synced accounts stay warn-only (no server truth →
      no gate) — verified (rc 0, no refusal, warn line only).

### T4 — Mining rebate reconciliation (the flywheel)
- [x] Pool invariants observed live: `pool_paid_micro 22,399,770` — the
      22.39977 = 70% × 32 invariant standing (Bench 001's number re-read);
      epoch-2958 receipts `Minted` (8.128904 / 14.270866 KAT), epoch-2959
      receipt `Allocated` mid-flight.
- [-] Full-epoch reconciliation (hold burns steady one epoch; settle split
      65/5/20/10; minted = 70% × burned per settled epoch) — DEFERRED:
      cadence-bound to the epoch boundary, > session window. Bench 097 §5.

### T5 — Conservation, live
- [x] Across ALL accounts via `/history`: `Σ balances + burned_total ==
      Σ grants` — per-account `balance_micro + Σ applied_micro == grant`
      held on every measurable account (acct1: 0 + 1e8 == 1e8; acct2: 0 + 0
      == 0; runner T5 asserts both per cycle). One fleet-side open question
      recorded for dapps (the exactly-8.000000 KAT line on `c79c…`, Bench
      097 §3/T5) — not a conservation failure.

### T6 — Concurrency + hourly settle
- [x] M3 and 4090 push burns in parallel against shared devnet: no lost
      updates (per-account seq contiguous on every hammered account), no
      cross-account interference. Runner T6 leg verifies the 4090 binary.
- [x] The hourly `KAT_FAST_SETTLE` bucket leg advances the pool within the
      hour — OBSERVED: `burn_bucket` 497133 → 497134 and
      `pool_available_micro` 67,200,000 → 98,000,000 over a 25-min window;
      anchor seq 215 → 216.

### T7 — Repeatability
- [x] The runner green twice consecutively with stable output — 9/9 PASS
      both runs (fresh throwaway keys per cycle; replays are no-ops by
      design).

## 3. Deliverables

- [x] `scripts/burn_e2e.sh` (the `replay_e2e.sh` precedent): drives T1–T7 on
      M3 locally + the 4090 leg over SSH; refuses off-devnet configs (G4);
      prints a per-track verdict table; fleet guard (G2); idempotent rerun
      (G3); 9/9 twice.
- [x] Bench record (`.benchmarks/097_devnet_burn_e2e_client_matrix.md`) with
      the full evidence table + the reconcile outputs + the honest remainder
      list.
- [x] HISTORY/AGENTS notes in riir-clippy (client half); server observations
      reported to the dapps session (NOT edited there, per the task rails);
      Issue 094 cross-referenced.

## 4. Gates

- **G1 (server-observable):** every track asserts SERVER state (`/history`,
      receipts, pool) — never client-side claims alone.
- **G2 (fleet safety):** zero balance damage to the fleet accounts; depletion
      tests run ONLY on the throwaway test account.
- **G3 (idempotent rerun):** the runner is safe to run repeatedly; replays
      are no-ops by design (T2).
- **G4 (env guard):** the runner hard-refuses any non-devnet service URL;
      mainnet is never touched.

## 5. Non-goals

- No client/server behavior changes beyond what Issue 094's fix tasks land —
  this plan is proof, not repair.
- No pricing/product decisions (Issue 094 §4 owns them); no explorer
  (riir-dapps Plan 036); no new gates (riir-dapps Plan 035).

## References

- riir-dapps Issue 094 (the unit bug + fix tasks — the precondition).
- `scripts/replay_e2e.sh` (the runner pattern); `src/kat_burn_client.rs`,
  `src/kat_soft_gate.rs`, `src/kat_refill.rs` (the client surfaces under
  test); riir-dapps burn contract tests (the invariants T2/T5 prove live).
- Live baseline 2026-09-17: 32 KAT burned all-time, 17 KAT/7d, pool backing
  0.00023 KAT, 22.39977 KAT minted (= 70% × burned).
