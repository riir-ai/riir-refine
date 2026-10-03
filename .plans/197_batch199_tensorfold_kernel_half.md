# Plan 197 — Batch 199: the TensorFold staged pool's kernel half

**Status:** LANDED 2026-10-03 (4090 slot, shikuwa) — Batch 199 = 7 new rules + 11 instance riders (the staged B/B−/C+ pool's kernel items 9/10/12/13/15/17/22; items 21/24 landed as riders; the rust_perf/serving half stages for Batch 200 per the recorded slot split). Quotes re-verified VERBATIM at the pin `c4646171139ee8a3c38103eaa1699dad226ec12b` (v0.6.0, **Apache-2.0, license flipped MIT→Apache-2.0 at 0.6.0**; ephemeral clone `.raw/tensorfold-b199`, removed at close — the standing clone lives on the M3). Gates: batch199 parse/contracts/riders 7/7/11 · kernel_opt lib **2833/0** (floor held; was 2826 at B198) · default lib **2424/0** (== baseline) · clippy `-D` ×3 postures green (24 doc_markdown header mechanicals in the three new files healed by hand — the B192 class; the residual build-script warning = the known box artifact). Freeze + score-bench NOT owed (zero clippy-corpus delta, P30); retrieval floors NOT owed (kernel_opt-only batch, P30). Corpus 739→746 (A 157→159, B 78→79, C 51→52, D 165→166, I 7→8, N 57→58).

## Scope this slot — the kernel half (the recorded split)

- [x] R9 `splitk-reduce-one-launch-slice-order-pinned` (C +1) — both split-K faces
      reduced by ONE kernel with the summation order pinned bit-identical to the
      per-slice loop, pinned by test; the qmm.cu cluster DSMEM (`cluster.sync()`,
      sm_90-gated `__trap()`) + exl3 linear.cu last-block atomicAdd election ride
      as IN-ENTRY instances. Staged line drift disclosed (:244-246 → :241-259).
- [x] R22 `per-group-epilogue-deferred-between-mma-bursts` (B +1) — the per-group
      scale/bias FMA epilogue parked in registers, applied BETWEEN the next
      group's MMA bursts; group order preserved. **Renamed from the staged
      `moe-epilogue-…`: qmm.cu is the general decode lane matmul (attention /
      DeltaNet / head projections per qmm.py's SHAPES16 table), not MoE.** The
      PIPE arm is DORMANT at the pin (every in-tree launch site omits PIPE,
      default false since a3274f1) — disclosed in-rule as
      decision-content-not-evidence.
- [x] R10 `declared-uncapturable-expert-declines-graphs` (D +1) — the capability
      DECLARATION axis: an expert that reads its plan on the host declares
      `capturable = False`, the engine declines graphs with one loud line BEFORE
      a capture fails; the capture tests ran EMPTY plans and could not see the
      class (the lane-intel gem). Correction recorded: the `capturable = True`
      flip commit is **603d3bc**, not the staged 64e2374 (that is the device-side
      plan commit that motivated it); the pin's test is renamed
      `test_the_nvfp4_experts_are_capturable`.
- [x] R13 `draft-context-ring-window-bounded` (A +1) — drafter KV in a ring of
      window+block+tile−1 rows; the capacity ESTIMATOR counts the ring not
      capacity (20 GiB estimated at a 1,048,576 window vs a 21.25 MiB ring), and
      kept-state snapshots copy the WINDOW rows first (else drop the drafter
      resume, never keep a torn window). Correction: the ring size rounds UP to
      whole tiles (2,176 @ window 2047/block 8).
- [x] R15 `row-independent-pass-block-bounded-scratch` (A +1) — prompt-chunk
      passes whose rows read no other row run in FIXED 512-row blocks; GiB-class
      fp32 partials (1.13 GiB/rank) and pool scores (2 GiB) shrink to ONE block
      each, block-vs-whole equality BITWISE-pinned.
- [x] R12 `quantized-stored-bytes-not-widened-patterns` (N +1) — the
      widened-pattern ANTI-PATTERN (their NVFP4 table stored one uint16 bf16
      pattern per weight = 4.00× checkpoint bytes); hold the checkpoint's wire
      bytes, decode in-kernel. The GOAT framing (decode-from-wire is novel) is
      REFUSED in-rule — it is the corpus's ambient default. Two provenance
      UPGRADES found: "the device holds the file's own bytes" + "The widened grid
      was four times the codes." are IN-SOURCE (tests/cuda/test_flashnext_nvfp4.py:186-187),
      not message-only as staged.
- [x] R17 `visibility-bounded-scan-shape-stable-allocations` (I +1) — tiles past
      the last visible complete pool store −inf WITHOUT dot products while
      KEEPING bucket-sized allocations for graph reuse; the radix select keeps
      max(K,·) well-defined; parity by torch.equal incl. ties UNDER
      capture+replay. Phantom staged quote ("The same function of each row on
      every path and at every row count" — exists nowhere at the pin) replaced
      with the pin's real law text, disclosed.
- [x] RIDERS ×11 — argmax-epilogue += the bit-preserving gate splice (item 21);
      verify-fold += streams-folded-into-row-axis; fwht-ladder += the CUDA warp
      tier (found at exl3/experts.cu:74-86 + a SECOND transcription at
      decode.cuh:143-158 — the twin-transcription hazard, flagged not deduped);
      gdn-transition-replay += the mamba deferred-commit window replay;
      ancestor-mask += the dflash2 analytic contiguous-window band;
      exact-bf16-lut += act_mode constexpr; exact-widening += the FP8_PB_WO
      stored-bytes form; consumer-offset-absorption += the Q8_0→affine-g32
      writer-side converter with the |d|>511 loud refusal (rider applied by the
      coordinator — its host lives in gpu_53, outside slice B's write set);
      cuda-graph-scalar += the capture-time derived-table arm (item 24);
      fp4-nibble += the parity-nibble + exact bitcast; shape-only += the
      FP8_PB_WO recipe + the n-gram weight_scale.

## Execution

- [x] T1 3 authoring subagents over DISJOINT write sets: A = rules 9+22 +
      riders on gpu_52/27/08 → entries_gpu_67.rs; B = rules 10+13+15 + riders on
      gpu_57/48 → entries_gpu_68.rs; C = rules 12+17 + riders on gpu_01/61/56 →
      entries_gpu_69.rs. Every quote VERBATIM at the pin; ~20 staging
      drifts/corrections found and disclosed (the headline: capturable flip
      commit 603d3bc; nvfp4.py lives at families/qwen4_exp/cuda/; 2 provenance
      upgrades; 1 phantom staged quote replaced). All work left UNCOMMITTED per
      the B198 method note; the coordinator applied the gpu_53 rider.
- [x] T2 coordinator spot-verification (5 Tier-1 quotes re-read at the pin:
      qmm.cu:136, dflash2.py:178, sparse.py:81, nvfp4.py:34, decode.py:141 — all
      verbatim) + mod.rs wiring (3 modules) + KERNEL_CATEGORIES (7 ids, 6 count
      comments) + tests.rs pins (746 total ×3 sites, expected table, batch199
      parse/contracts/riders gates) + router.rs count pin.
- [x] T3 Gates: batch199 parse (7) + contracts (7 markers) + riders (11 hosts) ·
      kernel_opt lib 2833/0 · default lib 2424/0 · clippy -D ×3 postures. Four
      gate failures during landing, all repaired: (a) one unescaped quote pair in
      the coordinator-applied rider (syn would not parse — fixed); (b) the
      batch22 exact-equality assert on the cuda-graph-scalar host is
      RIDER-FRAGILE — converted to a prefix assert with the law in a comment
      (riders APPEND to concept tails); (c) two stale count pins (batch116 row +
      router total) re-pinned 739→746 with honest labels; (d) the batch199
      'DORMANT' marker matched the entry's actual "dormant arm" wording.
- [x] T4 Commit `feat(batch199)` + push; `.distill/001` queue row (the
      rust_perf/serving half staged for Batch 200); `.plans/.highwater`
      196→197 in the same commit (dual_allocation_gate clean pre-allocation);
      `.raw/tensorfold-b199` removed at close (the ephemeral-clone law).

## Validation

Corpus tests at the batch parse/contracts/riders gates + full kernel_opt lib +
default lib + clippy at the three postures; every new rule carries its decision
content in the entry (annotation class, NotBenchable — no bench spec owed); the
rust_perf/serving staged half remains untouched for Batch 200 (items
5/6/7/8/11/14/16/18/19/20/23 + the Q8_0-adjacent instance filings' serving-side
half + the 10-03 fork pool staged behind per slot discipline).

## Landing corrections (disclosed, not silently absorbed)

1. The batch22 `concepts.contains` assert on `cuda-graph-scalar-via-device-pointer`
   pinned the FULL concept string by equality — any rider append breaks it. The
   B198 riders escaped because none of their hosts carried exact-equality asserts.
   The repair is a PREFIX assert (the documented mechanism text stays pinned as
   the concept's head; the rider law is cited in the comment). Future rider
   briefs should prefer hosts without exact-equality asserts, or apply the same
   prefix conversion when they land.
2. The staged pool's "15 rules + instances" arithmetic did not reconcile with
   the per-item split (21 and 24 were pre-marked rider-grade; 11(a) is a B100
   instance) — Batch 200's composer re-adjudicates its own merges; the split
   discipline (kernel half vs serving half) held.
