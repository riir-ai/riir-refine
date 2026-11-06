---
name: cargo-refine
description: Dev-time lint fixing with the riir-refine `cargo refine` binary. Activate when about to fix clippy warnings manually, when clippy reports mechanical findings before a commit, or when asked to auto-fix lints across files. Covers dry run, compile-gated write, feature-gated code, the documented divergence classes the rulewright refuses by design, and post-mining logging (feeding misses and divergences back to the rulewright's improvement queue).
---

# cargo-refine — dev-time lint fixing

Mechanical clippy findings are fixed by the rulewright FIRST, manual second.
It is span-preserving, comment-guarded, compile-gated under `--verify`
(bad edits auto-revert), auto-rustfmts fmt-clean files, and records
self-evolve trajectories on every apply.

## Setup (once per machine)

- Global binary `cargo refine` = `~/.cargo/bin/cargo-refine` → symlink to
  `/Users/katopz/git/riir-refine/target/release/cargo-refine`
  (built with `--features fix_verify,clippy_verify,latent_retrieval,rust_perf,clippy_lints,kat_service,kat_transport`
  — the bin's required-features + the verify features; Plan 107 T1 made
  kat_service mandatory).
- Rebuild after rulewright source changes:
  `cd /Users/katopz/git/riir-refine && cargo build --release --features fix_verify,clippy_verify,latent_retrieval,rust_perf,clippy_lints,kat_service,kat_transport --bin cargo-refine`
  (the symlink keeps pointing at the rebuilt binary).
- Fresh box without the sibling repo → fall back to manual fixes +
  `cargo clippy --fix`; do not block on the rulewright.
- ⛔ **A bare `cargo build --release` of riir-refine silently re-stales the
  global binary** (Issue 117, the 00:21 class): it overwrites the symlink
  target with a DEFAULT-FEATURES build, so `sync` / `update` / `--refill` /
  the burn push / the consent heartbeat are compiled out while the usage
  page still lists them. Always rebuild with the feature list above.
  **Diagnose it in one command — `cargo refine --version` prints the compiled
  feature set and, when the shipped set is incomplete, says `release set:
  STALE` and names the rebuild.** Invoking a compiled-out command now
  refuses loudly (`sync: refused — this build has no kat_transport …`,
  exit 2) and prints a rebuild recipe that supplies everything it asked
  for — never `unknown flag`, never silence.

## Usage (the just-works UX, 2026-09-08)

```bash
cargo refine <paths>                                        # DRY RUN (bare IS the dry run — zero edits)
cargo refine fix <paths>                                    # REAL fix: writes + compile-gates (fix_verify builds)
cargo refine fix --no-verify <paths>                        # write without the compile gate (syn-gated only)
cargo refine suggest <paths>                                # rule suggestions, no edits
cargo refine fix --verify-args "--features <set>" <paths>   # gated code
cargo refine info                                           # ONE COMMAND stats: wallet + account + mining + local work
cargo refine fix --groups pedantic <paths>                  # also fix allow-by-default groups the crate does NOT enable
```

- **Level gate (Issue 135)**: an allow-by-default lint (pedantic/nursery —
  `doc_markdown`, `match_bool`, `uninlined_format_args`, `map_unwrap_or`,
  `semicolon_if_nothing_returned`, … 20 of the fix set at clippy 1.98) is
  fixed ONLY where the target crate enables it or its group:
  `[lints.clippy]`, inherited `[workspace.lints.clippy]`, or a crate-root /
  file `#![warn(clippy::…)]`. Otherwise it is skipped LOUDLY
  (`lint-level: skipped …` per file + ` · N skipped: allow-by-default at
  crate level` on the verdict line). `--groups pedantic,nursery` (or `all`)
  restores the old apply-everything behaviour. `RUSTFLAGS` /
  `.cargo/config.toml` `-W clippy::…` are NOT read — pass `--groups` there.

- `fix` WRITES by default (the explicit verb is the consent) and
  verify-gates by default on `fix_verify` builds. `--write` still parses
  (no-op with fix). The `suggest` verb is the explicit review (`--suggest`
  a quiet alias). The `--fix`/`--suggest` flags stay accepted, taught
  nowhere (issue 210, the verb-first law).
- ⚠ Old scripts/docs that used `--fix` as a dry run now WRITE — use the
  bare form (or `suggest`) for review.

- `--verify` runs a baseline `cargo check --all-targets`, applies, re-checks,
  and REVERTS any edit that broke compilation (line-matched span-level).
- Feature-gated code: the default-features check compiles gated files EMPTY —
  always pass `--verify-args "--features <set>"` covering the edited files'
  gates (grep the enclosing `#[cfg(...)]`).
- Auto-rustfmt is default-on for files already fmt-clean at HEAD
  (pre-existing drift + CRLF files are skipped — no formatting churn).
- After applying: validate in the TARGET repo — `cargo clippy --all-targets`
  + `cargo test` (assert COUNTS for gated suites, never bare exit codes).
- **Auto-oracle (Issue 113) + the feature knob (Issue 115)**: a bare/suggest
  run over a cargo project lints it with REAL clippy at DEFAULT features
  (`[oracle]`-anchored spans replace similarity ranking; `[heuristic]` =
  feed-covered span with no diagnostic). Diagnostics that only fire behind a
  non-default feature are INVISIBLE at that posture — re-aim the spawn with
  `RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv>` (loud status line; a
  feature-superset clippy run is materially slower — pay it when the
  project's diagnostics of interest live behind cfg gates, e.g. after a
  fix touched `#[cfg(feature)]` code; `RIIR_REFINE_AUTO_ORACLE=0` disables
  the auto-run entirely; a manual `--clippy-json` still wins over both).
  Unknown feature names fail the spawn → LOUD heuristic-only fallback
  naming the knob.
- Commit the fix separately: `fix: refine <repo> <scope>`.
- **Invocation note (agents, the misattribution trap)**: run the rulewright with
  your terminal tool's `cd` PARAMETER + absolute target paths. A
  `cd <abs> && cargo-refine …` form INSIDE the command string dies with
  `'.' is ambiguous in multi-root workspaces` — that error is the Zed
  terminal wrapper's (it fires before the rulewright runs), NOT the rulewright's.
  Cross-workspace targets are fine: `owning_package_root` walks up from
  each target FILE, never the cwd (verified live 2026-08-28, Plan 550
  session — same command, `cd &&` form fails / `cd`-param form fixes).

## What it fixes vs refuses

Fixes (default build): `needless_return`, `needless_range_loop`,
`single_match`, `redundant_pattern_matching`, `double_parens`,
`unnecessary_map_or`, `vec-with-capacity`, `entry-vs-contains-insert`, and ~50
more clippy classes + the rust_perf proposers. The pedantic/nursery ones
(`uninlined_format_args`, `match_bool`, `map_unwrap_or`, `single_match_else`,
`semicolon_if_nothing_returned`, `doc_markdown`, …) apply only where the
target crate enables them, or with `--groups` (Issue 135 level gate above).

REFUSES BY DESIGN (fix manually — the rulewright is SAFER than clippy here):

| Divergence class | Why silent |
|---|---|
| comment-guarded matches | rewrite would displace interior comments |
| array-literal defaults (`map_or(&[], f)`) | E0308 risk without type context |
| array-literal `vec-with-capacity` iterables | no name to call `.len()` on |
| named-arg-with-rename (`{:<width$}`, `width = w`) | needs template rewriting |
| nested macro args (`format!` inside `println!`/`assert_eq!`) | descent is format-family, one level |
| guard-on-first-arm `match_bool`, `{0}`/dynamic-spec format args | rewrite shape not parse-provable |
| NaN-unsound negated comparisons (`!(a > b)` → `a <= b`, the `neg_cmp_op_on_partial_ord` shape) | NOT equivalent on floats — `!(NaN > x)` ≠ `NaN <= x`; the rewrite silently changes NaN semantics (the only correct fix is semantic) |
| cfg-attributed trailing `return` (needless_return; any attribute on the target stmt) | the edit span covers the stmt's attrs — the rewrite would drop them; a `#[cfg]`'d trailing return is only "last" on the arches whose cfg keeps it (clippy is post-expansion and SILENT on the switch — probe 2026-09-04, the seal-remake fps_bench misfire) |
| measurement-altering perf rules at BENCH targets (`benches/*.rs`, `#[bench]` fns — `dead-result-timing-loop → black_box`, loop restructures, allocation hoists) | the rewrite changes WHAT THE BENCH MEASURES; `--verify` proves compilation, never measurement validity — a GOAT bench's numbers are the artifact, and an LTO-folded timing loop is often the exact defect the bench exists to expose (observed 2026-09-16: the rule routed at katgpt-core `bench_775_adjoint_goat.rs` spans; declined manually). **AUTOMATED since P30 (2026-09-16): the lane declines-and-flags — `--fix` drops rust_perf edits on bench targets with a loud `bench-guard:` line (file byte-identical), `--suggest` stamps perf rows `⚠ bench-guard`; detection = `benches/` path component or a harness marker (`#[bench]`, criterion/divan/iai/bencher) — see `rust_perf::bench_guard`. Since Issue 118 (2026-09-16) the SAME door guards `--fix-compile` writes (declined on the routine path; an explicit bench path flags-and-proceeds) and oracle-consumed bench spans — manual divergence only for sec/docker/dist measurement-adjacent shapes |
| `doc_markdown` context shapes (Issue 120, 2026-09-16 — all SKIP-direction: clippy fires, the proposer stays silent) | words inside fenced/indented code blocks, blockquote lines, lines with an odd raw backtick count (clippy's per-paragraph unbalanced-tick suppression), link-shaped tokens (`[text](dest)` — clippy checks the TEXT through pulldown events), bare URLs (clippy suggests `<url>` — a different fix shape we do not produce), `#[doc = "…"]` attribute-form docs, doc text inside macro token trees, shell-command doc lines (Issue 135 T4 — first word `cargo`/`rustup`/`npm`/`git`/…, a `$ ` prompt, or `./`·`../`·`scripts/` path: backticking one argv token breaks copy-paste; fence/indent it by hand — the oracle lane DECLINES these too). The `--clippy-json` oracle lane consumes clippy's own spans for all of these — feed it when the proposer's silence leaves findings standing |
| LENGTH-MISMATCH iterator rewrites (`needless_range_loop` and kin — `for i in 0..N { slice[i] }` → `for x in slice` / `.iter_mut()` / `.enumerate()`, P34 2026-09-20, live evidence riir-ai Issue 981) | the rewrite is mechanical ONLY when `N == slice.len()` — a slice offset into a larger buffer (`let gs = &mut buf[h * sph..]` runs to the parent's END, len > sph) makes every iterator form a SEMANTIC rewrite (981: one head's decay multiplied every later head's BPTT slab; misread as a GPU kernel failure for a forensic session; note `.enumerate()` does NOT bound the iteration either — it iterates the WHOLE collection, `min` semantics differ from `0..N`). **The proposer refuses non-`len()` bounds + local collections (fixture-pinned); the oracle door declines range-dropping suggestions whose end is not a syntactic zero-arg `.len()` (P34). MANUAL conversions — the actual 981 emitter — must verify `N == coll.len()` at the site, or keep the range visible: `&mut buf[base..][..N]`, `.take(N)`** |

Semantic lints (`if_same_then_else`, …) are not fix class — owner call.

## Discipline

- NEVER fix files with sibling WIP mid-flight — `git status` first; commit
  only your own hunks; a fix diff is mechanical and must stay separable.
- One-off single finding → manual is fine. Refine shines on batches.
- Wrong suggestion or miss? Fix the PROPOSER at root cause in riir-refine
  (`src/domains/*/proposers/`) + add a fixture regression test — never
  suppress, never hand-edit past it silently.

## Post-mining (the self-evolve feedback loop)

Every run records trajectories to `~/.cache/riir-refine/memory.json`
(applied counts by lint; `--verify` reverts + the clippy_verify oracle feed
outcome verdicts). Usage artifacts are the rulewright's improvement queue —
**post-mining** (mined from usage) vs **pre-mining** (mined from external
sources — papers, crates, articles; Protocol A of the riir-refine skill):

- **miss** — real clippy fires, rulewright silent → proposer extension candidate
- **created** — the rulewright's output trips a NEW lint → terminal-state emission bug
- **revert** — an edit the compile gate caught → proposer guard bug

After fixing sessions, append observed misses/divergences to the post-mining
intake in `riir-refine/.distill/001_mining_queue_snapshot.md` (one line each:
class, site, evidence type). The idle loop (riir-refine skill queue item 14)
consumes it: probe each class against real clippy FIRST, then extend the
proposer + fixtures + score-bench.

The **aggregate** of the `miss` class — how much of clippy's surface the
corpus does not cover at all, rather than the misses one session happened to
hit — is `cargo refine --coverage-report` (Issue 119). It derives the lint list
from the INSTALLED driver, so the debt moves as clippy ships lints, and joins
two cited frequency sources (the Bench 061 real-code fired histogram + the
published kornel 100K-crate top-10) without ever estimating a count. Read the
DISJOINT warning if it prints: the local histogram was captured with
pedantic+nursery enabled over already-swept repos, so it ranks the opt-in tail
and can be structurally blind to the lints that fire on a plain clippy run —
staging off the ranked head alone then stages opt-in lints. Report-only; it
stages nothing and gates nothing.

Audit adoption + loop state in one command (Issue 037's instrument):
`/Users/katopz/git/riir-refine/scripts/self_evolve_audit.sh` — memory
tiers/outcomes, score-bench history, intake progress, fix-commit trail.

## Evidence (2026-08-24 adoption trial, M3)

- Probe: 12/12 planted fixes landed, 2/2 planted divergences refused,
  tests bit-stable, idempotent re-run, compile gate ran 2 checks 0 reverts.
- Real dry run: `riir-ai/crates/riir-games/src` → 222 edits / 54 files available.
- Score bench (run #3): 98.1% heal rate, 1.9% created, 100% parse safety.
- Campaign record: full katgpt-rs sweep + riir-train/ndb/chain/engine/games-civ
  landed with count-identical test validation (see riir-refine AGENTS.md).

**Source secrecy:** binary-only distribution (riir-refine Proposal 003) —
never publish riir-refine source; internal agents build from the sibling repo.
