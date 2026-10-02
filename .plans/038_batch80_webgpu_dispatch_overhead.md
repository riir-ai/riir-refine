# Plan 038 — Batch 80: WebGPU dispatch-overhead characterization distill (kernel_opt)

**Status:** COMPLETE (all tasks done 2026-08-27 — batch landed, see AGENTS.md)

Source: Jędrzej Maczan, "Characterizing WebGPU Dispatch Overhead for LLM
Inference Across Four GPU Vendors, Three Backends, and Three Browsers"
(arXiv:2604.02344v1, cs.LG/cs.DC/cs.PF, submitted 2026-02-09; open-source
code + raw data). Paper distillation (the Research-076 arxiv intake channel;
the B70 precedent — 3 rules from one measured paper). Routed **kernel_opt**:
the substrate is OURS (wgpu/CubeCL host orchestration — B26/B58 territory,
cross-validated by our own Bench 761 dispatch decomposition).

## Pool provenance (queue item 11, fresh online research)

Hunting grounds per the session directive — Khronos / LLVM OPT remarks /
GPU kernel literature:
- **frguthmann compute-shaders post** — ALREADY MINED (Batch 72,
  `skip-unchanged-writes-via-dirty-encoding`). No re-walk.
- **NVIDIA thread-group ID swizzling (L2 locality)** — ALREADY COVERED
  (`l2-grouped-tile-schedule`, popcorn corpus; verified by grep). Drop.
- **LLVM opt-remarks search lane** — tooling documentation (how to READ
  remarks), not pattern content. Drop (no bounded fix-space).
- **arXiv:2608.13696 "Instruction Wall"** — analysis-level recommendations
  (cache efficiency / occupancy+ILP / fewer instructions per access) all map
  to existing corpus ground (A/E category rules). Drop with reason.
- **arXiv:2604.02344 WebGPU dispatch overhead** — YES B+. Measured,
  cross-validated (4 vendors, Dawn + wgpu-native, 3 browsers), honest null
  results, decision-dense. THE source.

## Rules (3, corpus 384 → 387)

1. `dispatch-cost-sequential-measurement-not-single-op` (D 78→79) — a single
   isolated dispatch timed wall-clock CONFLATES the submit→sync round trip
   with enqueue cost: Dawn 496.8 µs single-op vs 23.8 µs sequential (~20×);
   Chrome 2071–3123 µs vs 32.8–66.5 (~60×); submit alone is 40% of the true
   per-dispatch cost (12.9 of 32.5 µs). Fix: N back-to-back dispatches, ONE
   tail sync, divide by N. Cross-validated in-tree: Bench 761's wgpu M3
   decode decomposition measured the same submit-dominant shape (~31 µs
   encoder-finish+submit+cleanup each, 577 submits/token).
2. `fusion-payback-gated-on-per-dispatch-overhead` (C 35→36) — the SAME
   structural fusion's payoff scales with (dispatches saved × per-dispatch
   overhead) and FLIPS across backends: RMSNorm 6→1 = 1.41× wgpu/Vulkan
   (24–36 µs/dispatch), 0.95× wgpu/Metal + 0.91× Safari/Metal (kernel-level
   regression dominates despite 31–71 µs dispatch cost), 0.92× CUDA
   (7.4 µs/dispatch — nothing to save). The batch crossover
   B* = T_overhead·throughput/(2·din·dout) marks the overhead-bound regime
   (batch=1 LLM matmuls: B* ≥ 7). Gate the fusion decision on the MEASURED
   sequential per-dispatch cost (rule 1), never dispatch count alone.
3. `grid-collapsing-mega-kernel-occupancy-trap` (E 22→23) — fusion that
   needs cross-workgroup sync (WebGPU has NONE — workgroupBarrier is
   intra-workgroup only) collapses the grid to ONE workgroup; at production
   scale a single workgroup serializes (~17K sequential MADs/thread vs
   65K+ threads available). The tiled middle ground (7→3 dispatches,
   parallelism preserved) is the scale-safe fix: 2.01× Metal / 1.17× Vulkan
   (both significant); mega vs multi INCONCLUSIVE at toy scale (p>0.38) and
   analytically worse at production — the honest regime split is the rule's
   content (mega CAN win at sizes one workgroup fills).

## Tasks

- [x] T0 Pool + dedup (pre-plan greps: no sequential-measurement rule, no
      fusion-payback rule, no mega-kernel/grid-collapse rule;
      `l2-grouped-tile-schedule` confirmed as the swizzle dedup kill;
      nearest neighbors named in-rule: merge-dispatches-into-single-compute-pass,
      kernelize-host-chain-after-sync-batching-floor,
      epilogue-must-be-workgroup-local, fold-query-tokens-for-occupancy)
- [x] T1 Author 3 entries in `entries_gpu_07.rs` (via `target/gen_b80.py`,
      the B77 rustlit()-escaped python path; opener-sanity check fixed to
      any-indent — 6 historical entries carry 16-space `KernelRule {`)
- [x] T2 KERNEL_CATEGORIES rows in `mod.rs` (C/D/E +1 each) + per-category
      count comments + header `387-rule` + Batch 80 header note
- [x] T3 Standing gates in `tests.rs`: batch80_rules_present +
      batch80_concepts_covered + batch80_pseudocode_parses (with the
      `#[cfg(not(feature = "kernel_verify"))]` guard)
- [x] T4 Count-pin walk — entries.len() 387, counts_match table (C 36,
      D 79, E 23), 2 sum sites, grouped-total, router.rs total + doc header,
      exhaustive-test comment, current-NNN snapshot comment; **3 STALE pins
      caught**: router.rs doc-header "379-rule" (pre-B79 stale), tests.rs
      exhaustive comment "379-rule", the "current 370" snapshot comment
      (2 batches stale) — all fixed to 387
- [x] T5 GOAT: G1 syn-parse (3/3 first run) ✓; G2 kernel_opt lib
      **926/0/1i** (exactly 923+3) + default **703/0** ✓; G3 clippy
      `--all-targets` **0 both states** (2 doc-markdown backtick fixes
      pre-commit: `RMSNorm` + `dispatches_saved` in the mod.rs `//!` note)
      + CI guard layers 1–4 + 5a (`-D warnings` both extremes) + 5b-default
      + 5c (**8/8 KAT binaries, 75 tests**) PASS; 5b-all-features =
      **1346/4/13i** (the 4 = known environmental l4_daemon Windows-spawn
      failures, identical B79 signature; count-consistent 1344+2 — the
      parses gate is cfg(not(kernel_verify))-compiled-out under
      all-features); retrieval floors **HELD zero decay**: kernel
      self-variants 100/100/100 (9th consecutive), blended 131
      93.9/96.9/97.7 identical to B79, 14/14 retrieval tests ✓
- [x] T6 Research note `.research/094_batch80_webgpu_dispatch_overhead.md`
      (Status: RECORD)
- [x] T7 Docs: kernel_opt.md Total line 387 (the B79 doc precedent),
      AGENTS.md batch note, queue snapshot entry
- [x] T8 Commit `feat: batch 80 ...` + push origin/develop; /tmp target dir
      removed after (Plan-337 discipline; box: trainer pids 71336/47448
      resident, CPU 43%/24c, RAM 5.4–6 GB free — the B79 free-box precedent
      followed)

## Non-goals / defers

- [-] Proposers: none this batch — all 3 are host-orchestration/measurement
      decision rules (suggestion-only, the B70 precedent: paper-distilled
      dispatch rules ship without proposers).
- [-] rust_perf side: untouched (the paper's WebGPU-specific findings have
      no CPU-side fix-space).
