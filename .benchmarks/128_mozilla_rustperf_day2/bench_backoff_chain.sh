#!/bin/sh
# bench_backoff_chain.sh — the multi-level bench cell for mozilla/sccache #2968
# (the companion to bench_backoff.sh: that one benches the standalone WebDAV
# backend; this one benches SCCACHE_MULTILEVEL_CHAIN=disk,webdav with a stalled
# remote level, which is the case from the design comment: a local disk tier
# keeps serving hits while the remote level is in cooldown).
#
# Fault shape: same fake WebDAV as bench_backoff.sh — GETs never answer,
# PUT/MKCOL/HEAD/DELETE succeed. SCCACHE_CACHE_READ_TIMEOUT_SECS=1.
#
# Workload: 48 distinct tiny .c compiles per measured run through a fresh
# sccache daemon per rep, in a fixed order:
#   3 cold keys  -> the remote level is probed and times out; the third
#                   timeout trips its gate (AFTER arm)
#   16 warm keys -> seeded into the DISK tier first, so these are disk hits
#                   served while the remote gate is open
#   29 cold keys -> skipped instantly by the open gate (AFTER arm), timed out
#                   one by one (BEFORE arm)
# The disk tier is seeded per rep into a fresh dir, so reps and arms cannot
# contaminate each other through the disk.
#
# Protocol: 3 alternating reps per arm (B A B A B A) + 1 demote rep, median
# per arm. Startup/retirement/seed outside the timed region.
#
# usage: bench_backoff_chain.sh <before-bin> <after-bin> <workdir> [arm]
#   arm: all | before | after | demote   (default all)
set -u

BEFORE_BIN=${1:?usage: bench_backoff_chain.sh <before-bin> <after-bin> <workdir> [arm]}
AFTER_BIN=${2:?usage: bench_backoff_chain.sh <before-bin> <after-bin> <workdir> [arm]}
WORK=${3:?usage: bench_backoff_chain.sh <before-bin> <after-bin> <workdir> [arm]}
ARM=${4:-all}

THRESHOLD=3
COOLDOWN=60

mkdir -p "$WORK"
cd "$WORK" || exit 2

now() { python3 -c 'import time; print(f"{time.time():.3f}")'; }
elapsed() { awk "BEGIN{printf \"%.3f\", $2 - $1}"; }

# ---------------------------------------------------------------- fake backend
# Modes via the flag file $2 (created by the script between phases):
#   absent  -> HEALTHY: GET serves previously-PUT bytes (200), 404 for
#              unknown keys; PUT/MKCOL/HEAD/DELETE succeed.
#   present -> UNHEALTHY: every GET stalls (the issue's fault shape — reads
#              never answer); writes still succeed.
# One backend process serves the whole rep: the seed runs healthy (entries
# land in BOTH chain levels through the live chain), then the flag flips the
# same backend to stalling — no daemon restart, no posture transition, the
# exact "backend goes unhealthy mid-flight" scenario.
cat > fake_webdav.py <<'PYEOF'
import os, sys, threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LOG = open(sys.argv[2], "a", buffering=1)
FLAG = sys.argv[3]
STORE = {}
LOCK = threading.Lock()

class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        LOG.write("%s\n" % (fmt % args))

    def do_GET(self):
        if os.path.exists(FLAG):
            LOG.write("GET %s (stalling)\n" % self.path)
            try:
                self.connection.settimeout(None)
                self.rfile.readline()
            except Exception:
                pass
            return
        with LOCK:
            data = STORE.get(self.path)
        if data is not None:
            LOG.write("GET %s (200 %db)\n" % (self.path, len(data)))
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        else:
            LOG.write("GET %s (404)\n" % self.path)
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()

    def do_PUT(self):
        n = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(n) if n else b""
        with LOCK:
            STORE[self.path] = body
        LOG.write("PUT %s (201 %db)\n" % (self.path, len(body)))
        self.send_response(201)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_MKCOL(self):
        LOG.write("MKCOL %s (201)\n" % self.path)
        self.send_response(201)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_HEAD(self):
        with LOCK:
            known = self.path in STORE
        LOG.write("HEAD %s (%d)\n" % (self.path, 200 if known else 404))
        self.send_response(200 if known else 404)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_DELETE(self):
        LOG.write("DELETE %s (200)\n" % self.path)
        self.send_response(200)
        self.send_header("Content-Length", "0")
        self.end_headers()

ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
PYEOF

# ------------------------------------------------------------- workload setup
# 16 warm keys (t0..t15, seeded into the disk tier) + 32 cold keys (t32..t63).
mkdir -p src obj
i=0
while [ "$i" -lt 16 ]; do
  cat > "src/t$i.c" <<CEOF
int f$i(int x) { return x + $i; }
CEOF
  i=$((i + 1))
done
i=32
while [ "$i" -lt 64 ]; do
  cat > "src/t$i.c" <<CEOF
int f$i(int x) { return x * 2 + $i; }
CEOF
  i=$((i + 1))
done

free_port() {
  python3 - <<'PYEOF'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PYEOF
}

wait_port_free() {
  n=0
  while [ "$n" -lt 50 ]; do
    if ! nc -z 127.0.0.1 "$1" 2>/dev/null; then
      return 0
    fi
    sleep 0.2
    n=$((n + 1))
  done
  echo "WARN: port $1 still accepting after 10s" >&2
  return 1
}

