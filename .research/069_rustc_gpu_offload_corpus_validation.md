# Research 069 — rustc GPU Offload (arXiv:2608.13759): External Validation of the kernel_opt Transfer-Rule Family

**Status:** DISTILLED — pending owner decision (the one unshipped pattern is filed as riir-ai Issue 725, a deferred watch item)

> Drehwald, Domínguez, Sala, Aspuru-Guzik, Doerfert — "GPU Offload in Rust: Portable, Safe, and Fast" [arXiv:2608.13759 "GPU Offload in Rust: Portable, Safe, and Fast"], LLNL + U Toronto + URJC, submitted 2026-08-13. cs.PL — a rustc/LLVM-native multi-vendor GPU offload toolchain (`-Z offload`, LLVM Offload/libomptarget infra, two-pass compilation). NOT an ML/training paper — no §3.5 Path 0 decomposition applies (no training loop anywhere); classified as a systems/compiler paper per the substrate-≠-value rule.

## TL;DR

The paper's §5 compiler-optimization semantics are the **same transfer-discipline family our kernel_opt corpus encodes as manual rules** (Batches 26–58) — arrived at independently from opposite directions (they build compiler passes; we mine production-kernel rules). One citable external number: naive per-launch auto-transfer is **>400× slower** than explicit management on MI250X loop benchmarks — the strongest published rationale for why the corpus transfer family exists. One genuinely unshipped pattern surfaced: **device residency encoded as a Rust borrow** (`Preload`/`PreloadMut` guards; Drop = the D2H sync point; intermediate host reads rejected by the *borrow checker*, not runtime discipline) — architectural, not proposer-shaped → riir-ai Issue 725 (deferred watch). **Mining yield: 0 proposer-shaped rules** — this paper is not a distill/batch candidate; the mapping below is the record that prevents re-evaluation.

## What the paper is

A prototype rustc + LLVM toolchain (LLVM 23.1.0-rc1, `-Z offload=Device` / `-Z offload=Host=...`) that compiles safe Rust kernels to native NVIDIA (nvptx64) + AMD (amdgcn) GPU code via the LLVM Offload infrastructure, with three host interfaces:

| Interface | What | Transfer semantics |
|---|---|---|
| A — compiler-managed | `offload!(kernel, &a, &b, &mut c)` | fully automatic per-launch H2D/kernel/D2H |
| B — vendor-library interop | `offload_args` around cuBLAS/rocBLAS calls | same auto-mapping machinery around opaque library launches |
| C — explicit type-staged | `Preload<'a,T>` / `PreloadMut<'a,T>` guards | residency encoded as a borrow; Drop = sync point |

Kernel-side safety via `Region<T, PartitioningStrategy>` — an `unsafe trait` whose implementations *promise* disjoint per-thread slice access, letting kernels be written over standard safe slices. RAJAPerf evaluation: kernel times at near-parity with hand-optimized CUDA/HIP C++ (Rust 32% faster to 43% slower whole-runtime on MI250X; 11% faster to 46% slower on H100); Rust moved LESS data (423 MB vs 468 MB H2D; 69 MB vs 99 MB D2H) but its transfers were SLOWER (46 ms vs 16 ms — they suspect memory kinds + missing async); register usage 33 vs 28 avg (bounds-checking, no measured runtime impact).

## The mapping: paper §5 ↔ corpus rules (mutual validation)

| Paper optimization (§5) | Corpus rule(s) | Batch | Verdict |
|---|---|---|---|
| Split offload into H2D / launch / D2H so each is optimizable | ambient in the whole transfer family; `batched-downloads-single-sync-cycle` | B26 | covered |
| Async prefetch: H2D started early if legal, awaited at launch; D2H started after kernel, awaited at next use | `defer-readback-until-after-submit` (timing), `independent-cpu-work-fills-dispatch-read-window` (fill the gap), `async-htod-host-source-outlives-copy` (async H2D lifetime) | B26/B36/B58 | covered |
| LICM: hoist transfers out of kernel-call loops | `host-cache-immutable-device-weights` (upload once at init), `immutable-operand-upload-at-set-creation` | B26/B33 | covered |
| Transparent lookup so callers keep CPU-ref APIs while handles stay resident | `weight-handle-cache-keyed-by-allocation-identity` (pointer-identity keyed, miss-path degrades to on-the-fly upload) | B29 | covered — a stronger form (their `Preload` requires explicit construction; ours is API-invisible) |
| Cancel intermediate transfers between back-to-back kernels on the same buffers | `sibling-dispatches-before-any-readback`, `same-input-gemv-segments-single-launch` (same-input dedup), `staging-reuse-grow-only-map-prefix` | B36/B58/B31 | covered |
| Their 46 ms vs 16 ms transfer-time regression (less data, slower) — root-caused to sync mechanics | `sync-map-result-stack-cell` (per-sync plumbing), `staging-reuse-grow-only-map-prefix` (driver alloc amortization), the whole per-sync cost family | B31 | covered — the corpus literally contains the mechanics they suspect |

