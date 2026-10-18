# Owner pending decisions — consolidated (the ONE surface to read on return)

**Status:** LIVE — consolidated 2026-10-18 ~15:4x +07 (shikuwa/4090 idle loop, zero cargo/GPU — the
plan-436 chain owns the box). Every prior queue row REFERENCE these items by name only; this file is
the first time the per-item lists are persisted. Freshness: each item carries its last-measured
date; re-measure before acting on stale rows. Deletion items follow the disk-clean contract:
LIST here, act only on explicit owner approval, per item.

## 1. Docker vhdx compact — 188 GB reclaimable (measured 2026-10-18)

- **Path:** `C:\Users\katop\AppData\Local\Docker\wsl\disk\docker_data.vhdx` — 187,927,887,872 B.
- **State:** DORMANT — the docker-desktop WSL distro is STOPPED, vmmemWSL ~37 MB. The vhdx never
  self-shrinks. It alone is ~21% of the C: drive (930G total / 904G used; C: free 27G at last read
  2026-10-18 15:36 +07, drifting down slowly — suspected pagefile expansion during the plan-436
  trainer's RAM spikes, unproven).
- **The owner question:** what's INSIDE (images/volumes) is a keep/kill decision. If kill:
  `docker system prune -a --volumes` (while stopped) → `wsl --shutdown` → compact via
  `Optimize-VHD -Path <vhdx> -Mode Full` (Hyper-V module) or `diskpart`'s
  `select vdisk … attach vdisk readonly … compact vdisk`. Expected reclaim: most of the 188 GB.
- **Risk:** none to the workspace repos; Docker is not used by any active lane (verified dormant).

## 2. C:/E: deletables — the "43GB class" (last full sweep 2026-10-14-era; re-measure pending quiet box)

Class-level entries, each zero-risk-reclassifiable on inspection. C: free 27G / E: free 65G at
2026-10-18 15:36 +07. **The per-target-dir size re-measure TIMED OUT twice this pass (2026-10-18,
loaded box under the plan-436 chain — msys du and PowerShell recursion both) — run the disk-clean
skill's scanner on a quiet box for fresh numbers before approving.** Known class members from the
prior sweeps:

- `E:\git/*/target/debug/incremental` — the standing zero-risk pick (~30G when dirty, workspace-wide).
- Cargo `target/` dirs of stale/sibling repos not built in weeks.
- `C:\t980`-class temp worktree/scratch dirs (verify empty-of-WIP first).
- Windows Update `SoftwareDistribution\Download` — measured 0.2 GB (2026-10-18, negligible).
- Root `pagefile.sys` 36.5G + `hiberfil.sys` 13.6G — STATIC system files, listed for completeness;
  shrink only with a conscious pagefile/hibernation decision (NOT recommended casually).

## 3. F:/models per-item list (~88.5 GB total, enumerated live 2026-10-18 15:4x +07)

| Item | Size | Last touched | Note / disposition ask |
|---|---|---|---|
| `qwen38-27b-dbirks-bf16.gguf` | 54.7 GB (54,657,734,368 B) | Aug 21 | The bf16 master copy of the league family — re-derivable? If quantized variants suffice, this is the single largest kill. |
| `Qwen3.8-27B-UD-Q4_K_XL.gguf` | 17.6 GB | Aug 22 | Q4_K_XL quant — keep/kill per league needs. |
| `qwen38-27b-dbirks-Q4_K_M.gguf` | 16.8 GB | Aug 21 | Q4_K_M quant — keep/kill per league needs. |
| `d3-v3.1.0/` | (dir) | Oct 14 | **CLARIFIED 2026-10-18: the known d3 teacher lane's weights** (owner-approved E:→F: offload, cycle-79/80; plan 629 T0 licence-clear + reflex --d3 lane). KEEP unless the d3 teacher lane is retired. |
| `dflash2-src/` | (dir) | Aug 22 | DFlash2 source — keep/kill. |
| `dbirks-qwen38-27b-w4a16/` | (dir) | Aug 21 | W4A16 variant — keep/kill. |
| `Qwen3.8-27B-DFlash2-Q2_K.gguf` | 0.7 GB | Aug 22 | Q2_K DFlash2 — small. |
| `mmproj-BF16.gguf` | 0.9 GB | Aug 22 | Multimodal projector — keep/kill. |
| `chat_template.jinja` | 9 KB | Aug 22 | Trivial. |

F: total 233G, 177G used, 57G free (2026-10-18) — no imminent pressure; this list is opportunistic.

## 4. M5-Ultra purchase cluster — 4 issues across 3 repos gated on ONE purchase

The dependency chain: the M5 Ultra hardware gates all four; no partial unblocking without it.

1. **riir-infer `.issues/015`** — FluidAudio/CoreML audio lane PoC (load FluidInference's published
   models from Rust). DEFERRED 2026-09-28 by owner directive: "audio until M5 Ultra". T0 boundary
   landed; T1–T6 deferred, recon recorded — pickup is turnkey.
2. **riir-auth `.issues/005`** — Voiceprint shard PoC: speaker embedding as a SOFT trust signal. OPEN.
3. **rethink `.issues/025`** — v1 hosted lane billing: the $5/month allowance plan (consumer side).
   Owner call 2026-10-09: M5-Ultra-gated ("not now, wait for m5 ultra") — allowance-pack lane stays
   lib-side until the M5 Ultra host exists. Economy design: riir-dapps Proposal 007 / Plan 044;
   tracked by rethink 024.
4. **rethink `.issues/016`** — the encoder class on GPU hosts: L3 think-depth seating via
   decision_wire. Owner call 2026-10-09: seating DEFERRED, M5-Ultra-gated ("the prod is M5 ultra
   metal so we do have gpu plan").

**The decision:** purchase (or defer) the M5 Ultra. On purchase, the four issues unblock in the
order above; on defer, they stay parked (no action needed — all are doc-parked, nothing rots).

## Cross-references

- The rolling agent-side queue state: `.distill/001_mining_queue_snapshot.md` (rows referencing
  "the three approval lists" all point HERE).
- vhdx row origin: snapshot SUBSTRATE-FIRST 10-18d row (2026-10-18 ~14:0x +07).
- F:/models d3-v3.1.0 clarification: snapshot L8-BENCH-PRE-STAGED row (2026-10-18 ~07:1x +07).
- M5-Ultra cluster surfacing: snapshot WORKSPACE-WIDE ISSUE SWEEP row (2026-10-18 ~15:3x +07).
