# Plan 102 — Batch 122: the ds4 double-delta re-mine (PR vision branch + upstream main surge)

**Status:** RECORD 2026-09-07 — Part A (Batch 122) landed: rust_perf io +3 (105 → 108), all gates green (lib 1162/0, retrieval floors held byte-identical, score-bench FLAT 96.2/1.9), Research 145. Part B = Batch 123 (landed via Plan 103 / Research 146). T7 discharged 2026-09-07 (see below).

## Triggers (both fired, verified by ls-remote 2026-09-07 05:52)

- **PR branch `qwen3.8-flash-next`: `c5ae2eb5` → `69c38d23`** (the Batch-121 pin beaten; 4 commits: `9f4313d` vision download target, `3bf8552` merge, `c64bdff` vision quality results, `69c38d2` model-aware chat formatting fix).
- **Upstream `antirez/ds4` main: `9ab7053` → `c0a6119f`** (the Batch-117 pin beaten; 41 files, **+6118/−1100**: GLM attention/rope edits, NEW `tests/test_metal_ssd_experts.c` +239, `test_ssd_cache.c` +33, `test_session_state.c` +116, `test_glm_rope_prefill.py` +64).
- Fork note: `ivanfioravanti/ds4-metal` main == `c0a6119f` == upstream main (fork synced).

## Staged clone (the `.raw` discipline)

`.raw/ds4-metal` (gitignored; MIT; HEAD `69c38d2` re-verified == ls-remote at stage time). Upstream ref fetched as `FETCH_HEAD` → `c0a6119f`. Diff commands:
- PR delta: `git -C .raw/ds4-metal diff c5ae2eb5..69c38d23`
- Upstream delta: `git -C .raw/ds4-metal diff 9ab7053..c0a6119f`

## Part A — the PR vision delta (SMALL: 361 ln, 8 files) — mine FIRST

Surveyed rule candidates (dedup grep 2026-09-07: zero corpus coverage on all four):

1. **model-aware-format-over-hand-assembled-tokens** (`69c38d2`, the lead) — the CLI hand-assembled a user turn (`token_user` + vision span + `"\n"` + text pushes); image turns broke because the assembly drifted from the model's chat template. Fix: ONE model-aware `ds4_chat_append_multimodal_message` owns the turn. Failure class: hand-pushed token sequences diverge from the template (silently — the model just answers wrong). Generalizes to any structured-transcript emission.
2. **implemented-surface-requires-provisioning-target** (`9f4313d`) — vision was fully implemented (tower + Metal kernel + make target) but the DOWNLOADER never fetched the encoder, so `--vision` was dead out of the box. Rule: a feature needing an external artifact ships its fetch target in the same change.
3. **diagnostics-not-just-exit-code** (`69c38d2` test comment, the methodology lead) — "The REPL can return zero after an image turn fails if a later text turn succeeds. Check diagnostics as well as the process exit code." Regression must scan stderr patterns (`failed|outside the vocabulary`) alongside the exit code. NotBenchable-annotated (methodology, the B121 precedent).
4. **stdin-fed-path-line-break-guard** (test input validation) — image paths fed through stdin must be rejected on `\n`/`\r` (line breaks forge additional commands in a stdin protocol). Small; sec-adjacent but files in rust_perf io as a robustness rule or dropped — author's call at entry time.

Domain routing: all four → **rust_perf io/methodology** (they are build-tooling + measurement shapes, no kernel code — the B121 precedent). Corpus +3–4 (io 9 → 12–13).

## Part B — the upstream surge (BIG: 6.1K ln, 41 files) — Batch 123, own session

GLM attention/rope rework + NEW Metal SSD-experts machinery + session-state + SSD-cache tests. Expect kernel_opt rules (the SSD dual-cache lineage already has corpus coverage — dedup grep `ssd|state.space|expert` BEFORE proposing; new axes likely: expert spilling, session-state restore, rope-prefill positions). **Do not mine in the same batch as Part A** — different domain, different size class; keep the batches reviewable.

## Tasks

- [x] T0 — verify triggers + stage clone + survey deltas (this plan; done at write time)
- [x] T1 — Part A: author 3–4 entries (re-verify every quoted line against `.raw/ds4-metal` at entry time)
- [x] T2 — extend the standing parse gate (`batch122_pseudocode_parses`)
- [x] T3 — count pins: rust_perf io 9 → N, total 105 → N, annotated 48 → N; walk every manifest/doc line
- [x] T4 — GOAT: batch gates FIRST-SHOT + lib default/kernel_opt/kernel_opt+latent + retrieval re-run (kernel 100/100/100, blended ≥94.7/96.9/97.7) + clippy 3 states + score-bench FLAT
- [x] T5 — Research 145 note + AGENTS.md corpus count + `.docs/11_domains/` + snapshot row (same commit)
- [x] T6 — remove `.raw/ds4-metal`; commit `feat: batch 122: ...` + push
- [x] T7 — Part B re-scan: after Batch 122 lands, re-check `c0a6119f` for movement; open Plan 123 if still current. **DISCHARGED 2026-09-07 09:15 +07 (API, zero-build): upstream UNCHANGED at `c0a6119f` (the B123 pin held); the PR axis fired instead — head `69c38d23` → `ffd85d42` (re-based onto upstream main, ~47 new commits, 09-05 21:48Z..09-06 23:11Z window; the compare-vs-pin trap recorded in-plan). Part B itself had already opened as Plan 103 / Batch 123. Next: `.plans/106_batch125_ds4_rebase_surge.md` (Batches 125/126).**
