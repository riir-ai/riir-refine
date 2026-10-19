# Plan 257 — Batch 253: upstream vectorized-transcendental + Stream-K cap + FA-vec WAR

**Status:** COMPLETE — Batch 253 landed (all gates green; see the Research 288 gates section), 2026-10-19 late +07

## Background

The B252 intake row's leads were consumed by Batch 252 (`f5099239`). This batch
converts the NEXT upstream window found by the 10-19 late watch poll
(F:/llama.cpp fetched past the B252 pin `ff5a2337`):

- `42b70032` — ggml-cpu: vectorize `ggml_v_silu` and `ggml_v_expf` (#30791) —
  CPU SIMD transcendental vectorization class. Dedup: NO existing CPU-side
  vectorized-transcendental rule (closest: `fused-vector-site-transcendental-form-nonportable`,
  `newton-refinement-vs-hardware-transcendental` — both distinct).
- `4c1004c4` — cuda: cap MMQ Stream-K blocks at available work (#30553) —
  occupancy/work-capped Stream-K dispatch. Dedup: `stream-k-persistent-cta-mac-segments`
  is the persistent-CTA segments law — the cap face is distinct.
- `99db719b` — CUDA: fix shared-memory WAR hazard in flash_attn_ext_vec (#30573) —
  hazard-face class. Dedup: `barrier-on-reader-open-group-conflict-swap-overlap`
  (lithos group-conflict) + `hazard-tracked-aliasing-scratch-allocator` — need the
  diff to adjudicate own-rule vs rider.
- `31288c9c` #30731 + `fe8841a7` #30218 — hexagon FA (platform-watch faces, likely
  watch/riders per the B250 hexagon-watch convention).
- `412a363c` #30607 qwen3 bidirectional attention + dense_2 — arch watch
  (league-relevant note for riir-ai, not a corpus rule).

Prism fork: tip `1180918489` UNCHANGED, newest tag still `prism-b10777` — decode
re-pin trigger NOT fired (verified via ls-remote this session). L8 A/B owner-gated
(verdict channel claude_code auth-failed ×3 across sessions).

## Tasks

- [x] Read the five upstream diffs at the local clone (F:/llama.cpp) — re-verify every quote
- [x] Dedup grep adjudication (concept-level, not name-level) per candidate
- [x] Author corpus entries (own rules + riders/watches per adjudication)
- [x] Extend the standing batch gate (batch253 rules landed)
- [x] Count-pin re-pins (corpus count + category counts) with annotations
- [x] GOAT: G1 parse/syn, G2 lib tests, G3 clippy −D ×3 postures, retrieval floors
- [x] Research note 288 + .research/.highwater bump
- [x] AGENTS.md kernel_opt bullet + .docs/11_domains/kernel_opt.md row
- [x] Queue-snapshot landing row
- [x] Commit `feat: batch 253: ...` + push

## Gates

Standard batch gates (G1–G3 + retrieval floors). Freeze/score-bench owed only if
rust_perf or non-kernel_opt domains move (P30: kernel_opt-only → not owed).
