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

## Live queue state (2026-09-05, post-Batch-104, M3 box)

- **Corpus counts:** kernel_opt 482 · rust_perf 99 · clippy_lints 45 · sec 12 · docker 31
  (AGENTS.md is the authoritative ledger — bump there per batch, snapshot counts are a convenience copy).
- **Last landed:** **Batch 106 (2026-09-05, Research 129 / Plan 076 — the MTPLX upstream
  residue distill; 4090 box, CPU-only; the queued NEXT-UP row consumed): 5 rules, corpus
  477 → 482 (D 106 → 111).** Source youssofal/MTPLX upstream main @ `dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d`
  (v2.11.1, Apache-2.0) — the three surfaces Batch 103 never read: the upstream `mistakes/`
  post-mortem corpus (16 files, filename IS the rule), docs/plans+specs+concurrency, and
  the targeted kernel diff since the mined base `8bc4d88`. `prefix-reuse-key-must-mirror-
  the-restore-predicate` (D, the B+ lead CONFIRMED with BOTH instances in-source: the
  sentinel bank-fingerprint post-mortem + the dd4031a8 fix commit itself — the fingerprint
  carries only what changes KV/MTP history without changing tokens, and every reuse
  estimator must ask the question the restore path answers; the 2.10.2 exact-containment
  shed re-prefilled 41,901 tokens cold / 54 s with a 41,391-token block restore available),
  `request-arrival-work-asks-reuse-state-first` (D — arrival-time work gated on the
  bank's shares_ram_prefix answer + never await unconsumed work; the PLE first-gather
  650k-row warm-turn waste), `serving-raises-only-behind-strict-flag` (D — the verdict-
  delivery policy composed with B103's measurement half: measurement = raise, serving =
  count + one warning per class), `derived-key-single-constructor-parity-test` (D — one
  constructor for every reader+writer of a hashed contract key + a drift-failing parity
  test + provenance markers for values equal to defaults), `vendor-fused-op-instantiation-
  check` (D, axis-A find — read the shipped metallib's symbol table before assuming a
  vendor fused op covers your shape; MLX sdpa is instantiated only for head dims
  64/96/128/256, the MLA latent is 512 wide, so the 'fast path' measured the vendor's own
  fallback). Riders: p99-inter-emit-gap census + delivered-cadence doctrine (league doc
  citations), stale-binary provenance (Bench-749/Issue-866 confirmation), git-stash-
  shared-by-worktrees (our AGENTS.md rule confirmed), census-vs-E2E 4th instance. 11
  mistake files dispositioned out-of-scope in Research 129; `docs/perf/` subdirs named
  unread-remainder. EN-ROUTE: fresh FULL clone needed (the queue's depth-1 note) +
  `core.longpaths` (mistake filenames exceed Windows MAX_PATH); the mod.rs "464-rule"
  doc line was 2 batches stale (post-B104/B105) — repaired to 482. GOAT: batch106 gates
  3/3 FIRST-SHOT + kernel_heal lib 1343/0/1i + default lib 1078/0 + kernel_heal+latent
  1451/0/2i (floors HELD zero re-pins) + clippy 0 --lib both states (the exFAT hard-link
  cargo noise excepted). Count pins walked: tests.rs 75 + 5368-sum + 5382-grouped +
  expected-array D + router.rs 461 + mod.rs D header/list + the 464→482 doc-line repair.
  Prior: **Batch 104 (2026-09-05, Research 126 / Plan 074 — the ggml-org/llama.cpp
  fa-vec offline swept launch-table distill; the 09-02-queued league opponent-watch lead
  consumed; M3 box, CPU-only, read-only source lane): 2 rules, corpus 464 → 466 (K 5 → 7).**
  Infra `efeda76b9` (08-24: ggml-metal-tuning.{h,cpp} the runtime table + tools/tuning/ the
  offline sweep harness), table pin `8963a9bdc` (08-28, #27863: the M3 Max rows — 91 rows
  F16+Q8_0, the league-relevant SKU; our pinned opponent fork e311ed38f does NOT carry fa-vec,
  upstream-only), harness `d230ddd76` (b10793; tools/tuning/ BYTE-IDENTICAL pin→HEAD verified
  by empty diff; post-pin movement is table ROWS only: 1,293 → 3,420 rows across 20 SKUs in
  two weeks, the community sweeping through discussion #27668). `swept-shape-bucket-launch-tables`
  (K — the A− GOAT lead: when a kernel's optimal launch config is non-monotonic across
  device × dtype × shape × axis buckets, sweep OFFLINE and ship a constexpr data table keyed
  by an 8-byte packed struct ("tightly packed for memcmp") with SIX decision contents:
  the fallback ladder (exact bucket → collapsed domain default → family-representative SKU →
  template baseline), the short-axis skip ("short KV: attention is a small slice of the step,
  left to baseline"), consumer-side device-limit validation ("fall back to baseline (Q=1) if
  the tuned config exceeds threadgroup memory"), the shared legality constraint
  ("single source shared by the offline tuner and test-backend-ops"), exception-row sparsity,
  and the swept-vs-derived axis split (Q/NE swept, nsg stays the runtime formula); grid
  derived from runtime specialization boundaries ("Bucket 0 is not sampled at all: the
  runtime leaves short KV at baseline, so no measurement there can reach the table");
  harness "never a pass/fail test" / "stdout carries nothing but table rows"; in-stack
  contrast: our tiled-flash m16/m32 single-axis single-SKU length gate),
  `no-harm-bucket-representative` (K — a bucket representative is a WORST-CASE-REGRET
  contract, never best-average: "no slower than the baseline config at every point that
  bucket covers" + the padding witness ("one of a bucket's first Q values carries the worst
  padding it can ever see, and that value has to be sampled ... a floor on the evidence, not
  a proof") + two gates not one (TUNE_THETA 1.05 aggregate / TUNE_TAU 0.05 pointwise) +
  unmeasured-is-baseline + anchor/cooldown drift gating with dirty-window re-measure +
  refusal reporting + fixture realism is timing-load-bearing (the -INF mask blocks)).
  Dedup: anchor-shape-bucketing (L) = cache-KEY bucketing, composed not duplicated;
  K-family runtime-autotune rules = RUNTIME mechanics; B77 twins = compile-time tile
  instances; B103 shape-class = capability verdicts. Drop ledger: sample-where-consumable
  folded into rule 1; harness-as-shipped-binary folded as process half; mask-timing note
  cited in rule 2 (B97 owns the mechanism). GOAT: batch104 gates 3/3 + kernel_heal lib
  1294/0/1i + default 1035/0 + kernel_heal+latent floors HELD (kernel self-variants
  100/100/100 — zero re-pins, blended 94.7/96.9/98.5) + clippy 0 --lib three states (one
  doc_markdown fix: F16+Q8_0 backticks — the B37 lesson). Count pins walked incl.
  router.rs expert total (the 18th-count-pin-site lesson — the FIRST failed run caught it).
  EN-ROUTE: the stale .plans/.highwater (read 72, Plan 073 existed — the 848 class)
  repaired to 74 at allocation; en-route gate-collision REPAIRED IN-PASS (the batch104
  insertion edit fuzzy-clobbered the batch97 oracle test's header + batch103's `for`
  header — both restored byte-exact before any commit, caught by the compile). Record:
  Research 126 / Plan 074 (.raw/llama-upstream removed at landing).**
  Prior: **Batch 103 (2026-09-04, Research 124 / Plan 072 — the MTPLX PR #391 + MLX NAX
  combined host-orchestration batch; T7 default COMBINE executed; M3 box, CPU-only): 4 rules,
  corpus 459 → 463 (D 98 → 102) + the B100 external-instances rider.** `shape-class-keyed-capability`
  (D — the A− GOAT lead: a capability verdict is valid ONLY for the shape tuple the native gate reads;
  the MTPLX wiring defect — the ONE call site that could reach the verify width asked about a width the
  flag never armed — measured the control on both seeds silently; the owner's rule 'a flag either works
  on every request path the server accepts, or it fails loudly at install'),
  `wavefront-overlap-pair-ceiling` (D — bound the overlap-pair count BEFORE building any multi-stream
  seam: exactly S-1 pairs, each bounded by min(producer tail, consumer head); their 2×2 wavefront:
  '0.4 % against a 15 % gate; tile ceiling 19.2 %' — 'the ceiling is too low to fund'; + 'cross-stream
  fences cost as much as the overlap gains'), `request-shape-decline-mirror-completeness` (D — THE
  MIRROR: every request-dependent decline branch of the native contract paired with a stable key;
  everything else is CONFIGURATION and raises; two production outages ('HTTP 500 on every HumanEval
  prompt' + the 1,024-token-in-a-2,048-bank crossover miss); decline sites named not totaled + reasons
  counted + the armed-lane-must-engage raise), `backing-provenance-causal-shadowed-extent-widen`
  (D — from the MLX NAX extraction @ f99e916: widen K to the tile multiple only after the slices PROVE
  the padded rows are backed (stride sanity + row-pitch containment + buffer end) and causally
  unreachable (qL_off from the WIDENED extent keeps the true diagonal); ragged-tail branch deleted).
  Rider: B100 register-budget-head-split += EXTERNAL INSTANCES (the NAX hd-256 dsplit — WN==2
  static_assert, the ONE-peer exchange, Q-half register residency; the #4380 GQA-12/16 table — hpt
  falls as GQA rises, KV ≥ 8192 threshold unchanged). T0 re-pin: MTPLX verdict pin ef96313 →
  **a5e38bb** (branch rewrite deleted the fable_* scaffolding + rewrote the body 09-03 18:24Z; TWO
  verdict anchors did NOT survive, carried honestly in-rule: the W87 attribution NOT re-confirmable +
  the hard-coded-draft-width note re-anchored to the wiring test). T5 graph-build overlap (W67)
  evaluated then DROPPED with reason (their lane is default-on at the new head, but our Bench-761/771
  caches consumed the same seam — no measured exposed host-enqueue window remains). T6 riders
  research-noted: Bench 805 mask-fuse external confirmation (+3.5% prefill but divergence 8/8 at
  median position 7.5 — ours bit-identical, the posture is the point); Issue 734 GDN regime note
  (theirs −0.35 s on unblocked-eager stock vs ours 0.97-0.98× on already-batched); Bench 641/B77
  micro-vs-e2e third external instance (micro 1.94×, e2e −3.6%). Non-goals held: NO prefill-throughput
  claim; no league action. GOAT: batch103 gates 3/3 + kernel_heal lib 1285/0/1i + default 1029/0 +
  kernel_heal+latent 1392/0/2i (floors HELD zero re-pins) + clippy 0 --lib all three states. Count
  pins walked + EN-ROUTE rot fixed: the 15 mod.rs category header comments were stale by up to 2
  batches (A 103 vs 105, C 39 vs 41, H 16 vs 17, N 27 vs 28 — pinned by NO test) — all 15 now match
  the pinned table. Generator lesson re-proven (B77 escape-strip class): the in-place B100 rider edit
  landed bare quotes (22 compile errors), repaired byte-precisely; the Python-heredoc GENERATOR path
  was clean first-shot.**
  Prior: **Batch 102** (2026-09-04, Research 123 / Plan 071 — the FlashQuant
  sparse-dense fusion distill, arXiv 2608.15531; taken over from a dead sibling session +
  renumbered 070 → 071 per the reserved-071 flag): 6 rules, corpus 453 → 459 (A +2, C +2,
  H +1, N +1). `heterogeneous-unit-paths-shared-tile-fusion` (C — the additive-heterogeneous-
  paths member of the same-input family: TC dense + CUDA-core sparse over one tile space,
  offline tile correspondence, EW extra-warps rider folded), `lpt-min-heap-disjoint-column-
  ownership` (H — assignment-time LPT balance, one owner per column, zero atomics — the work-
  map family's balance complement), `tile-coo-sparse-tile-aligned-format` (N — u8 indices
  from staged-tile extent + padding-slot repurpose), `bucketed-sparse-reader-column-partition`
  (A — access-granularity mismatch, consumer-side partition vs B92's address-transform arm),
  `bank-group-circular-element-reorder` (A — offline bank-group staggering keyed to the
  consumer walk), `stream-k-persistent-cta-mac-segments` (C — MAC-space decomposition with
  B58 uncapped-default as mandatory control arm; Lane B's decomposition-vs-reduction kill-check
  beat Lane A's rider deferral — coordinator adjudicated LAND). Headline honestly carried:
  batch ≥ 8 regime, NOT our batch-1 decode; INT4+BF16 evidence-only. GOAT: batch102 gates 3/3
  first-shot + kernel_heal lib 1282/0/1i + default 1029/0 + kernel_heal+latent 1389/0/2i with
  retrieval floors HELD (kernel 100/100/100, 13th consecutive) + clippy 0 both states. Full CI
  guard NOT run (the B95 honest-skip shape — the 504 T3 NoRA matrix holds the 4090 until
  ~09:00–10:30 09-04).** Prior: Batch 101 (2026-09-02, Plan 069 — `dispatch-handler-pair-hoist`,
  corpus 98 → 99, x86 re-gate ARMED) + Batch 100 (2026-09-02, Research 120 / Plan 068 —
  the Layr-Labs/qwen-3.8-mtp-challenge distill: 8 rules, corpus 445 → 453, A +3, D +3, E +2).**
- **Last landed:** **Batch 100 (2026-09-02, Research 120 / Plan 068 — the Layr-Labs/qwen-3.8-mtp-challenge
  distill, the M5-Max Swift/Metal inference arena, same team as ds4 B97/98): 8 rules + 1 B22 rider,
  corpus 445 → 453 (A +3, D +3, E +2). Lead `functional-cache-versioning-loses-donation-under-
  kernel-read` (the MLX slice-update donation-loss trap; ONE numeric correction from the verdict:
  23 layers × {K,V} × 8 GiB, not 24×; + the throttle-serialization rider ~13× + the ≥2-layers
  fixture corollary) + the in-place-contract family (fused single-writer-reads-newest-from-input;
  fence-token-consumer + the MLX <8-element CONSTANT-address-space ABI rider) + Metal
  residency-set wire-once + the register-budget HPT head split (test file confirmed:
  CBv2PagedEligibilityTests) + 3 NOVEL (batch-composition-invariant-grid-early-return;
  never-gqa-multiply-full-d-merge-smem — the source doc's own extracted rule; container-rebind-
  over-retrace-for-churn-survival — the FOURTH graph-churn mechanism beside B58/B81/B92/B97).
  Rider: B22 warp-shuffle += the Metal measured-cost note (simd_sum ~14.5 cyc vs ~2 shuffle).
  Pin byte-identical to the verdict (tree static since 08-23; leaderboard bumps are auth-gated).
  Two verification lanes; DFlashBatchedEngine = honest zero. R6 SKIPPED per plan (config-knob);
  7 defers ledgered in Research 120 (span-snapping, hidden-footprint ledger, chained-decode
  overlap, dtype priming, scratch-lane recycle, wrap-seam warmup, dispatch-batching note).
  GOAT: kernel_heal 1262/0/1i + default 1012/0 + clippy 0 both states + floors 14/14 HELD
  (bands identical); ALL count pins walked incl. the batch22 full-concept pin (moved WITH the
  rider — the gate caught the deliberate change as designed).** Prior: Batch 99 (2026-09-02, Research 119 / Plan 067 — the wasmi v2.0 blog
  distill, the first rust_perf batch since Batch 94): `mir-destprop-branch-site-merge`
  MEASURED ParetoDominant 2/0/1 (1.04/1.24/1.25×, corpus validator; probe-stable ×2),
  asm-verified mechanism (merged = cmovne + ONE indirect tail-jump; split = two
  DIRECT tail-jumps, no re-merge), negative control measured (runtime-opaque
  targets = 0.95-1.00× — rule applies ONLY to compile-time-known code pointers),
  toolchain regime rustc ≥1.92 DestProp; corpus 97 → 98, dispatch 3 → 4; the C+
  secondary `hot-dispatch-gpr-arg-budget` probed-Tie on msvc-x64 (1.009/0.999×) →
  queued-secondary kill-free (sysv64 window). GOAT: default 1012/0 + bench 1030/0 +
  floors 13/13 + score-bench FLAT + clippy 0 both states; SIX count pins walked
  (three caught by the full-suite run). Prior: Batch 98 (2026-09-02, Research 118 / Plan 065 — the ds4 dense.metal/moe.metal
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
- **2026-09-03 (4090 box, while B102 FlashQuant runs in a sibling lane): Protocol B
  heal sweep of riir-dapps/src — the repo was NEVER swept (created 08-20, postdates
  the Aug campaign; P8 wave recorded only "zero hits" there).** 16 edit sites / 23
  placeholder edits in src/kat/{crank,payreq}.rs (15 `use_self` — the Batch-89 rule's
  first live workspace yield — + 1 `match_bool` guard-free 2-arm). riir-dapps ff'd
  3eacd6c→5a55048 first (issue 010/011 hygiene + kat/ compose work landed upstream).
  Validation: dapps `cargo clippy --all-targets` 0 warnings + lib 25/25 + gated
  integration 25/25 (`kat_payments,kat_payreq,kat_staking` — the 4 kat test binaries
  are inner-`#![cfg(feature)]`-gated; default run shows "0 passed", the empty-binary
  class). Landed `b7f1fbe`, pushed. Zero residue post-heal; no proposer divergence
  surfaced (use_self + match_bool both first-shot correct). The `.plans/070` sibling
  plan (untracked, B102 IN FLIGHT) respected — kernel_opt corpus, AGENTS.md kernel
  lines, and the NAX/LDG/riders queue items all left untouched this cycle.
- **Next executable (cadence-gated):**
  - **~~Batch 99 candidate (queued 2026-09-02, Plan 067 / Research 119): wasmi v2.0 blog~~
    LANDED 2026-09-02** (see Last landed). Dual verdict consumed: the same source TRIGGERED
    the workspace wasmi 1.0 → 2.0 upgrade (riir-ai Plan 563 — 13 pins / 7 repos lockstep +
    fuel recalibration; that lane is riir-ai's, not this repo's) — **upgrade LANDED same day
    2026-09-02**: katgpt-rs `133664b9`, riir-ai `0b7cad41e`, neuron-db `22e8ff4`, game-sdk
    `c08c02b`, chain `2c3e0b9d`, mmorpg `b3aaac6` (closeout `146d50c0a`); unified on one
    `wasmi 2.0.0`, zero API breakage, gains 2.0–3.3×, Bench 851.
  - **~~Queued-secondary (kill-free): `hot-dispatch-gpr-arg-budget`~~ — probed Tie on
    windows-msvc x64 (1.009/0.999×, two runs; the blog's evidence is sysv64 tail-call
    dispatch). Re-open ONLY with a sysv64 measurement window; ABI-specific,
    interpreter/VM-loop-shaped pattern.** Related: B101's x86 re-gate DONE 2026-09-04
    (ParetoDominant 3/0/0, expected-Tie REFUTED — the runtime-table fixture loads
    in-loop on both arches; see Plan 069 + the corpus annotation).
  - **~~Batch 100 candidate (queued 2026-09-02, Plan 068 / Research 120): Layr-Labs/qwen-3.8-mtp-challenge~~
    LANDED 2026-09-02** (see Last landed — 8 rules, not the verdict's 3–4 estimate: the two verification
    lanes surfaced 3 novel rules beyond the planned R1-R5). **Deferred-with-reasons ledger carried in
    Research 120's appendix — the strongest reopen candidates:** hidden fast-path footprint charged
    to the admission ledger + refuse-on-half-budget (CompiledDecodeV2; reopen on the next
    qwen-3.8-mtp-challenge-adjacent batch or a paged-KV admission consumer) and indivisible-attention-
    span chunk snapping + one-shot starvation guard (SchedulerV2; reopen on a multimodal or span-atomic
    chunked-prefill consumer).
  - **~~Unnumbered candidate (probe-only, 2026-09-02 — Research 119 §Parallel-session addendum):
    `dispatch-handler-pair-hoist` (rust_perf/dispatch)~~ LANDED 2026-09-02 as Batch 101
    (Plan 069, corpus 98 → 99, dispatch 4 → 5; validator ParetoDominant 3/0/0 — 2.89×/1.21×/1.23×;
    x86 re-gate ARMED in Plan 069 — 4090 contention at batch time).** Original candidate row: The aarch64 complement of Batch 99's landed
    rule: the merged conditional-dispatch form keeps an IN-LOOP indexed handler load on aarch64 (asm-pinned,
    `csel` offset + `ldr` + one `blr`; the classic `HANDLERS[op](..)` per-dispatch idiom), and hoisting the
    handler pair once (register-resident select) measured ParetoDominant 2/0/0 — 1.19-1.23x at 10K/1M
    dispatches on UNPREDICTABLE streams (structured/periodic = Tie band; verdict-stable x3, 0/0 allocs).
    NOT landed with Batch 99 (collision yielded push-wins) — needs its own dedup cycle vs the landed
    `mir-destprop-branch-site-merge` (distinct mechanism/fixtures: value-returning + runtime table vs
    tail-call + known items; the landed rule's negative control marks the runtime-opaque regime
    out-of-scope) + a cross-arch re-gate (x86_64's merged form uses `lea rip+h_*` — no in-loop load — so
    the hoist win may be aarch64-shaped). Batch 99's own split-vs-merged negative control (0.95-1.00x on
    runtime-opaque targets) does NOT cover this transform: splitting re-merges structurally on aarch64,
    but the hoist still wins by eliminating the load.
  - **~~NEXT UP (UNBLOCKED by B102's landing): Plan 072 (Research 122) — youssofal/MTPLX
    PR #391 (davidtai Qwen3.8-Flash-Next restack) — W87-led kernel_opt batch, 2–3 rules
    + riders, MAY COMBINE (default COMBINE) with the queued NAX leads~~ — LANDED 2026-09-04
    as Batch 103 (Research 124: 4 rules incl. the NAX backing-provenance lead + the B100 rider;
    T5 graph-build overlap dropped with reason; T7 COMBINE executed; ~~the dispatcher-eligibility
    predicate lead is DISCHARGED as fix-space guidance inside the request-shape rule's engagement-
    canary complement — no separate corpus rule~~ — **OVERRIDDEN same day by the Issue-665
    collision addendum (07fa0df): the lead LANDED as the 5th rule** (`dispatcher-eligibility-
    predicate-lags-kernel-capability`, D, corpus 463 -> 464) after push-wins adjudication —
    the "discharged" call was an oversight (the lead sat in no drop ledger; the landed decline-
    mirror rule cited its fix-space without the rule existing), see Research 124 §addendum.** Original queue row: (`backing-provenance-
    causal-shadowed-extent-widen` — the MLX 846 rule-1; `dispatcher-eligibility-predicate-
    lags-kernel-capability` — the MLX #4416 rule-2) into ONE post-B102 kernel batch (one
    corpus cycle, one GOAT run, one count-pin walk). Verdict: YES grade B as a mining source,
    MARGINAL for prefill throughput — every retained prefill lever is HAVE / refuted / no-analog
    in-stack (Bench 805 mask-fuse twin, Bench 710 wider-chunk refuted, Issue 734 GDN-chunked
    negative, Issue 742 prefix cache). Candidates: `shape-class-keyed-capability` (A− lead — a
    one-shape capability refusal disarmed the flag process-wide; key by the full shape tuple,
    re-ask per call), `wavefront-overlap-pair-ceiling` (B — 2×2 wavefront ⇒ 1 pair ⇒ 19.2%
    ceiling vs the 15% gate), `request-shape-decline-mirror-completeness` (B — the fast path
    mirrors every request-dependent decline branch; two HTTP-500 outages); W67 graph-build
    overlap evaluate-then-likely-drop. Pin `ef96313…` (fork davidtai/MTPLX-1, Apache-2.0,
    unmerged; re-pin at T0). Also queued for the next B100 touch: the NAX hd-256 dsplit
    rider + the #4380 GQA-12/16 table (B100 rider enrichment).
  - **MTPLX RESIDUE (queued 2026-09-04, cadence-gated behind the B105 defers) — LANDED 2026-09-05 as Batch 106 (Research 129 / Plan 076): 5 rules, corpus 477 → 482 (D 106 → 111); the B+ lead CONFIRMED with both instances in-source; see the Last-landed entry above.** Batch 103's population was the PR #391 diff + body;
    upstream `youssofal/MTPLX` main has NEVER been mined (**196 commits / 300 files** ahead of the
    mined base `8bc4d88`, `behind_by` 0; v2.10.2 09-01 + v2.11.1 09-04; head
    `dd4031a8` 2026-09-04T10:25Z, Apache-2.0), the `mistakes/` post-mortem corpus is **16 files
    upstream vs 9 in the vendor clone** (7 upstream-only), and `docs/plans` 1217 / `docs/specs` 757
    / `docs/concurrency` 259 lines are unread. **Lead (B+):
    `prefix-reuse-key-must-mirror-the-restore-predicate`** — TWO independent instances in one
    source (HEAD `dd4031a8`: the 2.10.2 admission shed asked `SessionBank.longest_prefix` exact
    containment while restore serves by block prefix ⇒ 41,901 tokens re-prefilled cold, 54 s, with
    a 41,391-token restore available; + the trailing-sentinel bank-fingerprint mistake, the
    mirror-image direction). Sibling of the landed `shape-class-keyed-capability` one layer up, and
    **the first MTPLX row with an in-stack analog** (Issue 742 GDN+KV prefix cache, 441× TTFT) —
    every prefill lever in R122's table was HAVE / refuted / no-analog. Axis B is the best
    confidence-per-token (filenames ARE the rules): `git-stash-is-shared-by-every-worktree` is an
    external confirmation of our own stash rule + the `hash-object`/`update-index` fix pattern;
    `mean-tps-...-p99-inter-emit-gap` censuses delivered-cadence silence, an axis we do not measure
    at all. Blockers: the vendor clone is **depth-1 shallow** (`a5e38bb` + `ef96313`) so it cannot
    serve the upstream axis — fresh pinned clone required; `.research/.highwater` reads 127 while
    128 exists (stale-highwater class, repair at allocation). Index + anchors:
    `.research/122` §Residue (SCOPE-CORRECTED 2026-09-04). Execution = Research 129 / Plan 076,
    axes A+B in one corpus cycle (B103/B105 COMBINE precedent), axis C riders only.
  - **The arxiv pool re-walk #4: DONE 2026-09-02 ~07:15 +07** (`.research/113` — covered
    the Tuesday-09-01 announcement window; zero corpus delta; the spec-dec lane woke up:
    DBloom's ceiling-bin acceptance histogram (zero-cost telemetry adoption candidate for
    the verify-loop benches) + compressed draft-side KV (riir-train route) + a rigor-matched
    audit paper (measurement doctrine); two doctrine citations (G4/alloc budgets,
    modelless-first). **Next window 09-03+** (daily differential cadence; MCP quota 09-15).
  - **The arxiv pool re-walk #6: DONE 2026-09-03 (4090 box, read-only lane while the
    Plan-371 GPU window runs)** (`.research/121` — Wed-09-02 batch differential;
    strict window NOT FIRED: only AceSpec 2609.02514, C). **Walk yield = the NEW
    GPU-kernel query surface** (`abs:"GPU kernel"` × DC/PF/AR), which out-recalled
    every category-listing surface. **BOTH queued source-verifications DISCHARGED
    same day (`.research/121` §discharges):** **2608.15764 (A−) CONFIRMED** — H100
    plain-load bandwidth peaks at K≈2 outstanding loads/thread, −35% by K=8
    (fixed-work + order + 2-die + allocation-sweep + ncu-controlled; evidence
    quality unusually high); Ada transfer OPEN (paper H100-only; Blackwell ≠ Ada);
    mechanism explicitly UNIDENTIFIED by the authors (only the operational curve
    distills); cp.async axis COMBINES with B92 (equality at 1 group, 2.1–2.9× at
    16, shrinking with occupancy — the occupancy×depth WHEN-to-use axis B92 lacks).
    **BATCH RESOLVED ANNOTATION-ONLY 2026-09-04 (T0a ran in the post-matrix window on this box): the H100 knee does NOT transfer to Ada** — sparse-grid bandwidth saturates at K=2 (951.6 GB/s median, 94.4% of nominal) and stays FLAT through K=16 (−0.3%); no −35%-by-K=8 decline in either grid; K=1 only −1.7% below plateau; verdict-stable ×3 (pooled medians n=9/cell). Shape (b): NO rule landed (a depth-cap STOP rule would be WRONG on Ada); B92 gained the Ada-neutral measured rider (corpus prose only, kernel_opt stays 466); T2 MOOT by precondition. Plan 073 all tasks closed; instrument + raw CSVs at `scripts/ldg_probe/`; Research 121 §2608.15764 has the RESULT addendum. T0b stands: decode GEMV at natural K≈2 = optimal either way. Original armed-batch record: plan filed 2026-09-04: `.plans/073_offered_load_knee_ldg_probe.md` — T0b GEMV K-audit DONE in-plan, findings recorded there; headline: the production dp4a decode GEMV already sits at natural K≈2 = the paper's H100 optimum, so the decode surface is a likely NEGATIVE case for depth-adding levers and the T2 rider's home narrows to the prefill staged-tile kernels — the T0b audit pre-read that guided the probe design; superseded by the RESOLVED header above). **2608.15531 FlashQuant: headline REFUTED for
    direct adoption** (INT4+BF16 weight outliers unreachable from ternary/Q4_K;
    batch≥8 regime — pads to 8, advantage "diminishes for very small batches such
    as 1 and 2" — not our batch-1 decode); **mechanism KEPT QUEUED** as a
    paper-distill batch candidate: heterogeneous-EXECUTION-UNIT warp specialization
    (TC dense ∥ CUDA-core sparse over one tile space, Stream-K, disjoint
    output-column ownership via LPT heap) = the additive-heterogeneous-paths member
    of the B58/B98 same-input family; dedup surface large (B79 plane-split, B98
    moe-sum, B15) → needs its own plan-first session. **2608.12700 (A−) = doctrine
    citation, no batch** — contract-grade kernel verifier, 1,487/14
    standard-test-vs-verifier disagreement over 2,638 kernels = the external measured
    confirmation of the Bench-802/Research-116 fault-class doctrine; cite in the
    lossy-surface + tolerance-budget surfaces at the next doc-sync touch. Riders
    queued (land WITH the next adjacent batch, never standalone): Bole 2608.01651 →
    B82 (tree-verify state-free external confirmation, memory-side win measured),
    rl-triton 2608.17641 → B79 (associative-scan breadth: 7 RL estimators as ONE
    first-order recurrence). Lane records: DAMP 2608.27513 (B− GDN state-quant, the
    DASC group; the recurrent-state surface consolidating), H-Scale 2608.28113
    (the quant scale-AXIS triplet completed: 845-PoT / OCGQuant-grouping /
    H-Scale-value), sigmoid-eviction 2608.23296 (external sigmoid-not-softmax
    confirmation in the learned-eviction regime). Walk-#5 miss corrected honestly:
    SMELT 2609.01343 + mzCache 2609.01338 sat inside walk #5's Tue-batch window
    (09-01T14:4xZ) unrecorded — both C, cheap miss; walk #5's query set doesn't
    reach training-scaling/mobile-memory papers. Standing lanes for walk #7:
    spec-dec (unrestricted), KV+eviction, quant+inference, GPU-kernel (permanent),
    DC/PF listings. **Next window Fri 09-04 ~07:00 +07** (MCP quota 09-15).
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
    **GPU G1/G3 LANDED + PASSED same day 2026-09-02** (4090-exclusive window, Vulkan
    wgpu<spirv>; gate `crates/riir-gpu/tests/issue845_pot_scale_gpu_g1.rs` — fold invariance
    0 diffs on 3 Bonsai shapes, PoT-shift covariance 0 violations; structural scoping:
    f32-accumulator fold-invariance out of reach → the integer-dot slice landed as riir-ai
    Issue 850 / Bench 850, all four gates PASS). Lever RESOLVED-SCOPED; `pot_scales` opt-in,
    no consumer; q8kv slice + trit-tier mirror stay deferred (850 T4 scoped the q8kv
    discriminator to the f16 STORAGE site). Plan 586 + this line reconciled 2026-09-03.
  - **The arxiv pool re-walk #7: DONE 2026-09-05 ~04:15 +07** (`.research/125` —
    zero corpus delta). **Strict differential EMPTY — the frontier has NOT advanced**:
    measured 09-04T21:15Z, the whole API tops out at submissions 09-02T17:59:46Z
    (cs.LG probe) and the listing's newest announcement group is still "Thu 3 Sep";
    walk #6's predicted Fri-09-04 ~07:00 +07 batch did not post (next observable
    ≈ Sun 09-06 20:00 ET → **walk #8 window Mon 09-07 ~07:00 +07**, the
    09-02T18:00Z→09-03T18:00Z submissions rolling into it). **The walk's yield is
    the BACKFILL triage**: the broader standing queries surface 12 rows inside
    walk #6's own strict window of which walk #6 recorded ONE — root cause:
    walk #6's per-lane queries were narrower than the standing set (KV+eviction
    vs plain KV; quant+inference misses pretraining/capability rows). Graded:
    **2609.01962 Post-Training Ternarization of Qwen3-4B (B− doctrine — per-task
    retention spread BoolQ 84.6% vs ARC-C 43.8% under a −9.8pp aggregate = external
    lossy-surface confirmation, CITED into the katgpt-rs rule this walk;
    effective-bit accounting 1.641 vs nominal 1.58 = the honest-accounting analog
    for our ternary GGUF claims; lossless packing 8.29→3.96 GiB unchanged-PPL vs a
    LOSSY third-party packing excluded — the BLAKE3-committed-packing echo)**;
    2609.02029 HeadWiseKV (B− lane record — per-head STATIC residency for hybrid
    global layers, materialize-residency-not-mask; **out-of-stack: Issue-746
    append-only KV prohibition**); 2609.02846 UE5M3 FP4 (C+/B− doctrine — drops RHT
    + BF16 final-layer exemption, +21.2% throughput at lower loss = recipe-level
    ROTATION-TAX echo beside our Bench-716 refutation + LightRot/GyRot; the
    scale-FORMAT axis again: Issue-845 PoT narrow-deterministic vs UE5M3 wide);
    2609.02027 LRU mean-field (C+, serving-capacity theory); 2609.02052 SchedBlame
    (C+ — culprit-attributed CPU contention via eBPF = the principled version of
    Bench-649/`watch_midrun` victim-side gates; cite in the league doc at next
    touch). **Walk #6's queued 2608.12700 citation (contract-grade verifier →
    lossy-surface/tolerance-budget) DISCHARGED in the same docs commit.**
    Tooling note: the `fetch` MCP tool strips Atom `<entry>` bodies and its
    submittedDate range filter returns 0 through both encodings — walks go via
    `curl` + unbounded lane fetch + local `<published>` window filtering.
  - **09-15 MCP search quota reset** → online perf-search pool walk (queue item 11).
  - **2026-09-04 (M3, idle unit): Protocol B sweep of seal-remake — the repo was NEVER
    swept (created 09-01).** Zero own-crate findings: `cargo clippy --workspace
    --all-targets` clean (isolated /tmp target; the only warnings are the sibling
    katgpt-rs lib's known `logit_scratch` + riir-viewbridge-core build-script info
    lines — the landing sessions' gate discipline held). Nothing to heal; no commit.
  - **2026-09-04 (M3, idle unit): riir-ai Issue 866 FIXED** — the perf-rematch
    contention detector's two inline basename allowlists missed released cargo
    examples (digit_ood_eval held the M3 GPU 29 min while the gates read quiet).
    Fixed riir-ai `acd443b9a`: committed vocabulary file (contender_names.txt, 19
    patterns, empty=hard-refusal, floor pinned) + resource arm (≥80% CPU AND ≥2 GiB
    RSS, RIIR_CONTAM_* knobs, Darwin/Linux) + legible negative ([scan-stats],
    enforce_midrun tag contract pinned) + 32-assertion fixture selftest
    (perf_rematch_detector_selftest.sh) + en-route wrapper/VAR= interleave normalizer
    fix. Validated LIVE first-run: caught digit_ood_eval in the wild (589 scanned, 21
    matched, gate refused, both arms). Issue file removed at close; durable note in
    the league doc rule 4.
  - **2026-09-04 (M3, idle cycle 7, unit 1 — hygiene): katgpt-rs Issue 717 file
    REMOVED** (`0187b384`) — status CLOSED 09-03, record `.benchmarks/699_lt2_deep_loop_
    stability.md` carries T7-defer + Plan-373 contingency; zero external refs to the
    issue path. katgpt-rs issues re-triaged: 718 (T3 owner cost call), 719 (T2-T4
    trigger-gated), 721 (T3 owner sequencing) all gated; chain 095 (multi-repo
    extraction, unclaimed), 101 (constraint note), 110 (needs ESP32 silicon) not
    idle-loop units.
  - **2026-09-04 (M3, idle cycle 7, unit 2 — mmorpg re-sweep with the post-Batch-90
    fixable set, mmorpg `bd4a4f2`): 54+3 edits / 17 files** — first re-sweep of the
    08-28-swept repo under the new rules (use_self/manual_let_else/items_after_
    statements); yield = items x6 + manual_let_else x4 + match_bool x2 + map_unwrap_or
    x3 + uninlined bulk; chain/dao/auth/viewbridge dry-runs all ZERO (their fixpoint
    holds). **Drive-by: container-tick-test was UNBUILDABLE** — no [workspace] table +
    not a parent member → every cargo command refused ("believes it's in a workspace");
    fixed with the warm-tier-do self-rooting precedent. **The healer's verify gate
    cannot handle standalone-workspace-inside-repo layouts** (it ran the parent-context
    cargo check over the excluded crate and reverted the whole first batch) — heal the
    standalone crates in a SECOND scoped pass after their roots resolve; noted as
    tooling follow-up **— ROOT-CAUSE DIAGNOSTIC LANDED next cycle (this repo, lib test
    1035/0): `compile_oracle::workspace_belief_cause` + `enrich_cargo_run_error`** —
    when cargo's "believes it's in a workspace" signature line fails a baseline/loop
    check, the error now names the memberless nested manifest + the enclosing workspace
    + the self-rooting remedy (wired into BOTH CheckRunner::run_check and ::run_clippy;
    membership itself deliberately NOT parsed — ancestor members lists accept globs, a
    false "you are excluded" would be worse than none). 2 unit fixtures + e2e
    `fix_verify_e2e_workspace_belief_refusal_names_the_remedy` (real cargo temp layout:
    fail-closed BEFORE any write + enriched message). **build.rs map_or terminal-fix recurred** (the Unknown-receiver
    call-expr conservative emission — `git_out(..).map_or(false, |s| ..)` →
    is_some_and by hand) — **ROOT CAUSE FIXED same cycle, riir-clippy `609d7267`:
    map_unwrap_or same-file fn-signature resolution** — Collector fn_rets map
    (fn name → Option/Result from its RETURN type), classify_receiver consults it
    for bare-call receivers (qualified path OK; foreign calls/method receivers stay
    Unknown); 4 fixtures (45 total); e2e proof on the exact build.rs shape heals to
    is_some_and first-pass; score-bench #35 flat (96.2/1.9/100); lib 1033/0.
    Standing residual: shadowing-let misclassification accepted (parse-only;
    --verify reverts E0599). Validation: default lib 441/0, e2e_topology 12/0,
    warm_tier_e2e 3/0, bench_remote_smoothing 8/0, bench_game_sync 2/0/2i, taming 2/0
    + hero_fsm 1/0 (bevy_backend), out_of_box 1/0 (scenario_runner,social_signals,
    pet_pvp), duo 18/0 release (+scenario_runner), warm-tier-do contract 4/0,
    clippy 0 own-crate both states.
  - **2026-09-04 (M3, idle cycle 8): two units.** **Unit 1 — hygiene follow-through,
    mmorpg `97d60eb`:** container-tick-test's generated Cargo.lock TRACKED + a nested
    .gitignore (/target, /node_modules, /.wrangler — the warm-tier-do precedent; the
    repo-root .gitignore is root-anchored and never covered nested crate targets) —
    closes cycle 7's noted-not-fixed leftover; staged-set audit clean (2 files, both
    mine). **Unit 2 — the cycle-7 verify-gate tooling gap CLOSED, riir-clippy
    `97f4ca32`:** see the inline closure on the cycle-7 entry above
    (workspace_belief_cause + enrich_cargo_run_error; lib 1033→1035/0; fix_verify e2e
    2/2 incl. the new workspace-belief fixture; clippy 0; no-dep check clean; GLOBAL
    BINARY REBUILT release fix_verify,clippy_verify per the AGENTS.md rule). Gates
    re-checked at cycle start: T0a STILL GATED (no t504_confirm/status.txt — the 504
    T3 harvest lane is actively running; 4090 quiet at 5%/617 MiB but the gate needs
    ALL_DONE, not just a quiet box); walk #8 → Mon 09-07; MCP pool → 09-15; re-sweep
    surface still closed (riir-train 8 / riir-neuron-db 46 / riir-game-sdk 7 dirty —
    sibling lanes: 509/digit_ood, a src-wide sweep, riir-e2e NFT; riir-ai heavily
    dirty). The nearest_rank-copy housekeeping lane is ANOTHER AGENT's — not touched.
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
  - **MINED 2026-09-05 — Batch 105 LANDED (Research 127 + 128 COMBINED / Plan 075): 11 rules, corpus 466 → 477 (A +1, C +1, D +3, E +1, F +1, H +3, M +1; entries_gpu_18.rs).** The combine mandate executed: both sources mined in one corpus cycle with the cross-source pairs cross-referenced IN-rule (indirect-tile-map vs capacity-bound-sentinel = opposite postures, both landed; lane-sequenced vs atomic scatter = determinism contrast, both carried). Mine-time falsifications: uzu "RELAXED everywhere" REFUTED (strict-layout override arm on tree_verify); the "Hadamard of a constant column" comment DOES NOT EXIST in-tree (quote killed, the bias-demotion code is the evidence); lily tile policy is CHUNK-SIZE-routed (32/64 rows, both 4 simdgroups) NOT expert-load-routed — the B77 rider framing above is corrected; family-gate refusal text says "family 10, M5 and later". The lily GOAT lead (concurrent-dispatch) landed corpus-side with its M3 A/B fixture DEFERRED (Plan 075 T7 — GPU lane claimed by 851 Phase A; re-arms when quiet). Batch 106 slot MERGED into 105 (the next batch takes 106). Deferred candidates (recorded for a future batch): uzu placement-sparse-demand-paged-kv (B+), gumbel-argmax-unified-sampling-descent (B+), multi-field-argmax-tuple-reduction (B−), flat-topology-fast-path (weak), integer-mma-epilogue (B−), fused-count-scan (C+), radix-select (B−); lily chunk-command-buffer-prefill, cross-level-fused-residual-norm, RIF-sentinel, tensor-ops-device-q, load-time-concat-rows (= B63 external instance), weight-hygiene cluster. Upstream hazard found: uzu counts_offsets_fused.metal counts_shared[128] OOB writes for 128 < E ≤ 512 (threadgroup OOB, CPU ref unbounded — report to trymirai). Gates: batch105 3/3, kernel_heal 1338/0/1i, default 1076/0, retrieval floors HELD, count pins walked (tests.rs totals/expected-array/sums/grouped + router.rs + mod.rs member lists + 7 category comments).
  - **(verdict record, now mined — Research 128 — perplexityai/pplx-garden `lily/` @ 1ed972ed3f0bd5616c997c9507c25616c63394fc, Apache-2.0 — Perplexity's Rust+Metal Qwen3.6-35B-A3B engine, blog carries MEASURED matched ablations):** YES grade **A−**, route kernel_opt (ds4/B97-98 posture — M5+/family-10 runtime gate, read-only on the pinned M3). Two A− leads with ZERO corpus coverage: `gpu-resident-moe-routing-no-host-sync` (histogram→scan-offsets→scatter→tile-map in ONE command buffer, CPU never inspects routing intermediates, blog +89% prefill @512; distinct from B97 map-building — residency/sync-boundary is the decision) and `concurrent-dispatch-dependency-level-barriers` (MTLDispatchType::Concurrent decode pass, level_barrier at every true data edge, 795 kernels → 555 stages; distinct from B26 in-order merge, B70 counters, B100 fence-values; **the GOAT lead — benchable on M3, Concurrent dispatch is Metal 3.1-era not M5-gated**, riir-gpu's raw-Metal lane is the fixture vehicle). B+ pair: `pipelined-token-slot-ping-pong-submit-ahead` (U32[2] next_token slots, argmax writes step N+1's INPUT, submit-before-wait across two command buffers; distinct from B25 argmax-download + B58 pool rotation) + `split-k-two-pass-attention-decode-long-context` (FixedBlock route ≥32768 with exact_model_shape gate, MLX sdpa_vector_2pass_1 adapted; blog +7.7/27.4/40.2% @32K/64K/128K; zero corpus coverage). Riders: B16 dequant-in-tensor-op-GEMM (+77.4% external measured), B77 tile-by-expert-load axis (+13.2%), B100 GQA-fold table row hit a THIRD time (d256→hpt4 @KV≥8192; MLX+MTPLX+Lily), B95 GDN register-scan (same mlx-lm upstream), B62 register-A skinny router. Negative (league): spec-decode −18% batch-1 (THIRD independent confirmation of Issue 717/721), decode weight-read-bound ±0.2% (Bench 763/771), orchestration-not-kernels is where the headroom was (roofline 90-97% on M5). pplx-garden's RDMA surface (fabric-lib/p2p-all-to-all/gdrapi): **NO** — cluster domain, zero benchability; mine `lily/` only. **Mine-time mandate: cross-reference Batch 105/uzu in the dedup gate** (same model family, both MLX-derived — uzu lane-sequenced-scatter vs Lily atomic scatter, uzu indirect-tile-map vs Lily tile-map are likely one family with different determinism postures); combine 105+106 or sequence strictly. Repo-shape trap recorded: the zread index was STALE pre-#19 (showed RDMA only, no lily/) — verify repo shape against the commit graph, never a cached structure fetch. Plan-first at batch open. Research 128. YES grade **A−**, route kernel_opt; 2 inspection lanes DONE (A: matmul/GEMM/MXU/split-K/RHT/qmv; B: attention/MoE/GDN/SSD/radix-topk) + 16 quotes coordinator-verified + dual-vocab dedup DONE. **Leads: `indirect-device-dispatch-tile-map-moe` (A− — device-computed grid via MTLDispatchThreadgroupsIndirectArguments, ragged MoE M-axis never round-trips to host; zero corpus coverage), `cooperative-tensor-mma-capability-route` (B+ — Apple MPP cooperative-vector matmul2d, supportsMXU fail-closed probe, M5+/Gen18+ next-silicon, B103-NAX posture), `rht-epilogue-fusion-contract` (B+ — output-RHT fused only when split_k==1, bias demoted post-RHT ("Hadamard of a constant column is not that vector"), requires quantized W; composes with B98), `target-tile-split-k-deterministic-two-dispatch` (B — quant-class target-tile table + quant-group alignment ladder + TWO-kernel fixed-order reduce; two-regime vs B92 semaphore), `a8-activation-route-stability-guard` (B — re-run dispatch with a_full_precision=false, adopt Int8 only if still MXU-tier), `lane-sequenced-deterministic-bucket-scatter` (B — atomic-free deterministic MoE permute, 32-step lane lockstep; distinct from B97/B102/B94), `packed-nibble-tensor-operand-alignment-gated` (B — W4 consumed natively by the tensor unit when aligned).** Riders: B49 ×2 (SSD runtime-bounded lane_states + the state-64 specialization; METAL_PRAGMA_UNROLL ignored under metal4.0 -O2), B97 frontier-clamp instance, B103/B100 GQA-M-packing + route-ladder instances, B104 tg-per-core occupancy-target table + qmv 300-row unanimous-family-consensus table (1176-decision xxh3 fingerprint), B98 GEMM-over-GEMV carve-outs, LinearAttn state-resident-chunked instance (MXU_MIN_T 256 / CHUNK 64; chunked MXU-gated so M1–M4 run plain recurrent — corroborates our Issue 734 negative), Morton counter-regime note on l2-grouped-tile-schedule (4× padding-budget gate). Drops: Morton-as-rule (covered), attention ladder standalone (B104), chunked-GDN standalone (B79/B95/Issue-734), weaver/speculators/sparse (unread, decode-side or flag-at-mine). Caveats: NO committed perf numbers in-tree; MXU = M5+ only (zero pinned-box effect); quotes re-verify at mine time. Mined as Batch 105 (combined with Research 128; see the LANDED note above). Research 127.
  - **NEW 2026-09-02 late (league opponent watch #5, M3 lane — zero perf movement on the
    fork, upstream M3 Max fa-vec tuning LANDED):** fork `origin/prism` moved only 3
    commits past the pin `e311ed38f`, all docs (CONTRIBUTING/AGENTS ×2) — **the pinned
    opponent binary stays valid, no re-pin**. Upstream ggml-org since 08-25: ~26 Metal
    commits, ONE load-bearing for this box — `8963a9bdc` (08-28, #27863) adds
    `fa_vec_tuned_table` rows for **M3 Max** (F16 + Q8_0; 64 GB MacBook Pro, low-power
    mode) to `ggml-metal-tuning.cpp` — AFTER the fork pin, fork does not carry it.
    League reading: (a) all scored cells remain FAIR (both sides untuned at measure
    time); (b) the opponent bar is now POTENTIALLY STALE — any future re-pin at
    upstream absorbs per-(device, dtype, head_dim, ne11/ne01-bucket) fa-vec launch
    tuning and the attributable-delta rule applies to every flash-attention cell;
    (c) the fork policy (read-only vendor binary, owner call Issue 782) is unchanged
    by this — record only. **Distill lead (kernel_opt, mechanism class):**
    ~~`fa-vec` tuning-table dispatch — per-device/per-dtype/per-(dk,dv)/per-context-bucket
    swept launch-parameter tables for the flash-attention vector kernels + the
    `ggml-metal-tuning fa-vec` autotune sweep harness that generates them — vs our
    two-point length gate (m16 <8192 ≤ / m32 ≥8192) on the tiled flash; the swept
    multi-bucket axis is the richer shape.~~ **MINED 2026-09-05 — Batch 104 (Research 126 /
    Plan 074): 2 rules, corpus 464 → 466 (K 5 → 7); see Last landed above.** The rest of
    the upstream Metal movement is
    the same mechanism for OTHER chips (M1/M1 Max/M1 Ultra/M2/M2 Pro/M2 Max/M3 Pro/
    M3 Ultra/M4/M4 Pro/A18 Pro — ~15 commits) + `d5d993a09` Metal 4.0 tensor API on
    M5+/A19+ (next-silicon external confirmation of the matrix-core-attention family
    note in Issue 831's disposition) + `2a74817f9` top-k radix (sampling path, not our
    hot loop).
  - **VERDICT 2026-09-03 (this session — the 846 deferred intake, kernel-reading
    extraction from `../vendor/mlx` @ f99e916; the riir-ai 844 reading extract's
    two candidate rules verified against the ACTUAL kernel bodies at
    `steel/attn/kernels/steel_attention_nax.h` 907 lines, full read):**
    - **Candidate rule 1 — causal-shadowed padding read for tile alignment
      (`has_backing_rows`): CONFIRMED NOVEL, kernel-grade, corpus-worthy.** The
      orchestrator-level reading held: kL widens to the next bk multiple IFF the
      K/V slices provably back the padded rows (negative-stride reject + row-pitch
      containment `row0 + rows <= pitch` + buffer-bounds end check); padded rows
      sit beyond every query's causal horizon (`qL_off = kL − qL` preserves the
      true diagonal), garbage reads are shadowed by the causal mask, and the
      ragged-tail masked-store branch is DELETED for the common chunked-KV-cache
      shape. Dedup verified: `has_backing_rows`-style backing-provenance is
      ABSENT from the corpus (B84 pad-tail PADS THE DATA via a scratch copy +
      strip-before-stateful-consumer — a different mechanism entirely: data-side
      padding with a crossover tuner vs extent-side widening with read-provenance
      proof and NO data movement; B17 clamp+discard keeps the guard; B100
      early-return derives extent from row metadata — none carry "prove the rows
      exist behind the slice, then read past the logical extent under causal
      shadow"). The provenance check itself (stride sanity + pitch containment +
      buffer end) is portable to ANY engine whose KV slices view a ring/cache
      allocation. Proposed id: `backing-provenance-causal-shadowed-extent-widen`.
    - **Candidate rule 2 — hd-256 head-dim split (`attention_nax_dsplit`):
      CONFIRMED as the B100 external-confirmation instance** (`register-budget-
      head-split-gqa-reread-trade`), exactly as the 844 extract predicted —
      split the head dim across WN=2 simdgroups of the second warp dimension,
      halving the accumulator set, K/V re-read per half, redundant-per-half
      softmax with ONE smem exchange of the partial Q@K.T fragment pairs
      (`s_xchg[WM][WN][2*kEPF*32]`, both halves share the frag-to-lane mapping
      so the exchange needs no coordinate math). Fold as an instance/rider at
      the next B100 touch, not a new rule. Two EXTRA specifics worth pinning in
      the fold: the Q-half register-residency for the whole KV loop
      (`NAXTile<T,1,1> Qtiles[TDh]` hoisted OUT of the loop — the TDh fragments
      are "cheap next to the accumulators"), and the exchange reducing EXACTLY
      ONE peer so WN is structurally fixed at 2 (static_assert).
    - **NAX-gating boundary re-verified live:** `use_fallback`'s widening branch
      AND both NAX kernel routes require `metal::is_nax_available()` (M5-class
      per-core neural accelerator) — absent on M3/M4, so the widening cannot
      move our pinned M3 opponents at any version (the 0.32.2 reference stays
      structurally unaffected; version-pinning applies to M5-class cells only).
      ALSO unfused on M3: hd-256 falls to the decomposed path at seq > 8
      (`return query_head_dim == 192 || query_head_dim == 256`), and the
      non-NAX steel kernel's shape table tops out at bd=256 with bq32/bk16 —
      never compiled for the Bonsai shape's fused use.
    - **Disposition:** rule 1 QUEUED as a next-batch kernel_opt lead (batch
      ~B102; needs fixtures + GOAT per standing discipline — the pseudocode is
      host-side extent-widening + a provenance predicate, straightforward to
      fixture); rule 2 = B100 rider (deferred to the next B100 touch); nothing
      routes to the flash-vs-GEMM prize as a lever (NAX is hardware-unavailable
      on M3 — the prize stays KV-traffic amortization per T1). The 846 handoff
      item DISCHARGED at the reading level; fixtures own the rest.
  - **NEW 2026-09-02 late (MLX watch, `../vendor/mlx` 37c26e5 → 90846ad):** three
    Metal-kernel leads adjacent to our lanes: (1) `f99e916` #4416 — head-dim-256
    prefill with array mask now runs the FUSED kernel path (Bonsai IS hd=256 — read
    for what gated the fallback); (2) `685dba9` #4431 — `sdpa_vector_2pass_1_gqa`
    batch-offset fix for K/V in GQA decode — cross-check target for OUR split-K
    decode pair (Bench 831 `QwenAttentionDecodeGatedSplit{,Combine}`, G1 0/512 flips;
    their bug class = per-batch K/V offset in the 2-pass split — **VERIFIED CLEAN
    2026-09-03 (4090 box, read-only inspection unit, zero-contention while the
    Plan-371 window runs)**: our pair has NO batch axis (single-token decode —
    query `n_head*head_dim`, KV `n_positions*n_kv_head*head_dim`; sequences never
    share a buffer with an implicit batch stride), so MLX's bug dimension is
    structurally absent. The analogous axes all check out by inspection of
    `qwen_attention_cubecl.rs` (f32 split L948-1151, f16 split L1459-1688,
    combine L1160-1218, geometry L922-927) + the caller
    (`ternary_deltanet_gpu_forward.rs` L2387-2393 alloc, L4265-4300 launches):
    (a) GQA per-head K/V offset `kv_group = head_idx*n_kv_head/n_head` at
    `pos*kv_stride + kv_off + d` correct in BOTH twins (f16 word twins
    `kv_stride_words`/`kv_off_words` exact under the even-head_dim contract);
    (b) per-split position ranges disjoint + complete — hd-aligned split_len
    (`split_decode_geometry`), split i owns `[i*split_len, min(+split_len, n_pos))`,
    first_tile division exact, over-dispatched splits write EMPTY partials
    (m=-1e30/l=0/out=0 → w=0 in the merge; the stale-byte×NaN hazard guarded by
    design comment); (c) split↔combine partials layout identical
    `(head*n_splits+split)*(hd+2)`, BOTH launches take n_splits from the SAME
    geometry call, `new()` allocation uses the same 64-split cap (monotone-superior
    to any per-call n_splits; every combine read is written by the same call's
    split launch — no stale cross-call reads); (d) f16 K-dot q↔word pairing + V
    dim-pair ownership verified element-by-element, per-dim summation order
    matches f32. Boundary notes (NOT bugs): hd ≤ 256 is the family-wide smem
    ceiling (all 6 attention kernels allocate 256-float smem; tree/prefill kernels
    document it in comments, decode kernels imply it) — a future hd>256 model would
    OOB-write smem, pin with a debug_assert if such a model ever lands;
    `split_decode_geometry(0, ..)` would div-by-zero (unreachable — decode always
    has ≥1 position).; (3) `bbf8a7a` #4380 — widens the 2-pass
    GQA decode kernel to GQA sizes 12/16 (threshold data for when split-K pays —
    ours fires >512 pos regardless of GQA). Leads (1)/(3) were marked not-yet-mined
    low-priority — **both now DISCHARGED 2026-09-03, see the extraction unit below**
    (decode lane stays league-defended; prefill is the open prize — neither changes
    that); lead (2) DISCHARGED as the verification above — verdict CLEAN, no corpus
    delta, no issue filed.
    **Leads (1)+(3) DISCHARGED 2026-09-03 (4090 box, read-only kernel-extraction
    unit #2 — `git show` on the vendor clone, zero contention while the Plan-371
    window + B102 run):**
    - **(1) `f99e916` #4416 — CONFIRMED the dispatcher-drift rule shape, QUEUED
      as the second next-batch kernel_opt lead** (candidate id
      `dispatcher-eligibility-predicate-lags-kernel-capability`, name final at
      batch time). Mechanism, fully verified at source: #3842 (`714a7ef`) landed
      the hd-256 NAX fused path with the gate `do_causal && !has_arr_mask`, and
      the KERNEL carried full array-mask support from the SAME commit —
      `has_mask` function_constant(300), mask buffer bindings (5/6), in-kernel
      mask reads in both the main + dsplit kernels (verified in
      `steel_attention_nax.h` at 714a7ef). The `!has_arr_mask` conjunct was dead
      conservatism for the kernel's ENTIRE life: every array-mask hd-256
      seq≥1024 prefill silently ran the decomposed fallback — correct output,
      slower, NO test fails, profiler shows work happening. #4416 deleted the
      stale conjunct 13 days later (`(do_causal || has_arr_mask)`), touching
      ONLY the 2-line dispatcher predicate — zero kernel changes. The
      soundness evidence for any such widening lives in the kernel's
      SPECIALIZATION SURFACE (func consts / mask-typed kernel name), not in
      the predicate. Fix-space: align the predicate with capability + a
      POSITIVE ENGAGEMENT test (assert the fused path ENGAGES at every shape
      the kernel supports — output-parity tests cannot see this failure
      class). NAX-scoped evidence (M5-class), portable mechanism — our own
      capability-conditional gates (split-decode >512, tiled-flash/SMEM tiers,
      m16/cmma) carry engagement canaries from their GOATs, which IS the
      standing defense the rule prescribes. Dedup verified: B23
      hardcoded-fast-path (specializes KERNELS for known configs — not
      predicate drift), vendor-batched-gemm-contiguity-staging (external
      library's internal path chosen by LAYOUT — host fix is contiguization;
      here OUR predicate vs OUR kernel, fix is predicate re-alignment),
      aliased-slot-allocate-replace-fallback (a semantically WRONG fallback
      vs a merely-slower one), gate-prep-fold capability gate (loud REJECT on
      incapable backends — the inverse direction). Landing gated behind B102
      (kernel_opt corpus contention) with the NAX rule-1 lead.
    - **(3) `bbf8a7a` #4380 — B100 rider ENRICHMENT recorded** (folds at the
      next B100 touch beside the NAX rule-2 instance): the 2-pass GQA decode
      kernel adds gqa 12/16 instantiations (table: 64/g8/hpt8, 128/g8/hpt4,
      128/g12/hpt4, 128/g16/hpt2) — hpt FALLS as the GQA factor rises, the
      register-budget trade in the exact B100 direction, a SECOND external
      instance beside the NAX hd-256 dsplit. Dispatcher predicate generalized
      to `gqa_dims` (g8×{64,128} ∪ {g12,g16}×128); the 2-pass ELIGIBILITY
      length threshold is unchanged at KV ≥ 8192 — boundary note: MLX's 2-pass
      vector kernel pays only at 8K while our split-K flash decode fires >512
      (Bench 831) — DIFFERENT kernel families (their single-pass vector baseline
      vs our gated flash split), not a contradiction, do not cite as one. Our
      GQA factor is fixed so the g-axis does not bind for Bonsai — recorded as
      threshold data only.
    - **Watch pin + tooling note (same unit):** MLX origin/main re-verified ==
      `90846ad`, zero new movement since the recorded watch tip; the box's
      clone checkout sits behind-14 stale (local HEAD `c899c5a`) — refreshed at
      mine time per the `.raw` discipline, not by this read-only unit.
      Extraction-tooling: `git grep <rev> -- <wrong-path-prefix>` returns EMPTY
      exit-1 — indistinguishable from `absent at this rev` (this session first
      mis-read the NAX kernel as mask-less via a `metal/steel/...` vs
      `metal/kernels/steel/...` path slip; `git show <rev> --stat=200` prints
      full paths and is the disambiguator — the same silent-empty class the
      multi-root workspace grep warns about).
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
- **Sibling lanes (do not touch):** **riir-train — OWNER-RESTRICTED (2026-09-04): do not
  route NEW work to riir-train for any task; only riir-clippy-related units are sanctioned
  for this session lane.** Existing riir-train in-flight lanes stay with their owning
  siblings (e.g. `/tmp/riir-train-499b`); read-only citations of riir-train records are
  fine — never initiate fixes/gates/benches there. The 4090 perf lanes — sibling PV work
  mid-flight as of 08-29 (re-verify lane state at session start before any 4090 unit).
- **2026-09-03 (4090 box, second unit — plan-board truth sweep, zero-contention while the
  Plan-371 window + sibling B102 run):** katgpt-rs Plan 586 reconciled to RESOLVED-SCOPED
  (the GPU gates had PASSED 09-02 while the plan + the 845 line above still read
  "armed/GPU-window-gated" — a future session could have re-run a passed gate);
  riir-ai Issue 847 status line reconciled (T7/T8 were already `- [-]` defer-marked; the
  "Remaining" clause now names only the owner promotion call). Plans 582 (CLOSED-NEGATIVE)
  Plans 582 (CLOSED-NEGATIVE)
  and 584 (OPEN-DEFERRED with triggers) verified already-honest — untouched. Plan 530
  flagged for a future doc-sync pass (status "IN PROGRESS — pivoted to inference prefill"
  predates the Bench-641/653/730 prefill arc; absorption not verified, not marked blind).
  katgpt-rs ff'd 345e301f→f84f5c7e first (the 713/716 release-gate session's landings).
- **2026-09-03 (4090 box, third unit — doc-sync on the Plan-530 flag above, zero-contention
  while the resident riir-train GPU run holds the box): riir-ai Plan 530 reconciled to
  RESOLVED** (riir-ai `f5e8c430e`) — pivot arc CLOSED (Goal 2's ≥3× kernel gate superseded
  by the prefill arc: Benches 774/790/805 default-on, M3 prefill @2048 44.94→~105.78 tok/s;
  Goal 3 landed — batched prefill DEFAULT-ON via Issues 653/730); backward remainder VERIFIED
  LANDED cudarc-side in riir-train-gpu (`ternary_deltanet_backward_cudarc.rs` Phases 1–4,
  cites "Issue 641 T7.5", gate `test_gpu_backward_matches_cpu_reference`, consumer riir-train
  arm_c under `lora-gpu-backward` "T7.6") — tasks T6–T11 marked `[x]` DONE-AS-CUDA, T2–T5/T12
  `- [-]` superseded/void with reasons (Metal/CubeCL backward never built + now
  boundary-forbidden in riir-ai per Issue 741 / `ci_modelless_residue.sh` C6). The
  GPU-training-forward lane's live state = riir-train Issue 729 (open-loop, option-1
  deferred with reopen trigger). Flag consumed — no open doc-sync leads remain from the
  09-03 plan-board sweep.
- **Post-mining intake: 0 open classes** — P8 nan-unsafe float sort comparator CLOSED
  2026-09-01 (landed as sec `nan-unsafe-sort-comparator`, Plan 057; record below); all
  prior classes closed (records below; full text in the history note).
- **2026-09-03 (M3, idle loop — E4 decay-gate triage, the flat 35-run curve resolved):**
  the score bench has read 96.2/1.9 since 08-25 and the audit's E4 gate referenced the
  pre-P9 peak (98.1), printing a false decay context every run. Full miss-map triage
  (`per_fixture_diagnostics`): ALL THREE entries are documented, probe-verified
  boundaries — (1) `map_unwrap_or` persist on `riir-ai-ruliology-player-001` = the P9
  trait-object-cast mapper guard declining CORRECTLY (probe RE-VERIFIED 2026-09-03 on
  the current toolchain: the map_or rewrite still E0308s — expected `&fn(..)`, found
  `&dyn Fn(..)`; original compiles — the guard is sound, we-stay-silent is SAFER than
  clippy's autofix); (2) `useless_format` persist = the Run-#3 String-typed frontier
  (bench-symmetric trade, `created` is the trust axis); (3) `single_option_map`
  created = clippy's own cascade on `manual_map`'s output. **The 98.1→96.2 step was
  DENOMINATOR GROWTH (the P9 fixture deliberately frozen as a declined lint), not
  healing decay.** E4 re-pinned in `scripts/self_evolve_audit.sh` to the post-P9
  stable floor (96.2/<=1.9, tie slack) and now EVALUATES (was a static print; hr/cr
  were computed but never compared). Fixtures untouched — reclassifying correct
  declines as resolved would be the fake-coverage gaming Plan 014 exists to prevent.
- **2026-09-03 (4090 box, fourth unit — plan-number collision NOTE, discovered at the
  post-unit rebase):** `.plans/070` is DUAL-ALLOCATED between two sibling lanes —
  `070_kat062_t6a_counterfactual_surgery_gate.md` (COMMITTED `eae2e4e`, pushed —
  Status COMPLETE, Bench 066) vs `070_flashquant_sparse_dense_fusion_distill.md`
  (untracked WIP, Status IN FLIGHT, created 09-03). **Tie-breaker (Bench-697/848
  push-wins + baked-refs): kat062 KEEPS 070.** The flashquant plan MUST renumber →
  **071** at its next write time (read highwater 70, take 71, write back 71 — the
  current `.highwater`=70 is already the correct next-free state; do NOT take a
  number AT highwater, the 848 lesson) + grep-fix any live refs to "Plan 070"/"070_"
  inside the file. The number is not taken by another session here — this note is
  the flag; do not commit through the duplicate name. **UPDATE 2026-09-03 (verdict-filing
  session): `.plans/.highwater` now reads 72 — Plan 072 allocated to the MTPLX PR391
  batch (Plan 072 / Research 122, committed). 071 stays RESERVED for the flashquant
  renumber per the flag above; a renumber write-back must be max(current-highwater, 71)
  so the bump never regresses (the Bench-675 class).** **RESOLVED 2026-09-04 (B102
  takeover session): the flashquant plan renumbered to 071 and LANDED (Research 123
  / Plan 071, all tasks `[x]`); highwater stays 72; the duplicate-070 state is
closed.**
- **Owner-gated tails:** `.issues/044` (paging secrets `ALERT_WEBHOOK_URL` +
  `MONTHLY_BUDGET_USD`), `.issues/045` (T7 launchctl activation) — verified 08-31.
- **Queue items 6/7** (boundary-guard + substrate-first drift audit): last full run 08-29,
  quarterly — item 6 re-run OFF-CADENCE 2026-09-04 early (see the idle-cycle-4 entry: the
  19→16 owner move needed a fresh contract measurement + the SP5 phase was found unable to
  complete on a loaded box); item 7 stays at 08-29, due ~11-29.
- **2026-09-03 late (idle cycle 2, M3):** P18 verification sweep DONE (see the P18 row —
  all rust.yml full-guard on default branch; token missing on train/mmorpg/seal-remake,
  owner act); Issue 062 closed-negative + removed (`3e12635`, AGENTS.md record); plan 069
  landing-cleanup box verified done and closed; Protocol B sweep of **riir-dao** = ZERO
  residue (healer 0 proposable edits over 21 files / 118 spans; real `cargo clippy
  --workspace --all-targets` 0 warnings — the repo is 6 days old and written under its own
  `-D warnings` gate; the flashquant `.plans/070` dual-allocation self-resolved — file gone,
  highwater 72). Active siblings probed before the unit: riir-neuron-db UNCOMMITTED WIP
  (avoid), riir-chain last touched 21:52 same evening (defer its re-sweep to a quieter
  cycle — 3 weeks of post-campaign code incl. the closed 031/032 territory is still owed).
- **2026-09-03 night (idle cycle 3, M3 — four units, all pushed):**
  (1) **riir-train Issue 507 LANDED `e0716476`** — the repo's rust.yml compiled 15,147
  `#[test]` sites and executed none; new parallel `tests` job runs
  `scripts/test_gate.sh` (engine `--lib` default features, per-target FLOORS firing
  downward only — floor 1487 = the committed-HEAD measurement, strict single-result-line
  parse, `--test-threads=2` for the bench_transfer_overhead load-flake, `--canary`
  proven both directions: normal PASS 1498, canary FAIL via the floor branch). NOT
  covered, named: riir-train-gpu (1,669 GPU-bound), integration targets (277/487
  required-features), examples/benches. Guard stays compile-only by design. Issue file
  removed (noise-reduction); record in riir-train AGENTS.md + the workflow/script
  headers. NOTE: the tests job inherits the guard's SIBLING_REPOS_TOKEN loud-fail until
  the owner provisions it (P18).
  (2) **riir-deployer first-ever heal sweep `7a091db`** — 24 edits / 6 files (use_self
  x12, match_bool x5, + uninlined/vec-capacity/map_unwrap_or/single_match_else/
  shrink-to-fit); clippy 0 + 178/0 tests. (3) **riir-game-sdk heal sweep `fff904f`** —
  18 edits / 5 files; 3 targets inner-`#![cfg]`-gated → feature-aware validation (prod_e2e
  + game_e2e compile+run); sibling WIP arrived MID-UNIT (Cargo.lock + orchard demo dirty
  at commit time) → named-path staging held, theirs excluded. (4) **riir-chain re-sweep:
  the "still owed" note above is STALE — measured 0 proposable edits / 459 files** (a
  sibling discharged it in-window; dockerfile lines are the ledger-pinned declines).
  riir-auth scanned clean (0/17); riir-viewbridge PARKED (Proposal 001 D1/D3 — skip
  sweeps of parked repos, the CI gate is the rot check). Contended and skipped: katgpt-rs
  (717-719 agent), riir-ai (854-858 agent), mmorpg (092/094 agents), neuron-db (dirty),
  seal-remake (fps agent), 4090 (NoRA + Plan-371 + B102). kernel_opt mining stays gated
  behind sibling B102; B101 x86 re-gate armed on the 4090 window.
- **2026-09-04 early (idle cycle 4, M3 — boundary-contract unit + burner top-up):**
  (1) **Queue item 6 OFF-CADENCE re-run, and the gate could not previously complete on this
  box:** the contract script's SP5 phase walks 8 repos' manifests with
  `find -not -path '*/target/*'` — which FILTERS but still DESCENDS into every target tree
  (measured standalone: katgpt-rs alone 3m42s under build load, riir-train 43s) — two runs
  exceeded a 10-min timeout while healthy and read as hangs (the 862 collision note's
  line-621 death was a sibling session hitting this file mid-edit; crossfire documented both
  sides). Fixed in riir-ai: prune form + a **zero-scan hard guard** (a scan returning zero
  Cargo.toml for a Rust repo = hard error, never the vacuous green — the over-escaped
  predicate intermediate demonstrated that shape LIVE: 8 find syntax errors, zero rows, both
  SP5 gates ok). Canary: zero-manifest contract repo fires hard exit 2; full workspace run
  green in 35 s. **Edge count RE-MEASURED: 224 edges / 16 repos, contract clean exit 0** —
  the 862 issue's "currently unmeasured" record closed in its addendum. Landed riir-ai
  `3ae41bc83` via the detached-worktree push dance (main checkout carried sibling WIP + was
  diverged; patch-id dedup will collapse the local twin `18180d322` on the next rebase).
  (2) **riir-burner top-up `8c2ab9a`** — 2 semicolon_if_nothing_returned in pipeline.rs
  (missed by the 23:43 sweep; compile-gated, 2 Certified, clippy 0, 19 tests) — landed
  minutes before the owner's 19→16 move moved the repo to `obsolete/` (work preserved; the
  AGENTS.md lineage note now names it). (3) viewbridge re-dry-run post-`143aceb` = 0 edits
  (fully clean); riir-dapps confirmed already discharged by its finder (`d91881f` manual +
  `2b9cce9` post-fix re-heal). Contended/skipped: game-sdk (sibling WIP dirty), all other
  active-lane repos per the sibling-lanes note.
- **Batch 103 landed this cycle (2026-09-04):** see Last landed above; Plan 072 all tasks `[x]`;
  Plan 073 stays ARMED on the 4090 window (T0a probe blocked by the NoRA matrix until ~10:00-12:30).
- **2026-09-04 (this session, 4090 box — Plan 073 T0a EXECUTED, batch RESOLVED annotation-only):** the window opened (NoRA Issue-504 matrix ALL_DONE + closed; GPU compute-quiet — session-0 services + desktop GUI only per the warn-don't-block policy; the sibling had already yielded the lane). Pre-flight 3/3 → ran the staged `probe_873.exe` 3× (verdict-stable ×3) → **the H100 knee is ABSENT on Ada**: sparse grid saturates at K=2 (951.6 GB/s median, 94.4%) and stays flat through K=16 (−0.3%); saturating grid flat/noise (±8% within-cell, non-discriminating). Pooled medians + full record in Research 121 §2608.15764 RESULT; plan 073 all tasks closed; B92 Ada rider landed (corpus prose only, count 466 unchanged); T2 MOOT. Instrument + CSVs committed at `scripts/ldg_probe/` (the 848 lesson: compiled AND run before committing). One run-recipe lesson: git-bash needs `./probe_873.exe` (CWD is not in PATH); idle SM clocks park at ~270 MHz and boost to 2805 under the probe. Next on the 2608.15764 lane: nothing — the batch is closed ( reopen only on an H100-class part entering the fleet).
- **2026-09-04 ~04:30 (idle cycle 5, M3 — T0a harness PRE-AUTHORED, zero-contention unit):** the 4090
  was observed 0% util / 8.1 GB resident at 04:17 — the NoRA matrix is sequenced BEHIND the M3 509-T4
  run and had NOT started — but the window was NOT taken (the sibling's post-T4 pull/build/launch on
  that box is minutes-after-T4 and is itself probe-poisoning contention; the prior cycle already
  yielded the 504/509 lane). Instead the T0a authoring moved OUT of the window: `probe.cu` (nvcc sm_89
  standalone CUDA C; K∈{1,2,4,8,16} template arms; sparse 64×256 + saturating 512×256 grid regimes
  isolating depth from concurrency; fixed 4096 loads/thread; allocation-sweep; 3 interleaved reps; K
  independent 2-fadd consume chains so the 16B load cannot narrow) + run recipe + 3-condition pre-flight
  live at **`.raw/ldg_probe/`** (gitignored scratch; nvcc v13.3 confirmed on the box; design self-review
  caught the warmup nullptr + serial-consume-chain bugs BEFORE any run). Instrument commits only AFTER
  compile+run (the 848 lesson). Full details in `.plans/073` T0a. Everything else re-verified still gated
  at 04:14 (T4 running PID 48700; walk #7 not due; MCP quota; 066-T4 LANDED by idle cycle / P18 owner; mmorpg-092 siblings).
- **2026-09-04 ~04:50 (idle cycle 6, M3 — T0a harness COMPILE-VERIFIED on the box, zero-GPU-touch de-risk unit):** the window was still closed at cycle start (NO `t504_confirm/status.txt`; 4090 GPU 31% / 8.1 GB resident with `bonsai_lora_accuracy_parity_arm_c.exe` — the sibling 504 lane is LIVE on the box), so the compile gate moved out of the window too: scp → nvcc → GREEN, zero GPU state touched. **Three pre-window defects caught + fixed** (the de-risk paying for itself): (1) the warmup nullptr guard was INCOMPLETE (deref guarded, the cudaEventElapsedTime CALL not — InvalidValue → exit(1) during warmup, before any measurement); (2) CUDA 13 REMOVED `cudaDeviceProp::clockRate`/`memoryClockRate` (compile error on the box) — replaced with `cudaDeviceGetAttribute(cudaDevAttrClockRate/MemoryClockRate)`; (3) box recipe: non-interactive ssh PATH lacks cl.exe — prepend the MSVC 14.41 Hostx64/x64 bin. **ptxas: all 5 arms 40–44 regs, 0 spills — K=16 at 41 regs REFUTES the anticipated "spill at high K invalidates the arm" caveat; the depth axis is clean through K=16.** Compiled+verified `probe_873.exe` STAGED at `C:\Users\katop\ldg_probe_873\` — the window-open recipe is now: pre-flight → run → scp CSV back. `.raw/ldg_probe/README.md` + `.plans/073` T0a updated in-plan. Instrument still commits only AFTER compile+run (the 848 lesson). **Walk #7 correction: it ALREADY LANDED while this cycle ran (`4c9ba72`, sibling session) — strict window EMPTY, next announce Sun 09-06 20ET — so the inherited summary's "walk #7 due ~07:00 today" is superseded; no walk is pending until Sunday.** Everything else re-verified still gated (MCP quota; 066-T4/P18 owner; mmorpg-092 siblings).
- **Hygiene:** current after this entry. `.issues/` = 059/060/061 (owner/ops/legal-gated)
  + 065 (T3/T7 latent; T6 CLOSED on the 09-03 nightly PASS); **066 fully resolved + file
  REMOVED 09-04 (`1a1db877` — T4 had landed `4239dbdd`; durable record = the AGENTS.md
  paragraph beside Issue 050)**; plans tree 0 unchecked (069 closed 2026-09-03).
- **065 T6 verdict (2026-09-04 cycle):** the fix `319319a` passed the scheduled 18:23 UTC
  run end-to-end — regen OK 10.5 min, per-repo duration lines, the budget-exhaustion branch
  fired live (riir-clippy kept cached) with coverage=10 through the merge path, deploy
  verified, `=== done ===`. Checkbox flipped in `.issues/065`; first fresh dashboard
  artifact since 09-01 03:44. The T6 pre-run also surfaced 066 (docker corpus needle).
- **066 resolved same cycle:** T1+T2 (`c9863d6` — needle `    rule(` in both walkers +
  `corpus_stats_needle_tests` pinning walk==runtime for every compiled domain) and T3
  (`d731cac` — the four first-hand poison rows repaired via the one-shot dated upsert
  `examples/repair_066_corpus_store.rs`, git-verified values, provenance `repair-066`).
  Store = projection = mined now agree (27/31/31/31/31/31); the nightly regeneration diff
  that motivated `a7c473c`'s "do not commit" note is structurally gone. ~~Remaining: T4
  (log projection-vs-mined divergence instead of silently losing — owner call).~~
  **T4 LANDED same day** by the idle cycle (log-not-fail divergence lines + the mined-layer
  CWD anchor it exposed — `_count_in_rev` inherited the caller CWD and read zeros off-root;
  both subprocess calls now `cwd=REPO_ROOT`); the 3 residual drift lines root-caused benign
  (commit-time vs stamp-time definitional boundary), recorded in `.issues/066` T4.
  Corpus-count note: kernel_opt moved 453→459 in the sibling's Batch 102 (landed mid-cycle,
  `17f00e7`) — today's stamp carries it; no action needed here.
- **2026-09-04 ~07:10 (idle cycle 9, M3 — hygiene unit, zero-contention during the sibling
MLX 32K cell):** Issue 066 fully resolved + file REMOVED per noise-reduction (`1a1db877`;
T1–T4 were all already landed `c9863d6`/`d731cac`/`4239dbdd` — the closeout is the AGENTS.md
record + `git rm`). Plans 002/051 re-verified: 0 REAL unchecked boxes (the plan-002 grep
hits are prose; 051's last box was verified done 08-31). Everything else re-verified still
gated: T0a (NO `t504_confirm/` status file; 4090 busy 21% / 8.1 GB — the riir-chain verify
sibling holds the box), walk #8 → Sun 09-06, MCP pool → 09-15, Batch 104 = sibling session,
owner gates unchanged (P18, 059/060/061/065, katgpt-rs 718/719/721, riir-ai lanes).
Sweep surfaces all confirmed closed per the 09-03/09-04 entries (dao ZERO, deployer
`7a091db`, auth 0/17, chain 0/459, game-sdk `fff904f` — now sibling-dirty again, dapps
discharged, seal-remake zero-findings 09-04, viewbridge PARKED, mmorpg sibling lane).
Queues empty after this entry → idle report.
- **2026-09-05 (M3, idle cycle 11 — Batch 104 LANDED):** the 09-02-queued fa-vec league
    opponent-watch lead CONSUMED — see Last landed above (Research 126 / Plan 074, corpus
    464 → 466, K 5 → 7; `ecade10` + the plan-close `6f26898`). Everything else re-verified
    still gated: T0a (NO `t504_confirm/status.txt`; 4090 at 10% / 8.1 GB — near-quiet but the
    gate needs ALL_DONE, and the sibling 504-harvest lane is still running its analysis per the
    session context), walk #8 → Mon 09-07, MCP pool → 09-15, owner gates unchanged (P18,
    059/060/061 — 065 RESOLVED + removed by the sibling `0ba69cf` with T7 landed `1117f0a`).
    Upstream moved +3 mid-unit (the dispatcher mask-fuse rider `8bbd59e` + Plan-072 status
    `577b30d` + Issue-067 docs `0fb2a97`); rebased clean, counts re-verified post-rebase
    (the rider edits an existing rule's concept text — no corpus delta). Queues: the mining
    queue is again leadless for kernel_opt external sources (the oMLX pool closed at B94;
    ds4/PrismML/MTPLX/NAX/llama-fa-vec all mined; the arxiv walk is the remaining intake
    channel until 09-15) — next units are T0a when unblocked, walk #8 Monday, and the
    Protocol-B re-sweep when the dirty siblings settle.

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

One open class (below); the standing divergence (P4 residual) has no action planned.

- [ ] **2026-09-04 P10 NEW — LTO fold-elimination kills timing gates whose work
      has a dead result** (found by the katgpt-rs Issue 723 Class A2 fixes,
      commit `b4cef008`): rustc 1.98.1 + fat LTO eliminates inlined-callee
      work whose outer result is dead, EVEN THROUGH `black_box` INSIDE the
      callee — measured by a direct-call probe in one binary: used result
      16.6 µs vs `let _ = f()` ≈ 0. The rv gate measured 0 ns → NaN% for its
      entire release life. Fix patterns proven in-tree: (1) xorshift
      data-dependent work + caller-consumed sinks (`black_box` at the CONSUMER);
      (2) a bare CONSTANT work fn still folds through a sink (XOR of a
      constant 2M times = 0) — the sink must mix per-iteration data; (3)
      equal non-pruner work in both arms (one shared timed fn) so the ratio
      isolates the delta; (4) interleaved (baseline, candidate) chunks with a
      median-of-ratios for bar-adjacent gates — two sequential 2M arms read
      +5.2% and +21.7% thirty seconds apart, the paired median read −0.0%/
      +0.6% twice. Route: rust_perf measurement-honesty rule candidate
      (`dead-result-timing-loop-eliminated-under-lto` class) — the detector
      is the loud zero-baseline assert; the proposer would need CFG-level
      dataflow (suggest-only first). Evidence: katgpt-rs
      `tests/bench_gdsd_modelless.rs` header + `tests/rv_gated_routing.rs`
      sink docs.

- [x] **2026-09-05 P9 NEW — clippy 1.98 new-lint corpus candidates** (found by the
      1.98.1 toolchain re-gate sweep across the workspace; measured populations in
      brackets): `chunks_exact_to_as_chunks` [katgpt-rs 12 + 4 chunks_exact_mut,
      ndb 2 — `as_chunks::<N>().0`, needs const-generic N; MachineApplicable],
      `manual_checked_ops` [riir-dao 17, katgpt-rs 7 — if-guard + else-arm rewrite,
      MaybeIncorrect for clippy (the 17-site dao run applied 0 via `clippy --fix`),
      our proposer CAN see the else arm and emit `checked_div().unwrap_or(v)`,
      guarded against sample-accounting guards where the check gates more than the
      division — the riir-dao `sim/mod.rs:843/852` allow class], `collapsible_match`
      [katgpt-rs 6], `sort_by_key` [katgpt-rs 4], `byte_char_slices` [ndb 6,
      katgpt-rs 2 — `*b"FRZE"`, trivially fixable], `manual_slice_fill`
      [ndb 2], `explicit_counter_loop` [katgpt-rs 1], `manual_isolate_lowest_one`
      [1], `needless_pattern_in_..`-class [1], `redundant_reference_format_args`
      [1]. P0 candidates by population + bounded fix-space: `manual_checked_ops`
      + `byte_char_slices` + `manual_slice_fill`; `chunks_exact_to_as_chunks`
      needs an as-chunks MSRV check (std 1.88). Populations measured under load,
      re-count at batch time.
      **LANDED same day (Batch 91, M3): all four P0 candidates +
      `chunks_exact_to_as_chunks` as corpus rules + proposers — corpus 45 → 49
      (2 FIXABLE: `byte_char_slices`, `manual_slice_fill`; 2 VERIFY:
      `manual_checked_ops` [signed-divisor refusal gate; the MIN/-1 corner +
      custom-Div E0599 class rides the compile gate], `chunks_exact_to_as_chunks`
      [`&[u8]`→`&[u8; N]` body class + non-const Path size; MSRV floor std 1.88
      documented in the module doc — real clippy is MSRV-aware so the suggest
      path is inherently safe, the AST `--fix` path has no manifest access
      (fxhashmap precedent)]). Probe first (rustc 1.98.1 probe crate):
      `manual_checked_ops` fires ONLY on unsigned (i32/i64/isize silent), on
      `==`/zero-on-left/`!=`/`> 0` forms, NOT on `%`, span = the CONDITION;
      `manual_slice_fill` fires ONLY on the `&mut place` form (`iter_mut()`
      silent), span = the whole for-loop; all four lints are suggestion-only
      (no structured fix) — the proposers are the value. Gates: default lib
      1076/0, latent 1182/0/1i, clippy_verify 1090/0/6i, all-features 1883/0/16i,
      clippy `--all-targets` 0, retrieval floors HELD, suggest-retrievability
      PASS, score-bench #37 FLAT (96.2/1.9/100/100), real-clippy truthfulness
      4/4 (each corpus snippet fires its named lint). POST-MINING FINDING (same
      day, the loop working as designed): the `byte_char_slices` proposer
      shipped with a LATENT postfix-continuation bug -- `[b'-', b'.'].into_iter()`
      rewrote to `*b"- .".into_iter()`, which parses as `*(b"- .".into_iter())`
      (unary `*` binds looser than method-call `.`), so the emitted byte-string
      derefs the Iterator. Found live in the katgpt-rs consumer proof: the
      `--verify` compile gate AUTO-REVERTED both katgpt-pruners edits
      byte-exactly -- zero damage shipped, the safety net's first real catch on
      this class. Fix riir-clippy `012c68b`: when the next non-ws char after
      the rewrite is `.` or `[`, parenthesize the byte-string literal
      (`(*b"{[").iter()`); 2 new fixtures (method-receiver position, index
      position). Re-heal with the fixed binary landed the 2 sites (compiler.rs
      + dfa.rs, oracle 2/2 Certified) -- katgpt-rs `0ffe0e15`, 34 verified
      edits total across the four new classes. REMAINING P9 tail (not
      landed, populations small/judgement): `collapsible_match` [6],
      `sort_by_key` [4], `explicit_counter_loop` [1], `manual_isolate_lowest_one`
      [1], `needless_pattern_in_..`-class [1], `redundant_reference_format_args`
      [1] — re-file from the next fired histogram.

- [x] **2026-09-03 LIVE REPRO → CLOSED same night (items_after_statements try_hoist, fix
      `143aceb`):** a `--fix --write --verify` sweep of riir-viewbridge reproduced the
      column-0 defect on real code — `crates/riir-viewbridge-abi/tests/net_ffi_roundtrip.rs`
      `ws_burst_over_the_inbound_cap_drops_oldest_and_keeps_order`: the hoisted `const N`
      landed correctly BUT the displaced `let url = spawn_ws_relay();` lost its
      indentation (column 0). **Both standing gates passed it**: the compile gate is
      indentation-blind (column-0 `let` is valid Rust → verify kept the edit), and the
      fmt-clean-at-HEAD gate correctly skipped the file (pre-existing drift) — so the
      mangling SURVIVED TO DISK. Edit reverted by the sweeping session; surface yielded.
      The fixer landed `143aceb` (indent-aware splicing + all-or-nothing hoist +
      macro_rules-definition refusal, 4 regression fixtures) and rebuilt the binary the
      same hour; the re-heal produced a CLEAN diff (hoist at correct indent, displaced
      statement untouched), clippy 0 + 10/10 under `--features net` (the file-level
      `#![cfg(feature = "net")]` green-zero caught by count-assertion before trusting
      the default run), landed viewbridge `ea0337e`. **Standing criterion recorded:** the
      hoist preserves indentation BY CONSTRUCTION (not via rustfmt) — rustfmt-skip files
      are exactly where a malformed hoist ships.

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
- [-] **P18 (VERIFICATION SWEEP DONE 2026-09-03 — remainder is owner-only. Measured
      via `gh api` across all 13 gist-rs repos + katopz/katgpt-rs: (1) every repo's
      rust.yml runs the repo's own `scripts/ci_feature_guard.sh` (the full guard, not
      a subset) and EXISTS ON THE DEFAULT BRANCH (schedule/dispatch liveness, the
      `.issues/704` rot class) with schedule+dispatch+push triggers — including
      riir-train (rust.yml on `main`, its working branch) and riir-game-sdk (default
      flipped main→develop mid-day 2026-09-03 by the parallel CI session);
      (2) riir-unity has ZERO `.rs` files and no Cargo.toml — the boundary-only
      `no_rust_boundary.yml` is the correct shape, closed; (3) riir-dao's "no PAT
      needed" AGENTS.md claim VERIFIED — its only escaping dep is katgpt-core
      (public katopz/katgpt-rs, default GITHUB_TOKEN checkout), no riir-dapps path
      dep in the manifest yet; (4) `SIBLING_REPOS_TOKEN` configured on riir-chain /
      riir-dapps / riir-viewbridge / riir-auth / riir-clippy; **still MISSING on
      riir-train (Sun 04:17 cron), riir-mmorpg-examples (Mon 04:47), seal-remake
      (Wed 04:37)** — all three guards hard-fail LOUDLY at the first private
      checkout (the designed Issue-028 expiry-alarm behavior), but the owner should
      provision the secret before those first runs; provisioning is an owner act
      (secret value — not derivable, and minting one from a session's OAuth token
      is a security decision). riir-neuron-db (public-only dep) and riir-deployer
      (zero escaping deps by hard rule) legitimately need none. Original record —
      self-applied 2026-09-02, riir-clippy `2044722`; riir-deployer
      landed 2026-09-01, deployer `2b7d58d`; riir-ai landed 2026-09-02,
      riir-ai `4b085ebeb` — the workspace's WIDEST surface: 79 escaping deps
      across FIVE siblings pinned in its new scripts/standalone_dep_gate.sh
      (20 katgpt-* from public katgpt-rs; riir-neuron-db+neuron-db-sdk;
      riir-chain+riir-chaind; riir-train-engine; riir-dapps) + rust.yml with
      weekly Sat 03:58 UTC cron (default branch = develop, so the rot check
      audits it), Layer-4 Lean deliberately NOT duplicated (elan absent →
      loud skip; lean_proofs.yml strictly owns proofs); riir-train landed
      2026-09-02 same session, riir-train `66ffcd10` — 22 escaping deps
      across THREE siblings (12 katgpt-*; 9 riir-ai crates + the vendored
      cubecl-runtime; riir-neuron-db) + rust.yml with weekly Sun 04:17 UTC
      cron (default branch = main, this repo's working branch); its guard
      is COMPILE-LEVEL by design (five cargo check layers) so the ubuntu
      run is runner-friendly — the guard header's macOS exclusions
      (Issue 498) behave as on the Windows 4090 box on Linux; remaining:
      riir-train + riir-mmorpg-examples (ZERO workflows), riir-game-sdk
      (nightly+wasm32 only — verify test_gate.sh coverage), riir-unity
      (no_rust_boundary.yml only — no Rust, boundary-only shape may be
      correct), riir-dao + riir-chain + riir-dapps + riir-viewbridge (have
      rust.yml — verify it runs the FULL guard, per the 2026-09-02 re-scan:
      the original remaining-list predates several workflow landings); also
      verify the SIBLING_REPOS_TOKEN secret is configured per-repo (the
      hard-error makes an absent/expired one LOUD on the first scheduled
      run) — one
      per cycle) — full-gate
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

- [x] **P19 — needless_return on the cfg-switched dual-return fn (2026-09-04, live
      misfire → CONSUMED same day, `2dc31c0`):** a `--fix --write --verify` sweep of
      seal-remake fired needless_return on `fps_bench.rs::present_mode_from_env` —
      the parse-view last stmt was `#[cfg(target_arch = "wasm32")] return
      PresentMode::default();` and the rewrite (whose edit span COVERS the stmt's
      attributes) promoted the wasm32 arm to an UNGUARDED tail, dropping the attr:
      `unreachable_code` on native (the consumer's `-D warnings` clippy caught it;
      the healer's own `cargo check` verify passed — warnings don't fail check, the
      P-a screen class). Probe-first (clippy 1.93): real clippy is SILENT on the
      whole switch — it runs post-expansion, so its verdict is per-view, while the
      proposer's "last statement" is parse-view only and is NOT last on the arches
      whose cfg drops the stmt. Fix: refuse when the trailing return carries ANY
      attribute (cfg = view-dependent last; anything else = silent attr loss on the
      rewrite) — 3 fixtures incl. the positive control that a cfg'd NON-return stmt
      earlier in the body keeps firing (the guard is scoped to the target stmt, not
      cfg-anywhere). Lib 1038/0 (+3), score-bench #35 steady (96.2/1.9/100/100),
      binary rebuilt + live-smoked (1 edit proposed on the repro: the plain return
      only). Consumer-side disposition for the record: the heal's other 13 edits
      were sound; the misfired edit was reverted to the dual-#[cfg] form in the
      same session (seal-remake `ce5f7ee6`).

**Standing never-heal divergence table:** the cargo-heal skill owns it (comment-guarded
matches, array-literal defaults, named-arg renames, nested macro args, NaN-unsound
negated comparisons, cfg-attributed trailing returns).
