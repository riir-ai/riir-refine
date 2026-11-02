# Bench 124 — the kernel_opt GOAT-gate campaign (Issue 191 T5): G1–G4 measured, G2+G1 FAIL, promotion REFUSED (the domain stays opt-in)

**Status:** CAMPAIGN COMPLETE — VERDICT RATIFIED: the delegated owner-verdict review (native
fallback sub-agent, session 04ae2ccf, 2026-11-01 — the request_verdict claude_code channel
auth-unavailable on this box, the Issue-1115 arc) returned **#Verdict: AGREE** on all five
checks (numbers reconcile across Bench 124 ↔ Issue 191 ↔ Issue 193 ↔ harness code incl.
248+803=1051; the G2 FAIL honestly attributed; the G5 deferral disclosed in both surfaces;
no silent bar weakening — the floor correction documented pre-measurement in issue+code;
nothing owner-gated decided without the owner). Two non-blocking nits: the ≥290/≥74→≥70
historical reference (fixed same session) and exact per-class counts pinned at Issue-193 R6.
**Follow-up verdict 2026-11-02** (same delegated channel — native fallback sub-agent session
dfc6bea6, the claude_code request_verdict auth-unavailable on this box): **#Verdict: AGREE**
on the three remaining owner-gated calls — the G2-convention closure (§Next item 2, option c),
the G5-ceiling non-re-registration (§Next item 3), and the amo hand-off for the substrate-side
lever (§Next item 3) — records verified at develop 49d4dc99.
Claimed + executed by
zed-shikuwa-i191gate (epoch 1793523566) on the 4090 box (shikuwa), 2026-11-01.

## The verdict table

| Gate | Bar (pre-registered T1) | Measured | Verdict |
|---|---|---|---|
| **G1** correctness | every fixture rule compiles + its own asserts pass under the cascade; census reconciliation; floors ≥ 248 rules / ≥ 72 files | 248 fixture-bearing rules / 72 files (floors hold); **183 green / 65 FAIL** (compile or assert); 803 parse-only disclosed; 209.9 s; census reconciled (gpu_63/64 pinned at 1 each) | **FAIL** — 65 parse-valid fixtures are compile-invalid or assert-failing (Issue 193; 3 re-verified standalone → fixture bugs, not harness artifacts) |
| **G2** perf | CPU-portable ref-vs-opt microbench, interleaved 15-pair median ≥ 1.06 at release, deterministic tranche (15 files), rig exclusivity mandatory | tranche = 15 files / 74 green rules; **21 measured / 1 passing / 7 regressing / 53 not-measured** (no mirrored fn pair); tranche median **1.0000 < 1.06**; best: fp32-pipe-mma-role-split 2.05× | **FAIL** — the GPU-shape wins do not transfer to the CPU-portable surface (bar 2's own caveat: "a GPU-shaped rule's win is not CPU-measurable") |
| **G3** no-regression | clippy_lints + rust_perf score-bench verdicts byte-stable with kernel_opt ON (existing domains, disclosure boundary) | `scripts/g3_scorebench_stability.sh`: clippy lane 40 verdict lines byte-identical, perf lane 67 byte-identical | **PASS** |
| **G4** alloc-free | default build (kernel_opt OFF) byte-identical — compile-time-only cost | structural: the whole domain compiles only behind `kernel_opt`; the default clippy gate is clean and untouched | **PASS** (structural) |
| **G5** utility/cost | match census on a pinned corpus + p50 ≤ 1 ms / p95 ≤ 5 ms per file + ≥ 3-expert utility floor | **NOT RUN this campaign** — measured 2026-11-01 as **Bench 125** (the disclosed deferral, run early because it is convention-independent): utility **PASS** (15/15 experts at rank-1 over 4,055 kernel-cited spans), cost **FAIL** (added p50 38.795 ms / p95 177.612 ms — 39× over the ceiling; mechanism = the flat 1051-rule fan-out, repair filed as Issue 196). The repair measured 2026-11-02 as **Bench 126** (Issue 196 closed MEASURED-NEGATIVE): narrowed index FAIL, slower than flat, 14/15 — the cost fence stays up (Bench 124 §Next item 3) | **FAIL** (cost leg; utility leg pass — Bench 125; the repair path measured negative — Bench 126) |

**Promotion verdict: REFUSED — `kernel_opt` stays opt-in**, failing axes named: G1 (65
fixture defects) and G2 (no CPU-demonstrable median win). Per Issue-191 bar 6: "If any gate
fails, the domain stays opt-in with the failing axis named in the record."

## What the campaign measured (the honest reads)

1. **The gates WORK — both failures are the gates doing their job.** G1's first standing run
   found the corpus's fixture-compile debt (the per-entry gate was parse-only, never
   compile-enforced). G2's first standing run measured the CPU-portable surface and found it
   flat: 20 of 21 measured rules sit at 1.00±0.01 (the opt form is the same scalar code or a
   GPU-only shape), one at 2.05×, seven slightly below 1.0 (noise or CPU-hostile reordering).
