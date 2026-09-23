# Plan 114 — Batch 135: the Wild-linker "RustForge wild performance tricks" article distill

**Status:** COMPLETE — LANDED at `84d0d73f` (feat: Batch 135 — rust_perf 114→117, 3 measured rules; history stamp `7f1d1659`). Status-corrected 2026-09-23 by the idle-loop plan-status hygiene pass (the IN-PROGRESS line predated the landing).

Source: David Lattimore (Wild linker author), "RustForge wild performance tricks",
davidlattimore.github.io, 2025-09-02. Six techniques; verdict + dedup gate in
`.research/155_rustforge_wild_performance_tricks_verdict.md`. Corpus: `rust_perf`
(114 → 117). Batch number 135 — 132/133 are reserved (the CloudRift lead-pair),
134 is reserved (ds4 PR#5, Plan 113).

## Tasks

- [x] Pre-flight: read `.distill/001_mining_queue_snapshot.md` + grep the rust_perf
      corpus taxonomy + counts; read `.plans/.highwater` (113 → 114) and
      `.research/.highwater` (154 → 155).
- [x] Dedup gate, all 6 candidates, BOTH vocabularies (source terms + operator
      names); one evidence line each in the research note. Key leads read:
      `reuse-allocation-clear` (clear-reuse EXISTS — candidate 4 is new ONLY as
      the lifetime-change dimension), `mem-take`, `mutex-to-atomic` (opposite
      direction), `cache-line-block-partitioned-lock` (lock-granularity
      partition vs zero-sync ownership partition), `parallel-disjoint-pread`
      (io reads, not shared-memory mutation).
- [x] Decision-content test per candidate; grades: (3) atomic-convert B+ GOAT
      lead, (4) lifetime-reuse B, (1) partition B−, (2) sharded-vec DROP
      (third-party dep + composition), (5) defer-drop DROP (allocator-dependent,
      no deterministic verdict), (6) MaybeUninit shadow DROP (panic-unsound
      fix-space; recorded as the extension arm of rule (4)).
- [x] GOAT gate argument (G1 exact, G2 mechanism magnitude, G3 regime
      annotation, G4 alloc axis) in the research note; lane-intel axis
      evaluated: does NOT fire (host-side build/run-time Rust; no league
      lane surface — recorded explicitly).
- [x] Write `.research/155_rustforge_wild_performance_tricks_verdict.md`
      (Status: RECORD, born-complete, no checkboxes).
- [x] Author 3 corpus entries in `src/domains/rust_perf/corpus.rs`
      (ORIGINAL fixtures — never the article's code verbatim):
      `atomic-vec-zero-cost-convert` (concurrency),
      `vec-reuse-across-lifetime-change` (allocation),
      `partitioned-mut-slices-zero-sync` (concurrency). All three carry
      `BenchSpec` + `context_notes`.
- [x] Bench specs: (a) conversion alloc axis 2→1 (shared clone makes the
      elision exact); (b) alloc-per-call 1→0 via a leaked-row 'static buffer +
      `reuse_vec` (inline const size/align assert — syn-parse-gated); (c)
      scoped threads + `chunks_mut` partition vs per-bump `Mutex` contended,
      sizes = thread counts per the align-64 precedent, G1 closed-form
      checksum in-harness.
- [x] Add `partitioned-mut-slices-zero-sync` to `ALLOC_AXIS_EXEMPT`
      (bench.rs) — same measured platform class as
      `cache-line-block-partitioned-lock` (contended macOS pthread mutex
      allocates nondeterministically in the slow path, Batch-78 probe).
- [x] MEASURE the three rules: `#[ignore]`-gated `batch135_pareto_benches`
      test (rust_perf_bench feature) driving `PerfBenchValidator` on exactly
      the three rules; verdicts recorded into `context_notes`.
- [x] G1 parity gate: `batch135_fixtures_g1_parity` — both arms of all three
      fixtures produce identical results on fixed inputs (pure test, no
      feature gate).
- [x] Count pins walked IN THE SAME COMMIT:
      corpus.rs header (117 / allocation 23 / concurrency 10 / 90 benchable /
      27 NotBenchable — repairs the pre-existing 25-vs-14 inconsistency),
      benchable/not-benchable doc lists (87/111 → 90/117, 15/102 → 27/117),
      build-fn doc comment, `entries.len()` test 114 → 117, annotated test
      57 → 60, `fourteen_rules_are_not_benchable` 14 → 27 (pre-existing rot
      repaired — the B118–B129 io annotations were never counted; test is
      rust_perf_bench-gated so nobody ran it), AGENTS.md ×2, README.md
      rust_perf row (repairs the stale "112 through Batch 125") + the
      architecture-tree count, `.docs/11_domains/rust_perf.md` (count +
      Batch-135 paragraph + category prose + 3 table rows),
      `.docs/11_domains/README.md` index row.
- [x] Batch-135 tests: rules_present + pseudocode_parses (syn must accept the
      inline const block — the fixture's compile-time assert mechanism) +
      concepts_covered needles + the G1 parity gate + the ignored Pareto
      runner.
- [x] Gates: `cargo test -p riir-clippy --lib` (default) green;
      `cargo clippy -p riir-clippy --lib` green (default +
      `--features rust_perf_bench` for the gated arms);
      `cargo test -p riir-clippy --lib --features rust_perf_bench
      fourteen_rules_are_not_benchable` green after the pin repair;
      `cargo heal --score-bench` appended (standing rule after every corpus
      change) and the new number recorded in the queue line.
- [x] Queue line in `.distill/001_mining_queue_snapshot.md` (Batch 135 landed,
      counts old→new, score-bench run #, verdict + plan/research refs).
- [x] Bump `.plans/.highwater` 113 → 114 and `.research/.highwater` 154 → 155
      in the same commit.
- [x] Commit on develop (named files only, `feat:` prefix), push, report hash
      + the rule count delta (114 → 117).

## Honest-caveat ledger

- Fixtures are fresh reproductions of the PATTERNS; the article is an
  unlicensed-format blog post — cite-shape only, zero verbatim code (Girder
  BSL discipline).
- The (b) fixture leaks the row storage (`Box::leak`) to make the buffer's
  `'static` element type sound INSIDE the harness; the rule text carries the
  real-world safety contract (buffer cleared before any dead ref could be
  observed; panic between take and restore loses the buffer, never UB).
- (c) joins ALLOC_AXIS_EXEMPT on the Batch-78 probe evidence (same platform
  class), not a fresh probe — noted in the const's comment.
- The article's `split_off_mut` is nightly-shaped; the landed rule uses the
  portable `chunks_mut`/`split_at_mut` form and says so.
