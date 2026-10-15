# Plan 240 — the P64 conversion batch (Batch 238)

**Status:** IN FLIGHT — claimed 2026-10-15 (shikuwa/4090, riir-refine idle loop; claim banner in `.distill/001` this commit; P58–P61 remain the M3's Batch 237 in-flight claim — this batch does NOT touch entries_26 / rust_perf / the B223 family)

One count-neutral rider (corpus stands 813) on `l2-grouped-tile-schedule` (B130,
`entries_popcorn.rs`) from llama.cpp #30215 @ `3e390af18` (MIT) — the
L2-budget-derived group-size face + the bandwidth-starved regime law. Quotes
re-verified at the fetched .patch + PR body this session (the patch formulas
byte-verified; the PR tables re-read). Source clone NOT needed: the single-file
26-line patch + the PR body ARE the pin (recorded in Research 269).

- [ ] T0 — claim banner + this plan committed + PUSHED before authoring (the claim protocol's push-is-the-arbiter law)
- [ ] T1 — quote re-verification at the pin: DONE at claim time (the fetched .patch `3e390af18` — GROUP_A_BUDGET, the four per-datatype `a_panel_bytes` formulas, `group_m` clamp, `lin` linearization, the MUL_MAT_ID carve-out; the PR body — op-level table, model-level table incl. the Q3_K_S small-gain row, the RTX 3090 neutral quote, the jeffbolznv review intel incl. the 8k×8k threshold + the 5090 no-gain, the declined int-div fast-path measured null). Record verbatim needles in the Research note
- [ ] T2 — dedup grep: the three faces NEW vs B130's existing cloudrift rider (SWIZ=8 fixed → THIS owns the derivation axis; the merged-axis linearization = the THIRD formula shape; the MUL_MAT_ID carve-out = the applicability boundary cross-refing entries_gpu_39's expert-major grid)
- [ ] T3 — author the rider: append the Batch-238 RIDER clause to B130's `gpu_concept` (entries_popcorn.rs); no count change; opt_pattern untouched (the existing group_size-parameterized shape already carries the law's form — the rider owns the DERIVATION of that parameter)
- [ ] T4 — gates: G1 `batch238_rider_parse_and_needles_landed` needle gate + B130 both pseudocode halves still parse; G2 `cargo test --features kernel_opt --lib` domain suite green; G3 clippy `-D warnings` at default + kernel_opt postures; retrieval floors HOLD (kernel class, ~1 min, isolated `CARGO_TARGET_DIR`); freeze/score-bench NOT owed (zero clippy-corpus delta — the P30 premise, verified at T4)
- [ ] T5 — docs: Research 269 (born-complete distillation record, Status: DISTILLED — pending owner decision is N/A for count-neutral riders → RECORD); AGENTS.md kernel_opt bullet + the batch lineage sentence (813 stands); `.distill/001`: P64 tick `[x]` + the Batch-238 landing banner + the du-table cycle row (refine/target 41,066 MB new; 4 walkers still running: ai/train/katgpt targets + vendor; the duplicate 11:30 walker wave killed — the originals never died)
- [ ] T6 — commit `feat: batch 238: ...` + push (rebase non-interactive over any M3 slice landing mid-flight); plan status → DONE with landing hash
