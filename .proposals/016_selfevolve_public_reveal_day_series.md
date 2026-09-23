# Proposal 016 — Public self-evolve reveal: the healer quality day-series on the kat-service dashboard

Status: **REVISE round 2 — Claude verdict 2026-09-23** (round 1 B1–B4/C1–C3 accepted as closed; Q1/Q2 resolutions stand). FOUR new blocking findings, two of which CORRECT the round-1 verdict's own answers: **D1** the ladder headline has only THREE values over 27 days (measured running max: 08-19→1, 09-06→7, 09-11→8, FLAT for the last 12 days) and its largest step is the day the `generation` FIELD arrived, not a learning event — the round-1 Q3 answer argued from structure and never measured resolution; also `hardest_admitted` is unqualified and reads as the GLOBAL max (a flat 8), it must be the running max over ts ≤ day-end. **D2** `corpus.rules_total = vocab_size` is the wrong quantity under the wrong name — `vocabulary().len()` (score_bench/mod.rs:749) is the frozen bench's LINT-NAME coverage (~75), while the corpus card beside it derives real corpus rule counts per domain (clippy alone = 94 at HEAD); two numbers labeled "rules" on one page differing ~12x, and the field is polysemous (heal.rs:5752/5964 write `members_total` on the ladder lanes). **D3** B4 pinned UTC but the neighbouring card is pinned ICT by explicit owner anchor (mine_corpus_git_days.py:11) — PARSE as UTC (measured) and BUCKET in ICT are separate decisions the text conflates. **D4** C1 is not fixed: `ts: 1789963400` is 2026-09-21T04:03 UTC against its own `date: 2026-09-22` (the cited run is 1790095454), and the `frontier` block is still the round-1 fabrication (real latest row: gen 7 / pop 19 / band 0.05263158 / sat 2, not 8/17/0.056/2) — the `quality` block alone is real. Non-blocking: E1 the feed is NOT ts-sorted (1 inverted pair at idx 10) so "latest run of the day" must sort, not tail; E2 B1's untagged rule is empirically clean (all 45 untagged rows precede the first tagged) and has shipped precedent in traj_store.rs:255's serde default; E3 a day with no gen-1 run is unspecified (27/27 have one today, so forward risk only — the door must accept `quality` absent).
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: riir-dapps Proposal 004 (fix-stats lane) × riir-dapps Issues 102+104 (corpus-days signed sync) × riir-clippy Issues 037/039 (self-evolve stores + Elo) × riir-clippy Issues 089/102 (frontier snapshots + fade)
Related: [Research 203 — RRSI](../.research/203_rrsi_regularized_harness_evolution.md), [riir-dapps .proposals/004](../../../riir-dapps/.proposals/004_fix_stats_lane_consent_gated.md), katgpt-rs Research 440 (AIDE2), arXiv:2607.07663

## TL;DR

The healer's self-evolve is measured every day and visible to no one: 160 score-bench
runs since 2026-08-19 (heal 84.6% → 96.4%, created-warnings 5.8% → 1.8%, safety 100%),
Elo stores across 10 repos, frontier band-mass snapshots, trajectory rows — all local
jsonl/kv artifacts. The public dashboard shows only VOLUME (corpus contribution graph,
fixstats totals). This proposal adds the QUALITY axis: a nightly, Ed25519-signed daily
aggregate pushed through a new `POST /admin/healqual/days` merge door and rendered as
a "Self-evolve" card whose HEADLINE is the **hardest-generation-admitted ladder** — a
monotone, admission-gated difficulty step function — with the generation-1 heal% curve
(the one fixed external evaluator) and the born-warning decline as companions. Third
instance of the corpus-days lane: same key, same allowlist var, same bounded registry,
new domain separator; the collector is pure Python over plain files (never
cargo-builds in the nightly). Modelless, zero GPU, zero new deps.

## The problem this solves

The owner's question — "is the healer getting smarter each day?" — is answerable from
data we already measure, but the answer lives in `.benchmarks/score_history.jsonl`
(riir-clippy, 160 runs), `.benchmarks/frontier_snapshots.jsonl` (089 T4 emission), and
the per-repo trajectory stores (037 store-first). None of it is rendered anywhere.
Meanwhile the public site's only growth surface is the corpus contribution graph
(git-mined ADDS — volume) and the fixstats strip (epoch totals — volume). The owner
looked at devnet and asked where the self-evolve is. The cost of revealing it is small
because every mechanism it needs already shipped once: signed nightly sync (Issues
102+104), the publisher-key posture, the bounded door, the server-rendered card.

## The proposed design

### The day row (the contract — all values real, from the 2026-09-22 feed)

