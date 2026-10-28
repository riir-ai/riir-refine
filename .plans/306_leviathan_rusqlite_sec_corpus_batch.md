# Plan 306 — Leviathan rusqlite + sec corpus batch (nominal Batch 301)

**Status:** STAGED — plan-first per the distill protocol; verdict note `.research/343`, reference mapping `../riir-neuron-db/.research/312`. Serialized behind the queue heads (bbtree B298 rust_perf lease to 2026-10-29T09:44Z; perfect-hash B300 slot) — claim per the mining queue before any corpus edit. Cross-domain mix (rust_perf + sec) per the B293 precedent.
**Claimed:** zed-m3-leviathan-remains, 1793194861, expires 2026-10-30T13:41:01+00:00

Claim scope note: Phase 1 pre-flight + Phase 2 measurement/audit — the CORPUS batch (T3+) still awaits the queue heads per the Status line above.

Source: `elstongun/leviathan` @ `f1070fe525527eec8ed52bfaf014b9e927224889` (Apache-2.0, v0.2.0). Clone is REMOVED (pin recorded in 343 + ndb 312); re-clone at the pin into `.raw/b301` at T2, byte-verify quotes, remove at close.

## Phase 1 — pre-flight

- [ ] T1.1 Claim the slot in `.distill/001_mining_queue_snapshot.md` (append CLAIM row with session + epoch), after the B298/B300 heads adjudicate free.
- [ ] T1.2 Re-clone `.raw/leviathan` at the pin; `git rev-parse HEAD` must equal `f1070fe525527eec8ed52bfaf014b9e927224889`.
- [ ] T1.3 Dedup re-verification at mine time, both vocabularies, all three candidates (`fts|MATCH|re-quote|term cap` in sec; `journal_mode|synchronous|PRAGMA|bulk` + `OVER ()` in rust_perf; workspace-wide seal-* sweep included) — record one evidence line per candidate; a new hit = re-adjudicate before authoring.

## Phase 2 — the seal-game-editor audit (the reviewer-conditioned measurement FIRST) — DONE 2026-10-28

- [x] T2.1 **Measured** (real cache copy `content/quest_search.db`, 798 quests, rusqlite 0.32 bundled SQLite 3.46.0, M3 Max, load-disclosed, within-run median of 7, between-run variance ≈2× shape-stable over 3 runs): **FtsTrigram (the DEFAULT strategy, T1.9 GOAT winner)** — 10 terms 525 µs / 5K diverse terms (25,266 grams, 227 KB expr) 554 ms / **10K diverse terms (35,528 grams, 320 KB expr) 908 ms per query**; Fts arm — 10 terms 92 µs / 10K diverse 224 ms / 10K shared-prefix 102 ms / 10K repeated-single-term 227 ms. `EXPLAIN QUERY PLAN` IDENTICAL at every scale (`SCAN f VIRTUAL TABLE INDEX 0:M2`) — the plan's "planner blowup" hypothesis REFUTED as mechanism; it is FTS5 expression-EVALUATION cost, superlinear in OR-term count. Diverse base62 terms are the honest worst case (shared-prefix shapes dedupe to ~1.1K grams — first-run artifact, corrected). **Harm is real on the DEFAULT arm**, and the search re-runs on EVERY query change (per keystroke — `feat-quest/src/semantic_search.rs` module doc): a pasted blob stalls ~0.9 s per keystroke until cleared.
- [x] T2.2 Operator-coverage audit of `fts_match_expr` (`mod.rs:314-324`) — CONFIRMED, behaviorally on the real table: input `NOT NEAR "star ^tilde (paren) col:title AND OR *asterisk` → expr all-quoted-or-OR (`"NOT" OR "NEAR" OR """star" OR "^tilde" OR "(paren)" OR "col:title" OR "AND" OR "*asterisk"` — the embedded `"` doubled by the FSM); bare `wolv*` (planted probe row) rows=1 vs re-quoted `"wolv*"` rows=0 — prefix operator inert; no NEAR/`^`/`col:`/parens/AND-OR-NOT escape. **The re-quote is correct; the only gap is the term cap.**
- [x] T2.3 Harm SHOWN → **seal-game-editor issue 273 FILED** (`d6e28ec9d`): HARDENING-titled (never a DoS finding), term-cap truncation to a pinned constant in both expression builders (N=32, the Leviathan `.take(64)/.take(32)` posture), cites this plan + the measurement; alternatives priced (reject>N / debounce), not required. The FIX itself is an sge lane's work (issue is the deliverable of this task).
- [x] T2.4 The audit's positive fixture doubles as the corpus entry's in-repo known-good instance (cite `mod.rs:314-325` + `tests.rs:79/205`) — carried into T3.1's `context_notes` at batch time; ALSO now cites issue 273 (the cap gap, measured + filed).

