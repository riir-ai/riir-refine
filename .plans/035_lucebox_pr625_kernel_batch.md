# Plan 035 — lucebox PR #625 external kernel batch (Batch 77 candidate)

**Status:** COMPLETE 2026-08-27 — Batch 77 landed (6 kernel_opt rules + 1 B63 rider + 1 rust_perf crossover; corpus 373→379, rust_perf 85→86). All GOAT gates PASS; full CI guard ALL LAYERS. Created BEFORE any corpus edit (plan-first, the B60 discipline).

Source: [lucebox PR #625](https://github.com/Luce-Org/lucebox/pull/625) (merged 2026-08-24; all 69 patches read at verdict time). Verdict note: [Research 091](../.research/091_lucebox_pr625_kernel_distill.md). Strategic half: `../riir-ai/.research/350_lucebox_pr625_qwen38_dflash2_r9700.md` (Issues 754/755 updated in the same window).

Scope per the verdict: 4 keep-grade kernel_opt candidates + 1 rust_perf crossover + 1 B63 rider; expected yield after coordinator falsification 2–3 rules. All fixtures are ORIGINAL minimal reproductions (Apache-2.0 source, nothing verbatim).

## Phase 1 — pre-flight (dedup re-verification at batch time)

- [x] T1.1 Re-run the dedup greps from Research 091 against the CURRENT corpus (both vocabularies per pattern; the corpus moves between verdict and batch — the standing rule). Any new hit gets READ before a candidate is called duplicate.
- [x] T1.2 Re-read the four load-bearing source files at mine time (siblings drift; snapshot quotes then): `vecdotq.cuh` (nt loads), `mmq.cu/.cuh` + `mmq_big.h` (tile twins), `gated_delta_net.cu` (TREE_MODE parent reload, raw gates), `rope.cu` (binary exponentiation), `mmvq.cu` (q8 memo). DONE via subagent at head f28d85c — 6/7 verified; 3 corrections folded into the entries (mech-3's 61-196→37 µs + 8.4e-7 figures NOT in source — dropped from the entry; TREE_MODE pre-existed in the base kernel, the PR extends it to grouped-cols — provenance corrected; the q8 memo pre-existed opt-in, the PR's deltas are default-ON + stream guard + MAX_BUFFERS 256→1024 — provenance corrected).
- [x] T1.3 Confirm the rust_perf crossover isn't covered by a `powi-vs-powf` annotation change instead of a new rule (coordinator call: extend the existing rule's context_notes vs a separate `runtime-int-exponent-powi` entry — the bench harness shape decides).

## Phase 2 — corpus entries (kernel_opt, entries_gpu_07.rs unless >~950 lines → open entries_gpu_08.rs)

- [x] T2.1 `weight-stream-non-temporal-loads` (A) — streaming-vs-reused operand regime; cite BOTH measured proofs (source IQ4_XS vecdot + riir-ai T9.16 `__ldcs` / Bench 734); bit-identical by construction (load hint).
- [x] T2.2 `batch-width-routed-dual-tile-twins` (A−) — the N≤16 occupancy-starved regime vs prefill-width re-streaming; the namespace-twin instantiation mechanism (tile = baked macro, one shape per TU); the pathological mid tile (443→180 GB/s at mmq_x=32) as in-rule negative evidence; crossover table (small→64, tie@128, big@256+).
- [x] T2.3 `tree-verify-recurrent-state-parent-reload` (B+) — DFS branch transitions reload the register state shard from stored per-token intermediates (same-thread RAW, no barrier); 61–196→~37 µs/layer @8.4e-7; distinct-from B71 rotated-ring-undo (KV twin).
- [x] T2.4 `shared-activation-quantize-memo-per-eval` (B) — the third same-input-family mechanism (quantized-activation cache vs launch-merge vs weight-concat); LIFO pool discipline + the MAX_BUFFERS sizing fact + the concurrent-streams guard as the decision content.
- [x] T2.5 `vendor-batched-gemm-contiguity-staging` (B−) — contiguize slices to dodge the pinned-host staging path; CS≤32 tri-solve fast path; arch caveat (HIP-measured).
- [x] T2.6 `gate-prep-fold-into-consumer-via-packed-side-tensor` (B−) — coordinator call: rule or rider on the fold-into-consumer family (the packed side-tensor + hard capability-gate are the new content).
- [x] T2.7 B63 rider: loader-placed stacked aliases as the placement-time twin of `same-input-gemv-weight-concat-at-upload` (doc-comment note, the B72 rider precedent — remember the concepts test is exact-match `.contains`).

## Phase 3 — rust_perf crossover

- [x] T3.1 `runtime-int-exponent-powi` entry + `BenchSpec` (sizes 1K/100K/1M; buggy `x.powf(n as f64)` vs fixed `x.powi(n)` with runtime `n: i32`, f64 + f32 arms) — **rel-error gate, NOT bit-identity** (powi ≠ correctly-rounded pow; the ported-numeric MANDATE applies: exhaustive-boundary equivalence is impossible here, so gate on max rel-error across a swept exponent range + document the ulp bound honestly, the newton-refinement annotation precedent).
- [x] T3.2 Quiet-box measurement discipline (verdict-stable ×2) + alloc-det stable.

## Phase 4 — standing gates + count pins

- [x] T4.1 Extend the standing syn-parse gate (`batch77_pseudocode_parses`) + `batch77_rules_present` + `batch77_concepts_covered` (case-sensitivity: the B51 needle lesson — 5th strike on record).
- [x] T4.2 Walk the count-pin sites (the ~19-and-growing family: header counts, serde round-trip pins, drafter test names, kernel_categories table, unannotated/annotated counts ×3, router experts-cover pin, benchable counts). Grep-verify every edited rule's `bench_spec`/`context_notes` lines after each corpus edit (the B73 lesson).
- [x] T4.3 `//!` doc-lint pre-emption: backticks on every code-ish token (`.rs` names, `O(x)`, magic numbers); NO line-initial `+ `/`- ` in wrapped prose (the B37 9-strike lesson); string-heavy entries via python here-doc with explicit escaping, never edit_file (the B76 escape-stripping lesson).

## Phase 5 — GOAT + records

- [x] T5.1 GOAT: kernel_opt + default + all-features lib tests green; `clippy --all-targets` 0 both states; full `ci_feature_guard.sh` ALL LAYERS; retrieval floors HELD zero decay (kernel 100/100/100, blend@1, perf@1 if the crossover lands, clippy@1); verdict-stable ×2.
- [x] T5.2 Update: Research 091 status line → RECORD with the landed count; queue snapshot entry; `.docs/11_domains/rust_perf.md` + kernel_opt doc rows; AGENTS.md batch row.
- [x] T5.3 Commit + push (`feat:` prefix, develop).

## Non-goals

- No engine-policy corpus entries (adaptive spec / verify cap / block guard → riir-ai, already recorded there).
- No B28/B63 new rules for their positive instances (riders/validation notes only).
- No M3/Metal work (the source is HIP/CUDA; the M3 lane is Issue 755's separate track).
