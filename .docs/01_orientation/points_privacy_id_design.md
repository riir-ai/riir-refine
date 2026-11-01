# The points-plane privacy & client-ID design (the binding law)

**Status:** DECIDED as design law — T16.1 of issue 192; blocks the points implementation (Plan 325, Proposal 026 cite this note).
**Owner direction:** issue 192 GAP 1 (2026-10-31) — the derived-ID fingerprinting pattern is refused; a locally generated random client UUID is the law.

This note is the one home for the identifier, consent, and retention decisions of the
public-goods points plane. Proposal 026 (the wire) and Plan 325 (the CLI surface)
conform to it; where prose disagrees, this note wins until amended by the owner.

## 1. The client identifier — one law, four properties

Every install identifies itself with a **client UUID**: 128 bits of randomness from
the OS CSPRNG, rendered in the standard UUID shape, generated **locally**, stored in
the machine-local config, and **resettable by the user** at any time.

| Property | The law |
|---|---|
| **Local** | Generated on the machine from the OS CSPRNG (`getrandom`). Never derived, never fetched. |
| **Random** | No input but entropy. NOT uuid v7 — v7's embedded timestamp leaks the install/reset moment for zero benefit (there is no B-tree here; the server stores an opaque token). The house `Uuid::now_v7()` default is for entity keys, not secrets; this is the documented deviation. |
| **Resettable** | `cargo refine config reset-client-id` mints a fresh one instantly. Old aggregate rows remain (they are counts), but nothing new ever links to the old token. |
| **Blind** | The token identifies one INSTALL, never a person, never a machine. It carries no hardware, hostname, user, time, or network derivation — by construction, not by policy. |

## 2. Refused designs (with the reason each stays refused)

| Refused | Why |
|---|---|
| Unsalted hash of NIC MAC + hostname | Trivially reversible (small MAC space → rainbow tables; hostnames → dictionaries); stable across networks = a persistent cross-context tracker. **The pattern GAP 1 refuses.** |
| Any hash of any hardware serial or OS machine GUID | Same reversibility + the value is stable for the device's life — a tracker wearing a hash. |
| Advertising-ID-style cross-install persistence | Persists beyond reset; the whole point of reset is that it works. |
| IP-based identity for stats | Geolocates, churns, and punishes mobile/NAT users; not an identifier, a location. |
| Server-assigned IDs without client consent | The first contact must not mint tracking state; the client generates its own and only sends it under the consented paths below. |

The salted-hardware anti-abuse signal may EXIST only as designed in §4 — optional,
server-salted, disclosed — never as the primary identifier, never unsalted, never
silent.

## 3. What the client UUID is for, exactly

- **Anonymous aggregate counts** — machines-per-day, fixes-per-machine-class of
  stat, deduplicated without identity. Anonymous users keep full local function.
- **Rate sanity** — one token one vote in per-epoch contribution tallies; a reset
  simply re-enters the pool (accepted cost — the plane is reputation, not money).
- **Join binding** — `join` signs a binding row (client UUID ↔ GitHub identity ↔
  account key) so rank can be anti-Sybil without ever learning hardware. `leave`
  unbinds it server-side and local use continues anonymous.

It is NOT for: per-file attribution (the sync payload's redaction policy already
governs content), cross-repo profiling of a user, or any join across contexts the
user did not consent to.

## 4. The optional anti-abuse signal (the only hardware-shaped escape hatch)

If abuse ever requires a stronger signal than the token: the server salts a
client-supplied opaque hardware attestation with a **per-join server secret** that
never leaves the server, stores only the salted digest, and discloses the mechanism
in the consent text in plain words. Thresholds act on digests, never on the inputs.
Until that need is demonstrated, this paragraph is a fence, not a feature — nothing
in v1 sends any hardware-derived byte. Any implementation lands here first, with the
consent text updated in the same change.

## 5. Consent text (the exact words the UI shows)

First-run (once, on the first `fix` that could ever send anything):

> **Does `cargo refine` ever send my code anywhere?**
> Local healing never touches the network. Sharing is opt-in and secret-blind: the
> redaction layer never lets credentials or secrets leave your machine, and what is
> shared is fix metadata — rule verdicts and the code spans each fix touched — never
> whole files. Your install is identified by a random ID we generate on your machine;
> we never read your hardware, your hostname, or your network. You can share
> anonymously, join with GitHub to get contribution credit and a leaderboard rank, or
> run fully offline (`RIIR_REFINE_OFFLINE=1`). Leave any time — `cargo refine leave`
> — and the tool is identical locally either way.

The vocabulary law (issue 192 GAP 10) holds in every rendered variant: contribute /
contribution / contributor / contribution points — never mining/mine/miner.

## 6. Retention & derivation table

| Field | Derived from | Lives | Published? | Retention | Reset effect |
|---|---|---|---|---|---|
| `client_uuid` | OS CSPRNG only | machine config (`client_id`, 0600) | never verbatim; folded into server-side aggregates | server aggregates: epoch-scale counts only | old rows unlink; new token mints |
| GitHub id (joined only) | the OAuth `read:user` flow (existing device-flow app) | server join-binding row, signed by the account key | leaderboard handles, never raw ids | until `leave`; `leave` unbinds | n/a (join-scoped) |
| Account key signature | the user's ed25519 key (`~/.ssh`-backed account key) | client keyfile | signature over binding/rows only | until key rotation or `leave` | new key = new identity |
| Contribution rows | fix outcomes the user consented to share | server store (existing sync-payload lane) | corpus (MIT) after review + proven | permanent — **history is never erased by expiry** (verdict r1 GAP 4b: only the rank *weight* decays) | n/a |
| Rank weight | derived: decaying function of permanent contributions | derived at epoch recompute | leaderboard ranks only | recomputed per epoch | decays to zero; history unaffected |
| Published anchors | per-epoch `anchor_root` over ledger state | status/explorer JSON | **roots only** — no client UUIDs, no GitHub ids in any published anchor or stat (Proposal 025 privacy row) | permanent | n/a |

## 7. Feeds (what this note unblocks)

1. **The application's Intentional answers** — §5's consent text + §1's four
   properties are the on-page/plain-text answer to "what do you collect?" (the
   grant playbook §13.2 Intentional paragraph cites this note).
2. **Proposal 026 wire** — the `client_uuid` field, the join/leave binding rows,
   the anchor-published-roots-only rule.
3. **Plan 325 CLI surface** — the first-run consent UX, `join`/`leave` semantics,
   `RIIR_REFINE_OFFLINE=1`, and the sweep's positive assertions (the consent text
   above is asserted by the charitable gate once the surfaces ship).

## 8. Amendment rule

Any change to §1's four properties, §2's refusal list, §5's consent text, or §6's
table is an owner gate, landed in this note first, with the wire/CLI/consent surfaces
updated in the same change. The T14-family charitable sweep's positive assertions
pin the consent text verbatim — a silent drift reds the build.
