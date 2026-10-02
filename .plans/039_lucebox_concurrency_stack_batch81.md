# Plan 039 — Batch 81: the lucebox concurrency-stack external kernel distill (kernel_opt)

**Status:** COMPLETE (landed 2026-08-27, same session)

Source: Research 095's queued verdict (PRs #642/#651/#659/#668, YES A−).
Companion: Plan 548 P1-2 T2 port list (the port itself is GPU-gated — NOT this
batch). This batch is the corpus half only: 2-4 kernel_opt rules + the P3-2
rider (Plan 548's [CPU-NOW] item folded here per the session handoff).

Diffs pre-fetched to `target/lucebox_81/` (pr651/pr659/pr668.diff) — subagents
read LOCAL files, no network dependency.

## Task list

- [x] T1 Dedup greps (coordinator) — zero direct hits for
  virtual-tail/replay-log/graph-retention/identity-keyed/retry-block/
  commit-transaction/cross-lane (first pass done pre-plan; Lane C does the
  deep adjudication vs the ~10 adjacent rules)
- [x] T2 Source verification — A/B/G/H VERIFIED by coordinator from
  pr651.diff (full read: `paged_attn_tree_visible`, `virtual_tokens`,
  `retry_blocks` LIFO, `GGML_ABORT` transaction, `draft_pack_columns`,
  `capture_replay_log` owns-storage, `fmaf(key, delta, gate*current)`);
  D/F/I via Lane B subagent from pr659.diff + pr668.diff
- [x] T3 Lane B (subagent) — verify + propose D (cross-lane projection
  packing: mech 1-2 measured tables) + F (identity-keyed graph retention:
  mech 6, −99.98% target_graph_build) from the local diff files
- [x] T4 Lane C (subagent) — corpus dedup adjudication: A/B/G/H/D/F vs
  B58 same-input family + B25 hot-swap + B63 weight-concat + B77 memo +
  B39 rewind + B71 rotated-ring + B34 op-major-loop-inversion + B23
  fold-query-tokens + B29 weight-handle-cache-identity + batch80
  dispatch-cost; kill/merge verdicts + category + differentiation clauses
- [x] T5 Author 2-4 rules (expect A + B + F strongest; D/G/H per
  adjudication — G/H may fold into A's rule as contract content)
- [x] T6 P3-2 rider: extend B77 `shared-activation-quantize-memo-per-eval`
  with the RELEASE half (memo freed at request-scratch release; #668
  2.2-2.4 GiB; memo lifetime pairs with the REQUEST lifecycle) — update
  the pinned concept phrase in tests.rs on BOTH sides
- [x] T7 Corpus insertion (entries_gpu_07.rs) + `batch81_pseudocode_parses`
  standing gate
- [x] T8 Count pins: entries.len 387→N (×3 sites) + KERNEL_CATEGORIES
  table + Plan-003 table + router experts-cover + STALE header re-scan
- [x] T9 GOAT: kernel_opt lib + default lib + all-features + clippy 0
  both states (`--all-targets`) + CI guard layers + retrieval floors
  (kernel 100/100/100, blended — re-pinned only if decay, record honestly)
- [x] T10 Research 095 Status → RECORD + batch results appended
- [x] T11 AGENTS.md corpus count + `.docs/11_domains/kernel_opt.md` row
- [x] T12 Queue snapshot entry + commit `feat: batch 81: ...` + push

## Standing constraints carried

- Fixtures ORIGINAL (C++/HIP source; patterns port, code does not)
- Numbers are R9700/gfx1201 — cite as source evidence, never as our gate
- Provenance: #642 superseded by #651 (never cite as separate); the
  #651/#659 description tables re-verified at diff-read time (T2/T3)
- Generator: python here-doc with rustlit() escaping (B77/B80 path) —
  never the structured-edit tool for string-heavy entries
- Count-pin walk incl. STALE headers (B80 caught 3)
- Doc-markdown: backtick O()-math, `//!` prose identifiers, no line-initial
  `+ `/`- ` in wrapped list items

## Drops pre-recorded (Research 095)

- E standalone (graph-bucket specialization = B77 batch-width-routed
  positive instance unless we bucket identically)
- W16 widening (we ship block-16; confirmed stays)
- async metadata upload (their own rejection; rule-annotation class only)
- #642 wholesale (superseded)
- C as corpus rule (rider on B35 for the PORT — Plan 548 GPU-gated, not
  this batch; the B35 doc-comment rider may ride the port commit instead)
