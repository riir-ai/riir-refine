# Proposal 020 — Profile-Guided Healing: hotpath as the healer's dynamic oracle (the catalog+modelless quadrant)

**Status:** PROPOSED — rev 1; Claude verdict recorded in §Verdict (ping-pong closed before commit); owner addition 2026-10-04 ("we even can use rethink") folded as §5; **alignment re-call 2026-10-04** (post-take-over, §Verdict r2: the G4-**ceiling** correction — the rethink pin asserts `<= 190`, not equality — + the L5-frontier composition boundary)
Branch: `develop` (per global rule)
Owner: unassigned — **POC-gated**; Issue 146 is the execution home
Fusion of: [Research 235](../.research/235_profile_guided_healing_fusion.md) (the fusion note — full prior art + the 13 hazards + the 2-round verdict) × Issue 146 (T0) × [Research 234](../.research/234_hotpath_rs_repo_distill.md) (the hotpath repo distill; R032/R081 the earlier hotpath-sourced pair) × the `--clippy-json`/auto-oracle anchor-lane pattern × `self_evolve`/`traj_store`/`elo` (the shipped credit substrate) × [Proposal 019](019_refine_rethink_one_plane.md) (the one-plane law — §5's boundary) × [Proposal 021](021_decision_cpi_escalation_tier.md) (the sibling decision-CPI lane — P021's T3 frontier-triage escalation composes with this lane's hot-span anchoring; the two lanes' T3 composition is recorded in both; **the composition is L5-frontier-ONLY** — hot spans with no corpus rule, outside the fix/credit path; any fix chosen through escalation is tagged and kept OUT of this lane's modelless evidence and Elo credit)

## TL;DR

The healer today fixes what **matches** — clippy diagnostics, rustc error codes, rule similarity — and has **no view of what is actually hot at runtime** (grep-verified: zero dynamic-profile consumption in `src/`). Wire the loop to measured reality: `cargo refine --hotpath-json <report>` anchors spans at measured-hot sites as a SECOND oracle lane (the clippy-json pattern, zero new deps — the user runs hotpath, `hotpath init --agent` automates instrumentation), the rust_perf corpus reasons on them (modelless — the corpus IS the reasoning; no LLM anywhere in the fix/credit path), the fix is verified by **re-profiling** (alloc-delta with N≥3 agreeing runs + a timing/RSS no-regression guard + the repo's tests), `self_evolve` records measured-impact credit, and unhealed hot spans become **real mining leads** (L5). The loop shape is established 2025-2026 architecture (Google ECO, Uber PerfInsights, MOA — all LLM-driven); the **bounded-catalog + modelless quadrant is unoccupied** per the sweep, and that + L5 (corpus growth at *currently-hot* spans vs ECO's historical commits) + persisted Elo/Beta credit is the delta. POC-gated: nothing beyond T0 is committed until T0 shows ≥1 real alloc-axis improvement end-to-end.

## The loop

```
user workload --(hotpath report JSON, SHA-keyed)--> [L1 anchor hot spans]
  --> [L2 corpus retrieval reasons on hot spans (modelless)]
  --> [L3 fix --> re-profile --> alloc-delta verified --> Elo/Beta credit (self_evolve)]
  --> [L4 iterate; MCTS over (site × rule) ONLY if fix interaction is measured]
  --> [L5 unhealed hot spans = real mining leads --> the corpus grows where reality is hot]
```

## Position (full prior-art tables in Research 235 §3)

| | LLM in the loop | Modelless |
|---|---|---|
| **Bounded fix catalog** | ECO ✓ · PerfInsights ✓ · MOA ✓ (deployed, wins at scale) | **THIS — unoccupied per the sweep** |
| **Free-form edits** | PerfBench ✗ (3% success) | — (nonsensical) |

Deltas beyond the quadrant: **(a)** no LLM anywhere in the fix/credit path (also the modelless-first mandate — the LLM advisory only ever sits at the L5 frontier, opt-in); **(b)** L5 — the corpus grows at currently-hot unhealed spans rather than historical commits; **(c)** persisted per-rule↔site Elo/Beta credit (later: impact-verified fixes as a KAT trust signal). Three checks stay OWED (at plan/conversion): the ECO dictionary-growth section read (could narrow (b)); the classic non-LLM lineage re-check (Toddler / Caramel / Coccinelle perf patches / clippy's `perf` group — none profile-anchored or corpus-growing per their shapes, but the record must say so from a read, not a shape); the quadrant claim is sweep-limited (web + workspace).

## Layers (one line each — full table in Research 235 §2)

- **L1 hotness oracle** — `--hotpath-json <report>`; timing (calls × avg) is RANKING-only (B129: instrumented attribution ≠ production times); alloc bytes/count is the deterministic verification axis (R032); report keyed to repo SHA/per-file blake3, REFUSED on mismatch.
- **L2 reason** — rust_perf corpus + retrieval + `context_notes` regime matching gains a profile context (hot loop / contention-bound / alloc-heavy / poll-heavy).
- **L3 credit** — `self_evolve` outcome rows gain a measured-impact dimension; ONE fix per re-profile run (attribution); held-out second workload for Certified; the tier_guard no-free-rider law carries over.
- **L4 sequential** — greedy → beam/greedy-with-rollback → MCTS over (site × rule) **only on a measured T2 interaction** (perf fixes interact: each fix moves the hot spot); T3's first task is the katgpt-core `mcts` API-fit check — do not assume.
- **L5 corpus growth** — `[no corpus rule]` at a measured-hot site = a product-grade gap signal feeding the frontier miner + `--misses`; **local-first** (user code never enters a shared corpus unless opted in).

## §5 The rethink axis (owner addition 2026-10-04: "we even can use rethink")

Two bindings and one boundary:

1. **The WORKLOAD binding — rethink's serve lane is the first REAL profiled workload (Issue 146 T0.6).** The toy-repo e2e stays T0's deterministic gate; the rethink arm is the real-workload validation: profile `rethink serve` (or the arena seat loop) on the M3 at a PINNED SHA, feed the report, heal, re-profile. Why rethink: **owned** (a real product lane, not a synthetic fixture), **genuinely hot** (the encoder serve is ms-class per case — riir-instinct Bench 036's xnli 231 ms p50; rethink Bench 029's sst5 encode 14.7 ms), and **dogfooding** — internal evidence the loop works on a real lane before any user repo. The privacy direction is the OPPOSITE of free: riir-rethink is **private-forever moat** (its BOUNDARY.md: "private forever. Never public. No exceptions.") — L5 leads and any corpus rule mined from rethink spans **stay LOCAL and never enter riir-refine's shipped corpus or KAT fixstats unless the owner opts in**.
   - **The verification readout is the G4 test's printed `max_cheap`, with the pin as the no-regression ceiling.** `riir-rethink/tests/esc_g4_alloc.rs` (sibling-session, Issue 017 T5, **committed at rethink `f23df0a`** — the ESC T5 promotion) asserts `max_cheap <= PINNED_MAX_CHEAP_ALLOCS` where the pin is **190, the deterministic measured value, enforced as a CEILING** (the const's own doc: "the per-cheap-decision allocation ceiling"; a red means a NEW allocation class joined the hot path). So: the **oracle is the test's printed `max_cheap`** (deterministic across runs — the N≥3 rule trivially true; a heal reads as the printed value DROPPING), the **ceiling is the no-regression guard** (a heal that reduces allocations stays green — the pin does NOT go red on a heal), and the pin's doc names every allocation class — a ready-made target inventory. The r2 fallback clause ("if the pin never lands, hotpath counts + N≥3") is **satisfied and retired at `f23df0a`**. Consequences T0.6 records: **the re-pin after a heal is an OPTIONAL TIGHTENING of the ceiling** (measure the new value, inventory, pin value + margin — the assert message's own recipe), still coordinated with the owning session since it edits their gate; and the **posture is recorded** (the pin was measured at the metal posture — allocation classes can differ between postures).
   - **Cross-repo landing rule (the Issue-798 law).** The heal is only "landed" when it is COMMITTED in riir-rethink and the refine record cites that rethink commit SHA. The tree is actively edited by the Issue-017-T5 session (`serve.rs`, `encoder_serve.rs`, `escalating_backend.rs`, `decstat_verify.rs` — exactly T0.6's targets): coordinate with that session before editing, and keep hotpath instrumentation (`#[hotpath::measure]` + the dep) **worktree-only, never committed** — hotpath is not on rethink's dep allowlist, so committing it would need a boundary row first.
   - **decstat is OUT of T0.6's target set.** It is consent-gated (`RIIR_INSTINCT_STATS=on` + `INSTINCT_ACCOUNT_KEY`): profiling it would mean turning consent on and pushing synthetic-workload stats into the real economy flywheel over the riir-kat wire. Revisit only with a local no-push sink.
   - **Dependency spans are DECLINED.** Hot spans resolving into `riir-reflex/laya-riir` or riir-instinct code (the encoder forward and the HTTP edge live there) are never edited — the healer edits only the repo it was pointed at; such spans are filed as leads in the OWNING repo.
   - Honest scope: the GPU forward is kernel-bound and reads as ranking-only attribution under B129; the alloc-axis targets are rethink's own Rust-side paths (envelope/serialization, seat orchestration, the response paths — tokenization included only where it lives in rethink).
2. **The CREDIT binding (T1, later).** A measured, verified improvement to the rethink serve path is the cleanest Certified-credit shape we own: two workloads are trivially available (the arena seat loop vs `serve` HTTP traffic) and the box state is ours to record (`bench_preflight` posture). The credit is recorded INTERNALLY; any T4 publication of rethink-derived numbers into KAT fixstats is owner-gated by the private-forever law above.
3. **The ONE-PLANE boundary (Proposal 019).** Profile-guided healing is refine-side; rethink does not heal, so **no lane-generic split is owed now** — the new surfaces (`--hotpath-json`, SHA-keyed reports, measured-impact outcome rows) are refine-local, and pre-abstracting them into kat traits ahead of a second consumer would violate 019's own economy ("per-lane parameters where the products genuinely differ"; `reflex`/`rethink` adoption of any shared piece is optional and late, never a blocker). If a second product ever *consumes* profiles, the 019 lane pattern (traits + wire in kat, impls in the products) applies THEN, as a new decision.

## Hazards as gates (all 13 in Research 235 §4)

The load-bearing five: **alloc-reward gaming** → timing/RSS no-regression beside the alloc axis; **compiling ≠ behaving** → the profiled repo's test suite gates every kept edit; **stale profiles** → SHA-keyed reports refused on mismatch; **credit attribution** → one fix per re-profile run; **workload overfitting** → held-out second workload for Certified credit. Plus: **near-determinism** → alloc deltas need N≥3 agreeing runs (the rethink arm's G4 pin makes it deterministic there); bench-target guard (P30) routes profile-guided edits in `benches/`; the fix lane never edits hotpath's own attributes; instrumentation coverage bias is disclosed, never treated as exhaustive. (All 13 in Research 235 §4; MCTS-fit is §Phases T3's own gate.)

## Phases

- **T0 (Issue 146 — the POC gate)**: parse + SHA-key the report; anchor `[hot]` spans through the existing feed lane; guards; the toy-repo e2e (alloc-delta N≥3 + no-regression + tests) **+ T0.6 the rethink serve-lane real-workload arm** (G4-pin-oracle, cross-repo landing, private-forever); CLI + contract tests. GATE: ≥1 real alloc-axis improvement end-to-end, zero new deps.
- **T1**: self_evolve measured-impact outcome rows (one-fix-per-run; held-out workload for Certified; tier_guard carries over).
- **T2**: the iterative loop + the **recorded go/no-go interaction test** (greedy vs greedy-with-rollback vs shuffled on cumulative verified gain) — the decision, either way, is written into the issue.
- **T3**: katgpt-core `mcts` API-fit check, then MCTS over (site × rule) only if T2 measured interaction.
- **T4 (optional)**: KAT fixstats carry measured deltas; frontier-miner real-profile leads productized (L5, local-first).

## The plan-yield note (why no `.plans/` number yet)

Batch 201 is claimed and in flight on the 4090 with `.plans/200_batch201_mlxfast_mtp_slice.md` **uncommitted** and `.plans/.highwater` already bumped to 200 there (queue snapshot, 2026-10-04 row) — allocating a plan number from the M3 now is exactly the dual-allocation hazard the discipline names, and `dual_allocation_gate.py` cannot see an uncommitted bump. The plan opens at B201 conversion; until then **Issue 146's task list IS the execution state** (issue-first, Research 235 hazard 2). Nothing is lost: this proposal carries the phases, the issue carries the checkboxes.

## Gates

- T0's gate (above) — the fusion's only committed claim until it passes.
- All existing healer gates green through the landing (lib counts re-pinned; the e2e contract tests at the process boundary, the `repo_posture_cli.rs` hermetic pattern).
- The oracle-lane contract: a clean/absent report degrades LOUDLY (the `OracleOutcome` precedent); a manual feed wins over the auto-lane.

## Honest caveats

1. **Q2 is unmeasured until T0 runs** — "the healer fixes real hot paths" is asserted-by-design until the e2e gate reads a real delta. Hence POC-gated, issue-first.
2. The quadrant claim is sweep-limited; the three owed reads (§ Position) could narrow it — none is expected to kill it (the sweep found no profile-anchored, corpus-growing, modelless catalog).
3. hotpath sees only annotated functions (~40 ns/call overhead) — coverage bias disclosed; pure-compute workloads (no alloc signal) fall back to ranking + score-bench microbenches.
4. MCTS may not fit the action space — T3's first task is the API-fit check; shipping MCTS over independent fixes would be architecture theater.
5. The rethink workload arm depends on sibling-session coordination (a pinned worktree, the G4 re-pin, the Issue-798 landing rule) — disciplines, not blockers; its value is bounded to INTERNAL evidence by the private-forever law.

## Verdict (ping-pong)

**Claude verdict: AGREE (round 2, 2026-10-04 — session `93a8e8b3`, REVISE→AGREE).** Round 1 (REVISE) caught six real defects, all verified against the tree before incorporation: the privacy direction was BACKWARDS (rethink is private-forever moat — "publish the case" deleted, L5/local + owner-gated publication clauses added); the rethink arm's best oracle was missing (the G4 alloc pin at exactly 190, deterministic — now THE verification oracle with the re-pin-lands-with-the-heal consequence); no cross-repo landing rule (the Issue-798 SHA-citation law + sibling coordination + worktree-only instrumentation added); the decstat consent hazard (dropped from the target set); dependency-span leakage into riir-reflex/riir-instinct (declined + filed in the owning repo); and an unqualified cross-repo bench citation (now "riir-instinct Bench 036"). Round 2 confirmed all fixes re-read in the files and added three pre-commit items — the concurrent `.proposals/021`/`.issues/147` sibling allocation is NOT staged here (own-blob highwater commit), the G4-fallback clause (pin at its committed SHA, else hotpath counts + N≥3), and a bindings miscount — all applied. Plan-yield (no `.plans/` number under B201's uncommitted bump) and the 13-hazard carry-over were agreed both rounds.

