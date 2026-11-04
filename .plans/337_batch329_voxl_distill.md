# Plan 337 — Batch 329: the tembo/voxl distill conversion (rust_perf + sec)

**Status:** STAGED — pending queue slot (verdict: Research 367, YES B−; nominal Batch 329)

Source: tembo/voxl @ `eba3b7ced9f60e885bb7b3a96a12e3ccb347fdcd` (MIT code; tip moved
2026-11-03, one day before the 2026-11-04 inspection — FAST-MOVING, re-clone at the
pin, moved HEAD = re-pin + re-verify every quote BEFORE authoring). Inspected 2026-11-04
(Redirects 367); shallow clone removed at the verdict commit.

Targets: rust_perf 294→296 (+2: the damage-map lead B, the cost-fit governor B−) and
sec 38→39-40 (+1-2: the host/origin rebinding gate B, one of custom-header gate /
shm-attach verify at C+/B−) + 2-3 count-neutral faces (origin-key port
canonicalization on the R3 rule; symlink-listing re-check on `untrusted-path-join-escape`;
fixed-header reserve-in-place on the capacity-reservation family, adjudicate at
authoring). Honest drops allowed on the two C+ candidates (`burst-settle-coalesce`,
the /wait observe-at-source pattern). Zero GPU throughout (M3 mining posture).

## Tasks

- [ ] T0 — re-clone `.raw/voxl` at the pin (shallow); verify HEAD == `eba3b7ce…`;
      License row confirmed MIT; re-verify every load-bearing quote from Research 367
      at the clone (access.rs module doc + refuse() + origin_key; video.rs
      stripe_layout doc + ThreadGovernor docs + CostFit doc + flow_ok stall line +
      coalesce comment; encoder.rs mb_info comments + encode() doc; files.rs
      UPLOAD_HEADER + resolve() doc; capture.rs find_xvfb_framebuffer doc + pid-scope
      comment + wait.rs observe-at-source doc). Read the UNREAD files carried from the
      verdict (`server.rs` full, `desktop.rs`, `input.rs`, `mic.rs`, `audio.rs`,
      `corners.rs`, `mp4.rs`, `record.rs`, `screen.rs`, `login.rs` rest, `mac/*`,
      `win/*`) — never grade an unread file; re-grade anything that changes.
- [ ] T1 — dedup re-verification at authoring (3+ vocabularies, live counts):
      R1 vs the kernel_opt skip family (`skip-unchanged-writes-via-dirty-encoding`,
      `predicate-gated-skip-empty-workset-step`,
      `cost-model-counts-work-the-kernel-skips`) + rust_perf
      `prefilter-skip-density-demotion` — sink-distinct lines in-rule; R2 vs
      entries_19 pair + `dynamic-work-parallelism-ladder` + `thermal-hysteresis-*`;
      R3 vs `cors-any-origin-with-credentials` + `client-ip-from-untrusted-forwarded-header`;
      R4 vs the B258 CSRF-drop record (adjudicate: mechanical header-gate form vs the
      architectural bulk — honest drop allowed); R5 vs the InputValidation family.
      Zero new rule_id collisions.
- [ ] T2 — author R1 (rust_perf, dispatch): original fixtures BOTH ARMS
      standalone-compiled AND executed (buggy: full-frame per-cell work regardless of
      a change map; fixed: partition selection by band intersection + per-cell skip
      hints consumed by the worker). GOAT-eligible: bench on rust_perf_bench
      (work-count + copy-bytes axes; magnitude anchor 1/dirty-fraction). In-rule
      kinships: the gpu_07/gpu_129/gpu-B294 distinct-from lines.
- [ ] T3 — author R2 (rust_perf, concurrency, NotBenchable-annotation expected):
      fixtures for the fit + governor (EWLS fit over (work, ms) pairs, asymmetric
      hysteresis thresholds, min-samples, degenerate-variance fallback, switch cost
      disclosed in-rule). In-rule kinships: entries_19 pair (captured-table integrity
      — this is online mode selection), entries_06 (structure choice, not thread
      count), thermal-hysteresis (band vocabulary).
- [ ] T4 — author R3 (sec, web-trust, suggest-only): original axum-middleware
      fixtures (buggy: a localhost service answering any Host/Origin; fixed: the
      IP-literal/localhost/allowlist host gate + canonicalized origin gate + 403
      naming the fix). Faces: origin default-port canonicalization (count-neutral);
      non-UTF-8 header fail-closed arm as a negative test. Probe-parity posture per
      the sec batch protocol.
- [ ] T5 — adjudicate R4 (custom-header mutation gate) against the B258 CSRF-drop
      record; if it survives, author suggest-only with the unanswered-preflight
      regime in-rule; otherwise record the drop in this plan + Research 367 addendum.
- [ ] T6 — adjudicate R5 (shm-attach pid + content verify) at T0's re-read;
      author suggest-only or drop honestly.
- [ ] T7 — faces: symlink-listing re-check on `untrusted-path-join-escape` (if T0's
      files.rs re-read confirms the shape); fixed-header reserve-in-place on the
      capacity-reservation family (adjudicate the host at authoring). Count-neutral
      — no corpus-count pins move for faces.
- [ ] T8 — standing gates per Protocol A: batch329 needle + parse gates;
      default lib; rust_perf_bench lib; sec probe-parity posture; clippy −D ×3-4
      postures; retrieval floors RE-MEASURE (corpus delta) with pre/post attribution;
      freeze NOT expected owed (zero clippy_lints delta, P30) — run if clippy_lints
      sneaks in; score-bench appended; release-set binary rebuild + install BEFORE
      push (rust_perf is release-set surface, the §0 bin-age law).
- [ ] T9 — claim before any corpus edit (push the claim; expiry 48 h); count pins via
      ONE shape-asserted script pass (tests.rs + mod.rs + router.rs + AGENTS.md ×2 +
      README ×3 + rust_perf.md/sec.md batch blocks + pins manifest regen);
      Research 367 addendum at landing; AGENTS.md/README counts + the queue snapshot
      landing row in the same commit; `.raw/voxl` removed at close; commit+push.
