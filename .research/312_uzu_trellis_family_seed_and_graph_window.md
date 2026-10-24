# Research 312 — Batch 274: the uzu trellis family seed + the graph-window + splash residue

**Status:** RECORD — Batch 274 landed (Plan 278; landing commit named in the snapshot row)

## Sources (all verified at the pins this session, 2026-10-24)

| Source | Pin | License | Clone |
|---|---|---|---|
| trymirai/uzu | `2229bf8` #998 + `034294c` #1002 + `333c5f2` #1004 + `43e9fe3` #996 (rider) | MIT | `E:/git/vendor/uzu` (origin/main `14b770894`, fetched) |
| PrismML llama.cpp fork | `b8639015e3` #378 + `66ae2fd88c` #379 (prism branch) | MIT | `E:/git/vendor/prismml-llama.cpp` |
| ggml-org/llama.cpp upstream | `280141e` #30804 + `9ff0d7e` | MIT | fetched into the fork clone (default branch is `master`, NOT `main` — the Slice-B drift correction) |
| incoai/splash | `28d0e98` (own rule) + `7b21db3`/`720f5fd` (thin rider, not landed) + 7 drops | Apache-2.0 | `E:/git/vendor/splash` (origin/main `3135ab4`) |

## Conversion verdict

+3 OWN kernel_opt (A `trellis-reversed-lsb-tape-aligned-word-state-decode` / B
`trellis-gemm-narrow-m-plan-finalist-and-paired-k-chunks` / D
`hadamard-transform-runtime-specialize-bounded-max-layout`, entries_gpu_120 — the trellis
surface's FIRST owners; grep evidence: `trellis` had ZERO rule owners, only prose mentions in
gpu_47 + gpu_26) + 1 OWN rust_perf (allocation
`batch-load-admission-sized-from-files-refuses-past-the-hard-budget`, entries_47) + 5
count-neutral riders (gpu_08 ×2, gpu_02, gpu_112, rust_perf entries_41) + 8 honest drops
(thin decision content each; reasons in the .docs sections).

The slice structure: Slice B (subagent) authored entries_47 + graded the graph-window and
splash groups; Slice A2 (subagent) graded the #996 fusion RIDER (host gpu_112 + gpu_94, with
the kernel-count correction: the absorbed 5th kernel is `delta_net_norm_gate`, not a second
tree-verify); Slice A (trellis trio) died to the subagent stream-idle timeout TWICE (the B268
lesson re-confirmed: even 3-4-rule prompts can die on this box) and was authored by the
coordinator directly from the full diff reads.

## En-route lessons

1. **The pin-set-wider class, fourth+ instance**: the 941→944 move surfaced pins at SIX
   additional sites the corpus-grep pass missed — the Plan-003 category-table tuples
   (`kernel_categories_counts_match_plan_003_table`), the ROUTER total
   (`rules_for_expert_total_matches_corpus` in router.rs — OUTSIDE corpus, the B259 lesson
   re-confirmed), the cross-domain rust_perf pin in `batch249_slice3_rust_perf_riders`
   (the B272 lesson repeating), the batch26 wgpu EXACT-CONCEPT pin (a full-string
   `contains` on the gpu_02 concept — any rider extension to that concept must update the
   pinned string BYTE-IDENTICALLY), and the batch265 rust_perf rider pin. The
   lib-test run is the only reliable enumerator: every count pin must be re-derived from
   the failing-test list, never from grep alone.
2. **Fixture string literals**: fixtures carrying assert-message strings broke the parse
   gate (E0762 via single quotes, then raw-double-quote termination). The durable
   discipline: FIXTURES CARRY NO STRING LITERALS — assert messages dropped; the class is
   eliminated by construction (now stated in the entries_gpu_120 header).
3. **Category-list inserts by anchor**: the id-insert anchor must be a member of the TARGET
   category's list — the same rule-id text appears in MULTIPLE category comments, so
   "insert before the first occurrence" landed the B rule in A's list, then C's list.
   Verify the landing by running the category-count test, not by grep.
4. **The stale-pin repair that wasn't**: category A measured 191 vs the expected 190 —
   the extra was MY OWN misinserted B rule, not a stale pin. Adjudicate against the diff
   before writing a STALE-PIN REPAIR note.

## Lane intel (no corpus)

- `280141e` is NOT in the fork's prism branch (ancestry verified): the scored-tag baseline
  `prism-b10787-73bfe6c` predates #378 (graph key by shape), #379 (batched F32 copies), and
  upstream's chunked graphs — all three are opponent deltas for the NEXT bout's context.
- Scored-tag check at claim + landing: `prism-b10787-73bfe6c` STILL the tip (ls-remote both
  sessions) — the bout re-arm trigger has NOT fired.
- mlxfast floor: the arenas ENDED (the 10-23 readout) — the ~10-24 floor stays discharged;
  next poll only if the arenas re-open.
- nnethercote ~11-01: still pending.
