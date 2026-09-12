# Issue 096 — the two standing `Issue 665` CROSS rows resolve to katgpt-rs; the bench-file twin resolves to riir-ai

**Status:** RESOLVED 2026-09-12 — both CROSS rows qualified (2 -> 0); the
hyphenated + skill + research + bench surfaces repaired in the same commit.
Workspace citation CROSS is now **0 across all 19 repos**.

## The finding

katgpt-rs `scripts/citation_drift_floors.txt` pinned this repo's 2 standing
CROSS rows as "needing the AUTHOR rather than a lookup" — `Issue 665` names
"a class no repo's 665 is about". The author (the 2026-08-17 session) is
unavailable, so the referent was recovered by **archaeology over commit
history** — which settles it as a verdict, not a guess:

## The class namesake is katgpt-rs Issue 665 — proven

- The class-naming lesson landed `1c103568b` **2026-08-17 07:55** ("docs:
  queue-item dual-allocation lesson (Issue 665 collision, yield protocol)").
- At that moment the ONLY live 665 in the workspace was **katgpt-rs's**
  `665` — freedom-of-function/extension-count selection: filed `e7365f94`
  08-16, T1-T4 landed `96d01e91` 08-17, resolved+removed `3b8ed639` 08-17.
- **riir-ai's 665** (speculative_decoding_metal_decode) died 08-15 — TWO
  DAYS BEFORE the class was named; it cannot be the namesake.
- Corroboration: riir-ai `8bc3f65d2` (08-17 08:11, 16 min after the lesson)
  "feat(665): freedom-guided best-of-K defend-wrong PoC **(katgpt-rs
  96d01e91)**" — sessions were working katgpt-rs's 665 from the riir-ai
  tree that same morning. The collision was ON that issue's work.

So "the Issue 665 class" = the dual-allocation collision class named after
**katgpt-rs Issue 665**. The floors prose's "no repo's 665 is about [it]"
read the citation backwards: the issue wasn't ABOUT collisions — the class
is NAMED after the issue the collision happened on.

## The bench-file twin is a DIFFERENT referent — riir-ai's 665

`.benchmarks/011_rebench_pre_plan005.md` cites "Issue 665 T1-T6, speculative
decoding substrate for Metal decode, 1.21× at 88% acceptance, landed
2026-08-14" — that is **riir-ai Issue 665** (proven: riir-ai commit
`8512cd711` "feat: speculative decoding substrate for Metal decode (Issue
665 T1-T6)" touching `.issues/665_speculative_decoding_metal_decode.md`).
Two bare `665`s with two different referents in one repo is exactly the
rebinding hazard the campaign exists to kill.

## Repairs (same commit)

- [x] `HISTORY.md` ×5 — the two CROSS rows (1950, 2848) + the hyphenated
      forms the sweep regex cannot see (707, 810, 4113) →
      "katgpt-rs Issue[- ]665"
- [x] `.agents/skills/riir-clippy/SKILL.md` — the discipline's own
      "(Issue 665, 2026-08-17)" → katgpt-rs-qualified
- [x] `.research/108` — the 4th-occurrence row → katgpt-rs-qualified
- [x] `.benchmarks/011` ×6 — "riir-ai Issue 665" (out of sweep scope, same
      defect class; referent proven above)
- [x] katgpt-rs floors ratchet: this repo `max_cross` 2 -> 0 (with
      riir-game-sdk 1 -> 0 + the owed seal-remake 16 -> 0 — its 09:18
      repair `92dff5ca` was never re-pinned)

## Validation

- [x] `grep 'Issue[- ]665' HISTORY.md | grep -v 'katgpt-rs Issue'` → 0
- [x] no double-qualifiers introduced (`riir-ai riir-ai` / dup check) → 0
- [x] workspace sweep after all three repos land: CROSS 3 -> 0 over 19 repos
