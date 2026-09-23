# Proposal 016 — Public self-evolve reveal: the healer quality day-series on the kat-service dashboard

Status: **REVISE round 2 addressed — D1–D4 + E1–E3 incorporated, owner D1 call made (per-lane small-multiples headline, ladder → step markers). Cadence verdict recorded 2026-09-23 (client-side push; nightly anchor T4.1 + opportunistic interval T4.3; idempotent door replaces a retry queue; CF cron earns nothing). OWNER GO 2026-09-23 — nightly rider KEPT; Claude verdict REVISE round 1 ADDRESSED, round 2 AGREE (residual prose fixed at landing; T4.2 (date,machine) note added). Next: the riir-dapps plan.**
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: riir-dapps Proposal 004 (fix-stats lane) × riir-dapps Issues 102+104 (corpus-days signed sync) × riir-clippy Issues 037/039 (self-evolve stores + Elo) × riir-clippy Issues 089/102 (frontier snapshots + fade)
Related: [Research 203 — RRSI](../.research/203_rrsi_regularized_harness_evolution.md), [riir-dapps .proposals/004](../../../riir-dapps/.proposals/004_fix_stats_lane_consent_gated.md), katgpt-rs Research 440 (AIDE2), arXiv:2607.07663

## TL;DR

The healer's self-evolve is measured every day and visible to no one: 162 score-bench
runs since 2026-08-19 (heal 84.6% → 96.4%, created-warnings 5.8% → 1.8%, safety 100%),
Elo stores across 10 repos, frontier band-mass snapshots, trajectory rows — all local
jsonl/kv artifacts. The public dashboard shows only VOLUME (corpus contribution graph,
fixstats totals). This proposal adds the QUALITY axis: an Ed25519-signed daily
aggregate — cranked by the nightly anchor leg plus an opportunistic interval
launcher (Q3 cadence) — pushed through a new `POST /admin/healqual/days` merge door
and rendered as
a "Self-evolve" card whose HEADLINE — per the owner's call on the round-2 verdict — is
**per-lane small-multiples: each generation's own heal% climb since birth**, the only
series with real daily resolution. The difficulty ladder (3 steps in 27 days, its
biggest jump being the day the generation field arrived) renders as step MARKERS on
the time axis, never as a learning line. Third instance of the corpus-days lane: same
key, same allowlist var, same bounded registry, new domain separator; the collector is
pure Python over plain files (never cargo-builds in the nightly). Modelless, zero GPU,
zero new deps.

## The problem this solves

The owner's question — "is the healer getting smarter each day?" — is answerable from
data we already measure, but the answer lives in `.benchmarks/score_history.jsonl`
(riir-clippy, 162 runs), `.benchmarks/frontier_snapshots.jsonl` (089 T4 emission), and
the per-repo trajectory stores (037 store-first). None of it is rendered anywhere.
Meanwhile the public site's only growth surface is the corpus contribution graph
(git-mined ADDS — volume) and the fixstats strip (epoch totals — volume). The owner
looked at devnet and asked where the self-evolve is. The cost of revealing it is small
because every mechanism it needs already shipped once: signed nightly sync (Issues
102+104), the publisher-key posture, the bounded door, the server-rendered card.

## The proposed design

### The day row (the contract — all values mechanically derived from the 2026-09-22 feed state)

```json
{
  "date": "2026-09-22",
  "ts": 1790095454,
  "machine": "",
  "quality": {
    "heal_rate": 0.963636, "created_rate": 0.018182, "parse_safety": 1.0,
    "generation": 1, "n_ran": 35, "edits_total": 69
  },
  "lanes": {"1": {"heal_rate": 0.963636, "created_rate": 0.018182, "n_ran": 35}},
  "ladder": {"hardest_admitted": 8},
  "frontier": {"generation": 7, "population": 19, "band_mass": 0.052632, "saturated": 2}
}
```

Field rules (each closes a verdict finding):

- **`ts` = the selected bench RUN's own time** (B2/D4) — `1790095454` =
  `2026-09-22T16:44:14 UTC`, the run the quality block cites; never collection time.
  Nightly re-posts of a settled day are equal-ts no-ops; only the day still in
  progress moves. The whole series is re-derived statelessly from the feeds each run
  and posted in full (the corpus-days idempotent shape).
