# Bench 128 — Mozilla day-2: the owner-directed full-domain `cargo refine` sweep + the sccache #2968 backend-backoff G-PR1 bench (**7.60× median**)

**Status:** RECORD — T6a-exec of [Plan 331](../../.plans/331_mozilla_outbound_pr_wetrun.md) (claim `0c3860f0b`, expires 2026-11-06T13:49Z). G-PR1 **PASS** (7.60×), G-PR1b adjudicated, G-PR0 stated, **G-PR2 AGREE ×2** (fix+record: session 6ab80747, 3 rounds; body+chart: session 806b2620, 2 rounds — artifact FROZEN). G-PR3 built. **SEND ON OWNER HOLD** (2026-11-04: "prepare all but wait for my go… wait for prev PR accepted first") — the #2968 send sequences behind PR #2977's outcome; `gh-outbound-pr` seal + the go are the owner's remaining acts.
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

**PR framing law (G-PR2 round-1 should-fix, adopted):** lead with the quantity that carries over — "29 of 32 read-timeout waits avoided (~29 s saved; 3 backend GETs instead of 32)" — and present 7.60× as SPECIFIC TO THIS WORKLOAD (tiny compiles, 1 s read timeout). The issue's own 11.7× is a DIFFERENT quantity (stalled build 9.019 s vs healthy warm 0.773 s — stalled-vs-healthy; corrected from the earlier "cargo −j4" gloss, which the live issue does not contain) and the body carries that corrected framing.

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
| G-PR2 Claude verdict | **AGREE (2026-11-04, round 3 of 3; session 6ab80747)** — round 1 REVISE (bench re-derived sound; 4 must-fix + 3 should-fix: stale clippy log, false default-identical note, made-up fn name, counter-reset contact bug, probe wording, benchrun3 persistence, PR framing) · round 2 REVISE (one coordinator copy blocker: the after-log) · round 3 **AGREE**. Full arc in results file §12. **G-PR3 unblocked — send (G-PR4) owner-gated "later"** |
| G-PR3 SVG chart | **BUILT (2026-11-04)** — `bench128_backend_backoff.svg` (renderer `render_pr_svg.py` committed beside it; Plan-224 shape, attribution-ban-clean: no tool names/links anywhere). Rides the PR body via a raw gist URL at send time. Verdict-checked (layout: demote label clears the bars) |
| G-PR2 verdict (body + chart freeze) | **AGREE (2026-11-04, round 2, session 806b2620)** — round 1 REVISE caught: the 11.7× comparison is stalled-vs-healthy in the issue (different quantity from ours — reframed), "-j4" had no source (removed, live-issue-verified), "their CI" wording, + design-first acknowledgment, `Addresses` over `Fixes`, chart "(1 run)" + "setup" labels. Optional polish taken: trailing "Happy to" line cut (the body keeps the two substantive ones). **The artifact is FROZEN for the owner-gated send** |
| G-PR4 fingerprint send | **DEFERRED — owner hold 2026-11-04: "wait for my go for PR, we need to wait for prev PR accepted first" — ⚑ **THE HOLD'S PRECONDITION FIRED 2026-11-07: #2977 MERGED** (12:31:11Z, `907230be`, verified via API + sccache clone log). The send is UNBLOCKED pending the owner's go + the `gh-outbound-pr` seal (not yet in kat-secret). ⚠ Verdict round-1 (session ac57c17f): main is **21 commits** past the bench pin `ba261b8`, several bench-load-bearing (RemoteStorage lazy-create `31f3c62`, hickory→system resolver `d9797a7`, fifo-jobserver `10be8c8`/`8ccae55`) — none touch the backoff's seams (`compiler.rs`/`mod.rs`; the `Storage` import re-resolves via `cache/mod.rs:53`), MSRV still 1.91.0, but the FULL gate set re-runs at `c1d7204` including the BENCH itself with a fresh pristine-main failure baseline (the env-dep test rewrites `eb052a4`/`951a4ed` may have moved the 3 pre-existing failures; lib delta is +8), and the FROZEN body/SVG get a G-PR2 delta re-check if numbers move. The real CI clippy line (`ci.yml:19`) carries `-- -D warnings`; re-run at `c1d7204` with the CI lint SET (same `-A` allows after `--`), VERIFIED REAL (369 KB compiler-message JSON, 1 build-finished success:true) = **0 warnings — post-merge main is clean at their CI posture**; the 24 default-posture findings mapped per-lint: 13 unique, all newer-stable lints (`double_must_use` ×7, `nonstandard_macro_braces` ×4, `renamed_and_removed_lints`), none fire at 1.91.0, zero allowed-class hits, zero both-posture (see bench 128). Owner rulings owed WITH the go: (i) one-PR-per-repo exception (#2977 goodwill but maintainer burden real); (ii) G-PR0 ledger status — evidence only via explicit exception, else goodwill #2 (022 G-PR1: "#2977 was the last" goodwill). Collision re-check 2026-11-07: #2968 still OPEN, unassigned, 1 comment (11-03), no competing PR; adjacent PR #2965 (redis IO timeouts) noted, not competing — re-check again at push time (22 upstream commits landed in 2 days). Original 11-04 adjudication: merged → go possible; closed unmerged → the never-re-PR-after-close rule blocks; repo-block review = owner call. `gh-outbound-pr` token NOT yet sealed (only `gh-release-pat` in kat-secret) — the owner seals at go time. |

