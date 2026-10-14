# Plan 238 — Batch 236: the splash GDN mid-prefill state-capture law

**Status:** COMPLETE — landed 2026-10-15 (shikuwa/4090 idle loop); all gates green — claimed 2026-10-15 ~1x:xx +07 (shikuwa/4090 idle loop, off the 2026-10-15 league watch pass triage).

Batch 236 = ONE own kernel_opt D-rule `mid-prefill-recurrent-state-capture-boundary-unsplit`, corpus 812 → 813 — the watch triage's convertible lead:

- **Source** — incoai/splash (Apache-2.0, Rust/C++), the four-commit GDN boundary-state capture series read at the pins this session via git-show on `E:/git/vendor/splash`:
  - `711f8e54b47844f090827f8c885f8acdebdc3b6b` "Capture GDN states inside a prefill's rows" (the ops::GDN half — scan dispatch runs split at capture rows)
  - `c3dcf53` "Capture boundary states inside prefill commands" (the runtime/scheduler half — stateCaptures, reserveState, the replay-point cost rows)
  - `985463c` "Reserve a cached state's buffers before the command that writes it" (reserve/copyState/seal decomposed from snapshot)
  - `79a4329` "Keep a lane's checkpoint until its successor is published" (retire-on-publish; the cancelled-prefill retry rows)
- **Convergence face folded in-rule** — vLLM #60050 MERGED `badfcdbf9` 2026-10-14 (Apache-2.0): FlashInfer GDN prefill internal checkpoints; TTFT −20%/−12%/−10% at c1/c4/c16 (GB200, Qwen3.5-35B-A3B, 8000-token prompts); the numerics contract ("each emitted state matches a separate run truncated at that position (max abs err 5e-3), and the final state is bit-identical with checkpoints on"); the 64-token grid alignment law + the split-prefill fallback + the all-layers-can admission; Mamba2 #57329 + Kimi-K3 FlashKDA #52789/#53614 named as the same mechanism — FOUR independent implementations.

## Tasks

- [x] T0: quotes pinned — every needle verbatim at git-show THIS session (the four splash commit messages + the GDN.cpp capture-loop hunk; the vLLM PR body via the API fetch).
- [x] T1: dedup adjudication — entries_gpu_08's GDN-state family (gdn-transition-replay / triangular-solve) owns speculative-verify state algebra; B219 prompt-cache-rewind-states owns KV rewind placement; THIS owns the boundary-unsplit + scan-run-splitting + reserve-before-write + retire-on-publish axis. Kinship text in-rule.
- [x] T2: author `entries_gpu_95.rs` (new slice, ONE rule; D category; pseudocode halves Rust-shaped; NotBenchable annotation — no validator for this mechanism in this corpus, all perf rows source-reported).
- [x] T3: wire `mod.rs` (exhaustive doc + D comment/list + corpus count) + `router.rs` expert-total pin if applicable.
- [x] T4: `batch236_rule_parse_and_needles_landed` in tests.rs — verbatim needles + both pseudocode halves parse; corpus == 813 asserted; prior batch count asserts re-pinned per the convention.
- [x] T5: per-category table + seed-corpus + plan-003 sums re-pins in tests.rs (D 189→190).
- [x] T6: G2 — kernel_opt lib (expect 3109/0/1i = B235's 3108 + 1) + default lib (2659 standing) — isolated `CARGO_TARGET_DIR=/tmp/riir-refine-b236`.
- [x] T7: G3 — clippy `-D warnings` clean at default AND all-features.
- [x] T8: retrieval floors — HELD expected (kernel self-variants + blended; no re-pin).
- [x] T9: freeze/score-bench — NOT owed (zero clippy-corpus delta, P30).
- [x] T10: research note 268 + `.research/.highwater` 267→268; `.plans/.highwater` 237→238 (dual_allocation_gate clean at both).
- [x] T11: docs — AGENTS.md kernel_opt count 812→813 + the B236 row; `.docs/11_domains/kernel_opt.md` batch row; README count row if stale.
- [x] T12: snapshot landing row (claim discharged) + commit `feat: batch 236: ...` + push.

## Notes

- Count discipline: +1 own rule — corpus 812→813 at every pin site; D category 189→190.
- Deferred candidates recorded in Research 268, NOT authored: the splash SSD-cache volume-management series (`13f6e11`/`a212b81`/`1af85ef` — persistence-tier laws, watch-grade); the `3903390` runtime-oracle content-vs-count law (kinship with B233's content-oracle — a future rider if a host rule fits); uzu `ce6298c` (Metal GEMM tuning sweep, M3-lane intel); the vLLM reciprocal-multiply + 2-wide quant pair (family-owned by entries_gpu_03's one-division law / quant unroll families — NULL-grade on dedup).
- Build isolation: `CARGO_TARGET_DIR=/tmp/riir-refine-b236`, removed post-verification.
