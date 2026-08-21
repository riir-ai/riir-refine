# Plan 019 — `rustc_errors` domain: `cargo heal --fix-compile` (oracle-driven compile-error repair + self-evolving real-use loop)

**Date:** 2026-08-21
**Source:** owner call, same day. Motivating evidence — the E0597 incident
(riir-dapps `settlement_goat.rs`): a confident narrative root-cause ("the Box
coercion resolves the object lifetime to `'static`") shipped in a fix commit,
then FAILED reproduction — the original committed code compiles clean on
rustc 1.93/1.95/1.97.0/1.97.1/stable × `chain_backend`/`--all-features`. The
error came from an intermediate editor state; the narrative was fabricated
certainty. **Lesson encoded:** when reasoning and the compiler-oracle
disagree, the oracle wins — and reasoning without an oracle in the loop
doesn't just fail, it invents. This plan builds the oracle-in-the-loop
system.

**Product thesis:** rustc emits NO machine-applicable suggestions for the
borrowck family (E0597/E0596/E0502/E0499/E0505/E0507/E0382) — `cargo fix` is
structurally blind there; every LLM tool pays inference to fix exactly these
errors. We fix them **modelless**: enumerate playbook candidate edits → apply
→ `cargo check` → keep only what compiles. **Wrong fixes are impossible by
construction** — the failure mode is "no fix found", never "fix broke your
code". Coverage is partial by design and the coverage number is published —
partial-but-never-wrong beats comprehensive-but-sometimes-wrong for a tool
that edits your code.

## What already exists (the inventory — this is why the verdict was "engineering, not research")

| Component | Shipped as | Proof |
|---|---|---|
| Oracle loop (apply → cargo check → revert bad edits, line-mapped span-level) | `fix_verify` (Issue 015, Bench 031) | thousands of edit-landings across the workspace sweeps; reverted 2 real breakages (E0308 + E0631) on its first GOAT run |
| cargo JSON diagnostic parsing (spans, codes, rendered messages) | `ClippyOracle` (T3.5.5) | parses real `cargo clippy` JSON today; `cargo check` JSON is the same format |
| Candidate ranking / min-complexity preference | L3 `ast_delta_size` (Issue 005) | the "prefer the conservative fix" ordering among compiling candidates |
| Learning log + trajectory memory | `EvolveRecorder` → `memory.json` (Issue 018, BLAKE3-committed since T7.4) | persists across runs, per-(lint,fix) outcome buckets |
| Selection policy that learns | `SelectionMode::BetaPosterior` (T9.3, Bench 028) | −8.4% selection regret in the thin-evidence regime — exactly the playbook regime |
| Mutation-generated self-labeled eval | `mutate_shape` (kernel self-variant probes, Issue 017) | broken-code-with-known-inverse-fix, no hand labeling |
| **Candidate generator keyed by rustc error code** | ❌ **the missing piece — this plan** | playbook templates, same class as the 40 clippy proposers |

## Architecture

The 4th shipped domain (`clippy_lints`, `rust_perf`, `kernel_opt` → +
`rustc_errors`; `sec` is Plan 018, not yet landed). Differences from the
existing domains, stated up front:

- **Keying is exact, not latent.** The diagnostic carries the error code —
  no retrieval needed to FIND the playbook. Latent/memory ordering matters
  only for MOVE SELECTION within a family (Phase 4).
- **The driver is a loop, not a one-shot.** `cargo heal --fix-compile`:
  check → parse → propose → apply (min-delta first) → recheck → repeat,
  bounded `--max-iters` (default 5). Errors often cascade; one fix exposes
  the next.
- **Write semantics are clean from day one.** `--fix-compile` NEVER writes
  without `--write`; there is no verify-mode-writes-unconditionally trap
  (the recorded `--fix --verify` gotcha does not carry over).

New files:
```
src/domains/rustc_errors/mod.rs        # domain impl (HealDomain) + corpus entry shape
src/domains/rustc_errors/diag.rs       # cargo check --message-format=json → RustcErrorDiag
src/domains/rustc_errors/driver.rs     # the bounded fix loop
src/domains/rustc_errors/playbooks/    # per-family syn proposers (move templates)
                                       #   e0597.rs (first), borrowck.rs (rest)
scripts/watch_heal.sh                  # real-use watch wrapper (Phase 5)
tests/fix_compile_gates.rs             # GOAT gates
tests/mutations/                       # generated eval corpus (Phase 6)
```

Corpus entry shape: `(error_code, strategy_id, trigger_shape, buggy_code,
fixed_code)` — same BLAKE3 freeze/thaw commitment as the other domains.
`lint_name := error_code`, `rule_id := strategy_id` for the memory layer, so
`EvolveRecorder` works unmodified.

## The playbooks (candidate generators)

Each template is a syn proposer in the established fixture-tested style
(buggy/clean pair + clean-span negative control + comment-guard where a
rewrite would displace comments — the recorded divergence discipline).
A template may ALSO be tried blind when the oracle can answer what analysis
can't (e.g. "is this `Clone`?" — apply `move`+`.clone()`, let cargo check
arbitrate; revert is one iteration).

