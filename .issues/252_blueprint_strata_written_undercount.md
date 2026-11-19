# Issue 252 — blueprint strata under-count `written`: edits that revert were never counted as written

**Status:** OPEN — filed 2026-11-20 (katgpt-rs-93@amo) from Issue 250 T1a (Bench 162 addendum); unclaimed.

## The finding

`heal fix`'s blueprint row builds `strata[rule].written` from `fixstat_rules` and `strata[rule].reverted` from `report.reverted` (`src/bin/heal.rs`, the `bp_strata` block). In the fleet's 164 rows (Bench 162 data dir), **11 rule strata in 7 runs show `written: 0` with `reverted > 0`** across 6 repos and 2 rigs:
- unnecessary_get_then_check 0 / 16;
- explicit_auto_deref 0 / 3 and 0 / 2;
- redundant_closure 0 / 1 (×4) and 0 / 2;
- map_unwrap_or 0 / 1;
- assert_is_empty 29 / 32.

So some edit producer writes edits that `fixstat_rules` never counts, while `report.reverted` does count them.

Consequences:
- Per-rule revert rates read > 1 (Bench 162: assert_is_empty 1.07, redundant_closure 2.5).
- The 250 T1b similarity arm and any learned arm would train on wrong rule totals.
- Blueprint "rules_needing_eyes" ranking over-weights these rules.

## Tasks

- [ ] **T1** — find which producer's writes bypass `fixstat_rules` (the per-row repro set: the ts values in the Bench 162 data). Count `written` at the same seam the verify batch is built from, so `written ≥ reverted` holds by construction.
- [ ] **T2** — pin it: a unit or integration test asserting `reverted ≤ written` per stratum on a run with a forced revert. Bump `blueprint::ROW_VERSION` if the row semantics change, and note that rows before the bump carry the undercount.
