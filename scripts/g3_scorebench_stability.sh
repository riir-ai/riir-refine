#!/bin/sh
# G3 byte-stability arm (Issue 191 T4) — the standing proof that the
# clippy_lints + rust_perf default surfaces' score-bench verdicts are
# byte-stable with kernel_opt enabled.
#
# The claim (Issue 191 bar 3, with its disclosure boundary): the domain only
# ADDS a fan-out lane — the router verdict is a per-span annotation, never a
# gate — so the EXISTING domains' score-bench verdicts must be byte-identical
# between the default build (kernel_opt OFF) and the `+kernel_opt` build.
# Whole stdout is claimed, the feature-set disclosure lines excluded.
#
# The arm runs `--score-bench` twice per lane (v1 clippy lane, gen-5 perf
# lane) in ISOLATED target dirs, filters the two VOLATILE tail lines (the
# history run-count grows by one per bench run; the saturation verdict reads
# that history), and diffs. Any coupling a future edit introduces between the
# kernel domain and the existing lanes' verdicts reds here.
#
# Postures: `--score-bench` requires the `score_bench` feature (which
# implies clippy_verify + latent_retrieval + clippy_lints + rust_perf), so
# the OFF posture is exactly `--features score_bench` and ON adds kernel_opt.
# Both postures carry the same store backend, so the bench tail's
# best-effort lines are symmetric.
#
# Usage: scripts/g3_scorebench_stability.sh [feature]
#   feature: the domain feature under test (default kernel_opt — the Issue
#   191 arm, now a no-op diff since default kernel_pregate implies it; Issue
#   212 T3 runs it for sec and shader_perf). The OFF posture
#   is `--features score_bench`, ON adds the feature.
# Env overrides: G3_TARGET_OFF / G3_TARGET_ON / G3_WORK (scratch dir).
set -eu
cd "$(dirname "$0")/.."

FEATURE=${1:-kernel_opt}
OFF_TARGET=${G3_TARGET_OFF:-/tmp/riir_refine_g3_off}
ON_TARGET=${G3_TARGET_ON:-/tmp/riir_refine_g3_on_$FEATURE}
WORK=${G3_WORK:-/tmp/riir_refine_g3_work_$FEATURE}
mkdir -p "$WORK"

filter_volatile() {
    # `history:` — the append-only store's run counts (the second run always
    # sees one more row). `saturation:` — a function of that same history.
    # `frontier snapshot (089 T4 bench tail):` — the local snapshot store's
    # STATE TRANSITION (appended vs unchanged-since-last-row): run-order
    # dependent, posture-independent (measured: the only clippy-lane diff of
    # the first arm run — the verdict tables matched byte-for-byte).
    grep -v -e '^history: ' -e '^saturation: ' \
        -e '^frontier snapshot (089 T4 bench tail): ' "$1"
}

run_lane() {
    # $1 features-arg, $2 target dir, $3 generation-arg, $4 out path
    CARGO_TARGET_DIR="$2" cargo run --release --bin cargo-refine $1 -- \
        --score-bench $3 >"$4" 2>"$4.err"
}

echo "== G3 arm: building the OFF posture (--features score_bench)…"
run_lane "--features score_bench" "$OFF_TARGET" "" "$WORK/off_clippy.txt"
echo "== G3 arm: building the ON posture (+$FEATURE)…"
run_lane "--features score_bench,$FEATURE" "$ON_TARGET" "" "$WORK/on_clippy.txt"
echo "== G3 arm: gen-5 (rust_perf) lane…"
run_lane "--features score_bench" "$OFF_TARGET" "--generation 5" "$WORK/off_perf.txt"
run_lane "--features score_bench,$FEATURE" "$ON_TARGET" "--generation 5" "$WORK/on_perf.txt"

fail=0
for lane in clippy perf; do
    if filter_volatile "$WORK/off_$lane.txt" >"$WORK/off_$lane.f" &&
        filter_volatile "$WORK/on_$lane.txt" >"$WORK/on_$lane.f" &&
        diff -u "$WORK/off_$lane.f" "$WORK/on_$lane.f" >"$WORK/${lane}_diff.txt"; then
        echo "G3 lane $lane: BYTE-STABLE ($(wc -l <"$WORK/off_$lane.f") verdict lines identical)"
    else
        echo "G3 lane $lane: DIVERGED — see $WORK/${lane}_diff.txt"
        cat "$WORK/${lane}_diff.txt"
        fail=1
    fi
done

if [ "$fail" -eq 0 ]; then
    echo "G3 verdict: PASS — the existing domains' score-bench verdicts are byte-stable with $FEATURE enabled"
else
    echo "G3 verdict: FAIL — $FEATURE perturbed an existing domain's verdicts (bar 3)"
fi
exit "$fail"
