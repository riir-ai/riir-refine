# Bench 081: OOD fresh-family pairing — the convergent-validity experiment (Issue 082)

**Status:** RECORD — pre-registered **R1 (convergent validity demonstrated)** fired 2026-09-09; example `examples/ood_fresh_family.rs`, issue `.issues/082_ood_fresh_family_convergent_validity.md`; cross-repo addendum paragraph updated in katgpt-rs `.benchmarks/702_regime_probe_goat.md` (see it for this repo's landing hash).

## Question

Run #84 (the first OOD-axis row: ref = v1-frozen 3.840 nats vs ood = gen6 3.110 nats, gap −0.729, d −1.11 → `anomalous_negative`) was attributed to **set composition** — v1 is mostly authored composites, gen6 is all real — not to corpus memorization. That attribution was untested. This bench runs the disambiguating pairing: **composition held constant (real code vs real code), provenance held disjoint.**

- Does the entropy gap track corpus-family proximity (convergent validity — the axis detects memorization)? ⇒ R1.
- Or does it vanish on real-vs-real (the axis only ever measured authored-vs-real texture)? ⇒ R2.

## Pre-registration (fixed BEFORE the first run; mirrored verbatim in the example's module doc)

- Sign convention: `gap = mean_ref − mean_ood` (the axis's internal `mean_gap` is the exact negation; both printed). `d` = the katgpt-core kernel's `standardized_gap`, sign-flipped to match.
- **R1:** `gap < 0` AND `|d| ≥ 0.5` on BOTH fresh families ⇒ convergent validity demonstrated; #84's anomalous_negative WAS composition, as attributed.
- **R2:** `|gap| < 0.5` nats OR `gap > 0` on BOTH ⇒ convergent validity fails on real data; the probe stays a lab instrument.
- **R3:** families disagree (mixed arms), or either FRESH family yields < 20 carves (the pre-registered yield floor — it gates the carved OOD families; the reference is the frozen 17-span gen6 table, exempt by construction) ⇒ inconclusive, report per-family, stop.
- Per-family arm: R1-arm iff `gap < 0 && |d| ≥ 0.5`; R2-arm iff `|gap| < 0.5 || gap > 0`; else ambiguous. Both R1 ⇒ R1; both R2 ⇒ R2; any mix ⇒ R3.
- **Uncontested-pool edge (decided, pre-registered):** ALL parsed spans enter the entropy set — an all-flat edit-count vector is the legitimate "no rule fits" high-entropy signal (max ln V), not a measurement failure. Contested/uncontested split reported per set under BOTH censuses: rule-vector (≥ 2 rules fired) and latent pool (`span_margin`, the margin_weight access pattern — contested_domains ≥ 1). Contested-only mean entropy rides as a robustness view; it does NOT gate the verdict.
- **Oracle-free by construction:** no clippy oracle runs anywhere — the probe consumes proposer edit-count score vectors (`rule_score_vector`, the OOD axis's own statistic) + the latent census.

## Sets + provenance

| set | role | n | provenance |
|---|---|---|---|
| `gen6` (`RAW_SPANS_V6`) | reference (corpus-family) | 17 | real pre-heal states of katgpt-rs / riir-ai / riir-chain / riir-train — the repos the rule corpus was mined/authored against |
| riir-neuron-db carves | OOD-A | 24 | `2f30800` working-tree HEAD, clean `develop`; **absent from every known set** |
| riir-dapps carves | OOD-B | 24 | `c4c6ffe` working-tree HEAD, clean `develop`; **absent from every known set** |

Carve protocol (deterministic; no sampling, no randomness, no seeds): `collect_source_files_sorted` (hidden/`target`/`vendor` pruned) → `syn::parse_file` per file (0 failures in this run) → top-level `syn::Item::Fn` only, non-empty body, sliced by `item.span().byte_range()` (the `score_bench_mine_v6` carve convention — outer attrs/doc comments ride inside the item span) → ≥ 200-byte floor → per-file cap 2 → stop at 24 per family. Fresh carves measured as RAW item text, matching the reference (generation fixtures store raw span text; `wrap()` is an oracle-harness concern, deliberately absent).

**Fresh-set composition (recorded loudly):** the sorted-path prefix consumed ONLY `benches/` files (neuron-db) and ONLY `cloudflare/kat-service/` files (dapps) — the 24-carve budget was exhausted before the walk ever reached `src/`. This is the pre-registered selection doing exactly what it says, and it is itself informative (foreign BENCH and worker-contract code is all-flat), but an auditor should know the fresh families are bench/worker code, not library code.

## Results

| set | n | mean-H (nats) | H(contested-only) | vec≥2 | pool≥1 | minB | meanB |
|---|---|---|---|---|---|---|---|
| gen6 (ref) | 17 | **3.1104** | 1.0722 | 3 | 17 | 61 | 3474 |
| riir-neuron-db | 24 | **3.8712** | — (none contested) | 0 | 24 | 275 | 1612 |
| riir-dapps | 24 | **3.8712** | — (none contested) | 0 | 24 | 223 | 941 |

gen6 per-repo family breakdown: `katgpt-rs@5b2d42970~1` n=7 mean 3.6000 · `riir-ai@1e050442a~1` n=4 mean 3.2582 · `riir-chain@7421245a~1` n=2 mean 3.7881 · `riir-train@a1197e06~1` n=3 mean 1.0722 · `riir-train@e6084429~1` n=1 mean 3.8517.

| pairing | mean_ref | mean_ood | gap (ref−ood) | d | arm | artifact (BLAKE3) |
|---|---|---|---|---|---|---|
| gen6 vs neuron-db | 3.1104 | 3.8712 | **−0.7608** | **−1.026** | R1-arm | `7f002073…cf4b` |
| gen6 vs dapps | 3.1104 | 3.8712 | **−0.7608** | **−1.026** | R1-arm | `7f002073…cf4b` |

## Verdict: R1 — CONVERGENT VALIDITY DEMONSTRATED

The corpus-family reference fires measurably sharper than BOTH provenance-disjoint fresh families on real-vs-real code. The gap survives composition matching ⇒ run #84's `anomalous_negative` WAS set composition, as attributed — and the 702 addendum's unblock condition (a reference/OOD pairing with disjoint provenance families) is now satisfied **on the convergent-validity axis**.

**Instrument cross-check:** the recomputed gen6 mean (3.1104, 17 spans, sorted-id order, same `rule_score_vector` + kernel as `ood.rs`) reproduces run #84's recorded 3.110 exactly — the example and the production axis measure identically.

**Identical artifacts across the two pairings are expected, not a bug:** both fresh samples are 24 × 3.8712 (the all-flat entropy), so `entropy_gap_into` received identical entropy vectors and hashed identical bytes.

## Mechanism (what the gap is made of — read before citing "convergent validity")

- **0 / 48 fresh carves fire ANY proposer rule.** Every fresh span's score vector is all-flat ⇒ every fresh entropy sits exactly at ln(48) = 3.8712 nats (V = 48 = |FIXABLE_RULES − fxhashmap| + |FIXABLE_LINTS|). The gap is maximally mechanical: reference variance (3/17 gen6 spans carry ≥ 2 firing rules; the svr trio sits at H = 1.0722) vs a constant.
- **Coarse form.** The demonstrated validity is discriminative — the axis separates corpus-family material from foreign material — NOT a graded proximity measure; on this pairing it is all-or-nothing.
- **Selection confound.** gen6 spans are lint-bearing BY mining-time selection (each carries frozen vocabulary-lint claims); the fresh carves are unselected real code. Corpus proximity and selection are confounded in the reference by construction, and the prescribed disjoint-family pairing cannot decompose them. The 702 addendum's follow-up ("record provenance next to the frozen labels") is where a graded, selection-controlled design would live.
- **Contest dissociation (both censuses, per pre-registration):** the latent pool says 65/65 spans contested (≥ 2 retrieval candidates in ≥ 1 of the Clippy/Perf/Docker domain views — the matcher finds near neighbors everywhere), while the rule-vector census says 3/65. Two different "contest" notions; both reported, neither gates the verdict.

## Corrections during landing (recorded, not silent)

The first run printed **R3** because the yield-floor gate was mechanically applied to the REFERENCE as well (gen6 = 17 < 20 — a floor the frozen table cannot meet by construction; the pre-registration's "target ≥ 20 carves per family" belongs to the carving paragraph, i.e. the fresh families). Corrected to fresh-families-only BEFORE any record consumed a verdict; the measurements were bit-identical across both runs (same kernel, same inputs) — only the verdict gate changed.

## What this does NOT settle

- **Promotion is unchanged: `regime_probe` stays opt-in.** katgpt-rs 740/HISTORY require a default-path consumer, which still does not exist (the score_bench consumer is itself feature-gated). This experiment settles convergent validity, not promotion.
- Graded proximity, per-family significance beyond mean/d (no KS test — out of scope), and any selection-controlled reference design.

## Run + environment

```bash
CARGO_TARGET_DIR=/tmp/ood_fresh cargo run --example ood_fresh_family --features score_bench,latent_retrieval
```

Debug profile (deliberate — discriminative, not latency stats; the 702 convention), macOS M3 Max. Gates: `cargo clippy --examples --features score_bench,latent_retrieval -- -D warnings` clean; default-features `cargo clippy` clean. Zero new deps (syn + proc-macro2 `span-locations` + the latent stack were already present). No sibling repo was modified (both carved read-only at clean working-tree HEADs). No clippy oracle ran — the probe is oracle-free, which is precisely why it can ride every score-bench row.
