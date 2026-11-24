# Research 445 — Batch 412: the mlxfast staged pool (Research 437's 8 candidates)

**Status:** LANDED — 2026-11-24, Plan 411, session zed-amo-i412@amo (epoch 1795495491).
Source: Layr-Labs/mlxfast-bonsai2-27b-engine, pin `0c7c3923` (MIT re-verified at the
clone; UNMOVED vs the Batch 402/401 pins). Clone `.raw/mlxfast-27b` @ `0c7c3923`
(re-cloned this session — the Batch 402 clone was torn down; torn down again at
close). Zero GPU throughout; amo load 3-5 (a sibling single-core eval running).

## The staged pool conversion

Research 437 staged 8 candidates by name. Each was re-surfaced in-source this
session (the Batch-409 lesson live: a staged lead's distill-value claim expires
against the corpus — re-grep at claim time, never mine from the staging text):

| # | Staged name | Verdict | Landed as |
|---|---|---|---|
| 1 | speculative-substitution-prefix-dominance | RULE | sec `retired-mode-named-refusal-not-deletion` — "prefix-dominance" has NO verbatim anchor (grep `dominan\|dominat` = zero hits tree-wide); the substitution law matched exactly, the label half was the stager's gloss on the decode-axis dominance (weight 0.75) |
| 2 | route-selection-frozen-at-construction | RULE | kernel_opt `route-bound-at-construction-hot-closure-shape-predicate-only` (Dispatch) — the mechanism lives in the VENDORED model-finalizer surface (Qwen35A3BOptimization.swift:589-612), NOT the staged start file (Transform.swift carries only the thinner precedence-ordered family detection) — the label was a stage pointer |
| 3 | speculative-prefab-adopt-or-drop | RULE | rust_perf `cache-restore-adopt-or-drop-whole-entry` (caching) — "prefab" has ZERO hits tree-wide; "adopt-or-drop wholesale" matched build-cache.sh restore exactly |
| 4 | unit-arity-sibling-verb | RULE | rust_perf `acceptance-band-derived-from-own-axis-measured-cv` (measurement-honesty) — no standalone "arity" vocabulary in-tree; the staged words map to per-axis noise units + per-verb gate furniture + pair-support constants |
| 5 | tested-ceiling-explicit-per-decoder | RULE | sec `entity-keyed-envelope-widen-tripwire` — the per-decoder envelope law (structural outer + contract-membership inner + the widen tripwire); the reader's original name `entity-keyed-tested-ceiling-lower-not-raise` reshaped at authoring to the verified mechanism (the tripwire direction is the load-bearing content) |
| 6 | device-readiness-spin-deadline-bounded | RULE | rust_perf `readiness-wait-liveness-first-deadline-bounded` (concurrency) — the LIVENESS-FIRST ordering is the new axis (resident-up.sh:582-600: `kill -0` precedes the socket probe; a dead child named in one poll) |
| 7 | per-step-shared-scalars-host-advanced-one-array | **DROP — prior coverage** | ALREADY OWNED by `host-mirror-shared-scalar-advance-one-time-verdict` (rust_perf entries_10, Batch 182) mined from the SAME repo lineage — the LayerCacheV2 host-mirror/one-shared-array/one-time-verdict law is the identical mechanism; the staged label was its restatement |
| 8 | calibration-band-two-sided-bounded-zero-cv-refused | RULE | rust_perf `calibration-band-two-sided-straddles-one-zero-cv-frozen-clock` (measurement-honesty) — zero-CV evidence verified at ranked-box-preflight.sh:540-544 ("a zero CV across repeated passes is a frozen clock, not a stable box"); the straddle law at :546-553 (low < 1 < high); the not-the-denominator role law at calibrate-box.sh:7-21 |

**Yield: 7 rules from 8 staged** (above the ~50% falsification expectation — the
pool was reader-adjudicated in-source before authoring; 1 prior-coverage kill,
0 quality drops). All suggestion-only NotBenchable (infrastructure-integrity /
gate-honesty / boot-diagnosis / calibration verdicts — no validator hosts them).

## Counts

