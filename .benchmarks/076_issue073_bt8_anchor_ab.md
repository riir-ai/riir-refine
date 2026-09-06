# Bench 076 — Issue 073 T3: the Bt8 anchor-ledger A/B (entity-following twins)

**Status:** NEGATIVE — the pre-registered bar FAILs in both runs; the anchor-ledger builder stays opt-in, unpromoted. The failure mode is instrument-structural (edit-style fixer outputs never carry the identity-bearing signature line), and zero identity-regression events were observed in 48 calls.
**Date:** 2026-09-07
**Pre-registration:** `.issues/073` T3 block (committed 2026-08-06 / `085a5e8` lineage; DESIGN PICK + bars landed BEFORE any run) — this doc executes it verbatim, with the deviations below recorded BEFORE the run window opened.
**Runner:** `fixer_regate --arm BT8` / `--arm BT8DRY` (landed this commit) — pair construction from the frozen `score_bench::fixtures` set, T2 twin construction (`perturb_symbol_ident`, now a lib helper shared with the builder pins), grading via `ident_mentioned` (boundary-disjoint renamed/original), oracle scoring through the standard `score_case`.
**Backend:** CF `@cf/zai-org/glm-5.3-flash` via the wrangler OAuth bridge (Bench 067/068 transport; `wrangler whoami` refreshed OUTSIDE a `.env` dir first — the Bench-068 op note).

## Protocol (as pre-registered)

- **Pairs:** 12 perturbation twins = same-kind fn rename (`X` → `X_alt`) applied to frozen score-bench fixtures; graded property = the healed fix references the SWAPPED symbol (word-boundary).
- **Arms:** arm A = `prompt_growing` (the registered "plain prose baseline") vs arm Bt8 = `prompt_anchor_ledger` (same prose + the discrete identity ledger, condition-first). One call per pair per arm.
- **Bar (registered before any run):** arm Bt8 follows-swap in ≥10/12 AND (Bt8 − A) delta ≥ +3; secondary: arm Bt8 strict-keep (compile) must not regress below arm A.
- **Instrument validation:** `--arm BT8DRY` passed first — 12/12 pairs PINNED both directions (planted follow-generation graded `follows=true prior=false`; planted regress-generation graded `follows=false prior=true`). Construction canary: `renamed` total on every twin.

## Deviations (recorded BEFORE the run)

1. **Fixture window.** The registration's arithmetic ("the T2 fixture plus the next 11 in fixture-id sort order") is unsatisfiable forward: the frozen set has exactly 7 successors after the anchor. The window is the LAST 12 in id order — the anchor + all 7 successors + the 4 id-adjacent predecessors (`composite-static_lifetime_plus_useless_format` … `riir-ai-ruliology-player-001`). Deterministic, named before the run, contains the anchor.
2. **Empty trajectory store.** The repo dev store (`​.heal/trajectories.kv`) serves shape-similar channels into arm A only (`prompt_growing` takes channels; `prompt_anchor_ledger` does not) — leaving it in would confound the ledger delta with a channels-vs-ledger comparison. The run exported `RIIR_REGATE_STORE_DIR` at an empty dir so arm A's channels degrade empty, matching the registered "plain prose baseline". Recorded per-row (`channels_empty=true` throughout).
3. **Two runs, both recorded.** Run 1 used the harness default `--max-new 256`; six clean-fixture pairs hit the cap (exactly 256 completion tokens = truncation). Run 2 re-ran at `--max-new 1024` with a full prompt+generation dump. Run 1's verdict stands as run; run 2 is the truncation-free measurement. This is an instrument fix, not verdict shopping — both FAIL.
4. **Few-shot rule-label swap was a no-op.** The same-kind label map (the T2 landed pair `manual_map`↔`manual_unwrap_or`) never fired — retrieval returned 0/3 mapped labels on every pair (recorded `fs_swap=0/3` per row). The twin's graded perturbation is the fn rename only.

## Results

