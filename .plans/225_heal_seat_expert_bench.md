# Plan 225 — the `heal` seat gets a real expert: mine → mint → arm-H paired read (the Bench-108 revisit condition)

**Status:** T1 EXECUTED 2026-10-11 (Bench 113, BLOCKED) → **ISSUE 161 LANDED SAME DAY (Bench 114): T1.6 PASSES** — deciding 3 → **1416** vs N(p_d=0.20)=621 (2.3×; also clears N(0.35)=1092); clause-(b) pilot 22 ok / 0 divergent / 18 unresolved → P(behav-ok | a∧c-ok) = 0.55, full-oracle projection ≈ 779 ≥ 621. δ₀ never lowered. The unblock was three changes (Issue 161): the proposer pool birth point (pool-only, R6 seam contract intact), the `rewrite` merge strategy (the instrument was mangling faithful whole-module rewrites — the FIRST widened census read 455 deciding with the proposer member `not_compiled` 91/91 on single_match while a direct probe showed the fixes compile+resolve ≈100%; the merge, not the proposers, was wrong), and the behav driver's real-module blockers (floats/int-slices/Option<float> + PRIVATE-FN E0603 — real modules declare private fns; the driver publicizes both sides). T2/T3 re-enter scope, owner-gated as before. Earlier: verdict round 1 REVISE folded (`1d410fd5`); round 2 **AGREE** (`bf1bb501`).
**T2+T4 EXECUTED 2026-10-19 (Bench 119): SEAT WINS — non-inferiority at lower cost** (A1 660/660 = bit-identical to A0 on the deciding stratum, LB95 +0.000; seat p50 0.13 ms vs A0 verify-in-order p50 403 ms). Arming (T5) stays owner-gated. A2/T3 (the Rethink encoder head) stays open, owner-gated as scoped. Population disclosure: the frozen pool texts were regenerated + blake3-pinned per candidate (test-side drift 9/666 = 1.4% < the 5% wall → the read ran on 657 pin-verified rows; 296/1494 rows drift overall — environment moved since the harvest). First read execution INVALID-WIRING (server τ default ≠ trainer selection) disclosed in Bench 119; τ now travel in a digest-checked sidecar.

**Claimed:** zed-225-seat-bench, 2026-10-19T12:45Z, expires 2026-10-21T12:45Z — **EXECUTED 2026-10-19: T2 (A1) + T4 (the frozen read) done, Bench 119**; T3 (A2/Rethink encoder) and T5 (arming) remain as scoped — owner-gated, no lease needed.

## Why now

Plan 202 (R1–R7) landed the whole decision stack and Bench 108 measured it end-to-end over the
live `heal` seat: 9/9 reranks, 1 win / 8 ties / 0 losses, ~6 ms/case, $0 — verdict recorded
**INSTRUMENT-VALID, QUALITY-PENDING**: the seat was the R3 stub (deterministic index-1 pick), so the
modelless incumbent was correctly NOT demoted. Bench 108's own recorded revisit condition is
*"revisit when a real hosted heal expert exists (HOSTED-ONLY tier, riir-train mint lane)"*. This
plan builds that expert — twice (two independent candidates) — and lets the frozen read pick the
winner. The owner's 2026-10-11 directive: remove self-made evals (code_fixtures retired), build a
PROPER bench for the refine decision seat around the Instinct corpus lane and the Rethink encoder.

The oracle makes this honest where code_fixtures was not: **gold is T1.4's three-part definition —
applies + compiles + lint-clears, behaviour preserved (tests pass), and non-degenerate (no
allow/expect, no out-of-span silencing)** — with everything unverifiable in an UNRESOLVED bucket
that never counts as gold. No self-authored gold, no synthetic population (the
grammar-not-answers law), and every claim carries its n beside it.

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

### T1 — mine the heal-decision case set (refine) — EXECUTED 2026-10-11, Bench 113
- [x] T1.1 Harvest real candidate pools: walk `.refine/misses/`, rescue tallies, and past arm-H
      ledgers; for each case capture (snippet, rule/lint id, the L0–L3 candidate list, the
      compile-gate verdict per candidate). Pool-eligibility: `pool ≥ 2` distinct candidates
      (the Bench-107 law — rerank never fires at 1).
      **Result:** the persistent artifacts carry state-level tallies, not per-case pools — the
      harvest REPLAYS the real population instead (the full deduped v2edit corpus, 1500
      modules, through `heal_with_pool` with memory warming) via the new
      `fixer_regate --arm HARVEST`. pool≥2 = 643.
