# Plan 276 — Batch 272: dphnAI/sonar PR #1788 paired-PCIe-all-reduce distill conversion

**Status:** CLAIMED — zed-m3-idle-b272, 2026-10-24, expires 2026-10-26T12:00:00+00:00 (48 h; mining slot claimed per T12 claim-first; verdict Research 308 stands)

Source: dphnAI/sonar PR #1788 commit `927f3262f87936e518b602538f2fe24a25359675` (Apache-2.0, PR-stage OPEN) — paired PCIe all-reduce, SM120 TP4. Verdict: YES B− corpus / lane-intel N/A / product-coverage MODERATE (Research 308). Target: `kernel_opt` categories D/E — 2–3 rules + 3–4 riders, all NotBenchable-annotated (multi-GPU domain reason, the dial9 split).

## Tasks

- [ ] T0 — Re-pin at mine time: `git fetch origin pull/1788/head` in `.raw/b272/sonar` (ephemeral, removed at close), resolve the head sha, verify `927f326` is an ancestor (upstream may have rebased — a moved commit re-reads the diff, quotes re-grade), record the full sha + PR head state in the batch landing row.
- [ ] T1 — Quote re-verification: every Research 308 quote byte-checked against the pinned tree (`pcie_pull_all_reduce.py`, `pcie_pull_all_reduce.cu`, `cuda_communicator.py`, docs page); the PR-stage B193 law applies — quotes against an unpinned checkout are worthless.
- [ ] T2 — Dedup re-verification (3+ vocabulary pairs per candidate, BOTH corpora): fail-stop/numerics-fallback/selector-agreement; ipc/teardown/pin; relaxed/acquire/confirm/two-tier; parity/epoch/aba. Record one evidence line per pattern; the R1–R4 riders re-adjudicate their hosts at authoring (R3 home: B137 vs tma-single-issuer).
- [ ] T3 — Author rule 1 `faulted-collective-fail-stop-no-numerics-fallback` (D): original Rust fixture (two reduce backends with different rounding contracts + a selector + a terminal fault path); quotes from the pinned tree; NOT a transcription.
- [ ] T4 — Author rule 2 `ipc-peer-mapping-collective-teardown-pin-on-failure` (E): fixture = the ordered close→barrier→dispose→free protocol + pin-on-any-failure + destructor refusal.
- [ ] T5 — Author rule 3 `relaxed-spin-acquire-confirm-metadata-flag-wait` (E) IF it survives T2/T4 grading honestly (the B−/C+ borderline); a 2-rule batch is a fine outcome (B119 precedent).
- [ ] T6 — Riders (count-neutral): R1 parity-bank face on `ll-flag-in-data-packed-poll`; R3 proxy-fence mirror face (home adjudicated); R4 policy-agreement face on `tp-checkpoint-byte-agreement-hash-while-loading`; R2 rank-order provenance line only if an edit is warranted.
- [ ] T7 — NotBenchable annotations on every own rule (`bench_spec: None`, the multi-GPU domain reason named in-rule); magnitude carried source-reported only.
- [ ] T8 — Standing gates: batch272 needle + parse gates; kernel_opt lib; default lib; clippy −D ×3 postures; retrieval floors re-measure (isolated CARGO_TARGET_DIR where siblings are building); freeze/score-bench NOT owed (zero clippy-corpus delta, P30) unless a clippy_lints rule sneaks in; binary rebuild NOT owed (kernel_opt outside the release set).
- [ ] T9 — Count pins: entries_gpu_1NN registration + KERNEL_CATEGORIES slice + the pinned totals per the builder's domain (the B270 conflation lesson — read the corpus BUILDER, not the prose).
- [ ] T10 — Research note cross-ref (308 gains the landing addendum), queue snapshot landing line, `.raw/b272` removed at close (cleanup is part of the batch).
- [ ] T11 — Docs updates in the same landing commit: AGENTS.md kernel_opt bullet count, README feature list, `.docs/11_domains/kernel_opt.md` ledger — only if counts move.
- [ ] T12 — Commit + push on `develop` (`feat:` prefix), claim-first: append the Claimed line to this plan BEFORE the first corpus edit, push, then author.

## Drops carried from the verdict (do NOT re-litigate without new evidence)

- Paired-topology + BF16-pair packing (textbook recursive halving; no bounded rule shape).
- Output-gate placement — LIVE upstream bug (Codex finding); watch row only; re-open if upstream lands the fix.
- ftz/fmad per-file scoping (B03 positive instance); pynvml tuple fix (trivial).
