# Issue 184 — the migration lane survives a dead owner: autonomous close + the owner-dormancy succession rule

**Status:** OPEN — design, awaiting verdict. Owner-directed 2026-10-27 ("who
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

## Open questions for the verdict

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

- [ ] A: the cron closer lands + deployed to devnet; a due row closes with no
      human action (proven in-process and on the devnet rehearsal).
- [ ] B: verdict AGREE; kernel v3 + dao artifact v3 + worker reader + row
      field; E-cells: dormant+self-endorse released, dormant+young-binding
      cancelled, dormant+no-binding cancelled, dormant+owner-👎 cancelled,
      active owner unchanged (183 matrix green).

## Verdict log
