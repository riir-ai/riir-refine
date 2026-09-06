# Issue 074: Surface the trust annotations in `--fix` output — the `⚠ semantic change` / `context_notes` channels stop at the rustc_errors domain

**Status:** Open — T1–T3 actionable (display-only, modelless); T4 defer with the DRY constraint
**Source:** external review (Gemini, owner-relayed 2026-09-06) — two of its three points verified against the tree and accepted with mechanism corrections; point 3 (corpus densification) routed to its existing trackers, not this issue
**Filed:** 2026-09-06

## Problem

The healer's trust story is already built at the data layer and dropped at the render layer:

1. **The `⚠ semantic change:` channel exists but covers ONE domain.** The `rustc_errors` driver threads `FixCandidate.semantic_note: Option<&'static str>` through every playbook, and `heal.rs` renders it via `semantic_note_suffix` (`\n  ⚠ semantic change: {n}`) — **only** on the `--fix-compile` applied/plan records. The general `--fix` path (edit emitter over `DraftedFix`) prints no semantic annotation at all.

2. **`context_notes` propagates and is not printed.** `HealEntry::context_notes()` (Plan 001 post-promotion, 2026-08-13) reaches `DraftedFix::context_notes` — carried by **44 rust_perf rules** (ParetoDominated/Mixed/Tie verdicts, float-order caveats), docker (`judgment-call` rules), and sec — but the general `--fix` output never surfaces it. The `heal_demo` example prints `⚠️ Context-dependent:`; the production CLI does not. A consumer running `cargo heal --fix --write` on a rule whose annotation says "regime: usually-sorted data only" (e.g. `sortedness-check-branchless-accumulate`) sees the diff and the verdict line, not the caveat — the exact consumer the annotation channel was built for.

3. **The semantics-adjacent clippy_lints proposers carry no annotation.** The clippy class is mostly semantics-preserving by construction (format-arg inlining is output-identical; `match_bool` restructures branch syntax only), but a real minority shifts observable behavior in bounded, known ways: `map_unwrap_or`/`unwrap_or_else` (eager-vs-lazy evaluation order — the purity gates prove the team tracks this), `manual_map` (closure shape / control-flow capture — the `?`/`return`/`await` refusal exists *because* the rewrite would change semantics), `needless_range_loop` (borrow shape: `iter()` vs `iter_mut()` vs by-value), `question_mark` (short-circuit equivalence relies on the arm-shape gates), `bool-to-bitpack` / `static-str` (signature-changing — caller-gated). None of this is *named at the diff*; it lives in fixture comments and the divergence table in the skill doc.

The correction to the external framing, recorded so it is not re-derived: there is **no AST-diff pass to extend** — the rustc_errors notes are hand-authored per strategy because the proposer *knows* the shift (drop-order, clone insertion, ownership transfer); a generic post-hoc differ would be noisy and could not name intent. The cheap honest implementation is: per-proposer/per-rule notes (authoring) + one render line (wiring).

## Tasks

- [ ] **T1 — render the annotation in general `--fix` output**: in `heal.rs`'s edit-emitter reporting (dry-run review sections + `--write` applied lines), print the annotation when present — `context_notes` from `DraftedFix` (truncated to its first line; full text stays in `--explain`-style verbose mode if that exists, else the first line only) + the same `⚠ semantic change:` vocabulary as the rustc_errors renderer so the CLI has ONE annotation dialect. Zero behavior change when notes are absent (the common case — the renderer's default must be byte-identical, pinned by a test).
- [ ] **T2 — author the notes**: add `context_notes`/semantic notes to the semantics-adjacent clippy_lints proposers listed above (the corpus entries and/or the proposer-level constant — whichever surface the `DraftedFix` path actually reads; verify with a grep that the chosen surface flows to the emitter). Keep each note one line, named-class form ("⚠ semantic change: evaluation order — default arm now evaluated lazily"), never prose paragraphs. Scope guard: do NOT annotate unconditional-win mechanical classes (format-args, identity_op, semicolon) — an annotation that fires on 90% of fixes is warning fatigue and *erodes* the trust axis this issue serves.
- [ ] **T3 — gates**: (a) fixture-purity — the annotated fixtures still fire their own proposer and only their own; (b) a render test pinning byte-identical output for un-annotated classes and the `⚠` line for annotated ones; (c) run `--score-bench` after landing (standing workflow) and confirm heal/created/safety rates unmoved (display-only change; `created` is the trust axis — any move is a finding).
- [ ] **T4 (defer with constraint) — `--suggest` instantiated display**: the suggestion text should be the **proposer's own output** (the instantiation re-ranker already runs `propose_fix` visitors over the span under `RerankMode::Structural`), NOT a hand-bound presentation-layer template — a parallel binder can emit a plausible-but-noncompiling instantiation, which is the exact "suggestion that lies" failure the verbatim-corpus-text design prevents. Reopen when T1–T3 land and there is a consumer ask for instantiated suggest text.

## Non-goals

- **Corpus densification / evidence volume** (the review's point 3 — agreed it is the real selection bottleneck per T39/Research 135): already tracked by Issue 072 T5/T6 + the frontier-pairs path + the `--misses` authoring queue. Not duplicated here.
- **A generic semantic differ.** Rejected at filing: the proposer is the source of truth for intent; a differ adds noise and cannot name drop-order/overflow/laziness.

## References

- `src/bin/heal.rs` `semantic_note_suffix` + the rustc_errors `AppliedRecord`/`PlannedRecord.semantic_note` threading (the dialect to reuse)
- `src/domain.rs` `HealEntry::context_notes` → `DraftedFix::context_notes` (the propagation that already exists)
- `src/domains/rust_perf/corpus.rs` (44 annotated rules — the count pin test lives there)
- Plan 005 T3.5.4/T3.5.5 (instantiation re-ranker — the T4 machinery)
- Score-bench run #3 (heal 98.1% · created 1.9% — the baseline T3 must hold)