2. **The not-measured majority is the real G2 blocker.** 53/74 tranche rules expose NO
   mirrored fn pair — their fixtures are test-shaped (asserts over inline code), not
   bench-shaped (a callable ref/opt pair). A future G2 pass needs either a bench-entry
   convention in the corpus (owner-gated amendment) or GPU-host benches (the named Tier-B
   re-gate trigger), not more harness work.
3. **The one PASS (fp32-pipe-mma-role-split, 2.05×)** is exactly the kind of rule G2 exists
   to find: a CPU-meaningful arithmetic reordering (biased-bf16 operand split) with a real
   scalar twin. The tranche discipline surfaced it deterministically.

## Provenance

- Box: shikuwa (4090, Windows 11, i7-13700K 24 cpus, 32 GB) — verified QUIET via
  `agent-mail rig status` (no heavy compute consumers) before the G2 full run; the G1 full
  run and the G3 arm ran under the same posture.
- Toolchain: rustc 1.99.0 (b940084d7 2026-09-28), the repo pin.
- Harness commits: T2 `a9703c00a` (G1), T3 `82b77f88b` (verdict store + annotation), T4
  `2ff3c0a61` (G3 arm), T5 `96f3733ee` (G2 harness) — rebased onto origin/develop
  05a65e1ba, pushed.
- Reproduce:
  - G1: `cargo run --release --example kernel_goat_g1 --features kernel_opt -- --full --out <json>`
  - G2: `cargo run --release --example kernel_goat_g2 --features kernel_opt -- --full --out <json>`
    (`--tranche` prints the selector disclosure)
  - G3: `sh scripts/g3_scorebench_stability.sh`

## Next (the re-campaign path)

1. ~~Issue 193 repairs (R1–R5) → G1 green~~ **DONE 2026-11-01** (Issue 193 closed:
   65/65 repaired, zero demotions; full G1 248 green / 0 FAIL / 803 parse-only in 100.8 s;
   closing commits `50ba6983fb8`/`90c68b7f74e`/`84e4432dfe9`/`a82a5b29bd5`).
2. ~~The G2 convention decision (owner-gated)~~ **CLOSED — option (c), close the CPU-G2 axis
   with the disclosure** (delegated owner-verdict review, native fallback sub-agent session
   dfc6bea6, 2026-11-02 — the request_verdict claude_code channel auth-unavailable on this
   box, the Issue-1115 arc; **#Verdict: AGREE**): the measured median 1.0000 STANDS as the
   CPU-portable answer — the verdict-table row above is history, not erased, and the one
   2.05× scalar-twin finding (fp32-pipe-mma-role-split) stays on record; a future G2 re-gate
   rides the Tier-B GPU-host trigger (option b) if kernel_opt ever approaches promotion —
   NEVER a CPU tranche re-run. The corpus bench-entry convention (option a) declines with
   the same measured reasoning: 20 of 21 measured rules sit at 1.00±0.01 (the GPU-shape wins
   are structurally not CPU-demonstrable), so even full bench-entry coverage moves only the
   scalar-twin subset and the CPU median stays ~1.0.
3. ~~G5 census~~ **MEASURED — Bench 125** (2026-11-01, early because convention-independent):
   utility PASS, cost FAIL (added p50 38.8 ms ≫ 1.0 ms ceiling) — the re-campaign is now
   fenced on TWO axes: G2 (convention pending, item 2) and G5-cost (Issue 196's
   router-narrowing repair path; ceiling re-registration is an owner act if the narrowing
   alone cannot reach the bar).
   **G5-cost repair path MEASURED — Bench 126** (2026-11-02, Issue 196 closed
   MEASURED-NEGATIVE): the narrowed index (TF-cosine top-6 routed slices + stride tail, the
   K-sweep's gate-holding point) FAILS the same ceilings and is SLOWER than flat (added p50
   45.8 ms vs flat's 36.3 ms — the 7-calls-per-query × per-call floor beats the ~2× mass
   cut) and drops the utility floor to 14/15. The fence stays up; the named next lever is
   OUTSIDE this repo's index shape — substrate-level score-only retrieval (riir-rag,
   riir-neuron-db's lane) or a multi-scale query-mass lever; ceiling re-registration remains
   an owner act and is NOT requested (the residual is 46× the ceiling, wrong direction).
   **Confirmed by the same 2026-11-02 delegated verdict (#Verdict: AGREE): no
   re-registration — the 1.0/5.0 ms ceilings stay the contract for any future cost-axis
   mechanism; with the G2 axis closed (item 2), the re-campaign fence is single-axis
   (G5-cost).**
4. Re-campaign: G1 + G5 re-run in the same run (G2 is CLOSED per item 2 — its future re-gate
   rides the Tier-B GPU-host trigger, never a CPU tranche re-run). NOTE: the G1 digests moved
   with the 193 repairs — the G5 corpus pin re-derives in the same re-campaign run.
