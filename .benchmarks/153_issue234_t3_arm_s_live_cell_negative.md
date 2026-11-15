# Bench 153 — Issue 234 T3: the arm S live cell (steered steer_retry, 27B CPU daemon, 4090)

**Verdict: NEGATIVE — the decline law fires on BOTH legs. T3 CLOSES; issue 234 closes with it.**

## Protocol (the runbook posture, executed verbatim)

- Worktree `riir-refine@491daae3b`, self-managed detached worktree `E:\git\wt-i234` on the 4090 (the runner's temp-dir `--worktree` cannot resolve sibling path deps — mirror-dir law; setup trail in the issue).
- `cargo run --release -p riir-refine-fixer --features cf_workers_ai,steer_retry --bin fixer_regate -- --arm S --source v2edit --limit 32 --data-dir E:\git\riir-train\data --l4-bin <bonsai_clippy_l4_eval.exe> --l4-gguf <Ternary-Bonsai-2-27B-PQ2_0.gguf> --l4-lora <models/plan341/v2_best.BCL4.bin> --max-new 512 --out E:\tmp\i234_arms\ledger.jsonl`, `CARGO_BUILD_JOBS=8` (the 24-way cold build hit rustc STATUS_ACCESS_VIOLATION twice — the Windows compiler-load family; capped build is stable).
- Adapter = `models/plan341/v2_best.BCL4.bin` (the operative v2-corpus SFT best, riir-train Bench 496 provenance, SHA256 `1a0c2cff…`, rank 8 alpha 16) — this resolves the runbook's `<ckpt.bcl4>` placeholder.
- NO `--join-h`: no arm-H ledger exists over this case set → the takeover axis had no evidence; the cost leg decided.
- Job `01a1dd08` (R3 of 4 queues; setup lessons: cmd.exe runner, temp-worktree sibling break, rustc AV), exit 0, total wall 16632.3 s.

## Results (32 paired cases, rounds budget 3)

| arm | n | EM | parses | merge | keep_s | vetoed | divergent | regressions |
|---|---|---|---|---|---|---|---|---|
| S-modelless | 32 | 0/32 | 32 | 2 | 0 | 0 | 0 | 0 |
| S-unguided | 32 | 0/32 | 0 | 0 | 0 | 0 | 0 | 0 |
| S-steered | 32 | 0/32 | 0 | 0 | 0 | 0 | 0 | 0 |

- Rounds: verified at r0/r1/r2/r3 **all 0**; **never 32**; declined 0; deadline 0 — every case exhausted the budget with no verified fix; the steer guidance never changed a single outcome (paired keep_s W/L/T 0/0/32 on both axes).
- Cost: generation total **16,502,659 ms (≈ 4.6 h) over 128 calls** (per-case gen walls 458–555 s); oracle (score_case) total 14,201 ms. The steered arm pays up to 4 generations per case for exactly the modelless arm's output → **cost-dominated** on the per-case gen wall. Takeover unmeasurable (no H ledger) and irrelevant at EM 0. Both decline-law legs point NEGATIVE.

## The mechanism confound (disclosed, NOT a re-run blocker)

`WARN: generation hit the 1024-position cache at token 103` fired **64 times** (every long-prompt call): the v2edit prompts run ~920 tokens, so the daemon's fixed 1024-position cache truncates generation at ~100 tokens of decode. The 0-parse result on the unguided/steered arms is therefore confounded: it is "the served completion was truncated" at minimum, "the 27B cannot produce a mergeable edit" at most — **this cell cannot separate the two.** Recorded so no later lane quotes "steering doesn't work" from this run; the honest finding is **"the Bonsai CPU daemon at the 1024-position cache ceiling cannot serve the fixer task envelope."** A widened-cache daemon run would be a NEW cell with its own pre-registration, not a re-tune of this one.

## Box state (disclosure duty)

4090 Windows, 24 cores, AC, GPU 0–3% / 0.5 GB/24 GB, 21–22/32 GB free, fleet badge QUIET throughout; the 27B CPU daemon is the pre-registered posture. Build `CARGO_BUILD_JOBS=8` after the AV retries. Ledger committed as evidence: `.benchmarks/234_arm_s_4090_ledger.jsonl` (12004 B, SHA256 `be6f98c736a38ae5…`), 32 rows, one per case, `miss_class: ServedParsing` uniformly, `student_declined: false`, `deadline_expired: false`.

## Disposition

- Issue 234: T3 CLOSED NEGATIVE per the pre-registered decline law; the escalation manifest gains **no** steer knob (PASS-only path); `steer_retry` stays opt-in, never in the release set, and its decline-is-a-correct-answer contract is unchanged. Lease row (line 4, zed-df880732@m3) folded into `**Lease closed:**` at landing.
- Session: zed-53534b91@m3, epoch 1794774000 (029-watchdog lane continuation of zed-df880732).
