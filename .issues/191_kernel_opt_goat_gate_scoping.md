# Issue 191 — scope the `kernel_opt` GOAT gate: real corpus + full numeric validator + promotion bars (the pending default-promotion precondition)

**Status:** OPEN — scoping only; no campaign runs until the bars here are ratified and a session claims the campaign. Delegated-owner verdict 2026-11-01 (fallback reviewer sub-agent, the request_verdict claude_code channel auth-unavailable on that box): FILE, don't execute — the prefill-vs-llama.cpp lane stays the 4090's top priority, and kernel_opt is a heal-domain gate, not a league lane; PARK would leave a 1051-rule domain with a permanently unactionable promotion path.

## The question

`kernel_opt` stands at 1051 rules (Batch 323, entries_gpu_147) with fixtures that are
standalone-compiled AND executed before entry, 15 category experts + a thermal router — and is
NOT promoted to default because "its GOAT gate (real corpus + full numeric validator) is
pending". Nothing defines that gate. This issue scopes it so the pending promotion has a
pre-registered, measurable path (pin the numbers first, measure later — the rust_perf_bench
precedent shape).

## What exists (the substrate this gate composes — consume, never fork)

- `rust_perf_bench` (opt-in): the PROMOTION GATE PRECEDENT — generated microbenchmarks with a
  deterministic alloc-count axis; Pareto + alloc-count validator; GOAT PASS on record.
- `kernel_verify` (opt-in): the compile-check oracle (STUB — delegates to the shared
  `RustAnalyzerPruner`).
- The kernel_opt fixtures: standalone-compiled AND executed before entry (every rule).
- The GOAT gate axes (workspace Feature-Flag Discipline): G1 correctness, G2 perf (`--release`
  mandatory), G3 no-regression, G4 alloc-free or equivalent; promotion to default ALSO requires
  the gain to be modelless (deterministic construction — kernel rewrites are; no training).

## Pre-registered bars (ratify or amend BEFORE any campaign — never after a measurement)

1. **G1 — the full numeric validator.** Today's fixtures prove their transforms run; a promoted
   rule must prove its transforms are EQUIVALENT on real inputs. Definition of "full":
   - a REAL-GPU-CORPUS harness: ≥ N real kernel-shaped inputs per rule family (N ≥ 32 per
     family; sources: the vendor trees already mined — TensorFold / vllm / PrismML-Eng /
     llama.cpp fixture kernels, and our own riir-infer/riir-ai GPU kernel test inputs),
   - bit-identity where the corpus law demands it (Issue-940-style reductions: sequential fold
     order pinned; the NaN-visibility law — author "propagates", never "refuses"), and
   - a documented tolerance class ONLY where the mined source itself is approximate (the source
     PR names the class; the fixture carries it; never a post-hoc band).
2. **G2 — the perf bar.** Per-rule microbench median improvement ≥ 5% on the rule's own shape
   family at `--release`, measured per GPU generation the rule admits (the
   simdgroup-fragment law's per-generation admission is the precedent — an M3-class regression
   is a deliberate else-branch, not a pass).
3. **G3 — no-regression.** The clippy_lints + rust_perf default surfaces' score-bench verdicts
   are byte-stable with kernel_opt enabled (the domain only ADDS a fan-out lane; the router
   must not perturb the existing domains' selections).
4. **G4 — alloc-free.** The default build (kernel_opt OFF) stays byte-identical — the gate is
   compile-time-only cost; the ON-posture alloc axis rides the per-case fix path's existing
   counters (the rust_perf_bench deterministic alloc-count validator, reused).
5. **Promotion shape.** If G1–G4 pass: promote `kernel_opt` to `default` in ONE commit with the
   bench record + the GOAT table; if any gate fails, the domain stays opt-in with the failing
   axis named in the record (the G2-negative precedent: `RIIR_GDN_CORE_FUSED` — direction-
   positive without a passing gate stays opt-in).

## Tasks

- [ ] T1 — ratify/amend the bars above (owner-verdict ping-pong; the numbers are the contract).
- [ ] T2 — build the real-corpus harness (T1.1's source list → fixture kernels per family;
      blake3-pinned inputs; the harness runs per GPU generation with the box-state PROVENANCE
      line — the Issue-021 law).
- [ ] T3 — wire the full numeric validator into `kernel_verify` (replace the STUB delegation
      path for the promoted set only; the stub stays for the long tail).
- [ ] T4 — the G3 score-bench byte-stability arm (reuse the existing score-bench freeze-table
      machinery; plan 137 T7.5's regeneration law applies to any corpus-touching landing).
- [ ] T5 — run the campaign on a claimed slot (claim-lease discipline), record under a fresh
      `.benchmarks/` number, and take the promotion verdict to the owner with the full table.

## Refs

- The delegated-owner FILE verdict: 2026-11-01 session zed-shikuwa-t52-seatwire (fallback
  reviewer sub-agent; verdict text in the session record).
- Precedent gates: `rust_perf_bench` (the promotion-gate shape), Bench 045/046/056 (the domain
  GOATs on record), `RIIR_GDN_CORE_FUSED` (the stays-opt-in precedent).
- Corpus state: riir-refine AGENTS.md §Role (1051 rules at Batch 323; per-batch history in
  HISTORY.md).