- [x] T1.2 Build the frozen test split FIRST (digest-pinned, cases_digest), never trained on.
      Disclose n per class beside every future claim (the n=32 lesson).
      **Result:** lint-disjoint split (340 test / 303 train scored) — the leakage-safe fallback,
      the v2edit rows carry no source-repo/time provenance; census + split committed under
      Bench 113, cases.jsonl digest recorded there.
- [-] T1.3 If real multi-candidate cases are too few for training: synthesize TRAIN-side rows only
      through the reflex synthesis lane (echo gates + V5 + compile-gate veto). The frozen test
      split stays real-only, never synthetic — one contaminated row kills the bench.
      **Deferred:** supply was not thin, it was wrong-shaped (98.3% merge-fail) — synthesis
      cannot fix pool shape and never fires under the block.
- [x] T1.4 Gold verification harness — COMPILES+CLEARS IS NECESSARY, NEVER SUFFICIENT. A
      candidate is gold iff ALL of: (a) the patched snippet applies, compiles (the `fix_verify`
      posture, `--verify-args` for feature-gated code) and the lint clears; (b) behaviour is
      preserved — the crate's own tests pass on the patched tree (scoped `cargo test` per case,
      budgeted; isolated `CARGO_TARGET_DIR` + wall timeout — a TIMEOUT lands in UNRESOLVED, never
      in failed); (c) the patch is non-degenerate — gold is REFUSED when the candidate adds an
      `allow`/`expect` attribute, deletes code beyond the lint span, or otherwise silences rather
      than heals. Anything (a)–(c) cannot settle lands in an explicit UNRESOLVED bucket, never
      counted as gold (the percentile-audit lesson: UNRESOLVED is not clean). Every non-gold
      candidate records WHICH clause rejected it ((a)/(b)/(c)) — the breakdown shows whether
      (b)/(c) carry signal a compile-only head cannot see.
      **Result:** all three clauses implemented in the HARVEST arm ((a) via `score_case` + the NEW
      `after_compiled` gate; (c) via `contract_ok` + `attr_silence_added`; (b) per-fn differential
      driver on the pilot). En route the harness's compile-failure hole was closed at the seam:
      a type-broken candidate previously read `lint_resolved=true`/`strictly_fewer=true` — 17 such
      candidates refused by the new gate in this census (Bench 113 §oracle-hardening). The
      breakdown answers the clause-signal question empirically: (a) rejects 99.8% of candidates
      before (b)/(c) are even reached.
- [x] T1.5 Scored population + per-case score. TWO scored strata, n reported SEPARATELY for
      each: **deciding cases** (some but not all candidates gold — the stratum that separates
      pickers) and **none-gold cases** (the stratum that rewards ABSTAINING — A0 never abstains;
      its verify-in-order fallback always scores 0 there, so these cases DO separate the arms and
      are scored, never dropped). Pre-registered per-case score: pick-a-gold = 1 ·
      abstain-when-none-gold = 1 · pick-a-non-gold (incl. A0's smallest-diff fallback on
      none-gold) = 0. All-gold cases are unscored structure (every arm ties) — reported in the
      census, excluded from the paired read. Every result reports BOTH n's, never total-n alone
      (Bench 108's 8/9 ties is exactly this blindness).
      **Result:** deciding = 3, none-gold = 640, all-gold = 0 (n's reported separately in
      Bench 113). The two-stratum design proved load-bearing: with 640 none-gold vs 3 deciding,
      an always-abstain expert would have "won" vacuously — the deciding-supply gate killed that
      class before any bench ran.
- [x] T1.6 FEASIBILITY GATE (blocks T2 AND T3): the frozen split must carry ≥ N deciding cases,
      N derived from the power calculation with the inputs PINNED HERE, before any T1 data is
      seen: paired per-case score, superiority margin **δ₀ = +0.05** (five points on the T1.5
      score — a rerank plausibility over drafter ties), target power **0.80**, family α **0.05**
      (Holm, per T4.2), two-sided paired test; the arithmetic is recorded in the bench record.
      Gate fails → the plan is recorded BLOCKED ON CASE SUPPLY — no mint, no training, no bench
      against a split that cannot detect a difference. The owner's T3 approval is conditional on
      this gate passing.
      **VERDICT (Bench 113): BLOCKED ON CASE SUPPLY.** N(p_d=0.20)=621, N(0.35)=1092, N(0.50)=1562
      vs supply 3 (a∧c level) / 0 (full oracle); measured p̂_d = 3/643 = 0.0047. δ₀ NOT lowered
      post hoc. T2/T3 do not run — the block is the gate working (zero GPU spend).
      **RE-COMPUTED (Bench 114, post Issue 161): PASSES.** Supply 1416 deciding (a∧c) ≥ 621 at
      2.3× (also ≥ 1092 at p_d=0.35); clause-(b) pilot P(behav-ok | a∧c-ok) = 0.55 with ZERO
      divergences → full-oracle projection ≈ 779 ≥ 621. The gate unblocks; T2/T3 re-enter
      scope (owner-gated).
- [x] T1.7 Leakage-safe split: split by TIME or SOURCE REPO, never by row; near-duplicate
      removal by blake3 of (normalised snippet + rule) ACROSS train and test; assert no
      self-evolve trajectory contains a frozen case's patch; reflex V5 synthetic rows (if T1.3
      fires) take seeds from the TRAINING side only.
      **Result:** module-level dedup by blake3 at load (1500 unique); the split is LINT-DISJOINT
      (each lint wholly one side — 340 test / 303 train scored), the honest fallback because the
      v2edit rows carry NO source-repo/time provenance; a test rule's cases never appear in
      training in any form, which kills the near-duplicate class by construction. The harvest
      replay's memory writes are run-local (scratch store), so no self-evolve trajectory carries
      a case patch.

