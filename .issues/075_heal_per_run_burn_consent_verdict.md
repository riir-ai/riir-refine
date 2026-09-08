# Issue 075 — verdict: per-run burn + consent prompt on plain `cargo heal` (NOT LGTM)

**Status:** RECORD — verdict filed, no code change (2026-09-08)

## The question

Owner, 2026-09-08:

> Should plain `cargo heal` burn KAT tokens and prompt the user to accept
> contributing unheal (telemetry) on each run?

## Verdict

**NOT LGTM.** No per-run interactive burn, no per-run consent prompt. Four
independent grounds; each is sufficient alone. The verdict removes nothing
that ships — the metered burn stays exactly as built — it rejects
re-presenting both the charge and the consent ask at every invocation.

### R1 — the burn already exists; a per-run prompt double-charges the UX, not the wallet

The per-unit burn is LIVE local accounting: `CHARGE_KAT_PER_MILLION_TOKENS
= 1_000_000` (1 KAT per code word token) at `src/kat_meter.rs:279`,
charged incrementally by `KatCharge::settle` (`src/kat_meter.rs:442-454`)
against the token meter's watermark, surfaced as the `kat-charge` summary
line (`src/kat_meter.rs:510-522`). It is deliberately accounting-only:
"the balance below is a LOCAL PROJECTION for the warn UX; the server
ledger reconciles at sync (T0.3+) and is the authority … Nothing here
mints, spends, or blocks a heal" (`src/kat_meter.rs:328-332`). Proposal
008's own amendment says the same in prose: "The logged-in burn already
exists … nothing new ships for it. What landing makes real is the
reconcile: burn becomes DEBT on the service ledger at sync (the
epoch-settle input)" (`.proposals/008_adaptive_tokenomics_riir_dao.md:268-272`).

So a per-run burn prompt would present a second, interactive copy of a
charge that is already metered, already warned (`KAT_WARN_REMAINING`,
`src/kat_meter.rs:287-290`, under the "warn-and-pay, never brick" rule),
and already settled at sync — while moving zero additional money. Pure
UX cost, no economic change.

### R2 — the consent contract pins `Unset` = "running `sync` IS the consent"

Consent is a machine-global, persisted three-tier record — `Unset`
(default, no file) / `OptedIn` (`--mine`) / `OptedOut` (`--unmine`) —
`src/kat_mining_consent.rs:8-20`, with `push_allowed()` refusing ONLY
`OptedOut` (`src/kat_mining_consent.rs:70-72`). The contract is explicit:
"`Unset` MUST behave exactly as the pre-consent build did: **running
`sync` is itself the consent** … Making the absent state a refusal would
silently break every existing user's sync, so only `OptedOut` changes
behaviour" (`src/kat_mining_consent.rs:16-20`), and the shipped CLI UX
contract repeats it: "Never make the absent state a refusal; that breaks
every existing user" (`AGENTS.md:178-179`, owner mandate 2026-09-08).

A per-run accept prompt is that refusal in disguise: scripted, CI, and
non-TTY invocations have nobody to answer, so the prompt hangs, fails, or
gets auto-accepted — and auto-accept is the same consent-without-asking
the prompt claims to add, now with a broken non-interactive path. It also
contradicts the shipped docs (`AGENTS.md:175-179`: bare `cargo heal` IS
the dry run; `--fix` IS the real fix — neither advertises an interactive
gate).

### R3 — the free tier exists precisely for users with nothing to burn

010 §M2 makes the free grant treasury-funded: debit the treasury while it
sits above the reserve floor, fall back to the mint path below it
(`.proposals/010_kat_yield_ceiling_and_treasury_funded_free_tier.md:96-117`;
floor is a genesis constant, deliberately not an agent lever, `:111`;
local projection constant `FREE_TIER_GRANT_KAT = 100_000_000`,
`src/kat_meter.rs:285`). The tier's point is that a user with no KAT
still gets the default-on domains. The amendment prices that tier in
data, not KAT: "**unheal required** — the redacted miss/decline/
trajectory batch ships with (or after) the run"
(`.proposals/008_adaptive_tokenomics_riir_dao.md:255`), because "the
egress is the price, not a donation" (`:259-267`). Demanding a burn at
each run inverts M2: it charges the exact users the tier was funded for.
And the consent ask is already honored where it belongs — logged-in users
MAY decline the unheal egress (`008:256`), via the machine-global record,
decided once, not re-asked every run.

### R4 — money moves at epoch settle; per-run settlement violates the rate tier

