# Research 275 — Tokenmaxxing BERT (the tokenizers normalizer 50× pack) — distill verdict

**Status:** DISTILLED — batch 243 chartered (plan 245, nominal Batch 243; corpus edits deferred to the batch slot per the plan-first discipline). ⚠ NUMBERING NOTE: originally written as research 274 / plan 244 / nominal Batch 242; RENUMBERED pre-push (275/245/243) when the sibling's on-origin claim row (`4ace9a2e`) reserved Batch 242 / plan 244 / research 274 for the mlx-serve conversion — the Bench-881 first-committed-on-origin rule; their committed claim keeps the label, this verdict yields.

## Source

- **Blog:** <https://huggingface.co/blog/huggingface/tokenmaxxing-bert> — "Tokenmaxxing BERT: 50x faster tokenization" (Hugging Face, published 2026-10-15, same day as this verdict).
- **Code home:** `huggingface/tokenizers`, the BERT-normalizer optimization branch @ `5b5a3fc` (blog-stated; compared against `main` @ `fb49a29`, "a few commits after 1.0.0-rc2"). ⚠ **The changes were still in Pull Request stage at blog time** — the branch sha is the pin-of-record; re-verify it moved/merged at conversion (the B193 freshness law).
- **License:** tokenizers is Apache-2.0. Blog quotes only (no code pasted — fixtures stay original per the constraint).
- **Inspection date:** 2026-10-15 (M3, user-directed @distill). **Inspection depth: blog-depth** — the full blog read (mechanisms + code snippets + methodology appendix); the PR diff, the v1 announcement post, and the `hf-internal-testing/tokenizers-test-data` reproducer are UNREAD — re-read at conversion. No `.raw` clone this session (the standing conversion discipline: clone + quote re-verify at the pin when the batch slot opens).

## What the source did (the mechanism inventory)

