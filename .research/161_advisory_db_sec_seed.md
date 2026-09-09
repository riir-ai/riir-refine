# Research 161 — rustsec/advisory-db as a sec-domain seed source (Batch 140 candidate)

**Status:** DISTILLED — pending owner decision (verdict YES B; 1-2 suggest-only rules; plan 120 opened)

- **Source:** `rustsec/advisory-db` @ `b50980aad8b8f14f77e25a97b32dd94bf008b0af` (main, cloned 2026-09-09; **CC0 1.0 / public domain** — LICENSE.txt verified, the strongest possible corpus-seed license; fixtures remain ORIGINAL per the standing constraint, the DB is cited as class provenance only).
- **Inspection:** 2026-09-09, M3 box. Full shallow clone; 1,223 crate advisories counted; category + mechanism-keyword distributions computed over the whole tree; 4 advisories read in full (lru 2026-0253, h2 2024-0003, bcrypt 2026-0199, blurhash 2023-0083).
- **Provenance chain:** each advisory carries CVE/GHSA aliases + patched-version predicates; the FIX DIFFS are NOT in this repo (they live in the affected crates' repos) — the DB is a class TAXONOMY with real-world frequency data, not a fix corpus. This is the load-bearing caveat: every rule mined here cites the advisory class, not a code shape.

## What the data says (measured, whole tree)

Categories (top): memory-corruption 271 · **denial-of-service 126** · memory-exposure 88 · crypto-failure 81 · malicious 75 · thread-safety 64 · code-execution 37. Informational split: 206 unsound, 270 unmaintained. Recurring mechanisms: Send/Sync 78 · uninit/MaybeUninit 85 · use-after-free 67 · set_len/unchecked 45 · deserialization 25 · double-free 29 · panic-in-Drop 12-18 · uncontrolled recursion 17 · constant-time 17 · symlink 12 · path traversal 9.

## The verdict reasoning (why DATA-source changes the shape)

The unit of value here is the **class taxonomy with real-world frequency** — an oracle for WHICH security patterns are worth detecting, not a source of code shapes. Fixtures must be original syntheses of the CLASS (as with every batch); the DB contributes: the class name, a CVE-anchored real-world instance, frequency (impact proxy), and the patched-version predicate. Sec-domain GOAT shape applies (parse detector + zero-FP negative suite + boundary real-compile gate; Bench 046/131 precedent) — no perf axis exists for these.

## Dedup gate (both vocabularies, over riir-clippy/src)

| Advisory class | Greps | Result |
|---|---|---|
| Unbounded read/growth from untrusted stream (DoS 126-class; h2/CVE-2019-9514 shape) | `unbounded.*read\|read_to_end\|take(limit`; corpus `length-prefix` | **NOVEL** — `length-prefix-validate-before-alloc` covers alloc-from-attacker-LENGTH only; stream-GROWTH is a different facet, composes with it in the InputValidation family |
| Secret buffer not zeroed on drop (memory-exposure 88 + zeroize mentions) | `zeroiz\|sensitive.*zero` | **NOVEL** but fix introduces a third-party dep (zeroize crate) — the sharded-vec drop precedent rejected dep-introducing fixes; suggest-only at best, weak |
| Path traversal / zip-slip in archive joins (9 advisories) | `traversal\|zip.?slip` | NOVEL but detector is taint-heuristic-heavy, fix-space fuzzy — weak |
| Panic-in-Drop / FFI-unwind / Send-Sync-unsound / ct-compare / length-alloc | — | **COVERED** by existing rules 5× (`drop-impl-no-panic`, `ffi-extern-fn-catch-unwind`, `unsafe-impl-send-sync-unsound-field`, `constant-time-secret-compare`, `length-prefix-validate-before-alloc`) — positive instances, dropped |
| Recursion without depth limit / set_len / uninit / symlink | — | decision-content FAILS (fix-space unbounded or detector beyond parse-provable) — dropped |

## Ranked candidates

| Pattern | Grade | Notes |
|---|---|---|
| `unbounded-read-into-vec` (InputValidation family) | **A− (batch lead)** | `read_to_end`/`loop { read }` into a growable Vec from a network/untrusted-derived reader with no cap → suggest `Read::take(limit)` (signature-preserving, mechanical); suggest-only because "untrusted" is a taint judgment. Real-world anchor: h2 RUSTSEC-2024-0003 / CVE-2019-9514 resource-exhaustion class |
| zeroization-on-drop for secret buffers | C+ | 88-advisory family, but dep-introducing fix; suggest-only; hold for a future batch unless the lead proves retrieval value |
| panic-on-crafted-parse (bcrypt/blurhash class) | DROP for corpus | largest real-world class but fix-space unbounded (parser rewrite); recorded as the honest non-goal |
| path-traversal joins | DROP | taint-tracked joins beyond parse-provable |

## GOAT four-test (the lead)

Benchable in the sec-GOAT shape (parse detector + zero-FP negative suite incl. the bounded twin the way Girder gave both-ways evidence; boundary real-compile gate) · parse-level deterministic · security-magnitude stated as such (OOM DoS, not cycles) · exact-once semantics n/a. PASSES.

## Caveats

DATA source, not code — every fixture is an original synthesis; the DB's fix-diffs are absent (fix-space derived from the advisory text + std API shape). Yield honestly 1-2 rules (suggest-only). The `~30` unharvested classes are a RENEWABLE oracle — re-mine on demand when the sec corpus wants its next families; category frequencies re-computed at mine time (DB moves daily). Three stale `.raw/` entries observed (batch118, ldg_probe, veloxquant-mlx — possible sibling lanes, left untouched per the shared-worktree rule).

## Queue slot

Batch 140 candidate, plan `120_advisory_db_sec_batch.md` (plan-first). `.raw/advisory-db` removed at the verdict commit; re-clone at the pinned sha when the batch executes.
