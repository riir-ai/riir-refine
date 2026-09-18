# Research 189 — Batch 162: the frontier-lead mining pack VIII (`bytes_count_to_len` + `bool_comparison`)

**Status:** RECORD (batch landed 2026-09-19, Plan 153; the Batch 161 re-arm's
leads consumed as rules, both families graduated, **the frontier population is
now EMPTY** — the recorded spares are the next re-arm's leads).

## Source

The Batch 161 re-arm's frontier rows (`.benchmarks/frontier_miner_rows.jsonl`,
`3b35e0599fb14afc` / `5b19cbe52cd96362`, construction provenance,
`solving_rule: null`, probe-verified by `scripts/plan152_frontier_probe.py`
2026-09-18). Consumed per the frontier-miner discipline: land the rule →
graduate the family to the HeldOut loop → archive the consumed row →
**record the next re-arm (the spares) — no fresh leads exist this round**.

## The two rules

### `bytes_count_to_len` (complexity, default-on)

`x.bytes().count()` → `x.len()`. The FIXABLE-bar identity is **std's own
definition**: `str::bytes()` yields exactly the elements of `str::as_bytes()`
(the docs define it as that iterator) and `Iterator::count()` on an
`ExactSizeIterator` is `len()` — so the chain IS `as_bytes().len()`, which
for a `str` receiver IS `len()` (the same u8 slice); a `&[u8]` receiver's
`bytes()` re-yields the slice's own items likewise. Bounded to the EXACT
two-call chain: no args, no turbofish, no attrs, no adapter between
(`bytes_mut()` returns `&mut [u8]` — not an iterator — and refuses;
`chars().count()` is NOT the family). Nested-in-larger-expr fires in place
(`s.bytes().count() > 3` → `s.len() > 3`).

### `bool_comparison` (complexity, default-on)

`x == true` → `x`, `x != true` → `!x`, `x == false` → `!x`, `x != false` →
`x` — **boolean algebra on a proven-`bool` operand**: the equality/inequality
with a literal is the identity/negation law of the two-element algebra, and
Rust's `bool` is exactly that (`PartialEq` + `not`; no third value, no NaN
class). Both operand orders carry the same identity (`==` is commutative —
unlike the range rule's two DIFFERENT lattices). The non-literal operand must
be a BARE IDENT proven `bool` by the enclosing fn's signature
(`helpers::scalar_param_heads_over` already admits `bool`; the never-rebound
guard is the soundness leg). Calls, fields, casts, and literal-only pairs
refuse. The `(eq, lit)` truth table: `== true` and `!= false` keep;
`!= true` and `== false` negate.

## The all-graduated state (the first of its kind)

The frontier population is EMPTY — every lead the miner ever filed is now a
landed rule (17 committed rows, all `solving_rule`-set). The e2e's yield
floor (`frontier_yield >= 1`) read zero on a verify-pass and exited 1; the
example now records the state explicitly — `all_graduated` (every committed
row solved, the frontier table empty) satisfies the floor and the verdict
line names it: `ALL LEADS GRADUATED (verify-pass; the next re-arm pair is
owed)`. The next re-arm's leads are the plan152-verified spares:
`manual_string_new`, `needless_late_init` (the twice-used-binding dodge).

## GOAT

- **G1**: module tests ×23 (10 bytes_count_to_len + 13 bool_comparison);
  `all_buggy_codes_fire_their_lint` + `all_lints_fire_and_parse` + corpus
  `all_fixed_code_parses` lockstep at 82.
- **G2**: score-bench row #138 — heal_rate 0.96363634 **FLAT** vs the 96.36%
  baseline, vocab 61 → 63.
- **G3**: freeze e2e v2 PASS (29.9 s) + v3 PASS (26.2 s), **0 drift**;
  frontier e2e **VERDICT ✓** — 36 seeds clean / 17 family loops / 17
  committed rows 0 drift / band fold 17/17; retrieval floors HELD
  (15/15 suites + `retrieval_precision_regression_gate`).
- **G4**: no new deps; span-preserving single-expression replacements
  (interior comments refuse — tested both modules).

Freeze tables: 87 v2 labels / 12 exclusions + 87 v3 / 16. The v2 exclusion
count moved 13→12: a **STRIDE SHUFFLE** — the two new atoms' sorted positions
shifted the composite windows, so 5 old exclusion ids dropped out and 4 new
ids entered (the same interference composites at shifted k + atom
companions; v3 shuffled 4↔4 net-zero at 16). Bad-atom set UNCHANGED (6).

## Fallout repairs (the designed canaries, third firing)

- **RERANK_POOL 24 → 28** + **HEADROOM_DEPTHS +32 rung** +
  **PROD_DEPTH_IDX 4 → 5**: the `pool_depth_rescue_guard` re-fired — the
  pack VIII pair crowded the Issue-024 casualty (`needless_return` +
  `vec-with-capacity`) to first-found at depth **25** (the guard's discovery
  sweep measured it before the assert); 25 is the measured minimum, 28 the
  ladder-rung pin with slack 3. The array gained a rung so the recoverable
  band keeps a rung ABOVE production.
- The splice script (`plan150_splice_freeze_tables.py`) gained a REFUSAL on
  non-id rows under an EXCLUDED header: the first regen run appended into a
  stale out file and would have spliced LABELS rows into the exclusion
  tables — regenerated fresh instead, and the script now fails loud.

## Counts

corpus 80 → 82, FIXABLE 61 → 63, atoms 61 → 63, lib 1966/0 · latent 2119/0 ·
frontier_miner,score_bench 2283/0 · kernel_heal,latent retrieval 15/15 +
regression gate · clippy `-D` clean ×3 states (2 doc_markdown +
2 nonminimal_bool healed in the new modules).
