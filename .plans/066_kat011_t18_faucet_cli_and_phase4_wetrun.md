# Plan 066 — Proposal 011 T1.8 (CLI half) + Phase 4 (T4.1/T4.2/T4.3): the faucet preset and the wet-run harness

Status: **IN FLIGHT (2026-09-02)** — T1.8's client seam LANDED (`d72bbb6`,
cross-repo golden riir-dapps `1736010`).
Branch: `develop`
Issue: [`.issues/061_proposal_011_product_loop_rollout.md`](../.issues/061_proposal_011_product_loop_rollout.md)
Proposal: [`.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md`](../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md) §7 / §8.1 / G-WET
Blocked-by (named, NOT papered over): riir-dapps
[Issue 010](../../riir-dapps/.issues/010_treasury_grant_floor_equals_max_supply_unreachable_path.md)

## The blocker, stated first

`TREASURY_GRANT_FLOOR_DEFAULT == MAX_SUPPLY_PLACEHOLDER` (both 10¹²) in
riir-dapps, so `crank.rs`'s `*after >= c.treasury_grant_floor` filter can never
be satisfied and the daily crank **skips every account, every day**. The crank
is deliberately treasury-ONLY (caveat #2 forbids a mint fallback on an
unattended recurring path), so the skip is the *correct* behavior of a
*misconfigured* deployment.

**Consequence for this plan:** wet-run steps 2 and 5 — the faucet legs — cannot
succeed against a genesis-default deployment. The harness therefore must

1. **detect** the skip from the crank report the service already serves
   (`skipped_treasury > 0` with `credited == 0`),
2. **fail loudly and diagnostically**, naming riir-dapps Issue 010 and the two
   constants, and
3. **never** retry-forever, hang, or report the run as a pass.

The happy-path faucet legs are exercised against an **explicitly overridden
floor**, and the test says out loud that the override is what makes them
reachable. No riir-dapps constant is touched here — the replacement value is a
supply decision and an owner act (Issue 010 T2).

## T1 — `cargo heal faucet --request` (Issue 061 T1.8, the CLI half)

- [x] T1.1 the client seam in `src/kat_payreq_client.rs` — `FAUCET_LABEL`,
      `faucet_message`, `faucet_request_body`, `FaucetRequest`,
      `parse_faucet_request`, `PayreqTransport::post_faucet` (no default impl).
      **LANDED `d72bbb6`.**
- [x] T1.2 the golden digest pinned on BOTH repos, each side computed
      independently of either implementation (`b3sum` over the documented byte
      layout; method validated by first reproducing the already-pinned
      `create_message` golden bit-for-bit). **LANDED** — client `d72bbb6`,
      service riir-dapps `1736010`.
- [ ] T1.3 `faucet_request_flow` — one signed POST over the existing
      `HttpPayreqTransport`, the link-first render, and the pinned amount +
      expiry **displayed, never proposed**.
- [ ] T1.4 CLI wiring: `KatCommand::Faucet`, the usage line, the parser arms,
      the per-command flag guard. `--amount` is deliberately REFUSED on
      `faucet` (accepting it would advertise a choice the wire does not carry).
      `tests/heal_usage_flag_parity.rs` must stay green with **no new
      `NOT_HEAL_FLAGS` exemption**.

## T2 — `cargo heal wetrun` + G-WET (Issue 061 T4.1)

- [ ] T2.1 `src/kat_wetrun.rs` — the six §7 steps as a **pure step machine**
      over a transport trait, so every branch is drivable from a scripted mock
      with no cluster. Steps 1 and 6 reuse T0.2b's landed reads
      (`kat_account_client`: `/health`, `/mining/health`, `/balance`, the
      three-state `WelcomeGate`); the per-node reachability/height/sync% table
      is riir-deployer's `node-sync` (T3.2, landed there) and is **not** forked
      a third time — the harness points at it.
- [ ] T2.2 `SupplyCounters` read from `GET /telemetry?epoch=` with a
      **total-function `to_bytes`**, so the G-WET claim is literally
      bit-comparison and a future counter is covered automatically (the
      riir-dapps G-CRANK `Meta` pattern). `size_of` is pinned so a field added
      without extending the encoding reds the gate instead of escaping it.
- [ ] T2.3 the Issue-010 diagnostic path: the crank report's
      `skipped_treasury`/`credited` pair → a named FAIL.
- [ ] T2.4 G-WET in `tests/kat_wetrun_gwet.rs`: full run passes, **mint/burn
      bit-unchanged**, the refund lands exactly X once and is faucet-funded,
      the skip path fails loudly, mainnet refuses, dry-run touches nothing.

## T3 — mainnet refusal + `--dry-run` (Issue 061 T4.2)

- [ ] T3.1 network classification from the RESOLVED service URL, fail-closed:
      an unclassifiable host is refused for the money run exactly like mainnet.
      **There is no override env var at all** — per caveat #2 an override would
      be "an attack on our own bookkeeping", so the refusal has no bypass to
      test, and a source-level invariant test pins that no env read can widen
      it.
- [ ] T3.2 `wetrun --dry-run` — steps 1 + 6 only, allowed on **every** network
      including mainnet, proven to touch nothing by asserting the mock's
      recorded call log contains ONLY the read set and zero writes.

## T4 — install.sh copy alignment + the foo end-to-end doc (Issue 061 T4.3)

- [ ] T4.1 verify the surface before editing: per
      [Proposal 003](../.proposals/003_binary_only_distribution.md) the install
      surface is **gist-rs/cargo-heal** (public dist repo), NOT riir-clippy
      releases (that repo is private — assets need auth, and flipping it public
      would leak the whole source).
- [ ] T4.2 align the drifted copy and pin it by test.
- [ ] T4.3 the foo end-to-end doc (install → login → account → pay --request →
      faucet --request).

## Not this plan

- **T4.4 (testnet wet-run executed)** stays `- [-]`: it needs the owner-executed
  genesis ceremony (Issue 061 T2.2 / T2.3), which authors real testnet key
  material and writes to nodes as root (caveat #8). Not an agent act.
- Creating the `gist-rs/cargo-heal` public repo (Proposal 003 T1.1) — verified
  absent 2026-09-02 (`gh repo view` → *"Could not resolve to a Repository"*).
  Publishing binaries of private source is an owner/security decision.

## Gates this plan must not move

`cargo test --lib` = **1008** at default features (G3), `kat_account_g9` 6/6,
`kat_stake_client_g2s` 9/9, `kat_vote_client_g9gov` 5/5, `kat_meter_g9` 11/11,
`heal_usage_flag_parity` 3/3. The 14/14 retrieval gate (`src/draft/retrieval_eval.rs`)
sits at EXACTLY zero margin — corpus/retrieval files are untouched here.
