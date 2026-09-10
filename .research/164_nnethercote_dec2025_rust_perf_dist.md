# 164 — Batch 143 verdict→landing: the nnethercote Dec-2025 rustc-perf distill (R1+R2 rust_perf, R3 dist)

**Status:** RECORD (batch landed 2026-09-10, Plan 123, commit `522f6947` + docs).

## Source

nnethercote, "How to speed up the Rust compiler in December 2025"
(nnethercote.github.io, 2025-12-05). Fetched full-text 2026-09-10, two
independent renderings (direct + jina reader), identical text. No `.raw`
clone needed — blog + PR numbers only (the `distill` skill's cite-shape
lane). The PERF-SEARCH intake row staged at `cf9e44ae` (queue item 11).

## Quote verification (B31 discipline)

- **#147293 (R1's source)** verbatim: "In the query system there was a value
  computed on a hot path that was only used within a `debug!` call. In this
  PR I avoided doing that computation unless necessary, which gave icount
  reductions across many benchmark results, more than 3% in the best case.
  This was such a classic micro-optimization that I added it as an example to
  the Logging and Debugging chapter of the The Rust Performance Book" (the
  doubled "The" is in the original). PRECISION NOTE vs the staging row: the
  words `log_enabled!` / "lazy format args" are NOT in the post — the GATE
  MECHANISM lives in the linked perf-book chapter, cited as mechanism home,
  never quoted as post text.
- **#143684 (the wall-time rider)** verbatim: "the mean icount reduction
  across all benchmark results was an excellent 1.70%, and the mean cycle
  count reduction was 0.90%, but the mean wall-time saw an increase of 0.26%.
  Wall-time is the true metric, because it's what users perceive".
