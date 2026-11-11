# Issue 226 — the index-cache G3 redesign: store after print, size cap, cheaper write
**Status:** open — promotion blocker for `index_cache` (issue 224 T4), verdict-directed. — Delegated-owner verdict (b) 2026-11-11, session 98784e41: keep `index_cache` opt-in; this redesign is the ONLY blocker; G3 restated HERE, before re-measurement, as "time until results printed ≤ +5%".
**Claimed:** katgpt-rs-33@amo (T1+T2), 1794387239, expires 2026-11-13T09:00Z

## Why (the verdict's reasoning)

Promoting with G3 failed (+69% cold vs the +5% bar) would be rewriting a bar after seeing it fail — the habit the GOAT gate exists to stop. But the 233 ms is spent AFTER the work is done: the matcher is already built, the scan already printed — the cost is encode + blake3 + the 18.8 MiB write on the exit path. Three fixes, all small:

- [ ] T1 — **Store after print**: move the cache store (encode + write + prune) OFF the critical path — the results are printed first, the store runs on a thread joined at process exit (a detached write risks being killed mid-entry; the join keeps the atomic-rename contract). Time-until-results then matches OFF; total wall-clock recorded alongside. The print-then-store order must keep the stderr disclosure lines AFTER the run-end roll-up (one `[index-cache] stored …` line, still G4's size report).
- [ ] T2 — **Drop the extra buffer copy**: `encode_domain` builds a temp `body` buffer for the length prefix, then memcpy's it into the outer buffer (≈2× the 18.8 MiB). Patch the length prefix in place instead (reserve 4 bytes, remember the offset, `copy_from_slice` the length after the payload lands). Halves the encode wall cost; a thread-joined store still wants it (exit latency).
- [ ] T3 — **Cache size cap + eviction**: `KEEP_ENTRIES` (8) bounds the ENTRY COUNT, not the BYTES — at 18.8 MiB per corpus state and ~2 new states/day, an unpruned machine accrues ~7 GB/year if entries churn faster than prune... actually prune DOES bound count at 8 (~150 MiB ceiling) — the cap work is (a) pin the ceiling in bytes too (`RIIR_REFINE_INDEX_CACHE_MAX_MB`, default ~256, evict oldest-first beyond it) and (b) verify prune's mtime ordering survives the touch-on-hit (an entry pinned by constant use must not starve a fresh corpus state — LRU vs LFU decision recorded at implementation).
- [ ] T4 — **Re-run G3 on a quiet amo** with the restated bar: time-until-results ≤ +5% vs OFF (total wall-clock disclosed beside). PASS ⇒ promote `index_cache` to the release set + default IN THE SAME COMMIT, `RIIR_REFINE_INDEX_CACHE=0` the demote switch (env-switch law); FAIL ⇒ back to the verdict with the numbers.

## Context

- The feature lane + measurements: `.issues/224_persistent_matcher_index_cache.md` (G1 gate test + binary diff identical; G2 warm 118–120 ms vs OFF 273–296; G4 18.8 MiB disclosed).
- The upstream primitive: riir-neuron-db `Bm25Index::to_snapshot`/`restore_snapshot` (`7e98f4a` + `658b3f5`), vendored at the sanctioned sha.
- The verdict's "probably turns G3 green" premise: T1 alone moves time-until-results to ≈ OFF (the store leaves the critical path entirely); T2 is hygiene for the joined exit; T3 is the fleet-hygiene half (disk is a shared resource — the disk-clean skill's regenerable-safe class).
- Sequencing: T4's promotion commit touches `Cargo.toml` default + `build_stamp.rs` RELEASE_FEATURES + `release.yml` (the `release_set_matches_*` test trio moves in ONE commit).
