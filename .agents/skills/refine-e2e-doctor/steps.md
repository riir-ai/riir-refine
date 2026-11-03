# refine-e2e-doctor — the e2e steps

Every command below was executed and passed on 2026-10-13 (M3 Max, macOS,
`cargo-refine 0.3.1` @ `d04f7970`). Expected-output shapes are quoted from
that run. Times are upper bounds seen on a warm box.

Modes: §1–§4 + §6 = the **SWEEP** (~5 min). Everything incl. §5, §7–§9 =
**RELEASE** (~25 min, networked). §10 is the report + verdicts, always.

## 0. Conventions

- Work from `/Users/katopz/git/riir-refine` unless a step names a sibling.
- Sync git remotes first (standing rule): `git fetch origin && git
  --no-optional-locks status -sb` — note `behind/ahead` in the report.
- Any scratch build uses `CARGO_TARGET_DIR=/tmp/<name>_target` (concurrent
  cargo sessions share this box).
- Never `git add -A`; if you commit, stage named files only.

## 1. Binary health (~10 s)

```sh
~/.cargo/bin/cargo-refine --version
which -a cargo-refine
BUILD=$(~/.cargo/bin/cargo-refine --version | sed -n 's/^build: \([0-9a-f]*\).*/\1/p' | sed 's/-dirty//')
HEAD=$(git rev-parse --short=12 HEAD)
git diff --quiet "$BUILD" HEAD -- src Cargo.toml Cargo.lock build.rs && echo FRESH || echo BEHIND
```

PASS = `build:` == HEAD **or** `git diff --quiet <build> HEAD -- <build
paths>` exits 0 (the binary is fresh for every build-relevant path;
docs-only commits legitimately leave the binary behind bare HEAD — that
is FRESH, and `-dirty` is reported separately, never a failure alone).
Guard the unresolvable case first: `git cat-file -e "$BUILD^{commit}" 2>
/dev/null || echo UNKNOWN` — a hash missing from the local clone (built
elsewhere, history rewritten) is UNKNOWN, never BEHIND.
`release set: complete (latent_retrieval,kat_service,kat_transport,
fix_verify,update_check)`, `net: multi (default mainnet)`, and `which -a`
lists ONLY `~/.cargo/bin/cargo-refine`. A second path (esp.
`/opt/homebrew/bin/...`) = the Issue-150 shadow → call the binary by
absolute path and flag the rebuild in the report. **A red check 1
demotes §5's S6 to UNSEEN for HEAD** (or point `CARGO_REFINE_BIN` at a
fresh build and re-run).

## 2. Fixer lane (~2 min warm) — ONE known lint (`needless_return`)

```sh
rm -rf /tmp/refine_e2e && mkdir -p /tmp/refine_e2e/src
printf '[package]\nname = "refine_e2e_probe"\nversion = "0.1.0"\nedition = "2021"\n' > /tmp/refine_e2e/Cargo.toml
printf 'fn f() -> i32 {\n    let v: Vec<i32> = Vec::with_capacity(0);\n    return v.len() as i32;\n}\n\nfn main() {\n    println!("{}", f());\n}\n' > /tmp/refine_e2e/src/main.rs

# dry run — bare default IS the dry run, zero edits:
CARGO_TARGET_DIR=/tmp/refine_e2e_target ~/.cargo/bin/cargo-refine /tmp/refine_e2e | tail -12
# expect: "clippy oracle (auto): 1 real diagnostic(s) anchored", the
# needless_return span routed [oracle], "mining: batch pushed",
# the meter line (21 tokens → 0 TUNA while trial credit lasts)

# the real fix:
CARGO_TARGET_DIR=/tmp/refine_e2e_target ~/.cargo/bin/cargo-refine --fix --write --verify /tmp/refine_e2e | tail -12
cat /tmp/refine_e2e/src/main.rs   # `return` gone, tail expression, rustfmt clean

rm -rf /tmp/refine_e2e /tmp/refine_e2e_target
```

