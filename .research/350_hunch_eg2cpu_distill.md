# Research 350 — undrgrnd-labs/hunch `eg2cpu` distill: the EmbeddingGemma-2 CPU engine (SME/NEON/AMX) + the request scheduler

**Status:** DISTILLED — pending owner decision (mining verdict; batch staged as Plan 311, nominal batch number queue-assigned at landing, behind the live queue).

> Source: `undrgrnd-labs/hunch` @ `22cf73010b0d399894068b1b9d21c5e4101514b6` (shallow clone 2026-10-29 into `.raw/hunch`, read + removed at the verdict commit). **MIT OR Apache-2.0** (LICENSE files verified present). User-directed dual session (@research + @distill); the decision-tier half is riir-rethink Research 015 + Issue 042. Provenance: Google EmbeddingGemma 2 (Apache-2.0 checkpoint, not included) + the repo's own kernels (no upstream port chain beyond the model card; the SME/NEON/AMX kernels are original work, measured in `docs/performance.md`).

## Inspection (what was read, at the pin)

`eg2cpu/Cargo.toml` (release `lto=fat`, `codegen-units=1` — they benchmark; `memmap2`+`rayon`+`tokenizers`, zero GPU deps) · `src/simd.rs` (1,626 L — runtime-dispatched AVX-512/AVX2/NEON/scalar trait family, `EG2_ISA` cap for testing fallbacks, bf16 `WeightElem` loads + RNE `f32_to_bf16` matching hardware `vcvtne2ps2bf16`/`BFCVTN` quieting) · `src/sme.rs` (411 L — ARM SME fp32 FMOPA ZA-tile GEMM + bidirectional MHA, 512-bit SVL gate, 32-column weight panels, `EG2_SME=off` demote) · `src/sched.rs` (183 L — the cross-caller scheduler, read in full) · `src/model.rs` (EarlyExit/layer_means/resume) · `src/decision.rs` (870 L, the compiled handles — decision-tier surface, see rethink Research 015) · `docs/performance.md` (the measured tables) · `hunch/src/hunch/proxy.py` (the lifecycle). Unread at verdict: `vision.rs` (475 L, image preprocessing), `tensors.rs`, `python.rs`, `tests/` — re-read at mine time if the batch touches them.

## The routing (domain table applied)

CPU SIMD + concurrency + measured Apple-silicon matrix findings → **`rust_perf`** (simd + concurrency categories). NOT `kernel_opt` (SME is a CPU matrix unit; the CPU-SIMD-is-not-kernel_opt trap in reverse — no GPU anywhere). The Python proxy is out of scope for the corpus; its lifecycle is lane intel (rethink).

## Dedup gate (both vocabularies, recorded)

