# Research 352 — Batch 308: the 10-29b staged serving pool + rust_perf row

**Status:** RECORD — landed as Batch 308 (Plan 313); every quote verbatim at the pin.

## Source window

The 10-29b lead-delta sweep's staged addendum (shikuwa-4090-leads1029b, window
2026-10-29T04:43Z..11:45Z), rows 6-9 — the UNCLAIMED remainder after B307 consumed the
kernel_opt pool (rows 1-5). Four sources:

| # | Source | Grade | Disposition |
|---|---|---|---|
| 6 | vllm #62444 @ `c1b7dffa1457` (Apache-2.0) | DISTINCT B | OWN rule `tier-hit-refreshes-lru-recency-at-owning-tier` (caching) |
| 7 | vllm #64256 @ `16777da3475` (Apache-2.0) | RIDER-KIN B | rider face on kernel_opt gpu_111 `drain-device-work-before-free` |
| 8 | llama.cpp #31350 @ `7df8b876b91` (MIT) | RIDER-KIN B | rider face on entries_45 `admission-tests-the-bound-the-admitted-mode-serves` |
| 9 | TensorFold `d1fcb9d93458` #1539 (Apache-2.0) | B | OWN rule `refusal-quarantines-entry-transient-passes` (caching) |

## T0 verification notes

The `/pull/N.patch` PR-bundle SHAs do NOT contain the staged pins for #62444/#64256/#31350
(the PR branches were rebased between the sweep's 11:45Z clone-read and the fetch) — the
per-commit `.patch` at each PINNED sha is the verification surface; all three pins resolve
(`c1b7dffa1457f4ffe7eafb08092eb2505a9651e4`, `16777da3475084eda04813c98cfef4365d5dcb58`,
`7df8b876b91208a86a6d792e1140b3deec14535b`, `d1fcb9d934584b94789a84cde31d2ed655ab4c71`).
Every staged-addendum quote byte-verified at those pins. METHOD NOTE for future sweeps: pin
per-commit SHAs (or verify the bundle contains them) at staging time.

## The two laws (decision content)

1. `tier-hit-refreshes-lru-recency-at-owning-tier` — a hit served by a FASTER tier must
   still refresh the OWNING tier's recency: record at admission (`_pending_local_hits`,
   "Refreshed at admission: a request waiting for blocks repeats this lookup every step"),
   at the serving step touch + re-free at the MRU end ("Eager CPU is inclusive: a GPU
   prefix hit also uses the CPU copy, but eager stores skip it, so move it to the MRU end
   here"), drain on every terminal path (request_finished + reset). The corpus's
   touch/promote family is within-one-cache; this is coherence BETWEEN tiers.
2. `refusal-quarantines-entry-transient-passes` — a copy the receiving family REFUSES
   (words/size/range) is a failed check of the ENTRY: one `refused()` switch
   (SnapshotSize/BadKeptRange/DiskWords/KeptWordsDiffer — "so a family's new one is a
   single line"), refusals route to bad() (counted, logged with the file named, unlinked
   ONCE, every naming state dropped), the read-ahead entry drops AHEAD of the removal
   ("a state that is gone takes its read ahead with it" — a same-name state written later
   must not re-seed the evicted read); transient errors pass through keeping the state and
   its read ("Other errors, a failing link or memory, pass through as before").

## Dedup (3+ vocabularies per atom, at the slot)

- A: `lru|recency|owning tier|tier hit|touch` — the touch/promote family is all
  within-one-cache (`slru-probation-protected-promote-on-second-touch` entries_35,
  `cache-reuse-assertion-gated-on-eviction-counter` entries_40); zero cross-tier owner.
- D: `refus|quarantine|tombstone|negative|forget` — the cluster owns lifetime/pin/cause
  axes (B242 entries_06, B256 entries_36, kernel_opt B256; `in-use-state-retry...` entries_45
  keys retry on the cause at serve time); zero refusal-VERDICT owner, zero forget-ordering owner.
- B: `drain-device-work-before-free` (gpu_111) owns the LOCAL completion axis;
  `ipc-peer-mapping-collective-teardown-pin-on-failure` (gpu_118) the cross-process TEARDOWN
  axis; the new face is the steady-state failure-path free-pool handback gated on a per-chunk
  protocol ack + the sticky session fault — the REMOTE-writer arm.
- C: `admission-tests-the-bound...` (entries_45) owns which of two BOUNDS; the new face is
  which SIZE the test uses when a downstream guarantee scopes a quantity (the consuming size,
  never a derived rollback length that can diverge).

## Gates (all run this session)

- batch308 needles+parse: 2/2 PASS (+ the entries_45 rider needles + the gpu_111 rider
  needle on the kernel side).
- default lib: 2787/0 · rust_perf_bench: 2818/0 · kernel_opt: 3312/0 (the gpu_111 posture).
- clippy −D ×3 postures (default / rust_perf+kernel_opt / no-default): clean (13+
  doc_markdown findings in the new doc comments healed to zero).
- Retrieval floors HELD both postures: default+latent 27/0, rust_perf+latent 27/0; perf
  self-variants 92.9/94.6/98.2, blended 87.9/92.2/94.3 — above the 77.1/86.4/92.4 floors;
  no re-pin.
- freeze/score-bench NOT owed (P30 — the freeze corpus builder is the clippy_lints seed; a
  rust_perf-only delta moves nothing).
- Release-set binary rebuilt + installed at the landing HEAD (rust_perf IS release-set —
  corpus compiles into the bin; the B255 lesson).

## Count pins (the measured-truth loop)

- len-form 278→280: the line-19 pin + 4 more sites (7494/9114/9154/9213/9261) — all moved.
- annotated: measured truth 227 (225 true pre-batch + 2). The 1916/2074 pins sat STALE at
  222 (the B298/B305 lanes moved other sites and missed these — the missed-pin class),
  first moved to 224 by this batch's naive +2, then normalized to the measured 227 with the
  stale-pin repair disclosed in the message.
- NotBenchable (rust_perf_bench posture): 156→158. Benchable 122 UNCHANGED (both rules
  bench_spec None). Unannotated 53 UNCHANGED (both rules annotated).
- The kernel-side witness `batch249_slice3_rust_perf_riders`: 278→280.
- mod.rs doc: 275@B298 → 280@B308 (the B305 re-pin miss repaired en-route, disclosed).
- AGENTS.md rust_perf bullet: 278@B305 → 280@B308. README row: STALE at 266@B271 → 280@B308
  (the miss chain disclosed in the row). .docs/11_domains/rust_perf.md: Batch 308 row
  prepended + the standing count 274→280.

## Honest notes

- The fixture-D first compile had the ghost file at the family's accepted size (the wrong
  refusal class fired); fixed to a 3-word file so SnapshotSize fires — the fixture arms
  compile + run standalone (rustc --edition 2021 --test) before landing.
- The gpu_111 rider first landed OUTSIDE the string literal (a replace-anchor off-by-one
  over `".to_string(),`) — the kernel_opt posture compile caught it; repaired in-batch.
- Zero GPU throughout; worktree `E:/git/wt-b308` @ origin `93bfdbf0` (+ the rebased
  `57b7da900` docs(distill) intake commit mid-batch — zero overlap, push rejected once on
  the snapshot file, rebased clean).
