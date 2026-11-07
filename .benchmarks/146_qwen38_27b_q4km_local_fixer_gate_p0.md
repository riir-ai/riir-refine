# Bench 146 — Plan 335 P0: qwen3.8-27B **Q4_K_M** on llama.cpp (bmo) — the local fixer gate: **FAIL** (Arm B strict-keep 4/9 < 5/9)

**Status:** RECORDED 2026-11-08 — **P0.3 gate FAILS** (fixed pre-measurement bar: Arm B strict-keep ≥ 5/9 AND > 0/9, 0 divergent, 0 regressions). The local qwen lane does NOT serve (P1 stays shut); the lane waits for the GLM re-gate (Plan 335 P3). No threshold changed.
**Plan:** [335](../.plans/335_local_cluster_fixer_lane_qwen_local_gate_and_cli_runway.md) P0.1–P0.4 · **Protocol:** [Bench 049](049_fixer_regate_poc.md) (arms decline / A raw / B pre-pass, reachable-miss set, `fix_verify` strict-keep + clippy oracle) · linked from [Plan 333](../.plans/333_p005_fixer_leak_signoff_and_all_tier_prep.md) via Plan 335.

## Headline

| arm | instrument | keep_oracle / scoreable | **keep_strict** | vetoed | divergent | regressions (harness counter) |
|---|---|---|---|---|---|---|
| decline (DRY) | both | — | **0/9** (9/9 off-corpus, all declined) | — | — | — |
| A raw (qwen Q4_K_M) | as-shipped set (run 1) | 4/7 | **4/9** | 0 | 0 | 2 |
| B pre-pass (qwen Q4_K_M) | as-shipped set (run 1) | 3/7 | **3/9** | 0 | 0 | 2 |
| A raw (qwen Q4_K_M) | repaired controls (run 2) | 4/9 | **4/9** | 0 | 0 | 4 |
| **B pre-pass (qwen Q4_K_M)** | **repaired controls (run 2)** | 4/9 | **4/9** | 0 | 0 | 3 |
| *ref:* B pre-pass, CF `llama-3.3-70b-instruct-fp8-fast` (Bench 049, 2026-08-26) | 049-era set | 7/9 | *6/9* | 1 | 0 | — |

**Verdict: FAIL on both instruments.** Arm B = 4/9 (valid instrument) and 3/9 (as-shipped instrument, two dead controls) — both below the 5/9 bar. Both arms still beat decline (0/9), and never-wrong held in the sense that matters (0 divergent, no wrong fix KEPT); but the harness's own `regressions` counter (scoreable ∧ ¬strictly-fewer ∧ ¬noop — an attempted edit that did not reduce findings) is 3 on Arm B, so the literal "0 regressions" clause also fails. Per P0.3: recorded as a REGRESSION against the retired CF llama B-arm, no silent swap, no threshold fiddling.

## The confounds (read before drawing a model conclusion)

1. **Model AND quantization mixed.** The 6/9 reference is llama-3.3-70B at fp8 on Cloudflare; this run is qwen3.8-27B at **Q4_K_M** on llama.cpp. A gap cannot be blamed on qwen alone — a clean qwen-vs-llama comparison needs a matched-precision arm (qwen Q8_0/BF16, or llama at Q4_K_M on the same engine).
2. **The fixture set changed since Bench 049.** Two reachable fixtures were re-picked when their lints entered the corpus: `vec_init_then_push → clear_with_drain` (`56b3a756d`, 2026-09-13) and `ptr_arg → unused_self` (2026-09-18, Batch 155). llama's 6/9 included BOTH removed fixtures as keeps. **On the 7 fixtures common to both runs: llama B 4/7 (manual_strip, box_collection, option_if_let_else, large_enum_variant) vs qwen B 3/7** (the first three; `large_enum_variant` = noop echo under the pre-pass prompt, while qwen's raw arm A DID fix it). The CF arm could not be re-pinned on the current set in this run (CF tokens rolled — re-pin is a follow-up, see "What remains").
3. **Thinking switch differs by construction.** qwen ran with the server-level off-switch (`--reasoning off --reasoning-budget 0`) plus the riir-llm leaf's qwen3 `/no_think` user-turn suffix (applies to any model id containing `qwen3`); llama had none. With thinking off, completions were 11–74 tokens — the 512 budget never bound and **zero overflow retries fired**, so the budget is not a confound.
4. **Completion budget** `--max-new 512` (Bench 049 ran the default 128). Disclosed protocol deviation per Proposal 005 §window gate (a) ("budget sized for output"); measured harmless — max completion 74 tok.

## Instrument finding — two dead controls in the reachable set (repaired in this change)

