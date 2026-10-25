# Decode re-pin bout — PRE-STAGED 2026-10-20 ~00:5x UTC (shikuwa/4090 prep session)

## b10819 PRE-STAGE COMPLETE 2026-10-25 ~12:1x +07 (shikuwa/4090 idle lane — the fired `prism-b10819-909f0ca` trigger's 4090-side prep; GPU window closed mid-prep, the RUN stays queued)

**Done this session (all zero-GPU at write time):**

1. **Fork fetched + checked out + REBUILT at the new tag**: `E:/git/vendor/prismml-llama.cpp`
   HEAD = `909f0cac9c` (tag `prism-b10819-909f0ca`), detached (was b10809 `b3ebc47c28`;
   the fetch moved prism `63f16c408f..909f0cac9c`). Incremental MSVC Release rebuild GREEN;
   `build/bin/Release/llama-bench.exe` fresh 12:06 at `909f0cac`. Cache posture verified
   byte-identical to the pinned bout posture (VS 17 2022, GGML_CUDA=ON, GGML_CUDA_FA=ON,
   GGML_CUDA_GRAPHS=ON, GGML_NATIVE=ON, CMAKE_CUDA_ARCHITECTURES=89).
   ⚠ **BUILD LESSON: `-j 8` DIED with `ptxas fatal: Memory allocation failure` +
   `LLVM error: out of memory` + C1356 mspdbcore.dll (the first attempt also hit the
   25-min tool timeout with `| tail` swallowing all output — stream to a log file
   instead). Retry at `-j 4` = clean pass. nvcc's fat template-instance matrix +
   CUDA 13.3 does not fit 8-way on this 32 GB box alongside the desktop.**
2. **Model sha256 re-verified byte-exact**: `3907dc1658db1f78a9826bf8d5bcb8dc65db0d466388937af57f2294fae62ec1` (same pin as the b10787 bout).
3. **Ours-side tuple clean == origin at prep time** (riir-ai `fd3516b65`, katgpt-rs
   `a91b4ded7`, riir-infer `c598a9e` — all docs-only since the bout-relevant tips) →
   **NO wt-bout tuple needed**; attestation = riir-ai HEAD at run time (Bench-983
   precedent). RE-VERIFY at the window (step 6's stop-condition stays armed).
4. **No `.run_lock`** in `E:/git/riir-ai/.benchmarks/rematch/` at prep time.

**The b10787→b10819 delta (10 commits) adjudicated for sm_89:**

- **#393 `63f16c408f` — LOAD-BEARING**: cuts ~55-65 µs host time per decode step
  (batched sched input uploads, no node scans on CUDA graph replay; byte-identical).
  Host-side, every NVIDIA box with graphs — the decode cells' expected opponent gain
  (~0.5-0.6% at our ~10.5-11 ms/step decode cadence).
- **#391 `e75baeeb24` — sm_89-RELEVANT, 27B-caveated**: fuses residual ADD + RMS_NORM +
  MUL + signed FWHT into one launch at both norms, bit-identical; arch-gated only by
  `GGML_CUDA_CC_IS_NVIDIA` (verified in-diff — NOT Hopper-gated), so it fires here.
  Their measured 9B decode +4.8-5.4% on H200 but **27B NEUTRAL** — our model is the 27B;
  expect little at pp/tg cells, disclosed not assumed.
- **#387 `16cedb2445` + #383 `97ea5aeee1` + #394 `8bd114795e` — INERT at sm_89 without
  env**: #387 chunked MMA GDN prefill is opt-in `GGML_CUDA_GDN_MMA=1` + Hopper-enabled;
  #383 GDN decode fusion is opt-in `GGML_CUDA_GDN_FUSION=1`; #394 PTQ1_0 mat-vec is
  Hopper-class. llama-bench sets none of these.
- #392 (x86 CPU vec_dot) / #389 (sampler host time; llama-bench's greedy path barely
  touches it) / #325 (PTQ1_0 Turing/Ampere; we serve PQ2_0) / #296 (tests) / #400
  (build fix): bench-inert or negligible for our three cells.

**Bout-time steps (when the GPU verifies idle; ~15 min after this pre-stage):**
unchanged from the template below — idle gate → manifest re-pin ×3
(`d00e252587` → `909f0cac9c`; prior_pin extends with the 10-20 Bench-992 row: decode
93.48/88.75 = 1.0528×, pp2048 3991/3360.88 = 1.1875× PASS, pp4096 3803/3332.56 =
1.1412×; delta note = #393 host-time cut + #391 Hadamard fusion [27B-neutral datum]
+ the opt-in/Hopper trio inert at sm_89) → run the 3 cells → adjudicate (decode hold
≥1.0×; the ±8.4% clock band watches #391's fusion; G1 pins MUST re-print: decode
`98412dc406c4ea19` + `63496f6688c8f742`, pp2048 `0f8fddb9ef217548`, pp4096
`23a39649102714ec`) → record (next .benchmarks number — highwater + dual-allocation
check at write time) → push riir-ai ONE commit. New bench record for the escalation
question: the watch's second-pass prediction was "a REAL escalation event, ~+3-8%
opp-side vs the 1.0502× decode hold" — #391's own 27B-neutral datum suggests the
realized opp gain lands BELOW that band; measure, never assume.

