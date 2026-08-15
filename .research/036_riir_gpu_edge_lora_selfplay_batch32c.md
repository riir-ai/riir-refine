# Research 036 — riir-gpu edge_lora self-play/arena/consolidation mining (Batch 32c / 32b-2)

**Date:** 2026-08-15
**Source cluster:** `riir-gpu/src/edge_lora/` self-play + arena + consolidation half —
`sleep_consolidation.rs` (615L), `arena_eval.rs` (604L), `cross_game_edge.rs` (504L),
`cross_game_real.rs` (224L), `go_self_play.rs` (730L), `bomber_self_play.rs` (521L),
`fft_self_play.rs` (506L). ~3,704 LOC total. The edge_lora module is now **fully mined**
(32a: core; 32b-1: training half; 32c: this batch).

**Corpus:** 238 → **251 rules** (A 39→42, B 39→42, C 25→27, D 35→39, E 14→15).

## Method

Two parallel mining subagents (4-file consolidation/arena/cross-game cluster + 3-file
self-play cluster), each with a mandatory dedup-grep matrix against the 238-rule corpus.
All quotes re-verified line-by-line against the real files before landing (zero drift
found this batch — both subagents quoted verbatim). Landing via a Python generator
(`json.dumps(ensure_ascii=False)` for string literals — correct `\n` escapes by
construction) after the 32b-1 JSON-escaping incident; the awk orphan-`rule_id:` scan
ran BEFORE the first compile and passed clean (0 orphans, 251/251).

## The 13 rules

| Rule | Cat | One-line | Source |
|---|---|---|---|
| `incremental-centroid-running-sum` | C | per-comparison centroid recompute (full member sweep + fresh Vec) → maintain (sum, count) per cluster, update in O(dim) at push | sleep_consolidation L317-357 (naive side in prod) |
| `length-prefix-shape-faithful-hash` | B | payload-only multi-part hash is shape-blind (a=[1,2],b=[3] collides with a=[1],b=[2,3]); feed part lengths into the stream FIRST | sleep_consolidation L404-415 (optimized side) |
| `order-stable-topk-via-index-sort` | D | sort an INDEX array by score, truncate to the top-K SET, re-sort indices ascending — items never move, canonical order, NaN-safe comparator | sleep_consolidation L235-253 (optimized side) |
| `make-unmake-over-clone-per-candidate` | A | candidate search clones the ENTIRE state per probe; apply in place + roll back the recorded delta — bit-identical restore, zero per-candidate alloc | arena_eval L225-232 (naive side in prod) |
| `argmax-rng-tie-break-collect-ties` | B | `max_by` returns the first max (hidden move-order priority); softmax sampling adds normalization to solve a tie problem — one sweep collects epsilon-ties + one seeded RNG draw | arena_eval L298-319 (optimized side) |
| `shape-gated-warm-start-slot-copy` | D | warm-start copies slot-by-slot ONLY when (in_dim, out_dim, rank) all match; count-clamped, mismatched slots learn during training | cross_game_edge L160-185 (optimized side) |
| `domain-as-data-not-algorithm-fork` | D | the cross-game aspect is in the DATA, not the algorithm: convert episodes, delegate to the unchanged loop — no forked trainer, correctness inherited | cross_game_edge L122-131 + L196-209 (optimized side) |
| `one-sweep-spatial-feature-fanout` | C | 12 spatial statistics from ONE board visit: per-cell classification indices route accumulators, signed ±1 indicator serves both colors, geometry-derived denominators never counted | go_self_play L229-314 (optimized side) |
| `deterministic-tiling-projection-tables` | A | modelless fixed projection as construction-time tables (modulo schedule + even-spacing-with-jitter phases); same seed rebuilds the identical encoder; jitter avoids the degenerate all-zero column | go_self_play L94-170 (optimized side) |
| `latent-encode-at-capture` | A | snapshot the latent encoding at capture instead of buffering raw states — peak memory moves × hidden_dim, not moves × full state; no post-hoc drift risk | go/bomber/fft self-play (optimized side, all three) |
| `non-saturating-outcome-reward-floor` | B | raw 0/1 outcomes zero the gradient weight of every losing episode; 0.15 + 0.70·clamp keeps losses teaching "avoid this direction" | go_self_play L410-421 + triplicates |
| `collect-then-resolve-snapshot-batch` | E | simultaneous-turn loops: collect ALL selections against the frozen pre-tick snapshot (read-only, parallelizable) then resolve in fixed order — no information leak, deterministic | fft_self_play L297-328 (optimized side) |
| `timeout-scalar-tiebreak-decisive-outcome` | D | capped games fall through to a maintained-scalar (HP) tiebreak instead of a flat draw — signal density at zero maintenance cost | fft_self_play L201-228 (optimized side) |

