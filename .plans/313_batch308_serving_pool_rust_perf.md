# Plan 313 — Batch 308: the 10-29b staged serving pool + rust_perf row

**Status:** COMPLETE — Batch 308 landed 2026-10-30 (corpus 278 -> 280, 2 own rules + 2 count-neutral rider faces; lease closed by the landing row; Research 352).

## Scope

The UNCLAIMED remainder of the 10-29b staged addendum (rows 6-9): the serving pool (3) + the rust_perf pool (1), converted as a rust_perf batch under the B293 cross-domain precedent. Zero candidate overlap with the live claims: B300 (perfect-hash, clippy corpus, to 10-30T09:34Z), B301 (leviathan Phase 3, plan 306, to 10-30T13:41Z), B306 (infinity tokio family, plan 310, to 10-31T14:21Z), plan-311 (hunch sched/SME, staged) — grep-verified per atom at T1 (below). Worktree `E:/git/wt-b308` @ origin `93bfdbf0` (the shared checkout stays untouched — 62-behind + the B300 lane's foreign WIP); gates under isolated `CARGO_TARGET_DIR=/tmp/plan313-target`.

## Candidate slate (T0 grades from the staged addendum bind; honest drops allowed)

| # | Lead | Source | Grade | Disposition |
|---|---|---|---|---|
| A | #62444 `c1b7dff` `tier-hit-refreshes-lru-recency-at-owning-tier` (eager CPU-offload: a GPU-tier prefix hit bypassed the CPU store path — the CPU LRU never saw the touch; fix records the hit at admission `_pending_local_hits`, touches + re-frees at the MRU end at the decode step) | vllm | DISTINCT B | OWN rule (caching) — the CROSS-TIER recency-coherence axis; the corpus's touch/promote family (`slru-probation-protected-promote-on-second-touch` etc.) is within-one-cache |
| B | #64256 `16777da` `peer-write-ack-gates-free-pool-handback` (a timed-out P2P load fails AT ONCE but its primary chunk stays OUT of the free pool until the peer's AbortAck — the peer may still be writing it; a missed ack FAULTS the session sticky so later fetches skip the dead peer) | vllm | RIDER-KIN B | count-neutral rider face on `drain-device-work-before-free` (kernel_opt entries_gpu_111, B257) — the REMOTE-writer arm of the completion axis |
| C | #31350 `7df8b87` `guarantee-admission-tests-consuming-size-not-derived-rollback-length` (recurrent-snapshot rollback checkpoints are GUARANTEED only for batches <= n_rs_seq+1; the admission tested `n_rollback` (derived) instead of `n_draft` (the consuming size)) | llama.cpp | RIDER-KIN B | count-neutral rider face on `admission-tests-the-bound-the-admitted-mode-serves` (entries_45, B271) — test-the-quantity-the-guarantee-is-scoped-to |
| D | TensorFold `d1fcb9d` #1539 `refusal-quarantines-entry-transient-passes` (a store/scatter REFUSAL — family rejects words/size/range — left the bad state in the store uncounted+unlogged so EVERY later recall re-read its files; fix = one helper routes refusals to bad (counted, logged, unlinked ONCE, file named), TRANSIENT errors pass through un-quarantined, and `forget()` drops the read-ahead entry AHEAD of every forgotten state) | TensorFold | B (three crisp clauses) | OWN rule (caching) — the refusal-VERDICT clause + the forget-ordering clause are unowned (the tombstone/quarantine cluster owns lifetime/pin axes) |

Pre-adjudicated kinship (staged addendum + slot re-verified at T1):

- A: `slru-probation-protected-promote-on-second-touch` + `cache-reuse-assertion-gated-on-eviction-counter` — both within-one-cache; no cross-tier recency owner. The staged row names the axis: "coherence BETWEEN tiers, vs the corpus's within-one-cache touch/promote family".
- B: `drain-device-work-before-free` (gpu_111) owns the LOCAL completion axis (drain before free); `ipc-peer-mapping-collective-teardown-pin-on-failure` (gpu_118, B272) owns the cross-process TEARDOWN axis (collective close-barrier-dispose, failure pins to exit); the new face is the STEADY-STATE failure-path free-pool handback gated on a per-chunk protocol ack + the sticky session fault.
- C: `admission-tests-the-bound-the-admitted-mode-serves` (entries_45) owns WHICH of two BOUNDS an admission tests (solo vs shared); the new face is WHICH SIZE the test uses when a downstream guarantee scopes a quantity — the consuming size, never a derived rollback length that can diverge from it.
- D: `batch-local-memoization-negative-cache-tombstone` (B242) + `pinned-entry-tombstone-freed-at-last-unpin` (entries_36) + `copy-source-quarantine-unproven-free-pinned` (kernel_opt) own the tombstone/pin/quarantine lifetime axes; `in-use-state-retry-under-serving-mark-keyed-on-refusal-cause` (entries_45) keys retry on the refusal CAUSE at serve time — none owns the refusal-vs-transient VERDICT classification (a deterministic refusal quarantines the entry; transient passes through keeping retry semantics) nor the forget-ordering clause (the quarantine drop takes the read-ahead entry with it, else a same-name state re-seeds the evicted entry).

## Tasks

- [x] T0 quote re-verification at the pins: the four sources fetched as per-commit .patch into `.raw/b308/` (vllm #62444 `c1b7dffa1457`, vllm #64256 `16777da3475`, llama.cpp #31350 `7df8b876b91`, TensorFold `d1fcb9d93458`); every staged quote byte-verified against the diff text (02:0x +07, pre-claim)
- [x] T1 dedup greps at 3+ vocabularies vs the corpus at origin HEAD `93bfdbf0` (per-atom above; the slug ledger + the category-family reads)
- [x] T2 author `entries_54.rs`: 2 own rules (A caching, D caching), ALL ANNOTATED NotBenchable bench_spec None, fixtures ORIGINAL machinery (never source code), each with regime/anti-regime + distinct-from sets + pin-verified quotes
- [x] T3 rider faces (count-neutral): B on `drain-device-work-before-free` (entries_gpu_111 gpu_concept extension), C on `admission-tests-the-bound-the-admitted-mode-serves` (entries_45 context_notes extension)
- [x] T4 standing gates: batch308 needle + parse gate; default lib; rust_perf_bench lib; clippy -D x3 postures (default / rust_perf / no-default)
- [x] T5 retrieval-precision re-measure BOTH postures (Issue 017/170 T3): default+latent (the Layer 5b2 command) + rust_perf+latent; floors held or re-pin with the delta recorded; freeze/score-bench NOT owed (P30 — the freeze corpus builder is the clippy_lints seed, a rust_perf-only delta moves nothing)
- [x] T6 count pins to measured truth: tests.rs len-form (278-280), annotated (222-224 x2 sites), unannotated (53 UNCHANGED), benchable (122 UNCHANGED), the category ledger line (caching +2), AGENTS.md x2 (274-276), README, .docs/11_domains/rust_perf.md
- [x] T7 Research note 352 + snapshot landing row (same commit)
- [ ] T8 release-set binary rebuild + install at the landing HEAD BEFORE push (rust_perf is IN the release set — corpus compiles into the bin; verify the `--version` build hash == HEAD), then commit `feat: batch 308: ...` + push; worktree + `.raw/b308/` removed at close

## Notes

- Honest drops allowed at the T2 re-read (the B271 precedent: 3 of 13 dropped).
- The llamacpp #31350 PR-bundle SHAs moved vs the staged pins (rebase); the per-commit .patch at the PINNED SHAs is the verification surface (done, byte-exact).
- Zero-GPU batch (host-side rules).
