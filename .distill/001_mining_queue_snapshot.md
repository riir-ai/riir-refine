# Mining queue snapshot — the distill-program resume state

> **FOLDED 2026-08-31:** the closed-batch history (every idle-loop cycle entry through
> Batch 93 + the FULL post-mining intake records + the sweep/batch-landing sections)
> lives VERBATIM at `.research/108_mining_queue_snapshot_history.md`. This file keeps
> only the live queue — keep it under ~400 lines; fold again when history re-accumulates.
> This file IS the resume state: read at session start, update it in the same commit
> as each batch. Batch DETAIL lives in `.research/NNN_*.md` — this file holds only
> the rolling queue (next targets, sibling-blocked files, corpus count, standing
> lessons). Canonical batch ledger = AGENTS.md batch entries; per-batch detail =
> `.research/NNN_*.md` + `.plans/NNN_*.md`.
>
> Mining via a live external clone — the `.raw` discipline applies: clone into
> `<repo>/.raw/<name>` (gitignored), pin `@ <sha>` + license in the note BEFORE
> the verdict, `rm -rf` when the batch lands. Full contract = `distill` skill
> §"Full-source access" (Batch 96-98 vendor precedent) + katgpt-rs `research` §0.5.

## Live queue state (2026-08-31, post-Batch-93, M3 box)

- **Corpus counts:** kernel_opt 445 · rust_perf 97 · clippy_lints 45 · sec 12 · docker 31
  (AGENTS.md is the authoritative ledger — bump there per batch, snapshot counts are a convenience copy).
- **Last landed:** Batch 98 (2026-09-02, Research 118 / Plan 065 — the ds4 dense.metal/moe.metal
  leads Batch 97 queued: 10 rules, corpus 435 → 445, +4 riders on B97×2/B35/B32a; GOAT all green,
  floors HELD, score-bench #29 flat; 4 mine-time corrections incl. the INVERTED M5 sigmoid
  hazard and the args-aliasing-by-value correction). Prior: Batch 93 (`501fdb2` — ktransformers
  residue disposition, ZERO corpus delta, the last mined-source lane CLOSED) + the t48 substrate
  tie-break fix (`944741f`).
  **2026-09-01 late: P8 adoption wave 2** — the nan-unsafe sweep's benches/examples/
  tests residue + unclaimed repos discharged across riir-clippy (self + loop-habitat
  proposer fix `5122bbe`), riir-neuron-db (`4524854`), riir-train (`20b53943`),
  riir-ai (`1e6e4b06a`, 188 sites / 13 crates); katgpt-rs residue RE-SCANNED
  AND DISCHARGED 2026-09-02 (`a9a72a3e`, wave 3: the ~100 estimate was 17
  live sites / 13 files after the sibling sweeps consumed the rest; 3 gate
  configs count-identical; shared-checkout discipline held — exactly 13
  files committed while the sibling's 16 uncommitted files stayed out via
  explicit-path `git add`). Full record: P8 ADOPTION WAVE 2 below.
