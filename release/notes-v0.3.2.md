# Release notes — v0.3.2 (the claim-predicate cut)

Cut 2026-10-17. The `scripts/manual_release.sh` publish step requires this
file and its `corpus:` line (the line becomes the release TITLE — the update
alert quotes it). Corpus counts from the AGENTS domain tables at cut time
(the per-batch standing counts): clippy_lints 100 → 112, rust_perf 198 → 230,
kernel_opt 795 → 848, dist 7 unchanged.

---

corpus: +97 rules since v0.3.1 — clippy_lints 112 (+12) · rust_perf 230 (+32) · kernel_opt 848 (+53) · dist 7 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.3.1 (user-visible)

- **`cargo refine tuna` tells the truth about YOUR account** — the
  no-spendable-trial-credit answer split on the server's unified claim
  predicate: a consumed free tier now says so ("the free tier is consumed on
  this account — a KAT-plane grant, or TUNA spent/expired; it is granted once
  per account") and an unclaimed one names the automatic claim path
  (`cargo refine account` / any `--mine` sync). The old line blamed
  "pre-TUNA deployment" for the common case.
- **The service planes agree about one account** (riir-dapps 9582bf8, the
  server half your client talks to): the public leaderboard chip, the account
  page badge, and the web-wallet key-match now read the unified
  free-tier claim predicate instead of the raw KAT grant flag — a TUNA-claimed
  account no longer shows "grant unclaimed" on the website, and a TUNA-only
  key can bind a wallet session.
- **Mainnet is the default dial** (2026-10-14 law): a stored `config set`
  row is bookkeeping and never steers; every command targets
  `mainnet.gist.rs` unless `--net <devnet|mainnet>` or
  `RIIR_REFINE_KAT_SERVICE_URL` says otherwise.
- **The account view renders one computed table** — label columns come from
  the riir-auth substrate (`account_line`), the account line shows the FULL
  id, and the `key:` line names its `~/.ssh` source (the SSH-direct posture).
- **The run-end roll-up** — after the findings, one `findings by domain:` line
  over everything the run printed, plus the fix-next CTA and the mine line
  (Issue 165).
- **`buy <amount>` is friction-cut** — MAINNET by default, the exact amount
  resolves the catalog pack, and a no-exact-match refusal renders the catalog
  (nobody guesses a price).
- **The pay catalog carries the trust line** — `pays <payee> · USDC mint
  <mint>` plus the server's own checkout deadline (riir-kat Issue 006).
- **The CLI QR is back at full dense size** (the half-size braille variant
  was too small to scan — owner revert).
- **sec scan precision** — digest-pinned values (blake3:/sha256: tags,
  pin-named declarations) adjudicate benign IN THE REPORT COUNT ONLY
  (disclosed, never silently dropped); GUID identifier tags, bodyless PEM
  delimiter pairs, and GOLDEN pin names join the benign classes. The egress
  gate and `--blind` stay fail-closed.
- **The healer got smarter: 97 new rules** — the byte-scan/search mechanism
  pack (regex-automata + memchr + bstr), the text/unicode family seed (HF
  tokenizers), the simdutf + unicode-normalization conversion pack, the
  serving state-cache + kernel-oracle riders, and the clippy-1.99 new-lint
  wave. `cargo refine update` (the corpus lease) serves the rules; this
  binary serves the machinery.

## Honest exposure bound (the T15 note)

v0.3.2 fixes client display/attribution only; no prior release carries a
newly disclosed defect from this cut. The grant-badge defect lives in the
SERVICE deployment (riir-dapps), already fixed there 2026-10-17 — clients of
any version see the corrected badges; the fixed `tuna` wording ships here.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