- **Feed order is NOT trusted (E1):** the feed carries one inverted adjacent pair
  (`08-28T21:00:28` before `…T20:50:17`). The collector SORTS BY `ts` before any
  "latest run of that day" / "first tagged row" selection; a G1 fixture pins the
  out-of-order pair.
- **Parse UTC, bucket ICT (D3):** the feed writes naive ISO with no offset; measured
  against file mtimes it IS UTC — that is the PARSE rule. The DAY BUCKET is **+07:00
  (ICT)**, matching the corpus card's owner anchor
  (`mine_corpus_git_days.py`: "Dates convert in +07:00 (ICT) per the owner's anchor"),
  so the two cards side by side mean the same `2026-09-22`. The near-midnight fixture
  (run 16:44 UTC = 23:44 ICT same day, and a 17:30 UTC = 00:30 ICT next-day case)
  pins the CONVERSION, which is the part that can be wrong.
- **`quality` is pinned to generation 1** (B1) — the longest continuous series (87
  tagged runs) and the ONLY lane that is a fixed external evaluator. Rows carrying no
  `generation` field (45 early rows, all preceding the first tagged row
  `2026-09-06T07:41:44`) join the gen-1 series — this is the SHIPPED serde default,
  `src/traj_store.rs:255`: pre-generation rows default to 1 (E2 cites the precedent
  rather than inventing a parallel convention); any untagged row AFTER the first
  tagged row is skipped LOUD with a count in the sync output (measured 0 today).
  **A day with no gen-1 run is legal (E3):** `quality` is then ABSENT from the row
  (the small-multiples and lanes series still carry that day's data); the door
  accepts it, and a contract arm pins the absent-quality row.
- **`lanes` = every generation that ran that day** (D1 headline fuel): per-lane
  latest run `{heal_rate, created_rate, n_ran}`, keyed by generation (JSON string
  keys), ≤ 8 keys observed. Only lanes that actually ran that day appear — never
  backfilled, never interpolated client-side.
- **NO corpus field (D2):** the first draft's `rules_total = vocab_size` was the
  frozen bench's lint-name coverage (~75) wearing a corpus-rules name, and
  `vocab_size` is polysemous across lanes (`heal.rs` ladder lanes write
  `members_total` there). Corpus breadth is already public on the neighbouring
  corpus-days card — this row does not carry it. Reserved nothing; if a real corpus
  count is ever needed it comes from a Rust collector with its own field name.
- **`ladder.hardest_admitted` = running max of generation over rows with
  `ts ≤` day-end** (D1 repair of the round-1 "max in feed" wording, which read as a
  flat global max). Measured shape: 3 steps (1 → 7 on 09-06 — the day the field
  arrived, an instrumentation jump — → 8 on 09-11), flat at 8 for the last 9 days.
  This is why it is a MARKER, not a headline.
- **`frontier`** = latest frontier_snapshots.jsonl row (pure file read; the values
  above are ILLUSTRATIVE — feed state as derived 2026-09-22, not a live figure;
  D4's own rule applied to this prose: the golden test derives, never transcribes).
- `elo` and per-repo `stores` counts are RESERVED, absent in v1 (C3 — they need the
  Rust store reader; caveat 4 bars them from any headline either way).

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
  no publisher field) this lane gets from a self-asserted clock — the weaker
  property, closed by two bounds: **`ts ≤ now + skew` REJECTED, never clamped** (B3 —
  a clock skew otherwise wedges a date permanently), and the B2 provenance rule.
  Contract arms: newer-wins, equal-ts no-op, stale-ts no-op, future-ts 400,
  absent-quality accepted (E3).
- **Bounds** (the Issues-102 discipline): day ∈ [2026-08-19, today+1] (first bench
  run), ≤ 400 rows/post, rates ∈ [0,1], counts ≤ 1e6, `ts ≤ now+skew` (reject), ≤ 8
  lane keys — each key must parse as an INTEGER 1..=64 (shape bounded, not just
  count; admits no `"../../x"` and no 200-char key) — row ≤ 1 KB.
  **`ts` ∈ [date 00:00 ICT, date+1 00:00 ICT) REQUIRED** (verdict round 1): nothing
  else constrains `ts` against `date`, and with NO purge door a publisher misconfig
  (wrong TZ constant, stale `date` + fresh `ts`) would write today's numbers onto an
  arbitrary past day at a permanently-unbeatable ts — the same permanent-wedge B3
  rejects, reached through the other field. Exactly derivable, no tolerance, holds by
  construction for the collector. **Duplicate dates within ONE payload → 400** — the
  merge law is per-date ACROSS posts; two rows for one date inside one post has no
  rule, and 400 is cleaner than last-wins. No purge door; the day row is a
  KEYED-SLOT UPSERT (per-date slot — the template's measured shape, see Cadence)
  and the domain registry is content-compared: growth tracks days by construction,
  never post frequency.
- **Collector — pure Python over plain files (C3)**: `riir-dapps/scripts/mine_healqual_days.py`
  (sibling of `mine_corpus_git_days.py`), run INSIDE the riir-clippy checkout, reads
  score_history.jsonl + frontier_snapshots.jsonl only. `healqual_sync.sh` (sibling of
  `corpus_days_sync.sh`) signs and POSTs. **The nightly never cargo-builds** — the
  stale-binary class is structurally absent; `.corpus_sync.key` is the authentication.
- **Cadence — the scheduling verdict (recorded 2026-09-23; Claude REVISE round 1
  addressed same day)**: the producer stays CLIENT-SIDE push; data locality decides
  it — `crank_alarm_pass` owns server ledger state and can read none of these feeds,
  the feeds are local jsonl, and `.corpus_sync.key` lives on the publishing machine —
  so a CF-side cron has nothing to run and earns nothing for this card. The door's
  idempotent merge (newer-ts wins / equal-ts no-op) makes scheduling a LIVENESS
  concern, not a correctness one — no retry queue, no delivery ledger; a missed night
  self-heals on the next push, and every push re-derives the FULL series from the
  jsonl feeds (never a partial delta). "Distributed crank" therefore resolves as
  per-machine self-cranking, TWO triggers:
  - **(a) the nightly anchor (T4.1)** — the `ops_nightly.sh` leg BESIDE the existing
    corpus leg (C2 — the launcher lives in riir-clippy), same non-fatal `WARN …
    continuing` posture. The nightly ALWAYS POSTS — never `--if-changed` — because
    it is the design's only liveness probe: a quiet day that skipped the POST would
    let a rotated key hide behind a card that still reads as success (the LOUD-401
    requirement below).
  - **(b) the opportunistic leg (T4.3) — a SECOND launchd interval, NOT a hook.**
    "Whatever invokes score_bench" has NO SUBJECT: every `--score-bench` site in
    riir-clippy is documentation (README, .docs, AGENTS.md); the invoker is a human
    or an agent typing the documented command, and a wrapper-and-hook repair
    reproduces exactly the git-hook class this proposal rejects — the documented
    direct invocation bypasses it, silently. The activity signal already exists as
    a FILE (`score_history.jsonl`'s content). So: `healqual_sync.sh --if-changed`
    (cheap, idempotent — mine the series, digest-compare against the cached one,
    POST only when different) on its own launchd interval beside the
    `com.riir.ops-nightly` plist. Latency ≤ interval, no unowned seam in any
    caller, nothing to forget to install.
  - **`--if-changed` is the digest guard's ONLY caller** — the guard is a FLAG on
    the shared script, never unconditional: if it lived in the script's main path
    the nightly would skip quiet days too and (a)'s liveness probe would die.
  - **Digest guard DEMOTED to optimisation (measured, not reasoned)**: the round-0
    motivation — "append-only registry churn would track bench-run frequency" — is
    WRONG against the template: `corpus_view.rs` writes `CorpusViewRow::key(day)` —
    a PER-DAY KEYED SLOT, upsert — so a same-day re-push overwrites one slot and
    appends nothing; registry growth tracks days BY CONSTRUCTION, whatever `ts`
    does. The churn invariant belongs at the DOOR (the only thing that survives a
    deleted digest cache, a second publisher, or a forgotten flag): the door stores
    the newer ts but may report a content no-op when every RENDERED field (rates,
    lanes, ladder, frontier) is byte-identical. A last-pushed-TS watermark is
    REJECTED — it cannot see a past day's row changing under a collector-rule
    change or a late out-of-order feed row (the E1 class, measured to happen). The
    client digest stays as the cheap first line: bandwidth saved, nothing more.
  - **Digest write ordering (multi-env)**: `ENVS` is a list; a failed devnet POST
    with a written digest would skip that series forever. Pin: the digest is
    written ONLY after EVERY configured env returned ok — or keyed per host.
  - **The Rust-seam rejection STANDS — three reasons, strongest last**: (1) the key
    + an outbound POST inside the healer binary breaks C3's "pure Python,
    `.corpus_sync.key` is the authentication" posture; (2) `cargo heal` runs in
    10+ sibling repos (`scripts/swept_repos.txt`) — a lint healer that opens an
    outbound connection from any checkout it is invoked in is a materially
    different tool, not a wiring detail; (3) `score_bench` is FEATURE-GATED — a
    push seam behind it compiles to NOTHING on every ordinary build (this repo's
    own green-zero class): the trigger would be present in the source and absent
    in the binary anybody actually runs.
  - **Backoff rules**: transport / 5xx is transparent (the next opportunity catches
    up, by the full-series re-derivation); **408/429 are retryable** (next
    scheduled run); **401/403 is terminal AND LOUD** — key rotation or an allowlist
    drop must surface at the launcher, never warn-and-continue into a permanently
    silent card that still reads as success.

### Consent posture (Q2 — resolved round 1)

**v1 is NOT consent-gated — deliberately.** The row is the project's own bench
results, from files in the owner's own repo, published under the owner's own key:
structurally the corpus-days PUBLISHER posture (`corpus_days_sync.sh` has no consent
check; the key is the auth), not the fixstats third-party-telemetry posture. Consent
becomes load-bearing at Phase 4's fleet join — Proposal 004 names the mechanism: a
future work-stat lane joins the record by appending a **`lane: "healqual"`** tag,
never an implicit ride on `lane: "fix"`.

### Render — the "Self-evolve" card (D1 owner call: small-multiples headline)

- `GET /mining/healquality` — public JSON twin `{days:[…], provenance}`.
- Front-page card, server-rendered SVG, zero JS, ≤ 90 days:
  1. **HEADLINE (owner call): per-lane small-multiples** — one small curve per
     generation, each showing that lane's heal% climb since its first run (the
     only series with real daily resolution; each new bench's descent-and-climb
     IS "getting smarter" made visible). Lanes born in-window only; the gen-1
     panel shows convergence, which is the bounded-refinement achievement.
  2. **created-rate decline** companion — 5.8% → 1.8% on the shared time axis.
  3. **ladder step MARKERS** on the time axis (1→7→8) — position, never a line.
  4. **network fixes** bars (from existing fixstats views) — visually distinct
     VOLUME, never merged into the quality series.
- Caption: *"each curve = one benchmark generation's heal rate since birth (smaller
  panels = harder, newer benches); markers = hardest generation admitted; bars =
  volume from consenting fleet machines. Bounded self-refinement — fixed evaluator,
  convergent and evaluable (arXiv:2607.07663)."*

### The honest framing (why a growth curve is legitimate here)

arXiv:2607.07663 splits self-improvement into **bounded** (fixed external evaluator —
convergent, evaluable) and open-ended (self-referential — neither). Each per-lane
panel is a fixed external evaluator measured over time; the small-multiples headline
shows the healer meeting successively harder fixed evaluators — the evaluable kind of
self-improvement, honestly framed.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Small-multiples need guard rails against sparse-lane noise**: gen-8 has 2 runs;
   a two-point panel can render a dramatic fake slope. Panels with n < 3 runs render
   with an explicit "thin evidence" tag (the R135/Bench-047 law at the render layer);
   the ladder markers remain the difficulty context.
2. **Single-source quality.** All rows come from the dev box (the only machine that
   runs score-bench). `machine` is empty in v1; the field is the fleet-join seam.
   Card copy must not imply fleet-wide quality measurement before a second machine
   pushes.
3. **heal% certifies bench-fit, not real-world fix quality.** The corpus is the model.
   The real-world axis on the same card is fixstats volume + mining adoption; the two
   stay visually and semantically distinct — never one merged "score".
4. **Elo/band-mass are thin-n** (27 rated rules; latest snapshot read band_count 0 /
   population 27 on 2026-09-23 — the round-2 figure was band 1/19; both are thin-n,
   the conclusion is unchanged) — reserved out of v1 entirely (C3), barred from any
   headline even when a Rust collector lands. Issue 130's T0 owner call may change
   what admission evidence even is.
5. **New public door = new surface.** Inherited mitigations (signed, allowlisted
   pubkey, bounded, no purge, fail-closed) plus the ts-merge contract arms; full
   battery before the card lights; manual devnet deploy.
6. **Consent scope is fixed by construction**: v1 publishes only the owner's own bench
   artifacts under the owner's own key. ANY future row sourced from another party's
   machine arrives through the Phase-4 lane tag and ITS OWN consent decision — the
   architecture makes the silent extension impossible rather than trusting memory.
7. **The ICT bucket is an owner anchor, not a technical truth** — it exists so the two
   cards agree. If the corpus card's anchor ever moves, THIS card moves with it (one
   constant, same file convention).

