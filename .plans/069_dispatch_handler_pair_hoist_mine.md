# Plan 069 — Batch 101: land `dispatch-handler-pair-hoist` (rust_perf/dispatch)

**Status:** IN PROGRESS — the unnumbered probe candidate (Research 119 §Parallel-session addendum) promoted to a batch. Evidence already measured: aarch64 ParetoDominant 2/0/0 (1.19-1.23x at 10K/1M, LCG-random stream; Tie band on structured/periodic), verdict-stable x3, 0/0 allocs, alloc-det stable x3, asm-pinned mechanism (in-loop `ldr` eliminated by the hoist). Cross-arch x86_64 re-gate ARMED (4090 box had active sibling builds at batch time - defer with unblock condition below).

Source evidence: `riir-clippy/.research/119` §Parallel-session addendum (the M3 lane's probe, yielded then queued); queue snapshot unnumbered-candidate row.

## Tasks

- [x] Worktree isolation: `/Users/katopz/git/riir-clippy-wt-b101` at origin/develop (the shared checkout carries an active sibling's WIP - `src/lib.rs`, probe files; untouched).
- [x] Plan 069 created BEFORE any corpus edit (this file).
- [ ] Dedup re-verification vs the landed B99 rule (`mir-destprop-branch-site-merge`) - distinct mechanism (runtime TABLE + value-returning + HOIST vs known-items + tail-call + SPLIT), distinct regimes (B99's runtime-opaque negative control marks the SPLIT transform out-of-scope on unpredictable streams; it does NOT cover the hoist - splitting re-merges structurally on aarch64, the hoist still wins by eliminating the load). Confirm zero overlap in the corpus (grep pair-hoist|handler-pair|table-hoist).
- [ ] Author the corpus entry `dispatch-handler-pair-hoist` (category dispatch, suggestion-only): buggy = in-loop indexed handler load from a runtime table per dispatch; fixed = hoist both entries once before the loop, register-resident select in-loop. context_notes carries: aarch64 measured verdicts (raw probe + corpus validator), the unpredictable-stream regime boundary (structured/periodic = Tie), the aarch64 asm mechanism (in-loop ldr eliminated), the x86-pending caveat (x86_64 merged form uses lea rip+h_* - no in-loop load; verdict armed), dedup vs B99, provenance (Research 119 addendum / Batch 101).
- [ ] Bench spec (rust_perf_bench): value-returning fn(u64) -> u64 handlers through a runtime [Handler; 2] table, LCG-random 2-op stream (the unpredictable regime), sizes 1K/100K/1M - mirrors the addendum's probe harness + the B99 spec structure.
- [ ] Standing gates: batch101_rules_present + batch101_pseudocode_parses + batch101_concepts_covered + the G1 output-identity test (buggy and fixed produce identical accumulator over a fixed op stream; non-vacuity: both handlers distinguishable in the output).
- [ ] Count pins walked (ALL sites): corpus.rs doc header L12 (STALE at 97/dispatch-3 from B99 - fix to 99/5), fn doc L246 (98->99, dispatch 4->5), assert_eq!(entries.len(), 98) L7743, benchable 85->86 (fn name eighty_five_rules_have_bench_specs -> eighty_six), annotated 41->42 (fn name forty_one -> forty_two, x4 assertion sites incl. the 98-41 message + the annotated-contains list), AGENTS.md rust_perf count, .docs/11_domains/rust_perf.md, .plans/.highwater 68->69, queue snapshot candidate row -> LANDED.
- [ ] GOAT (M3, worktree, CARGO_TARGET_DIR=/tmp/riir-clippy-b101-target): default lib count-identical + --features rust_perf_bench (the corpus validator = the recorded instrument, aarch64 re-validation) + --features latent_retrieval retrieval floors 13/13 HELD + clippy 0 both states.
- [ ] Docs: Research 119 addendum disposition note (landed as Batch 101), AGENTS.md Batch 101 line, queue snapshot update, .docs/11_domains/rust_perf.md row.
- [ ] Commit + push from the worktree.
- [ ] git worktree remove + rm -rf /tmp/riir-clippy-b101-target at landing.
- [-] **x86_64 cross-arch re-gate - DEFERRED, armed**: the 4090 box carried 4 cargo + 3 rustc active sibling builds at batch time (contention = both boxes slowed + target-dir lock wedging their session). Unblock: a 4090 window (probe tasklist | findstr cargo); then pull, build --features rust_perf_bench, run the new rule's bench (PERF_BENCH_FILTER=dispatch-handler-pair-hoist), annotate the entry with the measured x86 verdict (expected Tie if the lea-rip analysis holds - annotation updates, never deletion). Until then the entry carries the x86-pending caveat in context_notes; the rule is suggestion-only (no --fix path can fire it).

## Non-goals

- No proposer (suggestion-only - codegen shape invisible to a parse-only proposer; the B59/B77 precedent).
- No change to the landed B99 rule (dedup stays a cross-ref both directions at the NEXT B99 doc touch; this batch only adds the cross-ref on the new entry).
- The hot-dispatch-gpr-arg-budget queued-secondary stays kill-free (cross-arch corroborated dead - addendum item 3).
