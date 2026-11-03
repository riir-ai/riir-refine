Thanks for the quick review! Fixed all four: `fs::read`, a shorter comment, skipping any CACHEDIR.TAG (with a nested-cache test), and the redundant assert. Rebased on main.

The `--locked` failures are on main too: Cargo.lock points zip 9.0.5's `typed-path = "0.12"` at 0.13.0. Carried the one-entry lock fix (keeping zip 9.0.5) as a second commit here.

The freebsd `Unknown preprocessor cache entry format 53` failure also reproduces on main, so I left it alone.
