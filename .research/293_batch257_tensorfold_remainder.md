# Research 293 — Batch 257: the TensorFold staged remainder conversion

**Status:** RECORD — converted as Batch 257 (Plan 261), 2026-10-20, shikuwa/4090 idle loop.

## Source

`ashhart/tensorfold` @ `d1018b318fb51981c4a18f12e40269a4d3ee9713` — RE-VERIFIED at claim: still origin/main tip, ZERO drift past the B256 staging pin. The pool is Research 291 §"STAGED-NOT-CONVERTED" (the B256 read agents' B+ remainder, ~13 candidates) + the `a739444c` watch lead (Research 291: "read at B257 staging if the pool reopens" — it did). Apache-2.0 throughout (Zig-era serving engine).

## Method

Claim-first (`7add28660`, plan 261 opened same commit). 2 parallel authoring subagents over disjoint write sets (kernel slice → `entries_gpu_111.rs`; rust_perf slice → `entries_37.rs`) + coordinator riders (RD1/RD2 on `entries_gpu_102.rs` hosts) + wiring/pins/gates. Every quote re-verified at the pin via `git show` in the clone `.raw/tensorfold-b257` (rev-parse EXACT). Dedup greps at 3+ vocabulary variants per candidate over BOTH corpora before authoring; dedup re-adjudicated by the agents with named distinct-from sets per rule.

## Converted (13 rules + 2 count-neutral riders, ZERO drops)

kernel_opt 910→917 (`entries_gpu_111.rs`, 7 rules: F+1, D+5, A+1):
1. `cluster-dsmem-shared-head-one-row-decode-attention` (F, `01b7f3bb`) — one decode head spread across a thread-block cluster; scores gathered over dsmem; one-CTA-order replay.
2. `pdl-admission-by-image-set-marker` (D, `bef432bb`) — adjudicated OWN RULE, not a rider: three named surfaces (the `drama_pdl` marker-emission contract, the both-images fail-closed probe "any driver failure answers false, never an error", the embedded-vs-directory posture). The THIRD PDL axis beside gpu_23 (in-kernel wait scoping) + gpu_18's rider (device capability) + gpu_57 (measured negative).
3. `copy-source-quarantine-unproven-free-pinned` (D, `2ba81a06`) — freeing async GPU memory requires WAIT PROOF; unprovable drops enter a charged quarantine; the free is RE-ARMED, not abandoned. Completes the custody family beside entries_36's pinned-entry-tombstone (host-cache sibling).
4. `prefill-batch-atomic-fallback-individual` (D, `c4995966`) — `prefill_many` batch with the atomic fall-back to individual launches; "each goes alone" refusal per job.
5. `teardown-takes-list-whole` (D, `30f05c6d`) — a failed round/cancel/close ends every held job, copying nothing, opening no stream. Name follows entries_36's forward reference (the staged `-not-copying-dupe` suffix dropped for the cross-ref to resolve).
6. `drain-device-work-before-free` (D, `98571a84`) — learning drains its device work before it frees; a failed pocket read is an ERROR, not an empty pocket. Distinct from gpu_04's stream-drain (memory-shape vs completion axis).
7. `verify-row-tiles-share-loaded-weight-panel` (A, `4d1a3f15`) — 32 GB10 verify rows share one loaded affine-4 weight panel; the row-doubled tile (5 pipeline stages); upstream status honesty carried ("pending compiler and device qualification" — the source's own gates are the promotion bar).

rust_perf 235→241 (`entries_37.rs`, 6 rules: dispatch+2, io+1, allocation+2, caching+1; ALL ANNOTATED NotBenchable):
8. `first-message-sized-to-smallest-job` (dispatch, `10ff8779`) — a decode step sends one message, not 16 rows of padding; 165,440 → 10,560 B/step; "Arithmetic is unchanged: drafted, serial and concurrent replies keep their token hashes".
9. `draft-cost-timed-at-served-widths-not-extrapolated` (dispatch, `abb860c5`) — "the rule's extension of the window table priced them at twice the truth": measure the served widths, interpolate, never extrapolate; widths {32,48,64,96,128}; 9 ms vs 27 at 8 streams.
10. `checkpoint-reads-skip-page-cache-with-refusal-fallback` (io, `221c2b5d`+`6bb9daac`) — O_DIRECT/`FILE_FLAG_NO_BUFFERING` with the EINVAL flag-clear fallback; span-alignment vectors span(0,1)=4096 / span(4095,2)=8192.
11. `reserve-stream-caches-at-admission-warm-request-paths-at-load` (allocation, `84c58d3f` sibling) — the server knows its stream count at admission; reserve allocates + WARMS each stream's caches at load (the 20 s first-request launch stall); loud refusal "the native CUDA engine cannot reserve {d} streams".
12. `cache-budget-first-sessions-take-remainder-never-reshrink` (allocation, `406b2090`) — shared sessions reserve the prompt cache before they size themselves; `const room = cap -| ready -| policy.margin`; `error.CacheOverCap`; never reshrink.
13. `kept-prompt-states-pinned-host-cache-budget` (caching, `a739444c`) — the a739444c watch lead, adjudicated OWN RULE: pinned-HOST-MEMORY placement + the budget axis (8 GiB unasked, ≤ half MemAvailable, 0 on unreadable /proc, coldCut). Distinct from entries_36's pinned-entry-tombstone (pin-lifetime face).

Count-neutral riders (gpu_102 hosts, gpu_concept-side):
- RD1 on `chain-rows-attend-own-keys-in-prefix-per-row-key-bound` — the row-keyed CHUNK-WIDTH face (`79bdb93d`+`02d3e2ed`): each row's chunk width from its own keys/position (`chunkFor(position)`, `chunk_keys(keys)`), never the forward's widest stream — the fp32-sums determinism guard ("either changed the fp32 sums"); heterogeneous widths coexist via a pass-per-width + mask; "Rounds that do not cross 8192 keep their bits and dispatches".
- RD2 on `fill-foreground-then-fewest-remaining-tokens` — the starvation aging bound (`48277078`): `preference_rounds = 8`; aged-wins DOMINATES the class split; aged ties → largest wait; saturating increment; bound = 8 + admitted lanes; winner reset re-arms the escape.

## Drift disclosures (staged brief vs source)

1. R5's staged name `teardown-takes-list-whole-not-copying-dupe` → landed `teardown-takes-list-whole` (entries_36's forward reference wins; the cross-ref resolves).
2. R1/R2 files restructured into the drama3 family after their commits — quotes verified at COMMIT BLOBS with in-entry DRIFT notes.
3. R4's gate doc evolved at the pin ("prompts batch (beside chunked fills up to many_rows)") — landing-commit posture quoted as LANDED with disclosure.
4. "env-fingerprint cost-cache persistence" (Research 291's R9 framing) NOT in `abb860c5` — the persistence is pin-state infra keyed by the EXECUTABLE's bytes (lanes/cost_cache.zig), CUDA-side; disclosed in-rule.
5. The O_DIRECT EINVAL fallback is Linux-only at the pin (`4e34b0e6` merged); Windows covered at open-time only.
6. The drafter-side pool (`Draft.reserve`) landed later (c3861d33) — at 84c58d3f the pool is the target engine's caches + warmCache (the generalization arc disclosed).
7. cuda_reuse.zig evolved past the landing at the pin (bf16/fp8 format checks, image-Seen gating, reading-flag) — disclosed.

## Gates (all green)

- batch257 gates: kernel needles 7 rules + 2 riders (one needle re-keyed — "8 + the admitted lane count" → the rule's own "bound = preference_rounds + admitted lanes (8 + n)" phrasing); rust_perf parse 6/6 + needles.
- default lib **2702/0** · kernel_opt lib **3174/0** (1 ignored = the documented freeze-regen e2e) · rust_perf_bench lib **2733/0** (6 ignored).
- clippy −D ×4 postures clean + the all-targets combo (2 doc_markdown mechanicals in my mod.rs header healed manually; 1 too_many_lines allow on the kernel gate).
- retrieval floors HELD no re-pin (21/21 tests, 285 s, blended/kernel suites green).
- score-bench **#192** appended (heal 0.9636 FLAT, parse_safety 1.0, gen-1 saturated; frontier snapshot gen 7).
- freeze NOT owed (zero clippy-corpus delta — the M4/M5 generations key on the clippy seed corpus).
- En-route fixes: fixture E0308 (single-element needle arrays in a homogeneous for-loop — widened to 2 needles); the RD2 mis-placement (both riders initially landed in the chain-rows rule's gpu_concept — the anchor string appeared twice inside it; rider 2 moved to the fill rule); kernel count pins 910→917 at 39 sites (17 message prefixes + 11 bare + 6 inline + 2 totals + 3 comment forms); the cross-domain batch249_slice3 rust_perf total pin (235→241) + the serde annotated pin (182→188) surfaced by the full-lib run — the pin-hiding class repeating.
- Research-number collision: 292 was allocated mid-flight by the sibling's cloudflare/security-audit-skill distill verdict (`449432e4d`, which explicitly yielded 257/261 to this claim) — this note is 293; the agents' "Research 292" provenance strings in the corpus files were rewritten to 293 pre-commit (15 sites).

## Provenance

- Clone: `.raw/tensorfold-b257`, detached at pin `d1018b31`, REMOVED at close (the ephemeral-clone law).
- Box state: Windows 11 / i7-13700K / RTX 4090, CPU-only batch (StartLux stack resident+idle to 2026-10-20T13:30Z), CARGO_TARGET_DIR=/tmp/b257-target (removed at close).
- CRLF hazard: the edit-lane class instances #10 (the snapshot, normalized) + #11 (the plan file, normalized) caught pre-commit this time; the write_file path CRLF's on this box — normalize after every write.