**E0597 (borrow outlives owner) — the motivating family, shipped first:**
1. `move` closure (+ span-scan: binding unused after the closure)
2. `move` + `.clone()` for multi-capture bindings (oracle-arbitrated Clone)
3. drop-the-box restructure (`&mut dyn FnMut()` runner family — the shape
   that fixed the incident)
4. explicit object-lifetime coercion (`as Box<dyn Fn + '_>`)
5. `Rc`/`Arc` shared ownership
6. scope restructuring (NLL: move the declaration down)
7. ~~`Box::leak`~~ — **excluded**: compiles, tests pass, leaks. Never
   auto-applied; documented as the canonical wrong-but-compiling example.

**Then the rest of the borrowck family (Phase 3):** E0596 (`&mut`), E0502
(borrow+move), E0499 (double mutable), E0505 (move out of borrow), E0507
(`.clone()` out of ref), E0382 (clone/borrow). ~30–40 templates total.

## Tasks

### Phase 0 — substrate-first gate (run BEFORE any implementation; global-rule mandate)

- [ ] T0.1 Vocabulary-translation grep across `*.rs` in all workspace repos
  (3+ name variants each): "repair loop", "error healing", "compile fix",
  "rustc json", "diagnostic parse", "E0597", "borrowck fix" — verify no
  parallel system exists beyond `fix_verify`/`ClippyOracle` (which this
  plan CONSUMES, not duplicates). Record the grep table in the bench doc.
- [ ] T0.2 Confirm `ClippyOracle`'s JSON machinery is reusable for
  `cargo check` diagnostics (error-code field, spans, children) — extend,
  don't fork.

### Phase 1 — the oracle driver

- [ ] T1.1 `diag.rs`: run `cargo check --message-format=json` (with
  `--verify-args` passthrough for feature sets), parse to `RustcErrorDiag
  { code, primary_span, spans, rendered }`, dedupe multi-span repeats.
- [ ] T1.2 `driver.rs`: the bounded loop — collect errors → group by file →
  propose per error (Phase 2/3 templates) → order candidates (memory
  BetaPosterior, AST-delta tiebreak) → apply best → recheck → repeat ≤
  `--max-iters` (default 5). Stop early when clean.
- [ ] T1.3 CLI: `cargo heal --fix-compile [--write] [--max-iters N]
  [--verify-args "..."] [PATHS]` — review mode prints the plan per file;
  `--write` applies. No unconditional-write path, verified by test.
- [ ] T1.4 Revert path: a candidate whose application leaves the file
  non-compiling is reverted before the next candidate (the `fix_verify`
  semantics, per-error-site).

### Phase 2 — E0597 vertical slice to REAL USE

- [ ] T2.1 Templates 1–6 above, each with fixture pairs + negative controls
  (`tests` per the proposer discipline; the `Box::leak` exclusion pinned by
  a never-emitted test).
- [ ] T2.2 End-to-end: a real E0597 (the `settlement_goat.rs` shape, with
  `Vec<Case<'static>>` as the injected broken state) → `--fix-compile
  --write` → compiles, tests pass, diff is the min-delta candidate.
- [ ] T2.3 The incident's lesson as a standing gate: probe-1 = unchanged
  code that ALREADY compiles → driver reports "nothing to fix", zero edits
  (the check that would have caught my unnecessary fix).

### Phase 3 — the rest of the borrowck family

- [ ] T3.1 E0507 + E0382 (`.clone()` / borrow) templates + fixtures.
- [ ] T3.2 E0596 + E0502 + E0499 + E0505 templates + fixtures.
- [ ] T3.3 Cascade handling: fixes whose application reveals the NEXT error
  (bounded by `--max-iters`); at least one two-level cascade fixture.

### Phase 4 — self-evolution around every fix (the owner requirement: "ensure we self evolve around this")

