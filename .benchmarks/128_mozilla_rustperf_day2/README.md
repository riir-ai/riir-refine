# Bench 128 — Mozilla day-2: the owner-directed full-domain `cargo refine` sweep + the sccache #2968 backend-backoff G-PR1 bench (**7.60× median**)

**Status:** RECORD — T6a-exec of [Plan 331](../../.plans/331_mozilla_outbound_pr_wetrun.md) (claim `0c3860f0b`, expires 2026-11-06T13:49Z). G-PR1 **PASS** (bench below), G-PR1b run + adjudicated, G-PR0 stated; **G-PR2 verdict is the next step** — this record is filed ready for it. Send (G-PR3 SVG + G-PR4 fingerprint) stays later per the owner's "pr/outbound-pr later" directive.
**Date:** 2026-11-04 · zed-m3-mozilla-day2 · M3 Max (macOS, **AC Power, powermode=2 HIGH, load 3.14**, swap 0.00M — `bench_preflight.sh` PASSED, quoted verbatim at the bench table)

**Owner directive executed this record** (2026-11-04, verbatim intent): "run `cargo refine` on the mozilla repos all over again — ALL domains, no perf filter, full report" + "for the real perf candidate waiting — run it now, report + bench md ready to verdict". Both halves landed.

---

## Part 1 — the full-domain sweep (the rulewright's own surface, all domains)

Method: `RIIR_REFINE_AUTO_ORACLE=0 cargo refine --suggest <clone>` per repo — parse-only, no builds, no clippy spawn; the rulewright's pattern-based surface at every compiled domain (rust_perf, clippy_lints, rustc_errors inert, docker, dist, sec scan). Binary: `cargo-refine 0.4.2`, build `cfb7f09d01d7` == HEAD (rust_perf compiled, release set complete). Fresh clones at the 2026-11-04 pins; sccache **moved** since the wetrun (71d107e0 → **ba261b8**), the other three unchanged. Full stdout committed beside this record (`scan_*.md`); sec findings adjudicated in-record below.

| repo | pin | roll-up (verbatim) | whole-tree spans | actionable perf |
|---|---|---|---|---|
| mozilla/sccache | `ba261b8` | `clippy 75 \| perf 75 \| sec 27 \| docker 7 \| dist 2 — 186 total` | 85 files · 1935 spans | **0** |
| mozilla/uniffi-rs | `ef9625b` | `clippy 75 \| perf 75 \| dist 15 \| sec 3 — 168 total` | 520 files · 3972 spans | **0** (1 shape-confirmed, cold path) |
| mozilla/mp4parse-rust | `d3a9abf` | `clippy 75 \| perf 75 \| dist 3 \| sec 2 — 155 total` | 22 files · 614 spans | **0** |
| mozilla/rust-code-analysis | `37e5d83` | `clippy 75 \| perf 75 \| dist 3 — 153 total` | 84 files · 826 spans | **0** (dormant — scan-only) |

⛔ **Measurement caveat (load-bearing for anyone citing the roll-ups):** the `clippy N | perf N` halves are **preview-capped citations** — 25 preview spans × top-3 per domain = 75 — NOT whole-tree counts (verified in `src/bin/heal.rs` `SPAN_PREVIEW_CAP`; the whole-tree signal is the span column above; `dist`/`sec` halves ARE whole-tree). Filed as **P115** in `.distill/001` §Post-mining intake (roll-up disclosure remedy candidates listed there).

