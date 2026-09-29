# Plan 187 — Batch 190: the cubecl #1731 umax-divisor LSR distill (single-lead kernel pack)

**Status:** COMPLETE — landed 2026-09-30 ~05:0x +07 (M3 idle loop; B189's queue row designated this lead the next kernel slot's adjudication). Source: `tracel-ai/cubecl` @ `30a7ef00cd56e72716130b7111f2cfaa8664664d` (PR #1731; clone ephemeral under `.raw/cubecl`, removed at close).

## Tasks

- [x] T0 Clone at the pin + read the #1731 diff + commit message; re-verify the watch row's quotes VERBATIM (the 01:5x grading was commit-message-only; diff re-verification owed at the slot) — CONFIRMED; the diff located the fix in the cubecl-llvm LOWERING (all UDivOp/URemOp rhs clamped; SDiv/SRem untouched — the unsigned-only contract has source evidence)
- [x] T1 Dedup grep under BOTH vocabularies (`divisor|udiv|umax|strength|scalar evolution|LSR|address reduction` + corpus operator names incl. the thermite `division-by-invariant-multiplication` kinship read); distinct-from rows written for every adjacency — ×8 kinships in-rule, zero collisions
- [x] T2 Adjudicate the verdict per the distill law — YES (solo pack, the B176 shape); category O (the IR-strength-reduction family's first RUNTIME-divisor member); NotBenchable-by-domain with the PTX-assertion verification discipline
- [x] T3 Author `entries_gpu_58.rs` (1 rule: `clamp-runtime-divisor-for-lsr-address-reduction`) + module tests + the batch190 parse gate (2 tests: parse + the unsigned-only contract gate)
- [x] T4 Corpus pins walked: 684 → 685 (tests.rs ×4 + router.rs:472 + the category-table comment + the O slice) + the C-comment repair 49→50 (stale since B186, honest label)
- [x] T5 GOAT gates: kernel_heal slice 2768/0; lib default 2385/0; clippy `-D` ×3 postures clean (1 doc_markdown backtick fix); retrieval floors 21/21 HELD (201.9s); score-bench + freeze NOT owed (zero clippy-corpus delta, P30)
- [x] T6 Research note `.research/222_cubecl_umax_divisor_lsr_distill.md` (RECORD) + `.research/.highwater` 221→222
- [x] T7 Docs: AGENTS.md kernel_opt 684→685 + batch row; `.docs/11_domains/kernel_opt.md` seed row + B190 pack; `.distill/001` queue row (same commit)
- [x] T8 Global binary rebuild at the release set + `--corpus-stats` verify (kernel_opt 685) + symlink target refresh
- [x] T9 Commit `feat: batch 190 — ...` + push (origin twin-check first); `.raw/cubecl` removed

## Ledger

| rule | category | kinship distinct-from | source quote verified |
|---|---|---|---|
| clamp-runtime-divisor-for-lsr-address-reduction | O (4→5) | strength-reduce-div/mod-pow2 (constant arm) · shader-fastdiv-mulhi (hand-written magic; compiler-runs-LSR is the routing axis) · division-by-invariant-multiplication (rust_perf CPU hand form) · hoist-bit-address-out-of-packed-row-sweep (LICM: dividend invariant vs advancing) · pack-scalar-params (host float reciprocal) · predicate-gated-skip-empty-workset (count≥1 host vs div site) · base-3-trit-packing (constant divisors) · telescoped-digit-coefficients (float-domain) | commit message + all 3 diff hunks (math.rs insert_umax_one + the UDiv/URem/SDiv/SRem quartet; offline_kernels.rs strided_walk; offline_tests.rs the no-mad.lo/mul.lo PTX assertion) — verbatim at `30a7ef00c` |
