# Plan 280 — Batch 276: mlxfast-qwen38-125b-a6b-engine → kernel_opt

**Status:** IN PROGRESS — claimed 2026-10-25 (claim `6104cf44`; the B275-staged pair's second half, Plan 279 T1 adjudication)

**Source:** Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ `59160bd` — Qwen 3.8 Flash Next 125B-A6B MoE
(6B active); 65 promoted submissions; stock head 1.0 tok/round everywhere = ZERO speculative
decoding; the whole 2.11× record is kernel-side. MIT (root LICENSE, Layr Labs 2026 — re-verified
directly this session; THIRD_PARTY_NOTICES.md credits third-party artifacts/packages separately).
Mine = the repo's OWN engine tree: `Runner/FastModel/*` (TrackFastModel ×25, TrackFastMoE ×24,
TrackFastKernels2 ×11, quantized_nax.cpp/.h ×15, TrackPrefillIndirect* ×~20, TrackFastGDNDecode ×5,
sdpa_vector.h ×6). `Vendor/` stays OUT (third-party notices).

⚠ **THE STAGED INVENTORY IS LEADS-ONLY** (`.raw/b275/proposals/qwen38_inventory.md`, the B275
honesty finding): 24/37 of its quotes failed HEAD verification — every failing text exists only
in mid-challenge submissions later REMOVED (git log -S confirmed). Every landed rule's quote is
machine-verified byte-exact at the pin; dead leads are dropped or re-derived from FINAL state.

Tasks (Protocol A; mark `[x]` as complete, `- [-]` for defers with reasons):

- [x] T0 — pin re-verify (`59160bd` rev-parse, clean tree) + licence re-verify at the root
      (MIT — read this session, NOT inherited from the B275 verdict)
- [ ] T1 — re-enumerate from FINAL state: run every staged lead (13 NEW candidates N1–N13 +
      13 riders R1–R13) through the verify_quotes.py instrument against HEAD `59160bd`;
      DEAD leads dropped loud (the git log -S class recorded), survivors re-derived from the
      final files; honest drops allowed (the C+ tier judged at T2)
- [ ] T2 — dedup grep BEFORE authoring: every surviving concept × 3+ vocabulary variants
      against `src/domains/kernel_opt/` (953 standing) + `src/domains/rust_perf/` (267
      standing), with the B251 CUDA-sibling entries (`entries_gpu_105`/`106`) read in full —
      a Metal face duplicating a B251 CUDA rule is a RIDER on the same host, not a new rule;
      also check the B275 rider faces (gpu_52/26/12/01/07/02) before re-riding the same hosts
- [ ] T3 — authoring slices (subagents over disjoint slices, ≤3 rules each per the B268
      slice-timeout lesson); every quoted line machine-verified against the real file at
      the pin; standalone fixture compile per rule
- [ ] T4 — coordinator wiring: `entries_gpu_124.rs` (next free — verified at write time),
      `KERNEL_CATEGORIES` slices, mod.rs registration, count pins via ONE shape-asserted
      python pass (tests.rs sites + router.rs + the expected table + category comments —
      the B265 script-job law); byte-level CRLF-safe edits to
      `.distill/001_mining_queue_snapshot.md` (natively CRLF)
- [ ] T5 — gates: batch276 needle + parse gates (feature-gated → `--features kernel_opt`),
      kernel_opt lib, default lib, clippy −D ×3 postures (default / kernel_opt /
      no-default, all --all-targets), retrieval floors RE-MEASURE (corpus delta owed,
      isolated CARGO_TARGET_DIR while the plan-622 sibling measurement holds the box);
      freeze/score-bench NOT owed (kernel_opt-only corpus, P30/B250) unless clippy_lints
      sneaks in; binary rebuild NOT owed (kernel_opt outside the release set)
- [ ] T6 — Research note `.research/314_*` (`.research/.highwater` 313→314 at landing),
      AGENTS.md kernel_opt bullet re-pin (953 → landed count), `.docs/11_domains/kernel_opt.md`
      if owed, queue-snapshot landing row (byte-level CRLF-safe splice), mining-queue
      snapshot update in the same commit
- [ ] T7 — commit `feat: batch 276: ...` + push; `.raw/b275/` removed at close (both clones —
      bonsai2's clone dies with the pair's completion); claim discharged by the landing row
