# 324 — Batch 286: the burn/cubek remainder (candidates H/I/J)

**Status:** RECORD — the Batch-286 landing note (Plan 291; the conversion of the Research-321 verdict's three unconverted candidates; the claim `c406d6c0e` discharged by this landing).

## What landed

Corpus 974 → 977 (kernel_opt; D 232→233, K 25→26, L 29→30; `entries_gpu_130`). THREE own rules, all `NotBenchable` (the domain-reason class — the win metric lives in GPU autotune rounds / launch vectorization the Pareto validator cannot host; the Batch-136 precedent):

| # | rule_id | grade | category | source |
|---|---|---|---|---|
| H | `elementwise-operands-presented-in-memory-order` | B− | L (layout) | burn @ 907ab111 `kernel/memory_order.rs` |
| I | `autotune-candidate-priority-orders-try-first` | B− | K (tune) | cubek @ 08659297 `autotune/{base,priority}.rs` + `kernels/tune/candidate.rs` + burn `attention/tune.rs` |
| J | `blueprint-minimal-compile-specialization-key` | C+ | D (dispatch) | cubek @ 08659297 `GUIDE.md` (Blueprint section) |

- **H** — the per-op presentation-permute: elementwise operands handed to the kernel in the order the bulk of them occupy memory in; the result permuted back to LOGICAL dims over the presented-order strides. "Permutation changes metadata, not data" — stride re-indexing over the same bytes, never a copy. The exact-shape vote ("Only operands of exactly that shape vote on the order"). THE CONDITIONAL LANDING (Research 321 §9's own requirement — the source text half-refutes the unconditional form): "This is a locality heuristic, not a guarantee…"; a fresh result keeps the presented physical layout, a later stride-inexpressible reshape must materialize ("The extra copy can outweigh the elementwise speedup"), and "Choosing the order here cannot account for consumers that have not run yet." The caveats ARE the decision content; the in-rule honesty clauses carry them.
- **I** — the priority-semantics law, the complementary half of the same base.rs doc as Batch 284's bounded-round rule A: group priority is a **cutoff** ("cubecl measures the highest-priority group whose candidates launch and stops there"), candidate priority is a **try-first order** ("a claim about which candidate to *try first*, never about which can win: a wrong priority costs a compile"). The PROOF ARM removes candidates: "a setting that compiles a launch an earlier candidate already compiles *is* that candidate"; ties resolve EARLIER ("the candidate a tie resolves to is the one the list leads with — the canonical setting, which a pin runs"). The matmul coverage formula ("the output block one cube covers against the key's extents"). The fallback law ("First entry should always work, since it is considered the fallback"). The speed factor (far-behind sampling stops after the second run).
- **J** — the blueprint minimality include/exclude law: "To prevent kernel explosion… the blueprint must be kept minimal"; the ONE admission test ("fundamentally alters the control flow or the specific instructions"); INCLUDE algorithm variants / per-algorithm comptime settings / safety logic (masks vs branching); EXCLUDE with the home named for each: vectorization → the JIT key via input types, cube dims → the compilation key, hardware → read in-kernel, problem sizes → runtime args. DOCS-BORNE (no enforcement site at the pins) — disclosed in-rule per the plan's convert-only-if-above-the-thinness bar; the four reasoned exclusions cleared it.

## Provenance

- Pins re-verified at authoring (plan 291 T0, 2026-10-26 ~00:5x +07): burn `907ab111a02c2dea47bc41633754f095c6b2220e` (v0.22.1), cubek `08659297eec3414311df573aada3f22fb8142054` (tip), burn-pin cubek rev `51c43a1af7eb07a904b8a13568e953d377d20bb4` (I's fallback quote — the Research-321 path CORRECTION confirmed: the quote lives in burn's `attention/tune.rs`, not "tune/base.rs"). Ephemeral blob-filtered clones `.raw/b286-{burn,cubek}`, dump-per-file reads (the plan-288 T0 lesson: tree-wide grep hangs on blob-filtered clones); removed at close.
- T1 dedup re-verified per candidate at 3+ vocabulary variants: H/I/J zero owners (corpus moved +4 via B284 since the census — A-F do not own these faces; `multi-priority-tune-groups` owns shape-keyed group MEMBERSHIP/pruning, I's semantics faces distinct, kinship named in-rule).
- Category routing: H → L (the memory-layout family beside `decode-layout-weight-permute-copy-registry`), I → K (beside the bounded round), J → D (beside `hardcoded-fast-path-jit-specialization`, its body-specialization kin).

## Gates

| gate | verdict |
|---|---|
| batch286 needle + parse | 2/2 FIRST-RUN (all 27 needles + 6 fixtures parse) |
| kernel_opt lib (`--features kernel_opt`) | 3238/0 |
| default lib | 2733/0 |
| clippy −D ×3 postures (--all-targets) | clean |
| retrieval floors (kernel+latent posture) | 22/22 HELD, no re-pin (~491 s) |
| freeze gen2/gen3 | NOT owed (P30 — zero clippy-corpus delta) |
| score-bench | NOT owed (kernel_opt-only delta, the B284 precedent) |
| release-set binary | NOT owed (kernel_opt outside the release set) |

Count pins: 11 single-line lineage asserts + 22 bare value sites + the two sum/bucketed asserts + the Plan-003 expected table (D 233, K 26, L 30) + the router total (974→977) — value flips by script, lineage prepends at the uniform `batch 284 re-pin` openers, the VALUE-tracks-live / MESSAGE-records-history convention. The stale `Sum must equal 969` comment (drifted through B284's 974) repaired to 977.

## En-route findings

1. **Fixture escape defect** (caught by inspection pre-gate): the J opt_pattern string carried a backslash-followed-by-spaces where the `\n` escape was intended — an invalid Rust string escape that would have failed compile. Repaired to the `\n` escape before the gates; one line, verified by od byte-dump.
2. **Pre-existing gate-blocker healed en-route** (the B254/B281 precedent): `batch284_rules_and_rider_landed` fails `too_many_lines` (109/100) at the `--all-features` posture — 122 lines at HEAD with no allow attribute, i.e. the Batch-284 landing's own clippy claim did not cover this posture for this fn. Allow-annotated in this batch with a comment naming the provenance; not this batch's defect.
3. **Ledger gap repaired**: `.docs/11_domains/kernel_opt.md` had NO Batch-284 row (the M3 lane's landing recorded gates in the snapshot only) — a one-line B284 row added in this landing alongside the B286 row, attributed.
4. The count +1 drift on the lib totals (kernel_opt 3235→3238 base, default 2732→2733): B284's recorded floors read 3235/2732; the current run reads 3238/2733 pre-my-tests — one test landed between the B284 record and this session (the M3's plan-289/290-era commits or a rider test). The floors are DOWNWARD-only; recorded, no action.

## Queue state after

Walk #17 (claimed, expiry 2026-10-27 ~04:3x +07) grades at/after 07:00 +07 today — note slot re-derived to **325** at ITS landing (this note took 324, exactly as the claim row anticipated). B285 bbtree = the M3's (rust_perf corpus — disjoint). B279 riders = the M3's. nnethercote ~11-01. The b10822 bout behind the sibling cell C. NOTHING INVENTED — every verdict a run or a pinned read this session.
