# Bench 048 — Issue 042: the self-collapse monitor (measured) — R346's PoC at our scale

**Date:** 2026-08-25
**Verdict:** **NOT ELEVATED — the RAG-collapse transfer does not hold at our
scale/params.** The monitor ships anyway (cheap standing tripwire + audit
columns); the negative is recorded in [R346 §"PoC Addendum"](../riir-ai/.research/346_rag_collapse_self_authored_retrieval.md).

**Harness:** `src/collapse_monitor.rs` (module + 7 tests; the production
measurement is env-gated `RIIR_COLLAPSE_ENVELOPE` / `RIIR_COLLAPSE_STORE_ROOT`
— copies, never live stores) + the `--collapse-report` bin mode feeding
`self_evolve_audit.sh` §1c.

```bash
# always-green CI part (synthetic conditions + determinism):
cargo test --features latent_retrieval --lib collapse_monitor
# production measurement (on /tmp COPIES of the stores):
cp -R /Users/katopz/git/<repo>/.heal /tmp/issue042-stores/<repo>/.heal   # key file REQUIRED
RIIR_COLLAPSE_ENVELOPE=/tmp/envelope.json RIIR_COLLAPSE_STORE_ROOT=/tmp/issue042-stores \
  cargo test --features latent_retrieval --lib collapse_monitor_production -- --nocapture
# the audit surface (read-only, same open-replay class as --dump-trajectories):
RIIR_HEAL_TRAJ_ROOT=<repo> cargo-heal --collapse-report
```

## Setup

