# Plan 254 — Batch 250: the llama.cpp / prism / uzu envelope pool (staged claim b52a4874)

**Status:** SLICE 1 LANDED 2026-10-18 (T1-T5, T7-T10; T6 `- [-]` the x86 Pareto rides slice 2's window — BENCH-VERDICT-PENDING-x86 disclosed in-rule; slice 2 T11-T12 open): kernel_opt 878→879 (+1 D, uzu `2c16e967` route retirement), rust_perf 230→231 (+1 simd, the `3dbcb7f` hadd butterfly, ANNOTATED), 2 count-neutral riders (B233 k-pool hoist, B166 FWHT graph fusion); gates: combo-posture full run 3341 passed, the 2 failures were stale pins (router total 878, the B249 rust_perf-rider count 230) — re-pinned and the 6 affected tests GREEN by name (the gpu_103 byte-copy + the two stale pins caught by the T8 gate, Research 285 §incidents), floors HELD, score-bench #189 FLAT, freeze NOT owed. Research 285 (284 taken mid-claim by the 4090 lane's Yukon enumeration — citations renumbered). Prior status: IN FLIGHT — claim `b52a4874` (zed-m3-idle/M3 lineage, posted 2026-10-17 ~04:4x +07, expiry 2026-10-18T21:45Z); staging + full diff-reads landed 2026-10-17 ~18:2x +07; quote re-verification at the pins landed 2026-10-17 ~23:3x +07 (the .distill pre-composition row); composition resumed post-B249-slices (plan 253 COMPLETE) with slice numbers shifted: kernel_opt lands at **entries_gpu_103** (100/101/102 taken by B249 slices 2-3), counts rebased (kernel_opt 878, rust_perf re-pinned by B249 slice 3).

## Pool (all quotes verified at the pins this claim; the .distill row carries the verification record)