---

**Status: DISCHARGED 2026-10-20 ~13:50-14:10Z** — the b10787 bout RAN from this runbook verbatim;
record = riir-ai `.benchmarks/992_prism_b10787_bout.md` (all holds held + prefill WIDENED,
decode parity-flat, NO escalation trigger; ours tip `fea367f2c`, G1 pins EXACT ×3). This
file now serves as (a) the 4090-side standing prebuild intel (fork rebuilt at tag
`prism-b10787-73bfe6c`, llama-bench fresh, model sha verified) and (b) the runbook
template for any FUTURE re-cut window. Nothing below is pending. Marked stale-discharged
2026-10-24 (zed-m3 idle loop — the file's waiting status was never flipped at landing).

**What this was:** the `prism-b10787-73bfe6c` decode re-pin re-cut trigger (fired at B261,
2026-10-20) fully PRE-BUILT. The bout itself was GPU-gated (StartLux llama-server
resident; window ended ~13:30Z Oct 20). At the verified-idle window, the run was ~15 min:
one manifest edit + one harness command + the bench record. Everything else was DONE.

## Already done (this prep, zero GPU, all verified by run)

1. **Fork fetched + checked out + REBUILT**: `E:/git/vendor/prismml-llama.cpp` HEAD =
   `73bfe6cc48` (tag `prism-b10787-73bfe6c`), detached. Incremental MSVC Release rebuild
   (`cmake --build build --config Release -j 8`) green; `build/bin/Release/llama-bench.exe`
   fresh at the tag. Cache flags verified == the pinned posture (VS 17 2022 generator,
   `GGML_CUDA=ON`, `GGML_CUDA_FA=ON`, `GGML_CUDA_GRAPHS=ON`, `CMAKE_CUDA_ARCHITECTURES=89`,
   `GGML_NATIVE=ON`, Release). The +3 CUDA-relevant deltas vs b10776 are compiled in
   (#307 MMA-FA GQA>4 Ada, #330 FA_PREFILL_F16 pool conversion, #368 Pascal mat-vec —
   inert for us at sm_89 except #307/#330, exactly what the bout measures).
2. **Ours pre-built BOTH postures** (`--no-run`, compile-only): the pp posture
   (8-feature set, both `bench_772_t0_gemm_share` + `bench_734_t5_decode` exes) and the
   decode posture (`bonsai2_hadamard` alone). Two transient rustc
   `STATUS_ACCESS_VIOLATION` crashes on `riir-infer-core`/`riir-infer-gpu` lib compile —
   both RETRIED CLEAN and passed on the second attempt (crash-then-pass on identical
   command = infra, not code). Main `target/` warmed: bout-time builds are incremental.
3. **Model sha256 re-verified byte-exact** vs the manifest:
   `3907dc1658db1f78a9826bf8d5bcb8dc65db0d466388937af57f2294fae62ec1`.
4. **Harness selftests green**: run-path 23/23, detector 69/69.
5. **Sibling state adjudicated**: riir-infer / katgpt-rs / neuron-db / chain / dapps /
   auth / kat / shader ALL clean == origin develop; riir-ai == origin `745b3b102`.
   → **NO wt-bout tuple needed** — ours runs from the main riir-ai worktree at HEAD
   (the 10-15 Bench-983 precedent: attestation commit = riir-ai HEAD when clean).

## Disclosed as-found (leave alone; do NOT sweep)

- `riir-ai/Cargo.lock` dirty +1 line: `riir-games-mmorpg` gained optional
  `dep:riir-agents` — plan-638 Phase-4 landing residue (`19d052d8b`, command_lane_escalate;
  Cargo.toml committed, lock line missed). NOT the prep session's file; NOT in the
  riir-gpu dep closure (zero effect on the bout binaries); left uncommitted.
- `riir-train` dirty=4 (the M3's live Plan-465 InFiLoop lane) — as-found, untouched;
  not in the riir-gpu dep closure either.

## The bout-time steps (when GPU verifies idle; ~15 min total)

1. **Idle gate**: `nvidia-smi` — no llama-server.exe / startlux processes; ~370-500 MiB
   desktop-only baseline (the Bench-982 posture). Verify no `.run_lock` in
   `E:/git/riir-ai/.benchmarks/rematch/`.
2. **Manifest re-pin** (3 files, `opponent.commit` field only):
   `.benchmarks/rematch/manifests/4090/bonsai-4090-{decode-tg128,prefill-pp2048,prefill-pp4096}.json`
   — `d00e252587 (tag prism-b10776-d00e252)` → `73bfe6cc48 (tag prism-b10787-73bfe6c)`;
   extend `prior_pin` with one row: `d00e252587 (the 10-15 Bench-983 cells: decode
   89.31/84.28 = 1.0597x, pp2048 3810/3255.76 = 1.1703x PASS, pp4096 3635/3226.32 =
   1.1266x; the +4 delta to b10787 = #307 MMA-FA GQA>4 quantized-K/V Ada + #330
   FA_PREFILL_F16 pool conversion [the PREFILL-content window] + #368 Pascal sm_61
   mat-vec [inert at sm_89] + #321 server-side [bench-inert])`.
3. **Run** (from `E:/git/riir-ai`, bash):
   `bash scripts/perf_rematch.sh run --box 4090 --rows bonsai-4090-decode-tg128,bonsai-4090-prefill-pp2048,bonsai-4090-prefill-pp4096`
   Exclusivity watcher + mid-run gates are live (the 10-13 VOID-and-rerun posture is
   the positive control — a transient-process refusal means rerun that cell, not force).
4. **Adjudicate**: gates are decode hold ≥1.0× (≥1.10 = GREEN stretch, our record
   1.0597×), prefill ≤1.2× behind i.e. ours/opp ≥ 0.8333 — we have been ABOVE parity
   (record 1.1765× pp2048). Watch #330 specifically: it converts quantized K/V to f16
   for prefill-sized FA batches — if their pp2048/pp4096 jump outside the ±8.4% clock
   band, that PR is the named cause (file the delta; the escalation-ladder trigger).
   Ours side must re-print the G1 pins: decode `argmax-walk FNV 98412dc406c4ea19` +
   `logits[::97] 63496f6688c8f742`; pp2048 `argmax 248046 fnv 0f8fddb9ef217548`; pp4096
   `argmax 99125 fnv 23a39649102714ec`. A pin MISS is a wrong answer, never a speedup.
5. **Record**: bench file `.benchmarks/992_prism_b10787_bout.md` (highwater reads 991
   at prep time, 2026-10-20 — verify `.benchmarks/.highwater` AND `ls` at write time,
   and run `py ../katgpt-rs/scripts/dual_allocation_gate.py` before allocating 992). Append the
   matrix rows to the durable league doc, diff vs the 10-15 matrix, update
   `.distill/001` intake row (the 10-19 trigger rule discharges at scoring).
6. **Push** riir-ai (manifests + results + bench record + league doc row in ONE commit,
   `feat:` or `bench:` prefix). Ours-side tips may have moved since this prep — if
   perf-path siblings (riir-infer, katgpt-rs) are no longer clean, STOP and re-adjudicate
   (wt-bout tuple per Bench-982 §Provenance) before running.

## PRE-CHECK ADDENDUM 2026-10-20 ~05:0x UTC (shikuwa/4090 gated-window session, zero GPU)

Re-validated the pre-stage 11h after staging, at riir-ai HEAD `9179ea2bc`:

1. **Harness selftests green again**: run-path 23/23, detector 69/69 (CPU-only, live re-run).
2. **Both bout postures compile clean at current HEAD** (`--no-run`, main target/, incremental):
   decode posture (`--features bonsai2_hadamard`, `bench_734_t5_decode`) and pp posture
   (8-feature set, both `bench_772_t0_gemm_share` + `bench_734_t5_decode` exes).
   En route: THREE rustc `STATUS_ACCESS_VIOLATION` transients this window (attempt 1 on
   `katgpt_core`, attempt 2 on `katgpt_forward` — crash-crash-**pass** on attempt 3; the
   pre-stage's same documented infra class, NOT code; retry-ladder is the remedy).
3. ✅ **THE step-6 TUPLE QUESTION ADJUDICATED (this session — the bout session need only RE-VERIFY no NEW movement):** katgpt-rs moved since the prep (Fathom 628 lane, lease to 2026-10-22): T0.2–T0.6 all inside `crates/katgpt-extract/src/` — and `cargo tree -p riir-gpu --edges normal` proves **katgpt-extract is NOT in the bench closure** (closure = katgpt core/dec/forward/hla/micro-belief/percepta/pruners/personality/sense/speculative/transformer/types). The one closure-adjacent commit is T0.1 `37d09d972` (extraction_wire): a katgpt-core **Cargo.toml feature-list ADDITION only** (new off-by-default feature; the bench postures do not enable it) → fingerprint churn (the crash-crash-pass rebuild recompiled katgpt_core + katgpt_forward), zero compiled-artifact change — the clean `--no-run` builds at the current tips ARE the proof. Both perf-path siblings verified **clean == origin** at adjudication time (riir-infer + katgpt-rs). **Verdict: NO wt-bout tuple freeze — record the tip tuple (riir-ai `9179ea2bc` + katgpt-rs `1f04468de` + riir-infer tip) in the attestation with this outside-closure note; the build is warm and verified, the ~15 min bout budget stands.** If the Fathom lane lands NEW closure-crate commits or dirties a perf-path tree before the bout, re-adjudicate then (the stop-condition stays armed).

**Claim/lease note:** this prep is recorded in riir-refine `.distill/001` (the queue
file) the same commit as this note. The bout session re-fetches before staging per the
standing yield rule.
