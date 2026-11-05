# Research 372 — Distill verdict: nnethercote "How to speed up the Rust compiler in November 2026"

**Status:** DISTILLED — staged as plan 343 (nominal Batch 334), unclaimed; claim-before-edit per protocol. Discharges the standing "nnethercote ~11-01" author-scoped dated lead (the lead awaited nnethercote-AUTHORED work; this is his month-2 Hexcat report; the 2026-11-03 #164484 verdict (Research 363) recorded the lead OPEN as author-scoped — this verdict is the author-scoped re-sweep that closes it).

## Source

- Blog: https://nnethercote.github.io/2026/11/04/how-to-speed-up-the-rust-compiler-in-november-2026.html (published 2026-11-04; fetched in full this session, no clone — the mine surface is the named rust-lang/rust PRs, API re-read at claim time).
- PRs named: #163857 (span compression scheme), #163971 (span op micro-opts, jttriplett), #164291 (pattern-type niche — ABANDONED, negative result), #163628 (single-entry cache in front of the parent-span hash table), #163981/#164234/#165160 (new trait solver micro-opts ×3), #164236 (early-return work avoidance), #165022 (AttrTokenStream/AttrTokenTree removal), #164965 (one type variable per element in `vec![...]` literals), #164341 (coercion early-return cases), #165112 (trivial_consts MIR-skip, DRAFT), #164484 (sparse metadata encoding — ALREADY LANDED here as Batch 326).
- Inspection date: 2026-11-05 (zed-m3/M3). Provenance chain: nnethercote (Hexcat) → rust-lang/rust PRs; every mechanism quote is the blog's own text, read live this session; PR-diff re-verification owed at claim (T0).

## Verdict

**YES B− corpus** [2 candidates: 1 benchable GOAT lead (B) + 1 layout candidate (B−, expected-drop-allowed)] / **lane-intel N/A STATED** (compile-time domain — the gains reach the workspace passively via `rust-toolchain.toml` pin bumps; build-hygiene owns our compile-time measurement discipline; zero mapping to the Metal/CUDA GDN/prefill/decode/quant league lanes; the new-trait-solver overhead reduction 1.49×→1.16× on Diesel is a WATCH-grade note for the next toolchain bump, not a lane) / **product-coverage N/A STATED** (the patterns live at std-collections level — no famous-lib family, no follow-up pool; the applicability is universal-by-construction, which needs no seeding).

## Ranked candidates

| # | Grade | Candidate | Source | Notes |
|---|---|---|---|---|
| 1 | **B — GOAT lead, BENCHABLE** | `single-entry-last-key-lookup-cache-hash-avoidance` | #163628 | One-slot last-(key → value) memo in front of a hot hash table: compare the query key against the cached last key (one cheap Eq) before hashing+probing; on match reuse the output. Upstream: "typically need to get information about the same parent span from a hash table over and over... a single-entry cache in front of that hash table... avoids the hash table lookup in most cases"; best case >4% instruction reduction across rustc benchmarks, mostly incremental builds (provenance, never bars). |
| 2 | **B− — expected-drop-allowed** | `skewed-payload-inline-encode-intern-overflow-tail` | #163857 + #163971 | Distribution-skewed multi-field payload (most values small/zero/small-int; one field only needed in incremental builds) → pack the common cases into one machine word inline; intern the rare tail into a side table; keep the frequently-read-alone field inline even in the interned case. Layout category, suggestion-tier, bench_spec: None per the size-axis class (the box-large-enum-variant / narrow-counter-overflow-promotion precedents). |

### The GOAT candidate — four-criteria test

1. **Benchable** — PASS: rust_perf_bench Pareto validator + exact alloc-count axis; a hot keyed-lookup loop with a deterministic repeated-key sequence, memo-hit arm vs direct-map arm, alloc-free (inline `Option<(K, V)>` slot).
2. **Exact semantics** — PASS: pure memoization; both arms return identical values (checksum-asserted in-fixture). The memo keys on Eq exactly as the HashMap does (the standard Hash/Eq consistency contract); the rule must carry ONE correctness constraint: the memo is sound only across a READ-STABLE window — invalidate (or drop the memo) when the map mutates (upstream's window is the metadata encode/decode + stable-hashing phase).
3. **Magnitude story** — PASS: hash+probe (hash ~10-40 cycles + 1-2 probe cache lines) vs one cache-hot Eq compare (~1-5 cycles); upstream measured >4% instructions at whole-crate granularity.
4. **Deterministic** — PASS: seeded key sequence, same input same output.

### Dedup evidence (greps run 2026-11-05 at rust_perf 300 live from pins_manifest; 3+ vocabularies)

- `single.entry|one.entry|last.value|last.lookup|last.key|temporal.local|one.slot|single.slot|last.hit|hot.key|lookup.cache|hash.avoid|skip.hash` over rust_perf + kernel_opt corpora → **ZERO owners**. Kinships to cite IN-RULE (never merge): entries_49 `certifying-token` (skip repeated work when the input repeats — a builder-minted identity witness vs this rule's query-path lookup memo); entries_29 `range-table-page-memoization` (per-page 256-answer memo for classification vs a 1-slot map-lookup memo); entries_09 owner-lifetime artifact memo (a computed artifact keyed on all inputs vs a map lookup); the `fxhashmap` proposer (FASTER hash — the alternative mechanism; this rule AVOIDS hashing entirely — the two compose, never conflict); entries_08 interleaved [value, fingerprint] slots (map LAYOUT vs query-path memo).
- `intern|inline.pack|small.string|compact.encod|niche|packed.repr` → no rust_perf owner of the pack-common-intern-tail law; kernel_opt `tokenizer-segment-memo-between-mutations` (gpu_102) is a cross-domain memo kinship; the landed `adaptive-sparse-blocked-table-encoding` (entries_60, Batch 326, same domain, same source family) is the NAMED counter-precedent that rustc representation work converts — rule 2 cites it.
- `early.return|short.circuit|reject` → the ordering-law shape has NO owner and must STAY unowned: the invent-check drop class (the sanstun candidate-5 precedent) — a buggy fixture would have to INVENT the misplaced work.
- `fixpoint|min-rank` → the Sept post's staged lead `dataflow-fixpoint-convergence-traversal-order` ALREADY LANDED (entries_16) — the monthly cadence item's prior conversion is on the books; positive instance, never re-graded.

## Drops (each with its reason)

- **#164484 sparse metadata encoding** — LANDED as Batch 326 / plan 330 / entries_60 `adaptive-sparse-blocked-table-encoding`. The article's later figures (metadata −37%, max-RSS −7%) are a CORROBORATION row for the landed rule, never new content.
- **#164236 / #164341 early-return work avoidance** — ambient code-motion ordering; the invent-check class; the 10-17 pass title-triaged #164236 "none converts at fidelity" — carried.
- **#164965 one type variable per element in `vec![...]`** — compiler-inference-internal mechanism; the app-level generalization collapses to loop-invariant hoisting (entries_48 class 5 owns invariant-resolution hoisting); upstream itself notes the win was stress-test-only.
- **#165022 AttrTokenStream removal** — DRY mirror-type deletion for one call site; simplification, no bounded mechanical app-code fix-space.
- **#165112 trivial_consts MIR-skip** — DRAFT PR, compiler-pass machinery, controversial upstream (their own Zulip thread); no app shape.
- **#163981 / #164234 / #165160 new-solver micro-opts (a dozen)** — ambient class (early returns, allocation avoidance, recomputation avoidance); each atom owned by existing families; #163981 pre-adjudicated by the 10-17 pass.
- **#163971 span-op micro-opts** — positive-instance micro-opt class (folded as rule 2's refinement face).
- **#164291 pattern-type niche** — NEGATIVE RESULT carried as rule 2's anti-regime honesty: "It worked but didn't help performance... and it made the code more complicated. So I abandoned it" — representation tricks can cost complexity for zero measurable win.
- **Overall-progress stats** (2.85% mean wall / 4.46% mean max-RSS; 560/561 instruction improvements YTD) — context, not rules.

## Yield estimate

1–2 rules land after ~50% falsification (rule 1 high-confidence; rule 2 ~60% survival — drop honestly at T0 if the 10-17 fidelity concern survives the full-diff read). Corpus targets: rust_perf 300→301-302 (benchable 127→128 on rule 1; notbenchable +1 if rule 2 lands).

## Honest caveats

- Blog-level mechanism quotes only this session — every PR diff is UNREAD; T0 re-reads #163628 + #163857/#163971 at the API and re-verifies every quote (the never-grade-an-unread-file law, carried as the claim's first task).
- The 10-17 watch pass pre-adjudicated #163857 by TITLE as "none converts at fidelity"; rule 2's staging leans on the B326 counter-precedent — the adjudication is genuinely open, which is why rule 2 is expected-drop-allowed rather than asserted.
- Upstream figures are rustc-benchmark provenance (never bars); the fixture's magnitude must stand on its own validator run at --release with bench_preflight PROVENANCE.
- #165112 is a draft PR — its drop is not adjudicated against a final state.

## Queue slot

Research 372 + plan 343 (nominal Batch 334, STAGED, unclaimed) + this note, one commit; highwaters 371→372 / 342→343. The standing "nnethercote ~11-01" dated lead is DISCHARGED by this verdict — the next nnethercote poll re-arms at ~12-01 (the monthly cadence continues). NOTHING INVENTED — every quote read at the live blog fetch this session; Session: zed-m3-nnov-distill, epoch 1793868141.
