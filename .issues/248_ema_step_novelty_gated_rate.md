# Issue 248: novelty-gated rate for `evolve::ema_step` (katgpt-rs Plan 639 T3.4)
**Status:** DONE-NEGATIVE — G1 PASS · **G2 FAIL (0/32 cells)** · **G3 FAIL (median Δ 28.89 ns > 10.0 ns at k = 1, bmo ×3)** · G4 PASS. `evolve_novelty` stays OPT-IN (instrument only, never promoted); the durable learn-row arm (T5) is moot. Pre-registration `b2e88a09`.
**Claimed:** p639-t34@amo, 1795077845, expires 2026-11-20T08:44:05Z

**Source:** katgpt-rs Plan 639 Phase 3 T3.4 ("outcome-signed α/β → evidence-shaped;
touches the durable learn-row format, needs its own determinism review"). Substrate:
katgpt-rs `f907edece` — `katgpt_core::sense::novelty_retention::{NoveltyChannel,
NoveltyRetentionConfig}` behind katgpt-core feature `novelty_retention` (DEFAULT-OFF).
Plan 639 T2.5 (`bd477ad63`): G3 PASS for k = 1 at Δ 9.96 ns on bmo — every consumer
re-measures `k × Δ ≤ 10 ns` at its own wiring; a pass is not inherited.

## The change

`ema_step` moves a rule's evolved direction by a FIXED outcome-signed rate (α = 0.1
toward on success, β = 0.05 away on fail). The gated variant keeps the sign and lets
the evidence shape the magnitude:

- **Observation signal (chosen):** the outcome-signed *agreement* of the evidence with
  the current belief, `x = (+1 | −1) · cos(dir, fix_dir)` (+1 success, −1 fail).
  A steady world reads a steady, high `x` (successes land near the belief, fails land
  away from it); a regime shift (the fix that used to work now fails / a new fix
  works) drives `x` down and KEEPS it down → sustained novelty → lower retention →
  faster adaptation; a noisy-steady rule makes `x` jump around its mean → transient
  share → higher retention → less estimate noise. On a single-fix rule `cos ≈ 1` and
  `x` degenerates to the ±1 outcome bit, so the signal generalises the outcome stream.
  Zero-norm `dir`/`fix_dir` → `x` non-finite → the substrate returns `alpha_base`
  (the incumbent rate) and leaves its EMAs untouched.
- **Rate ↔ retention map (explicit):** `NoveltyChannel::step` returns a RETENTION.
  `ema_step` uses a RATE. So `alpha_base = 1 − base_rate` (base_rate = α on success,
  β on fail), `g = channel.step(cfg, x, 1 − base_rate)`, `rate = 1 − g`, signed by the
  outcome. `g ∈ [g_min, g_max]` ⇒ `rate ∈ [1 − g_max, 1 − g_min]`.
- **k = 1:** one channel step per `ema_step` call, one `ema_step` per verdict.
- Feature `evolve_novelty = ["katgpt-core/novelty_retention"]`, OPT-IN, per-instance
  armed (`LatentSkillEvolution::arm_novelty(cfg)` → validated config + one
  `NoveltyChannel` per lint, created with the direction at the first success). Unarmed
  / feature-off = the incumbent code path, byte-for-byte.

## Determinism + durable-format review (explicit)

- **Learn rows ARE persisted** (`learn:{lint_key}` → JSON `LearnRowDoc`, written by
  `TrajStore::learn_step` → `ema_step` with `EvolveConfig::default()`); they are NOT
  BLAKE3-hashed, NOT synced, NOT replay-compared (read only by `--memory-stats` +
  tests). Still: a persisted row must be a deterministic function of its verdict
  sequence.
- **Decision: the durable learn-row path is NOT armed by this issue; `LearnRowDoc`'s
  format is unchanged; the novelty state stays OUT of the durable row.** Reasons:
  1. *Session-local state breaks the row's determinism contract.* A CLI invocation is
     one heal-run; a rule typically sees 1–few verdicts per run. A per-process channel
     would re-seed every run (seeded at `x₀` ⇒ `s = n = 0` ⇒ `g = alpha_base`), so the
     armed durable path would be (a) inert in practice and (b) a different function of
     the same verdict sequence depending on how verdicts split across processes.
  2. *Persisting the channel needs substrate accessors that do not exist.*
     `NoveltyChannel`'s `fast/slow/seeded` are private with no serde/getters
     (only `sustained()`); re-implementing the dual EMA here is forbidden (one law,
     one `cell`). A durable arm needs a katgpt-rs follow-up (state accessors or
     serde), then an ADDITIVE `#[serde(default, skip_serializing_if = "Option::is_none")]`
     field so unarmed rows stay byte-identical.
  3. The channel state is not recomputable from the row (counters only, no sequence).
