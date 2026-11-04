# sccache outbound lane — Day 2 record (2026-11-04)

Candidate: mozilla/sccache issue #2968 — backend-scoped read-timeout backoff.
Provenance class: **issue-driven** (not healer-emitted). The cargo refine
scan below is the campaign rulewright-provenance record only; no perf rule
emitted this fix. Owner-sanctioned via riir-refine plan 331 T6a.

## 1. Pin + feature set

- Pin: ba261b823392bf2020daef4ab86bd82165a60512 (origin/main tip 2026-11-04,
  "fix(clang): recognize -resource-dir"). Bench arm BEFORE = this pin,
  byte-identical worktree (/tmp/mozilla_day2/sccache-before).
- Branch: refine/backend-backoff — feature commit `135734f` + review-fix commit `3fad121` (G-PR2 round-1 fixes: the contact rule, the recovery-probe doc, the counter test). Clone
  /Users/kat/git/riir-refine/.raw/mozilla-heal/sccache. Bench arm AFTER = `135734f`; the review-fix commit touches only the recording condition (no benched-path change — see §12).
- Toolchain: 1.91.0 (rustc 1.91.0 f8297e35 2025-10-28) = their rust-version
  MSRV. All builds/tests/clippy/fmt at +1.91.0, --locked.
- Features: default = ["all"] — azure, cos, gcs, gha, memcached, oss, redis,
  s3, webdav, dist-client. IDENTICAL on both arms; no --features override
  needed (the benched backend webdav is default-on).
- Build: cargo +1.91.0 build --release --locked --bin sccache into isolated
  target dirs (/tmp/mozilla_day2/target-before, target-after). BEFORE
  finished 2m59s, AFTER 2m45s (first pass). AFTER binary verified to contain
  the new code (strings | grep SCCACHE_BACKEND_BACKOFF_THRESHOLD).

## 2. Rulewright scan (provenance capture)

RIIR_REFINE_AUTO_ORACLE=0 cargo refine --suggest <clone> ->
/tmp/mozilla_day2/scan_sccache.md.

- scan root: 91 source files · scanned 85 files · 1935 spans · 1935 with
  suggestions (top-3 per domain).
- Roll-up: Found clippy 75 | perf 75 | sec 27 | docker 7 | dist 2 — 186
  total; ~40% fixable via cargo refine fix (compile-gated; suggest-only
  rows stay manual).
- Secret scan: 27 secret-shaped literals in 9 files (+12 digest-pins
  adjudicated benign, suppressed; fail-closed sync gate).
- Perf findings are span-level routed suggestions (recorded in the scan
  file); none of them is the backoff mechanism — honest provenance split:
  scan = campaign record; fix = issue-driven.

## 3. The issue ask

mozilla/sccache#2968 (mitander, open): with SCCACHE_CACHE_READ_TIMEOUT_SECS=1
(the #2953 knob), each compilation still waits a fresh second on a backend
that is already timing out. Waits accumulate along the dependency graph and
recur in the same daemon session: stalled builds 9.019 s vs 0.773 s healthy
warm (11.7x), 32 cache timeouts per build, zero hits, 32 successful writes.
The author asks: "Would backend-scoped backoff be appropriate here ... I'd
like to understand the preferred behavior before proposing an
implementation." No implementation has been proposed in-thread. (A third
party commented 2026-11-03 claiming interest — collision risk for the send
decision, G-PR4.)

## 4. Design summary

Daemon-scoped gate at the cache-read seam; ~50 LOC of logic + 8 unit tests
(incl. the G-PR2-round-1 contact-rule test).

Seams (verified in source at the pin):
- Read path: src/compiler/compiler.rs get_cached_or_compile — the existing
  per-lookup tokio::time::timeout(cache_read_timeout(), storage.get(&key));
  on Elapsed -> MissType::TimedOut -> the compile proceeds locally. This is
  the seam the issue cost is paid at.
- Stats: src/server.rs — MissType::TimedOut => stats.cache_timeouts += 1.

