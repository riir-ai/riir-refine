# Plan 013 — Batch 68: tantivy varint word-composition + riders

**Source:** Research 075 (distill verdict YES-B scoped to `common/src/vint.rs` + `bitpacker/src/bitpacker.rs` + the bitset.rs rider).
**Scope:** 1 lead rule + 1 free rider + 1 optional B− secondary. Corpus 72 → 73 or 74 (rust_perf).
**Load note:** GOAT benches are single-threaded wall-clock + exact-alloc — runnable under MODERATE load with load-recording (B66 precedent), i.e. this batch may run BEFORE Batch 65 (whose contention bench needs a quiet box). Under heavy saturation (load >20, sibling measurement running) defer all builds.

## Tasks

- [ ] T1 — Dedup re-verification at batch start: corpus greps under both vocabularies (`varint|vint|leb128`, `compose|single.store`); re-scan for sibling corpus growth since 2026-08-19 (the dual-allocation rule — re-scan the corpus folder at WRITE time).
- [ ] T2 — Snapshot tantivy source at mine time (raw fetch of `common/src/vint.rs` + `common/src/bitset.rs` + `bitpacker/src/bitpacker.rs` to `.distill/tmp` or equivalent) + re-verify the Research 075 quotes still hold (fast-moving repo).
- [ ] T3 — Author `varint-compose-word-single-store` (category: io or arithmetic — decide at authoring; count-pin sites updated). ORIGINAL fixture (both conventions: stop-bit-on-last [tantivy] and continuation-bit [protobuf] — buggy loop form detectable, ladder emitted convention-faithfully). Regime: serialized width ≤ 8 bytes (≤u32); u64/u128 keep the loop — encode the two-regime boundary in the rule text using tantivy's own side-by-side as evidence.
- [ ] T4 — **Exhaustive equivalence gate (MANDATORY — ported numeric construction, Batch 59 precedent):** full u16 sweep + 128^i−1/+0/+1 boundaries + LCG u32 sample (or full u32 if release runtime acceptable) — ladder ≡ reference loop BIT-IDENTICAL, both conventions. Port the design of tantivy's own `aux_test_serialize_vint_u32` (boundary set) but write original code.
- [ ] T5 — Rider: `sparse-set-bit-iteration-tz-blsr` spec enrichment — add the `x ^= 1 << tz` ALTERNATE BUGGY form + the dependency-chain mechanism note (ARM64 `rbit→clz→lsl→eor` vs `sub→and`; x86 `tzcnt→btc` vs `blsr`; godbolt.org/z/fnfrP1T5f ref from tantivy `TinySet::pop_lowest`). No corpus-count change; update the rule's concept/notes fields + any fixture commentary.
- [ ] T6 — Optional (B−): `unaligned-word-load-from-bytes` — include ONLY if the fixture falls out cheap; otherwise record the defer with reason.
- [ ] T7 — Standing gates: extend the syn-parse gate (`batch68_pseudocode_parses`) + ALL count-pin sites (the snapshot's list — corpus.rs entries.len() assert, KERNEL_CATEGORIES-equivalents for rust_perf, per-category count comments, router totals, file-level `//!` header counts, the `benchable`/`annotated` count pins if applicable).
- [ ] T8 — GOAT: G1 syn/parse on new entries; G2 Pareto bench (N-value encode arena 100/1k/10k, loop-vs-ladder, release, load-recorded) + **exact alloc-count axis (N pushes → 0 expected)**; G3 no-regression — clippy 0 both feature states + retrieval floors HELD (`cargo test --features latent_retrieval,rust_perf_bench --lib retrieval` — floors: kernel self-variants 100%, blended per the Issue 022/Bench 024 re-pin).
- [ ] T9 — Docs: Research 075 status line → RECORD (batch landed); `.docs/11_domains/rust_perf.md` rule entry; AGENTS.md corpus count bump (72 → N); queue snapshot update (same commit).
- [ ] T10 — Commit `feat: batch 68: varint-compose-word-single-store + tz-blsr rider (tantivy distill, Research 075)` + push; update `.distill/001_mining_queue_snapshot.md` in the SAME commit.

## Defers

- [-] T6 may defer (B− secondary, include-if-cheap) — record decision either way.
- [-] Proposer for the varint rule deferrable (B59 precedent — suggestion-only first; proposer = recognize the `%128 //128`+push signature on ≤u32 integers, emit the convention-faithful ladder).

## Numbering

Batch 68 follows Batch 67 (regex-automata). If Batch 65 (align-64 contention bench) lands first under a quiet window, this becomes Batch 69 — the queue snapshot's ordering note governs (single-threaded-bench batches may leapfrog contention batches under load).
