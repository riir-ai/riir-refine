# Plan 129 — Batch 144: the B878–B884 measured-GOAT kernel_opt distillation

**Status:** COMPLETE — 2026-09-16 (Batch 144 landed; all gates green)

## Source

The three unchecked `- [ ]` P14 intake rows in `.distill/001_mining_queue_snapshot.md`
(L145 rung 6, L148 rung 5, L150 P14 NEW) — measured-GOAT distillations, the open
intake channel per the snapshot's standing rules. Source records: riir-ai
`.benchmarks/878`–`884` (read-only citations; no riir-ai edits this batch).
Authoring discipline: `.docs/02_corpus/authoring_discipline.md` (Issue 102 T5).

## Candidate rules (from the intake rows)

- (e) `packed-code-cpasync-stage-per-use-prmt-decode` — packed 2-bit codes shrink
  the A stage 4×, enabling deep multistage cp.async of BOTH operands + per-use
  register decode; beats amortized SWAR staging when the packed format is
  LDGSTS-servable (B883 GOAT, 1.222× rung). Negative-control arms folded in-rule:
  the 256-entry smem LUT / 2-blocks-SM / double-A variants (lead c) + per-fragment
  global-A staging at 16 warps (lead b). Category A +1. Distinct-from rider:
  `cpasync-multistage-tile-pipeline` (B9, entries_gpu_09).
- (f) `ldmatrix-byte-subword-granularity-limit` — ldmatrix distributes WHOLE words
  to lane columns; byte-sub-word fragment consumption (2-bit codes: 4 lanes of an
  mma quad need different BYTES of the same word) forces LDS-broadcast decode
  paths. Structural complement of `ldmatrix-mma-layout-loads` (B9). Category A +1.
  Distinct-from rider on the B9 rule.
- (g) `transposed-smem-stage-repairs-strided-epilogue` — an out-store loop whose
  lane→column map is transposed relative to the output layout costs 8× L2 write
  sectors; the transposed-smem-stage + `r = idx & 31` reader map + bank-shift +1
  mod 32 is the general fix (B884 GOAT, write sectors 35.65M → 4.46M, T4 1.8×
  gate CLEARED). Category A +1. Shared-epilogue-inheritance hazard in-rule.
- (h) `per-block-scalar-slab-cpasync-stage` — a scalar slab (scales) read per-warp
  from gmem at ≥4-B stride should be cp.async'd once per block per step into a
  double-buffered smem stage (4-B `.ca` chunks; `.cg` is 16-B-only). Honest cap:
  L2 share small — mine as ISSUE-COUNT/IPC rule (1.13→1.82), not bandwidth.
  Category A +1.
- (i) `profile-sol-table-before-compute-rung` — methodology: before proposing a
  compute-side kernel rung, record the SOL table + top-stall pair (the family
  spent 6 rungs optimizing instruction mix while the top SOL was L2 at 76–90%
  with a fixable 8× write amplification). Category: same home as
  `cross-run-profiles-key-on-content-not-stat-triplets` (adjudicate at authoring).

Dedup status (grepped 2026-09-16): no rule covers strided-epilogue store mapping,
scalar-slab staging, or SOL-first methodology; (e)/(f) are structural complements
of B9's `cpasync-multistage-tile-pipeline` / `ldmatrix-mma-layout-loads` — land
WITH distinct-from notes + riders, not duplicates. Lead (a) PRMT-table decode is
folded into (e) (zero dedup hits vs the B20 byte_perm crumb per the intake).

## Tasks

- [x] T1 — spawn 2 parallel subagents (quote-lane): A reads `884` (classes g/h/i +
      ncu data), B reads `883` + `882` + `880/881` (classes e/f + leads a–d +
      negatives). Each returns proposed rule text + exact quotes with doc line
      numbers. DONE: both lanes returned verified quote tables + 15 drift/cap
      findings.
- [x] T2 — re-verify every quoted line against the real benchmark docs (the
      Batch-31 discipline); correct drift before authoring. DONE: caller
      spot-checked 7 load-bearing quote sites (884:22/38-41/54-57/88-93,
      883:20-28, 879:78-86, 880:82-84) — verbatim; 9 intake drifts corrected at
      admission (see Research 169).
- [x] T3 — author `entries_gpu_41.rs` (5 rules, header with provenance +
      benchability note), wire mod.rs (decl + concat + slices A +4 / D +1 +
      category comment counts + Batch-144 history paragraph + 612 doc comment),
      standing gates (`batch144_rules_present` / `batch144_concepts_covered` /
      `batch144_pseudocode_parses` — one phrase pin fixed post-first-run), count
      pins (tests.rs ×6 sites incl. the per-category table, router.rs, AGENTS
      ×2, README ×2, `.docs/11_domains/kernel_opt.md` row + ledger paragraph).
      DONE.
- [x] T4 — riders: `cpasync-multistage-tile-pipeline` + `ldmatrix-mma-layout-loads`
      (entries_gpu_09.rs) gained the Batch-144 complement cross-references
      (file verified clean before touching). DONE.
- [x] T5 — (CARGO-GATED) GOAT: G1 batch gates 3/3 (one phrase-pin correction:
      rule (i)'s second distinct-from is "and from", not "Distinct from"); G2
      kernel_opt lib 1869/0 + default 1526/0; retrieval-precision kernel
      100/100/100 + blended 94.7/96.9/97.7 — HELD byte-identical, no re-pin;
      G3 clippy 0 in all three states (2 doc_markdown warnings on the new
      headers healed-0-then-manual); score-bench FLAT row #116 (96.2/1.9/64/
      vocab 39) + frontier snapshot appended. DONE.
- [x] T6 — research note `.research/169_batch144_b884_measured_goat_distill.md`
      (`**Status:** RECORD`), AGENTS.md corpus-count ×2, snapshot queue update
      (3 P14 rows → `[x]` with landing notes + the unverifiable-vocabulary drop
      recorded), commit `feat: batch 144: ...` + push. DONE.
- [x] T7 — plan close-out: tasks marked, `.plans/.highwater` = 129 (allocation
      consumed), queue-snapshot live-state in the same commit. DONE.

## Non-goals

- No riir-ai / riir-gpu edits (read-only citations; the owning lanes own their code).
- No league row (the rungs are already league-recorded in riir-ai benches; no new
  scoring content).
- No kernel_verify wiring (STUB stays; the G1 content is the measured tables).
- Not touching entries_gpu_27..40 beyond the two riders (sibling import-fixup
  history on those files — re-check `git status` at T4 time).

## Hold/unblock

No hold: sources are local docs; builds are M3-side with an isolated target dir
(the 4090 carries a sibling cargo build — no 4090 work this batch). CPU contention
from sibling dev builds is tolerated (box at 23%); if the box saturates mid-T5,
checkpoint T3 work and resume T5 after.
