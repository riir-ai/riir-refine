# Research 153 — the dhishwasher/Girder distill (verdict YES B− → sec-domain Batch 131)

**Status:** DISTILLED — pending owner decision (owner approved mining 2026-09-08 "do distill"; batch planned plan-first as Plan 112 / Batch 131 — flips to RECORD at landing)

## Source

- Repo: https://github.com/dhishwasher/girder — "gives coding agents exactly the code they
  need, instead of whole files": a semantic code-graph tool (tree-sitter → nodes/edges →
  MCP server, impact analysis, graph-addressed edits).
- **Pin:** `3b29453f1dec39ef24f975d76e25ab2600a38b8c` (2026-09-07, fresh at inspection
  2026-09-08). **License: BSL 1.1** (source-available; Apache-2.0 conversion 2030-09-04).
- Shape: 8-crate workspace, 120 `.rs` files, 60,679 LOC, single author, 10 stars / 0 forks,
  227 commits, actively developed. Release profile tuned (`opt-level 3`, thin LTO,
  `codegen-units 1`, strip, `panic = "abort"`) — they benchmark; good signal, grade-neutral.
- Cloned twice at the pin (verdict pass + DAP verification pass); first removed, second
  live at `.raw/girder` for mine time, removed at landing per the batch protocol.

## The correction story (honest provenance — the load-bearing lesson)

The FIRST verdict (2026-09-08, same day) graded **NO**, asserting "the `sec` domain DOES NOT
EXIST" — trusting the distill skill's routing table over the repository. Owner challenge:
"hmm, we do have sec domain? kernel_opt / rust_perf / clippy_lints / docker / sec". Measured
truth: **`sec` is live** — `sec = []` opt-in feature, 12 rules across 5 families
(3 Ffi, 2 DropLeak, 3 UnsafeHygiene, 3 CryptoHygiene, 1 PanicSafety), Phase 0+1 GOAT
(Bench 046), boundary gate `sec_boundary_gate` in Cargo.toml. The skill's table was a
snapshot taken before `sec` shipped.

