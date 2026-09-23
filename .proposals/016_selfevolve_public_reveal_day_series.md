# Proposal 016 — Public self-evolve reveal: the healer quality day-series on the kat-service dashboard

Status: **REVISE — Claude verdict 2026-09-23** (pending owner GO). Transport/GOAT/fusion-lineage halves ACCEPTED (verified: corpus_days_sync.sh + kat_corpus_sign + the monotone-max door exist as described; the 160-run feed's 84.6%→96.4% / 5.8%→1.8% / 100% claims all check out). FOUR blocking revisions before Phase 1 code: **B1** the v1 series must PIN generation 1 — "latest generation lane that ran" plots a lane-switch sawtooth (measured: gen1x87, gen3x4, gen4x3, gen5x4, gen6x6, gen7x9, gen8x2, 45 rows with no generation field) and falsifies the card's own fixed-external-evaluator caption; **B2** `row.ts` must be the selected bench RUN's time, not collection time, or newer-wins degenerates to whole-series last-write-wins and one bad collector run overwrites good history; **B3** bounds must add a future-ts ceiling (`ts <= now+skew`, reject) or a clock skew wedges a date permanently — a mode monotone max cannot have; **B4** pin the day bucketing timezone (the feed writes naive ISO with no `Z`; measured UTC, on a UTC+7 box whose nightly publishes an in-progress UTC day). Plus C1 the example row is hand-typed and inconsistent (by_domain sums 870 vs rules_total 927; `n_ran` collides run-count with the feed's per-run field), C2 `ops_nightly.sh` lives in riir-clippy not riir-dapps, C3 the Rust snapshot mode inherits the measured-twice stale-binary class. Q2 answered NO-consent for v1 (this is the corpus-days publisher posture, not the fixstats telemetry posture); Q3 answered LADDER-as-headline.
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: riir-dapps Proposal 004 (fix-stats lane) × riir-dapps Issues 102+104 (corpus-days signed sync) × riir-clippy Issues 037/039 (self-evolve stores + Elo) × riir-clippy Issues 089/102 (frontier snapshots + fade)
Related: [Research 203 — RRSI](../.research/203_rrsi_regularized_harness_evolution.md), [riir-dapps .proposals/004](../../../riir-dapps/.proposals/004_fix_stats_lane_consent_gated.md), katgpt-rs Research 440 (AIDE2), arXiv:2607.07663

## TL;DR

The healer's self-evolve is measured every day and visible to no one: 160 score-bench
runs since 2026-08-19 (heal 84.6% → 96.4%, created-warnings 5.8% → 1.8%, safety 100%),
Elo stores across 10 repos, frontier band-mass snapshots, trajectory rows — all local
jsonl/kv artifacts. The public dashboard (devnet.ai.gist.rs) shows only VOLUME (the
corpus contribution graph, fixstats totals). This proposal adds the missing QUALITY
axis: a nightly, consent-gated, Ed25519-signed daily aggregate — score-bench quality,
corpus breadth, frontier band-mass, Elo, store totals — pushed through a new
`POST /admin/healqual/days` merge door and rendered as a growing multi-series
"Self-evolve" graph card beside the corpus card. The design is a third instance of the
two proven lanes (corpus days, fixstats): same key, same allowlist var, same
bounded-registry pattern, new domain separator. Pure aggregation, zero GPU, modelless.

## The problem this solves

The owner's question — "is the healer getting smarter each day?" — is answerable from
data we already measure, but the answer lives in `.benchmarks/score_history.jsonl`
(riir-clippy, 160 runs), `.benchmarks/frontier_snapshots.jsonl` (089 T4 emission), the
per-repo trajectory stores (037 store-first) and the Elo replay (039 T4). None of it is
rendered anywhere; the self-evolve audits (`scripts/self_evolve_audit.sh`) print to a
terminal on one box. Meanwhile the public site's only growth surface is the corpus
contribution graph (git-mined ADDS per day — corpus breadth) and the fixstats strip
(epoch totals). The growth story that actually differentiates the product — measured
heal-quality climbing while born-warning-rate falls across an increasingly hard
generation ladder — is invisible. The cost of revealing it is small because every
mechanism it needs already shipped once: signed nightly sync (Issues 102+104), consent
plane (dapps Proposal 004), server-rendered graph card (the corpus card).

