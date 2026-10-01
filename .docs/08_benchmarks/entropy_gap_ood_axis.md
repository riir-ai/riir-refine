# The entropy-gap OOD axis (issue 077 T1/T4)

> **What:** the score-bench regime axis — whether the rule substrate
> responds to held-out code the same way it responds to its home ground,
> measured as the two-sample gap of per-fixture rule-selection entropies.
> Consumes `katgpt_core::regime_probe` (katgpt-rs Issue 740 / Bench 702);
> zero entropy arithmetic lives in this repo.

**Status:** RECORD — T1 + T4 landed 2026-09-09, riir-refine `1994a8a`; provenance surface landed same day at `80e8667` (see §Provenance families); T2 + T3 landed 2026-09-09 (`2e3ce9a` / `095686a`) — T3 as a recorded DEMOTE (`.benchmarks/080_margin_mining_ab.md`).

## Where it runs

Always-on with the `score_bench` feature (no CLI flag): the feature forwards
`katgpt-core/regime_probe`, and every score-bench run emits the axis twice —
as console lines in the report block and as the additive `ood` field of the
appended `score_history.jsonl` row. The measurement is
`OnceLock`-cached per process, so the printed numbers and the recorded row
are the same measurement by construction.

```sh
cargo run --features score_bench,kat_service --bin cargo-refine -- --score-bench
```

## What is measured

For every fixture span, `score_bench::ood::rule_score_vector` collects the
per-rule edit counts of the SAME pass-one proposer table the production
fixpoint uses (`FIXABLE_RULES` + `FIXABLE_LINTS`, `fxhashmap` skipped —
the documented divergence), then the regime_probe entropy kernel reads that
vector as logits: sharp firing (one rule dominates) → low entropy in nats,
diffuse firing (no rule stands out) → high entropy. A span no rule fires on
scores the maximum, ln V — the honest "no idea" entropy.

Two frozen in-tree sets, no oracle involved:

| set | source | role | first measured |
|---|---|---|---|
| `v1_frozen` | `score_bench::fixtures` — the frozen v1 bench set (mined 2026-08-19) | reference | 35 fixtures, mean **3.840 nats** |
| `gen6_real_repo` | `score_bench::generations::generation_v6` — real spans carved from pre-heal sibling states (mined 2026-09-06, AFTER the v1 freeze) | OOD | 17 spans, mean **3.110 nats** |

Set choice: the v2–v5 ladders are corpus-derived composites (in-distribution
by construction — composites OF corpus atoms), and v7 is authored-overlap;
generation 6 is the only real-code set mined outside the reference era, so
it is the OOD set by construction, not by simulation.

The row carries the primitive's two-sample statistic
(`entropy_gap_into`): `mean_gap = mean_ood − mean_ref`, a Cohen's-d-style
`standardized_gap`, and a BLAKE3 artifact over the canonical little-endian
encoding of both entropy samples. Fixtures are iterated in sorted-id order
per set, so the artifact is bit-deterministic — an audit can re-verify the
exact measurement. The artifact from run #84:
`faa021c89d1dbd2ce972e2e87bfe85dbd79fe93450b38dfa7bee866c31161904`.

## Regime classification

Bench 702 G1's threshold, applied to the mean gap:

| gap | regime | reading |
|---|---|---|
| > +0.5 nats | `memorizing` | held-out material fires more diffusely than the reference — the substrate distinguishes seen from unseen |
| within ±0.5 nats | `converged` | the two distributions have converged — the generalized regime |
| < −0.5 nats | `anomalous_negative` | held-out fires MORE sharply than the reference — not a paper regime; recorded loudly, never folded into either neighbor |
| either sample empty | `insufficient` | no verdict (raw fields keep the primitive's one-sided measurement, artifact-consistent) |

## The first measured row — and its honest reading

```
mean_ref 3.840 nats | mean_ood 3.110 nats | gap −0.729 nats (d = −1.11) → anomalous_negative
```

The held-out generation-6 spans fire MORE decisively (lower entropy) than
the v1 reference spans. Two readings, both recorded:

1. **Against the primitive's reference-vs-held-out convention** (reference =
   training corpus in Bench 702), a negative gap is the anomalous direction:
   the substrate is sharper on the set labelled held-out than on the set
   labelled reference. That says the LABELS are the problem, not the
   instrument — see reading 2.
