# 086 — llama.cpp v0.4.0 delta re-mine (Batch 113)

**Status:** QUEUED — verdict landed (Research 137, YES B+); batch not started

- **Source:** ggml-org/llama.cpp v0.4.0 @ `5266f24da75dc449bd56cbed7addb9c8e4a6a73e` (MIT; tag object `5ac847190e979e0da7c4a21806630805f396d487`). Shallow-clone read 2026-09-05, removed per the .raw discipline.
- **Verdict:** Research 137 — YES B+, route `kernel_opt`. Expected: 2–4 rules + ~12–15 riders + 1 league flag.
- **Prior contact:** B104 (Research 126 — fa-vec infra, pins INSIDE this window — fa-vec is NOT re-mined), B96 (Research 114 — PrismML fork).

## Tasks

- [ ] T1 Re-clone at pin `5266f24da75d`; re-verify every graded candidate byte-accurate (ssm.metal `kernel_ssm_scan_ssd_mma_f32` :300-473, ops.cpp `use_mma` gate ~L1790-1830, utils.glsl `fastdiv` :1-37, fa.metal `n_kv_max` :1077-1176, ops.cpp `top_k_radix` gate :5249-5422) + read the rider commits' diffs before writing any rider
- [ ] T2 Dedup re-verification under BOTH vocabularies at mine time (Research 137's greps are the record); declare adjacencies in every new entry (rule 1/2: B79 strategy ladder + B95 comparator ethos + B110 Gram reuse + B46 eligibility + B57 barriers; rule 3: rust_perf B59 + kernel_opt `strength-reduce-div-pow2`; rule 4: B107/B97)
- [ ] T3 Author Rule 1 `chunk-transition-consumed-by-mma` (category G — recurrence-strategy ladder, chunk family's MMA member; fold in the per-chunk Gram reuse + the `mma_tokens`/sequential-tail dispatch + `snap_reserve` interop note) — new slice `corpus/entries_gpu_24.rs` + mod.rs wiring
- [ ] T4 Author Rule 2 `carried-state-reduction-stays-scalar` (category D — tolerance-asymmetry partition; the source comment is the contract; distinct-from B95/B79/exact-op-parity)
- [ ] T5 Author Rule 3 `shader-fastdiv-mulhi` (category N — umulExtended mulhi + round-up `>>L` + `fastdiv_L` u32 L-packing + `fastdiv_small` quotient-zero branch-skip; regime: coordinate decomposition in shaders, general divisors, quotient-0-heavy leading divisions)
- [ ] T6 Rule 4 sparse-attention-kv-budget — decide RULE-vs-RIDER after the T1 fa.metal read (B- as rule: sentinel `-1`-padded index-list attention + clamp-on-hint-violation; else rider on B107/B97)
- [ ] T7 Rider sweep — map the confirmations into existing rules' name-fields: B92 (xor-swizzle FA), B98 (+#27621 specdec regime note), B46 (use_mma five-clause), B57 (s_buff handoff barriers), B110/B105 (tail fallback + capability route), B76 (lazy-mode), B105 (view-alias deps + Metal4 gating + f16-KV in-place bind), B77/B104 (crossover + Strix Halo rows), B110 (SYCL Q4_K multi-col), B65 (row-slab stream), B71/B58 (qwen4exp rollback + snap_reserve), B22/B19 (DFlash fuse + non-fused GDN/LID disabled), B23/B38 (mm_ids_helper + KleidiAI init), B107 (metal top-k radix + fixed-cost gate). Regenerate the FULL-STRING pins of every ridden rule (the B111 precedent — older gates pin the whole concept string)
- [ ] T8 `1f3d318` Kronecker FWHT: read the REVERT reason (#28184) at T1; record as regime evidence in the note or a B96 rider — never as a rule (upstream reverted it)
- [ ] T9 Standing gates: `batch113_rules_present` + `batch113_pseudocode_parses` (cfg(not(kernel_verify)) per B70) + concepts coverage; fixtures ORIGINAL (no verbatim source)
- [ ] T10 Count pins walk: corpus total, category expected-array, sums ×2, grouped, router total, mod.rs doc line + category comments + member-list tails; new-slice mod/extend wiring
- [ ] T11 GOAT: kernel_heal lib + default lib + kernel_heal+latent floors HELD zero re-pins + clippy 0 all states + retrieval_precision_regression_gate; count-identical default surface
- [ ] T12 League flag: record upstream fa-vec M3 Max completion (`8f83678` #28373 at v0.4.0) in `.docs/09_performance/perf_rematch_league.md` opponent-watch — the fork pin `e311ed38f` lacks fa-vec; a future re-pin to upstream ≥ v0.4.0 moves the opponent's M3 prefill cells
- [ ] T13 Docs/AGENTS/queue-snapshot updates (corpus counts, Last-landed line); commit+push; rm `.raw/llama-cpp-v040` if re-cloned
