# Research 278 — the prism fork delta: shard-alignment + pending-rollback fork laws

**Status:** DISTILLED — pending owner decision (Batch 246 landed 2026-10-16; the corpus is the authority)

## Source

PrismML-Eng/llama.cpp (MIT — the llama.cpp fork lineage; our league opponent),
branch `prism` delta `e3de715222..5ae2e169eac202d2f065606539c3ad6ab8b51333`,
3 commits, polled fresh at the vendor clone `E:/git/vendor/prismml-llama.cpp`
(both diffs read in full at the pins this session; every landed quote
verified verbatim):

1. `71c14732072dcdb959a5a2dac250c8da2d45bd22` — #363 "ggml : support -sm
   tensor for Hadamard-folded qwen35 (Bonsai 2)" → RULE-GRADE B+.
2. `5ae2e169eac202d2f065606539c3ad6ab8b51333` — #354 "memory : keep a
   pending rollback across seq_cp, and default MTP draft depth to 2 on
   CUDA" → RULE-GRADE B+ (the seq_cp half) + RIDER-GRADE B− (the MTP half).
3. `b858f3bdaa` — #362 "tests: portable popcount in arclab_init_hadamard
   (MSVC has no __builtin_popcountll)" → NULL (test portability, no law).

The same poll: llama.cpp upstream envelope `b76b1337` → `2f729cfa` (2
commits: Muse Spark Open model support — the quantizer-sensitivity datum
"the low-bit types turned the output into a repeating loop" is lane intel
for the lossy-surface family-conditional rule, not a mechanism; lightonocr
native-resolution mtmd default = NULL). No new prism-b tag (ladder top
stays 10777); no ggml-cuda absorption movement in the delta → the
watch-971 bout trigger does NOT fire (no league re-pin owed).

## The conversion (Batch 246 / Plan 250)

**+1 kernel_opt rule (L), corpus 847→848:**
`shard-granularity-lcm-never-cut-rotation-block` — the SHARD-ALIGNMENT law
family, four faces:
- **The lcm granularity**: 'Split blocks for Hadamard-rotated weights use
  lcm(type block, rotation block) so shards never cut a rotation block.'
  (`blck_size = std::lcm(blck_size, int64_t(it->second));` over the
  name-keyed `hadamard_weight_blocks` map; the grouped-GDN ssm_out row
  widens the grain by the group factor:
  `granularity_qkv * (hparams.ssm_dt_rank / hparams.ssm_n_group)`).
- **The axis-survival reshape**: 'llama_mul_mat_hadamard reshapes to 4D
  [n, ne0/n, ne1, ne2*ne3] so the split axis survives the rotation, and
  asserts ne0 % n == 0.' — the 2D fold destroyed the per-axis boundaries
  the splitter needs ("the meta backend had no split state for it").
- **View split-state inheritance**: views of the GATED_DELTA_NET output
  carry the matching axis-1 split when v is an integer multiple taller
  than q (`{GGML_BACKEND_SPLIT_AXIS_1, {0}, {uint32_t(v->ne[1] /
  q->ne[1])}, 1}`).
- **Mirrored-operand slicing**: 'Elementwise MUL/ADD/SUB/DIV of a
  feature-sharded activation with a full-width mirrored operand slices
  the operand per shard (ADD_ID is excluded).'
- Measured (source-reported, 2x H200): Bonsai 2 27B PQ2_0/Q2_0 under
  -sm tensor, mean KLD 2.5e-4 vs single GPU, top-1 99.3%; single-GPU +
  Qwen3-4B/Qwen3.5-4B bit-identical to base. Attribution: 'Split rules
  by Benjamin Brown (@KodeMunkie), posted on ggml-org/llama.cpp#29600;
  ADD_ID guard added here.'

