# Bench 164: Issue 251 — product (soft-AND) vs additive composition, paired replay over the real store

**Status:** Done — VERDICT TIE → challenger DEMOTED (the additive incumbent stands); feature stays opt-in as the reproducible negative artifact

## Cell

- Instrument: `examples/composition_product_replay.rs` (report-only, store-read-only).
- Command: `cargo run --release --features latent_retrieval,composition_product --example composition_product_replay` (default `--splits 200`).
- Box: m3 (M-series laptop, AC power, QUIET — load 2.9/16, 33/64 GB free, no rig jobs), 2026-11-20; `CARGO_TARGET_DIR=/tmp/riir-refine-i251`.
- Corpus: m3's durable `.refine/trajectories.kv`, which holds 429 traj rows and 36 learn rows. 326 rows have N < 2. That leaves **9 groups** (rules with ≥ 2 rows at N ≥ 2) and 94 pool rows.
- Cross-fit: each row's outcomes are split without replacement (⌊N/2⌋ evidence → reliability leg, the rest held out → quality of the pick), with 200 seeded splits per group and SE taken across groups. Disclosed leak: the `learn:` EMA direction has seen the held-out half. Both laws share that leg, so the paired delta stays fair between the laws, though absolute quality is optimistic.

## Result

| metric | additive (incumbent) | product (challenger) |
|---|---|---|
| held-out success of pick | 1.0000 | 1.0000 |
| paired delta | — | +0.0000 (SE 0.0000) |
| pick agreement | — | 1.0000 (0/9 groups ever disagree) |
| refused-leg pools | — | 0 |
| time, ns/pick (min of 5, 1800 pools) | 7.2 | 45.5 (6.30×) |
| alloc | stack-only | stack-only (alloc-neutral) |

Non-inferiority δ = max(1.0pp, 2.5·SE) = 0.0100. Delta 0 lies inside ±δ and the win test (mean − 1.96·SE > 0) fails, so the result is **TIE → DEMOTE**. The never-slower bar also fails: the per-leg `ln` makes the product 6.3× slower.

## Reading

- **No separation at two legs, on this corpus.** The store carries no weak-leg candidates for the product to punish: every pool's held-out success is 1.0, so both laws pick the same row in every split. This is the failure mode the issue named up front ("product-vs-additive differences are marginal at n=2 unless the corpus carries genuinely weak-leg candidates"), and the store here is success-saturated.
- **Cost note for any re-open:** for two legs in (0, 1], `p₁·p₂` ranks the same as `ln p₁ + ln p₂` and costs no transcendental. A re-open should score the plain product and keep the log form only for n ≫ 2 legs, where underflow becomes real.
- **T3 (sweep) not triggered:** the challenger won nowhere, so the 0.6/0.4 constant stays as Bench 042 left it (flat across ρ, 0.4 = sweep max in the harsh regime).
- **Re-open trigger:** a store with ≥ 2 rows per rule carrying real failures (held-out success < 1 in some pool), or a third selection leg (the n-leg generalization question). Run the same instrument and compare against this row.

## Standing proof the challenger changed nothing when demoted

The feature-off and feature-on postures both pass: `selection_tether` lib (8/8, including the "cold start must reproduce W_EVO·p1 + W_RATE·p2 exactly" pin), `self_evolve` lib (20/20), `t93_selection_regret` (2/2 default, 1 + 2 ignored at feature-on), and `t33_tether_g4_alloc` (2/2). The incumbent expression is untouched (`compose(false, …)` is the same `W_EVO * evo + W_RATE * rel` f32 expression).
