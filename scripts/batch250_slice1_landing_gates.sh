#!/bin/sh
# batch250_slice1_landing_gates.sh — Batch 250 slice 1 landing-gate runner.
#
# Source of truth: .plans/254_batch250_envelope_pool.md (T6-T10). This script
# encodes the RUNNABLE half of the deferred gate set; T9/T10 (Research 285,
# AGENTS count lines, .docs rows, the CRLF .distill landing row, the
# build-stamp == HEAD check, push) stay the landing agent's.
#
#   gate                            plan  host        pre-registered
#   ------------------------------- ----  ----------  ---------------------------
#   default-posture lib tests       T7/T8  any        2695 passed / 0 failed
#   combo lib tests                 T8     any        0 failed (the compile IS the cost)
#   rust_perf lib tests             T8     any        0 failed (+ T7 count asserts 879/231)
#   clippy x2 postures              T8     any        zero warnings
#   retrieval floors re-measure     T8     any        floors held — eyeball the floor lines
#   score-bench                     T8     any        delta row appended to score_history.jsonl
#   x86 Pareto (new rust_perf rule) T6     x86_64     3 runs + box state; SKIP on aarch64
#
# Notes:
# - Plan-337 quiet-box gate: refuses at 1-min load >= LOAD_MAX (default 8.0)
#   unless --force. The staged window measured load 20-50 (deferred) vs 4-6
#   (eligible) — this gate is the doctrine in executable form.
# - Freeze tables are NOT owed this batch: zero clippy-corpus delta (plan 254).
# - CARGO_TARGET_DIR defaults to /tmp/b250-target — the staged window's green
#   2695/0 incremental build lives there; keep it for the cheap re-run.
# - Pre-landing, run against the staging worktree:
#     scripts/batch250_slice1_landing_gates.sh ../riir-refine-b250wt
#   Post-landing (corpus on develop), run it in-place with no argument.

set -eu

REPO=""
FORCE=0
for a in "$@"; do
    if [ "$a" = "--force" ]; then FORCE=1; else REPO="$a"; fi
done
if [ -z "$REPO" ]; then
    if [ -f src/domains/kernel_opt/corpus/mod.rs ]; then REPO=.; else REPO=../riir-refine-b250wt; fi
fi
if [ ! -f "$REPO/src/domains/kernel_opt/corpus/mod.rs" ]; then
    echo "refuse: $REPO does not look like a riir-refine checkout" >&2
    exit 2
fi

# Plan-337 quiet-box gate.
LOAD_MAX="${LOAD_MAX:-8.0}"
load1="$(uptime | sed 's/.*load average[s]*: //' | cut -d, -f1 | tr -d ' ')"
over="$(awk -v l="$load1" -v m="$LOAD_MAX" 'BEGIN{print (l>=m)?1:0}')"
if [ "$over" = "1" ] && [ "$FORCE" != "1" ]; then
    echo "refuse: box load $load1 >= $LOAD_MAX (Plan-337 quiet-box gate); --force overrides" >&2
    exit 3
fi
echo "box load: $load1 (gate <= $LOAD_MAX)"

EXPECT_LIB="${EXPECT_LIB:-2695}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/b250-target}"
cd "$REPO"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

fail() { echo "FAIL: $1" >&2; tail -40 "$tmp" >&2; exit 1; }

echo
echo "=== [T7/T8] default-posture lib tests (expect $EXPECT_LIB passed / 0 failed)"
cargo test --lib >"$tmp" 2>&1 || fail "default lib tests"
grep 'test result' "$tmp" | tail -2
passed="$(grep 'test result' "$tmp" | tail -1 | grep -o '[0-9]* passed' | head -1 | cut -d' ' -f1)"
if [ -z "$passed" ] || [ "$passed" -lt "$EXPECT_LIB" ]; then
    echo "FAIL: passed=$passed < expected floor $EXPECT_LIB" >&2
    exit 1
fi
if [ "$passed" != "$EXPECT_LIB" ]; then
    echo "NOTE: passed=$passed != pre-registered $EXPECT_LIB — correct, then re-pin the number"
fi

echo
echo "=== [T8] combo lib tests (kernel_opt,latent_retrieval) — the compile IS the cost"
cargo test --lib --features kernel_opt,latent_retrieval >"$tmp" 2>&1 || fail "combo lib tests"
grep 'test result' "$tmp" | tail -2
grep 'test result' "$tmp" | tail -1 | grep -q '; 0 failed' || fail "combo posture has failures"

echo
echo "=== [T8] retrieval floors re-measure (from the combo run above)"
grep -i 'floor' "$tmp" | head -12 || echo "(no floor lines printed — read the floors test output manually)"

echo
echo "=== [T8] rust_perf lib tests (T7 count asserts 879/231 ride in here)"
cargo test --lib --features rust_perf >"$tmp" 2>&1 || fail "rust_perf lib tests"
grep 'test result' "$tmp" | tail -2
grep 'batch250_slice1' "$tmp" | head -6 || echo "(batch250 gate test names not in output — check the filter)"

echo
echo "=== [T8] clippy x2 postures (default + combo)"
cargo clippy --all-targets -- -D warnings >"$tmp" 2>&1 || fail "clippy default"
echo "clippy default: clean"
cargo clippy --all-targets --features kernel_opt,latent_retrieval -- -D warnings >"$tmp" 2>&1 || fail "clippy combo"
echo "clippy combo: clean"

echo
echo "=== [T8] score-bench (release-set build; doubles as the T10 rebuild)"
cargo build --release --features latent_retrieval,kat_service,kat_transport,fix_verify,update_check --bin cargo-refine >"$tmp" 2>&1 || fail "release-set build"
BIN="$CARGO_TARGET_DIR/release/cargo-refine"
"$BIN" --version | head -4 || true
echo "^ T10 reminder: build hash must equal the landing HEAD at push"
"$BIN" --score-bench || fail "score-bench run"

echo
echo "=== [T6] x86 Pareto leg (avx2-tiled-accumulator-butterfly-epilogue)"
case "$(uname -m)" in
    x86_64)
        cargo bench --bench perf_bench_demo --features rust_perf_bench >"$tmp" 2>&1 || fail "perf_bench_demo"
        echo "Pareto bench done — record 3 runs + box state beside the numbers (G2 law)"
        ;;
    *)
        echo "SKIP (aarch64 host): the rule is x86_64-only by cfg law."
        echo "On the 4090: cargo bench --bench perf_bench_demo --features rust_perf_bench"
        echo "(or ship fixture-pending again, disclosed — plan 254 T6 allows either)"
        ;;
esac

echo
echo "=== landing gates DONE (runnable half). Remaining per plan 254: T9 docs/research rows, T10 .distill CRLF row + stamp==HEAD + push."
