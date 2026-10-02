# Issue 143 — the retrieval GOAT's G2 latency bar fails on the Windows/4090 box at pre-batch HEAD

**Status:** OPEN — filed 2026-10-02 (Batch 196 session, T3 gate adjudication)

## The finding

`tests/latent_suggest_goat.rs::g2_latency_under_1ms_per_chunk` FAILS on this box
(shikuwa, Windows 11, i7-13700K, RTX 4090) at **pre-batch HEAD** — measured
during Batch 196's T3 (Plan 195):

| Run | Tree | G2 reading |
|---|---|---|
| 1 | B196 worktree (+4 corpus rules) | 1493 µs/chunk (480 chunks) |
| 2 | B196 worktree, warm re-run | 1480 µs/chunk |
| 3 | detached worktree at HEAD `c8cc9e86` (pre-batch) | 1458 µs/chunk |

The bar is `< 1ms per chunk` (the Plan 005 Phase 3 GOAT, G2). All three
readings sit in one band (1458–1493 µs, ~2.4% spread = box noise); the batch's
corpus growth (kernel_opt 726→728, rust_perf 165→167 ≈ +0.3%) is inside that
band. **G2 fails at HEAD — this is a pre-existing platform gap, not a batch
regression.**

G1 (retrieval precision ≥80% per domain) PASSES on this box, and the full
latent_retrieval feature lib suite passes (`cargo test --release --features
latent_retrieval --lib` = 2598/0, 75.9s) — the retrieval CONTENT floors are
held; only the M3-calibrated absolute-latency bar is out of reach here.

## Reading

The G2 bar is an Apple-silicon-calibrated absolute (`<1ms`, Plan 005-era, M3).
This box's retrieval path reads ~1.5ms/chunk in release — same modelless path,
different host (Windows scheduler/allocator; the machine that never ran the
gate before — the batch records' "retrieval 21/21 floors HELD (~200s, isolated
dir)" runs are the M3 lane). This is the katgpt-rs arch-conditional-dual-pin
class (T6/T7 precedent): an absolute bar calibrated on one arch read on another.

## The durable fix (owner-adjacent, not this batch's)

Arch/box-conditional dual pins for G2 — e.g. a
`#[cfg(target_os = "macos")]` arm holding the historical 1ms bar and a
measured Windows arm holding a floor in the 1458–1493 band (or a
relaxed-CEILING instrument), with the box state disclosed per the G2
box-state law (`scripts/bench_preflight` spirit). Until then: on this box,
read G2's failure as the recorded platform gap and adjudicate retrieval
batches on G1 + the latent_retrieval lib suite (this issue is that
permission).

## Notes

- The build-script clippy "1 warning" seen in the same T3 (body suppressed by
  cargo on this box; standalone + exact-flag replays clean; `-- -D warnings`
  gate PASSES exit 0 at `--all-targets`) is a second pre-existing box artifact
  recorded in the same pass — non-denyable, not this batch's.
- Batch 196 (Plan 195) landed with: batch196 gates 5/5, default lib 2424/0,
  kernel_opt lib 2824/0, rust_perf_bench 2455/0, latent_retrieval lib 2598/0,
  G1 pass, clippy `-D` ×3 postures green (this artifact disclosed).
