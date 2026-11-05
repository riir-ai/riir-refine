# Bench 137 — RUN RECORD: the whole-span query-text cap A/B (Bench 136's registration, executed) — THE G5-COST FENCE CLEARS FOR THE FIRST TIME: the tail 42.8 → 3.99/3.44 ms (~11×/12.5×) at QUERY_TEXT_CAP 2048/1024, confirmed alone; the cap is DETERMINISTIC-LOSSY by paired rank-1 (94–131/664 flips, identical counts across sessions, floor 15/15 held everywhere) — the bundle's promotion posture is OWNER-GATED per Bench 136's pre-pin

**Status:** RUN COMPLETE 2026-11-06 (zed-m3-i208pregate, the Bench-136 lease) — TWO sessions: the
first (00:53–01:04) found both clearing arms under the 2× headroom the prereg's marginal rule
requires re-running alone; the second (01:04–01:13) ran ALONE (load ≤ 2.0, zero cargo/rustc) and
CONFIRMED the verdicts. The fence verdict arithmetic is the Bench-136 pre-pin verbatim; nothing in
the registration was amended.

## The verdict tables (walk 600 files, root `src`, 3 timed passes each, per-file medians, nearest-rank)

Session 1 (load 4.89 falling → 1.98; the marginal-trigger session):

| arm | added p50 ms (≤ 1.0) | added p95 ms (≤ 5.0) | files > 1 ms | cost | utility floor |
|---|---|---|---|---|---|
| OFF | baseline | baseline | — | reference | n/a |
| INCUMBENT (SCOPE + LINE_CAP=2) | 0.201 ✅ | 42.846 ❌ | 171/600 | FAIL | 15/15 HOLDS (664 spans; 462/1061) |
| TC4096 | 0.218 ✅ | 5.741 ❌ | 160/600 | FAIL | 15/15 HOLDS (546/1061) |
| **TC2048** | 0.191 ✅ | **3.985** ✅ | 149/600 | PASS (marginal → alone rule fired) | 15/15 HOLDS (538/1061) |
| **TC1024** | 0.175 ✅ | **3.438** ✅ | 154/600 | PASS (marginal → alone rule fired) | 15/15 HOLDS (546/1061) |