# ------------------------------------------------------------------- one rep
# $1 = sccache binary, $2 = arm name, $3 = rep number
one_rep() {
  BIN=$1
  ARM_NAME=$2
  REP=$3

  BACKEND_PORT=$(free_port)
  SCCACHE_PORT=$(free_port)
  RUN_DIR="$WORK/run_${ARM_NAME}_${REP}"
  mkdir -p "$RUN_DIR"
  BACKEND_LOG="$RUN_DIR/backend.log"
  DISK_DIR="$RUN_DIR/sccache-dir"
  STALL_FLAG="$RUN_DIR/STALL"

  # fresh fake backend for this rep (healthy until the flag file appears)
  python3 fake_webdav.py "$BACKEND_PORT" "$BACKEND_LOG" "$STALL_FLAG" &
  BACKEND_PID=$!

  export SCCACHE_DIR="$DISK_DIR"
  export SCCACHE_WEBDAV_ENDPOINT="http://127.0.0.1:$BACKEND_PORT"
  export SCCACHE_WEBDAV_DISABLE_CREATE_DIR=true
  export SCCACHE_CACHE_READ_TIMEOUT_SECS=1
  export SCCACHE_SKIP_CACHE_CHECK=1
  export SCCACHE_IDLE_TIMEOUT=0
  export SCCACHE_SERVER_PORT=$SCCACHE_PORT
  SCCACHE_BACKEND_BACKOFF_THRESHOLD=${BACKOFF_THRESHOLD:-$THRESHOLD}
  export SCCACHE_BACKEND_BACKOFF_THRESHOLD
  export SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS=$COOLDOWN

  # ---- seed phase (untimed): 16 compiles THROUGH THE CHAIN against the
  # healthy backend — entries land in the disk tier (and the remote), exactly
  # the production warm state. One daemon per rep; the first compile
  # auto-spawns it with the current env, so there is no stop->start
  # transition anywhere in the rep (the explicit-daemon transition was
  # measured missing freshly seeded L0 entries on this box, 2026-11-10).
  export SCCACHE_MULTILEVEL_CHAIN=disk,webdav
  mkdir -p obj/seed$REP
  i=0
  while [ "$i" -lt 16 ]; do
    "$BIN" cc -c "src/t$i.c" -o "obj/seed$REP/w$i.o" || exit 3
    i=$((i + 1))
  done
  "$BIN" --show-stats >/dev/null 2>&1 || true

  # ---- flip the same backend to stalling; the timed region follows
  : > "$STALL_FLAG"

  T0=$(now)
  # 3 cold keys: probe + stall the remote level; the third timeout trips the gate
  i=32
  while [ "$i" -lt 35 ]; do
    "$BIN" cc -c "src/t$i.c" -o "obj/${ARM_NAME}${REP}_$i.o" || exit 5
    i=$((i + 1))
  done
  # 16 warm keys: disk hits served while the remote level is unhealthy
  i=0
  while [ "$i" -lt 16 ]; do
    "$BIN" cc -c "src/t$i.c" -o "obj/seed$REP/w$i.o" || exit 5
    i=$((i + 1))
  done
  # 29 cold keys: skipped (AFTER) or timed out one by one (BEFORE)
  i=35
  while [ "$i" -lt 64 ]; do
    "$BIN" cc -c "src/t$i.c" -o "obj/${ARM_NAME}${REP}_$i.o" || exit 5
    i=$((i + 1))
  done
  T1=$(now)
  WALL=$(elapsed $T0 $T1)

  "$BIN" --show-stats --stats-format json > "$RUN_DIR/show_stats.json" 2>"$RUN_DIR/show_stats.err"
  STATS=$(python3 -c "
import json
d = json.load(open('$RUN_DIR/show_stats.json'))
s = d.get('stats', d)
hits = s.get('cache_hits')
if isinstance(hits, dict):
    counts = hits.get('counts', {})
    hits = sum(counts.values())
print('reqs=%s hits=%s timeouts=%s writes=%s werrs=%s' % (
    s.get('compile_requests'), hits, s.get('cache_timeouts'),
    s.get('cache_writes'), s.get('cache_write_errors')))
" 2>/dev/null || echo STATS_PARSE_FAILED)
  GETS=$(grep -c '^GET ' "$BACKEND_LOG" 2>/dev/null)
  PUTS=$(grep -c '^PUT ' "$BACKEND_LOG" 2>/dev/null)

  echo "rep arm=$ARM_NAME rep=$REP wall=${WALL}s $STATS backend_gets=$GETS backend_puts=$PUTS"

  "$BIN" --stop-server >/dev/null 2>&1
  kill "$BACKEND_PID" 2>/dev/null
  wait_port_free "$SCCACHE_PORT" || true
  wait_port_free "$BACKEND_PORT" || true
}

# ------------------------------------------------------------------- driver
SEQ=0
case "$ARM" in
before) ARMS="before before before" ;;
after) ARMS="after after after" ;;
demote) ARMS="demote" ;;
all) ARMS="before after before after before after demote" ;;
*) echo "unknown arm $ARM" >&2; exit 2 ;;
esac

for a in $ARMS; do
  SEQ=$((SEQ + 1))
  if [ "$a" = demote ]; then
    BACKOFF_THRESHOLD=0
    one_rep "$AFTER_BIN" demote "$SEQ"
  else
    BACKOFF_THRESHOLD=$THRESHOLD
    if [ "$a" = before ]; then
      one_rep "$BEFORE_BIN" before "$SEQ"
    else
      one_rep "$AFTER_BIN" after "$SEQ"
    fi
  fi
done

echo "done"
