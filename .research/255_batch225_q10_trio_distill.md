# Research 255 — Batch 225: the Q1_0 trio distill (Plan 227)

**Status:** RECORD — landed 2026-10-13, corpus 796→801, commit <this-commit>

## Source

PrismML-Eng/llama.cpp (MIT), the walk #14-verified conversion set:

- **`59f2b8485`** — v0.0.1-prism founding commit (2026-03-02): "ggml: add Q1_0 and
  Q1_0_g128 1-bit sign-based quantization support (CPU, Metal, CUDA)" — 29 files
  +1382/−1 (the walk staged 33/+1439; the measured stat is 29/+1382 for the
  founding commit alone — the delta is the merge commit's content; per-file numbers
  matched exactly: metal +308, arm +140, x86 +159, cpu quants +101, vecdotq.cuh
  +117, CUDA +224 across 5 files, ggml-quants.c +185).
- **`43e67bda3`** — 2026-03-13, "[cuda] fix mmq/mma path": 7 files +128/−61 — the
  accuracy fix that enabled the kernels the founding commit shipped disabled
  ("CUDA MMQ kernels included but disabled (cuBLAS fallback used for prompt
  processing) pending accuracy debugging").
- **`bc8122e43a`** — 2026-03-19 merge: pure merge of the two above (CUDA-only
  boilerplate + an ftype-name trim); no content beyond its parents.

Fresh clone `.raw/prism-b225`, HEAD checked out at the founding pin; every quote
in the landed rules re-verified at the pins via git-show blob reads this session
(the full-quote re-verification, not just spot checks).

## Method

2 authoring subagents over disjoint slices (A: CUDA + Metal, 11 files at both
pins; B: CPU + tests, 8 files), each proposing dedup-checked candidates with
verbatim pin-verified quotes. Coordinator adjudication converted 10 candidates →
**5 own rules + 5 riders** (3 agent candidates became riders; 1 thin candidate
folded as annotation; the 3 staged LEADS adjudicated separately below).

## The five own rules