2. **Against issue 077's corpus framing** ("held-out shapes get near-zero
   drafter entropy ≈ overfit"), sharper-on-held-out IS the overfit-direction
   signal, at moderate effect size (d = −1.11). The likely mechanism: the
   seed corpus and the generation-6 spans are siblings in provenance — both
   carved from pre-heal/pre-sweep states of the same repos — while the v1
   set is a broader mixed-era sample. The corpus is sharper on its own
   provenance family than on the broader bench. That is a real, measurable
   distribution shift the axis detected on its first run.

Both readings agree on the operational takeaway: **the corpus-proximity of a
fixture set, not its "held-out" label, decides which side of the gap it
lands on.** Future sets should record their provenance family next to their
frozen labels so the axis's verdict can be read against it. Re-measure after
corpus or proposer changes: the axis is a per-build probe, and the JSONL
history makes the drift visible per run.

## Provenance families (the set-composition reading, made measurable)

The row now carries each set's distinct provenance families — the
`repo@sha` prefix of every fixture's mining provenance (`OodRow
::ref_provenance_families` / `::ood_provenance_families`, serde-default
additive; the console report prints `family×count` pairs). The OOD side
resolves through `RAW_SPANS_V6` by id: the frozen fixture conversion stamps
`construction:gen6:…` provenance (freeze-gate metadata), so the family
question reads the real `repo@sha` off the raw span table.

Measured:

| set | families |
|---|---|
| `v1_frozen` | authored-composite, katgpt-rs@8da93f6c, katgpt-rs@9e0e6a57, riir-ai@849ce8330 (4 — mostly authored composites) |
| `gen6_real_repo` | katgpt-rs@5b2d42970~1, riir-ai@1e050442a~1, riir-chain@7421245a~1, riir-train@a1197e06~1, riir-train@e6084429~1 (5 — all real pre-heal states) |

This pins reading 2 above as a set-COMPOSITION fact: the substrate fires
more sharply on gen6 because gen6 is entirely sibling-provenance real code
while v1 is mostly authored — not because the substrate is sharper on
"unseen" material. The artifact input is deliberately unchanged: families
are static in-tree metadata, and folding them into the BLAKE3 would rotate
every future artifact without changing a single measurement.

### Fresh-family disambiguation (issue 082, 2026-09-09) — the attribution TESTED

The composition attribution above was tested with the 740-unblock pairing:
composition held real-vs-real, provenance held disjoint (fresh carves from
`riir-neuron-db` + `riir-dapps`, absent from every known set, vs the gen6
reference). **R1 fired: gap −0.7608 nats (d −1.026) on BOTH fresh families**
— the axis tracks corpus-family proximity on composition-matched real code.
Honest mechanism: the fresh signal is all-or-nothing (0/48 fresh carves fire
ANY proposer rule — all-flat at ln 48), and gen6's mining-time lint selection
remains a confound the pairing cannot remove; convergent validity is
demonstrated in the coarse discriminative form. Full record:
`.benchmarks/081_ood_fresh_family_convergent_validity.md` (promotion is
unchanged — `regime_probe` stays opt-in; this settles convergent validity,
not promotion).

## Compatibility

- Rows predating the axis deserialize with `ood: None`
  (`#[serde(default, skip_serializing_if)]`) — the JSONL stays
  backward-compatible, and pre-axis rows (e.g. run #83) keep their exact
  legacy shape.
- The Warm-tier `ScoreRunRow` mirror deliberately DROPS the axis: it is run
  instrumentation, not a scored axis; the jsonl remains the committed
  projection. The asymmetry is pinned by
  `traj_store::tests::score_run_row_is_schema_identical_to_history_record`.
- The SVG renderer is untouched.

## Tests

- `score_bench::ood::tests` (8): the Bench 702 G1 shape in miniature
  (peaked vs flat → `Memorizing`, gap > 1.0; mirrored → `AnomalousNegative`;
  identical → `Converged`, gap 0.0), threshold edges, empty-sample honesty,
  artifact determinism, score-vector shape/firing, and the full-pipeline
  reconciliation over the real sets (0 skips, counts reconcile, cached).
- `score_bench::report::tests`: pre-ood rows read `None`; the payload
  round-trips; an absent axis writes no key.
- Suite: `cargo test --features score_bench --lib` — 1396 passed, 0 failed;
  clippy clean at `--features score_bench` and at default features
  (`-D warnings`).
