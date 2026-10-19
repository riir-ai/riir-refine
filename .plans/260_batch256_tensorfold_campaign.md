# Plan 260 — Batch 256: the TensorFold `9ccf4727`→`d1018b31` staged campaign

**Status:** ACTIVE — claimed 2026-10-19 ~21:0x +07 (shikuwa/4090 idle loop), expires 2026-10-21 ~21:00 +07. Claim row in `.distill/001_mining_queue_snapshot.md` (push = arbiter).

Batch 255's staged record: the TensorFold pool moved `9ccf4727` (the B249-era pin, fully converted) → `d1018b31` (+988 commits, the 1.1.0/1.1.1 waves). Named leads to re-verify AT THE PIN first: `018e73c6` (digest-cache) + `3c0d6d3c` (GLM prefix-move-before-evict). The window is campaign-sized — enumerate with parallel read agents, split kernel vs host, then convert ONE batch (or split into sub-slices under this one plan, the B249 precedent).

## Tasks

- [ ] T0 — claim posted + this plan allocated (highwater 259→260, dual_allocation_gate clean at allocation)
- [ ] T1 — clone `ashhart/tensorfold` at pin `d1018b31` into `.raw/tensorfold-b256` (ephemeral; rev-parse EXACT); verify the two named leads exist at the pin + read their full diffs; quote-level re-verification
- [ ] T2 — enumerate the +988 window: parallel read agents over disjoint slices (kernel/quant/attention vs host/serving/cache per the B249 D-A/D-B split); per-commit log + full-diff reads on perf-marked commits only
- [ ] T3 — dedup greps per candidate (3+ vocabulary variants) at HEAD corpus; false positives adjudicated; zero-owner confirmation per rule
- [ ] T4 — author entries: `entries_gpu_110.rs` (kernel_opt) + `entries_36.rs` (rust_perf) as the nominal homes (re-home at authoring if category demands); every quote verbatim at the pin, in-rule distinct-from sets
- [ ] T5 — gates: batch256 parse + needle gates (per entry file); default lib; feature-posture lib(s) per domain touched; clippy −D × all standing postures (--all-targets each)
- [ ] T6 — retrieval floors re-measure (isolated CARGO_TARGET_DIR) — floors HELD or re-pinned with attribution
- [ ] T7 — freeze/score-bench: NOT owed if zero clippy-corpus delta (P30); score-bench OWED if rust_perf corpus moved; record the adjudication either way
- [ ] T8 — maintained binary rebuild + deploy IF rust_perf/rules touched healer-serving corpus (the B255 lesson: rust_perf IS in the release set)
- [ ] T9 — docs: AGENTS.md domain bullets + counts, README domain rows, `.docs/11_domains/*.md`, Research 291 (`.research/.highwater` 290→291), snapshot landing row (same commit)
- [ ] T10 — commit `feat: batch 256: ...` + push; remove `.raw/tensorfold-b256` at close (the ephemeral-clone law)

## Notes

- Owner-gated lanes NOT in scope: L8 league-posture A/B (StartLux resident, claim to 2026-10-20T13:30Z, riir-train issue 635); walk #15 (M3, ~10-20 claim-first); ai-1017 (~10-21); mlxfast (~10-24); nnethercote (~11-01).
- Box state: E: at 67G free (97% used) — flagged owner-side (`.distill/002_owner_pending_decisions.md`); builds here are modest (corpus tests only), no GPU owed (CPU-only batch, StartLux stack resident).
- If the window enumeration yields < 3 solid candidates, convert what verified and close thin honestly (the B254/B255 thin-day precedent) — never invent yield.