PASS = `verify: 2 cargo check(s) — kept 1 file(s) / 1 edit(s), reverted 0
edit(s)`, `fmt: formatted`, the fixed source correct, no `no corpus rule`
for `needless_return`. The `mining: batch pushed` line is a DISCLOSED
production write: the synthetic shapes dedup server-side (they earn
nothing) and metering is ~µKAT while TUNA trial credit lasts — after the
TUNA expiry date the same sweep meters real KAT (put the date in the
report's watch items).

## 3. Trainer halves (~2 min + optional 1 min dev build)

Memory half:

```sh
stat -f '%z %Sm' .refine/trajectories.kv/wal.log     # note size
# run ONE new-shape fix (the §2 scratch counts if you changed the shape)
~/.cargo/bin/cargo-refine --info | grep -E 'healed|unsynced'
stat -f '%z %Sm' .refine/trajectories.kv/wal.log     # size grew
```

PASS = **WAL bytes grew** (that is the whole pass condition — the fixed
scratch shape dedups after the first-ever run, so the ROW count stays
flat on a CORRECT system; assert row growth only when you deliberately
varied the shape, e.g. a different lint per sweep).

Learning half — the release binary's line

`self-evolve: memory-only (learning disabled: built without the
clippy_verify oracle)` is the DESIGNED disclosure (source:
`src/recorder.rs::maybe_log_degradation` prints ONLY when disabled).
Live proof of the learning half (dev posture, isolated target dir,
~1 min if warm):

```sh
CARGO_TARGET_DIR=/tmp/refine_verify_target cargo build \
  --features latent_retrieval,rust_perf,clippy_lints,kat_service,clippy_verify,fix_verify \
  --bin cargo-refine
CARGO_TARGET_DIR=/tmp/refine_e2e_target /tmp/refine_verify_target/debug/cargo-refine \
  --fix --write --verify /tmp/refine_e2e | grep -E 'verify:|oracle:'
# expect: verify: 2 cargo check(s) — kept 1 file(s) / 1 edit(s), reverted 0
#         oracle: 1 confirmed resolved (Certified), 0 persist (verified-fail), 0 Heuristic
rm -rf /tmp/refine_verify_target /tmp/refine_e2e_target
```

Network lanes SKIP loudly in the dev build (no kat_transport/update_check)
and the deltas stack for the next release-set `sync` — expected, not an
incident. NOTE the bin's `required-features`: without the four-feature
prefix the build refuses with
`target cargo-refine requires the features: ...`.

## 4. KAT plane (~1 min)

```sh
~/.cargo/bin/cargo-refine --info
~/.cargo/bin/cargo-refine sync | tail -6
```

PASS = balance > 0 and `soft gate: ALLOW`; `corpus lease: up to date
(epoch N, ...) — no charge` or a fresh renewal line; mining rows with
`0 unproven`; sync `push accepted`. WATCH (not fail): `unsynced: 1
outbox batch(es)` persisting across a sync whose push was accepted (seen
2026-10-13, steady-state, reconcile clean). The `corpus lease epoch N:
-1000 KAT` line is the renewal PRICE line — confirm the NEXT lease read
says `up to date ... no charge` and the burn watermark is unmoved before
calling it money lost.

## 5. Pay-lane e2e — surfpool rehearsal (RELEASE MODE, ~15-20 min warm, networked)

Spends Helius fork credits per run — opt-in for sweeps, mandatory for a
release cut. Prereqs: `surfpool >= 1.6` (`SURFPOOL_BIN` override),
`solana` + `spl-token` CLIs, `uv`, the Helius api-key inside
`../riir-dapps/.env` (`SOLANA_RPC_URL` — the fork datasource; the public
mainnet RPC rate-limits surfpool mid-run), free ports 8898/18899/18897/8902.
The script takes a pid-stamped lock (`/tmp/kat-surfpool-e2e.lock`) and
REFUSES while another run is live — never delete another run's lock; it
discloses every pid it clears (`clearing port N: pid P (cmd)`).

```sh
cd ../riir-dapps
./cloudflare/kat-service/test_surfpool.sh            # add --keep-state to debug
```

Lanes and pass lines:

- S1 — deposit (CEGEL) on the MAINNET FORK (real USDC mint `EPjFW…`):
  `S1 MAINNET-SHAPE e2e: pay → poll → settle, zero real money ✓`
- S4 — webhook fast path: `S4 WEBHOOK fast path ✓ (trigger → ... in 1 s)`
- S2/S3 — pay (USDC→KAT) + ATA on the OFFLINE simnet (the pay lane pins
  the Circle DEVNET USDC mint `4zMMC9…`, squatted on mainnet; the mint is
  `surfnet_setAccount`-fabricated at its exact address — a mainnet fork
  serves REMOTE bytes for accounts a tx writes, so the fabrication only
  executes offline, measured 0x4 on-fork):
  `S2 PAY lane ✓ (reference path + amount-index fallback, both settled)`
  plus the reconcile identities block (settled == evt rows; credited ==
  pack base × 10; display agrees; `/explorer/reconcile identity_ok`).
- **S6 — the headless CLI checkout (the shipped client under test):**
  `S6 headless CLI checkout ✓ (import login → topup intent → on-fork
  payment → settled → CLI history renders it)`
- The overall pass line names every lane:
  `T3 + S1 + S2 + S3 + S4 + S6 rehearsal PASSED — SURFPOOL-GREEN`.

What S6 does, precisely:

1. generates a FRESH OpenSSH key and `cargo refine login --key <fresh>`
   inside an isolated `RIIR_AUTH_ACCOUNT_DIR` (⚠ plain `login` ADOPTS a
   key from THREE sources in order — `RIIR_REFINE_KAT_KEY`, the
   legacy-dir key, the machine `~/.ssh/id_ed25519` —
   `login_default` step 3 — and would buy as the machine's owner
   identity; the phase unsets `RIIR_REFINE_KAT_KEY` in its env AND
   asserts the account pubkey EQUALS the imported buyer key — a positive
   assertion, so any adoption source regressing fails the guard);
2. `cargo refine topup --pack 1` (non-TTY, explicit pack — agents never
   guess a price) against the phase-B worker at `http://127.0.0.1:8898`;
   dialing loopback is lawful via the dev-rig allowance
   (`kat_net_intent::intended_net_for_url`: loopback →
   `ServiceNet::Local` → unattesting host allowed; hosted hosts stay
   fail-closed);
3. parses the `solana:` URL's `amount` (decimal USDC → base units ×10⁶)
   + `reference`, pays on-fork with the rehearsal's `send_payment`
   (real `transferChecked`; the payee key never signs);
4. settles via the crank poll (`POST /pay/poll`, bearer the e2e crank
   token) and asserts `/balance?acct=<account_id[..32]>` == `10000000`
   µKAT — the acct head is `blake3(pubkey)[..16]` hex
   (`kat_protocol_account::service_account_hex`, the HOUSE derivation);
5. runs `cargo refine account --history` and asserts the purchase page
   renders (`purchases (1, newest first)` + a `settled` row).

FAIL triage: S6 red while S2 green ⇒ client-side (the CLI, `riir-kat`
`kat_pay_client` / the `topup` command in `riir-refine/src/bin/heal.rs`)
— but FIRST check §1: a stale binary's S6 result is UNSEEN for HEAD;
set `CARGO_REFINE_BIN` to a fresh build and re-run before blaming code.
Any S-lane red ⇒ read the wrangler/surfpool logs (the fail line names
`$LOG`); stale ports from a killed earlier run are cleared by the script
itself — lock-guarded and pid-disclosed (`clearing port N: pid P`).

## 6. Tokenomics verdict (~30 s, both modes)

```sh
cd ../riir-dapps
./scripts/tokenomics_reconcile.sh            # both envs
```

PASS = `all seven sources agree (<env>) ✓` for devnet AND mainnet. The
seven sources: compiled constants, deployer RATIFIED genesis, live
/health, the /leaderboard rendered copy, /history shape sanity, the burn
identity, the pay-lane purchase identities (+ treasury float ≥ lane
floor). Mainnet at genesis zeros with the pay lane unarmed is an HONEST
posture (`pay lane NOT ARMED on this env` — disclosed, not skipped
quietly). Any `DEPLOY PENDING or DRIFT` line = finding; name the two
disagreeing sources.

Governance read for the verdict: `../riir-dapps/src/kat/governance.rs`
(enforcement, `LeverBound` table incl. the unit-econ advisory rows) ↔
`../riir-dao/crates/riir-dao/src/guard.rs` (`LeverBounds` — the SAME
numbers pinned both sides by paired tests; do not "tune" either side).
The armed split (65/5/20/10) and the three v2 levers ride
`EpochParams` (Proposal 010 Phase 4, riir-dao Bench 003).
Measured 2026-10-13: the Helius api-key NEVER appears in surfpool's
startup log (booted with the datasource set, log grepped — 0 occurrences
of the key or the words `api-key`/`helius`), so the kept `$LOG` on a
failed run does not carry the secret.

## 7. update_check lane (RELEASE mode, ~10 s)

The release set carries `update_check`; prove the lane compiles and the
release verdict is current:

```sh
~/.cargo/bin/cargo-refine --version | tail -3   # no `release set: STALE` line
~/.cargo/bin/cargo-refine --info | grep 'release:'
# expect: release: current (installed 0.3.1, cache fresh)
```

PASS = no STALE line + `release: current`. `release set: STALE — missing
...` anywhere is a RELEASE-CUT BLOCKER (the rebuild command is printed
with it).

## 8. This repo's own gates (RELEASE mode, ~5-10 min)

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --lib
cargo test release_set_matches_workflow
```

Plus a `boundary-guard` skill run for the repo contract (dep allowlist,
BOUNDARY.md drift). A release cut lands only from a green tree — a red
gate here is a blocker, not a note.

## 9. Devnet worker liveness (RELEASE mode, ~5 s)

```sh
curl -sf https://devnet.gist.rs/health | python3 -m json.tool | head -6
curl -sf https://devnet.gist.rs/pay/catalog | python3 -c 'import json,sys; v=json.load(sys.stdin); print("armed:", v.get("armed"), "rate:", v.get("rate_kat_per_usdc"))'
```

PASS = health 200 JSON + the catalog renders `armed: True rate: 10`
(the pool-curve anchor while `kat_pay_pool` is un-armed). Front-page /
render changes only go live via the worker deploy — a stale deploy is a
blocker to name in the report, never silently absorbed.

## 10. Report + verdicts

1. Write the report: check verdicts, incidents (repro + blame),
   suggestions (each citing lever + bounds or naming the owner gate),
   dated watch items.
2. File tracked incidents as `.issues/NNN_*.md` (read `.highwater`,
   allocate +1, write back; status line second).
3. Owner-gated actions (arming `kat_pay_pool`, the D2 mainnet-USDC
   ceremony, rate/genesis changes, bounds edits): request the Claude
   verdict ping-pong with the evidence, record the verdict in the report,
   execute only the owner's decision.
4. A sweep COMMITS ONLY its own report and issue files. Script/code
   fixes discovered mid-sweep (a red check whose root cause is a product
   or script defect) go through their own `.issues/` entry — never a
   drive-by commit from the health check. The ONE exception: the sweep
   doc/skill files themselves when the owner asked for the sweep's
   process to be updated.
