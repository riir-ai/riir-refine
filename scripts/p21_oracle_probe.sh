#!/bin/sh
# p21_oracle_probe.sh — post-rebuild verification of the maintained cargo-refine
# binary's P21 oracle lane (verbatim MachineApplicable consumption) + the P104
# scope-guard posture, end to end on a scratch fixture.
#
# Why: the P98 dogfood caught 11 E0433s only because a final -D run asserted
# the exit code — the P21 + P104 mechanics need a REAL run, not unit tests.
# Every rebuild of the global binary (the MAINT-1 class) re-runs this.
# The 2026-09-14 `.raw/oracle_probe` scratch fixture predates P104 and was
# removed; this script is the durable form.
#
# The fixture exercises three MachineApplicable shapes (clippy 1.99.1):
#   useless_vec         single-span replacement    (corpus: typically NO rule)
#   iter_cloned_collect chain rename+deletion      (corpus: typically NO rule)
#   filter_next         receiver-carried rewrite   (corpus: HAS a rule)
#
# THE CONTRACT (not generic `cargo clippy --fix` — by design):
#   - in-lane lints (a corpus rule exists): the verbatim suggestion applies
#     as one atomic edit (P21);
#   - out-of-lane lints (no corpus rule): the edit is SKIPPED and the run
#     discloses the `[no corpus rule]` gap signal — the mining queue's feed;
#   - the P104 scope guard declines nothing here (path-free replacements).
# Assertions: filter_next APPLIED; any unapplied fixture lint must carry the
# gap-signal line (never silently unapplied); no scope decline; the fixture
# still compiles. The two corpus-less lints may flip to applied as the corpus
# grows — both postures pass; the run output is the record.
#
# The probe spawns cargo clippy (the auto-oracle) + the fix_verify compile
# gate, both inside the fixture's own target dir (isolated by construction).
# Run it niced on a box under measurement (the Plan-337 carve-out posture).
#
# Usage: scripts/p21_oracle_probe.sh [binary]    (default ~/.cargo/bin/cargo-refine)
# Exit 0 = PASS.
set -u

BIN="${1:-$HOME/.cargo/bin/cargo-refine}"
[ -x "$BIN" ] || { echo "PROBE FAIL: binary not executable: $BIN"; exit 2; }
echo "== binary =="
"$BIN" --version || exit 2

FIXDIR="$(mktemp -d /tmp/p21_probe.XXXXXX)"
trap 'rm -rf "$FIXDIR"' EXIT
mkdir -p "$FIXDIR/src"
cat > "$FIXDIR/Cargo.toml" <<'EOF'
[package]
name = "p21_probe"
version = "0.1.0"
edition = "2021"
EOF
cat > "$FIXDIR/src/main.rs" <<'EOF'
fn main() {
    let items = vec![1u32, 2, 3];
    let n = items.len();
    let s = format!("count: {}", n);
    let doubled: Vec<u32> = items.iter().cloned().collect();
    let first_over = items.iter().filter(|x| **x > 1).next();
    println!("{} {:?} {:?}", s, doubled, first_over);
}
EOF

echo "== fix run =="
PROBE_LOG="$FIXDIR/probe.log"
nice -n 19 "$BIN" --fix "$FIXDIR" >"$PROBE_LOG" 2>&1
RC=$?
echo "fix rc=$RC"
sed 's/^/  | /' "$PROBE_LOG"

FAIL=0
check_applied() {
    if grep -qF "$2" "$FIXDIR/src/main.rs"; then
        echo "applied: $1"
    else
        echo "MISSING: $1"
        FAIL=1
    fi
}
check_applied "filter_next -> find (in-lane, verbatim)" 'items.iter().find(|x| **x > 1)'

# Corpus-less lints: skipped by design, but the gap signal MUST be disclosed.
# If the corpus has since grown a rule, applied is ALSO correct; what is
# never correct is unapplied AND undisclosed.
if ! grep -qF 'let items = [1u32, 2, 3];' "$FIXDIR/src/main.rs"; then
    if grep -qiE "no[ -]corpus[ -]rule" "$PROBE_LOG"; then
        echo "skipped-by-design: useless_vec (gap signal disclosed)"
    else
        echo "MISSING: useless_vec neither applied nor gap-signalled"
        FAIL=1
    fi
else
    echo "applied (corpus grew a rule): useless_vec"
fi
if ! grep -qF 'items.to_vec()' "$FIXDIR/src/main.rs"; then
    if grep -qiE "no[ -]corpus[ -]rule" "$PROBE_LOG"; then
        echo "skipped-by-design: iter_cloned_collect (gap signal disclosed)"
    else
        echo "MISSING: iter_cloned_collect neither applied nor gap-signalled"
        FAIL=1
    fi
else
    echo "applied (corpus grew a rule): iter_cloned_collect"
fi
if grep -q "scope declined\|path_scope_declined" "$PROBE_LOG"; then
    echo "UNEXPECTED: P104 scope decline on path-free replacements"
    FAIL=1
fi

echo "== compile check =="
( cd "$FIXDIR" && nice -n 19 cargo check --quiet ) || FAIL=1

if [ "$RC" -ne 0 ] || [ "$FAIL" -ne 0 ]; then
    echo "PROBE FAIL (fix rc=$RC, assertions fail=$FAIL)"
    cat "$FIXDIR/src/main.rs"
    exit 1
fi
echo "PROBE PASS: P21 verbatim consumption + gap-signal disclosure + P104 scope posture hold; fixture compiles."