| run | max_new | follows A | follows Bt8 | delta | strict A | strict Bt8 | bar | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 256 (default) | 3/12 | 2/12 | −1 | 1 | 1 | primary FAIL · delta FAIL · strict PASS | **FAIL** |
| 2 | 1024 | 4/12 | 2/12 | −2 | 1 | 2 | primary FAIL · delta FAIL · strict PASS | **FAIL** |

Per-pair detail (run 2, from `076_bt8_run2_rows.jsonl`): follows-swap fired 4× on arm A, 2× on arm Bt8, never on the same pair; `prior_symbol` (the model spontaneously writing the ORIGINAL symbol) fired **0 times in 24 calls**.

## Failure mode (why the bar was unreachable — the load-bearing finding)

The graded property ("the healed fix references the swapped symbol") presumes signature-bearing outputs. glm-5.3-flash under the trained instruction responds in **minimal edit-lines style**: it emits only the changed lines (e.g. `.map_or(0, |(i, _)| i)`), never reproducing the function. The renamed symbol lives in the fn signature line, which a correct fix does not touch — so the symbol structurally cannot cross the output for either arm. Characterization of the 24 run-2 generations: 11 fenced-code, 7 code-plain (mostly ≤90-char edit fragments), 6 short prose; `fn_sig_line` present in **0/24**; neither-symbol outputs dominated both arms (A 8/12, Bt8 10/12).

Two consequences, both recorded honestly:

1. **The pre-registered bar FAILs and stands** — no promotion, no re-pin, no re-tuning of the bar until it passes. The anchor-ledger builder (`prompt_anchor_ledger`) stays as the landed opt-in surface with its builder-level pins (T1/T2); nothing promotes.
2. **The XBridge identity-collapse failure mode did not manifest to be guarded against.** In 48 calls across both runs the model never once wrote the prior/corpus-common symbol into the fix output (`prior_symbol=0` everywhere), and at the APPLIED-module level the merge contract preserves the twin's identity lines trivially (edits land on the twin module; the signature survives untouched). There is no measured identity loss for the discrete ledger channel to recover on this corpus with this fixer — the hypothesis is neither confirmed nor refuted in general; it is UNTESTABLE through this observable until the fixer emits signature-bearing outputs.

## Cost

48 calls total (2 runs × 24), `$0.0096` against the $1.00 daily cap (client meter, ledger v1 pricing v3). Server-side per-model GraphQL meter read 0.00 neurons — the known per-model view lag (Bench 068 caveat 4); the client meter is the record.

## Caveats

- n=12 pairs, one model, one seed-free temperature-0 pass per cell — the registered design; a thin pass by construction. The structural failure mode (edit-style outputs) is n-independent: no bar reachable at any n under this response contract.
- The clean fixtures (6/12) carry no target lint; the oracle is skipped for them (recorded `keep_strict=null`), so the strict-keep guard effectively runs on the 6 lint-bearing pairs (neutral-to-positive: 1→2).
- Arm A's few-shots and modelless candidate come from the standard retrieval pipeline (the product shape); arm Bt8 differs from arm A ONLY by the ledger block (empty channels, same twin, same few-shots with unswapped labels).

## What this closes / reopens

- **Closes** Issue 073 T3 as a measured negative under the pre-registered bar; the issue file is removed per the noise-reduction rule with this doc as the record. T4 (LAM cross-tokenizer mapper) remains deferred trigger-gated (recorded in `HISTORY.md`).
- **Reopens nothing automatically.** If a future fixer arm emits signature-bearing outputs (full-fn rewrites), the twin A/B is mechanically re-runnable (`--arm BT8 --max-new 1024 --gens <path>`) — the instrument is landed, validated (BT8DRY 12/12 PINNED), and cheap (~$0.005/run).

## Artifacts

- Rows (committed): `076_bt8_run1_rows.jsonl`, `076_bt8_run2_rows.jsonl`
- Generations (this run, /tmp, not committed): `/tmp/bt8_live/run2_gens.jsonl`
- Runner: `src/bin/fixer_regate.rs` `--arm BT8|BT8DRY`; helpers `perturb_symbol_ident` + `ident_mentioned` (`src/cf_workers_ai.rs`, shared with the builder-pin tests)