- `early.exit|round.robin|linger|collection window` over `src/domains/`: hits are hash-map probe early-exits (entries_08/03 — different class) and **entries_28's serving-slot-selection rule** (llama.cpp #30567, empty-first LRU victim ordering) — KINSHIP recorded: that orders VICTIMS inside one pool; the scheduler candidate is caller-FAIRNESS ACROSS pools of requests. Distinct fix-space; not a duplicate.
- `bfmmla|fmopa|amx|sme`: **zero hits in both corpora.** Novel.
- `pool.*thread|rayon`: the rayon family exists (Lattimore B136 precedent) — no per-length pool-sizing rule. Novel.
- Lane-vocab check `thread pool|dispatch window|batch window`: zero.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `caller-fair-round-robin-dispatch` (collection window + worker linger + longest-first-within-caller + `max_in_flight` cap + per-job panic isolation) | `sched.rs` (read in full) | **B+ decision content, NotBenchable-by-domain** (fairness/contention latency — the dial9/B136 annotation class, never a demotion) | The full named law: lone request runs immediately; under contention the first idle caller becomes dispatcher, waits `EG2_BATCH_WINDOW_US` (1 ms) for stragglers when under `max_in_flight`, then workers take jobs ROUND-ROBIN ACROSS CALLERS (BTreeMap range-scan past `last_caller`), longest-first WITHIN one caller's batch ("so its batch doesn't end waiting on one long text"), workers linger `EG2_LINGER_US` (2 ms) instead of tearing down ("callers that resubmit as soon as their result arrives... otherwise find the workers gone and the whole round restarts"), results publish per-job. Two regimes (uncontended solo vs contended dispatch) + a measured motivation (86 req/s @ 32 callers, p99 466 ms — README). Kinship: entries_28 (victim ordering) is placement, this is scheduling. |
| 2 | `bfmmla-below-fp32-fmla-apple-m5` (simd hazard / anti-regime row) | `docs/performance.md` + `simd.rs` bf16 path | **B measured hazard** | Verbatim: "a P-core issues 2.1 G `BFMMLA`/s (68 GFLOP/s), half its fp32 `FMLA` rate, so the bf16 path is slower than NEON fp32" — with the full A/B table (matmul 90–93 vs 65–68 GFLOP/s one core; 1×16tok 16.8 vs 22.5 ms; e2e ×1.4 worse) and the coupling note (SME takes matmuls when present, so the bf16 path only matters SME-off). The corpus's "so the shape is not re-tried blind" class — a measured anti-regime with a hardware reason. Fixture: aarch64 feature-gated bf16-vs-f32 dot/GEMM micro-kernel, original. |
| 3 | `per-length-thread-pool-selection` (+ the matrix-unit-straggle corollary) | `docs/performance.md` pools table, `EG2_POOLS` env | **B− content** | Pool size keyed by input length (`40:8,160:12` — lengths ≤40 tokens → 8 threads); measured 2× at 32 tokens between 4 vs 8 threads ("With 4 threads, a 32-token text takes twice as long as a 16-token text"); SME corollary: the matrix unit is per-cluster, so over-threading lands tasks on E-core units and straggles ("The vision tower runs on 8 threads: on more, matmul tasks land on the efficiency cores' slower matrix unit and straggle"). Regime: batched/variable-length small-forwards on heterogeneous-core CPUs. NotBenchable-by-domain (needs the encoder workload); content carries both directions (under- AND over-threading measured). |
| 4 | `sme-fp32-fmopa-weight-panel-gemm` | `sme.rs` | **NOT a corpus rule — lane intel, routed** | A whole ZA-tile asm kernel family, not a bounded fix-space pattern; the Rust fixture would be the kernel itself. VALUE GOES TO riir-infer laya CPU posture instead: fp32 FMOPA at "about 1.5 TFLOP/s from one core and 3 TFLOP/s for the chip", driven directly with "results stay within rounding of the reference (cosine ≥ 0.9999998)", matmuls "2–2.7 TFLOP/s on the vision tower's shapes", 2.2–2.7× PyTorch fp32 CPU. Offered follow-up (owner-gated): a riir-infer issue recording SME availability (M4/M5 class — amo/bmo qualify, M3 does NOT) + the BFMMLA negative + the per-length pools as laya-CPU lane intel. |
| 5 | `partial-forward-resumable-early-exit` | `model.rs` EarlyExit + glossary | **C, drop-leaning** | Real machinery (bit-identical resume, "Layer 8 takes about a third of the full pass", resume 8→end 44 vs full 70 ms) and zero dedup hits, but the pattern is application-shaped (encoder layers + consumer-declared readout depth); the decision-tier value (per-question layer readout) lives in rethink Research 015 §deltas-3. The batch may drop with the reason recorded. |
| 6 | bf16 RNE conversion matching hardware quieting (`f32_to_bf16` ties-to-even, NaN quieted "as the hardware conversions `vcvtne2ps2bf16` and `BFCVTN` do") | `simd.rs:115-118` | DROP | Positive instance of existing numeric-identity fixture hygiene; no new decision content. |

## GOAT candidate test (the two-class rule applied)