## Where the corpus is AHEAD of the paper

§5 "Further experiments": they did **not** prototype the preload-all heuristic for the shared-argument Energy benchmark because "the transformation would increase the peak memory consumption on the GPU, potentially significantly" — an ungated all-or-nothing concern. The corpus ships the **measured failure boundary** instead: `defer-readback-gated-by-live-output-size` (B29, Bench 437 — keeping 5 concurrent ~70 KB outputs live measured **16% slower** on Metal; ~8 KB fan-outs chain freely). Their deferred heuristic needs exactly this gate.

## The gap: residency-as-borrow (`Preload`/`PreloadMut`) — not shipped anywhere in the stack

The one pattern with no corpus or riir-gpu analog. Corpus transfer rules are **runtime discipline** (when to read back, what to cache); the paper's Interface C makes the discipline **structural**: `PreloadMut<'a, T>` holds the mutable borrow of the host buffer for the device copy's lifetime, so (a) Drop *is* the D2H sync point, (b) an intermediate host read between kernels is a **compile error** rather than the silent-transfer hazard the paper documents for Interface A ("data movement depend on seemingly unrelated host-side uses such as logging or debugging"). Grep confirms zero coverage: no `Preload`-shaped guard in riir-gpu (`preload_kv_cache` is KV-cache preloading, unrelated); the closest runtime cousins are the B26/B29 timing rules.

**Why it is NOT a corpus rule:** the corpus requires proposer-shaped `ref_shape`/`opt_pattern` pairs (local transforms the healer can apply); a borrow-guard is an architectural API pattern. **Why it is NOT a riir-gpu plan now:** the discipline it enforces is already manually applied (the corpus rules ARE the enforcement, in code review + healing); the paper's 400× number measures naive-vs-explicit, not disciplined-vs-guarded — expected perf gain ≈ 0; the value is structural safety, which has no GOAT gate shape. Filed as riir-ai Issue 725 (deferred watch, reopen triggers listed there).

## Prior-art verdicts (web search, 2026-08-18)

| Technique | Prior art | Verdict |
|---|---|---|
| T1 — transfer directionality from reference mutability (`&T`→MapTo, `&mut T`→MapToFrom) | SYCL 1.2.1 access modes (API-declared, same trichotomy); Emu (~2019, proc-macro auto-transfer); Rust-CUDA issue #124 (2021, unimplemented "model GPU data as a borrow" proposal) | modest novelty — clean compiler encoding of a known API idea |
| T2 — residency-as-borrow (Drop = sync, borrow-checked host reads) | **none found** — SYCL buffer/accessor is the RAII-shaped spec-level cousin; no paper/project composes mutable-borrow + Drop-sync + compile-time read rejection | the paper's strongest novel claim, and the one thing we lack |
| T3 — §5 transfer optimization passes | dense: Guo et al. 2024 (J. Supercomputing, redundant-transfer elimination in LLVM OpenMP target), Marzen et al. 2024 (static offload data mappings), TACO 2023 (mapping-free UM offloading) | weakest novelty — a re-combination of established passes; also the half our corpus already covers |

Nearest 2026 sibling worth knowing: cuTile Rust [arXiv:2606.15991 "Fearless Concurrency on the GPU"] — ownership *inside* kernels (orthogonal axis: kernel-internal safety vs host↔device transfer discipline).

## Strategic watch item (riir-ai)

The toolchain is being championed inside the Rust Language Team (acknowledgements name the champion + Rust Foundation support; targets nvptx64/amdgcn with Intel in progress). If `-Z offload` stabilizes upstream, the long-term implication for riir-gpu is a migration path **off the dual CubeCL + cudarc + WGSL stack toward safe in-tree Rust kernels** — same vendor coverage, one language. Early (prototype flag, LLVM 23-rc, std-on-GPU unsolved, transfer overheads still losing to RAJA). Watch, don't act. Triggers recorded in Issue 725.

## Fusion idea (novelty TBD — not Super-GOAT)

Paper × `weight-handle-cache-keyed-by-allocation-identity` (B29): a borrow-guarded residency handle whose *key* is the host allocation identity — combines Interface C's structural sync point with our API-invisible transparent lookup, i.e. `DeviceResident<'a, T>` that callers obtain without changing call signatures and that cannot outlive the host buffer. Both halves exist separately (theirs: explicit construction + lifetime; ours: invisible lookup + no lifetime binding); the composition is unexplored. Deliberately NOT scored against the 4-question gate (fails Q2/Q3 outright — no new behavior class, no product selling point; engineering-safety item, not a game capability). Tracked only via Issue 725.
