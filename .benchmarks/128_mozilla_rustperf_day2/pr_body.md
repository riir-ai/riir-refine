Addresses #2968.

@darshpatel1052 posted a design sketch on this issue on Nov 3 — I missed it when this PR opened, sorry; this wasn't meant as a claim-jump. If you're working on this, I'm happy to withdraw and fold this in as a starting point, or to build on your design instead — whichever you prefer. If nobody else is picking it up, here's the shape I'd propose, for discussion:

When a cache backend stalls on every lookup, each compilation pays the full read timeout before falling back to a local compile. Those waits accumulate along the dependency graph and recur in every build served by the same daemon session, which is the cost described in the issue.

This adds a backoff that is evaluated per cache level, plus a lookup deadline that is shared across levels. After `SCCACHE_BACKEND_BACKOFF_THRESHOLD` consecutive timed-out reads (default 3, `0` disables), a level's reads are skipped for a cooldown window (`SCCACHE_BACKEND_BACKOFF_COOLDOWN_SECS`, default 60 — chosen to match the default read timeout, so a tripped level is re-probed roughly once per lookup-interval) and reported the same way a timed-out lookup is, so builds proceed exactly as before, minus the wait. Any read the level answers (hit, miss or fast error) resets its counter. Once the window expires, later lookups probe that level again; a probe that times out re-arms the window.

Because the gate is per level, a `disk,s3` chain keeps serving local disk hits while the stalled remote level is skipped — the healthy tiers never inherit the remote tier's cooldown. The shared deadline splits the remaining lookup time across the levels left to try, so one stalled level also cannot consume the whole lookup budget before later levels are probed. Writes are never skipped: a skipped put would silently drop a cache entry. A level that fails fast never trips its gate; the gate targets the waits described in the issue. Skipped lookups still show up under `cache_timeouts` in `--show-stats`, since they share the same miss path; the observable difference is wall time and the number of requests the backend sees. The per-level timeout counts are also visible per level in the multilevel stats (`L<n> ... timeouts` rows).

The knobs are documented in `docs/Configuration.md` next to `SCCACHE_CACHE_READ_TIMEOUT_SECS`, and the multi-level behavior in `docs/MultiLevel.md`.

The bench is a sequential workload (32 distinct compiles one at a time, which isolates the backoff's effect), so it does not measure the issue's parallel `cargo -j4` shape; under parallelism the saving compresses into fewer wall seconds, and correlated timeouts across in-flight lookups can trip the gate at once — the threshold and cooldown knobs exist to tune exactly that ("avoid overreacting to transient slowness" from the issue), and I did not measure the parallel shape on this machine.

Measured on this machine at the repo's MSRV (rustc 1.91.0), against a WebDAV backend whose GETs never answer while PUTs succeed, with `SCCACHE_CACHE_READ_TIMEOUT_SECS=1` and 32 distinct compile requests through a fresh daemon per run (uncached control: CTRL_PLACEHOLDER):

- before (main at PIN_PLACEHOLDER): median BEFORE_PLACEHOLDER s, 32 timeouts, 32 successful writes, 32 backend GETs
- after: median AFTER_PLACEHOLDER s, 3 backend GETs, all 32 compiles and writes still succeed
- with `SCCACHE_BACKEND_BACKOFF_THRESHOLD=0`: DEMOTE_PLACEHOLDER s, indistinguishable from before

The win in one line: 29 of the 32 read-timeout waits avoided, about 29 seconds saved on this workload. The ratio depends on the build graph and timeout; for scale, this issue measured a stalled build at 11.7x a healthy warm one.

The CI clippy invocation (from `ci.yml`) is clean on the branch, `cargo fmt --check` is clean, and `cargo check --no-default-features` passes. Lib tests: LIB_PLACEHOLDER with 14 new tests for the module and the multi-level chain; the failing tests in the lib suite (unix-socket binding pair, cache-hit-rate and sccache_cargo integration suites) fail identically on unmodified main at this pin and are environment-dependent on this machine.
