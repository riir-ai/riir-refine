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

## 2. C:/E: deletables — fresh per-target-dir numbers (quiet-box scan 2026-10-19 ~05:0xZ, 4090 box, CPU 6%, no heavy jobs — the re-measure this row was waiting for)

**Drives at scan:** C: free 41G / E: free 77G / F: free 57G (all ~76-96% used). The box runs 26
worktree target dirs; the quiet-box scan (PowerShell Measure-Object, ~8 min) reads:

**E:\git\*/target — 245 GB total across 26 dirs. Top rows (debug/release split):**

| Repo | target total | debug | release | Note |
|---|---|---|---|---|
| riir-ai | 53.8 GB | 27.4 | 26.3 | release = league binaries (Bench-982/983 ours-arm builds) — kill-debug only |
| riir-train | 34.8 GB | 8.7 | 26.0 | release = league + trainer binaries |
| riir-shader | 24.9 GB | 24.9 | 0 | ALL debug (bevy tree); safest full-target kill on E: |
| katgpt-rs | 18.7 GB | 12.1 | 6.5 | |
| riir-infer | 18.0 GB | 13.5 | 4.4 | release holds the L8/chunked A/B binaries (plan-621 prep, GPU-queued) — keep until the L8 A/B runs |
| riir-refine | 16.9 GB | — | — | **LOAD-BEARING**: target/release/cargo-refine is the global healer binary (re-linked 10-19) — never rm the whole target |
| riir-dapps | 11.2 GB | 11.2 | 0 | all debug |
| riir-chain | 8.5 GB | 8.5 | 0 | all debug |
| zed (E:\git\zed) | 8.2 GB | 0 | 8.2 | cache; source is the only patched-Zed copy — target deletable, ~1h cold rebuild |
| riir-mmorpg-examples | 6.4 GB | — | — | |
| remaining 16 dirs | ~15 GB | — | — | small per-dir |

**⛔ Incremental dirs are ~EMPTY on this box** (0.0-0.8 GB across all 26) — the M3's "incremental
sweep" zero-risk pick DOES NOT apply here; the unit of account is whole debug dirs.

**Class answers this row previously lacked:**
- `E:\git/*/target/debug/incremental` — measured ~2 GB workspace-wide (NOT the ~30G prior estimate). Stale row.
- The big debug dirs (shader 24.9, ai 27.4, dapps 11.2, chain 8.5, katgpt-rs 12.1, infer 13.5 = ~97 GB) are the real reclaim class; each kills cold-rebuild time for its repo (10-30 min).
- `C:\t980` — not re-checked this pass (C: side); pagefile/hiberfil unchanged-static.
- C: cargo/rustup: registry 3.3 + git 1.4 + toolchains 4.9 ≈ 9.9 GB (normal; registry re-downloads).

**Max-space zero-risk pick (this box):** riir-shader target (24.9 GB, all debug, bevy re-imports on
next build) + riir-dapps debug (11.2) + riir-chain debug (8.5) + zed target (8.2) ≈ **53 GB on E:**
for ~1h total cold-rebuild cost across four repos. NOTHING DELETED — owner approves per item.

## 3. F:/models per-item list — RESCAN 2026-10-19 ~05:1xZ: 88.5 GB → **155 GB** (+66 GB in one day)

| Item | Size (10-19 scan) | Last touched | Note / disposition ask |
|---|---|---|---|
| `qwen38-27b-dbirks-bf16.gguf` | 51 GB | Aug 21 | The bf16 master copy — unchanged; still the single largest kill if quants suffice. |
| `d3-v3.1.0/` | **49 GB** | Oct 14→18 | CLARIFIED 10-18: the d3 teacher lane's weights (owner-approved E:→F: offload). KEEP unless the lane retires. The +49 GB is this offload landing — the growth is EXPLAINED. |
| `dbirks-qwen38-27b-w4a16/` | **19 GB** | Aug 21 | W4A16 variant — was unmeasured 10-18; keep/kill per league needs. |
| `Qwen3.8-27B-UD-Q4_K_XL.gguf` | 17 GB | Aug 22 | Q4_K_XL quant — keep/kill per league needs. |
| `qwen38-27b-dbirks-Q4_K_M.gguf` | 16 GB | Aug 21 | Q4_K_M quant — keep/kill per league needs. |
| `dflash2-src/` | 3.6 GB | Aug 22 | DFlash2 source — keep/kill. |
| `mmproj-BF16.gguf` | 0.9 GB | Aug 22 | Multimodal projector — keep/kill. |
| `Qwen3.8-27B-DFlash2-Q2_K.gguf` | 0.7 GB | Aug 22 | Q2_K DFlash2 — small. |
| `chat_template.jinja` | 9 KB | Aug 22 | Trivial. |

F: total 233G, 177G used, 57G free (re-read 2026-10-19) — the 10-18 "no imminent pressure" read
STANDS but the headroom halved vs the 10-14-era state; the growth is the d3 offload (explained),
not drift. StartLux capture artifacts (GGUF Q4_K_M 15.4 GiB) live on E: under riir-train models/, not here.

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
