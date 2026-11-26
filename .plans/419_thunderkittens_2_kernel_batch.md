# Plan 419 — Batch 425 (nominal): the ThunderKittens 2.0 kernel batch (Research 453)

**Status:** CLAIMED — the verdict lane continues into execution with the full T0 context loaded (the clone + corpus reads + dedup evidence are this session's); a pushed superseding claim wins on fetch. Staged 2026-11-26 (Research 453, the research-source sweep lead (6)); nominal Batch 425.
**Claimed:** zed-i1186-cont@amo, 1795774500, expires 2026-11-28T16:30:00+00:00
**Claim protocol:** claim THIS plan with a `**Claimed:**` lease + a CLAIM row in `.distill/001_mining_queue_snapshot.md` BEFORE any corpus edit; push is the arbiter; expiry 48 h. Batch number re-derives at landing HEAD (422/423/424 landed 2026-11-26; 425 unless the folder moved). Yield rules per Research 453: kernel_opt ×4 (R1-R4) + rust_perf ×1 (R5) + faces F1-F3, ALL suggestion-only `NotBenchable-by-platform`/`-by-domain`.

## Sources

- Post: https://hazyresearch.stanford.edu/blog/2026-02-19-tk-2 (Sul/Ré, 2026-02-19) — the URL the pin (live-fetch posture).
- Repo: https://github.com/HazyResearch/ThunderKittens @ `67845f5fa48d05343dbbc1ba1403f11061f08d2d` (tip 2026-09-12, MIT — post-2.0). Clone `.raw/thunderkittens` (exists at verdict time; **on session resume delete + re-clone at the pin, never verdict against a checkout you did not pin**).

## Tasks

- [ ] **T0 — re-pin + quote re-verification.** `git -C .raw/thunderkittens rev-parse HEAD` == `67845f5f…` (tip-move check; re-verify EVERY quote against the new pin per the B414 re-pin discipline if it moved). The load-bearing quotes: `elect_leader` asm (`include/ops/group/util/sync.cuh:194-208`), the commented-out fence + causality law (`include/ops/thread/mma/tcgen05.cuh:808-810`), the preferred+minimum LaunchConfig (`include/common/util.cuh:530-551`), the mxfp8 merged scale-cp+MMA warp (`kernels/gemm/mxfp8_b200/mxfp8_b200_gemm.cu:196-222`), the SplitMix64 fill (`kernels/gemm/common.cuh:28-48`), the input-group convention (`bf16_b200_gemm.cu:266-271`), the setmaxnreg sites (`bf16_b200_gemm.cu:118,186`), the CLC pipeline (`bf16_b200_gemm.cu:88,117-181`).
- [ ] **T1 — dedup re-verification at landing HEAD** (3+ vocabularies per candidate; the verdict's table is the base). Named adjacents to re-read before authoring: `hardware-work-queue-steals-pending-tiles` (B417 — F1 face host), `banded-launch-order-shares-weight-tiles` (B323 — F2 face host), `setmaxnreg-fit-gate-elected-arrive` (B137 — F3 face host + the sm_89 rider), `pdl-admission-by-image-set-marker` (B257 — D1's family), `benchmark-input-distribution-realism` + `isolated-bench-cache-state-vs-live` (B408 — R5's both-directions adjudication), the fence family (`host-context-mirror-fenced-drain-avoidance` etc. — R3's adjacency sweep).
- [ ] **T2 — authoring** (the two named open calls decide HERE): (a) R2 one-rule-with-two-clauses (cluster-stranding + tmem-occupancy-1 as the shared "check the real occupancy ceiling" decision) vs two thin rules — the fixture decides; (b) R5 lands in `rust_perf/corpus/entries_106.rs` with the B408 power-vs-memory axis adjudication in-rule, or folds as a face if the authoring read finds the axis thin. R1 is the LEAD (A−): `elect-sync-single-thread-issue-serialization-loop`, Dispatch, with the `elect_leader_from_active` second surface in-rule. R3 carries the two-regime fence law + the same-thread merge instance; compose vs B272's mirror-fence rider both directions. R4 carries the unpinned-crossover disclosure. All entries carry the post-reported-magnitude provenance discipline (never a bar).
- [ ] **T3 — ORIGINAL fixtures standalone-compiled AND executed pre-entry (debug AND -O)** — Rust reproductions of the DECISIONS (the assembler-loop-vs-elected-issue model; the cluster-size SM-stranding predicate; the causality-chain fence predicate; the tmem regime predicate; the power-vs-memory axis model). No TK source pasted (license + binary-only rule). The fixture gate catches arm defects pre-entry.
- [ ] **T4 — standing gates** at isolated `CARGO_TARGET_DIR=/tmp/plan419`: lib default + rust_perf_bench + sec suite untouched-check, clippy -D ×3 postures, fmt gate, pins_manifest --write + prose agree, retrieval floors HELD expected (no re-pin unless a span moves), health_views tree total, category slices (Dispatch +N, MemoryAccess +N, measurement-honesty +1), the count web one-landing (AGENTS rust_perf + kernel_opt heads, README, .docs/11_domains).
- [ ] **T5 — score-bench/freeze NOT-owed asserts** (zero clippy_lints delta — the standing convention, asserted in the landing) + release-set binary rebuild NOT owed (suggestion-only delta — the B420 convention).
- [ ] **T6 — records one-landing**: Research 453 Status → LANDED (batch number + commit), the queue snapshot LANDING row, `feat: batch 425` push, numbering_gate + dual_allocation gate + claim lint clean.
- [ ] **T7 — teardown**: `.raw/thunderkittens` + `/tmp/plan419` removed at close (a landed batch with a live `.raw/` entry is an unfinished batch).

## Pre-registered drops (per Research 453)

D1 PDL arrive placement (positive instance, family deepest-owned) · D2 MXFP8/NVFP4 scale-feeding economics (sm_100-only, no fleet lane) · D3 chiplet NUMA (no bounded fix-space, not our silicon). A T0/T1 read that strengthens any of these re-opens it by writing the reason in-rule, never silently.

## Box-state routing

Docs/authoring CPU-light anywhere quiet; fixture EXECUTION + all cargo gates defer to a quiet window per the Plan-337 convention (amo runs the i1244 shards; bmo holds the 1241 census — `agent-mail rig status` before the gate window, the receipt quotes the rig).
