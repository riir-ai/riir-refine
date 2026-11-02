Write a cache-directory tag (`CACHEDIR.TAG`) into the disk cache root when
it is created, per the [Cache Directory Tagging Specification]
(https://bford.info/cachedir/), so backup tools that honor the spec (rsync
with an exclude plugin, borg, restic, timeshift, …) can skip the cache
automatically — the same way cargo already tags `target/`.

## What

- `LruDiskCache::init` writes the canonical tag file into the cache root
  right after creating it.
- The write is best-effort: an unwritable cache directory logs a warning
  and continues — cache creation never fails because of the tag.
- On later runs the write is skipped when the existing tag already matches,
  so plain cache hits do not touch the directory's mtime.
- The startup scan now skips `CACHEDIR.TAG`: it is metadata for backup
  tools, not a cache entry, so it must not enter the LRU (eviction would
  otherwise delete it once the cache is full) and must not be subject to
  the per-file size limit (a tiny max-size cache would otherwise remove it
  at init).

## Why

Fixes #2975. The cache directory can grow to tens of GB, and today every
backup that sweeps `$HOME` copies all of it unless the user hand-configures
an exclude. The tag is the standard, zero-config mechanism for that, and it
is a no-op for tools that ignore it.

## How verified

- `cargo +1.91.0 test -p sccache --lib` — 598 passed, 0 failed (including
  two new tests: tag content matches the spec's signature line; the tag is
  not counted as a cache entry, survives re-opens and forced eviction).
- `cargo +1.91.0 clippy --workspace --all-targets --locked -- -D warnings
  -A unknown-lints -A clippy::type_complexity -A clippy::new-without-default`
  (the CI gate) — zero warnings.
- `cargo +1.91.0 fmt -- --check` — clean.
- MSRV note: the code is `const`/`b"..."`/generics-free-of-new-features;
  `rust-version = "1.91.0"` is untouched and the gates above ran pinned at
  1.91.0.

Happy to rework or close.