## The proposed design

### The day row (the contract)

```json
{
  "date": "2026-09-23",
  "ts": 1790123456,
  "machine": "a1b2c3d4…e5f6",
  "quality": {
    "heal_rate": 0.964, "created_rate": 0.018, "parse_safety": 1.0,
    "generation": 3, "lane_runs": 4, "n_ran": 160, "edits_total": 69
  },
  "corpus": {"rules_total": 927,
             "by_domain": {"clippy_lints": 94, "rust_perf": 149, "kernel_opt": 627}},
  "frontier": {"generation": 8, "population": 17, "band_mass": 0.056, "saturated": 2},
  "elo": {"rated_rules": 27, "mean": 1450.2, "top": 2071.9},
  "stores": {"rows": 12828, "certified": 3391, "repos": 10}
}
```

Sources, all read-only, all already on disk: `quality` = the latest
score_history.jsonl run of that UTC day on the LATEST generation lane that ran;
`corpus` = the run's `vocab_size` + the compiled corpus counts at that sha;
`frontier` = the latest frontier_snapshots.jsonl row; `elo` = the store replay the
audit already performs; `stores` = per-repo trajectory-row counts. `machine` is the
`/mining/adoption` pseudonym form (first4…last4), or empty. Signed body =
`{"days":[row,…]}` (the corpus-days shape, so re-posts are full-series and idempotent).

### Transport — third instance of the corpus-days lane

- **Signing**: the SAME `.corpus_sync.key` (one publisher identity for the ops plane),
  message = `kat:healqual:v1:` domain ‖ canonical row JSON. The server verifies against
  the SAME public `KAT_CORPUS_SYNC_PUBKEY_HEX` allowlist var — no new secret, and the
  domain separator makes cross-domain replay impossible. Bearer fallback
  (`KAT_CRANK_TOKEN`) mirrored from the corpus door for ops.
- **Door**: `POST /admin/healqual/days` (ops bearer | Ed25519) → the mining plane's
  merge route. **Merge = strictly-newer-ts wins per date** (equal-ts = no-op): corpus
  days uses monotone max because adds never decrease, but healqual rates CAN legitimately
  move down on a re-run, so the freshness key is `ts`, never the value. A stale
  full-series re-post cannot lower a newer row; a replay cannot mutate one.
- **Bounds** (the Issues-102 discipline): day ∈ [2026-08-19, today+1] (first bench
  run), ≤ 400 rows per post, rates ∈ [0,1], counts ≤ 1e6, ≤ 16 domain entries, name
  ≤ 32 B, row ≤ 1 KB. Registry append-only, no purge door.
- **Collector split** (mirrors corpus days exactly — cross-repo INVOCATION, never a
  cargo dep; the dapps direction gate stays intact):
  - riir-clippy: a read-only `cargo heal --selfevolve-snapshot` mode — pure
    aggregation over the four local feeds, emits the day-row JSON, zero network,
    zero writes. Golden test pins the shape.
  - riir-dapps: `scripts/healqual_sync.sh` (sibling of `corpus_days_sync.sh`) — cds
    into the clippy checkout, derives the full day series statelessly from the
    jsonl feeds + the snapshot mode, signs, POSTs to each configured env. Wired into
    `ops_nightly.sh` (the launchd nightly that already runs the corpus sync), gated on
    the existing stats consent plane (`--stats on` = work-stats sharing; `Unset` and
    `OptedOut` never push — the fix-stats consent posture verbatim).

### Render — a "Self-evolve" card beside the corpus card

