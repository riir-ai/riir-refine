# Plan 337 — Batch 329: the tembo/voxl distill conversion (rust_perf + sec)

**Status:** LANDED — rust_perf 294→296 + sec 39→41 + 2 count-neutral faces; all gates green (see the landing commit)
**Lease closed:** zed-m3-refine-b329, epoch 1793800137, claimed expiry 2026-11-06T13:48Z — discharged by this landing (claim commit `817e8567`)

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

- [x] T0 — re-clone `.raw/voxl` at the pin (shallow); verify HEAD == `eba3b7ce…`;
      License row confirmed MIT; re-verify every load-bearing quote from Research 367
      at the clone (access.rs module doc + refuse() + origin_key; video.rs
      stripe_layout doc + ThreadGovernor docs + CostFit doc + flow_ok stall line +
      coalesce comment; encoder.rs mb_info comments + encode() doc; files.rs
      UPLOAD_HEADER + resolve() doc; capture.rs find_xvfb_framebuffer doc + pid-scope
      comment + wait.rs observe-at-source doc). Read the UNREAD files carried from the
      verdict (`server.rs` full, `desktop.rs`, `input.rs`, `mic.rs`, `audio.rs`,
      `corners.rs`, `mp4.rs`, `record.rs`, `screen.rs`, `login.rs` rest, `mac/*`,
      `win/*`) — never grade an unread file; re-grade anything that changes.
      — DONE (T0 subagent: clone HEAD-verified at the pin; all 22 unread files read
      in full ~8.3k lines; headline: NO new candidate beats the staged set — the
      unread surface's value is ~30 line-verified KILLS + 3 face candidates +
      corroborations; the strong face (panicking conversion/clamp guard before
      client numbers — record.rs L207-214 + screen.rs L21-28, both panic=abort-
      commented, test-pinned) has NO corpus host rule, so it is recorded as the
      top follow-up-pool item, not authored this batch; the 2 weaker faces (DXGI
      rect-count>32 whole-vs-incremental crossover; mp4 avc1 `as u16` vs tkhd
      full-width sibling truncation) likewise recorded; corroborations: burst-
      settle coalesce ×3 input threads, /wait observe-at-source (record.rs L179),
      read-only allowlist (server.rs L303/L310), the oss-fuzz second-expansion
      shape (win/desktop.rs png_to_dib); 7 one-line marginal hardening notes).
- [x] T1 — dedup re-verification at authoring (3+ vocabularies, live counts):
      R1 vs the kernel_opt skip family (`skip-unchanged-writes-via-dirty-encoding`,
      `predicate-gated-skip-empty-workset-step`,
      `cost-model-counts-work-the-kernel-skips`) + rust_perf
      `prefilter-skip-density-demotion` — sink-distinct lines in-rule; R2 vs
      entries_19 pair + `dynamic-work-parallelism-ladder` + `thermal-hysteresis-*`;
      R3 vs `cors-any-origin-with-credentials` + `client-ip-from-untrusted-forwarded-header`;
      R4 vs the B258 CSRF-drop record (adjudicate: mechanical header-gate form vs the
      architectural bulk — honest drop allowed); R5 vs the InputValidation family.
      Zero new rule_id collisions.
      — DONE (greps live 2026-11-04: damage/dirty/hint-forwarding ZERO owners;
      governor/adaptive-thread/least-squares ZERO; rebind/host-allowlist/origin-gate
      ZERO; csrf/preflight/custom-header ZERO — the only hits are kinship-axis
      mentions inside auth_cookie_missing_flags + cors_any_creds prose; shm ZERO;
      the slices' kinship corrections verified live: gpu_129 owns cost-model-counts
      (V1's fix vs the brief's gpu_133), dynamic-work-parallelism-ladder = entries_06
      (V2's fix vs the brief's entries_59), thermal-hysteresis = entries_21+26).
- [x] T2 — author R1 (rust_perf, dispatch): original fixtures BOTH ARMS
      standalone-compiled AND executed (buggy: full-frame per-cell work regardless of
      a change map; fixed: partition selection by band intersection + per-cell skip
      hints consumed by the worker). GOAT-eligible: bench on rust_perf_bench
      (work-count + copy-bytes axes; magnitude anchor 1/dirty-fraction). In-rule
      kinships: the gpu_07/gpu_129/gpu-B294 distinct-from lines.
      — DONE (entries_61.rs; fixed arm's #[test] green — consumer-visible byte-identity
      over 4 moving-band steps + the empty step; the slice's pre-assembly validator
      probe ×3 measured 2.14×/3.20×/3.14× at a 12-12.5% dirty band (its own harness
      shape kept the checksum fold in both arms — ratios under the 1/dirty-fraction
      ceiling, alloc-det ×3); the LANDED harness's binding validator runs:
      ParetoDominant 3/0/0, 8.18×/6.96×/6.99× at the landing, reproduced 8.66×/
      6.93×/7.06× on a re-run at load ~21 (verdict class stable) — the corpus notes
      carry the landed-harness numbers, the earlier probe's lower ratios disclosed
      here as provenance).
- [x] T3 — author R2 (rust_perf, concurrency, NotBenchable-annotation expected):
      fixtures for the fit + governor (EWLS fit over (work, ms) pairs, asymmetric
      hysteresis thresholds, min-samples, degenerate-variance fallback, switch cost
      disclosed in-rule). In-rule kinships: entries_19 pair (captured-table integrity
      — this is online mode selection), entries_06 (structure choice, not thread
      count), thermal-hysteresis (band vocabulary).
      — DONE (entries_62.rs; fixtures 3+7 standalone green; NotBenchable-by-regime
      annotated, bench_spec None; the decayed-weight gate (w<30.0 → 33rd sample)
      verified against the clone and mirrored faithfully).
