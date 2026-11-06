# 383 — metal-graph distill verdict (robbiemu/metal-graph)

**Status:** DISTILLED — pending owner decision

## Source

- **Repo:** https://github.com/robbiemu/metal-graph — "a reusable explicit graph execution API over Apple Metal"
- **Inspected:** 2026-11-06 (full clone at `.raw/metal-graph`, shallow, this session; removed at this landing)
- **Pin:** `0cd855b72d23c44aa0f7511aa6f01f694c00da39`
- **License:** Apache-2.0 (LICENSE read at the pin — CLEAN, unlike the recent NO-LICENSE TileQuant/TriAttention posture)
- **Language:** C11 + Objective-C++ backend (CMake); Python ctypes adapter, Swift module, Rust bindings reserved. The transcription law applies: source language is never a gate; fixtures are ORIGINAL Rust.
- **Shape:** `Graph -> GraphExec -> Launch` — the Metal analog of CUDA Graphs. Build a logical graph, instantiate once into a frozen executable plan (deep-copied nodes, retained buffers, compiled pipelines, planned workspace), patch compatible runtime params, launch repeatedly on a stream. Per-launch fresh `MTLCommandBuffer`s; the exec is the reusable asset.

## Inspection record

Files read at the pin: `README.md`, `AGENTS.md` (the repo's own engineering doctrine — unusually disciplined for an early repo), `docs/architecture.md`, `docs/open_questions.md` (325 lines — the resolved-decisions record), `docs/phases/phase-0/2/3` docs, `src/core/patch.c` (395, FULL), `src/metal/metal_backend.mm` (~70%: clone/clear paths, ICB plan build + eligibility, buffer create + `mgMetalBufferWrap`, `mgGraphInstantiate`, `mgGraphLaunch` full dispatch loop, `mgLaunchSynchronize`, launch destroy), `include/metal_graph/metal_graph_metal.h` (via the wrap call sites).

**Unread at verdict (re-read at mine time):** `src/core/graph.c` (1048 — construction/validation/topo order), `src/core/arena.c`, `src/core/internal.h`, `src/core/buffer.c`, `src/core/error.c`, `tests/*`, `docs/metal_graph_api_spec_v0.md`, `docs/phases/phase-1/6/8`, `artifacts/*` evidence reports, examples.

## Completeness signals

- Early-stage v0, self-described "early implementation", single author. Repo created very recently (2026 id range).
- BUT the mined surface is COMPLETE and tested: `tests/unit/graph_unit_test.c`, `tests/gpu/*` (real-Metal integration), conformance rules spelled in AGENTS.md §7, an evidence-suite lane (`artifacts/phase*` reports) required for semver-moving PRs.
- Grade at MODULE level: patch.c + the launch/eligibility/wrap surfaces of metal_backend.mm are landed, tested, and quote-stable. Phase 4 ICB is single-node-only BY DESIGN with the limitation disclosed in open_questions ("multi-dispatch execs fall back to direct encoding until the planner can prove that indirect command execution preserves direct-encoding semantics").

## GOAT disposition

Criterion 1 fails for a DOMAIN reason: the win metric (command-encode cost, GPU submit latency, launch overhead) lives in the Metal runtime the Pareto validator cannot host ⇒ **NotBenchable-by-domain annotations**, not demotion — the B223 (lithos-metal) / B319 (cubecl) precedent, both landed as decision-content entries. Criteria 2–4 pass for the validation-law candidates: exact semantics (bit-identical fixtures trivially — these are refuse/pass classifiers), magnitude story (B319's own measured 10→23 ms ICB degradation; the patch-OOB correctness class), deterministic. Grade on decision content × novelty × dedup × yield.

## Ranked candidates

| # | pattern | source | grade | notes |
|---|---|---|---|---|
| 1 | `replay-group-eligibility-excludes-mutable-surface` | `src/metal/metal_backend.mm:496-512` (`mg_icb_dispatch_eligible`) + `:514-602` (`mg_icb_plan_build`) | **B** | A node with ANY patchable field or scalar binding is INELIGIBLE for the pre-encoded replay group (`patch_flags != 0 \|\| scalar_count != 0` ⇒ `MG_ICB_FALLBACK_PATCHABLE_FIELD`); every buffer binding needs a declared resource with `byte_count > 0` and access != UNKNOWN (⇒ `MG_ICB_FALLBACK_UNKNOWN_RESOURCE_ACCESS`); multi-node groups fall back to direct encode until hazard-safe grouping is proven. Refuse-and-fallback, typed reasons. Kin: `cuda-graph-scalar-via-device-pointer` (gpu_01) = the REWORK half (make scalars capturable); THIS is the refuse-and-fallback half — they compose. `spec-regime-admission` (gpu_10) is the admission-control genus, different protocol (residency latch vs structural eligibility). `capture-region-dispatch-only` owns the captured-region SCOPE, not per-node admission. |
| 2 | `patch-validity-against-declared-resource-contract` | `src/core/patch.c:154-169` + `:74-80` + `:106-115`; binding-vs-contract separation in `docs/open_questions.md` §Dispatch Buffer Patch Range Compatibility | **B−** | The law: a binding answers "which buffer at this slot, what offset"; a resource contract answers "what range/access is this node allowed through that binding". A patchable binding MUST carry a declared nonzero resource range — "dispatch buffer patch requires a declared resource range" (patch.c:162); the replacement is proven against the CONTRACT (alignment + range vs the replacement buffer's length — "dispatch buffer patch range is incompatible" :168), never against the old binding's identity alone; patchable grid values carry declared ceilings ("dispatch grid exceeds declared maximum" :113); patchability is PER-ASPECT opt-in (buffer/range/value changes gated by separate flags — copy/fill patch splits `changes_buffer`/`changes_range`/`changes_value`; "field is not declared patchable" :78). Adjacent-distinct from `graph-exec-update-gated-by-captured-topology-signature` (gpu_48): THAT derives exec-sharing class + update legality FROM the capture; THIS validates ONE patch against the DECLARED contract. Also kin: `table-reached-buffers-declare-residency` (declaration half), `aliased-slot-allocate-replace-fallback-severs-binding` (slot-identity half). |
| 3 | `amortized-reuse-claim-break-even-artifact` | `docs/open_questions.md` §Benchmark Evidence Claim Boundaries + the code's diagnostics anchors (`mgGraphExecGetDiagnostics`, patch.c:377-394) | **B−** (route `rust_perf` measurement-honesty) | The evidence-suite law: every report separates `baseline_encode_and_launch_time / instantiate / patch / repeated_launch / per_launch_savings` and reports `break_even_launches = instantiate / per_launch_savings`; "If `per_launch_savings <= 0`, the report must say that no launch-count break-even was observed for that case" (loud-zero defence for an amortization claim); "A baseline must not secretly implement a private graph-equivalent runtime" (fair-baseline clause); claim boundaries — the suite "may support" orchestration claims and "may not support" faster-kernels claims. Kin: `speculation-scheduler-break-even-flip` (gpu_29) = the RUNTIME-CONTROLLER axis (break-even moves under a feature, flips engagement); THIS is the REPORTED-ARTIFACT axis (a measurement law for one-shot amortization claims). Also kin: `capture-break-count-not-perf-target` (gpu_08), `whole-round-in-situ-adoption-trial` / `cold-l3-evict-min-of-n` (bench-honesty family, rust_perf entries_10). |
| 4 | `zero-copy-interop-requires-documented-export-contract` | README §ABI Direction Phase 11 + `docs/phases/phase-11-mlx-zero-copy-completion.md` + `mgMetalBufferWrap` | **C+** (face-eligible; B− if landed standalone) | MLX zero-copy held in `unsupported_public_api` because "the public MLX Python surface does not currently document a stable Metal buffer, byte range, device identity, and lifetime export that can feed `mgMetalBufferWrap`"; the adapter reports a REJECTED zero-copy path while explicit `mode="copy"` remains, labeled as an adapter copy. The law: cross-runtime zero-copy requires a documented stable export contract (buffer identity, byte range, device identity, lifetime); absent ⇒ refuse the zero-copy claim loud, keep an explicitly-labeled copy path. Kin: `capacity-exceeded-silent-fallback-not-refusal` (sec) owns refusal-vs-silent-fallback at the capacity instance; `arc-refcount-across-raw-boundary` owns the custody two-regime. |

### Faces (count-neutral riders for a landing batch)

- `graph-fallback-coverage-ladder` (gpu_57) **+= typed-fallback-reason face** — the counted-fallbacks half is owned (B336 remedy); the face is the TYPED reason: a fallback reason ENUM surfaced in diagnostics (`MG_ICB_FALLBACK_{PATCHABLE_FIELD,UNKNOWN_RESOURCE_ACCESS,UNSUPPORTED,DISABLED,BACKEND_ERROR,INELIGIBLE_NODE}` + `icb_groups_planned/used/fallback` + `last_fallback_reason`), with the last-vs-per-group limitation disclosed (open_questions names per-group diagnostics as future work).
- `arc-refcount-across-raw-boundary` (sec) **+= wrap-custody-agreement face** — external-buffer wrap requires an explicit retention declaration (RETAIN_BUFFER or RETAIN_OWNER — "Metal buffer wrap requires retained buffer or owner lifetime", metal_backend.mm:778-783); owner-callback/flag agreement BOTH directions (callbacks without the flag refused :791-795, flag without complete callbacks refused :785-790); write-access⇒MUTABLE cross-field (:797-801); unknown flag bits refused (:772-777).

### Drops

- **mixed-backend MPSGraph conservative segmentation** (commit+wait+verify before encoding into a tensor island, fresh buffer after) — C; sync-at-boundary is ambient, and `mid-prefill-recurrent-state-capture-boundary-unsplit` owns the capture-boundary axis from the opposite side.
- **serialization-must-not-become-a-binary-cache** — C ambient portability rule.
- **exec-owns-frozen-copy** — ambient snapshot-at-instantiate.
- **host-callbacks-outside-locks** — KILL: positive instance of sec `ffi-callback-invoke-outside-lock` (B325).
- **retain-in-flight-until-completion / idempotent-synchronize completed-guard** — ambient ARC discipline + small.
- **per-launch-command-buffer-not-reused** ("A GraphExec is not a reusable MTLCommandBuffer") — the founding axiom of the graph family itself; owned collectively by the cuda-graph/ICB family. Cited, not claimed.
- **unchanged-ICB-replay-as-shipped** — NOT a rule candidate: a **LANE WARN**. metal-graph replays its ICB buffer unchanged every launch — the exact shape `metal-icb-replay-rewrite-private-copy` (B319) forbids (cubecl upstream #1993 measured 10→23 ms over 20 replays on Apple drivers). **CORRECTION (research pass 2026-11-06, issue-1181 verdict round 2): the remedy does NOT exist in OUR tree** — riir-infer's vendored cubecl 0.11.0-pre.2 has no ICB/capture path (`IndirectCommandBuffer`: zero hits across riir-infer vendor/ + src/ + crates/) and `cubecl-metal` is absent from its lockfile; the remedy lives only at upstream tip (`c1d0125`). Any future replay arm must BUILD it (riir-infer issue 1181's mandatory gate T2). The same correction supersedes the identical claim in the 2026-11-06 zed-m3-metalgraph queue row (snapshot rows are append-only; corrected here + in 1181). Our substrate's replay-adjacent surface is the COrpus rule + upstream only — cubecl-metal capture.rs is not in this workspace.

## Dedup evidence (one line per pattern, both vocabularies)

- Candidate 1: `icb|indirect.command|eligib` over `src/domains/kernel_opt/corpus/` → the ICB hits route to `gpu-resident-control-loop-static-icb-replay` (control-loop PLACEMENT), `metal-icb-replay-rewrite-private-copy` (replay DEGRADATION), `table-reached-buffers-declare-residency` (residency), entries_gpu_134's state-across-replays test fixture — zero owners of per-node STRUCTURAL eligibility with typed refusal. `graph-exec-update-gated-by-captured-topology-signature` read in full — sharing/update axis, not admission.
- Candidate 2: `patchable|patch_|"patch` over all corpora → zero law-bearing hits (only prose mentions); the binding/contract vocabulary (`binding index`, `resource range`) → zero owners. B48 + the slot/residency kin read and ruled adjacent-distinct.
- Candidate 3: `break.even|amortiz` → `speculation-scheduler-break-even-flip` (runtime controller, read in full) + prose-only amortization mentions (the tokenizers page-cache rule's anti-regime note); the break-even-as-REPORTED-ARTIFACT + fair-baseline law is unowned. `capture-break-count-not-perf-target` read in full — diagnostics-vs-target axis.
- Candidate 4: `zero.copy|zero_copy|interop` → zero owners in the corpora.
- Wrap custody: `arc-refcount-across-raw-boundary` read (rule list + module doc) — the face lands there, not a new rule.

## Lane-intel axis

- **Mapped lanes:** riir-ai issue 1165 — the GPU-resident greedy decode substrate LANDED in riir-infer (`resident_begin/resident_enqueue/resident_harvest`, 0.872× at the census posture; T1–T4 done, owner `RIIR_DECODE_RESIDENT` flip + the league bout REMAIN). The M3/M5 decode league rows are behind (0.888×/0.863×/0.842× at the 11-03 re-cell); the NAMED lever is ffn_gate_up (kernel-level) — graph-level orchestration is NOT named anywhere. The uzu decode lane (riir-infer 1178, bmo frame-census pending) is the active decode consumer lane.
- **What is NOT transferable:** metal-graph's unchanged-ICB replay path (see the LANE WARN drop) — B319's measured degradation makes it a known-negative on Apple drivers; a static-replay arm over our resident decode path must BUILD the per-replay private-copy remedy (upstream cubecl tip #1993 — NOT in our vendored tree, corrected above).
- **Research-side follow-up (same day): riir-infer issue 1181** — the fusion/watch-class issue (Metal decode command-replay watch lane, pre-registered gates T0–T4, demote bar 1.0 ms/token pre-registered against the amo M5-Ultra resident loop).
- **Watch-class offer:** IF the issue-1165 owner flip lands and post-flip host overhead prices into the decode cells, candidates 1–2 (eligibility + patch contracts) + candidate 3 (break-even evidence) are the design gates for any static-replay arm. No sibling issue filed — no open issue names graph-exec as its lever.

## Product-coverage axis

- The Metal graph-lifecycle family currently holds 2 own rules (`gpu-resident-control-loop-static-icb-replay` B223 from lithos-metal; `metal-icb-replay-rewrite-private-copy` B319 from cubecl) + riders (B307's Metal dispatch-threadgroup faces, entries_gpu_134's state-across-ICB-replays fixture). metal-graph is the family's FIRST dedicated graph-exec-lifecycle source (build → instantiate → patch → launch → diagnostics), growing the LIFECYCLE axis rather than the replay-kernel axis.
- Ecosystem reach: Metal-only platform; the Rust-side consumers are cubecl-metal (our substrate) and wgpu-hosted Metal apps. The stronger product case: candidates 1–2 transcribe directly into the CUDA-graph vocabulary (patch eligibility/update-gating over `cudaGraphExecUpdate`), where the corpus family is large and actively fed by league opponents.
- Family continuation: YES — a batch from this source would be the family's third source and its lifecycle seed.

## Yield estimate

3 own rules + 2 faces candidates → after ~50% coordinator falsification: **2-3 rules + 1-2 faces** land. Nominal Batch 341 (plan 350), staged UNCLAIMED behind Batch 339 (TriAttention, in flight) and Batch 340 (SPILLWAY, staged).

## Caveats

- Early-stage v0, single author. Mine completed modules only (patch.c + launch/eligibility/wrap); the phase-4 ICB surface is intentionally narrow (single-node) — quote the limitation, don't generalize.
- C source: corpus fixtures must be ORIGINAL minimal Rust reproductions (license hygiene + binary-only distribution rule).
- Candidate 3's primary source is a DOCS section (open_questions.md) — the entry must anchor the code half too (the diagnostics struct the law feeds).
- The repo carries its own AGENTS.md with unusually strong engineering doctrine — cite it as the design record for provenance, but fixtures and quotes come from the implementation files read at the pin.
- Unread files listed above — re-read at mine time before quoting anything from graph.c/arena.c.

## Queue slot

`.distill/001_mining_queue_snapshot.md` VERDICT row 2026-11-06; plan `.plans/350_batch341_metal_graph_kernel_opt_distill.md` STAGED UNCLAIMED. Plan 350 was explicitly left FREE by the FastAFD record (research 382) — their framing allocates at their own landing per the LoopDuet precedent; this allocation is disclosed in the plan header and the queue row.
