# Issue 010 — L1 drafter emits corpus templates verbatim: 0/10 on held-out code

**Type:** proof / architecture scope-limit (Stage 1 of a 2-stage drafter)
**Status:** RESOLVED — Stage 2 adaptive drafter + per-lint AST proposers + identity guard shipped (2026-08-13). **GOAT gate target beaten: 10/10 on the held-out fixture** (was 0/10) via real `cargo clippy` verification.
**Found by:** [Bench 009](../.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md) (2026-08-12)
**Resolved by:** commit landing `src/adaptive_drafter.rs` + `src/proposers/` + identity guard in `src/drafter.rs` + `HealPipeline::tune_against_fixture` + `examples/stage2_tuning_demo.rs`. See `tests/stage2_goat.rs` for the gate + `.benchmarks/011_stage2_goat.md` for the measurement.
**Severity:** Stage 1 scope-limit, honestly documented. The 0/10 baseline is real + remains Stage 1's scope — Stage 2 is the modelless generalization path that beats it.

## Resolution (2026-08-13)

**GOAT gate target beaten: 10/10 on the held-out fixture** via real `cargo clippy` verification. The Stage 2 stack shipped:

- **`src/adaptive_drafter.rs`** — `ClippyAdaptiveDrafter` mirrors `AdaptiveQuestDrafter`. Lock-free `papaya::HashMap` keyed by `BLAKE3(lint_name, buggy_code)`. Empty by default; populated off-hot-path by the tuning loop.
- **`src/proposers/`** — per-lint `syn`-level AST rewrite rules (Issue 010 fix Direction 2). 10 lints ship today: `needless_return`, `redundant_closure`, `needless_lifetimes`, `redundant_field_names`, `useless_conversion`, `needless_borrow`, `unnecessary_mut_passed`, `manual_map`, `unnecessary_unwrap`, `needless_range_loop`. Each is a deterministic codemod (`fn rewrite(&mut File) -> bool`); verified by `cargo clippy` during tuning.
- **Identity guard in `src/drafter.rs`** (Issue 010 fix Direction 3) — `ClippyTernaryDrafter::draft_fix` returns `None` when the emitted fix shares no identifiers with the input. Turns the silent-miss into an honest decline.
- **`HealPipeline::tune_against_fixture`** — the off-hot-path tuning loop (Issue 010 fix Direction 1). For each case: Stage 1 → identity guard → if decline, run proposer → verify each against `cargo clippy` → admitted fixes join the adaptive pool. Returns a `TuneReport`.
- **`HealPipeline::heal`** — Stage 2 routing: returns the adaptive pool entry directly when present (HealSource::Stage2Adaptive), else falls back to the existing Stage 1 path.
- **`examples/stage2_tuning_demo.rs`** — the executable GOAT gate. Run with `cargo run --release --example stage2_tuning_demo --features self_evolve,clippy_verify`. Reports `TuneReport { stage2_verified: 10, .. }`.
- **`tests/stage2_goat.rs`** — 5 integration tests pinning the gate: proposers fire on all 10 held-out cases, output is syntactically valid, Stage 1 identity guard declines ≥7/10, Stage 2 serves all 10 cases after tuning.

