# Plan 069 — Batch 101: land `dispatch-handler-pair-hoist` (rust_perf/dispatch)

**Status:** COMPLETE 2026-09-02 — landed as riir-clippy `87d8d13` (rebased over the tpr agent's concurrent f4f5df3+fd45b46, pushed clean). The unnumbered probe candidate (Research 119 §Parallel-session addendum) promoted to a batch. Evidence already measured: aarch64 ParetoDominant 2/0/0 (1.19-1.23x at 10K/1M, LCG-random stream; Tie band on structured/periodic), verdict-stable x3, 0/0 allocs, alloc-det stable x3, asm-pinned mechanism (in-loop `ldr` eliminated by the hoist). Cross-arch x86_64 re-gate ARMED (4090 box had active sibling builds at batch time - defer with unblock condition below).

Source evidence: `riir-clippy/.research/119` §Parallel-session addendum (the M3 lane's probe, yielded then queued); queue snapshot unnumbered-candidate row.

## Tasks

- [x] Worktree isolation: `/Users/katopz/git/riir-clippy-wt-b101` at origin/develop (the shared checkout carries an active sibling's WIP - `src/lib.rs`, probe files; untouched).
- [x] Plan 069 created BEFORE any corpus edit (this file).
- [x] Dedup re-verification vs the landed B99 rule (`mir-destprop-branch-site-merge`) - distinct mechanism (runtime TABLE + value-returning + HOIST vs known-items + tail-call + SPLIT), distinct regimes (B99's runtime-opaque negative control marks the SPLIT transform out-of-scope on unpredictable streams; it does NOT cover the hoist - splitting re-merges structurally on aarch64, the hoist still wins by eliminating the load). Confirm zero overlap in the corpus (grep pair-hoist|handler-pair|table-hoist).
- [x] Author the corpus entry `dispatch-handler-pair-hoist` (category dispatch, suggestion-only): buggy = in-loop indexed handler load from a runtime table per dispatch; fixed = hoist both entries once before the loop, register-resident select in-loop. context_notes carries: aarch64 measured verdicts (raw probe + corpus validator), the unpredictable-stream regime boundary (structured/periodic = Tie), the aarch64 asm mechanism (in-loop ldr eliminated), the x86-pending caveat (x86_64 merged form uses lea rip+h_* - no in-loop load; verdict armed), dedup vs B99, provenance (Research 119 addendum / Batch 101).
- [x] Bench spec (rust_perf_bench): value-returning fn(u64) -> u64 handlers through a runtime [Handler; 2] table, LCG-random 2-op stream (the unpredictable regime), sizes 1K/100K/1M - mirrors the addendum's probe harness + the B99 spec structure.
- [x] Standing gates: batch101_rules_present + batch101_pseudocode_parses + batch101_concepts_covered + the G1 output-identity test (buggy and fixed produce identical accumulator over a fixed op stream; non-vacuity: both handlers distinguishable in the output).
- [x] Count pins walked (ALL sites): corpus.rs doc header L12 (STALE at 97/dispatch-3 from B99 - fix to 99/5), fn doc L246 (98->99, dispatch 4->5), assert_eq!(entries.len(), 98) L7743, benchable 85->86 (fn name eighty_five_rules_have_bench_specs -> eighty_six), annotated 41->42 (fn name forty_one -> forty_two, x4 assertion sites incl. the 98-41 message + the annotated-contains list), AGENTS.md rust_perf count, .docs/11_domains/rust_perf.md, .plans/.highwater 68->69, queue snapshot candidate row -> LANDED.
- [x] GOAT (M3, worktree, CARGO_TARGET_DIR=/tmp/riir-clippy-b101-target): default lib count-identical + --features rust_perf_bench (the corpus validator = the recorded instrument, aarch64 re-validation) + --features latent_retrieval retrieval floors 13/13 HELD + clippy 0 both states.
- [x] Docs: Research 119 addendum disposition note (landed as Batch 101), AGENTS.md Batch 101 line, queue snapshot update, .docs/11_domains/rust_perf.md row.
- [x] Commit + push from the worktree (`87d8d13`).
- [x] git worktree remove + rm -rf /tmp/riir-clippy-b101-target at landing.
      (VERIFIED 2026-09-03: neither exists — no b101 worktree in `git worktree list`,
      /tmp/riir-clippy-b101-target gone; the remaining `.riir-clippy-main` worktree is
      the intentional main-branch ops-deploy checkout, not b101.)
- [x] **x86_64 cross-arch re-gate - DONE 2026-09-04 (the armed defer FIRED, verdict REFUTES the expected Tie):** quiet 4090 window (zero cargo/rustc; the 504 NoRA matrix holds only the GPU), `rust_perf_bench --release`, `PERF_BENCH_FILTER=dispatch-handler-pair-hoist`, verdict-stable x2: **ParetoDominant 3/0/0 -- 2.69-2.73x @ 1K / 1.09-1.10x @ 100K / 1.07-1.08x @ 1M, alloc 0/0, alloc-det stable.** The expected Tie rested on the `lea rip+h_*` no-in-loop-load analysis -- which belongs to Batch 99's KNOWN-ITEMS fixture (compile-time code pointers); this rule's fixture is a RUNTIME TABLE, so x86 must also load the handler pointer in-loop and the hoist eliminates it on both arches. Caveat removed, verdict recorded (annotation-updates-never-deletes); the `batch101_concepts_covered` pin flipped from `PENDING` to `REFUTED` + the AGENTS.md Batch 101 caveat retired. Honest residue: the structured/periodic stream cells were NOT re-measured on x86 (the Tie regime is predictor-mechanical and expected arch-independent; noted in the annotation).

## Non-goals

- No proposer (suggestion-only - codegen shape invisible to a parse-only proposer; the B59/B77 precedent).
- No change to the landed B99 rule (dedup stays a cross-ref both directions at the NEXT B99 doc touch; this batch only adds the cross-ref on the new entry).
- The hot-dispatch-gpr-arg-budget queued-secondary stays kill-free (cross-arch corroborated dead - addendum item 3).
