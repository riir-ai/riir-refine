# Plan 225 — the `heal` seat gets a real expert: mine → mint → arm-H paired read (the Bench-108 revisit condition)

**Status:** PROPOSED — 2026-10-11; verdict round 1 REVISE (2026-10-11) folded in: behaviour-preserving oracle, deciding-case definition, verify-in-order A0, T1 feasibility gate before any GPU spend, leakage-safe split, pre-registered two-candidate comparison. Awaiting round-2 verdict + owner go. Owner-gated: T3 mint spend (GPU), T5 ARMED production run, T5 telemetry.

## Why now

Plan 202 (R1–R7) landed the whole decision stack and Bench 108 measured it end-to-end over the
live `heal` seat: 9/9 reranks, 1 win / 8 ties / 0 losses, ~6 ms/case, $0 — verdict recorded
**INSTRUMENT-VALID, QUALITY-PENDING**: the seat was the R3 stub (deterministic index-1 pick), so the
modelless incumbent was correctly NOT demoted. Bench 108's own recorded revisit condition is
*"revisit when a real hosted heal expert exists (HOSTED-ONLY tier, riir-train mint lane)"*. This
plan builds that expert — twice (two independent candidates) — and lets the frozen read pick the
winner. The owner's 2026-10-11 directive: remove self-made evals (code_fixtures retired), build a
PROPER bench for the refine decision seat around the Instinct corpus lane and the Rethink encoder.

The oracle makes this honest where code_fixtures was not: **a candidate is gold iff the patched
snippet compiles and the lint clears** — the healer's own compile-gate. No self-authored gold, no
synthetic population (the grammar-not-answers law), and every claim carries its n beside it.

## Substrate consumed (substrate-first — nothing new is built here)

| need | existing substrate |
|---|---|
| escalation + arming | Plan 202 chain: `escalation_manifest` + `escalation_dispatch` + `seat_rerank` (v1 contract frozen, `min_pool ≥ 2`, S8 cap 8, suite `heal`) |
| the seat | `riir-instinct` `install_ext_boots` + `LaneBackend` + `ext_seat_dogfood_server` (suite `heal`, standing rig since Bench 108); Rethink plugs the SAME seam |
| the harness | `fixer_regate --arm H` — paired per-case, cost axes, JSONL ledger, printed demote verdict. NO second harness |
| cases | real usage artifacts: `.refine/misses/` queues, rescue tallies (`--misses` `probe b0` column), arm-H ledgers, `heal_with_pool` pools |
| train-side corpus surround (if thin) | reflex corpus-synthesis lane (`--synth-corpus`: echo gates + V5 + probe-delta-ab) — with the compile-gate as the veto, strictly stronger than an agreement VETO |
| training rigs | riir-train specialist lanes (Issues 576/599/600/608/609 precedent) — GPU on the 4090 (never deferred) |
| demotion | manifest edit (the printed-verdict law); `RIIR_REFINE_ESC=0` stays DEMOTE-ONLY |

## Phases

### T1 — mine the heal-decision case set (refine)
- [ ] T1.1 Harvest real candidate pools: walk `.refine/misses/`, rescue tallies, and past arm-H
      ledgers; for each case capture (snippet, rule/lint id, the L0–L3 candidate list, the
      compile-gate verdict per candidate). Pool-eligibility: `pool ≥ 2` distinct candidates
      (the Bench-107 law — rerank never fires at 1).
- [ ] T1.2 Build the frozen test split FIRST (digest-pinned, cases_digest), never trained on.
      Disclose n per class beside every future claim (the n=32 lesson).
- [ ] T1.3 If real multi-candidate cases are too few for training: synthesize TRAIN-side rows only
      through the reflex synthesis lane (echo gates + V5 + compile-gate veto). The frozen test
      split stays real-only, never synthetic — one contaminated row kills the bench.
