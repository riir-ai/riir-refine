# Research 169 — Batch 144: the B878–B884 measured-GOAT kernel_opt distill

**Status:** RECORD — Batch 144 landed 2026-09-16 (corpus 607 → 612; A 130→134,
D 133→134; entries_gpu_41.rs + 2 riders on entries_gpu_09.rs).

## What was distilled

The three unchecked P14 intake rows in `.distill/001_mining_queue_snapshot.md`
(L145 rung 6, L148 rung 5, L150 P14 NEW) — the measured-GOAT distillation
channel (the B49 reopen path (c)), mined from the riir-ai 4090 prefill-mma
campaign Benches 878–884 (Issue 884 rungs T2a/T2b/T4). Sources are OUR OWN
league-lane records — read-only citations; the owning lanes keep their code.

## The five rules

1. `packed-code-cpasync-stage-per-use-prmt-decode` (A) — the B883 rung-5 GOAT.
   Packed 2-bit Q2_0 codes shrink the A stage 4× (resident 6,144 B = 128×48-B
   rows; payload 4,096 B — the 256-chunk fill computes from the PAYLOAD, never
   the resident size); the shrink buys double-buffered depth at ONE barrier per
   group (the SWAR predecessor paid stage-store ALU + a second barrier).
   Per-use 9-op PRMT decode measured ~free (B882 isolated it: v7→v8 = 1.26×
   from the decode alone; ~9 ops/8 weights vs bitplane SWAR ~15/4).
   v9 1.222×/1.224× vs v4-q8 AND 1.062×/1.064× vs v6tb (1.706→1.603 ms) —
   both denominators stated in-rule. Negative arms folded in-rule:
   per-fragment global-A staging (NEGATIVE both formats, both warp geometries;
   B882 clean 0.785×/0.861×, v8/v8t 0.989×/0.912×; B881's 1.006×/1.106× sat on
   a ~26% clock-inflated denominator — ranking stable only), the 256-entry smem
   decode LUT (NEGATIVE, B879 1.175× vs 1.261×, sequential-protocol caveat),
   full-size double-A (v6d 1.069× vs 1.112× — the 4× shrink is what flips
   double-buffering to positive). L2-SOL cap carried: the slab was
   bandwidth-bound (v9t 89.8%), never mma-pipe-bound. Selector hazard
   (0x7632→0x7362 transpose caught by the OFFLINE decode model, not on-device).

2. `ldmatrix-byte-subword-granularity-limit` (A) — the structural complement of
   B9's `ldmatrix-mma-layout-loads`: ldmatrix distributes WHOLE words to lane
   columns; byte-sub-word fragment consumption (2-bit codes, the mma quad's
   lanes consuming different bytes of one word) cannot be served under ANY
   swizzle. The forced shape (LDS.32 quad-broadcast + per-use decode) is also
   the measured winner. Word-or-wider consumers keep the positive rule
   (B880: +3.8%, 1.156×). The LUT negative (B879) bounds the decode mechanism.

3. `transposed-smem-stage-repairs-strided-epilogue` (A) — the B884 rung-6
   GOAT. An epilogue lane map across the contiguous axis amplifies L2 write
   sectors 8.01× (35.65M vs 4.46M minimal; byte-identical across v6tb/v9/v9t
   BECAUSE the epilogue text is shared — v6→v9 inherited v8's schedule; the
   audit-every-epilogue lesson is part of the rule). The [TOKS][33] stage
   aliases the freed B slab (stride = 1 mod 32 for writer conflict-freedom) +
   the `r = idx & 31; tk = idx >> 5` reader map = 128-B contiguous stores.
   L2 SOL 74.7→59.0%, tensor 34.2→55.1%, IPC 1.13→1.82, 1.533× over the route
   winner; T4's 1.8× gate CLEARED.

