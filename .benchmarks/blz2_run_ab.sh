#!/bin/bash
# Bench 139 Part B2 — blaze vs cargo on riir-refine (compatible-workspace check), same protocol shape.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
ROOT=/tmp/blz2; WT=$ROOT/riir-refine; OUT=$ROOT/out; mkdir -p "$OUT"
N=${N:-5}
cd "$ROOT"
for d in "$HOME"/git/*; do b=$(basename "$d"); [ "$b" = riir-refine ] && continue; [ -e "$ROOT/$b" ] || ln -s "$d" "$ROOT/$b"; done
[ -d "$WT" ] || git -C "$HOME/git/riir-refine" worktree add -q --detach "$WT" HEAD
cd "$WT"
echo "refine_commit=$(git rev-parse --short HEAD)" > "$OUT/meta.txt"
cargo -V >> "$OUT/meta.txt"; blaze --version >> "$OUT/meta.txt"
bash "$HOME/git/riir-reflex/scripts/bench_preflight.sh" 2>&1 | grep PROVENANCE >> "$OUT/meta.txt"
LIB=src/lib.rs
grep -q blaze_probe "$LIB" || printf '\n#[doc(hidden)]\npub fn blaze_probe() -> u32 {\n    0\n}\n' >> "$LIB"
edit() { python3 - "$LIB" "$1" <<'PY'
import re,sys
p,i=sys.argv[1],sys.argv[2]; s=open(p).read()
s=re.sub(r"(pub fn blaze_probe\(\) -> u32 \{\n    )\d+", r"\g<1>"+i, s); open(p,"w").write(s)
PY
}
now() { python3 -c 'import time;print(time.time())'; }
cell() {
  local label=$1; shift; local t0 t1 rc
  t0=$(now); "$@" > "$OUT/$label.log" 2>&1; rc=$?; t1=$(now)
  local res; res=$(grep -E "test result:" "$OUT/$label.log" | tail -1)
  printf '%s rc=%s wall=%.2f %s\n' "$label" "$rc" "$(python3 -c "print($t1-$t0)")" "$res" | tee -a "$OUT/cells.txt"
}
: > "$OUT/cells.txt"
i=1
edit $i; cell warmA_first cargo test --lib --quiet
i=$((i+1)); edit $i; cell warmB_first blaze test --lib
i=$((i+1)); edit $i; cell warmA_2 cargo test --lib --quiet
i=$((i+1)); edit $i; cell warmB_2 blaze test --lib
for k in $(seq 1 "$N"); do
  i=$((i+1)); edit $i; cell "A$k" cargo test --lib --quiet
  i=$((i+1)); edit $i; cell "B$k" blaze test --lib
done
blaze status > "$OUT/blaze_status.txt" 2>&1
du -sh "$WT/target" "$HOME/.blaze" > "$OUT/disk.txt" 2>&1
blaze stop >> "$OUT/blaze_status.txt" 2>&1
git -C "$HOME/git/riir-refine" worktree remove --force "$WT" && echo worktree_removed >> "$OUT/meta.txt"
for l in "$ROOT"/*; do [ -L "$l" ] && rm "$l"; done
echo DONE >> "$OUT/cells.txt"
