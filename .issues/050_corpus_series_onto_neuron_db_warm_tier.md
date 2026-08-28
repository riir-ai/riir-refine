# Issue 050 — corpus + score series onto the neuron-db Warm tier (kill the remaining dashboard jsonl)

**Status:** OPEN — T1 bridge SHIPPED 2026-08-28 (recorded-layer jsonl + git-mining backfill, live on the dashboard); T2 (KV row family) unclaimed
**Repo:** riir-clippy
**Filed:** 2026-08-28, owner question: *"data is mined from git history — that's a backfill right? from now on when corpus added it will get from neurondb instead right?"*

## The question, answered

Yes — the git-mining is backfill (plus a self-maintaining fallback: it re-derives
truth from source at every regen, so it cannot be forgotten, but it is
window-bounded and carries no metadata). And yes: the destination substrate for
recorded corpus events is the **neuron-db Warm tier** (`.heal/trajectories.kv` —
the Issue 038 owner call: "logs store in neuron-db local kv, no need to parse
back and-forth"). This issue tracks that landing.

## Current state (after T1)

The dashboard's corpus panel composes three layers, later-wins per day:

| layer | source | role |
|---|---|---|
| 1. git-mined | `rule_id:`/`lint_name:` greps at each rev that touched a corpus path (35d window) | backfill for all history before the store existed + **drift cross-check forever** (it reads ground truth; a missed stamp cannot hide) |
| 2. recorded | `.benchmarks/corpus_history.jsonl` (appended idempotently at every dashboard regen with working-tree counts) | durable history beyond the git window — **the KV-migration target** |
| 3. today | working-tree counts, upserted into layer 2 now | the "now" truth |

HEAD counts verified against the pinned batch totals (393 / 88 / 40 / 6) before
the method was trusted.

## T2 — the KV row family (unclaimed)

- [ ] `CorpusCountRow` Pod beside `TrajRowDoc`/`LearnRowDoc` in the `.heal/`
      store (`latent_retrieval`; the Issue 038 pattern — one store, typed rows):
      `{ts_day: [u8;4], domain: u8, count: u32, source_note: str-pod}` — the
      row gains what jsonl cannot carry cleanly: **provenance** (batch id /
      mining source / promotion path).
- [ ] Stamper moves from gen_dashboard into the tool (`cargo heal
      --corpus-stats`): counts the **compiled-in corpus** (runtime truth,
      feature-aware — absent domain ≠ 0), upserts the KV row. Wire into the
      batch-landing ritual (Protocol A step 7) + `--score-bench` standing
      workflow so stamps happen at the moments the corpus changes.
- [ ] One-time import: layer-2 jsonl → KV rows (the Issue 038 migration fence
      pattern: envelope present + store empty → import → rename).
- [ ] Dashboard reader flips: KV first, jsonl second, git-mined remainder.
      **Git-mining is NOT deleted** — it stays as the drift check (a KV row that
      disagrees with the git-derived count at the same day is a finding, not a
      display preference).
- [ ] `score_history.jsonl` rides the same migration (same store, `ScoreRunRow`
      family) — one landing, both series off jsonl.

## Why the jsonl bridge first (deliberate)

The KV row family needs real surface (Pod type, store methods, CLI flag,
ritual wiring, reader). Landing it half-done today would leave the series
recording nothing while the substrate was built. The bridge starts durable
accumulation NOW (any future migration imports real history, not an empty
store) and keeps the dashboard correct in the meantime. Same shape as
Issue 038's envelope→store sequencing.

## Non-goals

- No provenance in layer 2 (jsonl carries counts only; provenance is a T2 KV
  field — not worth a jsonl schema bump for a bridge).
- No rustc_errors domain in the series (driver-based, different unit — templates
  not rules; add only if the owner wants it with its own semantics).
- `docker` domain joins automatically when Plan 030 lands (one CORPUS_DOMAINS
  row + one KV domain byte).