- `GET /mining/healquality` — public JSON twin `{days:[…], provenance}`.
- Front-page card: server-rendered SVG, zero JS, ≤ 90 days. Three series on one card:
  heal% polyline (left axis), corpus rules area (right axis), network fixes per epoch
  bars (from the existing fixstats view rows). Caption carries the provenance line:
  *"quality measured by the dev harness on its frozen bench (bounded self-refinement
  — convergent + evaluable); volume from consenting fleet machines."*

### The honest framing (why a growth curve is legitimate here)

arXiv:2607.07663 splits self-improvement into **bounded** (fixed external evaluator —
convergent, evaluable) and open-ended (self-referential — neither). The healer's
score-bench is a fixed external evaluator, so its curve is the evaluable kind: heal%
climbing on a frozen bench while the generation ladder keeps adding harder rungs is a
REAL intelligence claim, not a dashboard vanity metric. The card copy says which kind
it is.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **The heal% ceiling flattens the headline series.** On a frozen lane heal% converges
   to ~96–100% and the curve reads flat-by-success. The growth signal then lives in the
   OTHER series (corpus breadth, born-warning decline, network volume) and in the
   generation ladder (each new bench generation re-drops the curve — 09-06 measured
   54.3% on the v5 lane's arrival). v1 renders latest-lane heal%; per-lane small-multiples
   are v2. If the owner wants the ladder as the headline, say so before Phase 3.
2. **Single-source quality.** v1 quality rows come from the dev box (the only machine
   that runs score-bench). That is a dev-signal, labeled as such; the schema's
   `machine` field is the fleet-join seam. Do not let the card copy imply fleet-wide
   quality measurement before a second machine pushes.
3. **heal% certifies bench-fit, not real-world fix quality.** The corpus is the model;
   a high heal% on our own frozen fixtures is necessary-not-sufficient. The real-world
   axis on the same card is fixstats volume + mining adoption (already public). The two
   axes must stay visually distinct — never merge them into one "score".
4. **Elo/band-mass are thin-n** (27 rated rules, band 1/17). Render as secondary
   series; never the headline. The R135/Bench-047 law (evidence volume is the
   bottleneck) applies to their interpretation, and Issue 130's T0 owner call
   (gate vs report-only) may later change what admission evidence even is.
5. **New public door = new surface.** Mitigations are all inherited (signed, allowlisted
   pubkey, bounded, no purge, fail-closed), but the door still needs the full
   contract-test battery (malformed / ceiling / stale-ts / replay / bad-signature /
   bearer-vs-signed arms) and a manual devnet deploy before the card lights.
6. **Consent wording must not drift.** This rides the existing `--stats on` plane; if
   the reveal scope ever exceeds "aggregate tool quality" (per-rule per-machine
   quality, say), that is a NEW consent decision, not a silent extension.

## Fusion lineage

- **riir-dapps Issues 102+104** — the signed nightly sync lane (corpus days): key
  handling, envelope, monotone merge door, front-page card lighting. This proposal is
  its quality-axis sibling with a ts-based merge.
- **riir-dapps Proposal 004** — the consent-gated work-stats plane (fixstats): consent
  posture, signed per-run rows, public view + privacy law (labeled accounts only).
- **riir-clippy Issues 037/039/089/102** — the measurement substrate: trajectory
  stores, Elo replay, frontier snapshots, score-history fields. The collector only
  reads what these already emit.
- **katgpt-rs Research 440 (AIDE2) + Research 203 (RRSI)** — the measurement frame:
  self-improvement progress is evaluable when the evaluator is fixed and external.

## GOAT gate

- **G1 (correctness):** golden day-row + golden signed payload both repos; merge arms:
  newer-wins, equal-ts no-op, stale-ts rejected-noop, ceiling violations 400, malformed
  400, bad signature 401, bearer fallback; the snapshot mode's unit tests over fixture
  jsonl feeds (empty-is-loud, never a fabricated row).
- **G2 (perf):** nightly push ≤ 5 s (stateless derivation over two small jsonl files);
  card render O(days ≤ 90); the door ≤ the corpus-days door's cost.
- **G3 (no-regression):** corpus-days lane and fixstats lane byte-untouched (their
  contract pins hold); worker lib counts bump ADDITIVE; no change to the heal path,
  the corpus, or any floor.
- **G4 (bounded):** one bounded registry; zero GPU; zero new cargo deps; modelless
  (pure arithmetic + one Ed25519 sign).

## What ships now (riir-clippy) vs deferred (riir-dapps)

### Ships now — Phase 1 scope
- `cargo heal --selfevolve-snapshot` (read-only aggregation mode, ungated, golden +
  fixture tests) — the collector's compute half lives where the feeds live.

### Deferred — riir-dapps Phases 2–3
- `kat:healqual:v1` signing mode, the `/admin/healqual/days` door, the registry,
  `/mining/healquality`, the front-page card, the devnet deploy. Lands as one dapps
  plan citing this proposal; the corpus-days door is the template.

### Explicitly NOT shipped by this proposal
- No on-chain commitment of the series (statroot covers mining telemetry; healqual can
  join a later stats-root stamping).
- No per-machine fleet quality pushes (schema seam only).
- No change to scoring, admission, corpus lifecycle, or any consent EXCEPT reusing the
  existing `--stats on` plane as-is.
- No new public key distribution (the pubkey var already ships committed).

## Phased rollout (sketch — a plan would expand this)

### Phase 1 — riir-clippy collector
- [ ] T1.1 `--selfevolve-snapshot` mode: read the four feeds, emit the day row; empty-day = loud empty, never fabricated
- [ ] T1.2 golden + fixture tests (shape pin, consent-independent — the mode never pushes)

### Phase 2 — riir-dapps door
- [ ] T2.1 `kat_corpus_sign --domain healqual` (new mode, same key)
- [ ] T2.2 door + merge + registry + contract-test battery (both auth postures)

### Phase 3 — reveal
- [ ] T3.1 `GET /mining/healquality` JSON twin + front-page SVG card + strips
- [ ] T3.2 lib-count pins bump; devnet manual deploy; curl-verify all three postures

### Phase 4 — cadence + fleet
- [ ] T4.1 `ops_nightly.sh` leg behind stats consent; provenance caption
- [ ] T4.2 (deferred) fleet join via `machine`; per-lane small-multiples

## Risks

1. **Public-surface risk** (correctness/security): a malformed-tolerant door is a
   spam magnet — bounds + signature + fail-closed are load-bearing, contract tests
   prove each arm.
2. **Metric-optics risk** (product): a flattening heal% line can read as "stopped
   improving" — mitigated by the multi-series card + ladder framing (caveat 1).
3. **Architectural risk**: none new — the collector is cross-repo invocation (the
   corpus-days precedent), no cargo dep, dapps direction gate untouched.

## Out of scope

On-chain stamping; real-time streaming; authenticated per-visitor views; any change to
the healer's heal path; any second consent decision; mainnet enablement (devnet first,
per every prior lane).

## References

1. arXiv:2607.07663 — *Recursive Self-Improvement in AI: From Bounded to Open-Ended* — the bounded-refinement-is-evaluable frame (distilled here, one paragraph).
2. arXiv:2609.11873 — *The Last AI Built by Humans* (RSI survey) — in-house as [Research 167](../../../katgpt-rs/.research/167_RSI_Survey_Autonomy_Ladder_Positioning.md); the Library Drift lifecycle axis (cited).
3. katgpt-rs Research 440 — AIDE2 recursive self-improvement, measured PASS — prior art for progress curves (cited).
4. Lilian Weng, *Harness Engineering for Self-Improvement* (2026-07) — harness-engineering context (cited-only).
5. riir-dapps Proposal 004 + Issues 102/104 — the two lanes this instantiates (in-house precedent).

## TL;DR

Ship it: a third signed-sync lane (quality axis) reusing the two proven ones end to
end — key, door pattern, consent plane, card render — with the ts-based merge as the
one genuinely new mechanic and the bounded-refinement framing keeping the graph
honest. Next action: Claude verdict, then owner GO → dapps plan for Phases 2–3;
Phase 1 can start immediately in this repo.
