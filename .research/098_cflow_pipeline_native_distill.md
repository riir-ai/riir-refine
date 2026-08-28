# Research 098 — arXiv:2608.23841 Pipeline-Native Transformers (cflow) — distill verdict

**Status:** DISTILLED — pending owner decision (queued as Batch 85 / Plan 046; build-gated — box saturated at verdict time per Plan 337).

## Source

- **Paper:** arXiv:2608.23841, "Pipeline-Native Transformers: Co-Designing Model Architecture and CPU Inference for Bandwidth-Efficient Autoregressive Decode" — Tom Poperszky, 2026-08-24, 77pp, CC BY 4.0. Read in FULL (arXiv HTML render, 2026-08-28).
- **Code:** cflow — Rust CPU-first streaming inference engine; weights + binaries at huggingface.co/tompoper/cflow. **Repo itself UNREAD** (verdict graded on the paper's own mechanism descriptions + measured tables — all mechanism shapes needed for original-fixture authoring are fully specified in-paper; re-verify the repo at mine time if a fixture wants a source shape).
- **Lane:** the queue snapshot's POOLED item "2608.23841 Pipeline-Native 77pp CPU decode" — consumed by this verdict. The other pooled item (2608.22110 ANE study) remains pooled for the riir-ai Issue 726 owner-gated lane.

## Verdict

**YES, B+ — route `rust_perf` (NOT kernel_opt: CPU cache-hierarchy/SIMD territory; the GPU kernel corpus is complete and this contributes nothing to it).**

External precedent value, not a league opponent: cflow's 5.94 tok/s (30.9B MoE) vs llama.cpp 4.75 (dense Qwen2.5-32B) on one Ice Lake box is **not model-matched** (the paper says so itself, twice), it is single-token CPU decode only, x86-64 only (no ARM/NEON path), and our league silicon is M3/4090 — cflow cannot even run on the M3. Record as external context only; no perf-rematch action.

## What the paper is

A CPU-first co-design report: single-token decode is bandwidth-bound (arithmetic intensity 4 FLOP/byte vs machine balance 20), so bytes/token is latency. Contributions: (1) the cflow runtime — L2-sized 128×256 Q4 tiles (~18 KB) stored in **compute-consumption order**, expert offset bank (top-k only reads), fused QKV/gate-up stripe interleave, AVX2/AVX-512 Q4 kernels, staged direct-I/O expert fetch; (2) five **pipeline-native architectures** whose inter-layer dependency graphs are rewritten (`dense_delay` δd, `expert_delay` δe, CombineStyle) so a vertical stage-major schedule is mathematically valid — δd=1/δe=2 cuts critical-path bandwidth 9.00→4.50 MB/token (2.00×); (3) an 8-claim scorecard with **6 proven, 1 refuted (PREFETCHT0), 1 inconclusive** — the negatives reported at full length with mechanistic explanations (the honest-reporting shape matches our GOAT culture; cite as external precedent). The wall-clock realization of the delay window (§5.14): async I/O overlap on an NVMe-resident expert tier nets up to **1.68×**, predicted by net = (C+IO)/max(C, IO_eff) within 1%, with a measured **interference tax τ** on starved desktop memory (+19% compute inflation when I/O overlaps) absent on 8-channel server memory.

## Ranked candidate table

| # | Candidate | Domain/category | Grade | Decision |
|---|---|---|---|---|
| 1 | `weight-tile-consumption-order` — repack weight matrices as L2-sized tiles in compute-consumption order at load/conversion time; single-token GEMV then streams tiles sequentially with the activation slice L1-resident across the tile's rows | `rust_perf`/layout (or caching) | **A− (GOAT lead)** | KEEP — batch lead |
| 2 | `software-prefetch-storage-bound` — explicit SW prefetch (`_mm_prefetch`/PREFETCHT0 threads) buys ≈nothing when bandwidth is storage/RAM-bound and is ACTIVELY HARMFUL at decode scale: the paper's prefetch thread consumed ~48% of decode time walking expert regions at stride 256; disabling it alone = **1.92×** speedup (their single largest decode win) | `rust_perf`/io (annotation class) | B− | KEEP — second entry |
| 3 | Sibling-projection stripe interleave (Q/K/V + gate/up interleaved by output stripe so one activation pass serves 3 matvecs; structural 3× activation-load saving) | — | C+ | DROP → fold as a variant note inside candidate 1 (on CPU the activation is L1-resident through the matvec anyway, so the load saving is nearly free; the real value is keeping the FILE sequential — which IS candidate 1) |
| 4 | Conditional expert loading (top-k offset-bank reads, 16× at Gemma-4 scale) | — | — | DROP — positive instance of `eager-pointer-collect-lazy-fetch` (B76: data-dependent subset fetch, direct-access path; the offset bank IS the "direct access path" B76's fixed_code names) |
| 5 | Async I/O overlap window / delay-aware scheduler (§5.14) | — | — | DROP — architectural scheduler, beyond bounded fix-space; the interference-tax measurement lesson recorded below as a research nugget |
| 6 | Per-matmul `Vec<Vec<usize>>` allocation elimination (their 2nd decode optimization, 2.30×/1.92× cumulative path) | — | — | DROP — positive instance of the alloc family (`vec-with-capacity`/`avoid-clone`) |

**Yield estimate: 1–2 rules** after ~50% coordinator falsification (candidate 1 near-certain; candidate 2 annotation-class, needs the honest-loser verdict from a PF A/B bench or lands source-cited-only).

## Dedup-grep evidence (both vocabularies, run 2026-08-28)

- **Candidate 1:** corpus greps `tile|consumption|L1|L2|working.set|matvec|gemv|soa` across `src/domains/rust_perf/corpus.rs` + kernel_opt entries — nearest cousins are B62 `transposed-weight-register-gemv` (repacks weights **column-major for register-level accumulator shape** — different granularity: SIMD registers vs L1/L2 cache tiles; composable, not duplicate), B64 `loop-order-innermost-contiguous` (**loop reordering over an unchanged layout** — its own in-corpus comment explicitly fences off "repacks the WEIGHTS" as the B62 family; candidate 1 changes the layout), `soa-hot-field` (struct-field layout, not matvec weight tiling), kernel_opt B14 `row-tile-amortize-input-load` + B15 `block-contiguous-weight-layout` (GPU **global-memory access-count** rules — different substrate, cite as kernel_opt cousins in-rule). ZERO hits for tile-sized L2-resident weight layout at cache granularity in rust_perf. PASSES.
- **Candidate 2:** `prefetch` — **zero hits** in either domain. PASSES.
- **Candidate 4:** `eager-pointer-collect-lazy-fetch` (B76) read in full — the expert-bank pattern is its data-dependent-fetch instance. Killed as duplicate.

## GOAT-candidate test (candidate 1)

1. **Benchable** — matvec harness, tiled-consumption-order vs row-major, identical arithmetic, size sweep crossing L1 and L2 (1×..32× L1 working sets). B62/B64 precedents supply the harness shape. Must **sweep, never interpolate** (B76/B44 lesson) and **parameterize cache sizes** — the source's numbers are Sandy Bridge (L1-d 32 KB); M3 Max P-core L1-d is 128 KB, so absolute sizes shift while the ratio story (win grows with working set) should hold.
2. **Exact semantics** — tile accumulation order ≠ row-major accumulation order → **float sum-order changes → rel-error gate, not bit-identity** (the B64 gate shape).
3. **Magnitude story** — PMU-measured 7.29× fewer L1-d read misses (2.64M vs 19.3M, dense-down at 4× L1); wall-clock 1.12–1.36× on AVX2 at 1–4× L1 working sets. Mechanism: activation slice (1 KB per 256 lanes) stays L1-resident across the tile's 128 rows; weights arrive at L2 granularity in consumption order.
4. **Deterministic** — same input, same output, fixed layouts. YES.

## Honest caveats

- x86-only source numbers; the aarch64 M3 has no PREFETCHT0 (PRFM/PLD is the analog — candidate 2's bench on M3 needs inline asm or lands x86-benched + annotated).
- The head-to-head (5.94 vs 4.75 vs 1.65) is engine-plus-architecture, NOT quality-controlled — the paper flags this honestly; never cite the 25% as a runtime-vs-runtime margin (the clean row is llama.cpp 2.9× over vLLM CPU, model-matched).
- Critical-path 2.00× is analytic at the proof-of-concept geometry; at Gemma-4-shaped geometries the reduction falls below 1.50× (attention becomes the floor) — the binding-stream rule ("delays help exactly while the stream they amortise is binding") is the transferable insight.
- Code repo unread (graded from the paper); paper carries a disclosed hardware-spec erratum (Ryzen box was SATA/16 GB, not NVMe/32 GB) — re-measured, conclusions unaffected per the author.
- Inconclusive stage-major layout claim (confounded by SSD SLC cache state) — no rule materialized from it.

## Strategic crossovers (recorded, no filings — no consumer yet)

- **§6.3 speculative pipeline recovery vs Issue 721:** the paper proposes layer-granular speculation for STANDARD transformers (start layer ℓ+1 on x_in before ℓ finishes; rollback if ‖δℓ‖/‖x_in‖ < threshold; or a tiny linear router-logit→FFN-delta predictor). This is the same speculate-past-unverified-dependency economics as our token-granular tree verify (Issue 721, structurally closed-negative on verify economics) at a different granularity — and it is UNTESTED in the source (proposed as future work). One-line cross-reference only; the Issue 721 reopen triggers are unchanged.
- **Bandwidth-bound decode thesis cross-validates Bench 763/771:** our M3 decode floor finding ("FFN ~62-65% of peak post-f16; the floor is genuine kernel bandwidth") is the GPU-side instance of the paper's roofline argument; the Issue 771 smem GEMM (stage W+X tiles once, shared across 8 simdgroups) is the GPU-side instance of tile-streaming amortization. External validation of the lane, no action.
- **Interference tax (§5.14.2):** off-critical-path reads are not free — concurrent I/O inflated overlapped compute +19% on a starved desktop memory system (τ ≈ 0.1–0.2), absent on 8-channel server memory. A useful measurement-discipline cite for any future overlap A/B on this M3 box (relevant to ANE overlap work, riir-ai Plan 550 lane).

## Queue slot

- **Batch 85 / Plan 046** (`riir-clippy/.plans/046_batch85_cflow_tile_consumption.md`) — opened plan-first per the 2026-08-18 discipline; ALL corpus edits build-gated (box saturated at verdict time: two sibling test-suite runs + bench_791 sustained thermal A/B active; Plan 337).
- Sibling Batch 84 (oMLX delta, kernel_opt lane, plan 045 on disk uncommitted) is disjoint — never sweep their WIP files.
- Pooled lane after consumption: 2608.22110 ANE study (riir-ai Issue 726 owner-gated reopen) remains.