4. `per-block-scalar-slab-cpasync-stage` (A) — the honest-cap arm. A scalar
   slab cp.async'd ONCE per group per BLOCK via the 4-B `.ca` helper (`.cg` is
   16-B-only). Read sectors UNMOVED (84.4M both arms — L1 was absorbing the
   reuse): an issue-count/IPC rule, never quote it as bandwidth; the IPC
   figure is the COMBINED v9t→v10t witness. The traffic model's "0.7 GB"
   over-attributed — counters own the attribution (the model-vs-counters
   lesson is in-rule).

5. `profile-sol-table-before-compute-rung` (D) — the methodology rung. Record
   the SOL table + write/read sector counts and route by the TOP limiter
   before any compute-side rung. The family's prior rungs ran with NO ncu data
   while L2 SOL sat at 69.9/89.8/74.7% with the fixable 8.01× amplification.
   SOL + write sectors = MINIMUM routing evidence (the record ran 5 sections
   at 13-pass replays — elevated counters, locked clock). Close the loop in
   profiler order: the next rung's justification is the previous rung's
   residual list. PERF axes only — never a numerics-rung gate.

Riders ×2 on B9: `cpasync-multistage-tile-pipeline` (+ the packed-code depth
face — staging depth and operand FORMAT are one lever), `ldmatrix-mma-layout-
loads` (+ the granularity-boundary face).

## The probe-first corrections (intake drift, corrected at admission)

Both quote lanes + a caller spot-check line-verified every load-bearing quote.
The intake text drifted in nine places; the corpus carries the corrected facts:

- L2 SOL range is 69.9–89.8% (v6tb/v9/v9t), not the intake's "76–90%".
- The exact write-amplification ratio is 8.01×, not "8×".
- The bank-shift is a row STRIDE of 33 (= 1 mod 32), not a runtime shift.
- "6 kernel generations" is intake arithmetic; the record says the shared
  epilogue v6→v9 inherited v8's schedule.
- The 256-entry-LUT / 2-blocks / double-A negatives are B879 §Negatives (the
  intake said B880); the LUT row is sequential-protocol (weaker grade).
- The global-A negative endpoint 0.785×/0.861× is B882's clean re-run; B881's
  own numbers sat on a ~26% inflated denominator (both runs negative, the
  clean one is quoted).
- Resident stage 6,144 B vs payload 4,096 B — the intake conflated them.
- Lead (a)'s "iterated v*3 / __byte_perm / __vsub4" vocabulary is grep-ZERO
  across 878–883 — DROPPED as unverifiable; the verifiable decode-cost half
  (1.26× isolated) folded into rule (e).
- The "B879→B883 six-rung enumeration" and "one-section ncu pass" are intake
  proposals, not record facts — the rule carries the record-supported forms
  ("first ncu data for the family"; SOL + write sectors as minimum evidence).

## Gates (GOAT)

- G1: batch144_rules_present / _concepts_covered / _pseudocode_parses 3/3
  (10 pseudocode blocks parse via the syn gate).
- G2: kernel_heal lib 1869/0, default 1526/0; retrieval-precision re-measure —
  kernel 100/100/100 (10th consecutive), blended 94.7/96.9/97.7 (floors HELD,
  byte-identical; no re-pin).
- G3: clippy 0 in all three states (default / kernel_heal / +latent). Two
  doc_markdown warnings on the new header text fixed (healer 0 edits —
  doc-comment backticks outside its fix-space — manual per the documented
  fallback).
- Score-bench: FLAT (row #116, heal 96.2%, created 1.9%, 64 edits, vocab 39 —
  kernel_opt corpus provably off the scored clippy/perf path). Frontier
  snapshot appended (gen-8 boundary).

## Sources

riir-ai `.benchmarks/878_issue884_t2a_q8_act_gates.md` …
`884_issue884_t4_l2_traffic_rung.md` (read-only). No riir-ai/riir-gpu edits.
Plan 129 · Batch 144 · this note. Snapshot rows L145/L148/L150 → `[x]` in the
same commit.
