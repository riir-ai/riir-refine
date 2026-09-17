# Bench 095 — shader_perf frame-time budget tier (Plan 134 T2.1)

**Status:** RECORD (2026-09-17; Plan 134 T2.1 COMPLETE — the tier ships, Phase 2 T2.1 closes)

## What landed

The frame-time budget tier — the measurement-VERDICT half of the
`shader_perf` domain — gated (Plan 134 T2.1) on riir-shader shipping the
capture-side frame-time axis. That prerequisite landed 2026-09-17 (issue
018 there; `34ff053` + `a92000a`); this bench record is the tier's GOAT.

- `src/frame_time_report.rs` (UNGATED — the corpus_coverage/horizon_report
  law): sidecar parse (schema-pinned `riir_shader_frame_time_v1`), budget
  rows, the A/B diff with warmup-parity + pairing-uniqueness evidence
  guards, both report renders, the blind() refusal predicates.
- CLI `--frame-time-report [PATHS…]` + `--frame-budget <MS>` +
  `--vs <PATH>` + `--frame-delta-pct <PCT>` (bin/heal.rs; standalone_mode +
  mode_flag_tests joined; usage parity holds).
- Domain handle: `domains::shader_perf::frame_time` re-export under
  `shader_perf`; charter doc in `.docs/11_domains/shader_perf.md`.

## Gates (Proposal 014's, scoped to an instrument tier)

- **G1 — fixtures fire, negative class refuses**: 11 lib tests green
  (`cargo test -p riir-clippy --lib frame_time`): valid parse, wrong-schema
  refusal, malformed-JSON refusal, null-stats → `n/a` verdict (nulls, never
  zeros), CLI-override precedence, threshold-edge verdicts (−5% → IMPROVED,
  +5% → REGRESSED, +4.9% → FLAT), unpaired/no-samples/warmup-mismatch
  verdicts, ambiguous-pairing refusal, blind laws, and the walker
  suffix test. 8 process-boundary CLI contracts green
  (`tests/frame_time_report_cli.rs`, the bin's required-features set): the
  two exit-0 rows, and exit-1 refusals for empty scan / unparsable sidecar
  (naming the file) / warmup mismatch / ambiguous pairing.
- **G2 — latency**: 29 real sidecars, whole-process wall **9.3 ms**
  (0.32 ms/sidecar incl. process startup) — the `< 1 ms/file` `wgsl_scan`
  bar holds with 3× headroom.
- **G3 — no regression**: `cargo check -p riir-clippy --lib` clean at
  default + `--features shader_perf`; lib `frame_time` + `heal_usage_flag_parity`
  + bin `mode_flag_tests` (16/16) all green; no corpus or count-pin rows
  move (the tier adds no rules — the corpus stays 8, FIXABLE unchanged).
- **G4 — alloc**: the tier is a cold report path (once per invocation,
  never in the heal loop) — the SuggestScratch bar does not apply; noted
  rather than gated.
- **G5-real — live corpus**: the fresh 29-effect riir-shader sweep
  (2026-09-17, M3 Metal, warmup 15 frames/150 ms, budget 16.7 ms carried in
  every sidecar): **29/29 parse-valid, 0 unparsable**, verdicts correct
  (fastest effects ~8.3 ms p50 = the vsync-less uncapped floor class;
  adaptive-quality 16.5 / black-hole 16.2 close to but under the 16.7
  budget). Self-A/B (sweep vs itself): 29 paired, all FLAT +0.00%, exit 0 —
  the pairing + warmup-parity + delta wiring proven on the real wire shape.
  Refusal legs proven live: warmup-mismatched post dir → exit 1 + the
  MISMATCH row; empty dir → exit 1.

## Honest scope notes

1. **Verdict math is proven on fixtures + the self-A/B; no real perf-gain
   A/B exists yet.** The first IMPROVED row will come from the first
   healed-effect re-sweep (the consumer protocol: heal → naga → capture →
   sidecar re-sweep). This bench pins the INSTRUMENT, not a gain.
2. **Noise discipline is documented, not mechanized**: one run is one run;
   a load-sensitive p50 passes the re-run and a real delta does not (the
   katgpt-rs x86_64-matrix lesson, four different failing sets over four
   runs of one commit). The A/B report says so in its header.
3. **The walker-suffix bug** (`Path::extension()` returns `json` for
   `gradient.frames.json`) was caught by the CLI contract test's first run
   — every real sweep dir would have read as an empty scan (the exact
   blind-walk class the refusal law exists for). Pinned by a unit test
   (`walker_matches_the_sidecar_suffix_not_the_extension`).
4. **Schema note**: the sidecar's warmup fields were added in riir-shader
   `a92000a` the same day (the issue text promised them; render_json never
   emitted them — caught here during the tier's design review, zero
   consumers existed, v1 extended in place).
5. **Pre-existing lib-test failure observed, not owned**:
   `downstream_hint::tests::relative_target_fires` fails at CLEAN
   origin/develop in ANY worktree (proven by stash-probe: my changes
   stashed, test still fails) — it derives its fake-repo parent from
   `cwd.parent()`, which is only the right volume from the canonical
   checkout root. Environment-dependent fixture placement; the owning
   session's canonical checkout passes. Left to them.
6. **Pre-existing clippy residue observed, not owned**: a fresh
   `-p`-cleaned `cargo clippy -p riir-clippy --lib` at origin/develop
   reports ~29 pedantic doc_markdown/format_push_string findings in
   `src/domains/rust_perf/proposers/` (Batch-150/151 files) that a stale
   diagnostic replay on the main checkout does not show. This landing's
   files are clean (0 findings in `frame_time_report.rs` + the heal.rs arms
   at default features). Left to the mining session that owns those files.

## Numbering note

`.benchmarks/094` was dual-allocated upstream mid-session (the idle-queue
session's `094_default_level_fired_histogram.md` + its highwater 94 pushed
while this bench was being written) — the numbering check fired, this
record took **095** and bumped the highwater to 95 in the same commit.
