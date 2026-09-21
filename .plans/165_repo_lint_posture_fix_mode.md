# Plan 165 — repo-lint-posture fix mode (`--repo-posture`)

**Status:** COMPLETE — all tasks `[x]`; landed as `79b1c1fd` (feat) + the docs close-out commit; the 09-21 intake's one OPEN product suggestion, closed.

## Why

The 2026-09-21 riir-ai clippy-sweep session (`e4e17b4be`) adjudicated OUT a
215-edit bulk apply: our clippy-lane corpus is BROADER than the target
repo's own clippy on some lints (the `doc_markdown` backticking class —
clippy's `[a-z][A-Z]` regex flags 16 spans where our proposer proposes
215), so `--fix` on a foreign repo restyles docs the target's gate never
complained about (26:1 churn). The hand-applied 16 real findings became
the intake's product suggestion: **a fix-lane posture mode that only
applies edits the TARGET repo's own clippy actually flags**.

The infrastructure already exists: the clippy feed (manual `--clippy-json`
or the Issue-113 auto-oracle) IS the target repo's own clippy diagnostic
set. The missing piece is gating the fix lane's PROPOSER edits on it.

## Contract (precise)

`cargo heal --fix [--write] --repo-posture <paths>`:

- **Scope**: gates the `clippy_lints` lane ONLY (the lane whose rules ARE
  clippy lint names — the target-gate mirror). The `rust_perf`/`sec` lanes
  (this tool's own rules, no clippy counterpart — a posture gate would
  decline them by construction) and the P21 oracle-edits lane (edits
  consumed FROM the diagnostics themselves — confirmed by construction)
  are UNAFFECTED.
- **Predicate**: a clippy-lane edit applies iff the feed carries a
  diagnostic with the SAME lint on the SAME file whose primary span
  OVERLAPS the edit's span (original coordinates at pass 0; later
  fixpoint passes compare against feed spans remapped through the applied
  splices, with authority-follows-replacement: a diagnostic span that a
  confirmed edit rewrote extends over the replacement text — nested
  re-exposed fixes inherit the enclosing confirmation).
- **Feed required**: `--repo-posture` with no feed (auto-oracle
  failed/disabled AND no `--clippy-json`) REFUSES exit 2 — never a
  silently-unverified apply wearing the posture name. A CLEAN tree
  (feed present, zero diagnostics) proceeds and declines every
  clippy-lane edit, loudly — that IS the posture answer.
- **Reporting**: per-file decline line + run-total (with per-lint
  breakdown) beside the existing oracle-edit totals. Declines counted at
  pass 0 only (the bench-guard precedent: later passes re-propose the
  identical declined set).
- **Misuse**: `--repo-posture` without `--fix` → exit 2 naming the
  requirement; the flag also breaks the bare posture (a silent suggest
  run under a fix-lane flag is the no-op trap).
- **Suggest lane**: untouched.

## Tasks

- [x] T1 — Feed span retention: `ClippyDiagnostic.span: Option<(usize, usize)>`
  (primary-span byte range; `None` when the JSON lacks byte offsets) +
  parser extension + parse tests. Additive; `line`/`suggestion` unchanged.
- [x] T2 — Posture primitives (lib): `posture_spans()` (per-lint original-coord
  span index for one file, `paths_ref_match` for file identity) +
  `posture_allows()` (same-lint overlap) in `clippy_feed.rs`;
  `map_span_through_splices()` in `edit_emitter.rs` (positions via
  `map_through_splices`; intersecting splices EXPAND the span to the
  replacement extent — the authority-follows-replacement rule) + tests.
- [x] T3 — CLI wiring in `bin/heal.rs`: `--repo-posture` flag + parse +
  `validate_mode_flags` (`requires --fix`, checked BEFORE the no-mode
  error so the message names the real problem) + `is_bare_posture`
  exclusion + `standalone_modes_are_never_bare`-class arm; the
  feed-required refusal (exit 2) after the feed computation; the fix-lane
  gate in the `lint_set` loop (per-pass filtering, pass-0-only counting);
  per-file + run-total reporting.
- [x] T4 — CLI contract test `tests/repo_posture_cli.rs` (the
  `build_stamp_cli.rs` process-boundary pattern; file-level cfg = the
  bin's required-features): (a) manual-feed gate wiring — a two-occurrence
  `manual_range_contains` fixture + a one-diagnostic feed → only the
  confirmed occurrence healed, exit 0, the decline reported; (b) no-feed
  refusal → exit 2. Hermetic: KAT kill switches + unreachable service
  URL + `RIIR_HEAL_AUTO_ORACLE=0` where the arm needs it; zero network.
- [x] T5 — GOAT: lib tests at the 2152 floor (+new); clippy clean at
  default + all-features; the CLI test green at its required feature set;
  score-bench run (#156) — the G3 lane-leak check (a posture leak into the
  default path would move heal rates).
  → **measured: lib default 2155/0 (+3); clippy_feed 29/29 under
  `latent_retrieval`; mode-flag 4/4; CLI contract 3/3; clippy 0 findings
  at default AND all-features (3 self-findings fixed pre-commit:
  2× field_reassign_with_default in the new test, 1× type_complexity on
  the parser helper → the `PrimarySpan` struct); score-bench 96.4% FLAT
  byte-identical (rows #156+#157, heal_rate 0.96363634 == Plan 164's).**
- [x] T6 — Docs: AGENTS.md CLI bullet; HISTORY entry; `.distill/001`
  intake row DONE marker; this plan's tasks checked off.
- [x] T7 — Global binary rebuild (beside-the-repo worktree, release
  feature set, `CARGO_TARGET_DIR` isolated) + `--version` non-stale verify.
  → built 6m23s at `79b1c1fd`; copied to BOTH `target/release/cargo-heal.exe`
  AND `~/.cargo/bin/cargo-heal.exe` (a plain FILE on this Windows box, not
  the documented symlink — the copy-half of the rebuild must touch both;
  live-verified: `--version` release set complete + the misuse arm exits 2
  with the flag's own message).
- [x] T8 — Commit + push (`feat:`), worktree cleaned up.

## GOAT gates

- **G1 correctness**: lib + CLI tests above; the two-occurrence fixture is
  the minimal faithful reproduction of the 26:1 incident class.
- **G2 cost**: O(lints × edits) per file with a per-file span index built
  once; no hot-path allocation beyond the index.
- **G3 no-regression**: default path byte-identical without the flag —
  score-bench at the 96.4% floor + existing lib/test floors.
- **G4 bounded**: zero new deps.
