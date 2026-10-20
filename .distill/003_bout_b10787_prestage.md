# Decode re-pin bout — PRE-STAGED 2026-10-20 ~00:5x UTC (shikuwa/4090 prep session)

**What this is:** the `prism-b10787-73bfe6c` decode re-pin re-cut trigger (fired at B261,
2026-10-20) is fully PRE-BUILT. The bout itself is GPU-gated (StartLux llama-server
resident; window ends ~13:30Z Oct 20). At the verified-idle window, the run is ~15 min:
one manifest edit + one harness command + the bench record. Everything else is DONE.

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

**Claim/lease note:** this prep is recorded in riir-refine `.distill/001` (the queue
file) the same commit as this note. The bout session re-fetches before staging per the
standing yield rule.
