# Issue 209 — the c′ referee: telemetry before/after + structural-identity certificate for single-rule corpus-note edits (issue 206's named follow-up)

**Claimed:** zed-ceeedd52@m3, 1793898463, expires 2026-11-07T17:07:43+00:00
**Status:** OPEN 2026-11-05 — pre-registration filed per the delegated-owner verdict on issue 206 (Claude verdict round 1, **REVISE → c′**, session `411cdf2b`, 2026-11-05): the fixture-set score-bench A/B is structurally UNDECIDED for single-rule arms (the affected stratum cannot reach n ≥ 30 on a finite fixture set) and option (b)'s zero-flip check cannot fail when the edit provably does not touch the bench axis — a check that cannot fail proves nothing. c′ replaces the referee's EVIDENCE SOURCE for single-rule note edits: **(1)** a structural-identity certificate where the edit provably does not touch the bench axis, **(2)** a before/after TELEMETRY referee measuring the note's real effect surface — the rule's own live failure rate. Batch and multi-rule arms are UNCHANGED: the fixture score-bench A/B with paired LCB95 > 0 and n ≥ 30 stays their bar.

## The pre-registered design (pinned BEFORE any data is read)

1. **Structural-identity certificate (the bench-axis half).** When a corpus-note edit provably does not touch the bench axis (the 11-04 case: atom tables are a pure function of `buggy_code`, healed output rides compile-time dispatch — corpus entry TEXT is not an input), the fixture A/B is replaced by the certificate: BLAKE3 digests of the gen-N atom tables + the healed output, baseline vs note-spliced, byte-identity asserted and recorded IN the binding record (the `CorpusBinding` artifact grows a `bench_identity` field). A cert that refuses (bytes differ) = the edit DID touch the axis → the fixture bench runs as usual. No trust, only digests.

2. **The telemetry referee (the advisory-channel half).** The note's real effect surface is the rule's live failure rate — fixture sets are finite, live attempts are not. For a note edit landing at corpus epoch E on rule R:
   - **Split:** R's attempts partition at E (pre-window / post-window), from the same durable stores the `corpus_diagnose` verb folds (`.refine/trajectories.kv` cumulative success/fail per (rule, shape) + blueprint histories).
   - **Bar:** one-sided **Beta-LCB95** (the house bound family, `katgpt_core::beta_lcb_order_into` lineage) on the failure-rate DROP: `LCB95(p_pre − p_post) > 0`, with **n ≥ 30 post-edit attempts** required before the read.
   - **Verdict stays PENDING, never folded into a pass**, until the post-edit window reaches 30.
   - **Confound control (the DiD arm):** each corpus batch also changes the drafter and the corpus, so the raw drop is not attributable. Compare R's failure-rate change against the **co-epoch control**: the median change across other W-class rules with NO note edit in the same window. The read is the DIFFERENCE-in-differences: `(ΔR) − median(Δcontrol)`, LCB on the difference.
   - **Kill window (starvation):** if the post-edit window does not reach n ≥ 30 within **8 corpus batches** (the observed batch cadence makes this ~a month of mining), the verdict is **UNDECIDED-by-starvation**, recorded as such — never silently dropped, never folded.
   - **Vintage:** the recent-window predicate applies (the writer lane's existing discipline) — an edit's telemetry referee only arms when the rule had attempts inside the recent window.

3. **The first enrolled case.** The 11-04 note on `hashmap-with-capacity` (epoch **ac280021**): its post-edit window STARTS at ac280021 — the attempts after that epoch are the n that accrues. It re-enrolls as the referee's first live case instead of staying permanently UNDECIDED. (Its attempts are W-class/compile-gate; the last-seen was 16 d stale at verb time — the vintage predicate gates the READ, not the enrollment.)

4. **Arm B's reopen trigger MOVES HERE** (from issue 206): deterministic demote-execution (the blueprint-named `--active-set-demote` command run programmatically behind an explicit flag) re-opens when a **C-class candidate appears at the pinned predicate in any live store**. Arm B's referee is NOT the telemetry referee — demotion is verified by the score-bench A/B at its own pre-registered bar (the evidence class is the fixture axis, which demotion directly touches).

## Scope restrictions (carried from 206)

- Probe-derivable vocabulary only: templated `context_notes` (arm A) + demote-execution (arm B). Template split / span narrowing / fix content stay EXCLUDED.
- REPORT-ONLY writer: the referee machinery never writes the corpus; note landings stay batch commits + pins-manifest discipline.
- Modelless throughout: attempt counts, Beta bounds, BLAKE3 certificates. No training, no gradient.

## Non-negotiables (house laws honored)

- **No silent bar-bend:** n ≥ 30 and LCB95 survive — only the evidence SOURCE changes (live telemetry instead of an unreachable fixture stratum), pre-registered HERE before any read.
- **PENDING is a verdict state**, not a pass: any consumer of the referee output must read PENDING/UNDECIDED-by-starvation as "no claim".
- The stale-epoch observation (the 11-04 note's epoch label vs the live corpus epoch) is EXPECTED behavior — the compose gate's stale-epoch refusal is the policing mechanism working as designed, not a defect to fix.

## Tasks

- [ ] T1 `CorpusBinding` grows the `bench_identity` certificate field (BLAKE3 atom-table + healed-output digests, baseline vs edited) + the refuse-loud path when bytes differ (the fixture bench then runs).
- [ ] T2 the telemetry fold: per-rule pre/post-epoch windows from the durable stores (reuse the `corpus_diagnose` sweep+fold), the co-epoch W-class control set, the DiD difference.
- [ ] T3 the Beta-LCB95 read + the PENDING/starvation state machine (8-batch kill window, verdict states: PASS / FAIL / PENDING / UNDECIDED-by-starvation) + tests with synthetic stores.
- [ ] T4 the report surface: `corpus_diagnose --referee <rule>` prints the enrolled cases + their windows/verdicts (REPORT-ONLY; the read never mutates anything).
- [ ] T5 enroll `hashmap-with-capacity` @ ac280021 as case #1; record the enrollment (not a read — the read happens at n ≥ 30).

## Prior art

- Issue 206 (the parent — probe-authored corpus repair; the referee's structural UNDECIDED record + the harness that landed).
- Interrupted time series / difference-in-differences on intervention windows (the econometrics standard — the DiD control is the transferable half; no paper pinned, the design is methodological).
- The house bound family: Wilson / Beta-LCB / paired-LCB95 (score_bench, stats.rs).
