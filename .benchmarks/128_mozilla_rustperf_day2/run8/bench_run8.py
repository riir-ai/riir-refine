#!/usr/bin/env python3
"""run-8 bench: the #2968 maintainer reproducer shape, three arms, burst metric.

Adapted from mitander's reproduce.py (sccache issue #2968 attachment 34127144)
with the macOS-required changes (no /proc process discovery, no platform gate)
and the maintainer-reply additions:

  - phases extended in the SAME daemon: stalled, stalled-repeat, burst wave,
    recovered (GET service restored) — his table shape plus the recovery leg
    he asked for;
  - a burst metric: concurrent cache lookups fired at cooldown expiry, counted
    from per-request fixture timestamps (the "no burst of concurrent probes"
    ask);
  - SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS=10 (short, recorded; the run-7 cells
    keep the 60 s default).

usage: bench_run8.py <sccache-bin> <arm-name> <work-dir(new)> <bins-dir>
"""
import argparse
import http.server
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

COOLDOWN = "10"
THRESHOLD = "3"
WAVE = 8  # concurrent lookups in the burst wave


class Cache(http.server.ThreadingHTTPServer):
    daemon_threads = False

    def __init__(self) -> None:
        self.objects: dict[str, bytes] = {}
        self.lock = threading.Lock()
        self.stall = False
        self.release = threading.Event()
        self.stalled_gets = 0
        self.get_times: list[float] = []  # monotonic timestamp per GET
        super().__init__(("127.0.0.1", 0), Handler)


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    def reply(self, status: int, body: bytes = b"") -> None:
        self.send_response(status)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        cache = self.server
        if not isinstance(cache, Cache):
            raise RuntimeError("unexpected HTTP fixture")

        with cache.lock:
            stall = cache.stall
            if stall:
                cache.stalled_gets += 1
            cache.get_times.append(time.monotonic())
            body = cache.objects.get(self.path)

        if stall:
            cache.release.wait()
        else:
            self.reply(404 if body is None else 200, body or b"")

    def do_PUT(self) -> None:
        cache = self.server
        if not isinstance(cache, Cache):
            raise RuntimeError("unexpected HTTP fixture")
        size = int(self.headers.get("Content-Length", "-1"))
        if self.headers.get("Transfer-Encoding") or not 0 <= size <= 2 * 1024 * 1024:
            self.reply(400)
            return
        body = self.rfile.read(size)
        if len(body) != size:
            self.reply(400)
            return
        with cache.lock:
            cache.objects[self.path] = body
        self.reply(201)


