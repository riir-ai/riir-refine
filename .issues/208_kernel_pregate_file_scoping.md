# Issue 208 — the kernel pregate: detect kernel-bearing files BEFORE the kernel matcher fans out, then scope the fan-out to the indexed/mapped files only (the G5-cost clearing lane)

**Status:** OPEN — Bench-129's named structural lever (b) ("a cheaper first-pass scan key"), owner-directed 2026-11-05. The G5-cost fence (added p50 ≤ 1.0 / p95 ≤ 5.0 ms per file, never re-registered per the 2026-11-04 D2 verdict) fails at 33×–39× because the flat 1053-rule kernel matcher queries EVERY span of EVERY file in the scan walk — 483/594 files pay > 1 ms added to learn they hold no kernel code. This lane gates the kernel domain's fan-out behind a cheap, recall-biased kernel-signal detector (file-level), then scopes within signaled files (span-level). Disjoint from Issue 191 T6 (the GPU-host G2 leg, leased zed-m3-i191t6) — this is the T7-side cost axis.
**Claimed:** zed-m3-i208pregate, 1793883835, expires 2026-11-07T13:03:55+00:00

## Premise (from the measured record, not new claims)

- Bench 125/129 measured the added cost over the MIXED walk `src/**/*.rs` (565→594 files): flat added p50 33–39 ms/file. The corpus is not kernel-only — the median file is ordinary Rust the kernel domain can never help.
- Bench 129 §"Where the remaining cost lives": per span, 1 + significant-lines queries × (query_for + O(N=1053) scan + BM25 + rank merge). The cost is query MASS, not allocation — the scratch-reuse twin (T7, −9/−15%) proved allocation reuse cannot reach the bar.
- Therefore the lever is INPUT scoping: files with zero kernel signals skip the kernel matcher entirely (~detector cost only); signaled files keep full matcher fidelity (utility preserved by construction on those files).

## The design (pinned before implementation)

1. **Detector (`kernel_signal`, file-level, recall-biased).** `.wgsl` → always signal. `.rs`: case-insensitive substring over the file source (the buffer the scan already reads — no extra I/O) for the pinned needle set: `cubecl`, `wgpu`, `@compute`, `wgsl`, `workgroup`, `threadgroup`, `compute_pass`, `#[cube]`, `#[kernel]`, `dispatch_compute`, `computepipeline`, `kernel`. Detected ONCE per file per run and cached (the run-level index). Recall-biased BY DESIGN: a false positive costs only the old matcher time on one file; a false negative HIDES a matching span (utility loss — the bench's kill criterion).
2. **Gate placement (file-level arm).** The kernel domain joins a file's matcher fan-out only when `kernel_signal(file)` is true. Explicit `--domain kernel` BYPASSES the gate (the operator asked for the domain). All other domains unaffected.
3. **Span scope (arm 2).** Within a signaled file, the kernel matcher queries only spans whose line window (± 24 lines, pinned) intersects a signal line; other spans in the file skip the kernel matcher. Exists for the p95 leg: the walk's top-5% files are kernel-corpus/fixture files (~14% of the walk by the Bench-129-era file set), so a file-level gate alone is EXPECTED to pass p50 but leave p95 on a signaled file — disclosed as an expectation, not a claim; the bench decides.
4. **Feature gate:** `kernel_pregate` (implies `kernel_opt`), DEFAULT-OFF; promotion per GOAT discipline after Bench 131 + verdict.
5. **Detector cost counts** in every ON arm's added-cost diff (it is part of the ON posture).

## Tasks

- [x] T1 `kernel_signal` detector module + tests (needle set pinned; .wgsl always-signal; cache-per-run) — `src/kernel_pregate.rs` (always compiled, inert without the feature; unit tests run at default features)
- [x] T2 file-level gate wiring in the scan fan-out behind `kernel_pregate` (+ the `--domain kernel` bypass) + gate tests — `suggest_file_inner` resolves `KernelPlan::for_file` once per file; the denied shape is the kernel-disabled `by_domain` shape; wiring tests in `latent_matcher.rs::kernel_pregate_tests`
- [x] T3 span-scope arm (± 24-line window) + tests — the GATE+SCOPE arm rides the promote-only `RIIR_KERNEL_PREGATE_SCOPE` env (same binary as GATE: only TWO bench builds needed); scope boundaries unit-tested
- [ ] T4 run Bench 131's pre-registered A/B (OFF / GATE / GATE+SCOPE) on the registered corpus, record the run bench
- [ ] T5 verdict: if an arm clears BOTH cost legs AND the utility floor holds, record it as the promotion-candidate posture for the Issue-191 re-campaign (T6 ∧ T7 → G1+G3+G4+G5); file the rulewright-queue finding if the utility floor drops (detector misses are the finding, never a needle-silent retry)
- [-] Per-span line-query CAP (Bench-129 lever (a)) — the named follow-up if GATE+SCOPE cannot reach the p95 bar; a separate design change, its own pre-registration
