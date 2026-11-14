# Issue 238 — the wide-posture lib suite takes 23 min: measure, then parallelize honestly

**Status:** OPEN — **Claimed:** zed-m3-ane-domain (Plan 377 landing session), epoch 1794670478, expires 1794843278 (48 h).

## The datum (measured, not guessed)

The ALL-compiled-retrieval-posture lib suite — defaults + `kernel_opt,sec,latent_retrieval,ane_pregate`
(8 domains + both pregates, the widest posture a landing verification realistically compiles) — on m3
(16 cores, AC, otherwise quiet):

- **1388.12 s wall** (23.1 min) for **4546 passed / 8 ignored**
  (`/tmp` run log, 2026-11-14, the Plan-377 landing verification posture).
- Arithmetic: 4546 tests / 1388 s on 16 threads ≈ 0.30 s/test average budget — but libtest
  ALREADY runs tests in parallel threads (default `RUST_TEST_THREADS` = core count), so a 23-min
  wall with that budget implies **strong skew or serial coupling**, not missing parallelism.
  A naive "add threads" is a non-lever: the threads are already there.

## Hypotheses (T1 falsifies; never presume)

- **H1 long-pole skew** — a small set of corpus-heavy tests dominates the wall: the
  `retrieval_eval` router/matcher sweeps, modelability oracle sweeps, score-bench generation
  freezes, the kernel category/router tests. Wall ≈ the longest few tests, not the sum.
- **H2 serial-coupled tests** — process-global state that serializes or would break under
  isolation: env-var readers (OnceLock process caches — `active_set`, `kernel_pregate`,
  `ane_pregate` demote envs, span-shape demote), the escalation seat's deliberate serial lane,
  `index_cache` writer-thread joins, fixed `temp_dir()` fixture names (the shared_temp_path
  class — these pass alone BY CONSTRUCTION today and race under any scheduler change).
- **H3 per-test fixed cost** — every test that calls `LatentMatcher::from_enabled_domains()`
  rebuilds the full matcher (8 corpora, 4500+ rules → seconds each); at hundreds of such tests
  the mass parallelizes but the total CPU is huge. Lever would be SHARING (a frozen base +
  cheap per-test views), not threads.

## Levers (attack in measured order)

- **L1 — measure first**: per-test wall times at the wide posture. Cleanest instrument:
  `cargo-nextest` (reports per-test times natively) or nightly `--report-time` from a /tmp
  toolchain. Output: the top-20 wall hogs + total serial mass, recorded HERE.
- **L2 — cargo-nextest as the dev/CI runner** (process-per-test): real per-test times,
  better scheduling than libtest's chunking, retries/quarantine. NO Cargo dep change (prebuilt
  tool binary; CI/dev-tool only). RISK (the honest half): process isolation re-pays every
  process-global cost and BREAKS env-var/static/fixture-sharing tests — the T2 audit gates it.
- **L3 — route the wide-posture suite to a quiet rig** (`agent-mail rig run --shared --est`,
  amo/bmo per the fleet law): zero code change; landing receipts quote the rig + PROVENANCE.
  This is the immediate relief for landing verification on m3, independent of L1/L2.
- **L4 — conditional (only if H3 dominates)**: a test-side matcher fixture — a frozen shared
  base built once + cheap per-test derivation. Careful: tests MUTATE matcher state
  (`with_active_set`, `with_rerank_mode`, `with_depth_policy`), so the design is
  clone-on-configure or a builder from a cached corpus snapshot — measured, never presumed.

## Non-goals

- No `--test-threads=1` band-aids, no test weakening, no skipped tests to make numbers move.
- No CI wiring before the measured win is recorded here (the GOAT bar applies to this too).
- L2 adoption never lands with a known-red quarantine set silently excluded — the excluded
  tests are named here with their coupling reason.

## Tasks

- [ ] T1 measure: per-test wall times at the wide posture; record top-20 hogs + serial mass + the parallel-efficiency ratio (wall vs total-test-time) in this issue.
- [ ] T2 nextest isolation audit: grep the suite for `env::set_var`/process singletons/fixed `temp_dir()` names/listeners; name every incompatible test + its coupling.
- [ ] T3 IF T1 shows ≥2× wall win AND T2's incompatible set is small: adopt nextest as the dev/CI runner (tool-level, no manifest change); name the quarantined set otherwise.
- [ ] T4 IF H3 dominates: design + measure the shared matcher fixture (frozen base, cheap views); GOAT the win before landing.
- [ ] T5 the standing lane: the wide-posture verification rides `agent-mail rig run --shared --est` off m3; landing receipts quote the rig. (Implementable immediately after Plan 377 lands — independent of T1–T4.)
