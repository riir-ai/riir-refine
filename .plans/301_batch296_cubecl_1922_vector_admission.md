# Plan 301 — Batch 296: the cubecl #1922 vector-admission law (kernel_opt)

**Status:** IN PROGRESS — claimed shikuwa-4090-b296-vector-admission, epoch 1793117000, expires 2026-10-29T05:00Z

**Source:** tracel-ai/cubecl PR #1922 "fix: check every outer stride before
vectorizing tensor reads" (laggui), MERGED 2026-10-26T20:10:36Z, merge
`5bd6d17aa883276bf4cf76eadb8ad518c4799647`, 3 files +61/−15, 0 review comments.
The staged lead from the 2026-10-27 item-11 sweep (B−) — re-graded **B** at the
full-diff read (the "thin" flag was the sweep's commit-message-only read; the
diff carries a concrete matcher, two source tests, and a real failure mode).

## The law

A vector width admitted for a strided/overlapping (unfold) tensor read is
valid only if EVERY non-axis stride is a multiple of it — the minimum
non-zero stride (or any single-stride proxy) does NOT certify the rest:
overlapping views carry outer strides that are not multiples of the innermost
(`shape [1,2,4], strides [10,4,1], axis 2`: min non-axis stride 4 passes width
4, but row 1 starts at offset 10 — misaligned). Broadcast strides (zero)
satisfy every width. The admission asymmetry: over-strict = false negative
(vectorization disabled, safe); under-strict = wrong results (a misvectorized
read). Host-side source quote (cubecl-core/src/lib.rs, the FIXED predicate):

```rust
.filter(|&vector_size| {
    axis_shape % vector_size == 0
        && strides
            .iter()
            .enumerate()
            .all(|(i, stride)| i == axis || stride % vector_size == 0)
})
```

The BUGGY predicate it replaced (the founding failure, verbatim from the diff):

```rust
.filter(|&vector_size| axis_shape % vector_size == 0 && next_stride % vector_size == 0)
// next_stride = strides.iter().enumerate().filter_map(|(i, &s)|
//     (i != axis && s != 0).then_some(s)).min().unwrap_or(0);
```

## Tasks

- [x] T0: pin verify — PR #1922 merged 2026-10-26T20:10:36Z, merge commit
      `5bd6d17aa883…9647` (API read this session); quotes re-verified against
      the fetched .diff (crates/cubecl-core/src/lib.rs
      `try_tensor_vector_size_parallel` + the two new tests
      `parallel_unfold_with_odd_outer_stride_rejects_vectorization` /
      `parallel_unfold_outer_stride_limits_vector_width` +
      `test_into_contiguous_unfold_unaligned_rows` e2e). No clone made (API
      diff fetch only) — nothing to sweep.
- [x] T1: dedup — 3+ vocabularies grepped (vectoriz/stride/align/vec_width).
      Nearest kin adjudicated: `stride-align-vectorization-key` (L, Batch 7
      popcorn) owns the WIDTH-DERIVATION axis (min trailing-zeros of
      stride×bytes → the width — a perf derivation); `smem-fill-align-chunk-not-element`
      (N, B137) owns smem base alignment from the FILL CHUNK (a fault class);
      `ptxas-recoalesces-scalar-runs-wash` (A, B137) owns the escape-arm
      static alignment PROOF. NONE owns the ADMISSION axis (a candidate width
      is valid only if every outer stride divides) nor the min-stride-proxy
      failure mode (a subset check that passes while another stride breaks)
      nor the unfold/overlap shape nor the broadcast-0 carve-out. OWN rule,
      cross-cites all three. NOTE: the min-trailing-zeros derivation happens
      to be safe-by-construction (it sees all strides); the failure class is
      the planner that validates a SUBSET — the rule's teeth.
- [ ] T2: author 1 own rule `vector-width-admission-checks-every-outer-stride`
      (category DtypeAlignment/N) in NEW slice `entries_gpu_132` (B295 took
      131): concept (the all-strides admission law + the min-proxy failure
      mode + the unfold shape + broadcast-0 + the asymmetry), ref/opt fixtures
      (the [10,4,1] unfold failure as the ref; the `.all()` predicate as the
      opt; a standalone-executable test), NotBenchable-by-domain annotation.
- [ ] T3: registration — mod.rs `mod entries_gpu_132;` +
      `entries.extend(entries_gpu_132::entries());`; batch296 gate
      (`batch296_rule_landed` needle + `batch296_pseudocode_parses` parse,
      batch295 shape).
- [ ] T4: count pins — 35 × `981,` → `982,` value sites in corpus/tests.rs
      (each message gains the "batch 296 re-pin: 982 (981 + 1: N
      vector-width-admission-checks-every-outer-stride — the cubecl #1922
      all-strides admission law, merge 5bd6d17aa, Plan 301 / Research 337)"
      prefix); router.rs:473 expert-total 981→982 + history prefix;
      tests.rs:5536 `(KernelCategory::DtypeAlignment, 85)` → 86 + comment.
- [ ] T5: gates — `cargo test -p katgpt... ` NO: `cargo test --lib` (default
      floor 2749→2752: +3 batch296 tests... re-derive at run), kernel_opt lib
      (floor 3259→+), clippy `-D warnings` ×3 postures (default/all-features/
      no-default), retrieval floors BOTH postures
      (`--features kernel_opt,latent_retrieval` + `--lib --features
      latent_retrieval retrieval_`) — riders/rules edit rule bodies even
      count-neutral; re-measure, re-pin only on measured decay (floor =
      measured − 5, Plan-030 convention).
- [ ] T6: freeze/score-bench adjudication — NOT owed expected (kernel_opt-only
      delta; the P30 law: generations.rs::atoms() builds from the clippy seed
      corpus ONLY — confirm live); binary rebuild NOT owed (kernel_opt outside
      the release set). Record both adjudications in the landing row.
- [ ] T7: research note 337 (`.research/337_*.md`, born-complete) + count
      bumps: AGENTS.md kernel_opt 981→982 (×2 sites), `.docs/11_domains/kernel_opt.md`
      batch row; snapshot `.distill/001` pool row (the cubecl #1922 lead
      DISCHARGED); plan checkboxes [x]; ONE commit `feat: batch 296: ...` +
      push (re-fetch before staging; the push is the arbiter).

## Gates owed (Protocol A §5, kernel_opt batch shape)

- G1: batch296 needle + parse (all new rules)
- G2: lib tests pass (kernel_opt feature + default)
- G3: no-regression — clippy 0 in THREE feature states
- retrieval floors re-measured both postures (corpus delta)
- freeze NOT owed (P30, kernel_opt-only) — confirm
- score-bench NOT owed (no rust_perf delta) — confirm
- release-set binary NOT owed (kernel_opt outside) — confirm

## Honesty notes

- The sweep graded B− "thin"; this plan re-grades B on the full diff (matcher,
  tests, failure mode all present). If authoring finds the decision content
  collapses, the honest drop is allowed — the plan then records the drop.
- Provenance: Apache-2.0 (tracel-ai/cubecl); fixtures are our own Rust, zero
  verbatim paste beyond the quoted predicate.
