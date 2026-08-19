# Research 075 — tantivy utility-layer distill: varint word-composition + 2 riders

**Status:** RECORD — Batch 68 LANDED 2026-08-19 (Plan 013): the lead GOAT candidate **measured ParetoDominated** (0/3 sizes × 3 writer regimes — Vec-push, inline-Cursor, Box<dyn Write>; the folk div-chain mechanism dies: 128 is a compile-time constant, LLVM strength-reduces it, and OoO hides the call chain) — the rule shipped as the corpus's 3rd annotated honest-loser (the `precompute-table` class: exists to STOP the tantivy folk port) + the tz-blsr xor-clear anti-shape rider landed. The distill verdict's B-grade GOAT claim was wrong on this box — the 2-regime evidence shipped in-source (ladder beside loop) does not make the ladder FASTER on modern aarch64.
**Original queued status (superseded):** batch queued as Plan 013 (implementation load-gated; single-threaded benches)
**Date:** 2026-08-19
**Source:** `quickwit-oss/tantivy` @ main — scoped inspection of 5 files: root `Cargo.toml`, `common/src/vint.rs`, `common/src/bitset.rs`, `bitpacker/src/bitpacker.rs`, `bitpacker/src/lib.rs`. All load-bearing quotes coordinator-verified against raw.githubusercontent fetches (the Research 057 rule).
**Routing:** `rust_perf` (io/arithmetic). NOT kernel_opt — these are CPU serialization utilities; "bitpacker" is not a GPU kernel despite the name.
**Whole-repo verdict:** MARGINAL B− — the ~95%-ambient-engine prior HOLDS (the hashbrown precedent); posting codecs, fastfield columnar, segment architecture all correctly out of scope.
**Scoped verdict (utility layer):** YES, B — yield 1–2 rules + 1 rider.

## Bench-signal evidence

`[profile.release] opt-level = 3`, `[profile.bench] opt-level = 3, debug = true`, 11 `[[bench]]` targets (criterion + binggan) + per-crate benches (`bitpacker/benches`, `common/benches`). No `lto`/`codegen-units` tuning (partial signal). Provenance bonus: tantivy *depends on* `fastdivide` + `rustc-hash` — independent production confirmation of our `division-by-invariant-multiplication` (B59) and `fxhashmap` rules.

## GOAT candidate: `varint-compose-word-single-store` (A−)

**Source:** `common/src/vint.rs` — `serialize_vint_u32` vs `VInt::serialize_into`, side by side.

