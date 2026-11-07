# Issue 213 — the `shader_perf` file gate (generalize the kernel pregate) → re-gate promotion; the per-query retrieval overhead finding (sec's blocker)

**Status:** OPEN — T1+T2 LANDED (design AMENDED pre-measurement, see below; gates: release-set lib 4151/0 + the 1 failure root-caused to a client_id temp-dir collision, fixed in its own commit; clippy -D clean at default / release set / release+sec+shader_perf; fmt clean); T3 run queued on amo. Filed 2026-11-06 (katgpt-rs-18), pre-registered BEFORE implementation or measurement; follow-up to Issue 212 (closed, Bench 140: sec + shader_perf FAIL the G5 cost leg).
**Claimed:** zed-13afd776@m3, 1794011737, expires 2026-11-09T00:35:37Z

## Why

Bench 140: `shader_perf` added p50 1.10–1.28 / p95 5.30–5.92 ms per file (bar 1.0 / 5.0) over a `src/**/*.rs` walk that contains ZERO WGSL. Its 7 rules are WGSL render-stage rules; matching them against plain Rust spans can only yield noise. The kernel pregate (Issue 208, `src/kernel_pregate.rs`) already solves this shape for the kernel domain — consume it, never fork: generalize its needle scanner (needle set + path predicate as data) and give the matcher fan-out a per-domain allow set instead of the single `kernel_allowed` bool.

## Direction probe (m3, NOT a claim — release, `sec` 45-rule index, 1377 real source lines, one pass)

| leg | total | per query |
|---|---|---|
| ModellessEmbedder query build | 1.4–1.7 ms | ~1.1 µs (≈8% of a retrieve) |
| `retrieve_for_scoring_into`, latent + BM25 | 16.8 ms | ~12.2 µs |
| … latent leg only (`text = None`) | 10.1 ms | ~7.4 µs |

So sharing query embeddings across domains buys ≤ 8% — rejected as a lever. The cost of a 45-rule domain is per-QUERY fixed overhead inside riir-rag (`retrieve_candidates_into`: two-stage diverse latent scan + BM25 `Vec<(String, f32)>` keys re-parsed per hit), multiplied by (1 + significant lines) queries per span per domain. Bench 027 already recorded that the 8-D latent term never changes a clippy top-1 — yet it is ≈ 60% of retrieve time. That is `sec`'s real lever (it has no cheap file signal: its 44 rules span unsafe/FFI/hash/HTTP/path/shell/numeric vocabulary) and it is cross-repo (riir-neuron-db) + ranking-changing → its own pre-registration (T5), never folded into this gate.

## Design amendment (2026-11-06, BEFORE any measurement — the substrate-first find)

The needle-set design below is SUPERSEDED. `DomainId::applies_to_path` (Issue 145 T4) already pins which file kinds each domain's rules can touch (`.rs` → clippy/perf/sec/kernel; `.wgsl` → shader/kernel; Dockerfiles → docker; manifests → dist) — but it ran DISPLAY-side only (`heal.rs` `fits_any` filter), so every `.rs` span paid retrieval for the shader AND the default-on docker + dist indexes, and the display then discarded it. The amended gate applies that contract BEFORE retrieval: a per-span `DomainMask` (file-level path set ∩ the kernel pregate's span verdict; empty → every domain, the display's `fits_any` fallback verbatim). Consequences:
- **Displayed output is identical by construction** (same contract, same fallback); the parity tests pin applying domains byte-identical to the ungated fan-out and non-applying ones absent.
- **Behavior change (disclosed):** the four all-domain rule pools (trajectory `span_candidates`, the seat-rerank fixed-code pool, the kat lease pass-1/pass-2 rule sets) no longer carry Dockerfile/manifest/WGSL rules for `.rs` spans — noise by the Issue-145 contract; the lease delta compares two same-gated passes, so it stays self-consistent.
- **Wider win:** docker + dist are DEFAULT-ON, so the default/release scan sheds two wasted indexes per span — G2 now also measures the default posture's scan time OFF→ON (no bar; recorded).
- No needle scanner, no new bypass: `--domain shader` on a `.rs` file showed nothing before and shows nothing now. The kernel pregate is untouched. Side finding recorded: the kernel needle `@compute` has never fired (the scanner folds `@` with `| 0x20` → backtick) — fixing it widens the kernel signal set = a new registration, not done here.

