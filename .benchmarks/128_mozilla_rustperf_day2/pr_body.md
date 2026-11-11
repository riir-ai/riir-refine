As proposed in [this comment on #2968](https://github.com/mozilla/sccache/issues/2968#issuecomment-6710208564) — per-level gates plus a lookup deadline shared across levels, the shape confirmed on the thread as the right direction. When a cache backend stalls on every read, each cache miss pays the full read timeout before the compile proceeds locally, and those waits recur for every build the daemon serves and accumulate along a build's dependency graph. In a `disk,s3` chain the cost sits on every disk miss: one read timeout wraps the whole lookup, so after the fast local miss each lookup still waits on the stalled `s3`, and a single stalled level can consume the entire lookup budget before any later level is tried.

This evaluates the backoff per cache level. After `SCCACHE_BACKEND_BACKOFF_THRESHOLD` consecutive timed-out reads (default 3, `0` disables the skip), a level's reads are skipped for a cooldown window (`SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS`, default 60 — matched to the default read timeout, so a tripped level is re-probed about once per lookup interval) and reported the same way a timed-out lookup is, so builds proceed exactly as before, minus the wait. Any read the level answers (hit, miss or fast error) resets its counter. Recovery is single-probe: when the window expires, the next lookup probes the level and concurrent lookups keep skipping until it answers, so recovery never fires a burst of requests at a backend that may still be sick. An answered lookup reopens the level immediately — including a late in-flight read finishing during the cooldown — and a probe that times out re-arms the window; a probe deadline is a safety net, so a wedged probe state costs one extra probe, never a stuck level. The lookup deadline is shared too: each level is tried with what is left of the `SCCACHE_CACHE_READ_TIMEOUT_SECS` deadline divided by the levels remaining — every level is guaranteed at least about a 1/N slice, the last level gets whatever remains, and a level that answers quickly leaves nearly its whole share to the later ones — so one stalled level cannot consume the entire budget before the rest are probed. The tradeoff is real for a healthy but slow tier: a level whose normal answer time is longer than its share has its reads cut at the share, and after the threshold consecutive cut-offs its gate trips even though the backend is healthy; raising `SCCACHE_CACHE_READ_TIMEOUT_SECS` raises every level's share, which is the lever for chains with slow tiers. At `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0` the skip and the counting turn off: on a single backend that keeps the existing wall-clock shape (the wrapper applies the same read timeout the lookup applies today), while in a chain the deadline division stays, because it is what keeps a hung first level from starving the rest of the chain.

Writes are never skipped — a skipped put would silently drop a cache entry. A level that fails fast never trips its gate; the gate targets the waits described in the issue. On the daemon compile path, skipped and timed-out lookups still show up under `cache_timeouts` in `--show-stats`, since they share the timed-out lookup's reporting path; the observable difference is wall time and the number of requests the backend sees. The per-level counts are visible in the multilevel stats (`  L0 (LocalDisk) timeouts`-style rows). Client-side mode (`SCCACHE_CLIENT_SIDE=1`) serves its lookups through raw reads; those are bounded by the same deadline and trip the same per-level gate (measured cells below cover the daemon compile path — the client-side shape was not benched separately).

The knobs are documented in `docs/Configuration.md` next to `SCCACHE_CACHE_READ_TIMEOUT_SECS`, and the multi-level behavior in `docs/MultiLevel.md`.

@darshpatel1052 — you called dibs on this issue before I sketched the design; happy to fold this into your work or hand it over, whichever you prefer, and equally happy to hear it's useful as a starting point.

The workload below is sequential (32 distinct compiles one at a time), which isolates the backoff's effect. The issue's parallel shape is measured too: the reproducer workload from the issue (four independent chains of eight small libraries plus an executable, `cargo build -j 4`, WebDAV on loopback with GETs stalled while PUTs succeed, one daemon across the runs), at `SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS=10` so the recovery window lands inside the measurement:

| build | main `33d12fb` | backoff without single-probe recovery (an earlier iteration) | this patch |
|---|---|---|---|
| uncached control | 0.67 s | 0.66 s | 0.71 s |
| healthy warm | 0.48 s, 32 hits | 0.52 s, 32 hits | 0.52 s, 32 hits |
| stalled | 8.84 s, 32 timeouts | 1.80 s | 1.84 s |
| stalled again, same daemon | 8.85 s | 0.81 s | 0.81 s |
| 8 concurrent lookups fired at cooldown expiry | 8 GETs, continuously | 8 GETs at once | 1 GET |
| service restored, same daemon | 0.50 s, 32 hits | 0.54 s, 32 hits | 0.54 s, 29 hits + 3 skips |

The burst row is the recovery behavior this patch adds: at cooldown expiry the earlier shape admitted every waiting lookup at once (all 8 GETs of the wave), this patch admits exactly one probe and the rest keep skipping until it answers. On the recovered build the patch shows 29 hits and 3 timeouts — the three lookups that raced the probe at the start of the build are skipped once and reported through the timeout path; the wall time is unchanged. Under parallelism the saving compresses into fewer wall seconds, and correlated timeouts across in-flight lookups can trip a gate at once — the threshold and cooldown knobs are the levers for that ("avoid overreacting to transient slowness").

Two cells, both against a WebDAV backend whose GETs stop answering while PUTs succeed, with `SCCACHE_CACHE_READ_TIMEOUT_SECS=1`, at the repo's MSRV (rustc 1.91.0):

Standalone backend, 32 distinct compile requests through a fresh daemon per run (uncached control 0.5 s), medians of 3 alternating reps at main `33d12fb`:

- before: 33.5 s — 32 timeouts, 32 successful writes, 32 backend GETs
- after: 4.3 s — 32 timeouts reported, but 3 backend GETs; all 32 compiles and writes still succeed
- with `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0` (single run): 33.5 s, indistinguishable from before

Multi-level chain `disk,webdav`, 48 compiles per run: 16 keys warmed through the chain while the backend was healthy, then the same backend turns unhealthy mid-run — 3 cold keys (the gate trips on the third), then the 16 warm keys, then 29 more cold keys:

- before: 33.7 s — every cold lookup pays the 1 s wait on the stalled level; the 16 warm lookups keep hitting the disk tier throughout
- after: 4.6 s — the disk tier serves the 16 warm keys while the remote level is skipped; 3 backend GETs during the outage instead of the before-arm's 32 stalled GETs; the cold compiles all still run and write
- with `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0` (single run): 34.2 s, indistinguishable from before

The win in one line: 29 of the 32 read-timeout waits avoided per workload, with the disk tier unaffected. The ratio depends on the build graph and timeout; for scale, this issue measured a stalled build at 11.7x a healthy warm one.

This implements the per-level option from the comment; happy to collapse to a single global gate if that's preferred. The CI clippy invocation (from `ci.yml`) is clean on the branch, `cargo fmt --check` is clean, and `cargo check --no-default-features` shows only warnings that are already on unmodified main. Lib tests: 657 passed with 19 new tests for the module and the multi-level chain; the failing tests in the lib and integration suites (the unix-socket binding pair, the cache-hit-rate suite, the sccache_cargo suite) fail identically on unmodified main at this pin and are environment-dependent on this machine.
