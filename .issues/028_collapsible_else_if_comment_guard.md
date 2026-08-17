# Issue 028 — `collapsible_else_if` proposer fires on comment-guarded else blocks (Issue 027 class, sibling proposer)

**Status:** OPEN
**Filed:** 2026-08-17
**Found by:** live heal sweep on katgpt-rs/katgpt-speculative (commit `e8cefad44` context)

## Summary

The `collapsible_else_if` proposer fires where real clippy 1.97 is silent:
when a comment sits between the `else {` open brace and the inner `if`, real
clippy skips the collapse (the merge would displace the comment). This is
**exactly the Issue 027 `collapsible_if` bug class**, but in the
`collapsible_else_if` proposer — which never received the comment-gap guard.

## Evidence (the katgpt-speculative sweep, 2026-08-17)

- `cargo heal --fix` (dry) proposed 44 edits; 8 were `collapsible_else_if`.
- Real clippy ground truth (`--all-features -W clippy::pedantic`): **ZERO**
  `collapsible_else_if` findings anywhere in the crate.
- All 8 proposed sites have a comment between `else {` and the inner `if`,
  e.g. `dd_tree/tree_builder.rs:287`:
  ```rust
  } else {
      // Original behavior: seed heap with root's children, filtered by pruner
      if config.vocab_size > 256 {
  ```
  and `spechop/hop_tree.rs:273` (`// ── Standard: seed depth-0 candidates ──`).
- One worse sub-case: `and_or_builder.rs:228` — the proposer fired on a
  **match arm** (`false => { if let ... }`), composing with the `match_bool`
  edit in the same pass; the composed edit **DELETED the comment**
  `// High relevance — close any open region.` outright (comment
  displacement into nothing). Caught in `git diff` review pre-commit; the
  hunk was hand-restored to the clippy-equivalent `match_bool`-only form.
- Post-fix clippy verification confirmed the 8 reverts were right: the
  collapsed shapes would NOT have been re-flagged (clippy stays silent on
  the comment-guarded originals).

## Fix shape

Port the Issue 027 T2 comment-gap guard from `collapsible_if` to
`collapsible_else_if`:

1. Scan the lead gap slice (between the outer `else {` and the inner `if`
   keyword) textually; if any comment text is present, skip the proposal.
2. Same for the trail gap (between inner `if` block close and outer `else`
   close).
3. Additionally: guard the match-arm composition path — the proposer must
   not fire on `match` arms at all (that shape only becomes an `else`
   block AFTER a `match_bool` edit composes; clippy never sees it as
   collapsible in one pass, and our emitter demonstrably drops the comment
   when it does compose).
4. Add buggy/clean fixtures: comment-guarded else-if (clean — no fire),
   bare else-if (buggy — fire), match-arm-with-inner-if (clean — no fire).

## Verification gate

- [ ] Proposer unit tests: 3 fixtures above (buggy fires / clean ×2 silent)
- [ ] `clean_spans_fire_no_lint` sample extended with the comment-guarded
      else-if shape
- [ ] Re-run the katgpt-speculative dry `--fix`: 0 `collapsible_else_if`
      proposals (the same 8 sites, previously 8)
- [ ] Existing corpus tests + retrieval floors unchanged (proposer change
      only, no corpus edit)

## Non-goals

- The 8 reverted sites in katgpt-speculative stay UNFIXED (clippy itself
  is silent on them; not our job to exceed clippy).
- Memory.json already cleaned of the 8 FP trajectories (Issue 027
  precedent) — no action needed there.
