# Plan 240 — the P64 conversion batch (Batch 238)

**Status:** COMPLETE — landed 2026-10-15 (shikuwa/4090 idle loop); all gates green — claim `d38a3f16` pushed plan-first per protocol (push accepted first try).

One count-neutral rider (corpus stands 813) on `l2-grouped-tile-schedule` (B130,
`entries_popcorn.rs`) from llama.cpp #30215 @ `3e390af18` (MIT) — the
L2-budget-derived group-size face + the bandwidth-starved regime law. Quotes
re-verified at the fetched .patch + PR body this session (the patch formulas
byte-verified; the PR tables re-read). Source clone NOT needed: the single-file
26-line patch + the PR body ARE the pin (recorded in Research 269).

- [x] T0 — claim banner + this plan committed + PUSHED before authoring (the claim protocol's push-is-the-arbiter law) — `d38a3f16`
- [x] T1 — quote re-verification at the pin: DONE at claim time (the fetched .patch `3e390af18` — GROUP_A_BUDGET, the four per-datatype `a_panel_bytes` formulas, `group_m` clamp, `lin` linearization, the MUL_MAT_ID carve-out; the PR body — op-level table, model-level table incl. the Q3_K_S small-gain row, the RTX 3090 neutral quote, the jeffbolznv review intel incl. the 8k×8k threshold + the 5090 no-gain, the declined int-div fast-path measured null). Record verbatim needles in the Research note — 18 needles in the G1 gate, all verbatim
- [x] T2 — dedup grep: the three faces NEW vs B130's existing cloudrift rider (SWIZ=8 fixed → THIS owns the derivation axis; the merged-axis linearization = the THIRD formula shape; the MUL_MAT_ID carve-out = the applicability boundary cross-refing entries_gpu_39's expert-major grid) — vocabulary grep `GROUP_A_BUDGET|a_panel_bytes|L2-friendly|workgroup scheduling` zero corpus hits pre-edit
- [x] T3 — author the rider: append the Batch-238 RIDER clause to B130's `gpu_concept` (entries_popcorn.rs); no count change; opt_pattern untouched (the existing group_size-parameterized shape already carries the law's form — the rider owns the DERIVATION of that parameter)
- [x] T4 — gates: G1 `batch238_rider_parse_and_needles_landed` PASS (18/18 + both halves parse); G2 `cargo test --features kernel_opt --lib` 3117/0; G3 clippy `-D warnings` default + kernel_opt postures exit 0/0; retrieval 21/21 HELD (228.65 s; floors 91.2/94.7/95.3 unmoved); freeze/score-bench NOT owed (P30 premise verified — zero clippy-corpus delta)
- [x] T5 — docs: Research 269 written (RECORD, born-complete); AGENTS.md kernel_opt bullet carries the batch lineage (813 stands); `.distill/001`: P64 tick `[x]` + the Batch-238 landing banner incl. the du-table cycle row (refine/target 41,066 MB new; 4 walkers still running; duplicate 11:30 wave killed — the 10:05 originals never died)
- [x] T6 — commit `feat: batch 238: ...` + push (rebase non-interactive over any M3 slice landing mid-flight — none did); plan status → DONE in the landing commit (this file)
