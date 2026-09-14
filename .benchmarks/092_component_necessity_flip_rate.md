# Bench 092 — Issue 108: the component-necessity flip-rate measurement — 18/18 LOAD-BEARING, zero decoratives, swap≈disable (shape-disjoint donors)

**Status:** RECORD — the T3 full pass over the frozen v1 set (35 ran
fixtures, real-clippy oracle): all **18 presence-ranked targets are
LOAD-BEARING** (every disable loses findings), **zero DECORATIVE** rules
found, and **Δswap ≈ Δdis for every rule** — the highest-co-occurrence
donors are shape-disjoint (rules co-occur because they fire on DIFFERENT
shapes in the same file, so a matched-swap behaves like a disable on the
swapped rule's shapes). The healer's composition carries no dead weight
among its busiest rules on this fixture set — an honest negative for the
decorative-hunt, measured by an instrument whose two-sided canaries were
green both arms.

**Date:** 2026-09-15 · **Box:** M3 Max (macOS, 16 cores), sibling agents
active (4 sessions; CPU ~26% at start — the oracle's stub-crate runs are
serial + tiny, contention-free by isolation) · **Toolchain:** rustc 1.98.1 ·
**Profile:** debug (the oracle's cargo-clippy subprocesses dominate; the
healer's syn parse is not the lane's cost).

> **Issue:** `108` · **Predecessors:** katgpt-rs Research 555 (CVRR) +
katgpt-rs Issue 776 (`be4ff672` — the intervention substrate semantics) ·
**Instrument:** `--necessity-bench` (`src/score_bench/necessity.rs`, Issue
108 T2) · **Data:** `.benchmarks/necessity_history.jsonl` (52 rows total;
the final 37-row block is this measurement — the 6 smoke rows + the 8-row
first-format run precede it, self-describing via `ts`).

## The schedule amendment (measured mid-T3)

The issue's schedule read "top-20 by heal rate + 5 random". The first full
run measured why that cannot fill on the v1 set: the T8 `rule_outcomes`
read attributes SINGLE-TARGET fixtures only, and the v1 set is
multi-target-dominated — **4 solo rows on 35 ran fixtures**. Ranking by
solo heal rate schedules 4 rules and misses exactly where composed
necessity lives (the `needless_return` Δ=−17 is nearly all multi-target
findings). The amendment (landed before the recorded run): rank by
**findings presence** — Σ ran fixtures where the rule is in `vocab_before`
— which counts every at-risk finding, multi-target included. The solo
columns stay the T8 read; the verdict ladder was reordered Δ-first (a
multi-target-only rule with a nonzero Δ is LOAD-BEARING, never
"NO-EVIDENCE" — the first format would have mislabeled 13 of 18 rows).

## The measurement

18 distinct rules present in `vocab_before` across the 35 ran fixtures
(2 of 37 excluded — the oracle's standard no-longer-compiles drift, same
as every v1 bench run); top-20 clamped to 18 present, stride had nothing
to add. 36 arms + baseline + the planted no-op; before-verdicts cached
across arms (one before pass, 36 after passes). Wall: ~3 minutes (the
stub-crate oracle runs ≈1 call/s).

| rule | solo | Δdis | Δswap | verdict |
|---|---|---|---|---|
| needless_return | 1 | **−17** | −17 | LOAD-BEARING |
| identity_op | 0 | −6 | −6 | LOAD-BEARING |
| map_unwrap_or | 1 | −3 | −3 | LOAD-BEARING |
| useless_format | 0 | −3 | −3 | LOAD-BEARING |
| needless_range_loop | 0 | −3 | −3 | LOAD-BEARING |
| collapsible_if | 0 | −2 | −2 | LOAD-BEARING |
| deref_addrof | 1 | −2 | −2 | LOAD-BEARING |
| double_parens | 0 | −2 | −2 | LOAD-BEARING |
| needless_bool | 0 | −2 | −2 | LOAD-BEARING |
| redundant_field_names | 0 | −2 | −2 | LOAD-BEARING |
| redundant_static_lifetimes | 0 | −2 | −2 | LOAD-BEARING |
| filter_next | 1 | −1 | (none) | LOAD-BEARING |
| manual_map | 0 | −1 | −1 | LOAD-BEARING |
| question_mark | 0 | −1 | −1 | LOAD-BEARING |
| redundant_pattern_matching | 0 | −1 | −1 | LOAD-BEARING |
| single_match | 0 | −1 | −1 | LOAD-BEARING |
| unnecessary_lazy_evaluations | 0 | −1 | −1 | LOAD-BEARING |
| unused_unit | 0 | −1 | −1 | LOAD-BEARING |
| planted-noop-control | 0 | +0 | (none) | NO-OP-CONTROL (Δ=0, byte-identical fingerprint) |

Canaries: **load-bearing OK** (top-1 `needless_return` Δ=−17), **no-op OK**
(Δresolved=Δcreated=Δedits=0, per-fixture fingerprints identical — the
determinism proof across arms).

## The three findings

1. **Zero decoratives.** Every rule's disable loses findings — the
   composition routes nothing around any of its 18 busiest rules. The
   CVRR decorative class exists in this pipeline's theory (a rule whose
   shapes other rules re-fix) but not in this population: name-presence
   scoring means each lint's findings survive unless SOMETHING dissolves
   the shape, and no two FIXABLE proposers claim overlapping shape
   territory on this set. An honest negative — recorded, not a failure:
   the instrument detected nothing to demote, and the audit-instrument
   law's canaries prove the instrument could have.
