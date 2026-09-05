# foo, end to end — install → login → earn → get paid

> **Scope:** Proposal 011 §8's *foo* path, as it actually ships today
> (Issue 061 T4.3, Plan 066). Every command below is either **runnable now**
> or carries the exact reason it is not. That distinction is the whole point
> of this page: an onboarding doc that reads uniformly runnable, when a third
> of it is blocked, is worse than no doc.
>
> The *bar* path (genesis ceremony → deploy → verify) is **riir-deployer's**
> — see its `.plans/003`. bar runs once, by the operator, and is what makes
> foo trivial.

## Status at a glance

| Step | Command | State |
|---|---|---|
| 8.1 install | `curl … install.sh \| sh` | **BLOCKED** — the dist repo does not exist yet (Proposal 003 T1.1) |
| 8.2 login | `cargo heal login` | ships |
| 8.3 welcome | `cargo heal account` | ships (service-plane gate, T0.2b) |
| 8.5 history | `cargo heal account --history` | ships, with a stated limit |
| — heal | `cargo heal --fix --write --verify <paths>` | ships |
| — sync | `cargo heal sync` | ships |
| T1.3 get paid | `cargo heal pay --request` | ships |
| T1.8 faucet | `cargo heal faucet --request` | ships (CLI); the credit needs a crank pass |
| T4.1 wet-run | `cargo heal wetrun [--dry-run]` | ships; the money legs are blocked by riir-dapps Issue 010 |

## 1. Install (§8.1) — the one blocked step

```sh
curl -fsSL https://raw.githubusercontent.com/gist-rs/cargo-heal/main/install.sh | sh
# or: brew tap gist-rs/tap && brew install cargo-heal
```

**This does not work yet, and the reason is not a copy problem.**
`gh repo view gist-rs/cargo-heal` answered *"Could not resolve to a
Repository"* on 2026-09-02: the public dist repo has never been created
(Proposal 003 T1.1). Publishing binaries built from private source is an
owner act, so the repo creation is not an agent move.

The URL above IS canonical — [Proposal 003](../../.proposals/003_binary_only_distribution.md)
§The canonical install one-liners owns it, and
`tests/install_copy_parity.rs` fails on any document that invents a
different one. It used to: Proposal 005 served both installers from
`cargo-heal.gist.rs`, a vanity host that does not exist, while Proposal 011
§8.1 used the `raw.githubusercontent.com` form. Two documents, two
one-liners, one unrunnable.

**Never** point this at `gist-rs/riir-clippy`'s own releases. That repo is
PRIVATE, so its assets need authentication and cannot be curl'd — and
flipping it public to make them work would leak the entire source. Pinned by
the same test.

Until then: `cargo install --path . --features latent_retrieval`.

## 2. `cargo heal login` (§8.2) — zero configuration

```sh
cargo heal login              # or: cargo heal login --key ~/.ssh/id_ed25519
```

Adopts the machine's default SSH key unless `RIIR_HEAL_KAT_KEY` overrides.
Idempotent: an existing key is loaded, never regenerated. The account id is
`BLAKE3(pubkey)`, which is the same derivation the sync plane's
`submitter_for` uses — golden-pinned across repos, because a drift there
would mean querying an account no route can settle from.

The free-tier grant is recorded as a **local projection** at this point
(accounting only). The authoritative row appears after the first `sync`.

## 3. `cargo heal account` (§8.3) — and why it can say "I don't know"

```sh
cargo heal account
cargo heal account --history
```

The welcome gate is **three-state**, evaluated from three real service reads
(`GET /health`, `GET /mining/health`, `GET /balance?acct=`):

- **Open** — every plane answered, the ledger is settled, the grant row
  exists. Prints the welcome.
- **Withheld** — the network was read and it is not ready.
- **Unknown** — the network could not be read. **No claim is made either
  way.** Folding this into Open would print a health claim the CLI cannot
  see; folding it into Withheld would tell an offline user their account is
  out of sync.

Offline costs ONE timeout, not three (the probe fail-fasts on an unreachable
host).

`--history` reports the grant STATE plus the local per-repo meter, and says
so. `/balance` serves a *position*, not an event log; a per-event history
needs a dapps route that does not exist. Fabricating the list would be the
exact violation T0.2 avoided.

## 4. Heal something, and watch it cost KAT

```sh
cargo heal --fix --write --verify src/
```

The meter records word tokens over the healed spans; the charge ledger
settles them at 1 KAT per code word token. `--verify` compiles baseline →
applies → re-checks → auto-reverts breaking edits. Feature-gated code needs
`--verify-args "--features <set>"` — a default-features check compiles gated
files to nothing, and a green check then proves nothing about them.

## 5. `cargo heal sync` — the data flywheel

Pushes redacted trajectory rows (signed, v2 envelope) and pulls the corpus
lease. This is also what makes the service ledger see the account, which is
what flips the welcome gate to Open.

## 6. `cargo heal pay --request` (T1.3) — getting paid

