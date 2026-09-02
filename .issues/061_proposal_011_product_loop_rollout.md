# Issue 061 — Proposal 011 rollout: the KAT product loop (pay-request, genesis ceremony, deploy UX, wet-run, onboarding)

Status: **OPEN (2026-09-01) — Phase 0 LANDED; Phase 1's T1.1/T1.3/T1.4/T1.8
LANDED; Phase 2's T2.1 + ALL of Phase 3 LANDED; Phase 4's T4.1/T4.2/T4.3
LANDED 2026-09-02.** Phase 4's engineering half is complete (riir-clippy
`d72bbb6` + `aecef31` + `1016754` + `9b0e777` + riir-dapps `1736010`,
[Plan 066](../.plans/066_kat011_t18_faucet_cli_and_phase4_wetrun.md)): the
faucet preset, the six-step wet-run harness with G-WET (mint/burn asserted
BIT-unchanged over the whole counter struct), the no-override mainnet refusal,
the measured `--dry-run` read-only proof, and the install-copy pin. **T4.4 is
the only Phase 4 row left and it is an owner act** (the genesis ceremony, T2.2
/T2.3); the wet-run's money legs are additionally blocked by riir-dapps
**Issue 010** (the treasury grant floor equals max supply, so the daily crank
skips every account) — the harness FAILS loudly and diagnostically on that
path rather than retrying or reporting a pass.
Phase 0 (T0.1 + T0.5 + the ledger/payreq halves of T0.4: riir-dapps
`6df7097`, [Plan 015](../.plans/015_kat011_phase0_payreq_substrate.md) — the
`kat_payreq` request-ledger rows, invoice rows, signed registration intents,
first-wins funding bindings, and the KAT-rail one-batch settle seam). **T0.3
LANDED 2026-09-02** (riir-chain-sdk `fe865ba1` + the dapps-side dual golden pin
`46e6660`). **T1.1 LANDED 2026-09-02** (riir-dapps `f43c014`,
[Plan 062](../.plans/062_kat011_t111_payreq_routes.md) — the signed `/payreq`
routes + the row-backed limiter). **T1.3 LANDED 2026-09-02** (riir-clippy
`9a5086a` + the dapps-side create-message golden `0a4d37a` — `cargo heal pay
--request`). **T0.2b + T1.4 LANDED 2026-09-02** (riir-clippy `dc9e179`,
[Plan 064](../.plans/064_kat011_t02b_account_service_and_t14_invoice_qr.md) —
the service-plane welcome gate + `/balance` history, and the invoice display
with the caveat-#3 QR pin canaried on both axes). **Phase 0 is now COMPLETE.**
**T2.1 + T3.1 + T3.2 + T3.3 LANDED 2026-09-02** (riir-deployer `96009cc`,
[Plan 003](../../riir-deployer/.plans/003_genesis_params_deploy_ux_node_sync.md) —
the genesis params pin with drift-FAILS validation, the TTY selector + the
env-gated mainnet boundary, the `node-sync` fleet gate, and G-DEPLOY RUN on a
real local 3-node cluster; 155 tests green, +27). Phase 2's remaining T2.2/T2.3
are OWNER acts, and the dry-run harness they need is shipped.
The proposal is draft and the phased scope survives the noise-reduction rule via
this tracker.
Branch: `develop` (per global rule — no feature branches)
Proposal: [`.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md`](../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md)
Related: [Issue 059](059_proposal_007_remainder_ops_signer_and_counsel_gate.md) (the
shared counsel envelope), [Issue 060](060_proposal_008_remainder_new_plan_and_gated_levers.md)
(the crank's grant machinery context), riir-deployer
[Proposal 001](../../riir-deployer/.proposals/001_riir_control_cf_worker_do.md), riir-chain
Proposal 008 (MemoShard).

## Phase 0 — ledger + SDK substrate

- [x] T0.1 `payreq` request-ledger lifecycle rows + invoice rows + faucet credit row
      type (riir-dapps, `kat_payreq`) — **LANDED 2026-09-02** (riir-dapps `6df7097`,
      [Plan 015](../.plans/015_kat011_phase0_payreq_substrate.md)): `kat:req:`
      lifecycle rows (status machine `Created→Resolved→Settled|Expired|Refused`,
      terminal states immutable, refusals are rows, `chain_anchor` slot rides every
      row), `kat:inv:` invoice rows (QR commitment slot fills exactly once), the
      faucet REQUEST row kind (the daily-cap crank settles these in T1.7; no separate
      credit row needed — the kind enum is the row type).
- [x] T0.2 clippy CLI: `login` default-SSH adoption + `RIIR_HEAL_KAT_KEY` override;
      `account` welcome + `--history` (local view; the copy rule test-enforced) —
      **LANDED** ([Plan 059](../.plans/059_kat011_t02_and_corpus_runtime_counts.md)).
      The service half (live `/balance`, the all-synced + nodes-healthy welcome gate,
      server-side history) is the `[-]` below.
- [x] T0.2b — the service half of account — **LANDED 2026-09-02** (riir-clippy
      `dc9e179`, [Plan 064](../.plans/064_kat011_t02b_account_service_and_t14_invoice_qr.md)):
      `src/kat_account_client.rs` — the §8.3 welcome gate evaluated from THREE REAL
      service reads (`GET /health` ledger plane + `GET /mining/health` +
      `GET /balance?acct=`), replacing T0.2's local `no burn yet` heuristic, and the
      §8.5 history read from the authoritative `/balance` instead of the per-repo
      projection. **`WelcomeGate` is three-state** (`Open` / `Withheld` — read the
      network, it is not settled / `Unknown` — could not read it): folding `Unknown`
      into `Open` would print a health claim the CLI cannot see, folding it into
      `Withheld` would tell an offline user their account is out of sync. The
      `pending_durable == 0` check is the all-synced half; the mining `unproven`
      backlog is deliberately NOT a gate (it is the steady state). `service_account_hex`
      pins the 64-hex local account id down to the ledger's 32-hex `AccountId`,
      cross-checked against `account_for_pubkey` — a drift there would query an
      account no route can settle from. Probe fail-fasts on an unreachable host (an
      offline `account` costs ONE timeout, not three). Same transport convention as
      T1.3's payreq client. **Residue, not a blocker:** `/balance` serves a POSITION,
      not an event log — the render says so rather than fabricating a transaction
      list; a per-event history needs a dapps route that does not exist.
- [x] T0.3 riir-chain-sdk `pay_request` module (KAT compose + resolve/transfer; USDC
      arm stubbed behind the mock) — **LANDED 2026-09-02** (riir-chain-sdk
      `fe865ba1`; dapps-side dual golden pin `46e6660`): `request_id` byte-exact
      mirror of the ledger half (golden hex `71b5cbf1…` asserted BOTH sides — the
      `dapps_contract_pins` discipline), `request_url`/`parse_request_url`
      (`{base}/payreq/{32 lowercase hex}`, strict), canonical payload v1
      (`encode/decode_request` — the bytes T1.1's route serves), `verify_request`
      (id cross-check + expiry, the T1.1 signature seam's call site), and
      `kat_transfer_intent` (ask-3 amount-resolution matrix + expiry/token guards;
      executes via `wallet_mut().transfer(payee, kat_slot, amount_minor)`).
      Opt-in `pay_request = []` — pulls NO `chain_prog_*` (settle is the ledger's)
      and stays out of `all-programs`; no signature seam (T1.1's), no QR rendering
      (clippy's `render_qr_ascii`), no USDC settle (the `[-]` rails). 12 module
      tests; validation: lib `pay_request` 24/0 + default 12/0 count-identical +
      goat compile + doctest, clippy 0 both states, dapps `kat_payreq` 11/0.
- [x] T0.4 self-pay fixtures (chain + ledger + payreq) — **ledger + payreq halves
      LANDED 2026-09-02** (Plan 015 tests/kat_payreq.rs: A→A settles to a paid
      invoice, no net supply change, exactly-once; insufficient-balance refusal;
      the `chain` TxDelta A→A half is riir-chain-sdk/chain territory — the chain
      fixtures there cover transfer mechanics; revisit only if the wet-run needs a
      payreq-shaped chain fixture).
- [x] T0.5 registration-intent + funding-binding row types (the ask-12 substrate) —
      **LANDED 2026-09-02** (Plan 015): `kat:uintnt:` signed intents + the fail-closed
      ed25519 verify seam (forged key refused) + `kat:fbind:` FIRST-WINS funding
      bindings (same source idempotent, different source refused with the existing
      binding surfaced). **The binding verdict is decided: the REFERENCE key is the
      mandatory machine link; the memo is mandatory-in-URL audit copy — a
      memo-mandatory gate would strand real payments.**

## Phase 1 — the KAT rail + web wallet (DEFAULT, ships first — owner call 2026-09-01)

- [x] T1.1 `/payreq` routes (create/resolve/settle/invoice) + write-ahead rows +
      limiter-backed-by-rows + signed resolve (riir-dapps; caveat #7 — signature-
      verified, idempotent, rate-limited, no unauthenticated state writes) —
      **LANDED 2026-09-02** (riir-dapps `f43c014`,
      [Plan 062](../.plans/062_kat011_t111_payreq_routes.md)): signed create
      (payee; `blake3(pk)[..16]` account derivation — golden-pinned equal to the
      sync plane's `submitter_for`), the ask-14 write-ahead row before the limiter
      (`KAT_PAYREQ_MAX_OPEN`, refusal is itself a row — G-REQ), the
      capability-URL view (caveat #12 — the request and nothing else), signed
      resolve (payer; idempotent; honest terminal states), signed settle (amount
      INSIDE the signature; ONE-batch transfer + invoice row; replay refused by
      the terminal row), invoice readback. Contract tests 16/16 (lifecycle +
      any-amount self-pay conservation + honest refusals + open-limit), ledger
      goldens 14/14, clippy clean. Expiry (`link_ttl`) stays T1.8 — the row has
      no expiry slot this phase, documented in the plan.
- [-] T1.2 the web wallet on the kat-service host: GitHub OAuth → `.keys` fetch →
      key-match → session → press-and-hold SEND (**server-enforced min-hold** — the
      client's 3 s is cosmetic until the worker enforces `held_ms ≥ 3000`) → settle →
      invoice row. **The root/genesis float stays CLI-only.** The settle/resolve
      signed routes it needs are T1.1's (landed).
- [x] T1.3 clippy `pay --request [--amount]` (KAT default; blank = any amount) +
      link-first (`--qr` fallback) + poll-on-credit (riir-clippy half — after T1.1) —
      **LANDED 2026-09-02** (riir-clippy `9a5086a`; dapps-side create-message
      golden `0a4d37a`): `src/kat_payreq_client.rs` — the create-message MIRROR
      (golden digest pinned BOTH repos, independently computed),
      `parse_kat_amount` (blank = ANY, ask 3), `HttpPayreqTransport` (the
      VoteTransport pattern), `pay_request_flow` (signed create → link-first →
      poll-on-credit, monotone Instant budget — the wall-clock-snapshot spin the
      mock test caught is documented in-source); CLI `pay` +
      `--amount/--qr/--label/--wait` (link-first print per ask 13, `--qr` renders
      ASCII, poll default 600 s). 9 tests incl. the scripted-mock full flow + the
      caveat-#15 no-dollar copy rule; lib 1127/1127; default build untouched (G3).
- [x] T1.4 invoice display + bit-exact recheck — **LANDED 2026-09-02** (riir-clippy
      `dc9e179`, [Plan 064](../.plans/064_kat011_t02b_account_service_and_t14_invoice_qr.md)):
      `InvoiceView`/`parse_invoice`/`QrRecheck`/`invoice_display_lines` +
      `recheck_invoice_qr` in `kat_payreq_client`, and the QR pin in
      `kat_lease_client` (`QrRenderParams`, `QR_RENDER_PARAMS`, `qr_commitment`).
      **A mismatch REFUSES to render the body** (tamper indicator; no amount, no
      payer, no token row — pinned), the unset service sentinel reads as
      `Unavailable` and LABELS itself instead of passing silently (caveat #3's
      hash-match-only fallback), and the caveat-#15 copy rule is pinned over all
      three states. **Caveat #3 made non-vacuous on BOTH axes, two mechanisms each,
      each canaried by reintroducing the defect:** the `qrcode` version rides IN the
      commitment preimage AND is read back from **`Cargo.lock`** (the RESOLVED
      version — the `"0.14"` requirement is satisfied by any `0.14.x`, so a
      manifest-only pin would sit green through a patch bump that re-rendered every
      stored asset); the render params ride in the preimage AND get per-axis
      divergence probes (`EcLevel::{L,Q,H}`, `quiet_zone: false`). Canaries:
      declaring `0.14.2` against the locked `0.14.1` fails 3 tests; flipping the ECC
      level fails 5. The commitment is over the QR **module matrix**, not a PNG —
      this repo carries `qrcode` with `default-features = false`, so there is no
      `image` and no PNG to hash, and every rendering is a function of the matrix.
      **CONTRACT NOTE for the dapps half:** `qr_png_blake3` is `[0u8; 32]` on every
      invoice the service has ever written (`invoice_set_qr_hash` is the unfilled
      render step). Whoever fills it MUST fill it with
      `kat_lease_client::qr_commitment` of the canonical request URL — filling it
      with a PNG hash or different params would make every invoice display as a
      tamper.
- [-] T1.5 G1-S/G-WEB money + auth + adversarial suite (replay, session theft, amount
      manipulation, self-pay, hold-bypass, root-float refusal).
- [-] T1.6 Stripe/USDC rails as MOCK tests only (five-check machinery against
      fixtures; no live rail this phase).
- [-] T1.7 the daily grant crank (alarm pass, `(account, utc_date)` exactly-once,
      activity gate, budget cap) + G-CRANK. **A GRANT crank via M2 treasury-first —
      never a mint faucet; the amount joins the 008 never-lever free-grant family.**
- [x] T1.8 `faucet --request` preset + `faucet_daily_cap_kat` enforcement + `link_ttl`
      expiry (expired = settle-refused) — **LANDED 2026-09-02**. Service half
      riir-dapps `891f9fd`; CLI half riir-clippy `d72bbb6` (the signing mirror +
      wire parse) + `aecef31` (`faucet_request_flow` + `cargo heal faucet
      --request [--qr]`), [Plan 066](../.plans/066_kat011_t18_faucet_cli_and_phase4_wetrun.md).
      The **golden digest is pinned on BOTH repos** to numbers computed
      independently of either implementation (`b3sum` over the documented byte
      layout `kat:payreq:faucet:v1:` ‖ pubkey[32]; the method validated by first
      reproducing the already-pinned `create_message` golden bit-for-bit) — the
      dapps-side half is riir-dapps `1736010`, which did not exist before and
      whose absence the client's own doc comment had already claimed away.
      **The command deliberately refuses `--amount` and `--wait`:** the wire
      carries neither (the ledger pins the amount to `faucet_daily_cap_kat` and
      the TTL to `link_ttl_secs`), and the credit lands at the DAILY crank, so
      a poll would be either a lie or a day-long hang. A pinned amount of 0 is
      an ERROR rather than the pay-request "any amount" convention — otherwise a
      faucet-disabled deployment would open an unbounded request.
      **A real hole closed on the way:** `HttpPayreqTransport` dropped the body
      of every non-200, and every payreq refusal IS a non-200 carrying
      `{"error": "<the operator's words>"}` — so the documented "the four honest
      refusals survive verbatim" contract was **unreachable on the deployed
      path** while every mock-driven test passed. `refusal_body_or_status` now
      carries a JSON error body through (POST arms only; the GET arms answer 404
      for the ordinary not-settled-yet case and their callers already have
      per-state copy for it).

## Phase 2 — genesis ceremony (bar)

- [x] T2.1 params yaml + validation against the pinned tokenomics constants (drift
      FAILS — no silent clamp; 008/010 ratified defaults) — **LANDED 2026-09-02**
      (riir-deployer `96009cc`,
      [Plan 003](../../riir-deployer/.plans/003_genesis_params_deploy_ux_node_sync.md)):
      `riir-deploy genesis --params <f>` + `examples/genesis-{devnet,testnet}.yaml`.
      `src/genesis.rs` carries a **committed pin** of the ratified set (`RATIFIED` +
      seven `LeverBoundPin` copies, each field naming its riir-dapps source symbol);
      `validate()` collects EVERY drift row (field · yaml value · pinned value ·
      provenance) and FAILS — **no clamp**, and the plan does not print for a refused
      file. Beyond equality: split partition Σ 10 000, per-share `LeverBound` range,
      the `epoch_split_bp[1] == staking_share_bp` cross-check, both **saturation**
      properties (`reward_decay == DECAY_FACTOR.max_bp`, `yield_ceiling ==
      YIELD_CEILING.max_bp` — genesis on the inflationary extreme, zero agent
      headroom), the linear authority schedule, pre-mine vs `max_supply`, and the 040
      topology. **The `decay_factor_bp` SENTINEL is enforced: a numeric value is a
      hard FAIL** (a baked 9_900 overrides every deployment whose
      `KatConfig::reward_decay_bp` differs — the bug riir-dapps caught live at decay
      10 000). **BOUNDARY: no dep edge added** — the deployer's dep row is empty by
      contract, so the legal move is a copy PLUS a gate (the `dapps_contract_pins`
      discipline): committed vocabulary (`SIBLING_PINS`, asserted in CI with no
      sibling) + derived population (`scripts/tokenomics_pin_gate.sh`,
      `RIIR_DEPLOYER_REQUIRE_DAPPS_PINS=1` makes absence a failure). 25/25 pins
      verified against riir-dapps **HEAD** (not its worktree — that repo was dirty
      with this issue's own T1.7/T1.8 session). 13 tests.
- [-] T2.2 ceremony: keys → chainspec authorship → registry genesis → SSH provision
      (caveat #6 — dry-run-by-default; wrong root key = locked treasury, double mint =
      supply bug; the ceremony is the ONLY sanctioned author of testnet/mainnet.toml —
      closes the 040 D4 gap). **The dry-run HARNESS is built and shipped** (T2.1
      above): validated params + the printed four-step plan (owner-held keys →
      chainspec rows → registry genesis → SSH provision, with the node count named).
      `genesis --execute` REFUSES, naming this task. **Unblock: OWNER EXECUTES** —
      this authors real testnet key material and writes to nodes as root (caveat #8);
      not an agent act.
- [-] T2.3 testnet ceremony executed once; `testnet.toml` lands.
      **Unblock: T2.2, then OWNER EXECUTES.**

## Phase 3 — deploy UX + health

- [x] T3.1 TTY selector + config/version acks (flags-only non-TTY; mainnet hardening —
      caveat #5: a prompt is not a security boundary) — **LANDED 2026-09-02**
      (riir-deployer `96009cc`, Plan 003): `src/ux.rs` — the destination selector
      (devnet first, **mainnet last, never the default, never auto-selected even at
      N=1**) + the two §5 read-back acks (resolved config; artifact version + BLAKE3).
      Non-TTY is **flags only**: an omitted `--dest` with no terminal is a hard error
      naming the flag, never a prompt, and the acks print as an audit record (G3-S —
      the CLI without a TTY is unchanged). **Caveat #5 honored structurally, not
      rhetorically:** the mainnet gate is `RIIR_DEPLOY_ALLOW_MAINNET` == the
      destination name EXACTLY (a truthy `1`/`true`/`yes` is REFUSED — a truthy env
      var is a standing invitation) + `RIIR_DEPLOY_ADMIN_KEY` (the existing `roll.rs`
      admin substrate, reused) + `--reason`, evaluated BEFORE any prompt and pinned
      **terminal-independent** by a source-level invariant test (the gate's body may
      not reference `is_interactive`/`is_terminal`/`read_line`/`select`/`ack`/`stdin`),
      canaried against a hollow scan. 12 tests.
- [x] T3.2 `node-sync` probe + 3-node verify gate (reachability + height + sync%) —
      **LANDED 2026-09-02** (riir-deployer `96009cc`, Plan 003):
      `VerifySpec::NodeSync`, the FLEET gate beside `chain-height`. Four independent
      axes per node with named failures — `UNREACHABLE` / `PUBKEY-PIN` (the published
      pin vs `/node-info`: a re-provisioned host on the old URL is the right address
      and the wrong node) / `FROZEN` / `BEHIND`, where `sync_bp = height /
      max(peer_heights)` comes from the probed set's own `/node-info` (no new network
      service, §6). `expect_nodes` IS the gate — a probe that silently accepts 2 is
      not a 3-node gate; `0` = all listed, so devnet's 1 node passes trivially.
- [x] T3.3 G-DEPLOY on a local 3-node cluster — **RAN 2026-09-02** (riir-deployer
      `96009cc`, record
      [`.benchmarks/003`](../../riir-deployer/.benchmarks/003_genesis_ux_node_sync_goat.md)):
      `build` → `deploy --dest local3` spawned **three real `mock-chain` processes** on
      three loopback ports → `verify` ran the probe through the actual CLI binary.
      Measured `PASS node-sync — 3/3 node(s) reachable, at height (fleet max 360),
      synced ≥ 9900bp`; all three failure axes fire (killed → UNREACHABLE naming the
      node, frozen → FROZEN,BEHIND, stale pin → PUBKEY-PIN), plus the count gate and
      the selector-degrades clause. Re-run 3×. **One G-DEPLOY clause is NOT run and
      NOT claimed:** *"a chainspec that `chaind --chain-spec` accepts"* — blocked twice
      and neither is a harness gap: (1) no authored chainspec exists (authoring one IS
      T2.2), and (2) `riir-chaind` cannot enter the deployer's build graph
      (BOUNDARY.md's zero-sibling-dep hard rule). The owner runs that arm at ceremony
      time from the ceremony runbook. Tracked as Plan 003 T3.3d.

## Phase 4 — wet-run + foo onboarding

- [x] T4.1 `wetrun` harness (steps 1–6) + G-WET. **Caveat #2 is load-bearing: the
      refund is FAUCET-funded, never mint/burn-pool — the G-WET assertion (mint/burn
      counters bit-unchanged) is the money-printer guard.** — **LANDED 2026-09-02**
      (riir-clippy `aecef31` + `1016754`,
      [Plan 066](../.plans/066_kat011_t18_faucet_cli_and_phase4_wetrun.md)):
      `src/kat_wetrun.rs` — the six §7 steps as a **pure step machine** over a
      transport trait, plus `HttpWetrunTransport`, and
      `tests/kat_wetrun_gwet.rs` (11/11) driving it against a scripted service
      that models the ledger and **records every call**.
      **How caveat #2 is enforced:** `SupplyCounters` (the wire view of
      riir-dapps' `Meta`, read from `GET /telemetry?epoch=`) carries a
      **total-function `to_bytes`**, so G-WET compares **bytes over the WHOLE
      struct** — "bit-unchanged" is literal, and a counter added later is covered
      without an edit anywhere (the riir-dapps G-CRANK `Meta` discipline).
      `size_of` is crossed against `ENCODED_LEN`, so a field added without
      extending the encoding **reds the gate** instead of escaping it; one
      divergence probe per field proves each field actually moves the bytes; the
      epoch rides IN the encoding so two epochs cannot compare as unchanged; and
      a missing counter parses as an ERROR, never a zero (a zeroed mint counter
      is exactly the reading that makes a printer look frozen). A full run that
      cannot read both snapshots is a FAIL, not a pass.
      **Step 3 is a REAL cycle**: the actual `clippy_lints` proposers over a
      seeded dirty fixture through the actual `KatMeter`/`KatCharge` (a
      fabricated spend would make the harness measure itself), and a zero-fix
      cycle FAILS rather than recording a vacuous zero. **Step 4** enforces
      redact ≥ floor in the HARNESS, not the composer (a composer that both
      builds a payload and vouches for its redaction is one structure agreeing
      with itself), refuses a 200 without a `recorded` count, and reports a
      missing `latent_retrieval` build as a FAILURE rather than a skip.
      Steps 1/6 reuse **T0.2b's landed reads verbatim** (three-state
      `WelcomeGate`); the per-node reachability/height/sync% table stays
      riir-deployer's `node-sync` (T3.2) and is **pointed at, not forked**.
      **Every gate canaried** — a moved mint micro flips the verdict while every
      step still passes; a money leg inside the dry-run branch reds the call-log
      assertion; reordering `classify_faucet_leg` reds the Issue-010 ordering
      test. That second canary also exposed that `Step::in_dry_run` was
      DECORATIVE (widening it changed nothing), so `finish` now refuses a
      dry-run report containing a money leg.
      **BLOCKER, NOT PAPERED OVER — riir-dapps [Issue 010](../../riir-dapps/.issues/010_treasury_grant_floor_equals_max_supply_unreachable_path.md):**
      `TREASURY_GRANT_FLOOR_DEFAULT == MAX_SUPPLY_PLACEHOLDER` (both 10¹²), so
      the treasury-ONLY crank skips every account every day and **steps 2/5 (the
      faucet legs) cannot succeed against a genesis-default deployment**.
      `classify_faucet_leg` checks `skipped_treasury` **before every other skip
      reason** (it marks a misconfigured DEPLOYMENT; the others mark a correctly
      refused ACCOUNT — conflating them is how "the faucet does nothing, forever"
      reads as "not eligible today"), and `issue_010_diagnostic` FAILS with the
      issue number, both constants, `STRUCTURALLY UNREACHABLE`, `NOT retryable`,
      the owner-act unblock, and what makes the legs reachable meanwhile. **No
      retry, no wait-it-out, never a pass** — pinned by a test asserting the
      write log is exactly one faucet open plus one crank attempt, the local legs
      never ran, and no supply counter moved. The happy-path tests state in their
      module docs that they script an **explicitly lowered**
      `treasury_grant_floor`. **No riir-dapps constant was touched** — the
      replacement value is a supply decision (Issue 010 T2, an owner act).
- [x] T4.2 mainnet refusal (unit + live, NO override by design) + `--dry-run`
      read-only proof — **LANDED 2026-09-02** (riir-clippy `aecef31` +
      `1016754`). **There is no override at all.** Per caveat #2 a
      `RIIR_ALLOW_MAINNET_WETRUN` would be "an attack on our own bookkeeping",
      so `src/kat_wetrun.rs` reads the environment **nowhere** — pinned by a
      source-level audit that strips line comments (its first cut failed on this
      module's own docs naming the hatch that must not exist) and canaries its
      predicate in both directions. The crank bearer is read in `heal.rs`, never
      there. **Live-verified**: with `RIIR_ALLOW_MAINNET_WETRUN`,
      `RIIR_DEPLOY_ALLOW_MAINNET` and `RIIR_HEAL_ALLOW_MAINNET` all exported at
      once, `cargo heal wetrun` still exits 2 with `REFUSED`. Classification is
      **fail-closed**: an unplaceable host is refused exactly like mainnet —
      including the production default `kat.heal.gist.rs`, which carries no
      network word (the safe answer, not an oversight). A refused run touches
      **nothing**: the call log must be EMPTY, not merely write-free.
      **`--dry-run` is proven read-only by measurement**, not prose: exactly six
      GETs, zero POSTs, no telemetry read (nothing moved, so there is nothing to
      snapshot), the scripted ledger bit-identical afterwards, on
      devnet/testnet/mainnet/unclassifiable. The read/write split lives at the
      TRANSPORT TRAIT, so a write added later shows up in the log automatically.
- [x] T4.3 install.sh copy alignment (gist-rs/cargo-heal — the 003 surface, NOT
      riir-clippy releases) + foo end-to-end doc — **LANDED 2026-09-02**
      (riir-clippy `9b0e777`). The surface was verified before editing and the
      corrected owner ask holds: no copy anywhere pointed at riir-clippy
      releases. **The drift was elsewhere and real** — Proposal 005's TL;DR
      served both installers from the vanity host `cargo-heal.gist.rs`, **which
      does not exist**, while Proposal 011 §8.1 used the
      `raw.githubusercontent.com` form: two documents, two one-liners, one
      unrunnable, and nothing to catch it (install copy is a string literal in
      prose with no structure to disagree with). Proposal 003 now DECLARES the
      canonical pair and records `cargo-heal.gist.rs` as a **deferred alias**;
      `tests/install_copy_parity.rs` (3/3) fails on any other install URL in any
      tracked `*.md` and on any pointer at the private repo's releases, with an
      instrument selftest that asserts it recognises the DRIFTED shape (else it
      could not fail) and ignores the three non-URL mentions in the tree. The
      audit fired twice while being written. Plus
      `.docs/01_orientation/foo_end_to_end.md` — every step marked runnable-now
      or blocked-and-why.
      **Residue (owner act, not a copy gap):** `gh repo view gist-rs/cargo-heal`
      answered *"Could not resolve to a Repository"* (2026-09-02) — the public
      dist repo has **never been created** (Proposal 003 T1.1), so neither
      one-liner is runnable yet. Publishing binaries built from private source
      is an owner/security decision.
- [-] T4.4 testnet wet-run executed; mesh-ready verdict recorded.
      **Unblock: T2.2/T2.3 (the OWNER-EXECUTED genesis ceremony) — a testnet
      wet-run needs real testnet key material authored and nodes written to as
      root (caveat #8), which is not an agent act.** The harness it will run is
      shipped (T4.1/T4.2 above) and its money legs additionally need riir-dapps
      Issue 010 T2 (the owner's supply decision) plus a deployment
      `RIIR_HEAL_KAT_CRANK_TOKEN` — `POST /admin/crank/daily` is bearer-gated
      fail-closed and is what settles a faucet row.

## Phase 5 — live USDC pay-and-register (real users; counsel + demand gated)

- [-] T5.1 live Solana-Pay settle path + G-REG matrix run live (the ten §12 cases;
      fixtures first; the `#[ignore]`d live arm un-ignored).
- [-] T5.2 ops reconcile runbook for expired-intent stuck funds (until outbound
      refunds exist — Issue 059 T2.2 territory).
- [-] T5.3 testnet-then-mainnet launch decision (the counsel envelope shared with the
      KAT rail — Issue 059 T3.1).

## Deferred — gated (from the proposal; tracked, not scheduled)

- [-] Chain-NFT invoice mint (after the Warm-tier invoice proves out; opt-in,
      Glacial cadence — a per-payment NFT would fail the rate test).
- [-] MemoShard v4 signing-message adoption (riir-chain 008 decision).
- [-] LIVE Stripe + Solana-Pay rails (mock-tested; only the launch decision waits).
- [-] KAT-rail launch beyond operator-owned accounts (the counsel gate — Issue 059).
- [-] `riir-wallet-wasm` invoice display (browser wallet; CLI view first).
- [-] Discord faucet bot (link-first made it pure automation; reopen on volume,
      gated on a per-account daily cap + identity proof no weaker than GitHub-SSH).
- [-] Baz fixer tier — its OWN plan, gated per 011 §9: (a) a fixer arm beats `decline`
      on the reachable set, (b) leak-posture sign-off, (c) reward economics with
      G5-class evidence (blocked by 060's advisory-only verdict). Opening it before
      clippy-tier onboarding lands would put the least-proven earning lane in front of
      the newest users.

## Summary

The proposal's fourteen owner asks are all verdicted SHIP (two corrected: 8.1 →
gist-rs/cargo-heal per 003; 12 → reference-key binding, memo as audit copy) or
explicitly deferred. Phase 0 ledger substrate LANDED 2026-09-02 (T0.1 + T0.5 + the
ledger/payreq halves of T0.4 — riir-dapps `6df7097` / Plan 015) alongside the
repo-native T0.2 CLI slice, and **T0.3 (the payer-side SDK half) landed the same
day** (`fe865ba1` + the dual golden pin `46e6660`). **Phase 1's payreq spine
landed the same day**: T1.1 (riir-dapps `f43c014` / Plan 062 — the signed
`/payreq` routes + the row-backed limiter) and T1.3 (riir-clippy `9a5086a` —
`cargo heal pay --request`, the payee create + link-first + poll-on-credit loop,
golden-pinned to the service byte-exactly, dapps-side golden `0a4d37a`).
**Phase 2's T2.1 and ALL of Phase 3 landed the same day** (riir-deployer
`96009cc` / [Plan 003](../../riir-deployer/.plans/003_genesis_params_deploy_ux_node_sync.md)):
the genesis params yaml validated against a committed pin of the ratified
008/010 constants (drift FAILS, nothing clamped; the `decay_factor_bp` sentinel
enforced; `--execute` refused), the TTY selector + the env-gated mainnet
boundary (caveat #5 honored structurally — the gate is pinned
terminal-independent), the `node-sync` fleet gate, and **G-DEPLOY RUN on a real
local 3-node cluster** with all three failure axes firing. Remaining Phase 0:
T0.2b's service half (unblocked — `/balance` already serves).
**Phase 4's engineering half landed the same day** (riir-clippy `d72bbb6` +
`aecef31` + `1016754` + `9b0e777`, dapps-side faucet golden `1736010`,
[Plan 066](../.plans/066_kat011_t18_faucet_cli_and_phase4_wetrun.md)): T1.8's
faucet preset (`cargo heal faucet --request`, amount and TTL deliberately
absent from both the wire and the flag set), T4.1's six-step wet-run harness
with **G-WET asserting every supply counter BIT-unchanged over the whole
struct** via a total-function encoding, T4.2's mainnet refusal with **no
override anywhere** (the module reads no environment; live-verified against
three plausible hatch variables set at once) plus a `--dry-run` proven
read-only by a **recorded call log**, and T4.3's install-copy pin (which found
Proposal 005 advertising a vanity host that does not exist).
Two blockers are recorded rather than absorbed: `gist-rs/cargo-heal` has never
been created (Proposal 003 T1.1 — publishing binaries of private source is an
owner act), and riir-dapps **Issue 010** makes the wet-run's faucet legs
unreachable at genesis defaults — the harness names it and FAILS, and the
happy-path tests say out loud that they script an explicitly lowered floor.
Next executable: T1.2 (the web wallet — every signed route it needs is landed),
T1.5 (the money/auth adversarial suite), T1.6 (the mock rails), T1.7 (the daily
grant crank's clippy-side follow-ups; **now that T1.8 has landed,
riir-deployer's `free_tier_daily_micro`/`faucet_daily_cap_micro` move from
structurally-bounded to PINNED**, per its Plan 003's named follow-up). Phase
2's T2.2/T2.3 and Phase 4's T4.4 are OWNER acts and every harness they need is
shipped — bar's dry-run genesis is what puts the mint in the wallet every foo
gets paid from, and the wet-run is what proves the loop once it does.

## Wait-state verification (2026-09-02)

- [Issue 059](059_proposal_007_remainder_ops_signer_and_counsel_gate.md):
  re-verified — both open items are ops/counsel-gated (RPC provider + custody-key
  posture; counsel engagement). No engineering unblock exists; the file stays as the
  wait-state tracker.
- [Issue 060](060_proposal_008_remainder_new_plan_and_gated_levers.md):
  re-verified — terminal-as-specified (G5 FAIL-wash stands); the only path is the
  owner-commissioned v2-candidates plan, which is an owner act, not an agent one.
  The file stays as the wait-state tracker.
