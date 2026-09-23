# Plan 045 — Batch 84: the oMLX v0.5.4rc1 → v0.6.3 release delta (kernel_opt external batch)

**Status:** COMPLETE — LANDED 2026-08-28 at `20d551a` (+ repair `e40cb6e`, T8 checked "LANDED", pushed `f67c628..20d551a`); `latch-failed-accelerator-shed-to-fallback` stays DEFERRED (see Deferred). Status-corrected 2026-09-23 by the idle-loop plan-status hygiene pass (the IN-PROGRESS line predated the T8 landing).
**Source:** jundot/omlx releases v0.5.4rc1 → v0.6.3 (PRs #2966 / #3133 / #3110), primary
distilled record: `../riir-ai/.research/353_ane_metal_cpu_heterogeneous_prefill_decode.md`
§2.5.1 (queued in `.distill/001_mining_queue_snapshot.md` 2026-08-28, commit `b2a3295`)
**Scope:** 2 kernel_opt rules + 1 B56 rider. The 4th queued candidate
(`latch-failed-accelerator-shed-to-fallback`) is DEFERRED by the queue verdict — fold
into a measured-GOAT from our own `status=0x9` work instead of an external-only rule.

## Tasks

- [x] T1 — dedup grep BEFORE proposing (Protocol A step 2): pad-tail/tile (nearest =
      B57 tile-loop-back-edge test-coverage rule — different concern; B16/B30 clean per
      queue verdict); approximate-scope (nearest = exact-op-parity-for-analytic-backward
      phase gate — differentiation to encode; polynomial-taylor-feature-map is a
      technique not a delegation boundary); assigned-columns staging (zero hits).
- [x] T2 — plan-first (this file; highwater 44 → 45, folder re-scanned at write: no 045).
- [x] T3 — source verification via 2 parallel subagents with quote re-verification
      (DONE: PR #2966 fully verified — page + diff + release notes, crossover
      formula + tail probe + named regression tests; PR #3133 fully verified —
      z/qkv row split, cap log, 32K quality + hash reproducibility, and the PP→
      PREFILL-THROUGHPUT gloss correction; PR #3110 verified — slice-before-
      dequant bit-identical, GB figures flagged research-record-only).
- [x] T4 — corpus entries: `pad-tail-to-fixed-tile` (dispatch family), 
      `token-local-only-approximate-accelerator` (numerics/delegation family), and the
      B56 `assigned-columns-only-staging-dequant` rider (live-fields-only class at
      column granularity) — with in-rule differentiations vs B16 clamp-tile-indices,
      B30 pad-uniform-pod, B56 live-fields-only, exact-op-parity-for-analytic-backward.
- [x] T5 — standing syn-parse gate `batch84_pseudocode_parses` + concepts/count pins
      (walked: entries.len 396→398, Plan-003 table D 84/N 22, both sum asserts,
      router header + experts-cover assert, mod.rs headers + history block).
- [x] T6 — GOAT: G1 parse PASS (batch84 gate); G2 kernel_heal 966/0/1i + default
      736/0 + all-features 1471/0/14i; G3 clippy 0 in-repo under --all-features
      -D warnings + CI guard ALL LAYERS PASS (5 pre-existing latent breaks repaired
      in a separate commit first); retrieval floors HELD zero decay (kernel
      100/100/100 — 12th consecutive; blended 93.9/96.9/97.7; 14/14 tests).
- [x] T7 — research note `.research/099_omlx_v063_delta_batch84.md` (renumbered
      098→099: the sibling's cflow verdict consumed 098 mid-batch; highwater → 100)
      + AGENTS.md corpus count 396→398 + Batch 84 entry + `.docs/11_domains/
      kernel_opt.md` row + queue snapshot update (same-commit discipline).
- [x] T8 — commit `feat: batch 84: ...` + push. LANDED: repair commit `e40cb6e`
      (5 pre-existing breaks, my-hunks-only via a hand-built patch — the shared
      checkout carried the sibling's heal.rs WIP) + batch commit `20d551a`
      (7 files incl. this plan + Research 099 + highwater 100). Pushed
      f67c628..20d551a. The .distill snapshot carried the sibling's queued cflow
      entry (their staging raced the commit — noted in the message).

## Deferred

- [-] `latch-failed-accelerator-shed-to-fallback` (B−) — queue verdict: fold into a
      measured-GOAT from our own `status=0x9` retry-once work (riir-ai) instead of
      landing external-only. Reopen when that measured work exists.
