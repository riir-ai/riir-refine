# Bench 126 — Mozilla outbound fix-PR wetrun (sccache CACHEDIR.TAG): before/after

**Status:** RECORD — T2–T4 of [Plan 329](../.plans/329_mozilla_outbound_pr_wetrun.md); SEND (T5) owner-gated on gh auth
**Date:** 2026-11-02 · zed-m3-mozilla-wetrun, epoch 1793602400 · M3 Max (macOS, AC power)

## What this measures

The Proposal 022 outbound campaign's first wetrun, run BY HAND (the `harvest`
feature does not exist yet): scan the curated Mozilla targets at THEIR OWN
lint posture → gate → heal → verify at their MSRV/toolchain → branch, ready
to send. The G1 per-PR gate (created-warnings == 0 at their CI flags; their
toolchain; healer-spans-only diff; tests green) is the before/after bench.

## BEFORE — scan inventory (T2)

| repo | pin | clippy @ their posture | result |
|---|---|---|---|
| mozilla/uniffi-rs | `ef9625b6` (2026-10-28) | `cargo clippy --workspace --all-targets` @ pinned **1.94.0** (their rust-toolchain.toml, clippy component; CI = CircleCI `clippy --all --all-targets -- -D warnings`) | **0 warnings** across 598 artifacts (verified twice incl. a forced recompile) |
| mozilla/sccache | `71d107e0` (2026-11-01) | their exact CI line: `clippy --locked --all-targets -- -D warnings -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default` @ MSRV **1.91.0** (rust-version field) | **0 warnings** pre-fix |
| mozilla/rust-code-analysis | `37e5d83c` | not scanned — **dormant since 2026-01-20** | scan-only, no PR (022 one-PR-per-repo-ever rule: don't burn the shot on a dormant repo) |
| mozilla/supply-chain | `e6648794` | n/a — **0 .rs files** (data-only; sibling distill Research 361/362 session confirmed) | skip |
| mozilla/mp4parse-rust | `d3a9abf` (2026-10-16) | `clippy --workspace --all-targets` (incl. a clean-tree rebuild) | **0 warnings** both crates |

**The gate worked:** all four scanned trees are clippy-clean at their own
posture. The clippy-lane fix set is EMPTY everywhere — the 022 pipeline
correctly produces zero mechanical PRs. (Pedantic sweeps — uniffi 3668 /
sccache 1577 findings — are NOT their posture and are excluded by the 022
"lint not in their allow list / their posture" gate; sending pedantic fixes
to repos that don't run pedantic is noise by the proposal's own rule.)

Consequence (recorded in Plan 329 §T-scc BEFORE any fix work): the
distill Research 362 "A-lead" (pre_exec/posix_spawn) is sccache's OWN
landed fix at the pin — `mock_command.rs:269–282` + `share_jobserver` +
call site `:707` — NOT open fix space. No sccache PR was built from it.

## The pivot (T3)

With zero mechanical material, the wetrun pivots to the campaign's
issue-driven lane: fix what the maintainers ALREADY asked for. One target
passed the gates: **mozilla/sccache#2975** ("tag the cache directory using
CACHEDIR.TAG", filed 2026-11-01, zero comments, zero labels). Additive,
spec'd by the Cache Directory Tagging Specification, zero behavior risk —
the clean first-PR shape.

Fix location re-derived from source: `LruDiskCache::init()`
(`src/lru_disk_cache/mod.rs:186–211`) is the single choke point where the
cache root is created and scanned. Two design constraints found in that
loop: (1) the tag must be EXCLUDED from the LRU scan — else eviction
(`make_space` → `remove_lru` → `remove_file`) deletes it under a full
cache; (2) it must bypass `can_store(size)` — else a tiny max-size cache
deletes it at init.

## AFTER — the fix (T4)

Branch `refine/cachedir-tag` @ `b97bcc0` — 80 insertions, 1 file
(`src/lru_disk_cache/mod.rs`), zero deletions, zero unrelated hunks:

- `CACHEDIR_TAG_FILE_NAME` + `CACHEDIR_TAG_CONTENTS` (the spec's canonical
  bytes, sccache named as creator).
- `write_cache_dir_tag()` — best-effort (unwritable dir logs, never fails
  cache creation); skips the write when the existing tag already matches so
  cache hits don't dirty the directory mtime.
- `init()` writes the tag after `create_dir_all`, and the scan loop skips
  the tag (not LRU-tracked, not size-limited) — the two constraints above.
- 2 new unit tests: `test_writes_cachedir_tag` (bytes match the signature
  constant), `test_cachedir_tag_not_a_cache_entry` (size/len stay 0 across
  re-opens; tag survives forced eviction).

### G1 verification at THEIR toolchain (rustup toolchain 1.91.0, installed for this run)

| gate | before | after | bar |
|---|---|---|---|
| `cargo +1.91.0 test -p sccache --lib --locked` | 596 passed, 0 failed | **598 passed, 0 failed** (+2 new) | green |
| `cargo +1.91.0 clippy --workspace --all-targets --locked -- -D warnings -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default` (their exact CI flags) | 0 findings | **0 findings** — created-warnings ZERO | external bar ZERO |
| `cargo +1.91.0 fmt -- --check` | clean | **clean** | no reformat |
| diff content | — | 1 file, only the tag writer + the scan skip + 2 tests | healer-spans only |

Pre-existing environment note (NOT the PR's problem): the
`tests/integration/randomize_readdir` harness crate panics on this box
(libc function-pointer load) under `cargo test --workspace` — pre-existing
on the clean tree, macOS-specific; their CI runs Linux. Not touched.

## The send queue (T5 — owner-gated, ≤3 PRs/day, one PR per repo)

| # | repo | branch | status |
|---|---|---|---|
| 1 | mozilla/sccache | `refine/cachedir-tag` (`b97bcc0`) | READY — awaiting gh auth (fork to `riir-ai/sccache` → push → PR referencing #2975) |

uniffi-rs day-1: **no PR** — the honest scan outcome. The quartet's only
upstream-engaged fix space (#3020, issue #3011) is another contributor's
PR under CHANGES_REQUESTED for 3 weeks; per 022 we do not pile a competing
PR onto it without an owner call. Recorded as the day-2 candidate.

## Campaign ledger (G2 — accept rate, 90-day window)

| sent | merged | closed | open | accept rate |
|---|---|---|---|---|
| 0 | 0 | 0 | 0 | n/a (sends pending) |

## Post-mining intake (the lane coupling, 022 fusion lineage)

- External-repo scan found **zero clippy-lane gaps** in the corpus on these
  trees — the corpus is not missing mechanical rules the targets need;
  the finding class is "clean trees", not corpus misses.
- Process miss worth recording: the distill lane's Research 362 graded a
  candidate as an OPEN fix-space lead that was in fact the repo's own
  landed fix (verified in-source before any PR was authored — the gate
  held, but the verdict's framing didn't check upstream state first).
  Feeding back to the distill queue: add "upstream-state check at the pin"
  to the mining candidate template (a `git log -S` / blame line per lead).
