# Issue 213 — the `shader_perf` file gate (generalize the kernel pregate) → re-gate promotion; the per-query retrieval overhead finding (sec's blocker)

**Status:** OPEN 2026-11-06 (katgpt-rs-18) — pre-registered BEFORE implementation or measurement; follow-up to Issue 212 (closed, Bench 140: sec + shader_perf FAIL the G5 cost leg).

## Why

Bench 140: `shader_perf` added p50 1.10–1.28 / p95 5.30–5.92 ms per file (bar 1.0 / 5.0) over a `src/**/*.rs` walk that contains ZERO WGSL. Its 7 rules are WGSL render-stage rules; matching them against plain Rust spans can only yield noise. The kernel pregate (Issue 208, `src/kernel_pregate.rs`) already solves this shape for the kernel domain — consume it, never fork: generalize its needle scanner (needle set + path predicate as data) and give the matcher fan-out a per-domain allow set instead of the single `kernel_allowed` bool.

## Direction probe (m3, NOT a claim — release, `sec` 45-rule index, 1377 real source lines, one pass)

| leg | total | per query |
|---|---|---|
| ModellessEmbedder query build | 1.4–1.7 ms | ~1.1 µs (≈8% of a retrieve) |
| `retrieve_for_scoring_into`, latent + BM25 | 16.8 ms | ~12.2 µs |
| … latent leg only (`text = None`) | 10.1 ms | ~7.4 µs |

So sharing query embeddings across domains buys ≤ 8% — rejected as a lever. The cost of a 45-rule domain is per-QUERY fixed overhead inside riir-rag (`retrieve_candidates_into`: two-stage diverse latent scan + BM25 `Vec<(String, f32)>` keys re-parsed per hit), multiplied by (1 + significant lines) queries per span per domain. Bench 027 already recorded that the 8-D latent term never changes a clippy top-1 — yet it is ≈ 60% of retrieve time. That is `sec`'s real lever (it has no cheap file signal: its 44 rules span unsafe/FFI/hash/HTTP/path/shell/numeric vocabulary) and it is cross-repo (riir-neuron-db) + ranking-changing → its own pre-registration (T5), never folded into this gate.

## Pre-registered bars (pinned 2026-11-06, before any code)

- **Shader signal (recall-biased, pinned):** `.wgsl` paths always signal; else case-insensitive ASCII needles `@vertex`, `@fragment`, `wgsl`, `texturesample`, `shader`. File-level gate only (no span scope). Widening the set = a NEW registration.
- **G1 recall:** on every SIGNALED file, the `shader_perf` `by_domain` entries are byte-identical gated vs ungated (structural — the gate passes those spans through untouched; asserted by unit test). Unsignaled files lose their shader entries BY DESIGN; the count of shader rank-1 citations removed on the Bench-140 walk is recorded (disclosure, not a bar).
- **G3:** every other domain's output byte-identical (the allow set only adds a shader bit; unit test pins kernel/other domains unchanged).
- **G2 cost:** the Bench-140 protocol verbatim (`scripts/g5_domain_cost.sh`, same walk, 3 interleaved rounds, all rounds must pass), `shader_perf` ON: added **p50 ≤ 1.0 ms/file AND p95 ≤ 5.0 ms/file**, quiet rig via `agent-mail rig run`, PROVENANCE beside every number. Fail → stays opt-in, axis named.
- **Promotion (pass):** `default` + RELEASE_FEATURES + release.yml (build + licenses) + manual_release.sh in lockstep, one commit.
- **Escape hatch:** `--domain shader` bypasses the gate (the kernel `arm_domain_bypass` precedent, one shared bypass mechanism).

## Tasks

- [ ] T1 generalize `kernel_pregate::signal_lines` → a data-driven `NeedleSet` scanner (kernel set unchanged, byte-identical — its tests stay green unmodified); add the shader set + `shader_signaled(src, path)`
- [ ] T2 `LatentMatcher`: replace `kernel_allowed: bool` with a per-file domain allow set; the shader bit gates the `ShaderPerf` index exactly as the kernel bit gates `Kernel`; tests for G1/G3
- [ ] T3 the G2 run (Bench, rig queue) + the citation-removal disclosure
- [ ] T4 promote `shader_perf` iff G2 passes
- [ ] T5 file the riir-neuron-db retrieval-overhead issue (per-query fixed cost; latent-leg-skip A/B at latent_weight 0 — Bench 027's refutation says top-1 is flat; pre-register floors across all 7 domains) — `sec`'s promotion path
