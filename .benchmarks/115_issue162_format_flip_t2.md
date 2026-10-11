# Bench 115 — Issue 162 T2: the formatting-churn instrument (format_flip)

**Status:** COMPLETE — T1+T2 recorded; T3 decided (NOT armed); T4/T5 not triggered.

**Date:** 2026-10-12 · **Box:** M3 Max (16P), AC power, load 8-15 (two sibling agents
building concurrently — a CPU-bound report lane, numbers are rate/count reads, not
latency bars; box state does not bear on them). Lane:
`cargo run --release --features latent_retrieval,basis_cert --example formatting_flip
-- --pairs <11 dump files>`.

## What ran (the instrument, landed as `examples/formatting_flip.rs`)

Report-only, deterministic end to end. Inputs: `--dump-trajectories` JSONL from 11
repo-local stores (riir-refine, riir-ai, katgpt-rs, riir-neuron-db, riir-chain,
riir-dapps, riir-game-sdk, riir-infer, riir-reflex, riir-train, seal-remake);
12,852 rows total → **1,935 Certified + usable fix pairs** after the tier filter
(`EvidenceTier::Certified` = verified-mechanical only) and the nonempty/changed
filters. Slice digest `c159e9fd2a9fb7ef…f6183` (BLAKE3 over the sorted pair texts;
raw texts stay machine-local, never committed).

## The three reads

### N1 — pair flip (before-span vs after-span of Certified mechanical fixes)

| family | fused (shipped) | full8 latent | struct4 latent | own-rule rank b→a | own-rule lost/seen |
|---|---|---|---|---|---|
| clippy (n=1935) | **44.1%** | 99.3% | 79.1% | 0.50 → 3.47 | 442/225 |

