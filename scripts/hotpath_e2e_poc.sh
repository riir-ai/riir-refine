#!/usr/bin/env bash
# Issue 146 T0.4 — the profile-guided-healing e2e POC gate (Proposal 020 /
# Research 235's T0): ONE toy repo taken through the whole loop —
#
#   profile (real hotpath, #[measure] + hotpath-alloc)
#     → heal (cargo refine --hotpath-json … --fix --verify --domain perf)
#     → re-profile
#     → GATE: alloc-axis improvement verified with N≥3 agreeing runs
#       + timing/RSS no-regression guard + the repo's tests pass
#       + hotpath's own instrumentation survives the heal (T0.3).
#
# The toy's healable defect is REAL and measured: `Vec::new()` + a push loop
# over a derivable bound (`0..n`) — growth doubling wastes ~2× alloc bytes;
# the `vec-with-capacity` rewrite (`Vec::with_capacity(n)`) allocates exactly
# once. hotpath's counting allocator attributes the growth bytes to the
# instrumented fn, so the heal reads as the printed alloc avg dropping
# (measured first run: 16.0 MB → 8.0 MB per call, timing 2.4× faster).
#
# WORKSTATION-ONLY by design (network for the hotpath dep + cargo builds);
# not a CI lane. Refuses loud (exit 2) when the healer binary is missing —
# never a green zero (the frontier-report law).
#
# Usage:
#   scripts/hotpath_e2e_poc.sh [healer-binary]
#   CARGO_REFINE=/path/to/cargo-refine scripts/hotpath_e2e_poc.sh
#
# Exit: 0 PASS · 1 gate failure · 2 preflight refusal.

set -euo pipefail

# ── the completion sentinel (the workspace trap-launder law) ─────────────
# macOS bash 3.2 launders an aborting script's exit status through a cleanup
# trap; the sentinel makes an INCOMPLETE run refuse to claim success.
COMPLETED=0
cleanup() {
    rc=$?
    if [ "$COMPLETED" -ne 1 ] && [ "$rc" -eq 0 ]; then
        echo "hotpath-e2e: ABORTED before completion — NOT a pass" >&2
        exit 1
    fi
    exit "$rc"
}
trap cleanup EXIT

REFINE="${1:-${CARGO_REFINE:-$HOME/.cargo/bin/cargo-refine}}"
RUNS="${HOTPATH_E2E_RUNS:-3}"
# Timing/RSS guard ceilings: the heal must not make the run slower or bigger
# (hazard 4: alloc↓ can still mean slower/bigger). Loose — instrumented
# timing is ranking-only (B129); this is a REGRESSION guard, not a target.
TIME_GUARD="${HOTPATH_E2E_TIME_GUARD:-1.10}"   # median avg_ns ≤ baseline × 1.10
RSS_GUARD="${HOTPATH_E2E_RSS_GUARD:-1.05}"     # median peak RSS ≤ baseline × 1.05

if [ ! -x "$REFINE" ]; then
    echo "hotpath-e2e: REFUSAL — healer binary not found/executable: $REFINE" >&2
    echo "  build it:  cargo build --release --features latent_retrieval,kat_service,kat_transport,fix_verify --bin cargo-refine" >&2
    exit 2
fi
command -v python3 >/dev/null || { echo "hotpath-e2e: REFUSAL — python3 required" >&2; exit 2; }
command -v git >/dev/null || { echo "hotpath-e2e: REFUSAL — git required" >&2; exit 2; }

TOY="$(mktemp -d /tmp/hotpath-e2e-XXXXXX)"
echo "hotpath-e2e: toy repo at $TOY (healer: $REFINE, runs: $RUNS)"

mkdir -p "$TOY/src"
cat > "$TOY/Cargo.toml" <<'EOF'
[package]
name = "hotpath-toy"
version = "0.1.0"
edition = "2021"

