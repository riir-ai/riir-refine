Addresses #2968.

As proposed in the design comment upthread — per-level gates plus a lookup deadline shared across levels. When a cache backend stalls on every lookup, each compilation pays the full read timeout before falling back to a local compile, and those waits recur for every build the daemon serves. In a `disk,s3` chain they also hit healthy tiers: one read timeout wraps the whole lookup, so a stalled `s3` costs every lookup its wait even when the local disk would have answered, and a gate at that outer seam cannot tell which level stalled.

This evaluates the backoff per cache level. After `SCCACHE_BACKEND_BACKOFF_THRESHOLD` consecutive timed-out reads (default 3, `0` disables), a level's reads are skipped for a cooldown window (`SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS`, default 60 — matched to the default read timeout, so a tripped level is re-probed about once per lookup interval) and reported the same way a timed-out lookup is, so builds proceed exactly as before, minus the wait. Any read the level answers (hit, miss or fast error) resets its counter. Once the window expires, later lookups probe that level again; a probe that times out re-arms the window. The lookup deadline is shared too: each level is tried with its share of the remaining `SCCACHE_CACHE_READ_TIMEOUT_SECS` time, so one stalled level cannot consume the whole budget before later levels are probed (and a hung first level no longer starves the rest of the chain). Because the gate is per level, a `disk,s3` chain keeps serving local disk hits while the stalled remote level is skipped.

Writes are never skipped — a skipped put would silently drop a cache entry. A level that fails fast never trips its gate; the gate targets the waits described in the issue. Skipped and timed-out lookups still show up under `cache_timeouts` in `--show-stats`, since they share the timed-out lookup's reporting path; the observable difference is wall time and the number of requests the backend sees. The per-level counts are visible in the multilevel stats (`L<n> ... timeouts` rows).

The knobs are documented in `docs/Configuration.md` next to `SCCACHE_CACHE_READ_TIMEOUT_SECS`, and the multi-level behavior in `docs/MultiLevel.md`.

The workload below is sequential (32 distinct compiles one at a time), which isolates the backoff's effect; the issue's parallel `cargo -j4` shape was not measured on this machine. Under parallelism the saving compresses into fewer wall seconds, and correlated timeouts across in-flight lookups can trip a gate at once — the threshold and cooldown knobs are the levers for that ("avoid overreacting to transient slowness").

Two cells, both against a WebDAV backend whose GETs stop answering while PUTs succeed, with `SCCACHE_CACHE_READ_TIMEOUT_SECS=1`, at the repo's MSRV (rustc 1.91.0):

Standalone backend, 32 distinct compile requests through a fresh daemon per run (uncached control 0.7 s), medians of 3 alternating reps at pin `33d12fb`:

- before: 34.2 s — 32 timeouts, 32 successful writes, 32 backend GETs
- after: 6.0 s — 32 timeouts reported, but 3 backend GETs; all 32 compiles and writes still succeed
- with `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0`: 35.4 s, indistinguishable from before

Multi-level chain `disk,webdav`, 48 compiles per run: 16 keys warmed through the chain while the backend was healthy, then the same backend turns unhealthy mid-run — 3 cold keys (the gate trips on the third), then the 16 warm keys, then 29 more cold keys:

- before: 33.8 s — every cold lookup pays the 1 s wait; the 16 warm lookups keep hitting the disk tier throughout
- after: 4.7 s — the disk tier serves the 16 warm keys while the remote level is skipped; 3 backend GETs instead of 32; the cold compiles all still run and write
- with `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0`: 33.8 s, indistinguishable from before

The win in one line: 29 of the 32 read-timeout waits avoided per workload, and in the chain the local tier keeps serving through the remote's outage. The ratio depends on the build graph and timeout; for scale, this issue measured a stalled build at 11.7x a healthy warm one.

The CI clippy invocation (from `ci.yml`) is clean on the branch, `cargo fmt --check` is clean, and `cargo check --no-default-features` shows only warnings that are already on unmodified main. Lib tests: 652 passed with 14 new tests for the module and the multi-level chain; the failing tests in the lib and integration suites (the unix-socket binding pair, the cache-hit-rate suite, the sccache_cargo suite) fail identically on unmodified main at this pin and are environment-dependent on this machine.
