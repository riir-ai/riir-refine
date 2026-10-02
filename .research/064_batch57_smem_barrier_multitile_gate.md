# Research 064 — Batch 57: smem slot role-reversal barrier + multi-tile bit-exact gate

**Status:** RECORD (batch landed, closed)

**Date:** 2026-08-18 · **Source:** riir-ai Issue 715 (resolved + removed 2026-08-17/18; commits `9f9e68402` + `93d853582`) · **Corpus:** 353 → 355 (A 82 → 83, D 65 → 66) · **Kind:** measured-GOAT distillation, FIFTH after the corpus-complete declaration (the B49/B52/B54/B55/B56 reopen path (c)). Opens `entries_gpu_07.rs`.

## 1. Why this, why now

Issue 715's fix commits landed 08-18 11:07–11:44 — BEFORE Batch 56 (13:16) — but the B56 queue scan picked Issues 719 + 714/720 and the handoff's standing-candidate list never evaluated 715. This session's queue re-scan found it via the riir-ai git log (`fix(715): the same two smem races exist in the cudarc CUDA attention kernels`). The commit messages carry full measured evidence (27B model A/B, two-directional gate validation), which is exactly the reopen-path-(c) shape.

Issue 715's two defects:

- **Defect 1 (the races)**: two missing barriers around the smem scratch shared by flash-attention online-softmax tiles — measured on the 27B model, 256-token probe, same binary back-to-back: NONDETERMINISTIC before (first divergent token index 71), BIT-IDENTICAL (3/3) after. Five kernels shipped the races: 3 CubeCL + 2 cudarc CUDA twins.
- **Defect 2 (the RoPE table)**: `arm_c.rs` built `RopeFreqTable` with `config.head_dim` (256) instead of the effective rotary dim (64) — +36.9% CPU-vs-GPU divergence, masked at pos 0 (both conventions are the identity there).

## 2. The two rules

### 2.1 `smem-slot-role-reversal-needs-barrier` (A 82 → 83)

Decision content (from `9f9e68402` qwen_attention_cubecl.rs + `93d853582` cudarc_kernels/attention.rs):

1. **The trap**: a shared-memory scratch whose slot ROLES reverse between phases needs a barrier at each reversal — the corpus's entire barrier content to date is perf-side (eliminate/fuse barriers via warp shuffles, plane sizing, dual-accumulator sweeps); this is the correctness complement marking the reversals that can NEVER be removed.
2. **Window (a), intra-iteration**: every thread reads the reduction result `smem[0]`, then a later phase has thread 0 OVERWRITE that same slot with its weight — no barrier between: a slower warp reads a weight as the tile max, threads disagree on the softmax shift, the tile's weights come out inconsistently scaled. Length-independent — fires at any size.
3. **Window (b), loop back-edge**: the last phase reads every `smem[p]` while the next iteration's first phase writes `smem[tid]` — no barrier on the back-edge: a thread that finishes its read loop early clobbers a weight another thread has not consumed. Reachable only when the loop trips more than once (`n_tiles >= 2`, i.e. n_positions > cube_size = head_dim = 256).
4. **The deadlock constraint**: both barriers must sit in BLOCK-UNIFORM control flow — the kernels' early returns (`head_idx >= n_head`, `n_positions == 0`) are uniform across the block, so no divergent-`__syncthreads` deadlock is introduced.
5. **The twin-audit lesson (folded in-rule)**: the CubeCL fix did NOT reach the cudarc arm — that arm selects `TernaryDeltanetGpuForwardCudarc`, which dispatches hand-written CUDA from `cudarc_kernels/attention.rs`, not `qwen_attention_cubecl.rs`. "Grepping the CubeCL source proves nothing about which kernels it dispatches" — a correctness fix must be audited against every dispatch ARM's kernel selection, not the source tree. (Not a standalone rule: B39 `entrypoint-variants-share-layer-body` already carries the fork-divergence rationale; the arm-selection audit is this rule's applicability note — the twins CANNOT share a body across languages.)
6. **Recurrence**: "Same class as the Issue 612 smem corruption — that fix removed the destructive reduction but left both windows open." Two issues, one loop, three fixes deep.

