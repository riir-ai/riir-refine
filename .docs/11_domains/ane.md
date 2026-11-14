# `ane` — the Apple Neural Engine host-orchestration domain (Plan 377)

> Eighth retrieval `HealDomain` (`DomainId::Ane`, `DomainMask` u8 bit 7 — a
> const guard pins the 8-domain ceiling; `ALL` is a const fold, the u32-cast
> spelling refused by `cast_possible_truncation`). Scope: **CoreML/ANE
> placement, streaming, fp16-numerics, graph-lowering, DSP-edge and
> palette-retention decisions** — host-orchestration vocabulary, not kernel
> math (the ANE-family rules already in `kernel_opt` did not move; future ANE
> distills route here). Opt-in (`ane`, default-off, NOT in the release set);
> **zero new deps**; the win metric lives in the CoreML/ANE runtime, so the
> domain is `NotBenchable-by-domain` — disclosed in every rule's notes (the
> domain-unbenchable annotation class).

## Landing (2026-11-14, `486978a2`)

The owner's two-pass ask: "code scan for ane first pass before apply second
pass like kernel" — the `kernel_pregate` shape applied to the CoreML/ANE
surface. Seed corpus = the six Research 409 candidates (the liquidcodec-ane
distill, R1 fixed-window-mask-stateless-streaming B+ two-source lead +
R2–R6), **suggestion-only, no proposers** (the fix lane declines), **parse-only
validator**. Surfaced in `--info`'s corpus row (🧠 ane) + the pins manifest
(`ane_rules: 6`) + the eval fixtures + router-eval + demonstrability
`T3_DOMAINS[9]`.

## The pregate (`ane_pregate` = ["ane"])

ONE recall-biased needle scan per file — 10 pinned CoreML/ANE needles + the
`neural engine` prose needle + the boundary word `ane` (letter-boundary-checked
so `lane`/`plane` never signal) + the path-word signal (any path component
word `== "ane"` → the whole file is an ANE surface) — then the domain joins
the fan-out only on **signal-line ± 24 spans**. `span_allow` widened to two
gates (kernel + ane) with the same empty-set fallback; explicit
`--domain ane` bypass wins. `RIIR_ANE_PREGATE=0` (exact literal) is
DEMOTE-only from birth (env-switch law). NOT promoted: the G2 cost leg
(added p50 ≤ 1.0 / p95 ≤ 5.0 ms per file over the mixed walk, 3 rounds,
quiet-box PROVENANCE, utility floor = every eval-fixture span still cited
under the gate) is pre-registered in Plan 377 P2.1 and unrun — the owner's
standing promotion law applies only on a measured pass.

## Gates at landing

clippy `-D warnings` at no-default / default / ane / lone /
ane_pregate+latent / all-features (0 new warnings — the all-features
pre-existing set verified by a −u stash A/B); lib tests 3908 default / 4548
wide posture (the wide posture's 1220 s wall filed Issue 238); deny gate
PASS; pins manifest regenerated (carries BOTH this domain's and the
concurrent Batch-362 counts — the shared-checkout coexistence arc is
disclosed in HISTORY.md); numbering gate PASSED.

## Substrate knowledge

The Apple Neural Engine substrate itself (private runtime API, MIL/blob
formats, M3 Max hardware findings, the Rust-bridge negative result + the
working ObjC substrate) lives in [`../12_ane/`](../12_ane/README.md) — the
riir-ai Issue 726 T0 distillation. This file is the retrieval-domain record;
that folder is the hardware/API knowledge it routes on.