## Fusion lineage

- **riir-dapps Issues 102+104** — the signed nightly sync lane (corpus days): key
  handling, envelope, bounded door, front-page card lighting, the launcher this
  extends, the ICT anchor this bucket matches.
- **riir-dapps Proposal 004** — the lane-tag mechanism Phase 4 uses, and the privacy
  law (labeled accounts only) the bars series already honors.
- **riir-clippy Issues 037/039/089/102** — the measurement substrate: trajectory
  stores, Elo replay, frontier snapshots, score-history fields; `traj_store.rs:255`'s
  pre-generation-defaults-to-1 is the shipped precedent for the untagged-row rule.
- **katgpt-rs Research 440 (AIDE2) + Research 203 (RRSI)** — the measurement frame:
  self-improvement progress is evaluable when the evaluator is fixed and external.

## GOAT gate

- **G1 (correctness):** golden day-row derived MECHANICALLY from the feed (D4 — a
  hand-typed example pinned a ts↔date contradiction once already; the golden test
  derives, never transcribes) + golden signed payload both repos; merge arms:
  newer-wins, equal-ts no-op, stale-ts no-op, **future-ts 400 (B3)**,
  **ts/date mismatch 400 (verdict round 1 — ts outside [date 00:00 ICT,
  date+1 00:00 ICT))**, **duplicate dates within one payload 400**,
  **lane-key shape 400 (non-integer or out-of-range key)**, ceiling violations 400,
  malformed 400, bad signature 401, bearer fallback,
  **absent-quality accepted (E3)**; collector fixtures: **out-of-order feed pair
  sorted by ts (E1)**, **near-midnight ICT conversion both sides of midnight (D3)**,
  untagged pre-field rows default to lane 1 + post-field loud skip (B1/E2),
  settled-day equal-ts idempotence (B2), empty-feed loud-empty (never fabricated),
  thin-lane n<3 tag (caveat 1), **`--if-changed` skips a byte-identical series and
  POSTS a changed one (T4.3)**.
