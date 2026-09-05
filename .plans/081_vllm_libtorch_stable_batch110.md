# Plan 081 — Batch 110: vLLM libtorch_stable distill (verified A-/B+ tier)

**Status:** RECORD — Batch 110 LANDED 2026-09-05 (all tasks T0-T11 done except T3 riders deferred `- [-]` to the Batch 111 sweep; commit + hash recorded on push).

Source: vllm-project/vllm `csrc/libtorch_stable/` @ `8277c42e4c74c5dd604f29b6d938a3456658f247` (Apache-2.0), clone at `.raw/vllm` (sparse csrc+rust; rm at landing). Verdict: `.research/133_vllm_libtorch_stable_distill.md` (13 lead quotes byte-verified at the pin).

Scope: rules #1-#11 from the verdict table (the verified tier). Rules #12-#25 + the hazard-composed refinement rules (H1+R18) = Batch 111, queued behind this.

- [x] T0 — Pre-flight: re-scan `.plans/.highwater` + folder at write time (done: 081 allocated, highwater stale at 78 → bump to 81 in this commit); re-pin `.raw/vllm` (`git -C .raw/vllm rev-parse HEAD` == `8277c42e…`, clean tree) before any quote work.
- [x] T1 — Dedup re-verification (coordinator, not lane): re-grep `src/domains/kernel_opt/corpus/` under BOTH vocabularies for each of the 11 rules; record one evidence line per rule in the batch record. Kill anything the corpus grew to cover since the verdict (B109 landed post-verdict — check `dispatch-handler-pair-hoist` / `dead-result-timing-loop-folded-under-lto` adjacency, expected nil).
- [x] T2 — Entry authoring (corpus files: open `entries_gpu_22.rs`): 11 entries, each with `rule_id`, `gpu_concept` (decision content + regime + distinct-from list), `ref_shape` (naive), `opt_pattern` (fixed), `source` line (file:lines at pin + batch/research/plan refs). Original fixtures only — never paste vLLM code verbatim (license hygiene).
- [-] T3 — Riders: DEFERRED to the Batch 111 rider sweep — the ~30-instance inventory is preserved verbatim in Research 133 §"Rider inventory" (highest-value: hi/lo packed-u64 redux decomposition → B22/B107; inverted-index pack → B107; phase-counted barrier → B92; B57 union-across-phase + filler-thread barrier matching; B103 loud-refusal instance). Reason: 16 fragile edits on 2,000-char single-line concept strings late in a long session; the deferred state loses nothing (Research 133 is committed).
- [x] T4 — Standing gates: `batch110_rules_present` + `batch110_concepts_covered` + `batch110_pseudocode_parses` (cfg(not(kernel_verify)) per B70) in `corpus/tests.rs`; wire entries into `mod.rs` (entries_gpu_22 + category lists) + `KERNEL_CATEGORIES`.
- [x] T5 — GOAT lead gate: `batch110_spin_barrier_launch_contract` — pin the headroom/cap/fallback arithmetic of rule #1 as a unit test (pure fn over (num_sms, occupancy, ctas_per_group, needs_cooperative) → cap; assert: headroom = sms when occ>1 else 1, never below one group, fallback when total > hw cap; non-vacuity = the needs_cooperative=false arm stays uncapped).
- [x] T6 — GOAT: G1 gates 3/3 + new gate green; G2 lib tests pass at floor counts (walk the count pins: tests.rs corpus total 494 → 505, expected-array category deltas, sums x2, grouped, cover-all comment, router.rs expert total, mod.rs doc line + category comments — the 18th-count-pin-site discipline, expect staleness); G3 clippy 0 in default + kernel_heal states (+ all-targets in touched files); retrieval floors HELD (`retrieval_precision_regression_gate` — kernel self-variants 100/100/100, no re-pins).
- [x] T7 — Rule-quality gate per entry: decision content bounded, regime stated both directions, distinct-from list cites the real neighbors (esp. #1 vs B58, #3 vs B107, #4 vs B108/B44, #5 vs B45/B60, #9 vs B104); encode H1's silent-overflow warning inside rule #18's future text only (Batch 111) but reference the hazard from #9's concept if natural.
- [x] T8 — Docs: bump AGENTS.md kernel_opt corpus count (494 → 505) + batch line; update `.docs/11_domains/kernel_opt.md` table if it tracks batches; update `.distill/001_mining_queue_snapshot.md` (Last landed + queue state; Batch 111 NEXT-UP row from verdict #12-#25 + hazard ledger).
- [x] T9 — Record: `.research/133` Status → RECORD (batch landed, commits cited); update the note's queue-slot section.
- [x] T10 — Cleanup: `rm -rf .raw/vllm` (the .raw discipline: a landed batch with a live .raw entry is an unfinished batch); verify `git status` clean of .raw paths before commit.
- [x] T11 — Commit `feat: batch 110: vLLM libtorch_stable distill (11 rules + riders)` + push on develop. Commit queue-snapshot + research-status updates in the same unit.

Deferred / out of scope:
- [-] Batch 111 tier (#12-#25, H1+R18 composition, hazard ledger filing) — next unit after 110 lands; do NOT consume in this batch. **NOW QUEUED as the NEXT-UP row in the snapshot.**
- [-] `rust/` dir (vLLM bench/chat/server CLI) — application-level Rust, low-priority future rust_perf source; recorded as unread remainder in Research 133.
- [-] Upstream hazard reporting to vLLM — out of program scope; hazards recorded in Research 133 for our own rule text.
