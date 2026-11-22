# Plan 397 — maya-inkling-mini moved-surface re-distill (Batch NNN, 4090 lane)

**Status:** STAGED unclaimed 2026-11-22 (verdict YES B−, Research 429 §Delta; nominal Batch number re-derives at landing; 4090-lane execution posture — CUDA fixtures/cells run on the 4090, authoring/gates per the box-state law).

Source: alesha-pro/maya-inkling-mini, delta pin `be849daa` (B379's pin) → `e819852f822479af783638a467bfdfe2ed6882ec` (tip 2026-11-22). MIT (code); weights Apache-2.0 (never mined). Provenance: Project Maya v1.1.70 (mw00) ← Strata (Niko1221); most offered upstream in mw00/project-maya#1095. Delta content: RTX-3090 resident run (128 tok/s plain / 150–210 with the official drafter / 2,742 prefill / 344K FP8-KV context vs llama.cpp 57), specdec with the official DSpark drafter as default, dual-GPU EP + CUDA 13 + Windows, the 8-GB tier, the VRAM resident guard.

## Phase 0 — T0 re-verification

- [ ] Re-clone `.raw/maya-inkling-mini` at tip (or re-pin if upstream moved again); byte-verify every quote in Research 429 §Delta at the pin; close the unread residue (the 8-GB cache body in `ik2_main.cu` :825–1100 region — `ik2_kernels.cu`/`ik2_main.cu` cache pools + the miss-pool thread lifecycle; B379 closed the earlier residue, the 8-GB path is NEW).
- [ ] Dedup re-run at landing HEAD, 3+ vocabularies per candidate: `S3.FIFO|miss.*compute|pinned.*expert|host memory.*expert` (expect zero owners on the overlap axis; entries_39 owns ghost-table structures — adjudicate in-rule both directions); `break.even|SpecCtl|re.probe` (gpu_29 owns the flip — face, never a rule); `INT8 limb|DP4A` (B379-R2 owns the MMA arm — face on it).
- [ ] P113 upstream-state check per fix-shaped lead (expected: none — all decision content, upstream's own landed code; no outbound-PR lane).

## Phase 1 — authoring (order = grade)

- [ ] R-A `expert-miss-computes-on-cpu-overlap-gpu-cache` (kernel_opt, category G/host-orchestration; suggestion-only NotBenchable-by-domain — heterogeneous-overlap latency, no CPU validator; measured-source provenance 40–43 vs 12.5 tok/s RTX-5070-Laptop/8GB carried as regime data, never our magnitude). Fixture: original four-arm model — hit (GPU cache), miss-computed-on-CPU overlapped, miss-computed-serialized (the negative), tampered (CPU pool silent-fails ⇒ loud); the overlap-vs-serial divergence is the law's arm; the S3-FIFO/ghost-history admission + per-size-class pools carried as the cache shape.
- [ ] F1 face on gpu_29 — THE CONTRAST ARM, never a positive instance: gpu_29's bypass is monotone (flips to plain and never returns); SpecCtl re-probes (after 8 plain tokens, draft once to look again) because acceptance varies by prompt class — the face's regime condition names WHEN re-probing beats a permanent bypass. Count-neutral.
- [ ] F2 face on B379-R2 (the DP4A INT8-limb sibling arm + the exactness law "a code times its scale exact in FP16, the sum FP32" + "an NVFP4 weight is exact in FP16"). Count-neutral.
- [ ] F3 face on the measurement-honesty family (the FP8-KV 1e-5/3-bit-mantissa drift disclosure; Issue-940 carve-out + drift-guard corroboration in-rule). Count-neutral.

## Phase 2 — gates

- [ ] Fixtures standalone-compiled AND executed pre-entry (kernel_opt fixture slot re-derived at T3; debug AND -O).
- [ ] clippy -D × postures + lib counts + fmt gate + retrieval floors HELD (no re-pin) + pins_manifest --write + count web one-landing at landing HEAD.
- [ ] Score-bench/freeze NOT owed (zero clippy_lints surface — assert); release-set binary rebuild NOT owed (suggestion-only kernel delta — disclosed, owed at next maintained-binary lane).

## Phase 3 — record + teardown

- [ ] Research 429 §Delta Status → landed (batch number + counts); queue-snapshot LANDING row; claim row pushed BEFORE any corpus edit (expiry 48 h; push is the arbiter).
- [ ] `rm -rf .raw/maya-inkling-mini` at close; dual_allocation + numbering gates at push.
- [ ] 4090-lane note in the landing record: any timing/cell numbers quoted from OUR runs carry bench_preflight/box-state PROVENANCE (4090 exclusive posture per the rig rules); foreign 3090/5070 numbers stay source-provenance, never bars.