```json
{
  "date": "2026-09-22",
  "ts": 1789963400,
  "machine": "",
  "quality": {
    "heal_rate": 0.963636, "created_rate": 0.018182, "parse_safety": 1.0,
    "generation": 1, "n_ran": 35, "edits_total": 69
  },
  "corpus": {"rules_total": 75},
  "ladder": {"hardest_admitted": 8},
  "frontier": {"generation": 8, "population": 17, "band_mass": 0.056, "saturated": 2}
}
```

Field rules (each closes a verdict finding):

- **`ts` = the selected bench RUN's own time** (B2) — never collection time. Nightly
  re-posts of a settled day are equal-ts no-ops; only the day still in progress moves.
  The whole series may be re-derived statelessly from the feeds each run and posted in
  full (the corpus-days idempotent shape) — newer-wins then touches exactly the
  in-progress day.
- **Timezone pinned: UTC, naive-parsed** (B4). The feed writes naive ISO with no
  offset; measured against file mtimes it IS UTC. The collector parses it as UTC and
  buckets days in UTC; a near-midnight fixture (run 23:58 UTC on a UTC+7 box)
  pins the bucketing.
- **`quality` is pinned to generation 1** (B1) — the longest continuous series (87
  tagged runs) and the ONLY lane that is a fixed external evaluator, which is what the
  card's bounded-refinement caption claims. Later generations are different benches,
  not rungs of one; a lane-switching line would plot "which lane ran", not quality.
  Rows carrying no `generation` field (45 early rows, pre-field era) join the gen-1
  series ONLY IF their ts precedes the first tagged row (provably pre-field); any
  untagged row after that point is skipped LOUD with a count in the sync output.
- **`corpus.rules_total`** = the run's `vocab_size` (the compiled corpus size the bench
  measured). Per-domain breakdown is deliberately NOT in v1 — it would need a Rust
  build (C3); the field set stays open for a later Rust collector.
- **`ladder.hardest_admitted`** = max `generation` in score_history.jsonl (Q3). A new
  generation's append is admitted only by measuring harder than v1 (the generation
  ladder's own admission gate), so each step is a real, ungameable difficulty increase
  and the series is monotone by construction. This is the headline.
- **`frontier`** = latest frontier_snapshots.jsonl row (pure file read).
- `elo` and per-repo `stores` counts are RESERVED, absent in v1 (C3 — they need the
  Rust store reader; caveat 4 already bars them from the headline).

### Transport — third instance of the corpus-days lane

- **Signing**: the SAME `.corpus_sync.key`, message = `kat:healqual:v1:` domain ‖
  canonical row JSON; the server verifies against the SAME public
  `KAT_CORPUS_SYNC_PUBKEY_HEX` allowlist var — no new secret, cross-domain replay
  impossible by the domain separator. Bearer fallback (`KAT_CRANK_TOKEN`) mirrored.
- **Door**: `POST /admin/healqual/days` → the mining plane's merge route.
  **Merge = strictly-newer-ts wins per date; equal-ts = no-op** (Q1). Unlike corpus
  adds, quality rates legitimately move DOWN on a re-run — monotone max would ratchet
  a rate upward and lie — so freshness keys on the run's own ts (inside the signed
  bytes). The safety corpus-days gets algebraically (max is order-independent, trusts
  no publisher field) this lane gets from a self-asserted clock, which is the weaker
  property — closed by two bounds: **`ts ≤ now + skew` REJECTED, never clamped** (B3 —
  a clock skew otherwise wedges a date permanently), and the B2 provenance rule.
  Contract arms: newer-wins, equal-ts no-op, stale-ts no-op, future-ts 400.
- **Bounds** (the Issues-102 discipline): day ∈ [2026-08-19, today+1] (first bench
  run), ≤ 400 rows/post, rates ∈ [0,1], counts ≤ 1e6, `ts ≤ now+skew` (reject), row
  ≤ 1 KB. Registry append-only, no purge door.
- **Collector — pure Python over plain files (C3)**: `riir-dapps/scripts/mine_healqual_days.py`
  (sibling of `mine_corpus_git_days.py`), run INSIDE the riir-clippy checkout, reads
  score_history.jsonl + frontier_snapshots.jsonl only. `healqual_sync.sh` (sibling of
  `corpus_days_sync.sh`) signs and POSTs. **The nightly never cargo-builds** — the
  stale-binary class is structurally absent because no binary is involved; the sync
  script is the publisher and `.corpus_sync.key` is its authentication.
- **Cadence wiring**: `riir-clippy/scripts/ops_nightly.sh` gains the leg BESIDE the
  existing corpus leg (C2 — the launcher lives in riir-clippy), same non-fatal
  `WARN … continuing` posture.

### Consent posture (Q2 — reversed from the first draft)