**+1 rust_perf rule (caching), corpus 217→218:**
`seq-fork-carries-pending-rollback-record` — the serving state-cache
family's FIFTH face (admission / keying / eviction / reuse-admission /
FORK): 'With n_rs_seq != 0, a partial seq_rm records the rollback in
rs_idx and the next decode applies it. seq_cp shared the cell but not
rs_idx, so a sequence forked right after a rollback read the state from
before the rollback.' Three repair arms: the recurrent record copy
('a pending partial rollback is applied at the next decode, so the copy
must use the same snapshot row' — `rs_idx[seq_id_dst] =
rs_idx[seq_id_src];`), the snapshot-plane copy ('Copy the snapshot plane
that the pending rollback points to into the current plane of the
destination instead.' — `sc_info.ssrc.push_back(rs_src*n_stream +
(uint32_t) seq_id_src);`), and the plane-count sizing
(`size_t(2)*(1 + size_t(n_stream)*(1 + n_rs_seq))*...` — the u32 product
overflows first; views iterate `n_planes = n_stream*(1 + n_rs_seq)`).
The oracle: 'Add a test case that forks a sequence with a pending
rollback and compares its state with a sequence that rolled back
directly.' — 'The fork must match it bitwise' + 'compare the state bytes'
(logits mask the state defect; the state bytes do not).

**+1 count-neutral rider** on `cost-aware-verify-length-survival-cost-table`
(entries_gpu_86, B223): the PLATFORM-KEYED verify-cost face — 'On CUDA
the cost of a verify batch grows faster with the draft depth than on
Metal. A depth of 3 drafts tokens that do not pay back their verify cost,
and 1 or 2 is faster. On Metal, 3 is still the fastest of 1, 2 and 3.'
(COMMON_SPEC_MTP_N_MAX_CUDA 2; n_max_set tracks the explicit flag — 'An
explicit --spec-draft-n-max always wins'; the platform axis is the cost
table's third key column beside (format, path)).

## Dedup (both vocabularies, pre-edit)

- `seq_cp|pending rollback|rs_idx|snapshot plane|fork.{0,20}rollback` →
  ZERO corpus hits. The family kinships adjudicated in-rule: P65 owns
  WHAT a partial write owes; B219 owns keying/eviction; P66 owns
  admission; P71 owns reuse acceptance; P72's save-before-overwrite is
  the write-time twin; THIS owns the copy-time axis (pending mutations
  travel with the fork).
- `tensor.?split|shard.{0,25}(cut|block|boundary)|lcm(|rotation
  block|mirrored operand|split granularity` → ZERO corpus hits. Kinships:
  B233's reshape content-oracle is the OTHER half of the reshape contract
  (keep the boundaries vs prove the contents); smem/stride-align are
  within-kernel vectorization alignment; l2-grouped-tile-schedule is
  within-device; the ragged-chunk-pad family owns where zeros come from.

## Lane intel

The fork (our league opponent lineage) can now serve OUR league model
Bonsai 2 27B PQ2_0/Q2_0 across 2 GPUs (2x H200) — irrelevant to the
single-4090 league posture, load-bearing for any future multi-GPU bout.
The MTP depth datum (CUDA depth-2) is serving-policy intel for our own
speculative lanes (closed-negative here, but the cost-growth asymmetry is
the datum).

## Gates (all green, this session)

batch246 needle+parse both domains (needles verbatim from the diffs at
the pins) · rust_perf corpus tests 154/154 · kernel_opt corpus tests
405/405 + router 23/23 · default lib 2678/2678 · kernel_opt lib
3133/0/1i · clippy ×3 postures 0 errors (7 doc_markdown healed via
`cargo refine --fix --write --verify`, the B192 class) · retrieval floors
HELD no re-pin (blended 90.6/94.2/95.3 vs floors 89.4/93.6/93.6; kernel
self-variants 100.0%; 58.55 s) · score-bench run #185 appended · gen2+gen3
freeze e2e `--ignored` both ok (57.84 s / 39.56 s).

Count pins walked: rust_perf 218 (mod.rs doc + tests.rs ×5 incl. the
serde + drafter 163→164 pair) · kernel_opt 848 (mod.rs doc + L-category
comment + membership + tests.rs ×14 historical live-count asserts +
router.rs) · AGENTS ×2 · README · both .docs ledgers.
