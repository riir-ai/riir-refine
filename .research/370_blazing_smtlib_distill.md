# Research 370: blazing-smtlib — distill-source verdict

**Status:** DISTILLED — pending owner decision (verdict YES B+ corpus / lane-intel MEDIUM / product-coverage: seeds the text-processing family; batch plan 340 opened, conversion unclaimed)

**Source:** https://github.com/onderjan/blazing-smtlib @ `42b45c20b0045495a40b9eaf0a717e0f1c976bed` (MIT OR Apache-2.0; v0.1.0; HEAD commit dated 2026-10-29; shallow clone `.raw/blazing-smtlib`, removed at this note's commit — quotes re-verified at mine time against the pin).
**Paper provenance:** arXiv 2611.00185 "Blazing-Fast SMT-LIB Parsing for Rust and Lean" (Onderka, Biere, Böving; submitted 2026-10-30) — the library is NEW (no upstream port); the only "upstream" is the SMT-LIB 2 standard itself (v2.0/2.5/2.6/2.7). Source-reported performance: >6× geometric mean vs previous parsing libraries on a 2025 SMT-LIB corpus sample (14-job parallel, Ryzen 9 9950X3D) — source-reported figures are PROVENANCE, never bars.
**Inspection date:** 2026-11-05 (zed-m3, M3, network+read-only; zero cargo, zero GPU).
**Queue slot:** Batch 332 / Plan 340.

## What the source is

A zero-dependency (`[dependencies]` EMPTY), zero-unsafe, stack-overflow-proof-by-design SMT-LIB 2 parser: `rust/` (the Rust lib, ~20.5k LOC, edition 2024, rust-version 1.88, MIT OR Apache-2.0 dual) + `lean/` (BSmtLib, the Lean 4 port with the same architecture) + `tools/`. Completeness signals all green: published on crates.io (v0.1.0), three benches (`parse`, `census`, `compare`, `[profile.bench] debug = true` — they measure), corpus round-trip + trace-hash tests over the whole 2025 SMT-LIB benchmark corpus, grammar/recovery/version test batteries, docs. WIP signals: none found in the rust lib. Graded at MODULE level: everything read is complete.

## Ranked candidate table

| # | Pattern | Source site | Grade | Notes |
|---|---|---|---|---|
| 1 | **`byte-flag-table-masked-run-scan`** — 256-entry u8 class-flag table (one byte, 8 flag bits: SIMPLE_SYMBOL / DIGIT / HEX / BINARY / NON_LF_WS / QUOTED_PLAIN / STRING_PLAIN / COMMENT_PLAIN) + `run_end(&[u8], mask)` masked-prefix scan | `rust/src/lexer/char_class.rs` (`run_flags`, `RUN_FLAGS`, `run_end`) + `rust/src/lexer.rs` `lex_normal` hot loop | **A−/B+ (GOAT lead)** | In-source: "PERFORMANCE: Measured as superior to iter().position(), chunks, and SWAR." Folded half: `helpers.rs` `FAST` packed-attribute dispatch byte (DELIM/ON/EMIT/PIPE + 3-bit run-mask in ONE u8, const-eval built, coverage + agreement asserts). Two-regime: class-masked runs vs pure-block skip |
| 2 | **`token-batch-cursor-handover`** — fixed 256-slot staging array (`BATCH_CAPACITY`) of zero-copy `(InnerCategory, &[u8])` entries, flushed via `SmtLexerCursor` bulk handover; consumer actions `None`/`Relex`/`Discard` with the stop taking effect AFTER consumed tokens | `rust/src/lexer.rs` (`Batch`, `flush_batch`, `flush_cursor`) | **B− suggest-only** | CONSUMER-DISPATCH amortization — distinct from all four batching families (syscall / wakeup / command-barrier / pool-coordination); `fanout-transform-emit-callback-not-vec` is the opposite pole (callback-emit, allocation axis) |
| 3 | Interleaved A/B compare driver (build both revisions, run alternately file-by-file round-by-round, median of per-round ratios) | `rust/benches/compare.rs` | **KILL → CORROBORATION** | Positive instance of the workspace's OWN sequential-AB law (katgpt-rs `scripts/sequential_ab_timing_audit.py` + `tests/common/ab_timing.rs`): "Two sequential arms of the same work differ by +5–22% on a loaded box" — an independent third-party implementation of the same interleaved-ratio-median instrument, with the same drift premise stated in its header ("run-to-run drift … hides small changes when benchmarked one after the other") |
| 4 | Arena-as-explicit-stack + resumable slots (heap nesting for recursive descent; `SmtParseState` slots suspend/resume mid-construct across I/O chunks) | `rust/src/parser/state.rs`, `grammar/multiple.rs` | **KILL/ANNOTATE** | explicit-stack mechanism OWNED by `recursive-input-walk-explicit-stack-indices-not-copies` (entries_51); the streaming-resumable-slot half is decision content with a weak perf axis (robustness + memory-control) — watch row, rides a future parser-focused batch if one opens |
| 5 | Drop rows: 1 MiB `BUF_READER_CAPACITY` (positive instance, buffered-io family) · `FastState` stack locals around fallback ("use locals for variables commonly used on the fast path" — loop-invariant hoist family, micro) · recovering-lexer FSM (robustness, not perf) · SoA arena with typed handles + drain views (`SmtMultiple::new_starting` + truncate-after-handling — adjacent flat-pool/hot-slot-recycler families) | various | DROP | dedup gate working as designed |