- **Armed in-process path:** every op is per-scalar f32 in a fixed order; the
  agreement `cos` uses sequential `+=` folds over the 8 components (never a SIMD
  lane-sum — katgpt-rs AGENTS.md Determinism law, Issue 940), then the substrate's
  elementwise `cell`. No HashMap-order dependence (per-lint lookups only).

## Pre-registered gates (pushed BEFORE any measurement)

- **G1 bit-exact incumbent.** (a) Feature off: `ema_step` unchanged; the existing
  evolve tests + `traj_store_tests` learn-row tests stay green. (b) Feature ON,
  unarmed: `LatentSkillEvolution` over a seeded 2 000-outcome stream is bit-identical
  (`assert_eq!` on every direction, `f32::to_bits`) to a feature-off-equivalent
  reference built from `ema_step`. (c) Feature ON: learn rows byte-identical — the
  durable path does not consume the gate (by construction: `learn_step` untouched;
  asserted by the learn-row tests passing under `--features evolve_novelty`).
- **G2 quality (refine fixture, T2.2's matched-point model).** Two real embedder
  fixes `A = embed("g2_rule", "fix_a")`, `B = embed("g2_rule", "fix_b")`; readout
  margin `m = dot(dir, A) − dot(dir, B)`; every stream starts at `dir = A`; each tick
  applies A or B by a fair coin and succeeds ~ Bernoulli(p_good | p_bad). splitmix64,
  seeds {1, 2, 3}.
  - *Adaptation (tracking lag):* T = 3 000, the good fix flips every 300 ticks
    (p_good 0.9 / p_bad 0.1); lag = first tick after a flip from which `sign(m)`
    matches the new regime for 10 consecutive ticks (censored at 300); adaptation =
    mean lag over the 9 flips.
  - *Noise (estimate noise):* T = 3 000, A good throughout, p_good 0.7 / p_bad 0.3;
    noise = RMSE of `m` about its own mean over ticks [500, 3 000).
  - *Fixed curve:* α ∈ {0.01, 0.02, 0.03, 0.05, 0.07, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5},
    β = α/2 (the shipped 2:1 ratio).
  - *Gated grid (32 cells):* base α ∈ {0.05, 0.1} (β = α/2) × κ ∈ {0.4, 0.8} ×
    μ ∈ {0.2, 0.4} × β_sat ∈ {4, 8} × g_min ∈ {0.5, 0.7}; g_max 0.995, EMAs 0.3/0.03.
  - *Pass (a cell):* (i) on EACH of the 3 seeds, the cell has adaptation ≤ 0.7 × and
    noise ≤ 1.1 × of the SAME fixed point p (one p for all 3 seeds); (ii) on seed
    means, no fixed point dominates (≤ on both axes); (iii) on seed means, distance
    below the piecewise-linear fixed curve (noise interpolated at the cell's
    adaptation) > 0 — undefined outside the curve's adaptation range ⇒ not claimable.
    ≥ 1 passing cell = G2 PASS; both full tables recorded, no cherry-picks. No
    passing cell ⇒ G2 FAIL (honest negative, DONE-NEGATIVE close).
- **G3 cost at k = 1.** Added cost per update Δ = (armed `ema_step_gated`) −
  (`ema_step`), ns/call, interleaved A/B (51 chunk pairs × 20 000 calls, per-arm
  median of chunk ns/call, `black_box` on inputs and outputs), release test binary
  on **bmo** (binary only), `bench_preflight.sh` (MAX_LOAD = 11.2) PASS at launch
  AND end per run (tainted ⇒ re-run, never folded); 3 runs; statistic = median of
  the per-run Δ. **PASS iff 1 × Δ ≤ 10.0 ns.** Call-frequency mapping: one
  `ema_step` per verdict, one verdict per rule-update; Plan 639 T2.5's 10.0 ns
  per-entity-tick budget is applied per rule-update (k = 1) — verdicts arrive
  behind a compile/clippy run (≥ ms), so this is the strict reading, not the
  binding one.
- **G4 alloc-free armed path.** Own test binary with a counting global allocator:
  0 allocations over 1 000 armed `ema_step_gated` calls and over 1 000 armed
  `LatentSkillEvolution::record_outcome` calls after the first-success warm-up.

## Tasks
- [x] T1 feature + `ema_step_gated` + `LatentSkillEvolution::arm_novelty` (G1, G4 tests)
- [x] T2 G2 fixture (ignored test, `--release`), both tables recorded here
- [x] T3 G3 bmo cell (3 runs), PROVENANCE quoted
- [x] T4 verdict + close (HISTORY row, issue removed, feature stays OPT-IN)
- [-] T5 durable learn-row arm — MOOT (G2 + G3 failed); would have needed a
  katgpt-rs `NoveltyChannel` state accessor/serde follow-up (see the review above).

## Results (2026-11-19)

Code: `src/evolve.rs` (`ema_apply` shared body — `ema_step` bit-identical;
`outcome_agreement`, `ema_step_gated`, `LatentSkillEvolution::arm_novelty` /
`novelty_armed`), `tests/evolve_novelty_gates.rs`, `tests/evolve_novelty_g4_alloc.rs`,
feature `evolve_novelty`. Substrate resolved at katgpt-rs `f907edece`.

- **G1 PASS.** Feature ON unarmed (and refused-arm on an invalid config) layer
  bit-identical (`to_bits`) to the `ema_step` reference over a 2 000-outcome,
  3-lint stream; armed path bit-reproducible across two runs and NOT vacuous
  (differs from unarmed); zero-norm evidence falls back to the base rate. Evolve unit
  tests + `traj_store` learn-row tests 42/42 in BOTH postures (`learn_step`
  untouched ⇒ rows byte-identical by construction); `t71_evolved_directions` green.
- **G4 PASS.** 0 allocations over 1 000 armed `ema_step_gated` calls and 1 000 armed
  `record_outcome` calls after warm-up (counting allocator, own binary).
- **G2 FAIL — 0/32 cells** (`--release`, amo; fixture `cos(A, B) = −0.2656`).
  Fixed curve (β = α/2), seed means adaptation / noise:
  α 0.01 125.5/0.2539 · 0.02 86.5/0.3872 · 0.03 61.6/0.4706 · 0.05 36.8/0.5973 ·
  0.07 26.4/0.7024 · 0.10 17.8/0.8351 · 0.15 11.7/1.0178 · 0.20 8.4/1.1745 ·
  0.30 5.5/1.4450 · 0.40 4.4/1.6923 · 0.50 3.7/1.9259.
  Gated grid (base α κ μ β_sat g_min → mean adapt/noise, (iii) distance):

  | cell | adapt/noise | (ii) | (iii) |
  |---|---|---|---|
  | 0.05 0.4 0.2 4 0.5 | 9.1/0.9860 | ok | +0.1528 |
  | 0.05 0.4 0.2 4 0.7 | 9.1/0.9860 | ok | +0.1528 |
  | 0.05 0.4 0.2 8 0.5 | 7.6/1.1893 | ok | +0.0615 |
  | 0.05 0.4 0.2 8 0.7 | 7.9/1.1847 | ok | +0.0418 |
  | 0.05 0.4 0.4 4 0.5 | 10.0/1.0707 | ok | +0.0290 |
  | 0.05 0.4 0.4 4 0.7 | 10.0/1.0707 | ok | +0.0290 |
  | 0.05 0.4 0.4 8 0.5 | 8.6/1.1962 | DOM | −0.0288 |
  | 0.05 0.4 0.4 8 0.7 | 9.2/1.2006 | DOM | −0.0653 |
  | 0.05 0.8 0.2 4 0.5 | 5.0/1.4693 | ok | +0.0954 |
  | 0.05 0.8 0.2 4 0.7 | 7.2/1.3665 | ok | −0.0776 |
  | 0.05 0.8 0.2 8 0.5 | 4.4/1.7366 | DOM | −0.0443 |
  | 0.05 0.8 0.2 8 0.7 | 6.6/1.4713 | DOM | −0.1234 |
  | 0.05 0.8 0.4 4 0.5 | 5.4/1.4297 | ok | +0.0472 |
  | 0.05 0.8 0.4 4 0.7 | 7.7/1.2547 | ok | −0.0109 |
  | 0.05 0.8 0.4 8 0.5 | 5.0/1.5203 | ok | +0.0444 |
  | 0.05 0.8 0.4 8 0.7 | 7.3/1.2970 | ok | −0.0219 |
  | 0.10 0.4 0.2 4 0.5 | 7.5/1.2118 | ok | +0.0459 |
  | 0.10 0.4 0.2 4 0.7 | 7.7/1.2113 | ok | +0.0325 |
  | 0.10 0.4 0.2 8 0.5 | 6.4/1.3249 | ok | +0.0369 |
  | 0.10 0.4 0.2 8 0.7 | 7.3/1.2848 | ok | −0.0098 |
  | 0.10 0.4 0.4 4 0.5 | 8.4/1.1965 | ok | −0.0220 |
  | 0.10 0.4 0.4 4 0.7 | 8.6/1.1965 | DOM | −0.0291 |
  | 0.10 0.4 0.4 8 0.5 | 7.3/1.2979 | ok | −0.0159 |
  | 0.10 0.4 0.4 8 0.7 | 8.4/1.2481 | ok | −0.0701 |
  | 0.10 0.8 0.2 4 0.5 | 4.8/1.5358 | ok | +0.0608 |
  | 0.10 0.8 0.2 4 0.7 | 6.7/1.4220 | ok | −0.0845 |
  | 0.10 0.8 0.2 8 0.5 | 4.1/1.7889 | ok | −0.0228 |
  | 0.10 0.8 0.2 8 0.7 | 6.3/1.5190 | DOM | −0.1434 |
  | 0.10 0.8 0.4 4 0.5 | 5.0/1.4801 | ok | +0.0686 |
  | 0.10 0.8 0.4 4 0.7 | 7.5/1.2837 | ok | −0.0225 |
  | 0.10 0.8 0.4 8 0.5 | 4.8/1.5451 | ok | +0.0594 |
  | 0.10 0.8 0.4 8 0.7 | 7.2/1.3306 | ok | −0.0417 |

  Condition (i) (the 0.7× / 1.1× matched-point margins vs ONE fixed point, 3/3
  seeds) holds in NO cell. Reading: the gate mostly slides the operating point
  ALONG the fixed curve toward the fast/noisy end (every cell adapts in 4–10 ticks,
  i.e. the α 0.15–0.4 region). 14/32 cells sit below the line (best
  `0.05 0.4 0.2 4 *` at +0.153 — ≈ 13 % less noise than the interpolated fixed
  curve at matched lag), but that best cell is 0.78× the lag of α 0.15 and 1.18×
  the noise of α 0.10 — between points, never inside both margins of one.
  Per the pre-registration this is a FAIL; no post-hoc grid widening was run.
- **G3 FAIL — median Δ 28.888 ns > 10.0 ns (k = 1).** bmo, release test binary
  only (no source), interleaved 51 × 20 000, `black_box` both ends, under
  `/tmp/bmo-bench.lock`, preflight `MAX_LOAD=11.2` PASS at launch AND end every run:
  run 1 a 8.221 / b 37.075 → Δ 28.854 ns; run 2 a 8.162 / b 37.085 → Δ 28.923 ns;
  run 3 a 8.221 / b 37.108 → Δ 28.888 ns (ratio 4.51–4.54×). PROVENANCE (all six):
  `power=AC Power load=0.83–0.93 swap=18627.69M canary=skipped powermode=0(auto) ane=ok`.
  Disclosure: one unfenced diagnostic run (the first fenced loop's grep missed the
  output line) read Δ 22.4 ns at a 6.4 ns incumbent — not folded in. The armed path
  is a serial latency chain (3 folds + sqrt + div for the agreement, then the
  channel's division) on top of an ~8 ns 8-lane EMA; not decomposed further.
- **Verdict: DONE-NEGATIVE.** `evolve_novelty` stays OPT-IN as a re-runnable
  instrument; nothing promoted; the durable learn-row path and `LearnRowDoc` are
  unchanged.
