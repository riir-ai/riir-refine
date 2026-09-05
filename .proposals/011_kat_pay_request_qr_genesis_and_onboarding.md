# Proposal 011 — KAT pay-request QR (invoice as NFT), genesis ceremony + root-SSH account, network deploy UX, wet-run, and the bar/foo/baz onboarding loop

Status: **draft**
**RE-TIERED 2026-09-04 — OWNER DECISION (riir-clippy Issue 068 — resolved + removed
2026-09-05; the tier digest lives in riir-clippy AGENTS.md "Issues 061 + 068" + git
history): Tier 0 (devnet + testnet, operator-owned accounts only, no fiat, no custody) is AUTHORIZED and needs no counsel; Tier 1 (non-operator accounts, or the operator loop on mainnet) carries one narrow counsel question plus 005 caveat #7's Phase-1 exit list; Tier 2 (live USDC custody, live Stripe) is not proposed. Caveat #1 below is NARROWED accordingly — this proposal framed the initial surface as "the closed-loop bar→foo operator network" without naming a network, and the decision pins Tier 0 to devnet/testnet so the posture question cannot be asked of it. Phase 2 gains T2.0, an agent-executable DEVNET ceremony rung that rehearses the owner's one-shot testnet act.**
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: [Proposal 005](005_kat_heal_service_network.md) (KAT service network + payment rails) × [Proposal 007](007_kat_usdc_staking.md) (Solana-Pay rail, staking, authority share) × [Proposal 010](010_kat_yield_ceiling_and_treasury_funded_free_tier.md) (yield ceiling + treasury free tier) × riir-chain [Proposal 008](../../riir-chain/.proposals/008_payment_memo_commitment_and_memoshard.md) (payment memo / MemoShard) × riir-ai [Proposal 040](../../riir-ai/.proposals/040_riir_deployer_game_and_chain_devops.md) (deployer chain domain, devnet/testnet/mainnet topology)
Related: riir-deployer `Proposal 001` (control plane / node tiers), riir-dapps Plans 004/005/013 (Stripe / Solana-Pay rails / kat-service worker shell), [Proposal 003](003_binary_only_distribution.md) (binary-only distribution — governs the install surface)

## TL;DR

This proposal turns the KAT network from "a healing tool with a ledger" into a
**product loop**: anyone (`foo`) downloads the binary, boots it against their
SSH identity, sees a balance and a welcome, and earns/spends KAT — because an
operator (`bar`) ran a genesis ceremony and deployed a 3-node network first,
and the whole loop is proven by a **wet-run** (a real paid transaction round-trip
that refunds itself from a faucet). The owner asks (nine original + five session
follow-ups — the verdict table below carries all fourteen) map onto substrates
that are ~80% already shipped: the top-up QR (`payment_url` + `render_qr_ascii`),
the claim rails (`solana`/`stripe`), the five-check verification, the deployer
ops ladder + verify probes, and the chainspec genesis format. What is genuinely
new: a `PayRequest` object (any-amount capable, invoice-indexed, QR-asset with
bit-exact recheck), a `riir-deploy genesis` ceremony driven by a params yaml
whose defaults ARE the ratified tokenomics constants, an interactive deploy
target selector, a `node-sync` probe, and the wet-run harness. This is a
katgpt/riir composition, not prior art — the QR/invoice pattern is Solana Pay
v1 (cited), everything else composes in-house substrate.

## The problem this solves

Everything mechanical exists; nothing product-shaped does. Today: the top-up QR
appears only at 0-KAT (`heal.rs` — `payment_url` + `render_qr_ascii`), a payer
cannot *request* a payment, there is no invoice artifact to show in a wallet,
genesis means hand-editing `chainspec/devnet.toml` (whose own header forbids
deriving testnet from it — so **testnet.toml and mainnet.toml do not exist**,
the Proposal 040 D4 gap), `riir-deploy` takes a `--dest` flag with no target
selection or post-deploy sync proof, and there is no end-to-end loop test that
proves pay → record → reward with real (testnet) money. Foo cannot onboard; bar
cannot promote past devnet; nobody can prove the network works.

## The owner asks → verdicts

