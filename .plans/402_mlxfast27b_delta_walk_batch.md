# Plan 402 — Batch 402: the mlxfast-bonsai2-27b 113-commit delta walk (KV / recurrent-state / Hadamard / MTP surface)

**Status:** IN_FLIGHT — claim 2026-11-22T23:36Z, expires 2026-11-24T23:36Z

Queue head: Batch 401's slice-(1) staging — `Layr-Labs/mlxfast-bonsai2-27b-engine` delta
`cfb35b0d`→`0c7c3923` = 113 commits, ALL arena Accept/Validate submissions (window 09-25→10-02,
the arena close phase), +31,368 insertions across 32 Swift files (ContiguousKVBackend +373,
Hadamard 883±, RecurrentStateV2 108±, MTPContractsV2 +33, FullSequenceKV 218±) + the README's
speculative-decoders-as-separate-exports section — the KV/recurrent-state/Hadamard machinery the
B100/B115/B181 visits never mined. Template: Research 198 per-submission diff-history walk.
Prior pins: B181 = `cfb35b0d` (Research 212). License MIT (B181 verified; re-check at clone).

- [ ] T0 — re-clone full-history blobless at tip (`.raw/mlxfast-27b`); verify delta `cfb35b0d..HEAD`
      (re-derive the count at tip; note any delta past `0c7c3923`), LICENSE re-check, per-file diff-stat
- [ ] T1 — dedup greps at 3+ vocabularies over kernel_opt + rust_perf corpora
      (contiguous-KV / Hadamard-rotation / recurrent-state / MTP-contract / full-sequence-KV),
      every hit read and ruled in-row; prior mined set named per candidate
- [ ] T2 — two parallel subagent readers over the per-submission diffs (Research 198 template);
      coordinator re-verifies every quoted line at the clone
- [ ] T3 — author corpus entries (kernel_opt primary; rust_perf only if a measurement rule emerges)
      + riders; per-entry adjudications vs the named kin, in-rule both directions
- [ ] T4 — fixtures standalone-compiled AND executed pre-entry (debug AND -O); the gate's caught
      defects fixed before entry
- [ ] T5 — gates: lib tests, clippy −D both postures, retrieval floors BOTH lanes, fmt gate,
      pins_manifest --write, count web one-landing; score-bench only if clippy_lints moves
- [ ] T6 — Research 437 batch record; snapshot claim → LANDED reshape; commit + push;
      teardown `.raw/mlxfast-27b`
