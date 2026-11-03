Fixes #2975.

Writes a `CACHEDIR.TAG` ([spec](https://bford.info/cachedir/)) into the disk cache root on init, like cargo does for `target/`, so backup tools that honor it (GNU tar / borg / restic `--exclude-caches`) skip the cache automatically.

- Best-effort: if the write fails it logs a warning and cache creation continues.
- Skips the write when an identical tag already exists, so cache hits don't touch the directory mtime.
- The init scan ignores the root-level tag, so it never enters the LRU (where eviction would delete it) and isn't hit by the per-file size limit.

Two new tests cover the tag contents and that it survives re-opens and eviction. Ran `cargo test --lib`, the CI clippy invocation and `cargo fmt --check` on 1.91.0.

Happy to adjust.
