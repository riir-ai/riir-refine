# 435 — Batch 400: the mvm (Magni VM) distill → kernel_opt

**Status:** LANDED
**Batch:** 400 · **Plan:** 400 · **Session:** zed-m3-mvm-b400, epoch 1795381511
**Source:** github.com/Bobliuuu/mvm @ `60c42deb5628480d62a50d53296e5f8be77e9030`
(main tip 2026-11-19T16:55Z — **UNMOVED from the 11-21 verdict pin**: the T0
re-clone landed HEAD == pin, every quote byte-exact at the same SHA; 507
commits, shallow 50).
**License:** Magni Community License 1.0 — source-available, free ≤$1M revenue
/ research tiers. **Transcription-only GO, outbound-PR FAIL-CLOSED** (every
row decision-content; the revenue-gated license forbids carrying fixes
upstream).
**Controlling verdict:** the 11-21 staged-distill row
(`.distill/001_mining_queue_snapshot.md`, session zed-m3-mvm-distill) +
`../riir-infer/.research/36_*.md`.

## What landed

**kernel_opt 1105 → 1107** (`entries_gpu_162`):

1. `version-snapshot-reader-before-next-publisher` (A, the verdict's R1
   LEAD) — snapshot isolation as a GRAPH FACT: an `exact N` version wait
   inserts the edge reader → publisher-of-N+1, so the next update cannot
   overwrite the data while the reader runs. The premise is the
   memory-model honesty: acquire/release buys happens-before for version
   N's own stores, never EXCLUSIVITY against N+1 ("Whether it can also see
   NEWER stores is the graph's job", mvm_exec.cuh:31-32). Three compile-time
   refusals (stale `N < base_version` / N+1 already ordered before the
   reader / N never going to be published) + the fourth
   (`published only after the waiter`, FRAMEWORK.md:93-94); `unordered
   publishers` a compile error (:84-85); one wait slot per descriptor, extra
   waiting reads materialized as preceding `__gate/...` noop tasks (:90-92).
   Coherence is measured as a per-reader census (demo-tiny: 184/184 exact;
   demo-large-trace: 22,935 all exact; the VM "checks `exact` reads at
   start and finish and fails the task on a violation"). The executor-side
   lost-wakeup half is Dekker-closed (cuh:35-36) and adversarially litigated
   (litmus.cu "a litmus that cannot fail proves nothing"; REVIEW.md controls:
   SB-without-fence 19,222 / publish-before-stores 2.9M / reader-without-
   acquire 55,737 / relaxed CAS 244 / waiter-without-recheck 23,020 /
   publisher-exchange-first 11,800 — all detected).
   Adjudications owed at claim time all ran: gpu_138
   `page-reuse-fenced-behind-reader-completion` (page-REUSE lifecycle fence
   at runtime vs the VERSION-COHERENCE edge at compile time — compose on a
   paged serving lane), gpu_61 `ring-overwrite-gated-k-back-readiness`
   (device-ring distance-K gating — adjacent, cited), gpu_85 (barrier-cost
   pricing — no version axis), rust_perf entries_86
   `closed-channel-select-arm-gate` (arm-gating liveness vs static deadlock
   refusal), the gpu_148 epoch-tag echo (cross-domain, recorded never
   landed). Suggestion-only NotBenchable (`bench_spec` n/a — kernel rules
   carry no bench spec field; the Batch-136 annotation class in-concept).

2. `parked-waiters-release-off-worker` (E, the verdict's R2 — LANDED after
   its pre-registered gpu_50 adjudication survived) — waiter custody: a
   version-waiter occupies NO worker block and NO ready-queue cell (cuh:5-8
   "waiters occupy no block and no queue cell"); it parks on the variable's
   lock-free list, released by the publisher's detach-all scan
   (`atom_exch(park_head, -1)`), the unmet re-parked ("A scan re-parks
   nodes whose version is still too low"). The SECOND budget is the
   liveness half (cuh:11-16): the runnable window `(admitted - done -
   parked + n) > live_cap` EXCLUDES parked, parked carry their own
   `live_cap` budget, so a publishing bundle is admissible behind a full
   parked set (the review's C2 deadlock — "All of bundle 1 parks,
   admitted - done == live_cap, bundle 2 never fits" — cannot form);
   over-budget parking is the explicit `PARK_STALL`, never a silent hang.
   The fast path: an already-met waiter never parks. The mutant harness is
   the fixture discipline at scale (`MVM_BREAK_PARK_RECHECK`: 12,834 lost
   wakeups / 0.3M detected; clean 6.0M rounds / 864 CTAs zero).
   Adjudications: gpu_50 `split-submission-in-flight-cap-overlap` (in-flight
   CAP + overlap — how much may be submitted; THIS where waiters WAIT —
   compose), rust_perf entries_74 (class-share partition vs the
   runnable-vs-parked custody split — different axes, compose), entries_83
   singleflight (leader election — no custody axis), gpu_85 (the liveness
   half's straggler face — cost vs custody). Total in-flight bounded at
   2×live_cap by the two budgets.

**rust_perf: count-NEUTRAL** — 1 face on entries_74
`writeclass-share-partition-inflight-writes` (the verdict's R3): the mvm
admission-FIFO measured instance of the same partition law — the admitter is
ONE FIFO at 1.9 µs/task (0.52M tasks/s), head-of-line is K×1.9 µs; "A
47k-task training record puts every Doom frame behind 100-300 ms; no policy
fixes it, host-side per-class budgets help at the price of shedding load
(38% of serve requests dropped in the tuned run)" (README.md:329); the
tuned run's honest cost (adaptive README:23: "sheds 38% of the serve
requests (797 of 2101 dropped) and cuts training 32%. This is load
shedding, not a free win"); the shared-live-window cell (README:57):
112.7 ms ungated vs 0.26 ms with the flood limited to half the window.
Counts stay 389 / annotated 337.

## Honest drops (the verdict's pre-registered drop-or-face rows)

- **R4 `ready-ring-single-head-contention` (C+ thin) — DROPPED.** The datum
  (about 107 pollers race one head CAS, one winner per L2 round trip → 9/106
  workers busy, README.md:133) is real but the source's own remedy attempts
  measured no change ("perf/completion-path: no end-to-end change (the step
  is bound by the ready-queue pop)", README.md:194) and no bounded fix-space
  landed in-source; the corpus's contention owners (singleflight, gpu_148
  mailbox rotation) own different axes. The datum is recorded here; a future
  pop-batching/delegation source re-opens the slot.
- **R5 `scheduler-regime-split-overload-vs-steady` (C+/B− thin) — DROPPED.**
  The honesty row (dynamic scheduler serve p99 1.7–1.8× worse than static
  priority at equal throughput, wins only under overload 8–11×; the
  adaptive controller loses to hand-set config — README.md:331 + the
  adaptive README) restates the corpus's existing regime-split annotation
  practice (the B64 `Mixed regime-split` class) without a new bounded
  mechanism; the cells corroborate, they do not seed a rule.

## Riders (recorded, never landed)

- `MVM_MIN_BLOCKS` launch-bounds register-cap mechanism
  (mvm_static.cuh `__launch_bounds__(MVM_THREADS, MVM_MIN_BLOCKS)`; the
  255-register GPT-pack kernel → 1 block/SM grid 108, README.md:332; the
  Doom build's `-maxrregcount=112`) — the gpu_44 register-envelope family
  already owns the axis; the verdict's "+40–60% matmul" figure was NOT
  verifiable in-tree at T0 and is NOT quoted anywhere.
- The ~0.7M tasks/s dispatch ceiling / sub-100 µs dispatch-bound tasks
  (README.md:332) — the gpu_07 occupancy-trap family's corroboration.
- gpu_86 static-ICB-replay + gpu_60 deadline-bounded-poll are the
  static/dynamic boundary neighbors of R5's dropped row.

## Gates (all run this session)

- Fixtures `corpus/fixtures/b400_fixtures.rs`: **16/16 standalone-compiled
  AND executed BEFORE entry** at debug AND -O, zero warnings. The gate
  caught **five arm defects** (four in the first run + one after the guard
  rework): the ids-vs-program-order conflation (the "already ordered"
  check compared task ids, not positions), a publishing-bundle admission
  wrongly gated on the parked budget (the window arithmetic excludes
  parked), a scan with a hardcoded `need == 1` (needs now ride the park
  list), the Held-borrow guard unrepresentable in safe Rust (reworked to
  the explicit held-flag protocol), and an ordering-unwrap sequencing bug
  (the edge must exist before the second publisher is admitted). The
  entries' ref/opt shapes were synced to the corrected models in the same
  session.
- lib tests **3960/0** (default; +3 batch-400 tests: rules-present-and-
  parse ×2 rules, concepts-covered needles ×2 rules, the rust_perf face
  test).
- clippy `-D warnings`: default lib, `--all-targets` (4 PRE-EXISTING
  ane/corpus/tests.rs B399 casts + an always-true assert in the M2/M7
  needle tests — NOT this batch's, disclosed), kernel_opt, rust_perf,
  rust_perf_bench, no-default-features — all clean at the lib posture.
- Retrieval floors BOTH lanes: DEFAULT `--features latent_retrieval
  retrieval_` **30/0** + `--features kernel_opt,latent_retrieval retrieval`
  **30/0** — held, no re-pin.
- fmt gate PASSED (921 files; one unpinned drift on router.rs found and
  formatted same session).
- pins_manifest --write: kernel_opt rules 1105→1107 (the ONLY manifest
  move — confirming rust_perf/ane/sec neutrality).
- Count web one-landing: AGENTS.md §Role kernel_opt head + README +
  `.docs/11_domains/kernel_opt.md` + health_views (kernel 1107, total
  1768) + the router total pin + the rust_perf narrative chains (the face
  rows, counts neutral).
- freeze/score-bench NOT owed (zero clippy_lints delta asserted).
- Binary rebuild NOT owed (corpus-only delta — no rulewright source
  touched).
- G1 verdict store regenerated over the full population
  (`kernel_goat_g1 --full --emit-status`): 1107 rows; **276 green / 4 FAIL /
  827 parse-only**. The 4 FAILs are PRE-EXISTING at HEAD (verified by a
  stash-run: identical 4 failures with this batch's changes stashed —
  `replay-group-eligibility-excludes-mutable-surface` +
  `patch-validity-against-declared-resource-contract` (B341 entries_gpu_153,
  E0433/parse-class in the opt arm), `accelerator-backend-first-tile-
  acceptance-gate` (B345 entries_gpu_154, `cannot find type Verdict`),
  `sah-tree-depth-bounded-by-traversal-stack` (B362 entries_gpu_157)) —
  the standing B354-disclosure class (their rows were ABSENT from the
  committed store since the last full run; the regeneration surfaces them
  as `fail` honestly). My 2 rules carry `parse-only` records (the fixture
  FILE is the executed evidence; the embedded-fixture G1 cascade is the
  follow-up lane the 4 pre-existing FAILs share).

## Dedup evidence (3+ vocabularies, all hits read in-row)

`reader.*publisher|publisher.*reader` → 0 kernel owners; `wait.*version` →
gpu_61/gpu_35/gpu_160 hits read and ruled (ring overwrite / function
constants / pool admission — no version-coherence edge); `park|PARKED` →
no kernel owner of waiter custody; `parked.*budget|live_cap` → gpu_50 +
entries_74 hits read and ruled (cap+overlap / write-class shares); epoch
tags → gpu_148 (cross-rank mailbox freshness, different axis).

## Teardown

`.raw/mvm` REMOVED at close (df recorded in the landing summary).

NOTHING INVENTED — every quote read at the clone this session; every
corpus claim a live grep this session; the 11-21 verdict row + riir-infer
Research 36 are the controlling records.
