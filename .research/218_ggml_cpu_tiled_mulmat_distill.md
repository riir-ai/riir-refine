# Research 218 — ggml-cpu tiled mul_mat k-quant distill verdict (Batch 186 / Plan 182)

**Status:** RECORD (Batch 186 landed 2026-09-28 — the staged 09-26 lead converted at the kernel slot; 15 kernel_opt rules + 2 rust_perf members + 5 staged; plan renumbered 181→182 in-commit, the sibling pickup session took 181 first — the dual-allocation gate adjudicated by mentions, zero inbound to mine). **UPDATE Batch 187 (2026-09-28): the 5 staged rules CONVERTED** at the next kernel slot (Plan 183, `entries_gpu_55.rs`, corpus 670→675) — the staging queue is empty; the record below stands as the distill of record for all 20 rules.

## Provenance

- Source: `ggml-org/llama.cpp` @ `d834d44e643681f7b046a22d357335f6f4ff6107` —
  "ggml-cpu: tiled mul_mat for k-quants" (#27851, jbooth, 2026-09-26T05:38:57Z, MIT).
  11 files ALL ggml-cpu + tests: `iqp.cpp/.h` REMOVED (~1292 lines), a NEW
  `ggml/src/ggml-cpu/tiled/` family (~2175: tiled.cpp 1245 + tiled-kernel.cpp 719 + headers),
  `ggml-cpu.c` +48 restructure, `tests/test-tiled-mulmat.cpp` 852. Commit's own numbers:
  **3-6x for large matmul, break-even at 4096x64 * 64x4096, 80% (net loss) for GEMV**,
  quantization-class error only (max ~1e-4, rmse ~1e-5).
- Reading box: shikuwa/4090 (Windows); **source located IN the standing
  `E:/git/vendor/prismml-llama.cpp` clone** (the commit is upstream history the prism fork
  tracks; HEAD there is `adfffbe41b` = prism-b10743) — NO fresh clone consumed, nothing to
  clean (the ephemeral-clone law discharged by the standing vendor clone).
- Verification: every load-bearing quote re-verified VERBATIM at the pin THIS batch (Batch 31
  discipline) — min_batch tiled.cpp:1202-1205, the dead-tail 615-642, narrow k_extent 664-679,
  wdata_size 651-658, ACTBIAS precompute 685-692 + the b128 xor 651-659, maddubs overflow
  216-226 + the instantiations 540-548, NK dispatch 505-512, VNNI interleave 568-576 + the
  q8_K static_assert, jit repack 543-546 + 815-822, zero-pad 541-562, chunk stepdown 1096-1106,
  force env 578-586, the consult site ggml-cpu.c:1269 + the plan-site reservation, the MMID
  group 710-722 + the per-expert floor 1230-1244, the owned F32->Q8_K transform 1068-1082,
  the flush bench 343-357 + 441-443, the toggle oracle 108-126 + 201-207, metadata widening
  tiled.cpp:36-56 + kernel 125-133, the writeback transpose 684-696, the removed iqp
  late-consult comment (the ggml-cpu.c `-` lines). Two subagents over disjoint slices
  (driver+microkernel | integration+tests+removal) proposed; I merged, adjudicated, and
  re-verified.
- Staging caveats discharged (the 09-26 staging row's three): (a) k-quants are NOT our scored
  PQ2_0/PTQ1_0 ternary class — what landed is the REGIME/admission/staging/measurement
  substrate, never the architecture (a kernel port is not a bounded fix-space); (b) dedup grep
  CLEAN — `tiled mul_mat` / `tile dot` / `microtile` / `dpbusd|VNNI` / `activation bias` /
  `offset term` / `bsums` / `iqp` / `unpack-once` / `gfni` / `workspace slot` /
  `batch floor|GEMV` = 0 corpus hits each; the 13 `repack` file hits are the GPU
  register/smem repacks (one-time-dense-repack-keyed-sync-publish,
  register-double-buffer-operand-pipeline, c-fragment-to-a-operand-register-repack) —
  different layer, cited as kinship not duplicated; (c) the iqp.cpp removal — the corpus has
  ZERO iqp-derived rows (grep clean), nothing staled; iqp's surviving lessons are folded
  into the landing rules (its MIN_BATCH 8 floor survives in regime-batch-floor-force-escape;
  its late-consult ordering comment is the death certificate cited by
  owned-input-transform-early-admission; its unpack heirs are the shared
  tiled_lut8/tiled_unpk_sign32/tiled_unpk_tern8 primitives).

## The live upstream defect found en route

`unified-admission-dead-tail` (landed as a rule) records a **LIVE DEFECT this commit ships**:
in `ggml_tiled_supported` (tiled.cpp:615-642, verified dead at the pin) the src1 type gate +
contiguity check sit AFTER a switch whose every arm returns and never execute — a non-F32
non-Q8_K src1 passes the "gate" and hits only a debug assert inside the driver (silent
garbage in release). The commit message's own "Unified/simplified dispatch and support
checks" introduced it; the test file carries a third copy of the gate (three spellings of one
predicate, one dead). Worth an upstream report by the owner at their discretion.

## Landed — Batch 186 (15 kernel_opt + 2 rust_perf)

Corpus 655 -> 670; A 139->145, B 69->71, C 49->50, D 143->146, H 29->30, M 20->21, N 51->52;
rust_perf 163 -> 165 (NotBenchable 64 -> 66, annotated 109 -> 111). Files: entries_gpu_53.rs
(A x6 + B x2 + C x1), entries_gpu_54.rs (D x3 + H x1 + M x1 + N x1), rust_perf entries_10.rs
(+2). Gates: batch186 parse gate x1 (15 rules, all fixture pairs parse); default lib 2385/0;
kernel slice 2762/0; +latent retrieval 2936/0 (21/21 floors HELD); clippy -D both postures
clean (the healer took 11 doc_markdown/doc_lazy_continuation mechanicals in the two new
entries files, compile-gated); score-bench NOT owed + no freeze regen (zero clippy-corpus
delta, P30). The one landing defect: I pinned 671 (16 rules) before counting — the ledger is
15 kernel_opt; the kernel-slice run caught it in the 5 count pins, fixed to 670 in the same
batch (the pins did their job).

1. `dequant-once-macrotile-stage-reuse-across-microtiles` (A) — the CORE win: per 256x256
   macrotile decode once into the widened staging tile; the 16x16 microtile sweep runs 256
   MACs over plain bytes ('quants are unpacked into (max) 256x256 tiles of int8, one routine
   per quant'; the microkernel loads codes with aligned loads, no decode). Regime: reuse >= 16
   (3-6x band); anti: GEMV reuse=1 (the 80% net-loss arm). Kinship: the DRAM/L2 tier of
   c-fragment-to-a-operand-register-repack; amortize-over-reuse vs
   packed-code-cpasync's per-use decode.
2. `stage-operand-mac-layout-inplace-transpose-at-repack` (A) — the VNNI operand reshape:
   'interleave the natural [row][k] codes in-place into the group-local
   [kg%16][kg/16][row][4] layout' (16x16 int32 butterfly, 4 phases); per-ISA clause (the
   AVX2 arm keeps natural layout — maddubs reads rows, dpbusd wants the word order).
3. `plan-time-thread-slot-scratch` (A) — per-thread 512KB slots in plan-time wdata
   ('64 + n_tasks * ggml_tiled_ws_size(); // 64 for alignment plus one 512KB slot per
   thread'), ith-sliced, 64B static_assert pins, **0 when unsupported**; the commit paid for
   the threadlocals->wdata migration TWICE ('No threadlocals anymore, aligned wdata access' +
   'Cleaning up alignment of wdata').
4. `narrow-m-l1-panel-sizing` (A) — the memory-bound regime's K-panel formula:
   `k_extent = 31*1024 / (16 + rows)` ('fit weights in 31k to leave room for scales'), 256
   floor, buffer cap, K-divisibility walk-down, **ZERO = refuse** to the stock path (not
   clamp — a wrong extent silently mis-tiles).
5. `jit-band-repack-stage-per-use-l1-hot` (A) — repack each 16-row band at first touch
   (j0-outer loop), never the whole operand: 'so each repacked region is L1-hot for its
   uses' + 'Save work in low n_rows cases and optimize L1 usage'. Anti: random band access
   (repack thrash — the narrow path's whole-chunk choice is the correct contrast).
6. `zero-pad-ragged-tail-additive-identity` (A) — pad tails to the microtile multiple with
   zeros ('(n_rows + TILED_MICRO - 1) & ~(TILED_MICRO - 1)' + the memset arm); waste MAC
   over zero rows < predication on every tile. Anti: non-linear consumers.
7. `maddubs-overflow-two-exact-mac-selector` (B) — the i16-intermediate overflow law:
   act-bias maddubs fits iff `2*255*Wmax < 32767` (Wmax <= 64); the sign trick always fits
   (`2*127*127 = 32258 < 32767`) at two extra sign ops; the selection is a COMPILE-TIME
   template param per format ('ACTBIAS selects the act-bias MAC (Wmax <= 64); iq4_xs (Wmax
   127) keeps the sign trick'). The consumer-side completion of B181's ISA law.
8. `hoist-row-invariant-scale-to-subblock-epilogue` (B) — exact-int accumulation;
   per-subblock scales fold in the int domain, the per-row d applied ONCE at writeback
   ('d is NOT applied here: it is constant over the s-loop, so we apply it last before
   write-out'). Kinship: B107 epilogue placement, B180 exact-isum.
9. `consumer-offset-absorption-precomputed-weight-row-correction` (C) — the offset family's
   FIFTH member: activation +128 at stage (one xor per vector), corr =
   `128 * sum_s scales[s] * w_bsum[s]` PRECOMPUTED per weight row at repack ('Reused across
   all act bands and weight groups'), stored in the mins slot HAS_MIN formats leave unused.
   Deltas recorded in-rule: vs B107 (lookup-per-use), B96 (dot-time sums), B180 (weight-side
   deferred), B181 (writer-side elimination — this is consumer-side ABSORPTION where the
   u8xs8 ISA forced the offset to exist). Regime: Wmax <= 64 (the selector rule is its
   gate).
10. `regime-batch-floor-force-escape` (D) — the measured break-even as a one-line in-callee
    floor ('Profitable at rows >= 8, take even when unprofitable if we're forced') +
    `GGML_CPU_TILED_MM_FORCE` the TEST/BENCH-ONLY escape ('take the tiled path even when
    unprofitable' — the bench sets it to measure the LOSS regime); the floor is re-evaluated
    PER EXPERT (cne1, never the node batch). The escape is what keeps the floor falsifiable.
11. `unified-admission-dead-tail` (D) — the live-defect rule (see above); the B176 contrast
    arm: the caller side IS one predicate (B176's confirmation), the callee-side unification
    was done by concatenation after a total-return switch — admits-then-UB, strictly worse
    than B176's admits-then-rejects. Fix shape: merge, never append.
12. `owned-input-transform-early-admission` (D) — consult at the op's TOP
    ('If tiled is supported, it will execute the full op here and we return') and own the
    F32->Q8_K convert ith-partitioned inside the gated driver; the removed iqp path's own
    comment ('must come after the barrier above, it consumes the q8_K rows') was its death
    certificate — the late-consult coupling died with the stock restructure.
13. `moe-per-expert-mixed-dispatch-worst-case-workspace` (H) — per-expert floor on the
    EXPERT'S routed rows; hot experts tiled / cold experts fall through IN ONE OP; the
    workspace reserved unconditionally at plan time for the worst case ('The reservation is
    unconditional, the per expert eligibility is decided at dispatch time'); gather/scatter
    staging (expert_rows pair table + col_ptrs) reusing the dense kernel UNCHANGED; MMID
    group 64 'finer than the TILED_TILE_K kernel tile so all threads stay busy at small
    ne01'.
14. `const-generic-microkernel-format-unification` (M) — 13 formats -> ONE widened tile +
    ONE const-generic kernel `<SUBBLK, HAS_MIN, BIAS, ACTBIAS>` ('q4_K and q5_K share the
    constants'); **iqp.cpp -1253L is the deletion receipt**; the NK=1/NK=0 instantiation
    split is the second instance ('num_k==1 dispatches to the NK=1 instantiation, whose
    strides/offsets are compile-time constants (shifts/scaled-leas). num_k>1 ... is
    memory-bound, so one version serves all of it').
15. `widen-metadata-once-to-mac-native-widths` (N) — scales u8->i32, d/dmin fp16->f32,
    bsums i16->i32 at unpack ('widened to int32 so the kernels load them directly, no
    per-use cvt'; 'f32 (not f16): q8_k stores fp16, the unpack converts once'); ONE rule —
    the same polarity across three granularities (per-row d, per-subblock scales,
    per-column bsums).

rust_perf (entries_10.rs, both NotBenchable complexity-annotated):

16. `fallthrough-toggle-oracle` — the correctness oracle for a gated fast path is the SAME
    graph through ONE admission switch (cpu_set_use_ref true/false), on identical quantized
    bytes, at the magnitude-scaled tolerance `fmaxf(1e-3f, 1e-3f*scale)` with an f32 control
    at 1e-4 — never a drifting hand-written second impl. The measurement-honesty family's
    ORACLE member (B177 prices the comparator; this builds it from the gate).
17. `cold-l3-evict-min-of-n` — memory-bound kernel benches flush a >whole-chip-cache 256MB
    buffer between reps (`memset(flush_buf, 0xAB, flush_size); // Evict L3 cache first`,
    flush_size = 256*1024*1024, n_reps = 5) and take min-of-N after one warmup — every
    reading streams DRAM like deployment; the warm path stays reachable by passing no flush
    buffer. The COLD member, inverse polarity of B181's seed-warm (regime decides which).

## STAGED for the next kernel slot (full quotes — zero re-mining needed)

All five verified at the pin this session; quotes below are the conversion record.
**CONVERTED at Batch 187 (Plan 183, 2026-09-28)** — entries_gpu_55.rs (A×1 + B×3 + E×1),
corpus 670 → 675 (A 145→146, B 71→74, E 64→65); quotes re-verified VERBATIM at the pin
again at conversion time; gates: batch187 parse gate, kernel slice 2763/0, lib 2385/0,
clippy -D ×2, retrieval floors HELD (21/21); score-bench + freeze not owed (zero
clippy-corpus delta). The staging queue is now EMPTY — the next kernel slot needs a
fresh lead.

S1. `jmajor-acc-buffer-transpose-writeback` (A) — accumulate in a j-major buffer (row
    stride buf_stride) so MAC stores are contiguous, then write back to the i-major dst
    through a 16x8 register transpose: '// Writeback of the 256x256 window: buf is j-major
    (row stride buf_stride), dst is i-major (column stride dst_stride).' (tiled.cpp:679) +
    the `float r[16][8]` load-then-transposed-store loop (tiled.cpp:686-695: `r[t][u] =
    buf[(ri + t) * buf_stride + rj + u];` then `dst[(ri + t) + (size_t)(rj + u) *
    dst_stride] = r[t][u];`). Anti: dst already j-major (store directly). Kinship:
    c-fragment-to-a-operand-register-repack (same transpose shape, opposite direction —
    they repack for the MAC operand, this for the store).

S2. `register-budget-band-partition-indep-correction-offband` (B) — partition the 16x16
    microtile into two 8-row band passes sized to the register file; the independent
    correction term computes AFTER the band pass: '// Register pressure: the band pass
    holds acc16 + s1_acc = 16 zmm for the / 8-row band (s2_acc has no dpbusd dependency, so
    it is computed after the / band pass and adds no register pressure);'
    (tiled-kernel.cpp:91-93) + '// s2_acc += mins*bsums; independent of the dpbusd results,
    so it runs here instead of in the / band pass: the band pass keeps the register budget
    for the 8-row band' (~:169-170). Anti: 128-bit ISAs (the AVX arm needs no banding).

S3. `chunk-downsize-work-stealing-microtile-floor` (E) — chunk size starts at one macrotile
    (256) and halves while `nchunk0 * nchunk1 < nth * 4 && chunk_size > 16` ('// Step down
    chunk size if too few chunks to saturate cores, minimum is microtile size'
    tiled.cpp:1103-1104); the floor is load-bearing (chunks below the tile break the tiling
    invariants); first chunk pre-assigned by thread id ('// Every thread starts at ith, so
    the first unprocessed chunk is nth. This saves a bit of coordination right at the
    start.' ~:1082-1083); dense path = atomic work-stealing (with the recorded TODO to drop
    it when cores are provably equal) vs the MoE path's static uniform-cost split
    (`(ngroups * ith) / nth`). Anti: huge workloads (fixed chunks, no counter traffic).

S4. `lut16-pshufb-byte-expansion-at-stage` (B) — 16-entry byte-LUT code expansion via one
    pshufb per 16 codes at unpack: '// 16-entry byte LUT: dst[j] = lut[src[j]] (16 bytes)' +
    `_mm_shuffle_epi8(_mm_loadu_si128(lut), _mm_loadu_si128(src))` (tiled-kernel.h:92-95);
    the +128 activation bias is FOLDED INTO the table entries ('// LUT values are stored as
    kvalues + 128 so the +128 shift the kernel applies to / the activations cancels against
    the bsums correction (BIAS = 128)' tiled.cpp:234-236). Anti: >16-entry tables (gather,
    not pshufb). Kinship: packed-code-cpasync-stage-per-use-prmt-decode — the CPU (pshufb)
    tier of the GPU (prmt) shuffle decode.

S5. `gfni-sign-mask-expand-cmpeq-fallback` (B) — grid+sign expansion: broadcast 4 sign
    bytes via pshufb, then the all-ones-per-set-bit mask in ONE instruction when GFNI is
    present: '// 0xFF in every lane whose sign bit is set; GFNI does the and+compare in one
    instruction' + `_mm256_gf2p8affine_epi64_epi8(sel, sv, 0)` with the portable fallback
    `_mm256_cmpeq_epi8(_mm256_and_si256(sv, sel), sel)`; negation is branch-free
    (`v ^ mask - mask`), +128 gives the biased code (tiled-kernel.h:105-116, verified at the
    pin). Anti: non-x86 (the cmpeq form IS the portable version).

## Adjudicated NOT rules (the record)

- The 13-format eligible-type list — a backend config fact, not a mechanism (the gate shape
  is unified-admission-dead-tail's/B176's territory).
- The per-type SUBBLK/HAS_MIN/BIAS/ACTBIAS parameter table — decode plumbing; the
  const-generic rule (M) carries the unification lesson.
- The test's shadow path-labeler (a third spelling of the gate) — evidence for the dead-tail
  rule, never an independent mechanism.
- Scatter-store blocked transpose internals (tiled_store_window's 16x8 detail) — the
  transferable half IS staged rule S1.
- `std::call_once` env-flag caching — OnceLock idiom, clippy culture, no kernel content.
- iqp panel layout / VNNI bias trick as standalone rules — dead at this commit (path
  removed); the surviving lessons are folded into the landing rules (floor 8, late-consult).
- Scalar-twin discipline (every AVX2 primitive carries a scalar arm) — standard practice;
  the ONE nuance kept as fixture guidance: the scalar kernel is written as EXACT integer
  math so it doubles as the bit-parity oracle for the SIMD arms.

## Batch-186 landing record

- Commits: this batch (feat: batch 186) on riir-clippy develop; plan
  `.plans/182_batch186_ggml_cpu_tiled_mulmat.md`; snapshot row in `.distill/001`.
- Pins walked: corpus/tests.rs (total x4 sites + per-category table x7 deltas + the batch186
  parse gate), corpus/mod.rs (mods + extends + KERNEL_CATEGORIES lists x7 categories +
  comment counts), router.rs:472, rust_perf tests.rs (total + NotBenchable + annotated x3),
  AGENTS.md x4 (kernel_opt bullet + kernel_opt line + rust_perf bullet + rust_perf line),
  README kernel row, kernel_opt.md (seed row + the Batch-186 ledger paragraph), rust_perf.md.
- Landing defect (caught by the pins, fixed in-batch): the first pin pass wrote 671 (+16)
  — the ledger is 15 kernel_opt; the kernel-slice run failed 5 count pins and the 670 fix
  landed before commit. The pins did their job.
- Queue after: staged pack above (next kernel slot) + derivable_impls (DEFER maintained) +
  the coverage-report re-arm + walk #12 Mon 10-05. NOTHING INVENTED — every mechanism is in
  the commit; the fixtures are corpus-shaped Rust, never the source code itself.