2. **Solo rate massively understates composed importance.** The star:
   `needless_return` solo_resolved=1 (one single-target fixture) but
   Δdisabled=−17 — 17 findings across multi-target fixtures ride its
   slot. `identity_op` (solo=0, Δ=−6) does not even HAVE a solo row. Any
   per-rule pruning decision made off the T8 solo read alone would
   misrank both — this is the "informativeness ≠ use" gap, measured.
3. **Swap ≈ disable: co-occurrence donors are shape-disjoint.** For every
   rule with a donor, Δswap equals Δdis — the donor's collect fn does
   not fire on the swapped rule's shapes, so the slot goes effectively
   empty. The `flips_to_donor` axis (outcome moves TOWARD the donor's
   contribution) is unreachable with co-occurrence-selected donors: high
   co-occurrence correlates with shape DISJOINTNESS (rules co-fire on the
   same FILE, not the same span). A follow-up swap study wanting real
   donor-flip signal needs SHAPE-similar donors (the cross-OOO
   absorption-candidate pairs — Issue 098's high-overlap rule pairs —
   are the natural donor pool).

## Honest scope

- The 36 FIXABLE rules NOT present in any `vocab_before` were never
  scheduled (presence-ranked by construction) — their disable would
  measure Δ=0 with nothing at risk (the NO-EVIDENCE class, correctly
  excluded rather than measured as pseudo-decorative).
- `rust_perf` rules: the clippy oracle cannot see their lane's value
  (the T3-extension gen-5 mechanical-oracle mode remains unwired; the
  `ComponentOverride` seam already covers the domain).
- Δ measures FINDING-count movement (name-presence), not severity or
  latency — the composition question the issue asked, nothing broader.

## Verdicts

- **G1 (instrument):** PASS — two-sided canaries green; the no-op arm's
  per-fixture fingerprint byte-identical to baseline (determinism across
  arms proven, not assumed).
- **G3 (no-regression):** PASS — the harness changes nothing in the heal
  path (empty override byte-identity pinned by test; chain-truth gates
  90/90 green post-refactor).
- **Demote/repair decisions:** NONE warranted — zero decoratives found;
  no follow-up issues to file. The issue's "report decoratives by name"
  clause resolves to an empty list, on record here.