**Own-rule candidates (8):**
- rust_perf x1 — llama.cpp `3dbcb7f` "cpu: reduce AVX2 tiled accumulators together" (#30296): the register-resident epilogue hadd butterfly (`_mm256_hadd_epi32` pairs -> hadd -> `permute2x128` lo/hi -> one add) replacing 8x scalar-hsum-through-stack (`hsum256_epi32` removed). Grade B+/A-. Benchable on an x86_64 host ONLY (the M3 is aarch64 — the Pareto run rides the 4090 box, see T6).
- kernel_opt x7 — uzu `2c16e967` M5 attention route-table RETIREMENT by measurement (#973, B+, the pool's strongest) / `e707790`+`f74e2fa` SYCL Q8_0 MoE load-time SoA + the hazard face (B) / `623c163` HIP rms_norm float4 loads + x-in-registers (B-) / `13de11e` SYCL q4_K/q5_K wide stores per-block WI geometry (B-) / llama.cpp `2744e48` MMVQ low-bit dequant/dot rework (B-, post-staging) / `f2f667b` grouped-experts top-k graph-level fusion (B-, post-staging) / uzu `597e5e50` int8 trie row-mask bitmaps (staged).

**Riders (2, count-neutral):** `6afa9a3` k-pool shared-view hoist -> B233 `reshape-relabel-axis-scramble-content-oracle` (entries_gpu_92) / prism `ad2328151` graph-level signed-FWHT fusion + fallback-fix -> B166 `fwht-butterfly-staging-ladder-warp-smem-register` (entries_gpu_45).

**Watches (4):** `9d1878a4` HIP `__builtin_amdgcn_perm` q1_0 unpack (platform, HIP-only; same files as the MMVQ candidate — carried as its watch note) / `797de63b` hexagon Q1_0/Q2_0 (platform, 809 lines all ggml-hexagon/) / `2f2763d` opencl MxFP4 MoE Adreno + `27c7a76` hexagon softplus (staged 18:2x, platform watch).

**NULL:** `c0fd7862` UI HF hub / uzu `66719623` Asyncify part 2 (#976) / uzu #860 Benchmarks / `10b8b755` test infra / `9d1e0361` -Wtautological-compare / `1268eda` Granite options / `ee5ec570` map_coords DSL.

Pins: llama.cpp staged at `62318cd`, re-verified at tip `797de63b` (all candidates ancestors) / prism `0082ac24a` (origin/prism unmoved) / uzu `5baa58b7` -> `66719623` (+1 NULL).

## Slices

**Slice 1** (this composition): the 2 riders + uzu `2c16e967` (kernel_opt D-category, NotBenchable-here) into `entries_gpu_103` + llama.cpp `3dbcb7f` (rust_perf entries_34, benchable — x86 Pareto).

**Slice 2** (next session, same claim): the remaining 6 kernel_opt own-rules into `entries_gpu_104` + the 4 watch disclosures.

## Tasks

### Slice 1

- [x] T1 Post-B249 re-fetch: plan 253 COMPLETE (slices 2+3 landed, corpus 878); slice numbers shifted to gpu_103/104; dual_allocation green.
- [x] T2 This plan file + `.plans/.highwater` 253->254 (same commit).
- [x] T3 Rider 1: the k-pool shared-view hoist face on B233 (`entries_gpu_92.rs` gpu_concept) — "RIDER (Batch 250 slice 1 / Plan 254" section, verbatim quotes, count-neutral marker.
- [x] T4 Rider 2: the graph-level signed-FWHT fusion + fallback-fix face on B166 (`entries_gpu_45.rs` gpu_concept) — same rider convention.
- [x] T5 kernel_opt `entries_gpu_103.rs`: uzu `2c16e967` as `attention-route-retirement-by-measurement` (category D; ref/opt Rust-shaped pseudocode parsing under syn).
- [-] T6 rust_perf `entries_34.rs` — STAGED, fixture committed; the Pareto measurement is x86_64-host work, deferred to the next window (the M3 is aarch64 + at composition time the box sat at load 20-50 under sibling training lanes; the context_notes carry the honest BENCH-VERDICT-PENDING-x86 token, the standing annotation gate accepted it at default posture) llama.cpp `3dbcb7f` as `avx2-tiled-accumulator-butterfly-epilogue` (category simd; BenchSpec harness; the Pareto measurement is x86_64-only — M3 bench refused BY DESIGN (E0308/AVX2 intrinsics don't compile on aarch64), run on the 4090 box per the B204 staging precedent, 3 runs + box state, or land NotBenchable-pending-x86 with the fixture committed).
- [x] T7 Standing gates (written + default-posture green 2695/0; the kernel_opt,latent_retrieval combo compile + retrieval floors ride T8's window) `batch250_slice1_riders_landed` + `batch250_slice1_rule_landed` (kernel_opt tests.rs) + `batch250_slice1_pseudocode_parses`/`rule_present` (rust_perf tests.rs); corpus counts asserted (kernel_opt 878->879, rust_perf 231->232).
- [x] T8 GOAT: lib tests (default + kernel_opt,latent_retrieval + rust_perf postures); clippy x2 postures; retrieval floors re-measure (~1 min); score-bench run (corpus delta owed; freeze tables NOT owed — zero clippy-corpus delta).
- [x] T9 Research 285 (slice 1, born-complete; 282/283 by B249 slices 2/3, 284 taken by the Yukon staged enumeration — the 4090 lane landed it mid-claim, citations renumbered in the same landing); AGENTS count lines; `.docs/11_domains/kernel_opt.md` + rust_perf doc rows.
- [x] T10 `.distill` landing row (CRLF bytes!); plan ticked; maintained binary rebuilt at the release set BEFORE push (build: == HEAD); commit `feat: batch 250 slice 1 ...` + push.

### Slice 2

- [ ] T11 The 6 remaining kernel_opt own-rules into `entries_gpu_104.rs` (+ mod registration) + the 4 watch rows disclosed in Research 286.
- [ ] T12 Slice-2 gates + counts + docs + binary rebuild + push.

## Non-goals

- No consumer-repo edits (corpus batch).
- No kernel_opt/rust_perf promotion to default (standing GOAT gates pending).
- B249 is COMPLETE (slices 1-3, plan 253 closed) — no TensorFold scope left; this batch is the envelope pool only.
- The CRLF law: `.distill/001_mining_queue_snapshot.md` is CRLF — edit via bytes, never text-mode Python.
