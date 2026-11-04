#!/bin/sh
# bench_backoff.sh — G-PR1 bench for mozilla/sccache issue #2968
# "Avoid repeated read-timeout waits when a cache backend is unhealthy".
#
# Fault shape (mirrors the issue author's setup): a fake WebDAV backend that
# answers cache WRITES immediately but never answers cache READS (GET), so
# every cache lookup pays SCCACHE_CACHE_READ_TIMEOUT_SECS before the compile
# proceeds locally. PUTs work, exactly as in the issue ("Leave PUTs working
# against the healthy backend").
#
# Timeout knob: SCCACHE_CACHE_READ_TIMEOUT_SECS=1 (configurable since #2953,
# read in src/compiler/compiler.rs: cache_read_timeout()).
#
# Workload: 32 distinct tiny .c files compiled through sccache with the real
# system compiler (cc), sequentially, fresh sccache daemon per run, fresh
# backend per run. (sccache ships no standalone mock-compiler binary in
# tests/, so the real compiler is used — noisier per-compile times, identical
# across both arms.)
#
# Protocol: 3 alternating reps per arm (B A B A B A), all reps printed,
# median per arm. Wall time of the 32-request workload only; daemon startup
# (--start-server) and retirement (--stop-server) are outside the timed
# region.
#
# usage: bench_backoff.sh <before-sccache-bin> <after-sccache-bin> <workdir> [arm]
#   arm: all | before | after | demote   (default all)
set -u

BEFORE_BIN=${1:?usage: bench_backoff.sh <before-bin> <after-bin> <workdir> [arm]}
AFTER_BIN=${2:?usage: bench_backoff.sh <before-bin> <after-bin> <workdir> [arm]}
WORK=${3:?usage: bench_backoff.sh <before-bin> <after-bin> <workdir> [arm]}
ARM=${4:-all}

NREQ=32
THRESHOLD=3
COOLDOWN=60

mkdir -p "$WORK"
cd "$WORK" || exit 2

now() { python3 -c 'import time; print(f"{time.time():.3f}")'; }
elapsed() { awk "BEGIN{printf \"%.3f\", $2 - $1}"; }

# ---------------------------------------------------------------- fake backend
# GET (cache read) -> accepted, never answered. Everything else (PUT/MKCOL
# cache writes, HEAD, DELETE) -> answered immediately with success.
cat > fake_webdav.py <<'PYEOF'
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LOG = open(sys.argv[2], "a", buffering=1)

class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        LOG.write("%s\n" % (fmt % args))

    def do_GET(self):
        LOG.write("GET %s (stalling)\n" % self.path)
        try:
            # Accept and never respond: the read timeout fires against us.
            self.connection.settimeout(None)
            self.rfile.readline()
        except Exception:
            pass

    def do_PUT(self):
        n = int(self.headers.get("Content-Length") or 0)
        if n:
            self.rfile.read(n)
        LOG.write("PUT %s (201)\n" % self.path)
        self.send_response(201)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_MKCOL(self):
        LOG.write("MKCOL %s (201)\n" % self.path)
        self.send_response(201)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_HEAD(self):
        LOG.write("HEAD %s (200)\n" % self.path)
        self.send_response(200)
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
mkdir -p src obj
i=0
while [ "$i" -lt "$NREQ" ]; do
  cat > "src/t$i.c" <<CEOF
int f$i(int x) { return x + $i; }
CEOF
  i=$((i + 1))
done

# Fixed uncached control: the 32 compiles without sccache, once.
CTRL_START=$(now)
i=0
while [ "$i" -lt "$NREQ" ]; do
  cc -c "src/t$i.c" -o "obj/control$i.o" || exit 3
  i=$((i + 1))
done
CTRL_END=$(now)
echo "uncached control (32 compiles, no sccache): $(elapsed $CTRL_START $CTRL_END)s"

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