def command(argv, root, env, log):
    with (root / f"{log}.log").open("wb") as output:
        process = subprocess.Popen(
            argv, cwd=root, env=env, stdout=output, stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        try:
            code = process.wait(timeout=90)
        except BaseException:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
            raise
    if code:
        raise RuntimeError(f"{log} failed, see {root / (log + '.log')}")


def prepare(root: Path) -> None:
    # mitander's prepare() verbatim: four lanes of eight dependent crates.
    tips = []
    for lane in range(4):
        for level in range(8):
            name = f"lane{lane}_level{level}"
            crate = root / name
            (crate / "src").mkdir(parents=True)
            dependency = ""
            value = "1"
            if level:
                previous = f"lane{lane}_level{level - 1}"
                dependency = f'{previous} = {{ path = "../{previous}" }}\n'
                value = f"{previous}::value() + 1"
            (crate / "Cargo.toml").write_text(
                f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n'
                f"[dependencies]\n{dependency}"
            )
            (crate / "src/lib.rs").write_text(f"pub fn value() -> u32 {{ {value} }}\n")
        tips.append(f"lane{lane}_level7")

    (root / "src").mkdir()
    (root / "src/main.rs").write_text(
        'fn main() { println!("{}", '
        + " + ".join(f"{tip}::value()" for tip in tips)
        + "); }\n"
    )
    (root / "Cargo.toml").write_text(
        '[package]\nname = "timeout_repro"\nversion = "0.1.0"\nedition = "2021"\n'
        + "[dependencies]\n"
        + "".join(f'{tip} = {{ path = "{tip}" }}\n' for tip in tips)
        + '[workspace]\nmembers = ["lane*"]\n'
    )


def statistics(binary, root, env, phase):
    deadline = time.monotonic() + 10
    while True:
        command([binary, "--show-stats", "--stats-format", "json"], root, env, phase + "-stats")
        stats = json.loads((root / f"{phase}-stats.log").read_text())["stats"]
        hits = sum(stats["cache_hits"]["counts"].values())
        misses = sum(stats["cache_misses"]["counts"].values())
        writes = stats["cache_writes"]
        errors = stats["cache_write_errors"]
        timeouts = stats["cache_timeouts"]
        if errors:
            raise RuntimeError(f"{phase}: cache write failed")
        if writes >= misses and (hits or misses or phase in ("stalled", "stalled-repeat")):
            return hits, timeouts
        if time.monotonic() >= deadline:
            return hits, timeouts
        time.sleep(0.05)


def build(root, binary, env, phase):
    """One cargo build of the 4x8 workspace; returns (wall, hits, timeouts)."""
    target = root / "target"
    if target.exists():
        shutil.rmtree(target)
    build_env = env.copy()
    build_env["RUSTC_WRAPPER"] = binary
    command([binary, "--zero-stats"], root, env, phase + "-zero")
    started = time.monotonic()
    command(["cargo", "build", "--locked", "--offline", "-j", "4"], root, build_env, phase)
    hits, timeouts = statistics(binary, root, env, phase)
    elapsed = time.monotonic() - started
    executable = target / "debug/timeout_repro"
    if subprocess.check_output([str(executable)], timeout=10) != b"32\n":
        raise RuntimeError(f"{phase}: executable did not print 32")
    return elapsed, hits, timeouts


def wave(root, binary, env, name):
    """Fire WAVE concurrent sccache'd cc lookups; returns wall of the wave."""
    src = root / "wave_src"
    src.mkdir(exist_ok=True)
    obj = root / "wave_obj"
    obj.mkdir(exist_ok=True)
    for i in range(WAVE):
        (src / f"w{i}.c").write_text(f"int f{i}(int x) {{ return x + {i}; }}\n")
    procs = []
    started = time.monotonic()
    for i in range(WAVE):
        argv = [binary, "cc", "-c", str(src / f"w{i}.c"), "-o", str(obj / f"w{i}.o")]
        procs.append(subprocess.Popen(argv, cwd=root, env=env,
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
    for p in procs:
        p.wait(timeout=60)
    return time.monotonic() - started


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sccache", type=Path, required=True)
    parser.add_argument("--arm", required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()

    binary = args.sccache.resolve()
    if binary.name != "sccache" or not binary.is_file():
        parser.error("the binary must be named sccache (copy/symlink it)")
    root = args.work_dir.resolve()
    root.mkdir(parents=True)
    prepare(root)

    env = {k: v for k, v in os.environ.items() if not k.startswith("SCCACHE_")}
    for key in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS",
                "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    env.update(
        CARGO_INCREMENTAL="0",
        CARGO_TARGET_DIR=str(root / "target"),
        NO_PROXY="127.0.0.1",
    )

    socket_dir = Path(tempfile.mkdtemp(prefix="r8-"))
    (socket_dir / "config").write_text("")
    env.update(
        SCCACHE_SERVER_UDS=str(socket_dir / "s"),
        SCCACHE_CONF=str(socket_dir / "config"),
        SCCACHE_CACHED_CONF=str(socket_dir / "cached-config"),
        SCCACHE_DIR=str(socket_dir / "disk-cache"),
        SCCACHE_CACHE_READ_TIMEOUT_SECS="1",
        SCCACHE_SKIP_CACHE_CHECK="1",
        SCCACHE_IDLE_TIMEOUT="0",
        SCCACHE_WEBDAV_RW_MODE="READ_WRITE",
        SCCACHE_WEBDAV_KEY_PREFIX="repro",
        SCCACHE_WEBDAV_DISABLE_CREATE_DIR="true",
        SCCACHE_BACKEND_BACKOFF_THRESHOLD=THRESHOLD,
        SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS=COOLDOWN,
    )

    command([binary, "--version"], root, env, "sccache-version")
    command(["rustc", "-vV"], root, env, "rustc")
    command(["cargo", "generate-lockfile", "--offline"], root, env, "lockfile")

    cache = Cache()
    thread = threading.Thread(target=cache.serve_forever, daemon=True)
    thread.start()
    env["SCCACHE_WEBDAV_ENDPOINT"] = f"http://127.0.0.1:{cache.server_port}"

    # the daemon reads its config at startup: start it AFTER the endpoint is set
    command([binary, "--start-server"], root, env, "start")

    print(f"arm={args.arm} cooldown={COOLDOWN} threshold={THRESHOLD} wave={WAVE}", flush=True)
    print(f"{'phase':16} {'Seconds':>9} {'Hits':>6} {'Timeouts':>9} {'HeldGETs':>9}", flush=True)

    def held():
        with cache.lock:
            return cache.stalled_gets

    def set_stall(stalled: bool) -> None:
        with cache.lock:
            cache.stall = stalled

    try:
        # uncached control
        target = root / "target"
        if target.exists():
            shutil.rmtree(target)
        started = time.monotonic()
        command(["cargo", "build", "--locked", "--offline", "-j", "4"], root, env, "uncached")
        uncached_wall = time.monotonic() - started
        print(f"{'uncached':16} {uncached_wall:9.3f} {0:6d} {0:9d} {held():9d}", flush=True)

        # populate (healthy) — writes land, no hits expected
        set_stall(False)
        w, h, to = build(root, binary, env, "populate")
        print(f"{'populate':16} {w:9.3f} {h:6d} {to:9d} {held():9d}", flush=True)

        # healthy warm — 32 hits expected
        set_stall(False)
        w, h, to = build(root, binary, env, "healthy-warm")
        print(f"{'healthy-warm':16} {w:9.3f} {h:6d} {to:9d} {held():9d}", flush=True)

        # stalled — gate trips mid-build on the gated arms
        set_stall(True)
        w, h, to = build(root, binary, env, "stalled")
        print(f"{'stalled':16} {w:9.3f} {h:6d} {to:9d} {held():9d}", flush=True)

        # stalled-repeat, same daemon — the gated arms skip (fast); pristine pays again
        set_stall(True)
        w, h, to = build(root, binary, env, "stalled-repeat")
        print(f"{'stalled-repeat':16} {w:9.3f} {h:6d} {to:9d} {held():9d}", flush=True)

        # idle past the first cooldown expiry so no probe is mid-flight
        time.sleep(int(COOLDOWN) + 5)

        # burst wave: WAVE concurrent lookups at cooldown expiry, backend stalled
        w = wave(root, binary, env, "burst")
        with cache.lock:
            times = list(cache.get_times)
            n = len(times)
        # the wave's GETs are the last ones; count those within 100 ms of the first
        burst = 0
        if times:
            # walk back to the first GET of this wave: GETs before the sleep
            # are older than the sleep window
            cutoff = time.monotonic() - (w + 0.5)
            wave_times = [t for t in times if t >= cutoff]
            if wave_times:
                t0 = wave_times[0]
                burst = sum(1 for t in wave_times if t <= t0 + 0.1)
        print(f"{'burst-wave':16} {w:9.3f} {'-':>6} {'-':>9} {held():9d}  GETs_at_wave={burst}", flush=True)

        # restore GET service, wait past the re-opened cooldown, recover
        set_stall(False)
        time.sleep(int(COOLDOWN) + 2)
        w, h, to = build(root, binary, env, "recovered")
        print(f"{'recovered':16} {w:9.3f} {h:6d} {to:9d} {held():9d}", flush=True)

        # final JSON line for the record
        with cache.lock:
            total_gets = len(cache.get_times)
        print(json.dumps({
            "arm": args.arm, "uncached": uncached_wall, "burst_gets": burst,
            "total_gets": total_gets, "held_gets": held(),
            "get_times": times,
        }))
    finally:
        try:
            command([binary, "--stop-server"], root, env, "stop")
        except Exception:
            pass
        cache.release.set()
        cache.shutdown()
        cache.server_close()
        thread.join(timeout=5)


if __name__ == "__main__":
    main()
