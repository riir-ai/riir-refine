# Plan 064 — kat-011 T0.2b (the service half of `account`) + T1.4 (invoice display + bit-exact QR recheck)

Status: **COMPLETE (2026-09-02)** — every task landed and validated; see §Landed.
Branch: `develop` (per global rule — no feature branches)
Issue: [Issue 061](../.issues/061_proposal_011_product_loop_rollout.md) T0.2b + T1.4
Proposal: [`011`](../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md)
(§8.3/§8.5 for the welcome gate + history; §1 "Bit-exact recheck before
display" for T1.4; **caveat #3** — the re-encode pin rides the `qrcode` crate
version + render params; **caveat #15** — no `$` beside a KAT figure, accruals
labelled service credit)
Reference pattern: [Plan 062](062_kat011_t111_payreq_routes.md) /
`src/kat_payreq_client.rs` — the landed service-plane client convention
(one `*Transport` trait per channel + one ureq impl + a scripted mock + a
monotone `Instant` budget). **Not a second HTTP convention** — the same one.

## Substrate-first (run BEFORE writing any type)

Greps over `*.rs` in this repo, 3+ name variants per concept:

| concept | variants grepped | found |
|---|---|---|
| node/service health | `node_info`, `NodeInfo`, `healthy`, `NodeHealth`, `health` | **nothing reusable** — only the T0.2 comment at `kat_account.rs:584` + `heal.rs:253` that PROMISE this task, and unrelated docker/kernel rule text |
| sync state | `sync_pct`, `sync_percent`, `all_synced`, `SyncStatus`, `node_sync` | **nothing** — `SyncWatermark`/`PendingEstimate` (`kat_pending_estimator.rs`) are the LOCAL row-coverage vocabulary, a different axis (and behind `latent_retrieval`) |
| balance | `balance` | `kat_meter::ChargeState::balance_projection` — the LOCAL per-repo projection, explicitly not the ledger |
| HTTP client | `Transport`, `HttpResponse`, `ureq::Agent` | `kat_lease_client::HttpResponse` + the `Lease/Vote/Stake/PayreqTransport` family — **consumed, not re-invented** |
| QR | `render_qr_ascii`, `qrcode`, `qr_png` | `kat_lease_client::render_qr_ascii` (params implicit) — **extended in place** |
| KAT decimal render | `render_kat`, `parse_kat_amount` | `kat_payreq_client` — **consumed** (no second renderer) |

Verdict: the account SERVICE plane has no substrate here; the transport
convention, the KAT renderer and the QR renderer all do and are consumed.
New module `src/kat_account_client.rs` = the missing channel client, same
shape as the four that ship.

## Item A — T0.2b: the service half of `account`

