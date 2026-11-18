# Issue 245 — `--help-all` names kat_service/points verbs in builds that lack the features

**Status:** OPEN — found 2026-11-18 while gating the generated agent skill (issue 244 follow-up, commit 0ffecf2ec).

## The wart

`usage_text()`'s verb block spells `cargo refine login / account / kat / tuna /
topup / buy / get / send / faucet / wetrun / replay / claim / stake / unstake /
vote / pay / refill / corpus-pr / totp / migrate` **unconditionally** — annotated
`(needs --features kat_service)` / `(kat_transport)` — so EVERY build's
`--help-all` lists commands its own binary refuses (a default or release-set
build compiles those verbs out; invoking one refuses `exit 2` naming the
rebuild). The user-facing `--help` already does this right: its
`account`/`points` fragments are `#[cfg(feature)]`-gated
(`src/bin/heal.rs` ~L7261–7283), so a charitable build prints no KAT lines at
all (the T14 output bar). `usage_text` predates that discipline.

## Why it matters

- The just-works UX contradicts itself: `--help` (correct) and `--help-all`
  (lists unbuilt verbs) disagree about what the binary can run.
- It leaked into the public agent skill's gate: the served SKILL.md deliberately
  omits the ops/money verbs (the zero-token law, 2026-11-18 — the dapps worker's
  `refine_product_surfaces_carry_no_kat_or_tuna` gate), so the skill gate must
  carry an explicit exemption list for exactly these verbs
  (`tests/agent_skill_md_fresh.rs` `OPS_EXEMPT`, pinned two-sided). Fixing the
  wart lets that list shrink to nothing.

## The fix

Gate the verb lines in `usage_text()` the way `user_help()` already does:
`#[cfg(feature = "kat_service")]` / `#[cfg(feature = "points")]` fragments (or
the same `let credit_help`-style pre-rendered strings). The usage block spans
`src/bin/heal.rs` ~L7365–7395 (`login/account/offline/online/off/on/update/
sync/claim/stake/vote/unstake/pay/topup/buy/kat/tuna/get|send/faucet/wetrun/
replay/config`); note `offline/online/off/on/update/sync/config` are UNGATED
(Plan 342 / points posture) and must stay unconditional — only the
kat_service/kat_transport-annotated rows move behind their gates.

## Acceptance

- A default-features `--help-all` prints no kat_service/kat_transport verbs.
- `cargo test --test agent_skill_md_fresh` still green (the OPS_EXEMPT list
  then deletes naturally — delete it in the same commit).
- The dapps worker's vendored skill copy is unaffected (its bytes do not name
  the ops verbs today).
