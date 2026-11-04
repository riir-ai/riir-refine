# 368 — Distill verdict: vectorize-io/hindsight (agent memory that learns)

**Status:** DISTILLED — pending owner decision

- **Source:** https://github.com/vectorize-io/hindsight @ `841979dfda4e1bebf7cbf68d0144d5d5454d93df` (shallow clone, tip commit dated 2026-11-04T11:10:58+01:00 — actively maintained; MIT, root LICENSE + README "MIT — see LICENSE").
- **Inspection date:** 2026-11-04 (zed-m3/M3, user-directed @distill).
- **What it is:** an agent memory system ("retain / recall / reflect") — LLM-assisted fact extraction at retain, 4-arm parallel retrieval (semantic / BM25 / graph / temporal) fused by reciprocal rank fusion then cross-encoder rerank, background observation consolidation (deduplicated, evidence-backed beliefs refined not overwritten), mental models (materialized standing answers refreshed on a schedule), per-bank "Memory Defense" (regex secret/PII redact-or-block at the persist boundary). LongMemEval SOTA claims, independently reproduced per their README; production-operated (Fortune 500 per their README — provenance, never a bar).
- **Language split (the transcription law — language is never a gate):** ~1,998 `.py` (the core is `hindsight-api-slim/hindsight_api/`: `engine/{search,consolidation,retain,reflect}`, `extensions/`), 36 `.rs` (hindsight-cli + hindsight-clients/rust — thin HTTP clients, bodies UNREAD), 591 `.ts` (Node client). The Rust surface is NOT the value; the decision content transcribes.
- **Provenance chain:** no upstream port — the mechanisms are hindsight-original (RRF and interleave are standard IR; the CJK boundary fix, the Luhn gate, and the cancel-then-await helper are their own production-repair surfaces with in-source tests pinning several).
- **Queue slot:** plan 338, nominal **Batch 330** — STAGED behind plans 336 (Batch 328, oss-fuzz) and 337 (Batch 329, voxl); claim plan-first at the next free slot.

## Verdict

