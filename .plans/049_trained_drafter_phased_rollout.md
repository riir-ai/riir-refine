# Plan 049 — Trained clippy drafter: phased rollout (Proposal 009)

Status: **in progress — Phase 0 pending (no code written yet; owner go received via "do 3" on the proposal offer)**
Branch: `develop` (per global rule — no feature branches)
**Proposal:** [`.proposals/009_trained_clippy_drafter_moka_puct.md`](../.proposals/009_trained_clippy_drafter_moka_puct.md)
Predecessor evidence: Bench 049 (CF-70B 5–6/9, decline 0/9), riir-train Plan 336 + Benches 463/467 (0/60 generative-LoRA failure), Plan 017 + Bench 043 (headroom instrument + negative), Plan 014 (score-bench axes), Research 001 (Model-evolution category), Research 077 (DFlash2).

## The one reframing (read before any task)

Train a **drafter**, not a fixer. Bounded output space (rule-id + template slots);
verifier-gated correctness (syn → `clippy_verify` → `fix_verify`); draft quality = acceptance
rate only. Never emit free-text fixes (the 0/60 shape). Never evaluate against our own
proposers (the T3.5.4 contamination lesson) — `clippy_oracle` labels everything.

## Gate table (falsifiable; mirrors Proposal 009 §GOAT gate)

| Gate | Bar | Instrument | Phase |
|---|---|---|---|
| G0 kill switch | trainable band on real-repo spans is NONZERO via `decompose_headroom()` | `src/draft/headroom_probe.rs` re-run on the refreshed span set | 0 |
| G1 correctness | never-wrong preserved; decline stays default; oracle-labeled eval only | `clippy_oracle` + uncontaminated composite | 1–3 |
| G2 heal-rate | beats `decline` on the REFRESHED reachable set; bar = max(Bench 049 70B-arm, refreshed) | Bench 049 harness, re-measured at gate time | 2 |
| G3 no-regression | score-bench: heal ≥ 98.1%, created ≤ 1.9%, safety 100%, decline 100%; feature-OFF suites count-identical | `--score-bench` + CI guard all layers | 1–3 |
| G4 latency/alloc | suggest path in Bench 018 budget shape; counting-allocator stable; feature-OFF byte-identical | `SuggestScratch` pattern | 1–3 |
| G5 acceptance curve | held-out drafter acceptance improves monotonically per epoch (the moka axis) | training telemetry on the frozen dataset | 1+ |
| G6 headroom moved | selector-addressable band 0 → nonzero on real-repo spans | `decompose_headroom()` | 2–3 |

**Promotion rule:** G0 must pass before training code exists; G1–G6 must ALL pass before the
`trained_prior` feature may leave opt-in; `decline` is the default answer at every phase.

## Phase 0 — kill-or-proceed (gate zero; cheapest first)

- [ ] T0.1 Oracle-label a fresh real-repo span set across the workspace repos (extend the
      `src/draft/clippy_oracle.rs` batch path; target ≥ 300 labeled spans; commit the dataset
      BLAKE3-committed under `.benchmarks/` or a data dir — freeze/thaw precedent).
- [ ] T0.2 Re-run `decompose_headroom()` (Plan 017 instrument) with the trained-arm lens:
      classify each pipeline failure as (a) retrieval miss (rule absent from pool), (b) ordering
      miss (present, not top-1), (c) proposer-absent (no bounded fix exists). The TRAINABLE band
      = (a)+(b) where a span-conditioned type-aware signal could plausibly flip the outcome.
- [ ] T0.3 **Stop rule:** trainable band < 2% of failures → record the negative bench,
      close the plan, keep decline (the Plan 017 precedent — G0's denominator must be non-empty
      before any GPU hour is spent). Record as `.benchmarks/NNN_trained_drafter_phase0.md`.

## Phase 1 — dataset + bi-encoder prior (feature `trained_prior`, opt-in, implies `latent_retrieval`)

- [ ] T1.1 Freeze the dataset (T0.1 output + corpus pairs): (span, rule-id, fix-template) triples,
      train/val/held-out split seeded and committed; BLAKE3 commitment over the canonical
      serialization (the corpus-commitment pattern).
