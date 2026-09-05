# Bench 068 — Issue 067 arm G' (same-rule, condition-first) vs arm B: the Trace-as-State reopen

**Status: VERDICT PASS — G2 beats B on the pre-registered gate (2026-09-05).**
The Bench 051 negative (cross-rule, condition-LAST channels, `NEGATIVE-no-gain`)
is recalibrated, not overturned: with the two Research-530 deltas applied
(same-rule-only channels + condition-first placement), the growing-input arm
clears the bar the original construction missed.

- Issue: `riir-clippy/.issues/067_trace_as_state_growing_input_reopen.md`
  (T1 instrument + T2 construction + this A/B = T3; T4 = this doc, issue
  closes with the commit).
- Paper: katgpt-rs `.research/530` — arXiv:2609.02702 "Trace as State"
  (Zou & Tang 2026). Two falsifiable deltas vs Bench 051's arm G:
  1. **Placement** — conditioning before the context (`[z, C]`) dominated
     `[C, z]` in 26/27 model×task×metric cells. Arm G appended the channels
     after the code module (the paper's controlled variable); arm G2 places
     them between the instruction header and the module.
  2. **Relevance** — the paper's Random-Trace arm scored BELOW no-trace
     (Parents EM 14.2 vs 29.2): weakly-relevant conditioning is actively
     harmful. Arm G's channels were cross-rule by construction (the
     reachable set is off-corpus — Bench 051's own diagnosis). Arm G2
     restricts both channels to the queried lint's family (boundary-safe
     `{lint}:` retrieval prefix + family-scoped pattern fold).

## Construction (landed with T2, `e053671`)

- `prompt_growing_same_rule` — arm B's body with the channel blocks inserted
  BEFORE the code module. Arm G's rendering extracted byte-preserved
  (`render_growing_channels`); arm A/B/G prompts untouched.
- `RegateHarness::trajectory_context_same_rule` — channel 1 retrieves with
  the `{lint}:` key prefix (a bare prefix would admit `ptr_arg_extra` —
  unit-pinned); channel 3 folds only the family's rows (the row's own
  `lint_key` field, `== fam || starts_with(fam + ":")`).
- `fixer_regate --arm G2` — JSONL carries `growing_prime_channels_empty`;
  the DRY golden line gained `growing2=` (hash) + `channels2=`.
- `RegateHarness::new` accepts `RIIR_REGATE_STORE_DIR` (default CWD, the
  arm-G convention) so the A/B can point at a planted store without CWD
  tricks.
- Degradation contract pinned twice: unit test (`empty ⇒ == prompt_prepass`)
  and live DRY (`growing2 == prepass` hashes for every uncovered sample).

## The store (honest provenance)