## Dedup-grep evidence (corpus = `src/domains/rust_perf/corpus/`, 298 rules / 126 benchable / 172 NotBenchable per `.benchmarks/pins_manifest.json`, re-derived 2026-11-05)

- **Candidate 1** — greps `run_end|run flag|run mask|flag table|char.*class|byte.*table`: owners found = `ascii-run-swar-block-skip` (entries_29: high-bit gather skipping PURE-ASCII blocks — ONE class, LONG blocks, SIMD axis) + `swar-digit-chunk-parse` (SWAR arithmetic on digit runs) + `minimal-perfect-hash-static-lookup` (hash, not dense byte index) + `dense-subdomain-arithmetic-before-table-lookup` + `loop-invariant-dispatch-resolution-hoist`. ZERO owner of multi-class MASKED runs over a dense 256-entry flag table with a scalar unrolled scan. The two-regime pair is regime-COMPLEMENTARY with B243 (long pure blocks → SWAR gather; short mixed-class runs → masked table scan) — cross-cite, never merge.
- **Candidate 2** — greps `token batch|zero-copy token|batch.*handover`: the four batching families (`bufwriter-batched` entries_02, `channel-send-batching-unpark`, `deferred-command-flush-hoist-out-of-batch-loop` entries_08, `batch-blocking-pool-segment` entries_06/Research 179) own syscall/wakeup/barrier/pool-coordination barriers; none owns consumer-callback amortization with a fixed staging array + mid-batch abort semantics.
- **Candidate 4** — `recursive-input-walk-explicit-stack-indices-not-copies` (entries_51) owns explicit-stack-for-recursive-walks; dropped as same-mechanism family.
- P113 upstream-state check: ALL candidates are the repo's OWN deliberate landed decisions ("they DO X" — mineable decision content), none reads as "the repo should do X" — zero outbound-PR exposure.

## GOAT identification (the lead, four tests)

1. **Benchable** — rust_perf_bench Pareto validator + alloc-count axis: fixture = per-byte predicate match-chain walk vs table+`run_end` scan over original synthetic byte streams. YES.
2. **Exact semantics** — byte classification is integer table logic; bit-identical G1 trivially in the byte domain (Unicode-char generalization rides a class fn; the rule's regime is byte-oriented lexing — stated). YES.
3. **Magnitude story** — one table load + AND + branch per byte vs 3–5 unpredictable predicate branches per byte + per-byte dispatch; a token completes in ONE scan instead of a per-byte loop. Source-reported 6× geomean whole-library (their corpus/hardware — provenance); our fixture expectation: 2–5× on class-pure bulk, ~parity on delimiter-dense input. YES.
4. **Deterministic** — same input, same tokens, no timing-dependent verdicts. YES.

→ GOAT lead: `byte-flag-table-masked-run-scan`.

## Lane-intel axis (MANDATORY second output)

- **Mapped lane: riir-infer tokenizer pre-tokenization.** `../riir-infer/src/tokenizer.rs:1514-1595` — the GPT-2-style split rules (Rules 1–7) walk per-CHAR Unicode predicate chains (`chars[j].1.is_numeric()` / `.is_whitespace()` / `.is_alphabetic()`, plus the bounded digit-run and whitespace-run walks) on the laya encode path. Letters/digits/whitespace/punctuation are SEPARATE run classes with distinct predicates — exactly candidate 1's masked-run shape; B243's pure-ASCII skip is the sibling already on the books.
- **League context:** encode-side (prompt processing). The decode cell WON (Bench 1155); prefill is behind and rides the Issue-1146 tensor stack; tokenizer overhead is a small share vs attention/FFN — price it via the `laya_fixture_timing` lane before any change, never assume.
- **NOT transferable:** the 6× geomean (whole-library, their corpus); the SMT-LIB-specific tables themselves.
- **Offered follow-up (plan 340 T5):** the owning lane files an riir-infer issue naming the mechanism + our two corpus rules as reference designs; the verdict alone does not update sibling repos.

## Product-coverage axis (MANDATORY third output)

- **Family: text/byte processing — currently ONE member** (`ascii-run-swar-block-skip`, B243, the founding member). Candidates 1+2 are members 2–3.
- **Famous-lib surface:** hand-rolled lexers are top-tier Rust practice — rustc's own `rustc_lexer`, serde_json's `SliceRead`, quick-xml, csv, the HF tokenizers BPE pretokenizers. The family's rules fire at home too (riir-infer tokenizer, refine's own scanners).
- **Follow-up source pool (accumulation clause — read before the batch window, they fold into Batch 332):** memchr (the canonical byte-scan crate; its scalar fallbacks document the run-scan vs SIMD tradeoffs from the other side), rustc_lexer (the canonical in-tree hand-rolled lexer), logos (lexer generator with per-byte transition tables — the GENERATED form of candidate 1), serde_json `SliceRead` (whitespace-run skipping), quick-xml (byte-class readers).