### T2 — candidate A1: the Instinct-corpus specialist (riir-train mints, instinct serves)
⛔ Gated on T1.6 (the feasibility gate) — an unproven case supply trains nothing.
   **T1.6 verdict (Bench 114, post Issue 161): PASSES** — 1416 deciding ≥ 621; the pool
   substrate now carries the module-applicable proposer member (pool-only), and the census
   instrument merges whole-module rewrites faithfully (`rewrite` strategy). T2 is IN SCOPE,
   owner-gated as before.
- [x] T2.1 Mine the heal training corpus (TRAIN-side split of T1 only, post T1.7 dedupe): refine
      corpora + misses + self-evolve trajectories (`LatentFixMemory` rows), train-side synth
      augment per T1.3. Corpus manifest + blake3s.
      **Result (2026-10-19, Bench 119):** the corpus is the frozen Bench-114 set's TRAIN side
      with candidate texts regenerated by `fixer_regate --arm POOLTEXTS` (the frozen file
      carries flags + blake3_12, not texts) and pinned per candidate; 291/828 train rows
      pin-dropped (environment drift since the harvest), 1384 pairs over 537 cases trained.
      Input digests in the trainer's stdout, recorded in Bench 119. T1.3 synthesis stayed
      deferred (supply was not thin).
- [x] T2.2 Train the specialist head on the riir-train rigs (4090; small head — the nbsvm-class
      presence-bag shape trained on candidate snippets + rule context is the first arm; scale up
      only if it loses for capacity reasons, not tuning).
      **Result:** trained in riir-instinct on CPU — post-lift the trainer family lives there
      (the plan's riir-train/4090 wording predates the 2026-10-03 lift; the nbsvm-class arm
      needs no GPU — the "scale up only on capacity loss" clause is what would have gone to
      the rigs, and it never fired). 2-class NBSVM presence-bag over the EDIT-DIFF pair text
      (added/removed words vs the module — the full-pair composition measured BELOW-majority
      holdout accuracy, the edit-diff composition measured 1.0000 vs control 0.6053).
      Holdout (case-level, 135 scored): A1 135/135 vs A0 133/135.
