# Plan 120 — Batch 140: the advisory-db sec seed (unbounded-read rule)

**Status:** IN PROGRESS — verdict YES B (Research 161); 1-2 suggest-only rules; `.raw/advisory-db` removed at the verdict commit — re-clone at the pin when this batch executes

- [ ] Re-clone `rustsec/advisory-db` @ `b50980aad8b8f14f77e25a97b32dd94bf008b0af` into `.raw/advisory-db` (CC0 verified at the verdict; re-verify HEAD == pin, else re-pin + note)
- [ ] Dedup re-verification at mine time: `unbounded.*read\|read_to_end\|take(limit` zero-hit + read the `length-prefix-validate-before-alloc` proposer to confirm the composition boundary (stream-growth vs alloc-from-length)
- [ ] Author `unbounded-read-into-vec` (InputValidation family, SUGGEST-ONLY): ORIGINAL fixtures — buggy: `read_to_end`/`loop { read?; extend }` into `Vec` from a network/untrusted-derived reader (TcpStream-child of a listener, reqwest body ident vocabulary), no cap; fixed: `.take(limit)` chain + `Vec::with_capacity(limit)`; sec_concept cites the h2 RUSTSEC-2024-0003 / CVE-2019-9514 class + Research 161
- [ ] Zero-FP negative suite: bounded twin (`take(N)` present ⇒ suppress), trusted-file reader (`File::open` on a literal path ⇒ suppress — the trusted regime), `with_capacity`-only (no read ⇒ no match), capacity-capped loop (`vec.resize(n, 0)` after a bound check ⇒ suppress)
- [ ] Optional second rule ONLY if the lead lands clean: `secret-buffer-not-zeroed` suggest-only sketch (C+ — dep-introducing caveat recorded in Research 161; owner may decline)
- [ ] Standing gates: sec lib count floor (14 → 15/16), rule_ids_unique_and_kebab, FIXABLE_SEC unchanged (suggest-only — collect_edits untouched), boundary real-compile gate over the grown probe crate (the B131 E0425 lesson — run BEFORE claiming fixture readiness)
- [ ] Retrieval floors: kernel_heal+latent byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7) + router annotation re-measure per the Issue-020 contract (sec arms moved at B131 — re-measure, record, never silently weaken)
- [ ] Clippy 0 (default + sec) + score-bench FLAT + `--corpus-stats` count walk (sec 14 → 15)
- [ ] Count pins walked: corpus.rs header + family table + AGENTS sec row + README + .docs/11_domains/sec.md + index
- [ ] Research 161 status flip: DISTILLED → RECORD (batch landed) + queue-snapshot line updated in the same commit
- [ ] `rm -rf .raw/advisory-db` at the landing tail + commit+push (develop, `feat:` prefix)

Defers:
- [-] zeroization rule if the lead's negative suite needs more than one iteration (reason: C+ grade, dep-introducing fix — decline is a correct answer)
- [-] taxonomy harvest of the remaining ~30 classes (reason: renewable oracle — re-mine on demand only if the lead proves retrieval value)