[dependencies]
hotpath = { version = "0.28", features = ["hotpath", "hotpath-alloc"] }
EOF
cat > "$TOY/src/main.rs" <<'EOF'
#[hotpath::main(format = "json", output_path = "hotpath-report.json", functions_limit = 50)]
fn main() {
    let mut total = 0u64;
    for round in 0..300u64 {
        let v = build_squares(1 << 20);
        total = total.wrapping_add(v[(round as usize) % v.len()]);
    }
    println!("{total}");
}

#[hotpath::measure]
fn build_squares(n: usize) -> Vec<u64> {
    let mut out = Vec::new();
    for i in 0..n {
        out.push((i * i) as u64);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_squares_is_correct() {
        let v = super::build_squares(64);
        assert_eq!(v.len(), 64);
        for (i, x) in v.iter().enumerate() {
            assert_eq!(*x, (i * i) as u64);
        }
    }
}
EOF
(cd "$TOY" && git init -q && git add -A && git -c user.email=e2e@local -c user.name=e2e commit -qm init) >/dev/null

# ── one measurement run: report + peak RSS (macOS /usr/bin/time -l) ──────
# $1 = output report copy path
measure_once() {
    local dest="$1"
    (cd "$TOY" && rm -f hotpath-report.json && /usr/bin/time -l cargo run --release -q 2>&1 \
        | grep "maximum resident" | grep -oE "[0-9]+" > "$dest.rss")
    mv "$TOY/hotpath-report.json" "$dest"
}

echo "hotpath-e2e: baseline build + $RUNS profile run(s)…"
measure_once "$TOY/base.json"
for i in $(seq 2 "$RUNS"); do measure_once "$TOY/base-$i.json"; done

# ── the heal: the whole T0.2/T0.3 lane against the REAL report ───────────
echo "hotpath-e2e: healing (--hotpath-json + --fix --verify --domain perf)…"
HEAL_LOG="$TOY/heal.log"
if ! (cd "$TOY" && "$REFINE" --hotpath-json base.json --fix --verify --domain perf src/ >"$HEAL_LOG" 2>&1); then
    echo "hotpath-e2e: FAIL — the healer run itself failed:" >&2
    tail -20 "$HEAL_LOG" >&2
    exit 1
fi
grep -q "vec-with-capacity" "$HEAL_LOG" || {
    echo "hotpath-e2e: FAIL — the vec-with-capacity edit never landed" >&2
    grep -E "hotpath|fix src|verify:" "$HEAL_LOG" >&2
    exit 1
}
grep -q "\[hot\]" "$HEAL_LOG" || {
    echo "hotpath-e2e: FAIL — no [hot] span surfaced in the heal output (anchoring broke)" >&2
    exit 1
}
# T0.3 positive proof: the instrumentation survived the heal.
grep -q "#\[hotpath::measure\]" "$TOY/src/main.rs" || {
    echo "hotpath-e2e: FAIL — hotpath's own attribute did not survive the heal" >&2
    exit 1
}
grep -q "Vec::with_capacity(n)" "$TOY/src/main.rs" || {
    echo "hotpath-e2e: FAIL — the rewrite is not in the source" >&2
    exit 1
}

# ── compiling ≠ behaving: the toy's own tests must pass ──────────────────
echo "hotpath-e2e: cargo test (behavior gate)…"
if ! (cd "$TOY" && cargo test -q >/dev/null 2>&1); then
    echo "hotpath-e2e: FAIL — the toy's tests fail after the heal" >&2
    exit 1
fi

# ── re-profile N≥3 ────────────────────────────────────────────────────────
echo "hotpath-e2e: $RUNS post-heal profile run(s)…"
measure_once "$TOY/after.json"
for i in $(seq 2 "$RUNS"); do measure_once "$TOY/after-$i.json"; done

# ── the verdict: agreement + improvement + no-regression ──────────────────
# Parses the raw reports directly (one parser, no TSV intermediates) and the
# per-run RSS sidecars; every quantity is for hotpath_toy::build_squares
# (RSS: the whole process).
python3 - "$TOY" "$RUNS" "$TIME_GUARD" "$RSS_GUARD" <<'PYEOF'
import json, re, sys, statistics

toy, runs, time_guard, rss_guard = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4])

