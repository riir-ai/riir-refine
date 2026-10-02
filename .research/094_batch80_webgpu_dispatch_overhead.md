# Research 094 — Batch 80: the WebGPU dispatch-overhead characterization distill (arXiv:2604.02344)

**Status:** RECORD (batch landed same day — corpus 384 → 387, commit see AGENTS.md). **1.98.1 toolchain re-gate 2026-09-05 (M3, loaded box): `runtime-feature-detect-per-chunk` verdict class UNCHANGED (Mixed — was 0/0/3 all-ties on 1.97 aarch64, now 1/0/2 with a mild 1.06× @4096); the std::detect const-fold note stands, no annotation flip.**

**Source:** Jędrzej Maczan, *Characterizing WebGPU Dispatch Overhead for LLM
Inference Across Four GPU Vendors, Three Backends, and Three Browsers*
(arXiv:2604.02344v1 [cs.LG], submitted 2026-02-09; open-source code + raw
data at github.com/jmaczan/torch-webgpu). Measured across NVIDIA/AMD/Apple/
Intel GPUs, Dawn + wgpu-native, Chrome/Safari/Firefox, Linux/Windows/macOS.
Verdict: **YES B+** for kernel_opt — the substrate is OURS (wgpu/CubeCL host
orchestration, the B26/B58 territory), the numbers are measured +
cross-validated, and the paper is honest about null results (mega-kernel
p<0.38, device-side argmax inconclusive, Firefox rate-limiting mechanism
unverified).

## Pool provenance (queue item 11 — the fresh online research the session directed)

Hunting grounds evaluated before settling on this source:

| Candidate | Verdict | Reason |
|---|---|---|
| frguthmann "Exploring ways to optimize compute shaders" | SKIP — consumed | Already Batch 72 (`skip-unchanged-writes-via-dirty-encoding`); no re-walk |
| NVIDIA thread-group ID swizzling (L2 locality) | DROP — covered | `l2-grouped-tile-schedule` (popcorn corpus) IS the mechanism; grep-verified |
| LLVM opt-remarks lane | DROP — tooling | Remarks documentation is how-to-READ tooling, not pattern content; no bounded fix-space |
| arXiv:2608.13696 "Instruction Wall" (ETH) | DROP — analysis-level | Its 3 recommendations (cache efficiency / occupancy+ILP / fewer instr per access) all map to existing corpus ground (A/E categories) |
| **arXiv:2604.02344 (this batch)** | **YES B+** | Measured, cross-impl, decision-dense, our exact substrate |

## The 3 rules (B1-B3)

### B1 `dispatch-cost-sequential-measurement-not-single-op` (D 78→79)

The paper's primary methodological contribution: **naive single-operation
dispatch benchmarks overestimate per-dispatch cost 10-60×**. Dawn: 496.8 µs
single-op vs 23.8 µs sequential; Chrome: 2071-3123 µs vs 32.8-66.5 µs. The
mechanism: WebGPU queue submission is async, so one isolated op's wall clock
measures the submit → GPU-execute → completion-drain round trip (plus
first-op validation), NOT the enqueue cost a pipelined producer pays.
Protocol: warm up, N back-to-back dispatches, ONE tail sync, divide by N.

The steady-state breakdown (wgpu/Vulkan, 100 dispatches, Table 20): submit
12.9 µs = 40% of the 32.5 µs total; encoder create 6.4; encoder finish 6.1;
pass begin 3.2; set pipeline 1.4; set bind group 1.0; dispatch call 0.6;
pass end 0.7; GPU sync 0.5. **Submit dominates → submit-count reduction is
the lever** (batch dispatches per submit), not per-dispatch-call micro-cost.