# ------------------------------------------------------------------- one run
# $1 = sccache binary, $2 = arm name, $3 = rep number
one_run() {
  BIN=$1
  ARM_NAME=$2
  REP=$3

  BACKEND_PORT=$(free_port)
  SCCACHE_PORT=$(free_port)
  RUN_DIR="$WORK/run_${ARM_NAME}_${REP}"
  mkdir -p "$RUN_DIR"
  BACKEND_LOG="$RUN_DIR/backend.log"

  # fresh fake backend
  python3 fake_webdav.py "$BACKEND_PORT" "$BACKEND_LOG" &
  BACKEND_PID=$!

  # daemon-scoped config: everything the daemon reads once at startup
  SCCACHE_DIR="$RUN_DIR/sccache-dir"
  export SCCACHE_DIR
  export SCCACHE_WEBDAV_ENDPOINT="http://127.0.0.1:$BACKEND_PORT/"
  # the fake has no PROPFIND; skip opendal's MKCOL-parent dance so cache
  # WRITES land like on the author's healthy-write backend
  export SCCACHE_WEBDAV_DISABLE_CREATE_DIR=true
  export SCCACHE_CACHE_READ_TIMEOUT_SECS=1
  export SCCACHE_SKIP_CACHE_CHECK=1
  export SCCACHE_IDLE_TIMEOUT=0
  export SCCACHE_SERVER_PORT=$SCCACHE_PORT
  # the candidate's knobs (demote arm sets THRESHOLD=0)
  SCCACHE_BACKEND_BACKOFF_THRESHOLD=${BACKOFF_THRESHOLD:-$THRESHOLD}
  export SCCACHE_BACKEND_BACKOFF_THRESHOLD
  export SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS=$COOLDOWN

  # fresh daemon, startup + retirement OUTSIDE the timed region
  "$BIN" --stop-server >/dev/null 2>&1
  if ! "$BIN" --start-server >/dev/null 2>&1; then
    echo "FAIL: --start-server ($ARM_NAME rep $REP)" >&2
    exit 4
  fi

  T0=$(now)
  i=0
  while [ "$i" -lt "$NREQ" ]; do
    "$BIN" cc -c "src/t$i.c" -o "obj/${ARM_NAME}${REP}_$i.o" || exit 5
    i=$((i + 1))
  done
  T1=$(now)
  WALL=$(elapsed $T0 $T1)

  # stats for this run (fresh daemon => fresh stats); JSON for exact counts
  "$BIN" --show-stats --stats-format json > "$RUN_DIR/show_stats.json" 2>"$RUN_DIR/show_stats.err"
  "$BIN" --show-stats > "$RUN_DIR/show_stats.txt" 2>/dev/null
  STATS=$(python3 -c "
import json
d = json.load(open('$RUN_DIR/show_stats.json'))
s = d.get('stats', d)
print('reqs=%s timeouts=%s writes=%s werrs=%s comps=%s' % (
    s.get('compile_requests'), s.get('cache_timeouts'), s.get('cache_writes'),
    s.get('cache_write_errors'), s.get('compilations')))
" 2>/dev/null || echo STATS_PARSE_FAILED)
  GETS=$(grep -c '^GET ' "$BACKEND_LOG" 2>/dev/null)
  PUTS=$(grep -c '^PUT ' "$BACKEND_LOG" 2>/dev/null)

  echo "run arm=$ARM_NAME rep=$REP wall=${WALL}s $STATS backend_gets=$GETS backend_puts=$PUTS"

  "$BIN" --stop-server >/dev/null 2>&1
  kill "$BACKEND_PID" 2>/dev/null
  wait_port_free "$SCCACHE_PORT" || true
  wait_port_free "$BACKEND_PORT" || true
}

# ------------------------------------------------------------------- driver
SEQ=0
BEFORE_REP=0
AFTER_REP=0

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
    one_run "$AFTER_BIN" demote "$SEQ"
  else
    BACKOFF_THRESHOLD=$THRESHOLD
    if [ "$a" = before ]; then
      BEFORE_REP=$((BEFORE_REP + 1))
      one_run "$BEFORE_BIN" before "$SEQ"
    else
      AFTER_REP=$((AFTER_REP + 1))
      one_run "$AFTER_BIN" after "$SEQ"
    fi
  fi
done

echo "done"