Mechanics (new module src/compiler/backend_backoff.rs, declared private in
src/compiler/mod.rs):
- Daemon-scoped state in a LazyLock (the sccache daemon IS the process):
  consecutive_timeouts + open_until under a std::sync::Mutex.
- Gate closed: lookup proceeds; on completion Ok(_) (hit/miss/none/recache/
  fast backend error) -> record_response() resets the counter; Err(_)
  (real timeout elapsed) -> record_timed_out() increments; at threshold
  consecutive, the gate opens for cooldown.
- Gate open: storage.get is NOT called — the lookup future returns
  Err(LookupSkipped) (zero-sized marker, std::error::Error); a guard arm
  placed before the generic read-error arm maps it to the SAME
  MissType::TimedOut path, so the build still completes exactly as a
  timeout would, minus the wait. Skips are not data points (nothing
  recorded).
- Recovery: after cooldown expiry the gate reads closed; the next lookup is
  the recovery probe. Probe success resets; probe timeout re-trips
  immediately (counter already >= threshold).
- Env knobs, read ONCE at first use:
  - SCCACHE_BACKEND_BACKOFF_THRESHOLD — consecutive timed-out lookups to
    trip (default 3; 0 disables = the demote path; invalid -> warn +
    default).
  - SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS — skip window (default 60;
    invalid -> warn + default).
- Scope law: READ path only. PUTs are not gated — a skipped put would
  silently drop a cache write (server.rs awaits each cache-write future; a
  stalled put is pre-existing behavior, out of scope; the issue is
  read-timeout waits and the author fault keeps writes working).
- House style: rustfmt 2024 defaults, Apache header, log macros, no
  attribution, no tool names. No new deps (Cargo.lock unchanged; --locked
  stays valid).

## 5. Diff stat (vs pin)

 src/compiler/backend_backoff.rs | 297 ++++++++++++++++++++++++++++++++++++++
 src/compiler/compiler.rs        |  28 +++++-
 src/compiler/mod.rs             |   1 +
 3 files changed, 325 insertions(+), 1 deletion(-)

## 6. G-PR1 bench (the deliverable)

PROVENANCE (bench_preflight at bench time): power=AC Power load=3.14
swap=0.00M canary=skipped powermode=2(high) — PASSED. Box: M3 (this
workstation), High Power mode, AC; no exclusive hold (Unity/Zed exempt per
owner call); sibling load ~3/16.

Harness: /tmp/mozilla_day2/bench_backoff.sh (POSIX sh, self-contained;
becomes the bench-128 harness verbatim).

- Fake backend: python3 ThreadingHTTPServer on 127.0.0.1:<random port> —
  GET accepted and NEVER answered (blackhole; the read timeout fires
  against it), PUT/MKCOL/HEAD/DELETE answered immediately (201/200).
  Mirrors the issue fault: stalled GETs, working PUTs.
- Timeout config: SCCACHE_CACHE_READ_TIMEOUT_SECS=1 (the #2953 knob,
  cache_read_timeout() in compiler.rs); SCCACHE_SKIP_CACHE_CHECK=1;
  SCCACHE_IDLE_TIMEOUT=0; SCCACHE_WEBDAV_DISABLE_CREATE_DIR=true (the fake
  has no PROPFIND; without this opendal write-time parent probe fails
  every cache write — pilot-run finding, fixed before the record run);
  SCCACHE_WEBDAV_ENDPOINT=http://127.0.0.1:<port>/; random
  SCCACHE_SERVER_PORT per run.
- Workload: 32 DISTINCT tiny .c files, compiled sequentially through
  sccache with the real cc (sccache ships no standalone mock-compiler
  binary in tests/ — mock_command/mock_storage are in-process test utils).
  Fresh daemon per run (--start-server before the timed region,
  --stop-server after; startup/retirement excluded, port verified free).
  Fresh backend per run. Uncached control per invocation.
