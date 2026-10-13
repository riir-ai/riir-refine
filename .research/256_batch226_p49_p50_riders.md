# Research 256 — Batch 226: the P49+P50 rider conversion (B211 + B208)

**Status:** RECORD — landed 2026-10-13 ~09:2x +07 (m3-max idle loop; Plan 228; count-neutral, corpus 801 stands both domains)

## Sources (both verified verbatim this session via the GitHub .patch endpoint)

- **P49** — bevyengine/bevy @ `48ae9fc0a1a445dba9f3c5fe55fb5d8de3186ac7` (#26090, fixes #26087,
  MIT OR Apache-2.0, merged 2026-10-12) — `crates/bevy_render/src/render_resource/buffer_vec.rs`,
  1 file +22/−1. Graded B / rider-grade in the 10-13 ~08:2x intake envelope.
- **P50** — ggml-org/llama.cpp @ `358a420ec4ed59b6b0be08b1d1f71dd478b99a99` (#30416, MIT,
  merged 2026-10-12) — `src/llama-graph.cpp` + `src/llama-graph.h` + `src/models/clef.cpp`,
  3 files +22/−12. Graded B / rider-grade in the same envelope.

## The conversions

### P49 → rust_perf B211 `repr-align-register-partition-store-forwarding` (the DEVICE-CONTRACT view)

B211's law was "field width/alignment is ONE decision with TWO downstream costs — the register
PARTITION (codegen) and the CONTAINER footprint (size at N)". The rider adds the **third
audience**: the DEVICE wire contract. A host struct feeding a GPU buffer is sized by the
consumer's DECLARED layout (the shader's, via encase `SHADER_SIZE` — the WGSL-aligned round-up),
never the host's `size_of`. The one-line fix: `item_size: size_of::<T>()` →
`item_size: T::SHADER_SIZE.get() as usize`.

Verbatim atom (PR body): "The shader expects each entry to be 176 bytes. Bevy worked out each
entry's size by asking Rust how big the struct is, not how big the shader says it is. Normally
both answers are 176, but w/ glam's scalar-math feature on, Rust packs the struct tighter, to
168 bytes. If a render phase draws exactly one mesh, the buffer comes out 168 bytes when the
shader needs at least 176, and wgpu throws the binding-size error. With two or more meshes the
buffer happens to be big enough, so the bug stays hidden."

TWO FAILURE ASYMMETRIES (the new atoms): single-entry = LOUD (binding-size error at draw),
multi-entry = SILENT (spare bytes absorb the shortfall — **capacity mistaken for contract**).
The divergence trigger is a FEATURE FLAG (glam scalar-math), so a cfg flip silently re-shapes
every host-fed GPU buffer with no local signal — the host faces (codegen-vs-footprint) fail at
compile time or local bench; this face fails at RUNTIME on the GPU.

