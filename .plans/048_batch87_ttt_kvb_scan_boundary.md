# Plan 048 — Batch 87: TTT-KVB associative-scan boundary rules (kernel_opt)

**Status:** COMPLETE (2026-08-29) — all tasks closed; batch landed + pushed.

## Scope

The App-I **boundary complement** of B79's `first-order-affine-recurrence-associative-scan` (B79 = affine ⇒ scan, the positive case): what BREAKS chunk-parallel prefix-scan eligibility for linear-attention-class recurrences. 2 rules + 1 B79 rider. Zero in-tree firing population (GDN normalizes Q/K pre-recurrence = φ-side, SAFE; deltanet chunked kernels are the delta-rule class) — paper-distilled precedent = B79 itself. Source grade B: theory sound; 4.0×/1.19× numbers are authors' LaCT end-to-end runs — never cite isolated.

## Tasks

- [x] R0 — this plan opened BEFORE any corpus edit.
- [x] T1 — dedup grep: R1/R2 concepts vs B79 (`first-order-affine-recurrence-associative-scan`), `chunk-reset-cumsum`, `hot-swap-weights-preserve-autotune-caches`, `frozen-cooperand-hoist-out-of-perturbation-loop` + full-corpus concept grep — confirm COMPLEMENT not overlap (issue close condition: if R1/R2 collapse into B79 regime notes, land R3 alone + close). RESULT: complements confirmed — no state-norm coverage (all "normaliz" hits = RMSNorm fusion / quant unit-norms), no dynamic-kernel-params coverage (zero hits); chunk-reset-cumsum = the ZEROING contrast, encoded in the distinct-from.
- [x] T2 — 2 parallel subagents drafted R1 + R2; coordinator re-verified every quoted claim vs R516/B79 — ONE drift caught + corrected (R2 "the ONLY shape" → FIRST/minimal; R516 Table 2 shows V3-V6 parallel tok/s) + R1's "from t=1" softened.
- [x] T3 — corpus entries landed in `entries_gpu_08.rs` (R1 + R2, G category); grade-B provenance in-rule.
- [x] T4 — B79 rider landed (entries_gpu_07.rs): SCAN-ELIGIBILITY BOUNDARY sentences in gpu_concept + comment-line note in the provenance block.
- [x] T5 — standing gates: batch87_pseudocode_parses (cfg(not(kernel_verify)) per B70) + NEW batch87_state_norm_breaks_associative_scan (falsifiable breaker gate); count pins walked: tests.rs 400/G25 + history marker, mod.rs G-slice + count comment + 400-rule doc + history block, router.rs doc + assert, entries_gpu_08 header.
- [x] T6 — GOAT: G1 parse PASS; G2 kernel_opt 984/0/1i, default 752/0; G3 clippy 0 both states (3 self-inflicted doc backticks fixed) + kernel_verify compile + retrieval floors 14/14 HELD. Full CI guard NOT run (sibling in-flight B85/docker WIP on the shared checkout — targeted states validated; re-run at the sibling's quiet window).
- [x] T7 — research note `.research/103_batch87_ttt_kvb_scan_boundary.md` (Status: RECORD); .research/.highwater → 103.
- [x] T8 — docs: AGENTS.md Batch 87 section + kernel_opt count 400; `.docs/11_domains/kernel_opt.md` Batch 87 section + total; queue snapshot entry.
- [x] T9 — issue 054: all tasks closed, file removed per noise-reduction rule (record = Research 103 + AGENTS.md).
- [x] T10 — commit `feat: batch 87: ...` + push (staged-path guard held: sibling WIP on rust_perf/docker untouched).

## Non-goals

- No in-tree kernel changes (issue Non-goals).
- No riir-train filings (recipes govern TTT-KVB inner loops, not outer-loop LoRA).
- 4-probe kit + reduction trajectory stay note-only (issue Deferred).
