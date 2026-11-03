# Issue 198 — traj-store score-run key: the toolchain axis (the 197-followup)

**Status:** Open — filed from plan 329 / issue 197 P4-b (2026-11-03); behavior change with its own gates, deliberately NOT in the 197 T0b set

## The finding

`src/traj_store.rs` carries ZERO toolchain/rustc/clippy pinning (verified twice: issue 197 substrate-first at filing, re-verified at T0b claim 2026-11-03 — grep `toolchain|rustc_version|clippy_version` returns zero hits). The doctrine (sccache cache-key completeness, `.research/362`): **an input that can change the outcome belongs in the key.** A score-run row keyed `scorerun:{recorded_unix}:{sha-or-nosh}` pools measurements taken under different rustc/clippy toolchains — the same completeness class issue 197 P4-a fixed on the `score_history.jsonl` axis (the `rustc_version`/`clippy_driver_version` provenance fields + the report's pooling split).

## What P4-a (197 T0b) already landed, and what this issue adds

P4-a puts the provenance ON the jsonl row and splits the report comparisons on (toolchain ∨ generation). The store's `ScoreRunRow` mirror deliberately stays key-unpinned — changing the key shape is a behavior change (existing keys keep resolving; new semantics partition) — so it is sequenced HERE, not smuggled into T0b.

## The migration rule (the issue-197 round-1 shape, carried verbatim)

- Rows (existing and future) WITHOUT a toolchain stamp read as `unknown-toolchain`.
- `unknown-toolchain` rows are NEVER pooled with pinned rows — not in any store-side comparison, series, or roll-up; the pool-split report G1 from P4-a is the template.
- New rows carry the same `rustc_version` (+ `clippy_driver_version` when the oracle lane ran) capture `score_bench::report::HistoryRecord` stamps; the jsonl ↔ store schema-identity test extends by the additive law.
- Key options to price at implementation time: (a) stamp fields only, key unchanged (rows carry the axis, comparisons split — the cheapest honest shape); (b) toolchain in the KEY itself (`scorerun:{recorded_unix}:{sha}:{toolchain-digest}`) — invalidates nothing (new rows just carry longer keys) but re-reads every consumer of the key format. (a) is the default candidate; (b) only if a store-side query needs toolchain-selective fetch.

## Gates (at implementation)

- Schema-identity test extension (additive fields, mirror law).
- A pooling G1: two `ScoreRunRow`s differing on toolchain never share a comparison bucket (mirror of P4-a's report G1).
- `cargo test --lib` + the traj-store gates battery.

## Precedents

- Issue 197 P4-a/P4-b split (the round-1 verdict): audit-answered ≠ behavior-landed.
- `.research/362` (sccache): cache-key completeness doctrine.
- P104's UNVERIFIED-KEPT audit: a measurement whose inputs can silently differ is not a measurement.
