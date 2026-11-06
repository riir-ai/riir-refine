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
- [x] Update this sweep's doc rows to teach the verb forms (e.g. `cargo refine suggest`) wherever the fix/suggest spellings are rewritten — **done for the in-repo surfaces 2026-11-06 (same lane).**

## Landed 2026-11-06 (zed-c29c7b20@m3 idle lane — the in-repo half)

- **Source teach-strings (src/bin/heal.rs), 8 spots found stale beyond the landed rows — fixed in the working tree, YIELDED to the concurrent session editing heal.rs** (its header-hint removal superseded the broken `cargo refine help` pointer fix — same defect class, better design; coordination mailed to zed-715e3914@m3): (1) user_help fix-compile line teaches the verb; (2) usage_text USAGE line same; (3) the no-mode refusal teaches `fix, suggest, fix-compile`; (4) the dry-run banner "Real fixes: cargo refine fix"; (5) run_mine_intro's heal line; (6) the mine-intro auto-sync string; (7) the stats-consent "future fix runs" string; (8) the fixable-CTA fallback line. NOT committed by this lane (shared-file discipline) — the concurrent session's landing carries them.
- **Docs surfaces committed by this lane:** this repo's AGENTS.md, README.md (quickstart + rustfmt section + feature-table teach rows; plan/bench history rows untouched per the sweep rule), the cargo-refine SKILL.md usage block, and 12 .docs/ pages (07_examples/cargo_refine.md rewritten to the current law — its usage block still described the pre-2026-09-08 review-default posture; orientation/roles/domains/lessons index rows). `scripts/manual_release.sh` carries no fix spelling (verified).
- Historical records NEVER touched: `.distill/001` batch rows, README plan-history rows (L22/L64), seed_corpus probe records.

## Sweep — every doc still teaching `--fix --write --verify` / `--fix` as THE spelling

One batched sweep, not piecemeal (verdict decision). Each: replace the fix-spelling rows with the two-form law above; keep `--fix` accepted (quiet parser alias) but undocumented.

**Sibling half LANDED 2026-11-06 (zed-c29c7b20@m3, one `docs:` commit per repo, pushed):** katgpt-rs `e0835a1a4` · riir-ai `63cd36cfd` · riir-chain `d5f56c04` (folded the missed `--fix --domain docker` row via amend) · riir-dapps `8531523` · riir-deployer `3698307` · riir-game-sdk `8a37345` · riir-neuron-db `e81ff5e` · riir-reflex `2282944` · riir-rethink `0f9efd6` · riir-train `0ae4c960` · riir-viewbridge `849e2c9` · riir-refab `402875e`. Verified ZERO stale refs (no edit needed): riir-dao · riir-instinct · riir-kat · riir-reflexer. SKIPPED: seal-remake (its AGENTS.md + 5 seal-view files are sibling-session WIP — sweep lands after that does).

- [x] THIS repo: `AGENTS.md` (Lint fixing section + Build Commands block) — **LANDED 2026-11-06 (zed-c29c7b20@m3 idle lane):** the Status row, the global-binary row, the CLI-UX rows (bare-dry-run + roll-up CTA + fix row + burn-sync), and the Plan-005 verbs row all teach the verb forms; the roll-up row now quotes the SHIPPED CTA (`🛠️ ~N% fixable via `cargo refine fix` …`).
- [x] `../katgpt-rs/AGENTS.md` (Lint fixing section)
- [x] `../riir-ai/AGENTS.md` (Lint fixing section)
- [x] `../riir-chain/AGENTS.md` (Lint fixing section)
- [x] `../riir-dapps/AGENTS.md` (Lint fixing section)
- [x] `../riir-dao/AGENTS.md` (no lint section — verified zero refs)
- [x] `../riir-deployer/AGENTS.md` (Lint fixing)
- [x] `../riir-game-sdk/AGENTS.md` (Lint fixing)
- [x] `../riir-instinct/AGENTS.md` (Build commands: `--release` note rows carry no fix spelling — verified zero refs)
- [x] `../riir-kat/AGENTS.md` (no lint section — verified zero refs)
- [x] `../riir-neuron-db/AGENTS.md` (Lint fixing)
- [x] `../riir-reflex/AGENTS.md` (Lint fixing)
- [x] `../riir-reflexer/AGENTS.md` (Lint fixing — verified zero refs)
- [x] `../riir-rethink/AGENTS.md` (Lint fixing)
- [x] `../riir-train/AGENTS.md` (Lint fixing)
- [x] `../riir-viewbridge/AGENTS.md` (Lint fixing)
- [x] `../riir-refab/AGENTS.md` (Lint fixing)
- [ ] `../seal-remake/AGENTS.md` (Lint fixing) — BLOCKED on the sibling session's WIP in that file; re-sweep when its tree lands.
- [x] `.agents/skills/cargo-refine/SKILL.md` — the primary consumer doc: every `--fix`/`--write --verify` row — **LANDED 2026-11-06 (same lane):** the usage block teaches the verb forms, the consent paragraph names the verb-first law + the quiet aliases.
- [x] release docs teaching the old form: `scripts/manual_release.sh` comments (verified: no fix spelling — no work), README.md quickstart **LANDED same lane**, `.docs/` book pages that name `--fix` **LANDED same lane (12 pages)** (grep `--fix` across `*.md`, fix doc rows only — never bench/fixture goldens).

## Owner-gated decisions (verdict #OwnerGate, recorded)

1. **`--fix` flag hard-removal: DEFERRED.** The parser keeps accepting `--fix` as a quiet alias through the transition; revisit only after this sweep lands AND a workspace grep confirms nothing teaches the flag spelling anymore.
2. **`cargo refine verify` verb: DECLINED.** Verify is already implied in `fix` on release builds (`fix_verify`); a standalone verb needs defined semantics in a proposal first.
3. **Sweep batching: ONE issue (this one), one sweep, not per-repo dribbles.**