```sh
cargo heal pay --request                       # ANY amount — the payer chooses
cargo heal pay --request --amount 2.5 --qr     # a pinned amount, with an ASCII QR
cargo heal pay --request --wait 0              # don't poll; `account` shows the credit
```

**Link-first** (ask 13): the shareable URL is printed first and is the
primitive; `--qr` is a rendering of it, never a separate channel. The create
is signed with the account key over a message the service verifies with its
own composer — golden-pinned on both repos to an independently computed
digest, so a client-side mirror cannot drift.

Then it polls on credit until the wait budget ends (default 600 s). The
budget is a monotone `Instant` elapsed check; a wall-clock snapshot never
advances and spins forever, which a mock test caught.

When the payer settles, the invoice renders — **gated on a bit-exact QR
recheck**. A mismatch REFUSES to render the body (no amount, no payer, no
token row): a tamper indicator is not a footnote. The unset service sentinel
reads as `Unavailable` and labels itself rather than passing silently.

No `$` ever appears beside a KAT figure (caveat #15, test-enforced).

## 7. `cargo heal faucet --request` (T1.8) — and what it deliberately won't do

```sh
cargo heal faucet --request
cargo heal faucet --request --qr
```

**There is no `--amount`.** The wire carries the pubkey and the signature and
nothing else; the amount is the deployment's `faucet_daily_cap_kat`, pinned
server-side. Accepting an amount flag would advertise a choice that does not
exist — and the amount being absent from the signed message is exactly what
stops a captured signature being replayed at a different value. (Compare
`settle`, where the amount is INSIDE the signature *because* the payer
chooses it.)

**There is no `--wait`.** A faucet row is settled by the DAILY crank, so a
poll would be either a lie (seconds against a once-a-day pass) or a day-long
hang. The render says where the credit shows up instead.

Four refusals reach you in the ledger's own words — faucet disabled at this
deployment, service accounts cannot request, daily cap already used, one
request already open. None of them is reworded into a timeout.

## 8. `cargo heal wetrun` (T4.1/T4.2) — the full-loop proof

```sh
cargo heal wetrun --dry-run    # steps 1+6, read-only, allowed on EVERY network
cargo heal wetrun              # all six steps; devnet/testnet only
```

Six steps: preflight → faucet top-up → one scored scan→fix cycle (metered)
→ signed sync push → faucet refund → mesh readiness.

**Mainnet is refused, and there is no override.** Per Proposal 011 caveat #2
an env hatch would be "an attack on our own bookkeeping", so none exists;
`src/kat_wetrun.rs` reads the environment nowhere at all, pinned by a
source-level audit. Classification is fail-closed: a host that cannot be
placed on a network is refused exactly like mainnet — including the
production default, which carries no network word in its name. That is the
safe answer, not an oversight.

`--dry-run` issues exactly six GETs and zero writes, proven by a recorded
call log rather than by reading the code.

The full run also asserts **G-WET**: every supply counter bit-unchanged
across the run, compared as bytes over the whole counter struct. The refund
is a treasury GRANT, never a mint or a burn-pool draw. If that ever moved,
`wetrun` would be a money printer with a test wrapper.

### The money legs do not work against a default deployment

Steps 2 and 5 need a crank pass to settle the faucet row, and
**riir-dapps Issue 010** makes that impossible at genesis defaults:
`TREASURY_GRANT_FLOOR_DEFAULT == MAX_SUPPLY_PLACEHOLDER` (both 10¹²), so
satisfying `surplus >= treasury_grant_floor` needs a treasury larger than
the entire max supply. The crank is deliberately treasury-ONLY (a mint
fallback on an unattended recurring path is what caveat #2 forbids), so it
skips every account, every day.

The harness does not hide this. It reads the crank report, sees
`skipped_treasury`, and FAILS with a diagnostic naming the issue, both
constants, "NOT retryable", and the fact that the fix is an owner supply
decision. It never retries, never waits it out, and never reports a pass.

You also need `RIIR_HEAL_KAT_CRANK_TOKEN`: `POST /admin/crank/daily` is
bearer-gated fail-closed service-side. Without it the CLI says so up front
rather than letting a 401 look like a service fault.

## Where the fleet view lives

Per-node reachability / height / sync% is **not** in this repo. It is
`riir-deploy verify --network <net>` (`node-sync`, Proposal 011 §6), and
`wetrun` points at it rather than forking a third implementation.

## Related

- [`../../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md`](../../.proposals/011_kat_pay_request_qr_genesis_and_onboarding.md) — §8 onboarding, the G-WET gate
- [`../../.proposals/003_binary_only_distribution.md`](../../.proposals/003_binary_only_distribution.md) — the dist surface + the canonical install copy
- Issue 061 — the rollout tracker, RESOLVED + removed 2026-09-05 per the
  noise-reduction rule; the rollout record lives in the repo-root `AGENTS.md`
  ("Issues 061 + 068" entry) and git history
