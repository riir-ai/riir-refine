# Plan 013 — Batch 68: tantivy varint word-composition + riders

**Source:** Research 075 (distill verdict YES-B scoped to `common/src/vint.rs` + `bitpacker/src/bitpacker.rs` + the bitset.rs rider).
**Scope:** 1 lead rule + 1 free rider + 1 optional B− secondary. Corpus 72 → 73 or 74 (rust_perf).
**Load note:** GOAT benches are single-threaded wall-clock + exact-alloc — runnable under MODERATE load with load-recording (B66 precedent), i.e. this batch may run BEFORE Batch 65 (whose contention bench needs a quiet box). Under heavy saturation (load >20, sibling measurement running) defer all builds.

## Tasks

- [x] T1 — Dedup re-verification at batch start: corpus greps under both vocabularies (`varint|vint|leb128`, `compose|single.store`); re-scan for sibling corpus growth since 2026-08-19 (the dual-allocation rule — re-scan the corpus folder at WRITE time).
- [x] T2 — Snapshot tantivy source at mine time (raw fetch of `common/src/vint.rs` + `common/src/bitset.rs`; bitpacker read by the lane) + re-verify the Research 075 quotes still hold (fast-moving repo). Quotes coordinator-verified against raw.githubusercontent 2026-08-19.
- [x] T3 — Author `varint-compose-word-single-store` (category: io). ORIGINAL fixture. Regime CORRECTED mid-batch after the first bench: Vec-push shape → writer shape (per-byte `write_all`) — the honest fix at root cause, not a suppression. Convention note encoded (stop-bit-on-last vs continuation-bit; the loop body reveals which).
- [x] T4 — **Exhaustive equivalence gate (MANDATORY — ported numeric construction, Batch 59 precedent):** full u16 sweep + 128^i−1/+0/+1 boundaries + LCG u32 — ladder ≡ reference loop BIT-IDENTICAL, BOTH conventions (`varint_ladder_matches_reference_loop_exhaustive`).
- [x] T5 — Rider: `sparse-set-bit-iteration-tz-blsr` spec enrichment — the `x ^= 1 << tz` ANTI-SHAPE + the dependency-chain mechanism note (ARM64/x86 chains, godbolt ref) in the rule's comment block (comment-only — zero test/count impact, the float-sum rider precedent).
- [-] T6 — Optional (B−): `unaligned-word-load-from-bytes` — DEFERRED: the varint lead's honest-loser verdict showed the M3 aarch64 box hides exactly this call-chain class of win (OoO execution); benching the unaligned-load rule under the same box would likely reproduce the same dominated verdict — re-evaluate on a quiet box or x86 before authoring (the same re-gate condition as the lead's annotation).
- [x] T7 — Standing gates: no rust_perf syn-parse standing gate exists (verified — only the pruner's validate tests); the exhaustive gate + canary serve as G1. Count pins updated: entries.len() 72→73, header counts (io 2, benchable 63/73, batch-history line), annotated 15→16 / unannotated 57, `sixty_two_rules_have_bench_specs`→`sixty_three`, category-count comment.
- [x] T8 — GOAT: G1 exhaustive+canary PASS; **G2 measured honestly = ParetoDominated 0/3 × 3 regimes** (Vec-push / inline-Cursor / Box<dyn Write>, load-recorded ~25-33, alloc axis equal+stable, outputs byte-identical) → the rule ships with the honest-loser `context_notes` annotation (the `precompute-table` class) — NOT promoted, per "never promote a perf gain on a wrong result"; G3 clippy 0 both states + 620/638 lib tests + retrieval floors HELD (13 retrieval tests green with the rule in the corpus); G-det alloc determinism stable ×3.
- [x] T9 — Docs: Research 075 status → RECORD (honest outcome); `.docs/11_domains/rust_perf.md` — counts fixed (73/63/16) + the 10 missing B61-B68 table rows added (pre-existing doc debt) + the varint row; AGENTS.md corpus count bump (72→73); queue snapshot update (same commit).
- [x] T10 — Commit `feat: batch 68: varint-compose-word-single-store (annotated honest-loser) + tz-blsr anti-shape rider (tantivy distill, Research 075)` + push; update `.distill/001_mining_queue_snapshot.md` in the SAME commit.

## Defers

- [-] T6 may defer (B− secondary, include-if-cheap) — record decision either way.
- [-] Proposer for the varint rule deferrable (B59 precedent — suggestion-only first; proposer = recognize the `%128 //128`+push signature on ≤u32 integers, emit the convention-faithful ladder).

## Numbering

Batch 68 follows Batch 67 (regex-automata). If Batch 65 (align-64 contention bench) lands first under a quiet window, this becomes Batch 69 — the queue snapshot's ordering note governs (single-threaded-bench batches may leapfrog contention batches under load).
