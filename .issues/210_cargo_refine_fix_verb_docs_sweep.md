# Issue 210 — cargo refine docs sweep: teach the `fix` verb + bare dry run (one way per action)

**Status:** OPEN — owner UX law 2026-11-06 (one way per action, Claude verdict AGREE `72d8f279`): fix = `cargo refine fix <paths>` (writes + compile-gates on release builds; `--no-verify` opts out; `--verify-args "--features <set>"` for gated code), dry run = bare `cargo refine` (no heal-mode dry flag; `--dry-run` belongs to `get`/`send`/`wetrun`).

## Landed already (the source of truth)

- [x] `src/bin/heal.rs` — usage_text top line teaches `cargo refine fix ...`; "(no mode)" block states bare IS the dry run (stale "--dry-run" claim removed); `--fix` flag desc names the alias; parse_args comment + validate_mode_flags docstring corrected; the `--dry-run` refusal names bare as the heal dry run; run_unmine explainer teaches `cargo refine fix`.
- [x] Personal `~/.config/zed/AGENTS.md` — lint rule + golden rule teach the two forms (the file this started from).

## Verb-first law (owner, 2026-11-06): any verb command is a bare word

- [x] `suggest` verb landed (parser arm + user_help + usage_text + run_unmine; `--suggest` stays a quiet alias) — same shape as the `fix` verb.
- [x] `fix-compile` verb landed (same arm block).
- [x] `suggest` with no paths now defaults to `.` like bare/`fix` (was a green-zero trap: "no .rs/... files under the given paths", exit 0 — found by the round-2 verdict probe, reproduced against the built binary, then fixed; all three review spellings now behave identically).
- [x] `fix-compile` with no paths joins the same cwd default (verdict closing note — probed: printed only the help banner, exit 0, scanned nothing; now reviews the cwd and hits its typed refusals, e.g. "fix-compile: no Cargo.toml above .").
- [x] Already bare verbs, no work: `fix`, `scan`, and every KatCommand word (`login`, `account`, `update`, `sync`, `topup`, `buy`, `wetrun`, `replay`, `config`, ...).
- [ ] Parse tests for the verb arms (verdict round-2 gap): parse_args() is env-fed today — test that `fix`/`suggest`/`fix-compile` verb forms set exactly what their flag forms set, and that a path before the verb word still counts as a path. Needs a small parse_args(impl IntoIterator<Item = String>) refactor or an argv-snapshot seam; ship with the follow-up verb pass.
- [ ] Follow-up verbs (same law, next pass — each needs its dispatch/docs row updated + the flag kept as quiet alias): `mine`, `unmine`, `purge`, `stats`, `info`, `refill`, `misses`, `offline`, and the maintainer report lane (`frontier-report`, `coverage-report`, `horizon-report`, `blueprint-report`, `gate-calibration`, `oracle-census`, `corpus-gap`, `config-audit`, `orphan-report`, `platform-audit`, `frame-time-report`, `score-bench`, `necessity-bench`, `active-set`). `--version`/`--help`/`--help-all` stay flags (conventional, not verb commands).
- [ ] Update this sweep's doc rows to teach the verb forms (e.g. `cargo refine suggest`) wherever the fix/suggest spellings are rewritten.

## Sweep — every doc still teaching `--fix --write --verify` / `--fix` as THE spelling

One batched sweep, not piecemeal (verdict decision). Each: replace the fix-spelling rows with the two-form law above; keep `--fix` accepted (quiet parser alias) but undocumented.

- [ ] THIS repo: `AGENTS.md` (Lint fixing section + Build Commands block) — NOTE: AGENTS.md is sibling-WIP-dirty as of 2026-11-06; sweep after that lands.
- [ ] `../katgpt-rs/AGENTS.md` (Lint fixing section)
- [ ] `../riir-ai/AGENTS.md` (Lint fixing section)
- [ ] `../riir-chain/AGENTS.md` (Lint fixing section)
- [ ] `../riir-dapps/AGENTS.md` (Lint fixing section)
- [ ] `../riir-dao/AGENTS.md` (no lint section — verify zero refs)
- [ ] `../riir-deployer/AGENTS.md` (Lint fixing)
- [ ] `../riir-game-sdk/AGENTS.md` (Lint fixing)
- [ ] `../riir-instinct/AGENTS.md` (Build commands: `--release` note rows carry no fix spelling — verify)
- [ ] `../riir-kat/AGENTS.md` (no lint section — verify zero refs)
- [ ] `../riir-neuron-db/AGENTS.md` (Lint fixing)
- [ ] `../riir-reflex/AGENTS.md` (Lint fixing)
- [ ] `../riir-reflexer/AGENTS.md` (Lint fixing)
- [ ] `../riir-rethink/AGENTS.md` (Lint fixing)
- [ ] `../riir-train/AGENTS.md` (Lint fixing)
- [ ] `../riir-viewbridge/AGENTS.md` (Lint fixing)
- [ ] `../riir-refab/AGENTS.md` (Lint fixing)
- [ ] `../seal-remake/AGENTS.md` (Lint fixing)
- [ ] `.agents/skills/cargo-refine/SKILL.md` — the primary consumer doc: every `--fix`/`--write --verify` row
- [ ] release docs teaching the old form: `scripts/manual_release.sh` comments, README.md quickstart, `.docs/` book pages that name `--fix` (grep `--fix` across `*.md`, fix doc rows only — never bench/fixture goldens)

## Owner-gated decisions (verdict #OwnerGate, recorded)

1. **`--fix` flag hard-removal: DEFERRED.** The parser keeps accepting `--fix` as a quiet alias through the transition; revisit only after this sweep lands AND a workspace grep confirms nothing teaches the flag spelling anymore.
2. **`cargo refine verify` verb: DECLINED.** Verify is already implied in `fix` on release builds (`fix_verify`); a standalone verb needs defined semantics in a proposal first.
3. **Sweep batching: ONE issue (this one), one sweep, not per-repo dribbles.**