Distinct from `smem-broadcast-scalar-then-parallel-scatter` (E — the broadcast pattern ASSUMES this discipline; this rule is the required ordering when it is missing), `smem-staging-gated-by-cross-thread-reuse` (B35 — whether to stage in smem at all; this is the barrier contract across a staged buffer's lifetime), and the barrier-elimination family (warp-shuffle / plane-sized workgroups).

### 2.2 `tile-loop-back-edge-needs-multi-tile-bitexact-gate` (D 65 → 66)

Decision content (from the `9f9e68402` regression gate + Issue 710 H10):

1. **The trap**: a loop whose trip count crosses an iteration boundary only past a size threshold hides every back-edge defect from any fixture population whose inputs all sit inside ONE iteration — the loop body runs once, the back-edge is never taken, and back-edge bugs (carry/rescale races, cross-tile state, tail-tile guards) are untestable BY CONSTRUCTION.
2. **The silent bias**: every prior decode test used n_positions <= 200 — strictly inside one 256-wide tile — and PASSED, not because the kernel was correct but because the code under test never ran.
3. **Gate half 1 — cross the boundary**: at least one fixture with n strictly greater than the boundary (n_positions=300 > cube_size=256 → n_tiles=2) so the back-edge actually executes.
4. **Gate half 2 — bit-exact reproducibility**: a bit-exact assert over repeats of the SAME binary — a tolerance-only assert cannot see a race in the low mantissa bits (two runs agreeing to 1e-5 still differ bitwise under a race). The 715 gate: 64 repeats, correctness vs CPU reference AND bit-exact reproducibility.
5. **Two-directional validation**: the gate must fail with the defect present (fails on run 0 with max_diff=0.0586 when the barriers are removed) and pass with the fix — a gate that cannot fail proves nothing.
6. **Recurrence**: Issue 710 H10 — the q8kv multi-tile path (cross-tile rescale + tail-tile guard) had ZERO coverage at n_positions > 256 until a 2-tile fixture was written (n_positions=512, 2 tiles, pinned cross-tile rescale + tail-tile guard). Two occurrences of the fixture-population class.

Distinct from `size-gate-covers-every-cost-factor` (B46 — a dispatch cost gate's input completeness; this is a test fixture's coverage completeness), `verify-once-at-trust-boundary` (B32b — where verification happens; this is what it must be shaped like to see loop-boundary defects), and the benign-case-only probe corollary in `buffer-length-derived-from-allocation-not-binding` (B55 — a probe measuring only the benign case answers wrongly; here the fixture population never enters the defective region at all).

## 3. Evaluated and dropped (with reasons)

- **The RoPE-table dim defect (715 Defect 2)** — "build the table the way the forward consumes it; at pos 0 both conventions are the identity, which is why every pos-0 check passed." Rope-table rules are 3rd-strike-dropped territory (B17 drop + B36-F4 + B37-F1) for the PERF axis; the correctness angle is a driver-contract lesson (`effective_rotary_dim` is `pub` precisely for external drivers), not kernel-rule-shaped. The pos-0-identity test blind spot is noted here as a falsifiable tell, not a rule.
- **The reduction-tree slips** (stride-64 compared `smem[tid+64]` but assigned `smem[tid+32]`; stride-32 missing entirely; all trees stopped at stride 8 leaving `smem[0]` a max over 32 of 256 lanes — "correct but needlessly near the f32 overflow edge" since online softmax works for ANY shift) — copy-paste typo class, not rule-shaped; the incomplete-tree overflow-edge observation is ambient in `online-softmax-streaming`.
- **Dispatch-twin fix propagation as a standalone rule** — folded into rule 1's concept as the applicability note (see 2.1 point 5); B39 carries the structural half.

## 4. Dedup verification (Protocol A step 2)

Terminal grep (the project grep tool silently fails on this multi-root workspace — re-confirmed): every corpus `barrier` mention is perf-side (warp-shuffle B22, dual-accumulator sweep B24, plane-sized workgroup B28, broadcast-scatter, `post-reduction-broadcast-scatter` explicitly ASSUMES "the leader overwrites it after every lane has read"); zero intra-kernel smem race rules. The `n_tiles` hits in entries_gpu_04 are the Batch-33 REJECTED spectralquant seed (Issue 686 H1 — uninitialized partials, different class). Race/determinism hits = B38 env-flag, B49 local-memory demotion, B56 init-lock — none intra-kernel.

## 5. GOAT

- **G1**: `batch57_pseudocode_parses` 3/3 (both rules' ref_shape + opt_pattern syn-parse).
- **G2**: kernel_opt lib **766/0/1** + default lib **571/0/0**.
- **G3**: clippy **0** in both states (all `//!`-header backticks pre-emptively applied — zero fixes needed post-write).
- **Retrieval floors HELD with zero decay**: kernel self-variants **100/100/100**, blended 3-domain **94.6/97.7/97.7** of 130 — identical to the B53/B55/B56 re-pin (fifth consecutive batch at these numbers). 13/13 retrieval tests green (179 s).
- **Count-baseline note**: B56's recorded 760/568 predate the sibling's `c85ff1c` proposer fixtures (2 anyhow!/bail + 1 self-arg = 3 tests, landed via the `d8e614d` rebase after the B56 GOAT run). True HEAD baseline was 763/571; with batch57's 3 gates: 766/571. The sibling's own record ("571/571 default lib") corroborates.

## 6. Generator

One-shot Python (proven method): `json.dumps(ensure_ascii=False)`, `assert count==1` anchors on every replace, dry-run before write, content computed before file open. **All anchors unique on the FIRST dry run — third consecutive clean landing** (B55, B56, B57). First compile clean. New file `entries_gpu_07.rs` opened per the B56 handoff instruction (gpu_06 at 843 lines). Drive-by: fixed the stale gpu_06 file header ("rules 325-351 of 351" → "325-353 of 355" — B56 landed its rules there without updating the header).

## 7. Count pins updated

`entries.len()` assert + message (353→355, "Batch 56"→"Batch 57"), per-batch comment line, KERNEL_CATEGORIES A + D row appends, per-category count comments (A 82→83, D 65→66), the Plan-003 expected-counts table (2 rows + comment), sum + grouped asserts (×2), exhaustive-coverage comment, router.rs ×4 (doc ×2 + assert ×2 + comment), mod.rs doc "353-rule corpus", mod.rs `//!` batch block, gpu_06 file header, `.docs/11_domains/kernel_opt.md` Total, SKILL.md snapshot + Progress, AGENTS.md batch block, `.research/.highwater` 63→64.
