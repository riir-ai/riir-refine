# Plan 066 — Proposal 011 T1.8 (CLI half) + Phase 4 (T4.1/T4.2/T4.3): the faucet preset and the wet-run harness

Status: **COMPLETE (2026-09-02)** — T1 + T2 + T3 + T4 all LANDED: `d72bbb6`
(client seam) + cross-repo golden riir-dapps `1736010` + `aecef31` (faucet CLI
+ the wet-run harness) + `1016754` (G-WET, every gate canaried) + `9b0e777`
(install copy + the foo doc). Issue 061 **T1.8, T4.1, T4.2 and T4.3 are
closed**; **T4.4 stays deferred** — a testnet wet-run needs the
owner-executed genesis ceremony (061 T2.2/T2.3).
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
- [x] T1.3 `faucet_request_flow` — one signed POST over the existing
      `HttpPayreqTransport`, the link-first render, and the pinned amount +
      expiry **displayed, never proposed**. **LANDED `aecef31`** — plus a real
      hole closed on the way: `HttpPayreqTransport` DROPPED the body of every
      non-200, and every payreq refusal IS a non-200 carrying
      `{"error": "<the operator's words>"}`, so the documented "the four honest
      refusals survive verbatim" contract was unreachable on the deployed path
      while every mock-driven test passed (`refusal_body_or_status`, POST arms
      only — the GET arms answer 404 for the ordinary not-settled-yet case).
- [x] T1.4 CLI wiring: `KatCommand::Faucet`, the usage line, the parser arms,
      the per-command flag guard. `--amount` is deliberately REFUSED on
      `faucet` (accepting it would advertise a choice the wire does not carry).
      `tests/heal_usage_flag_parity.rs` must stay green with **no new
      `NOT_HEAL_FLAGS` exemption**. **LANDED `aecef31`** — `faucet --request [--qr]`
      and `wetrun [--dry-run]`; `--amount/--label/--wait` stay `pay`-only and
      each refusal says why. Parity 3/3, no exemption added.

## T2 — `cargo heal wetrun` + G-WET (Issue 061 T4.1)

- [x] T2.1 `src/kat_wetrun.rs` — the six §7 steps as a **pure step machine**
      over a transport trait, so every branch is drivable from a scripted mock
      with no cluster. Steps 1 and 6 reuse T0.2b's landed reads
      (`kat_account_client`: `/health`, `/mining/health`, `/balance`, the
      three-state `WelcomeGate`); the per-node reachability/height/sync% table
      is riir-deployer's `node-sync` (T3.2, landed there) and is **not** forked
      a third time — the harness points at it. **LANDED `aecef31`**.
- [x] T2.2 `SupplyCounters` read from `GET /telemetry?epoch=` with a
      **total-function `to_bytes`**, so the G-WET claim is literally
      bit-comparison and a future counter is covered automatically (the
      riir-dapps G-CRANK `Meta` pattern). `size_of` is pinned so a field added
      without extending the encoding reds the gate instead of escaping it, and one
      divergence probe per field proves each field actually moves the bytes. The
      epoch rides IN the encoding, so a comparison across two epochs cannot
      masquerade as bit-unchanged. **LANDED `aecef31`**.
- [x] T2.3 the Issue-010 diagnostic path: the crank report's
      `skipped_treasury`/`credited` pair → a named FAIL. **LANDED `aecef31`** —
      `classify_faucet_leg` checks `skipped_treasury` BEFORE every other skip
      reason (it marks a misconfigured DEPLOYMENT; the others mark a correctly
      refused ACCOUNT), and `issue_010_diagnostic` prints the issue, both
      constants, "STRUCTURALLY UNREACHABLE", "NOT retryable", the owner-act
      unblock, and what makes the legs reachable meanwhile.
- [x] T2.4 G-WET in `tests/kat_wetrun_gwet.rs`: full run passes, **mint/burn
      bit-unchanged**, the refund lands exactly X once and is faucet-funded,
      the skip path fails loudly, mainnet refuses, dry-run touches nothing.
      **LANDED `1016754`** — 11/11, the exact 13-call sequence pinned, and
      THREE canaries run and reverted (a moved mint micro flips the verdict
      while every step still passes; a money leg inside the dry-run branch reds
      the call-log assertion; reordering `classify_faucet_leg` reds the
      Issue-010 ordering test). The dry-run canary also exposed that
      `Step::in_dry_run` was DECORATIVE — widening it changed nothing — so
      `finish` now refuses a dry-run report containing a money leg.

## T3 — mainnet refusal + `--dry-run` (Issue 061 T4.2)

- [x] T3.1 network classification from the RESOLVED service URL, fail-closed:
      an unclassifiable host is refused for the money run exactly like mainnet.
      **There is no override env var at all** — per caveat #2 an override would
      be "an attack on our own bookkeeping", so the refusal has no bypass to
      test, and a source-level invariant test pins that no env read can widen
      it. **LANDED `aecef31`** — the module reads the environment NOWHERE
      (audited at source, line comments stripped: the first cut failed on this
      module's own docs naming the hatch that must not exist, and the audit
      canaries its predicate in both directions). Live-verified: with
      `RIIR_ALLOW_MAINNET_WETRUN`, `RIIR_DEPLOY_ALLOW_MAINNET` and
      `RIIR_HEAL_ALLOW_MAINNET` all set at once the binary still exits 2 with
      REFUSED. The production default host carries no network word, so it is
      refused too.