## Honest caveats

- Single-maintainer NEW library (v0.1.0, Oct 2026; crates.io reservation showed 8 downloads pre-publication). Maintenance matters only for the provenance note per skill — patterns are timeless — but a v0.1.0 API may move; the PIN is load-bearing for every quote.
- All performance figures are source-reported on their hardware/corpus; the `compare` bench needs the external SMT-LIB corpus (not in-repo) to reproduce.
- Unread files (re-read at mine time): `handler.rs` (2221 ln, read structurally only), `lexicon/value.rs`, `writer.rs`, `token.rs`, `parser/category/*` details, the full `lean/` port (not graded — Lean-side decisions are out of rust_perf scope; noted: the paper's Lean-formalised lexer/parser equivalence is provenance lore for the FV family, not corpus material).
- The rust lib is ~20.5k LOC; this verdict read the lexer + parser-state + reader + benches + corpus-test halves; the grammar node halves were grepped, not read line-by-line.

## Bottom line

Mine as **Batch 332 (plan 340, opened this session)** — lead with `byte-flag-table-masked-run-scan` (GOAT-benchable, regime-complementary sibling of B243), optional second rule `token-batch-cursor-handover`. Yield estimate 1–2 rules after coordinator falsification. Lane follow-up: riir-infer tokenizer issue via the owning lane. Product-coverage: seeds the text-processing family; the follow-up pool folds into the same batch window.

## Addendum — the conversion landed (Batch 332, 2026-11-05, this session)

Both candidates survived T0 re-verification and falsification; the batch landed as `entries_65` (corpus 298 → 300, dispatch +2):

- **`byte-flag-table-masked-run-scan`** — BENCHABLE, category `dispatch`, **MEASURED ParetoDominant 3/0/0** on the rust_perf_bench validator (M3 Max / Mac15,8, release, m3 QUIET, AC power): **2.62× / 2.94× / 2.10×** at n=1K/16K/262K, **alloc 0/0** at every size (both arms alloc-free by construction). The standalone rustc direction check agreed (2.87× at n=262K, rounds=200). Fixture ORIGINAL (never paste source), standalone rustc-compiled AND executed pre-landing: per-byte class census 0..=255 + block-edge straddles 3/4/5/7/8/9/31/32/33 + generated SMT-script-shaped corpus equivalence at 1K/16K/262K.
- **`token-batch-cursor-handover`** — suggestion-only, NotBenchable (the consumer-cost class: a timing bench of the boundary would measure the validator's own consumer stub). Census fixture: identical token streams at n = 0/1/100/255/256/257/1000/16384 + the dispatch law (per-token arm = tokens calls; batched arm = ceil(tokens/256) calls + one stop-check per batch). Standalone-verified 1/1.
- The corroboration row stands (the interleaved compare driver ≡ the workspace's sequential-AB law), never a rule.
- Yield 2/2 (above the ~50% rate — the verdict's fixtures were already mechanism-level, the plan-first discipline held).

Gates ALL GREEN at the isolated `/tmp/b332_target`: batch332 needles+parse 1/1; default lib 2838/0; rust_perf_bench 2869/0 (8 ignored); score_bench suite 3221/0 (11 ignored); clippy −D ×4 postures (default / sec / rust_perf_bench / latent+sec) ZERO (1 too_many_lines mechanical on the bench-spec fn, allow-annotated en-route); retrieval floors: blended top-1 124/142 + top-3 130/142 UNCHANGED (floors 123/129 held with margin), **top-5 133 → 132 of 142 — ONE span (`repr-u8-enum`) demoted 5th → 6th by legitimate corpus growth (the two new text-rich dispatch entries outrank it inside the perf pool; the P7/B200 cross-pool-competition class, no old-span content change) — floor re-derived measured − ONE span → `BLENDED_TOP5` 131/142 per the Issue-063 law**; retrieval suite 27/0; score-bench run #227 appended (heal 0.9636 FLAT, parse 1.0, decline 1.0 — sha 4901fc4e at run time); freeze NOT owed (zero clippy_lints delta, the P30 rule); pins_manifest --write (rust_perf 298→300 / benchable 126→127 / notbenchable 172→173) + the 3 prose sites + the live-row history-row disambiguation (the B330 history row's "now" spelling rewritten non-live); count literals moved (×5 corpus total + ×3 annotated + 1 unannotated-unchanged + 1 expected-message); release-set binary REBUILT + installed at the pinned charitable set (`latent_retrieval,points,net-mainnet,fix_verify,update_check`, "release set: complete"; the build hash reads `-dirty` pre-landing, clean at the landing sha). `.raw/blazing-smtlib` + the fixture scratch removed at the landing.

T5 lane follow-up: the riir-infer issue recommendation stands as recorded in the verdict (the owning lane files it; this repo never updates sibling issues directly).
