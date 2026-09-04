# Plan 076 — Batch 106: MTPLX upstream residue (axes A+B, mistakes/ corpus + the prefix-reuse lead)

Status: **COMPLETE (2026-09-05) — Batch 106 landed: 5 rules, corpus 477 → 482 (D 106 →
111); all gates green first-shot; Research 129 is the record.**

Consumes the queued MTPLX RESIDUE row from
`.distill/001_mining_queue_snapshot.md` (queued 2026-09-04, cadence-gated behind the
B105 defers — B105 landed, gate open). Source: youssofal/MTPLX upstream main
**@ `dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d`** (v2.11.1, 2026-09-04T10:25Z, Apache-2.0)
— 196 commits / 300 files ahead of the Batch-103 mined base `8bc4d88`; upstream `mistakes/`
= **16 files** (7 upstream-only vs the 9 in the retired vendor clone); `docs/plans` 1217 /
`docs/specs` 757 / `docs/concurrency` 259 lines unread. Fresh full clone at
`.raw/mtplx-upstream` (`core.longpaths` — the mistake filenames are Windows-MAX_PATH-hostile).
License + pin recorded BEFORE any verdict, per the `.raw` discipline.

## GOAT lead (B+, from the queue row)

`prefix-reuse-key-must-mirror-the-restore-predicate` — TWO independent instances in one
source: (1) the 2.10.2 admission shed asked `SessionBank.longest_prefix` EXACT containment
while restore serves by BLOCK prefix ⇒ 41,901 tokens re-prefilled cold (54 s) with a
41,391-token restore available; (2) the trailing-sentinel bank-fingerprint mistake
(`mistakes/per-request-steering-…trailing-sentinel…bank-fingerprint….md`) — the mirror-image
direction (fingerprint keyed on a token change that does NOT change KV ⇒ every transition
round re-prefilled cold). Sibling of the landed B103 `shape-class-keyed-capability` one
layer up (capability-key vs shape-tuple; this is cache-key vs consumer predicate), and the
FIRST MTPLX row with an in-stack analog (Issue 742 GDN+KV prefix cache, 441× TTFT).

## Tasks

- [x] T0: repair `.research/.highwater` (stale 127, 128 exists → 128, allocate 129); bump
  `.plans/.highwater` 75 → 76; file this plan; mark the queue row CLAIMED; push the claim.
- [x] T1: axis B — read all 16 `mistakes/` files; grade each as corpus-rule candidate /
  rider / doctrine citation / out-of-scope. Filenames ARE the rules (best
  confidence-per-token axis). Result: 4 corpus rules + riders; 11 files dispositioned
  out-of-scope (AppKit/QA-hygiene classes) in Research 129.
- [x] T2: the B+ lead — BOTH instances found in-source (the sentinel mistake file + the
  dd4031a8 fix commit itself + `_policy_fingerprint` comment + `longest_shared_prefix_tokens`
  docstring); third corroborating instance in the numerics spec. Rule written.
- [x] T3: axis A — docs/plans + specs + concurrency read (the mode-vs-lane split and
  width-neutral identity restate landed rules; the numerics-profile METHOD is our GOAT
  doctrine externally confirmed); targeted diff = the vendored Steel sparse-GQA cluster →
  rule 5 (`vendor-fused-op-instantiation-check`) + B100/B103 riders; `docs/perf/` subdirs
  + `docs/research/` recorded as the named-unread remainder.
- [x] T4: dedup vs the landed corpus — bodies grepped; 5 rules distinct with explicit
  cross-references (B103 ×2, B94 ×2, B77, B36, B25, B23, B105); ledger in Research 129.
- [x] T5: landed via the file-form Python generator (the heredoc form hit a shell
  length limit — new tooling lesson) + parse-only fixtures + the 3 standing gates.
- [x] T6: GOAT — batch106 gates 3/3 first-shot + kernel_heal lib 1343/0/1i + default lib
  1078/0 + kernel_heal+latent 1451/0/2i (floors HELD zero re-pins) + clippy 0 both
  states; count pins walked + the "464-rule" mod.rs doc line repaired 464 → 482 (2
  batches stale).
- [x] T7: records — Research 129, AGENTS.md Batch-106 entry + kernel count line, queue
  snapshot row consumed, `.docs/11_domains/kernel_opt.md` D-row, `.raw/mtplx-upstream`
  removed.

## Routing

kernel_opt corpus (category D — dispatch/host-orchestration/admission) for the KV-shaped
rules; measurement-doctrine rows (p99 inter-emit gap, stale-binary provenance, delivered
cadence) = riders/doctrine citations, NOT corpus rules (the B49 measured-GOAT vs
process-note boundary). Axis C (cp.async B92 combine etc.) = riders only.

## Non-goals

No prefill-throughput claims; no league action; no MTPLX code changes (read-only mine).