**YES B− corpus** — 3–4 rules expected to land: 2–3 sec transcription candidates (founding the corpus's first sensitive-data-at-rest family) + 1–2 rust_perf candidates (concurrency + io), all NotBenchable/suggest-only annotation-class per the dial9/B258/penthera line. **Lane-intel: N/A on the league, STATED** (no GPU/kernel surface — an agent-memory product; the Metal/CUDA GDN/prefill/decode league lanes share no mechanism with it). **Retrieval-lane intel MODERATE** (below — recorded, never a batch item). **Product-coverage MODERATE** (below).

## Ranked candidate table

| # | Pattern | Source file (at pin) | Domain / category | Grade | Dedup evidence |
|---|---|---|---|---|---|
| 1 | `ascii-token-unicode-word-boundary-scan-miss` — a `\b`-anchored ASCII-prefix secret/PII pattern silently misses tokens adjacent to non-ASCII word characters, because `\b` (and `\w`) are Unicode-aware | `extensions/memory_defense.py` L133–148 (`_ASCII_TOKEN_START/_END`) | **sec** — new sensitive-data-at-rest family | **B−** | greps `unicode|cjk|word.bound|look.?around` over sec/ + rust_perf/: zero owners; the word-boundary hits are the corpus's own identifier-vocabulary gates (`secret_file_create.rs`, `secret_ct_eq.rs` — code-identifier detection, a different surface) |
| 2 | `luhn-validate-numeric-pii-candidates-before-redact` — digit-format-only detectors redact benign numeric runs; checksum-validate candidates (Luhn) before substitution | `extensions/memory_defense.py` L227–236 + `_is_luhn_card` L245–259 + the credit-card arm of `apply_redaction` L281–313 | **sec** | **B−** | greps `luhn|credit.card|checksum|validate.before`: zero owners (the checksum hits are `packed-field-silent-overflow-wrong-key`'s different mask/checksum vocabulary) |
| 3 | `fanout-first-error-leaves-siblings-running` — first-error propagation over a task fan-out leaves spawned siblings running; orphans keep side-effecting after the orchestrator has marked the operation failed and re-queued it | `engine/consolidation/consolidator.py` `_gather_or_cancel` doc L90–114 | **rust_perf** — concurrency | **B** | greps `joinset|abort|detach|orphan|sibling|cancel.*task|gather`: zero owners; entries_02's park/unpark idle-loop class and entries_05's bench-cancel methodology are different classes |
| 4 | `redaction-audit-hit-carries-raw-match` — detection/audit emissions carry the matched value; emit length-tiered fingerprint previews instead | `extensions/memory_defense.py` `_fingerprint_value` L79–105 + `DefenseDecision.hits` L54–67 | **sec** | **C+** (may drop at T0) | greps `fingerprint|mask|preview|blind` over sec/corpus.rs: zero owners; adjacent to `constant-time-secret-compare` (comparison-timing vs emission) and to the scanner lane's `--blind` discipline (a different surface) — distinctness argued in-rule |
| 5 | `query-term-truncation-by-selectivity-not-input-order` — first-N query-term truncation keeps common low-signal terms that drive fan-out; keep the lowest-df N (stats-backed, fail-soft fallback) | `engine/search/bm25_term_selection.py` module doc | **rust_perf** — io | **C+** (likely drop at T0) | greps `idf|document.freq|term.select|bm25`: zero owners in rust_perf; the only BM25 hit is kernel_opt `route_narrow.rs` (per-slice scoring, unrelated) |

## The quotes (read at the pin this session)

**C1 — the CJK boundary miss** (`memory_defense.py` L133–146, verbatim):

> "``re`` compiles ``\b`` (and ``\w``) with Unicode semantics, so a CJK
> character counts as a word character: there is no word boundary between
> 为 and s, and ``\bsk_test_...\b`` silently fails to match in
> 凭证为sk_test_ABC... . Secrets embedded in Chinese/Japanese/Korean prose
> therefore reached memory units unredacted."

and L145–146: "New boundary-based patterns must use this helper, not ``\b`` —
test_redaction_patterns_do_not_use_unicode_word_classes enforces it." — an
in-source test pinning the law. **Rust transcription:** the `regex` crate's
`\b` is Unicode-aware by default too (it keys on Unicode `\w`), so the same
class fires at home. The fix-space DIFFERS from the source's: the `regex`
crate has NO lookarounds — the bounded Rust spellings are (a) `(?-u:\b)`
(ASCII word boundary), (b) fancy-regex lookarounds (dep-gated), (c) post-match
byte-boundary validation (`is_ascii_alphanumeric`/`_` on the flanking bytes).
The CLASS is identical; the fixture is exact and trivial (scan multilingual
prose containing an embedded prefixed secret; assert the match).

**C2 — Luhn before redact** (`memory_defense.py` L228–230, verbatim): "Candidates
also need a Luhn check below: digit count alone redacted timestamps and port
lists as if they were cards." The bounded moves: Luhn checksum + the
repeated-digit guard ("Repeated-digit placeholders (including all zeroes) can
pass Luhn") + the UUID-overlap window ("A UUID's numeric segments may pass
Luhn. Inspect only the bounded neighborhood of this candidate (UUIDs are 36
chars)") + the overlap-rescan mechanic (L283–286, verbatim): "A rejected
candidate must not consume a later valid start (e.g. \"2026 4111 1111 1111
1111\"). re.sub resumes at the rejected span's end; search again from its next
character instead." — the rescan is a rider note on the same rule, not its own.

**C4 — orphaned fan-out siblings** (`consolidator.py` L93–113, verbatim):

> "Plain ``asyncio.gather`` re-raises the first exception immediately but does
> NOT cancel its siblings — they keep running detached. In consolidation that
> is actively harmful: the failure propagates out of ``run_consolidation_job``
> to the worker, which marks the operation failed and re-queues it with a 5s
> base backoff, while the orphaned tag groups are still calling the LLM,
> stamping ``mark_consolidated`` and committing write-groups. The per-scope
> ``scope_locks`` are local to one dispatch, so nothing serialises an orphan
> against the retry, and the "batches within a group run serially" invariant
> that keeps two consolidators out of the same observation scope is broken
> exactly when it matters."

The Rust twin: `tokio::spawn` + first-error early return — spawned tasks are
DETACHED; `try_join_all` stops polling futures but does not abort spawned
tasks. Fix-space: cancel-then-await before propagating (`JoinSet::join_all`
with `abort_all()` on the error path; the std-thread fixture twin is a
cancellation flag set before the early return, per the corpus's
standalone-compile fixture discipline — the law text names the async
spelling). Regime: side-effecting fan-out under an orchestrator that retries.
Their second atom, L109–113 verbatim: "Deliberately not ``asyncio.TaskGroup``:
it wraps failures in an ``ExceptionGroup``, and the worker's
``_is_non_retryable_task_error`` does ``isinstance`` checks on the raised
exception — a wrapped ``IntegrityConstraintViolationError`` would be
misclassified as retryable and retried forever." — carried as a context note
with the honest Rust-side caveat: anyhow's `.context()` PRESERVES downcast
through the envelope, so the Rust face of error-envelope-classification is
narrower (JoinError/panic envelopes + cancellation-safety), not the full
ExceptionGroup break. NotBenchable-by-domain: the cost is correctness
(double-writes breaking a serial-per-scope invariant), not a Pareto-validator
metric — the entries_43–46 concurrency annotation class.

**C5 — the +60s BM25 hang** (`bm25_term_selection.py` module doc, verbatim):
"On the native backend the ``@@`` gate then matches a large fraction of the
bank and ``ts_rank_cd`` — which has no IDF and is not index-backed — is
computed for *every* matched row, so ``ORDER BY ... LIMIT`` cannot prune before
ranking. A query that matches thousands of memories times out (the production
+60s BM25 hang this module addresses)." and: "Blindly truncating to the first N
tokens is the wrong cut: it keeps whichever terms happen to come first, which
are usually the common, low-signal words that drive the fan-out, and drops the
discriminative ones. Instead we keep the N tokens with the *lowest* corpus
document frequency". Fail-soft half verbatim: "On any failure ... we fall back
to keeping the first N tokens, so selection can never block recall." Grade C+
honestly: the regime needs a document-frequency stats source (pg_stats there);
at T0 it either stands as an io-family decision-content rule (selection-before-
fan-out with the loud fallback) or collapses into the generic pre-filter
intuition — adjudicated at the batch lane.

**C3 — fingerprinted audit previews** (`memory_defense.py` L83–86, verbatim):
"The preview keeps the prefix and a short suffix so a SIEM operator can
correlate against their credential inventory ... without the raw secret
crossing the wire." Length tiers: <6 fully masked; 6–15 first2+last2; >15
first4+last4. Bounded emission-time fix; the class is the AUDIT emission, not
the detection (which stays scanner-lane, never corpus — the skill's explicit
law).

## Drops (dedup/adjudication evidence, recorded not re-graded)

- **RRF itself** — algorithm adoption, not a bounded fix-space.
- **`interleave_fusion` vs RRF** (`fusion.py` L112–130) — a recall-QUALITY
  selection law, no rust_perf/sec fix-space; carried as retrieval-lane intel
  below. Their failure-mode doc, verbatim: "the near-identical existing
  observation (the \"twin\" to merge into) is semantic rank #1, yet shares no
  source-fact graph link and little lexical overlap, so RRF drops it below the
  recall budget cutoff and the LLM never sees it → creates a duplicate."
- **The rank-space-vs-score-space boost law** (`recall_boost.py` — see lane
  intel below) — same class: fusion-quality decision content, not a fix-space.
- **`cap_per_source`** — generic per-arm resource cap before a global trim; ambient.
- **Shared-context hoisted out of per-candidate payloads** (`decision_reranker.py`
  `rerank_shared_context`) — adjacent to the corpus's hoist family; thin.
- **The Memory Defense pattern catalog itself** (the 45-regex list) —
  scanner-lane material, never corpus (the skill's explicit law).
- **The allow/redact/block persist gate** (`DefenseAction`/`DefensePolicy`) —
  architectural policy, skill-checklist material (the B258 bulk precedent).
- **Observation consolidation** (evidence + proof count, refined-not-overwritten,
  `consolidator.py`) and **mental models** (materialized standing answers,
  scheduled refresh migration `f4d1c2b3a5e6`) — design corroboration for the
  neuron-db lane (below), not corpus rules.

## Retrieval-lane intel (recorded; never a batch item; no sibling issue filed)

The league lanes are N/A (stated, not skipped). But two fusion laws in
`engine/search/` map onto the workspace's retrieval/dedup surfaces:

1. **Rank-space, not score-space, when a hard cap follows** (`recall_boost.py`,
   verbatim): "RRF with ``k=60`` is deliberately flat: across the whole
   300-candidate cap window the score only spans ``1/61 -> 1/360``, a factor of
   5.9. Any ``w`` above that spread exceeds the entire dynamic range of the rank
   term, so the sort degenerates into a *lexicographic* one — boosted arm
   first, rank merely a tiebreaker. ... the boosted arm then filled all 300
   slots and no semantic-only candidate ever reached the cross-encoder
   (measured: recall@20 0.97 -> 0.40)." and "Boosting the rank instead —
   ``1/(k + rank/w)`` — cancels ``k``: the boosted arm's rank ``r`` beats
   another arm's rank ``s`` iff ``r < w*s``." — a MEASURED two-regime law
   (weight-below-dynamic-range vs degenerate-lexicographic) for any fused-then-
   truncated pipeline: riir-refine's own blended retrieval floors
   (BLENDED_TOP*/PERF_TOP* — any future per-domain boost before the top-K trim
   must move ranks, never multiply the fused score), neuron-db's AnyRAG
   multi-channel fusion, reflex/rethink candidate budgets before a think leg.
2. **Interleave when the consumer is dedup** (`fusion.py`): RRF's averaging-down
   drops the semantic-rank-#1 twin that lacks cross-arm support — the exact
   shape of the dedup-twin problem in reflex's near-duplicate leak index
   (slice_leak), refine's LatentFixMemory dedup, and neuron-db observation
   consolidation. Round-robin interleave guarantees every arm's top hits a
   slot; RRF is the law when the consumer scores the FUSED ranking.

Plus design corroboration (not new): observation consolidation (evidence +
proof count, refine-not-overwrite) echoes neuron-db's bitemporal-KG /
Utopia-distill direction; mental models = materialized standing answers with a
scheduled refresh — the "standing answer is a read, not a retrieval" cache
shape.

## Product-coverage axis

MODERATE. The sec corpus's families (web-trust, crypto hygiene, input
validation, unsafe/FFI hygiene, secrets-in-code scanner) all scan CODE or wire
trust; C1–C3 found the first family scanning DATA at a persistence boundary
(sensitive-data-at-rest). Follow-up pool (accumulation clause applies): the
source's own named layer-ups — Yelp detect-secrets and trufflehog pattern
catalogs ("operators who need that should layer a context-aware secret scanner
(detect-secrets, trufflehog) on top", `memory_defense.py` L157–162) — plus the
`secrecy` crate (SecretBox Debug-leak — already named in the sansstun follow-up
pool) and OWASP sensitive-data-exposure shapes.

## GOAT candidate statement

No candidate passes all four GOAT criteria — and every failure is criterion 1
for a DOMAIN reason: sec has no perf validator (probe-parity is the substitute
per the B258/B291 discipline); the concurrency candidate's cost is correctness
(double-writes) living in tokio runtime semantics, the entries_43–46
NotBenchable-by-domain annotation class; the io candidate needs a live DB
backend. Per the dial9 two-class split these are per-rule ANNOTATIONS, never a
demotion: graded on decision content × novelty × dedup clearance × yield.

## Honest caveats

- **Unread files (re-read at T0/mine time):** `reranking.py` body (555 l),
  `recall_boost.py` tail, `consolidator.py` body beyond the head (4,224 l —
  the proof_count/refine logic only skimmed), `engine/retain/` + `engine/reflect/`
  bodies, `hindsight-embed/` (Python embedding package), the Rust cli/client
  bodies (thin, marked unverified-thin), `engine/transfer/`.
- **The exact live corpus counts at claim time are the pins manifest's** (the
  `rule_id:` grep undercounts); this verdict cites rust_perf 294 / sec 38 per
  the Batch-326 landing row + AGENTS.md, re-derive at claim time.
- **WIP status:** none — production system, tip landed the day of inspection.
- **Maintenance:** very active (2026-11-04 tip); stars/history not graded (the
  skill: patterns are timeless).
- **Fixture discipline note for C4:** corpus fixtures are standalone
  rustc-compiled; the tokio spelling is named in law text, the fixture is the
  std-thread cancellation-flag twin — consistent with the concurrency family's
  landed precedent.
- **Fetch disclosure:** `git fetch origin` errored this session
  (auth/transient); `develop == origin/develop` at `0a960ec68` per last-known
  refs, no divergence markers — dual_allocation_gate run on local refs.

## Landings this commit

- Research 368 (this note; `.research/.highwater` 367→368).
- Plan 338 `.plans/338_batch330_hindsight_distill.md`
  (`.plans/.highwater` 337→338) — Batch 330, staged behind 336/337.
- Queue line in `.distill/001_mining_queue_snapshot.md`.
- `.raw/hindsight` REMOVED at this commit (the clone is throwaway; the pin
  above is the provenance — re-clone at the sha for T0).
