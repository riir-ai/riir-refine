# Plan 296 — Batch 292: CVEngine sec transcription (3 rules + 1 rider)

**Status:** STAGED — authoring gated behind the live mining-slot claims (per the penthera/plan-295 posture)
**Claimed:** shikuwa-4090-b292-cvenge, 1793050367, expires 2026-10-28T21:32:47Z

Source: hevnsnt/CVEngine @ `9dd9c441630eaeb15b61b173b635bb673a27bb02` (GPL-3.0, pure Python — transcription law, fixtures are OUR original Rust). Verdict: **YES B** (Research 330). Pure-Python source, so every detector is name-gated/field-literal per the penthera precedent — probe parity (plan-262 T7) is the correctness gate: detector fires on the buggy fixture, silent on the fixed form, zero-FP negative suite.

## Tasks

- [ ] T0 — re-verify quotes at the pin: re-clone `.raw/CVEngine` @ `9dd9c441` (shallow), re-read `web/reNgine/client_ip.py` (whole), `web/targetApp/target_types.py:164-255`, `web/reNgine/tasks.py:590-600`, `web/reNgine/input_guard.py:1-70+183-260`, `web/cveFeed/discovery.py:215-235`; record any drift vs Research 330 before authoring. Sweep the clone at close.
- [ ] T1 — dedup re-verification (both vocabularies, corpus at HEAD): `forwarded|xff|client_ip|remote_addr`, `to_ipv4|ipv4_mapped`, `rebind|use_time`, `Command|shell|sh -c`, `strike|sec_fetch` over `src/domains/` — confirm still zero sec owners; name the kinship to `unvalidated-egress-url` in every new entry.
- [ ] T2 — author `client-ip-from-untrusted-forwarded-header` (InputValidation, A− lead): buggy = leftmost/unconditional `x-forwarded-for` read as the client identity feeding a rate-limit/ban decision; fixed = the three-regime resolver (no trusted proxies → ignore header; peer trusted → walk right-to-left first untrusted hop; malformed hop → fail closed to peer) + `to_ipv4_mapped` unwrap in the parse. Original Rust fixture, std-only. Suggest-only, no `collect_edits` arm, no `FIXABLE_SEC` membership. Detector: header-literal gate + identity/rate-limit vocabulary co-occurrence without trusted-proxy vocabulary.
- [ ] T3 — author `shell-string-command-interpolation` (InputValidation, B+): buggy = `Command::new("sh").arg("-c").arg(format!("scan {host}"))`; fixed = arg arrays on the real binary (shell features regime + strict-token-validation face in context_notes). Detector: `format!`/concat expression inside the `-c` (or `/C`) arg of a sh/bash/cmd Command chain. Suggest-only.
- [ ] T4 — author `ipv4-mapped-range-check-unwrap` (InputValidation, B): buggy = `match ip { IpAddr::V4(v4) => nets.iter().any(|n| n.contains(&v4)), IpAddr::V6(_) => false }` (mapped `::ffff:127.0.0.1` bypasses); fixed = `to_ipv4_mapped()` unwrap before range tests. Context carries the scope-id/parse-first face (`fe80::1%eth0` — Rust's parse refuses it; pass the parsed form onward, never the raw token). Detector: IpAddr match with a non-unwrapping V6 arm.
- [ ] T5 — count-neutral rider on `unvalidated-egress-url`: the USE-TIME face — "an admission-time check on a STORED name rots (DNS moves); re-filter at every sink; the honest framing is 'narrows the rebinding window, never closes it'" (CVEngine `scannable_hosts` + `discovery.py:222` verbatim law, quoted at the pin). Rider lives in corpus DATA (context_notes), never a comment (the batch282 law).
- [ ] T6 — author `abuse-strike-counts-cross-site-requests` (B−, honest-drop-allowed): buggy = a strike/penalty counter incremented on every rejected request regardless of browser origin (a hostile page bans your visitors); fixed = cross-site/same-site-labeled requests rejected-but-never-counted. Name-gated on strike/rate-limit vocabulary. If the fixture cannot carry a zero-FP negative suite at T7, DROP with the reason recorded here.
- [ ] T7 — standing gates: batch292 needle gate (one needle per new rule + the rider text assert) + parse gates FIRST-RUN; sec proposers compile; default lib tests green; `cargo clippy --all-targets -- -D warnings` ×3 postures (default / all-features / no-default); probe-parity evidence per rule recorded (real shapes: buggy fires, fixed silent, negative suite zero-FP).
- [ ] T8 — count pins: AGENTS.md sec count 19→22/23, README corpus table, router/domain totals if sec appears in them, `.docs/11_domains/sec.md` ledger row. Grep the lib-test failure list for stale count sites before writing pins (the pin-set-wider lesson).
- [ ] T9 — retrieval floors BOTH postures re-measured (corpus delta = sec; the B274 law: sec is release-set-adjacent — check whether the release binary rebuild is owed by the §0 bin-age gate after landing; sec IS in the release feature set per the sec domain's default posture — rebuild + `--info` CORPUS verify if owed).
- [ ] T10 — score-bench: run `cargo refine --score-bench` (corpus-touching landing) + gen2/gen3 freeze tables ONLY if the clippy_lints-seeded freeze corpus moves (sec-only delta → expected no movement, run-to-confirm per the P30 law).
- [ ] T11 — research note 330 stays the provenance record (update its queue-slot line to the landing commit at close); docs/AGENTS updates in the same commit; queue snapshot landing row.
- [ ] T12 — commit + push (develop, `feat:` prefix): plan + research note + queue line landed as the verdict commit FIRST (done at staging), then the batch landing as its own commit with every gate green.

## Honest scoping

- 3 own rules + 1 rider expected to land; T6 carries an explicit drop path.
- All rules suggest-only, NotBenchable-by-domain (the dial9 split — annotation, never a demotion).
- GPL-3.0 source: zero verbatim paste; fixtures are original Rust; quotes at the pin in the research note only.