- Protocol: 3 alternating batches (B A B A B A) + one demote run; per-run
  wall time of the 32-request workload; per-run --show-stats JSON + text
  and backend GET/PUT counts from the fake log.
- Arm binaries: BEFORE = pin build; AFTER = branch build; identical
  feature set; --locked; release profile.

### Record run (benchrun2)

| run | arm | wall (s) | reqs | timeouts | writes | werrs | comps | GETs | PUTs |
|---|---|---|---|---|---|---|---|---|---|
| control | no sccache | 0.590 | – | – | – | – | – | – | – |
| 1 | before | 33.595 | 32 | 32 | 32 | 0 | 32 | 32 | 32 |
| 2 | after | 4.773 | 32 | 32 | 32 | 0 | 32 | 3 | 32 |
| 3 | before | 33.478 | 32 | 32 | 32 | 0 | 32 | 32 | 32 |
| 4 | after | 4.409 | 32 | 32 | 32 | 0 | 32 | 3 | 32 |
| 5 | before | 33.527 | 32 | 32 | 32 | 0 | 32 | 32 | 32 |
| 6 | after | 4.407 | 32 | 32 | 32 | 0 | 32 | 3 | 32 |
| 7 | demote (thr=0) | 33.528 | 32 | 32 | 32 | 0 | 32 | 32 | 32 |

Medians: BEFORE 33.527 s · AFTER 4.409 s · ratio 7.60x (means 33.533 vs
4.530 = 7.40x; worst-vs-best floor 33.478 / 4.773 = 7.01x).

- The author shape reproduces exactly on BEFORE: 32 timeouts, zero hits,
  32 successful writes, ~32 x 1 s timeout serially = 33.5 s (author
  measured 11.7x on a cargo -j4 graph; ours is a controlled sequential
  workload).
- AFTER pays exactly threshold = 3 real backend GETs (3 x 1 s timeouts),
  skips the remaining 29 lookups (zero backend contact), still compiles
  and stores all 32 entries. Expectation met: "threshold x timeout +
  cooldown skips".
- cache_timeouts reads 32 in BOTH arms — gate skips are REPORTED as
  timed-out lookups by design (same miss path; no stats wire change). The
  arm distinction is wall time + backend GETs (32 vs 3).

### Stability reps (benchrun3 — BLOCK-ORDERED: 3 before runs, then 3 after runs; not alternating)

before: 33.649 / 33.999 / 33.523 · after: 4.438 / 4.434 / 4.442 —
consistent with the record run (after-arm even tighter: 4.43-4.44 s).
Per-run evidence COMMITTED in the bench record (`benchrun3_stats/` —
show_stats JSONs + backend contact logs for before_1/after_1); the wall
lines were captured at the harness stdout (harness v1 did not persist
them per-run — disclosed, fix owed to `bench_backoff.sh` if the lane
re-benches). Text --show-stats captured for one before-run and one
after-run (benchrun3/run_before_1/show_stats.txt,
run_after_1/show_stats.txt): Compile requests 32 · Cache timeouts 32 ·
Compilations 32 · Cache read errors 0 · Cache writes 32 in both.

### Demote arm

SCCACHE_BACKEND_BACKOFF_THRESHOLD=0, AFTER binary: 33.528 s vs BEFORE
median 33.527 s (+0.03 %, noise); backend GETs 32 — the gate is fully
inert; identical code path (`lookup_blocked()` returns false before any
state access; the record functions early-return).

## 7. Gate outputs

- cargo +1.91.0 test --lib --locked — BEFORE: 624 passed / 3 failed;
  AFTER: 631 passed / 3 failed. Delta = +7 (the new module tests, all
  pass). The 3 failures are IDENTICAL by name on both arms
  (compiler::rust::test::test_hash_env_dep_equal_to_basedir_is_not_unset,
  server::tests::unix_socket_binding_preserves_live_listener,
  server::tests::unix_socket_binding_serializes_concurrent_startup) —
  pre-existing, environment-dependent on this box (verified on the
  pristine pin BEFORE touching anything).