- [x] T3.2 `wetrun --dry-run` — steps 1 + 6 only, allowed on **every** network
      including mainnet, proven to touch nothing by asserting the mock's
      recorded call log contains ONLY the read set and zero writes. **LANDED**
      `aecef31` + `1016754` — exactly six GETs, zero POSTs, no telemetry read,
      the scripted ledger bit-identical afterwards, on all four networks.

## T4 — install.sh copy alignment + the foo end-to-end doc (Issue 061 T4.3)

- [x] T4.1 verify the surface before editing: per
      [Proposal 003](../.proposals/003_binary_only_distribution.md) the install
      surface is **gist-rs/cargo-heal** (public dist repo), NOT riir-clippy
      releases (that repo is private — assets need auth, and flipping it public
      would leak the whole source). **DONE `9b0e777`** — and `gh repo view
      gist-rs/cargo-heal` answered *"Could not resolve to a Repository"*
      (2026-09-02): the dist repo has never been created (Proposal 003 T1.1).
- [x] T4.2 align the drifted copy and pin it by test. **LANDED `9b0e777`** —
      the drift was real: Proposal 005 served both installers from the vanity
      host `cargo-heal.gist.rs` (does not exist) while Proposal 011 §8.1 used
      the `raw.githubusercontent.com` form. Proposal 003 now DECLARES the
      canonical pair; `tests/install_copy_parity.rs` (3/3) fails on any other
      install URL in any tracked `*.md` and on any pointer at riir-clippy's
      own private releases. The audit fired twice while being written.
- [x] T4.3 the foo end-to-end doc. **LANDED `9b0e777`** —
      `.docs/01_orientation/foo_end_to_end.md`, every step marked runnable-now
      or blocked-and-why (a doc that reads uniformly runnable when a third of it
      is blocked is worse than none).

## Not this plan

- **T4.4 (testnet wet-run executed)** stays `- [-]`: it needs the owner-executed
  genesis ceremony (Issue 061 T2.2 / T2.3), which authors real testnet key
  material and writes to nodes as root (caveat #8). Not an agent act.
- Creating the `gist-rs/cargo-heal` public repo (Proposal 003 T1.1) — verified
  absent 2026-09-02 (`gh repo view` → *"Could not resolve to a Repository"*).
  Publishing binaries of private source is an owner/security decision.

## Gates this plan must not move — MEASURED 2026-09-02, all held

Every figure is read off the `test result:` line. A pipeline exit code is not
evidence: piping cargo into `tail` has reported exit 0 for a FAILED test in
this repo.

| Gate | Recorded | Measured | Command |
|---|---|---|---|
| G3 default-features lib | 1008 | **1008** / 0 failed / 0 ignored | `cargo test -p riir-clippy --lib` |
| `kat_account_g9` | 6 | **6** / 0 / 0 | `--features kat_transport,latent_retrieval,score_bench --test kat_account_g9` |
| `kat_stake_client_g2s` | 9 | **9** / 0 / 0 | same, `--test kat_stake_client_g2s` |
| `kat_vote_client_g9gov` | 5 | **5** / 0 / 0 | same, `--test kat_vote_client_g9gov` |
| `kat_meter_g9` | 11 | **11** / 0 / 0 | same, `--test kat_meter_g9` |
| `heal_usage_flag_parity` | 3 | **3** / 0 / 0 | same, `--test heal_usage_flag_parity` |
| retrieval gate | zero margin | **12** / 0 / 0 (1103 filtered) | `--features latent_retrieval --lib retrieval_eval` |

**Note on the retrieval row:** the `retrieval_eval` filter selects **12**, not
the 14 carried in the hand-off note. Recorded as measured rather than
reconciled — no corpus, retrieval or draft file is touched by this plan, and
all 12 pass. Whoever owns that figure should re-derive which selector produces
14 before treating either number as the pin.

### New surfaces (the additions, not a moved gate)

| Surface | Count | Command |
|---|---|---|
| `kat_payreq_client` unit | **21** / 0 / 0 (was 16) | `--features kat_transport --lib kat_payreq_client` |
| `kat_wetrun` unit | **11** / 0 / 0 | `--features kat_transport --lib kat_wetrun` |
| `kat_wetrun` unit | **12** / 0 / 0 | `--features kat_transport,latent_retrieval --lib kat_wetrun` |
| G-WET | **11** / 0 / 0 | `--features kat_transport --test kat_wetrun_gwet` |
| install copy parity | **3** / 0 / 0 | `--test install_copy_parity` |
| full lib, kat+retrieval | **1297** / 0 / 1 ignored | `--features kat_transport,latent_retrieval --lib` |

The `kat_wetrun` unit count differs by feature **by design**: the 12th test
cross-checks the redact floor against `kat_sync_payload::PAYLOAD_VERSION`,
which only exists under `latent_retrieval`. Both counts are recorded because a
cfg-gated test compiles to nothing and reports "0 passed" rather than failing.

Clippy: `--features kat_transport,latent_retrieval --all-targets --keep-going`
→ **0 warnings, 0 errors**, after `touch`ing every changed file (a warm clippy
emits nothing, so an untouched zero is not evidence).
