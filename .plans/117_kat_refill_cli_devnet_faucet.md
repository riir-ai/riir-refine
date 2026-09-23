# Plan 117 — `cargo heal --refill`: the dev/test faucet refill (client half)

**Status:** COMPLETE — client half implemented 2026-09-09; CREDIT blocker DISCHARGED 2026-09-10 (server half landed: riir-dapps genesis re-scale resolved Issue 025, settlement drain resolved Issue 035; live smoke `--refill --refill-wait 45` exit 0, burn-ack server truth written — AGENTS.md records the owed item DISCHARGED). Status-corrected 2026-09-23 by the idle-loop plan-status hygiene pass.

## The verdict (grep + prior art, 2026-09-09)

**What the owner asked:** a Cloudflare authorized-node faucet for devnet/testnet behind a new
`cargo heal --refill`, so a depleted client can fill KAT and continue daily batches / testing on the
built binary. "Manual request proposed before" — dev-only, not prod.

**What already exists (do NOT rebuild any of this):**

| Surface | Where | State |
|---|---|---|
| Faucet money route `POST /payreq/faucet` | riir-dapps `cloudflare/kat-service/src/routes.rs:1568-1634`, substrate `src/kat/crank.rs:459-504` | SHIPPED — no amount/TTL on the wire (both server config), refuses disabled cap / service account / cap-used / second-open |
| Faucet CLI `cargo heal faucet --request` | Plan 066 COMPLETE (`d72bbb6`, `aecef31`), golden `kat:payreq:faucet:v1:` digest pinned BOTH sides (`tests/kat_payreq.rs:582-631` ↔ `kat_payreq_client.rs:98-103`) | SHIPPED |
| Once-per-key welcome grant `POST /account/claim` + `ensure_grant_flow` wired into `sync`/`account` | Plan 079 COMPLETE (2026-09-05) | SHIPPED |
| Authorized crank trigger `POST /admin/crank/daily` (bearer `KAT_CRANK_TOKEN`, fail-closed) + hourly DO alarm | riir-dapps `do_host.rs:626-684`, `routes.rs:1694-1717` | SHIPPED — wetrun already drives both (`src/kat_wetrun.rs:507-547`, `RIIR_HEAL_KAT_CRANK_TOKEN`) |
| Soft gate refuel paths | Issue 075 (README:324-337), Issue 079 contribute-now offer | SHIPPED — but "top up" names only the web payment URL `/pay?acct=` |

**Grep verdict:** no `refill`/`topup`/`top_up` identifier exists in any `.rs` file; "tops up" is named
but never implemented (`kat_soft_gate.rs:5-7`, `heal.rs:340-342`, `heal.rs:6409-6413`, Issue 079 L46).
The earlier manual-request proposal = Proposal 011 ask 11→13 (faucet by QR → **faucet by LINK**,
L334-346; Discord bot deferred) — that landed as `faucet --request`. `--refill` is therefore **not a
new money surface**: it is the dev-workflow composition layer on shipped primitives:

```
--refill = faucet request (signed) → [optional authorized crank trigger] →
           bounded settle wait → GET /balance (server truth) → soft-gate ack refresh
```

**The REAL blockers are ops/server, not client code** (refill plumbing credits NOTHING until these land):

1. **Treasury floor gates every crank grant leg.** `treasury_grant_floor` = 100,000 KAT; crank SKIPS
   (`skipped_treasury`, never mints) while below floor — observed LIVE on devnet (Issue 079 L20-22:
   `GET /balance → 0`, `skipped_treasury:1`). Fix is ops: `scripts/setup.sh` devnet funds 500K KAT
   via `POST /admin/treasury/fund` (`setup-devnet-bootstrap-v1`) — re-run/verify on the live worker.
2. **riir-dapps Issue 025 (OPEN, owner call):** free-grant units mismatch (client README says 100M KAT;
   `FREE_GRANT_KAT = 100_000_000` micro = **100 KAT**); m3 account at balance 0 needs explicit refuel.
   Refill unblocks m3 the moment the treasury is funded.
