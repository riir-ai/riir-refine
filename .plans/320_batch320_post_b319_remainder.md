# Plan 320 — Batch 320: the post-B319 staged remainder (vllm pool + cubecl faces)

**Status:** CLAIMED 2026-10-30T10:3xZ (claim row at the snapshot head, commit pushed BEFORE authoring; expiry 2026-11-01T10:3xZ). kernel_opt-only per the B293 precedent — rust_perf/sec rule counts ZERO behind the live foreign claims (plan-306 to 2026-10-30T13:41Z, plan-310 to 2026-10-31T14:21Z). Executes the post-B319 staged remainder named in the B319 landing row (`.distill/001_mining_queue_snapshot.md` L1). Plan-337 posture: gates under the i1117 128k leg (isolated CARGO_TARGET_DIR=/tmp/b320_target + capped jobs); NO unverified corpus reaches develop. En-route disclosure: `.plans/.highwater` was STALE at 318 through plan 319's whole life (the B319 claim commit `62abe061` never bumped it — the same 4090-lane miss class B310 repaired 313→315); repaired 318→320 at this allocation with this note.

- [ ] T0 clones + grading: vllm blobless clone — fresh-grade #64494 (PDL), #64467 (MoE tiles), #64553 (Muse routing); re-confirm the C6/C8 watch rows #59131/#60103/#64378/#62009 stand consumed; llama.cpp upstream — read #31073/#31368 (B319 adjudicated host-side/not-kernel_opt; re-check for kernel content, else stage-for-rust_perf note only); cubecl c1d0125 — verify CR4 (#1976 e2m1 byte-conversion face) + CR5 (#1977 block-scaled-operand-form face) quotes at the squash body AND diff; grade #1986 (sync_plane_storage barrier semantics) + #1967 (read_lazy) fresh.
- [ ] T0b fresh window check 10-30T08:31Z..now across the watch set (llamacpp, vllm, mlx, bevy, cubecl, wgpu, uzu, splash, lithos, tensorfold, advisory-db) — new leads join this batch's pool if kernel_opt-grade, else staged for the next sweep row.
- [ ] T1 dedup greps (3+ vocabularies per candidate atom) at origin HEAD.
- [ ] T2 author corpus entries (own rules + count-neutral rider faces as graded; honest drops allowed; B-tier adjudication at authoring).
- [ ] T3 fixtures standalone-compiled AND executed BEFORE entry (rustc --edition 2021 --test).
- [ ] T4 gates: batch320 needles+parse; kernel_opt lib; default lib; rust_perf_bench lib; clippy −D ×4 postures; retrieval floors (kernel_opt+latent_retrieval); Layer 5b2 default-posture retrieval (`cargo test --lib --features latent_retrieval retrieval_`).
- [ ] T5 count pins: 1033→N (whole-array convention, every site) + per-category expected table + router expert-total.
- [ ] T6 docs: AGENTS kernel_opt bullet + README row + `.docs/11_domains/kernel_opt.md` batch line.
- [ ] T7 research note (next free `.research` number, re-read highwater at write time) + dual_allocation_gate at allocation.
- [ ] T8 snapshot landing row + queue-after.
- [ ] T9 commit `feat: batch 320: ...` + push (push is the arbiter). Freeze/score-bench NOT owed (P30, kernel_opt-only); binary rebuild NOT owed (kernel_opt outside the release set); `.raw/b320` removed at close.

Scope notes: llama.cpp #31073/#31368 carry NO corpus content in this batch unless T0 finds kernel-shaped decision content B319's host-side adjudication missed. The serving/spec-dec tier stays untouched (S8/S9-remnant per the B319 adjudication). #1986/#1967 stay watch/lane-intel unless T0 grades a second-instance/own-rule case.
