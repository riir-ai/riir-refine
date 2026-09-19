# The Replay Lane — `cargo heal replay` (Max tier's earn class)

> **Purpose:** the operator + contributor doc for the replay-endorsement
> lane — what the command does, what it earns (honestly: nothing yet),
> what it refuses, and why. The design argument lives in
> [Proposal 015](../../.proposals/015_replay_endorsement_reward_class.md);
> the server half landed as riir-dapps Plan 029 (ledger surface + the
> `/mining/replay/*` worker routes); the client half (this page's subject)
> landed as [Plan 127](../../.plans/127_p015_phase2_replay_client.md).
> The tier framing lives in [`node_tiers.md`](node_tiers.md) §Max.

## What it is

The Max tier's earn lane: a verifier leases third-party heal rows from the
service, **replays them locally** (the real clippy oracle in a scratch
project — the same instrument the score-bench uses), and submits a signed
verdict. K=2 verifiers from distinct accounts agreeing on a row's verdict
vector confirm it; confirmed rows split the row's reward weight between
the miner and the verifiers (`replay_cut_bp`, genesis **0** — see
"Honest status" below).

The lane is the P005 §Mining C1→C5 escalation slot filled with economics:
TrueBit-style canaries (known-bad rows at a secret rate ≤ 5‰) catch
work-skipping verifiers; submitter exclusion + server-side lease
assignment bound self-pairing; the verdict commitment rides
BLAKE3-canonical bytes both sides compile from ONE definition (the
riir-kat protocol leaf — the Issue-082 law).

## The command

```sh
cargo heal replay
```

One command = one lease (≤ 16 items, server-assigned) → local replay of
every item → one signed verdict per item → a summary line. Exit 0 iff at
least one verdict was accepted and none failed.

**Feature gates:** `kat_transport` + `clippy_verify` (+ the bin's
`latent_retrieval`/`rust_perf`/`clippy_lints`/`kat_service` base). The
shipped release set (default + latent_retrieval + kat_service +
kat_transport + fix_verify) does **not** carry `clippy_verify`, so the
release binary degrades with the exact remedy line — the verdict is
measured by the real clippy oracle, and a build without it cannot honestly
judge. A bespoke `--features kat_transport,clippy_verify` build exercises
the lane (the `nonergodic_driver` precedent). Promoting `clippy_verify`
into the release set is a Phase-3 owner call that ships with the lane's
activation.

## The gates, in order

1. **Kill switch** — `RIIR_HEAL_REPLAY=0` disables the lane (exit 0, one
   line; the `RIIR_HEAL_KAT_METER` idiom).
2. **Login** — load-only (the wetrun precedent: a SIGNING command never
   silently mints an identity; run `cargo heal login` first).
3. **Mining consent** — `OptedOut` refuses with the `--mine` remedy
   (verdicts are contributions to the same economy the sync push feeds);
   `Unset` proceeds — running the command IS the consent (the sync
   precedent).
4. **Toolchain fingerprint** (T2.2) — `blake3(rustc -vV stdout)`. The
   lease pins the fingerprint every verdict must be computed under; a
   local mismatch aborts BEFORE any compile (a skewed verdict is refused
   unscored server-side — running it would be pure waste). The
   fingerprint pins the toolchain BUILD (version + commit + host):
   cross-platform disagreement is an expected split, never a slash
   (P015 caveat 2). Operator-derivable: `rustc -vV | blake3`.

## What one item's replay does

- **Step 0 — the canary check** (before any compile): recompute
  `blake3(fix)` and compare against the lease's pinned `fix_hash`. A
  mismatch means the item is a canary → the verdict is `Tamper` (the
  catch pays a jackpot weight; stamping a canary as a vector revokes the
  account's replay standing for the epoch — the no-slashing TrueBit
  adaptation: a weight slash, never a balance confiscation).
- **The oracle replay** — ONE batched `cargo clippy` per item over
  `[shape, fix]` (two modules of a scratch crate; the oracle's honesty
  contract: any hard error falls back to per-snippet runs).
- **The pinned rate rule** (the determinism law — two honest verifiers
  must agree on the FULL 18-byte vector, rates included):
  - `compile_ok` = the FIX compiles.
  - `oracle_resolved` = shape compiles ∧ fix compiles ∧
    `lints(shape) \ lints(fix) ≠ ∅` — the fix STOPPED at least one lint
    the pre-fix span fired. The wire carries no `lint_key` (the
    stripped-vector design), so the REMOVED set is the observable
    evidence; ambient pedantic/nursery lints firing on both arms cancel
    in the diff by construction. A clean shape + clean fix reads
    UNRESOLVED (no evidence, never a vacuous pass).
  - `heal_rate_milli` = 1000 iff resolved; `created_rate_milli` = 1000
    iff the fix births lints the shape never had (the gen-8 shaped
    half-win: healed + created prices as exactly that).
- **Unmeasurable is not a verdict** — cargo missing / crate setup failure
  → the item is SKIPPED with the reason. Never submit what was not
  measured.

## Honest status (the P013 law)

- **Server surface:** shipped (riir-dapps Plan 029; G2 measured
  58 µs/verdict intake) and **ARMED on devnet since 2026-09-15**
  (riir-dapps Issue 091: `KAT_REPLAY_SECRET` + `KAT_REPLAY_TOOLCHAIN_HEX`
  set via wrangler secrets; the lease starvation fix `pull_open` landed the
  same day). Mainnet stays deliberately unarmed until the reward class
  activates. Apex/mainnet reads of the lane still 503 by design.
- **Client surface:** shipped (Plan 127) — gated behind `clippy_verify`
  builds as above.
- **Settlement: NONE.** `replay_cut_bp` is genesis 0 — verifier reward
  rows are never written at cut 0; an agreed FAIL verdict still rejects a
  row (honesty arriving early), but nothing pays. **Do not advertise
  replay earnings** until the owner ratifies a live cut (the decision
  brief is riir-dapps Bench 013 §T5; recommended v1 posture = keep 0).
- **Arming constraint (found in review 2026-09-19 — riir-dapps Issue 098 +
  the Bench 013 addendum):** the ratification memo's canary step
  ("canary 10%") is **20× above the shipped public bound**
  (`REPLAY_CANARY_RATE_BOUND_PERMYRIAD = 50` = 0.5%, enforced at config
  validation — the "≤ 5‰" rate above). Raising the bound is part of the
  same owner act, and by the memo's own F4 arithmetic arming under the
  current bound is unsafe at every rate it permits (per-lease survival
  ≥ ~92% at ≤0.5%, vs the 2% rung already failing the ≤5% zero-work-capture
  tolerance at ≥30% adversary share).
- **What "credit gates on structural soundness" means, precisely** (the
  residual owner-gated with the cut): `plan_proofs` refuses an
  empty-or-no-op fix, and the sole shipped verifier `C1BootstrapVerifier`
  checks only that the span/fix hashes are nonzero and distinct, then
  **ratifies the submitter's own `verdict_vector`** (its doc comment says
  exactly that). No compile gate, no oracle, one replica — the service
  never re-runs the fix before credit; the real oracle exists only on
  THIS side (the verifier rig). That near-ungated state is the exact
  content of the credit-gating question.