- [x] A1 **`/balance` consumption + the account-id derivation pin.** `/balance`
      takes **32 hex chars** (the ledger's 16-byte `AccountId`), while
      `kat_account::account_id_for` is the **64-hex full BLAKE3**. The service
      id is the first 32 chars — the same bytes as
      `kat_payreq_client::account_for_pubkey` (`blake3(pk)[..16]`). Ship
      `service_account_hex()` and PIN the two derivations equal, so this cannot
      drift into "requests for an account no route can settle from".
- [x] A2 **The welcome gate from REAL service health.** Replace the T0.2
      local heuristic (`spent == 0`). Inputs are three real service reads:
      `GET /health` (ledger plane), `GET /mining/health`, `GET /balance?acct=`.
      **Three-state verdict, never two** — `Open` / `Withheld` (read the
      network, it is NOT all-synced) / `Unknown` (could not read it at all).
      A gate that folded Unknown into Withheld would be honest but mute; one
      that folded Unknown into Open would print a health claim the CLI cannot
      see, which is the exact thing T0.2 refused to do.
- [x] A3 **Server-side history replaces the local-only view.** `/balance`
      serves `{account_id, balance_micro, granted}` and **no event log** — so
      the history renders the authoritative balance + grant flag and says
      plainly that a per-event log is not what this route carries. Local burn
      stays local (per-repo meter). Unit trap handled: local figures are whole
      KAT, `balance_micro` is micro-units.
- [x] A4 **CLI wiring** (`heal.rs::run_kat_account`) + fail-fast probing so an
      offline `account` costs one timeout, not three.
- [x] A5 **Copy rule (caveat #15)** on every new render surface: no `$`, KAT
      figures labelled service credit — pinned by test over ALL verdict states.

## Item B — T1.4: invoice display + bit-exact recheck

- [x] B1 **Make the render params explicit + pinned.** `render_qr_ascii` used
      `QrCode::new` (implicit `EcLevel::M`) + `quiet_zone(true)`. Lift both
      into a typed `QR_RENDER_PARAMS` constant and render through it —
      behaviour-preserving, pinned by a test that the refactor is
      byte-identical to the old call shape.
- [x] B2 **The QR asset commitment.** `qr_commitment(url)` = BLAKE3 over a
      canonical, **param- and version-stamped** serialization of the QR
      **module matrix** (renderer-independent: an ASCII render and a PNG are
      both functions of the matrix, and this repo cannot render PNG —
      `qrcode` ships here with `default-features = false`, no `image`).
- [x] B3 **Caveat #3, made non-vacuous — the pin must MOVE on both axes.**
      (i) *crate version*: `QRCODE_PINNED_VERSION` rides IN the commitment
      preimage, and a test reads the **resolved** version out of `Cargo.lock`
      (not the `Cargo.toml` requirement) so a `0.14.1 → 0.14.2` patch bump
      inside the `"0.14"` range FAILS instead of silently re-rendering every
      stored asset; the requirement string is pinned separately so widening it
      to `"0.15"` fails first. (ii) *render params*: per-axis divergence
      probes assert the commitment differs for each of `EcLevel::{L,Q,H}` and
      for `quiet_zone: false`, and that the ASCII render digest moves on the
      quiet zone. A version-axis probe asserts a stamped `"0.15.0"` diverges.
      Both instruments must FIRE, not merely exist.
- [x] B4 **Invoice display + the 3-state recheck.** `parse_invoice` →
      `InvoiceView`; `recheck_invoice_qr` → `Match` / `Mismatch` (⇒ **refuse
      to display**, tamper indicator, per §1) / `Unavailable` (⇒ display
      LABELLED — caveat #3's hash-match-only fallback for pre-bump/unfilled
      assets). The service leaves `qr_png_blake3` all-zero today
      (dapps `invoice_set_qr_hash` is the unfilled T1.4 render step), so
      all-zero must read as **Unavailable**, never as a false tamper.
- [x] B5 **Copy rule (caveat #15)** over all three recheck states.

## Gates

- G1 correctness: new unit tests in both modules; the cross-derivation pin
  (A1) and the divergence probes (B3) computed independently of the code
  under test.
- G2 feature states: `kat_service` (parse/render/gate halves) and
  `kat_service,kat_transport` (the network halves). A default-features run
  compiles both modules EMPTY — it proves nothing and is not accepted as
  validation.
- G3 no-regression: default build untouched (zero KAT code compiled); the
  pre-existing lib suite count-identical at equal feature sets.
- G4 zero new dependencies.
- G5 clippy clean (`--all-targets`) in every state touched.

## Item C — the defect the T1.4 smoke found (unplanned, fixed)

- [x] C1 **`pay --request` was advertised and unparsed.** `fn usage()` has
      carried `cargo heal pay --request …` since T1.3, while the arg parser had
      no `"--request"` arm — so the DOCUMENTED invocation printed
      `unknown flag "--request"` and the whole usage page. Nothing could have
      caught it: the usage text is a string literal and the parser is a
      `match`, two structures with no obligation to agree, and the gap is
      silent in every compile, clippy state and unit test. Fixed (`pay_request`
      field + arm + the misplacement guard), and pinned by a new gate.
- [x] C2 **`tests/heal_usage_flag_parity.rs`** — every long flag in the usage
      text must have a parser arm. Source-text audit (the parser `exit`s via
      `usage()` on an unknown flag, so it is not drivable from a unit test
      without restructuring `main`), with one justified exemption
      (`--features` in the usage text is cargo's flag, not heal's). It carries
      its own `the_audit_can_see_what_it_audits` instrument pin — population
      floors plus three shape pins — because a regex regression would
      otherwise print parity over zero flags. **Canaried**: removing the
      `--request` arm again fails it with the flag named.

## Landed (2026-09-02)

### Files

- `src/kat_account_client.rs` (**new**, T0.2b) — `service_account_hex`,
  `parse_balance` / `parse_ledger_health` / `parse_mining_health`,
  `WelcomeGate` + `evaluate_welcome` + `welcome_gate_lines`,
  `service_history_lines`, `AccountTransport` + `HttpAccountTransport` +
  `account_service_view`.
- `src/kat_lease_client.rs` (T1.4 B1–B3) — `QrRenderParams`,
  `QR_RENDER_PARAMS`, `QRCODE_PINNED_VERSION`, `QRCODE_PINNED_REQUIREMENT`,
  `QR_COMMITMENT_DOMAIN`, `render_qr_ascii_with`, `qr_commitment` /
  `_with` / `_stamped`, + 6 pin tests.
- `src/kat_payreq_client.rs` (T1.4 B4–B5) — `InvoiceView`, `parse_invoice`,
  `QrRecheck`, `recheck_invoice_qr`, `invoice_line` (now pure),
  `invoice_display_lines`, `fetch_invoice`, `SettleOutcome`;
  `PayRequestOutcome::settled` is now `Option<SettleOutcome>`.
- `src/kat_account.rs`, `src/lib.rs`, `src/bin/heal.rs` — wiring + the doc
  comments that promised this task.
- `tests/heal_usage_flag_parity.rs` (**new**, C2).

### How the caveat-#3 pin was made non-vacuous (the part most likely to be faked)

Two mechanisms, and each one was **canaried by reintroducing the defect**:

| axis | structural half | measured half | canary result |
|---|---|---|---|
| `qrcode` crate version | `QRCODE_PINNED_VERSION` rides in the commitment preimage (length-prefixed), so the digest moves with it; `qr_commitment_stamped` exposes the axis and a probe asserts `"0.14.2"`, `"0.15.0"` and `""` all diverge from the shipped digest | `qrcode_resolved_version_is_the_pinned_one` reads the **resolved** version out of `Cargo.lock` (NOT the `Cargo.toml` requirement, which `"0.14"` satisfies for any `0.14.x`), pins the requirement string separately so widening it to `"0.15"` fails first, asserts the two agree, and asserts `default-features = false` (the default set drags `image` in). It `expect`s on a missing lock / missing package / missing manifest line — an auditor that passes on an empty read is worse than none | declaring `0.14.2` against the locked `0.14.1` → **3 tests FAIL** |
| render params (ECC, quiet zone) | both members of `QrRenderParams` ride in the preimage; per-axis probes assert the commitment diverges for each of `EcLevel::{L,Q,H}` and for `quiet_zone: false`, and that the ASCII render digest moves on both | `qr_render_params_are_the_pinned_ones` pins the constant AND `EcLevel::M as u8 == 1` (the byte that actually enters the preimage, so an upstream enum reorder is visible) | flipping `QR_RENDER_PARAMS.ec_level` to `H` → **5 tests FAIL** |

Neither half suffices alone, and the plan file says why: mechanism 1 without 2
lets a `cargo update` change every stored asset's commitment while every golden
still passes (the hand-written constant did not move, so neither did the
preimage); mechanism 2 without 1 leaves the commitment version-blind. The
commitment also gets an **independent preimage recompute** inside
`qr_commitment_golden` (built in the test, not via the function) so a
layout regression fails rather than being re-recorded, and
`render_qr_ascii_is_byte_identical_to_the_pre_t14_call_shape` proves the
param lift moved no stored asset.

`qr_commitment` commits the QR **module matrix**, not a rendered image: this
repo carries `qrcode` with `default-features = false`, so there is no PNG to
hash, and every rendering is a function of the matrix anyway.

### Validation

- `cargo clippy --all-targets`, **0 errors / 0 warnings** in four states:
  default, `kat_service`, `kat_transport`, `kat_transport,latent_retrieval`
  (re-run after `touch`ing every changed file — a warm clippy emits nothing,
  so a zero from an up-to-date build proves nothing).
- lib tests: default **1008/0** (count-identical with Plan 059's recorded
  1008 — G3, the default build compiles zero KAT code); `kat_service`
  **1128/0**; `kat_transport` **1142/0**; `kat_service,latent_retrieval`
  **1262/0/1**; `kat_transport,latent_retrieval` **1276/0/1**.
  Delta arithmetic checks out: +5 at `kat_service` (4 account-client + 1
  payreq parse), +10 more at `kat_transport` (2 + 3 + 6 new, −1 replaced),
  and transport-only tests went 4 → 14.
- new/changed suites: `kat_account_client` 6/6, `kat_payreq_client` 12/12
  (was 9), `kat_lease_client` 29/29 (was 23), `heal_usage_flag_parity` 3/3.
- standing gates re-run green at Plan 059's recorded counts:
  `kat_account_g9` 6/6, `kat_stake_client_g2s` 9/9,
  `kat_vote_client_g9gov` 5/5, `kat_meter_g9` 11/11.
- **live smoke** (isolated `RIIR_HEAL_KAT_ACCOUNT_DIR`; the real account dir
  untouched), against a scripted local mock and against a dead port:
  - unreachable service → `welcome held back — … no claim is made about sync
    or node health`, the reason names the refused connection, and the LOCAL
    history renders. Returned immediately: the fail-fast rule cost one read.
  - mock serving all three routes → `welcome to the KAT network …`,
    `all planes healthy, ledger settled at epoch 7 (4 record(s) awaiting
    proof)`, `42.500000 KAT service credit`, and `/balance?acct=` received
    the **32-hex** truncation (`06df8f0d35ee52ae9c13fe224954c32b`) of the
    64-hex local id.
  - `pay --request --amount 1.5` with an UNSET service QR slot → full invoice
    display + `qr asset: NOT bit-exact-verified` + the caveat-#3 fallback note.
  - the same with a bogus stored digest → `REFUSED — tamper indicator`, both
    digests printed, and **no amount, no payer, no token row**.
- no new dependencies (G4).

### Honest residues

- **`/balance` is a position, not an event log.** There is no server-side
  per-event earn/spend history route, so `service_history_lines` renders the
  authoritative balance + grant STATE and says so in its own copy rather than
  fabricating a transaction list. A real history needs a dapps-side route
  that does not exist; not filed as a blocker because the §8.5 ask
  ("earned/spent") is served honestly by balance + local burn.
- **The dapps side of T1.4 is not this repo's.** `qr_png_blake3` is
  `[0u8; 32]` on every invoice the service has ever written (dapps
  `invoice_set_qr_hash` is the unfilled render step), so the recheck is
  `Unavailable` in production today — the caveat-#3 fallback, correctly
  labelled. **Contract note for whoever fills it:** the slot must be filled
  with `kat_lease_client::qr_commitment` of the canonical request URL, or
  every invoice will display as a tamper. Recorded in Issue 061's T1.4 row.
- **The node-level `node-sync` probe is still T3.2.** This gate reads the
  SERVICE plane's health, which is what serves today; it does not claim
  per-node reachability/height/sync%, and the module docs say so.
- **Near-miss worth recording: a python rewrite LF-ified a CRLF file.**
  `src/kat_payreq_client.rs` is a pure-CRLF file (638/638 lines at HEAD~1; this
  repo has no `.gitattributes` and several other `src/**.rs` files are CRLF
  too, from the Windows box pin). Editing it with `open(p,'w').write(s)`
  silently rewrote every line ending, turning a 513-line change into a
  2,086-line diff that buried the real edit. Caught by reading `git show
  --stat` rather than trusting the commit, and fixed by converting the file
  back to CRLF before amending. Same class as the
  "never re-serialize a JSONL archive" rule: read/write bytes, or check the
  stat.
- **`cargo heal` observation** (usage-artifact intake): the healer's
  `items_after_statements` edit hoisted two `impl` blocks out of a test fn
  but left one `struct` behind and dropped the indentation of the following
  statement to column 0. It compiled (so `--verify` kept it) and was
  correct-but-ugly; reverted to a hand-written form that declares both
  structs up front. Worth a healer-side look: the fix should move the item's
  whole declaration group, and should not reflow the trailing statement.
