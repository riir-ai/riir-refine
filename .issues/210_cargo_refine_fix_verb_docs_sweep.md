# Issue 210 — cargo refine docs sweep: teach the `fix` verb + bare dry run (one way per action)

**Status:** OPEN — parse-tests + follow-up verbs LANDED 2026-11-06 (zed-m3-idle210); the mine/unmine surface REMOVED same day (GAP 10 extension). Remaining: seal-remake AGENTS.md re-sweep (blocked on sibling WIP), the --fix flag hard-removal revisit (owner-gated DEFERRED). — owner UX law 2026-11-06 (one way per action, Claude verdict AGREE `72d8f279`): fix = `cargo refine fix <paths>` (writes + compile-gates on release builds; `--no-verify` opts out; `--verify-args "--features <set>"` for gated code), dry run = bare `cargo refine` (no heal-mode dry flag; `--dry-run` belongs to `get`/`send`/`wetrun`).
**Lease closed:** zed-m3-idle210, 1793954917 — the claimed scope (parse-tests + follow-up verbs) landed 2026-11-06; provenance and residue in the Status row above.

## Landed already (the source of truth)

- [x] `src/bin/heal.rs` — usage_text top line teaches `cargo refine fix ...`; "(no mode)" block states bare IS the dry run (stale "--dry-run" claim removed); `--fix` flag desc names the alias; parse_args comment + validate_mode_flags docstring corrected; the `--dry-run` refusal names bare as the heal dry run; run_unmine explainer teaches `cargo refine fix`.
- [x] Personal `~/.config/zed/AGENTS.md` — lint rule + golden rule teach the two forms (the file this started from).

## Verb-first law (owner, 2026-11-06): any verb command is a bare word

- [x] `suggest` verb landed (parser arm + user_help + usage_text + run_unmine; `--suggest` stays a quiet alias) — same shape as the `fix` verb.
- [x] `fix-compile` verb landed (same arm block).
- [x] `suggest` with no paths now defaults to `.` like bare/`fix` (was a green-zero trap: "no .rs/... files under the given paths", exit 0 — found by the round-2 verdict probe, reproduced against the built binary, then fixed; all three review spellings now behave identically).
- [x] `fix-compile` with no paths joins the same cwd default (verdict closing note — probed: printed only the help banner, exit 0, scanned nothing; now reviews the cwd and hits its typed refusals, e.g. "fix-compile: no Cargo.toml above .").
- [x] Already bare verbs, no work: `fix`, `scan`, and every KatCommand word (`login`, `account`, `update`, `sync`, `topup`, `buy`, `wetrun`, `replay`, `config`, ...).
- [x] Parse tests for the verb arms (verdict round-2 gap): **LANDED 2026-11-06 (zed-m3-idle210)** — `parse_args_from(impl IntoIterator<Item = String>)` seam + `mod verb_parse_tests` (8 tests): verb-form ≡ flag-form for the heal verbs + every follow-up verb, path-before-verb stays a path, own-name strip, first-word-only precedence, `--stats` peek semantics, the points-plane words stay claimed (`stats`/`info`/`offline`), bare IS the dry run. Flags that take a VALUE (`--stats on`, the `--active-set-*` family) keep their flag spellings (documented in the usage VERB-FIRST LAW block).
- [x] Follow-up verbs (same law — each: dispatch/docs row updated + the flag kept as quiet alias): **LANDED 2026-11-06 (zed-m3-idle210), SCOPE AMENDED same day by the owner's vocabulary-law extension** — landed: `refill`, `misses`, and the maintainer report lane (`frontier-report`, `coverage-report`, `horizon-report`, `blueprint-report`, `gate-calibration`, `oracle-census`, `corpus-gap` (modifier class — alone it rides the bare posture, pinned), `config-audit`, `orphan-report`, `platform-audit`, `frame-time-report`, `score-bench`, `necessity-bench`, `active-set`). **`mine`/`unmine`/`purge` are NOT verbs — the whole `--mine`/`--unmine`/`--purge`/`--purge-server` CLI was REMOVED the same day per the owner's vocabulary law (issue 192 GAP 10 extension: mining vocabulary reads as crypto-extraction to non-blockchain users; `join`/`leave` is the contribution surface).** `stats`/`info` bare words stay the points plane's (landed Plan 325); `offline` landed Plan 342. `--version`/`--help`/`--help-all` stay flags (conventional, not verb commands).

## The mine/unmine removal (owner directive 2026-11-06, GAP 10 extension — landed zed-m3-idle210)

- [x] Verdict context checked (owner asked): issue 192 GAP 10 (2026-10-31) bans "mining" from user-facing copy; the CLI table ships `join`/`leave` as THE contribution surface; the release build is the POINTS posture (release.yml: `latent_retrieval,points,net-mainnet,fix_verify,update_check,kernel_pregate` — no kat_service), so nothing in the shipped binary reads `kat_mining_consent`.
- [x] heal.rs: `--mine`/`--unmine`/`--purge`/`--purge-server` flags + parse arms + the mine/unmine/purge verb arms + `run_mine_intro`/`run_unmine`/`run_unmine_purge`/`unmine_outbox_dir`/`print_mine_encouragement` runners + the flags_conflict guard + the mine-nag roll-up line — REMOVED. The stand-alone verb-family test rows updated; `ensure_logged_in` kept (run_stats_consent uses it), `DEFAULT_SYNC_POLICY_JSON` kept (the export preview + sync-policy gate read the SHAPE).
- [x] The `kat_mining_consent` STATE machinery stays (GAP-10 internal-identifier exemption): the persisted record still gates `sync`'s push half (Unset pushes — running sync IS consent; OptedOut refuses — re-enable = remove the record file, or `cargo refine offline` for the whole firewall); the auto-push tail still fires on OptedIn (existing opted-in machines keep working).
- [x] Every user-facing teach line re-pointed to join/leave/contribute vocabulary (soft-gate refusal, sync/replay opt-out lines, refill remedy, kat-charge line, the footer queue line, the `--info` panel headers, the trainer loop line, the help pages — user_help account fragment dropped the mine/unmine rows, usage_text credit fragments emptied).
- [x] `tests/mining_consent_gate_cli.rs` (renamed from `unmine_consent_cli`, Cargo.toml row same commit): the gate driven by PLANTED consent records + the removal pin (`--mine`/`--unmine`/`--purge` are unknown flags, write nothing, taught nowhere).
- [x] `scripts/charitable_gate.sh`: the `--mine` typed-refusal arm → the removed-surface pin (unknown flag, rc != 0, state unchanged).
- [x] riir-kat (the leaf, one commit there): the D4 refill refusal teach line → join+sync, the consent-module docs/warnings → the persisted-record framing (CLI writer removed), the purge wire doc → client-half framing (the wire route stays), tests updated.
- [x] AGENTS.md + .docs pages updated (the mine-line roll-up row, the mine onboarding row, the sync-policy writer note, role_flows/replay_lane/kat_billing/node_tiers/self_evolve_flow); dated audit records untouched.
- [x] Pre-existing (NOT this lane's): `corpus_stats_needle_tests::source_walk` red (kernel_opt 1077 walked vs 1061 runtime — the B336 landing's needle rot, proven via stash at clean HEAD) and the two `pins_manifest.rs` clippy findings (byte-identical to HEAD) — both belong to the stats/corpus lane, filed to that lane's owner via the session summary.
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