- **G2 (perf):** nightly push ≤ 5 s (two small jsonl files, pure Python); card render
  O(days ≤ 90 × lanes ≤ 8); door ≤ corpus-days door cost.
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
  small-multiples card, devnet manual deploy. One dapps plan citing this proposal;
  corpus-days door is the template.

### Deferred — riir-clippy Phase 4
- The `ops_nightly.sh` anchor leg + the T4.3 opportunistic launchd interval
  (launchers live here, C2) and the `lane: "healqual"` fleet
  join (Proposal 004 mechanism, new consent decision at that point).
- Rust store reader for `elo`/`stores` fields (reserved; never headline).

### Explicitly NOT shipped by this proposal
- No on-chain commitment of the series (statroot covers mining telemetry; healqual can
  join a later stats-root stamping).
- No per-machine fleet quality pushes in v1; no `--stats` coupling.
- No corpus-rule counts on this card (D2 — the corpus card owns breadth).
- No change to scoring, admission, corpus lifecycle, or the healer's heal path.

## Phased rollout (sketch — a plan would expand this)

### Phase 1 — riir-dapps publisher
- [ ] T1.1 `mine_healqual_days.py`: read the two jsonl feeds → full day series; sort-by-ts, ICT bucket, lane-1 pin, lanes map, ladder running-max; loud-empty
- [ ] T1.2 golden fixtures (derived, not transcribed) + `healqual_sync.sh` + sign mode `--domain healqual`