**Mechanism.** Byte-at-a-time varint/LEB128 encoding pays (a) a serial `%128 / 128` dependency chain (each byte's extraction depends on the previous division) and (b) a per-byte `Vec::push` / `write_all` (bounds check + amortized realloc + call overhead). When the total serialized width fits one machine word (u32 → ≤5 bytes ≤ 8), a 5-threshold ladder composes ALL bytes into a `u64` with masked shifts and stores ONCE via `res.to_le_bytes()` into a caller stack buffer. Tantivy ships exactly this split: `serialize_vint_u32` = ladder; `VInt::serialize_into` (u64 → 10 bytes, does NOT fit) = loop. **The two-regime boundary is serialized-width ≤ 8 bytes** — the file itself is the demo, and it ships its own equivalence test (`aux_test_serialize_vint_u32` asserts ladder ≡ loop at 128^i−1/+0/+1 boundaries + `u32::MAX`).

**Quotes (verified 2026-08-19):**

```rust
// serialize_vint_u32 — the ladder, ending in ONE store:
let (res, num_bytes) = if val < START_2 { (val | STOP_BIT, 1) }
    else if val < START_3 { ((val & MASK_1) | ((val & MASK_2) << 1) | (STOP_BIT << (8)), 2) } /* ... 3, 4, 5 */;
*buf = res.to_le_bytes();
&buf[0..num_bytes]

// VInt::serialize_into — the contrasting loop regime (u64, 10 bytes):
let mut remaining = self.0;
for (i, b) in buffer.iter_mut().enumerate() {
    let next_byte: u8 = (remaining % 128u64) as u8;
    remaining /= 128u64;
    if remaining == 0u64 { *b = next_byte | STOP_BIT; return i + 1; } else { *b = next_byte; }
}
```

**GOAT 4-test:** (1) benchable on the Pareto validator — N-value encode arena at 100/1k/10k, loop vs ladder; (2) bit-identical G1 — pure integer composition, and **the exhaustive-equivalence gate is MANDATORY (ported numeric construction, the Batch 59 precedent: full u16 sweep + 128^i boundaries + LCG u32, both conventions)**; (3) magnitude on BOTH axes — timing (serial div chain vs branch ladder) AND exact allocs (N pushes → 0, stack buffer); (4) deterministic. Proposer deferrable (B59 precedent) — recognize the `%128 //128`+push loop signature on ≤u32-width integers.

**Convention nuance (batch must encode):** tantivy sets STOP_BIT on the FINAL byte; protobuf/LEB128 sets the continuation bit on all but the final. The buggy loop body reveals which convention it is — the ladder is convention-faithful either way. Fixtures cover the detected convention; the rule text states the mechanism convention-agnostically.

## Rider (free): tz-blsr anti-shape documentation

**Source:** `common/src/bitset.rs` — `TinySet::pop_lowest` carries an in-source rationale (verified verbatim): `n &= n - 1` clears the lowest set bit WITHOUT depending on `trailing_zeros` (parallel chains: ARM64 `sub→and` beside `rbit→clz`; x86 `blsr` beside `tzcnt`), while the `x ^= 1 << lowest` form serializes them. Our `sparse-set-bit-iteration-tz-blsr` fixture (corpus.rs:1518) already ships the FIXED form but never documents the ANTI-shape — add `x ^= 1 << tz` as an alternate buggy form + the dependency-chain mechanism note (+ godbolt ref) to the rule's spec text. No corpus-count change. (The Batch 66 float-sum rider precedent.)

## Secondary: `unaligned-word-load-from-bytes` (B−)

**Source:** `bitpacker/src/bitpacker.rs` — `BitUnpacker::get`: byte-assembly of 8 loads + bounds checks + 7 shifts + 7 ors → one unaligned `u64::from_le_bytes` + shift + mask, with the file's own `get_slow_path` showing the outlined tail. Positive-instance only (no in-file bug shape) — include only if the fixture falls out cheap; distinct from the GPU `alignment-gated-zero-copy-view` (view-vs-copy, different concept).

## Killed (lane + coordinator)

- `minmax` fused NaN-skip sweep — `loop-fusion` + kernel B44 sweep rule (DEDUP).
- `compute_num_bits` 56-bit clamp — codec invariant, engine-specific.
- BitPacker u64 accumulator, `get_batch_u32s` ramp decomposition, `get_ids_for_value_range` split — serialization architecture. (Bonus: the file's "could make this branchless, but the slow path should rarely kick in" comment is regime CONFIRMATION for `sortedness-check-branchless-accumulate`.)
- TinySet/BitSet structure incl. `/64 %64` — ambient (pow2 strength-reduction).
- `VInt::deserialize` byte-loop — cold IO path.
- `fastdivide`/`rustc-hash` deps — provenance, not candidates.
- `src/fastfield/` + `src/postings/` — UNVERIFIED (3 lane tool timeouts); expected architectural, not counted.

## Dedup evidence (coordinator greps, 2026-08-19)

- `varint|vint|leb128|variable.length` over `riir-clippy/src`: 0 rule hits (one unrelated kernel_opt GPU "variable-length sequences" concept).
- `kernighan|wrapping_sub.*clear|x ^= 1`: only the existing tz-blsr FIXED form (corpus.rs:1518) — confirms rider classification.
- `unaligned|word.load|from_le_bytes` in rust_perf: 0 hits.
- (rust-csv lane, same session: `is_ascii|from_utf8` — 0 rule hits; its A− candidate `ascii-fast-path-before-utf8-validate` stays conversation-only per the MARGINAL B− verdict.)

## Companion verdict: `BurntSushi/rust-csv` = MARGINAL B− (conversation-only)

Best candidate `ascii-fast-path-before-utf8-validate` (A−, `src/byte_record.rs::validate` — whole-buffer `is_ascii()` implies per-field UTF-8 validity) is real but niche; everything else kills (memchr = B67 consumer, record-reuse = API-specific, DFA = architectural). Do not queue.

## Caveats

- Fixtures are ORIGINAL minimal reproductions (MIT hygiene; binary-only distribution rule) — never verbatim tantivy code.
- Snapshot tantivy files at mine time (fast-moving repo); re-verify quotes then.
- Batch is load-gated only for its GOAT builds/benches; single-threaded benches tolerate moderate load WITH load-recording (the B66 note) — unlike Batch 65.
