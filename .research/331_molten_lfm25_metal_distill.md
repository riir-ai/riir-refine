# Distill 331 — zhangshuibai/molten: the LFM2.5-24B-A2B Rust+Metal specialized engine

**Status:** DISTILLED — pending owner decision

- **Source:** https://github.com/zhangshuibai/molten @ `611e704fe9a3f77e56a459ade78e77bd560cf9a7` (v0.1.0-era tip, cloned shallow into `.raw/molten` 2026-10-26; **pin this sha before any conversion** — the README says the RSI framework behind it "will be open-sourced soon" and the repo is fast-moving).
- **License:** Apache-2.0 (root LICENSE is Lily's file byte-identical per VENDORED.md; MLX portions MIT; model weights NOT in tree — LFM Open License v1.0, never mined).
- **Inspected 2026-10-26** (zed session, macOS): README, Cargo.toml, docs/ARCHITECTURE.md + docs/BENCHMARKS.md in full, VENDORED.md + NOTICE in full, `src/kernels/lfm.rs` (outline + targeted reads: router/argmax/token-tail), `src/kernels/metal/lfm.metal` L700–870 + L1194–1300 (the two headline kernels verbatim), `src/metal.rs` (greps: pipeline_fc, cache), contract.json, corpus + league greps (below). NOT read (unverified — re-read at mine time): `src/session.rs` streaming loop, `src/lfm.rs` prefill orchestration half, `src/lfm/sample.rs`, the NAX `.metal` files in full, tools/ generators, bench/.
- **Scale:** ~6,443 lines Rust + 9 `.metal` files (11.5K lines of MSL, of which 8,168 are the two GENERATED MLX-NAX files). Tiny, dense, complete. No `todo!()`s observed in read files.

## Provenance chain (exceptional hygiene — cite ALL THREE)

1. **Molten** (this verdict's subject) — Apache-2.0.
2. **Lily** — Perplexity AI's Rust+Metal engine, `pplx-garden/lily` @ `1ed972ed3f0bd5616c997c9507c25616c63394fc`, Apache-2.0. Molten started from Lily's Metal runtime/tensor/checkpoint code + a kernel subset, then rewrote for LFM2.5-24B-A2B. **VENDORED.md carries a per-file upstream sha256 table** for every derived file — quote verification at conversion is mechanical.
3. **MLX 0.32.3** (MIT, Apple) — the two biggest `.metal` files are GENERATED from MLX headers by committed generator scripts (`tools/gen_quant_nax.py` from `quantized_nax.h` + `steel/gemm/nax.h`; `tools/gen_steel_attn.py` from `steel_attention_nax.h`), with Molten's own modifications listed in VENDORED.md. **The NAX machinery itself is already-corpus territory** (MLX NAX watch: Issue 844 resolved 2026-09-12; `f99e916` Batch-103-mined; B265/B276 mlxfast faces cover NAX quantized paths) — the mine is Molten's ORIGINAL surface, not the NAX files.

Molten-original surface: `src/kernels/metal/lfm.metal` (2,301 lines, the fused decode kernels), `src/kernels/lfm.rs` (1,141, host dispatch), `src/kernels/metal/lfm_sample.metal` (931, GPU sampling), `src/metal.rs` additions (`pipeline_fc`, per-(source, MSL-version) compiled-library cache), plus the model graph `src/lfm.rs` (1,815).

## What it is

A single-model, single-chip, batch-1 inference engine: LFM2.5-24B-A2B (30 short-conv layers + 10 GQA attention + 38 MoE layers, 64 experts top-4 sigmoid routing with selection bias, MLX affine 4-bit/64 weights mmap'd in place, 6-bit tied head). M5-series Macs only. Measured (author-reported, M5 Max 128 GB, two-pass opposite-order protocol, methodology in BENCHMARKS.md): decode 305 tps @256 ctx vs mlx-lm 190 / llama.cpp 198.5; TTFT @16K 1.76 s vs 2.17 / 4.20; prefill @4K 10,361 tok/s vs 8,829 / 4,796. Correctness: token parity vs mlx-lm (≤25% per 65-step stream, 10% overall; held-out 4.5%), bitwise run determinism, and **"Every decode optimization in the last stage of development was verified token for token against the version before it: 84 prompts of up to 131008 tokens, in three request shapes each"** — the per-optimization bit-parity gate is our own G1 discipline, externally instantiated.

## Verdict

**YES B corpus / LANE INTEL HIGH / product-coverage MODERATE — extends the Metal-inference family to its fifth independent engine instance** (after ds4-metal, lithos-metal, uzu, mlxfast). The Molten-original fused-decode surface is dense decision content in exactly the corpus's shape: fusion-boundary choices with admission conditions, exactness contracts, and coordination protocols. The NAX-derived bulk is already-covered (caps yield, not grade).

## Domain routing

`kernel_opt` — Metal kernel patterns + host-orchestration-around-GPU (the routing table's explicit accepts). The tempting WRONG domain: `rust_perf` (the engine is Rust) — but the value lives in the MSL kernels and the GPU-resident loop, not CPU hot paths. NOT clippy_lints (nothing lint-shaped). NOT a new domain (Metal is long-landed in kernel_opt).

## Ranked candidates

| # | pattern | source | grade | disposition |
|---|---|---|---|---|
| R1 | orderable-key atomic-max single-dispatch argmax | `lfm.metal` L1194–1282 + `lfm.rs` gemv_head_argmax | **B+ (lead, own rule)** | land |
| R2 | wait-free last-finisher cross-threadgroup reduce (+ prologue load-overlap) | `lfm.metal` L705–855 | **B (face)** | rider on entries_gpu_22 spin-barrier rule |
| R3 | token-tail: the argmax reader prepares the next step's input | `lfm.rs` token_tail + `lfm.metal` L1284–1290 | **B− (face)** | rider on entries_gpu_52 argmax-epilogue rule |
| R4 | pipeline-cache key gains the MSL-version axis | `src/metal.rs` compiled-library cache | **B− (face)** | rider on entries_gpu_35 function-constant cache-key rule |
| R5 | GPU-resident sampling: counter-Philox + Gumbel-fold into the argmax key path | `lfm_sample.metal` + ARCHITECTURE.md §Sampling | **B− (family seed)** | own rule, first member of a GPU-sampling family |
| D1 | conv B/C/X fused projection; QKV writes V straight into cache; QK-norm+RoPE prologue | ARCHITECTURE.md + lfm.rs norm_rope_qk_scatter | thin | drop (P83 intake adjacent — llama.cpp MRoPE+set-rows; re-grade faces at conversion) |
| D2 | GQA shared-KV-tiles steel grid rewrite | gen_steel_attn.py modification | thin | drop (lane intel only; B208 GQA family adjacent) |
| D3 | grouped-expert per-segment row skip (`lfm_qmm_t_shared_body`) | quant_nax.metal | thin | drop (B268 moe family adjacent) |
| D4 | 128×128 tile at rows ≥ 4096 | gemm.metal | thin | drop (tile-sizing families adjacent) |
| D5 | exact rewind + conv-state ring + prefix-reuse snapshot | ARCHITECTURE.md §Streaming | owned | drop (rust_perf entries_27/13/37 own the partial-state rollback + resume-reuse families — positive instance) |
| D6 | `lto=false` reproducible-build rationale | Cargo.toml L42–47 | not corpus | drop (build-config flavor; dist-domain at most) |

### R1 — orderable-key-atomic-max-single-dispatch-argmax (the GOAT candidate)

The decode head GEMV never stores logits: every threadgroup folds its argmax candidate into ONE device-wide `atomic_ulong` via a total-order-packed 64-bit key; a tiny follow-up kernel (`lfm_token_tail`) reads and clears it. Zero scratch, zero inter-dispatch barrier, deterministic tie rule.

The key law, verbatim (`lfm.metal` L1196–1207):

> Orderable 64-bit key of an argmax candidate: a larger key is a larger value, or an equal value at a lower index, so atomic_max over keys selects what lfm_argmax_merge's "value desc, index asc" rule selects (-0 ties +0, as the merge's float == does). Candidates are never NaN (the merge never takes one).
> `u = as_type<uint>(v); if ((u << 1) == 0u) { u = 0u; } const uint o = (u & 0x80000000u) != 0u ? ~u : (u | 0x80000000u); return (ulong(o) << 32) | ulong(~i);`

The partition-equivalence contract (L1212–1214): "«Value desc, index asc» is a total order on non-NaN candidates, so the winner is the two-stage argmax's for any partition (including its index-0 result when no logit beats -inf)."

GOAT test: (1) Benchable — NO in this corpus (Metal validator absent) → NotBenchable annotation, the dial9 domain-reason class, never a demotion. (2) Exact semantics — YES: integer key packing is bit-deterministic; the sign-flip transform is the standard IEEE-754 total-order trick; `-0/+0` fold and the `~i` low bits make ties resolve to the LOWEST index (the Issue-940 first-index strict-tie law, third external echo — B249's llama.cpp top-k and B182's "lower column id on ties" are the other two). (3) Magnitude — mechanism-level: deletes one dispatch + the logits store/load per generated token on the head (11% of step time per ARCHITECTURE.md's profile; the B269 shape pays scratch traffic + an encoder memory barrier this shape does not). (4) Deterministic — YES.

Dedup: B269 `two-pass-partial-reduction-inter-dispatch-barrier` (entries_gpu_116) owns the two-pass shape — DIFFERENT reduction protocol (scratch + barrier vs single-word atomic); B182 `argmax-epilogue-replaces-output-materialization` (entries_gpu_52) owns the epilogue-swap + lazy-fallback + [rows,k] shape — this is the single-word device-wide REDUCTION atom its shape doesn't own (B182 returns per-row top-k; Molten folds to one global winner through an atomic); grep `atomic_max|orderable|argmax_key|64-bit key` over kernel_opt = only entries_gpu_86 (a `64-bit key` coincidence, no atomic_max hit — verified). **Carry the B269+B182 kinships in-rule to avoid double-count.**

### R2 — the wait-free last-finisher face (entries_gpu_22)

`lfm_router_addnorm_route_f32` fuses residual-add+RMSNorm + F32 router GEMV + top-4 route in ONE dispatch across 64 threadgroups (one per expert). Coordination: each publishes its logit, then `atomic_thread_fence(mem_device, seq_cst)` → relaxed `atomic_fetch_add(counter)` → the threadgroup observing `E-1` fences again, runs the warp top-k, writes ids/weights, and resets the counter in-kernel (`atomic_store_explicit(counter, 0u, relaxed)`). The host contract (`lfm.rs` L261–264): "The threadgroup that publishes the last router logit runs the warp top-k; `counter` is a zeroed U32 word that the kernel leaves zeroed again. The kernel needs MSL 3.2's device-scope `atomic_thread_fence`, so it comes from the Metal 4 compile of the source."

The load-bearing design sentence (L714–717): "**No threadgroup ever waits on another, so forward progress never depends on co-residency.**"

That is the design-side avoidance arm the corpus's hazard rule doesn't own: `spin-barrier-needs-co-residency-capped-launch` (entries_gpu_22) owns the spin-barrier hazard + its capped-launch remedy; B223's `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` (entries_gpu_85) + the B260 uzu arrival-merge faces own the POLLING/fan-in + fence-discipline shapes — all of them have workers waiting or polling. Molten's shape: single relaxed fetch_add, only the observed-last worker acts, nobody spins — co-residency independence as a DESIGN GOAL, with the MSL-version cost stated (device-scope fences need MSL 3.2 → Metal 4 compile; the rest of the file stays 3.1). Secondary atom, same kernel (L796–799): "Issue this expert's 8 KB F32 router row and the top-k bias at kernel entry, with all 256 threads, so their DRAM latency overlaps the add_rmsnorm prologue (whose barriers also publish the staged row)." — prologue load-overlap: stage the far-DRAM operand before the barrier-heavy prologue so the latency hides under it.

### R5 — GPU-resident sampling family seed (zero corpus owners)

ARCHITECTURE.md §Sampling: "the token is drawn with the Gumbel-max trick from counter-based Philox noise (seed, position), folded into the same key the greedy argmax uses, and the greedy token tail continues from it." Temperature = one extra kernel; top-k/top-p = "exact threshold search: count/mass levels with integer accumulators, refinement rounds, a band gather and a one-threadgroup sort." Measured cost vs greedy (paired rounds, nonce prompts): temperature-only within noise (−0.6/+0.1/0.0%); temp+top_k50+rep_penalty1.05 = +0.6/+1.0/+1.2%; "The greedy path is unchanged (same tokens, same speed)."

Dedup: grep `gumbel|philox` over all domains = ZERO hits. No sampling family exists. The rule shape: sampling that must not evict the GPU-resident decode loop is implemented as GPU kernels fed by counter-based RNG keyed (seed, position), Gumbel-perturbing the EXISTING argmax-key machinery so the greedy path stays bit-identical and the sampled path adds O(1) kernels. NotBenchable (Metal). First member of a `gpu-sampling` family.

## Dedup-grep evidence (per pattern, both vocabularies)

- R1: `atomic_max|orderable|argmax_key` → zero owners (entries_gpu_86 hit is `64-bit key` prose, no atomic_max — read, cleared). Kinships B269/B182 carried.
- R2: `co-residen` → 3 hits, all OTHER classes (entries_08 rust_perf mutual-wait note; entries_gpu_21 recorded-not-landed co-resident-wave; entries_gpu_102 occupancy coupling). The spin-barrier rule + B223/B260 faces are the family hosts — face, not own rule.
- R3: `argmax-epilogue` → the entries_gpu_52 rule + 9 kinship citations. Token-tail atom unowned.
- R4: `function_constant|function constant` → entries_gpu_35 (B129, the cache-key-completeness defect law) + entries_gpu_65's save_lse rider. The MSL-VERSION axis (one compile per (source, MslVersion)) unowned.
- R5: `gumbel|philox` → zero. `sigmoid rout|top-4|selection bias` → zero (the routing fusion itself is inside R2's kernel).
- D5: `rewind|prefix.*reuse|recurrent state` → rust_perf entries_27 (PARTIAL_ONLY leaf-cache law, llama.cpp #30498), entries_13 (resume-reuse boundary), entries_37 (pinned-host prompt states) — OWNED, positive instance.
- `fusion-scope-by-barrier-cost-not-dispatch-count` (mod.rs L2710) owns the dispatch-count-proxy warning — Molten's "262 dependent dispatches (down from 355)" is a positive instance carrying the barrier-cost framing (each fusion also removed the barrier/wait, which is why the count proxy held here).

## Lane intel — HIGH (the user's stated axis: riir-infer related)

1. **riir-ai Issue 1113 (m5ultra decode+prefill defense)** — Molten is a same-family-silicon (M5) specialized-engine REFERENCE, not a league cell (different model — LFM2.5-24B-A2B MoE vs Bonsai-27B ternary). Transferable calibation data: batch-1 decode streams **1.46 GB weights/step at 445 GB/s average = ~75% of the ~590 GB/s bus** (plain streaming read measured 590 GB/s on M5 Max); weight-streaming kernels 430–590 GB/s; GPU busy 99%; **262 dependent dispatches/token, down from 355** — the same dispatch-count lever our Issue 1113 measured (−160 launches/token EXACT), externally confirmed as the decode lever class at −26%. NOT transferable: every tps/TTFT figure (different model + quant + silicon; the absolute-numbers caveat stands — only same-box league cells own comparisons).
2. **The M5 NAX prefill axis** — Molten's prefill wins ride MLX steel/NAX (M5 matrix units) + GQA shared-KV tiles + grouped expert GEMMs. The NAX lane is ALREADY WATCHED (Issue 844 resolved 2026-09-12; `f99e916` Batch-103-mined; the watch note "the M3 Max IS NAX silicon — prefill-axis watch"). Our m5ultra pp rows (0.258× at pp32k, the open loss) are the lane a NAX arm would target; Molten's generator-scripts pattern (committed `.py` → flattened `.metal` from pinned MLX headers, per-file sha256 table) is a reusable adoption shape for that lane. Owner-gated — this verdict offers, does not file.
3. **Issue 851 external confirmation** — 851's finding: "the MLX-class advantage is MODEL-CLASS-dependent (MoE routing + prefill geometry), not a universal kernel trick." Molten's tables confirm on a SECOND MoE model: llama.cpp Metal prefill degrades hard with context (4,796 → 1,799 tok/s from 4K→128K) while the MLX-class engine holds (8,829 → 4,032). Same shape as 851's qwen3.6-35B-A3B probe (llama 1153→712 vs MLX-class flat 1252–1460).
4. **riir-infer Issue 1127 (decode-hold remaining rungs)** — Molten's decode fusion menu is a rung checklist for ANY platform: QKV projections write V straight into the KV cache; QK-norm+RoPE inside the attention kernel prologue; LM-head argmax epilogue (logits never stored during generation); token-tail (the argmax reader writes the next step's embedding+norm); expert gate/up+SiLU and down/combine as single gather kernels. Our lane already owns several rungs (K1–K5 fused rotations; the GLU FFN arm just landed, Issue 1124; RIIR_GDN_CORE_FUSED negative stands); the **token-tail + V-into-cache + argmax-epilogue trio are un-priced rungs** — offered to 1127's bundle, owner-gated.
5. **RSI meta watch** — Molten is a completed worked example of the RSI loop (78 sessions, ~$21, fixed evaluator, token-parity gate, profile feedback; the engine "replaces the best one only if it is correct and faster"). The frozen evaluator contract ships IN-REPO (`contract.json`: per-model constants incl. `greedy_tie: "lowest_token_id"` — the strict-tie law again — `max_checkpoints: 16`, `chunk_bytes`, model/state sha256 pins) — the same shape as our GOAT gate structure. **WATCH: when the RSI framework repo lands (README: "will be open-sourced soon"), distill it** — the evaluator harness patterns are decision content for any future self-improvement lane.
6. **Lily is the stronger follow-up** — the PARENT (`pplx-garden/lily` @ `1ed972ed`) carries surfaces Molten dropped: MoE, GDN, SDPA kernels + the Qwen3.6-35B-A3B graph (its NOTICE names `moe.metal`, `gdn.metal`, `gdn.rs` as not-in-this-tree). If this verdict's batch yields well, Lily is the next claim — likely a bigger batch than Molten.

## Product coverage — MODERATE

The Metal-inference family is already large (ds4-metal B129, lithos-metal B223, uzu B259–264, mlxfast B275/276, MLX itself); Molten adds a fifth engine instance + the first LFM2.5-architecture (short-conv + sigmoid-routing MoE) member. Family follow-up pool: **Lily** (named above — the priority), MLX steel/NAX headers (mined family), mlx-lm `switch_layers.py` (MoE prefill chunk partition — Molten's prefill reference). No new famous-lib seed (Metal is not a crate ecosystem; the corpus's Metal coverage is deliberate and deep).

## Honest caveats

- v0.1.0, fast-moving, single-author + agents; pin the sha at conversion (done above) and re-verify every quote (the B275 lesson — quotes against an unpinned fast-moving checkout are worthless; here VENDORED.md's per-file sha256 table makes that mechanical).
- M5-only claims are UNTESTED on M5 Ultra (the owner's box): README names M5/M5 Pro/M5 Max; the M5 Ultra shares the NAX family but nothing here has verified it — never assume the engine runs there without a run.
- Unread files (listed at top) — the streaming/rewind and prefill-orchestration reads rest on ARCHITECTURE.md prose, not source; re-read at conversion before landing anything that cites them.
- Metal 4 / MSL 3.2 requirement for R2's device-scope fences (the kernel guard is `#if __METAL_VERSION__ >= 320`) — any rule landed from it must carry the portability constraint.
- The $21/6-day/78-session RSI claims are the author's own account — provenance, never evidence.
- Benchmarks are author-run on their box with a sound protocol (two-pass opposite order, cool-downs, medians, published scripts) — still source-reported; our league discipline (same-box cells) does not transfer.
- R1/R2's exactness claims are in-source asserts + the README's token-parity gates — re-verify at conversion by reading the tests (the corpus's ported-numeric-algorithm exhaustive-equivalence law applies to the key packing).

## Recording

Research 331 (this note) + plan 297 (nominal **Batch 293**, created before any corpus edit) + the queue line in `.distill/001_mining_queue_snapshot.md`, one commit, `docs:` prefix, on `develop`. `.raw/molten/` removed at commit. Queue after: UNCHANGED — the batch joins the staged pool behind the live claims (B279 glm in flight; plan-289 bbtree holds the slot to 2026-10-27T16:44+07; plan-294 resin staged).

## Landing (Batch 293, 2026-10-27 — zed-amo-b293-molten/amo)

LANDED as **FIVE count-neutral rider faces on four hosts; corpus STANDS 981** — claim `0d8f3791` (expires 2026-10-29T00:49:09Z) discharged; plan 297 all tasks closed. **THE T1 FALSIFICATION IS THE BATCH'S REAL FINDING: BOTH staged own rules had corpus owners this note's 10-26 greps missed.** R1's `lfm_argmax_key` is the packed-total-order key law — `packed-total-order-multi-field-argmax` (entries_gpu_20, uzu Batch 107) owns it ("one u64 key (f32_total_order_bits(score) << index_bits) | (index_mask - index)"), and R5's Gumbel-max + Philox + filtered-sampling family is OWNED by the SAME file's `gumbel-argmax-filtered-sampling-descent` ("argmax over (logit + negated log of negated log(u)) is an EXACT sample from the categorical", Philox determinism, the u-clamp hazard, MAX_ITERS descent). This note's dedup greps claimed "ZERO corpus owners for gumbel/philox/sampling" — the vocabulary was right but the grep evidently missed the family (likely scoped to rule NAMES, not concept text); the plan's T1 re-verification clause caught it exactly as designed. The R1 grep hit (entries_gpu_97's split-softmax "argmax key" prose) was a FALSE POSITIVE for the packing law and correctly ignored.

Landed faces (each pinned by verbatim-quote marks + the `Batch 293 / Plan 297 / Research 331` provenance assert in the new `batch293_riders_landed` gate):

1. **DEVICE-SCOPE ATOMIC-MAX face** on `packed-total-order-multi-field-argmax` — the cross-threadgroup arm (one atomic_max per threadgroup, no partials/barrier/second kernel) + the −0/+0 equality-match fold + the inverted-index low bits; B269 named the oversized-row alternative.
2. **WAIT-FREE LAST-FINISHER face** on `spin-barrier-needs-co-residency-capped-launch` — the host's own portable note ("or a redesign to a device-scope atomic") realized: relaxed fetch_add + device-scope seq_cst fences both sides + in-kernel counter reset; "No threadgroup ever waits on another, so forward progress never depends on co-residency."; the MSL 3.2 version gate; + the prologue load-overlap atom (8 KB router row staged at kernel entry, pure copies, lane-order preserved).
3. **TOKEN-TAIL face** on `argmax-epilogue-replaces-output-materialization` — the consumer half: the reader writes the token + the next step's dequant-embed + RMSNorm; read-and-clear key lifecycle (no reset dispatch); the host's sampling anti-regime lifted by the sampled arm's same-key/same-tail reuse.
4. **LANGUAGE-VERSION axis face** on `pipeline-cache-key-covers-every-function-constant` — every cache tier keys on (source, MslVersion); the two-version posture real in-source (`#if __METAL_VERSION__ >= 320`).
5. **SPLIT-INVARIANT POSITION-KEYED COUNTER + SMALL-E SERIES + EXACT-TAU LADDER faces** on `gumbel-argmax-filtered-sampling-descent` — counter = (word, absolute position, domain tag, 0) so the draw survives request splitting; the −log1p(−V) series keeps full relative precision to V = 2^-33 where the draws win; the 32-cutoff + ≤2-refinement exact-tau ladder delivers sequential HF/vLLM filter semantics with integer/fixed-order sums; greedy floor bit-identical; +0.6/+1.0/+1.2% sampled source-reported.

The staged R2 prologue atom, R3 token-tail, R4 MSL-version axis all verified verbatim at the pin (this note's quotes were taken from ARCHITECTURE.md prose and the claim-row reads; the conversion read the actual sources: lfm.metal 705-730/796-852/1195-1330, lfm_sample.metal 1-180, sample.rs 1-120, metal.rs 78-90, ARCHITECTURE.md 32-33, session.rs, norm.rs/attention.rs/quant.rs/steel.rs heads, tools/gen_*.py heads — the plan-T0 unread-file clause discharged).

Gates ALL GREEN: batch293 needle 1/1 (24 marks); kernel_opt lib 3259/0; default lib 2749/0; clippy −D ×3 postures; retrieval floors HELD both postures (kernel+latent 22/0 ~209 s, default 21/0 ~52 s) no re-pin; freeze/score-bench NOT owed (P30); binary NOT owed (kernel_opt outside the release set). `.raw/molten` + `/tmp/b293-gate` removed at close.

En-route defects (both caught by the gates, both recorded in the .docs row): (1) the raw-double-quote splice — the Python splice script's `\"` unescaped to a literal `"` inside the Rust string literal (6 parse errors on the first gate run; corpus restored from git, re-spliced with house single-quote wrapping); (2) lost-edit-on-restore — a post-splice face-4 quote correction was wiped by the pre-re-splice `git checkout` and re-surfaced as two gate mark misses (fix AFTER the final splice; verify against the landed state, never the pre-splice memory).