Run 1 showed `f 0→0` on `clear_with_drain` and `unused_self`: the fixture never fires its own lint, so the case is unscoreable in EVERY arm and the "n=9" bar silently reads over n=7. Probed on bmo's clippy 0.1.98 (`48a229ceae 2026-09-01`):

- `pub fn f(v: &mut Vec<i32>) { v.drain(..); }` — **silent**; the owned shape `pub fn f(mut v: Vec<i32>) -> Vec<i32> { v.drain(..); v }` fires.
- `pub struct S; impl S { pub fn f(&self) {} }` — **silent** (a `pub` method is exempt under clippy's default `avoid-breaking-exported-api`); `struct S; impl S { fn f(&self) {} } pub fn g() { S.f() }` fires.

The t40 test file already KNEW ("they stay decline-only, never controls") but nothing carried that into the arm bar. Repair: both fixtures re-shaped in `src/cf_workers_ai.rs` (still off-corpus — `reachable_set_is_off_corpus_and_declined` green), and a new wall `every_reachable_fixture_fires_its_control` in `tests/t40_fixer_regate_harness.rs` (t40: 5 passed, 1 ignored, on bmo). Run 2 re-measured BOTH qwen arms on the repaired set in the same session (the CF arm could not be re-pinned — confound 2). The DRY arm's `ptr_arg` planted positives also silently skip ("not in sample set") since the 09-18 re-pick — the DRY verdict still reads ALL PASS over the remaining controls; noted, not repaired here.

**Reproducibility:** temperature 0; on the 7 unchanged fixtures run 1 and run 2 produced identical per-case outcomes in both arms (A 4 keeps, B 3 keeps).

## Per-case (run 2, repaired controls)

| fixture | A raw | B pre-pass | note |
|---|---|---|---|
| or_fun_call | f 1→0, contract ✗ | f 1→0, contract ✗ | lint resolved, item contract broken — vetoed both arms (llama: same, A) |
| large_enum_variant | **keep** | noop | the pre-pass prompt made qwen echo the input |
| too_many_arguments | f 1→1 | f 1→1 | signature reshape exceeds the corrected-lines shape (llama: same) |
| redundant_clone | noop | noop | (llama: A regressed, B noop) |
| clear_with_drain | f 1→0, not strictly fewer | **keep** (behav Equivalent{4}) | |
| manual_strip | **keep** (Equivalent{10}) | **keep** | |
| box_collection | **keep** (Unsupported, no Debug — admissible, disclosed) | **keep** | |
| option_if_let_else | **keep** (Equivalent{5}) | **keep** | |
| unused_self | f 1→0, contract ✗ | f 1→0, contract ✗ | the fix dropped/renamed an item the contract pins |

Run 1 (as-shipped set) differs only on the two dead-control rows (unscoreable; B `unused_self` also failed to parse).

## Setup (full attribution)

- **Weights:** `bartowski/Qwen3.8-27B-GGUF` → `Qwen3.8-27B-Q4_K_M.gguf` (17,442,399,968 B; bartowski's 2026-11-09-dated "new tensor-layout" re-upload, quantized with llama.cpp b10896, imatrix; MTP layers present at Q4_0, MTP speculation NOT enabled). sha256 `880a52af8317974f7b4b064a6bc29ff49e188b91aa218e08eea3078b6b1cc673` (= the HF LFS oid), **blake3 `ac9e4cfb10e9fba6f3b7eef4f6bc96e200669df1eb1f2846a199a34675439e3c`**. On bmo at `~/git/riir-train/data/hf_bartowski_qwen38/`. Base model `Qwen/Qwen3.8-27B` (`Qwen3_5ForConditionalGeneration`, hybrid linear/full attention). Choice: the plain Q4_K_M (not unsloth's `UD-Q4_K_M` dynamic mix) — the closest thing to a "standard Q4_K_M" record.
- **NOT the plan's original weight:** `riir-train/data/Qwen3.8-27B-DFlash2-Q4_K_M.gguf` (1.14 GB) is `general.architecture = dflash`, size_label 1.9B — a z-lab speculative-decoding DRAFT model, not the 27B target. Plan 335 P0.1 text corrected.
- **Engine:** llama.cpp **`llama-server` b12189** (commit `80d2c0c2c`, official `llama-b12189-bin-macos-arm64.tar.gz`, tarball sha256 `f30e2f3032df…bc20aee`), Metal. riir-infer was NOT used: it ships no OpenAI-compatible chat-serving surface for qwen3.8-27B today (grep 2026-11-08: `qwen38_*_g1` kernel tests + the Issue-1179 DFlash2 probe examples + `src/qwen38_dflash2.rs`, no `/chat/completions` server); Plan 627 is claimed elsewhere and untouched. Flags: `-c 16384 -np 1 -ngl 99 --reasoning off --reasoning-budget 0 --host 127.0.0.1 --port 8089 --no-webui`.
- **§window gates as serving params (Proposal 005):** (a) budget `--max-new 512`; (b) length-with-null-content → retry with a doubled budget, cap `--overflow-retries 2`, then HARD-DECLINE (unscoreable) — fired 0 times; (c) server-side thinking off + `/no_think`; (d) one-call smoke (`"Reply with the single word OK."` → `"OK"`, 2 tok) before every arm; (e) both response schemas parsed by the riir-llm leaf.
- **Harness:** `fixer_regate --backend local --base http://127.0.0.1:8089/v1 --model qwen3.8-27b-q4_k_m --source reachable --max-new 512 --overflow-retries 2` (`-p riir-refine-fixer --features cf_workers_ai,local_backend`, `--release`). `$` column forced to 0 for the local backend (no fabricated cost); no CF meters, no USD cap (local $0 marginal). Egress guard: a non-loopback `--base` is REFUSED unless `RIIR_REFINE_CODE_EGRESS=snippet` (new strict `is_loopback_base` predicate, unit-tested) — the run was loopback-only, harness and server on the same box. Rows: `.benchmarks/fixer_quality_history.jsonl` (4 rows, `model: local:qwen3.8-27b-q4_k_m`, ts 1794071…).
- **Box (bmo, Apple M5 Ultra 96 GB, macOS 27.2):** `PROVENANCE: power=AC Power load=2.05 swap=13864.88M canary=skipped powermode=0(auto)` (run 1, 2026-11-07T17:10Z) / `load=2.22 swap=13884.12M` (run 2, 17:16Z); memory free 31% → 48%; no concurrent heavy jobs (two idle `serve` processes and an idle mammoth python server resident, 0% CPU). Swap warn from `bench_preflight` disclosed. Every number here is a COUNT; latencies (≈0.5–1.9 s/case, 27-36 tok/s prefill on the smoke) are informational only.
- **Rig discipline (deviation, disclosed):** `agent-mail rig status` reports bmo OFFLINE (its direct tailscale address times out), so `agent-mail rig run bmo --exclusive --key p335-p0` could not reach it. The cells ran directly over the amo jump host (`ssh -J katopz@100.111.186.76 katopz@192.168.10.2`) in a detached worktree `~/git/wt-p335`; exclusivity was checked by hand (process table above), not by the queue.

## What this settles / what it does not

- **Settles:** at Q4_K_M on llama.cpp with thinking off, qwen3.8-27B does not meet the pre-registered local-lane bar on this smoke set (n=9) — B 4/9. The pipeline is cheap and clean locally (no overflow, ~1 s/case, $0), so the Bench-049 "unusable" verdict on CF was indeed a pricing/thinking-economics artifact; the remaining gap is QUALITY.
- **Does not settle:** whether qwen3.8-27B is worse than llama-70B at matched precision (confound 1), nor by how much on the CURRENT fixture set (confound 2; the matched-fixture read is 3/7 vs 4/7, one keep). n=9 is a smoke set, not a ranking.

## What remains (follow-ups, not part of the P0 verdict)

- Re-pin the CF llama B-arm on the current (repaired) set when CF tokens are refreshed — the gate's reference number was measured on a different fixture set.
- Optional matched-precision arm (qwen Q8_0 on the same engine) to separate quantization from model.
- The DRY arm's `ptr_arg` planted positives skip silently since the 09-18 re-pick — re-point them at a live fixture.
- P3: GLM-5.3-Flash re-gate (Plan 627 lineage) is now the lane's path.

## Reproduce

```sh
# on bmo (worktree under ~/git/wt-* so ../ path deps resolve)
llama-b12189/llama-server -m Qwen3.8-27B-Q4_K_M.gguf --host 127.0.0.1 --port 8089 \
  -c 16384 -np 1 -ngl 99 --reasoning off --reasoning-budget 0 --no-webui &
CARGO_TARGET_DIR=/tmp/p335-target cargo build --release -p riir-refine-fixer \
  --features cf_workers_ai,local_backend --bin fixer_regate
for arm in DRY A B; do
  /tmp/p335-target/release/fixer_regate --arm $arm --backend local \
    --base http://127.0.0.1:8089/v1 --model qwen3.8-27b-q4_k_m \
    --source reachable --max-new 512 --overflow-retries 2 --out arm_$arm.jsonl
done
```