| # | Ask | Verdict | Composes (grep-verified) | New surface |
|---|---|---|---|---|
| 1 | Request KAT/USDC via CLI QR; index/ledger/NFT the invoice; bit-exact QR + tx recheck before wallet display | **Ship — KAT rail FIRST via a CF web wallet (owner call 2026-09-01): scan → GitHub-SSH sign-in → press-and-hold SEND 3 s → settle. Stripe/USDC rails = mock-tested for now** | `payment_url`, `render_qr_ascii` (qrcode crate already a dep), the genesis root account = bar's SSH key (ceremony, ask 4), GitHub `.keys` identity, MemoShard design, ARTB content-hash pattern, NftProgram | `PayRequest` + `payreq` ledger row + the web wallet (OAuth + key-match + hold-send) + QR-asset recheck + optional chain NFT |
| 2 | Payer == receiver must work | **Ship — as an explicit test matrix** | wet-run loop IS self-pay; chain `TxDelta` transfer semantics | self-pay fixtures (chain + ledger + payreq) |
| 3 | Blank amount param = any amount | **Ship** | Solana Pay v1: omit `amount` → payer composes | `amount: Option<u64>` end-to-end; claim drops the amount gate |
| 4 | Genesis mints KAT via params yaml; defaults = tokenomics; root = deployer SSH, auto-copied to nodes, env-overridable | **Ship — via `riir-deploy genesis`** | `[[genesis]]` chainspec rows, KatConfig constants (008/010-ratified), SSH-key account derivation, `KAT_ANCHOR_PUBKEY_HEX` env pattern, `chainspec_ceremony.md` runbook | genesis params yaml + ceremony + SSH provision step |
| 5 | Deploy CLI: arrow-select target (default devnet), ack config, ack version | **Ship** | deployer ops-ladder ack discipline (`--i-know-this-wipes-N-nodes`, admin key + `--reason`); 040 networks table | TTY selector + two acks (non-TTY = flags only) |
| 6 | Post-deploy: 3-node availability + sync percent | **Ship** | `chain-height` probe (must advance), `/node-info`, catchup substrate | `node-sync` probe (reachability + height + sync% table) |
| 7 | Wet-run e2e (scan→fix→rewards→record→self-evolve→mesh 100%); real pay + equal refund on testnet; mainnet = dry-run only, no state | **Ship — faucet-funded refund, mainnet-refused** | kat_drain `--dry-run` precedent, lease/sync/meter plumbing, /balance route | `cargo heal wetrun` + G-WET gate |
| 8 | Onboarding: install via sh; run with default SSH; welcome + balance; bar pre-does genesis; earned/spent history | **Ship — with one owner correction** | 003 binary-only distribution; `login`/`account`; `/balance`; kat_charge.json | install.sh copy tweak; login default-SSH; `account --history` |
| 8.1 correction | "releases at gist-rs/riir-clippy" | **CORRECTED** | 003 hard rule: private-repo release assets are not publicly downloadable; the sanctioned surface is **gist-rs/cargo-heal** releases + install.sh (zero .rs files) | none — align to 003, do not open riir-clippy releases |
| 9 | Fixer tier "baz" (local LLM earns KAT) — my call | **DEFER to its own plan** | l4_lora seam + l4_daemon (`bonsai_clippy_l4_eval --serve`) + fixer_regate harness all ship | nothing now — see §Baz |
| 10 | Free-tier daily crank: CF schedule credits every synced account daily | **Ship — a GRANT crank, not a mint faucet** | 010 M2 treasury-first grant + floor, the kat-service DO alarm (10-min sweep/settle/anchor), housekeeping cron/cursor precedent, the exactly-once `(date, x)` key pattern, 008's never-lever free-grant family | daily pass in the alarm + `FREE_TIER_DAILY_KAT` (genesis yaml) + activity gate + budget cap |
| 11 | Faucet request by QR (owner scans; Discord bot long-run) | **Ship v1 = a `faucet` preset on the ask-1 web wallet; Discord bot DEFERRED** | the ask-1 web wallet (GitHub-SSH + hold-SEND + G-WEB), payreq labels/caps, the wet-run faucet leg | `faucet --request` preset + daily cap; bot later |
| 12 | USDC pay → register → mint KAT ("pay and register in one take"), memo links back to the KAT account, tests | **Ship later-phase — with one binding correction: the REFERENCE key is the mandatory machine link; the memo is mandatory-in-URL audit copy** | Solana Pay transfer-request + five checks, on-curve reference keys (dapps Plan 005), 007's token-balance-delta address capture, the account-key-signed sync envelope, chain 008 MemoShard (the later on-chain leg) | registration-intent rows + funding-address binding + settle/mint path + the G-REG test matrix |
| 13 | Walk-to-scan QR is hacky — share the LINK instead; render the QR in the browser as optional; with a timeout | **AGREED — link-first: the URL was always the primitive, the QR is a rendering** | the PayRequest URL + signed resolve + the existing expiry field, the web wallet (no camera needed on any device), the page's Solana-Pay QR rendering for wallet-scan rails | CLI prints the link first (`--qr` fallback); `link_ttl` expiry; expired = honest expired state + settle-refused |
| 14 | Record EVERY request in DB — track, limit, and quick mapping on chain from that shard | **AGREED — promoted to the request-ledger rule (§13)** | Warm-tier rows (826 ns/write), the queue.rs write-ahead intent discipline, MemoShard commitment mapping (chain 008) | lifecycle rows (`created→resolved→settled\|expired\|refused`) with a `chain_anchor` slot; the limiter READS these rows; refusals are rows too |

## The proposed design

### 1. PayRequest — the payment-request object (ask 1)

Two rails, one object. The request is created by the payee, rendered as a QR,
paid by the payer, settled, and kept as an invoice.

```
PayRequest {
  id: [u8; 16]              // BLAKE3(canonical bytes)[..16] — the invoice id
  payee_account: AccountId  // BLAKE3(pubkey)-derived (the house derivation)
  token: Token::{Kat, Usdc}
  amount: Option<u64>       // None = ANY amount (ask 3)
  reference: [u8; 32]       // fresh on-curve key (KAT) / reference key (USDC) — the binding
  memo: Option<String>      // human label ("invoice #1042")
  created_unix, expiry_unix
}
```