Lesson (same class as feature-gate-audit defense #1): **a domain table in a skill is a doc;
the pre-flight corpus grep is the measurement.** The first pass skipped skill pre-flight step
2 (grep the target domain's corpus) precisely where it mattered. The corrected pass ran the
grep and the verdict flipped NO → YES B−.

## What the source is (and why an application repo still mines)

Not a technique library (the thermite/libdivide class). Application code is normally a rule
CONSUMER. Girder mines for exactly one reason: its security-hardening discipline is dense,
consistent, and negative-tested — and it contains a **live hazard instance of the target
class beside its own fixed twin**, which is the strongest recurrence evidence a corpus rule
can cite.

## Verified pattern evidence (both ways in one codebase)

| Site | Form | Detail |
|---|---|---|
| `crates/aether-dap/src/transport.rs` L24–55 | **UNBOUNDED** | `read_frame` parses `Content-Length: N` from the peer and executes `vec![0u8; len]` with NO upper bound — adapter-controlled `usize` straight into allocation (multi-GB → OOM; huge N → immediate alloc abort). |
| `crates/aether-app/src/project/collaboration_transport.rs` L1209–1221 | **BOUNDED twin** | `read_frame(.., limit)`: 4-byte BE prefix → validate `0 < n ≤ limit` BEFORE `vec![0; n]` → `read_exact`. Negative test writes `limit + 1` as the prefix (L2314). Outbound `write_frame` checks size too (L1199). |
| `collaboration_transport.rs` L248–266, `collaboration_identity.rs` L857–866, `collaboration_discovery.rs` L335–422 | secret-file gate family | `symlink_metadata` (lstat) → reject symlink → require regular file → enforce 0600/0700, on read AND write, each with negative tests (symlink fixture, broad-permissions fixture). |

Same author, two transports, one missed the gate. The class recurs even in careful code —
that is the corpus rule's reason to exist.

## Dedup evidence (both vocabularies, run against the live corpora)

- `src/domains/sec/`: zero hits for the frame/length-prefix-alloc class
  (`length|frame|read_exact|Content-Length|alloc|bound|limit` → only prose boundaries and
  the slicing rules; nothing owns wire-length allocation).
- Nearest neighbour: kernel_opt `length-prefix-shape-faithful-hash` (B32c) — the HASHING
  class (shape metadata into a digest), a different mechanism entirely; zero overlap.
- `constant-time-secret-compare`: not a duplicate — its vocabulary-gating discipline
  (secret-word-boundary + plain-ident operands) is the GATE PATTERN the new rule reuses.
- `kat_account.rs` `write_private` 0600: our own code = precedent for the portable
  `set_permissions` fix form, not corpus coverage (repo code is not corpus).
- `rust_perf` / `clippy_lints`: no bounded-fix-space shape — the petgraph reverse BFS
  (`impact.rs`), TF-IDF/Jaccard similarity (`similarity.rs`), and incremental tree-sitter
  diff sync (`sync.rs`) are standard implementations, not nameable techniques.

## Ranked candidates

| Candidate | Grade | Decision |
|---|---|---|
| `length-prefix-validate-before-alloc` (suggest-only) | **B− — the batch GOAT** | Two-regime (bounded vs unbounded length source), parse-visible same-fn trigger, deterministic, strict-gateable. Needs additive `SecFamily::InputValidation`. |
| `secret-file-create-without-0600` (write-half) | C+ | Admission-gated at mine time (T5 of Plan 112): enters only if its negative suite stays zero-FP. |
| secret-file-read-integrity-gate (symlink/regular-file before read) | C− | Absence-detection, loose — likely coordinator-killed; queue lead at best. |
| fail-closed config validation | DROP | No parse-visible bounded trigger (judgment-bearing — the exact class sec refuses). |
| digest-bound approval flow | DROP | Process pattern, not a code pattern. |
| route-context BFS pruning (`impact.rs`) | DROP | Genuinely clever domain algorithm; not a rule with bounded fix-space. |
| incremental parse→diff-into-graph sync (`sync.rs`) | DROP | Architecture pattern, not lint/perf/security rule. |

## Boundary fit — why suggest-only

`src/domains/sec/mod.rs` boundary: parse-only provable, bounded fix OR suggest-only,
strict-gated, **"a wrong security fix is worse than no fix."** A mechanical fix for the frame
rule must invent a limit constant and an error-return path (the fn may return `()`), which
is not mechanical → suggest-only, per the shipped `ffi-export-panicking-body` precedent.
Suggest-only rules emit, never auto-fix; `FIXABLE_SEC` stays unchanged.

## Yield estimate

1–2 suggest-only rules before the ~50% coordinator falsification; **~1 landing is a
success** (recent-batch norm is 1–3; this is the low end, honestly).

## Lane-intel axis

Checked per the mandatory second output — **does not fire**: no serving configs, no kernels,
no quant/ANE/prefill mechanisms; zero overlap with the league rows (prefill 0.53–0.57× vs
llama.cpp; decode WON). No league line, no issue-filing recommendation.

## Tooling side-question (owner, 2026-09-08) — recorded to prevent re-evaluation

Owner: "apart from clippy distill, their Semantic code graph is useless for us? we already
have better one eg RAG right?" — **Verdict: redundant for THIS workspace**, with the precise
boundary stated honestly: RAG answers intent-relevance (fuzzy, latent); a static call graph
answers exact structural queries (callers/callees/impact) — NOT the same tool in general,
and RAG is not a call-graph replacement in the abstract. But for our workflows the
call-graph need is already covered, cross-repo, by the existing stack:

- substrate-first greps with vocabulary translation — crosses all 17 sibling repos; Girder
  indexes ONE repo (project root fixed at server start, single-repo measurements by its own
  README) and cannot see our cross-repo path-dep substrate.
- `cargo tree` — the dependency graph our boundary contracts actually enforce
  (`ci_boundary_contract.sh`); a tree-sitter function graph is not this graph.
- riir-rag / BM25 / AnyRAG / KG triples (riir-neuron-db) — intent retrieval + entity
  relations in our own vocabulary.
- Zed's LSP — exact symbol navigation in-editor, where the work happens.

Girder's own honest limits bite our code hardest: **feature-gate-blind** (tree-sitter sees
`#[cfg]`-gated code as plain text — it would index never-compiled code as live; our heaviest
axis, per the AGENTS.md full-gate table), and **dynamic-dispatch recall 0.000 by its own
measurement** (`docs/core-representative-mutations.md`) — our trait-heavy code
(`HealDomain`, ECS systems) is exactly that shape. Net: do not adopt; the corpus rule (the
bounded-frame discipline) is the transferable part, which is this batch.

## Honest caveats

- BSL 1.1: fixtures must be ORIGINAL minimal reproductions (the standing constraint, applied
  strictly); quotes limited to evidence citations in this note; no Girder bytes land in the
  corpus.
- 10-star single-author provenance (grade-neutral per the skill, recorded for the note).
- Unread at verdict: `aether-debugger/*`, `aether-extensions/src/marketplace.rs` — not
  load-bearing for the two candidates; re-clone at the pin if a third candidate emerges.
- The first verdict's NO was wrong for a process reason (stale table trusted over the
  corpus grep), not a measurement reason — the corrected verdict's dedup and boundary
  evidence is the authoritative record.

## Queue slot

Plan 112 / Batch 131 (plan-first, created before any corpus edit). The CloudRift lead-pair
(queued ahead of this verdict) renumbered to batch-132/133 candidates in the same commit —
no number recycled; the pair's queue line is untouched except the earmark numbers.
