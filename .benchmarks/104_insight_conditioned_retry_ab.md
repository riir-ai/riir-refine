# Bench 104 — Insight-Conditioned Retry A/B (Issue 140 T3)

**Status:** RECORD (T3 wiring + eval LANDED; verdict measured — see §3)
**Date:** 2026-10-01
**Corpus:** the floor harness's generated/interacting corpora, live toolchain `rustc 1.98.1`
**Arms:** static min-delta · control (code+shape-class stats, no admission) · insight (admission-filtered) · raw (site-keyed negative control)
**Run:** `cargo run --release --example rustc_errors_floor --features rustc_errors -- --only insight [--interacting|--multi K --strided]`

## 1. What landed

- `src/insight.rs` += `shape_class(message)` — the retry-family key's shape half: the message's leading clause with backticked identifier spans blanked (`"cannot borrow \`x\` as mutable, …"` → `"cannot borrow _ as mutable"`). Different variables, same error shape, ONE family.
- `playbooks::DiagDigest` — `{shape_class, insight}` computed ONCE per diagnostic at proposal time (the `span_embed` law: the message is current only where the diagnostic is), carried on `FixCandidate.diag_digest` and stamped into the ring (`AttemptRecord.shape_class` + `.insight`, both `#[serde(default)]` — pre-140 rings decode `None`).
- `domains::rustc_errors::insight_retry` — `InsightConditioning { Mode::Insight | Control | Raw }` composing the driver's existing `with_ordering` + `with_observer` seams (the Issue-093 composition shape; zero new seams, zero new deps):
  - **observer**: appends `(strategy, kept)` rows keyed `(code, shape_class)` (Raw: `(code, span_hash)`), applies the admission filter (Insight mode only), updates the goldilocks gate;
  - **ordering**: when ANY family is armed, ranks candidates by `best_belief_score(s, f, 0.32)` over the family's ≤`MAX_INSIGHTS`=5 most recent rows (the Beta-LCB primitive the flat/shape-conditional arms already use); when NONE armed, the STATIC order byte-identically (`order_candidates`, now `pub` for exactly this fallback);
  - **gate**: arm at running-success ≤0.50 (the paper's consult line), release at ≥0.65 (15pt hysteresis band);
  - **contextual-drag guard**: structurally — there is no API path that stuffs raw ring text into the conditioning context; the record API is the attempt ring + its DiagDigest, and `Mode::Raw` (the eval's negative control) recovers the episode-bound failure shape lawfully by keying on site identity.
- `driver.rs`: `push_attempt` now takes `&FixCandidate` (the digest rides at the one chokepoint — same single-point law as `span_embed`); `collect_candidates` fills the digest.
- Eval: `--only insight` section in `examples/rustc_errors_floor.rs` — 4 arms × paired per-case clean (McNemar b/c + paired LB95) AND per-case attempts (the efficiency axis: `attempts_b − attempts_a`, LB95 > 0 ⇒ a cheaper), + the headroom predicate (Bench 088's law, carried over) + per-arm policy counters (`recorded / refused-admission / families / armed / gate-arms / gate-releases`).
- Tests: +25 (4 `shape_class`, 7 `insight_retry`, extended round-trip/legacy-decode pins). Lib 2423/2423 at default AND `rustc_errors` postures; clippy clean ×3 postures (`rustc_errors`, `+latent_retrieval,nonergodic_driver` all-targets, examples).

## 2. Measured (the interacting corpus — the discriminative one; n=46, 184 multi-attempt runs)

| arm | clean | reverts | attempts | families | gate arms/releases |
|---|---|---|---|---|---|
| static (min-delta) | 46/46 | **24** | 116 | — | — |
| control (code+shape-class) | 46/46 | **10** | 102 | 4 | 3/2 |
| insight-conditioned | 46/46 | **10** | 102 | 4 | 3/2 |
| raw (site-keyed, neg-ctl) | 46/46 | **24** | 116 | 92 | 12/0 |

- **insight vs static**: efficiency **+0.128 attempts/case, paired LB95 > 0**; reverts 24 → 10 (−58%) at equal clean rate. The paper's loop mechanism — retries conditioned on accumulated verifier-distilled memory beat unconditioned retries — REPRODUCED on the attempt-efficiency axis.
- **insight vs raw (negative control)**: **+0.128 LB95 > 0** — the gain is the TRANSFER (the family key), not the presence of a policy: site-keyed conditioning is byte-identical to static (24 reverts), exactly the paper's "conditioning on raw attempt summaries FAILS" reproduced as a PASSING negative control.
- **insight vs control (THE control arm)**: **0/0 discordants, +0.000 efficiency — identical**. `refused-admission 0`: every distilled insight admitted (rustc's top-level messages are short and literal-sparse — the domain's structural property). **On this corpus the deterministic-template arm collapses into ordering**, precisely as the issue's honesty note anticipated; the admission filter is not exercised by this domain's diagnostics.

Other corpora: the stock template-covered stratum (126 cases) is first-try-perfect for every arm (headroom predicate: UNMEASURABLE — the Bench-045 class); MULTI-5 strided (40 cases) is all-keep never-clean (families never arm; all arms identical). The interacting corpus is the one place the mechanism is measurable.

**Determinism**: the full 4-arm interacting run is byte-identical ×2 (diff-clean).

## 3. Verdict

- **The conditioning mechanism (T3 wiring): LANDED and EARNED vs static and vs the negative control** — LB95 > 0 on attempts/case, reverts −58%, determinism pinned.
- **The insight-vs-ordering claim (admission filter's marginal value): NOT EARNED on this corpus — honestly collapsed into ordering.** The refused-admission counter reads 0 because rustc messages admit ~universally; the reader that would separate conditioning from ordering is a FROZEN GENERATOR consuming the sentence text (riir-train Plan 432's lane — the insight TEXT is now stamped in every ring record for exactly that consumer). Reopen trigger for the modelless lane: a corpus/domain whose verifier messages carry literal-laden shapes (paths, spans, hashes) at a nontrivial rate — the counter to watch is `refused-admission` > 0 with a paired difference.
- **Production posture**: stays unwired in the CLI (the `nonergodic_driver` precedent — a measured gain on the efficiency axis with the clean axis saturated is not a product claim); the policy is the library surface + the eval lane, opt-in by construction.

## 4. Provenance

Box: m3 · AC · loadavg ~7 at the eval runs (the sibling PSSA bench storm decaying — wall timings irrelevant here; every metric is attempt counts, not latency). Eval binary: release, `/tmp/insight140-t3` scratch target dir (no sibling lock contention). Raw outputs: `/tmp/insight_ab_stock.txt`, `/tmp/insight_ab_interacting.txt`, `/tmp/insight_ab_multi5strided.txt`.