No candidate passes all four criteria — **criterion 1 fails for DOMAIN reasons** (fairness/contention tables, hardware-gated matrix units, encoder workloads all live outside the PerfBenchValidator's host shape): per the dial9 correction + B136 precedent these are **per-rule NotBenchable annotations, never a demotion**; the corpus lands suggestion/decision-content entries as first-class, and THIS source ships source-measured tables (stronger provenance than B136's zero-number source). Criteria 2–4: exact semantics trivial for the scheduler fixture (byte-identical output order); magnitudes source-reported (2× thread cells, ×1.4 BFMMLA e2e, 86 req/s); deterministic. The strongest rule is candidate 1 (B+ content, three named sub-mechanisms); the strongest overall finding is candidate 2's measured anti-regime (the only row that could later graduate to a measured-GOAT on an M5-class box — amo — if a bench harness ever runs there).

## Lane-intel axis (mandatory second output)

- **League lanes: N/A — stated, not skipped.** hunch is an embedding+head proxy, NOT a serving runtime for our league models; no perf-rematch cell maps. No scoreboard row exists or is proposed.
- **Decision-tier lanes: HIGH** — the full mapping is rethink Research 015 / Issue 042 (audit-demotion lane, one-sided posture, per-question layer readout → Issue-1106/tokengate lane; teacher-tuning + gold-mixing → the RIDT capture lane). This distill's share of it is candidates 3+5 (the engine-side halves).
- **laya CPU posture (riir-infer)**: candidates 2+4 + the per-length pools are directly binding IF a CPU serving posture ever matters on M4/M5 (amo/bmo have SME; M3 does not). Offered follow-up named in the queue line: file the riir-infer lane-intel issue on owner go.

## Product-coverage axis (mandatory third output)

Which ecosystem surfaces do these patterns live in? **std::sync/rayon servers (candidate 1)** — any in-process batched multiplexer; our own reflex/instinct serve edges and the e2e harness are consumers ("fires at home", the dial9 lesson — issue-ledger zero ≠ surface zero). **aarch64 SIMD/bf16 kernel code (candidates 2–3)** — our own laya CPU lane, riir-infer simd/, any feature-gated NEON/BFMMLA kernel in the workspace. Family pools for continuation: std/rayon concurrency → the Lattimore posts (already mined, B136) + tokio blocking docs (adjacent family); aarch64 matrix units → Apple's AMX/BF16 toolchain guides + Arm SME ACLE docs. Accumulation clause applies.

## Verdict

**YES B− corpus [2–3 own rust_perf rules: candidates 1–3, one B+ content lead + one measured hazard + one content row; candidate 4 routed as lane intel, candidate 5 drop-leaning] / league-intel N/A (stated) / decision-tier intel HIGH (rethink 015/042) / product-coverage MODERATE (std/rayon servers + aarch64 simd hazard families, fires at home).**

Yield estimate: 2–3 rules after ~50% coordinator falsification (candidate 5 the likely drop; candidate 2's fixture must stay baseline-ISA + feature-gated, never paste their asm). Caveats: source-measured numbers are M5-Pro/Sapphire-Rapids provenance, never bars; the scheduler's fairness numbers (p99 466 ms @ 32 callers) are THEIR workload — quoted as motivation only; `vision.rs`/`tensors.rs`/`python.rs` unread — re-read at mine time; the repo is young (31 commits) but COMPLETE for what the batch needs (the engine modules are done; the WIP surface is examples/docs). Nothing here touches the corpus until the batch plan runs.

## Recording

- Queue line: `.distill/001_mining_queue_snapshot.md` (this verdict, dated 2026-10-29).
- Batch plan: `.plans/311_hunch_eg2cpu_batch.md` (nominal batch number queue-assigned at landing; staged behind the live queue — B305 claim, B300/B301 foreign, plan-306).
- Decision-tier artifacts (same session, sibling repo): riir-rethink `.research/015` + `.issues/042`.
