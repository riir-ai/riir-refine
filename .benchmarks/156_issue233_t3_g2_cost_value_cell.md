# Bench 156 — Issue 233 T3: the calibrated-dispatch G2 cost + real-scan value cell

**Status:** COMPLETE — verdict **NOT-PROMOTED** (`calibrated_dispatch` stays opt-in, out of the release set); the value leg of the pre-registered GOAT grammar fails on the real corpus. Session: zed-m3-i233-g2, 1794790207.

- **Date:** 2026-11-16 (m3, quiet window, AC, powermode=2(high) disclosed)
- **Issue:** [`.issues/233_sparseread_reading_control_wiring.md`](../.issues/233_sparseread_reading_control_wiring.md) T3 (the G2 cost leg — the gate's remaining half)
- **Instrument:** `examples/calibrated_dispatch_g2.rs` (new; one process = one arm; the runner invokes it at feature sets differing ONLY in `calibrated_dispatch`)
- **Provenance:** preflight PASSED before the series (`power=AC Power load=2.20 swap=0.00M powermode=2(high)`), again after (`load=1.60`); one mid-series event (round b3, see §Noise). Walk: `riir-refine/src` + `katgpt-rs/crates/katgpt-core/src` = **1,371 files**, `top_k` 8, warmup + 3 timed passes per invocation, 6 interleaved invocations (A,B,A,B,A,B), release profile, `CARGO_TARGET_DIR=/tmp/i233g2`.

## The pre-registered gate (issue 233 T3)

"recall@k of the narrowed posture ≥ full fan-out at the floor, plus a measured cost cut on the G2 leg" — bars: added p50 ≤ 1.0 ms / p95 ≤ 5.0 ms per file (quiet-box), utility floor held, feature stays opt-in until the gate passes.

## Verdict rows

| leg | result | verdict |
|---|---|---|
| **Recall (fixture cell, census re-run)** | 46/46 narrowed, recall 1.0000 both postures, 0 full→narrowed losses; pinned rows hold (perf FnDecl 49/49 LCB 0.927, perf HotLoop 24/24 LCB 0.862); fixture cost proxy 0.0% | **PASS** (unchanged from the 11-14 landing) |
| **G2 added p50** | pooled over all 9 round-pairs (3 incumbent × 3 armed medians) **+0.055 ms**; clean-window pairs −0.615 … +0.414 ms | **PASS** (an order of magnitude under the 1.0 ms bar) |
| **G2 added p95** | pooled +42.4 ms is box noise, not the feature: per-file base ≈ 76 ms mean (a 5 ms bar = 6.6% of base — inside the measured cross-invocation band); clean-window pairs 3.77–6.69 ms straddle the bar; round b3 inflated 24% by a cross-run event | **UNDECIDABLE at this noise floor** — and mechanism-bounded: the feature's entire added work is a 3-row table consult per chunk + the retained-rule delta (§value), microsecond-scale |
| **Value ("measured cost cut")** | **no cut exists**: armed drops **24** of 251,697 pool rules (0.0095%) vs the full fan-out and retains **+451 rules (+0.18%) vs the incumbent dispatch** (it refuses 6,343 of 7,077 non-ALL Perf chunks) | **FAIL** — the conjunction's second clause is not met |

## The real-scan value read (deterministic; identical across all rounds within an arm)

10,363 chunks over 1,371 files; 7,077 non-ALL Perf chunks (68%); pool 251,697 rule-slots (mean 35.6/chunk).

| posture | drops | retained | narrowing arms on |
|---|---|---|---|
| incumbent (`span_shape_dispatch`) | **475** (0.19%) | 251,222 | every non-ALL chunk (7,077) |
| armed (`calibrated_dispatch`) | **24** (0.0095%) | 251,673 (+451) | 734 chunks (10.4%); refuses 6,343 |

Read: on THIS corpus the hand-registered dispatch itself barely cuts (475 slots over 7,077 chunks = 0.067/chunk — the 6 annotated rules almost never co-occur in real pools, the landing caveat now measured on the real lane), and the evidence-gated gate narrows 10× less than that. **The calibrated gate's runtime value premise is refuted on the real corpus** — it is strictly more conservative than the promoted incumbent while cutting nothing the incumbent doesn't already cut.

**What the gate IS worth (kept):** the registration BAR — a future shape annotation widens only when its measured group clears the Wilson-LCB floor over oracle-labeled evidence, instead of hand judgment (the Issue-216 widening path can consult `verdict_of` before a row ships). The census now derives clippy FnDecl 47/47 (LCB 0.774) and kernel FnDecl 30/30 (LCB 0.886) as Calibrated — inert rows (the dispatch is Perf-only) but the floor machinery demonstrably derives them from real evidence.

## Noise disclosure (the b3 event; the box-state part of the claim)

Rounds a1→b2 ran 08:26–08:52Z at stable totals (incumbent 104.4–105.5 s/pass, armed 101.3–103.8 s/pass). Round b3 (the last invocation) inflated to 121.8 s/pass (+24% vs b2) — a cross-run event (sibling build activity resumed on the box; the post-series preflight read load 1.60, the mid-series window carried the sibling's `cargo check --profile gate --target wasm32 -p seal-view`). Round a3 (98.2 s/pass) ran BEFORE that event and is 10% below a1/a2. Cross-arm pairs involving b3 or a3 are contaminated; the verdict reads the clean-window pairs (a1/a2 × b1/b2, 4 pairings) for p95 and all 9 for p50. The within-round A→B order carries a ~2–3% warm-cache bias FAVORING the armed arm (B runs second) — against the feature, i.e. the +0.055 ms p50 is if anything an overestimate.

## Re-open triggers (recorded; the standing census is the monitor)

1. The annotated-rule population grows so that evidence-gated narrowing cuts a measurable share of real pools (today: 6 annotated rules, 0.067 incumbent drops/chunk, 0.0095% armed).
2. A consumer wants evidence-gating for a NEW domain's dispatch registration — consult `verdict_of` at registration time (the design's real value), no runtime gate needed.
3. A corpus where the hand-registered dispatch widens UNREASONABLY (a widening the floor would refuse) — the gate becomes the check, still not a runtime win.