Three rails, one object — **the KAT rail is the default and ships first**
(owner call 2026-09-01: it is the easiest, because the genesis ceremony already
minted the operator's wallet); Stripe and Solana-Pay are exercised as MOCK
tests until the KAT loop lands.

- **KAT rail (in-network, DEFAULT — the web-wallet flow):** the request IS a
  **shareable link** — `{service}/payreq/{id}` — the same host as the
  existing `/pay?acct=` top-up page. The QR is just one rendering of that
  URL (§13): foo shares the LINK, bar opens it on any device; the page:
  1. resolves the signed PayRequest (payee, amount — or "any amount", expiry);
  2. **Sign in with GitHub** → OAuth → the worker fetches the signer's
     published SSH keys (`https://github.com/{user}.keys`) and derives each
     key's account id (the house `BLAKE3(pubkey)` derivation) — the account
     that matches the ledger IS the session (GitHub enforces one-key-one-
     account, so the binding is unique; zero usernames typed, zero config);
  3. shows the invoice view (payee, amount, own balance) and a **SEND button
     that must be pressed AND HELD for 3 seconds** — release early cancels —
     preventing the accidental tap;
  4. settles: the worker moves KAT **inside the ledger** (Warm-tier rows, one
     `CommitBatch` per op — the 007 discipline; the chain never sees a
     per-payment write, per the rate test) → invoice row written → foo's CLI
     `account` shows the credit on the next poll.
  The genesis alignment is the reason this is easiest: the ceremony derives
  the root account from **bar's SSH public key** (ask 4); when bar publishes
  that same key on GitHub, the web wallet's key-match finds the funded genesis
  wallet with nothing to configure — the genesis wallet already holds the
  mint, exactly as the owner framed it.
- **Stripe rail (money-proof API → mint KAT; MOCK for now):** the existing
  claim machinery (`rail: "stripe"`, event-id reference) mints KAT on webhook
  proof. Kept green via fixture tests only this phase.
- **USDC rail (Solana Pay transfer-request → mint KAT; MOCK for now):** the QR
  would encode `solana:<payee>?spl-token=USDC&amount=…&reference=…` —
  `amount` omitted ⇔ payer chooses (the spec's own semantics) — verified by
  the **existing five checks** (finalized; SPL-USDC to payee; amount ≥ minimum
  only when pinned; reference present; signature unseen →
  `blake3("solana-tx:"‖sig64)` dedupe). Any Solana Pay wallet scans it; until
  this phase ships, the rail runs against recorded fixtures, not mainnet
  Solana.
- **Invoice (ask 1, the ledger/NFT half):** every PayRequest settles into a
  `payreq` invoice row: `{id, payee, payer, token, amount_paid, settled_unix,
  qr_png_blake3}` in the Warm tier (the MemoShard placement: commitment/pointer
  on the money path, bytes off it — chain Proposal 008 §3/§4). The QR PNG is a
  BLAKE3-committed asset of the row (the ARTB `content_hash` pattern — no
  vessel machinery needed for the KAT wallet).
- **Optional chain NFT (opt-in, Glacial):** an invoice above a threshold (or on
  explicit request) mints via the existing `NftProgram`/registry composition
  (`dapps::item_nft` shape). Three-test: product PASS (an NFT is a token),
  value PASS (the invoice NFT is an authority binding on the payment), rate —
  **opt-in per invoice, never per-payment-by-default**; high-volume settlement
  batches through the epoch intent path like everything else.
- **Bit-exact recheck before display (ask 1, the sanity half):** the wallet
  (CLI `account` view now; `riir-wallet-wasm` later) renders an invoice only
  after ALL of: (a) stored `qr_png_blake3` == recomputed hash of the fetched
  PNG; (b) a **fresh re-encode** from the canonical request bytes is
  byte-identical to the stored PNG (QR encoding is deterministic — the
  `qrcode` crate version, ECC level, and quiet-zone are pinned by a golden
  test, the same re-pin discipline as every wire golden); (c) the tx is
  re-verified (five checks for USDC / ledger row lookup for KAT). Any mismatch
  ⇒ refuse to render + tamper indicator.

### 2. Self-pay (ask 2)

Payer == payee is legal end-to-end and pinned by tests, because the wet-run
(loop below) IS a self-pay. Fixtures: (a) chain `TxDelta` transfer A→A —
balance bit-identical, nonce advances, exactly one log row, no double-credit;
(b) ledger grant/burn to self — conservation holds (`wallet + till` style);
(c) payreq A→A — settles to a paid invoice, no net supply change, and the
reward leg cannot re-trigger off it (the faucet refund is single-shot per
wet-run id). The deduct-refill hazard the owner names is exactly the wet-run
loop, so the loop test doubles as the regression guard.

### 3. Any amount (ask 3)

`amount: None` end-to-end. URI: the `amount` query param is omitted (Solana Pay
v1 semantics — the payer composes the amount). Claim: the five-check amount
gate fires only when the request pinned an amount; an any-amount claim accepts
`amount > 0` and the invoice records what actually arrived. Payer UI (CLI and
page) prompts for the amount when the request says any. Abuse bound: the
payee's own `minimum` (existing config) still floors dust.

### 4. Genesis ceremony (ask 4)

`riir-deploy genesis --params genesis.yaml` (dry-run print by default;
`--execute` applies — the kat_drain precedent: the printed plan IS the
deliverable until you opt in).

```yaml
network: testnet                  # devnet | testnet | mainnet
nodes: 3                          # 040 topology: devnet 1, testnet/mainnet 3
root:
  ssh: auto                       # auto = deployer machine's default key (~/.ssh/id_ed25519.pub)
  env_override: KAT_ROOT_PUBKEY   # CF secret / server env — wins over auto
tokenomics:                       # DEFAULTS = the ratified constants; drift ⇒ ceremony FAILS
  max_emission_kat: 100000000000  # REWARD_MAX_EMISSION_DEFAULT (010/008)
  reward_decay_bp: 9900           # 010 T2.5 ratification (saturation = zero upward headroom)
  yield_ceiling_bp: 22            # YIELD_CEILING.max_bp — genesis-saturated
  epoch_split_bp: [6500, 500, 2000, 1000]  # [mining, staking, treasury, true_burn] — sums to 10 000
  treasury_grant_floor: 1000000   # KAT — the 1 M KAT M2 runway (10^12 micro = FREE_GRANT_KAT × 10 000)
  free_tier_daily_kat: <n>        # ask 10 — daily crank amount (never-lever family)
  faucet_daily_cap_kat: <n>       # ask 11 — per-request faucet cap
  usdc_kat_sub_split_bp: 7000     # 010 T1.2 — StakingParams::DEFAULT USDC/KAT class sub-split
  staking_share_bp: 500           # 010 Phase 4 ACTIVATED 2026-08-30 (the 0-bp bootstrap posture is
                                  #   historical): ceiling armed in the same default, ordering constraint
                                  #   enforced — equals epoch_split_bp[1]; payout routes to the reward
                                  #   pool until 007's own staking destination lands
  authority_share: { initial_bp: 2000, floor_bp: 500, decay_epochs: 60 }
mint:
  root_float_kat: <n>             # pre-mine; MUST be ≤ max_emission − (treasury + faucet rows)
faucet:
  label: faucet
  kat: <n>                        # wet-run + grant float — "give the faucet an owner" (devnet.toml)
```

The ceremony: validate params against the pinned `KatConfig` constants
(genesis is where parameter freeze is *enforced*, per 007 caveat #6 — the yaml
cannot drift from tokenomics, it can only restate it) → generate **fresh** node
keys, publish public halves only (the devnet spec's own rule, encoded) → write
`chainspec/<network>.toml` `[[genesis]]` rows for root / treasury / faucet
(`balance = [i64; 8]`, `player_id_hash` derived by the loader as today) → emit
the KAT registry genesis (register + mint to root/treasury/faucet through the
dapps composition — pre-launch, Glacial) → **SSH provision**: the root account
is derived from the deployer's default SSH public key (the m3 mac's
`~/.ssh/id_ed25519.pub` — the same pubkey-as-identity derivation every KAT
account uses), the ceremony `ssh-copy-id`s it into each node's
`authorized_keys` (handling the Windows-admin `administrators_authorized_keys`
path the 4090 nodes need — the Plan 318 gotcha), and `KAT_ROOT_PUBKEY` (CF
secret or server env) overrides auto-detection. This authoring step is also
what closes the 040 D4 gap: **`testnet.toml` and `mainnet.toml` come into
existence by ceremony, never by editing devnet's `network` line.**

### 5. Deploy UX (ask 5)

```
$ riir-deploy deploy
  ? Target network   devnet / testnet / mainnet      (↑/↓ · default devnet)
  ? Config           chainspec/testnet.toml · 3 nodes · rolling plan211 — deploy? y/N
  ? Version          riir-chaind <semver> · blake3 <ab12…> — deploy? y/N
```

TTY present ⇒ arrow selector (devnet default). Non-TTY ⇒ flags only
(`--dest/--yes`), so CI never blocks on a prompt. Mainnet is NEVER the selector
default and adds the ops-ladder discipline: explicit `--dest mainnet`, admin
key + `--reason`, typed acknowledgment. The two acks (resolved config, artifact
version+hash) are the manifest-driven staging surfaces read back to the
operator before execution — no new trust model, the ops ladder already refuses
without them.

### 6. Post-deploy health (ask 6)

New `node-sync` probe beside the existing `chain-height` (which already demands
the height ADVANCE — a frozen height is a halted node): per node — RPC
reachable, `/node-info` pubkey matches the published pin, height advancing, and
`sync% = node_height / max(peer_heights)`. Output is a table; pass = all nodes
reachable + advancing + `sync% ≥ 99`. `riir-deploy verify --network testnet`
gates the 3-node topology (devnet's 1 node passes trivially). Peer heights come
from the nodes' own `/node-info` + the bootstrap set in the chainspec — no new
network service.

### 7. Wet-run (ask 7)

`cargo heal wetrun` — the full-loop proof with real money movement on
devnet/testnet:

```
1. preflight (read-only): 3/3 nodes healthy, sync% 100, my /balance fetched
2. faucet tops MY account +X KAT            (real ledger rows, testnet)
3. one scored scan→fix cycle on a seeded dirty fixture (charge meter records the spend)
4. sync push (signed, redact ≥ floor) → service records → self-evolve dry-run queued
5. faucet refund credit X arrives           (labeled wet-run refund — see the gate)
6. mesh readiness: all nodes healthy + sync% 100 → "ready" verdict printed
```

**The refund is faucet-funded, not pool-funded** — step 5 is a faucet credit
labeled as the wet-run refund, and the G-WET gate asserts the mint/burn
counters are bit-unchanged by the whole run. Mining/pool economics are never
"promised X back" by a test harness; the loop proves the *plumbing* (pay →
ledger → record → credit → mesh), which is what "rewards back in the same
amount" needs to demonstrate. **Mainnet: `wetrun` refuses** (network check in
the CLI; no env override — deliberately, the way `RIIR_ALLOW_MAINNET_WETRUN`
would be an attack on our own bookkeeping). `wetrun --dry-run` (steps 1+6,
read-only, zero state) is allowed on every network and is what mainnet gets:
no touch, no record, no rewards — the state stays clean by construction, which
also closes the "dryrun dirties state / enables attack" concern.

### 8. Onboarding — bar, foo (ask 8)

- **bar first (8.4):** `riir-deploy genesis --params genesis.yaml` (dry-run
  print, review, `--execute`) → `riir-deploy deploy` → `riir-deploy verify`
  (3/3 + sync 100%). Once, by the operator. This is what makes foo trivial.
  Paying foo afterward is the owner's flow: foo runs `pay --request`, bar
  scans the QR → web wallet → GitHub sign-in (the key-match finds the genesis
  root wallet, since the ceremony derived it from bar's SSH key) → **hold
  SEND 3 s** → foo sees the credit.
- **foo (8.1):** `curl -fsSL https://raw.githubusercontent.com/gist-rs/cargo-heal/main/install.sh | sh`
  → mac-arm binary from the **gist-rs/cargo-heal releases** (the 003 public
  surface — NOT riir-clippy releases, see the corrected row in the verdict
  table; local uploads via `gh release create` are exactly what the runbook
  sanctions while CI budget is tight).
- **foo (8.2):** `cargo heal login` — exists; T0.2 extends it so the default
  key source is the machine's default SSH public key (`~/.ssh/id_ed25519.pub`)
  unless `RIIR_HEAL_KAT_KEY` overrides. Account id = the existing
  BLAKE3(pubkey) derivation. Zero configuration.
- **foo (8.3):** `cargo heal account` — exists; T0.2 adds the welcome gate:
  when all-synced + nodes healthy, print balance + grant + the welcome line
  (no $ beside KAT — the caveat-#15 copy rule, enforced by test as everywhere).
- **foo (8.5):** `account --history` — earned (mining credits, settlement,
  wet-run refunds labeled) vs spent (charges, lease) from the local
  `kat_charge.json` + the service `/balance` history (route exists).

### 9. Baz — the fixer tier (ask 9, the owner-delegated call)

**DEFER to its own plan. Not now.** The seam already ships (`l4_lora` →
`L4Fallback` → `DaemonL4` = `bonsai_clippy_l4_eval --serve` client), and the
m3's local Bonsai is the right substrate for the local-LLM leg. The measured
blockers stand: fixer G1 is **0/60 degenerate** (riir-train Plan 336, Benches
465/467 — tied with the frozen-backbone control), the serving-lane decision is
local-Bonsai **opt-in + UNARMED** with decline as default and the CF-llama arm
awaiting the owner's leak-posture sign-off, and the reward side needs 008
Phase-4 dopamine levers — blocked by the G5 verdict (FAIL-wash; the dao is
advisory-only, terminally, until a NEW plan with v2 candidates re-opens it).
Baz earns KAT only after: (a) a fixer arm beats `decline` on the reachable
case set, (b) leak-posture sign-off, (c) reward economics with G5-class
evidence. Opening that plan before clippy-tier onboarding lands would put the
least-proven earning lane in front of the newest users.

### 10. Free-tier daily crank (ask 10)

The one-time login grant (`FREE_TIER_GRANT_KAT`) covers install-day; it does
not survive a week of daily use. The crank makes the free tier renewable:

- **Where:** a daily pass in the kat-service DO's existing alarm (the 10-min
  sweep/settle/anchor tick gains a date-bucketed grant sweep — the DO alarm is
  guaranteed to wake the DO; the housekeeping worker's scheduled-cron +
  cursor discipline is the ops precedent).
- **What:** for every ledger account with activity (a successful `/sync` push
  or `/lease` pull) within `FREE_TIER_ACTIVITY_DAYS` (default 7), call the
  **existing M2 grant** (`KatLedger::grant`) for `FREE_TIER_DAILY_KAT`. M2
  makes this treasury-first — zero mint pressure above the
  `treasury_grant_floor`; the mint fallback at/below the floor is the floor's
  own ratified design — so the ask's "mint KAT to each account" is implemented
  as a **grant**: one entry point, supply accounting already proven.
- **Exactly-once:** idempotent per `(account, utc_date)` — the same shape as
  the KAT mint swap-in's exactly-once `(date, neurons)` key. The clock is DO
  host time, never caller-supplied (the Issue-004 rule).
- **Bounds:** the amount joins the 008 never-lever free-grant family
  (genesis-frozen, never agent-movable); a global `daily_grant_budget` caps
  total outflow per day (overflow skips + alerts via the existing housekeeping
  path); the activity gate is the sybil tax — a farm of keypairs must actually
  run syncs to harvest. Day-0 coverage stays with the login grant; the crank
  sustains from day 1.
- **UX:** `account` shows the daily credit line and 8.3's welcome reads it.

### 11. Faucet by QR → faucet by LINK (ask 11, revised by ask 13)

No new mechanism — the faucet request IS a PayRequest with a preset:
`cargo heal faucet --request` creates a payreq whose label is `faucet`, amount
fixed to `faucet_daily_cap_kat`, payer hint = the root/faucet account — and
**prints the shareable link** (the ASCII QR renders only as a TTY fallback or
with `--qr`; the page renders the QR optionally for wallet-scan rails). Foo
sends the link to bar over any channel; bar opens it → web wallet (GitHub-SSH
sign-in, hold-SEND — all of ask 1's machinery and G-WEB gates) → settle.
**Shared links carry a TTL** (`link_ttl`, default 24 h for faucet — expiry is
already a PayRequest field): an expired link resolves to an honest "expired"
state and settle-refuses. No camera, no walking to the machine. The wet-run's
programmatic faucet leg already proves the ledger half. **Discord bot:
DEFERRED** — with link-first sharing the manual path is just "send a URL", so
the bot's only remaining value is automation; reopen when volume demands it,
gated on a per-account daily cap + a proof-of-identity no weaker than the
GitHub-SSH key-match.

### 13. The request-ledger rule (ask 14 — track · limit · chain mapping)

Every request — payreq create, faucet request, registration intent, resolve,
settle — is a **row first** (write-ahead, the queue.rs intent discipline):
`{request_id, kind, account, status: created→resolved→settled|expired|refused,
amounts, unix, chain_anchor}`. The row is not bookkeeping after the fact; it
is the gate:

- **Track** — the lifecycle of every request is auditable end-to-end by id.
- **Limit** — the rate limiter READS these rows (no second store): per-account
  request counts and per-kind caps (the faucet daily cap, `daily_grant_budget`,
  requests-per-window) are enforced from them, and a refusal writes a
  `refused` row — the limits are auditable too. Cost is trivial at request
  cadence (Warm-tier writes measured at 826 ns).
- **Chain mapping ("from that shard")** — every row's `chain_anchor` slot is
  filled when epoch settlement composes the chain intent (the MemoShard-shaped
  commitment id, chain Proposal 008): one lookup maps any chain artifact back
  to its request row and any request forward to its chain artifact — which is
  what makes reconcile, expired-intent recovery, and audits tractable.

### 12. USDC pay-and-register in one take (ask 12 — later phase, tests now)

For real users later: paying USDC should both fund and REGISTER — the payer's
Solana address becomes the funding address bound to their KAT account.

1. `cargo heal pay --token usdc [--amount N]` → POSTs a **registration
   intent** `{reference, kat_account, min_amount, expiry}` (account-key-signed
   — the v2 sync envelope; nobody can bind an intent to an account whose key
   they do not hold) → composes the Solana Pay transfer-request URL: revenue
   payee, the fresh on-curve reference, min amount, **memo = KAT account id +
   label, always composed by us** → QR. The same page on the kat-service host
   renders it + polls — the site "takes care of that like usual payment".
2. Foo pays from any Solana wallet. The watcher (five checks, existing) finds
   the tx by reference → finalized / payee / amount / reference-present /
   sig-unseen → captures the **payer's source address** from the token-balance
   delta (007's refund-to-source capture, reversed) → writes the funding
   binding → credits KAT per the pack economics → invoice row.

**The binding verdict (the memo-mandatory ask, corrected):** the **reference
key is the mandatory machine link**; the memo is mandatory in the URL and an
on-chain audit copy. Solana Pay's `memo` is composed by the PAYER'S wallet,
and wallet support is inconsistent — a payment whose wallet dropped the memo
must still settle via the reference (the intent row already carries the
linkage), flagged `memo_missing`; refusing it would orphan real funds.
Memo-present/reference-absent is impossible by construction (the reference
rides the tx account keys, not the wallet's goodwill). When the KAT side later
settles on riir-chain, chain 008's MemoShard is the memo vehicle —
complementary, not this phase.

**Test matrix (G-REG, the owner's demand):** happy path (credit to the right
account + funding binding + invoice); memo-dropped ⇒ settles + `memo_missing`
flag; replayed signature ⇒ rejected; wrong payee ⇒ rejected; amount < min ⇒
rejected; expired intent ⇒ stuck-funds reconcile path (honest gap — outbound
refund signing is 007 T2.2-deferred; long expiry + ops reconcile + the funding
binding enable a late manual credit); forged intent for a victim account ⇒
refused (intent POST is account-key-signed); self-pay USDC ⇒ allowed, binds
funding, credits once; double-processing ⇒ single credit (sig dedupe); one
account paying from two Solana addresses ⇒ first-wins binding, later recorded
+ flagged.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **The KAT rail ships FIRST (owner call 2026-09-01) but its public launch
   still amends the 005 caveat-#7 posture (counsel gate).** 005 ships KAT as
   non-transferable, non-redeemable service credit; user→user KAT payment is a
   transferability change — the same regulated-activity surface 007 flagged.
   The owner's sequencing is respected (KAT loop lands first, Stripe/USDC
   mocked), and the initial surface is the closed-loop bar→foo operator
   network — but **opening the KAT rail beyond operator-owned accounts is
   gated on the same single counsel review** (007 caveat #1 — one review
   covers staking custody + the transfer rail). The Stripe/USDC rails carry
   no new posture (inbound-only, existing watch-only machinery).
2. **Wet-run is a faucet loop, not an emission channel.** If the refund ever
   routes from mint or the burn pool, `wetrun` is a money printer with a test
   wrapper. The G-WET assertion (mint/burn counters bit-unchanged) is
   load-bearing; so is the labeled faucet credit. Mainnet refusal has **no
   override** by design — an override env var would be a standing invitation.
3. **QR determinism is a pinned dependency, not a property of QR.** Byte-exact
   re-encode holds only while the `qrcode` crate version + render params
   (ECC, quiet zone) stay pinned. A bump re-pins the golden and every stored
   invoice's PNG (re-encode-on-read absorbs old assets: recheck (b) falls back
   to hash-match-only for pre-bump invoices, still refusing mismatches).
4. **The chain NFT is opt-in per invoice, and MemoShard is still a DRAFT.**
   Minting an NFT per payment at payment cadence would fail the rate test —
   default is the Warm-tier invoice row only; the chain NFT rides explicit
   request/threshold at Glacial cadence. Adopting chain 008's `TxSigningMessage`
   v4 bump (on-chain memo binding) is its own decision — v1 binds via the
   reference key + service signature and ships no signing-message change.
5. **Interactive selection must never bypass typed acks.** The arrow selector
   is sugar over the existing ack discipline; mainnet additionally requires the
   explicit flag + admin key + reason. A prompt is not a security boundary —
   non-TTY CI paths keep the flags-only form.
6. **Genesis ceremony is where mistakes are unbounded.** Wrong root key =
   locked treasury; double mint = supply bug. Mitigations are structural:
   dry-run-by-default, params validated against pinned constants (drift
   fails, no silent clamp), fresh keys with published public halves only,
   and the faucet owner requirement inherited from the devnet spec. The
   ceremony is the ONLY sanctioned author of testnet.toml/mainnet.toml.
7. **`/payreq/{id}` is a new public surface on the live kat-service worker.**
   It inherits the signed-payload + replay-guard + redact-floor lessons
   (v2 sync envelope, `MIN_REDACT_VERSION`) — resolution is signature-verified,
   idempotent, rate-limited; no unauthenticated state writes.
8. **SSH provisioning writes to nodes as root.** Auto-copying the deployer key
   into node `authorized_keys` is convenient and expands blast radius; the
   ceremony prints exactly which keys land on which nodes and `KAT_ROOT_PUBKEY`
   override exists precisely so the default is never load-bearing in prod.
9. **GitHub-SSH web-wallet login is an OAuth-bound identity, NOT private-key
   possession.** The flow proves control of the GitHub account that published
   the matching SSH key (GitHub enforces one-key-one-account, so the account
   binding is unique and stable), but the private key itself is never seen by
   the web wallet — a hijacked GitHub session could move KAT that the CLI
   could not move without the key. The 3-second press-and-hold is accident
   protection, not a security boundary. Hardening ladder (ship in this
   order, each cheap): server-side minimum-hold timestamp (the client's
   3 s is cosmetic until the worker enforces `held_ms ≥ 3000` on the send
   route); a per-send threshold above which the web wallet refuses and
   points at the CLI (which holds the real key); an SSH signature challenge
   over the transfer intent for large sends; the **root/genesis float stays
   CLI-only** regardless — the web wallet spends a working float, never the
   treasury keys.
10. **The crank is a grant engine, not a mint engine — and a sybil surface.**
    Treasury-first per M2 (the word "mint" in the ask maps to `grant`);
    the never-lever freeze means the amount is a genesis decision, not a
    tuning knob; the activity gate + daily budget cap are load-bearing. Size
    `FREE_TIER_DAILY_KAT` so farming keypairs-just-to-sync loses to one real
    healing session — the crank must never out-pay the work it subsidizes.
11. **Memo vs reference (ask 12), restated as a warning.** A
    `memo`-mandatory settlement would strand payments from wallets that drop
    the memo field; the reference key is the binding. The CLI/page copy still
    shows the memo to the payer — the user-visible contract is intact; only
    the settlement gate ignores it.
12. **Shared links are capability URLs.** Anyone holding the link can VIEW
    the request (payee, amount) — the same exposure as the QR it replaces,
    but a link forwards further than a camera scan. Bounds: settlement still
    requires the GitHub-SSH session (G-WEB), `link_ttl` bounds the exposure
    window, and resolve stays signed + rate-limited. View-only must never
    leak more than the request itself (no balance, no history on the
    pre-auth page).

## Fusion lineage

1. **[Proposal 005](005_kat_heal_service_network.md)** — the rails, the
   five checks, the never-brick/copy rules, the closed-loop posture this
   amendments (caveat #1), and the service plane this extends.
2. **[Proposal 007](007_kat_usdc_staking.md)** — the Solana-Pay
   transfer-request composer + QR CLI already proven in `stake` (T2.1), the
   per-op `CommitBatch` ledger discipline, and the counsel-gate pattern.
3. **riir-chain [Proposal 008](../../riir-chain/.proposals/008_payment_memo_commitment_and_memoshard.md)** —
   the commitment-on-chain/bytes-off-chain split that places the invoice row
   in the Warm tier and keeps the money path clean.
4. **riir-ai [Proposal 040](../../riir-ai/.proposals/040_riir_deployer_game_and_chain_devops.md)**
   + riir-deployer Plan 002 — the networks table (devnet 1 / testnet 3 /
   mainnet 3, quorum 2of3), the ops-ladder ack discipline, and the verify
   probes `node-sync` extends.
5. **Solana Pay Transfer Request v1 (prior art, cited)** — the URL schema,
   amount-omitted semantics, and reference-based confirmation; our five checks
   were verified against it in riir-dapps Plan 005.

## GOAT gate (service gates — the healer's offline gates untouched)

- **G1-S (money e2e, exact-once):** payreq create → QR → pay (both rails) →
  settle → invoice row; self-pay matrix green; any-amount claim; replayed
  reference rejected; chaos sweep (16×4 standard) zero loss / zero double-pay.
- **G2-S (perf/placement):** QR render < 50 ms; payreq resolve+settle bounded;
  zero per-payment chain writes (grep-pinned); `node-sync` probe < 5 s/3 nodes.
- **G3-S (no-regression):** with the payreq/wetrun features off, riir-clippy
  and riir-dapps default builds are bit-identical to today; the deployer CLI
  without a TTY behaves exactly as now.
- **G4-S (alloc/typing):** integer money end-to-end (KAT micro-units i64, USDC
  base units u64); f32/f64 in any money path = review-blocking; invoice PNG is
  the only new blob and is hash-pinned.
- **G-WET (the loop gate):** on a 3-node local cluster then testnet — full
  wet-run passes, mint/burn counters bit-unchanged, faucet refund lands
  exactly X once, mainnet refusal fires (unit + live), dry-run touches nothing
  (read-only assertion).
- **G-DEPLOY:** ceremony dry-run on a fresh workdir produces a chainspec that
  `chaind --chain-spec` accepts and boots 3 nodes; `node-sync` fails loudly on
  a killed node, a stalled height, and a stale pubkey pin; selector degrades
  to flags without a TTY.
- **G-WEB (the web-wallet auth gate):** only the GitHub account whose
  published keys derive the payee-matched account can obtain a send-capable
  session; a session bound to account A cannot settle a request that pays
  from B; replayed/nonces-fresh sessions refused; the server-side min-hold
  fires when `held_ms < 3000`; a send above the threshold is refused with
  CLI pointing; the root float account is refused outright.
- **G-CRANK:** two alarm passes on the same UTC date credit exactly once per
  account; a dormant account (outside the activity window) earns zero; the
  budget cap skips + alerts; below-floor treasury follows the M2 fallback
  exactly; the amount constants are asserted immutable (never-lever pin).
- **G-REG (the register matrix):** the ten §12 cases green against recorded
  fixtures — including the forged-intent refusal, the memo-dropped settle,
  and the single-credit dedupe; the live-rail arm stays `#[ignore]` until
  Phase 5.
- **G-REQ (the request-ledger gate):** no request path processes without its
  write-ahead row; the limiter refuses at cap and the refusal is itself a
  row; an expired link resolves to the expired state and settle-refuses;
  `chain_anchor` round-trips (row → chain intent → row) in one lookup.

## What ships now vs deferred

### Ships now — this proposal's phased scope
- riir-clippy: `pay`/`payreq` CLI (create/request/show), `faucet --request`
  preset, `pay --token usdc` intent composer, invoice view + recheck,
  `wetrun` + `--dry-run`, `login` default-SSH, `account` welcome + `--history`
  (feature `kat_payreq`, forwards `kat_transport`).
- riir-dapps: `/payreq` routes (create/resolve/settle/invoice) on the
  kat-service worker + **the web wallet page on the same host** (static
  assets + GitHub OAuth callback + key-match + session + hold-send route),
  `payreq` invoice rows + faucet refund credit, the **daily grant crank** in
  the DO alarm, registration-intent + funding-binding rows (feature
  `kat_payreq`); Stripe/USDC rails stay behind their existing features,
  fixture-tested only.
- riir-deployer: `genesis` command + params yaml + SSH provision; deploy
  selector + acks; `node-sync` probe; chainspec authorship for testnet/mainnet
  (closes 040 D4 / chain Issue 103's authoring half).
- riir-chain-sdk: `pay_request` module — URI compose + resolve + KAT transfer
  encode for payer tooling (USDC arm lands later, behind the live rail).

### Deferred — gated
- Chain-NFT invoice mint (after the Warm-tier invoice proves out; opt-in,
  Glacial).
- MemoShard v4 signing-message adoption (chain 008 decision).
- LIVE Stripe + Solana-Pay rails (mock-tested this phase; the five-check
  machinery is already proven — only the launch decision waits).
- KAT-rail launch beyond operator-owned accounts (counsel gate).
- `riir-wallet-wasm` invoice display (browser wallet; CLI view ships first).
- **Discord faucet bot** (sybil surface; reopen trigger + gates in §11).
- **Live USDC pay-and-register** (Phase 5 — counsel + demand gated; the
  intent/binding substrate and the G-REG fixture matrix land earlier).
- **Baz fixer tier** — own plan, gated per §9.

### Explicitly NOT shipped by this proposal
- A per-payment chain program or per-payment chain writes (rate test rejects).
- An always-mint daily faucet (the crank grants treasury-first via M2 — it is
  not an emission channel, and its amount is never-lever).
- Recurring/subscription requests (chain Proposal 002's territory — complement,
  later).
- Any mainnet wet-run path, with or without overrides.
- Any change to KAT transferability by default (counsel decides; posture
  unchanged until then).
- Public source distribution of riir-clippy (003 stands; cargo-heal releases
  stay the only public binary surface).

## Phased rollout (sketch — plans would expand this)

### Phase 0 — ledger + SDK substrate
- [ ] T0.1 `payreq` request-ledger lifecycle rows + invoice rows + faucet
      credit row type (riir-dapps, `kat_payreq`)
- [ ] T0.2 clippy CLI: `login` default-SSH, `account` welcome/history
- [ ] T0.3 riir-chain-sdk `pay_request` module (KAT compose + resolve/transfer;
      USDC arm stubbed behind the mock)
- [ ] T0.4 self-pay fixtures (chain + ledger + payreq)
- [ ] T0.5 registration-intent + funding-binding row types (the ask-12 substrate)

### Phase 1 — the KAT rail + web wallet (DEFAULT, ships first — owner call 2026-09-01)
- [ ] T1.1 `/payreq` routes (create/resolve/settle/invoice) + request-ledger
      write-ahead rows + limiter-backed-by-rows + signed resolve
- [ ] T1.2 the web wallet on the kat-service host: GitHub OAuth → `.keys` fetch →
      key-match → session → **press-and-hold SEND** (server-enforced min-hold)
      → settle → invoice row
- [ ] T1.3 clippy `pay --request [--amount]` (KAT default; blank = any amount)
      + shareable link first (ASCII QR as TTY fallback / `--qr`) + poll-on-credit
- [ ] T1.4 invoice display + bit-exact recheck (golden-pinned QR)
- [ ] T1.5 G1-S/G-WEB money + auth + the adversarial suite (replay, session
      theft, amount manipulation, self-pay, hold-bypass, root-float refusal)
- [ ] T1.6 Stripe/USDC rails as MOCK tests only (five-check machinery exercised
      against fixtures; no live rail this phase)
- [ ] T1.7 the daily grant crank (alarm pass, `(account, utc_date)` exactly-once,
      activity gate, budget cap) + G-CRANK
- [ ] T1.8 `faucet --request` preset + `faucet_daily_cap_kat` enforcement +
      `link_ttl` expiry (expired = settle-refused)

### Phase 2 — genesis ceremony (bar)
- [ ] T2.1 params yaml + validation against pinned tokenomics
- [ ] T2.2 ceremony: keys → chainspec authorship → registry genesis → SSH provision
- [ ] T2.3 testnet ceremony executed once; `testnet.toml` lands (D4 closed)

### Phase 3 — deploy UX + health
- [ ] T3.1 TTY selector + config/version acks (flags-only non-TTY; mainnet hardening)
- [ ] T3.2 `node-sync` probe + 3-node verify gate
- [ ] T3.3 G-DEPLOY on a local 3-node cluster

### Phase 4 — wet-run + foo onboarding
- [ ] T4.1 `wetrun` harness (steps 1–6) + G-WET
- [ ] T4.2 mainnet refusal (unit + live) + `--dry-run` read-only proof
- [ ] T4.3 install.sh copy alignment (cargo-heal releases) + foo end-to-end doc
- [ ] T4.4 testnet wet-run executed; mesh-ready verdict recorded

### Phase 5 — live USDC pay-and-register (real users; counsel + demand gated)
- [ ] T5.1 live Solana-Pay settle path + G-REG matrix run live (fixtures first;
      the `#[ignore]`d live arm un-ignored)
- [ ] T5.2 ops reconcile runbook for expired-intent stuck funds (until
      outbound refunds exist — 007 T2.2 territory)
- [ ] T5.3 testnet-then-mainnet launch decision (the counsel envelope shared
      with the KAT rail)

## Risks

1. **Regulatory (highest, KAT rail only):** transferability misclassification —
   mitigated by the counsel gate + USDC-first sequencing.
2. **Money bugs in a new public route:** the payreq settle path handles value —
   exact-once rows, replay dedupe, and the chaos sweep are the load-bearing
   defenses; the route is small and composes proven rails.
3. **Genesis mistakes:** unbounded blast radius (locked root, wrong mint) —
   dry-run default, constant-validation, fresh keys, single sanctioned author.
4. **Determinism drift:** QR/png re-encode pinning — golden tests + the
   hash-only fallback for pre-bump invoices.
5. **Operational:** SSH-root provisioning on hetero nodes (Windows admin path) —
   handled + printed, override env exists.

## Out of scope

- Governance changes (008's), staking economics (007's), reward-schedule
  changes (010's) — this proposal restates their ratified constants at genesis
  and moves nothing.
- Wallet UI beyond the CLI view; POS/merchant tooling; multi-currency beyond
  KAT/USDC; fiat rails beyond the existing Stripe claim.
- Cross-chain bridges, KAT↔USDC swap (a market, not a payment request).

## References

### In-house (substrates this composes — grep-verified 2026-09-01)
1. `riir-clippy/src/kat_lease_client.rs` — `payment_url`, `render_qr_ascii`,
   `ClaimRequest { rail: solana|stripe }`, `DEFAULT_SERVICE_URL`, signature
   validation (the QR + claim spine).
2. `riir-clippy/src/bin/heal.rs` — `login`/`account`/`update`/`sync`/`claim`,
   the 0-KAT top-up QR, `kat_charge.json` (the onboarding spine).
3. riir-dapps `src/kat/solana.rs` + `kat/stripe.rs` + Plan 013 worker shell —
   five checks, reference keys, the live `/balance` `/lease` `/sync` routes.
4. `riir-chain/chainspec/devnet.toml` — `[[genesis]]` row shape,
   `player_id_hash` derivation, the no-derive rule + faucet-owner rule.
5. riir-ai Proposal 040 §networks + D4; riir-deployer ops ladder + probes
   (`chain-height` must-advance) + `chainspec_ceremony.md` runbook.
6. riir-chain Proposal 008 (MemoShard placement), Proposal 002 (recurring —
   complement), `NftProgram` (invoice-NFT vehicle).
7. 008/010 ratified tokenomics constants (`KatConfig`) — the params-yaml
   defaults; 007's authority-share schedule.
8. The KAT mint swap-in (riir-clippy `a07b9e6` worker half / riir-dapps
   `10ffb18` server half) — the exactly-once `(date, neurons)` grant key the
   crank reuses; the `cloudflare/housekeeping/` worker — the scheduled-cron
   + cursor + alert discipline the crank's ops shape follows.

### External (cited-only)
9. **Solana Pay Transfer Request v1** (docs.solanapay.com) — URL schema,
   omitted-`amount` payer-composes semantics, reference confirmation.
10. **BIP-21** — the earlier amount-optional payment-URI precedent.
11. **GitHub SSH keys endpoint** (`https://github.com/{user}.keys`) — the
    published-keys lookup the web wallet's key-match uses; GitHub's
    one-key-one-account uniqueness is what makes the binding an identity.

## TL;DR

**Ship it phased, KAT-rail-first through the web wallet (GitHub-SSH sign-in,
hold-SEND) with Stripe/USDC mock-tested until the loop lands; genesis by
ceremony (never by editing devnet), deploy behind the existing ack discipline
plus a selector, health proven by a sync-percentage probe, and the whole loop
certified by a faucet-funded wet-run that mainnet structurally cannot run.**
The install surface stays gist-rs/cargo-heal per 003 — the owner's
riir-clippy-releases URL is corrected, not adopted. Baz waits for its own
measured plan. The one hard rule to carry out of this document: the web
wallet's hold-to-send is UX, the session is an OAuth-bound identity, and the
root/genesis float stays CLI-only. The follow-up trio lands inside the same
envelope: the daily free tier is a **treasury-first grant crank** (never a
mint faucet, never-lever), the faucet is a **payreq preset behind the same
web wallet** (Discord bot deferred), and pay-and-register binds via the
**reference key** with the memo as the audit copy — a memo-mandatory gate
would strand real payments. Next action: open the Phase 0 plan
(riir-dapps rows + SDK module + self-pay fixtures), then the Phase 1 web
wallet — and Phase 2's ceremony first, because bar's dry-run genesis is what
puts the mint in the wallet that every foo gets paid from. Requests go out
as **shareable links first** (the QR is an optional rendering — browser page
or `--qr` fallback) with a TTL, and **every request is a row first** —
tracked, limited, and chain-mapped from the same ledger.
