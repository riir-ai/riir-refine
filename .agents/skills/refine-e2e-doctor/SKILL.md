---
name: refine-e2e-doctor
description: End-to-end health check of the cargo-refine product — binary, fixer lane, self-evolve trainer halves, KAT plane (sync/lease/balance), the tokenomics reconcile verdict and DAO-adjust suggestions, plus the opt-in mainnet-shape USDC→KAT pay rehearsal via surfpool. Produces a summary report with incidents and suggestions, and requests a Claude verdict for any owner-gated call. Use when asked whether refine works e2e, whether fixer or trainer work, for a periodic health sweep, before a release cut, or when asked for a tokenomics verdict or a DAO-adjust proposal.
---

# refine-e2e-doctor

You are checking the whole `cargo refine` product end to end — not just
compiling it. Every step below was executed and verified live (2026-10-13);
the traps are recorded in [steps.md](steps.md). Read steps.md for exact
commands, expected output shapes, and the failure folklore before running
anything unfamiliar.

## Two modes — pick by the ask

- **SWEEP** (the periodic health check, ~5 min): binary health, fixer
  lane, trainer halves, KAT plane, tokenomics reconcile, then the report.
  Default when someone asks "is refine working?".
- **RELEASE** (before a release cut or after a client/lane change):
  everything in SWEEP, plus the surfpool pay-lane rehearsal (§5),
  the update_check lane, this repo's own gates, and the devnet worker
  liveness read. ~25 min, networked, spends Helius fork credits.

**Disclosed production writes (both modes):** the scratch fix and `sync`
push real rows to the LIVE service — the synthetic shapes dedup
server-side (they earn nothing) and metering is ~µKAT per sweep while
TUNA trial credit lasts. Once the TUNA trial expires (dated watch item in
every report), the fixer lane meters real KAT — say so in the report.

A red check does NOT stop the sweep — the report needs the whole picture.

## SWEEP checks

1. **Binary health** — `cargo refine --version`: the `build:` hash must be
   FRESH for every build-relevant path (`git diff --quiet <build> HEAD --
   src Cargo.toml Cargo.lock build.rs` exits 0 — docs-only commits
   legitimately leave the binary behind bare HEAD; report `-dirty`
   separately; an unresolvable hash is UNKNOWN, not BEHIND); `release set:
   complete`; exactly one binary on PATH (`which -a cargo-refine` — a
   `/opt/homebrew/bin` hit is the Issue-150 shadow).
2. **Fixer lane** — scratch cargo project in `/tmp` with ONE known
   mechanical lint (`needless_return`), dry run (auto-oracle anchors a
   real clippy diagnostic), then `--fix --write --verify` with an
   isolated `CARGO_TARGET_DIR`: assert "kept N edit(s), reverted 0",
   rustfmt applied, the rewrite correct, and the mining batch pushed
   (the disclosed write). Remove the scratch afterwards.
3. **Trainer (self-evolve) halves** — memory half: the
   `.refine/trajectories.kv` WAL grows on every fix (THE pass condition;
   the row count only advances for a NEW shape — a fixed scratch shape
   dedups by design, which is not a failure). Learning half: the release
   binary prints `memory-only (learning disabled: built without the
   clippy_verify oracle)` and that is the DESIGNED posture
   (`recorder.rs::maybe_log_degradation` logs only when disabled); to
   prove the learning half live, build the dev posture with
   `clippy_verify` + `fix_verify` and re-run the scratch fix — assert
   `oracle: N confirmed resolved (Certified)`. Never ship `clippy_verify`
   in the release set (the `release_set_matches_workflow` test pins it).
4. **KAT plane** — `cargo refine --info`: balance positive, soft gate
   ALLOW, corpus lease current, mining rows sane (0 unproven). `cargo
   refine sync` drains the outbox — a persistently-unsynced batch with
   accepted pushes is a watch item, not an incident. TUNA trial expiry is
   a dated item for the report.
5. **Tokenomics verdict** — `../riir-dapps/scripts/tokenomics_reconcile.sh`
   (both envs; public reads, cheap): seven sources per env. All-agree =
   verdict GREEN; any `DEPLOY PENDING or DRIFT` line = a finding with the
   disagreeing sources named. Read the governance posture from
   `../riir-dapps/src/kat/governance.rs` (the enforcement point) and
   `../riir-dao/crates/riir-dao/src/guard.rs` (`LeverBounds`) — the SAME
   table is pinned both sides; a drift between them is always an
   incident.

## RELEASE-only checks

6. **Pay-lane e2e (USDC→KAT, mainnet shape)** — the surfpool rehearsal:
   `../riir-dapps/cloudflare/kat-service/test_surfpool.sh`. The script
   takes a pid-stamped lock and REFUSES when another run is live (the
   lanes collide on fixed ports); it discloses every pid it clears.
   Lanes: S1 deposit + S4 webhook on a mainnet fork (real USDC mint);
   S2/S3 pay + ATA on a dedicated OFFLINE simnet (the pay lane pins the
   Circle devnet USDC mint, squatted on mainnet — the mint is fabricated
   at its exact address, and fabrication only executes offline);
   **S6 = the headless CLI checkout** — `cargo refine login --key` +
   `topup --pack 1` + on-fork payment + settle + `account --history`
   renders the purchase, with the identity asserted POSITIVELY against
   the imported buyer key. Pass line:
   `T3 + S1 + S2 + S3 + S4 + S6 rehearsal PASSED — SURFPOOL-GREEN`.
   The CLI `topup` command is the shipped client (plan 045 B7); if S6
   fails while the wire lanes pass, the defect is client-side — BUT if
   check 1 was red (stale binary), S6's result is UNSEEN for HEAD: point
   `CARGO_REFINE_BIN` at a fresh build and re-run before triaging.