- rust_perf 396 → 400 (entries_99: caching ×1, measurement-honesty ×2, concurrency ×1; notbenchable 254 → 258; annotated 345 → 349)
- kernel_opt 1125 → 1126 (entries_gpu_168: Dispatch; the D-slice census + Plan-003 table + the KERNEL_CATEGORIES header repaired to measured truth 275 — the header had sat at "269" through B411's +3, the B310-era pin-drift class, disclosed at the header)
- sec 85 → 87 (corpus.rs: InputValidation ×2)
- Total 1795 → 1802

## Fixtures (T4 before entry)

- `corpus/fixtures/b412_perf_fixtures.rs` 18/18 (debug AND -O) — the gate caught
  TWO fixture defects pre-entry: the boot-probe closure was FnMut not Fn (the
  wait signatures now take the general bound), and the corrupt cache member
  sorted FIRST alphabetically defeating the "late member" arm (renamed so the
  corrupt member sorts last).
- `fixtures/b412_sec_fixtures.rs` 8/8 (debug AND -O).
- `corpus/fixtures/b412_fixtures.rs` 4/4 (debug AND -O).

## T2 reader adjudication

Two parallel reader subagents over the clone; every QUOTE re-verified
byte-verbatim at the pinned lines by the coordinator (all 5 reader-A quotes +
3 reader-B surfaces re-pinned first-hand; reader-B's full report arrived as a
summary only — its three surviving surfaces were re-read directly, satisfying
the re-verification duty). Reader corrections carried: the C2 surface location
(vendor finalizer, not Transform.swift), the C7 prior-coverage identification
(entries_10 Batch 182 — verified by grep + in-row read before the DROP stood).

## Router annotation floor (Issue-020 contract)

Re-measured at the landing posture: **102/195 (52.3%)** — IDENTICAL to the
Batch 411 pin. The batch's delta = **0 spans** (the +1 kernel/+4 rust_perf/+2
sec rules' vocabulary joined the existing centroids; the labeled-fixture
population is unchanged at 195). **No re-pin — the 97/195 floor holds** (measured
102 ≥ floor 97, the same 5-span margin). Both lanes (latent + kernel+latent)
verified the same reading at the landing.

## Gates (T5)

- default lib 3981/0 (5 ignored) · kernel+latent 4128/0 · sec+latent 4629/0
- retrieval floors BOTH lanes 30/0 HELD (no re-pin — see the router paragraph)
- clippy -D ×3 postures clean (default / no-default / kernel,sec,clippy_lints,rust_perf;
  8 mechanicals in the new files healed in-place: raw-string hashes, doc
  backticks, the too_many_lines allow on the count-pin test whose tail comments
  grew it past 100)
- fmt bare rustfmt on the touched files
- numbering gate PASSED (1040 files, claim lint clean over 417) · dual-allocation clean
- pins_manifest --write (kernel 1126, perf 400/notbenchable 258, sec 87)
- count web one-landing (health_views kernel/perf/sec/total 1126/400/87/1802 +
  router.rs 1126 + goat_g1 census 1..=168 + the census count pin 169 + the
  kernel-side rust_perf witness 400 + the annotated pins 349 + the Plan-003
  Dispatch table 275 + AGENTS head ×3 + README table ×3 + .docs/11_domains ×3)
- Score-bench/freeze NOT owed (zero clippy_lints delta — the standing convention)
- G1 store NOT regenerated (the B354 precedent, pre-existing FAILs standing, disclosed)
- Release-set binary rebuild NOT owed (suggestion-only corpus delta — the
  B402/B409/B411 disclosure convention; the maintained binary folds into the
  next maintained-binary lane)

## Lane intel (recorded, not landed)

- The arena's retired-mode grammar (C1) is the house env-switch law's
  independent arrival on a THIRD surface (the enum-case axis: retired cases
  stay live, refuse by name).
- The widen-tripwire direction (C5) is the feature-cap gate's content axis
  inverted: the envelope may tighten silently, widening must trip red.
- The lower-median-pair scoring methodology (entries_96, B402) + this batch's
  band-derivation law (C4) together form the arena's full gate-furniture
  methodology — a candidate bar shape for perf-rematch scoreboard cells.

## Unmined residue (staged for future windows, read in passing, never proposed)

- Golden.swift's single-door wrapper-adoption law (`loadQwenGoldenFixture`
  the only Qwen golden door; `model_provenance` REQUIRED; absence-is-a-reject
  whose diagnostic NAMES the missing field — :404-415).
- ranked-box-preflight.sh:270-300's per-depth-oracle pre-flight ("a
  speculative submission never reaches the GPU to find its oracle missing").
- stage-baseline-workspace.sh:191-196's SwiftPM-symlink staging-order lesson
  (files placed into `.build-worker/release` BEFORE the build are discarded).

NOTHING INVENTED — every quote read at the clone at the pin this session.
Teardown: `.raw/mlxfast-27b`, `/tmp/b412_tgt`, the /tmp fixture binaries
removed at close.
