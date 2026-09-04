# Plan 076 — Batch 106: MTPLX upstream residue (axes A+B, mistakes/ corpus + the prefix-reuse lead)

Status: **IN FLIGHT (2026-09-05, claimed by the 4090 box idle loop — allocation claim pushed
early per the Issue-665 collision protocol).**

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
- [ ] T1: axis B — read all 16 `mistakes/` files; grade each as corpus-rule candidate /
  rider / doctrine citation / out-of-scope. Filenames ARE the rules (best
  confidence-per-token axis).
- [ ] T2: the B+ lead — locate BOTH instances' source anchors (the admission-shed commit /
  code + the sentinel mistake file), extract the decision content, write the rule.
- [ ] T3: axis A — skim `docs/plans` + `docs/specs` + `docs/concurrency` for kernel-shaped
  rule material; targeted diff-mining of `8bc4d88..dd4031a8` kernel/Metal/CUDA surfaces
  (NOT the full 300 files — targeted grep lanes).
- [ ] T4: dedup vs the landed corpus — B103's five (`shape-class-keyed-capability`,
  `wavefront-overlap-pair-ceiling`, `request-shape-decline-mirror-completeness`,
  `backing-provenance-causal-shadowed-extent-widen`,
  `dispatcher-eligibility-predicate-lags-kernel-capability`) + the same-input family
  (B58/B63/B77/B97/B98) + cache-key rules. Grep `buggy_code`/`fixed_code` BODIES, not just
  concepts (the B66 standing lesson).
- [ ] T5: land rules via the Python heredoc generator (the B77 escape-strip discipline;
  ensure_ascii=False) + buggy/clean fixtures + gates (present / parses /
  concepts_covered, cfg(not(kernel_verify)) per B70).
- [ ] T6: GOAT — batch106 gates + kernel_heal lib + default lib + kernel_heal+latent
  retrieval floors + clippy 0 both states; walk ALL count pins (tests.rs corpus count +
  expected-array + sums x2 + grouped + router.rs expert total + mod.rs header/category
  comments — the 18th-count-pin-site lesson).
- [ ] T7: records — Research 129 (verdict + anchors + license + pin + axis dispositions),
  AGENTS.md Batch-106 entry, queue snapshot update (row consumed), `rm -rf .raw/mtplx-upstream`.

## Routing

kernel_opt corpus (category D — dispatch/host-orchestration/admission) for the KV-shaped
rules; measurement-doctrine rows (p99 inter-emit gap, stale-binary provenance, delivered
cadence) = riders/doctrine citations, NOT corpus rules (the B49 measured-GOAT vs
process-note boundary). Axis C (cp.async B92 combine etc.) = riders only.

## Non-goals

No prefill-throughput claims; no league action; no MTPLX code changes (read-only mine).