## Send queue

| # | repo | branch | status |
|---|---|---|---|
| 1 | mozilla/sccache (the #2968 PR-to-be) | `refine/backend-backoff` @ `135734f`+`3fad121` (fork push after the body verdict; not yet pushed) | **SEND-READY, UNBLOCKED 2026-11-07 — #2977 merged, the owner hold's precondition fired; awaiting the owner go + the `gh-outbound-pr` seal. ⚠ VERDICT ROUND-1 CORRECTION (session ac57c17f): the bench pin is **21 commits** behind main (not two) and the 21 include bench-load-bearing changes (RemoteStorage lazy-creation, hickory→system resolver, fifo-jobserver passthrough) — a FULL re-verify + RE-BENCH at `c1d7204` is REQUIRED before push: fresh pristine-main failure baseline (the env-dep test rewrites may have changed the 3 pre-existing names), patch applied, MSRV lib (+8 delta), the real CI clippy line (`ci.yml:19`, incl. `-- -D warnings`), fmt, G-PR1b, the bench itself; the FROZEN body/SVG re-check at G-PR2 if any number moves (threshold rule decided pre-run). Send flow: re-clone at main tip, re-apply the durable patch, re-verify, push, PR (the frozen body + SVG ride). Owner rulings owed WITH the go: (i) one-PR-per-repo exception (#2977 was goodwill but maintainer burden is real); (ii) G-PR0 ledger status — evidence-bearing only via explicit exception, else goodwill #2 outside the denominator (022 said "#2977 was the last" goodwill send)** |
| — | uniffi-rs / mp4parse / rca | none | Part-1 verdict: nothing to send (PASS — the gate worked) |

**Campaign ledger:** unchanged — 1 sent (#2977, goodwill, outside the denominator), **0 evidence-bearing**. #2968 is the first evidence-bearing candidate; it does not enter the ledger until sent.

## ⚠ STALENESS CORRECTION (2026-11-07, the round-1 verdict of the re-verify lane, session ac57c17f): the bench pin `ba261b8` is **21 commits** behind main `c1d7204`, not two — the send-flow note elsewhere in this README said "main moved past the bench pin … rebase check" and understated it. The 21 commits include bench-load-bearing changes: `31f3c62`/`ee56747` (RemoteStorage created only when a remote cache is used — how the WebDAV backend is wrapped behind `storage.get`), `d9797a7` (reqwest drops hickory-dns for the system resolver — the WebDAV HTTP client), `10be8c8`/`8ccae55`/`7933de5`/`b2c2346` (local compiles get the build's fifo jobserver — the per-request compile path), `e715999` (GCS service-account token exchange), plus the env-dep/basedir test rewrites (`eb052a4`, `951a4ed`+revert `8edcbe9`) that may change the pre-existing failure baseline. None touches `compiler.rs`/`compiler/mod.rs` (the patch applies; `Storage`'s move to `src/cache/storage.rs` re-resolves through `cache/mod.rs:53`'s re-export) and MSRV stays 1.91.0 — but **the bench MUST re-run at `c1d7204` with a FRESH pristine-main BEFORE baseline** ("3 failures, same names" no longer assumed; lib delta is +8 per this record, not +3), and the FROZEN body/SVG need a G-PR2 delta re-check if the numbers move (decide the threshold rule before the run). The clippy-line transcription in this record's gates table was missing `-- -D warnings` (the results file §198 has the real line: `cargo +1.91.0 clippy --locked --all-targets -- -D warnings -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default`, from `ci.yml:19`). ⚠ **VERDICT ROUND-2 GREEN-ZERO CATCH (session ac57c17f): the first 11-07 post-merge clippy attempt was INVALID — `-A` flags before `--` made cargo refuse the command before any compilation, stderr went to /dev/null, and the empty JSON read as "0 warnings" (the exact loud-zero class). RE-RUN PROPERLY (21:1x, same evening): `rustup run 1.91.0 cargo clippy --locked --all-targets --message-format=json -- -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default` at `c1d7204` (the CI lint SET; `-D warnings` is their severity half and only moves findings from warning→error, so the warning count is the faithful equivalent) — VERIFIED REAL THIS TIME: stderr captured, 369 KB of `compiler-message` JSON, exactly 1 `"reason":"build-finished"` with `"success":true`, **0 warning-level messages**. Post-merge main is CLEAN at their CI posture. Per-lint mapping for the 24 default-posture findings (the round-1 caveat, delivered): a plain-oracle-line run at stable (`--workspace --no-deps --all-targets`, 810 KB json, build-finished success) yields exactly **13 unique findings**: `double_must_use` ×7 (storage.rs:42, compiler/c.rs:195, compiler.rs:85/151/549, dist/mod.rs:729, mock_command.rs:62+84), `nonstandard_macro_braces` ×4 (test/utils.rs:34/41/48, tests/system.rs:91), `renamed_and_removed_lints` ×1 (a renamed-lint advisory), plus `type_complexity`-class zero — cross-checked against the MSRV-posture run: ALL 13 are stable-only (none fire at 1.91.0), zero CI-allowed classes present, zero fire on both postures → **every default-posture finding is a newer-stable lint their MSRV CI cannot even emit; classification proven per-lint, not by count-matching.** The uniffi/mp4parse 11-07 "0 oracle-anchored" rows came from the refine oracle (valid runs); the rca 6 came from a valid direct clippy run (no `-A` before `--`); unaffected by the green-zero. Also noted by the verdict: upstream PR #2965 ("redis: add configurable IO timeouts") is adjacent, not competing; re-check collisions immediately before push (22 upstream commits landed in 2 days). **Rule rulings the owner must give with the go (round-1 verdict): (i) the one-PR-per-repo exception — #2977 was goodwill but the rule exists to protect maintainer burden; (ii) G-PR0 ledger status — #2968 is issue-driven/hand-written ("no perf rule emitted this fix"), so it enters the accept-rate denominator ONLY with an explicit G-PR0 exception, or ships as goodwill #2 outside it (022 G-PR1 said "#2977 was the last" goodwill send — the owner's call to amend).**

## Honesty notes / lessons

- ⚠ **The backoff is ON BY DEFAULT** (threshold 3) — a default build without env takes the new gate path; this is a default-behaviour change the PR body must state plainly (`THRESHOLD=0` opts out; demote measured ≈ BEFORE, +0.03%). The G-PR2 round-1 verdict caught the record's original false claim here.
- Pilot run (`benchrun/`) had two harness defects — both disclosed and fixed before the record run: driver loop shape; the fake backend missing PROPFIND broke opendal cache writes (`SCCACHE_WEBDAV_DISABLE_CREATE_DIR=true` fixes; record run shows writes=32/werrs=0, matching the author's "32 successful cache writes"). Harness v1 also did not persist per-run wall lines in benchrun3 (block-ordered reps; wall numbers agent-captured, stats JSONs committed) — owed to the harness if the lane re-benches.
- The default-on note doubles as the PR posture question: whether upstream prefers opt-in is a maintainer conversation, opened in the PR text, never decided unilaterally.
- Sequential workload = deterministic + conservative vs the author's parallel graph; the PR leads with "29 of 32 waits avoided (~29 s), 3 GETs instead of 32" and keeps 7.60× workload-scoped (G-PR2 framing law).
- Roll-up preview-cap discovery → **P115** (`.distill/001`); the cross-repo scan methodology (RIIR_REFINE_AUTO_ORACLE=0, parse-only) is now the standing campaign scan posture.
- P113's upstream-state check was applied this time: #2968's mechanism verified ABSENT at the pin before implementing (the issue is open, no in-thread implementation).

## Files (this record dir — self-contained, /tmp may be cleaned)

- `bench_backoff.sh` — the pinned harness, verbatim (POSIX sh, self-contained).
- `backend_backoff_ba261b8_to_3fad121.patch` — **the full fix as a patch (`ba261b8..3fad121`, +325/−1)** — the durable copy of the branch's diff, so the ephemeral clone law can never orphan the send material; `git apply` on a fresh pin checkout reconstructs the branch.
- `clippy_after.log` — **the real post-fix run at `3fad121`** (rc=0, ends `Finished`, zero warnings; captured full-output, verified to differ from the prefix log after the round-2 catch).
- `bench_output.txt` — the raw per-run log of the record run (all 7 rows, verbatim).
- `clippy_prefix.log` / `clippy_after.log` — the pre-fix clippy run (4 findings, the round-1 catch) and the post-fix zero-warning run at `3fad121`.
- `bench_output_stability.txt` + `benchrun3_stats/` — the block-ordered stability reps with per-run show_stats JSONs + backend contact logs (committed per the round-1 should-fix).
- `gpr1b_harness.log` — the G-PR1b harness output (run at `135734f`; `3fad121` re-ran clippy/fmt/lib separately).
- `sccache_day2_results.md` — the lane agent's full record (§1–§11 + §12 the G-PR2 round-1 verdict and the applied fixes).
- `scan_sccache.md` · `scan_uniffi-rs.md` · `scan_mp4parse-rust.md` · `scan_rust-code-analysis.md` — the four complete scan stdout captures (Part 1's evidence).
- Clone + branch state: `.raw/mozilla-heal/sccache` @ `refine/backend-backoff` `135734f` (ephemeral per the campaign law — re-clone at the pin + re-apply if the send re-opens after close-out; the diff is reconstructible from the results file §4 + the branch, which is NOT pushed anywhere yet — **the branch lives only on this box until G-PR4**).
