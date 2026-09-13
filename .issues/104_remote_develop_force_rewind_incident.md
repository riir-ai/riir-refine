# Issue 104 — remote `develop` force-rewound 6 commits (23134993 → e3e497c4) at 2026-09-14 01:09 +07; work preserved on backup branch; owner adjudication owed

**Status:** OPEN — attribution CONFIRMED 01:2x (see §Attribution update): a sibling session on the 4090's stale checkout force-pushed, then landed its own unit (`29d30bfd`) on the rewound base and is MID-FLIGHT on the next (scope_hint). Preservation DONE (backup branch). Repair plan: broadcast-on-idle → reconcile-rebase — NOT while that session is live (a push from here mid-unit would trigger its rejection→force habit and wipe the broadcast unread). Until then: no `develop` writes from the M3; do NOT reset the M3 checkout; do NOT build/tag/release from the rewound base as HEAD-of-record.

## What happened (all times +07 local)

The remote `develop` of THIS repo was force-moved BACKWARD from `23134993`
(the P13 binding-association commit, pushed 00:59:55) to `e3e497c4` (a
~09-13 daytime commit, before the whole evening wave) at **2026-09-14
01:09:08** (GitHub event `2026-09-13T18:09:08Z`).

Rewound with it (now on NO remote branch except the backup):

| commit | what |
|---|---|
| `23134993` | P13 — cfg-audit binding-association mark (the .distill/001 P13 row's second half) |
| `4f17cff5` | P20 — post-heal cfg audit, per-edit `unverified(cfg)` marks |
| `e8571fb9` | HISTORY row for the `--version` exit-2 regression + rebuild protocol |
| `ad31a04a` | fix(cli): `--version` exit-2 regression (shared `standalone_mode` predicate) |
| `c134eb18` | clippy doc_markdown backtick hygiene |
| `51bdb80b` + `98628b65a`-era rows | docs(942) substrate note etc. (the full 6-commit span `e3e497c4..23134993`) |

Evidence (GitHub events API, authenticated `katopz`):

```
18:09:08Z PUSH before=231349933 after=e3e497c4a   ← the rewind
18:00:03Z PUSH before=4f17cff5  after=231349933   ← P13 landing (normal FF)
17:38:01Z PUSH before=e8571fb9  after=4f17cff5    ← P20 landing (normal FF)
```

Observed by this session: fetch at 01:01 still showed origin/develop =
`23134993`; `ls-remote` at 01:19 showed `e3e497c4`. No other repo was
touched (riir-train `bedcc670`, katgpt-rs `b5bd9ab7`, riir-ai `4b31367de`
all still on their remotes).

## Attribution update (01:2x — CONFIRMED, mechanism certain, intent clear)

- 01:21:56Z-bis: the SAME remote moved FORWARD `e3e497c4` → `29d30bfd` (a
  "self-audit unit — config-audit + orphan-report over riir-clippy" docs
  commit, author date 01:21:31 +07) — landed clean-FF ON the rewound base.
- On-box verification (ssh, 01:2x): the 4090 checkout's `develop` =
  `29d30bfd` (base `e3e497c4`), working tree DIRTY (`M src/bin/heal.rs`,
  `M src/lib.rs`, untracked `src/scope_hint.rs` — implementing the
  scan-scope hint its commit announced), 2× cargo.exe + rustc.exe LIVE.
- Mechanism: the 4090 session works from a stale checkout (last fetch
  pre-wave); its 01:09 push of that base was a force (rejection→force
  habit or explicit --force "sync"), which rewound remote `develop` past
  the evening wave; it does not know. It then landed its unit on the
  rewound base.
- Deliberate-owner-rollback hypothesis is DEAD: the subsequent clean unit
  on the rewound base matches ordinary sibling work (a cargo-heal-driven
  self-audit — `cargo-heal.exe` was the process seen at 01:16), and no owner
  note exists anywhere (the owner's rollback style is loud; cf. 916).
- The actor identity is NOT a blame finding — shared account, shared box,
  ordinary session shape. The finding is the FAILURE MODE: a stale
  checkout + force-on-rejection rewinds shared history silently.

## Preservation (done, 2026-09-14 ~01:25)

- Local M3 `develop` still at `23134993` (6 ahead of origin; the commits
  are safe — do not reset until adjudication).
- **`backup/2026-09-14-develop-23134993` pushed** — new branch, no force,
  nothing overwritten; contains the full rewound span. If the rewind was
  deliberate, delete the branch and this issue's restore option with it.
- The M3 global binary (`~/.cargo/bin/cargo-heal` → target/release, sha
  `047df27f…`) was rebuilt from `23134993` at 01:07-01:09 and verified e2e
  (`--version` exit 0; the P13 binding mark live). It embeds the rewound
  work — leave it until adjudication.

## Repair plan (supersedes the owner-decision-menu framing; owner may still veto)

The rollback-was-deliberate hypothesis is dead — this is collateral damage
from a stale-checkout force-push. The repair is mechanical, sequenced
around the live sibling session:

1. **Wait for 4090 session idle** (cargo/rustc processes exit; its scope_hint
   unit lands or pauses). Poll, don't interrupt.
2. **Broadcast**: commit this issue (+ `.issues/.highwater`) on top of
   `origin/develop` (`29d30bfd`+) and push — normal FF, no force. The
   sibling's next unit-start fetch then sees the incident + guardrails.
   Timing matters: pushed mid-unit, its next push is rejected and its
   rejection→force habit wipes the broadcast unread (the exact recurrence
   trap this plan avoids).
3. **Reconcile-rebase**: re-land the preserved chain
   (`e3e497c4..23134993` — P20 `4f17cff5`, P13 `23134993`, `--version` fix
   `ad31a04a`, docs rows) on top of the post-broadcast tip. Conflict
   surface: `HISTORY.md` + `.distill/001` (the sibling's `29d30bfd` touched
   both; textual, small). Re-run the P20/P13 validation legs (cfg_audit unit
   tests + fix_verify lane) after the rebase. Then fast-forward remote
   develop, and delete the backup branch in the same commit-window.
4. If the sibling force-pushes AGAIN after the broadcast lands, the
   guardrail text in this issue is the corrective instruction for it:
   fetch-before-push, never force-on-rejection — rebase or ask.

Owner veto remains: if the owner ACTUALLY wanted the evening wave gone,
   say so in this issue; the backup branch is deletable in one command.

## Guardrails until reconciled

- **No pushes to `develop` from the M3 while the 4090 session is live**
  (the recurrence trap above); the broadcast step is timed for its idle
  window.
- On ANY box in this repo: **never force-push on rejection** — fetch,
  rebase, or coordinate. Rejection means your base is stale, not that the
  remote is wrong.
- No release/tag builds from the rewound base as HEAD-of-record (it lacks
  P20/P13 + the `--version` fix).
- The 4090 `.exe` rebuild item stays blocked until reconcile-rebase lands
  (it should build from the reconciled tip).

Filed 2026-09-14 01:2x +07 by the idle-loop session that detected the
rewind. Detector: 4090 rebuild preflight (`git ls-remote` anomaly) — the
M3-side fetch had shown `23134993` 18 minutes earlier.
