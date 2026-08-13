#!/usr/bin/env python3
"""Bench 010 re-run — llama.cpp arm (Arm A).

Same protocol as the original Bench 010: the SAME 3 clippy prompts + 64-token
decode budget that the katgpt ternary arm (benches/bench_clippy_ternary_tps.rs)
gets, with `--temp 0.0` and the model's chat template applied via `--jinja`.

Reads `timings.prompt_ms` (prefill wall) + `timings.predicted_n` /
`timings.predicted_ms` (decode wall) from llama-server's /completion response —
the exact fields Bench 010 recorded. Reports per-case + aggregate tok/s so the
two arms are directly comparable.

Usage:
    # 1. start the server (separate terminal):
    llama-server --model Ternary-Bonsai-27B-Q2_0.gguf --port 8125 \\
      --ctx-size 4096 --temp 0.0 --seed 42 -ngl 99 --jinja --no-webui

    # 2. run this driver against it:
    python3 scripts/bench010_llama_arm.py --port 8125 --out /tmp/bench010_llama.json
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.request
from pathlib import Path

SYSTEM = (
    "You are a Rust clippy auto-fixer. Given a clippy lint name and the offending "
    "code, output ONLY the corrected Rust code in a single ```rust code block. "
    "No explanation, no commentary."
)


def build_prompt(lint: str, buggy: str) -> str:
    # Byte-identical to the Rust harness's chat string (chat template applied
    # by llama-server via --jinja, so we send the raw system+user messages and
    # let the server wrap them).
    return (
        f"clippy lint: `{lint}`\n\nOffending code:\n```rust\n{buggy}\n```\n\n"
        f"Corrected code:"
    )


def call_completion(port: int, lint: str, buggy: str, n_predict: int) -> dict:
    # /completion takes the raw prompt string + applies the chat template when
    # --jinja is set. We pass the system message as the prompt prefix so the
    # template sees the same system+user shape the Rust arm constructs.
    payload = {
        "prompt": build_prompt(lint, buggy),
        "system_prompt": SYSTEM,
        "temperature": 0.0,
        "n_predict": n_predict,
        "stream": False,
        "cache_prompt": False,
        "seed": 42,
    }
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/completion",
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=600) as resp:
        return json.loads(resp.read())


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8125)
    ap.add_argument(
        "--cases",
        type=Path,
        default=Path("benches/fixtures/e2e_cases_seed.json"),
    )
    ap.add_argument("--n-predict", type=int, default=64)
    ap.add_argument("--n-cases", type=int, default=3)
    ap.add_argument("--out", type=Path, default=Path("/tmp/bench010_llama.json"))
    args = ap.parse_args()

    case_file = json.loads(args.cases.read_text())
    cases = case_file["cases"][: args.n_cases]
    print(
        f"=== Bench 010 llama arm — fixture={case_file['name']} "
        f"n={len(cases)} n_predict={args.n_predict} ===",
        file=sys.stderr,
    )

    rows = []
    print(
        f"{'lint':<24}{'prompt':>8}{'decode':>8}{'prefill s':>11}"
        f"{'decode s':>11}{'dec t/s':>10}{'tot t/s':>10}",
        file=sys.stderr,
    )
    print("-" * 82, file=sys.stderr)
    for c in cases:
        lint = c["lint"]
        buggy = c["buggy"]
        out = call_completion(args.port, lint, buggy, args.n_predict)
        t = out.get("timings", {})
        prompt_n = t.get("prompt_n", out.get("tokens_evaluated", 0))
        predicted_n = t.get("predicted_n", out.get("tokens_predicted", 0))
        prompt_ms = t.get("prompt_ms", 0.0)
        predicted_ms = t.get("predicted_ms", 0.0)
        prefill_s = prompt_ms / 1000.0
        decode_s = predicted_ms / 1000.0
        dec_tps = predicted_n / decode_s if decode_s > 0 else 0.0
        tot_tps = (prompt_n + predicted_n) / (prefill_s + decode_s) if (prefill_s + decode_s) > 0 else 0.0
        rows.append(
            {
                "lint": lint,
                "prompt_tokens": prompt_n,
                "decode_tokens": predicted_n,
                "prefill_s": prefill_s,
                "decode_s": decode_s,
                "dec_tps": dec_tps,
                "tot_tps": tot_tps,
            }
        )
        print(
            f"{lint:<24}{prompt_n:>8}{predicted_n:>8}{prefill_s:>11.3f}"
            f"{decode_s:>11.3f}{dec_tps:>10.3f}{tot_tps:>10.3f}",
            file=sys.stderr,
        )

    print("-" * 82, file=sys.stderr)
    pt = sum(r["prompt_tokens"] for r in rows)
    dt = sum(r["decode_tokens"] for r in rows)
    ps = sum(r["prefill_s"] for r in rows)
    ds = sum(r["decode_s"] for r in rows)
    print(
        f"{'AGGREGATE':<24}{pt:>8}{dt:>8}{ps:>11.3f}{ds:>11.3f}"
        f"{(dt/ds if ds else 0.0):>10.3f}{((pt+dt)/(ps+ds) if (ps+ds) else 0.0):>10.3f}",
        file=sys.stderr,
    )

    args.out.write_text(json.dumps({"rows": rows, "aggregate": {
        "prompt_tokens": pt, "decode_tokens": dt,
        "prefill_s": ps, "decode_s": ds,
        "prefill_tps": pt/ps if ps else 0.0,
        "decode_tps": dt/ds if ds else 0.0,
        "total_tps": (pt+dt)/(ps+ds) if (ps+ds) else 0.0,
    }}, indent=2))
    print(f"\nwrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
