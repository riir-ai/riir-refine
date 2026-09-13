# Issue 104 — RETRACTED: the "remote develop force-rewind" was a false alarm — my own ancestry misread; no rewind ever happened; no session did anything wrong

**Status:** CLOSED (RETRACTED 2026-09-14 ~01:4x +07). The incident this file
originally alleged DID NOT OCCUR. Remote `develop` was never rewound; the
P20/P13 evening wave (`4f17cff5`, `23134993`, …) has been intact and
linear the entire time. The 4090 session (and every session) pushed only
normal fast-forwards. This file survives as the post-mortem of a FALSE
diagnosis — the failure mode is real and worth recording. The backup
branch has been deleted; nothing about any repo's history needs repair.

## What actually happened (the true timeline, all +07)

- 00:37 / 00:59 — P20 (`4f17cff5`) then P13 (`23134993`) pushed, normal FF.
- 01:09:08 — a session pushed `e3e497c4` ("clippy raw-string hygiene in
  config_audit fixtures"): **a single NEW commit whose parent is `23134993`**.
  Normal FF. This is the commit I later misread as "an old pre-wave commit".
- 01:21:56 — the same lane pushed `29d30bfd` (self-audit docs). Normal FF.
- 01:2x — THIS session, misreading the above as a 6-commit force-rewind,
  pushed a backup branch + an accusation commit (`f8020f49`). Both wrong.
- ~01:4x — this retraction. `git log --oneline origin/develop` shows the
  full intact chain: `f8020f49 → 29d30bfd → e3e497c4 → 23134993 → …`.

## The two method errors that manufactured the incident

1. **`git ls-remote` lists ref TIPS, not ancestry.** My "confirmation" —
   grepping ls-remote output for `23134993`/`4f17cff5` and finding nothing —
   proved only that they were not ref tips (true of every non-tip commit in
   a healthy history). I reported it as "on NO remote ref ⇒ the commits are
   GONE". Wrong: they were (and always had been) ancestors of `develop`.
2. **The GitHub events `before`/`after` fields are old-tip → new-tip.**
   `before=231349933 after=e3e497c4a` is FORWARD motion. I had already
   decided a rewind occurred (from error 1), so I read an unfamiliar commit
   id in `after` as "older" — without ever running `git log`/`merge-base`
   on it. I even "recognized" `e3e497c4` as "a ~09-13 daytime commit" from
   its message text alone. Anchoring: every subsequent observation (the
   4090's checkout state, its live processes, its "rejection→force habit")
   was interpreted through the false prior and each felt like confirmation.

## Withdrawals (explicit)

- "Attribution CONFIRMED: a sibling session on the 4090 force-pushed" —
  WITHDRAWN. No force-push occurred. The 4090 lane's pushes (`e3e497c4`,
  `29d30bfd`) were clean fast-forwards of legitimate units.
- "rejection→force habit" — WITHDRAWN. Invented from the false prior.
- The backup branch `backup/2026-09-14-develop-23134993` — deleted
  (it duplicated an intact chain).
- The guardrail advice in the broadcast commit ("never force-push on
  rejection") remains generically sound but was attached to a fabricated
  incident; this retraction supersedes it.

## The durable lesson (why this file survives)

**Verify ancestry before declaring history loss.** The one-command test
that would have prevented all of this:

```
git merge-base --is-ancestor <allegedly-lost-commit> origin/<branch> && echo INTACT
```

or simply `git log --oneline origin/<branch> | grep <sha>`. An unfamiliar
commit id in a push event is a NEW commit until `merge-base` says otherwise
— commit-message vibes ("lib 1353/1353" sounding like old numbers) are not
ancestry evidence. This is the same class as the workspace's other
canonical failures: a confident wrong instrument reading that every
subsequent observation gets filtered through.

Filed + retracted 2026-09-14 by the same idle-loop session, ~40 minutes
apart. Apologies to the 4090 lane — your work was correct throughout.
