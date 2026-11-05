# Research 376: jitengine — distill-source verdict (MLX/Metal MoE serving engines)

**Status:** DISTILLED — pending owner decision (verdict YES B corpus held: kernel_opt +3–4 own rules + 2–3 faces, sec +1 own rule; nominal **Batch 337** / plan 346 STAGED, unclaimed; all quotes read + grep-verified at the pin this session)

**Source:** https://github.com/skydiscover-ai/jitengine @ `2c34508d617800ab7448a69e14a6dd551aec156e` (Apache-2.0; shallow clone `.raw/jitengine` lived at the pin through the verdict + reviewer round 1, REMOVED at this note's landing commit per the skill's remove-at-verdict law; plan 346 T0 re-clones at the pin before any quote is re-verified).
**Blogs:** https://skydiscover-ai.github.io/blog-jitengine.html (2026-11-04, "Your Mac Deserves Its Own LLM Engine", UC Berkeley Sky Lab; citation block names Cemri, Agarwal, Liu, Krentsel, Sankhe, Dimakis, Stoica) + the methodology twin https://skydiscover-ai.github.io/blog-skysynth.html (distilled separately in Research 377). The blogs cite arXiv:2605.24096 (JIT systems) + arXiv:2605.23109 (IDS) — neither previously distilled in this workspace (internal-first grep zero hits, 2026-11-05).
**Inspection date:** 2026-11-05 (zed-m3-jit-distill, M3; network + reads only; zero cargo, zero GPU).
**Queue slot:** Batch 337 / Plan 346 (STAGED — claim-before-edit at the next kernel_opt/sec slot; yields to the live walk-23 pair claim on 374/375/345, landed first by them).

## What the source is

Six agent-synthesized LLM serving engines for Apple silicon (SkySynth-built, human-code-free): Qwen3.6-35B-A3B (MoE), Qwen3.8-Flash-Next (104 GB MoE), Recursive Flint (40B MoE int4), each for a 24 GB M2 and a 48 GB M4 Pro. Python on MLX 0.32.2 + 14 hand-shape `.metal` kernel files; ~12 K lines/engine; per-engine pytest suites (unit + metal + e2e), benchmark harnesses, and a per-engine `DETAILS.md` design doc with measured numbers. Blog claims: up to 2.72× the fastest rival on the same Mac (Flash-Next 32.9 vs slotstream 12.1 tok/s single-user M4 Pro; Qwen3.6 44.9 vs llama.cpp+MTP 32.9 on M2; Flint 27.7 vs rec 22.6 at 252 K context). Every headline change is bit-identity-gated ("kept eleven changes, each with outputs bit-identical to the build before").

Completeness signals: strong — every engine ships tests incl. `tests/metal/` kernel parity suites and anti-reward-hack test machinery; release-grade packaging (install.sh, launch/ CLI); `.importlinter`. No `todo!()` class. Benchmarked everywhere (measured crossovers pinned inline, e.g. plan.py "40 / 28 beat 64 / 64 by 0.10 ms at 1.5K").

## Verdict

**YES B corpus** [kernel_opt +3–4 own rules + 2–3 count-neutral faces; sec +1 own rule] / **lane-intel MEDIUM-HIGH** / **product-coverage MODERATE** (extends an already-seeded family — see below).

Tempting WRONG domains named: rust_perf (the host half is Python serving policy — only two shapes carry Rust fix-spaces and they route sec/caching); clippy_lints (no parse-only provable shapes).

## Ranked candidate table