The reachable-miss set is off-corpus, so no dev store carried same-rule
history for it (the T1 trigger fired for OTHER lints — corpus-side, e.g.
riir-ai's `uninlined_format_args`). The A/B therefore ran against a planted
scratch store (`examples/plant_067_store`, kept as the reproducible tool):
**4 lints × 3 Certified rows** (ptr_arg, or_fun_call, redundant_clone,
box_collection) — real fix semantics, hand-varied representative shapes,
every row `success && verified` ⇒ Certified (the T1 K=3 condition holds per
lint, self-asserted by the planter).

**The exemplar content is CONSTRUCTED, not production store history.** The
4 uncovered reachable lints (large_enum_variant, too_many_arguments,
vec_init_then_push, + the second ptr_arg/box_collection rows) got no rows —
arm G2 ≡ arm B byte-identically there, per-row recorded. The
production-density path stays the T1 trigger watch.

## Run

2026-09-05, M3, one session: `--arm B` then `--arm G2`, same model
(`@cf/meta/llama-3.3-70b-instruct-fp8-fast` — the Issue-040 arm; the CLI
default has since drifted to glm-5.3-flash, which 401s under the token —
pass `--model` explicitly), same store, 25 samples (`source=both`,
limit 16), strict-keep + never-wrong filters on, `$0.0099` (B) / `$0.0102`
(G2) against the $1.00 daily cap. B ran before G2 (not interleaved); scoring
is deterministic clippy-based, not latency-sensitive.

| arm | n | keep_o/scoreable | keep_s (all) | reachable-miss strict-keep | vetoed | regressions | prompt tok | $ |
|---|---|---|---|---|---|---|---|---|
| B | 25 | 21/24 | 20 | **5/10** | 1 | 1 | 10,644 | 0.0099 |
| G2 | 25 | 22/24 | 21 | **6/10** | 1 | 1 | 11,710 | 0.0102 |

Per-case flip (the only keep_strict change across all 25 samples):
`reachable:redundant_clone` False (B) → **True (G2)** — a COVERED lint
(channels2 live). Zero kept-wrong in both arms (no keep_oracle row carries
`strictly_fewer: false` or a noop); the vetoed/regression case profiles are
identical across arms.

## Verdict (pre-registered in the issue)

| clause | bar | measured | |
|---|---|---|---|
| strict-keep | G2 ≥ B + 1 | 6 ≥ 5 + 1 | ✅ (exactly +1) |
| zero kept-wrong | none | none, both arms | ✅ |
| token cost | ≤ 2× B | 11,710 ≤ 21,288 (+9.9%) | ✅ |

**PASS.** The discriminating subset is exactly the 4 covered lints; the 5
uncovered rows flip nothing (the degradation contract held through the real
model path, not just unit tests).

## Honest caveats

1. **The margin is exactly the pre-registered bar (+1)** on n=9 reachable
   rows (10 scored incl. one duplicate id) — a thin pass by design: the
   issue's own gate was written before the run and it passed as written,
   but one flip is one flip. The Bench-051-vs-this comparison (same
   harness, same set, cross-rule vs same-rule) is the load-bearing
   qualitative delta; production density (the T1 watch) is the path to a
   wider sample.
2. **Constructed exemplars** (real fix semantics, hand-varied shapes) — the
   channels carried true, verified content for the queried rule, but not
   organic store history. A production store may carry noisier same-rule
   content (failed attempts alongside kept ones — the FailurePattern
   channel renders REJECTED pairs deliberately).
3. **B-before-G2 ordering** — not interleaved. CF serving variance applies
   per-call; the scoring axis (clippy findings) is deterministic per
   response, and the flip is a large behavioral change (vetoed → clean
   keep), not a borderline re-grade.
4. **Model pin**: the run used llama-3.3-70b-fp8-fast explicitly. The CLI
   default model has drifted (glm-5.3-flash) and 401s under both the .env
   API token (analytics-scoped, no Workers-AI run permission) and a stale
   OAuth token; the working credential path is the wrangler OAuth bridge
   refreshed by running any wrangler command OUTSIDE a directory carrying
   `.env` (wrangler auto-loads `.env` from CWD, which masks OAuth — the
   ops_nightly.sh 0b note, hit live here).
5. Server-side neuron meter read 0.00 for the model today (the GraphQL
   per-model view lagged/omitted these runs); the client-side meter is the
   record. Total spend well under the $1.00 cap.

## What this reopens / closes

- The growing-input class is REOPENED for the healer with the corrected
  construction (same-rule + condition-first). Bench 051's
  "second negative closes this class permanently" clause does not fire —
  this was the first properly-placed negative-to-positive cycle.
- The placement principle applies to the healer's short prompts only as
  measured here; the paper's large gains are long-context — do not cite
  +38 EM on this harness.
- Production adoption path: no flag flip — the fixer service's prompt
  builder gains the same-rule channels when its store has fired keys for
  the queried lint (the T1 trigger log line is the per-lint green light).
  Until then the channels degrade to arm B byte-identically, which this
  bench measured as safe (identical outputs on uncovered rows).
