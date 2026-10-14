# 165 — run-end per-domain roll-up + mine encouragement; update/sync corpus epoch gain

**Status:** IN PROGRESS — 2026-10-14

Owner ask (verbatim intent): the bare run ends in `… and 94 more (the report
lists every finding)` — a dead end; it should show a per-topic total
(`perf`, `sec`, …), suggest `--fix` with the gain, and suggest `--mine` with
an estimated KAT gain to encourage contributing. And `cargo refine update`
shows only the epoch being installed — it should show current → new plus a
corpus diff so the reader sees what the lease actually buys.

## Tasks

- [x] riir-kat: `corpus_epoch_diff` / `EpochDiff::summary_line` /
      `corpus_shape_line` — pure, over the existing `DomainTriples` overlay
      type; NO wire change (both overlays come from content-addressed
      segments the atomic swap never removes)
- [x] riir-kat: unit tests (adds/removes per domain sorted, payload-insensitive
      id-set comparison, empty-diff → None, shape line + empty-corpus refusal)
- [x] `update` (and `sync`'s Applied arm): current → new epoch diff line;
      first install shows the installed shape; 304 shows the shape it keeps
- [x] bare run: `findings by domain: clippy N · … — N total` — tallied by
      `print_suggestion` itself (tally == printed lines by construction),
      lanes merge after (disjoint by file kind; kernel cube-body overlap
      disclosed), secret scan merges under `sec`
- [x] fix CTA on suggest-only runs with fix-surface findings:
      `next: ~N clippy finding(s) look auto-fixable — run cargo refine --fix`
      (N = citation ∩ compiled FIXABLE∪VERIFY floor, clippy-only, "look" —
      never a promise); secrets-only runs get no fix CTA (never auto-fixed)
- [x] mine line (`kat_service` + `latent_retrieval`, Unset consent only):
      composes the pending estimator verbatim (caveat #15 language, the
      "provisional rate" travels with the figure); zero pending → the flow
      line on suggest-only runs with findings; synced-awaiting-proof rows
      name no CTA (contributed; the quorum decides)
- [x] bin tests: roll-up line, lane merge + zero-silence, total gate shape
- [x] AGENTS.md CLI UX contract rows (roll-up + epoch gain surface)
- [-] defer: a per-domain FIXABLE set for rust_perf (no static set exists
      today; the CTA stays clippy-only until one lands)
- [-] defer: kernel cube-body double-count resolution (the syn pass and the
      wgsl/cube lane can both cite one body; disclosed in-code, a roll-up
      approximation)

## Verdict

Both surfaces land honest-by-construction: every number is either counted
from what the run printed, read from bytes both epochs still have on disk,
or composed by the existing estimator with its own caveat language. No
fabricated rates, no wire change, no new consensus surface.