| # | Candidate | Source file (at the pin) | Grade | Decision content + dedup |
|---|---|---|---|---|
| 1 | **verify-parity-position-pure-grid** | `engines/flint-m4pro/src/.../kernels/plan.py` | **B+** | "The grid is a pure function of the row's own position, never of N_max / max_tokens / context_limit / capacity, so a greedy or seeded stream is the same whatever the request asks for, and a verify row equals the M = 1 pass at its position across every grid change (rows straddling one simply have different S)." Speculation-verify determinism BY GRID CONSTRUCTION — the Issue-940 determinism family in grid form, and the enabling law for the verify twin (`PASS_M1` macro: one kernel source, M=1 pass and verify pass). Dedup (3 vocab): `pure function\|request.independent\|position only` → zero owners (reviewer spot-check concurred; only hit was gpu_137's unrelated merge-tile law prose). The corpus's speculation family (confidence-windowed MTP, TableTune) owns DEPTH/confidence policy — nobody owns grid-invariance. |
| 2 | **compiled-vs-eager-per-op-exactness-admission** | `engines/qwen3.6-35b-a3b-m2/DETAILS.md` (results table row, L102) | **B** | "Integer ops and the combine's bf16 multiply-add compile to the eager bits (checked on every bf16 pair); float transcendentals do not: a compiled fp32 sigmoid is one unit in the last place off on 1,949 of the 65,280 finite bf16 logits, which changed a raw decode at step 637, so the sigmoid stays eager." Per-op-class admission with a per-pair check and a decode-changing drift refusing the compiled form. THE RUST FACE (reviewer round-1 binding note): Rust has no compiled/eager split — the Rust twin is **fast-math/FMA contraction vs strict scalar** for float transcendentals (integer and bf16-MAC paths exact). Name that face in the fixture or the rule has no detector. Kinship: our laya gelu_erf lesson (candle's tanh gelu ~1000× the gate) is the same genus — cite, never merge. |
| 3 | **gdn-state-snapshot-per-position-verify-early-exit** | `engines/qwen3.6-35b-a3b-m4pro/src/.../kernels/gdn_recurrence.py` (`gdn_recurrence_steps`) + `kernels/README.md` contract table | **B** | "gdn_recurrence_steps stores the state after every position, so a verify pass can stop at the first rejected guess." Per-position recurrent-state snapshots as the speculation early-exit substrate for a linear-attention/GDN layer. Our GDN lane (riir-infer deltanet) materializes chunk state; the verify-early-exit PURPOSE is the unowned atom. Rides the standing serving/spec-dec tier (S8/S9 posture per B319/B321) or kernel_opt direct. |
| 4 | **command-buffer-declared-input-pacing** (Metal) | `engines/qwen3.6-35b-a3b-m2/DETAILS.md` L126-129 | **B** | "MLX commits a command buffer every 40 operations or 40 MB of declared inputs, and each expert call declared the whole 235 MB expert stack, so it committed a buffer on its own. Here each call declares only its own expert's weights and a buffer holds up to 100 operations: 22.05 → 20.03 ms, for about 1 GB more memory." The DECLARED-INPUT-SIZE → commit-threshold economics. Metal dispatch tax; NotBenchable-by-domain + the CONSTRUCTIBLE-NOT-RUN disclosure class (B335 precedent — an M3 Metal A/B is constructible; the landing gate carries a measured cell or the disclosure). |
| 5 | **multi-row-share-weight-read-keep-one-row-arithmetic** | `engines/qwen3.6-35b-a3b-m4pro/src/.../kernels/qmv_mrow.py` + `kernels/README.md` | **B−** | `qmv_fast_mrow(_rl4)`: "keep stock's one-row arithmetic (`qmv_fast`) while several input rows share each weight read, so a verify window or a batch matches one-row decoding at any row count"; per-shape admission "1 row if weight>=64 MiB; 2+ rows in 2-row blocks where rl4 is off", calibrated per shape. Bit-identical multi-row GEMV + weight-size-keyed admission table. Kinship: gpu_13 few-row-MMA rows_min crossover (llama.cpp, per-TYPE) — same family, distinct fix-space (bit-identity contract + per-shape calibration). |
| 6 | **conditional-reuseaddr-after-liveness-probe** | `engines/flint-m4pro/src/.../server.py` + DETAILS.md Server section | **B−** | "A port already in use is refused, also when the other server listens at `0.0.0.0` or `[::]`. The server binds without `SO_REUSEADDR`, so it cannot sit on `127.0.0.1` beside a live server and take its local traffic. It rebinds with `SO_REUSEADDR` only when no live server answers, so a quick restart over a previous server's closed connections still works." sec (the harm is loopback traffic hijack via bind takeover); sec corpus grep ZERO owners (reviewer concurred: `reuseaddr\|reuse_address\|SO_REUSE` → zero). Rust face: gate explicit `socket2::set_reuseaddr` call sites — std `TcpListener` never sets it (safe-by-default carried IN-RULE as the anti-regime). Platform-split decision content (macOS/BSD vs Linux SO_REUSEADDR semantics). Suggest-only, NotBenchable-by-domain (dial9/136 split). Drop-expected allowed without hurting the verdict. |
| 7 | **bulk-phase-recency-only-cache-refresh** | `engines/flint-m2/DETAILS.md` L96 | **B−** | "A long prompt touches nearly every expert of every layer; counting that sweep evicted the experts decode keeps returning to. Prefill now refreshes recency only, and the first decode after an 8K prompt is 8 to 12% faster." Cache statistics scoping keyed to phase: bulk sweeps touch recency, never count toward eviction. Rust-shape: an LRU whose bulk-phase accesses refresh recency but record no hits/evictions. Caching family; dedup clean on `recency.*only\|bulk.*refresh\|stats.*scope`. |
| 8 | **compute-experts-before-router-lookup-fixup** | `engines/flint-m2/DETAILS.md` L99+L101 | **B−** | "Each layer's experts are submitted on a device-side copy of the slot map before the host's lookup finishes; when the lookup finds every expert resident, that work is kept, and otherwise the layer is rebuilt on the real slots" (+9.4%/+18.0%) refined by "only the missing experts' rows are rebuilt, not the whole layer" (+1.2–2.2%). Dependent-compute-ahead with device-side snapshot + MISS-ONLY rebuild. Kinship: gpu_111 compute-ahead tile-copies — the MoE twin; the partial-rebuild granularity is the delta. |
| 9 | **output-neutral-clock-keeper** | `engines/flint-m2/DETAILS.md` L94 | **C+/B−** | "While a request runs, a helper process streams tiny kernels on its own Metal queue, and the GPU stays at full clock. It shares no memory with the engine, so it cannot change the output. This is the largest gain: 9.3 to 13.6 tok/s." (GPU idles at ~800 of 1,398 MHz between SSD-read bursts; `FLINT_GPU_KEEPALIVE` env.) The decision content is output-neutrality BY PROCESS ISOLATION (no shared memory → provably cannot change results), not clock management per se. Niche; honest carry allowed. |

**Faces (count-neutral):** router top-k reproducing `mx.argpartition` "exactly, tie order included" → Issue-940 tie-discipline face on the numerics family; MoE team counters `[49][64]` "on 64-byte lines (synchronisation state, zeroed by every pass; **never compared**)" → sync-state-is-not-data face adjacent to gpu_30's sentinel/epoch family; "every uint4 load is issued before the first store (one memory round trip)" (megakernel_prelude L62) → positive instance of gpu_75's TK-batched prefetch (corroboration row, never a rule).

## Corroborations (recorded, never re-landed)

- jitengine `tests/unit/test_host_origin.py` + `--allowed-host NAME` = a positive instance of Batch 329's `browser-reachable-service-host-origin-rebinding-gate` — the corpus's class fires in a fresh third-party codebase.
- `--request-deadline-s` (stops a request S seconds after submission, answers with tokens so far, `x_metrics.deadline_exceeded`) = a positive instance of Batch 316's `per-read-timeout-vs-request-deadline`.
- The megakernel verify twin (`megakernel_verify_layers.metal`, `PASS_M1`) + the per-kernel contract table in `kernels/README.md` (stock op / covers / contract / claim / check / test columns) + the selfcheck machinery (`selfcheck.py` — checks, flags, log, `/stats`) + the blog's two dishonest-engine negative controls = our proof_negative_test / `--canary` / `--prove-fires` / G5-parity discipline externalized in a serving engine. Discipline corroboration — inflating it into rules would be double-counting.

## Drops (with reasons)

- Persistent-megakernel genus (flint's one-persistent-program decode pass) — heavily owned: gpu_51/10/86/24/146 + Batch 333's sgl-nvidia-mega megakernel pair (mailbox rotation, fail-stop trap). The flint grid barrier's counter/generation/error words on 128-byte lines ride the same family.
- TK-batched load-ahead — gpu_75 owns it FROM MLX (the same upstream the engines build on).
- kv8 history profile (8-bit KV in groups of 64) — a lossy-surface discipline instance, not a rule (our lossy-surface promotion rule owns the law).
- Heap-based eviction + 8-worker parallel checkpoint-slice reads — io-family thin, no new atom.
- Turn-taking decode quantum (Flash-Next M2: 64-token slices/request to stop mutual expert eviction; blog Figure 6: batched SSD reads 243.5 → 140.9/token, +65% eight-request throughput) — real decision content but serving-scheduler-deep with a MoE-offload premise; watch row, not a rule (re-open if a Rust serving surface ever needs the shape).
- Compact experts (2.76 vs 3.07 MB/expert) — model-specific repack, no fix-space.

## Lane-intel (mandatory axis)

- **League cells NOT comparable** — MoE models (Qwen3.6-A3B / Flash-Next / Flint) on M2/M4 Pro vs our Bonsai-2-27B PQ2_0 ternary + dense-qwen3.8 lanes on M3/m5ultra/4090. Absolute tok/s never headroom evidence (the serving-recipe law).
- **Transferable atoms for OUR lanes:** (a) the position-pure verify grid + verify twin → any future speculation/verify work in our serving stack; (b) `gdn_recurrence_steps` per-position state snapshots → the riir-infer GDN lane + the serving/spec-dec tier; (c) the declared-input command-buffer economics → our Metal/wgpu host dispatch path (wgpu abstracts encoders; the SIZE-of-declared-inputs law maps to encoder batching) — the ffn_gate_up 28.1% decode lever's host-side neighbor.
- **Not transferable:** FreeToken-style SSD expert offload (we have no MoE-offload lane; Flash-Next-class budgets don't exist in our fleet).

## Product-coverage axis

The MLX/Metal kernel family is SEEDED, not new: gpu_75 (Batch 205, `ml-explore/mlx` sdpa TK=4) + the Batch-276 rider (mlxfast-qwen38-125b-a6b-engine low-residency quantifier — the same MLX-MoE ecosystem, independently mined). jitengine adds the MoE-serving + exactness-contract subfamily. Follow-up pool (accumulation clause, folds into Batch 337's window): mlxfast's other engines, `ml-explore/mlx/backend/metal/kernels/` beyond the mined sdpa file, uzu's Metal path.

## Honest caveats

- Agent-built provenance (SkySynth runs) — the engines are snapshots of synthesis runs; quotes pinned at the clone sha; repos may move. T0 re-clones at the pin.
- Dedup greps this session were targeted at the 8 candidates (corpus 1,056 rules and moving — B335 landed yesterday); the plan's T0 re-runs 3+ vocabularies per candidate at authoring; 1–2 candidates may die there (baked into the yield).
- Candidate 6's Rust applicability is thinner than the source's (std safe-by-default) — drop-expected allowed.
- The blog's numbers are the authors' own measurements under their own memory budgets — recorded as provenance, never as our claims.

## Yield estimate

**3–5 rules** after ~50% coordinator falsification (kernel_opt 1056 → 1058–1060, sec 45 → 46): leads 1–4 + one of 6/7/8; faces 2–3.

## Queue slot

Batch 337 / Plan 346 (STAGED, unclaimed — claim-first at the next kernel_opt/sec mining slot; yields to the walk-23 pair's live claim through 2026-11-07). `.raw/jitengine` REMOVED at this note's landing commit.

Session: zed-m3-jit-distill, epoch 1794055300