The ledger leg of the burn is the epoch-cadence burn-first batch:
`epoch_settlements` composes `BurnFrom(liability, B)` → `MintTo(pool, …)`
→ `MintTo(treasury, …)` → true-burn sink
(`riir-dapps/src/kat/compose.rs:248-277`, order at `:259-260`), executed
by the crank's settle leg (`riir-dapps/src/kat/crank.rs:153-164`),
funding the mining+trainer pool with 70% of burn (`pool_share_bp 6_500 +
staking_share_bp 500`, `compose.rs:100-108`; "the pool is demand-funded
by the burn split", `riir-dapps/src/kat/rewards.rs:38-41`). Settlement is
Glacial-class by contract: "does its write rate fit a Glacial tier
(≤ 0.1 Hz)? Hot/Warm rates disqualify a settlement **regardless of what
it carries**" (`riir-dapps/AGENTS.md:55-57`), with the measured floor —
chain write 1,344.5 µs p50, `commitment_root` 31.6 ms at 10⁵ accounts
against a 50 ms hot-tick budget — at `riir-dapps/AGENTS.md:59-65` and the
verdict "quorum is Glacial (≤ 0.1 Hz), *never 20Hz*" at `:66-70`.

Naming the limit per the two-limits rule (`riir-chain/AGENTS.md:66-75`):
a per-run burn is neither a batched async burst nor bounded settlement —
it makes ledger coordination cadence = heal-run cadence (unbounded
aggregate across users, prompt-coupled UX), failing the capacity-share
column outright and re-creating exactly the per-event write rate the tier
model forbids. The epoch batch already absorbs the burst idempotently;
per-run writes would multiply settlement work by developer activity for
no settlement the epoch batch does not already perform.

## The refuted alternative (what a per-run prompt would have looked like)

- Plain `cargo heal` / `cargo heal --fix` opens an interactive
  "this run will burn N KAT; contribute unheal telemetry? [y/N/a]" gate
  before doing work.
- The charge moves from the meter watermark to a per-run ledger/chain
  (or per-run server round-trip) burn.
- Consent becomes per-invocation state instead of the machine-global
  `mining_consent.json` record.

Why each half fails: the burn half re-implements what `KatCharge::settle`
already does, minus the crash-safe watermark and salvage semantics
(`src/kat_meter.rs:442-454`; `recovered` at `:428-434`; the
version-mismatch salvage test at `:770-795`); the prompt half breaks
non-interactive users (R2) and re-litigates the free tier's settled
pricing every invocation (R3); the ledger half violates the tier model
(R4).

## What would change this verdict

1. **An explicit opt-in PAID per-run tier at epoch cadence.** Per-run
   *pricing* shown to a logged-in user who asked for it — settlement
   still batched at sync/epoch. Requires three deliberate contract
   changes in one act: an amendment to the CLI UX contract
   (`AGENTS.md:175-179`), a revision of this proposal's funnel table
   (`008:253-257`), and a consent-model extension (an ask-once flag or a
   per-run tier in `MiningConsent`). Not a tuning change.
2. **Evidence the just-works funnel fails without per-run re-consent** —
   e.g. telemetry showing `Unset` users feel deceived at rates `--unmine`
   does not capture. Even then the fix is ask-ONCE (a first-run prompt
   that writes the consent record), not per-run.
3. **A tier-model change** legalizing settlement at user-action cadence —
   needs new measurements overturning `riir-chain/.benchmarks/020` §G6e
   and `028`. Nothing current suggests that.

## Honest caveats

- **"Burn on each run" does not exist today in the sense the question
  assumes.** The only burn in this repo is the local projection
  (`src/kat_meter.rs:328-332`); no chain or ledger write happens per run
  anywhere in the stack. R4 therefore rules out an alternative, not
  shipped behavior — and the server-side reconcile that would make the
  burn real debt (`008:268-272`) is `kat-service`/`riir-dapps` surface
  work, not yet landed (`008:284-286`).
- **The free tier's egress is REQUIRED** (anonymous tier "pays in
  unheal", `008:255`) — this verdict defends the amendment's settled
  pricing, not per-run user choice on the free tier; choice exists at the
  logged-in tier (`008:256`) and globally via `--unmine`
  (`src/kat_mining_consent.rs:60-62`).
- **The unsigned anonymous sync arm is still open** (`008:264-267`) —
  until it lands, "ships with (or after) the run" is design, not shipped
  behavior. This verdict does not change that dependency.
- **Epoch cadence here means the ledger batch cadence** (the crank settle
  leg settling completed epochs, `crank.rs:153-164`) — not proposal
  008's agent advisory epochs, which are a separate, unrelated timer. Do
  not conflate them when reading R4.
- Filed docs-only in a shared worktree (sibling agent editing code); no
  code, no `*.rs`, no AGENTS.md/README touched, nothing committed by this
  filing.
