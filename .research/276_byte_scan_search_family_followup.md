# Research 276 — the text/unicode family follow-up: the byte-scan/search mechanism batch (regex byte-class law + memchr + bstr)

**Status:** DISTILLED — batch 244 LANDED 2026-10-16 (+10 rust_perf rules, all benchable, all ParetoDominant; see the landing commit). Chartered by Research 275's follow-up pool (items 2 + 5; the accumulation clause folded what was READ before the batch window — simdutf + the unicode-normalization tables stay STAGED, unread, for a later batch).

## Sources (release pins, all verified at checkout-before-read)

| Source | Pin | License | Yield |
|---|---|---|---|
| `rust-lang/regex` | tag `regex-automata-0.4.18` @ `e6ad938198e68339a593f85d228f2d44cb54718f` | Apache-2.0 OR MIT | 3 rules + the merged-rank law's half |
| `BurntSushi/memchr` | tag `2.8.3` @ `5fdb40c054e1fff359a2f7bdf7f87a13b34b465d` | Unlicense OR MIT | 4 rules + the merged-rank law's half |
| `BurntSushi/bstr` | tag `1.13.1` @ `134195be38c7c9a4887980fd0f6f8bae042dd91e` | Apache-2.0 OR MIT | 3 rules |

All three checked out AT the release tags before any read (the B193 freshness law, the stable-crate form); every quote in the landed rules re-verified byte-for-byte at the `.raw` pin clones this session (the Batch 31 law). Sources read by three disjoint read subagents (regex-automata class machinery / memchr internals / bstr internals) + coordinator dedup re-run at the 207-rule corpus before authoring.

## What landed (10 rules, corpus 207 → 217)

| Rule | Category | Source anchor | GOAT (M3 Max, release, AC, load ~4.6/16) |
|---|---|---|---|
| `byte-class-equivalence-table-shrink` | layout | regex-automata `util/alphabet.rs` | 1.39×/1.08×/tie (1024-state fixture) |
| `prefilter-choice-shape-ladder` | dispatch | regex-automata `util/prefilter/mod.rs` | 46.62×/44.50×/44.00× |
| `alternation-prefix-collapse-at-build` | complexity | regex-syntax `hir/literal.rs` | 639.67×/355.14×/334.80× |
| `byte-rank-select-rarest-probe` | caching | regex-syntax `rank()` + memchr `packedpair` (MERGED) | 38.09×/24.02×/22.93× |
| `prefilter-skip-density-demotion` | dispatch | memchr `memmem/searcher.rs` PrefilterState | 5.40×/3.89×/3.76× |
| `iter-count-bulk-specialization` | iterators | memchr `sse2/memchr.rs` count override | 9.21×/9.28×/9.19× |
| `mod64-byte-membership-skip` | caching | memchr `arch/all/twoway.rs` ApproximateByteSet | 5.27×/5.71×/5.84× |
| `byteset-cardinality-dispatch` | dispatch | bstr `src/byteset/mod.rs` | 1.34×/1.58×/1.49× |
| `utf8-fast-lane-cold-diagnostics` | dispatch | bstr `src/utf8.rs` fast/cold lane split | 10.33×/8.60×/8.88× |
| `bufread-scan-in-buffer-scratch-fragment` | io | bstr `src/io.rs` fill_buf drain | 3.63×/3.00×/2.77×, alloc 10892→0 |

The regex byte-class law (alphabet equivalence classes) is the family's canonical upstream of Batch 243's ASCII-run-skip mechanism class. `prefilter-skip-density-demotion` is the corpus's FIRST feedback-control rule (measure-and-demote at runtime) — a new class, not a scan shape.

## Honest fixture restructures (disclosed in-rule, the two-shape precedent)

- `byte-class-equivalence-table-shrink`: FIRST fixture (24 states, 6 KiB table, both arms L1-resident) measured **Mixed 1/0/2** — the table must exceed L1 for the law to breathe; restructured to 1024 states (256 KiB vs 8 KiB) per the rule's own stated regime → ParetoDominant. The regime IS the fixture.
- `prefilter-skip-density-demotion`: FIRST fixture (scalar probe + identical verify — no accelerator asymmetry) measured **ParetoDominated 0/3/0**; restructured so the probe is the actual accelerator (memchr2 head finder) and the demoted base is the plain scalar scan → ParetoDominant. The probe must BE the accelerator for the demotion to pay.
- `byteset-cardinality-dispatch`: FIRST fixture (10% delimiter density — per-hit restart overhead dominating) measured **ParetoDominated 1/2/0**; restructured to sparse delimiters (~1.5%, long SIMD spans) per the rule's stated regime ("haystack non-trivial") → ParetoDominant.

