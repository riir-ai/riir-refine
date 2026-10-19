# Plan 260 — Batch 256: the TensorFold `9ccf4727`→`d1018b31` staged campaign

**Status:** COMPLETE — landed 2026-10-19 ~2x:xx +07 (shikuwa/4090 idle loop). 7 rules (4 kernel_opt + 3 rust_perf); all gates green; see the snapshot landing row for the commit hash.

Batch 255's staged record: the TensorFold pool moved `9ccf4727` (the B249-era pin, fully converted) → `d1018b31` (+988 commits, the 1.1.0/1.1.1 waves). Named leads to re-verify AT THE PIN first: `018e73c6` (digest-cache) + `3c0d6d3c` (GLM prefix-move-before-evict). The window is campaign-sized — enumerate with parallel read agents, split kernel vs host, then convert ONE batch (or split into sub-slices under this one plan, the B249 precedent).

## Tasks

- [x] T0 — claim posted + this plan allocated (highwater 259→260, dual_allocation_gate clean at allocation)
- [x] T1 — clone `ashhart/tensorfold` at pin `d1018b31` into `.raw/tensorfold-b256` (ephemeral; rev-parse EXACT — the pin == origin/main tip, the pool did NOT move past the B255 staging pin); both named leads verified at the pin + full diffs read; quote-level re-verification DONE (zero drift; the digest-cache corrupt-index semantics CORRECTED at authoring — loud refusal, not silent recompute)
- [x] T2 — window enumerated: 3 parallel read agents over disjoint slices (kernels 252 commits / host-core 433 / families 425); 5 A− + ~10 B+ found; 2 independent discoveries of the strongest kernel lead (c05ce72c — agents A and C converged)
- [x] T3 — dedup greps per candidate (3+ vocabulary variants) at HEAD corpus; zero-owner verdicts recorded in-rule; the weight-STREAM amortization axis confirmed FIRST-OWNER (the Yukon-conversion drop note cited in-rule)
- [x] T4 — authored: `entries_gpu_110.rs` (4 kernel rules F/A/D/G) + `entries_36.rs` (3 rust_perf rules caching/allocation/concurrency); every quote verbatim at the pin; fixtures throwaway-crate proven 25/25 + 18/18 pre-landing
- [x] T5 — gates: batch256 parse + needle gates 3/3 (one needle fix); default lib 2700/0; kernel_opt lib 3171/0; rust_perf_bench lib 2731/0; clippy −D ×4 postures (7 doc_markdown healed via cargo refine)
- [x] T6 — retrieval floors re-measured: HELD no re-pin (blended 90.6/94.2/95.3, kernel 100%, 21/21 tests)
- [x] T7 — freeze NOT owed (zero clippy-corpus delta); score-bench #191 appended (heal 0.9636 FLAT, parse_safety 1.0) — rust_perf corpus moved so the bench WAS owed
- [x] T8 — maintained binary rebuild + deploy (rust_perf in the release set — the B255 lesson)
- [x] T9 — docs: AGENTS.md ×2 bullets + counts, README rows ×2 (kernel row repaired from stale-885 with honest label), `.docs/11_domains/{rust_perf,kernel_opt}.md`, Research 291 (highwater 290→291), snapshot landing row
- [x] T10 — commit `feat: batch 256: ...` + push; `.raw/tensorfold-b256` removed at close (the ephemeral-clone law)

## Notes

- Owner-gated lanes NOT in scope: L8 league-posture A/B (StartLux resident, claim to 2026-10-20T13:30Z, riir-train issue 635); walk #15 (M3, ~10-20 claim-first); ai-1017 (~10-21); mlxfast (~10-24); nnethercote (~11-01).
- Box state: E: at 67G free (97% used) — flagged owner-side (`.distill/002_owner_pending_decisions.md`); builds here are modest (corpus tests only), no GPU owed (CPU-only batch, StartLux stack resident).
- If the window enumeration yields < 3 solid candidates, convert what verified and close thin honestly (the B254/B255 thin-day precedent) — never invent yield.
