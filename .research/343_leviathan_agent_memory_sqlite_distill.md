# Leviathan — agent memory + lexical retrieval over SQLite FTS5 (distill)

**Status:** DISTILLED — pending owner decision (corpus batch staged as plan 306; the ndb/agents/rag reference mapping is the sibling note riir-neuron-db `.research/312_Leviathan_Agent_Memory_Lexical_Retrieval_Reference.md` @ `ced4379` — the owner-asked deliverable)

> **Verdict (Claude ping-pong, `request_verdict` session `103b2bad-5364-481a-bf73-33225be842c6`, rounds 1–3: REVISE → evidence round → AGREE close):**
> **YES B− corpus** [2–3 own rules; candidate grades below] **/ league-lane N/A (stated, not skipped — no GPU/serving-perf surface) / product-coverage: EXISTING CONSUMER WITH AN FTS5 LANE — rusqlite in 12 seal-game-editor crate/tool manifests, one shipping FTS5 store / research mapping HIGH** (the owner asked for the ndb/agent/rag ref; that mapping lives in ndb 312).

## Source

- `github.com/elstongun/leviathan` @ `f1070fe525527eec8ed52bfaf014b9e927224889` (tag v0.2.0), **Apache-2.0** verified at the pin. 868 stars, 6 commits, single Rust crate (`leviathan-index` on crates.io), ~10.5K LOC `src/`, `unsafe_code = "forbid"`, edition 2024, release profile `lto=thin, codegen-units=1, strip=true`.
- "Deep memory for agents": one static binary, two stores — (a) a RECORDS index (JSONL/CSV/SQLite → contentless FTS5; ranked cited cards, ~436 tokens/answer at 1M records vs 107,122 for grep, 33 ms median — their box, their synthetic dataset, provenance never bars) and (b) an agent MEMORY store (keyed slots, bitemporal supersede, per-kind half-life decay, token-budgeted recall + briefings, write-time secret refusal). CLI + MCP stdio + Streamable HTTP + OAuth 2.1. No models anywhere.
- Inspected 2026-10-28 from `riir-refine/.raw/leviathan` (shallow clone, read-only); clone removed at the verdict commit; every quote below byte-verified at the pin.

## Ranked candidate table