**v1 is NOT consent-gated — deliberately.** The row is the project's own bench
results, from files in the owner's own repo, published under the owner's own key:
structurally the corpus-days PUBLISHER posture (`corpus_days_sync.sh` has no consent
check; the key is the auth), not the fixstats third-party-telemetry posture. Gating on
`--stats on` would couple the dashboard's liveness to a per-machine telemetry switch
nobody would think to flip back on. Consent becomes load-bearing at Phase 4's fleet
join — and Proposal 004 already names the mechanism: a future work-stat lane joins the
record by appending a **`lane: "healqual"`** tag, never an implicit ride on
`lane: "fix"`.

### Render — the "Self-evolve" card (Q3 ladder as headline)

- `GET /mining/healquality` — public JSON twin `{days:[…], provenance}`.
- Front-page card, server-rendered SVG, zero JS, ≤ 90 days, four series with distinct
  visual grammar:
  1. **HEADLINE: the ladder** — `hardest_admitted` step function (monotone, quality-shaped,
     admission-gated; each step answers "smarter how?").
  2. **gen-1 heal%** polyline — the pinned fixed-evaluator curve; converged at ~96.4%
     and that convergence IS the achievement the caption describes.
  3. **born-warning decline** — `created_rate` 5.8% → 1.8%, a genuinely falling
     quality curve that reads as progress without the heal% line needing to rise.
  4. **network fixes** bars (from existing fixstats views) — visually distinct VOLUME,
     never merged into the quality series.
- Caption: *"ladder = hardest benchmark generation admitted (each step measures
  harder); curves measured on the frozen gen-1 bench — bounded self-refinement
  (arXiv:2607.07663), convergent and evaluable; bars = volume from consenting fleet
  machines."*

### The honest framing (why a growth curve is legitimate here)