- [ ] T1.2 Train the two-tower bi-encoder in **riir-train** (GDN towers over span/rule tokens,
      contrastive loss, `dot + sigmoid` scoring — sigmoid never softmax; M3 Metal first, no 4090
      unless scaling). Export a freeze snapshot.
- [ ] T1.3 Ship the artifact as runtime DATA: BLAKE3-committed weight file (freeze/thaw
      NeuronShard-style envelope, riir-neuron-db precedent); `trained_prior` loads + hash-verifies
      at startup; stale/mismatched hash → loud fallback to the modelless path (never silent).
- [ ] T1.4 Integrate as a pool scorer behind the feature: scores the fan-out pool, feeds the
      `RerankMode` slot; feature-OFF path byte-identical (assert by test).
- [ ] T1.5 Gates: G1 (oracle-labeled composite eval), G3 (score-bench + CI guard, feature OFF
      count-identical), G4 (latency/alloc), G5 (acceptance curve on held-out). Record
      `.benchmarks/NNN_trained_prior_g1.md`.

## Phase 2 — drafter heads for the reachable misses (existing L4 seam, unchanged contract)

- [ ] T2.1 Slot-filling heads: which identifier/range fills which template hole, bounded by the
      corpus template surface; template-instantiation output only (no free text).
- [ ] T2.2 Bench 049 harness arm: strict-keep / v2edit / parse / merge partial credit vs
      `decline` and vs the CF-70B numbers, on the REFRESHED reachable set (G2).
- [ ] T2.3 Miss-only reachability contract enforced: the trained drafter is consulted on L4
      misses exactly like `DaemonL4` (`a_parsing_modelless_fix_is_never_replaced_by_l4` holds).
- [ ] T2.4 Gates: G2 + G6 (headroom band 0 → nonzero). Record the bench.

## Phase 3 — PUCT search on contested spans (the moka transfer proper; only if Phase 1–2 win)

- [ ] T3.1 Policy head = the bi-encoder prior; value head = V(passes-verify) trained on verify
      outcomes (self-play over repo sweeps; oracle = reward; moka PUCT port pattern
      `katgpt-moka-wasm/src/puct.rs`, sigmoid-gated, bounded budget).
- [ ] T3.2 Search only on contested/composite spans (multi-lint, ambiguous) — measure the
      contested-span rate first; if < 1% of spans, defer `[-]` with the number recorded.
- [ ] T3.3 Self-play loop: dataset re-freeze per epoch, G5 curve per epoch, corruption-guard on
      the oracle (label-drift check vs pinned fixtures).
- [ ] T3.4 Gates: full G1–G6 re-run + score-bench series row.

## Phase 4 — promotion decision

- [ ] T4.1 All gates green + real-repo G5-style validation (heal a real file end-to-end:
      compile + tests) → promotion proposal to the owner; feature stays opt-in until the owner
      flips it.
- [ ] T4.2 Doc-sync: AGENTS.md feature list + `.docs/11_domains/` entry (or the drafter doc),
      references back to Proposal 009 + this plan.

## Deliberate defers

- [-] AST-path representation refinement (code2vec-style paths) — only if the token bi-encoder
      plateaus below G2; cite-and-verify code2vec/TBCNN at that point (search quota was
      exhausted at proposal time).
- [-] CF llama / Bonsai serving-lane changes — untouched; this artifact is a separate
      domain-trained arm, and the existing leak-posture gate stays.
- [-] 4090 training — only if M3 Metal cannot converge the towers in reasonable time; check
      GPU exclusivity per the standing rule first.

## Non-goals

- No generative fix emission; no LoRA on general LMs; no default-on without all gates; no new
  cargo dep for riir-clippy (weights are runtime data); no public-surface change.

## References

- [Proposal 009](../.proposals/009_trained_clippy_drafter_moka_puct.md) — the design argument.
- Bench 049 / Bench 031 / Bench 043 / Plan 014 / Research 001 / Research 077 — see the
  proposal's References for the full annotated list.
- riir-train Plan 336 + Benches 463/467 — the failed predecessor and its postmortem shape.
