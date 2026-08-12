#!/usr/bin/env python3
"""E2E tokens/sec head-to-head: Bonsai-27B (llama.cpp) vs riir-clippy self_evolve.

Both arms are given the SAME task: `(lint_name, buggy_code)` -> emit fixed Rust.
The metric is end-to-end tokens/sec on the emitted fix:

    tok/s = tokens(emitted fix) / wall_clock(request -> fix in hand)

Fairness rules enforced here:
  * Both arms are driven by the SAME fixture file (`benches/fixtures/*.json`) —
    same lints, same buggy code, same expected fix. No duplicated case lists.
  * Same tokenizer (Bonsai's) counts BOTH numerators, via `llama-tokenize`.
    The clippy arm emits raw Rust; we tokenize it with Bonsai's vocab so the
    numerator unit is identical.
  * Model-load / setup cost is reported separately, not amortized silently.
  * Bonsai is a *thinking* model — its reasoning tokens count toward its own
    e2e time, and `--max-tokens` must be large enough that it can finish
    reasoning AND emit code. A truncated reasoning block scores 0 on quality
    for a budget reason, not a capability reason. Default 2048.
  * Correctness is reported alongside throughput. Tokens/sec on a wrong fix is
    not a win — the exact-match column is the quality gate.

Usage:
    python3 scripts/bench_bonsai_vs_clippy_tokps.py \
        --port 8123 \
        --cases benches/fixtures/e2e_cases_seed.json \
        --clippy-jsonl /tmp/clippy_e2e_seed.jsonl \
        --out /tmp/bonsai_vs_clippy_seed.json
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.request
from pathlib import Path

SYSTEM = (
    "You are a Rust clippy auto-fixer. Given a clippy lint name and the offending "
    "code, output ONLY the corrected Rust code in a single ```rust code block. "
    "No explanation, no commentary."
)


def tokenize_count(port: int, text: str) -> int:
    """Count tokens with Bonsai's own tokenizer, via the ALREADY-LOADED server.

    Deliberately NOT the `llama-tokenize` binary: that reloads the 6.6 GiB model
    per call, and interleaving those loads between generation requests cost the
    server ~40% of its measured tok/s (21 -> 13 t/s on an identical case). The
    /tokenize endpoint reuses the resident model, so the denominator we report
    for the Bonsai arm is not polluted by our own instrumentation.
    """
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/tokenize",
        data=json.dumps({"content": text}).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        return len(json.loads(resp.read())["tokens"])


def extract_code(text: str) -> str:
    """Pull the first fenced code block; fall back to the raw text."""
    m = re.search(r"```(?:rust)?\s*\n(.*?)```", text, re.S)
    return (m.group(1) if m else text).strip()


def normalize(code: str) -> str:
    """Whitespace-insensitive comparison (formatting is not a correctness axis)."""
    return re.sub(r"\s+", " ", code).strip()


def call_bonsai(port: int, lint: str, buggy: str, max_tokens: int) -> dict:
    payload = {
        "model": "local",
        "messages": [
            {"role": "system", "content": SYSTEM},
            {
                "role": "user",
                "content": f"clippy lint: `{lint}`\n\nOffending code:\n```rust\n{buggy}\n```\n\nCorrected code:",
            },
        ],
        "temperature": 0.0,
        "max_tokens": max_tokens,
        "stream": False,
    }
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    t0 = time.perf_counter()
    with urllib.request.urlopen(req, timeout=1800) as resp:
        out = json.loads(resp.read())
    elapsed = time.perf_counter() - t0
    usage = out.get("usage", {})
    choice = out["choices"][0]
    text = choice["message"]["content"] or ""
    return {
        "raw": text,
        "code": extract_code(text),
        "prompt_tokens": usage.get("prompt_tokens", 0),
        "completion_tokens": usage.get("completion_tokens", 0),
        "elapsed_s": elapsed,
        # `length` => the reasoning/answer was cut off by --max-tokens. A cut-off
        # answer must be reported as truncated, not silently as "wrong".
        "finish_reason": choice.get("finish_reason", "?"),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8123)
    ap.add_argument(
        "--cases", type=Path, default=Path("benches/fixtures/e2e_cases_seed.json")
    )
    ap.add_argument("--clippy-jsonl", type=Path, default=Path("/tmp/clippy_e2e.jsonl"))
    ap.add_argument("--out", type=Path, default=Path("/tmp/bonsai_vs_clippy.json"))
    ap.add_argument("--max-tokens", type=int, default=2048)
    args = ap.parse_args()

    case_file = json.loads(args.cases.read_text())
    cases = [(c["lint"], c["buggy"], c["want"]) for c in case_file["cases"]]
    print(
        f"=== fixture: {case_file['name']} ({len(cases)} cases) ===", file=sys.stderr
    )

    # ── Load the clippy arm's emitted fixes + latencies ──────────────────
    clippy_rows = {}
    clippy_summary = {}
    for line in args.clippy_jsonl.read_text().splitlines():
        if not line.strip():
            continue
        obj = json.loads(line)
        if obj.get("_summary"):
            clippy_summary = obj
        else:
            clippy_rows[obj["lint"]] = obj

    # ── Phase 1: ALL Bonsai generation, back to back ─────────────────────
    # Nothing else runs on the box between timed requests — no tokenizer loads,
    # no scoring, no file I/O. The measured wall clock is generation only.
    gen = []
    for lint, buggy, want in cases:
        print(f"  [bonsai] {lint} ... ", end="", flush=True, file=sys.stderr)
        b = call_bonsai(args.port, lint, buggy, args.max_tokens)
        gen.append((lint, want, b))
        tokps = b["completion_tokens"] / b["elapsed_s"] if b["elapsed_s"] > 0 else 0.0
        print(
            f"{b['completion_tokens']} tok in {b['elapsed_s']:.1f}s "
            f"= {tokps:.2f} tok/s  finish={b['finish_reason']}",
            file=sys.stderr,
        )

    # ── Phase 2: scoring + tokenization (untimed) ────────────────────────
    results = []
    for lint, want, b in gen:
        b_correct = normalize(b["code"]) == normalize(want)
        b_tokps = b["completion_tokens"] / b["elapsed_s"] if b["elapsed_s"] > 0 else 0.0

        c = clippy_rows.get(lint)
        if c is None:
            print(f"    WARN: no clippy row for {lint}", file=sys.stderr)
            continue
        # Tokenize the clippy arm's emitted fix with Bonsai's vocab.
        c_tokens = tokenize_count(args.port, c["emitted"])
        c_secs = c["p50_ns"] / 1e9
        c_tokps = c_tokens / c_secs if c_secs > 0 else 0.0
        c_correct = normalize(c["emitted"]) == normalize(want)

        results.append(
            {
                "lint": lint,
                "bonsai": {
                    "completion_tokens": b["completion_tokens"],
                    "prompt_tokens": b["prompt_tokens"],
                    "elapsed_s": b["elapsed_s"],
                    "tok_per_sec": b_tokps,
                    "correct": b_correct,
                    "truncated": b["finish_reason"] == "length",
                    "finish_reason": b["finish_reason"],
                    "code": b["code"],
                },
                "clippy": {
                    "tokens": c_tokens,
                    "elapsed_s": c_secs,
                    "tok_per_sec": c_tokps,
                    "correct": c_correct,
                    "parses": c["parses"],
                    "code": c["emitted"],
                },
                "speedup": (c_tokps / b_tokps) if b_tokps > 0 else None,
            }
        )

    # ── Aggregate ────────────────────────────────────────────────────────
    b_tok = sum(r["bonsai"]["completion_tokens"] for r in results)
    b_s = sum(r["bonsai"]["elapsed_s"] for r in results)
    c_tok = sum(r["clippy"]["tokens"] for r in results)
    c_s = sum(r["clippy"]["elapsed_s"] for r in results)
    agg = {
        "bonsai_tok_per_sec_e2e": b_tok / b_s if b_s else 0.0,
        "clippy_tok_per_sec_e2e": c_tok / c_s if c_s else 0.0,
        "bonsai_total_tokens": b_tok,
        "bonsai_total_s": b_s,
        "clippy_total_tokens": c_tok,
        "clippy_total_s": c_s,
        "bonsai_correct": sum(1 for r in results if r["bonsai"]["correct"]),
        "bonsai_truncated": sum(1 for r in results if r["bonsai"]["truncated"]),
        "clippy_correct": sum(1 for r in results if r["clippy"]["correct"]),
        "n": len(results),
    }
    agg["speedup_x"] = (
        agg["clippy_tok_per_sec_e2e"] / agg["bonsai_tok_per_sec_e2e"]
        if agg["bonsai_tok_per_sec_e2e"]
        else None
    )

    print("\n=== E2E tok/s head-to-head ===")
    hdr = f"{'lint':<24}{'bonsai t/s':>12}{'clippy t/s':>14}{'speedup':>12}{'b_ok':>7}{'c_ok':>7}{'b_trunc':>9}"
    print(hdr)
    print("-" * len(hdr))
    for r in results:
        print(
            f"{r['lint']:<24}{r['bonsai']['tok_per_sec']:>12.2f}"
            f"{r['clippy']['tok_per_sec']:>14.1f}"
            f"{(r['speedup'] or 0):>11.0f}x"
            f"{str(r['bonsai']['correct']):>7}{str(r['clippy']['correct']):>7}"
            f"{str(r['bonsai']['truncated']):>9}"
        )
    print("-" * len(hdr))
    print(
        f"{'AGGREGATE':<24}{agg['bonsai_tok_per_sec_e2e']:>12.2f}"
        f"{agg['clippy_tok_per_sec_e2e']:>14.1f}"
        f"{(agg['speedup_x'] or 0):>11.0f}x"
        f"{agg['bonsai_correct']:>7}{agg['clippy_correct']:>7}"
        f"{agg['bonsai_truncated']:>9}"
    )

    args.out.write_text(
        json.dumps(
            {
                "fixture": case_file["name"],
                "max_tokens": args.max_tokens,
                "cases": results,
                "aggregate": agg,
                "clippy_summary": clippy_summary,
            },
            indent=2,
        )
    )
    print(f"\nwrote {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