- [x] T4 — author R3 (sec, web-trust, suggest-only): original axum-middleware
      fixtures (buggy: a localhost service answering any Host/Origin; fixed: the
      IP-literal/localhost/allowlist host gate + canonicalized origin gate + 403
      naming the fix). Faces: origin default-port canonicalization (count-neutral);
      non-UTF-8 header fail-closed arm as a negative test. Probe-parity posture per
      the sec batch protocol.
      — DONE (origin_rebind_gate.rs 9 tests + corpus entry + eval span + sec.md;
      boundary gate 2/2; both faces carried in-concept; fixtures std-only so the
      probe crate gained NO dep for R3).
- [x] T5 — adjudicate R4 (custom-header mutation gate) against the B258 CSRF-drop
      record; if it survives, author suggest-only with the unanswered-preflight
      regime in-rule; otherwise record the drop in this plan + Research 367 addendum.
      — DONE, SURVIVES + landed (mutation_header_gate.rs: the declined bulk is
      session ARCHITECTURE — rate limiting/IDOR/authz/session/OpenAPI; the mechanical
      header-gate form with the in-source unanswered-preflight mechanism reason is a
      different fix-space; adjudicated in-rule; 9 tests standalone + in-crate parity;
      probe parity PASS — real clippy 1.99.0 pedantic+nursery+suspicious+correctness+
      perf+style+complexity on the buggy shape: ZERO warnings; fixture API repaired
      to real axum-0.8 (Body::into_inner does not exist there — to_bytes; caught by
      the boundary gate's new build-failure witness); probe crate gains axum+tokio
      deps per the B88/B258/B291/B328 convention).
- [x] T6 — adjudicate R5 (shm-attach pid + content verify) at T0's re-read;
      author suggest-only or drop honestly.
      — DONE, DROPPED honestly (niche population — who else attaches to enumerated
      SysV shm? the verdict's own drop-expected; recorded in the Research 367
      addendum + this plan; the creator-pid + content-oracle law remains in the
      research as the seed if the corpus ever opens the family).
- [x] T7 — faces: symlink-listing re-check on `untrusted-path-join-escape` (if T0's
      files.rs re-read confirms the shape); fixed-header reserve-in-place on the
      capacity-reservation family (adjudicate the host at authoring). Count-neutral
      — no corpus-count pins move for faces.
      — DONE (both landed: the listing-time rider on untrusted-path-join-escape —
      voxl files.rs::list L94-105 follows symlinks for type/size but hides links
      whose canonicalize escapes the root, two sinks one boundary, corpus.rs rider;
      the wire-header reserve-in-place rider on rust_perf
      reserve-then-commit-capacity-accounting — entries_58.rs rider, out.resize(HDR,0)
      + in-place writes + Bytes::from zero-copy, video.rs L911/L1107-1114 verified).
- [x] T8 — standing gates per Protocol A: batch329 needle + parse gates;
      default lib; rust_perf_bench lib; sec probe-parity posture; clippy −D ×3-4
      postures; retrieval floors RE-MEASURE (corpus delta) with pre/post attribution;
      freeze NOT expected owed (zero clippy_lints delta, P30) — run if clippy_lints
      sneaks in; score-bench appended; release-set binary rebuild + install BEFORE
      push (rust_perf is release-set surface, the §0 bin-age law).
      — DONE (mutation_header 9/9 + batch329 gates 11/0 + rust_perf batch329 gates;
      default lib 2819/0; sec+latent 3361/0 (4i); rust_perf 2819/0;
      rust_perf_bench 2850/0 (7i); dist 2819/0 + dist_gates 15/0; clippy −D ×4
      postures (default / no-default / rust_perf+sec / +latent+verify) + all-targets
      +bench+sec ALL ZERO; boundary gate 2/2 (now with the probe build-failure
      witness — the green-zero blindness repaired en-route); retrieval re-measured:
      default+latent router 86/142 UNCHANGED (floor held, no re-pin), sec-on:
      sec self-variants 37/37 top-1 (35 → 37, both new spans route + retrieve
      top-1), blended 179 @ 89.9%, router 78/179 — measured notes recorded in
      retrieval_eval.rs; freeze NOT owed (zero clippy_lints delta, the P30 rule);
      score-bench run #224 heal 0.9636 FLAT parse 1.0 decline 1.0 (gen-8 row
      appended); release-set binary rebuilt at the pinned set + installed via the
      global symlink, hash == HEAD).
- [x] T9 — claim before any corpus edit (push the claim; expiry 48 h); count pins via
      ONE shape-asserted script pass (tests.rs + mod.rs + router.rs + AGENTS.md ×2 +
      README ×3 + rust_perf.md/sec.md batch blocks + pins manifest regen);
      Research 367 addendum at landing; AGENTS.md/README counts + the queue snapshot
      landing row in the same commit; `.raw/voxl` removed at close; commit+push.
      — DONE (claim 817e8567 pushed pre-work; counts moved 294→296 + 39→41 via the
      pins_manifest --write pass + the prose surfaces (AGENTS.md L15/L18/L137,
      README L13/L15/L135, rust_perf.md batch block, sec.md batch blocks ×2, the
      tests.rs pins 296-total + 243-annotated + the 242-message-string repair);
      the retrieval floor comments carry the Batch-329 measured notes; the queue
      snapshot landing row + the Research 367 addendum + this plan's discharge in
      the landing commit; .raw/voxl + the scratch harnesses removed at close).
