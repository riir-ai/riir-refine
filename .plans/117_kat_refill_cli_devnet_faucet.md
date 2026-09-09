# Plan 117 — `cargo heal --refill`: the dev/test faucet refill (client half)

**Status:** PLANNED — 2026-09-09 (code unblocked; CREDIT blocked on riir-dapps Issue 025 owner call + devnet treasury funding — see T0)

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

- [ ] T0 — Prereqs (owner/ops, NOT this repo's code; plan is executable around them, refill prints
      the honest refusal until they land): (a) fund/verify devnet treasury ≥ 100K floor
      (`scripts/setup.sh` bootstrap or `POST /admin/treasury/fund`); (b) riir-dapps Issue 025 owner
      call; (c) Plan 079 refund-leg decision (a vs b).
- [ ] T1 — CLI flag `--refill` (flag-mode like `--mine`/`--unmine`, per the owner's exact ask):
      parser arm + `Args` field + `validate_mode_flags` + `usage()` row. Keep
      `tests/heal_usage_flag_parity.rs` green WITHOUT a `NOT_HEAL_FLAGS` exemption (Plan 064
      lesson); a `kat_service`-off build must still parse and error cleanly (KatCommand doc
      convention, `heal.rs:47-50`).
- [ ] T2 — `src/kat_refill.rs` (pure core + transport trait, `kat_service`-gated; HTTP half under
      `kat_transport` like every network surface): reuse `kat_payreq_client::faucet_message` +
      `post_faucet` (golden digest already pinned cross-repo — no new wire bytes); classify the four
      service refusals verbatim (disabled / service account / cap used / one already open).
- [ ] T3 — Authorized crank trigger leg: if `RIIR_HEAL_KAT_CRANK_TOKEN` is set → reuse the wetrun
      `post_crank` transport (`POST /admin/crank/daily`); else print the honest line "settles on the
      next crank pass (≤ ~1 h on devnet; export RIIR_HEAL_KAT_CRANK_TOKEN to force)". Never require
      the token; fail-soft like every ops surface.
- [ ] T4 — Bounded settle wait + server-truth readback: `--refill-wait SECS` (default 0 = no poll;
      poll interval ≥ 5 s, hard poll cap); `GET /balance?acct=` (public route) → one-line
      `kat-fuel: <render_kat>` verdict. The command's exit code IS its outcome (explicit command —
      the burn push's never-blocks contract is untouched).
- [ ] T5 — Soft-gate refresh: on confirmed `balance_micro > 0`, write `last_balance_micro` into the
      burn ack (`.heal/kat_burn_ack.json`, stays v3, `burn_kat_total` + `contributed` untouched) so
      the Issue 075 gate lifts on server truth without waiting for a new burn. Gate input remains
      the ack — never the local projection (`kat_soft_gate.rs:16-19` rule).
- [ ] T6 — Refusal copy update: the depleted-gate refusal gains `cargo heal --refill` as the first
      refuel path beside contribute + pay URL (`heal.rs:336-346`). Update the `"top up"` text
      assertions in `tests/kat_soft_gate_cli.rs:185` deliberately in the same commit.
- [ ] T7 — Tests: unit tests for refusal classification + ack refresh + gate-lift predicate; CLI
      contract test driving the real bin (`CARGO_BIN_EXE_cargo-heal`, closed-port + seeded ack
      pattern of `tests/kat_soft_gate_cli.rs`); golden-digest parity inherits from the existing
      cross-repo pins (nothing new to mirror).
- [ ] T8 — Docs: README §KAT network + §soft gate (refuel paths = contribute | `--refill` | pay URL),
      AGENTS.md CLI-UX-contract bullet, `.docs/01_orientation/foo_end_to_end.md` refill walkthrough,
      HISTORY.md record; update `.docs` foo wetrun §7 cross-links.
- [ ] T9 — riir-dapps riders (file THERE when picked up, reference here): optional `env`/`mode` field
      in `GET /` service info so the client can label devnet/testnet; confirm faucet rows settle on
      the live devnet's hourly alarm (workerd e2e already proves the alarm leg at 5 s).
- [ ] T10 — Validation + land: `cargo clippy` (heal-first per house rule) + the T7 suites +
      `./scripts/ci_feature_guard.sh` locally (develop has no CI trigger); stage named files only
      (`scripts/staged_set_audit.py` before the multi-file commit); commit `feat:` on develop, push.

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
