# Plan 228 — Batch 226: the P49+P50 rider conversion (B211 + B208)

**Status:** IN PROGRESS — claimed 2026-10-13 ~09:1x +07 (m3-max idle loop; count-neutral, corpus 801 stands both domains)

## Goal

Convert the two staged rider-grade intake leads (P49, P50 — graded in the 10-13 ~08:2x idle
envelope) into count-neutral riders on their adjudicated hosts:

- **P49 — bevy `48ae9fc0a1` #26090** ("fix UninitBufferVec to use shader's size for each entry
  instead of Rust's"): the host-view-vs-GPU-stride law. Host struct feeds a GPU buffer → the
  per-entry stride is the CONSUMER's (shader's) declared size, never the host's `size_of`;
  feature flags (glam scalar-math) and WGSL alignment diverge the two views; single-entry buffer
  = LOUD (wgpu binding-size error), multi-entry = SILENT (spare bytes absorb the shortfall).
  → rider on **rust_perf B211 `repr-align-register-partition-store-forwarding`**
  (`src/domains/rust_perf/corpus/entries_18.rs`) — its third one-decision-two-views face.
- **P50 — llama.cpp `358a420e` #30416** ("graph: pad the KV dimension of the no-cache attention
  to 256"): TWO faces, one incident. (1) PAD-AT-THE-PRODUCER — a fast kernel's stride/alignment
  contract on a dynamic dimension must be met where the tensors are BUILT (the graph builder
  pads mask + K/V like the cached path's `get_n_kv` already does), never repaired at dispatch;
  root cause is a PATH-VARIANT divergence (cached path padded, no-cache variant skipped — the
  sibling-transcription class). (2) PROBE-SHAPE-CLASS — the `-fa` auto probe measured at ONE
  token (degenerate, outside FATTN_KQ_STRIDE's domain) and disabled flash-attn GLOBALLY for
  hs-512 models: a capability probe must probe the shape class it gates, never a degenerate
  point. → rider on **kernel_opt B208 `fa-fast-path-validated-exact-shape-admission-v-as-strided-view`**
  (`src/domains/kernel_opt/corpus/entries_gpu_78.rs`) — home adjudicated at claim: the `-fa`
  probe IS the FA admission surface, and the pad omission is what made the probed world
  degenerate; the pad family (B176 rungs / pad-tail-to-fixed-tile) cross-referenced in-rule.

## Tasks

- [ ] Claim row appended to `.distill/001_mining_queue_snapshot.md` + pushed (the arbiter).
- [ ] Plan-first (this file; `.plans/.highwater` 227→228; dual_allocation_gate 0/0/0).
- [ ] Quote re-verification at the pins: bevy `48ae9fc0a1` (#26090) + llama.cpp `358a420e`
      (#30416) — fresh `.raw` clones or the GitHub patch endpoint, rm at close.
- [ ] Dedup check at the hosts: B211/B208 full text re-read; confirm the rider faces add NEW
      content (stride-vs-size_of ≠ register-partition; probe-shape + pad-at-producer ≠
      exact-shape-admission's existing conjuncts); pad-family cross-ref non-duplicative.
- [ ] Rider authoring: `entries_18.rs` B211 += P49 face; `entries_gpu_78.rs` B208 += P50 face.
- [ ] Needle tests: `batch226_rider_needles_landed` in rust_perf tests.rs + kernel_opt tests.rs
      (the batch225 rider-needle pattern; quotes verbatim).
- [ ] GOAT: G1 parse/needles both domains · G2 kernel_opt lib + default lib counts hold
      (2945/2504 floors, rider-only = unchanged) · G3 clippy -D ×4 postures clean + retrieval
      floors HELD (kernel self-variants 100.0%, blended 91.2/94.7/95.3) · freeze/score-bench NOT
      owed (zero clippy-corpus delta — riders on kernel_opt/rust_perf hosts, the P30 precedent).
- [ ] Research note `.research/256_batch226_p49_p50_riders.md` (`.research/.highwater` 255→256)
      + snapshot update (P49/P50 rows → landed; a BATCH-LANDED head row) + AGENTS.md rider lines
      (count-neutral — 801 stands; the two feature-table lines gain the rider note) +
      `.docs/11_domains/` ledger rows.
- [ ] Commit `feat: batch 226: ...` + push (the claim discharged).

## Non-goals

- No riir-train contact (routing restriction).
- No league/perf-rematch edits (watch-only M3 owner call stands).
- No owner-gated lanes (plan 225 T2/T3, plan 223 W1–W9, reflex 085, Rethink lanes, etc.).
- TensorFold EXLR8 + lithos wave-2 pools stay ledgered (future batches, not this one).
