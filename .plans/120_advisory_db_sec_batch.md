# Plan 120 — Batch 140: the advisory-db sec seed (unbounded-read rule)

**Status:** LANDED 2026-09-09 — 1 suggest-only rule (`unbounded-read-into-vec`,
InputValidation); the optional zeroization second rule DECLINED per the plan's
own defer note (C+ grade, dep-introducing fix — decline is a correct answer).

- [x] Re-clone `rustsec/advisory-db` @ `b50980aad8b8f14f77e25a97b32dd94bf008b0af` into `.raw/advisory-db` (CC0 verified at the verdict; HEAD == pin verified)
- [x] Dedup re-verification at mine time: `unbounded.*read\|read_to_end\|take(limit` zero-hit in `src/domains/sec/` + `length-prefix-validate-before-alloc` proposer read — the composition boundary holds (stream-growth ≠ alloc-from-length)
- [x] Author `unbounded-read-into-vec` (InputValidation family, SUGGEST-ONLY): ORIGINAL fixtures — buggy: UFCS `Read::read_to_end(&mut stream, &mut buf)` from a TcpStream-typed param, no cap; fixed: `Read::take(stream, max_bytes)` binding + `Vec::with_capacity(..)`; sec_concept cites RUSTSEC-2024-0003 / CVE-2019-9514 + Research 161
- [x] Zero-FP negative suite (18 tests, `batch140_*`): take-chain bounded twin, take-binding UFCS form, File trusted regime, unknown slice reader, with_capacity-without-read, resize-capped drain, buf.len() cap, read-count vs non-zero const cap, EOF `n == 0` does-not-suppress (pinned both ways), R: Read honest NO, cfg(test) skip, span slicing
- [-] Optional second rule (`secret-buffer-not-zeroed`) — DECLINED: C+ grade + dep-introducing (zeroize) caveat recorded in Research 161; the defer note anticipated this
- [x] Standing gates: sec lib 1290/0 (floor walked; batch140 detector suite 18/18), rule_ids_unique_and_kebab (corpus test), FIXABLE_SEC unchanged (suggest-only — collect_edits untouched), boundary real-compile gate 2/2 (the E0425-class lesson fired: the first buggy fixture was E0596-broken and the unwrap_used pin red'd the vacuity BEFORE fixture readiness could be claimed)
- [x] Retrieval floors: blended 93.1/96.0/97.0 of 101 (floors 93/95/97 held byte-identical class), kernel 30/30/30 untouched, sec self-variant top-1 holds, router annotation re-measured per Issue-020: sec+docker 34/101 (was 35; floor 30→29/101) + full state 83/131 (was 79, +4 UP; floor 74→78/131) — both re-pinned with deltas recorded, neither weakened
- [x] Clippy 0 (default + sec) + score-bench #87 FLAT 96.2/1.9 + `--corpus-stats` sec 14 → 15 (en-route develop fix: `ce74bac`'s healer redundant-closure rewrite of the score-bench filter broke the `&String→&str` deref-coercion and compiled to nothing under default features — restored, verified by the score-bench run itself)
- [x] Count pins walked: corpus.rs header table + batch note, AGENTS sec row ×2, README ×2, this file, `.docs/11_domains/sec.md` batch section + index
- [x] Research 161 status flip: DISTILLED → RECORD (batch landed) + queue-snapshot line updated in the same commit
- [x] `rm -rf .raw/advisory-db` at the landing tail + commit+push (develop, `feat:` prefix)

Defers:
- [-] zeroization rule if the lead's negative suite needs more than one iteration (reason: C+ grade, dep-introducing fix — decline is a correct answer)
- [-] taxonomy harvest of the remaining ~30 classes (reason: renewable oracle — re-mine on demand only if the lead proves retrieval value)