## Dedup decisions (14 candidates dropped)

- `scratch` reuse (`legal_buf` go L439/456, `actions_buf` bomber L270/311, `GoArenaPolicy`
  forward scratch) — covered by `caller-owned-split-scratch` / `reuse-allocation-clear` /
  `superset-sized-scratch-across-variants`.
- Per-game seed derivation (`cfg.seed.wrapping_add(game_i)`, arena L379) —
  `counter-derived-seed-stateless-noise`.
- 3-accumulator single-sweep + 8-wide unrolled cosine (sleep_consolidation L360-402) —
  `dual-accumulator-single-reduction-sweep` + rust_perf `simd-chunk` combination.
- Zip-cycle modulo pairing (`i % n_a`, cross_game_real L78-80) —
  `token-flattened-elementwise-modulo-shared-weights` adjacency; residual teaching too thin.
- Consolidation digest audit trail (`merged_hash`/`source_hashes`) —
  `commit-over-maintained-item-digests` + `incremental-multi-part-hash-no-concat` pair.
- Mode-gated lazy prerequisite (arena PredictThenMatch) —
  `predicate-gated-skip-empty-workset-step` adjacency, too thin.
- Bounded-slot fixed array (bomber `[BomberRandomOrGreedy; 4]`) — rust_perf fixed-array
  family; unit structs make it moot.
- Xorshift seed derivation (go L129-136) — implementation detail of the projection rule.
- Per-tick grid-view rebuild (bomber L284/L342-353) — the source is the NAIVE side;
  no optimized production instance to quote → bug candidate, not a rule.
- Lazy-encode-for-discarded-transitions (go encodes input even for Pass) — naive side → bug.
- Standalone cos/sin-table precompute — previously dropped by the corpus
  (rope_geglu_cubecl, corpus header note); folded into
  `deterministic-tiling-projection-tables` as the natural completion.