In-tree cross-validation: riir-ai Bench 761 (the M3 wgpu decode dispatch
decomposition, Issue 761) independently measured ~31 µs per
encoder-finish + submit + memory_cleanup cycle at 577 submits/token — the
same submit-dominant shape by an entirely different method (always-on
counters in vendored cubecl-wgpu vs the paper's CPU-side profiler). The
paper's numbers also give the B761 work an external anchor: their
wgpu/Metal sequential = 71.1 µs vs our ~31 µs/cycle measurement bands.

### B2 `fusion-payback-gated-on-per-dispatch-overhead` (C 35→36)

The fusion economics, causal evidence: 312 fewer dispatches with the SAME
kernels = +53% end-to-end (13.5 → 21.0 tok/s) on Dawn/Vulkan. But the same
RMSNorm 6→1 fusion measures:
- 1.41× wgpu/Vulkan (24-36 µs/dispatch)
- 0.95× wgpu/Metal, 0.91× Safari/Metal (kernel-level regression dominates
  despite 31-71 µs dispatch cost — fused-kernel penalty is backend-specific)
- 0.92× CUDA (7.4 µs/dispatch — nothing to save)

Decision content: payback = dispatches_saved × per_dispatch_overhead vs the
fused kernel's own penalty; the overhead input MUST come from the sequential
protocol (B1) or the decision flips. The batch crossover B* = T_overhead ×
throughput / (2·d_in·d_out) separates overhead-bound from compute-bound: at
batch=1 every LLM matmul sits at B* ≥ 7 — per-op overhead dominates
REGARDLESS of kernel quality there. Repetition ranks fusions: RMSNorm (240
dispatches/forward saved) ≫ K+V projection (24) — 10× payoff for similar
effort. K+V fusion honestly did not reach significance (p=0.42) — encoded.

### B3 `grid-collapsing-mega-kernel-occupancy-trap` (E 22→23)

WebGPU has NO cross-workgroup synchronization (workgroupBarrier is
intra-workgroup only), so a mega-kernel (whole region in one dispatch) runs
as a SINGLE workgroup. Toy scale (256×256): mega 1.48 ms < tiled 2.85 <
unfused 5.74 on Metal — but mega-vs-multi was honestly inconclusive
(p>0.38). Production scale: a 256-thread workgroup would do ~17,000
sequential MADs/thread vs 65,000+ threads for the grid form — analytically
a trap. The scale-safe middle ground: PARTIAL fusion preserving the grid —
7→3 dispatches measured **2.01× on Metal (p<0.001), 1.17× on Vulkan
(p<0.01)**. The regime split IS the rule: mega pays exactly when one
workgroup saturates the fused region; past that, tile.

## Drops (recorded, with reasons)

- **Firefox ~1040 µs rate-limiting** — environmental/browser-policy, no
  code fix-space; the paper itself did not verify the mechanism.
- **Device-side argmax** — inconclusive on both backends (p=0.35/0.62); our
  corpus already carries device-side-argmax-minimal-download (B25) with
  better evidence.
- **Command batching / buffer pooling / bind-group caching null results** —
  the paper's null is autoregressive-sync-specific (per-token argmax
  readback flushes batches); our B26 rules already carry the general forms
  with positive evidence. The null REGIME note (per-token sync negates
  intra-token batching) is folded into B2's regime content implicitly via
  the crossover.
- **torch-webgpu FX compiler / PrivateUse1 backend** — infrastructure, not
  kernel patterns.
- **Mega-kernel as a rule's positive form** — inconclusive; kept only as
  B3's honest regime half.

## Generator + process notes

- Entry generation via `target/gen_b80.py` (python with explicit
  `rustlit()` escaping — the B77-safe path; zero `"` characters in any
  pseudocode field by construction). The script's opener-sanity check
  initially mis-fired: 6 historical entries in `entries_gpu_07.rs` carry
  16-space `KernelRule {` indentation vs the dominant 8-space — the check
  now matches any indent. No file was written on the mis-fire (abort
  before write held).
- Count-pin walk: 387 at entries.len(), counts_match table (C 36, D 79,
  E 23), 2 sum sites, grouped-total, router.rs total + doc header (the
  header's "379-rule" was STALE from before B79 — a missed B79 pin, now
  387), the exhaustive-test comment (was "379-rule" — also stale, fixed),
  the "current NNN" snapshot comment (was 370 — 2 batches stale, fixed).
  Three stale pins caught = the standing "re-scan at write time" lesson
  keeps paying.
- Clippy doc-markdown struck again (the B37/B56 gotcha): `RMSNorm` +
  `dispatches_saved` in the mod.rs `//!` batch note needed backticks —
  2 warnings, fixed pre-commit.
- The B32c adjacent-string-literal lesson struck DURING authoring: the
  concepts_covered needle was drafted as two adjacent Rust string literals
  (invalid Rust) — caught on read-back before compile, fixed to one
  literal.

## GOAT (all gates green)

- **G1**: `batch80_pseudocode_parses` ✓ (all 3 entries' ref+opt syn-parse)
- **G2**: kernel_opt lib **926/0/1i** (exactly B79's 923 + 3); default lib
  **703/0** (unchanged — corpus is feature-gated)
- **G3**: clippy `--all-targets` **0 warnings both states**; CI guard
  layers 1-4 + 5a (`-D warnings` no-default + all-features) + 5b-default +
  5c (**8/8 KAT binaries, 75 tests**) ALL PASS; 5b-all-features = **1346/4/13i**
  where the 4 are the KNOWN l4_daemon Windows-spawn environmental failures
  (identical signature to B79's record, proven pre-existing there via
  stash; count-consistent: 1344 + 2 — the parses gate is
  cfg(not(kernel_verify))-compiled-out under all-features)
- **Retrieval floors HELD zero decay**: kernel self-variants
  **100/100/100** (9th consecutive batch); blended 131
  **93.9/96.9/97.7** — identical to B79's floors; 14/14 retrieval tests
- Box: trainer (Plan 341 L4 SFT, pids 71336/47448) resident throughout at
  CPU 43%/24 cores, RAM 5.4-6 GB free — the B79 free-box precedent
  followed (isolated CARGO_TARGET_DIR=/tmp/riir-clippy-b80, deleted after).

## Post-batch queue state

- kernel_opt corpus: **387 rules** (A 91, B 53, C 36, D 79, E 23, F 10,
  G 21, H 14, I 4, J 5, K 5, L 11, M 10, N 21, O 4).
- Remaining intake paths unchanged: measured-GOAT distillations +
  issue-driven + arxiv systems papers via the research skill; the
  named-crate + blog-article hunting grounds stay adjudicated/exhausted
  (this batch's Khronos/LLVM lane re-confirmed it — 2 dedup kills +
  1 tooling drop).
