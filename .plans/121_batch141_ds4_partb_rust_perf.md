# Plan 121 — Batch 141: the antirez/ds4 Part-B rust_perf distill (Research 162's staged batch)

**Status:** DONE — Batch 141 LANDED 2026-09-09; all tasks complete; record in the queue snapshot ("Last landed: Batch 141") + Research 162 (Status → RECORD).

- Source: `antirez/ds4` @ `6289c516` (MIT, verified at clone; `.raw/ds4b` — `rm -rf` at landing).
- Rules (R162, quotes re-verified at the pin this session):
  1. `compaction-reserve-before-prompt-build` (allocation) — the reserve for a destructive context rewrite is MEASURED from the real prompt (`agent_compact_make_prompt` tokenized + `tokens.len + 64 + summary_budget`), never a constant guess; every free-threshold check derives from it (call sites 6537/9273/9602/10010).
  2. `compaction-tail-boundary-prefers-complete-turn` (io) — truncation boundaries are semantic: walk back `2 × tail_budget` (floor sys_len) to the most recent complete user turn, bounded fragment fallback; image-block spans snap to span START (`agent_compact_image_boundary`) — never sever an atomic span. Verbatim source comment is the rule.
  3. `streaming-prefix-holdback-bounded-buffer` (io) — a streaming matcher holds back AT MOST O(pattern) bytes (`sizeof("> **Hint:**") - 1`), gated to prose line boundaries, flushing held bytes verbatim on first mismatch — ordinary text streams immediately, no width-dependent repainting.

## Tasks

- [x] Plan opened (this file; `.plans/.highwater` 120 → 121)
- [x] Clone `.raw/ds4b` at `6289c516`, verify HEAD == pin + MIT
- [x] Source quotes re-verified (reserve 9354-9364, tail 9286-9300, image 9367-9377, holdback 225-226/3290-3330)
- [x] Dedup re-verification (both vocabularies) against `src/domains/rust_perf/` — zero hits; `vec-with-capacity` = the named distinct-from, not coverage; B123's tri-state rule = the named compose-with
- [x] Author 3 corpus entries in `src/domains/rust_perf/corpus.rs` (after the batch136 trio), all NotBenchable-annotated (`bench_spec: None`, rationale in `context_notes`), fixtures ORIGINAL Rust
- [x] Count pins walked: header 121→124 (io 19→21, allocation 23→24), NotBenchable 27→30, header ledger +3 clauses, batch-list +3 lines, "Not benchable (31/121)"→"(34/124)", category prose, `entries.len` assert 121→124, `without_spec` 31→34 + 3 membership asserts, annotated ×3 (the serde + drafter sites surfaced by the test run)
- [x] Batch gate `batch141_rules_present_and_parse` + G1 function tests (reserve max-lift; boundary completes-turn + image snap; holdback bounded + flush-on-mismatch) — 2/2 FIRST-SHOT
- [x] AGENTS.md corpus counts (×2 sites: L18 domain list + L157 feature flag)
- [x] README.md rust_perf row + tree count
- [x] `.docs/11_domains/rust_perf.md` count + Batch-141 paragraph + 67-of-124; `.docs/11_domains/README.md` index (repaired a pre-existing 120 stale)
- [x] Research 162 Status: DISTILLED → RECORD (cite plan + batch)
- [x] Queue snapshot live-state: Part-B queued → LANDED + "Last landed: Batch 141" record
- [x] Gates: rust_perf lib 1193/0 + default 1193/0 + kernel_opt 1530/0/1i + kernel_opt+latent 1652/0/2i (retrieval + router floors held) + clippy 0 ×2 + score-bench #90 FLAT 96.2/1.9 + corpus-stats 124
- [x] `rm -rf .raw/ds4b` at landing; ONE commit; push