- `min(reward_a, reward_b)` multi-objective blend — semantics, not perf.
- Dead `output_size == 0` guard — dead code, not a rule.
- `sigmoid_outcome_reward` triplication — dedup candidate (the family is now tracked in
  the batch's riir-ai issue), not a rule candidate.

## Bug yield (filed as riir-ai Issue 685)

**P1 — quadratic centroid storm** (sleep_consolidation L317-337 + L342-357): the
clustering loop recomputes each cluster centroid inside the innermost comparison —
O(edges × clusters × members × dim) + one heap Vec per comparison. Fix = the
`incremental-centroid-running-sum` rule.

**P2 — swallowed failed move** (arena_eval L404-407): `let _ = state.play_move(r, c);`
— a rejected "legal" move silently does nothing; the same policy can re-pick it forever
until forced double-pass. Plus **wasted encodes** (go L457 vs L472: `input_enc` computed
unconditionally, discarded on Pass/rejected moves — O(size²) scan + projection + heap
alloc per discarded probe).

**P2 — `warm_start` field duplicates every prior weight for a boolean's worth of info**
(cross_game_edge L183): `Some(warm_start.to_vec())` retained for the trainer's lifetime;
sole consumer is `.is_some()` in one test.

**P2 — per-call dataset deep-clone** (cross_game_edge L197-207 + L217-223): `train` +
`evaluate` clone every episode's input+target into a fresh `GameEpisodeSet` per
invocation (2× dataset memory per call). A borrowing adapter keeps the no-fork property
without the copy.

**P3 — false-premise sigmoid comment** (go L416-420): the fn is named
`sigmoid_outcome_reward` + the comment describes `sigmoid(1.7) ≈ 0.846`, but the body is
the affine ramp `0.15 + 0.70·clamp` (exactly 0.85/0.15, tests pin 0.85). No sigmoid is
computed. Misleading on all three copies.

**P3 — dead init-touch loop** (bomber L263-265): `for p in players.iter_mut() { let _ =
p.name(); }` — `name()` is a pure getter; the loop initializes nothing.

**P3 — Wait-vs-continue mismatch** (bomber L307-314): comment promises "fall back to
Wait"; the code `continue`s — skipping the player's `advance()` (which ticks the world:
bomb fuses) AND silently dropping the already-computed `input_enc`.

**P3 — per-tick nested-Vec grid rebuild** (bomber L284 + L342-353): fresh
`vec![vec![Cell; W]; H]` every tick of every game (in-source note acknowledges it);
also the root cause of the stale-view mismatch above.

**P3 — misleading synthetic match** (cross_game_edge L280-290): the four-label arm
routes everything except Bomber to `game_a.forward_teacher`, so `target =
GameLabel::Fft` silently trains against game A's teacher; `_` arm dead for the real
label set. Test-fixture only.

**Adapter-skeleton dedup family** (the Issue 685 companion): `sigmoid_outcome_reward`
triplication (go/bomber/fft), `encode_*_position` triplicated 3-line shape,
type-erased opponent-holder enums ×3, entry-point skeletons ×3, transitions closeout
×3, 5-field config structs ×3, test triplication ×3 — one shared adapter substrate
collapses ~200+ duplicated lines.

## GOAT

- **G1:** `batch32c_pseudocode_parses` 13/13 (26 pseudocode blocks syn-parse) +
  `batch32c_rules_present` + `batch32c_concepts_covered` — 3 new tests, all PASS.
- **G2:** 312 kernel_heal lib tests (was 309) + 201 default lib tests PASS.
- **G3:** clippy 0 warnings in BOTH feature states (default + kernel_heal) incl.
  `--tests`; 4 doc_markdown warnings fixed pre-commit (missing backticks on `max_by`,
  `hidden_dim`, `legal_buf`, `warm_start` in the new header block).
- `--all-features` still broken by SIBLING WIP (`rust_perf/bench.rs:335 alloc_axis`) —
  pre-existing since 32a, not this batch's failure.

## Process notes

- The Python-generator landing (script ran once, then deleted) produced a clean landing
  on the first compile — vs the 32b-1 manual-escape incident. The awk orphan scan is
  now the standing pre-compile defense.
- The `.research/.highwater` was stale (34 while 035 existed) — the 32b-1 session
  forgot the write-back. Fixed in this commit (now 36). Lesson repeated: re-scan the
  FOLDER at write time, never trust the highwater file alone.
- Both subagents quoted verbatim this time (0 corrections needed during re-verification)
  — the per-agent quality bar (verbatim-or-mark-it) is holding.

## State

`riir-gpu` mined: ~69 of ~200 files (edge_lora complete: 32a core + 32b-1 training +
32c self-play/arena/consolidation). Next unmined cluster: TBD by the next session's
`ls` — candidates: `spectralquant/`, `gemma*_cubecl/` residuals if any, or the wgpu
orchestration edges. `ternary_deltanet_gpu_forward.rs` (+`_cudarc.rs`) still BLOCKED
on sibling WIP. Corpus: **251 rules**.