**Alignment re-call (2026-10-04, post-take-over): REVISE→AGREE.** Round 1 caught one real factual defect inherited from the take-over: the G4 pin was described as "exactly 190" with "any real heal turns this exact pin red" — verified against the committed source (`f23df0a`): the assert is `max_cheap <= 190`, a **CEILING** (the measured 190 is deterministic but the gate is an upper bound). Corrected: the oracle is the test's printed `max_cheap` (a heal reads as the printed value dropping, N≥3 trivially true), the re-pin after a heal is an OPTIONAL TIGHTENING (not a forced red), and the untracked-file fallback clause is satisfied + retired at `f23df0a`. Also added this round: the **L5-frontier-ONLY composition boundary** with P021 (hot spans with no corpus rule; escalated fixes tagged, kept out of this lane's modelless evidence and Elo credit), the **clean-baseline condition** on P021's T0′ (runs with no `--hotpath-json` feed, at a pinned refine SHA), and **T0.8's disposition on a NO-GO** (moves to its own issue — standalone corpus value; the tier closes, the contribution lane does not). All corrections applied to §5 + Issues 146/147 + P021.

## TL;DR (closer)

Verdict: **GOAT-tier fusion for the healer, POC-gated, modelless-first** — occupy the catalog+modelless quadrant, grow the corpus where reality is hot (L5), persist credit on measured impact. T0's deterministic gate is the toy-repo e2e; the rethink serve-lane arm (T0.6) is the real-workload validation on an owned, private-forever lane. Zero new deps; every hazard named upfront and encoded as a T0/T1 gate; nothing beyond T0 committed until T0 reads a real alloc-axis improvement.
