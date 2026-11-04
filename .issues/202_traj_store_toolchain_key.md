# Issue 202 — traj-store score-run key: the toolchain axis (the 197-followup)

**Status:** CLOSED LANDED 2026-11-04 (zed-m3-issues-fixall) — option (a) implemented: the stamp-fields-only shape (the KEY unchanged, `scorerun:{recorded_unix}:{sha-or-"nosh"}`). `ScoreRunRow` carries `rustc_version` + `clippy_driver_version` (serde-default `None` = unknown-toolchain for every pre-202 row), `From<&HistoryRecord>` mirrors the pair verbatim (new store rows stamp automatically through the dual-write tail — the jsonl side has stamped since P4-a), and `score_runs_share_toolchain` is the store-side bucket rule (P4-a's report G1 mirrored: exact-pair match only; `None` pools only with `None`; a clippy-only difference is its own bucket). Gates: the schema-identity test extended (the pair now RIDES — pinned both directions incl. the legacy no-key decode), the NEW pooling G1 test (truth table + the store round-trip + the detect_saturation anchor rule store-side, both anchor directions); `cargo test --lib` 2817/0 default · traj battery 49/0 · retrieval gate 27/0 · schema+G1 3/3 at score_bench · 47/0 at all-features; clippy -D clean at score_bench / latent_retrieval+score_bench / all-features. **Owner reassignment 2026-11-04 (the owner's "do 10" word): the foreign claim below was reassigned to the m3 session per the issue-197-takeover precedent — executed inside its window, before the 2026-11-05T15:00 expiry.** Original status: Open — filed from plan 329 / issue 197 P4-b (2026-11-03); behavior change with its own gates, deliberately NOT in the 197 T0b set. **Renumbered 198 → 202 on 2026-11-04 (COLLISION resolution): this file and `198_mozilla_outbound_pr_wetrun.md` both held 198 — the wetrun allocated first (`531b83af` 2026-11-02 16:39) vs this file's `7702625e` (2026-11-03 07:41), so the first-allocation rule decides the wetrun keeps 198 and this file is the loser; `citation_weight.py` independently agrees on direction (weak lead for the wetrun, attributed +3 vs 0) with the 5 UNRESOLVED sites disclosed. The live claim below is unaffected (the session name's `-198` suffix is historical).**
**Lease closed:** zed-riir-refine-198, 1793565213, was expires 2026-11-05T15:00:00+00:00 — REASSIGNED + DISCHARGED 2026-11-04 inside the window (owner "do 10", the issue-197-takeover precedent): zed-m3-issues-fixall landed the implementation the same day (option (a), see the Status line); the lease is closed by the landing, not by expiry.

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