- [ ] T4.1 Trajectories: every applied/reverted candidate → `EvolveRecorder`
  (`record_applied` / `record_outcome(verified = cargo-check-passed)`) —
  outcome buckets keyed `(error_code, strategy_id)`. Evidential tiers apply
  (oracle-passed = the strongest tier we have — better than the clippy
  path's parse-only successes).
- [ ] T4.2 Move-ordering policy: candidates ordered by BetaPosterior over
  `(error_code, strategy_id)` from memory, AST-delta tiebreak, unseen
  strategy prior 0.32 (the T9.3 shape). Measured benefit required at G6 —
  if ordering shows no gain on the corpus, record negative + keep static
  min-delta ordering (the honest T9.4 precedent).
- [ ] T4.3 Per-repo memory + manual-mining log: watch wrapper pins
  `RIIR_HEAL_MEMORY=<repo>/.heal/memory.json` AND appends every fix to
  `<repo>/.heal/fix_log.jsonl` (one line per event: ts, file, error_code,
  strategy, applied|reverted, iter). This is the "log to some folder and we
  manual grab it later" surface — grep-able, no tooling needed.
- [ ] T4.4 **Miss capture — the corpus-growth loop:** when NO template fixes
  an error, dump the full diagnostic + source excerpt to
  `<repo>/.heal/misses/E0597-<ts>.md`. Misses are the authoring queue for
  the next playbook templates (mine via git-log fix-pairs, the `wgsl_mine`
  precedent). A standing `misses/README.md` documents the manual-grab
  workflow.
- [ ] T4.5 EMA direction vectors for template retrieval: DEFER unless
  shape-matching within a family proves insufficient (error code is exact;
  the refuted-over-learning precedents — T9.4, `gemma2_directions` — are
  binding).

### Phase 5 — real use: the watch loop

- [ ] T5.1 `scripts/watch_heal.sh` — wrapper over `watchexec` (preferred) or
  `cargo watch`; per invocation: scope to SESSION-TOUCHED files (git
  diff-name-only vs HEAD, or an explicit file list — NEVER whole-repo on
  shared checkouts, the recorded sibling-collision discipline), run
  `cargo heal --fix-compile --write --verify-args "<repo feature set>"`,
  pin `.heal/memory.json` + `fix_log.jsonl`.
- [ ] T5.2 README section "Watch mode (real use)" documenting the three
  recorded caveats as REQUIREMENTS of any wrapper: (1) write semantics
  clean (`--fix-compile` has no unconditional-write trap — assert by test),
  (2) feature-gate blindness → `--verify-args` MUST carry the repo's real
  feature set (the Issue 031 H3 class), (3) sibling agents → session file
  scoping, never whole-repo.
- [ ] T5.3 `CARGO_TARGET_DIR` isolation guidance for concurrent sibling
  builds (the global rule) in the wrapper's header comment.
- [ ] T5.4 Built-in `--watch` (notify crate): DEFER — the wrapper is the
  real-use surface; a new dep must beat it measurably first.

### Phase 6 — GOAT + record

- [ ] T6.1 Mutation corpus: extend `mutate_shape` to Rust-level mutators
  (drop `&`, add `move`, `'static` injection, swap borrow for move, …) over
  a sample of THIS repo's + katgpt-rs's compiling code → N≥200 labeled
  (broken, known-inverse-fix) pairs under `tests/mutations/` (generated,
  committed for determinism).
- [ ] T6.2 **G1 correctness:** every shipped fix compiles (by construction —
  assert the invariant anyway) + `cargo test` green post-fix on the corpus
  + the T2.3 already-compiles no-op gate.
- [ ] T6.3 **G2 latency:** per-iteration time (incremental `cargo check`
  dominates; report p50/p99 per iteration + per fix, release mode).
- [ ] T6.4 **G3 no-regression:** default + all-features lib suites +
  clippy 0 + existing retrieval floors unchanged.
- [ ] T6.5 **G4 alloc:** the propose/apply path itself alloc-free
  steady-state (counting allocator, the perf-bench axis) — cargo-check
  subprocess cost is out of scope (it's the oracle, not the healer).
- [ ] T6.6 **G-coverage (the honest product metric):** % of the mutation
  corpus fixed, per error family, PUBLISHED in the bench doc + README. Also
  the never-wrong number: non-compiling edits surviving a run = 0 (pinned
  by test, incl. a deliberately-bad template that MUST be reverted).
- [ ] T6.7 **G6/G7 self-evolve:** (a) trajectories persist across runs
  (memory round-trip); (b) move ordering measurably improves selection on
  the corpus after a learning pass (fewer candidates-to-first-success) —
  or honest negative recorded (T4.2's clause); (c) miss-capture produces a
  readable authoring queue from seeded unfixed errors.
- [ ] T6.8 Bench doc + AGENTS.md feature-matrix row (`rustc_errors`,
  default-off until G-gates pass; promotion per the modelless-gain rule —
  oracle-gated compile-fixing IS the modelless gain) + mining-queue
  snapshot pointer.

## Non-goals

- **E0308/E0277 long tail** (type mismatch, trait bounds) — the biggest
  volume but semantic, not mechanical. The L4 fallback seam exists for the
  tail; modelless covers the head. Revisit per-family only with mutation
  evidence.
- **No LLM anywhere in the loop.** The L4 seam remains the escalation path,
  unchanged.
- **No `Box::leak`-class wrong-but-compiling fixes.** Ever. The exclusion
  list grows as wrong-but-compiling shapes are discovered; each gets a
  never-emitted test.
- **No built-in watcher dependency** (`notify`) until the wrapper is
  measurably insufficient.
- **No cross-crate/project-wide restructures** — single-file edits only
  (multi-file fixes change review economics; defer with evidence).

## Honest caveats (recorded up front, not discovered later)

- Coverage will be partial. The pitch is never-wrong + free, not
  universal. The mutation corpus bounds the claim; real-use misses feed
  growth via T4.4.
- The oracle is `cargo check` — the same feature-set blindness as
  `fix_verify` (a fix can compile under default features and break a gated
  config). `--verify-args` is the mitigation; the wrapper requires it.
- Editor-intermediate states (the incident's likely origin) may emit
  diagnostics the playbook can't parse — the driver treats unparseable
  diagnostics as misses (T4.4), never crashes.
- `cargo fix` parity: where rustc DOES emit machine-applicable suggestions,
  we do not compete — the playbook triggers only on codes rustc won't fix.
