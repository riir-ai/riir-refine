# Research 110 — Distill verdict: oMLX PR #3260 "SSD Expert Streaming and Soft-REAP for MoE models" → kernel_opt (host orchestration) + rust_perf (io/caching)

**Status:** DISTILLED — Batch 94 queued (verdict-only landing 2026-08-31; yield
estimate ≈ 8 new rules + 2 riders + rich bug yield; mine-time source re-read
REQUIRED before any corpus entry per Protocol A — the clone at `/tmp/omlx3260`
is disposable and the head is force-push-prone, so re-clone `refs/pull/3260/head`
at mine time and re-verify every anchor)

- **Source:** [jundot/omlx PR #3260](https://github.com/jundot/omlx/pull/3260) —
  "feat(moe): SSD Expert Streaming and Soft-REAP for MoE models" by onthehub97,
  opened 2026-08-28, force-pushed to `3343e44` 2026-08-29, 8 commits, ready for
  review, NOT merged at research time. ~3,300 LOC new module
  `omlx/expert_streaming/` (10 files) + `docs/soft_reap.md` +
  `benchmarks/soft_reap_streaming.py` + synthetic microbench + 372-test suite.
  Cloned full head to `/tmp/omlx3260` (Apache-2.0, same repo as Research
  082/083 Batch 71 and Research 099 Batch 84 — third oMLX distill).
- **Inspection date:** 2026-08-31. `docs/soft_reap.md` + all 10 module files read
  in full via 4 parallel subagent extractions (pool/residency/routing,
  execution/integrate/manifest, safetensors/fast_resource, web prior-art).
  Six load-bearing anchors re-verified byte-accurate by the coordinator
  (fingerprint, fill-latch, decay, guard asymmetry, snapshot leaves, lazy-promotion
  comment). PR comment thread read (pedroraft's measured review is a finding
  source of its own — §7).
- **Verdict:** **YES, grade A-** — one batch (Batch 94), lead with the
  serving-orchestration cluster. The *class* (SSD expert offloading) is heavily
  published; the value is a cluster of host-orchestration mechanisms with
  measured numbers, a bug yield of ~24 inventoried hazards (4 corpus-grade
  defense rules), and direct league relevance on the M3 lane (§8).
- **Domain routing:** `kernel_opt` D (host orchestration) × 6 new + 2 riders;
  `rust_perf` `io` + `caching` × 2 new. NOT clippy_lints (no parse-only lint
  shape — Python serving code), NOT sec/docker.

## 1. Provenance lineage (matters for both novelty and league)

The PR's cache foundation is explicitly attributed to **stefandsl/DwarfStar**
(verified: a public fork of **antirez/ds4**, the self-contained C inference
engine for DeepSeek V4 Flash/PRO). ds4's README states the thesis in one line:
*"The KV cache is actually a first-class disk citizen... SSD streaming allows to
turn the available amount of RAM from a hard cutoff (can I run this model or
not?) to continuous spectrum of speed levels."* — `ds4_ssd.c` +
`ds4_streaming_hotlist.inc` + `--ssd-streaming-preload-experts` are the ancestral
machinery the PR ports to Python/MLX and extends (cold execution bank, bounded
group overlap, 64 KiB coalescing, gate/up bank fusion, physical-slot sorting are
the PR's own additions per `docs/soft_reap.md` §"DwarfStar-derived").

Same-universe tie-ins already in our records: our Issue 746 phantom-token KV
audit analyzed THIS repo's `mlx_lm_mtp/cache_rollback.py` (DeepSeek-V4-Flash,
sliding_window=128) — the rollback substrate this PR's speculative executor
reuses ("restores the pre-pass KV/SSM cache state", `execution.py:283-306`).

## 2. What the PR does (measured facts from the source doc)

- Routed MoE experts live in **fixed, preallocated per-layer MLX banks**
  (manifest-pinned REAP experts + evictable hot slots + optional separate cold
  execution bank); non-resident experts are read from the original safetensor
  shards through a **row-addressable index** (weights/scales/quant-biases per
  expert row, no payload mmap).
- **Soft-REAP pinning** (Cerebras REAP manifest, arXiv 2510.13999) or
  **analytical cache-only**; bit-exact at 0% substitution threshold.
- **Qwen3.8-Flash-Next oQ4e MTP (98.995 GiB baseline):** 10 hot/layer + 288-expert
  REAP manifest → 72.565 GiB resident, load 19.8–20.3 s; cache-only 32/layer →
  **39.684 GiB measured, load 6.794 s**. On an 18 GB MacBook (pedroraft's
  review), Qwen3.6-35B-A3B-4bit streams at **15.0–17.4 t/s decode**.
- **Honest negative recorded by the author:** in an isolated all-hot replay,
  speculative output was bit-identical (`max_abs=0`) at **0.579 s vs 0.556 s
  checked — no throughput benefit even at 100% resident hit rate**. The
  speculative graph exists to avoid the per-layer router readback (the
  pre-graph-native path measured **127.9–132.8 s for one cold token**).
- Synthetic gates: cold bank cut hot-cache evictions **42 → 0** and sped the
  miss-heavy prefill→decode fixture **2.12–2.19×**; physical-slot sorting
  improved 256-token prefill **26.6%** (bit-identical); gate/up bank fusion cut
  decode QMM launches **3 → 2** and improved the isolated single-token fixture
  **8.6%**.

## 3. Mechanism inventory (anchors verified)

| # | Mechanism | Anchor (head `3343e44`) | Note |
|---|---|---|---|
| M1 | Decay route-hotness: `_route_hotness >>= 1` every 16 routed tokens | `pool.py:194,431-434` | Hits AND misses both score (`pool.py:571→435-441`) |
| M2 | Evict min-hotness, recency tie-break; current-route experts protected from eviction | `pool.py:516-520,528-534` | Fail-loud if no candidate |
| M3 | Learned per-layer hotlist: lifetime selection counts, saved on close, `mkstemp`+`fsync`+`os.replace` atomic | `integrate.py:155-163,166-175,184-185` | Ranks by UNdecayed counts (`pool.py:444,451-457`) — see H4 |
| M4 | Warm start: learned experts first, then optimistic zero-hotness fill of every remaining slot | `pool.py:477-488,498-501` | "Eager RAM saturation" — more RAM = more route coverage, no implicit pins |
| M5 | Profile fingerprint = SHA-256 of `path\0size\0mtime_ns` — **stat-based, not content** | `integrate.py:196-200` | Bug yield — §6 R1 |
| M6 | Fill gate: speculation refused until every layer's evictable cache is at capacity; readiness LATCHED; `fill_gated_passes` counter | `execution.py:240-244,235-237` | Prefill always checked; single-token only |
| M7 | Speculative executor: resident-hit mapping inside the MLX graph, ONE post-pass check, KV/SSM `CacheSnapshot` restore, promote first missing layer, retry ×1, checked fallback, rejected passes never returned | `execution.py:186,224-244,261-280,283-308` | First-miss-only trust: "Downstream routing was computed from placeholder expert output and is not a valid analytical cache signal" (`execution.py:277-278`) |
| M8 | Lazy promotion: "Dynamic updates deliberately remain lazy. The layer's QMM consumes the updated bank immediately, and the next layer's router evaluation provides the required synchronization" | `pool.py:411-413` | Binding (`bank_bind_seconds`) vs materialization (`bank_materialize_seconds`) split in stats (`integrate.py:87-88`) |
| M9 | Fixed banks, in-place slot reuse (free list → evict-and-reuse same slot), eviction overwrites content in place | `pool.py:243-246,542-551,399-401` | Shape/address stability = the captured-graph contract (B58 class) |
| M10 | Cold execution bank: one-shot prefill experts execute in bounded groups in a SEPARATE bank — never evict hot-cache rows; one group in flight (request next while current runs) | `pool.py:852-902`, `integrate.py:111-114` | H7: the no-scratch path can thrash |
| M11 | Prefetch workers do host-only `pread` + NumPy decode; MLX array construction stays on the caller's inference thread ("thread-local Metal streams are never transferred between threads") | `safetensors.py:688,454-459,627-634` | 9-thread I/O pool + 1-thread request pool, ordered `map` join |
| M12 | 64 KiB bounded-gap read coalescing, **coupled to cacheability**: direct/F_NOCACHE mode coalesces with gap 0 | `safetensors.py:448,560-562,597-598` | Merged rows re-selected from payload by index |
| M13 | Darwin hints: `fcntl(F_RDAHEAD)` on cacheable fds, `fcntl(F_NOCACHE)` on direct fds, per persistent descriptor, advisory-swallowed | `safetensors.py:536-549,526-531` | NOT posix_fadvise; fds cached per `(path, cache-mode)` |
| M14 | Row-addressable safetensor index: header-only offset math, stacked (even-division row stride) + fragmented per-expert layouts, w1/w2/w3 aliases, fused gate/up halves | `safetensors.py:66,91-95,403-411,626,116-118,242-253` | Guard asymmetry — §6 H-storage2 |
| M15 | Physical-slot sorting: resident prefill routes sorted by bank slot once a group reaches 64 routed rows, restoring MLX's sorted gather-QMM path (bit-identical) | doc §perf + `integrate.py:75-77` counters | Amortization threshold = decision content |
| M16 | Gate/up bank fusion: one preallocated quantized bank + one gather-QMM for both projections | `integrate.py:115-116` + doc | 4th member of the same-input family — rider, §5 |
| M17 | Resident substitution: max relative router-weight loss threshold; original weights retained + renormalized; Qwen-only; 0% = exact; 100% = resident-set-only | `routing.py:23-38`, `integrate.py:359-369` | Partial prior (BuddyMoE); kept disabled pending quality eval |

## 4. Prior art (web-verified 2026-08-31)

- **Class published, densely:** MoE-Infinity (arXiv 2401.14361, OSDI'24 —
  activation-aware expert offloading), Fiddler (arXiv 2402.07033), Pre-gated MoE
  (ISCA'24, arXiv 2308.12066), Mixtral Offloading (arXiv 2312.17238 — LRU +
  speculative prefetch), EdgeMoE, ProMoE (arXiv 2410.22134), CommitMoE, OD-MoE.
  **We are not mining novelty of expert streaming** — we mine the mechanisms.
- **REAP** = Cerebras, arXiv 2510.13999 ("REAP the Experts") — the manifest
  format source; vLLM llm-compressor already integrates it.
- **No published prior found** for: cross-run persisted expert-popularity
  hotlists warmed on next load (M3/M4), and speculative execution with KV-cache
  rollback + retry preserving exact routing (M7). Partial prior for M17
  (BuddyMoE, arXiv 2511.10054 — resident substitution without the
  threshold/renormalization contract).
- DwarfStar/ds4 = engine engineering (no paper), verified fork-of-antirez/ds4.

## 5. Coverage map — existing corpus rules vs Batch 94 candidates

**Covered (no new rule; cross-ref only):**
- KV/SSM rollback-on-reject → **B71 `rotated-ring-undo-via-rebind-reference-snapshot`**
  (same lineage — our rule came from this repo's earlier code; B79
  `staged-append-rollback-lifo-retry-blocks` adjacent). The PR's delta is
  policy (when to speculate), not undo mechanics.
- Fixed-bank address stability under a live captured graph → **B58
  `captured-graph-bakes-buffer-addresses`** + **B39
  `slow-path-bootstraps-raw-buffer-hotswap-cache`** (in-place weight hot-swap).
- Gate/up single-bank fusion → **B63 `same-input-gemv-weight-concat-at-upload`**
  (4th member: B58 segments, B63 weight-concat, B77 quantize-memo, now
  bank-level fusion) → **rider on B63**, not a new rule.
- Overlapped group loading → **B36 `sibling-dispatches-before-any-readback`** +
  `independent-cpu-work-fills-dispatch-read-window` family (bounded, one-in-flight
  is the delta but the mechanism family is ours).
- Row-addressable reads (only addressed rows cross the bus) → storage-side twin
  of **B37 `kv-window-device-resident-ring-append`** (append-vs-snapshot) and
  the B56 rider `assigned-columns-only-staging-dequant` (device columns ↔ SSD
  rows). Diffed: B37/B56 are device-axis; the PR is storage-axis. Kept as a
  candidate (R8) rather than a rider because the failure shapes differ
  (mmap-bloat / header random-read amplification vs bus traffic).
- Resident substitution → **B84 `token-local-only-approximate-accelerator`**
  family (substitution is token-local in the state-horizon sense; renormalized
  threshold = the bounded-approximation contract; author keeps it disabled
  pending quality eval — same honesty shape). No new rule; recorded.

## 6. Batch 94 rule candidates (mine-time verification required)

**kernel_opt D (host orchestration):**

- **R1 `cross-run-profiles-key-on-content-not-stat-triplets`** — warm-start
  profiles fingerprinted `path\0size\0mtime_ns` (`integrate.py:196-200`): an
  mtime-preserving copy of a DIFFERENT checkpoint silently matches the stale
  profile; any copy/touch invalidates a still-valid one. Fix shape: hash the
  shard bytes (house rule: BLAKE3, our `EMPTY_HASH` lesson — "a fingerprint
  whose derivation is never asserted is an unverified claim"). Correctness-axis
  rule (B44 `serialize-container-canonical-order` precedent). Sibling hazard:
  `_hotlist_identity` crashes on non-indexed checkpoints (unconditional
  `index.json` stat, no try — kills the whole install).
- **R2 `fill-gate-speculation-until-working-set-resident`** — speculation held
  until EVERY layer's tier is at capacity, readiness latched, held-passes
  counted (`execution.py:240-244,235-237`). The author's own honest datum is
  the justification: speculation at 100% hit is 4% SLOWER (0.579 vs 0.556 s),
  so admitting a partially-filled cache buys only rejected whole-model passes.
- **R3 `decayed-frequency-eviction-recency-tiebreak-route-protection`** —
  hotness halves every N routed tokens (`pool.py:431-434`); evict min-hotness
  with recency tie-break (`pool.py:528-534`); score hits AND misses;
  current-route experts excluded from victims (`pool.py:516-520`). Bench axis:
  hot-set retention/hit-rate on synthetic route traces vs plain LRU/LFU.
- **R4 `defer-promoted-write-to-first-consumer-dispatch`** — binding (route→slot)
  and materialization (bank bytes) are separate phases; materialize at the first
  consumer QMM; the next layer's sync is the implicit fence (`pool.py:411-413`).
  Honest caveat: lazy-eval frameworks get part of this free; the decision
  content is the two-phase split + fence identification, not the laziness.
- **R5 `rollback-snapshot-covers-all-mutable-state`** — the PR's `CacheSnapshot`
  (a) treats `np.ndarray` as an immutable leaf by REFERENCE (a cache mutating a
  numpy buffer in place after capture defeats the restore — rejected-pass state
  leaks into the retry, `execution.py:33-34,60-62`); (b) silently skips
  `__slots__`-style state (`getattr(value, "__dict__", None) is None → return`,
  `execution.py:96-97`); (c) scopes the snapshot to the conventionally-named
  `cache` argument only — attribute-held state outside it is never restored
  (`execution.py:219-221,283`). Defense rule for B71/B79-family rollback
  implementations; also relevant to our own Issue 746 append-only-KV tripwires.
- **R6 `scratch-bank-isolates-one-shot-operands-from-hot-cache`** — one-shot
  cold experts execute in a SEPARATE fixed bank so they never evict hot rows
  (evictions 42 → 0, 2.12–2.19× on the miss-heavy fixture). Anti-shape is
  in-source: the no-scratch expert-major path chunks ALL cold experts including
  resident ones and can evict a later group's experts → bounded double-I/O
  churn (`pool.py:852-858` vs the scratch path's own comment at `:902`).
- **R7 `sort-consumer-routes-by-physical-slot`** — sort resident routes by bank
  slot once a group reaches 64 routed rows, restoring the vendor's sorted
  gather-QMM fast path; bit-identical; +26.6% prefill at 1024×256. Decision
  content: the amortization threshold + "physical order is an API contract with
  the gather kernel".
- **R8 `row-addressable-operand-index-not-full-tensor-map`** — header-only
  offset index; read addressed quantized rows (weight+scales+biases) without
  payload mmap; storage-axis twin of B37/B56 (see §5 diff). Byte-overflow
  hazard class: the schema path floor-divides `data_bytes // num_experts`
  WITHOUT the even-division guard the legacy path raises
  (`safetensors.py:269` vs `403-404`) — silent mis-addressing of every row
  after 0 on an uneven tensor. The `merkle_root` lesson class (guard in one
  constructor, missing in its twin).

**rust_perf:**

- **R9 `coalesce-nearby-reads-bounded-gap-cacheability-gated`** (io) — merge
  reads within a bounded gap (64 KiB) into one `pread`; the coupling is the
  decision content: direct/no-cache mode coalesces with gap 0 because the
  over-read bytes are only free when the page cache absorbs them
  (`safetensors.py:448,560-562,597-598`). Rider: Darwin hints are
  `fcntl(F_RDAHEAD)`/`fcntl(F_NOCACHE)` applied ONCE per persistent descriptor
  (`:536-549`), advisory-swallowed — not per-read `posix_fadvise`.
- **R10 candidate folded into R1** (content-keying is one rule; its cache-tier
  cousin B31 `autotune-cache-extract-outlives-owner` gains a cross-ref).

**Riders (not new rules):**

- **Rider on B63** — bank-level gate/up fusion as the 4th same-input member
  (QMM 3 → 2 per layer; the bank is the concat, preallocated at load).
- **Rider on B71 `gpu-api-thread-affinity-snapshot-metrics`** — the
  producer-side complement: prefetch WORKERS decode to host bytes only; device
  array construction stays on the owner/inference thread because GPU streams
  are thread-local (`safetensors.py:688`). B71 documented the observer role;
  this documents the staging role — same mechanism, second half of the family.
- **Cross-ref on B84** — resident substitution as the routing-shaped member of
  the token-local-approximate family (threshold + renormalization contract;
  disabled pending quality eval).

## 7. Bug yield (hazards inventoried for corpus defense; not ours to file upstream)

Strongest, all anchored (full list preserved in the subagent extractions; the
four below are corpus-grade):

1. **Stat-triplet fingerprint** (R1 above) + crash on non-indexed checkpoints.
2. **Warm-start scale confusion** — lifetime counts (~10⁶) seeded into the
   DECAYED hotness field: seeded experts stay eviction-immune ~log₂(count)×16
   routed tokens with zero recent use; asymmetric (only MISSING experts get
   seeded) (`pool.py:497-499`). Split-brain scoring: eviction ranks by decayed
   score, the persisted hotlist by undecayed lifetime counts — the two
   disagree after long sessions (`pool.py:451-457` vs `530-533`).
3. **Speculative placeholder poison** — non-resident experts map to slot 0 (a
   PINNED slot) and the rejected pass COMPUTES on those weights
   (`pool.py:182,597-599`); contained only by the reject-and-restore contract.
   Any future consumer of the speculative output before inspection reads
   garbage.
4. **Reader fd race** — `_fds` dict get/set unsynchronized under concurrent
   first access → double `os.open`, loser's fd leaks (never in the dict, never
   closed); stats counters non-atomic; `close()` vs in-flight read race
   (`safetensors.py:526-531,638,743-746`).
5. Also: latch is runtime-global not per-request (multi-request server
   speculates on a fresh cache); promote-failure leaves a partially-promoted
   bank (no compensating rollback); install failure leaves the model mutated;
   silent cache-budget inflation (min top-k raise); `fused_half` + separate
   storage = guaranteed loud decode failure; class-wide monkeypatch never
   unpatched (identity-guarded, so correctness holds).

## 8. Measurement lessons + league relevance

- **External corroboration of Bench 649 / the quiet-box rule:** pedroraft's
  review measured the SAME config 90 minutes apart differing **10.6%** purely
  from desktop swap pressure (2.9 → 16.9 GB), noise floor inflated **~15×**,
  and — load-bearing — **the comparison REVERSED**: contended, 144/0 looked
  1.3% better; quiet, 112/32 is 2.7% better ("An earlier version of my own
  conclusion here was backwards for exactly that reason"). Independent arrival
  #4 at the same rule (ours: Bench 649's 12.3× χ² swing; Bench 761/768
  quiet-box; Issue 730's load-disclosure).
- **Metric contamination class:** `hit_rate` is not comparable across scratch
  settings because prefetch-supplied experts never count as cache hits (58.2%
  vs 75.8% at nearly the same speed) — the Bench-749 "instrument must measure
  the thing it claims" class.
- **Admission must include runtime growth:** `cache_experts=160` passed the
  residency estimate then OOM'd on the SECOND generation under a 3,343-token
  prompt — nothing reserves KV growth on top of the bank
  (`kIOGPUCommandBufferCallbackErrorOutOfMemory`). Mirrors our CCU/sizing
  discipline (Bench 034): admission = static model + working-set growth.
- **Benchmark blind spot:** the shipped bench takes `--cache-experts` but
  hard-codes `scratch_experts` — the tool cannot sweep the split its own
  review exposes. Gate-design lesson: a bench must expose every knob whose
  tradeoff it claims to characterize.
- **League (perf-rematch):** DwarfStar/ds4 publishes DeepSeek V4 Flash q2 on
  **M3 Max 128 GB: 26.68 t/s decode, 58.5 t/s short prefill, 250.1 t/s long
  prefill** — a live opponent data point on our home silicon, alongside
  llama.cpp/vLLM/MLX. This PR's streaming mode is what makes Flash-Next-class
  checkpoints (99 GiB) run on 18–36 GB Macs at 15–17 t/s — the queue's
  Flash-Next re-arm condition ("≤24-GB-feasible variant") should be read as
  "RAM-feasible **or streaming-feasible**" going forward; owner call whether
  the league adds a streaming opponent.

## 9. Verdict mechanics

- Novelty gate: not applicable as a novelty claim (external source evaluation);
  the two PR mechanisms with no published prior (cross-run hotlists, exact
  speculative-rollback-retry) are recorded as prior-art facts, not as our
  claims.
- MOAT gate: corpus + defense rules strengthen the healer/serving corpus;
  zero game-domain coupling → all value lands in riir-clippy. No riir-ai /
  katgpt-rs files (no-default-consumer discipline: our league models are
  dense; the streaming lane re-arms only if the league adds an MoE opponent).
- Three-track: modelless-inference serving infrastructure throughout; no
  training content; adversarial panel skipped (no training-adjacent framing —
  hotlists are runtime-observed counts, not GD).