- **Next executable (cadence-gated):**
  - **Batch 99 candidate (queued 2026-09-02, Plan 067 / Research 119): wasmi v2.0 blog**
    (wasmi-labs, Robin Freyler, first-party + measured) — route `rust_perf`/dispatch, yield 1–2
    rules, lead = `mir-destprop-branch-site-merge` (rustc ≥1.92 DestinationPropagation merges a
    conditional dispatch's two call sites into one branch site → predictor history mixing;
    wasmi measured ~2800 → >4200 CoreMark from the single fix; Stitch reproduced), secondary
    = `hot-dispatch-gpr-arg-budget` (sysv64 6-GPR budget; float-domain pack; preserve_none
    future). Dual verdict: the same source TRIGGERED the workspace wasmi 1.0 → 2.0 upgrade
    (riir-ai Plan 563 — 13 pins / 7 repos lockstep + fuel recalibration).
  - **The arxiv pool re-walk #4: DONE 2026-09-02 ~07:15 +07** (`.research/113` — covered
    the Tuesday-09-01 announcement window; zero corpus delta; the spec-dec lane woke up:
    DBloom's ceiling-bin acceptance histogram (zero-cost telemetry adoption candidate for
    the verify-loop benches) + compressed draft-side KV (riir-train route) + a rigor-matched
    audit paper (measurement doctrine); two doctrine citations (G4/alloc budgets,
    modelless-first). **Next window 09-03+** (daily differential cadence; MCP quota 09-15).
  - **The arxiv pool re-walk #5: DONE 2026-09-02 (later session, subagent read-only lane)**
    (`.research/115` — Sept-ID (2609.x) differential wave + two NEW query surfaces
    [KV+eviction, quant+inference]; zero corpus delta). **Queued from the walk — both
    DISCHARGED same day (`.research/116`):** 2609.00363 (A−) CONFIRMED on all three pinned
    questions — PoT requantization is deterministic weights-only w/ a byte-exact
    reconstruction gate (mirrorable; caveat: pin our own scale inventory first), the
    ≤1-spacing blindness transfers to f16-KV AS DOCTRINE (not the INT32 alibi — our fp32
    accumulation is order-dependent and multi-ULP, hence tolerance-visible), and Bench 802's
    flat-budget verdict is upgraded to the fault-class classification (tolerance gates
    certify boundedness + preconditions + operand provenance ONLY; Bench 728's div.full is
    the live invisible-class instance). **Lever FILED: riir-ai Issue 845** (PoT-scale
    determinism for ternary group scales + q8kv block scales — kills the div.full class by
    construction; requantize-not-rewrite + byte-exact gate mandatory; entry via GOAT with
    bit-identity promoted from measured to required). 2609.00066 (B+) — runtime-cost concern
    REFUTED in the B+ direction: decision = offline deterministic greedy (RMS-ranked,
    ~5 min/8B model), runtime = static permutation absorbed into the quantize kernel
    (contiguous-segment fast path; −1.5..−2.9% prefill vs fused-RTN, decode memory
    identical). **Grouping lever QUEUED** with a falsifiable entry condition (measured
    collateral-error case the B55 sidecar does not fix at lower cost); block-invariance
    boundary recorded: OCG applies ONLY to block-shared-scale surfaces (q8kv, Q4_K, ternary
    groups, KVarN), never per-channel-scaled ones (Bench 556's insight).
    DASC 2608.30386 = GDN lane record only. Standing fix for walk #6: drop `cat:cs.LG`
    from the spec-dec query (walk-#4 recall hole).
  - **Issue 845 (the PoT lever) CPU HALF LANDED 2026-09-02** — katgpt-rs `91a9cdaf`
    (Plan 586, `pot_scales` opt-in in katgpt-types): `quantize_from_f32_pot` (snap-before-
    quantize = requantize-from-parents), exact `snap_f32_to_pow2`, `all_scales_are_pow2`,
    scale-covariance property test, SNR harness — PoT rel-err ratio 0.988–1.012 vs native
    (the carry loop absorbs the ≤√2 snap factor). T0 scale inventory in Plan 586.
    **Remaining: GPU G1/G3** (Vulkan-vs-CPU bit-identity at scale sites + negative
    control), 4090-window-gated — the box was running the sibling's L4 eval at landing
    time. q8kv arm + trit-tier mirror stay deferred per the issue's entry rules.
  - **09-15 MCP search quota reset** → online perf-search pool walk (queue item 11).
  - **Batch 94 (LANDED 2026-09-01, Plan 058 / Research 112): oMLX PR #3260
    expert-streaming distill CONSUMED** — 8 kernel_opt D + 1 rust_perf io +
    3 riders (B63 4th member, B71 producer complement, B84 routing member);
    corpus 414 → 422 / 96 → 97; head never force-pushed, all anchors
    re-verified at mine time (4 line-range corrections + 2 paraphrases
    folded). GOAT all green: 1238/0 kernel_heal + 1001/0 default + 1728/0
    all-features + clippy 0 both states + floors 14/14 + score-bench #25
    flat. The oMLX external-distill pool is now CLOSED.
  - League opponent-watch: **AWAKE as of walk #4 (09-02)** — 2608.30427 (DBloom:
    block-widening erosion + histogram telemetry; third echo of our DFlash verdict class)
    + 2608.30252 (compressed draft-side KV; riir-train route). Re-arm condition unchanged
    (a ≤24-GB-feasible Flash-Next-class variant with native MTP); the zero-cost adoption
    (ceiling-bin histogram in the verify-loop benches) rides the next bench touch.
    History note (the pinned Rapid-MLX reference row, riir-ai Bench 831, clean window
    `11895e9e5`): prefill 179/189 pp512/pp2048 — decode ≈ llama.cpp; format-crossed non-scoring, scoreboard unmoved;
    corpus delta ZERO. The contaminated first window is the case study: numbers wrong
    by 20–94% and one conclusion inverted — armed-autofire sequencing rule recorded).
    **09-01 same-window CORRECTION (riir-ai `f098133a7` league update): llama.cpp reads
    pp2048 192.95 / pp512 221.49 in the same window ≈ Rapid-MLX's 189.59 — the
    "1.45× llama.cpp" was CROSS-WINDOW; the ~190 prefill frontier is ENGINE-GENERIC.**
    The same session found + FIXED our long-context decode deficit (0.74× → 0.93× @2K,
    +25% e2e, greedy-token-identity EXACT): split-K Metal flash decode
    (`QwenAttentionDecodeGatedSplit{,Combine}CubeCL`, default-ON >512 pos).
    **Batch 95 (LANDED 2026-09-01, Plan 055 / Research 111): mlx-lm
    `gated_delta.py` GDN prefill — 3 rules, corpus 411 → 414 (D 85, E 26,
    G 26).** `simdgroup-row-packing-reduction-amortization` (E),
    `whole-sequence-zero-barrier-single-dispatch-scan` (G — the FOURTH
    recurrence dispatch strategy beside scan/chunk),
    `reduction-restructure-needs-source-owned-comparator` (D — the
    comparator-ORACLE discipline; the snapshot's rider estimate was
    PROMOTED to a full rule: both subagent lanes independently converged
    on it as decision content). Riders: B38 env-flag, B23
    hardcoded-fast-path eligibility, B23 gqa; B79 sequential-loop note
    confirmed for GDN. Snapshot corrections folded at mine time: the
    reduction split is 3-level in-lane + 2-level cross-lane (not 2+2);
    the "(3) RIDER" comparator discipline carried full-rule content.
    GOAT: batch95 gate + kernel_heal 1235/0 + default 1001/0 + clippy 0
    both states + retrieval floors 14/14 HELD; full CI guard not run (the
    3 affected states validated; recorded in Research 111). **Streaming axis added 08-31 (Research 110 §8):**
    oMLX PR #3260 streams Flash-Next-class checkpoints (99 GiB) on 18–36 GB Macs at
    15–17 t/s, and DwarfStar/ds4 (antirez's DeepSeek-V4 engine) publishes M3 Max 128 GB
    q2 at 26.68 t/s decode / 250.1 t/s long prefill — the ≤24-GB re-arm condition should
    read "RAM-feasible or streaming-feasible"; owner call whether the league admits a
    streaming opponent.
  - **NEW 2026-09-02 (riir-ai Issue 843 T5 / league opponent-watch intake): the 1.4×
    prefill jump is REAL, root-caused from source — PrismML fork `9ca265a → e311ed38f`
    landed native Metal PQ2_0 kernels (`73f0fed04`, 08-25, `ggml-metal/kernels/mul_mv.metal`
    `kernel_mul_mv_pq2_0_f32` + quantize.h codec — verified ZERO PQ2_0 mentions in
    `ggml/src/ggml-metal/` at 9ca265a, so the old pin ran type-142 through the generic
    dequant fallback). Secondary in-range fork commits: GDN ring decode (`bed6e9a6b` +
    `08d8c9035`, the known +3.2% decode move, already distilled as the mlx-lm GDN
    batch-95 lane's external confirmation), `e8b6452b1` im2col fix + wide-row set_rows,
    `e0c828b90` Hadamard weight-fold runtime. ~~ARMED DISTILL SOURCE:
    `../vendor/prismml-llama.cpp @ e311ed38f` — `ggml/src/ggml-metal/kernels/mul_mv.metal`
    (PQ2_0/Q2_0 group-128 native Metal mul_mv: 8 sub-blocks × 16 weights, SIMDWIDTH/8
    lane packing — same ternary-class codec as our Q2_0 GEMV family; verdict B, same
    model/quant as the league opponent) + `set_rows.metal` wide-row KV write. Not yet
    mined; next idle-loop batch candidate after the arxiv re-walk.~~ **MINED 2026-09-02
    — Batch 96 (Research 114 / Plan 060): 4 rules, corpus 422 → 426 (N +2, D +1, B +1):
    telescoped-digit-coefficients-byte-fma-dot (N), bit-plane-plane-sums-input-side-
    offset-dot (N), wide-row-copy-tile-rows-across-workgroups (D),
    fwht-shuffle-xor-butterfly-register-resident (B) + riders (B20/B92 byte-perm
    external measured confirmation on HIP v_perm_b32, MI300X 1.9–2.9× decode rows;
    CDNA-vs-RDNA MMVQ geometry regime). Source verdict upgraded to A- at mine time
    (measured HIP rows + upstream PR 26980 lineage).**
  - **LANDED 2026-09-02 (Batch 97, Research 117 addendum / Plan 063): antirez/ds4
    (DwarfStar) distill — 9 rules, corpus 426 → 435 (N+1 E+2 C+2 H+1 M+1 A+1 F+1).**
    Pin `110afdd8`; quotes re-verified byte-accurate; provenance vs upstream
    ggml-org/llama.cpp @ 9ca265a (the check FALSIFIED the mask-block classification
    as llama.cpp-lineage — landed with corrected origin, the popcorn precedent).
    GOAT lead `half-domain-exact-band-dequant-with-raw-bit-oracle` (N) landed with
    the mandatory T6 exhaustive CPU twin — which caught its own first-implementation
    bug (implicit mantissa bit overlapping the exponent field in the normal path).
    Lane B delivered `bit-exact-dispatch-fusion-boundary-emulation` (C) +
    `virtual-thread-folding-preserves-reduction-tree` (E) + the mask rule (F).
    Dropped: candidate 7 row_blocks (LICM/B44). GOAT all green (kernel_heal 1252/0,
    default 1008/0, floors 14/14 HELD, clippy 0, score-bench FLAT).**
  - **~~NEXT BATCH LEADS (from the same source, dense.metal — DS4-native matvec
    family, NOT llama.cpp-lineage):~~ MINED 2026-09-02 — Batch 98 (Research 118 /
    Plan 065): all 5 leads + both P8-intake candidates consumed (10 rules + 4
    riders; the dense.metal lead file is now FULLY MINED).** Original leads for
    the record: (1) quad-composition via pair-impl
    composition + args aliasing — ONE kernel hosts FOUR same-input projections
    by splitting the grid at tgs0 and calling the existing pair impl twice with
    a per-range aliased args struct (dense.metal L1100-1153, device-scope barrier
    before the state append); (2) `kernel_mul_mv_id_mxfp4_sum6_*` route-fused
    down-projection SUM — six routes fused into one accumulation pass with a
    single final simd_sum (moe.metal family, same-input member); (3) the
    shared-read-not-merging decision boundary — two same-input Q8_0 matrices
    that share the ACTIVATION read but deliberately keep per-projection
    reduction shapes (dense.metal L325-330, the family's boundary case); (4)
    tiny-dense-matvec scalar fallback — parallelism DOWN-shift for short rows,
    trading lanes for dispatch + reduction overhead (dense.metal L1440-1441);
    (5) small-batch matvec-ext bridging GEMV and GEMM for 2..8 prompt tokens
    (dense.metal L1596+L1697). P8-intake candidates: host ds4_metal.m
    single-entry pipeline memo (L3077-3081, "decode calls this once per layer
    with identical arguments"); M5 stable-sigmoid-vs-exp annotation (tanh form
    produced non-finite logits at fused vector sites on M5 Max while finite in
    the standalone path — dsv4_hc lane).**
- **Sibling lanes (do not touch):** riir-train — sibling actively building
  (`/tmp/riir-train-499b`); the 4090 perf lanes — sibling PV work mid-flight as of 08-29
  (re-verify lane state at session start before any 4090 unit).
- **Post-mining intake: 0 open classes** — P8 nan-unsafe float sort comparator CLOSED
  2026-09-01 (landed as sec `nan-unsafe-sort-comparator`, Plan 057; record below); all
  prior classes closed (records below; full text in the history note).
- **Owner-gated tails:** `.issues/044` (paging secrets `ALERT_WEBHOOK_URL` +
  `MONTHLY_BUDGET_USD`), `.issues/045` (T7 launchctl activation) — verified 08-31.
- **Queue items 6/7** (boundary-guard + substrate-first drift audit): last run 08-29,
  quarterly — not due until ~11-29.
- **Hygiene:** the snapshot fold itself is this unit (`.research/108`); no other
  pending hygiene. Plans tree clean (0 unchecked boxes); `.issues/` = the two
  owner-gated tails only.

## Standing rules (pointers — full text in the skills + AGENTS.md)

- Plan-first per batch (Protocol A step 0): `.plans/NNN` via `.plans/.highwater`,
  re-scan the folder at WRITE time (dual-allocation discipline).
- Per batch, ONE commit: research note (with a `**Status:**` line) + AGENTS.md batch
  entry + corpus-count bump + this snapshot's one-line queue update.
- Generator lessons (edit_file escape-stripping, `ensure_ascii=False`, line-initial
  `+ ` doc-markdown, CRLF check before fmt, count-pin walks) are recorded per-batch in
  AGENTS.md entries — read the last 2–3 batch entries before writing corpus content.
- kernel_opt file-mining corpus is COMPLETE (B51 declared); open intake channels:
  paper distills (the B70+ arxiv channel), measured-GOAT distillations (the B49
  reopen path (c)), online search (quota-gated), arxiv listing walks.
- Instrument note: `scripts/self_evolve_audit.sh` §3/E5 parses `- [x] P\d` items in
  the intake section below — keep that exact item format when updating.

## Post-mining intake (compact — full records in `.research/108_mining_queue_snapshot_history.md`)

All classes closed; the standing divergence (P4 residual) has no action planned.

- [x] P1 **macro-nesting family** — CONSUMED 2026-08-28 (the descent unit): match_bool +
      uninlined macro-arg descent with the oracle boundary (uninlined direct-child skip —
      format_in_format_args owns that position); 9 fixtures (5 match_bool + 4 uninlined);
      score-bench run #9 steady (96.2/1.9/100/100); live sites healed e2e in ndb /
      riir-train / riir-ai (compile-gated per target).
- [x] P2 **array-literal guard family** — CLOSED 2026-08-29: documented permanent
      divergence, verdict evidence-backed (the `2bf2a44` E0308 proof; population measured
      36 raw hits → 6 map-form → 3 must-not-fire); fails the P7 value/rate bar (machinery
      ~60–80 lines for 3 cosmetic sites); joins the we-stay-silent table.
- [x] P3 **named-arg-with-rename** — CONSUMED 2026-08-29: probe-first oracle table
      (rename + width-$ re-point + identical-delete compositing; field-access values
      SILENT; unreferenced named arg = hard rustc error), the `RenameArg` emitter with
      17 fixtures, module 61/61, e2e byte-for-byte vs oracle; adopted cross-repo
      (katgpt-rs `2e445664` 42 edits, ndb `ffb3234`); + the stale-binary mtime
      discipline (check `~/.cargo/bin/cargo-heal` mtime vs the last proposer commit
      before adoption runs); incident record 2026-08-31: the installed binary sat STALE at the
            08-30 08:48 build across five 08-31 src/ commits (P15 narrowing, P12 suppression,
            Batch 92 corpus +11, guard repair, t48 tie-break) despite the P12 commit message
            claiming "installed binary rebuilt" — cause suspected: the session built under a
            /tmp CARGO_TARGET_DIR and the claim referred to that build. Rebuilt from HEAD
            23:28 +07 (`kernel_opt 411` probe PASS + suggest smoke) — the mtime-vs-commit
            check is load-bearing, trust the mtime not the message. **Recurred 09-02** (the
            Sep-1 18:01 binary sat through the 062 OOD harness + Batch 98's corpus +10) and
            **MECHANIZED same day**: `self_evolve_audit.sh` §0 `bin-age:` check — binary
            mtime (symlink FOLLOWED; macOS stat does not deref by default, caught live)
            vs the last commit touching src/ + Cargo.toml; sibling path-dep state
            deliberately out of scope; gates grep `bin-age:.*STALE`; both branches
            probe-verified. Binary rebuilt from HEAD 17:26 +07 (`kernel_opt 445` probe
            PASS + suggest smoke).
- [~] P4 **created-warning pair** — PARTIALLY CONSUMED 2026-08-28 (`1941712`): the
      plain-expression closure gap was already closed in expr_is_bool; the live residue
      was RECEIVER classification (turbofish `get::<` prefix match, `.lock(` joined
      RESULT_SHAPES); 8 sites terminal-fixed manually (opaque domain-method closure
      bodies — parse-only undetectable). Remaining open: crate-local Option-producing
      helpers (`env_var(` etc.) stay Unknown BY DESIGN (allowlist overfitting) —
      **no action planned** (documented divergence).
- [x] P5 **outcome-verdict wiring** — feed `--verify`-run outcomes + post-heal clippy
      results back into memory tiers. Tracked in [Issue 037] T2 (root-caused — CLI
      records outcomes as `false` only). **LANDED 2026-08-24 via 037-T2 (`667562a`):**
      kept-edit verdict matrix + the clippy lint-resolution oracle + the durable
      `.heal/trajectories.kv` landing; legacy rows stay Heuristic (history is not
      re-judged). Self-evidence-excluded from the E5 gate by design.
- [x] P6 **match_bool guard-on-first-arm + single_match_else literal arms** — CLOSED
      2026-08-29: 20+ shapes probed vs clippy 1.97 across 5 probe crates; flat-conjunction
      guards (`true if g` → `if flag && g`), arm-order-preserving emission, BOOL literals
      confirmed single_match_else-SILENT, and the single-stmt-else OVERSHOOT gate (real
      clippy silent there — never outrun the oracle); match_bool 24 tests + sm_else 28
      tests; e2e byte-for-byte on all 3 new shapes.
- [x] P7 **no-proposer default lints** — CONSUMED 2026-08-29: `manual_is_multiple_of` →
      VERIFY_LINTS tier (corpus 40 → 41; the receiver-allowlist lane rejected — covers
      ~none of the real sites), signed-site auto-revert proven e2e; retrieval floor
      re-pinned per the floor contract (blended top-3 97/101 → 95/101, fix-before-repin,
      baseline isolated via a HEAD worktree); `if_same_then_else` stays owner-gated.
- [x] P8 **unnecessary_sort_by deref-binding shape** — CLOSED 2026-08-29: 12-shape probe
      (deref → MA `sort_by_key(|&a| a)`, plain sort_unstable_by → MA sort_unstable — was
      also proposer-skipped); `Binding::{Plain,Deref}` classification + same-form-only
      rule; 13 module tests; e2e reproduces the oracle byte-for-byte.
- [x] P8b **unnecessary_sort_by field-key family** — CLOSED 2026-08-29: the key family is
      GENERAL (fields/methods/postfix/tuples/consts/external idents); THE FALSE-ORACLE-FACT
      correction (`custom_comparator_unchanged` asserted silence where the probe shows MA —
      test now fires); Paren-grouped keys deliberately skipped (oracle de-parens — parity);
      30 module tests; e2e parity on all 4 fired shapes.
- [x] P9 **compile-breaking proposal cluster in ruliology/player.rs** — CLOSED 2026-08-25,
      commit `64820c1`: root cause = the map_unwrap_or → map_or rewrite needs an unsize
      coercion (`&fn(..)` → `&dyn Fn(..)`) that map_or's free inference variable
      structurally cannot perform; fix = skip mappers whose TAIL casts to a trait-object
      type; 5 new fixture tests pin the guard; **score-bench run #5 delta** (heal
      98.1% → 96.2% — the honest we-stay-silent pin; created 1 unchanged, parse_safety
      100% held); the two innocent single_match_else edits landed on the real file
      afterward (riir-ai `6ca4f00e3`, 70/70 tests green).
- [x] P10 **needless_borrows_for_generic_args + needless_pass_by_value** — CLOSED
      2026-08-29, architecturally out-of-scope: `buffer.extend(&items)` vs
      `buffer.extend(items)` BOTH compile with DIFFERENT semantics — the compile gate
      cannot see it, so even VERIFY-tier is unsound for these lints (semantic-and-silent
      breaks); the correct consumer is `cargo clippy --fix` or the rustc_errors domain.
- [x] P11 **`is_postfix_safe` includes `Expr::Cast`** — CLOSED 2026-08-29 (same-day):
      `x as u64.method()` groups the method into the TYPE position (broken); cast
      receivers now parenthesize; the P7 local exclusion subsumed + the two identical
      LOCAL copies in manual_map/unnecessary_unwrap deduped to the shared helper (the
      copy drift that CAUSED P11); latent `!x as u8` wrong-math closed as
      defense-in-depth; 828/0 lib, score-bench #14 steady.
- [x] **P12 — verify-revert CHURN: the verify gate does not suppress re-proposal.**
      CONSUMED 2026-08-31: the re-proposal suppression filter — the fixpoint pass drops
      any edit whose exact (rule, restore, replacement) pair has 2+ consecutive
      compile-gate fails in the verdict backend (two, not one: whole-file reverts
      attribute fails to every edit in the file — run 2 is the retry window); text
      change auto-unlocks; kill-switch `RIIR_HEAL_SUPPRESS_REVERTED=0`; e2e proven
      (`tests/p12_suppress_reverted_e2e.rs`); score-bench #23 flat (96.2/1.9/100/100).
- [x] **P14 (mis-diagnosed) — MOOTED:** the "unformatted bare-block shape" was the
      manual_let_else mangle × fmt-skip interaction, not an items_after_statements
      emission defect. Closed; no code change.
- [x] **P15 — useless_format emitter can produce invalid cast-then-method text.**
      CONSUMED 2026-08-31 (probe-first, 4 rounds vs clippy 1.97): the single-arg branch
      is TYPE-GATED upstream (peeled type str/String) so cast/binary/unary/paren args
      are ALL oracle-silent — the single-positional path narrowed to the one
      syntax-provable firing shape (a `.to_string()` method-call arg, which also kills
      the double-call emission); every other non-ident arg refuses; refusal tests + 4
      new fixtures; score-bench run #22 UNCHANGED; retrieval floors 13/13.
- [x] **manual_let_else binding-name mangling** — ROOT-CAUSED + FIXED 2026-08-31
      (CRITICAL, 15 real gated breaks): the emitted pattern kept the ARM's inner binding
      instead of the OUTER let name; fix = `renamed_single_binding` byte-precise
      span-splice + the SHORTHAND-FIELD refusal (renaming rewrites the FIELD — refuse);
      4 fixtures.
- [x] **unnecessary_sort_by field keys (E0507)** — REFUSED 2026-08-31 (parse-only
      type-blind): `sort_by_key(|a| a.k)` moves the field out of `&K`, compiles only for
      Copy; the whole field subfamily now refuses (12 fixtures flipped to refusal tests
      + 5 new); whole-binding method keys keep firing.
- [x] **use_self `Self<A>` (E0109)** — FIXED 2026-08-31: `Self` cannot carry explicit
      generic args; whole-segment replacement when the matched path's args match the
      impl head; 1 new fixture.
- [x] **`neg_cmp_op_on_partial_ord`** — REFUSED-FOREVER class, no corpus rule:
      `!(a > b)` → `a <= b` is NaN-UNSAFE on floats; an autofix would be a wrong-code
      generator. Recorded in the cargo-heal skill divergence table as the canonical
      never-heal example.
- [x] **`type_complexity`** — REFUSED class, no corpus rule: the fix invents a type name
      + declaration site — no bounded parse-only rewrite exists; clippy itself does not
      suggest one.
- [x] **`unusual_byte_groupings`** — CLOSED-REFUSED 2026-08-31: below the frequency bar
      by 20–360× (2 sites vs the 39-site smallest admitted candidate); style-only with a
      fiddly fix-space; the frequency bar protects retrieval floors. Re-open only on a
      future fired-histogram population.
- [x] **`unused_assignments` + rustc-side lints** — OUT OF SCOPE by construction (not
      clippy — the clippy_verify oracle never sees them).
- [x] **P-a (guard-landing residue protocol)** — CLOSED 2026-08-31:
      `scripts/consumer_residue_sweep.sh` + `scripts/swept_repos.txt` + the AGENTS.md
      "Guard-landing protocol" section (a guard is not landed until the residue is
      counted); the script was caught lying twice (stale-build silence + the JSON shape)
      and now prints a liveness witness.
- [x] **P-b (the all-features screen)** — CLOSED 2026-08-31: the sweep screen is
      `cargo clippy --workspace --all-features --all-targets --keep-going` (all 32
      residue sites sat behind non-default features; check is structurally blind to
      both guarded classes); landed consumer-side as riir-ai `ci_feature_guard.sh`
      Layer 2.

- [x] P8 **nan-unsafe float sort comparator** (`partial_cmp(..).unwrap_or(Equal)`
      inside a sort closure) — **DONE 2026-09-01 (Plan 057, sec domain): landed as
      `nan-unsafe-sort-comparator`** — 12th sec rule, new `PanicSafety` family, the 6th
      FIXABLE proposer. **The intake's type-blindness concern DISSOLVED by std
      `total_cmp`:** the intake assumed the replacement needed operand DIRECTION +
      float TYPE (the `float_order::{desc,asc,desc_f64,asc_f64}` twin problem), but
      `a.partial_cmp(b)` → `a.total_cmp(b)` preserves the recv/arg order mechanically
      (direction rides the same positions), `total_cmp` is name-identical on f32/f64,
      AND the arg carries over VERBATIM — `PartialOrd::partial_cmp` pins the trait
      signature `(&self, other: &Self)`, exactly the shape `total_cmp` takes, so a
      pre-compiling call site rewrites signature-compatibly and only non-`total_cmp`
      receivers (`Option<f32>`, other Ord primitives) E0599 → verify-reverted. Gates:
      sort context (sort_by/sort_unstable_by/select_nth_unstable_by in the enclosing
      fn — the intake's precision guard) + exact chain shape + `Equal`-only default +
      comment guard + macro args refuse; helper-fn comparators passed to
      `sort_by(helper)` by name are the honest recall limit. Probe-first (Protocol C):
      zero clippy lints fire on the shape (1.97 pedantic+nursery+perf+suspicious+
      correctness+style) — the suggest-only tier the intake recommended was for a
      type-ambiguous rewrite that no longer exists; the strict-gated FIXABLE proposer
      (the `constant-time-secret-compare` precedent) is the more valuable form.
      Validation: sec lib 1051/0 (11 proposer tests incl. the exact corpus round-trip),
      default lib 1001/0 unchanged, boundary gate 2/2 with the new fixture
      auto-covered, retrieval floors HELD (13/13 + 14/14 combined, sec self-variant
      top-1), score-bench run #24 flat (96.2/1.9/100/100), clippy 0 touched files.
      ≈300 sibling library sites are now addressable via heal sweeps with
      `--features sec` (a later per-repo unit).
      **ADOPTED 2026-09-01 (same day):** katgpt-rs `e854958e` (64 files / 113 sites,
      src only) + unused-import cleanup `26d055c6` (7 files); riir-train `d37553e5`
      (41 files / 59 sites); riir-neuron-db `4af0c63` (7 files / 19); riir-mmorpg-
      examples `77d1319` (1).
      **ADOPTION WAVE 2 — 2026-09-01 late session (4090 box): the benches/examples/
      tests residue + the unclaimed repos DISCHARGED.** riir-clippy self `02a5580`
      (11 sites incl. the loop-habitat catch) + **proposer root-cause fix `5122bbe`**:
      the nan_safe_cmp walker's visit_expr had NO ForLoop/While/Loop/Let/Assign/
      Return/Try/Yield/Await/Range/Reference/Group/Repeat arms — the whole-block
      sort SCAN (syn Visit) detected loop-habitat sorts but the edit COLLECTOR
      never descended into loop bodies (found live: sort inside the per-case `for`
      at bench_clippy_ternary_tps.rs:310; 3 fixtures). riir-neuron-db `4524854`
      (16 files / 19 sites post-collision-dedup vs sibling 4af0c63); riir-train
      `20b53943` (6 sites post-dedup vs sibling 24bc7780's 53 in the same surface —
      both sessions swept tests/benches/examples within the hour, rebase
      auto-dropped identical hunks cleanly BOTH times, zero manual conflict
      resolution); riir-ai `1e6e4b06a` (**188 sites / 138 files across 13 crates** —
      the lib was NEVER swept: engine 47, gpu 71, poc 20, examples 15, civ 12,
      games 8, agents 4, rag 4, + 9 singles; per-crate lib suites all green,
      ci all-features screen's 4 error classes proven PRE-EXISTING not-mine).
      Post-wave live residue: ~9 sites in riir-train (2 tuple-receiver suggest-only
      + min_by + helper-fn comparator shapes = the strict-gate recall limits),
      ~2 max_by sites ndb, riir-clippy raw-string fixtures (correctly untouched).
      **WAVE 3 — katgpt-rs DISCHARGED 2026-09-02 (`a9a72a3e`):** the yielded
      tests/benches/examples surface re-scanned to **17 sites / 13 files**
      (the ~100 estimate was pre-sibling-sweep; their waves consumed the
      rest — re-scan-before-heal vindicated). All
      `partial_cmp(..).unwrap()/unwrap_or(Equal)` sort comparators →
      `total_cmp` (the latent NaN-panic class). Validation: 3 gate configs
      count-identical pre/post — default (spec_reconciliation 1 +
      latent_steering 2 + smooth_min_poc 0; latent_field_steering and
      micro_belief are default-on, the gate map said so), one 10-feature
      union (8 targets / 30 tests), and the 2 harness=false binaries
      (bench + example run as exes, structure-identical normalized outputs).
      12/13 rustfmt'd; 1 CRLF healed unfmt'd. Shared-checkout discipline:
      the sibling had 16 uncommitted files mid-flight; explicit-path
      `git add` + staged-set check (13, 0 outside tests/benches/examples)
      kept the commit clean — and the `git diff HEAD~1` 29-file scare was
      the working-tree artifact (single-ref diff reads the WORKING TREE,
      not HEAD; `git show HEAD` is the commit-truth command).
      Fixture-literal sites (score-bench raw strings) correctly never touched.
      Residue: tests/benches/examples slices (katgpt-rs ~47, riir-train ~53),
      riir-wallet-worker 1 FFI site + moka-wasm 10 FFI sites
      (ffi-extern-fn-catch-unwind unit, needs wasm32 validation), seal-game-editor
      17 raw unexamined. **New boundary found by the sweep (P-intake class):**
      TUPLE receivers — `(f32, f32)` sort keys suggest the rewrite but verify
      auto-reverts (E0599: tuples have no `total_cmp`; observed
      riir-mmorpg-examples `src/pet_visual.rs:550`, a cfg(test) site — the compile
      gate caught it correctly, manual fix = sort-key decomposition, a semantic
      choice). Tuple-receiver decomposition = candidate proposer extension
      (rewrite `|a, b| a.partial_cmp(b).unwrap()` over tuple keys to
      lexicographic `.0`-then-`.1` total_cmp — needs a decision on which key
      dominates; leave suggest-only until a real NaN risk shows up).
- [x] **P16 — manual_let_else binding-mangle residue: class CLOSED at 17 gated
      breaks in katgpt-rs (2026-09-02).** The 6038e59 root-cause fix repaired 15
      same-day; a 4090-side audit of every macOS-gated file the heal touched
      found the last 2 (`katgpt-backend/src/gpu.rs` test sites — invisible to a
      Windows compile gate), repaired by the sibling `c284dbb2` (M3-validated:
      both repaired tests EXECUTE on Metal) while a 4090 session independently
      derived the same fix and YIELDED per the dual-allocation discipline
      (duplicate commit reset, never pushed). Audit method: enumerate the 5
      macOS-gated source files, hand-diff each against the heal commit — the
      rest of the gated surface got only shrink_to_fit/const-hoist additions.
- [x] **P17 — standalone `rustc` is a VACUOUS checker for E0425-class mangles
      in dependency-heavy gated files (2026-09-02; the anti-vacuity control
      caught it).** Compiling a single gated file (`rustc --emit=metadata
      gpu.rs`) aborts at IMPORT resolution (E0432/E0433 unlinked crates) BEFORE
      late resolution can fire the E0425 — mangled and fixed versions both
      report 0 "cannot find value", so the check cannot discriminate. The
      discriminating check: extract the rewritten lines VERBATIM into a stub
      harness (fake receiver type) and rustc THAT — type-checks the real
      content without the dependency graph, ~2 min, and it is the only
      Windows-reachable gate for macOS-gated repairs.
- [-] **P18 (open — self-applied 2026-09-02, riir-clippy `2044722`; riir-deployer
      landed 2026-09-01, deployer `2b7d58d`; remaining: riir-ai, riir-chain,
      riir-dao, riir-game-sdk, riir-dapps, riir-train, riir-unity,
      riir-viewbridge, riir-mmorpg-examples — one per cycle) — full-gate
      workflows missing in the sibling repos (katgpt-rs Issue 701 R2 table,
      2026-09-01).** Self-apply landed: riir-clippy's own guard (which was
      already a SUPERSET of the katgpt-rs full gate — layer 5a is clippy
      `-D warnings` both feature extremes) is now CI-enforced via
      `.github/workflows/rust.yml` (weekly Fri 03:23 UTC on develop + main
      pushes + dispatch, mirroring the ndb/riir-chain spending-limit
      convention) + `scripts/standalone_dep_gate.sh` (9 escaping path deps
      pinned across 3 siblings — katgpt-rs public; riir-ai + riir-neuron-db
      private via `SIBLING_REPOS_TOKEN` with the Issue 028 expiry-alarm
      hard-error). Measured error count at landing: the full 5-layer guard
      ALL LAYERS PASS after (a) syncing all 3 floating siblings (riir-ai was
      2 behind — the stale checkout showed a dead `note_tiled_flash_cmma_pv_launch`
      that upstream had already wired) and (b) one own-crate fix: the denied
      `redundant_closure_for_method_calls` at `src/draft/jury_disagreement.rs:426`
      (→ `sort_by(f64::total_cmp)`). Two load-bearing facts learned: path-dep
      warnings are UNCAPPED but NOT denied — they surface in the log yet can
      never fail the gate (riir-gpu's tiled-flash warning leak, from the
      sibling's 09-01/02 work, is visible here but is their lint debt to
      close; do NOT fix mid-flight files in a hot sibling repo) — so the
      `-D warnings` gate fails ONLY on own-crate warnings, no sibling-hostage
      problem; and Cargo.lock is GITIGNORED here (floating siblings churn it),
      so the CI cache key hashes Cargo.toml + rust-toolchain.toml instead.
      CI runner notes live in the workflow preamble: docker Tier-2
      deterministically degraded via dead `DOCKER_HOST` (daemon+registry from
      shared runner IPs = flake factory; local runs own live-oracle
      coverage), layer-5c wedge budget 7200 s for the 4-core runner,
      ~300-min job budget (30-feature --each-feature layer + the GPU
      all-features tree), first scheduled runs are the cost measurement.
      **riir-deployer landed 2026-09-01 (`2b7d58d`)** — the cheapest unit in
      the set and the only INVERTED shape in the workspace: its dep-gate pin
      is EMPTY (zero escaping path deps IS the repo's hard rule — "a deployer
      must build when everything it deploys is broken"), so its
      `standalone_dep_gate.sh` asserts none rather than pinning some; the
      lexical `escapes()` kept verbatim so the four gates stay
      audit-equivalent. Verified by injection at landing (an injected
      `../../../riir-chain` dev-dep fires exit-1; `cloudflare/control-do`'s
      contained `../../crates/riir-control` edge does not). Full guard = 4
      layers (dep gate + clippy `-D warnings` at both feature extremes +
      `cargo test --workspace` incl. the loopback e2e), ALL LAYERS PASS ~40 s
      warm locally; workflow = standalone (no toolchain) + feature-guard
      jobs, weekly Fri 04:07 UTC (distinct slot — the default branch there IS
      develop, so the rot check audits it), main pushes + dispatch.
      Note for the remaining repos: the katgpt-rs 701 R2 table predates some
      workflow landings — re-scan `ls .github/workflows/` per repo at unit
      time instead of trusting the list (riir-dao already carries rust.yml;
      whether it runs the FULL guard is the check that matters).

**Standing never-heal divergence table:** the cargo-heal skill owns it (comment-guarded
matches, array-literal defaults, named-arg renames, nested macro args, NaN-unsound
negated comparisons).