| Rule | Cat | Mechanism (one line) |
|---|---|---|
| `mmq-tile-loader-thread-map-derives-from-block-geometry` | D | porting a shared tile loader: derive threads-per-row/scale-slots from the NEW format's geometry — the founding Q1_0 port copied Q4_0's `MMQ_ITER_K_Q1_0 / (4 * QR1_0)` (the /4 = Q4_0's two-nibble structure), landing 32 threads on a 4-block span; the fix `43e67bda3` derives `blocks_per_iter * QI1_0`; the stale `MMQ_ITER_K_Q1_0` macro SURVIVES the fix with the wrong-derivation comment (the tail the rule names) |
| `subbyte-preexpand-int8-tile-reuses-validated-mma-consumer` | B | admit a sub-byte format to the int8 MMA path by pre-expanding to ±1 lanes AT TILE-LOAD in the sibling type's tile layout; the `vec_dot_q8_0_q8_1_mma` consumer reused verbatim; only the smem tile carries the 8× expanded payload (1.5 bpw survives in gmem); the g128 twin converged onto the SAME tile macro (the bespoke `_g128` macro deleted) |
| `superblock-scale-replication-to-subblock-scale-slots` | A | the staging tile is the ADAPTER: one super-block scale (per 128) broadcast into all four QK8_1 scale slots at load; redundant reads hit L1; the consumer's per-32 indexing untouched — layout broadcast, not arithmetic folding |
| `sign-expand-one-bit-dp4a-symmetric-epilogue` | N | ±1 bytes ARE valid int8 dp4a lanes; the symmetric epilogue is one scale product — `'ds8f.y is the precomputed sum (not needed for symmetric quant)'`; carries the supersession caveat (Metal replaced by the popcount consume @ d0ce941ec, corpus-owned; CUDA dp4a alive in MMVQ) + the computed-but-unused-sumy anti-shape |
| `sign-codec-shared-magnitude-mean-abs` | N | the L2-optimal shared magnitude of a sign codec is `d = sum_abs / qk` (minimizes Σ(|x|−d)²), never max-abs; `'Just store sign of each weight directly (no normalization)'`; validation gate = the family-conditional BINARY budget rungs `0.025/0.40` vs ternary `0.01/0.15` (the dot budget diverges WIDER than the total-error budget — 1-bit dot error compounds through the ±d scale) |

## The five riders (count-neutral)

1. **gpu_16 dispatcher-eligibility** += the surface-REDUCTION face (`43e67bda3`):
   a deliberately-disabled sibling path is a loud `NO_DEVICE_CODE` stub citing the
   validation policy (`vec_dot_q1_mmq_dp4a_disabled`), and the admission predicate
   shrinks with the surface in the same commit (`!turing_mma_available(cc)` →
   pre-Turing refuses to cuBLAS) — never a plausible-but-wrong body.
2. **gpu_33 lut-free** += the x86 face (the THIRD ISA spelling: CUDA `__vadd4` /
   ARM b2b-LUT / x86 shuffle-fanout + mask-cmpeq @ `59f2b8485`), the provenance
   correction (the b2b-NEON g128 form exists FROM THE FOUNDING PIN — 7 months
   before the Batch-212 rider's `4fbda129`; #21273/#21636 are upstream-PR dates,
   not the form's origin), and the OR-with-1 one-op fold improvement note.
3. **gpu_33 per-block-rescale** += the mismatched-block face: when block ≠
   activation-block, the integer window SHRINKS TO THE ALIGNMENT — 4×32 sub-dots
   per 128-block, per-sub-block scale fold (`vmlaq_n_f32(..., d0 * d1)`); `QK1_0`
   pinned to 32 ("MUST match QK8_0") keeps the aligned path free.
4. **gpu_02 widen-index** += the vendor-view face (llama.cpp #28748 @ `48e9857`):
   the int32 wrap inside MSL's typed tensor view ("a slice is not used because
   its byte offset is int32 and wraps at 2 GiB"); fix = uint64 tile base +
   extent CLAMP (tileM/tileN), never a far base + slice. Third surface of the
   class (CUDA index math / Metal vendor view / typed-view APIs generally).
5. **entries_popcorn contiguity-guard** += the processing-set predicate face
   (vLLM #59340 @ `deedc7be`): `outputs=quanted_input if has_input_processing
   else None` — a staging copy is owed only when a transform actually runs; with
   None the helper aliases the input. The contiguity guard is the
   single-transform member of this family.

## Lead adjudications (the walk #14 intake)

- **#28748** → rider 4 above (correctness-class, thin adoption surface — the
  walk's grade confirmed; the class was already owned, the vendor-view face was
  the new content).
- **vLLM #59340** → rider 5 above (staging-family lead confirmed).
- **#30164** (`08ffba21e`, vulkan switch→ternary for CM2 quant block size) →
  **WATCH-ONLY**: the mechanism is real (a switch over a specialization constant
  lowers to worse code than chained ternaries, which keep `qk` as a
  specialization-constant expression) but the origin is GLSL→SPIR-V via glslang;
  transferability to WGSL (naga/tint) and MSL is UNVERIFIED and no wgpu/naga
  evidence exists on this box — per the intake row's own disclosure
  ("watch-only if the compiler paths diverge"), no rule lands. Re-open trigger:
  a real wgpu shader showing the same lowering divergence.

## Honest notes

- The walk staged the trio as "the future kernel_opt distill batch is
  unblocked" — the authoring found the Prism lineage HEAVILY pre-mined
  (Batches 96/115/116/127/180/193/198/204/208/211/214/217/223 cite later pins:
  `e311ed38f`, `d0ce941ec`, `7113f6462`, `01fd9521c`...). The trio's genuinely
  NEW content was the founding INFRASTRUCTURE the later commits refined — the
  five rules above — plus the two provenance corrections (riders 2's pin-date
  law; the founding-stat correction 33/+1439 → 29/+1382 for the founding commit
  alone).
- Slice B's defect notes (the ARM nrc>1 pointer fabrication at the founding pin,
  the x86 g128 scalar-only coverage gap, the NEON dequant uninitialized-read)
  are RECORDED NOT RULED — upstream #21273/#21636 later filled the ARM gap (the
  Batch-212 rider's lineage carries it).
- Slice B candidate 4 (the binary error-budget ladder) folded as the validation
  gate context inside `sign-codec-shared-magnitude-mean-abs` rather than a
  standalone rule (thin standalone content; the coordinator call the agent
  anticipated).

## Gates

- G1 parse: batch225_rules_parse_and_needles_landed (5 rules × needles + the
  5 rider hosts × needles + both pseudocode halves parse) — PASS.
- G2 lib: `--features kernel_opt` 2945 passed / 0 failed (1 ignored = the
  documented freeze-regen e2e); default lib 2504 / 0.
- G3 clippy `-D warnings`: default + kernel_opt + kernel_opt,latent_retrieval
  libs + all-targets kernel_opt — ALL CLEAN (after the header doc-lint heal:
  backticks + the `+`-initial doc line that read as a markdown list).
- Retrieval floors HELD: kernel self-variants 100.0%, blended 91.2/94.7/95.3 —
  no re-pin (21/21 retrieval tests green, the regression gate inside).
- Freeze/score-bench NOT owed: zero clippy-corpus delta (kernel_opt-only
  corpus; P30, the B223 precedent).
- Count pins re-pinned 796→801 at the ×5 sites (tests.rs ×4 + router.rs) +
  per-category counts (A 163, B 87, D 187, N 65; sum 801) — the B211
  pin-set-wider lesson held (two OLD full-string pins on rider-extended hosts
  converted to the rider-tolerant prefix form, annotated).
