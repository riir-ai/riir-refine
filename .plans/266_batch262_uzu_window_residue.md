# Plan 266 — Batch 262: the uzu radix-top-k window residue

**Status:** COMPLETE 2026-10-20 ~19:0x +07 — T1 adjudicated RIDER (the PR body carried the full mechanism; the in-source WHY hypothesis confirmed and exceeded), T2 landed (the count-neutral rider on the gpu_101 host), T3 needle gate FIRST-RUN PASS, T4 clippy ×2 postures clean + libs green (kernel_opt 3191/0, default 2715/0), T5 floors HELD / freeze/score-bench NOT owed / rebuild NOT owed, T6 docs landed (Research 298, snapshot row, this plan). **Claimed:** shikuwa/4090 idle loop, 2026-10-20T18:20+07 (epoch 1792658400), expires 2026-10-22T18:20+07 — discharged by the landing.

## Why now

The walk this session (`riir-ai/.benchmarks/rematch/2026-10-20_watch.md`, since
2026-10-19) re-armed the standing watch and surfaced the ONE unmined source
window: **trymirai/uzu `e744e7c..2b4c1ae`** — 5 commits, of which exactly one
carries mineable decision content. B260 already mined the
`56dbb91e` race-fix kernel (the arrival-merge rider on the B223 host); the
envelope pin moved to `2b4c1ae4` at B261; the RESIDUE between the two lanes'
coverage is this window's tail:

| commit | verdict | why |
|---|---|---|
| `56dbb91` (#991 race fix) | **MINED** (B260) | the arrival-merge + scope-widening rider, landed on `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` |
| `e6399cb` (#988 residency-set removal, 1 line) | **ADJUDICATE** | a Metal 4 sparse-queue residency-set REMOVAL — candidate count-neutral rider on the gpu_101 `residency-set` budgeting family (holding-bound law) — OR a NULL (context-internal rework, no transferable law) |
| `af697cb` (#977 unify workspace + deps) | screens NULL | chores |
| `2fa2d06` (#992 sysinfo ios workaround) | screens NULL | infra |
| `0b5f991` (#993 xgrammar-rs rewrite + simulator stubs) | screens NULL | grammar substrate swap, no kernel/host decision content |
| `2b4c1ae` (Release 0.6.10) | screens NULL | release tag |

The working hypothesis for the ONE live atom: `e6399cb` removes
`sparse_queue.add_residency_set(&residency_set)` from the Metal 4 context —
if the commit/PR body or surrounding code records WHY (a residency-set
incompatibility with MTL4CommandQueue, a leak, a perf regression, an OS
behavior), it lands as a count-negative-scope rider face on the residency
family (the family that owns WHEN buffers join residency sets); if the
removal is unmotivated context plumbing, the window is fully consumed at
B260 and this batch records the sweep as a NULL close (the B209/B210
thin-day precedent allows an honest empty).

## Scope

- T1: read `e6399cb` in full (1-line diff) + the surrounding `context.rs`
  residency wiring at BOTH pins (`e744e7c` and `2b4c1ae`) + the PR #988
  body if fetchable. Adjudicate: rider-grade face vs NULL. Honest default
  when no WHY exists in-source: NULL (never invent a law).
- T2 (conditional on T1 = rider): land the count-neutral rider on the
  gpu_101 residency host (or its named kin) — quotes verbatim at the pin,
  batch-262 tag, no count pins move.
- T2' (conditional on T1 = NULL): record the window close in
  `.research/.highwater`-adjacent notes (Research 298, one page) + the
  snapshot row; no corpus edit.
- T3: needle gate (`batch262_*` test in corpus/tests.rs either way — a
  rider asserts its needles; a NULL asserts the window's commits screened
  with reasons, the negative-control convention).
- T4: clippy `-D` (default + kernel_opt + no-default postures,
  `--all-targets`), kernel_opt lib + default lib green.
- T5: retrieval floors re-measure ONLY if the corpus text changed (T2);
  freeze/score-bench NOT owed (zero clippy-corpus delta either way);
  binary rebuild NOT owed (kernel_opt outside the release set).
- T6: docs — AGENTS.md batch history row (kernel_opt bullet, one line),
  snapshot landing row, Research 298 (the window adjudication record),
  `.research/.highwater` 297→298.

## Gates

- Every quote verbatim at the pin (the vendor clone `E:/git/vendor/uzu` at
  `2b4c1ae`, fetched this session — pin drift re-checked at write time).
- No count pins move unless a rule is added (not expected).
- Zero GPU throughout (StartLux resident to ~13:30Z; this batch is
  CPU-only by construction).

## Queue context

Unchanged from the snapshot: the bout runbook (`.distill/003`) waits on the
verified-idle 4090; walk #16 ≥04:00Z Wed 10-21; dated leads unchanged.
