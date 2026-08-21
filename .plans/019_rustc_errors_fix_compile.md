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

**Product thesis (REVISED 2026-08-21 — the original was refuted by the
oracle; see the section below).** The thesis as first written ("rustc emits NO
machine-applicable suggestions for the borrowck family
E0597/E0596/E0502/E0499/E0505/E0507/E0382; `cargo fix` is structurally blind
there") is FALSE on 4 of the 7 codes and was refuted the same day by the very
instrument this plan is built around. The surviving thesis has two parts:

1. **Uncontested territory (`cargo fix` is genuinely blind):** E0597 (true
   local-lifetime), E0499, E0502. Zero suggestion spans, zero `cargo fix`
   edits — measured. This is the head we own outright, and it INCLUDES the
   motivating incident family, so the Phase 2 vertical slice stands.
2. **Contested territory (rustc already ships a MachineApplicable fix):**
   E0382, E0505, E0507, E0596, and E0373 (the closure-outlives shape). Here
   the differentiator is NOT "we can fix it" — it is that rustc offers exactly
   ONE suggestion, applied blind, and that suggestion is frequently a
   semantics-changing `.clone()` (`&v.clone()` for E0505 inserts a temporary
   whose borrow no longer aliases `v` at all). Our claim there is *ranked
   alternatives + revert safety*, and it must be MEASURED against
   `cargo fix --broken-code` as the floor (Issue 035) — not assumed.

We fix both classes **modelless**: enumerate playbook candidate edits → apply
→ `cargo check` → keep only what compiles. Coverage is partial by design and
the coverage number is published.

**"Wrong fixes are impossible by construction" is OVERSTATED and is hereby
downgraded.** What the oracle guarantees is *compiles*, not *means the same
thing*. `.clone()` breaks aliasing, `move` transfers ownership out of the
enclosing scope, and scope-restructuring changes drop order and therefore
observable `Drop` timing. `Box::leak` is not a lone outlier — it is the
loudest member of a whole class. The honest claim is: **never leaves your tree
non-compiling** (that IS by construction), and **never applies a fix from the
semantics-changing exclusion class without saying so in the diff**.

## Oracle verdict on the premise (measured 2026-08-21, rustc 1.93.0 stable)

Per this plan's own founding lesson — *when reasoning and the compiler-oracle
disagree, the oracle wins* — the premise was put to the oracle BEFORE any
implementation. A probe crate with one minimal reproduction per error code was
checked with `cargo check --message-format=json` (counting spans carrying
`suggested_replacement` + their `suggestion_applicability`), then repaired with
`cargo fix --broken-code --allow-dirty --allow-no-vcs` and diffed.

| Code | rustc suggestion | `cargo fix --broken-code` result | Territory |
|---|---|---|---|
| E0597 (true local lifetime) | 0 spans | no change | **ours — uncontested** |
| E0499 (two `&mut`) | 0 spans | no change | **ours — uncontested** |
| E0502 (borrow + `&mut`) | 0 spans | no change | **ours — uncontested** |
| E0382 (use of moved) | 1, `MachineApplicable` `.clone()` | fixed: `let w = v.clone();` | contested |
| E0505 (move out of borrowed) | 1, `MachineApplicable` `.clone()` | fixed: `let r = &v.clone();` | contested |
| E0507 (move out of `&`) | 1, `MachineApplicable` `.clone()` | fixed: `s.v.clone()` | contested |
| E0596 (borrow as mutable) | 1, `MachineApplicable` `mut ` | fixed: `let mut x = v;` | contested |
| E0373 (closure outlives) | 1, `MachineApplicable` `move ` | fixed: `Box::new(move || ...)` | contested |

Three consequences, all binding on the tasks below:

- **The addressable head is 3 codes, not 7.** Any claim of "~30–40 templates"
  across the borrowck family must first prove per-family that it beats
  `cargo fix`; templates for contested codes are speculative until then.
- **Playbook template 1 (`move` closure) is already `cargo fix`'s job.** The
  E0597-shaped closure case (`Box<dyn Fn>` capturing a local) is reported as
  **E0373**, not E0597, and rustc's `move ` suggestion is MachineApplicable
  and applied automatically. Demoted accordingly.
- **`cargo fix --broken-code` is the mandatory floor**, exactly as the
  conformal-naive floor is for UQ primitives. A family we do not beat, we
  delegate to `cargo fix` and ship no template. Gate: G-floor (T6.9),
  tracked in Issue 035.

Repro: probe sources + the JSON extractor are reproduced in Issue 035; rerun on
toolchain bumps, since the contested set only ever GROWS as rustc improves —
this table has a shelf life and must be re-measured, not cited.

## What already exists (the inventory — this is why the verdict was "engineering, not research")

| Component | Shipped as | Proof |
|---|---|---|
| Oracle loop (apply → cargo check → revert bad edits, line-mapped span-level) | `fix_verify` (Issue 015, Bench 031) | thousands of edit-landings across the workspace sweeps; reverted 2 real breakages (E0308 + E0631) on its first GOAT run |
| cargo JSON diagnostic parsing (spans, codes, rendered messages) | `ClippyOracle` (T3.5.5) | parses real `cargo clippy` JSON today; `cargo check` JSON is the same format |
| Candidate ranking / min-complexity preference | L3 `ast_delta_size` (Issue 005) | the "prefer the conservative fix" ordering among compiling candidates |
| Learning log + trajectory memory | `EvolveRecorder` → `memory.json` (Issue 018, BLAKE3-committed since T7.4) | persists across runs, per-(lint,fix) outcome buckets |
| **Durable indexed trajectory storage** | riir-neuron-db `LocalKvStore` + `Bm25Index` + `DenseEmbedIndex` | the Warm-tier substrate already in production (riir-dapps, riir-games); transitively present under `latent_retrieval` — ZERO new dep surface |
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
src/traj_store.rs                      # NeuronBackend for trajectories (behind
                                       #   latent_retrieval; JsonBackend stays the
                                       #   zero-dep default — one active backend)
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
1. ~~`move` closure (+ span-scan: binding unused after the closure)~~ —
   **DEMOTED**: this shape is diagnosed as E0373 and `cargo fix` already
   applies `move ` (MachineApplicable). Ship only if the floor gate (T6.9)
   shows a case rustc's single suggestion gets wrong.
2. `move` + `.clone()` for multi-capture bindings (oracle-arbitrated Clone)
3. drop-the-box restructure (`&mut dyn FnMut()` runner family — the shape
   that fixed the incident)
4. explicit object-lifetime coercion (`as Box<dyn Fn + '_>`)
5. `Rc`/`Arc` shared ownership
6. scope restructuring (NLL: move the declaration down)
7. ~~`Box::leak`~~ — **excluded**: compiles, tests pass, leaks. Never
   auto-applied; documented as the canonical wrong-but-compiling example.

**Then the rest of the borrowck family (Phase 3), split by territory:**
- **Uncontested, build unconditionally:** E0499 (double mutable), E0502
  (borrow + `&mut`). No rustc suggestion exists; any coverage is net-new.
- **Contested, build ONLY after the floor gate:** E0596 (`&mut`), E0505 (move
  out of borrow), E0507 (`.clone()` out of ref), E0382 (clone/borrow). Each
  needs a measured win over `cargo fix --broken-code` (T6.9) BEFORE templates
  are authored. The template-count estimate is withdrawn — "~30–40" was sized
  against a 7-code market that measurement cut to 3.

## Tasks

### Phase 0 — substrate-first gate (run BEFORE any implementation; global-rule mandate)

- [x] T0.1 Vocabulary-translation grep across `*.rs` in all workspace repos
  (3+ name variants each): "repair loop", "error healing", "compile fix",
  "rustc json", "diagnostic parse", "E0597", "borrowck fix" — verify no
  parallel system exists beyond `fix_verify`/`ClippyOracle` (which this
  plan CONSUMES, not duplicates). Record the grep table in the bench doc.
  **DONE 2026-08-21 (multi-root workspace grep, 3 pattern families ×
  `*.rs`).** Verdict: **no parallel system — PASS.** Hit families:

  | Family (variants greped) | Hits | Classification |
  |---|---|---|
  | `repair loop / error healing / fix-compile / broken-code / compile repair / RustcErrorDiag` | riir-clippy `rustc_errors/{diag,probe,floor}.rs` + `tests/premise_probe.rs` + `examples/rustc_errors_floor.rs` | the CONSUMED substrate (Issue 035 T1/T2) — expected |
  | same | riir-clippy `fix_verify.rs` (apply→check→revert oracle), `clippy_lints/verify.rs`, `draft/clippy_oracle.rs`, `rust_perf/verify.rs` | the consumed/existing in-crate JSON consumers — expected |
  | `message-format=json / compiler-message / parse_diag / cargo check --message` | **riir-train ×3**: `src/rlvr_reward.rs` + examples `bonsai_clippy_l4_sft_train.rs` / `clippy_l4_rlvr_train.rs` — inline clippy-JSON lint-code parse (~50 LOC, one site self-documents "reimplemented inline to avoid pulling riir-clippy as a dependency") | **boundary-correct duplicate (architectural)** — riir-train's allowlist has no riir-clippy dep (pattern-borrow ≠ dependency); consumer is the L4 RLVR reward signal, NOT compile repair. No action; recorded here as the one known out-of-crate sibling of the diagnostic-parse family |
  | `E0499/E0502/E0505/E0507/E0382/E0596/E0597/E0373` | riir-ai comments explaining borrow-workarounds (dllm, lm_head_lora_gpu, crowd_motion_bridge) + riir-clippy proposer guard comments | prose ABOUT the codes, not repair systems |
  | all families | katgpt-rs / riir-chain / riir-neuron-db / riir-game-sdk / mmorpg-examples / viewbridge / dapps: 0 relevant hits (one katgpt-percepta "compile pipeline" doc-comment false positive) | clean |
- [x] T0.2 Confirm `ClippyOracle`'s JSON machinery is reusable for
  `cargo check` diagnostics (error-code field, spans, children) — extend,
  don't fork.
  **DONE 2026-08-21 — verdict: NOT the right base, and no fork is needed;
  the division of labor already landed.** `ClippyOracle::parse_verdict` is
  a lint-LABEL extractor: `clippy::`-prefix codes only (rustc E-codes are
  silently dropped — no prefix match), no spans, no children walk;
  error-level messages only set the `compiles=false` flag. The three
  fields this domain keys on (E-code, primary span, help-child
  suggestions) are exactly the three it discards — "extending" it would
  change `OracleVerdict`'s contract and perturb the T3.5.5 eval fixtures,
  i.e. an extension that is a fork in disguise. The genuinely reusable
  machinery exists elsewhere and is CONSUMED, not duplicated:
  `rustc_errors/diag.rs` (Issue 035 T1) owns the coded-error parse
  (spans + recursive child suggestions, pinned by the `E0596_LINE`
  fixture — "the shape a non-recursive walk misses"), and `fix_verify.rs`'s
  `parse_error_locations` + line-mapped revert IS the apply→check→revert
  core (the pending `compile_oracle` extraction per T0.3). **Binding on
  Phase 1:** the driver consumes `diag.rs` for parsing + `fix_verify`'s
  core for apply/revert and authors NO new parser. In-crate JSON-consumer
  census recorded (5 sites, one wire format, different fidelities):
  `fix_verify` (error locations for revert), `clippy_oracle` (eval labels),
  `clippy_lints/verify` (lint codes), `rust_perf/verify` (error count),
  `rustc_errors/diag` (coded errors + suggestions) — the consolidation
  boundary is the DRIVER, not a shared mega-parser; coupling the eval
  harness to the repair parse would let one's changes perturb the other's
  pinned tests.
- [x] T0.3 **Feature graph** (DONE 2026-08-21) — name the flag `rustc_errors` and keep it OFF
  the `latent_retrieval` dep tree. Verified 2026-08-21: `src/fix_verify.rs`
  imports only `std` + `serde` (zero `riir_rag` / `riir_neuron_db`), so
  `fix_verify = ["latent_retrieval"]` is a CLI-surface implication, not a code
  dependency. This domain is exact-keyed by error code and needs no retrieval;
  extract the apply→check→revert core behind a retrieval-free
  `compile_oracle` feature that `fix_verify` and `rustc_errors` both consume,
  rather than dragging riir-rag + riir-neuron-db into a compile fixer.
  **Shipped:** `rustc_errors = []` in Cargo.toml with zero implications; the
  measurement layer (`diag`/`probe`/`floor`) uses std + serde only. The
  `compile_oracle` extraction is still pending — needed only when the driver
  consumes `fix_verify`'s apply→check→revert core (Phase 1).
- [x] T0.4 Re-run the premise probe (Issue 035) on the repo's pinned toolchain
  and pin the measured table as a test, so a rustc bump that moves a code from
  uncontested → contested fails loudly instead of silently invalidating the
  product claim. **DONE 2026-08-21** — `tests/premise_probe.rs`, 6 tests green
  on the pin (1.97.0) AND on 1.93.0. The table held on both; see Bench 041.

### Phase 1 — the oracle driver

- [x] T1.1 `diag.rs`: run `cargo check --message-format=json` (with
  `--verify-args` passthrough for feature sets), parse to `RustcErrorDiag
  { code, primary_span, spans, rendered }`, dedupe multi-span repeats.
  **DONE 2026-08-21.** The run lives in the shared `compile_oracle`
  substrate (`CheckRunner` — T0.3's pending extraction, landed with this
  phase: `fix_verify` delegates its run_check to it; the driver consumes
  it with `--all-targets` + extra-args passthrough). Parse gained the
  `rendered` field + byte offsets on suggestion spans
  (`parses_rendered_and_byte_offsets`), and `dedupe_by_site` collapses
  same-`(code, file, line)` repeats merging differing suggestions
  (`dedupe_by_site_merges_repeats`).
- [x] T1.2 `driver.rs`: the bounded loop — collect errors → group by file →
  propose per error (Phase 2/3 templates) → order candidates (memory
  BetaPosterior, AST-delta tiebreak) → apply best → recheck → repeat ≤
  `--max-iters` (default 5). Stop early when clean.
  **DONE 2026-08-21.** `CompileDriver::run` — ordering is the Phase 1
  static min-delta seam (`order_candidates`: file → line → code →
  shortest replacement → strategy id; `BetaPosterior` is T4.2's swap,
  measured-benefit-gated). Keep criterion: coded-error count strictly
  decreased; everything else reverts byte-exact. E2E-pinned by
  `tests/fix_compile.rs` (applied-to-clean / reverted-byte-exact /
  nothing-on-clean / miss / review-plans — real cargo, real E0596/E0597).
- [x] T1.3 CLI: `cargo heal --fix-compile [--write] [--max-iters N]
  [--verify-args "..."] [PATHS]` — review mode prints the plan per file;
  `--write` applies. No unconditional-write path, verified by test.
  **DONE 2026-08-21.** Early-exit arm in `main` (before the matcher — no
  retrieval). Review mode prints `plan` lines + misses; write mode prints
  applied/reverted + the summary. `review_mode_plans_without_writing`
  pins no-write; the CLI smoke (review + write on a real E0596 scaffold,
  empty registry) exits 0 with the file untouched.
- [x] T1.4 Revert path: a candidate whose application leaves the file
  non-compiling is reverted before the next candidate (the `fix_verify`
  semantics, per-error-site).
  **DONE 2026-08-21** — generalized to "no strict progress" (a fix that
  trades one error for another — the `Box::leak` shape — must not survive
  arbitration either; stricter than non-compiling-only, by design).
  `driver_reverts_no_progress_candidate` pins byte-exact restore.
- [x] T1.5 **CLI validation matrix** — `src/bin/heal.rs` currently rejects
  `--write` unless `--fix` is set and `--verify-args` unless `--verify` is set
  (the `--write requires --fix` / `--verify-args requires --verify` guards).
  As written, `--fix-compile --write --verify-args "..."` is rejected by the
  existing argument checks. Extend both guards to accept `--fix-compile` as a
  satisfying mode, and pin each accepted/rejected combination by test — this
  is a silent-usability trap, not a compile error.
  **DONE 2026-08-21.** `validate_mode_flags` extracted; `mode_flag_tests`
  pins 9 accepted + 5 rejected combinations (incl. the trap:
  `--fix-compile --write --verify-args "..."` now OK; `--verify` still
  requires `--fix` — its batch semantics are `--fix`'s, not the driver's).

### Phase 2 — E0597 vertical slice to REAL USE

- [x] T2.1 Templates 1–6 above, each with fixture pairs + negative controls
  (`tests` per the proposer discipline; the `Box::leak` exclusion pinned by
  a never-emitted test).
  **DONE 2026-08-21** — `playbooks/e0597.rs`: six proposers (templates
  2–6; template 1 stays demoted to `cargo fix`'s E0373 territory), 17
  unit fixtures incl. negative controls + the
  `box_leak_is_never_emitted` pin. **Oracle-first discipline held**: every
  FIXED form was probe-verified to compile on rustc 1.93 BEFORE becoming a
  fixture — and the probe refuted a textbook assumption en route (`let
  f: Box<dyn Fn…> = Box::new(closure-borrowing-local)` now COMPILES on
  1.93; the live shapes are the explicit-`'static` annotation + the
  genuine scope escape + coercion-site captures). New anchor surface:
  `RustcErrorDiag.labeled_spans` (the oracle's own labeled spans with byte
  offsets — "type annotation requires…'static" indicts the annotation,
  "value captured here" covers `||`, "binding `x` declared here",
  "dropped here while still borrowed" covers the block's `}`). A rustc
  that changes label texts makes the family go SILENT (misses), never
  wrong — the floor gate re-measures on toolchain bumps.
- [x] T2.2 End-to-end: a real E0597 (the `settlement_goat.rs` shape, with
  `Vec<Case<'static>>` as the injected broken state) → `--fix-compile
  --write` → compiles, tests pass, diff is the min-delta candidate.
  **DONE** — `tests/fix_compile_e0597.rs::incident_shape_fixed_to_clean_with_min_delta`:
  the incident shape → CLEAN via `e0597-anon-lifetime` (the `'static` →
  `'_` token swap), the scaffold's own `cargo test` passes, `Vec<Case<'_>>`
  on disk. Plus `scope_escape_hoisted_to_clean` (template 6, byte-exact
  hoist form) and `ladder_reverts_move_and_keeps_move_clone` (the product
  claim e2e: `move` tried → E0382 no-progress → byte-exact revert →
  `move`+`.clone()` kept → CLEAN + tests pass). Live CLI verified:
  `applied E0597 [e0597-anon-lifetime] (errors 1 → 0) — CLEAN`.
- [x] T2.3 The incident's lesson as a standing gate: probe-1 = unchanged
  code that ALREADY compiles → driver reports "nothing to fix", zero edits
  (the check that would have caught my unnecessary fix).
  **DONE** — driver-level pin landed with Phase 1
  (`fix_compile.rs::driver_reports_nothing_on_clean_crate`); Phase 2 adds
  the family-level pin
  (`fix_compile_e0597.rs::already_compiling_incident_tree_is_untouched`:
  the incident's FIXED tree re-run through the real family — one check,
  zero iterations, zero edits, byte-identical file).

### Phase 3 — the rest of the borrowck family

- [-] T3.1 E0507 + E0382 (`.clone()` / borrow) templates + fixtures.
  **RESOLVED-NEGATIVE by the floor gate (T6.9, Bench 041):** `cargo fix
  --broken-code` scores 100% MATCHED on both (no divergence, no headroom).
  A template that cannot beat the floor does not ship — the gate, not
  authoring ambition, decides the inventory. Re-opened only if a
  toolchain bump regresses the floor (the premise probe is the standing
  tripwire).
- [x] T3.2 E0596 + E0502 + E0499 + E0505 templates + fixtures.
  **DONE 2026-08-22** (`playbooks/borrowck.rs`) — scoped by measurement:
  E0596 ships NO template (floor 100% matched × 2 — delegated); E0502
  ships deref-removal (the atomic `&`+`*` deletion PAIR via the new
  `FixCandidate::extra_edits` — half-landing trades E0502 for E0614, so
  atomicity IS the correctness argument) + clone-out (`&e` → `e.clone()`,
  the non-`Copy` rung); E0499 ships the decl/use swap (the adjacent-only
  reorder; foreign-statement relocation refused); E0505 ships
  clone-at-move (the contested-opening win: rustc's own
  `MachineApplicable` fix rewrites the BORROW site — `let r = &v.clone()`,
  aliasing silently changed — ours matches the known inverse). Oracle-
  first discipline held: every label + every FIXED form probed on rustc
  1.97.0 before authoring (the E0502 "immutable borrow" label covers the
  borrowed PLACE, not the `&` — coverage varies by shape, so the
  playbooks locate the `let` via syn and never splice on the label span).
  13 unit fixtures incl. negative controls + the family no-leak pin.
- [x] T3.3 Cascade handling: fixes whose application reveals the NEXT error
  (bounded by `--max-iters`); at least one two-level cascade fixture.
  **DONE 2026-08-22** —
  `fix_compile_borrowck.rs::cascade_two_e0502_errors_fixed_across_iterations`:
  two independent E0502 errors, one fixed per iteration (2→1→0); the
  first kept fix invalidates the first pass's byte spans and only
  re-proposal (the Phase 1 loop design) keeps iteration 2's spans valid —
  asserted via `report.iterations >= 2` + both seeds restored on disk.

### Phase 4 — self-evolution around every fix (the owner requirement: "ensure we self evolve around this")

- [x] T4.1 Trajectories: every applied/reverted candidate → `EvolveRecorder`
  (`record_applied` / `record_outcome(verified = cargo-check-passed)`) —
  outcome buckets keyed `(error_code, strategy_id)`. Evidential tiers apply
  (oracle-passed = the strongest tier we have — better than the clippy
  path's parse-only successes).
  **DONE 2026-08-22** (`rustc_errors/evolve.rs` + CLI wiring). Trajectory
  key `"{code}:{strategy_id}"` (the plan's lint_name/rule_id contract
  folded into the memory's single id — strategy ids are already
  family-prefixed so greps stay unambiguous). **Verdict soundness held:**
  applied → `record_outcome_verified(success, verified=TRUE)` — the keep
  criterion IS a full cargo-check verdict (Certified tier, the only
  promotion path); reverted → `record_outcome(success=false)` (counts
  toward the Withdrawn streak). Driver records now carry the trajectory
  halves (`AppliedRecord.shape/fix`, `RevertedRecord.shape/fix` — the
  trigger text + replacement); only the WRITE path records (review runs
  touch nothing, the Issue 018 rule). Live-verified end-to-end: applied
  fix → `tier: Certified` trajectory in the store → `--dump-trajectories`
  exports it.
- [x] T4.2 **Shape-conditional move-ordering policy (owner direction:
  consume the trajectory store directly):** on propose, hybrid-retrieve
  k-nearest past trajectories for the current error shape (BM25 lexical +
  8-D KNN via `DenseEmbedIndex`/`ModellessEmbedder` — the
  NeuronDbRetriever composition) → BetaPosterior prior per strategy
  weighted by NEIGHBOR OUTCOMES (relevance-weighted, not flat family
  counters — flat `(code,strategy)` buckets cannot distinguish
  E0597-boxed-closure from E0597-returns-ref; retrieved neighbors can).
  AST-delta tiebreak, unseen-strategy prior 0.32, Withdrawn excluded from
  retrieval (the T7.4 rule). Measured benefit required at G6 — if ordering
  shows no gain on the corpus, record negative + keep static min-delta
  ordering (the honest T9.4 precedent). Falls back to flat
  `(error_code, strategy_id)` BetaPosterior when the JsonBackend is active
  (default build).
  **MECHANISM SHIPPED 2026-08-22** (`traj_store::shape_conditional_order`
  + the driver's `with_ordering` hook — a closure seam, so the
  `compile_oracle`-gated driver never references the `latent_retrieval`-
  gated store type). Candidates carry their trigger `shape` (driver-filled
  at proposal); the ordering = neighbor-weighted `best_belief_score`
  pseudo-counts DESC → min-delta tiebreak; unseen = `Beta(1,1)` @ ε=0.32;
  Withdrawn excluded upstream by `retrieve`. **Pinned by the flip test**:
  reversed store history flips the candidate order — the property static
  ordering can never have. **The G6 benefit measurement remains OPEN**
  (Phase 6 G6.7) — until it passes, this ships as the installed-but-honest
  mechanism with the negative-record clause standing.
- [x] T4.3 **Trajectory store = neuron-db substrate (primary, behind
  `latent_retrieval`):** `LocalKvStore` at `<repo>/.heal/trajectories.kv`
  — encrypted WAL (stable per-repo key derived from a generated `.heal/key`
  seed; the threat model is integrity+durability, not secrecy),
  `CompactionPolicy`-bounded growth, `CommitBatch` flushes (94.5× measured
  vs per-row fsync, Bench 472). Rows via `put_pod` keyed
  `traj:{error_code}:{strategy_id}:{blake3_12(shape)}`; family scans via
  `scan_prefix_bytes`; lexical axis `Bm25Index`; latent axis
  `DenseEmbedIndex` with the SAME deterministic `ModellessEmbedder` the
  corpus RuleIndex uses — one embedding substrate for rules + lived
  experience. Zero-dep default build keeps the `memory.json` envelope
  (Issue 018 G4: dep surface unchanged) — the tiering is the constraint,
  one active backend per build, no dual-write drift.
  **DONE 2026-08-22** (`src/traj_store.rs`, behind `latent_retrieval`) —
  with TWO substrate-first corrections recorded in the module docs:
  (1) `DenseEmbedIndex` is the 768-D Bonsai path (blocked on C13) while
  the modelless embedders are 8-D — the honest latent axis is the
  sigmoid-dot over stored 8-D directions (the memory's own retrieval
  math, made durable); (2) `put_pod` would make bytemuck a NEW direct dep
  for one struct — rows are one JSON doc each via plain `put` (the
  dep-light rule wins; rows are tens-per-family, not a hot path). What
  shipped as planned: `LocalKvStore` encrypted WAL at
  `.heal/trajectories.kv` with the `.heal/key` seed, `traj:{code}:
  {strategy}:{blake3_12(shape)}` keys, `scan_prefix` family scans, the
  `Bm25Index` lexical axis rebuilt at open, the SAME `embed_into` 8-D
  embedding, Issue 021 tier transitions durable (verified-success
  certifies, 3 consecutive failures withdraw — absorbing), one backend
  per build (the CLI picks by feature; never dual-writes).
  `CompactionPolicy`/`CommitBatch` deferred to the watch-loop unit (T5.1)
  where the flush cadence exists — single-event recording pays one WAL
  append per event either way.
- [x] T4.3b **Human grep surface = on-demand export, not a shadow log:**
  `cargo heal --dump-trajectories [--jsonl]` reads either backend →
  stdout (redirectable). One source of truth; `.heal/misses/*.md` stays
  the human authoring queue (T4.4, unchanged). The watch wrapper pins the
  store path (not a JSONL append).
  **JSON-backend half DONE 2026-08-22** — the flag is live: reads the
  v1 envelope (version-checked through `LatentFixMemory::load`), emits
  one trajectory per JSONL line (standalone mode, no paths needed,
  `--write` rejected — pinned in the mode matrix). **The neuron-backend
  reader landed the same day** (T4.3): `latent_retrieval` builds check
  `.heal/trajectories.kv` first (`RIIR_HEAL_TRAJ_ROOT` overrides the
  root), falling back to the JSON envelope — the same flag, both
  backends, one source of truth per build.
- [x] T4.4 **Miss capture — the corpus-growth loop:** when NO template fixes
  an error, dump the full diagnostic + source excerpt to
  `<repo>/.heal/misses/E0597-<ts>.md`. Misses are the authoring queue for
  the next playbook templates (mine via git-log fix-pairs, the `wgsl_mine`
  precedent). A standing `misses/README.md` documents the manual-grab
  workflow.
  **DONE 2026-08-22** (`evolve::dump_misses` + CLI wiring in the write
  path): timestamped `{code}-{nanos}.md` with the RENDERED diagnostic
  verbatim (the `rendered` field now rides `MissRecord`) + file:line +
  message; idempotent README (written once — the documented contract,
  not a growing log); empty-miss runs create nothing. Root = the
  workspace root of the first input path's owning package (cwd fallback).
  Live-verified: an E0596 (deliberately delegated code) miss produced the
  full authoring entry with rustc's own `let mut` help rendered.
- [ ] T4.5 EMA direction vectors for template retrieval: DEFER unless
  shape-matching within a family proves insufficient (error code is exact;
  the refuted-over-learning precedents — T9.4, `gemma2_directions` — are
  binding).
- [x] T4.6 Store scope: per-repo `.heal/trajectories.kv` (repo idiom
  locality) vs global `~/.cache/riir-clippy/trajectories.kv` (cross-project
  volume is where selection learning compounds) — decide via the G6
  measurement run both ways; default per-repo, `--global-traj` opt-in flag.
  **RESOLVED 2026-08-22 (Bench 045)**: the G6 ordering A/B measured NO
  headroom for ANY ordering policy on the corpus (static min-delta is
  first-try-perfect — 126/126 at 0 reverts for every arm), so there is no
  benefit to compound cross-project and no `--global-traj` flag ships.
  **Per-repo stays the default + only scope.** Revisit only if real-use
  misses (repeated shape families) reopen T4.2 with measured benefit.

### Phase 5 — real use: the watch loop

- [x] T5.1 `scripts/watch_heal.sh` — wrapper over `watchexec` (preferred) or
  `cargo watch`; per invocation: scope to SESSION-TOUCHED files (git
  diff-name-only vs HEAD, or an explicit file list — NEVER whole-repo on
  shared checkouts, the recorded sibling-collision discipline), run
  `cargo heal --fix-compile --write --verify-args "<repo feature set>"`,
  pin `.heal/memory.json` + `fix_log.jsonl`.
  **DONE 2026-08-22** — with the Phase 4 amendment applied: the wrapper
  pins the STORE (`.heal/trajectories.kv` / `memory.json`), never a
  JSONL append (the amendment's "no shadow log" rule — the plan text
  above predates it). macOS-bash-3.2-portable scope collection (no
  `mapfile`); `HEAL_BIN` overridable (the installed `cargo-heal`
  binary by default, a `cargo run` prefix for dev builds); `HEAL_FEATURES`
  defaults to `--all-features`. Live-smoked end-to-end: scope resolution →
  oracle pass (CLEAN, zero writes on a compiling tree — the T2.3 gate) →
  watch entry. **The smoke caught a real CLI bug**: `--verify-args` was
  unconditionally rejected on `fix_verify`-less builds even under
  `--fix-compile` (whose substrate is `compile_oracle`, not `fix_verify`)
  — fixed: the gate now keys on `any(fix_verify, rustc_errors)` + a
  fix_verify-less build rejects `--verify-args` only when the consumer
  is NOT `--fix-compile` (the exact Issue 031 H3 feature-blindness trap
  the flag exists to prevent, previously unavoidable on that build).
- [x] T5.2 README section "Watch mode (real use)" documenting the three
  recorded caveats as REQUIREMENTS of any wrapper: (1) write semantics
  clean (`--fix-compile` has no unconditional-write trap — assert by test),
  (2) feature-gate blindness → `--verify-args` MUST carry the repo's real
  feature set (the Issue 031 H3 class), (3) sibling agents → session file
  scoping, never whole-repo.
  **DONE 2026-08-22** — README §"Watch mode (real use)": the three
  requirements, the store-path map (`.heal/` layout), the dump command,
  and the `CARGO_TARGET_DIR` isolation pointer.
- [x] T5.3 `CARGO_TARGET_DIR` isolation guidance for concurrent sibling
  builds (the global rule) in the wrapper's header comment.
  **DONE 2026-08-22** — the script header documents the recipe + WHY the
  script does not set it unconditionally (shared target dir = the fast
  path when idle, the contention point when not — check first, isolate
  when concurrent).
- [-] T5.4 Built-in `--watch` (notify crate): DEFER — the wrapper is the
  real-use surface; a new dep must beat it measurably first.

### Phase 6 — GOAT + record

- [x] T6.1 Mutation corpus: N≥200 labeled (broken, known-inverse-fix) pairs
  under `tests/mutations/` (generated, committed for determinism) via
  Rust-level compile-breaking mutators. **DONE 2026-08-22 (Bench 045)** —
  `rustc_errors::corpus_gen` (200 parametric cases across 15 sub-families,
  8 error codes, deterministic table-driven generation pinned by
  `generation_is_deterministic`) + the 12 committed anchors = **212**.
  Every generated seed carries a `#[cfg(test)]` ground-truth behavior
  test — the G1 semantics axis (a compiling fix that changes behavior
  FAILS the seed's own test). Stratified by territory (uncontested 105 /
  contested 107 — per-territory reporting is enforced in the harness's
  print). Shape gates fast in-unit (N≥200, unique names, strata ≥10 per
  code, braces balance, seeds ≠ broken); the cargo-verified integrity is
  the env-gated `RIIR_FULL_CORPUS_INTEGRITY=1` test + the measurement
  example — **all 212 mutants provoke their declared code; all 212 seed
  tests pass; the null arm scores exactly 0%**. Two real corpus bugs
  caught by these gates on the first run: a literal `!` in a case name
  made an invalid Cargo manifest (fixed: scaffold package-name
  sanitization), and **10 hand-counted string-length expectations were
  wrong** ("seven7" is 6 — the seeds compiled, their TESTS failed; only
  the seed-validity pass can see this class — lesson recorded: never
  hand-count literals, verify with `len()`).
  The original `tests/mutations/` layout was superseded by the
  in-crate generator (byte-stable per build = "committed for
  determinism" without a 4,000-line literal table; the plan's
  `mutate_shape` mis-citation correction stands).
- [x] T6.2 **G1 correctness:** every shipped fix compiles (by construction —
  assert the invariant anyway) + `cargo test` green post-fix on the corpus
  + the T2.3 already-compiles no-op gate.
  **DONE 2026-08-22** — 126/126 repaired cases pass their seeds'
  behavior tests post-fix (`cargo test` in every repaired scaffold — the
  G1 semantics pass; zero semantic breaks). T2.3 gates unchanged + green
  (`driver_reports_nothing_on_clean_crate`,
  `already_compiling_incident_tree_is_untouched`).
- [x] T6.3 **G2 latency:** per-iteration time (incremental `cargo check`
  dominates; report p50/p99 per iteration + per fix, release mode).
  **DONE 2026-08-22** — release, M3, otherwise-idle: per CASE p50 151ms /
  p99 164ms / max 174ms; per ITERATION p50 78ms / p99 95ms / max 96ms
  (n=126). Driver timing lives on `FixCompileReport.iter_ms` (the
  clean-break path records too — the first draft's `break` skipped the
  push, which is why per-iteration stats initially came back empty).
- [x] T6.4 **G3 no-regression:** default + all-features lib suites +
  clippy 0 + existing retrieval floors unchanged.
  **DONE 2026-08-22** — default 657 / rustc_errors+latent 787 /
  all-features 1082 lib tests green; clippy 0 on every touched surface
  (the 4 all-features warnings are the sibling riir-gpu crate's
  pre-existing state); e2e suites fix_compile 6 + borrowck 6 + e0597 4 +
  floor_corpus_integrity 3 + premise_probe 6 green; retrieval precision
  floor green.
- [x] T6.5 **G4 alloc:** the propose/apply path itself alloc-free
  steady-state (counting allocator, the perf-bench axis) — cargo-check
  subprocess cost is out of scope (it's the oracle, not the healer).
  **DONE 2026-08-22 (measured honestly: BOUNDED, not zero)** — syn-based
  proposers allocate (candidate Strings); the gate is boundedness +
  non-growth, not literal zero (a batch repair tool, not a hot tick):
  ~**1,504 allocations per proposal pass**, stable across repeated
  identical passes (10-pass growth + spread bounds —
  `propose_path_allocations_are_bounded_not_growing`, counting
  allocator). Absolute count published in Bench 045.
- [x] T6.10 **G-semantics:** the exclusion class (§Non-goals) is enforced by
  test, not prose — for each of `Box::leak`, aliasing-breaking `.clone()`
  insertion, and drop-order-changing scope restructure, either a never-emitted
  test or a diff annotation that names the semantic change. "Compiles + tests
  pass" is NOT the same as "means the same thing", and the corpus's tests are
  not complete.
  **DONE 2026-08-22** — `FixCandidate.semantic_note` (T6.10) surfaces on
  every applied/reverted/planned record + the CLI prints `⚠ semantic
  change: …` in the diff (live-verified). **8 templates annotated**:
  e0597-anon-lifetime (lifetime-contract loosening), e0597-move (ownership
  transfer), e0597-move-clone (closure owns a copy), e0597-drop-the-box
  (unboxed; drop timing), e0597-shared-ownership (Rc/Arc sharing),
  e0597-scope-hoist (drop order), e0502-clone-out (copy instead of
  borrow), e0505-clone-at-move (destination owns a copy). **2
  unannotated by design**: e0502-deref-removal (Copy reads are
  byte-identical) + e0499-swap (pure reorder) — documented in-source.
  `Box::leak`: never-emitted pin unchanged (`box_leak_is_never_emitted`).
  Gate: `every_annotated_strategy_actually_carries_its_note` + the CLI
  print (live-verified on a real E0502 repair).
- [x] T6.9 **G-floor (the gate the refuted premise forces)** — harness DONE
  2026-08-21, baseline arm MEASURED (Bench 041): 58.3% over 12 known-inverse
  mutants, bimodal — **0% on E0597/E0499/E0502, 100% on E0596/E0382/E0507/
  E0373/E0505**, and 7 of 8 contested repairs matched the known inverse
  exactly. Policy consequence: author NO templates for E0596/E0382/E0507/
  E0373 (no headroom); E0505's `&v.clone()` aliasing divergence is the only
  contested opening. The `--fix-compile` arm re-runs through the same
  `CompileFixer` seam once Phase 1 lands. **ARM MEASURED 2026-08-22 (Bench
  041 §3, `DriverFixer`)**: uncontested subset 5/5 fixed where the floor is
  0/5 (E0597 ×2, E0499 ×1, E0502 ×2 — the E0502 `Copy` case byte-matches
  the seed); **E0505 100% MATCHED where the floor's only repair 100%
  diverges** — the measured differentiator on the one contested opening;
  the four delegated codes score 0 by design (no template — the floor owns
  them). Original scope, for the record: on the mutation
  corpus, `cargo heal --fix-compile` vs `cargo fix --broken-code` head-to-head,
  reported PER ERROR CODE: fix rate, and — where both produce a fix — which
  diff a reviewer prefers (min-delta + semantics-preserving beats a blind
  `.clone()`). A contested code we do not beat ships NO template and is
  delegated to `cargo fix` in the driver. Publish the per-code table; the
  uncontested three are exempt from the comparison (floor rate = 0 by
  measurement) but must still publish absolute coverage. **Absolute
  coverage on the uncontested head: 100% (5/5) at this corpus size.
  RE-MEASURED AT N=212 (Bench 045, 2026-08-22): the bimodal floor held
  exactly — 0% uncontested (105), 100% matched delegated, 100% diverged
  E0505 (21/21); driver 105/105 uncontested + 21/21 E0505 matched.**
- [x] T6.6 **G-coverage (the honest product metric):** % of the mutation
  corpus fixed, per error family, PUBLISHED in the bench doc + README. Also
  the never-wrong number: non-compiling edits surviving a run = 0 (pinned
  by test, incl. a deliberately-bad template that MUST be reverted).
  **DONE 2026-08-22** — per-family tables in Bench 045 + README
  §"Coverage" (the head-to-head table with verdicts). Never-wrong:
  `driver_reverts_no_progress_candidate` (the deliberately-garbage
  template) + the report-side gate; non-compiling survivors = 0 by the
  keep criterion (strict coded-error decrease) + byte-exact revert.
- [x] T6.7 **G6/G7 self-evolve:** (a) trajectories persist across runs
  (store round-trip, BOTH backends); (b) move ordering measurably improves
  selection on the corpus after a learning pass (fewer
  candidates-to-first-success; shape-conditional retrieval vs flat counters
  measured separately — the T4.2 upgrade must earn its keep) — or honest
  negative recorded (T4.2's clause); (c) miss-capture produces a
  readable authoring queue from seeded unfixed errors.
  **DONE 2026-08-22** — (a) neuron backend:
  `record_then_reopen_then_retrieve` (traj_store fixtures); JSON backend:
  `json_backend_trajectory_survives_save_load_roundtrip`
  (`fix_compile_gates`). (b) **HONEST NEGATIVE recorded (the T4.2
  clause)**: learning pass (126 trajectories into a warm store) then
  static vs flat vs shape-conditional over the 126 template-covered
  cases — **all three arms 126/126 clean at 126 iterations / 0 reverts**;
  static min-delta is first-try-perfect on this corpus, so ordering has
  NO headroom here and the A/B cannot differentiate. The mechanism stays
  installed (the CLI seam + the flip test pin the property — reversed
  history flips the order, which static can never do); no benefit
  claimed. Reopen condition: real-use misses with repeated shape
  families (the regime the corpus cannot synthesize). **T4.6
  consequence: per-repo store scope stays the default; no
  `--global-traj` work.** (c) `dump_writes_entries_and_readme_once` +
  `dump_on_no_misses_touches_nothing` +
  `driver_records_miss_when_no_playbook_matches`; the 86 delegated-code
  corpus misses are the standing authoring-queue demonstration.
- [x] T6.8 Bench doc + AGENTS.md feature-matrix row (`rustc_errors`,
  default-off until G-gates pass; promotion per the modelless-gain rule —
  oracle-gated compile-fixing IS the modelless gain) + mining-queue
  snapshot pointer.
  **DONE 2026-08-22** — Bench 045 (this session's record); AGENTS.md
  `rustc_errors` row (feature matrix, with the honest-negative A/B +
  both-bugs-caught notes); README §"Coverage" + the feature-matrix row;
  mining-queue snapshot entry (2026-08-22). **Promotion stays an owner
  call**: all GOAT gates passed, but the feature remains default-off per
  the plan's own default-off-until-promotion discipline (the watch loop
  is the real-use surface that would justify it).

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

- Coverage will be partial. The pitch is **never-leaves-you-broken** + free,
  not universal and not never-wrong (see the revised thesis). The mutation
  corpus bounds the claim; real-use misses feed growth via T4.4.
- **The market is 3 codes, measured, not 7 as first claimed** — and it shrinks
  as rustc adds suggestions. The premise probe is a standing test (T0.4), not
  a one-time finding. A future rustc that suggests for E0502 would delete a
  third of the uncontested head; that is the main product risk and it is
  outside our control.
- A compiling fix can still change behavior (aliasing via `.clone()`,
  ownership via `move`, drop order via scope restructure). rustc itself ships
  this class as MachineApplicable (`&v.clone()` for E0505), so we cannot claim
  the high ground by refusing it — only by naming it in the diff (T6.10).
- The oracle is `cargo check` — the same feature-set blindness as
  `fix_verify` (a fix can compile under default features and break a gated
  config). `--verify-args` is the mitigation; the wrapper requires it.
- Editor-intermediate states (the incident's likely origin) may emit
  diagnostics the playbook can't parse — the driver treats unparseable
  diagnostics as misses (T4.4), never crashes.
- `cargo fix` parity: where rustc DOES emit machine-applicable suggestions,
  we do not compete — the playbook triggers only on codes rustc won't fix.