- Full CI test lane cargo +1.91.0 test --locked --lib --bins --tests
  --no-fail-fast: lib 631/3 as above + all bins/tests targets GREEN
  (2+3+4+12+3+14 passed, 1 ignored) — zero new failures anywhere.
- Their exact CI clippy line (cargo +1.91.0 clippy --locked --all-targets
  -- -D warnings -A unknown-lints -A clippy::type_complexity
  -A clippy::new-without-default, from .github/workflows/ci.yml:19):
  ZERO warnings after fixing 4 findings in the new code (2x
  unnecessary_map_or -> is_some_and/is_none_or, 2x
  semicolon_if_nothing_returned). Evidence: `clippy_prefix.log` (the
  pre-fix run, 4 errors — the round-1 verdict caught that the committed
  log was the PRE-fix run) + `clippy_after.log` (the post-fix run at
  `3fad121`, ends `Finished`, zero warnings).
- cargo +1.91.0 fmt -- --check (their CI command, whole package): CLEAN.
- cargo check --no-default-features (their feature-matrix lane): PASS
  (6 pre-existing warnings, none touching the diff).
- cargo build -p randomize_readdir (their integration-helper lane): PASS.

## 8. G-PR1b harness (mechanized)

scripts/outbound_pr_ci_harness.sh <clone> 1.91.0 with BASE_REF=origin/main,
DIFF_WRITTEN_PATHS=backend_backoff -> log
/tmp/mozilla_day2/gpr1b_harness.log. Inventory it ran: build
-p randomize_readdir PASS · check --no-default-features PASS · clippy
--version SKIP (probe) · test --locked --lib --bins --tests FAIL rc=101 —
ADJUDICATED: the only failures are the 3 pre-existing environmental lib
tests (identical names on the pristine pin; proven before any edit);
bins/tests targets are green (verified with --no-fail-fast). Blast radius:
"(no unaccounted blast-radius hits)". Verdict: NOT READY on the raw rc;
green-for-the-diff after adjudication. Skipped by the harness parser
(their matrix uses cargo_cmd: templates its regex cannot see): the clippy
+ fmt lines — both run BY HAND above and green. Honest surface statement:
everything their CI runs locally-runnable was executed at 1.91.0 except
their cross-compile/nightly release lanes (not applicable to a local MSRV
gate; those lines target matrix triples with vendored openssl).

## 9. Honesty notes

- **The backoff is ON BY DEFAULT (threshold 3)** — a default build WITHOUT env takes the new gate path; this is a default-behaviour change the PR body must state plainly (`SCCACHE_BACKEND_BACKOFF_THRESHOLD=0` opts out; the demote arm measured ≈ BEFORE). Whether to offer an opt-in posture instead is a maintainer question to raise in the PR text, not a decision made here.
- Skips share the Cache timeouts stat label — --show-stats alone cannot distinguish a skip from a real timeout; backend request counts or debug logs do. A new MissType variant was considered and rejected (stats wire
  surface change; larger PR; upstream would need to want it).
