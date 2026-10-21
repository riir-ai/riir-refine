# Research 307 — Batch 271: the splash-wave second-tier serving pool (Plan 275)

**Status:** RECORD — landed 2026-10-21 (shikuwa/4090, riir-refine idle loop; the claim
`2cfcb104d` discharged by this landing). Source: incoai/splash — the B268/B270 staged
second tier (13 B/B− candidates enumerated in Research 305 §STAGED + Research 306
§staged), author Jian Chen, Apache-2.0. Clone `.raw/b271/splash` (re-cloned mid-session
after a sibling batch-close cleanup removed it; all quotes re-verified at the pins).
Zero-GPU throughout.

## What landed

+12 OWN rust_perf rules, corpus 254→**266** (entries_43/44/45/46 — four authoring
slices at ≤3 rules each; every fixture standalone-compiled + executed before landing;
ALL NotBenchable annotated; every quote machine-verified at its pin), + **4
count-neutral rider faces**, + **3 honest drops** at the T0 re-read:

| # | rule | category | sha(s) | slice |
|---|---|---|---|---|
| 1 | resume-admission-sized-to-per-command-window-not-suspended-peak | allocation | fa63d97 (+face 593f2a6) | A |
| 2 | phase-scoped-staging-released-at-successor-reservation-when-plan-budgets-max | allocation | 88d0e57 | A |
| 3 | derived-reservation-taken-once-after-admission-settles | allocation | 81ae003 | A |
| 4 | size-class-keyed-reader-cap-bimodal-reads | dispatch | 4d17995 | B |
| 5 | fixed-read-command-runs-alone-sized-by-its-own-reads | dispatch | 68b9e70 | B |
| 6 | reject-plan-runs-before-setup-with-outliving-side-effects | dispatch | dab8841 | B |
| 7 | admission-tests-the-bound-the-admitted-mode-serves | dispatch | 5ea4043+82b93c5 | C |
| 8 | in-use-state-retry-under-serving-mark-keyed-on-refusal-cause | dispatch | b7844dc | C |
| 9 | one-event-per-writer-consumer-waits-on-the-side-it-needs | concurrency | e2c0af2 | C |
| 10 | finished-command-copy-leaves-at-once-slot-returns-at-last-reader | concurrency | 91d4ddb | D |
| 11 | part-cost-measured-by-planning-ablation-not-component-sum | complexity | ed1092f | D |
| 12 | plan-input-precomputed-per-output-variant-bounded-convergence | complexity | 2f5fe8d | D |

Riders: 253506f (spill-tier victim-exclusion face →
`restore-point-chain-copies-keyed-on-disk-state-count`, entries_39) + 4fc7465
(durable-label-at-reclassification face → same host) + 4d17995's buffer half
(task-scoped-read-buffer, the FOURTH retention policy → `reuse-allocation-clear`,
entries_02) + 593f2a6 (anchor-bound output face → slice A's own rule 1, folded in its
context_notes). Drops: 6fcd9b1 (error-copy/config-honesty), 25502fd (DRY refactor, the
commit's own "Effect: none"), 7e51fcb (error-copy-only). 4d17995's third mechanism
(batch-end retirement) dropped as lifetime plumbing.

## Slice-reported drift disclosures (all landed in-rule)

- Rule 1 slice A: the anchor comment sits at Engine.cpp :2154-2158 (the staged
  :2149-2153 was the suspendLane signature); Runtime.cpp is runtime/model/ (not
  runtime/engine/).
- Rule 1 slice B: `page-miss-blocks-one-ranged-read-scatter` lives in entries_42 (the
  grading staged entries_41).
- Rule 3 slice B: the tier-files comment begins at :898 (the staged :900-917 was the
  second sentence onward).
- Rule 2 slice C: the Serving-doc quote lives in Engine.hpp:651-653 (the staging said
  Model.hpp — FILE DRIFT disclosed in-note).
- Rule 3 slice C: the shared-event comment spans :497-505 (staged :498-502); the
  source says macOS 27 (four occurrences), not 26.

## Gates (all run this session, isolated target /tmp/b271_rel)

- default lib **2729/0** (incl. `batch271_rules_landed` FIRST-RUN PASS after its own
  author fixed the rider-label assert spelling + slice A's `batch271_slice_a_rules_landed`
  green).
- rust_perf_bench lib **2760/0** (6 ignored documented).
- freeze gen2+gen3 e2es PASS (28.6 s + 24.7 s) — NO table movement (the freeze corpus
  builder is the clippy_lints `build_seed_corpus()`; a rust_perf-only delta moves
  nothing — the P30/B250/B270 line).
- clippy −D clean across FOUR postures (default / no-default / rust_perf_bench /
  kernel_opt+latent). En-route: my batch header's doc-list items (lines starting
  `+ `) + unbackticked `entries_NN` tokens (slice A's own line 19) healed by a
  header rewrite; + 3 PRE-EXISTING B270 residues at the kernel_opt+latent posture
  repaired in-batch (batch251/batch268 gates' too_many_lines + retrieval_eval
  doc_markdown — the posture B270's own gates never ran).
- retrieval floors HELD 22/22 no re-pin (blended floors 124/141, 130/141, 132/141;
  462 s suite).
- score-bench **#198** heal 0.9636 FLAT / parse 1.0 (gen-1 SATURATED, 19th
  consecutive).
- release-set binary rebuilt + installed at the landing HEAD (the §0 bin-age law;
  the first build carried `-dirty` — rebuilt after the landing commit so the stamp
  is clean).

## Lessons (this session)

1. **Slice-agent commits raced on pins**: slice A committed+pushed with pins at its
   own +3 (257/136/204) while slices B/C/D files were already in the tree but
   unregistered — consistent-by-construction (unregistered files compile to
   nothing), but the landing's pin pass found the annotated count asserted at THREE
   sites (the 1905 `annotated.len()` form, the 2021 `annotated,` form, and the
   drafter-propagation form at 2061) — a count pin's SITES are discovered by the
   failing lib, not by grep (grep found 1, the lib found 3).
2. **Rider-face label spelling**: entries_02's house style is `BATCH NNN RIDER`
   (the B248 precedent's multi-line continuation form) vs entries_39's
   `RIDER (Batch NNN / Plan NNN` — the needle gate asserts BOTH spellings.
3. **Doc-list trap**: a `//!` line starting with `+ ` opens a markdown list and
   every following line becomes a lazy-continuation finding (56 errors from 3
   bullets) — keep `+` at line END in module headers.

## Staged next

Splash post-window (`1482f6b`.. tag 1.4.0 + past) = the next standing-watch's
concern. The second tier is now EMPTY — the pool re-arms on the next watch poll.
