# Research 277 — Batch 245: the Batch-244 defer-list conversion

**Status:** RECORD — batch 245 LANDED 2026-10-16 (the 8 staged leads convert 6 + rider + defer, zero dropped; plan 248).

Source: the Batch-244 defer list (Research 276's staged table) — eight leads from the three byte-scan/search sources re-read at their release pins: `rust-lang/regex` @ tag `regex-automata-0.4.18` (commit `e6ad938198e68339a593f85d228f2d44cb54718f`, Apache-2.0/MIT), `BurntSushi/memchr` @ tag `2.8.3` (commit `5fdb40c054e1fff359a2f7bdf7f87a13b34b465d`, Unlicense/MIT), `BurntSushi/bstr` @ tag `1.13.1` (commit `134195be38c7c9a4887980fd0f6f8bae042dd91e`, Apache-2.0/MIT). Every quote re-verified byte-for-byte at the `.raw` pin clones this session (the Batch 31 law); sources read by two disjoint read subagents (regex-automata leads / memchr+bstr leads) + coordinator re-verification.

## The conversions (6 rules, corpus 217 → 223)

| Rule | Category | Bench | Verdict |
|---|---|---|---|
| `self-loop-state-memchr-skip` | dispatch | benchable | **ParetoDominant 3/0/0** — 6.35×/5.71×/5.51× at n=1K/16K/262K (first run 5.56× at max), alloc 0/0, det-stable ×3 |
| `pair-probe-confirm-fused` | simd | benchable | **ParetoDominant 3/0/0** — 8.82×/8.14×/9.91×, alloc 0/0, det-stable ×3 |
| `finder-hoist-reuse` | caching | benchable | **ParetoDominant 3/0/0** — 6.35×/5.71×/5.51×, alloc 0/0, det-stable ×3 |
| `canonical-set-guarded-rebuild` | complexity | NotBenchable | the invariant-guard family (canonicalize + folded + union), premise-corrected |
| `nonmax-sentinel-option-niche` | layout | NotBenchable | the Option-niche law, payoff RELABELED memory-first per source |
| `match-branch-cold-confirm` | dispatch | NotBenchable | suggest-only; the counter-case is NARROW (helpers) vs the benchmark-cited rabinkarp positive |

Plus the count-neutral CHAR rider on `byte-find-position-to-memchr` (the `char-search-encode-once-stack` lead — bstr `find_char` encode-once-into-stack, ext_slice.rs:897-900/921-924, the 1-byte→memchr arity route, the U+FFFD caveat).

Box state (every number): M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, AC power, load ~10/16 disclosed (the sibling Warp-gate bevy build was live through the runs; both arms interleaved so ratios stay fair).

## The premise corrections (the deep read's two findings, both carried into the rules)

1. **The interval-set guard is NOT on push.** The staged lead named "the is_canonical guard before sort" on the insert path; at this pin `IntervalSet::push` (interval.rs:83-119) is binary_search + union-window + splice — sort-free, never calls `is_canonical`. The guard family lives in `canonicalize()` (interval.rs:369-381), `case_fold_simple`'s `folded` flag (142-145, with the honesty clause at 37-53: true ⟹ folded, false ⟹ unknown — "Bottom line: this is a performance optimization"), and `union`'s equality short-circuit (160-166). A rule written to the stale premise would be unfireable against real code of this vintage.
2. **The accel representation is `[u8; 8]`**, not the lead's guessed `[u8; 4]` — `ACCEL_CAP = 8` (accel.rs:88), byte 0 = length (≤3), exit bytes padded for alignment across state-ID widths. Also banked: no `accel` feature exists (always-on, `Config::accelerate(bool)` demote); the reverse-DFA wrapper disables it for limited searches (meta/wrappers.rs:1266-1273); a memchr miss ≠ quit (the EOI fallthrough, search.rs:294-303).

## The counter-case narrowness (match-branch-cold-confirm's verdict change)

The staging reason ("the source records its own counter-case") was TRUE but NARROW: both recorded washes are tiny-helper sites — the offset-arithmetic fn (packedpair.rs:302-306: "This used to be marked `#[cold]` and `#[inline(never)]`, but I couldn't observe a consistent measureable difference") and a generic match handler (memchr.rs:84-91, where the attribute is UNEXPRESSIBLE — generic code can't know the `target_feature` annotation for the unlineable fn). Meanwhile the repo's one `#[cold]` on the rare confirm memcmp carries a benchmark-cited POSITIVE (rabinkarp.rs:350-355: "This causes the inner rabinkarp loop above to be a bit tighter and leads to some performance improvement. See the memmem/krate/prebuilt/sliceslice-words/words benchmark."). The law is conditional: cold the rare heavyweight confirm, never the arithmetic helper, never in generic/target_feature-inlined code, and ALWAYS with a before/after measurement. A fixture would be a coin flip → NotBenchable suggest-only.

## The fixture restructures + the parity catch (the batch's honest-methods record)

Both benchable memchr fixtures were restructured ONCE after a first honest misread, per the Batch-244 disclose-and-restructure discipline:

1. **pair-probe-confirm-fused** — the FIRST shape planted individually-RARE bytes (X/Q strays ~2.7%) under a jointly-rare pair: measured ParetoDominated 0/3/0. With a rare single byte available the single-probe arm already wins; the source's law lives where NO rare byte exists ("bytes that occur rarely" is RELATIVE — when the best available byte is still common, the pair is the tool). Restructured to the common-byte regime (seven symbols ~14% each, needle `nation`, offsets (1,3) both common, pair ~2%): ParetoDominant 8-10×.
   **The parity instrument caught a REAL fixture defect the timing-only validator cannot see:** the SWAR equality flag sits at bit **8j+7** (bit 7 of byte j); the first extraction treated `trailing_zeros` as the byte index → candidates anchored at `base + 8j + 7` → the fixed arm verified at garbage positions and SKIPPED nearly all verifies → the pre-fix 8.9× read was an ARTIFACT of missing verifies. A standalone brute-force-oracle harness (both arms vs oracle at 3 sizes) exposed the divergence; the fix (`tz/8`) restores parity AND the honest read stands at 8.1-9.9× WITH parity proven. Standing lesson: **a timing verdict on a fixture whose arms don't provably compute the same function is not a measurement** — the corpus's bench validator checks G1 real-measurements, not cross-arm equality; parity needs its own instrument.
2. **finder-hoist-reuse** — the FIRST shape (~30B log lines) measured ParetoDominated 0/3/0: below memchr's own 64B dispatch threshold (`memmem::find`: `haystack.len() < 64` → the direct rabinkarp path, no construction) the per-call shape never builds the meta searcher, so the "hoisted" arm forced the HEAVIER path. Restructured to ~200B lines (the meta-Finder route the rule claims): ParetoDominant 3-6×. The fixture must sit in the regime where the per-call cost the rule names actually exists.

## The defer confirmed

`align-probe-overlap-tail` stays deferred, reasons deepened: the align half — the source deliberately aligns (memchr.rs:54-60) but never states a modern-µarch justification anywhere in `memchr/src` (grep-verified silent), and on M3 Max (NEON, cheap unaligned loads) the prologue likely washes (labeled inference); the tail half — real and source-justified (memchr.rs:66-77 "This is OK because it converts a loop into a small number of very fast vector instructions. The overlap is OK because we know the place where the overlap occurs does not contain a match." + the offset-shift code at 219-228 `cur = cur.sub(V::BYTES - end.distance(cur))` — the citable atoms) but the magnitude is ≤V−1 scalar iterations saved per scan call, offset-dependent — the timing-only-at-scale drop class. Revisit trigger: a future hand-SIMD rule authoring session absorbs it as a rider.

## Dropped: none

All eight staged leads adjudicated — six converted, one rider, one defer confirmed. The conversion rate against the ~50% falsification precedent: the staging reasons were this time conservative (two CONVERT verdicts overturned the staging's lean).

## Gates (the full ladder, all green)

- G1: batch245 parse gates + needle gates + the char-rider gate (253 corpus tests at the bench posture)
- G2/G5: 3/3 ParetoDominant, alloc 0/0, alloc-det stable ×3
- G3: lib 2679/0 at default features; clippy `-D warnings` at default / no-default / all-features
- Retrieval floors HELD 21/0/1i (no re-pin — blended pool shape absorbed the +6)
- gen2+gen3 freeze e2e both ok (the rust_perf delta moved no deterministic window — tables verified, not regenerated)
- score-bench appended run #185
- Count pins re-pinned: total 223 / with_spec 114 / without_spec 109 / annotated 169 / unannotated 54

## The stale-pin repair (found + fixed in passing)

`with_spec` read 101 through Batch 244's landing — the +10 byte-scan rules (ALL bench_spec Some) were never re-pinned because only the rust_perf_bench posture compiles that test and B244's gates ran the default-feature lib (the B239/B207 pin-hiding class repeating). `without_spec` was re-pinned to 96 at B244 with arithmetic on the WRONG axis ("-10" treating the new Some entries as None→Some moves); measured truth at post-B244 was 106. Both repaired to measured truth (114/109 post-batch) with the attribution in the pin comments.

## Staging residue (for the queue)

- simdutf + unicode-normalization: STAGED UNREAD (Batch 246 candidates — never folded by the accumulation clause).
- The memchr 2.7→2.8 pin-bump watch leg + memchr 3.x watch (byteshift-unpack, Twice searchers — absent at 2.8.3): recorded in the queue snapshot.
- `.raw/{regex,memchr,bstr}` + the root-level `memchr/` `bstr/` clones (the read subagent's stray — root, not .raw/) removed at close.
