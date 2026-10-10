# Plan 214 — Batch 214: the conversion set's TAIL (Leads 4+5, then 6/7/8)

**Status:** COMPLETE (landed 2026-10-10, M3 idle-loop continuation session; `.plans/.highwater` 213→214, dual_allocation_gate clean 0/0/0 at allocation; commit + envelope row in the batch landing)

Carries the riir-refine idle-loop queue head after Batch 213 (`1a8f068f`): the census-unblocked
conversion set's remaining leads, ALL rider-tier per the staging rows (snapshot lines 15-16).

## Tasks

- [x] T1 — Fetch + quote-verify all five pins (the B213 verification-pass discipline: every quote
      VERBATIM at the pin BEFORE authoring):
      - Lead 4: ggml-org/llama.cpp `1167d3f4` (#29453 CCCL packed version compare) — VERIFIED;
        STAGING DRIFT carried: the LANDED guard is the ordered component form
        (`> 3 || (== 3 && >= 1)`, the ORippler review suggestion); the packed compare is the
        commit message's rationale, not the landed spelling. Both in-rule.
      - Lead 5: tracel-ai/cubecl `5247cd5d` (#1779 lost-device tune stop) — VERIFIED verbatim;
        the lazy-alloc arm is `Vec::new()` + push (allocates only on first failure).
      - Lead 6: PrismML-Eng/llama.cpp `5e9365f05` (#225 Metal PTQ1 5-row dense) — VERIFIED;
        NEW staging detail: the M4 mini canary datum (9.775→11.686, the honest caveat rides).
      - Lead 7: PrismML-Eng/llama.cpp `bf197c43d0` (#312 evict-to-reuse buffers) — VERIFIED.
      - Lead 8: PrismML-Eng/llama.cpp `527ea4d9f2` (#313 top-k from logits) — VERIFIED.
- [x] T2 — Dedup greps per lead (3+ vocabulary variants, both domains) — ALL CLEAN: Lead 4 (one
      unrelated versioning hit), Lead 5 (zero election-axis owners), Lead 6 (adjacent-but-distinct
      ids-path/tile-picker/eligibility rules), Lead 7 (zero eviction-handoff owners), Lead 8
      (adjacent-but-distinct transform-elision/tie-collection/beam rules).
- [x] T3 — Adjudicated: Lead 4 = OWN RULE (kernel_opt D — guard arithmetic, the B186 upstream-
      defect precedent; rust_perf fit rejected — the pattern is domain-neutral version arithmetic);
      Lead 5 = rider on B207 `profile-window-drains-queued-work-before-open` (entries_gpu_77);
      Lead 6 = rider on B198 `fewrow-tensor-tile-decode-into-rhs-operand` (entries_gpu_66);
      Lead 7 = rider on B200 `one-shot-staging-release-to-system` (rust_perf entries_15,
      context_notes — the three-pole disposition law: release-to-system / hand-off / drop);
      Lead 8 = rider on B189 `argmax-epilogue-replaces-output-materialization` (entries_gpu_52).
- [x] T4 — Authored: `entries_gpu_80.rs` (the new D rule with the rollover-table test fixture)
      + 4 riders with distinct-from sets, verbatim quotes, and honest provenance labels.
- [x] T5 — Gates ALL GREEN: batch214 gate FIRST SHOT; batch* family 454/0; count pins 7/7
      (778 total, Dispatch 179); default lib 2488/0; kernel_opt+latent_retrieval lib 3104/0
      (3 ignored = the documented freeze-regen e2es, 2049.8 s — ONE full run at the isolated
      `/tmp/b214tgt`); clippy `-D` ×3 postures clean (2 doc_markdown mechanicals in the new
      slice header healed en route — the B192 class); retrieval floors 21/21 HELD (396 s,
      no decay, no re-pin); freeze/score-bench NOT owed (zero clippy-corpus delta, P30);
      count pins re-pinned honestly (corpus 777→778, D 178→179, 6 test sites + router
      + mod.rs D header + AGENTS ×2 + README ×2).
- [x] T6 — Envelope row in `.distill/001_mining_queue_snapshot.md` + this plan completed (all
      `[x]`) in the SAME commit as the batch landing; commit `feat: batch 214: ...` + push.
      Named-path staging ONLY (the sibling's dirty files untouched).

## Defers

- [-] Leads 9/10/11 (Metal FA admission / bevy exclusive-guard / bevy bench split) — BEHIND this
      batch per the queue order; next slot.
- [-] Lead 12 (PrismML `2a42998c5` PQ2_0 3-8-columns Ada kernel) — head of kernel_opt staging,
      own-rule-or-rider at ITS slot (bigger read: the full q8_1 PQ2 layout + dp4a pairing).
- [-] k=8-MMA (Research 250) — rider tier behind Lead 12.
- [-] The lithos batch (Research 249) — plan-first Protocol A step 0, re-clone at pin
      `236475fa`, compile + score-bench + freeze-table regen in one landing; deserves a fresh
      session's context, NOT this session's tail (the 00:2x envelope row's law).

## Clocks discharged this session (pre-batch)

- mlxfast floor check: `origin/main` == pin `0c7c39233` — UNCHANGED, any-new-accept rider NOT
  fired, weekly floor re-arms ~2026-10-17. (Local-fetch instrument, vendor clone.)
- 4090 mining-partition check: riir-refine tree CLEAN at `2dfb0f02` (1 behind = this lane's own
  Batch 213, will ff); its two cargo processes = riir-train-gpu plan437 training lanes, NOT
  riir-clippy batches. CLEAR.