### Phase 2 — riir-dapps door
- [ ] T2.1 door + newer-ts merge + registry + full contract battery (future-ts 400, ts/date-mismatch 400, duplicate-dates-per-payload 400, lane-key-shape 400, absent-quality accepted — G1 is the authoritative arm list)

### Phase 3 — reveal
- [ ] T3.1 `/mining/healquality` JSON twin + front-page small-multiples SVG card + ladder markers + volume bars
- [ ] T3.2 lib-count pins bump; devnet manual deploy; curl-verify all postures

### Phase 4 — riir-clippy cadence + fleet
- [ ] T4.1 `ops_nightly.sh` leg beside the corpus leg (WARN-continuing posture;
      the nightly anchor) — plus the LOUD 401/403 surface at the launcher
      (cadence verdict: terminal auth failures surface, never silent-skip)
- [ ] T4.2 (deferred, new consent decision) `lane: "healqual"` fleet join; Rust store fields. NOTE (verdict round 2): the merge law is per-DATE while `machine` is the fleet seam — under two publishers a later push for an existing date silently DISCARDS the earlier machine's row, never joins it; the door key must widen to `(date, machine)` or the lane declares single-writer-per-date explicitly
- [ ] T4.3 (deferred, independent of T4.2) opportunistic leg — a SECOND launchd
      interval (beside the `com.riir.ops-nightly` plist) running
      `healqual_sync.sh --if-changed` (mine → digest-compare against the cached
      series → POST only when different; digest written ONLY after every
      configured env returned ok). NOT a hook on score_bench's invoker — no such
      caller exists (every `--score-bench` site is documentation), and a wrapper
      hook re-creates the git-hook bypass class. NEVER a Rust seam in the healer
      binary (C3 + the swept-repos blast radius + the feature-gated green-zero
      class). The nightly leg (T4.1) never passes `--if-changed` — it always
      POSTs, as the LOUD-401 liveness probe.

