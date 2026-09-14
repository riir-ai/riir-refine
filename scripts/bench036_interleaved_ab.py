#!/usr/bin/env python3
"""Bench 036 — interleaved per-pair A/B: llama.cpp vs katgpt ternary on clippy prompts.

WHY THIS EXISTS (what it fixes vs Bench 010/014)
------------------------------------------------
Bench 010 and 014 measured each arm in a separate block ("both arms timed with
the other's process stopped"), then divided aggregate-A by aggregate-B. That is
a between-block comparison: any drift in machine state between the two blocks
(thermal, page cache, sibling load) lands entirely in the ratio. The repo has
been burned by exactly this twice — riir-ai Bench 645's 2.48x was retracted for
contention and its "corrected" 1.42x was ALSO wrong (sequential-arm inflation).

This driver instead runs INTERLEAVED PAIRS and reports PER-PAIR RATIOS:

    pair k  ->  ratio_k = armA_tps_k / armB_tps_k

with the within-pair ORDER ALTERNATING (A->B, B->A, A->B, ...) so that any
monotonic drift cancels rather than accumulating into one arm. The headline is
the MEDIAN of the per-pair ratios, and the spread (min..max) is reported so a
wide spread is visible instead of hidden inside an average.

GPU EXCLUSIVITY
---------------
Only ONE arm holds the GPU at a time. llama-server is started ONCE (a ~4 s
load) and then SIGSTOP'd; it is SIGCONT'd only for its own visit and SIGSTOP'd
again immediately after. A stopped process submits no GPU work, so Arm B never
competes with it. (It does retain its ~7 GB residency, which is fine on an
M3 Max: Bench 012 recorded a 53 GB free working set.)

Arm B gets a fresh process per visit because the harness loads the GGUF at
startup (~137 s) and exits after its case loop. That load is EXCLUDED from the
timed rates by the harness itself, exactly as in Bench 010/014.

The driver also re-checks for sibling GPU compute consumers before every visit
and ABORTS (rather than silently producing a contended number) if one appears.

MEASURED CONFIG
---------------
Arm B runs the production DEFAULT-ON path: batched simdgroup cmma prefill
(`set_prefill_use_simdgroup(true)`, hardcoded in the harness), zerocopy OFF
(ZEROCOPY unset -- it is a measured 47x decode REGRESSION at 27B scale, see
commit 3f354f4), speculative OFF (SPECULATIVE unset -- measured 3.3-4.4x
SLOWER, Bench 012 / Issue 665 T1).

Usage:
    python3 scripts/bench036_interleaved_ab.py --repeats 3 --out /tmp/bench036.json
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GGUF = Path("/Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf")
LLAMA_SERVER = Path(
    "/Users/katopz/git/vendor/prismml-llama.cpp/build/bin/llama-server"
)
CASES = REPO / "benches" / "fixtures" / "e2e_cases_seed.json"

# Sibling GPU-compute process names that invalidate a measurement.
#
# Matched against the EXECUTABLE BASENAME (argv[0]), never the full command line.
# Substring-matching the whole command line produces false positives that are
# easy to mistake for real contention -- both of these were hit on the first run:
#   * a zsh poller whose wait condition literally contains
#     "bonsai_lora_accuracy_parity_arm_c" (it is an ssh loop, not a GPU job)
#   * `rustc --crate-name riir_games` whose `--extern` / `-L` flags reference
#     riir-gpu (it is a compiler, not a GPU job)
#
# Deliberately excludes rustc/cargo (CPU-only: Bench 010 measured Arm B decode
# as insensitive to CPU load -- 18.24 tok/s at load 12.6 vs 17.94 at load 3.6)
# and Unity/Zed (the documented display-only exemption).
GPU_CONSUMER_MARKERS = (
    "bonsai_lora",
    "arm_c",
    "bench_clippy_ternary_tps",
    "llama-bench",
    "llama-server",
    "test-dspark",
)


def gpu_consumers(exclude_pids: set[int]) -> list[str]:
    """Return command lines of sibling GPU compute consumers, if any."""
    out = subprocess.run(
        ["ps", "-Ao", "pid,command"], capture_output=True, text=True, encoding="utf-8", errors="replace", check=True
    ).stdout.splitlines()
    hits = []
    for line in out[1:]:
        line = line.strip()
        if not line:
            continue
        pid_str, _, cmd = line.partition(" ")
        try:
            pid = int(pid_str)
        except ValueError:
            continue
        if pid in exclude_pids or pid == os.getpid():
            continue
        # `--cpu-eval` forces the arm_c probe onto a CPU-only forward (its log
        # prints "CPU-only forward"; no `CubeCL runtime initialized` line, unlike
        # the GPU runs). It contends for CPU, not the GPU, so it does not
        # invalidate a GPU measurement -- and Bench 010 measured Arm B decode as
        # insensitive to CPU load (18.24 tok/s at load 12.6 vs 17.94 at 3.6).
        # The interleaved per-pair design is what absorbs residual CPU noise.
        if "--cpu-eval" in cmd:
            continue
        # Match the executable basename only -- see GPU_CONSUMER_MARKERS.
        exe = os.path.basename(cmd.split(" ", 1)[0])
        for marker in GPU_CONSUMER_MARKERS:
            if marker in exe:
                hits.append(f"pid={pid} {cmd[:110]}")
                break
    return hits


def require_exclusive(stage: str, exclude_pids: set[int]) -> None:
    hits = gpu_consumers(exclude_pids)
    if hits:
        print(
            f"\n!! ABORT at {stage}: sibling GPU compute consumer(s) present.\n"
            "   A gate measured under contention is not evidence "
            "(riir-ai AGENTS.md; Bench 645 retraction).\n"
            + "\n".join(f"   {h}" for h in hits),
            file=sys.stderr,
        )
        sys.exit(2)


def loadavg() -> float:
    return os.getloadavg()[0]


# Two-tier load policy, because "ratio valid" and "absolutes comparable to a
# prior bench" are different questions with different thresholds.
#
# MAX_LOADAVG -- hard abort. Above this the box is saturated enough that the two
# arms cannot be assumed to degrade together: both are partly CPU-dispatch-bound
# (Bench 669: ~832 dispatches/token, 37.7% unaccounted dispatch overhead), and
# asymmetric degradation is exactly what a ratio cannot repair. The first attempt
# in this session saw loadavg 56 -- that is the catastrophe this guards against.
# 16.0 is the ambient floor on this box while a sibling runs a CPU-only 27B
# forward ("~10-30 min/step", no GPU), which cannot be killed (it is another
# agent's work) and would otherwise block the bench indefinitely.
#
# LOAD_COMPARABLE_CEILING -- soft flag, NOT an abort. Bench 010 established Arm B
# decode insensitivity only up to 12.6 (18.24 tok/s at load 12.6 vs 17.94 at
# 3.6). Above it, the per-pair RATIO is still the designed metric (interleaving +
# alternating order + tight spread is what earns it), but the ABSOLUTE tok/s are
# load-depressed and must not be compared against Bench 010/014's absolutes.
# The output records this verdict explicitly so the writeup cannot forget it.
MAX_LOADAVG = 16.0
LOAD_COMPARABLE_CEILING = 12.6


def require_sane_load(stage: str, ceiling: float) -> None:
    la = loadavg()
    if la > ceiling:
        print(
            f"\n!! ABORT at {stage}: loadavg {la:.1f} > {ceiling} "
            "(outside Bench 010's validated insensitivity band of ~12.6).\n"
            "   Both arms are partly CPU-dispatch-bound; a saturated box does "
            "not perturb them equally, so\n   neither the absolute tok/s nor "
            "the per-pair ratio would be evidence. Wait for a quieter box.",
            file=sys.stderr,
        )
        sys.exit(3)


# ── Arm A: llama.cpp (resident server, SIGSTOP'd between its own visits) ──


def start_llama(port: int) -> subprocess.Popen:
    print(f"[armA] starting llama-server on :{port} ...", file=sys.stderr)
    proc = subprocess.Popen(
        [
            str(LLAMA_SERVER),
            "--model", str(GGUF),
            "--port", str(port),
            "--ctx-size", "4096",
            "--temp", "0.0",
            "--seed", "42",
            "-ngl", "99",
            "--jinja",
            "--no-webui",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    deadline = time.time() + 600
    while time.time() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(f"llama-server died early (rc={proc.returncode})")
        try:
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/health", timeout=5
            ) as r:
                if json.loads(r.read()).get("status") == "ok":
                    print("[armA] health ok", file=sys.stderr)
                    return proc
        except (urllib.error.URLError, OSError, ValueError):
            time.sleep(2)
    raise RuntimeError("llama-server never became healthy")


def run_arm_a(port: int, n_predict: int, n_cases: int, tag: str) -> dict:
    out = Path(f"/tmp/bench036_armA_{tag}.json")
    subprocess.run(
        [
            sys.executable, str(REPO / "scripts" / "bench010_llama_arm.py"),
            "--port", str(port),
            "--cases", str(CASES),
            "--n-predict", str(n_predict),
            "--n-cases", str(n_cases),
            "--out", str(out),
        ],
        check=True,
        cwd=REPO,
        stdout=subprocess.DEVNULL,
        # The child is one of our own scripts and prints non-ASCII; without
        # this its stdout encoder is the system locale and the write raises
        # before a byte reaches us (katgpt-rs Issue 778).
        env={**os.environ, 'PYTHONIOENCODING': 'utf-8'},
    )
    return json.loads(out.read_text())["aggregate"]


# ── Arm B: katgpt ternary (fresh process per visit) ──


def run_arm_b(n_predict: int, n_cases: int, tag: str, target_dir: str) -> dict:
    jsonl = Path(f"/tmp/bench036_armB_{tag}.jsonl")
    env = {
        **os.environ,
        "BONSAI_GGUF": str(GGUF),
        "CLIPPY_CASES": str(CASES),
        "DECODE_TOKENS": str(n_predict),
        "N_CASES": str(n_cases),
        "OUT_JSONL": str(jsonl),
        "CARGO_TARGET_DIR": target_dir,
    }
    # Measure the production default: no ZEROCOPY (47x decode regression at 27B),
    # no SPECULATIVE (3.3-4.4x slower, Bench 012).
    env.pop("ZEROCOPY", None)
    env.pop("SPECULATIVE", None)
    subprocess.run(
        [
            "cargo", "bench", "--bench", "bench_clippy_ternary_tps",
            "--features", "ternary_inference",
        ],
        check=True,
        cwd=REPO,
        env=env,
        stdout=subprocess.DEVNULL,
    )
    rows = [json.loads(l) for l in jsonl.read_text().splitlines() if l.strip()]
    pt = sum(r["prompt_tokens"] for r in rows)
    dt = sum(r["decode_tokens"] for r in rows)
    ps = sum(r["prefill_s"] for r in rows)
    ds = sum(r["decode_s"] for r in rows)
    return {
        "prompt_tokens": pt, "decode_tokens": dt,
        "prefill_s": ps, "decode_s": ds,
        "prefill_tps": pt / ps if ps else 0.0,
        "decode_tps": dt / ds if ds else 0.0,
        "total_tps": (pt + dt) / (ps + ds) if (ps + ds) else 0.0,
        "rows": rows,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repeats", type=int, default=3, help="measured pairs")
    ap.add_argument("--warmup-pairs", type=int, default=1, help="discarded pairs")
    ap.add_argument("--port", type=int, default=8125)
    ap.add_argument("--n-predict", type=int, default=64)
    ap.add_argument("--n-cases", type=int, default=3)
    ap.add_argument("--target-dir", default="/tmp/b036_clippy_tps")
    ap.add_argument("--out", type=Path, default=Path("/tmp/bench036.json"))
    ap.add_argument(
        "--max-load",
        type=float,
        default=MAX_LOADAVG,
        metavar="LOADAVG",
        help=(
            f"Hard-abort ceiling (default {MAX_LOADAVG}). Pass "
            f"{LOAD_COMPARABLE_CEILING} for a QUIET-BOX run whose absolute tok/s "
            "are comparable to Bench 010/014 — that converts the per-pair ratios "
            "from an upper bound into a point estimate."
        ),
    )
    ap.add_argument(
        "--pair-load-timeout",
        type=int,
        default=1800,
        metavar="SECONDS",
        help=(
            "How long to wait for loadavg to fall back under --max-load BEFORE "
            "each pair. Prevents a mid-run load spike from contaminating the "
            "median. Default 1800."
        ),
    )
    ap.add_argument(
        "--wait-for-window",
        type=int,
        default=0,
        metavar="SECONDS",
        help=(
            "Poll until the box is clean (no GPU consumers AND loadavg <= "
            f"{MAX_LOADAVG}) before measuring, giving up after SECONDS. "
            "0 = check once and abort if dirty. This box routinely runs "
            "sibling agents, so a clean window has to be waited for rather "
            "than assumed."
        ),
    )
    args = ap.parse_args()

    for p in (GGUF, LLAMA_SERVER, CASES):
        if not p.exists():
            print(f"missing required path: {p}", file=sys.stderr)
            return 1

    if args.wait_for_window > 0:
        deadline = time.time() + args.wait_for_window
        waited = 0
        while True:
            hits = gpu_consumers(set())
            la = loadavg()
            if not hits and la <= args.max_load:
                print(
                    f"[window] OPEN after {waited}s (loadavg {la:.1f}, "
                    "no GPU consumers)",
                    file=sys.stderr,
                )
                break
            if time.time() >= deadline:
                print(
                    f"\n!! GAVE UP waiting {args.wait_for_window}s for a clean "
                    f"window (loadavg {la:.1f}, {len(hits)} GPU consumer(s)).\n"
                    "   Nothing was measured -- no number is better than a "
                    "contended one.",
                    file=sys.stderr,
                )
                return 3
            if waited % 300 == 0:
                print(
                    f"[window] waiting... loadavg {la:.1f} "
                    f"(need <= {args.max_load}), "
                    f"{len(hits)} GPU consumer(s), {waited}s elapsed",
                    file=sys.stderr,
                )
            time.sleep(30)
            waited += 30

    require_exclusive("startup", exclude_pids=set())
    require_sane_load("startup", args.max_load)
    load_start = loadavg()

    llama = start_llama(args.port)
    os.kill(llama.pid, signal.SIGSTOP)
    print("[armA] SIGSTOP (parked; releases GPU)", file=sys.stderr)

    total_pairs = args.warmup_pairs + args.repeats
    pairs: list[dict] = []
    try:
        for k in range(total_pairs):
            warm = k < args.warmup_pairs
            label = f"warmup{k}" if warm else f"pair{k - args.warmup_pairs}"
            a_first = (k % 2) == 0
            order = "A->B" if a_first else "B->A"
            print(
                f"\n=== {label} ({order}){' [DISCARDED]' if warm else ''} ===",
                file=sys.stderr,
            )
            # Re-gate the load BEFORE each pair, not just at startup. Learned the
            # hard way: the first quiet-box run opened at loadavg 11.47 and a
            # sibling's `cargo build` (bevy, 587% CPU) then drove it to 12.06 ->
            # 24.61 -> 35.29 across the three pairs. Startup-only gating produced
            # one clean pair and two contaminated ones, and since the median of 3
            # is the MIDDLE pair, the headline silently became a load-24.61
            # measurement. Waiting here keeps every pair inside the ceiling
            # instead of quietly reporting a contaminated median.
            waited = 0
            while loadavg() > args.max_load:
                if waited == 0:
                    print(
                        f"  [pause] loadavg {loadavg():.1f} > {args.max_load} — "
                        "waiting for the window to reopen before measuring this "
                        "pair (startup-only gating is what contaminated the "
                        "first quiet-box run)",
                        file=sys.stderr,
                    )
                if waited >= args.pair_load_timeout:
                    print(
                        f"\n!! ABORT at {label}: loadavg stayed above "
                        f"{args.max_load} for {waited}s. "
                        f"{len(pairs)} clean pair(s) measured so far are in the "
                        "output; the run stops rather than appending "
                        "contaminated pairs.",
                        file=sys.stderr,
                    )
                    raise SystemExit(4 if not pairs else 0)
                time.sleep(30)
                waited += 30

            def visit_a() -> dict:
                require_exclusive(f"{label}/armA", {llama.pid})
                os.kill(llama.pid, signal.SIGCONT)
                try:
                    r = run_arm_a(args.port, args.n_predict, args.n_cases, label)
                finally:
                    os.kill(llama.pid, signal.SIGSTOP)
                print(
                    f"  armA prefill={r['prefill_tps']:.2f} "
                    f"decode={r['decode_tps']:.2f} total={r['total_tps']:.2f}",
                    file=sys.stderr,
                )
                return r

            def visit_b() -> dict:
                require_exclusive(f"{label}/armB", {llama.pid})
                r = run_arm_b(
                    args.n_predict, args.n_cases, label, args.target_dir
                )
                print(
                    f"  armB prefill={r['prefill_tps']:.2f} "
                    f"decode={r['decode_tps']:.2f} total={r['total_tps']:.2f}",
                    file=sys.stderr,
                )
                return r

            if a_first:
                a = visit_a()
                b = visit_b()
            else:
                b = visit_b()
                a = visit_a()

            if warm:
                continue

            # Token-count parity is a hard gate, not an assumption.
            if a["prompt_tokens"] != b["prompt_tokens"]:
                print(
                    f"!! ABORT: tokenizer parity broken "
                    f"(armA={a['prompt_tokens']} armB={b['prompt_tokens']})",
                    file=sys.stderr,
                )
                return 2

            pairs.append({
                "label": label, "order": order, "armA": a, "armB": b,
                "ratio_prefill": a["prefill_tps"] / b["prefill_tps"],
                "ratio_decode": a["decode_tps"] / b["decode_tps"],
                "ratio_total": a["total_tps"] / b["total_tps"],
                "loadavg": loadavg(),
            })
    finally:
        try:
            os.kill(llama.pid, signal.SIGCONT)
        except ProcessLookupError:
            pass
        llama.terminate()
        try:
            llama.wait(timeout=30)
        except subprocess.TimeoutExpired:
            llama.kill()
        print("\n[armA] llama-server stopped", file=sys.stderr)

    if not pairs:
        print("no measured pairs", file=sys.stderr)
        return 1

    def summarize(key: str) -> dict:
        vals = [p[key] for p in pairs]
        return {
            "median": statistics.median(vals),
            "min": min(vals),
            "max": max(vals),
            "values": vals,
        }

    summary = {
        "n_pairs": len(pairs),
        "warmup_pairs": args.warmup_pairs,
        "prefill": summarize("ratio_prefill"),
        "decode": summarize("ratio_decode"),
        "total": summarize("ratio_total"),
        "armA_median_tps": {
            m: statistics.median([p["armA"][f"{m}_tps"] for p in pairs])
            for m in ("prefill", "decode", "total")
        },
        "armB_median_tps": {
            m: statistics.median([p["armB"][f"{m}_tps"] for p in pairs])
            for m in ("prefill", "decode", "total")
        },
        "loadavg_start": load_start,
        "loadavg_end": loadavg(),
        "loadavg_per_pair": [p["loadavg"] for p in pairs],
        "load_comparable_ceiling": LOAD_COMPARABLE_CEILING,
        "max_load_ceiling_used": args.max_load,
        # If any pair ran above the ceiling, the per-pair ratios still stand
        # (that is what interleaving buys) but the absolute tok/s are depressed
        # and must NOT be compared to Bench 010/014 absolutes.
        "absolutes_comparable_to_bench_014": all(
            p["loadavg"] <= LOAD_COMPARABLE_CEILING for p in pairs
        ),
    }

    print("\n" + "=" * 68, file=sys.stderr)
    print("PER-PAIR RATIOS (llama.cpp / katgpt) — median is the headline",
          file=sys.stderr)
    print("=" * 68, file=sys.stderr)
    for p in pairs:
        print(
            f"  {p['label']:<8} {p['order']:<6} "
            f"prefill={p['ratio_prefill']:.3f}x "
            f"decode={p['ratio_decode']:.3f}x "
            f"total={p['ratio_total']:.3f}x",
            file=sys.stderr,
        )
    for m in ("prefill", "decode", "total"):
        s = summary[m]
        print(
            f"  {m:<8} median={s['median']:.3f}x "
            f"spread={s['min']:.3f}..{s['max']:.3f}",
            file=sys.stderr,
        )
    print(
        f"\n  armB median tok/s: "
        f"prefill={summary['armB_median_tps']['prefill']:.2f} "
        f"decode={summary['armB_median_tps']['decode']:.2f} "
        f"total={summary['armB_median_tps']['total']:.2f}",
        file=sys.stderr,
    )

    args.out.write_text(json.dumps({"summary": summary, "pairs": pairs}, indent=2))
    print(f"\nwrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
