# Plan 376 — Batch 362: the ANE conversion/host-placement family (liquidcodec-ane + beshkenadze corroboration)

**Status:** OPEN — filed 2026-11-14 from Research 409 (verdict YES B− / lane-intel HIGH). Nominal Batch 362. Deep-dive + lane execution: riir-infer Research 029 / Plan 633 (the measured cells this batch's fixtures cite).

## Pre-batch gates

- [ ] T0 — dedup re-verification at batch time (the verdict's greps re-run against the live corpus; R1–R6 re-checked for new owners landed since 2026-11-14).
- [ ] T1 — source re-pin: re-clone `shershah1024/liquidcodec-ane` (or fetch the pinned raw files) at `177f90246605f60d10df0372396acc3f5222d597`; re-verify every quote used in an entry (the young-repo drift law); re-read the two UNREAD files (`convert/export_tail.py`, `examples/swift/main.swift`) if R4/R5 entries need them.

## Entry authoring (each: decision content + regime conditions + fixture + falsification arm, the 4-arm discipline)

- [ ] T2 — R1 `ane-fixed-window-mask-stateless-streaming` (the B+ lead): kinship cites `pad-tail-to-fixed-tile` (gpu_08) in-rule; fixture = a Rust streaming host choosing window-vs-state by the receptive-field bound, with the designed negative-control arm (chunks without left context MUST fail the equivalence check).
- [ ] T3 — R2 `ane-fp16-norm-pow2-prescale-overflow`: fixture = fp16 norm overflow vs the pow2-prescale form (residual ~2,300 → x² overflow → prescale caps scaled max at 32); kinship cites the B242/hi-lo prescale family (subnormal-floor pole) both directions.
- [ ] T4 — R3 `ane-compute-plan-pin-cpu-and-neural-engine`: fixture = the `.all`-vs-`cpuAndNeuralEngine` placement decision (host config enum; the measured GPU-trap quote in-rule).
- [ ] T5 — R4 `ane-graph-restructure-for-ane-lowering`: fixture = fold-projections + linear-as-1×1-conv graph construction; the int4 386/464 fallback row as the regime negative.
- [ ] T6 — R5 `ane-precision-dsp-stays-cpu-fp32` + R6 `ane-weight-palette-depth-retention-gate`: author or fold — R5 may fold as a face onto R4's host (the CPU-tail placement decision) if the standalone fix-space reads thin at authoring time; R6 carries the FluidAudio-6-bit discrepancy as its regime condition IN-rule.
- [ ] T7 — all suggestion-only, all `NotBenchable`/`bench_spec: None` annotations per the dial9 split; NO exit-code or perf claims.

## Standing gates + landing

- [ ] T8 — fixtures standalone-compiled AND executed pre-entry (`corpus/fixtures/b3XX_fixtures.rs` convention); retrieval floors checked both lanes; `cargo refine --score-bench` after landing.
- [ ] T9 — count pins via `scripts/pins_manifest.py --write` ONLY with the corpus change in the same commit (never run-to-pass).
- [ ] T10 — records: Research 409 status flipped DISTILLED→LANDED (or the honest per-rule outcome); queue snapshot line finalized; riir-infer Plan 633 P4.2 ticked when fixtures cite the measured cells; commit+push `feat:` on develop.
