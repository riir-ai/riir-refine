# Issue 184 — the migration lane survives a dead owner: autonomous close + the owner-dormancy succession rule

**Status:** OPEN — verdict REVISE (2026-10-27) → DECIDED: **B′** (below)
supersedes B1–B5; implementation next. Owner-directed 2026-10-27 ("who
will check the 👍? worker? cranker? we need to automate that — think I'm dead
and only auto dao is running"). Follows [Issue 183](183_migration_release_requires_owner_endorsement.md)
(owner 👍 REQUIRED). Two halves: **A** (the closer) is implementation-only
and in flight; **B** (succession) changes who can authorize a release and
needs the verdict before code.

## The two liveness holes Issue 183 left

| # | Hole | Effect if the owner is dead |
|---|---|---|
| A | The close is LAZY — `POST /account/migrate/close` or a status read triggers it; kat-service has NO cron | nothing ever closes; rows sit pending forever, the one-pending slot is held, nobody's migration finishes |
| B | Release REQUIRES `owner_login`'s 👍 (structural, R5) | every migration cancels at window close, forever — fail-SAFE but the lane is dead; R7's escape (the dao re-pins `owner_login`) needs a governance act that, in the "only auto dao" world, nobody performs |

## A. The autonomous closer (implementation — no verdict needed)

A Cloudflare cron on kat-service itself (`0 * * * *`, apex + devnet) runs the
SAME close engine for every live row whose window elapsed. The worker
already owns the verdict ("the worker decides when"); the cron only removes
the dependence on a human/external poke. Zero due rows ⇒ no GitHub call, no
write (CF billing). Bounded closes per tick. Read faults leave the row for
the next tick. Status: in flight (riir-dapps, the `kat_migrate` feature).

## B. The succession rule (design — the verdict question)

**Principle unchanged:** GitHub is the root of trust; a silent auto-release
(nobody GitHub-attested acted) must stay impossible. What changes: WHO the
required GitHub attestation can come from when the owner is gone.

**B1. Owner dormancy is MEASURED, never declared.** At close, the reader
fetches `owner_login`'s public GitHub events (`GET /users/{owner}/events/public`,
first page) and records `owner_last_active_unix` (the newest event's
`created_at`; `None` if the list is empty) into the snapshot (digest-bound —
the verdict is recomputable). Owner is **dormant** ⟺ `close_unix −
owner_last_active_unix > OWNER_DORMANT_SECS` (proposed 30 d) or no public
event exists. An attacker cannot make the owner look dormant (they cannot
delete katopz's events) and cannot make a dead owner look active without
katopz's GitHub — which already beats the owner gate.

**B2. While the owner is ACTIVE — Issue 183 exactly** (owner 👍 required;
victim veto; weights). No change.

**B3. While the owner is DORMANT — the old identity self-endorses.** Release
⟺ proposal issue exists ∧ the OLD account's filing-time bound login
(`old_bound_login`) net-👍 ∧ that binding's `bound_at` is at least
`BINDING_MIN_AGE_SECS` (proposed 30 d) older than the filing (frozen at
filing beside the login) ∧ weighted 👍 > weighted 👎 (the two-human out-vote
still cancels). No bound login, or a binding younger than the floor ⇒
Cancel (the T9 guardian lane remains the answer for a holder with no
GitHub). An owner 👎 still cancels in either mode (a dormant-looking owner
who returns can always stop a migration).

**Why this is the right autonomous root:** the honest dormant-owner case is
"I lost my key; my GitHub account still exists" — the bound GitHub login IS
the holder's second factor, already PoW-gated at binding time. The thief
case needs the old key AND (the victim's GitHub OR a binding they created
≥ 30 d before filing — the rebind-before-filing attack from 183's residual
risk, now closed by the age floor, which 183 itself did not need while the
owner 👍 gated everything).

**B4. Explicit heirs (optional, owner-time).** The dao artifact may pin an
ordered `heir_logins` list; in dormant mode an heir's 👍 substitutes for the
bound-login self-endorsement. Empty by default — the lane works with no
heirs. (Question for the verdict: include B4 now or defer it?)

**B5. Kernel shape.** Snapshot gains `owner_last_active_unix: Option<u64>`,
`old_binding_bound_at_unix: Option<u64>`, `close_unix` (or reuse
`fetched_at_unix`); the domain tag bumps v2 → v3; `TallyReason` gains
`OwnerDormantSelfEndorsed` / `OwnerDormantNoAttestation` /
`BindingTooYoung`. The constants are LAW in the dao artifact (v3), never
runtime knobs (183 R5). Row: `old_binding_bound_at_unix` frozen at filing
(binding row value is already `acct16 ‖ bound_at u64` — `ledger.rs:193`).

## B′ — THE DECISION (supersedes B1–B5; verdict round 1's "simpler alternative")

**No dormancy detection at all.** Two release paths through ONE kernel:

| Path | Releases when | Window |
|---|---|---|
| **Owner** (Issue 183, unchanged) | owner net-👍 ∧ no victim veto ∧ weighted 👍 > 👎 | closes at 24 h |
| **Self** (new) | the OLD account's filing-time bound GitHub identity (NUMERIC id, release-created `kat:mgithub:` source only, binding ≥ 30 d old AT FILING) net-👍 ∧ owner did NOT net-👎 ∧ fewer than 2 *qualified* 👎 (distinct non-owner, non-self numeric ids whose GitHub account is ≥ 365 d old at filing) | releases no earlier than filing + 7 d |

At the 24 h close: owner path passes → **Release**; else the self path is
eligible-so-far → **Hold** (the request stays pending until filing + 7 d,
then re-judged); else **Cancel**. The cron closer (A) re-judges held rows —
no human anywhere. An owner 👎 cancels on either path at any close.

**Why this over B1–B5:** the owner's own automation pushes as `katopz`
daily, so "owner dormant" would never fire in the only-auto-DAO world (MUST 1);
dormancy reads fail both ways (MUST 2). B′ never releases without a
GitHub-attested human act: the owner, or the victim's own ≥30-day-bound
GitHub identity with 7 days of owner silence. A thief needs the old key AND
the victim's GitHub AND 7 days unvetoed. **Policy delta vs 183, stated:**
while the owner is alive, the self path turns the owner's REQUIRED 👍 into
a 7-day VETO for holders who self-endorse from their bound GitHub; every
issue body says so (`@owner` mentioned — a 👎 stops it).

**Folded MUST-fixes:** (3) identity is the GitHub NUMERIC id — reactions
carry `user.id`; `TallyWeights` pins `owner_id` beside `owner_login`; the
binding row grows `acct16 ‖ bound_at ‖ gh_id` (legacy 24 B rows → no id →
no self path, the `NameOnlyBinding` precedent); the filing freezes
`old_bound_id` + `old_bound_at` in the row. (4) the grief floor: the
self-endorser weighs 5100 and only qualified (≥365 d) 👎 count on the self
path — two needed to cancel. (5) filed separately: riir-dapps Issue 142
(recovery lane, latent, blocks its mainnet promotion). (6) only the
release-created `kat:mgithub:` binding is a self root; `pbind` never.
**SHOULD-fixes folded:** issue body + ack + CLI state the active path
("owner 👍 releases at 24 h; or the bound GitHub @x 👍 releases at 7 d
unless the owner 👎"); dispute pairs judged on one read; v2 rows fail
closed on the self path. **Deferred:** B4 heirs (a global root — if ever,
dead-man succession on `owner_login` itself, revocable).

**Constants (dao artifact v3 LAW, never knobs):** `SELF_WINDOW_SECS` =
604 800 · `BINDING_MIN_AGE_SECS` = 2 592 000 · `VOTER_MIN_ACCOUNT_AGE_SECS`
= 31 536 000 · `SELF_ENDORSE_WEIGHT_BP` = 5100 · `QUALIFIED_OBJECTIONS_TO_CANCEL`
= 2 · `MAX_QUALIFY_LOOKUPS` = 100 · snapshot domain `kat:migrate:snapshot:v3\n`
· `TallyVerdict` gains `Hold`.

## Open questions for the verdict (round 1 — answered, see the log)

1. Is owner dormancy via the public events API sound (gameable? false-dormant
   when katopz works only in private repos — acceptable since B3 is still
   GitHub-attested)?
2. 30 d dormancy / 30 d binding-age floors — right order of magnitude?
3. B3 self-endorsement vs requiring heirs — is the bound login alone a strong
   enough autonomous root?
4. B4 heirs: now or defer?
5. Should dormant mode lengthen the window (e.g. 7 d) so the victim has
   more time to notice the @-mention?

## Acceptance

- [x] A (code): riir-dapps `024f80b` — `#[event(scheduled)]` hourly (apex +
      devnet) → `/internal/migrate/tick` in the ledger DO → the SAME
      `migrate_close_request` per due row; idle tick = no GitHub call, no
      write (fixture call-counter proven); ≤ 8 closes/tick (CF 50-subrequest
      plan cap), backlog drains over ticks; dispute rival-state report bug
      fixed in passing. 586 kat-service lib green. Apex build has no
      `kat_migrate` → its cron is a no-op until promotion.
- [ ] A (live): devnet redeploy carrying the cron (after the B′ chain lands —
      riir-kat is a path dep, a mid-edit build is unsound).
- [ ] A gap (→ the B′ worker slice): issue-less LIVE rows not yet due get a
      creation-only retry per tick — today a filing whose creation failed
      gets its issue only AT close, so the owner never sees it before the
      verdict reads it.
- [ ] B′ kernel (riir-kat): v3 snapshot, numeric ids, `Hold`, self path;
      E-cells S1 self-👍 held@24h→released@7d · S2 binding <30 d → cancel ·
      S3 legacy/no-id binding → cancel · S4 owner 👎 → cancel (both paths) ·
      S5 one qualified 👎 → still releases · S6 two qualified 👎 → cancel ·
      S7 two unqualified (<365 d) 👎 → still releases · 183 matrix green.
- [ ] B′ worker (riir-dapps): reaction ids + voter-age lookups (bounded,
      cached), binding row +gh_id, filing freezes id + bound_at, Hold
      re-judged by the cron; dao artifact v3; CLI/issue-body path text.

## Verdict log

- **Round 1 — REVISE (Claude, 2026-10-27):** 6 MUST — (1) the owner's
  own `katopz` automation keeps a dead owner "active" forever; (2) one-shot
  events-API dormancy is unsound (90 d window, private activity, CF-egress
  rate limits, suspension/404 — and empty-on-error ⇒ dormant fails open);
  (3) login STRINGS let a recycled username inherit the self/veto/owner
  authority → numeric ids; (4) one throwaway 👎 ties a 4900 self-endorser →
  grief-kills the lane; (5) the recovery lane (Issue 140) already releases
  silently → riir-dapps Issue 142; (6) only release-created bindings may be
  a self root. Code checks: `bound_at` exists but was not frozen at filing;
  bindings are written only by releases (`bound_at = now`, reset per move).
  Recommended the dormancy-free alternative → **adopted as B′** (owner call
  delegated to the verdict per the standing rule).