- **Instrumented loop (the REAL one, not a reimplementation):** for each
  fixture span — `LatentQuery::new(rule, buggy)` (heal Step 1's exact query
  construction) → `LatentFixMemory::retrieve_into(&q, 5, …)` (Step 1's exact
  call) → `TernaryDrafter::draft_fix` (Step 2's exact call) → heal's Step-3
  exact-code dedup (mirrored one-line) → `HealPipeline::heal` itself for the
  selection verdict (`HealResult.source`). The measured memory is installed
  into the pipeline via `*pipeline.memory_mut() = from_trajectories(…)`.
- **Fixture set:** Issue 017's `CLIPPY_SPANS` — 48 real-clippy-verified
  hand-authored variant spans over the 40-lint corpus (the production input
  regime: real-world spans are never exact corpus templates). Secondary
  regimes measured: exact-corpus spans (40) + the dominant rule's spans (2).
- **Corpus:** the real 40-entry clippy seed corpus (matched across all
  conditions).
- **Store inventory (measured 2026-08-25):**

| Store | Rows | Notes |
|---|---|---|
| envelope `~/.cache/riir-clippy/memory.json.migrated` | **8,241** | the sweep campaign's aggregate (8,235 Heuristic + 6 Certified; 88.4% `uninlined_format_args` = 7,286) — the issue's "~8,235-trajectory production store"; PRIMARY condition |
| riir-game-sdk `.heal/trajectories.kv` | 8,117 | the envelope's migrated twin (038-T3 one-time import) |
| riir-ai | 101 | |
| riir-mmorpg-examples | 24 | |
| riir-clippy | 8 | |
| riir-chain / riir-dapps | 2 each | |

**Why the envelope is primary:** exact count match with the issue's number +
chronological insertion order (dose-response prefixes = store age under the
campaign's rule-coverage growth). The per-repo stores ride along as the
store-backend validation (game-sdk's numbers reproduce the envelope's).

**Metric definitions (precise):**
- *pool memory-fraction* — Memory candidates / total Step-3 pool candidates
  (drafter + ≤5 memory, post-dedup), pooled + per-span mean.
- *memory win-rate* — Memory-sourced share of SERVED `heal()` results (the
  paper's citation-rate analog).
- *per-span top-k entropy* — normalized Shannon entropy `H/log₂(n)` over the
  rule-id distribution of the retrieved top-k (n = retrieved count; single
  rule → 0, uniformly-spread n-rule pool → 1), averaged over spans with ≥1
  retrieval.
- *pooled rule entropy* — same normalization over ALL retrieved trajectories
  pooled across spans.
- *same-rule rate* — retrieved trajectories whose rule equals the query's.

**Determinism:** double-run bit-identical on every condition (asserted in the
test + printed `[deterministic]` by the bin mode). No RNG, no wall-clock;
insertion-order iteration.

## Two structural findings (before any numbers)

1. **The dry `--suggest` path cannot collapse.** `LatentMatcher` retrieves
   from the corpus `RuleIndex` only — the trajectory store is never
   consulted. Collapse risk on suggest is structurally zero (verified by
   code path read: `bin/heal.rs` suggest pass + `draft/latent_matcher.rs`).
2. **The CLI `--fix` path records but never reads back.** `EvolveRecorder` /
   `record_traj` write trajectories; no CLI retrieval consults them. The
   memory-READING loop is the API surface (`HealPipeline::heal`) — used by
   the examples/demos/tuning loop, not the CLI's proposer-gated fix path.
   **The collapse risk is LIVE at the API level, latent at the CLI level.**
   The monitor is the tripwire for the day a consumer wires retrieval in.

## Measured (the tables)

**Regime A — variant spans (production input regime, 48 spans):**

| Condition | rows | pool mem-frac | mem win-rate | per-span H | pooled H | same-rule | distinct |
|---|---|---|---|---|---|---|---|
| fresh (empty) | 0 | 0/0 — **declined 48** | 0% | — | — | — | 0 |
| envelope[:100] | 100 | 100% | 100% | 0.123 | 0.050 | 4.2% | 2 |
| envelope[:1000] | 1,000 | 100% | 100% | 0.141 | 0.116 | 10.4% | 5 |
| envelope[:4000] | 4,000 | 100% | 100% | 0.358 | 0.280 | 22.5% | 12 |
| **envelope[8,241]** | **8,241** | **100%** | **100%** | **0.322** | **0.345** | **33.3%** | **18** |
| store:game-sdk | 8,117 | 100% | 100% | 0.350 | 0.341 | 32.5% | 18 |
| store:riir-ai | 101 | 100% | 100% | 0.517 | 0.262 | 12.1% | 6 |
| store:riir-clippy | 8 | 100% | 100% | 0.246 | 0.080 | 4.6% | 2 |
| store:riir-mmorpg | 24 | 100% | 100% | 0.168 | 0.075 | 3.3% | 3 |
| store:chain / dapps | 2 | 100% | 100% | 0.000 | 0.000 | 2.1% | 1 |

**Dominant-rule regime** (`uninlined_format_args` spans, full envelope — the
paper's collapse regime if it exists anywhere, 7,286 same-rule
self-trajectories): **top-k H = 0.000, same-rule = 100%, distinct = 1.**

**Regime B — exact-corpus spans (the only regime with pool competition):**

| Condition | pool mem-frac | selection | head-to-head |
|---|---|---|---|
| fresh | 0/15 (drafter fires 15/40, declined 25) | drafter 15 | — |
| envelope | **93.0%** | **mem 33 / drafter 7** (mem win 82.5%) | drafter present on 15: **drafter 7, memory 8 (53%)** |

## Interpretation — why NOT ELEVATED despite 100% memory-fraction

1. **The source-share saturation is a VACUUM, not a crowding-out.** The L1
   corpus path declines on ALL 48 variant spans (`TernaryDrafter::fallback`
   returns `candidates[0]`'s template for every rule; the Issue-010 identity
   guard then declines it — Stage-1 decline-by-default, the designed
   behavior). With no external candidate in the pool, memory fills 100% of
   it at ANY store size ≥ 2 rows. The paper's mechanism requires self content
   to OUT-COMPete external content; on the production input regime there is
   no external content to out-compete. The fresh-vs-aged comparison the
   issue specified (0% vs 100%) is real but measures the drafter's
   abstention, not retrieval bias.
2. **The diversity-collapse prediction is REFUTED at our scale.** Top-k rule
   entropy RISES with store age/size (per-span 0.123 → 0.322, pooled 0.050 →
   0.345, distinct 2 → 18). The paper predicts the pool's diversity SHRINKS
   as self-content accumulates; ours grew — the campaign's rule coverage
   grew with the store (format-args first, then the structural lints), and
   nothing became unreachable. The paper's own caveat ("a lack of diversity
   can indicate real consensus") covers the one collapse-shaped reading: the
   fmt-args spans retrieve a same-rule monoculture (H=0) because the store
   legitimately holds 7,286 format-args fixes — high-precision on-topic
   retrieval, not entity invisibility (every other rule stays reachable:
   distinct=18 across the fixture set).
3. **Where competition exists, memory wins on EVIDENCE, not similarity.**
   In Regime B's 15 head-to-head spans, memory wins 8 — and the store's
   verdict-carrying buckets are exactly 9 (8 successes + 6 Certified, all
   `uninlined_format_args`/`match_bool`): BetaPosterior belief for a counted
   trajectory beats the drafter's flat 0.32 (unseen) — the designed
   reliability axis. Ties (no counts on either side) go to the DRAFTER
   (first-candidate tie-break). The similarity advantage the paper measured
   does not surface in selection because selection reads
   evolution+reliability, never the retrieval score.

**The honest composite:** source-share elevated-but-structural; diversity
not reduced (refuted); selection wins evidence-driven. Negative transfer at
our scale/params — recorded in R346.

## Gates

- **G1 (determinism):** double-run bit-identical signatures on every
  condition — PASS (asserted).
- **G2 (cost):** `--collapse-report` on the 8,117-row store incl. the
  double-run: **139 ms** release. Stateless (no migration, no puts; the
  O(n²) envelope-load concern is bypassed by `from_trajectories`). PASS.
- **G3 (no-regression):** `--features latent_retrieval --lib` **760/0/1i**;
  default `--lib` **677/0**; `cargo clippy --all-targets --features
  latent_retrieval -- -D warnings` **0** (both feature states). PASS.
- **G4 (read-only):** production stores copied to `/tmp` for measurement
  (`.heal/key` MUST ride along — without it the store decrypts to garbage
  and every row reads "corrupt"); the bin mode opens with no migration and
  writes nothing. PASS.
- **Synthetic canary (always-green):** the maximal-self-similarity store
  (same-rule trajectory per span) elevates pool memory-fraction vs fresh —
  the instrumentation can SEE the signature it watches for. PASS.

## Honest caveats

- **Store provenance:** the envelope is one campaign's history (88%
  format-args) measured against a clippy fixture set — the numbers describe
  THIS store, not a steady-state. Dose-response prefixes conflate age with
  rule-coverage growth (chronological order); they are NOT a controlled
  age-variation.
- **Load:** measurement runs were not under GPU/load gates (CPU-only,
  deterministic — load cannot move the numbers; timing number (139 ms) is
  informational).
- **The fmt-args dominance means the entropy axis is near its floor for the
  biggest rule by construction** — a future store dominated by ONE rule
  would show pooled entropy falling for coverage reasons, not collapse.
  The monitor's trend reading (audit §1c) must pair the two axes: fraction
  rising + entropy falling TOGETHER is the signature; either alone is not.
- **The audit script needs a `latent_retrieval` binary build** for §1c to
  render (a stale binary prints the rebuild note; the canonical
  `~/.cargo/bin/cargo-heal` symlink target rebuilds on the next legit
  release build — not forced here to avoid target-dir contention with the
  concurrent sibling session).

## T3 — owner-gated mitigation design note (numbers first; NO code change)

The options the issue named, evaluated against the measured regime:

**(a) Self-fraction cap on any top-k (diversity quota).** The paper's
+26pp-survives-quality-controls finding argues structural caps beat per-item
quality discounts — a quota cannot be out-scored by any single strong
self-candidate. BUT our measured regime says the cap would fire on EVERY
real-world span (pool memory-fraction is 100% by vacuum — the quota would
force including drafter candidates the drafter declined to produce, i.e.
un-collapse an abstention). **Cost at measured numbers: the cap as specified
is inapplicable to the vacuum regime — it can only be defined where the pool
has both sources (Regime B: cap at, say, 80% would bind on 0 of 40 spans —
93% pooled but the 7 drafter candidates already exceed a 1-in-5 quota).**

**(b) Origin discount composing with trust (`origin(self) ∧ !certified →
discount`; certified-self neutral).** The composition preserves the
reliability axis's wins (the 8 head-to-head wins are Certified buckets —
neutral under (b), correctly). But the discount acts on the RETRIEVAL score,
and selection never reads the retrieval score (it reads
evolution+reliability) — so in the CURRENT loop (b) changes which
trajectories are retrieved without changing which fix is served, EXCEPT via
the dedup/pool composition. **At measured numbers: (b) is a no-op on the
served answer in Regime A (memory-only pools stay memory-only) and in
Regime B it would demote the 8 evidence-wins' retrieval rank while leaving
their selection rank intact — pure instrumentation noise until a consumer
reads retrieval order.**

**(c) Monitor-only.** Matches every measured fact: the collapse mechanism is
not present (vacuum, not competition); the one live risk channel is FUTURE
wiring (a consumer reading retrieval order, or verdict accumulation making
memory's reliability systematically dominate). The monitor + audit columns
watch exactly those two (fraction+entropy trend; win-rate vs
verdict-carrying-bucket count).

**Recommendation: (c) monitor-only, with a concrete re-arm trigger** —
promote to (b) [origin discount, certified-neutral] the day a production
consumer reads retrieval ORDER (not just membership), or when the audit's
mem-win-rate on exact-corpus spans exceeds the verdict-carrying-bucket share
by a margin (i.e. similarity, not evidence, starts winning head-to-heads).
Do NOT adopt (a): in the vacuum regime it mandates serving candidates the
corpus path declined to produce — the opposite of the Stage-1
decline-by-default design that makes the current loop safe.

## What shipped

- `src/collapse_monitor.rs` — the monitor (measure + entropy + store
  bridges + 7 tests incl. the synthetic canary + env-gated production run).
- `LatentFixMemory::from_trajectories` — O(n) batch constructor (caller
  guarantees dedup).
- `--collapse-report` bin mode (read-only; store-or-envelope + fresh
  baseline + `[deterministic]` verdict).
- `self_evolve_audit.sh` §1c — per-store memory-fraction + top-k entropy +
  mem-win-rate lines (Issue 037 A-axis degradation feed).
- This bench + R346 §"PoC Addendum" (riir-ai) + Issue 042 resolution.