## Pre-registered bars (pinned 2026-11-06, before any code)

- **Shader signal (recall-biased, pinned):** `.wgsl` paths always signal; else case-insensitive ASCII needles `@vertex`, `@fragment`, `wgsl`, `texturesample`, `shader`. File-level gate only (no span scope). Widening the set = a NEW registration.
- **G1 recall:** on every SIGNALED file, the `shader_perf` `by_domain` entries are byte-identical gated vs ungated (structural — the gate passes those spans through untouched; asserted by unit test). Unsignaled files lose their shader entries BY DESIGN; the count of shader rank-1 citations removed on the Bench-140 walk is recorded (disclosure, not a bar).
- **G3:** every other domain's output byte-identical (the allow set only adds a shader bit; unit test pins kernel/other domains unchanged).
- **G2 cost:** the Bench-140 protocol verbatim (`scripts/g5_domain_cost.sh`, same walk, 3 interleaved rounds, all rounds must pass), `shader_perf` ON: added **p50 ≤ 1.0 ms/file AND p95 ≤ 5.0 ms/file**, quiet rig via `agent-mail rig run`, PROVENANCE beside every number. Fail → stays opt-in, axis named.
- **Promotion (pass):** `default` + RELEASE_FEATURES + release.yml (build + licenses) + manual_release.sh in lockstep, one commit.
- **Escape hatch:** `--domain shader` bypasses the gate (the kernel `arm_domain_bypass` precedent, one shared bypass mechanism).

## Tasks

- [x] T1 (amended) `DomainMask` (`src/draft/domain_router.rs`, re-exported) + `DomainMask::applying_to_path` over the Issue-145 contract — no new scanner
- [x] T2 `LatentMatcher`: `kernel_allowed: bool` → per-span `DomainMask` (`span_allow`: path set ∩ pregate verdict, empty → ALL); `path_gate_tests` (3) + the kernel pregate tests green
- [ ] T3 the G2 run (Bench, rig queue) + the citation-removal disclosure —
      QUEUED: Exclusive amo job `37f5c2b5` (full `01a1b372-8a98-771a-a37d-`
      `c10237f5c2b5`, worktree `riir-refine@206a9ebf1`, the pre-registered
      script verbatim, 3 interleaved rounds). **THE RECORD MUST CITE
      `37f5c2b5` ONLY** — the stale duplicate entry `484ef998`
      (`/Users/kat/i213_g2.sh`, deleted from m3) was CANCELLED 2026-11-07 by
      full UUID per verdict round 1. **G3 gate ADDED (verdict round 1, D2):**
      promotion requires G2 ∧ G3 — G3 = the retention/disclosure diff
      (`scripts/g5_shader_g3_diff.py`, `cf526c69`) + lib tests at
      `latent_retrieval,shader_perf`, QUEUED as Exclusive amo job `01a1b3e9`
      (PRE arm = revert of the three T1/T2 files only, SAME tree both arms;
      the diff adjudicates zero non-shader rank-1 changes across the other
      six domains and emits the citation-removal disclosure)
- [ ] T4 promote `shader_perf` iff G2 passes **AND G3 passes** (verdict round 1: no owner gate if both pass — the GOAT gain is proven modellessly; either fails ⇒ stays opt-in with the failing check named)
- [ ] T5 file the riir-neuron-db retrieval-overhead issue (per-query fixed cost; latent-leg-skip A/B at latent_weight 0 — Bench 027's refutation says top-1 is flat; pre-register floors across all 7 domains) — `sec`'s promotion path