3. **Settle latency = crank cadence.** Faucet rows settle on the NEXT crank pass (hourly alarm default
   3600 s; `POST /admin/crank/daily` forces it). Without a crank token, `--refill` on devnet waits
   ≤ ~1 h — the command must say so, not spin.
4. **Daily cap.** `FAUCET_DAILY_CAP_KAT_DEFAULT` = 10 KAT/day/account; devnet override
   `KAT_FAUCET_DAILY_CAP_KAT` exists (`do_host.rs:409-421`). Plan 079 Finding 3: `faucet_request_create`
   pins the FULL cap, so a same-day second draw is always refused — interacts with the wetrun's
   two-draws-per-run shape; Plan 079's refund-leg owner decision (a: settled-invoice readback
   [recommended] vs b: free-tier leg) should be resolved alongside T0.

## Environment posture (dev/test only, prod untouched)

| Env | URL | Clock | Refill behavior |
|---|---|---|---|
| localnet | `RIIR_HEAL_KAT_SERVICE_URL=http://127.0.0.1:8811` | virtual | full loop + crank trigger |
| devnet (the authorized CF node) | `https://riir-kat-service.foxfox.workers.dev` | virtual | full loop + crank trigger |
| testnet | same service, wall clock | wall | faucet per server config; crank leg same rules |
| mainnet | — | wall | **no client bypass**: faucet refusal is SERVER config (Proposal 011: mainnet refusal, no override); the client never carries prod credentials — the crank leg requires `RIIR_HEAL_KAT_CRANK_TOKEN`, which prod does not distribute |

The client is URL-honest: it prints the exact service URL it drives. It adds no new credential class.

## Tasks

- [-] T0 — Prereqs (owner/ops, NOT this repo's code; plan is executable around them, refill prints
      the honest refusal until they land): (a) fund/verify devnet treasury ≥ 100K floor
      (`scripts/setup.sh` bootstrap or `POST /admin/treasury/fund`); (b) riir-dapps Issue 025 owner
      call; (c) Plan 079 refund-leg decision (a vs b). — OWNER-GATED, deferred
