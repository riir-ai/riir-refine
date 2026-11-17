# Issue 238 — the wide-posture lib suite takes 23 min: measure, then parallelize honestly

**Status:** CLOSED 2026-11-17 — T1 DONE (H1 confirmed, the wall was one 1140 s headroom_probe test; T3/nextest REFUTED; T2 refuted-prerequisite); T4 DONE (the pool-size derivation: wide suite wall 990 → 217 s, 4.5×); T5 DONE (the rig-routed standing lane)
**Lease closed:** zed-amo-i238-t4, epoch 1794877029, was expires 2026-11-19T00:57:09Z — T4 landed the same session (the pool-size derivation + guards; record in the T4 row)
**Lease closed:** zed-amo-i238-t1, epoch 1794872400, was expires 2026-11-19T10:00:00Z — T1 landed the same session (measurement + verdicts recorded in the task rows below)
**Lease closed:** zed-m3-ane-domain (Plan 377 landing session), epoch 1794670478, was expires 2026-11-16T15:34:38Z (expired unexecuted — the filing session moved on; re-claimed for T1)

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

- [x] T1 measure: per-test wall times at the wide posture; record top-20 hogs + serial mass + the parallel-efficiency ratio (wall vs total-test-time) in this issue. **DONE 2026-11-17 (amo idle lane, nightly `-Z unstable-options --report-time`, isolated `CARGO_TARGET_DIR=/tmp/i238-target`, defaults + `kernel_opt,sec,latent_retrieval,ane_pregate`):** two runs 982.1 s / 989.9 s wall (4569 passed / 8 ignored — vs the m3 datum 1388 s; different box, see provenance). ⚠ PROVENANCE: amo = M5 Ultra 30-core, AC, load ~2.5–3, with the sibling i1186 quantize-capture shared jobs RUNNING throughout — the numbers are a SHARE DISTRIBUTION (box-independent structure), not absolute wall bars; a quiet-box rerun would tighten the absolutes only. 4567 timed rows, total test-time **4219 s**; wall/total = **0.23**. **H1 CONFIRMED, overwhelmingly:** the top-12 tests carry **94.1%** of all test-time (3970 s of 4219 s); top-20 = 95.6%. Module buckets: `headroom_probe` **3034.8 s (72%)**, `retrieval_eval` **961.3 s (23%)**, everything else ~5%. The wall is bound by ONE test — `draft::headroom_probe::tests::span_source_defaults_delegate_to_the_fixture_set` at **1139.9 s** (next: `t1_headroom_across_rerank_modes` 642 s, `t1_headroom_probe_is_deterministic` 479.7 s, `t1_headroom_decomposition_report` 478.8 s, `retrieval_sweep_report` 344.2 s) — while the other 4,555 tests sum to **~249 s** and hide entirely inside the long poles' tail. **H3 REFUTED as the wall driver** (`latent_matcher` per-test matcher rebuilds total 58.6 s across the whole suite); **H2 not binding** (efficiency 0.23 is fully explained by the long poles, no serialization signature). **Structural floor:** with the longest test in-suite, NO scheduler takes the wall below ~1140 s — the wall is a fixture-cost problem, not a scheduling one.
- [-] T2 nextest isolation audit: REFUTED-PREREQUISITE by T1 — the audit only gates T3's adoption and T3 has no wall win to adopt (below). Re-opens only if nextest is adopted for other reasons (per-test reporting, retries).
- [-] T3 IF T1 shows ≥2× wall win AND T2's incompatible set is small: adopt nextest as the dev/CI runner (tool-level, no manifest change); name the quarantined set otherwise. **REFUTED by T1:** the wall is one 1140 s test; process-per-test scheduling cannot beat libtest's overlap of the long poles with everything else (libtest's 990 s wall is ALREADY better than nextest's ~1140 s floor). A runner swap moves nothing.
- [x] T4 IF H3 dominates: design + measure the shared matcher fixture (frozen base, cheap views); GOAT the win before landing. **RESHAPED by T1 — H3 does not dominate, but H1's fixture cost does; DONE 2026-11-17 (`src/draft/headroom_probe.rs`, the pool-size derivation):** the lever was INSIDE the fixture eval, not across tests — `suggest_all_domains` retrieves `max(top_k, RERANK_POOL)` candidates, so all 8 depths ≤ RERANK_POOL share the identical retrieval+mark+rerank pool and the depth only truncates the reranked list; the probe now issues ONE query per pool size (production pool + one per deeper rung: 10 queries/span → 3) and derives each depth ≤ RERANK_POOL by `canonical_rank` over the truncated prefix (the per-depth shape, byte-exact — canonicalization stays AFTER truncation, never over one deep ranked list). Cross-test eval sharing was REJECTED on guard grounds: the T0.0a delegation + determinism tests compare independent computations, and a cache would vacate them (test weakening is a non-goal). `demonstrability_gate` keeps the per-depth loop (its per-domain depth scaling makes the truncation count a scaled quantity the derivation does not replicate). **Measured A/B (amo, AC, load ~6–8 with the sibling i1186 cq3_quantize cores busy throughout — both arms same box same window):** wall test `span_source_defaults_delegate` 1011.6 s → 210.6 s (4.8×); full wide-posture lib suite **982–990 s (T1) → 217.05 s (4.5×), 4570 passed / 0 failed** (+1 passed = the new guard, +1 ignored = the one-off — counts consistent); per-eval cost 169 s → 38.5 s (linear across all four tests: 231/6, 115/3, 77/2). G1: one-off full fixture-set derived-vs-per-depth equality PASS (blended, per-domain rows, every miss rank+chosen+kind, skipped census); standing guard `span_source_depth_derivation_matches_per_depth_queries` (one injected span, ~2 s, every posture) pins the law; clippy `-D warnings` at default + wide + `demonstrability_gate` + `--all-features`. The suite floor moved to the parallel pair span_source_defaults (6 evals, 210 s) / `retrieval_sweep_report` (210 s, retrieval_eval — the NEXT fixture-economics lever if the fleet wants more; its sweep shape is the same class, unexamined here).
- [x] T5 the standing lane: the wide-posture verification rides `agent-mail rig run --shared --est` off m3; landing receipts quote the rig. (Implementable immediately after Plan 377 lands — independent of T1–T4.) **LANDED 2026-11-17 (`5c870590`): the convention lives in AGENTS.md Build Commands.** T1's numbers STRENGTHEN it: with one 19-minute test in-suite, a 16-core m3 landing verification can never be quick; route the wide posture to a quiet rig and keep the m3 lane for the default suite only.
