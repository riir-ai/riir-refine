# Research 330 — CVEngine sec distill (hevnsnt/CVEngine)

**Status:** RECORD — LANDED as Batch 292 (Plan 296; sec 22 → 26; the four rules + the egress use-time rider; gates ALL GREEN, landing commit in the plan + the queue snapshot row; yield 3 own + 1 rider as estimated, the B− strike rule's honest-drop path never fired).

## Source

- **Repo:** https://github.com/hevnsnt/CVEngine @ `9dd9c441630eaeb15b61b173b635bb673a27bb02` (master, cloned shallow into `.raw/CVEngine`, REMOVED after this note — quotes re-verify at mine time per the B96-98 law).
- **License:** GPL-3.0. **Provenance chain:** a hardened fork of reNgine 2.x (Yogesh Ojha) — the security subsystem distilled here (`targetApp/target_types.py`, `reNgine/input_guard.py`, `reNgine/client_ip.py`, `reNgine/ip_allowlist.py`) is CVEngine's own hardening layer (CVENGINE_-prefixed envs, its own test suite `web/tests/test_input_guard.py` / `test_target_types.py` / `test_hard_security.py`), not upstream reNgine code. Cite the fork for the class shapes; the fixtures below are OUR original Rust reproductions.
- **Language:** pure Python (Django/Celery) — **ZERO `.rs` files. Per the transcription law (the penthera lesson, Research 329): language is never a gate.** The sec domain's Accepts column names the RULE's Rust shape; fixtures are always ours. Every candidate below carries a bounded Rust-shaped fix-space we can author.
- **Maintenance:** 303 stars / 50 forks / 144 commits, actively developed (2026). Not that the grade depends on it.

## Verdict

**YES B corpus / lane-intel N/A (non-perf source) / product-coverage POSITIVE — extends the sec InputValidation family onto the two most universal std surfaces in the ecosystem (`std::net::IpAddr`, `std::process::Command`), which every Rust server/CLI binary touches.**

- **Domain routing:** `sec` (19 rules, LANDED — Batch 258). Families per-rule: `InputValidation` (network-target validation + command construction). **No new `SecFamily` variant** (append-only discipline). The tempting WRONG routing is not a domain — it is the LANGUAGE gate ("Python app, nothing to mine"), refuted by the skill's own transcription law and the corpus's own history (B258 landed 3 rules from a pure-Markdown source).
- **Lane mapping:** NONE. CVEngine is an attack-surface-management platform — no GPU kernels, no serving perf, no league cell. The lane-intel axis produces no artifacts (correctly).
- **Product coverage:** the sec corpus's web-trust family (5 live + 3 penthera-staged) covers tower-http/jsonwebtoken/cookie. CVEngine's classes cover **client-IP identity extraction (X-Forwarded-For), internal-range SSRF guards, and subprocess command construction** — std-only shapes that fire on every Rust HTTP service and every CLI that shells out. That is broader ecosystem coverage than any single famous-lib family.

## Why this source is good (inspection evidence)

This is a **production-hardened scanner** whose entire value proposition is "safe by design": it must aim nmap/naabu/nuclei/bbot at user-supplied targets without ever hitting loopback, link-local (cloud metadata 169.254.169.254), RFC1918, or its own deployment network — and must resist exactly the request-level attack classes our corpus transcribes (command injection, path traversal, SSRF, spoofed identity). The hardening layer is unusually thoughtful, with several mechanisms our corpus has NO owner for:

1. **`client_ip.py` is a textbook-correct X-Forwarded-For resolver** — and its INVERSE (the naive leftmost read) is one of the most common real Rust web bugs (rate-limit/ban bypass via header spoofing). Its three-regime decision content: no trusted proxies → header ignored entirely (attacker-settable); peer ∈ trusted set → walk right-to-left, first untrusted hop wins; **malformed hop mid-chain → the whole chain is untrusted → fall back to the peer address (fail-closed)**. Plus IPv4-mapped unwrap inside the parse.
2. **`target_types.py::is_scannable_ip`** — parse-first IP validation: always-blocked set (loopback 127/8, link-local 169.254/16, multicast, unspecified, `::1`, `fe80::/10`, `ff00::/8`, `::`), internal ranges (RFC1918 + CGNAT 100.64/10 + ULA fc00::/7) gated behind an explicit operator env (`CVENGINE_ALLOW_PRIVATE_TARGETS`), operator extra-block CIDRs, and two bypass faces most validators miss: **IPv4-mapped IPv6 unwrap** (`::ffff:127.0.0.1` must be seen as loopback — version-split matching misses it) and **scope-id refusal** (`fe80::1%eth0` — Python's parser round-trips it and it gets handed to scanners as a CLI arg; Rust's `IpAddr::from_str` rejects it, which is itself the lesson: pass the PARSED form onward, never the raw token).
3. **Use-time re-validation of stored names** (`scannable_hosts` + `discovery.py:222`): "A name a tenant controls can be pointed at 169.254.169.254 or an internal range AFTER it was stored" — every path that aims a scanner at a stored host list re-filters (admission-time checks rot as DNS moves; the docstring honestly calls it "narrows the DNS-rebinding window rather than closing it").
4. **`input_guard.py`** — layered command-injection defense: signature regexes weighted, strict-field charset allowlists, strike accumulation → account disable + token revocation + 24h IP ban. Two laws stand out: the **cross-site-strike asymmetry** ("A page on another site can make a victim's browser send these requests... reject them but never count them, so a hostile page cannot get a visitor banned") and the **shared-IP ban gate** (`_ip_is_banable` refuses to IP-ban private/loopback/trusted-proxy addresses — "that would block a whole site").
5. **The `tlsx` site** (`tasks.py:595-596`) is the live NEGATIVE specimen: `cmd = f'tlsx -san -cn -silent -ro -host {domain}'` → `run_command(cmd, shell=True)` — f-string interpolation into a shell string, defended only by the guard's charset allowlist. **An allowlist that excludes `;|&$\`` still permits argument injection (CWE-88)**: `foo -oG /tmp/x` passes `^[A-Za-z0-9._:/\[\] \-]*$`. The defense-in-depth layering does not make the shell string safe; the arg-array fix does.

## Ranked candidate table

| # | Candidate rule_id | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `client-ip-from-untrusted-forwarded-header` | `reNgine/client_ip.py` (whole file) | **A− (lead)** | Leftmost/unconditional XFF read as client identity → rate-limit/ban/audit bypass. Fix = the three-regime resolver (peer-gate, rightmost walk, fail-closed malformed hop). Detector: header-literal name gate (`x-forwarded-for`) + identity/rate-limit vocabulary co-occurrence without trusted-proxy vocabulary (the penthera field-literal precedent). Suggest-only, NotBenchable-by-domain. |
| 2 | `shell-string-command-interpolation` | `reNgine/tasks.py:595` (the tlsx site, read as the class) | **B+** | `sh -c` + `format!`-interpolated user data vs arg arrays. Decision content: metachar allowlist ≠ safety (CWE-88 survives); regime = shell features genuinely needed vs not. Detector: `format!`/concat expression inside the `-c` arg of a `sh`/`bash`/`cmd /C` Command chain — parse-only feasible. Suggest-only. |
| 3 | `ipv4-mapped-range-check-unwrap` | `target_types.py:196-199` + `client_ip.py:18` (the `ipv4_mapped` face) | **B** | Version-split range checks miss `::ffff:127.0.0.1`. Fix: `to_ipv4_mapped()` unwrap before any range/allowlist/denylist test. Detector: `match` on `IpAddr` whose V6 arm rejects/falls-through without an unwrap (parse-only). Bypass class for every SSRF guard + firewall + allowlist in the ecosystem. |
| 4 | `stored-target-revalidated-at-use-time` (RIDER on `unvalidated-egress-url`) | `target_types.py:228-255` + `discovery.py:222` | **B− (rider)** | Admission-time validation rots; the sink re-checks (use-time face). Cleanest as a count-neutral rider paragraph on the existing egress rule — its use-time face, the Plan-156 T2 sink-distinct shape. |
| 5 | `abuse-strike-counts-cross-site-requests` | `input_guard.py:246-249` | **B−** | Reject cross-site attack payloads but never COUNT them (else a hostile page bans your visitors — DoS-via-defense). Name-gated on strike/rate-limit vocabulary + no same-site/sec-fetch check. Thin but genuinely novel; the corpus's name-gated web-trust precedents cover the shape. |
| 6 | scope-id refusal (`fe80::1%eth0`) | `target_types.py:59-64` | DROP (context face) | In Rust the parse refuses scope ids — fold into #3's context as the "pass the parsed form onward" law, not a standalone rule. |
| 7 | IP-ban refuses shared/private ranges | `input_guard.py:183-188` | DROP (thin) | The fix is one range-check insertion — the `poisoned-lock-fail-open` thinness precedent. Fold the face into #1's context. |
| 8 | AI-triage bounds ("fixed rules mean the model can never wave away hard evidence"), human-approved AI-drafted templates | README + `nuclei_scan.py` approval flow | DROP | Trust-boundary posture, no bounded Rust code shape — the B258 declined bulk. |
| 9 | stored `update_command` → `shell=True` | `api/views.py:1315` | DROP | By-design admin feature (tool update); the class is already owned by #2's shape. |
| 10 | strike threshold / ban policy / 2FA / IP allowlist middleware | `input_guard.py`, `ip_allowlist.py` | DROP | Rate-limit/authz architecture — skill-checklist material, not corpus. |

## Dedup-gate evidence (grep `src/`, both vocabularies, 2026-10-26)

- `forwarded|client_ip|remote_addr|xff` → only rust_perf "forwarded store" (a compiler IR concept) and rustc_errors "cross-site transfer" (insight transfer). **Zero sec coverage.**
- `to_ipv4|ipv4_mapped|mapped` → `MappedRows` (rusqlite) + prose "mapped". **Zero.**
- `rebind|use_time|resolv` → fix_verify path resolution only. **Zero.**
- `Command|shell|sh -c` in `src/domains/sec/` → **zero.**
- `strike|abuse|sec_fetch` → idiom "third strike" in kernel_opt comments. **Zero.**
- Nearest kin named: `unvalidated-egress-url` (B258) — owns egress URL allowlist SELECTION; #3 owns the mapped-address bypass of ANY range test, #4 the use-time face. Sink- and defect-distinct per the Plan-156 T2 adjudication shape; kinship recorded in-rule at authoring.

## GOAT candidate (the two-class test)

Criterion 1 fails for a **DOMAIN reason** (sec has no perf validator — the dial9 split): every rule is `NotBenchable`, `bench_spec: None`, suggest-only, probe-parity substituting per plan-262 T7 (detector fires on the buggy fixture, silent on the fixed form, zero-FP negative suite). That is an annotation, never a demotion. The lead GOAT-shaped candidate is #1: its correctness axis is the probe-parity gate, its magnitude story is "the single most-deployed web identity bug in the ecosystem" (every rate limiter behind a proxy), and its regime conditions are exact (three deployment postures).

## Yield estimate

**3 own rules + 1-2 riders after ~50% coordinator falsification** (#1, #2, #3 own; #4 rider count-neutral; #5 honest-drop-allowed). Matches the recent sec batch shapes (B258: 3 rules; penthera staged: 3).

## Honest caveats

- **Unread files (re-read at mine time):** `agentapi/services.py` (MCP tool surface — beyond the line-297 refusal), `cveFeed/validation.py` / `sensitive.py` / `bounty_scope.py`, the `startScan/` scan-construction internals, notification webhooks (where `webhook-signature-over-raw-bytes` kin might live — if CVEngine signs outbound webhooks, that is a positive instance, not a rule), `install.sh`, docker-compose. Graded from the mechanism-bearing core only.
- **GPL-3.0:** fixtures are OUR original Rust reproductions; never paste source (standing law).
- **Python-to-Rust transcription risk:** the detector shapes are field-literal/name-gated (the penthera precedent), not proven against real Rust crates yet — the probe-parity gates at authoring decide.
- **reNgine fork provenance:** upstream reNgine carries its own CVE history; the distilled subsystem is CVEngine's divergence. Cite the fork pin for shapes; do not attribute to upstream.
- Class re-verification at pin `9dd9c441` owed at mine time (clone deleted after this note).

## Queue slot

Staged as the **Batch 292 pool** (`sec` transcription, plan 296). Authoring gated behind the live mining-slot claims per the standing discipline; this verdict + plan + queue line land in one commit (`docs:` prefix, develop).