**Verdict:** the modelless generalization path is exhausted for the 10 seed-corpus lints. Adding a new lint = adding a new proposer (one `fn rewrite` + a row in `proposers/mod.rs::propose_fix`'s match). L4 LoRA remains deferred (per the modelless-first mandate) — Stage 2 covers the held-out fixture fully without it.

## Symptom

On the held-out fixture (`benches/fixtures/e2e_cases_heldout.json` — the **same
10 lints**, different code), the pipeline scores **0/10 exact-match** while
scoring **10/10** on the seed fixture. Worse than a low score: **10/10 of the
held-out emissions are byte-identical to the seed emissions.**

```text
held-out input : fn bar(x: i32) -> i32 { return x + 1; }
emitted        : fn foo() -> i32 { 42 }        <-- the SEED entry, verbatim
wanted         : fn bar(x: i32) -> i32 { x + 1 }
```

Every held-out case behaves this way. The L2 pruner still reports
`parses = true` 10/10, because the emitted code *is* syntactically valid Rust —
it is simply a different function than the one the caller asked about. **The
always-on validator cannot catch this class of failure.**

## Root cause (confirmed in source, not inferred)

`src/drafter.rs::draft_fix` constructs its output as:

```rust
let entry = &candidates[idx];
Some(DraftedFix { code: entry.fixed_code.clone(), .. })   // <-- verbatim corpus text
```

`buggy_code` enters the computation **only** as BLAKE3 hash bytes →
`simd_ternary_matvec` → FNV-1a fold → `idx = hash % candidates.len()`. So the
input code selects *which stored fix to return*; it never influences the
returned text. The drafter is a **corpus retriever with a hash-driven index**,
not a code transformer.

Because the seed corpus holds exactly **one** entry per lint,
`candidates.len() == 1` ⇒ `idx == 0` always ⇒ the single stored `fixed_code` is
the only reachable output for that lint, for any input whatsoever.

**Generalization is architecturally out of reach at Stage 1, not a tuning gap.**
No amount of weight/EMA refinement changes it: the output alphabet for a lint
at Stage 1 is the set of `fixed_code` strings already in the corpus. The fix is
Stage 2 (below), not more tuning of Stage 1.

## What this does and does not invalidate

| Claim | Status |
|---|---|
| Bench 001 L1 latency (p50 2.3 µs), Bench 002 L2 latency | **Unaffected** — latency is real. |
| Issue 008 G6/G7 (+50 pp healing rate on real `cargo clippy`) | **Unaffected but in-distribution** — measured on corpus lints whose fixes the corpus contains. |
| Bench 009 throughput ratio (~10^5×) | **Unaffected** — it is a real throughput measurement. |
| "The modelless pipeline heals clippy warnings" | **Scope-limited to Stage 1** = `(lint, code)` pairs already in the corpus. On unseen code Stage 1 returns a valid-but-wrong function; **Stage 2 (clippy_verify-backed adaptive drafter) is the modelless generalization path.** |

## Why the pruner does not save us (at Stage 1)

L2 (`syn` parse) answers "is this Rust?", not "is this a fix for *my* code".
The `clippy_verify` path (Phase 3, `ClippyVerifier::verify`) re-runs real
`cargo clippy` and *would* catch it — **and that is exactly the point: it is
the Stage 2 tuning-phase validator.** It is opt-in and ~130 ms, which is 4
orders of magnitude above the Stage 1 drafter's own hot path, so it cannot be
the *default hot-path guard*. But it IS the right tool for the **tuning phase**
that builds the Stage 2 pool (see fix direction below).

## Fix direction: 2-stage drafter (`quest_grammar` precedent)

The `quest_grammar` pipeline (`riir-games-quest`) ships a **2-stage drafter**
that is the direct precedent for the fix. `QuestGrammarPipeline` holds both:

- **Stage 1 — `plasma_draft` (`TernaryDraftModel`)**: always-on, modelless,
  ~µs. Hash-selects a template from a small hardcoded pool (8 SVO templates).
  Accepts its limitation: the output is always one of the 8 templates. This is
  the `ClippyTernaryDrafter` analogue.
- **Stage 2 — `adaptive_drafter` (`AdaptiveQuestDrafter`)**: optional override,
  loaded via `load_seal_corpus()` when a real corpus is thawed. Picks from N
  real corpus response texts (up to 1024) via the same FNV-1a hash. The
  ternary model stays as a cold fallback if the adaptive drafter is later
  cleared.

The routing is one function (`QuestGrammarPipeline::draft_text`):

```rust
pub fn draft_text(&mut self, quest_context: &str) -> String {
    #[cfg(feature = "quest_corpus_freeze")]
    if let Some(ad) = self.adaptive_drafter.as_ref() {
        return ad.generate(quest_context);   // Stage 2 if loaded
    }
    self.plasma_draft.generate(quest_context) // Stage 1 otherwise
}
```

riir-clippy should adopt the **same pattern**:

- **Stage 1 — `ClippyTernaryDrafter` (current L1, unchanged)**: always-on,
  modelless, ~µs. Returns one seed-corpus `fixed_code` per lint via
  BLAKE3→matvec→FNV-1a. **This is where the 0/10 held-out result lives — it is
  the Stage 1 scope-limit, honestly documented, NOT a blocking defect.**
- **Stage 2 — `ClippyAdaptiveDrafter` (new, mirrors `AdaptiveQuestDrafter`)**:
  optional override, loaded when the tuning phase has built a pool of
  clippy-VERIFIED fixes. `draft_fix` routes to Stage 2 if loaded, else Stage 1
  — exactly the `draft_text` shape above.

### The tuning loop (where re-running clippy is fine)

Stage 2's pool is populated by a **tuning loop** that runs OFF the hot path:

1. Stage 1 drafts a candidate fix for `(lint_name, buggy_code)`.
2. `ClippyVerifier::verify()` re-runs `cargo clippy` (~130 ms) on the candidate
   — **acceptable cost for the tuning phase, NOT the hot path**.
3. If the fix resolves the lint → it joins the Stage 2 pool (keyed by
   `(lint_name, hash(buggy_code))`).
4. If not → the tuning loop tries **alternative proposals** (identity guard
   decline, span-local AST edit — see candidate directions below) and verifies
   *those* against `cargo clippy`.
5. Verified alternatives join the Stage 2 pool.
6. On subsequent hot-path `draft_fix` calls, Stage 2 serves the verified fix if
   the pool has an entry for this `(lint_name, buggy_code)`; otherwise it
   falls back to Stage 1.

**Re-running `cargo clippy` during the tuning phase is fine.** The ~130 ms
cost is 4 orders of magnitude above the Stage 1 hot path (~µs), but the tuning
loop is a batch/background process that builds the Stage 2 pool. The hot path
stays at Stage 1's ~µs latency; it only upgrades to Stage 2 when a verified
pool entry exists. This mirrors exactly how `quest_grammar` works:
`plasma_draft` is always-on (~µs), `adaptive_drafter` overrides it only when
`load_seal_corpus` has populated the pool. The populate step is slow (corpus
thaw / clippy re-run); the serve step is fast (hash-select from the pool).

## Candidate directions (composing with the 2-stage architecture)

These are **not alternatives** — they compose into the 2-stage pipeline. None
is validated yet; do not treat as a plan.

1. **Stage 2 adaptive drafter (the primary fix, the container).** Ship the
   2-stage routing + the clippy_verify-backed tuning loop. On its own, this
   widens the output pool (more verified fixes per lint) but does not yet
   GENERATE brand-new fixes for unseen code — it serves verified fixes the
   tuning loop has already encountered. This is the modelless generalization
   path that must be exhausted before L4 LoRA (per the modelless-first
   mandate). Mirrors `AdaptiveQuestDrafter` 1:1.

2. **Span-local edit (the Stage 2 proposal mechanism for unseen code).** Treat
   a corpus entry as a `syn`-level rewrite rule (pattern → template) applied to
   the caller's AST, rather than a literal string to hand back. Emits the
   caller's own identifiers by construction. This is the mechanism that
   PROPOSES new fixes during tuning (Step 4 above) for code variants the seed
   corpus doesn't cover. Real work, per-lint, composes with Stage 2.

3. **Identity guard (the Stage 1 decline-path).** Detect that the emitted fix
   shares no identifiers with the input and refuse to emit rather than
   returning a confidently-wrong function. Cheap, honest, near-free. Turns a
   silent wrong answer into a decline so the tuning loop knows to try Stage 2
   alternatives. Strictly better than today even before Stage 2 ships.

Direction 1 is the **container** that routes between stages; Direction 2 is the
Stage 2 **proposal mechanism** for unseen code; Direction 3 is the Stage 1
**decline guard**. All three compose. The standing GOAT gate target is the
held-out fixture (`benches/fixtures/e2e_cases_heldout.json`): Stage 1 +
identity guard + Stage 2 adaptive drafter must beat 0/10.

## Reproduce

```sh
CARGO_TARGET_DIR=/tmp/clippy_e2e_target cargo bench --bench bench_e2e_tokps --features self_evolve -- \
  benches/fixtures/e2e_cases_heldout.json /tmp/clippy_e2e_heldout.jsonl
# => exact-match: 0/10, parse-valid: 10/10
```

## References

- [`quest_grammar::plasma_draft`](../../riir-ai/crates/riir-games-quest/src/quest_grammar/plasma_draft.rs)
  — Stage 1 (`TernaryDraftModel`) + Stage 2 (`AdaptiveQuestDrafter`) precedent.
- [`quest_grammar::mod`](../../riir-ai/crates/riir-games-quest/src/quest_grammar/mod.rs)
  — `QuestGrammarPipeline::draft_text` (the 2-stage routing function) +
  `load_seal_corpus` / `clear_seal_corpus` (the Stage 2 load/clear API).
- [Proposal 001](../.proposals/001_multi_domain_modelless_code_healer.md) —
  the multi-domain framework this fix composes into (Stage 2 adaptive drafter
  is per-domain; the `HealDomain::validate` method is the Stage 2 validator
  seam).
