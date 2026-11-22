# Batch 400 — the mvm (Magni VM) distill → kernel_opt corpus

**Status:** IN FLIGHT
**Claimed:** zed-m3-mvm-b400, 1795381511, expires 2026-11-25T21:05Z

Executing the staged mvm distill verdict (snapshot row 2026-11-21, session
zed-m3-mvm-distill; controlling record: `../riir-infer/.research/36_*.md`, PIN
`60c42deb5628480d62a50d53296e5f8be77e9030`). Source: github.com/Bobliuuu/mvm —
Magni Community License 1.0, source-available free ≤$1M/research tiers:
**transcription-only GO, outbound-PR FAIL-CLOSED** (per-company revenue-gated
license — every row decision-content).

Queue-head claim per the snapshot's standing rows: this batch discharges the
mvm staged row; pool slices (1) mlxfast-bonsai2 re-visit and (3) the vendored
ml-explore/mlx tree stay unclaimed after it.

- [ ] T0 — re-clone Bobliuuu/mvm (shallow, single-branch) into `.raw/mvm`;
      re-pin at the tip (expected MOVED past `60c42deb` — fast-moving young
      repo caveat); two-pin diff `60c42deb..NEW` recorded; re-verify EVERY
      staged quote at the NEW pin (FRAMEWORK.md snapshot-isolation law,
      mvm_exec.cuh park/wake + Dekker closure, scheduler result files: the
      Doom-frame FIFO 100–300 ms/47k-task record, ready-ring 107-poller/9-of-106
      cells, dynamic-vs-static p99 1.7–1.8× / overload 8–11× cells, 184/184
      coherence-exact). Anchors re-derived by `grep -n` (never trust the old
      line numbers). Quote vintage per row (README Nov-17 rows vs tip).
- [ ] T1 — dedup re-verification at the live corpus (3+ vocabulary variants
      per atom, hits read and ruled in-row): R1 adjudication set = kernel_opt
      gpu_138 `page-reuse-fenced-behind-reader-completion` +
      gpu_85 `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` +
      rust_perf entries_86 `closed-channel-select-arm-gate` (+ A2 epoch-tag
      echo — cross-domain, recorded never landed); R2 = gpu_50
      `split-submission-in-flight-cap-overlap` + rust_perf `singleflight-*`;
      R3 host = rust_perf entries_74 `writeclass-share-partition-inflight-writes`;
      R4 = gpu_148 `peer-mailbox-rotation-*` + `singleflight`; R5 =
      measurement-honesty family (wall-clock / spread-unstable neighbors).
- [ ] T2 — authoring: R1 `version-snapshot-reader-before-next-publisher`
      (the LEAD — kernel_opt, suggestion-only NotBenchable-by-domain,
      bench_spec None; the reader→publisher-of-N+1 edge + the three compile
      errors as the regime shape); R2 `parked-waiters-release-off-worker`
      face-or-thin decision AT AUTHORING (pre-registered: land only if the
      park-off-worker admission axis survives the gpu_50 adjudication);
      R3 = count-neutral FACE on rust_perf entries_74 (the mvm measured
      instance: single FIFO put Doom frames behind 100–300 ms of a 47k-task
      record, per-class budgets shed 38% of serve); R4/R5 drop-or-face
      decisions at authoring (thin rows — honest C+ drops recorded, never
      padded). Riders recorded in the research note never landed:
      MVM_MIN_BLOCKS=2 register-cap (+40–60% matmul, gpu_44/R30-lead-6 face
      opportunity noted), ~0.7M tasks/s dispatch ceiling (gpu_07), decode
      bandwidth-bound corroboration (league-side, riir-ai R402 lineage),
      gpu_86 static-ICB-replay + gpu_60 deadline-bounded-poll as
      static/dynamic boundary neighbors.
- [ ] T3 — fixtures: kernel_opt rule fixtures standalone-compiled AND
      executed BEFORE entry (debug AND -O, zero warnings); the entries_74
      face arm in the rust_perf corpus tests (count-neutral).
- [ ] T4 — gates: lib tests (default posture, batch-400 needles green);
      clippy `-D warnings` ×3 postures (default / kernel_opt / rust_perf);
      retrieval floors BOTH lanes — DEFAULT `cargo test --lib --features
      latent_retrieval retrieval_` + the kernel,latent gate — decay FAILS
      the batch; fmt gate; freeze/score-bench NOT owed (zero clippy_lints
      delta asserted, P30).
- [ ] T5 — count pins one-landing: `scripts/pins_manifest.py --write`
      (kernel_opt 1105→N, the annotated count per the face rows) + AGENTS.md
      §Role kernel_opt head + the Latest chain + README + the
      `.docs/11_domains/kernel_opt.md` head + health_views totals + the
      kernel-side rust_perf witness stays (entries_74 face is count-neutral;
      its narrative head gains the face row) + rust_perf mod.rs narrative.
- [ ] T6 — research note `.research/435_mvm_distill_batch.md` (the batch
      record; the 11-21 verdict row + `../riir-infer/.research/36_*.md` are
      the controlling records; license + transcription-only posture
      restated; drops/positive-instances carried).
- [ ] T7 — teardown: `.raw/mvm` REMOVED at close (df delta in the summary).
- [ ] T8 — landing: commit `feat: batch 400: ...` + push; snapshot LANDED
      row in the same commit; `py ../katgpt-rs/scripts/dual_allocation_gate.py`
      + `py ../katgpt-rs/scripts/numbering_gate.py .` clean; binary rebuild
      NOT owed (corpus-only delta — no rulewright source touched, disclosed).

Standing gates this batch does NOT owe (pre-registered): score-bench /
freeze-table regeneration (zero clippy_lints delta); outbound-PR lane (license
fail-closed); GOAT Pareto bench (NotBenchable-by-domain — the CUDA host we do
not run; the Batch-136 annotation class).
