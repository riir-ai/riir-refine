# Research 389 — Distill verdict: HerRei/local-upscale (LocalSR)

**Status:** DISTILLED — LANDED as Batch 345 (Plan 357): rust_perf 307→312 (entries_70: R1, R2, R5, R6, R7), sec 50→51 (R3), kernel_opt 1075→1076 (entries_gpu_154: R4); R8 dropped at authoring; all gates green, retrieval floors HELD.

## Source

- URL: https://github.com/HerRei/local-upscale (LocalSR — free offline AI image/video upscaling desktop app)
- PIN: `cf6f8b55f407b17141b338ba3955e337fc0f6d12` (tip 2026-11-07T10:51:17Z, "README for the launch" #87), MIT
- Inspection date: 2026-11-09; shallow clone `riir-refine/.raw/local-upscale`, deleted at close
- Provenance chain: ORIGINAL app (no port lineage). The MODELS are third-party (SPAN NomosUni / RealPLKSR NomosWebPhoto CC BY 4.0, Philip Hofmann; HAT face companions excluded by unresolved checkpoint rights — their own call). `src/localsr/video_models/seedvr2/vendor/` is a VENDORED SeedVR2 upstream tree — EXCLUDED from this verdict's surface; it routes to SeedVR2's own upstream, mine THERE (own verdict) if ever wanted. Quotes below are the HOST's integration code, not vendor code.

## Shape

Tauri 2 + Svelte desktop app: a Rust control-plane host (`desktop/src-tauri`, ~11 kLOC: job queue, worker-process supervision, verified downloads/updates, cancellation, loopback media server) + a Python worker (PyTorch/Spandrel inference: tiling, model cache, Core ML runtime, video memory planning) + the vendored SeedVR2 tree. Completeness: shipped beta (v0.3.0), signed/notarized macOS releases, brew cask, dozens of tests incl. `tests/test_checkpoint_security.py`, local CI script. ⚠ NO `[profile]` in the host Cargo.toml — NOT a perf-tuned repo; the grade rests on decision content, never bench figures.

## Verdict

**YES B− corpus** (mixed transcription batch — nominal **Batch 345, Plan 357**) **/ lane-intel NONE** (grep `../riir-ai/.issues/` for `coreml|spandrel|upscale|neural.engine` = 0 hits — no mechanism maps to the inference-perf league; the source's ANE/Core ML use is conv upscalers, not LLM decode) **/ product-coverage YES — tokio concurrency family +1 candidate** (R5, the latest-wins coalescing axis the bounded-backpressure rule does not own).

No benchable GOAT lead exists — named honestly below. Grade rests on decision content × novelty × dedup × yield (the Batch-136 NotBenchable-first-class precedent).

## Ranked candidate table

| # | Pattern | Source (at pin) | Grade | Domain / category | Notes |
|---|---|---|---|---|---|
| R1 | `benchmark-spread-above-threshold-reports-unstable` | `src/localsr/core/benchmark_v2.py` | **B+** | rust_perf / measurement | see below |
| R2 | `macos-pressure-reads-footprint-not-rss` | `src/localsr/worker/edit_guard.py` + `core/hardware.py` | **B** | rust_perf / measurement | see below |
| R3 | `execution-artifact-trust-ladder-before-deserialize` | `core/model_adapter.py` + `desktop/.../commands.rs:1614` | **B** | sec | see below |
| R4 | `accelerator-backend-first-tile-acceptance-gate` | `core/coreml_runtime.py` | **B−** | kernel_opt (host-orchestration) | adjacent to B337 + entries_19 |
| R5 | `slow-consumer-latest-value-single-slot` | `core/live_preview.py` | **B−** | rust_perf / concurrency (tokio family) | adjacent to entries_02 |
| R6 | `untrusted-download-verify-then-atomic-install` (+ re-verify-at-use face) | `desktop/.../downloads.rs` + `updates.rs:782-835` | **B−** | rust_perf / io (sec face) | zero owners |
| R7 | `ui-event-stream-sampled-gate-terminal-forward` | `desktop/.../worker.rs:31-145` | **C+** | rust_perf / io | thin but test-pinned |
| R8 | `spawn-child-clean-dyld-dll-search` | `worker/edit_guard.py` (`native_environment`, `reset_dll_search`) | **C+** | sec (low) | merge-or-drop gate at authoring |
| — | `_fixed_window` fixed-shape real-pixel windows | `core/inference.py:85-130` | annotation | — | decision content real, Rust fixture contrived |
| — | worker restart `sleep(attempt)` ≤3, user-forced restarts don't consume the crash budget | `desktop/.../worker.rs:340-356` | annotation | — | thin; supervisor-library territory |
| — | per-job scratch dir `.localsr-job-<uuid>` + CANCEL_GRACE | `desktop/.../cancellation.rs` | positive instance | — | the shared-temp-path REPAIR form; corroboration row, never a rule |
| — | streaming `minisign verify_stream` | `desktop/.../updates.rs:424-428` | drop | — | API usage, not a rule |
| — | loopback ephemeral-port media server + unguessable session-lifetime URL capabilities | `desktop/.../media_server.rs` | drop-watch | — | request-side handshake family owned (voxl B329); capability-URL axis thin here |

### R1 — benchmark stability gate (B+, measurement)

Docstring verbatim: "**Repeatability over speed.** Warm-up runs until the rolling coefficient of variation settles (or a time cap) … a result whose measured spread exceeds the stability threshold is reported as *unstable* instead of silently producing a different headline score on every run." Constants: `TARGET_CV = 0.05` (warm-up stops once three consecutive rolling CV samples settle), `WARMUP_MIN_ITERATIONS = 3`, `MEASURE_TARGET_SECONDS = 20.0` / `MEASURE_MIN_ITERATIONS = 6` ("whichever is later"), `MEASURE_MIN_SLOW_ITERATIONS = 3` ("Three complete measurements are still required to assess consistency" on a slow CPU), fixed versioned scenes (`WORKLOAD_VERSION = "localsr-benchmark-v2.1"`, "in the spirit of Blender's BMW27"), scene content "generated from integer arithmetic only … so results stay byte-stable across machines and releases", per-device scores ("A run never switches to another device silently").

Dedup: measurement category owned set = entries_19 (calibration-table noise-repair persisted per build) + entries_09 (seam warm at load) — NEITHER owns the report-UNSTABLE verdict shape nor the CV-settle warmup; the timed_region_guard family is gate-side loud-zero defence, not report-side honesty. This rule is the REPORT-VERDICT face of our own house law ("a latency number without its BOX STATE is not a measurement"). NotBenchable-by-domain.

### R2 — macOS pressure reads footprint, not RSS (B, measurement)

`edit_guard.process_memory` docstring verbatim: "On macOS RSS includes the memory-mapped weights, clean pages the system drops under pressure at no cost; the footprint is what Activity Monitor shows. A FLUX.2 klein 4B edit on a 16 GB Mac peaked at 6.4 GiB RSS and 2.6 GiB footprint." Implementation: `proc_pid_rusage` → `phys_footprint` via ctypes, RSS fallback elsewhere. Second face: `hardware._system_memory` macOS branch reads `vm_stat` and sums free+inactive+speculative+purgeable pages as "reclaimable" available memory (NOT total−used; Linux-MemAvailable analog), with a `0.25 × total` fallback on failure.

Dedup: every corpus "footprint" hit is container/register footprint (entries_03/09/18/37) — ZERO owners of the macOS counter-choice law. The class generalizes: "a memory-pressure or limit decision must read the counter the kernel actually acts on (phys_footprint for a process's pressure contribution; reclaimable pages for system headroom), never RSS/total-free" — measured instances carried in-rule. NotBenchable-by-domain.

### R3 — execution-artifact trust ladder before deserialize (B, sec)

`model_adapter._checkpoint_trust` ladder, verbatim shape: catalog match → verify size+SHA-256 else `CheckpointSecurityError("…failed its catalog SHA-256 check. LocalSR refused to deserialize it…")` (= catalog-verified); risky suffix (`.pth/.pt/.ckpt`) without opt-in → refusal naming the env (`"…set {UNVERIFIED_CHECKPOINT_ENV}=1"`) (= explicitly-trusted); else safe-format. The gate runs BEFORE `self.loader.load_from_file(...)` — `test_checkpoint_security.py` asserts "Spandrel must not receive an unverified pickle checkpoint" (fail-closed AT THE SINK). TWO-SIDED: the Rust host independently refuses at `commands.rs:1614` ("pickle-based checkpoints require the explicit unsafe checkpoint confirmation") — neither end trusts the other's check across the process boundary.

The Rust face: a host handing an untrusted artifact to an execution-capable runtime (a wasm module into wasmi/wasmtime, a dylib, a spawned interpreter) needs the same ladder AT THE HOST boundary — safe format / catalog digest / explicit consent — before the artifact is handed over. ADJACENT-DISTINCT adjudications (mandatory at authoring): (a) the 0sec verdict's recorded drop ("pickle/deser + eval DROPPED language-absent — typed serde = fix by construction") owns the deserialize-DATA class; THIS is the execution-capable-ARTIFACT class — the runtime executes arbitrary semantics FROM the file — whose Rust instances are real (our own wasm-validator store manifest `version + blake3 + active/revoked` is the positive instance; the corpus's B294 wasmtime family proves executors are in-scope). (b) `untrusted-archive-extract-paths` owns the zip-slip path axis, not the trust ladder. NotBenchable-by-domain.

### R4 — accelerator-backend first-tile acceptance gate (B−, kernel_opt host-orchestration)

Docstring verbatim: "The first real tile is a gate: Core ML must reproduce the PyTorch result on the same pixels and must not be meaningfully slower than the PyTorch MPS path. The verdict is stored per Mac, macOS version and toolchain, so later jobs skip the gate. Anything that fails keeps PyTorch." Constants: `MINIMUM_PSNR = 45.0`, `NEURAL_ENGINE_SPEED_MARGIN = 1.3` ("The Neural Engine uses far less energy … so keep it unless the PyTorch path is clearly faster on this Mac"), `MAXIMUM_PIXELS = 576 * 576` ("Converting large shapes exhausted 16 GB of memory for RRDBNet (768 px failed, 512 px was measured)"), and the per-architecture `COMPUTE_UNITS` table with measured provenance ("Measured on an M1 Pro (2026-10-24) … HAT, SwinIR and NAFNet produce broken fp16 output and SCUNet does not convert, so every other architecture keeps PyTorch").

ADJACENT-DISTINCT: B337 `compiled-vs-eager-per-op-exactness-admission` is compile-time per-OP admission (no speed axis, no fallback-verdict caching); entries_19 owns the calibration-table noise-repair + per-build persistence (the PERSISTENCE half only). THIS owns runtime whole-backend ACCEPTANCE: correctness floor + speed margin + first-real-input gate + per-machine cached verdict + reference-keeps-the-job fallback. NotBenchable-by-domain (needs coremltools).

### R5 — slow-consumer latest-value single-slot (B−, tokio family +1)

Docstring verbatim: "Inference submits at most one pending NumPy frame. A daemon encoder owns JPEG compression and transport emission, so a slow webview can never build an unbounded frame queue or make the inference loop wait for preview encoding." Rust face: `watch::channel` / single-slot `Mutex<Option<T>>` swap + drop-stale, vs `unbounded_channel` growth against a slow UI/telemetry consumer.

ADJACENT-DISTINCT from entries_02's unbounded→bounded-backpressure rule (Research 032, hotpath-rs): backpressure SUSPENDS the producer; coalescing NEVER blocks the producer and DROPS stale items (latest-wins) — different semantics axis, mandatory adjudication note. Same metric-mismatch NotBenchable class as its sibling ("the buggy path is often FASTER on wall-clock precisely because it buffers"). This is the product-coverage +1: the tokio family gains the coalescing axis.

### R6 — untrusted download: verify-then-atomic-install + re-verify at use (B−, io)

`downloads.rs`: `validate_artifact_metadata(url, expected_size, expected_sha256)` BEFORE the request; `.part` resume accepted only when `partial_size <= expected_size` (else restart); size check on completion; full-partial SHA-256 in `spawn_blocking`; mismatch → `fs::remove_file(&partial)` ("the untrusted partial file was removed"); `install_atomically` rename. FACE (updates.rs): the same artifact is re-verified at EVERY trust-boundary crossing — minisign `verify_stream` at download AND again per part at install (782, 809–815) before the `engine.adopting` staging swap (staging → rename, `backup_profile("before-install")` as the rollback leg). The TOCTOU half may split: `staged-artifact-reverified-at-use` — verification is a property of the BYTES, not the file's history.

Dedup: the sec corpus's "download" hit is the zip-slip rule's regime INSTANCE (corpus.rs:458), not an owned verify-then-promote rule — zero owners. NotBenchable-by-domain (network io).

### R7 — UI event-stream sampled gate (C+, io)

`worker.rs:31-145` `WorkerEventGate`: per-event-type sampling intervals (80–150 ms); duplicated high-frequency snapshot types dropped entirely ("Keep the typed stage event in the protocol, but do not duplicate a high-frequency webview event"); terminal events (`job_completed/failed/cancelled`) ALWAYS forward + reset the gate; structural pairs never sampled ("Position and completion metadata must stay paired even for fast models. Only expensive JPEG payloads are sampled"); last-item force-forward (`is_last_item`). Test-pinned: `high_frequency_worker_events_are_sampled_but_final_progress_is_forwarded`. Rust face: high-frequency telemetry/UI emission into a slow consumer. Thin — likely the batch's drop candidate.

## GOAT candidate identification (honest)

**No candidate passes all four.** Criterion 1 fails for every entry by metric mismatch: the wins are bounded-queue depth (R5), measurement honesty (R1/R2), trust gating (R3/R6), emission rate (R7), backend-adoption correctness (R4) — none is a wall-clock Pareto shape on the perf validator. This is the PATTERN-weak class, not the domain-unbenchable annotation class (tokio/bevy siblings in the corpus ARE Pareto-gradeable; these patterns are not) — so per the two-class split the entries land as suggestion/decision-content first-class rows with `bench_spec: None`, and the grade stands on decision content × novelty × dedup × yield (Batch 136 precedent).

## Dedup-grep evidence (both vocabularies)

- event sampling/throttle → entries_19's calibration `sampl` hit is a measurement-table protocol, unrelated; zero UI-emission owners.
- phys_footprint → all corpus "footprint" hits are container/register footprint; zero owners.
- CV/stability/unstable → zero owners in the measurement set.
- pickle/deserializ in sec → jwt-disabled-validation owns the JWT axis; B294 wasmtime owns executor BUG classes (memory-access, br_table), not trust gating; the 0sec drop is adjudicated in R3.
- watch/coalesc/single-slot → entries_02 unbounded→bounded owns the backpressure axis only.
- download/checksum/atomic-install → sec corpus.rs:458 is the zip-slip instance; zero verify-then-promote owners.
- restart/backoff → entries_39/55 hits unrelated (dispatch/rule contexts).
- Core ML/acceptance → B337 + entries_19 adjacent, adjudicated in R4.

## Caveats

- Consumer-app source, not perf-tuned (no `[profile]`, no benches) — never quote its numbers as perf evidence; they are its OWN harm pricing (6.4 GiB vs 2.6 GiB; 2–3× ANE vs MPS).
- v0.3.0-beta, fast-moving — re-clone + re-pin at mine time; all quotes re-verified at the fresh pin (T0).
- UNREAD at verdict time (re-read rows before entry authoring): `commands.rs` full body (2715 lines — greps only), `updates.rs` full body, `benchmark_v2.py` remainder, `video_pipeline.py`/`video_io.py`/`video_memory.py` remainders, `coreml_runtime.py` remainder, `hardware.py` remainder, `queue_timing.rs` remainder, all of `scripts/` (packaging/release tooling), the Svelte frontend, and the entire `seedvr2/vendor/` tree (excluded — upstream provenance).
- Models are third-party CC BY 4.0 (attribution in-rule, fixtures stay original regardless — the transcription law).
- The per-arch Core ML table rows (R4) are the source's M1-Pro measurements — carry as SOURCE-REPORTED provenance, never bars (entries_18's posture).

## Queue slot

Batch 345 nominal (after 0sec's Batch 344). Plan 357 carries the checkbox tasks. Tokio-family accumulation clause: if the dial9 pool's follow-up batch (Research 179) fires first, R5 rides that batch's window (Batch-145/146 staged-charter precedent).

## Coverage note for the famous-lib axis

The tokio concurrency family gains its latest-wins/coalescing axis (R5). The family's follow-up pool is unchanged (Research 179: Alice Ryhl "What is Blocking?", the official blocking docs, mini-redis); no new pool needed.

Session: zed-m3-localsr-distill, epoch 1794199xxx (committed with the queue line).
