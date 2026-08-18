# Plan 012 — Batch 67: regex-automata memchr byte-find (rust_perf)

> Executes Research 074 (`.research/074_batch67_regex_automata_memchr_pool.md`) — the
> regex-automata distill verdict. Plan-first per the standing discipline.

## Scope

Land the coordinator-sharpened lead `byte-find-position-to-memchr`. The secondary
(`ascii-class-chain-to-const-lut`) is expected-skip (precompute-table adjacency, heavy
proposer — the pool's own note says only-if-cheap). The heal-our-own-proposers rider is
evaluated + likely deferred (adds a direct memchr dep for non-hot healer-internal scans).

- [x] T1 — Dedup re-verification at write time: fixture BODIES grep for position/rposition/
      memchr/byte-find shapes (the Research 073 standing lesson).
- [x] T2 — Corpus entry `byte-find-position-to-memchr` (category: iterators — the
      position/rposition trigger family, beside filter-count-to-any). ORIGINAL minimal
      fixture. Trigger = position/rposition/manual-scan on &[u8] with byte-literal needle;
      EXCLUDES contains (std SliceContains already SWAR-specializes it — the exclusion +
      its std evidence go in the entry comment). Dep-gate note (fxhashmap precedent:
      fire only when the target manifest declares memchr; else suggestion-only).
- [x] T3 — Bench spec: `// [dep: memchr]` harness; haystack sizes {32, 256, 4_096, 65_536}
      (the small-wash → SIMD-win curve; Teddy bound ~16-32B); needle near the end (~90%
      position — both arms do ~full scan; memchr's SIMD block scan vs scalar early-exit).
      Honest expectation: wash/tie at 32B, growing win at 4K/64K.
- [x] T4 — Rider evaluation: the 3 proposer-helper positive instances
      (collect_append_to_extend.rs:173, hashmap_with_capacity.rs:232,
      vec_macro_zero_fill.rs:183) — defer unless memchr is already a direct dep (it is
      not expected to be; adding a dep for non-hot healer-internal scans is not justified).
- [x] T5 — Count-pin updates (the 21 known sites; deltas: entries 71→72, benchable 61→62,
      unannotated 56→57 IF unannotated, annotated 15 unchanged).
- [x] T6 — GOAT: G1 fixture + harness compile (dep marker path); G2 Pareto bench
      (single-threaded, load-recorded); G3 lib tests default + bench + latent states;
      G4 alloc axis; retrieval floors.
- [x] T7 — Clippy 0 in touched files.
- [x] T8 — Docs: AGENTS.md Batch 67 entry; queue snapshot pool → DONE + Progress 71→72;
      Research 074 Status → RECORD.
- [x] T9 — Commit + push (`feat:` prefix).

## Non-goals

- Batch 65 (hotpath align-64 — needs the quietest box; still blocked by p335).
- Proposers for the new rule (detection-subtle; deferred per the standing backlog).
- The ascii-class-chain secondary (expected-skip with reason).