- **The Bevy `#[derive(Reflect)]` series (R2's source)** — same post, "Proc
  macro wins in Bevy": bevy_ui "around 16,000 lines and 563,000 bytes" of
  source vs "around 27,000 lines and 1,544,000 bytes" generated ("Macro
  expansion almost quadrupled the size of the code, mostly because of this
  one macro!"); bevy_window generated code "reduced by 39%, which reduced
  cargo check wall-time for that crate by 16%, and peak memory usage by 5%".
  Load-bearing sentence verbatim: "it is more efficient to optimize away
  redundancies at the generation point, where context-specific and
  domain-specific information is available, rather than relying on
  sophisticated optimization machinery further down the compilation pipeline
  that has to reconstruct information." `-Zmacro-stats` itself is a June-2025
  post (cited as pointer, not re-quoted).
- **`-Zhint-mostly-unused` (R3's source)** — same post, "Faster compilation
  of large API crates": "Josh Triplett introduced a new experimental flag,
  `-Zhint-mostly-unused`, which can give big compile time wins for people
  using small fractions of very large crates. This is typically the case for
  certain large API crates, such as `windows`, `rustix`, and `aws-sdk-ec2`."
  PRECISION NOTE vs the staging row: the post names **aws-sdk-ec2**
  specifically; the `aws-sdk-` prefix in the detector's crate table is OUR
  generalization of the service-crate family, recorded as ours. Deep link:
  inside-rust call-for-testing 2025-07-15.

## What landed (Plan 123 T2–T4)

1. **`debug-level-gated-computation`** (rust_perf, complexity, BENCHABLE).
   Decision content: a hot-path binding whose only consumer is a
   `debug!`/`trace!` call executes every iteration — the binding is a real
   statement, and log's runtime max-level atomic cannot be const-folded (no
   `release_max_level_*` features), so the compiler must keep the computation
   alive for the enabled branch it can't rule out. Fix: `log_enabled!` around
   the computation, or move it into the macro's lazy format args.
   **MEASURED ParetoDominant 3/0/0** (`batch143_pareto`): 2184–2534× at
   n=100–10K, alloc axis EXACT 2200/22000/220000 → 0 — with no logger
   installed the gated arm builds ZERO summaries. Honest magnitude caveat
   in-rule: the bench's ratio is the whole log-only computation's share; a
   real tick's speedup = summary cost ÷ whole loop body. G1 semantics gate
   (`batch143_log_gate_computation_parity`) uses the REAL `log` crate (new
   dev-dep, already in the lock — zero new packages) and proves the four-arm
   contract: ungated binding computes n with the level off; the gate skips
   entirely; the lazy-args form is the equivalent skip; and gating the CALL
   while hoisting the COMPUTATION fixes nothing (the consumer-shape trap).
2. **`macro-generated-code-optimize-at-generation-point`** (rust_perf,
   complexity, NotBenchable, annotated). Build-time content — no runtime A/B
   axis exists; the upstream numbers are the measurement. Nearest corpus
   neighbor is none: zero proc-macro/generated-code coverage (dedup grep
   re-verified at write time).
3. **`large-api-dependency-mostly-unused-hint`** (dist, NEW family
   `BuildTime`, suggest-only). Trigger: any `*dependencies` section naming
   `windows` / `windows-sys` / `rustix` / `aws-sdk-*` (exact table + one
   prefix; deliberately small — a miss is silent no-advice, an over-broad
   table is noise). The scanner extension records only dependency NAME
   presence (`ManifestScan::large_dependencies`), keeping the
   key-presence-only value contract. **Out-of-manifest honesty:** the fix
   (`[build] rustflags` in `.cargo/config.toml`) is invisible to a
   manifest-scoped scanner, so the fixed fixture IS the config file (the
   precision contract's "fixed → zero findings" holds mechanically: no
   dependency sections → silent) and the notes carry the measure-first +
   nightly-only caveats.

## Gates (T5)

- batch143 gates 3/3: `batch143_rules_present_and_parse` (categories, syn
  parse, needles — the gate caught one case-mismatched needle pre-landing:
  "Wall-time" is capitalized in the quote), `batch143_log_gate_computation_parity`
  (the four-arm G1 above), `batch143_pareto_benches` (the measured verdict).
- lib counts: default **1192/0**; `rust_perf_bench` **1210/0** (2 ignored =
  the B135+B143 Pareto runners); `dist` **1212/0**; `kernel_heal,latent_retrieval`
  **1654/0**; `--all-features` **2241/0**.
- Count pins walked (7 sites — the corpus now has MORE count sites than
  B141's ledger named): entries.len 124→126; with_spec 90→91; without_spec
  34→35 (+R2 membership); annotated 67→69 (+R1 +R2) in BOTH the
  forty_three_annotated test AND the drafter-propagation integration test;
  unannotated 57 UNCHANGED (both adds joined the annotated side — the plan's
  "55" was an arithmetic slip, caught by the test); serde backward-compat
  67→69; the bench-spec count 90→91; header ledger (126, complexity 21,
  91/35) + the STALE header prose "30 are honestly NotBenchable" repaired to
  the pinned 35 (pre-existing drift the pin walk surfaced); the line-245
  ledger comment 34/124→35/126.
- Retrieval: kernel self-variants + blended floors HELD byte-identical
  (op-bigram blend 94.7/96.9); **router re-measured 92/131 = 70.2% —
  byte-identical to the B142 pin, floor 87 held, no re-pin** (rust_perf +
  dist adds did not move the kernel-labeled centroid mass).
- `--corpus-stats`: rust_perf **126**, dist **5** (kernel_opt 607, clippy 49,
  sec 15, docker 31 unchanged).
- **Score-bench #94 FLAT**: 96.2% / 1.9%, parse 100, decline 100, edits 64,
  vocab 38, artifact blake3 `faa021c89d1dbd2c…` **byte-identical to #93**
  (11th consecutive flat) — the rust_perf + dist adds provably do not move
  the scored clippy/perf path.
- clippy: whole-lib at `-D warnings` = 1 PRE-EXISTING warning
  (`rustc_errors/corpus_gen.rs:332` `too_many_lines`, the sibling dogfood
  commit 78532d8b's own expansion — their active lane, not this batch's;
  my write set is clean).

## Environment (the collision record)

A sibling session landed its Batch-142 lane in the shared worktree earlier
today (12:10–14:37) and by 16:25 was actively writing a NEW untracked lane
(`kat_service_config.rs` + heal.rs config commands, riir-dapps Plan 023)
whose in-flight test code temporarily broke the shared tree's compile
(E0382/E0433 in their unfinished tests). Batch 143's write set is fully
disjoint (rust_perf/dist/docs vs kat-service files) — the commit stages only
the 10 named batch files (staged_set_audit reviewed; its stale-vs-HEAD flags
on the docs are the deliberate count rewrites this batch exists to make).
The T5 battery ran in BOTH places: the five lib states in the shared worktree
BEFORE the sibling's WIP landed, and clippy/corpus-stats/retrieval/score-bench
in an isolated worktree at `522f6947` (`git worktree add` beside the repo so
the sibling path deps resolve; `CARGO_TARGET_DIR=/tmp/b143_target`). The
main repo's `.benchmarks/score_history.jsonl` carries the sibling's own
uncommitted 09:52 #93 append — left untouched per commit-only-your-hunks;
Batch 143's #94 row is recorded in the isolated worktree's jsonl + this note.

## Dispositions

- Weak lead #142095 (VecCache first-segment fast path): still recorded, not
  staged (store-specific fix space, detection hard).
- Drops re-affirmed: #148040 / #148706 / #148789 (rustc/std-internal, no
  bounded consumer-side fix); macOS "secret build setting" (low prior).
- The dynM catalog-gap rider still rides the next `entries_gpu_34.rs` touch
  (this batch's write set did not include it).
- arxiv walk #9 opens Mon 09-14 — sibling lane.