No rule was landed on a dominated measurement; all three restructures are within the rules' own stated regimes and are disclosed in the rules' context_notes.

## The merge adjudication (one law in two repos)

`rare-byte-rank-prefilter-gate` (regex-syntax) and `pair-rank-select-by-frequency` (memchr packedpair) are the SAME background-frequency law — same mechanism, same `250` threshold (`is_poisonous` vs `MAX_FALLBACK_RANK`), independently maintained. MERGED as `byte-rank-select-rarest-probe` with both citations. The bstr twin of memmem-Finder hoisting (its `ext_slice.rs` Finder doc says the same thing as memchr's `memmem/mod.rs`) was folded into the same adjudication and DEFERRED with the memchr half.

## Deferred to Batch 245 (staged leads, with reasons)

- `self-loop-state-memchr-skip` (regex-automata `dfa/accel.rs` — replace N state-machine steps with one primitive skip): high value, bench needs long homogeneous runs + the disclosed negative arm (common exit bytes).
- `pair-probe-confirm-fused` (memchr generic packedpair — two-offset AND mask): dedup vs `dual-mask-fused-block-scan` needs its own pass.
- `align-probe-overlap-tail` (the aligned-loop skeleton): the alignment half washes on modern unaligned-load µarchs (incl. this box); the tail conversion is real but offset-dependent.
- `match-branch-cold-confirm` (`#[cold] #[inline(never)]` the rare confirm arm): attribute-only; the source records its own counter-case (packedpair measured no consistent difference).
- `char-search-encode-once-stack` (bstr `find_char` — encode the needle once, scan bytes): thin sibling of `byte-find-position-to-memchr`.
- `canonical-set-guarded-rebuild` (regex-syntax `interval.rs` — the is_canonical guard before sort): adjacent to `sortedness-check-branchless-accumulate`.
- `nonmax-sentinel-option-niche` (regex-automata `NonMaxUsize`): timing-only-at-scale; composes with the u32-width rule.
- The memmem Finder-hoist pair (memchr + bstr): adjacent to `regex-hoist-lazylock` — likely a rider or a merge, needs that rule's body read first.
- **Dropped**: `sentinel-run-word-broadcast-skip` (bstr `byteset/scalar.rs` — same genus as `ascii-run-swar-block-skip`: run-skip via word-wide ops), `cross-product-budget-degrade` (the Batch-243 fixture-artificiality class), the byteshift/unpack + Twice-searcher suspects (NOT PRESENT at memchr 2.8.3 — memchr 3.x material, verified absent at the pin), the reverse-substring accelerator (upstream's own negative: "not clear it's worth the extra code").

## Reading intel for the next pass

- memchr 2.8.x: the `Twice`/`Three` unrolled searchers and the byteshift-hash portable fallback are memchr 3.x — a pin-bump watch leg is worth arming (the crate moved 2.7 → 2.8 recently; the delta may carry the iter-unroll family).
- The union+fn-pointer dispatch pinning in memmem's searcher (with the crate's own counter-evidence from the iterator layer) is real but not mechanically bounded — suggest-only at best.
- The 4×-unroll calibration split (memchr byte loop: unroll wins; packedpair loop: "quadruple unroll was unambiguously slower") is free calibration data for the unroll family's regime text.
- `unreachable_unchecked` bounds-check elision after opaque calls (memchr iter): real trick, UB-injecting — recorded, never proposed.

## Gates at landing

Full lib 2676/0 (default); clippy `-D warnings` ×4 postures (default / `--no-default-features` / `rust_perf,latent_retrieval` / `--all-features`); all 10 rules ParetoDominant via `perf_bench_demo` with alloc-det stable ×3 (2 mechanical findings healed via `cargo refine --fix --write --verify`, the B192 doc class; 2 too_many_lines allows per the entries_02 pattern); retrieval BLENDED floors HELD with no re-pin (125/131/133 vs floors 124/130 — attribution recorded in `retrieval_eval.rs`); gen2+gen3 freeze e2e `--ignored` both `ok` (rust_perf corpus delta, plan 137 T7.5); score-bench appended (run #184); count pins re-pinned (tests.rs total/without_spec/annotated/unannotated + mod.rs doc + AGENTS ×2 + README + `.docs/11_domains/rust_perf.md` ledger).

Batch 244 / Plan 247. `.raw/{regex,memchr,bstr}` removed at close (ephemeral-clone law).