7. **update_check lane** — the release set carries it; `--version` runs
   the release check after its own block and `--info` shows
   `release: current (installed <v>, cache fresh)` from the cache. A
   `release set: STALE` line anywhere is a release-cut blocker.
8. **This repo's own gates** — `cargo clippy --workspace --all-targets`,
   `cargo test --lib`, and the `release_set_matches_workflow` test; run
   the `boundary-guard` skill for the repo contract. A release cut lands
   only from a green tree.
9. **Devnet worker liveness** — `curl -sf https://devnet.gist.rs/health`
   and `GET /pay/catalog` (armed posture) render; front-page changes only
   go live via the worker deploy, so a lapsed deploy is a release blocker
   to name, not silently absorb.

## Tokenomics verdict → DAO-adjust posture

The verdict (check 5) is REPORT-ONLY. Any adjust suggestion MUST cite the
lever it moves and stay inside its `LeverBounds` row (the ledger refuses
out-of-bounds params — the dao proposes, the ledger enforces). Name the
owner-gated acts explicitly (arming, rate changes, mint pins — e.g. the
pay lane's mainnet USDC mint is a D2 owner ceremony;
`KAT_PAY_RATE_KAT_PER_USDC` narrows to the pool-curve genesis anchor when
`kat_pay_pool` arms). Consuming a suggestion is the `riir-dao` lane's job
(Proposal 008); hand the report there and record the issue reference back.

## Owner-gated calls — Claude verdict

Per the standing rule, any owner-gated decision surfaced by this sweep
(arming, promotion, demotion, rate/mint/bounds changes) gets a Claude
verdict ping-pong before the report finalizes: present the measured
evidence and the proposed action, request `#Verdict: AGREE|REVISE`,
address every REVISE bullet with evidence, and record the final verdict +
decision in the report. Do not gate routine green checks behind this —
only genuine owner acts.

## The report

End with a summary report: (1) mode + what was checked and each check's
verdict; (2) incidents found (with reproduction line + the file/commit to
blame); (3) suggestions (each citing its lever/bounds or naming its owner
gate); (4) dated watch items (TUNA expiry, lease windows, soak
deadlines). File incidents as `.issues/` entries per the numbering
discipline when they need tracking; a transient observation stays in the
report. A sweep COMMITS ONLY its report and issue files — script/code
fixes discovered mid-sweep go through their own issue, never a drive-by
commit.

## Traps (full list in steps.md)

- `login` ADOPTS a key into a fresh account dir from THREE sources —
  `RIIR_REFINE_KAT_KEY`, the legacy-dir key, then the machine `~/.ssh`
  (`login_default` step 3) — the e2e buyer identity must come from
  `login --key <fresh ssh key>` (import bypasses the chain), and the
  rehearsal asserts the adopted pubkey EQUALS the imported one.
- The account id on the wire is the record's `account_id` head —
  `blake3(pubkey)[..16]` hex, 32 chars (`service_account_hex`).
- Loopback money intents are lawful via the dev-rig allowance
  (`kat_net_intent`: loopback → `ServiceNet::Local`); hosted hosts stay
  fail-closed. Never weaken that for a test.
- The pay lane's pinned mint is the Circle DEVNET USDC mint (owner-ratified);
  mainnet's real USDC is a D2 owner ceremony. The rehearsal fabricates the
  mint at its exact address on an OFFLINE simnet — a mainnet fork serves
  remote bytes for accounts a tx writes and the fabrication fails 0x4.
  (Measured: the Helius api-key never appears in surfpool's startup log.)
- Concurrent cargo on this box: isolate with `CARGO_TARGET_DIR=/tmp/...`;
  the dev-posture build needs `--features latent_retrieval,rust_perf,
  clippy_lints,kat_service` (+ `clippy_verify,fix_verify` for the trainer
  proof) — the bin has required-features and refuses without them.
- A `-1000 KAT` lease line during sync is the renewal price line; confirm
  with the follow-up `corpus lease: up to date ... no charge` and an
  unmoved burn watermark before calling it an incident.
- The rehearsal refuses on a live lock (`/tmp/kat-surfpool-e2e.lock`) —
  never delete another run's lock; stale (dead-pid) locks are taken over
  by atomic rename and the script itself. A millisecond window remains
  where two racers can both take over the SAME stale lock (A's `mv` can
  land on B's fresh lock); if that ever fires, the symptom is two runs
  colliding on ports — kill both and re-run one.
- The freshness probe's `git diff --quiet <build> HEAD` reports BEHIND on
  exit ≠ 0 — including when the hash is unresolvable (binary built
  elsewhere or history rewritten). Resolve the hash first
  (`git cat-file -e "$BUILD^{commit}"`) and call that case UNKNOWN.