- [x] T1 — CLI flag `--refill` (flag-mode like `--mine`/`--unmine`, per the owner's exact ask):
      parser arm + `Args` field + validate_mode_flags-path guard + `usage()` rows (`--refill`,
      `--refill-wait <s>` + the `--refill-wait requires --refill` guard). `tests/heal_usage_flag_parity.rs`
      green WITHOUT a `NOT_HEAL_FLAGS` exemption (Plan 064 lesson); a `kat_service`-off build parses
      and errors cleanly (KatCommand doc convention).
- [x] T2 — `src/kat_refill.rs` (pure core + transport trait, `kat_service`-gated; HTTP half under
      `kat_transport`): reuses `kat_payreq_client::faucet_message` + the golden-pinned faucet POST;
      classifies the four service refusals verbatim (the composed `HttpPayreqTransport` carries the
      refusal-body rule). DECISION vs plan draft: NO client-side host admission — the wetrun
      classifier names the production default (`heal.gist.rs`, the devnet front) Unclassifiable, so
      an admission rule would break the primary use case; the faucet's gate is the DEPLOYMENT's cap
      config and the crank's authorization is the bearer itself (module doc owns the reasoning).
- [x] T3 — Authorized crank trigger leg: `RIIR_HEAL_KAT_CRANK_TOKEN` set → the wetrun's bearer
      `POST /admin/crank/daily` (transport COMPOSED, not forked); unset → `CrankLeg::Skipped` + the
      honest render line. Fail-soft: a refused crank never fails the refill; `skipped_treasury`
      surfaces as the Issue-010/025 diagnosis.
- [x] T4 — Bounded settle wait + server-truth readback: `--refill-wait SECS` (default 0 = one
      after-read; poll interval 5 s, monotone budget, early-break on the credit);
      `kat-fuel: <render_kat>` line. Exit contract: **0 iff the final synced balance is positive**
      (a cap-used refusal with fuel in the account is exit 0 — the goal was fuel).
- [x] T5 — Soft-gate refresh: the confirmed balance is written into the burn ack
      (`record_server_balance`, stays v3, `burn_kat_total` + `contributed` untouched) so the Issue
      075 gate lifts on server truth. SERVER TRUTH WINS whatever the value — a depleted read is
      WRITTEN (the issue-080 lesson applied to the balance leg), never stale optimism.
- [x] T6 — Refusal copy update: the depleted-gate refusal names `cargo heal --refill` FIRST (the
      dev escape hatch), beside contribute + pay URL; "Refuel with any of:"; the end-of-run
      0-KAT line gained `(or: cargo heal --refill)`. `tests/kat_soft_gate_cli.rs` rows all green
      (text assertions extended, none weakened) + a new order-pinning row in `kat_refill_cli.rs`.
- [x] T7 — Tests: 7 unit (`kat_refill::tests`: ack-refresh purity, exit table, happy path,
      refusal-continues, pinned-zero, poll early-break, copy rule) + 4 CLI through the real binary
      against a std-TCP mock (`tests/kat_refill_cli.rs`, the `kat_burn_stack` shape — no-account
      refusal; live happy path asserting the ack refresh + the bearer ON THE WIRE; the gate naming
      refill; the `--refill-wait` guard). Golden-digest parity inherits from the existing
      cross-repo pins (zero new wire bytes). Cargo.toml `[[test]] kat_refill_cli` row added (the
      Issue-713 green-zero rule).
- [x] T8 — Docs: README §KAT network (the `--refill` subsection) + §soft gate (three refuel
      paths), AGENTS.md CLI-UX bullet (the full posture + upstream-blocked note),
      `.docs/01_orientation/foo_end_to_end.md` §7b (vs `faucet --request`), HISTORY.md record.
- [-] T9 — riir-dapps riders: service-info `env`/`mode` field (optional display nicety) +
      live-devnet crank confirmation — FILED as riir-dapps `.issues/029_plan117_service_info_env_and_live_crank.md`
      (`fa5162c`); the heavy lift there stays Issue 025 (owner call).
- [x] T10 — Validation + land: `cargo clippy --all-targets` green at the bin feature set +
      `--lib` at kat_service,kat_transport; the T7 suites + kat_soft_gate_cli (13) +
      heal_usage_flag_parity (3) green; named-file staging only; commit `feat:` on develop, push.

## GOAT gate (CLI-shaped — no perf bench applies; correctness + budget)

- **G1 correctness:** zero new wire bytes (faucet message golden parity inherits); refusal table
  verbatim; ack refresh server-truth only (GET /balance), `contributed` flag never client-written.
- **G2 budget:** bounded wait (default 0), ≤ 1 crank POST per invocation, poll interval ≥ 5 s with a
  hard cap; no background work, no exit-code side effects outside the explicit command.
- **G3 no regression:** all `kat_soft_gate_cli.rs` rows green (copy update deliberate), burn
  never-blocks + wetrun byte-shape untouched (transport fns REUSED, not forked).
- **G4:** n/a — one-shot CLI, no hot loop.
- **Feature flag decision: NONE new.** Rides the shipped release set
  `default + latent_retrieval + kat_service + kat_transport` (since `v0.1.2`, `56bd9bf`) — same
  gating as `sync`/`faucet`. Promotion question does not arise (no default-flip: the surface is
  transport-gated exactly like its siblings).

## References

- riir-dapps: Issue 025 (refuel gap, owner call) · Issue 027 (v3 ack) · Plan 015 (grant crank +
  faucet expiry) · Plan 079-equivalent `account/claim` + `admin/treasury/fund` routes ·
  `cloudflare/kat-service/src/routes.rs` (route table L8-54) · `scripts/setup.sh` (env funding).
- riir-clippy: Proposal 011 (faucet by LINK) · Plans 066 (faucet CLI) · 079 (claim grant + wetrun
  findings) · Issues 075 (gate, README record) / 079 (mine offer, live-depleted evidence) / 080
  (server-truth contributed) · `src/kat_wetrun.rs` (the programmatic-faucet precedent).
