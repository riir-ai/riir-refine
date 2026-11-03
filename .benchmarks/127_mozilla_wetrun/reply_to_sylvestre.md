Good catch on the preprocessor cache, thanks for the quick review.

Pushed f767469 — all four addressed:
- used `fs::read`, dropped the open/read dance
- skip any file named CACHEDIR.TAG (shortened the comment too)
- nested-cache test: builds a real `LruDiskCache` under `<root>/preprocessor`, fills the outer cache until eviction kicks in, checks both tags survive
- removed the redundant assert

Rebased on main (329fa19).

On the red CI: the `--locked` failures are actually on main as well — the lockfile has zip 9.0.5 pulling `typed-path` bare but only lists 0.13.0, while zip's manifest wants `= "0.12"`, so any locked build refuses to start. Added the missing 0.12.4 entry as the second commit (kept zip at 9.0.5). Might be worth a follow-up on main since every PR hitting `--locked` will trip over it.

The freebsd `Unknown preprocessor cache entry format 53` one also fails on a clean main checkout, so I left it alone — looks like it came in with the preprocessor keying rework, not this patch.
