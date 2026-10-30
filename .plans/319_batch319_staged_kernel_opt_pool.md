# Plan 319 — Batch 319: the 10-30 staged kernel_opt pool conversion

**Status:** IN FLIGHT — claimed by zed-m3-b319 (see the snapshot claim row); kernel_opt-only per the B293 cross-domain precedent (the live claims are B301/plan-306 + B306/plan-310, rust_perf/sec — their rule counts stay ZERO in this batch).

Executes the watch1030b staged pool (`.distill/001_mining_queue_snapshot.md`, WATCH PASS 2026-10-30 ~08:31Z) — the leads staged at 08:31Z, dedup + byte-verify at the pin is this lane's T0–T1. Scope: 3–6 kernel_opt rules + count-neutral rider faces, mirroring the b317 shape. NOTHING INVENTED — every lead re-verified at its commit before entry.

## Source pool (from the watch, to be re-verified)

- **A — cubecl #1974 series (via the c1d0125 squash body; PR refs = the byte-verify path):**
  - (a) Metal indirect-command-buffer replay degradation (Apple driver degrades repeated ICB replays; copy-to-private-ICB keeps replay-1 speed) — HOME-RELEVANT (wgpu/Metal serving lane).
  - (b) loop-reuse of freed shared memory (a free inside a loop released nothing; barrier-ordering proof + two-array runtime test) — allocation law.
  - (c) nested profile windows (an autotune round under per-launch profiling read zero samples; windows read command buffers committed since open) — instrument-validity, the timed-region family.
  - (d) the perf/refactor pair: #1976 e2m1 word-at-a-time (PRMT/LOP3/SHF → single PTX block), #1977 block-scaled reads the form the blueprint states (`Packing::Words { field }`), #1986 sync_plane_storage (plane barrier ordering global memory; Metal mem_device, SPIR-V availability/visibility, WGSL double barrier), #1967 lazy read holds its source until read (`Handle::can_mut` correctness).
- **B — TensorFold f350cd3 (GLM `--mtp-confidence` series, 5 commits 10-30T04:32Z):** the `tf_glm_top1_sums` kernel (1024 threads, pick-parity + f64-sum contract) + Nemotron-style confidence-windowed MTP + the lanes law ("under a confidence floor a sampled stream keeps its rates' depth").
- **C — splash #621 staged-prefill-split** (merge 04:06Z 10-30; 64-row staged prefill tile + 33→64-row fused projection chunk in ONE dispatch) — DEDUP CHECK owed vs Research 345 lane-B / the S10 hand-off.
- **D — vllm kernel-shaped candidates:** #64494 size-gated PDL on silu_and_mul_nvfp4_quant · #64467 32-row MoE prepare_megamoe tiles · #64553 Muse Spark Open CUDA routing kernel · #64425 Qwen3.5 GDN in_proj_ba/in_proj_qkvz aux-stream overlap (HOME-RELEVANT, our GDN prefill lane) · #59131 DSv4.1 gather-compressed-KV-once · #60103 HiSparse per-layer slot-invalidation fusion · #64378 ROCm NVFP4 MoE re-quant + fused shared experts · #62009 EPLB symm_mem weight-migration.
- **E — upstream llama.cpp since the release pin:** #31073 kv-cache rotation constants kept on the GPU · #31368 refuse rollbacks past the last ubatch's snapshots.
- **F — opponent-class intel (league rows, NOT corpus unless kernel-shaped):** omlx #4946 A8 prefill, #4971 batch-KV bank growth; rapid-mlx #5376 self-measurement methodology (league-fairness intel only).

## Tasks

- [ ] T0 — pins + clones: fresh blobless clones under `.raw/b319/` (cubecl, TensorFold, splash, vllm, llama.cpp as needed); resolve every staged commit to its sha; the squash-body quotes re-anchored to the PR refs where the squash is the only surface (fetch `refs/pull/N/head`).
- [ ] T1 — dedup at origin HEAD (grep the corpus for each candidate concept, 3+ vocabulary variants; entries_gpu_142 is the newest owner file; S3/R1/R3 stay behind B301 to 10-31T13:41Z) + the C-vs-Research-345-lane-B/S10 dedup verdict + home-firing checks where the watch named a home class.
- [ ] T2 — selection: admit 3–6 B-tier rules (B STRONG/DISTINCT preferred: A(a) Metal ICB replay, A(b) loop-reuse freed SMEM, B `tf_glm_top1_sums` f64-sum contract, C staged-prefill-split if the dedup clears, D #64425 GDN aux-stream overlap if distinct); annotate bench_spec (BENCHABLE vs NotBenchable) honestly; count-neutral rider faces folded compose-never-merge.
- [ ] T3 — byte-verify every staged quote at the resolved shas (the b317 standard: verbatim body + guard + comment lines); corrections ride the entry, never the memory.
- [ ] T4 — fixtures: standalone-compiled AND executed (rustc --edition 2021 --test) BEFORE entry; one fixture file per rule-family, arms bounded both directions; fix fixture defects en route (disclose).
- [ ] T5 — entries: `entries_gpu_143.rs` (+ mod/tests/router wires; router expert-total bump); syn-parse gate extension `batch319_pseudocode_parses`.
- [ ] T6 — gates at the native 1.99.0 pin, isolated `CARGO_TARGET_DIR=/tmp/b319_target`, `CARGO_BUILD_JOBS` capped (the i1117 128k leg holds the box — b316/b317/b318 posture): G1 needles+parse; kernel_opt lib + default lib counts; clippy −D ×4 postures; retrieval floors (suggest-class delta, no re-pin expected); freeze/score-bench NOT owed (kernel_opt-only, P30); binary rebuild NOT owed (kernel_opt outside the release set).
- [ ] T7 — docs: AGENTS kernel_opt bullet + README + `.docs/11_domains/kernel_opt.md` batch line + this plan's checkboxes + the snapshot landing row (claim discharged) in the SAME commit.
- [ ] T8 — commit `feat: batch 319: ...` + push (the push is the arbiter); `.raw/b319` + `/tmp/b319_target` removed at close.

## Non-goals

- The rust_perf/sec pools (B301 Leviathan rusqlite/sec to 10-31T13:41Z; B306 infinity tokio to 10-31T14:21Z) — foreign live claims; this batch's rust_perf/sec rule count stays ZERO.
- Resin 294 + Batch 300 (plan 305) — rust_perf-domain, serialized behind the B301/B306 heads per the recorded adjudications.
- F-row opponent intel → the league lanes, not the corpus (except a kernel-shaped exception adjudicated at T2).
- S8/S9 (serving/spec-dec tier) — untouched. S10 — the splash lane's.