## Repeatable e2e — `scripts/replay_e2e.sh`

The 2026-09-15 manual two-verifier run, productized (riir-dapps Issue 091's
"setup a mode"):

```sh
./scripts/replay_e2e.sh
```

Two identities × one `cargo heal replay` cycle each against devnet:
verifier A = the default account dir (the fleet rig's identity), verifier
B = a FRESH throwaway identity (ssh-keygen + `login --key` into a temp
account dir — never the machine-SSH chain, which would derive the fleet
account again, and one account cannot counter-sign its own verdict
twice). Builds the oracle feature set
(`latent_retrieval,kat_service,kat_transport,fix_verify,clippy_verify`)
itself; refuses non-devnet URLs by default. Verdict ladder: PASS
(exercised) needs a K=2 confirmation line; PASS (idempotent) = all
duplicates (write-once held); EMPTY (exit 3) = both leases empty — push a
real mining batch and re-run; everything else FAIL. Logs land in a
`mktemp` work dir, kept and printed for forensics.

First run (2026-09-15): A = 16 duplicates (this morning's verdicts,
write-once held), B = 16 stored + **4 fresh K=2 confirmations** — the full
lane (lease → oracle → signed submit → confirmation) green in ~30 s.

## What is deliberately NOT here

No stake requirement (the class is stakeless — the work is the Sybil
cost), no balance slashing (weight slash only), no LLM anywhere in the
replay path, no service-side execution of untrusted fix text in ANY phase
(the verifier's rig executes, inside the scratch-project discipline).

## See also

- [`node_tiers.md`](node_tiers.md) — the Max tier this lane earns for
- [Proposal 015](../../.proposals/015_replay_endorsement_reward_class.md)
  — the design + honest caveats (binding)
- riir-dapps `.docs/` Plan 029 records — the server half
- riir-kat `kat_protocol_replay` — the wire composers (goldens pinned)
