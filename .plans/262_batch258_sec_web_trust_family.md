# Plan 262 — Batch 258: the sec web-trust family (cloudflare/security-audit-skill distill)

**Status:** OPEN — staged, corpus edits deferred to the next cargo-quiet slot (verdict: Research 292; source pin `c1c8a8c1471069fb0e188eeaff69b8e8db6564a8`, MIT)

Staged from the 2026-10-19 distill verdict on cloudflare/security-audit-skill. Sec-domain batch: 2-3 suggest-only rules + 1 possible rider, all transcriptions (the source is prose — every fixture is an ORIGINAL minimal Rust reproduction; never paste upstream text into fixtures).

## Tasks

- [ ] T0 — re-clone + re-pin: `git clone --depth 1 https://github.com/cloudflare/security-audit-skill.git .raw/security-audit-skill`, `git -C .raw/security-audit-skill rev-parse HEAD` must equal `c1c8a8c1471069fb0e188eeaff69b8e8db6564a8`; drift → re-read the verdict's candidate rows against the new pin before authoring (the B193 freshness law).
- [ ] T1 — dedup re-verification at authoring time: re-run the four greps (traversal/ParentDir, webhook/raw-body, ssrf/egress-url, GzDecoder) over `src/domains/sec/` — corpus may have moved since 2026-10-19; a new owner demotes the candidate to a rider or drops it.
- [ ] T2 — author `untrusted-path-join-escape` (B lead): SecFamily placement adjudicated at authoring (likely a new `WebTrust` family or UnsafeHygiene — the count-pin math changes with a new family, prefer existing family if honest); trigger = a non-const `str`/`&Path`/`PathBuf` arg joined into a base path then a `fs::*` sink; fixture = original minimal (config-filename + `Path::new(base).join(name)` + `fs::read_to_string`); fixed form = the `components()` walk refusing `ParentDir`/`RootDir`/`Prefix` (the riir-deployer `48d63f7` in-house shape, cited in-rule); suggest-only.
- [ ] T3 — author `webhook-signature-over-raw-bytes` (B−): trigger = a verify-shaped call (`verify`/`verify_signature`/`hmac` vocabulary in fn or callee name) whose operand is a re-serialization (`serde_json::to_string`/`to_vec` of a just-parsed value) instead of the raw input binding; fixture = original minimal webhook verify; fixed form = verify over the raw bytes; suggest-only; distinct-from note naming `constant-time-secret-compare` (compare primitive vs coverage axis).
- [ ] T4 — author `unvalidated-egress-url` (B−→C+) OR drop honestly: trigger = URL string assembled from request/config vocabulary reaching a named egress sink (reqwest/get/dial); honest recall limits stated in-rule (framework-shaped extraction is an honest NO — mirror `unbounded-read-into-vec`'s disclosure style); fixed form = scheme/host allowlist + private-range refusal sketch; suggest-only. Drop at authoring if the trigger scope cannot be made parse-shaped without fiction.
- [ ] T5 — decompression-cap RIDER on `unbounded-read-into-vec` (C+): fold only if the owner rule carries the "bound after every expansion stage" face without mushing the trigger; otherwise record the drop in the batch note.
- [ ] T6 — poisoned-lock-fail-open (C): default DROP; author only if the batch wants a third body and the regime note survives the honest-args test.
- [ ] T7 — probe pass (the sec-domain GOAT substitute): confirm zero upstream clippy lints fire on the three buggy fixtures (real clippy, pedantic+nursery+suspicious+correctness, the nan-unsafe-sort-comparator discipline) — a probe hit demotes the rule to a clippy_lints pointer or kills it.
- [ ] T8 — standing gates: `cargo test --lib` sec module (parse + unique-kebab + count pins), full lib at default + the sec feature posture, clippy −D ×3 postures, score-bench append (P74 precedent: sec delta expects FLAT, no floor re-pin), freeze-tables adjudicated per which lane's corpus moved (sec is not the clippy lane — expected NOT owed; verify against the plan-137 T7.5 rule).
- [ ] T9 — count pins: corpus total (sec 16 → 17-19), family counts if a new family lands, AGENTS.md sec bullet, README/.docs/11_domains/sec.md if the family table moves.
- [ ] T10 — doc updates: Research 292 addendum (landed counts + any drops with reasons), queue-snapshot landing row (discharge this plan's slot), `.raw/security-audit-skill` REMOVED at close.
- [ ] T11 — commit + push, `feat:` prefix, Session marker in the body.

## Deferred

- [-] The workspace skill half is NOT this plan's — it landed 2026-10-19 in `../katgpt-rs/.agents/skills/security-audit/SKILL.md` (the user's ask, same session as the verdict).
