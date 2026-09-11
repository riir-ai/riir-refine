# Issue 089 — Learnability-frontier band admission for the healer corpus + mining queue

**Status:** OPEN — T1+T2 (the P0 pair) LANDED 2026-09-11 (`--frontier-report`, commit in git log): the report-first instrument exists and took its first measurements. First measurements (the report-first law working): **riir-ai's store: 0.0% band-mass** (21 rules — 20 above + 1 saturated `single_match_else` r=63 — ZERO in the admission band); riir-clippy's own store: 5.6% (1 of 18 in band; 2 saturated: `use_self` r=172, `nan-unsafe-sort-comparator` r=133; 76 score runs, latest heal 96.2%). T4's admission gate would have NOTHING to admit on these stores — exactly why no gate wired. Remaining: T3 (P1), T4 (P2 — additionally blocked: see T2's correlation note), T5–T8.

Distill source: FrogNano/TaskPilot [arXiv:2609.07925] + BugPilot [arXiv:2510.19898], modelless extraction. The frontier estimator is rule-based: empirical resolve rate `p̂` per corpus entry, admission band `0 < p̂ ≤ 0.5`, Beta-LCB low-n protection, one-axis authoring refinement, provenance weighting. Our outcome feed (trajectory store + score history) already records everything the band consumes.

**Design law (from R135/Bench 047): report-first, gate-later.** At real pool sparsity (98.5% unseen) selection sophistication measured ≤0.04pp — evidence volume is the bottleneck. No admission gate wires until the report proves band-mass is predictive.

## Tasks

- [x] T1 (P0) — Report-first frontier stats: per-rule `r / f / n / p̂ / Beta-LCB₅` from the trajectory store + score history; surfaced via `--corpus-stats` extension or a new read-only `--frontier-report` (exit 0, floors on scanned population — never a green zero). **LANDED 2026-09-11**: `src/frontier_report.rs` (ungated pure stats — `summarize`/`render`, 9 lib tests incl. population floor + empty-is-loud) + the `--frontier-report` CLI (bin/heal.rs, the memory-stats pattern: `RIIR_HEAL_TRAJ_ROOT` → cwd, refuses exit 1 on no-store AND on population 0). LCB₅ = `katgpt_core::best_belief_score(r, f, 0.05)` — the shipped Beta posterior primitive at the frontier ε (NOT the 0.32 strategy-ordering ε). Live-validated on riir-ai (21 rules) + this repo's store (18 rules + the score-history line).
- [x] T2 (P0) — Band-mass health metric: fraction of entries with `0 < p̂ ≤ 0.5`; saturation demotion report-only (W=50 trailing oracle encounters at `p̂=1` → demote-to-suggest-only CANDIDATE list, never delete); GO evidence for T4 = band-mass ↔ next-generation yield correlation on ≥3 historical generations. **LANDED 2026-09-11 (metric + candidates)**: band-mass in the report header; saturation candidates = the CONSERVATIVE PROXY `p̂=1 ∧ n≥50` (per-attempt order is unreconstructible from cumulative counters — the Issue 91 T1 mining verdict; all-time-perfect with deep evidence ⟹ trailing-50 perfect; a rule with an early fail then a long win streak is MISSED — conservative for a report-only list). **The correlation GO-evidence is BLOCKED, honestly**: band-mass ↔ next-generation yield needs per-generation band-mass snapshots, and the store holds CUMULATIVE counters — historical band-mass at generation boundaries cannot be reconstructed. From now on each `--frontier-report` run COULD snapshot (the report is cheap); retroactively there is nothing. T4's GO therefore also needs ≥3 future generations of snapshots — recorded as T4's blocker, not silently dropped.
- [ ] T3 (P1) — Generalize Plan 091 `--misses` frontier ordering from rustc_errors-only to all 6 domains (keep the existing per-code classes; this issue's per-RULE band is additive, not a replacement).
- [ ] T4 (P2, GO-gated on T2) — Admission wiring: mining-queue admission `p̂ ∈ (0, 0.5]` with `r ≥ 1 ∧ n ≥ n_min`; `p̂=1` → suggest-only demotion; `p̂=0` → triage fork (`fires=0` unreachable → drop; `fires>0 ∧ resolves=0` expressiveness gap → L3 ruliology or `decline`). Consume katgpt-core `beta_lcb_order_into`.
- [ ] T5 (P1) — Provenance weighting field on mined pairs: organic-feature-churn = 1.0, synthetic/injected = ε; per-provenance yield per batch (paper effect: +2.06 organic vs +0.00 injected — expect organic ≫ synthetic; never inject failures on purpose).
- [ ] T6 (P2) — Closed-form difficulty proxy `D = log₂(1+oracle_calls) + log₂(1+files_touched)` (+ rustc error-chain depth); stratified score-bench read.
- [ ] T7 (P3, optional) — Shaped 0.5 verdict ("target cleared ∧ new diags introduced") as a NEW append-only score-bench generation; half-win feed into Elo (ties) / Beta (half-count).
- [ ] T8 (P3, optional) — Log-length tie-break among oracle-passing candidates: `argmax R(n)`, `R = 1 if n≤N else 1−α·ln(n/N)`, N = per-rule median mined diff, α offline grid-fit on score history replay.

## GOAT gate

- **G1:** in-band yield per mining batch ↑ (admitted batches produce more promoted fixes than the pre-gate FIFO baseline) at **flat binary heal rate**.
- **G2:** false-admit rate ↓ (rules admitted at low n that later measure out-of-band).
- **G3 (no-regression):** corpus membership floor — no entry lost by demotion (coverage is a separate decision); existing count pins unchanged; score-bench history append-only.
- **G4:** report/gate cost bounded (pure arithmetic over the existing outcome feed; zero new stores, zero GPU).

## Cautions

1. Band assumes drafter expressiveness is the bottleneck — keep T4's L3 pressure valve or the corpus starves of hard rules.
2. Saturation demotion never deletes (suggest-only); deletion is a coverage-pass decision.
3. `n_min` protects the log/Beta math at thin evidence (R135 lesson 2: online count-updates only, never frozen rating tables).
4. Mining-side pair validation (research 165 row 5) doubles oracle runs at mining time — price per batch, gate on measured precision gain before adopting.
