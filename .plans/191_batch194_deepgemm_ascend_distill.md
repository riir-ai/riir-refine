# Plan 191 — Batch 194: DeepGEMM-Ascend distill (kernel_opt)

**Status:** IN PROGRESS (verdict staged 2026-09-30; conversion at the next cargo-calm slot, one-batch-per-day behind the standing queue)

Source: `deepseek-ai/DeepGEMM-Ascend` @ `8491bbb4b8c02a094a2318965f50c70438a3e73c` (MIT). Verdict: [.research/226](../.research/226_deepgemm_ascend_distill_verdict.md) — YES B+ / 10 KEEP candidates (2 A−) / 5 drops.

## Tasks

- [ ] T0 — re-clone at the pin into `.raw/deepgemm-ascend` (ephemeral law); COMPLETE THE UNREAD READ: `heuristics.hpp` 300-722 (candidate enumeration + MQA/MegaMoE heuristics), `mega_moe.hpp` full pipeline body, `mqa_logits.hpp` body, `fp8_dequant_gemm.hpp`, `bf16_gemm.hpp`, `comm/mega_moe.hpp` (HCCL), `simd/mega_moe.hpp`, `transform_sf.hpp` rest, `ascend/copy.hpp` rest. Re-verify EVERY load-bearing quote VERBATIM at the pin (the Research-226 line numbers are the map). New leads found in the unread set get graded + dedup-gated before admission (candidates may grow; drops may grow).
- [ ] T1 — dedup re-verification: re-run both-vocabulary rule-id greps for every KEEP against the CURRENT corpus (703 + whatever lands first); read each kinship hit before calling duplicate. Confirm zero `deepgemm`-derived rows still.
- [ ] T2 — authoring, 2 subagent lanes over disjoint slices + coordinator quote re-verification (the B193 method): (a) GEMM-core lane: candidates 1 (l1-resident-operand-reuse-address-identity-rewind), 3 (l0-operand-pin-eviction-aware-reload), 4 (decoupled-sf-sidecar-pipeline-own-granularity), 6 (align-constrained-dma-head-scratch-staging), 7 (measured-split-cost-load-balance) + monotone-cursor + odd-stride-bank riders; (b) fused-MoE/heuristics lane: candidate 2 (analytic-config-cost-sector-wave-model), 5 (moe-fused-ep-kernel-progress-state-machine), 8 (l1-region-overlay) + packed-arrival-value-counter + identity-epilogue + UE8M0-validation riders. Vocabulary translation mandatory: Ascend terms (L1/L0A/L0B/L0C/UB, MTE1/2/3, FIX, AIC/AIV) spelled as operator names in rule text with the source term cited once.
- [ ] T3 — standing gates: batch194 parse gate (fixture count = landed rules); kernel_heal slice green; lib default green; clippy -D ×3 postures (default / no-default / all-features — heal doc-lint mechanicals en route, the B192 class); retrieval 21/21 floors HELD.
- [ ] T4 — count pins walked: corpus total + per-category + router.rs + mod.rs doc + AGENTS.md kernel bullet + `.docs/11_domains/kernel_opt.md` seed row + batch paragraph + README (the B192 ledger).
- [ ] T5 — score-bench + freeze NOT owed unless the clippy corpus moved (P30 — kernel_opt-only batch); `.research/226` → Status RECORD with the mine-time appendix (corrections disclosed, the B192/B193 staging-correction precedent).
- [ ] T6 — commit + push (`feat:` prefix, Session line per convention); `.raw/deepgemm-ascend` REMOVED in the same unit (an unfinished batch keeps no live clone).
- [ ] T7 — queue row close-out in `.distill/001` (landing row: counts, gates, corrections, queue-after state).

## Notes

- All Ascend 950DT numbers are provenance, never adoption bars.
- Lane offer (detection-only): candidate 5 as the fused-EP reference for riir-infer's owner-gated kimi_k3 `moe_cubecl` lane — rides the rule text; no sibling edit from this batch.
- Expect the fresh-release source to drift — if the pin is unreachable at T0, STOP and re-verdict (never mine an unpinned tree).