The shipped channel's top-1 flips under **44.1%** of compile-verified mechanical
fixes. The own rule (the fix's own lint) holds its fused top-8 slot for only ~36%
of pairs after the fix (mean rank 0.50 → 3.47; 442 pairs lost it outright) — the
fixed code is expected to look less like the rule's anti-pattern, so flip ≠ error;
the churn decomposition is N3's job.

### N3 — whitespace floor (token-identical mutators over the 141 committed eval fixtures)

| mutator | n | fused | full8 | struct4 |
|---|---|---|---|---|
| indent (+4 sp/line) | 141 | **1.4%** | 100.0% | 100.0% |
| trail (+3 sp/nonblank) | 141 | **1.4%** | 100.0% | 98.6% |
| gap (blank after `{`/`;`) | 141 | **0.0%** | 73.0% | 58.9% |

**Pure formatting churn on the shipped channel is 0.0-1.4%.** The fused channel's
BM25 term (whitespace-immune tokenizer) already carries the formatting invariance;
the 8-D latent term is hash-dominated and moves under ANY byte change (the
`LatentChoice::Structural4` zero-locality precedent, reconfirmed: even with the
hash dims zeroed, indent/trail reshuffle the byte histogram → the DFT dims move).

**Churn decomposition: 44.1% mechanical-fix flip − ~1.4% whitespace floor ⇒ ~43 pt
of churn rides the fix's TOKEN changes, not formatting.**

### N4 — the formatting basis (katgpt-core `basis_split`, Plan 625; reflex `basis_cert` contracts)

Population Σ over all 3,870 certified endpoints; spectrum
`[1.09e-1, 1.01e-1, 9.98e-2, 9.87e-2, 1.90e-2, 9.96e-3, 4.83e-3, 2.51e-4]`,
cond 4.3e2 — well-conditioned, whitener bounded. Reflex posture (explicit
`build_zca_whitener` + the `Whitener` accumulate arm — the arms are EXCLUSIVE;
the TotalCovariance-on-whitened-moment double-whitening artifact was reproduced
en route (k=2, gap 2e5) and is exactly the failure reflex documented). Shrinkage
floors: `SHRINKAGE_REL=1e-8` on Σ and on the moment diagonal. `τ_gap=4.0`. The
moment's even half fits; the odd half calibrates.

| run | fit | verdict | k | gap | pr | λ₁ share | calibration leakage | holdout leakage |
|---|---|---|---|---|---|---|---|---|
| family-split (fit=clippy) | n=1935 | SufficientVariation | 1 | 731.4 | 1.00 | **0.998** | 0.9824 | — (no second family exists) |
| rule-split (cross-generator) | 14 rules, n=633 | SufficientVariation | 1 | 692.7 | 1.00 | **0.998** | 0.9753 | **0.9852** (12 rules, n=1302) |

The family-level holdout has **no second family**: the workspace's certified
mechanical-fix population IS one family (clippy_lints — including the healer's own
custom lints `byte_char_slices`/`manual_checked_ops`/`chunks_exact_to_as_chunks`/
`double_parens`, which the census initially mis-flagged as perf/kernel ids; the
seed corpus is the authority). The issue's cross-generator holdout law is
instantiated at the RULE level instead: each corpus rule IS a generator; blake3
parity assigns 14 fit / 12 holdout rules.

**The verdict the numbers force:** the "variant basis" is ONE whitened direction
carrying 99.8% of the fit moment's energy — but only ~1.5-2.5% of held-out pairs'
energy (calibration 0.975-0.982 ≈ cross-rule holdout 0.985). That is the
outlier-mass signature: a handful of giant-Δ pairs dominate the moment; typical
pairs project almost nothing onto the direction, and what little does transfer is
not a formatting structure. **There is no transferable low-rank formatting
subspace in the 8-D modelless embedder — the pair differences are effectively
isotropic noise around one outlier axis.**

The span_embed 32-D token channel under the same real fixes: mean ΔL2 0.492, mean
cos 0.831 — the token-level channel holds its geometry where the hash channel
shatters.

## The T3 decision (what this bench exists to decide)

Issue 162 T3: "if formatting churn is material, opt-in projected dense KNN."
Both halves of the conditional are now measured:

1. Churn IS material on the raw latent channels (99.3% full8 / 79.1% struct4 pair
   flip) — but those channels are not the shipped path.
2. On the SHIPPED fused channel, churn is 44.1% under real fixes but only 0-1.4%
   under pure formatting — and the fix-induced churn has NO basis to project out
   (N4): a projected-KNN lane would remove a k=1 outlier axis carrying ~2% of the
   energy and change nothing else, while paying a matvec per query.

**T3 is NOT armed.** The projected-KNN primitive is refuted-for-this-channel by
T2's own evidence — the record says so, no code gates on a dead lane. The honest
churn remedies, should the 44.1% ever read as a problem, live in a token-level
embedder (the span_embed contrast row is the direction) — an owner-gated
embedder-swap decision, not a projection.

## Cross-run: the concurrent T1 freeze (the Issue-825 composition)

Mid-build, origin landed the sibling cycle-22 T1 (`cea4d679`):
`assets/format_pairs_v1.jsonl` — 4,213 usable pairs / 37 families, sha256-pinned,
with the recorded law that **T2 fits against this frozen slice, never a re-dump**.
This bench's population A (tier-Certified across 11 live stores) was mined
independently; the instrument gained `--tier-filter certified|success|all` (the
T1 in-band menu) and crossed both. Three populations, one instrument:

| population | n | fused flip | full8 flip | basis verdict (all runs) |
|---|---|---|---|---|
| A: tier-Certified, 11 stores (`c159e9fd…`) | 1,935 | 44.1% | 99.3% | k=1, λ₁ share 0.998, cal 0.975-0.982 ≈ holdout 0.985 |
| B1: T1 freeze, verified-lived (`64febbe8…`) | 281 | 68.0% | 99.6% | k=1, λ₁ 0.994, cal 0.9967 ≈ holdout 0.988-0.9926 |
| B2: T1 freeze, all pairs (`a14228c5…`) | 4,213 | 35.3% | 99.4% | k=1, λ₁ 0.999, cal 0.9951-0.9963 ≈ holdout 0.9939-0.9950 |

**The verdict is cross-validated:** an independently-mined, committed dataset
reproduces the same signature at every posture — the k=1 basis is a giant-Δ
outlier axis carrying ~0.4-2% of held-out energy, and the fused channel's churn
is token-change churn far above the 0-1.4% whitespace floor. Population B1's
higher fused flip (68.0%) is the doc_markdown×227 mass (text-heavy rewrites);
B2's lower (35.3%) includes the corpus-seeded buggy→fixed pairs the corpus was
AUTHORED to discriminate. The whitespace floor (N3) is population-independent
(the committed eval fixtures) and is the number that separates formatting churn
from fix churn in every row.

Composition notes: B's 30 `unattributed` rows are lint_keys outside every seed
corpus (disclosed, never guessed); B's family census confirms the T1 finding
that the certified lived mass is clippy-only on this box (rust_perf n=1-10,
THIN-flagged). The frozen artifact committed the pair texts (the cycle-22 owner
call); this bench's population-A texts stay machine-local behind their digest.
Two independent populations agreeing on the N4 refusal is the recorded
preference: "two benches that agree are stronger than either alone."

## Hygiene + disclosures

- Report-only: no lane armed, no production path touched, off-flag behavior is
  the only behavior.
- The certified population is machine-local store state (11 repos, this box);
  the slice digest pins it. Re-running after more healer work shifts the numbers
  — the benchmark pins THIS slice.
- The `- [ ]` ticks in `.issues/162`: T1, T2 done; T3 decided here; T4/T5 not
  triggered (T4 gates a lane that did not arm; T5's churn feature rides the same
  pair stream and is available to mining triage if wanted — unclaimed).
- `MIN_REVERSE_FIT`/`THIN_HOLDOUT`/`MIN_FIT_PAIRS` = 8/32/8 disclosed in-source.
- The example compiles warning-clean at its required-features posture;
  `basis_cert` forwards `katgpt-core/basis_split` (mirrors reflex's feature name).
