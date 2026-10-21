# Research 303 — Batch 267: the cubecl fold/autotune/barrier wave + the vllm MLA-prefill prep fusion

**Status:** LANDED — Batch 267 (Plan 271), 2026-10-21, shikuwa/4090 idle loop.
Method: the standing-watch re-poll (blobless clones `.raw/bwatch1021/`, removed at close) enumerated
the post-B265 delta across the watch set; 2 authoring subagents over disjoint slices
(A: cubecl wave; B: vllm/bevy wave); coordinator wiring + gates. Every quote byte-verified at the
pin by the authoring lane (Slice A: 26 quotes machine-checked; Slice B: 39 fragments).

## Corpus delta

kernel_opt 924 → **925** (ONE own rule, category N 81 → 82) + FOUR count-neutral riders.

### The own rule — `partial-constant-float-fold-ieee-unsound` (entries_gpu_116, NEW)

- Source: `tracel-ai/cubecl @ 1ad5040f` #1816 "fix(ir,opt): drop IEEE-invalid float constant folds
  and keep FMA formation real" (Apache-2.0 OR MIT, dual license verified at the clone root).
- The law: a PARTIAL float constant fold (one operand constant, the other runtime) is not
  IEEE-754-sound — signed zeros, NaN and infinity make the result depend on the runtime operand:
  `0.0 + x` is not `x` (`+0.0 + -0.0` is `+0.0`); `x - x` is not `0` (`NaN - NaN` and `inf - inf`
  are NaN); `0 * x` is not `0` (`-4.0 * 0.0` is `-0.0`, `NaN * 0.0` is NaN); `0 / x` is NaN at
  x = 0; `0 % x` likewise; the `x % 1.0 -> x` simplify was wrong on its own terms (`2.5 % 1.0`
  is `0.5`). Only the BOTH-CONSTANTS case folds; the surviving partial simplifications are exactly
  the bit-exact identity arms (`x*1.0`, `x/1.0`, `x-(+0.0)` via `to_bits()==0`).
- The FMA face: formation from mul+add pairs type-tests the SCALAR (deref through the vector) and
  refuses `Complex32Type`/`Complex64Type` — no complex fma overload exists, the rewrite emitted a
  call that does not compile.