- Only backend-CONTACTED lookups move the gate state: `ForceNoCache`/`ForceRecache` requests answer without touching the backend and gate-skipped lookups never run — neither resets the consecutive-timeout counter (`record_outcome`'s contact rule, tested: `uncontacted_lookups_do_not_reset_the_counter`; G-PR2 round-1 catch, fixed `3fad121`).
- Sequential workload (32 requests, one at a time) vs the author cargo -j4
  graph: sequential is deterministic and conservative — parallelism
  compresses the same saving into fewer wall seconds; the magnitude
  depends on the build graph (author: 11.7x on theirs).
- Pilot run (benchrun) had two harness defects, both fixed and disclosed:
  (a) driver looped 9+9 runs with appended backend logs (per-run deltas
  were exactly 32/3 — data consistent, medians 33.6 vs 4.4), (b) cache
  writes failed (cache_write_errors 32) because the fake lacked PROPFIND
  for opendal write-time parent probe — fixed via
  SCCACHE_WEBDAV_DISABLE_CREATE_DIR=true; record run shows writes=32 /
  write_errs=0, matching the author "32 successful cache writes".
- cache_timeouts=32 on the AFTER arm is designed reporting behavior, not a
  bug — a maintainer reading --show-stats still sees "32"; the wall time
  and the network contact count are the win. Worth an explicit sentence in
  any future PR body.
- MSRV release builds used isolated target dirs under /tmp (206 GB free);
  the clone ./target was populated only by the G-PR1b harness (their CI
  posture, default target dir).
- No push, no PR, no issue comments — records only, per the campaign gate.

## 10. PR-readiness checklist (five gates)

- G-PR0 provenance — READY: issue-driven (upstream #2968 asks for this
  exact mechanism; no implementation proposed in-thread), owner-sanctioned
  via plan 331 T6a, no attribution, no tool names in the diff.
- G-PR1 GOAT bench — READY: 7.60x median (33.527 s -> 4.409 s) at THEIR
  MSRV toolchain 1.91.0, pinned harness beside the record, box provenance
  quoted, author failure shape reproduced (32 timeouts / 32 writes),
  demote arm ~ BEFORE, +8 tests, clippy/fmt clean, identical feature sets.
  (Bench taken at `135734f`; the review-fix commit `3fad121` changes only
  the recording condition — see §12 — the reviewer explicitly judged the
  numbers stand.)
- G-PR1b harness — RUN, adjudicated: raw verdict NOT READY solely from the
  3 pre-existing environmental test failures (identical on pristine pin);
  every other runnable surface green; blast radius clean. (Harness ran at
  `135734f`; `3fad121` re-ran its own gates separately: their CI clippy
  line ZERO, fmt clean, lib 632/3 with the same 3 pre-existing names.)
- G-PR2 Claude verdict — **AGREE** (2026-11-04, round 3 of 3, session 6ab80747;
  the full 3-round arc in §12 below).
- G-PR3 SVG chart + G-PR4 owner fingerprint send — **PREPARED, ON HOLD (owner 2026-11-04: "do prepare all but wait for my go for PR, we need to wait for prev PR accepted first")**: the chart is built (`bench128_backend_backoff.svg` + renderer, attribution-ban-clean), the body is drafted (`pr_body.md`, every claim diff-verified), the artifact goes to the G-PR2 verdict for a body-freeze pass; the fork push rides the verdict AGREE; the send itself waits for (a) #2977's outcome — merged unblocks, closed-unmerged blocks under never-re-PR-after-close, and (b) the owner's go + `gh-outbound-pr` seal. Collision re-check 2026-11-04: no competing PR; darshpatel1052's question is answered in the body.

## 11. Files written

- Clone (branch refine/backend-backoff, commits `135734f` + `3fad121`):
  src/compiler/backend_backoff.rs (new), src/compiler/compiler.rs,
  src/compiler/mod.rs; worktree /tmp/mozilla_day2/sccache-before (pin);
  clone ./target/ populated by the G-PR1b harness only.
- /tmp/mozilla_day2/: scan_sccache.md, bench_backoff.sh,
  sccache_day2_results.md (this file), build_before.log, build_after.log,
  test_before_full.log, test_after_full.log, clippy_prefix.log,
  clippy_after.log, test_backend_backoff.log, bench_output.txt,
  gpr1b_harness.log, benchrun/ (pilot), benchrun2/ (record run),
  benchrun3/ (stability + text stats), target-before/, target-after/,
  target-check/, target-clippy/, target-test-before/, target-fix/.

## 12. G-PR2 round 1 verdict (2026-11-04, delegated session 6ab80747) — REVISE, all items applied

Verdict summary (verbatim intent): "The fix is sound and the 7.60× bench holds up when I re-derive it." REVISE on: (1) the committed clippy log was the PRE-fix run; (2) the §9 honesty note claimed default-build code-path-identical while DEFAULT_THRESHOLD=3 makes the backoff default-on; (3) a made-up name `gate_open()` in the demote explanation (real: `lookup_blocked()`); (4) the counter reset even when the backend was never contacted (ForceNoCache/ForceRecache flows) — plus should-fix: recovery-probe wording, benchrun3 not persisted + block-ordered not alternating, PR framing (lead with saved seconds, ratio is workload-specific). The reviewer independently re-ran their CI clippy line + fmt at the branch (both clean) and verified no lock held across .await, the skip-path map, the classification, and the G-PR1b adjudication.

Applied this session (commit `3fad121` + record edits):
- (1) fresh post-fix clippy run committed as `clippy_after.log`; old log renamed `clippy_prefix.log` (both in the record dir).
- (2) §9 note replaced with the default-on truth (see Honesty notes).
- (3) `gate_open()` → `lookup_blocked()` (§Demote).
- (4) the contact rule: `backend_backoff::record_outcome(contacted, outcome)` — only `!skip_lookup && cache_control == CacheControl::Default` lookups move the state; `LookupOutcome::{TimedOut, Answered}` classified at the call site; new test `uncontacted_lookups_do_not_reset_the_counter` (lib now 632 passed / 3 pre-existing failed). The dead free wrappers `record_timed_out()`/`record_response()` removed (clippy -D dead_code).
- Should-fix: module doc rewritten ("later lookups probe the backend again; each probe that times out re-arms the skip window ... one cooldown cycle can still pay up to one timeout per already-running lookup"); benchrun3 persisted (`benchrun3_stats/` JSONs + backend logs + `bench_output_stability.txt`, block-ordered disclosed); PR-framing note added to the README (lead with "29 of 32 read-timeout waits avoided (~29 s), 3 backend GETs instead of 32"; 7.60× is workload-specific).
- NOT changed: the bench arms — `135734f` was benched; `3fad121` touches only the recording condition (the reviewer judged the numbers stand; a fresh bench is owed only if the send re-opens after close-out).

### Round 3 (same session): **AGREE** — G-PR2 CLEARED (round 3 of 3)

The reviewer re-verified on origin/develop `b6697879`: the real `clippy_after.log` (ends `Finished`, zero warnings, differs from prefix), the renamed patch matches `git diff ba261b8 3fad121` byte-for-byte with no stale references, all round-2 code findings hold, and the bench carry-over stands (the record names `135734f` as the benched commit in §1/§6/§10/§12). **G-PR3 (SVG chart) is UNLOCKED; G-PR4 send remains owner-fingerprint-gated (owner: "later"), collision watch on #2968 stays open.** The PR-body laws are pinned in the README framing section (saved-seconds lead; default-on stated plainly + opt-in as a maintainer question; the cache_timeouts reporting behavior explained).

**Gate state: G-PR0 READY · G-PR1 READY · G-PR1b adjudicated · G-PR2 AGREE (3 rounds) · G-PR3/G-PR4 pending (owner-gated "later").**

### Round 2 (same session): REVISE — one blocker, fixed same hour

The reviewer verified every round-1 fix in the code at `3fad121` (contact guard, classification, the new test, no lock across .await, probe wording, record-matches-code) and ACCEPTED the no-re-bench call ("cache_control is Default on the benched path, so the condition reduces to `!skip_lookup` — the 135734f numbers carry over, provided the record keeps saying which commit was benched"). The ONE blocker: the committed `clippy_after.log` was byte-identical to `clippy_prefix.log` — a coordinator copy mistake (the fresh post-fix run was teed in /tmp but never re-copied into the record; the record still held the pre-fix log under the after name). FIXED: the real post-fix run at `3fad121` captured full-output straight into the record (`rc=0`, ends `Finished`, zero warnings), `diff -q` verified the two logs now differ. Should-fix also applied: the patch renamed to `backend_backoff_ba261b8_to_3fad121.patch` (it had named the wrong head) with references updated. Round 3 = the AGREE confirmation.
