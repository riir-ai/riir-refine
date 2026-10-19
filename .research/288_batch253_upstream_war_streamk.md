# Research 288 — Batch 253: the upstream window past the B252 pin (fattn-vec WAR, MMQ Stream-K cap, the VXE transcription rider)

**Status:** RECORD

**Date:** 2026-10-19 (shikuwa/4090 idle loop; StartLux stack resident+idle throughout — CPU-only batch)

## Provenance

The standing watch polled the upstream clone (`F:/llama.cpp`, fetched past the
B252 pin `ff5a2337`) and the prism fork (`git ls-remote` — no clone needed):
fork tip `1180918489` UNCHANGED, newest tag still `prism-b10777` — the decode
re-pin trigger NOT fired. Upstream yielded six new commits; five read at the
pin by two parallel reader subagents (every quote line-verified against
`git show <sha>:<path>`); the sixth (#30774, a warning fix) has no mechanism.

| commit | PR | subject | disposition |
|---|---|---|---|
| `99db719b` | #30573 | CUDA: fix shared-memory WAR hazard in flash_attn_ext_vec | **OWN RULE (D)** `loop-back-edge-scratch-war-trailing-warp-barrier` |
| `4c1004c4` | #30553 | cuda: cap MMQ Stream-K blocks at available work | **OWN RULE (E)** `stream-k-grid-capped-at-useful-work-quanta` |
| `42b70032` | #30791 | ggml-cpu: vectorize ggml_v_silu and ggml_v_expf | **RIDER (count-neutral)** on `fused-vector-site-transcendental-form-nonportable` |
| `31288c9c` | #30731 | hexagon: FLASH_ATTN_EXT improvements for longer context and SWA | WATCH (negative-assert pinned) |
| `fe8841a7` | #30218 | hexagon: allow multistream FA | WATCH (negative-assert pinned) |
| `412a363c` | #30607 | qwen3: bidirectional attention + dense_2 output projection | ARCH-WATCH only (no rule, no league relevance) |

## The two own rules

### D: `loop-back-edge-scratch-war-trailing-warp-barrier` (#30573)

`flash_attn_ext_vec`'s `__shared__ KQ` scratch is rewritten every KV-tile
iteration: each lane writes only its own slot (`KQ[j*nthreads + tid] =
KQ_reg[j];` — :303), a mid-body barrier covers the RAW edge
(`ggml_cuda_syncwarp(); // Needed for KQ synchronization.` — :320, the comment
ADDED by this commit), then an all-lanes read sweep consumes other lanes'
slots (:323-330). The loop back-edge had NO ordering: reads of tile *i* vs
writes of tile *i+1* to the same slots — a genuine WAR hazard that today's
schedulers happen to mask (the latent-race class: correct today, undefines
under Volta+ ITS or a compiler reordering). The fix is ONE trailing
`ggml_cuda_syncwarp()` as the loop body's last statement (:378) — warp scope,
justified by the per-warp-disjoint slice invariant (base = threadIdx.y *
WARP_SIZE). `ggml_cuda_syncwarp` = `__syncwarp()` with a HIP-empty guard
(common.cuh:128-132). Numerics: zero change; cost one warp barrier per tile
(left unconditional by the authors — the cheap-enough signal).

Decision content encoded: (a) the write-side barrier covers ONLY the RAW edge
— the loop-carried WAR edge is a distinct ordering requirement; (b) scope by
ownership — per-warp-disjoint ⇒ `__syncwarp`, cross-warp overlap ⇒ block
barrier; (c) correctness tests CANNOT pin this face (the hazard is masked by
the scheduler) — only the ordering argument can, so the test is a
deterministic hazard-DETECTOR that discriminates both postures; (d) the
barrier carries the buffer name it orders (an unexplained barrier is the
first thing a refactor deletes — the PR's own review half).

Dedup: distinct from `barrier-on-reader-open-group-conflict-swap-overlap`
(command-buffer level, MINIMIZES barriers) and
`hazard-tracked-aliasing-scratch-allocator` (spatial aliasing across
dispatches) — both cited in-rule. Category D beside the two barrier rules.

### E: `stream-k-grid-capped-at-useful-work-quanta` (#30553)

The MMQ Stream-K launcher sized the persistent grid at the bare SM count on
the non-high-efficiency arm; the cap computes
`nwork = ntiles_dst * ((args.ncols_x + config.K_vram - 1) / config.K_vram)` —
tiles × k-iterations, CEIL-ROUNDED so a partial trailing iteration still has
work — and launches `min(nsm, nwork)` (mmq.cuh:1447-1448). Why blocks go
unused: the kbc partition divides the k-space by gridDim.x and snaps to full
iterations (mmq.cuh:1067-1071), so when work < nsm the tail blocks get empty
ranges — they launch, do nothing, and each buys a fixup block
(`fixup_needed`, `tmp_fixup.alloc`, `block_nums_fixup` all key on the same
grid x, :1450-1458). The cap never reduces parallelism below the work.

Decision content encoded: (a) cap at the useful-work quanta, floored at the
SM count; (b) the CEILING-SIDE regime split is load-bearing — capping BELOW
the work is the measured null (`occupancy-grid-cap-refuted-uncapped-default`
lost 1-2% on the 4090); state both sides in-rule; (c) downstream fixup
couplings size from the CAPPED grid; (d) the determinism caveat is disclosed
in-rule — a different gridDim.x moves every kbc split point, changing FP32
summation grouping (the commit's own test disclosure; the Issue-940 class for
any synced reduction). Upstream shipped NO perf figures —
commit-message-sourced, NotBenchable-here (the B205 precedent); league note:
inert for the big Bonsai cells, watch the small-matrix shapes at any fork
re-pin. Dedup: composes with B102 (decomposition granularity vs grid size),
refines B127's admission shape (ncta = nsm is exactly what the cap refines),
opposite direction to B234 (fill under-filled vs trim over-provisioned), a
different axis from B148 (residency vs launch grid).

## The rider (count-neutral)

`42b70032` #30791 adds the s390x VXE arm to `ggml_v_expf`/`ggml_v_silu` — a
VERBATIM transcription of the ARM-optimized-routines polynomial (same
constants `0x1.715476p+0f` / `0x1.8p23f`, same in-code error-bound contract
comment "the maximum error is 1.45358 plus 0.5 ulps...", same all-lanes
early-out shape) re-expressed in `vec_madd`/`vec_nmsub`/`vec_sl`/`vec_sel`.
This is B98's porting discipline executed, not a new law → RIDER on
`fused-vector-site-transcendental-form-nonportable`. The COVERAGE face rides
beside it: a lane-switch `#elif` chain missing a platform arm is a SILENT
scalar fallback (the parent vec.h had ZERO VXE arms while vec.cpp:239 keyed
bf16 dot on the same macro). Subagent dedup verdicts recorded: DISTINCT from
`newton-refinement-vs-hardware-transcendental` (no hardware vector expf on
s390x — a different axis) and from `exact-bf16-domain-activation-lut` (whose
declared f32 anti-regime arm — inline evaluation, 2^32 patterns infeasible —
is exactly this commit's posture). No perf numbers ship in-commit; not
claimed.

## The watch rows

- **#30731 (hexagon FA long-context/SWA):** the mask-skip prepass (compact
  list of KV blocks with any entry ≠ −inf) generalizes but is ALREADY OWNED
  by `mask-block-classification-skip-placement` + the tree-attention
  block-skip family; the hmx→hvx crossover retuning is Hexagon-DSP-specific.
  WATCH (rides the B250 hexagon-watch convention).
- **#30218 (hexagon multistream FA):** support-matrix widening (drops a
  `ne[3] != 1` refusal) — no mechanism. WATCH.
- **#30607 (qwen3 bidirectional attention + dense_2):** embedding-model arch
  (non-causal attention + an output projection head, voyage-4-nano).
  ARCH-WATCH only — orthogonal to our causal GDN serving path; introduces
  `n_embd_out`/DENSE_2_OUT vocabulary worth remembering for teacher/student
  models.

## Gates (all run this session, box state: 16-core i7-13700K, CPU 9%, StartLux resident+idle)

- batch253 needle gates **2/2 FIRST-RUN PASS** (2 rule sets + the rider host
  + 3 WATCH negative-asserts). En-route repairs BEFORE first green: (1) the
  rider's raw quotes broke the string literal (the B252 R1 lesson RE-FIRED —
  raw quotes in gpu_concept need `\"`; caught by the compile, fixed
  verbatim-escaped); (2) the #30218 watch needle `ne[3] != 1` was TOO
  GENERIC (matched 2 existing entries — 2 left / 0 right) — re-keyed to
  `ggml_hexagon_supported_flash_attn_ext`; (3) one `doc_markdown` residue
  (`entries_gpu_13` unbackticked in the file header).