Profiling a warm `bert-base-uncased` encode pass: **88% of encode time in the normalizer, 8% in WordPiece**, and about two-thirds of the WHOLE encode inside `binary_search_by` from `char::is_mark_nonspacing()` (the `unicode_categories` crate's "is this an accent?" check — a binary search over a sorted `&'static [char]` of 1,567 entries ≈ 6 KB, ~11 unpredictable branches per lookup) — **run for every character, including plain `s`, `l`, `o`, `w`**. The fix is not making the work faster; it is SKIPPING it, on three data-distribution facts: most text is ASCII, the rest is clustered by script, and most text is already normalized.

1. **ASCII-run skip via high-bit-gather block classification (SIMD/SWAR):** in UTF-8 a byte is ASCII iff its high bit is 0; no normalization form changes an ASCII character and an accent can't attach across one — so whole ASCII runs are skipped: never decoded, never looked up, never copied. 16-byte chunks: gather the 16 high bits into a bitmask with one instruction (`u8x16::from(chunk).to_bitmask()`); mask == 0 → skip the whole block; else `mask.trailing_zeros()` = the first non-ASCII byte. "~three instructions for 16 bytes." BERT's ASCII-only fixes (whitespace, control, A–Z) ride the same block pass: 16-wide compares + one OR `0x20` on capitals (ASCII case differs by bit 5 alone).
2. **Per-page memoized property cache:** text uses few scripts and scripts are contiguous in code point space (Cyrillic = 256 consecutive code points). The only question the normalizer asks per char is the boolean "could normalization touch it?" — so compute it ONCE per page of 256 code points on first touch, keep the 256 yes/no bytes (4 cache lines, L1-resident); every later hit is one memory read instead of a binary search. The answer cached is the PREDICATE, not the value.
3. **Borrow-until-first-change:** stretches with nothing to do are never copied; if nothing changed the input is handed back as-is (`Cow::Borrowed`, zero allocation); on the first real change the output is allocated ONCE and the untouched prefix is copied in bulk.
4. (Rider) ASCII lowercase = single bit-5 OR, done vectorized inside the block pass.

**Results (their hardware — EPYC 7R13 ×2 + Xeon 8488C, one pinned thread, alternating A/B rounds, token-id hashes identical across main/branch/0.23.2 on every corpus):** BERT up to another 5.7× over the already-8.6×-faster v1 main ≈ **50× vs 0.23**; normalizer share 88% → 13%; callgrind instruction count on 1 MB English 482M → 72M (6.7× fewer). Speedup scales with ASCII share (English/code biggest; Russian/Greek/Arabic ~11% ASCII still 2.2–3.2× via the page cache; ModernBERT/NFC gains mostly from the cache since NFC-on-main already skipped ASCII). **Honest caveats in-source:** tiny inputs ≤ 32 B can be SLOWER (down to 0.35× on 8 B of Chinese); cased BERT on Chinese 8–15% slower from 64 B up; Chinese gains ~nothing (little ASCII, and every CJK char must be padded with spaces anyway — pays for optimizations it can't use). 55 of 320 normalizer-only sweep cells slower beyond noise (19 of them ≤ 32 B).

## Domain routing

**`rust_perf`** — three categories: simd (candidate 1), caching (candidate 2), allocation (candidate 3). ⚠ The tempting WRONG domain named: `kernel_opt` — "SIMD" here is CPU 128-bit vector/SWAR byte classification, zero GPU surface anywhere in the source (the CPU-SIMD-is-not-kernel_opt trap). Not `clippy_lints` — perf patterns, none parse-provable.

## Dedup gate (run over `riir-refine/src/`, both vocabularies, 2026-10-15)

| Candidate | Source vocab grep | Corpus operator-name grep | Evidence |
|---|---|---|---|
| ascii-run-swar-block-skip | `ascii`, `to_bitmask`, `bitmask`, `trailing_zeros`, `high bit`, `0x20` | block-skip / run-skip / byte-class / simd-scan | Only app-code `is_ascii_*` uses (`heal.rs`, `build_stamp.rs`, `cf_workers_ai.rs`); NO corpus entry. Nearest neighbors owned by OTHER mechanisms: `byte-find-position-to-memchr` (entries_04 — single-LITERAL find; its Batch-172 rider = multi-NEEDLE anchor-byte; both find-shaped, one hit) · `bool-to-bitpack` (entries_01 — set-bit COUNTING over pre-packed words, no skip/locate) · `simd-chunk`/`chunks-exact` (arithmetic auto-vectorize) · `unaligned-word-load-from-bytes` (word assembly). **Run-PURITY classification + bulk SKIP (the per-char work never paid) is unowned. PASS.** |
| range-table-page-memoization | `page cache`, `memo`, `codepoint`, `binary search table` | lookup-table / precompute / lazy-cache / page | `precompute-table` (entries_01, caching — FULL-domain const-eval table; its own notes record it PARETO-DOMINATED by POPCNT for tiny domains — it owns the small-domain rung, and shows the family is regime-sensitive) · `bsearch-sorted-slice` (entries_01 — linear→binary search; the corpus ENDORSES binary search — this source moves to the rung ABOVE it) · `regex-hoist-lazylock` / `runtime-feature-detect-per-chunk` / `sort-key-cache-precompute-out-of-lock` (compile-once / detection-hoist / maintained-key — different axes). **Lazy per-RANGE memoization over a HUGE dense domain (1.1M code points) is unowned. PASS.** |
| transform-borrow-until-first-change | `Cow`, `borrowed`, `no-op`, `alloc once`, `skip copy` | borrowed-view / collect-avoidance / defer-alloc | `encoded-layout-borrowed-view` (entries_04, allocation — borrowing a DICTIONARY-ENCODED data layout's view instead of materializing R owned Strings; an ACCESS-shape law over pre-encoded data, no transform, no first-change pivot) · `collect-avoidance` (entries_01 — chained iterators) · `static-str` (constant labels). **The scan-validate-then-Cow shape (transform usually a no-op → return the borrowed input; first change → allocate ONCE + bulk-copy the untouched prefix) is unowned. PASS — weakest of the three; at-mine adjacency re-read owed.** |

Zero-hit confirmation under both vocabularies for the family as a whole: the corpus has **no text/unicode/normalization family at all** (no rule_id matching `text|str|char|unicode|normaliz|tokeniz|bert|wordpiece|nfd`).

## Product-coverage axis — the reason this grades YES despite a modest candidate count

`huggingface/tokenizers` is the ecosystem's tokenizer substrate — the Rust core behind `transformers`/`tokenizers` Py wheels; every HF model load passes through it (the blog: across the 10K most-downloaded Hub models, BPE = 842M monthly downloads, WordPiece = 627M — every one of the latter runs BERT's normalizer; more than half of BPE downloads run a Unicode normal form). **The corpus carries bevy/wasm/rayon families for exactly this class of coverage and has ZERO text/tokenization family.** This verdict seeds it (founding member).

**The family's follow-up pool** (accumulation clause: pool sources read before the batch window fold into the same batch):

1. `huggingface/tokenizers` @ the branch pin `5b5a3fc` — the actual normalizer impls (`tokenizers/tk-encode/src/normalizers/bert.rs` is the blog's named home; the NFC/Unicode-normalization arm too). THE primary conversion source.
2. BurntSushi's regex byte-class law (regex-automata translates character classes to BYTE classes precisely so SIMD fast paths fire — the ecosystem's canonical upstream of the same mechanism class; the "Rust regex 1.0" writeups + the crate's `class`/`dfa` modules).
3. `simdutf` — the C++ reference library for UTF-8 validation/classification SIMD (the ASCII-fast-path shape industrialized; license/shape to verify at read).
4. `unicode-normalization` crate — the NFD slow path this work bypasses; its table structure is the honest fixture shape for the page-cache rule.
5. `bstr` / `memchr` internals — the byte-scan family's mature forms (memchr already mined once via `byte-find-position-to-memchr`; its iterator/avx2 modules hold more class-scan shapes).

## Lane-intel axis

No scored league lane (the league is GPU decode/prefill; this is CPU host-stage tokenization). Two detection-only notes:

- **GraphTok alignment (Research 265, walk #15):** our own arxiv intel already named the axis — "under prefix caching TOKENIZATION becomes the largest TTFT part and bounds the fleet one CPU frontend feeds" (2610.13325). GraphTok attacks it by GPU-replaying the BPE pipeline; this source attacks the same stage's CPU cost head-on (50× on the normalizer half). External confirmation that the host-stage tokenization cost is real and the CPU-side fix class exists.
- **In-lane surface (UNVERIFIED this session — check at conversion):** `riir-infer` ships hand-written GGUF tokenizer code (`SentencePieceGgufTokenizer` / `BpeTokenizer` — the `scripts/plan402_gguf_probe.py` tokenizer-model-type dispatch). Any per-char classification/normalization loop there is a direct consumer of all three laws. Also `riir-infer-laya` path-deps the `tokenizers` crate (v0.22 pinned): reflex's recorded tokenizers-v1 reopen trigger (vocab compat — the 14 ByteLevel byte atoms v1 validates eagerly) now carries a measured 30–50× encode payload on the same release, raising the trigger's eventual payoff. Detection-only — never a scored bar; no sibling issue is edited from this verdict.

## Ranked candidate table

| # | Pattern (proposed rule_id) | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `ascii-run-swar-block-skip` — classify whole byte runs against a class predicate with a high-bit-gather mask; skip pure runs wholesale (never decode/lookup/copy), scalar only at the boundary. SWAR form: `u64::from_le_bytes(chunk) & 0x8080_8080_8080_8080` (stable, auto-vectorizes; the 16-byte `to_bitmask()` form is the same law). Rider: ASCII case = bit-5 OR, vectorized in the same block pass. | Blog §"Most of the text is ASCII" | **B+** | GOAT-lead. Distinct-from `byte-find-position-to-memchr` (find ONE literal vs skip CLASS-pure runs), `bool-to-bitpack` (count vs locate+skip), `simd-chunk` (arithmetic). Anti-regimes from source: ≤32 B inputs pay machinery (0.35× measured); CJK ~no gain; win scales with class-pure share (their Figure 16 = speedup-vs-ASCII-share — the regime IS the data distribution). Benchable: per-char classify loop vs SWAR skip + scalar tail. |
| 2 | `range-table-page-memoization` — a sorted-table property lookup over a HUGE dense domain, queried with locality → memoize the PREDICATE ANSWER per aligned page (256 entries / 4 cache lines), built lazily on first touch; one L1 read per hit replaces ~11 unpredictable branches. The domain-size ladder: full-domain const table (domain fits, `precompute-table`) > page cache (huge dense domain + locality) > binary search (sparse/random access — `bsearch-sorted-slice` stands). | Blog §"The rest of the text is clustered" + Appendix A | **B+** | GOAT co-lead. Integer-exact; bench: binary search over a 1,567-entry sorted table vs 256-page byte cache, text-like query distribution. Anti-regimes: single-shot/random queries (every miss builds 256 answers for one question); emoji/page-scatter text; tiny inputs (init overhead — the same ≤32 B class). Fixture shape honest: a sorted `[char]`/`[u32]` table + `binary_search`, per the real `unicode_categories` villain. |
| 3 | `transform-borrow-until-first-change` — a usually-no-op transform returns `Cow::Borrowed(input)` (zero alloc) after a validation pass; on the FIRST change allocates ONCE and bulk-copies the untouched prefix, then writes only the changed tail. | Blog §"Most of the text is already normalized" | **B−** | Distinct-from `encoded-layout-borrowed-view` (access-shape over pre-encoded data; this is a TRANSFORM pipeline's no-op fast path + the bulk-prefix copy-on-first-change) and `collect-avoidance` (chained iterators). Weakest: the defer-the-allocation half is the corpus's most familiar family; the bulk-prefix sub-content is the distinctive part. At-mine adjacency re-read owed. |
| — | ASCII-case bit-5 OR | Blog §ASCII properties | **rider** | Rides rule 1 (same block pass: 16-wide compares + one OR on capitals). Not standalone. |
| — | "Profile your DATA, not just your code" | Blog §"Profiling your data" | **DROP** | Ambient meta-law, no bounded fix-space — rides as regime framing in rules 1–3 (the ASCII-share-of-YOUR-corpus note; the win is data-distribution-conditional by their own Figure 16). |

## GOAT assessment (lead: rule 1, co-lead: rule 2)

| Criterion | `ascii-run-swar-block-skip` | `range-table-page-memoization` |
|---|---|---|
| 1. Benchable on the Pareto validator | YES — pure std, no nightly `std::simd` needed (the SWAR u64 form); sizes axis = input length × ASCII share | YES — plain Rust; sizes axis = query count × locality |
| 2. Exact semantics | YES — boolean class property, bit-identical trivially | YES — boolean predicate over a fixed table, exact |
| 3. Magnitude story | YES — per-char: UTF-8 decode + table binary search (~11 unpredictable branches) → ~3 instr per 16 B; source end-to-end 6.7× fewer instructions, normalizer share 88%→13% | YES — ~11 unpredictable branches per lookup → 1 L1 read (source: two-thirds of the whole encode in one `binary_search_by`) |
| 4. Deterministic | YES | YES |

## Yield estimate

3 rules + 1 rider authored; ~50% coordinator falsification → **2–3 rules land** (rules 1+2 are the robust pair; rule 3 is the likely drop if the adjacency read goes badly). Counts move ×1 site per rule (+1 rust_perf each; rider count-neutral).

## Honest caveats

- **Blog-depth inspection** — no `.raw` clone this session; every quote above is from the blog (its code snippets are self-declared simplified). At conversion: clone `huggingface/tokenizers`, pin the branch sha (blog says `5b5a3fc`; verify merged/moved), re-verify every quote at the pin, read the real normalizer impls. The PR-stage WIP status makes the pin load-bearing (the standing discipline).
- **Their numbers are their hardware** (EPYC/Xeon, their corpora, alternating A/B protocol with same-token-hash verification — a genuinely good protocol worth citing as the G2 shape). Never cited as our bench claims; our numbers come from the validator.
- **The 50× is compound** — 8.6× (v1 vs 0.23, prior work) × ~5.7× (this PR). Our rules cover only THIS PR's mechanisms.
- The villain fixture must be honest: `unicode_categories`' 1,567-entry sorted `[char]` table is the real shape — not a strawman linear scan.
- Maintenance/provenance: Hugging Face production library, actively maintained, stable release train (v1.0.0 in flight) — no WIP-repo warning beyond the PR-stage pin.

## Queue slot

**Batch 243, chartered by plan 245** — deferred to the next cargo-quiet mining slot (the queue's sibling claim stands through Batch 241; this batch takes the next one). Plan-first per Protocol A step 0: the plan EXISTS (244); corpus edits happen only at the batch slot, with the .raw clone + at-pin quote re-verification as its step 0.