arXiv:2607.07663 splits self-improvement into **bounded** (fixed external evaluator —
convergent, evaluable) and open-ended (self-referential — neither). The gen-1 bench is
a fixed external evaluator, so its curve and the difficulty ladder above it are the
evaluable kind. The caption says which kind it is — and B1's pin is what makes that
sentence TRUE.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **gen-1 heal% has converged** (96.4%) and will read flat. That is by design: the
   ladder carries "smarter each day", the flat line certifies the frozen evaluator,
   the declining created-rate shows precision still improving. If the owner wants the
   per-lane small-multiples (each generation's climb since birth) as v1 too, say so
   before Phase 3 — the data supports it, the first draft's caption did not.
2. **Single-source quality.** All rows come from the dev box (the only machine that
   runs score-bench). `machine` is empty in v1; the field is the fleet-join seam.
   Card copy must not imply fleet-wide quality measurement before a second machine
   pushes.
3. **heal% certifies bench-fit, not real-world fix quality.** The corpus is the model.
   The real-world axis on the same card is fixstats volume + mining adoption; the two
   stay visually and semantically distinct — never one merged "score".
4. **Elo/band-mass are thin-n** (27 rated rules, band 1/17) — reserved out of v1
   entirely (C3), and barred from any headline even when a Rust collector lands. The
   R135/Bench-047 law applies; Issue 130's T0 owner call may change what admission
   evidence even is.
5. **New public door = new surface.** Inherited mitigations (signed, allowlisted
   pubkey, bounded, no purge, fail-closed) plus the four ts-merge contract arms; full
   battery before the card lights; manual devnet deploy.
6. **Consent scope is fixed by construction**: v1 publishes only the owner's own bench
   artifacts under the owner's own key. ANY future row sourced from another party's
   machine arrives through the Phase-4 lane tag and ITS OWN consent decision — the
   architecture makes the silent extension impossible rather than trusting memory.

## Fusion lineage

- **riir-dapps Issues 102+104** — the signed nightly sync lane (corpus days): key
  handling, envelope, bounded door, front-page card lighting, the launcher this
  extends.
- **riir-dapps Proposal 004** — the lane-tag mechanism Phase 4 uses, and the privacy
  law (labeled accounts only) the bars series already honors.
- **riir-clippy Issues 037/039/089/102** — the measurement substrate: trajectory
  stores, Elo replay, frontier snapshots, score-history fields. The collector only
  reads what these already emit.
- **katgpt-rs Research 440 (AIDE2) + Research 203 (RRSI)** — the measurement frame:
  self-improvement progress is evaluable when the evaluator is fixed and external.

## GOAT gate

- **G1 (correctness):** golden day-row (real values, C1-consistent) + golden signed
  payload both repos; merge arms: newer-wins, equal-ts no-op, stale-ts no-op,
  **future-ts 400 (B3)**, ceiling violations 400, malformed 400, bad signature 401,
  bearer fallback; collector fixtures: **near-midnight UTC bucketing (B4)**, untagged-
  pre-field-era inclusion + post-era loud skip (B1), settled-day equal-ts idempotence
  (B2), empty-feed loud-empty (never fabricated).
- **G2 (perf):** nightly push ≤ 5 s (two small jsonl files, pure Python); card render
  O(days ≤ 90); door ≤ corpus-days door cost.
- **G3 (no-regression):** corpus-days lane and fixstats lane byte-untouched (their
  contract pins hold); worker lib counts bump ADDITIVE; no change to the heal path,
  the corpus, any floor, or any consent record.
- **G4 (bounded):** one bounded registry; zero GPU; zero new cargo deps; the nightly
  builds NOTHING (C3 by construction); modelless.

## What ships now vs deferred

### Ships first — riir-dapps Phases 1–2
- `mine_healqual_days.py` + `healqual_sync.sh` + `kat_corpus_sign --domain healqual`
  (publisher half — pure Python + the existing sign bin mode).

### Deferred — riir-dapps Phase 3
- The `/admin/healqual/days` door, registry, `/mining/healquality` twin, front-page
  card, devnet manual deploy. One dapps plan citing this proposal; corpus-days door
  is the template.

### Deferred — riir-clippy Phase 4
- The `ops_nightly.sh` leg (launcher lives here, C2) and the `lane: "healqual"` fleet
  join (Proposal 004 mechanism, new consent decision at that point).
- Rust store reader for `elo`/`stores`/`by_domain` fields (reserved; never headline).

### Explicitly NOT shipped by this proposal
- No on-chain commitment of the series (statroot covers mining telemetry; healqual can
  join a later stats-root stamping).
- No per-machine fleet quality pushes in v1; no `--stats` coupling.
- No change to scoring, admission, corpus lifecycle, or the healer's heal path.

## Phased rollout (sketch — a plan would expand this)

### Phase 1 — riir-dapps publisher
- [ ] T1.1 `mine_healqual_days.py`: read the two jsonl feeds → full day series; B1/B2/B4 rules; loud-empty
- [ ] T1.2 golden fixtures + `healqual_sync.sh` + sign mode `--domain healqual`

### Phase 2 — riir-dapps door
- [ ] T2.1 door + newer-ts merge + registry + full contract battery (incl. future-ts 400)

### Phase 3 — reveal
- [ ] T3.1 `/mining/healquality` JSON twin + front-page SVG card (ladder headline)
- [ ] T3.2 lib-count pins bump; devnet manual deploy; curl-verify all postures

### Phase 4 — riir-clippy cadence + fleet
- [ ] T4.1 `ops_nightly.sh` leg beside the corpus leg (WARN-continuing posture)
- [ ] T4.2 (deferred, new consent decision) `lane: "healqual"` fleet join; Rust store fields

## Risks

1. **Public-surface risk**: a malformed-tolerant door is a spam magnet — bounds +
   signature + fail-closed are load-bearing; contract tests prove each arm.
2. **Clock-trust risk** (Q1): the ts merge trusts a self-asserted clock where
   corpus-days trusts algebra — accepted deliberately (rates must be able to fall),
   bounded by B3 + the signed bytes; the residual is a publisher misconfig, not an
   attacker win (they'd need the key).
3. **Metric-optics risk**: a flat gen-1 line next to a rising ladder could confuse —
   mitigated by distinct visual grammar + caption (caveat 1).
4. **Architectural risk**: none new — cross-repo invocation (corpus-days precedent),
   no cargo dep, dapps direction gate untouched.

## Out of scope

On-chain stamping; real-time streaming; authenticated per-visitor views; per-machine
fleet pushes in v1; any change to the healer's heal path; mainnet enablement (devnet
first, per every prior lane).

## References

1. arXiv:2607.07663 — *Recursive Self-Improvement in AI: From Bounded to Open-Ended* — the bounded-refinement-is-evaluable frame (distilled here).
2. arXiv:2609.11873 — *The Last AI Built by Humans* (RSI survey) — in-house as [Research 167](../../../katgpt-rs/.research/167_RSI_Survey_Autonomy_Ladder_Positioning.md) (cited).
3. katgpt-rs Research 440 — AIDE2 recursive self-improvement, measured PASS (cited).
4. Lilian Weng, *Harness Engineering for Self-Improvement* (2026-07) (cited-only).
5. riir-dapps Proposal 004 + Issues 102/104 — the lanes this instantiates (in-house precedent).

## TL;DR

Ship it with the revised series set: ladder headline (admission-gated, monotone,
ungameable), gen-1 pinned heal% (the caption becomes true), created-rate decline,
distinct volume bars; publisher posture without a consent gate; pure-Python collector
that never builds; ts merge with the future-ts ceiling. Next action: round-2 verdict →
owner GO → dapps plan for Phases 1–3; Phase 4 leg in this repo's launcher.