- Count pins: 17 corpus-total pins + the category-table D/E rows + the table
  sum + the grouped total + the router experts pin — all re-pinned 903→905
  with annotations (the "18 count-pin sites" count from B252 = 17 here + the
  router pin; form1 6 + form2 11 swept by script, asserted == 17).
- Category headers: D 212→213; E 79→**83** — STALE-COMMENT REPAIR (the
  header read 79 through B249-slice-3's +3; the tests table pinned 82; noted
  in-rule per the K-header repair precedent).
- kernel_opt lib **3165/0** (was 3163, +2 gate fns); default lib **2696/0**
  (kernel_opt corpus compiled out — the corpus tests are feature-gated, the
  default count unchanged by design).
- clippy −D **×3 postures** clean (kernel_opt / all-features /
  no-default-features).
- Retrieval floors **HELD** — all 21 retrieval tests green (kernel
  self-variants 100%, blended floors above pin) — no re-pin owed.
- Freeze/score-bench NOT owed (kernel_opt-only corpus + count-neutral rider;
  the P30/B250 precedent).

## En-route lessons (re-fired, not new)

1. **The raw-quote class re-fired** (B252's R1 asm lesson): a rider quoting
   in-code comments with raw `"` inside a gpu_concept string literal breaks
   the parse — the compile gate caught it, the repair is verbatim-escaped
   (`\"`), never softened quotes. The house rule stands: quote verbatim,
   escape mechanically.
2. **Negative-assert watch needles must be UNIQUE anchors**: a generic shape
   (`ne[3] != 1`) matches unrelated entries and false-reds the negative
   half; key the needle on the commit's own function/symbol names.
3. **Stale category-header comments are a standing class** (K at B242, F at
   B252, now E): the header comments and the tests-table pins drift
   independently — repaired E 79→83 with the note; the header count must be
   read as a CLAIM, the table is the assertion.

## Queue state after this batch

Fork tip `1180918489` unchanged (decode re-pin trigger NOT fired). L8 A/B
owner-gated (verdict channel claude_code auth-failed ×3 across sessions; the
StartLux stop-go ask recorded in riir-train issue 635). Dated leads
unchanged: walk #15 ~10-20 M3, ai-1017 ~10-21, mlxfast ~10-24,
nnethercote ~11-01. B254 candidates: none staged — the intake row's leads
are consumed; the next window needs a fresh watch pass.