| Candidate | Domain / category | Grade | Source | Notes |
|---|---|---|---|---|
| `fts-query-dsl-term-requote` | `sec` / input-validation | **B+** | `src/text.rs:36-97` — user input re-tokenized (word regex `[\p{L}\p{N}][\p{L}\p{N}_.-]*`), phrases + `-exclusions` split, every term RE-QUOTED `format!("\\"{}\\"", t)` before entering the FTS5 MATCH expression ("so query syntax in user input is inert"), **term caps `.take(64)` words / `.take(32)` exclusions** | The class: interpolating raw user text into a search-engine query DSL (FTS5 MATCH, Lucene, tantivy) — operator injection (NOT/NEAR/*/`^`/`col:`/parens) → wrong results, unbounded expressions. Dedup: distinct from `shell-string-command-interpolation` (CWE-88 process-argument injection). **In-workspace known-good fixture:** seal-game-editor `editor-data/src/quest_search/mod.rs:314-325` `fts_match_expr` (quote + double embedded `"`, OR-join; quote-carrying tests at `tests.rs:79/205`). The measured-vs-editor-data DELTA is the term cap — editor-data's expression is UNBOUNDED; the 10K-term planner-blowup is UNMEASURED/HYPOTHESIZED (plan 306 T2 measures it first; the seal-game-editor issue, if filed, is HARDENING-grade — local editor user, no trust boundary crossed — never a DoS finding). Suggest-only. |
| `sqlite-bulk-load-pragmas` | `rust_perf` / io | **B−** | `src/index.rs:145-148` — `journal_mode=OFF; synchronous=OFF; temp_store=MEMORY; cache_size=-262144; page_size=8192` before the single-tx bulk build | Regime: one-shot bulk build vs OLTP. **REGIME WORDING (reviewer condition): the OFF/OFF half applies ONLY to building a fresh file that is atomically renamed into place (the `.building` → `fs::rename` shape, `index.rs:140-189`); importing into a LIVE store under WAL+NORMAL is NOT that regime — a single transaction is the fix there, never OFF/OFF.** rusqlite-dep-gated; NotBenchable-by-domain annotation (the rust_perf validator hosts no filesystem) — suggestion-grade, Lattimore-136 precedent; GOAT path = an fs-hosted microbench at a consumer call site. Consumer call site: seal-game-editor `store_open.rs:142-143` sets WAL+NORMAL (correct OLTP posture); the bulk import path carries no bulk posture — the rule applies **only if the import builds into a fresh file then swaps it in; otherwise a single transaction is the fix**. |
| `count-over-window-total` | `rust_perf` / io | **C+** | `src/query.rs:462-475` — `count(*) OVER ()` inside the ranked subquery gives the pagination total in one scan, vs a second COUNT query | Mechanical, rusqlite-dep-gated, expected drop-leaning. |

**Drops (with reasons):**
- **Secret-entropy gate** (`guard.rs:50-75` — Shannon entropy > 4.2 over mixed-case+digit tokens so hex/base32 ids pass) → our `secret_blind.rs` scanner lane already navigated the entropy-vs-hex false-positive terrain with documented blind surfaces (`:217` "Scoped to the GENERIC high-entropy kinds", `:632-634` "shapes carry ≈0 bits/char… v2 dropped the entropy" with reason). No new rule. The genuinely adoptable bit — **refusal at the WRITE API** vs our scan-at-mine/report posture — is a research-note item (ndb 312 row 12), not a corpus rule.
- **Retrieval-quality mechanisms** (half-life recency decay, confidence-as-multiplier scoring, subject-dominance damping, briefing round-robin, token-budget packing, slot semantics, hashed facet tokens, resolution ladder, atomic-swap build, overfetch-rerank) → retrieval/storage DESIGN content; the full 20-row mapping lives in ndb 312, never corpus.

## Dedup evidence (grep, both vocabularies)

- `journal_mode|synchronous|PRAGMA|rusqlite|sqlite` over `riir-refine/src/` — zero bulk-load rule hits (the `hashmap_with_capacity` hit is unrelated). **Workspace-wide:** seal-game-editor `sqlite_store/store_open.rs:142-143/:222-223` sets OLTP (WAL+NORMAL) and in-memory (MEMORY+OFF) postures only; NO bulk-load posture anywhere (`store_import.rs` carries zero PRAGMA rows) → dedup-CLEAN with a live consumer call site.
- `OVER ()|count(*) OVER|window function` over `riir-refine/src/` — zero hits.
- FTS/query-DSL: zero hits in `riir-refine/src/`. **Workspace-wide (case-insensitive, all seal-* repos):** seal-game-editor `editor-data/src/quest_search/` SHIPS FTS5 (`quest_fts` external-content + trigger sync, `mod.rs:24`; `fts_match_expr` `mod.rs:314`; `trigram_match_expr` `lexical.rs:28-30`; `WHERE quest_fts MATCH ?1` at `lexical.rs:89` + `cosine.rs:121`) — the re-quote idiom is already shipped at home, so candidate 1 is NOT novel-as-idiom; it is novel as the **term-cap + cross-DSL class rule** with a known-good in-repo positive fixture. seal-remake / seal-std / seal-render: zero FTS5 surface.

## Provenance chain

SQLite FTS5 + BM25 are the upstream techniques; the repo's contribution is the integration posture (contentless vs external-content per read/write pattern), the memory ergonomics (slots/supersede/half-life/budgets/secret-refusal), and the measured token-economics bench harness (`bench/`, `docs/BENCHMARKS.md`, `docs/MEMORY.md`). Benchmark dataset is synthetic and self-described as such.

## Honest caveats

- 6-commit repo, v0.2.0 — young but COMPLETE at module level (README, CI, tests, SECURITY.md, THIRD_PARTY.md, unsafe-forbid, clippy-configured); every module read is finished.
- Unread at verdict time (re-read at mine time): `bench/` harness internals, `examples/`, `skills/` content, `wrap.rs` (1,353-line agent-config template printer), `http/client.rs` beyond grep, `oauth.rs` beyond the token-storage path.
- No Pareto-benchable candidate in the batch — the whole batch is suggestion-grade with decision-content entries (NotBenchable annotations recorded per rule); the batch's value rides the rusqlite-family seeding against the live seal-game-editor consumer + the ndb 312 reference note.

## Queue slot

`riir-refine/.plans/306_leviathan_rusqlite_sec_corpus_batch.md` — nominal **Batch 301**, staged behind the queue heads (bbtree B298 rust_perf lease to 2026-10-29T09:44Z; perfect-hash B300 slot), cross-domain mix (rust_perf + sec) per the B293 precedent. Landing duties: dedup re-verification, 3 entries + original-Rust fixtures, standing gates, corpus count pins (rust_perf 274→276, sec 31→32 in AGENTS.md), freeze-table regen (plan 137 T7.5), score-bench, retrieval floors, clippy ×3 postures, release-set binary rebuild.