- [ ] T1.4 Gold verification harness — COMPILES+CLEARS IS NECESSARY, NEVER SUFFICIENT. A
      candidate is gold iff ALL of: (a) the patched snippet applies, compiles (the `fix_verify`
      posture, `--verify-args` for feature-gated code) and the lint clears; (b) behaviour is
      preserved — the crate's own tests pass on the patched tree (scoped `cargo test` per case,
      budgeted); (c) the patch is non-degenerate — gold is REFUSED when the candidate adds an
      `allow`/`expect` attribute, deletes code beyond the lint span, or otherwise silences rather
      than heals. Anything (a)–(c) cannot settle lands in an explicit UNRESOLVED bucket, never
      counted as gold (the percentile-audit lesson: UNRESOLVED is not clean).
- [ ] T1.5 Deciding cases — the population the bench can actually read. Per case, the pool holds
      0/1/many gold candidates: all-gold → every arm ties; none-gold → the right answer is
      ABSTAIN. Only **deciding cases** (some but not all candidates gold) separate the arms.
      Pre-registered per-case score: pick-a-gold = 1 · abstain-when-none-gold = 1 · otherwise 0.
      Every result reports **deciding-n**, never total-n (Bench 108's 8/9 ties is exactly this
      blindness).
- [ ] T1.6 FEASIBILITY GATE (blocks T2 AND T3): the frozen split must carry ≥ N deciding cases,
      N derived from a power calculation for paired LB95 > 0 at a pre-stated plausible effect
      size (record the arithmetic in the bench record). Gate fails → the plan is recorded
      BLOCKED ON CASE SUPPLY — no mint, no training, no bench against a split that cannot detect
      a difference. The owner's T3 approval is conditional on this gate passing.
- [ ] T1.7 Leakage-safe split: split by TIME or SOURCE REPO, never by row; near-duplicate
      removal by blake3 of (normalised snippet + rule) ACROSS train and test; assert no
      self-evolve trajectory contains a frozen case's patch; reflex V5 synthetic rows (if T1.3
      fires) take seeds from the TRAINING side only.

### T2 — candidate A1: the Instinct-corpus specialist (riir-train mints, instinct serves)
⛔ Gated on T1.6 (the feasibility gate) — an unproven case supply trains nothing.
- [ ] T2.1 Mine the heal training corpus (TRAIN-side split of T1 only, post T1.7 dedupe): refine
      corpora + misses + self-evolve trajectories (`LatentFixMemory` rows), train-side synth
      augment per T1.3. Corpus manifest + blake3s.
- [ ] T2.2 Train the specialist head on the riir-train rigs (4090; small head — the nbsvm-class
      presence-bag shape trained on candidate snippets + rule context is the first arm; scale up
      only if it loses for capacity reasons, not tuning).
- [ ] T2.3 Mint the artifact as `heal_winner_v1.bin` (raw-winner route) + serve it behind the
      `heal` suite through the custom `lane_loader` (the R3-proven route; `check_winner_file`
      bridging added ONLY if the raw route refuses). A10: nothing public ever carries this corpus.

### T3 — candidate A2: the Rethink encoder head (riir-train HOSTED-ONLY mint lane)
⛔ Gated on T1.6 (the feasibility gate) + owner mint approval.
- [ ] T3.1 Train the encoder decision head (the laya encoder + a pick/abstain head) on the SAME
      T1 training corpus — the distill-teacher lanes are available but NOT required; the compile
      gate is the label source, not a teacher.