- Category disclosure: N (dtype/numerics) over O (IR strength reduction) — the fix lands in
  cubecl-ir/cubecl-opt and O hosts the cubecl-compiler precedent, but the load-bearing law is
  IEEE-754 float semantics (N's numerics family). In-entry.
- Rust surface: any constant-folding pass / partial-evaluation over float ops (our DEC/projection
  lanes' folded pipelines; the ternary drafter's folded transforms).

### The riders (count-neutral)

1. **gpu_87 `autotune-fidelity-synthetic-carries-producer-params`** += RIDER-2, the
   HARDWARE-SHARING axis (cubecl `c8a4728c` #1848): the persisted tune namespace moves from
   `{fingerprint}/{device_id}/{name}` to `autotune/<v>/layout-v<L>/<hardware>/<tuner>` — the
   identity is the HARDWARE (`HardwareIdentity` = DeviceIdentity minus the card slot minus memory;
   parallelism the name-collision disambiguator), the device instance LEAVES the key, a miss reads
   through to the store before tuning, and LocalTuner opens ONE tuner per
   (HardwareIdentity, candidate-list checksum). Composes with the Batch-237 face: that face ADDED
   the adapter fingerprint (cross-adapter hits impossible); this face REMOVES the device instance
   (same-hardware hits possible) — the two halves of one completeness law.
2. **gpu_85 `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder`** += RIDER-3, the
   STORAGE-CLASS-EXISTENCE face (cubecl `aad86f45` #1871): the SPIR-V backend lowered a
   plane-scoped barrier's memory semantics as ACQUIRE_RELEASE | SUBGROUP_MEMORY, but Vulkan has no
   subgroup storage class (VUID-StandaloneSpirv-MemorySemantics-10870 — module fails validation);
   the fix orders plane barriers through WORKGROUP_MEMORY ("A plane shares memory through the
   workgroup's, as the cube does."). Precision: the EXECUTION scope stays Subgroup; only the
   memory-semantics class moves. The barrier-host's third face (B260 width, B264 qualifier, B267
   existence).
3. **entries_popcorn `panic-safe-autotune-candidate`** += RIDER, the REJECTION-SEMANTICS face
   (cubecl `1b4099b2` #1876): a new `AutotuneError::Rejected` splits the old Unknown — a candidate
   whose OWN function returned an error queued no work ("it does not serve these inputs, or failed
   before launching"), so it is kernel-unusability evidence, excluded from the
   executed-unmeasured fallback exactly like a launch failure or manual skip ("deciding a
   rejection would run it for real and fail the same way"). HOST FIT NOTE (in-entry): the
   plan's named first host (`tuned-config-resource-limit-fallback`, gpu_34) owns the baseline-CONFIG
   re-check at dispatch — a different fallback on a different axis; entries_gpu_34's own
   distinct-from line routes candidate-outcome classification to panic-safe-autotune-candidate.
   gpu_34 untouched.
4. **entries_gpu_112 `precompute-chain-fused-intermediates-never-materialized`** (B259-C) += RIDER,
   the HOST-DISPATCHED EAGER-PREP-CHAIN face (vllm `09fd02721` #59634, Apache-2.0): the Kimi-K3
   MLA FP8 prefill prep collapses a three-op eager chain — k_pe concat + head padding + FP8 cast —
   into ONE AITER triton launch ("The k_pe concat, the head padding and the FP8 cast in one
   launch."), the never-born-intermediates law one layer up in the launcher language; the
   admission ladder (cached `@lru_cache(maxsize=1)` capability probe: ROCm + CDNA4 + import probe
   + `not any(d & (d - 1) for d in head_dims)`), the fail-soft eager else, the padded-heads
   strided view, and the gate-fusion half (merged QKV+Gate linear splits `gate` from the same
   GEMM output — the gate GEMM never launches separately).

## Pre-graded drops / watch rows

- cubecl `3a7c0e06` #1879 AutotuneKey shorthand — NULL (clippy-cosmetic macro init form; no
  key-collision defect).
- vllm `7b569ec89` #62029 trtllm-gen for the FlashInfer DCP prefill context pass — WATCH
  (backend-selection for the DCP prefill pass; no portable law extracted this pass).
- vllm `fce64bc5c` #35911 NCCL weight metadata sent once and reused — WATCH (pre-decided: the RL
  weight-transfer lane is niche, the amortization family is well-covered; kept kernel-focused so
  no rust_perf rule landed → freeze/binary NOT owed).
- vllm `34ac4f96a` #62282 Mooncake KV write-after-free peer-wait — WATCH (concurrency correctness
  in a niche KV-transfer connector).
- bevy `59a4d89f9` #25869 clustering GPU buffer allocations — WATCH (cross-frame
  buffer-persistence/capacity-keyed allocation law; no clean in-file home; Slice B report).
- llama.cpp `39da130fc` #30873 ssm_conv kernel size 7 — no new law (per-size constexpr template
  instantiation widening; the corpus owns the variant-axis shape).
- llama.cpp `7e3eeee04` #30361 H2O-Lightning-4B decision model — serving intel: llama.cpp now
  natively hosts a SECOND decision model (after Granite Switch #30676) — the opponent converging
  on the decision-serving wire; detection-only for reflex/rethink.
- vllm `9da5f2d31` #60250 rms_norm_gated custom op default-on (Qwen3.8-27B, ROCm) — league lane
  intel (the league family's gated-norm serving posture).
- MLX `bff10220` #4682 staged affine_qmm_t_nax split-k LANDED (the B265 watch row's landing; M3
  intel, recorded).

## Watch-lane pins after this pass

prism branch `73bfe6cc` + tag top `prism-b10787-73bfe6c` STATIC (the 971 tripwire NOT fired);
lithos `5485ad6` static; TensorFold `d1018b31` static (== the B257 landing pin); uzu consumed
(`32cd07c` #994 = B264's rider; `319787f6` Release 0.6.11 infra NULL); wgpu NULL (5 fix/infra
commits past `e084b66a`); mlxfast not re-polled (weekly floor ~10-24; B265 read it unchanged 13 h
prior).

## STAGED — Batch 268's pool: the splash release wave

`incoai/splash be3b083c..1e532541` = 205 non-merge commits, the release/01-memory→11-cache-quota
wave. Enumeration by subject (grade at claim): split-K sixteen ways on the register tile's
narrowest grids (`6198321`); constrained+unconstrained decode lanes in one command (`f52d1ad`);
the QSA indexer's q/k projection in one dispatch (`4944799`); Flash-Next's down projection
prepares its own table at decode (`55b0071`); auto's PLE row cache 512 MiB + streamed experts the
rest (`18f9c69`); persistent cache emptied only on repeated/engine-reported failure (`6e21cc9`);
eviction-count-gated re-encode (`2712e67`); the Spill series (page-at-a-time resume `fa63d97`,
anchor-bounded output `593f2a6`, leaf-to-Spill-page only `253506f`); publication-take-over-under-
reader (`716760d`); dropped-point chain copies freed before the next point (`a20f344`); restore-
point chain copies kept when a request recomputes (`65f9713`); disk-copy relabel at
checkpoint-ordinary (`4fc7465`); max-cache-disk held while the volume keeps its floor (`51ae046`);
host-step event written from one side only (`e2c0af2`); model-cache refused when the host refuses
the allocation (`f6957e8`); ANE lag derived from what the test times (`26c6f1f`, M3 intel);
pendingGdnRows moved to the test seam (`cfa91f5` — the B227/B247 oracle family's own lane).

## Gates (all run this session, isolated target /tmp/b267_rel, removed at close)

batch267 needle gate PASS (after one expected needle re-key — the entry carried no plan/research
ref until the coordinator's numbering pass); kernel_opt+latent_retrieval lib **3393/0** (3 ignored
documented); default lib **2726/0**; clippy −D ×3 postures clean (default / kernel_opt /
no-default; one PRE-EXISTING B266 doc_markdown residue in entries_38.rs healed en route,
disclosed); retrieval floors HELD 22/22 no re-pin (kernel self-variant 100% 30/30, blended 90.6/94.2,
317 s); the B266 landing's two missed rust_perf pins (241→242 rules, 188→189 annotated) re-pinned
by this gate run, disclosed in-pin; freeze/score-bench NOT owed (zero clippy-corpus delta, P30);
binary rebuild NOT owed (no release-set surface moved — the rust_perf touch is module-doc only;
rebuild at the next release-set landing).
