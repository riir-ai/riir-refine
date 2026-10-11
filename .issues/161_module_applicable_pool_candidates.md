# Issue 161 — module-applicable pool candidates: the `heal` seat's pool substrate cannot support a rerank bench

**Status:** LANDED 2026-10-11 (Bench 114) — the pool widening + TWO instrument repairs the first census exposed; T1.6 re-computed on the widened census: **deciding 1416 ≥ 621 → the gate PASSES** (also clears N(0.35)=1092). Filed from Plan 225 T1's BLOCKED verdict (Bench 113); the bench lane (Plan 225 T2–T5) is unblocked behind the owner-gated T2/T3 items.

## The finding (measured, Bench 113)

The harvest replayed the FULL deduped v2edit population (1500 real mined
modules) through `heal_with_pool` with production-shaped memory warming and
scored every pool candidate through the arm-H oracle:

- 2400 candidates, **98.3% fail at merge** — they are corpus-shaped snippets,
  not transformations of the target module;
- **4 gold (a∧c) candidates in the entire population**, 0 under the full
  oracle (behaviour clause);
- deciding cases **3** vs the T1.6 requirement **≥ 621** (p_d = 0.20) —
  BLOCKED ON CASE SUPPLY.

Root cause: `TernaryDrafter::draft_fix` is a RETRIEVER — it emits the corpus
entry's `fixed_code` verbatim (corpus-local fn names), so every pool member is
structurally alien to the module under repair. Memory warming cannot fix this:
on real modules nothing is ever kept, so trajectories stay corpus-shaped and
pools stay corpus-shaped forever.

## The fix direction (substrate that already exists)

Widen the pool's birth points (Plan 202 R6's `heal_with_pool`) with
**module-applicable candidates**:

1. **The per-lint AST proposers** (`domains::clippy_lints::proposers` —
   `collect_edits`/`FIXABLE_LINTS`): span-preserving edits computed FROM the
   target module; they power `--fix` and apply by construction.
2. **The oracle feed's MachineApplicable suggestions** (the P21 lane's
   verbatim-apply shape) where a feed is present.

## What landed (Bench 114)

Three changes; only the first is the pool widening — the other two are
instrument repairs the widened census immediately exposed:

1. **Pool birth point 3 (the proposer member)** — `heal_inner` appends
   `propose_fix(rule, module)`'s output to the POOL ONLY (never modelless
   selection — `heal_with_pool.result` stays bit-identical to `heal`, the
   R6 seam contract; plain `heal` pays one branch, G4 untouched). A
   singleton-on-miss pool exposes the true pool (still below every
   `min_pool` bound). Deduped against drafter+memory members.
2. **The `rewrite` merge strategy** (`apply_edit`) — a response that parses
   as a file and preserves every top-level named item (`use` exempt) is a
   faithful whole-module rewrite and merges WHOLE. Root cause: the snippet
   strategies mangle real module rewrites — `substitute_fn_spans` splits on
   BLANK LINES (a body-internal blank line bails the `?`), then the LCS
   line-merge drops mid-item lines into parseable-but-broken modules. The
   first widened census scored the proposer member `not_compiled` on 91/91
   single_match cases while a direct probe showed the fixes compile+resolve
   ~100% — the instrument, not the proposers, was wrong.
3. **The behavioural driver's real-module blockers** (the clause-(b) pilot
   settled 0/40 before, all `unresolved`): (a) domain extension — floats
   (f32/f64 with signed-zero/NaN/inf corpora), non-u8 integer slices,
   `Vec<int>`, `Option<float>` (the population's top Unsupported blockers:
   26 float fns, 29 int-slice fns); (b) **private-fn visibility** — real
   mined modules declare private `fn`s and every driver call read E0603;
   the driver now publicizes both sides' top-level fns (name-anchored,
   col-0, identical transform, compile-fail = honest Unsupported). Plus
   the pilot tally itself: `deciding_full` was structurally unreachable
   (it required ALL candidates behav-scored, but behav only ever runs on
   gold_a∧c candidates) — now confirmed-based (≥1 full-gold ∧ ≥1 confirmed
   non-gold), and the census carries `p_behav_ok_given_gold_ac`.

En route: `tests/t40_fixer_regate_harness.rs` was repaired — it had been
broken on develop since the 2026-09-18 REACHABLE re-pick (`ptr_arg` →
`unused_self`) left its planted controls referencing a removed fixture id;
the controls now use `manual_strip` (probe-checked to FIRE; the
`clear_with_drain` and `unused_self` re-picks do NOT fire on their fixture
shapes and stay decline-only).

## The re-run census (Bench 114, `--source v2edit_all`)

- pool≥2: 1446/1494 (was 643); **deciding 1416** (was 3), none-gold 29
  (28 = useless_conversion, the one lint with NO proposer; 1 =
  redundant_field_names), all-gold 1.
- T1.6: N(p_d=0.20)=621, N(0.35)=1092 → **1416 clears both**; N(0.50)=1562
  not cleared (a per-case half-win rate that extreme is not the design
  premise). δ₀ = 0.05 NOT lowered.
- The 12-lint mix: 11 lints at 91-203 deciding each; useless_conversion
  0 (no proposer fires on its shapes — a suggest-tier mining lead, filed
  below).

## What remains (follow-ups, non-blocking)

- A `useless_conversion` proposer (76 cases at 0 deciding — the only
  unserved lint).
- The oracle-feed half (MachineApplicable suggestions as pool members)
  stays UNLANDED by design: in the with-feed production posture the P21
  lane already applies those spans verbatim, so feed-candidates-as-pool-
  members would model a posture that does not exist. Disclosed here so the
  T2/T3 owner decision knows the pool's composition basis.

## The gate that re-opens the bench

After the widening, re-run `fixer_regate --arm HARVEST --source v2edit_all`
(the tool is landed, deterministic, ~11 min on the M3) and re-compute T1.6 on
the new census: the gate passes iff deciding ≥ N(p̂_d) with the SAME pinned
inputs (δ₀ = 0.05, α = 0.05, power 0.80 — never lowered post hoc). No mint, no
GPU, until it does.

## Laws that bind

- The rerank wire stays options-in/pick-or-abstain-out (decision_wire v1) —
  pool composition is transparent to the seat.
- A0 stays the floor, never demotable; every widened pool member must pass the
  same three-clause gold oracle the bench scores with.
- No new harness: the HARVEST arm IS the census instrument.

## References

- Bench 113 (the census + verdict + oracle hardening)
- Plan 225 (T1.6 + the amended revisit condition) · Bench 108 (the original
  revisit condition this supersedes)
- `src/drafter.rs` (the retriever) · `src/self_evolve.rs::heal_with_pool`
  (the pool birth points) · `src/bin/fixer_regate.rs` (`--arm HARVEST`)