Session 2 — ALONE (the confirmation the prereg's marginal rule demands):

| arm | added p50 ms | added p95 ms | headroom over 5.0 | files > 1 ms | cost | utility floor |
|---|---|---|---|---|---|---|
| INCUMBENT | 0.306 ✅ | 43.233 ❌ | 0.12× | 173/600 | FAIL | 15/15 HOLDS |
| TC4096 | 0.268 ✅ | 6.038 ❌ | 0.83× | 169/600 | FAIL | 15/15 HOLDS |
| **TC2048** | 0.363 ✅ | **4.685** ✅ | **1.07×** | 185/600 | **PASS (confirmed marginal)** | 15/15 HOLDS (538/1061) |
| **TC1024** | 0.218 ✅ | **3.366** ✅ | **1.49×** | 140/600 | **PASS (confirmed marginal)** | 15/15 HOLDS (546/1061) |

- **The fence verdict: PASS — the first cost-leg PASS of the G5 lane** (Benches 129/132/135 all
  FAIL). Per the pre-pinned candidate rule the candidate posture is the LARGEST clearing budget:
  **GATE + SCOPE + LINE_CAP=2 + QUERY_TEXT_CAP=2048** (p95 4.685 alone, 1.07× headroom —
  confirmed marginal). **TC1024 is the sturdier margin (1.49×) and its numbers are named beside
  the candidate** — the 2048-vs-1024 choice is part of the owner gate below (2048 moved +17%
  between sessions; a +7% drift reds it; 1024 moved −2%).
- **Mechanism confirmed:** the incumbent's tail (43.2 ms) is the whole-span + giant-line query
  TEXT mass — bounding it at 2048 bytes cuts the tail 9.2× at IDENTICAL query counts (the
  Bench-135 invariant-cost finding, now measured). INCUMBENT reproduced stable across sessions
  (42.8 → 43.2, +1%).
- **The cap is DETERMINISTIC-LOSSY — the paired rank-1 compare (INCUMBENT ↔ each TC arm, 664
  kernel-cited spans):** TC4096 flips 94, TC2048 flips 109 (16.4%), TC1024 flips 131 — and the
  flip COUNTS ARE IDENTICAL ACROSS BOTH SESSIONS (deterministic: the truncation is a pure
  function of the text, not box noise). 0 spans lost rank-1 entirely in any arm. Cited-set
  shrinkage vs the incumbent: 43/46/56 rules cited by the incumbent but never by the TC arm.
- **The utility nuance the prereg's lossy framing predicted, with a twist:** the truncation
  REMOVES the giant-string-literal dominance of the whole-span queries — the incumbent's giant
  literal bodies over-matched a few rules, so the cited census RISES under the cap (462/1061 →
  538–546/1061; +76–84 newly-cited rules vs 43–56 no-longer-cited). The floor (15/15 distinct
  rank-1 experts) HOLDS in every armed arm in both sessions. Whether the 109/664 flipped picks
  are better or worse is NOT decidable by this instrument (no gold labels on the 664 spans —
  the paired compare is a BEHAVIOR delta, honestly recorded as such).
- **The promotion posture is OWNER-GATED per Bench 136's pre-pin** ("if ONLY lossy arms clear,
  the promotion call is owner-gated with the flip counts named"): no 0-flip clearing arm exists.
  The owner decision: accept TC2048 (candidate, 109 flips/664, 46-rule relative shrinkage,
  1.07× fragile margin) or TC1024 (131 flips/664, 56-rule shrinkage, 1.49× margin) — or decline
  and keep the non-lossy cost lane open (the substrate score-only retrieval lever,
  riir-neuron-db Issue 642, remains the named complementary cut). `kernel_opt` stays opt-in
  until that call.

## The verdict arithmetic outcome

Per Bench 136's pre-pins: both clearing arms PASS both ceilings with the floor held → the fence
CLEARS (marginal, confirmed alone, recorded marginal). The flip discipline fired exactly as
pinned: cost-clear + flips > 0 → utility-LOSSY record → promotion owner-gated. No ceiling
re-registration (forbidden by D2) was requested or performed.

## Provenance (T1 bar 2 — Bench 132's block applies, deltas only)

- Box: m3, AC, `powermode=2(high)`, swap 644.44M. Preflight: `PROVENANCE: power=AC Power
  load=4.89 swap=644.44M canary=skipped powermode=2(high)` (session 1 start). Session 2: load
  1.29–1.98, ZERO cargo/rustc processes, verified before and after (`uptime` quoted in the run
  log). Session 1 disclosed: load fell 4.89 → 1.98 DURING the arms; the alone re-run exists
  precisely because of that.
- Commit `b9c9c033e` (origin/develop, pushed) — the T7 implementation on top of the sibling
  sessions' tier-crate workspace migration + the Batch-336 corpus (kernel_opt 1056 → **1061
  rules** mid-flight; the census rows above read 1061 — disclosed, the prereg's contract is a
  fresh same-session walk at the implementation commit). Worktree
  `/Users/kat/git/riir-refine-i208` (the sibling-layout mirror: a worktree under /tmp cannot
  resolve `../katgpt-rs` — the relocation is part of the lesson).
- Build verification at the commit: clippy `-D warnings` default `--all-targets` exit 0, gated
  (`latent_retrieval,kernel_opt,kernel_pregate`) `--all-targets` exit 0 AND `--lib` exit 0;
  tests default `2857/0`, gated `3624/0` (5 ignored). The gated suite ran twice (pre- and
  post-rebase trees) — 1747 s then 1388 s (the first under sibling-build contention).
- Release builds isolated: OFF `/tmp/t7bench/t_off` (2m00s), ON `/tmp/t7v_wt` (1m56s; binary
  staged `/tmp/t7bench/g5_on_bin`). The ON arms are the ONE ON binary + env literals
  (`SCOPE=1 LINE_CAP=2 QUERY_TEXT_CAP=4096|2048|1024` — the pinned literals; one binary, five
  runs per session).