- [ ] T3.2 Mint HOSTED-ONLY (vessel per R4), serve as a `LaneBackend` behind `heal` via the same
      seat rig. This is the actual Refine↔Rethink fusion point: refine's rerank traffic hardening
      the rethink-served expert (Plan 202's own words).
- [ ] T3.3 GPU box state recorded beside every number (bench_preflight / PROVENANCE law).

### T4 — the bench: one frozen arm-H paired read (refine + instinct seat)
- [ ] T4.1 Arms: **A0** modelless chain + the DETERMINISTIC verify-in-order control — try each
      candidate in pool order through the compile gate, keep the first that passes (else the
      smallest diff); this is what the healer can already compute for itself (`--verify` exists),
      so a learned head must beat it on quality OR match it at strictly lower cost (the
      modelless-first mandate's bar). **A1** + T2 seat · **A2** + T3 seat · hosted generate =
      comparison-only (cost axis, never the winner lane).
- [ ] T4.2 Protocol: paired per-case over the frozen deciding population, ONE read, pre-registered
      comparison: Holm correction across the two candidate-vs-A0 comparisons (or the fixed α
      split stated here: α=0.025 each); superiority = paired LB95 > 0 on the per-case score of
      T1.5. Cost axes with NUMERIC budgets pinned NOW: seat lanes ≤ 50 ms/decision p50 (the
      0.8–6 ms Bench-107 floor + wire slack) and $0 (loopback); the hosted leg priced through
      `llm_spend` with its recorded cap. Two-signal law: accuracy = commissioning; rate = cost
      ceiling only. Seat receipts recorded (decision_blake3 audit trail).
- [ ] T4.3 Verdict rule (pre-registered): a candidate WINS the `heal` manifest row iff its
      Holm-corrected paired LB95 > 0 vs A0 on the deciding population AND cost within the T4.2
      budgets. If BOTH clear: the paired A1-vs-A2 comparison decides; an A1-vs-A2 tie breaks on
      lower cost, then on the simpler artifact (raw winner over trained head). Ties with A0 keep
      A0; else the seat stays empty and the result is recorded — **decline is a correct answer**
      (the L4/603 precedent). Loser demoted by manifest edit; A0 itself never demoted by this
      bench (it is the floor, not a candidate).
- [ ] T4.4 Record under a fresh `.benchmarks/` number; the arm-H run-record law (Plan 202 §11)
      satisfied by the same record.

### T5 — arm + telemetry (OWNER GATED)
- [ ] T5.1 Owner go → the winner's manifest row ARMED for real heal traffic (the first armed
      `heal` row — ALL-EMPTY remains the shipped default until this checkbox).
- [ ] T5.2 The armed run is the consumption trigger already recorded: `rescue_probe`'s arm-H
      escalation-ordering consumption + the katgpt-rs Plan 621 T5.4 real-telemetry re-gate route.
- [ ] T5.3 Decision telemetry consent-gated (`RIIR_INSTINCT_STATS` unset never pushes); decstat
      arming stays its own owner gate.

## Laws this plan binds itself to
- Never train or score on the frozen test rows; synthesis NEVER touches them.
- Every refusal path keeps the modelless answer (`StoodByModelless`) — nothing here can make a
  heal worse, only spend-tested.
- Code egress consent scope unchanged (snippet-only, `Unset` opens no socket).
- A number without its box state is not a measurement (GPU/latency rows carry PROVENANCE).
- One definition everywhere: escalate grammar = instinct's `EscalateSpec`; wire = decision_wire v1
  (additive v2 only); `heal` stays ONE suite for v1 (per-domain lanes wait for measured need).

## References
- riir-refine `.plans/202` (R1–R7) · `.benchmarks/107` (dogfood + R7 inputs) · `.benchmarks/108`
  (the stub verdict + revisit condition) · `src/seat_rerank.rs` · `src/escalation_dispatch.rs` ·
  `src/bin/fixer_regate.rs` (arm H)
- riir-instinct `tests/ext_seat_rerank_gates.rs` · `src/server.rs` (`install_ext_boots`/`LaneBackend`)
- riir-rethink ESC lane (Issue 017 T5 promoted; 023 T2/T3 manifest state) · Issue 028 (two-signal law)
- reflex corpus-synthesis lane (V5 + echo gates + probe-delta-ab) · decision_wire v1
  (`.docs/03_decision_flow/wire_contract_v1.md`)
- riir-train specialist lanes (Issues 576/599/600/608/609) · katgpt-rs Plan 621 T5.4
- The code_fixtures retirement (reflex `658e920` + reflex-site edition 2026-10-11): the honesty
  bar this bench must clear.