**Triage (all 225+ perf citations read):**
- **sccache** — perf citations are span-level routed suggestions; none is a backoff-class or hot-loop finding on the moved tree. (Scan is the campaign provenance record — see Part 2's honest split.)
- **uniffi-rs** — 39 distinct perf rules; the one shape-confirmed site is `cow-conditional-alloc` @ `uniffi_bindgen/src/test_util.rs:31-34` (`library_name.replace("-", "_")`, the canonical unconditional-alloc shape) — but the site is a cold bindgen CLI helper: real-world gain ≈ 0, fails any upstream PR-value bar. Everything else: retrieval noise on `use` lines or placeholder-exemplar design laws.
- **mp4parse-rust** — all 25 preview spans are TEST files (walk starts in `mp4parse_capi/tests/`); even benchable-shaped laws land where the tool's own bench-guard declines perf edits. Zero candidates.
- **rust-code-analysis** — the huge-score citations (3929.84) track span size (a 127-line JS string-literal fixture), confirming score ≠ applicability; the tree's only shared lock is a once-per-file `Mutex<usize>` counter. Zero candidates. Dormant repo — correctly scan-only.
- **sec findings (whole-tree, all adjudicated):** sccache 27 (secret-shaped literals; scan file carries the detail run), uniffi 3 + mp4parse 2 = build-script strings / the public Apple "bipbop" test-vector keys — classic false positives, no action. rca 0 (3 digest pins benign).

**Verdict of Part 1:** the quartet is mechanically clean on the rulewright's pattern surface too — **0 actionable perf findings across all four repos**. This corroborates bench 127's clippy-clean-at-their-posture verdict from an independent surface. Honest consequence for Proposal 025: the perf/sec proof CANNOT ride mechanical healer-emitted fixes on these trees; it rides Part 2.

## Part 2 — the #2968 lane (the perf/sec proof)

**Provenance class (honest):** **issue-driven, not healer-emitted** — no perf rule emitted this fix. mozilla/sccache#2968 asks for exactly this mechanism; owner-sanctioned as the G-PR1-gated candidate via plan 331 T6a (the "corrected material hunt", owner verdict 2026-11-02). The cargo refine scan of sccache (Part 1) is captured as the campaign's rulewright-provenance record. The send class is governed by plan 331 T6a's four send gates; G-PR4 owner fingerprint remains the final authority.

**The fix:** branch `refine/backend-backoff` — feature commit `135734f` + review-fix `3fad121` — in `.raw/mozilla-heal/sccache` (LOCAL — never pushed; full patch in this record dir). Daemon-scoped consecutive-timeout gate at the cache-read seam (`src/compiler/compiler.rs` `get_cached_or_compile`, the exact `tokio::time::timeout(cache_read_timeout(), storage.get(&key))` site the issue's cost is paid at):

- N consecutive backend read-timeouts (default 3; `SCCACHE_BACKEND_BACKOFF_THRESHOLD`, 0 = demote) open the gate for a cooldown (default 60 s; `SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS`).
- Gate open: `storage.get` is NOT called — returns the same `MissType::TimedOut` path (build completes identically, minus the wait). Any backend success resets. After expiry, later lookups probe the backend again; each probe that times out re-arms the window.
- **Only backend-CONTACTED lookups move the state** (`record_outcome`'s contact rule — `ForceNoCache`/`ForceRecache` and gate-skipped lookups never reset the counter; tested).
- Read path only (PUTs deliberately not gated — a skipped put would drop a cache write). Env read once; invalid values warn + default. No new deps (`Cargo.lock` untouched, `--locked` stays valid). No attribution, no tool names.
- `3 files changed, +325/−1` — `src/compiler/backend_backoff.rs` (+297, incl. 8 unit tests) · `compiler.rs` (+28/−1) · `mod.rs` (+1).

### The G-PR1 bench (record run `benchrun2`, harness `bench_backoff.sh` committed verbatim beside this README)

**PROVENANCE: `power=AC Power load=3.14 swap=0.00M canary=skipped powermode=2(high)` — bench_preflight PASSED.** Toolchain **1.91.0** (their `rust-version` MSRV), `--locked`, identical feature sets (default `["all"]` — webdav backend default-on), release profile, isolated target dirs.

- **Fault injection mirrors the issue exactly:** fake WebDAV backend = python3 blackhole (GET accepted, NEVER answered → the 1 s `SCCACHE_CACHE_READ_TIMEOUT_SECS` fires; PUT/HEAD answered immediately → "stalled GETs, working PUTs", the author's fault shape).
- **Workload:** 32 distinct tiny `.c` compiles sequentially through a fresh sccache daemon per run (startup/retirement excluded, port verified free); fresh backend per run; uncached control per invocation.

| arm | wall of 32-request workload (3 reps) | **median** | timeouts | writes | backend GETs | PUTs |
|---|---|---|---|---|---|---|
| uncached control (no sccache) | 0.590 s | 0.590 s | – | – | – | – |
| **BEFORE** (pin `ba261b8`) | 33.595 / 33.478 / 33.527 | **33.527 s** | 32 | 32 | 32 | 32 |
| **AFTER** (`135734f`) | 4.773 / 4.409 / 4.407 | **4.409 s** | 32 (3 real + 29 skips) | 32 | **3** | 32 |
| **DEMOTE** (AFTER, `THRESHOLD=0`) | 33.528 | 33.528 s (≈ BEFORE, +0.03%) | 32 | 32 | 32 | 32 |

**Ratio: 7.60× median** (means 7.40×; worst-vs-best floor 7.01×). Stability pair (`benchrun3`, block-ordered — see its file): before 33.649/33.999/33.523 · after 4.438/4.434/4.442 — consistent, after-arm tighter.

**PR framing law (G-PR2 round-1 should-fix, adopted):** lead with the quantity that carries over — "29 of 32 read-timeout waits avoided (~29 s saved; 3 backend GETs instead of 32)" — and present 7.60× as SPECIFIC TO THIS WORKLOAD (tiny compiles, 1 s read timeout; the author's own 11.7× on their cargo −j4 graph stays the carry-over context).

- **The author's shape reproduces exactly on BEFORE:** 32 timeouts, zero hits, 32 successful writes, ~32 × 1 s serial = 33.5 s (author measured 11.7× on their cargo −j4 graph; ours is the controlled sequential workload — conservative, parallelism compresses the same saving into fewer wall seconds).
- **AFTER pays exactly threshold=3 real backend GETs** (3 × 1 s), skips the remaining 29 with **zero backend contact**, still compiles and stores all 32 entries. Expectation ("threshold × timeout + cooldown skips") met.
- ⚠ `cache_timeouts` reads 32 in BOTH arms — **by design**: skips share the `MissType::TimedOut` path (no stats wire change; a maintainer's `--show-stats` still says 32). The arm distinction is wall time + backend contact. **This sentence belongs in the future PR body.**

### Gates (all at their 1.91.0 MSRV, --locked)

| gate | verdict |
|---|---|
| `cargo +1.91.0 test --lib --locked` | BEFORE 624 passed / 3 failed → feature `135734f`: 631/3 → **review-fix `3fad121`: 632 passed / 3 failed** — delta = **+8** module tests (all green, incl. the new contact-rule test); the 3 failures are **identical by name on the pristine pin** (unix-socket + hash-env tests, environment-dependent, proven before any edit) |
| full lane `test --lib --bins --tests --no-fail-fast` | all bins/tests targets GREEN (2+3+4+12+3+14 passed) — zero new failures anywhere |
| their exact CI clippy line (`ci.yml:19`) | **ZERO warnings** at `3fad121` (evidence: `clippy_after.log` post-fix run; `clippy_prefix.log` = the honest pre-fix run with 4 findings, kept); fixed in new code: 2× `unnecessary_map_or`, 2× `semicolon_if_nothing_returned`, then 2 dead free wrappers after the recording refactor |
| `cargo +1.91.0 fmt -- --check` | **CLEAN** |
| `cargo check --no-default-features` (their feature-matrix lane) | PASS (6 pre-existing warnings, none in the diff) |
| G-PR1b `outbound_pr_ci_harness.sh` (log committed: `gpr1b_harness.log` → `/tmp/mozilla_day2/`, verdict quoted here) | raw NOT READY **solely** from the 3 pre-existing environmental lib failures — **adjudicated green-for-the-diff**; blast radius "(no unaccounted blast-radius hits)"; the harness parser misses their matrix-template clippy/fmt lines — both run BY HAND above, green. Honest surface: everything locally-runnable at MSRV was executed |

### PR-readiness (five gates)

| gate | status |
|---|---|
| G-PR0 provenance | READY (stated: issue-driven, owner-sanctioned T6a; no attribution/tool names) |
| G-PR1 GOAT proof | **READY — this bench** |
| G-PR1b blast-radius harness | RUN + adjudicated |
| G-PR2 Claude verdict | **ROUND 1 = REVISE (2026-11-04)** — verdict re-derived the bench and found it sound; items: stale clippy log, a false "default-identical" honesty note (the backoff is DEFAULT-ON), a made-up fn name, the counter-reset contact bug, + 3 wording/persistence fixes. **All applied (`3fad121` + record edits, results file §12). Round 2 re-submission next** |
| G-PR3 SVG + G-PR4 fingerprint send | PENDING (owner: "later"); **collision watch: a third party claimed #2968 on 2026-11-03 — if the verdict AGREEs, move promptly** |

## Send queue

| # | repo | branch | status |
|---|---|---|---|
| 1 | mozilla/sccache (the #2968 PR-to-be) | `refine/backend-backoff` @ `135734f`+`3fad121` (LOCAL, not pushed) | **G-PR1 READY — G-PR2 round 2 pending** |
| — | uniffi-rs / mp4parse / rca | none | Part-1 verdict: nothing to send (PASS — the gate worked) |

**Campaign ledger:** unchanged — 1 sent (#2977, goodwill, outside the denominator), **0 evidence-bearing**. #2968 is the first evidence-bearing candidate; it does not enter the ledger until sent.

## Honesty notes / lessons

- ⚠ **The backoff is ON BY DEFAULT** (threshold 3) — a default build without env takes the new gate path; this is a default-behaviour change the PR body must state plainly (`THRESHOLD=0` opts out; demote measured ≈ BEFORE, +0.03%). The G-PR2 round-1 verdict caught the record's original false claim here.
- Pilot run (`benchrun/`) had two harness defects — both disclosed and fixed before the record run: driver loop shape; the fake backend missing PROPFIND broke opendal cache writes (`SCCACHE_WEBDAV_DISABLE_CREATE_DIR=true` fixes; record run shows writes=32/werrs=0, matching the author's "32 successful cache writes"). Harness v1 also did not persist per-run wall lines in benchrun3 (block-ordered reps; wall numbers agent-captured, stats JSONs committed) — owed to the harness if the lane re-benches.
- The default-on note doubles as the PR posture question: whether upstream prefers opt-in is a maintainer conversation, opened in the PR text, never decided unilaterally.
- Sequential workload = deterministic + conservative vs the author's parallel graph; the PR leads with "29 of 32 waits avoided (~29 s), 3 GETs instead of 32" and keeps 7.60× workload-scoped (G-PR2 framing law).
- Roll-up preview-cap discovery → **P115** (`.distill/001`); the cross-repo scan methodology (RIIR_REFINE_AUTO_ORACLE=0, parse-only) is now the standing campaign scan posture.
- P113's upstream-state check was applied this time: #2968's mechanism verified ABSENT at the pin before implementing (the issue is open, no in-thread implementation).

## Files (this record dir — self-contained, /tmp may be cleaned)

- `bench_backoff.sh` — the pinned harness, verbatim (POSIX sh, self-contained).
- `backend_backoff_ba261b8_to_135734f.patch` — **the full fix as a patch (now `ba261b8..3fad121`, +325/−1)** — the durable copy of the branch's diff, so the ephemeral clone law can never orphan the send material; `git apply` on a fresh pin checkout reconstructs the branch.
- `bench_output.txt` — the raw per-run log of the record run (all 7 rows, verbatim).
- `clippy_prefix.log` / `clippy_after.log` — the pre-fix clippy run (4 findings, the round-1 catch) and the post-fix zero-warning run at `3fad121`.
- `bench_output_stability.txt` + `benchrun3_stats/` — the block-ordered stability reps with per-run show_stats JSONs + backend contact logs (committed per the round-1 should-fix).
- `gpr1b_harness.log` — the G-PR1b harness output (run at `135734f`; `3fad121` re-ran clippy/fmt/lib separately).
- `sccache_day2_results.md` — the lane agent's full record (§1–§11 + §12 the G-PR2 round-1 verdict and the applied fixes).
- `scan_sccache.md` · `scan_uniffi-rs.md` · `scan_mp4parse-rust.md` · `scan_rust-code-analysis.md` — the four complete scan stdout captures (Part 1's evidence).
- Clone + branch state: `.raw/mozilla-heal/sccache` @ `refine/backend-backoff` `135734f` (ephemeral per the campaign law — re-clone at the pin + re-apply if the send re-opens after close-out; the diff is reconstructible from the results file §4 + the branch, which is NOT pushed anywhere yet — **the branch lives only on this box until G-PR4**).
