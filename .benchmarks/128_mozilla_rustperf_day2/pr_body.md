Addresses #2968.

The issue asked about the preferred behavior before an implementation; this is one concrete shape to discuss. Happy to change it or hand it over.

When a cache backend stalls on every lookup, each compilation pays the full read timeout before falling back to a local compile. Those waits accumulate along the dependency graph and recur in every build served by the same daemon session, which is the cost described in the issue.

This adds a daemon-scoped backoff at the cache-read seam. After 3 consecutive timed-out lookups, cache reads are skipped for a 60-second window and reported the same way a timed-out lookup is, so builds proceed exactly as before, minus the wait. Any lookup the backend answers (hit, miss or fast error) resets the count. Once the window expires, later lookups probe the backend again; a probe that times out re-arms the window. A parallel build can have several probes in flight, so one window can still pay up to one timeout per already-running lookup.

Two knobs, read once at first use:

- `SCCACHE_BACKEND_BACKOFF_THRESHOLD`, consecutive timeouts to trip (default 3, `0` disables)
- `SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS`, skip window (default 60)

The backoff is on by default. If a default-on circuit breaker is too strong a posture for main, making it opt-in is a small flip and I'm happy to do it. I also kept it global rather than per-backend; splitting the state per backend is doable if that's preferred.

Scope notes: only the read path is gated. Cache writes are never skipped, since a skipped put would silently drop a cache entry. A backend that fails fast never trips the gate either; the gate targets the waits described in the issue. Skipped lookups still show up under `cache_timeouts` in `--show-stats`, since they share the same miss path; the observable difference is wall time and the number of requests the backend sees.

Measured on this machine at the repo's MSRV (rustc 1.91.0), against a WebDAV backend whose GETs never answer while PUTs succeed, with `SCCACHE_CACHE_READ_TIMEOUT_SECS=1` and 32 distinct compile requests through a fresh daemon per run (uncached control: 0.590 s):

- before (main at ba261b8): median 33.527 s, 32 timeouts, 32 successful writes, 32 backend GETs
- after: median 4.409 s, 3 backend GETs, all 32 compiles and writes still succeed
- with `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0`: 33.528 s, indistinguishable from before

The win in one line: 29 of the 32 read-timeout waits avoided, about 29 seconds saved on this workload. The ratio depends on the build graph and timeout; for scale, this issue measured a stalled build at 11.7x a healthy warm one.

The CI clippy invocation (from `ci.yml`) is clean on the branch, `cargo fmt --check` is clean, and `cargo check --no-default-features` passes. Lib tests: 632 passed with 8 new tests for the module; the 3 failures in the lib suite are environment-dependent on this macOS machine and fail identically on unmodified main.
