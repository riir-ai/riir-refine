# Plan 167 — Batch 173: serde zero-copy family distill (borrow-deserialize + RawValue passthrough)

**Status:** QUEUED — verdict landed 2026-09-23 (Research 204, YES B− / seeds the serde family); batch executes at the standing cadence gate or owner GO. Source: serde official docs (serde.rs lifetimes + `#[serde(borrow)]`) + serde_json `RawValue` docs. Lead: `serde-borrow-deserialize` (GOAT 4/4). Yield estimate: 2–3 rules.

## Tasks

- [x] T1 Verdict + first-pass dedup (both vocabularies: id-level `serde|borrow|zero|cow|deserial`, content-level `deserial` / `RawValue|visit_borrowed|serde_json`) — **DONE 2026-09-23 in Research 204: PASS, zero covering rule; distinct-from paid to `avoid-clone` / `cow-conditional-alloc` / `encoded-layout-borrowed-view` / `group-keys-borrow-allocate-on-first-sight`**
- [ ] T2 Re-verify quotes at mine time: re-fetch serde.rs/lifetimes.html + the RawValue docs page and re-read every quoted sentence in Research 204 (docs are canonical/versionless, the standing discipline applies anyway)
- [ ] T3 Re-run the dedup gate (the queue snapshot's standing rule): a fresh batch may have landed since 2026-09-23 — re-grep both vocabularies before the first corpus edit
- [ ] T4 Author candidate 1 `serde-borrow-deserialize` (allocation): ORIGINAL fixtures — owned-`String`-fields struct vs `&'a str`/`#[serde(borrow)]` Cow twin over the same escape-free payload; alloc-count axis exact (R·S → 0); **the escaped-payload regime arm is REQUIRED** (borrowed-vs-owned delta collapses there — both arms print); from_reader anti-regime + escape regime in context_notes
- [ ] T5 Author candidate 2 `rawvalue-passthrough-defer-parse`: `Value`-tree field vs `#[serde(borrow)] &'a RawValue` (bench arm via from_str) + boxed-form note for from_reader; G1 compares re-parsed VALUES (RawValue serialization retains original formatting — a behavioral delta, never byte-compare); `raw_value` feature rides the DEV-dependency declaration (serde_json already a main dep, L26 — check feature-unification impact before adding)
- [ ] T6 Adjudicate candidate 3 `deserialize-typed-not-value` — rider on candidate 2's context_notes vs a small third entry; decide on the dedup read, record the call here
- [ ] T7 Standing gates: count pins (rust_perf 149 → new), lib test count floor, `cargo clippy -D warnings`, freeze-table check (gen2/gen3 are clippy-corpus-derived; gen5 filters FIXABLE_RULES — proposer-less rules excluded, expected UNMOVED; verify, never assume)
- [ ] T8 GOAT: G1 value-equality incl. the escaped arm; G2 Pareto + alloc axis (R·S → 0 is the headline; the tree-vs-slice arm second); G3 lib suite green; G4 alloc-free fixed arm (0 allocs, axis 0/0)
- [ ] T9 Retrieval floors: `--score-bench` run + `rule_outcomes` rows (the T8 tail law)
- [ ] T10 Docs: HISTORY.md Batch 173 entry + AGENTS.md rust_perf count bump + Research 204 → RECORD + queue-snapshot line update + this plan COMPLETE
- [ ] T11 Commit+push (named paths only; develop)

## Defers

- [-] Lane-intel artifacts: none owed — no lane mapping (recorded in Research 204; nothing to file in riir-ai).
