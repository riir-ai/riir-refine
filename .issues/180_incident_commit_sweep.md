# Incident note — the 179-claim-COMMIT sweep (2026-10-27, amo-auth-followup)

> Renumbered 179 → 180 (zed-amo-idle, 2026-10-27): the number was dual-held
> with `.issues/179_kat_account_provider_identity_adoption.md` (which keeps
> it — inbound citations in HISTORY.md). The "179-claim" in the title refers
> to the removed claim record `.issues/179_claim_now_half.md`, not this
> file's number.

**What happened:** committing `.issues/179_claim_now_half.md` with a bare
`git commit` (no path re-check) swept EVERYTHING staged-or-tracked-dirty
into `7f965bc3` on `origin/develop`: the 178 Phase A session's
uncommitted work (`.plans/299_meta_blueprint_phase_a.md`,
`src/blueprint.rs`, `src/blueprint_tests.rs`, the `src/bin/heal.rs`
blueprint wiring hunks, the AGENTS.md `--blueprint-report` row, the
proposal 024 status flip, AND their worktree deletion of
`.issues/178_healer_meta_consumer_blueprint.md`) landed under MY commit
message, minutes before their own landing.

**Why it happened (the root cause, not the blame):** the shared worktree
carries BOTH sessions' dirty files. My earlier commits in this session
used `git add <named files> && git commit` — but this one used a bare
`git commit` after a long edit chain (and my `git stash push/pop` cycles
mid-session had churned the index state). A bare commit in a shared
worktree is indistinguishable, to git, from intent — the exact
`staged_set_audit` lesson; I skipped the pre-commit census on the LAST
commit because the FIRST three were clean.

**Damage assessment (measured):**
- The sibling's WORKTREE was never touched (worktree state is what their
  session reads) — their flow continues unbroken.
- Their in-flight work is now PUBLIC under my message hash `7f965bc3`,
  including the premature deletion of the 178 issue file on origin.
- Their landing commit supersedes the swept content (same tree content,
  their message, their session marker); `git` history keeps both.
- Repair options measured: force-push = rewrites published history;
  follow-up commit = cannot remove in-tree content without re-deleting
  their files (worse). **Verdict (owner call, this session): leave
  origin as-is, note the incident** — the non-destructive path.

**The rule this incident re-arms for every future session:** in a shared
worktree, EVERY commit stages NAMED files and runs
`scripts/staged_set_audit.py` first — a clean streak is not a census.

Session: amo-auth-followup, 1795800000