- [x] T2.3 Mint the artifact as `heal_winner_v1.bin` (raw-winner route) + serve it behind the
      `heal` suite through the custom `lane_loader` (the R3-proven route; `check_winner_file`
      bridging added ONLY if the raw route refuses). A10: nothing public ever carries this corpus.
      **Result:** minted as `heal_head_v1.bin` (the `_winner_v1` suffix deliberately avoided —
      it would trip the winner_bridge Count-bag default; the RISP format IS the raw-winner
      shape), served via `heal_specialist_server.rs` (suite `heal`, `AnySuiteServer::Ext`, ENC
      manifest row with the REAL digest, custom fn-pointer lane_loader — no dataset seat, no
      ext-boots install). A10: the artifact lives in riir-refine `.refine/heal_seat/`
      (gitignored); the public instinct repo carries only the CODE. The τ travel in a
      digest-checked sidecar the server requires at boot.

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
- [x] T4.1 Arms: **A0** modelless chain + the DETERMINISTIC verify-in-order control — try each
      candidate in pool order through the compile gate, keep the first that passes (else the
      smallest diff); this is what the healer can already compute for itself (`--verify` exists),
      so a learned head must beat it on quality OR match it at strictly lower cost (the
      modelless-first mandate's bar). **A1** + T2 seat · **A2** + T3 seat · hosted generate =
      comparison-only (cost axis, never the winner lane).
      **Result:** A0 derived offline from the frozen gate fields (first `compiled &&
      target_resolved` else the served fallback) — PERFECT on this population (660/660 deciding;
      the pool's first birth is the stage-2 verified fix, and gold_a_c ≡ the gate fields on
      1625/1625 entries), so superiority was structurally out and the bench decided on the
      NI-cost route. A1 = the T2 seat over the LIVE wire. A2/T3 not minted this round (family
      amendment recorded pre-read: Holm m=1).
- [x] T4.2 Protocol: paired per-case over the frozen scored population (T1.5's two strata), ONE
      read, ONE pre-registered comparison rule: **Holm at family α = 0.05** across the two
      candidate-vs-A0 comparisons (no alternate spelling). Superiority = paired LB95 > 0 on the
      T1.5 score. Cost axes with NUMERIC budgets pinned NOW: seat lanes ≤ 50 ms/decision p50 (the
      0.8–6 ms Bench-107 floor + wire slack) and $0 (loopback); the hosted leg priced through
      `llm_spend` with its recorded cap. Two-signal law: accuracy = commissioning; rate = cost
      ceiling only. Seat receipts recorded (decision_blake3 audit trail).
      **Result:** one read, 8.4 s wall (657 pin-verified scored rows; test-side drift 1.4% < the
      5% pre-registered wall). Fresh `EscalationDispatch` per case (the 200-observe latch window
      would RateLatch a ~100%-escalation bench at case 200 — a harness artifact, pre-registered).
      Seat p50 0.13 ms ≤ 50 ms; receipts p50 14 µs; $0 loopback; consent snippet-scope.
- [x] T4.3 Verdict rule (pre-registered): a candidate WINS the `heal` manifest row through EITHER
      route — **superiority**: Holm-corrected paired LB95 > 0 vs A0 on the scored population; OR
      **non-inferiority at lower cost**: paired LB95 > **−δ_NI** (δ_NI = 0.02, pinned) AND
      measurably lower decision cost than A0's verify-in-order compile time (A0 pays a real
      compile per candidate — a cheaper seat matching its quality is the likeliest real win, and
      this route is why the cost axis exists). If BOTH candidates clear a route: the paired
      A1-vs-A2 comparison decides (superiority first, then the non-inferiority route); still tied
      → lower cost, then the simpler artifact (raw winner over trained head). A0 itself is NEVER
      demotable by this bench (it is the floor, not a candidate). If nothing clears: the seat
      stays empty and the result is recorded — **decline is a correct answer** (the L4/603
      precedent). Loser demoted by manifest edit.
      **VERDICT (Bench 119): SEAT WINS via non-inferiority at lower cost** — deciding n=660:
      A1 660/660, W/L/T 0/0/660, paired LB95 +0.000 > −0.02 (bit-identical quality to the
      perfect A0) AND seat p50 0.13 ms vs A0 verify-in-order p50 403 ms (the 20-case
      pre-registered subsample, gates run exactly as the harvest ran them). Superiority
      correctly NOT claimed (LB95 = 0 is not > 0). A0 undemotable, untouched. The first read
      execution was INVALID-WIRING (server τ default ≠ trainer selection) — discarded and
      disclosed in Bench 119; the τ now travel in a digest-checked sidecar, and the winning
      posture is byte-identical to the trainer's pre-read selection (no post-read retune).
- [x] T4.4 Record under a fresh `.benchmarks/` number; the arm-H run-record law (Plan 202 §11)
      satisfied by the same record.
      **Recorded: `.benchmarks/119_heal_seat_frozen_read.md`.**

### T5 — arm + telemetry (OWNER GATED)
- [ ] T5.1 Owner go → the winner's manifest row ARMED for real heal traffic (the first armed
      `heal` row — ALL-EMPTY remains the shipped default until this checkbox). **Bench 119's
      verdict is the input: the A1 specialist cleared the NI-cost route 660/660 at 0.13 ms p50.**
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