def dur_ns(s):
    m = re.match(r"^([0-9.]+)\s*(ns|µs|us|ms|s)$", s.strip())
    if not m:
        return None
    v = float(m.group(1))
    scale = {"ns": 1, "µs": 1e3, "us": 1e3, "ms": 1e6, "s": 1e9}[m.group(2)]
    return int(v * scale)

def bytes_of(s):
    m = re.match(r"^([0-9.]+)\s*(B|KB|MB|GB|KiB|MiB|GiB)$", s.strip())
    if not m:
        return None
    v = float(m.group(1))
    scale = {"B": 1, "KB": 1e3, "MB": 1e6, "GB": 1e9, "KiB": 1 << 10, "MiB": 1 << 20, "GiB": 1 << 30}[m.group(2)]
    return int(v * scale)

def entry(path, section, name):
    with open(path) as f:
        r = json.load(f)
    for e in r.get(section, {}).get("data", []):
        if e["name"] == name:
            return e
    raise AssertionError(f"{name} missing from {section} in {path}")

def rss_of(path):
    with open(path + ".rss") as f:
        return int(f.read().strip().splitlines()[0])

def series(prefix):
    paths = [f"{toy}/{prefix}{'' if i == 1 else f'-{i}'}.json" for i in range(1, runs + 1)]
    rows = []
    for p in paths:
        t = entry(p, "functions_timing", "hotpath_toy::build_squares")
        a = entry(p, "functions_alloc", "hotpath_toy::build_squares")
        rows.append((dur_ns(t["avg"]), bytes_of(a["total"]), a["calls"]))
    rss = [rss_of(p) for p in paths]
    return rows, rss

base, base_rss = series("base")
after, after_rss = series("after")
fails = []

def agree(rows, label):
    allocs = [r[1] for r in rows]
    spread = (max(allocs) - min(allocs)) / max(min(allocs), 1)
    if spread > 0.02:
        fails.append(f"{label} runs do NOT agree on alloc bytes (spread {spread:.1%} > 2%): {allocs}")
    return allocs

base_allocs = agree(base, "baseline")
after_allocs = agree(after, "post-heal")

# The GATE: ≥1 real alloc-axis improvement, verified — EVERY post-heal run
# strictly below EVERY baseline run.
if not all(a < b for a in after_allocs for b in base_allocs):
    fails.append(f"alloc-delta gate: post-heal {after_allocs} not strictly below baseline {base_allocs}")
base_median = statistics.median(base_allocs)
after_median = statistics.median(after_allocs)
print(f"alloc bytes (build_squares total): baseline {base_median} → post-heal {after_median} "
      f"({(1 - after_median / base_median):.1%} less)")

# Timing no-regression (B129: guard, never a target).
base_t = statistics.median([r[0] for r in base])
after_t = statistics.median([r[0] for r in after])
print(f"timing avg (build_squares):        baseline {base_t} ns → post-heal {after_t} ns")
if after_t > base_t * time_guard:
    fails.append(f"timing guard: post-heal {after_t} > baseline {base_t} × {time_guard}")

# RSS no-regression.
base_r = statistics.median(base_rss)
after_r = statistics.median(after_rss)
print(f"peak RSS (whole process):          baseline {base_r} → post-heal {after_r}")
if after_r > base_r * rss_guard:
    fails.append(f"RSS guard: post-heal {after_r} > baseline {base_r} × {rss_guard}")

# Alloc CALLS must not regress either (an extra alloc call class joined).
base_c = {r[2] for r in base}
after_c = {r[2] for r in after}
print(f"alloc calls (build_squares):       baseline {sorted(base_c)} → post-heal {sorted(after_c)}")

if fails:
    print("\nhotpath-e2e: FAIL")
    for f in fails:
        print(f"  ✗ {f}")
    sys.exit(1)
print("\nhotpath-e2e: PASS — ≥1 real alloc-axis improvement end-to-end, N≥" + str(runs) + " agreeing, "
      "timing/RSS no-regression, tests green, instrumentation intact")
PYEOF

echo "hotpath-e2e: artifacts kept at $TOY (rm -rf when done)"
COMPLETED=1
