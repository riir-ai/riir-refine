# Research 245 — Batch 210: the 10-08 late-envelope repoll + the llama.cpp MoE expert GPU-cache write-path rule

**Status:** RECORD — converted same-session (Plan 210 / Batch 210, shikuwa/4090, riir-refine idle loop).

## The envelope re-poll (item 11, ~10 h after Batch 209's morning poll)

Pins read via the GitHub API commit lists, zero cargo, box quiet:

| repo | B209 pin | this poll | verdict |
|---|---|---|---|
| llama.cpp | `18b5f8b1` (server-parse, off-lane) | `50a6c5cf` (+2) | **1 lead** (`d6cf9acb` #29887) + 1 off-lane (`50a6c5cf` mtmd cohere2 vision #30062 — model plumbing, watch-only) |
| cubecl | `2f0d8e8df5` | `2f0d8e8df5` UNCHANGED | quiet (#1771 readme + #1772 workspace-deps chore + #1773 dry-run-untimed all previously adjudicated watch) |
| bevy | `5cd69c1357` | `5cd69c1357` UNCHANGED | quiet (#26046 test simplification, #25768 WESL 0.6 — dependency churn, no rule) |
| wgpu | `5c9660184d` | not re-polled (feed 404 at B209) | watch-only, carry |

## The lead — graded B−, converted as ONE kernel_opt D rule

**Source:** `ggml-org/llama.cpp` @ `d6cf9acb25e6c657d6c6a4645ab7b69fce119995` (#29887, MIT, merged 2026-10-07T18:07:45Z — ~1.5 h after the B209 poll's newest-seen `18b5f8b1`), "llama : add a GPU cache for MoE experts kept in host memory" (Aman Gupta, Assisted-by: Claude). 14 files +761/−18; new `src/llama-moe-cache.{cpp,h}` (550 lines).

**Provenance:** the full `.patch` fetched at the pin (`github.com/…/commit/d6cf9acb….patch`) — the patch-at-pin read is the verification source (equivalent evidence to the `.raw` clone diff convention); every quoted string in-rule verified verbatim against it. No ephemeral clone created; nothing to remove.

**The rule — `expert-cache-write-path-regime-gated` (D +1, corpus 776 → 777):**
1. **The write-path law** — an expert cache shared by narrow + wide regimes plans-and-evicts ONLY for batches whose working set fits; wider batches read-if-present and never touch the LRU (verbatim: "large batches use most experts of a layer, so they gain little from the cache and would evict the experts used in generation"; "large batches only read the cache, so the experts used in generation stay in it"). Scratch-isolation WITHOUT a second region.
2. **The scheduler-seam update hook** — the per-layer slot map is a HOST weight, so the scheduler's own split boundary (forced by the host-weight read) carries the plan callback (verbatim: "the slot map is a host weight, so the scheduler starts a new split here and copies it with the copy callback") — no new synchronization point.
3. Fit-gated plan refusal ("if (uniq.size() > (size_t) n_slots) { return false; }"), hit-before-evict ordering ("hits go to the tail first, so the head can be evicted below"), consecutive-run merge ("sorted misses usually get consecutive slots, so the uploads can be merged" + the `slots[e + n] == slots[e] + n` run detection), the MMQ overread slot ("one extra slot at the end, CUDA MMQ can read past the last expert"), the proportional budget split ("split the budget by the size of the experts, so each group caches the same fraction of its experts"), the forced-eviction NMSE pin ("the cache has only a few slots (4 for 288 KiB experts), so the experts are evicted and uploaded again").

**Dedup (both vocabularies, grep over the corpus at authoring):** the B94 family (jundot/omlx PR #3260) owns the victim POLICY (`decayed-frequency-eviction-recency-tiebreak-route-protection`), the one-shot REGION isolation (`scratch-bank-isolates-one-shot-operands-from-hot-cache`), the regime-START admission (`fill-gate-speculation-until-working-set-resident`); B26 `host-cache-immutable-device-weights` is the inverse direction (device weights cached on host). The WRITE-PATH regime gate + the seam hook are owned by nobody — all four distinct-from'd in-rule. `prefix-cache-entry-one-token-early` (B200) is a KV/prefix cache, different operand class.

**Grade rationale (B−, not B):** the device-bank + slot-map + LRU mechanism core is B94-family-shaped (omlx's pool.py resident expert cache); the NEW decision content is the regime-gated write path and the scheduler-seam hook — a genuine missing family member, but narrower than a standalone mechanism.

**NotBenchable** (no in-source perf numbers; the NMSE correctness test + teardown hit-rate logs are the evidence).

## Gates (all green, this session)

- batch210 gate FIRST SHOT (parse both sides via `KernelOptDomain::validate` + 13 needle markers + 4 distinct-froms)
- kernel_opt+latent_retrieval lib **3083/0** (3 ignored = the documented freeze-regen e2es; 1159.7 s — ONE full run)
- default lib **2473/0**
- clippy `-D` ×3 postures clean (1 doc_markdown mechanical in the new slice header healed — `MoE` backticks, the B192 class)
- retrieval **21/21 floors HELD** (216.6 s, no decay, no re-pin)
- freeze/score-bench NOT owed (zero clippy-corpus delta, P30)

**En-route pin catch (the pins doing their job):** the first category-array edit landed the rule in the **H (MoeDispatchBackward)** list instead of **D (Dispatch)** — the H block's tail (`dependency-wave-task-picker-one-wave-lookahead`) superficially resembled D's; `kernel_categories_counts_match_plan_003_table` red'd ("Dispatch has 177 rules, expected 178") and the move was made before anything landed. Count pins re-pinned ×7 sites (tests.rs total ×2 + batch116 + Dispatch 178 + sum/grouped, router.rs :472, mod.rs D header).

## Queue after

- nnethercote re-test on the October post or ~11-01 (unchanged)
- walk #14 opens Tue 10-13 (unchanged)
- the Research-239 follow-up pool + the llama.cpp watch set (incl. the new `50a6c5cf` mtmd plumbing watch) stay in intake
- Plan 204 T11 (M3-Max GOAT) rides the M3 idle loop
- owner-gated set unchanged (riir-ai 1036, SYSTEM orphans, Plan 199 rescope, C: disk 38G — dropped from 43.1G yesterday, the burn rate is ~5G/day)

NOTHING INVENTED — every quote read at the pin (the .patch at `d6cf9acb`).