## Risks

1. **Public-surface risk**: a malformed-tolerant door is a spam magnet — bounds +
   signature + fail-closed are load-bearing; contract tests prove each arm.
2. **Clock-trust risk** (Q1): the ts merge trusts a self-asserted clock where
   corpus-days trusts algebra — accepted deliberately (rates must be able to fall),
   bounded by B3 + the signed bytes; the residual is a publisher misconfig, not an
   attacker win (they'd need the key).
3. **Thin-lane optics risk**: two-point panels fake slopes — n<3 "thin evidence" tag
   (caveat 1); ladder markers stay context, never a learning line (D1).
4. **Architectural risk**: none new — cross-repo invocation (corpus-days precedent),
   no cargo dep, dapps direction gate untouched.

## Out of scope

On-chain stamping; real-time streaming; authenticated per-visitor views; per-machine
fleet pushes in v1; corpus breadth on this card; any change to the healer's heal path;
mainnet enablement (devnet first, per every prior lane).

## References

1. arXiv:2607.07663 — *Recursive Self-Improvement in AI: From Bounded to Open-Ended* — the bounded-refinement-is-evaluable frame (distilled here).
2. arXiv:2609.11873 — *The Last AI Built by Humans* (RSI survey) — in-house as [Research 167](../../../katgpt-rs/.research/167_RSI_Survey_Autonomy_Ladder_Positioning.md) (cited).
3. katgpt-rs Research 440 — AIDE2 recursive self-improvement, measured PASS (cited).
4. Lilian Weng, *Harness Engineering for Self-Improvement* (2026-07) (cited-only).
5. riir-dapps Proposal 004 + Issues 102/104 — the lanes this instantiates (in-house precedent).
6. Citation note for the future dapps plan: cite this document QUALIFIED as
   **"riir-clippy Proposal 016"** everywhere — riir-dapps `.proposals/.highwater`
   reads 6, so a bare "Proposal 016" in a dapps doc is live headroom that rebinds
   to the wrong document the day dapps allocates the number.

## TL;DR

Ship it: small-multiples headline (owner call), ladder as markers, publisher posture
without a consent gate, pure-Python collector that never builds, ts merge with the
future-ts ceiling, ICT bucket matching the corpus card's anchor. Next action: owner GO
→ dapps plan for Phases 1–3; Phase 4 leg in this repo's launcher.