- Session 1 runs 00:53–01:04, session 2 (alone) 01:04:43–01:11:27, sequential minutes apart,
  per-file medians, top_k=5, nearest-rank p50/p95 + tail support — the standing instrument
  verbatim (`kernel_goat_g5` + `g5_utility_cost.sh`'s adjudication math, extended to five arms;
  walk-agreement + rank1-length REFUSE guards carried).
- The GATE bonus posture (ON, no envs) also ran as the session-1 cache warmup
  (`g5_incumbent_warm.json`) — not an adjudicated row (unregistered posture), disclosed only.

## Reproduce

```sh
# the worktree at the implementation commit (sibling-layout mirror):
git worktree add --detach /Users/kat/git/riir-refine-i208 b9c9c033e
CARGO_TARGET_DIR=/tmp/t7bench/t_off cargo build --manifest-path /Users/kat/git/riir-refine-i208/Cargo.toml \
  --release --example kernel_goat_g5 --features latent_retrieval
CARGO_TARGET_DIR=/tmp/t7v_wt cargo build --manifest-path /Users/kat/git/riir-refine-i208/Cargo.toml \
  --release --example kernel_goat_g5 --features latent_retrieval,kernel_opt,kernel_pregate
cd /Users/kat/git/riir-refine-i208
OFF=/tmp/t7bench/t_off/release/examples/kernel_goat_g5; ON=/tmp/t7v_wt/release/examples/kernel_goat_g5
$OFF --root src --top-k 5 --out g5_off.json
RIIR_KERNEL_PREGATE_SCOPE=1 RIIR_KERNEL_PREGATE_LINE_CAP=2 $ON --root src --top-k 5 --out g5_inc.json
RIIR_KERNEL_PREGATE_SCOPE=1 RIIR_KERNEL_PREGATE_LINE_CAP=2 RIIR_KERNEL_PREGATE_QUERY_TEXT_CAP=4096 \
  $ON --root src --top-k 5 --out g5_tc4096.json
RIIR_KERNEL_PREGATE_SCOPE=1 RIIR_KERNEL_PREGATE_LINE_CAP=2 RIIR_KERNEL_PREGATE_QUERY_TEXT_CAP=2048 \
  $ON --root src --top-k 5 --out g5_tc2048.json
RIIR_KERNEL_PREGATE_SCOPE=1 RIIR_KERNEL_PREGATE_LINE_CAP=2 RIIR_KERNEL_PREGATE_QUERY_TEXT_CAP=1024 \
  $ON --root src --top-k 5 --out g5_tc1024.json
# adjudicate: per-file medians over the 3 passes, added (arm−OFF), nearest-rank p50/p95,
# utility floor == 15, paired rank1_rule_ids compare — the math in g5_utility_cost.sh
```

## What lands from this run

1. **The fence CLEARS (marginal, confirmed alone)** — the composed posture
   `kernel_pregate + SCOPE + LINE_CAP=2 + QUERY_TEXT_CAP=2048` (or 1024) meets both G5 ceilings
   with the utility floor held. First PASS after three FAIL generations.
2. **The text cap ships as opt-in, evidence-carrying** (promote-only env, pinned literals) —
   deterministic behavior, identical flip counts across sessions.
3. **NOT claimable without the owner call:** the bundle's promotion into the Issue-191
   re-campaign — lossy arms only cleared (flips named above); the owner picks 2048 vs 1024 vs
   decline. `kernel_opt` stays opt-in until then.
4. **The named complementary lever (unchanged):** the substrate score-only retrieval lane
   (riir-neuron-db Issue 642) — the non-lossy per-query-cost cut if the owner declines the
   lossy trade.

## Refs

- Bench 136 (this run's registration — nothing amended), Issue 208 T7 (ticked by this record),
  Bench 135 (the invariant-cost finding that re-targeted the lever here), Bench 134 (the
  line-cap pair), Bench 132/131 (the pregate registration + first run), Bench 129 (the
  query-mass attribution), Bench 125 (the instrument + floors), Issue 191 (the re-campaign
  sequence: T6-GPU ∧ the cost lane — the cost lane is NOW GREEN-MARGINAL), riir-neuron-db
  Issue 642 (the substrate lever's home).