Probe note: the harness was TEMPORARY (untracked `tests/fts_blowup_probe.rs`, deleted after recording; the /tmp db copy + target dir removed). The issue carries the full reproduction recipe (shape generators + protocol), so the measurement is re-runnable from the issue text alone.

## Phase 3 — entries (fixtures are ORIGINAL minimal Rust reproductions; never paste Leviathan source)

- [ ] T3.1 `fts-query-dsl-term-requote` (sec, suggest-only, B+): buggy = `format!` interpolation of user text into a query-DSL string (FTS5-shaped); fixed = word-tokenize → re-quote each term → cap term count; regime condition: whenever input crosses a trust boundary into a search-engine expression. NotBenchable-by-domain annotation (`bench_spec: None`).
- [ ] T3.2 `sqlite-bulk-load-pragmas` (rust_perf/io, rusqlite-dep-gated, B−): decision content = the bulk-posture PRAGMA set + the REGIME LAW verbatim: OFF/OFF applies ONLY to fresh-file builds atomically renamed into place; a live WAL+NORMAL store takes a single transaction, never OFF/OFF. Suggestion-grade; `NotBenchable` in the validator (no fs); GOAT path named (fs-hosted microbench at a consumer call site).
- [ ] T3.3 `count-over-window-total` (rust_perf/io, C+, rusqlite-dep-gated): single-scan pagination total via `count(*) OVER ()` vs the second COUNT query.
- [ ] T3.4 Every entry carries `context_notes` provenance (Leviathan file:line at the pin + the in-repo fixture/call-site refs).

## Phase 4 — gates + pins

- [ ] T4.1 Corpus count pins re-measured and re-pinned: rust_perf 274→276, sec 31→32 (tests.rs asserts + the Plan-003 per-category table + `router.rs` total — the B299 lesson: the pin surface is ~37 asserts, grep them all).
- [ ] T4.2 Freeze-table regeneration in the SAME batch (plan 137 T7.5; `src/score_bench/generations.rs`).
- [ ] T4.3 `cargo refine --score-bench` run appended.
- [ ] T4.4 Retrieval floors both postures (kernel_opt unused here — run the default + `rust_perf`-relevant lanes per the landing duty set).
- [ ] T4.5 Clippy `-D warnings` ×3 postures (default / the batch's feature sets / all-features).
- [ ] T4.6 Release-set binary rebuild + `--version` stamp check (corpus changed → the maintained binary must serve the new rules; the bin-age check passes only after rebuild).
- [ ] T4.7 AGENTS.md corpus counts updated in the same commit (rust_perf + sec lines).

## Phase 5 — close

- [ ] T5.1 Update `.research/343` status (DISTILLED → LANDED as Batch 301) + this plan's checkboxes.
- [ ] T5.2 Queue snapshot row updated (Batch 301 LANDED line with counts + gate verdicts).
- [ ] T5.3 ndb 312 cross-ref note: batch landed (one line).
- [ ] T5.4 `rm -rf .raw/leviathan` (re-clone scratch) — cleanup is part of the batch.
- [ ] T5.5 Commit + push, `feat:` prefix, named staging only, `Session:` marker.

## Defers

- [-] T-D1 Pareto-benchable validation for the two rust_perf entries — the validator hosts no filesystem; the fs-hosted microbench belongs to a consumer-repo lane (seal-game-editor) and is OWNER-GATED there. Recorded as the rules' GOAT path, not this batch's task.
