---
name: cargo-heal
description: Dev-time lint healing with the riir-clippy `cargo heal` binary. Activate when about to fix clippy warnings manually, when clippy reports mechanical findings before a commit, or when asked to auto-fix lints across files. Covers dry run, compile-gated write, feature-gated code, the documented divergence classes the healer refuses by design, and post-mining logging (feeding misses and divergences back to the healer's improvement queue).
---

# cargo-heal — dev-time lint healing

Mechanical clippy findings are fixed by the healer FIRST, manual second.
It is span-preserving, comment-guarded, compile-gated under `--verify`
(bad edits auto-revert), auto-rustfmts fmt-clean files, and records
self-evolve trajectories on every apply.

## Setup (once per machine)

- Global binary `cargo heal` = `~/.cargo/bin/cargo-heal` → symlink to
  `/Users/katopz/git/riir-clippy/target/release/cargo-heal`
  (built with `--features fix_verify,clippy_verify`).
- Rebuild after healer source changes:
  `cd /Users/katopz/git/riir-clippy && cargo build --release --features fix_verify,clippy_verify --bin cargo-heal`
  (the symlink keeps pointing at the rebuilt binary).
- Fresh box without the sibling repo → fall back to manual fixes +
  `cargo clippy --fix`; do not block on the healer.

## Usage

```bash
cargo heal --fix <paths>                                  # dry run (review only)
cargo heal --fix --write --verify <paths>                 # compile-gated apply
cargo heal --fix --write --verify --verify-args "--features <set>" <paths>   # gated code
cargo heal --suggest <paths>                              # rule suggestions, no edits
```

- `--verify` runs a baseline `cargo check --all-targets`, applies, re-checks,
  and REVERTS any edit that broke compilation (line-matched span-level).
- Feature-gated code: the default-features check compiles gated files EMPTY —
  always pass `--verify-args "--features <set>"` covering the edited files'
  gates (grep the enclosing `#[cfg(...)]`).
- Auto-rustfmt is default-on for files already fmt-clean at HEAD
  (pre-existing drift + CRLF files are skipped — no formatting churn).
- After applying: validate in the TARGET repo — `cargo clippy --all-targets`
  + `cargo test` (assert COUNTS for gated suites, never bare exit codes).
- Commit the heal separately: `fix: heal <repo> <scope>`.
- **Invocation note (agents, the misattribution trap)**: run the healer with
  your terminal tool's `cd` PARAMETER + absolute target paths. A
  `cd <abs> && cargo-heal …` form INSIDE the command string dies with
  `'.' is ambiguous in multi-root workspaces` — that error is the Zed
  terminal wrapper's (it fires before the healer runs), NOT the healer's.
  Cross-workspace targets are fine: `owning_package_root` walks up from
  each target FILE, never the cwd (verified live 2026-08-28, Plan 550
  session — same command, `cd &&` form fails / `cd`-param form heals).

## What it fixes vs refuses

Fixes (default build): `uninlined_format_args`, `match_bool`, `map_unwrap_or`,
`single_match`/`single_match_else`, `needless_return`, `needless_range_loop`,
`semicolon_if_nothing_returned`, `redundant_pattern_matching`,
`double_parens`, `vec-with-capacity`, `entry-vs-contains-insert`, and ~25
more clippy classes + the rust_perf proposers.

REFUSES BY DESIGN (fix manually — the healer is SAFER than clippy here):

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

Semantic lints (`if_same_then_else`, …) are not heal class — owner call.

## Discipline

- NEVER heal files with sibling WIP mid-flight — `git status` first; commit
  only your own hunks; a heal diff is mechanical and must stay separable.
- One-off single finding → manual is fine. Heal shines on batches.
- Wrong suggestion or miss? Fix the PROPOSER at root cause in riir-clippy
  (`src/domains/*/proposers/`) + add a fixture regression test — never
  suppress, never hand-edit past it silently.

## Post-mining (the self-evolve feedback loop)

Every run records trajectories to `~/.cache/riir-clippy/memory.json`
(applied counts by lint; `--verify` reverts + the clippy_verify oracle feed
outcome verdicts). Usage artifacts are the healer's improvement queue —
**post-mining** (mined from usage) vs **pre-mining** (mined from external
sources — papers, crates, articles; Protocol A of the riir-clippy skill):

- **miss** — real clippy fires, healer silent → proposer extension candidate
- **created** — the healer's output trips a NEW lint → terminal-state emission bug
- **revert** — an edit the compile gate caught → proposer guard bug

After healing sessions, append observed misses/divergences to the post-mining
intake in `riir-clippy/.distill/001_mining_queue_snapshot.md` (one line each:
class, site, evidence type). The idle loop (riir-clippy skill queue item 14)
consumes it: probe each class against real clippy FIRST, then extend the
proposer + fixtures + score-bench.

Audit adoption + loop state in one command (Issue 037's instrument):
`/Users/katopz/git/riir-clippy/scripts/self_evolve_audit.sh` — memory
tiers/outcomes, score-bench history, intake progress, heal-commit trail.

## Evidence (2026-08-24 adoption trial, M3)

- Probe: 12/12 planted fixes landed, 2/2 planted divergences refused,
  tests bit-stable, idempotent re-run, compile gate ran 2 checks 0 reverts.
- Real dry run: `riir-ai/crates/riir-games/src` → 222 edits / 54 files available.
- Score bench (run #3): 98.1% heal rate, 1.9% created, 100% parse safety.
- Campaign record: full katgpt-rs sweep + riir-train/ndb/chain/engine/games-civ
  landed with count-identical test validation (see riir-clippy AGENTS.md).

**Source secrecy:** binary-only distribution (riir-clippy Proposal 003) —
never publish riir-clippy source; internal agents build from the sibling repo.