Regression-test shape: the fixture constructs the divergence directly ("A `Vec3` is 12 bytes on
the CPU but the struct is 16 bytes in WGSL.") — "fails before the fix and passes after."

### P50 → kernel_opt B208 `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` (probe-shape-class + pad-at-the-producer)

**Home adjudication (made at claim, recorded in the claim row):** B208, not the pad family.
The `-fa` probe IS the FA admission surface — the incident is an admission-gate failure (a
degenerate probe sample disabling the fast arm globally), and the pad omission is the root
cause that made the probed world degenerate. One incident, one rider, both faces. The pad
family (B176 rungs, pad-tail-to-fixed-tile) cross-referenced in-rule via the producer-side law.

Face 1 — **PROBE-SHAPE-CLASS**: the `-fa` auto probe measured flash-attn support at ONE token —
a degenerate shape outside FATTN_KQ_STRIDE's domain — and disabled flash attention GLOBALLY for
hs-512 models. Verbatim: "the -fa auto probe (1 token) disables flash attention for such
models." A capability probe that arms a dispatch flag must sample the SHAPE CLASS the flag
governs (K->ne[1] a multiple of FATTN_KQ_STRIDE for head-size-512 CUDA kernels), never a
degenerate point. The B103 delta face in reverse: the stale conjunct concluded LOCAL fallback;
the degenerate probe concludes GLOBAL disable.

Face 2 — **PAD-AT-THE-PRODUCER**: the no-cache path built a [n_tokens, n_tokens] mask and
passed K/V unpadded so FLASH_ATTN_EXT saw K->ne[1] == n_tokens while the head-size-512 CUDA
kernels require a FATTN_KQ_STRIDE multiple — the op fell back to CPU for most lengths. The
cached path padded via `llama_kv_cache::get_n_kv`; the no-cache VARIANT skipped it — the
sibling-transcription class (path-variant divergence). The repair meets the contract where the
tensors are BUILT: `const int64_t n_kv = GGML_PAD(n_tokens, 256);` on the mask at construction
+ K/V zero-padded at the same site ("padded like the KV cache (see llama_kv_cache::get_n_kv)").
Zero-pad semantics preserved (the mask's −inf tail neutralizes padded columns).

**The triangle (the new decision content):** a validated admission is vacuous unless (a) the
gated shape class is REACHABLE (the producer pads it into legality) and (b) the arming probe
sampled inside the class — **admission, reachability, and probe agreement are one contract**;
each face alone silently voids the other two.

Cross-incident note carried in-rule: the same commit pads `clef.cpp`'s own causal mask "the
same way" — the model family that surfaced the #30100 fusion mispick (B208's sibling rule in
the same file) needed the same producer-side pad.

Distinct-from the B221 rider on the same host (admission == compiled template set, the consent
triple): B221 couples admission to the COMPILER; B226 couples it to the PROBE that arms it and
the PRODUCER that feeds it — three sites, one verdict.

## Dedup check (done at the hosts before authoring)

- B211's existing text: codegen partition + footprint faces; the #163916 alignment-drop rider.
  No stride/shader/size_of content existed — P49 is genuinely new (the third view).
- B208's existing text: exact-shape allowlist + V-as-strided-view conjuncts + the B221
  set-mirror face. No probe-domain content, no pad-at-producer content — P50 is genuinely new.
- Pad-family grep (cachedNarrowZeros / pad-tail-to-fixed-tile / GGML_PAD): no existing owner of
  the producer-side stride-contract law; the cross-ref stays a mention, not a transfer.

## Gates (ALL GREEN, one session)

- **G1** parse/needles: `batch226_rider_landed_on_repr_align_layout_host` (rust_perf) +
  `batch226_rider_landed_on_fa_admission_host` (kernel_opt) — 2/2 pass; every needle verbatim
  at the fetched .patch.
- **G2** lib counts: kernel_opt **2947 passed / 0 failed** (2945 + 2 new = 2947 ✓; 1 ignored =
  the documented freeze-regen e2e); default **2505 / 0** (2504 + 1 = 2505 ✓). Counts move
  EXACTLY by the new tests — rider-only, corpus 801 stands.
- **G3** clippy `-D warnings` ×4 postures clean (default lib / kernel_opt lib /
  kernel_opt+latent_retrieval lib / all-targets).
- **Retrieval floors HELD**: kernel self-variants 100.0% ×3; blended 91.2/94.7/95.3 — the exact
  pinned row, no re-pin. Rider text did not decay retrieval.
- **Freeze/score-bench NOT owed** — zero clippy-corpus delta (riders on kernel_opt/rust_perf
  hosts; the P30/B223 precedent).

## Provenance

- Quote verification: GitHub `.patch` endpoints (both commits fetched in full this session);
  no `.raw` clone needed (the diffs are small and self-contained — the clone-plus-git-show
  path was unnecessary overhead for 2 commits).
- Count-neutral: no count-pin moves (801 stands at all ×5 sites; rust_perf 198 stands).
- AGENTS.md: two feature-table rider lines (kernel_opt B223-family note + rust_perf note),
  no count changes. `.docs/11_domains/` ledger rows updated.
- Queue: P49/P50 intake rows marked landed in `.distill/001_mining_queue_snapshot.md`; the
  claim row discharged by the batch commit.

## Lane intel (recorded, not converted)

- The `clef.cpp` tie (two silent GPU-side defects in one model family within one month —
  #30100 fusion mispick, #30416 pad divergence) suggests model-family-specific GPU paths are a
  recurring silent-defect class: worth a boundary/substrate glance when the decision-serving
  lane next touches clef-shaped encoders.
- The bevy feature-flag-divergence atom (glam scalar-math) is the same CLASS as our
  wgasm32/cubecl vendored-pin posture: a cfg flip changing host-device layout contracts. The
  cubecl pin-bump candidate (B224 lane intel) is the natural place to re-check.
