# Issue 149 — three QA findings from the v0.2.1 funnel capture (real-workload QA)

**Status:** OPEN — 2026-10-06 (filed from riir-dapps Issue 112 T1: the refine.gist.rs capture generator ran v0.2.1 against a real sample crate — these are what it caught)

**Priority:** P2 — all three are quality/accounting issues, none blocking.

## Findings (from riir-dapps `ccdec24` + `cf4983c` capture work)

1. **The healer heals the current directory tree when given no paths — untracked files included.** The capture generator's first draft forgot the `cd` into its throwaway sample copy and `cargo refine --fix` healed the REAL kat-service tree (7 files; the untracked demo crate among the "kept"). Reverted there, but the binary behavior is real and dangerous for anyone running bare `cargo refine --fix` from a repo root with WIP. Fix candidate: a loud pre-scan banner naming the absolute directory about to be scanned (+"N untracked files included"), and/or requiring explicit paths above a tree-size threshold.
2. **Double-billing on one span:** `needless_return` + `let_and_return` both landed on the same line (7 metered edits for 5 lints). The result is correct; the accounting double-counts. Fix candidate: per-span dedup in the meter before it accumulates into `.refine/kat_charge.json`.
3. **clippy 0.1.98 drift vs the corpus:** `manual_clamp` no longer fires (even on the canonical doc example — silently) and `manual_range_contains` is silent on field-projection receivers (`item.f >= a && item.f <= b`; a local ident still fires). Both rules are in the corpus — a `--coverage-report` re-arm against the live toolchain would surface the drift class.

## Tasks

- [ ] T1 cwd-scan banner (finding 1) — loud absolute-path disclosure before any scan; consider the untracked count in the same line
- [ ] T2 per-span meter dedup (finding 2)
- [ ] T3 `--coverage-report` re-arm against clippy 0.1.98 (finding 3) — reconcile `manual_clamp`/`manual_range_contains` corpus entries

## Notes

Recorded in riir-dapps `.benchmarks/018` provenance chain and the `ccdec24`/`cf4983c` commit messages. The capture generator itself now cd-guards with a remote-origin sanity check (`gen_refine_capture.sh`) — that guard is client-side; this issue is about the binary's own behavior.
