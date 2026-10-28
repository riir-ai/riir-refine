# Research 344 — Batch 302: the 10-28c staged kernel_opt pool (TensorFold own-rule set)

**Status:** RECORD — Batch 302 LANDED (the landing commit carries this note; corpus 994 → 999).

## Source + provenance

ashhart/TensorFold, five pins (blobless clones `.raw/b302/tf`, verified 2026-10-28, plan 307 T0):

| pin | commit | rule |
|---|---|---|
| fd058f5 | 10-28T09:03Z | `topk-sampler-specialization-carries-general-tie-rule` (B) |
| a05d620 | 10-28T09:25Z | `multi-projection-prefill-parts-table-body-literal` (C) |
| d59a895 | 10-27T11:08Z | `stage-copy-preserves-fetch-unit-order-swizzle-at-read` (A) |
| 0ecaf33 | 10-27T16:13Z | `whole-window-specialization-omits-metadata-poison-verified` (D) |
| 15af3c0 + c8084c4 | 10-27/28 | `capacity-widening-priced-by-shared-allocation-layout` (A) |

Faces (count-neutral, landed by the slice-B lane): cubecl `a38a1258` #1949 (compile-pipeline overlap → `compile-run-rejection-persists-patience-compiles-on-reach`, gpu_134) · cubecl `12c4840c` #1948 (cmma lhs fragment layout → `stage-fill-shared-physical-axis-reads-through-projection`, gpu_134) · llama.cpp `f5222f25` #31188 tag b11962 (routed-MoE GLU landing → `fuse-gate-up-gemv-activation`, gpu_01) · llama.cpp `a7af149a` #31288 tag b11953 (384-expert ladder → `moe-group-selection-fused-into-topk`, gpu_104). All quotes byte-verified at the pins (33/33 by the slice-B verification pass; the coordinator's rule quotes read at the pins directly).

## The five laws, one line each

1. **Sampling specialization must carry the general path's tie rule.** A top_k draw depends only on the row's top_k members, so a two-pass path (slice-partitioned candidate cut, bounded at 16×top_k per slice; one-simdgroup merge) serves it — but only because it reuses tf_sample_full's "own expressions, sum orders and tie rule"; the race is a tie-rule surface. Bit-equality is by construction (216,000 shaped draws equal; served tokens SHA-equal).
2. **Multi-projection prompt fusion is a parts table + a body literal.** One grid routes blocks to per-part descriptors and rebases to projection-local indices; admission is a screened-shape allowlist; the body must stay a literal — sharing its helper changed Blackwell SASS under nvcc 13.0/13.3 (compiler-version stability surface, pinned by five-SM receipts).
3. **The stage copy's unit is the fetch unit; the swizzle lives at the read when the desired layout splits one.** A stage-time swizzle that scatters a 32-byte L2 sector's halves double-fetches the sector; global-order copy + read-side fragment indexing is bit-identical and −28%. This states the granularity boundary that bounds the gpu_41 Batch-252 rider's within-unit swizzle-at-stage form.
4. **A specialization that omits metadata must prove it never reads it — by poison.** The whole-window loader's ABI omits the page arguments; the host test runs production tile reads with `UINT64_MAX`-poisoned page fields against the page-aware instance — identical reads are the proof (the absent-work guard, kernel-side).
5. **Capacity widening prices its own delta through the one shared layout source.** `choose` admits the chip-default widening only when runner-scratch + frame delta fit the room beside weights and margin at the configured stream count; explicit widths stay exact; admission and the allocator derive from the same `Layout`/`allocationSizes`, so the cost model cannot drift.

## Adjudications + drops

- **TF 0afe9dd** (nvfp4 equal waves): DROP — thin wave-quantization cleanup, one small-bore guard.
- **uzu 6d96616 #1026** (f32 matmul path removed): DROP — routing hygiene.
- **vllm 6abdff5 #63018**: DROP — python host-side tiling.
- **The staged "#29181 still unlanded" note was STALE** — the grouped-expert top-k fusion landed at Batch 250 as `moe-group-selection-fused-into-topk`; #31288 therefore landed as its face, not a new rule.
- **Row 27 (15af3c0 + c8084c4)**: LANDED as an own rule (A) — the widening gate + shared-layout law is kernel_opt memory governance (the `residency-hold-gated-ram-fraction` precedent); the M5 config datum itself stays league-lane intel, never a corpus claim.
- **mlx `32a183e` #4775** (all-negative affine groups: `w_max = 0` → `-Limits<T>::max` in BOTH arms): VERIFIED AT PIN by the slice-B lane (grep verdict: no identity-init owner); face PROPOSED on `single-sweep-minmax-for-quant-scale` (gpu_06) — recorded, NOT landed this batch (scope control). The proposal text lives in the slice-B report.

## Gates (all green, 2026-10-28, M3, RUSTUP_TOOLCHAIN=1.99.0 per the 963-floor override, CARGO_TARGET_DIR=/tmp/b299-gate)

- batch302 needle gate + parse gate: 2/2 FIRST-RUN PASS.
- kernel_opt lib **3277/0**; default lib **2759/0**.
- clippy -D ×3 postures (default / kernel_opt / no-default-features): clean (5 doc-backtick mechanicals in the new file header healed en route).
- Retrieval floors HELD both postures, no re-pin: kernel_opt+latent 28/0 (258.5 s); default `retrieval_` 27/0 (63.4 s).
- Freeze/score-bench NOT owed (kernel_opt-only, P30); release-set binary rebuild NOT owed (kernel_opt outside the release set).
- Fixtures: all 10 (5 ref + 5 opt) standalone rustc-validated + test-executed green at `/tmp/b302_fix` before landing.

## Session note (the parallel-lane shape)

Slice A's subagent session wedged (repeated stream idle timeouts, no file written); the coordinator authored entries_gpu_135.rs directly from the pinned diffs. Slice B (the faces) landed its own commit (`119b6f02`) with a recovered index-sweep incident (temp-index rebuild; the sibling's staged set preserved byte-identically). The checkout carried the sibling auto-oracle lane's live WIP throughout (`clippy_oracle.rs` / `heal.rs` / `fix_verify.rs` — the union tree read E0433 at claim time and compiled again by gate time); zero overlap with the batch's write set, all gates run on the union tree.

Queue state at landing: B298 (bbtree, rust_perf) still leased to 2026-10-29T09:44Z (respected — zero rust_perf rows here); B300 (perfect-hash, plan 305) unclaimed; B301 (plan 306 Phase 3, this lane's staged batch) staged behind both.
