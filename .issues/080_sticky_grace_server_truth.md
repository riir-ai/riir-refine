# Issue 080 — the sticky-grace input is a locally forgeable file: swap the gate to server truth (burn-ack v3 `contributed`), retire `contributed_once.json`

**Status:** OPEN — filed 2026-09-09 (owner: "data should be in neuron-db as possible so no one can hack it and can be synced to system"); consumed by riir-neuron-db Plan 327 (substrate + sizing) and riir-dapps Issue 027 (server half)

## The finding

`kat_contribution::has_contributed` — the 075 soft gate's sticky-grace leg —
reads `~/.config/riir-heal/contributed_once.json` and resolves ANY present
content as CONTRIBUTED (anti-brick: corrupt reads permissive). That means:

1. It is NOT a dev/debug artifact — it is the live production gate input on
   every `kat_transport` build.
2. It is **trivially forgeable**: `echo '{}' > contributed_once.json` lifts
   the depleted-account refusal forever, on that machine, with no network,
   no key, no contribution. The anti-brick direction that protects
   contributors also arms free-riders.
3. It is single-machine, unreplicated, and unsigned — strictly worse than
   the server-side fact it shadows (the mining ingest ALREADY knows whether
   an account ever had an accepted batch).

## The fix (Plan 327 T3; server half = riir-dapps 027)

- Burn-ack v3 gains `contributed: Option<bool>` +
  `first_commitment_hex: Option<String>` — written ONLY from the burn-push
  response (the same authenticated channel that already carries
  `last_balance_micro`).
- Gate predicate: `contributed_and_synced = ack.contributed ==
  Some(true) || legacy_marker_present()` — **server truth wins**; the legacy
  marker only fills in while the server fleet is pre-v3, so the transition
  never bricks a real contributor and never widens the forge beyond today.
- After fleet v3: delete the legacy read and `kat_contribution`'s gate use
  (dated removal task — the file itself stays gone with it).
- Anti-brick preserved: a corrupt/missing ack reads empty → `NeverSynced` →
  warn-only (the permissive direction), no forgeable file needed.

## Why the gate reads the SERVICE answer and not a neuron-db shard directly

riir-clippy already deps `riir-neuron-db` under `latent_retrieval`, but the
gate input must stay single-source: the signed service answer. The registry
shard (Plan 327) is the SERVICE's durable, replicated, chain-anchored store
of the same fact — the client never reads it directly. This keeps the dep
edge clean (boundary-guard T8 in the plan).

## Tests (with T3)

- server truth `Some(false)` + planted legacy marker → REFUSED (the forge is
  closed — the regression test for this issue)
- `Some(true)` → allowed (sticky grace, unchanged)
- `None` (pre-v3 server) + marker → allowed (transition)
- `None` + no marker → NeverSynced warn-only (unchanged 075 shape)

## Docs (owner rule: docs follow the fix)

AGENTS.md CLI-contract soft-gate bullet + `kat_contribution` module docs
update in the SAME commit as T3; removal task updates them again when the
legacy read dies.
