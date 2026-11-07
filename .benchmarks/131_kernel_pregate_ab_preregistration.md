# Bench 131 — PRE-REGISTRATION: the kernel pregate A/B (file-level gate + span scope) — the G5-cost clearing lane, Bench-129 lever (b)

**Status:** PRE-REGISTRATION LANDED 2026-11-05 (zed-m3-i208pregate, lease on issue 208) — **NO TIMING RUN HAS HAPPENED.** Implementation-first lane: the detector + gate arms land as code commits (issue 208 T1–T3) BEFORE any timed chunk. Nothing in this file may be amended after the first timed chunk (the T1 law: amended pre-measurement, never after). The run record is a SEPARATE later bench document (fresh number at write time).

## What this registers

The A/B that decides whether Bench-129's named lever (b) — a cheaper first-pass scan key, owner-directed 2026-11-05 as issue 208 — clears the G5-cost fence AT THE REGISTERED CEILINGS, with the utility floor HELD. This is the second attempt on the cost axis after Issue 196's route-narrowing measured negative (Bench 126); it is mechanically different: route-narrowing paid per-query routing overhead inside the matcher, the pregate removes WHOLE FILES and SPANS from the kernel matcher's input set at ~lexical cost. The mechanism and its premise are Bench-129's own attribution: "the added latency is the kernel domain's per-span multi-scale query MASS" over a walk whose median file is not kernel code (483/594 files > 1 ms added).

## The pinned contract (unchanged from Issue 191 T1/D2)

- Corpus: THIS repo's `src/**/*.rs` at the implementation commit (the registered walk — grew 565→594 across Benches 125→129; the run record re-counts and discloses).
- Ceilings: added (ON−OFF) p50 ≤ 1.0 ms/file AND p95 ≤ 5.0 ms/file — never re-registered (the 2026-11-04 D2 verdict), same nearest-rank per-file definitions, tail support disclosed.
- Utility floor: 15/15 category experts own ≥ 1 rank-1 kernel suggestion — must HOLD in every ON arm (the gate may not hide an expert's only spans). Rule census (cited/zero-match counts) disclosed per arm, never auto-demoted.
- `top_k = 5`, rank-1 "fires" operationalization, no-min-score law — verbatim Bench 125.

## The arms (three postures, one build family)

| arm | kernel matcher input | feature set |
|---|---|---|
| OFF | n/a (baseline latency, re-measured same session) | `latent_retrieval` |
| GATE | files with `kernel_signal == true` only | `latent_retrieval,kernel_opt,kernel_pregate` |
| GATE+SCOPE | signaled files only; within them, spans within ± 24 lines of a signal line only | `latent_retrieval,kernel_opt,kernel_pregate` (+scope flag ON) |

- The OFF↔ON diff isolates kernel-index + detector cost exactly (the Bench-125 posture-delta design); the detector's own cost is IN the ON arms.
- The GATE↔GATE+SCOPE diff isolates the span-scope axis alone.

## Harness law (verbatim Bench 125/129)

`examples/kernel_goat_g5.rs` (the standing instrument, extended with the gate postures): the production scan path over the sorted walk, one untimed warmup pass then THREE timed passes per posture, per-file medians, top_k=5, census from the last pass; per-posture isolated `CARGO_TARGET_DIR`; `--release`. The three postures run as sequential processes (the sequential-AB confound class, +5–22%) — each takes the median of 3 passes, and the pre-registered read accepts the same load-robustness caveat Bench 129 recorded: the 33× fail margin was orders outside the class; a MARGINAL pass (< 2× headroom over either ceiling) must be re-run alone before any green is claimed.

## Verdict arithmetic (pre-pinned)

- The fence CLEARS iff the WINNING arm's added p50 ≤ 1.0 AND added p95 ≤ 5.0, with the utility floor held in that arm.
- Expected shape (DISCLOSED EXPECTATION, not a claim): GATE passes p50 (the ~86% non-signaled majority drops to ~detector cost); p95 likely stays on a signaled file (kernel-corpus/fixture files ≈ 14% of the walk ⇒ the top-5% tail is all signaled). GATE+SCOPE exists to attack exactly that leg. If GATE+SCOPE also fails p95, the fence verdict is FAIL and issue 208's deferred cap lever (a) is the named next pre-registration — no ceiling re-registration is requested or permitted.
- Per-arm disclosure: added p50/p95 with tail support, files-over-1 ms count, census counts, expert rank-1 distribution.

## Kill criteria

- Any ON arm's utility floor drops below 15/15 → that arm FAILS and the detector's misses are the finding (rulewright improvement queue); widening the needle set is a NEW pre-registration, never an in-place amendment.
- A build red in any posture → LOUD, the leg reads TOOLCHAIN/BUILD-BLOCKED, never a silent skip.
- Any pass with < 2× headroom → re-run alone (the box-state law); a confirmed marginal pass is recorded as marginal, never rounded to green.

## PROVENANCE law (T1 bar 2)

Box state disclosed at run start and end in the run record: load, free/commit RAM, concurrent compute consumers ranked by commit, AC power + `pmset powermode` (the Bench-129 disclosure shape — the bench_preflight script is the reflex lane, not this repo's), toolchain pin, the implementation commit hash, per-arm target dirs, start/end wall clock. A latency number without this block is not a measurement.

## What this does NOT claim

No promotion claim. Promotion still requires BOTH fences (G2 GPU-host per Bench 130's registration ∧ G5-cost) then the re-campaign (G1+G3+G4+G5, Issue 191's sequence). A G5 pass here is one fence lifting, nothing more. ndb's 0.92 ms/file projection (riir-neuron-db Issue 642) is ndb's corpus — the gap settles HERE at this re-measure, per the D2 verdict.

## Refs

- Issue 208 (the lane + the pinned design), Bench 129 (lever (b) naming + the T7 attribution), Bench 125 (the instrument + floors), Bench 126 (the negative first attempt, mechanically distinct), Bench 130 (the disjoint T6 GPU leg), Issue 191 (the re-campaign sequence), riir-neuron-db Issue 642 (the substrate scratch twins, adopted by T7).

---

**Annotation 2026-11-07 (Issue 214 — appended, the registration above is unamended):** the pinned needle set's EFFECTIVE set until `8e781fce0` was the nine letter-initial needles — `@compute` never fired (the `| 0x20` first-byte fold mapped `@` to a backtick, outside the first-byte set). Fixed by an ASCII-letter-only fold; the widening was registered in Issue 214 (C0: 0 file flips / 72 span-seat self-matches on the walk) and cost-gated in [Bench 143](143_issue213_g2_and_issue214_g5_bmo_cost_runs.md) (PASS, all rounds) — the effective set now equals the pinned ten.
