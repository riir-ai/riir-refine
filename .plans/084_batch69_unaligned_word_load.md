# Plan 084 — Batch 69: unaligned-word-load-from-bytes (the Plan 013 T6 refutation)

> **Renumbered 015 -> 084 on 2026-09-05 (Issue 069 citation-weight arbitration).**
> `015` was allocated twice: this plan and `015_kat011_phase0_payreq_substrate.md`
> (the KAT lane — 10 inbound citations vs 4 here); the KAT plan keeps 015.

**Source:** Research 075 §Secondary (the B− `unaligned-word-load-from-bytes` candidate, DEFERRED at
Batch 68 with the reason "the varint lead's honest-loser verdict showed the M3 aarch64 box hides exactly
this call-chain class of win (OoO execution)" — **that defer reason is now MEASURED-REFUTED**: the two
mechanisms are different classes. The varint loser was CALL-COUNT (N dyn-dispatch writer calls hidden by
OoO); this candidate is LOAD-COUNT (8 separately-loaded bytes vs 1 fused unaligned load) — OoO cannot
hide 8 L1 loads behind 1, and LLVM does NOT fuse the shift/or assembly chain into a byte copy.)

**Pre-implementation probe (both arches, 2026-08-19):** standalone crate
(`target/probe_unaligned/` on M3; `E:/git/probe_unaligned/` on the 4090 — the scp-in-dir trap caught:
`scp -r src host:.../src` nests; copy the FILE). G1 bit-identical at EVERY byte alignment (4088/4092
offsets, both arches). G2 median-of-5, interleaved arms:

| shape | arch | 100K | 1M | 10M | load |
|---|---|---|---|---|---|
| u64 (8-byte) | aarch64 (M3) | 2.17x | 2.18x | 2.23x | ~118 (shared, interleaved) |
| u64 | x86 (4090) | 1.47x | 1.47x | 1.39x | 7-31 |
| u32 (4-byte) | aarch64 | 1.38x | 1.38x | 1.27x | ~118 |
| u32 | x86 | 1.20x | 1.29x | 1.21x | 7 |

**ParetoDominant on BOTH arches, BOTH widths. No annotation needed — this is a WINNER rule.**

**Scope:** 1 rule. Corpus 73 → 74 (io 2 → 3; benchable 63 → 64; annotated stays 16).

## Tasks

- [x] T1 — Dedup re-verification at author time: `from_le_bytes|unaligned|byte.assembl` over the corpus
      (0 hits at plan time — re-verify at WRITE time per the standing rule); also verify the B66 secondary
      `u128-funnel-unaligned-word-read` never landed (queue-snapshot-only).
- [x] T2 — Author `unaligned-word-load-from-bytes` (category: io — the byte-buffer reading family beside
      varint; also tantivy-sourced). ORIGINAL fixture (MIT hygiene): buggy = 8-term (and 4-term u32)
      shift/or byte assembly; fixed = `(&data[a..a+8]).try_into().unwrap()` + `from_le_bytes` (+ shift+mask
      when bit-packed — the tantivy `get` shape; keep the fixture to the plain word-read, the bit-shift
      variant noted in the comment). Scope note: u64 the measured headline, u32 the common real-world shape
      — BOTH measured ParetoDominant. u16 excluded (2 bytes — unmeasured, diminishing).
- [x] T3 — BenchSpec from the probe (u64 headline; sizes 100K/1M/10M lookups over a 4 KiB buffer;
      median-of-5 is the validator's own loop — match its harness conventions). G1 exhaustive-at-every-
      alignment as a corpus TEST (`unaligned_word_load_matches_assembly_exhaustive` — every offset in a
      4 KiB buffer, u64 + u32).
- [x] T4 — Count pins: entries.len() 73→74, io category count 2→3 (header comment + category-count
      comment + any per-category test table), benchable 63→64 (`sixty_three_rules_have_bench_specs` →
      `sixty_four_...`), batch-history header line, annotated 16 unchanged, router/docs count mentions.
- [x] T5 — GOAT: G1 exhaustive PASS (every alignment, u64+u32); **G2 validator ParetoDominant 3.76x/3.71x/3.67x @ 100K/1M/10M (M3, load ~68 recorded), 0 allocs both arms, G-det stable**; G3 649/649 default lib + corpus tests 16/16 + retrieval floors HELD (12 retrieval tests green) + clippy 0 in my files (the 10 warnings are the sibling's in-flight clippy_lints proposers, verified not mine); the x86 numbers recorded in the rule comment + spec doc.
- [x] T6 — Docs: Research 075 §Secondary status line (defer→refuted→landed, numbers); queue snapshot
      (Batch 69 entry + Progress line corpus count); AGENTS.md (Batch 69 paragraph + counts);
      `.docs/11_domains/rust_perf.md` (row + counts 73/63/16 → 74/64/16).
- [x] T7 — Commit `feat: batch 69 — unaligned-word-load-from-bytes (cross-arch ParetoDominant …)` + push;
      update `.distill/001_mining_queue_snapshot.md` in the SAME commit; sync the 4090; clean up
      `E:/git/probe_unaligned` + keep `target/probe_unaligned` locally (gitignored, the probe record).

## Defers

- [-] Proposer (recognize the ≥3-term shift/or byte-assembly chain on indexed byte loads → emit the
      try_into+from_le_bytes form) — deferrable per the B59/B68 suggestion-first precedent. Record either way.
- [-] u128 window (the B66 `u128-funnel` shape — 2 overlapping u64 loads + funnel) — out of scope,
      unmeasured here; the queue snapshot's B66 secondary note remains the record.
