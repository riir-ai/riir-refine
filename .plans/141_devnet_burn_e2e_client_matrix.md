# Plan 141 — Devnet burn-process e2e: M3 + RTX4090 as the full client fleet

**Status:** OPEN — planned 2026-09-17 on the owner ask ("it's devnet and only us m3, rtx4090 — use this as a chance to test the burn process from the client"). Blocks on riir-dapps Issue 094's fix tasks (grant 10⁸ µ + devnet ÷10⁶ rescale + faucet retune). No code yet.

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

- [ ] P1: riir-dapps Issue 094 fix tasks landed (grant 10⁸ µ; devnet ÷10⁶
      rescale — D3; faucet/drip user tier RETIRED per D4, devnet ops-only
      credit remains for this rig; front-page copy from constant; devnet
      worker runs `KAT_MINT_VEST_DAYS=1` — D4a).
- [ ] P2: both machines rebuilt with the RELEASE feature set
      `default + latent_retrieval + kat_service + kat_transport + fix_verify`
      — the 00:21 stale-binary class; verify via `cargo heal --version`
      (build_stamp prints the compiled feature set + STALE warning).
- [ ] P3: both machines pinned to devnet (`cargo heal config set --url
      devnet`); the test account created + funded; no mainnet config anywhere
      in the rig (env guard in the runner).
- [ ] P4: 4090 reachable from M3 (LAN `192.168.1.33`, Tailscale fallback
      `100.85.179.44`; PowerShell `;` separators — no `&&`).

## 2. Tracks

### T1 — Metering coverage (repo-local charge ledgers)
- [ ] Scratch repo `cargo heal --fix` on each machine: charge ledger row
      created; µ accumulation; footer numbers sane
      (`run_tokens`, `run_micro_kat`, `cum_micro_kat`, `grant_micro_kat`,
      `rate_micro_per_mtok`).
- [ ] Multi-repo independence: ledgers are repo-local, the account is
      machine-global — verify the split (one account, N repo ledgers, one
      server watermark).

### T2 — Push + watermark (the wire half)
- [ ] Auto burn-sync tail fires after every `--fix` run; server
      `/history?acct=` shows the burn event rows (per-account seq contiguous).
- [ ] Replay: re-reporting the same cumulative total = 200 no-op, zero debit.
- [ ] Clamp (from the TEST account only): a report exceeding balance debits
      to exactly 0, never negative — the server contract proven LIVE.

### T3 — Depletion + soft gate (the headline)
- [ ] Test account burns to 0 (repeated `--fix` over a large corpus; no
      fleet accounts touched).
- [ ] Next scanning run REFUSES: exit 1, the three refuel paths named
      (`--refill`, `--mine` + sync, payment page).
- [ ] Refuel arm A (post-D4): NO user faucet exists — assert `--refill`
      refuses loud and names the two real paths; the RIG itself uses the
      devnet ops-only credit to reset the throwaway account between runs
      (ops tool, never a user path).
- [ ] Refuel arm B (the primary user path): `--mine` + sync contributes; the
      contribute-now prompt (`RIIR_HEAL_KAT_PROMPT_YES=1`) exercises clean;
      the sticky contributed-grace lifts the gate.
- [ ] D4a cycle datum: measure the depletion→spendable gap on the test
      account (epoch-settle cadence + vest) and record it. NOTE: vest is only
      HALF the gap — the weekly settle cadence is the other half; for the
      full fast cycle pair `VEST=1d` with an ops-triggered settle (the
      existing crank) — otherwise settle dominates.
- [ ] Negative arm: never-synced accounts stay warn-only (no server truth →
      no gate) — the designed leniency, verified rather than assumed.

### T4 — Mining rebate reconciliation (the flywheel)
- [ ] Hold burns steady for one epoch on both machines; assert pool backing
      ≈ 0.70 × epoch burn; settle allocates per the 65/5/20/10 split.
- [ ] Receipt states observed: allocated (locked, vesting) → minted after the
      vest window (devnet `KAT_MINT_VEST_DAYS=1`, D4a); reconcile minted =
      70% × burned for the settled epochs (live echo of the 22.39977 = 70% ×
      32 invariant).

### T5 — Conservation, live
- [ ] Across ALL accounts via `/history`: `Σ balances + burned_total ==
      Σ grants` — the burn-contract invariant read from production.

### T6 — Concurrency + hourly settle
- [ ] M3 and 4090 push burns in parallel against shared devnet: no lost
      updates (per-account seq), no cross-account interference.
- [ ] The hourly `KAT_FAST_SETTLE` bucket leg advances the pool within the
      hour (0.7–10 µs/pass budget unchanged).

### T7 — Repeatability
- [ ] The runner (below) green twice consecutively with stable output —
      the harness determinism discipline.

## 3. Deliverables

- [ ] `scripts/burn_e2e.sh` (the `replay_e2e.sh` precedent): drives T1–T7 on
      M3 locally + the 4090 leg over SSH; refuses off-devnet configs; prints
      a per-track verdict table.
- [ ] Bench record (`.benchmarks/`, named for this plan) with the full
      evidence table + the reconcile outputs.
- [ ] HISTORY/AGENTS notes in riir-clippy (client half) and riir-dapps
      (server observations); Issue 094 cross-referenced.

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
